//! Jak 3's `lightning-bolt` (`lightning-new.gc`): a bolt along spans
//! between given points, each span's inner points thrown off at random and
//! drawn anew every so often (`regenerate-time`), easing towards their new
//! places (`inner-point-travel-time`), drawn as two crossed strips (one
//! turned to the camera's up, one across it) whose alpha, width and texture
//! run along the bolt's length (`lightning-appearance`). The Arc Wielder's
//! lightning (`gun-blue-2-lightning-tracker`) is six of these.
//!
//! Points are in Jak units; times in the game's ticks (300 a second).
use glam::{Mat3, Vec3, Vec4};

/// A `curve2d-fast` the appearances use.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Curve {
    /// `*curve-linear-down*`: 1 at 0 down to 0 at 1, held past either end.
    LinearDown,
}

impl Curve {
    /// `(evaluate curve x (loop-behavior use-default))`.
    pub fn evaluate(self, x: f32) -> f32 {
        match self {
            Curve::LinearDown => 1.0 - x.clamp(0.0, 1.0),
        }
    }
}

/// `lightning-appearance`.
#[derive(Clone, Debug)]
pub struct Appearance {
    pub base_alpha: f32,
    pub width_range: (f32, f32),
    pub texture: &'static str,
    /// `blend-mode` 1 (`*lightning-alpha-additive*`), else blended.
    pub additive: bool,
    pub fade_time: i64,
    pub regenerate_time: (i64, i64),
    /// `(curve, mode, repeat-dist)` for the first and second alpha and the
    /// width (`alpha-1-*`, `alpha-2-*`, `width-*`).
    pub alpha_1: (Option<Curve>, u8, f32),
    pub alpha_2: (Option<Curve>, u8, f32),
    pub width: (Option<Curve>, u8, f32),
    pub uv_repeat_dist: f32,
    /// `uv-shift?` and its `uv-shift-speed` (ticks for a whole repeat).
    pub uv_shift: Option<i64>,
    pub accurate_interp: bool,
}

/// `lightning-span`.
#[derive(Clone, Copy, Debug, Default)]
pub struct Span {
    pub random_offset_size_start: f32,
    pub inner_random_offset_size: f32,
}

/// `lightning-span-internal`.
#[derive(Clone, Copy, Debug, Default)]
struct SpanInternal {
    index: i16,
    /// 1: drawn last frame; 2: its points drawn anew this frame.
    flags: u8,
    num_inner_points: i8,
}

/// A strip vertex: where, its colour (0 to 255, the PS2's 128 whole), its
/// texture coordinates (`stq` s across, t along).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct StripVertex {
    pub pos: Vec3,
    pub rgba: [f32; 4],
    pub st: [f32; 2],
}

/// A drawn strip: its texture, blend and vertices (a triangle strip).
#[derive(Clone, Debug, Default)]
pub struct Strip {
    pub texture: String,
    pub additive: bool,
    pub verts: Vec<StripVertex>,
}

/// `lightning-bolt`.
#[derive(Clone, Debug)]
pub struct Bolt {
    current: Vec<Vec4>,
    desired: Vec<Vec4>,
    pub span_pts_start: Vec<Vec3>,
    pub spans: Vec<Span>,
    internal: Vec<SpanInternal>,
    capacity: usize,
    pub inner_point_travel_time: i64,
    start_fade_time: i64,
    new_inner_point_generate_time: i64,
    last_generate_time: i64,
    base_width: f32,
    current_uv_shift: f32,
    current_fade_scalar: f32,
    pub fractal_reduction: f32,
    pub appearance: Appearance,
    fade_mode: u8,
    /// 0: each inner point at random; 1: halved and thrown off in turn
    /// (`lightning-bolt-method-21`).
    pub generate_mode: u8,
    pub snap_inner_points: bool,
    pub num_active_spans: usize,
    num_spans: usize,
    seed: u32,
}

