//! Jak 3's gameplay, rebuilt in Rust: Jak on foot, the JET-Board and the
//! Blaster, with their own collision against whatever world the host hands
//! over.
//!
//! The simulation keeps Jak's own units (4096 per meter, y up, 300 ticks a
//! second) and runs at the 60 Hz the game's movement was tuned at; the host
//! converts at the boundary and feeds one [`Jak::step`] per frame.
pub mod align;
pub mod anim;
pub mod attack;
pub mod board;
pub mod board_anim;
mod board_code;
pub mod collide;
pub mod control;
mod darkjak;
pub mod dark_shock;
pub mod eco_blast;
pub mod lightning_bolt;
pub mod eco_power;
pub mod edge_grab;
pub mod flut_swim;
pub mod flut;
mod foot_code;
pub mod flight;
pub mod grind;
pub mod gun;
pub mod hellcat;
pub mod hurt;
pub mod lightjak;
pub mod math;
pub mod pad;
pub mod particles;
pub mod projectile;
pub mod reframe;
pub mod surface;
pub mod step;
pub mod ladder;
pub mod swim;
pub mod target;
pub mod weapons;
pub mod zoom;

use std::sync::Arc;

pub use glam;
use glam::{Quat, Vec3};

pub use anim::{AlignTrack, Anim, Anims, Channel, Push};
pub use attack::{Attack, Danger};
pub use board::BoardInfo;
pub use board_anim::BoardAnim;
pub use collide::{
    CollideCache, CollideWorld, EmptyWorld, Pat, PatMaterial, PatMode, Tri, TriangleGrid,
};
pub use control::Control;
pub use gun::Gun;
pub use hurt::{HurtFx, HurtMode};
pub use weapons::ShotModel;
pub use lightjak::LightJak;
pub use math::{Basis, FRAME_TICKS, METER, SECONDS_PER_FRAME, TICKS_PER_SECOND};
pub use pad::{Pad, PadInput, button};
pub use projectile::{ActorWorld, Projectile, ProjectileHit};
pub use target::{FootInfo, StateHook};
pub use weapons::{Beam, GunColor, GunKind, Weapons};

/// What a high jump leaves from.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HighJump {
    Plain,
    /// Ducking with the stick let go.
    Duck,
    /// Straight after landing the roll's flip.
    Flip,
    /// Bounced off a dive's landing, standing still or running on.
    Flop,
    FlopForward,
}

/// What a spin in the air came out of.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AirFrom {
    Jump,
    Flop,
    Uppercut,
}

