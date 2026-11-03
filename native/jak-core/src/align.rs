//! The motion an animation carries: each frame, how far its align joint
//! moved and turned since the last, turned into Jak's velocity or heading
//! by the states that move with their animation (a roll, a flip, the
//! uppercut's rise, a spin's turn).
use glam::{Quat, Vec3};

use crate::Jak;
use crate::anim::{Anim, NumFunc};
use crate::math::*;

pub mod opts {
    pub const X_VEL: u32 = 1 << 0;
    pub const Y_VEL: u32 = 1 << 1;
    pub const XZ_VEL: u32 = 1 << 2;
    pub const KEEP_OTHER: u32 = 1 << 3;
    pub const QUAT: u32 = 1 << 4;
    pub const NO_GRAVITY: u32 = 1 << 11;
    pub const IGNORE_Y_IF_ZERO: u32 = 1 << 12;
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Align {
    pub anim: Option<Anim>,
    pub frame: f32,
    pub trans: [Vec3; 2],
    pub quat: [Quat; 2],
    pub delta_trans: Vec3,
    pub delta_quat: Quat,
    /// The animation changed or restarted since the last look: no motion
    /// this frame.
    pub disabled: bool,
}

impl Default for Align {
    fn default() -> Self {
        Self {
            anim: None,
            frame: 0.0,
            trans: [Vec3::ZERO; 2],
            quat: [Quat::IDENTITY; 2],
            delta_trans: Vec3::ZERO,
            delta_quat: Quat::IDENTITY,
            disabled: true,
        }
    }
}

impl Jak {
    /// How far the align joint moved since the last call.
    pub(crate) fn compute_delta_align(&mut self) {
        let chan = self.chan;
        let a = &mut self.align;
        a.disabled = if a.anim != Some(chan.anim) {
            true
        } else if let NumFunc::Loop { rate } = chan.func {
            rate * (chan.frame - a.frame) < 0.0
        } else {
            chan.frame == 0.0
        };
        a.anim = Some(chan.anim);
        a.frame = chan.frame;
        let (trans, quat) = self
            .anims
            .align(chan.anim)
            .map_or((Vec3::ZERO, Quat::IDENTITY), |t| t.sample(chan.frame));
        let a = &mut self.align;
        a.trans[1] = a.trans[0];
        a.quat[1] = a.quat[0];
        a.trans[0] = trans;
        a.quat[0] = quat;
        // Callers such as the punch read delta_trans directly, without
        // passing through align(). Never expose another clip's root jump.
        a.delta_trans = if a.disabled {Vec3::ZERO} else {a.trans[0] - a.trans[1]};
        a.delta_quat = if a.disabled {Quat::IDENTITY} else {(a.quat[1].inverse() * a.quat[0]).normalize()};
    }

    /// Takes the last align motion into Jak's velocity, in his own frame,
    /// scaled per axis, and his heading.
    pub(crate) fn align(&mut self, flags: u32, sx: f32, sy: f32, sz: f32) {
        if self.align.disabled {
            return;
        }
        let delta = self.align.delta_trans;
        let quat = self.align.delta_quat;
        let slope = self.control.local_slope_z;
        let alignv = self.control.current.alignv;
        let sz = sz
            * if 0.0 < slope {
                (1.0 - slope) * alignv
            } else {
                alignv
            };
        if flags & (opts::X_VEL | opts::Y_VEL | opts::XZ_VEL) != 0 {
            let c = &mut self.control;
            let gravity = if flags & opts::NO_GRAVITY != 0 {
                Vec3::ZERO
            } else {
                c.local_to_world
                    .to_local(-c.gravity_normal * c.gravity_length)
            };
            let mut v = c.local_to_world.to_local(c.transv);
            if flags & opts::X_VEL != 0 {
                v.x = delta.x * sx * 60.0 + gravity.x * SECONDS_PER_FRAME;
                if flags & (opts::XZ_VEL | opts::KEEP_OTHER) == 0 {
                    v.z = 0.0;
                }
            }
            if flags & opts::Y_VEL != 0 && !(flags & opts::IGNORE_Y_IF_ZERO != 0 && delta.y == 0.0)
            {
                v.y = delta.y * sy * 60.0 + gravity.y * SECONDS_PER_FRAME;
            }
            if flags & opts::XZ_VEL != 0 {
                v.z = delta.z * sz * 60.0 + gravity.z * SECONDS_PER_FRAME;
                if flags & (opts::X_VEL | opts::KEEP_OTHER) == 0 {
                    v.x = 0.0;
                }
            }
            c.transv = c.local_to_world.to_world(v);
        }
        if flags & opts::QUAT != 0 {
            let c = &mut self.control;
            c.quat = (c.quat * quat).normalize();
        }
    }

    /// The forward speed set straight, keeping the vertical.
    pub(crate) fn set_forward_vel(&mut self, speed: f32) {
        let c = &mut self.control;
        let mut v = c.local_to_world.to_local(c.transv);
        v.z = speed;
        v.x = 0.0;
        c.transv = c.local_to_world.to_world(v);
    }

    /// Velocity along the ground kept as the align target the move is
    /// measured against.
    pub(crate) fn keep_align_xz_vel(&mut self) {
        let c = &mut self.control;
        let mut v = c.local_to_world.to_local(c.transv);
        v.y = 0.0;
        c.align_xz_vel = c.local_to_world.to_world(v);
    }

    /// Turns Jak's heading about the vertical, rotation units.
    pub(crate) fn rotate_heading(&mut self, angle: f32) {
        let c = &mut self.control;
        c.quat = (glam::Quat::from_rotation_y(to_radians(angle)) * c.quat).normalize();
    }
}

#[cfg(test)]
mod revision_tests {
    use super::*;
    #[test]
    fn revision_dark_combo_transition_has_no_previous_clip_impulse() {
        let mut jak=Jak::new(Vec3::ZERO,0.0);
        jak.align.anim=Some(crate::anim::DARKJAK_COMBO2);
        jak.align.trans=[Vec3::splat(100_000.0);2];
        jak.chan.anim=crate::anim::DARKJAK_COMBO3;
        jak.chan.frame=0.5;
        jak.compute_delta_align();
        assert!(jak.align.disabled);
        assert_eq!(jak.align.delta_trans,Vec3::ZERO);
        assert_eq!(jak.align.delta_quat,Quat::IDENTITY);
    }
}
