//! Jak's gun: drawing it, choosing among the four guns and their three
//! modes on the D-pad, and the fire timing each mode keeps. What a mode
//! fires is in [`crate::weapons`].
use glam::Vec3;

use crate::control::Control;
use crate::math::*;
use crate::pad::{Pad, button};
use crate::weapons::{FireControl, GunColor, GunKind};
use crate::{Event, Jak, State};

/// Ticks between the Blaster's shots.
pub const FIRE_DELAY: i64 = 96;
pub const AMMO_MAX: f32 = 100.0;
pub const AMMO_PER_SHOT: f32 = 1.0;
/// How far ahead an auto-aimed target may be.
pub const TRACK_FIND_RANGE: f32 = 286720.0;
pub const FIRE_RANGE: f32 = 409600.0;
/// Where the muzzle sits relative to Jak's origin while he holds the gun:
/// up, then ahead.
pub const MUZZLE_UP: f32 = meters(1.2);
pub const MUZZLE_AHEAD: f32 = meters(0.6);
/// The blue gun's barrels at full spin (`fire-spinv`), and how fast they
/// spin up while R1 is held and down after.
pub const SPIN_MAX: f32 = 218453.33;
const SPIN_UP: f32 = 145635.56;
const SPIN_DOWN: f32 = 109226.664;
/// Holding a D-pad direction this long, with R1 up, puts the gun away.
const PUT_AWAY_HOLD: i64 = seconds(0.8);
/// The most ammo each color holds: red, yellow, blue, dark.
pub const AMMO_CAP: [f32; 4] = [50.0, AMMO_MAX, AMMO_MAX, 20.0];

#[derive(Clone, Debug)]
pub struct Gun {
    pub out: bool,
    pub latch: bool,
    pub get_on_time: i64,
    gun_time: i64,
    pub fire_time: i64,
    pub fire_pending: i32,
    pub fire_pending_time: i64,
    pub active: bool,
    pub active_time: i64,
    /// Ammo per gun color: red, yellow, blue, dark.
    pub ammo: [f32; 4],
    pub endless_ammo: bool,
    /// The host's inventory picks the gun instead of the D-pad: `Some(None)`
    /// keeps his hands empty, `Some(Some(kind))` has that gun out.
    pub equipped: Option<Option<GunKind>>,
    pub turn_blend: f32,
    /// A point the host's auto-aim picked, in Jak space; the gun fires at it
    /// instead of straight ahead.
    pub aim: Option<Vec3>,
    /// Aiming by hand (the host's over-the-shoulder aim): the gun fires
    /// this way, whatever auto-aim found.
    pub aim_dir: Option<Vec3>,
    /// Exact crosshair intersection, independent of the muzzle animation.
    pub aim_point: Option<Vec3>,
    /// Posed muzzle relative to Jak, supplied by the model host.
    pub muzzle_local: Option<Vec3>,
    /// Optional view-rig basis. First-person aim must not turn with locomotion.
    pub muzzle_basis: Option<glam::Quat>,
    /// Optional host eye offset for preventing a long view weapon crossing walls.
    pub muzzle_guard: Option<Vec3>,
    pub fire_point: Vec3,
    pub fire_dir: Vec3,
    /// The mode in hand, and when each mode last fired (the dark guns'
    /// cooldowns read it).
    pub kind: GunKind,
    pub last_fire: [i64; 12],
    /// When each D-pad direction was last pressed: up, down, left, right.
    pub select_time: [i64; 4],
    pub select_change_time: i64,
    /// The blue gun's barrels: their speed and angle.
    pub spinv: f32,
    pub spin: f32,
}

const LONG_AGO: i64 = i64::MIN / 4;

