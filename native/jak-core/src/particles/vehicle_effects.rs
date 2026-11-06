//! A vehicle's engine particles, run the way the game runs them: once a 60 Hz
//! frame, at the vehicle's bone transform of that frame.
//!
//! - `vehicle-method-78` (levels/city/traffic/vehicle/vehicle-effects.gc): the
//!   exhaust's smoke (part 923) with the number, alpha and velocity the engine
//!   power sets, and the damaged vehicle's smoke and sparks (parts 925, 928, 930
//!   and 921);
//! - `hvehicle-method-78` (levels/factory/car/hvehicle-effects.gc) and
//!   `vehicle-draw-thruster`: the two rear thruster flames (part 922, three crossed
//!   blades each);
//! - `h-bike-base`'s `vehicle-method-78` (bike.gc): a flame, by the same `vehicle-draw-thruster`, at each roll
//!   thruster that fires;
//! - `vehicle-method-78`'s scrape: the sparks (part 920) at the point of the impact while the scrape is loud;
//! - `vehicle-method-135`: the puff at each exhaust as the engine starts
//!   (`group-vehicle-engine-start`).
//!
//! Every position is the part's local offset from the vehicle's data
//! ([`Specs`]) put through the bone transform, so the particles are born where
//! the vehicle is drawn.
use super::vehicle_lights::Specs;
use super::{Overrides, Particles};
use glam::{Mat3, Quat, Vec3};

const THRUSTER: u32 = 922;
const EXHAUST: u32 = 923;
const SCRAPE: u32 = 920;
const SPARK: u32 = 921;
const SMOKE_HEAVY: u32 = 925;
const SMOKE_MEDIUM: u32 = 928;
const SMOKE_LIGHT: u32 = 930;
const ENGINE_START: &str = "group-vehicle-engine-start";
const METER: f32 = 4096.0;
const DT: f32 = 1.0 / 60.0;
/// `part-vel` is in units a 300 Hz tick.
const TICKS: f32 = 300.0;
/// 182.04445 rotation units to the degree.
const ANGLE: f32 = std::f32::consts::TAU / 65536.0;

/// The flames of the h-bikes' roll thrusters (bike.gc `vehicle-method-78`): where each hangs on the bike (units), the
/// direction the thruster is aimed (`vector-rotate*!` of this by the bone transform, it is the `forward` of
/// `forward-up->quaternion`), and the width and length the flame is drawn to (units).
pub const ROLL_FLAMES: [(Vec3, Vec3); 2] = [
    (Vec3::new(2048.0, -1228.8, 2457.6), Vec3::new(-0.707, 0.707, 0.0)),
    (Vec3::new(-2048.0, -1228.8, 2457.6), Vec3::new(0.707, 0.707, 0.0)),
];
pub const ROLL_FLAME_WIDTH: f32 = 1638.4;
pub const ROLL_FLAME_LENGTH: f32 = 12288.0;

/// One frame of the vehicle, in the particles' own space (Jak units).
#[derive(Clone, Copy, Debug, Default)]
pub struct Frame {
    /// `node-list data 0 bone transform`: where the vehicle is and how it faces.
    pub trans: Vec3,
    pub rotation: Quat,
    /// `rbody lin-velocity`, units a second.
    pub velocity: Vec3,
    /// `vehicle-flag ignition`: the engine is on.
    pub ignition: bool,
    /// `vehicle-flag riding`: someone is aboard.
    pub riding: bool,
    /// `engine-thrust`: the throttle after the engine's intake limit (negative reversing).
    pub engine_thrust: f32,
    /// `hit-points`, 1 whole to 0 destroyed.
    pub hit_points: f32,
    /// Each roll thruster's thrust as the flame is given it: `roll-thrust * force-scale * inv-mass / gravity`.
    pub roll_thrust: [f32; 2],
    /// `scrape-sound-envelope`, and `impact-pos` (where the vehicle last hit something).
    pub scrape: f32,
    pub impact_pos: Vec3,
}

