//! Jak's walls and roofs as blocks that join by themselves (Adrii, 23/10:
//! "edificios compuestos por bloques reales de la rejilla… los bloques
//! compatibles pueden conectarse automáticamente, como los paneles de
//! cristal o las vallas de hierro: tramos continuos, esquinas, extremos y
//! uniones según sus vecinos"; "que las paredes tengan más detalle y que
//! los tejados se parezcan más a los reales: usar solo la cara visible y
//! arreglar la parte de atrás").
//!
//! A wall block (`jak:muro_<place>_<wall>`) is one of the game's panes to
//! the world (a clone of `glass_pane`: it joins the blocks beside it as
//! panes do, `north`/`east`/`south`/`west` kept in its state, updated by
//! the game whenever a neighbour is put down, broken or loaded, across
//! chunk edges too; its collision the pane's thin posts and arms). What it
//! is drawn as is a metre of its place's own walls: the level cut into
//! metres (`jak_pieces::cut_area`) over its wall textures, each metre
//! filed by which of its sides its wall reaches (its joins) and whether it
//! goes on up and down, kept turned four ways. The metre is the level's
//! facade as the game shows it, with its relief: its back is slid to the
//! far face of the block and closed with a flat face of its texture, and
//! the block turns its facade to the open side (the outside of a house).
//!
//! A roof block (`jak:tejado_<place>`) is a whole block to the world (a
//! clone of `packed_mud`: Jak and the creatures stand on it) drawn as a
//! metre of its place's own roofs, picked by which roof blocks lie beside
//! it, its top at the block's top, closed underneath and at its open sides.
//!
//! Each block stands alone: broken, that metre goes and its neighbours
//! join afresh; nothing holds a building together but the blocks.
use std::collections::HashMap;
use std::sync::OnceLock;

use glam::Vec3;

use crate::jak_items::{BlockLook, JakBlock, JakItem, JakKind};
use crate::jak_pieces::{self, PieceTri};
use crate::original_places::{self, PLACES};

/// One of a place's walls, or its roof.
#[derive(Clone, Copy, Debug)]
pub struct Wall {
    pub place: &'static str,
    /// Its block (`jak:<block>`).
    pub block: &'static str,
    pub title: &'static str,
    /// Parts of the names of the textures it is made of.
    pub textures: &'static [&'static str],
    /// The texture its item shows, and its closed faces wear.
    pub icon: &'static str,
    /// A roof (`tejado_<place>`): lying metres, a whole block.
    pub roof: bool,
}

