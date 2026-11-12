//! Jak objects (Adrii, 23/10: "cada cosa ocupa bloques de la rejilla pero no
//! es cúbica… una sola entidad de varios bloques, como camas o puertas
//! dobles"): a thing of a Jak level (a palm, a barrel, a bush, a rock, a
//! snowy pine…) cut out of it whole, its triangles cut into the metres of
//! the grid it fills. Each of those metres is a block of the world (its
//! place's piece block, `jak_pieces`) drawn as the thing's own triangles
//! there; together they are one object: put down at once from its item,
//! turned to the way it was put, and broken whole when any of its blocks
//! is, its item back.
//!
//! The objects are found in the levels by their textures' triangles,
//! joined where they touch (`jak_objects_catalog`, made by the session's
//! `gen_objects.py`): the biggest few of each kind in each place. The
//! objects put in the world are kept in the world's folder
//! (`dimensions/<namespace>/<path>/jak_objetos.jsonl`, a line an object
//! put down or taken away). The old root log remains an Overworld base.
use std::collections::HashMap;
use std::sync::{OnceLock, RwLock};

use glam::Vec3;

use crate::jak_items::{BlockLook, JakBlock, JakItem, JakKind};
use crate::jak_pieces::{Piece, PieceTri};
use crate::original_places::{self, PLACES};

pub use crate::jak_objects_catalog::OBJECTS;

/// One kind of object: where it is in its level.
#[derive(Clone, Copy, Debug)]
pub struct ObjectKind {
    /// Its block and item (`jak:<block>`).
    pub block: &'static str,
    pub title: &'static str,
    pub place: &'static str,
    pub group: &'static str,
    /// The textures it is made of.
    pub textures: &'static [&'static str],
    /// Its box: its corner in its level (metres) and the corner plus its
    /// size along its own axes, which are the level's turned `yaw` degrees
    /// about the upright (a building piece's: along its wall, up, out of
    /// it; 0 for a loose object, whose box is the level's).
    pub lo: [f32; 3],
    pub hi: [f32; 3],
    pub yaw: f32,
}

impl ObjectKind {
    /// A building piece (a wall, floor, roof, door frame or stairs), not a
    /// loose object.
    pub fn is_piece(&self) -> bool {
        matches!(self.group, "pared" | "suelo" | "tejado" | "marco" | "escalera")
    }

    /// These are vines hanging from the slums' upper walls, not bushes
    /// standing on the pavement. Keep that distinction in every generator.
    pub fn hangs_from_wall(&self) -> bool {
        self.textures.iter().any(|t| t.starts_with("city-slum-shrub-overhang"))
    }

    /// Leaf sheets and thin plant stems are decoration. Palms and pines
    /// keep their solid trunks; a shrub must never become a staircase.
    pub fn soft_vegetation(&self) -> bool { self.group == "planta" }
    pub fn versioned_foliage(&self) -> bool { self.block.starts_with("objeto_haven_colgante_") }
}

/// The objects' item blocks (a cube of their first texture: what the hand
/// holds and the inventory shows).
pub fn object_blocks() -> &'static [JakBlock] {
    static BLOCKS: OnceLock<Vec<JakBlock>> = OnceLock::new();
    BLOCKS.get_or_init(|| OBJECTS.iter().map(|o| JakBlock { path: o.block, look: BlockLook::All(o.textures[0]) }).collect())
}

pub fn object_items() -> &'static [JakItem] {
    static ITEMS: OnceLock<Vec<JakItem>> = OnceLock::new();
    ITEMS.get_or_init(|| OBJECTS.iter().map(|o| JakItem { path: o.block, title: o.title, kind: JakKind::Block }).collect())
}

/// Which object kind an item block is, by its name (`objeto_...`).
pub fn kind_of(path: &str) -> Option<usize> {
    if !path.starts_with("objeto_") {
        return None;
    }
    OBJECTS.iter().position(|o| o.block == path)
}

/// An object's metres: by its cell (0 up from its box's low corner), the
/// triangles in it; and how many metres it spans (x, y, z).
pub struct Shape {
    pub place: usize,
    pub cells: HashMap<(i32, i32, i32), Piece>,
    pub size: (i32, i32, i32),
    /// A real vertex of a hanging plant's upper root, on its rear z=0
    /// plane. Quarter turns put this point against the supporting wall.
    pub wall_root: Option<[f32; 3]>,
    /// Native actor metres to blocks, including its grid fit.
    pub actor_scale: f32,
}

/// Fit the complete mesh before cutting it. Uniform scale preserves its
/// proportions, UVs and every triangle; a fractional corner never decides
/// the grid origin. Buildings and attached foliage use their own frames.
fn fit_grid(tris:&mut [crate::jak_pieces::SourceTri],footprint:(i32,i32))->Option<(Vec3,f32)> {
    let lo=tris.iter().flat_map(|t|t.p).reduce(Vec3::min)?;
    let hi=tris.iter().flat_map(|t|t.p).reduce(Vec3::max)?;
    let extent=hi-lo;
    let scale=(footprint.0 as f32/extent.x).min(footprint.1 as f32/extent.z).min(1.0);
    let offset=Vec3::new((footprint.0 as f32-extent.x*scale)*0.5,0.0,(footprint.1 as f32-extent.z*scale)*0.5);
    for t in tris.iter_mut(){t.p=t.p.map(|p|(p-lo)*scale+offset);}
    Some((extent*scale+offset,scale))
}

/// The three native slum overhangs are the same kind of leaf curtain,
/// exported at oblique angles. Align their upper root row with a wall;
/// this is a rigid rotation/translation, never a clipped or flattened mesh.
fn align_overhang(tris: &mut [crate::jak_pieces::SourceTri]) -> Option<[f32; 3]> {
    let mut points: Vec<_> = tris.iter().flat_map(|t| t.p).collect();
    points.sort_by(|a,b| a.x.total_cmp(&b.x).then(a.y.total_cmp(&b.y)).then(a.z.total_cmp(&b.z)));
    points.dedup();
    let lo = points.iter().copied().reduce(Vec3::min)?;
    let hi = points.iter().copied().reduce(Vec3::max)?;
    let root_row: Vec<_> = points.iter().copied().filter(|p| p.y >= hi.y-(hi.y-lo.y)*0.2).collect();
    let left = *root_row.iter().min_by(|a,b|a.x.total_cmp(&b.x))?;
    let right = *root_row.iter().max_by(|a,b|a.x.total_cmp(&b.x))?;
    let mut along = Vec3::new(right.x-left.x,0.0,right.z-left.z).try_normalize()?;
    let mut out = Vec3::new(-along.z,0.0,along.x);
    let bottom = *points.iter().min_by(|a,b|a.y.total_cmp(&b.y))?;
    let root_middle = root_row.iter().copied().sum::<Vec3>() / root_row.len() as f32;
    if (bottom-root_middle).dot(out)<0.0 { along=-along;out=-out; }
    let turn = |p:Vec3| Vec3::new(p.dot(along),p.y,p.dot(out));
    let min = points.iter().copied().map(turn).reduce(Vec3::min)?;
    let max = points.iter().copied().map(turn).reduce(Vec3::max)?;
    // The top sits on an integral course; the leaves hang below it.
    let shift = Vec3::new(min.x,max.y-(max.y-min.y).ceil(),min.z);
    let root = points.iter().copied().map(turn).min_by(|a,b|a.z.total_cmp(&b.z).then(b.y.total_cmp(&a.y)))? - shift;
    if root.y < (max.y-min.y)*0.5 { return None; }
    for tri in tris { tri.p=tri.p.map(|p|turn(p)-shift); }
    Some(root.to_array())
}

/// A kind's shape, cut once (none without its level).
pub fn shape(kind: usize) -> Option<&'static Shape> {
    static SHAPES: OnceLock<Vec<OnceLock<Option<Shape>>>> = OnceLock::new();
    let shapes = SHAPES.get_or_init(|| (0..OBJECTS.len()).map(|_| OnceLock::new()).collect());
    shapes.get(kind)?.get_or_init(|| make_shape(kind)).as_ref()
}

