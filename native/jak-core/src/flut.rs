//! Jak 1's Flut Flut, ridden (`levels/flut_common/target-flut.gc`, Jak 1):
//! its tuning (`*FLUT-bank*`, the `*flut-*-mods*` surfaces), and its states:
//! getting on, standing, walking (the walk with the run mixed in by speed),
//! the jump (5 to 7 m held) and its flap (1 to 2 m more), landing, falling,
//! the running attack (a lunge as far as its animation carries, stopped by a
//! wall), the air attack (a dive at 20 m/s that strikes the ground round it),
//! being hit, and getting off. Its animations are Jak 1's `eichar-flut-*`,
//! carried onto Jak 3's skeleton by the model loader as `jakb1-flut-*`; the
//! bird itself is the host's, drawn where Jak sits with the same animation.
//!
//! The surfaces keep Jak 1's speeds and turns; the fields Jak 1 had not (the
//! turn and tilt factors) are Jak 3's flut's, which this engine reads.
use glam::{Quat, Vec3};

use crate::anim::{self, NumFunc};
use crate::attack::Danger;
use crate::collide::CollideWorld;
use crate::control::{self, status};
use crate::math::*;
use crate::pad::button;
use crate::surface::{self, Mode, Name, Surface, flag};
use crate::target::{FALL_HEIGHT, STUCK_DISTANCE, STUCK_TIME, STUCK_TIMEOUT};
use crate::{Jak, State};

/// `*FLUT-bank*`.
pub const JUMP_HEIGHT_MIN: f32 = meters(5.0);
pub const JUMP_HEIGHT_MAX: f32 = meters(7.0);
pub const DOUBLE_JUMP_HEIGHT_MIN: f32 = meters(1.0);
pub const DOUBLE_JUMP_HEIGHT_MAX: f32 = meters(2.0);
pub const AIR_ATTACK_SPEED: f32 = meters(20.0);
pub const GROUND_TIMEOUT: i64 = seconds(0.5);

const BASE: Surface = Surface {
    name: Name::Run,
    turnv: 54613.332,
    turnvf: 30.0,
    turnvv: 524288.0,
    turnvvf: 30.0,
    tiltv: 65536.0,
    tiltvf: 150.0,
    tiltvv: 131072.0,
    tiltvvf: 15.0,
    vel_turn: 0.0,
    transv_max: 81920.0,
    target_speed: 81920.0,
    seek0: 1.5,
    seek90: 3.0,
    seek180: 2.0,
    fric: 1.0,
    nonlin_fric_dist: 0.1,
    slip_factor: 1.0,
    slide_factor: 1.0,
    slope_up_factor: 1.0,
    slope_down_factor: 1.0,
    slope_slip_angle: 1.0,
    impact_fric: 1.0,
    bend_factor: 1.0,
    bend_speed: 1.0,
    alignv: 1.0,
    slope_up_traction: 1.0,
    align_speed: 1.0,
    slope_change_preserve: 0.0,
    hook: surface::Hook::None,
    mode: Mode::Ground,
    flags: flag::NO_TURN_AROUND,
};

/// `*flut-walk-mods*`.
pub const WALK: Surface = BASE;

/// `*flut-jump-mods*`.
pub const JUMP: Surface = Surface {
    name: Name::Jump,
    turnv: 131072.0,
    turnvv: 54613.332,
    tiltv: 32768.0,
    tiltvv: 131072.0,
    seek0: 0.9,
    seek90: 1.5,
    seek180: 1.5,
    fric: 0.2,
    nonlin_fric_dist: 1.0,
    mode: Mode::Air,
    flags: flag::AIR,
    ..BASE
};

/// `*flut-double-jump-mods*`.
pub const DOUBLE_JUMP: Surface = Surface {
    name: Name::JumpDouble,
    fric: 0.1,
    nonlin_fric_dist: 10.0,
    ..JUMP
};

/// `*flut-run-attack-mods*` (its turns set as it begins).
pub const RUN_ATTACK: Surface = Surface {
    name: Name::Attack,
    turnv: 0.0,
    turnvv: 0.0,
    tiltv: 32768.0,
    tiltvv: 131072.0,
    transv_max: 122880.0,
    target_speed: 122880.0,
    seek0: 0.0,
    seek90: 0.5,
    seek180: 0.15,
    fric: 0.0,
    nonlin_fric_dist: 0.0,
    slope_up_factor: 0.25,
    mode: Mode::Attack,
    flags: flag::NO_TURN_AROUND | flag::ATTACK,
    ..BASE
};

/// `*flut-air-attack-mods*`.
pub const AIR_ATTACK: Surface = Surface {
    name: Name::Flop,
    turnv: 0.0,
    turnvv: 0.0,
    tiltv: 32768.0,
    tiltvv: 131072.0,
    transv_max: 122880.0,
    target_speed: 122880.0,
    seek0: 1.0,
    seek90: 0.3,
    seek180: 1.5,
    fric: 0.0,
    nonlin_fric_dist: 0.0,
    slope_up_factor: 0.25,
    mode: Mode::Attack,
    flags: flag::AIR | flag::ATTACK,
    ..BASE
};

/// The walk, jog and run cycles' ground per cycle (`:dist`): the walk's and
/// the run's (Jak 1 mixes the two).
const WALK_DIST: f32 = 49152.0;
const RUN_DIST: f32 = 40960.0;

