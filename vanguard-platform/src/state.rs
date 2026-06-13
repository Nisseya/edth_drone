use std::collections::{HashMap, HashSet};

use uuid::Uuid;

use vanguard_core::{DetectedThreat, PlatformInterceptor, ThreatTrack};

pub struct PlatformState {
    pub platform: PlatformInterceptor,
    pub threats: HashMap<Uuid, DetectedThreat>,
    pub engaged_threats: HashSet<Uuid>,
    pub tracks: HashMap<Uuid, ThreatTrack>,
}

impl PlatformState {
    pub fn new(platform: PlatformInterceptor) -> Self {
        Self {
            platform,
            threats: HashMap::new(),
            engaged_threats: HashSet::new(),
            tracks: HashMap::new(),
        }
    }
}
