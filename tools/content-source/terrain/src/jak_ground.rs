//! Jak's ground as blocks that join by themselves (Adrii, 23/10: "que estos
//! cubos del suelo, la capa superficial de suelo al juntarse debe crear
//! patrones diferentes o caminos", as stairs take their shape from the
//! blocks round them).
//!
//! Each place has four grounds (Sandover's sand, path, grass and rock, …),
//! each a block. A ground block's top is a metre of its place's own floor:
//! the one whose middle wears its ground and whose four edges wear what the
//! ground blocks beside it are (the path's edge into sand where sand lies
//! beside it, its bend where the path turns), found in the level itself. A
//! metre of floor is "of" a ground where most of its quarter-metre samples'
//! top texture is that ground's (the levels' blended layers, a path laid
//! over sand, over what they lie on). Every such metre of the level that is
//! level and whole is a tile; each fits its edges turned four ways. Its
//! floor is laid flat on the block's top; its colour is its ground's
//! average one (the levels' baked light would patch the tiles), its blended
//! layers keep their vertex alpha.
use std::collections::HashMap;
use std::sync::OnceLock;

use glam::Vec3;

use crate::jak_items::{BlockLook, JakBlock, JakItem, JakKind};
use crate::jak_pieces::{self, PieceTri};
use crate::original_places::{self, PLACES};

/// One of a place's grounds.
#[derive(Clone, Copy, Debug)]
pub struct Ground {
    pub place: &'static str,
    /// Its block (`jak:<block>`).
    pub block: &'static str,
    pub texture: &'static str,
    pub title: &'static str,
}

const fn g(place: &'static str, block: &'static str, texture: &'static str, title: &'static str) -> Ground {
    Ground { place, block, texture, title }
}

/// Every place's five grounds: its floors' textures with the most area
/// (and Sandover's path and Snowy Mountain's, which are few but are
/// paths), and its roof (the houses of blocks' flat roofs, `jak_hamlets`).
pub const GROUNDS: [Ground; 25] = [
    g("spargus", "suelo_spargus_tierra", "wascity-ground-01", "Suelo de Spargus: tierra"),
    g("spargus", "suelo_spargus_zanja", "wascity-ground-2-ditch-04", "Suelo de Spargus: zanja"),
    g("spargus", "suelo_spargus_piedra", "wascitya-stone-top", "Suelo de Spargus: piedra"),
    g("spargus", "suelo_spargus_roca", "wstd-rockwall-01", "Suelo de Spargus: roca"),
    g("spargus", "suelo_spargus_tejado", "wascity-roof-1", "Tejado de Spargus"),
    g("haven", "suelo_haven_tierra", "slum-ground-01", "Suelo de Haven: tierra"),
    g("haven", "suelo_haven_zanja", "city-slum-ground-2-ditch-03", "Suelo de Haven: zanja"),
    g("haven", "suelo_haven_fondo", "slum-ditch-bottom-01", "Suelo de Haven: fondo de zanja"),
    g("haven", "suelo_haven_tablas", "city-dirtywood", "Suelo de Haven: tablas"),
    g("haven", "suelo_haven_tejado", "city-slum-roof", "Tejado de Haven"),
    g("villarrefugio", "suelo_villarrefugio_arena", "vil-beach-01", "Suelo de Villarrefugio: arena"),
    g("villarrefugio", "suelo_villarrefugio_camino", "vil-beach-01path", "Suelo de Villarrefugio: camino"),
    g("villarrefugio", "suelo_villarrefugio_hierba", "vil1-jng-leafyground", "Suelo de Villarrefugio: hierba"),
    g("villarrefugio", "suelo_villarrefugio_roca", "vil-beachrock", "Suelo de Villarrefugio: roca"),
    g("villarrefugio", "suelo_villarrefugio_tejado", "vil-hut-roof-tile-01", "Tejado de Villarrefugio"),
    g("montana", "suelo_montana_nieve", "snow", "Suelo de la Montaña Nevada: nieve"),
    g("montana", "suelo_montana_camino", "snow-path", "Suelo de la Montaña Nevada: camino"),
    g("montana", "suelo_montana_roca", "snow-rock-hi", "Suelo de la Montaña Nevada: roca"),
    g("montana", "suelo_montana_hielo", "snow-ice", "Suelo de la Montaña Nevada: hielo"),
    g("montana", "suelo_montana_tejado", "snow-metalroof-01", "Tejado de la Montaña Nevada"),
    g("pantano", "suelo_pantano_hojas", "swp-leafyground", "Suelo del Pantano: hojarasca"),
    g("pantano", "suelo_pantano_barro", "swp-mud-floor", "Suelo del Pantano: barro"),
    g("pantano", "suelo_pantano_alquitran", "swp-tar", "Suelo del Pantano: alquitrán"),
    g("pantano", "suelo_pantano_roca", "swp-beachrock", "Suelo del Pantano: roca"),
    g("pantano", "suelo_pantano_tejado", "swp-roof-tile-01", "Tejado del Pantano"),
];

