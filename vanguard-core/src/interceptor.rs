use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::position::{Position, Speed};

#[derive(Clone, Debug)]
pub struct PlatformInterceptor {
    pub id: Uuid,
    pub name: String,
    pub position: Position,
    pub interceptors: Vec<Interceptor>,
    pub reach: f64,
    pub neighbor_platforms: Vec<NeighborPlatform>,
}

#[derive(Clone, Debug)]
pub struct NeighborPlatform {
    pub id: Uuid,
    pub position: Position,
    pub reach: f64,
    pub interceptors_remaining: usize,
}

// possibilité: étendre le lien par la suite
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Interceptor {
    pub id: Uuid,
    pub position: Position,
    pub state: InterceptorState,
    pub assigned_track: Option<Uuid>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum InterceptorState {
    Idle,
    MovingTo(Position),
    Searching(Uuid),
    Intercepting(Uuid),
    Destroyed,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct InterceptorReport {
    pub platform_id: Uuid,
    pub name: String,
    pub position: Position,
    pub reach: f64,
    pub threats: Vec<DetectedThreat>,
    pub interceptors_remaining: usize,
    pub timestamp: u64,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct DetectedThreat {
    pub id: Uuid,
    pub position: Position,
    pub speed: Speed,
    pub threat_level: usize,
    pub classification: ThreatClassification,
    pub confidence: f64,
    pub detected_at: f64,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub enum ThreatClassification {
    Unknown,
    Drone,
    FPVDrone,
    Helicopter,
    Aircraft,
    CruiseMissile,
    BallisticMissile,
    Friendly,
    Civilian,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ThreatTrack {
    pub threat_id: Uuid,
    pub position: Position,
    pub velocity: Speed,
    pub confidence: f64,
    pub threat_level: usize,
    pub last_update: f64,
    pub source_platforms: Vec<Uuid>,
    pub status: TrackStatus,
    pub engaged_by: Option<Uuid>,
}

impl ThreatTrack {
    pub fn predict_position(&self, dt: f64) -> Position {
        Position {
            x: self.position.x + self.velocity.x * dt,
            y: self.position.y + self.velocity.y * dt,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum TrackStatus {
    Detected,
    Engaged,
    Destroyed,
}
