//! Hanging from a ledge (`target-edge-grab`, `target-edge-grab-jump` and
//! `target-edge-grab-off` in `target2.gc`): falling past the top of a wall
//! with his hands at it, Jak catches it (`jakb-falling-to-edge-grab`) and
//! hangs (`jakb-edge-grab-stance0`). X climbs up with `jakb-edge-grab-to-jump`
//! and its own motion, into a forward jump of `edge-grab-jump-height`;
//! X with the stick pulled away lets go (`jakb-edge-grab-off`), pushed back
//! off the wall.
//!
//! The original finds its edges in the collide mesh; here a ledge is a wall
//! in front of him whose top, with room over it, is where his hands are.
//! The host says how high over the ground below a ledge must stand to be
//! caught ([`Jak::edge_grab_min_height`]).
use glam::{Quat, Vec3};

use crate::anim::{self, NumFunc};
use crate::collide::{CollideCache, CollideWorld};
use crate::math::*;
use crate::pad::button;
use crate::{Jak, State};

/// His hands over his feet as he hangs, at his own size.
const HANG: f32 = meters(1.85);
/// How far from the wall his feet hang.
const OFF_WALL: f32 = meters(0.4);
/// `edge-grab-jump-height-min` and `-max`.
pub const JUMP_HEIGHT: f32 = meters(1.7);
/// After letting go, how long before he catches a ledge again.
const REGRAB: i64 = seconds(0.5);
/// Hands farther than this from the edge are brought to it a step at a
/// time (`logic-target.gc`: 0.3 m, 0.4 m a frame); nearer, put on it.
const HOLD_SNAP: f32 = meters(0.3);
const HOLD_STEP: f32 = meters(0.4);
/// Not caught in this long: he falls.
const HOLD_GIVE_UP: i64 = seconds(0.5);

/// A ledge he holds: the top of the wall, the wall's outward normal (flat)
/// and a point on the wall under the edge.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Ledge {
    pub top: f32,
    pub normal: Vec3,
    pub wall: Vec3,
}

impl Jak {
    pub(crate) fn probe(world: &mut dyn CollideWorld, from: Vec3, motion: Vec3, radius: f32) -> Option<crate::collide::TriHit> {
        let mut cache = CollideCache::default();
        cache.fill_line_sphere(world, from, motion, radius);
        cache.probe_line_sphere(from, motion, radius)
    }

    /// A ledge in front of his hands, if there is one he can catch.
    pub(crate) fn find_ledge(&self, world: &mut dyn CollideWorld) -> Option<Ledge> {
        if self.time - self.edge_grab_off_time < REGRAB || self.state.is_board() {
            return None;
        }
        // Only on the way down, or about the top of the jump.
        if self.control.transv.y > meters(2.0) {
            return None;
        }
        let s = self.size;
        let ahead = self.control.quat * Vec3::Z;
        let ahead = Vec3::new(ahead.x, 0.0, ahead.z).normalize_or_zero();
        if ahead == Vec3::ZERO {
            return None;
        }
        let feet = self.control.trans;
        let hands = feet.y + HANG * s;
        // The wall: just under where his hands would hold.
        let low = Vec3::new(feet.x, hands - meters(0.35) * s, feet.z);
        let wall = Self::probe(world, low, ahead * meters(0.9) * s, meters(0.1))?;
        if wall.normal.y.abs() > 0.3 {
            return None;
        }
        let normal = Vec3::new(wall.normal.x, 0.0, wall.normal.z).normalize_or_zero();
        // Its top: down onto it from over it, just past the face.
        let over = Vec3::new(wall.intersect.x, hands + meters(0.6) * s, wall.intersect.z) - normal * meters(0.25) * s;
        // Room over the edge: nothing in the way down to it from up there.
        let top = Self::probe(world, over, Vec3::new(0.0, -meters(1.2) * s, 0.0), meters(0.1))?;
        if top.u <= 0.0 || top.normal.y < 0.7 {
            return None;
        }
        let top_y = top.intersect.y;
        if (top_y - hands).abs() > meters(0.45) * s {
            return None;
        }
        // High enough over the ground below him to hang from.
        let reach = self.edge_grab_min_height + meters(0.3);
        if let Some(floor) = Self::probe(world, feet + Vec3::Y * meters(0.2), Vec3::new(0.0, -reach, 0.0), meters(0.2)) {
            // A swept sphere stops with its centre above the floor. The actual
            // floor is its contact point; using the centre rejects exact-height
            // Minecraft ledges by the probe's own radius.
            let floor_y = floor.intersect.y;
            if top_y - floor_y < self.edge_grab_min_height {
                return None;
            }
        }
        Some(Ledge {
            top: top_y,
            normal,
            wall: wall.intersect,
        })
    }

    /// Hanging from `ledge` at once (a host's test).
    pub fn hang_from(&mut self, ledge: Ledge) {
        self.ledge = Some(ledge);
        self.go(State::EdgeGrab);
    }

