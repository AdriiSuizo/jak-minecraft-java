use crate::roll::{Roll, RollModel};
use crate::wheeled::Ground;
use glam::{Quat, Vec3};

const DT: f32 = 1.0 / 60.0;
const METRES_PER_BLOCK: f32 = 6400.0 / crate::METER;
const TOP_SPEED: f32 = 33.5 * 36.0 / 25.0;
const HIGH_LANE: f32 = 13.0;

#[derive(Clone, Copy, Debug)]
pub enum Model {
    BikeA,
    BikeB,
    BikeC,
    CarA,
    CarB,
    CarC,
}

impl Model {
    pub fn name(self) -> &'static str {
        match self {
            Self::BikeA => "bikea",
            Self::BikeB => "bikeb",
            Self::BikeC => "bikec",
            Self::CarA => "cara",
            Self::CarB => "carb",
            Self::CarC => "carc",
        }
    }

    pub fn car(self) -> bool {
        matches!(self, Self::CarA | Self::CarB | Self::CarC)
    }

    pub fn seat(self) -> Vec3 {
        (match self {
            Self::BikeA => Vec3::ZERO,
            Self::BikeB => Vec3::new(0.0, 757.76, 1175.552),
            Self::BikeC => Vec3::new(0.0, 802.816, 1318.912),
            Self::CarA => Vec3::new(4079.616, 819.2, -311.296),
            Self::CarB => Vec3::new(4218.88, 999.424, -172.032),
            Self::CarC => Vec3::new(0.0, 761.856, 1368.064),
        }) / 4096.0
    }

    pub fn base_hover(self) -> f32 {
        // Preserve the prior mashup's half-block increase above GOAL clearance.
        (if self.car() { 2.5 } else { 2.3 }) + 0.5 * METRES_PER_BLOCK
    }

    pub fn health(self) -> f32 {
        if self.car() { 24.0 } else { 15.0 }
    }

    fn half(self) -> Vec3 {
        if self.car() {
            Vec3::new(1.4, 0.7, 2.4)
        } else {
            Vec3::new(0.8, 0.6, 1.5)
        }
    }
}

#[derive(Clone, Copy, Default)]
pub struct Controls {
    pub steer: f32,
    pub lean: f32,
    pub throttle: f32,
    pub brake: f32,
    pub handbrake: bool,
    pub lane: bool,
    pub hop: bool,
}

#[derive(Clone, Copy, Debug)]
pub enum Event {
    Lane(bool),
    Hop,
    Impact(f32),
}

pub struct Zoomer {
    pub model: Model,
    /// The rigid body's roll about the forward axis (hvehicle-physics.gc `apply-gravity!`'s roll thrusters, the
    /// rider's weight shifted by the steering) and the constants it needs; `bank` is its angle. Without the model
    /// (a vehicle whose constants were not read) the bank just eases to the steering.
    pub roll: Roll,
    pub roll_model: Option<RollModel>,
    /// `vehicle-flag riding`: somebody is aboard, so their weight is on the body.
    pub riding: bool,
    /// `hit-points`, 1 whole to 0 destroyed: the power fluctuation of the roll thrusters follows it.
    pub hit_points: f32,
    /// `scrape-sound-envelope`: 1 as the hull slides along something at 3 m/s or more, easing to 0 at 2 a second.
    pub scrape: f32,
    /// `impact-pos`: where the hull last hit something (metres).
    pub impact_pos: Vec3,
    pub position: Vec3,
    pub velocity: Vec3,
    pub yaw: f32,
    pub pitch: f32,
    pub bank: f32,
    pub steer: f32,
    pub lean: f32,
    pub throttle: f32,
    pub brake: f32,
    pub thrust: f32,
    pub high: bool,
    pub grounded: bool,
    pub climb: f32,
    /// How much the velocity changed in the last step, per second (m/s^2): `clear-impulse-force-flag!`'s lin-acceleration.
    pub accel: Vec3,
    /// Town traffic on its route (Rust zoomers.rs `route`): it never backs up when it brakes.
    pub routed: bool,
    yaw_rate: f32,
    collision_yaw: f32,
    incoming_impact: f32,
    /// Frames spent trying to reach the high lane. A roof can abort the ascent.
    rising_frames: u8,
    tilt: [f32; 2],
    tilt_rate: [f32; 2],
    hop_charge: f32,
    hopping: bool,
    /// `rand-vu` of the roll thrusters.
    rng: u32,
}

