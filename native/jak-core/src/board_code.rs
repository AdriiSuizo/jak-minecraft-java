//! What each board state does over time: the animations it plays and the
//! decisions it makes as they finish. Each state's sequence resumes every
//! frame where it last waited, so a trick lasts as long as its animation
//! and only ends between animations.
use crate::anim::{self, Anim, NumFunc};
use crate::control::status;
use crate::math::*;
use crate::pad::button;
use crate::{Jak, State, Trick};

/// The stick's push forward (positive) or back as the tricks read it.
fn stick_z(jak: &Jak) -> f32 {
    let (_, z) = jak.pad.left_trick_axes();
    analog_input((128.0 * z) as i32, 0.0, 96.0, 110.0, 1.0)
}

fn stick_x(jak: &Jak) -> f32 {
    let (x, _) = jak.pad.left_trick_axes();
    analog_input((128.0 * x) as i32, 0.0, 96.0, 110.0, 1.0)
}

/// The held grabs: the stick picks one; each plays its start, then loops.
const HOLDS: [(Anim, Anim, Anim, Trick); 4] = [
    (
        anim::BOARD_METHOD_CROSS,
        anim::BOARD_METHOD_CROSS_LOOP,
        anim::BOARD_METHOD_CROSS_END,
        Trick::MethodCross,
    ),
    (
        anim::BOARD_BACKGRAB,
        anim::BOARD_BACKGRAB_LOOP,
        anim::BOARD_BACKGRAB_END,
        Trick::Backgrab,
    ),
    (
        anim::BOARD_AIRWALK,
        anim::BOARD_AIRWALK_LOOP,
        anim::BOARD_AIRWALK_END,
        Trick::Airwalk,
    ),
    (
        anim::BOARD_METHOD,
        anim::BOARD_METHOD_LOOP,
        anim::BOARD_METHOD_END,
        Trick::Method,
    ),
];

impl Jak {
    pub(crate) fn ja_push(&mut self, ticks: i64) {
        self.chan.mix = None;
        self.push = crate::Push {
            tick: self.time,
            ticks,
        };
    }

    /// Plays `anim` from `frame` toward `func`'s target, without advancing.
    pub(crate) fn ja_set(&mut self, anim: Anim, func: NumFunc, frame: f32) {
        self.chan.set(anim, func, frame);
    }

    pub(crate) fn ja_play(&mut self, anim: Anim, rate: f32) {
        self.ja_set(anim, NumFunc::seek(rate), 0.0);
    }

    pub(crate) fn aframe(&self, anim: Anim, artist: f32) -> f32 {
        self.anims.aframe(anim, artist)
    }

    pub(crate) fn goto(&mut self, pc: u16) {
        self.code.pc = pc;
        self.code.arrived = true;
    }

    /// Waiting on an animation the way the game's do-while loops do: never
    /// done on arrival; after each suspend it advances a frame and is done
    /// once it has played out.
    pub(crate) fn ja_wait(&mut self) -> bool {
        if self.code.arrived {
            return false;
        }
        self.chan.eval(&self.anims);
        self.code.arrived = true;
        self.chan.done(&self.anims)
    }

    /// Waiting that checks first: done on arrival if it already played out.
    pub(crate) fn ja_while(&mut self) -> bool {
        if !self.code.arrived {
            self.chan.eval(&self.anims);
            self.code.arrived = true;
        }
        self.chan.done(&self.anims)
    }

    /// A loop that runs until the state changes: one step a frame.
    pub(crate) fn ja_loop(&mut self) -> Option<State> {
        if !self.code.arrived {
            self.chan
                .eval_with(&self.anims, NumFunc::Loop { rate: 1.0 });
            self.code.arrived = true;
        }
        None
    }

    /// Velocity upward cut to nothing, the way a trick that plays out ends.
    fn end_rise(&mut self) {
        let c = &mut self.control;
        let g = c.gravity_normal;
        let up = g.dot(c.transv).min(0.0);
        c.transv = with_vertical(c.transv, g, up);
    }

