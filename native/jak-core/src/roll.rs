//! The roll of a hovering vehicle: the part of `apply-gravity!` of the hvehicle
//! (levels/factory/car/hvehicle-physics.gc) that holds it level, run on the one axis zoomer.rs
//! was missing, with the rigid body's own numbers (`rigid-body-info`: mass, inertial tensor box,
//! angular damping; the handling's `roll-control-factor`, `player-weight`, `player-shift-x`;
//! the physics model's `roll-thruster-array`).
//!
//! Two things turn the vehicle about its forward axis here: the rider's weight, which is
//! shifted sideways by the steering (`player-shift-x`), and the vehicle's own impacts. The two roll
//! thrusters push back, each only on its own side and by the cube of how far the vehicle has rolled
//! plus a little of how fast (`0.075` of the roll rate). So the vehicle leans into a turn as far as
//! those thrusters can hold it, and the thrusters' push is also the size of the flame
//! `vehicle-method-78` of the bike draws at each.
use crate::particles::vehicle_lights::Specs;
use crate::math::METER;
use glam::Vec3;

const DT: f32 = 1.0 / 60.0;
/// A roll past this is the vehicle on its side: a safety the rigid body has no need of (its thrusters
/// hold it far from here), kept so that a bad impulse cannot spin a vehicle for ever.
const LIMIT: f32 = 1.3;

/// One roll thruster: where it sits relative to the centre of mass (metres, vehicle axes) and the
/// `rot` of its attach point, whose negative times the thrust is the force it applies.
#[derive(Clone, Copy, Debug)]
pub struct Thruster {
    pub position: Vec3,
    pub normal: Vec3,
}

#[derive(Clone, Debug)]
pub struct RollModel {
    pub mass: f32,
    /// m/s^2.
    pub gravity: f32,
    /// About the forward axis: `mass / 12 * (x^2 + y^2)` of the inertial tensor box, metres.
    pub inertia: f32,
    /// `angular-damping`, the share of the angular momentum kept each 60 Hz frame.
    pub damping: f32,
    pub control: f32,
    /// `player-weight` as a force in mass * m/s^2, `player-shift-x` in metres.
    pub weight: f32,
    pub shift: f32,
    pub thrusters: Vec<Thruster>,
}

impl RollModel {
    /// The model from a vehicle's constants, as read in [`Specs`]; none when they lack the rigid body.
    pub fn from_specs(spec: &Specs) -> Option<Self> {
        if spec.mass <= 0. || spec.gravity <= 0. || spec.roll_thrusters.len() < 2 || spec.inertial_box.min_element() <= 0. {
            return None;
        }
        let b = spec.inertial_box / METER;
        Some(Self {
            mass: spec.mass,
            gravity: spec.gravity / METER,
            inertia: spec.mass / 12. * (b.x * b.x + b.y * b.y),
            damping: spec.angular_damping,
            control: spec.roll_control,
            weight: spec.player_weight / METER,
            shift: spec.player_shift_x / METER,
            thrusters: spec
                .roll_thrusters
                .iter()
                .map(|&(position, normal)| Thruster { position: (position - spec.cm_offset) / METER, normal })
                .collect(),
        })
    }
}

/// The roll's state: angle (radians, positive with the left side up), rate, and what the thrusters
/// pushed this step.
#[derive(Clone, Debug, Default)]
pub struct Roll {
    pub angle: f32,
    pub rate: f32,
    /// Each thruster's `roll-thrust * force-scale * inv-mass / gravity` of the last step: the thrust
    /// `vehicle-draw-thruster` is given (the push as a share of the vehicle's weight).
    pub thrust: [f32; 2],
    /// Each thruster's `roll-thrust` (force, mass * units/s^2), which the bike's thrust sound reads.
    pub force: [f32; 2],
}