    /// Takes hold: facing the wall, hanging under the edge, still.
    pub(crate) fn edge_grab_enter(&mut self) {
        let Some(ledge) = self.ledge else { return };
        let s = self.size;
        let face = -ledge.normal;
        let quat = Quat::from_rotation_y(face.x.atan2(face.z));
        self.control.quat = quat;
        self.control.dir_targ = quat;
        let at = Vec3::new(ledge.wall.x, ledge.top - HANG * s, ledge.wall.z) + ledge.normal * OFF_WALL * s;
        self.control.trans = at;
        self.control.transv = Vec3::ZERO;
        self.gun.end_mode();
    }

    /// Hanging: the catch, then the hang over and over; X climbs, X with the
    /// stick pulled back lets go.
    pub(crate) fn edge_grab_code(&mut self) -> Option<State> {
        self.control.transv = Vec3::ZERO;
        // As the original each frame after the pose: his body moved so his
        // finger bases are on the edge (`center-hold`); how far he hangs
        // from the wall is the pose's own.
        if let (Some(ledge), Some(hold)) = (self.ledge, self.hold) {
            let center = Vec3::new(ledge.wall.x, ledge.top, ledge.wall.z);
            let hands = self.control.trans + self.control.quat * (hold * self.size);
            let off = center - hands;
            if off.length() <= HOLD_SNAP {
                self.control.trans += off;
            } else if self.time_elapsed(self.state_time, HOLD_GIVE_UP) {
                self.ledge = None;
                self.edge_grab_off_time = self.time;
                return Some(State::Falling { uppercut: false });
            } else {
                self.control.trans += off.normalize_or_zero() * HOLD_STEP;
            }
        }
        if self.code.pc == 0 {
            self.ja_push(seconds(0.15));
            self.ja_set(anim::FALLING_TO_EDGE_GRAB, NumFunc::seek(1.0), 0.0);
            self.goto(1);
        }
        if self.code.arrived {
            return None;
        }
        self.code.arrived = true;
        self.chan.eval(&self.anims);
        if self.code.pc == 1 && self.chan.done(&self.anims) {
            self.ja_set(anim::EDGE_GRAB_STANCE0, NumFunc::Loop { rate: 1.0 }, 0.0);
            self.goto(2);
        }
        None
    }

    /// X while hanging (after a fifth of a second): up and over, or, with
    /// the stick pulled away from the wall, off.
    pub(crate) fn edge_grab_trans(&mut self, world: &mut dyn CollideWorld) -> Option<State> {
        if !self.time_elapsed(self.state_time, seconds(0.2)) || !self.pad.recently_pressed(button::X) {
            return None;
        }
        let away = self.ledge.is_some_and(|l| {
            let stick = self.pad_dir_world();
            self.pad.stick0_speed > 0.0 && stick.dot(l.normal) > 0.2
        });
        if away {
            return Some(State::EdgeGrabOff);
        }
        // Room to stand up there (`world-player-leap-up-spheres`).
        let s = self.size;
        let ledge = self.ledge?;
        let up = Vec3::new(ledge.wall.x, ledge.top + meters(0.3), ledge.wall.z) - ledge.normal * meters(0.6) * s;
        if Self::probe(world, up, Vec3::Y * meters(1.4) * s, meters(0.3) * s).is_some() {
            return Some(State::EdgeGrabOff);
        }
        Some(State::EdgeGrabJump)
    }

    /// Up onto the ledge by the animation's own motion, then a forward jump.
    pub(crate) fn edge_grab_jump_code(&mut self) -> Option<State> {
        self.control.transv = Vec3::ZERO;
        if self.code.pc == 0 {
            self.ja_set(anim::EDGE_GRAB_TO_JUMP, NumFunc::seek(1.0), 0.0);
            self.compute_delta_align();
            self.goto(1);
        }
        if self.code.arrived {
            return None;
        }
        self.code.arrived = true;
        self.chan.eval(&self.anims);
        self.compute_delta_align();
        if !self.align.disabled {
            let delta = self.control.quat * (self.align.delta_trans * self.size);
            self.control.trans += delta;
        }
        if !self.chan.done(&self.anims) {
            return None;
        }
        self.ledge = None;
        self.control.transv = Vec3::ZERO;
        self.set_forward_vel(16384.0);
        // Up from a vehicle's edge: 2 m higher (`target-edge-grab-jump`).
        let extra = if std::mem::take(&mut self.pilot_edge_grab) { meters(2.0) } else { 0.0 };
        Some(State::Jump {
            min: JUMP_HEIGHT + extra,
            max: JUMP_HEIGHT + extra,
        })
    }