/// How many grounds a place has.
pub const PER_PLACE: usize = 5;

const fn look(i: usize) -> JakBlock {
    JakBlock { path: GROUNDS[i].block, look: BlockLook::All(GROUNDS[i].texture) }
}

const fn item(i: usize) -> JakItem {
    JakItem { path: GROUNDS[i].block, title: GROUNDS[i].title, kind: JakKind::Block }
}

/// The ground blocks' looks (their item and far look: a cube of their
/// texture; the world draws their tiles).
pub const GROUND_BLOCKS: [JakBlock; 25] = [
    look(0), look(1), look(2), look(3), look(4), look(5), look(6), look(7), look(8), look(9),
    look(10), look(11), look(12), look(13), look(14), look(15), look(16), look(17), look(18), look(19),
    look(20), look(21), look(22), look(23), look(24),
];

pub const GROUND_ITEMS: [JakItem; 25] = [
    item(0), item(1), item(2), item(3), item(4), item(5), item(6), item(7), item(8), item(9),
    item(10), item(11), item(12), item(13), item(14), item(15), item(16), item(17), item(18), item(19),
    item(20), item(21), item(22), item(23), item(24),
];

/// Which ground a block is, by its name (`suelo_<place>_<ground>`).
pub fn ground_of(path: &str) -> Option<usize> {
    if !path.starts_with("suelo_") {
        return None;
    }
    GROUNDS.iter().position(|g| g.block == path)
}

/// A ground's place (`PLACES`) and its own number among that place's four.
fn place_and_ground(ground: usize) -> Option<(usize, u8)> {
    let g = GROUNDS.get(ground)?;
    let place = PLACES.iter().position(|p| p.name == g.place)?;
    let own = GROUNDS[..ground].iter().filter(|o| o.place == g.place).count() as u8;
    Some((place, own))
}

/// Edges: north (-z), east (+x), south (+z), west (-x).
const SIDES: [(i32, i32); 4] = [(0, -1), (1, 0), (0, 1), (-1, 0)];

/// A tile's middle and four edges' grounds (own numbers).
type Signature = [u8; 5];

/// A metre of floor laid flat on a block's top (its corners in the block).
struct Tile {
    tris: Vec<PieceTri>,
}

/// A place's tiles, by the signature they fit (turned `0..4` quarter turns
/// clockwise seen from above).
pub struct Library {
    tiles: Vec<Tile>,
    by_signature: HashMap<Signature, Vec<(u32, u8)>>,
    /// Each ground's average colour (0..1, 1 the texture as it is).
    colors: Vec<[f32; 3]>,
    /// Each of the place's textures' ground, if one.
    ground_of_texture: Vec<Option<u8>>,
}

impl Library {
    pub fn tile_count(&self) -> usize {
        self.tiles.len()
    }

