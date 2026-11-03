//! Jak 3's lightning of `lightning-spec`s (`lightning.gc`, its
//! `lightning-control` driven by a `lightning-tracker` of `generic-obs.gc`):
//! what Dark Jak's shocks, the guns' shocks and the Peacemaker's tendrils
//! draw. Not the Arc Wielder's own rope (`lightning-new.gc`, its
//! `lightning-bolt`: `crate::lightning_bolt`).
//!
//! - A bolt is `num-points` points from its first end to its last; every
//!   `merge-count` frames its target points (`meet`) are drawn anew
//!   (`lightning-fractal-gen`, or `lightning-uniform-gen`, inside its
//!   `box-size`) and every frame the drawn points (`line`) close
//!   `merge-factor` of the way to them (`lightning-update`). With `lsf4` the
//!   box grows with the bolt's length (to five times, over
//!   `adjust-distance` metres).
//! - Its tracker keeps its ends on what they hang on, lasts its `duration`
//!   (and up to `duration-rand` more), then fades `fade-time` toward
//!   `fade-to-color` (`lightning-fade`), and goes. With
//!   `lightning-probe-callback` (every shock effect's), on its second frame
//!   it takes a colour of its own: grey from 64 to 128, blue 128, alpha 32
//!   to 64.
//! - It is drawn as two strips through its points facing the camera, one
//!   widened `radius` each way along the camera's x, the other along its y
//!   (`lightning-draw`); its texture laid once along the whole bolt, across
//!   from 0 to 1; its colour by `lsf0` from the first colour at its start to
//!   the last at its end (`lsf1`: the first all along but the end; `lsf2`:
//!   the last at both ends); added over what is behind.
use super::{Sexp, keyword, random};
use crate::lightning_bolt::{Strip, StripVertex};
use glam::{Vec3, Vec4};
use std::collections::HashMap;

/// `lightning-spec-flags`.
const LSF0: u16 = 1;
const LSF1: u16 = 1 << 1;
const LSF2: u16 = 1 << 2;
const LSF4: u16 = 1 << 4;

#[derive(Clone, Debug)]
pub(super) struct Spec {
    pub texture: u32,
    flags: u16,
    rand_func: u8,
    adjust_distance: f32,
    points: usize,
    size: f32,
    reduction: f32,
    merge: f32,
    count: f32,
    radius: f32,
    duration: f32,
    duration_rand: f32,
    fade_time: f32,
    fade_start_factor: f32,
    start: Vec4,
    end: Vec4,
    fade: Vec4,
}

pub(super) fn read(forms: &[Sexp], specs: &mut HashMap<String, Spec>) {
    for form in forms {
        let Sexp::List(items) = form else { continue };
        if items.first().and_then(Sexp::symbol) == Some("new")
            && items.iter().any(|v| v.symbol() == Some("lightning-spec"))
        {
            if let Some(name) = keyword(items, ":name").and_then(Sexp::symbol) {
                let num = |key, default| {
                    keyword(items, key)
                        .and_then(Sexp::number)
                        .unwrap_or(default)
                };
                let colour = |key| -> Option<Vec4> {
                    let Sexp::List(values) = keyword(items, key)? else {
                        return None;
                    };
                    let n = |field| keyword(values, field).and_then(Sexp::number).unwrap_or(0.0);
                    Some(Vec4::new(n(":r"), n(":g"), n(":b"), n(":a")))
                };
                // `(lightning-spec-flags lsf0 lsf4)`.
                let flags = match keyword(items, ":flags") {
                    Some(Sexp::List(values)) => values
                        .iter()
                        .filter_map(Sexp::symbol)
                        .filter_map(|s| s.strip_prefix("lsf")?.parse::<u16>().ok())
                        .fold(0u16, |bits, n| bits | (1 << n)),
                    _ => 0,
                };
                if let Some(Sexp::List(texture)) = keyword(items, ":texture") {
                    let page = keyword(texture, ":page")
                        .and_then(Sexp::number)
                        .unwrap_or(0.0) as u32;
                    let index = keyword(texture, ":index")
                        .and_then(Sexp::number)
                        .unwrap_or(0.0) as u32;
                    specs.insert(
                        name.trim_matches('"').to_owned(),
                        Spec {
                            texture: (page << 20) | (index << 8),
                            flags,
                            rand_func: num(":rand-func", 0.0) as u8,
                            adjust_distance: num(":adjust-distance", 0.0),
                            points: (num(":num-points", 2.0) as usize).clamp(2, 128),
                            size: num(":box-size", 0.0),
                            reduction: num(":reduction", 0.0),
                            merge: num(":merge-factor", 0.0),
                            count: num(":merge-count", 0.0),
                            radius: num(":radius", 0.0),
                            duration: num(":duration", 0.0),
                            duration_rand: num(":duration-rand", 0.0),
                            fade_time: num(":fade-time", 0.0),
                            fade_start_factor: num(":fade-start-factor", 0.0),
                            start: colour(":start-color").unwrap_or(Vec4::ZERO),
                            end: colour(":end-color").unwrap_or(Vec4::ZERO),
                            fade: colour(":fade-to-color").unwrap_or(Vec4::ZERO),
                        },
                    );
                }
            }
        }
        read(items, specs);
    }
}

