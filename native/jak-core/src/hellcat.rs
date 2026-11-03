//! The KG HellCat as Jak flies it in "Destroy War Factory Defenses"
//! (`h-warf.gc`, `warf-projectile.gc`), which Jak can also park, leave and
//! board again like the street HellCat (`h-hellcat`).
//!
//! Piloted, it drives like the street HellCat: it hovers over the ground,
//! X accelerates up to 30 m/s, Square brakes and backs up, the stick steers
//! and it banks into turns. It keeps the mission craft's weapons: R1
//! fires the nose gun (four shots in any second), L1 a torpedo (eight,
//! refilled one every 20 s) whose blast sweeps 60 m. Left alone it falls to
//! the street HellCat's hover and settles 1.5 m over the ground.
//!
//! Units are Jak's: 4096 to the meter, y up, 300 ticks a second, stepped
//! at 60 Hz.
use glam::{Quat, Vec3};

use crate::collide::{CollideCache, CollideWorld};
use crate::math::*;
use crate::pad::{PadInput, button};
use crate::projectile::{ActorWorld, ProjectileHit};
use crate::weapons::Beam;

/// The six spheres `init-collision!` gives it: local x, y, z, radius.
pub const SPHERES: [[f32; 4]; 6] = [
    [meters(1.0), meters(0.3), meters(2.2), meters(1.5)],
    [meters(-1.0), meters(0.3), meters(2.2), meters(1.5)],
    [0.0, meters(0.5), meters(-0.1), meters(1.5)],
    [0.0, meters(1.0), meters(-2.5), meters(2.0)],
    [meters(2.5), meters(0.5), meters(-2.0), meters(1.5)],
    [meters(-2.5), meters(0.5), meters(-2.0), meters(1.5)],
];
/// The driver's seat (`vsf0`), local.
pub const SEAT: Vec3 = Vec3::new(meters(1.25), meters(0.2), meters(0.6));
pub const MASS: f32 = 10.0;
/// `*h-warf-constants*` gravity, and the street HellCat's.
pub const GRAVITY: f32 = meters(20.0);
pub const PARKED_GRAVITY: f32 = meters(40.0);
pub const MAX_SPEED: f32 = 204800.0;
pub const CRUISE_SPEED: f32 = 153600.0;
/// The street HellCat's top speed (`max-xz-speed`, 30 m/s), the push of
/// its engine and brakes, and how fast it backs up.
pub const STREET_TOP: f32 = 122880.0;
/// The heights R2 and L2 step between: the street, over the rooftops, and
/// high above (`flight-level` in `hvehicle`).
pub const FLIGHT_LEVELS: [f32; 3] = [meters(1.5), meters(10.0), meters(24.0)];
/// How fast R2 and L2 held lift it and let it down.
const STREET_REVERSE: f32 = meters(6.0);
const SETTLE: f32 = meters(1.5);
const GUN_MUZZLE: Vec3 = Vec3::new(0.0, 7372.8, 10649.6);
const TORPEDO_MUZZLE: Vec3 = Vec3::new(0.0, 7372.8, 18841.6);
const GUN_SPEED: f32 = 4096000.0;
const TORPEDO_SPEED: f32 = 819200.0;
const TORPEDO_MAX: u8 = 8;
const BLAST_RADIUS: f32 = 245760.0;
const BLAST_GROWTH: f32 = 491520.0;
const LONG_AGO: i64 = i64::MIN / 4;

/// What the craft did this frame, for sounds and the host.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum HellCatEvent {
    Fire { pos: Vec3 },
    FireAlt { pos: Vec3 },
    /// Damage the guns dealt: to a creature, or the world when `actor` is
    /// none.
    Impact(ProjectileHit),
    /// A torpedo burst.
    Blast { center: Vec3 },
    /// The hull struck something: how hard, in m/s.
    Crash { pos: Vec3, speed: f32 },
    /// The hull touched something (any contact while piloted): the point,
    /// and the surface's normal there. A host whose world breaks under it
    /// (Minecraft's leaves) breaks it.
    Brush { pos: Vec3, normal: Vec3 },
    /// Destroyed.
    Explode { center: Vec3 },
}

#[derive(Clone, Debug)]
struct Shot {
    trans: Vec3,
    tail: Vec3,
    vel: Vec3,
    spawn: i64,
    torpedo: bool,
}

#[derive(Clone, Debug)]
struct Blast {
    center: Vec3,
    start: i64,
    hit: Vec<u64>,
}

/// Where the HellCat flies, as the host sees it: one physics from the
/// street to space, scaled from outside. Its frame is always local, y up;
/// a host flying it round a planet or between planets moves that frame
/// with [`HellCat::reframe`].
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FlightEnv {
    /// Gravity against the street's: 1 down here, 0 in space.
    pub gravity: f32,
    /// Top speed and thrust against the street's 30 m/s: 1 down here,
    /// hundreds out in space.
    pub speed_scale: f32,
    /// The fastest it may go, Jak units a second, whatever the gear and the
    /// boost (in Minecraft: an elytra's with rockets); 0 for no cap.
    pub max_speed: f32,
}

impl Default for FlightEnv {
    fn default() -> Self {
        Self {
            gravity: 1.0,
            speed_scale: 1.0,
            max_speed: 0.0,
        }
    }
}

/// The boost (X held): four times the push and the top speed, and
/// the full orange jet.
pub const BOOST: f32 = 4.0;

/// The speed gears, X tapped twice up and Square tapped twice down while
/// piloting (Adrii, 15/10): slow, normal and fast, the top speed, thrust,
/// brakes and lift times this (the fast one is Minecraft's elytra with
/// rockets, 48 m/s, over the street's 30). The speed eases to a new gear
/// over a second or so: smooth shifts. Space's speeds come from the host's
/// [`FlightEnv`].
///
/// (The space gears for crossing between the universe's planets went with
/// the planets, 16/10: the HellCat stays in the air.)
pub const GEARS: [f32; 3] = [0.6, 1.0, 1.6];
/// The gears usable in a planet's air.
pub const AIR_GEARS: u8 = 3;
/// The gear it starts in: normal.
pub const START_GEAR: u8 = 1;
/// Two presses of a button within this make a double tap (seconds).
const DOUBLE_TAP: f32 = 0.35;