    pub fn signatures(&self) -> usize {
        self.by_signature.len()
    }

    fn fits(&self, signature: Signature) -> Option<&Vec<(u32, u8)>> {
        self.by_signature.get(&signature).filter(|v| !v.is_empty())
    }

    /// The tiles fitting `signature`, else those fitting it with the fewest
    /// edges taken as the middle's ground (the level's paths are wider than
    /// a metre: a path one block wide gets one of its edges).
    fn best_fit(&self, signature: Signature) -> Option<&Vec<(u32, u8)>> {
        let mut masks: Vec<u8> = (0..16).collect();
        masks.sort_by_key(|m| m.count_ones());
        masks.into_iter().find_map(|mask| {
            let mut s = signature;
            for k in 0..4 {
                if mask & (1 << k) != 0 {
                    s[k + 1] = s[0];
                }
            }
            self.fits(s)
        })
    }
}

/// `signature` turned a quarter turn clockwise: its north edge goes east.
fn turned(s: Signature) -> Signature {
    [s[0], s[4], s[1], s[2], s[3]]
}

/// A point of a tile turned a quarter turn clockwise about its middle.
fn turn_point([x, y, z]: [f32; 3]) -> [f32; 3] {
    [1.0 - z, y, x]
}

/// A place's tiles, made once (none without its levels).
pub fn library(place: usize) -> Option<&'static Library> {
    static LIBRARIES: [OnceLock<Option<Library>>; 5] = [const { OnceLock::new() }; 5];
    LIBRARIES[place]
        .get_or_init(|| {
            let started = std::time::Instant::now();
            let made = make_library(place)?;
            diag::info!(
                World,
                "{} ground: {} tiles, {} edge patterns, in {:.2} s",
                PLACES[place].name,
                made.tiles.len(),
                made.by_signature.len(),
                started.elapsed().as_secs_f32()
            );
            Some(made)
        })
        .as_ref()
}

/// Samples a metre side.
const SAMPLES: i32 = 4;