/// What Jak is doing. On-foot states first, board states after.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum State {
    Ladder,
    Stance,
    Walk,
    /// Reversing the stick at a run: the skid round.
    TurnAround,
    /// Original accepted eco pickup reaction (`target-eco-powerup`).
    EcoPowerup,
    PilotGetOff { bike: bool },
    /// Off the ground against a slope too steep to stand on.
    SlideDown,
    /// Ducking; `keep_time` carries the time from the duck state before.
    DuckStance {
        keep_time: bool,
    },
    DuckWalk {
        keep_time: bool,
    },
    Jump {
        min: f32,
        max: f32,
    },
    DoubleJump {
        min: f32,
        max: f32,
    },
    HighJump {
        min: f32,
        max: f32,
        kind: HighJump,
    },
    /// The crouch before a high jump from ducking or a dive's landing.
    DuckHighJump {
        min: f32,
        max: f32,
        kind: HighJump,
    },
    DuckHighJumpJump {
        min: f32,
        max: f32,
        kind: HighJump,
    },
    Falling {
        uppercut: bool,
    },
    /// Hanging from a ledge ([`edge_grab`]).
    EdgeGrab,
    /// Climbing up from it.
    EdgeGrabJump,
    /// Letting go of it.
    EdgeGrabOff,
    HitGround {
        stuck: bool,
    },
    /// Landing from a fall higher than Jak takes unhurt; `height` is the
    /// fall.
    HitGroundHard {
        height: f32,
    },
    /// The spin kick.
    Attack,
    /// The punch.
    RunningAttack,
    /// The spin kick in the air.
    AttackAir {
        from: AirFrom,
    },
    AttackUppercut {
        min: f32,
        max: f32,
    },
    AttackUppercutJump {
        min: f32,
        max: f32,
    },
    /// The dive.
    Flop {
        forward: bool,
    },
    FlopHitGround {
        stuck: bool,
    },
    Roll,
    /// The flip out of a roll.
    RollFlip {
        height: f32,
        dist: f32,
    },
    /// Holding the power button, ready to change.
    PowerJakGetOn,
    /// Changing into Light Jak; `swoop` brings the flight.
    LightJakGetOn {
        swoop: bool,
    },
    /// Changing back.
    LightJakGetOff,
    /// `target-lightjak-regen`: the heal, Jak lifted and slowed, its light
    /// from the sky, a point of health each half second.
    LightJakRegen,
    /// A flap of Light Jak's wings: the first of a flight, or another
    /// `held` seconds after the last.
    LightJakSwoop {
        first: bool,
        held: f32,
    },
    /// Falling between flaps.
    LightJakSwoopFalling,
    BoardGetOn,
    BoardStance,
    BoardGrind,
    BoardDuckStance,
    BoardJump {
        min: f32,
        max: f32,
        duck: bool,
    },
    BoardFalling,
    BoardHitGround,
    BoardTurnTo {
        dir: Vec3,
        duration: i64,
    },
    BoardFlip,
    /// L1 with the stick forward or back: a grab or a nose flip.
    BoardTricky,
    /// L1 with the stick across: a kick flip or a board spin.
    BoardTrickx,
    /// L2: a held grab.
    BoardHold,
    BoardJumpKick,
    BoardWallKick {
        dir: Vec3,
        speed: f32,
    },
    BoardGetOff,
    /// Climbing onto a Flut Flut ([`flut`]).
    FlutGetOn,
    FlutStance,
    FlutWalk,
    FlutJump {
        min: f32,
        max: f32,
    },
    /// Its flap in the air.
    FlutDoubleJump {
        min: f32,
        max: f32,
    },
    FlutHitGround,
    FlutFalling,
    FlutRunningAttack,
    FlutAirAttack,
    FlutAirAttackHitGround,
    /// Struck, thrown back away from `dir`'s opposite.
    FlutHit {
        dir: Vec3,
    },
    /// Down to the ground to get off.
    FlutGetOff,
    /// Jumping down off its back.
    FlutGetOffJump,
}

impl State {
    pub fn is_board(&self) -> bool {
        self.kind() != board::StateKind::OnFoot
    }

    pub(crate) fn kind(&self) -> board::StateKind {
        use board::StateKind as K;
        match self {
            State::BoardGetOn => K::BoardGetOn,
            State::BoardStance => K::BoardStance,
            State::BoardGrind => K::BoardGrind,
            State::BoardDuckStance => K::BoardDuckStance,
            State::BoardJump { .. } => K::BoardJump,
            State::BoardFalling => K::BoardFalling,
            State::BoardHitGround => K::BoardHitGround,
            State::BoardTurnTo { .. } => K::BoardTurnTo,
            State::BoardFlip => K::BoardFlip,
            State::BoardTricky => K::BoardTricky,
            State::BoardTrickx => K::BoardTrickx,
            State::BoardHold => K::BoardHold,
            State::BoardJumpKick => K::BoardJumpKick,
            State::BoardWallKick { .. } => K::BoardWallKick,
            State::BoardGetOff => K::BoardGetOff,
            _ => K::OnFoot,
        }
    }