const fn w(place: &'static str, block: &'static str, title: &'static str, textures: &'static [&'static str], icon: &'static str) -> Wall {
    Wall { place, block, title, textures, icon, roof: false }
}

const fn r(place: &'static str, block: &'static str, title: &'static str, textures: &'static [&'static str], icon: &'static str) -> Wall {
    Wall { place, block, title, textures, icon, roof: true }
}

/// Every place's walls (its buildings' walls by their textures, its
/// windows) and its roof.
pub const WALLS: [Wall; 17] = [
    w("spargus", "muro_spargus_estuco", "Muro de Spargus: estuco", &["wascity-stucco-wall", "wascity-wall-weathered"], "wascity-stucco-wall-bleached-01"),
    w("spargus", "muro_spargus_piedra", "Muro de Spargus: piedra", &["wascity-stone-plain-wall", "wascity-stonewall-bricks"], "wascity-stonewall-bricks"),
    w("spargus", "muro_spargus_metal", "Muro de Spargus: metal", &["wascity-outerwall-metal", "wascity-metal-wall"], "wascity-outerwall-metal"),
    w("spargus", "muro_spargus_ventana", "Ventana de Spargus", &["wascity-stain-window", "wascity-window-glass", "waspala-window"], "wascity-stain-window-01"),
    w("haven", "muro_haven_tablas", "Muro de Haven: tablas", &["city-slumwall", "city-slumbase-wall"], "city-slumwall-01"),
    w("haven", "muro_haven_chapa", "Muro de Haven: chapa", &["city-slumwall-metalsiding", "city-fort-red", "city-fort-yellow"], "city-slumwall-metalsiding-03"),
    w("villarrefugio", "muro_villarrefugio_madera", "Muro de Villarrefugio: madera", &["vil-hut-wood", "vil-plankwood-onwalls"], "vil-hut-wood-01"),
    w("villarrefugio", "muro_villarrefugio_piedra", "Muro de Villarrefugio: piedra", &["vil-stonewall", "vil-rockwall", "vil-stone-wall"], "vil-hut-wood-01"),
    w("montana", "muro_montana_madera", "Muro de la Montaña Nevada: madera", &["snow-wood", "snow-woodpanels"], "snow-woodpanels"),
    w("montana", "muro_montana_fuerte", "Muro de la Montaña Nevada: fuerte", &["snow-fortwall", "snow-fort-wall", "snow-fortwallbase"], "snow-fortwallbase"),
    w("pantano", "muro_pantano_madera", "Muro del Pantano: madera", &["swp-hut-wood", "swp-plankwood"], "swp-hut-wood-01"),
    w("pantano", "muro_pantano_piedra", "Muro del Pantano: piedra", &["swp-stonewall", "swp-rockwall", "swp-stone-wall"], "swp-hut-wood-01"),
    r("spargus", "tejado_spargus", "Tejado de Spargus", &["wascity-roof"], "wascity-roof-1"),
    r("haven", "tejado_haven", "Tejado de Haven", &["city-slum-roof", "city-roofmetal"], "city-slum-roof"),
    r("villarrefugio", "tejado_villarrefugio", "Tejado de Villarrefugio", &["vil-hut-roof", "vil1-sages-strawroof", "vil-huttoprim"], "vil-hut-roof-tile-01"),
    r("montana", "tejado_montana", "Tejado de la Montaña Nevada", &["snow-metalroof"], "snow-metalroof-01"),
    r("pantano", "tejado_pantano", "Tejado del Pantano", &["swp-roof-tile"], "swp-roof-tile-01"),
];

/// The wall and roof blocks' looks (their item and far look: vanilla's
/// pane in their texture, a cube of it for a roof; the world draws their
/// metres).
pub fn wall_blocks() -> &'static [JakBlock] {
    static BLOCKS: OnceLock<Vec<JakBlock>> = OnceLock::new();
    BLOCKS.get_or_init(|| {
        WALLS
            .iter()
            .map(|w| JakBlock {
                path: w.block,
                look: if w.roof {
                    BlockLook::All(w.icon)
                } else {
                    BlockLook::Like { vanilla: "glass_pane", textures: Box::leak(vec![("*", w.icon)].into_boxed_slice()) }
                },
            })
            .collect()
    })
}

pub fn wall_items() -> &'static [JakItem] {
    static ITEMS: OnceLock<Vec<JakItem>> = OnceLock::new();
    ITEMS.get_or_init(|| WALLS.iter().map(|w| JakItem { path: w.block, title: w.title, kind: JakKind::Block }).collect())
}

/// Which wall or roof a block is, by its name (`muro_<place>_<wall>`,
/// `tejado_<place>`).
pub fn wall_of(path: &str) -> Option<usize> {
    if !path.starts_with("muro_") && !path.starts_with("tejado_") {
        return None;
    }
    WALLS.iter().position(|w| w.block == path)
}

/// A wall's place (`PLACES`).
pub fn place_of(wall: usize) -> Option<usize> {
    let place = WALLS.get(wall)?.place;
    PLACES.iter().position(|p| p.name == place)
}

/// Joins: north (-z), east (+x), south (+z), west (-x), up, down.
pub type Joins = [bool; 6];

/// A metre of a wall, its back at the block's far face (or a metre of a
/// roof, its top at the block's top).
struct Tile {
    tris: Vec<PieceTri>,
    /// Which way its facade looks, unturned (x, z; 0 where it has no
    /// facade that way).
    front: (i8, i8),
}

/// A wall's metres, by the joins they fit (turned `0..4` quarter turns
/// clockwise seen from above).
pub struct Library {
    tiles: Vec<Tile>,
    by_joins: HashMap<Joins, Vec<(u32, u8)>>,
}

impl Library {
    pub fn tile_count(&self) -> usize {
        self.tiles.len()
    }

    pub fn fits(&self, joins: Joins) -> Option<&Vec<(u32, u8)>> {
        self.by_joins.get(&joins).filter(|v| !v.is_empty())
    }