/// The ride's own memory (`flut-info`, and what its states keep in the
/// control block's spare words).
#[derive(Clone, Debug, Default)]
pub struct FlutInfo {
    /// A Flut Flut to climb onto: where it stands and faces (Jak space).
    pub request: Option<(Vec3, Quat)>,
    /// Off it: where the bird is left standing and how it faces.
    pub dismounted: Option<(Vec3, Quat)>,
    pub get_off_requested: bool,
    /// Getting on or off: from Jak's place and turn to the bird's.
    from: Vec3,
    to: Vec3,
    quat_from: Quat,
    quat_to: Quat,
    /// The walk's own turn and speed, restored as it ends.
    walk_turnv: f32,
    walk_speed: f32,
    /// How hard the jump was pressed, and how far forward it leans (the
    /// jump-forward channel's mix).
    jump_press: f32,
    pub forward: f32,
    /// The running attack: when it last struck, when it was stopped, and
    /// whether what it struck was an enemy.
    struck_at: i64,
    stopped_at: i64,
    struck_enemy: bool,
    /// Its lunge's strength and speed.
    lunge: f32,
    lunge_speed: f32,
    /// The run mixed over the walk (0 walks, 1 runs).
    pub run_mix: f32,
    /// Swimming ([`crate::flut_swim`]).
    pub swimming: bool,
    /// Jumped out of the water: no flap on the way (both games').
    pub(crate) water_jump: bool,
    /// How long it has pushed against a bank while swimming, seconds.
    pub(crate) blocked: f32,
    /// The bird's animation follows Jak's from the moment he is in the
    /// saddle (`clone-anim`).
    pub clone: bool,
}

impl State {
    /// One of the Flut Flut's states.
    pub fn is_flut(&self) -> bool {
        matches!(
            self,
            State::FlutGetOn
                | State::FlutStance
                | State::FlutWalk
                | State::FlutJump { .. }
                | State::FlutDoubleJump { .. }
                | State::FlutHitGround
                | State::FlutFalling
                | State::FlutRunningAttack
                | State::FlutAirAttack
                | State::FlutAirAttackHitGround
                | State::FlutHit { .. }
                | State::FlutGetOff
                | State::FlutGetOffJump
        )
    }
}

impl Jak {
    /// Climbs onto the Flut Flut standing at `trans` facing `quat` (Jak
    /// space), when on his feet and not busy.
    pub fn request_flut(&mut self, trans: Vec3, quat: Quat) {
        self.flut.request = Some((trans, quat));
    }

    /// Off the Flut Flut (Triangle), once on the ground.
    pub fn request_flut_off(&mut self) {
        self.flut.get_off_requested = true;
    }

    /// Riding a Flut Flut.
    pub fn on_flut(&self) -> bool {
        self.state.is_flut() && !matches!(self.state, State::FlutGetOffJump)
    }

    /// Climbing on: where the bird stands and how it faces (Jak space).
    pub fn flut_mount_target(&self) -> Option<(Vec3, Quat)> {
        (self.state == State::FlutGetOn).then_some((self.flut.to, self.flut.quat_to))
    }

    /// Struck while riding, from `from` (Jak space): thrown back as Jak 1's
    /// flut is (`target-flut-hit`), its hit animation, then down.
    pub fn flut_hit(&mut self, from: Vec3) {
        if self.on_flut() && !matches!(self.state, State::FlutHit { .. } | State::FlutGetOn) {
            let away = (self.control.trans - from).with_y(0.0).normalize_or(-crate::math::z_axis(self.control.quat));
            self.pending = Some(State::FlutHit { dir: away });
        }
    }

    fn flut_falls(&self) -> bool {
        let c = &self.control;
        !c.on_surface()
            && self.time_elapsed(c.last_time_on_surface, GROUND_TIMEOUT)
            && c.gravity_normal.dot(c.transv) <= 0.0
            && c.height_above_ground() >= FALL_HEIGHT
    }

    fn flut_stuck(&self, after: i64) -> bool {
        self.control.move_dist(self.time, STUCK_TIME) < STUCK_DISTANCE && self.time_elapsed(self.state_time, after)
    }

    /// `target-flut-get-off?`: not in the air, a second after climbing on.
    fn flut_can_get_off(&self) -> bool {
        !(self.control.mod_surface.flags & flag::AIR != 0 && !self.control.on_surface())
            && self.time_elapsed(self.state_time, seconds(0.0))
    }

    fn flut_forward_target(&self) -> f32 {
        ((self.control.ctrl_xz_vel - 10240.0) * 0.000048828126).clamp(0.0, 1.0)
    }

    /// Starts the ride if one was asked for. On the ground or in the air,
    /// as Jak 1's `change-mode 'flut` goes to `target-flut-start` from any
    /// state (Adrii, 21/10: from a jump near it, as in the original).
    pub(crate) fn flut_start_check(&mut self) -> Option<State> {
        let (trans, quat) = self.flut.request.take()?;
        if self.state.is_board() || self.state.is_flut() {
            return None;
        }
        self.flut.from = self.control.trans;
        self.flut.to = trans;
        self.flut.quat_from = self.control.quat;
        self.flut.quat_to = quat;
        Some(State::FlutGetOn)
    }

    /// Where a ride starts from and goes to: from Jak as he is, onto the
    /// bird at `trans` facing `quat`.
    pub(crate) fn flut_set_mount(&mut self, trans: Vec3, quat: Quat) {
        self.flut.from = self.control.trans;
        self.flut.to = trans;
        self.flut.quat_from = self.control.quat;
        self.flut.quat_to = quat;
    }

    pub(crate) fn flut_displace_mount(&mut self, delta: Vec3) {
        if self.state == State::FlutGetOn {
            self.flut.from += delta;
            self.flut.to += delta;
        }
    }