    pub fn name(&self) -> &'static str {
        match self {State::Ladder=>"target-ladder",
            State::Stance => "stance",
            State::Walk => "walk",
            State::TurnAround => "turn-around",
            State::EcoPowerup => "eco-powerup",
            State::PilotGetOff { .. } => "pilot-get-off",
            State::SlideDown => "slide-down",
            State::DuckStance { .. } => "duck-stance",
            State::DuckWalk { .. } => "duck-walk",
            State::Jump { .. } => "jump",
            State::DoubleJump { .. } => "double-jump",
            State::HighJump { .. } => "high-jump",
            State::DuckHighJump { .. } => "duck-high-jump",
            State::DuckHighJumpJump { .. } => "duck-high-jump-jump",
            State::Falling { .. } => "falling",
            State::EdgeGrab => "edge-grab",
            State::EdgeGrabJump => "edge-grab-jump",
            State::EdgeGrabOff => "edge-grab-off",
            State::HitGround { .. } => "hit-ground",
            State::HitGroundHard { .. } => "hit-ground-hard",
            State::Attack => "attack",
            State::RunningAttack => "running-attack",
            State::AttackAir { .. } => "attack-air",
            State::AttackUppercut { .. } => "attack-uppercut",
            State::AttackUppercutJump { .. } => "attack-uppercut-jump",
            State::Flop { .. } => "flop",
            State::FlopHitGround { .. } => "flop-hit-ground",
            State::Roll => "roll",
            State::RollFlip { .. } => "roll-flip",
            State::PowerJakGetOn => "powerjak-get-on",
            State::LightJakGetOn { .. } => "lightjak-get-on",
            State::LightJakGetOff => "lightjak-get-off",
            State::LightJakRegen => "lightjak-regen",
            State::LightJakSwoop { first: true, .. } => "lightjak-swoop",
            State::LightJakSwoop { first: false, .. } => "lightjak-swoop-again",
            State::LightJakSwoopFalling => "lightjak-swoop-falling",
            State::BoardGetOn => "board-get-on",
            State::BoardStance => "board-stance",
            State::BoardGrind => "board-grind",
            State::BoardDuckStance => "board-duck-stance",
            State::BoardJump { .. } => "board-jump",
            State::BoardFalling => "board-falling",
            State::BoardHitGround => "board-hit-ground",
            State::BoardTurnTo { .. } => "board-turn-to",
            State::BoardFlip => "board-flip",
            State::BoardTricky => "board-tricky",
            State::BoardTrickx => "board-trickx",
            State::BoardHold => "board-hold",
            State::BoardJumpKick => "board-jump-kick",
            State::BoardWallKick { .. } => "board-wall-kick",
            State::BoardGetOff => "board-get-off",
            State::FlutGetOn => "target-flut-get-on",
            State::FlutStance => "target-flut-stance",
            State::FlutWalk => "target-flut-walk",
            State::FlutJump { .. } => "target-flut-jump",
            State::FlutDoubleJump { .. } => "target-flut-double-jump",
            State::FlutHitGround => "target-flut-hit-ground",
            State::FlutFalling => "target-flut-falling",
            State::FlutRunningAttack => "target-flut-running-attack",
            State::FlutAirAttack => "target-flut-air-attack",
            State::FlutAirAttackHitGround => "target-flut-air-attack-hit-ground",
            State::FlutHit { .. } => "target-flut-hit",
            State::FlutGetOff => "target-flut-get-off",
            State::FlutGetOffJump => "target-flut-get-off-jump",
        }
    }
}

/// Board tricks, as the combo counts them.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Trick {
    Spin,
    Boost,
    Flip,
    Jump,
    DuckJump,
    QuickJump,
    Nosegrab,
    Noseflip,
    Kickspin,
    Kickflip,
    BoardSpin,
    Method,
    MethodCross,
    Backgrab,
    Airwalk,
}

