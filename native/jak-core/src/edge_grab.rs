//! Hanging from a ledge (`target-edge-grab`, `target-edge-grab-jump` and
//! `target-edge-grab-off` in `target2.gc`): falling past the top of a wall
//! with his hands at it, Jak catches it (`jakb-falling-to-edge-grab`) and
//! hangs (`jakb-edge-grab-stance0`). X climbs up with `jakb-edge-grab-to-jump`
//! and its own motion, into a forward jump of `edge-grab-jump-height`;
//! X with the stick pulled away lets go (`jakb-edge-grab-off`), pushed back
//! off the wall.
//!
//! The ledges are found as the original finds them ([`crate::edge_search`]:
//! `find-edge-grabs!` over the triangles the host gives): an up-facing top's
//! side that no other top shares, within his hands' reach, with room for his
//! body to hang and to climb up. The host says how high over the ground below
//! a ledge must stand to be caught ([`Jak::edge_grab_min_height`]).
use glam::{Quat, Vec3};

use crate::anim::{self, NumFunc};
use crate::collide::{CollideCache, CollideWorld, Tri};
use crate::edge_search::{self, Edges, Query};
use crate::math::*;
use crate::pad::button;
use crate::{Jak, State};

/// His hands over his feet as he hangs, at his own size.
const HANG: f32 = meters(1.85);
/// How far from the wall his feet hang.
const OFF_WALL: f32 = meters(0.4);
/// How far ahead of his feet his hands are when the pose does not say.
const HANDS_AHEAD: f32 = meters(0.35);
/// `edge-grab-jump-height-min` and `-max`.
pub const JUMP_HEIGHT: f32 = meters(1.7);
/// After letting go, how long before he catches a ledge again.
const REGRAB: i64 = seconds(0.5);
/// Hands farther than this from the edge are brought to it a step at a
/// time (`logic-target.gc`: 0.3 m, 0.4 m a frame); nearer, put on it.
const HOLD_SNAP: f32 = meters(0.3);
const HOLD_STEP: f32 = meters(0.4);
/// `last-successful-compute-edge-time`: hands this soon after they were last on the edge are put on it, not
/// stepped toward it; and not caught in this long since both that and the grab began: he falls.
const HOLD_RECENT: i64 = seconds(0.2);
const HOLD_GIVE_UP: i64 = seconds(0.5);
/// `target-edge-grab`'s `:trans`: with the gun out, the body is raised (`draw-offset y`) this far, 778.24 units,
/// at 4096 a second.
const GUN_RAISE: f32 = 778.24;
const GUN_RAISE_SPEED: f32 = 4096.0;

/// A ledge he holds: the top of the wall, the wall's outward normal (flat)
/// and a point on the wall under the edge.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Ledge {
    pub top: f32,
    pub normal: Vec3,
    pub wall: Vec3,
}

/// What the host's triangles round a point are, held for the spheres to be tried against.
struct Held<'a>(&'a [Tri]);

impl CollideWorld for Held<'_> {
    fn fill(&mut self, _min: Vec3, _max: Vec3, out: &mut Vec<Tri>) {
        out.extend_from_slice(self.0);
    }
}

impl Jak {
    pub(crate) fn probe(world: &mut dyn CollideWorld, from: Vec3, motion: Vec3, radius: f32) -> Option<crate::collide::TriHit> {
        let mut cache = CollideCache::default();
        cache.fill_line_sphere(world, from, motion, radius);
        cache.probe_line_sphere(from, motion, radius)
    }

    /// His size as the host draws him, over the original's: what his hands, his reach and the room his body
    /// takes are measured in (the original's metres are 4096 units; the model is drawn bigger than that in the
    /// blocks, `drawn_size`).
    pub(crate) fn body_size(&self) -> f32 {
        self.swim_size()
    }

    /// Between his hands (`unknown-vector90`, the average of the two finger joints) in the world, and the way
    /// he faces, flat.
    fn hands_and_heading(&self) -> (Vec3, Vec3) {
        let s = self.body_size();
        let ahead = self.control.quat * Vec3::Z;
        let heading = Vec3::new(ahead.x, 0.0, ahead.z).normalize_or_zero();
        // stand-in: the pose's finger joints, when the host has not sent them, are over his feet and ahead.
        let local = self.hold.unwrap_or(Vec3::new(0.0, HANG, HANDS_AHEAD));
        (self.control.trans + self.control.quat * (local * s), heading)
    }

