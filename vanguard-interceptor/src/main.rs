mod agent;
mod state;
mod subjects;

use std::collections::HashMap;

use anyhow::Result;
use async_nats::connect;
use uuid::Uuid;

use agent::InterceptorAgent;
use state::InterceptorRuntimeState;

use vanguard_core::{Interceptor, InterceptorState, Position};

#[tokio::main]
async fn main() -> Result<()> {
    let nats = connect("nats://localhost:4222").await?;

    let interceptor = Interceptor {
        id: Uuid::new_v4(),
        position: Position { x: 0.0, y: 0.0 },
        state: InterceptorState::Idle,
        assigned_track: None,
    };

    let state = InterceptorRuntimeState {
        interceptor,
        platform_id: Uuid::nil(),
        target_id: None,
        tracks: HashMap::new(),
    };

    let agent = InterceptorAgent::new(state, nats);

    agent.run().await
}
