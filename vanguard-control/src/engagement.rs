//! Engagement layer: assigns platforms/in-flight interceptors to confirmed real
//! threats (Hungarian, with hysteresis for dynamic re-tasking), flies them to a
//! predicted intercept point, and diverts aborted ones to a safe drop zone.

use std::collections::{HashMap, HashSet};

use pathfinding::kuhn_munkres::kuhn_munkres;
use pathfinding::matrix::Matrix;
use uuid::Uuid;
use vanguard_core::{Engagement, FlyingInterceptor, Position, Radar, Speed, Threat, predicted_intercept};

const INTERCEPTOR_SPEED: f64 = 800.0;
const HIT_RADIUS: f64 = 400.0;
const MAX_IN_FLIGHT: usize = 3;
const REACHABLE_BASE: i64 = 100_000;
const UNREACHABLE: i64 = -1_000_000;
/// Bonus keeping an in-flight interceptor on its current target — avoids
/// flip-flopping between near-equal targets every tick.
const HYST_BONUS: i64 = 5_000;
/// Designated safe drop zone (metres, local frame): empty area NE of the city.
pub const SAFE_ZONE: Position = Position { x: 40_000.0, y: 40_000.0 };

enum Assignment {
    /// Chasing a threat. `locked` = forced by the operator (excluded from auto re-task).
    Target { id: Uuid, locked: bool },
    /// Aborted: heading to the safe zone, self-destructs on arrival.
    Divert,
}

struct Shot {
    id: Uuid,
    position: Position,
    assignment: Assignment,
}

struct Engager {
    ammo: usize,
    shots: Vec<Shot>,
}

#[derive(Default)]
pub struct Engagements {
    engagers: HashMap<Uuid, Engager>,
    last_pos: HashMap<Uuid, Position>,
    pub neutralized: usize,
}

impl Engagements {
    pub fn reset(&mut self) {
        self.engagers.clear();
        self.last_pos.clear();
        self.neutralized = 0;
    }

    pub fn sync(&mut self, radars: &HashMap<Uuid, Radar>) {
        self.engagers.retain(|id, _| radars.contains_key(id));
        for (id, radar) in radars {
            self.engagers
                .entry(*id)
                .or_insert(Engager { ammo: radar.spec().ammo, shots: Vec::new() });
        }
    }

    /// Operator override: lock interceptor `iid` onto threat `tid`.
    pub fn retarget(&mut self, iid: Uuid, tid: Uuid) {
        if let Some(shot) = self.shot_mut(iid) {
            shot.assignment = Assignment::Target { id: tid, locked: true };
        }
    }

    /// Operator override: abort interceptor `iid` → divert to the safe zone.
    pub fn abort(&mut self, iid: Uuid) {
        if let Some(shot) = self.shot_mut(iid) {
            shot.assignment = Assignment::Divert;
        }
    }

    fn shot_mut(&mut self, iid: Uuid) -> Option<&mut Shot> {
        self.engagers.values_mut().flat_map(|e| e.shots.iter_mut()).find(|s| s.id == iid)
    }

    pub fn step(
        &mut self,
        radars: &HashMap<Uuid, Radar>,
        threats: &[Threat],
        engageable: &HashSet<Uuid>,
        dt: f64,
        time_scale: f64,
    ) -> Vec<Uuid> {
        self.sync(radars);

        let mut vel: HashMap<Uuid, Speed> = HashMap::new();
        if dt > 0.0 {
            for t in threats {
                if let Some(prev) = self.last_pos.get(&t.id) {
                    vel.insert(
                        t.id,
                        Speed { x: (t.position.x - prev.x) / dt, y: (t.position.y - prev.y) / dt },
                    );
                }
            }
        }
        self.last_pos = threats.iter().map(|t| (t.id, t.position.clone())).collect();

        let alive: HashSet<Uuid> = threats.iter().map(|t| t.id).collect();
        // Lost target (killed / leaked) → abort to the safe zone.
        for eng in self.engagers.values_mut() {
            for shot in &mut eng.shots {
                if let Assignment::Target { id, .. } = shot.assignment {
                    if !alive.contains(&id) {
                        shot.assignment = Assignment::Divert;
                    }
                }
            }
        }

        self.retask(radars, threats, engageable);

        // Advance + resolve.
        let by_id: HashMap<Uuid, &Threat> = threats.iter().map(|t| (t.id, t)).collect();
        let int_speed = INTERCEPTOR_SPEED * time_scale.max(0.0);
        let step = int_speed * dt;
        let mut destroyed = Vec::new();

        for eng in self.engagers.values_mut() {
            eng.shots.retain_mut(|shot| match shot.assignment {
                Assignment::Divert => {
                    if shot.position.distance(&SAFE_ZONE) <= step + HIT_RADIUS {
                        return false; // reached safe zone → self-destruct, no kill
                    }
                    shot.position = shot.position.step_toward(&SAFE_ZONE, step);
                    true
                }
                Assignment::Target { id, .. } => {
                    let Some(threat) = by_id.get(&id) else { return false };
                    if shot.position.distance(&threat.position) <= step + HIT_RADIUS {
                        destroyed.push(id);
                        return false;
                    }
                    let v = vel.get(&id).cloned().unwrap_or(Speed { x: 0.0, y: 0.0 });
                    let aim = predicted_intercept(&shot.position, int_speed, &threat.position, &v)
                        .unwrap_or_else(|| threat.position.clone());
                    shot.position = shot.position.step_toward(&aim, step);
                    true
                }
            });
        }
        self.neutralized += destroyed.len();
        destroyed
    }