pub struct HellCat {
    pub trans: Vec3,
    pub vel: Vec3,
    /// Heading about y (radians, 0 along +z), and the pitch and bank the
    /// hull shows.
    pub yaw: f32,
    pub pitch: f32,
    pub roll: f32,
    pub steering: f32,
    pub lean: f32,
    pub throttle: f32,
    pub brake: f32,
    /// What the engine gives, 0 to 1: the boost sound and the flames.
    pub engine: f32,
    /// The wind over the hull as it turns: the bank sound.
    pub wind: f32,
    /// 1 when new; destroyed below 0.
    pub health: f32,
    pub piloted: bool,
    pub destroyed: bool,
    pub destroyed_at: i64,
    pub torpedoes: u8,
    /// Set by the host: how strong gravity is and how fast it may go.
    pub env: FlightEnv,
    /// X held: the boost is on.
    pub boosting: bool,
    /// Parked in the open air (brought beside Jak on a ledge): it hangs
    /// there until he climbs in, instead of dropping to whatever is below.
    pub hold: bool,
    /// Left running when Jak got out: it hangs where it was on a soft
    /// hover, its thrusters lit, until a blow shuts it off and it drops
    /// (Adrii, 16/10).
    pub idling: bool,
    /// Which of [`GEARS`] it is in (X or Square tapped twice).
    pub gear: u8,
    /// The gear's scale as it eases there.
    gear_scale: f32,
    /// When X and Square were last pressed (for double taps).
    x_tap: i64,
    square_tap: i64,
    /// The flight level, 0 on the street: the highest of `FLIGHT_LEVELS`
    /// at or below `altitude`.
    pub level: u8,
    /// How high the hover holds it over the ground while piloted: R2 and
    /// L2 held lift and lower it, and tilting the nose climbs or sinks it.
    pub altitude: f32,
    pub time: i64,
    launch: Option<i64>,
    impact_time: i64,
    stop_time: i64,
    fire_slots: [i64; 4],
    torpedo_time: i64,
    refill_time: i64,
    roll_rate: f32,
    /// How fast the nose pitched last frame, radians a second
    /// (`langvel` x: the flames bend with it too).
    pitch_rate: f32,
    /// R2 held (+1) or L2 held (-1).
    lift: f32,
    /// The roll asked, -1 left to +1 right: Q and E on the keyboard, and in
    /// open flight R2 and L2 too.
    roll_in: f32,
    /// How fast the hull turns about its own axes in open flight (pitch,
    /// yaw, roll; radians a second), easing to what the stick asks.
    spin: Vec3,
    /// How fast the hover is asked to climb, Jak units a second.
    climb: f32,
    /// How fast it turned last frame, radians a second (the flames bend).
    yaw_rate: f32,
    /// Free flight, with no gravity to hold it level (out in space): the
    /// hull's own orientation, which the stick turns about its own axes as a
    /// starship's, nose over tail and upside down if asked.
    free: Option<Quat>,
    shots: Vec<Shot>,
    blasts: Vec<Blast>,
    cache: CollideCache,
    pub events: Vec<HellCatEvent>,
}

/// `analog-input` with the vehicle's dead zone: centered 128, nothing
/// inside 48, full at 110.
pub fn analog(v: f32) -> f32 {
    let x = v.abs() * 128.0;
    if x <= 48.0 {
        return 0.0;
    }
    ((x - 48.0) / (110.0 - 48.0)).min(1.0) * v.signum()
}

impl HellCat {
    pub fn contact_spin(&self) -> Vec3 {
        self.orientation() * self.spin
    }

    pub fn apply_contact_spin(&mut self, world_delta: Vec3) {
        self.spin += self.orientation().conjugate() * world_delta;
        self.yaw_rate += world_delta.y;
        self.roll_rate += world_delta.dot(self.orientation() * Vec3::Z);
        self.pitch_rate += world_delta.dot(self.orientation() * Vec3::X);
    }

    /// Parked at `trans`, facing `yaw`.
    pub fn new(trans: Vec3, yaw: f32) -> Self {
        Self {
            trans,
            vel: Vec3::ZERO,
            yaw,
            pitch: 0.0,
            roll: 0.0,
            steering: 0.0,
            lean: 0.0,
            throttle: 0.0,
            brake: 0.0,
            engine: 0.0,
            wind: 0.0,
            health: 1.0,
            piloted: false,
            destroyed: false,
            destroyed_at: LONG_AGO,
            torpedoes: TORPEDO_MAX,
            env: FlightEnv::default(),
            boosting: false,
            hold: false,
            idling: false,
            gear: START_GEAR,
            gear_scale: 1.0,
            x_tap: LONG_AGO,
            square_tap: LONG_AGO,
            level: 0,
            altitude: SETTLE,
            time: 0,
            launch: None,
            impact_time: LONG_AGO,
            stop_time: LONG_AGO,
            fire_slots: [-seconds(4.0); 4],
            torpedo_time: LONG_AGO,
            refill_time: 0,
            roll_rate: 0.0,
            pitch_rate: 0.0,
            lift: 0.0,
            roll_in: 0.0,
            spin: Vec3::ZERO,
            climb: 0.0,
            yaw_rate: 0.0,
            free: None,
            shots: Vec::new(),
            blasts: Vec::new(),
            cache: CollideCache::default(),
            events: Vec::new(),
        }
    }

    pub fn heading(&self) -> Vec3 {
        match self.free {
            Some(q) => q * Vec3::Z,
            None => Vec3::new(self.yaw.sin(), 0.0, self.yaw.cos()),
        }
    }

    /// The hull's orientation: heading, then nose pitch, then bank (or, in
    /// free flight, its own).
    pub fn orientation(&self) -> Quat {
        self.free.unwrap_or_else(|| {
            Quat::from_rotation_y(self.yaw)
                * Quat::from_rotation_x(-self.pitch)
                * Quat::from_rotation_z(-self.roll)
        })
    }

