//! Hamlets in the Jak lands (Adrii, 23/10: "usa estos suelos al generar
//! los biomas de Jak (caminos y claros que se forman solos) y pon copias de
//! edificios del nivel"): now and then a clearing of the land's own ground
//! (`jak_ground`) with a building of its level copied into it as pieces
//! (`jak_pieces`: the level's own triangles, metre by metre), and paths of
//! its path ground to the hamlets round it, laid on the land, their edges
//! joined by the ground blocks themselves.
//!
//! - Boggy Swamp's lands: Boggy Swamp's huts, on its leafy ground, mud
//!   paths;
//! - Snowy Mountain's lands: its wooden huts and posts, on its snow, its
//!   trodden path;
//! - the Wasteland: Spargus's houses, on its ground, its ditches for paths.
//!
//! A level's buildings are picked by eye off its map seen from above
//! (`picked`: Boggy Swamp's huts, Snowy Mountain's fort, Spargus's houses
//! round its plaza, Sandover's huts),
//! each from its ground to its top; elsewhere found by their walls' and
//! roofs' textures (the metres they cover, two by two, joined where they
//! touch, a building when 5 to 34 metres across and 3 to 30 high). Which building a hamlet copies, and where, is the
//! world's seed's; where its pieces stand the drawing finds from the same
//! plan (`source_at`), so the world keeps nothing for them.
use std::collections::HashMap;
use std::sync::{Arc, Mutex, OnceLock};

use minecraftoss_core::block::BlockStateId;
use minecraftoss_core::chunk::{Chunk, HeightmapKind};
use minecraftoss_world::chunk_map::WorldGen;

use crate::jak_lands::Land;
use crate::original_places::{self, PLACES};

/// One land's hamlets.
pub struct Kind {
    pub name: &'static str,
    pub land: &'static Land,
    /// The place its buildings are copied from.
    pub place: &'static str,
    /// Its clearing's ground and its paths' (`jak_ground::GROUNDS` blocks).
    pub ground: &'static str,
    pub path: &'static str,
    /// One perhaps a square this many blocks wide.
    pub cell: i32,
    /// How many of a hundred squares have one.
    pub chance: u64,
    pub salt: u64,
    /// Its house built of blocks (`house_blocks`: its ground, its walls
    /// that join, its door, its roof tiles), else one of its level's own
    /// buildings copied whole.
    pub house: Option<HouseKind>,
}

/// The blocks a house is built of (`house_blocks`).
#[derive(Clone, Copy, Debug)]
pub struct HouseKind {
    /// Its floor (`jak:<block>`).
    pub ground: &'static str,
    /// Its walls (`jak_walls`).
    pub wall: &'static str,
    /// Its windows (a wall of its windows), if its level has any.
    pub window: Option<&'static str>,
    /// Its door (a door block: lower and upper halves).
    pub door: &'static str,
    /// Its roof's tiles (a ground block: its roof texture).
    pub roof: &'static str,
    /// What stands in the middle of its village (stacked three high).
    pub landmark: &'static str,
    /// Its bed.
    pub bed: &'static str,
}

pub const SPARGUS_HOUSE: HouseKind = HouseKind {
    ground: "jak:suelo_spargus_tierra",
    wall: "jak:muro_spargus_estuco",
    window: Some("jak:muro_spargus_ventana"),
    door: "jak:spargus_door",
    roof: "jak:tejado_spargus",
    landmark: "jak:totem_stone",
    bed: "minecraft:red_bed",
};

pub const SNOW_HOUSE: HouseKind = HouseKind {
    ground: "jak:suelo_montana_nieve",
    wall: "jak:muro_montana_madera",
    window: None,
    door: "jak:snow_gate_door",
    roof: "jak:tejado_montana",
    landmark: "jak:snow_pole[axis=y]",
    bed: "minecraft:white_bed",
};

/// A village's houses round its middle: where each stands (its floor's
/// low corner), its footprint and which way its door looks (toward the
/// middle): three to five, 11 to 13 blocks out, none over another.
pub fn village_houses((sx, y, sz): (i32, i32, i32), seed: i64, salt: u64, copied: bool) -> Vec<((i32, i32, i32), i32, u8)> {
    let n = 3 + (mix(seed, sx, sz, salt) % 3) as i32;
    let offset = (mix(seed, sx, sz, salt + 1) % 100) as f32 / 100.0;
    let mut out: Vec<((i32, i32, i32), i32, u8)> = Vec::new();
    for k in 0..n {
        // (With the level's building to the south, the houses keep to the
        // northern half.)
        let span = if copied { std::f32::consts::PI * 1.1 } else { std::f32::consts::TAU };
        let start = if copied { std::f32::consts::PI * 0.95 } else { 0.0 };
        let angle = start + (k as f32 + offset) / n as f32 * span + ((mix(seed, sx + k, sz, salt + 2) % 40) as f32 - 20.0) / 100.0;
        let w = if mix(seed, sx, sz + k, salt + 3) % 2 == 0 { 7 } else { 9 };
        let r = VILLAGE_R - 4 - w / 2 + (mix(seed, sx - k, sz, salt + 4) % 3) as i32;
        let (hx, hz) = (sx + (angle.cos() * r as f32).round() as i32, sz + (angle.sin() * r as f32).round() as i32);
        let (dx, dz) = (sx - hx, sz - hz);
        let facing = if dx.abs() > dz.abs() { if dx < 0 { 3 } else { 1 } } else if dz < 0 { 0 } else { 2 };
        let origin = (hx - w / 2, y, hz - w / 2);
        // Apart from the others (their roofs a block past their walls).
        let apart = out.iter().all(|(o, ow, _)| origin.0 > o.0 + ow + 2 || origin.0 + w + 2 < o.0 || origin.2 > o.2 + ow + 2 || origin.2 + w + 2 < o.2);
        if apart {
            out.push((origin, w, facing));
        }
    }
    out
}

/// A village's round: its houses stand inside, its clearing a little past.
pub const VILLAGE_R: i32 = 17;

