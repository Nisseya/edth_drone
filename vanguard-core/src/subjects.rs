//! NATS subjects shared across every binary. All wire-protocol topics live
//! here so producers and consumers can never drift out of sync.

use uuid::Uuid;

// --- Platform / orchestrator event bus (`vanguard.*`) ---
pub const NEW_PLATFORM: &str = "vanguard.platform.new";
pub const THREAT_DETECTED: &str = "vanguard.threat.detected";
pub const THREAT_ENGAGED: &str = "vanguard.threat.engaged";
pub const THREAT_DESTROYED: &str = "vanguard.threat.destroyed";
pub const NEIGHBOR_UPDATE: &str = "vanguard.neighbor.update";
pub const TRACK_UPDATED: &str = "vanguard.track.updated";
pub const STRATEGY_UPDATE: &str = "vanguard.strategy.update";

// --- Interceptor agent (`vanguard.interceptor.*`) ---
pub const INTERCEPTOR_UPDATE: &str = "vanguard.interceptor.update";
pub const INTERCEPTOR_TARGET_ASSIGNED: &str = "vanguard.interceptor.target.assigned";
pub const INTERCEPTOR_OBSERVATION: &str = "vanguard.interceptor.observation";

// --- World feed ---
pub const WORLD_THREAT_DETECTED: &str = "world.threat.detected";

/// Per-platform subject carrying neighbour updates.
pub fn neighbor_subject(platform_id: &Uuid) -> String {
    format!("platform.{platform_id}.neighbor")
}