fn seek(value: f32, target: f32, delta: f32) -> f32 {
    value + (target - value).clamp(-delta, delta)
}

impl Zoomer {
    pub fn new(model: Model, yaw: f32) -> Self {
        Self {
            model,
            roll: Roll::default(),
            roll_model: None,
            riding: false,
            hit_points: 1.,
            scrape: 0.,
            impact_pos: Vec3::ZERO,
            position: Vec3::ZERO,
            velocity: Vec3::ZERO,
            yaw,
            pitch: 0.,
            bank: 0.,
            steer: 0.,
            lean: 0.,
            throttle: 0.,
            brake: 0.,
            thrust: 0.,
            high: false,
            grounded: false,
            climb: 0.,
            accel: Vec3::ZERO,
            routed: false,
            yaw_rate: 0.,
            collision_yaw: 0.,
            incoming_impact: 0.,
            rising_frames: 0,
            tilt: [0.; 2],
            tilt_rate: [0.; 2],
            hop_charge: 0.,
            hopping: false,
            rng: 0x9E37_79B9,
        }
    }

    /// The thrust each roll thruster is firing with, the share of the vehicle's weight (`roll-thrust` as the flames take it).
    pub fn roll_thrust(&self) -> [f32; 2] {
        self.roll.thrust
    }

    /// The seed of the roll thrusters' random power fluctuation.
    pub fn seed(&mut self, seed: u64) {
        self.rng = (seed ^ (seed >> 32)) as u32 | 1;
    }

    /// `rand-vu`.
    fn random(rng: &mut u32) -> f32 {
        *rng ^= *rng << 13;
        *rng ^= *rng >> 17;
        *rng ^= *rng << 5;
        (*rng >> 8) as f32 / 16_777_216.0
    }

    pub fn heading(&self) -> Vec3 {
        Quat::from_rotation_y(self.yaw) * Vec3::Z
    }

    pub fn orientation(&self) -> Quat {
        Quat::from_rotation_y(self.yaw)
            * Quat::from_rotation_x(self.pitch + self.tilt[0])
            * Quat::from_rotation_z(self.bank + self.tilt[1])
    }

    /// The original waiting state sends the vehicle back to the low lane.
    pub fn reset_lane(&mut self) {
        self.high = false;
        self.rising_frames = 0;
    }