    /// In free flight (no gravity), the hull's orientation.
    pub fn free_flight(&self) -> Option<Quat> {
        self.free
    }

    /// The host moved its frame (another patch of a planet, another
    /// planet): `rotation` turns the old frame into the new one and
    /// `offset` follows. Position and velocity carry over, the speed is
    /// kept, and the hull keeps its heading in the new frame.
    pub fn reframe(&mut self, rotation: Quat, offset: Vec3) {
        self.trans = rotation * self.trans + offset;
        self.vel = rotation * self.vel;
        if let Some(q) = self.free.as_mut() {
            *q = (rotation * *q).normalize();
        }
        let ahead = rotation * self.heading();
        if ahead.x.hypot(ahead.z) > 1e-4 {
            self.yaw = ahead.x.atan2(ahead.z);
        }
        for shot in &mut self.shots {
            shot.trans = rotation * shot.trans + offset;
            shot.tail = rotation * shot.tail + offset;
            shot.vel = rotation * shot.vel;
        }
    }

    /// Out of every planet's air (no gravity): the space gears are open.
    pub fn in_space(&self) -> bool {
        self.env.gravity < 0.02
    }

    /// The gear's name, for the HUD.
    pub fn gear_name(&self) -> String {
        match self.gear {
            0 => "slow".into(),
            1 => "normal".into(),
            2 => "fast".into(),
            g => format!("space {}/{}", g - AIR_GEARS + 1, GEARS.len() as u8 - AIR_GEARS),
        }
    }

    /// What the speeds are scaled by: the host's (space is faster) times
    /// the gear.
    pub fn speed_scale(&self) -> f32 {
        self.env.speed_scale.max(1.0) * self.gear_scale.max(0.1)
    }

    /// How far into open flight it is, 0 on the street to 1 high up (its
    /// hover above the top flight level, or out where gravity fades): it
    /// grips the air less and banks deeper the higher it is, a little like
    /// a starship.
    pub fn airborne(&self) -> f32 {
        let high = ((self.altitude - FLIGHT_LEVELS[1]) / (FLIGHT_LEVELS[2] - FLIGHT_LEVELS[1])).clamp(0.0, 1.0);
        high.max(1.0 - self.env.gravity.clamp(0.0, 1.0))
    }

    pub fn seat(&self) -> Vec3 {
        self.trans + self.orientation() * SEAT
    }

    pub fn speed(&self) -> f32 {
        self.vel.dot(self.heading())
    }

    /// Jak takes the controls: the engine lights, and it waits for X.
    pub fn board(&mut self) {
        self.piloted = true;
        self.hold = false;
        self.idling = false;
        self.launch = None;
        self.stop_time = LONG_AGO;
    }

    /// Put down by the host somewhere new (out of a planet it had flown
    /// into): still, level, facing the way it headed.
    pub fn settle(&mut self) {
        let f = self.heading();
        if f.x.hypot(f.z) > 1e-3 {
            self.yaw = f.x.atan2(f.z);
        }
        if self.free.is_some() {
            self.free = Some(Quat::from_rotation_y(self.yaw));
        }
        self.pitch = 0.0;
        self.roll = 0.0;
        self.vel = Vec3::ZERO;
        self.spin = Vec3::ZERO;
        self.throttle = 0.0;
    }

    pub fn leave(&mut self) {
        self.piloted = false;
        self.idling = true;
        // Parked, it rests level on its hover again, facing where it flew.
        if let Some(q) = self.free.take() {
            let f = q * Vec3::Z;
            if f.x.hypot(f.z) > 1e-3 {
                self.yaw = f.x.atan2(f.z);
            }
            self.spin = Vec3::ZERO;
        }
        self.gear = START_GEAR;
        self.gear_scale = 1.0;
        self.level = 0;
        self.altitude = SETTLE;
        self.launch = None;
        self.throttle = 0.0;
        self.brake = 0.0;
        self.steering = 0.0;
        self.lean = 0.0;
    }

    /// A hit from outside: `h-warf`'s `apply-damage` takes the same small
    /// bite whatever struck it.
    pub fn damage(&mut self) {
        if !self.destroyed {
            self.health -= 0.4 * 0.025;
            self.check_dead();
        }
    }

    fn check_dead(&mut self) {
        if self.health < 0.0 && !self.destroyed {
            self.destroyed = true;
            self.destroyed_at = self.time;
            self.piloted = false;
            self.events.push(HellCatEvent::Explode { center: self.trans });
        }
    }

    /// One 60 Hz frame. `input` is read only while piloted.
    pub fn step(
        &mut self,
        input: &PadInput,
        world: &mut dyn CollideWorld,
        mut actors: Option<&mut dyn ActorWorld>,
    ) {
        self.time += FRAME_TICKS;
        let dt = SECONDS_PER_FRAME;
        if self.piloted && !self.destroyed {
            self.controls(input, dt);
            self.fly_free(dt, world);
            self.guns(input, world, crate::projectile::reborrow(&mut actors));
        } else {
            self.controls(&PadInput::default(), dt);
            self.hover(dt, world);
        }
        self.step_shots(world, crate::projectile::reborrow(&mut actors));
        self.step_blasts(actors);
        if self.torpedoes < TORPEDO_MAX {
            if self.time - self.refill_time >= seconds(20.0) {
                self.torpedoes += 1;
                self.refill_time = self.time;
            }
        } else {
            self.refill_time = self.time;
        }
    }

