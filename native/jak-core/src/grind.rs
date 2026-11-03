use std::sync::Arc;

use glam::Vec3;

use crate::collide::CollideWorld;
use crate::control::status;
use crate::math::{SECONDS_PER_FRAME, forward_up_nopitch_quat, meters, seconds};
use crate::{Jak, PadInput, State, anim, board, button};

const CAPTURE_RADIUS: f32 = meters(0.55);
const MIN_SPEED: f32 = meters(3.0);
const LIFT: f32 = meters(0.05);

#[derive(Clone, Debug)]
pub struct Rail {
    pub points: Vec<Vec3>,
}

impl Rail {
    pub fn new(mut points: Vec<Vec3>) -> Option<Self> {
        if points.iter().any(|point| !point.is_finite()) {
            return None;
        }
        // A native edge cut near a cell corner can have a genuine sub-cm
        // segment. Only identical consecutive points have no tangent; they
        // can arise from f32 conversion at a distant map origin.
        points.dedup();
        if points.len() < 2 || points.windows(2).any(|p|(p[1]-p[0]).length_squared()==0.0) { return None; }
        Some(Self { points })
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Ride {
    pub rail: usize,
    pub segment: usize,
    pub distance: f32,
    pub direction: f32,
    pub speed: f32,
}

#[derive(Default)]
pub struct Grind {
    pub rails: Arc<Vec<Rail>>,
    pub ride: Option<Ride>,
    pub cooldown_until: i64,
    /// The share of a spark the last frames launched (part 670's `num`
    /// under one a frame).
    pub spark_carry: f32,
}

impl Jak {
    /// A block edit may split, remove or reorder the nearby rails. A ride
    /// belongs to its geometric segment, never to the old array index.
    pub fn replace_grind_rails(&mut self, rails: Arc<Vec<Rail>>) {
        let replacement = self.grind.ride.and_then(|ride| {
            let old = self.grind.rails.get(ride.rail)?.points.get(ride.segment..ride.segment + 2)?;
            let same = |a: Vec3, b: Vec3| a.distance_squared(b) <= meters(0.0001).powi(2);
            for (rail, line) in rails.iter().enumerate() {
                for (segment, pair) in line.points.windows(2).enumerate() {
                    let reversed = same(old[0], pair[1]) && same(old[1], pair[0]);
                    if reversed || (same(old[0], pair[0]) && same(old[1], pair[1])) {
                        return Some(Ride { rail, segment,
                            distance: if reversed { pair[0].distance(pair[1]) - ride.distance } else { ride.distance },
                            direction: if reversed { -ride.direction } else { ride.direction }, ..ride });
                    }
                }
            }
            None
        });
        let lost = self.grind.ride.is_some() && replacement.is_none();
        self.grind.rails = rails;
        self.grind.ride = replacement;
        if lost { self.grind_lost_support(); }
    }

    fn grind_lost_support(&mut self) {
        self.grind.ride = None;
        self.grind.cooldown_until = self.time + seconds(0.3);
        self.control.status &= !(status::ON_GROUND | status::ON_SURFACE | status::TOUCH_SURFACE);
        // Mining the support is a fall. The deliberate hop in grind_leave
        // would put Jak through a low ceiling and disguise the removed rail.
        if self.state == State::BoardGrind { self.go(State::BoardFalling); }
    }

    fn grind_clear_path(&mut self, world: &mut dyn CollideWorld, to: Vec3) -> bool {
        let motion = to - self.trans();
        for height in self.control.standing_heights {
            let start = self.trans() + Vec3::Y * height;
            self.probe_cache
                .fill_line_sphere(world, start, motion, self.control.body_radius);
            if self
                .probe_cache
                .probe_line_sphere(start, motion, self.control.body_radius)
                .is_some()
            {
                return false;
            }
        }
        true
    }

    pub(crate) fn grind_attach(&mut self, previous: Vec3, world: &mut dyn CollideWorld) {
        if self.grind.ride.is_some()
            || self.time < self.grind.cooldown_until
            || !matches!(
                self.state,
                State::BoardStance
                    | State::BoardFalling
                    | State::BoardJump { .. }
                    | State::BoardHitGround
            )
            || self.velocity().length() < MIN_SPEED
            || self.velocity().y > meters(1.0)
        {
            return;
        }
        let mut best = None;
        let mut best_distance = CAPTURE_RADIUS * CAPTURE_RADIUS;
        for (rail_index, rail) in self.grind.rails.iter().enumerate() {
            for (segment, pair) in rail.points.windows(2).enumerate() {
                let edge = pair[1] - pair[0];
                let length = edge.length();
                let tangent = edge / length;
                let along = self.velocity().dot(tangent);
                if along.abs() < MIN_SPEED || along.abs() < self.velocity().length() * 0.5 {
                    continue;
                }
                for sample in 0..=4 {
                    let point = previous.lerp(self.trans(), sample as f32 / 4.0);
                    let distance = (point - pair[0]).dot(tangent).clamp(0.0, length);
                    let rail_point = pair[0] + tangent * distance + Vec3::Y * LIFT;
                    let separation = point.distance_squared(rail_point);
                    if separation < best_distance {
                        best_distance = separation;
                        best = Some((
                            Ride {
                                rail: rail_index,
                                segment,
                                distance,
                                direction: along.signum(),
                                speed: along.abs(),
                            },
                            rail_point,
                        ));
                    }
                }
            }
        }
        let Some((ride, point)) = best else { return };
        if !self.grind_clear_path(world, point) {
            return;
        }
        self.go(State::BoardGrind);
        self.grind.ride = Some(ride);
        self.board.ride_time = self.time;
        self.control.trans = point;
        self.control.draw_offset_y = meters(0.1);
        self.ja_set(
            anim::BOARD_RIDE_TURN_FRONT,
            anim::NumFunc::Identity,
            self.anims.max(anim::BOARD_RIDE_TURN_FRONT) * 0.5,
        );
    }

    fn grind_leave(&mut self, jumping: bool) {
        self.grind.ride = None;
        self.grind.cooldown_until = self.time + seconds(0.3);
        self.control.status &= !(status::ON_GROUND | status::ON_SURFACE | status::TOUCH_SURFACE);
        self.control.trans.y += meters(0.3);
        if jumping {
            self.pad.clear(button::X);
            self.go(State::BoardJump {
                min: board::JUMP_HEIGHT_MIN,
                max: board::JUMP_HEIGHT_MAX,
                duck: false,
            });
        } else {
            self.go(State::BoardFalling);
        }
    }

    pub(crate) fn grind_step(&mut self, input: &PadInput, world: &mut dyn CollideWorld) -> bool {
        let Some(mut ride) = self.grind.ride else {
            return false;
        };
        if self.state != State::BoardGrind {
            self.grind.ride = None;
            return false;
        }
        if self.pad.pressed(button::X) {
            self.grind_leave(true);
            return false;
        }
        if self
            .pad
            .pressed(button::R2 | button::SQUARE | button::CIRCLE)
        {
            self.grind_leave(false);
            return false;
        }
        let Some(rail) = self.grind.rails.get(ride.rail).filter(|r| ride.segment + 1 < r.points.len()) else {
            self.grind_lost_support();
            return false;
        };
        let tangent = (rail.points[ride.segment + 1] - rail.points[ride.segment]).normalize();
        let brake = if input.left[1] < -0.3 {
            meters(20.0)
        } else {
            meters(0.4)
        };
        ride.speed = (ride.speed
            - (brake + tangent.y * ride.direction * self.control.gravity_length)
                * SECONDS_PER_FRAME)
            .clamp(0.0, board::MAX_SPEED);
        ride.distance += ride.speed * ride.direction * SECONDS_PER_FRAME;
        let point = loop {
            let a = rail.points[ride.segment];
            let edge = rail.points[ride.segment + 1] - a;
            let length = edge.length();
            if ride.distance > length {
                if ride.segment + 2 >= rail.points.len() {
                    break None;
                }
                ride.distance -= length;
                ride.segment += 1;
            } else if ride.distance < 0.0 {
                if ride.segment == 0 {
                    break None;
                }
                ride.segment -= 1;
                ride.distance += rail.points[ride.segment].distance(rail.points[ride.segment + 1]);
            } else {
                break Some((
                    a + edge / length * ride.distance + Vec3::Y * LIFT,
                    edge / length,
                ));
            }
        };
        let Some((point, tangent)) = point else {
            self.grind_leave(false);
            return false;
        };
        if ride.speed < MIN_SPEED * 0.2 || !self.grind_clear_path(world, point) {
            self.grind_leave(false);
            return false;
        }
        self.grind.ride = Some(ride);
        self.control.trans = point;
        self.control.transv = tangent * ride.speed * ride.direction;
        self.control.quat = forward_up_nopitch_quat(tangent * ride.direction, Vec3::Y);
        self.control.dir_targ = self.control.quat;
        self.control.status = status::ON_SURFACE | status::ON_GROUND;
        self.control.log_trans(self.time);
        self.control.last_time_on_surface = self.time;
        self.control.list_time_on_ground = self.time;
        self.control.surface_normal = Vec3::Y;
        self.build_conversions();
        self.chan.eval_with(&self.anims, anim::NumFunc::Identity);
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn line(x: f32) -> Rail { Rail::new(vec![Vec3::new(meters(x), 0.0, 0.0), Vec3::new(meters(x + 1.0), 0.0, 0.0)]).unwrap() }
    fn rider() -> Jak {
        let mut j = Jak::new(Vec3::new(meters(0.4), LIFT, 0.0), 0.0);
        j.state = State::BoardGrind;
        j.control.transv = Vec3::X * meters(5.0);
        j.grind.rails = Arc::new(vec![line(0.0), line(10.0)]);
        j.grind.ride = Some(Ride { rail: 0, segment: 0, distance: meters(0.4), direction: 1.0, speed: meters(5.0) });
        j
    }
    #[test]
    fn a_sub_centimetre_cell_cut_does_not_discard_a_real_rail() {
        let points=vec![Vec3::ZERO,Vec3::X*meters(0.0001),Vec3::X*meters(1.0)];
        assert_eq!(Rail::new(points.clone()).unwrap().points,points);
        assert!(Rail::new(vec![Vec3::ZERO,Vec3::ZERO]).is_none());
    }
    #[test]
    fn replacing_rails_preserves_the_ridden_segment_after_reordering_and_reversal() {
        let mut j = rider(); let p = j.trans(); let v = j.velocity();
        let mut reversed = line(0.0); reversed.points.reverse();
        j.replace_grind_rails(Arc::new(vec![line(10.0), reversed]));
        let ride = j.grind.ride.unwrap();
        assert_eq!(ride.rail, 1); assert_eq!(ride.direction, -1.0);
        assert!((ride.distance - meters(0.6)).abs() < 0.01);
        assert_eq!(j.trans(), p); assert_eq!(j.velocity(), v);
        assert!(j.grind_step(&PadInput::default(), &mut crate::collide::EmptyWorld));
        assert!(j.trans().x > p.x && j.trans().x < p.x + meters(0.1));
    }
    #[test]
    fn mining_the_ridden_segment_releases_without_position_or_velocity_jump() {
        let mut j = rider(); let p = j.trans(); let v = j.velocity();
        j.replace_grind_rails(Arc::new(vec![line(10.0)]));
        assert!(j.grind.ride.is_none()); assert_eq!(j.state, State::BoardFalling);
        assert_eq!(j.trans(), p); assert_eq!(j.velocity(), v);
        // Defensive handling also protects callers which supplied stale indices.
        j.state = State::BoardGrind;
        j.grind.ride = Some(Ride { rail: 99, segment: 0, distance: 0.0, direction: 1.0, speed: meters(5.0) });
        assert!(!j.grind_step(&PadInput::default(), &mut crate::collide::EmptyWorld));
        assert_eq!(j.trans(), p);
    }
}
