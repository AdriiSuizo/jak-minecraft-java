//! Jak hurt, as Jak 3's `target-hit` (`target-death.gc`) and what goes with
//! it (`target-hit-effect`, `target-attacked`, `target-timed-invulnerable`,
//! `process-drawable-burn-effect`, `target-color-effect-process`): knocked
//! away from the blow facing it (`jakb-hit-from-front` or its `-alt1`), or
//! from behind (`-from-back`), lightning's `jakb-hit-elec` before it, on
//! the board its own (`target-board-hit`, `jakb-board-hit`), on the Flut
//! Flut its own; Daxter follows each of them (his own `daxter-` moves of
//! the same names). Each one: `group-target-hit` where it struck, "oof", the
//! pad's buzz (half a second), and a second when blows pass him by
//! (`hit-invulnerable-timeout`) while he blinks (hidden one 60 Hz frame in
//! four, `post-flag-setup`). Fire burns him dark and back over four seconds
//! with smoke off him (`process-drawable-burn-effect`, part 157); lightning
//! flickers his colours (`'shock` color effect). Damage with no blow
//! (hunger, drowning, inside a block) is `target-attacked`'s `'damage`: the
//! sparks, "oof", the buzz and the blink, no knock.

use glam::{Mat3, Quat, Vec3};

use crate::math::{self, FRAME_TICKS, TICKS_PER_SECOND};
use crate::{Event, Jak, State, anim};

/// What hurt Jak, as Jak 3's `attack-info` modes.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum HurtMode {
    /// A blow, a shot, an arrow, a cactus: `'generic`.
    Generic,
    /// A blast (a creeper, TNT, a vehicle's): `'explode`, knocked away
    /// from its middle.
    Explode,
    /// Fire: `'burn`, knocked away and burnt.
    Burn,
    /// Lightning: `'shock`, held in `jakb-hit-elec` flickering, then
    /// knocked as from behind.
    Shock,
    /// Damage with no blow (hunger, drowning, inside a block):
    /// `target-attacked`'s `'damage`.
    Damage,
    /// A hard landing (`target-hit-ground-hard`, its own state): the blink
    /// and a short buzz only.
    Fall,
}

/// `hit-invulnerable-timeout`: a second.
pub const INVULNERABLE: i64 = TICKS_PER_SECOND;
/// `process-drawable-burn-effect`'s 1200: four seconds.
pub const BURN: i64 = 4 * TICKS_PER_SECOND;
/// The `'shock` color effect's own length after its last renewal.
const SHOCK_COLOUR: i64 = TICKS_PER_SECOND / 5;
/// The default `attack-info`: shoved 1.5 m back (6144) and 1.2 m up
/// (4915.2).
const SHOVE_BACK: f32 = 1.5;
const SHOVE_UP: f32 = 1.2;

/// The effects of a hurt still going on him.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct HurtFx {
    /// When the burn began (ticks).
    pub burn_start: Option<i64>,
    /// Lightning holds him until then, flickering (ticks).
    pub shock_hold_until: i64,
    /// Where the lightning came from, for the knock after it.
    pub shock_from: Option<Vec3>,
    /// The flicker lasts until then (ticks).
    pub shock_until: i64,
}

