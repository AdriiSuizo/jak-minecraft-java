//! Swimming (`target-swim.gc`): in water deeper than he stands, Jak swims
//! at the surface with `*swim-mods*` (a 7 m/s stroke, slow turns, guns
//! away), his head above it. Square or L1 takes him under, as
//! `target-swim-down` does; there, unlike the original's fixed dive, the
//! player steers up and down (X rises, Square or L1 sinks; letting go he
//! swims on level with the surface stroke, or drifts back up when still).
//! X at the surface jumps him out (`swim-jump`).
//!
//! The host says where the water's surface is each frame
//! ([`Jak::water`]); without water he never swims.
use glam::Vec3;

use crate::anim::{self, Anim, NumFunc};
use crate::collide::CollideWorld;
use crate::math::*;
use crate::pad::button;
use crate::{Event, Jak, State};

/// How deep the water is before he swims, and where he floats: the
/// surface this far above his feet (`swim-height`, 1.8 m).
pub const SWIM_HEIGHT: f32 = 7372.8;
/// Shallower than this he wades out and stands again.
const LEAVE_DEPTH: f32 = 5734.4;
/// Water at least this deep over his feet and he wades (`target-wade`).
pub const WADE_DEPTH: f32 = meters(0.5);

/// Swimming, he wades out only this far shallower than he floats.
const LEAVE_UNDER_FLOAT: f32 = meters(0.4);

/// How far the top of his head stays over the water as he floats.
pub const CROWN_OUT: f32 = meters(0.3);
/// Sinking and rising under water, meters a second.
const DIVE_SPEED: f32 = meters(3.0);
const RISE_SPEED: f32 = meters(3.5);
const DRIFT_UP: f32 = meters(1.5);
/// The jump out of the water (`swim-jump-height-min`..`max`), over the
/// height he floats at: Minecraft's banks stand straight out of the water,
/// their top a little over its surface, and his feet must clear them.
const SWIM_JUMP_HEIGHT: f32 = meters(1.1);
/// Pushing against a bank this long while floating, he climbs out by
/// himself (seconds).
const CLIMB_OUT_AFTER: f32 = 0.2;

/// Where he is in the water.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Swim {
    /// Under the surface.
    pub diving: bool,
    /// When he went into the water (ticks).
    pub since: i64,
    /// How long he has pushed against a bank at the surface, seconds.
    pub blocked: f32,
}

impl Jak {
    /// Ground to climb onto just ahead: its top no higher than a meter over
    /// the water's surface (a bank, not a wall to swim along).
    fn low_bank(&self, world: &mut dyn CollideWorld, surface: f32) -> bool {
        let ahead = self.control.quat * Vec3::Z;
        let ahead = Vec3::new(ahead.x, 0.0, ahead.z).normalize_or_zero();
        let from = Vec3::new(self.control.trans.x, surface + meters(1.0), self.control.trans.z) + ahead * meters(0.8);
        let motion = Vec3::new(0.0, -meters(1.4), 0.0);
        let mut cache = crate::collide::CollideCache::default();
        cache.fill_line_sphere(world, from, motion, meters(0.2));
        cache
            .probe_line_sphere(from, motion, meters(0.2))
            .is_some_and(|hit| hit.u > 0.0)
    }

    /// The size the water measures him by: his drawn one.
    pub(crate) fn swim_size(&self) -> f32 {
        self.drawn_size.unwrap_or(self.size)
    }

    /// How far under the surface his feet float: the swim height, less
    /// if the pose would hold the top of his head under the water.
    pub(crate) fn float_depth(&self) -> f32 {
        let (size, swim_height) = (self.swim_size(), SWIM_HEIGHT * self.swim_size());
        match self.crown {
            Some(crown) => swim_height.min((crown.y - CROWN_OUT) * size),
            None => swim_height,
        }
    }

    /// Wading, how much of his walking speed he keeps: from 0.8 in half a
    /// meter of water to 0.6 in two (`target-wade-walk`'s 10 m/s times
    /// that: 7.33 m/s in a meter).
    pub(crate) fn wade_scale(&self) -> f32 {
        match self.wade {
            Some(depth) => {
                let t = ((depth / self.swim_size() - WADE_DEPTH) / meters(1.5)).clamp(0.0, 1.0);
                0.8 - 0.2 * t
            }
            None => 1.0,
        }
    }

    pub fn swimming(&self) -> bool {
        self.swim.is_some()
    }