/// A house of blocks (Adrii, 23/10: "edificios compuestos por bloques
/// reales de la rejilla… romper un bloque de pared debe retirar esa parte
/// y actualizar sus vecinos"): its floor of its ground, its walls a ring
/// of its wall blocks `h` high (panes to the world, each joined to the
/// walls beside it), its door in the middle of its front (`facing`: 0
/// north, 1 east, 2 south, 3 west), a window in the middle of each other
/// side, its roof of its roof tiles a block past its walls, and air inside.
/// `origin` is its floor's low corner, `w` its footprint (odd, the door in
/// the middle). Each block stands alone: broken, the game joins the rest
/// afresh.
pub fn house_blocks(kind: &HouseKind, (x0, y, z0): (i32, i32, i32), w: i32, h: i32, facing: u8) -> Vec<((i32, i32, i32), &'static str)> {
    let mut out: Vec<((i32, i32, i32), &'static str)> = Vec::new();
    let ring = |x: i32, z: i32| x == 0 || z == 0 || x == w - 1 || z == w - 1;
    let m = w / 2;
    // The door's cell on the ring, and the windows'.
    let door = match facing % 4 {
        0 => (m, 0),
        1 => (w - 1, m),
        2 => (m, w - 1),
        _ => (0, m),
    };
    let windows = [(m, 0), (w - 1, m), (m, w - 1), (0, m)];
    let out_facing = ["north", "east", "south", "west"][usize::from(facing % 4)];
    let is_wall = |x: i32, z: i32| (0..w).contains(&x) && (0..w).contains(&z) && ring(x, z) && (x, z) != door;
    for x in 0..w {
        for z in 0..w {
            out.push(((x0 + x, y, z0 + z), kind.ground));
            for k in 1..=h {
                let at = (x0 + x, y + k, z0 + z);
                if !ring(x, z) {
                    out.push((at, "minecraft:air"));
                    continue;
                }
                if (x, z) == door {
                    match k {
                        1 => out.push((at, kept(format!("{}[facing={out_facing},half=lower,hinge=left,open=false,powered=false]", kind.door)))),
                        2 => out.push((at, kept(format!("{}[facing={out_facing},half=upper,hinge=left,open=false,powered=false]", kind.door)))),
                        _ => {
                            let joins = [is_wall(x, z - 1), is_wall(x + 1, z), is_wall(x, z + 1), is_wall(x - 1, z)];
                            out.push((at, pane(kind.wall, joins)));
                        }
                    }
                    continue;
                }
                let joins = [is_wall(x, z - 1), is_wall(x + 1, z), is_wall(x, z + 1), is_wall(x - 1, z)];
                let block = match kind.window {
                    Some(window) if k == 2 && windows.contains(&(x, z)) => window,
                    _ => kind.wall,
                };
                out.push((at, pane(block, joins)));
            }
        }
    }
    // The roof, a block past the walls, and air over it.
    for x in -1..=w {
        for z in -1..=w {
            out.push(((x0 + x, y + h + 1, z0 + z), kind.roof));
            out.push(((x0 + x, y + h + 2, z0 + z), "minecraft:air"));
        }
    }
    // Inside: a bed against the back wall (its head to the wall), a
    // crafting table and a barrel in the other back corner, a lantern
    // hung from the roof's middle.
    let back = ["north", "east", "south", "west"][usize::from((facing + 2) % 4)];
    let toward_back: (i32, i32) = [(0, -1), (1, 0), (0, 1), (-1, 0)][usize::from((facing + 2) % 4)];
    let inner = |a: i32, b: i32| -> (i32, i32) {
        // `a` along the back wall from its left end, `b` in from it.
        match (facing + 2) % 4 {
            0 => (1 + a, 1 + b),
            1 => (w - 2 - b, 1 + a),
            2 => (w - 2 - a, w - 2 - b),
            _ => (1 + b, w - 2 - a),
        }
    };
    let (hx, hz) = inner(1, 0);
    out.push(((x0 + hx, y + 1, z0 + hz), kept(format!("{}[facing={back},occupied=false,part=head]", kind.bed))));
    out.push(((x0 + hx - toward_back.0, y + 1, z0 + hz - toward_back.1), kept(format!("{}[facing={back},occupied=false,part=foot]", kind.bed))));
    let (tx, tz) = inner(w - 3, 0);
    out.push(((x0 + tx, y + 1, z0 + tz), "minecraft:crafting_table"));
    let (bx, bz) = inner(w - 4, 0);
    out.push(((x0 + bx, y + 1, z0 + bz), "minecraft:barrel[facing=up,open=false]"));
    out.push(((x0 + w / 2, y + h, z0 + w / 2), "minecraft:lantern[hanging=true,waterlogged=false]"));
    out
}

/// A wall block's state with its joins (north, east, south, west).
fn pane(block: &str, [n, e, s, w]: [bool; 4]) -> &'static str {
    kept(format!("{block}[east={e},north={n},south={s},waterlogged=false,west={w}]"))
}

/// A house's footprint (blocks, odd) and its walls' height.
pub const HOUSE_W: i32 = 7;
pub const HOUSE_H: i32 = 3;

pub static KINDS: [Kind; 3] = [
    Kind {
        name: "aldea del pantano",
        land: &crate::swamp::LAND,
        place: "pantano",
        ground: "suelo_pantano_hojas",
        path: "suelo_pantano_barro",
        cell: 160,
        chance: 55,
        salt: 701,
        // (Boggy Swamp's huts, on stilts over the water, copied whole.)
        house: None,
    },
    Kind {
        name: "aldea de la Montaña Nevada",
        land: &crate::snowy::LAND,
        place: "montana",
        ground: "suelo_montana_nieve",
        path: "suelo_montana_camino",
        cell: 176,
        chance: 55,
        salt: 711,
        house: Some(SNOW_HOUSE),
    },
    Kind {
        name: "aldea del Páramo",
        land: &crate::wasteland::LAND,
        place: "spargus",
        ground: "suelo_spargus_tierra",
        path: "suelo_spargus_zanja",
        cell: 192,
        chance: 45,
        salt: 721,
        house: Some(SPARGUS_HOUSE),
    },
];

/// A building of a level: its box in the level's metres (inclusive), its
/// ground's metre the lowest.
#[derive(Clone, Copy, Debug)]
pub struct Building {
    pub lo: (i32, i32, i32),
    pub hi: (i32, i32, i32),
}

/// The textures a place's buildings are made of, by part of their names.
fn building_textures(place: &str) -> &'static [&'static str] {
    match place {
        "spargus" => &["stucco", "wascity-roof", "airlock", "wascity-wall", "wascitya-wall", "door"],
        "haven" => &["slumwall", "slum-roof", "slumbase", "dirtywood"],
        "villarrefugio" => &["vil-hut", "huttoprim", "sages", "vil-under-roof", "roof"],
        "montana" => &["fort", "snow-wood", "metalroof", "gate", "snow-door"],
        _ => &["hut", "roof", "swp-wood", "plankwood"],
    }
}

/// A place's buildings, found once (none without its levels).
pub fn buildings(place: usize) -> &'static [Building] {
    static FOUND: [OnceLock<Vec<Building>>; 5] = [const { OnceLock::new() }; 5];
    FOUND[place].get_or_init(|| find_buildings(place)).as_slice()
}

/// Buildings picked by eye off each level's map seen from above (23/10:
/// the levels' textures alone find posts and fences as often as houses):
/// their footprints, metres of the level (x from, x to, z from, z to).
fn picked(place: &str) -> &'static [[i32; 4]] {
    match place {
        // Boggy Swamp's huts and their platforms on stilts.
        "pantano" => &[
            [555, 581, -2111, -2083],
            [585, 603, -2101, -2083],
            [585, 613, -2071, -2053],
            [625, 641, -2071, -2053],
            [587, 607, -2037, -2017],
            [495, 525, -1957, -1929],
            [419, 439, -1945, -1923],
            [423, 441, -2073, -2053],
            [535, 559, -1913, -1891],
            [587, 623, -1915, -1887],
            [509, 533, -2119, -2099],
            [563, 581, -2157, -2139],
        ],
        // Snowy Mountain's lurker fort: its keep and its two towers.
        "montana" => &[[805, 870, -3220, -3188], [790, 820, -3292, -3265], [860, 888, -3292, -3265]],
        // Spargus's free-standing houses round its plaza: the dome, the
        // long houses, the triangular tents.
        "spargus" => &[
            [2272, 2302, -230, -195],
            [2270, 2305, -172, -125],
            [2322, 2377, -220, -175],
            [2212, 2255, -202, -157],
            [2405, 2440, -160, -110],
            [2437, 2480, -160, -110],
        ],
        // Sandover's huts.
        "villarrefugio" => &[
            [-36, -4, -90, -58],
            [-76, -46, -44, -12],
            [-142, -112, -34, -2],
            [-133, -102, 27, 58],
            [-74, -43, 17, 48],
            [-84, -53, 56, 87],
        ],
        _ => &[],
    }
}

/// A picked footprint's box: from its ground (the lowest few hundredths of
/// its triangles) to its top, thirty metres at most.
fn picked_box(source: &crate::jak_pieces::Source, [x0, x1, z0, z1]: [i32; 4]) -> Option<Building> {
    let mut ys: Vec<f32> = source
        .tris
        .iter()
        .map(|t| (t.p[0] + t.p[1] + t.p[2]) / 3.0)
        .filter(|c| c.x >= x0 as f32 && c.x < x1 as f32 && c.z >= z0 as f32 && c.z < z1 as f32)
        .map(|c| c.y)
        .collect();
    if ys.len() < 20 {
        return None;
    }
    ys.sort_by(f32::total_cmp);
    let lo = ys[ys.len() * 3 / 100];
    let hi = ys[ys.len() - 1].min(lo + 30.0);
    Some(Building { lo: (x0, lo.floor() as i32 - 1, z0), hi: (x1 - 1, hi.ceil() as i32 + 1, z1 - 1) })
}