impl Jak {
    /// Hurt by `mode` from `from` (Jak units; none: from ahead of him, the
    /// game's vector-less attack). Riding a vehicle the host keeps it from
    /// him (Jak 3's pilot takes no knock). Whether it struck him (not in
    /// the second after another).
    pub fn hurt(&mut self, mode: HurtMode, from: Option<Vec3>) -> bool {
        if self.fire_resistant && mode == HurtMode::Burn { return false; }
        if self.invulnerable() {
            return false;
        }
        self.invulnerable_until = self.time + INVULNERABLE;
        let middle = self.control.trans + Vec3::Y * math::meters(0.9) * self.size;
        if mode != HurtMode::Fall {
            if let Some(particles) = self.particles.as_mut() {
                particles.spawn("group-target-hit", middle, Mat3::IDENTITY);
                if mode == HurtMode::Shock {
                    particles.spawn("group-lightning-glow", middle, Mat3::IDENTITY);
                }
            }
            self.events.push(Event::Sound { name: "OOF", volume: 1.0, pitch: 0.0 });
        }
        self.events.push(Event::Hurt {
            mode,
            pos: middle,
            buzz: if mode == HurtMode::Fall { 0.1 } else { 0.5 },
        });
        if mode == HurtMode::Burn && self.hurt_fx.burn_start.is_none_or(|t| self.time - t >= BURN) {
            self.hurt_fx.burn_start = Some(self.time);
            self.events.push(Event::Sound { name: "GET_BURNED", volume: 1.0, pitch: 0.0 });
        }
        match mode {
            HurtMode::Damage | HurtMode::Fall => return true,
            // In the water he is not thrown out of it.
            _ if self.swim.is_some() => return true,
            _ => {}
        }
        if self.on_flut() {
            let from = from.unwrap_or(self.control.trans + math::z_axis(self.control.quat) * math::METER);
            self.flut_hit(from);
            return true;
        }
        if self.state.is_board() {
            self.board_hit(mode, from);
            return true;
        }
        self.ledge = None;
        self.grind.ride = None;
        if mode == HurtMode::Shock {
            // Held flickering in `jakb-hit-elec` to its end, then knocked as
            // from behind (`(set! angle 'back)`).
            let info = self.anims.info(anim::HIT_ELEC);
            let ticks = (self.anims.max(anim::HIT_ELEC) / info.speed.max(1e-3)) as i64 * FRAME_TICKS;
            self.hurt_fx.shock_hold_until = self.time + ticks;
            self.hurt_fx.shock_until = self.time + ticks + SHOCK_COLOUR;
            self.hurt_fx.shock_from = from;
            self.hurled_until = self.hurt_fx.shock_hold_until;
            self.go(State::Falling { uppercut: true });
            self.clear_ground();
            self.control.mod_surface = crate::surface::foot::SMACK_JUMP;
            self.chan.set(anim::HIT_ELEC, anim::NumFunc::seek(1.0), 0.0);
            self.control.transv = Vec3::ZERO;
            return true;
        }
        self.knock(from, false);
        true
    }

    /// `target-hit-move` on foot: shoved `SHOVE_BACK` away from `from` and
    /// `SHOVE_UP` up, facing the blow (from behind: facing away);
    /// `back` makes it from behind whatever the side.
    fn knock(&mut self, from: Option<Vec3>, back: bool) {
        self.swim = None;
        self.swim_tilt = 0.0;
        let facing = math::z_axis(self.control.quat);
        let away = from
            .map(|f| (self.control.trans - f).with_y(0.0))
            .and_then(|v| v.try_normalize())
            .unwrap_or(-facing);
        let (shove_back, shove_up) = (math::meters(SHOVE_BACK), math::meters(SHOVE_UP));
        // In front when the shove runs against the way he faces.
        let front = !back && away.dot(facing) <= 0.0;
        let face = if front { -away } else { away };
        let turn = Quat::from_rotation_y(face.x.atan2(face.z));
        self.control.quat = turn;
        self.control.dir_targ = turn;
        let g = crate::control::STANDARD_GRAVITY;
        // `(+ 2048.0 (sqrtf (* 491520.0 shove-up)))`: half a meter a second
        // over the rise to the shove's height.
        let up = math::meters(0.5) + (2.0 * g * shove_up).sqrt();
        let flight = 2.0 * up / g;
        self.hurled_until = self.time + (flight * TICKS_PER_SECOND as f32) as i64;
        self.go(State::Falling { uppercut: true });
        self.clear_ground();
        self.control.mod_surface = crate::surface::foot::SMACK_JUMP;
        let clip = if !front {
            anim::HIT_FROM_BACK
        } else {
            self.hit_alt = !self.hit_alt;
            if self.hit_alt { anim::HIT_FROM_FRONT_ALT1 } else { anim::HIT_FROM_FRONT }
        };
        self.chan.set(clip, anim::NumFunc::seek(1.0), 0.0);
        let across = away * (shove_back / flight.max(0.2));
        self.control.transv = Vec3::new(across.x, up, across.z);
    }

