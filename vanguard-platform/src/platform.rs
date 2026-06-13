use std::{collections::HashMap, time::Duration};

use anyhow::Result;
use async_nats::Client;
use futures::StreamExt;
use vanguard_interceptor::{InterceptorAgent, InterceptorRuntimeState};

use crate::state::PlatformState;

use vanguard_core::{InterceptorState, Message, subjects::*};

pub struct Platform {
    pub state: PlatformState,
    pub nats: Client,
}

impl Platform {
    pub fn new(state: PlatformState, nats: Client) -> Self {
        Self { state, nats }
    }

    pub(crate) async fn publish(&self, subject: &'static str, msg: Message) -> Result<()> {
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

    /// Announce this platform to the orchestrator so neighbours get linked up.
    async fn announce(&self) -> Result<()> {
        self.publish(
            NEW_PLATFORM,
            Message::NewPlatform {
                platform_id: self.state.platform.id,
                position: self.state.platform.position.clone(),
                reach: self.state.platform.reach,
            },
        )
        .await
    }

    /// Spawn one autonomous agent per physical interceptor.
    fn spawn_interceptors(&self) {
        for interceptor in self.state.platform.interceptors.clone() {
            let agent = InterceptorAgent::new(
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
    }

    /// Per-second tick: advance every Kalman track and republish the local picture.
    async fn tick(&mut self) -> Result<()> {
        for tracked in self.state.tracks.values_mut() {
            tracked.kalman.predict(1.0);

            let (x, y) = tracked.kalman.position();
            let (vx, vy) = tracked.kalman.velocity();

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

        self.publish_neighbor_update().await?;

        for track in tracks {
            self.publish(TRACK_UPDATED, Message::TrackUpdated { track })
                .await?;
        }

        Ok(())
    }

    pub async fn run(mut self) -> Result<()> {
        let mut threat_sub = self.nats.subscribe(THREAT_DETECTED).await?;
        let mut interceptor_sub = self.nats.subscribe(INTERCEPTOR_UPDATE).await?;
        let mut engaged_sub = self.nats.subscribe(THREAT_ENGAGED).await?;
        let mut destroyed_sub = self.nats.subscribe(THREAT_DESTROYED).await?;
        let mut world_sub = self.nats.subscribe(WORLD_THREAT_DETECTED).await?;
        let mut track_sub = self.nats.subscribe(TRACK_UPDATED).await?;
        let mut neighbor_sub = self
            .nats
            .subscribe(vanguard_core::neighbor_subject(&self.state.platform.id))
            .await?;
        let mut strategy_sub = self.nats.subscribe(STRATEGY_UPDATE).await?;

        let mut heartbeat = tokio::time::interval(Duration::from_secs(1));

        self.announce().await?;
        self.spawn_interceptors();

        loop {
            tokio::select! {
                _ = heartbeat.tick() => self.tick().await?,
                Some(m) = destroyed_sub.next() => self.handle_message(serde_json::from_slice(&m.payload)?).await?,
                Some(m) = interceptor_sub.next() => self.handle_message(serde_json::from_slice(&m.payload)?).await?,
                Some(m) = track_sub.next() => self.handle_message(serde_json::from_slice(&m.payload)?).await?,
                Some(m) = world_sub.next() => self.on_world_threat(&m.payload).await?,
                Some(m) = threat_sub.next() => self.handle_message(serde_json::from_slice(&m.payload)?).await?,
                Some(m) = engaged_sub.next() => self.handle_message(serde_json::from_slice(&m.payload)?).await?,
                Some(m) = neighbor_sub.next() => self.handle_message(serde_json::from_slice(&m.payload)?).await?,
                Some(m) = strategy_sub.next() => self.handle_message(serde_json::from_slice(&m.payload)?).await?,
            }
        }
    }
}