/// Follow shared vertices across the extraction box so it selects entire barrels.
fn snow_barrel_mesh(
    source: &[crate::jak_pieces::SourceTri],
    wanted: &[bool],
    local: impl Fn(Vec3) -> Vec3,
    lo: Vec3,
    hi: Vec3,
) -> Vec<crate::jak_pieces::SourceTri> {
    let mut nearby = Vec::new();
    let mut vertices: HashMap<[i32; 3], Vec<usize>> = HashMap::new();
    let mut seeds = Vec::new();
    let key = |p: Vec3| (p * 1000.0).round().to_array().map(|v| v as i32);
    // A neighbouring barrel is at most a few source metres across.
    let padding = Vec3::splat(5.0);
    for tri in source {
        if !wanted.get(usize::from(tri.texture)).copied().unwrap_or(false) {
            continue;
        }
        let p = tri.p.map(&local);
        let center = (p[0] + p[1] + p[2]) / 3.0;
        if !center.cmpge(lo - padding).all() || !center.cmple(hi + padding).all() {
            continue;
        }
        let index = nearby.len();
        if center.cmpge(lo).all() && center.cmple(hi).all() {
            seeds.push(index);
        }
        for point in p {
            vertices.entry(key(point)).or_default().push(index);
        }
        nearby.push(crate::jak_pieces::SourceTri { p, ..*tri });
    }
    let mut selected = vec![false; nearby.len()];
    while let Some(index) = seeds.pop() {
        if std::mem::replace(&mut selected[index], true) {
            continue;
        }
        for point in nearby[index].p {
            if let Some(adjacent) = vertices.remove(&key(point)) {
                seeds.extend(adjacent);
            }
        }
    }
    nearby.into_iter().zip(selected).filter_map(|(tri, keep)| keep.then_some(tri)).collect()
}

fn make_shape(kind: usize) -> Option<Shape> {
    let o = &OBJECTS[kind];
    let place = PLACES.iter().position(|p| p.name == o.place)?;
    let actor = if crate::jak_decor::contains(o.block) {
        Some(crate::jak_decor::read(o.block)?)
    } else if original_places::street_actor(o.block).is_some() {
        Some(original_places::read_street_actor(o.block)?)
    } else { None };
    let source = match &actor {
        Some(triangles) => triangles.as_slice(),
        None => crate::jak_pieces::source(place)?.tris.as_slice(),
    };
    let textures = original_places::textures_of(&PLACES[place]);
    let wanted: Vec<bool> = textures.iter().map(|t| o.textures.contains(t)).collect();
    // Into its own axes: a loose object's whole metres of the level kept
    // (its corner on the grid), a building piece's from its exact corner.
    let a = o.yaw.to_radians();
    let (along, out) = (Vec3::new(a.cos(), 0.0, -a.sin()), Vec3::new(a.sin(), 0.0, a.cos()));
    let corner = if o.is_piece() { Vec3::from_array(o.lo) } else { Vec3::from_array(o.lo).floor() };
    let local = |p: Vec3| {
        let d = p - corner;
        Vec3::new(d.dot(along), d.y, d.dot(out))
    };
    let (lo, hi) = (local(Vec3::from_array(o.lo)) - 0.05, Vec3::from_array(o.hi) - Vec3::from_array(o.lo) + local(Vec3::from_array(o.lo)) + 0.05);
    let snow_barrel = o.block.starts_with("objeto_montana_barril_");
    let tris: Vec<crate::jak_pieces::SourceTri> = if snow_barrel {
        snow_barrel_mesh(source, &wanted, local, lo, hi)
    } else { source
        .iter()
        .filter(|t| wanted.get(usize::from(t.texture)).copied().unwrap_or(false))
        // Every triangle reaching into the box (a wall's are often bigger
        // than a piece): the cutting keeps what is inside.
        .filter(|t| {
            let [a, b, c] = t.p.map(local);
            let (tlo, thi) = (a.min(b).min(c), a.max(b).max(c));
            if !o.is_piece() {
                let m = (a + b + c) / 3.0;
                return m.cmpge(lo).all() && m.cmple(hi).all();
            }
            thi.cmpge(lo).all() && tlo.cmple(hi).all()
        })
        .map(|t| {
            let mut moved = *t;
            moved.p = t.p.map(local);
            moved
        })
        .collect() };
    if tris.is_empty() {
        return None;
    }
    // Its level's light is baked into its vertex colours (a palace plant
    // in shade, Spargus's palms near black: the game lit them itself).
    // Taken out of its level it stands under this world's sky, so its
    // colours are brought up to the texture as it is (128) on average,
    // keeping their own shading (23/10: the plants stood as dark slivers
    // all over the Wasteland).
    let mut tris = if o.block=="objeto_haven_bidon_fuego" {
        // Rebuild the damaged level fragment as a complete upright vessel:
        // closed base, outer wall, rim and inside, all in its original metal.
        burning_can(*textures.iter().position(|n|*n=="city-slum-burning-can").as_ref()? as u16)
    } else {tris};
    let rock = o.group == "roca";
    if rock {
        let turned = level_rock(&mut tris);
        let base = rock_base(&tris);
        diag::info!(World, "{}: levelled {:.1} degrees, base of {} triangles", o.block, turned, base.len());
        tris.extend(base);
    }
    let luma: f32 = tris.iter().flat_map(|t| t.color.iter()).map(|c| 0.299 * f32::from(c[0]) + 0.587 * f32::from(c[1]) + 0.114 * f32::from(c[2])).sum::<f32>()
        / (tris.len() * 3) as f32;
    if luma < 110.0 {
        let gain = if luma < 24.0 { 0.0 } else { 128.0 / luma };
        for t in &mut tris {
            for c in &mut t.color {
                for k in 0..3 {
                    c[k] = if gain == 0.0 { 128 } else { (f32::from(c[k]) * gain).round().clamp(0.0, 255.0) as u8 };
                }
            }
        }
    }
    let fit=if o.block=="objeto_haven_propaganda_bloque" {
        let lo=tris.iter().flat_map(|t|t.p).reduce(Vec3::min)?;
        let hi=tris.iter().flat_map(|t|t.p).reduce(Vec3::max)?;
        let scale=(hi-lo).max_element().recip();
        let offset=Vec3::new((1.0-(hi.x-lo.x)*scale)*0.5,0.0,(1.0-(hi.z-lo.z)*scale)*0.5);
        for t in &mut tris {t.p=t.p.map(|p|(p-lo)*scale+offset);}
        Some(((hi-lo)*scale+offset,scale))
    } else if o.block=="objeto_haven_bidon_fuego" {
        let lo=tris.iter().flat_map(|t|t.p).reduce(Vec3::min)?;let hi=tris.iter().flat_map(|t|t.p).reduce(Vec3::max)?;
        let extent=hi-lo;let scale=(0.8/extent.x).min(0.9/extent.y).min(0.8/extent.z);
        let offset=Vec3::new((1.-extent.x*scale)*0.5,0.,(1.-extent.z*scale)*0.5);
        for t in &mut tris{t.p=t.p.map(|p|(p-lo)*scale+offset);}
        Some((extent*scale+offset,scale))
    }
    else if snow_barrel {
        let low = tris.iter().flat_map(|t| t.p).reduce(Vec3::min)?;
        let high = tris.iter().flat_map(|t| t.p).reduce(Vec3::max)?;
        for tri in &mut tris { tri.p = tri.p.map(|p| p - low); }
        Some((high - low, 1.0))
    }
    else if o.block=="objeto_spargus_palmera_completa" {Some(fit_grid(&mut tris,(5,5))?)}
    else if o.block=="objeto_haven_propaganda_ajustada" || o.group=="decorado" {Some(fit_grid(&mut tris,(1,1))?)} else{None};
    let wall_root = if o.versioned_foliage() { Some(align_overhang(&mut tris)?) } else { None };
    let top = if let Some((top,_))=fit {(top-0.00001).ceil()}
        else if wall_root.is_some() { tris.iter().flat_map(|t|t.p).reduce(Vec3::max)?.ceil() }
        else if rock { (tris.iter().flat_map(|t|t.p).reduce(Vec3::max)? - 0.0001).ceil() } else { (hi - 0.05).ceil() };
    let size = (top.x.max(1.0) as i32, top.y.max(1.0) as i32, top.z.max(1.0) as i32);
    let cut = crate::jak_pieces::cut_area(&crate::jak_pieces::Source::new(tris), (0, size.0 - 1), (0, size.2 - 1));
    let cells: HashMap<(i32, i32, i32), Piece> = cut
        .pieces
        .into_iter()
        .filter(|((_, y, _), p)| (0..size.1).contains(y)
            && p.tris.iter().map(tri_area).sum::<f32>() > if wall_root.is_some()||fit.is_some()||rock{0.0}else{0.002})
        .collect();
    (!cells.is_empty()).then_some(Shape { place, cells, size, wall_root,actor_scale:0.75*fit.map_or(1.0,|(_,s)|s) })
}

/// Footprint columns of a rock's base, metres.
const ROCK_COLUMN: f32 = 0.25;