    /// `target-board-hit`: `jakb-board-hit` (lightning's `-elec`), shoved
    /// as on foot, staying on the board.
    fn board_hit(&mut self, mode: HurtMode, from: Option<Vec3>) {
        let facing = math::z_axis(self.control.quat);
        let away = from
            .map(|f| (self.control.trans - f).with_y(0.0))
            .and_then(|v| v.try_normalize())
            .unwrap_or(-facing);
        let g = crate::control::STANDARD_GRAVITY;
        let up = math::meters(0.5) + (2.0 * g * math::meters(SHOVE_UP)).sqrt();
        let flight = 2.0 * up / g;
        self.hurled_until = self.time + (flight * TICKS_PER_SECOND as f32) as i64;
        self.go(State::BoardFalling);
        self.clear_ground();
        let clip = if mode == HurtMode::Shock { anim::BOARD_HIT_ELEC } else { anim::BOARD_HIT };
        self.chan.set(clip, anim::NumFunc::seek(1.0), 0.0);
        let across = away * (math::meters(SHOVE_BACK) / flight.max(0.2));
        self.control.transv = Vec3::new(across.x, up, across.z);
    }

    /// Each 60 Hz frame: lightning's hold and the knock after it, the
    /// burn's smoke.
    pub(crate) fn hurt_frame(&mut self) {
        let fx = self.hurt_fx;
        if fx.shock_hold_until > 0 {
            if self.time < fx.shock_hold_until {
                // `(+ f0-44 (* 286720.0 (seconds-per-frame)))`: held up
                // against his gravity while it plays.
                self.control.transv = Vec3::ZERO;
            } else {
                self.hurt_fx.shock_hold_until = 0;
                if !self.state.is_board() && !self.on_flut() && self.swim.is_none() {
                    self.knock(fx.shock_from, true);
                }
            }
        }
        if let Some(start) = fx.burn_start {
            if self.time - start >= BURN {
                self.hurt_fx.burn_start = None;
            } else if let Some(particles) = self.particles.as_mut() {
                // Part 157 at a point about his body each frame.
                let k = (self.time / FRAME_TICKS) as u32;
                let r = |n: u32| (hash(k.wrapping_mul(3).wrapping_add(n)) - 0.5) * 2.0;
                let size = self.size;
                let at = self.control.trans
                    + Vec3::new(r(0) * math::meters(0.35), (r(1) * 0.5 + 0.5) * math::meters(1.8), r(2) * math::meters(0.35)) * size;
                particles.launch(157, at, Mat3::IDENTITY);
            }
        }
    }

    /// Hidden this frame: one 60 Hz frame in four while blows pass him by
    /// (`post-flag-setup`'s `tinvuln1` blink).
    pub fn blink_hidden(&self) -> bool {
        self.invulnerable() && (self.time / FRAME_TICKS) % 4 == 0
    }

    /// His colours now (1 as they are): burnt dark to half and back over
    /// the burn, lightning's flicker (`rand-vu-float-range 0.5 2.0`, blue
    /// half over).
    pub fn hurt_tint(&self) -> [f32; 3] {
        if self.time < self.hurt_fx.shock_until {
            let f = 0.5 + 1.5 * hash((self.time / FRAME_TICKS) as u32);
            return [f, f, f + 0.5];
        }
        if let Some(start) = self.hurt_fx.burn_start {
            let e = (self.time - start) as f32 / BURN as f32;
            if (0.0..1.0).contains(&e) {
                let k = if e < 0.5 { e } else { 1.0 - e };
                return [1.0 - k; 3];
            }
        }
        [1.0; 3]
    }
}