    pub(crate) fn flut_enter(&mut self, state: &State) {
        let f = &mut self.flut;
        match *state {
            State::FlutGetOn => {
                self.control.mod_surface = WALK;
                self.control.transv = Vec3::ZERO;
                f.clone = false;
                self.events.push(sound("jak1:uppercut", 1.0, 0.0));
            }
            State::FlutStance | State::FlutHitGround => {
                self.control.mod_surface = WALK;
                if !self.flut.swimming {
                    self.flut.water_jump = false;
                }
                if *state == State::FlutHitGround {
                    self.events.push(crate::Event::Land);
                }
            }
            State::FlutWalk => {
                self.control.mod_surface = WALK;
                f.walk_turnv = self.control.mod_surface.turnv;
                f.walk_speed = self.control.mod_surface.target_speed;
            }
            State::FlutJump { min, max } => {
                self.init_var_jump(min, max, true, true, 0.0);
                self.control.status &= !(status::ON_SURFACE | status::ON_GROUND | status::TOUCH_SURFACE);
                self.control.mod_surface = JUMP;
                let v = self.control.ctrl_xz_vel;
                self.flut.jump_press = ((v - 11468.8) * 0.00004359654).clamp(0.0, 1.0);
                self.flut.forward = ((v - 10240.0) * 0.000048828126).clamp(0.0, 1.0);
                self.events.push(crate::Event::Jump);
            }
            State::FlutDoubleJump { min, max } => {
                self.init_var_jump(min, max, true, true, 0.0);
                self.control.gravity_max = 40960.0;
                self.control.gravity_length = 245760.0;
                self.control.status &= !(status::ON_SURFACE | status::ON_GROUND | status::TOUCH_SURFACE);
                self.control.mod_surface = DOUBLE_JUMP;
                // `(sound-play "flut-flap")` while it flaps.
                self.events.push(sound("jak1:flut-flap", 1.0, 0.0));
            }
            State::FlutFalling => self.control.mod_surface = JUMP,
            State::FlutRunningAttack => {
                f.struck_at = 0;
                f.stopped_at = 0;
                f.struck_enemy = false;
                self.control.mod_surface = Surface { turnv: 655360.0, turnvv: 655360.0, ..RUN_ATTACK };
                self.start_attack();
                self.danger_set(Some(Danger::Flut));
                self.events.push(sound("jak1:flut-hit", 1.0, 0.0));
            }
            State::FlutAirAttack => {
                self.set_forward_vel(AIR_ATTACK_SPEED);
                self.control.status &= !(status::ON_SURFACE | status::ON_GROUND | status::TOUCH_SURFACE);
                self.control.mod_surface = AIR_ATTACK;
                self.start_attack();
                self.danger_set(Some(Danger::Flut));
                self.events.push(sound("jak1:flut-hit", 1.0, -0.5));
                // No climb left: only the flat part of the motion.
                let c = &mut self.control;
                let up = c.gravity_normal.dot(c.transv);
                c.transv -= c.gravity_normal * up;
            }
            State::FlutAirAttackHitGround => {
                self.control.mod_surface = AIR_ATTACK;
                self.events.push(crate::Event::Land);
                // `flop-land` a little down, and the strike's dust and sparks.
                self.events.push(sound("jak1:flop-land", 1.0, -0.4));
                self.events.push(crate::Event::FlutSlam { pos: self.control.trans });
                // The touch-tracker it leaves: a blast round where it lands.
                self.start_attack();
                self.danger_set(Some(Danger::FlutGround));
            }
            State::FlutHit { dir } => {
                // `target-hit-move` with Jak 1's flut values: 2.5 m back,
                // 2.2 m up.
                self.control.mod_surface = surface::JUMP;
                let back = dir * 10240.0 * 2.0;
                self.control.transv = back + self.control.gravity_normal * (2.0 * self.control.gravity_length * 9011.2).sqrt();
                self.control.status &= !(status::ON_SURFACE | status::ON_GROUND | status::TOUCH_SURFACE);
                self.events.push(sound("jak1:flut-hit", 1.0, -1.0));
            }
            State::FlutGetOff => {
                self.set_forward_vel(0.0);
                self.events.push(sound("jak1:flut-coo", 0.9, -0.5));
            }
            State::FlutGetOffJump => {
                self.control.transv = Vec3::ZERO;
                self.flut.from = self.control.trans;
                self.flut.quat_from = self.control.quat;
            }
            _ => {}
        }
    }

    pub(crate) fn flut_exit(&mut self, state: &State, next: &State) {
        match *state {
            State::FlutWalk => {
                self.control.mod_surface.turnv = self.flut.walk_turnv;
                self.control.mod_surface.target_speed = self.flut.walk_speed;
            }
            State::FlutDoubleJump { .. } | State::FlutRunningAttack | State::FlutAirAttack => {
                self.control.gravity_max = control::STANDARD_GRAVITY_MAX;
                self.control.gravity_length = control::STANDARD_GRAVITY;
                if *state == State::FlutRunningAttack {
                    self.foot.last_running_attack_end_time = self.time;
                }
                if *state == State::FlutAirAttack && !matches!(next, State::FlutAirAttackHitGround) {
                    self.danger_set(None);
                }
            }
            State::FlutAirAttackHitGround => {
                self.danger_set(None);
                self.control.gravity_max = control::STANDARD_GRAVITY_MAX;
                self.control.gravity_length = control::STANDARD_GRAVITY;
            }
            _ => {}
        }
        if matches!(*state, State::FlutRunningAttack) {
            self.danger_set(None);
        }
        // Getting off cut short, still on it: it was not left anywhere.
        if *state == State::FlutGetOffJump && next.is_flut() {
            self.flut.dismounted = None;
        }
        // `target-flut-start`'s exit: off the ride altogether.
        if !next.is_flut() {
            self.control.mod_surface = surface::WALK;
            self.control.gravity_max = control::STANDARD_GRAVITY_MAX;
            self.control.gravity_length = control::STANDARD_GRAVITY;
            self.danger_set(None);
            self.flut.clone = false;
            self.target_exit();
        }
    }

