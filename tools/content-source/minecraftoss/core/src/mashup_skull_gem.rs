//! Jak 3 `gem-deploy` / `gem-common-post`, in Minecraft block/tick units.
//! The server owns these items just like other stacks; rendering never pays
//! the inventory or creates a second collectible.
use std::sync::atomic::{AtomicU32, AtomicBool, Ordering};

pub const ID: &str = "jak:skull_gem";
static NATIVE_VISUALS:AtomicBool=AtomicBool::new(false);
pub fn set_native_visuals(value:bool){NATIVE_VISUALS.store(value,Ordering::Relaxed);}
pub fn native_visuals()->bool{NATIVE_VISUALS.load(Ordering::Relaxed)}
static BLOCKS_PER_METER: AtomicU32 = AtomicU32::new((25.0f32 / 36.0).to_bits());
pub fn set_blocks_per_meter(value: f32) {
    if value.is_finite() && (0.1..2.0).contains(&value) {
        BLOCKS_PER_METER.store(value.to_bits(), Ordering::Relaxed);
    }
}
pub fn blocks_per_meter() -> f64 { f32::from_bits(BLOCKS_PER_METER.load(Ordering::Relaxed)) as f64 }

/// Horizontal -5..5 m/s, upward 20 m/s (`gem-deploy` enter).
pub fn launch(x: f64, z: f64) -> [f64; 3] {
    let unit = blocks_per_meter() / 20.0;
    [(x * 10.0 - 5.0) * unit, 20.0 * unit, (z * 10.0 - 5.0) * unit]
}

#[derive(Clone, Copy, Debug)]
pub struct Motion {
    pub seconds: f32,
    pub angle: f32,
    pub spin: f32,
    pub settled: bool,
    pub bounces: u32,
    pub last_bounce: f32,
    /// Active homing state. Collection remains an inventory transaction,
    /// never a timeout or a render-side deletion.
    pub suction: Option<Suction>,
}
#[derive(Clone, Copy, Debug)]
pub struct Suction {
    pub seconds: f64,
    pub speed: f64,
    pub acceleration: f64,
    pub turn: f64,
    pub direction: f64,
}
impl Suction {
    pub fn new(random: f64, direction: f64) -> Self {
        Self { seconds: 0.0, speed: 0.0, acceleration: 80.0 + random * 120.0, turn: 0.0, direction }
    }
    /// OpenGOAL add-blue-motion: 10 m/s², ramping to 80..200 between
    /// 45 and 60 time frames; 400°/s² spiral, limited to 1600°/s.
    /// The caller sweeps this displacement against Minecraft geometry.
    pub fn step(&mut self, from: [f64; 3], target: [f64; 3]) -> [f64; 3] {
        let t = ((self.seconds - 0.75) / 0.25).clamp(0.0, 1.0);
        self.speed += (10.0 + (self.acceleration - 10.0) * t) * 0.05;
        self.seconds += 0.05;
        let d: [f64; 3] = std::array::from_fn(|i| from[i] - target[i]);
        let length = d.iter().map(|v| v * v).sum::<f64>().sqrt();
        let left = (length - self.speed * blocks_per_meter() * 0.05).max(0.0);
        let ratio = if length > 1e-9 { left / length } else { 0.0 };
        self.turn = (self.turn + 400.0 * 0.05).min(1600.0);
        let (sin, cos) = (self.turn.to_radians() * self.direction * 0.05).sin_cos();
        let next = [(d[0] * cos + d[2] * sin) * ratio, d[1] * ratio, (d[2] * cos - d[0] * sin) * ratio];
        std::array::from_fn(|i| target[i] + next[i] - from[i])
    }
}
impl Default for Motion {
    fn default() -> Self { Self { seconds: 0.0, angle: 0.0, spin: 1024.0, settled: false,bounces:0,last_bounce:-1.0, suction:None } }
}
impl Motion {
    pub fn tick(&mut self) {
        self.seconds += 0.05;
        if self.seconds>=15.0{self.settled=true;}
        if !self.settled { self.spin = (self.spin - 90.0 * 0.05).max(0.0); }
        self.spin += (110.0 - self.spin).clamp(-360.0 * 0.05, 360.0 * 0.05);
        self.angle = (self.angle + self.spin.to_radians() * 0.05).rem_euclid(std::f32::consts::TAU);
    }
    /// `projectile-bounce-reaction`: v -= n*dot(v,n)*rand(1.6,2.2),
    /// followed by gem-deploy's horizontal .7 and vertical rand(.7,.75).
    pub fn bounce(&mut self, velocity: &mut [f64; 3], axes: [bool; 3], reflection: f64, damping: f64) {
        if self.seconds-self.last_bounce>=0.1 {self.bounces=self.bounces.wrapping_add(1);self.last_bounce=self.seconds;}
        let impact = velocity.iter().map(|v| v * v).sum::<f64>().sqrt() * 20.0 / blocks_per_meter();
        for axis in 0..3 { if axes[axis] { velocity[axis] *= 1.0 - (1.6 + reflection * 0.6); } }
        velocity[0] *= 0.7;
        velocity[2] *= 0.7;
        velocity[1] *= 0.7 + damping * 0.05;
        self.spin = (self.spin + impact.min(30.0) as f32 * 24.0).min(720.0);
        let speed = velocity.iter().map(|v| v * v).sum::<f64>().sqrt() * 20.0 / blocks_per_meter();
        if axes[1] && (speed < 0.3 || self.seconds >= 10.0) {
            self.settled = true;
            *velocity = [0.0; 3];
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn launch_and_bounce_use_original_units_and_limits() {
        let unit = blocks_per_meter();
        let v = launch(0.0, 1.0);
        assert!((v[1] * 20.0 / unit - 20.0).abs() < 1e-9);
        assert!((v[0] * 20.0 / unit + 5.0).abs() < 1e-9);
        let mut motion = Motion::default();
        let mut fall = [unit * 0.1, -unit * 0.5, 0.0];
        motion.bounce(&mut fall, [false, true, false], 0.5, 0.5);
        assert!((fall[1] / unit - 0.5 * 0.9 * 0.725).abs() < 1e-9);
        assert!((fall[0] / unit - 0.07).abs() < 1e-9);
        assert_eq!(motion.spin, 720.0);
        motion.seconds = 10.1;
        motion.bounce(&mut fall, [false, true, false], 0.5, 0.5);
        assert!(motion.settled);
        assert_eq!(fall, [0.0; 3]);
    }
}