    /// One frame in the water; false when he is not swimming (and the
    /// usual states run).
    pub(crate) fn swim_step(&mut self, world: &mut dyn CollideWorld) -> bool {
        self.wade = self.water.and_then(|surface| {
            let depth = surface - self.control.trans.y;
            let on_ground = self.control.status & crate::control::status::ON_GROUND != 0;
            (self.swim.is_none() && on_ground && depth >= WADE_DEPTH * self.swim_size() && depth < SWIM_HEIGHT * self.swim_size())
                .then_some(depth)
        });
        // Triangle by a Flut Flut, swimming: onto it.
        if self.flut_mount_from_water() {
            return false;
        }
        let Some(surface) = self.water else {
            self.swim_tilt = 0.0;
            if self.swim.take().is_some() {
                self.control.mod_surface = crate::surface::WALK;
            }
            return false;
        };
        let depth = surface - self.control.trans.y;
        // As deep as his drawn body would float in his own game.
        let (swim_height, leave_depth) = (SWIM_HEIGHT * self.swim_size(), LEAVE_DEPTH * self.swim_size());
        if self.swim.is_none() {
            // Dark and Light Jak swim as he does.
            // Dropping into it (off the board as it slows, from a jump)
            // he swims as soon as it is past his knees, not running on as he
            // sinks to swimming depth (15/10).
            let falling_in = self.control.status & crate::control::status::ON_GROUND == 0
                && self.control.transv.y <= 0.0
                && depth >= 0.35 * swim_height;
            // On the Flut Flut it swims instead ([`crate::flut_swim`]).
            if (depth < swim_height && !falling_in) || self.state.is_board() || self.state.is_flut() {
                return false;
            }
            self.swim = Some(Swim {
                diving: false,
                since: self.time,
                blocked: 0.0,
            });
            self.gun.end_mode();
            self.state = State::Stance;
            self.state_time = self.time;
            self.events.push(Event::Splash {
                pos: Vec3::new(self.control.trans.x, surface, self.control.trans.z),
                speed: -self.control.transv.y,
            });
        }
        let Some(mut swim) = self.swim else { return false };
        let down = self.pad.hold(button::SQUARE) || self.pad.hold(button::L1);
        let up = self.pad.hold(button::X);
        // X at the surface, or a bank pushed against: out of the water with
        // a jump high enough to land on it, carried forward onto it.
        let climb_out = swim.blocked >= CLIMB_OUT_AFTER && self.low_bank(world, surface);
        if !swim.diving && (self.pad.pressed(button::X) || climb_out) {
            self.swim = None;
            self.swim_tilt = 0.0;
            let height = self.float_depth() + SWIM_JUMP_HEIGHT * self.swim_size();
            // Out of the water into his own jump (its animation, not the
            // walk the state he swam in would go on with), as high as the
            // bank needs.
            self.go(crate::State::Jump { min: height, max: height });
            // This takeoff is already at the actual floating feet position.
            // The normal ground jump subtracts JUMP_COLLIDE_OFFSET to account
            // for floor separation. Applying that again here consumed nearly
            // all of the clearance over water at the Minecraft avatar scale.
            self.init_var_jump(height,height,true,false,0.0);
            if climb_out || self.pad.stick0_speed > 0.1 {
                let ahead = self.control.quat * Vec3::Z;
                self.control.transv += Vec3::new(ahead.x, 0.0, ahead.z).normalize_or_zero() * meters(3.0);
            }
            self.events.push(Event::Splash {
                pos: Vec3::new(self.control.trans.x, surface, self.control.trans.z),
                // Out of the water: negative.
                speed: -meters(2.0),
            });
            return false;
        }
        // Out of the water only well over where he floats: floating higher
        // for his head (`float_depth`), he never wades out and falls back
        // in over and over (a splash each time).
        let leave_depth = leave_depth.min(self.float_depth() - LEAVE_UNDER_FLOAT * self.swim_size());
        // (Just dropped in he is still sinking to it: not out again yet.)
        let settling = self.time - swim.since < seconds(0.5)
            && self.control.status & crate::control::status::ON_GROUND == 0;
        if depth < leave_depth && !swim.diving && !settling {
            self.swim = None;
            self.swim_tilt = 0.0;
            self.control.mod_surface = crate::surface::WALK;
            return false;
        }
        if down && !swim.diving {
            swim.diving = true;
        }
        // Back at the surface from below.
        if swim.diving && !down && depth <= swim_height {
            swim.diving = false;
        }
        let moving = self.pad.stick0_speed > 0.1;
        let vy = if swim.diving {
            if down {
                -DIVE_SPEED
            } else if up {
                RISE_SPEED
            } else if moving {
                // Swimming along under water he keeps his depth.
                0.0
            } else {
                DRIFT_UP
            }
        } else {
            // Floating: his feet held the swim height under the surface,
            // his head always out of it (Adrii: only Daxter showed). The
            // stroke's own pose lifts his body in his game; whatever the
            // drawn model's size, the top of his head ends over the water.
            let float = self.float_depth();
            ((surface - float - self.control.trans.y) * 4.0).clamp(-RISE_SPEED, RISE_SPEED)
        };
        self.swim = Some(swim);
        // The stroke: the usual walk physics with the swim surface and no
        // gravity, the climb set directly.
        self.control.mod_surface = crate::surface::SWIM;
        let gravity = self.control.gravity_length;
        self.control.gravity_length = 0.0;
        self.control.transv.y = vy;
        self.target_post(world);
        self.control.gravity_length = gravity;
        // Swimming into something at the surface (a bank): it is counted,
        // for him to climb out once he has pushed a moment.
        let along = Vec3::new(self.control.transv.x, 0.0, self.control.transv.z).length();
        if let Some(s) = self.swim.as_mut() {
            let wall = self.control.status & crate::control::status::TOUCH_WALL != 0;
            s.blocked = if !s.diving && moving && wall && along < meters(1.5) {
                s.blocked + crate::SECONDS_PER_FRAME
            } else {
                0.0
            };
        }
        if self.control.transv.y.abs() < vy.abs() * 0.5 && vy < 0.0 {
            // On the bottom: stop sinking.
            self.control.transv.y = 0.0;
        }
        // Under water and going somewhere he leans into it: his dive laid
        // level as he swims along, half way as he dives ahead.
        let tilt = match (swim.diving, moving, down, up) {
            (true, true, false, false) => std::f32::consts::FRAC_PI_2,
            (true, true, true, false) => std::f32::consts::FRAC_PI_4,
            _ => 0.0,
        };
        self.swim_tilt = crate::math::seek(self.swim_tilt, tilt, 3.0 * crate::SECONDS_PER_FRAME);
        let want: Anim = if swim.diving {
            if down || (moving && !up) {
                // Head first: straight down, or laid level as he swims
                // along (`swim_tilt`).
                anim::SWIM_DOWN
            } else if up {
                anim::SWIM_UP
            } else {
                // Still: treading water as at the surface, drifting up.
                anim::SWIM_STANCE
            }
        } else if moving {
            anim::SWIM_WALK
        } else {
            anim::SWIM_STANCE
        };
        if !self.chan.is(want) {
            self.chan.set(want, NumFunc::Loop { rate: 1.0 }, 0.0);
        }
        self.chan.eval(&self.anims.clone());
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn surface_jump_clears_water_at_minecraft_scale() {
        let block=6400.0;
        let mut jak=Jak::new(Vec3::ZERO,0.0);
        jak.size=0.64;
        // The model is drawn 1.94 blocks tall for 2.51 m of his own.
        jak.drawn_size=Some(1.94/2.510636*block/crate::METER);
        jak.control.set_body_dimensions(0.3*block,1.8*block);
        jak.water=Some(2.0*block);
        jak.control.trans.y=2.0*block-jak.float_depth();
        jak.swim=Some(Swim{diving:false,since:0,blocked:0.0});
        let mut world=crate::collide::EmptyWorld;
        for _ in 0..60 {jak.step(&crate::PadInput::default(),&mut world,None);}
        let mut peak=f32::NEG_INFINITY;
        for frame in 0..90 {
            jak.step(&crate::PadInput{held:button::X,pressed:if frame==0{button::X}else{0},..Default::default()},&mut world,None);
            peak=peak.max(jak.trans().y-2.0*block);
        }
        // `SWIM_JUMP_HEIGHT` drawn: 1.1 m of 1.2 blocks to the metre.
        assert!(peak>0.75*block,"feet only rise {} blocks over the water",peak/block);
        assert!(peak<0.95*block,"water jump must retain the authored height: {}",peak/block);
    }

    /// Drawn two blocks tall, he floats as deep as in his game: his feet
    /// 1.8 of his meters under it (1.4 blocks), not 0.74 as at `size`.
    #[test]
    fn floats_by_his_drawn_size() {
        let block=6400.0;
        let mut jak=Jak::new(Vec3::ZERO,0.0);
        jak.size=0.64;
        jak.drawn_size=Some(1.94/2.510636*block/crate::METER);
        let under=jak.float_depth()/block;
        assert!((under-1.39).abs()<0.02,"feet {under} blocks under the water");
    }

    /// Floating, the top of his head stays over the water however the pose
    /// holds it; a pose that holds it high keeps the game's swim height.
    #[test]
    fn head_stays_out_of_the_water() {
        let mut jak = Jak::new(Vec3::ZERO, 0.0);
        jak.size = 1.3;
        jak.crown = Some(Vec3::new(0.0, meters(1.5), 0.0));
        let feet_under = jak.float_depth();
        let crown_over = jak.crown.unwrap().y * jak.size - feet_under;
        assert!(crown_over >= CROWN_OUT * jak.size - 1.0, "head {crown_over} over the water");
        jak.crown = Some(Vec3::new(0.0, meters(2.5), 0.0));
        assert_eq!(jak.float_depth(), SWIM_HEIGHT * jak.size);
    }

    /// A Minecraft block of water wades him at about 7.3 m/s of his 10.
    #[test]
    fn a_meter_of_water_slows_his_walk() {
        let mut jak = Jak::new(Vec3::ZERO, 0.0);
        jak.wade = Some(meters(1.0));
        assert!((jak.wade_scale() - 0.7333).abs() < 1e-3);
        jak.wade = None;
        assert_eq!(jak.wade_scale(), 1.0);
    }

    /// However high he floats for his head, he wades out only well over
    /// it: floating never takes him out of the water.
    #[test]
    fn floating_never_leaves_the_water() {
        let mut jak = Jak::new(Vec3::ZERO, 0.0);
        jak.size = 1.3;
        jak.crown = Some(Vec3::new(0.0, meters(1.2), 0.0));
        let leave = (LEAVE_DEPTH * jak.size).min(jak.float_depth() - LEAVE_UNDER_FLOAT * jak.size);
        assert!(jak.float_depth() > leave);
    }
}
