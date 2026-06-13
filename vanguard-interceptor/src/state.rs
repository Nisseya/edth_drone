use std::collections::HashMap;

use uuid::Uuid;

use vanguard_core::{Interceptor, ThreatTrack};

pub struct InterceptorRuntimeState {
    pub interceptor: Interceptor,
    pub platform_id: Uuid,
    pub target_id: Option<Uuid>,
    pub tracks: HashMap<Uuid, ThreatTrack>,
}
