use std::{collections::HashMap, time::Duration};

use anyhow::Result;
use async_nats::Client;
use futures::StreamExt;
use uuid::Uuid;
use vanguard_interceptor::InterceptorRuntimeState;

use crate::{
    kalman::KalmanTrack,
    state::{PlatformState, TrackedThreat},
};

const THREAT_DETECTED: &str = "vanguard.threat.detected";

const THREAT_ENGAGED: &str = "vanguard.threat.engaged";

const NEIGHBOR_UPDATE: &str = "vanguard.neighbor.update";

const STRATEGY_UPDATE: &str = "vanguard.strategy.update";

use vanguard_core::{
    DetectedThreat, Interceptor, InterceptorState, Message, NEW_PLATFORM, NeighborPlatform,
    Position, THREAT_DESTROYED, ThreatClassification, ThreatTrack, WORLD_THREAT_DETECTED,
    interceptor::TrackStatus,
};

pub struct Platform {
    pub state: PlatformState,
    pub nats: Client,
}

impl Platform {
    pub fn new(state: PlatformState, nats: Client) -> Self {
        Self { state, nats }
    }

    async fn publish(&self, subject: &'static str, msg: Message) -> Result<()> {
        println!("[{}] PUB {} {:?}", self.state.platform.name, subject, msg);

        self.nats
            .publish(subject, serde_json::to_vec(&msg)?.into())
            .await?;

        Ok(())
    }

    pub async fn publish_neighbor_update(&self) -> Result<()> {
        let available = self
            .state
            .platform
            .interceptors
            .iter()
            .filter(|i| matches!(i.state, InterceptorState::Idle))
            .count();

        self.publish(
            NEIGHBOR_UPDATE,
            Message::NeighborUpdate {
                platform_id: self.state.platform.id,
                position: self.state.platform.position.clone(),
                reach: self.state.platform.reach,
                interceptors_remaining: available,
            },
        )
        .await
    }

    pub async fn detect_threat(&mut self, threat: DetectedThreat) -> Result<()> {
        println!("[{}] DETECT {}", self.state.platform.name, threat.id,);

        let is_new_track = self.handle_threat_detected(threat.clone())?;

        self.state.threats.insert(threat.id, threat.clone());

        self.publish(
            THREAT_DETECTED,
            Message::ThreatDetected {
                threat: threat.clone(),
                source_platform: self.state.platform.id,
            },
        )
        .await?;

        if is_new_track && self.is_best_platform(&threat) {
            self.engage_threat(threat.id).await?;
        }

        Ok(())
    }

    fn is_best_platform(&self, threat: &DetectedThreat) -> bool {
        let my_distance = self.state.platform.position.distance(&threat.position);
        if my_distance > self.state.platform.reach {
            return false;
        }

        self.state
            .platform
            .neighbor_platforms
            .iter()
            .filter(|n| n.interceptors_remaining > 0)
            .all(|neighbor| neighbor.position.distance(&threat.position) >= my_distance)
    }

    async fn engage_threat(&mut self, threat_id: Uuid) -> Result<()> {
        if self.state.engaged_threats.contains(&threat_id) {
            return Ok(());
        }

        let Some(interceptor) = self
            .state
            .platform
            .interceptors
            .iter()
            .find(|i| matches!(i.state, InterceptorState::Idle))
        else {
            return Ok(());
        };

        let interceptor_id = interceptor.id;

        self.state.engaged_threats.insert(threat_id);

        self.publish(
            "vanguard.interceptor.target.assigned",
            Message::InterceptorTargetAssigned {
                interceptor_id,
                threat_id,
            },
        )
        .await?;

        self.publish(
            THREAT_ENGAGED,
            Message::ThreatEngaged {
                threat_id,
                platform_id: self.state.platform.id,
                interceptor_id,
            },
        )
        .await?;

        Ok(())
    }

    fn handle_threat_detected(&mut self, threat: DetectedThreat) -> Result<bool> {
        match self.state.tracks.get_mut(&threat.id) {
            Some(track) => {
                track.kalman.update(threat.position.x, threat.position.y);

                let (x, y) = track.kalman.position();

                let (vx, vy) = track.kalman.velocity();

                track.track.position.x = x;
                track.track.position.y = y;

                track.track.velocity.x = vx;
                track.track.velocity.y = vy;

                track.track.confidence = threat.confidence;

                track.track.threat_level = threat.threat_level;

                Ok(false)
            }

            None => {
                self.state.tracks.insert(
                    threat.id,
                    TrackedThreat {
                        track: ThreatTrack {
                            threat_id: threat.id,
                            position: threat.position.clone(),
                            velocity: threat.speed.clone(),
                            confidence: threat.confidence,
                            threat_level: threat.threat_level,
                            last_update: threat.detected_at,
                            source_platforms: vec![self.state.platform.id],
                            status: TrackStatus::Detected,
                            engaged_by: None,
                        },
                        kalman: KalmanTrack::new(
                            threat.position.x,
                            threat.position.y,
                            threat.speed.x,
                            threat.speed.y,
                        ),
                    },
                );

                Ok(true)
            }
        }
    }