fn find_buildings(place: usize) -> Vec<Building> {
    let Some(source) = crate::jak_pieces::source(place) else { return Vec::new() };
    let picked = picked(PLACES[place].name);
    if !picked.is_empty() {
        return picked.iter().filter_map(|&footprint| picked_box(source, footprint)).collect();
    }
    let keys = building_textures(PLACES[place].name);
    let textures = original_places::textures_of(&PLACES[place]);
    let wanted: Vec<bool> = textures.iter().map(|t| keys.iter().any(|k| t.contains(k))).collect();
    // Two-metre squares their triangles' corners fall in, with the heights
    // there.
    let mut squares: HashMap<(i32, i32), (f32, f32)> = HashMap::new();
    for t in &source.tris {
        if !wanted.get(usize::from(t.texture)).copied().unwrap_or(false) {
            continue;
        }
        for p in t.p {
            let key = ((p.x / 2.0).floor() as i32, (p.z / 2.0).floor() as i32);
            let e = squares.entry(key).or_insert((p.y, p.y));
            e.0 = e.0.min(p.y);
            e.1 = e.1.max(p.y);
        }
    }
    // Joined where they touch (a square apart at most).
    let mut seen = std::collections::HashSet::new();
    let mut out = Vec::new();
    let mut keys: Vec<(i32, i32)> = squares.keys().copied().collect();
    keys.sort();
    for start in keys {
        if !seen.insert(start) {
            continue;
        }
        let mut stack = vec![start];
        let (mut lo, mut hi) = ((start.0, f32::MAX, start.1), (start.0, f32::MIN, start.1));
        while let Some((x, z)) = stack.pop() {
            let (ylo, yhi) = squares[&(x, z)];
            lo = (lo.0.min(x), lo.1.min(ylo), lo.2.min(z));
            hi = (hi.0.max(x), hi.1.max(yhi), hi.2.max(z));
            for dx in -2..=2 {
                for dz in -2..=2 {
                    let next = (x + dx, z + dz);
                    if squares.contains_key(&next) && seen.insert(next) {
                        stack.push(next);
                    }
                }
            }
        }
        // Across with a square round it (as wide as it is taken).
        let (w, d, h) = ((hi.0 - lo.0 + 3) * 2, (hi.2 - lo.2 + 3) * 2, hi.1 - lo.1);
        if (5..=34).contains(&w) && (5..=34).contains(&d) && (3.0..=30.0).contains(&h) {
            // A metre round it, and the metre of ground under it.
            out.push(Building {
                lo: (lo.0 * 2 - 1, lo.1.floor() as i32 - 1, lo.2 * 2 - 1),
                hi: (hi.0 * 2 + 2, hi.1.ceil() as i32 + 1, hi.2 * 2 + 2),
            });
        }
    }
    out
}

/// Objects the world makes together (a square's things, a house's
/// pieces), with each of their blocks found at once.
#[derive(Default)]
pub struct Made {
    pub instances: Vec<crate::jak_objects::Instance>,
    /// Each block of them: which instance, and the cell of its shape there.
    pub by_block: HashMap<(i32, i32, i32), (u32, (i32, i32, i32))>,
}

impl Made {
    pub(crate) fn new(instances: Vec<crate::jak_objects::Instance>) -> Self {
        let mut by_block = HashMap::new();
        for (i, instance) in instances.iter().enumerate() {
            for (at, cell) in instance.cells() {
                by_block.insert(at, (i as u32, cell));
            }
        }
        Self { instances, by_block }
    }

    /// The instance one of whose blocks is `pos`, and its cell there.
    pub(crate) fn at(&self, pos: (i32, i32, i32)) -> Option<(crate::jak_objects::Instance, (i32, i32, i32))> {
        let &(i, cell) = self.by_block.get(&pos)?;
        Some((self.instances[i as usize], cell))
    }
}

impl std::ops::Deref for Made {
    type Target = [crate::jak_objects::Instance];
    fn deref(&self) -> &Self::Target {
        &self.instances
    }
}

/// A hamlet the world makes: its building, where its box's low corner
/// stands in the world, and its spot.
#[derive(Clone, Copy, Debug)]
pub struct Hamlet {
    pub kind: usize,
    pub place: usize,
    pub building: Building,
    pub origin: (i32, i32, i32),
    pub spot: (i32, i32),
    /// Built of blocks (`Kind::house`): `building` is then only its size.
    pub house: bool,
    /// A village's own building of its level (Adrii, 23/10: "que las
    /// estructuras de los pueblos sean más complejas y fieles a Jak"): one
    /// of the level's buildings copied metre by metre as pieces, and where
    /// its box's low corner stands in the world.
    pub copied: Option<(Building, (i32, i32, i32))>,
}

impl Hamlet {
    /// The metre of its level a world block shows, if its box (or its
    /// copied building's) holds it.
    pub fn source(&self, (x, y, z): (i32, i32, i32)) -> Option<(i32, i32, i32)> {
        let (b, origin) = if self.house {
            let (b, origin) = self.copied?;
            (b, origin)
        } else {
            (self.building, self.origin)
        };
        let cell = (x - origin.0 + b.lo.0, y - origin.1 + b.lo.1, z - origin.2 + b.lo.2);
        let inside = (b.lo.0..=b.hi.0).contains(&cell.0) && (b.lo.1..=b.hi.1).contains(&cell.1) && (b.lo.2..=b.hi.2).contains(&cell.2);
        inside.then_some(cell)
    }

    fn size(&self) -> (i32, i32, i32) {
        let b = &self.building;
        (b.hi.0 - b.lo.0 + 1, b.hi.1 - b.lo.1 + 1, b.hi.2 - b.lo.2 + 1)
    }

    /// Its clearing's radius.
    fn radius(&self) -> i32 {
        let (w, _, d) = self.size();
        w.max(d) / 2 + 5
    }
}

fn mix(seed: i64, a: i32, b: i32, salt: u64) -> u64 {
    let mut h = (seed as u64) ^ ((a as u32 as u64) << 32 | u64::from(b as u32)) ^ salt.wrapping_mul(0x9E37_79B9_7F4A_7C15);
    h ^= h >> 33;
    h = h.wrapping_mul(0xff51_afd7_ed55_8ccd);
    h ^= h >> 33;
    h = h.wrapping_mul(0xc4ce_b9fe_1a85_ec53);
    h ^ (h >> 33)
}

/// A square's spot, if it has one.
fn spot(kind: &Kind, seed: i64, cell: (i32, i32)) -> Option<(i32, i32)> {
    if mix(seed, cell.0, cell.1, kind.salt) % 100 >= kind.chance {
        return None;
    }
    let margin = 40;
    let span = (kind.cell - 2 * margin) as u64;
    Some((
        cell.0 * kind.cell + margin + (mix(seed, cell.0, cell.1, kind.salt + 1) % span) as i32,
        cell.1 * kind.cell + margin + (mix(seed, cell.0, cell.1, kind.salt + 2) % span) as i32,
    ))
}

/// The ground and land a world's generator gives: the ground's top (the
/// first block over it), the water's, and whether a column is the land's.
struct Ground<'a> {
    terrain: &'a minecraftoss_generator::terrain::TerrainGenerator,
    ids: Vec<minecraftoss_core::BiomeId>,
}

impl Ground<'_> {
    fn ground(&self, x: i32, z: i32) -> i32 {
        self.terrain.base_height(x, z, HeightmapKind::OceanFloorWg)
    }

    fn surface(&self, x: i32, z: i32) -> i32 {
        self.terrain.base_height(x, z, HeightmapKind::WorldSurfaceWg)
    }

    fn inside(&self, x: i32, z: i32) -> bool {
        self.ids.contains(&self.terrain.biome_at_quart(x >> 2, self.ground(x, z) >> 2, z >> 2))
    }
}

/// The hamlet of kind `k` in a square, if the world makes one there: on
/// its land, clear of the games' places, dry, and its clearing's ground
/// within four blocks of level.
fn hamlet_in(k: usize, seed: i64, cell: (i32, i32), terrain: &minecraftoss_generator::terrain::TerrainGenerator) -> Option<Hamlet> {
    type Kept = HashMap<(i64, usize, (i32, i32)), Option<Hamlet>>;
    static KEPT: Mutex<Option<Kept>> = Mutex::new(None);
    if let Some(found) = KEPT.lock().ok().and_then(|kept| kept.as_ref().and_then(|kept| kept.get(&(seed, k, cell)).copied())) {
        return found;
    }
    let made = make_hamlet(k, seed, cell, terrain);
    if let Ok(mut kept) = KEPT.lock() {
        let map = kept.get_or_insert_with(HashMap::new);
        if map.len() > 4096 {
            map.clear();
        }
        map.insert((seed, k, cell), made);
    }
    made
}

