//! Dark Jak's purple lightning (Jak 3, `target-darkjak.gc` and
//! `target-handler.gc`), on him and on whoever his blows strike.
//!
//! - On him while he is Dark Jak (`target-darkjak-process`, `darkjak-interp`
//!   at least half): a spark (part 173) at a random point of his body every
//!   frame; every 0.02 s at most, a quarter of the time the wall effect (an
//!   empty function in Jak 3, so nothing but its clock), else a tenth of
//!   the time a shock (`process-drawable-shock-effect`,
//!   `lightning-darkjak-transform`) out of one of his joints to what a probe
//!   meets 2 to 10 m off, with a glow (part 187) where it ends.
//! - On a victim (`target-send-attack`, then `target-darkjak-effect-trans`
//!   once for each one his attack strikes): every frame while it reacts to
//!   the blow (hit, knocked or dying) and for 15 s at most, a spark at a
//!   random point of it and its tint (`color-effect 'dark`); every 0.05 s,
//!   unless ten counts of lightning are out already, two counts more and,
//!   for the first half second, a bolt (`process-drawable2-shock-effect`,
//!   `lightning-darkjak-attack`) from one of his joints to one of its, with
//!   a glow at each end, after that a shock out of one of its joints. The
//!   count wears off one a frame.
//!
//! Each bolt is a `lightning-tracker` with `lightning-probe-callback`: it
//! lasts its spec's duration and some at random, its ends kept on the joints
//! they hang on, its sound where it ends; the particles draw it as Jak 3's
//! `lightning-draw` does (`particles::lightning`).
//!
//! The host says what still reacts and where its joints are drawn
//! (`ActorWorld::actor_reacting`, `actor_joints`), and draws the tints. A
//! body without a skeleton of at least seven joints (Minecraft's mobs) gets
//! what Jak 3 gives such a drawable: its sparks at random points of its
//! sphere, and no bolts (both shock effects need its nodes).
use glam::{Mat3, Vec3};

use crate::collide::CollideWorld;
use crate::math::{TICKS_PER_SECOND, seconds};
use crate::particles::{LightningRequest, Overrides};
use crate::projectile::ActorWorld;
use crate::{Event, Jak, METER};

/// The spark on a body (`hotdot`, purple).
const SPARK: u32 = 173;
/// The glow where a bolt meets something (`glow`, purple; its 188 after).
const GLOW: u32 = 187;
const ATTACK: &str = "lightning-darkjak-attack";
const TRANSFORM: &str = "lightning-darkjak-transform";
/// `process-drawable-shock-effect`'s reach (40960 units), how far off the
/// probe must meet something (8192), and its sphere (409.6).
const SHOCK_REACH: f32 = 10.0 * METER;
const SHOCK_MIN: f32 = 2.0 * METER;
const PROBE_RADIUS: f32 = 0.1 * METER;
/// `*lightning-probe-vars*`'s directions, tried from the last.
const PROBE_DIRS: [Vec3; 6] = [Vec3::Z, Vec3::X, Vec3::NEG_Z, Vec3::NEG_X, Vec3::Y, Vec3::NEG_Y];
/// A victim's lightning: at most 15 s, a bolt every 0.05 s, from him for
/// the first 0.5 s; ten counts out at most, two a bolt, one off a frame.
const VICTIM_MAX: f32 = 15.0;
const VICTIM_EVERY: f32 = 0.05;
const FROM_HIM: f32 = 0.5;
const COUNT_MAX: i32 = 10;
/// His own: every 0.02 s at most, the wall effect a quarter of the time,
/// else a bolt a tenth of it.
const SELF_EVERY: f32 = 0.02;
const SELF_WALL: f32 = 0.25;
const SELF_BOLT: f32 = 0.1;
/// `process-drawable-random-point!` takes a node of a body with this many
/// at least, else a point of its sphere.
const NODES_FOR_POINT: usize = 7;
/// Minecraft's tick: by then a struck mob shows its hurt (its server ticks
/// at 20 Hz), so a victim that does not react yet is not given up before
/// (in Jak 3 the blow's `'hit` comes before the effect's first frame).
const FIRST_REACTION: f32 = 0.05;

