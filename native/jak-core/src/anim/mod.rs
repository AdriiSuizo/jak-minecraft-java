//! Jak's animation clock. States drive gameplay windows off the frame an
//! animation has reached, so the simulation keeps the same per-frame
//! playback the game does: a frame number advanced by a playback function
//! and the animation's own speed. Only timing lives here; poses are the
//! renderer's.
//!
//! The timing itself is the player's: [`Anims::from_rows`] takes it from
//! their own Jak 3 files at run time. Without them every animation gets
//! [`NOMINAL`].
mod table;

use std::sync::Arc;

use glam::{Quat, Vec3};
pub use table::*;

/// One animation's timing.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AnimInfo {
    pub frames: u16,
    /// Frames advanced per 60 Hz frame at rate 1.
    pub speed: f32,
    pub artist_base: f32,
    pub artist_step: f32,
}

/// What an animation the player's files do not time plays at.
pub const NOMINAL: AnimInfo = AnimInfo {
    frames: 21,
    speed: 0.5,
    artist_base: 0.0,
    artist_step: 1.0,
};

/// Where an animation carries Jak: its align joint at each frame, as an
/// offset from where it started (units) and a turn.
#[derive(Clone, Debug, PartialEq)]
pub struct AlignTrack {
    pub trans: Vec<Vec3>,
    pub quat: Vec<Quat>,
}

impl AlignTrack {
    /// At a frame between two, as the joint is evaluated.
    pub fn sample(&self, frame: f32) -> (Vec3, Quat) {
        let n = self.trans.len().min(self.quat.len());
        if n == 0 {
            return (Vec3::ZERO, Quat::IDENTITY);
        }
        let f = frame.clamp(0.0, (n - 1) as f32);
        let a = f.floor() as usize;
        let b = (a + 1).min(n - 1);
        let t = f - a as f32;
        (
            self.trans[a].lerp(self.trans[b], t),
            self.quat[a].slerp(self.quat[b], t).normalize(),
        )
    }
}

/// What an animation's frame fires (`effect-control`'s `do-effect`), of
/// what Jak's own art groups name: a footstep, a landing, a roll or a slide
/// (each sounding the ground under him), or a sound by its own name.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EffectKind {
    /// `effect-walk-step-left` (1) and `-right` (2).
    WalkStep(u8),
    /// `effect-run-step-left` (1) and `-right` (2).
    RunStep(u8),
    Land,
    Roll,
    Slide,
    /// A sound played as named (`hand-grab`, `pull-up`), Jak 3's.
    Sound(&'static str),
}

impl EffectKind {
    /// The kind an effect's name is, for those Jak's sounds play; the rest
    /// (particles, footprints, sounds already played by his states) none.
    pub fn of(name: &str) -> Option<Self> {
        Some(match name {
            "effect-walk-step-left" => EffectKind::WalkStep(1),
            "effect-walk-step-right" => EffectKind::WalkStep(2),
            "effect-run-step-left" => EffectKind::RunStep(1),
            "effect-run-step-right" => EffectKind::RunStep(2),
            "effect-land" => EffectKind::Land,
            "effect-roll" => EffectKind::Roll,
            "effect-slide" => EffectKind::Slide,
            "hand-grab" => EffectKind::Sound("HAND_GRAB"),
            "pull-up" => EffectKind::Sound("PULL_UP"),
            _ => return None,
        })
    }
}

/// One effect at an artist frame, at the volume (a fraction) and pitch
/// (octaves) its `effect-param` gives.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AnimEffect {
    pub frame: f32,
    pub kind: EffectKind,
    pub volume: f32,
    pub pitch: f32,
}

/// The timing of every animation in [`NAMES`], and the motion of those the
/// player's files carry it for.
#[derive(Clone, Debug, PartialEq)]
pub struct Anims {
    infos: Vec<AnimInfo>,
    aligns: Vec<Option<Arc<AlignTrack>>>,
    effects: Vec<Arc<[AnimEffect]>>,
    /// How many came from the player's files.
    pub timed: usize,
}