/// The lowest height of the triangles' surface over (x, z), if any covers it.
fn lowest_surface(tris: &[crate::jak_pieces::SourceTri], x: f32, z: f32) -> Option<f32> {
    let mut low: Option<f32> = None;
    for t in tris {
        let [a, b, c] = t.p;
        let d = (b.z - c.z) * (a.x - c.x) + (c.x - b.x) * (a.z - c.z);
        if d.abs() < 1e-9 { continue; }
        let u = ((b.z - c.z) * (x - c.x) + (c.x - b.x) * (z - c.z)) / d;
        let v = ((c.z - a.z) * (x - c.x) + (a.x - c.x) * (z - c.z)) / d;
        let w = 1.0 - u - v;
        if u < -1e-4 || v < -1e-4 || w < -1e-4 { continue; }
        let y = u * a.y + v * b.y + w * c.y;
        low = Some(low.map_or(y, |l| l.min(y)));
    }
    low
}

/// stand-in: the original rocks sit on the level's slopes, bottoms tilted and
/// hollow. Adrii (06/11) wants them on flat ground: a plane fitted to the lowest
/// vertex of each footprint column (leaving out the hollow's higher ones) is turned
/// level (a rigid turn, as `align_overhang`), the rock set on y=0 and its footprint
/// centred in the grid it fills. Returns the turn, degrees.
fn level_rock(tris: &mut [crate::jak_pieces::SourceTri]) -> f32 {
    let mut low: HashMap<(i32, i32), Vec3> = HashMap::new();
    for p in tris.iter().flat_map(|t| t.p) {
        let e = low.entry((p.x.floor() as i32, p.z.floor() as i32)).or_insert(p);
        if p.y < e.y { *e = p; }
    }
    let mut points: Vec<Vec3> = low.into_values().collect();
    points.sort_by(|a, b| a.x.total_cmp(&b.x).then(a.z.total_cmp(&b.z)));
    let mut turn = glam::Quat::IDENTITY;
    let mut plane: Option<(f32, f32, f32)> = None;
    for _ in 0..4 {
        // Without the points a first fit shows to be the hollow's.
        let used: Vec<Vec3> = match plane {
            None => points.clone(),
            Some((a, b, c)) => points.iter().copied().filter(|p| p.y - (a * p.x + b * p.z + c) < 0.25).collect(),
        };
        if used.len() < 3 { break; }
        let mean = used.iter().copied().sum::<Vec3>() / used.len() as f32;
        let (mut sxx, mut sxz, mut szz, mut sxy, mut szy) = (0.0, 0.0, 0.0, 0.0, 0.0);
        for p in &used {
            let (x, y, z) = (p.x - mean.x, p.y - mean.y, p.z - mean.z);
            sxx += x * x; sxz += x * z; szz += z * z; sxy += x * y; szy += z * y;
        }
        let det = sxx * szz - sxz * sxz;
        if det < 1e-4 { break; }
        let (a, b) = ((sxy * szz - szy * sxz) / det, (szy * sxx - sxy * sxz) / det);
        plane = Some((a, b, mean.y - a * mean.x - b * mean.z));
        turn = glam::Quat::from_rotation_arc(Vec3::new(-a, 1.0, -b).normalize(), Vec3::Y);
    }
    let centre = tris.iter().flat_map(|t| t.p).fold(Vec3::ZERO, |s, p| s + p) / (tris.len() * 3).max(1) as f32;
    for t in tris.iter_mut() { t.p = t.p.map(|p| turn * (p - centre) + centre); }
    let lo = tris.iter().flat_map(|t| t.p).fold(Vec3::splat(f32::MAX), Vec3::min);
    let hi = tris.iter().flat_map(|t| t.p).fold(Vec3::splat(f32::MIN), Vec3::max);
    let grid = |extent: f32| (extent - 1e-4).ceil().max(1.0);
    let shift = Vec3::new((grid(hi.x - lo.x) - (hi.x - lo.x)) * 0.5 - lo.x, -lo.y, (grid(hi.z - lo.z) - (hi.z - lo.z)) * 0.5 - lo.z);
    for t in tris.iter_mut() { t.p = t.p.map(|p| p + shift); }
    turn.to_axis_angle().1.to_degrees()
}

/// stand-in: closes the hollow under a levelled rock (the original sits on the
/// ground, which closes it): a flat base on y=0 under its footprint and side faces
/// up to its lowest surface in each column, in the rock's own texture, colours and
/// texture density.
fn rock_base(tris: &[crate::jak_pieces::SourceTri]) -> Vec<crate::jak_pieces::SourceTri> {
    use glam::Vec2;
    let Some(first) = tris.first() else { return Vec::new() };
    let lo = tris.iter().flat_map(|t| t.p).fold(Vec3::splat(f32::MAX), Vec3::min);
    let hi = tris.iter().flat_map(|t| t.p).fold(Vec3::splat(f32::MIN), Vec3::max);
    let step = ROCK_COLUMN;
    let (x0, z0) = ((lo.x / step).floor() * step, (lo.z / step).floor() * step);
    let (nx, nz) = (((hi.x - x0) / step).ceil() as usize + 1, ((hi.z - z0) / step).ceil() as usize + 1);
    // Per column: the lowest surface over it, if its middle is under the rock.
    let mut height: Vec<Option<f32>> = vec![None; nx * nz];
    for j in 0..nz {
        for i in 0..nx {
            let (cx, cz) = (x0 + (i as f32 + 0.5) * step, z0 + (j as f32 + 0.5) * step);
            if lowest_surface(tris, cx, cz).is_none() { continue; }
            let mut low = f32::MAX;
            for (dx, dz) in [(-1.0, -1.0), (0.0, -1.0), (1.0, -1.0), (-1.0, 0.0), (0.0, 0.0), (1.0, 0.0), (-1.0, 1.0), (0.0, 1.0), (1.0, 1.0)] {
                if let Some(y) = lowest_surface(tris, cx + dx * step / 3.0, cz + dz * step / 3.0) { low = low.min(y); }
            }
            height[j * nx + i] = Some(low.max(0.0));
        }
    }
    let at = |i: i64, j: i64| if i < 0 || j < 0 || i >= nx as i64 || j >= nz as i64 { None } else { height[j as usize * nx + i as usize] };
    // The rock's texture density (texture per metre) and colours.
    let (mut uv_area, mut area) = (0.0f32, 0.0f32);
    let (mut sum, mut count) = ([0u32; 4], 0u32);
    for t in tris {
        let [a, b, c] = t.p;
        area += (b - a).cross(c - a).length() * 0.5;
        let [ua, ub, uc] = t.uv;
        uv_area += (ub - ua).perp_dot(uc - ua).abs() * 0.5;
        for c in t.color { for k in 0..4 { sum[k] += u32::from(c[k]); } count += 1; }
    }
    let density = if area > 1e-6 && uv_area > 1e-9 { (uv_area / area).sqrt() } else { 0.25 };
    let color = sum.map(|s| (s / count.max(1)) as u8);
    let mut out = Vec::new();
    let mut quad = |p: [Vec3; 4], outward: Vec3| {
        let uv = p.map(|v| {
            let along = if outward.y != 0.0 { Vec2::new(v.x, v.z) } else if outward.x != 0.0 { Vec2::new(v.z, v.y) } else { Vec2::new(v.x, v.y) };
            along * density
        });
        let flip = (p[1] - p[0]).cross(p[2] - p[0]).dot(outward) < 0.0;
        for k in if flip { [[0, 2, 1], [0, 3, 2]] } else { [[0, 1, 2], [0, 2, 3]] } {
            out.push(crate::jak_pieces::SourceTri { p: k.map(|n| p[n]), uv: k.map(|n| uv[n]), texture: first.texture, color: [color; 3], repeat: [true; 2], blend: false });
        }
    };
    for j in 0..nz as i64 {
        // The floor, one strip for each run of columns.
        let mut i = 0i64;
        while i < nx as i64 {
            if at(i, j).is_none() { i += 1; continue; }
            let start = i;
            while at(i, j).is_some() { i += 1; }
            let (xa, xb) = (x0 + start as f32 * step, x0 + i as f32 * step);
            let (za, zb) = (z0 + j as f32 * step, z0 + (j + 1) as f32 * step);
            quad([Vec3::new(xa, 0.0, za), Vec3::new(xb, 0.0, za), Vec3::new(xb, 0.0, zb), Vec3::new(xa, 0.0, zb)], -Vec3::Y);
        }
        for i in 0..nx as i64 {
            let Some(h) = at(i, j) else { continue };
            if h < 1e-4 { continue; }
            let (xa, xb) = (x0 + i as f32 * step, x0 + (i + 1) as f32 * step);
            let (za, zb) = (z0 + j as f32 * step, z0 + (j + 1) as f32 * step);
            for (di, dj) in [(-1i64, 0i64), (1, 0), (0, -1), (0, 1)] {
                if at(i + di, j + dj).is_some() { continue; }
                let (outward, edge) = match (di, dj) {
                    (-1, _) => (-Vec3::X, [(xa, za), (xa, zb)]),
                    (1, _) => (Vec3::X, [(xb, za), (xb, zb)]),
                    (_, -1) => (-Vec3::Z, [(xa, za), (xb, za)]),
                    _ => (Vec3::Z, [(xa, zb), (xb, zb)]),
                };
                quad([Vec3::new(edge[0].0, 0.0, edge[0].1), Vec3::new(edge[1].0, 0.0, edge[1].1), Vec3::new(edge[1].0, h, edge[1].1), Vec3::new(edge[0].0, h, edge[0].1)], outward);
            }
        }
    }
    out
}