    pub fn step(&mut self, input: Controls, terrain: &mut dyn Ground, events: &mut Vec<Event>) {
        let car = self.model.car();
        let impact = std::mem::take(&mut self.incoming_impact);
        if impact > 0. {
            events.push(Event::Impact(impact));
        }
        let start_velocity = self.velocity;
        let (accel, max_brake, max_gain, intake, response) = if car {
            (39., 54., 3.5, 1., 20.)
        } else {
            (50., 84., 4., 1.5, 60.)
        };
        self.steer = seek(self.steer, input.steer, 8. * DT);
        self.lean = seek(self.lean, input.lean, 8. * DT);
        self.throttle = if input.handbrake { 0. } else { seek(self.throttle, input.throttle, 4. * DT) };
        self.brake += (input.brake - self.brake) * (8. * DT);
        if self.throttle > 0.0 && self.brake > 0.0 {
            self.brake = self.brake.min(0.25);
        }
        if input.lane {
            self.high = !self.high;
            self.rising_frames = u8::from(self.high);
            events.push(Event::Lane(self.high));
        }

        let ahead = self.heading();
        let left = Vec3::Y.cross(ahead);
        let forward_speed = self.velocity.dot(ahead);
        let lateral_speed = self.velocity.dot(left);
        let speed = self.velocity.with_y(0.0).length();
        let mut rise = self.velocity.y;
        let target = if self.high {
            HIGH_LANE
        } else {
            self.model.base_hover()
        };
        let below = envelope(terrain, self.position, ahead, target);
        if self.rising_frames > 0 {
            if below.is_some_and(|height| (height - target).abs() < 2.) && rise.abs() < 2. {
                self.rising_frames = 0;
            } else if self.rising_frames >= 120 {
                self.reset_lane();
            } else {
                self.rising_frames += 1;
            }
        }
        let target = if self.high { HIGH_LANE } else { self.model.base_hover() };
        let mut hop = 0.0;
        if !car && !self.high {
            if input.hop && self.hop_charge > 0.0 {
                if self.hop_charge >= 0.099 {
                    self.hopping = true;
                    events.push(Event::Hop);
                }
                self.hop_charge = (self.hop_charge - DT).max(0.0);
                hop = 150.0;
            } else if !input.hop && below.is_some_and(|h| h < target + 0.5) {
                self.hop_charge = (self.hop_charge + 0.5 * DT).min(0.1);
            }
        }
        if self.hopping && rise < 0.0 && below.is_some_and(|h| h < target + 0.05) {
            self.hopping = false;
        }
        let lift = match below {
            Some(h) if !self.hopping && h < target + 1.5 => {
                30. * (target - h) - 2. * 30_f32.sqrt() * rise
            }
            Some(h) if self.hopping && h < target + 0.05 => 30. * (target - h) - 8. * rise,
            _ => -40.0,
        };
        rise += (lift + hop).clamp(-80., 160.) * DT;
        self.grounded = below.is_some_and(|h| h < target + 1.5);
        let gain = max_gain * 15.0 / (15.0 + speed);
        let steer = if forward_speed < -0.5 {
            -self.steer
        } else {
            self.steer
        };
        let turn = if self.grounded { steer * gain } else { 0.0 };
        self.yaw_rate += (turn - self.yaw_rate) * (DT / 0.1);
        let old_yaw=self.yaw;
        self.yaw += (self.yaw_rate + self.collision_yaw) * DT;
        self.collision_yaw *= (-7. * DT).exp();
        if !self.routed && self.yaw!=old_yaw && crate::wheeled::hull_hit(terrain,self.position,Vec3::ZERO,self.model.half(),self.heading()).is_some()
            && crate::wheeled::hull_hit(terrain,self.position,Vec3::ZERO,self.model.half(),ahead).is_none(){
            self.yaw=old_yaw;self.yaw_rate=0.0;
        }

        // Rust zoomers.rs: `self.route.is_none() &&`, the traffic only stops.
        let reversing = !self.routed && !input.handbrake && self.brake > 0.05 && forward_speed < 2.0 && self.throttle <= 0.0;
        let throttle = if reversing {
            -0.5 * self.brake
        } else {
            self.throttle
        };
        let intake_limit = 0.83333 * (0.5 + (speed * intake / TOP_SPEED).clamp(0.0, 1.0));
        let thrust = throttle.min(intake_limit);
        if input.handbrake {
            self.thrust = 0.;
        } else if thrust > self.thrust {
            self.thrust += (thrust - self.thrust) * (response * DT).min(1.);
        } else {
            self.thrust = seek(self.thrust, thrust, DT);
        }
        let drag = forward_speed / TOP_SPEED;
        let mut next_speed = forward_speed + (self.thrust * accel - accel * drag * drag.abs()) * DT;
        let braking = if reversing {
            0.0
        } else {
            self.brake * max_brake
        };
        next_speed = seek(
            next_speed,
            0.0,
            (braking + if input.handbrake { 86. } else { 0. }) * DT,
        );
        if reversing {
            next_speed = next_speed.max(-6.0);
        }
        let grip = if self.grounded {
            12.0 * (1.0 - 0.75 * self.thrust.clamp(0.0, 1.0))
        } else {
            0.6
        };
        let new_ahead = self.heading();
        let new_left = Vec3::Y.cross(new_ahead);
        let slide = if input.handbrake { seek(lateral_speed, 0., 86. * DT) } else { lateral_speed * (-grip * DT).exp() };
        let horizontal = new_ahead * next_speed + new_left * slide;
        self.velocity = horizontal.clamp_length_max(TOP_SPEED * 1.05) + Vec3::Y * rise;
        self.move_hull(terrain, new_ahead, events);
        self.climb *= (-10. * DT).exp();
        self.accel = (self.velocity - start_velocity) / DT;

        let reach = target + 3.;
        let center = terrain.below(self.position, reach).unwrap_or(target);
        let half = self.model.half();
        let height = |g: &mut dyn Ground, p| g.below(p, reach).unwrap_or(center);
        let front = height(terrain, self.position + new_ahead * half.z);
        let back = height(terrain, self.position - new_ahead * half.z);
        let left = height(terrain, self.position + new_left * half.x);
        let right = height(terrain, self.position - new_left * half.x);
        let tilt = [
            (((front - back) / (2.0 * half.z)).atan() * 0.35).clamp(-0.18, 0.18),
            ((right - left) / (2.0 * half.x)).atan().clamp(-0.5, 0.5),
        ];
        if self.grounded && rise < -6.0 {
            self.tilt_rate[0] += rise.abs().min(25.0) * 0.08;
        }
        for (i, desired) in tilt.into_iter().enumerate() {
            let desired = if self.grounded {
                desired
            } else {
                desired * 0.3
            };
            self.tilt_rate[i] += (36. * (desired - self.tilt[i]) - 12. * self.tilt_rate[i]) * DT;
            self.tilt[i] = (self.tilt[i] + self.tilt_rate[i] * DT).clamp(-0.7, 0.7);
        }
        match &self.roll_model {
            Some(model) => {
                // The roll the thrusters hold: the bank is its angle (set from outside it is taken as it stands).
                self.roll.angle = self.bank;
                let mut rng = self.rng;
                let fluctuation = crate::particles::vehicle_effects::fluctuation(self.hit_points);
                self.roll.step(model, self.steer, self.riding, fluctuation, || Self::random(&mut rng), 0.0);
                self.rng = rng;
                self.bank = self.roll.angle;
            }
            None => {
                let sway = speed / (speed + 5.0);
                self.bank += (-20_f32.to_radians() * self.steer * sway - self.bank + self.roll.rate / 6.0) * (6.0 * DT);
                self.roll.rate *= (-7. * DT).exp();
            }
        }
        // vehicle.gc: the scrape sound's envelope eases off 2 a second after the frame's impacts set it.
        self.scrape = seek(self.scrape, 0., 2. * DT);
        self.pitch +=
            (20_f32.to_radians() * self.lean - 0.06 * self.thrust - self.pitch) * (4.0 * DT);
    }

