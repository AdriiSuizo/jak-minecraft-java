//! The Flut Flut in the water (Adrii, 20/10). Neither game lets it swim:
//! ridden into deep water Jak 3 drowns it (`target-flut.gc`, its `'swim`
//! event) and Jak 1 pushes it back ashore. Here it swims as a Minecraft
//! horse does: its body half under the surface, slower than on land, never
//! drowning. Jak's own water control is what tells it (`water.gc`: swim
//! once the surface is over its feet by the swim height), its walk and
//! idle are its strokes and its rest (no Flut Flut has a swimming
//! animation), and X jumps it out (no flap in the water, as on both games'
//! `target-flut-jump`), as does pushing against a bank a block high.
use glam::Vec3;

use crate::collide::CollideWorld;
use crate::control::status;
use crate::math::*;
use crate::pad::button;
use crate::{Event, Jak, State};

/// Its root (its feet) under the surface as it floats: its body half in.
pub const SWIM_HEIGHT: f32 = meters(1.0);
/// Its stroke: a Minecraft horse's in the water, slower than Jak's 7 m/s.
pub const SWIM_SPEED: f32 = meters(4.0);
/// Out of the water when shallower than this over its feet (wading out).
const LEAVE_DEPTH: f32 = meters(0.45);
/// Its bob on the water (Jak 1's `water-control` bob, smaller).
const BOB: f32 = meters(0.05);
/// The jump out of the water: over a bank a block high (1.44 m) from where
/// it floats.
pub const JUMP_OUT_HEIGHT: f32 = SWIM_HEIGHT + meters(1.9);
/// Pushing against a bank this long, it jumps out by itself (seconds).
const CLIMB_OUT_AFTER: f32 = 0.2;

impl Jak {
    /// Whether the ridden bird is in the water this frame (before its
    /// checks): in once the water is over its float height and it is not on
    /// the way up out of it, out again in the shallows.
    pub(crate) fn flut_water(&mut self) {
        let riding = self.state.is_flut() && !matches!(self.state, State::FlutGetOn | State::FlutGetOffJump);
        let Some(surface) = self.water.filter(|_| riding) else {
            self.flut.swimming = false;
            self.flut.blocked = 0.0;
            return;
        };
        let depth = surface - self.control.trans.y;
        let up = self.control.gravity_normal.dot(self.control.transv);
        if self.flut.swimming {
            // Jumping out, or walked up into the shallows.
            if (up > meters(1.0) && depth < SWIM_HEIGHT) || depth < LEAVE_DEPTH {
                self.flut.swimming = false;
                self.flut.blocked = 0.0;
            }
        } else if depth >= SWIM_HEIGHT * 0.8 && up <= 0.0 {
            self.flut.swimming = true;
            self.flut.water_jump = false;
            self.events.push(Event::Splash {
                pos: Vec3::new(self.control.trans.x, surface, self.control.trans.z),
                speed: -up,
            });
        }
    }