    /// The states' checks (`:trans`).
    pub(crate) fn flut_trans(&mut self) -> Option<State> {
        let jump = State::FlutJump {
            min: JUMP_HEIGHT_MIN,
            max: JUMP_HEIGHT_MAX,
        };
        let get_off = std::mem::take(&mut self.flut.get_off_requested);
        // The mashup's (Adrii, 21/10): Triangle in the air lets go of the
        // bird; Jak falls on his own, and it, left where he let go, falls
        // on its own.
        if get_off
            && matches!(self.state, State::FlutJump { .. } | State::FlutDoubleJump { .. } | State::FlutFalling)
        {
            self.flut.dismounted = Some((self.control.trans, self.control.quat));
            return Some(State::Falling { uppercut: false });
        }
        match self.state {
            State::FlutStance | State::FlutWalk | State::FlutHitGround => {
                let walking = self.state == State::FlutWalk;
                if self.state == State::FlutStance && self.move_legs() {
                    return Some(State::FlutWalk);
                }
                if walking && !self.move_legs() {
                    return Some(State::FlutStance);
                }
                if let Some(out) = self.flut_swim_jump() {
                    return Some(out);
                }
                if self.pad.recently_pressed(button::X) && self.can_jump(false) {
                    return Some(jump);
                }
                if self.state == State::FlutHitGround && self.move_legs() {
                    return Some(State::FlutWalk);
                }
                if self.state != State::FlutHitGround
                    && !self.flut.swimming
                    && self.pad.pressed(button::SQUARE)
                    && self.can_hands(true)
                {
                    return Some(State::FlutRunningAttack);
                }
                if self.flut_falls() {
                    return Some(State::FlutFalling);
                }
                if walking {
                    // Turning hard it turns faster.
                    let off = deg_diff(y_angle_of(self.control.dir_targ), y_angle_of(self.control.quat)).abs();
                    self.control.mod_surface.turnv =
                        lerp_scale(self.flut.walk_turnv, 4.0 * self.flut.walk_turnv, off, 8192.0, 21845.334);
                    self.control.mod_surface.target_speed =
                        seek(self.control.mod_surface.target_speed, self.flut.walk_speed, 81920.0 * SECONDS_PER_FRAME);
                }
                if get_off && self.flut_can_get_off() {
                    return Some(State::FlutGetOff);
                }
                None
            }
            State::FlutJump { .. } => {
                let x = if self.pad.hold(button::X) { 1.0 } else { 0.0 };
                self.flut.jump_press = self.flut.jump_press.max(x);
                if self.control.on_surface() {
                    return Some(State::FlutHitGround);
                }
                let up = self.control.gravity_normal.dot(self.control.transv);
                if self.pad.pressed(button::X)
                    && up < 40960.0
                    && self.control.height_above_ground() > 4096.0
                    && !self.flut.water_jump
                {
                    return Some(State::FlutDoubleJump {
                        min: DOUBLE_JUMP_HEIGHT_MIN,
                        max: DOUBLE_JUMP_HEIGHT_MAX,
                    });
                }
                if self.pad.pressed(button::SQUARE)
                    && up < 61440.0
                    && -61440.0 < up
                    && self.time_elapsed(self.control.last_time_of_stuck, STUCK_TIMEOUT)
                {
                    return Some(State::FlutAirAttack);
                }
                if self.flut_stuck(seconds(0.1)) {
                    self.control.status |= status::ON_SURFACE;
                    return Some(State::FlutHitGround);
                }
                self.mod_var_jump(self.pad.hold(button::X));
                let target = self.flut_forward_target();
                self.flut.forward = seek(self.flut.forward, target, SECONDS_PER_FRAME);
                None
            }
            State::FlutDoubleJump { .. } => {
                if self.control.on_surface() {
                    return Some(State::FlutHitGround);
                }
                if self.pad.recently_pressed(button::SQUARE)
                    && self.time_elapsed(self.control.last_time_of_stuck, STUCK_TIMEOUT)
                    && self.control.height_above_ground() > 4096.0
                {
                    return Some(State::FlutAirAttack);
                }
                if self.state_time != self.time {
                    self.mod_var_jump(self.pad.hold(button::X));
                }
                let target = self.flut_forward_target();
                self.flut.forward = seek(self.flut.forward, target, SECONDS_PER_FRAME);
                None
            }
            State::FlutFalling => {
                if self.control.on_surface() || self.flut_stuck(STUCK_TIME / 2) {
                    self.control.status |= status::ON_SURFACE;
                    return Some(State::FlutHitGround);
                }
                let target = self.flut_forward_target();
                self.flut.forward = seek(self.flut.forward, target, SECONDS_PER_FRAME);
                None
            }
            State::FlutRunningAttack => {
                if (!self.attack.struck.is_empty() || self.attack.struck_world) && self.flut.struck_at == 0 {
                    self.flut.struck_at = self.time;
                    self.flut.struck_enemy = !self.attack.struck.is_empty();
                }
                if self.state_time != self.time
                    && (self.smack_surface(true) || (self.control.local_slope_z >= 0.7 && self.control.status & status::TOUCH_ACTOR == 0))
                {
                    self.flut.stopped_at = self.time;
                    self.control.bend_target = 0.0;
                    let c = &mut self.control;
                    let up = c.gravity_normal.dot(c.transv);
                    c.transv = c.transv - c.gravity_normal * up + c.gravity_normal * up.min(0.0);
                    if (self.flut.struck_at == 0 || self.time - self.flut.struck_at >= 12) && !self.flut.struck_enemy {
                        // Smacked into a wall: thrown back (`target-shoved`).
                        let back = -crate::math::z_axis(self.control.quat);
                        return Some(State::FlutHit { dir: back });
                    }
                }
                None
            }
            State::FlutAirAttack => {
                // Never backward along its heading.
                let fwd = crate::math::z_axis(self.control.quat);
                let c = &mut self.control;
                let along = fwd.dot(c.transv);
                if along < 0.0 {
                    c.transv -= fwd * along;
                }
                if self.control.on_surface() {
                    return Some(State::FlutAirAttackHitGround);
                }
                if self.control.move_dist(self.time, STUCK_TIME) < 4096.0 && self.time_elapsed(self.state_time, seconds(0.5)) {
                    self.control.status |= status::ON_SURFACE;
                    return Some(State::FlutHitGround);
                }
                None
            }
            State::FlutAirAttackHitGround => {
                if let Some(out) = self.flut_swim_jump() {
                    return Some(out);
                }
                if self.pad.recently_pressed(button::X) && self.can_jump(false) {
                    return Some(jump);
                }
                if self.move_legs() {
                    return Some(State::FlutWalk);
                }
                if self.flut_falls() {
                    return Some(State::FlutFalling);
                }
                None
            }
            State::FlutHit { .. } => None,
            _ => None,
        }
    }

