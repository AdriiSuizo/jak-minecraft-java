//! Jak on foot: his moves (stance, walking, ducking and crawling, jumps,
//! the spin, the punch, the uppercut, the dive, the roll and its flip),
//! each with the checks it makes every frame for what may cut in, the frame
//! loop that moves him, and the call for the board.
use glam::{Quat, Vec3};

use crate::anim;
use crate::attack::Danger;
use crate::collide::CollideWorld;
use crate::control::{self, GROUND_TIMEOUT, status};
use crate::math::*;
use crate::pad::button;
use crate::surface::{self, Surface, flag};
use crate::{AirFrom, Event, HighJump, Jak, State};

pub const JUMP_HEIGHT_MIN: f32 = meters(1.01);
pub const JUMP_HEIGHT_MAX: f32 = meters(3.5);
pub const DOUBLE_JUMP_HEIGHT_MIN: f32 = meters(1.0);
pub const DOUBLE_JUMP_HEIGHT_MAX: f32 = meters(2.5);
pub const FLIP_JUMP_HEIGHT_MIN: f32 = meters(5.0);
pub const FLIP_JUMP_HEIGHT_MAX: f32 = meters(7.0);
pub const DUCK_JUMP_HEIGHT_MIN: f32 = meters(7.0);
pub const DUCK_JUMP_HEIGHT_MAX: f32 = meters(7.0);
pub const FLOP_JUMP_HEIGHT_MIN: f32 = meters(5.0);
pub const FLOP_JUMP_HEIGHT_MAX: f32 = meters(7.0);
pub const ATTACK_JUMP_HEIGHT_MIN: f32 = meters(5.0);
pub const ATTACK_JUMP_HEIGHT_MAX: f32 = meters(6.5);
pub const ROLL_JUMP_PRE_WINDOW: i64 = seconds(1.0);
pub const ROLL_JUMP_POST_WINDOW: i64 = seconds(0.1);
pub const ROLL_SPEED_MIN: f32 = meters(11.5);
pub const ROLL_SPEED_INC: f32 = meters(1.5);
pub const ROLL_FLIP_HEIGHT: f32 = meters(3.52);
pub const ROLL_FLIP_DIST: f32 = meters(17.3);
pub const ROLL_FLIP_ART_HEIGHT: f32 = meters(3.2969);
pub const ROLL_FLIP_ART_DIST: f32 = meters(12.5);
pub const ATTACK_TIMEOUT: i64 = seconds(0.3);
pub const FALL_HEIGHT: f32 = meters(1.0);
pub const FALL_TIMEOUT: i64 = seconds(1.0);
pub const FALL_STUMBLE_THRESHOLD: f32 = meters(39.9);
pub const STUCK_TIME: i64 = seconds(0.3);
pub const STUCK_TIMEOUT: i64 = seconds(2.0);
pub const STUCK_DISTANCE: f32 = meters(0.05);
pub const DUCK_WALK_CYCLE_DIST: f32 = meters(3.25);
pub const RUN_CYCLE_LENGTH: f32 = 60.0;
pub const FALL_FAR: f32 = meters(30.0);
pub const FALL_FAR_INC: f32 = meters(20.0);

/// How far one cycle of each walk and run animation carries Jak.
pub mod cycle {
    use crate::math::meters;

    pub const WALK: f32 = meters(2.8);
    pub const WALK_UP: f32 = meters(2.8);
    pub const WALK_DOWN: f32 = meters(2.8);
    pub const WALK_SIDE: f32 = meters(2.8);
    pub const RUN: f32 = meters(6.25);
    pub const RUN_UP: f32 = meters(5.0);
    pub const RUN_DOWN: f32 = meters(5.0);
    pub const RUN_SIDE: f32 = meters(6.25);
}
/// The dive's own jump: what it leaves the ground at, and how fast it
/// drives down.
const FLOP_UP: f32 = 29491.2;
pub const FLOP_DOWN: f32 = -368640.0;

/// A window a finished move leaves open in the states after it.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum StateHook {
    #[default]
    None,
    /// Just out of a roll: X still turns it into the roll's flip.
    RollJump,
    /// Just landed from the flip: X jumps higher than usual.
    FlipJump,
}

/// What Jak's moves on foot keep from one frame and state to the next.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FootInfo {
    pub roll_count: i32,
    pub last_attack_end_time: i64,
    pub last_running_attack_end_time: i64,
    pub last_roll_end_time: i64,
    pub hook: StateHook,
    pub hook_time: i64,
    /// The punch's table: its turn rates change as it goes.
    pub run_attack: Surface,
    /// The punch's speed: set from its animation, or a bounce back off a
    /// wall.
    pub punch_speed: f32,
    pub punch_bounce: f32,
    pub smack_time: i64,
    pub sliding_start_time: i64,
    pub move_start_time: i64,
    /// The heading the spin started from.
    pub saved_dir: Quat,
    pub flop_frames: u32,
    pub ducking: bool,
    pub prev_state_time: i64,
    /// When Jak last stood on ground that rules out a turn-around.
    pub no_turn_around_time: i64,
    rng: u32,
}

const LONG_AGO: i64 = i64::MIN / 4;

impl Default for FootInfo {
    fn default() -> Self {
        Self {
            roll_count: 0,
            last_attack_end_time: LONG_AGO,
            last_running_attack_end_time: LONG_AGO,
            last_roll_end_time: LONG_AGO,
            hook: StateHook::None,
            hook_time: LONG_AGO,
            run_attack: surface::foot::RUN_ATTACK,
            punch_speed: 0.0,
            punch_bounce: 0.0,
            smack_time: 0,
            sliding_start_time: 0,
            move_start_time: 0,
            saved_dir: Quat::IDENTITY,
            flop_frames: 0,
            ducking: false,
            prev_state_time: 0,
            no_turn_around_time: 0,
            rng: 0x1234_5678,
        }
    }
}

impl FootInfo {
    /// True three times in ten, from a fixed sequence.
    pub(crate) fn chance(&mut self, p: f32) -> bool {
        let mut x = self.rng;
        x ^= x << 13;
        x ^= x >> 17;
        x ^= x << 5;
        self.rng = x;
        ((x >> 8) as f32 / (1u32 << 24) as f32) < p
    }
}

impl Jak {
    pub fn pilot_get_off(&mut self, bike: bool) { self.go(State::PilotGetOff { bike }); }

    /// `powerup` accepts stance/walk only; it never interrupts an attack,
    /// jump, ride or existing pickup reaction, and never calls `hurt`.
    pub fn eco_powerup(&mut self) -> bool {
        if !matches!(self.state, State::Stance | State::Walk) || self.swim.is_some()
            || self.time < self.hurled_until { return false; }
        self.powerup_light = false;
        self.go(State::EcoPowerup);
        true
    }