    /// A ledge in front of his hands, if there is one he can catch: `find-edge-grabs!`.
    pub(crate) fn find_ledge(&self, world: &mut dyn CollideWorld) -> Option<Ledge> {
        if self.time - self.edge_grab_off_time < REGRAB || self.state.is_board() {
            return None;
        }
        // `edgegrab-possible` (`target-real-post`): only falling.
        if self.control.transv.y >= 0.0 {
            return None;
        }
        let (hands, heading) = self.hands_and_heading();
        if heading == Vec3::ZERO {
            return None;
        }
        let s = self.body_size();
        let feet = self.control.trans;
        // `fill-using-bounding-box`: the triangles round him.
        let (lo, hi) = edge_search::fill_box(feet, s);
        let mut tris = Vec::new();
        world.fill(lo, hi, &mut tris);
        if tris.is_empty() {
            return None;
        }
        let mut cache = CollideCache::default();
        cache.fill_box(&mut Held(&tris), lo, hi);
        let query = Query { tris: &tris, cache: &cache, trans: feet, hands, scale: s, min_height: self.edge_grab_min_height };
        let edges = Edges::build(&query);
        // With the stick pushed, that way first; then the way he faces.
        let stick = (self.pad.stick0_speed != 0.0).then(|| self.pad_dir_world()).filter(|dir| *dir != Vec3::ZERO);
        let grab = stick.and_then(|dir| edges.best(dir)).or_else(|| edges.best(heading))?;
        let top_y = grab.center.y;
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
            normal: grab.outward,
            wall: grab.center,
        })
    }

    /// Why [`Self::find_ledge`] finds no ledge now (host diagnostics): what the search had.
    pub fn ledge_why(&self, world: &mut dyn CollideWorld) -> String {
        let (hands, heading) = self.hands_and_heading();
        let s = self.body_size();
        let feet = self.control.trans;
        let (lo, hi) = edge_search::fill_box(feet, s);
        let mut tris = Vec::new();
        world.fill(lo, hi, &mut tris);
        let mut cache = CollideCache::default();
        cache.fill_box(&mut Held(&tris), lo, hi);
        let query = Query { tris: &tris, cache: &cache, trans: feet, hands, scale: s, min_height: self.edge_grab_min_height };
        let edges = Edges::build(&query);
        let found = edges.best(heading);
        format!(
            "{} triangles, {} tops in reach, descending {}, found {:?}",
            tris.len(),
            edges.tris,
            self.control.transv.y < 0.0,
            found.map(|g| (g.center.y, g.outward))
        )
    }

    /// Whether his body's spheres at `ledge` are clear of the world: the hanging ones as he holds on
    /// (`edge-grab-info-method-9`), the climb's room over the ledge before he goes up (`target-edge-grab`'s
    /// leap-up spheres).
    fn ledge_clear(&self, world: &mut dyn CollideWorld, ledge: &Ledge, leap: bool) -> bool {
        let s = self.body_size();
        let center = Vec3::new(ledge.wall.x, ledge.top, ledge.wall.z);
        let spheres = edge_search::spheres(center, ledge.normal, s, leap);
        let lo = spheres.iter().fold(Vec3::splat(f32::MAX), |lo, &(at, r)| lo.min(at - Vec3::splat(r)));
        let hi = spheres.iter().fold(Vec3::splat(f32::MIN), |hi, &(at, r)| hi.max(at + Vec3::splat(r)));
        let mut cache = CollideCache::default();
        cache.fill_box(world, lo, hi);
        let limit = edge_search::floor_limit(ledge.top, self.edge_grab_min_height);
        edge_search::body_clear(&cache, center, ledge.normal, s, !leap, leap, limit)
    }

    /// Hanging from `ledge` at once (a host's test).
    pub fn hang_from(&mut self, ledge: Ledge) {
        self.ledge = Some(ledge);
        self.go(State::EdgeGrab);
    }

    /// Takes hold: facing the wall, hanging under the edge, still.
    pub(crate) fn edge_grab_enter(&mut self) {
        let Some(ledge) = self.ledge else { return };
        let s = self.body_size();
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
            let hands = self.control.trans + self.control.quat * (hold * self.body_size());
            let off = center - hands;
            if off.length() > HOLD_SNAP && self.time_elapsed(self.edge_grab_hold_time, HOLD_RECENT) {
                // Far, and not just now on it: a step toward it; if that goes on for half a second he lets go.
                if self.time_elapsed(self.state_time, HOLD_GIVE_UP) && self.time_elapsed(self.edge_grab_hold_time, HOLD_GIVE_UP) {
                    self.ledge = None;
                    self.edge_grab_off_time = self.time;
                    return Some(State::Falling { uppercut: false });
                }
                self.control.trans += off.normalize_or_zero() * HOLD_STEP;
            } else {
                self.control.trans += off;
                self.edge_grab_hold_time = self.time;
            }
        }
        // Hanging counts as ground for the fall (`target-compute-edge` sets `last-trans-any-surf`).
        self.control.last_trans_any_surf = self.control.trans;
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
    /// the stick pulled away from the wall, off. Each frame, the room his body hangs in must be clear
    /// (`target-compute-edge`: `edge-grab-info-method-9`, else `end-mode`: he falls).
    pub(crate) fn edge_grab_trans(&mut self, world: &mut dyn CollideWorld) -> Option<State> {
        if let Some(ledge) = self.ledge {
            if !self.ledge_clear(world, &ledge, false) {
                self.ledge = None;
                self.edge_grab_off_time = self.time;
                return Some(State::Falling { uppercut: false });
            }
        }
        // With the gun out the body is raised to clear it.
        if self.gun.out {
            self.control.draw_offset_y = seek(self.control.draw_offset_y, GUN_RAISE, GUN_RAISE_SPEED * crate::SECONDS_PER_FRAME);
        }
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
        // Room to stand up there: the leap-up spheres clear (`world-player-leap-up-spheres`); if they are not,
        // X does nothing and he goes on hanging.
        let ledge = self.ledge?;
        if !self.ledge_clear(world, &ledge, true) {
            return None;
        }
        Some(State::EdgeGrabJump)
    }

    /// Up onto the ledge by the animation's own motion, then a forward jump. Each frame the room over the ledge
    /// must stay clear (`target-compute-edge-rider`: `edge-grab-info-method-9` with the leap-up spheres, else
    /// `end-mode`: he falls).
    pub(crate) fn edge_grab_jump_code(&mut self, world: &mut dyn CollideWorld) -> Option<State> {
        if let Some(ledge) = self.ledge {
            if !self.ledge_clear(world, &ledge, true) {
                self.ledge = None;
                self.edge_grab_off_time = self.time;
                return Some(State::Falling { uppercut: false });
            }
        }
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
            let delta = self.control.quat * (self.align.delta_trans * self.body_size());
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
            let delta = self.control.quat * (self.align.delta_trans * self.body_size());
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
            let mut jak=jak;jak.edge_grab_min_height=2.*6400.;jak.control.transv.y=-meters(1.);
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

    /// A run and a jump at a Minecraft wall, as the host plays it: which
    /// jumps (X frame after the start) end hanging from its top.
    #[test]
    fn running_jump_at_minecraft_wall_reports(){
        for height in [2.,3.] {
            let mut caught=Vec::new();let mut peaks=Vec::new();
            for press in (0..60).step_by(3) {
                let mut world=minecraft_ledge(height);
                let mut jak=Jak::new(Vec3::new(0.,0.,-3.*6400.),0.);
                jak.control.set_body_dimensions(0.3*6400.,1.8*6400.);jak.size=0.64;
                jak.foot_smooth_step=Some(1.25*6400.);jak.edge_grab_min_height=2.*6400.;
                let mut grabbed=false;let mut peak=0f32;
                for f in 0..150 {
                    let x=f>=press&&f<press+12;
                    let input=crate::pad::PadInput{held:if x{button::X}else{0},pressed:if f==press{button::X}else{0},left:[0.,1.],..Default::default()};
                    jak.step(&input,&mut world,None);
                    peak=peak.max(jak.trans().y/6400.);
                    if matches!(jak.state,State::EdgeGrab){grabbed=true;}
                }
                if grabbed{caught.push(press);}peaks.push((press,(peak*100.).round()/100.,jak.trans().z/6400.));
            }
            eprintln!("height {height}: caught at presses {caught:?}; (press, peak y, final z) {peaks:?}");
        }
    }

    /// The in-game fixture: Minecraft cubes (six faces each, the host's own
    /// layout), a wall two high four blocks ahead, X at frame 24.
    #[test]
    fn cube_wall_like_the_host_reports(){
        use crate::collide::{Tri,Pat,PatMode};
        let b=6400.;let mut tris=Vec::new();
        let mut cube=|x:f32,y:f32,z:f32|{
            let (x0,y0,z0,x1,y1,z1)=(x*b,y*b,z*b,(x+1.)*b,(y+1.)*b,(z+1.)*b);
            let mut q=|v:[Vec3;4],mode|{let pat=Pat{mode,..Default::default()};tris.push(Tri{v:[v[0],v[1],v[2]],pat});tris.push(Tri{v:[v[0],v[2],v[3]],pat});};
            q([Vec3::new(x0,y1,z0),Vec3::new(x0,y1,z1),Vec3::new(x1,y1,z1),Vec3::new(x1,y1,z0)],PatMode::Ground);
            q([Vec3::new(x0,y0,z0),Vec3::new(x1,y0,z0),Vec3::new(x1,y0,z1),Vec3::new(x0,y0,z1)],PatMode::Wall);
            q([Vec3::new(x0,y0,z0),Vec3::new(x0,y0,z1),Vec3::new(x0,y1,z1),Vec3::new(x0,y1,z0)],PatMode::Wall);
            q([Vec3::new(x1,y0,z1),Vec3::new(x1,y0,z0),Vec3::new(x1,y1,z0),Vec3::new(x1,y1,z1)],PatMode::Wall);
            q([Vec3::new(x1,y0,z0),Vec3::new(x0,y0,z0),Vec3::new(x0,y1,z0),Vec3::new(x1,y1,z0)],PatMode::Wall);
            q([Vec3::new(x0,y0,z1),Vec3::new(x1,y0,z1),Vec3::new(x1,y1,z1),Vec3::new(x0,y1,z1)],PatMode::Wall);
        };
        for x in -5..=5 {for z in -5..=6 {cube(x as f32,-1.,z as f32);} for y in 0..2 {cube(x as f32,y as f32,4.);}}
        let mut world=Blocks(tris);
        let mut jak=Jak::new(Vec3::new(0.5*b,0.,-0.5*b),0.);
        jak.control.set_body_dimensions(0.3*b,1.8*b);jak.size=0.64;jak.foot_smooth_step=Some(1.25*b);jak.edge_grab_min_height=2.*b;
        let mut log=Vec::new();
        for f in 0..105 {
            let x=f>=24&&f<60;
            let input=crate::pad::PadInput{held:if x{button::X}else{0},pressed:if f==24{button::X}else{0},left:[0.,1.],..Default::default()};
            jak.step(&input,&mut world,None);
            if f%3==2 {log.push(format!("{}:{}:{:.2},{:.2}",f/3,jak.state.name(),jak.trans().y/b,jak.trans().z/b));}
        }
        eprintln!("cube wall: {}",log.join(" "));
    }
}

