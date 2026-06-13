use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::position::Position;

/// Live-tunable swarm/simulation parameters, published by the UI.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MapConfig {
    pub decoy_ratio: f64,
    pub swarm_min: usize,
    pub swarm_max: usize,
    pub spawn_interval_s: f64,
    pub zone_radius: f64,
    pub max_active: usize,
}

impl Default for MapConfig {
    fn default() -> Self {
        Self {
            decoy_ratio: 0.4,
            swarm_min: 6,
            swarm_max: 12,
            spawn_interval_s: 45.0,
            zone_radius: 6_000.0,
            max_active: 40,
        }
    }
}

/// A platform the control host should run, defined from the UI.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PlatformSpec {
    pub id: Uuid,
    pub name: String,
    pub position: Position,
    pub reach: f64,
    pub ammo: usize,
}

/// UI → map: update simulation parameters.
pub const MAP_CONFIG: &str = "control.map.config";
/// UI → control host: add a platform.
pub const PLATFORM_ADD: &str = "control.platform.add";
/// UI → control host: remove a platform (payload = platform id string).
pub const PLATFORM_REMOVE: &str = "control.platform.remove";
/// UI → map + host: reset to the baseline scenario (default config, preset
/// platforms, cleared threats). Payload is ignored.
pub const CONTROL_RESET: &str = "control.reset";