fn make_hamlet(k: usize, seed: i64, cell: (i32, i32), terrain: &minecraftoss_generator::terrain::TerrainGenerator) -> Option<Hamlet> {
    let kind = &KINDS[k];
    let (sx, sz) = spot(kind, seed, cell)?;
    // Not where one of the games' places stands (it takes the land there).
    if [(-40, -40), (40, -40), (-40, 40), (40, 40), (0, 0)]
        .iter()
        .any(|&(dx, dz)| original_places::inside_a_place(&terrain.registries, seed, sx + dx, sz + dz))
    {
        return None;
    }
    // Not on a flat (its ground is levelled after the hamlet's is read).
    if [(0, 0), (12, 0), (-12, 0), (0, 12), (0, -12)].iter().any(|&(dx, dz)| crate::jak_lands::flat_at(kind.land, terrain, sx + dx, sz + dz).is_some()) {
        return None;
    }
    let biomes = &terrain.registries.biomes;
    let land = Ground { terrain, ids: kind.land.biomes.iter().filter_map(|b| biomes.id(b)).collect() };
    let place = PLACES.iter().position(|p| p.name == kind.place)?;
    if kind.house.is_some() {
        return house_site(k, place, (sx, sz), &land);
    }
    // Houses, not posts: 48 square metres at least.
    let all: Vec<Building> = buildings(place)
        .iter()
        .copied()
        .filter(|b| (b.hi.0 - b.lo.0 + 1) * (b.hi.2 - b.lo.2 + 1) >= 48)
        .collect();
    if all.is_empty() {
        return None;
    }
    let building = all[(mix(seed, cell.0, cell.1, kind.salt + 3) % all.len() as u64) as usize];
    let (w, _, d) = (building.hi.0 - building.lo.0 + 1, 0, building.hi.2 - building.lo.2 + 1);
    let r = w.max(d) / 2 + 5;
    if !(-r..=r).all(|dx|(-r..=r).all(|dz|land.inside(sx+dx,sz+dz))) {return None;}
    let (mut lo, mut hi) = (i32::MAX, i32::MIN);
    for (dx, dz) in [(0, 0), (r, 0), (-r, 0), (0, r), (0, -r), (r, r), (-r, -r), (r, -r), (-r, r)] {
        let (px, pz) = (sx + dx, sz + dz);
        if !land.inside(px, pz) || land.surface(px, pz) > land.ground(px, pz) {
            return None;
        }
        let y = land.ground(px, pz);
        lo = lo.min(y);
        hi = hi.max(y);
    }
    if hi - lo > 4 {
        return None;
    }
    let g = land.ground(sx, sz);
    Some(Hamlet {
        kind: k,
        place,
        building,
        // The box's low corner a metre under the ground's top.
        origin: (sx - w / 2, g - 1, sz - d / 2),
        spot: (sx, sz),
        house: false,
        copied: None,
    })
}

/// A house of blocks' site: on its land, dry, level within four blocks.
fn house_site(k: usize, place: usize, (sx, sz): (i32, i32), land: &Ground) -> Option<Hamlet> {
    let r = VILLAGE_R + 2;
    if !(-r..=r).all(|dx|(-r..=r).all(|dz|land.inside(sx+dx,sz+dz))) {return None;}
    let (mut lo, mut hi) = (i32::MAX, i32::MIN);
    for (dx, dz) in [(0, 0), (r, 0), (-r, 0), (0, r), (0, -r), (r, r), (-r, -r), (r, -r), (-r, r)] {
        let (px, pz) = (sx + dx, sz + dz);
        if !land.inside(px, pz) || land.surface(px, pz) > land.ground(px, pz) {
            return None;
        }
        let y = land.ground(px, pz);
        lo = lo.min(y);
        hi = hi.max(y);
    }
    if hi - lo > 6 {
        return None;
    }
    let g = land.ground(sx, sz);
    // One of its level's buildings (a house of a few hundred square
    // metres: Spargus's domes and halls, Snowy Mountain's fort towers),
    // its box south of the village's middle, four blocks off it.
    let all: Vec<Building> = buildings(place)
        .iter()
        .copied()
        .filter(|b| {
            let area = (b.hi.0 - b.lo.0 + 1) * (b.hi.2 - b.lo.2 + 1);
            (48..=1600).contains(&area)
        })
        .collect();
    let copied = (!all.is_empty()).then(|| {
        let b = all[(mix(seed_of(land), sx, sz, KINDS[k].salt + 50) % all.len() as u64) as usize];
        let w = b.hi.0 - b.lo.0 + 1;
        (b, (sx - w / 2, g - 1, sz + 4))
    });
    let reach = copied.map_or(VILLAGE_R, |(b, _)| VILLAGE_R.max(4 + (b.hi.2 - b.lo.2 + 1) + 3).max((b.hi.0 - b.lo.0 + 1) / 2 + 4));
    let corner = (sx - reach, sz - reach);
    Some(Hamlet {
        kind: k,
        place,
        building: Building { lo: (0, 0, 0), hi: (2 * reach - 1, HOUSE_H + 2, 2 * reach - 1) },
        origin: (corner.0, g - 1, corner.1),
        spot: (sx, sz),
        house: true,
        copied,
    })
}

/// The world's seed, as the land's generator has it.
fn seed_of(land: &Ground) -> i64 {
    land.terrain.seed
}

/// A village's things (made once): the land's objects beside its houses
/// (a barrel, a plant or a rock by a wall; a pine by a snowy hut).
pub fn village_objects(h: &Hamlet, seed: i64) -> Arc<Made> {
    type Kept = HashMap<(i64, usize, (i32, i32)), Arc<Made>>;
    static KEPT: Mutex<Option<Kept>> = Mutex::new(None);
    let key = (seed, h.kind, h.spot);
    if let Some(found) = KEPT.lock().ok().and_then(|k| k.as_ref().and_then(|k| k.get(&key).cloned())) {
        return found;
    }
    let made = Arc::new(Made::new(if h.house { make_village_objects(h, seed) } else { Vec::new() }));
    if let Ok(mut kept) = KEPT.lock() {
        let map = kept.get_or_insert_with(HashMap::new);
        if map.len() > 4096 {
            map.clear();
        }
        map.insert(key, made.clone());
    }
    made
}

fn make_village_objects(h: &Hamlet, seed: i64) -> Vec<crate::jak_objects::Instance> {
    use crate::jak_objects::{placed, shape, OBJECTS};
    let kind = &KINDS[h.kind];
    let groups = detail_groups(kind.place);
    let kinds: Vec<usize> = OBJECTS
        .iter()
        .enumerate()
        .filter(|(i, o)| o.place == kind.place && groups.contains(&o.group) && o.group != "palmera" && !o.versioned_foliage() && shape(*i).is_some())
        .map(|(i, _)| i)
        .collect();
    if kinds.is_empty() {
        return Vec::new();
    }
    let (sx, sz) = h.spot;
    let top = h.origin.1 + 1;
    let mut out = Vec::new();
    for (n, (origin, w, facing)) in village_houses((sx, h.origin.1, sz), seed, kind.salt + 20, h.copied.is_some()).into_iter().enumerate() {
        // Beside a side wall (not the door's), two blocks out.
        let side = (facing + 1 + (mix(seed, sx + n as i32, sz, kind.salt + 30) % 2) as u8 * 2) % 4;
        let m = w / 2;
        let (x, z) = match side {
            0 => (origin.0 + m, origin.2 - 3),
            1 => (origin.0 + w + 2, origin.2 + m),
            2 => (origin.0 + m, origin.2 + w + 2),
            _ => (origin.0 - 3, origin.2 + m),
        };
        let which = kinds[(mix(seed, sx, sz + n as i32, kind.salt + 31) % kinds.len() as u64) as usize];
        let turns = (mix(seed, sx - n as i32, sz, kind.salt + 32) % 4) as u8;
        if let Some(instance) = placed(which, (x, top, z), turns) {
            let apart = out.iter().all(|o: &crate::jak_objects::Instance| (o.origin.0 - instance.origin.0).abs() > 4 || (o.origin.2 - instance.origin.2).abs() > 4);
            if apart {
                out.push(instance);
            }
        }
    }
    out
}