    /// [`Self::eco_powerup`] for light eco (`target-eco-powerup` 8).
    pub fn eco_powerup_light(&mut self) -> bool {
        let reacted = self.eco_powerup();
        self.powerup_light = reacted;
        reacted
    }

    /// Drawn as `target-eco-powerup` draws light eco's: no textures, the
    /// rim lights (`te2`, `te5`), for as long as the reaction lasts.
    pub fn rim_lit(&self) -> bool {
        self.state == State::EcoPowerup && self.powerup_light
    }

    pub(crate) fn on_foot_walking(&self) -> bool {
        self.state == State::Walk
    }

    fn walk_mods(&self) -> Surface {
        if self.gun.out {
            surface::GUN_WALK
        } else {
            surface::WALK
        }
    }

    pub(crate) fn clear_ground(&mut self) {
        self.control.status &= !(status::ON_SURFACE | status::ON_GROUND | status::TOUCH_SURFACE);
    }

    /// Ducking lowers every body sphere to the bottom one.
    pub(crate) fn collide_set(&mut self, duck: bool) {
        self.foot.ducking = duck;
        let r = self.control.body_radius;
        self.control.sphere_heights = if duck {
            [r, r, r]
        } else {
            self.control.standing_heights
        };
    }

    /// Jak 1 logic-target.gc `target-real-post`: a frog's tongue on Jak lerps the stick toward
    /// `force-turn-to-direction` × `force-turn-to-speed` (1) by `force-turn-to-strength`.
    fn tongue_pull(&mut self, pad: Vec3, speed: f32) -> (Vec3, f32) {
        let c = &mut self.control;
        c.tongue_adding = false;
        if self.time >= c.tongue_until || c.tongue_strength == 0.0 {
            return (pad, speed);
        }
        let k = c.tongue_strength.min(1.0);
        let from = if speed == 0.0 { Vec3::ZERO } else { pad * speed };
        let v = from.lerp(c.tongue_dir, k);
        (v.normalize_or_zero(), v.length())
    }

    /// Jak 1 target-handler.gc `target-apply-tongue`: a swamp frog's tongue, its base at `from`, pulls Jak's
    /// steering toward it, harder the farther he is (0.15 at 5 m to 0.22 at 15 m, `*TARGET-bank*`).
    /// stand-in: the original's prevent-attack and prevent-duck flags are not kept.
    pub fn apply_tongue(&mut self, from: Vec3) {
        if !from.is_finite() {
            return;
        }
        let now = self.time;
        let c = &mut self.control;
        let to = from - c.trans;
        c.tongue_strength = crate::math::lerp_scale(0.15, 0.22, to.length(), 20480.0, 61440.0);
        if !c.tongue_adding {
            c.tongue_dir = Vec3::ZERO;
            c.tongue_adding = true;
        }
        let n = c.local_normal;
        c.tongue_dir += (to - n * to.dot(n)).normalize_or_zero();
        c.tongue_until = now + seconds(0.1);
    }

    /// What every move leaves behind it.
    pub(crate) fn target_exit(&mut self) {
        let c = &mut self.control;
        c.mod_surface = surface::WALK;
        c.draw_offset_y = 0.0;
        c.force_turn_to_strength = 0.0;
        c.bend_target = 0.0;
        self.danger_set(None);
    }

    fn standard_gravity(&mut self) {
        self.control.gravity_length = control::STANDARD_GRAVITY;
        self.control.gravity_max = control::STANDARD_GRAVITY_MAX;
    }

    fn up(&self) -> f32 {
        self.control.gravity_normal.dot(self.control.transv)
    }

    fn set_up(&mut self, up: f32) {
        let c = &mut self.control;
        c.transv = with_vertical(c.transv, c.gravity_normal, up);
    }

    fn surface_flags(&self) -> u32 {
        self.control.current.flags
    }

    pub(crate) fn can_feet(&self) -> bool {
        self.surface_flags() & (flag::NO_ATTACK | flag::NO_FEET) == 0
            && self.time_elapsed(self.foot.last_attack_end_time, ATTACK_TIMEOUT)
    }

    pub(crate) fn can_hands(&self, ground: bool) -> bool {
        self.surface_flags() & (flag::NO_ATTACK | flag::NO_HANDS) == 0
            && (!ground
                || (!self.time_elapsed(self.control.last_time_on_surface, GROUND_TIMEOUT)
                    && self.control.local_slope_z < 0.7))
            && self.time_elapsed(
                self.foot.last_running_attack_end_time,
                // (Jak 1's yellow eco quickens this to `eco_blast::ATTACK_TIMEOUT`;
                // Adrii 22/10: no yellow shot, so never.)
                ATTACK_TIMEOUT,
            )
    }

    fn can_duck(&self) -> bool {
        self.control.on_surface()
            && self.control.local_normal.dot(self.control.gravity_normal) >= 0.7
            && !self.light()
    }

    /// Room to stand up: the two upper body spheres clear of the world.
    pub(crate) fn can_exit_duck(&mut self, world: &mut dyn CollideWorld) -> bool {
        let base = self.control.trans;
        let r = self.control.body_radius;
        let points = [
            self.control.standing_heights[2],
            self.control.standing_heights[1],
        ]
        .map(|y| base + Vec3::new(0.0, y, 0.0));
        let lo = base + Vec3::new(-r, self.control.standing_heights[1] - r, -r);
        let hi = base + Vec3::new(r, self.control.standing_heights[2] + r, r);
        self.probe_cache.fill_box(world, lo, hi);
        points
            .iter()
            .all(|&p| !self.probe_cache.overlaps_sphere(p, r))
    }

    fn can_roll(&self) -> bool {
        let c = &self.control;
        (!self.time_elapsed(c.last_time_on_surface, seconds(0.1)) || c.on_surface())
            && (c.status & status::TOUCH_WALL == 0 || 0.7 >= c.touch_angle)
            && c.local_slope_z < 0.7
            && self.surface_flags() & flag::NO_ATTACK == 0
            && !self.light()
    }

    /// Up against a wall square-on.
    pub(crate) fn smack_surface(&self, actors_too: bool) -> bool {
        let c = &self.control;
        0.7 < c.touch_angle
            && c.surface_angle < 0.3
            && c.status & status::TOUCH_WALL != 0
            && (actors_too || c.status & status::TOUCH_ACTOR == 0)
    }

    pub(crate) fn move_legs(&self) -> bool {
        self.pad.stick0_speed != 0.0
    }

    fn fall_test(&self) -> bool {
        let c = &self.control;
        !c.on_surface()
            && self.time_elapsed(c.last_time_on_surface, GROUND_TIMEOUT)
            && c.gravity_normal.dot(c.transv) <= 0.0
            && c.height_above_ground() >= FALL_HEIGHT
    }

