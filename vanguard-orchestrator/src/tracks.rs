use std::collections::HashMap;

use uuid::Uuid;

use crate::kalman::KalmanTrack;
use crate::state::InternalTrack;

use vanguard_core::{DetectedThreat, ThreatTrack, interceptor::TrackStatus};

const MATCH_DISTANCE: f64 = 50.0;

pub fn update_track(
    tracks: &mut HashMap<Uuid, InternalTrack>,
    threat: DetectedThreat,
    source_platform: Uuid,
) -> ThreatTrack {
    if let Some(internal) = tracks.get_mut(&threat.id) {
        internal.kalman.update(threat.position.x, threat.position.y);

        let (x, y) = internal.kalman.position();
        let (vx, vy) = internal.kalman.velocity();

        internal.track.position.x = x;
        internal.track.position.y = y;

        internal.track.velocity.x = vx;
        internal.track.velocity.y = vy;

        internal.track.confidence = internal.track.confidence.max(threat.confidence);

        internal.track.threat_level = internal.track.threat_level.max(threat.threat_level);

        internal.track.last_update = threat.detected_at;

        if !internal.track.source_platforms.contains(&source_platform) {
            internal.track.source_platforms.push(source_platform);
        }

        return internal.track.clone();
    }

    for internal in tracks.values_mut() {
        let (x, y) = internal.kalman.position();

        let dx = x - threat.position.x;
        let dy = y - threat.position.y;

        if (dx * dx + dy * dy).sqrt() < MATCH_DISTANCE {
            internal.kalman.update(threat.position.x, threat.position.y);

            let (x, y) = internal.kalman.position();
            let (vx, vy) = internal.kalman.velocity();

            internal.track.position.x = x;
            internal.track.position.y = y;

            internal.track.velocity.x = vx;
            internal.track.velocity.y = vy;

            internal.track.confidence = internal.track.confidence.max(threat.confidence);

            internal.track.threat_level = internal.track.threat_level.max(threat.threat_level);

            internal.track.last_update = threat.detected_at;

            if !internal.track.source_platforms.contains(&source_platform) {
                internal.track.source_platforms.push(source_platform);
            }

            return internal.track.clone();
        }
    }

    let track = ThreatTrack {
        threat_id: threat.id,
        position: threat.position.clone(),
        velocity: threat.speed.clone(),
        confidence: threat.confidence,
        threat_level: threat.threat_level,
        last_update: threat.detected_at,
        source_platforms: vec![source_platform],
        status: TrackStatus::Detected,
        engaged_by: None,
    };

    tracks.insert(
        threat.id,
        InternalTrack {
            kalman: KalmanTrack::new(
                threat.position.x,
                threat.position.y,
                threat.speed.x,
                threat.speed.y,
            ),
            track: track.clone(),
        },
    );

    track
}

pub fn cleanup_tracks(tracks: &mut HashMap<Uuid, InternalTrack>, now: f64) {
    tracks.retain(|_, track| now - track.track.last_update < 10.0);
}
