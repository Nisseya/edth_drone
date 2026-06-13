use uuid::Uuid;

pub const NEW_PLATFORM: &str = "vanguard.platform.new";

pub fn neighbor_subject(platform_id: &Uuid) -> String {
    format!("platform.{platform_id}.neighbor")
}

pub const WORLD_THREAT_DETECTED: &str = "world.threat.detected";
pub const THREAT_DESTROYED: &str = "vanguard.threat.destroyed";