/// Where a bolt's end is: one of his joints, one of a victim's, or a fixed
/// point (`lightning-tracker`'s `target-joint` or `offset`).
#[derive(Clone, Copy, Debug, PartialEq)]
enum End {
    Joint(usize),
    VictimJoint(u64, usize),
    At(Vec3),
}

#[derive(Clone, Debug)]
struct Bolt {
    id: u64,
    spec: &'static str,
    from: End,
    to: End,
    start: i64,
    /// Its life with its fade, ticks.
    life: f32,
    /// Its ends as last resolved.
    at: (Vec3, Vec3),
}

#[derive(Clone, Debug)]
struct Victim {
    key: u64,
    start: i64,
    last_bolt: i64,
    reacted: bool,
    /// Its box (Jak space) and its joints as last seen.
    lo: Vec3,
    hi: Vec3,
    joints: Vec<Vec3>,
}

#[derive(Clone, Debug)]
pub struct DarkShock {
    /// `darkjak lightning-count`.
    pub lightning_count: i32,
    shock_effect_time: i64,
    victims: Vec<Victim>,
    bolts: Vec<Bolt>,
    next_bolt: u64,
    seed: u32,
    /// The host's camera is in his head (first person): none on him then
    /// (`focus-test? self in-head`).
    pub in_head: bool,
    /// This frame's tints on the victims: key and the dark colour effect's
    /// strength (0.2 to 1).
    pub tints: Vec<(u64, f32)>,
}

impl Default for DarkShock {
    fn default() -> Self {
        Self {
            lightning_count: 0,
            shock_effect_time: i64::MIN / 2,
            victims: Vec::new(),
            bolts: Vec::new(),
            next_bolt: 0,
            seed: 0x9e37_79b9,
            in_head: false,
            tints: Vec::new(),
        }
    }
}

impl DarkShock {
    fn random(&mut self) -> f32 {
        let s = &mut self.seed;
        *s ^= *s << 13;
        *s ^= *s >> 17;
        *s ^= *s << 5;
        (*s >> 8) as f32 / (1u32 << 24) as f32
    }

    /// `rand-vu-int-range lo hi`: lo to hi, both in.
    fn int_range(&mut self, lo: usize, hi: usize) -> usize {
        if hi <= lo {
            return lo;
        }
        lo + ((self.random() * (hi - lo + 1) as f32) as usize).min(hi - lo)
    }

    /// `rand-vu-sphere-point!` of radius `r`: a point inside the sphere.
    fn sphere_point(&mut self, r: f32) -> Vec3 {
        loop {
            let p = Vec3::new(self.random(), self.random(), self.random()) * 2.0 - Vec3::ONE;
            if p.length_squared() <= 1.0 {
                return p * r;
            }
        }
    }

    /// The victims under his lightning now.
    pub fn victims(&self) -> Vec<u64> {
        self.victims.iter().map(|v| v.key).collect()
    }

    /// How many bolts are out.
    pub fn bolts_out(&self) -> usize {
        self.bolts.len()
    }

    /// The bolts out this frame, for the particles.
    pub(crate) fn lightning_requests(&self, now: i64) -> Vec<LightningRequest> {
        self.bolts
            .iter()
            .map(|b| LightningRequest {
                id: 0xda00_0000_0000 ^ b.id,
                spec: b.spec,
                from: b.at.0,
                to: b.at.1,
                age: (now - b.start) as f32 / TICKS_PER_SECOND as f32,
                // Both `target-darkjak-effect-trans`'s and his own shocks
                // pass `lightning-probe-callback`.
                probe: true,
            })
            .collect()
    }
}

/// `lerp-scale a b x x0 x1`.
fn lerp_scale(a: f32, b: f32, x: f32, x0: f32, x1: f32) -> f32 {
    a + (b - a) * ((x - x0) / (x1 - x0)).clamp(0.0, 1.0)
}

impl Jak {
    /// `darkjak-interp`: going Dark, 0 to 1 over its `get-on-fast` clip's
    /// artist frames 5 to 15; changing back, 1 to 0 over the `get-off`
    /// clips' 10 to 60; else whether he is Dark Jak.
    fn dark_interp(&self) -> f32 {
        let aframe = self.chan.aframe_num(&self.anims);
        match self.dark_transition {
            Some(true) => lerp_scale(0.0, 1.0, aframe, 5.0, 15.0),
            Some(false) => lerp_scale(1.0, 0.0, aframe, 10.0, 60.0),
            None => f32::from(u8::from(self.dark)),
        }
    }