    pub fn receive_impact(&mut self, impulse: Vec3, offset: Vec3) {
        self.velocity += impulse;
        self.rock(impulse, offset);
        self.incoming_impact = self.incoming_impact.max(impulse.length());
    }

    fn rock(&mut self, velocity_change: Vec3, offset: Vec3) {
        let heading = self.heading();
        let half = self.model.half();
        // Apply the point impulse about the hull, using the original roll inertia where available.
        // The contact plane sits below the centre of the hover's roll mass.
        let offset = offset - Vec3::Y * (half.y * 0.45);
        let torque = offset.cross(velocity_change);
        let roll_inertia = self.roll_model.as_ref().map_or((half.x * half.x + half.y * half.y) / 3.,
            |model| model.inertia / model.mass);
        self.roll.rate += (torque.dot(heading) / roll_inertia.max(0.15)).clamp(-7., 7.);
        self.tilt_rate[0] += (torque.dot(Vec3::Y.cross(heading)) / (half.z * half.z).max(1.)).clamp(-4., 4.);
        self.collision_yaw += (torque.y * 3. / (half.x * half.x + half.z * half.z)).clamp(-4., 4.);
    }

    fn move_hull(&mut self, terrain: &mut dyn Ground, heading: Vec3, events: &mut Vec<Event>) {
        let step = self.velocity.with_y(0.0) * DT;
        let half = self.model.half();
        let hit = |g: &mut dyn Ground, pos, motion| {
            crate::wheeled::hull_hit(g, pos, motion, half, heading)
        };
        if let Some((fraction, normal)) = hit(terrain, self.position, step) {
            let can_step = terrain.can_step_on_contact();
            let climb = (can_step && self.grounded && normal.y.abs() < 0.3)
                .then(|| {
                    [0.6, 1.2, 1.8, 2.4, 3.0]
                        .into_iter()
                        .map(|v| v * METRES_PER_BLOCK)
                        .find(|&rise| {
                            let up = Vec3::Y * rise;
                            hit(terrain, self.position, up).is_none_or(|(_, n)| n.y >= -0.1)
                                && hit(terrain, self.position + up, step).is_none()
                        })
                })
                .flatten();
            if let Some(rise) = climb {
                self.position += Vec3::Y * rise + step;
                self.climb += rise;
                self.velocity.y = self.velocity.y.max(0.0);
            } else {
                self.position += step * fraction;
                // vehicle.gc on-impact: sliding along what it hit faster than 12288 units (3 m) a second sets the
                // scrape; the contact point is the hull's side facing the thing.
                let sliding = self.velocity - normal * self.velocity.dot(normal);
                if sliding.length_squared() > 9. {
                    self.scrape = 1.;
                    let up = Vec3::Y;
                    let left = up.cross(heading);
                    // stand-in: the original takes the rigid body's contact point; the hull is a box, whose support
                    // point toward the thing is the nearest to it.
                    let reach = normal.dot(left).abs() * half.x + normal.dot(up).abs() * half.y + normal.dot(heading).abs() * half.z;
                    self.impact_pos = self.position - normal * reach;
                }
                let normal_speed = self.velocity.dot(normal);
                if normal_speed < 0.0 {
                    // Step probes may leave the overhead vehicle as the host's
                    // last contact even though this committed hit was a wall.
                    let (change, offset) = (!can_step).then(|| terrain.exchange_impulse(self.velocity, normal, self.position)).flatten()
                        .unwrap_or_else(|| {
                            let right = Vec3::Y.cross(heading);
                            let side = |projection: f32| if projection.abs() < 0.05 { 0. } else { projection.signum() };
                            (-normal_speed * 1.4, -right * half.x * side(normal.dot(right))
                                - heading * half.z * side(normal.dot(heading)))
                        });
                    self.velocity += normal * change;
                    self.rock(normal * change, offset);
                    if change > 0. { events.push(Event::Impact(change)); }
                }
            }
        } else {
            self.position += step;
        }
        // Unlike the old renderer's downward-only floor test, use the same
        // complete hull vertically so a lane switch cannot clip through a roof.
        let vertical = Vec3::Y * self.velocity.y * DT;
        if let Some((fraction, normal)) = hit(terrain, self.position, vertical) {
            self.position += vertical * fraction;
            let speed = self.velocity.dot(normal);
            if speed < 0.0 {
                let (change, offset) = terrain.exchange_impulse(self.velocity, normal, self.position)
                    .unwrap_or((-speed, Vec3::ZERO));
                self.velocity += normal * change;
                if offset != Vec3::ZERO { self.rock(normal * change, offset); }
            }
        } else {
            self.position += vertical;
        }
    }
}