/// A bolt's `duration` (`lightning-tracker-init`: `(the int (rand-vu-float-
/// range duration (+ duration duration-rand)))` ticks at 300 Hz), from its id
/// alone, and the seed it goes on with.
fn duration_of(spec: &Spec, id: u64) -> (f32, u32) {
    let mut seed = (id as u32 ^ (id >> 32) as u32) | 1;
    ((spec.duration + random(&mut seed) * spec.duration_rand).trunc(), seed)
}

/// How long bolt `id` of `spec` lasts, and that with its fade, in ticks at
/// 300 Hz.
pub(super) fn life_ticks(spec: &Spec, id: u64) -> (f32, f32) {
    let duration = duration_of(spec, id).0;
    (duration, duration + spec.fade_time.max(0.0))
}

/// One bolt asked for: its id (the same each frame it lasts), its spec, its
/// ends as they are now, how long it has been out (seconds), and whether
/// its tracker runs `lightning-probe-callback`.
#[derive(Clone, Debug)]
pub struct LightningRequest {
    pub id: u64,
    pub spec: &'static str,
    pub from: Vec3,
    pub to: Vec3,
    pub age: f32,
    pub probe: bool,
}

#[derive(Clone, Debug)]
pub(super) struct Bolt {
    pub request: LightningRequest,
    /// `line`: the points drawn; `meet`: where they head.
    line: Vec<Vec3>,
    meet: Vec<Vec3>,
    counter: f32,
    box_size: f32,
    seed: u32,
    duration: f32,
    life: f32,
    start_color: Vec4,
    end_color: Vec4,
    /// `user-time 0` of `lightning-probe-callback`: its frames so far.
    probe_frames: u32,
}

/// The `rand-uint31-gen` share the generators take: from -0.5 to 0.5.
fn jitter(seed: &mut u32) -> f32 {
    random(seed) - 0.5
}

/// `lightning-fractal-gen`: the middle of each span set at its ends' middle
/// thrown up to half `size` each way, and the halves the same at
/// `reduction` of it.
fn fractal(points: &mut [Vec3], first: usize, last: usize, size: f32, reduction: f32, seed: &mut u32) {
    if last <= first + 1 {
        return;
    }
    let mid = (first + last) / 2;
    let (a, b) = (points[first], points[last]);
    points[mid] = (a + b) * 0.5 + Vec3::new(jitter(seed), jitter(seed), jitter(seed)) * size;
    fractal(points, first, mid, size * reduction, reduction, seed);
    fractal(points, mid, last, size * reduction, reduction, seed);
}

/// `lightning-uniform-gen`: every point, its ends among them, on the line
/// between the ends thrown up to a fifth of `size` each way.
fn uniform(points: &mut [Vec3], size: f32, seed: &mut u32) {
    let n = points.len();
    let (a, b) = (points[0], points[n - 1]);
    for (i, p) in points.iter_mut().enumerate() {
        let t = i as f32 / (n - 1) as f32;
        *p = a.lerp(b, t) + Vec3::new(jitter(seed), jitter(seed), jitter(seed)) * 0.4 * size;
    }
}

impl Bolt {
    pub fn reframe(&mut self, frame: &crate::reframe::Frame) {
        self.request.from = frame.point(self.request.from);
        self.request.to = frame.point(self.request.to);
        for point in self.line.iter_mut().chain(&mut self.meet) {
            *point = frame.point(*point);
        }
    }