impl Bolt {
    /// `init!`: `num_spans` spans of at most `per_span` points each.
    pub fn new(num_spans: usize, per_span: usize, appearance: Appearance, now: i64, seed: u32) -> Self {
        let points = per_span * (num_spans - 1) + 1;
        let mut bolt = Self {
            current: vec![Vec4::ZERO; points],
            desired: vec![Vec4::ZERO; points],
            span_pts_start: vec![Vec3::ZERO; num_spans],
            spans: vec![Span::default(); num_spans],
            internal: (0..num_spans)
                .map(|i| SpanInternal { index: i as i16, flags: 0, num_inner_points: 0 })
                .collect(),
            capacity: 2 * per_span * num_spans,
            inner_point_travel_time: 0,
            start_fade_time: now,
            new_inner_point_generate_time: 0,
            last_generate_time: 0,
            base_width: 0.0,
            current_uv_shift: 0.0,
            current_fade_scalar: 1.0,
            fractal_reduction: 0.0,
            appearance,
            fade_mode: 0,
            generate_mode: 0,
            snap_inner_points: false,
            num_active_spans: 0,
            num_spans,
            seed: seed | 1,
        };
        let (a, b) = bolt.appearance.regenerate_time;
        bolt.new_inner_point_generate_time = bolt.int_range(a, b);
        let (w0, w1) = bolt.appearance.width_range;
        bolt.base_width = bolt.float_range(w0, w1);
        if bolt.appearance.uv_shift.is_some() {
            bolt.current_uv_shift = bolt.unit();
        }
        bolt
    }

    fn next(&mut self) -> u32 {
        self.seed ^= self.seed << 13;
        self.seed ^= self.seed >> 17;
        self.seed ^= self.seed << 5;
        self.seed >> 1
    }

    /// `(+ -1.0 (logior #x3f800000 (/ (rand-uint31-gen) 256)))`: 0 to 1.
    fn unit(&mut self) -> f32 {
        f32::from_bits(0x3f80_0000 | (self.next() >> 8)) - 1.0
    }

    fn float_range(&mut self, a: f32, b: f32) -> f32 {
        a + (b - a) * self.unit()
    }

    fn int_range(&mut self, a: i64, b: i64) -> i64 {
        a + ((b - a + 1) as f32 * self.unit()) as i64
    }

    /// `lightning-bolt-method-14`: 0 lit, 1 fading in, 2 fading out, 3 out.
    pub fn fade_mode(&self) -> u8 {
        self.fade_mode
    }

    /// `lightning-bolt-method-13`.
    pub fn set_fade(&mut self, mode: u8, now: i64) {
        if self.fade_mode == mode {
            return;
        }
        match mode {
            0 => {
                self.current_fade_scalar = 1.0;
                self.start_fade_time = now;
            }
            2 => {
                self.start_fade_time = now;
                self.current_fade_scalar = 1.0;
            }
            1 => {
                self.current_fade_scalar = 0.0;
                self.start_fade_time = now;
            }
            3 => {
                for s in &mut self.internal {
                    s.flags &= !1;
                }
            }
            _ => {}
        }
        self.fade_mode = mode;
    }

    /// `lightning-bolt-method-20`: span `span`'s start point thrown off anew.
    fn new_start(&mut self, at: usize, span: usize) {
        let index = f32::from(self.internal[span].index);
        let (x, y) = (-0.5 + self.unit(), -0.5 + self.unit());
        self.desired[at] = Vec4::new(x, y, index, 1.0);
        self.current[at].z = index;
    }

