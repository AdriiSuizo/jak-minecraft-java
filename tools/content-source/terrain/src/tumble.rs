//! Something tumbling through the air (Adrii, 16/10): a mob, an NPC or an
//! enemy the Mass Inverter left without gravity takes a blow or a shot and
//! goes spinning, its turn real (an angular velocity, kept up while it
//! floats and slowly lost as it falls). Where it comes down it stays as it
//! landed (on its back, on its side, upright), settled flat on the ground,
//! lies there a while, and only then gets back up, slowly, never snapping
//! upright.
//!
//! It turns about the middle of its body, in the world's axes (y up), and
//! keeps time in game ticks (20 a second).
use glam::{Quat, Vec3};
/// The glam these turns are in, for callers on another one.
pub use glam as glam_t;

/// Ticks it settles onto the ground in, lies there, and takes to get up.
const SETTLE: u32 = 5;
const LIE: u32 = 80;
const RECOVER: u32 = 18;
/// Radians a tick a blow gives at full strength (some 7 a second), and how
/// much of its spin it keeps each tick floating and falling.
const SPIN: f32 = 0.35;
const KEEP_FLOATING: f32 = 0.995;
const KEEP_FALLING: f32 = 0.985;

#[derive(Clone, Copy, Debug, PartialEq)]
enum Phase {
    Spinning,
    Settling { from: Quat, to: Quat, pivot_from: f32, pivot_to: f32, t: u32 },
    Lying { t: u32 },
    Recovering { from: Quat, pivot_from: f32, t: u32 },
}

/// How it lies: upright, on its back (upside down) or on its side.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Rest {
    Upright,
    Upturned,
    Side,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Tumble {
    turn: Quat,
    turn_o: Quat,
    /// Radians a tick about each world axis.
    spin: Vec3,
    /// How high over its feet it turns about (its middle, or half its
    /// width lying on its side), now and last tick.
    pivot: f32,
    pivot_o: f32,
    phase: Phase,
    /// Its box: width and height.
    size: (f32, f32),
}

impl Tumble {
    pub fn new(width: f32, height: f32) -> Self {
        Self {
            turn: Quat::IDENTITY,
            turn_o: Quat::IDENTITY,
            spin: Vec3::ZERO,
            pivot: height * 0.5,
            pivot_o: height * 0.5,
            phase: Phase::Spinning,
            size: (width, height),
        }
    }

    /// A blow or a shot travelling along `dir` (world), `strength` 0 to 1:
    /// it tips over away from it, with a twist of `twist` (-1 to 1) about
    /// the up axis. Lying or getting up, it is knocked back into a spin.
    pub fn hit(&mut self, dir: Vec3, strength: f32, twist: f32) {
        let flat = Vec3::new(dir.x, 0.0, dir.z);
        let axis = if flat.length_squared() > 1e-6 {
            Vec3::Y.cross(flat.normalize())
        } else {
            // From above or below: over sideways.
            Vec3::X
        };
        self.spin += axis * SPIN * strength.clamp(0.0, 1.5) + Vec3::Y * SPIN * 0.6 * twist.clamp(-1.0, 1.0);
        if !matches!(self.phase, Phase::Spinning) {
            self.phase = Phase::Spinning;
        }
    }

    /// One game tick: `airborne` while it floats or falls. False once it
    /// has got back up (nothing left to do).
    pub fn tick(&mut self, airborne: bool, floating: bool) -> bool {
        self.turn_o = self.turn;
        self.pivot_o = self.pivot;
        let half_height = self.size.1 * 0.5;
        match self.phase {
            Phase::Spinning => {
                let step = Quat::from_scaled_axis(self.spin);
                self.turn = (step * self.turn).normalize();
                self.spin *= if floating { KEEP_FLOATING } else { KEEP_FALLING };
                self.pivot = half_height;
                if !airborne {
                    let (to, rest) = rest_of(self.turn);
                    let pivot_to = match rest {
                        Rest::Side => self.size.0 * 0.5,
                        Rest::Upright | Rest::Upturned => half_height,
                    };
                    self.spin = Vec3::ZERO;
                    self.phase = Phase::Settling { from: self.turn, to, pivot_from: self.pivot, pivot_to, t: 0 };
                }
            }
            Phase::Settling { from, to, pivot_from, pivot_to, t } => {
                let t = t + 1;
                let k = t as f32 / SETTLE as f32;
                self.turn = from.slerp(to, k.min(1.0));
                self.pivot = pivot_from + (pivot_to - pivot_from) * k.min(1.0);
                self.phase = if t >= SETTLE { Phase::Lying { t: 0 } } else { Phase::Settling { from, to, pivot_from, pivot_to, t } };
            }
            Phase::Lying { t } => {
                self.phase = if t + 1 >= LIE {
                    Phase::Recovering { from: self.turn, pivot_from: self.pivot, t: 0 }
                } else {
                    Phase::Lying { t: t + 1 }
                };
            }
            Phase::Recovering { from, pivot_from, t } => {
                let t = t + 1;
                // Eased: slow to start, slow to finish.
                let k = (t as f32 / RECOVER as f32).min(1.0);
                let k = k * k * (3.0 - 2.0 * k);
                self.turn = from.slerp(Quat::IDENTITY, k);
                self.pivot = pivot_from + (half_height - pivot_from) * k;
                if t >= RECOVER {
                    return false;
                }
                self.phase = Phase::Recovering { from, pivot_from, t };
            }
        }
        true
    }

    /// Its turn at a partial tick.
    pub fn rotation(&self, partial: f32) -> Quat {
        self.turn_o.slerp(self.turn, partial.clamp(0.0, 1.0))
    }