/// What persists from frame to frame: the parts' `birthaccum`s and the vehicle's `power-level`.
#[derive(Clone, Debug, Default)]
pub struct Emitter {
    exhaust: [f32; 2],
    smoke: [f32; 2],
    /// `power-level`.
    pub power_level: f32,
    ignited: bool,
    rng: u64,
    /// The scrape sparks' `birthaccum`.
    scrape: f32,
    /// A spark of a wrecked vehicle was born in the last [`Self::step`]: `vehicle-method-78` plays "damage-zaps".
    pub zapped: bool,
    /// The power fluctuation popped in the last [`Self::step`]: `hvehicle`'s method 38 plays "damage-pops".
    pub popped: bool,
}

fn seek(value: f32, target: f32, delta: f32) -> f32 {
    value + (target - value).clamp(-delta, delta)
}

/// `power-fluctuation-factor` by hit-points (`vehicle-method-93`).
pub fn fluctuation(hit_points: f32) -> f32 {
    match hit_points {
        h if h >= 0.9 => 0.01,
        h if h >= 0.75 => 0.02,
        h if h >= 0.5 => 0.04,
        h if h >= 0.3 => 0.08,
        h if h >= 0.15 => 0.16,
        h if h >= 0.05 => 0.32,
        _ => 0.5,
    }
}

impl Emitter {
    pub fn new(seed: u64) -> Self {
        Self {
            rng: seed | 1,
            ..Default::default()
        }
    }

    /// A vehicle first seen already running did not just start: it gets no start puff.
    pub fn adopt(&mut self, running: bool) {
        self.ignited = running;
    }

    /// `rand-vu`.
    fn random(&mut self) -> f32 {
        let mut x = self.rng;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.rng = x;
        (x.wrapping_mul(0x2545_F491_4F6C_DD1D) >> 40) as f32 / 16_777_216.0
    }

    fn range(&mut self, low: f32, high: f32) -> f32 {
        low + (high - low) * self.random()
    }


