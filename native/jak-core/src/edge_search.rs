//! Finding a ledge to hang from, as the original does (`collide-edge-grab.gc`: Jak 1's `find-edge-grabs!`, Jak 3's
//! `target-method-28`; their tables for Jak are the same). The triangles round him are narrowed to the ones that
//! face up and reach his hands (`find-grabbable-tris`), their sides made into edges and the edges two of them
//! share dropped (`find-grabbable-edges`), the edges he is outside of and can reach are rated by how near they
//! are to his hands (`search-for-edges`, `should-add-to-list?`), and the best is held only if his hands have edge
//! to rest on at both sides and the room for his body, hanging and climbing, is clear of everything
//! (`find-best-grab!`, `check-grab-for-collisions`: twelve spheres).
//!
//! The host gives the triangles ([`crate::collide::CollideWorld`]); an edge exists where an up-facing triangle's
//! side is not another's. A block with a block on it has no top to hold, a wall has no edge but its top, and what
//! stands over a top (a block, a slab, a ceiling) is in the way of the spheres. All distances are the original's,
//! in his drawn size `scale` (see [`crate::Jak::body_size`]).
use glam::Vec3;

use crate::collide::{CollideCache, PatMode, Tri};
use crate::math::METER;

/// `local-cache-fill-box`, from where his feet are.
const FILL_MIN: Vec3 = Vec3::new(-8192.0, -11059.2, -8192.0);
const FILL_MAX: Vec3 = Vec3::new(8192.0, 24576.0, 8192.0);
/// `local-within-reach-box`: where the center of a hold may be (1.3 to 2.7 m over his feet).
const REACH_MIN: Vec3 = Vec3::new(-6144.0, 5324.8, -6144.0);
const REACH_MAX: Vec3 = Vec3::new(6144.0, 11059.2, 6144.0);
/// `outward-offset` x: the point off the edge he is measured from, 0.173 m out.
const OUTWARD_OFFSET: f32 = 708.608;
/// `max-dist-sqrd-to-outward-pt`: 1.5 m, squared.
const MAX_DIST_SQRD: f32 = 37_748_736.0;
/// `max-dir-cosa-delta`: the edge within about 53 degrees of the way he heads or pushes.
const MAX_DIR_COSA: f32 = 0.6;
/// `split-dists`: when a hold fails, those 0.25 m and then 0.35 m along the edge to each side are tried.
const SPLIT: [f32; 2] = [1024.0, 1433.6];
/// How far along the edge from the hold each hand rests (0.27 m), and how near an edge it must be (0.12 m).
const HAND_OFFSET: f32 = 1105.92;
const HAND_TOLERANCE: f32 = 491.52;
/// Corners closer than this are one (0.01 m, squared).
const MERGE_SQRD: f32 = 1677.7216;
/// The least upward a triangle's normal may point to be something to hang from (about 45 degrees).
const MIN_NORMAL_Y: f32 = 0.707;
const MAX_TRIS: usize = 48;
const MAX_VERTS: usize = 64;
const MAX_EDGES: usize = 96;
const MAX_ITEMS: usize = 32;
/// The most holds tried for one grab.
const MAX_ATTEMPTS: usize = 16;

/// `local-player-spheres`: out from the edge, up from it and the radius. The first six hang under the edge (the
/// body), the other six are the room over it he climbs into (`world-player-leap-up-spheres`).
pub(crate) const SPHERES: [[f32; 3]; 12] = [
    [1720.32, -819.2, 1433.6],
    [2293.76, -3276.8, 1884.16],
    [1966.08, -6144.0, 1556.48],
    [1966.08, -8601.6, 1556.48],
    [1761.28, -11059.2, 1351.68],
    [1679.36, -13312.0, 1269.76],
    [-737.28, 4096.0, 3072.0],
    [-737.28, 6553.6, 3072.0],
    [-737.28, 9420.8, 3072.0],
    [1720.32, 3686.4, 2949.12],
    [1720.32, 5734.4, 2949.12],
    [1720.32, 8601.6, 2949.12],
];

/// Where the six spheres hanging (`leap` false) or climbing (true) lie for a hold at `center` with `outward` (flat,
/// unit) pointing away from the wall: `transform-vectors!` by the hanging matrix (outward, up, along the edge).
/// The climb's reach no higher over the edge than `room` (see [`Query::room`]).
pub(crate) fn spheres(center: Vec3, outward: Vec3, scale: f32, leap: bool, room: f32) -> [(Vec3, f32); 6] {
    let from = if leap { 6 } else { 0 };
    std::array::from_fn(|i| {
        let [out, up, radius] = SPHERES[from + i];
        let up = if leap { (up * scale).min(room - radius * scale) } else { up * scale };
        (center + outward * (out * scale) + Vec3::Y * up, radius * scale)
    })
}