    /// `new lightning-control` (its distance 0: `num-points` points), then
    /// its first `lightning-update`: the ends set, the points drawn and laid
    /// straight on them.
    pub fn new(request: LightningRequest, spec: &Spec) -> Self {
        let (duration, seed) = duration_of(spec, request.id);
        let n = spec.points;
        let mut bolt = Self {
            meet: vec![Vec3::ZERO; n],
            line: Vec::new(),
            counter: 0.0,
            box_size: spec.size,
            seed,
            duration,
            life: duration + spec.fade_time.max(0.0),
            start_color: spec.start,
            end_color: spec.end,
            probe_frames: 0,
            request,
        };
        bolt.ends();
        bolt.adjust_box(spec);
        bolt.generate(spec);
        bolt.line.clone_from(&bolt.meet);
        bolt.counter = spec.count;
        bolt
    }

    /// The tracker's `update`: its ends where they hang.
    fn ends(&mut self) {
        let last = self.meet.len() - 1;
        self.meet[0] = self.request.from;
        self.meet[last] = self.request.to;
    }

    /// `lsf4`: the box by the bolt's length, from once to five times over
    /// `adjust-distance` metres.
    fn adjust_box(&mut self, spec: &Spec) {
        if spec.flags & LSF4 != 0 {
            let length = self.meet[0].distance(self.meet[self.meet.len() - 1]);
            let over = spec.adjust_distance.max(1.0) * 4096.0;
            self.box_size = spec.size * (length / over).clamp(1.0, 5.0);
        }
    }

    fn generate(&mut self, spec: &Spec) {
        let last = self.meet.len() - 1;
        match spec.rand_func {
            1 => uniform(&mut self.meet, self.box_size, &mut self.seed),
            // 0, and the trails' (2, 3: along a path of points of their
            // own, which none of the specs read here asks for).
            _ => fractal(&mut self.meet, 0, last, self.box_size, spec.reduction, &mut self.seed),
        }
    }

    /// Ticks out (300 Hz).
    fn ticks(&self) -> f32 {
        self.request.age * 300.0
    }

    /// A frame of `dt` seconds: while it lasts its tracker's `update` (the
    /// probe callback, its ends) and `lightning-update`; fading,
    /// `lightning-fade`'s colours.
    pub fn step(&mut self, dt: f32, spec: &Spec) {
        let ratio = dt * 60.0;
        let ticks = self.ticks();
        if ticks < self.duration {
            if self.request.probe {
                // `lightning-probe-callback`: on `(seconds 0.007)`, its
                // second, a colour of its own.
                self.probe_frames += 1;
                if self.probe_frames == 2 {
                    let grey = 64.0 + (random(&mut self.seed) * 65.0).floor();
                    let alpha = 32.0 + (random(&mut self.seed) * 33.0).floor();
                    let colour = Vec4::new(grey, grey, 128.0, alpha);
                    self.start_color = colour;
                    self.end_color = colour;
                }
            }
            self.ends();
        } else if spec.fade_time > 0.0 {
            let counter = ticks - self.duration;
            let f = ((1.0 - counter / spec.fade_time) * spec.fade_start_factor).clamp(0.0, 1.0);
            self.start_color = spec.fade.lerp(spec.start, f);
            self.end_color = spec.fade.lerp(spec.end, f);
        }
        self.adjust_box(spec);
        self.counter = (self.counter - ratio).max(0.0);
        if self.counter <= 0.0 {
            self.generate(spec);
            self.counter = spec.count;
        }
        let merge = (spec.merge * ratio).clamp(0.0, 1.0);
        for (point, target) in self.line.iter_mut().zip(&self.meet) {
            *point += (*target - *point) * merge;
        }
    }

    /// Whether it is still drawn.
    pub fn alive(&self) -> bool {
        self.ticks() < self.life
    }