    /// The bird's move while it swims: no gravity, its feet held the swim
    /// height under the surface (bobbing), at its stroke's speed; it counts
    /// as on the ground (it stands, walks, turns, Triangle gets off). False
    /// when it is not swimming (its usual move runs).
    pub(crate) fn flut_swim_post(&mut self, world: &mut dyn CollideWorld) -> bool {
        // On the way up out of it (a jump), not held.
        let rising = self.control.gravity_normal.dot(self.control.transv) > 0.0;
        if !self.flut.swimming || (rising && matches!(self.state, State::FlutJump { .. } | State::FlutDoubleJump { .. })) {
            return false;
        }
        let Some(surface) = self.water else {
            return false;
        };
        let bob = BOB * (self.time as f32 * 0.012).sin();
        let float = surface - SWIM_HEIGHT * self.size + bob;
        let vy = ((float - self.control.trans.y) * 4.0).clamp(-meters(3.0), meters(3.0));
        let surface_mods = self.control.mod_surface;
        self.control.mod_surface.target_speed = surface_mods.target_speed.min(SWIM_SPEED);
        self.control.mod_surface.transv_max = surface_mods.transv_max.min(SWIM_SPEED);
        let gravity = self.control.gravity_length;
        self.control.gravity_length = 0.0;
        self.control.transv.y = vy;
        let flat = Vec3::new(self.control.transv.x, 0.0, self.control.transv.z);
        if flat.length() > SWIM_SPEED {
            let keep = SWIM_SPEED / flat.length();
            self.control.transv.x *= keep;
            self.control.transv.z *= keep;
        }
        self.target_post(world);
        self.control.gravity_length = gravity;
        self.control.mod_surface = surface_mods;
        // Held up by the water: standing, as far as its states go.
        self.control.status |= status::ON_SURFACE | status::ON_GROUND;
        self.control.last_time_on_surface = self.time;
        // A bank it pushes against: out over it with a jump.
        let moving = self.pad.stick0_speed > 0.1;
        let along = Vec3::new(self.control.transv.x, 0.0, self.control.transv.z).length();
        let wall = self.control.status & status::TOUCH_WALL != 0;
        self.flut.blocked = if moving && wall && along < meters(1.0) {
            self.flut.blocked + SECONDS_PER_FRAME
        } else {
            0.0
        };
        if self.flut.blocked >= CLIMB_OUT_AFTER
            && self.pending.is_none()
            && matches!(self.state, State::FlutWalk | State::FlutStance)
            && self.flut_bank_ahead(world, surface)
        {
            self.flut.blocked = 0.0;
            self.pending = Some(self.flut_jump_out());
        }
        true
    }

    /// Ground to climb onto just ahead: its top no higher than a block and
    /// a half over the water (a bank, not a wall to swim along).
    fn flut_bank_ahead(&self, world: &mut dyn CollideWorld, surface: f32) -> bool {
        let ahead = self.control.quat * Vec3::Z;
        let ahead = Vec3::new(ahead.x, 0.0, ahead.z).normalize_or_zero();
        let from = Vec3::new(self.control.trans.x, surface + meters(2.2), self.control.trans.z) + ahead * meters(1.4);
        let motion = Vec3::new(0.0, -meters(2.6), 0.0);
        let mut cache = crate::collide::CollideCache::default();
        cache.fill_line_sphere(world, from, motion, meters(0.2));
        cache
            .probe_line_sphere(from, motion, meters(0.2))
            .is_some_and(|hit| hit.u > 0.0)
    }

    /// X in the water: up and out, high enough for a bank a block high,
    /// carried forward if the stick asks (no flap after it).
    pub(crate) fn flut_jump_out(&mut self) -> State {
        self.flut.water_jump = true;
        self.flut.swimming = false;
        if let Some(surface) = self.water {
            self.events.push(Event::Splash {
                pos: Vec3::new(self.control.trans.x, surface, self.control.trans.z),
                speed: -meters(2.0),
            });
        }
        State::FlutJump {
            min: JUMP_OUT_HEIGHT * self.size,
            max: JUMP_OUT_HEIGHT * self.size,
        }
    }

    /// Swimming, Triangle by a Flut Flut floating near: on it from the
    /// water (`target-swim`'s handler takes `'change-mode 'flut` as the foot
    /// states do).
    pub(crate) fn flut_mount_from_water(&mut self) -> bool {
        if self.swim.is_none_or(|s| s.diving) {
            return false;
        }
        let Some((trans, quat)) = self.flut.request.take() else {
            return false;
        };
        self.swim = None;
        self.swim_tilt = 0.0;
        self.control.mod_surface = crate::surface::WALK;
        self.flut_set_mount(trans, quat);
        self.go(State::FlutGetOn);
        true
    }

    /// X while swimming on it: out of the water.
    pub(crate) fn flut_swim_jump(&mut self) -> Option<State> {
        (self.flut.swimming && self.pad.recently_pressed(button::X)).then(|| self.flut_jump_out())
    }
}