    /// `lightning-bolt-method-21`: the points between `a` and `b` halved and
    /// thrown off by `size`, less each time (`fractal-reduction`).
    fn halve(&mut self, a: usize, b: usize, size: f32) {
        if b <= a + 1 {
            return;
        }
        let mid = (a + b) / 2;
        let (pa, pb) = (self.desired[a], self.desired[b]);
        let x = 0.5 * (pa.x + pb.x) + size * (-0.5 + self.unit());
        let y = 0.5 * (pa.y + pb.y) + size * (-0.5 + self.unit());
        self.desired[mid].x = x;
        self.desired[mid].y = y;
        self.desired[mid].z = pa.z + (pb.z - pa.z) * 0.5;
        let less = size * self.fractal_reduction;
        self.halve(a, mid, less);
        self.halve(mid, b, less);
    }

    /// `lightning-bolt-method-15`: a span's inner points drawn anew.
    fn generate(&mut self, at: usize, span: usize) {
        let inner = self.internal[span].num_inner_points.max(0) as usize;
        match self.generate_mode {
            0 => {
                let index = f32::from(self.internal[span].index);
                let step = 1.0 / (inner + 1) as f32;
                let mut t = step;
                for k in 0..inner {
                    let z = index + t;
                    let (x, y) = (-0.5 + self.unit(), -0.5 + self.unit());
                    self.desired[at + 1 + k] = Vec4::new(x, y, z, 1.0);
                    self.current[at + 1 + k].z = z;
                    t += step;
                }
            }
            1 => self.halve(at, at + inner + 1, 1.0),
            _ => {}
        }
    }

    /// `lightning-bolt-method-11`, a frame from `prev` to `now`: its fade,
    /// its points drawn anew when due and eased towards where they go.
    pub fn update(&mut self, now: i64, prev: i64) {
        match self.fade_mode {
            2 => {
                let t = ((now - self.start_fade_time) as f32 / self.appearance.fade_time.max(1) as f32).clamp(0.0, 1.0);
                self.current_fade_scalar = 1.0 - t;
                if self.current_fade_scalar <= 0.0 {
                    self.set_fade(3, now);
                }
            }
            1 => {
                let t = ((now - self.start_fade_time) as f32 / self.appearance.fade_time.max(1) as f32).clamp(0.0, 1.0);
                self.current_fade_scalar = t;
                if self.current_fade_scalar >= 1.0 {
                    self.set_fade(0, now);
                }
            }
            _ => {}
        }
        if self.fade_mode == 3 {
            return;
        }
        if self.fade_mode == 0 {
            self.current_fade_scalar = 1.0;
        }
        self.num_active_spans = self.num_active_spans.min(self.num_spans - 1);
        for s in &mut self.internal[self.num_active_spans..] {
            s.flags &= !1;
        }
        let due = now - self.last_generate_time >= self.new_inner_point_generate_time;
        for s in &mut self.internal[..self.num_active_spans] {
            if due || s.flags & 1 == 0 {
                s.flags |= 2;
            } else {
                s.flags &= !2;
            }
        }
        let mut at = 0;
        for span in 0..self.num_active_spans {
            let s = self.internal[span];
            let count = s.num_inner_points.max(0) as usize + 1;
            if s.flags & 2 != 0 {
                if self.snap_inner_points || s.flags & 1 == 0 {
                    for v in 0..count {
                        self.current[at + v] = self.desired[at + v];
                    }
                }
                self.new_start(at, span);
            }
            at += count;
        }
        if due {
            self.new_start(at, self.num_active_spans);
            if self.snap_inner_points {
                self.current[at] = self.desired[at];
            }
            self.last_generate_time = now;
            let (a, b) = self.appearance.regenerate_time;
            self.new_inner_point_generate_time = self.int_range(a, b);
        }
        let mut at = 0;
        for span in 0..self.num_active_spans {
            let s = self.internal[span];
            if s.flags & 2 != 0 {
                self.generate(at, span);
            }
            at += s.num_inner_points.max(0) as usize + 1;
        }
        let travel = self.inner_point_travel_time.max(1) as f32;
        let mut at = 0;
        for span in 0..=self.num_active_spans {
            let s = self.internal[span];
            let count = if span == self.num_active_spans { 1 } else { s.num_inner_points.max(0) as usize + 1 };
            if !self.appearance.accurate_interp {
                let elapsed = if s.flags & 2 != 0 { 0 } else { now - prev };
                let t = (elapsed as f32 / travel).clamp(0.0, 1.0);
                for v in 0..count {
                    let (c, d) = (self.current[at + v], self.desired[at + v]);
                    self.current[at + v] = c.lerp(d, t);
                }
            } else {
                let from = ((prev - self.last_generate_time) as f32 / travel).clamp(0.0, 1.0);
                let to = ((now - self.last_generate_time) as f32 / travel).clamp(0.0, 1.0);
                let left = 1.0 - from;
                for v in 0..count {
                    let (c, d) = (self.current[at + v], self.desired[at + v]);
                    let delta = glam::Vec2::new(d.x - c.x, d.y - c.y);
                    let len = delta.length();
                    if left > 0.0 {
                        let step = delta.normalize_or_zero() * (len / left * (to - from));
                        self.current[at + v].x += step.x;
                        self.current[at + v].y += step.y;
                        self.current[at + v].z = d.z;
                    }
                }
            }
            at += count;
        }
        for s in &mut self.internal[..self.num_active_spans] {
            s.flags |= 1;
        }
    }

