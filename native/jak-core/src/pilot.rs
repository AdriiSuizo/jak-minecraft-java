//! How Jak sits on a vehicle: `target-pilot-trans` (the sitting loop's three
//! interpolants, driven by the vehicle's acceleration) and the get-on motion
//! of `target-pilot-get-on` (engine/target/pilot-states.gc, Jak 3).
//!
//! and `target-pilot-impact`, the smack he takes when the vehicle is hit.
//!
//! Bevy-free and Minecraft-free: the host feeds the vehicle's acceleration and
//! controls once a 60 Hz frame and reads back where in each sitting clip Jak is.
use crate::math::{METER, SECONDS_PER_FRAME};
use glam::{Quat, Vec3};

/// `rider-stance` of the vehicle's `vehicle-rider-info`: which sitting loop
/// the pilot plays (`target-pilot-stance`'s `cond`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Stance {
    /// `target-pilot-bike-anim-loop`: the bikes.
    Bike,
    /// `target-pilot-car-anim-loop`: the hover cars.
    Car,
    /// `target-pilot-wcar-anim-loop`: the desert's wheeled cars.
    WheeledCar,
}

/// `target-pilot-init`: `left-right-accel-factor` and `front-back-accel-factor`
/// start at 0.0000012207031 for every stance of Jak, `up-down-accel-factor` at
/// 0.00000051879886 on a bike and 0.00000061035155 otherwise, each times the
/// vehicle's `pilot-x/y/z-accel-factor` (1.0 on all six zoomers).
const LEFT_RIGHT_FACTOR: f32 = 0.000_001_220_703_1;
const FRONT_BACK_FACTOR: f32 = 0.000_001_220_703_1;
const UP_DOWN_FACTOR_BIKE: f32 = 0.000_000_518_798_86;
const UP_DOWN_FACTOR_OTHER: f32 = 0.000_000_610_351_55;

/// `target-pilot-get-on`: Jak's trans reaches the seat when the animation
/// reaches artist frame 10; the vehicle is told `pilot-on` (and takes the
/// pilot's controls) from artist frame 20.5 (`target-pilot-signal-ready`).
pub const GET_ON_ARRIVE_AFRAME: f32 = 10.0;
pub const GET_ON_READY_AFRAME: f32 = 20.5;

/// `target-pilot-get-on`'s per-frame blend at animation frame `aframe`:
/// `(horizontal, vertical)`. Horizontally and in facing Jak goes
/// `sin(lerp-scale(0, 16384, aframe, 0, 10))` of the way (a quarter sine: fast
/// then slow), vertically `lerp-scale(0, 1, aframe, 0, 10)` of the way
/// (straight).
pub fn get_on_blend(aframe: f32) -> (f32, f32) {
    let t = (aframe / GET_ON_ARRIVE_AFRAME).clamp(0.0, 1.0);
    ((t * std::f32::consts::FRAC_PI_2).sin(), t)
}

/// Where Jak is `aframe` frames into getting on, from where he stood (`from`)
/// to the seat (`seat`, wherever the vehicle has taken it by now):
/// `vector-lerp!` by the sine, then trans.y by the straight line.
pub fn get_on_position(from: Vec3, seat: Vec3, aframe: f32) -> Vec3 {
    let (horizontal, vertical) = get_on_blend(aframe);
    let mut at = from.lerp(seat, horizontal);
    at.y = from.y + (seat.y - from.y) * vertical;
    at
}

/// `target-pilot-trans` goes to `target-pilot-impact` when the vehicle's acceleration of this frame (x and z, units a
/// second squared) is above 2048000 (500 m/s^2).
pub const IMPACT_ACCEL: f32 = 2_048_000.0;
/// `target-pilot-impact` opens with `(ja-channel-push! 1 (seconds 0.05))`, and the stance that follows it with
/// `(ja-channel-push! 3 (seconds 0.1)`: the blends into and out of the smack.
const SMACK_BLEND_IN: f32 = 0.05;
const SMACK_BLEND_OUT: f32 = 0.1;

/// The clips of `target-pilot-impact` (`jakb-pilot-bike-smack-*`, `jakb-pilot-car-smack-*`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Smack {
    Left,
    Right,
    Front,
    Back,
    Shock,
}

impl Smack {
    pub const ALL: [Smack; 5] = [Smack::Left, Smack::Right, Smack::Front, Smack::Back, Smack::Shock];