    /// Letting go: `jakb-edge-grab-off` by its motion, then falling, pushed
    /// back from the wall.
    pub(crate) fn edge_grab_off_code(&mut self) -> Option<State> {
        self.control.transv = Vec3::ZERO;
        if self.code.pc == 0 {
            self.ja_set(anim::EDGE_GRAB_OFF, NumFunc::seek(1.0), 0.0);
            self.compute_delta_align();
            self.goto(1);
        }
        if self.code.arrived {
            return None;
        }
        self.code.arrived = true;
        self.chan.eval(&self.anims);
        self.compute_delta_align();
        if !self.align.disabled {
            let delta = self.control.quat * (self.align.delta_trans * self.size);
            self.control.trans += delta;
        }
        if !self.chan.done(&self.anims) {
            return None;
        }
        let back = self.ledge.map_or(Vec3::ZERO, |l| l.normal);
        self.ledge = None;
        self.edge_grab_off_time = self.time;
        self.control.transv = back * meters(10.0);
        Some(State::Falling { uppercut: false })
    }

    /// The stick's direction on the ground, in the world (unit, flat), as
    /// the walk reads it through the camera.
    pub(crate) fn pad_dir_world(&self) -> Vec3 {
        let local = Vec3::new(sin(self.pad.stick0_dir), 0.0, cos(self.pad.stick0_dir));
        let turned = self.camera.to_world(local);
        Vec3::new(turned.x, 0.0, turned.z).normalize_or_zero()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Blocks(Vec<crate::collide::Tri>);
    impl CollideWorld for Blocks {
        fn fill(&mut self,_min:Vec3,_max:Vec3,out:&mut Vec<crate::collide::Tri>){out.extend_from_slice(&self.0);}
    }
    fn minecraft_ledge(height:f32)->Blocks{
        use crate::collide::{Tri,Pat,PatMode};
        let block=6400.;let mut triangles=Vec::new();
        let mut quad=|v:[Vec3;4],mode|{
            let pat=Pat{mode,..Default::default()};
            triangles.push(Tri{v:[v[0],v[1],v[2]],pat});triangles.push(Tri{v:[v[0],v[2],v[3]],pat});
        };
        quad([Vec3::new(-4.,0.,-4.),Vec3::new(-4.,0.,4.),Vec3::new(4.,0.,4.),Vec3::new(4.,0.,-4.)].map(|v|v*block),PatMode::Ground);
        quad([Vec3::new(-3.,0.,0.4),Vec3::new(-3.,height,0.4),Vec3::new(3.,height,0.4),Vec3::new(3.,0.,0.4)].map(|v|v*block),PatMode::Wall);
        quad([Vec3::new(-3.,height,0.4),Vec3::new(-3.,height,4.),Vec3::new(3.,height,4.),Vec3::new(3.,height,0.4)].map(|v|v*block),PatMode::Ground);
        Blocks(triangles)
    }
    #[test]
    fn two_block_ledge_measures_actual_floor_not_probe_sphere_center(){
        for (height,expected) in [(2.,true),(1.5,false)] {
            let mut world=minecraft_ledge(height);
            let jak=Jak::new(Vec3::new(0.,height*6400.-HANG,0.),0.);
            let mut jak=jak;jak.edge_grab_min_height=2.*6400.;
            assert_eq!(jak.find_ledge(&mut world).is_some(),expected,"ledge height {height}");
        }
    }

    /// A block's top edge along the axes: the block fills x 0..1 m, its top
    /// at 1 m, its face at x = 1 m looking +x. However the pose holds his
    /// hands (here 0.35 m ahead, 1.9 m up), they end on the edge and his
    /// body outside the block.
    #[test]
    fn hands_end_on_a_block_edge() {
        let mut jak = Jak::new(Vec3::new(meters(1.5), 0.0, meters(0.5)), 0.0);
        jak.ledge = Some(Ledge {
            top: meters(1.0),
            normal: Vec3::X,
            wall: Vec3::new(meters(1.0), meters(0.65), meters(0.5)),
        });
        jak.hold = Some(Vec3::new(0.0, meters(1.9), meters(0.35)));
        jak.edge_grab_enter();
        jak.edge_grab_code();
        let hands = jak.control.trans + jak.control.quat * jak.hold.unwrap();
        let edge = Vec3::new(meters(1.0), meters(1.0), meters(0.5));
        assert!(hands.abs_diff_eq(edge, 1.0), "hands {hands:?} edge {edge:?}");
        assert!(jak.control.trans.x > meters(1.0), "inside the block: {:?}", jak.control.trans);
        // Facing the wall.
        assert!((jak.control.quat * Vec3::Z).abs_diff_eq(-Vec3::X, 1e-4));
    }

    /// Hands far from the edge (a pose that holds them elsewhere): brought
    /// there 0.4 m a frame, not in one jump.
    #[test]
    fn far_hands_come_a_step_at_a_time() {
        let mut jak = Jak::new(Vec3::new(meters(1.5), 0.0, meters(0.5)), 0.0);
        jak.ledge = Some(Ledge {
            top: meters(1.0),
            normal: Vec3::X,
            wall: Vec3::new(meters(1.0), meters(0.65), meters(0.5)),
        });
        jak.hold = Some(Vec3::new(0.0, meters(0.9), meters(0.35)));
        jak.edge_grab_enter();
        let before = jak.control.trans;
        jak.edge_grab_code();
        let moved = (jak.control.trans - before).length();
        assert!((moved - HOLD_STEP).abs() < 1.0, "moved {moved}");
    }
}