/// The land's own things about it (Adrii, 23/10: "detalles… cada cosa
/// ocupa bloques de la rejilla pero no es cúbica"): its place's loose
/// objects (`jak_objects`: palms, rocks and bushes in the Wasteland, snowy
/// pines and barrels on Snowy Mountain, plants, barrels and rocks in the
/// swamp), one now and then on its open ground, clear of hamlets and the
/// games' places. A square of `DETAIL_CELL` blocks has up to two.
const DETAIL_CELL: i32 = 24;

fn detail_groups(place: &str) -> &'static [&'static str] {
    match place {
        "spargus" => &["palmera", "roca", "planta"],
        "montana" => &["pino"],
        _ => &["planta", "barril", "roca"],
    }
}

/// The things of kind `k`'s land in a detail square (made once).
pub fn details_in(k: usize, seed: i64, cell: (i32, i32), terrain: &minecraftoss_generator::terrain::TerrainGenerator) -> Arc<Made> {
    type Kept = HashMap<(i64, usize, (i32, i32)), Arc<Made>>;
    static KEPT: Mutex<Option<Kept>> = Mutex::new(None);
    if let Some(found) = KEPT.lock().ok().and_then(|kept| kept.as_ref().and_then(|kept| kept.get(&(seed, k, cell)).cloned())) {
        return found;
    }
    let made = Arc::new(Made::new(make_details(k, seed, cell, terrain)));
    if let Ok(mut kept) = KEPT.lock() {
        let map = kept.get_or_insert_with(HashMap::new);
        if map.len() > 8192 {
            map.clear();
        }
        map.insert((seed, k, cell), made.clone());
    }
    made
}

fn make_details(k: usize, seed: i64, cell: (i32, i32), terrain: &minecraftoss_generator::terrain::TerrainGenerator) -> Vec<crate::jak_objects::Instance> {
    use crate::jak_objects::{placed, shape, OBJECTS};
    let kind = &KINDS[k];
    let groups = detail_groups(kind.place);
    let kinds: Vec<usize> = OBJECTS
        .iter()
        .enumerate()
        .filter(|(i, o)| o.place == kind.place && groups.contains(&o.group) && !o.versioned_foliage() && shape(*i).is_some())
        .map(|(i, _)| i)
        .collect();
    if kinds.is_empty() {
        return Vec::new();
    }
    let biomes = &terrain.registries.biomes;
    let land = Ground { terrain, ids: kind.land.biomes.iter().filter_map(|b| biomes.id(b)).collect() };
    let mut out = Vec::new();
    for n in 0..(mix(seed, cell.0, cell.1, kind.salt + 40) % 3) as u64 {
        let salt = kind.salt + 41 + n * 7;
        let x = cell.0 * DETAIL_CELL + 3 + (mix(seed, cell.0, cell.1, salt) % (DETAIL_CELL as u64 - 6)) as i32;
        let z = cell.1 * DETAIL_CELL + 3 + (mix(seed, cell.0, cell.1, salt + 1) % (DETAIL_CELL as u64 - 6)) as i32;
        if !land.inside(x, z) || land.surface(x, z) > land.ground(x, z) {
            continue;
        }
        // Not on a flat (left open for vehicles), nor its slopes.
        if crate::jak_lands::flat_at(kind.land, terrain, x, z).is_some() {
            continue;
        }
        // Clear of the games' places and of the hamlets round.
        if original_places::inside_a_place(&terrain.registries, seed, x, z) {
            continue;
        }
        let (hx, hz) = (x.div_euclid(kind.cell), z.div_euclid(kind.cell));
        let near_hamlet = (-1..=1).any(|dx| {
            (-1..=1).any(|dz| spot(kind, seed, (hx + dx, hz + dz)).is_some_and(|s| (s.0 - x).abs() < 30 && (s.1 - z).abs() < 30))
        });
        if near_hamlet {
            continue;
        }
        let which = kinds[(mix(seed, cell.0, cell.1, salt + 2) % kinds.len() as u64) as usize];
        let turns = (mix(seed, cell.0, cell.1, salt + 3) % 4) as u8;
        let Some(instance) = placed(which, (x, land.ground(x, z), z), turns) else { continue };
        // On level ground under all of it, apart from the others.
        let cells = instance.cells();
        let feet: Vec<i32> = cells.iter().filter(|(_, c)| c.1 == 0).map(|(at, _)| land.ground(at.0, at.2)).collect();
        let level = feet.iter().all(|&y| (y - land.ground(x, z)).abs() <= 1);
        let apart = out.iter().all(|o: &crate::jak_objects::Instance| (o.origin.0 - instance.origin.0).abs() > 8 || (o.origin.2 - instance.origin.2).abs() > 8);
        if level && apart {
            out.push(instance);
        }
    }
    out
}

/// The generated object (a pieces house's piece, or a thing of the land)
/// one of whose blocks is `pos`, in the world being played, and the cell
/// of its shape there.
pub fn instance_at(pos: (i32, i32, i32)) -> Option<(crate::jak_objects::Instance, (i32, i32, i32))> {
    if !original_places::jak_places_on() {return None;}
    generated_instance_at(pos)
}
fn generated_instance_at(pos: (i32, i32, i32)) -> Option<(crate::jak_objects::Instance, (i32, i32, i32))> {
    let (seed, _) = original_places::current_world()?;
    // Spargus's own palms and rocks.
    if let Some(found) = crate::spargus_town::objects(seed).at(pos) {
        return Some(found);
    }
    let terrain = current_terrain()?;
    for k in 0..KINDS.len() {
        let (cx, cz) = (pos.0.div_euclid(DETAIL_CELL), pos.2.div_euclid(DETAIL_CELL));
        for dx in -1..=1 {
            for dz in -1..=1 {
                if let Some(found) = details_in(k, seed, (cx + dx, cz + dz), &terrain).at(pos) {
                    return Some(found);
                }
            }
        }
    }
    // A village's things.
    for (k, kind) in KINDS.iter().enumerate().filter(|(_, kind)| kind.house.is_some()) {
        let (cx, cz) = (pos.0.div_euclid(kind.cell), pos.2.div_euclid(kind.cell));
        for dx in -1..=1 {
            for dz in -1..=1 {
                let cell = (cx + dx, cz + dz);
                let Some(s) = spot(kind, seed, cell) else { continue };
                if (s.0 - pos.0).abs() > VILLAGE_R + 48 || (s.1 - pos.2).abs() > VILLAGE_R + 48 {
                    continue;
                }
                let Some(h) = hamlet_in(k, seed, cell, &terrain) else { continue };
                if let Some(found) = village_objects(&h, seed).at(pos) {
                    return Some(found);
                }
            }
        }
    }
    None
}

/// The terrain of the world being played, for the drawing (which asks
/// where hamlets' pieces stand): the generator of the seed and registries
/// the places last found their sites for.
fn current_terrain() -> Option<Arc<minecraftoss_generator::terrain::TerrainGenerator>> {
    static TERRAIN: Mutex<Option<(i64, Arc<minecraftoss_generator::terrain::TerrainGenerator>)>> = Mutex::new(None);
    let (seed, registries) = original_places::current_world()?;
    let mut held = TERRAIN.lock().ok()?;
    if let Some((s, terrain)) = held.as_ref() {
        if *s == seed {
            return Some(terrain.clone());
        }
    }
    let terrain = Arc::new(minecraftoss_generator::terrain::TerrainGenerator::overworld(registries, seed).ok()?);
    *held = Some((seed, terrain.clone()));
    Some(terrain)
}

