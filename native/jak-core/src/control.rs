//! Jak's movement controller: the state every mode shares (position,
//! velocity, orientation, what he stands on) and the per-frame steps that
//! move it — turning the stick into a heading, thrust and gravity in his
//! local frame, the swept collision and what it does to his velocity.
use glam::{Quat, Vec3};

use crate::Jak;
use crate::collide::{CollideWorld, Pat, PatMode};
use crate::math::*;
use crate::surface::{self, Surface, flag};

pub mod status {
    pub const ON_SURFACE: u32 = 1 << 0;
    pub const ON_GROUND: u32 = 1 << 1;
    pub const TOUCH_SURFACE: u32 = 1 << 2;
    pub const TOUCH_WALL: u32 = 1 << 3;
    pub const TOUCH_CEILING: u32 = 1 << 4;
    pub const TOUCH_CEILING_STICKY: u32 = 1 << 5;
    pub const TOUCH_ACTOR: u32 = 1 << 6;
    pub const TOUCH_EDGE: u32 = 1 << 7;
    pub const BLOCKED: u32 = 1 << 8;
    pub const ON_WATER: u32 = 1 << 9;
    pub const IMPACT_SURFACE: u32 = 1 << 10;
    pub const TOUCH_BACKGROUND: u32 = 1 << 11;
    pub const GLANCE: u32 = 1 << 12;
    /// Cleared at the start of every collision step.
    pub const PER_STEP: u32 = ON_SURFACE
        | ON_GROUND
        | TOUCH_SURFACE
        | TOUCH_WALL
        | TOUCH_CEILING
        | TOUCH_ACTOR
        | TOUCH_EDGE
        | BLOCKED
        | ON_WATER
        | IMPACT_SURFACE
        | TOUCH_BACKGROUND
        | GLANCE;
}

/// How the last contact was classified, read by the code after the sweep.
pub mod reaction {
    pub const WALL_PAT: u32 = 1 << 0;
    pub const WALL: u32 = 1 << 1;
    pub const WALL_RESPONSE: u32 = 1 << 2;
    pub const LOW_COVERAGE: u32 = 1 << 3;
    pub const LOW_COVERAGE_ALSO: u32 = 1 << 4;
    pub const AIR: u32 = 1 << 5;
    pub const EDGE_WALL: u32 = 1 << 6;
    pub const LEDGE: u32 = 1 << 7;
    pub const CREASE: u32 = 1 << 8;
    pub const OVERHANG: u32 = 1 << 9;
    pub const EDGE_SURFACE: u32 = 1 << 10;
    pub const GROUND: u32 = 1 << 11;
    pub const EDGE_AIR_WALL: u32 = 1 << 12;
    pub const HAZARD: u32 = 1 << 14;
    pub const UPPER_SPHERE: u32 = 1 << 15;
}

/// The three spheres Jak collides with the world as, bottom first: their
/// heights above his origin and their radius.
pub const BODY_RADIUS: f32 = meters(0.7);
pub const SPHERE_HEIGHTS: [f32; 3] = [BODY_RADIUS, 2867.2 + BODY_RADIUS, 5734.4 + BODY_RADIUS];
/// The sphere the collision cache is filled around.
const ROOT_OFFSET_Y: f32 = 4915.2;
const ROOT_RADIUS: f32 = meters(2.2);
const MAX_ITERATIONS: usize = 8;
pub const STANDARD_GRAVITY: f32 = meters(60.0);
pub const STANDARD_GRAVITY_MAX: f32 = meters(40.0);
const STUCK_TIME: i64 = seconds(0.3);
const STUCK_DISTANCE: f32 = meters(0.05);
pub const GROUND_TIMEOUT: i64 = seconds(0.2);

#[derive(Clone, Copy, Debug, Default)]
pub struct LowCoverage {
    pub tangent: Vec3,
    pub tangent_xz: Vec3,
    pub overhang_normal: Vec3,
    pub slope_to_next1: f32,
    pub dist_to_next2: f32,
    pub pat_next1: Pat,
    pub pat_next2: Pat,
}

#[derive(Clone, Debug)]
pub struct Control {
    pub trans: Vec3,
    pub transv: Vec3,
    /// The heading and tilt the movement uses.
    pub quat: Quat,
    pub last_quat: Quat,
    /// Where `quat` is turning to.
    pub dir_targ: Quat,
    pub local_to_world: Basis,
    /// `transv` in the local frame (x across, y up the surface, z ahead).
    pub local_transv: Vec3,

    pub status: u32,
    pub old_status: u32,
    pub prev_status: u32,
    pub reaction: u32,
    pub no_normal_reset: bool,

    pub gravity_normal: Vec3,
    pub gravity_length: f32,
    pub gravity_max: f32,

    pub local_normal: Vec3,
    pub surface_normal: Vec3,
    pub poly_normal: Vec3,
    pub pre_collide_local_normal: Vec3,
    pub bent_gravity_normal: Vec3,
    pub ground_poly_normal: Vec3,
    pub ground_contact_normal: Vec3,
    pub ground_touch_point: Vec3,
    pub wall_contact_normal: Vec3,
    pub wall_contact_pt: Vec3,
    pub wall_contact_pat: Pat,
    pub gspot_pos: Vec3,
    pub gspot_normal: Vec3,
    pub gspot_pat: Pat,

    pub surface_angle: f32,
    pub poly_angle: f32,
    pub touch_angle: f32,
    pub coverage: f32,
    pub cur_pat: Pat,
    pub poly_pat: Pat,
    pub ground_pat: Pat,

    /// The ground's table.
    pub surf: Surface,
    pub prev_surf: Surface,
    /// The movement mode's table.
    pub mod_surface: Surface,
    /// The two multiplied for this frame.
    pub current: Surface,

    pub pad_magnitude: f32,
    pub last_pad_magnitude: f32,
    pub pad_xz_dir: Vec3,
    pub last_pad_xz_dir: Vec3,
    pub turn_to_magnitude: f32,
    pub turn_to_angle: f32,
    pub turn_to_target: Vec3,
    pub to_target_pt_xz: Vec3,
    pub target_transv: Vec3,
    pub turn_to_alt_heading: Vec3,

    pub ctrl_xz_vel: f32,
    pub local_slope_z: f32,
    pub local_slope_x: f32,
    pub surface_slope_z: f32,
    pub surface_slope_x: f32,
    pub gspot_slope_z: f32,
    pub gspot_slope_x: f32,
    pub ctrl_slope_z: f32,
    pub ctrl_slope_x: f32,
    pub ctrl_slope_heading: f32,

    pub btransv: Vec3,
    pub blocked_factor: f32,
    pub blocked_in_air_factor: f32,
    pub velocity_after_thrust: f32,

    pub bend_amount: f32,
    pub bend_speed: f32,
    pub bend_target: f32,
    pub draw_offset_y: f32,

    pub turn_lockout_end_time: i64,
    pub turn_go_the_long_way: f32,
    pub force_turn_to_strength: f32,