/// A number in 0..1 from `n`.
fn hash(n: u32) -> f32 {
    let mut x = n.wrapping_mul(0x9E37_79B9) ^ 0x85EB_CA6B;
    x ^= x >> 15;
    x = x.wrapping_mul(0x2C1B_3C6D);
    x ^= x >> 12;
    (x & 0xFFFF) as f32 / 65535.0
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Tri, TriangleGrid};

    type FlatWorld = TriangleGrid;

    /// A floor 100 m across at height 0.
    fn floor() -> TriangleGrid {
        let m = math::meters(50.0);
        let (a, b, c, d) = (Vec3::new(-m, 0.0, -m), Vec3::new(-m, 0.0, m), Vec3::new(m, 0.0, m), Vec3::new(m, 0.0, -m));
        TriangleGrid::new(
            vec![Tri { v: [a, b, c], pat: Default::default() }, Tri { v: [a, c, d], pat: Default::default() }],
            math::meters(4.0),
        )
    }

    fn standing() -> (Jak, FlatWorld) {
        let mut world = floor();
        let mut jak = Jak::new(Vec3::ZERO, 0.0);
        let input = crate::PadInput::default();
        for _ in 0..60 {
            jak.step(&input, &mut world, None);
        }
        (jak, world)
    }

    fn run(jak: &mut Jak, world: &mut FlatWorld, frames: usize) {
        let input = crate::PadInput::default();
        for _ in 0..frames {
            jak.step(&input, world, None);
        }
    }

    /// A blow from ahead knocks him back facing it, plays a hit, and he is
    /// back on his feet and in control within two seconds; the second blow
    /// within the second passes him by.
    #[test]
    fn a_blow_knocks_him_back_and_lets_go() {
        let (mut jak, mut world) = standing();
        let ahead = jak.trans() + math::z_axis(jak.orientation()) * math::meters(1.0);
        assert!(jak.hurt(HurtMode::Generic, Some(ahead)));
        assert!(jak.chan.is_any(&[anim::HIT_FROM_FRONT, anim::HIT_FROM_FRONT_ALT1]));
        assert!(!jak.hurt(HurtMode::Generic, Some(ahead)), "invulnerable a second");
        let before = jak.trans();
        run(&mut jak, &mut world, 120);
        let moved = jak.trans() - before;
        assert!(moved.dot(math::z_axis(jak.orientation())) < -math::meters(0.5), "shoved back {moved:?}");
        assert!(matches!(jak.state, State::Stance | State::Walk | State::HitGround { .. }), "{:?}", jak.state);
        assert!(!jak.invulnerable());
        assert!(jak.take_events().iter().any(|e| matches!(e, Event::Hurt { mode: HurtMode::Generic, .. })));
    }

    /// From behind: `jakb-hit-from-back`, thrown ahead.
    #[test]
    fn from_behind_he_is_thrown_ahead() {
        let (mut jak, mut world) = standing();
        let facing = math::z_axis(jak.orientation());
        let behind = jak.trans() - facing * math::meters(1.0);
        jak.hurt(HurtMode::Explode, Some(behind));
        assert!(jak.chan.is(anim::HIT_FROM_BACK));
        let before = jak.trans();
        run(&mut jak, &mut world, 30);
        assert!((jak.trans() - before).dot(facing) > 0.0);
    }

    /// Damage without a blow does not move him, but he blinks and is
    /// invulnerable for a second.
    #[test]
    fn damage_only_does_not_move_him() {
        let (mut jak, mut world) = standing();
        let before = jak.trans();
        assert!(jak.hurt(HurtMode::Damage, None));
        let mut hidden = 0;
        for _ in 0..60 {
            run(&mut jak, &mut world, 1);
            hidden += usize::from(jak.blink_hidden());
        }
        assert!((jak.trans() - before).length() < math::meters(0.05));
        assert!((14..=16).contains(&hidden), "hidden {hidden} of 60 frames");
        assert_eq!(jak.state, State::Stance);
    }

    /// Lightning holds him in `jakb-hit-elec` flickering, then knocks him;
    /// he ends standing.
    #[test]
    fn lightning_holds_then_knocks() {
        let (mut jak, mut world) = standing();
        jak.hurt(HurtMode::Shock, None);
        assert!(jak.chan.is(anim::HIT_ELEC));
        assert_ne!(jak.hurt_tint(), [1.0; 3]);
        let held = jak.trans();
        run(&mut jak, &mut world, 3);
        assert!((jak.trans() - held).length() < math::meters(0.1));
        run(&mut jak, &mut world, 240);
        assert!(matches!(jak.state, State::Stance | State::Walk | State::HitGround { .. }), "{:?}", jak.state);
        assert_eq!(jak.hurt_tint(), [1.0; 3]);
    }

    /// Fire burns him to half dark and back over four seconds.
    #[test]
    fn fire_burns_dark_and_back() {
        let (mut jak, mut world) = standing();
        jak.hurt(HurtMode::Burn, None);
        run(&mut jak, &mut world, 120);
        let mid = jak.hurt_tint()[0];
        assert!((0.45..0.6).contains(&mid), "{mid}");
        run(&mut jak, &mut world, 130);
        assert_eq!(jak.hurt_tint(), [1.0; 3]);
    }

    /// On the board he stays on it, its own hit playing.
    #[test]
    fn on_the_board_he_stays_on_it() {
        let (mut jak, mut world) = standing();
        jak.request_board();
        run(&mut jak, &mut world, 60);
        assert!(jak.state.is_board(), "{:?}", jak.state);
        jak.hurt(HurtMode::Generic, None);
        assert!(jak.chan.is(anim::BOARD_HIT));
        run(&mut jak, &mut world, 120);
        assert!(jak.state.is_board(), "{:?}", jak.state);
    }
}
