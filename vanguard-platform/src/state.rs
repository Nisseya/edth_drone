use std::collections::{HashMap, HashSet};

use uuid::Uuid;

use vanguard_core::{DetectedThreat, Interceptor, PlatformInterceptor, ThreatTrack};

use crate::kalman::KalmanTrack;

pub struct TrackedThreat {
    pub track: ThreatTrack,
    pub kalman: KalmanTrack,
}

pub struct PlatformState {
    pub platform: PlatformInterceptor,
    pub threats: HashMap<Uuid, DetectedThreat>,
    pub engaged_threats: HashSet<Uuid>,
    pub tracks: HashMap<Uuid, TrackedThreat>,
    pub known_interceptors: HashMap<Uuid, Interceptor>,
}

impl PlatformState {
    pub fn new(platform: PlatformInterceptor) -> Self {
        Self {
            platform,
            threats: HashMap::new(),
            engaged_threats: HashSet::new(),
            tracks: HashMap::new(),
            known_interceptors: HashMap::new(),
        }
    }
}
