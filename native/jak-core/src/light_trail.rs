//! Light trails (`light-trails.gc`, Jak 3): a ribbon through the places something has been, laid out by a
//! `light-trail-composition`. A tracker adds the thing's position each frame (`add-crumb!`, which refuses a
//! point nearer than 1 cm to the last); `common-trans!` ages the trail's two markers and lets old points go
//! (`expire-old-points!`); `build-prim-strip!` makes the mesh, each point's colour, alpha, width and texture
//! coordinate by the composition's curves.
//!
//! Only what `*water-wake-trail*` (`water.gc`) uses is here: the appearance-1 flat ribbon, tape mode and the
//! scale modes 0 to 3 (age and the three distances). The others (a camera distance and a random one, the
//! two-strip lie mode) are for trails of other things.
use glam::Vec3;

use crate::lightning_bolt::{Strip, StripVertex};

/// One of the composition's two-point curves (`curve2d-piecewise`, clamped as `allocate!` makes them): its points
/// from the first's x to the last's.
type Curve = &'static [(f32, f32)];

/// `*color-curve-tan-brown*` and its likes (`curve-color-piecewise`).
type ColorCurve = &'static [(f32, [f32; 4])];

/// `light-trail-composition`.
#[derive(Clone, Copy, Debug)]
pub struct Composition {
    pub color_mode: u8,
    pub color_curve: ColorCurve,
    pub color_repeat_dist: f32,
    pub alpha_1_mode: u8,
    pub alpha_2_mode: u8,
    pub base_alpha: f32,
    pub alpha_curve_1: Curve,
    pub alpha_curve_2: Curve,
    pub alpha_repeat_dist: f32,
    pub width_mode: u8,
    pub base_width: f32,
    pub width_curve: Curve,
    pub width_repeat_dist: f32,
    pub uv_mode: u8,
    pub uv_repeat_dist: f32,
    /// Ticks (300 a second).
    pub max_age: i64,
    pub texture: &'static str,
    /// `lie-mode appearance1`: flat on the ground, across the way it goes (the only one here).
    pub lie_vector: Vec3,
    pub tape: bool,
    /// `blend-mode`: 0 the usual alpha blend, 1 and 2 others; 0 only here.
    pub additive: bool,
}

/// `*water-wake-trail*` (`water.gc`): 1 m wide at its head and growing to 10 m as it ages, tan fading to brown, in
/// over its first 10 m and out over its three seconds, the `water-wake` texture repeated every 10 m, flat on the
/// water (`lie-vector` up), 0.6 as bright as it can be.
pub const WATER_WAKE: Composition = Composition {
    color_mode: 0,
    color_curve: &[(0.0, [1.0, 1.0, 0.781_25, 1.0]), (1.0, [0.781_25, 0.781_25, 0.625, 1.0])],
    color_repeat_dist: 40960.0,
    alpha_1_mode: 1,
    alpha_2_mode: 0,
    base_alpha: 0.6,
    alpha_curve_1: &[(0.0, 0.0), (1.0, 1.0)],
    alpha_curve_2: &[(0.0, 1.0), (1.0, 0.0)],
    alpha_repeat_dist: 40960.0,
    width_mode: 0,
    base_width: 4096.0,
    width_curve: &[(0.0, 1.0), (1.0, 10.0)],
    width_repeat_dist: 40960.0,
    uv_mode: 3,
    uv_repeat_dist: 40960.0,
    max_age: 900,
    texture: "level-default-water/water-wake",
    lie_vector: Vec3::Y,
    tape: true,
    additive: false,
};

/// `light-trail-decision`: what the last `add-crumb!` came to.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Decision {
    Added,
    NotAdded,
    Reset,
}

/// The nearest a crumb may be to the last (`(< f0-1 40.96)`).
const MIN_STEP: f32 = 40.96;

/// `light-trail`: its crumbs, oldest first (position and `birth-time`), and the markers.
#[derive(Clone, Debug)]
pub struct Trail {
    pub id: u64,
    appearance: Composition,
    crumbs: Vec<(Vec3, i64)>,
    max_crumbs: usize,
    start: i64,
    end: i64,
    decision: Decision,
    total_distance: f32,
    /// The tracker is in `die`: nothing more is added; the trail is gone when a crumb is left.
    pub ending: bool,
}

