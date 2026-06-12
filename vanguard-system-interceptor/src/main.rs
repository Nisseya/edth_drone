mod cli;

use clap::Parser;
use uuid::Uuid;
use vanguard_core::{Interceptor, InterceptorState, PlatformInterceptor, Position};

use crate::cli::Args;

const DETECTION_RANGE: f64 = 1_500.0;

fn main() {
    let args = Args::parse();
    let position = Position { x: args.x, y: args.y };

    let interceptors: Vec<Interceptor> = (0..args.interceptors)
        .map(|_| Interceptor {
            id: Uuid::new_v4(),
            position: position.clone(),
            state: InterceptorState::Idle,
        })
        .collect();

    let platform = PlatformInterceptor {
        id: Uuid::new_v4(),
        name: args.name,
        position,
        interceptors,
        range: DETECTION_RANGE,
    };

    println!(
        "{} (id {}) online at ({:.0}, {:.0}) — range {:.0} m, {} interceptor(s) ready",
        platform.name,
        platform.id,
        platform.position.x,
        platform.position.y,
        platform.range,
        platform.interceptors.len(),
    );
    for interceptor in &platform.interceptors {
        println!("  interceptor {} idle", interceptor.id);
    }
}