    /// The board state's sequence up to its next wait. Returns the state to
    /// go to when it decides one.
    pub(crate) fn board_code(&mut self) -> Option<State> {
        match self.state {
            State::BoardGetOn => self.get_on_code(),
            State::BoardStance | State::BoardDuckStance | State::BoardTurnTo { .. } => {
                self.stance_code()
            }
            State::BoardJump { .. } => self.jump_code(),
            State::BoardFalling => self.falling_code(),
            State::BoardHitGround => self.hit_ground_code(),
            State::BoardFlip => self.flip_code(),
            State::BoardTricky => self.tricky_code(),
            State::BoardTrickx => self.trickx_code(),
            State::BoardHold => self.hold_code(),
            State::BoardJumpKick => self.jump_kick_code(),
            State::BoardWallKick { dir, speed } => self.wall_kick_code(dir, speed),
            State::BoardGetOff => self.get_off_code(),
            _ => None,
        }
    }

    fn get_on_code(&mut self) -> Option<State> {
        loop {
            match self.code.pc {
                0 => {
                    let short = self.code.flag;
                    self.ja_push(if short { 9 } else { 60 });
                    self.code.rate = if short {
                        1.25
                    } else {
                        (249.99 / self.time_to_ground() as f32).clamp(1.0, 2.0)
                    };
                    self.ja_play(anim::BOARD_GET_ON, self.code.rate);
                    self.goto(1);
                }
                1 => {
                    if self.ja_wait() {
                        self.board.anim.duck_vel = 15.0;
                        self.goto(2);
                    } else if self.control.on_surface() {
                        self.board.anim.duck_vel = 0.0;
                        self.goto(3);
                    } else {
                        return None;
                    }
                }
                2 => {
                    if self.hit_ground_or_stuck() {
                        self.goto(3);
                    } else {
                        return None;
                    }
                }
                _ => {
                    self.control.status |= status::ON_SURFACE;
                    return Some(State::BoardHitGround);
                }
            }
        }
    }

    /// Stance, duck and turn: after the landing from getting on, the turn
    /// animation the board drives, every frame.
    fn stance_code(&mut self) -> Option<State> {
        loop {
            match self.code.pc {
                0 => {
                    if self.chan.is(anim::BOARD_TURN) {
                        self.goto(4);
                    } else if self.chan.is(anim::BOARD_GET_ON) {
                        self.ja_play(anim::BOARD_GET_ON_LAND, 1.8);
                        self.goto(1);
                    } else {
                        self.goto(3);
                    }
                }
                1 => {
                    if !self.ja_wait() {
                        return None;
                    }
                    self.code.pc = 2;
                    return None;
                }
                2 => {
                    self.board.anim.duck = 1.0;
                    self.board.anim.duck_vel = 15.0;
                    self.goto(3);
                }
                3 => {
                    self.ja_push(30);
                    self.ja_set(anim::BOARD_TURN, NumFunc::Identity, 0.0);
                    self.goto(4);
                }
                _ => {
                    self.board_turn_anim();
                    return None;
                }
            }
        }
    }

    fn jump_code(&mut self) -> Option<State> {
        match self.code.pc {
            0 => {
                self.code.flag = self.control.jump_height_min >= 20480.0;
                self.ja_push(15);
                let first = if self.code.flag {
                    anim::BOARD_JUMP_HIGH
                } else {
                    anim::BOARD_JUMP
                };
                self.ja_set(first, NumFunc::Identity, 0.0);
                self.code.pc = 1;
                None
            }
            1 => {
                // The jump plays on to its top at the rate the rise
                // leaves, slower once falling.
                let up = self.control.gravity_normal.dot(self.control.transv);
                let (apex, rise, fall) = if self.code.flag {
                    (25.0, 1.5, 0.8)
                } else {
                    (10.0, 0.5, 0.25)
                };
                let left = apex - self.chan.aframe_num(&self.anims);
                let rate = if 0.0 < up && 0.0 < left {
                    let to_apex = (up / (245760.0 / 300.0)).trunc();
                    1.5f32.min(left).min(5.0 * left / to_apex)
                } else if seconds(0.165) < self.time_to_ground() {
                    rise
                } else {
                    fall
                };
                self.chan.eval_with(&self.anims, NumFunc::seek(rate));
                self.code.pc = 2;
                None
            }
            2 => {
                if !self.chan.done(&self.anims) {
                    self.code.pc = 1;
                    return self.jump_code();
                }
                self.ja_push(15);
                self.chan.anim = anim::BOARD_JUMP_LOOP;
                self.goto(3);
                None
            }
            _ => self.ja_loop(),
        }
    }