impl Default for Gun {
    fn default() -> Self {
        Self {
            out: false,
            latch: false,
            get_on_time: LONG_AGO,
            gun_time: LONG_AGO,
            fire_time: LONG_AGO,
            fire_pending: 0,
            fire_pending_time: LONG_AGO,
            active: false,
            active_time: LONG_AGO,
            ammo: AMMO_CAP,
            endless_ammo: false,
            equipped: None,
            turn_blend: 0.0,
            aim: None,
            aim_dir: None,
            aim_point: None,
            muzzle_local: None,
            muzzle_basis: None,
            muzzle_guard: None,
            fire_point: Vec3::ZERO,
            fire_dir: Vec3::Z,
            kind: GunKind::Yellow1,
            last_fire: [LONG_AGO; 12],
            select_time: [LONG_AGO; 4],
            select_change_time: LONG_AGO,
            spinv: 0.0,
            spin: 0.0,
        }
    }
}

impl Gun {
    pub(crate) fn end_mode(&mut self) {
        self.out = false;
        self.active = false;
        self.latch = false;
        self.fire_pending = 0;
    }

    pub(crate) fn time_since_use(&self, now: i64) -> i64 {
        now.saturating_sub(self.gun_time)
    }

    /// The ammo of the color in hand.
    pub fn current_ammo(&self) -> f32 {
        self.ammo[self.kind.color() as usize]
    }

    pub fn has_ammo(&self, kind: GunKind) -> bool {
        self.endless_ammo || self.ammo[kind.color() as usize] >= kind.ammo_required()
    }

    pub(crate) fn use_ammo(&mut self, color: GunColor, amount: f32) {
        if !self.endless_ammo {
            let a = &mut self.ammo[color as usize];
            *a = (*a - amount).max(0.0);
        }
    }

    /// Muzzle and aim from Jak's pose.
    fn compute_pos(&mut self, c: &Control,world:Option<&mut dyn crate::collide::CollideWorld>) {
        let forward = flatten(z_axis(c.quat), crate::math::y_axis(c.quat)).normalize_or(Vec3::Z);
        self.fire_point = self.muzzle_local.filter(|p| p.is_finite()).map_or_else(
            || c.trans + Vec3::new(0.0, MUZZLE_UP, 0.0) + forward * MUZZLE_AHEAD,
            |local| c.trans + self.muzzle_basis.filter(|q|q.is_finite()).unwrap_or(c.quat) * local,
        );
        if let (Some(eye),Some(world))=(self.muzzle_guard.filter(|v|v.is_finite()),world) {
            let start=c.trans+eye;let motion=self.fire_point-start;
            if motion.length_squared()>1. {
                let radius=crate::math::meters(0.1875);
                let mut cache=crate::collide::CollideCache::default();
                cache.fill_line_sphere(world,start,motion,radius);
                if let Some(hit)=cache.resolve_moving_sphere(start,radius,motion,-1.,true){
                    self.fire_point=start+motion*hit.u.max(0.);
                }
            }
        }
        let range = self.kind.track_range();
        if let Some(dir) = self.aim_point.and_then(|p| (p - self.fire_point).try_normalize())
            .or_else(|| self.aim_dir.and_then(Vec3::try_normalize))
        {
            self.fire_dir = dir;
            return;
        }
        self.fire_dir = match self.aim {
            Some(target) if target.distance(self.fire_point) <= range => {
                (target - self.fire_point).normalize_or(forward)
            }
            _ => forward,
        };
    }

    /// The D-pad, as `target-gun-marking-menu` reads it: up red, down
    /// yellow, left blue, right dark. The color in hand steps to its next
    /// mode; another color starts at its first. Returns the new mode.
    pub(crate) fn select(&mut self, now: i64, pad: &Pad) -> Option<GunKind> {
        let dirs = [
            (button::UP, GunColor::Red),
            (button::DOWN, GunColor::Yellow),
            (button::LEFT, GunColor::Blue),
            (button::RIGHT, GunColor::Dark),
        ];
        for (slot, (bit, color)) in dirs.into_iter().enumerate() {
            if !pad.pressed(bit) {
                continue;
            }
            self.select_time[slot] = now;
            let next = if self.kind.color() == color {
                self.kind.next_mode()
            } else {
                GunKind::of(color, 1)
            };
            self.kind = next;
            self.select_change_time = now;
            self.fire_pending = 0;
            self.latch = true;
            return Some(next);
        }
        None
    }