    /// Re-optimise assignments over {in-flight movers + free tubes} × engageable
    /// threats. Movers keep their target unless another is clearly better (hysteresis).
    fn retask(&mut self, radars: &HashMap<Uuid, Radar>, threats: &[Threat], engageable: &HashSet<Uuid>) {
        // Threats already held by a locked interceptor are off the table.
        let locked_targets: HashSet<Uuid> = self
            .engagers
            .values()
            .flat_map(|e| &e.shots)
            .filter_map(|s| match s.assignment {
                Assignment::Target { id, locked: true } => Some(id),
                _ => None,
            })
            .collect();

        // Rows: movers (unlocked in-flight shots) then free tubes.
        // A mover row = (platform_id, shot_id, current_target, position).
        let mut movers: Vec<(Uuid, Uuid, Uuid, Position)> = Vec::new();
        let mut tubes: Vec<Uuid> = Vec::new();
        for (pid, e) in &self.engagers {
            for s in &e.shots {
                if let Assignment::Target { id, locked: false } = s.assignment {
                    movers.push((*pid, s.id, id, s.position.clone()));
                }
            }
            let capacity = MAX_IN_FLIGHT.saturating_sub(e.shots.len()).min(e.ammo);
            for _ in 0..capacity {
                tubes.push(*pid);
            }
        }

        let targets: Vec<&Threat> = threats
            .iter()
            .filter(|t| engageable.contains(&t.id) && !locked_targets.contains(&t.id))
            .collect();

        let rowc = movers.len() + tubes.len();
        if rowc == 0 || targets.is_empty() {
            return;
        }

        let n = rowc.max(targets.len());
        let rows: Vec<Vec<i64>> = (0..n)
            .map(|i| {
                (0..n)
                    .map(|j| {
                        let threat = match targets.get(j) {
                            Some(t) => *t,
                            None => return 0, // dummy column = "no target"
                        };
                        if i < movers.len() {
                            let (_, _, cur, pos) = &movers[i];
                            mover_score(pos, threat, *cur)
                        } else if i - movers.len() < tubes.len() {
                            self.tube_score(radars, &tubes[i - movers.len()], threat)
                        } else {
                            0 // dummy row
                        }
                    })
                    .collect()
            })
            .collect();
        let (_, assignment) = kuhn_munkres(&Matrix::from_rows(rows).expect("square"));

        // Apply: movers get their (possibly new) target or divert if unmatched/infeasible;
        // matched free tubes launch a new shot.
        for (i, &j) in assignment.iter().enumerate() {
            let threat = targets.get(j).copied();
            if i < movers.len() {
                let (_, sid, _, pos) = &movers[i];
                let new = threat.filter(|t| mover_score(pos, t, t.id) > 0).map(|t| t.id);
                let sid = *sid;
                if let Some(shot) = self.shot_mut(sid) {
                    shot.assignment = match new {
                        Some(tid) => Assignment::Target { id: tid, locked: false },
                        None => Assignment::Divert, // no worthwhile target left
                    };
                }
            } else {
                let ti = i - movers.len();
                let (Some(&pid), Some(t)) = (tubes.get(ti), threat) else { continue };
                let Some(radar) = radars.get(&pid) else { continue };
                if radar.spec().position.distance(&t.position) > radar.spec().reach {
                    continue;
                }
                if let Some(eng) = self.engagers.get_mut(&pid) {
                    if eng.ammo == 0 {
                        continue;
                    }
                    eng.shots.push(Shot {
                        id: Uuid::new_v4(),
                        position: radar.spec().position.clone(),
                        assignment: Assignment::Target { id: t.id, locked: false },
                    });
                    eng.ammo -= 1;
                }
            }
        }
    }

    fn tube_score(&self, radars: &HashMap<Uuid, Radar>, pid: &Uuid, threat: &Threat) -> i64 {
        let Some(radar) = radars.get(pid) else { return UNREACHABLE };
        let spec = radar.spec();
        let d = spec.position.distance(&threat.position);
        if d > spec.reach {
            return UNREACHABLE;
        }
        REACHABLE_BASE + (threat.threat_level as i64) * 1000 - (d as i64) / 10
    }

    pub fn ammo(&self, platform_id: &Uuid) -> usize {
        self.engagers.get(platform_id).map_or(0, |e| e.ammo)
    }

    pub fn lines(&self) -> Vec<Engagement> {
        self.engagers
            .iter()
            .flat_map(|(pid, e)| {
                e.shots.iter().filter_map(move |s| match s.assignment {
                    Assignment::Target { id, .. } => Some(Engagement { platform_id: *pid, threat_id: id }),
                    Assignment::Divert => None,
                })
            })
            .collect()
    }

    pub fn interceptors(&self) -> Vec<FlyingInterceptor> {
        self.engagers
            .values()
            .flat_map(|e| {
                e.shots.iter().map(|s| FlyingInterceptor {
                    id: s.id,
                    position: s.position.clone(),
                    target_id: match s.assignment {
                        Assignment::Target { id, .. } => id,
                        Assignment::Divert => Uuid::nil(),
                    },
                    diverting: matches!(s.assignment, Assignment::Divert),
                })
            })
            .collect()
    }
}

/// Value of an in-flight interceptor at `pos` engaging `threat`; `current` is
/// its present target (gets the hysteresis bonus). Reachable if a PIP exists.
fn mover_score(pos: &Position, threat: &Threat, current: Uuid) -> i64 {
    let base = REACHABLE_BASE + (threat.threat_level as i64) * 1000 - (pos.distance(&threat.position) as i64) / 10;
    if threat.id == current { base + HYST_BONUS } else { base }
}