    fn falling_code(&mut self) -> Option<State> {
        // Knocked about on it (`target-board-hit`): its hit plays out
        // first, then the fall.
        if self.code.pc == 0 && self.chan.is_any(&[anim::BOARD_HIT, anim::BOARD_HIT_ELEC]) {
            self.chan.func = anim::NumFunc::seek(1.0);
            self.goto(2);
        }
        if self.code.pc == 2 {
            if !self.ja_while() {
                return None;
            }
            self.ja_push(seconds(0.2));
            self.chan.anim = anim::BOARD_JUMP_LOOP;
            self.goto(1);
            return None;
        }
        if self.code.pc == 0 {
            let push = if self.chan.is(anim::BOARD_JUMP_LOOP) {
                None
            } else if self.chan.is(anim::BOARD_NOSEFLIP) {
                Some(seconds(0.5))
            } else if self.chan.is_any(&[
                anim::BOARD_FLIP_BACKWARD,
                anim::BOARD_FLIP_BACKWARD_LOOP,
                anim::BOARD_FLIP_FORWARD,
                anim::BOARD_FLIP_FORWARD_LOOP,
            ]) {
                Some(seconds(0.2))
            } else if self.chan.is(anim::BOARD_SPIN) {
                Some(seconds(0.1))
            } else {
                Some(seconds(0.5))
            };
            if let Some(ticks) = push {
                self.ja_push(ticks);
            }
            self.chan.anim = anim::BOARD_JUMP_LOOP;
            self.goto(1);
            return None;
        }
        self.ja_loop()
    }

    /// Landing: a held grab finishes its end before the stance.
    fn hit_ground_code(&mut self) -> Option<State> {
        loop {
            match self.code.pc {
                0 => {
                    self.control.mod_surface = crate::surface::board::WALK;
                    if self
                        .chan
                        .is_any(&[anim::BOARD_METHOD_END, anim::BOARD_NOSEGRAB_END])
                    {
                        self.chan.func = NumFunc::seek(1.5);
                        self.goto(2);
                    } else if self.chan.is(anim::BOARD_METHOD_LOOP) {
                        self.ja_push(seconds(0.08));
                        self.ja_play(anim::BOARD_METHOD_END, 1.5);
                        self.goto(1);
                    } else if self.chan.is(anim::BOARD_NOSEGRAB_LOOP) {
                        self.ja_push(seconds(0.08));
                        self.ja_play(anim::BOARD_NOSEGRAB_END, 1.5);
                        self.goto(1);
                    } else {
                        return Some(State::BoardStance);
                    }
                }
                1 => {
                    if !self.ja_wait() {
                        return None;
                    }
                    return Some(State::BoardStance);
                }
                _ => {
                    if !self.ja_while() {
                        return None;
                    }
                    return Some(State::BoardStance);
                }
            }
        }
    }

