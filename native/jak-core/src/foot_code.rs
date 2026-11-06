//! What each of Jak's moves on foot does over time: the animations it
//! plays, the motion they carry, and the decisions it makes as they play
//! out. Each sequence resumes every frame where it last waited.
use crate::align::opts;
use crate::anim::{self, NumFunc, WalkMix};
use crate::attack::Danger;
use crate::collide::CollideWorld;
use crate::control::STANDARD_GRAVITY;
use crate::math::*;
use crate::pad::button;
use crate::surface;
use crate::target::*;
use crate::{Event, HighJump, Jak, State};

/// How far toward the run the walk cycle mixes at a speed.
fn walk_run(speed: f32) -> f32 {
    ((speed - 16384.0) / 20480.0).clamp(0.0, 1.0)
}

/// The falling loop's steps, shared by every jump.
const FALL: u16 = 200;
const LONG_AGO: i64 = i64::MIN / 4;

impl Jak {
    pub(crate) fn foot_code(&mut self, world: &mut dyn CollideWorld) -> Option<State> {
        match self.state {
            State::Stance => self.foot_stance_code(),
            State::EcoPowerup => self.eco_powerup_code(),
            State::PilotGetOff { bike } => self.pilot_get_off_code(bike),
            State::Walk => self.walk_code(),
            State::TurnAround => self.turn_around_code(),
            State::SlideDown => self.slide_down_code(),
            State::HitGroundHard { height } => self.hit_ground_hard_code(height),
            State::DuckStance { .. } => self.duck_stance_code(),
            State::DuckWalk { .. } => self.duck_walk_code(),
            State::Jump { .. } | State::HighJump { .. } => self.jump_code_foot(),
            State::DoubleJump { .. } => self.double_jump_code(),
            State::DuckHighJump { min, max, kind } => self.duck_high_jump_code(min, max, kind),
            State::DuckHighJumpJump { kind, .. } => self.duck_high_jump_jump_code(kind),
            State::Falling { uppercut } => self.falling_code_foot(uppercut),
            State::EdgeGrab => self.edge_grab_code(),
            State::EdgeGrabJump => self.edge_grab_jump_code(world),
            State::EdgeGrabOff => self.edge_grab_off_code(),
            State::HitGround { .. } => self.hit_ground_code_foot(),
            State::Attack => self.attack_code(),
            State::RunningAttack => self.running_attack_code(),
            State::AttackAir { .. } => self.attack_air_code(),
            State::AttackUppercut { min, max } => self.uppercut_code(min, max),
            State::AttackUppercutJump { .. } => self.uppercut_jump_code(),
            State::Flop { .. } => self.flop_code(),
            State::DarkBomb => self.dark_bomb_code(),
            State::DarkBlast => self.dark_blast_code(),
            State::FlopHitGround { stuck } => self.flop_hit_ground_code(stuck),
            State::Roll => self.roll_code(),
            State::RollFlip { height, dist } => self.roll_flip_code(height, dist, world),
            State::PowerJakGetOn
            | State::LightJakGetOn { .. }
            | State::LightJakGetOff
            | State::LightJakRegen
            | State::LightJakShield
            | State::LightJakFreeze
            | State::LightJakSwoop { .. }
            | State::LightJakSwoopFalling => self.lightjak_code(),
            _ => None,
        }
    }

    fn pilot_get_off_code(&mut self, bike: bool) -> Option<State> {
        let anim = if bike { anim::PILOT_BIKE_GET_OFF } else { anim::PILOT_CAR_GET_OFF };
        let cutoff = self.anims.aframe(anim, if bike { 7.2 } else { 15.2 });
        if self.code.pc == 0 {
            self.ja_push(seconds(0.05));
            self.ja_set(anim, NumFunc::seek_to(cutoff, 2.0), 0.0);
            self.goto(1);
        }
        // target-pilot-post: on the seat, going as the vehicle goes, until the animation has played.
        self.control.transv = self.foot.pilot_velocity;
        // (until (ja-done? 0) (suspend) (ja :num! (seek! cutoff 2.0)))
        if self.ja_wait() {
            self.control.transv += self.pad_dir_world() * meters(4.0);
            self.control.transv = self.control.transv.clamp_length_max(meters(30.0));
            // (go target-jump 10240.0 10240.0 *pilot-get-off-mods*)
            self.foot.pilot_exit = true;
            return Some(State::Jump { min: meters(2.5), max: meters(2.5) });
        }
        None
    }

    fn eco_powerup_code(&mut self) -> Option<State> {
        if self.code.pc == 0 {
            self.ja_push(seconds(0.05));
            self.ja_set(anim::POWERUP, NumFunc::seek(2.0), self.anims.aframe(anim::POWERUP, 0.0));
            self.goto(1);
        } else if self.chan.done(&self.anims) {
            return Some(State::Falling { uppercut: false });
        }
        self.compute_delta_align();
        self.align(opts::Y_VEL, 1.0, 2.0, 1.0);
        self.ja_loop()
    }

    fn up_speed(&self) -> f32 {
        self.control.gravity_normal.dot(self.control.transv)
    }

    fn aframe_num(&self) -> f32 {
        self.chan.aframe_num(&self.anims)
    }

    fn at_max(&self) -> bool {
        self.chan.frame == self.anims.max(self.chan.anim)
    }

    /// The jump animations' playback: faster the sooner the top, so the
    /// animation's own top meets the jump's.
    fn rise_rate(&self, apex: f32, otherwise: f32) -> f32 {
        let up = self.up_speed();
        let left = apex - self.aframe_num();
        if 0.0 < up && 0.0 < left {
            let to_apex = (up / (245760.0 / 300.0)).trunc();
            3.0f32.min(left).min(5.0 * left / to_apex)
        } else {
            otherwise
        }
    }

    /// Into the falling loop and on in it while the state lasts.
    fn falling_anim(&mut self, push: i64) -> Option<State> {
        loop {
            match self.code.pc {
                FALL => {
                    if !self.chan.is_any(&[anim::JUMP_LOOP, anim::ATTACK_UPPERCUT]) {
                        self.ja_push(push);
                    }
                    self.ja_set(anim::JUMP_LOOP, NumFunc::Loop { rate: 1.0 }, 0.0);
                    self.goto(FALL + 1);
                }
                _ => return self.ja_loop(),
            }
        }
    }

