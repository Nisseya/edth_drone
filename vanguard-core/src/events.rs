use uuid::Uuid;
pub type PlatformId = Uuid;
pub type InterceptorId = Uuid;
pub type ThreatId = Uuid;
use crate::{DetectedThreat, Interceptor, Position, ThreatTrack};

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub enum Message {
    // généré par plateformes
    // intercepteurs pas implémentés
    // consommé par plateformes et orchestrateur
    ThreatDetected {
        threat: DetectedThreat,
        source_platform: PlatformId,
    },

    // consommé par plateformes et orchestrateur
    // généré par plateformes
    // plateformes qui consomment: Calculent les distances et tirent si nécessaire,
    // orchestrateur: met à jour et avertit tout le monde
    ThreatEngaged {
        threat_id: ThreatId,
        platform_id: PlatformId,
        interceptor_id: InterceptorId,
    },

    // Généré par les plateformes
    // Uniquement pour les plateformes?
    NeighborUpdate {
        platform_id: PlatformId,
        position: Position,
        reach: f64,
        interceptors_remaining: usize,
    },

    // généré par plateforme
    // Pour orchestrateur
    // l'orchestrateur va ensuite communiquer à tous les vrais voisins cette nouvelle plateforme
    NewPlatform {
        platform_id: PlatformId,
        position: Position,
        reach: f64,
    },

    // de l'orchestrateur vers les plateformes requises
    StrategyUpdate {
        assignments: Vec<Assignment>,
    },

    // evenement de la plateforme vers les voisins et l'orchestrateur
    TrackUpdated {
        track: ThreatTrack,
    },

    //mise à jour de l'interceptor
    // qui va le consommer?
    InterceptorUpdate {
        platform_id: PlatformId,
        interceptor: Interceptor,
    },

    ThreatDestroyed {
        threat_id: ThreatId,
        platform_id: PlatformId,
        interceptor_id: InterceptorId,
    },
    InterceptorTargetAssigned {
        interceptor_id: Uuid,
        threat_id: Uuid,
    },
    InterceptorObservation {
        interceptor_id: Uuid,
        threat: DetectedThreat,
    },
    InterceptorLaunched {
        interceptor_id: Uuid,
        threat_id: Uuid,
    },
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct Assignment {
    pub platform_id: Uuid,
    pub interceptor_id: Uuid,
    pub track_id: Uuid,
}