    /// R1 and the stick forward or back: flips, one after another while R1
    /// stays held and the ground is far enough off.
    fn flip_code(&mut self) -> Option<State> {
        let forward = |jak: &Jak| jak.code.flag;
        loop {
            match self.code.pc {
                0 => {
                    self.code.flag = self.board.flip_control >= 0.0;
                    self.code.z = stick_z(self);
                    self.code.count = 0;
                    self.goto(1);
                }
                1 => {
                    let stop = !self.pad.hold(button::R1)
                        || self.code.z == 0.0
                        || self.hit_ground_or_stuck()
                        || self.time_to_ground() < seconds(0.5);
                    if stop && self.code.count != 0 {
                        self.goto(10);
                        continue;
                    }
                    let first = self.code.count == 0;
                    if first {
                        self.ja_push(seconds(0.1));
                    }
                    let (anim, func) = match (forward(self), first) {
                        (true, true) => (
                            anim::BOARD_FLIP_FORWARD,
                            NumFunc::seek_to(self.aframe(anim::BOARD_FLIP_FORWARD, 15.0), 1.0),
                        ),
                        (true, false) => (anim::BOARD_FLIP_FORWARD_LOOP, NumFunc::seek(1.0)),
                        (false, true) => (
                            anim::BOARD_FLIP_BACKWARD,
                            NumFunc::seek_to(self.aframe(anim::BOARD_FLIP_BACKWARD, 10.0), 1.0),
                        ),
                        (false, false) => (anim::BOARD_FLIP_BACKWARD_LOOP, NumFunc::seek(1.0)),
                    };
                    self.ja_set(anim, func, 0.0);
                    self.goto(2);
                }
                2 => {
                    if !self.ja_wait() {
                        return None;
                    }
                    self.board.flip_count += 1;
                    self.code.count += 1;
                    self.goto(1);
                }
                10 => {
                    let (anim, from) = if forward(self) {
                        (anim::BOARD_FLIP_FORWARD, 15.0)
                    } else {
                        (anim::BOARD_FLIP_BACKWARD, 10.0)
                    };
                    let frame = self.aframe(anim, from);
                    self.ja_set(anim, NumFunc::seek(1.0), frame);
                    self.goto(11);
                }
                _ => {
                    if self.ja_wait() {
                        return Some(State::BoardFalling);
                    }
                    if self.hit_ground_or_stuck() {
                        return Some(State::BoardHitGround);
                    }
                    return None;
                }
            }
        }
    }

    /// L1 and the stick forward or back: a kick spin back, a nose grab held
    /// high up, a nose flip lower down. Repeats while L1 stays held.
    fn tricky_code(&mut self) -> Option<State> {
        loop {
            match self.code.pc {
                0 => {
                    self.code.z = stick_z(self);
                    self.goto(1);
                }
                1 => {
                    if !self.pad.hold(button::L1)
                        || self.hit_ground_or_stuck()
                        || self.time_to_ground() < seconds(0.3)
                    {
                        self.goto(20);
                    } else if self.code.z < 0.0 {
                        self.add_trick(Trick::Kickspin, 500.0);
                        self.ja_push(seconds(0.08));
                        self.ja_play(anim::BOARD_KICKSPIN_A, 1.0);
                        self.goto(2);
                    } else if self.chan.is(anim::BOARD_NOSEGRAB_LOOP) {
                        self.chan
                            .eval_with(&self.anims, NumFunc::Loop { rate: 1.0 });
                        self.code.pc = 9;
                        return None;
                    } else if 40960.0 < self.control.height_above_ground() {
                        self.add_trick(Trick::Nosegrab, 500.0);
                        self.ja_push(seconds(0.08));
                        self.ja_play(anim::BOARD_NOSEGRAB, 1.0);
                        self.goto(5);
                    } else {
                        self.add_trick(Trick::Noseflip, 500.0);
                        self.ja_push(seconds(0.08));
                        let to = self.aframe(anim::BOARD_NOSEFLIP, 20.0);
                        self.ja_set(anim::BOARD_NOSEFLIP, NumFunc::seek_to(to, 1.0), 0.0);
                        self.goto(7);
                    }
                }
                2 => {
                    if !self.ja_wait() {
                        return None;
                    }
                    self.ja_play(anim::BOARD_KICKSPIN_B, 1.05);
                    self.goto(3);
                }
                3 => {
                    if !self.ja_wait() {
                        return None;
                    }
                    self.ja_play(anim::BOARD_KICKSPIN_C, 1.0);
                    self.goto(4);
                }
                4 => {
                    if !self.ja_wait() {
                        return None;
                    }
                    self.code.pc = 9;
                    return None;
                }
                5 => {
                    if !self.ja_wait() {
                        return None;
                    }
                    self.ja_set(anim::BOARD_NOSEGRAB_LOOP, NumFunc::Identity, 0.0);
                    self.code.pc = 9;
                    return None;
                }
                7 => {
                    if !self.ja_wait() {
                        return None;
                    }
                    self.goto(20);
                }
                9 => self.goto(1),
                20 => {
                    if self.chan.is(anim::BOARD_NOSEGRAB_LOOP) {
                        self.ja_push(seconds(0.08));
                        self.ja_play(anim::BOARD_NOSEGRAB_END, 1.5);
                        self.goto(21);
                    } else {
                        self.goto(22);
                    }
                }
                21 => {
                    if !self.ja_wait() {
                        return None;
                    }
                    self.goto(22);
                }
                _ => {
                    self.end_rise();
                    return Some(State::BoardFalling);
                }
            }
        }
    }