    pub last_time_on_surface: i64,
    pub list_time_on_ground: i64,
    pub last_time_touching_actor: i64,
    pub time_of_last_lc: i64,
    pub time_of_last_lc_touch_edge: i64,
    pub last_time_of_stuck: i64,
    pub time_of_last_clear_wall_in_jump: i64,
    pub time_of_last_surface_change: i64,
    pub last_trans_any_surf: Vec3,
    pub last_trans_leaving_surf: Vec3,
    /// The top of the current jump or fall, under Jak.
    pub highest_jump_mark: Vec3,
    pub last_transv: Vec3,
    /// The last frames' velocities and speeds, newest first, as the walk
    /// keeps them to tell a turn-around.
    pub transv_history: [(Vec3, f32); 16],
    pub idx_of_fastest_xz_vel: usize,
    pub average_xz_vel: f32,
    pub time_of_last_pad_read: i64,

    pub ground_impact_vel: f32,
    pub normal_impact_vel: f32,
    pub transv_on_last_impact: Vec3,

    /// The variable jump: how long the button may still raise it, the
    /// lowest and highest apex, and where it started.
    pub jump_window: f32,
    pub jump_height_min: f32,
    pub jump_height_max: f32,
    pub jump_start: Vec3,

    pub low_coverage: LowCoverage,
    /// How high above Jak's origin each body sphere sits: standing, or all
    /// at the bottom while ducking.
    pub sphere_heights: [f32; 3],
    pub standing_heights: [f32; 3],
    pub body_radius: f32,
    /// The ground speed an animation's own motion last asked for, and how
    /// much of it the move kept.
    pub align_xz_vel: Vec3,
    pub zx_vel_frac: f32,
    trans_log: Vec<(i64, Vec3)>,
    trans_log_idx: usize,
}

impl Control {
    /// Carried into a new frame (see [`crate::reframe`]): its points and
    /// directions turn and shift; gravity stays the frame's down.
    pub fn reframe(&mut self, frame: &crate::reframe::Frame) {
        for p in [
            &mut self.trans,
            &mut self.ground_touch_point,
            &mut self.wall_contact_pt,
            &mut self.gspot_pos,
            &mut self.turn_to_target,
            &mut self.to_target_pt_xz,
            &mut self.last_trans_any_surf,
            &mut self.last_trans_leaving_surf,
            &mut self.highest_jump_mark,
            &mut self.jump_start,
        ] {
            *p = frame.point(*p);
        }
        for v in [
            &mut self.transv,
            &mut self.local_normal,
            &mut self.surface_normal,
            &mut self.poly_normal,
            &mut self.pre_collide_local_normal,
            &mut self.ground_poly_normal,
            &mut self.ground_contact_normal,
            &mut self.wall_contact_normal,
            &mut self.gspot_normal,
            &mut self.pad_xz_dir,
            &mut self.last_pad_xz_dir,
            &mut self.target_transv,
            &mut self.turn_to_alt_heading,
            &mut self.btransv,
            &mut self.last_transv,
            &mut self.transv_on_last_impact,
            &mut self.align_xz_vel,
        ] {
            *v = frame.dir(*v);
        }
        for (v, _) in &mut self.transv_history {
            *v = frame.dir(*v);
        }
        for (_, p) in &mut self.trans_log {
            *p = frame.point(*p);
        }
        self.quat = frame.quat(self.quat);
        self.last_quat = frame.quat(self.last_quat);
        self.dir_targ = frame.quat(self.dir_targ);
        self.local_to_world = frame.basis(self.local_to_world);
    }

    pub fn new(trans: Vec3, yaw: f32, now: i64) -> Self {
        let quat = Quat::from_rotation_y(to_radians(yaw));
        Self {
            trans,
            transv: Vec3::ZERO,
            quat,
            last_quat: quat,
            dir_targ: quat,
            local_to_world: Basis::from_quat(quat),
            local_transv: Vec3::ZERO,
            status: 0,
            old_status: 0,
            prev_status: 0,
            reaction: 0,
            no_normal_reset: false,
            gravity_normal: Vec3::Y,
            gravity_length: STANDARD_GRAVITY,
            gravity_max: STANDARD_GRAVITY_MAX,
            local_normal: Vec3::Y,
            surface_normal: Vec3::Y,
            poly_normal: Vec3::Y,
            pre_collide_local_normal: Vec3::Y,
            bent_gravity_normal: Vec3::Y,
            ground_poly_normal: Vec3::Y,
            ground_contact_normal: Vec3::Y,
            ground_touch_point: trans,
            wall_contact_normal: Vec3::Y,
            wall_contact_pt: trans,
            wall_contact_pat: Pat::default(),
            gspot_pos: trans,
            gspot_normal: Vec3::Y,
            gspot_pat: Pat::default(),
            surface_angle: 1.0,
            poly_angle: 1.0,
            touch_angle: 0.0,
            coverage: 0.0,
            cur_pat: Pat::default(),
            poly_pat: Pat::default(),
            ground_pat: Pat::default(),
            surf: surface::STONE,
            prev_surf: surface::STONE,
            mod_surface: surface::WALK,
            current: Surface::mult(&surface::WALK, &surface::STONE),
            pad_magnitude: 0.0,
            last_pad_magnitude: 0.0,
            pad_xz_dir: Vec3::Z,
            last_pad_xz_dir: Vec3::Z,
            turn_to_magnitude: 0.0,
            turn_to_angle: 0.0,
            turn_to_target: Vec3::ZERO,
            to_target_pt_xz: z_axis(quat),
            target_transv: Vec3::ZERO,
            turn_to_alt_heading: z_axis(quat),
            ctrl_xz_vel: 0.0,
            local_slope_z: 0.0,
            local_slope_x: 0.0,
            surface_slope_z: 0.0,
            surface_slope_x: 0.0,
            gspot_slope_z: 0.0,
            gspot_slope_x: 0.0,
            ctrl_slope_z: 0.0,
            ctrl_slope_x: 0.0,
            ctrl_slope_heading: 0.0,
            btransv: Vec3::ZERO,
            blocked_factor: 0.0,
            blocked_in_air_factor: 0.0,
            velocity_after_thrust: 0.0,
            bend_amount: 0.0,
            bend_speed: 0.0,
            bend_target: 0.0,
            draw_offset_y: 0.0,
            turn_lockout_end_time: 0,
            turn_go_the_long_way: 0.0,
            force_turn_to_strength: 0.0,
            last_time_on_surface: now,
            list_time_on_ground: now,
            last_time_touching_actor: i64::MIN / 2,
            time_of_last_lc: i64::MIN / 2,
            time_of_last_lc_touch_edge: i64::MIN / 2,
            last_time_of_stuck: i64::MIN / 2,
            time_of_last_clear_wall_in_jump: i64::MIN / 2,
            time_of_last_surface_change: now,
            last_trans_any_surf: trans,
            last_trans_leaving_surf: trans,
            highest_jump_mark: trans,
            last_transv: Vec3::ZERO,
            transv_history: [(Vec3::ZERO, 0.0); 16],
            idx_of_fastest_xz_vel: 0,
            average_xz_vel: 0.0,
            time_of_last_pad_read: i64::MIN / 2,
            ground_impact_vel: 0.0,
            normal_impact_vel: 0.0,
            transv_on_last_impact: Vec3::ZERO,
            jump_window: -1.0,
            jump_height_min: 0.0,
            jump_height_max: 0.0,
            jump_start: trans,
            low_coverage: LowCoverage::default(),
            sphere_heights: SPHERE_HEIGHTS,
            standing_heights: SPHERE_HEIGHTS,
            body_radius: BODY_RADIUS,
            align_xz_vel: Vec3::ZERO,
            zx_vel_frac: 0.0,
            trans_log: vec![(i64::MIN / 2, trans); 128],
            trans_log_idx: 0,
        }
    }