    /// The tiles fitting `joins`, else the nearest: the up and down joins
    /// let go first, then the sides one by one (a lone post for a block
    /// joined to nothing).
    pub fn best_fit(&self, joins: Joins) -> Option<&Vec<(u32, u8)>> {
        if let Some(found) = self.fits(joins) {
            return Some(found);
        }
        for vertical in [[joins[4], joins[5]], [true, true], [false, true], [true, false], [false, false]] {
            let mut masks: Vec<u8> = (0..16).collect();
            masks.sort_by_key(|m| m.count_ones());
            for mask in masks {
                let mut j = joins;
                j[4] = vertical[0];
                j[5] = vertical[1];
                for k in 0..4 {
                    if mask & (1 << k) != 0 {
                        j[k] = !joins[k];
                    }
                }
                if let Some(found) = self.fits(j) {
                    return Some(found);
                }
            }
        }
        None
    }
}

/// `joins` turned a quarter turn clockwise: its north join goes east.
fn turned(j: Joins) -> Joins {
    [j[3], j[0], j[1], j[2], j[4], j[5]]
}

/// A point of a tile turned a quarter turn clockwise about its middle.
fn turn_point([x, y, z]: [f32; 3]) -> [f32; 3] {
    [1.0 - z, y, x]
}

/// A direction (x, z) turned a quarter turn clockwise (as `turn_point`).
fn turn_dir((x, z): (i8, i8)) -> (i8, i8) {
    (-z, x)
}

/// A wall's tiles, made once (none without its levels, or with no wall of
/// its textures).
pub fn library(wall: usize) -> Option<&'static Library> {
    static LIBRARIES: OnceLock<Vec<OnceLock<Option<Library>>>> = OnceLock::new();
    let all = LIBRARIES.get_or_init(|| (0..WALLS.len()).map(|_| OnceLock::new()).collect());
    all.get(wall)?
        .get_or_init(|| {
            let started = std::time::Instant::now();
            let made = make_library(wall)?;
            diag::info!(
                World,
                "{}: {} metres, {} join patterns, in {:.2} s",
                WALLS[wall].block,
                made.tiles.len(),
                made.by_joins.len(),
                started.elapsed().as_secs_f32()
            );
            Some(made)
        })
        .as_ref()
}

/// How near a face a point counts as reaching it.
const TOUCH: f32 = 0.08;

fn make_library(wall: usize) -> Option<Library> {
    let kind = &WALLS[wall];
    let place = place_of(wall)?;
    if kind.block=="muro_haven_chapa" {
        // Cropped level fragments contain unrelated adjoining facades. They
        // cannot tile as independent wall blocks. Use closed grid panels in
        // their original corrugated-metal material, with full shared edges.
        let textures=original_places::textures_of(&PLACES[place]);
        let icon=textures.iter().position(|t|*t==kind.icon)? as u16;
        let mut tris=Vec::new();
        for d in [(0,-1),(1,0),(0,1),(-1,0)]{tris.extend(face_quad(d,icon));}
        for y in [0.,1.] {
            let q=[[0.,y,0.],[1.,y,0.],[1.,y,1.],[0.,y,1.]];
            for i in [[0,1,2],[0,2,3]]{tris.push(PieceTri{p:i.map(|k|q[k]),uv:[[0.,0.],[1.,0.],[1.,1.]],color:[[128,128,128,255];3],texture:icon,blend:false});}
        }
        let mut by_joins=HashMap::new();
        for bits in 0..64 {by_joins.insert(std::array::from_fn(|i|bits&(1<<i)!=0),vec![(0,0)]);}
        return Some(Library{tiles:vec![Tile{tris,front:(0,-1)}],by_joins});
    }
    let source = jak_pieces::source(place)?;
    let textures = original_places::textures_of(&PLACES[place]);
    let wanted: Vec<bool> = textures.iter().map(|t| kind.textures.iter().any(|k| t.contains(k))).collect();
    let icon = textures.iter().position(|t| *t == kind.icon).unwrap_or(0) as u16;
    // The level's triangles of its textures (a wall's upright ones, a
    // roof's lying ones), as a level of their own.
    let (mut lo, mut hi) = (Vec3::MAX, Vec3::MIN);
    let tris: Vec<jak_pieces::SourceTri> = source
        .tris
        .iter()
        .filter(|t| wanted.get(usize::from(t.texture)).copied().unwrap_or(false) && !t.blend)
        .filter(|t| {
            let n = (t.p[1] - t.p[0]).cross(t.p[2] - t.p[0]).normalize_or_zero();
            if kind.roof { n.y > 0.3 } else { n.y.abs() < 0.5 }
        })
        .inspect(|t| {
            for p in t.p {
                lo = lo.min(p);
                hi = hi.max(p);
            }
        })
        .copied()
        .collect();
    if tris.len() < 20 {
        return None;
    }
    let cut = jak_pieces::cut_area(&jak_pieces::Source::new(tris), (lo.x.floor() as i32, hi.x.ceil() as i32), (lo.z.floor() as i32, hi.z.ceil() as i32));
    // Candidates per joins: (score, cell).
    let mut found: HashMap<Joins, Vec<(f32, (i32, i32, i32))>> = HashMap::new();
    for (&cell, piece) in cut.iter() {
        let classified = if kind.roof { classify_roof(piece) } else { classify(piece) };
        let Some((joins, score)) = classified else { continue };
        let list = found.entry(joins).or_default();
        list.push((score, cell));
        if list.len() > 24 {
            list.sort_by(|a, b| b.0.total_cmp(&a.0));
            list.truncate(8);
        }
    }
    let mut tiles = Vec::new();
    let mut by_joins: HashMap<Joins, Vec<(u32, u8)>> = HashMap::new();
    for (joins, mut list) in found {
        list.sort_by(|a, b| b.0.total_cmp(&a.0));
        for &(_, cell) in list.iter().take(4) {
            let Some(piece) = cut.get(&cell) else { continue };
            let tile = if kind.roof { topped(piece) } else { backed(piece, joins, icon) };
            let index = tiles.len() as u32;
            tiles.push(tile);
            let mut j = joins;
            for turn in 0..4u8 {
                by_joins.entry(j).or_default().push((index, turn));
                j = turned(j);
            }
        }
    }
    if tiles.is_empty() {
        return None;
    }
    Some(Library { tiles, by_joins })
}

