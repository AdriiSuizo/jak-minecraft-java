//! Jak 1's yellow eco shot (`projectile-yellow`, `projectiles.gc`), fired in
//! place of his punch while he carries yellow eco (`target-yellow-blast`,
//! `target2.gc`; Jak 3's own punch plays, Jak 1's `eichar` clips being for
//! Jak 1's body). It leaves his hand at `yellow-projectile-speed` (60 m/s)
//! along his facing, flattened, skims the ground (pushed down every frame,
//! lifted back over the floor: options bit 0 and `projectile-method-24`),
//! slides along what it meets (`projectile-collision-reaction`), homes in
//! on the nearest creature ahead (`projectile-method-28`,
//! `projectile-update-velocity-space-wars`, its `tween` growing to 1), and
//! bursts on the first creature it touches (`max-hits` 1) or once it is
//! stuck, else fades after four seconds. Its blow is the punch's.
use crate::collide::CollideWorld;
use crate::math::*;
use crate::projectile::ActorWorld;
use crate::{Event, Jak};
use glam::Vec3;

/// `*TARGET-bank*` `yellow-projectile-speed`.
pub const SPEED: f32 = 245760.0;
/// `projectile-init-by-other` `timeout`.
pub const LIFE: i64 = seconds(4.0);
/// `yellow-attack-timeout`: how soon after one shot the next.
pub const ATTACK_TIMEOUT: i64 = seconds(0.2);
/// Its collide sphere (`projectile-method-26`): this wide, its middle as
/// high over its root (which skims the floor).
const RADIUS: f32 = 5324.8;
/// Pushed down each frame (options bit 0), lifted to this over the floor.
const DROP: f32 = -40960.0;
const OVER_FLOOR: f32 = 1228.8;
/// `projectile-method-27`: below the hand it leaves.
const BELOW_HAND: f32 = 5324.8;

/// What became of a shot this frame, for the host's effects and sounds
/// (`group-part-yellow-eco-fireball-launcher`, `-fireball`, `-fireball-hit`;
/// `yellow-fire`, `yellow-buzz`, `yellow-explode`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum BlastEvent {
    Launch { id: u32, pos: Vec3, dir: Vec3 },
    Burst { id: u32, pos: Vec3, actor: Option<u64> },
    Fade { id: u32, pos: Vec3 },
}

#[derive(Clone, Debug)]
pub struct EcoBlast {
    pub id: u32,
    pub trans: Vec3,
    pub transv: Vec3,
    spawn_time: i64,
    /// The point it homes on, the creature it chose, and where it would go
    /// with none (`target`, `last-target`, `target-base`).
    target: Vec3,
    target_base: Vec3,
    last_target: Option<u64>,
    target_count: u32,
    /// Where it was fired from and which way (`parent-base`, `base-vector`).
    parent_base: Vec3,
    base_vector: Vec3,
    tween: f32,
    old_dist: [f32; 16],
    old_dist_count: usize,
    touched_wall: bool,
}

/// `find-nearest-attackable`: the nearest creature within `radius` whose
/// direction from `from` lies within `cone` (rotation units, 65536 a turn)
/// of `dir` on the ground.
fn nearest(actors: &mut dyn ActorWorld, from: Vec3, radius: f32, dir: Vec3, cone: f32) -> Option<(u64, Vec3)> {
    let flat = Vec3::new(dir.x, 0.0, dir.z).normalize_or_zero();
    let max_angle = cone / 65536.0 * std::f32::consts::TAU;
    actors
        .actors_near(from, radius)
        .into_iter()
        .filter(|(_, at)| {
            let to = Vec3::new(at.x - from.x, 0.0, at.z - from.z).normalize_or_zero();
            to != Vec3::ZERO && flat.dot(to).clamp(-1.0, 1.0).acos() <= max_angle
        })
        .min_by(|a, b| a.1.distance_squared(from).total_cmp(&b.1.distance_squared(from)))
}

impl EcoBlast {
    /// Its sphere's middle (`root-prim prim-core world-sphere`), where its
    /// fireball burns.
    pub fn center(&self) -> Vec3 {
        self.trans + Vec3::Y * RADIUS
    }

    fn new(id: u32, hand: Vec3, facing: Vec3, now: i64) -> Self {
        let dir = Vec3::new(facing.x, 0.0, facing.z).normalize_or(Vec3::Z);
        let transv = dir * SPEED;
        let trans = hand - Vec3::Y * BELOW_HAND;
        let target = trans + transv * 2.0;
        Self {
            id,
            trans,
            transv,
            spawn_time: now,
            target,
            target_base: target,
            last_target: None,
            target_count: 0,
            parent_base: hand,
            base_vector: dir,
            tween: 0.05,
            old_dist: [4095996000.0; 16],
            old_dist_count: 0,
            touched_wall: false,
        }
    }