    /// The stick and buttons, smoothed as `hvehicle` does.
    fn controls(&mut self, input: &PadInput, dt: f32) {
        // Circle and Square roll; X boosts, consistently with the cars.
        self.lift = f32::from(u8::from(input.held & button::CIRCLE != 0))
            - f32::from(u8::from(input.held & button::SQUARE != 0));
        self.roll_in = f32::from(u8::from(input.held & button::ROLL_RIGHT != 0))
            - f32::from(u8::from(input.held & button::ROLL_LEFT != 0));
        let steer = -analog(input.left[0]);
        let lean = analog(input.left[1]);
        let throttle = input.pedal(true);
        let brake = input.pedal(false);
        self.boosting = input.held & button::X != 0 && throttle > 0.0;
        // R2 tapped twice shifts up a gear, L2 tapped twice down (the
        // D-pad is never the craft's: Adrii).
        let double = seconds(DOUBLE_TAP);
        if input.pressed & button::R2 != 0 {
            if self.time - self.x_tap <= double {
                if usize::from(self.gear) + 1 < GEARS.len() && (self.gear + 1 < AIR_GEARS || self.in_space()) {
                    self.gear += 1;
                }
                self.x_tap = LONG_AGO;
            } else {
                self.x_tap = self.time;
            }
        }
        if input.pressed & button::L2 != 0 {
            if self.time - self.square_tap <= double {
                self.gear = self.gear.saturating_sub(1);
                self.square_tap = LONG_AGO;
            } else {
                self.square_tap = self.time;
            }
        }
        // Into a planet's air: down to the fast gear by itself.
        if self.gear >= AIR_GEARS && !self.in_space() {
            self.gear = AIR_GEARS - 1;
        }
        // The new gear's scale comes in smoothly (a third of the way to it
        // every half second or so, in octaves).
        let want = GEARS[usize::from(self.gear).min(GEARS.len() - 1)];
        let (now, target) = (self.gear_scale.max(0.1).log2(), want.log2());
        self.gear_scale = 2f32.powf(now + (target - now) * (2.2 * dt).min(1.0));
        self.steering = seek(self.steering, steer, 8.0 * dt);
        self.lean = seek(self.lean, lean, 8.0 * dt);
        self.throttle = seek(self.throttle, throttle, 4.0 * dt);
        self.brake += (brake - self.brake) * (8.0 * dt).min(1.0);
        if self.throttle > 0.0 && self.brake > 0.0 {
            self.brake = self.brake.min(0.25);
        }
        let speed = self.vel.length();
        let target = self
            .throttle
            .min(0.83333 * (0.5 + (speed / (meters(30.0) * self.speed_scale())).clamp(0.0, 1.0)));
        if target > self.engine {
            self.engine += (target - self.engine) * (60.0 * dt).min(1.0);
        } else {
            self.engine = seek(self.engine, target, dt);
        }
    }

    /// Open flight, as the universe's starships fly: the stick pitches and
    /// yaws the hull about its own axes, all the way round if held (loops),
    /// Q and E (Circle and Square) roll it (barrel rolls), the throttle pushes it
    /// along its nose. The handling is No Man's Sky's fighter's (its
    /// `GcPlayerSpaceshipControlData`, planet and space engines): about 95°
    /// a second of pitch in the air and 63° in space, 165° a second of roll,
    /// the turn rates easing in over a tenth of a second (`AngularFactor`),
    /// and, under gravity, the wings levelling by themselves in about a
    /// second when no roll is asked (`BalanceTime`), banked into a turn,
    /// never the nose. In space nothing levels it. Back where gravity holds
    /// it, flying level with the stick let go, it is the HellCat over a
    /// street again.
    fn fly_free(&mut self, dt: f32, world: &mut dyn CollideWorld) {
        let mut q = self.free.unwrap_or_else(|| self.orientation());
        let g = self.env.gravity.clamp(0.0, 1.0);
        // `TurnStrength`: 1.65 over a planet, 1.1 in space.
        let turn = lerp(1.1, 1.65, g);
        // The stick's response: gentle near the middle, full at the edge.
        let curve = |v: f32| v * (0.35 + 0.65 * v * v);
        // Right (E, R2) positive. Jak's x is the hull's left, so a turn
        // about +z lifts the left wing: a roll to the right.
        let roll_in = (self.roll_in + self.lift).clamp(-1.0, 1.0);
        let want = Vec3::new(
            // Stick back (lean -1) lifts the nose (a turn about -x).
            turn * curve(self.lean),
            0.6 * turn * curve(self.steering),
            // `RollAmount` 2.3 times `RollForce` 1.25.
            2.9 * roll_in,
        );
        // `AngularFactor` 0.1: the turn rates ease to the stick.
        self.spin += (want - self.spin) * (dt / 0.1).min(1.0);
        q = (q
            * Quat::from_rotation_y(self.spin.y * dt)
            * Quat::from_rotation_x(self.spin.x * dt)
            * Quat::from_rotation_z(self.spin.z * dt))
        .normalize();
        let f = q * Vec3::Z;
        // Under gravity, with no roll asked, the wings level about the nose
        // (banked a little into a turn), unless the nose points nearly
        // straight up or down, where level means nothing.
        let steep = f.y.abs();
        // A loop held (the stick pulled) is left alone.
        if g > 0.05 && roll_in == 0.0 && self.lean.abs() < 0.5 && steep < 0.94 {
            let up = q * Vec3::Y;
            let left = q * Vec3::X;
            let flat_left = Vec3::Y.cross(f).normalize_or_zero();
            let flat_up = f.cross(flat_left).normalize_or_zero();
            // The bank, positive to the right (the left wing up).
            let bank = left.dot(flat_up).atan2(up.dot(flat_up));
            let target = -0.6 * self.steering;
            let err = target - bank;
            // `BalanceTime` about a second over a planet, fading out as the
            // nose nears the vertical.
            let fade = 1.0 - ((steep - 0.7) / 0.24).clamp(0.0, 1.0);
            let level = err * (2.2 * g * fade * dt).min(1.0);
            q = (q * Quat::from_rotation_z(level)).normalize();
        }
        // Too slow to fly under gravity, with the stick let go, the nose
        // drops back towards the horizon (a stall), more the slower it is.
        let stall = meters(8.0) * self.speed_scale();
        let speed = self.vel.dot(q * Vec3::Z);
        if g > 0.05 && self.lean.abs() < 0.3 && speed < stall {
            let nose = (q * Vec3::Z).y.clamp(-1.0, 1.0).asin();
            let rate = 0.9 * g * (1.0 - speed.max(0.0) / stall);
            let turn = nose.signum() * (rate * dt).min(nose.abs());
            q = (q * Quat::from_rotation_x(turn)).normalize();
        }
        self.yaw_rate = self.spin.y;
        self.roll_rate = self.spin.z;
        self.pitch_rate = self.spin.x;
        let f = q * Vec3::Z;
        self.free = Some(q);
        // Along the nose: the throttle up to the gear's top speed (four
        // times it boosting), the brake down to a crawl backwards; let go,
        // it coasts, as in space (gravity barely sags it, and near the
        // ground its hover holds it up, so it can hang there and land).
        let boost = if self.boosting && self.gear < AIR_GEARS { BOOST } else { 1.0 };
        let mut top = STREET_TOP * self.speed_scale() * boost;
        if self.env.max_speed > 0.0 {
            top = top.min(self.env.max_speed);
        }
        let speed = self.vel.dot(f);
        let want = if self.brake > 0.05 {
            -STREET_REVERSE * self.brake
        } else if self.throttle > 0.0 {
            top * self.throttle
        } else if g > 0.05 {
            // Let go under gravity it slows to a stop (Adrii, 15/10: it
            // went on by itself unless Square braked it).
            0.0
        } else {
            // Out in space it coasts.
            speed.max(0.0)
        };
        // The velocity swings round to the nose (a starship's arcade
        // handling, `DirectionBrake`), keeping its speed through loops and
        // turns, and eases to the speed asked.
        if want < 0.0 {
            self.vel += (f * want - self.vel) * (1.5 * dt).min(1.0);
        } else {
            let mag = self.vel.length();
            let dir = if mag > 1.0 { self.vel / mag } else { f };
            let dir = dir.lerp(f, (4.0 * dt).min(1.0)).normalize_or(f);
            let along = mag * dir.dot(f).max(0.0);
            let mag = mag + (want - along) * (1.5 * dt).min(1.0);
            self.vel = dir * mag.max(0.0);
        }
        self.vel.y -= PARKED_GRAVITY * 0.15 * g * dt;
        if self.env.max_speed > 0.0 {
            self.vel = self.vel.clamp_length_max(self.env.max_speed);
        }
        // The hover cushion: over ground nearer than its street height it
        // pushes the hull up to it (how it takes off and lands).
        if g > 0.05 {
            let from = self.trans + Vec3::Y * meters(0.5);
            let motion = -Vec3::Y * (SETTLE + meters(2.5));
            self.cache.fill_line_sphere(world, from, motion, 409.6);
            if let Some(hit) = self.cache.probe_line_sphere(from, motion, 409.6) {
                let below = motion.length() * hit.u - meters(0.5);
                let error = SETTLE - below;
                if error > 0.0 {
                    let up = (error * 6.0).min(meters(8.0));
                    if self.vel.y < up {
                        self.vel.y += (up - self.vel.y) * (8.0 * dt).min(1.0);
                    }
                }
            }
        }
        self.travel(dt, world);
        self.climb = self.vel.y;
        let turning = (0.25 * self.spin.y.abs() + 0.3 * self.spin.z.abs() + 0.2 * self.spin.x.abs()).clamp(0.0, 1.0);
        self.wind = seek(self.wind, turning, 2.0 * dt);
    }

