//! Swimming (`target-swim.gc`): in water deeper than he stands, Jak swims
//! at the surface with `*swim-mods*` (a 7 m/s stroke, slow turns, guns
//! away), his head above it. Square or L1 takes him under, as
//! `target-swim-down` does; there, unlike the original's fixed dive, the
//! player steers up and down (X rises, Square or L1 sinks; letting go he
//! swims on level with the surface stroke, or drifts back up when still).
//! X at the surface jumps him out (`swim-jump`).
//!
//! The host says where the water's surface is each frame
//! ([`Jak::water`]); without water he never swims.
use glam::{Mat3, Vec3};

use crate::anim::{self, Anim, NumFunc};
use crate::collide::CollideWorld;
use crate::math::*;
use crate::pad::button;
use crate::particles::Overrides;
use crate::{Event, Jak, State};

/// How deep the water is before he swims, and where he floats: the
/// surface this far above his feet (`swim-height`, 1.8 m).
pub const SWIM_HEIGHT: f32 = 7372.8;
/// Shallower than this he wades out and stands again.
const LEAVE_DEPTH: f32 = 5734.4;
/// Water at least this deep over his feet and he wades (`target-wade`).
pub const WADE_DEPTH: f32 = meters(0.5);

/// Swimming, he wades out only this far shallower than he floats.
const LEAVE_UNDER_FLOAT: f32 = meters(0.4);

/// How far the top of his head stays over the water as he floats.
pub const CROWN_OUT: f32 = meters(0.3);
/// How fast his feet settle toward the swim height at the surface, meters a second.
const FLOAT_SPEED: f32 = meters(3.5);
/// Sinking: `target-swim-down` sets `gravity-length` and `gravity-max` to 16384 (4 m/s a second up to 4 m/s), which
/// is how he falls under water: from the speed he has, a second to the full one. Adrii 06/11 asked for faster dives
/// and rises; the old 3 m/s was under the original's 4, so this is the original's own, with no extra factor.
const DIVE_GRAVITY: f32 = 16384.0;
const DIVE_SPEED: f32 = 16384.0;
/// Rising: `target-swim-up` sets `transv.y = 8192 + 8192 * f30`, `f30` 1.0 (4 m/s) and 2.0 (6 m/s) from an X press,
/// seeking back to 1.0 one a second. The same rise when he lets go of the dive still (the original always swims
/// up then; here, moving, he keeps his depth).
const RISE_BASE: f32 = 8192.0;
const RISE_BOOST: f32 = 2.0;
/// `target-swim-down`'s entry plays `jakb-swim-walk-to-down`; at its artist frame 73 (once) a dive from the
/// surface sounds "swim-dive" and splashes `spawn-ripples 0.7`.
const DIVE_FX_AFRAME: f32 = 73.0;
/// `*TARGET-bank*` `min-dive-depth`: the ground must be this far under his feet to dive (`target-swim-stance`, `-walk`, `-up`).
const MIN_DIVE_DEPTH: f32 = meters(2.0);
/// `target-flop`'s `swim` event dives him only from over this much water (`(< 6144.0 (target-height-above-ground))`).
const FLOP_DIVE_HEIGHT: f32 = 6144.0;
/// `target-swim-up` over `jakb-swim-down-to-up`: "swim-surface" and its ripples at artist frame 240 or as his head is out,
/// and out to the stance from 222 (200 with the stick pushed) once his feet are over the float line.
const SURFACE_AFRAME: f32 = 240.0;
const LIFT_EXIT_AFRAME: f32 = 222.0;
const LIFT_EXIT_MOVING_AFRAME: f32 = 200.0;
/// The jump out of the water (`swim-jump-height-min`..`max`), over the
/// height he floats at: Minecraft's banks stand straight out of the water,
/// their top a little over its surface, and his feet must clear them.
const SWIM_JUMP_HEIGHT: f32 = meters(1.1);
/// Pushing against a bank this long while floating, he climbs out by
/// himself (seconds).
const CLIMB_OUT_AFTER: f32 = 0.2;

/// Where he is in the water.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Swim {
    /// Under the surface.
    pub diving: bool,
    /// When he went into the water (ticks).
    pub since: i64,
    /// How long he has pushed against a bank at the surface, seconds.
    pub blocked: f32,
    /// `target-swim-up`'s `f30`: his rise is `RISE_BASE` times one plus it; 1.0, an X press makes it 2.0 and it
    /// goes back to 1.0 at one a second.
    pub rise: f32,
    /// `target-swim-down`'s first part, while its entry animation plays.
    pub entry: Option<DiveEntry>,
    /// Whether the dive input was held last frame (`recently-pressed?` of the next).
    pub down_held: bool,
    /// When `target-swim-down` or `-up` began (`state-time`, ticks).
    pub state_since: i64,
    /// The ticks `target-swim-down` runs before it lets go of the dive (`gp-0`: 60, 120 from a flop).
    pub min_dive: i64,
    /// In `target-swim-up` (else `target-swim-down`, under the surface).
    pub rising: bool,
    /// `target-swim-up` over `jakb-swim-down-to-up`, before its loop.
    pub lift: Option<Lift>,
    /// At the surface again after the rise, `jakb-swim-up-to-stance` playing.
    pub settle: bool,
}

impl Swim {
    /// `target-swim-down` begins: its state-time, its minimum, and its entry from the surface or from `swim-up`.
    fn begin_down(&mut self, now: i64, from_surface: bool) {
        self.diving = true;
        self.rising = false;
        self.lift = None;
        self.settle = false;
        self.state_since = now;
        self.min_dive = seconds(0.2);
        self.entry = Some(DiveEntry { from_surface, effects: false });
    }
}

/// `target-swim-up`'s code up to its loop: `jakb-swim-down-to-up` played through (its motion up while he is under water).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Lift {
    /// "swim-surface" and its ripples are made (`gp-0`).
    pub surfaced: bool,
    /// His feet are still under the float line (`s5-0`): the animation carries him.
    pub under: bool,
}

/// `target-swim-down`'s code up to its loop: `jakb-swim-walk-to-down` played through, its own motion (`align!`
/// adjust-y-vel adjust-xz-vel) moving him, before the dive proper.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DiveEntry {
    /// He came from `target-swim-stance` or `target-swim-walk` (`prev-state`): only then the second splash and "swim-dive".
    pub from_surface: bool,
    /// The aframe-73 effects have been made.
    pub effects: bool,
}

/// What `water-control` keeps of him from frame to frame, for the effects it launches.
#[derive(Clone, Debug)]
pub(crate) struct WaterFx {
    /// His head's height last frame (`top 1`), while there was water.
    head: Option<f32>,
    /// His feet under the surface (`touch-water`).
    touching: bool,
    /// When they last went under it (`enter-water-time`, ticks).
    entered: i64,
    /// `distort-time`: when `spawn-ripples` last made its ripple mesh.
    distort: i64,
    /// The fraction of a drop that carries from one frame to the next.
    carry: f32,
    /// The generator behind the joint the drops come off (`rand-vu-int-range`).
    seed: u32,
    /// How deep the water is here, the surface over the ground below it, when that ground is near enough to
    /// matter (none: deeper).
    pub(crate) column: Option<f32>,
    /// How wide the water is round him, along its narrower way across: banks within [`BASIN_REACH`] each way
    /// (none: wider). The original's water is a lake; a pool of one block cannot take its ripples.
    pub(crate) basin: Option<f32>,
    /// When the basin was last measured (ticks).
    basin_at: i64,
    /// The wake's light trail, if its tracker is alive (`ripple`, a handle), and the number of the next.
    wake: Option<u64>,
    next_wake: u64,
    /// The ripple meshes `spawn-ripples` has made, each playing its `idle` once (the manipy's `play1`).
    ripples: Vec<Ripple>,
    /// The wade walk's cycle (the channel his feet's ripples are read from) and when a foot last made one.
    wade: crate::anim::Channel,
    stepped: i64,
    /// Whether the last frame was the circle's spin (`target-attack` out of the wade states).
    spinning: bool,
    /// Whether the last frame was a running attack's (`target-running-attack`, Dark Jak's too).
    punching: bool,
}