    pub fn on_surface(&self) -> bool {
        self.status & status::ON_SURFACE != 0
    }

    pub fn world_to_local(&self, v: Vec3) -> Vec3 {
        self.local_to_world.to_local(v)
    }

    pub fn height_above_ground(&self) -> f32 {
        self.trans.y - self.gspot_pos.y
    }

    pub fn set_body_dimensions(&mut self, radius: f32, height: f32) {
        if !radius.is_finite() || !height.is_finite() || radius <= 0.0 || height < 2.0 * radius {
            return;
        }
        let ducked = self
            .sphere_heights
            .iter()
            .all(|h| (*h - self.body_radius).abs() < 1.0);
        self.body_radius = radius;
        self.standing_heights = [radius, height * 0.5, height - radius];
        self.sphere_heights = if ducked {
            [radius; 3]
        } else {
            self.standing_heights
        };
    }

    pub fn sphere_center(&self, i: usize) -> Vec3 {
        self.trans + Vec3::new(0.0, self.sphere_heights[i], 0.0)
    }

    pub fn y_angle(&self) -> f32 {
        quat_y_angle(self.quat)
    }

    pub(crate) fn log_trans(&mut self, now: i64) {
        self.trans_log[self.trans_log_idx] = (now, self.trans);
        self.trans_log_idx = (self.trans_log_idx + 1) & 127;
    }

    /// The average distance from the mean of the positions logged over the
    /// last `window` ticks: how much Jak has really moved, however hard he
    /// pushes.
    pub fn move_dist(&self, now: i64, window: i64) -> f32 {
        let mut sum = Vec3::ZERO;
        let mut count = 0usize;
        let mut remaining = 127i64;
        let mut idx = (self.trans_log_idx + 127) & 127;
        while now - self.trans_log[idx].0 < window && remaining > 0 {
            sum += self.trans_log[idx].1;
            count += 1;
            remaining -= 1;
            idx = (idx + remaining as usize) & 127;
        }
        if count == 0 {
            return f32::NAN;
        }
        let mean = sum / count as f32;
        let mut total = 0.0;
        for k in (128 - count)..128 {
            let i = (self.trans_log_idx + k) & 127;
            total += mean.distance(self.trans_log[i].1);
        }
        total / count as f32
    }
}

/// `src` turned by the rotation that takes the camera's up onto `normal`:
/// a stick direction laid onto the surface Jak stands on.
fn warp_into_surface(src: Vec3, normal: Vec3, cam: &Basis) -> Vec3 {
    let from = cam.u.normalize_or(Vec3::Y);
    let to = normal.normalize_or(Vec3::Y);
    let c = from.dot(to).clamp(-1.0, 1.0);
    let axis = from.cross(to);
    if axis.length_squared() < 1e-12 {
        return src;
    }
    Quat::from_axis_angle(axis.normalize(), c.acos()) * src
}

impl Jak {
    pub(crate) fn time_elapsed(&self, since: i64, duration: i64) -> bool {
        self.time - since >= duration
    }

    /// The per-frame bookkeeping before any movement.
    pub(crate) fn flag_setup(&mut self) {
        let now = self.time;
        let c = &mut self.control;
        c.last_transv = c.transv;
        c.last_quat = c.quat;
        if c.on_surface() {
            c.last_time_on_surface = now;
            c.last_trans_any_surf = c.trans;
        } else {
            if c.old_status & status::ON_SURFACE != 0 {
                c.last_trans_leaving_surf = c.last_trans_any_surf;
                c.highest_jump_mark = c.last_trans_any_surf;
            }
            c.highest_jump_mark.x = c.trans.x;
            c.highest_jump_mark.z = c.trans.z;
            if c.gravity_normal.dot(c.highest_jump_mark - c.trans) < 0.0 {
                c.highest_jump_mark.y = c.trans.y;
            }
        }
        if self.slow_falling && c.gravity_normal.dot(c.transv)<=0. {
            // LivingEntity resets fall distance on every tick, so expiry in
            // mid-air starts a new fall from here rather than the old peak.
            c.highest_jump_mark=c.trans;c.last_trans_any_surf=c.trans;
        }
        c.bend_speed = if c.on_surface() { 32.0 } else { 2.0 };
    }

    /// This frame's tuning and the local frame: forward from the heading,
    /// up from the surface normal.
    pub(crate) fn build_conversions(&mut self) {
        if self.control.prev_surf != self.control.surf {
            self.control.prev_surf = self.control.surf;
            self.control.time_of_last_surface_change = self.time;
        }
        let mut current = Surface::mult(&self.control.mod_surface, &self.control.surf);
        current.name = surface::Name::Current;
        self.apply_mult_hook(&mut current);
        if !self.state.is_flut() && !self.state.is_board() {
            current.target_speed *= self.movement_scale.max(0.0);
            current.transv_max *= self.movement_scale.max(0.0);
            // Jak 1's blue eco: faster running and jumping.
            if self.has_eco(crate::eco_power::EcoKind::Blue)
                && matches!(self.control.mod_surface.name, surface::Name::Run | surface::Name::Jump)
            {
                current.target_speed += crate::eco_power::BLUE_SPEED;
                current.transv_max += crate::eco_power::BLUE_SPEED;
            }
        }
        self.control.current = current;
        let c = &mut self.control;
        let mut forward = z_axis(c.quat);
        if c.mod_surface.flags & flag::XZ_LOCAL != 0 {
            forward = flatten(forward, c.gravity_normal).normalize_or_zero();
        }
        c.local_to_world = Basis::forward_up_nopitch(forward, c.local_normal);
        c.local_transv = c.local_to_world.to_local(c.transv);
    }

    pub(crate) fn do_rotations1(&mut self) {
        let c = &mut self.control;
        c.quat = rotate_toward_orientation(
            c.quat,
            c.dir_targ,
            0.0,
            c.current.tiltv,
            150,
            c.current.tiltvf as i32,
            0.0,
        );
    }

