use crate::{Event, Jak, PadInput, Push, State, anim, anim::NumFunc, collide::CollideWorld};

/// `darkjak-punch-inc`: what each dark punch of a combo spends (times
/// [`crate::lightjak::ECO_COST`], Adrii 22/10).
pub(crate) const DARK_PUNCH: f32 = 4.0;

impl Jak {
    /// Dark eco spent (`get-pickup eco-pill-dark` of minus `amount`).
    pub(crate) fn spend_dark(&mut self, amount: f32) {
        self.dark_eco = (self.dark_eco - amount * crate::lightjak::ECO_COST).max(0.0);
    }

    /// `target-darkjak-process`: with under a point of dark eco left, back
    /// to Jak (here a point is [`crate::lightjak::ECO_COST`]).
    pub(crate) fn dark_eco_frame(&mut self) {
        // The Bomb and the Blast end in `target-darkjak-get-off`.
        if std::mem::take(&mut self.bomb.get_off) {
            self.request_dark(false);
            return;
        }
        if matches!(self.state, State::DarkBomb | State::DarkBlast) {
            return;
        }
        if self.dark && self.dark_transition.is_none() && self.dark_eco < crate::lightjak::ECO_COST && self.attack.danger.is_none() {
            self.request_dark(false);
        }
    }

    /// target.gc / target-darkjak.gc: Dark Jak, `darkjak-bomb-min` of dark eco
    /// or more, can bomb (his dive) or blast (his uppercut).
    pub(crate) fn want_dark_bomb(&self) -> bool {
        self.dark && self.dark_transition.is_none() && !self.powers_blocked && self.dark_eco >= DARK_BOMB_MIN * crate::lightjak::ECO_COST
    }

    pub fn request_dark(&mut self, on: bool) {
        if self.dark_transition.is_some() || self.dark == on || self.state.is_board() || (on && self.powers_blocked) {
            return;
        }
        // `want-to-darkjak?`: only with dark eco.
        if on && self.dark_eco <= 0.0 {
            return;
        }
        // target-darkjak-get-off's exit: under a point left, none left (the
        // meter showed nothing but kept him Dark, Adrii 06/11).
        if !on && self.dark_eco < crate::lightjak::ECO_COST {
            self.dark_eco = 0.0;
        }
        self.bomb.scale = 0.0;
        self.bomb.blast = None;
        self.lightjak_end_mode(true);
        self.gun.end_mode();
        // A board called for before (R2 pressed while it could not come)
        // never comes once he changes back (Adrii 17/10).
        self.board.latch = false;
        self.go(State::Stance);
        self.pending = None;
        self.danger_set(None);
        self.dark_transition = Some(on);
        if on {
            self.dark = true;
        }
        self.push = Push {
            tick: self.time,
            ticks: crate::math::seconds(if on { 0.1 } else { 0.05 }),
        };
        self.chan.set(
            if on {
                anim::DARKJAK_GET_ON
            } else {
                anim::DARKJAK_GET_OFF
            },
            NumFunc::seek(1.0),
            0.0,
        );
        self.events.push(Event::DarkJak { on });
    }

    pub(crate) fn dark_transition_step(&mut self, world: &mut dyn CollideWorld) {
        let Some(on) = self.dark_transition else {
            return;
        };
        self.pad.update(&PadInput::default());
        if self.control.on_surface() {
            self.control.transv = crate::glam::Vec3::ZERO;
        }
        self.chan.eval(&self.anims);
        self.target_post(world);
        self.pending = None;
        if self.chan.done(&self.anims) {
            if !on && self.chan.is(anim::DARKJAK_GET_OFF) {
                self.chan
                    .set(anim::DARKJAK_GET_OFF_END, NumFunc::seek(1.0), 0.0);
            } else {
                self.dark = on;
                self.dark_transition = None;
                self.board.latch = false;
                self.go(if self.control.on_surface() {
                    State::Stance
                } else {
                    State::Falling { uppercut: false }
                });
            }
        }
    }
}

/// `*FACT-bank*` darkjak-bomb0-inc and darkjak-bomb1-inc (what each spends)
/// and darkjak-bomb-min (what either needs), OpenGOAL's points.
pub(crate) const DARK_BOMB_COST: f32 = 100.0;
pub(crate) const DARK_BOMB_MIN: f32 = 50.0;
/// target-util.gc `'bomb`: the danger sphere grows to 30 m.
pub const BOMB_RADIUS: f32 = 122880.0;
/// target-darkjak-bomb1: its search (`w` 327680, 80 m), a bolt every 0.1 s, at
/// least 1.5 s, each bolt's blow (15, `'ice`).
const BLAST_REACH: f32 = 327680.0;
const BLAST_EVERY: f32 = 0.1;
const BLAST_FOR: f32 = 1.5;
const BLAST_DAMAGE: f32 = 15.0;
/// target-darkjak-bomb0's fall: its own gravity on top.
const BOMB_FALL: f32 = -491520.0;