/// Things the host presents: sounds, effects, attacks on the world.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Event {
    BoardOn,
    BoardOff,
    BoardJump,
    BoardLaunch,
    BoardFlip,
    BoardBoost,
    BoardGlance,
    BoardBounce {
        pitch: f32,
    },
    BoardZap {
        center: Vec3,
        radius: f32,
    },
    Trick {
        trick: Trick,
        points: f32,
    },
    Jump,
    Land,
    /// One completed ground pound, at the feet on contact with the floor.
    /// Independent of how many attack spheres also strike world geometry.
    GroundPound { pos: Vec3 },
    /// A hard landing: the health it takes, in Jak's units.
    HardLanding {
        health: f32,
    },
    /// The Blaster fired: muzzle and direction.
    Fire {
        from: Vec3,
        dir: Vec3,
    },
    /// A shot stopped: where, the surface it struck and the damage it does.
    Impact(ProjectileHit),
    /// Jak 1's yellow eco shot: fired, burst or faded (its blow on a
    /// creature comes as a [`Event::Strike`]).
    EcoBlast(eco_blast::BlastEvent),
    /// A blow landed: on an actor, or on the world when `actor` is none;
    /// where, which way and the hit points it takes.
    Strike {
        actor: Option<u64>,
        pos: Vec3,
        dir: Vec3,
        damage: f32,
    },
    /// The punch's fist met a wall and Jak bounced back.
    PunchWall,
    /// Light Jak began, or ended.
    LightJak {
        on: bool,
    },
    DarkJak {
        on: bool,
    },
    /// A beat of Light Jak's wings.
    Flap,
    /// He went into the water (speed: how fast he fell in, Jak units a
    /// second) or jumped out of it (a negative speed): where on the surface.
    Splash {
        pos: Vec3,
        speed: f32,
    },
    /// A gun mode was picked on the D-pad.
    GunSelect {
        kind: GunKind,
    },
    /// A gun other than the Blaster fired (the Blaster sends `Fire`).
    GunFire {
        kind: GunKind,
        from: Vec3,
    },
    /// A hitscan shot's whole path, once it landed: from the muzzle to where
    /// it stopped.
    Trace {
        kind: GunKind,
        from: Vec3,
        to: Vec3,
    },
    /// A shot bounced or ricocheted.
    GunBounce {
        kind: GunKind,
        pos: Vec3,
    },
    /// A shot struck the world without stopping (the Beam Reflexor's
    /// bounce off a wall): a blow to what it met there, for a host whose
    /// world breaks (a Minecraft block), with no impact of its own.
    WorldHit {
        surface: Vec3,
        normal: Vec3,
        damage: f32,
    },
    /// A shot burst: a grenade, a wave, a Peace Maker, a Super Nova.
    Explosion {
        kind: GunKind,
        center: Vec3,
        radius: f32,
    },
    /// The edit blaster fired: place the block in hand where the camera's
    /// line `dir` from `from` first meets the world.
    Build {
        from: Vec3,
        dir: Vec3,
    },
    /// The Mass Inverter caught a creature: it floats for `seconds`, then
    /// falls.
    Lift {
        actor: u64,
        seconds: f32,
    },
    /// A sound by its game's name (Jak 1's as `jak1:` and its name), at
    /// Jak: `sound-play`'s volume (1 full) and pitch (octaves).
    Sound {
        name: &'static str,
        volume: f32,
        pitch: f32,
    },
    /// A sound by its game's name where it is played (Jak units): a
    /// lightning tracker's at its end (`sound-play-by-spec`).
    SoundAt {
        name: &'static str,
        pos: Vec3,
    },
    /// The Flut Flut's air attack met the ground here: Jak 1's
    /// `group-flut-attack-strike-ground`.
    FlutSlam {
        pos: Vec3,
    },
    /// He was hurt ([`hurt`]): by what, where it struck him, and how long
    /// the pad buzzes (seconds).
    Hurt {
        mode: HurtMode,
        pos: Vec3,
        buzz: f32,
    },
}

/// Where a state's code is: the step it resumes at and the locals it keeps
/// from one frame to the next.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Code {
    pub pc: u16,
    pub count: i32,
    /// The stick as the state read it when it began.
    pub x: f32,
    pub z: f32,
    pub rate: f32,
    pub flag: bool,
    /// Times the state keeps: when a button went down, when a wall was met.
    pub t: i64,
    pub t2: i64,
    /// Reached this frame, without a suspend since.
    pub(crate) arrived: bool,
    /// The state leaves without its exit running.
    pub(crate) no_exit: bool,
}

/// What changed Jak's state last, and the jump it last set off, for the
/// debug readout and the parity tests.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Trace {
    pub prev: &'static str,
    /// What made the change: the state's per-frame checks ("trans"), its
    /// own sequence ("code"), or the movement ("post").
    pub why: &'static str,
    pub changed_at: i64,
    /// The upward speed the last jump or trick set, units a second.
    pub impulse: f32,
    pub impulse_at: i64,
    /// The highest point since last on the ground.
    pub peak: f32,
}