    /// The hover: gravity, held at the flight level over whatever lies
    /// under the lift thrusters (1.5 m on the street, higher once R2 lifts
    /// it). Piloted over a drop it holds its height; left alone it falls.
    fn hover_spring(&mut self, dt: f32, world: &mut dyn CollideWorld) {
        let height = if self.piloted { self.altitude } else { SETTLE };
        // Far enough to find the ground from the highest level, so lowering
        // a level always has something to settle to.
        // Searched no deeper than this: higher up there is only the climb.
        let reach = if self.piloted {
            self.altitude.clamp(FLIGHT_LEVELS[2], meters(48.0)) + meters(6.0)
        } else {
            height + meters(4.5)
        };
        let gravity = PARKED_GRAVITY * self.env.gravity.clamp(0.0, 1.0);
        self.vel.y -= gravity * dt;
        let q = Quat::from_rotation_y(self.yaw);
        let mut floor = None;
        for local in [Vec3::new(0.0, meters(0.5), meters(2.0)), Vec3::new(0.0, meters(0.5), meters(-3.0))] {
            let at = self.trans + q * local;
            let motion = -Vec3::Y * reach;
            self.cache.fill_line_sphere(world, at, motion, 409.6);
            if let Some(hit) = self.cache.probe_line_sphere(at, motion, 409.6) {
                let h = reach * hit.u;
                floor = Some(floor.map_or(h, |f: f32| f.min(h)));
            }
        }
        // Coming down from high up, the hover takes the ground it finds
        // and holds the height it is at.
        if let Some(h) = floor
            && self.piloted
            && self.altitude > reach
        {
            self.altitude = (h - meters(0.5)).max(FLIGHT_LEVELS[0]);
        }
        let height = if self.piloted { self.altitude } else { SETTLE };
        match floor {
            Some(h) => {
                let error = height + meters(0.5) - h;
                // A firm spring near the ground, a gentler climb to the
                // higher levels.
                let k = if error > meters(2.0) { 6.0 } else { 30.0 };
                let target_v = (error * k / 8.0).clamp(-meters(12.0), meters(12.0));
                self.vel.y += (target_v - self.vel.y) * (8.0 * dt).min(1.0) + gravity * dt;
            }
            // Over a drop, or high up: it rises and sinks as asked.
            None if self.piloted => {
                self.vel.y += gravity * dt;
                self.vel.y += (self.climb - self.vel.y) * (4.0 * dt).min(1.0);
            }
            None => {}
        }
    }
    /// The street HellCat's hover with nobody at the controls: it falls,
    /// rides 1.5 m over the ground and coasts to a stop. A wreck just falls.
    fn hover(&mut self, dt: f32, world: &mut dyn CollideWorld) {
        if self.destroyed {
            self.vel.y -= PARKED_GRAVITY * dt;
        } else if self.idling {
            // Running, nobody at the controls: it settles where it is and
            // bobs there on its thrusters.
            self.vel *= (-3.0 * dt).exp();
            self.vel.y += (self.time as f32 * 0.02).sin() * meters(0.15) * dt;
        } else if self.hold {
            self.vel *= (-4.0 * dt).exp();
        } else {
            self.hover_spring(dt, world);
            let damp = (-2.0 * dt).exp();
            self.vel.x *= damp;
            self.vel.z *= damp;
        }
        self.roll += -self.roll * (4.0 * dt).min(1.0);
        self.pitch += -self.pitch * (4.0 * dt).min(1.0);
        self.travel(dt, world);
    }