#[cfg(test)]
mod flat_wall_probe {
    use super::*;
    use crate::collide::{Tri, Pat, PatMode};
    struct Blocks(Vec<Tri>);
    impl CollideWorld for Blocks { fn fill(&mut self,_min:Vec3,_max:Vec3,out:&mut Vec<Tri>){out.extend_from_slice(&self.0);} }
    fn wall(high: i32, cull: bool) -> Blocks {
        let b = 6400.0; let mut tris = Vec::new();
        let solid = |x: i32, y: i32, z: i32| (z == 4 && (0..high).contains(&y) && (-5..=5).contains(&x)) || (y == -1 && (-5..=5).contains(&x) && (-5..=6).contains(&z));
        for x in -5..=5 { for z in -5..=6 { for y in -1..high {
            if !solid(x, y, z) { continue; }
            let (x0,y0,z0,x1,y1,z1) = (x as f32*b,y as f32*b,z as f32*b,(x+1) as f32*b,(y+1) as f32*b,(z+1) as f32*b);
            let mut q = |v: [Vec3;4], mode, hidden: bool| { if hidden && cull { return; } let pat = Pat{mode,..Default::default()}; tris.push(Tri{v:[v[0],v[1],v[2]],pat}); tris.push(Tri{v:[v[0],v[2],v[3]],pat}); };
            q([Vec3::new(x0,y1,z0),Vec3::new(x0,y1,z1),Vec3::new(x1,y1,z1),Vec3::new(x1,y1,z0)],PatMode::Ground,solid(x,y+1,z));
            q([Vec3::new(x0,y0,z0),Vec3::new(x1,y0,z0),Vec3::new(x1,y0,z1),Vec3::new(x0,y0,z1)],PatMode::Wall,solid(x,y-1,z));
            q([Vec3::new(x0,y0,z0),Vec3::new(x0,y0,z1),Vec3::new(x0,y1,z1),Vec3::new(x0,y1,z0)],PatMode::Wall,solid(x-1,y,z));
            q([Vec3::new(x1,y0,z1),Vec3::new(x1,y0,z0),Vec3::new(x1,y1,z0),Vec3::new(x1,y1,z1)],PatMode::Wall,solid(x+1,y,z));
            q([Vec3::new(x1,y0,z0),Vec3::new(x0,y0,z0),Vec3::new(x0,y1,z0),Vec3::new(x1,y1,z0)],PatMode::Wall,solid(x,y,z-1));
            q([Vec3::new(x0,y0,z1),Vec3::new(x1,y0,z1),Vec3::new(x1,y1,z1),Vec3::new(x0,y1,z1)],PatMode::Wall,solid(x,y,z+1));
        }}}
        Blocks(tris)
    }
    /// A flat wall of blocks, with the host's hidden faces left out or not, is never grabbed below its top.
    #[test]
    fn flat_wall_is_never_a_ledge() {
        let b = 6400.0;
        for cull in [true, false] {
            let mut world = wall(6, cull);
            let mut found = Vec::new();
            for step in 0..400 {
                let y = step as f32 * 0.01 * b;
                for zf in [3.3f32, 3.5, 3.7] {
                    let mut jak = Jak::new(Vec3::new(0.5*b, y, zf*b), 0.0);
                    jak.control.set_body_dimensions(0.3*b,1.8*b); jak.size = 0.64; jak.edge_grab_min_height = 1.9*b;
                    jak.control.transv = Vec3::new(0.0, -meters(5.0), 0.0);
                    if let Some(l) = jak.find_ledge(&mut world) { found.push((step, zf, l.top / b)); }
                }
            }
            assert!(found.is_empty(), "cull={cull}: a flat wall was grabbed at {:?}", found.iter().step_by(7).collect::<Vec<_>>());
        }
    }
}