    fn joint_world(&self, j: usize) -> Vec3 {
        self.joints
            .get(j)
            .map_or(self.control.trans + Vec3::Y * METER, |&local| self.control.trans + self.control.quat * local)
    }

    fn end_at(&self, end: End) -> Option<Vec3> {
        match end {
            End::Joint(j) => Some(self.joint_world(j)),
            End::At(p) => Some(p),
            End::VictimJoint(key, j) => self.dark_shock.victims.iter().find(|v| v.key == key).and_then(|v| v.joints.get(j).copied()),
        }
    }

    /// A tracker of `spec` from `from` to `to` (its duration its spec's and
    /// some at random), the glow at `glows` lasting as long as it, its
    /// spec's sound where it ends.
    fn dark_bolt(&mut self, spec: &'static str, from: End, to: End, glows: &[Vec3]) {
        let (Some(a), Some(b)) = (self.end_at(from), self.end_at(to)) else {
            return;
        };
        self.dark_shock.next_bolt += 1;
        let id = self.dark_shock.next_bolt;
        let Some((duration, life)) = self
            .particles
            .as_ref()
            .and_then(|p| p.library().lightning_life(spec, 0xda00_0000_0000 ^ id))
            .or_else(|| (self.particles.is_none()).then_some((30.0, 30.0)))
        else {
            return;
        };
        if let Some(particles) = self.particles.as_mut() {
            for &at in glows {
                let mut carry = 0.0;
                particles.launch_var(GLOW, at, Mat3::IDENTITY, Overrides { timer: Some(duration), ..Default::default() }, &mut carry);
            }
        }
        self.dark_shock.bolts.push(Bolt { id, spec, from, to, start: self.time, life, at: (a, b) });
        // Its spec's `:sound`, as its tracker starts, at its end.
        let name = if spec == ATTACK { "STRETCHED_ZAP" } else { "TRANSFORM_ZAP" };
        self.events.push(Event::SoundAt { name, pos: b });
    }

    /// `process-drawable-shock-effect` from `from` (at `at`): a probe out
    /// 10 m along one of the six directions turned at random, the first to
    /// meet the world over 2 m off; a bolt to there, the glow at its end.
    fn dark_shock_out(&mut self, world: &mut dyn CollideWorld, spec: &'static str, from: End, at: Vec3) {
        let turn = Mat3::from_rotation_z(self.dark_shock.random() * std::f32::consts::TAU)
            * Mat3::from_rotation_y(self.dark_shock.random() * std::f32::consts::TAU)
            * Mat3::from_rotation_x(self.dark_shock.random() * std::f32::consts::TAU);
        for dir in PROBE_DIRS.iter().rev() {
            let motion = (turn * *dir).normalize_or(Vec3::Y) * SHOCK_REACH;
            self.probe_cache.fill_line_sphere(world, at, motion, PROBE_RADIUS);
            let Some(hit) = self.probe_cache.probe_line_sphere(at, motion, PROBE_RADIUS) else { continue };
            let end = hit.intersect;
            if end.distance(at) <= SHOCK_MIN {
                continue;
            }
            self.dark_bolt(spec, from, End::At(end), &[end]);
            return;
        }
    }

    /// `process-drawable-random-point!` on victim `i`: one of its joints
    /// from the fourth with enough of them, else a point of its sphere.
    fn victim_point(&mut self, i: usize) -> Vec3 {
        let (n, lo, hi) = {
            let v = &self.dark_shock.victims[i];
            (v.joints.len(), v.lo, v.hi)
        };
        if n >= NODES_FOR_POINT {
            let j = self.dark_shock.int_range(3, n - 1);
            return self.dark_shock.victims[i].joints[j];
        }
        let (centre, radius) = ((lo + hi) * 0.5, (hi - lo).length() * 0.5);
        centre + self.dark_shock.sphere_point(radius)
    }