    /// The stick as a world direction, through the camera.
    pub(crate) fn read_pad(&mut self) -> Vec3 {
        let c = &mut self.control;
        if c.time_of_last_pad_read != self.time {
            c.last_pad_xz_dir = c.pad_xz_dir;
            c.last_pad_magnitude = c.pad_magnitude;
            c.time_of_last_pad_read = self.time;
        }
        let local = Vec3::new(sin(self.pad.stick0_dir), 0.0, cos(self.pad.stick0_dir));
        c.pad_xz_dir = local;
        c.pad_magnitude = self.pad.stick0_speed;
        self.camera.to_world(local)
    }

    /// The stick as a world direction laid into the ground's plane.
    pub(crate) fn pad_in_surface(&mut self) -> Vec3 {
        let v = self.read_pad();
        warp_into_surface(v, self.control.local_normal, &self.camera)
    }

    pub(crate) fn turn_to_vector(&mut self, v: Vec3, magnitude: f32) {
        let cam = self.camera;
        let wade = self.wade_scale();
        let c = &mut self.control;
        let warped = warp_into_surface(v, c.local_normal, &cam);
        c.turn_to_target = warped * magnitude;
        if magnitude > 0.0 {
            c.to_target_pt_xz = warp_into_surface(v, Vec3::Y, &cam);
        }
        let local = c.local_to_world.to_local(warped);
        c.turn_to_angle = atan(local.x, local.z);
        c.turn_to_magnitude = magnitude;
        c.target_transv = normalize(local, magnitude * c.current.target_speed * wade);
    }

    /// A stick flicked through the middle reads as released for a frame, so
    /// reversing it does not drag Jak through a half-speed turn.
    pub(crate) fn debounce_speed(&self) -> f32 {
        let c = &self.control;
        let m = c.pad_magnitude;
        if 0.3 < m
            && m < 0.7
            && 0.0 < c.last_pad_magnitude
            && c.pad_xz_dir.dot(c.last_pad_xz_dir) < 0.2
        {
            0.0
        } else {
            m
        }
    }

    /// Walking thrust: the local velocity seeks the stick's target velocity
    /// at the table's seek rate, or brakes at its friction.
    pub(crate) fn add_thrust(&mut self) {
        let now = self.time;
        let c = &mut self.control;
        let original_target = c.target_transv;
        let mut v = c.local_transv;
        // Turning the velocity toward straight ahead.
        if c.current.vel_turn < 0.0 {
            v = Vec3::new(0.0, v.y, xz_length(v));
        } else if c.current.vel_turn != 0.0 {
            let angle = atan(v.x, v.z);
            v = rotate_y(
                v,
                (0.03 * -angle).min(c.current.vel_turn * SECONDS_PER_FRAME),
            );
        }
        let mut target = c.target_transv;
        self.add_slide_factor(&mut target);
        let c = &mut self.control;
        c.target_transv = target;
        let target_dir = Vec3::new(target.x, 0.0, target.z);
        let target_dir = xz_normalize(target_dir, 1.0);
        let heading = target_dir.z;
        let mut rate = if xz_length(v) >= xz_length(target) {
            c.current.fric * ps2_div(xz_length(v), c.current.nonlin_fric_dist).max(1.0)
        } else if heading >= 0.0 {
            heading * c.current.seek0 + (1.0 - heading) * c.current.seek90
        } else {
            heading.abs() * c.current.seek180 + (1.0 + heading) * c.current.seek90
        };
        let mut b = c.local_to_world.to_local(c.btransv);
        let clamp_toward = |x: f32, limit: f32| {
            if limit < 0.0 {
                x.min(0.0).max(limit)
            } else {
                x.min(limit).max(0.0)
            }
        };
        b.x = clamp_toward(b.x, original_target.x);
        b.y = clamp_toward(b.y, original_target.y);
        b.z = clamp_toward(b.z, original_target.z);
        if 0.2 < c.blocked_factor {
            b = vector_seek(b, original_target, 122880.0 * SECONDS_PER_FRAME);
        }
        c.btransv = c.local_to_world.to_world(b);
        let gap = Vec2Xz::distance(b, original_target);
        if c.status & status::TOUCH_SURFACE == 0 && xz_length(v) < xz_length(b) {
            let mut extra = lerp_scale(163840.0, 0.0, gap, 0.0, 20480.0);
            if gap < 20480.0 && original_target.z < 0.0 {
                extra *= 2.0;
            }
            rate += extra;
        }
        if c.status & status::TOUCH_WALL == 0
            && c.old_status & status::TOUCH_WALL != 0
            && c.mod_surface.flags & flag::AIR != 0
            && 0.0 < v.y
            && 0.0 < c.gravity_normal.dot(c.sphere_center(1) - c.wall_contact_pt)
        {
            c.time_of_last_clear_wall_in_jump = now;
        }
        if now - c.time_of_last_clear_wall_in_jump < seconds(0.2) {
            rate += 204800.0;
        }
        if c.status & status::TOUCH_WALL == 0
            && c.old_status & status::TOUCH_WALL != 0
            && c.mod_surface.flags & flag::AIR != 0
            && 0.0 < v.y
            && v.z < 0.0
        {
            v.z = 0.0;
        }
        let mut step = target - v;
        step.y = 0.0;
        let limit = rate * SECONDS_PER_FRAME;
        if limit < xz_length(step) {
            step = xz_normalize(step, limit);
        }
        v += step;
        c.local_transv = v;
        c.velocity_after_thrust = v.length();
    }

    /// The pull a slope puts on the walking target velocity.
    fn add_slide_factor(&self, target: &mut Vec3) {
        let c = &self.control;
        let down_slope = flatten(-c.gravity_normal, c.local_normal);
        let local = c.local_to_world.to_local(down_slope);
        let dir = local.normalize_or_zero();
        *target += local * c.current.slide_factor;
        let tangent = c.low_coverage.tangent;
        if !(1.0 - c.current.slope_up_traction == 0.0 || local.length() < 0.1) {
            let t = c.local_to_world.to_local(tangent);
            let mut along = t.dot(*target);
            let rest = *target - t * along;
            if along < 0.0 {
                along *= c.current.slope_up_traction;
            }
            along += c.current.slope_down_factor;
            *target = t * along + rest;
        }
        let mut flat = xz_normalize(*target, 1.0);
        flat.y = 0.0;
        let d = dir.dot(flat);
        let speed = xz_length(*target);
        *target = if d >= 0.0 {
            xz_normalize(*target, (speed + d * c.current.slope_down_factor).max(0.0))
        } else {
            xz_normalize(*target, (speed - (-d) * c.current.slope_up_factor).max(0.0))
        };
    }