/// Jak, his board and his gun.
pub struct Jak {
    pub ladder: Option<ladder::Ride>,
    pub(crate) ladder_after: i64,
    /// At a vehicle's controls (or on a ride): Dark Jak and Light Jak are
    /// out of reach, as in Jak 3's vehicles (Adrii, 16/10).
    pub powers_blocked: bool,
    pub control: Control,
    pub board: BoardInfo,
    pub gun: Gun,
    pub pad: Pad,
    pub state: State,
    pub state_time: i64,
    /// The game clock, in ticks.
    pub time: i64,
    /// The camera's frame: the stick is read through it.
    pub camera: Basis,
    pub projectiles: Vec<Projectile>,
    /// Jak 1's yellow eco shots in flight, and the last one's number.
    pub eco_blasts: Vec<eco_blast::EcoBlast>,
    eco_blast_count: u32,
    /// The other guns' shots and blasts.
    pub weapons: Weapons,
    /// The guns' sparticles, once the host gives the game's definitions
    /// ([`Jak::set_particles`]).
    pub particles: Option<particles::Particles>,
    pub(crate) events: Vec<Event>,
    pub(crate) cache: CollideCache,
    pub(crate) probe_cache: CollideCache,
    pub(crate) pending: Option<State>,
    /// A world pad detected after movement starts its jump inside the next
    /// movement tick, before that tick's state checks and collision sweep.
    pub(crate) pad_bounce: Option<f32>,
    /// The animation the state is playing, and the timing it plays by.
    pub chan: Channel,
    pub push: Push,
    pub anims: Arc<Anims>,
    pub code: Code,
    pub trace: Trace,
    /// The moves on foot: their timers, windows and the punch's table.
    pub foot: FootInfo,
    /// The attack under way, and what it struck.
    pub attack: Attack,
    /// The align joint's motion the animation carries.
    pub align: align::Align,
    /// Jak's joints in his own frame as last posed (units), from the host
    /// when his model is there; the blows that strike from a hand use them.
    pub joints: Vec<Vec3>,
    /// Between the bases of his index fingers in his own frame as last
    /// posed (units, at his own size), from the host: what hangs on a
    /// ledge ([`edge_grab`]).
    pub hold: Option<Vec3>,
    /// The top of his head in his own frame as last posed (units, at his
    /// own size), from the host: what stays out of the water as he swims.
    pub crown: Option<Vec3>,
    /// Wading (`target-wade-walk`): how deep the water over his feet is,
    /// when it is enough to wade and not enough to swim ([`swim`]).
    pub wade: Option<f32>,
    /// Aiming by hand: the heading (radians about y, 0 along +z) his
    /// upper body turns to while his legs follow the stick.
    pub aim_yaw: Option<f32>,
    /// The host's water: the height of its surface over Jak (Jak units),
    /// none where there is no water.
    pub water: Option<f32>,
    /// In the water ([`swim`]).
    pub swim: Option<swim::Swim>,
    /// How far his body leans from upright while he swims under water
    /// (radians, head forward): level as he swims along, half down as he
    /// dives ahead.
    pub swim_tilt: f32,
    /// Until when he is being thrown (by lava): the stick does not steer
    /// him meanwhile (ticks).
    hurled_until: i64,
    /// Until when blows do not touch him after one did
    /// (`hit-invulnerable-timeout`, ticks).
    invulnerable_until: i64,
    /// Which of the two front blows he took last (`target-hit-setup-anim`
    /// rolls between them).
    hit_alt: bool,
    /// The ledge he hangs from ([`edge_grab`]).
    pub ledge: Option<edge_grab::Ledge>,
    /// That ledge is a vehicle's edge (the host says): climbing up from it
    /// jumps 2 m higher (`*edge-grab-info*`'s `pilot-edge-grab?`).
    pub pilot_edge_grab: bool,
    /// Host survival effects, applied to Jak as well as the soldier.
    pub movement_scale: f32,
    /// Jak 1's eco he carries (blue, yellow; [`eco_power`]).
    pub eco_power: Option<eco_power::EcoPower>,
    /// His powerup reaction is light eco's (`target-eco-powerup` with the
    /// light pickup: `te2 te5`, drawn untextured under the rim lights).
    pub powerup_light: bool,
    /// Jump boost's launch speed against Minecraft's own (1 without):
    /// `(0.42 + getJumpBoostPower) / 0.42` scales his take-off speed.
    pub jump_scale: f32,
    pub fire_resistant: bool,
    pub slow_falling:bool,
    /// How high over the ground under it a ledge must be for him to catch
    /// it (the host's: in Minecraft two blocks).
    pub edge_grab_min_height: f32,
    /// The board climbs steps up to this high (Jak units) smoothly, rising
    /// to meet them, instead of the game's kick up each (the host's: in
    /// Minecraft a block, so a hill of one-block steps rides like a slope);
    /// none keeps the game's own.
    pub board_smooth_step: Option<f32>,
    /// The Flut Flut walks up steps this high (Jak units) with a hop as it
    /// reaches them, as a Minecraft horse steps up a block (the host's: a
    /// block; 17/10: ridden, it stood against one-block steps); none keeps
    /// Jak 1's own.
    pub flut_smooth_step: Option<f32>,
    /// On foot he walks up steps this high (Jak units), rising onto them
    /// as he reaches them (`step`; the host's: in Minecraft a block); none
    /// keeps the game's own.
    pub foot_smooth_step: Option<f32>,
    /// How far his walk leans into its uphill cycle for a step just
    /// climbed (1 as he rises, easing off).
    pub(crate) step_lean: f32,
    edge_grab_off_time: i64,
    /// In the air, something right under his feet (for the "stuck counts
    /// as landed" rule: wedged on a block's edge beside a wall is not).
    pub(crate) support_below: bool,
    /// He has his JET-Board (the host's: in Minecraft, equipped in the off
    /// hand): without it he cannot board, and with it he carries it on his
    /// back.
    pub board_carried: bool,
    /// How many times his own size the host draws him (in Minecraft he is
    /// scaled to stand two blocks tall): he floats as deep as his drawn
    /// body would in his own game.
    pub size: f32,
    pub lightjak: LightJak,
    /// Dark Jak: changed into with dark eco (Jak 3's `eco-pill-dark`), which
    /// his dark punches spend, back to Jak when it runs out (`darkjak.rs`).
    pub dark: bool,
    /// The dark eco meter, 0 to 100 (the host's, kept here while he steps).
    pub dark_eco: f32,
    /// Dark Jak's purple lightning on him and his victims (`dark_shock.rs`).
    pub dark_shock: dark_shock::DarkShock,
    pub dark_transition: Option<bool>,
    pub(crate) dark_combo: u8,
    pub(crate) dark_combo_alternate: bool,
    pub(crate) dark_combo_buffer: Option<i64>,
    pub grind: grind::Grind,
    /// Riding a Flut Flut ([`flut`]).
    pub flut: flut::FlutInfo,
    /// A hurt's effects still going on him ([`hurt`]).
    pub hurt_fx: HurtFx,
}

