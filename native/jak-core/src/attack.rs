//! Jak's attacks as the world feels them. A state that attacks starts an
//! attack and sets its danger; while the danger lasts, its spheres around
//! Jak strike whatever they touch, each target once per attack, with the
//! damage the kind of attack does. The host decides what a struck target
//! is (a mob, a block, later Jak's own enemies).
use glam::Vec3;

use crate::collide::CollideWorld;
use crate::projectile::ActorWorld;
use crate::{Event, Jak};

/// What makes Jak dangerous right now.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Danger {
    Spin,
    SpinAir,
    Punch,
    Uppercut,
    Flop,
    FlopDown,
    /// Changing into Light Jak.
    GetOn,
    /// The Flut Flut's attacks (`'flut-attack`).
    Flut,
    /// What its air attack's landing strikes round it (its touch-tracker,
    /// grown to 7 m).
    FlutGround,
}

/// A sphere the attack strikes with: at an offset from Jak's origin in his
/// own frame, or on one of his joints.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct HitSphere {
    pub offset: Vec3,
    pub radius: f32,
    pub joint: Option<usize>,
}

const fn at(y: f32, radius: f32) -> HitSphere {
    HitSphere {
        offset: Vec3::new(0.0, y, 0.0),
        radius,
        joint: None,
    }
}

const fn on(joint: usize, radius: f32) -> HitSphere {
    HitSphere {
        offset: Vec3::ZERO,
        radius,
        joint: Some(joint),
    }
}

const SPIN: [HitSphere; 1] = [at(6553.6, 9011.2)];
const PUNCH: [HitSphere; 2] = [at(5324.8, 5324.8), on(27, 5324.8)];
const UPPERCUT: [HitSphere; 3] = [at(3276.8, 4096.0), on(27, 4096.0), at(9011.2, 4096.0)];
const FLOP: [HitSphere; 1] = [at(3276.8, 5734.4)];
const FLOP_DOWN: [HitSphere; 2] = [at(3276.8, 5734.4), at(9011.2, 5734.4)];
const FLUT: [HitSphere; 2] = [at(6553.6, 12288.0), at(6553.6, 7372.8)];
const FLUT_GROUND: [HitSphere; 1] = [at(4096.0, 28672.0)];

impl Danger {
    pub fn spheres(self) -> &'static [HitSphere] {
        match self {
            Danger::Spin | Danger::SpinAir | Danger::GetOn => &SPIN,
            Danger::Punch => &PUNCH,
            Danger::Uppercut => &UPPERCUT,
            Danger::Flop => &FLOP,
            Danger::FlopDown => &FLOP_DOWN,
            Danger::Flut => &FLUT,
            Danger::FlutGround => &FLUT_GROUND,
        }
    }

    /// Hit points it takes off what it strikes.
    pub fn damage(self) -> f32 {
        match self {
            Danger::Spin | Danger::SpinAir | Danger::Punch | Danger::GetOn | Danger::Flut | Danger::FlutGround => 3.0,
            Danger::Uppercut | Danger::Flop | Danger::FlopDown => 2.0,
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Danger::Spin => "spin",
            Danger::SpinAir => "spin-air",
            Danger::Punch => "punch",
            Danger::Uppercut => "uppercut",
            Danger::Flop => "flop",
            Danger::FlopDown => "flop-down",
            Danger::GetOn => "get-on",
            Danger::Flut => "flut-attack",
            Danger::FlutGround => "flut-ground",
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Attack {
    pub danger: Option<Danger>,
    /// Counts attacks: a target is struck once per attack.
    pub id: u32,
    pub struck: Vec<u64>,
    pub struck_world: bool,
    /// The spheres striking this frame, world space.
    pub live: Vec<(Vec3, f32)>,
}

impl Jak {
    pub(crate) fn start_attack(&mut self) {
        let a = &mut self.attack;
        a.id = a.id.wrapping_add(1);
        a.struck.clear();
        a.struck_world = false;
    }

    pub(crate) fn danger_set(&mut self, danger: Option<Danger>) {
        self.attack.danger = danger;
    }

    /// The danger's spheres this frame, and what they strike.
    pub fn attack_post<'a>(
        &mut self,
        world: &mut dyn CollideWorld,
        mut actors: Option<&mut (dyn ActorWorld + 'a)>,
    ) {
        self.attack.live.clear();
        let Some(danger) = self.attack.danger else {
            return;
        };
        let forward = crate::math::z_axis(self.control.quat);
        for sphere in danger.spheres() {
            let center = match sphere.joint {
                Some(j) => match self.joints.get(j) {
                    Some(&local) => self.control.trans + self.control.quat * local,
                    None => continue,
                },
                None => self.control.trans + self.control.quat * sphere.offset,
            };
            self.attack.live.push((center, sphere.radius));
            if let Some(actors) = actors.as_deref_mut() {
                for key in actors.actors_touching(center, sphere.radius) {
                    if self.attack.struck.contains(&key) {
                        continue;
                    }
                    // Test from the torso, not the hand: an animated fist can
                    // itself protrude through a wall. Use the actual hurtbox's
                    // nearest point so a low obstacle does not hide a tall mob.
                    let origin = self.control.trans + self.control.gravity_normal * crate::METER;
                    let contact = actors.actor_contact(key, origin).unwrap_or(center);
                    if crate::collide::fill_and_probe_line_sphere(
                        world, &mut self.probe_cache, origin, contact - origin, 0.01 * crate::METER,
                    ).is_some_and(|hit| hit.u < 0.999) {
                        continue;
                    }
                    self.attack.struck.push(key);
                    self.events.push(Event::Strike {
                        actor: Some(key),
                        pos: center,
                        dir: forward,
                        damage: danger.damage(),
                    });
                }
            }
            if !self.attack.struck_world {
                let r = Vec3::splat(sphere.radius);
                self.probe_cache.fill_box(world, center - r, center + r);
                if self.probe_cache.overlaps_sphere(center, sphere.radius) {
                    self.attack.struck_world = true;
                    self.events.push(Event::Strike {
                        actor: None,
                        pos: center + forward * sphere.radius,
                        dir: forward,
                        damage: danger.damage(),
                    });
                }
            }
        }
    }
}