    fn handle_threat_engaged(&mut self, threat_id: Uuid, _interceptor_id: Uuid) {
        self.state.engaged_threats.insert(threat_id);

        self.state.threats.remove(&threat_id);

        if let Some(track) = self.state.tracks.get_mut(&threat_id) {
            track.track.status = TrackStatus::Engaged;
        }
    }

    fn handle_neighbor_update(
        &mut self,
        platform_id: Uuid,
        position: Position,
        reach: f64,
        interceptors_remaining: usize,
    ) {
        if platform_id == self.state.platform.id {
            return;
        }

        if let Some(neighbor) = self
            .state
            .platform
            .neighbor_platforms
            .iter_mut()
            .find(|n| n.id == platform_id)
        {
            neighbor.position = position;
            neighbor.reach = reach;
            neighbor.interceptors_remaining = interceptors_remaining;

            return;
        }

        println!(
            "[{}] neighbor added {}",
            self.state.platform.name, platform_id
        );

        self.state
            .platform
            .neighbor_platforms
            .push(NeighborPlatform {
                id: platform_id,
                position,
                reach,
                interceptors_remaining,
            });
    }

    fn handle_track_updated(&mut self, track: ThreatTrack) -> Result<()> {
        println!(
            "[{}] TRACK {} {:?}",
            self.state.platform.name, track.threat_id, track.status,
        );

        let status = track.status.clone();

        if let Some(local_track) = self.state.tracks.get_mut(&track.threat_id) {
            local_track.track.status = status.clone();

            local_track.track.engaged_by = track.engaged_by;
        }

        match status {
            TrackStatus::Detected => {}

            TrackStatus::Engaged => {
                self.state.engaged_threats.insert(track.threat_id);
            }

            TrackStatus::Destroyed => {
                self.state.engaged_threats.remove(&track.threat_id);

                self.state.tracks.remove(&track.threat_id);

                self.state.threats.remove(&track.threat_id);
            }
        }

        Ok(())
    }

    fn handle_interceptor_update(&mut self, interceptor: Interceptor) {
        if let Some(local) = self
            .state
            .platform
            .interceptors
            .iter_mut()
            .find(|i| i.id == interceptor.id)
        {
            *local = interceptor.clone();
        }

        self.state
            .known_interceptors
            .insert(interceptor.id, interceptor);
    }

    fn handle_threat_destroyed(&mut self, threat_id: Uuid, interceptor_id: Uuid) {
        self.state.engaged_threats.remove(&threat_id);

        self.state.tracks.remove(&threat_id);

        self.state.threats.remove(&threat_id);

        if let Some(interceptor) = self
            .state
            .platform
            .interceptors
            .iter_mut()
            .find(|i| i.id == interceptor_id)
        {
            interceptor.state = InterceptorState::Idle;

            interceptor.assigned_track = None;
        }
    }

    async fn handle_message(&mut self, message: Message) -> Result<()> {
        match message {
            Message::ThreatDetected {
                threat,
                source_platform,
            } => {
                self.handle_threat_detected(threat);
            }

            Message::ThreatDestroyed {
                threat_id,
                interceptor_id,
                ..
            } => {
                self.handle_threat_destroyed(threat_id, interceptor_id);
            }

            Message::ThreatEngaged {
                threat_id,
                platform_id,
                interceptor_id,
            } => {
                self.handle_threat_engaged(threat_id, interceptor_id);
            }

            Message::NeighborUpdate {
                platform_id,
                position,
                reach,
                interceptors_remaining,
            } => {
                self.handle_neighbor_update(platform_id, position, reach, interceptors_remaining);
            }

            Message::StrategyUpdate { .. } => {}
            Message::TrackUpdated { track } => {
                self.handle_track_updated(track);
            }
            Message::NewPlatform {
                platform_id,
                position,
                reach,
            } => {}
            Message::InterceptorUpdate {
                platform_id,
                interceptor,
            } => self.handle_interceptor_update(interceptor),
            _ => {}
        }

        Ok(())
    }

