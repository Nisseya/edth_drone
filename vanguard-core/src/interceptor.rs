use uuid::Uuid;

use crate::position::Position;

#[derive(Clone, Debug)]
pub struct PlatformInterceptor {
    pub id: Uuid,
    pub name: String,
    pub position: Position,
    pub interceptors: Vec<Interceptor>,
    pub range: f64,
}

#[derive(Clone, Debug)]
pub struct Interceptor {
    pub id: Uuid,
    pub position: Position,
    pub state: InterceptorState,
}

#[derive(Clone, Debug)]
pub enum InterceptorState {
    Idle,
    MovingTo(Position),
    Intercepting(Uuid),
    Destroyed,
}

#[derive(Clone, Debug)]
pub struct InterceptorReport {
    pub platform_id: Uuid,
    pub threats: Vec<DetectedThreat>,
    pub interceptors_remaining: usize,
    pub timestamp: u64,
}

#[derive(Clone, Debug)]
pub struct DetectedThreat {
    pub id: Uuid,
    pub position: Position,
    pub threat_level: usize,
}