    /// `projectile-method-28`: whom it homes on.
    fn choose_target(&mut self, actors: Option<&mut dyn ActorWorld>) {
        let live = self.last_target.and_then(|key| {
            actors.and_then(|a| a.actors_near(self.trans, 409600.0).into_iter().find(|(k, _)| *k == key))
        });
        match live {
            Some((_, at)) => {
                self.target = at;
                let flat = Vec3::new(at.x - self.trans.x, 0.0, at.z - self.trans.z).length();
                if flat < 20480.0 && (at.y - self.trans.y).abs() > 24576.0 {
                    self.last_target = None;
                }
            }
            None => self.target = self.target_base,
        }
        self.target_count += 1;
    }

    /// The first frame's choice, from where it was fired.
    fn first_target(&mut self, actors: &mut dyn ActorWorld) {
        let mut choice = nearest(actors, self.parent_base, 409600.0, self.base_vector, 8192.0);
        if let Some(c) = nearest(actors, self.parent_base, 163840.0, self.base_vector, 8192.0) {
            choice = Some(c);
        }
        if let Some(c) = nearest(actors, self.parent_base, 28672.0, self.base_vector, 16384.0) {
            choice = Some(c);
        }
        if let Some((key, at)) = choice {
            self.last_target = Some(key);
            self.target = at;
        }
        self.target_count += 1;
    }

    /// `projectile-update-velocity-space-wars`; false when it turned away
    /// from a target it no longer has (`projectile-dissipate`).
    fn update_velocity(&mut self) -> bool {
        let to = self.target - self.trans;
        let want = to.normalize_or_zero();
        let along = self.transv.normalize_or_zero();
        let speed = self.transv.length();
        if self.last_target.is_none() && want.dot(along) < 0.0 {
            return false;
        }
        let turned = slerp_dir(along, want, self.tween);
        self.transv = turned * speed + to;
        if self.transv.length() > SPEED {
            self.transv = self.transv.normalize() * SPEED;
        }
        self.transv.y = DROP;
        true
    }
}

/// `vector-deg-slerp`: from `a` towards `b` by `t` of the angle between.
fn slerp_dir(a: Vec3, b: Vec3, t: f32) -> Vec3 {
    if a == Vec3::ZERO || b == Vec3::ZERO {
        return if b == Vec3::ZERO { a } else { b };
    }
    let angle = a.dot(b).clamp(-1.0, 1.0).acos();
    if angle < 1e-5 {
        return b;
    }
    let s = angle.sin();
    (a * ((1.0 - t) * angle).sin() / s + b * (t * angle).sin() / s).normalize_or(b)
}

impl Jak {
    /// Square with yellow eco: a shot from his right hand along his facing.
    pub(crate) fn fire_eco_blast(&mut self) {
        let facing = z_axis(self.control.quat);
        // His right hand (`sk_rhand`) when his body is posed; before that, about where it is.
        let hand = self.joints.get(crate::lightjak::JOINT_RHAND).map(|&local| self.control.trans + self.control.quat * local)
            .unwrap_or(self.control.trans + Vec3::Y * 1.3 * METER + facing * 0.4 * METER);
        self.eco_blast_count = self.eco_blast_count.wrapping_add(1);
        let blast = EcoBlast::new(self.eco_blast_count, hand, facing, self.time);
        self.events.push(Event::EcoBlast(BlastEvent::Launch { id: blast.id, pos: hand, dir: blast.base_vector }));
        self.eco_blasts.push(blast);
    }