fn envelope(terrain: &mut dyn Ground, position: Vec3, heading: Vec3, radius: f32) -> Option<f32> {
    let reach = radius + 10.0;
    let left = Vec3::Y.cross(heading);
    let mut closest = terrain.below(position, reach);
    for dir in [heading, -heading, left, -left] {
        for share in [0.25, 0.5, 0.75] {
            let d = share * radius;
            if let Some(h) = terrain.below(position + dir * d, reach) {
                let effective = h + radius - (radius * radius - d * d).sqrt();
                closest = Some(closest.map_or(effective, |old| old.min(effective)));
            }
        }
    }
    closest
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Flat;
    impl Ground for Flat {
        fn below(&mut self, at: Vec3, reach: f32) -> Option<f32> {
            (0.0..=reach).contains(&at.y).then_some(at.y)
        }
        fn sweep(&mut self, at: Vec3, step: Vec3, half: Vec3) -> Option<(f32, Vec3)> {
            let floor = at.y - half.y;
            (step.y < 0.0 && floor >= 0.0 && floor + step.y < 0.0)
                .then_some((-floor / step.y, Vec3::Y))
        }
    }

    #[test]
    fn a_side_impact_moves_the_parked_zoomer_and_rocks_the_hull() {
        let mut zoomer = Zoomer::new(Model::BikeA, 0.);
        zoomer.roll_model = Some(bike_roll());
        zoomer.position.y = zoomer.model.base_hover();
        zoomer.receive_impact(Vec3::X * 8., Vec3::new(-0.8, 0., 1.));
        let mut events = Vec::new();
        for _ in 0..10 {
            zoomer.step(Controls::default(), &mut Flat, &mut events);
        }
        assert!(zoomer.position.x > 0.2);
        assert!(zoomer.bank.abs() > 0.02 && zoomer.yaw.abs() > 0.02);
        assert!(events.iter().any(|e| matches!(e, Event::Impact(speed) if *speed == 8.)));
    }

    #[test]
    fn a_failed_zoomer_climb_does_not_push_the_vehicle_above_the_wall() {
        let mut zoomer = Zoomer::new(Model::BikeA, 0.);
        zoomer.grounded = true;
        zoomer.velocity = Vec3::X * 12.;
        let mut ground = crate::wheeled::collision_tests::ClimbContacts::new(false);
        zoomer.move_hull(&mut ground, Vec3::Z, &mut Vec::new());
        assert!(ground.headroom_probes > 0);
        assert_eq!(ground.exchanged, 0, "a rejected step is not a collision with the overhead vehicle");
        assert!(zoomer.velocity.x <= 0., "the wall must still stop forward movement");
    }

    #[test]
    fn a_zoomer_still_exchanges_impulse_with_the_vehicle_it_actually_hits() {
        let mut zoomer = Zoomer::new(Model::BikeA, 0.);
        zoomer.grounded = true;
        zoomer.velocity = Vec3::X * 12.;
        let mut ground = crate::wheeled::collision_tests::ClimbContacts::new(true);
        zoomer.move_hull(&mut ground, Vec3::Z, &mut Vec::new());
        assert_eq!(ground.headroom_probes, 0);
        assert_eq!(ground.exchanged, 1);
        assert_eq!(zoomer.velocity.x, 6.);
    }

    #[test]
    fn a_blocked_high_lane_returns_to_low_after_two_seconds() {
        struct UnderRoof;
        impl Ground for UnderRoof {
            fn below(&mut self, at: Vec3, reach: f32) -> Option<f32> {
                Flat.below(at, reach)
            }
            fn sweep(&mut self, at: Vec3, step: Vec3, half: Vec3) -> Option<(f32, Vec3)> {
                let clearance = 4.0 - at.y - half.y;
                if step.y > 0. && clearance >= 0. && step.y > clearance {
                    return Some((clearance / step.y, Vec3::NEG_Y));
                }
                Flat.sweep(at, step, half)
            }
        }
        let mut zoomer = Zoomer::new(Model::CarA, 0.);
        zoomer.position.y = zoomer.model.base_hover();
        let mut events = Vec::new();
        zoomer.step(Controls { lane: true, ..Default::default() }, &mut UnderRoof, &mut events);
        assert!(zoomer.high);
        for _ in 0..180 {
            zoomer.step(Controls::default(), &mut UnderRoof, &mut events);
        }
        assert!(!zoomer.high);
        assert!((zoomer.position.y - zoomer.model.base_hover()).abs() < 0.15);
    }

    /// The floor, and a wall facing -x at x = 3 that stands while `wall` is set.
    struct Walled {
        wall: bool,
    }
    impl Ground for Walled {
        fn below(&mut self, at: Vec3, reach: f32) -> Option<f32> {
            Flat.below(at, reach)
        }
        fn sweep(&mut self, at: Vec3, step: Vec3, half: Vec3) -> Option<(f32, Vec3)> {
            let right = at.x + half.x;
            if self.wall && step.x > 0.0 && right <= 3.0 && right + step.x > 3.0 {
                return Some(((3.0 - right) / step.x, Vec3::NEG_X));
            }
            Flat.sweep(at, step, half)
        }
    }

    /// The h-bike-a's rigid body (bike.gc), as `RollModel::from_specs` makes it.
    fn bike_roll() -> RollModel {
        use crate::roll::Thruster;
        RollModel {
            mass: 2.,
            gravity: 40.,
            inertia: 2. / 12. * (9. + 16.),
            damping: 0.995,
            control: 1.,
            weight: 40.,
            shift: 0.6,
            thrusters: vec![
                Thruster { position: Vec3::new(1.7, 0.2, 0.), normal: Vec3::new(0.3, -0.6, 0.) },
                Thruster { position: Vec3::new(-1.7, 0.2, 0.), normal: Vec3::new(-0.3, -0.6, 0.) },
            ],
        }
    }

    #[test]
    fn a_rider_leans_the_bike_into_the_turn_and_an_empty_bike_stays_level() {
        let drive = |riding: bool, steer: f32| {
            let mut z = Zoomer::new(Model::BikeA, 0.);
            z.roll_model = Some(bike_roll());
            z.riding = riding;
            z.position = Vec3::Y * z.model.base_hover();
            let mut events = Vec::new();
            for _ in 0..600 {
                z.step(Controls { steer, throttle: 0.5, ..Default::default() }, &mut Flat, &mut events);
            }
            z
        };
        let left = drive(true, 1.);
        // A left turn puts the rider's weight on the left: that side dips, about 37 degrees (the roll thrusters'
        // cubic law against 0.6 m of a 40 weight), and only the low side's thruster fires.
        assert!(left.bank < 0. && (left.bank.to_degrees() + 37.).abs() < 4., "{}", left.bank.to_degrees());
        assert!(left.roll_thrust()[0] > 0.15 && left.roll_thrust()[1] == 0., "{:?}", left.roll_thrust());
        let right = drive(true, -1.);
        assert!((right.bank + left.bank).abs() < 0.02);
        assert!(right.roll_thrust()[1] > 0.15 && right.roll_thrust()[0] == 0.);
        let empty = drive(false, 1.);
        assert!(empty.bank.abs() < 1e-4 && empty.roll_thrust() == [0.; 2], "{} {:?}", empty.bank, empty.roll_thrust());
    }

    #[test]
    fn flipping_the_steering_back_and_forth_keeps_the_roll_bounded_and_settling() {
        let mut z = Zoomer::new(Model::BikeA, 0.);
        z.roll_model = Some(bike_roll());
        z.riding = true;
        z.position = Vec3::Y * z.model.base_hover();
        let mut events = Vec::new();
        let mut widest: f32 = 0.;
        // A second left, a second right, over and over, at speed.
        for frame in 0..1800 {
            let steer = if (frame / 60) % 2 == 0 { 1. } else { -1. };
            z.step(Controls { steer, throttle: 1., ..Default::default() }, &mut Flat, &mut events);
            assert!(z.bank.is_finite());
            widest = widest.max(z.bank.abs());
        }
        // The thrusters are lightly damped (0.075 of the roll rate, 0.995 a frame): a flip from full lock to full lock overshoots the 37 degrees
        // of the lean, and never goes near the safety stop.
        assert!(widest > 0.6 && widest < 1.2, "swung to {} degrees", widest.to_degrees());
        // Let go: it comes back to level (the cubic law is weak near it, so the last degree takes its time) and the thrusters nearly go quiet.
        for _ in 0..600 {
            z.step(Controls { throttle: 1., ..Default::default() }, &mut Flat, &mut events);
        }
        assert!(z.bank.abs() < 0.03 && z.roll_thrust().iter().all(|t| *t < 0.01), "{} {:?}", z.bank, z.roll_thrust());
    }

    #[test]
    fn sliding_along_a_wall_sets_the_scrape_at_its_contact_and_it_dies_away() {
        let mut z = Zoomer::new(Model::BikeA, 0.);
        let mut terrain = Walled { wall: true };
        z.position = Vec3::new(1.8, z.model.base_hover(), 0.);
        z.velocity = Vec3::new(8., 0., 20.);
        let mut events = Vec::new();
        let mut loud = false;
        for _ in 0..40 {
            z.step(Controls { throttle: 0.5, ..Default::default() }, &mut terrain, &mut events);
            loud |= z.scrape >= 0.75;
        }
        assert!(loud, "scraped at 20 m/s along the wall");
        // On the hull's side that faces the wall: the wall at x = 3, the bike 0.8 m wide.
        assert!((z.impact_pos.x - 3.0).abs() < 0.2, "{:?}", z.impact_pos);
        // Away from it the envelope eases off 2 a second.
        terrain.wall = false;
        z.velocity.x = 0.;
        for _ in 0..30 {
            z.step(Controls::default(), &mut terrain, &mut events);
        }
        assert!(z.scrape < 0.05, "{}", z.scrape);
        // Straight into a wall, with no sliding to speak of, does not scrape.
        let mut head_on = Zoomer::new(Model::BikeA, 0.);
        head_on.position = Vec3::new(1.8, head_on.model.base_hover(), 0.);
        head_on.velocity = Vec3::new(8., 0., 0.);
        head_on.yaw = std::f32::consts::FRAC_PI_2;
        let mut wall = Walled { wall: true };
        for _ in 0..20 {
            head_on.step(Controls::default(), &mut wall, &mut events);
        }
        assert_eq!(head_on.scrape, 0.);
    }

    #[test]
    fn zoomer_maintains_both_lanes_and_reverses_after_braking() {
        let mut z = Zoomer::new(Model::BikeA, 0.);
        let mut terrain = Flat;
        z.position = Vec3::Y * z.model.base_hover();
        let mut events = Vec::new();
        z.step(
            Controls {
                lane: true,
                ..Default::default()
            },
            &mut terrain,
            &mut events,
        );
        for _ in 0..180 {
            z.step(
                Controls {
                    throttle: 1.0,
                    ..Default::default()
                },
                &mut terrain,
                &mut events,
            );
        }
        assert!((z.position.y - HIGH_LANE).abs() < 0.05);
        assert!(z.position.z > 20.0);
        for _ in 0..240 {
            z.step(
                Controls {
                    brake: 1.0,
                    ..Default::default()
                },
                &mut terrain,
                &mut events,
            );
        }
        assert!(z.velocity.z < 0.0);
        z.step(
            Controls {
                lane: true,
                ..Default::default()
            },
            &mut terrain,
            &mut events,
        );
        for _ in 0..180 {
            z.step(Controls::default(), &mut terrain, &mut events);
        }
        assert!((z.position.y - z.model.base_hover()).abs() < 0.05);
    }
}