    pub fn suffix(self) -> &'static str {
        match self {
            Self::Left => "smack-left",
            Self::Right => "smack-right",
            Self::Front => "smack-front",
            Self::Back => "smack-back",
            Self::Shock => "smack-shock",
        }
    }

    fn index(self) -> usize {
        self as usize
    }

    /// `target-pilot-impact`'s choice from the smoothed local acceleration: `(x * 4, y, z) / |a|` against 0.707 in the
    /// order x (right, left), y (up and down: the shock), z (back, front), else the shock. (Its numbers 5, 4, 3, 2, 1, 0, 6.)
    pub fn pick(local_accel: Vec3) -> Self {
        let length = local_accel.length();
        let s = Vec3::new(4.0 * local_accel.x, local_accel.y, local_accel.z) / length;
        if s.x > 0.707 {
            Self::Right
        } else if s.x < -0.707 {
            Self::Left
        } else if s.y > 0.707 || s.y < -0.707 {
            Self::Shock
        } else if s.z > 0.707 {
            Self::Back
        } else if s.z < -0.707 {
            Self::Front
        } else {
            Self::Shock
        }
    }
}

/// How a smack clip runs, from its art group: its frames, how many it plays a second (the clip's `speed` at 60 Hz) and
/// its artist base and step (`ja-aframe`: the frame of artist frame a is (a - base) / step).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ClipTiming {
    pub frames: usize,
    pub frames_per_second: f32,
    pub base: f32,
    pub step: f32,
}

/// Where `target-pilot-impact` starts a clip and how fast it runs it: bike left and right at frame 0, the others at
/// artist frame 0; the car's left and right at frame 0, its front at artist frame 4 and 1.3 times the speed
/// (`seek! max 1.3`), its back at artist frame 2, its shock at artist frame 0.
fn smack_start(stance: Stance, smack: Smack) -> (Option<f32>, f32, f32) {
    // (raw start frame, artist start frame, speed)
    let car = stance != Stance::Bike;
    match (car, smack) {
        (_, Smack::Left | Smack::Right) => (Some(0.0), 0.0, 1.0),
        (true, Smack::Front) => (None, 4.0, 1.3),
        (true, Smack::Back) => (None, 2.0, 1.0),
        _ => (None, 0.0, 1.0),
    }
}

/// A smack in progress, or just over.
#[derive(Clone, Copy, Debug)]
struct Running {
    smack: Smack,
    /// Seconds since the clip started.
    age: f32,
}

/// What the client plays of the smack: the clip (`pilot-bike-` or `pilot-car-` plus `suffix`), its frame, and how much of
/// the pose is the smack's rather than the sitting loop's.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SmackPose {
    pub smack: Smack,
    pub frame: f32,
    pub weight: f32,
}

/// `target-pilot-trans`: the sitting loop's interpolants.
#[derive(Clone, Debug)]
pub struct Pilot {
    /// Timings of the five smack clips of this stance (in [`Smack::ALL`] order), if the art has them.
    timings: [Option<ClipTiming>; 5],
    running: Option<Running>,
    /// The smack that just ended, held on its last frame while it blends out: (smack, seconds since).
    ended: Option<(Smack, f32)>,
    pub stance: Stance,
    /// `left-right-interp`, `front-back-interp`, `up-down-interp`.
    pub left_right: f32,
    pub front_back: f32,
    pub up_down: f32,
    /// `accel-array`: the last eight frames' linear acceleration (Jak units
    /// a second squared, world axes), newest first.
    accel: [Vec3; 8],
    factors: [f32; 3],
    /// `player-turn-anim-bias`, `-min`, `-max`: 0, -1 and 1 on every zoomer.
    bias: f32,
    min: f32,
    max: f32,
    /// `local-accel`, for the host's eyes.
    pub local_accel: Vec3,
}

impl Pilot {
    pub fn new(stance: Stance) -> Self {
        let up_down = if stance == Stance::Bike { UP_DOWN_FACTOR_BIKE } else { UP_DOWN_FACTOR_OTHER };
        Self {
            timings: [None; 5],
            running: None,
            ended: None,
            stance,
            left_right: 0.0,
            front_back: 0.0,
            up_down: 0.0,
            accel: [Vec3::ZERO; 8],
            factors: [LEFT_RIGHT_FACTOR, FRONT_BACK_FACTOR, up_down],
            bias: 0.0,
            min: -1.0,
            max: 1.0,
            local_accel: Vec3::ZERO,
        }
    }