/// `(piecewise curve) evaluate` at `x` in 0 to 1 (clamped).
fn evaluate(curve: Curve, x: f32) -> f32 {
    let x = x.clamp(0.0, 1.0);
    if x == 0.0 {
        return curve[0].1;
    }
    for i in 0..curve.len() {
        if curve[i].0 >= x {
            let (x0, y0) = curve[i.saturating_sub(1)];
            let (x1, y1) = curve[i];
            let span = x1 - x0;
            let t = if span == 0.0 { 1.0 } else { ((x - x0) / span).clamp(0.0, 1.0) };
            return y0 * (1.0 - t) + y1 * t;
        }
    }
    0.0
}

/// `(curve-color-piecewise) evaluate`.
fn evaluate_color(curve: ColorCurve, x: f32) -> [f32; 4] {
    let x = x.clamp(0.0, 1.0);
    if x == 0.0 {
        return curve[0].1;
    }
    for i in 0..curve.len() {
        if curve[i].0 >= x {
            let (x0, c0) = curve[i.saturating_sub(1)];
            let (x1, c1) = curve[i];
            let span = x1 - x0;
            let t = if span == 0.0 { 1.0 } else { ((x - x0) / span).clamp(0.0, 1.0) };
            return std::array::from_fn(|k| c0[k] * (1.0 - t) + c1[k] * t);
        }
    }
    curve[curve.len() - 1].1
}

/// `compute-trail-scaled-t`: the position along the trail that a curve is read at: the age (0), the distance
/// from the oldest crumb (2), from the newest (1) or travelled by the thing to there (3), in repeats.
fn scaled(mode: u8, age: f32, from_oldest: f32, from_newest: f32, travelled: f32, repeat: f32) -> f32 {
    match mode {
        0 => age,
        2 => from_oldest / repeat,
        1 => from_newest / repeat,
        3 => travelled / repeat,
        _ => 1.0,
    }
}

impl Trail {
    /// `setup!`: a trail for up to `max_crumbs` crumbs, started at tick `now`.
    pub fn new(id: u64, appearance: Composition, max_crumbs: usize, now: i64) -> Self {
        Self {
            id,
            appearance,
            crumbs: Vec::with_capacity(max_crumbs),
            max_crumbs,
            start: now,
            end: now,
            decision: Decision::Reset,
            total_distance: 0.0,
            ending: false,
        }
    }

    pub fn crumbs(&self) -> usize {
        self.crumbs.len()
    }

    /// `add-crumb!`: `elapsed` ticks since the last frame. Refused (returns 1) nearer than 1 cm to the last; 0 when
    /// the oldest was kicked out to make room, else 2.
    pub fn add_crumb(&mut self, pos: Vec3, elapsed: i64) -> i32 {
        if let Some(&(last, _)) = self.crumbs.last() {
            let d = last.distance(pos);
            if d < MIN_STEP {
                self.decision = Decision::NotAdded;
                return 1;
            }
            self.total_distance += d;
        }
        let mut result = 2;
        if self.crumbs.len() == self.max_crumbs {
            self.crumbs.remove(0);
            result = 0;
        }
        let behind = if self.appearance.tape { self.appearance.max_age as f32 - (self.start - self.end) as f32 } else { 0.0 };
        let birth = ((self.start + elapsed) as f32 - behind) as i64;
        self.crumbs.push((pos, birth));
        self.decision = Decision::Added;
        result
    }

    /// `expire-old-points!`: the crumbs born before the end marker go, the last of them left as the point the end
    /// marker is on, between itself and the next.
    fn expire_old_points(&mut self) {
        let mut last = None;
        for (i, &(pos, birth)) in self.crumbs.iter().enumerate() {
            if birth < self.end {
                last = Some((i, pos, birth));
            } else {
                break;
            }
        }
        let Some((i, old_pos, old_birth)) = last else { return };
        if let Some(&(next_pos, next_birth)) = self.crumbs.get(i + 1) {
            let f0 = (self.start - next_birth) as f32;
            let f1 = (self.start - old_birth) as f32;
            let t = (self.appearance.max_age as f32 - f0) / (f1 - f0);
            self.crumbs[i].0 = next_pos.lerp(old_pos, t);
            self.crumbs[i].1 = self.end;
        }
        if i > 0 {
            self.crumbs.drain(0..i);
        }
    }