    /// Moves the six spheres by the velocity, bouncing off what they meet
    /// (`bounce-factor` 0.4).
    fn travel(&mut self, dt: f32, world: &mut dyn CollideWorld) {
        let q = Quat::from_rotation_y(self.yaw);
        let mut remaining = self.vel * dt;
        for _ in 0..3 {
            if remaining.length_squared() < 1.0 {
                break;
            }
            let mut contact: Option<(f32, Vec3, Vec3)> = None;
            for s in SPHERES {
                let center = self.trans + q * Vec3::new(s[0], s[1], s[2]);
                self.cache.fill_line_sphere(world, center, remaining, s[3]);
                if let Some(h) = self.cache.probe_line_sphere(center, remaining, s[3])
                    && contact.is_none_or(|c| h.u < c.0)
                {
                    let point = center + remaining * h.u - h.normal * s[3];
                    contact = Some((h.u, h.normal, point));
                }
            }
            let Some((u, n, point)) = contact else {
                self.trans += remaining;
                break;
            };
            self.trans += remaining * (u - 0.01).max(0.0);
            if self.piloted {
                self.events.push(HellCatEvent::Brush { pos: point, normal: n });
            }
            let into = self.vel.dot(n);
            if into < 0.0 {
                let hard = -into / METER;
                // Skimming the ground is the hover's business; walls and
                // hard landings hurt.
                if hard > 8.0 && (n.y.abs() < 0.7 || hard > 15.0) && self.piloted {
                    self.impact_time = self.time;
                    self.events.push(HellCatEvent::Crash {
                        pos: self.trans,
                        speed: hard,
                    });
                    self.damage();
                }
                self.vel -= n * (1.4 * into);
                let tangent = self.vel - n * self.vel.dot(n);
                self.vel -= tangent * 0.05;
            }
            remaining = self.vel * dt * (1.0 - u).max(0.0);
        }
    }

    /// R1: the nose gun, four shots in any second, aimed at the nearest
    /// creature within 60° and 405 m. L1: a torpedo, 2 s apart.
    fn guns(&mut self, input: &PadInput, world: &mut dyn CollideWorld, actors: Option<&mut dyn ActorWorld>) {
        let now = self.time;
        let q = self.orientation();
        let forward = q * Vec3::Z;
        if input.pressed & button::R1 != 0 {
            let slot = (0..4).min_by_key(|&i| self.fire_slots[i]).unwrap_or(0);
            if now - self.fire_slots[slot] >= seconds(1.0) {
                self.fire_slots[slot] = now;
                let muzzle = self.trans + q * GUN_MUZZLE;
                let aimed = actors.and_then(|a| {
                    a.actors_near(muzzle, meters(405.0))
                        .into_iter()
                        .filter(|(_, c)| (*c - muzzle).normalize_or_zero().dot(forward) > 0.5)
                        .min_by(|x, y| {
                            let ax = (x.1 - muzzle).normalize_or_zero().dot(forward);
                            let ay = (y.1 - muzzle).normalize_or_zero().dot(forward);
                            ay.total_cmp(&ax)
                        })
                        .map(|(_, c)| c)
                });
                let vel = match aimed {
                    Some(c) => (c - muzzle).normalize_or(forward) * GUN_SPEED,
                    None => forward * GUN_SPEED + self.vel,
                };
                self.shots.push(Shot {
                    trans: muzzle,
                    tail: muzzle,
                    vel,
                    spawn: now,
                    torpedo: false,
                });
                self.events.push(HellCatEvent::Fire { pos: muzzle });
            }
        }
        if input.pressed & button::L1 != 0
            && self.torpedoes > 0
            && now - self.torpedo_time >= seconds(2.0)
        {
            self.torpedo_time = now;
            if self.torpedoes == TORPEDO_MAX {
                self.refill_time = now;
            }
            self.torpedoes -= 1;
            let muzzle = self.trans + q * TORPEDO_MUZZLE;
            self.shots.push(Shot {
                trans: muzzle,
                tail: muzzle,
                vel: forward * TORPEDO_SPEED + self.vel,
                spawn: now,
                torpedo: true,
            });
            self.events.push(HellCatEvent::FireAlt { pos: muzzle });
        }
        let _ = world;
    }

    fn step_shots(&mut self, world: &mut dyn CollideWorld, mut actors: Option<&mut dyn ActorWorld>) {
        let now = self.time;
        let mut shots = std::mem::take(&mut self.shots);
        shots.retain_mut(|s| {
            let age = now - s.spawn;
            if s.torpedo && age >= seconds(0.5) {
                self.burst(s.trans);
                return false;
            }
            if !s.torpedo && age >= seconds(2.0) {
                return false;
            }
            let motion = s.vel * SECONDS_PER_FRAME;
            let radius = if s.torpedo { 819.2 } else { 819.2 };
            self.cache.fill_line_sphere(world, s.trans, motion, radius);
            let wall = self.cache.probe_line_sphere(s.trans, motion, radius);
            let actor = crate::projectile::reborrow(&mut actors)
                .and_then(|a| a.actor_hit(s.trans, motion, if s.torpedo { 2457.6 } else { 4096.0 }))
                .filter(|(_, u, _)| wall.is_none_or(|w| *u < w.u));
            let dir = s.vel.normalize_or(Vec3::Z);
            if let Some((key, u, point)) = actor {
                s.trans += motion * u;
                if s.torpedo {
                    self.burst(s.trans - dir * 2048.0);
                } else {
                    self.events.push(HellCatEvent::Impact(ProjectileHit {
                        pos: point,
                        surface: point,
                        normal: -dir,
                        damage: 2.0,
                        actor: Some(key),
                        dir,
                    }));
                }
                return false;
            }
            if let Some(h) = wall {
                s.trans += motion * h.u;
                if s.torpedo {
                    self.burst(s.trans - dir * 2048.0);
                } else {
                    self.events.push(HellCatEvent::Impact(ProjectileHit {
                        pos: s.trans,
                        surface: h.intersect,
                        normal: h.normal,
                        damage: 2.0,
                        actor: None,
                        dir,
                    }));
                }
                return false;
            }
            s.trans += motion;
            let max = if s.torpedo { meters(40.0) } else { meters(30.0) };
            let trail = s.tail - s.trans;
            if trail.length() > max {
                s.tail = s.trans + normalize(trail, max);
            }
            true
        });
        self.shots = shots;
    }