fn make_library(place: usize) -> Option<Library> {
    let source = jak_pieces::source(place)?;
    let name = PLACES[place].name;
    let own: Vec<&Ground> = GROUNDS.iter().filter(|g| g.place == name).collect();
    let ground_of_texture: Vec<Option<u8>> = original_places::textures_of(&PLACES[place])
        .iter()
        .map(|t| own.iter().position(|g| g.texture == *t).map(|i| i as u8))
        .collect();
    let level = |t: &jak_pieces::SourceTri| {
        let n = (t.p[1] - t.p[0]).cross(t.p[2] - t.p[0]).normalize_or_zero();
        n.y.abs() > 0.9
    };
    let ground = |t: &jak_pieces::SourceTri| ground_of_texture.get(usize::from(t.texture)).copied().flatten();
    // Each ground's colour: the area-weighted mean of its lit half (the
    // levels' colours carry their shadows: Sandover's beach as a whole
    // came out near black).
    let mut shades: Vec<Vec<(f32, [f32; 3], f32)>> = vec![Vec::new(); PER_PLACE];
    for t in &source.tris {
        let Some(m) = ground(t).filter(|_| level(t) && !t.blend) else { continue };
        let area = (t.p[1] - t.p[0]).cross(t.p[2] - t.p[0]).length() * 0.5;
        let mean: [f32; 3] = std::array::from_fn(|c| (f32::from(t.color[0][c]) + f32::from(t.color[1][c]) + f32::from(t.color[2][c])) / 3.0);
        let luma = 0.299 * mean[0] + 0.587 * mean[1] + 0.114 * mean[2];
        shades[usize::from(m)].push((luma, mean, area));
    }
    let colors: Vec<[f32; 3]> = (0..PER_PLACE).map(|m| {
        let list = &mut shades[m];
        list.sort_by(|a, b| b.0.total_cmp(&a.0));
        let total: f32 = list.iter().map(|s| s.2).sum();
        let (mut sum, mut area) = ([0.0f32; 3], 0.0f32);
        for &(_, c, a) in list.iter() {
            if area >= total * 0.5 {
                break;
            }
            for k in 0..3 {
                sum[k] += c[k] * a;
            }
            area += a;
        }
        if area <= 0.0 {
            return [1.0; 3];
        }
        // Its brightness only: the levels' light is coloured (Snowy
        // Mountain's snow lit purple), which beside Minecraft's own blocks
        // read as a purple ground (23/10, even at half its tint).
        let c = sum.map(|c| (c / area / 128.0).min(1.0));
        let luma = 0.299 * c[0] + 0.587 * c[1] + 0.114 * c[2];
        [luma; 3]
    }).collect();
    // Candidates per signature: (flatness, x, y, z).
    let mut found: HashMap<Signature, Vec<(f32, i32, i32, i32)>> = HashMap::new();
    let side = (jak_pieces::TILE * SAMPLES) as usize;
    for (&(tx, tz), list) in &source.tiles {
        // The tile's samples: the top floor of its grounds, and its ground
        // (a blended layer over what it lies on).
        let mut top: Vec<Option<(f32, u8)>> = vec![None; side * side];
        let (x0, z0) = ((tx * jak_pieces::TILE) as f32, (tz * jak_pieces::TILE) as f32);
        for &i in list {
            let t = &source.tris[i as usize];
            let Some(m) = ground(t).filter(|_| level(t)) else { continue };
            if t.blend && t.color.iter().map(|c| u32::from(c[3])).sum::<u32>() < 3 * 64 {
                continue;
            }
            let [a, b, c] = t.p;
            let longest = (b - a).length().max((c - b).length()).max((a - c).length());
            let n = ((longest * SAMPLES as f32 * 1.5).ceil() as usize).clamp(1, 600);
            for i in 0..=n {
                for j in 0..=n - i {
                    let p = a + (b - a) * (i as f32 / n as f32) + (c - a) * (j as f32 / n as f32);
                    let (sx, sz) = (((p.x - x0) * SAMPLES as f32).floor(), ((p.z - z0) * SAMPLES as f32).floor());
                    if sx < 0.0 || sz < 0.0 || sx >= side as f32 || sz >= side as f32 {
                        continue;
                    }
                    let cell = &mut top[sz as usize * side + sx as usize];
                    *cell = match *cell {
                        None => Some((p.y, m)),
                        Some((y, _)) if p.y > y + 0.05 => Some((p.y, m)),
                        Some((y, _)) if t.blend && (p.y - y).abs() <= 0.05 => Some((y.max(p.y), m)),
                        keep => keep,
                    };
                }
            }
        }
        // Each whole, level metre of it.
        for cz in 0..jak_pieces::TILE {
            for cx in 0..jak_pieces::TILE {
                let at = |sx: i32, sz: i32| top[((cz * SAMPLES + sz) as usize) * side + (cx * SAMPLES + sx) as usize];
                let mut ys = Vec::with_capacity(16);
                let mut whole = true;
                for sz in 0..SAMPLES {
                    for sx in 0..SAMPLES {
                        match at(sx, sz) {
                            Some((y, _)) => ys.push(y),
                            None => whole = false,
                        }
                    }
                }
                if !whole {
                    continue;
                }
                let (lo, hi) = ys.iter().fold((f32::MAX, f32::MIN), |(a, b), &y| (a.min(y), b.max(y)));
                if hi - lo > 0.4 {
                    continue;
                }
                let most = |points: &[(i32, i32)]| -> u8 {
                    let mut count = vec![0u8; PER_PLACE];
                    for &(sx, sz) in points {
                        if let Some((_, m)) = at(sx, sz) {
                            count[usize::from(m)] += 1;
                        }
                    }
                    (0..PER_PLACE as u8).max_by_key(|&m| (count[usize::from(m)], u8::from(m == 0))).unwrap_or(0)
                };
                let middle = most(&[(1, 1), (2, 1), (1, 2), (2, 2)]);
                let edge = |k: usize| -> u8 {
                    let row: Vec<(i32, i32)> = (0..SAMPLES)
                        .map(|i| match k {
                            0 => (i, 0),
                            1 => (SAMPLES - 1, i),
                            2 => (i, SAMPLES - 1),
                            _ => (0, i),
                        })
                        .collect();
                    most(&row)
                };
                let signature = [middle, edge(0), edge(1), edge(2), edge(3)];
                let (wx, wz) = (tx * jak_pieces::TILE + cx, tz * jak_pieces::TILE + cz);
                let list = found.entry(signature).or_default();
                list.push((hi - lo, wx, lo.floor() as i32, wz));
                if list.len() > 12 {
                    list.sort_by(|a, b| a.0.total_cmp(&b.0));
                    list.truncate(6);
                }
            }
        }
    }
    // The flattest few of each signature, cut out of the level.
    let mut tiles = Vec::new();
    let mut by_signature: HashMap<Signature, Vec<(u32, u8)>> = HashMap::new();
    for (signature, mut list) in found {
        list.sort_by(|a, b| a.0.total_cmp(&b.0));
        for &(_, x, y, z) in list.iter().take(3) {
            let Some(tile) = cut_tile(source, &ground_of_texture, x, y, z) else { continue };
            let index = tiles.len() as u32;
            tiles.push(tile);
            let mut s = signature;
            for turn in 0..4u8 {
                by_signature.entry(s).or_default().push((index, turn));
                s = turned(s);
            }
        }
    }
    Some(Library { tiles, by_signature, colors, ground_of_texture })
}