    /// Where its feet are drawn and how it is turned, for feet `feet`: it
    /// turns about its pivot, so its middle stays put while it spins.
    pub fn place(&self, feet: Vec3, partial: f32) -> (Vec3, Quat) {
        let turn = self.rotation(partial);
        let pivot = self.pivot_o + (self.pivot - self.pivot_o) * partial.clamp(0.0, 1.0);
        let middle = Vec3::Y * self.size.1 * 0.5;
        (feet + Vec3::Y * pivot - turn * middle, turn)
    }

    /// How it lies now, if it has come down.
    pub fn resting(&self) -> Option<Rest> {
        match self.phase {
            Phase::Lying { .. } => Some(rest_of(self.turn).1),
            _ => None,
        }
    }

    pub fn spinning(&self) -> bool {
        matches!(self.phase, Phase::Spinning) && self.spin.length_squared() > 1e-8
    }
}

/// The turn it comes to rest in from `turn`: its up laid flat on the
/// ground's plane (on its side), or straight up or down, whichever it is
/// nearest; the rest of its turn kept.
pub fn rest_of(turn: Quat) -> (Quat, Rest) {
    let up = turn * Vec3::Y;
    let (target, rest) = if up.y > 0.6 {
        (Vec3::Y, Rest::Upright)
    } else if up.y < -0.6 {
        (Vec3::NEG_Y, Rest::Upturned)
    } else {
        let flat = Vec3::new(up.x, 0.0, up.z);
        (flat.try_normalize().unwrap_or(Vec3::X), Rest::Side)
    };
    ((Quat::from_rotation_arc(up, target) * turn).normalize(), rest)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Floating, hit, then let go: it falls for `fall` ticks and lands.
    fn tumble_after(dir: Vec3, strength: f32, float: u32, fall: u32) -> Tumble {
        let mut t = Tumble::new(0.6, 1.8);
        t.hit(dir, strength, 0.3);
        for _ in 0..float {
            assert!(t.tick(true, true));
        }
        for _ in 0..fall {
            assert!(t.tick(true, false));
        }
        assert!(t.tick(false, false));
        t
    }

    #[test]
    fn a_blow_while_floating_sets_it_spinning() {
        let mut t = Tumble::new(0.6, 1.8);
        t.hit(Vec3::X, 1.0, 0.0);
        t.tick(true, true);
        assert!(t.spinning());
        let angle = t.rotation(1.0).angle_between(Quat::IDENTITY);
        assert!((angle - SPIN).abs() < 1e-3, "{angle}");
        // Ten ticks on, it has kept turning, by real angular velocity.
        for _ in 0..10 {
            t.tick(true, true);
        }
        assert!(t.rotation(1.0).angle_between(Quat::IDENTITY) > 1.0);
    }

    #[test]
    fn it_comes_to_rest_as_it_landed_not_upright() {
        // A quarter turn over: it lands on its side and stays so.
        let mut t = Tumble::new(0.6, 1.8);
        t.spin = Vec3::Z * (std::f32::consts::FRAC_PI_2 / 4.0);
        for _ in 0..4 {
            t.tick(true, false);
        }
        t.spin = Vec3::ZERO;
        t.tick(false, false);
        for _ in 0..SETTLE + 2 {
            assert!(t.tick(false, false));
        }
        assert_eq!(t.resting(), Some(Rest::Side));
        // Lying on its side, its middle sits half its width up.
        let (feet, turn) = t.place(Vec3::ZERO, 1.0);
        let middle = feet + turn * Vec3::Y * 0.9;
        assert!((middle.y - 0.3).abs() < 1e-3, "{middle:?}");
        // Still on its side well after (no snapping up).
        for _ in 0..LIE - 10 {
            t.tick(false, false);
        }
        assert_eq!(t.resting(), Some(Rest::Side));
    }

    #[test]
    fn upside_down_it_stays_on_its_back() {
        let mut t = Tumble::new(0.6, 1.8);
        t.spin = Vec3::X * (std::f32::consts::PI / 8.0);
        for _ in 0..8 {
            t.tick(true, false);
        }
        t.spin = Vec3::ZERO;
        t.tick(false, false);
        for _ in 0..SETTLE + 1 {
            t.tick(false, false);
        }
        assert_eq!(t.resting(), Some(Rest::Upturned));
    }

    #[test]
    fn it_gets_up_slowly_then_is_done() {
        let mut t = tumble_after(Vec3::Z, 1.0, 20, 10);
        let mut ticks = 0;
        let mut biggest_step = 0.0f32;
        while t.tick(false, false) {
            biggest_step = biggest_step.max(t.turn_o.angle_between(t.turn));
            ticks += 1;
            assert!(ticks < 1000);
        }
        assert!(ticks >= (SETTLE + LIE + RECOVER - 2) as usize);
        // Never more than a fifth of a turn in one tick getting up.
        assert!(biggest_step < 0.4, "{biggest_step}");
    }

    #[test]
    fn spinning_keeps_its_middle_in_place() {
        let mut t = Tumble::new(0.6, 1.8);
        t.hit(Vec3::new(1.0, 0.0, 1.0), 1.0, 0.0);
        for _ in 0..13 {
            t.tick(true, true);
            let (feet, turn) = t.place(Vec3::new(5.0, 2.0, 1.0), 1.0);
            let middle = feet + turn * Vec3::Y * 0.9;
            assert!(middle.distance(Vec3::new(5.0, 2.9, 1.0)) < 1e-4);
        }
    }
}
