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
    yaw_rate: f32,
    tilt: [f32; 2],
    tilt_rate: [f32; 2],
    hop_charge: f32,
    hopping: bool,
}

fn seek(value: f32, target: f32, delta: f32) -> f32 {
    value + (target - value).clamp(-delta, delta)
}

impl Zoomer {
    pub fn new(model: Model, yaw: f32) -> Self {
        Self {
            model,
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
            yaw_rate: 0.,
            tilt: [0.; 2],
            tilt_rate: [0.; 2],
            hop_charge: 0.,
            hopping: false,
        }
    }

    pub fn heading(&self) -> Vec3 {
        Quat::from_rotation_y(self.yaw) * Vec3::Z
    }

    pub fn orientation(&self) -> Quat {
        Quat::from_rotation_y(self.yaw)
            * Quat::from_rotation_x(self.pitch + self.tilt[0])
            * Quat::from_rotation_z(self.bank + self.tilt[1])
    }

    pub fn step(&mut self, input: Controls, terrain: &mut dyn Ground, events: &mut Vec<Event>) {
        let car = self.model.car();
        let (accel, max_brake, max_gain, intake, response) = if car {
            (39., 54., 3.5, 1., 20.)
        } else {
            (50., 84., 4., 1.5, 60.)
        };
        self.steer = seek(self.steer, input.steer, 8. * DT);
        self.lean = seek(self.lean, input.lean, 8. * DT);
        self.throttle = seek(self.throttle, input.throttle, 4. * DT);
        self.brake += (input.brake - self.brake) * (8. * DT);
        if self.throttle > 0.0 && self.brake > 0.0 {
            self.brake = self.brake.min(0.25);
        }
        if input.lane {
            self.high = !self.high;
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
        self.yaw += self.yaw_rate * DT;

        let reversing = self.brake > 0.05 && forward_speed < 2.0 && self.throttle <= 0.0;
        let throttle = if reversing {
            -0.5 * self.brake
        } else {
            self.throttle
        };
        let intake_limit = 0.83333 * (0.5 + (speed * intake / TOP_SPEED).clamp(0.0, 1.0));
        let thrust = throttle.min(intake_limit);
        if thrust > self.thrust {
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
            (braking + if input.handbrake { 30. } else { 0. }) * DT,
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
        let horizontal = new_ahead * next_speed + new_left * (lateral_speed * (-grip * DT).exp());
        self.velocity = horizontal.clamp_length_max(TOP_SPEED * 1.05) + Vec3::Y * rise;
        self.move_hull(terrain, new_ahead, events);
        self.climb *= (-10. * DT).exp();

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
        let sway = speed / (speed + 5.0);
        self.bank += (-20_f32.to_radians() * self.steer * sway - self.bank) * (6.0 * DT);
        self.pitch +=
            (20_f32.to_radians() * self.lean - 0.06 * self.thrust - self.pitch) * (4.0 * DT);
    }

    fn move_hull(&mut self, terrain: &mut dyn Ground, heading: Vec3, events: &mut Vec<Event>) {
        let step = self.velocity.with_y(0.0) * DT;
        let half = self.model.half();
        let hit = |g: &mut dyn Ground, pos, motion| {
            crate::wheeled::hull_hit(g, pos, motion, half, heading)
        };
        if let Some((fraction, normal)) = hit(terrain, self.position, step) {
            let climb = (self.grounded && normal.y.abs() < 0.3)
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
                let normal_speed = self.velocity.dot(normal);
                if normal_speed < 0.0 {
                    self.velocity -= normal * (normal_speed * 1.4);
                    events.push(Event::Impact(-normal_speed * 1.4));
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
                self.velocity -= normal * speed;
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