/// One `skel-generic-ripples` manipy: where, how big (its root scale's x and z) and when it began.
#[derive(Clone, Copy, Debug)]
struct Ripple {
    at: Vec3,
    scale: f32,
    start: i64,
}

impl Default for WaterFx {
    fn default() -> Self {
        Self {
            head: None,
            touching: false,
            entered: i64::MIN / 2,
            distort: i64::MIN / 2,
            carry: 0.0,
            seed: 0x9e37_79b9,
            column: None,
            basin: None,
            basin_at: i64::MIN / 2,
            wake: None,
            next_wake: 0,
            ripples: Vec::new(),
            wade: crate::anim::Channel::default(),
            stepped: i64::MIN / 2,
            spinning: false,
            punching: false,
        }
    }
}

/// `collectables-generic-ripples-idle`: how long the ripple mesh plays (0.9333 s), then its manipy is gone.
const RIPPLE_LIFE: i64 = seconds(0.9334);
/// `wade-shallow-walk-cycle-dist` and `wade-deep-walk-cycle-dist` (`target-util.gc`): the ground a cycle covers.
const WADE_SHALLOW_CYCLE: f32 = meters(6.0);
const WADE_DEEP_CYCLE: f32 = meters(6.0);
/// The water over his feet for his steps to splash (`(< (- height trans.y) 4096.0)`), at his own size.
const WADE_STEP_DEPTH: f32 = meters(1.0);
/// `water-control` `ripple-size` (`water-h.gc`): the random range of the ripple's scale.
const RIPPLE_SIZE: f32 = 1638.4;

/// `spawn-ripples` and the wake sit this far over the water's surface (`(+ 40.96 surface-height)`).
pub(crate) const SURFACE_LIFT: f32 = 40.96;
/// How far each way the banks are looked for (3 blocks), and the narrowest water that gets the wake's flat ripples
/// (its ripple grows to some 2.5 m across, the splash's rings to six: a block of water is 1.56 m).
const BASIN_REACH: f32 = meters(4.7);
pub(crate) const BASIN_WIDE: f32 = meters(3.2);

impl Jak {
    /// Ground to climb onto just ahead: its top no higher than a meter over
    /// the water's surface (a bank, not a wall to swim along).
    fn low_bank(&self, world: &mut dyn CollideWorld, surface: f32) -> bool {
        let ahead = self.control.quat * Vec3::Z;
        let ahead = Vec3::new(ahead.x, 0.0, ahead.z).normalize_or_zero();
        let from = Vec3::new(self.control.trans.x, surface + meters(1.0), self.control.trans.z) + ahead * meters(0.8);
        let motion = Vec3::new(0.0, -meters(1.4), 0.0);
        let mut cache = crate::collide::CollideCache::default();
        cache.fill_line_sphere(world, from, motion, meters(0.2));
        cache
            .probe_line_sphere(from, motion, meters(0.2))
            .is_some_and(|hit| hit.u > 0.0)
    }

    /// The size the water measures him by: his drawn one.
    pub(crate) fn swim_size(&self) -> f32 {
        self.drawn_size.unwrap_or(self.size)
    }

    /// How far under the surface his feet float: the swim height, less
    /// if the pose would hold the top of his head under the water.
    pub(crate) fn float_depth(&self) -> f32 {
        let (size, swim_height) = (self.swim_size(), SWIM_HEIGHT * self.swim_size());
        match self.crown {
            Some(crown) => swim_height.min((crown.y - CROWN_OUT) * size),
            None => swim_height,
        }
    }

    /// Wading, how much of his walking speed he keeps: from 0.8 in half a
    /// meter of water to 0.6 in two (`target-wade-walk`'s 10 m/s times
    /// that: 7.33 m/s in a meter).
    pub(crate) fn wade_scale(&self) -> f32 {
        match self.wade {
            Some(depth) => {
                let t = ((depth / self.swim_size() - WADE_DEPTH) / meters(1.5)).clamp(0.0, 1.0);
                0.8 - 0.2 * t
            }
            None => 1.0,
        }
    }

    pub fn swimming(&self) -> bool {
        self.swim.is_some()
    }

    /// Which of the water's looped sounds play: `swim-bubbles` (`target-swim-stance`, `target-swim-walk`: every frame at
    /// the surface) and `water-bubbles` (`target-swim-up` every frame, `target-swim-down` from half a second in).
    pub fn bubble_loops(&self) -> (bool, bool) {
        let Some(swim) = self.swim else { return (false, false) };
        let deep = swim.diving && (swim.rising || (swim.entry.is_none() && self.time - swim.state_since >= seconds(0.5)));
        (!swim.diving, deep)
    }