impl Roll {
    /// One 60 Hz step. `steering` is the vehicle's `controls steering`, `riding` its `riding` flag,
    /// `fluctuation` its `power-fluctuation-factor`, `random` `rand-vu`, `impulse` an angular impulse
    /// from an impact (mass * m^2/s about the forward axis).
    pub fn step(&mut self, model: &RollModel, steering: f32, riding: bool, fluctuation: f32, mut random: impl FnMut() -> f32, impulse: f32) {
        // `dir` is the world's up (the bikes and cars set no `roll-angle`): `dot(rvec, dir)` is sin(angle).
        let d = self.angle.sin();
        let along = d * d * d + 0.075 * self.rate;
        let weight = model.mass * model.gravity;
        let mut torque = 0.0;
        self.thrust = [0.; 2];
        self.force = [0.; 2];
        for (i, thruster) in model.thrusters.iter().take(2).enumerate() {
            // `(logior (logand bits sign) 1.0)`: the sign of the thruster's x as +-1.
            let side = if thruster.position.x.is_sign_negative() { -1.0 } else { 1.0 };
            let share = (-side * along).clamp(0., 1.);
            if share > 0. {
                let push = (share + (-fluctuation + 2. * random() * fluctuation)) * model.control * weight;
                self.force[i] += push.max(0.) * METER;
                self.thrust[i] += push.max(0.) / weight;
                // force = -push * rot; its moment about the forward axis, r x F (z).
                let (fx, fy) = (-push * thruster.normal.x, -push * thruster.normal.y);
                torque += thruster.position.x * fy - thruster.position.y * fx;
            }
        }
        if riding {
            // The rider's weight, a point shifted by the steering, acts straight down in the world.
            torque -= model.weight * model.shift * steering * self.angle.cos();
        }
        let mut momentum = self.rate * model.inertia + torque * DT + impulse;
        momentum *= model.damping;
        self.rate = momentum / model.inertia;
        self.angle += self.rate * DT;
        if self.angle.abs() > LIMIT {
            self.angle = LIMIT.copysign(self.angle);
            self.rate = 0.;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The h-bike-a constants (bike.gc): mass 2, 40 m/s^2, a 3 x 4 x 6 m box, damping 0.995, the roll
    /// thrusters 1.7 m to each side of the middle, the rider 40 and 0.6 m.
    fn bike() -> RollModel {
        RollModel {
            mass: 2.,
            gravity: 40.,
            inertia: 2. / 12. * (9. + 16.),
            damping: 0.995,
            control: 1.,
            weight: 40.,
            shift: 0.6,
            thrusters: vec![
                Thruster { position: Vec3::new(6963.2, 819.2, 0.) / METER, normal: Vec3::new(0.3, -0.6, 0.) },
                Thruster { position: Vec3::new(-6963.2, 819.2, 0.) / METER, normal: Vec3::new(-0.3, -0.6, 0.) },
            ],
        }
    }

    fn settle(steering: f32, frames: usize) -> Roll {
        let model = bike();
        let mut roll = Roll::default();
        for _ in 0..frames {
            roll.step(&model, steering, true, 0., || 0.5, 0.);
        }
        roll
    }

    #[test]
    fn a_level_bike_with_a_still_rider_stays_level_and_its_thrusters_are_quiet() {
        let roll = settle(0., 600);
        assert_eq!(roll.angle, 0.);
        assert_eq!(roll.thrust, [0.; 2]);
    }

    #[test]
    fn steering_leans_the_bike_into_the_turn_as_far_as_the_cubed_thrusters_hold_it() {
        // Left (positive) steering shifts the rider's weight to the left: the left side goes down.
        let left = settle(1., 1200);
        assert!(left.angle < 0., "left side down, got {}", left.angle);
        // 24 cos(a) = 1.08 * 80 * sin(a)^3 -> sin(a) near 0.6: 37 degrees, as the original's numbers give.
        assert!((left.angle.to_degrees() + 37.).abs() < 3., "rolled to {} degrees", left.angle.to_degrees());
        assert!(left.rate.abs() < 1e-3, "at rest, {}", left.rate);
        // Only the thruster on the low side pushes, and as hard as the steady lean needs: 0.28 of the weight.
        assert!(left.thrust[0] > 0.2 && left.thrust[0] < 0.4, "{:?}", left.thrust);
        assert_eq!(left.thrust[1], 0.);
        let right = settle(-1., 1200);
        assert!((right.angle + left.angle).abs() < 1e-3, "right is the mirror of left");
        assert!(right.thrust[1] > 0.2 && right.thrust[0] == 0.);
        // Half the steering leans it less, but not by half (the cube root).
        let half = settle(0.5, 1200);
        assert!(half.angle.abs() < left.angle.abs() && half.angle.abs() > 0.5 * left.angle.abs());
    }

    #[test]
    fn an_impact_rolls_the_bike_and_the_thrusters_bring_it_back() {
        let model = bike();
        let mut roll = Roll::default();
        roll.step(&model, 0., false, 0., || 0.5, 6.);
        assert!(roll.rate > 1., "a side blow spins it up: {}", roll.rate);
        let mut fired = false;
        let mut peak: f32 = 0.;
        for _ in 0..1200 {
            roll.step(&model, 0., false, 0., || 0.5, 0.);
            fired |= roll.thrust.iter().any(|&t| t > 0.);
            peak = peak.max(roll.angle.abs());
        }
        assert!(fired && peak > 0.05);
        assert!(roll.angle.abs() < 0.1 && roll.rate.abs() < 0.05, "back near level: {} {}", roll.angle, roll.rate);
    }

    #[test]
    fn the_power_fluctuation_shakes_the_push_about_its_mean() {
        let model = bike();
        let mut pushes = Vec::new();
        for r in [0., 0.5, 1.] {
            let mut roll = Roll { angle: -1.0, ..Default::default() };
            roll.step(&model, 0., false, 0.1, || r, 0.);
            pushes.push(roll.thrust[0]);
        }
        assert!(pushes[0] < pushes[1] && pushes[1] < pushes[2], "{pushes:?}");
        assert!(((pushes[2] - pushes[0]) - 0.2).abs() < 1e-4, "twice the fluctuation across the random range");
    }
}