    fn can_dive(&self) -> bool {
        self.time_elapsed(self.control.last_time_of_stuck, STUCK_TIMEOUT)
            && self.surface_flags() & (flag::NO_ATTACK | flag::NO_HANDS) == 0
    }

    fn dive(&self) -> State {
        State::Flop {
            forward: self.pad.stick0_speed != 0.0,
        }
    }

    fn jump(&self) -> State {
        State::Jump {
            min: JUMP_HEIGHT_MIN,
            max: JUMP_HEIGHT_MAX,
        }
    }

    fn uppercut(&self) -> State {
        State::AttackUppercut {
            min: ATTACK_JUMP_HEIGHT_MIN,
            max: ATTACK_JUMP_HEIGHT_MAX,
        }
    }

    /// In the air: a spin, landing, or stuck long enough to count as
    /// landed.
    pub(crate) fn falling_trans(&mut self, stuck_after: i64, air_spin: bool) -> Option<State> {
        if air_spin && self.pad.pressed(button::CIRCLE) && self.can_feet() {
            return Some(State::AttackAir {
                from: AirFrom::Jump,
            });
        }
        if self.control.on_surface() {
            return Some(State::HitGround { stuck: false });
        }
        // `target-falling-trans`: Triangle as Light Jak heals in the air.
        if self.regen_wanted() {
            return Some(State::LightJakRegen);
        }
        // Stuck counts as landed only on something: hanging on a block's
        // edge against a wall (the Minecraft world is all such edges) gave
        // him his jumps back in the air, and he climbed walls with them.
        if stuck_after >= 0
            && self.support_below
            && self.control.move_dist(self.time, STUCK_TIME) < STUCK_DISTANCE
            && self.time_elapsed(self.state_time, stuck_after)
        {
            self.control.status |= status::ON_SURFACE;
            return Some(State::HitGround { stuck: true });
        }
        if self.state_time != self.time {
            return self.slide_down_test();
        }
        None
    }

    /// Off the ground a while and against a surface Jak cannot stand on.
    fn slide_down_test(&self) -> Option<State> {
        let c = &self.control;
        (c.status & (status::ON_SURFACE | status::TOUCH_EDGE) == 0
            && self.time_elapsed(c.last_time_on_surface, GROUND_TIMEOUT)
            && c.status & status::TOUCH_SURFACE != 0
            && cos(c.current.slope_slip_angle) < c.surface_angle)
            .then_some(State::SlideDown)
    }

    /// The stick turned back against a run: true once Jak has been moving
    /// near full speed and the stick points away from his velocity. Keeps
    /// the velocity history the skid starts from.
    fn turn_around(&mut self) -> bool {
        let pad = self.pad_in_surface().normalize_or_zero();
        let along = pad.dot(self.control.transv.normalize_or_zero());
        let c = &mut self.control;
        c.transv_history.rotate_right(1);
        c.transv_history[0] = (c.transv, c.ctrl_xz_vel);
        let mut fastest = -1000.0;
        let mut sum = 0.0;
        for (i, &(_, speed)) in c.transv_history.iter().enumerate() {
            sum += speed;
            if fastest < speed {
                c.idx_of_fastest_xz_vel = i;
                fastest = speed;
            }
        }
        c.average_xz_vel = sum / 16.0;
        if c.current.flags & flag::NO_TURN_AROUND != 0 {
            self.foot.no_turn_around_time = self.time;
        }
        let c = &self.control;
        self.time - self.foot.no_turn_around_time >= seconds(1.0)
            && along < 0.0
            && 0.8 * c.current.target_speed < c.average_xz_vel
            && 0.7 < c.pad_magnitude
            && self.time_elapsed(c.last_time_touching_actor, seconds(0.3))
            && self.time_elapsed(c.time_of_last_lc, seconds(0.3))
            && c.on_surface()
            && 0.7 < c.surface_angle
    }

    /// Faces Jak along `v`, level, at once.
    fn vector_turn_to(&mut self, v: Vec3) {
        let c = &mut self.control;
        c.dir_targ = forward_up_nopitch_quat(v.normalize_or_zero(), y_axis(c.dir_targ));
        c.quat = c.dir_targ;
        self.build_conversions();
    }

    /// The window a finished move leaves open, checked first thing by the
    /// standing and ducking states.
    fn run_state_hook(&mut self) -> Option<State> {
        let since = self.foot.hook_time;
        match self.foot.hook {
            StateHook::None => None,
            StateHook::RollJump => {
                if self.time_elapsed(since, ROLL_JUMP_POST_WINDOW) {
                    self.foot.hook = StateHook::None;
                    None
                } else if self.pad.recently_pressed(button::X) && self.can_jump_roll_flip() {
                    Some(State::RollFlip {
                        height: ROLL_FLIP_HEIGHT,
                        dist: ROLL_FLIP_DIST,
                    })
                } else {
                    None
                }
            }
            StateHook::FlipJump => {
                if self.time_elapsed(since, seconds(0.1)) {
                    self.foot.hook = StateHook::None;
                    None
                } else if self.pad.recently_pressed(button::X) && self.can_jump(false) {
                    Some(State::HighJump {
                        min: FLIP_JUMP_HEIGHT_MIN,
                        max: FLIP_JUMP_HEIGHT_MAX,
                        kind: HighJump::Flip,
                    })
                } else {
                    None
                }
            }
        }
    }

    pub(crate) fn can_jump_roll_flip(&self) -> bool {
        self.can_jump(false) && 0.5 >= self.control.local_slope_z
    }

    pub(crate) fn set_hook(&mut self, hook: StateHook) {
        self.foot.hook = hook;
        self.foot.hook_time = self.time;
    }