/// The original's ledge search over Minecraft blocks the way the host hands them over (an up top's side no other
/// top shares is an edge; the room for his body is checked with the spheres).
#[cfg(test)]
mod ledge_search {
    use super::*;
    use crate::collide::{Pat, PatMode};

    const B: f32 = 6400.0;
    /// The sizes he is tried at: the Rust port's 0.64 and the host's drawn size (a 2 block model of 2.51 m).
    const SCALES: [f32; 2] = [0.64, 1.94 / 2.510636 * B / 4096.0];

    struct Blocks(Vec<Tri>);
    impl CollideWorld for Blocks {
        fn fill(&mut self, _min: Vec3, _max: Vec3, out: &mut Vec<Tri>) {
            out.extend_from_slice(&self.0);
        }
    }

    /// The six faces of the box `lo..hi` (blocks): up, down, -x, +x, -z, +z, each left out where `hidden`.
    fn faces(tris: &mut Vec<Tri>, lo: [f32; 3], hi: [f32; 3], hidden: [bool; 6]) {
        let (x0, y0, z0, x1, y1, z1) = (lo[0] * B, lo[1] * B, lo[2] * B, hi[0] * B, hi[1] * B, hi[2] * B);
        let mut q = |v: [Vec3; 4], mode, hide: bool| {
            if hide {
                return;
            }
            let pat = Pat { mode, ..Default::default() };
            tris.push(Tri { v: [v[0], v[1], v[2]], pat });
            tris.push(Tri { v: [v[0], v[2], v[3]], pat });
        };
        q([Vec3::new(x0, y1, z0), Vec3::new(x0, y1, z1), Vec3::new(x1, y1, z1), Vec3::new(x1, y1, z0)], PatMode::Ground, hidden[0]);
        q([Vec3::new(x0, y0, z0), Vec3::new(x1, y0, z0), Vec3::new(x1, y0, z1), Vec3::new(x0, y0, z1)], PatMode::Wall, hidden[1]);
        q([Vec3::new(x0, y0, z0), Vec3::new(x0, y0, z1), Vec3::new(x0, y1, z1), Vec3::new(x0, y1, z0)], PatMode::Wall, hidden[2]);
        q([Vec3::new(x1, y0, z1), Vec3::new(x1, y0, z0), Vec3::new(x1, y1, z0), Vec3::new(x1, y1, z1)], PatMode::Wall, hidden[3]);
        q([Vec3::new(x1, y0, z0), Vec3::new(x0, y0, z0), Vec3::new(x0, y1, z0), Vec3::new(x1, y1, z0)], PatMode::Wall, hidden[4]);
        q([Vec3::new(x0, y0, z1), Vec3::new(x1, y0, z1), Vec3::new(x1, y1, z1), Vec3::new(x0, y1, z1)], PatMode::Wall, hidden[5]);
    }