    /// Standing: first the end of whatever move came before, then the
    /// stance loop.
    fn foot_stance_code(&mut self) -> Option<State> {
        loop {
            match self.code.pc {
                0 => {
                    self.code.count = 22;
                    if self.chan.is(anim::ROLL_FLIP) {
                        self.ja_play(anim::ROLL_FLIP_LAND, 1.0);
                        self.goto(1);
                    } else if self.chan.is_any(&[
                        anim::ATTACK_FROM_STANCE_END,
                        anim::ATTACK_FROM_STANCE_ALT_END,
                    ]) {
                        self.chan.func = NumFunc::seek(1.0);
                        self.goto(3);
                    } else if self.chan.is(anim::ATTACK_FROM_STANCE) {
                        self.control.mod_surface = surface::foot::ATTACK_END;
                        let end = if self.foot.chance(0.3) {
                            anim::ATTACK_FROM_STANCE_ALT_END
                        } else {
                            anim::ATTACK_FROM_STANCE_END
                        };
                        self.ja_play(end, 1.0);
                        self.goto(4);
                    } else if self.chan.is(anim::ATTACK_PUNCH) {
                        self.control.bend_target = self.control.bend_amount;
                        self.control.mod_surface = surface::foot::WALK_NO_TURN;
                        let end = if self.foot.chance(0.3) {
                            anim::ATTACK_PUNCH_ALT_END
                        } else {
                            anim::ATTACK_PUNCH_END
                        };
                        self.ja_play(end, 1.0);
                        self.goto(5);
                    } else if self.chan.is_any(&[
                        anim::DUCK_STANCE,
                        anim::DUCK_WALK,
                        anim::DUCK_ROLL,
                    ]) {
                        self.ja_push(seconds(0.04));
                        let max = self.anims.max(anim::STANCE_TO_DUCK);
                        self.ja_set(anim::STANCE_TO_DUCK, NumFunc::seek_to(0.0, 1.2), max);
                        self.goto(3);
                    } else if self.light() && self.chan.is(anim::LIGHTJAK_GET_ON_LAND) {
                        self.code.count = 45;
                        self.goto(9);
                    } else if let Some(mix) = self.chan.mix {
                        let end = if mix.run > 0.75 {
                            anim::RUN_TO_STANCE_FAST
                        } else {
                            anim::RUN_TO_STANCE
                        };
                        self.ja_push(seconds(0.1));
                        self.ja_play(end, 1.0);
                        self.goto(3);
                    } else {
                        self.goto(9);
                    }
                }
                1 => {
                    if !self.ja_wait() {
                        return None;
                    }
                    let from = self.aframe(anim::JUMP_LAND, 55.0);
                    self.ja_set(anim::JUMP_LAND, NumFunc::seek(1.0), from);
                    self.goto(2);
                }
                2 => {
                    if !self.ja_wait() {
                        return None;
                    }
                    self.goto(9);
                }
                3 => {
                    if !self.ja_while() {
                        return None;
                    }
                    self.goto(9);
                }
                4 => {
                    if self.ja_wait() {
                        self.control.dir_targ = self.foot.saved_dir;
                        self.control.mod_surface = surface::WALK;
                        self.goto(9);
                        continue;
                    }
                    self.compute_delta_align();
                    self.align(opts::QUAT, 1.0, 1.0, 1.0);
                    return None;
                }
                5 => {
                    if self.ja_wait() {
                        self.control.mod_surface = surface::WALK;
                        self.control.bend_target = 0.0;
                        self.control.dir_targ = self.control.quat;
                        self.goto(9);
                        continue;
                    }
                    let c = &mut self.control;
                    c.bend_target = seek(c.bend_target, 0.0, SECONDS_PER_FRAME);
                    return None;
                }
                9 => {
                    if self.light_stance() == Some(true) {
                        self.ja_push(seconds(0.05));
                        self.ja_play(anim::LIGHTJAK_STANCE_TO_STANCE, 1.0);
                        self.goto(12);
                    } else {
                        self.goto(10);
                    }
                }
                10 => {
                    let stance = if self.light_stance().is_some() {
                        anim::LIGHTJAK_STANCE
                    } else {
                        anim::STANCE_LOOP
                    };
                    if !self.chan.is(stance) || self.chan.mix.is_some() {
                        self.ja_push(i64::from(self.code.count));
                        self.chan.anim = stance;
                        self.chan.mix = None;
                    }
                    self.goto(11);
                }
                12 => {
                    if !self.code.arrived {
                        self.chan.eval(&self.anims);
                        self.code.arrived = true;
                        if self.chan.done(&self.anims) {
                            self.goto(10);
                            continue;
                        }
                    }
                    if !self.light() {
                        self.goto(10);
                        continue;
                    }
                    return None;
                }
                _ => return self.ja_loop(),
            }
        }
    }