    /// Gravity in the local frame: on a surface only the part `slip` lets
    /// through pulls along it, and the fall speed is capped.
    pub(crate) fn add_gravity(&mut self) {
        let c = &mut self.control;
        let up=c.local_to_world.to_local(c.gravity_normal);
        let before=up.dot(c.local_transv);
        let slow=self.slow_falling && !c.on_surface() && before<=0.;
        let gravity=if slow {c.gravity_length.min(5.76*METER)}else{c.gravity_length};
        let down = -(c.gravity_normal * gravity);
        let slip = if c.on_surface() {
            c.current.slip_factor
        } else {
            0.0
        };
        let acc = down - reflect_flat(down, c.local_normal) * slip;
        c.local_transv += c.local_to_world.to_local(acc) * SECONDS_PER_FRAME;
        let mut vertical = up.dot(c.local_transv);
        let mut rest = c.local_transv - up * vertical;
        if slow {
            let drag=0.98f32.powf(20.*SECONDS_PER_FRAME);
            let terminal=gravity/20.*0.98/0.02;
            vertical=before*drag-terminal*(1.-drag);
        }
        if rest.length() < 0.00004096 {
            rest = Vec3::ZERO;
        }
        if vertical < -c.gravity_max {
            vertical = -c.gravity_max;
        }
        c.local_transv = up * vertical + rest;
    }

    pub(crate) fn do_rotations2(&mut self) {
        let now = self.time;
        let walking = self.on_foot_walking();
        let state_young = !self.time_elapsed(self.state_time, seconds(0.5));
        let c = &mut self.control;
        let from = z_axis(c.dir_targ);
        let use_pad = (c.status | c.old_status) & (status::ON_SURFACE | status::TOUCH_SURFACE) == 0
            || now - c.last_time_touching_actor < seconds(0.5)
            || !walking
            || state_young
            || now - c.time_of_last_lc < seconds(0.5)
            || c.current.flags & flag::TURN_TO_PAD != 0
            || c.force_turn_to_strength != 0.0;
        let to = if c.current.flags & flag::TURN_TO_ALT != 0 {
            c.turn_to_alt_heading
        } else if use_pad && c.current.flags & flag::TURN_TO_VEL == 0 {
            c.to_target_pt_xz
        } else {
            c.transv
        };
        let q_from = forward_up_nopitch_quat(from, c.bent_gravity_normal);
        let q_to = forward_up_nopitch_quat(to, c.bent_gravity_normal);
        let angle = acos(from.dot(to)).max(1e-11);
        let step = c.current.turnvv * SECONDS_PER_FRAME;
        let t = if (c.turn_to_magnitude <= 0.0 || now < c.turn_lockout_end_time)
            && c.current.flags & flag::TURN_WHEN_CENTERED == 0
        {
            0.0
        } else if angle < step {
            1.0
        } else {
            step / angle
        };
        c.dir_targ = q_from.slerp(q_to, t);
        c.quat = rotate_toward_orientation(
            c.quat,
            c.dir_targ,
            c.current.turnv,
            0.0,
            c.current.turnvf as i32,
            150,
            c.turn_go_the_long_way,
        );
        if c.turn_go_the_long_way != 0.0
            && deg_diff(quat_y_angle(c.quat), quat_y_angle(c.dir_targ)).abs() < 182.04445
        {
            c.turn_go_the_long_way = 0.0;
        }
        self.compute_slopes();
    }

    pub(crate) fn compute_slopes(&mut self) {
        let c = &mut self.control;
        let up = y_axis(c.quat);
        let down_slope = flatten(c.gravity_normal, c.local_normal);
        let frame = Basis::forward_up_nopitch(c.local_to_world.f, c.gravity_normal);
        let z = frame.f;
        c.surface_slope_z = -c.surface_normal.dot(z);
        c.local_slope_z = -c.local_normal.dot(z);
        c.gspot_slope_z = -c.gspot_normal.dot(z);
        c.ctrl_slope_z = -up.dot(z);
        let x = frame.r;
        c.surface_slope_x = -c.surface_normal.dot(x);
        c.local_slope_x = -c.local_normal.dot(x);
        c.gspot_slope_x = -c.gspot_normal.dot(x);
        c.ctrl_slope_x = -up.dot(x);
        c.ctrl_slope_heading = c.local_to_world.r.dot(down_slope);
    }

    pub(crate) fn reverse_conversions(&mut self) {
        let c = &mut self.control;
        c.ctrl_xz_vel = xz_length(c.local_transv);
        c.transv = c.local_to_world.to_world(c.local_transv);
        c.old_status = c.status;
    }

    pub(crate) fn pre_collide_setup(&mut self) {
        let c = &mut self.control;
        c.pre_collide_local_normal = c.local_normal;
    }

    /// Sweeps Jak's spheres along this frame's velocity through the world,
    /// sliding off what they hit.
    pub(crate) fn integrate_and_collide(&mut self, world: &mut dyn CollideWorld) {
        if 1638400.0 < self.control.transv.length() {
            self.control.transv = Vec3::ZERO;
        }
        let motion = self.control.transv * SECONDS_PER_FRAME;
        let center = self.control.trans + Vec3::new(0.0, ROOT_OFFSET_Y, 0.0);
        let reach = Vec3::splat(ROOT_RADIUS + motion.length() + METER);
        self.cache.fill_box(world, center - reach, center + reach);

        let before = self.control.transv;
        let mut v = before;
        {
            let c = &mut self.control;
            c.prev_status = c.status;
            c.status &= !status::PER_STEP;
            if !c.no_normal_reset {
                c.local_normal = c.gravity_normal;
                c.surface_normal = c.gravity_normal;
                c.poly_normal = c.gravity_normal;
                c.coverage = 0.0;
                c.touch_angle = 0.0;
            }
        }
        let mut remaining = 1.0f32;
        let mut iterations = 0;
        while 0.05 < remaining && iterations < MAX_ITERATIONS && v != Vec3::ZERO {
            let used = self.step_collision(&mut v, remaining);
            remaining -= used * remaining;
            iterations += 1;
        }
        let c = &mut self.control;
        let up = c.gravity_normal;
        let a = flatten(before, up).normalize_or_zero();
        let b = flatten(v, up).normalize_or_zero();
        let d = a.dot(b);
        let turned = if c.status & status::TOUCH_WALL != 0 {
            d < 0.9999
        } else {
            d < 0.95
        };
        if c.target_transv.length() != 0.0 && turned {
            c.blocked_factor = seek(c.blocked_factor, 1.0, 4.0 * SECONDS_PER_FRAME);
            let air = if c.mod_surface.mode == surface::Mode::Air {
                1.0
            } else {
                0.0
            };
            c.blocked_in_air_factor = seek(c.blocked_in_air_factor, air, 4.0 * SECONDS_PER_FRAME);
            c.status |= status::BLOCKED;
        } else {
            c.blocked_factor = seek(c.blocked_factor, 0.0, 2.0 * SECONDS_PER_FRAME);
            c.blocked_in_air_factor = seek(c.blocked_in_air_factor, 0.0, 2.0 * SECONDS_PER_FRAME);
        }
        c.transv = v;
        if c.on_surface()
            && c.status & (status::TOUCH_WALL | status::BLOCKED) == 0
            && c.btransv.length() < before.length()
        {
            c.btransv = before;
        }
        let along = c.align_xz_vel.length();
        c.zx_vel_frac = if along == 0.0 {
            0.0
        } else {
            (c.transv.dot(c.align_xz_vel / along) / along).max(0.0)
        };
    }