fn burning_can(texture:u16)->Vec<crate::jak_pieces::SourceTri>{
    use glam::Vec2;
    let mut out=Vec::new();let sides=10;
    let rings=[(0.,0.30),(0.05,0.32),(0.72,0.38),(0.84,0.40),(0.88,0.40),(0.88,0.36),(0.08,0.28)];
    let point=|y:f32,r:f32,i:usize|{let a=(i%sides)as f32*std::f32::consts::TAU/sides as f32;Vec3::new(0.5+r*a.cos(),y,0.5+r*a.sin())};
    let mut tri=|p:[Vec3;3],uv:[Vec2;3]|out.push(crate::jak_pieces::SourceTri{p,uv,texture,color:[[128;4];3],repeat:[true;2],blend:false});
    for pair in rings.windows(2){let[(y0,r0),(y1,r1)]=[pair[0],pair[1]];
        for i in 0..sides{
            let p=[point(y0,r0,i),point(y0,r0,i+1),point(y1,r1,i+1),point(y1,r1,i)];
            let u=i as f32/sides as f32;let v=(i+1)as f32/sides as f32;
            let uv=[Vec2::new(u,1.-y0/0.88),Vec2::new(v,1.-y0/0.88),Vec2::new(v,1.-y1/0.88),Vec2::new(u,1.-y1/0.88)];
            for k in [[0,2,1],[0,3,2]]{tri(k.map(|n|p[n]),k.map(|n|uv[n]));}
        }
    }
    for (y,r,inside) in [(0.,0.30,false),(0.08,0.28,true)]{for i in 0..sides{
        let a=point(y,r,i);let b=point(y,r,i+1);let p=if inside{[Vec3::new(0.5,y,0.5),b,a]}else{[Vec3::new(0.5,y,0.5),a,b]};
        tri(p,p.map(|v|Vec2::new(v.x,v.z)));
    }}
    out
}

#[cfg(test)]mod rock_tests{
 use super::*;
 /// Adrii 06/11: the rocks stood tilted, floating and hollow. Each now has its lowest
 /// vertex on y=0 and a bottom layer of cells under its whole footprint.
 #[test]fn rocks_sit_level_with_a_closed_base(){
  let mut seen=0;
  for(kind,o)in OBJECTS.iter().enumerate().filter(|(_,o)|o.group=="roca"){
   let Some(s)=shape(kind)else{continue};seen+=1;
   let world:Vec<[Vec3;3]>=s.cells.iter().flat_map(|(c,p)|p.tris.iter().map(move|t|t.p.map(|v|Vec3::from_array(v)+Vec3::new(c.0 as f32,c.1 as f32,c.2 as f32)))).collect();
   let lowest=world.iter().flatten().map(|v|v.y).fold(f32::MAX,f32::min);
   assert!(lowest.abs()<1e-3,"{} lowest vertex {lowest}",o.block);
   let source:Vec<crate::jak_pieces::SourceTri>=world.iter().map(|p|crate::jak_pieces::SourceTri{p:*p,uv:[glam::Vec2::ZERO;3],texture:0,color:[[0;4];3],repeat:[true;2],blend:false}).collect();
   let mut columns=0;
   for x in 0..s.size.0*4{for z in 0..s.size.2*4{
    let(cx,cz)=((x as f32+0.5)*0.25,(z as f32+0.5)*0.25);
    if lowest_surface(&source,cx,cz).is_none(){continue;}
    columns+=1;
    assert!(s.cells.contains_key(&(x/4,0,z/4)),"{}: no bottom cell under ({cx}, {cz})",o.block);
   }}
   assert!(columns>0);
   let(ax,az)=world.iter().flatten().fold((f32::MAX,f32::MAX),|m,v|(m.0.min(v.x),m.1.min(v.z)));
   assert!(ax>=-1e-3&&az>=-1e-3,"{} outside its grid",o.block);
   let top=world.iter().flatten().map(|v|v.y).fold(f32::MIN,f32::max);
   assert!(top<=s.size.1 as f32+1e-3&&top>s.size.1 as f32-1.0,"{} height {top} vs {}",o.block,s.size.1);
   eprintln!("{}: size {:?}, {} cells, {columns} base columns, top {top:.2}",o.block,s.size,s.cells.len());
  }
  assert!(seen>=3,"rocks cut: {seen}");
 }
}

#[cfg(test)]mod palm_tests{
 use super::*;
 #[test]fn detail30_palm_has_grounded_trunk_and_native_crown_inside_grid(){
  let kind=kind_of("objeto_spargus_palmera_completa").unwrap();let s=shape(kind).unwrap();
  assert!(s.size.0<=5&&s.size.2<=5);
  let names=original_places::textures_of(&PLACES[s.place]);
  let mut trunk_min=f32::INFINITY;let mut leaves_max=f32::NEG_INFINITY;
  for(cell,p)in &s.cells{for t in &p.tris{
   if names[t.texture as usize].contains("trunk"){for v in t.p{trunk_min=trunk_min.min(cell.1 as f32+v[1]);}}
   if names[t.texture as usize].contains("leaf"){for v in t.p{leaves_max=leaves_max.max(cell.1 as f32+v[1]);}}
  }}
  assert!(trunk_min<0.7,"native trunk must reach the ground: {trunk_min}");
  assert!(leaves_max>2.,"native crown must remain above the trunk: {leaves_max}");
  assert_eq!(placement_kind(kind_of("objeto_spargus_planta_2").unwrap()),kind);
 }
}

fn tri_area(t: &PieceTri) -> f32 {
    let [a, b, c] = t.p.map(Vec3::from_array);
    (b - a).cross(c - a).length() * 0.5
}

/// A point of a metre turned a quarter turn clockwise (seen from above)
/// about the metre's middle.
fn turn_point([x, y, z]: [f32; 3]) -> [f32; 3] {
    [1.0 - z, y, x]
}

/// A cell of a footprint `(w, d)` turned `turns` quarter turns clockwise.
fn turn_cell((x, y, z): (i32, i32, i32), (w, d): (i32, i32), turns: u8) -> (i32, i32, i32) {
    let (mut x, mut z, mut w, mut d) = (x, z, w, d);
    for _ in 0..turns % 4 {
        (x, z) = (d - 1 - z, x);
        (w, d) = (d, w);
    }
    let _ = (w, d);
    (x, y, z)
}

/// An object put in the world: its kind, the world block of its turned
/// box's low corner, and its quarter turns.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Instance {
    pub kind: u16,
    pub origin: (i32, i32, i32),
    pub turns: u8,
}

impl Instance {
    /// Its blocks: world position and the cell of its shape each shows.
    pub fn cells(&self) -> Vec<((i32, i32, i32), (i32, i32, i32))> {
        let Some(shape) = shape(usize::from(self.kind)) else { return Vec::new() };
        shape
            .cells
            .keys()
            .map(|&cell| {
                let (x, y, z) = turn_cell(cell, (shape.size.0, shape.size.2), self.turns);
                ((self.origin.0 + x, self.origin.1 + y, self.origin.2 + z), cell)
            })
            .collect()
    }

    pub fn wall_root(&self) -> Option<[f64;3]> {
        let shape=shape(usize::from(self.kind))?;
        let mut root=shape.wall_root?;
        let(mut w,mut d)=(shape.size.0 as f32,shape.size.2 as f32);
        for _ in 0..self.turns%4 { root=[d-root[2],root[1],root[0]];std::mem::swap(&mut w,&mut d); }
        Some([f64::from(root[0])+f64::from(self.origin.0),f64::from(root[1])+f64::from(self.origin.1),f64::from(root[2])+f64::from(self.origin.2)])
    }
}