    /// `common-trans!`: each frame, `frame` ticks long, after the tracker's `add-crumb!` (or without it).
    pub fn common_trans(&mut self, frame: i64) {
        if self.crumbs.is_empty() {
            return;
        }
        match self.decision {
            Decision::Reset => return,
            Decision::Added => {
                self.start += frame;
                let young = self.start - self.appearance.max_age;
                if self.end < young {
                    self.end = young;
                    self.expire_old_points();
                }
            }
            Decision::NotAdded => {
                self.end += frame;
                if self.start < self.end {
                    self.crumbs.clear();
                    self.end = self.start;
                }
                self.end += frame;
                self.start += frame;
            }
        }
        self.decision = Decision::NotAdded;
    }

    /// `build-prim-strip!` (appearance 1, one strip): the ribbon's triangle strip, newest point first.
    pub fn strip(&self) -> Strip {
        let a = &self.appearance;
        let n = self.crumbs.len();
        let mut strip = Strip { texture: a.texture.to_owned(), additive: a.additive, verts: Vec::new() };
        if n < 2 {
            return strip;
        }
        // The length from the oldest crumb to each, over the crumbs that are in their age.
        let mut dist = vec![0.0f32; n];
        let mut total = 0.0f32;
        for i in 1..n {
            let age = self.start - self.crumbs[i].1;
            if age >= 0 && age < a.max_age {
                total += self.crumbs[i - 1].0.distance(self.crumbs[i].0);
                dist[i] = total;
            }
        }
        // (`allocated-num-verts` is two a crumb.)
        let room = self.max_crumbs * 2;
        let mut uv_before = if a.uv_mode == 3 { 100.0f32 } else { -100.0 };
        let (mut pos_before, mut width_before, mut color_before, mut across_before) = (Vec3::ZERO, 0.0f32, [0.0f32; 4], Vec3::ZERO);
        let push_pair = |strip: &mut Strip, pos: Vec3, color: [f32; 4], width: f32, across: Vec3, along: f32| {
            if strip.verts.len() + 2 > room {
                return;
            }
            let half = across.normalize_or_zero() * (width / 2.0);
            // `rgba<-rgbaf`: a colour is 128 for one.
            let rgba = color.map(|c| (c * 128.0).trunc());
            strip.verts.push(StripVertex { pos: pos + half, rgba, st: [along, 0.0] });
            strip.verts.push(StripVertex { pos: pos - half, rgba, st: [along, 1.0] });
        };
        for i in (0..n).rev() {
            let (pos, birth) = self.crumbs[i];
            let age = (self.start - birth) as f32 / a.max_age as f32;
            if !(0.0..=1.0).contains(&age) {
                continue;
            }
            let from_oldest = dist[i];
            let from_newest = total - from_oldest;
            let travelled = self.total_distance - from_newest;
            let alpha_1 = evaluate(a.alpha_curve_1, scaled(a.alpha_1_mode, age, from_oldest, from_newest, travelled, a.alpha_repeat_dist));
            let alpha_2 = evaluate(a.alpha_curve_2, scaled(a.alpha_2_mode, age, from_oldest, from_newest, travelled, a.alpha_repeat_dist));
            let width_t = evaluate(a.width_curve, scaled(a.width_mode, age, from_oldest, from_newest, travelled, a.width_repeat_dist));
            let mut uv = scaled(a.uv_mode, age, from_oldest, from_newest, travelled, a.uv_repeat_dist);
            if !(0.0..=1.0).contains(&uv) {
                uv -= uv.trunc();
            }
            let alpha = alpha_1 * alpha_2 * a.base_alpha;
            let width = width_t * a.base_width;
            let mut color = evaluate_color(a.color_curve, scaled(a.color_mode, age, from_oldest, from_newest, travelled, a.color_repeat_dist));
            color[3] *= alpha;
            // `calc-vertex-pos!` (appearance 1): across the way it goes, flat.
            let along = if i > 0 { pos - self.crumbs[i - 1].0 } else { self.crumbs[1].0 - pos };
            let mut across = along.cross(a.lie_vector);
            across.y = 0.0;
            // The texture's coordinate wraps: where it jumps from the end of one repeat to the start of the next, a
            // pair at each edge of the join (u 0 and u 1) keeps the repeat whole.
            let wrapped = if a.uv_mode == 3 { uv_before < uv } else { uv < uv_before };
            if wrapped {
                let t = uv_before / (uv_before + uv);
                let join_color = std::array::from_fn(|k| color_before[k] * (1.0 - t) + color[k] * t);
                let join_pos = pos_before.lerp(pos, t);
                let join_width = width_before * (1.0 - t) + width * t;
                let join_across = across_before.lerp(across, t);
                push_pair(&mut strip, join_pos, join_color, join_width, join_across, 0.0);
                push_pair(&mut strip, join_pos, join_color, join_width, join_across, 1.0);
            }
            push_pair(&mut strip, pos, color, width, across, uv);
            uv_before = uv;
            pos_before = pos;
            width_before = width;
            color_before = color;
            across_before = across;
        }
        strip
    }