    /// The states' own sequences (`:code`).
    /// A step ahead no higher than the host's (a block): a hop up onto it
    /// as the bird reaches it, peaking just over its top, as the board's
    /// smooth steps (17/10: ridden, it stood against one-block steps, its
    /// walk frozen at 0 m/s).
    pub(crate) fn flut_step_up(&mut self, world: &mut dyn CollideWorld) {
        self.control.draw_offset_y=seek(self.control.draw_offset_y,0.0,meters(5.5)*SECONDS_PER_FRAME);
        let Some(step) = self.flut_smooth_step else { return };
        if !self.control.on_surface() || self.pad.stick0_speed <= 0.0 {
            return;
        }
        self.terrain_step_up(world,step,false);
    }

    pub(crate) fn flut_code(&mut self, world: &mut dyn CollideWorld) -> Option<State> {
        match self.state {
            State::FlutGetOn => self.flut_get_on_code(),
            State::FlutStance => self.flut_stance_code(),
            State::FlutWalk => self.flut_walk_code(),
            State::FlutJump { .. } => self.flut_jump_code(),
            State::FlutDoubleJump { .. } => self.flut_double_jump_code(),
            State::FlutHitGround => self.flut_hit_ground_code(),
            State::FlutFalling => self.flut_falling_code(),
            State::FlutRunningAttack => self.flut_running_attack_code(),
            State::FlutAirAttack => self.flut_air_attack_code(),
            State::FlutAirAttackHitGround => self.flut_air_attack_land_code(),
            State::FlutHit { .. } => self.flut_hit_code(),
            State::FlutGetOff => self.flut_get_off_code(),
            State::FlutGetOffJump => self.flut_get_off_jump_code(world),
            _ => None,
        }
    }

    /// `ja :num! (seek! (ja-aframe n 0))` waited on.
    fn flut_seek_to(&mut self, artist: f32) -> bool {
        let to = self.aframe(self.chan.anim, artist);
        if self.code.arrived {
            return false;
        }
        self.chan.eval_with(&self.anims, NumFunc::seek_to(to, 1.0));
        self.code.arrived = true;
        self.chan.done(&self.anims)
    }

    fn flut_get_on_code(&mut self) -> Option<State> {
        // Carried from where he stood onto the saddle: across in 0.75 s, up
        // in 0.5 s, turned as the bird faces.
        let t = (self.time - self.state_time) as f32;
        let across = (t * 0.0044444446).min(1.0);
        let up = (t * 0.006666667).clamp(0.0, 1.0);
        let mut at = self.flut.from.lerp(self.flut.to, across);
        at.y = lerp(self.flut.from.y, self.flut.to.y, up);
        self.control.trans = at;
        self.control.quat = self.flut.quat_from.slerp(self.flut.quat_to, across);
        self.control.transv = Vec3::ZERO;
        loop {
            match self.code.pc {
                0 => {
                    self.ja_push(seconds(0.05));
                    let to = self.aframe(anim::FLUT1_GET_ON, 24.0);
                    self.ja_set(anim::FLUT1_GET_ON, NumFunc::seek_to(to, 1.0), 0.0);
                    self.flut.clone = true;
                    self.goto(1);
                }
                1 => {
                    if self.flut_seek_to(24.0) {
                        self.events.push(sound("jak1:flut-coo", 1.0, 0.0));
                        self.control.quat = self.flut.quat_to;
                        self.control.dir_targ = self.flut.quat_to;
                        return Some(State::FlutStance);
                    }
                    return None;
                }
                _ => return None,
            }
        }
    }

    fn flut_stance_code(&mut self) -> Option<State> {
        loop {
            match self.code.pc {
                0 => {
                    self.code.count = 22;
                    if self.chan.is_any(&[anim::FLUT1_WALK, anim::FLUT1_SQUASH_RUN]) {
                        self.code.count = 60;
                        self.goto(4);
                    } else if self.chan.is(anim::FLUT1_GET_ON) {
                        self.chan.func = NumFunc::seek(1.0);
                        self.goto(1);
                    } else if self.chan.is(anim::FLUT1_SMACK_SURFACE) {
                        self.ja_play(anim::FLUT1_SMACK_SURFACE_END, 1.0);
                        self.goto(2);
                    } else if self.chan.is(anim::FLUT1_RUNNING_ATTACK) {
                        self.ja_play(anim::FLUT1_RUNNING_ATTACK_END, 1.0);
                        self.goto(2);
                    } else {
                        self.goto(4);
                    }
                }
                1 => {
                    if self.ja_wait() {
                        self.goto(4);
                        continue;
                    }
                    return None;
                }
                2 => {
                    if self.ja_wait() {
                        self.goto(4);
                        continue;
                    }
                    return None;
                }
                4 => {
                    if !self.chan.is(anim::FLUT1_IDLE) {
                        self.ja_push(i64::from(self.code.count));
                    }
                    self.ja_set(anim::FLUT1_IDLE, NumFunc::Loop { rate: 1.0 }, 0.0);
                    self.flut.run_mix = 0.0;
                    self.code.pc = 5;
                    return None;
                }
                _ => {
                    self.chan.eval(&self.anims);
                    return None;
                }
            }
        }
    }

