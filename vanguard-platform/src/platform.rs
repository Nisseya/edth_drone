use std::time::Duration;

use anyhow::Result;
use async_nats::Client;
use futures::StreamExt;
use uuid::Uuid;

use crate::state::PlatformState;

const THREAT_DETECTED: &str = "vanguard.threat.detected";

const THREAT_ENGAGED: &str = "vanguard.threat.engaged";

const NEIGHBOR_UPDATE: &str = "vanguard.neighbor.update";

const STRATEGY_UPDATE: &str = "vanguard.strategy.update";

use vanguard_core::{
    DetectedThreat, InterceptorState, Message, NEW_PLATFORM, NeighborPlatform, Position,
    ThreatTrack, interceptor::TrackStatus,
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
        println!("[{}] DETECT {}", self.state.platform.name, threat.id);

        self.state.threats.insert(threat.id, threat.clone());

        self.publish(
            THREAT_DETECTED,
            Message::ThreatDetected {
                threat: threat.clone(),
                source_platform: self.state.platform.id,
            },
        )
        .await?;

        if self.is_best_platform(&threat) {
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
        self.state.engaged_threats.insert(threat_id);

        let interceptor_id = {
            let Some(interceptor) = self
                .state
                .platform
                .interceptors
                .iter_mut()
                .find(|i| matches!(i.state, InterceptorState::Idle))
            else {
                return Ok(());
            };

            interceptor.state = InterceptorState::Intercepting(threat_id);

            interceptor.id
        };

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

    async fn handle_threat_detected(
        &mut self,
        threat: DetectedThreat,
        source_platform: Uuid,
    ) -> Result<()> {
        self.state.threats.insert(threat.id, threat.clone());

        if source_platform == self.state.platform.id {
            return Ok(());
        }

        if self.is_best_platform(&threat) {
            self.engage_threat(threat.id).await?;
        }

        Ok(())
    }

    fn handle_threat_engaged(&mut self, threat_id: Uuid) {
        self.state.engaged_threats.insert(threat_id);
        self.state.threats.remove(&threat_id);
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

        self.state.tracks.insert(track.threat_id, track.clone());

        if track.status == TrackStatus::Engaged {
            self.state.engaged_threats.insert(track.threat_id);

            self.state.threats.remove(&track.threat_id);
        }

        Ok(())
    }

    async fn handle_message(&mut self, message: Message) -> Result<()> {
        match message {
            Message::ThreatDetected {
                threat,
                source_platform,
            } => {
                self.handle_threat_detected(threat, source_platform).await?;
            }

            Message::ThreatEngaged { threat_id, .. } => {
                self.handle_threat_engaged(threat_id);
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
            } => {}
            Message::ThreatDestroyed {
                threat_id,
                platform_id,
                interceptor_id,
            } => {}
        }

        Ok(())
    }

    pub async fn run(mut self) -> Result<()> {
        let mut threat_sub = self.nats.subscribe(THREAT_DETECTED).await?;

        let mut engaged_sub = self.nats.subscribe(THREAT_ENGAGED).await?;

        let subject = vanguard_core::neighbor_subject(&self.state.platform.id);

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

        loop {
            tokio::select! {

                _ = heartbeat.tick() => {
                    self.publish_neighbor_update()
                        .await?;
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