    /// One 60 Hz frame: `vehicle-method-93`'s `power-level`, then the particles of
    /// `vehicle-method-78` (the vehicle's and the hvehicle's), launched into `particles`.
    pub fn step(&mut self, spec: &Specs, frame: &Frame, particles: &mut Particles) {
        self.zapped = false;
        let fluctuation = fluctuation(frame.hit_points);
        // hvehicle-effects.gc rigid-body-object-method-38: `(< (rand-vu) power-fluctuation-factor)`.
        self.popped = self.random() < fluctuation;
        let mut level = 0.0;
        if frame.ignition {
            level = (1.0 - self.random() * fluctuation).powi(2);
            if !frame.riding {
                level /= 2.0;
            }
        }
        self.power_level += (level - self.power_level) * ((1.0 + 50.0 * fluctuation) * DT).min(1.0);

        let rotation = Mat3::from_quat(frame.rotation);
        let world = |local: Vec3| frame.trans + rotation * local;
        if frame.ignition && !self.ignited {
            self.start(spec, frame, particles);
        }
        self.ignited = frame.ignition;

        // `engine-power-factor`: |engine-thrust| (vehicle-method-93), seeking the speed over
        // 15 m on the way to the particles (hvehicle's post).
        let speed = frame.velocity.length();
        let power = seek(frame.engine_thrust.abs(), speed / (15.0 * METER), 6.0 * DT);

        if frame.ignition {
            // The part's `:num` (its first value) and `:a`, set before the launch.
            let number = 6.0 * (0.25 + power) * self.random();
            let fade = 1.0 - power * power;
            let push = spec.max_engine_thrust / 5.0 * (0.5 + power);
            let count = spec.exhaust.len().min(spec.exhaust_dir.len()).min(2);
            for i in 0..count {
                let direction = rotation * spec.exhaust_dir[i];
                let velocity = (frame.velocity + direction * push) / TICKS;
                // stand-in: alpha 0 (the engine at full power) births nothing that could be seen.
                if fade <= 0.0 {
                    continue;
                }
                particles.launch_var(
                    EXHAUST,
                    world(spec.exhaust[i]),
                    Mat3::IDENTITY,
                    Overrides {
                        num: Some(number),
                        alpha: Some((16.0 * fade, 48.0 * fade)),
                        vel: Some(velocity),
                        ..Default::default()
                    },
                    &mut self.exhaust[i],
                );
            }
        }

        if frame.hit_points < 0.75 {
            self.smoke(spec, frame, particles, &rotation);
        }

        // vehicle-method-78: while the scrape is loud (envelope 0.75 or more), sparks at the impact.
        if frame.scrape >= 0.75 {
            particles.launch_var(SCRAPE, frame.impact_pos, Mat3::IDENTITY, Overrides::default(), &mut self.scrape);
        }

        // hvehicle-method-78: `vehicle-draw-thruster` at each `thruster-local-pos`, the thruster aimed along the
        // vehicle's forward and its blades turned 30 degrees about it (`quaternion-rotate-local-z!` 5461.3335 of the root
        // quaternion).
        if spec.flame_length > 0.0 {
            let thrust = (self.power_level * frame.engine_thrust).max(0.0);
            let forward = rotation * Vec3::Z;
            let turned = rotation * Vec3::new(-0.5, 0.5 * 3f32.sqrt(), 0.0);
            for i in 0..spec.thrusters.len().min(2) {
                self.thruster_flame(particles, world(spec.thrusters[i]), forward, turned, thrust, spec.flame_width, spec.flame_length);
            }
        }

        // h-bike-base's vehicle-method-78: the roll thrusters that are firing, aimed by `forward-up->quaternion` of
        // their direction and the body's up.
        if spec.roll_flame && frame.hit_points > 0.0 {
            let up = rotation * Vec3::Y;
            for (i, (at, toward)) in ROLL_FLAMES.into_iter().enumerate() {
                if frame.roll_thrust[i] > 0.0 {
                    self.thruster_flame(particles, world(at), rotation * toward, up, frame.roll_thrust[i], ROLL_FLAME_WIDTH, ROLL_FLAME_LENGTH);
                }
            }
        }
    }

    /// `vehicle-draw-thruster` (vehicle-effects.gc): a flame at `at` whose frame is `forward-up->quaternion` of
    /// (`forward`, `up`) turned half a turn about x, so that it burns along minus `forward`. The length is the
    /// full length times the thrust times a random 1 to 1.33, the width the full width times the same (never
    /// more than half the length), and three blades 60 degrees apart (`quaternion-rotate-local-z!` 10922.667) are
    /// launched about its axis. The glow that goes with it is drawn by the client.
    #[allow(clippy::too_many_arguments)]
    fn thruster_flame(&mut self, particles: &mut Particles, at: Vec3, forward: Vec3, up: Vec3, thrust: f32, width: f32, length: f32) {
        let growth = self.range(1.0, 1.33);
        let length = growth * length * thrust;
        let width = (width * growth).min(length / 2.0);
        // Nothing of the flame is longer than nothing.
        if length <= 0.0 {
            return;
        }
        let Some(z) = forward.try_normalize() else { return };
        let Some(y) = (up - z * up.dot(z)).try_normalize() else { return };
        let x = y.cross(z);
        for blade in 0..3 {
            let angle = (60.0 * blade as f32).to_radians();
            // The blade's flat is the y axis of the half-turned frame, turned about its z by `angle`.
            let normal = -angle.sin() * x - angle.cos() * y;
            particles.launch_blade(THRUSTER, at, -z, normal, length, width);
        }
    }

    /// `vehicle-method-135`: a puff of smoke and sparks at each exhaust, on the ignition.
    fn start(&mut self, spec: &Specs, frame: &Frame, particles: &mut Particles) {
        let rotation = Mat3::from_quat(frame.rotation);
        for i in 0..spec.exhaust.len().min(spec.exhaust_dir.len()).min(2) {
            let at = frame.trans + rotation * spec.exhaust[i];
            let direction = rotation * spec.exhaust_dir[i];
            // The matrix the tracker is given: x = dir x up, y = dir, z = x x dir.
            let side = direction.cross(Vec3::Y).try_normalize().unwrap_or(Vec3::X);
            let basis = Mat3::from_cols(side, direction, side.cross(direction));
            particles.spawn(ENGINE_START, at, basis);
        }
    }