/// Attach the native root to a wall block's outward face: 0 south, 1
/// west, 2 north, 3 east, matching Instance's clockwise mesh rotations.
pub fn hanging_at(kind:usize,wall:(i32,i32,i32),turns:u8)->Option<Instance> {
    let shape=shape(kind)?;
    shape.wall_root?;
    let mut instance=Instance{kind:kind as u16,origin:(0,0,0),turns:turns%4};
    let root=instance.wall_root()?;
    instance.origin=(wall.0-root[0].floor() as i32,wall.1-root[1].floor() as i32,wall.2-root[2].floor() as i32);
    match instance.turns {
        0=>instance.origin.2=wall.2+1,
        1=>instance.origin.0=wall.0-shape.size.2,
        2=>instance.origin.2=wall.2-shape.size.2,
        _=>instance.origin.0=wall.0+1,
    }
    Some(instance)
}

/// Test the mesh's actual root against the visible outer face, not the
/// presence of some block somewhere behind its bounding box.
pub fn supported_wall(instance:&Instance,boxes:impl Fn((i32,i32,i32))->Vec<[f64;6]>)->bool {
    let Some(root)=instance.wall_root() else{return false;};
    let mut wall=(root[0].floor() as i32,root[1].floor() as i32,root[2].floor() as i32);
    let (axis,face)=match instance.turns%4 {
        0=>{wall.2-=1;(2,1.0)},1=>(0,0.0),2=>(2,0.0),_=>{wall.0-=1;(0,1.0)},
    };
    let local=[root[0]-f64::from(wall.0),root[1]-f64::from(wall.1),root[2]-f64::from(wall.2)];
    boxes(wall).iter().any(|b|(b[axis+if face==1.0{3}else{0}]-face).abs()<1e-6
        && (0..3).filter(|a|*a!=axis).all(|a|local[a]>=b[a]&&local[a]<=b[a+3]))
}

/// Older inventory items use the corrected attachment when placed again;
/// their existing saved instances retain the original catalog index.
pub fn placement_kind(kind:usize)->usize {
    let Some(o)=OBJECTS.get(kind) else{return kind;};
    if o.block.starts_with("objeto_haven_bidon_"){return kind_of("objeto_haven_bidon_fuego").unwrap();}
    if o.block=="objeto_spargus_planta_2"||o.group=="palmera"{return kind_of("objeto_spargus_palmera_completa").unwrap();}
    if matches!(o.block,"objeto_haven_propaganda"|"objeto_haven_propaganda_ajustada") {return kind_of("objeto_haven_propaganda_bloque").unwrap();}
    if o.hangs_from_wall()&&!o.versioned_foliage() {
        let suffix=o.block.rsplit('_').next().unwrap_or("");
        if let Some(next)=OBJECTS.iter().position(|o|o.versioned_foliage()&&o.block.ends_with(&format!("_{suffix}"))) {return next;}
    }
    kind
}

#[derive(Default)]
struct Table {
    file: Option<std::path::PathBuf>,
    /// Pieces of the world's own hamlets taken away (`jak_hamlets`).
    removed: std::collections::HashSet<(u16, (i32, i32, i32), u8)>,
    objects: HashMap<u64, Instance>,
    by_block: HashMap<(i32, i32, i32), (u64, (i32, i32, i32))>,
    next: u64,
    upgrades: Vec<(Instance,Instance)>,
}

impl Table {
    fn insert(&mut self, instance: Instance) -> u64 {
        let id = self.next;
        self.next += 1;
        for (at, cell) in instance.cells() {
            self.by_block.insert(at, (id, cell));
        }
        self.objects.insert(id, instance);
        id
    }

    fn take(&mut self, id: u64) -> Option<Instance> {
        let instance = self.objects.remove(&id)?;
        for (at, _) in instance.cells() {
            if self.by_block.get(&at).is_some_and(|(i, _)| *i == id) {
                self.by_block.remove(&at);
            }
        }
        Some(instance)
    }

    fn log(&self, line: [i32; 6]) -> bool {
        let Some(file) = &self.file else { return true };
        use std::io::Write;
        let write = || -> std::io::Result<()> {
            if let Some(parent)=file.parent() {std::fs::create_dir_all(parent)?;}
            let mut f=std::fs::OpenOptions::new().create(true).append(true).open(file)?;
            writeln!(f,"{}",serde_json::to_string(&line).expect("integer object record"))?;
            f.sync_data()
        };
        if let Err(error)=write() {
            diag::warn!(World,"Cannot save Jak object record {}: {error}",file.display());
            return false;
        }
        true
    }
}

struct Tables {
    root: Option<std::path::PathBuf>,
    dimension: String,
    current: Table,
    parked: HashMap<String, Table>,
}

impl Default for Tables {
    fn default()->Self {Self{root:None,dimension:"minecraft:overworld".into(),current:Table::default(),parked:HashMap::new()}}
}
impl std::ops::Deref for Tables {
    type Target=Table;
    fn deref(&self)->&Table {&self.current}
}
impl std::ops::DerefMut for Tables {
    fn deref_mut(&mut self)->&mut Table {&mut self.current}
}

fn table() -> &'static RwLock<Tables> {
    static TABLE: OnceLock<RwLock<Tables>> = OnceLock::new();
    TABLE.get_or_init(Default::default)
}

fn dimension_log_path(root:&std::path::Path,dimension:&str)->Option<std::path::PathBuf> {
    let (namespace,path)=dimension.split_once(':')?;
    let valid=|s:&str|!s.is_empty()&&s!="."&&s!=".."
        &&s.bytes().all(|c|c.is_ascii_lowercase()||c.is_ascii_digit()||b"_.-".contains(&c));
    if !valid(namespace)||!path.split('/').all(valid) {return None;}
    Some(root.join("dimensions").join(namespace).join(path).join("jak_objetos.jsonl"))
}

fn replay_log(fresh:&mut Table,file:&std::path::Path) {
    if let Ok(text)=std::fs::read_to_string(file) {
        for line in text.lines() {
            let Ok([op, kind, x, y, z, turns]) = serde_json::from_str::<[i32; 6]>(line) else { continue };
            if !(0..=3).contains(&op)||kind<0||kind as usize>=OBJECTS.len()||!(0..4).contains(&turns) {continue;}
            let instance = Instance { kind: kind as u16, origin: (x, y, z), turns: turns as u8 };
            if op==3 {
                if let Some(next)=compact_replacement(instance){apply_compact(fresh,instance,next);}
            } else if op == 2 {
                fresh.removed.insert((instance.kind, instance.origin, instance.turns));
            } else if op == 1 {
                fresh.insert(instance);
            } else if let Some(id) = fresh.objects.iter().find(|(_, i)| **i == instance).map(|(id, _)| *id) {
                fresh.take(id);
            }
        }
    }
}

pub fn compact_replacement(old:Instance)->Option<Instance> {
    if !matches!(OBJECTS.get(old.kind as usize)?.block,"objeto_haven_propaganda"|"objeto_haven_propaganda_ajustada") {return None;}
    let size=shape(old.kind as usize)?.size;
    let (w,d)=if old.turns%2==0{(size.0,size.2)}else{(size.2,size.0)};
    placed(placement_kind(old.kind as usize),(old.origin.0+w/2,old.origin.1,old.origin.2+d/2),old.turns)
}

fn apply_compact(t:&mut Table,old:Instance,next:Instance) {
    if let Some(id)=t.objects.iter().find(|(_,i)|**i==old).map(|(id,_)|*id){t.take(id);}
    t.removed.insert((old.kind,old.origin,old.turns));
    t.insert(next);
    if !t.upgrades.contains(&(old,next)){t.upgrades.push((old,next));}
}

pub fn compact_jobs(pos:(i32,i32,i32),reach:i32)->Vec<(Instance,Instance)> {
    table().read().map(|t|t.upgrades.iter().filter(|(i,_)|(i.origin.0-pos.0).abs()<=reach&&(i.origin.2-pos.2).abs()<=reach).copied().collect()).unwrap_or_default()
}

/// One durable record commits both ownership changes. The caller has
/// checked that every source cell is still intact in the loaded world.
pub fn commit_compact(old:Instance)->Option<Instance> {
    let next=compact_replacement(old)?;
    let mut t=table().write().ok()?;
    if t.removed.contains(&(old.kind,old.origin,old.turns)) {return None;}
    if next.cells().iter().any(|(p,_)|t.by_block.get(p).is_some_and(|(id,_)|t.objects[id]!=old)){return None;}
    if !t.log([3,old.kind.into(),old.origin.0,old.origin.1,old.origin.2,old.turns.into()]){return None;}
    apply_compact(&mut t.current,old,next);
    Some(next)
}