/// The floor of the place's grounds in metre `(x, z)` near height `y`,
/// flat on the top of a block: none unless it covers the metre.
fn cut_tile(source: &jak_pieces::Source, ground_of_texture: &[Option<u8>], x: i32, y: i32, z: i32) -> Option<Tile> {
    let cut = jak_pieces::cut_area(source, (x, x), (z, z));
    let mut tris = Vec::new();
    let mut covered = 0.0f32;
    for dy in -1..=1 {
        let Some(piece) = cut.get(&(x, y + dy, z)) else { continue };
        for t in &piece.tris {
            if ground_of_texture.get(usize::from(t.texture)).copied().flatten().is_none() {
                continue;
            }
            let [a, b, c] = t.p.map(Vec3::from_array);
            let n = (b - a).cross(c - a);
            if n.normalize_or_zero().y.abs() <= 0.9 {
                continue;
            }
            // Laid flat on the top, a blended layer a hair over the rest.
            let lift = if t.blend { 1.003 } else { 1.0 };
            let mut flat = *t;
            for k in 0..3 {
                flat.p[k][1] = lift;
            }
            if !t.blend {
                covered += n.length() * 0.5;
            }
            tris.push(flat);
        }
    }
    (covered >= 0.9).then_some(Tile { tris })
}

/// What drawing a ground block needs of a block beside it: its ground, if
/// it is one, and whether it hides the face against it.
#[derive(Clone, Copy)]
pub(crate) struct Beside {
    pub ground: Option<usize>,
    pub opaque: bool,
}