    /// One frame in the water; false when he is not swimming (and the
    /// usual states run).
    pub(crate) fn swim_step(&mut self, world: &mut dyn CollideWorld) -> bool {
        self.wade = self.water.and_then(|surface| {
            let depth = surface - self.control.trans.y;
            let on_ground = self.control.status & crate::control::status::ON_GROUND != 0;
            (self.swim.is_none() && on_ground && depth >= WADE_DEPTH * self.swim_size() && depth < SWIM_HEIGHT * self.swim_size())
                .then_some(depth)
        });
        // Triangle by a Flut Flut, swimming: onto it.
        if self.flut_mount_from_water() {
            return false;
        }
        let Some(surface) = self.water else {
            self.swim_tilt = 0.0;
            if self.swim.take().is_some() {
                self.control.mod_surface = crate::surface::WALK;
            }
            return false;
        };
        let depth = surface - self.control.trans.y;
        // How deep and how wide the water is here, for the size of the splash it gets and for whether its flat
        // ripples have room ([`Jak::water_particles`]).
        let reach = SWIM_HEIGHT * self.swim_size() + meters(1.0);
        let top = Vec3::new(self.control.trans.x, surface, self.control.trans.z);
        self.water_fx.column = Self::probe(world, top, Vec3::new(0.0, -reach, 0.0), meters(0.1))
            .map(|floor| (surface - floor.intersect.y).max(0.0));
        if self.time_elapsed(self.water_fx.basin_at, seconds(0.1)) {
            self.water_fx.basin_at = self.time;
            // The banks are the solid blocks at the water's own height: along each axis, the free run each way.
            let level = Vec3::new(top.x, surface - meters(0.3) * self.swim_size(), top.z);
            let run = |dir: Vec3, world: &mut dyn CollideWorld| {
                Self::probe(world, level, dir * BASIN_REACH, meters(0.05)).map_or(BASIN_REACH, |hit| hit.u * BASIN_REACH)
            };
            let across_x = run(Vec3::X, world) + run(-Vec3::X, world);
            let across_z = run(Vec3::Z, world) + run(-Vec3::Z, world);
            let width = across_x.min(across_z);
            self.water_fx.basin = (width < 2.0 * BASIN_REACH - 1.0).then_some(width);
        }
        // As deep as his drawn body would float in his own game.
        let (swim_height, leave_depth) = (SWIM_HEIGHT * self.swim_size(), LEAVE_DEPTH * self.swim_size());
        if self.swim.is_none() {
            // Dark and Light Jak swim as he does.
            // Dropping into it (off the board as it slows, from a jump)
            // he swims as soon as it is past his knees, not running on as he
            // sinks to swimming depth (15/10).
            let falling_in = self.control.status & crate::control::status::ON_GROUND == 0
                && self.control.transv.y <= 0.0
                && depth >= 0.35 * swim_height;
            // On the Flut Flut it swims instead ([`crate::flut_swim`]).
            if (depth < swim_height && !falling_in) || self.state.is_board() || self.state.is_flut() {
                return false;
            }
            // `target-flop`'s `swim` event: from over 1.5 m of water the flop goes on into `target-swim-down` (with
            // "swim-flop"); lower it lands (`target-flop-hit-ground`), which here is the usual swim start.
            let flop_dive = matches!(self.state, State::Flop { .. })
                && self.control.trans.y - self.control.gspot_pos.y > FLOP_DIVE_HEIGHT * self.swim_size();
            self.swim = Some(Swim {
                diving: flop_dive,
                since: self.time,
                blocked: 0.0,
                rise: 1.0,
                entry: None,
                down_held: false,
                state_since: self.time,
                min_dive: seconds(if flop_dive { 0.4 } else { 0.2 }),
                rising: false,
                lift: None,
                settle: false,
            });
            self.gun.end_mode();
            self.state = State::Stance;
            self.state_time = self.time;
            if flop_dive {
                // stand-in: Jak 3's banks have no "swim-flop" (its `sound-play` finds nothing); Jak 1's is the sound of that
                // name, from the same trigger there.
                self.events.push(Event::Sound { name: "jak1:swim_flop", volume: 1.0, pitch: 0.0 });
                // `target-swim-down` from a flop: the channel at artist frame 124 of `swim-down`, and `f30` -16384, which
                // takes him straight to the dive's top speed.
                self.control.transv.y = -DIVE_SPEED;
                self.ja_push(22);
                let frame = self.anims.aframe(anim::SWIM_DOWN, 124.0);
                self.chan.set(anim::SWIM_DOWN, NumFunc::Loop { rate: 1.0 }, frame);
            }
            // (Its splash is `enter-water`'s, as his feet touch the water: [`Jak::water_particles`].)
        }
        let Some(mut swim) = self.swim else { return false };
        let down = self.pad.hold(button::SQUARE) || self.pad.hold(button::L1);
        let up = self.pad.hold(button::X);
        // X at the surface, or a bank pushed against: out of the water with
        // a jump high enough to land on it, carried forward onto it.
        let climb_out = swim.blocked >= CLIMB_OUT_AFTER && self.low_bank(world, surface);
        if !swim.diving && (self.pad.pressed(button::X) || climb_out) {
            self.swim = None;
            self.swim_tilt = 0.0;
            let height = self.float_depth() + SWIM_JUMP_HEIGHT * self.swim_size();
            // Out of the water into his own jump (its animation, not the
            // walk the state he swam in would go on with), as high as the
            // bank needs.
            self.go(crate::State::Jump { min: height, max: height });
            // This takeoff is already at the actual floating feet position.
            // The normal ground jump subtracts JUMP_COLLIDE_OFFSET to account
            // for floor separation. Applying that again here consumed nearly
            // all of the clearance over water at the Minecraft avatar scale.
            self.init_var_jump(height,height,true,false,0.0);
            if climb_out || self.pad.stick0_speed > 0.1 {
                let ahead = self.control.quat * Vec3::Z;
                self.control.transv += Vec3::new(ahead.x, 0.0, ahead.z).normalize_or_zero() * meters(3.0);
            }
            self.events.push(Event::Splash {
                pos: Vec3::new(self.control.trans.x, surface, self.control.trans.z),
                // Out of the water: negative.
                speed: -meters(2.0),
            });
            return false;
        }
        // Out of the water only well over where he floats: floating higher
        // for his head (`float_depth`), he never wades out and falls back
        // in over and over (a splash each time).
        let leave_depth = leave_depth.min(self.float_depth() - LEAVE_UNDER_FLOAT * self.swim_size());
        // (Just dropped in he is still sinking to it: not out again yet.)
        let settling = self.time - swim.since < seconds(0.5)
            && self.control.status & crate::control::status::ON_GROUND == 0;
        if depth < leave_depth && !swim.diving && !settling {
            self.swim = None;
            self.swim_tilt = 0.0;
            self.control.mod_surface = crate::surface::WALK;
            return false;
        }
        let now = self.time;
        let was_down = std::mem::replace(&mut swim.down_held, down);
        let moving = self.pad.stick0_speed > 0.1;
        // `(< (-> *TARGET-bank* min-dive-depth) (target-height-above-ground))`: the ground has to be well under his feet.
        let room = self.control.trans.y - self.control.gspot_pos.y > MIN_DIVE_DEPTH * self.swim_size();
        if down && !swim.diving && room {
            // `target-swim-stance` / `-walk`: the dive's own `spawn-ripples 0.7` at the surface as he goes under, then
            // `go target-swim-down`: its entry from the stance or the stroke.
            let at = Vec3::new(self.control.trans.x, surface, self.control.trans.z);
            let big = !self.small_water();
            self.spawn_ripples(0.7, at, surface, big, Vec3::ZERO, true);
            swim.begin_down(now, true);
            // (`ja-channel-push! 1 22`: the old pose blends out over 22 ticks.)
            self.ja_push(22);
            self.chan.set(anim::SWIM_WALK_TO_DOWN, NumFunc::seek(1.0), 0.0);
        } else if swim.diving
            && swim.rising
            && swim.entry.is_none()
            && down
            && !was_down
            && room
            && now - swim.state_since >= seconds(0.05)
        {
            // `target-swim-up` with the dive pressed again: `target-swim-down` from `swim-up` or `swim-down-to-up` plays the
            // same entry (its sound and second splash only after the stance or the stroke), blending in over 22 ticks, 105
            // from the lift.
            let from_lift = self.chan.is(anim::SWIM_DOWN_TO_UP);
            swim.begin_down(now, false);
            self.ja_push(if from_lift { 105 } else { 22 });
            self.chan.set(anim::SWIM_WALK_TO_DOWN, NumFunc::seek(1.0), 0.0);
        }
        // `target-swim-down` lets go of the dive to `target-swim-up` once it is not held and the state has run its minimum
        // (the original always rises then; moving and still, here, he keeps his depth).
        if swim.diving && !swim.rising && swim.entry.is_none() && !down && (up || !moving) && now - swim.state_since >= swim.min_dive {
            swim.rising = true;
            swim.state_since = now;
            swim.lift = Some(Lift { surfaced: false, under: true });
            // `(ja-channel-push! 1 (seconds 0.1))`, then `jakb-swim-down-to-up` from its start.
            self.ja_push(30);
            self.chan.set(anim::SWIM_DOWN_TO_UP, NumFunc::seek(1.0), 0.0);
        }
        // Level swimming under the surface that has come back up to it.
        if swim.diving && !swim.rising && !down && depth <= swim_height && swim.entry.is_none() && now - swim.state_since >= swim.min_dive {
            swim.diving = false;
        }
        // `target-swim-up`'s loop: an X press sets `f30` to 2.0 and it seeks back to 1.0 a second.
        if swim.diving && swim.rising && swim.lift.is_none() && self.pad.pressed(button::X) {
            swim.rise = RISE_BOOST;
        }
        swim.rise = crate::math::seek(swim.rise, 1.0, crate::SECONDS_PER_FRAME);
        // His vertical speed as the last frame left it, for the dive's gravity to work on.
        let was = self.control.transv.y;
        if let Some(mut entry) = swim.entry {
            // The entry's own motion takes him (`compute-delta-align!`, `align!` adjust-y-vel adjust-xz-vel) under the dive's
            // gravity; the controls wait for the animation to be done.
            self.compute_delta_align();
            let gravity = std::mem::replace(&mut self.control.gravity_length, DIVE_GRAVITY);
            self.align(crate::align::opts::Y_VEL | crate::align::opts::XZ_VEL, 1.0, 1.0, 1.0);
            self.control.gravity_length = gravity;
            if !entry.effects && self.chan.aframe_num(&self.anims) >= DIVE_FX_AFRAME {
                entry.effects = true;
                if entry.from_surface {
                    // `(sound-play "swim-dive")`, `(spawn-ripples water 0.7 trans 1 *null-vector* #f)`.
                    self.events.push(Event::Sound { name: "SWIM_DIVE", volume: 1.0, pitch: 0.0 });
                    let big = !self.small_water();
                    self.spawn_ripples(0.7, self.control.trans, surface, big, Vec3::ZERO, false);
                }
            }
            swim.entry = Some(entry);
        }
        // `target-swim-up`: `swim-down-to-up` first, its motion lifting him while his feet are under the float line.
        if swim.diving && swim.rising {
            let under = depth > swim_height;
            let mut gone = !under && swim.lift.is_none();
            if let Some(mut lift) = swim.lift {
                let aframe = self.chan.aframe_num(&self.anims);
                if !lift.surfaced && (aframe >= SURFACE_AFRAME || self.head_y() >= surface) {
                    // `(sound-play "swim-surface")`, `(spawn-ripples water 0.2 trans 1 transv #t)`.
                    lift.surfaced = true;
                    self.events.push(Event::Sound { name: "SWIM_SURFACE", volume: 1.0, pitch: 0.0 });
                    let big = !self.small_water();
                    self.spawn_ripples(0.2, self.control.trans, surface, big, self.control.transv, true);
                }
                gone = !under && (aframe >= LIFT_EXIT_AFRAME || (moving && aframe >= LIFT_EXIT_MOVING_AFRAME));
                if !under {
                    lift.under = false;
                }
                if !gone {
                    self.compute_delta_align();
                    if lift.under {
                        let gravity = std::mem::replace(&mut self.control.gravity_length, DIVE_GRAVITY);
                        self.align(crate::align::opts::Y_VEL, 1.0, 1.0, 1.0);
                        self.control.gravity_length = gravity;
                    }
                }
                swim.lift = Some(lift);
            }
            if gone {
                // `go target-swim-stance` (`-walk` with the stick pushed, blending in over 0.3 s): its code plays
                // `swim-up-to-stance` after a `swim-up` or a `swim-down-to-up`.
                swim.diving = false;
                swim.rising = false;
                swim.lift = None;
                swim.state_since = now;
                if moving {
                    self.ja_push(90);
                } else {
                    swim.settle = true;
                    self.ja_push(22);
                    self.chan.set(anim::SWIM_UP_TO_STANCE, NumFunc::seek(1.0), 0.0);
                }
            }
        }
        let float = |jak: &Self| {
            // Floating: his feet held the swim height under the surface, his head always out of it (Adrii: only Daxter
            // showed). The stroke's own pose lifts his body in his game; whatever the drawn model's size, the top of his
            // head ends over the water.
            ((surface - jak.float_depth() - jak.control.trans.y) * 4.0).clamp(-FLOAT_SPEED, FLOAT_SPEED)
        };
        let vy = if swim.entry.is_some() {
            self.control.transv.y
        } else if swim.diving && swim.rising {
            match swim.lift {
                // The animation's own, while he is under the float line.
                Some(lift) if lift.under => self.control.transv.y,
                Some(_) => float(self),
                // `transv.y = 8192 + 8192 * f30`.
                None => RISE_BASE * (1.0 + swim.rise),
            }
        } else if swim.diving {
            if down || now - swim.state_since < swim.min_dive {
                // `add-gravity` with the dive's `gravity-length` 16384, no faster than `gravity-max`.
                if was <= -DIVE_SPEED { was } else { (was - DIVE_GRAVITY * crate::SECONDS_PER_FRAME).max(-DIVE_SPEED) }
            } else {
                // Swimming along under water he keeps his depth.
                0.0
            }
        } else {
            float(self)
        };
        self.swim = Some(swim);
        // The stroke: the usual walk physics with the swim surface and no
        // gravity, the climb set directly.
        self.control.mod_surface = crate::surface::SWIM;
        let gravity = self.control.gravity_length;
        self.control.gravity_length = 0.0;
        self.control.transv.y = vy;
        self.target_post(world);
        self.control.gravity_length = gravity;
        // The water takes his fall. The original holds him up on the surface each frame he swims (`swim-ground`
        // sets `onsurf`), so `last-trans-any-surf` follows him, and Minecraft resets a fall in water as well.
        // Adrii 06/11: after diving, touching ground he took fall damage, measured from wherever he had last
        // stood before the water.
        self.control.last_trans_any_surf = self.control.trans;
        self.control.highest_jump_mark = self.control.trans;
        // Swimming into something at the surface (a bank): it is counted,
        // for him to climb out once he has pushed a moment.
        let along = Vec3::new(self.control.transv.x, 0.0, self.control.transv.z).length();
        if let Some(s) = self.swim.as_mut() {
            let wall = self.control.status & crate::control::status::TOUCH_WALL != 0;
            s.blocked = if !s.diving && moving && wall && along < meters(1.5) {
                s.blocked + crate::SECONDS_PER_FRAME
            } else {
                0.0
            };
        }
        if self.control.transv.y.abs() < vy.abs() * 0.5 && vy < 0.0 {
            // On the bottom: stop sinking.
            self.control.transv.y = 0.0;
        }
        // Under water and going somewhere he leans into it: his dive laid
        // level as he swims along, half way as he dives ahead.
        let tilt = match (swim.diving && swim.entry.is_none() && !swim.rising, moving, down, up) {
            (true, true, false, false) => std::f32::consts::FRAC_PI_2,
            (true, true, true, false) => std::f32::consts::FRAC_PI_4,
            _ => 0.0,
        };
        self.swim_tilt = crate::math::seek(self.swim_tilt, tilt, 3.0 * crate::SECONDS_PER_FRAME);
        let mut settle = swim.settle;
        let want: Anim = if swim.entry.is_some() {
            anim::SWIM_WALK_TO_DOWN
        } else if swim.diving && swim.rising {
            if swim.lift.is_some() { anim::SWIM_DOWN_TO_UP } else { anim::SWIM_UP }
        } else if swim.diving {
            // Head first: straight down, or laid level as he swims along (`swim_tilt`).
            anim::SWIM_DOWN
        } else if settle && !moving && self.chan.is(anim::SWIM_UP_TO_STANCE) && !self.chan.done(&self.anims) {
            // `swim-up-to-stance`, then the stance.
            anim::SWIM_UP_TO_STANCE
        } else if moving {
            if settle {
                // The stick pushed in the settling: the stroke, blending in over 0.3 s.
                self.ja_push(90);
            }
            settle = false;
            anim::SWIM_WALK
        } else {
            settle = false;
            anim::SWIM_STANCE
        };
        if !self.chan.is(want) {
            self.chan.set(want, NumFunc::Loop { rate: 1.0 }, 0.0);
        }
        // The loops' rates: `swim-down` with his speed (`(loop! (lerp-scale 0.4 1.0 (vector-length transv) 0.0 16384.0))`),
        // `swim-up` with `f30`.
        if self.chan.is(anim::SWIM_DOWN) {
            let rate = lerp_scale(0.4, 1.0, self.control.transv.length(), 0.0, 16384.0);
            self.chan.func = NumFunc::Loop { rate };
        } else if self.chan.is(anim::SWIM_UP) {
            self.chan.func = NumFunc::Loop { rate: swim.rise };
        }
        self.chan.eval(&self.anims.clone());
        // The entry is over when its animation is (`ja-done?`): the dive proper goes on from here.
        let entry_done = swim.entry.is_some() && self.chan.done(&self.anims);
        let lift_done = swim.lift.is_some() && self.chan.is(anim::SWIM_DOWN_TO_UP) && self.chan.done(&self.anims);
        if let Some(s) = self.swim.as_mut() {
            s.settle = settle;
            if entry_done {
                s.entry = None;
            }
            if lift_done {
                s.lift = None;
            }
        }
        true
    }