    /// One 60 Hz frame of `target-pilot-trans`.
    ///
    /// `accel` is the vehicle's `lin-acceleration` (Jak units a second
    /// squared, world axes), `rotation` the vehicle's orientation (Jak's own
    /// bone transform is the vehicle's), `steering` and `lean_z` the vehicle's
    /// controls and `ticks` `(current-time)` in 300 Hz ticks.
    pub fn step(&mut self, accel: Vec3, rotation: Quat, steering: f32, lean_z: f32, ticks: f64) {
        for i in (1..8).rev() {
            self.accel[i] = self.accel[i - 1];
        }
        self.accel[0] = accel;
        // The eight added up, times a half (not averaged: the factors are tuned to it).
        let sum: Vec3 = self.accel.iter().copied().sum::<Vec3>() * 0.5;
        self.local_accel = rotation.inverse() * sum;
        // `f30-0`: 182.04445 * (0.6 * time) in rotation units, so 0.6 degrees a tick.
        let wobble = (0.6 * ticks).to_radians();
        let (sin, cos) = (wobble.sin() as f32, wobble.cos() as f32);
        let rate = 8.0 * SECONDS_PER_FRAME;
        let lr = steering + self.bias + 0.03 * sin - self.local_accel.x * self.factors[0];
        self.left_right = (self.left_right + (lr - self.left_right) * rate).min(self.max).max(self.min);
        let fb = lean_z + 0.03 * cos - self.local_accel.z * self.factors[1];
        self.front_back = (self.front_back + (fb - self.front_back) * rate).clamp(-1.0, 1.0);
        let ud = 0.03 * cos - self.local_accel.y * self.factors[2];
        self.up_down = (self.up_down + (ud - self.up_down) * rate).clamp(-1.0, 1.0);
    }

    /// The art's timing of the smack clips of this stance, in [`Smack::ALL`] order.
    pub fn set_smacks(&mut self, timings: [Option<ClipTiming>; 5]) {
        self.timings = timings;
    }

    /// The seconds from the start of a smack to its last frame, or none when the art lacks the clip.
    pub fn smack_seconds(&self, smack: Smack) -> Option<f32> {
        let (_, rate, last, _) = self.plan(smack)?;
        Some((last - self.start_frame(smack)?) / rate)
    }

    /// (frames, frames a second including the clip's speed multiplier, last frame, base).
    fn plan(&self, smack: Smack) -> Option<(usize, f32, f32, f32)> {
        let timing = self.timings[smack.index()]?;
        if timing.frames < 2 || timing.frames_per_second <= 0.0 {
            return None;
        }
        let (_, _, speed) = smack_start(self.stance, smack);
        Some((timing.frames, timing.frames_per_second * speed, (timing.frames - 1) as f32, timing.base))
    }

    fn start_frame(&self, smack: Smack) -> Option<f32> {
        let timing = self.timings[smack.index()]?;
        let (raw, artist, _) = smack_start(self.stance, smack);
        Some(raw.unwrap_or_else(|| (artist - timing.base) / timing.step))
    }

    /// The end of the frame, after [`Self::step`]: `target-pilot-trans`'s change to `target-pilot-impact` when the
    /// frame's acceleration is above [`IMPACT_ACCEL`] in x and z while he is in `target-pilot-stance` (`seated`:
    /// the get-on is over), the smack's clip playing out, and then `(go target-pilot-stance)`, the acceleration of
    /// the frame cleared.
    pub fn after_step(&mut self, seated: bool) {
        if !seated {
            self.running = None;
            self.ended = None;
            return;
        }
        if let Some(mut running) = self.running {
            running.age += SECONDS_PER_FRAME;
            match self.smack_seconds(running.smack) {
                Some(length) if running.age < length => self.running = Some(running),
                _ => {
                    self.running = None;
                    self.ended = Some((running.smack, 0.0));
                    self.accel[0] = Vec3::ZERO;
                }
            }
            return;
        }
        if let Some((smack, since)) = self.ended {
            let since = since + SECONDS_PER_FRAME;
            self.ended = (since < SMACK_BLEND_OUT).then_some((smack, since));
        }
        let a = self.accel[0];
        if a.x * a.x + a.z * a.z > IMPACT_ACCEL * IMPACT_ACCEL {
            let smack = Smack::pick(self.local_accel);
            if self.smack_seconds(smack).is_some() {
                self.running = Some(Running { smack, age: 0.0 });
                self.ended = None;
            }
        }
    }