    /// Walking and running: the end of the move before it, then the walk
    /// cycle's channels at the pace the ground covered sets.
    fn walk_code(&mut self) -> Option<State> {
        loop {
            match self.code.pc {
                0 => {
                    let speed = self.control.ctrl_xz_vel;
                    let mut mix = WalkMix {
                        run: walk_run(speed),
                        ..WalkMix::default()
                    };
                    let mut from = 0.0;
                    let ch = self.chan;
                    let landing = ch.is_any(&[
                        anim::JUMP_LOOP,
                        anim::ROLL_FLIP,
                        anim::ATTACK_FROM_JUMP_END,
                        anim::ATTACK_UPPERCUT,
                        anim::FLOP_DOWN_LAND,
                    ]) || (ch.is(anim::JUMP) && 30.0 < self.aframe_num());
                    if ch.is(anim::TURN_AROUND) {
                        mix.run = 1.0;
                        self.ja_push(seconds(0.05));
                    } else if ch.is(anim::DUCK_ROLL) {
                        self.ja_push(seconds(0.075));
                        mix.run = 1.0;
                    } else if ch.is(anim::ATTACK_FROM_STANCE) {
                        self.code.rate = (speed / meters(5.0)).clamp(0.8, 1.0);
                        self.code.flag = self.foot.chance(0.3) && 20480.0 < speed;
                        let end = if self.code.flag {
                            anim::ATTACK_FROM_STANCE_ALT_END
                        } else {
                            anim::ATTACK_FROM_STANCE_END
                        };
                        let to = self.aframe(end, 29.0);
                        self.ja_set(end, NumFunc::seek_to(to, self.code.rate), 0.0);
                        self.goto(1);
                        continue;
                    } else if ch.is_any(&[anim::ATTACK_PUNCH, anim::ATTACK_PUNCH_END]) {
                        mix.run = 1.0;
                        from = 30.0;
                        self.ja_push(seconds(0.15));
                    } else if let (true, Some(m)) = (ch.is(anim::WALK), ch.mix) {
                        from = self.aframe_num();
                        mix = m;
                    } else if landing && 12288.0 < speed {
                        let jump = ch.is(anim::JUMP);
                        if ch.is_any(&[anim::ROLL_FLIP, anim::ATTACK_FROM_JUMP_END]) {
                            self.ja_push(seconds(0.05));
                        }
                        let impact = self.control.ground_impact_vel;
                        let (hard, firm) = if jump {
                            (77824.0, 61440.0)
                        } else {
                            (102400.0, 102400.0)
                        };
                        let (squash, to, start) = if hard < impact {
                            (anim::RUN_SQUASH, 3.0, None)
                        } else if firm < impact {
                            (anim::RUN_SQUASH, 3.0, Some(-1.0))
                        } else {
                            (anim::RUN_SQUASH_WEAK, 4.0, None)
                        };
                        let to = self.aframe(squash, to);
                        let start = start.map_or(0.0, |a| self.aframe(squash, a));
                        self.ja_set(squash, NumFunc::seek_to(to, 1.00001), start);
                        self.goto(3);
                        continue;
                    } else if ch.is(anim::SMACK_SURFACE) {
                        self.ja_push(seconds(0.15));
                    } else {
                        self.ja_push(seconds(0.05));
                    }
                    self.walk_start(mix, from);
                    self.goto(10);
                }
                1 => {
                    if !self.ja_wait() {
                        return None;
                    }
                    let end = if self.code.flag {
                        anim::ATTACK_FROM_STANCE_RUN_ALT_END
                    } else {
                        anim::ATTACK_FROM_STANCE_RUN_END
                    };
                    self.ja_set(end, NumFunc::seek(self.code.rate), 0.0);
                    self.goto(2);
                }
                2 => {
                    if !self.ja_wait() {
                        return None;
                    }
                    self.ja_push(seconds(0.05));
                    self.walk_start(
                        WalkMix {
                            run: 1.0,
                            ..WalkMix::default()
                        },
                        30.0,
                    );
                    self.goto(10);
                }
                3 => {
                    if !self.ja_wait() {
                        return None;
                    }
                    self.goto(4);
                }
                4 => {
                    if self.code.arrived {
                        return None;
                    }
                    let pace = self.control.ctrl_xz_vel.max(20480.0) * SECONDS_PER_FRAME;
                    let rate = pace / (cycle::RUN_UP / RUN_CYCLE_LENGTH);
                    self.chan.eval_with(&self.anims, NumFunc::seek(rate));
                    self.code.arrived = true;
                    if !self.chan.done(&self.anims) {
                        return None;
                    }
                    self.ja_push(seconds(0.1));
                    self.walk_start(
                        WalkMix {
                            run: 1.0,
                            ..WalkMix::default()
                        },
                        30.0,
                    );
                    self.goto(10);
                }
                _ => {
                    self.walk_cycle();
                    return None;
                }
            }
        }
    }

    /// The walk cycle's channels, from an artist frame of the walk.
    fn walk_start(&mut self, mix: WalkMix, from: f32) {
        let frame = self.aframe(anim::WALK, from);
        self.ja_set(anim::WALK, NumFunc::Identity, frame);
        self.chan.mix = Some(mix);
    }

    /// One frame of the walk cycle: the run eases in with the speed, the
    /// slope and side leans with the ground, and the cycle advances by the
    /// ground covered against the distance the mixed cycles cover.
    fn walk_cycle(&mut self) {
        let c = &self.control;
        // (A step just climbed leans it uphill too, `step`.)
        let up = (2.0 * c.local_slope_z).clamp(-1.0, 1.0).max(self.step_lean);
        let side = (1.6 * c.local_slope_x).clamp(-1.0, 1.0);
        let speed = c.ctrl_xz_vel;
        let mut mix = self.chan.mix.unwrap_or(WalkMix::default());
        mix.run = seek(mix.run, walk_run(speed), 2.0 * SECONDS_PER_FRAME);
        mix.up = seek(mix.up, up, ((up - mix.up).abs() / 4.0).clamp(0.05, 0.2));
        mix.side = seek(
            mix.side,
            side,
            ((side - mix.side).abs() / 4.0).clamp(0.05, 0.2),
        );
        let rate = speed / (60.0 * (mix.cycle_dist() / RUN_CYCLE_LENGTH));
        self.chan.eval_with(&self.anims, NumFunc::Loop { rate });
        self.chan.mix = Some(mix);
    }

    /// The skid round: the turn-around animation turns Jak half a turn by
    /// its own motion, and he sets off the other way at a walk.
    fn turn_around_code(&mut self) -> Option<State> {
        if self.code.pc == 0 {
            self.ja_push(seconds(0.04));
            self.ja_set(anim::TURN_AROUND, NumFunc::seek(2.0), 0.0);
            let c = &mut self.control;
            c.dir_targ =
                (glam::Quat::from_rotation_y(to_radians(32768.0)) * c.dir_targ).normalize();
            self.compute_delta_align();
            self.goto(1);
        }
        if self.code.arrived {
            return None;
        }
        self.chan.eval(&self.anims);
        self.code.arrived = true;
        self.compute_delta_align();
        self.align(opts::QUAT, 1.0, 1.0, 1.0);
        if !self.chan.done(&self.anims) {
            return None;
        }
        self.code.no_exit = true;
        self.control.bend_target = 0.0;
        self.control.ctrl_xz_vel = 40960.0;
        self.set_forward_vel(40960.0);
        self.foot.hook = StateHook::None;
        Some(State::Walk)
    }