    /// Cubes where `solid` says, their faces against another cube left out when `cull` (as the host does), and
    /// partial blocks `boxes` (low and high corners) with all their faces.
    fn world(solid: impl Fn(i32, i32, i32) -> bool, cull: bool, boxes: &[([f32; 3], [f32; 3])]) -> Blocks {
        let mut tris = Vec::new();
        for x in -7..=7 {
            for y in -3..=7 {
                for z in -7..=8 {
                    if !solid(x, y, z) {
                        continue;
                    }
                    let hidden = [solid(x, y + 1, z), solid(x, y - 1, z), solid(x - 1, y, z), solid(x + 1, y, z), solid(x, y, z - 1), solid(x, y, z + 1)].map(|h| h && cull);
                    faces(&mut tris, [x as f32, y as f32, z as f32], [x as f32 + 1.0, y as f32 + 1.0, z as f32 + 1.0], hidden);
                }
            }
        }
        for (lo, hi) in boxes {
            faces(&mut tris, *lo, *hi, [false; 6]);
        }
        Blocks(tris)
    }

    /// The floor (cells at y = -1), and a column of `high` cells along z = 4 over x in `columns`.
    fn wall(columns: std::ops::RangeInclusive<i32>, high: i32) -> impl Fn(i32, i32, i32) -> bool {
        move |x, y, z| (y == -1 && (-6..=6).contains(&x) && (-6..=7).contains(&z)) || (z == 4 && (0..high).contains(&y) && columns.contains(&x))
    }