impl Jak {
    /// Jak standing at `trans` facing `yaw` (rotation units, 0 along +z).
    pub fn new(trans: Vec3, yaw: f32) -> Self {
        let time = 0;
        Self {ladder:None,ladder_after:0,
            powers_blocked: false,
            control: Control::new(trans, yaw, time),
            board: BoardInfo::default(),
            gun: Gun::default(),
            pad: Pad::default(),
            state: State::Falling { uppercut: false },
            state_time: time,
            time,
            camera: Basis::IDENTITY,
            projectiles: Vec::new(),
            eco_blasts: Vec::new(),
            eco_blast_count: 0,
            weapons: Weapons::default(),
            particles: None,
            events: Vec::new(),
            cache: CollideCache::default(),
            probe_cache: CollideCache::default(),
            pending: None,
            pad_bounce: None,
            chan: Channel::default(),
            push: Push::default(),
            anims: Arc::new(Anims::nominal()),
            code: Code::default(),
            trace: Trace::default(),
            foot: FootInfo::default(),
            attack: Attack::default(),
            align: align::Align::default(),
            joints: Vec::new(),
            aim_yaw: None,
            water: None,
            swim: None,
            swim_tilt: 0.0,
            hurled_until: 0,
            invulnerable_until: 0,
            hit_alt: false,
            ledge: None,
            pilot_edge_grab: false,
            movement_scale: 1.0,
            eco_power: None,
            powerup_light: false,
            jump_scale: 1.0,
            fire_resistant: false,
            slow_falling:false,
            edge_grab_min_height: math::meters(2.3),
            board_smooth_step: None,
            flut_smooth_step: None,
            foot_smooth_step: None,
            step_lean: 0.0,
            edge_grab_off_time: i64::MIN / 4,
            support_below: true,
            board_carried: true,
            hold: None,
            crown: None,
            wade: None,
            size: 1.0,
            lightjak: LightJak::default(),
            dark: false,
            dark_eco: 0.0,
            dark_shock: dark_shock::DarkShock::default(),
            dark_transition: None,
            dark_combo: 0,
            dark_combo_alternate: false,
            dark_combo_buffer: None,
            grind: grind::Grind::default(),
            flut: flut::FlutInfo::default(),
            hurt_fx: HurtFx::default(),
        }
    }

    pub fn trans(&self) -> Vec3 {
        self.control.trans
    }

    pub fn velocity(&self) -> Vec3 {
        self.control.transv
    }