/// A metre's joins and how good a wall metre it is (its area and relief,
/// for ones axis-aligned and upright enough), or none for a metre that is
/// no wall (a scrap, a slanted or jumbled one).
fn classify(piece: &jak_pieces::Piece) -> Option<(Joins, f32)> {
    let mut area = 0.0f32;
    let mut aligned = 0.0f32;
    let mut touch = [false; 6];
    for t in &piece.tris {
        let [a, b, c] = t.p.map(Vec3::from_array);
        let n = (b - a).cross(c - a);
        let size = n.length() * 0.5;
        let n = n.normalize_or_zero();
        area += size;
        if n.y.abs() < 0.3 && (n.x.abs() > 0.9 || n.z.abs() > 0.9) {
            aligned += size;
        }
        for p in t.p {
            touch[0] |= p[2] < TOUCH;
            touch[1] |= p[0] > 1.0 - TOUCH;
            touch[2] |= p[2] > 1.0 - TOUCH;
            touch[3] |= p[0] < TOUCH;
            touch[4] |= p[1] > 1.0 - TOUCH;
            touch[5] |= p[1] < TOUCH;
        }
    }
    if area < 0.6 || aligned < area * 0.7 || piece.tris.len() > 90 {
        return None;
    }
    // A wall reaches up and down: a metre reaching neither is a scrap, one
    // reaching both is a whole metre of wall (preferred: the others are
    // its top or its foot).
    if !touch[4] && !touch[5] {
        return None;
    }
    let whole = if touch[4] && touch[5] { 2.0 } else { 0.0 };
    // Its facade: one side's faces, not both (a wall seen from one side
    // in its level; a slab with both faces is a wall's end or a parapet).
    let sides = [touch[0], touch[1], touch[2], touch[3]];
    let along_x = sides[1] || sides[3];
    let along_z = sides[0] || sides[2];
    if along_x != along_z {
        let (mut front, mut back) = (0.0f32, 0.0f32);
        for t in &piece.tris {
            let [a, b, c] = t.p.map(Vec3::from_array);
            let n = (b - a).cross(c - a);
            let size = n.length() * 0.5;
            let n = n.normalize_or_zero();
            let across = if along_x { n.z } else { n.x };
            if across.abs() > 0.7 {
                if across < 0.0 {
                    front += size;
                } else {
                    back += size;
                }
            }
        }
        let one_sided = front.max(back) >= (front + back) * 0.8;
        if !one_sided {
            return None;
        }
    }
    Some((touch, whole + aligned / area + area.min(3.0) / 3.0 - piece.tris.len() as f32 / 180.0))
}