    /// A D-pad direction held 0.8 s with R1 up puts the gun away.
    fn put_away_held(&self, now: i64, pad: &Pad) -> bool {
        let dirs = [button::UP, button::DOWN, button::LEFT, button::RIGHT];
        !pad.hold(button::R1)
            && dirs
                .iter()
                .zip(self.select_time)
                .any(|(&bit, at)| pad.hold(bit) && now - at >= PUT_AWAY_HOLD)
    }

    /// One frame of the gun on foot: draw it on R1, queue a shot as the
    /// mode's control reads the trigger, and hand back the mode to fire once
    /// its delay allows. `charging` holds the trigger modes off while their
    /// last shot still charges.
    pub(crate) fn check(
        &mut self,
        now: i64,
        pad: &Pad,
        c: &Control,
        board_latch: bool,
        charging: bool,
        world:&mut dyn crate::collide::CollideWorld,
    ) -> Option<GunKind> {
        let elapsed = |since: i64, d: i64| now - since >= d;
        if !self.out {
            if pad.hold(button::R1) {
                self.latch = true;
            }
            if self.latch && !board_latch && elapsed(self.gun_time, seconds(0.1)) {
                self.out = true;
                self.latch = false;
                self.get_on_time = now;
                self.active = false;
            } else {
                self.spinv = seek(self.spinv, 0.0, SPIN_DOWN * SECONDS_PER_FRAME);
                return None;
            }
        }
        if (board_latch || self.put_away_held(now, pad))
            && elapsed(self.get_on_time, seconds(0.1))
        {
            self.end_mode();
            return None;
        }
        self.compute_pos(c,Some(world));
        self.active = elapsed(self.get_on_time, seconds(0.1));
        if self.active {
            self.active_time = now;
        }
        if !self.active
            && elapsed(self.fire_pending_time, seconds(0.2))
            && elapsed(self.active_time, seconds(0.2))
            && self.fire_pending == 1
        {
            self.fire_pending = 0;
        }
        let control = self.kind.control();
        if control == FireControl::Spin && pad.hold(button::R1) && self.active {
            self.spinv = seek(self.spinv, SPIN_MAX, SPIN_UP * SECONDS_PER_FRAME);
        } else {
            self.spinv = seek(self.spinv, 0.0, SPIN_DOWN * SECONDS_PER_FRAME);
        }
        self.spin = (self.spin + self.spinv * SECONDS_PER_FRAME) % TURN;
        let delay = self.kind.fire_delay(self.spinv);
        let wants = match control {
            FireControl::Press => pad.pressed(button::R1),
            FireControl::Hold => pad.hold(button::R1) && !charging,
            FireControl::Spin => pad.hold(button::R1) && self.active,
        };
        if wants && delay - 30 < now - self.fire_time {
            self.fire_pending_time = now;
            if self.fire_pending == 0 {
                self.fire_pending += 1;
            }
        }
        let mut fire = None;
        if elapsed(self.fire_time, delay) && self.fire_pending > 0 && self.active {
            self.fire_pending -= 1;
            fire = Some(self.kind);
        }
        self.gun_time = now;
        fire
    }
}