/// Draws ground block `ground` at `pos`: its tile on top, by the grounds
/// beside it, and its texture on every face open to the air.
pub(crate) fn append(
    pos: (i32, i32, i32),
    ground: usize,
    beside: impl Fn((i32, i32, i32)) -> Beside,
    atlas: &crate::mesh::Atlas,
    light: &crate::lighting::SkyLight,
    mesh: &mut crate::mesh::ChunkMesh,
    transparent_indices: &mut Vec<u32>,
) {
    let Some((place, own)) = place_and_ground(ground) else { return };
    let (x, y, z) = pos;
    let base = [(x - mesh.origin[0]) as f32, (y - mesh.origin[1]) as f32, (z - mesh.origin[2]) as f32];
    let region_of = |texture: &str| {
        crate::pack::ResourceId::parse(&format!("jak:block/{}", texture.to_lowercase()))
            .map(|id| atlas.region(&id))
            .unwrap_or([0.0, 0.0, 1.0, 1.0])
    };
    let library = library(place);
    let color = library.map_or([1.0; 3], |l| l.colors[usize::from(own)]);
    let lit = |at: (i32, i32, i32)| (light.get(at) as f32, light.get_block(at) as f32);
    let mut push = |corners: [[f32; 3]; 3], uvs: [[f32; 2]; 3], rgba: [[f32; 4]; 3], at: (i32, i32, i32), blend: bool| {
        let (sky, block) = lit(at);
        let start = mesh.vertices.len() as u32;
        for k in 0..3 {
            mesh.vertices.push(crate::mesh::Vertex {
                position: [base[0] + corners[k][0], base[1] + corners[k][1], base[2] + corners[k][2]],
                uv: uvs[k],
                color: rgba[k],
                sky_light: sky,
                block_light: block,
            });
        }
        let indices = if blend { &mut *transparent_indices } else { &mut mesh.indices };
        indices.extend_from_slice(&[start, start + 1, start + 2, start, start + 2, start + 1]);
        mesh.faces += 1;
    };
    let above = (x, y + 1, z);
    let own_region = region_of(GROUNDS[ground].texture);
    // The top: the tile that fits, else a plain one, else its texture.
    if !beside(above).opaque {
        let signature: Signature = std::array::from_fn(|k| {
            if k == 0 {
                return own;
            }
            let (dx, dz) = SIDES[k - 1];
            beside((x + dx, y, z + dz))
                .ground
                .and_then(place_and_ground)
                .filter(|&(p, _)| p == place)
                .map_or(own, |(_, m)| m)
        });
        let pick = library.and_then(|l| l.best_fit(signature).map(|fits| (l, fits)));
        match pick {
            Some((l, fits)) => {
                let h = (x as u32).wrapping_mul(0x9E37_79B1) ^ (z as u32).wrapping_mul(0x85EB_CA77) ^ (y as u32).wrapping_mul(0xC2B2_AE3D);
                let (tile, turns) = fits[(h >> 7) as usize % fits.len()];
                let textures = original_places::textures_of(&PLACES[place]);
                for t in &l.tiles[tile as usize].tris {
                    let Some(name) = textures.get(usize::from(t.texture)) else { continue };
                    let [u0, v0, u1, v1] = region_of(name);
                    let ground_color = l.ground_of_texture[usize::from(t.texture)].map_or(color, |m| l.colors[usize::from(m)]);
                    let corners = t.p.map(|mut p| {
                        for _ in 0..turns {
                            p = turn_point(p);
                        }
                        p
                    });
                    let uvs = t.uv.map(|[u, v]| [u0 + (u1 - u0) * u, v0 + (v1 - v0) * v]);
                    let rgba = t.color.map(|c| [ground_color[0], ground_color[1], ground_color[2], if t.blend { (f32::from(c[3]) / 128.0).min(1.0) } else { 1.0 }]);
                    push(corners, uvs, rgba, above, t.blend);
                }
            }
            None => {
                let [u0, v0, u1, v1] = own_region;
                let rgba = [[color[0], color[1], color[2], 1.0]; 3];
                push([[0.0, 1.0, 0.0], [1.0, 1.0, 0.0], [1.0, 1.0, 1.0]], [[u0, v0], [u1, v0], [u1, v1]], rgba, above, false);
                push([[0.0, 1.0, 0.0], [1.0, 1.0, 1.0], [0.0, 1.0, 1.0]], [[u0, v0], [u1, v1], [u0, v1]], rgba, above, false);
            }
        }
    }
    // The sides and the bottom open to the air: its own texture, shaded as
    // Minecraft's faces are.
    let [u0, v0, u1, v1] = own_region;
    let faces: [((i32, i32, i32), [[f32; 3]; 4], f32); 5] = [
        ((0, 0, -1), [[0.0, 1.0, 0.0], [1.0, 1.0, 0.0], [1.0, 0.0, 0.0], [0.0, 0.0, 0.0]], 0.8),
        ((1, 0, 0), [[1.0, 1.0, 0.0], [1.0, 1.0, 1.0], [1.0, 0.0, 1.0], [1.0, 0.0, 0.0]], 0.6),
        ((0, 0, 1), [[1.0, 1.0, 1.0], [0.0, 1.0, 1.0], [0.0, 0.0, 1.0], [1.0, 0.0, 1.0]], 0.8),
        ((-1, 0, 0), [[0.0, 1.0, 1.0], [0.0, 1.0, 0.0], [0.0, 0.0, 0.0], [0.0, 0.0, 1.0]], 0.6),
        ((0, -1, 0), [[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [1.0, 0.0, 1.0], [0.0, 0.0, 1.0]], 0.5),
    ];
    for (d, quad, shade) in faces {
        let at = (x + d.0, y + d.1, z + d.2);
        if beside(at).opaque {
            continue;
        }
        let rgba = [[color[0] * shade, color[1] * shade, color[2] * shade, 1.0]; 3];
        let uv = [[u0, v0], [u1, v0], [u1, v1], [u0, v1]];
        push([quad[0], quad[1], quad[2]], [uv[0], uv[1], uv[2]], rgba, at, false);
        push([quad[0], quad[2], quad[3]], [uv[0], uv[2], uv[3]], rgba, at, false);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every ground's texture is one of its place's, and Sandover's tiles
    /// are mined from its level: its sand and grass plain, and its path
    /// with its edges into the sand (a straight path and its end), each
    /// fitting turned.
    #[test]
    fn sandover_ground_tiles_are_its_own() {
        for g in &GROUNDS {
            let place = original_places::named(g.place).expect(g.place);
            assert!(original_places::textures_of(place).contains(&g.texture), "{}", g.block);
            assert!(ground_of(g.block).is_some());
        }
        let place = PLACES.iter().position(|p| p.name == "villarrefugio").unwrap();
        let Some(library) = library(place) else {
            println!("no export, skipped");
            return;
        };
        let names = ["arena", "camino", "hierba", "roca", "tejado"];
        let mut plain = Vec::new();
        for m in 0..PER_PLACE as u8 {
            plain.push((names[usize::from(m)], library.fits([m; 5]).map_or(0, Vec::len)));
        }
        let mut edges: Vec<(Signature, usize)> = library.by_signature.iter().map(|(s, v)| (*s, v.len())).filter(|(s, _)| s[0] == 1).collect();
        edges.sort();
        println!("{} tiles, {} patterns; plain {plain:?}; path patterns {edges:?}; colours {:?}", library.tile_count(), library.signatures(), library.colors);
        // Lit, not the shadows' near black.
        assert!(library.colors.iter().take(4).all(|c| c.iter().sum::<f32>() > 0.6), "{:?}", library.colors);
        assert!(library.fits([0; 5]).is_some(), "plain sand");
        assert!(library.fits([2; 5]).is_some(), "plain grass");
        // A path's edge into grass on its east, and turned south; a path one
        // block wide (grass both sides, none such in the level) still gets
        // a grass edge.
        let edge = [1, 1, 2, 1, 1];
        assert!(library.fits(edge).is_some() && library.fits(turned(edge)).is_some(), "a path's edge");
        let narrow = library.best_fit([1, 1, 2, 1, 2]).unwrap();
        let tile = &library.tiles[narrow[0].0 as usize];
        assert!(tile.tris.iter().any(|t| library.ground_of_texture[usize::from(t.texture)] == Some(2)), "the grass of its edge");
    }
}
