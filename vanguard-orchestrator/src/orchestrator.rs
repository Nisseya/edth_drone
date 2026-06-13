use anyhow::Result;
use async_nats::Client;
use futures::StreamExt;
use uuid::Uuid;

use crate::{
    assignment::compute_assignments,
    state::{InterceptorInfo, OrchestratorState},
    subjects::*,
    tracks::{cleanup_tracks, update_track},
};

use vanguard_core::{
    Assignment, DetectedThreat, Message, NEW_PLATFORM, NeighborPlatform, THREAT_DESTROYED, interceptor::TrackStatus
};

pub struct Orchestrator {
    pub state: OrchestratorState,
    pub nats: Client,
}

impl Orchestrator {
    pub fn new(nats: Client) -> Self {
        Self {
            state: OrchestratorState::new(),
            nats,
        }
    }

    async fn publish(&self, subject: &'static str, msg: Message) -> Result<()> {
        println!("[{}] PUB {} {:?}", "Orchestrator", subject, msg);
        self.nats
            .publish(subject, serde_json::to_vec(&msg)?.into())
            .await?;

        Ok(())
    }

    async fn publish_strategy(&self, assignments: Vec<Assignment>) -> Result<()> {
        self.publish(STRATEGY_UPDATE, Message::StrategyUpdate { assignments })
            .await
    }

    pub async fn run(mut self) -> Result<()> {
        let mut threat_sub = self.nats.subscribe(THREAT_DETECTED).await?;
        let mut destroyed_sub =
            self.nats
                .subscribe(THREAT_DESTROYED)
                .await?;

        let mut neighbor_sub = self.nats.subscribe(NEIGHBOR_UPDATE).await?;

        let mut interceptor_sub = self.nats.subscribe(INTERCEPTOR_UPDATE).await?;
        let mut new_platform_sub = self.nats.subscribe(NEW_PLATFORM).await?;
        let mut engaged_sub = self.nats.subscribe(THREAT_ENGAGED).await?;

        let mut heartbeat = tokio::time::interval(std::time::Duration::from_secs(1));

        loop {
            tokio::select! {

                _ = heartbeat.tick() => {

                    cleanup_tracks(
                        &mut self.state.tracks,
                        0.0,
                    );

                    let tracks =
                        self.state
                            .tracks
                            .values()
                            .map(|t| t.track.clone())
                            .collect::<Vec<_>>();

                    let interceptors =
                        self.state
                            .interceptors
                            .values()
                            .cloned()
                            .collect::<Vec<_>>();

                    let assignments =
                        compute_assignments(
                            &tracks,
                            &interceptors,
                        );

                    self.publish_strategy(
                        assignments,
                    )
                    .await?;
                }

                Some(msg) = destroyed_sub.next() => {

                    let msg: Message =
                        serde_json::from_slice(
                            &msg.payload
                        )?;

                    if let Message::ThreatDestroyed {
                        threat_id,
                        ..
                    } = msg {

                        let updated_track =
                            if let Some(track) =
                                self.state.tracks.get_mut(&threat_id)
                            {
                                track.track.status =
                                    TrackStatus::Destroyed;

                                Some(track.track.clone())
                            } else {
                                None
                            };

                        if let Some(track) = updated_track {

                            self.publish(
                                TRACK_UPDATED,
                                Message::TrackUpdated {
                                    track,
                                },
                            )
                            .await?;
                        }

                        self.state.tracks.remove(
                            &threat_id
                        );
                    }
                }

                Some(msg) =
                    threat_sub.next() =>
                {
                    let msg: Message =
                        serde_json::from_slice(
                            &msg.payload
                        )?;

                    if let Message::ThreatDetected {
                        threat,
                        source_platform,
                    } = msg {

                        let track =
                            update_track(
                                &mut self.state.tracks,
                                threat,
                                source_platform,
                            );

                        self.publish(
                            TRACK_UPDATED,
                            Message::TrackUpdated {
                                track,
                            },
                        )
                        .await?;
                    }
                }

                Some(msg) = engaged_sub.next() => {

                    let msg: Message =
                        serde_json::from_slice(
                            &msg.payload
                        )?;

                    if let Message::ThreatEngaged {
                        threat_id,
                        platform_id,
                        ..
                    } = msg {

                        let updated_track =
                            if let Some(track) =
                                self.state.tracks.get_mut(&threat_id)
                            {
                                track.track.status =
                                    TrackStatus::Engaged;

                                track.track.engaged_by =
                                    Some(platform_id);

                                println!(
                                    "[Orchestrator] Track {} engaged by {}",
                                    threat_id,
                                    platform_id
                                );

                                Some(track.track.clone())
                            } else {
                                None
                            };

                        if let Some(track) = updated_track {
                            self.publish(
                                TRACK_UPDATED,
                                Message::TrackUpdated {
                                    track,
                                },
                            )
                            .await?;
                        }
                    }
                }

                Some(msg) = neighbor_sub.next() => {
                    let msg: Message =
                        serde_json::from_slice(&msg.payload)?;

                    if let Message::NeighborUpdate {
                        platform_id,
                        position,
                        reach,
                        interceptors_remaining,
                    } = msg {

                        if let Some(platform) =
                            self.state.platforms.get_mut(&platform_id)
                        {
                            platform.position = position;
                            platform.interceptors_remaining =
                                interceptors_remaining;
                        }
                    }
                }


                Some(msg) = new_platform_sub.next() => {

                    let msg: Message =
                        serde_json::from_slice(
                            &msg.payload
                        )?;

                    if let Message::NewPlatform {
                        platform_id,
                        position,
                        reach,
                    } = msg {

                        let neighbors: Vec<_> = self
                            .state
                            .platforms
                            .values()
                            .cloned()
                            .filter(|other| {
                                other.position.distance(&position)
                                    <= (other.reach + reach) as f64
                            })
                            .collect();

                        let new_platform =
                            NeighborPlatform {
                                id: platform_id,
                                position: position.clone(),
                                reach: reach,
                                interceptors_remaining: 0,
                            };

                        self.state.platforms.insert(
                            platform_id,
                            new_platform,
                        );

                        for neighbor in neighbors {

                            let msg =
                                Message::NeighborUpdate {
                                    platform_id,
                                    position: position.clone(),
                                    reach: reach,
                                    interceptors_remaining: 0,
                                };

                            self.nats
                                .publish(
                                    vanguard_core::neighbor_subject(
                                        &neighbor.id
                                    ),
                                    serde_json::to_vec(
                                        &msg
                                    )?
                                    .into(),
                                )
                                .await?;

                            let msg =
                                Message::NeighborUpdate {
                                    platform_id:
                                        neighbor.id,
                                    position:
                                        neighbor.position,
                                    reach:
                                        neighbor.reach,
                                    interceptors_remaining:
                                        neighbor
                                            .interceptors_remaining,
                                };

                            self.nats
                                .publish(
                                    vanguard_core::neighbor_subject(
                                        &platform_id
                                    ),
                                    serde_json::to_vec(
                                        &msg
                                    )?
                                    .into(),
                                )
                                .await?;
                        }
                    }
                }

                Some(msg) =
                    interceptor_sub.next() =>
                {
                    let msg: Message =
                        serde_json::from_slice(
                            &msg.payload
                        )?;

                    if let Message::InterceptorUpdate {
                        platform_id,
                        interceptor,
                    } = msg {

                        self.state
                            .interceptors
                            .insert(
                                interceptor.id,
                                InterceptorInfo {
                                    platform_id,
                                    interceptor,
                                },
                            );
                    }
                }
            }
        }
    }
}