    /// His feet under the host's water: a fall ends in it, as in Minecraft (the original's landing on
    /// `on-water` skips its hard-landing too).
    pub(crate) fn feet_in_water(&self) -> bool {
        self.swim.is_some() || self.water.is_some_and(|surface| self.control.trans.y < surface)
    }

    /// The water here is too shallow to swim in, or too narrow for the big splash's ripples: it gets the small splash.
    pub(crate) fn small_water(&self) -> bool {
        self.water_fx.column.is_some_and(|column| column < SWIM_HEIGHT * self.swim_size())
            || self.water_fx.basin.is_some_and(|width| width < BASIN_WIDE)
    }

    /// His head's height now (`water-control`'s `top`): the top of his head as posed, at his drawn size.
    /// stand-in: the original's `top` is a joint (`joint-index`) and a `top-y-offset`; the host's crown is what
    /// the water already measures his float by.
    fn head_y(&self) -> f32 {
        self.control.trans.y + self.crown.map_or(2.0 * METER, |crown| crown.y) * self.swim_size()
    }

    /// `spawn-ripples` (`water.gc`): the splash group (`splash-spawn`: the big one or, for a footstep, the small) at
    /// `at` on the water of height `surface`, a twentieth of a second ahead along `velocity` and 40.96 over it, its
    /// drops as many as `intensity` says (`part-water-splash-callback`); and with `mesh`, the ripple mesh
    /// (`skel-generic-ripples`, a manipy at the same place, root scale `(2 intensity, 0.5, 2 intensity)` from 0.6
    /// to 1, playing its `idle` once), no more often than every 0.3 s.
    pub(crate) fn spawn_ripples(&mut self, intensity: f32, at: Vec3, surface: f32, big: bool, velocity: Vec3, mesh: bool) {
        let at = Vec3::new(at.x + velocity.x * 0.05, surface + SURFACE_LIFT, at.z + velocity.z * 0.05);
        // `splash-spawn` waits on `distort-time` 0.1 s; the mesh sets it, every 0.3 s.
        if self.time_elapsed(self.water_fx.distort, seconds(0.1)) {
            if let Some(particles) = self.particles.as_mut() {
                let _ = particles.configure_tracker("part-water-splash-callback", intensity, at);
                particles.spawn(if big { "group-part-water-splash" } else { "group-part-water-splash-small" }, at, Mat3::IDENTITY);
            }
        }
        if mesh && self.time_elapsed(self.water_fx.distort, seconds(0.3)) {
            self.water_fx.distort = self.time;
            // stand-in: the mesh is ten metres of rings, wider than a block or two of water.
            if self.water_fx.basin.is_none_or(|width| width >= BASIN_WIDE) {
                let scale = (2.0 * intensity).clamp(0.6, 1.0);
                self.water_fx.ripples.push(Ripple { at, scale, start: self.time });
            }
        }
    }