    /// His feet `y` blocks up and `x` along, falling in front of the wall and facing it, drawn at `scale`.
    fn falling(x: f32, y: f32, scale: f32) -> Jak {
        let mut jak = Jak::new(Vec3::new(x * B, y * B, 3.4 * B), 0.0);
        jak.control.set_body_dimensions(0.3 * B, 1.8 * B);
        jak.size = 0.64;
        jak.drawn_size = Some(scale);
        jak.edge_grab_min_height = 1.9 * B;
        jak.control.transv = Vec3::new(0.0, -meters(3.0), 0.0);
        jak
    }

    /// Where his feet are (blocks) for a top at `top` to be at the middle of the original's reach (1.3 to 2.7 m
    /// over his feet, in his drawn size).
    fn feet_for(top: f32, scale: f32) -> f32 {
        top - (5324.8 + 11059.2) / 2.0 * scale / B
    }

    /// A block with a block on it has no ledge (its top is covered, or its top's room is taken); the real top of
    /// the column does (Adrii 06/11: no grabbing blocks that have blocks on top).
    #[test]
    fn a_block_with_a_block_on_it_is_not_a_ledge_its_top_is() {
        for scale in SCALES {
            for cull in [true, false] {
                let mut blocks = world(wall(0..=1, 3), cull, &[]);
                // The top of the second cell (2 blocks) is in reach from here, the real top (3 blocks) is not.
                let low = falling(1.0, feet_for(2.0, scale), scale);
                assert!(low.find_ledge(&mut blocks).is_none(), "scale {scale} cull {cull}: grabbed under a block: {}", low.ledge_why(&mut blocks));
                // The same spot under a column of two, whose top it is: caught (so the refusal is the block on it).
                let mut two = world(wall(0..=1, 2), cull, &[]);
                assert!(low.find_ledge(&mut two).is_some(), "scale {scale} cull {cull}: a column of two was not caught: {}", low.ledge_why(&mut two));
                // From here the real top is: his hands about at it.
                let high = falling(1.0, feet_for(3.0, scale), scale);
                let ledge = high.find_ledge(&mut blocks).unwrap_or_else(|| panic!("scale {scale} cull {cull}: the column's top was not caught: {}", high.ledge_why(&mut blocks)));
                assert!((ledge.top - 3.0 * B).abs() < 1.0, "cull {cull}: ledge at {}", ledge.top / B);
                assert!(ledge.normal.abs_diff_eq(-Vec3::Z, 1e-3), "outside is toward him: {:?}", ledge.normal);
            }
        }
    }