    /// Span `span`'s inner point count (`num-inner-points`).
    pub fn set_inner_points(&mut self, span: usize, n: i8) {
        if let Some(s) = self.internal.get_mut(span) {
            s.num_inner_points = n;
        }
    }

    /// `lightning-bolt-method-17`: an alpha or width factor.
    fn factor(&mut self, (curve, mode, repeat): (Option<Curve>, u8, f32), travel: f32, along: f32) -> f32 {
        let Some(curve) = curve else { return 1.0 };
        match mode {
            0 => curve.evaluate(along / repeat),
            1 => curve.evaluate(travel),
            2 => self.unit(),
            _ => 1.0,
        }
    }

    /// `lightning-bolt-method-12` (with `-22`'s texture shift), a frame
    /// from `prev` to `now`: its two strips, the first turned to the
    /// camera's `up` (Jak space).
    pub fn draw(&mut self, now: i64, prev: i64, up: Vec3) -> [Strip; 2] {
        let mut strips = [
            Strip { texture: self.appearance.texture.to_owned(), additive: self.appearance.additive, verts: Vec::new() },
            Strip { texture: self.appearance.texture.to_owned(), additive: self.appearance.additive, verts: Vec::new() },
        ];
        if let Some(speed) = self.appearance.uv_shift {
            self.current_uv_shift += (now - prev) as f32 / speed as f32;
            self.current_uv_shift -= self.current_uv_shift.trunc();
            if self.current_uv_shift < 1.0 {
                self.current_uv_shift += 1.0;
            }
        }
        if self.fade_mode == 3 || self.num_active_spans == 0 {
            return strips;
        }
        let mut point = 0;
        let mut along = 0.0;
        // (last texture t, last distance along)
        let mut last = (0.0f32, 0.0f32);
        let mut dir = Vec3::ZERO;
        for span in 0..self.num_active_spans {
            let (s, si) = (self.spans[span], self.internal[span]);
            let (a, b) = (self.span_pts_start[span], self.span_pts_start[span + 1]);
            let ahead = (b - a).normalize_or(Vec3::Z);
            let index = f32::from(si.index);
            let length = a.distance(b);
            let frame = yz2(ahead, nice_perp(ahead));
            let size = s.random_offset_size_start;
            dir = self.offset_dir(point, point + 1, frame, size, length);
            let mut local = self.current[point].truncate();
            local.x *= size;
            local.y *= size;
            local.z = (local.z - index) * length;
            self.vertex(&mut strips, frame * local + a, local.length(), along, &mut last, dir, now, up);
            let inner = s.inner_random_offset_size;
            for k in 0..si.num_inner_points.max(0) as usize {
                let p = point + 1 + k;
                dir = self.offset_dir(p - 1, p, frame, inner, length);
                let mut local = self.current[p].truncate();
                local.x *= inner;
                local.y *= inner;
                local.z = (local.z - index) * length;
                self.vertex(&mut strips, frame * local + a, local.length(), along, &mut last, dir, now, up);
            }
            along += length;
            point += si.num_inner_points.max(0) as usize + 1;
        }
        let end = self.num_active_spans;
        let ahead = (self.span_pts_start[end] - self.span_pts_start[end - 1]).normalize_or(Vec3::Z);
        let frame = yz2(ahead, nice_perp(ahead));
        let size = self.spans[end].random_offset_size_start;
        let mut local = self.current[point].truncate();
        local.x *= size;
        local.y *= size;
        local.z -= f32::from(self.internal[end].index);
        let at = frame * local + self.span_pts_start[end];
        self.vertex(&mut strips, at, 0.0, along, &mut last, dir, now, up);
        strips
    }

