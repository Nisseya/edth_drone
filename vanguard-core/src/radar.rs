use std::collections::HashMap;
use std::time::Instant;

use uuid::Uuid;

use crate::control::PlatformSpec;
use crate::interceptor::{DetectedThreat, InterceptorReport, ThreatClassification};
use crate::position::{Position, Speed};
use crate::threat::Threat;

const MISSILE_SPEED: f64 = 300.0;

/// One platform's sensor: detects threats within `reach`, estimates their
/// velocity from successive sightings, and can only tell a real drone from a
/// decoy once the contact is within `classification_range`.
pub struct Radar {
    spec: PlatformSpec,
    classification_range: f64,
    last_seen: HashMap<Uuid, (Position, Instant)>,
}

impl Radar {
    pub fn new(spec: PlatformSpec, classification_range: f64) -> Self {
        Self {
            spec,
            classification_range,
            last_seen: HashMap::new(),
        }
    }

    pub fn spec(&self) -> &PlatformSpec {
        &self.spec
    }

    /// Builds this platform's radar report from the ground-truth threats.
    pub fn observe(&mut self, threats: &[Threat], now_ms: u64) -> InterceptorReport {
        let now = Instant::now();
        let mut contacts = Vec::new();

        for threat in threats {
            let range = self.spec.position.distance(&threat.position);
            if range > self.spec.reach {
                continue;
            }

            let speed = match self.last_seen.get(&threat.id) {
                Some((previous, at)) => {
                    let dt = now.duration_since(*at).as_secs_f64().max(1e-3);
                    Speed {
                        x: (threat.position.x - previous.x) / dt,
                        y: (threat.position.y - previous.y) / dt,
                    }
                }
                None => Speed { x: 0.0, y: 0.0 },
            };
            self.last_seen
                .insert(threat.id, (threat.position.clone(), now));

            let (classification, confidence) = if range <= self.classification_range {
                let class = if threat.is_decoy {
                    ThreatClassification::Decoy
                } else if threat.speed >= MISSILE_SPEED {
                    ThreatClassification::CruiseMissile
                } else {
                    ThreatClassification::Drone
                };
                (class, 0.95)
            } else {
                (ThreatClassification::Unknown, 0.3)
            };

            contacts.push(DetectedThreat {
                id: threat.id,
                position: threat.position.clone(),
                speed,
                threat_level: threat.threat_level,
                classification,
                confidence,
                detected_at: now_ms as f64 / 1000.0,
            });
        }

        // Forget tracks no longer present so the map doesn't grow unbounded.
        let alive: std::collections::HashSet<Uuid> = threats.iter().map(|t| t.id).collect();
        self.last_seen.retain(|id, _| alive.contains(id));

        InterceptorReport {
            platform_id: self.spec.id,
            name: self.spec.name.clone(),
            position: self.spec.position.clone(),
            reach: self.spec.reach,
            threats: contacts,
            interceptors_remaining: self.spec.ammo,
            timestamp: now_ms,
        }
    }
}