/// A hold found: where his hands go and which way the wall's outside is.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Grab {
    pub center: Vec3,
    pub outward: Vec3,
}

/// What the search measures from.
pub(crate) struct Query<'a> {
    /// The triangles round him.
    pub tris: &'a [Tri],
    /// The same, to try spheres against.
    pub cache: &'a CollideCache,
    /// Where his feet are (`control trans`).
    pub trans: Vec3,
    /// Between his hands (`unknown-vector90`).
    pub hands: Vec3,
    /// His drawn size over the original's.
    pub scale: f32,
    /// How high a ledge must be over the ground below for the host (two blocks of wall are to be hung from).
    pub min_height: f32,
    /// stand-in: how much room over the edge the climb needs at most (his body's height in the host; infinite, the
    /// original's spheres, without one).
    pub room: f32,
}

/// Floor this far under a hold is the ground the wall stands on, which his legs' spheres reach for a ledge only
/// as high as the host allows: it is not counted against them (stand-in: the original's wall is always higher).
pub(crate) fn floor_limit(top: f32, min_height: f32) -> f32 {
    top - min_height + 0.1 * METER
}

/// The box the triangles are asked of (`local-cache-fill-box` from his feet).
pub(crate) fn fill_box(trans: Vec3, scale: f32) -> (Vec3, Vec3) {
    (trans + FILL_MIN * scale, trans + FILL_MAX * scale)
}

struct Edge {
    /// Shared by two triangles, or he is not outside it.
    ignore: bool,
    /// Its two corners.
    v: [usize; 2],
    /// Flat, away from the triangle.
    outward: Vec3,
    /// Along it, unit.
    dir: Vec3,
}

#[derive(Clone, Copy)]
struct Item {
    rating: f32,
    split: i8,
    edge: usize,
    center: Vec3,
}

/// The triangles and edges of one search, as `*collide-edge-work*` holds them.
pub(crate) struct Edges<'a> {
    q: &'a Query<'a>,
    verts: Vec<Vec3>,
    edges: Vec<Edge>,
    /// How many triangles were grabbable, for the host's diagnostics.
    pub tris: usize,
}