    /// The new state's entry; a state can turn straight into another here.
    pub(crate) fn foot_enter(&mut self, state: &State) -> Option<State> {
        let now = self.time;
        if matches!(state, State::DoubleJump { .. }) && self.swoop_instead() {
            return Some(State::LightJakSwoop {
                first: true,
                held: 1.2,
            });
        }
        self.lightjak_enter(state);
        match *state {
            State::EdgeGrab => self.edge_grab_enter(),
            State::Stance | State::Walk => {
                self.control.mod_surface = self.walk_mods();
            }
            State::PilotGetOff { .. } => {
                self.control.mod_surface = surface::DOUBLE_JUMP;
                self.gun.end_mode();
            }
            State::EcoPowerup => {
                self.control.mod_surface = surface::DOUBLE_JUMP;
            }
            State::TurnAround => {
                let v = self.control.transv;
                self.vector_turn_to(v);
                self.control.mod_surface = surface::TURN_AROUND;
                self.control.bend_target = 1.0;
            }
            State::SlideDown => self.control.mod_surface = surface::JUMP,
            State::HitGroundHard { .. } => {
                self.foot.last_running_attack_end_time = 0;
                self.foot.last_attack_end_time = 0;
                self.set_forward_vel(0.0);
                self.control.mod_surface = surface::foot::HIT_GROUND_HARD;
            }
            State::DuckStance { keep_time } | State::DuckWalk { keep_time } => {
                if keep_time {
                    self.state_time = self.foot.prev_state_time;
                }
                self.control.bend_target = 1.0;
                let rolling = self.chan.is_any(&[anim::DUCK_ROLL]);
                if matches!(state, State::DuckStance { .. }) || !rolling {
                    self.control.mod_surface = surface::foot::DUCK;
                }
                self.collide_set(true);
            }
            State::Jump { min, max } => {
                self.events.push(Event::Jump);
                self.init_var_jump(min, max, true, true, 2.0);
                self.clear_ground();
                self.control.mod_surface = surface::JUMP;
            }
            State::DoubleJump { min, max } => {
                self.events.push(Event::Jump);
                self.init_var_jump(min, max, true, true, 2.0);
                self.clear_ground();
                self.control.mod_surface = surface::DOUBLE_JUMP;
            }
            State::HighJump { min, max, kind } => {
                self.events.push(Event::Jump);
                self.clear_ground();
                self.init_var_jump(min, max, true, true, 2.0);
                self.control.mod_surface = match kind {
                    HighJump::Flip => surface::foot::FLIP_JUMP,
                    HighJump::FlopForward => surface::foot::FORWARD_HIGH_JUMP,
                    _ => surface::foot::HIGH_JUMP,
                };
            }
            State::DuckHighJump { .. } => {
                self.clear_ground();
                self.control.mod_surface = surface::TURN_AROUND;
            }
            State::DuckHighJumpJump { min, max, .. } => {
                self.events.push(Event::Jump);
                self.init_var_jump(min, max, true, false, 2.0);
                self.clear_ground();
                self.control.mod_surface = surface::foot::HIGH_JUMP;
            }
            State::Falling { uppercut } => {
                self.control.mod_surface = if uppercut {
                    surface::foot::UPPERCUT_JUMP
                } else {
                    surface::JUMP
                };
            }
            State::HitGround { stuck } => {
                self.control.turn_go_the_long_way = 0.0;
                if !stuck {
                    let c = &self.control;
                    let fall = c.gravity_normal.dot(c.last_trans_any_surf - c.trans);
                    if FALL_FAR < fall && c.status & status::ON_WATER == 0 {
                        return Some(State::HitGroundHard { height: fall });
                    }
                    self.events.push(Event::Land);
                }
                self.foot.last_running_attack_end_time = 0;
                self.foot.last_attack_end_time = 0;
                if !self.slow_falling && self.control.ground_impact_vel >= FALL_STUMBLE_THRESHOLD {
                    self.set_forward_vel(0.0);
                }
                self.delete_back_vel();
                self.control.mod_surface = surface::WALK;
            }
            State::Attack => {
                self.start_attack();
                self.danger_set(Some(Danger::Spin));
                self.control.mod_surface = surface::foot::ATTACK;
                let c = &mut self.control;
                if 0.0 < c.turn_to_magnitude {
                    c.dir_targ = forward_up_nopitch_quat(c.to_target_pt_xz, y_axis(c.dir_targ));
                }
                c.quat = c.dir_targ;
                self.foot.saved_dir = self.control.dir_targ;
            }
            State::RunningAttack => {
                // (Jak 1's yellow eco makes this punch a shot, `target-yellow-
                // blast`, kept in `eco_blast`; Adrii 22/10: its auto-aim
                // went wrong, so the punch stays a punch.)
                self.pad.clear(button::SQUARE);
                let f = &mut self.foot;
                f.move_start_time = now;
                f.sliding_start_time = 0;
                f.smack_time = 0;
                f.punch_bounce = 0.0;
                let slide = self.control.ground_pat.event == crate::collide::PatEvent::Slide;
                let turn = if slide { 0.0 } else { 655360.0 };
                f.run_attack.turnv = turn;
                f.run_attack.turnvv = turn;
                self.control.mod_surface = self.foot.run_attack;
                let c = &mut self.control;
                if c.local_slope_x.abs() < 0.3 || 0.3 < c.local_slope_z.abs() {
                    c.bend_target = 1.0;
                }
            }
            State::AttackAir { from } => {
                self.clear_ground();
                self.start_attack();
                self.danger_set(Some(Danger::SpinAir));
                self.control.mod_surface = surface::foot::JUMP_ATTACK;
                let up = self.up();
                if 0.0 >= up || from == AirFrom::Flop {
                    self.set_up(33775.48);
                } else {
                    let g = self.control.gravity_length * SECONDS_PER_FRAME;
                    let mut rise = (up / g) / 2.0 * SECONDS_PER_FRAME * up;
                    if self.chan.is(anim::ATTACK_UPPERCUT) {
                        let gained = self
                            .control
                            .gravity_normal
                            .dot(self.control.trans - self.control.last_trans_any_surf);
                        rise = rise.max(ATTACK_JUMP_HEIGHT_MAX - gained);
                    }
                    self.set_up(1024.0 + (245760.0 * (2048.0 + rise)).sqrt());
                }
                self.control.gravity_length = 122880.0;
                self.control.last_trans_any_surf = self.control.trans;
            }
            State::AttackUppercut { .. } => {
                self.start_attack();
                self.danger_set(Some(Danger::Uppercut));
                self.control.mod_surface = surface::foot::UPPERCUT;
            }
            State::AttackUppercutJump { min, max } => {
                if self.control.ground_pat.material.is_ice()
                    && 32768.0 < self.control.ctrl_xz_vel
                {
                    self.set_forward_vel(32768.0);
                }
                self.init_var_jump(min, max, true, false, 2.0);
                self.clear_ground();
                self.control.mod_surface = surface::foot::UPPERCUT_JUMP;
                self.start_attack();
                self.danger_set(Some(Danger::Uppercut));
            }
            State::Flop { forward } => {
                let speed = if forward {
                    self.control.ctrl_xz_vel
                } else {
                    0.0
                };
                self.set_forward_vel(speed);
                self.clear_ground();
                self.control.mod_surface = surface::foot::FLOP;
                self.foot.flop_frames = 0;
                self.control.gravity_max = 245760.0;
                self.control.gravity_length = 245760.0;
                self.set_up(FLOP_UP);
            }
            State::FlopHitGround { .. } => {
                let c = &self.control;
                let fall = c.gravity_normal.dot(c.highest_jump_mark - c.trans);
                if FALL_FAR < fall {
                    return Some(State::HitGroundHard { height: fall });
                }
                self.events.push(Event::Land);
                self.events.push(Event::GroundPound { pos: self.control.trans });
                self.set_forward_vel(0.0);
                let mut land = surface::foot::FLOP_LAND;
                land.flags &= !flag::CHECK_EDGE;
                self.control.mod_surface = land;
            }
            State::Roll => {
                self.collide_set(true);
                self.control.mod_surface = surface::foot::ROLL;
                self.foot.roll_count += 1;
                self.control.dir_targ = self.control.quat;
                let speed = ROLL_SPEED_MIN + ROLL_SPEED_INC * (self.foot.roll_count - 1) as f32;
                self.set_forward_vel(speed);
                let c = &mut self.control;
                if c.local_slope_x.abs() < 0.3 || 0.3 < c.local_slope_z.abs() {
                    c.bend_target = 1.0;
                }
            }
            State::RollFlip { .. } => {
                self.control.mod_surface = surface::foot::ROLL_FLIP;
                self.collide_set(true);
            }
            _ => {}
        }
        None
    }