impl Default for Anims {
    fn default() -> Self {
        Self::nominal()
    }
}

impl Anims {
    pub fn nominal() -> Self {
        Self {
            infos: vec![NOMINAL; NAMES.len()],
            aligns: vec![None; NAMES.len()],
            effects: vec![Arc::from([]); NAMES.len()],
            timed: 0,
        }
    }

    /// Timing by animation name; names Jak's states never play are skipped.
    pub fn from_rows<'a>(rows: impl IntoIterator<Item = (&'a str, AnimInfo)>) -> Self {
        let mut anims = Self::nominal();
        for (name, info) in rows {
            if let Some(anim) = Anim::by_name(name)
                && info.frames > 0
                && info.speed > 0.0
                && info.artist_step != 0.0
            {
                anims.infos[usize::from(anim.0)] = info;
                anims.timed += 1;
            }
        }
        anims
    }

    /// Adds the motion of the animation called `name`.
    pub fn with_align(mut self, name: &str, track: AlignTrack) -> Self {
        if let Some(anim) = Anim::by_name(name) {
            self.aligns[usize::from(anim.0)] = Some(Arc::new(track));
        }
        self
    }

    /// Sets the effects the animation called `name` fires.
    pub fn with_effects(mut self, name: &str, effects: Vec<AnimEffect>) -> Self {
        if let Some(anim) = Anim::by_name(name) {
            self.effects[usize::from(anim.0)] = Arc::from(effects);
        }
        self
    }

    /// The effects an animation fires, by artist frame.
    pub fn effects(&self, anim: Anim) -> &[AnimEffect] {
        self.effects.get(usize::from(anim.0)).map_or(&[], |e| e)
    }

    pub fn align(&self, anim: Anim) -> Option<&AlignTrack> {
        self.aligns.get(usize::from(anim.0))?.as_deref()
    }

    /// How many animations carry their motion.
    pub fn aligned(&self) -> usize {
        self.aligns.iter().filter(|a| a.is_some()).count()
    }

    pub fn info(&self, anim: Anim) -> AnimInfo {
        self.infos
            .get(usize::from(anim.0))
            .copied()
            .unwrap_or(NOMINAL)
    }

    /// The last frame.
    pub fn max(&self, anim: Anim) -> f32 {
        f32::from(self.info(anim).frames.saturating_sub(1))
    }

    /// The frame an artist frame number names.
    pub fn aframe(&self, anim: Anim, artist: f32) -> f32 {
        let i = self.info(anim);
        (artist - i.artist_base) / i.artist_step
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Anim(pub u16);

impl Anim {
    pub fn name(self) -> &'static str {
        NAMES.get(usize::from(self.0)).copied().unwrap_or("")
    }

    pub fn by_name(name: &str) -> Option<Anim> {
        NAMES
            .iter()
            .position(|a| *a == name)
            .map(|i| Anim(i as u16))
    }
}

/// Where a seek heads.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Target {
    Max,
    Frame(f32),
}

/// How a channel's frame moves each time it is evaluated.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum NumFunc {
    /// Stays where it is.
    Identity,
    /// Toward `target` by at most `rate` (times the animation's speed).
    Seek { target: Target, rate: f32 },
    /// Forward by `rate`, wrapping at the end.
    Loop { rate: f32 },
}

impl NumFunc {
    pub const fn seek(rate: f32) -> Self {
        NumFunc::Seek {
            target: Target::Max,
            rate,
        }
    }

    pub const fn seek_to(frame: f32, rate: f32) -> Self {
        NumFunc::Seek {
            target: Target::Frame(frame),
            rate,
        }
    }
}

/// The walk cycle's seven channels as one: the walk and the run, each
/// leaned toward its uphill or downhill cycle and its sideways one, all at
/// the base channel's frame, the run mixed over the walk.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct WalkMix {
    /// 0 walks, 1 runs.
    pub run: f32,
    /// Uphill positive, downhill negative.
    pub up: f32,
    /// Toward the right cycle positive, the left negative.
    pub side: f32,
}