    /// One sweep of the remaining fraction of the move: the earliest contact
    /// over the three spheres, and the reaction to it. Returns how much of
    /// the move it used.
    fn step_collision(&mut self, v: &mut Vec3, fraction: f32) -> f32 {
        let motion = *v * (fraction * SECONDS_PER_FRAME);
        let mut best: Option<(crate::collide::TriHit, usize)> = None;
        for i in 0..3 {
            let center = self.control.sphere_center(i);
            let limit = best.as_ref().map_or(-1.0, |(h, _)| h.u);
            if let Some(hit) = self.cache.resolve_moving_sphere(
                center,
                self.control.body_radius,
                motion,
                limit,
                true,
            ) {
                best = Some((hit, i));
            }
        }
        match best {
            Some((hit, sphere)) => {
                self.collision_reaction(&hit, sphere, motion, v);
                hit.u
            }
            None => {
                let c = &mut self.control;
                c.reaction = if c.mod_surface.mode == surface::Mode::Air {
                    reaction::AIR
                } else {
                    0
                };
                c.prev_status = 0;
                c.trans += motion;
                1.0
            }
        }
    }

    /// What a contact does: classify it as ground or wall, update what Jak
    /// stands on, and bend the velocity along the surface.
    fn collision_reaction(
        &mut self,
        hit: &crate::collide::TriHit,
        sphere: usize,
        motion: Vec3,
        v: &mut Vec3,
    ) {
        let now = self.time;
        let on_board = self.on_board();
        let incoming = *v;
        let mut rvec = incoming;
        let mut st = 0u32;
        let mut rf = 0u32;
        self.control.trans += motion * hit.u;
        self.react_to_pat(hit.pat);
        let c = &mut self.control;
        if c.poly_pat.mode == PatMode::Wall {
            rf |= reaction::WALL_PAT;
        }
        if c.mod_surface.flags & flag::AIR != 0 {
            rf |= reaction::AIR;
        }
        let mut n = (c.sphere_center(sphere) - hit.intersect).normalize_or_zero();
        c.coverage = n.dot(hit.normal);
        if c.coverage < 0.0 {
            c.coverage = 0.0;
            n = flatten(n, hit.normal).normalize_or_zero();
        }
        if c.coverage < 0.9999 {
            rf |= reaction::LOW_COVERAGE | reaction::LOW_COVERAGE_ALSO;
        }
        if hit.u == 0.0 {
            c.trans += normalize(n, 3.0);
        }
        c.poly_normal = hit.normal;
        c.surface_normal = n;
        c.surface_angle = n.dot(c.gravity_normal);
        c.poly_angle = c.poly_normal.dot(c.gravity_normal);
        c.touch_angle = c.touch_angle.max(n.dot((-rvec).normalize_or_zero()));
        if c.poly_angle < -0.2 {
            st |= status::TOUCH_CEILING;
        }
        if c.poly_angle < 0.0 {
            st |= status::TOUCH_CEILING_STICKY;
        }
        if sphere != 0 {
            rf |= reaction::UPPER_SPHERE;
        }
        let mut wall = c.surface_angle.abs() < c.cur_pat.mode.wall_angle();
        if rf & reaction::UPPER_SPHERE != 0
            && rf & reaction::AIR != 0
            && c.poly_angle >= 0.0
            && !wall
        {
            wall = true;
        }
        if wall {
            rf |= reaction::WALL;
        }
        if rf & reaction::LOW_COVERAGE != 0 {
            self.low_coverage_reaction(hit, n, &mut rf, &mut st, &mut wall);
        }
        // A ceiling blocks the upper body, but cannot support its weight.
        // abs(surface_angle) above also admits a flat downward normal;
        // the edge probe can similarly clear `wall` for a ground material.
        // Keep the real swept contact while excluding both from support. The lower
        // sphere still uses the source ledge/step probe; an upper-body wall
        // contact must never become a floor underneath the feet.
        if self.control.surface_angle < 0.0 || (sphere != 0 && hit.pat.mode == PatMode::Wall && self.control.poly_angle.abs() < 0.01) {
            wall = true;
            rf |= reaction::WALL;
        }
        let c = &mut self.control;
        if c.prev_status & status::ON_SURFACE == 0 {
            c.transv_on_last_impact = c.transv;
            c.ground_impact_vel = -c.transv.dot(c.gravity_normal);
            c.normal_impact_vel = -c.transv.dot(n);
            st |= status::IMPACT_SURFACE;
            if !wall {
                let keep = 1.0 - c.current.impact_fric;
                if keep < 1.0 {
                    let up = c.gravity_normal;
                    let mut vertical = up.dot(rvec);
                    if vertical < 0.0 {
                        vertical *= keep;
                    }
                    rvec = with_vertical(rvec, up, vertical);
                }
            }
        }
        st |= status::TOUCH_SURFACE | status::TOUCH_BACKGROUND;
        let out;
        if wall {
            rf |= reaction::WALL_RESPONSE;
            st |= status::TOUCH_WALL;
            c.cur_pat.mode = PatMode::Wall;
            c.wall_contact_pt = hit.intersect;
            c.wall_contact_normal = n;
            c.wall_contact_pat = hit.pat;
            let mut o = reflect_flat(rvec, n);
            let to_hit = hit.intersect - c.ground_touch_point;
            if ((c.body_radius >= c.ground_poly_normal.dot(to_hit)
                && now - c.list_time_on_ground < seconds(0.3))
                || rf & reaction::UPPER_SPHERE != 0)
                && c.gravity_normal.dot(to_hit) >= 0.0
                && 0.0 < c.ground_poly_normal.dot(o)
                && rf & reaction::AIR == 0
            {
                rf |= reaction::CREASE;
                rf &= !reaction::EDGE_WALL;
                let crease = c
                    .poly_normal
                    .cross(c.ground_poly_normal)
                    .normalize_or_zero();
                o = crease * rvec.dot(crease) + c.poly_normal;
            }
            out = o;
        } else {
            st |= status::ON_SURFACE;
            // Keep the authored curved-ramp material. Erasing it here made
            // board contacts flatten their upward movement into the ramp.
            if c.cur_pat.mode != PatMode::Halfpipe {c.cur_pat.mode=PatMode::Ground;}
            if sphere == 0 {
                c.local_normal = n;
            }
            let mut o = if on_board
                && c.cur_pat.mode != PatMode::Halfpipe
                && c.ground_pat.mode != PatMode::Halfpipe
            {
                reflect_flat_gravity(rvec, n, c.gravity_normal) + n
            } else {
                reflect_flat(rvec, n) + n
            };
            let preserve = c.current.slope_change_preserve;
            if 0.0 < preserve {
                let along = flatten(rvec, c.pre_collide_local_normal).length();
                let out_len = o.length();
                if 409.6 < (out_len - along).abs() && out_len < along {
                    o = normalize(o, lerp_scale(out_len, along, preserve, 0.0, 1.0));
                }
            }
            if rf
                & (reaction::WALL_PAT
                    | reaction::WALL
                    | reaction::WALL_RESPONSE
                    | reaction::LOW_COVERAGE
                    | reaction::HAZARD)
                == 0
            {
                st |= status::ON_GROUND;
                c.ground_poly_normal = c.poly_normal;
                c.ground_contact_normal = n;
                c.list_time_on_ground = now;
                c.ground_pat = c.poly_pat;
                c.ground_touch_point = hit.intersect;
                rf |= reaction::GROUND;
            }
            out = o;
        }
        c.status |= st;
        c.prev_status = st;
        c.reaction = rf;
        *v = out;
    }