/// A roof metre's joins (the sides its roof reaches) and its worth: a
/// lying metre with most of the block covered.
fn classify_roof(piece: &jak_pieces::Piece) -> Option<(Joins, f32)> {
    let mut area = 0.0f32;
    let mut touch = [false; 6];
    let mut covered = [[false; 4]; 4];
    for t in &piece.tris {
        let [a, b, c] = t.p.map(Vec3::from_array);
        let n = (b - a).cross(c - a);
        area += n.length() * 0.5;
        for p in t.p {
            touch[0] |= p[2] < TOUCH;
            touch[1] |= p[0] > 1.0 - TOUCH;
            touch[2] |= p[2] > 1.0 - TOUCH;
            touch[3] |= p[0] < TOUCH;
        }
        let m = (a + b + c) / 3.0;
        covered[(m.x * 4.0).clamp(0.0, 3.99) as usize][(m.z * 4.0).clamp(0.0, 3.99) as usize] = true;
    }
    let cover = covered.iter().flatten().filter(|c| **c).count();
    if area < 0.7 || cover < 10 || piece.tris.len() > 90 {
        return None;
    }
    touch[4] = false;
    touch[5] = false;
    Some((touch, cover as f32 + area.min(2.0) - piece.tris.len() as f32 / 180.0))
}

/// The brightness gain that brings a metre's colours to the texture as it
/// is on average (the levels' light is in them, and coloured).
fn gain_of(piece: &jak_pieces::Piece) -> f32 {
    let luma: f32 = piece.tris.iter().flat_map(|t| t.color.iter()).map(|c| 0.299 * f32::from(c[0]) + 0.587 * f32::from(c[1]) + 0.114 * f32::from(c[2])).sum::<f32>()
        / (piece.tris.len() * 3).max(1) as f32;
    if luma < 24.0 { 0.0 } else { (128.0 / luma).clamp(0.5, 4.0) }
}

fn greyed(t: &PieceTri, gain: f32) -> PieceTri {
    let mut moved = *t;
    for c in &mut moved.color {
        let l = 0.299 * f32::from(c[0]) + 0.587 * f32::from(c[1]) + 0.114 * f32::from(c[2]);
        let grey = if gain == 0.0 { 128 } else { (l * gain).round().clamp(0.0, 255.0) as u8 };
        c[0] = grey;
        c[1] = grey;
        c[2] = grey;
    }
    moved
}

/// A flat face of texture `icon` on the block's face toward `d` (x, z),
/// looking out that way.
fn face_quad(d: (i8, i8), icon: u16) -> [PieceTri; 2] {
    let q: [[f32; 3]; 4] = match d {
        (0, 1) => [[1.0, 1.0, 1.0], [0.0, 1.0, 1.0], [0.0, 0.0, 1.0], [1.0, 0.0, 1.0]],
        (0, _) => [[0.0, 1.0, 0.0], [1.0, 1.0, 0.0], [1.0, 0.0, 0.0], [0.0, 0.0, 0.0]],
        (1, _) => [[1.0, 1.0, 0.0], [1.0, 1.0, 1.0], [1.0, 0.0, 1.0], [1.0, 0.0, 0.0]],
        _ => [[0.0, 1.0, 1.0], [0.0, 1.0, 0.0], [0.0, 0.0, 0.0], [0.0, 0.0, 1.0]],
    };
    let uv = [[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]];
    let c = [[110, 110, 110, 255]; 3];
    [
        PieceTri { p: [q[0], q[1], q[2]], uv: [uv[0], uv[1], uv[2]], color: c, texture: icon, blend: false },
        PieceTri { p: [q[0], q[2], q[3]], uv: [uv[0], uv[2], uv[3]], color: c, texture: icon, blend: false },
    ]
}