    /// The smack to blend over the sitting loop, if one is playing or has just ended.
    pub fn smack(&self) -> Option<SmackPose> {
        if let Some(running) = self.running {
            let (_, rate, last, _) = self.plan(running.smack)?;
            let start = self.start_frame(running.smack)?;
            return Some(SmackPose {
                smack: running.smack,
                frame: (start + running.age * rate).min(last),
                weight: (running.age / SMACK_BLEND_IN).min(1.0),
            });
        }
        let (smack, since) = self.ended?;
        let (_, _, last, _) = self.plan(smack)?;
        Some(SmackPose { smack, frame: last, weight: 1.0 - since / SMACK_BLEND_OUT })
    }

    /// Where in each of the stance's clips the sitting loop is, as
    /// `target-pilot-*-anim-loop` sets its three channels.
    pub fn loop_pose(&self) -> LoopPose {
        match self.stance {
            // Channel 0 and 1 (turn-back, turn-front) at 5 * (1 - left-right) artist frames
            // (their clips are 3 frames 5 artist frames apart), blended by 0.5 * (1 +
            // 1.42 * (front-back - 0.3)); channel 2 (up-down: 6 frames, 2 apart) at
            // 5 * (1 - up-down) artist frames, weighted |up-down|.
            Stance::Bike => LoopPose {
                turn: 1.0 - self.left_right,
                front: (0.5 * (1.0 + 1.42 * (self.front_back - 0.3))).clamp(0.0, 1.0),
                vertical: 2.5 * (1.0 - self.up_down),
                vertical_weight: self.up_down.abs(),
            },
            // The cars: 5 * (1 + left-right), blend 0.5 * (1 + front-back), channel 2
            // (up-down: 6 frames, 4 apart) at 10 * (1 - up-down) artist frames.
            Stance::Car | Stance::WheeledCar => LoopPose {
                turn: 1.0 + self.left_right,
                front: 0.5 * (1.0 + self.front_back),
                vertical: 2.5 * (1.0 - self.up_down),
                vertical_weight: self.up_down.abs(),
            },
        }
    }
}

/// The sitting loop's channels, in the clips' own frame numbers.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LoopPose {
    /// Frame (0..2) of `turn-back` and `turn-front` alike.
    pub turn: f32,
    /// Weight of `turn-front` over `turn-back`.
    pub front: f32,
    /// Frame (0..5) of `up-down`.
    pub vertical: f32,
    /// Weight of `up-down` over the other two.
    pub vertical_weight: f32,
}