    /// L1 and the stick across: a kick flip to one side, a board spin to
    /// the other; again while R1 is held with the stick pushed.
    fn trickx_code(&mut self) -> Option<State> {
        loop {
            match self.code.pc {
                0 => {
                    self.code.flag = self.board.trick_x >= 0.0;
                    self.code.z = stick_z(self);
                    self.code.count = 0;
                    self.goto(1);
                }
                1 => {
                    let stop = !self.pad.hold(button::R1)
                        || self.code.z == 0.0
                        || self.hit_ground_or_stuck()
                        || self.time_to_ground() < seconds(0.5);
                    if stop && self.code.count != 0 {
                        return Some(State::BoardFalling);
                    }
                    if self.code.flag {
                        self.add_trick(Trick::Kickflip, 500.0);
                        self.ja_push(seconds(0.07));
                        self.ja_play(anim::BOARD_KICKFLIP_A, 1.0);
                        self.goto(2);
                    } else {
                        self.add_trick(Trick::BoardSpin, 500.0);
                        self.ja_push(seconds(0.05));
                        self.ja_play(anim::BOARD_SPIN, 0.95);
                        self.goto(5);
                    }
                }
                2 => {
                    if !self.ja_wait() {
                        return None;
                    }
                    self.ja_play(anim::BOARD_KICKFLIP_B, 1.05);
                    self.goto(3);
                }
                3 => {
                    if !self.ja_wait() {
                        return None;
                    }
                    self.ja_play(anim::BOARD_KICKFLIP_C, 1.0);
                    self.goto(5);
                }
                _ => {
                    if !self.ja_wait() {
                        return None;
                    }
                    self.board.trickx_count += 1;
                    self.code.count += 1;
                    self.goto(1);
                }
            }
        }
    }

    /// L2: a grab the stick picks, held while L2 is.
    fn hold_code(&mut self) -> Option<State> {
        loop {
            match self.code.pc {
                0 => {
                    self.code.x = stick_x(self);
                    self.code.z = stick_z(self);
                    self.goto(1);
                }
                1 => {
                    if !self.pad.hold(button::L2)
                        || self.hit_ground_or_stuck()
                        || self.time_to_ground() < seconds(0.3)
                    {
                        self.goto(10);
                        continue;
                    }
                    let (x, z) = (self.code.x, self.code.z);
                    let pick = if x < 0.0 && z.abs() < x.abs() {
                        0
                    } else if z < 0.0 && x.abs() < z.abs() {
                        1
                    } else if 0.0 < z && x.abs() < z.abs() {
                        2
                    } else {
                        3
                    };
                    let (start, held, _, trick) = HOLDS[pick];
                    if self.chan.is(held) {
                        self.chan
                            .eval_with(&self.anims, NumFunc::Loop { rate: 1.0 });
                        self.code.pc = 3;
                        return None;
                    }
                    self.add_trick(trick, 500.0);
                    self.ja_push(seconds(0.08));
                    self.ja_play(start, 1.0);
                    self.code.count = pick as i32;
                    self.goto(2);
                }
                2 => {
                    if !self.ja_wait() {
                        return None;
                    }
                    let held = HOLDS[self.code.count as usize].1;
                    self.ja_set(held, NumFunc::Identity, 0.0);
                    self.code.pc = 3;
                    return None;
                }
                3 => self.goto(1),
                10 => match HOLDS.iter().find(|h| self.chan.is(h.1)) {
                    Some(&(_, _, end, _)) => {
                        self.ja_push(seconds(0.08));
                        self.ja_play(end, 1.5);
                        self.goto(11);
                    }
                    None => self.goto(12),
                },
                11 => {
                    if !self.ja_wait() {
                        return None;
                    }
                    self.goto(12);
                }
                _ => {
                    self.end_rise();
                    return Some(State::BoardFalling);
                }
            }
        }
    }

