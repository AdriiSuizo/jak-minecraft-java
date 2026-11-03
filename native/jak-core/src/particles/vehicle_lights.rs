//! Vehicle-method-78 and the original per-model light attachment points.
use super::{Sexp, keyword, parse};
use glam::Vec3;
use std::collections::HashMap;
#[derive(Clone, Debug)]
pub struct Specs {
    pub heads: Vec<Vec3>,
    pub tails: Vec<Vec3>,
    pub damage_stages: usize,
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
            let n = keyword(p, count).and_then(Sexp::number).unwrap_or(0.0) as usize;
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
        out.insert(
            model.into(),
            Specs {
                heads: points(":headlight-local-pos", ":headlight-count"),
                tails: points(":taillight-local-pos", ":taillight-count"),
                damage_stages: stages(body).max(1),
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