/// The metre of which place a piece block at `pos` shows, if a hamlet's
/// building holds it.
pub fn source_at(pos: (i32, i32, i32)) -> Option<(usize, (i32, i32, i32))> {
    if !original_places::jak_places_on() {return None;}
    // (No hamlet inside one of the games' places.)
    if original_places::inside_current_place(pos.0, pos.2) {
        return None;
    }
    let (seed, _) = original_places::current_world()?;
    let terrain = current_terrain()?;
    for (k, kind) in KINDS.iter().enumerate() {
        let (cx, cz) = (pos.0.div_euclid(kind.cell), pos.2.div_euclid(kind.cell));
        for dx in -1..=1 {
            for dz in -1..=1 {
                let cell = (cx + dx, cz + dz);
                // (Only squares whose spot is near: most have none.)
                let Some(s) = spot(kind, seed, cell) else { continue };
                if (s.0 - pos.0).abs() > 64 || (s.1 - pos.2).abs() > 64 {
                    continue;
                }
                if let Some(h) = hamlet_in(k, seed, cell, &terrain) {
                    if let Some(source) = h.source(pos) {
                        return Some((h.place, source));
                    }
                }
            }
        }
    }
    None
}

/// The hamlets' blocks over one chunk: their clearings, buildings and
/// paths.
pub fn place(k: usize, chunk: &mut Chunk, gen: &WorldGen, states: &mut HashMap<&'static str, BlockStateId>) {
    if !gen.sky_light {
        return;
    }
    let kind = &KINDS[k];
    let seed = gen.terrain.seed;
    // (The drawing's terrain is found from the places' world.)
    let _ = original_places::sites(gen.terrain.registries.clone(), seed);
    let (x0, z0) = (chunk.pos.x * 16, chunk.pos.z * 16);
    // The land's things over this chunk (their blocks where they stand; the
    // drawing finds which thing by `instance_at`).
    {
        let registries = &gen.terrain.registries;
        let block = format!("jak:pieza_{}", kind.place);
        for cx in (x0 - 16).div_euclid(DETAIL_CELL)..=(x0 + 31).div_euclid(DETAIL_CELL) {
            for cz in (z0 - 16).div_euclid(DETAIL_CELL)..=(z0 + 31).div_euclid(DETAIL_CELL) {
                for instance in details_in(k, seed, (cx, cz), &gen.terrain).iter() {
                    // Retain the old source lookup for saved chunks, but
                    // never grow a wall overhang on fresh open ground.
                    if crate::jak_objects::OBJECTS[usize::from(instance.kind)].hangs_from_wall() { continue; }
                    let Some(shape) = crate::jak_objects::shape(usize::from(instance.kind)) else { continue };
                    for ((x, y, z), cell) in instance.cells() {
                        if x < x0 || x >= x0 + 16 || z < z0 || z >= z0 + 16 || !(-63..=319).contains(&y) {
                            continue;
                        }
                        let slab = shape.cells.get(&cell).map_or("bottom", crate::jak_objects::slab_type);
                        let name = kept(format!("{block}[type={slab},waterlogged=false]"));
                        let id = *states.entry(name).or_insert_with(|| registries.blocks.parse_state(name).unwrap_or(BlockStateId::AIR));
                        chunk.set_block_raw((x - x0) as usize, y, (z - z0) as usize, id, registries);
                    }
                }
            }
        }
    }
    let cell_of = |v: i32| v.div_euclid(kind.cell);
    for cx in cell_of(x0) - 1..=cell_of(x0 + 15) + 1 {
        for cz in cell_of(z0) - 1..=cell_of(z0 + 15) + 1 {
            let Some(h) = hamlet_in(k, seed, (cx, cz), &gen.terrain) else { continue };
            let Some(blocks) = blocks_of(&h, seed, &gen.terrain) else { continue };
            let registries = &gen.terrain.registries;
            for &((x, y, z), name) in blocks.iter() {
                if x < x0 || x >= x0 + 16 || z < z0 || z >= z0 + 16 || !(-63..=319).contains(&y) {
                    continue;
                }
                let id = *states.entry(name).or_insert_with(|| registries.blocks.parse_state(name).unwrap_or(BlockStateId::AIR));
                chunk.set_block_raw((x - x0) as usize, y, (z - z0) as usize, id, registries);
            }
        }
    }
}

/// A name made once and kept.
fn kept(name: String) -> &'static str {
    static NAMES: OnceLock<Mutex<HashMap<String, &'static str>>> = OnceLock::new();
    let mut names = NAMES.get_or_init(Default::default).lock().unwrap_or_else(|e| e.into_inner());
    if let Some(&found) = names.get(&name) {
        return found;
    }
    let leaked: &'static str = Box::leak(name.clone().into_boxed_str());
    names.insert(name, leaked);
    leaked
}