/// A vehicle's acceleration (m/s^2) in Jak units a second squared.
pub fn accel_units(metres_per_second_squared: Vec3) -> Vec3 {
    metres_per_second_squared * METER
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn get_on_arrives_by_a_sine_and_rises_in_a_line() {
        let from = Vec3::new(2.0, 0.0, 0.0);
        let seat = Vec3::new(0.0, 3.0, 0.0);
        let start = get_on_position(from, seat, 0.0);
        assert_eq!(start, from);
        // Halfway through its ten frames: horizontally 70.7 %, vertically half.
        let half = get_on_position(from, seat, 5.0);
        assert!((half.x - 2.0 * (1.0 - (std::f32::consts::FRAC_PI_4).sin())).abs() < 1e-5);
        assert!((half.y - 1.5).abs() < 1e-6);
        // From the tenth frame on he is on the seat, and stays.
        assert!((get_on_position(from, seat, 10.0) - seat).length() < 1e-6);
        assert!((get_on_position(from, seat, 29.0) - seat).length() < 1e-6);
        assert!(GET_ON_READY_AFRAME > GET_ON_ARRIVE_AFRAME);
    }

    #[test]
    fn a_level_steady_vehicle_leaves_the_pilot_at_the_wobble() {
        let mut pilot = Pilot::new(Stance::Bike);
        for frame in 0..600 {
            pilot.step(Vec3::ZERO, Quat::IDENTITY, 0.0, 0.0, frame as f64 * 5.0);
        }
        assert!(pilot.left_right.abs() <= 0.03 + 1e-4);
        assert!(pilot.front_back.abs() <= 0.03 + 1e-4);
        assert!(pilot.up_down.abs() <= 0.03 + 1e-4);
        let pose = pilot.loop_pose();
        assert!((pose.turn - 1.0).abs() < 0.05, "straight ahead sits on the middle frame");
    }

    #[test]
    fn steering_and_braking_move_the_loop_the_way_the_original_does() {
        // Bike: left-right-interp 1 (a full left) is frame 0 of the turn clips; car: frame 2.
        let mut bike = Pilot::new(Stance::Bike);
        let mut car = Pilot::new(Stance::Car);
        for frame in 0..300 {
            bike.step(Vec3::ZERO, Quat::IDENTITY, 1.0, 0.0, frame as f64 * 5.0);
            car.step(Vec3::ZERO, Quat::IDENTITY, 1.0, 0.0, frame as f64 * 5.0);
        }
        assert!(bike.left_right > 0.95 && bike.left_right <= 1.0);
        assert!(bike.loop_pose().turn < 0.05);
        assert!(car.loop_pose().turn > 1.95);
        // Braking hard (acceleration backwards, -z in the vehicle's own axes) leans him forward:
        // front-back = -local.z * factor rises.
        let mut pilot = Pilot::new(Stance::Car);
        let braking = Vec3::new(0.0, 0.0, -20.0) * METER;
        for frame in 0..60 {
            pilot.step(braking, Quat::IDENTITY, 0.0, 0.0, frame as f64 * 5.0);
        }
        assert!(pilot.front_back > 0.15, "got {}", pilot.front_back);
        // Turned half a turn, the same push is the same lean in the vehicle's frame.
        let mut turned = Pilot::new(Stance::Car);
        let around = Quat::from_rotation_y(std::f32::consts::PI);
        for frame in 0..60 {
            turned.step(braking * Vec3::new(-1.0, 1.0, -1.0), around, 0.0, 0.0, frame as f64 * 5.0);
        }
        assert!((turned.front_back - pilot.front_back).abs() < 0.02);
    }

    /// `target-pilot-get-off`: from the seat the get-off animation (to artist frame 7.2 on a bike, at twice
    /// the speed: 3.6 frames), then a 2.5 m jump with the vehicle's velocity, and the fall.
    #[test]
    fn the_original_exit_jumps_with_the_vehicles_velocity_and_falls_to_the_ground() {
        use crate::anim::{AnimInfo, Anims};
        use crate::collide::{CollideWorld, Pat, PatMode, Tri};
        use crate::math::meters;
        use crate::{Jak, PadInput};
        use std::sync::Arc;
        struct Floor(Vec<Tri>);
        impl CollideWorld for Floor {
            fn fill(&mut self, _: Vec3, _: Vec3, out: &mut Vec<Tri>) {
                out.extend_from_slice(&self.0);
            }
        }
        let big = meters(300.0);
        let pat = Pat { mode: PatMode::Ground, ..Default::default() };
        let corners = [Vec3::new(-big, 0.0, -big), Vec3::new(-big, 0.0, big), Vec3::new(big, 0.0, big), Vec3::new(big, 0.0, -big)];
        let mut world = Floor(vec![
            Tri { v: [corners[0], corners[1], corners[2]], pat },
            Tri { v: [corners[0], corners[2], corners[3]], pat },
        ]);
        // jakb-pilot-bike-get-off: 5 frames at 0.25 a tick, artist frames two apart (the export's own numbers).
        let anims = Anims::from_rows([(
            "jakb-pilot-bike-get-off",
            AnimInfo { frames: 5, speed: 0.25, artist_base: 0.0, artist_step: 2.0 },
        )]);
        // A zoomer's high lane: 13 m up, going 20 m/s.
        let mut jak = Jak::new(Vec3::new(0.0, meters(13.0), 0.0), 0.0);
        jak.anims = Arc::new(anims);
        jak.control.transv = Vec3::new(0.0, 0.0, meters(20.0));
        jak.pilot_get_off(true);
        let (mut peak, mut get_off_frames, mut jumped_at) = (meters(13.0), 0, None);
        let mut held_height = meters(13.0);
        let mut landed = None;
        for frame in 0..600 {
            jak.step(&PadInput::default(), &mut world, None);
            peak = peak.max(jak.trans().y);
            if jak.state.name() == "pilot-get-off" {
                held_height = held_height.min(jak.trans().y);
            }
            if frame < 14 && std::env::var("JAKMC_TRACE").is_ok() { eprintln!("frame {frame}: {} frame {} y {}", jak.state.name(), jak.chan.frame, jak.trans().y); }
            match jak.state.name() {
                "pilot-get-off" => get_off_frames += 1,
                "jump" if jumped_at.is_none() => jumped_at = Some(frame),
                _ => {}
            }
            if landed.is_none() && jak.trans().y < 1.0 && frame > 20 {
                landed = Some(frame);
            }
        }
        // On the seat while the animation plays: eight frames, the vehicle's velocity kept (gravity only sags him a hand's breadth).
        assert!((7..=9).contains(&get_off_frames), "the get-off ran {get_off_frames} frames");
        assert!(held_height > meters(13.0) - meters(0.25), "he sagged to {held_height} while getting off");
        assert!(jumped_at.is_some(), "he never jumped; ended in {}", jak.state.name());
        assert!(peak > meters(13.0) + meters(2.0), "the jump rose only to {peak}");
        assert!(landed.is_some(), "he never came down: y {}", jak.trans().y);
        // He keeps going the way the vehicle was (it carried 20 m/s for the 1.7 s or so he was in the air).
        assert!(jak.trans().z > meters(15.0), "carried to z {}", jak.trans().z);
        assert!(jak.trans().y < 1.0, "on the ground: y {}", jak.trans().y);
    }

    fn timing(frames: usize) -> ClipTiming {
        ClipTiming { frames, frames_per_second: 15.0, base: 0.0, step: 2.0 }
    }

    fn smacking(stance: Stance) -> Pilot {
        let mut pilot = Pilot::new(stance);
        pilot.set_smacks([Some(timing(31)), Some(timing(31)), Some(timing(21)), Some(timing(21)), Some(timing(16))]);
        pilot
    }

    /// One frame at `accel` (m/s^2, world axes) with the vehicle level.
    fn frame(pilot: &mut Pilot, accel: Vec3, seated: bool) {
        pilot.step(accel * METER, Quat::IDENTITY, 0.0, 0.0, 0.0);
        pilot.after_step(seated);
    }

    #[test]
    fn target_pilot_impact_picks_its_clip_from_the_local_acceleration() {
        // x counts four times: a hit from the side wins over most; right is +x (the vehicle's left is pushed).
        assert_eq!(Smack::pick(Vec3::new(1.0, 0.0, 0.0)), Smack::Right);
        assert_eq!(Smack::pick(Vec3::new(-1.0, 0.0, 0.0)), Smack::Left);
        assert_eq!(Smack::pick(Vec3::new(0.3, 0.0, 1.0)), Smack::Right, "0.3 * 4 / 1.04 is past 0.707");
        assert_eq!(Smack::pick(Vec3::new(0.1, 0.0, 1.0)), Smack::Back);
        assert_eq!(Smack::pick(Vec3::new(0.0, 0.0, -1.0)), Smack::Front);
        assert_eq!(Smack::pick(Vec3::new(0.0, 1.0, 0.0)), Smack::Shock);
        assert_eq!(Smack::pick(Vec3::new(0.0, -1.0, 0.0)), Smack::Shock);
        // Nothing in any one direction: the shock.
        assert_eq!(Smack::pick(Vec3::new(0.1, 0.5, 0.5)), Smack::Shock);
    }

    #[test]
    fn a_hard_enough_hit_smacks_the_pilot_and_a_gentle_one_does_not() {
        let mut pilot = smacking(Stance::Bike);
        // 400 m/s^2 is under the 500 of the threshold.
        for _ in 0..8 {
            frame(&mut pilot, Vec3::new(0.0, 0.0, -400.0), true);
        }
        assert_eq!(pilot.smack(), None);
        // A hard stop: 900 m/s^2 backwards.
        let mut pilot = smacking(Stance::Bike);
        frame(&mut pilot, Vec3::new(0.0, 0.0, -900.0), true);
        let pose = pilot.smack().expect("a smack");
        assert_eq!(pose.smack, Smack::Front);
        assert_eq!(pose.frame, 0.0, "the bike's front smack starts at artist frame 0");
        assert_eq!(pose.weight, 0.0);
        // Not on the seat yet (getting on): nothing.
        let mut boarding = smacking(Stance::Bike);
        frame(&mut boarding, Vec3::new(0.0, 0.0, -900.0), false);
        assert_eq!(boarding.smack(), None);
        // Only the horizontal acceleration counts: a drop of 900 m/s^2 up and down is not a hit.
        let mut falling = smacking(Stance::Bike);
        frame(&mut falling, Vec3::new(0.0, 900.0, 0.0), true);
        assert_eq!(falling.smack(), None);
    }

    #[test]
    fn the_smack_plays_its_clip_then_blends_out_over_a_tenth_of_a_second() {
        let mut pilot = smacking(Stance::Bike);
        frame(&mut pilot, Vec3::new(900.0, 0.0, 0.0), true);
        assert_eq!(pilot.smack().unwrap().smack, Smack::Right);
        // 31 frames at 15 a second: 2 seconds to its last frame.
        assert!((pilot.smack_seconds(Smack::Right).unwrap() - 2.0).abs() < 1e-5);
        let mut weights = Vec::new();
        for _ in 0..60 {
            frame(&mut pilot, Vec3::ZERO, true);
            weights.push(pilot.smack().unwrap());
        }
        // In over 0.05 s, then full; the frame is at the clip's own rate (15 a second).
        assert!(weights[0].weight > 0.3 && weights[0].weight < 0.4, "{:?}", weights[0]);
        assert_eq!(weights[3].weight, 1.0);
        assert!((weights[59].frame - 15.0 * 1.0).abs() < 0.3, "one second in is frame 15: {:?}", weights[59]);
        // A second hit during it does not restart it (it is not in the stance state).
        frame(&mut pilot, Vec3::new(0.0, 0.0, -900.0), true);
        assert_eq!(pilot.smack().unwrap().smack, Smack::Right);
        let mut ended = None;
        for k in 0..200 {
            frame(&mut pilot, Vec3::ZERO, true);
            if pilot.smack().is_some_and(|p| p.weight < 1.0 && p.frame >= 30.0) && ended.is_none() {
                ended = Some(k);
            }
        }
        assert!(ended.is_some(), "the smack ran out");
        // After the blend out nothing remains, and the acceleration it cleared does not smack him again.
        assert_eq!(pilot.smack(), None);
    }

    #[test]
    fn the_car_starts_its_front_and_back_smacks_partway_in_and_runs_the_front_a_third_faster() {
        let mut pilot = smacking(Stance::Car);
        frame(&mut pilot, Vec3::new(0.0, 0.0, -900.0), true);
        let front = pilot.smack().unwrap();
        assert_eq!(front.smack, Smack::Front);
        // artist frame 4 with a step of 2: frame 2. 21 frames, 20 - 2 = 18 left at 15 * 1.3 a second.
        assert_eq!(front.frame, 2.0);
        assert!((pilot.smack_seconds(Smack::Front).unwrap() - 18.0 / (15.0 * 1.3)).abs() < 1e-5);
        let mut back = smacking(Stance::Car);
        frame(&mut back, Vec3::new(0.0, 0.0, 900.0), true);
        assert_eq!(back.smack().unwrap().smack, Smack::Back);
        assert_eq!(back.smack().unwrap().frame, 1.0, "artist frame 2");
        assert!((back.smack_seconds(Smack::Back).unwrap() - 19.0 / 15.0).abs() < 1e-5);
        let mut bike = smacking(Stance::Bike);
        frame(&mut bike, Vec3::new(0.0, 0.0, 900.0), true);
        assert_eq!(bike.smack().unwrap().frame, 0.0, "the bike's back smack starts at artist frame 0");
    }

    #[test]
    fn without_the_smack_clips_the_pilot_just_sits() {
        let mut pilot = Pilot::new(Stance::Bike);
        frame(&mut pilot, Vec3::new(0.0, 0.0, -900.0), true);
        assert_eq!(pilot.smack(), None);
    }

    /// The vehicle he steps off is solid to him: the hull's faces, wound as the world's are (counterclockwise seen from
    /// outside), blocking from outside only. Standing still, he comes down on its roof, not through it to the ground.
    #[test]
    fn a_stationary_vehicle_catches_the_pilot_who_steps_off_it() {
        use crate::anim::{AnimInfo, Anims};
        use crate::collide::{CollideWorld, Pat, PatMode, Tri};
        use crate::math::meters;
        use crate::{Jak, PadInput};
        use std::sync::Arc;
        struct World(Vec<Tri>);
        impl CollideWorld for World {
            fn fill(&mut self, _: Vec3, _: Vec3, out: &mut Vec<Tri>) {
                out.extend_from_slice(&self.0);
            }
        }
        let pat = Pat { mode: PatMode::Ground, ..Default::default() };
        let tri = |a: Vec3, b: Vec3, c: Vec3| Tri { v: [a, b, c], pat };
        let big = meters(300.0);
        let (c0, c1, c2, c3) = (Vec3::new(-big, 0., -big), Vec3::new(-big, 0., big), Vec3::new(big, 0., big), Vec3::new(big, 0., -big));
        let floor = vec![tri(c0, c1, c2), tri(c0, c2, c3)];
        // minecraft's MinecraftCollision.box, the hull of the h-bike (0.8 x 0.6 x 1.5 m half sizes) 3 m up.
        let (x, y, z) = (-meters(0.8), meters(2.4), -meters(1.5));
        let (xx, yy, zz) = (meters(0.8), meters(3.6), meters(1.5));
        let mut hull = Vec::new();
        let mut face = |a: Vec3, b: Vec3, c: Vec3, d: Vec3| {
            hull.push(tri(a, b, c));
            hull.push(tri(a, c, d));
        };
        face(Vec3::new(x, yy, z), Vec3::new(x, yy, zz), Vec3::new(xx, yy, zz), Vec3::new(xx, yy, z));
        face(Vec3::new(x, y, z), Vec3::new(xx, y, z), Vec3::new(xx, y, zz), Vec3::new(x, y, zz));
        face(Vec3::new(x, y, z), Vec3::new(x, y, zz), Vec3::new(x, yy, zz), Vec3::new(x, yy, z));
        face(Vec3::new(xx, y, zz), Vec3::new(xx, y, z), Vec3::new(xx, yy, z), Vec3::new(xx, yy, zz));
        face(Vec3::new(xx, y, z), Vec3::new(x, y, z), Vec3::new(x, yy, z), Vec3::new(xx, yy, z));
        face(Vec3::new(x, y, zz), Vec3::new(xx, y, zz), Vec3::new(xx, yy, zz), Vec3::new(x, yy, zz));
        let anims = Arc::new(Anims::from_rows([(
            "jakb-pilot-bike-get-off",
            AnimInfo { frames: 5, speed: 0.25, artist_base: 0.0, artist_step: 2.0 },
        )]));
        let land = |solid: bool| {
            let mut tris = floor.clone();
            if solid {
                tris.extend(hull.iter().copied());
            }
            let mut world = World(tris);
            // On the seat, inside the hull, and not moving.
            let mut jak = Jak::new(Vec3::new(0.0, meters(3.185), 0.0), 0.0);
            jak.anims = anims.clone();
            jak.control.transv = Vec3::ZERO;
            jak.pilot_get_off(true);
            let mut peak = 0.0f32;
            for _ in 0..600 {
                jak.step(&PadInput::default(), &mut world, None);
                peak = peak.max(jak.trans().y);
            }
            (jak.trans(), peak, jak.state.name().to_string())
        };
        let (down, peak, state) = land(false);
        assert!(down.y < meters(0.5), "with nothing under him he falls to the ground: {down:?}");
        assert!(peak > meters(5.0), "the jump rose to {peak}");
        let (on_roof, peak, state_on) = land(true);
        // Up through the hull from inside, and down onto its roof (3.6 m): he stands there.
        assert!(peak > meters(5.0), "through the roof on the way up: {peak}");
        assert!((on_roof.y - meters(3.6)).abs() < meters(0.15), "on the roof: {on_roof:?} ({state_on}; without it {state})");
        assert!(on_roof.x.abs() < meters(0.9) && on_roof.z.abs() < meters(1.6), "over the hull: {on_roof:?}");
    }

    #[test]
    fn the_bike_loop_maps_front_back_through_the_original_blend() {
        let mut pilot = Pilot::new(Stance::Bike);
        pilot.front_back = 0.3;
        assert!((pilot.loop_pose().front - 0.5).abs() < 1e-6, "front-back 0.3 is the even mix");
        pilot.front_back = 1.0;
        assert!((pilot.loop_pose().front - (0.5 * (1.0 + 1.42 * 0.7))).abs() < 1e-6);
        pilot.front_back = -1.0;
        assert_eq!(pilot.loop_pose().front, 0.0);
        pilot.up_down = -1.0;
        let pose = pilot.loop_pose();
        assert_eq!(pose.vertical_weight, 1.0);
        assert!((pose.vertical - 5.0).abs() < 1e-6, "up-down -1: 10 artist frames, frame 5 of the clip");
    }
}