    /// The ripple meshes alive: where, the root scale's x and z, and how long they have played (seconds).
    pub fn ripple_meshes(&self) -> Vec<(Vec3, f32, f32)> {
        self.water_fx
            .ripples
            .iter()
            .map(|ripple| (ripple.at, ripple.scale, (self.time - ripple.start) as f32 / TICKS_PER_SECOND as f32))
            .collect()
    }

    /// A splash event (his feet into the water, or out of it) as the particles: `enter-water`'s `spawn-ripples` with the
    /// intensity the height he came in from sets (`lerp-scale 0.3 1.0` over 0.5 to 6 m), the big splash and the
    /// mesh, or the small one where the water is too shallow or narrow for the big; going out only a small splash.
    pub(crate) fn splash_event(&mut self, pos: Vec3, speed: f32) {
        let depth = self.control.height_above_ground();
        let intensity = lerp_scale(0.3, 1.0, depth, 0.5 * METER, 6.0 * METER);
        if speed >= 0.0 {
            let big = !self.small_water();
            self.spawn_ripples(intensity, pos, pos.y, big, self.control.transv, true);
        } else {
            self.spawn_ripples(intensity, pos, pos.y, false, Vec3::ZERO, false);
        }
    }

    /// The wade walk's cycle, advanced a frame (`target-wade-walk`'s `loop!`: the ground covered against what a cycle
    /// covers), for the steps' ripples and for the walk he is drawn with.
    fn wade_walk_frame(&mut self) {
        let Some(depth) = self.wade.filter(|_| self.state == State::Walk) else {
            self.water_fx.wade = crate::anim::Channel::default();
            return;
        };
        let size = self.swim_size();
        let (wade, swim) = (WADE_DEPTH * size, SWIM_HEIGHT * size);
        // The deep walk comes in over the shallow one between a quarter and three quarters of the way to swimming.
        let deep = lerp_scale(0.0, 1.0, depth, lerp(wade, swim, 0.25), lerp(wade, swim, 0.75));
        let cycle = lerp(WADE_SHALLOW_CYCLE, WADE_DEEP_CYCLE, deep);
        let anim = self.wade_anim();
        let channel = &mut self.water_fx.wade;
        if channel.anim != anim {
            // (Going deeper he goes on from where the cycle was.)
            if !channel.is_any(&[anim::WADE_SHALLOW_WALK, anim::WADE_DEEP_WALK]) {
                channel.set(anim, NumFunc::Identity, 0.0);
            }
            channel.anim = anim;
        }
        let rate = self.control.ctrl_xz_vel / (60.0 * (cycle / crate::target::RUN_CYCLE_LENGTH));
        let anims = self.anims.clone();
        self.water_fx.wade.eval_with(&anims, NumFunc::Loop { rate });
    }

    /// Which wade walk he is drawn with: the deep one past 1.25 m of water over his feet.
    pub fn wade_anim(&self) -> Anim {
        let deep = self.wade.is_some_and(|depth| depth / self.size.max(0.1) > meters(1.25));
        if deep { anim::WADE_DEEP_WALK } else { anim::WADE_SHALLOW_WALK }
    }

    /// The animation and frame his pose is read from: while he wades the wade walk's own cycle, else the base channel's.
    pub fn pose_anim(&self) -> (Anim, f32) {
        if self.state == State::Walk && self.wade.is_some() && self.water_fx.wade.is_any(&[anim::WADE_SHALLOW_WALK, anim::WADE_DEEP_WALK]) {
            (self.water_fx.wade.anim, self.water_fx.wade.frame)
        } else {
            (self.chan.anim, self.chan.frame)
        }
    }