    fn react_to_pat(&mut self, pat: Pat) {
        let c = &mut self.control;
        c.cur_pat = pat;
        c.poly_pat = pat;
        c.surf = surface::of_material(pat.material);
    }

    /// A contact on an edge or corner: probe past it to tell a ledge Jak can
    /// stand on from one he should slide off.
    fn low_coverage_reaction(
        &mut self,
        hit: &crate::collide::TriHit,
        n: Vec3,
        rf: &mut u32,
        st: &mut u32,
        wall: &mut bool,
    ) {
        let now = self.time;
        let c = &mut self.control;
        let mut overhang = c.poly_normal.cross(n).normalize_or_zero();
        if c.gravity_normal.dot(overhang).abs() >= 0.866 {
            *rf |= reaction::OVERHANG;
        }
        let mut tangent = n.cross(overhang);
        if 0.0 < c.gravity_normal.dot(tangent) {
            tangent = -tangent;
        }
        tangent = tangent.normalize_or_zero();
        overhang = overhang.normalize_or_zero();
        c.low_coverage.overhang_normal = overhang;
        c.low_coverage.tangent = tangent;
        let mut out = flatten(tangent, c.gravity_normal).normalize_or_zero();
        if out.dot(n) < 0.0 {
            out = -out;
        }
        c.low_coverage.tangent_xz = out;
        c.low_coverage.slope_to_next1 = 4095996000.0;
        c.low_coverage.dist_to_next2 = 4095996000.0;
        let up = c.gravity_normal;
        let start1 = hit.intersect + out * 2867.2;
        let move1 = -up * 20480.0 + out * 4096.0;
        if let Some(h) = self.cache.probe_line_sphere(start1, move1, 409.6) {
            let c = &mut self.control;
            c.low_coverage.slope_to_next1 = up.dot(hit.intersect - h.intersect);
            c.low_coverage.pat_next1 = h.pat;
        }
        let start2 = hit.intersect + up * 819.2 - out * 819.2;
        let move2 = -up * 20480.0 - out * 4096.0;
        if let Some(h) = self.cache.probe_line_sphere(start2, move2, 409.6) {
            let c = &mut self.control;
            c.low_coverage.dist_to_next2 = start2.distance(h.intersect);
            c.low_coverage.pat_next2 = h.pat;
        }
        let c = &mut self.control;
        let lc = c.low_coverage;
        let ground_like = |m: PatMode| m == PatMode::Ground || m == PatMode::Halfpipe;
        if *rf & reaction::OVERHANG == 0
            && (1.25 * c.body_radius < lc.slope_to_next1 || lc.pat_next1.mode == PatMode::Wall)
            && lc.dist_to_next2 < 2.0 * c.body_radius
            && ground_like(lc.pat_next2.mode)
        {
            *rf |= reaction::LEDGE;
            c.time_of_last_lc_touch_edge = now;
            *st |= status::TOUCH_EDGE;
            c.time_of_last_lc = now;
            let toward = tangent.dot(c.turn_to_target);
            let cover = if *rf & reaction::WALL != 0 {
                cos(16384.0 - acos(c.coverage))
            } else {
                c.coverage
            };
            let height = c.gravity_normal.dot(c.trans - hit.intersect);
            if *rf & reaction::AIR == 0 || 0.5 < cover {
                *wall = false;
            }
            if ((cover < 0.95 && toward >= 0.0)
                || (*rf & reaction::AIR != 0 && cover < 0.3)
                || height < c.body_radius / -4.0)
                && tangent.dot(n) >= -0.000001
            {
                c.surf = surface::EDGE;
                *rf |= reaction::EDGE_SURFACE;
                *wall = false;
                if *rf & reaction::AIR != 0 {
                    *wall = true;
                    *rf |= reaction::EDGE_AIR_WALL;
                }
            }
        }
        if c.surface_angle < 0.0 {
            *wall = true;
        }
        if *wall {
            *rf |= reaction::EDGE_WALL;
            *st |= status::TOUCH_EDGE;
            c.time_of_last_lc = now;
            let standable = ground_like(c.poly_pat.mode)
                || (*rf & reaction::LOW_COVERAGE != 0
                    && 1.25 * c.body_radius >= lc.slope_to_next1
                    && ground_like(lc.pat_next1.mode)
                    && 0.3 < c.surface_angle.abs());
            if standable && *rf & reaction::LEDGE == 0 {
                *wall = false;
            }
        }
    }

    pub(crate) fn bend_gravity(&mut self) {
        let c = &mut self.control;
        if c.no_normal_reset && !c.on_surface() {
            return;
        }
        let target = if c.status & status::TOUCH_WALL != 0 && !c.on_surface() {
            0.0
        } else {
            c.bend_target
        };
        c.bend_amount = seek(c.bend_amount, target, c.bend_speed * SECONDS_PER_FRAME);
        c.gravity_normal = Vec3::Y;
        let toward = vector_deg_slerp(Vec3::Y, c.gspot_normal, c.bend_amount);
        let turn = smooth_rotation(
            c.bent_gravity_normal.normalize_or(Vec3::Y),
            toward.normalize_or(Vec3::Y),
            c.current.tiltvv,
            c.current.tiltvvf as i32,
            Vec3::X,
        );
        c.bent_gravity_normal = (turn * c.bent_gravity_normal).normalize_or(Vec3::Y);
    }

    pub(crate) fn post_flag_setup(&mut self) {
        let now = self.time;
        let c = &mut self.control;
        if c.status & (status::TOUCH_WALL | status::TOUCH_ACTOR) != 0 {
            c.last_time_touching_actor = now;
        }
        c.log_trans(now);
    }

    /// Where the ground is below Jak: under his feet when he stands, else a
    /// probe straight down.
    pub(crate) fn target_gspot(&mut self, world: &mut dyn CollideWorld) {
        let c = &mut self.control;
        if c.on_surface() {
            c.gspot_pos = c.trans;
            c.gspot_normal = c.ground_poly_normal;
            c.gspot_pat = c.ground_pat;
            return;
        }
        let mut above = 8192.0;
        if c.transv.y < 0.0 {
            above -= (c.transv.y * SECONDS_PER_FRAME).max(-40960.0);
        }
        let below = 81920.0;
        let radius = 1024.0;
        let start = c.trans + Vec3::new(0.0, above, 0.0);
        let motion = Vec3::new(0.0, -(above + below), 0.0);
        c.gspot_pos = c.trans;
        match crate::collide::fill_and_probe_line_sphere(
            world,
            &mut self.probe_cache,
            start,
            motion,
            radius,
        ) {
            Some(h) => {
                let at = start + motion * h.u;
                let c = &mut self.control;
                c.gspot_pos.y = at.y - radius;
                c.gspot_normal = (at - h.intersect).normalize_or(Vec3::Y);
                c.gspot_pat = h.pat;
            }
            None => {
                let c = &mut self.control;
                c.gspot_pos.y = -40959590.0;
                c.gspot_normal = Vec3::Y;
            }
        }
    }