    /// Nothing along a flat wall is a ledge, whatever his height, until its top is at his hands.
    #[test]
    fn no_ledge_in_the_middle_of_a_wall() {
        for scale in SCALES {
            for cull in [true, false] {
                let mut blocks = world(wall(-3..=3, 3), cull, &[]);
                // Up to where the top (3 blocks) comes into reach (2.7 m over his feet).
                let reach = 3.0 - 11059.2 * scale / B;
                let mut y = 0.0;
                while y < reach - 0.02 {
                    for x in [-0.5, 0.5, 1.0, 2.5] {
                        assert!(falling(x, y, scale).find_ledge(&mut blocks).is_none(), "scale {scale} cull {cull}: a ledge at feet {y}, x {x}");
                    }
                    y += 0.05;
                }
            }
        }
    }

    /// A slab on the top takes the room he climbs into: the column's top is no ledge (a block on it was none
    /// already); the slab's own top may be.
    #[test]
    fn what_stands_on_the_top_is_in_the_way() {
        let column_top = |ledge: Option<Ledge>| ledge.is_some_and(|ledge| (ledge.top - 3.0 * B).abs() < 1.0);
        for scale in SCALES {
            let feet = feet_for(3.0, scale);
            let mut blocks = world(wall(0..=1, 3), true, &[([0.0, 3.0, 4.0], [2.0, 3.5, 5.0])]);
            let jak = falling(1.0, feet, scale);
            assert!(!column_top(jak.find_ledge(&mut blocks)), "slab: {}", jak.ledge_why(&mut blocks));
            // A ceiling low over it too.
            let mut blocks = world(wall(0..=1, 3), true, &[([-3.0, 4.0, 3.0], [4.0, 4.5, 8.0])]);
            let jak = falling(1.0, feet, scale);
            assert!(!column_top(jak.find_ledge(&mut blocks)), "ceiling: {}", jak.ledge_why(&mut blocks));
            // Room enough: caught.
            let mut blocks = world(wall(0..=1, 3), true, &[([-3.0, 6.5, 3.0], [4.0, 7.0, 8.0])]);
            assert!(column_top(falling(1.0, feet, scale).find_ledge(&mut blocks)));
        }
    }

    /// Two tops side by side are one run of edge: it is held at their seam.
    #[test]
    fn a_run_of_tops_holds_at_the_seam() {
        for scale in SCALES {
            for cull in [true, false] {
                let mut blocks = world(wall(0..=1, 3), cull, &[]);
                let jak = falling(1.0, feet_for(3.0, scale), scale);
                assert!(jak.find_ledge(&mut blocks).is_some(), "scale {scale} cull {cull}: {}", jak.ledge_why(&mut blocks));
            }
        }
    }

    /// Only falling: rising he catches nothing (`edgegrab-possible` needs his vertical speed under zero).
    #[test]
    fn only_falling_catches_a_ledge() {
        let mut blocks = world(wall(0..=1, 3), true, &[]);
        let mut jak = falling(1.0, feet_for(3.0, SCALES[1]), SCALES[1]);
        assert!(jak.find_ledge(&mut blocks).is_some());
        jak.control.transv.y = meters(1.0);
        assert!(jak.find_ledge(&mut blocks).is_none());
    }