/// A wall metre with its facade kept and its back slid to the far face of
/// the block and closed with a flat face of the wall's texture: the faces
/// looking across the wall say which way the facade looks (most of them
/// look out of the building); for a run along x looking -z the metre's
/// deepest point goes to z 1 and a face at z 1 looks +z. A corner is slid
/// both ways. Lit to the texture as it is on average, in grey.
fn backed(piece: &jak_pieces::Piece, joins: Joins, icon: u16) -> Tile {
    let (mut fx, mut fz) = (0.0f32, 0.0f32);
    let (mut lo, mut hi) = ([f32::MAX; 3], [f32::MIN; 3]);
    for t in &piece.tris {
        let [a, b, c] = t.p.map(Vec3::from_array);
        let n = (b - a).cross(c - a);
        let size = n.length() * 0.5;
        let n = n.normalize_or_zero();
        if n.x.abs() > 0.7 {
            fx += n.x * size;
        } else if n.z.abs() > 0.7 {
            fz += n.z * size;
        }
        for p in t.p {
            for k in 0..3 {
                lo[k] = lo[k].min(p[k]);
                hi[k] = hi[k].max(p[k]);
            }
        }
    }
    let along_x = joins[1] || joins[3];
    let along_z = joins[0] || joins[2];
    // The facade looks where most of its across faces look; a run along x
    // has its facade across z, a run along z across x, a corner both.
    let front_z: i8 = if (along_x || !along_z) && fz.abs() > 0.05 { if fz < 0.0 { -1 } else { 1 } } else { 0 };
    let front_x: i8 = if (along_z || !along_x) && fx.abs() > 0.05 { if fx < 0.0 { -1 } else { 1 } } else { 0 };
    // Slid: the back to the far face (a centred wall where it has no
    // facade that way).
    let dz = match front_z {
        -1 => 1.0 - hi[2],
        1 => -lo[2],
        _ => {
            if along_x {
                0.5 - (lo[2] + hi[2]) * 0.5
            } else {
                0.0
            }
        }
    };
    let dx = match front_x {
        -1 => 1.0 - hi[0],
        1 => -lo[0],
        _ => {
            if along_z {
                0.5 - (lo[0] + hi[0]) * 0.5
            } else {
                0.0
            }
        }
    };
    let gain = gain_of(piece);
    let mut tris: Vec<PieceTri> = piece
        .tris
        .iter()
        .map(|t| {
            let mut moved = greyed(t, gain);
            for p in &mut moved.p {
                p[0] = (p[0] + dx).clamp(0.0, 1.0);
                p[2] = (p[2] + dz).clamp(0.0, 1.0);
            }
            moved
        })
        .collect();
    // The back closed.
    if front_z != 0 {
        tris.extend(face_quad((0, -front_z), icon));
    }
    if front_x != 0 {
        tris.extend(face_quad((-front_x, 0), icon));
    }
    Tile { tris, front: (front_x, front_z) }
}

/// A roof metre with its top at the block's top, lit in grey.
fn topped(piece: &jak_pieces::Piece) -> Tile {
    let top = piece.tris.iter().flat_map(|t| t.p.iter()).map(|p| p[1]).fold(f32::MIN, f32::max);
    let dy = 1.0 - top;
    let gain = gain_of(piece);
    let tris = piece
        .tris
        .iter()
        .map(|t| {
            let mut moved = greyed(t, gain);
            for p in &mut moved.p {
                p[1] = (p[1] + dy).clamp(0.0, 1.0);
            }
            moved
        })
        .collect();
    Tile { tris, front: (0, 0) }
}

/// What drawing a wall block needs of a block beside it.
#[derive(Clone, Copy)]
pub struct Beside {
    /// The wall or roof it is, if one.
    pub wall: Option<usize>,
    /// Whether it is a whole block (a wall goes on into it).
    pub opaque: bool,
}

/// Which way a wall block's open side is (its outside): the way whose
/// column two blocks off has nothing solid from the block's height to four
/// over it (inside a house the roof is there; outside, past the eaves, the
/// sky). Both or neither open: toward -z or -x.
fn outside(pos: (i32, i32, i32), along_x: bool, beside: &impl Fn((i32, i32, i32)) -> Beside) -> (i8, i8) {
    let (x, y, z) = pos;
    let open = |d: (i32, i32)| (0..=4).all(|k| !beside((x + 2 * d.0, y + k, z + 2 * d.1)).opaque);
    if along_x {
        if open((0, -1)) || !open((0, 1)) {
            (0, -1)
        } else {
            (0, 1)
        }
    } else if open((-1, 0)) || !open((1, 0)) {
        (-1, 0)
    } else {
        (1, 0)
    }
}

