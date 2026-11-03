//! Moving Jak into a new frame of the world: when the world under him is
//! turned and shifted (a planet turned so that its ground under him stays
//! level, the map recentred on him), everything of his that lives in world
//! space turns and shifts with it, so that nothing he is doing notices.
//! Gravity stays the frame's own down.
use glam::{Mat3, Quat, Vec3};

use crate::Jak;
use crate::math::Basis;

/// A point and a direction carried into the new frame.
#[derive(Clone, Copy, Debug)]
pub struct Frame {
    pub rotation: Quat,
    pub offset: Vec3,
}

impl Frame {
    pub fn new(rotation: Quat, offset: Vec3) -> Self {
        Self {
            rotation: rotation.normalize(),
            offset,
        }
    }

    pub fn point(&self, p: Vec3) -> Vec3 {
        self.rotation * p + self.offset
    }

    pub fn dir(&self, v: Vec3) -> Vec3 {
        self.rotation * v
    }

    pub fn quat(&self, q: Quat) -> Quat {
        (self.rotation * q).normalize()
    }

    pub fn basis(&self, b: Basis) -> Basis {
        Basis {
            r: self.dir(b.r),
            u: self.dir(b.u),
            f: self.dir(b.f),
        }
    }

    pub fn mat3(&self, m: Mat3) -> Mat3 {
        Mat3::from_quat(self.rotation) * m
    }
}

impl Jak {
    /// The world turned by `rotation` and then shifted by `offset` (Jak's
    /// units and axes): Jak, his shots, his effects and what he remembers
    /// of the ground go with it. Gravity is not turned.
    pub fn reframe(&mut self, rotation: Quat, offset: Vec3) {
        if !rotation.is_finite() || !offset.is_finite() {
            return;
        }
        let frame = Frame::new(rotation, offset);
        let before = self.control.trans.y;
        self.control.reframe(&frame);
        self.trace.peak += self.control.trans.y - before;
        self.camera = frame.basis(self.camera);
        self.foot.saved_dir = frame.quat(self.foot.saved_dir);
        for p in &mut self.joints {
            *p = frame.point(*p);
        }
        for p in &mut self.projectiles {
            p.trans = frame.point(p.trans);
            p.tail = frame.point(p.tail);
            p.transv = frame.dir(p.transv);
            p.dir = frame.dir(p.dir);
        }
        for (p, _) in &mut self.attack.live {
            *p = frame.point(*p);
        }
        let b = &mut self.board;
        b.slow_transv = frame.dir(b.slow_transv);
        b.up_vector = b.up_vector.map(|v| frame.dir(v));
        b.smack_normal = frame.dir(b.smack_normal);
        b.glance_normal = frame.dir(b.glance_normal);
        b.glance_in_transv = frame.dir(b.glance_in_transv);
        b.glance_out_transv = frame.dir(b.glance_out_transv);
        let g = &mut self.gun;
        g.aim = g.aim.map(|p| frame.point(p));
        g.aim_dir = g.aim_dir.map(|v| frame.dir(v));
        g.fire_point = frame.point(g.fire_point);
        g.fire_dir = frame.dir(g.fire_dir);
        self.weapons.reframe(&frame);
        if let Some(particles) = &mut self.particles {
            particles.reframe(&frame);
        }
        // The ledge he hangs from goes with the world (a platform's edge
        // carries him on).
        if let Some(ledge) = self.ledge.as_mut() {
            let wall = frame.point(ledge.wall);
            ledge.top += wall.y - ledge.wall.y;
            ledge.wall = wall;
            let n = frame.dir(ledge.normal);
            ledge.normal = Vec3::new(n.x, 0.0, n.z).normalize_or(ledge.normal);
        }
        // Rails are the map's own: a ride on one does not survive the move.
        self.grind.ride = None;
        self.cache = Default::default();
        self.probe_cache = Default::default();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reframe_carries_jak_along() {
        let mut jak = Jak::new(Vec3::new(4096.0, 0.0, 0.0), 0.0);
        jak.control.transv = Vec3::new(0.0, 0.0, 4096.0);
        let forward = jak.orientation() * Vec3::Z;
        let turn = Quat::from_rotation_y(std::f32::consts::FRAC_PI_2);
        let shift = Vec3::new(0.0, 100.0, 0.0);
        jak.reframe(turn, shift);
        assert!(jak.trans().abs_diff_eq(turn * Vec3::new(4096.0, 0.0, 0.0) + shift, 1e-2));
        assert!(jak.velocity().abs_diff_eq(turn * Vec3::new(0.0, 0.0, 4096.0), 1e-2));
        assert!((jak.orientation() * Vec3::Z).abs_diff_eq(turn * forward, 1e-4));
        assert_eq!(jak.control.gravity_normal, Vec3::Y);
    }
}
