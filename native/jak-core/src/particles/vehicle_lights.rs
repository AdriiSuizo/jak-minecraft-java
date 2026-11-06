//! Vehicle-method-78 and the original per-model light attachment points.
use super::{Sexp, keyword, parse};
use glam::Vec3;
use std::collections::HashMap;
#[derive(Clone, Debug, Default)]
pub struct Specs {
    pub heads: Vec<Vec3>,
    pub tails: Vec<Vec3>,
    pub damage_stages: usize,
    pub thrusters: Vec<Vec3>,
    pub exhaust: Vec<Vec3>,
    /// `exhaust-local-dir`, `smoke-local-pos` and `smoke-local-vel` (vehicle-method-78), in Jak units.
    pub exhaust_dir: Vec<Vec3>,
    pub smoke_pos: Vec<Vec3>,
    pub smoke_vel: Vec<Vec3>,
    /// `thruster-flame-width` and `thruster-flame-length` (units), and the handling's `max-engine-thrust` (units a second).
    pub flame_width: f32,
    pub flame_length: f32,
    pub max_engine_thrust: f32,
    /// The rigid body (`rigid-body-info`) and what hvehicle-physics's roll reads of the constants: mass, gravity (units a
    /// second squared), `inertial-tensor-box` (units), `cm-offset-joint`, `angular-damping`, `roll-control-factor`,
    /// `player-weight` (force), `player-shift-x` and the roll thrusters (`roll-thruster-array`: local position and `rot`).
    pub mass: f32,
    pub gravity: f32,
    pub inertial_box: Vec3,
    pub cm_offset: Vec3,
    pub angular_damping: f32,
    pub roll_control: f32,
    pub player_weight: f32,
    pub player_shift_x: f32,
    pub roll_thrusters: Vec<(Vec3, Vec3)>,
    /// `h-bike-base`'s `vehicle-method-78` draws a flame at each roll thruster that fires; the cars have none.
    pub roll_flame: bool,
    /// The rider's place: `seat-array`'s first `position`, and the two `rider-hand-offset`s that `vehicle-method-113`
    /// puts the pilot's hands at (target-pilot-post's arm IK): from the seat on a bike, from the steering wheel's joint on a car.
    pub seat: Vec3,
    pub hand_offsets: Vec<Vec3>,
    pub masks: [u64; 3],
}
/// The first value anywhere under `items` that follows `key` (the handling is nested in the constants).
fn find<'a>(items: &'a [Sexp], key: &str) -> Option<&'a Sexp> {
    if let Some(found) = keyword(items, key) {
        return Some(found);
    }
    items.iter().find_map(|item| match item {
        Sexp::List(inner) => find(inner, key),
        _ => None,
    })
}
pub fn read(source: &str) -> HashMap<String, Specs> {
    let mut out = HashMap::new();
    for form in parse(source) {
        let Sexp::List(form) = form else { continue };
        if form.first().and_then(Sexp::symbol) != Some("define") {
            continue;
        }
        let Some(name) = form.get(1).and_then(Sexp::symbol) else {
            continue;
        };
        let model = match name {
            "*v-toad-constants*" => "toad",
            "*v-snake-constants*" => "snake",
            "*h-bike-a-constants*" => "bikea",
            "*h-bike-b-constants*" => "bikeb",
            "*h-bike-c-constants*" => "bikec",
            "*h-car-a-constants*" => "cara",
            "*h-car-b-constants*" => "carb",
            "*h-car-c-constants*" => "carc",
            "*h-hellcat-constants*" | "*helldog-constants*" => "hellcat",
            _ => continue,
        };
        let Some(Sexp::List(body)) = form.get(2) else {
            continue;
        };
        // Jak 3 nests these in :particles; Jak II's Hell Dog stores the
        // same authored attachments directly on its vehicle constants.
        let p = match keyword(body, ":particles") {Some(Sexp::List(p))=>p,_=>body};
        let points = |key: &str, count: &str| -> Vec<Vec3> {
            let n = if count.is_empty() { 2 } else { keyword(p, count).and_then(Sexp::number).unwrap_or(0.) as usize };
            let Some(Sexp::List(values)) = keyword(p, key) else {
                return Vec::new();
            };
            values
                .iter()
                .filter_map(|v| match v {
                    Sexp::List(v) => Some(Vec3::new(
                        keyword(v, ":x").and_then(Sexp::number).unwrap_or(0.),
                        keyword(v, ":y").and_then(Sexp::number).unwrap_or(0.),
                        keyword(v, ":z").and_then(Sexp::number).unwrap_or(0.),
                    )),
                    _ => None,
                })
                .take(n)
                .collect()
        };
        fn stages(items: &[Sexp]) -> usize {
            let here = keyword(items, ":damage-seg-count")
                .and_then(Sexp::number)
                .unwrap_or(1.) as usize;
            items
                .iter()
                .filter_map(|v| match v {
                    Sexp::List(v) => Some(stages(v)),
                    _ => None,
                })
                .fold(here, usize::max)
        }
        fn collect_masks(items: &[Sexp], output: &mut [u64; 3]) {
            if let Some(Sexp::List(parts)) = keyword(items, ":damage-seg-array") {
                for (index, value) in parts.iter().skip(5).take(3).enumerate() {
                    if let Some(mask) = value.number() {
                        output[index] |= mask.max(0.) as u64;
                    }
                }
            }
            for value in items {
                if let Sexp::List(values) = value { collect_masks(values, output); }
            }
        }
        let mut masks = [0;3];
        collect_masks(body, &mut masks);
        let number = |key: &str| find(body, key).and_then(Sexp::number).unwrap_or(0.);
        // `(new 'static 'vector :x .. :y .. :z ..)`
        let vector = |at: &Sexp| match at {
            Sexp::List(v) => Vec3::new(
                keyword(v, ":x").and_then(Sexp::number).unwrap_or(0.),
                keyword(v, ":y").and_then(Sexp::number).unwrap_or(0.),
                keyword(v, ":z").and_then(Sexp::number).unwrap_or(0.),
            ),
            _ => Vec3::ZERO,
        };
        let inertial_box = match find(body, ":inertial-tensor-box") {
            Some(Sexp::List(items)) => {
                let sizes: Vec<f32> = items.iter().filter_map(|i| matches!(i, Sexp::List(_)).then(|| i.number()).flatten()).collect();
                Vec3::new(*sizes.first().unwrap_or(&0.), *sizes.get(1).unwrap_or(&0.), *sizes.get(2).unwrap_or(&0.))
            }
            _ => Vec3::ZERO,
        };
        let hand_offsets: Vec<Vec3> = match find(body, ":rider-hand-offset") {
            Some(Sexp::List(items)) => items.iter().filter(|i| matches!(i, Sexp::List(_))).map(&vector).take(2).collect(),
            _ => Vec::new(),
        };
        let seat = match find(body, ":seat-array") {
            Some(Sexp::List(items)) => items
                .iter()
                .find_map(|item| match item {
                    Sexp::List(seat) => keyword(seat, ":position").map(&vector),
                    _ => None,
                })
                .unwrap_or(Vec3::ZERO),
            _ => Vec3::ZERO,
        };
        let roll_count = number(":roll-thruster-count") as usize;
        let roll_thrusters: Vec<(Vec3, Vec3)> = match find(body, ":roll-thruster-array") {
            Some(Sexp::List(items)) => items
                .iter()
                .filter_map(|item| match item {
                    Sexp::List(point) => Some((
                        keyword(point, ":local-pos").map_or(Vec3::ZERO, &vector),
                        keyword(point, ":rot").map_or(Vec3::ZERO, &vector),
                    )),
                    _ => None,
                })
                .take(roll_count)
                .collect(),
            _ => Vec::new(),
        };
        out.insert(
            model.into(),
            Specs {
                heads: points(":headlight-local-pos", ":headlight-count"),
                tails: points(":taillight-local-pos", ":taillight-count"),
                damage_stages: stages(body).max(1),
                thrusters: points(":thruster-local-pos", ""),
                exhaust: points(":exhaust-local-pos", ""),
                exhaust_dir: points(":exhaust-local-dir", ""),
                smoke_pos: points(":smoke-local-pos", ""),
                smoke_vel: points(":smoke-local-vel", ""),
                flame_width: keyword(p, ":thruster-flame-width").and_then(Sexp::number).unwrap_or(0.),
                flame_length: keyword(p, ":thruster-flame-length").and_then(Sexp::number).unwrap_or(0.),
                max_engine_thrust: find(body, ":max-engine-thrust").and_then(Sexp::number).unwrap_or(0.),
                mass: number(":mass"),
                gravity: number(":gravity"),
                inertial_box,
                cm_offset: find(body, ":cm-offset-joint").map_or(Vec3::ZERO, &vector),
                angular_damping: number(":angular-damping"),
                roll_control: number(":roll-control-factor"),
                player_weight: number(":player-weight"),
                player_shift_x: number(":player-shift-x"),
                roll_thrusters,
                roll_flame: model.starts_with("bike"),
                seat,
                hand_offsets,
                masks: masks.map(|mask|mask|1),
            },
        );
    }
    out
}
#[derive(Clone, Debug, Default)]
pub struct State {
    pub factor: f32,
    pub dead: bool,
    pub on: bool,
    pub last_time: Option<f64>,
    clock: f64,
    frame: u64,
}
impl State {
    /// Lights-dead survives partial healing. A complete repair restores the
    /// lenses, as vehicle-method-76 does. Returns the headlight-pop event.
    pub fn step(
        &mut self,
        dt: f32,
        hour: f32,
        riding: bool,
        stage: usize,
        fully_repaired: bool,
        key: u64,
    ) -> bool {
        let broken = stage > 0 && !self.dead;
        if stage > 0 {
            self.dead = true;
        } else if fully_repaired {
            self.dead = false;
        }
        self.clock += f64::from(dt.max(0.).min(0.25)) * 60.;
        while self.clock >= 1. - 1e-6 {
            self.clock = (self.clock - 1.).max(0.);
            self.frame += 1;
            if self.frame & 31 == key & 31 {
                let hour = hour + 0.048387095 * (key & 31) as f32;
                self.on = riding && (hour < 7. || hour > 19.) && !self.dead;
            }
        }
        if self.dead {
            self.on = false;
        }
        let wanted = if self.on { 1.0 } else { 0.0 };
        self.factor += (wanted - self.factor).clamp(-2. * dt, 2. * dt);
        broken
    }
}
pub fn far_fade(metres: f32) -> f32 {
    (-0.0000122044 * metres * 4096. + 8.997864).clamp(0., 1.)
}
pub fn near_fade(metres: f32) -> f32 {
    (-0.00001356 * metres * 4096. + 2.3332994).clamp(0., 1.)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn night_riding_damage_and_full_repair_follow_original_lamp_flags() {
        for fps in [30., 60., 120.] {
            let mut s = State::default();
            for _ in 0..(fps * 2.) as usize {
                s.step(1. / fps, 22., true, 0, true, 7);
            }
            assert_eq!(s.factor, 1.);
            assert!(s.step(1. / fps, 22., true, 1, false, 7));
            assert!(!s.on && s.dead);
            for _ in 0..(fps * 2.) as usize {
                s.step(1. / fps, 22., true, 0, false, 7);
            }
            assert_eq!(s.factor, 0., "partial healing cannot mend a broken light");
            for _ in 0..(fps * 2.) as usize {
                s.step(1. / fps, 22., true, 0, true, 7);
            }
            assert_eq!(s.factor, 1.);
            for _ in 0..(fps * 2.) as usize {
                s.step(1. / fps, 12., true, 0, true, 7);
            }
            assert_eq!(s.factor, 0.);
            for _ in 0..(fps * 2.) as usize {
                s.step(1. / fps, 22., false, 0, true, 7);
            }
            assert_eq!(s.factor, 0., "unoccupied vehicle fades its lights out");
        }
    }
    #[test]
    fn original_distance_fades_keep_near_lenses_and_far_glows_distinct() {
        assert_eq!(far_fade(150.), 1.);
        assert_eq!(far_fade(181.), 0.);
        assert_eq!(near_fade(20.), 1.);
        assert_eq!(near_fade(43.), 0.);
    }
}