/// Known street props and placed objects near an already streamed view.
/// This never scans or generates terrain in search of effects.
pub fn nearby_posts(pos:(i32,i32,i32),reach:i32)->Vec<Instance> {
    let Ok(t)=table().read()else{return Vec::new()};
    let near=|i:&&Instance|(i.origin.0-pos.0).abs()<=reach&&(i.origin.2-pos.2).abs()<=reach
        &&(OBJECTS[i.kind as usize].block.starts_with("objeto_haven_propaganda")||OBJECTS[i.kind as usize].block=="objeto_haven_bidon_fuego");
    let mut found:Vec<_>=t.objects.values().filter(near).copied().collect();
    if t.dimension=="minecraft:overworld" {
        if let Some((seed,_))=original_places::current_world(){
            found.extend(crate::jak_town::known_props_near(seed,pos,reach).iter().filter(near)
                .filter(|i|!t.removed.contains(&(i.kind,i.origin,i.turns))).copied());
        }
    }
    found
}

pub fn retired_hoops(pos:(i32,i32,i32),reach:i32)->Vec<Instance> {
    let Ok(t)=table().read()else{return Vec::new()};
    let near=|i:&&Instance|(i.origin.0-pos.0).abs()<=reach&&(i.origin.2-pos.2).abs()<=reach
        &&OBJECTS[i.kind as usize].block=="objeto_spargus_barril_1";
    let mut found:Vec<_>=t.objects.values().filter(near).copied().collect();
    if t.dimension=="minecraft:overworld" {
        if let Some((seed,_))=original_places::current_world(){
            found.extend(crate::jak_town::known_props_near(seed,pos,reach).iter().filter(near)
                .filter(|i|!t.removed.contains(&(i.kind,i.origin,i.turns))).copied());
        }
    }
    found
}

fn read_table(root:Option<&std::path::Path>,dimension:&str)->Table {
    let mut fresh=Table{file:root.and_then(|d|dimension_log_path(d,dimension)),..Table::default()};
    if let Some(root)=root {
        // New logs contain deltas, not a snapshot. Replay the untouched
        // legacy base on every Overworld load, then this dimension's deltas.
        if dimension=="minecraft:overworld" {replay_log(&mut fresh,&root.join("jak_objetos.jsonl"));}
        if let Some(file)=fresh.file.clone() {replay_log(&mut fresh,&file);}
    }
    fresh
}

/// Start a saved world (or an unsaved one) with its Overworld table.
/// The destination must be activated before terrain workers are started.
pub fn load(dir: Option<&std::path::Path>) {
    let fresh=Tables{root:dir.map(std::path::Path::to_owned),current:read_table(dir,"minecraft:overworld"),..Tables::default()};
    if let Ok(mut held) = table().write() {
        *held = fresh;
    }
}

/// A complete object at the same coordinates in another dimension has a
/// different owner and removal log, including naturally generated objects.
pub fn activate_dimension(dimension:&str) {
    if dimension_log_path(std::path::Path::new("."),dimension).is_none() {
        diag::warn!(World,"Invalid Jak object dimension: {dimension}");return;
    }
    let Ok(mut held)=table().write() else{return;};
    if held.dimension==dimension {return;}
    let incoming=held.parked.remove(dimension).unwrap_or_else(||read_table(held.root.as_deref(),dimension));
    let previous=std::mem::replace(&mut held.current,incoming);
    let previous_dimension=std::mem::replace(&mut held.dimension,dimension.to_owned());
    held.parked.insert(previous_dimension,previous);
}

/// Puts `instance` in the world's table (none of its blocks another
/// object's), and logs it.
pub fn add(instance: Instance) -> bool {
    let Ok(mut t) = table().write() else { return false };
    if instance.cells().iter().any(|(at, _)| t.by_block.contains_key(at)) {
        return false;
    }
    if !t.log([1, i32::from(instance.kind), instance.origin.0, instance.origin.1, instance.origin.2, i32::from(instance.turns)]) {return false;}
    t.insert(instance);
    true
}

/// Takes away the object one of whose blocks is `pos`: it, and its blocks.
pub fn remove_at(pos: (i32, i32, i32)) -> Option<(Instance, Vec<(i32, i32, i32)>)> {
    // A piece of a hamlet's house: kept taken away.
    if let Some((generated, _)) = generated_at(pos).or_else(||generated_foliage_at(pos)) {
        let mut t = table().write().ok()?;
        if !t.log([2, i32::from(generated.kind), generated.origin.0, generated.origin.1, generated.origin.2, i32::from(generated.turns)]) {return None;}
        t.removed.insert((generated.kind, generated.origin, generated.turns));
        let blocks = generated.cells().into_iter().map(|(at, _)| at).collect();
        return Some((generated, blocks));
    }
    let mut t = table().write().ok()?;
    let (id, _) = *t.by_block.get(&pos)?;
    let instance = *t.objects.get(&id)?;
    if !t.log([0, i32::from(instance.kind), instance.origin.0, instance.origin.1, instance.origin.2, i32::from(instance.turns)]) {return None;}
    t.take(id)?;
    let blocks = instance.cells().into_iter().map(|(at, _)| at).collect();
    Some((instance, blocks))
}

/// The object a world block belongs to, if one.
pub fn at(pos: (i32, i32, i32)) -> Option<Instance> {
    let placed = {
        let t = table().read().ok()?;
        t.by_block.get(&pos).and_then(|(id, _)| t.objects.get(id).copied())
    };
    placed.or_else(|| generated_at(pos).or_else(||generated_foliage_at(pos)).map(|(instance, _)| instance))
}

/// A piece of the world's own hamlets at `pos` and the cell of its shape
/// there, not taken away (and none where something was put over it, nor
/// inside one of the games' places, which have none).
fn generated_at(pos: (i32, i32, i32)) -> Option<(Instance, (i32, i32, i32))> {
    {
        let t = table().read().ok()?;
        if t.dimension!="minecraft:overworld"||t.by_block.contains_key(&pos) {
            return None;
        }
    }
    if original_places::inside_current_place(pos.0, pos.2) {
        return None;
    }
    let seed=original_places::current_world()?.0;
    // A tower's native cells belong to its own source, not to an object
    // in an optional imported hamlet. Avoid reconstructing those levels
    // while collision or a street path asks about the tower.
    if crate::jak_tower::source_at_seed(seed,pos).is_some(){return None;}
    let found=crate::jak_town::cached_original_at(seed,pos)
        .or_else(||crate::jak_hamlets::instance_at(pos))
        .or_else(||crate::jak_town::original_at(pos))?;
    let t = table().read().ok()?;
    (!t.removed.contains(&(found.0.kind, found.0.origin, found.0.turns))).then_some(found)
}

fn generated_foliage_at(pos:(i32,i32,i32))->Option<(Instance,(i32,i32,i32))> {
    {
        let t=table().read().ok()?;
        if t.dimension!="minecraft:overworld"||t.by_block.contains_key(&pos) {return None;}
    }
    let found=crate::jak_town::foliage_at(pos)?;
    let t=table().read().ok()?;
    (!t.removed.contains(&(found.0.kind,found.0.origin,found.0.turns))).then_some(found)
}

/// The piece the block at `pos` shows if it is an object's: its place, and
/// its triangles turned as the object is.
fn piece_source(pos: (i32, i32, i32), foliage:bool) -> Option<(usize, &'static Piece, u8, bool)> {
    let placed = {
        let t = table().read().ok()?;
        t.by_block.get(&pos).and_then(|&(id, cell)| Some((*t.objects.get(&id)?, cell)))
    };
    let (instance, cell) = match placed {
        Some(found) => found,
        None => if foliage {generated_foliage_at(pos)?} else {generated_at(pos)?},
    };
    if OBJECTS[usize::from(instance.kind)].versioned_foliage()!=foliage {return None;}
    let shape = shape(usize::from(instance.kind))?;
    Some((shape.place, shape.cells.get(&cell)?, instance.turns % 4, OBJECTS[usize::from(instance.kind)].soft_vegetation()))
}

pub fn piece_at(pos: (i32, i32, i32)) -> Option<(usize, Piece)> {
    let (place, source, turns, _) = piece_source(pos,false)?;
    Some((place,turned_piece(source,turns)))
}

pub fn foliage_piece_at(pos:(i32,i32,i32))->Option<(usize,Piece)> {
    let(place,source,turns,_)=piece_source(pos,true)?;
    Some((place,turned_piece(source,turns)))
}

