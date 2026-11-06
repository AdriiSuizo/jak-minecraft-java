//! Jak 3's four guns and their three modes each, as `target-gun.gc` and the
//! `gun-*-shot.gc` files fire them: the Scatter Gun and Plasmite RPG (red,
//! including the charging Wave Concussor), the Blaster, Beam Reflexor and Gyro Burster (yellow),
//! the Vulcan Fury, Arc Wielder and Needle Lazer (blue), and the Peace
//! Maker, Mass Inverter and Super Nova (dark).
//!
//! Every shot lives in Jak's own units and steps at 60 Hz with the rest of
//! him. Damage leaves as [`Event::Impact`], one per creature struck (and
//! one with no creature where a blast meets the world), so the host applies
//! it the same way it applies the Blaster's.
use glam::{Mat3, Quat, Vec3};

use crate::collide::{CollideCache, CollideWorld};
use crate::math::*;
use crate::pad::button;
use crate::projectile::{ActorWorld, Projectile, ProjectileHit, reborrow};
use crate::{Event, Jak};
mod wave;
pub mod presentation;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
pub enum GunColor {
    Red,
    #[default]
    Yellow,
    Blue,
    Dark,
}

/// The twelve gun modes, in pickup order (`gun-red-1` is 26).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
pub enum GunKind {
    Red1,
    Red2,
    Red3,
    #[default]
    Yellow1,
    Yellow2,
    Yellow3,
    Blue1,
    Blue2,
    Blue3,
    Dark1,
    Dark2,
    Dark3,
}

/// How a mode reads R1 (`gun-control`): fire on each press, fire while
/// held once per delay, or spin the barrels up while held.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FireControl {
    Press,
    Hold,
    Spin,
}

impl GunKind {
    pub const ALL: [GunKind; 12] = [
        GunKind::Red1,
        GunKind::Red2,
        GunKind::Red3,
        GunKind::Yellow1,
        GunKind::Yellow2,
        GunKind::Yellow3,
        GunKind::Blue1,
        GunKind::Blue2,
        GunKind::Blue3,
        GunKind::Dark1,
        GunKind::Dark2,
        GunKind::Dark3,
    ];

    pub fn color(self) -> GunColor {
        match self as u8 / 3 {
            0 => GunColor::Red,
            1 => GunColor::Yellow,
            2 => GunColor::Blue,
            _ => GunColor::Dark,
        }
    }

    /// 1, 2 or 3.
    pub fn mode(self) -> u8 {
        self as u8 % 3 + 1
    }

    pub fn of(color: GunColor, mode: u8) -> Self {
        Self::ALL[color as usize * 3 + (mode.clamp(1, 3) - 1) as usize]
    }

    pub fn next_mode(self) -> Self {
        Self::of(self.color(), self.mode() % 3 + 1)
    }

    pub fn name(self) -> &'static str {
        match self {
            GunKind::Red1 => "Scatter Gun",
            GunKind::Red2 => "Wave Concussor",
            GunKind::Red3 => "Plasmite RPG",
            GunKind::Yellow1 => "Blaster",
            GunKind::Yellow2 => "Beam Reflexor",
            GunKind::Yellow3 => "Gyro Burster",
            GunKind::Blue1 => "Vulcan Fury",
            GunKind::Blue2 => "Arc Wielder",
            GunKind::Blue3 => "Needle Lazer",
            GunKind::Dark1 => "Peace Maker",
            GunKind::Dark2 => "Mass Inverter",
            GunKind::Dark3 => "Super Nova",
        }
    }

    pub fn control(self) -> FireControl {
        match self {
            GunKind::Blue1 | GunKind::Blue2 | GunKind::Blue3 => FireControl::Spin,
            GunKind::Red2 | GunKind::Dark1 | GunKind::Dark2 | GunKind::Dark3 => FireControl::Hold,
            _ => FireControl::Press,
        }
    }

    /// `fire-delay`, in ticks; the blue gun's falls as its barrels spin up.
    pub fn fire_delay(self, spinv: f32) -> i64 {
        match self {
            GunKind::Yellow1 | GunKind::Yellow2 => 96,
            GunKind::Yellow3 | GunKind::Blue2 => 0,
            GunKind::Red1 | GunKind::Red3 => 330,
            GunKind::Red2 => 0,
            GunKind::Blue1 | GunKind::Blue3 => {
                lerp_scale(120.0, 30.0, spinv, 0.0, crate::gun::SPIN_MAX) as i64
            }
            GunKind::Dark1 | GunKind::Dark2 | GunKind::Dark3 => 255,
        }
    }

    /// `ammo-required`.
    pub fn ammo_required(self) -> f32 {
        match self {
            GunKind::Red3 | GunKind::Yellow3 | GunKind::Dark3 => 10.0,
            _ => 1.0,
        }
    }

    /// `track-find-range`: how far auto-aim reaches for this mode.
    pub fn track_range(self) -> f32 {
        match self.color() {
            GunColor::Red => meters(20.0),
            GunColor::Yellow => crate::gun::TRACK_FIND_RANGE,
            GunColor::Blue => meters(80.0),
            GunColor::Dark => meters(400.0),
        }
    }

    /// The color its beams and glows are drawn in.
    pub fn rgba(self) -> [f32; 4] {
        match self.color() {
            GunColor::Red => [1.0, 0.35, 0.12, 1.0],
            GunColor::Yellow => [1.0, 0.9, 0.35, 1.0],
            GunColor::Blue => [0.35, 0.75, 1.0, 1.0],
            GunColor::Dark => [0.8, 0.3, 1.0, 1.0],
        }
    }
}

/// A beam for the host to draw: a line from `tail` to `head`, `width` in
/// meters.
#[derive(Clone, Copy, Debug, serde::Serialize, serde::Deserialize)]
pub struct Beam {
    pub head: Vec3,
    pub tail: Vec3,
    pub rgba: [f32; 4],
    pub width: f32,
}

const DEG: f32 = std::f32::consts::PI / 180.0;
const GRAVITY_BOUNCE: f32 = 184320.0;
const TAIL_MAX: f32 = 65536.0;
const LONG_AGO: i64 = i64::MIN / 4;

pub fn plasmite_velocity_to(offset: Vec3) -> Option<Vec3> {
    if !offset.is_finite() {
        return None;
    }
    let flat = Vec3::new(offset.x, 0.0, offset.z);
    let x = f64::from(flat.length());
    if x < 1.0 {
        return None;
    }
    let y = f64::from(offset.y);
    let speed = 266240.0f64;
    let g = f64::from(GRAVITY_BOUNCE);
    let v2 = speed * speed;
    let d = v2 * v2 - g * (g * x * x + 2.0 * y * v2);
    if d < 0.0 {
        return None;
    }
    let tangent = (g * x * x + 2.0 * y * v2) / (x * (v2 + d.sqrt()));
    let horizontal = speed / (1.0 + tangent * tangent).sqrt();
    Some(flat.normalize() * horizontal as f32 + Vec3::Y * (horizontal * tangent) as f32)
}

/// A small xorshift, so a replay of the same inputs fires the same spread.
#[derive(Clone, Debug)]
struct Rng(u32);