    /// `lightning-bolt-method-19`: from point `a` to `b`, in the world.
    fn offset_dir(&self, a: usize, b: usize, frame: Mat3, size: f32, length: f32) -> Vec3 {
        let d = (self.current[b] - self.current[a]).truncate();
        frame * Vec3::new(d.x * size, d.y * size, d.z * length)
    }

    /// `lightning-bolt-method-16`: a point's pair of vertices on each
    /// strip, with its alpha, width and texture coordinate along.
    #[allow(clippy::too_many_arguments)]
    fn vertex(&mut self, strips: &mut [Strip; 2], at: Vec3, offset: f32, along: f32, last: &mut (f32, f32), dir: Vec3, now: i64, up: Vec3) {
        let travel = (now - self.last_generate_time) as f32 / self.inner_point_travel_time.max(1) as f32;
        let distance = along + offset;
        let appearance = self.appearance.clone();
        let alpha = self.factor(appearance.alpha_1, travel, distance)
            * self.factor(appearance.alpha_2, travel, distance)
            * appearance.base_alpha
            * self.current_fade_scalar;
        let width = self.factor(appearance.width, travel, distance) * self.base_width;
        let u = distance / appearance.uv_repeat_dist;
        let t = (u - (u / 6.0).trunc() * 6.0) + self.current_uv_shift;
        let forward = dir.normalize_or(Vec3::Z);
        // `matrix-f-u-compose` with the camera's up.
        let right = up.cross(forward).normalize_or(Vec3::X);
        let upward = forward.cross(right);
        let half = width / 2.0;
        let (r0, r1) = (at + right * half, at - right * half);
        let (u0, u1) = (at + upward * half, at - upward * half);
        let colour = [128.0, 128.0, 128.0, (128.0 * alpha).trunc()];
        // The texture's t wrapped round (past its sixth repeat): the strip
        // carried on to t 6 and started again from 0 where it was.
        if t < last.0 && strips[0].verts.len() >= 2 && strips[1].verts.len() >= 2 {
            let k = (6.0 + t - last.0) / 6.0;
            let n0 = strips[0].verts.len();
            let (p0, p1) = (strips[0].verts[n0 - 2].pos.lerp(r0, k), strips[0].verts[n0 - 1].pos.lerp(r1, k));
            self.push(&mut strips[0], p0, colour, 0.0, 6.0);
            self.push(&mut strips[0], p1, colour, 1.0, 6.0);
            self.push(&mut strips[0], p0, colour, 0.0, 0.0);
            self.push(&mut strips[0], p1, colour, 1.0, 0.0);
            let n1 = strips[1].verts.len();
            let (q0, q1) = (strips[1].verts[n1 - 2].pos.lerp(u0, k), strips[1].verts[n1 - 1].pos.lerp(u1, k));
            self.push(&mut strips[1], q0, colour, 1.0, 6.0);
            self.push(&mut strips[1], q1, colour, 0.0, 6.0);
            self.push(&mut strips[1], q0, colour, 1.0, 0.0);
            self.push(&mut strips[1], q1, colour, 0.0, 0.0);
        }
        self.push(&mut strips[0], r0, colour, 0.0, t);
        self.push(&mut strips[0], r1, colour, 1.0, t);
        self.push(&mut strips[1], u0, colour, 1.0, t);
        self.push(&mut strips[1], u1, colour, 0.0, t);
        *last = (t, distance);
    }