    /// `lightning-draw`: its two strips, the first widened along the
    /// camera's `right`, the second along its `up`.
    pub fn strips(&self, spec: &Spec, textures: &HashMap<u32, String>, right: Vec3, up: Vec3) -> Vec<Strip> {
        let n = self.line.len();
        if !self.alive() || n < 2 {
            return Vec::new();
        }
        let Some(texture) = textures.get(&spec.texture) else {
            return Vec::new();
        };
        let colour = |i: usize| -> Vec4 {
            if spec.flags & LSF0 != 0 {
                self.start_color.lerp(self.end_color, i as f32 / (n - 1) as f32)
            } else if spec.flags & LSF1 != 0 {
                if i + 1 == n { self.end_color } else { self.start_color }
            } else if spec.flags & LSF2 != 0 {
                if i == 0 || i + 1 == n { self.end_color } else { self.start_color }
            } else {
                self.start_color
            }
        };
        [right, up]
            .into_iter()
            .map(|axis| {
                let axis = axis.normalize_or_zero() * spec.radius;
                let mut verts = Vec::with_capacity(2 * n);
                for (i, p) in self.line.iter().enumerate() {
                    let rgba = colour(i).to_array();
                    let t = i as f32 / n as f32;
                    verts.push(StripVertex { pos: *p + axis, rgba, st: [0.0, t] });
                    verts.push(StripVertex { pos: *p - axis, rgba, st: [1.0, t] });
                }
                Strip { texture: texture.clone(), additive: true, verts }
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn spec(flags: u16, fade_time: f32) -> Spec {
        Spec {
            texture: 1,
            flags,
            rand_func: 0,
            adjust_distance: 10.0,
            points: 20,
            size: 8192.0,
            reduction: 0.42,
            merge: 0.6,
            count: 2.0,
            radius: 1638.4,
            duration: 30.0,
            duration_rand: 150.0,
            fade_time,
            fade_start_factor: 0.2,
            start: Vec4::new(255.0, 255.0, 255.0, 128.0),
            end: Vec4::new(255.0, 255.0, 255.0, 128.0),
            fade: Vec4::new(191.0, 0.0, 143.0, 5.0),
        }
    }

    fn request(probe: bool, age: f32) -> LightningRequest {
        LightningRequest { id: 7, spec: "x", from: Vec3::ZERO, to: Vec3::X * 40960.0, age, probe }
    }

    /// Two strips through its points, along the camera's x and y, its
    /// texture once along the bolt; its ends on its ends; its life a whole
    /// number of ticks between its duration and that and its random more.
    #[test]
    fn a_bolt_is_two_camera_strips_through_its_points() {
        let s = spec(LSF0 | LSF4, 0.0);
        let bolt = Bolt::new(request(false, 0.0), &s);
        assert_eq!(bolt.line.len(), 20);
        assert_eq!((bolt.line[0], bolt.line[19]), (Vec3::ZERO, Vec3::X * 40960.0));
        assert!((30.0..=180.0).contains(&bolt.duration) && bolt.duration.fract() == 0.0);
        let textures = HashMap::from([(1, "lightning".to_owned())]);
        let strips = bolt.strips(&s, &textures, Vec3::X, Vec3::Y);
        assert_eq!(strips.len(), 2);
        for (strip, axis) in strips.iter().zip([Vec3::X, Vec3::Y]) {
            assert_eq!(strip.verts.len(), 40);
            let across = strip.verts[1].pos - strip.verts[0].pos;
            assert!((across + axis * 2.0 * 1638.4).length() < 1e-2, "{across:?}");
            assert_eq!((strip.verts[0].st, strip.verts[39].st), ([0.0, 0.0], [1.0, 19.0 / 20.0]));
        }
    }

    /// `lightning-probe-callback`: on its second frame a colour of its own,
    /// grey 64 to 128 on blue 128, alpha 32 to 64.
    #[test]
    fn the_probe_callback_greys_it_on_its_second_frame() {
        let s = spec(LSF0, 0.0);
        let mut bolt = Bolt::new(request(true, 0.0), &s);
        bolt.step(1.0 / 60.0, &s);
        assert_eq!(bolt.start_color, s.start);
        bolt.step(1.0 / 60.0, &s);
        let c = bolt.start_color;
        assert!((64.0..=128.0).contains(&c.x) && c.x == c.y && c.z == 128.0 && (32.0..=64.0).contains(&c.w), "{c:?}");
    }

    /// Fading: from its fade-start share of its colour to the fade colour;
    /// gone after it.
    #[test]
    fn it_fades_to_its_fade_colour_then_goes() {
        let s = spec(LSF0, 30.0);
        let mut bolt = Bolt::new(request(false, 0.0), &s);
        bolt.request.age = bolt.duration / 300.0;
        bolt.step(1.0 / 60.0, &s);
        let expect = s.fade.lerp(s.start, 0.2);
        assert!((bolt.start_color - expect).length() < 1e-3);
        bolt.request.age = (bolt.duration + 30.0) / 300.0;
        assert!(!bolt.alive());
    }
}