    /// The water's effects on him each frame (`water-control-method-10`, Jak 3 `water.gc`): the splash as his
    /// feet go under (`enter-water`), and with his head over the surface the flat wake (part 762, denser the
    /// faster he goes), the ripple when he is slow (758, its size range `ripple-size`), the wake's light trail
    /// (`water-control-method-17`: a tracker adding where he is each frame) and, as his head comes up, another
    /// splash; and from the joints under water the drops rising (756). Wading, each foot's step in shallow water
    /// (`target-wade-walk`) and the spin (`target-wade-stance`) splash too. All at the host's surface, the fluid's
    /// own height.
    pub(crate) fn water_particles(&mut self) {
        let now = self.time;
        self.water_fx.ripples.retain(|ripple| now - ripple.start < RIPPLE_LIFE);
        self.wade_walk_frame();
        let Some(surface) = self.water else {
            self.water_fx.touching = false;
            self.water_fx.head = None;
            self.end_wake();
            self.water_fx.spinning = false;
            self.water_fx.punching = false;
            return;
        };
        let pos = self.control.trans;
        let skimming = self.state.is_board() || self.state.is_flut();
        let touching = pos.y < surface;
        let head = self.head_y();
        let before = self.water_fx.head.replace(head);
        // `enter-water`: its splash and its sound; the Flut Flut's own are `flut_swim`'s.
        if touching && !self.water_fx.touching {
            self.water_fx.entered = self.time;
            if !skimming {
                self.events.push(Event::Splash {
                    pos: Vec3::new(pos.x, surface, pos.z),
                    speed: (-self.control.transv.y).max(0.0),
                });
            }
        }
        self.water_fx.touching = touching;
        let swimming_under = self.swim.is_some_and(|swim| swim.diving);
        // Head over the water (`top 0 y >= height`), feet under it (`break-surface`); the board and the Flut Flut skim it.
        let breaking = if skimming {
            (pos.y - surface).abs() < 0.6 * METER
        } else {
            touching && head >= surface && !swimming_under
        };
        // stand-in: the wake's flat ripples and its trail only where the water is wide enough for them (Adrii 06/11: in
        // one block of water they came out over the bank); the original's water is a lake.
        let roomy = self.water_fx.basin.is_none_or(|width| width >= BASIN_WIDE);
        let wake = breaking && roomy;
        let emerging = !skimming && touching && head >= surface && before.is_some_and(|before| before < surface);
        let speed = Vec3::new(self.control.transv.x, 0.0, self.control.transv.z).length();
        let heading = crate::math::z_axis(self.control.quat);
        let flat = Vec3::new(heading.x, 0.0, heading.z).try_normalize().unwrap_or(Vec3::Z);
        let level = Mat3::from_cols(Vec3::Y.cross(flat), Vec3::Y, flat);
        if emerging {
            // `(< (-> this top 1 y) (-> this height))`: `spawn-ripples 0.2` as the head breaks the surface.
            let big = !self.small_water();
            self.spawn_ripples(0.2, pos, surface, big, self.control.transv, true);
        }
        // `water-control-method-17`: the wake's tracker, spawned once he has been in the water a tenth of a second, adds
        // his place each frame after (`should-track?`: in the water) and ends as his head goes under or he is out.
        self.wake_trail(wake, Vec3::new(pos.x, surface + SURFACE_LIFT, pos.z));
        // Wading: each foot's step (`target-wade-walk`, the shallow walk's frames 15 to 18 on the right toes, 46 to 49 on
        // the left, no oftener than every 0.2 s while the water is shallow) and the spin (`target-wade-stance`).
        self.wade_splashes(surface);
        // The drops: a joint from the third on (`rand-vu-int-range 3 ...`), if it is under the surface.
        let (drop, joint) = if touching && !self.state.is_flut() && self.joints.len() > 3 {
            let seed = &mut self.water_fx.seed;
            *seed ^= *seed << 13;
            *seed ^= *seed >> 17;
            *seed ^= *seed << 5;
            let index = 3 + (*seed as usize) % (self.joints.len() - 3);
            let at = pos + self.control.quat * (self.joints[index] * self.swim_size());
            (at.y < surface, at)
        } else {
            (false, pos)
        };
        let since = (self.time - self.water_fx.entered) as f32;
        let mut carry = self.water_fx.carry;
        let Some(particles) = self.particles.as_mut() else { return };
        if wake {
            let at = Vec3::new(pos.x, surface + SURFACE_LIFT, pos.z);
            // `:num` of part 762 set from his speed (its own 0.05 is replaced, its random 0.4 stays).
            particles.launch_flat(762, at, level, (speed * 0.000_004_150_391 + 0.2) / 0.25);
            if speed < METER {
                // `(set! (-> *part-id-table* 758 init-specs 2 random-rangef) (-> this ripple-size))`.
                particles.set_part_range(758, "scale-x", Some(RIPPLE_SIZE));
                particles.launch_flat(758, at, level, 1.0);
            }
        }
        if drop {
            particles.set_part_value(756, "userdata", Some(surface));
            let num = lerp_scale(5.0, 0.4, since, 0.0, 600.0) + 0.000_122_070_31 * speed;
            particles.launch_var(756, joint, Mat3::IDENTITY, Overrides { num: Some(num), ..Default::default() }, &mut carry);
        }
        self.water_fx.carry = carry;
    }

    /// `water-control-method-17` and `light-trail-tracker-water`'s `:trans`: the wake's trail follows `at` while he
    /// `breaking` the surface; it ends (the trail wears away from its tail) when he does not.
    fn wake_trail(&mut self, breaking: bool, at: Vec3) {
        match self.water_fx.wake {
            Some(id) if breaking => {
                if let Some(particles) = self.particles.as_mut() {
                    particles.trail_crumb(id, at);
                }
            }
            Some(_) => self.end_wake(),
            None if breaking && self.time_elapsed(self.water_fx.entered, seconds(0.1)) => {
                // (The tracker starts adding next frame.)
                self.water_fx.next_wake += 1;
                self.water_fx.wake = Some(self.water_fx.next_wake);
            }
            None => {}
        }
    }

    fn end_wake(&mut self) {
        if let Some(id) = self.water_fx.wake.take() {
            if let Some(particles) = self.particles.as_mut() {
                particles.trail_end(id);
            }
        }
    }