#[derive(Clone, Debug, Default)]
pub struct DarkBomb {
    /// `control bomb-scale`: 0 to 1.
    pub scale: f32,
    pub(crate) blast: Option<Blast>,
    /// Done: back to Jak on the next frame (no longer used by the Bomb and the Blast, see below).
    pub(crate) get_off: bool,
    /// Where and when the bomb went off: the host draws skel-bomb-blast and skel-generic-blast there.
    pub blast_at: Option<(crate::glam::Vec3, i64)>,
}

#[derive(Clone, Debug, Default)]
pub(crate) struct Blast {
    searched: bool,
    targets: Vec<(u64, crate::glam::Vec3)>,
    shots: usize,
    last_shot: i64,
    start: i64,
}

impl Jak {
    /// `target-darkjak-bomb0`'s code.
    pub(crate) fn dark_bomb_code(&mut self) -> Option<State> {
        use crate::anim::{self, NumFunc};
        use crate::math::{seconds, SECONDS_PER_FRAME};
        loop {
            match self.code.pc {
                0 => {
                    self.ja_push(seconds(0.15));
                    if self.control.on_surface() {
                        self.ja_set(anim::DARKJAK_BOMB_PRE, NumFunc::seek(1.0), 0.0);
                        self.goto(1);
                    } else {
                        self.goto(2);
                    }
                }
                1 => {
                    let v = self.control.ctrl_xz_vel * 0.85;
                    self.set_forward_vel(v);
                    if !self.ja_wait() {
                        return None;
                    }
                    self.goto(2);
                }
                2 => {
                    self.ja_set(anim::DARKJAK_BOMB, NumFunc::seek(1.0), 0.0);
                    self.goto(3);
                }
                3 => {
                    self.compute_delta_align();
                    self.align(crate::align::opts::Y_VEL, 1.0, 1.0, 1.0);
                    let v = self.control.ctrl_xz_vel * 0.98;
                    self.set_forward_vel(v);
                    if !self.ja_wait() {
                        return None;
                    }
                    // The loop's speed from how far the ground is: 1.5 near, 0.3 far.
                    let ticks = self.time_to_ground() as f32;
                    self.code.z = 1.5 + (0.3 - 1.5) * ((ticks - 30.0) / 180.0).clamp(0.0, 1.0);
                    self.ja_set(anim::DARKJAK_BOMB_LOOP, NumFunc::seek(self.code.z), 0.0);
                    self.goto(4);
                }
                4 => {
                    let v = self.control.ctrl_xz_vel * 0.98;
                    self.set_forward_vel(v);
                    let c = &mut self.control;
                    c.transv += c.gravity_normal * (BOMB_FALL * SECONDS_PER_FRAME);
                    if self.control.on_surface() {
                        self.goto(5);
                        continue;
                    }
                    self.chan.eval_with(&self.anims, NumFunc::seek(self.code.z));
                    return None;
                }
                5 => {
                    self.set_forward_vel(0.0);
                    let to = self.aframe(anim::DARKJAK_BOMB_LAND, 35.0);
                    self.ja_set(anim::DARKJAK_BOMB_LAND, NumFunc::seek_to(to, 1.0), 0.0);
                    self.goto(6);
                }
                6 => {
                    if !self.ja_wait() {
                        return None;
                    }
                    // aframe 35: the blast, from his left hand at his middle's height.
                    self.start_attack();
                    self.danger_set(Some(crate::attack::Danger::Bomb));
                    self.spend_dark(DARK_BOMB_COST);
                    self.bomb.scale = 0.0;
                    let mut at = self.joints.get(crate::lightjak::JOINT_LHAND).map_or(self.control.trans, |&j| self.control.trans + self.control.quat * j);
                    at.y = self.control.trans.y + 4915.2;
                    if let Some(p) = self.particles.as_mut() {
                        p.spawn("group-darkjak-bomb", at, crate::glam::Mat3::IDENTITY);
                    }
                    self.bomb.blast_at = Some((at, self.time));
                    let frame = self.chan.frame;
                    self.ja_set(anim::DARKJAK_BOMB_LAND, NumFunc::seek(1.0), frame);
                    self.goto(7);
                    return None;
                }
                7 => {
                    self.bomb_grow();
                    self.set_forward_vel(0.0);
                    if !self.ja_wait() {
                        return None;
                    }
                    let to = self.aframe(anim::DARKJAK_BOMB_END, 90.0);
                    self.ja_set(anim::DARKJAK_BOMB_END, NumFunc::seek_to(to, 1.0), 0.0);
                    self.goto(8);
                    return None;
                }
                _ => {
                    self.bomb_grow();
                    if !self.ja_wait() {
                        return None;
                    }
                    self.danger_set(None);
                    self.bomb.scale = 0.0;
                    // stand-in: the original changes back (target-darkjak-get-off); Adrii 06/11 wants
                    // him to stay Dark Jak after it.
                    return Some(State::Stance);
                }
            }
        }
    }