    /// A physical contact correction, preserving the current move, velocity,
    /// animation and attacks. Teleports use `relocate` instead.
    pub fn displace(&mut self,delta:Vec3) {
        if !delta.is_finite(){return;}
        self.control.trans+=delta;
        self.flut_displace_mount(delta);
        self.cache=CollideCache::default();
        self.probe_cache=CollideCache::default();
    }

    pub fn relocate(&mut self, position: Vec3) {
        if !position.is_finite() {
            return;
        }
        self.ladder=None;
        let board = self.state.is_board();
        let yaw = math::y_angle(self.orientation() * Vec3::Z);
        self.control = Control::new(position, yaw, self.time);
        self.cache = CollideCache::default();
        self.probe_cache = CollideCache::default();
        self.foot = FootInfo::default();
        self.align = align::Align::default();
        self.attack = Attack::default();
        self.pending = None;
        self.pad_bounce = None;
        self.projectiles.clear();
        self.weapons.clear();
        self.grind.ride = None;
        self.trace.peak = position.y;
        self.go(if board {
            State::BoardFalling
        } else {
            State::Falling { uppercut: false }
        });
    }

    pub fn orientation(&self) -> Quat {
        self.control.quat
    }

    /// Whether blows pass him by now (a second after one struck him).
    pub fn invulnerable(&self) -> bool {
        self.time < self.invulnerable_until
    }

    /// Struck by a blow from `from` (another Jak's, local two-player):
    /// `target-hit` with its generic `attack-info` (shove-back 1.5 m,
    /// shove-up 1.2 m), knocked away from the blow with
    /// `jakb-hit-from-front` (or its `-alt1`) facing it, `-from-back` struck
    /// from behind; then a second when blows pass him by
    /// (`hit-invulnerable-timeout`). On the Flut Flut its own hit. Whether
    /// it struck.
    pub fn hit_by(&mut self, from: Vec3) -> bool {
        self.hurt(HurtMode::Generic, Some(from))
    }

    /// The events since the last call.
    pub fn take_events(&mut self) -> Vec<Event> {
        std::mem::take(&mut self.events)
    }

    /// Mount the board at once, without the button, as if R2 had been held
    /// with room above.
    pub fn request_board(&mut self) {
        self.board.latch = true;
    }

    /// One 60 Hz frame: input, the state's checks and its own sequence,
    /// the movement, then the shots in flight.
    pub fn step(
        &mut self,
        input: &PadInput,
        world: &mut dyn CollideWorld,
        mut actors: Option<&mut dyn ActorWorld>,
    ) {
        let events_before = self.events.len();
        self.time += FRAME_TICKS;
        self.pad.update(input);
        if self.time < self.hurled_until {
            self.pad.stick0_speed = 0.0;
        }
        self.code.arrived = false;
        self.hurt_frame();
        self.eco_power_frame();
        self.dark_eco_frame();
        if let Some(height) = self.pad_bounce.take() {
            // A hurt or relocation may have removed the supporting contact
            // since the world requested the bounce. Do not override it.
            if self.control.status & control::status::ON_GROUND != 0 {
                self.bounce_from_pad(height);
            }
        }
        let previous = self.trans();
        self.flut_water();
        if self.dark_transition.is_some() {
            self.dark_transition_step(world);
        } else if self.ladder_step(world) {
        } else if self.swim_step(world) {
        } else if !self.grind_step(input, world) {
            self.run_state(world);
            self.post(world);
        }
        if let Some(next) = self.pending.take() {
            self.trace.why = "post";
            self.go(next);
        }
        self.grind_attach(previous, world);
        self.attack_post(world, actors.as_deref_mut());
        self.dark_shock_frame(world, projectile::reborrow(&mut actors), events_before);
        self.track_peak();
        self.gun_frame(world, projectile::reborrow(&mut actors));
        self.step_projectiles_with(world, projectile::reborrow(&mut actors));
        self.step_eco_blasts(world, projectile::reborrow(&mut actors));
        let first_event = events_before;
        self.step_weapons(world, actors);
        self.gun_particles(first_event);
    }

    /// The state's checks, then its code up to its next suspend. A change of
    /// state runs the new state's checks and code in the same frame.
    fn run_state(&mut self, world: &mut dyn CollideWorld) {
        for _ in 0..8 {
            if let Some(next) = self.decide(world) {
                self.trace.why = "trans";
                self.go(next);
                continue;
            }
            match self.run_code(world) {
                Some(next) => {
                    self.trace.why = "code";
                    self.go(next);
                }
                None => return,
            }
        }
    }