    /// Frames until Jak's current fall reaches the ground below, as ticks.
    pub(crate) fn time_to_ground(&self) -> i64 {
        let c = &self.control;
        let mut height = c.height_above_ground();
        let mut vy = c.gravity_normal.dot(c.transv);
        let mut ticks = 0.0f32;
        let mut guard = 0;
        while 0.0 < height && guard < 100_000 {
            ticks += 5.0;
            height += vy * SECONDS_PER_FRAME;
            vy -= c.gravity_length * SECONDS_PER_FRAME;
            guard += 1;
        }
        ticks as i64
    }

    pub(crate) fn hit_ground_or_stuck(&mut self) -> bool {
        if self.control.on_surface() {
            return true;
        }
        if self.control.move_dist(self.time, STUCK_TIME) < STUCK_DISTANCE {
            self.control.last_time_of_stuck = self.time;
            return true;
        }
        false
    }

    pub(crate) fn can_jump(&self, board: bool) -> bool {
        let c = &self.control;
        let grounded = c.on_surface()
            || !self.time_elapsed(c.last_time_on_surface, GROUND_TIMEOUT)
            || (board
                && (!self.time_elapsed(self.board.last_jump_time, seconds(0.05))
                    || (!self.time_elapsed(c.last_time_of_stuck, seconds(0.5))
                        && c.height_above_ground() < 2048.0)));
        grounded && c.current.flags & flag::NO_JUMP == 0
    }

    /// Velocity off the ground for a jump whose apex lands between
    /// `min` and `max` above the take-off, depending on how long the button
    /// is held. `bonus` scales extra height carried in from an upward motion.
    pub(crate) fn init_var_jump(
        &mut self,
        min: f32,
        max: f32,
        set_velocity: bool,
        collide_offset: bool,
        bonus: f32,
    ) {
        self.delete_back_vel();
        let c = &mut self.control;
        c.status &= !status::TOUCH_CEILING_STICKY;
        let rise = c.gravity_length / 600.0 + c.gravity_normal.dot(c.transv).max(0.0);
        let extra = bonus * (rise / 2.0 * rise / c.gravity_length).max(0.0);
        // A take-off speed k times as fast rises k² as high (host effects;
        // not on the Flut Flut or the board, as `movement_scale`).
        let boost = if self.state.is_flut() || self.state.is_board() { 1.0 } else { self.jump_scale.max(0.0).powi(2) };
        let c = &mut self.control;
        let low = (min + extra) * boost;
        let high = (max + extra) * boost;
        c.jump_window = 0.0;
        if collide_offset {
            c.jump_height_min = low - (-409.6 + JUMP_COLLIDE_OFFSET);
            c.jump_height_max = high - JUMP_COLLIDE_OFFSET;
        } else {
            c.jump_height_min = low;
            c.jump_height_max = high;
        }
        if set_velocity {
            let up_speed = (2.0 * c.gravity_length * low).sqrt() - 0.008333334 * -c.gravity_length;
            c.transv = with_vertical(c.transv, c.gravity_normal, up_speed);
            self.trace.impulse = up_speed;
            self.trace.impulse_at = self.time;
        }
        self.control.jump_start = self.control.trans;
    }

    /// While the jump button stays down through the first tenth of a second,
    /// keeps raising the apex toward the jump's maximum.
    pub(crate) fn mod_var_jump(&mut self, holding: bool) {
        let window = (self.time - self.state_time) as f32 * 0.033333335;
        let c = &mut self.control;
        if 1.0 < window || c.jump_window < 0.0 || !holding {
            c.jump_window = -1.0;
            return;
        }
        c.jump_window = window;
        let gained = c.gravity_normal.dot(c.trans - c.jump_start);
        let apex = lerp_scale(c.jump_height_min, c.jump_height_max, window, 0.0, 1.0);
        let up_speed = (2.0 * c.gravity_length * (apex - gained)).max(0.0).sqrt()
            - 0.008333334 * -c.gravity_length;
        c.transv = with_vertical(c.transv, c.gravity_normal, up_speed);
    }

    /// Drops any velocity pointing backwards from where Jak is turning to.
    pub(crate) fn delete_back_vel(&mut self) {
        let c = &mut self.control;
        let f = z_axis(c.dir_targ);
        let along = f.dot(c.transv).max(0.0);
        c.transv = flatten(c.transv, f) + f * along;
    }
}

pub const JUMP_COLLIDE_OFFSET: f32 = meters(0.7);

struct Vec2Xz;

impl Vec2Xz {
    fn distance(a: Vec3, b: Vec3) -> f32 {
        let dx = a.x - b.x;
        let dz = a.z - b.z;
        (dx * dx + dz * dz).sqrt()
    }
}

#[cfg(test)] mod host_slow_falling_regressions {
    use super::*;
    #[test] fn slow_falling_uses_native_potion_terminal_and_resets_both_native_fall_markers() {
        let mut jak=Jak::new(Vec3::Y*100.*METER,0.);jak.slow_falling=true;
        jak.control.status=0;jak.control.local_transv=Vec3::ZERO;
        for _ in 0..1800 {
            jak.add_gravity();jak.control.transv=jak.control.local_to_world.to_world(jak.control.local_transv);
        }
        assert!((jak.control.transv.y/METER+14.112).abs()<0.002,"{}",jak.control.transv.y/METER);
        let old=jak.control.trans;jak.control.trans-=Vec3::Y*50.*METER;jak.flag_setup();
        assert_ne!(jak.control.trans,old);assert_eq!(jak.control.highest_jump_mark,jak.control.trans);
        assert_eq!(jak.control.last_trans_any_surf,jak.control.trans);
        jak.slow_falling=false;let before=jak.control.local_transv.y;jak.add_gravity();
        assert!((jak.control.local_transv.y-before+STANDARD_GRAVITY*SECONDS_PER_FRAME).abs()<0.05);
    }
    #[test] fn slow_falling_never_changes_the_rising_half_of_a_jump() {
        let mut normal=Jak::new(Vec3::ZERO,0.);let mut slow=Jak::new(Vec3::ZERO,0.);slow.slow_falling=true;
        for jak in [&mut normal,&mut slow] {jak.control.status=0;jak.control.local_transv=Vec3::Y*8.*METER;jak.control.transv=Vec3::Y*8.*METER;}
        for _ in 0..7 {normal.add_gravity();slow.add_gravity();assert_eq!(normal.control.local_transv,slow.control.local_transv);}
    }
}