fn turned_piece(source:&Piece,turns:u8)->Piece {
    let mut piece = source.clone();
    piece.invalidate_collision();
    for t in &mut piece.tris {
        for _ in 0..turns {
            t.p = t.p.map(turn_point);
        }
    }
    piece
}

pub fn collision_piece_at(pos:(i32,i32,i32))->Option<(usize,Piece)> {
    let(place,source,turns,soft)=piece_source(pos,false)?;
    Some((place,contact_piece(source,turns,soft)))
}

fn contact_piece(source:&Piece,turns:u8,soft:bool)->Piece {
    if soft {Piece::default()} else {turned_piece(source,turns)}
}

pub(crate) fn collision_boxes(pos: (i32,i32,i32)) -> Option<std::sync::Arc<[[f32; 6]]>> {
    let (_, source, turns, soft) = piece_source(pos,false)?;
    Some(contact_boxes(source,turns,soft))
}

fn contact_boxes(source:&Piece,turns:u8,soft:bool)->std::sync::Arc<[[f32;6]]> {
    if soft {
        static EMPTY:OnceLock<std::sync::Arc<[[f32;6]]>>=OnceLock::new();
        return EMPTY.get_or_init(||Vec::new().into()).clone();
    }
    source.rotated_collision_boxes(turns)
}

/// Where an object of kind `kind` put down at `at` facing `turns` stands:
/// its box's middle over `at`, its foot level with it.
///
/// A building piece goes on the grid of 3 blocks (`GRID`) so the pieces
/// meet (Adrii, 23/10: "piezas tipo Rust… que encajan entre sí"): a floor
/// or roof fills a square of it, a wall or door frame stands on one of its
/// square's sides, the side the player faces from inside.
pub fn placed(kind: usize, at: (i32, i32, i32), turns: u8) -> Option<Instance> {
    let shape = shape(kind)?;
    let turns = turns % 4;
    let (w, d) = if turns % 2 == 0 { (shape.size.0, shape.size.2) } else { (shape.size.2, shape.size.0) };
    if !OBJECTS[kind].is_piece() {
        return Some(Instance { kind: kind as u16, origin: (at.0 - w / 2, at.1, at.2 - d / 2), turns });
    }
    let square = (at.0.div_euclid(GRID) * GRID, at.2.div_euclid(GRID) * GRID);
    // A wall's strip at the square's far side from where it was faced
    // (turned 0 it runs along x at the square's low z, and so on round).
    let origin = match (d < GRID, w < GRID, turns) {
        (true, _, 0) => (square.0, at.1, square.1),
        (true, _, _) => (square.0, at.1, square.1 + GRID - d),
        (_, true, 1) => (square.0 + GRID - w, at.1, square.1),
        (_, true, _) => (square.0, at.1, square.1),
        _ => (square.0, at.1, square.1),
    };
    Some(Instance { kind: kind as u16, origin, turns })
}

/// The building pieces' grid (blocks).
pub const GRID: i32 = 3;

/// The slab a block of an object is to the game (its collision for
/// creatures, vehicles and the player's box): the half its triangles keep
/// to, or whole where they reach into both.
pub fn slab_type(piece: &Piece) -> &'static str {
    let (mut low, mut high) = (false, false);
    for t in &piece.tris {
        for p in t.p {
            low |= p[1] < 0.45;
            high |= p[1] > 0.55;
        }
    }
    match (low, high) {
        (true, true) => "double",
        (false, true) => "top",
        _ => "bottom",
    }
}