    /// `vehicle-method-78`'s damaged-vehicle branch: smoke from the two `smoke-local-pos`, the
    /// worse the hit-points the denser, and sparks under a quarter.
    fn smoke(&mut self, spec: &Specs, frame: &Frame, particles: &mut Particles, rotation: &Mat3) {
        // A random direction on the lower half of a sphere: `cos`/`sin` of -32768..0 and a full turn.
        let elevation = (-32768.0 + 32768.0 * self.random()) * ANGLE;
        let turn = 65536.0 * self.random() * ANGLE;
        let kick = Vec3::new(
            elevation.cos() * turn.sin(),
            elevation.sin(),
            elevation.cos() * turn.cos(),
        );
        let (smoke, sparks) = if frame.hit_points < 0.25 {
            (Some(SMOKE_HEAVY), Some(SPARK))
        } else if frame.hit_points < 0.5 {
            (Some(SMOKE_MEDIUM), None)
        } else if self.random() < 0.05 {
            (Some(SMOKE_LIGHT), None)
        } else {
            (None, None)
        };
        let Some(smoke) = smoke else { return };
        for i in 0..spec.smoke_pos.len().min(spec.smoke_vel.len()).min(2) {
            let at = frame.trans + *rotation * spec.smoke_pos[i];
            let velocity = *rotation * spec.smoke_vel[i] + frame.velocity + kick * (24576.0 * self.random());
            if let Some(spark) = sparks {
                if self.random() < 0.005 {
                    particles.launch_var(spark, at, Mat3::IDENTITY, Overrides::default(), &mut 0.0);
                    self.zapped = true;
                }
            }
            particles.launch_var(
                smoke,
                at,
                Mat3::IDENTITY,
                Overrides {
                    vel: Some(velocity / TICKS),
                    ..Default::default()
                },
                &mut self.smoke[i],
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::particles::Library;
    use std::sync::Arc;

    /// The three parts of `vehicle-part.gc` the tests launch, as the game defines them.
    const PARTS: &str = "
(defpart 922
  :init-specs ((:texture (gun-yellow-muzzleflash level-default-sprite))
    (:num 1.0)
    (:scale-x (meters 0.4))
    (:scale-y (meters 3))
    (:r 128.0 128.0)
    (:g 32.0 96.0)
    (:b :copy g)
    (:a 128.0 64.0)
    (:timer (seconds 0.017))
    (:flags (sp-cpuinfo-flag-2 sp-cpuinfo-flag-3 left-multiply-quat))
    (:rotate-y (degrees 0))
    )
  )
(defpart 923
  :init-specs ((:texture (bigpuff level-default-sprite))
    (:birth-func 'birth-func-set-vel)
    (:num 0.1 0.2)
    (:scale-x (meters 0.2) (meters 0.2))
    (:rot-z (degrees 0) (degrees 360))
    (:scale-y :copy scale-x)
    (:r 196.0)
    (:g 160.0)
    (:b 96.0)
    (:a 1.0 1.0)
    (:scalevel-x (meters 0.006666667))
    (:rotvel-z (degrees -0.2) (degrees 0.4))
    (:scalevel-y :copy scalevel-x)
    (:fade-r -1.0)
    (:fade-g 0.0)
    (:fade-b 0.0)
    (:fade-a -0.64)
    (:accel-y (meters 0.00016666666) (meters 0.0005))
    (:friction 0.94)
    (:timer (seconds 0.5))
    (:flags (sp-cpuinfo-flag-2 sp-cpuinfo-flag-13 sp-cpuinfo-flag-14))
    (:next-time (seconds 0.167))
    (:next-launcher 924)
    )
  )
(defpart 924
  :init-specs ((:fade-r 0.0) (:fade-g 0.0) (:fade-b 0.0))
  )
(defpart 930
  :init-specs ((:texture (bigpuff level-default-sprite))
    (:birth-func 'birth-func-set-vel)
    (:num 1.0 3.0)
    (:scale-x (meters 0.5) (meters 0.25))
    (:a 64.0 32.0)
    (:timer (seconds 1))
    (:flags (sp-cpuinfo-flag-2))
    )
  )
";

    fn bike() -> Specs {
        // The h-bike-a constants (bike.gc): the thrusters and exhausts 1024 to each side, 3276.8 behind.
        Specs {
            heads: vec![],
            tails: vec![],
            damage_stages: 1,
            thrusters: vec![Vec3::new(1024.0, 0.0, -3276.8), Vec3::new(-1024.0, 0.0, -3276.8)],
            exhaust: vec![Vec3::new(1024.0, 204.8, -3276.8), Vec3::new(-1024.0, 204.8, -3276.8)],
            exhaust_dir: vec![Vec3::NEG_Z, Vec3::NEG_Z],
            smoke_pos: vec![Vec3::new(1638.4, -2048.0, 7372.8), Vec3::new(-1638.4, -2048.0, 7372.8)],
            smoke_vel: vec![Vec3::ZERO, Vec3::ZERO],
            flame_width: 0.6 * METER,
            flame_length: 2.0 * METER,
            max_engine_thrust: 50.0 * METER,
            roll_flame: true,
            masks: [1; 3],
            ..Default::default()
        }
    }

    fn particles() -> Particles {
        let mut library = Library::default();
        library.read(PARTS);
        Particles::new(Arc::new(library))
    }

    fn frame(trans: Vec3, rotation: Quat) -> Frame {
        Frame {
            trans,
            rotation,
            velocity: Vec3::ZERO,
            ignition: true,
            riding: true,
            engine_thrust: 0.0,
            hit_points: 1.0,
            ..Default::default()
        }
    }

    #[test]
    fn the_exhaust_is_born_at_its_offset_through_the_bone_transform_every_frame() {
        let spec = bike();
        let mut emitter = Emitter::new(7);
        let mut p = particles();
        let turned = Quat::from_rotation_y(std::f32::consts::FRAC_PI_2);
        let mut seen = 0;
        for k in 0..240 {
            // The bike drives off along +x at 30 m/s while it is turned a quarter.
            let trans = Vec3::new(k as f32 * 30.0 * METER / 60.0, 3.0 * METER, 0.0);
            let puffs = |p: &Particles| p.sprites().into_iter().filter(|s| s.stretch.is_none()).count();
            let before = puffs(&p);
            let mut f = frame(trans, turned);
            f.velocity = Vec3::X * 30.0 * METER;
            f.engine_thrust = 0.5;
            emitter.step(&spec, &f, &mut p);
            let born = puffs(&p) - before;
            if born == 0 {
                continue;
            }
            seen += born;
            // The newest puffs are the frame's: each at one of the two exhausts of THIS frame.
            let sprites: Vec<_> = p.sprites().into_iter().filter(|s| s.stretch.is_none()).collect();
            let expected = [
                trans + turned * spec.exhaust[0],
                trans + turned * spec.exhaust[1],
            ];
            for s in &sprites[sprites.len() - born..] {
                assert!(
                    expected.iter().any(|e| (s.pos - *e).length() < 0.5),
                    "frame {k}: born at {:?}, not at the exhausts {expected:?}",
                    s.pos
                );
            }
        }
        assert!(seen > 100, "only {seen} puffs in 4 s");
    }

    #[test]
    fn the_exhaust_rate_follows_the_original_number_and_keeps_its_fractions() {
        let spec = bike();
        let mut emitter = Emitter::new(11);
        let mut p = particles();
        let mut born = 0usize;
        let frames = 3000;
        for _ in 0..frames {
            let before = p.sprites().len();
            // Idle: engine-power-factor 0, so 6 * 0.25 * rand + 0.1 on average per exhaust.
            emitter.step(&spec, &frame(Vec3::ZERO, Quat::IDENTITY), &mut p);
            born += p.sprites().len() - before;
            // Age them out so the cap never limits the count.
            p.step(1.0);
        }
        let per_frame_per_exhaust = born as f32 / frames as f32 / 2.0;
        let expected = 6.0 * 0.25 * 0.5 + 0.1;
        assert!(
            (per_frame_per_exhaust - expected).abs() < 0.08,
            "{per_frame_per_exhaust} births a frame at each exhaust, the original gives {expected}"
        );
    }

    #[test]
    fn a_stopped_engine_or_full_power_launches_no_smoke() {
        let spec = bike();
        let mut emitter = Emitter::new(3);
        let mut p = particles();
        let mut off = frame(Vec3::ZERO, Quat::IDENTITY);
        off.ignition = false;
        for _ in 0..120 {
            emitter.step(&spec, &off, &mut p);
        }
        assert_eq!(p.sprites().len(), 0);
        // alpha 16 * (1 - power^2) and 48 * (1 - power^2) reach 0 at full power.
        let mut full = frame(Vec3::ZERO, Quat::IDENTITY);
        full.engine_thrust = 1.0;
        full.velocity = Vec3::Z * 40.0 * METER;
        for _ in 0..120 {
            emitter.step(&spec, &full, &mut p);
            p.step(DT);
        }
        // The flames (blades) are there at full thrust; the smoke is not.
        assert_eq!(p.sprites().into_iter().filter(|s| s.stretch.is_none()).count(), 0);
    }

    #[test]
    fn the_thruster_flames_hang_behind_the_thrusters_and_grow_with_the_thrust() {
        let spec = bike();
        let mut library = Library::default();
        library.read(PARTS);
        let mut emitter = Emitter::new(5);
        let mut p = Particles::new(Arc::new(library));
        let heading = Quat::from_rotation_y(0.7);
        let mut lengths = Vec::new();
        for thrust in [0.1, 0.5, 1.0] {
            // Settle `power-level` first.
            let mut f = frame(Vec3::new(1000.0, 2000.0, -500.0), heading);
            f.engine_thrust = thrust;
            for _ in 0..90 {
                emitter.step(&spec, &f, &mut p);
                p.step(1.0);
            }
            let before = p.sprites().len();
            emitter.step(&spec, &f, &mut p);
            let sprites = p.sprites();
            let flames: Vec<_> = sprites[before.min(sprites.len())..].iter().filter(|s| s.stretch.is_some()).collect();
            assert!(!flames.is_empty(), "thrust {thrust}: no flame");
            // Each blade's middle lies behind a thruster, half its length along the back direction.
            let back = heading * Vec3::NEG_Z;
            for s in &flames {
                let long = s.size[0];
                let near = spec.thrusters.iter().any(|t| {
                    let root = f.trans + heading * *t;
                    (s.pos - (root + back * long * 0.5)).length() < 1.0
                });
                assert!(near, "flame of length {long} is not on a thruster");
            }
            lengths.push(flames[0].size[0]);
        }
        assert!(lengths[0] < lengths[1] && lengths[1] < lengths[2], "{lengths:?}");
        // At full thrust the flame is the original 2 m (1 to 1.33 times, with power-level a hair under 1).
        assert!(lengths[2] > 1.8 * METER && lengths[2] < 2.7 * METER, "{lengths:?}");
    }

    /// The game's own vehicle-part.gc, when this machine has it.
    fn real_particles() -> Option<Particles> {
        let home = std::env::var_os("USERPROFILE")?;
        let path = std::path::PathBuf::from(home)
            .join(r"Documents\Jak OpenGOAL\active\jak3\data\goal_src\jak3\levels\city\traffic\vehicle\vehicle-part.gc");
        let mut library = Library::default();
        library.read(&std::fs::read_to_string(path).ok()?);
        Some(Particles::new(Arc::new(library)))
    }

    #[test]
    fn a_firing_roll_thruster_burns_down_and_out_from_its_side_in_three_crossed_blades() {
        let spec = bike();
        let mut emitter = Emitter::new(21);
        let attitude = Quat::from_rotation_y(0.7) * Quat::from_rotation_z(-0.4);
        let trans = Vec3::new(500.0, 3000.0, -800.0);
        let rotation = Mat3::from_quat(attitude);
        // Idle, whole, and the left thruster firing at a third of the bike's weight.
        let mut f = frame(trans, attitude);
        f.ignition = false;
        f.roll_thrust = [0.3, 0.0];
        let mut p = particles();
        emitter.step(&spec, &f, &mut p);
        let blades: Vec<_> = p.sprites().into_iter().filter(|s| s.stretch.is_some()).collect();
        assert_eq!(blades.len(), 3, "three blades, one flame");
        // Aimed out and down from the left side, as the thruster pushes the bike up and in.
        let (at, toward) = ROLL_FLAMES[0];
        let flame = -(rotation * toward.normalize());
        let root = trans + rotation * at;
        let length = blades[0].size[0];
        // 12288 units at 0.3 of the thrust, 1 to 1.33 times.
        assert!(length >= 0.3 * ROLL_FLAME_LENGTH * 0.999 && length <= 0.3 * ROLL_FLAME_LENGTH * 1.331, "{length}");
        let dir = rotation * Vec3::new(0.707, -0.707, 0.0).normalize();
        for b in &blades {
            assert!((b.stretch.unwrap().normalize() - dir).length() < 0.01, "{:?} against {dir:?}", b.stretch);
            assert!((b.pos - (root + dir * length * 0.5)).length() < 1.0, "the blade's middle is half its length along the flame");
            assert!(b.size[1] <= length / 2.0 + 1e-3 && b.size[1] <= ROLL_FLAME_WIDTH * 1.331, "width {:?}", b.size);
            assert!(b.flat.unwrap().dot(dir).abs() < 1e-3, "the blade's flat contains the flame");
        }
        assert!((flame - dir).length() < 0.01);
        // 60 degrees apart about the flame: the three flats' pairwise angles (as planes) are 60 degrees.
        for (i, j) in [(0, 1), (1, 2), (0, 2)] {
            let c = blades[i].flat.unwrap().dot(blades[j].flat.unwrap()).abs();
            assert!((c - 0.5).abs() < 1e-3, "blades {i} and {j}: cos {c}");
        }
        // The other thruster and a quiet one launch nothing; a dead vehicle's don't either.
        let mut quiet = particles();
        f.roll_thrust = [0.0, 0.0];
        emitter.step(&spec, &f, &mut quiet);
        assert_eq!(quiet.sprites().len(), 0);
        f.roll_thrust = [0.0, 0.5];
        emitter.step(&spec, &f, &mut quiet);
        let right_root = trans + rotation * ROLL_FLAMES[1].0;
        assert!(quiet.sprites().iter().all(|s| (s.pos - right_root).length() < ROLL_FLAME_LENGTH * 1.4), "the right flame hangs on the right");
        assert_eq!(quiet.sprites().len(), 3);
        let mut dead = particles();
        f.hit_points = 0.0;
        emitter.step(&spec, &f, &mut dead);
        assert_eq!(dead.sprites().len(), 0);
        // The cars have none.
        let mut car = bike();
        car.roll_flame = false;
        f.hit_points = 1.0;
        emitter.step(&car, &f, &mut dead);
        assert_eq!(dead.sprites().len(), 0);
    }

    #[test]
    fn scrape_sparks_are_born_at_the_impact_only_while_the_scrape_is_loud() {
        let Some(mut p) = real_particles() else {
            eprintln!("vehicle-part.gc unavailable");
            return;
        };
        let spec = bike();
        let mut emitter = Emitter::new(4);
        let mut f = frame(Vec3::new(0.0, 5000.0, 0.0), Quat::IDENTITY);
        f.ignition = false;
        f.impact_pos = Vec3::new(3000.0, 4000.0, 1000.0);
        f.scrape = 0.74;
        for _ in 0..30 {
            emitter.step(&spec, &f, &mut p);
        }
        assert_eq!(p.sprites().len(), 0, "an envelope under 0.75 makes no sparks");
        f.scrape = 0.8;
        for _ in 0..30 {
            emitter.step(&spec, &f, &mut p);
        }
        let sparks = p.sprites();
        // Part 920 launches 1 to 3 sparks a frame, all at the impact point.
        assert!(sparks.len() >= 30, "{}", sparks.len());
        assert!(sparks.iter().all(|s| (s.pos - f.impact_pos).length() < 600.0), "sparks away from the impact");
    }

    #[test]
    fn only_a_wrecked_vehicle_throws_sparks_and_zaps() {
        let Some(mut p) = real_particles() else {
            eprintln!("vehicle-part.gc unavailable");
            return;
        };
        let spec = bike();
        let mut wrecked = Emitter::new(8);
        let mut f = frame(Vec3::ZERO, Quat::IDENTITY);
        f.ignition = false;
        f.hit_points = 0.1;
        let mut zaps = 0;
        for _ in 0..3000 {
            wrecked.step(&spec, &f, &mut p);
            zaps += usize::from(wrecked.zapped);
            p.step(1.0);
        }
        // 0.5 % of the frames a smoke point: about thirty zaps in 3000 frames.
        assert!(zaps > 10 && zaps < 80, "{zaps}");
        let mut healthy = Emitter::new(8);
        f.hit_points = 0.6;
        for _ in 0..3000 {
            healthy.step(&spec, &f, &mut p);
            assert!(!healthy.zapped, "a half-wrecked vehicle smokes but does not zap");
            p.step(1.0);
        }
    }

    #[test]
    fn the_power_fluctuation_pops_at_its_own_rate() {
        let spec = bike();
        let mut p = particles();
        let mut f = frame(Vec3::ZERO, Quat::IDENTITY);
        f.ignition = false;
        let mut count = |hp: f32| {
            let mut emitter = Emitter::new(15);
            let mut f = f;
            f.hit_points = hp;
            (0..20000).filter(|_| {
                emitter.step(&spec, &f, &mut p);
                emitter.popped
            }).count()
        };
        // 0.01 of the frames whole, 0.16 at a sixth of the hit-points.
        let whole = count(1.0) as f32 / 20000.0;
        let hurt = count(0.2) as f32 / 20000.0;
        assert!((whole - 0.01).abs() < 0.004, "{whole}");
        assert!((hurt - 0.16).abs() < 0.02, "{hurt}");
    }

    #[test]
    fn a_damaged_vehicle_smokes_from_its_smoke_points_with_the_vehicles_velocity() {
        let spec = bike();
        let mut emitter = Emitter::new(9);
        let mut p = particles();
        let mut f = frame(Vec3::new(0.0, 5000.0, 0.0), Quat::IDENTITY);
        f.ignition = false;
        f.hit_points = 0.6;
        f.velocity = Vec3::Z * 20.0 * METER;
        let mut puffs = 0;
        for _ in 0..2000 {
            let before = p.sprites().len();
            emitter.step(&spec, &f, &mut p);
            puffs += p.sprites().len() - before;
            for s in &p.sprites()[before.min(p.sprites().len())..] {
                assert!(spec.smoke_pos.iter().any(|q| (s.pos - (f.trans + *q)).length() < 1.0));
            }
            p.step(1.0);
        }
        // 5 % of the frames, 1 to 4 puffs at each of two points: a hundred frames' worth, then some.
        assert!(puffs > 100, "{puffs}");
        f.hit_points = 0.9;
        let mut healthy = particles();
        let mut calm = Emitter::new(9);
        for _ in 0..600 {
            calm.step(&spec, &f, &mut healthy);
        }
        assert_eq!(healthy.sprites().len(), 0);
    }
}