    pub(crate) fn foot_exit(&mut self, state: &State, next: &State) {
        let now = self.time;
        self.lightjak_exit(state, next);
        match *state {
            State::Stance => {
                self.control.bend_target = 0.0;
                self.foot.hook = StateHook::None;
            }
            State::Walk => self.foot.hook = StateHook::None,
            State::TurnAround => {
                self.foot.hook = StateHook::None;
                self.set_forward_vel(0.0);
                let c = &mut self.control;
                c.ctrl_xz_vel = 0.0;
                c.quat = c.dir_targ;
                c.bend_target = 0.0;
            }
            State::DuckStance { .. } | State::DuckWalk { .. } => {
                if !matches!(
                    next,
                    State::DuckStance { .. } | State::DuckWalk { .. } | State::Walk | State::Stance
                ) {
                    self.foot.hook = StateHook::None;
                }
                self.target_exit();
                if self.foot.ducking {
                    self.collide_set(false);
                }
            }
            State::Attack => {
                self.control.dir_targ = self.foot.saved_dir;
                self.foot.last_attack_end_time = now;
                self.target_exit();
            }
            State::RunningAttack => {
                self.standard_gravity();
                self.foot.run_attack.turnv = 0.0;
                self.foot.run_attack.turnvv = 0.0;
                self.foot.last_running_attack_end_time = now;
                self.target_exit();
            }
            State::AttackAir { .. } => {
                self.standard_gravity();
                self.foot.last_attack_end_time = now;
                self.target_exit();
            }
            State::Flop { .. } => {
                self.danger_set(None);
                self.standard_gravity();
            }
            State::Roll => {
                if *next != State::Roll {
                    self.foot.roll_count = 0;
                    self.foot.last_roll_end_time = now;
                }
                self.target_exit();
                self.collide_set(false);
            }
            State::RollFlip { .. } => {
                self.target_exit();
                self.collide_set(false);
            }
            State::Jump { .. }
            | State::DoubleJump { .. }
            | State::HighJump { .. }
            | State::DuckHighJump { .. }
            | State::DuckHighJumpJump { .. }
            | State::Falling { .. }
            | State::AttackUppercut { .. }
            | State::AttackUppercutJump { .. }
            | State::FlopHitGround { .. }
            | State::HitGroundHard { .. }
            | State::EcoPowerup
            | State::PilotGetOff { .. } => self.target_exit(),
            _ => {}
        }
    }