    /// The climb checks the room over the ledge each frame too: a block put there while he comes up makes him fall.
    #[test]
    fn a_block_put_over_the_ledge_during_the_climb_drops_him() {
        let scale = SCALES[1];
        let mut blocks = world(wall(0..=1, 3), true, &[]);
        let mut jak = falling(1.0, feet_for(3.0, scale), scale);
        let ledge = jak.find_ledge(&mut blocks).expect("a ledge to hang from");
        jak.ledge = Some(ledge);
        jak.go(State::EdgeGrab);
        jak.go(State::EdgeGrabJump);
        assert!(jak.edge_grab_jump_code(&mut blocks).is_none(), "falling with the room clear");
        let mut covered = world(wall(0..=1, 3), true, &[([0.0, 3.0, 4.0], [1.0, 4.0, 5.0])]);
        assert!(matches!(jak.edge_grab_jump_code(&mut covered), Some(State::Falling { .. })));
        assert!(jak.ledge.is_none());
    }

    /// Hands far off the edge are stepped toward it, unless they were on it a moment ago: then put on it at once
    /// (`target-compute-edge`: 0.2 s after `last-successful-compute-edge-time`); and hanging is ground for the fall.
    #[test]
    fn hands_just_on_the_edge_are_put_back_and_hanging_resets_the_fall() {
        let mut jak = Jak::new(Vec3::new(meters(1.5), 0.0, meters(0.5)), 0.0);
        jak.ledge = Some(Ledge { top: meters(1.0), normal: Vec3::X, wall: Vec3::new(meters(1.0), meters(1.0), meters(0.5)) });
        jak.hold = Some(Vec3::new(0.0, meters(0.9), meters(0.35)));
        jak.go(State::EdgeGrab);
        jak.control.last_trans_any_surf = Vec3::new(0.0, meters(50.0), 0.0);
        jak.edge_grab_hold_time = jak.time;
        jak.edge_grab_code();
        let hands = jak.control.trans + jak.control.quat * jak.hold.unwrap();
        assert!(hands.abs_diff_eq(Vec3::new(meters(1.0), meters(1.0), meters(0.5)), 1.0), "hands {hands:?}");
        assert_eq!(jak.control.last_trans_any_surf, jak.control.trans);
    }

    /// Hanging, a block put in the room under the hands makes him let go; the room over the ledge, blocked, keeps
    /// him from climbing (X does nothing); clear, he climbs.
    #[test]
    fn the_hang_and_the_climb_need_their_room() {
        let scale = SCALES[1];
        let mut blocks = world(wall(0..=1, 3), true, &[]);
        let mut jak = falling(1.0, feet_for(3.0, scale), scale);
        let ledge = jak.find_ledge(&mut blocks).expect("a ledge to hang from");
        assert!(jak.ledge_clear(&mut blocks, &ledge, false));
        assert!(jak.ledge_clear(&mut blocks, &ledge, true));
        // A block put right over the ledge.
        let mut covered = world(wall(0..=1, 3), true, &[([0.0, 3.0, 4.0], [1.0, 4.0, 5.0])]);
        assert!(!jak.ledge_clear(&mut covered, &ledge, true));
        // A block jutting from the wall under the hands.
        let mut jutting = world(wall(0..=1, 3), true, &[([0.0, 1.0, 3.0], [2.0, 2.0, 4.0])]);
        assert!(!jak.ledge_clear(&mut jutting, &ledge, false));
        jak.ledge = Some(ledge);
        jak.go(State::EdgeGrab);
        jak.pad.update(&crate::PadInput { held: button::X, pressed: button::X, ..Default::default() });
        jak.state_time = jak.time - seconds(1.0);
        assert!(jak.edge_grab_trans(&mut covered).is_none(), "X climbed into a block");
        assert!(matches!(jak.edge_grab_trans(&mut blocks), Some(State::EdgeGrabJump)));
        // Hanging, the jutting block: he lets go.
        assert!(matches!(jak.edge_grab_trans(&mut jutting), Some(State::Falling { .. })));
    }
}