    fn burst(&mut self, center: Vec3) {
        self.blasts.push(Blast {
            center,
            start: self.time,
            hit: Vec::new(),
        });
        self.events.push(HellCatEvent::Blast { center });
        self.events.push(HellCatEvent::Impact(ProjectileHit {
            pos: center,
            surface: center,
            normal: Vec3::Y,
            damage: 128.0,
            actor: None,
            dir: -Vec3::Y,
        }));
    }

    /// The torpedo's blast sweeps out to 60 m in half a second, striking
    /// each creature once for 128.
    fn step_blasts(&mut self, mut actors: Option<&mut dyn ActorWorld>) {
        let now = self.time;
        let mut blasts = std::mem::take(&mut self.blasts);
        blasts.retain_mut(|b| {
            let t = (now - b.start) as f32 / TICKS_PER_SECOND as f32;
            let radius = (BLAST_GROWTH * t).min(BLAST_RADIUS);
            if radius < BLAST_RADIUS
                && let Some(a) = crate::projectile::reborrow(&mut actors)
            {
                for (key, c) in a.actors_near(b.center, radius) {
                    if !b.hit.contains(&key) {
                        b.hit.push(key);
                        let dir = (c - b.center).normalize_or(Vec3::Y);
                        self.events.push(HellCatEvent::Impact(ProjectileHit {
                            pos: c,
                            surface: c,
                            normal: -dir,
                            damage: 128.0,
                            actor: Some(key),
                            dir,
                        }));
                    }
                }
            }
            t < 1.5
        });
        self.blasts = blasts;
    }

    /// Shots and blasts as beams.
    pub fn beams(&self) -> Vec<Beam> {
        let mut out = Vec::new();
        for s in &self.shots {
            out.push(Beam {
                head: s.trans,
                tail: s.tail,
                rgba: if s.torpedo {
                    [1.0, 0.55, 0.2, 1.0]
                } else {
                    [1.0, 0.85, 0.4, 1.0]
                },
                width: if s.torpedo { 2.5 } else { 1.4 },
            });
        }
        for b in &self.blasts {
            let t = (self.time - b.start) as f32 / TICKS_PER_SECOND as f32;
            let radius = (BLAST_GROWTH * t).min(BLAST_RADIUS);
            let fade = (1.0 - (t - 0.5).max(0.0)).clamp(0.0, 1.0);
            for k in 0..3 {
                let y = b.center.y + radius * 0.25 * k as f32;
                let r = radius * (1.0 - 0.25 * k as f32);
                for i in 0..32 {
                    let p = |i: u32| {
                        let a = i as f32 / 32.0 * std::f32::consts::TAU;
                        Vec3::new(b.center.x + a.cos() * r, y, b.center.z + a.sin() * r)
                    };
                    out.push(Beam {
                        head: p(i + 1),
                        tail: p(i),
                        rgba: [1.0, 0.6, 0.25, fade],
                        width: 6.0,
                    });
                }
            }
        }
        out
    }
}

/// The exhausts (`h-warf`'s `thruster-local-pos`), in the hull's frame.
const THRUSTERS: [Vec3; 2] = [Vec3::new(6144.0, 4096.0, -17612.8), Vec3::new(-6144.0, 4096.0, -17612.8)];

impl HellCat {
    /// The engine's flames as `h-warf`'s `draw-thrusters` lays them each
    /// frame: from each exhaust, fifteen sprites out along the jet, the
    /// near ones biggest, longer and hotter with the engine; the jet bent
    /// against the turn and the roll. Part 2761 (`laser-hit2`, white
    /// going yellow) and part 2762 (`glow`, blue going red) of
    /// `fac-part.gc`, with the values `draw-thrusters` writes into them;
    /// both added on (`sp-cpuinfo-flag-3`).
    ///
    /// Flying about, the flames follow the engine up to half their size;
    /// the full, orange-hot jet ([`HellCat::thruster_sprites_with`] at 1)
    /// is for the boost between planets. Parked or idling there are none.
    pub fn thruster_sprites(&self) -> Vec<crate::particles::Sprite> {
        let idle = if self.piloted || self.idling { 0.08 } else { 0.0 };
        self.thruster_sprites_with(if self.boosting { 1.0 } else { (0.5 * self.engine).max(idle) })
    }

