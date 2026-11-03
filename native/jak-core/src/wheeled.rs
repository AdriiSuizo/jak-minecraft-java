//! Jak 3 wheeled handling from wcar.gc and the source mashup's 120 Hz suspension.
//! Coordinates are metres in the original Y-up vehicle frame.
use crate::particles::wcars::Specs;
use glam::{Quat, Vec3};

const DT: f32 = 1.0 / 120.0;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Model {
    Toad,
    Snake,
}
impl Model {
    pub fn name(self) -> &'static str {
        match self {
            Self::Toad => "toad",
            Self::Snake => "snake",
        }
    }
    pub fn half(self) -> Vec3 {
        match self {
            Self::Toad => Vec3::new(2.3, 1.15, 4.8),
            Self::Snake => Vec3::new(1.8, 0.65, 3.5),
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
    pub turbo: bool,
    pub hop: bool,
    pub fire: bool,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Event {
    PrepareHop,
    Jump,
    Creak,
    BottomOut,
    Impact(f32),
    Fire(bool),
}

pub trait Ground {
    fn below(&mut self, start: Vec3, reach: f32) -> Option<f32>;
    fn sweep(&mut self, start: Vec3, step: Vec3, half: Vec3) -> Option<(f32, Vec3)>;
}

pub struct Car {
    pub model: Model,
    pub position: Vec3,
    pub velocity: Vec3,
    pub yaw: f32,
    pub pitch: f32,
    pub roll: f32,
    pub yaw_rate: f32,
    pitch_rate: f32,
    roll_rate: f32,
    pub extension: [f32; 4],
    pub spin: [f32; 4],
    pub grounded: usize,
    pub steer: f32,
    pub throttle: f32,
    pub brake: f32,
    pub thrust: f32,
    pub rpm: f32,
    pub gear: usize,
    pub boosted: bool,
    pub recoil: [f32; 2],
    was_hop: bool,
    hop_age: f32,
    hop_force: f32,
    shot_delay: f32,
    barrel: bool,
    support_grace: f32,
    landing_cooldown: f32,
}

fn approach(value: f32, target: f32, delta: f32) -> f32 {
    value + (target - value).clamp(-delta, delta)
}

impl Car {
    pub fn new(model: Model, position: Vec3, yaw: f32, s: &Specs) -> Self {
        let [front, rear] = s.wheels.each_ref().map(|w| w.settle);
        Self {
            model,
            position,
            yaw,
            velocity: Vec3::ZERO,
            pitch: 0.,
            roll: 0.,
            yaw_rate: 0.,
            pitch_rate: 0.,
            roll_rate: 0.,
            extension: [front, front, rear, rear],
            spin: [0.; 4],
            grounded: 0,
            steer: 0.,
            throttle: 0.,
            brake: 0.,
            thrust: 0.,
            rpm: s.idle_rpm,
            gear: 1,
            boosted: false,
            recoil: [0.; 2],
            was_hop: false,
            hop_age: 0.,
            hop_force: 0.,
            shot_delay: 0.,
            barrel: false,
            support_grace: 0.,
            landing_cooldown: 0.,
        }
    }

    pub fn orientation(&self) -> Quat {
        Quat::from_rotation_y(self.yaw)
            * Quat::from_rotation_x(self.pitch)
            * Quat::from_rotation_z(self.roll)
    }

    pub fn heading(&self) -> Vec3 {
        Quat::from_rotation_y(self.yaw) * Vec3::Z
    }

    pub fn wheel_pose(&self, s: &Specs, i: usize) -> (Vec3, Quat, f32) {
        let w = &s.wheels[i / 2];
        let side = if i % 2 == 0 { 1. } else { -1. };
        let turn = if i < 2 { self.steer * s.steer } else { 0. };
        let center = w.center(self.extension[i], side, turn);
        let camber = 2. * w.camber * side
            + if w.wheel_follows_arm {
                w.suspension_angle(self.extension[i]) * side
            } else {
                0.
            };
        let rotation = Quat::from_rotation_y(turn)
            * Quat::from_rotation_z(camber)
            * Quat::from_rotation_x(self.spin[i]);
        (center, rotation, w.scale)
    }

    pub fn step(
        &mut self,
        s: &Specs,
        c: Controls,
        ground: &mut dyn Ground,
        events: &mut Vec<Event>,
    ) {
        for _ in 0..2 {
            self.advance(s, c, ground, events);
        }
    }

    fn advance(
        &mut self,
        s: &Specs,
        c: Controls,
        ground: &mut dyn Ground,
        events: &mut Vec<Event>,
    ) {
        self.steer = approach(self.steer, c.steer, 4. * DT);
        self.throttle = approach(self.throttle, c.throttle, 8. * DT);
        self.brake = approach(self.brake, c.brake, 8. * DT);
        self.hop_age = (self.hop_age - DT).max(0.);
        self.shot_delay = (self.shot_delay - DT).max(0.);
        self.boosted = c.turbo && c.throttle > 0.1;
        self.recoil
            .iter_mut()
            .for_each(|r| *r = (*r - 8. * DT).max(0.));
        let toad = self.model == Model::Toad;
        let rotation = self.orientation();
        let forward = self.heading();
        let across = Vec3::Y.cross(forward);
        let mut force = -Vec3::Y * s.gravity;
        let (mut pitch_torque, mut roll_torque, mut touching) = (0., 0., 0);
        for i in 0..4 {
            let w = &s.wheels[i / 2];
            let side = if i % 2 == 0 { 1. } else { -1. };
            let turn = if i < 2 { self.steer * s.steer } else { 0. };
            let arm = rotation * w.center(w.settle, side, turn);
            let tire = w.radius * w.scale;
            let probe = self.position + arm + Vec3::Y * (tire + w.travel);
            let reach = 2. * (tire + w.travel);
            self.extension[i] = 1.;
            if let Some(distance) = ground.below(probe, reach) {
                let terrain = probe.y - distance;
                let bottom = self.position.y + arm.y - tire;
                let extension = w.settle + (bottom - terrain) / w.vertical_travel();
                self.extension[i] = extension.clamp(0., 1.);
                if extension < 1. && (rotation * Vec3::Y).y > 0.3 {
                    touching += 1;
                    let vertical = self.velocity.y - self.pitch_rate * arm.dot(forward)
                        + self.roll_rate * arm.dot(across);
                    let crouch = if toad && c.hop { 0.28 } else { 0. };
                    let stiffness = (s.gravity / (w.travel * 0.65)).max(12.);
                    let damping = 2. * stiffness.sqrt() * (0.65 + w.damping).min(1.2);
                    let lift = (s.gravity * 0.25
                        + (w.settle - crouch - extension) * w.travel * stiffness
                        - damping * vertical * 0.25)
                        .clamp(0., s.gravity * 2.);
                    force.y += lift;
                    pitch_torque -= lift * arm.dot(forward);
                    roll_torque += lift * arm.dot(across);
                }
            }
            self.spin[i] = (self.spin[i] + self.velocity.dot(forward) * DT / tire)
                .rem_euclid(std::f32::consts::TAU);
        }
        self.landing_cooldown = (self.landing_cooldown - DT).max(0.);
        if touching >= 2
            && self.grounded == 0
            && self.velocity.y < -2.
            && self.landing_cooldown == 0.
        {
            events.push(if self.velocity.y < -6. {
                Event::BottomOut
            } else {
                Event::Creak
            });
            self.landing_cooldown = 0.3;
        }
        self.grounded = touching;
        self.support_grace = if touching >= 2 {
            0.18
        } else {
            (self.support_grace - DT).max(0.)
        };
        if toad {
            if c.hop && !self.was_hop && touching >= 2 {
                events.push(Event::PrepareHop);
            }
            if !c.hop && self.was_hop && touching >= 2 && self.hop_age == 0. {
                self.hop_force = 0.2;
                self.hop_age = 1.;
                events.push(Event::Jump);
            }
        } else if c.hop && !self.was_hop && touching >= 2 && self.hop_age == 0. {
            self.velocity.y += 0.3 * s.gravity * s.jump;
            self.hop_age = 0.3;
            events.push(Event::Jump);
        }
        self.was_hop = c.hop;
        if self.hop_force > 0. {
            force.y += 2. * s.gravity * s.jump;
            self.hop_force = (self.hop_force - DT).max(0.);
        }
        let speed = self.velocity.dot(forward);
        let lateral = self.velocity.dot(across);
        let reverse = c.brake > 0.05 && speed < 0.5 && c.throttle < 0.05;
        self.thrust = approach(
            self.thrust,
            if reverse { -c.brake * 0.45 } else { c.throttle },
            s.response * DT,
        );
        let boost = if self.boosted { 1. + s.boost } else { 1. };
        if touching > 0 {
            let ratio = s.gears.get(self.gear).copied().unwrap_or(1.).abs();
            let radius = s.wheels[1].radius * s.wheels[1].scale;
            let rpm = (speed.abs() / radius * 60. / std::f32::consts::TAU * ratio * s.final_drive)
                .max(s.idle_rpm);
            self.rpm += (rpm - self.rpm) * (DT * 12.).min(1.);
            if self.rpm > s.upshift && self.gear + 1 < s.gears.len() {
                self.gear += 1;
            } else if self.rpm < s.downshift && self.gear > 1 {
                self.gear -= 1;
            }
            let limit = s.speed * boost * if reverse { 0.35 } else { 1. };
            let torque = (1. - ((self.rpm - s.peak_rpm) / s.max_rpm).powi(2)).clamp(0.2, 1.);
            force += forward
                * (s.thrust
                    * self.thrust
                    * boost
                    * torque
                    * (1. - (speed.abs() / limit).powi(2)).max(0.));
            let brake = if reverse { 0. } else { c.brake };
            force -= forward
                * (speed.signum()
                    * (brake * 54. + if c.handbrake { 28. } else { 0. } + s.gravity * 0.08)
                        .min(speed.abs() / DT));
            let grip = if c.handbrake {
                s.sliding_grip * 0.18
            } else {
                s.grip
            };
            force -= across * (lateral / DT).clamp(-grip * s.gravity, grip * s.gravity);
            let wheelbase = (s.wheels[0].anchor.z - s.wheels[1].anchor.z).abs();
            let turn = (self.steer * s.steer / (1. + speed.abs() / 35.)).tan() * speed / wheelbase;
            self.yaw_rate += (turn - self.yaw_rate) * (DT * 8.).min(1.);
        } else {
            if toad && self.boosted {
                let jet = rotation * Vec3::Z;
                let gain =
                    (1. - (self.velocity.dot(jet).max(0.) / (s.speed * boost)).powi(2)).max(0.);
                force += jet * (s.thrust * s.boost * c.throttle * gain);
            }
            self.pitch_rate += c.lean * 1.8 * DT;
            self.roll_rate -= c.steer * 1.2 * DT;
            self.yaw_rate *= (-1.2 * DT).exp();
        }
        self.velocity += force * DT;
        self.yaw += self.yaw_rate * DT;
        self.pitch_rate += pitch_torque / (if toad { 10. } else { 5. }) * DT;
        self.roll_rate += roll_torque / 5. * DT;
        let damping = (-2.5 * DT).exp();
        self.pitch_rate = (self.pitch_rate * damping).clamp(-3., 3.);
        self.roll_rate = (self.roll_rate * damping).clamp(-3., 3.);
        self.pitch = (self.pitch + self.pitch_rate * DT).clamp(-1.2, 1.2);
        self.roll = (self.roll + self.roll_rate * DT).clamp(-1.2, 1.2);
        self.move_hull(ground, forward, events);
        if c.fire && self.shot_delay == 0. {
            self.shot_delay = if toad { 1. } else { 0.06 };
            self.barrel = !self.barrel;
            self.recoil[usize::from(self.barrel)] = 0.8;
            events.push(Event::Fire(self.barrel));
        }
    }

    fn move_hull(&mut self, ground: &mut dyn Ground, forward: Vec3, events: &mut Vec<Event>) {
        let motion = self.velocity * DT;
        let half = self.model.half();
        let Some((fraction, normal)) = hull_hit(ground, self.position, motion, half, forward)
        else {
            self.position += motion;
            return;
        };
        if self.support_grace > 0.
            && normal.y.abs() < 0.45
            && self.velocity.y < 3.
            && self.hop_age == 0.
            && self.hop_force == 0.
            && let Some(at) = climb(ground, self.position, motion, half, forward)
        {
            self.position = at;
            self.velocity.y = self.velocity.y.max(0.);
            return;
        }
        self.position += motion * fraction.max(0.);
        let blocked = self.velocity.dot(normal);
        if blocked < 0. {
            self.velocity -= normal * blocked;
            if -blocked > 2. {
                events.push(Event::Impact(-blocked));
            }
        }
        let mut remaining = motion * (1. - fraction.clamp(0., 1.));
        remaining -= normal * remaining.dot(normal).min(0.);
        for _ in 0..2 {
            if remaining.length_squared() < 1e-8 {
                break;
            }
            if let Some((time, n)) = hull_hit(ground, self.position, remaining, half, forward) {
                self.position += remaining * time.clamp(0., 1.);
                remaining *= 1. - time.clamp(0., 1.);
                remaining -= n * remaining.dot(n).min(0.);
                self.velocity -= n * self.velocity.dot(n).min(0.);
            } else {
                self.position += remaining;
                break;
            }
        }
    }
}

pub(super) fn hull_hit(
    world: &mut dyn Ground,
    position: Vec3,
    step: Vec3,
    dimensions: Vec3,
    forward: Vec3,
) -> Option<(f32, Vec3)> {
    let reach = (dimensions.z - dimensions.x).max(0.);
    let half = Vec3::new(dimensions.x, dimensions.y, dimensions.x);
    [0., reach, -reach]
        .into_iter()
        .filter_map(|offset| world.sweep(position + forward * offset, step, half))
        .filter(|(fraction, _)| *fraction < 1.)
        .min_by(|a, b| a.0.total_cmp(&b.0))
}

fn climb(
    world: &mut dyn Ground,
    position: Vec3,
    motion: Vec3,
    half: Vec3,
    forward: Vec3,
) -> Option<Vec3> {
    let horizontal = motion.with_y(0.);
    if horizontal.length_squared() < 1e-8 {
        return None;
    }
    // One Minecraft block in the port's metres, plus contact epsilon.
    let height = 6400. / crate::METER + 0.02;
    let up = Vec3::Y * height;
    if hull_hit(world, position, up, half, forward).is_some_and(|(_, normal)| normal.y < -0.1)
        || hull_hit(world, position + up, horizontal, half, forward).is_some()
    {
        return None;
    }
    let (mut lo, mut hi) = (0., height);
    for _ in 0..6 {
        let mid = (lo + hi) * 0.5;
        if hull_hit(world, position + Vec3::Y * mid, horizontal, half, forward).is_some() {
            lo = mid;
        } else {
            hi = mid;
        }
    }
    Some(position + Vec3::Y * hi + horizontal)
}