    fn flut_walk_code(&mut self) -> Option<State> {
        loop {
            match self.code.pc {
                0 => {
                    self.code.x = 0.0;
                    self.code.count = 22;
                    if self.chan.is(anim::FLUT1_IDLE) {
                        self.code.count = 60;
                    } else if self.chan.is_any(&[
                        anim::FLUT1_JUMP,
                        anim::FLUT1_JUMP_LOOP,
                        anim::FLUT1_AIR_ATTACK,
                        anim::FLUT1_AIR_ATTACK_LAND,
                    ]) {
                        // Landing into the walk: its squash first.
                        self.ja_push(seconds(0.08));
                        self.ja_play(anim::FLUT1_SQUASH_RUN, 1.0);
                        self.goto(1);
                        continue;
                    }
                    self.goto(2);
                }
                1 => {
                    if self.ja_wait() {
                        self.goto(2);
                        continue;
                    }
                    return None;
                }
                2 => {
                    let mut frame = 0.0;
                    if self.chan.is(anim::FLUT1_SQUASH_RUN) {
                        self.flut.run_mix = 1.0;
                    } else if self.chan.is(anim::FLUT1_WALK) {
                        frame = self.chan.frame;
                    } else {
                        self.ja_push(i64::from(self.code.count));
                    }
                    self.ja_set(anim::FLUT1_WALK, NumFunc::Loop { rate: 1.0 }, frame);
                    self.code.pc = 3;
                    return None;
                }
                _ => {
                    // The run mixed in from 12 to 19 m/s, at 4 a second.
                    let want = lerp_scale(0.0, 1.0, self.control.ctrl_xz_vel, 49152.0, 77824.0);
                    self.flut.run_mix = seek(self.flut.run_mix, want, 4.0 * SECONDS_PER_FRAME);
                    let dist = lerp(WALK_DIST, RUN_DIST, self.flut.run_mix);
                    let rate = 58.0 * self.control.ctrl_xz_vel / (60.0 * dist) / self.anims.info(anim::FLUT1_WALK).speed.max(1e-6)
                        * self.anims.max(anim::FLUT1_WALK).max(1.0)
                        / 60.0;
                    self.chan.eval_with(&self.anims, NumFunc::Loop { rate });
                    return None;
                }
            }
        }
    }

    /// The jump's animation runs faster the sooner its top: its own top
    /// (artist frame 10) meets the jump's.
    fn flut_rise_rate(&self) -> f32 {
        let up = self.control.gravity_normal.dot(self.control.transv);
        let info = self.anims.info(self.chan.anim);
        let left = (10.0 - self.chan.aframe_num(&self.anims)) / (info.artist_step * info.speed).max(1e-6);
        if 0.0 < up && 0.0 < left {
            let to_apex = (up / (245760.0 / 300.0)).trunc().max(1.0);
            3.0f32.min(left).min(5.0 * left / to_apex)
        } else {
            1.0
        }
    }

    fn flut_jump_code(&mut self) -> Option<State> {
        loop {
            match self.code.pc {
                0 => {
                    self.ja_push(seconds(0.12));
                    self.ja_set(anim::FLUT1_JUMP, NumFunc::Identity, 0.0);
                    self.code.pc = 1;
                    return None;
                }
                1 => {
                    let speed = self.anims.info(anim::FLUT1_JUMP).speed;
                    self.chan.frame += speed;
                    self.code.pc = 2;
                    return None;
                }
                2 => {
                    let rate = self.flut_rise_rate();
                    self.chan.eval_with(&self.anims, NumFunc::seek(rate));
                    if self.chan.done(&self.anims) {
                        self.goto(3);
                        continue;
                    }
                    return None;
                }
                3 => {
                    self.ja_set(anim::FLUT1_JUMP_LOOP, NumFunc::Loop { rate: 1.0 }, 0.0);
                    self.code.pc = 4;
                    return None;
                }
                _ => {
                    self.chan.eval(&self.anims);
                    return None;
                }
            }
        }
    }

    fn flut_double_jump_code(&mut self) -> Option<State> {
        loop {
            match self.code.pc {
                0 => {
                    self.ja_push(seconds(0.05));
                    self.ja_play(anim::FLUT1_DOUBLE_JUMP, 1.0);
                    self.events.push(crate::Event::Jump);
                    self.goto(1);
                }
                1 => {
                    if self.ja_wait() {
                        self.control.mod_surface = JUMP;
                        // Once more from its artist frame 14.
                        let from = self.aframe(anim::FLUT1_DOUBLE_JUMP, 14.0);
                        self.ja_set(anim::FLUT1_DOUBLE_JUMP, NumFunc::seek(1.0), from);
                        self.goto(2);
                        continue;
                    }
                    return None;
                }
                2 => {
                    if self.ja_wait() {
                        self.goto(3);
                        continue;
                    }
                    return None;
                }
                3 => {
                    // Flapping on, from frame 14, while high over the ground;
                    // the pull eased back meanwhile.
                    if self.control.height_above_ground() > 8192.0 {
                        if !self.code.arrived {
                            self.chan.eval_with(&self.anims, NumFunc::Loop { rate: 1.0 });
                            let from = self.aframe(anim::FLUT1_DOUBLE_JUMP, 14.0);
                            if self.chan.frame < from {
                                self.chan.frame = from;
                            }
                            self.flut_ease_gravity();
                            self.code.arrived = true;
                        }
                        return None;
                    }
                    self.ja_push(seconds(0.1));
                    self.ja_set(anim::FLUT1_JUMP_LOOP, NumFunc::Loop { rate: 1.0 }, 0.0);
                    self.code.pc = 4;
                    return None;
                }
                _ => {
                    self.flut_ease_gravity();
                    self.chan.eval(&self.anims);
                    return None;
                }
            }
        }
    }