impl<'a> Edges<'a> {
    /// `find-grabbable-tris` and `find-grabbable-edges`.
    pub(crate) fn build(q: &'a Query<'a>) -> Self {
        let s = q.scale;
        let reach_min = q.trans + REACH_MIN * s;
        let reach_max = q.trans + REACH_MAX * s;
        // The tops: not walls, facing up, in reach.
        let mut tops: Vec<[Vec3; 3]> = Vec::new();
        for tri in q.tris {
            if !matches!(tri.pat.mode, PatMode::Ground | PatMode::Obstacle) {
                continue;
            }
            let [a, b, c] = tri.v;
            if a.min(b).min(c).cmpgt(reach_max).any() || reach_min.cmpgt(a.max(b).max(c)).any() {
                continue;
            }
            let normal = (b - a).cross(c - a).normalize_or_zero();
            if normal.y < MIN_NORMAL_Y {
                continue;
            }
            if tops.len() == MAX_TRIS {
                break;
            }
            tops.push(tri.v);
        }
        let mut work = Edges { q, verts: Vec::new(), edges: Vec::new(), tris: tops.len() };
        // `vf1`: where he stands, at the reach box's bottom, for which side of an edge he is on.
        let stand = Vec3::new(q.trans.x, reach_min.y, q.trans.z);
        'tris: for top in &tops {
            let mut corner = [0usize; 3];
            for (i, v) in top.iter().enumerate() {
                match work.vertex(*v) {
                    Some(index) => corner[i] = index,
                    None => break 'tris,
                }
            }
            for (a, b) in [(0, 1), (1, 2), (2, 0)] {
                if !work.edge(corner[a], corner[b], stand) {
                    break 'tris;
                }
            }
        }
        work
    }

    /// The corner `v` is, merged with one within a centimeter; none when there is no room for another.
    fn vertex(&mut self, v: Vec3) -> Option<usize> {
        if let Some(found) = self.verts.iter().position(|old| old.distance_squared(v) <= MERGE_SQRD) {
            return Some(found);
        }
        if self.verts.len() == MAX_VERTS {
            return None;
        }
        self.verts.push(v);
        Some(self.verts.len() - 1)
    }

    /// One side of a triangle: the same edge the other way round is the other triangle's (both go); else a new
    /// edge, kept if he is on its outside. False when there is no room.
    fn edge(&mut self, a: usize, b: usize, stand: Vec3) -> bool {
        if let Some(shared) = self.edges.iter_mut().find(|e| e.v == [b, a]) {
            shared.ignore = true;
            return true;
        }
        if self.edges.len() == MAX_EDGES {
            return false;
        }
        let (v0, v1) = (self.verts[a], self.verts[b]);
        let along = v1 - v0;
        // Across it, flat: its cross with up, the way the triangle does not lie.
        let across = Vec3::new(-along.z, 0.0, along.x);
        let outside = across.x * (stand.x - v0.x) + across.z * (stand.z - v0.z);
        let usable = across.length_squared() > 1e-6 && outside >= 0.0;
        self.edges.push(Edge {
            ignore: !usable,
            v: [a, b],
            outward: if usable { across.normalize() } else { Vec3::ZERO },
            dir: if usable { along.normalize() } else { Vec3::ZERO },
        });
        true
    }

    /// The point of an edge nearest his hands, measured along it flat (`compute-center-point!`).
    fn center_point(&self, edge: &Edge) -> Vec3 {
        let (v0, v1) = (self.verts[edge.v[0]], self.verts[edge.v[1]]);
        let along = v1 - v0;
        let length = (along.x * along.x + along.z * along.z).sqrt();
        let to_hands = self.q.hands - v0;
        let reach = ((along.x * to_hands.x + along.z * to_hands.z) / length).clamp(0.0, length);
        v0 + along * (reach / length)
    }

    /// `should-add-to-list?`: the hold at `center` on `edge` where he is, going `dir` (flat, unit).
    fn rate(&self, center: Vec3, edge: &Edge, dir: Vec3) -> Option<f32> {
        let q = self.q;
        let s = q.scale;
        let lo = q.trans + REACH_MIN * s;
        let hi = q.trans + REACH_MAX * s;
        if center.cmplt(lo).any() || center.cmpgt(hi).any() {
            return None;
        }
        let out = center + edge.outward * (OUTWARD_OFFSET * s);
        let (dx, dz) = (q.trans.x - out.x, q.trans.z - out.z);
        if dx * dx + dz * dz > MAX_DIST_SQRD * s * s {
            return None;
        }
        let (tx, tz) = (center.x - q.trans.x, center.z - q.trans.z);
        let length = (tx * tx + tz * tz).sqrt();
        let along = if length > 1e-3 { (tx * dir.x + tz * dir.z) / length } else { 0.0 };
        if along < MAX_DIR_COSA {
            return None;
        }
        let (rx, rz) = (q.hands.x - center.x, q.hands.z - center.z);
        Some(rx * rx + rz * rz)
    }

    /// `search-for-edges`: every edge worth a hold, best (nearest his hands) first.
    fn search(&self, dir: Vec3) -> (Vec<Item>, Vec<usize>) {
        let mut items: Vec<Item> = Vec::new();
        let mut list: Vec<usize> = Vec::new();
        for (index, edge) in self.edges.iter().enumerate() {
            if edge.ignore {
                continue;
            }
            let center = self.center_point(edge);
            let Some(rating) = self.rate(center, edge, dir) else { continue };
            items.push(Item { rating, split: 0, edge: index, center });
            insert(&items, &mut list, items.len() - 1);
            if items.len() == MAX_ITEMS {
                break;
            }
        }
        (items, list)
    }

    /// How near `p` is to any edge that counts (`get-best-hand-point`).
    fn hand_distance(&self, p: Vec3) -> f32 {
        let mut best = f32::INFINITY;
        for edge in self.edges.iter().filter(|e| !e.ignore) {
            let (v0, v1) = (self.verts[edge.v[0]], self.verts[edge.v[1]]);
            let along = v1 - v0;
            let t = if along.length_squared() > 0.0 { ((p - v0).dot(along) / along.length_squared()).clamp(0.0, 1.0) } else { 0.0 };
            best = best.min((v0 + along * t).distance(p));
        }
        best
    }

    /// `check-grab-for-collisions`: his hands have edge both sides of the hold, and his body and the room over
    /// it are clear.
    fn check(&self, item: &Item) -> Option<Grab> {
        let s = self.q.scale;
        let edge = &self.edges[item.edge];
        for sign in [1.0, -1.0] {
            let hand = item.center + edge.dir * (sign * HAND_OFFSET * s);
            if self.hand_distance(hand) > HAND_TOLERANCE * s {
                return None;
            }
        }
        let limit = floor_limit(item.center.y, self.q.min_height);
        if !body_clear(self.q.cache, item.center, edge.outward, s, true, true, limit, self.q.room) {
            return None;
        }
        Some(Grab { center: item.center, outward: edge.outward })
    }

    /// `find-best-grab!`: the best hold, and when one fails the places 0.25 and 0.35 m along its edge each side
    /// of it, up to sixteen tries.
    pub(crate) fn best(&self, dir: Vec3) -> Option<Grab> {
        let s = self.q.scale;
        let (mut items, mut list) = self.search(dir);
        for _ in 0..MAX_ATTEMPTS {
            if list.is_empty() {
                return None;
            }
            let head = list.remove(0);
            let item = items[head];
            if let Some(grab) = self.check(&item) {
                return Some(grab);
            }
            let edge = &self.edges[item.edge];
            let (v0, v1) = (self.verts[edge.v[0]], self.verts[edge.v[1]]);
            let split = item.split;
            if split == 0 {
                let step = edge.dir * (SPLIT[0] * s);
                // Along it first: not past its end.
                let mut next = head;
                let ahead = item.center + step;
                if (v1 - ahead).dot(step) >= 0.0 {
                    if let Some(rating) = self.rate(ahead, edge, dir) {
                        items[head] = Item { rating, split: 1, edge: item.edge, center: ahead };
                        insert(&items, &mut list, head);
                        if items.len() == MAX_ITEMS {
                            continue;
                        }
                        items.push(Item { rating: 0.0, split: 0, edge: item.edge, center: item.center });
                        next = items.len() - 1;
                    }
                }
                // Then back.
                let behind = item.center - step;
                if (behind - v0).dot(step) >= 0.0 {
                    if let Some(rating) = self.rate(behind, edge, dir) {
                        items[next] = Item { rating, split: -1, edge: item.edge, center: behind };
                        insert(&items, &mut list, next);
                    }
                }
                if next != head && !list.contains(&next) {
                    items.pop();
                }
            } else {
                let more = split.unsigned_abs() as usize;
                if more >= 2 {
                    continue;
                }
                let step = edge.dir * (SPLIT[more] * s);
                let (to, end) = if split > 0 { (item.center + step, v1) } else { (item.center - step, v0) };
                let inside = if split > 0 { (end - to).dot(step) >= 0.0 } else { (to - end).dot(step) >= 0.0 };
                if !inside {
                    continue;
                }
                if let Some(rating) = self.rate(to, edge, dir) {
                    let next = if split > 0 { split + 1 } else { split - 1 };
                    items[head] = Item { rating, split: next, edge: item.edge, center: to };
                    insert(&items, &mut list, head);
                }
            }
        }
        None
    }
}

/// `add-to-list!`: into the list before the first rated worse (the best first, ties in the order they came).
fn insert(items: &[Item], list: &mut Vec<usize>, index: usize) {
    let at = list.iter().position(|&other| items[index].rating < items[other].rating).unwrap_or(list.len());
    list.insert(at, index);
}

/// Whether the spheres for a hold at `center` (outward `outward`) touch nothing: `hanging` the body's six,
/// `leap` the climb's (`probe-using-spheres`). Ground at or under `floor_limit` is not counted (see
/// [`Query::floor_limit`]).
pub(crate) fn body_clear(
    cache: &CollideCache,
    center: Vec3,
    outward: Vec3,
    scale: f32,
    hanging: bool,
    leap: bool,
    floor_limit: f32,
    room: f32,
) -> bool {
    let keep = |v: &[Vec3; 3], normal: Vec3| !(normal.y >= MIN_NORMAL_Y && v[0].y.max(v[1].y).max(v[2].y) <= floor_limit);
    for (wanted, set) in [(hanging, false), (leap, true)] {
        if !wanted {
            continue;
        }
        if spheres(center, outward, scale, set, room).iter().any(|&(at, radius)| cache.overlaps_sphere_where(at, radius, keep)) {
            return false;
        }
    }
    true
}