    /// target-darkjak-bomb-collide: `bomb-scale` up to 1 at 4 a second.
    fn bomb_grow(&mut self) {
        self.bomb.scale = crate::math::seek(self.bomb.scale, 1.0, 4.0 * crate::math::SECONDS_PER_FRAME);
    }

    /// target.gc's dark uppercut (its own clip to aframe 7), then
    /// `target-darkjak-bomb1`'s code: the rise, the float with its bolts, the end.
    pub(crate) fn dark_blast_code(&mut self) -> Option<State> {
        use crate::anim::{self, NumFunc};
        use crate::math::seconds;
        loop {
            match self.code.pc {
                0 => {
                    self.ja_push(seconds(0.05));
                    let to = self.aframe(anim::DARKJAK_ATTACK_ICE, 7.0);
                    self.ja_set(anim::DARKJAK_ATTACK_ICE, NumFunc::seek_to(to, 1.0), 0.0);
                    self.goto(1);
                }
                1 => {
                    if !self.ja_wait() {
                        return None;
                    }
                    // target-attack-uppercut-jump's enter.
                    self.init_var_jump(crate::target::ATTACK_JUMP_HEIGHT_MIN, crate::target::ATTACK_JUMP_HEIGHT_MAX, true, false, 2.0);
                    self.clear_ground();
                    self.control.mod_surface = crate::surface::foot::UPPERCUT_JUMP;
                    self.compute_delta_align();
                    let frame = self.chan.frame;
                    self.ja_set(anim::DARKJAK_ATTACK_ICE, NumFunc::seek(0.9), frame);
                    self.goto(2);
                    return None;
                }
                2 => {
                    self.compute_delta_align();
                    self.control.turn_go_the_long_way = 1.0;
                    self.align(crate::align::opts::Y_VEL, 1.0, 1.0, 1.0);
                    let c = &self.control;
                    if c.gravity_normal.dot(c.transv) < -8192.0 {
                        self.danger_set(None);
                    }
                    if !self.ja_wait() {
                        return None;
                    }
                    self.danger_set(None);
                    self.spend_dark(DARK_BOMB_COST);
                    self.bomb.blast = Some(Blast { start: self.time, last_shot: self.time, ..Default::default() });
                    self.ja_push(seconds(0.05));
                    self.ja_set(anim::DARKJAK_ATTACK_ICE_LOOP, NumFunc::Loop { rate: 1.0 }, 0.0);
                    self.goto(3);
                    return None;
                }
                3 => {
                    // Floating: no speed along gravity.
                    let c = &mut self.control;
                    let up = c.gravity_normal.dot(c.transv);
                    c.transv -= c.gravity_normal * up;
                    let done = self.bomb.blast.as_ref().is_none_or(|b| {
                        b.searched && b.shots >= b.targets.len() && self.time_elapsed(b.start, seconds(BLAST_FOR))
                    });
                    if done || self.control.on_surface() {
                        self.goto(4);
                        continue;
                    }
                    self.chan.eval_with(&self.anims, NumFunc::Loop { rate: 1.0 });
                    return None;
                }
                4 => {
                    self.bomb.blast = None;
                    self.control.mod_surface = crate::surface::DOUBLE_JUMP;
                    self.ja_push(seconds(0.1));
                    self.ja_set(anim::DARKJAK_ATTACK_ICE_END, NumFunc::seek(1.0), 0.0);
                    self.goto(5);
                }
                _ => {
                    self.compute_delta_align();
                    self.control.turn_go_the_long_way = 1.0;
                    if !self.ja_wait() && !self.control.on_surface() {
                        return None;
                    }
                    // stand-in: Dark Jak still, as after the Bomb (Adrii 06/11).
                    return Some(if self.control.on_surface() { State::Stance } else { State::Falling { uppercut: false } });
                }
            }
        }
    }