/// A hamlet's blocks (made once): its clearing of ground level at the
/// building's foot (the land cleared over it, the ground filled under it),
/// its building's pieces, and its paths half way to the hamlets beside it.
fn blocks_of(h: &Hamlet, seed: i64, terrain: &minecraftoss_generator::terrain::TerrainGenerator) -> Option<Arc<Vec<((i32, i32, i32), &'static str)>>> {
    type Kept = HashMap<(i64, usize, (i32, i32)), Arc<Vec<((i32, i32, i32), &'static str)>>>;
    static KEPT: Mutex<Option<Kept>> = Mutex::new(None);
    let key = (seed, h.kind, h.spot);
    if let Some(found) = KEPT.lock().ok().and_then(|k| k.as_ref().and_then(|k| k.get(&key).cloned())) {
        return Some(found);
    }
    let kind = &KINDS[h.kind];
    let biomes = &terrain.registries.biomes;
    let land = Ground { terrain, ids: kind.land.biomes.iter().filter_map(|b| biomes.id(b)).collect() };
    let ground_block = kept(format!("jak:{}", kind.ground));
    let path_block = kept(format!("jak:{}", kind.path));
    let piece = |low: bool| kept(format!("jak:pieza_{}[type={},waterlogged=false]", PLACES[h.place].name, if low { "bottom" } else { "top" }));
    let mut out: Vec<((i32, i32, i32), &'static str)> = Vec::new();
    let (w, height, d) = h.size();
    let top = h.origin.1 + 1; // the clearing's first block over its ground
    let r = h.radius();
    let (sx, sz) = h.spot;
    // The clearing: round, its rim a little ragged, level at the
    // building's foot and sloping over its last four blocks to the land's
    // own height (23/10: cut level all the way, it was a pit in the swamp).
    for dx in -r..=r {
        for dz in -r..=r {
            let edge = r as f32 - (mix(seed, sx + dx, sz + dz, kind.salt + 9) % 3) as f32;
            let distance = ((dx * dx + dz * dz) as f32).sqrt();
            if distance > edge {
                continue;
            }
            let (x, z) = (sx + dx, sz + dz);
            let land_top = land.ground(x, z) - 1;
            let slope = ((distance - (edge - 4.0)) / 4.0).clamp(0.0, 1.0);
            let y = ((top - 1) as f32 + (land_top - (top - 1)) as f32 * slope).round() as i32;
            for up in y + 1..(top + height + 3).max(land_top + 12) {
                out.push(((x, up, z), "minecraft:air"));
            }
            out.push(((x, y, z), ground_block));
            let mut down = y - 1;
            while down > land_top - 1 && down > y - 12 {
                out.push(((x, down, z), ground_block));
                down -= 1;
            }
        }
    }
    // Paths to the hamlets of the squares beside it, to half way: a path
    // two wide on the land's top, its own ground a block either side.
    let cell = (sx.div_euclid(kind.cell), sz.div_euclid(kind.cell));
    for (dx, dz) in [(1, 0), (-1, 0), (0, 1), (0, -1)] {
        let other = (cell.0 + dx, cell.1 + dz);
        let Some(o) = hamlet_in(h.kind, seed, other, terrain) else { continue };
        let (ex, ez) = ((sx + o.spot.0) / 2, (sz + o.spot.1) / 2);
        let length = (((ex - sx) as f32).powi(2) + ((ez - sz) as f32).powi(2)).sqrt();
        let steps = length.ceil() as i32;
        let mut laid: HashMap<(i32, i32), bool> = HashMap::new();
        for i in 0..=steps {
            let t = i as f32 / steps.max(1) as f32;
            // A gentle wander, the same seen from either end.
            let wobble = ((t * std::f32::consts::PI).sin()) * ((mix(seed, sx.min(o.spot.0), sz.min(o.spot.1), kind.salt + 11) % 9) as f32 - 4.0);
            let (nx, nz) = (-(ez - sz) as f32 / length.max(1.0), (ex - sx) as f32 / length.max(1.0));
            let (px, pz) = (sx as f32 + (ex - sx) as f32 * t + nx * wobble, sz as f32 + (ez - sz) as f32 * t + nz * wobble);
            for ox in -2i32..=2 {
                for oz in -2i32..=2 {
                    let (x, z) = ((px + ox as f32 * 0.5).round() as i32, (pz + oz as f32 * 0.5).round() as i32);
                    let core = ox.abs() <= 1 && oz.abs() <= 1;
                    let e = laid.entry((x, z)).or_insert(false);
                    *e |= core;
                }
            }
        }
        for ((x, z), core) in laid {
            if ((x - sx).pow(2) + (z - sz).pow(2)) < (r - 1).pow(2) || !land.inside(x, z) || land.surface(x, z) > land.ground(x, z) {
                continue;
            }
            let y = land.ground(x, z) - 1;
            out.push(((x, y, z), if core { path_block } else { ground_block }));
            out.push(((x, y + 1, z), "minecraft:air"));
            out.push(((x, y + 2, z), "minecraft:air"));
        }
    }
    // A house of blocks, its door toward the nearest hamlet's way (south
    // by default).
    if let Some(house) = kind.house.filter(|_| h.house) {
        // Its houses round the middle, a street of its path ground from
        // each door to the middle, its landmark there, its things beside
        // the houses.
        let houses = village_houses((sx, h.origin.1, sz), seed, kind.salt + 20, h.copied.is_some());
        // Its level's own building, metre by metre (`jak_pieces`), over
        // the clearing's ground.
        if let Some((b, origin)) = h.copied {
            if let Some(source) = crate::jak_pieces::source(h.place) {
                let cut = crate::jak_pieces::cut_area(source, (b.lo.0, b.hi.0), (b.lo.2, b.hi.2));
                for (&(cx, cy, cz), p) in cut.iter() {
                    if !(b.lo.1..=b.hi.1).contains(&cy) {
                        continue;
                    }
                    let mut area = 0.0f32;
                    let mut high = 0.0f32;
                    for t in &p.tris {
                        let [a, bb, c] = t.p.map(glam::Vec3::from_array);
                        area += (bb - a).cross(c - a).length() * 0.5;
                        high = high.max(a.y).max(bb.y).max(c.y);
                    }
                    if area < 0.002 {
                        continue;
                    }
                    let at = (cx - b.lo.0 + origin.0, cy - b.lo.1 + origin.1, cz - b.lo.2 + origin.2);
                    out.push((at, piece(high <= 0.5)));
                }
            }
        }
        for &(origin, w, facing) in &houses {
            out.extend(house_blocks(&house, origin, w, HOUSE_H, facing));
            let m = w / 2;
            let (door, step): ((i32, i32), (i32, i32)) = match facing {
                0 => ((origin.0 + m, origin.2 - 1), (0, -1)),
                1 => ((origin.0 + w, origin.2 + m), (1, 0)),
                2 => ((origin.0 + m, origin.2 + w), (0, 1)),
                _ => ((origin.0 - 1, origin.2 + m), (-1, 0)),
            };
            let (mut px, mut pz) = door;
            for _ in 0..30 {
                if (px - sx).pow(2) + (pz - sz).pow(2) <= 4 {
                    break;
                }
                for (ox, oz) in [(0, 0), (step.1, step.0), (-step.1, -step.0)] {
                    let inside_house = houses.iter().any(|(o, ow, _)| px + ox >= o.0 && px + ox < o.0 + ow && pz + oz >= o.2 && pz + oz < o.2 + ow);
                    if inside_house {
                        continue;
                    }
                    out.push(((px + ox, top - 1, pz + oz), if ox == 0 && oz == 0 { path_block } else { ground_block }));
                }
                let (dx, dz) = (sx - px, sz - pz);
                if dx.abs() > dz.abs() {
                    px += dx.signum();
                } else {
                    pz += dz.signum();
                }
            }
        }
        for k in 0..3 {
            out.push(((sx, top + k, sz), house.landmark));
        }
        for instance in village_objects(h, seed).iter() {
            if crate::jak_objects::OBJECTS[usize::from(instance.kind)].hangs_from_wall() { continue; }
            let Some(shape) = crate::jak_objects::shape(usize::from(instance.kind)) else { continue };
            let block = format!("jak:pieza_{}", PLACES[h.place].name);
            for (at, cell) in instance.cells() {
                let slab = shape.cells.get(&cell).map_or("bottom", crate::jak_objects::slab_type);
                out.push((at, kept(format!("{block}[type={slab},waterlogged=false]"))));
            }
        }
        let made = Arc::new(out);
        if let Ok(mut held) = KEPT.lock() {
            held.get_or_insert_with(HashMap::new).insert(key, made.clone());
        }
        return Some(made);
    }
    // The building's pieces, last (over the clearing's ground).
    let b = h.building;
    let source = crate::jak_pieces::source(h.place)?;
    let cut = crate::jak_pieces::cut_area(source, (b.lo.0, b.hi.0), (b.lo.2, b.hi.2));
    for (&(cx, cy, cz), p) in cut.iter() {
        if !(b.lo.1..=b.hi.1).contains(&cy) {
            continue;
        }
        let mut area = 0.0f32;
        let mut high = 0.0f32;
        for t in &p.tris {
            let [a, bb, c] = t.p.map(glam::Vec3::from_array);
            area += (bb - a).cross(c - a).length() * 0.5;
            high = high.max(a.y).max(bb.y).max(c.y);
        }
        if area < 0.002 {
            continue;
        }
        let at = (cx - b.lo.0 + h.origin.0, cy - b.lo.1 + h.origin.1, cz - b.lo.2 + h.origin.2);
        out.push((at, piece(high <= 0.5)));
    }
    let _ = (w, d);
    let made = Arc::new(out);
    if let Ok(mut held) = KEPT.lock() {
        let map = held.get_or_insert_with(HashMap::new);
        if map.len() > 1024 {
            map.clear();
        }
        map.insert(key, made.clone());
    }
    Some(made)
}

/// What the far terrain shows of world column `(x, z)`: a house of
/// blocks' roof there (its top's height, its place and its roof texture),
/// if a hamlet's house stands on it.
pub fn far_house_at(seed: i64, terrain: &minecraftoss_generator::terrain::TerrainGenerator, x: i32, z: i32) -> Option<(f32, usize, &'static str)> {
    for (k, kind) in KINDS.iter().enumerate() {
        let Some(house) = kind.house else { continue };
        let (cx, cz) = (x.div_euclid(kind.cell), z.div_euclid(kind.cell));
        for dx in -1..=1 {
            for dz in -1..=1 {
                let cell = (cx + dx, cz + dz);
                let Some(s) = spot(kind, seed, cell) else { continue };
                if (s.0 - x).abs() > VILLAGE_R + 40 || (s.1 - z).abs() > VILLAGE_R + 40 {
                    continue;
                }
                let Some(h) = hamlet_in(k, seed, cell, terrain) else { continue };
                if !h.house {
                    continue;
                }
                let roof = crate::jak_walls::WALLS.iter().find(|w| format!("jak:{}", w.block) == house.roof).map_or("", |w| w.icon);
                if let Some((b, o)) = h.copied {
                    let (w, d, height) = (b.hi.0 - b.lo.0 + 1, b.hi.2 - b.lo.2 + 1, b.hi.1 - b.lo.1 + 1);
                    if x >= o.0 && x < o.0 + w && z >= o.2 && z < o.2 + d {
                        let wall = crate::jak_walls::WALLS.iter().find(|w| w.place == kind.place && !w.roof).map_or(roof, |w| w.icon);
                        return Some(((o.1 + height) as f32, h.place, wall));
                    }
                }
                let on_a_house = village_houses((h.spot.0, h.origin.1, h.spot.1), seed, kind.salt + 20, h.copied.is_some())
                    .iter()
                    .any(|(o, w, _)| x >= o.0 - 1 && x <= o.0 + w && z >= o.2 - 1 && z <= o.2 + w);
                if !on_a_house {
                    continue;
                }
                return Some(((h.origin.1 + HOUSE_H + 2) as f32, h.place, roof));
            }
        }
    }
    None
}

/// The nearest hamlet of kind `k` to `(x, z)` within `reach` (tests, `mc
/// casa`): where its spot's ground is.
pub fn nearest(k: usize, registries: Arc<minecraftoss_core::Registries>, seed: i64, (x, z): (i32, i32), reach: i32) -> Option<Hamlet> {
    let terrain = minecraftoss_generator::terrain::TerrainGenerator::overworld(registries, seed).ok()?;
    let kind = &KINDS[k];
    let (cx, cz) = (x.div_euclid(kind.cell), z.div_euclid(kind.cell));
    let rings = reach / kind.cell + 1;
    (0..=rings).find_map(|ring| {
        let mut found: Vec<Hamlet> = Vec::new();
        for dx in -ring..=ring {
            for dz in -ring..=ring {
                if dx.abs().max(dz.abs()) == ring {
                    found.extend(hamlet_in(k, seed, (cx + dx, cz + dz), &terrain));
                }
            }
        }
        found.into_iter().min_by_key(|h| (h.spot.0 - x).pow(2) + (h.spot.1 - z).pow(2))
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn registries() -> Option<Arc<minecraftoss_core::Registries>> {
        let paths = minecraftoss_core::registries::DataPaths::discover().ok()?;
        paths.block_catalog.is_file().then(|| Arc::new(minecraftoss_core::Registries::load(&paths).unwrap()))
    }

    /// The lands have their things about (seed 12345): a fair few in the
    /// squares round each land's nearest hamlet, each one found where it
    /// stands.
    #[test]
    fn the_lands_have_their_things() {
        let Some(registries) = registries() else { return };
        let terrain = minecraftoss_generator::terrain::TerrainGenerator::overworld(registries.clone(), 12345).unwrap();
        let _ = original_places::sites(registries.clone(), 12345);
        for (k, kind) in KINDS.iter().enumerate() {
            let place = PLACES.iter().position(|p| p.name == kind.place).unwrap();
            if crate::jak_pieces::source(place).is_none() {
                continue;
            }
            let Some(h) = nearest(k, registries.clone(), 12345, (0, 0), 6000) else { continue };
            let (cx, cz) = (h.spot.0.div_euclid(DETAIL_CELL), h.spot.1.div_euclid(DETAIL_CELL));
            let mut found = Vec::new();
            for dx in -6..=6 {
                for dz in -6..=6 {
                    found.extend(details_in(k, 12345, (cx + dx, cz + dz), &terrain).iter().copied());
                }
            }
            let names: Vec<&str> = found.iter().map(|i| crate::jak_objects::OBJECTS[usize::from(i.kind)].group).collect();
            println!("{}: {} things {names:?}", kind.name, found.len());
            assert!(found.len() >= 10, "{}", kind.name);
            let (at, cell) = found[0].cells()[0];
            assert_eq!(generated_instance_at(at), Some((found[0], cell)));
            assert_eq!(instance_at(at),original_places::jak_places_on().then_some((found[0],cell)),
                "disabled optional hamlets must not be reconstructed by an ordinary city mesh lookup");
        }
    }

    /// Each land's place has buildings to copy (picked off its map),
    /// and the world makes hamlets of them (seed 12345): its building's
    /// pieces in the generated chunk, on the clearing's ground, the drawing
    /// finding the same metres there.
    #[test]
    fn hamlets_copy_their_levels_buildings() {
        // (Only with the games' places in the world, `IW4L_LUGARES_JAK=1`.)
        if !crate::original_places::jak_places_on() {
            return;
        }
        let Some(registries) = registries() else { return };
        for (k, kind) in KINDS.iter().enumerate() {
            let place = PLACES.iter().position(|p| p.name == kind.place).unwrap();
            if crate::jak_pieces::source(place).is_none() {
                println!("{}: no export, skipped", kind.name);
                continue;
            }
            let found = buildings(place);
            println!("{}: {} buildings, first {:?}", kind.name, found.len(), found.first());
            assert!(kind.house.is_some() || found.len() >= 3, "{}", kind.name);
            let h = nearest(k, registries.clone(), 12345, (0, 0), 6000).unwrap_or_else(|| panic!("{} near", kind.name));
            println!("{}: {:?}", kind.name, h);
            let terrain = Arc::new(minecraftoss_generator::terrain::TerrainGenerator::overworld(registries.clone(), 12345).unwrap());
            let gen = Arc::new(WorldGen::new(terrain).unwrap());
            crate::jak_structures::install();
            let mut map = minecraftoss_world::chunk_map::ChunkMap::with_worldgen(gen, 2, 4);
            // (Every chunk its building reaches: a house straddles two.)
            let blocks = &registries.blocks;
            let mut names: HashMap<String, usize> = HashMap::new();
            let (lo, hi) = ((h.origin.0 - 1, h.origin.2 - 1), (h.origin.0 + h.size().0 + 1, h.origin.2 + h.size().2 + 1));
            for cx in lo.0 >> 4..=hi.0 >> 4 {
                for cz in lo.1 >> 4..=hi.1 >> 4 {
                    let chunk = map.load_now(minecraftoss_core::ChunkPos::new(cx, cz));
                    for x in 0..16 {
                        for z in 0..16 {
                            for y in h.origin.1..h.origin.1 + h.size().1 {
                                *names.entry(blocks.block(blocks.block_of(chunk.block(x, y, z))).name.to_string()).or_default() += 1;
                            }
                        }
                    }
                }
            }
            println!("{}: {:?}", kind.name, names.iter().filter(|(n, _)| n.starts_with("jak:")).collect::<Vec<_>>());
            let piece = format!("jak:pieza_{}", kind.place);
            assert!(kind.house.is_some() || names.get(&piece).copied().unwrap_or(0) > 20, "{}: its building's pieces", kind.name);
            assert!(names.get(&format!("jak:{}", kind.ground)).copied().unwrap_or(0) > 0, "{}: its clearing", kind.name);
            // The drawing finds the building's metres where they stand.
            let (seed, _) = (12345, ());
            let _ = original_places::sites(registries.clone(), seed);
            if let Some(house) = kind.house {
                // Its house of blocks: its walls joined, its door, its roof.
                let walls = names.get(house.wall).copied().unwrap_or(0) + house.window.and_then(|w| names.get(w).copied()).unwrap_or(0);
                let door = names.get(house.door).copied().unwrap_or(0);
                let roof = names.get(house.roof).copied().unwrap_or(0);
                println!("{}: {walls} wall blocks, {door} door halves, {roof} roof tiles", kind.name);
                let houses = village_houses((h.spot.0, h.origin.1, h.spot.1), 12345, kind.salt + 20, h.copied.is_some()).len();
                let pieces = names.get(&format!("jak:pieza_{}", kind.place)).copied().unwrap_or(0);
                println!("{}: {houses} houses, copied {:?}, {pieces} pieces of its level", kind.name, h.copied);
                assert!(h.copied.is_none() || pieces > 100, "{}: its level's building", kind.name);
                assert!(houses >= 3 && walls as i32 >= houses as i32 * (4 * (HOUSE_W - 1) * HOUSE_H - 4) && door == 2 * houses && roof as i32 >= houses as i32 * (HOUSE_W + 2).pow(2), "{}", kind.name);
                assert!(instance_at((h.origin.0 + 1, h.origin.1 + 1, h.origin.2 + 1)).is_none(), "{}: no object holds its walls", kind.name);
            } else {
                let inside = (h.origin.0 + 2, h.origin.1 + 2, h.origin.2 + 2);
                assert_eq!(source_at(inside).map(|(p, _)| p), Some(place), "{}", kind.name);
            }
        }
    }
}