/// Whether a piece block of an object keeps to the bottom half of its
/// block (a bottom slab; else a top one).
pub fn is_low(piece: &Piece) -> bool {
    piece.tris.iter().all(|t| t.p.iter().all(|p| p[1] <= 0.5))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn saved_object_fixture(origin:(i32,i32,i32),turns:u8)->Instance {
        let kind=kind_of("objeto_haven_propaganda").unwrap();
        let object=placed(kind,origin,turns).expect("original Jak II object fixture");
        assert!(!object.cells().is_empty());object
    }

    #[test]
    fn whole_object_dimensions_keep_independent_owners_and_removal_logs() {
        let dir=crate::test_dir::tempdir().unwrap();
        load(Some(dir.path()));
        let one=saved_object_fixture((-31,72,47),0);
        let two=Instance{turns:1,..one};
        assert!(add(one));
        let block=one.cells()[0].0;
        activate_dimension("minecraft:the_nether");
        assert!(at(block).is_none());
        assert!(generated_at(block).is_none());
        assert!(generated_foliage_at(block).is_none());
        assert!(add(two));
        let nether_block=two.cells()[0].0;
        assert_eq!(at(nether_block),Some(two));
        activate_dimension("minecraft:overworld");
        assert_eq!(at(block),Some(one));
        assert_eq!(remove_at(block).unwrap().0,one);
        activate_dimension("minecraft:the_nether");
        assert_eq!(at(nether_block),Some(two),"removing in Overworld cannot remove the Nether object");
        activate_dimension("minecraft:the_end");
        assert!(at(block).is_none());
        load(None);load(Some(dir.path()));
        assert!(at(block).is_none(),"Overworld removal survives disk reload");
        activate_dimension("minecraft:the_nether");
        assert_eq!(at(nether_block),Some(two),"Nether placement survives disk reload");
        assert_eq!(remove_at(nether_block).unwrap().0,two);
        load(None);load(Some(dir.path()));activate_dimension("minecraft:the_nether");
        assert!(at(nether_block).is_none());load(None);
    }

    #[test]
    fn whole_object_legacy_log_is_an_unchanged_overworld_base_with_new_deltas() {
        let dir=crate::test_dir::tempdir().unwrap();
        let one=saved_object_fixture((215,73,-224),2);
        let record=[1,i32::from(one.kind),one.origin.0,one.origin.1,one.origin.2,i32::from(one.turns)];
        let legacy=dir.path().join("jak_objetos.jsonl");
        let bytes=format!("{}\n",serde_json::to_string(&record).unwrap());
        std::fs::write(&legacy,&bytes).unwrap();
        load(Some(dir.path()));let block=one.cells()[0].0;
        assert_eq!(at(block),Some(one));
        activate_dimension("minecraft:the_nether");assert!(at(block).is_none());
        activate_dimension("minecraft:overworld");assert_eq!(remove_at(block).unwrap().0,one);
        let replacement=saved_object_fixture((245,73,-224),3);
        assert!(add(replacement));
        load(None);load(Some(dir.path()));
        assert!(at(block).is_none(),"the new removal must override the legacy base");
        assert_eq!(at(replacement.cells()[0].0),Some(replacement));
        assert_eq!(std::fs::read_to_string(&legacy).unwrap(),bytes,"legacy file must not be rewritten");
        load(None);
    }

    #[test]
    fn whole_object_unsaved_dimensions_stay_separate_and_log_paths_stay_inside_world() {
        let base=std::path::Path::new("world-fixture");
        assert_eq!(dimension_log_path(base,"mod:one/two").unwrap(),base.join("dimensions/mod/one/two/jak_objetos.jsonl"));
        for invalid in ["../minecraft:overworld","minecraft:../outside","minecraft:/absolute","minecraft:one//two","C:\\outside","minecraft:"] {
            assert!(dimension_log_path(base,invalid).is_none(),"{invalid}");
        }
        load(None);let one=saved_object_fixture((500,70,800),0);let block=one.cells()[0].0;
        assert!(add(one));activate_dimension("jak2:skatea");assert!(at(block).is_none());
        assert!(generated_at(block).is_none());activate_dimension("minecraft:overworld");
        assert_eq!(at(block),Some(one));load(None);assert!(at(block).is_none());
    }

    #[test]
    fn whole_object_failed_log_does_not_publish_an_unpersisted_placement() {
        let dir=crate::test_dir::tempdir().unwrap();
        std::fs::write(dir.path().join("dimensions"),"a file cannot contain dimension logs").unwrap();
        load(Some(dir.path()));let one=saved_object_fixture((710,71,-810),0);let block=one.cells()[0].0;
        assert!(!add(one));assert!(at(block).is_none());load(None);
    }

    #[test]
    fn hanging_foliage_preserves_legacy_sources_and_attaches_its_real_root() {
        for n in 1..=3 {
            let old=kind_of(&format!("objeto_haven_planta_{n}")).unwrap();
            let new=kind_of(&format!("objeto_haven_colgante_{n}")).unwrap();
            assert!(new>old);assert_eq!(placement_kind(old),new);
            let legacy=shape(old).expect("original slum overhang");
            let foliage=shape(new).expect("versioned slum overhang");
            assert!(legacy.wall_root.is_none());
            let root=foliage.wall_root.unwrap();
            assert!(foliage.cells.iter().any(|(cell,p)|p.tris.iter().any(|t|t.p.iter().any(|v|
                (v[0]+cell.0 as f32-root[0]).abs()<1e-5
                && (v[1]+cell.1 as f32-root[1]).abs()<1e-5
                && (v[2]+cell.2 as f32-root[2]).abs()<1e-5))),"the visible mesh lost its attachment vertex");
            let original_area:f32=legacy.cells.values().flat_map(|p|&p.tris).map(tri_area).sum();
            let new_area:f32=foliage.cells.values().flat_map(|p|&p.tris).map(tri_area).sum();
            assert!((new_area-original_area).abs()<original_area*0.002,"native leaves were clipped: {new_area}/{original_area}");
            let lowest=legacy.cells.iter().flat_map(|(cell,p)|p.tris.iter().flat_map(move|t|t.p.iter().map(move|v|cell.1 as f32+v[1])))
                .fold(f32::INFINITY,f32::min);
            assert!((lowest-[0.1873207,0.9463644,0.8807287][n-1]).abs()<0.001,"legacy source moved");
            for turns in 0..4 {
                let wall=(123,80,-89);
                let instance=hanging_at(new,wall,turns).unwrap();
                let root=instance.wall_root().unwrap();
                assert!((80.0..81.0).contains(&root[1]));
                let face=match turns {0=>root[2]+88.0,1=>root[0]-123.0,2=>root[2]+89.0,_=>root[0]-124.0};
                assert!(face.abs()<1e-6,"root is not on the wall face: {root:?}");
                assert!(supported_wall(&instance,|p|if p==wall{vec![[0.0,0.0,0.0,1.0,1.0,1.0]]}else{vec![]}));
                assert!(!supported_wall(&instance,|_|vec![]));
                assert!(!supported_wall(&instance,|_|vec![[0.1,0.0,0.1,0.9,1.0,0.9]]),"an inset post is not a wall face");
                for shape in [legacy,foliage] {for cell in shape.cells.values() {
                    assert!(!cell.tris.is_empty());
                    assert!(contact_piece(cell,turns,OBJECTS[new].soft_vegetation()).tris.is_empty());
                    assert!(contact_boxes(cell,turns,OBJECTS[old].soft_vegetation()).is_empty());
                }}
            }
        }
    }

    #[test]
    fn haven_street_actors_keep_their_whole_mesh_and_rest_on_the_ground() {
        for block in ["objeto_haven_puesto", "objeto_haven_propaganda"] {
            let triangles = original_places::read_street_actor(block).expect("original Jak II actor");
            let k = kind_of(block).unwrap();
            let shape = shape(k).unwrap();
            let original_area: f32 = triangles.iter().map(|t| {
                (t.p[1]-t.p[0]).cross(t.p[2]-t.p[0]).length()*0.5
            }).sum();
            let cut_area: f32 = shape.cells.values().flat_map(|p| &p.tris).map(tri_area).sum();
            assert!((cut_area-original_area).abs()<original_area*0.002,
                "{block}: clipping lost triangles ({cut_area}/{original_area})");
            let lowest = shape.cells.iter().flat_map(|(cell,piece)| piece.tris.iter()
                .flat_map(move |t|t.p.iter().map(move |p|cell.1 as f32+p[1])))
                .fold(f32::INFINITY,f32::min);
            assert!(lowest.abs()<0.001,"{block}: floating {lowest}");
            assert!(shape.size.0<=8 && shape.size.1<=3 && shape.size.2<=3,
                "unused export vertices became part of {block}: {:?}",shape.size);
            for turns in 0..4 {
                let instance = placed(k,(12,70,-3),turns).unwrap();
                assert_eq!(instance.cells().len(),shape.cells.len());
            }
        }
    }

    /// Every object kind is cut from its level whole: some metres, within
    /// its box; turned, its blocks keep to its turned box; the table puts
    /// one in, finds each of its blocks, and takes it away whole, its log
    /// read back the same.
    #[test]
    fn objects_are_whole_and_go_whole() {
        let mut made = 0;
        let mut empty = Vec::new();
        for (k, o) in OBJECTS.iter().enumerate() {
            let place = PLACES.iter().position(|p| p.name == o.place).unwrap();
            if crate::jak_pieces::source(place).is_none() {
                continue;
            }
            let Some(shape) = shape(k) else {
                empty.push(o.block);
                continue;
            };
            println!("{}: {} blocks, {:?}", o.block, shape.cells.len(), shape.size);
            assert!(!shape.cells.is_empty());
            for &(x, y, z) in shape.cells.keys() {
                assert!((0..shape.size.0).contains(&x) && (0..shape.size.1).contains(&y) && (0..shape.size.2).contains(&z), "{}", o.block);
            }
            made += 1;
        }
        println!("{made} objects cut; none of {empty:?}");
        assert!(empty.is_empty(), "{empty:?}");
        if made == 0 {
            return;
        }
        let dir = std::env::temp_dir().join(format!("jak-objetos-test-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        load(Some(&dir));
        let k = (0..OBJECTS.len()).find(|&k| shape(k).is_some_and(|s| s.cells.len() > 3)).unwrap();
        let one = placed(k, (100, 70, 100), 1).unwrap();
        assert!(add(one));
        let blocks = one.cells();
        assert!(blocks.iter().all(|(at, _)| self::at(*at) == Some(one)));
        assert!(!add(one), "not twice over the same blocks");
        let (at0, _) = blocks[0];
        assert!(piece_at(at0).is_some());
        load(Some(&dir));
        assert_eq!(self::at(at0), Some(one), "read back from its log");
        let (gone, all) = remove_at(blocks[blocks.len() - 1].0).unwrap();
        assert_eq!(gone, one);
        assert_eq!(all.len(), blocks.len());
        assert!(self::at(at0).is_none());
        load(Some(&dir));
        assert!(self::at(at0).is_none(), "its removal read back");
        load(None);
        let _ = std::fs::remove_dir_all(&dir);
    }
}

#[cfg(test)]mod projector_revision_tests {
    #[test]fn revision_projector_is_one_cell_and_both_old_shapes_upgrade(){
        let kind=super::kind_of("objeto_haven_propaganda_bloque").unwrap();
        let shape=super::shape(kind).expect("native projector");assert_eq!(shape.size,(1,1,1));assert_eq!(shape.cells.len(),1);
        for old in ["objeto_haven_propaganda","objeto_haven_propaganda_ajustada"]{
            let old=super::kind_of(old).unwrap();assert_eq!(super::placement_kind(old),kind);
            let instance=super::placed(old,(120,64,180),0).unwrap();assert_eq!(super::compact_replacement(instance).unwrap().kind,kind as u16);
        }
    }
}

#[cfg(test)]mod systems31_prop_tests{
 #[test]fn props32_fire_can_is_an_upright_closed_shell_with_a_real_inside(){
  use super::*;
  let tris=burning_can(0);let mut edges=std::collections::HashMap::new();
  for tri in &tris{for i in 0..3{
   let mut edge=[tri.p[i].to_array().map(f32::to_bits),tri.p[(i+1)%3].to_array().map(f32::to_bits)];edge.sort_unstable();
   *edges.entry(edge).or_insert(0)+=1;
  }}
  assert!(edges.values().all(|n|*n==2),"every shell edge joins two faces, including the base, inside and rim");
  let hi=tris.iter().flat_map(|t|t.p).reduce(Vec3::max).unwrap();assert!((hi.y-0.88).abs()<0.0001);
  let bottom=tris.iter().flat_map(|t|t.p).filter(|v|v.y<0.01).map(|v|(v-Vec3::new(0.5,v.y,0.5)).length()).fold(0.,f32::max);
  let mouth=tris.iter().flat_map(|t|t.p).filter(|v|v.y>0.87).map(|v|(v-Vec3::new(0.5,v.y,0.5)).length()).fold(0.,f32::max);
  assert!(mouth>bottom*1.2,"upright taper, never the inverted waist");
 }
 #[test]fn systems31_burning_can_fits_one_cell_and_rests_on_its_base(){
  use super::*;let kind=kind_of("objeto_haven_bidon_fuego").unwrap();let shape=shape(kind).unwrap();assert_eq!(shape.size,(1,1,1));
  let lo=shape.cells.values().flat_map(|p|&p.tris).flat_map(|t|t.p).map(glam::Vec3::from_array).reduce(glam::Vec3::min).unwrap();
  assert!(lo.y.abs()<0.0001,"floating base: {lo:?}");
 }
}