    pub(crate) fn foot_trans(&mut self, world: &mut dyn CollideWorld) -> Option<State> {
        let can_deploy_board = matches!(
            self.state,
            State::Stance
                | State::Walk
                | State::DuckStance { .. }
                | State::DuckWalk { .. }
                | State::HitGround { .. }
                | State::Jump { .. }
                | State::DoubleJump { .. }
                | State::HighJump { .. }
                | State::DuckHighJumpJump { .. }
                | State::Falling { .. }
                | State::AttackAir { .. }
                | State::AttackUppercutJump { .. }
        );
        if can_deploy_board && !self.dark && self.want_to_board(world) {
            self.board_init();
            self.events.push(Event::BoardOn);
            return Some(State::BoardGetOn);
        }
        let x = self.pad.recently_pressed(button::X);
        // Falling past a ledge with his hands at its top: he catches it.
        if matches!(
            self.state,
            State::Jump { .. }
                | State::DoubleJump { .. }
                | State::HighJump { .. }
                | State::DuckHighJumpJump { .. }
                | State::Falling { .. }
        ) {
            if let Some(ledge) = self.find_ledge(world) {
                self.ledge = Some(ledge);
                return Some(State::EdgeGrab);
            }
            let feet = self.control.trans + glam::Vec3::Y * meters(0.3);
            self.support_below = Self::probe(
                world,
                feet,
                glam::Vec3::new(0.0, -meters(0.6), 0.0),
                meters(0.1),
            )
            .is_some();
        }
        match self.state {
            State::EcoPowerup => {
                if x { return Some(State::Jump { min: JUMP_HEIGHT_MIN, max: JUMP_HEIGHT_MAX }); }
                if self.pad.pressed(button::CIRCLE) && self.can_feet() {
                    return Some(State::AttackAir { from: crate::AirFrom::Jump });
                }
                if self.pad.pressed(button::SQUARE) && self.can_hands(false) { return Some(State::RunningAttack); }
                self.slide_down_test()
            }

            State::EdgeGrab => return self.edge_grab_trans(world),
            State::Stance => {
                if let Some(next) = self.run_state_hook() {
                    return Some(next);
                }
                if self.regen_wanted() {
                    return Some(State::LightJakRegen);
                }
                if self.move_legs() {
                    self.control.bend_target = 0.0;
                    self.code.no_exit = true;
                    return Some(State::Walk);
                }
                if self.want_to_powerjak() {
                    return Some(State::PowerJakGetOn);
                }
                if self.pad.hold(button::L1) && self.can_duck() {
                    self.control.bend_target = 0.0;
                    self.code.no_exit = true;
                    return Some(State::DuckStance { keep_time: false });
                }
                self.ground_actions(x, false)
            }
            State::Walk => {
                if let Some(next) = self.run_state_hook() {
                    return Some(next);
                }
                if self.regen_wanted() {
                    return Some(State::LightJakRegen);
                }
                if self.pad.recently_pressed(button::L1) && self.move_legs() && self.can_roll() {
                    return Some(State::Roll);
                }
                if self.pad.hold(button::L1) && self.can_duck() {
                    self.code.no_exit = true;
                    return Some(State::DuckWalk { keep_time: false });
                }
                if !self.move_legs() {
                    self.code.no_exit = true;
                    return Some(State::Stance);
                }
                if self.want_to_powerjak() {
                    return Some(State::PowerJakGetOn);
                }
                self.ground_actions(x, true)
            }
            State::TurnAround => {
                if let Some(next) = self.run_state_hook() {
                    return Some(next);
                }
                if x && self.can_jump(false) {
                    return Some(self.jump());
                }
                if self.pad.pressed(button::CIRCLE) && self.can_feet() {
                    return Some(State::Attack);
                }
                if self.pad.pressed(button::SQUARE) && self.can_hands(true) {
                    return Some(State::RunningAttack);
                }
                if !self.control.on_surface()
                    && self.time_elapsed(self.control.last_time_on_surface, seconds(0.08))
                {
                    return Some(State::Falling { uppercut: false });
                }
                self.slide_down_test()
            }
            State::SlideDown => {
                let c = &self.control;
                if c.on_surface() || c.move_dist(self.time, STUCK_TIME) < STUCK_DISTANCE {
                    self.control.status |= status::ON_SURFACE;
                    return Some(if self.gun.out {
                        State::Stance
                    } else {
                        State::DuckStance { keep_time: false }
                    });
                }
                None
            }
            State::DuckStance { .. } | State::DuckWalk { .. } => {
                if let Some(next) = self.run_state_hook() {
                    return Some(next);
                }
                let walking = matches!(self.state, State::DuckWalk { .. });
                let rolling = self.chan.is_any(&[anim::DUCK_ROLL, anim::DUCK_ROLL_END]);
                if !self.pad.hold(button::L1) && (walking || !rolling) && self.can_exit_duck(world)
                {
                    return Some(if walking { State::Walk } else { State::Stance });
                }
                if !walking && self.move_legs() && self.control.current.seek0 != 0.0 {
                    return Some(State::DuckWalk { keep_time: true });
                }
                if walking && !self.move_legs() {
                    return Some(State::DuckStance { keep_time: true });
                }
                if self.want_to_powerjak() {
                    return Some(State::PowerJakGetOn);
                }
                if x && self.can_jump(false) {
                    return Some(if self.pad.stick0_speed == 0.0 {
                        State::DuckHighJump {
                            min: DUCK_JUMP_HEIGHT_MIN,
                            max: DUCK_JUMP_HEIGHT_MAX,
                            kind: HighJump::Duck,
                        }
                    } else {
                        self.jump()
                    });
                }
                if self.pad.pressed(button::SQUARE)
                    && self.can_hands(true)
                    && self.can_exit_duck(world)
                    && self.surface_flags() & flag::NO_JUMP == 0
                {
                    return Some(self.uppercut());
                }
                if self.fall_test() {
                    return Some(State::Falling { uppercut: false });
                }
                self.slide_down_test()
            }
            State::HitGround { .. } => {
                if x && self.can_jump(false) {
                    return Some(self.jump());
                }
                if (self.pad.hold(button::L1) || !self.can_exit_duck(world)) && self.can_duck() {
                    return Some(State::DuckStance { keep_time: false });
                }
                if self.move_legs() {
                    return Some(State::Walk);
                }
                if self.want_to_powerjak() {
                    return Some(State::PowerJakGetOn);
                }
                if self.pad.recently_pressed(button::CIRCLE) && self.can_feet() {
                    return Some(State::Attack);
                }
                if self.pad.recently_pressed(button::SQUARE) && self.can_hands(true) {
                    return Some(State::RunningAttack);
                }
                if self.fall_test() {
                    return Some(State::Falling { uppercut: false });
                }
                self.slide_down_test()
            }
            State::Jump { .. } | State::DoubleJump { .. } => {
                let double = matches!(self.state, State::DoubleJump { .. });
                let stuck_after = if self.chan.is(anim::JUMP_LOOP) {
                    15
                } else {
                    -1
                };
                if let Some(next) = self.falling_trans(stuck_after, true) {
                    return Some(next);
                }
                let up = self.up();
                if !double && self.pad.pressed(button::X) && up < 12288.0 && -116736.0 < up {
                    return Some(State::DoubleJump {
                        min: DOUBLE_JUMP_HEIGHT_MIN,
                        max: DOUBLE_JUMP_HEIGHT_MAX,
                    });
                }
                let ceiling = if double { 22118.4 } else { 26624.0 };
                if self.pad.pressed(button::SQUARE)
                    && up < ceiling
                    && -61440.0 < up
                    && self.can_dive()
                {
                    return Some(self.dive());
                }
                if !double || self.state_time != self.time {
                    self.board_var_jump_foot();
                }
                if double { None } else { self.slide_down_test() }
            }
            State::HighJump { .. } | State::DuckHighJumpJump { .. } => {
                let stuck_after = if self.chan.is(anim::JUMP_LOOP) {
                    15
                } else {
                    -1
                };
                if let Some(next) = self.falling_trans(stuck_after, true) {
                    return Some(next);
                }
                let up = self.up();
                if self.pad.recently_pressed(button::SQUARE)
                    && up < 73728.0
                    && -61440.0 < up
                    && self.can_dive()
                {
                    return Some(self.dive());
                }
                self.board_var_jump_foot();
                None
            }
            State::Falling { uppercut } => {
                let stuck_after = if uppercut { STUCK_TIME / 2 } else { 0 };
                self.falling_trans(stuck_after, true)
            }
            State::RunningAttack => self.running_attack_trans(),
            State::AttackAir { .. } => {
                if self.control.on_surface() {
                    self.control.quat = self.control.dir_targ;
                    return Some(State::HitGround { stuck: false });
                }
                if self.time_elapsed(self.state_time, seconds(0.5)) {
                    let c = &mut self.control;
                    c.gravity_length = seek(
                        c.gravity_length,
                        control::STANDARD_GRAVITY,
                        245760.0 * SECONDS_PER_FRAME,
                    );
                }
                let c = &self.control;
                if self.time_elapsed(self.state_time, seconds(0.05))
                    && c.gravity_normal.dot(c.last_transv) < c.gravity_normal.dot(c.transv)
                {
                    self.standard_gravity();
                }
                None
            }
            State::AttackUppercutJump { .. } => {
                if self.control.on_surface() {
                    return Some(State::HitGround { stuck: false });
                }
                let up = self.up();
                if self.pad.pressed(button::SQUARE)
                    && up < 22118.4
                    && -61440.0 < up
                    && self.can_dive()
                {
                    self.control.quat = self.control.dir_targ;
                    self.build_conversions();
                    return Some(self.dive());
                }
                if self.pad.pressed(button::CIRCLE)
                    && self.can_feet()
                    && self.chan.is(anim::ATTACK_UPPERCUT)
                    && self.chan.aframe_num(&self.anims) >= 12.0
                {
                    return Some(State::AttackAir {
                        from: AirFrom::Uppercut,
                    });
                }
                self.board_var_jump_foot();
                if self.attack.danger == Some(Danger::Uppercut) && self.up() < -8192.0 {
                    self.danger_set(None);
                }
                None
            }
            State::Flop { .. } => self.flop_trans(),
            State::FlopHitGround { stuck } => {
                if !stuck
                    && self.state_time != self.time
                    && self.pad.recently_pressed(button::CIRCLE)
                    && self.can_feet()
                {
                    return Some(State::AttackAir {
                        from: AirFrom::Flop,
                    });
                }
                if self.chan.is(anim::FLOP_DOWN_LAND) && self.chan.aframe_num(&self.anims) >= 28.0 {
                    self.control.mod_surface.flags |= flag::CHECK_EDGE;
                }
                None
            }
            State::LightJakSwoop { .. } | State::LightJakSwoopFalling => self.lightjak_trans(),
            State::RollFlip { .. } => {
                if self.pad.pressed(button::CIRCLE)
                    && self.can_feet()
                    && self.chan.is(anim::JUMP_LOOP)
                {
                    return Some(State::AttackAir {
                        from: AirFrom::Jump,
                    });
                }
                None
            }
            _ => None,
        }
    }

