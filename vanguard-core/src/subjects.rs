use uuid::Uuid;

pub const NEW_PLATFORM: &str = "vanguard.platform.new";

pub fn neighbor_subject(platform_id: &Uuid) -> String {
    format!("platform.{platform_id}.neighbor")
}