    fn jump_kick_code(&mut self) -> Option<State> {
        loop {
            match self.code.pc {
                0 => {
                    self.ja_push(seconds(0.1));
                    self.ja_play(anim::BOARD_JUMP_KICK, 1.0);
                    self.goto(1);
                }
                1 => {
                    if !self.ja_wait() {
                        return None;
                    }
                    self.ja_push(seconds(2.0));
                    self.chan.anim = anim::BOARD_JUMP_LOOP;
                    self.goto(2);
                    return None;
                }
                _ => return self.ja_loop(),
            }
        }
    }

    /// Off a wall: the kick-off plays out while the board keeps the speed
    /// away from the wall it was given.
    fn wall_kick_code(&mut self, dir: glam::Vec3, speed: f32) -> Option<State> {
        if self.code.pc == 0 {
            self.ja_play(anim::BOARD_JUMP_KICKOFF, 1.0);
            self.goto(1);
        }
        if self.ja_wait() {
            return Some(State::BoardFalling);
        }
        if self.hit_ground_or_stuck() && self.chan.aframe_num(&self.anims) > 31.0 {
            return Some(State::BoardHitGround);
        }
        let c = &mut self.control;
        c.transv.x = dir.x;
        c.transv.z = dir.z;
        c.transv = xz_normalize(c.transv, 81920.0f32.max(0.8 * speed));
        None
    }

    fn get_off_code(&mut self) -> Option<State> {
        loop {
            match self.code.pc {
                0 => {
                    self.code.rate = 1.5151515;
                    self.ja_push(seconds(0.1));
                    if self.code.flag {
                        self.ja_play(anim::BOARD_GET_OFF_PRE, self.code.rate);
                        self.goto(1);
                    } else {
                        self.code.rate = (150.0 / self.time_to_ground() as f32).clamp(0.5, 2.0);
                        let to = self.aframe(anim::BOARD_GET_OFF, 24.0);
                        let from = self.aframe(anim::BOARD_GET_OFF, 11.0);
                        self.ja_set(
                            anim::BOARD_GET_OFF,
                            NumFunc::seek_to(to, self.code.rate),
                            from,
                        );
                        self.goto(2);
                    }
                }
                1 => {
                    if !self.ja_wait() {
                        return None;
                    }
                    let to = self.aframe(anim::BOARD_GET_OFF, 24.0);
                    self.ja_set(
                        anim::BOARD_GET_OFF,
                        NumFunc::seek_to(to, self.code.rate),
                        0.0,
                    );
                    self.goto(2);
                }
                2 => {
                    if self.ja_wait() {
                        self.chan.func = NumFunc::seek(self.code.rate);
                        self.goto(3);
                    } else if self.hit_ground_or_stuck()
                        && self.chan.aframe_num(&self.anims) >= 14.0
                    {
                        return Some(State::Falling { uppercut: false });
                    } else {
                        return None;
                    }
                }
                3 => {
                    if self.ja_while() {
                        self.goto(4);
                    } else if self.hit_ground_or_stuck() {
                        return Some(State::Falling { uppercut: false });
                    } else {
                        return None;
                    }
                }
                _ => {
                    self.board.anim.duck_vel = 15.0;
                    return Some(if self.control.on_surface() {
                        State::HitGround { stuck: false }
                    } else {
                        State::Falling { uppercut: false }
                    });
                }
            }
        }
    }
}