    /// Sliding down: crouched, the duck stance over and over.
    fn slide_down_code(&mut self) -> Option<State> {
        loop {
            match self.code.pc {
                0 => {
                    if !self.chan.is(anim::DUCK_STANCE) {
                        self.ja_push(seconds(0.1));
                    }
                    self.goto(1);
                }
                1 => {
                    self.ja_play(anim::DUCK_STANCE, 1.0);
                    self.goto(2);
                }
                _ => {
                    if !self.ja_wait() {
                        return None;
                    }
                    self.goto(1);
                }
            }
        }
    }

    /// A fall too high: the health it costs, then the painful landing and
    /// getting up, with no way out until both have played.
    fn hit_ground_hard_code(&mut self, height: f32) -> Option<State> {
        loop {
            match self.code.pc {
                0 => {
                    if height != 0.0 {
                        let lost = (1.0 + (height - FALL_FAR) / FALL_FAR_INC).trunc().max(0.0);
                        self.events.push(Event::HardLanding { health: lost });
                    }
                    self.ja_push(1);
                    self.ja_set(anim::PAINFUL_LAND, NumFunc::seek(1.0), 0.0);
                    self.goto(1);
                }
                1 => {
                    if !self.ja_wait() {
                        return None;
                    }
                    self.ja_set(anim::PAINFUL_LAND_END, NumFunc::seek(1.0), 0.0);
                    self.goto(2);
                }
                _ => {
                    if !self.ja_wait() {
                        return None;
                    }
                    return Some(State::Stance);
                }
            }
        }
    }

    fn duck_stance_code(&mut self) -> Option<State> {
        loop {
            match self.code.pc {
                0 => {
                    if self.chan.is(anim::DUCK_ROLL) {
                        self.ja_play(anim::DUCK_ROLL_END, 1.0);
                        self.goto(1);
                    } else if self.chan.is(anim::DUCK_STANCE) {
                        self.goto(2);
                    } else if self.chan.is(anim::DUCK_WALK) {
                        self.ja_push(seconds(0.1));
                        self.goto(2);
                    } else {
                        self.ja_push(seconds(0.04));
                        self.ja_play(anim::STANCE_TO_DUCK, 1.0);
                        self.goto(1);
                    }
                }
                1 => {
                    if !self.ja_wait() {
                        return None;
                    }
                    self.goto(2);
                }
                2 => {
                    self.ja_play(anim::DUCK_STANCE, 1.0);
                    self.goto(3);
                }
                _ => {
                    if !self.ja_wait() {
                        return None;
                    }
                    self.goto(2);
                }
            }
        }
    }

    fn duck_walk_code(&mut self) -> Option<State> {
        if self.code.pc == 0 {
            if !self.chan.is(anim::DUCK_WALK) {
                let push = if self.chan.is(anim::DUCK_STANCE) {
                    seconds(0.45)
                } else {
                    seconds(0.1)
                };
                self.ja_push(push);
                self.ja_set(anim::DUCK_WALK, NumFunc::Identity, 0.0);
            }
            self.code.pc = 1;
        }
        if self.time >= self.push.tick + self.push.ticks {
            self.control.mod_surface = surface::foot::DUCK;
        }
        let rate = (self.control.ctrl_xz_vel / (60.0 * (DUCK_WALK_CYCLE_DIST / RUN_CYCLE_LENGTH)))
            .min(3.0);
        self.chan.eval_with(&self.anims, NumFunc::Loop { rate });
        None
    }

    /// The jump and the high jumps: the jump played up to its top at the
    /// rate the rise leaves, then the falling loop. Out of a dive's landing
    /// the bounce plays instead.
    fn jump_code_foot(&mut self) -> Option<State> {
        loop {
            match self.code.pc {
                0 => {
                    let aframe = self.aframe_num();
                    if self.chan.is(anim::FLOP_DOWN_LAND) && (17.0..=25.0).contains(&aframe) {
                        self.ja_push(0);
                        let at = self.aframe(anim::FLOP_JUMP, 25.0);
                        self.ja_set(anim::FLOP_JUMP, NumFunc::Identity, at);
                        self.code.z = 24.0;
                        self.code.x = 0.3;
                        self.goto(2);
                    } else {
                        self.ja_push(seconds(0.05));
                        self.ja_set(anim::JUMP, NumFunc::Identity, 0.0);
                        self.code.z = 20.0;
                        self.code.x = 1.0;
                        self.code.pc = 1;
                        return None;
                    }
                }
                1 => {
                    let speed = self.anims.info(anim::JUMP).speed;
                    self.chan.frame += speed;
                    self.code.pc = 2;
                    return None;
                }
                2 => {
                    let rate = self.rise_rate(self.code.z, self.code.x);
                    self.chan.eval_with(&self.anims, NumFunc::seek(rate));
                    self.code.pc = 3;
                    return None;
                }
                3 => {
                    if self.chan.done(&self.anims) {
                        self.goto(FALL);
                    } else {
                        self.goto(2);
                    }
                }
                _ => return self.falling_anim(seconds(0.2)),
            }
        }
    }

    fn double_jump_code(&mut self) -> Option<State> {
        loop {
            match self.code.pc {
                0 => {
                    self.ja_push(seconds(0.05));
                    let from = self.aframe(anim::JUMP, 5.0);
                    self.ja_set(anim::JUMP, NumFunc::seek(1.0), from);
                    self.goto(1);
                }
                1 => {
                    if !self.ja_wait() {
                        return None;
                    }
                    self.goto(FALL);
                }
                _ => return self.falling_anim(seconds(0.2)),
            }
        }
    }