    /// What the current state answers to, as `button move`, and whether
    /// each would be taken now. Duck exits and walls are left out.
    pub fn moves(&self) -> Vec<(&'static str, bool)> {
        let feet = self.can_feet();
        let jump = self.can_jump(false);
        let up = self.up();
        let dive = self.can_dive();
        let mut moves = match self.state {
            State::Stance | State::Walk | State::HitGround { .. } => {
                let mut m = vec![
                    ("X jump", jump),
                    ("O spin", feet),
                    ("[] punch", self.can_hands(true)),
                    ("L1 duck", self.can_duck()),
                ];
                if self.state == State::Walk {
                    m.push(("L1 roll", self.move_legs() && self.can_roll()));
                }
                m
            }
            State::DuckStance { .. } | State::DuckWalk { .. } => vec![
                if self.pad.stick0_speed == 0.0 {
                    ("X high jump", jump)
                } else {
                    ("X jump", jump)
                },
                (
                    "[] uppercut",
                    self.can_hands(true) && self.surface_flags() & flag::NO_JUMP == 0,
                ),
            ],
            State::Jump { .. } => vec![
                ("X double jump", up < 12288.0 && -116736.0 < up),
                ("[] dive", up < 26624.0 && -61440.0 < up && dive),
                ("O air spin", feet),
            ],
            State::DoubleJump { .. } => vec![
                ("[] dive", up < 22118.4 && -61440.0 < up && dive),
                ("O air spin", feet),
            ],
            State::HighJump { .. } | State::DuckHighJumpJump { .. } => vec![
                ("[] dive", up < 73728.0 && -61440.0 < up && dive),
                ("O air spin", feet),
            ],
            State::TurnAround => vec![
                ("X jump", jump),
                ("O spin", feet),
                ("[] punch", self.can_hands(true)),
            ],
            State::Falling { .. } => vec![("O air spin", feet)],
            State::Attack => vec![("X jump", jump)],
            State::RunningAttack => vec![(
                "X uppercut",
                4096.0 < self.control.ctrl_xz_vel
                    && (self.time_elapsed(self.state_time, seconds(0.1))
                        || !self.pad.hold(button::SQUARE)),
            )],
            State::AttackUppercutJump { .. } => vec![
                ("[] dive", up < 22118.4 && -61440.0 < up && dive),
                (
                    "O air spin",
                    feet && self.chan.is(anim::ATTACK_UPPERCUT)
                        && self.chan.aframe_num(&self.anims) >= 12.0,
                ),
            ],
            State::FlopHitGround { stuck } => vec![
                ("O air spin", !stuck && feet),
                (
                    "X flop jump",
                    !stuck && (21.0..=25.0).contains(&self.chan.aframe_num(&self.anims)) && jump,
                ),
            ],
            State::Roll => vec![("X roll flip", self.can_jump_roll_flip())],
            State::RollFlip { .. } => vec![("O air spin", feet && self.chan.is(anim::JUMP_LOOP))],
            _ => Vec::new(),
        };
        match self.foot.hook {
            StateHook::RollJump
                if !self.time_elapsed(self.foot.hook_time, ROLL_JUMP_POST_WINDOW) =>
            {
                moves.push(("X roll flip", self.can_jump_roll_flip()));
            }
            StateHook::FlipJump if !self.time_elapsed(self.foot.hook_time, seconds(0.1)) => {
                moves.push(("X flip jump", jump));
            }
            _ => {}
        }
        moves
    }

    /// The window a finished move left open, and how long it has left.
    pub fn cancel_window(&self) -> Option<(&'static str, i64)> {
        let (name, length) = match self.foot.hook {
            StateHook::None => return None,
            StateHook::RollJump => ("roll-jump", ROLL_JUMP_POST_WINDOW),
            StateHook::FlipJump => ("flip-jump", seconds(0.1)),
        };
        let left = self.foot.hook_time + length - self.time;
        (left > 0).then_some((name, left))
    }

    /// The moves standing and walking share: jump, spin, punch, a skid
    /// round while walking, sliding or falling off.
    fn ground_actions(&mut self, x: bool, walking: bool) -> Option<State> {
        if x && self.can_jump(false) {
            return Some(self.jump());
        }
        if self.pad.recently_pressed(button::CIRCLE) && self.can_feet() {
            return Some(State::Attack);
        }
        if self.pad.recently_pressed(button::SQUARE) && self.can_hands(true) {
            return Some(State::RunningAttack);
        }
        if walking && self.turn_around() && self.time_elapsed(self.state_time, seconds(0.3)) {
            let (fastest, _) = self.control.transv_history[self.control.idx_of_fastest_xz_vel];
            self.control.transv = fastest;
            return Some(State::TurnAround);
        }
        if let Some(next) = self.slide_down_test() {
            return Some(next);
        }
        self.fall_test()
            .then_some(State::Falling { uppercut: false })
    }

    /// The jump window and the held jump on foot.
    fn board_var_jump_foot(&mut self) {
        self.control.jump_window = self.control.jump_window.max(self.pad.pressure(button::X));
        self.mod_var_jump(self.pad.hold(button::X));
    }