impl WalkMix {
    /// The ground one cycle covers, as the channels' distances mix.
    pub fn cycle_dist(&self) -> f32 {
        use crate::target::cycle;
        let (walk_slope, run_slope) = if self.up >= 0.0 {
            (cycle::WALK_UP, cycle::RUN_UP)
        } else {
            (cycle::WALK_DOWN, cycle::RUN_DOWN)
        };
        let up = self.up.abs();
        let side = self.side.abs();
        let walk = lerp(lerp(cycle::WALK, walk_slope, up), cycle::WALK_SIDE, side);
        let run = lerp(lerp(cycle::RUN, run_slope, up), cycle::RUN_SIDE, side);
        lerp(walk, run, self.run)
    }

    /// The animations of the six cycles: walk, its slope and side, run, its
    /// slope and side.
    pub fn anims(&self) -> [Anim; 6] {
        let (walk_slope, run_slope) = if self.up >= 0.0 {
            (WALK_UP, RUN_UP)
        } else {
            (WALK_DOWN, RUN_DOWN)
        };
        let (walk_side, run_side) = if self.side >= 0.0 {
            (WALK_RIGHT, RUN_RIGHT)
        } else {
            (WALK_LEFT, RUN_LEFT)
        };
        [WALK, walk_slope, walk_side, RUN, run_slope, run_side]
    }
}

fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}

/// The base animation channel.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Channel {
    pub anim: Anim,
    pub frame: f32,
    pub func: NumFunc,
    /// The walk cycle's mix, while it plays.
    pub mix: Option<WalkMix>,
}

impl Default for Channel {
    fn default() -> Self {
        Self {
            anim: STANCE_LOOP,
            frame: 0.0,
            func: NumFunc::Identity,
            mix: None,
        }
    }
}

impl Channel {
    /// Switches to `anim` at `frame` with `func`, without advancing.
    pub fn set(&mut self, anim: Anim, func: NumFunc, frame: f32) {
        self.anim = anim;
        self.func = func;
        self.frame = frame;
        self.mix = None;
    }

    fn target(&self, anims: &Anims, target: Target) -> f32 {
        match target {
            Target::Max => anims.max(self.anim),
            Target::Frame(f) => f,
        }
    }

    /// One evaluation of the playback function.
    pub fn eval(&mut self, anims: &Anims) {
        let speed = anims.info(self.anim).speed;
        match self.func {
            NumFunc::Identity => {}
            NumFunc::Seek { target, rate } => {
                let target = self.target(anims, target);
                self.frame = crate::math::seek(self.frame, target, rate * speed);
            }
            NumFunc::Loop { rate } => {
                let length = anims.max(self.anim);
                if length > 0.0 {
                    let f = self.frame + length + rate * speed;
                    self.frame = f - (f / length).trunc() * length;
                }
            }
        }
    }

    /// Changes the playback function and evaluates it.
    pub fn eval_with(&mut self, anims: &Anims, func: NumFunc) {
        self.func = func;
        self.eval(anims);
    }

    /// A seek that has reached its target; anything else counts as done.
    pub fn done(&self, anims: &Anims) -> bool {
        match self.func {
            NumFunc::Seek { target, .. } => self.frame == self.target(anims, target),
            _ => true,
        }
    }

    /// The current frame in artist numbering.
    pub fn aframe_num(&self, anims: &Anims) -> f32 {
        let i = anims.info(self.anim);
        self.frame * i.artist_step + i.artist_base
    }

    pub fn is(&self, anim: Anim) -> bool {
        self.anim == anim
    }

    pub fn is_any(&self, anims: &[Anim]) -> bool {
        anims.contains(&self.anim)
    }
}

/// When the pose last started blending from the previous one, and over how
/// many ticks.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Push {
    pub tick: i64,
    pub ticks: i64,
}