    /// The crouch before the high jump, then the jump itself.
    fn duck_high_jump_code(&mut self, min: f32, max: f32, kind: HighJump) -> Option<State> {
        loop {
            match self.code.pc {
                0 if kind == HighJump::Launch => {
                    // target-launch: still, the duck stance's crouch to aframe 15 at
                    // three times its speed, then the high jump's own crouch.
                    self.ja_push(seconds(0.15));
                    self.set_forward_vel(0.0);
                    let to = self.aframe(anim::DUCK_STANCE, 15.0);
                    self.ja_set(anim::DUCK_STANCE, NumFunc::seek_to(to, 3.0), 0.0);
                    self.goto(10);
                }
                10 => {
                    if !self.ja_wait() {
                        return None;
                    }
                    let to = self.aframe(anim::DUCK_HIGH_JUMP, 16.0);
                    self.ja_set(anim::DUCK_HIGH_JUMP, NumFunc::seek_to(to, 1.0), 0.0);
                    self.goto(1);
                }
                0 => {
                    if !self.chan.is(anim::DUCK_STANCE) {
                        self.ja_push(seconds(0.04));
                    }
                    let (art, to) = if matches!(kind, HighJump::Flop | HighJump::FlopForward) {
                        (anim::FLOP_JUMP, 25.0)
                    } else {
                        (anim::DUCK_HIGH_JUMP, 16.0)
                    };
                    let to = self.aframe(art, to);
                    self.ja_set(art, NumFunc::seek_to(to, 1.0), 0.0);
                    self.goto(1);
                }
                _ => {
                    if !self.ja_wait() {
                        return None;
                    }
                    return Some(State::DuckHighJumpJump { min, max, kind });
                }
            }
        }
    }

    fn duck_high_jump_jump_code(&mut self, kind: HighJump) -> Option<State> {
        let flop = matches!(kind, HighJump::Flop | HighJump::FlopForward);
        loop {
            match self.code.pc {
                0 => {
                    self.code.z = if flop { 44.0 } else if kind == HighJump::Launch { 110.0 } else { 35.0 };
                    self.code.x = if flop { 0.75 } else { 1.0 };
                    self.goto(1);
                }
                1 => {
                    let rising = self.rise_rate(self.code.z, f32::NAN);
                    let rate = if rising.is_nan() {
                        self.code.x
                    } else {
                        // 'launch: the rate kept for after the rise eases to four times this one.
                        if kind == HighJump::Launch {
                            self.code.x += (4.0 * rising - self.code.x) * 0.25;
                        }
                        rising
                    };
                    self.chan.eval_with(&self.anims, NumFunc::seek(rate));
                    self.code.pc = 2;
                    return None;
                }
                2 => {
                    if !self.chan.done(&self.anims) {
                        self.goto(1);
                        continue;
                    }
                    if flop {
                        self.ja_push(seconds(0.5));
                    }
                    self.ja_set(anim::JUMP_LOOP, NumFunc::Loop { rate: 1.0 }, 0.0);
                    self.goto(3);
                }
                _ => return self.ja_loop(),
            }
        }
    }

    fn falling_code_foot(&mut self, uppercut: bool) -> Option<State> {
        loop {
            match self.code.pc {
                0 => {
                    if uppercut {
                        self.chan.func = NumFunc::seek(1.0);
                        self.goto(1);
                    } else {
                        self.goto(FALL);
                    }
                }
                1 => {
                    if !self.ja_while() {
                        return None;
                    }
                    self.goto(FALL);
                }
                _ => return self.falling_anim(seconds(0.33)),
            }
        }
    }

    /// The landing that fits what Jak landed out of, then standing.
    fn hit_ground_code_foot(&mut self) -> Option<State> {
        loop {
            match self.code.pc {
                0 => {
                    let aframe = self.aframe_num();
                    let c = self.chan;
                    if self.landing_from_swoop() {
                        self.ja_push(seconds(0.05));
                        self.ja_play(anim::LIGHTJAK_SWOOP_LAND, 1.0);
                        self.goto(3);
                    } else if c.is_any(&[anim::JUMP_LOOP, anim::FLOP_JUMP])
                        || (c.is(anim::JUMP) && aframe >= 38.0)
                    {
                        self.ja_push(seconds(0.02));
                        self.ja_play(anim::JUMP_LAND, 1.0);
                        self.goto(3);
                    } else if c.is(anim::JUMP) && aframe >= 35.0 {
                        self.ja_play(anim::JUMP_SHORT_LAND, 1.0);
                        self.goto(1);
                    } else if c.is_any(&[anim::JUMP, anim::DUCK_HIGH_JUMP]) {
                        let from = self.aframe(anim::JUMP_SHORT_LAND, 38.0);
                        self.ja_set(anim::JUMP_SHORT_LAND, NumFunc::seek(1.0), from);
                        self.goto(1);
                    } else if c.is_any(&[
                        anim::ATTACK_FROM_JUMP,
                        anim::ATTACK_FROM_JUMP_LOOP,
                        anim::ATTACK_FROM_JUMP_END,
                        anim::FLOP_DOWN_LAND,
                        anim::ATTACK_UPPERCUT,
                        anim::BOARD_GET_OFF,
                    ]) {
                        self.ja_push(seconds(0.04));
                        let from = self.aframe(anim::JUMP_LAND, 42.0);
                        self.ja_set(anim::JUMP_LAND, NumFunc::seek(1.0), from);
                        self.goto(3);
                    } else if c.is(anim::JUMP_LAND) {
                        self.chan.func = NumFunc::seek(1.0);
                        self.goto(4);
                    } else {
                        return Some(State::Stance);
                    }
                }
                1 => {
                    if !self.ja_wait() {
                        return None;
                    }
                    let from = self.aframe(anim::JUMP_LAND, 50.0);
                    self.ja_set(anim::JUMP_LAND, NumFunc::seek(1.0), from);
                    self.goto(3);
                }
                3 => {
                    if !self.ja_wait() {
                        return None;
                    }
                    return Some(State::Stance);
                }
                _ => {
                    if !self.ja_while() {
                        return None;
                    }
                    return Some(State::Stance);
                }
            }
        }
    }