    /// The punch: stopped by a wall or a steep slope, and X turns it into
    /// the uppercut while it still carries speed.
    fn running_attack_trans(&mut self) -> Option<State> {
        if self.state_time == self.time {
            return None;
        }
        let c = &self.control;
        if self.smack_surface(true)
            || (c.surface_slope_z >= 0.7 && c.status & status::TOUCH_ACTOR == 0)
        {
            self.foot.smack_time = self.time;
            self.control.bend_target = 0.0;
            let up = self.up().min(0.0);
            self.set_up(up);
        }
        let square_up = !self.pad.hold(button::SQUARE);
        if self.pad.pressed(button::X)
            && 4096.0 < self.control.ctrl_xz_vel
            && (self.time_elapsed(self.state_time, seconds(0.1)) || square_up)
            && self.surface_flags() & (flag::NO_ATTACK | flag::NO_JUMP) == 0
        {
            return Some(self.uppercut());
        }
        None
    }

    /// The dive: landed, or stuck on the way down; dangerous once its
    /// animation reaches the strike.
    fn flop_trans(&mut self) -> Option<State> {
        self.delete_back_vel();
        let mut landed = self.control.on_surface();
        let mut stuck = false;
        if !landed
            && self
                .chan
                .is_any(&[anim::FLOP_DOWN_LOOP, anim::MOVING_FLOP_DOWN])
        {
            let c = &self.control;
            if (c.move_dist(self.time, seconds(0.1)) < 1638.4
                || (c.status & status::TOUCH_WALL != 0 && 0.7 < c.poly_angle))
                && c.status & status::TOUCH_ACTOR == 0
                && self.foot.flop_frames >= 2
            {
                self.control.last_time_of_stuck = self.time;
                landed = true;
                stuck = true;
            }
        }
        if landed {
            self.control.status |= status::ON_SURFACE;
            // The slam strikes what it lands on (a Minecraft block cracks
            // as under a pick, 15/10).
            if self.control.status & status::TOUCH_ACTOR == 0 {
                self.events.push(crate::Event::Strike {
                    actor: None,
                    pos: self.control.trans,
                    dir: Vec3::NEG_Y,
                    // Enough to break grass, dirt or sand at one slam,
                    // stone at two.
                    damage: 10.0,
                    sphere: None,
                });
            }
            return Some(State::FlopHitGround { stuck });
        }
        if self.attack.danger.is_none()
            && self.chan.is(anim::FLOP_DOWN)
            && self.chan.aframe_num(&self.anims) >= 8.0
        {
            self.start_attack();
            self.danger_set(Some(Danger::Flop));
        }
        None
    }

    /// R2: once there is room above Jak and the gun has been put away, he
    /// calls the board.
    fn want_to_board(&mut self, world: &mut dyn CollideWorld) -> bool {
        // Only with his board on him (in Minecraft, equipped).
        if !self.board_carried {
            self.board.latch = false;
            return false;
        }
        if self.pad.pressed(button::R2) && self.room_above(world) {
            self.board.latch = true;
        }
        let c = &self.control;
        self.board.latch
            && (self.board.board_time == i64::MIN / 4
                || self.time_elapsed(self.board.board_time, seconds(0.5)))
            && self.board.board_time < c.list_time_on_ground
            && c.current.flags & surface::flag::DUCK == 0
            && self.gun.time_since_use(self.time) >= seconds(0.4)
    }

    /// Three spheres stacked above Jak's head, clear of the world.
    fn room_above(&mut self, world: &mut dyn CollideWorld) -> bool {
        let base = self.control.trans;
        let r = 2867.2;
        let lo = base + Vec3::new(-r, 8192.0 - r, -r);
        let hi = base + Vec3::new(r, 16384.0 + r, r);
        self.probe_cache.fill_box(world, lo, hi);
        [8192.0, 12288.0, 16384.0].iter().all(|&y| {
            !self
                .probe_cache
                .overlaps_sphere(base + Vec3::new(0.0, y, 0.0), r)
        })
    }

    /// One frame on foot. The skid and the hard landing ignore the stick.
    pub(crate) fn target_post(&mut self, world: &mut dyn CollideWorld) {
        if self.state == State::BoardGetOff {
            self.control.bend_speed = 0.0;
            self.control.bend_target = 0.0;
            self.control.draw_offset_y =
                seek(self.control.draw_offset_y, 0.0, 16384.0 * SECONDS_PER_FRAME);
        }
        self.flag_setup();
        let no_stick = matches!(
            self.state,
            State::TurnAround
                | State::HitGroundHard { .. }
                | State::PowerJakGetOn
                | State::LightJakGetOn { .. }
                | State::LightJakGetOff
                | State::LightJakFreeze
        );
        if matches!(self.state, State::PowerJakGetOn | State::LightJakGetOff)
            && self.control.on_surface()
        {
            self.control.transv = Vec3::ZERO;
        }
        if self.control.force_turn_to_strength < 0.0 && !no_stick {
            self.control.force_turn_to_strength = 1.0 - self.pad.stick0_speed;
        }
        self.build_conversions();
        self.do_rotations1();
        let pad_dir = self.read_pad();
        let speed = if no_stick { 0.0 } else { self.debounce_speed() };
        let (pad_dir, speed) = self.tongue_pull(pad_dir, speed);
        self.turn_to_vector(pad_dir, speed);
        self.add_thrust();
        self.add_gravity();
        self.do_rotations2();
        self.reverse_conversions();
        self.pre_collide_setup();
        self.foot_step_up(world);
        self.integrate_and_collide(world);
        if matches!(self.state, State::Walk | State::Stance | State::DuckWalk { .. }) {
            if let Some(limit)=self.foot_smooth_step {self.terrain_settle(world,limit);}
        }
        self.bend_gravity();
        self.post_flag_setup();
        self.target_gspot(world);
        self.lightjak_process();
    }

    /// While the gun is out: turning eases to the aim while Jak walks, and
    /// snaps round while he stands.
    pub(crate) fn gun_walk_hook(&mut self, cur: &mut Surface) {
        let to_target = self.control.to_target_pt_xz;
        let off = deg_diff(self.control.y_angle(), y_angle(to_target));
        let want = lerp_scale(0.0, 1.0, off.abs(), 1820.4445, 6371.5557);
        let g = &mut self.gun;
        if g.turn_blend < want {
            g.turn_blend = seek(g.turn_blend, want, 4.0 * SECONDS_PER_FRAME);
        } else {
            g.turn_blend = seek(g.turn_blend, want, SECONDS_PER_FRAME);
        }
        let mut turnv = 131072.0;
        let mut turnvf = 30.0;
        if 1.0 < g.turn_blend {
            turnv = lerp_scale(131072.0, 291271.12, g.turn_blend, 1.0, 2.0);
            turnvf = lerp_scale(30.0, 15.0, g.turn_blend, 1.0, 2.0);
            cur.turnvv = turnv;
            cur.turnvvf = turnvf;
        }
        cur.turnv = turnv;
        cur.turnvf = turnvf;
        if self.state == State::Stance {
            cur.turnv = 364088.88;
            cur.turnvf = 30.0;
        }
    }
}