    /// `lightning-bolt-method-18`.
    fn push(&self, strip: &mut Strip, pos: Vec3, rgba: [f32; 4], s: f32, t: f32) {
        if strip.verts.len() < self.capacity {
            strip.verts.push(StripVertex { pos, rgba, st: [s, t] });
        }
    }
}

/// `choose-nice-perp`.
fn nice_perp(v: Vec3) -> Vec3 {
    if v.x * v.x + v.z * v.z < 0.01 { Vec3::X } else { Vec3::Y }
}

/// `matrix<-vector-yz2!`: z along `ahead`, x across it and `perp`, y up
/// from them; as columns (a local point's x, y, z to the world).
fn yz2(ahead: Vec3, perp: Vec3) -> Mat3 {
    let right = perp.cross(ahead).normalize_or(Vec3::X);
    let up = ahead.cross(right).normalize_or(Vec3::Y);
    Mat3::from_cols(right, up, ahead)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn appearance() -> Appearance {
        Appearance {
            base_alpha: 1.0,
            width_range: (1228.8, 2048.0),
            texture: "lightning-tile",
            additive: true,
            fade_time: 90,
            regenerate_time: (50, 75),
            alpha_1: (Some(Curve::LinearDown), 0, 262144.0),
            alpha_2: (None, 3, 4096.0),
            width: (Some(Curve::LinearDown), 3, 4096.0),
            uv_repeat_dist: 28672.0,
            uv_shift: Some(-150),
            accurate_interp: true,
        }
    }

    /// Along a straight rope of three spans: two strips of as many pairs
    /// as points, near the rope, fading along it, never past its end.
    #[test]
    fn a_bolt_follows_its_spans_and_fades_along_them() {
        let mut bolt = Bolt::new(12, 6, appearance(), 0, 7);
        bolt.generate_mode = 1;
        bolt.snap_inner_points = true;
        bolt.fractal_reduction = 0.4 * 9f32.ln() / 12f32.ln();
        bolt.inner_point_travel_time = 150;
        bolt.num_active_spans = 3;
        for i in 0..=3 {
            bolt.span_pts_start[i] = Vec3::new(0.0, 0.0, 24576.0 * i as f32);
            bolt.spans[i].random_offset_size_start = if i > 0 { 8192.0 } else { 0.0 };
            bolt.spans[i].inner_random_offset_size = 8192.0;
            bolt.set_inner_points(i, 4);
        }
        let mut prev = 0;
        for frame in 1..30 {
            let now = frame * 5;
            bolt.update(now, prev);
            let [a, b] = bolt.draw(now, prev, Vec3::Y);
            prev = now;
            assert_eq!(a.verts.len(), b.verts.len());
            // 3 spans of 5 points and the end, two vertices each (and a
            // texture wrap's four now and then).
            assert!(a.verts.len() >= 32, "{}", a.verts.len());
            for v in &a.verts {
                assert!(v.pos.x.abs() < 3.0 * 4096.0 && v.pos.z > -8192.0 && v.pos.z < 4.0 * 24576.0, "{:?}", v.pos);
            }
            let first = a.verts.first().unwrap().rgba[3];
            let end = a.verts.last().unwrap().rgba[3];
            assert!(first > end, "fades along: {first} to {end}");
        }
        bolt.set_fade(2, 200);
        bolt.update(400, 395);
        assert_eq!(bolt.fade_mode(), 3);
        assert!(bolt.draw(400, 395, Vec3::Y)[0].verts.is_empty());
    }
}