    fn flut_ease_gravity(&mut self) {
        let c = &mut self.control;
        c.gravity_max = seek(c.gravity_max, control::STANDARD_GRAVITY_MAX, 163840.0 * SECONDS_PER_FRAME);
        c.gravity_length = seek(c.gravity_length, control::STANDARD_GRAVITY, 163840.0 * SECONDS_PER_FRAME);
    }

    fn flut_falling_code(&mut self) -> Option<State> {
        match self.code.pc {
            0 => {
                if !self.chan.is(anim::FLUT1_JUMP_LOOP) {
                    let push = if self.chan.is(anim::FLUT1_DOUBLE_JUMP) { seconds(0.2) } else { seconds(0.5) };
                    self.ja_push(push);
                }
                self.ja_set(anim::FLUT1_JUMP_LOOP, NumFunc::Loop { rate: 1.0 }, 0.0);
                self.code.pc = 1;
                None
            }
            _ => {
                self.chan.eval(&self.anims);
                None
            }
        }
    }

    /// `target-flut-hit-ground-anim`, then the stance.
    fn flut_hit_ground_code(&mut self) -> Option<State> {
        loop {
            match self.code.pc {
                0 => {
                    if self.chan.is(anim::FLUT1_JUMP_LOOP) {
                        self.ja_push(seconds(0.05));
                        self.ja_play(anim::FLUT1_JUMP_LAND, 1.0);
                        self.goto(1);
                    } else if self.chan.is_any(&[anim::FLUT1_JUMP, anim::FLUT1_JUMP_LAND]) {
                        return Some(State::FlutStance);
                    } else if self.chan.is(anim::FLUT1_DOUBLE_JUMP) {
                        self.ja_play(anim::FLUT1_JUMP_LAND, 1.0);
                        self.goto(1);
                    } else if self.chan.is_any(&[anim::FLUT1_AIR_ATTACK, anim::FLUT1_AIR_ATTACK_LOOP]) {
                        self.ja_play(anim::FLUT1_AIR_ATTACK_LAND, 1.0);
                        self.goto(2);
                    } else {
                        return Some(State::FlutStance);
                    }
                }
                1 => {
                    if self.ja_wait() {
                        return Some(State::FlutStance);
                    }
                    return None;
                }
                _ => {
                    self.compute_delta_align();
                    self.align(crate::align::opts::XZ_VEL, 1.0, 1.0, 1.0);
                    if self.ja_wait() {
                        return Some(State::FlutStance);
                    }
                    return None;
                }
            }
        }
    }

    fn flut_running_attack_code(&mut self) -> Option<State> {
        loop {
            match self.code.pc {
                0 => {
                    self.ja_push(seconds(0.02));
                    self.ja_set(anim::FLUT1_RUNNING_ATTACK, NumFunc::seek(1.0), 0.0);
                    self.control.gravity_max = 368640.0;
                    self.control.gravity_length = 368640.0;
                    self.flut.lunge_speed = 0.0;
                    self.flut.lunge = 0.8;
                    self.events.push(crate::Event::Jump);
                    self.goto(1);
                }
                1 => {
                    // `(until (ja-done? 0) <its frame> (suspend) (ja :num!
                    // (seek! max align-speed)) ...)`.
                    if !self.code.arrived {
                        let rate = self.control.current.align_speed;
                        self.chan.eval_with(&self.anims, NumFunc::seek(rate));
                        if self.time_elapsed(self.state_time, seconds(0.1)) {
                            self.control.mod_surface.turnvv = 0.0;
                            self.flut.lunge *= self.control.zx_vel_frac.min(1.0);
                        }
                        self.code.arrived = true;
                    }
                    if self.chan.done(&self.anims) || self.flut.lunge < 0.05 {
                        self.goto(2);
                        continue;
                    }
                    self.compute_delta_align();
                    if self.chan.frame != 0.0 {
                        if self.chan.aframe_num(&self.anims) >= 20.0
                            && self.flut_falls()
                            && self.time - self.flut.struck_at >= 12
                        {
                            return Some(State::FlutFalling);
                        } else if self.flut.stopped_at != 0 && self.time - self.flut.stopped_at >= 12 {
                            self.set_forward_vel(0.0);
                            self.flut.lunge = 0.0;
                        } else {
                            self.flut.lunge_speed = self.flut.lunge * self.align_vel_z() * 60.0;
                            self.set_forward_vel(self.flut.lunge_speed);
                        }
                    }
                    return None;
                }
                2 => {
                    if self.flut_falls() {
                        return Some(State::FlutFalling);
                    }
                    if self.flut.lunge != 0.0 {
                        if !self.chan.done(&self.anims) {
                            self.ja_push(seconds(0.05));
                        }
                        self.ja_play(anim::FLUT1_RUNNING_ATTACK_END, 1.0);
                        self.goto(3);
                        continue;
                    }
                    return Some(State::FlutStance);
                }
                _ => {
                    self.compute_delta_align();
                    let lunge = self.flut.lunge;
                    self.align(crate::align::opts::XZ_VEL, 1.0, 1.0, lunge);
                    if self.ja_wait() {
                        return Some(State::FlutStance);
                    }
                    return None;
                }
            }
        }
    }

    /// How far the align joint moved ahead this frame (units).
    fn align_vel_z(&self) -> f32 {
        self.align.delta_trans.z
    }