impl Jak {
    /// The gun's part of a frame, after Jak has moved.
    pub(crate) fn gun_frame(
        &mut self,
        world: &mut dyn crate::collide::CollideWorld,
        actors: Option<&mut dyn crate::projectile::ActorWorld>,
    ) {
        match self.gun.equipped {
            // The inventory chooses: the gun in the slot comes out, none
            // goes away; the D-pad picks nothing.
            Some(equipped) => {
                if !self.dark && !self.state.is_board() {
                    match equipped {
                        Some(kind) => {
                            if self.gun.kind != kind {
                                self.gun.kind = kind;
                                self.gun.select_change_time = self.time;
                                self.gun.fire_pending = 0;
                                self.weapons.cancel_charges();
                                self.events.push(Event::GunSelect { kind });
                            }
                            if !self.gun.out {
                                self.gun.latch = true;
                            }
                        }
                        None if self.gun.out => {
                            self.gun.end_mode();
                            self.weapons.cancel_charges();
                        }
                        None => {}
                    }
                }
                if equipped.is_none() {
                    return;
                }
            }
            None => {
                if !self.dark
                    && !self.state.is_board()
                    && let Some(kind) = self.gun.select(self.time, &self.pad)
                {
                    self.weapons.cancel_charges();
                    self.events.push(Event::GunSelect { kind });
                }
            }
        }
        if self.dark || self.state.is_board() {
            self.gun.end_mode();
            self.weapons.cancel_charges();
            return;
        }
        if matches!(self.state, State::BoardGetOff) {
            return;
        }
        let gun_off = self.control.current.flags & crate::surface::flag::GUN_OFF != 0
            || matches!(
                self.state,
                State::PowerJakGetOn | State::LightJakGetOn { .. }
            );
        if gun_off {
            if self.gun.out {
                self.gun.end_mode();
            }
            self.weapons.cancel_charges();
            return;
        }
        let charging = self.weapons.charging();
        let fire = self.gun.check(
            self.time,
            &self.pad,
            &self.control,
            self.board.latch,
            charging,
            world,
        );
        if !self.gun.out {
            self.weapons.cancel_charges();
        }
        if let Some(kind) = fire {
            if self.gun.has_ammo(kind) && self.fire_gun(kind, world, actors) {
                self.gun.fire_time = self.time;
                self.gun.last_fire[kind as usize] = self.time;
                // (A shot as Light Jak cost light eco; Adrii 22/10: only the
                // freeze, the shield and the heal spend it.)
            } else {
                self.gun.fire_time = 0;
            }
        }
    }
}

#[cfg(test)]
mod view_muzzle_tests {
    use super::*;
    #[test]
    fn view_socket_aim_does_not_follow_body_steering() {
        let mut j=Jak::new(Vec3::new(13.,17.,29.),0.);
        j.control.quat=glam::Quat::from_rotation_y(std::f32::consts::FRAC_PI_2);
        j.gun.muzzle_local=Some(Vec3::new(-2.,4.,7.));
        j.gun.muzzle_basis=Some(glam::Quat::IDENTITY);
        j.gun.compute_pos(&j.control,None);
        assert!(j.gun.fire_point.distance(j.control.trans+Vec3::new(-2.,4.,7.))<0.0001);
        j.gun.muzzle_basis=None;
        j.gun.compute_pos(&j.control,None);
        assert!(j.gun.fire_point.distance(j.control.trans+j.control.quat*Vec3::new(-2.,4.,7.))<0.0001);
    }
    #[test]
    fn view_muzzle_cannot_spawn_beyond_a_near_wall() {
        use crate::collide::{CollideWorld,Tri,Pat};
        struct Wall;
        impl CollideWorld for Wall {fn fill(&mut self,_:Vec3,_:Vec3,out:&mut Vec<Tri>){
            out.extend([Tri{v:[Vec3::new(-10000.,-10000.,1000.),Vec3::new(-10000.,10000.,1000.),Vec3::new(10000.,-10000.,1000.)],pat:Pat::default()},
                Tri{v:[Vec3::new(10000.,-10000.,1000.),Vec3::new(-10000.,10000.,1000.),Vec3::new(10000.,10000.,1000.)],pat:Pat::default()}]);
        }}
        let mut j=Jak::new(Vec3::ZERO,0.);j.gun.muzzle_local=Some(Vec3::new(0.,100.,3000.));
        j.gun.muzzle_guard=Some(Vec3::Y*100.);j.gun.aim_point=Some(Vec3::new(500.,100.,10000.));
        j.gun.compute_pos(&j.control,Some(&mut Wall));
        assert!(j.gun.fire_point.z>0.&&j.gun.fire_point.z<1000.);
        assert!(j.gun.fire_dir.distance((j.gun.aim_point.unwrap()-j.gun.fire_point).normalize())<0.00001);
    }

}