/// Draws wall or roof block `wall` at `pos`: a wall joined as its state
/// says (`sides`: north, east, south, west) and as the blocks over and
/// under it are, its facade to its open side; a roof joined to the roof
/// blocks beside it, closed under and at its open sides.
pub(crate) fn append(
    pos: (i32, i32, i32),
    wall: usize,
    sides: [bool; 4],
    beside: impl Fn((i32, i32, i32)) -> Beside,
    atlas: &crate::mesh::Atlas,
    light: &crate::lighting::SkyLight,
    mesh: &mut crate::mesh::ChunkMesh,
    transparent_indices: &mut Vec<u32>,
) {
    let Some(place) = place_of(wall) else { return };
    let kind = &WALLS[wall];
    let (x, y, z) = pos;
    let same = |d: (i32, i32, i32)| beside((x + d.0, y + d.1, z + d.2)).wall == Some(wall);
    let joins: Joins = if kind.roof {
        [same((0, 0, -1)), same((1, 0, 0)), same((0, 0, 1)), same((-1, 0, 0)), false, false]
    } else {
        let vertical = |d: i32| {
            let b = beside((x, y + d, z));
            b.opaque || b.wall.is_some_and(|w| place_of(w) == Some(place) && !WALLS[w].roof)
        };
        [sides[0], sides[1], sides[2], sides[3], vertical(1), vertical(-1)]
    };
    let Some(library) = library(wall) else { return };
    let Some(fits) = library.best_fit(joins) else { return };
    // The best metre of the pattern, the same every block, turned so its
    // facade looks to the open side.
    let mut pick = fits[0];
    if !kind.roof {
        let along_x = joins[1] || joins[3];
        let want = outside(pos, along_x, &beside);
        if let Some(&found) = fits.iter().find(|&&(tile, turns)| {
            let mut f = library.tiles[tile as usize].front;
            for _ in 0..turns {
                f = turn_dir(f);
            }
            if along_x {
                f.1 == want.1 && want.1 != 0
            } else {
                f.0 == want.0 && want.0 != 0
            }
        }) {
            pick = found;
        }
    }
    let (tile, turns) = pick;
    let regions = jak_pieces::regions(place, atlas);
    let base = [(x - mesh.origin[0]) as f32, (y - mesh.origin[1]) as f32, (z - mesh.origin[2]) as f32];
    let around = [(0, 0, 0), (0, 1, 0), (0, -1, 0), (1, 0, 0), (-1, 0, 0), (0, 0, 1), (0, 0, -1)];
    let sky = around.iter().map(|d| light.get((x + d.0, y + d.1, z + d.2))).max().unwrap_or(15) as f32;
    let block = around.iter().map(|d| light.get_block((x + d.0, y + d.1, z + d.2))).max().unwrap_or(0) as f32;
    let mut push = |t: &PieceTri, turns: u8| {
        let Some(&[u0, v0, u1, v1]) = regions.get(usize::from(t.texture)) else { return };
        let start = mesh.vertices.len() as u32;
        for k in 0..3 {
            let mut p = t.p[k];
            for _ in 0..turns {
                p = turn_point(p);
            }
            let [u, v] = t.uv[k];
            let c = t.color[k].map(|c| (f32::from(c) / 128.0).min(1.0));
            mesh.vertices.push(crate::mesh::Vertex {
                position: [base[0] + p[0], base[1] + p[1], base[2] + p[2]],
                uv: [u0 + (u1 - u0) * u, v0 + (v1 - v0) * v],
                color: [c[0], c[1], c[2], if t.blend { c[3] } else { 1.0 }],
                sky_light: sky,
                block_light: block,
            });
        }
        let indices = if t.blend { &mut *transparent_indices } else { &mut mesh.indices };
        indices.extend_from_slice(&[start, start + 1, start + 2, start, start + 2, start + 1]);
        mesh.faces += 2;
    };
    for t in &library.tiles[tile as usize].tris {
        push(t, turns);
    }
    if kind.roof {
        // Closed underneath and at its open sides, in its texture.
        let icon = original_places::textures_of(&PLACES[place]).iter().position(|t| *t == kind.icon).unwrap_or(0) as u16;
        if !beside((x, y - 1, z)).opaque {
            let c = [[100, 100, 100, 255]; 3];
            let q = [[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [1.0, 0.0, 1.0], [0.0, 0.0, 1.0]];
            let uv = [[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]];
            push(&PieceTri { p: [q[0], q[1], q[2]], uv: [uv[0], uv[1], uv[2]], color: c, texture: icon, blend: false }, 0);
            push(&PieceTri { p: [q[0], q[2], q[3]], uv: [uv[0], uv[2], uv[3]], color: c, texture: icon, blend: false }, 0);
        }
        for (d, j) in [((0i8, -1i8), 0usize), ((1, 0), 1), ((0, 1), 2), ((-1, 0), 3)] {
            let b = beside((x + i32::from(d.0), y, z + i32::from(d.1)));
            if joins[j] || b.opaque {
                continue;
            }
            for t in face_quad(d, icon) {
                push(&t, 0);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every wall's block is named for its place, and Spargus's stucco
    /// walls are mined from its level: a straight run (joined east and
    /// west), its corner and its end, each fitting turned, every metre's
    /// triangles inside the block, a run's facade on one side with its
    /// back closed; and its roof has lying metres.
    #[test]
    fn spargus_walls_join_as_the_level_has_them() {
        for w in &WALLS {
            assert!(original_places::named(w.place).is_some(), "{}", w.block);
            assert_eq!(wall_of(w.block).map(|k| WALLS[k].block), Some(w.block));
        }
        let wall = wall_of("muro_spargus_estuco").unwrap();
        let Some(library) = library(wall) else {
            println!("no export, skipped");
            return;
        };
        let mut patterns: Vec<(Joins, usize)> = library.by_joins.iter().map(|(j, v)| (*j, v.len())).collect();
        patterns.sort();
        println!("{} tiles, {} patterns: {patterns:?}", library.tile_count(), library.by_joins.len());
        let run = [false, true, false, true, true, true];
        assert!(library.fits(run).is_some(), "a straight run");
        assert!(library.fits(turned(run)).is_some(), "the run turned");
        assert!(library.best_fit([false, true, true, false, true, true]).is_some(), "a corner");
        assert!(library.best_fit([false, true, false, false, true, true]).is_some(), "an end");
        for (joins, fits) in &library.by_joins {
            if joins[..4] != [false, true, false, true] {
                continue;
            }
            for &(tile, turns) in fits {
                if turns % 2 != 0 {
                    continue;
                }
                let t = &library.tiles[tile as usize];
                assert!(t.front.1 != 0, "{joins:?}: a run's facade looks across it");
                for tri in &t.tris {
                    for p in tri.p {
                        assert!(p.iter().all(|c| (0.0..=1.0).contains(c)), "{joins:?}");
                    }
                }
                // Its back at the far face: a face there, looking out.
                let back = if t.front.1 < 0 { 1.0 } else { 0.0 };
                assert!(t.tris.iter().any(|tri| tri.p.iter().all(|p| (p[2] - back).abs() < 1e-3)), "{joins:?}: closed at the back");
            }
        }
        let roof = wall_of("tejado_spargus").unwrap();
        let Some(roofs) = super::library(roof) else { return };
        println!("roof: {} tiles, {} patterns", roofs.tile_count(), roofs.by_joins.len());
        assert!(roofs.best_fit([true, true, true, true, false, false]).is_some(), "a roof's middle");
    }
}

#[cfg(test)]mod systems31_wall_tests{
 #[test]fn systems31_industrial_wall_panels_are_closed_inside_the_grid(){
  use super::*;let i=wall_of("muro_haven_chapa").unwrap();let lib=library(i).unwrap();
  let mut edges:HashMap<([i32;3],[i32;3]),usize>=HashMap::new();
  for t in &lib.tiles[0].tris{
   for p in t.p{assert!(p.iter().all(|v|*v>=0.&&*v<=1.));}
   let p=t.p.map(|p|p.map(|v|(v*1000.).round()as i32));
   for (a,b)in[(p[0],p[1]),(p[1],p[2]),(p[2],p[0])]{*edges.entry(if a<b{(a,b)}else{(b,a)}).or_default()+=1;}
  }
  assert!(edges.values().all(|n|*n==2),"each panel has a watertight perimeter");
  for bits in 0..64 {assert!(lib.fits(std::array::from_fn(|i|bits&(1<<i)!=0)).is_some());}
 }
}