    /// The Blast's search (its first frame afloat) and its bolts: one at
    /// each creature found, in turn, a tenth of a second apart. Each is a
    /// blow of 15 that freezes (`'ice`); the bolt itself is his dark
    /// lightning on the victim (`dark_shock.rs`).
    pub(crate) fn dark_blast_frame(&mut self, actors: Option<&mut (dyn crate::projectile::ActorWorld + '_)>) {
        let Some(mut blast) = self.bomb.blast.take() else { return };
        let center = self.control.trans + self.control.gravity_normal * crate::METER;
        if !blast.searched {
            blast.searched = true;
            if let Some(actors) = actors {
                blast.targets = actors.actors_near(center, BLAST_REACH);
            }
        }
        if self.state == State::DarkBlast && self.time_elapsed(blast.last_shot, crate::math::seconds(BLAST_EVERY)) {
            blast.last_shot = self.time;
            if let Some(&(key, pos)) = blast.targets.get(blast.shots) {
                self.attack.id = self.attack.id.wrapping_add(1);
                self.events.push(Event::Strike { actor: Some(key), pos, dir: (pos - center).normalize_or_zero(), damage: BLAST_DAMAGE, sphere: None });
                self.events.push(Event::Freeze { actor: key });
            }
            blast.shots += 1;
        }
        self.bomb.blast = Some(blast);
    }
}

#[cfg(test)]
mod riding_tests {
    use crate::{Jak, glam::Vec3};

    #[test]
    fn no_dark_jak_at_a_vehicles_controls() {
        let mut jak = Jak::new(Vec3::ZERO, 0.0);
        jak.powers_blocked = true;
        jak.dark_eco = 100.0;
        jak.request_dark(true);
        assert!(!jak.dark && jak.dark_transition.is_none(), "driving: no Dark Jak");
        assert!(!jak.want_to_powerjak(), "nor Light Jak");
        // On foot again: both as ever.
        jak.powers_blocked = false;
        jak.dark_eco = 100.0;
        jak.request_dark(true);
        assert!(jak.dark);
    }

    /// Jak 3's `want-to-darkjak?`: no Dark Jak without dark eco; a dark
    /// punch spends `darkjak-punch-inc` (a tenth of it, Adrii 22/10); under
    /// a point he changes back.
    #[test]
    fn dark_jak_runs_on_dark_eco() {
        let mut jak = Jak::new(Vec3::ZERO, 0.0);
        jak.request_dark(true);
        assert!(!jak.dark, "none without dark eco");
        jak.dark_eco = 1.0;
        jak.request_dark(true);
        assert!(jak.dark);
        jak.dark_transition = None;
        jak.spend_dark(super::DARK_PUNCH);
        assert!((jak.dark_eco - 0.6).abs() < 1e-5);
        jak.dark_eco = 0.05;
        jak.dark_eco_frame();
        assert_eq!(jak.dark_transition, Some(false), "changes back with under a point");
    }
}

#[cfg(test)]
mod bomb_tests {
    use crate::{Jak, State, glam::Vec3};

    fn dark(eco: f32) -> Jak {
        let mut jak = Jak::new(Vec3::ZERO, 0.0);
        jak.dark = true;
        jak.dark_eco = eco;
        jak
    }

    /// Jak 3: his dive bombs and his uppercut blasts with darkjak-bomb-min
    /// (5 here); without it they stay a dive and an uppercut.
    #[test]
    fn bomb_and_blast_need_dark_eco() {
        let mut jak = dark(6.0);
        jak.go(State::Flop { forward: false });
        assert_eq!(jak.state, State::DarkBomb);
        let mut jak = dark(6.0);
        jak.go(State::AttackUppercut { min: 1.0, max: 2.0 });
        assert_eq!(jak.state, State::DarkBlast);
        let mut jak = dark(4.0);
        jak.go(State::Flop { forward: false });
        assert!(matches!(jak.state, State::Flop { .. }));
    }

    /// Adrii 06/11: changing back with a sliver left leaves none.
    #[test]
    fn leftover_wiped_on_change_back() {
        let mut jak = dark(0.05);
        jak.request_dark(false);
        assert_eq!(jak.dark_eco, 0.0);
    }
}