    /// Every shot in flight, a frame.
    pub(crate) fn step_eco_blasts(&mut self, world: &mut dyn CollideWorld, mut actors: Option<&mut dyn ActorWorld>) {
        let now = self.time;
        let mut cache = std::mem::take(&mut self.probe_cache);
        let punch = crate::attack::Danger::Punch.damage();
        let mut i = 0;
        while i < self.eco_blasts.len() {
            let b = &mut self.eco_blasts[i];
            let mut ended = None;
            if now - b.spawn_time >= LIFE {
                ended = Some(BlastEvent::Fade { id: b.id, pos: b.center() });
            } else {
                if b.target_count == 0 {
                    if let Some(a) = actors.as_mut() {
                        b.first_target(&mut **a);
                    } else {
                        b.target_count = 1;
                    }
                } else {
                    b.choose_target(crate::projectile::reborrow(&mut actors));
                }
                if !b.update_velocity() {
                    ended = Some(BlastEvent::Fade { id: b.id, pos: b.center() });
                }
            }
            if ended.is_none() {
                // Options bit 2: the homing grows firmer, faster near.
                let near = b.trans.distance(b.target);
                let rate = if near < 20480.0 { 3.0 } else if near < 40960.0 { 1.0 } else { 0.5 };
                b.tween = (b.tween + rate * SECONDS_PER_FRAME).min(1.0);
                // The sphere rides its own height over the root.
                let start = b.center();
                let motion = b.transv * SECONDS_PER_FRAME;
                let hit = actors.as_mut().and_then(|a| a.actor_hit(start, motion, RADIUS));
                // Against the world: up to two slides a frame.
                let (mut at, mut left) = (start, motion);
                b.touched_wall = false;
                for _ in 0..2 {
                    if left.length_squared() < 1e-6 {
                        break;
                    }
                    cache.fill_line_sphere(world, at, left, RADIUS);
                    match cache.resolve_moving_sphere(at, RADIUS, left, -1.0, true) {
                        Some(h) => {
                            at += left * h.u;
                            let n = (at - h.intersect).normalize_or(h.normal);
                            if n.y.abs() < 0.5 {
                                b.touched_wall = true;
                            }
                            // `vector-reflect-flat-above!`: what goes into it, gone.
                            let rest = left * (1.0 - h.u);
                            left = rest - n * rest.dot(n).min(0.0);
                            b.transv -= n * b.transv.dot(n).min(0.0);
                        }
                        None => {
                            at += left;
                            break;
                        }
                    }
                }
                match hit {
                    Some((key, u, _)) if u <= 1.0 => {
                        let center = start + motion * u;
                        b.trans = center - Vec3::Y * RADIUS;
                        self.events.push(Event::Strike { actor: Some(key), pos: center, dir: b.base_vector, damage: punch });
                        ended = Some(BlastEvent::Burst { id: b.id, pos: center, actor: Some(key) });
                    }
                    _ => {
                        b.old_dist[b.old_dist_count] = 0.0625 * start.distance(at);
                        b.old_dist_count = (b.old_dist_count + 1) & 15;
                        b.trans = at - Vec3::Y * RADIUS;
                        // `projectile-method-24`: its root never under the floor.
                        let root = b.trans;
                        cache.fill_line_sphere(world, root + Vec3::Y * 8192.0, Vec3::Y * -90112.0, 409.6);
                        if let Some(g) = cache.resolve_moving_sphere(root + Vec3::Y * 8192.0, 409.6, Vec3::Y * -90112.0, -1.0, true)
                            && b.trans.y < g.intersect.y
                        {
                            b.trans.y = g.intersect.y + OVER_FLOOR;
                        }
                        let moved: f32 = b.old_dist.iter().sum();
                        if (b.touched_wall && moved < 0.5 * METER) || moved < 0.05 * METER {
                            ended = Some(BlastEvent::Burst { id: b.id, pos: b.center(), actor: None });
                        }
                    }
                }
            }
            match ended {
                Some(event) => {
                    self.events.push(Event::EcoBlast(event));
                    self.eco_blasts.swap_remove(i);
                }
                None => i += 1,
            }
        }
        self.probe_cache = cache;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct One(Vec3);
    impl ActorWorld for One {
        fn actor_hit(&mut self, start: Vec3, motion: Vec3, radius: f32) -> Option<(u64, f32, Vec3)> {
            // A sphere of 1 m round the creature.
            let r = radius + METER;
            let d = motion;
            let f = start - self.0;
            let (a, b, c) = (d.dot(d), 2.0 * f.dot(d), f.dot(f) - r * r);
            let disc = b * b - 4.0 * a * c;
            if a <= 0.0 || disc < 0.0 {
                return None;
            }
            let u = (-b - disc.sqrt()) / (2.0 * a);
            (0.0..=1.0).contains(&u).then_some((7, u, start + d * u))
        }
        fn actors_near(&mut self, center: Vec3, radius: f32) -> Vec<(u64, Vec3)> {
            if center.distance(self.0) <= radius { vec![(7, self.0)] } else { Vec::new() }
        }
    }

    /// Fired with a creature off to one side ahead, the shot turns to it and
    /// bursts on it with the punch's blow; with none, it fades in 4 s.
    #[test]
    fn the_shot_homes_on_a_creature_ahead_and_fades_without_one() {
        let mut jak = Jak::new(Vec3::ZERO, 0.0);
        jak.fire_eco_blast();
        // A floor 1 km across at height 0, as the shot skims it.
        let m = 500.0 * METER;
        let (a, b2, c, d) = (Vec3::new(-m, 0.0, -m), Vec3::new(-m, 0.0, m), Vec3::new(m, 0.0, m), Vec3::new(m, 0.0, -m));
        let mut world = crate::collide::TriangleGrid::new(
            vec![crate::collide::Tri { v: [a, b2, c], pat: Default::default() }, crate::collide::Tri { v: [a, c, d], pat: Default::default() }],
            4.0 * METER,
        );
        let mut target = One(Vec3::new(6.0 * METER, 1.0 * METER, 30.0 * METER));
        let mut struck = None;
        for _ in 0..120 {
            jak.time += FRAME_TICKS;
            jak.step_eco_blasts(&mut world, Some(&mut target));
            if let Some(e) = jak.events.iter().find_map(|e| match e {
                Event::Strike { actor, damage, .. } => Some((*actor, *damage)),
                _ => None,
            }) {
                struck = Some(e);
                break;
            }
        }
        assert_eq!(struck, Some((Some(7), crate::attack::Danger::Punch.damage())));
        assert!(jak.eco_blasts.is_empty());

        jak.events.clear();
        jak.fire_eco_blast();
        let mut faded = false;
        for _ in 0..(LIFE / FRAME_TICKS + 2) {
            jak.time += FRAME_TICKS;
            jak.step_eco_blasts(&mut world, None);
            faded |= jak.events.iter().any(|e| matches!(e, Event::EcoBlast(BlastEvent::Fade { .. })));
        }
        assert!(faded && jak.eco_blasts.is_empty());
    }
}