    fn flut_air_attack_code(&mut self) -> Option<State> {
        loop {
            match self.code.pc {
                0 => {
                    self.ja_push(seconds(0.05));
                    let to = self.aframe(anim::FLUT1_AIR_ATTACK, 8.0);
                    self.ja_set(anim::FLUT1_AIR_ATTACK, NumFunc::seek_to(to, 1.0), 0.0);
                    self.goto(1);
                }
                1 => {
                    self.compute_delta_align();
                    self.align(crate::align::opts::Y_VEL | crate::align::opts::XZ_VEL, 1.0, 1.0, 1.0);
                    if self.flut_seek_to(8.0) {
                        let from = self.aframe(anim::FLUT1_AIR_ATTACK, 8.0);
                        self.ja_set(anim::FLUT1_AIR_ATTACK, NumFunc::seek(1.0), from);
                        self.goto(2);
                        continue;
                    }
                    return None;
                }
                2 => {
                    self.compute_delta_align();
                    self.align(crate::align::opts::Y_VEL | crate::align::opts::XZ_VEL, 1.0, 1.0, 1.0);
                    if self.ja_wait() {
                        self.ja_set(anim::FLUT1_AIR_ATTACK_LOOP, NumFunc::Identity, 0.0);
                        self.code.pc = 3;
                    }
                    return None;
                }
                _ => return None,
            }
        }
    }

    fn flut_air_attack_land_code(&mut self) -> Option<State> {
        loop {
            match self.code.pc {
                0 => {
                    let to = self.aframe(anim::FLUT1_AIR_ATTACK_LAND, 22.0);
                    self.ja_set(anim::FLUT1_AIR_ATTACK_LAND, NumFunc::seek_to(to, 1.0), 0.0);
                    self.goto(1);
                }
                1 => {
                    self.compute_delta_align();
                    self.align(crate::align::opts::XZ_VEL, 1.0, 1.0, 1.0);
                    if self.flut_seek_to(22.0) {
                        self.danger_set(None);
                        let from = self.aframe(anim::FLUT1_AIR_ATTACK_LAND, 22.0);
                        self.ja_set(anim::FLUT1_AIR_ATTACK_LAND, NumFunc::seek(1.0), from);
                        self.goto(2);
                        continue;
                    }
                    return None;
                }
                _ => {
                    self.compute_delta_align();
                    self.align(crate::align::opts::XZ_VEL, 1.0, 1.0, 1.0);
                    if self.ja_wait() {
                        return Some(State::FlutStance);
                    }
                    return None;
                }
            }
        }
    }

    fn flut_hit_code(&mut self) -> Option<State> {
        match self.code.pc {
            0 => {
                if !self.chan.is(anim::FLUT1_HIT_BACK) {
                    self.ja_push(seconds(0.075));
                    self.ja_play(anim::FLUT1_HIT_BACK, 1.0);
                }
                self.code.pc = 1;
                None
            }
            _ => {
                self.chan.eval(&self.anims);
                if self.time_elapsed(self.state_time, seconds(0.1)) && self.control.on_surface() {
                    return Some(State::FlutHitGround);
                }
                None
            }
        }
    }

    fn flut_get_off_code(&mut self) -> Option<State> {
        // Down to the ground first, falling as he must.
        if self.control.on_surface() {
            return Some(State::FlutGetOffJump);
        }
        self.flut_falling_code()
    }

    fn flut_get_off_jump_code(&mut self, world: &mut dyn CollideWorld) -> Option<State> {
        loop {
            match self.code.pc {
                0 => {
                    self.ja_push(seconds(0.05));
                    self.ja_play(anim::FLUT1_GET_OFF, 1.0);
                    // The bird stays where it stands, playing its own
                    // `flut-get-off` with him as Jak 1's does (it kept
                    // cloning him: 19/10 it vanished here and turned up
                    // where he landed, inside him).
                    self.flut.dismounted = Some((self.control.trans, self.control.quat));
                    self.goto(1);
                }
                1 => {
                    self.control.transv = Vec3::ZERO;
                    if self.ja_wait() {
                        // Down off its back where his body ended up (across,
                        // his main joint's place; up, the ground the bird
                        // stands on, a little over it).
                        let main = self.joints.get(2).copied().unwrap_or(Vec3::ZERO);
                        let mut across = self.control.quat * main.with_y(0.0);
                        // Short of a wall that way (19/10: Jak 1's levels
                        // leave room there; set down inside a block's
                        // wall, he fell through the ground): his body's
                        // sphere a meter up, swept out from the bird.
                        const BODY: f32 = 0.6 * 4096.0;
                        let from = self.control.trans + Vec3::Y * 4096.0;
                        if across.length() > 1.0
                            && let Some(hit) =
                                crate::collide::fill_and_probe_line_sphere(world, &mut self.probe_cache, from, across, BODY)
                        {
                            across *= hit.u.clamp(0.0, 1.0);
                        }
                        let mut at = self.control.trans + across;
                        at.y = self.control.trans.y + 409.6;
                        self.relocate(at);
                        return Some(State::HitGround { stuck: false });
                    }
                    return None;
                }
                _ => return None,
            }
        }
    }
}

fn y_angle_of(q: Quat) -> f32 {
    let z = q * Vec3::Z;
    z.x.atan2(z.z) * (32768.0 / std::f32::consts::PI)
}

fn deg_diff(a: f32, b: f32) -> f32 {
    let d = (b - a).rem_euclid(65536.0);
    if d > 32768.0 { d - 65536.0 } else { d }
}

/// `sound-play` of one of Jak 1's sounds.
fn sound(name: &'static str, volume: f32, pitch: f32) -> crate::Event {
    crate::Event::Sound { name, volume, pitch }
}