    fn run_code(&mut self, world: &mut dyn CollideWorld) -> Option<State> {
        if self.state.is_flut() {
            self.flut_code(world)
        } else if self.state.is_board() {
            self.board_code()
        } else {
            self.foot_code(world)
        }
    }

    fn track_peak(&mut self) {
        let y = self.control.trans.y;
        if self.control.on_surface() {
            self.trace.peak = y;
        } else {
            self.trace.peak = self.trace.peak.max(y);
        }
    }

    /// Leaves the current state for `next`: the old state's exit runs with
    /// the new one already known, then the new one's entry, and its code
    /// starts from the top.
    pub(crate) fn go(&mut self, next: State) {
        let old = self.state;
        if old.is_flut() {
            self.flut_exit(&old, &next);
        } else if old.is_board() {
            self.board_exit_state(&old, &next);
        } else if !self.code.no_exit {
            self.foot_exit(&old, &next);
        }
        self.trace.prev = old.name();
        self.trace.changed_at = self.time;
        self.foot.prev_state_time = self.state_time;
        self.state = next;
        self.state_time = self.time;
        self.code = Code {
            arrived: true,
            ..Code::default()
        };
        if next.is_flut() {
            self.flut_enter(&next);
        } else if next.is_board() {
            self.board_enter(&next);
        } else if let Some(instead) = self.foot_enter(&next) {
            self.trace.why = "enter";
            self.go(instead);
        }
    }

    fn decide(&mut self, world: &mut dyn CollideWorld) -> Option<State> {
        if self.state.is_flut() {
            return self.flut_trans();
        }
        if let Some(on) = self.flut_start_check() {
            return Some(on);
        }
        if let Some(flap) = self.flap_from_the_air() {
            return Some(flap);
        }
        if self.state.is_board() {
            self.board_trans()
        } else {
            self.foot_trans(world)
        }
    }

    fn post(&mut self, world: &mut dyn CollideWorld) {
        // Hanging and climbing move him by hand (`target-no-move-post`), and
        // so do climbing onto the Flut Flut and jumping down off it (Jak
        // 1's `target-flut-get-on` and `-get-off-jump`; 19/10: pulled down
        // meanwhile, he sank through the ground getting off and was set
        // down under it, falling for ever).
        if matches!(
            self.state,
            State::EdgeGrab | State::EdgeGrabJump | State::EdgeGrabOff | State::FlutGetOn | State::FlutGetOffJump
        ) {
            return;
        }
        if self.state.is_board() && self.state != State::BoardGetOff {
            self.board_post(world);
        } else {
            // Swimming on the Flut Flut: held up by the water.
            if self.state.is_flut() && self.flut_swim_post(world) {
                return;
            }
            if matches!(self.state, State::FlutWalk | State::FlutStance) {
                self.flut_step_up(world);
            } else if !self.state.is_flut() {
                // Foot stepping runs after steering inside target_post.
            }
            self.target_post(world);
        }
    }
}

#[cfg(test)] mod contact_displacement_tests {
    use super::*;
    #[test]fn contact_does_not_cancel_climbing_onto_a_mount(){
        let mut j=Jak::new(Vec3::ZERO,0.0);j.state=State::FlutGetOn;
        j.flut_set_mount(Vec3::new(4.0,2.0,3.0),Quat::IDENTITY);
        j.control.transv=Vec3::new(8.0,9.0,10.0);j.chan.frame=12.0;
        let delta=Vec3::new(0.2,0.0,-0.1);j.displace(delta);
        assert_eq!(j.state,State::FlutGetOn);assert_eq!(j.chan.frame,12.0);
        assert_eq!(j.velocity(),Vec3::new(8.0,9.0,10.0));assert_eq!(j.trans(),delta);
        assert_eq!(j.flut_mount_target().unwrap().0,Vec3::new(4.0,2.0,3.0)+delta);
    }
    #[test]fn contact_preserves_a_riding_state_while_a_teleport_ends_it(){
        let mut j=Jak::new(Vec3::ZERO,0.0);j.state=State::FlutStance;j.chan.frame=7.0;
        j.displace(Vec3::X);assert_eq!(j.state,State::FlutStance);assert_eq!(j.chan.frame,7.0);
        j.relocate(Vec3::Y);assert!(!j.state.is_flut());assert_eq!(j.trans(),Vec3::Y);
    }
}