    /// The spin: a full turn the animation carries, which X may cut short
    /// with a jump at any point.
    fn attack_code(&mut self) -> Option<State> {
        loop {
            match self.code.pc {
                0 => {
                    self.rotate_heading(-1365.3334);
                    self.ja_push(seconds(0.05));
                    let rate = self.control.current.align_speed;
                    self.ja_play(anim::ATTACK_FROM_STANCE, rate);
                    self.goto(1);
                }
                1 => {
                    if self.ja_wait() {
                        self.rotate_heading(-1365.3334);
                        return Some(State::Stance);
                    }
                    self.compute_delta_align();
                    self.align(opts::QUAT, 1.0, 1.0, 1.0);
                    self.control.mod_surface = surface::foot::ATTACK;
                    if self.pad.recently_pressed(button::X) && self.can_jump(false) {
                        self.control.quat = self.control.dir_targ;
                        return Some(State::Jump {
                            min: JUMP_HEIGHT_MIN,
                            max: JUMP_HEIGHT_MAX,
                        });
                    }
                    return None;
                }
                _ => return None,
            }
        }
    }

    /// The punch: a lunge as far as its animation carries, slowed when the
    /// button comes up, cut off by a wall it bounces back from.
    fn running_attack_code(&mut self) -> Option<State> {
        if self.dark && self.code.pc == 1 {
            if self.pad.pressed(button::SQUARE) && self.dark_combo < 2 {
                self.dark_combo_buffer = Some(self.time);
            }
            if self
                .dark_combo_buffer
                .is_some_and(|time| self.time_elapsed(time, seconds(0.5)))
            {
                self.dark_combo_buffer = None;
            }
            let step = self
                .anims
                .info(self.chan.anim)
                .artist_step
                .abs()
                .max(f32::EPSILON);
            if self.dark_combo_buffer.is_some()
                && self.chan.frame > self.anims.max(self.chan.anim) - 2.0 / step
            {
                self.dark_combo_buffer = None;
                self.dark_combo += 1;
                // Adrii 06/11: his blows cost no dark eco, only the Bomb and the Blast
                // (OpenGOAL spends darkjak-punch-inc here).
                self.ja_push(seconds(0.1));
                self.ja_play(
                    if self.dark_combo == 1 {
                        anim::DARKJAK_COMBO2
                    } else {
                        anim::DARKJAK_COMBO3
                    },
                    1.0,
                );
                self.start_attack();
                self.danger_set(Some(Danger::Punch));
            }
        }
        loop {
            match self.code.pc {
                0 => {
                    self.control.gravity_max = 368640.0;
                    self.control.gravity_length = 368640.0;
                    self.ja_push(seconds(0.02));
                    let punch = if self.dark {
                        self.dark_combo = 0;
                        self.dark_combo_buffer = None;
                        self.dark_combo_alternate = !self.dark_combo_alternate;
                        if self.dark_combo_alternate {
                            anim::DARKJAK_COMBO1
                        } else {
                            anim::DARKJAK_COMBO1B
                        }
                    } else {
                        anim::ATTACK_PUNCH
                    };
                    self.ja_play(punch, 1.0);
                    self.start_attack();
                    self.danger_set(Some(Danger::Punch));
                    self.foot.punch_speed = 0.0;
                    self.code.x = 1.0;
                    self.code.count = 0;
                    self.goto(1);
                }
                1 => {
                    if !self.code.arrived {
                        let rate = self.control.current.align_speed;
                        self.chan.eval_with(&self.anims, NumFunc::seek(rate));
                        self.code.arrived = true;
                        if self.time_elapsed(self.state_time, seconds(0.1)) {
                            self.foot.run_attack.turnvv = 0.0;
                            self.control.mod_surface.turnvv = 0.0;
                        }
                        if 2 < self.code.count {
                            self.code.x *= self.control.zx_vel_frac.min(1.0);
                        }
                        self.code.count += 1;
                        if self.chan.done(&self.anims) {
                            self.goto(2);
                            continue;
                        }
                    }
                    if let Some(next) = self.punch_body() {
                        return Some(next);
                    }
                    return None;
                }
                _ => {
                    if self.fall_test_foot() {
                        return Some(State::Falling { uppercut: false });
                    }
                    return Some(State::Stance);
                }
            }
        }
    }

    pub(crate) fn fall_test_foot(&self) -> bool {
        let c = &self.control;
        !c.on_surface()
            && self.time_elapsed(c.last_time_on_surface, crate::control::GROUND_TIMEOUT)
            && c.gravity_normal.dot(c.transv) <= 0.0
            && c.height_above_ground() >= FALL_HEIGHT
    }

    fn punch_body(&mut self) -> Option<State> {
        self.compute_delta_align();
        if self.chan.frame != 0.0 {
            let speed = self.foot.punch_speed;
            if self.aframe_num() >= 20.0
                && self.fall_test_foot()
                && self.time_elapsed(self.foot.sliding_start_time, seconds(0.04))
            {
                return Some(State::Falling { uppercut: false });
            } else if speed < 0.0 {
                self.foot.punch_speed = seek(speed, -0.04096, 491520.0 * SECONDS_PER_FRAME);
                self.set_forward_vel(self.foot.punch_speed);
            } else if self.foot.smack_time != 0
                && self.time_elapsed(self.foot.smack_time, seconds(0.04))
            {
                self.set_forward_vel(0.0);
            } else if !self.pad.hold(button::SQUARE)
                && (!self.dark || self.dark_combo == 0)
                && self.time_elapsed(
                    self.foot.move_start_time,
                    seconds(if self.dark { 0.2 } else { 0.05 }),
                )
            {
                let v = self.control.ctrl_xz_vel;
                if self.control.ground_pat.material.is_ice() {
                    self.set_forward_vel((0.8 * v).max(32768.0));
                } else {
                    self.set_forward_vel(0.8 * v);
                }
            } else if self.chan.done(&self.anims) {
                self.set_forward_vel(speed);
            } else {
                let z = self.align.delta_trans.z;
                let slope = self.control.local_slope_z;
                let alignv = self.control.current.alignv;
                let adjusted = z * if 0.0 < slope {
                    (1.0 - slope) * alignv
                } else {
                    alignv
                };
                self.foot.punch_speed = adjusted * 60.0 * self.code.x;
                self.set_forward_vel(self.foot.punch_speed);
            }
        }
        self.keep_align_xz_vel();
        if self.foot.punch_bounce != 0.0 {
            self.foot.punch_speed = self.foot.punch_bounce;
            self.foot.punch_bounce = 0.0;
        }
        // The fist into a wall: bounced back.
        if self.foot.punch_speed >= 0.0
            && self.chan.is(anim::ATTACK_PUNCH)
            && self.aframe_num() >= 10.0
            && self.control.status & crate::control::status::TOUCH_WALL != 0
            && self
                .control
                .wall_contact_normal
                .dot(z_axis(self.control.quat))
                < -0.7
        {
            self.foot.punch_speed = -61440.0;
            self.events.push(crate::Event::PunchWall);
        }
        None
    }