    /// The flames at `thrust` (`engine-thrust`, 0 to 1).
    pub fn thruster_sprites_with(&self, thrust: f32) -> Vec<crate::particles::Sprite> {
        use crate::particles::Sprite;
        let thrust = thrust.clamp(0.0, 1.0);
        if self.destroyed || thrust <= 0.02 {
            return Vec::new();
        }
        let q = self.orientation();
        let (right, up) = (q * Vec3::X, q * Vec3::Y);
        let back = q * -Vec3::Z;
        let angle = std::f32::consts::TAU / 65536.0;
        let mut seed = (self.time as u32).wrapping_mul(2_654_435_761) | 1;
        let mut random = |lo: f32, hi: f32| {
            seed ^= seed << 13;
            seed ^= seed >> 17;
            seed ^= seed << 5;
            lo + (hi - lo) * ((seed >> 8) as f32 / (1u32 << 24) as f32)
        };
        let mut out = Vec::with_capacity(2 * 15 * 4);
        for (index, local) in THRUSTERS.iter().enumerate() {
            let side = if index == 0 { -1.0 } else { 1.0 };
            // `langvel`: pitch rate about x, turn rate about y, roll about z,
            // turned in that order (each `vector-rotate-around-axis!`).
            let dir = Quat::from_axis_angle(right, 182.04445 * self.roll_rate * 5.0 * side * angle)
                * Quat::from_axis_angle(up, -1820.4445 * self.yaw_rate * angle)
                * Quat::from_axis_angle(right, -2184.5334 * self.pitch_rate * angle)
                * back;
            let mut at = self.trans + q * *local;
            for step in 0..15 {
                let f = (15 - step) as f32 / 15.0;
                let small = f * random(8192.0, 9011.2);
                let large = f * random(8192.0, 12288.0);
                at += dir * (819.2 * f + 1024.0 * thrust * f);
                out.push(Sprite {
                    screen_aligned: false,
                    pos: at,
                    size: [small, small],
                    rotation: 0.0,
                    rgba: [
                        215.0 * thrust + 40.0 * f,
                        255.0,
                        // `(the float (- 255 (the int ...)))`: whole steps.
                        255.0 - (255.0 * thrust).trunc(),
                        random(0.0, 5.0) + 60.0 * f * thrust + 40.0 * f,
                    ],
                    texture: "laser-hit2".into(),
                    blend: crate::particles::Blend::Add,
                    corner: 0,
                    stretch: None,
                    flat: None,
                });
                let glow_alpha = random(0.0, 40.0 * thrust) + 20.0 * f * thrust + 30.0 * f.floor();
                // `:num 3.0`; the written values keep their specs' random
                // ranges (`:g 100.0 28.0`, `:a 30.0 8.0`), drawn for each.
                for _ in 0..3 {
                    let green = 28.0 - (28.0 * f * thrust).trunc() + random(0.0, 28.0);
                    let alpha = glow_alpha + random(0.0, 8.0);
                    out.push(Sprite {
                        screen_aligned: false,
                        pos: at,
                        size: [large, large],
                        rotation: 0.0,
                        rgba: [
                            255.0 * f * thrust,
                            green,
                            255.0 - (255.0 * f * thrust).trunc(),
                            alpha,
                        ],
                        texture: "glow".into(),
                        blend: crate::particles::Blend::Add,
                        corner: 0,
                        stretch: None,
                        flat: None,
                    });
                }
            }
        }
        out
    }
}

#[cfg(test)]
mod gear_tests {
    use super::*;

    /// X tapped twice, then a quiet frame.
    fn shift_up(craft: &mut HellCat) {
        let tap = PadInput { held: button::R2, pressed: button::R2, ..Default::default() };
        craft.time += FRAME_TICKS;
        craft.controls(&tap, SECONDS_PER_FRAME);
        craft.time += FRAME_TICKS;
        craft.controls(&tap, SECONDS_PER_FRAME);
        craft.time += seconds(1.0);
        craft.controls(&PadInput::default(), SECONDS_PER_FRAME);
    }

    /// `draw-thrusters`: a pitching hull bends its jet about its right axis
    /// (`langvel` x, -2184.5334), and the blue/green channels step whole.
    #[test]
    fn flames_bend_with_the_pitch_and_keep_whole_channels() {
        let mut craft = HellCat::new(Vec3::ZERO, 0.0);
        let level = craft.thruster_sprites_with(0.37);
        craft.pitch_rate = 2.0;
        let pitched = craft.thruster_sprites_with(0.37);
        let last = |s: &[crate::particles::Sprite]| s[s.len() - 1].pos;
        assert!(last(&level).distance(last(&pitched)) > 100.0);
        // One exhaust's fifteen samples, four sprites each.
        let jet = |s: &[crate::particles::Sprite]| (s[56].pos - s[0].pos).normalize();
        assert!(jet(&level).dot(Vec3::X).abs() < 1e-3 && jet(&pitched).dot(Vec3::X).abs() < 1e-3);
        for s in &level {
            assert_eq!(s.rgba[2].fract(), 0.0);
        }
    }

    #[test]
    fn three_gears_and_no_more() {
        let mut craft = HellCat::new(Vec3::ZERO, 0.0);
        for _ in 0..6 {
            shift_up(&mut craft);
        }
        assert_eq!(craft.gear, 2, "it stops at fast");
        craft.env.gravity = 0.0;
        for _ in 0..6 {
            shift_up(&mut craft);
        }
        assert_eq!(craft.gear, 2, "out in space too");
    }
}

#[cfg(test)] mod analog_vehicle_tests {
    use super::*;
    #[test] fn architecture27_hellcat_uses_analog_pedals_and_remapped_roll() {
        let mut craft=HellCat::new(glam::Vec3::ZERO,0.);
        let throttle=PadInput{held:button::R2,triggers:[0.,0.515],..Default::default()};
        for _ in 0..120{craft.controls(&throttle,1./60.);}
        assert!((craft.throttle-0.5).abs()<0.001);assert_eq!(craft.lift,0.);
        let brake=PadInput{held:button::L2,triggers:[0.2725,0.],..Default::default()};
        for _ in 0..120{craft.controls(&brake,1./60.);}
        assert!((craft.brake-0.25).abs()<0.001);assert_eq!(craft.throttle,0.);assert_eq!(craft.lift,0.);
        craft.controls(&PadInput{held:button::CIRCLE,..Default::default()},1./60.);assert_eq!(craft.lift,1.);
        craft.controls(&PadInput{held:button::X|button::R2,..Default::default()},1./60.);
        assert!(craft.boosting);assert_eq!(craft.lift,0.);
        craft.controls(&PadInput{held:button::SQUARE,..Default::default()},1./60.);assert_eq!(craft.lift,-1.);
    }
}