impl Rng {
    fn unit(&mut self) -> f32 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 17;
        x ^= x << 5;
        self.0 = x;
        (x >> 8) as f32 / 16_777_216.0
    }

    fn range(&mut self, a: f32, b: f32) -> f32 {
        a + (b - a) * self.unit()
    }

    /// `(mod (rand-uint31-gen) n)`.
    fn below(&mut self, n: u32) -> u32 {
        ((self.unit() * n as f32) as u32).min(n - 1)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum SaucerPhase {
    Moving,
    Navigating,
    Spinning,
    Falling,
    Sitting,
    Husk,
}

#[derive(Clone, Debug)]
enum ShotState {
    /// Beam Reflexor: bounces off walls, and off creatures `deflects` times.
    Reflect {
        deflects: u8,
        hit_enemy: bool,
        ignore: Vec<(u64, i64)>,
        last_wall: i64,
        /// Its light trail's id in [`Weapons::trails`].
        trail: u32,
    },
    /// Plasmite RPG's grenade, and its light trail's id.
    Grenade { fuse: Option<i64>, sitting: bool, last_bounce: i64, trail: u32 },
    /// Needle Lazer.
    Needle {
        target: Option<u64>,
        last_pos: Option<Vec3>,
        travel: f32,
        travelled: f32,
        mode: u8,
        mode_time: i64,
    },
    /// Peace Maker: charging on the muzzle, then a corkscrewing core.
    Peace {
        charging: bool,
        target: Option<u64>,
        core: Vec3,
        dir: Vec3,
        spin: f32,
        launch: i64,
    },
    /// Super Nova.
    Nuke {
        phase: u8,
        phase_time: i64,
        base_y: f32,
        dir: Vec3,
        embedded: Option<i64>,
    },
    /// Gyro Burster's saucer.
    Saucer {
        phase: SaucerPhase,
        phase_time: i64,
        first_shot: Option<i64>,
        last_volley: i64,
        last_probe: i64,
        spin: f32,
        drained: f32,
    },
}

#[derive(Clone, Debug)]
struct Shot {
    kind: GunKind,
    trans: Vec3,
    transv: Vec3,
    tail: Vec3,
    spawn: i64,
    timeout: i64,
    damage: f32,
    radius: f32,
    state: ShotState,
}

#[derive(Clone, Debug)]
enum Effect {
    /// Lines drawn for a moment: the Scatter Gun's probes, lightning.
    Rays {
        rays: Vec<(Vec3, Vec3)>,
        start: i64,
        duration: i64,
        kind: GunKind,
    },
    /// A ball of light swelling and fading: explosions.
    Blast {
        center: Vec3,
        radius: f32,
        start: i64,
        duration: i64,
        kind: GunKind,
    },
    /// The Peace Maker's tendrils from the first victim to the rest.
    Tendrils {
        origin: Vec3,
        queue: Vec<(u64, Vec3)>,
        arcs: Vec<(u64, Vec3, i64, bool)>,
        next: i64,
        start: i64,
    },
    /// The Mass Inverter's spreading wave.
    Gravity {
        center: Vec3,
        start: i64,
        caught: Vec<u64>,
    },
    /// The Super Nova's blast.
    Nuke {
        center: Vec3,
        start: i64,
        killed: bool,
        small: bool,
    },
    /// A model a shot leaves where it went off: the Scatter Gun's cone at
    /// the muzzle (`skel-gun-red-cone`, along the shot), the Plasmite RPG's
    /// sphere where its grenade burst (`red-3-sphere`).
    Left {
        which: LeftModel,
        pos: Vec3,
        dir: Vec3,
        start: i64,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum LeftModel {
    Cone,
    RedSphere,
}

/// `rope-constraint-method-9` from knot `from`: each knot after it bent
/// towards the one before's direction, faster the further off (30 to
/// 12000 degrees a second by the square of the angle's share of a half
/// turn, 720 at most), then each put at its length along the way to it.
fn rope_constraint(knots: &mut [Knot; 12], from: usize) {
    for i in from..knots.len() - 1 {
        let (a, b) = (knots[i].pt, knots[i + 1].pt);
        let mut d = b - a;
        let len = d.length();
        d = d.normalize_or(Vec3::Z);
        let dir = knots[i].dir.normalize_or(Vec3::Z);
        knots[i].dir = dir;
        let dot = d.dot(dir);
        if dot < 0.9999 {
            let angle = dot.clamp(-1.0, 1.0).acos();
            let share = (angle / std::f32::consts::PI).clamp(0.0, 1.0).powi(2);
            let rate = 5461.3335 + (2_184_533.2 - 5461.3335) * share;
            let unit = std::f32::consts::TAU / 65536.0;
            let turn = angle.min(rate * unit * SECONDS_PER_FRAME).min(131072.0 * unit * SECONDS_PER_FRAME);
            let axis = d.cross(dir).normalize_or_zero();
            if axis != Vec3::ZERO {
                d = Quat::from_axis_angle(axis, turn) * d;
            }
            d = d.normalize_or(Vec3::Z);
            knots[i + 1].dir = d;
            knots[i + 1].pt = a + d * len;
        }
    }
    for i in from..knots.len() - 1 {
        let (a, b) = (knots[i].pt, knots[i + 1].pt);
        knots[i + 1].pt = a + (b - a).normalize_or(Vec3::Z) * knots[i].length;
    }
}

/// A 16-bit angle (65536 a turn, wrapped to a half turn either way) in
/// radians.
fn wrap_angle(units: f32) -> f32 {
    f32::from(units as i64 as i16) * std::f32::consts::TAU / 65536.0
}

/// The first creature a sphere of `radius` meets from `from` along
/// `motion`, `ignore` passed by: its key, how far along (0 to 1), where.
fn first_actor(actors: &mut dyn ActorWorld, from: Vec3, motion: Vec3, radius: f32, ignore: Option<u64>) -> Option<(u64, f32, Vec3)> {
    let mut start = 0.0f32;
    for _ in 0..4 {
        let rest = motion * (1.0 - start);
        let (key, t, at) = actors.actor_hit(from + motion * start, rest, radius)?;
        let along = start + (1.0 - start) * t;
        if Some(key) != ignore {
            return Some((key, along, at));
        }
        start = (along + 0.01).min(1.0);
        if start >= 1.0 {
            return None;
        }
    }
    None
}

/// A frame with `up` as its y.
fn surface_basis(up: Vec3) -> Mat3 {
    let side = if up.y.abs() < 0.99 { Vec3::Y } else { Vec3::X };
    let x = side.cross(up).normalize_or(Vec3::X);
    Mat3::from_cols(x, up, x.cross(up))
}

/// `*blue-light-test*` (thin) and `*blue-light-test-big*` (thick).
fn blue_light(big: bool) -> crate::lightning_bolt::Appearance {
    use crate::lightning_bolt::{Appearance, Curve};
    Appearance {
        base_alpha: 1.0,
        width_range: if big { (8192.0, 8192.0) } else { (1228.8, 2048.0) },
        texture: if big { "pal-lightning" } else { "lightning-tile" },
        additive: true,
        fade_time: seconds(0.3),
        regenerate_time: if big { (seconds(0.035), seconds(0.067)) } else { (seconds(0.167), seconds(0.25)) },
        alpha_1: (Some(Curve::LinearDown), 0, 262144.0),
        alpha_2: (None, 3, 4096.0),
        width: (Some(Curve::LinearDown), 3, 4096.0),
        uv_repeat_dist: 28672.0,
        uv_shift: Some(seconds(-0.5)),
        accurate_interp: true,
    }
}

/// A knot of the Arc Wielder's rope (`constraint-knot`).
#[derive(Clone, Copy, Debug)]
struct Knot {
    pt: Vec3,
    dir: Vec3,
    length: f32,
}

/// The Arc Wielder (`gun-blue-2-lightning-tracker`, `gun-blue-shot.gc`):
/// a rope of twelve 6 m knots out of the muzzle (`*blue-2-lightning-
/// shape*`), each bending after the one before it (`rope-constraint-
/// method-9`), snapping onto a creature near it and stopping on the world;
/// along it a 3 m sweep that fires an unseen shot (`gun-blue-shot-2`, 2.5)
/// at what it finds; drawn as six lightning bolts (four thin, two thick);
/// its end on the world sparking.
#[derive(Clone, Debug)]
struct ArcTracker {
    created: i64,
    knots: [Knot; 12],
    /// In its `active` state (R1 held), else `inactive` (fading out).
    active: bool,
    state_time: i64,
    last_fired: i64,
    last_deduct_ammo_time: i64,
    prev_targ_pos: Vec3,
    revolve_angle: f32,
    sway_angle: f32,
    last_probe_index: i16,
    /// `lt-array`: four `*blue-light-test*` bolts, two `*blue-light-test-big*`.
    bolts: Vec<crate::lightning_bolt::Bolt>,
    /// The rope as last laid out (`lightning-info`): its points, which are
    /// on a creature, where it sparks.
    pts: Vec<Vec3>,
    attached: Vec<bool>,
    terminal: Option<Vec3>,
    /// `*gun-blue-2-last-attack-id-time*` and who that attack struck.
    attack_time: i64,
    attack_hits: Vec<u64>,
    last_spark_time: i64,
    spark_interval: i64,
    /// Shock lightning out: its id, from, to, when, spec.
    sparks: Vec<(u64, Vec3, Vec3, i64, &'static str)>,
    next_spark: u64,
    strips: Vec<crate::lightning_bolt::Strip>,
    last_draw: i64,
}

/// The Beam Reflexor's light trail (`*yellow-shot-2-trail*`): where its
/// shot has been over the last half second (`max-age`), each point with how
/// far the shot had flown there and when.
#[derive(Clone, Debug)]
struct LightTrail {
    id: u32,
    /// Whose: the Beam Reflexor's (`*yellow-shot-2-trail*`) or the Plasmite
    /// RPG grenade's (`*red-shot-3-trail*`).
    kind: GunKind,
    crumbs: Vec<(Vec3, f32, i64)>,
}

/// How long a light trail's points last.
const TRAIL_AGE: i64 = seconds(0.5);

/// `*curve-yellow2-shot-alpha*`: along the trail, back from its head over
/// 70 m (`alpha-repeat-dist`), faded in, then whole, then dashed.
const TRAIL_DASHES: [(f32, f32); 21] = [
    (0.0, 0.0),
    (0.1, 1.0),
    (0.4, 1.0),
    (0.401, 0.0),
    (0.45, 0.0),
    (0.451, 1.0),
    (0.5, 1.0),
    (0.501, 0.0),
    (0.55, 0.0),
    (0.551, 1.0),
    (0.6, 1.0),
    (0.601, 0.0),
    (0.65, 0.0),
    (0.651, 1.0),
    (0.7, 1.0),
    (0.701, 0.0),
    (0.75, 0.0),
    (0.751, 1.0),
    (0.8, 1.0),
    (0.801, 0.0),
    (1.0, 0.0),
];

/// A piecewise curve at `x` (clamped to its ends).
fn piecewise(points: &[(f32, f32)], x: f32) -> f32 {
    let Some(i) = points.iter().position(|&(px, _)| px >= x) else {
        return points.last().map_or(0.0, |p| p.1);
    };
    if i == 0 {
        return points[0].1;
    }
    let ((x0, y0), (x1, y1)) = (points[i - 1], points[i]);
    y0 + (y1 - y0) * ((x - x0) / (x1 - x0).max(1e-6))
}

/// Everything the guns have in flight.
pub struct Weapons {
    waves:Vec<wave::Wave>,
    shots: Vec<Shot>,
    trails: Vec<LightTrail>,
    next_trail: u32,
    effects: Vec<Effect>,
    arc: Option<ArcTracker>,
    /// Particle groups the guns start themselves (where, how turned).
    pending_groups: Vec<(&'static str, Vec3, Mat3)>,
    /// The Super Nova's white flash: when it began.
    pub flash: Option<i64>,
    rng: Rng,
    cache: CollideCache,
}

impl Default for Weapons {
    fn default() -> Self {
        Self {
            waves:Vec::new(),
            shots: Vec::new(),
            trails: Vec::new(),
            next_trail: 0,
            effects: Vec::new(),
            arc: None,
            pending_groups: Vec::new(),
            flash: None,
            rng: Rng(0x2545_f491),
            cache: CollideCache::default(),
        }
    }
}

impl Weapons {
    pub(crate) fn lightning_requests(&mut self, now: i64) -> Vec<crate::particles::LightningRequest> {
        // The Arc Wielder's sparks and shocks, each while its lightning lasts
        // (`:duration` and `:fade-time`).
        let mut arc_sparks = Vec::new();
        if let Some(arc) = self.arc.as_mut() {
            arc.sparks.retain(|s| now - s.3 < 75 + 30);
            arc_sparks.extend(arc.sparks.iter().map(|&(id, from, to, start, spec)| crate::particles::LightningRequest {
                id: 0xa7c0_0000_0000 ^ id,
                spec,
                from,
                to,
                age: (now - start) as f32 / TICKS_PER_SECOND as f32,
                // `process-drawable-shock-effect-replace` with
                // `lightning-probe-callback` (`gun-blue-shot.gc`).
                probe: true,
            }));
        }
        arc_sparks.into_iter().chain(self.effects
            .iter()
            .flat_map(|effect| {
                let Effect::Tendrils { origin, arcs, .. } = effect else {
                    return Vec::new();
                };
                arcs.iter()
                    .map(|(actor, at, start, _)| crate::particles::LightningRequest {
                        id: actor.wrapping_mul(2_654_435_761) ^ *start as u64,
                        spec: "lightning-dark-shot-attack-thick",
                        from: *origin,
                        to: *at,
                        age: (now - start) as f32 / TICKS_PER_SECOND as f32,
                        // `process-drawable-shock-effect-bullseye` with
                        // `lightning-probe-callback` (`gun-dark-shot.gc`).
                        probe: true,
                    })
                    .collect::<Vec<_>>()
            }))
            .collect()
    }
    pub(crate) fn particle_sources(&self, now: i64) -> Vec<crate::particles::ProjectileSource> {
        self.shots
            .iter()
            .filter_map(|shot| {
                let (pos, parts, charge): (_, &'static [u32], _) = match shot.state {
                    ShotState::Grenade { .. } => (shot.trans, &[218], None),
                    ShotState::Peace { charging, core, .. } => (
                        if charging { core } else { shot.trans },
                        &[],
                        Some(if charging {
                            ((now - shot.spawn) as f32 / 60.0).min(1.0)
                        } else {
                            1.0
                        }),
                    ),
                    ShotState::Nuke { .. } => (shot.trans, &[387, 388], None),
                    _ => return None,
                };
                Some(crate::particles::ProjectileSource {
                    id: (shot.spawn as u64)
                        .wrapping_mul(16)
                        .wrapping_add(shot.kind as u64),
                    at: pos,
                    parts,
                    charge,
                })
            })
            .collect()
    }
    /// Shots and effects carried into a new frame (see [`crate::reframe`]).
    pub(crate) fn reframe(&mut self, frame: &crate::reframe::Frame) {
        for wave in &mut self.waves{wave.origin=frame.point(wave.origin);}
        for shot in &mut self.shots {
            shot.trans = frame.point(shot.trans);
            shot.tail = frame.point(shot.tail);
            shot.transv = frame.dir(shot.transv);
        }
        for trail in &mut self.trails {
            for crumb in &mut trail.crumbs {
                crumb.0 = frame.point(crumb.0);
            }
        }
        for effect in &mut self.effects {
            match effect {
                Effect::Rays { rays, .. } => {
                    for (a, b) in rays {
                        *a = frame.point(*a);
                        *b = frame.point(*b);
                    }
                }
                Effect::Blast { center, .. }
                | Effect::Gravity { center, .. }
                | Effect::Nuke { center, .. } => *center = frame.point(*center),
                Effect::Left { pos, dir, .. } => {
                    *pos = frame.point(*pos);
                    *dir = frame.dir(*dir);
                }
                Effect::Tendrils {
                    origin,
                    queue,
                    arcs,
                    ..
                } => {
                    *origin = frame.point(*origin);
                    for (_, p) in queue {
                        *p = frame.point(*p);
                    }
                    for (_, p, ..) in arcs {
                        *p = frame.point(*p);
                    }
                }
            }
        }
        if let Some(arc) = &mut self.arc {
            for k in &mut arc.knots {
                k.pt = frame.point(k.pt);
                k.dir = frame.dir(k.dir);
            }
            arc.prev_targ_pos = frame.point(arc.prev_targ_pos);
            for p in &mut arc.pts {
                *p = frame.point(*p);
            }
        }
    }

    /// A mode whose last shot still charges on the muzzle.
    pub fn charging(&self) -> bool {
        self.waves.iter().any(wave::Wave::charging)||self.shots.iter().any(|s| {
                matches!(
                    s.state,
                    ShotState::Peace {
                        charging: true,
                        ..
                    }
                )
            })
    }

    /// The gun changed or went away: a charging wave fires as it is, a
    /// charging Peace Maker shot fizzles, the arc stops.
    pub(crate) fn cancel_charges(&mut self) {
        self.shots.retain(|s| {
            !matches!(
                s.state,
                ShotState::Peace {
                    charging: true,
                    ..
                }
            )
        });
        self.arc = None;
    }

    /// The Arc Wielder's rope is out this frame.
    pub fn arc_active(&self) -> bool {
        self.arc.as_ref().is_some_and(|a| a.active)
    }

    /// Particle groups the guns started since last taken.
    pub(crate) fn take_pending_groups(&mut self) -> Vec<(&'static str, Vec3, Mat3)> {
        std::mem::take(&mut self.pending_groups)
    }

    pub fn clear(&mut self) {
        self.waves.clear();
        self.shots.clear();
        self.trails.clear();
        self.effects.clear();
        self.arc = None;
        self.flash = None;
    }

    /// A new light trail of `kind`'s starting at `at`; its id.
    fn new_trail(&mut self, kind: GunKind, at: Vec3, now: i64) -> u32 {
        let id = self.next_trail;
        self.next_trail = self.next_trail.wrapping_add(1);
        self.trails.push(LightTrail {
            id,
            kind,
            crumbs: vec![(at, 0.0, now)],
        });
        id
    }

    /// The shot of trail `id` is at `at` now.
    fn crumb(&mut self, id: u32, at: Vec3, now: i64) {
        if let Some(trail) = self.trails.iter_mut().find(|t| t.id == id) {
            let flown = trail.crumbs.last().map_or(0.0, |&(p, d, _)| d + p.distance(at));
            trail.crumbs.push((at, flown, now));
        }
    }

    /// The saucer still flying, if any, and whether its second press has
    /// come.
    fn saucer(&mut self) -> Option<&mut Shot> {
        self.shots
            .iter_mut()
            .find(|s| matches!(s.state, ShotState::Saucer { .. }))
    }
}

/// Whether nothing solid lies on the line, with a thin probe.
fn clear_line(world: &mut dyn CollideWorld, cache: &mut CollideCache, from: Vec3, to: Vec3) -> bool {
    let motion = to - from;
    if motion.length_squared() < 1.0 {
        return true;
    }
    cache.fill_line_sphere(world, from, motion, 409.6);
    cache.probe_line_sphere(from, motion, 409.6).is_none()
}

/// `dir` turned toward `want` by at most `max` radians.
fn turn_toward(dir: Vec3, want: Vec3, max: f32) -> Vec3 {
    let (a, b) = (dir.normalize_or(Vec3::Z), want.normalize_or(Vec3::Z));
    let angle = a.dot(b).clamp(-1.0, 1.0).acos();
    if angle <= max || angle < 1e-5 {
        return b;
    }
    let axis = a.cross(b);
    let axis = if axis.length_squared() < 1e-10 {
        a.any_orthonormal_vector()
    } else {
        axis.normalize()
    };
    Quat::from_axis_angle(axis, max) * a
}

fn impact(at: Vec3, dir: Vec3, damage: f32, actor: Option<u64>) -> Event {
    Event::Impact(ProjectileHit {
        pos: at,
        surface: at,
        normal: -dir.normalize_or(Vec3::Y),
        damage,
        actor,
        dir: dir.normalize_or(Vec3::Z),
    })
}

/// A ring of beams around `center` in the horizontal plane.
fn ring(out: &mut Vec<Beam>, center: Vec3, radius: f32, rgba: [f32; 4], width: f32, segments: u32) {
    if radius <= 1.0 {
        return;
    }
    let point = |i: u32| {
        let a = i as f32 / segments as f32 * std::f32::consts::TAU;
        center + Vec3::new(a.cos(), 0.0, a.sin()) * radius
    };
    for i in 0..segments {
        out.push(Beam {
            head: point(i + 1),
            tail: point(i),
            rgba,
            width,
        });
    }
}

/// A rough ball of three rings.
fn ball(out: &mut Vec<Beam>, center: Vec3, radius: f32, rgba: [f32; 4], width: f32) {
    if radius <= 1.0 {
        return;
    }
    ring(out, center, radius, rgba, width, 20);
    for k in 0..2 {
        let rot = Quat::from_rotation_y(k as f32 * std::f32::consts::FRAC_PI_2);
        for i in 0..20 {
            let p = |i: u32| {
                let a = i as f32 / 20.0 * std::f32::consts::TAU;
                center + rot * Vec3::new(a.cos(), a.sin(), 0.0) * radius
            };
            out.push(Beam {
                head: p(i + 1),
                tail: p(i),
                rgba,
                width,
            });
        }
    }
}

/// A jagged line from `a` to `b`, as a bolt.
fn bolt(out: &mut Vec<Beam>, rng: &mut Rng, a: Vec3, b: Vec3, rgba: [f32; 4], width: f32) {
    let n = 5;
    let side = (b - a).any_orthonormal_vector();
    let up = (b - a).normalize_or(Vec3::Y).cross(side);
    let jitter = a.distance(b).min(meters(6.0)) * 0.08;
    let mut last = a;
    for i in 1..=n {
        let t = i as f32 / n as f32;
        let mut p = a.lerp(b, t);
        if i < n {
            p += side * rng.range(-jitter, jitter) + up * rng.range(-jitter, jitter);
        }
        out.push(Beam {
            head: p,
            tail: last,
            rgba,
            width,
        });
        last = p;
    }
}

impl Jak {
    /// Fires `kind` from the muzzle; false when the mode cannot fire now
    /// (a dark gun cooling down, a saucer still at work).
    pub(crate) fn fire_gun(
        &mut self,
        kind: GunKind,
        world: &mut dyn CollideWorld,
        mut actors: Option<&mut dyn ActorWorld>,
    ) -> bool {
        let now = self.time;
        let from = self.gun.fire_point;
        let dir = self.gun.fire_dir.normalize_or(Vec3::Z);
        let fired = match kind {
            GunKind::Yellow1 => {
                self.events.push(Event::Fire { from, dir });
                let mut shot = Projectile::yellow(from, dir, now);
                if let Some(hit) = shot.point_blank(world, &mut self.probe_cache) {
                    self.events.push(Event::Impact(hit));
                } else {
                    self.projectiles.push(shot);
                }
                true
            }
            GunKind::Yellow2 => {
                let trail = self.weapons.new_trail(GunKind::Yellow2, from, now);
                self.weapons.shots.push(Shot {
                    kind,
                    trans: from,
                    transv: dir * 819200.0,
                    tail: from,
                    spawn: now,
                    timeout: seconds(3.0),
                    damage: 1.5,
                    radius: 819.2,
                    state: ShotState::Reflect {
                        deflects: 4,
                        hit_enemy: false,
                        ignore: Vec::new(),
                        last_wall: LONG_AGO,
                        trail,
                    },
                });
                true
            }
            GunKind::Yellow3 => self.fire_saucer(from, dir),
            GunKind::Red1 => {
                self.fire_scatter(from, dir, world, reborrow(&mut actors));
                true
            }
            GunKind::Red2 => self.fire_wave(from),
            GunKind::Red3 => {
                self.fire_grenade(from, dir);
                true
            }
            GunKind::Blue1 => {
                let w = &mut self.weapons;
                let spread = w.rng.range(0.0, 1.1 * DEG);
                let roll = w.rng.range(0.0, std::f32::consts::TAU);
                let side = dir.any_orthonormal_vector();
                let bent = Quat::from_axis_angle(dir, roll) * (Quat::from_axis_angle(side, spread) * dir);
                let mut shot = Projectile::yellow(from, bent, now);
                shot.transv = bent * meters(80.0) * 60.0;
                shot.timeout = crate::math::FRAME_TICKS;
                shot.kind = GunKind::Blue1;
                self.projectiles.push(shot);
                true
            }
            GunKind::Blue2 => {
                self.arc_frame(world, reborrow(&mut actors));
                true
            }
            GunKind::Blue3 => {
                self.fire_needles(from, dir, world, reborrow(&mut actors));
                true
            }
            GunKind::Dark1 => {
                self.weapons.shots.push(Shot {
                    kind,
                    trans: from,
                    transv: Vec3::ZERO,
                    tail: from,
                    spawn: now,
                    timeout: seconds(5.0),
                    damage: 16.0,
                    radius: meters(0.4),
                    state: ShotState::Peace {
                        charging: true,
                        target: None,
                        core: from,
                        dir,
                        spin: 0.0,
                        launch: now,
                    },
                });
                true
            }
            GunKind::Dark2 => {
                if now - self.gun.last_fire[kind as usize] < seconds(2.0) {
                    return false;
                }
                let base = self.trans();
                let start = base + Vec3::Y * meters(6.0);
                let motion = -Vec3::Y * meters(20.0);
                let ground = {
                    let cache = &mut self.weapons.cache;
                    cache.fill_line_sphere(world, start, motion, 409.6);
                    cache
                        .probe_line_sphere(start, motion, 409.6)
                        .map_or(base, |h| h.intersect)
                };
                self.weapons.effects.push(Effect::Gravity {
                    center: ground,
                    start: now,
                    caught: Vec::new(),
                });
                self.events.push(Event::GravityField { center: ground });
                true
            }
            GunKind::Dark3 => {
                if now - self.gun.last_fire[kind as usize] < seconds(9.0) {
                    return false;
                }
                self.weapons.shots.push(Shot {
                    kind,
                    trans: from,
                    transv: dir * meters(8.0),
                    tail: from,
                    spawn: now,
                    timeout: seconds(4.0) + seconds(1.1),
                    damage: 32.0,
                    radius: 40.96,
                    state: ShotState::Nuke {
                        phase: 0,
                        phase_time: now,
                        base_y: from.y,
                        dir,
                        embedded: None,
                    },
                });
                true
            }
        };
        if fired {
            let upfront = matches!(
                kind,
                GunKind::Yellow1
                    | GunKind::Yellow2
                    | GunKind::Red1
                    | GunKind::Red3
                    | GunKind::Blue1
                    | GunKind::Dark1
                    | GunKind::Dark2
                    | GunKind::Dark3
            );
            if upfront {
                self.gun.use_ammo(kind.color(), kind.ammo_required());
            }
            if kind != GunKind::Yellow1 && kind != GunKind::Blue2 {
                self.events.push(Event::GunFire { kind, from });
            }
        }
        fired
    }

    /// Gyro Burster: a saucer, or the press that sets a flying one spinning.
    fn fire_saucer(&mut self, from: Vec3, dir: Vec3) -> bool {
        let now = self.time;
        if let Some(saucer) = self.weapons.saucer() {
            if let ShotState::Saucer {
                phase, phase_time, ..
            } = &mut saucer.state
                && matches!(*phase, SaucerPhase::Moving | SaucerPhase::Navigating)
            {
                *phase = SaucerPhase::Spinning;
                *phase_time = now;
                saucer.transv = Vec3::ZERO;
                self.gun.fire_pending = 0;
            }
            // The second press only activates it: no new saucer, no ammo.
            return false;
        }
        let launch = Vec3::new(dir.x, 0.6, dir.z).normalize_or(Vec3::Y);
        self.weapons.shots.push(Shot {
            kind: GunKind::Yellow3,
            trans: from,
            transv: launch * meters(100.0),
            tail: from,
            spawn: now,
            timeout: seconds(30.0),
            damage: 2.0,
            radius: meters(0.4),
            state: ShotState::Saucer {
                phase: SaucerPhase::Moving,
                phase_time: now,
                first_shot: None,
                last_volley: LONG_AGO,
                last_probe: now,
                spin: 0.0,
                drained: 0.0,
            },
        });
        true
    }

    /// Scatter Gun: nineteen probes in a cone, aimed at what is in front
    /// first, each creature struck once.
    fn fire_scatter(
        &mut self,
        from: Vec3,
        dir: Vec3,
        world: &mut dyn CollideWorld,
        mut actors: Option<&mut dyn ActorWorld>,
    ) {
        let now = self.time;
        // `gun-fire-red-1`: the cone at the muzzle along the shot (`manipy`
        // playing `idle` once).
        self.weapons.effects.push(Effect::Left { which: LeftModel::Cone, pos: from, dir, start: now });
        let yaw0 = dir.x.atan2(dir.z);
        let pitch0 = dir.y.clamp(-1.0, 1.0).asin();
        let probe = |yaw: f32, pitch: f32| {
            Vec3::new(yaw.sin() * pitch.cos(), pitch.sin(), yaw.cos() * pitch.cos())
        };
        let mut dirs = Vec::with_capacity(19);
        if let Some(a) = reborrow(&mut actors) {
            for (_, center) in a.actors_near(from + dir * meters(10.6), meters(10.7)) {
                if dirs.len() >= 19 {
                    break;
                }
                let to = (center - from).normalize_or(dir);
                let dyaw = wrap_pi(to.x.atan2(to.z) - yaw0).clamp(-45.0 * DEG, 45.0 * DEG);
                let dpitch = (to.y.clamp(-1.0, 1.0).asin() - pitch0).clamp(-15.0 * DEG, 15.0 * DEG);
                dirs.push(probe(yaw0 + dyaw, pitch0 + dpitch));
            }
        }
        while dirs.len() < 19 {
            let w = &mut self.weapons;
            let dyaw = w.rng.range(-45.0, 45.0) * DEG;
            let dpitch = w.rng.range(-15.0, 15.0) * DEG;
            dirs.push(probe(yaw0 + dyaw, pitch0 + dpitch));
        }
        let mut struck: Vec<u64> = Vec::new();
        let mut rays = Vec::with_capacity(19);
        let mut walls = 0;
        for d in dirs {
            let motion = d * meters(15.0);
            let cache = &mut self.weapons.cache;
            cache.fill_line_sphere(world, from, motion, 819.2);
            let wall = cache.probe_line_sphere(from, motion, 819.2);
            let wall_u = wall.map_or(1.0, |h| h.u);
            let actor = actors
                .as_deref_mut()
                .and_then(|a| a.actor_hit(from, motion, 819.2))
                .filter(|(_, u, _)| *u < wall_u);
            match actor {
                Some((key, u, point)) => {
                    rays.push((from, from + motion * u));
                    if !struck.contains(&key) {
                        struck.push(key);
                        let damage = if point.distance(from) <= meters(6.0) {
                            3.0
                        } else {
                            2.0
                        };
                        self.events.push(impact(point, d, damage, Some(key)));
                    }
                }
                None => {
                    rays.push((from, from + motion * wall_u));
                    if let Some(h) = wall
                        && walls < 6
                    {
                        walls += 1;
                        self.events.push(Event::Impact(ProjectileHit {
                            pos: h.intersect,
                            surface: h.intersect,
                            normal: h.normal,
                            damage: 1.0,
                            actor: None,
                            dir: d,
                        }));
                    }
                }
            }
        }
        self.weapons.effects.push(Effect::Rays {
            rays,
            start: now,
            duration: seconds(0.15),
            kind: GunKind::Red1,
        });
    }

    /// Plasmite RPG: lobbed at a target in front, or high and ahead.
    fn fire_grenade(&mut self, from: Vec3, dir: Vec3) {
        let speed = 266240.0;
        let mut launch = Vec3::new(dir.x, dir.y.max(0.3), dir.z).normalize_or(Vec3::Y);
        if self.gun.aim_dir.is_none()
            && let Some(target) = self.gun.aim
        {
            let d = target - from;
            let flat = Vec3::new(d.x, 0.0, d.z);
            let dxz = flat.length();
            if d.y.abs() < meters(6.0)
                && dxz > 1.0
                && dxz < meters(70.0)
                && flat
                    .normalize()
                    .dot(Vec3::new(dir.x, 0.0, dir.z).normalize_or(Vec3::Z))
                    > 0.707
            {
                let s = (GRAVITY_BOUNCE * dxz / (speed * speed)).min(1.0);
                let theta = 0.5 * s.asin();
                launch = flat.normalize() * theta.cos() + Vec3::Y * theta.sin();
            }
        }
        let trail = self.weapons.new_trail(GunKind::Red3, from, self.time);
        self.weapons.shots.push(Shot {
            kind: GunKind::Red3,
            trans: from,
            transv: self
                .gun
                .aim_dir
                .and_then(plasmite_velocity_to)
                .unwrap_or(launch * speed),
            tail: from,
            spawn: self.time,
            timeout: seconds(3.0),
            damage: 12.0,
            radius: 819.2,
            state: ShotState::Grenade {
                fuse: None,
                sitting: false,
                last_bounce: LONG_AGO,
                trail,
            },
        });
    }

    /// Needle Lazer: three shots that fan out, then home on targets picked
    /// at random, nearer and on-screen ones likelier.
    fn fire_needles(
        &mut self,
        from: Vec3,
        dir: Vec3,
        world: &mut dyn CollideWorld,
        actors: Option<&mut dyn ActorWorld>,
    ) {
        let now = self.time;
        let mut candidates: Vec<(u64, Vec3, f32)> = Vec::new();
        if let Some(a) = actors {
            for (key, center) in a.actors_near(from, meters(40.0)) {
                let to = center - from;
                if to.normalize_or_zero().y >= 0.5 {
                    continue;
                }
                if !clear_line(world, &mut self.weapons.cache, from, center) {
                    continue;
                }
                let near = Vec3::new(to.x, 0.0, to.z).length() < meters(12.0);
                let ahead = to.normalize_or_zero().dot(dir) > 0.5;
                let weight = 1.0 + 28.0 + if near { 4.0 } else { 0.0 } + if ahead { 2.0 } else { 0.0 };
                candidates.push((key, center, weight));
            }
        }
        self.gun.use_ammo(GunColor::Blue, 2.0);
        for _ in 0..3 {
            let w = &mut self.weapons;
            let total: f32 = candidates.iter().map(|c| c.2).sum();
            let picked = if total > 0.0 {
                let mut roll = w.rng.range(0.0, total);
                candidates
                    .iter()
                    .find(|c| {
                        roll -= c.2;
                        roll <= 0.0
                    })
                    .or(candidates.last())
                    .map(|c| (c.0, c.1))
            } else {
                None
            };
            let aim = picked.map_or_else(
                || {
                    let r = Vec3::new(w.rng.range(-1.0, 1.0), w.rng.range(-0.2, 0.5), w.rng.range(-1.0, 1.0));
                    from + r.normalize_or(dir) * meters(40.0)
                },
                |p| p.1,
            );
            let to = (aim - from).normalize_or(dir);
            let left = rotate_y_rad(dir, 15.0 * DEG);
            let right = rotate_y_rad(dir, -15.0 * DEG);
            let mut launch = if left.dot(to) < right.dot(to) { left } else { right };
            launch = rotate_y_rad(launch, w.rng.range(-15.0, 15.0) * DEG);
            launch.y += w.rng.range(-0.1, 0.4);
            let launch = launch.normalize_or(dir);
            let travel = 12288.0 + 28672.0 * w.rng.unit();
            w.shots.push(Shot {
                kind: GunKind::Blue3,
                trans: from,
                transv: launch * meters(80.0),
                tail: from,
                spawn: now,
                timeout: seconds(3.0),
                damage: 1.0,
                radius: 819.2,
                state: ShotState::Needle {
                    target: picked.map(|p| p.0),
                    last_pos: picked.map(|p| p.1),
                    travel,
                    travelled: 0.0,
                    mode: 0,
                    mode_time: now,
                },
            });
        }
    }

    /// `gun-fire-blue-2`, a frame of R1 held with the Arc Wielder: its
    /// tracker made if need be and sent `active` (`create-lightning-
    /// tracker-if-necessary`), then its `active` state's frame.
    fn arc_frame(&mut self, world: &mut dyn CollideWorld, mut actors: Option<&mut dyn ActorWorld>) {
        let now = self.time;
        if self.weapons.arc.is_none() {
            self.weapons.arc = Some(self.new_arc_tracker(now));
        }
        let entering = self.weapons.arc.as_ref().is_some_and(|a| !a.active);
        if entering {
            self.arc_enter_active(now);
        }
        let Some(arc) = self.weapons.arc.as_mut() else { return };
        arc.last_fired = now;
        // `:trans`: a turn a second round each knot; the ammunition 7.5 a
        // second, a tenth of a second at a time.
        arc.revolve_angle += 65536.0 * SECONDS_PER_FRAME;
        arc.sway_angle += 65536.0 * SECONDS_PER_FRAME;
        let owed = if now - arc.last_deduct_ammo_time >= seconds(0.1) {
            let owed = (now - arc.last_deduct_ammo_time) as f32 / TICKS_PER_SECOND as f32 * 7.5;
            arc.last_deduct_ammo_time = now;
            owed
        } else {
            0.0
        };
        if owed > 0.0 {
            self.gun.use_ammo(GunColor::Blue, owed);
        }
        self.arc_rope(world, reborrow(&mut actors));
    }

    /// `gun-blue-2-lightning-init-by-other`: its six bolts.
    fn new_arc_tracker(&self, now: i64) -> ArcTracker {
        let bolts = presentation::arc_bolts(now);
        let knot = Knot { pt: Vec3::ZERO, dir: Vec3::Z, length: meters(6.0) };
        ArcTracker {
            created: now,
            knots: [knot; 12],
            active: false,
            state_time: now,
            last_fired: now,
            last_deduct_ammo_time: now,
            prev_targ_pos: self.trans(),
            revolve_angle: 0.0,
            sway_angle: 0.0,
            last_probe_index: 0,
            bolts,
            pts: Vec::new(),
            attached: Vec::new(),
            terminal: None,
            attack_time: LONG_AGO,
            attack_hits: Vec::new(),
            last_spark_time: 0,
            spark_interval: 0,
            sparks: Vec::new(),
            next_spark: 0,
            strips: Vec::new(),
            last_draw: now,
        }
    }

    /// The `active` state's `:enter`: the rope laid straight out of the
    /// muzzle, an ammunition taken, its bolts lit, a new attack.
    fn arc_enter_active(&mut self, now: i64) {
        let from = self.gun.fire_point;
        let dir = self.gun.fire_dir.normalize_or(Vec3::Z);
        let trans = self.trans();
        let Some(arc) = self.weapons.arc.as_mut() else { return };
        arc.active = true;
        arc.prev_targ_pos = trans;
        arc.state_time = now;
        arc.last_deduct_ammo_time = now;
        for bolt in &mut arc.bolts {
            bolt.set_fade(0, now);
        }
        for (i, k) in arc.knots.iter_mut().enumerate() {
            *k = Knot { pt: from + dir * meters(6.0) * i as f32, dir, length: meters(6.0) };
        }
        if now - arc.attack_time >= seconds(0.4) || now < arc.attack_time {
            arc.attack_time = now;
            arc.attack_hits.clear();
        }
        self.gun.use_ammo(GunColor::Blue, 1.0);
    }

    /// `gun-blue-2-lightning-tracker-method-25`: the rope moved with Jak,
    /// bent after the muzzle, snapped onto creatures and stopped by the
    /// world, its shots fired along it; then drawn.
    fn arc_rope(&mut self, world: &mut dyn CollideWorld, mut actors: Option<&mut dyn ActorWorld>) {
        let trans = self.trans();
        let from = self.gun.fire_point;
        let laser = self.gun.fire_dir.normalize_or(Vec3::Z);
        let Some(mut arc) = self.weapons.arc.take() else { return };
        let mut moved = trans - arc.prev_targ_pos;
        moved.y = 0.0;
        for k in &mut arc.knots {
            k.pt += moved;
        }
        arc.prev_targ_pos = trans;
        arc.knots[0].pt = from;
        arc.knots[0].dir = laser;
        rope_constraint(&mut arc.knots, 0);
        if arc.last_probe_index == 0 {
            arc.last_probe_index = 1;
        }
        let mut cache: Vec<Vec3> = arc.knots.iter().map(|k| k.pt).collect();
        // `-26`: the muzzle inside a wall: only its sparks there.
        if !self.arc_backcheck(&mut arc, world, reborrow(&mut actors), laser, from) {
            self.weapons.arc = Some(arc);
            return;
        }
        let near = actors.as_deref_mut().map_or_else(Vec::new, |a| a.actors_near(from, meters(80.0)));
        let mut found: Vec<u64> = Vec::new();
        let mut attached = vec![false; 12];
        let mut last_target: i32 = -1;
        let mut num_pts = 1usize;
        let mut terminal = None;
        for s in 1..12 {
            let mut point = cache[s];
            // `find-gun-blue-2-target-old`: the nearest creature within 4 m
            // of the knot, ahead of it (dot over 0.6), not already on it.
            let dir = arc.knots[s].dir;
            let target = near
                .iter()
                .filter(|(k, c)| {
                    !found.contains(k) && c.distance(point) < meters(4.0) && (*c - point).normalize_or_zero().dot(dir) > 0.6
                })
                .min_by(|a, b| a.1.distance(point).total_cmp(&b.1.distance(point)))
                .copied();
            if let Some((key, center)) = target {
                found.push(key);
                attached[s] = true;
                last_target = s as i32;
                point = center;
                cache[s] = point;
                arc.knots[s].dir = (cache[s] - arc.knots[s - 1].pt).normalize_or(dir);
            }
            let prev = cache[s - 1];
            let motion = point - prev;
            let cache_world = &mut self.weapons.cache;
            cache_world.fill_line_sphere(world, prev, motion, 860.16);
            let wall = cache_world.probe_line_sphere(prev, motion, 860.16);
            if let Some(hit) = &wall {
                point = prev + motion * hit.u;
                arc.last_probe_index = s as i16;
                let back = (point - prev).normalize_or_zero();
                terminal = Some(point - back * 6144.0);
                self.arc_world_hit(&mut arc, hit.intersect, hit.normal);
            }
            cache[s] = point;
            num_pts += 1;
            let ignore = if found.len() > 1 { Some(found[found.len() - 2]) } else { None };
            self.arc_fire_if_necessary(&mut arc, world, reborrow(&mut actors), prev, point, ignore);
            if wall.is_some() {
                break;
            }
        }
        // The last point on a creature: no sparks there.
        if last_target == num_pts as i32 - 2 {
            terminal = None;
        }
        // The knots on no creature swung half a metre about the rope.
        let mut pts = cache[..num_pts].to_vec();
        for (i, p) in pts.iter_mut().enumerate().skip(1) {
            if attached[i] {
                continue;
            }
            let dir = arc.knots[i].dir.normalize_or(Vec3::Z);
            let right = Vec3::Y.cross(dir).normalize_or(Vec3::X);
            let up = dir.cross(right);
            // `(sar (shl (the int angle) 48) 48)`: as a 16-bit angle.
            let angle = wrap_angle(arc.revolve_angle + 6144.0 * i as f32);
            *p += (Quat::from_axis_angle(dir, angle) * up).normalize_or_zero() * 2048.0;
        }
        arc.pts = pts;
        arc.attached = attached[..num_pts].to_vec();
        arc.terminal = terminal;
        self.arc_setup_draw(&mut arc, world);
        self.weapons.arc = Some(arc);
    }

    /// `gun-blue-2-lightning-tracker-method-26`: a probe from 1.8 m behind
    /// the muzzle to just past it; blocked (the gun in a wall), only the
    /// sparks where it stops and false; else a shot along it and true.
    fn arc_backcheck(
        &mut self,
        arc: &mut ArcTracker,
        world: &mut dyn CollideWorld,
        actors: Option<&mut dyn ActorWorld>,
        laser: Vec3,
        from: Vec3,
    ) -> bool {
        let start = from - laser * 7372.8;
        let motion = (from - start) * 1.2;
        let cache = &mut self.weapons.cache;
        cache.fill_line_sphere(world, start, motion, 409.6);
        if let Some(hit) = cache.probe_line_sphere(start, motion, 409.6) {
            let at = start + motion * hit.u;
            arc.pts = vec![at];
            arc.attached = vec![false];
            arc.terminal = Some(at);
            self.arc_setup_draw(arc, world);
            return false;
        }
        self.arc_fire_if_necessary(arc, world, actors, start, start + motion, None);
        true
    }

    /// `fire-projectile-if-necessary`: a 3 m sweep along twice the span;
    /// anything there, an unseen shot (`gun-blue-shot-2`) down four times
    /// the span, 2.5 to the first creature it meets that this attack has
    /// not yet struck (one attack every 0.4 s).
    fn arc_fire_if_necessary(
        &mut self,
        arc: &mut ArcTracker,
        world: &mut dyn CollideWorld,
        mut actors: Option<&mut dyn ActorWorld>,
        a: Vec3,
        b: Vec3,
        ignore: Option<u64>,
    ) {
        let now = self.time;
        let Some(actors) = actors.as_deref_mut() else { return };
        let sweep = (b - a) * 2.0;
        if first_actor(actors, a, sweep, meters(3.0), ignore).is_none() {
            return;
        }
        if now - arc.attack_time >= seconds(0.4) {
            arc.attack_time = now;
            arc.attack_hits.clear();
        }
        // The shot: its 0.2 m sphere against the world, its 1 m one against
        // creatures (`setup-collision!`), a frame of flight.
        let flight = sweep * 2.0;
        let cache = &mut self.weapons.cache;
        cache.fill_line_sphere(world, a, flight, 819.2);
        let reach = cache.probe_line_sphere(a, flight, 819.2).map_or(1.0, |h| h.u);
        let Some((key, t, at)) = first_actor(actors, a, flight, meters(1.0), ignore) else { return };
        if t > reach || arc.attack_hits.contains(&key) {
            return;
        }
        arc.attack_hits.push(key);
        self.events.push(impact(at, flight, 2.5, Some(key)));
        // `deal-damage!`: two times in eleven its lightning on what it struck.
        if self.weapons.rng.below(11) < 2 {
            self.arc_shock(arc, world, at, "lightning-blue-shot-attack-explode", meters(10.0), Some(key));
        }
    }

    /// Where the rope ends on the world, a blow to it as a creature's, once
    /// an attack (the mashup's: 19/10, the Arc Wielder left Minecraft's
    /// blocks whole; Jak 3's world takes no blows).
    fn arc_world_hit(&mut self, arc: &mut ArcTracker, surface: Vec3, normal: Vec3) {
        const WALL: u64 = u64::MAX;
        if self.time - arc.attack_time >= seconds(0.4) {
            arc.attack_time = self.time;
            arc.attack_hits.clear();
        }
        if !arc.attack_hits.contains(&WALL) {
            arc.attack_hits.push(WALL);
            self.events.push(Event::WorldHit { surface, normal, damage: 2.5 });
        }
    }

    /// `process-drawable-shock-effect(-replace)` from `at`: a probe out
    /// `reach` along one of six directions turned at random; meeting the
    /// world over 2 m off, lightning of `spec` out to it (and the tip's
    /// sparks there).
    fn arc_shock(&mut self, arc: &mut ArcTracker, world: &mut dyn CollideWorld, at: Vec3, spec: &'static str, reach: f32, _on: Option<u64>) {
        const DIRS: [Vec3; 6] = [Vec3::Z, Vec3::X, Vec3::NEG_Z, Vec3::NEG_X, Vec3::Y, Vec3::NEG_Y];
        let rng = &mut self.weapons.rng;
        let turn = Mat3::from_rotation_z(rng.unit() * std::f32::consts::TAU)
            * Mat3::from_rotation_y(rng.unit() * std::f32::consts::TAU)
            * Mat3::from_rotation_x(rng.unit() * std::f32::consts::TAU);
        for dir in DIRS.iter().rev() {
            let motion = (turn * *dir).normalize_or(Vec3::Y) * reach;
            let cache = &mut self.weapons.cache;
            cache.fill_line_sphere(world, at, motion, 409.6);
            let Some(hit) = cache.probe_line_sphere(at, motion, 409.6) else { continue };
            let end = hit.intersect;
            if end.distance(at) <= 8192.0 {
                continue;
            }
            arc.next_spark += 1;
            arc.sparks.push((arc.next_spark, at, end, self.time, spec));
            let up = hit.normal.normalize_or(Vec3::Y);
            self.weapons.pending_groups.push(("group-lightning-dark-shot-tip-hit-replace", end + up * 409.6, surface_basis(up)));
            break;
        }
    }

    /// `setup-draw!`: the rope's points given to the six bolts, each moved
    /// on and drawn; where it ends on the world, three to five sparks every
    /// 15 to 25 ticks.
    fn arc_setup_draw(&mut self, arc: &mut ArcTracker, world: &mut dyn CollideWorld) {
        let now = self.time;
        presentation::configure_arc(&mut arc.bolts, &arc.pts);
        self.arc_bolts_frame(arc);
        if let Some(_) = arc.terminal
            && now - arc.last_spark_time >= arc.spark_interval
        {
            arc.last_spark_time = now;
            let rng = &mut self.weapons.rng;
            arc.spark_interval = (5.0 * (rng.below(3) + 3) as f32) as i64;
            let count = rng.below(3) + 3;
            let at = arc.bolts[5].span_pts_start[arc.bolts[5].num_active_spans];
            arc.sparks.retain(|s| s.4 != "lightning-blue-shot-terminal");
            for _ in 0..count {
                self.arc_shock(arc, world, at, "lightning-blue-shot-terminal", meters(10.0), None);
            }
        }
    }

    /// The six bolts moved on a frame and drawn (`lightning-bolt-method-11`
    /// and `-12`).
    fn arc_bolts_frame(&mut self, arc: &mut ArcTracker) {
        let now = self.time;
        let prev = arc.last_draw.min(now);
        let up = self.camera.u;
        arc.strips.clear();
        for bolt in &mut arc.bolts {
            bolt.update(now, prev);
            let [a, b] = bolt.draw(now, prev, up);
            arc.strips.push(a);
            arc.strips.push(b);
        }
        arc.last_draw = now;
    }

    /// The guns' shots and effects, one frame.
    pub(crate) fn step_weapons(
        &mut self,
        world: &mut dyn CollideWorld,
        mut actors: Option<&mut dyn ActorWorld>,
    ) {
        let now = self.time;
        self.arc_idle(world, reborrow(&mut actors));
        self.step_waves(world,reborrow(&mut actors));
        let mut shots = std::mem::take(&mut self.weapons.shots);
        let mut keep = Vec::with_capacity(shots.len());
        for mut shot in shots.drain(..) {
            if self.step_shot(&mut shot, world, reborrow(&mut actors)) {
                keep.push(shot);
            }
        }
        keep.append(&mut self.weapons.shots);
        self.weapons.shots = keep;
        // Light trails fade from their tails, gone with their last point.
        for trail in &mut self.weapons.trails {
            trail.crumbs.retain(|c| now - c.2 < TRAIL_AGE);
        }
        self.weapons.trails.retain(|t| !t.crumbs.is_empty());
        let mut effects = std::mem::take(&mut self.weapons.effects);
        let mut kept = Vec::with_capacity(effects.len());
        for mut effect in effects.drain(..) {
            if self.step_effect(&mut effect, world, reborrow(&mut actors)) {
                kept.push(effect);
            }
        }
        kept.append(&mut self.weapons.effects);
        self.weapons.effects = kept;
        if self
            .weapons
            .flash
            .is_some_and(|at| now - at > seconds(2.0))
        {
            self.weapons.flash = None;
        }
    }

    /// Sweeps a shot by `motion` against the world and the creatures; the
    /// first contact.
    fn sweep_shot(
        &mut self,
        shot: &Shot,
        motion: Vec3,
        world: &mut dyn CollideWorld,
        actors: Option<&mut dyn ActorWorld>,
        actor_radius: f32,
    ) -> Option<(f32, Vec3, Vec3, Option<u64>)> {
        let cache = &mut self.weapons.cache;
        cache.fill_line_sphere(world, shot.trans, motion, shot.radius);
        let mut best = cache
            .resolve_moving_sphere(shot.trans, shot.radius, motion, -1.0, true)
            .map(|h| (h.u, h.intersect, h.normal, None));
        if actor_radius > 0.0
            && let Some(a) = actors
            && let Some((key, u, point)) = a.actor_hit(shot.trans, motion, actor_radius)
            && best.is_none_or(|b| u < b.0)
        {
            best = Some((u, point, -motion.normalize_or(Vec3::Z), Some(key)));
        }
        best
    }

    /// One frame of a shot; false when it is gone.
    fn step_shot(
        &mut self,
        shot: &mut Shot,
        world: &mut dyn CollideWorld,
        mut actors: Option<&mut dyn ActorWorld>,
    ) -> bool {
        let now = self.time;
        let dt = SECONDS_PER_FRAME;
        let age = now - shot.spawn;
        match shot.state.clone() {
            ShotState::Reflect {
                mut deflects,
                mut hit_enemy,
                mut ignore,
                mut last_wall,
                trail: trail_id,
            } => {
                if age >= shot.timeout {
                    return false;
                }
                ignore.retain(|(_, at)| now - at < seconds(0.15));
                let motion = shot.transv * dt;
                let contact = self.sweep_shot(shot, motion, world, reborrow(&mut actors), 4096.0);
                let contact = contact.filter(|c| c.3.is_none_or(|k| !ignore.iter().any(|(i, _)| *i == k)));
                match contact {
                    None => shot.trans += motion,
                    Some((u, point, normal, actor)) => {
                        shot.trans += motion * u.max(0.0);
                        let mut v = shot.transv.normalize_or(Vec3::Z);
                        match actor {
                            Some(key) => {
                                let damage = if hit_enemy { 1.0 } else { shot.damage };
                                hit_enemy = true;
                                self.events.push(impact(point, v, damage, Some(key)));
                                ignore.push((key, now));
                                deflects = deflects.saturating_sub(1);
                                if deflects == 0 {
                                    self.weapons.crumb(trail_id, shot.trans, now);
                                    return false;
                                }
                                v = -v;
                            }
                            None => {
                                let incoming = v;
                                // (19/10: it bounced off a Minecraft block
                                // and left it whole.)
                                self.events.push(Event::WorldHit { surface: point, normal, damage: shot.damage });
                                v = v - normal * 2.0 * v.dot(normal);
                                if v.y > 0.2 {
                                    v.y = incoming.y.max(0.2);
                                }
                                shot.trans = point + normal * 1024.0;
                            }
                        }
                        let retarget = now - last_wall >= seconds(0.05) && self.weapons.rng.unit() < 0.75;
                        last_wall = now;
                        if retarget && let Some(a) = reborrow(&mut actors) {
                            let flat = Vec3::new(v.x, 0.0, v.z).normalize_or_zero();
                            let best = a
                                .actors_near(shot.trans + v * meters(50.0), meters(100.0))
                                .into_iter()
                                .filter(|(k, _)| !ignore.iter().any(|(i, _)| i == k))
                                .map(|(_, c)| {
                                    let d = Vec3::new(c.x - shot.trans.x, 0.0, c.z - shot.trans.z)
                                        .normalize_or_zero();
                                    (d.dot(flat), c)
                                })
                                .filter(|(d, _)| *d > 0.0)
                                .max_by(|a, b| a.0.total_cmp(&b.0));
                            if let Some((_, c)) = best {
                                v = (c - shot.trans).normalize_or(v);
                            }
                        }
                        shot.transv = v.normalize_or(Vec3::Z) * 546133.3;
                        shot.tail = shot.trans;
                        self.events.push(Event::GunBounce {
                            kind: shot.kind,
                            pos: shot.trans,
                        });
                    }
                }
                shot.state = ShotState::Reflect {
                    deflects,
                    hit_enemy,
                    ignore,
                    last_wall,
                    trail: trail_id,
                };
                self.weapons.crumb(trail_id, shot.trans, now);
                trail(shot, TAIL_MAX);
                true
            }
            ShotState::Grenade {
                mut fuse,
                mut sitting,
                mut last_bounce,
                trail: trail_id,
            } => {
                let explode_now = age >= shot.timeout || fuse.is_some_and(|f| now >= f);
                if explode_now {
                    // Not straight into a creature (`immediate-detonation?`):
                    // its sphere where it burst.
                    self.weapons.effects.push(Effect::Left { which: LeftModel::RedSphere, pos: shot.trans, dir: Vec3::Z, start: now });
                    self.explode(shot.kind, shot.trans, meters(20.0), shot.damage, actors);
                    return false;
                }
                if !sitting {
                    shot.transv.y -= GRAVITY_BOUNCE * dt;
                    let speed = shot.transv.length();
                    if speed > 327680.0 {
                        shot.transv *= 327680.0 / speed;
                    }
                    let motion = shot.transv * dt;
                    match self.sweep_shot(shot, motion, world, reborrow(&mut actors), 2048.0) {
                        None => shot.trans += motion,
                        Some((_, point, _, Some(_))) => {
                            self.explode(shot.kind, point, meters(20.0), shot.damage, actors);
                            return false;
                        }
                        Some((u, point, normal, None)) => {
                            shot.trans += motion * u.max(0.0);
                            shot.trans = shot.trans.lerp(point + normal * shot.radius * 1.05, 0.5);
                            let k = self.weapons.rng.range(1.6, 2.2);
                            let v = shot.transv;
                            shot.transv = (v - normal * v.dot(normal) * k) * 0.6;
                            if now - last_bounce >= seconds(0.3) {
                                self.events.push(Event::GunBounce {
                                    kind: shot.kind,
                                    pos: shot.trans,
                                });
                                last_bounce = now;
                            }
                            if shot.transv.length() < meters(1.0) && normal.y > 0.5 {
                                sitting = true;
                                shot.transv = Vec3::ZERO;
                            }
                        }
                    }
                }
                if let Some(a) = reborrow(&mut actors) {
                    let speed = shot.transv.length().max(1.0);
                    for (_, c) in a.actors_near(shot.trans, meters(13.3)) {
                        let to = c - shot.trans;
                        let dist = to.length();
                        let dot = to.normalize_or_zero().dot(shot.transv / speed);
                        if dot > 0.0 && age > seconds(0.05) {
                            let delay = (300.0 * dist / speed * dot).min(150.0) as i64;
                            let at = now + delay;
                            fuse = Some(fuse.map_or(at, |f| f.min(at)));
                        } else if dist < meters(2.0) {
                            fuse = Some(now);
                        }
                    }
                }
                shot.state = ShotState::Grenade {
                    fuse,
                    sitting,
                    last_bounce,
                    trail: trail_id,
                };
                self.weapons.crumb(trail_id, shot.trans, now);
                trail(shot, meters(2.0));
                true
            }
            ShotState::Needle {
                target,
                mut last_pos,
                travel,
                mut travelled,
                mut mode,
                mut mode_time,
            } => {
                if age >= shot.timeout {
                    return false;
                }
                let live = target.and_then(|key| {
                    reborrow(&mut actors).and_then(|a| {
                        a.actors_near(last_pos.unwrap_or(shot.trans), meters(60.0))
                            .into_iter()
                            .find(|(k, _)| *k == key)
                            .map(|(_, c)| c)
                    })
                });
                if let Some(c) = live {
                    last_pos = Some(c);
                } else if mode == 1 {
                    mode = 2;
                    mode_time = now;
                }
                let goal = last_pos;
                for _ in 0..2 {
                    let dir = shot.transv.normalize_or(Vec3::Z);
                    let (dot, dist) = goal.map_or((1.0, f32::MAX), |g| {
                        ((g - shot.trans).normalize_or(dir).dot(dir), g.distance(shot.trans))
                    });
                    let speed = lerp(122880.0, 409600.0, ((1.0 + dot) / 2.0).powi(2));
                    let mut new_dir = dir;
                    if mode == 0 && (travelled >= travel || dist < meters(10.0)) {
                        mode = 1;
                        mode_time = now;
                    }
                    if mode == 2 && dist < meters(8.0) {
                        mode = 3;
                    }
                    if (mode == 1 || mode == 2)
                        && let Some(g) = goal
                        && dist > meters(2.0)
                    {
                        let mut rate = 360.0 * DEG;
                        if dist < meters(12.0) {
                            rate = lerp(360.0, 720.0, dist / meters(12.0)) * DEG * (2.0 - (1.0 + dot) / 2.0);
                        }
                        let t = (now - mode_time) as f32 / TICKS_PER_SECOND as f32;
                        if mode == 1 && t > 1.0 {
                            rate *= lerp(1.0, 2.0, ((t - 0.75) * 1.333).clamp(0.0, 1.0));
                        }
                        new_dir = turn_toward(dir, g - shot.trans, rate * dt * 0.5);
                    }
                    shot.transv = new_dir * speed;
                    let motion = shot.transv * dt * 0.5;
                    match self.sweep_shot(shot, motion, world, reborrow(&mut actors), 2048.0) {
                        None => {
                            shot.trans += motion;
                            travelled += motion.length();
                        }
                        Some((_, point, normal, actor)) => {
                            let hit_dir = shot.transv.normalize_or(Vec3::Z);
                            match actor {
                                Some(_) => self.events.push(impact(point, hit_dir, shot.damage, actor)),
                                None => self.events.push(Event::Impact(ProjectileHit {
                                    pos: point,
                                    surface: point,
                                    normal,
                                    damage: shot.damage,
                                    actor: None,
                                    dir: hit_dir,
                                })),
                            }
                            return false;
                        }
                    }
                }
                shot.state = ShotState::Needle {
                    target,
                    last_pos,
                    travel,
                    travelled,
                    mode,
                    mode_time,
                };
                trail(shot, meters(6.0));
                true
            }
            ShotState::Peace {
                mut charging,
                mut target,
                mut core,
                mut dir,
                mut spin,
                mut launch,
            } => {
                if charging {
                    core = self.gun.fire_point;
                    shot.trans = core;
                    shot.tail = core;
                    dir = self.gun.fire_dir.normalize_or(dir);
                    let held = self.pad.hold(button::R1);
                    if (!held && age >= seconds(0.3)) || age >= seconds(4.0) {
                        charging = false;
                        launch = now;
                        target = reborrow(&mut actors).and_then(|a| {
                            a.actors_near(core, meters(400.0))
                                .into_iter()
                                .filter(|(_, c)| (*c - core).normalize_or_zero().dot(dir) > 0.6)
                                .min_by(|x, y| x.1.distance(core).total_cmp(&y.1.distance(core)))
                                .map(|(k, _)| k)
                        });
                        self.events.push(Event::GunFire {
                            kind: GunKind::Dark1,
                            from: core,
                        });
                    }
                } else {
                    let flight = now - launch;
                    let goal = target.and_then(|key| {
                        reborrow(&mut actors).and_then(|a| {
                            a.actors_near(core, meters(400.0))
                                .into_iter()
                                .find(|(k, _)| *k == key)
                                .map(|(_, c)| c)
                        })
                    });
                    if let Some(g) = goal {
                        let dist = g.distance(core);
                        let max = lerp_scale(1.0, 45.0, dist, meters(15.0), meters(1.0)) * DEG;
                        dir = turn_toward(dir, g - core, max);
                    }
                    let motion = dir * meters(1.8);
                    let probe = Shot {
                        trans: core,
                        ..shot.clone()
                    };
                    let contact = self.sweep_shot(&probe, motion, world, reborrow(&mut actors), 4096.0);
                    let end = match contact {
                        Some((u, _, _, _)) => {
                            core += motion * u.max(0.0);
                            true
                        }
                        None => {
                            core += motion;
                            flight >= seconds(1.5)
                        }
                    };
                    if end {
                        self.peace_impact(core, dir, actors);
                        return false;
                    }
                    spin += 11.0 * DEG;
                    let t = flight as f32;
                    let radius = if t < 90.0 {
                        lerp(0.0, 4096.0, t / 90.0)
                    } else {
                        lerp_scale(4096.0, 1228.8, t, 30.0, 300.0)
                    } * 0.5;
                    let side = dir.any_orthonormal_vector();
                    let offset = Quat::from_axis_angle(dir, spin) * side * radius;
                    shot.trans = core + offset;
                    trail(shot, meters(4.0));
                }
                shot.state = ShotState::Peace {
                    charging,
                    target,
                    core,
                    dir,
                    spin,
                    launch,
                };
                true
            }
            ShotState::Nuke {
                mut phase,
                mut phase_time,
                mut base_y,
                mut dir,
                mut embedded,
            } => {
                if let Some(at) = embedded {
                    if now - at >= seconds(1.4) {
                        self.weapons.effects.push(Effect::Nuke {
                            center: shot.trans,
                            start: now,
                            killed: false,
                            small: true,
                        });
                        // impact-small: its stream "pg3wxplo" (gui-control's explode-wall-sound).
                        self.events.push(Event::Sound { name: "jak3:pg3wxplo", volume: 1.0, pitch: 0.0 });
                        return false;
                    }
                    let beep = |s: f32| now - at >= seconds(s) && now - at - FRAME_TICKS < seconds(s);
                    if beep(0.3) || beep(0.5) {
                        self.events.push(Event::GunBounce {
                            kind: GunKind::Dark3,
                            pos: shot.trans,
                        });
                    }
                    return true;
                }
                let t = (now - phase_time) as f32 / TICKS_PER_SECOND as f32;
                let (speed, next) = match phase {
                    0 => (meters(8.0), t >= 0.2),
                    1 => (lerp(meters(15.0), meters(50.0), t / 0.4), t >= 0.4),
                    2 => (lerp(meters(50.0), meters(135.0), t / 0.5), t >= 0.5),
                    _ => (meters(135.0), false),
                };
                if phase == 3 && now - shot.spawn >= seconds(1.1) + seconds(4.0) {
                    return false;
                }
                let mut motion = dir * speed * dt;
                if phase == 2 {
                    let flat = Vec3::new(dir.x, 0.0, dir.z).normalize_or(Vec3::Z);
                    let y = base_y
                        + 34816.0 * (lerp(0.0, 270.0, (t / 0.5).clamp(0.0, 1.0).powi(2)) * DEG).sin();
                    motion = flat * speed * dt + Vec3::Y * (y - shot.trans.y);
                }
                match self.sweep_shot(shot, motion, world, None, 0.0) {
                    None => shot.trans += motion,
                    Some((u, _, normal, _)) => {
                        shot.trans += motion * u.max(0.0);
                        let wall = normal.y.abs() < 0.5;
                        if phase < 2 || (phase == 2 && t <= 0.7) || wall {
                            embedded = Some(now);
                            shot.transv = Vec3::ZERO;
                        } else {
                            self.weapons.effects.push(Effect::Nuke {
                                center: shot.trans,
                                start: now,
                                killed: false,
                                small: false,
                            });
                            // impact's :enter: its stream "pg3nxplo" (explode-sound).
                            self.events.push(Event::Sound { name: "jak3:pg3nxplo", volume: 1.0, pitch: 0.0 });
                            return false;
                        }
                    }
                }
                if next && phase < 3 {
                    phase += 1;
                    phase_time = now;
                    if phase == 2 {
                        base_y = shot.trans.y;
                    }
                    if phase == 3 {
                        dir = Vec3::new(dir.x, 0.0, dir.z).normalize_or(Vec3::Z);
                    }
                }
                shot.transv = motion / dt;
                shot.state = ShotState::Nuke {
                    phase,
                    phase_time,
                    base_y,
                    dir,
                    embedded,
                };
                trail(shot, meters(8.0));
                true
            }
            ShotState::Saucer {
                mut phase,
                mut phase_time,
                mut first_shot,
                mut last_volley,
                mut last_probe,
                mut spin,
                mut drained,
            } => self.step_saucer(
                shot,
                world,
                actors,
                &mut phase,
                &mut phase_time,
                &mut first_shot,
                &mut last_volley,
                &mut last_probe,
                &mut spin,
                &mut drained,
            ),
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn step_saucer(
        &mut self,
        shot: &mut Shot,
        world: &mut dyn CollideWorld,
        mut actors: Option<&mut dyn ActorWorld>,
        phase: &mut SaucerPhase,
        phase_time: &mut i64,
        first_shot: &mut Option<i64>,
        last_volley: &mut i64,
        last_probe: &mut i64,
        spin: &mut f32,
        drained: &mut f32,
    ) -> bool {
        let now = self.time;
        let dt = SECONDS_PER_FRAME;
        let t = now - *phase_time;
        *spin += 720.0 * DEG * dt;
        let go = |p: SaucerPhase, phase: &mut SaucerPhase, at: &mut i64| {
            *phase = p;
            *at = now;
        };
        match *phase {
            SaucerPhase::Moving => {
                shot.transv.y -= GRAVITY_BOUNCE * dt;
                let motion = shot.transv * dt;
                if let Some((u, _, _, _)) = self.sweep_shot(shot, motion, world, None, 0.0) {
                    shot.trans += motion * u.max(0.0);
                    self.explode(GunKind::Yellow3, shot.trans, 0.0, 2.0, actors);
                    return false;
                }
                shot.trans += motion;
                if t >= seconds(0.05) {
                    go(SaucerPhase::Navigating, phase, phase_time);
                }
            }
            SaucerPhase::Navigating => {
                let speed = lerp(409600.0, 40960.0, (t as f32 / 30.0).min(1.0));
                let mut v = shot.transv;
                v.y = lerp(v.y, 0.0, (t as f32 / 210.0).min(1.0));
                let flat = Vec3::new(v.x, 0.0, v.z).normalize_or(Vec3::Z);
                let flat_speed = (speed * speed - v.y * v.y).max(0.0).sqrt();
                v = flat * flat_speed + Vec3::Y * v.y;
                if now - *last_probe >= seconds(0.5) {
                    *last_probe = now;
                    let probe = flat * meters(10.0);
                    let cache = &mut self.weapons.cache;
                    cache.fill_line_sphere(world, shot.trans, probe, shot.radius);
                    if let Some(h) = cache.probe_line_sphere(shot.trans, probe, shot.radius) {
                        let n = Vec3::new(h.normal.x, 0.0, h.normal.z).normalize_or(-flat);
                        v = (flat - n * 2.0 * flat.dot(n)).normalize_or(-flat) * 32768.0;
                    }
                }
                let motion = v * dt;
                match self.sweep_shot(shot, motion, world, None, 0.0) {
                    None => shot.trans += motion,
                    Some((u, _, n, _)) => {
                        shot.trans += motion * u.max(0.0);
                        let n = Vec3::new(n.x, 0.0, n.z).normalize_or(-flat);
                        v = (flat - n * 2.0 * flat.dot(n)).normalize_or(-flat) * 32768.0;
                    }
                }
                shot.transv = v;
            }
            SaucerPhase::Spinning => shot.transv = Vec3::ZERO,
            SaucerPhase::Falling | SaucerPhase::Sitting => {
                if *phase == SaucerPhase::Falling {
                    shot.transv.x = 0.0;
                    shot.transv.z = 0.0;
                    shot.transv.y -= GRAVITY_BOUNCE * dt;
                    let motion = shot.transv * dt;
                    match self.sweep_shot(shot, motion, world, None, 0.0) {
                        None => shot.trans += motion,
                        Some((u, _, _, _)) => {
                            shot.trans += motion * u.max(0.0);
                            shot.transv = Vec3::ZERO;
                            self.events.push(Event::GunBounce {
                                kind: GunKind::Yellow3,
                                pos: shot.trans,
                            });
                            go(SaucerPhase::Sitting, phase, phase_time);
                        }
                    }
                    if t >= seconds(3.0) {
                        go(SaucerPhase::Sitting, phase, phase_time);
                    }
                } else if t >= seconds(1.0) {
                    go(SaucerPhase::Husk, phase, phase_time);
                }
            }
            SaucerPhase::Husk => {
                if t >= 60 {
                    return false;
                }
            }
        }
        // Firing: on its own a second into navigating, at once spinning.
        let firing = match *phase {
            SaucerPhase::Navigating => t >= seconds(1.0) || first_shot.is_some(),
            SaucerPhase::Spinning => true,
            _ => false,
        };
        if firing {
            let first = *first_shot.get_or_insert(now);
            let total = seconds(4.0);
            let owed = ((now - first) as f32 / total as f32 * 50.0).min(50.0);
            if owed - *drained >= 5.0 || owed >= 50.0 {
                self.gun.use_ammo(GunColor::Yellow, owed - *drained);
                *drained = owed;
            }
            let out_of_ammo = !self.gun.endless_ammo && self.gun.ammo[GunColor::Yellow as usize] < 1.0;
            if now - first >= total || out_of_ammo {
                go(SaucerPhase::Falling, phase, phase_time);
                shot.transv = Vec3::ZERO;
            } else if now - *last_volley >= 15 {
                *last_volley = now;
                let mut targets: Vec<Vec3> = Vec::new();
                if let Some(a) = reborrow(&mut actors) {
                    let mut near = a.actors_near(shot.trans, meters(35.0));
                    let len = near.len();
                    for i in 0..len.min(2) {
                        let j = i + (self.weapons.rng.unit() * (len - i) as f32) as usize;
                        near.swap(i, j.min(len - 1));
                        let c = near[i].1;
                        if clear_line(world, &mut self.weapons.cache, shot.trans, c) {
                            targets.push(c);
                        }
                    }
                }
                let rng = &mut self.weapons.rng;
                while targets.len() < 2 {
                    targets.push(
                        shot.trans
                            + Vec3::new(rng.range(-1.0, 1.0), rng.range(-0.85, -0.3), rng.range(-1.0, 1.0))
                                * meters(10.0),
                    );
                }
                for aim in targets {
                    let aim = aim
                        + Vec3::new(rng.range(-1.0, 1.0), rng.range(-1.0, 1.0), rng.range(-1.0, 1.0)) * 4096.0;
                    let flat = Vec3::new(aim.x - shot.trans.x, 0.0, aim.z - shot.trans.z).normalize_or(Vec3::Z);
                    let from = shot.trans + flat * 1228.8 - Vec3::Y * 819.2;
                    let dir = (aim - from).normalize_or(flat);
                    let mut sub = Projectile::yellow(from, dir, now);
                    sub.kind = GunKind::Yellow3;
                    self.projectiles.push(sub);
                }
            }
        }
        shot.tail = shot.trans;
        shot.state = ShotState::Saucer {
            phase: *phase,
            phase_time: *phase_time,
            first_shot: *first_shot,
            last_volley: *last_volley,
            last_probe: *last_probe,
            spin: *spin,
            drained: *drained,
        };
        true
    }

    /// An explosion: every creature within `radius` takes `damage` once,
    /// and the world at the center takes it too.
    fn explode(
        &mut self,
        kind: GunKind,
        center: Vec3,
        radius: f32,
        damage: f32,
        actors: Option<&mut dyn ActorWorld>,
    ) {
        if let Some(a) = actors
            && radius > 0.0
        {
            for (key, c) in a.actors_near(center, radius) {
                self.events.push(impact(c, c - center, damage, Some(key)));
            }
        }
        self.events.push(Event::Impact(ProjectileHit {
            pos: center,
            surface: center,
            normal: Vec3::Y,
            damage,
            actor: None,
            dir: -Vec3::Y,
        }));
        self.events.push(Event::Explosion {
            kind,
            center,
            radius,
        });
        self.weapons.effects.push(Effect::Blast {
            center,
            radius: radius.clamp(meters(1.5), meters(6.0)),
            start: self.time,
            duration: seconds(0.5),
            kind,
        });
    }

    /// The Peace Maker's shot lands: the nearest creature takes it, then
    /// tendrils reach the rest, farthest first, one every 0.1 s.
    fn peace_impact(&mut self, core: Vec3, dir: Vec3, actors: Option<&mut dyn ActorWorld>) {
        let now = self.time;
        let mut near = actors.map_or_else(Vec::new, |a| a.actors_near(core, meters(10.0)));
        near.sort_by(|a, b| a.1.distance(core).total_cmp(&b.1.distance(core)));
        near.truncate(16);
        self.events.push(Event::Explosion {
            kind: GunKind::Dark1,
            center: core,
            radius: meters(10.0),
        });
        self.events.push(Event::Impact(ProjectileHit {
            pos: core,
            surface: core,
            normal: -dir,
            damage: 16.0,
            actor: None,
            dir,
        }));
        self.weapons.effects.push(Effect::Blast {
            center: core,
            radius: meters(3.0),
            start: now,
            duration: seconds(0.6),
            kind: GunKind::Dark1,
        });
        let Some(&(first, at)) = near.first() else {
            return;
        };
        self.events.push(impact(at, dir, 16.0, Some(first)));
        near.remove(0);
        if !near.is_empty() {
            self.weapons.effects.push(Effect::Tendrils {
                origin: at,
                queue: near,
                arcs: Vec::new(),
                next: now,
                start: now,
            });
        }
    }

    /// One frame of an effect; false when it is gone.
    fn step_effect(
        &mut self,
        effect: &mut Effect,
        _world: &mut dyn CollideWorld,
        mut actors: Option<&mut dyn ActorWorld>,
    ) -> bool {
        let now = self.time;
        match effect {
            Effect::Rays { start, duration, .. } | Effect::Blast { start, duration, .. } => {
                now - *start < *duration
            }
            // The host ends their drawing with their clips; kept long enough.
            Effect::Left { start, .. } => now - *start < seconds(3.0),
            Effect::Tendrils {
                origin,
                queue,
                arcs,
                next,
                start,
            } => {
                if now >= *next
                    && let Some(target) = queue.pop()
                {
                    arcs.push((target.0, target.1, now, false));
                    *next = now + seconds(0.1);
                }
                for arc in arcs.iter_mut() {
                    if !arc.3 && now - arc.2 >= seconds(0.1) {
                        arc.3 = true;
                        self.events.push(Event::Impact(ProjectileHit {
                            pos: arc.1,
                            surface: arc.1,
                            normal: (*origin - arc.1).normalize_or(Vec3::Y),
                            damage: 16.0,
                            actor: Some(arc.0),
                            dir: (arc.1 - *origin).normalize_or(Vec3::Z),
                        }));
                    }
                }
                arcs.retain(|a| now - a.2 < seconds(0.35));
                (!queue.is_empty() || !arcs.is_empty()) && now - *start < seconds(10.0)
            }
            Effect::Gravity {
                center,
                start,
                caught,
            } => {
                let t = (now - *start) as f32 / TICKS_PER_SECOND as f32;
                if t <= 1.0
                    && let Some(a) = reborrow(&mut actors)
                {
                    for (key, _) in a.lift_near(*center, meters(30.0) * t) {
                        if !caught.contains(&key) {
                            caught.push(key);
                            let seconds = (self.weapons.rng.range(7.0, 9.0) - t).max(2.0);
                            self.events.push(Event::Lift { actor: key, seconds });
                        }
                    }
                }
                // (Its field lasts three seconds; its rings, drawn, about
                // seven and a half.)
                t < GRAVITY_RING_SECONDS
            }
            Effect::Nuke {
                center,
                start,
                killed,
                small,
            } => {
                let t = now - *start;
                if !*killed && t >= seconds(0.3) {
                    *killed = true;
                    self.weapons.flash = Some(now);
                    if let Some(a) = reborrow(&mut actors) {
                        for (key, c) in a.actors_near(*center, meters(300.0)) {
                            self.events.push(impact(c, c - *center, 32.0, Some(key)));
                        }
                    }
                    self.events.push(Event::Impact(ProjectileHit {
                        pos: *center,
                        surface: *center,
                        normal: Vec3::Y,
                        damage: 32.0,
                        actor: None,
                        dir: -Vec3::Y,
                    }));
                    self.events.push(Event::Explosion {
                        kind: GunKind::Dark3,
                        center: *center,
                        radius: meters(300.0),
                    });
                }
                t < if *small { seconds(8.0) } else { seconds(10.0) }
            }
        }
    }

    /// The beams of every shot and effect, for the host to draw.
    pub fn gun_beams(&self) -> Vec<Beam> {
        let now = self.time;
        let mut out = Vec::new();
        // With the game's particles the Blaster's and the Gyro Burster's
        // shots are theirs (`projectile-method-25`), not plain beams.
        let particles = self.particles_draw_shots();
        for p in &self.projectiles {
            if particles && matches!(p.kind, GunKind::Yellow1 | GunKind::Yellow3 | GunKind::Blue1) {
                continue;
            }
            let (rgba, width) = match p.kind {
                GunKind::Blue1 => (GunKind::Blue1.rgba(), 0.6),
                GunKind::Yellow3 => (GunKind::Yellow3.rgba(), 0.7),
                _ => ([1.0, 0.9, 0.35, 1.0], 1.0),
            };
            out.push(Beam {
                head: p.trans,
                tail: p.tail,
                rgba,
                width,
            });
        }
        let mut rng = Rng(0x9e37_79b9 ^ (now as u32).wrapping_mul(2_654_435_761));
        for s in &self.weapons.shots {
            let rgba = s.kind.rgba();
            match &s.state {
                ShotState::Saucer { phase, spin, .. } => {
                    let r = meters(0.7) * if *phase == SaucerPhase::Husk { 0.4 } else { 1.0 };
                    for k in 0..3 {
                        let a = spin + k as f32 * std::f32::consts::TAU / 3.0;
                        let arm = Vec3::new(a.cos(), 0.0, a.sin()) * r;
                        out.push(Beam {
                            head: s.trans + arm,
                            tail: s.trans - arm * 0.3,
                            rgba,
                            width: 1.4,
                        });
                    }
                }
                ShotState::Peace { .. } if particles => {}
                ShotState::Peace { charging, core, .. } => {
                    let grow = ((now - s.spawn) as f32 / 60.0).min(1.0);
                    ball(&mut out, if *charging { *core } else { s.trans }, meters(0.25 + 0.5 * grow), rgba, 1.6);
                    if !*charging {
                        out.push(Beam {
                            head: s.trans,
                            tail: s.tail,
                            rgba,
                            width: 1.8,
                        });
                    }
                }
                ShotState::Nuke { .. } if particles => {}
                ShotState::Nuke { .. } => {
                    ball(&mut out, s.trans, meters(0.35), [1.0, 0.5, 1.0, 1.0], 1.5);
                    out.push(Beam {
                        head: s.trans,
                        tail: s.tail,
                        rgba,
                        width: 1.2,
                    });
                }
                ShotState::Grenade { .. } => {
                    // Its glowing ball; its light trail is drawn above.
                    ball(&mut out, s.trans, meters(0.3), [1.0, 0.45, 0.15, 1.0], 1.2);
                    if !particles {
                        out.push(Beam {
                            head: s.trans,
                            tail: s.tail,
                            rgba,
                            width: 0.8,
                        });
                    }
                }
                _ => out.push(Beam {
                    head: s.trans,
                    tail: s.tail,
                    rgba,
                    width: 1.0,
                }),
            }
        }
        // The Beam Reflexor's light trails: a ribbon through where the shot
        // has been (0.7 m wide, `base-width`), narrowing and fading over its
        // half second (`width-curve`, `alpha-curve-1`), white at the head
        // going to orange down its length (`*curve-yellow2-shot-color*`),
        // and dashed along its last stretch (`alpha-curve-2`).
        for trail in &self.weapons.trails {
            let Some(&(_, head, _)) = trail.crumbs.last() else {
                continue;
            };
            if trail.kind == GunKind::Red3 {
                // The grenade's: half a meter wide, half bright, its glow
                // rising again every 1.5 m (`*curve-linear-up-red*` over
                // `alpha-repeat-dist`), red, fading over half a second.
                for w in trail.crumbs.windows(2) {
                    let ((a, _, _), (b, flown, at)) = (w[0], w[1]);
                    let age = ((now - at) as f32 / TRAIL_AGE as f32).clamp(0.0, 1.0);
                    let saw = ((head - flown) / meters(1.5)).fract();
                    let alpha = 0.5 * (1.0 - age) * (1.0 - saw);
                    if alpha > 0.02 {
                        out.push(Beam {
                            head: b,
                            tail: a,
                            rgba: [1.0, 0.3 + 0.3 * (1.0 - age), 0.1, alpha],
                            width: 2.4 * (1.0 - age).max(0.2),
                        });
                    }
                }
                continue;
            }
            for w in trail.crumbs.windows(2) {
                let ((a, _, _), (b, flown, at)) = (w[0], w[1]);
                let age = ((now - at) as f32 / TRAIL_AGE as f32).clamp(0.0, 1.0);
                let back = ((head - flown) / meters(70.0)).fract();
                let alpha = (1.0 - age / 0.9).max(0.0) * piecewise(&TRAIL_DASHES, back);
                if alpha <= 0.02 {
                    continue;
                }
                let fade = piecewise(&[(0.0, 1.0), (0.2, 0.8), (1.0, 0.5)], back);
                out.push(Beam {
                    head: b,
                    tail: a,
                    rgba: [1.0, 0.9 * fade, 0.45 * fade, alpha],
                    width: 3.0 * (1.0 - age).max(0.15),
                });
            }
        }
        // (The Arc Wielder's is lightning among the particles' sprites when
        // the game's particles are in.)
        if let Some(arc) = self.weapons.arc.as_ref().filter(|a| a.active && !particles) {
            for w in arc.pts.windows(2) {
                bolt(&mut out, &mut rng, w[0], w[1], [0.55, 0.85, 1.0, 1.0], 0.7);
            }
        }
        for e in &self.weapons.effects {
            match e {
                // The Scatter Gun draws no lines for its probes (`gun-red-shot`
                // shows its blast at the muzzle only): with the game's
                // particles none.
                Effect::Rays { kind: GunKind::Red1, .. } if particles => {}
                Effect::Rays {
                    rays,
                    start,
                    duration,
                    kind,
                } => {
                    let fade = 1.0 - (now - start) as f32 / (*duration).max(1) as f32;
                    let mut rgba = kind.rgba();
                    rgba[3] = fade;
                    for (a, b) in rays {
                        out.push(Beam {
                            head: *b,
                            tail: *a,
                            rgba,
                            width: 0.8,
                        });
                    }
                }
                Effect::Blast {
                    center,
                    radius,
                    start,
                    duration,
                    kind,
                } => {
                    // Blasts with their own groups in the game's particles
                    // (`group_for`) are those alone, not a ball of light over
                    // them too.
                    if particles && matches!(kind, GunKind::Red3 | GunKind::Dark1 | GunKind::Dark3 | GunKind::Yellow3) {
                        continue;
                    }
                    let t = (now - start) as f32 / (*duration).max(1) as f32;
                    let mut rgba = kind.rgba();
                    rgba[3] = 1.0 - t;
                    ball(&mut out, *center, radius * (0.3 + 0.7 * t), rgba, 2.0);
                }
                Effect::Tendrils { .. } if self.particles.as_ref().is_some_and(|p| p.library().has_lightning("lightning-dark-shot-attack-thick")) => {}
                Effect::Tendrils { origin, arcs, .. } => {
                    for (_, at, _, _) in arcs {
                        bolt(&mut out, &mut rng, *origin, *at, [0.85, 0.45, 1.0, 1.0], 0.8);
                    }
                }
                Effect::Left { .. } => {}
                Effect::Gravity { .. } if particles => {}
                Effect::Gravity { center, start, .. } => {
                    let t = (now - start) as f32 / TICKS_PER_SECOND as f32;
                    for k in 0..5 {
                        let local = t - k as f32 * 0.15;
                        if local <= 0.0 {
                            continue;
                        }
                        let r = meters(30.0) * (local / 1.0).min(1.0);
                        let fade = (1.0 - (t - 1.0).max(0.0) / 2.0).clamp(0.0, 1.0);
                        ring(
                            &mut out,
                            *center + Vec3::Y * meters(0.3 + 0.4 * k as f32),
                            r,
                            [0.7, 0.35, 1.0, fade],
                            4.0 - 0.6 * k as f32,
                            40,
                        );
                    }
                }
                Effect::Nuke { .. } if particles => {}
                Effect::Nuke {
                    center,
                    start,
                    small,
                    ..
                } => {
                    let t = (now - start) as f32 / TICKS_PER_SECOND as f32;
                    let fade = (1.0 - (t - 5.0).max(0.0) / 5.0).clamp(0.0, 1.0);
                    let shock = meters(if *small { 40.0 } else { 120.0 }) * (t / 1.5).min(1.0);
                    ring(&mut out, *center, shock, [1.0, 0.7, 1.0, (1.0 - t / 2.0).max(0.0)], 6.0, 48);
                    if !*small {
                        let top = *center + Vec3::Y * (meters(5.0) + meters(2.0) * t);
                        let grow = 1.7 * lerp(8.0, 12.0, (t / 5.0).min(1.0));
                        let rgba = [1.0, 0.55, 0.85, fade];
                        for k in 0..4 {
                            let y = top + Vec3::Y * meters(k as f32 * 1.2 - 2.0);
                            ring(&mut out, y, meters(grow * (1.0 - (k as f32 - 1.5).abs() * 0.2)), rgba, 8.0, 32);
                        }
                        for k in 0..6 {
                            let a = k as f32 / 6.0 * std::f32::consts::TAU;
                            let base = *center + Vec3::new(a.cos(), 0.0, a.sin()) * meters(3.0);
                            out.push(Beam {
                                head: top + Vec3::new(a.cos(), 0.0, a.sin()) * meters(2.0),
                                tail: base,
                                rgba,
                                width: 10.0,
                            });
                        }
                    }
                }
            }
        }
        out
    }
}

fn trail(shot: &mut Shot, max: f32) {
    let trail = shot.tail - shot.trans;
    if trail.length() > max {
        shot.tail = shot.trans + normalize(trail, max);
    }
}

fn wrap_pi(a: f32) -> f32 {
    (a + std::f32::consts::PI).rem_euclid(std::f32::consts::TAU) - std::f32::consts::PI
}

fn rotate_y_rad(v: Vec3, a: f32) -> Vec3 {
    Quat::from_rotation_y(a) * v
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_reflexor_trail_is_whole_near_the_shot_then_dashed() {
        assert_eq!(piecewise(&TRAIL_DASHES, 0.0), 0.0);
        assert!((piecewise(&TRAIL_DASHES, 0.05) - 0.5).abs() < 1e-3);
        assert_eq!(piecewise(&TRAIL_DASHES, 0.25), 1.0);
        assert_eq!(piecewise(&TRAIL_DASHES, 0.42), 0.0);
        assert_eq!(piecewise(&TRAIL_DASHES, 0.47), 1.0);
        assert_eq!(piecewise(&TRAIL_DASHES, 0.9), 0.0);
        assert_eq!(piecewise(&TRAIL_DASHES, 2.0), 0.0);
    }

    /// The Mass Inverter's rings grow as `gravity-ring` grows them: past
    /// 30 m in their first second, then slowly, a band that thins to half a
    /// meter, gone by seven and a half seconds; each 0.15 s after the last.
    #[test]
    fn the_mass_inverter_rings_grow_and_fade_as_the_games() {
        let (outer, inner) = gravity_ring(0, 1.0).unwrap();
        assert!((30.0..36.0).contains(&outer), "{outer}");
        assert!(outer - inner >= 0.5);
        assert!(gravity_ring(4, 0.5).is_none());
        let (late, _) = gravity_ring(0, 5.0).unwrap();
        assert!(late > outer && late < outer + 15.0, "{late}");
        assert!(gravity_ring(0, 6.5).is_some());
        assert!(gravity_ring(0, 7.2).is_none());
    }

    #[test]
    fn a_trail_keeps_how_far_its_shot_flew() {
        let mut weapons = Weapons::default();
        let id = weapons.new_trail(GunKind::Yellow2, Vec3::ZERO, 0);
        weapons.crumb(id, Vec3::new(3.0, 4.0, 0.0), 1);
        weapons.crumb(id, Vec3::new(3.0, 4.0, 12.0), 2);
        let flown: Vec<f32> = weapons.trails[0].crumbs.iter().map(|c| c.1).collect();
        assert_eq!(flown, [0.0, 5.0, 17.0]);
    }
}

/// How long the Mass Inverter's rings last, seconds.
const GRAVITY_RING_SECONDS: f32 = 8.0;

/// One of the Mass Inverter's rings (`gravity-ring`), `since` seconds after
/// the shot: the `k`th of five, spawned 0.15 s apart with its band 4 m wide
/// less 0.6 m for each before it. Its outer and inner radius (meters), none
/// once it has faded. As `gravity-ring-method-21` grows it: fast at first
/// (2.3 slowing to 0.4 a second over the field's second), then 0.4, and
/// from 1.1 s after the shot 0.5 slowing to 0.05; the band thins from its
/// width to half a meter as it grows, and goes after six seconds.
fn gravity_ring(k: usize, since: f32) -> Option<(f32, f32)> {
    let born = 0.15 * k as f32;
    let age = since - born;
    if age <= 0.0 {
        return None;
    }
    let width = 4.0 - 0.6 * k as f32;
    let total = 1.0;
    let dt: f32 = 1.0 / 60.0;
    let mut scale = 0.0f32;
    let mut t = 0.0;
    while t < age {
        let global = born + t;
        let rate = if global > 1.1 {
            0.5 + (0.05 - 0.5) * (global - 1.1).clamp(0.0, 1.0)
        } else if t / total <= 1.0 {
            2.3 + (0.4 - 2.3) * (t / total)
        } else {
            0.4
        };
        scale += rate * dt.min(age - t);
        t += dt;
    }
    let mut least = 0.5;
    if age > 6.0 * total {
        least = 0.5 * (1.0 - (age - 6.0 * total).clamp(0.0, 1.0));
        if least <= 0.0 {
            return None;
        }
    }
    let outer = 25.0 * scale;
    let band = (width * (1.0 - scale * scale)).max(least);
    Some((outer, (outer - band).max(0.0)))
}

impl crate::Jak {
    /// The Mass Inverter's rings as the game shows them: `gravity-ring`'s
    /// shadow cast straight down from 30 m over the shot (its own mesh is
    /// masked off), dark bands on the ground spreading out.
    pub(crate) fn gravity_ring_sprites(&self) -> Vec<crate::particles::Sprite> {
        self.weapons.effects.iter().filter_map(|e| match e {
            Effect::Gravity { center, start, .. } => Some(presentation::GravityVisual {
                center: *center, age: (self.time-start) as f32/TICKS_PER_SECOND as f32,
            }), _=>None,
        }).flat_map(|g| presentation::gravity_sprites(g.center,g.age)).collect()
    }

    /// The Arc Wielder's lightning this frame: its six bolts' strips (Jak
    /// units).
    pub fn arc_strips(&self) -> &[crate::lightning_bolt::Strip] {
        self.weapons.arc.as_ref().map_or(&[], |a| a.strips.as_slice())
    }

    /// All his lightning drawn as strips this frame: the Arc Wielder's rope
    /// and every `lightning-spec` bolt (Dark Jak's, the guns' shocks, the
    /// Peacemaker's tendrils), Jak units.
    pub fn lightning_strips(&self) -> Vec<crate::lightning_bolt::Strip> {
        let mut strips = self.arc_strips().to_vec();
        if let Some(particles) = self.particles.as_ref() {
            strips.extend(particles.lightning_strips());
        }
        strips
    }

    /// Its tracker when not fired this frame: R1 let go (or the gun gone,
    /// its ammunition out), `inactive`, its bolts fading out while it still
    /// follows the gun (`:trans` runs the rope until they are out); gone
    /// eight seconds on.
    fn arc_idle(&mut self, world: &mut dyn CollideWorld, actors: Option<&mut dyn ActorWorld>) {
        let now = self.time;
        let Some(arc) = self.weapons.arc.as_mut() else { return };
        if arc.active && now - arc.last_fired <= FRAME_TICKS {
            return;
        }
        if arc.active {
            // `:exit` of `active`: its lit bolts fade out.
            arc.active = false;
            arc.state_time = now;
            for bolt in &mut arc.bolts {
                if bolt.fade_mode() == 0 {
                    bolt.set_fade(2, now);
                }
            }
        }
        if now - arc.state_time >= seconds(8.0) {
            self.weapons.arc = None;
            return;
        }
        let lit = arc.bolts.first().is_some_and(|b| b.fade_mode() != 3);
        if lit {
            self.arc_rope(world, actors);
        }
        if let Some(mut arc) = self.weapons.arc.take() {
            self.arc_bolts_frame(&mut arc);
            self.weapons.arc = Some(arc);
        }
    }

    #[cfg(test)]
    pub(crate) fn arc_for_test(&mut self, pts: Vec<Vec3>) {
        let mut arc = self.new_arc_tracker(self.time);
        arc.active = true;
        arc.attached = vec![false; pts.len()];
        arc.pts = pts;
        self.weapons.arc = Some(arc);
        let mut arc = self.weapons.arc.take().unwrap();
        let mut world = crate::collide::EmptyWorld;
        self.arc_setup_draw(&mut arc, &mut world);
        self.weapons.arc = Some(arc);
    }

    /// The Mass Inverter's fields up now: each one's centre and radius (game
    /// units), growing to 30 m over its first second, for its three.
    pub fn gravity_fields(&self) -> Vec<(Vec3, f32)> {
        self.weapons
            .effects
            .iter()
            .filter_map(|effect| match effect {
                Effect::Gravity { center, start, .. } => {
                    let t = (self.time - start) as f32 / TICKS_PER_SECOND as f32;
                    (t < 3.0).then(|| (*center, meters(30.0) * t.clamp(0.0, 1.0)))
                }
                _ => None,
            })
            .collect()
    }
}

#[cfg(test)]
mod arc_draw_tests {
    use crate::{Jak, glam::Vec3, math::meters};

    /// The Arc Wielder's electricity fades out along its rope (its first
    /// alpha down over the rope's length and 4 m more), gone by its end:
    /// six bolts of two strips each, the thick ones `pal-lightning`.
    #[test]
    fn the_arc_dies_away_where_it_reaches() {
        let mut jak = Jak::new(Vec3::ZERO, 0.0);
        let pts: Vec<Vec3> = (0..12).map(|i| Vec3::Z * meters(6.0) * i as f32).collect();
        jak.arc_for_test(pts);
        let strips = jak.arc_strips();
        assert_eq!(strips.len(), 12);
        assert_eq!(strips.iter().filter(|s| s.texture == "pal-lightning").count(), 4);
        let verts: Vec<_> = strips.iter().flat_map(|s| s.verts.iter()).collect();
        assert!(!verts.is_empty());
        let reach = verts.iter().map(|v| v.pos.z).fold(0.0f32, f32::max);
        assert!(reach <= meters(66.0) + meters(4.0), "within its rope: {} m", reach / meters(1.0));
        let near = verts.iter().filter(|v| v.pos.z < meters(6.0)).map(|v| v.rgba[3]).fold(0.0, f32::max);
        let far = verts.iter().filter(|v| v.pos.z > meters(60.0)).map(|v| v.rgba[3]).fold(0.0, f32::max);
        assert!(near > 110.0, "bright at the muzzle: {near}");
        assert!(far < 20.0, "almost gone at its end: {far}");
    }

    /// The arc ending on the world strikes it there, once every 0.4 s, as
    /// it strikes a creature (19/10: the Arc Wielder left Minecraft's
    /// blocks whole).
    #[test]
    fn the_arc_strikes_the_wall_it_ends_on() {
        use crate::collide::{Pat, Tri, TriangleGrid};
        use crate::{Event, FRAME_TICKS};
        let mut jak = Jak::new(Vec3::ZERO, 0.0);
        jak.gun.fire_point = Vec3::Y * meters(1.0);
        jak.gun.fire_dir = Vec3::Z;
        let (z, s, h) = (meters(5.0), meters(50.0), meters(20.0));
        let wall = vec![
            Tri { v: [Vec3::new(-s, 0.0, z), Vec3::new(-s, h, z), Vec3::new(s, h, z)], pat: Pat::default() },
            Tri { v: [Vec3::new(-s, 0.0, z), Vec3::new(s, h, z), Vec3::new(s, 0.0, z)], pat: Pat::default() },
        ];
        let mut world = TriangleGrid::new(wall, meters(4.0));
        let mut hits = Vec::new();
        for _ in 0..30 {
            jak.arc_frame(&mut world, None);
            for e in jak.events.drain(..) {
                if let Event::WorldHit { surface, .. } = e {
                    hits.push(surface);
                }
            }
            jak.time += FRAME_TICKS;
        }
        assert!((1..=3).contains(&hits.len()), "{} blows", hits.len());
        assert!(hits.iter().all(|p| (p.z - meters(5.0)).abs() < meters(1.0)), "on the wall: {hits:?}");
    }
}

/// A shot drawn as its model (Jak 3's `defskelgroup`s for its guns' shots):
/// the model, where it is and how it is turned (Jak units).
#[derive(Clone, Debug, PartialEq)]
pub struct ShotModel {
    /// Stable within a simulation session, independent of vector removal/order.
    pub id: u64,
    pub model: &'static str,
    pub pos: Vec3,
    pub rot: Quat,
    /// Its scale on each of its axes (1 its own).
    pub scale: Vec3,
    /// What it plays.
    pub anim: ShotAnim,
    /// How much of it shows (1 all).
    pub alpha: f32,
}

/// What a shot's model plays, by its clips' names: its first frame; a clip
/// once from the start, `age` seconds in (gone at its end); a burst then
/// its fade at five times its speed (`red-3-sphere`; gone at its end); a
/// clip at a fraction of its length.
#[derive(Clone, Debug, PartialEq)]
pub enum ShotAnim {
    Rest,
    Once { clip: &'static str, age: f32 },
    BurstFade { burst: &'static str, fade: &'static str, age: f32 },
    At { clip: &'static str, fraction: f32 },
}

impl crate::Jak {
    /// The shots in flight that are models (17/10: only their particles
    /// were drawn): the Gyro Burster's saucer (`skel-gun-yellow-3-saucer`,
    /// spinning), the Plasmite RPG's grenade (`skel-gun-red-3-grenade`,
    /// along its flight) and the Super Nova's missile (`skel-gun-dark-3-
    /// nuke`, along its heading).
    /// The latest Super Nova going off (gun-dark-3-nuke's `impact` or
    /// `impact-small` state): where, its age in seconds, and whether small.
    /// The host draws its screen from it: `do-white-screen-effect`,
    /// `do-blur-effect`, the music's silence and the camera's smush.
    pub fn nuke_screen(&self) -> Option<(Vec3, f32, bool)> {
        self.weapons.effects.iter().rev().find_map(|e| match e {
            Effect::Nuke { center, start, small, .. } => {
                Some((*center, (self.time - start) as f32 / TICKS_PER_SECOND as f32, *small))
            }
            _ => None,
        })
    }

    pub fn shot_models(&self) -> Vec<ShotModel> {
        let along = |v: Vec3| {
            let d = v.try_normalize().unwrap_or(Vec3::Z);
            Quat::from_rotation_arc(Vec3::Z, d)
        };
        let plain = |id, model, pos, rot| ShotModel { id, model, pos, rot, scale: Vec3::ONE, anim: ShotAnim::Rest, alpha: 1.0 };
        let mut out: Vec<ShotModel> = self
            .weapons
            .shots
            .iter()
            .filter_map(|shot| match &shot.state {
                ShotState::Saucer { phase, spin, .. } if *phase != SaucerPhase::Husk => {
                    Some(plain((shot.spawn as u64)<<8 | shot.kind as u64,"gun-saucer", shot.trans, Quat::from_rotation_y(*spin)))
                }
                ShotState::Grenade { .. } => Some(ShotModel { scale: Vec3::splat(2.5), ..plain((shot.spawn as u64)<<8 | shot.kind as u64,"gun-grenade", shot.trans, along(shot.transv)) }),
                ShotState::Nuke { dir, .. } => Some(plain((shot.spawn as u64)<<8 | shot.kind as u64,"gun-nuke", shot.trans, along(*dir))),
                _ => None,
            })
            .collect();
        let now = self.time;
        let cam = &self.camera;
        // Turned to face the camera (the camera's own axes), and only about
        // the up axis.
        let facing = Quat::from_mat3(&glam::Mat3::from_cols(cam.r, cam.u, cam.f)).normalize();
        let back = Vec3::new(-cam.f.x, 0.0, -cam.f.z).try_normalize().unwrap_or(Vec3::Z);
        let upright_facing = Quat::from_rotation_y(back.x.atan2(back.z));
        for effect in &self.weapons.effects {
            match effect {
                Effect::Left { which: LeftModel::Cone, pos, dir, start } => {
                    // `forward-up->quaternion` along the shot.
                    let age = (now - start) as f32 / TICKS_PER_SECOND as f32;
                    out.push(ShotModel {
                        anim: ShotAnim::Once { clip: "gun-red-cone-idle", age },
                        ..plain((1u64<<63)|((*start as u64)<<8)|0,"gun-red-cone", *pos, along(*dir))
                    });
                }
                Effect::Left { which: LeftModel::RedSphere, pos, start, .. } => {
                    // Its scale from 1 up 24 a second, the camera's turn.
                    let age = (now - start) as f32 / TICKS_PER_SECOND as f32;
                    out.push(ShotModel {
                        scale: Vec3::splat(1.0 + 24.0 * age),
                        anim: ShotAnim::BurstFade { burst: "gun-red-sphere-burst", fade: "gun-red-sphere-fade", age },
                        ..plain((1u64<<63)|((*start as u64)<<8)|1,"gun-red-sphere", *pos, facing)
                    });
                }
                Effect::Nuke { center, start, small: false, .. } => {
                    // `gun-dark-3-sphere` on the mushroom's top: 5 m up,
                    // rising 2 m a second (`*gun-dark-3-mushroom-speed*`), its
                    // size along the mushroom's curves over 5 s (8 to 12 m
                    // across, 4 to 5 m high), 1.7 times that; after 5 s 1.3
                    // times more and fading out over five seconds more.
                    let t = (now - start) as f32 / TICKS_PER_SECOND as f32;
                    let k = (t / 5.0).min(1.0);
                    let (mut x, mut y) = (lerp(8.0, 12.0, k), lerp(4.0, 5.0, k));
                    let mut alpha = 1.0;
                    if t > 5.0 {
                        let late = ((t - 5.0) / 5.0).min(1.0);
                        let grow = lerp(1.0, 1.3, late * late);
                        x *= grow;
                        y *= grow;
                        alpha = lerp(1.0, 0.0, late);
                    }
                    let alpha = alpha * alpha;
                    out.push(ShotModel {
                        scale: Vec3::new(x, y, 1.0) * 1.7,
                        anim: ShotAnim::At { clip: "gun-nuke-sphere-fade", fraction: 1.0 - alpha },
                        alpha,
                        ..plain((1u64<<63)|((*start as u64)<<8)|2,"gun-nuke-sphere", *center + Vec3::Y * (meters(5.0) + meters(2.0) * t), upright_facing)
                    });
                }
                _ => {}
            }
        }
        out
    }
}

#[cfg(test)]
mod left_model_tests {
    use super::*;

    /// The Plasmite RPG's sphere grows 24 a second; the Super Nova's sphere
    /// rides its mushroom up and is all but gone by its tenth second; the
    /// Scatter Gun's cone points along its shot.
    #[test]
    fn the_shots_models_grow_rise_and_fade_as_in_the_game() {
        let mut jak = crate::Jak::new(Vec3::ZERO, 0.0);
        jak.weapons.effects.push(Effect::Left { which: LeftModel::RedSphere, pos: Vec3::ZERO, dir: Vec3::Z, start: 0 });
        jak.weapons.effects.push(Effect::Left { which: LeftModel::Cone, pos: Vec3::ZERO, dir: Vec3::X, start: 0 });
        jak.weapons.effects.push(Effect::Nuke { center: Vec3::ZERO, start: 0, killed: true, small: false });
        jak.time = TICKS_PER_SECOND / 2;
        let models = jak.shot_models();
        let sphere = models.iter().find(|m| m.model == "gun-red-sphere").expect("sphere");
        assert!((sphere.scale.x - 13.0).abs() < 0.01);
        let cone = models.iter().find(|m| m.model == "gun-red-cone").expect("cone");
        assert!((cone.rot * Vec3::Z - Vec3::X).length() < 1e-4);
        let nuke = models.iter().find(|m| m.model == "gun-nuke-sphere").expect("nuke");
        assert!((nuke.pos.y - meters(6.0)).abs() < 1.0 && nuke.alpha == 1.0);
        jak.time = TICKS_PER_SECOND * 99 / 10;
        let late = jak.shot_models();
        let nuke = late.iter().find(|m| m.model == "gun-nuke-sphere").expect("nuke");
        assert!(nuke.alpha < 0.01, "{}", nuke.alpha);
        assert!(nuke.scale.x > 12.0 * 1.7);
    }
}