    /// The spin in the air: the turn, spinning on until near the ground,
    /// then its end facing round to where Jak heads.
    fn attack_air_code(&mut self) -> Option<State> {
        let spin = -393216.0;
        loop {
            match self.code.pc {
                0 => {
                    self.ja_push(22);
                    self.ja_play(anim::ATTACK_FROM_JUMP, 1.0);
                    self.goto(1);
                }
                1 => {
                    if self.ja_wait() {
                        self.ja_set(anim::ATTACK_FROM_JUMP_LOOP, NumFunc::Identity, 0.0);
                        self.goto(2);
                        continue;
                    }
                    self.compute_delta_align();
                    self.align(opts::QUAT, 1.0, 1.0, 1.0);
                    return None;
                }
                2 => {
                    let h = self.control.height_above_ground();
                    let up = self.up_speed();
                    let near = (h / (up / 300.0)).abs() < 150.0 && up < 0.0;
                    if near || self.time_elapsed(self.state_time, seconds(1.7)) {
                        self.ja_play(anim::ATTACK_FROM_JUMP_END, 1.0);
                        self.goto(4);
                        continue;
                    }
                    self.rotate_heading(spin * SECONDS_PER_FRAME);
                    self.code.pc = 3;
                    return None;
                }
                3 => {
                    self.chan
                        .eval_with(&self.anims, NumFunc::Loop { rate: 1.0 });
                    self.goto(2);
                }
                _ => {
                    if self.ja_wait() {
                        return Some(State::Falling { uppercut: false });
                    }
                    if self.aframe_num() < 32.0 {
                        self.rotate_heading(spin * SECONDS_PER_FRAME);
                    } else {
                        let off = deg_diff(
                            quat_y_angle(self.control.quat),
                            quat_y_angle(self.control.dir_targ),
                        );
                        self.rotate_heading(-0.2 * off.abs());
                    }
                    return None;
                }
            }
        }
    }

    /// The uppercut's crouch, then its jump.
    fn uppercut_code(&mut self, min: f32, max: f32) -> Option<State> {
        loop {
            match self.code.pc {
                0 => {
                    let start = if self.chan.is(anim::DUCK_STANCE) {
                        5.0
                    } else {
                        0.0
                    };
                    let to = self.aframe(anim::ATTACK_UPPERCUT, 7.0);
                    let from = self.aframe(anim::ATTACK_UPPERCUT, start);
                    self.ja_set(anim::ATTACK_UPPERCUT, NumFunc::seek_to(to, 1.0), from);
                    self.goto(1);
                }
                _ => {
                    if !self.ja_wait() {
                        return None;
                    }
                    return Some(State::AttackUppercutJump { min, max });
                }
            }
        }
    }

    /// The uppercut's rise: the animation's own lift, then a double jump's
    /// control, then the fall.
    fn uppercut_jump_code(&mut self) -> Option<State> {
        if self.code.pc == 0 {
            self.compute_delta_align();
            self.code.pc = 1;
            return None;
        }
        self.chan.eval_with(&self.anims, NumFunc::seek(0.9));
        self.compute_delta_align();
        self.control.turn_go_the_long_way = 1.0;
        let aframe = self.aframe_num();
        let lift = if self.chan.is(anim::ATTACK_UPPERCUT) {
            30.0
        } else {
            35.0
        };
        let flags = if aframe <= lift {
            opts::Y_VEL | opts::QUAT
        } else if aframe <= 43.0 {
            self.control.mod_surface = surface::DOUBLE_JUMP;
            opts::Y_VEL
        } else {
            0
        };
        self.align(flags, 1.0, 0.95, 1.0);
        if self.chan.done(&self.anims) {
            return Some(State::Falling { uppercut: false });
        }
        None
    }

    /// The dive: the strike, then driven straight down.
    fn flop_code(&mut self) -> Option<State> {
        loop {
            match self.code.pc {
                0 => {
                    self.ja_play(anim::FLOP_DOWN, 1.0);
                    self.goto(1);
                }
                1 => {
                    if !self.ja_wait() {
                        return None;
                    }
                    self.control.gravity_length = STANDARD_GRAVITY;
                    self.danger_set(Some(Danger::FlopDown));
                    let c = &mut self.control;
                    c.align_xz_vel = with_vertical(c.align_xz_vel, c.gravity_normal, FLOP_DOWN);
                    c.transv = with_vertical(c.transv, c.gravity_normal, FLOP_DOWN);
                    self.code.pc = 2;
                    return None;
                }
                _ => {
                    self.foot.flop_frames += 1;
                    self.chan.eval_with(&self.anims, NumFunc::seek(1.0));
                    let c = &mut self.control;
                    let up = c.gravity_normal.dot(c.transv);
                    let mut flat = flatten(c.transv, c.gravity_normal);
                    if c.current.transv_max < flat.length() {
                        flat = normalize(flat, c.current.transv_max);
                    }
                    c.transv = flat + c.gravity_normal * up;
                    if self.time_elapsed(self.state_time, FALL_TIMEOUT) {
                        return Some(State::Falling { uppercut: false });
                    }
                    return None;
                }
            }
        }
    }