    /// The trail carried into a new frame (see [`crate::reframe`]).
    pub fn reframe(&mut self, frame: &crate::reframe::Frame) {
        for crumb in &mut self.crumbs {
            crumb.0 = frame.point(crumb.0);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A tracker's frames: the thing at `pos`, 5 ticks.
    fn frame(trail: &mut Trail, pos: Vec3) {
        trail.add_crumb(pos, 5);
        trail.common_trans(5);
    }

    #[test]
    fn the_numbers_are_water_dot_gcs() {
        let Some(home) = std::env::var_os("USERPROFILE") else { return };
        let path = std::path::PathBuf::from(home).join(r"Documents\Jak OpenGOAL\active\jak3\data\goal_src\jak3\engine\common-obs\water.gc");
        let Ok(text) = std::fs::read_to_string(path) else { return };
        let number = |field: &str| -> f32 {
            let key = format!("(set! (-> *water-wake-trail* {field}) ");
            let at = text.find(&key).unwrap_or_else(|| panic!("no {field}")) + key.len();
            let rest = &text[at..];
            let end = rest.find(')').unwrap();
            let value = rest[..end].trim().trim_start_matches("(the-as uint ").trim_start_matches("(seconds ").to_owned();
            value.parse().unwrap_or_else(|_| panic!("{field}: {value}"))
        };
        let w = WATER_WAKE;
        assert_eq!(number("color-mode"), f32::from(w.color_mode));
        assert_eq!(number("color-repeat-dist"), w.color_repeat_dist);
        assert_eq!(number("alpha-1-mode"), f32::from(w.alpha_1_mode));
        assert_eq!(number("alpha-2-mode"), f32::from(w.alpha_2_mode));
        assert_eq!(number("base-alpha"), w.base_alpha);
        assert_eq!(number("alpha-repeat-dist"), w.alpha_repeat_dist);
        assert_eq!(number("width-mode"), f32::from(w.width_mode));
        assert_eq!(number("base-width"), w.base_width);
        assert_eq!(number("width-repeat-dist"), w.width_repeat_dist);
        assert_eq!(number("uv-mode"), f32::from(w.uv_mode));
        assert_eq!(number("uv-repeat-dist"), w.uv_repeat_dist);
        // (seconds 3), 300 ticks a second.
        assert_eq!(number("max-age") * 300.0, w.max_age as f32);
        assert_eq!(number("base-alpha"), 0.6);
        // Curves and colours: the points of `*growing-curve*` (1 to 10), the fades and the tan to brown.
        for (needle, value) in [
            ("(set! (-> *growing-curve* pts data 1 second) ", 10.0),
            ("(set! (-> *water-simple-alpha-curve-in* pts data 1 second) ", 1.0),
            ("(set! (-> *water-simple-alpha-curve-fade-out* pts data 0 second) ", 1.0),
            ("(set! (-> *color-curve-tan-brown* pts data 0 second z) ", 0.781_25),
            ("(set! (-> *color-curve-tan-brown* pts data 1 second z) ", 0.625),
        ] {
            let at = text.find(needle).unwrap_or_else(|| panic!("{needle}")) + needle.len();
            let end = text[at..].find(')').unwrap();
            assert_eq!(text[at..at + end].trim().parse::<f32>().unwrap(), value, "{needle}");
        }
        assert_eq!(w.width_curve, &[(0.0, 1.0), (1.0, 10.0)]);
        assert_eq!(w.alpha_curve_1, &[(0.0, 0.0), (1.0, 1.0)]);
        assert_eq!(w.alpha_curve_2, &[(0.0, 1.0), (1.0, 0.0)]);
    }

    /// A crumb closer than a centimetre to the last is refused; the trail is not extended.
    #[test]
    fn crumbs_closer_than_a_centimetre_are_refused() {
        let mut trail = Trail::new(1, WATER_WAKE, 225, 0);
        assert_eq!(trail.add_crumb(Vec3::ZERO, 5), 2);
        assert_eq!(trail.add_crumb(Vec3::new(40.0, 0.0, 0.0), 5), 1);
        assert_eq!(trail.add_crumb(Vec3::new(41.0, 0.0, 0.0), 5), 2);
        assert_eq!(trail.crumbs(), 2);
    }

    /// Tape mode: a new trail begins faint (its crumbs are born nearly at the end of their age) and, as it goes on,
    /// each new one is younger; the ribbon is flat on the ground, symmetrical about its crumbs.
    #[test]
    fn a_wake_fades_in_and_lies_flat() {
        let mut trail = Trail::new(1, WATER_WAKE, 225, 0);
        for i in 0..30 {
            frame(&mut trail, Vec3::new(i as f32 * 400.0, 100.0, 0.0));
        }
        let early = trail.strip();
        assert!(early.verts.len() >= 4);
        let early_alpha = early.verts.iter().map(|v| v.rgba[3]).fold(0.0, f32::max);
        for i in 30..400 {
            frame(&mut trail, Vec3::new(i as f32 * 400.0, 100.0, 0.0));
        }
        let late = trail.strip();
        let late_alpha = late.verts.iter().map(|v| v.rgba[3]).fold(0.0, f32::max);
        assert!(late_alpha > early_alpha * 2.0, "faint at first ({early_alpha}), bright later ({late_alpha})");
        // Flat on the water; each pair across the way it goes (along x here), centred on the crumb.
        for pair in late.verts.chunks(2) {
            assert!((pair[0].pos.y - 100.0).abs() < 1e-3 && (pair[1].pos.y - 100.0).abs() < 1e-3);
            assert!((pair[0].pos.x - pair[1].pos.x).abs() < 1e-2, "across is z");
            assert!(pair[0].pos.z * pair[1].pos.z <= 0.0, "either side of the crumb");
        }
        // The head's width is a metre (4096), growing to ten as it ages: the head of the strip is its first pair.
        let head = (late.verts[0].pos - late.verts[1].pos).length();
        // (The oldest crumb can sit on the next one, a pair of no width, at the end where it is gone to the eye.)
        let tail = late.verts.chunks(2).map(|pair| (pair[0].pos - pair[1].pos).length()).fold(0.0, f32::max);
        assert!((head - 4096.0).abs() < 400.0, "head {head}");
        assert!(tail > 5.0 * head, "tail {tail}");
    }

    /// The oldest crumbs go as the trail ages past its three seconds; with nothing added it wears away from its
    /// tail until none is left, and a crumb is a pair of vertices with the texture coordinate along the ribbon.
    #[test]
    fn an_unfed_trail_wears_away() {
        let mut trail = Trail::new(1, WATER_WAKE, 225, 0);
        for i in 0..500 {
            frame(&mut trail, Vec3::new(i as f32 * 400.0, 0.0, 0.0));
        }
        let before = trail.crumbs();
        assert!(before > 100 && before <= 225, "{before} crumbs");
        for _ in 0..600 {
            trail.common_trans(5);
            if trail.crumbs() <= 1 {
                break;
            }
        }
        assert!(trail.crumbs() <= 1, "{} crumbs left", trail.crumbs());
        let strip = trail.strip();
        assert!(strip.verts.len() % 2 == 0);
    }

    /// The texture coordinate runs along the ribbon, one repeat every ten metres, and wraps with a pair of joins.
    #[test]
    fn the_texture_repeats_every_ten_metres() {
        let mut trail = Trail::new(1, WATER_WAKE, 225, 0);
        for i in 0..200 {
            frame(&mut trail, Vec3::new(i as f32 * 2048.0, 0.0, 0.0));
        }
        let strip = trail.strip();
        let along: Vec<f32> = strip.verts.iter().step_by(2).map(|v| v.st[0]).collect();
        assert!(along.iter().all(|u| (0.0..=1.0).contains(u)));
        // A wrap is a join pair at 0 then 1, and the coordinate drops overall newest to oldest.
        assert!(along.windows(2).any(|w| w[0] == 0.0 && w[1] == 1.0), "no join pair");
    }
}