    /// The steps and the spin of a wader (`target-wade-walk`, `target-wade-stance`).
    fn wade_splashes(&mut self, surface: f32) {
        let size = self.swim_size();
        let shallow = surface - self.control.trans.y < WADE_STEP_DEPTH * size;
        if self.state == State::Walk && self.wade.is_some() && shallow && self.time_elapsed(self.water_fx.stepped, seconds(0.2)) {
            let frame = self.water_fx.wade.aframe_num(&self.anims) as i32;
            let toe = match frame {
                15..=18 => self.toe_joints.map(|(right, _)| right),
                46..=49 => self.toe_joints.map(|(_, left)| left),
                _ => None,
            };
            if let Some(joint) = toe.and_then(|index| self.joints.get(index)) {
                let at = self.control.trans + self.control.quat * (*joint * size);
                // `spawn-ripples 0.2 toes 0 (transv * 2.5) #f`: the small splash ahead of the foot.
                self.spawn_ripples(0.2, at, surface, false, self.control.transv * 2.5, false);
                self.water_fx.stepped = self.time;
            }
        }
        // The spin out of the wade (`target-wade-stance`: `spawn-ripples 1.4 trans 0 transv #f` as he goes into
        // `target-attack`).
        let spinning = self.state == State::Attack;
        if spinning && !self.water_fx.spinning && self.wade.is_some() {
            // (The wade states' `(sound-play "swim-stroke")` just before.)
            self.events.push(Event::Sound { name: "SWIM_STROKE", volume: 1.0, pitch: 0.0 });
            self.spawn_ripples(1.4, self.control.trans, surface, false, self.control.transv, false);
        }
        self.water_fx.spinning = spinning;
        // The punch (`target-running-attack`, `target-darkjak-running-attack`): its code opens with "swim-stroke" if he
        // touches the water.
        let punching = self.state == State::RunningAttack;
        if punching && !self.water_fx.punching && self.water_fx.touching {
            self.events.push(Event::Sound { name: "SWIM_STROKE", volume: 1.0, pitch: 0.0 });
        }
        self.water_fx.punching = punching;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn surface_jump_clears_water_at_minecraft_scale() {
        let block=6400.0;
        let mut jak=Jak::new(Vec3::ZERO,0.0);
        jak.size=0.64;
        // The model is drawn 1.94 blocks tall for 2.51 m of his own.
        jak.drawn_size=Some(1.94/2.510636*block/crate::METER);
        jak.control.set_body_dimensions(0.3*block,1.8*block);
        jak.water=Some(2.0*block);
        jak.control.trans.y=2.0*block-jak.float_depth();
        jak.swim=Some(Swim{diving:false,since:0,blocked:0.0,rise:1.0,entry:None,down_held:false,state_since:0,min_dive:60,rising:false,lift:None,settle:false});
        let mut world=crate::collide::EmptyWorld;
        for _ in 0..60 {jak.step(&crate::PadInput::default(),&mut world,None);}
        let mut peak=f32::NEG_INFINITY;
        for frame in 0..90 {
            jak.step(&crate::PadInput{held:button::X,pressed:if frame==0{button::X}else{0},..Default::default()},&mut world,None);
            peak=peak.max(jak.trans().y-2.0*block);
        }
        // `SWIM_JUMP_HEIGHT` drawn: 1.1 m of 1.2 blocks to the metre.
        assert!(peak>0.75*block,"feet only rise {} blocks over the water",peak/block);
        assert!(peak<0.95*block,"water jump must retain the authored height: {}",peak/block);
    }

    /// A diver: floating at the surface of two blocks of water, drawn at the mod's size.
    fn floating() -> (Jak, f32) {
        let block = 6400.0;
        let mut jak = Jak::new(Vec3::ZERO, 0.0);
        jak.size = 0.64;
        jak.drawn_size = Some(1.94 / 2.510636 * block / crate::METER);
        jak.control.set_body_dimensions(0.3 * block, 1.8 * block);
        jak.water = Some(6.0 * block);
        jak.control.trans.y = 6.0 * block - jak.float_depth();
        jak.swim = Some(Swim { diving: false, since: 0, blocked: 0.0, rise: 1.0, entry: None, down_held: false, state_since: 0, min_dive: 60, rising: false, lift: None, settle: false });
        (jak, block)
    }

    /// Sinking and rising are the original's: down by the dive's gravity (4 m/s a second) to 4 m/s
    /// (`target-swim-down`), 4 m/s up at once and 6 for a moment after an X press (`target-swim-up`).
    #[test]
    fn dive_and_rise_are_the_originals() {
        let mut world = crate::collide::EmptyWorld;
        let (mut jak, _) = floating();
        jak.step(&crate::PadInput { held: button::SQUARE, pressed: button::SQUARE, ..Default::default() }, &mut world, None);
        assert!(jak.swim.unwrap().diving);
        // `target-swim-down` plays `jakb-swim-walk-to-down` through first, its own motion taking him.
        assert!(jak.swim.unwrap().entry.is_some() && jak.chan.is(anim::SWIM_WALK_TO_DOWN));
        for _ in 0..100 {
            jak.step(&crate::PadInput { held: button::SQUARE, ..Default::default() }, &mut world, None);
            if jak.swim.unwrap().entry.is_none() {
                break;
            }
        }
        assert!(jak.swim.unwrap().entry.is_none(), "the entry never ended");
        jak.step(&crate::PadInput { held: button::SQUARE, ..Default::default() }, &mut world, None);
        assert!(jak.chan.is(anim::SWIM_DOWN), "the dive goes on in {:?}", jak.chan.anim);
        let start = -jak.control.transv.y;
        for _ in 0..30 {
            jak.step(&crate::PadInput { held: button::SQUARE, ..Default::default() }, &mut world, None);
        }
        // Half a second more of 4 m/s a second: two more meters a second.
        let gained = (-jak.control.transv.y - start) / METER;
        assert!((gained - 2.0).abs() < 0.1, "gained {gained} m/s in half a second");
        for _ in 0..90 {
            jak.step(&crate::PadInput { held: button::SQUARE, ..Default::default() }, &mut world, None);
        }
        let sank = -jak.control.transv.y;
        assert!((sank - meters(4.0)).abs() < meters(0.05), "sinks {} m/s", sank / METER);
        // Letting go: once the dive has run its minimum `target-swim-up` plays `swim-down-to-up` through, then its loop rises
        // at 4 m/s.
        for _ in 0..100 {
            jak.step(&crate::PadInput::default(), &mut world, None);
            if jak.swim.unwrap().rising {
                break;
            }
        }
        assert!(jak.swim.unwrap().rising && jak.swim.unwrap().lift.is_some() && jak.chan.is(anim::SWIM_DOWN_TO_UP));
        for _ in 0..100 {
            jak.step(&crate::PadInput::default(), &mut world, None);
            if jak.swim.unwrap().lift.is_none() {
                break;
            }
        }
        assert!(jak.swim.unwrap().lift.is_none(), "the lift never ended");
        jak.step(&crate::PadInput::default(), &mut world, None);
        assert!(jak.chan.is(anim::SWIM_UP), "the loop goes on in {:?}", jak.chan.anim);
        assert!((jak.control.transv.y - meters(4.0)).abs() < meters(0.1), "rises {} m/s", jak.control.transv.y / METER);
        // X pressed: 6 m/s, back to 4 within a second.
        let before = jak.trans().y;
        jak.step(&crate::PadInput { held: button::X, pressed: button::X, ..Default::default() }, &mut world, None);
        let first = (jak.trans().y - before) * 60.0;
        assert!(first > meters(5.7) && first < meters(6.1), "first rise {} m/s", first / METER);
        for _ in 0..70 {
            jak.step(&crate::PadInput { held: button::X, ..Default::default() }, &mut world, None);
            if !jak.swim.is_some_and(|swim| swim.diving) {
                break;
            }
        }
        let rise = jak.control.transv.y;
        assert!(rise <= meters(4.1), "settles at {} m/s", rise / METER);
    }

    /// A floor under the water: one big quad at `y`.
    struct Floor(Vec<crate::collide::Tri>);
    impl crate::collide::CollideWorld for Floor {
        fn fill(&mut self, _: Vec3, _: Vec3, out: &mut Vec<crate::collide::Tri>) {
            out.extend_from_slice(&self.0);
        }
    }
    fn floor_at(y: f32) -> Floor {
        let b = 400_000.0;
        let pat = crate::collide::Pat { mode: crate::collide::PatMode::Ground, ..Default::default() };
        let q = [Vec3::new(-b, y, -b), Vec3::new(-b, y, b), Vec3::new(b, y, b), Vec3::new(b, y, -b)];
        Floor(vec![crate::collide::Tri { v: [q[0], q[1], q[2]], pat }, crate::collide::Tri { v: [q[0], q[2], q[3]], pat }])
    }

    /// `target-flop`'s `swim` event: a flop into water with more than 1.5 m under him goes on into `target-swim-down` with
    /// "swim-flop" (here Jak 1's, Jak 3 having none), at once at the dive's top speed and on `swim-down`'s artist frame 124.
    #[test]
    fn a_flop_into_deep_water_goes_on_as_a_dive() {
        let (mut jak, block) = floating();
        jak.swim = None;
        let mut world = floor_at(0.0);
        jak.control.trans.y = 12.0 * block;
        jak.go(State::Flop { forward: false });
        let mut sounds = 0;
        let mut dived = false;
        for _ in 0..900 {
            jak.step(&crate::PadInput::default(), &mut world, None);
            sounds += jak.take_events().iter().filter(|e| matches!(e, Event::Sound { name: "jak1:swim_flop", .. })).count();
            if let Some(swim) = jak.swim {
                dived = swim.diving && swim.entry.is_none() && !swim.rising;
                break;
            }
        }
        assert!(dived, "the flop did not go on as a dive: {:?}", jak.swim);
        assert_eq!(sounds, 1, "one swim-flop");
        assert!(jak.chan.is(anim::SWIM_DOWN));
        assert!(jak.control.transv.y <= -meters(3.9), "dives at {} m/s", jak.control.transv.y / METER);
        // It holds the dive 0.4 s without the button, then rises.
        for _ in 0..12 {
            jak.step(&crate::PadInput::default(), &mut world, None);
        }
        assert!(jak.swim.unwrap().diving && !jak.swim.unwrap().rising, "let go of the dive before its minimum");
        for _ in 0..30 {
            jak.step(&crate::PadInput::default(), &mut world, None);
        }
        assert!(jak.swim.unwrap().rising, "never rose");
    }

    /// Water under his feet shallower than `min-dive-depth` is not dived in.
    #[test]
    fn diving_needs_the_ground_two_meters_under_his_feet() {
        for (blocks, dives) in [(2.0, false), (5.0, true)] {
            let mut world = floor_at(0.0);
            let (mut jak, block) = floating();
            jak.water = Some(blocks * block);
            jak.control.trans.y = blocks * block - jak.float_depth();
            for _ in 0..90 {
                jak.step(&crate::PadInput { held: button::SQUARE, ..Default::default() }, &mut world, None);
            }
            assert_eq!(jak.swim.is_some_and(|swim| swim.diving), dives, "{blocks} blocks of water");
        }
    }

    /// Water takes his fall (Adrii 06/11: after a dive, solid ground gave him fall damage): the marks the fall is
    /// measured from follow him while he swims, and a landing with his feet in water takes none.
    #[test]
    fn water_resets_the_fall() {
        let mut world = crate::collide::EmptyWorld;
        let (mut jak, block) = floating();
        jak.control.last_trans_any_surf = Vec3::new(0.0, 60.0 * block, 0.0);
        jak.control.highest_jump_mark = Vec3::new(0.0, 60.0 * block, 0.0);
        for _ in 0..10 {
            jak.step(&crate::PadInput::default(), &mut world, None);
        }
        assert!((jak.control.last_trans_any_surf.y - jak.trans().y).abs() < 1.0, "marks {}", jak.control.last_trans_any_surf.y);
        assert!((jak.control.highest_jump_mark.y - jak.trans().y).abs() < 1.0);
        jak.swim = None;
        jak.water = Some(jak.trans().y + 100.0);
        assert!(jak.feet_in_water());
        jak.water = Some(jak.trans().y - 100.0);
        assert!(!jak.feet_in_water());
    }

    /// His feet go under the water: its splash once, at the surface the host gave (the fluid's height), and not
    /// again while they stay under it.
    #[test]
    fn entering_the_water_splashes_once_at_its_surface() {
        let mut world = crate::collide::EmptyWorld;
        let block = 6400.0;
        let mut jak = Jak::new(Vec3::new(0.0, 3.0 * block, 0.0), 0.0);
        jak.control.set_body_dimensions(0.3 * block, 1.8 * block);
        let surface = 3.0 * block + 0.889 * block;
        jak.water = Some(surface);
        jak.step(&crate::PadInput::default(), &mut world, None);
        let splashes: Vec<_> = jak.take_events().into_iter().filter_map(|e| if let Event::Splash { pos, .. } = e { Some(pos) } else { None }).collect();
        assert_eq!(splashes.len(), 1, "{splashes:?}");
        assert!((splashes[0].y - surface).abs() < 1.0);
        for _ in 0..5 {
            jak.step(&crate::PadInput::default(), &mut world, None);
        }
        let again = jak.take_events().into_iter().filter(|e| matches!(e, Event::Splash { .. })).count();
        assert_eq!(again, 0);
    }

    /// The wake and ripples of a swimmer whose head is over the water come up at its surface (40.96 over it, the
    /// original's), flat on it; with his head under it nothing is launched (Adrii 06/11: no water effects while
    /// swimming).
    #[test]
    fn a_swimmer_leaves_a_wake_at_the_surface() {
        let Some(home) = std::env::var_os("USERPROFILE") else { return };
        let root = std::path::PathBuf::from(home).join(r"Documents\Jak OpenGOAL\active\jak3\data\goal_src\jak3");
        let mut library = crate::particles::Library::default();
        for file in ["engine/common-obs/water-part.gc", "engine/common-obs/water.gc"] {
            let Ok(text) = std::fs::read_to_string(root.join(file)) else { return };
            library.read(&text);
        }
        let library = std::sync::Arc::new(library);
        let mut world = crate::collide::EmptyWorld;
        let (mut jak, block) = floating();
        jak.crown = Some(Vec3::new(0.0, meters(2.0), 0.0));
        jak.control.trans.y = 6.0 * block - jak.float_depth();
        jak.set_particles(library.clone());
        let surface = 6.0 * block;
        let mut near = 0;
        for _ in 0..40 {
            jak.step(&crate::PadInput::default(), &mut world, None);
            near += jak.particle_sprites().iter().filter(|s| (s.pos.y - (surface + SURFACE_LIFT)).abs() < 2.0 && s.flat.is_some()).count();
        }
        assert!(near > 0, "no wake sprites at the surface");
        // Diving, his head is under it: the wake stops.
        let (mut jak, _) = floating();
        jak.crown = Some(Vec3::new(0.0, meters(2.0), 0.0));
        jak.set_particles(library);
        for _ in 0..5 {
            jak.step(&crate::PadInput { held: button::SQUARE, pressed: button::SQUARE, ..Default::default() }, &mut world, None);
        }
        for _ in 0..120 {
            jak.step(&crate::PadInput { held: button::SQUARE, ..Default::default() }, &mut world, None);
        }
        assert!(jak.trans().y + jak.crown.unwrap().y * jak.swim_size() < surface, "his head is not under the surface");
        for _ in 0..90 {
            jak.step(&crate::PadInput { held: button::SQUARE, ..Default::default() }, &mut world, None);
        }
        assert!(
            jak.particle_sprites().iter().all(|s| s.flat.is_none() || (s.pos.y - (surface + SURFACE_LIFT)).abs() > 2.0 || s.texture.is_empty()),
            "a wake while he is under"
        );
    }

    /// Banks a block apart (a one-block pool): the water is narrow, its splash the small one and no flat ripples
    /// spread over the bank; in open water they come up.
    #[test]
    fn a_pool_of_one_block_gets_no_flat_ripples() {
        struct Banks(Vec<crate::collide::Tri>);
        impl CollideWorld for Banks {
            fn fill(&mut self, _min: Vec3, _max: Vec3, out: &mut Vec<crate::collide::Tri>) {
                out.extend_from_slice(&self.0);
            }
        }
        let block = 6400.0;
        let pat = crate::collide::Pat { mode: crate::collide::PatMode::Wall, ..Default::default() };
        let mut tris = Vec::new();
        // Their faces look into the pool.
        for (a, b) in [((-0.5, 0.5), (-0.5, -0.5)), ((0.5, -0.5), (0.5, 0.5)), ((-0.5, -0.5), (0.5, -0.5)), ((0.5, 0.5), (-0.5, 0.5))] {
            let v = |x: f32, y: f32, z: f32| Vec3::new(x * block, y * block, z * block);
            tris.push(crate::collide::Tri { v: [v(a.0, 0.0, a.1), v(b.0, 0.0, b.1), v(b.0, 12.0, b.1)], pat });
            tris.push(crate::collide::Tri { v: [v(a.0, 0.0, a.1), v(b.0, 12.0, b.1), v(a.0, 12.0, a.1)], pat });
        }
        let mut pool = Banks(tris);
        let (mut jak, _) = floating();
        jak.crown = Some(Vec3::new(0.0, meters(2.0), 0.0));
        jak.set_particles(std::sync::Arc::new(crate::particles::Library::default()));
        for _ in 0..30 {
            jak.step(&crate::PadInput::default(), &mut pool, None);
        }
        let width = jak.water_fx.basin.expect("banks a block apart");
        assert!(width < 1.2 * block, "a pool {} blocks wide", width / block);
        assert!(jak.small_water());
        // Open water: no banks within reach.
        let mut open = crate::collide::EmptyWorld;
        let (mut jak, _) = floating();
        for _ in 0..30 {
            jak.step(&crate::PadInput::default(), &mut open, None);
        }
        assert!(jak.water_fx.basin.is_none());
    }

    /// Drawn two blocks tall, he floats as deep as in his game: his feet
    /// 1.8 of his meters under it (1.4 blocks), not 0.74 as at `size`.
    #[test]
    fn floats_by_his_drawn_size() {
        let block=6400.0;
        let mut jak=Jak::new(Vec3::ZERO,0.0);
        jak.size=0.64;
        jak.drawn_size=Some(1.94/2.510636*block/crate::METER);
        let under=jak.float_depth()/block;
        assert!((under-1.39).abs()<0.02,"feet {under} blocks under the water");
    }

    /// Floating, the top of his head stays over the water however the pose
    /// holds it; a pose that holds it high keeps the game's swim height.
    #[test]
    fn head_stays_out_of_the_water() {
        let mut jak = Jak::new(Vec3::ZERO, 0.0);
        jak.size = 1.3;
        jak.crown = Some(Vec3::new(0.0, meters(1.5), 0.0));
        let feet_under = jak.float_depth();
        let crown_over = jak.crown.unwrap().y * jak.size - feet_under;
        assert!(crown_over >= CROWN_OUT * jak.size - 1.0, "head {crown_over} over the water");
        jak.crown = Some(Vec3::new(0.0, meters(2.5), 0.0));
        assert_eq!(jak.float_depth(), SWIM_HEIGHT * jak.size);
    }

    /// A Minecraft block of water wades him at about 7.3 m/s of his 10.
    #[test]
    fn a_meter_of_water_slows_his_walk() {
        let mut jak = Jak::new(Vec3::ZERO, 0.0);
        jak.wade = Some(meters(1.0));
        assert!((jak.wade_scale() - 0.7333).abs() < 1e-3);
        jak.wade = None;
        assert_eq!(jak.wade_scale(), 1.0);
    }

    /// However high he floats for his head, he wades out only well over
    /// it: floating never takes him out of the water.
    #[test]
    fn floating_never_leaves_the_water() {
        let mut jak = Jak::new(Vec3::ZERO, 0.0);
        jak.size = 1.3;
        jak.crown = Some(Vec3::new(0.0, meters(1.2), 0.0));
        let leave = (LEAVE_DEPTH * jak.size).min(jak.float_depth() - LEAVE_UNDER_FLOAT * jak.size);
        assert!(jak.float_depth() > leave);
    }
}