    /// The dive's landing; X held in its window bounces into the flop jump.
    fn flop_hit_ground_code(&mut self, stuck: bool) -> Option<State> {
        loop {
            match self.code.pc {
                0 => {
                    self.code.flag =
                        self.pad.stick0_speed == 0.0 || self.control.ctrl_xz_vel < 61440.0;
                    self.ja_play(anim::FLOP_DOWN_LAND, 1.0);
                    self.goto(1);
                }
                _ => {
                    if self.ja_wait() {
                        return Some(State::Falling { uppercut: false });
                    }
                    self.compute_delta_align();
                    let still = self.code.flag;
                    let flags = if still {
                        opts::Y_VEL
                    } else {
                        opts::Y_VEL | opts::XZ_VEL
                    };
                    self.align(flags, 1.0, 1.0, 1.0);
                    let aframe = self.aframe_num();
                    if (21.0..=25.0).contains(&aframe)
                        && !stuck
                        && self.pad.hold(button::X)
                        && self.can_jump(false)
                    {
                        let c = &mut self.control;
                        c.transv = flatten(c.transv, c.gravity_normal);
                        return Some(State::DuckHighJump {
                            min: FLOP_JUMP_HEIGHT_MIN,
                            max: FLOP_JUMP_HEIGHT_MAX,
                            kind: if still {
                                HighJump::Flop
                            } else {
                                HighJump::FlopForward
                            },
                        });
                    }
                    return None;
                }
            }
        }
    }

    /// The roll: carried by its animation; an X pressed during it, or just
    /// after, flips out of it.
    fn roll_code(&mut self) -> Option<State> {
        loop {
            match self.code.pc {
                0 => {
                    self.code.t = LONG_AGO;
                    self.code.t2 = 0;
                    self.code.x = 1.0;
                    self.ja_push(seconds(0.04));
                    self.ja_set(anim::DUCK_ROLL, NumFunc::Identity, 0.0);
                    self.goto(1);
                }
                1 => {
                    if !self.code.arrived {
                        self.chan.eval_with(&self.anims, NumFunc::seek(1.0));
                        self.code.arrived = true;
                        self.code.x *= self.control.zx_vel_frac.min(1.0);
                        if self.chan.done(&self.anims) {
                            self.goto(2);
                            continue;
                        }
                    }
                    if self.pad.pressed(button::X) {
                        self.code.t = self.time;
                    }
                    if (self.smack_surface_roll() || self.control.surface_slope_z >= 0.7)
                        && self.time_elapsed(self.state_time, 1)
                        && self.code.t2 == 0
                    {
                        self.code.t2 = self.time;
                    }
                    self.compute_delta_align();
                    if self.code.t2 == 0 {
                        self.align(opts::XZ_VEL, 1.0, 1.0, self.code.x);
                        self.keep_align_xz_vel();
                    }
                    return None;
                }
                _ => {
                    if (!self.time_elapsed(self.code.t, ROLL_JUMP_PRE_WINDOW)
                        || self.pad.pressed(button::X))
                        && self.can_jump_roll_flip()
                    {
                        return Some(State::RollFlip {
                            height: ROLL_FLIP_HEIGHT,
                            dist: ROLL_FLIP_DIST,
                        });
                    }
                    self.set_hook(StateHook::RollJump);
                    return Some(State::DuckStance { keep_time: true });
                }
            }
        }
    }

    fn smack_surface_roll(&self) -> bool {
        let c = &self.control;
        0.7 < c.touch_angle
            && c.surface_angle < 0.3
            && c.status & crate::control::status::TOUCH_WALL != 0
            && c.status & crate::control::status::TOUCH_ACTOR == 0
    }

    /// The flip out of a roll: the animation's arc scaled to the flip's
    /// height and length; a jump just after landing goes higher.
    fn roll_flip_code(
        &mut self,
        height: f32,
        dist: f32,
        world: &mut dyn CollideWorld,
    ) -> Option<State> {
        loop {
            match self.code.pc {
                0 => {
                    self.ja_push(seconds(0.04));
                    self.ja_set(anim::ROLL_FLIP, NumFunc::Identity, 0.0);
                    self.code.x = 1.0;
                    self.goto(1);
                }
                1 => {
                    self.compute_delta_align();
                    let along = self.code.x * (dist / ROLL_FLIP_ART_DIST);
                    if !self.at_max() {
                        self.align(
                            opts::Y_VEL | opts::XZ_VEL,
                            1.0,
                            height / ROLL_FLIP_ART_HEIGHT,
                            along,
                        );
                    } else {
                        self.align(opts::XZ_VEL, 1.0, 1.0, along);
                    }
                    self.keep_align_xz_vel();
                    self.code.pc = 2;
                    return None;
                }
                2 => {
                    self.chan.eval_with(&self.anims, NumFunc::seek(1.0));
                    self.code.x *= self.control.zx_vel_frac.min(1.0);
                    let done =
                        self.at_max() || (self.aframe_num() >= 4.0 && self.control.on_surface());
                    if done {
                        self.state_time = self.time;
                        self.goto(3);
                    } else {
                        self.goto(1);
                    }
                }
                3 => {
                    if self.control.on_surface() {
                        self.goto(5);
                        continue;
                    }
                    if self.time_elapsed(self.state_time, 3) && !self.chan.is(anim::JUMP_LOOP) {
                        self.ja_push(seconds(0.1));
                        self.ja_set(anim::JUMP_LOOP, NumFunc::Identity, 0.0);
                    }
                    let c = &mut self.control;
                    let up = c.gravity_normal.dot(c.transv);
                    c.transv = flatten(c.transv, c.gravity_normal) * 0.9 + c.gravity_normal * up;
                    self.code.pc = 4;
                    return None;
                }
                4 => {
                    if self.chan.is(anim::JUMP_LOOP) {
                        self.chan
                            .eval_with(&self.anims, NumFunc::Loop { rate: 1.0 });
                    } else {
                        self.chan.func = NumFunc::Identity;
                        self.chan.frame = self.anims.max(self.chan.anim);
                    }
                    self.goto(3);
                }
                _ => {
                    self.events.push(crate::Event::Land);
                    self.set_hook(StateHook::FlipJump);
                    if !self.can_exit_duck(world) {
                        return Some(State::DuckStance { keep_time: false });
                    }
                    return Some(if self.chan.is(anim::JUMP_LOOP) {
                        State::HitGround { stuck: false }
                    } else {
                        State::Stance
                    });
                }
            }
        }
    }
}