    pub async fn run(mut self) -> Result<()> {
        let mut threat_sub = self.nats.subscribe(THREAT_DETECTED).await?;
        let mut interceptor_sub = self.nats.subscribe("vanguard.interceptor.update").await?;
        let mut engaged_sub = self.nats.subscribe(THREAT_ENGAGED).await?;
        let mut destroyed_sub = self.nats.subscribe(THREAT_DESTROYED).await?;
        let subject = vanguard_core::neighbor_subject(&self.state.platform.id);
        let mut world_sub = self.nats.subscribe(WORLD_THREAT_DETECTED).await?;
        let mut track_sub = self.nats.subscribe("vanguard.track.updated").await?;
        let mut neighbor_sub = self.nats.subscribe(subject).await?;

        let mut strategy_sub = self.nats.subscribe(STRATEGY_UPDATE).await?;

        let mut heartbeat = tokio::time::interval(Duration::from_secs(1));

        self.publish(
            NEW_PLATFORM,
            Message::NewPlatform {
                platform_id: self.state.platform.id,
                position: self.state.platform.position.clone(),
                reach: self.state.platform.reach,
            },
        )
        .await?;

        for interceptor in self.state.platform.interceptors.clone() {
            let agent = vanguard_interceptor::InterceptorAgent::new(
                InterceptorRuntimeState {
                    platform_id: self.state.platform.id,
                    interceptor,
                    target_id: None,
                    tracks: HashMap::new(),
                },
                self.nats.clone(),
            );

            tokio::spawn(async move {
                let _ = agent.run().await;
            });
        }

        loop {
            tokio::select! {

                _ = heartbeat.tick() => {

                    for tracked in self.state.tracks.values_mut() {

                        tracked.kalman.predict(1.0);

                        let (x, y) =
                            tracked.kalman.position();

                        let (vx, vy) =
                            tracked.kalman.velocity();

                        tracked.track.position.x = x;
                        tracked.track.position.y = y;

                        tracked.track.velocity.x = vx;
                        tracked.track.velocity.y = vy;
                    }

                    let tracks = self
                        .state
                        .tracks
                        .values()
                        .map(|t| t.track.clone())
                        .collect::<Vec<_>>();

                    self.publish_neighbor_update()
                        .await?;

                    for track in tracks {

                        self.publish(
                            "vanguard.track.updated",
                            Message::TrackUpdated {
                                track,
                            },
                        )
                        .await?;
                    }
                }

                Some(msg) = destroyed_sub.next() => {

                    let msg: Message =
                        serde_json::from_slice(
                            &msg.payload,
                        )?;

                    self.handle_message(msg)
                        .await?;
                }

                Some(msg) = interceptor_sub.next() => {

                    let msg: Message =
                        serde_json::from_slice(
                            &msg.payload,
                        )?;

                    self.handle_message(msg)
                        .await?;
                }

                Some(msg) = track_sub.next() => {

                    let msg: Message =
                        serde_json::from_slice(
                            &msg.payload,
                        )?;

                    self.handle_message(msg)
                        .await?;
                }

                Some(msg) = world_sub.next() => {

                    let threat: DetectedThreat =
                        serde_json::from_slice(
                            &msg.payload
                        )?;

                    println!(
                        "[{}] WORLD {} ({})",
                        self.state.platform.name,
                        threat.id,
                        threat.position.x,
                    );

                    if self.state.platform.position.distance(
                        &threat.position
                    ) <= self.state.platform.reach {

                        // Use detect_threat so that engagement logic is triggered
                        self.detect_threat(threat).await?;
                    }
                }

                Some(msg) = threat_sub.next() => {
                    let msg: Message =
                        serde_json::from_slice(
                            &msg.payload,
                        )?;

                    self.handle_message(msg)
                        .await?;
                }

                Some(msg) = engaged_sub.next() => {
                    let msg: Message =
                        serde_json::from_slice(
                            &msg.payload,
                        )?;

                    self.handle_message(msg)
                        .await?;
                }

                Some(msg) = neighbor_sub.next() => {
                    let msg: Message =
                        serde_json::from_slice(
                            &msg.payload,
                        )?;

                    self.handle_message(msg)
                        .await?;
                }

                Some(msg) = strategy_sub.next() => {
                    let msg: Message =
                        serde_json::from_slice(
                            &msg.payload,
                        )?;

                    self.handle_message(msg)
                        .await?;
                }
            }
        }
    }
}