    /// One frame of his lightning: his own, the victims' (new ones from
    /// this frame's blows, from event `first_event` on), and the bolts'
    /// ends moved with what they hang on.
    pub(crate) fn dark_shock_frame(
        &mut self,
        world: &mut dyn CollideWorld,
        mut actors: Option<&mut dyn ActorWorld>,
        first_event: usize,
    ) {
        let now = self.time;
        let joints = self.joints.len();
        // The count wears off a frame at a time (`seekl! ... 0 1`).
        self.dark_shock.lightning_count = (self.dark_shock.lightning_count - 1).max(0);
        // His blows as Dark Jak take their victims, once each.
        if self.dark && self.dark_transition.is_none() && self.attack.danger.is_some() {
            let struck: Vec<u64> = self.events[first_event.min(self.events.len())..]
                .iter()
                .filter_map(|e| match e {
                    Event::Strike { actor: Some(key), .. } => Some(*key),
                    _ => None,
                })
                .collect();
            for key in struck {
                if self.dark_shock.victims.iter().any(|v| v.key == key) {
                    continue;
                }
                let at = self.control.trans;
                self.dark_shock.victims.push(Victim { key, start: now, last_bolt: now, reacted: false, lo: at, hi: at, joints: Vec::new() });
            }
        }
        // His own sparks and shocks.
        if self.dark_interp() >= 0.5 && !self.dark_shock.in_head {
            let j = self.dark_shock.int_range(3, joints.saturating_sub(1));
            let at = self.joint_world(j);
            if let Some(p) = self.particles.as_mut() {
                p.launch(SPARK, at, Mat3::IDENTITY);
            }
            let due = now - self.dark_shock.shock_effect_time >= seconds(SELF_EVERY);
            if self.dark_shock.random() < SELF_WALL {
                if due {
                    self.dark_shock.shock_effect_time = now;
                }
            } else if self.dark_shock.random() < SELF_BOLT && due {
                self.dark_shock.shock_effect_time = now;
                // A joint up to half of his (`rand-vu-int-range 3 (/ (+ n -1) 2)`).
                let j = self.dark_shock.int_range(3, joints.saturating_sub(1) / 2);
                let at = self.joint_world(j);
                self.dark_shock_out(world, TRANSFORM, End::Joint(j), at);
            }
        }
        // The victims, while they react.
        self.dark_shock.tints.clear();
        let mut kept = Vec::new();
        for mut v in std::mem::take(&mut self.dark_shock.victims) {
            let age = (now - v.start) as f32 / TICKS_PER_SECOND as f32;
            match actors.as_deref_mut().and_then(|a| a.actor_reacting(v.key)) {
                Some((lo, hi)) => {
                    v.reacted = true;
                    v.lo = lo;
                    v.hi = hi;
                    v.joints = actors.as_deref_mut().map(|a| a.actor_joints(v.key)).unwrap_or_default();
                }
                None if !v.reacted && age < FIRST_REACTION => {}
                None => continue,
            }
            if age >= VICTIM_MAX {
                continue;
            }
            kept.push(v);
        }
        self.dark_shock.victims = kept;
        for i in 0..self.dark_shock.victims.len() {
            if !self.dark_shock.victims[i].reacted {
                continue;
            }
            let spark = self.victim_point(i);
            if let Some(p) = self.particles.as_mut() {
                p.launch(SPARK, spark, Mat3::IDENTITY);
            }
            let f = 0.2 + 0.8 * self.dark_shock.random();
            let key = self.dark_shock.victims[i].key;
            self.dark_shock.tints.push((key, f));
            if now - self.dark_shock.victims[i].last_bolt < seconds(VICTIM_EVERY) {
                continue;
            }
            self.dark_shock.victims[i].last_bolt = now;
            if self.dark_shock.lightning_count >= COUNT_MAX {
                continue;
            }
            self.dark_shock.lightning_count += 2;
            let nodes = self.dark_shock.victims[i].joints.len();
            let age = (now - self.dark_shock.victims[i].start) as f32 / TICKS_PER_SECOND as f32;
            if age < FROM_HIM {
                // `process-drawable2-shock-effect`: a joint of his and one
                // of its, each from the fourth (both skeletons needed).
                if nodes == 0 {
                    continue;
                }
                let mine = self.dark_shock.int_range(3, joints.saturating_sub(1));
                let its = self.dark_shock.int_range(3, nodes - 1).min(nodes - 1);
                let ends = (self.joint_world(mine), self.dark_shock.victims[i].joints[its]);
                self.dark_bolt(ATTACK, End::Joint(mine), End::VictimJoint(key, its), &[ends.0, ends.1]);
            } else {
                // `color-effect 'shock` on him for 0.2 s (his
                // `target-color-effect-process`: a random bright flicker),
                // then `process-drawable-shock-effect` out of one of its
                // joints up to half of them.
                self.hurt_fx.shock_until = self.hurt_fx.shock_until.max(now + seconds(0.2));
                // (Its nodes needed.)
                if nodes == 0 {
                    continue;
                }
                let its = self.dark_shock.int_range(3, (nodes - 1) / 2).min(nodes - 1);
                let at = self.dark_shock.victims[i].joints[its];
                self.dark_shock_out(world, ATTACK, End::VictimJoint(key, its), at);
            }
        }
        // The bolts' ends follow what they hang on; spent ones go.
        let mut bolts = std::mem::take(&mut self.dark_shock.bolts);
        bolts.retain(|b| ((now - b.start) as f32) < b.life);
        for b in &mut bolts {
            if let Some(a) = self.end_at(b.from) {
                b.at.0 = a;
            }
            if let Some(c) = self.end_at(b.to) {
                b.at.1 = c;
            }
        }
        self.dark_shock.bolts = bolts;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::math::FRAME_TICKS;

    /// Reacting bodies: key, box and joints.
    struct Reacting(Vec<(u64, Vec3, Vec3, Vec<Vec3>)>);
    impl ActorWorld for Reacting {
        fn actor_hit(&mut self, _: Vec3, _: Vec3, _: f32) -> Option<(u64, f32, Vec3)> {
            None
        }
        fn actor_reacting(&mut self, key: u64) -> Option<(Vec3, Vec3)> {
            self.0.iter().find(|b| b.0 == key).map(|b| (b.1, b.2))
        }
        fn actor_joints(&mut self, key: u64) -> Vec<Vec3> {
            self.0.iter().find(|b| b.0 == key).map(|b| b.3.clone()).unwrap_or_default()
        }
    }

    fn dark_jak() -> Jak {
        let mut jak = Jak::new(Vec3::ZERO, 0.0);
        jak.dark = true;
        jak.dark_eco = 100.0;
        jak.joints = (0..40).map(|j| Vec3::Y * j as f32 * 100.0).collect();
        jak.danger_set(Some(crate::attack::Danger::Punch));
        jak
    }

    fn strike(jak: &mut Jak, key: u64) -> usize {
        let first = jak.events.len();
        jak.events.push(Event::Strike { actor: Some(key), pos: Vec3::ZERO, dir: Vec3::Z, damage: 1.0 });
        first
    }

    fn frame(jak: &mut Jak, actors: &mut Reacting) {
        jak.time += FRAME_TICKS;
        let n = jak.events.len();
        jak.dark_shock_frame(&mut crate::EmptyWorld, Some(actors), n);
    }

    fn body(key: u64) -> (u64, Vec3, Vec3, Vec<Vec3>) {
        let joints = (0..20).map(|j| Vec3::new(3.0 * METER, j as f32 * 200.0, 0.0)).collect();
        (key, Vec3::new(2.0 * METER, 0.0, -METER), Vec3::new(4.0 * METER, 2.0 * METER, METER), joints)
    }

    /// A victim stays under his lightning while it reacts to the blow, for
    /// 15 s at most; tinted each frame; bolts from his joints to its
    /// joints, their sound at its end; never over ten counts out.
    #[test]
    fn a_victim_is_shocked_while_it_reacts_and_15_s_at_most() {
        let mut jak = dark_jak();
        let mut actors = Reacting(vec![body(7)]);
        let first = strike(&mut jak, 7);
        jak.dark_shock_frame(&mut crate::EmptyWorld, Some(&mut actors), first);
        assert_eq!(jak.dark_shock.victims(), vec![7]);
        assert_eq!(jak.dark_shock.tints.len(), 1);
        assert!((0.2..=1.0).contains(&jak.dark_shock.tints[0].1));
        // Struck again: still one victim.
        let first = strike(&mut jak, 7);
        jak.dark_shock_frame(&mut crate::EmptyWorld, Some(&mut actors), first);
        assert_eq!(jak.dark_shock.victims(), vec![7]);
        let mut zaps = 0;
        for _ in 0..20 {
            frame(&mut jak, &mut actors);
            assert!(jak.dark_shock.lightning_count <= COUNT_MAX);
            for e in &jak.events {
                if let Event::SoundAt { name: "STRETCHED_ZAP", pos } = e {
                    zaps += 1;
                    assert!((pos.x - 3.0 * METER).abs() < 1.0, "at one of its joints: {pos:?}");
                }
            }
            jak.events.clear();
        }
        assert!(zaps > 0, "bolts from him to it");
        for b in &jak.dark_shock.bolts {
            assert!(matches!(b.to, End::VictimJoint(7, j) if j >= 3), "{:?}", b.to);
        }
        // Up again (no longer reacting): let go, untinted.
        actors.0.clear();
        frame(&mut jak, &mut actors);
        assert!(jak.dark_shock.victims().is_empty());
        assert!(jak.dark_shock.tints.is_empty());
        // Reacting on and on: let go at 15 s.
        let mut actors = Reacting(vec![body(7)]);
        let first = strike(&mut jak, 7);
        jak.dark_shock_frame(&mut crate::EmptyWorld, Some(&mut actors), first);
        for _ in 0..(15 * 60 + 2) {
            frame(&mut jak, &mut actors);
        }
        assert!(jak.dark_shock.victims().is_empty(), "15 s at most");
    }

    /// A body without nodes (a Minecraft mob): its sparks and tint, no
    /// bolts.
    #[test]
    fn a_body_without_a_skeleton_gets_no_bolts() {
        let mut jak = dark_jak();
        let mut actors = Reacting(vec![(9, Vec3::ZERO, Vec3::splat(METER), Vec::new())]);
        let first = strike(&mut jak, 9);
        jak.dark_shock_frame(&mut crate::EmptyWorld, Some(&mut actors), first);
        for _ in 0..10 {
            frame(&mut jak, &mut actors);
        }
        assert_eq!(jak.dark_shock.victims(), vec![9]);
        assert!(!jak.dark_shock.tints.is_empty());
        assert!(jak.dark_shock.bolts.iter().all(|b| !matches!(b.to, End::VictimJoint(..))));
    }

    /// Only Dark Jak's blows take victims; one not reacting by Minecraft's
    /// tick is let go.
    #[test]
    fn only_his_dark_blows_take_victims() {
        let mut jak = dark_jak();
        jak.dark = false;
        let mut actors = Reacting(Vec::new());
        let first = strike(&mut jak, 3);
        jak.dark_shock_frame(&mut crate::EmptyWorld, Some(&mut actors), first);
        assert!(jak.dark_shock.victims().is_empty(), "not as Jak");
        jak.dark = true;
        let first = strike(&mut jak, 3);
        jak.dark_shock_frame(&mut crate::EmptyWorld, Some(&mut actors), first);
        assert_eq!(jak.dark_shock.victims(), vec![3], "waits a tick for its hurt");
        for _ in 0..4 {
            frame(&mut jak, &mut actors);
        }
        assert!(jak.dark_shock.victims().is_empty(), "never reacted: let go");
    }

    /// The count wears off a frame at a time; `lerp-scale` as Jak 3's.
    #[test]
    fn the_lightning_count_wears_off_a_frame_at_a_time() {
        let mut jak = Jak::new(Vec3::ZERO, 0.0);
        jak.dark_shock.lightning_count = 3;
        jak.dark_shock_frame(&mut crate::EmptyWorld, None, 0);
        assert_eq!(jak.dark_shock.lightning_count, 2);
        assert_eq!(lerp_scale(0.0, 1.0, 10.0, 5.0, 15.0), 0.5);
        assert_eq!(lerp_scale(1.0, 0.0, 70.0, 10.0, 60.0), 0.0);
    }
}
