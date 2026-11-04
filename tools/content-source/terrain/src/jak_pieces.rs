//! The Jak games' places as pieces (Adrii, 23/10: "que la forma sea la de
//! las cosas de jak", not Minecraft cubes in Jak's textures): each metre of
//! a place's levels is a block of the world that is drawn as the level's own
//! triangles inside that metre, cut out of the level (its textures, its
//! texture coordinates, its baked vertex colours), and that Jak stands on
//! and bumps into by those same triangles.
//!
//! A piece is one block, `jak:pieza_<place>` (a slab to the game's rules:
//! the bottom half for a piece low in its metre, else the top half; never a
//! whole block, inside which the game would smother Jak), the same for every
//! metre: which metre of the level it shows comes from where it stands in
//! the world (the place's site, `original_places::current_sites`), so the
//! world keeps nothing more than for any block.
//!
//! The levels are drawn from both sides (their glTF materials are all
//! double-sided: the games never cull them), so a piece is too: the back of
//! a cliff or of a house front never meant to be seen still shows.
use std::collections::HashMap;
use std::sync::{Arc, Mutex, OnceLock};

use glam::{Vec2, Vec3, Vec4};

use crate::original_places::{self, Place, PLACES};
pub const TOWER_SOURCE: usize = 5;
pub const FOLIAGE_SOURCE: usize = 6;
/// The physical skatea/palace composite has its own immutable source bank.
/// Sources 1 and 5 keep the geometry already carried by saved pieces.
pub const CIRCUIT_SOURCE: usize = 7;
pub const ARENA_SOURCE: usize = 10;
pub const LEGACY_PALACE_SOURCE: usize = 9;
pub const LEGACY_ARENA_SOURCE: usize = 8;
fn texture_place(place: usize) -> Option<usize> {
    match place { TOWER_SOURCE | FOLIAGE_SOURCE | CIRCUIT_SOURCE | ARENA_SOURCE | LEGACY_PALACE_SOURCE | LEGACY_ARENA_SOURCE => Some(1), p if p < PLACES.len() => Some(p), _ => None }
}
fn portable_source(place: usize) -> bool { place < PLACES.len() || matches!(place, TOWER_SOURCE | CIRCUIT_SOURCE | ARENA_SOURCE | LEGACY_PALACE_SOURCE | LEGACY_ARENA_SOURCE) }
fn independent_source(place: usize) -> bool { matches!(place, TOWER_SOURCE | CIRCUIT_SOURCE | ARENA_SOURCE | LEGACY_PALACE_SOURCE | LEGACY_ARENA_SOURCE) }

/// A triangle of a place's level as the level draws it.
#[derive(Clone, Copy)]
pub(crate) struct SourceTri {
    pub p: [Vec3; 3],
    pub uv: [Vec2; 3],
    /// Its vertex colours at the PS2's scale (0.5 the texture as it is).
    pub color: [[u8; 4]; 3],
    pub texture: u16,
    /// Whether its texture repeats across u and v (else it is clamped).
    pub repeat: [bool; 2],
    /// Blended over what is under it by its vertex alpha (its material's
    /// `BLEND`: the levels' grime, shadow and fade layers), not drawn solid.
    pub blend: bool,
}

/// Tiles the source triangles are filed by (metres).
pub(crate) const TILE: i32 = 16;

pub(crate) struct Source {
    pub tris: Vec<SourceTri>,
    pub tiles: HashMap<(i32, i32), Vec<u32>>,
    cell_owners: HashMap<u32,[i32;3]>,
    /// Its inside, worked out once (`enclosed`).
    enclosed: OnceLock<Enclosed>,
}

/// The inside of a place's levels: every metre of its crop that is
/// neither on a triangle nor reachable from the crop's outside through
/// metres with nothing in them (a hill's inside, the ground under a house's
/// floor), found by flooding the air in from the crop's faces, six ways.
/// The ground under a place's floors is filled only there (23/10: filled
/// under every lowest floor, the fill stood as towers under walkways,
/// balconies and stands over open air).
pub struct Enclosed {
    x0: i32,
    y0: i32,
    z0: i32,
    w: usize,
    h: usize,
    d: usize,
    bits: Vec<u64>,
}

impl Enclosed {
    fn index(&self, x: i32, y: i32, z: i32) -> Option<usize> {
        let (i, j, k) = (x - self.x0, y - self.y0, z - self.z0);
        if i < 0 || j < 0 || k < 0 || i as usize >= self.w || j as usize >= self.h || k as usize >= self.d {
            return None;
        }
        Some((j as usize * self.d + k as usize) * self.w + i as usize)
    }

    fn get(bits: &[u64], at: usize) -> bool {
        bits[at >> 6] & (1 << (at & 63)) != 0
    }

    fn set(bits: &mut [u64], at: usize) {
        bits[at >> 6] |= 1 << (at & 63);
    }

    /// Whether metre `(x, y, z)` of the level is inside: under the crop's
    /// bottom it is (the ground), over its top or past its sides it is not.
    pub fn is_enclosed(&self, x: i32, y: i32, z: i32) -> bool {
        if y < self.y0 {
            return x >= self.x0 && z >= self.z0 && (x - self.x0) < self.w as i32 && (z - self.z0) < self.d as i32;
        }
        self.index(x, y, z).is_some_and(|at| Self::get(&self.bits, at))
    }

    fn make(tris: &[SourceTri], crop: [f32; 6]) -> Self {
        let (x0, x1, z0, z1, y0, y1) = (crop[0].floor() as i32, crop[1].ceil() as i32, crop[2].floor() as i32, crop[3].ceil() as i32, crop[4].floor() as i32, crop[5].ceil() as i32);
        let (w, h, d) = ((x1 - x0 + 1) as usize, (y1 - y0 + 1) as usize, (z1 - z0 + 1) as usize);
        let mut out = Self { x0, y0, z0, w, h, d, bits: Vec::new() };
        let words = (w * h * d).div_ceil(64);
        let mut solid = vec![0u64; words];
        // Every metre a triangle passes through: points over it half a
        // metre apart, its corners too.
        for t in tris {
            let [a, b, c] = t.p;
            let longest = (b - a).length().max((c - b).length()).max((a - c).length());
            let n = ((longest * 2.0).ceil() as usize).clamp(1, 400);
            for i in 0..=n {
                for j in 0..=n - i {
                    let p = a + (b - a) * (i as f32 / n as f32) + (c - a) * (j as f32 / n as f32);
                    if let Some(at) = out.index(p.x.floor() as i32, p.y.floor() as i32, p.z.floor() as i32) {
                        Self::set(&mut solid, at);
                    }
                }
            }
        }
        // The air flooded in from the crop's faces.
        let mut reached = vec![0u64; words];
        let mut stack: Vec<u32> = Vec::new();
        let seed = |x: i32, y: i32, z: i32, stack: &mut Vec<u32>, reached: &mut Vec<u64>| {
            if let Some(at) = out.index(x, y, z) {
                if !Self::get(&solid, at) && !Self::get(reached, at) {
                    Self::set(reached, at);
                    stack.push(at as u32);
                }
            }
        };
        for x in x0..=x1 {
            for z in z0..=z1 {
                seed(x, y0, z, &mut stack, &mut reached);
                seed(x, y1, z, &mut stack, &mut reached);
            }
        }
        for y in y0..=y1 {
            for x in x0..=x1 {
                seed(x, y, z0, &mut stack, &mut reached);
                seed(x, y, z1, &mut stack, &mut reached);
            }
            for z in z0..=z1 {
                seed(x0, y, z, &mut stack, &mut reached);
                seed(x1, y, z, &mut stack, &mut reached);
            }
        }
        while let Some(at) = stack.pop() {
            let at = at as usize;
            let i = at % w;
            let k = (at / w) % d;
            let j = at / (w * d);
            for (di, dj, dk) in [(1i32, 0i32, 0i32), (-1, 0, 0), (0, 1, 0), (0, -1, 0), (0, 0, 1), (0, 0, -1)] {
                let (ni, nj, nk) = (i as i32 + di, j as i32 + dj, k as i32 + dk);
                if ni < 0 || nj < 0 || nk < 0 || ni as usize >= w || nj as usize >= h || nk as usize >= d {
                    continue;
                }
                let next = (nj as usize * d + nk as usize) * w + ni as usize;
                if !Self::get(&solid, next) && !Self::get(&reached, next) {
                    Self::set(&mut reached, next);
                    stack.push(next as u32);
                }
            }
        }
        // Inside: neither on a triangle nor reached.
        out.bits = solid.iter().zip(&reached).map(|(s, r)| !(s | r)).collect();
        out
    }
}

/// The inside of place `place`'s levels, worked out once (none without
/// its levels).
pub fn enclosed(place: usize) -> Option<&'static Enclosed> {
    if place >= PLACES.len() { return None; }
    let source = source(place)?;
    Some(source.enclosed.get_or_init(|| {
        let started = std::time::Instant::now();
        let made = Enclosed::make(&source.tris, PLACES[place].crop);
        let inside = made.bits.iter().map(|w| w.count_ones() as usize).sum::<usize>();
        diag::info!(World, "{} inside: {} m³ enclosed of {} in {:.2} s", PLACES[place].name, inside, made.w * made.h * made.d, started.elapsed().as_secs_f32());
        made
    }))
}

impl Source {
    /// Closed modules already have a cell owner. A cap on an integer plane
    /// belongs to the solid behind its outward normal, not the next cell.
    pub(crate) fn own_cells(&mut self,range:std::ops::Range<usize>){
        for i in range {
            let t=&self.tris[i];let p=t.p.map(|p|p.as_dvec3());
            let normal=(p[2]-p[0]).cross(p[1]-p[0]).normalize_or_zero();
            let c=((p[0]+p[1]+p[2])/3.-normal*1e-5).floor().as_ivec3().to_array();
            self.cell_owners.insert(i as u32,c);
            let list=self.tiles.entry((c[0].div_euclid(TILE),c[2].div_euclid(TILE))).or_default();
            if !list.contains(&(i as u32)){list.push(i as u32);}
        }
    }
    /// Triangles filed by the tiles they reach.
    pub(crate) fn new(tris: Vec<SourceTri>) -> Self {
        let mut tiles: HashMap<(i32, i32), Vec<u32>> = HashMap::new();
        for (i, t) in tris.iter().enumerate() {
            let lo = t.p[0].min(t.p[1]).min(t.p[2]);
            let hi = t.p[0].max(t.p[1]).max(t.p[2]);
            for tx in (lo.x.floor() as i32).div_euclid(TILE)..=(hi.x.floor() as i32).div_euclid(TILE) {
                for tz in (lo.z.floor() as i32).div_euclid(TILE)..=(hi.z.floor() as i32).div_euclid(TILE) {
                    tiles.entry((tx, tz)).or_default().push(i as u32);
                }
            }
        }
        Self { tris, tiles, cell_owners:HashMap::new(), enclosed: OnceLock::new() }
    }
}

/// A triangle of a piece: its corners in the block (0 to 1), its texture
/// coordinates in its texture (0 to 1), its colours.
#[derive(Clone, Copy, Debug)]
pub struct PieceTri {
    pub p: [[f32; 3]; 3],
    pub uv: [[f32; 2]; 3],
    pub color: [[u8; 4]; 3],
    pub texture: u16,
    pub blend: bool,
}

/// One metre of a level.
#[derive(Clone, Debug, Default)]
pub struct Piece {
    pub tris: Vec<PieceTri>,
    /// The top of its level faces (floors), 0 to 1 up the block.
    pub floor: Option<f32>,
    collision: [OnceLock<Arc<[[f32; 6]]>>; 4],
}

impl Piece {
    /// Shared by Minecraft mobs and all native actor body sweeps. This lives
    /// with the source column cache, not in an unbounded world-position map.
    pub fn collision_boxes(&self) -> Arc<[[f32; 6]]> {
        self.collision[0].get_or_init(|| crate::piece_collision::boxes(self.tris.iter().map(|t|t.p)).into()).clone()
    }
    pub(crate) fn rotated_collision_boxes(&self, turns: u8) -> Arc<[[f32; 6]]> {
        let turns = turns % 4;
        if turns == 0 { return self.collision_boxes(); }
        self.collision[usize::from(turns)].get_or_init(|| self.collision_boxes().iter().map(|b| {
            let mut b = *b;
            for _ in 0..turns { b = [1.0-b[5],b[1],b[0],1.0-b[2],b[4],b[3]]; }
            b
        }).collect::<Vec<_>>().into()).clone()
    }
    pub(crate) fn invalidate_collision(&mut self) { for slot in &mut self.collision { slot.take(); } }
}

/// The pieces of 16 by 16 metres of a level (every height), by metre, and
/// per metre column the lowest piece with a floor in it: the level's
/// ground there (what the fill goes under, what is cut down to it).
#[derive(Debug, Default)]
pub struct Column {
    pub pieces: HashMap<(i32, i32, i32), Piece>,
    pub lowest_floor: HashMap<(i32, i32), i32>,
}

impl Column {
    fn new(pieces: HashMap<(i32, i32, i32), Piece>) -> Self {
        let mut lowest_floor: HashMap<(i32, i32), i32> = HashMap::new();
        for (&(x, y, z), piece) in &pieces {
            if piece.floor.is_some() {
                let e = lowest_floor.entry((x, z)).or_insert(y);
                *e = (*e).min(y);
            }
        }
        Self { pieces, lowest_floor }
    }

    /// Whether the piece at `cell` is the lowest with a floor in its metre
    /// column: the level's ground there.
    pub fn is_ground(&self, cell: (i32, i32, i32)) -> bool {
        self.lowest_floor.get(&(cell.0, cell.2)) == Some(&cell.1)
    }
}

impl std::ops::Deref for Column {
    type Target = HashMap<(i32, i32, i32), Piece>;
    fn deref(&self) -> &Self::Target {
        &self.pieces
    }
}

/// A place's triangles with their texture coordinates and colours, read
/// once (none without its levels).
pub(crate) fn source(place: usize) -> Option<&'static Source> {
    if place == TOWER_SOURCE { return crate::jak_tower::source_expanded(); }
    if place == CIRCUIT_SOURCE { return crate::jak_skate_tower::source(); }
    if place == LEGACY_ARENA_SOURCE { return crate::jak_city_arena_legacy::source(); }
    if place == LEGACY_PALACE_SOURCE { return crate::jak_city_arena_v9::source(); }
    if place == ARENA_SOURCE { return crate::jak_city_arena::source(); }
    if place >= PLACES.len() { return None; }
    static SOURCES: [OnceLock<Option<Source>>; 5] = [const { OnceLock::new() }; 5];
    SOURCES[place]
        .get_or_init(|| {
            let started = std::time::Instant::now();
            let tris = original_places::read_place_detailed(&PLACES[place])?;
            diag::info!(
                World,
                "{} pieces: {} triangles of its levels read in {:.2} s",
                PLACES[place].name,
                tris.len(),
                started.elapsed().as_secs_f32()
            );
            (!tris.is_empty()).then(|| Source::new(tris))
        })
        .as_ref()
}

/// Reads every place's levels, their ground tiles and the objects' shapes
/// on a background thread, once (the world's start): a place read when its
/// first piece is drawn or met held the frame 0.5 to 1.8 s (23/10, the
/// Wasteland bench: a 900 ms frame as Spargus was read).
pub fn prefetch() {
    if !original_places::jak_places_on() {
        return;
    }
    static STARTED: OnceLock<()> = OnceLock::new();
    STARTED.get_or_init(|| {
        let _ = std::thread::Builder::new().name("jak-pieces-prefetch".into()).spawn(|| {
            minecraftoss_core::thread_priority::background();
            for place in 0..PLACES.len() {
                let _ = source(place);
            }
            // (Spargus's level stands only when asked for: its inside,
            // 27 s and 120 MB, is not worked out otherwise.)
            for place in 0..PLACES.len() {
                if place == 0 && !crate::spargus_town::level_wanted() {
                    continue;
                }
                let _ = enclosed(place);
            }
            for place in 0..PLACES.len() {
                let _ = crate::jak_ground::library(place);
            }
            for kind in 0..crate::jak_objects::OBJECTS.len() {
                let _ = crate::jak_objects::shape(kind);
            }
            for place in 0..PLACES.len() {
                let _ = crate::jak_hamlets::buildings(place);
            }
            for wall in 0..crate::jak_walls::WALLS.len() {
                let _ = crate::jak_walls::library(wall);
            }
        });
    });
}

/// The pieces of the 16 metres square whose corner is `(jx0, jz0)` (metres
/// of the level), cut once and kept a while (generation and drawing ask for
/// the same ones).
pub fn column(place: usize, (jx0, jz0): (i32, i32)) -> Option<Arc<Column>> {
    if place == TOWER_SOURCE { return crate::jak_tower::column(true); }
    if place == CIRCUIT_SOURCE { return crate::jak_skate_tower::column(jx0, jz0); }
    if place == LEGACY_ARENA_SOURCE { return crate::jak_city_arena_legacy::column(jx0, jz0); }
    if place == LEGACY_PALACE_SOURCE { return crate::jak_city_arena_v9::column(jx0, jz0); }
    if place == ARENA_SOURCE { return crate::jak_city_arena::column(jx0, jz0); }
    if crate::jak_tower::source_column(place,jx0,jz0) { return crate::jak_tower::column(false); }
    static CACHE: OnceLock<Mutex<HashMap<(usize, i32, i32), Arc<Column>>>> = OnceLock::new();
    let cache = CACHE.get_or_init(Default::default);
    if let Some(found) = cache.lock().unwrap_or_else(|e| e.into_inner()).get(&(place, jx0, jz0)) {
        return Some(found.clone());
    }
    let source = source(place)?;
    let cut = Arc::new(cut_column(source, jx0, jz0));
    let mut cache = cache.lock().unwrap_or_else(|e| e.into_inner());
    if cache.len() > 768 {
        cache.clear();
    }
    cache.insert((place, jx0, jz0), cut.clone());
    Some(cut)
}

/// The piece the block at world `pos` shows, a piece of place `place`.
pub fn piece_at(place: usize, pos: (i32, i32, i32)) -> Option<(usize, Arc<Column>, (i32, i32, i32))> {
    let (place, cell) = source_at(place, pos)?;
    // A moved metre is independent of where (or whether) its original
    // landmark is generated in this world. Cut in source coordinates.
    let column = column(place, (cell.0.div_euclid(TILE)*TILE,cell.2.div_euclid(TILE)*TILE))?;
    Some((place, column, cell))
}

/// Skatea's native collision excludes visual decals and keeps the original
/// ramp surfaces. Resolve carried cells first so it works outside the town.
fn collision_piece_at(place: usize, pos: (i32, i32, i32)) -> Option<(Arc<Column>, (i32, i32, i32))> {
    let (place, cell) = source_at(place, pos)?;
    let (x, z) = (cell.0.div_euclid(TILE) * TILE, cell.2.div_euclid(TILE) * TILE);
    let column = if place == CIRCUIT_SOURCE { crate::jak_skate_tower::collision_column(x, z)? }
        else if place == LEGACY_ARENA_SOURCE {crate::jak_city_arena_legacy::collision_column(x,z)?}
        else if place == LEGACY_PALACE_SOURCE {crate::jak_city_arena_v9::collision_column(x,z)?}
    else if place == ARENA_SOURCE {crate::jak_city_arena::collision_column(x,z)?}
        else { column(place, (x, z))? };
    Some((column, cell))
}

/// Which metre of which place the piece block at `pos` (a block of place
/// `place`) shows: the one it was moved with, else the one where it stands.
pub fn source_at(place: usize, pos: (i32, i32, i32)) -> Option<(usize, (i32, i32, i32))> {
    if let Some(&(moved_place, cell)) = moved().read().ok()?.current.at.get(&pos) {
        return Some((usize::from(moved_place), cell));
    }
    if place == TOWER_SOURCE { return crate::jak_tower::source_at_expanded(pos); }
    if place == CIRCUIT_SOURCE { return crate::jak_skate_tower::source_at(pos); }
    if place == LEGACY_ARENA_SOURCE {return crate::jak_city_arena_legacy::source_at(pos);}
    if place == LEGACY_PALACE_SOURCE {return crate::jak_city_arena_v9::source_at(pos);}
    if place == ARENA_SOURCE {return crate::jak_city_arena::source_at(pos);}
    if place >= PLACES.len() { return None; }
    if place==1 {if let Some(found)=crate::jak_tower::source_at(pos){return Some(found);}}
    // A building copied into a hamlet of the Jak lands.
    if let Some(found) = crate::jak_hamlets::source_at(pos) {
        return Some(found);
    }
    let site = original_places::current_sites()?[place]?;
    Some((place, original_places::to_level(&PLACES[place], site, pos)))
}

/// Whether the piece of `place`'s metre `cell` keeps to the bottom half of
/// its block (a bottom slab; else a top one).
pub fn is_low(place: usize, cell: (i32, i32, i32)) -> bool {
    let Some(column) = column(place,(cell.0.div_euclid(TILE)*TILE,cell.2.div_euclid(TILE)*TILE)) else { return true };
    column
        .get(&cell)
        .is_none_or(|piece| piece.tris.iter().all(|t| t.p.iter().all(|p| p[1] <= 0.5)))
}

type MovedMap=HashMap<(i32,i32,i32),(u8,(i32,i32,i32))>;
/// Each dimension has its own coordinates and file. The root-level legacy
/// file is read only as an Overworld fallback, never copied into the Nether.
#[derive(Default)]
struct DimensionPieces {
    file: Option<std::path::PathBuf>,
    at: MovedMap,
    revision:u64,
    dirty:bool,
    load_error:Option<String>,
}
#[derive(Default)]
struct Moved {
    root:Option<std::path::PathBuf>,
    dimension:String,
    current:DimensionPieces,
    parked:HashMap<String,DimensionPieces>,
}
fn decode_moved(text:&str)->Option<MovedMap>{
    let rows=serde_json::from_str::<Vec<[i32;7]>>(text).ok()?;
    Some(rows.into_iter().filter(|r|usize::try_from(r[3]).ok().is_some_and(portable_source))
        .map(|r|((r[0],r[1],r[2]),(r[3] as u8,(r[4],r[5],r[6])))).collect())
}

fn moved() -> &'static std::sync::RwLock<Moved> {
    static MOVED: OnceLock<std::sync::RwLock<Moved>> = OnceLock::new();
    MOVED.get_or_init(Default::default)
}

/// The moved pieces of the saved world in `dir` (none for a world not
/// saved).
pub fn load_moved(dir: Option<&std::path::Path>) {
    if let Err(e)=flush_moved(){diag::warn!(World,"native piece save before world change failed: {e}");}
    let current=read_dimension_pieces(dir,"minecraft:overworld");
    if let Ok(mut moved) = moved().write() {
        *moved = Moved { root:dir.map(std::path::Path::to_owned),dimension:"minecraft:overworld".into(),current,parked:HashMap::new() };
    }
}

fn dimension_piece_path(root:&std::path::Path,dimension:&str)->Option<std::path::PathBuf>{
    let(namespace,path)=dimension.split_once(':')?;
    let valid=|s:&str|!s.is_empty()&&s.bytes().all(|c|c.is_ascii_lowercase()||c.is_ascii_digit()||b"._-".contains(&c));
    if !valid(namespace)||namespace=="."||namespace==".."||!path.split('/').all(|s|valid(s)&&s!="."&&s!=".."){return None;}
    Some(root.join("dimensions").join(namespace).join(path).join("jak_piezas.json"))
}
fn read_dimension_pieces(root:Option<&std::path::Path>,dimension:&str)->DimensionPieces{
    let Some(root)=root else{return DimensionPieces::default();};
    let Some(file)=dimension_piece_path(root,dimension)else{
        return DimensionPieces{load_error:Some(format!("invalid native-piece dimension {dimension}")),..Default::default()};
    };
    let read=|path:&std::path::Path|->Result<Option<MovedMap>,String>{
        match std::fs::read_to_string(path){
            Ok(text)=>decode_moved(&text).map(Some).ok_or_else(||format!("invalid native piece JSON: {}",path.display())),
            Err(e)if e.kind()==std::io::ErrorKind::NotFound=>Ok(None),
            Err(e)=>Err(format!("read {}: {e}",path.display())),
        }
    };
    let loaded=read(&file).and_then(|found|if found.is_none()&&dimension=="minecraft:overworld"{read(&root.join("jak_piezas.json"))}else{Ok(found)});
    match loaded{
        Ok(at)=>DimensionPieces{file:Some(file),at:at.unwrap_or_default(),..Default::default()},
        Err(e)=>{diag::warn!(World,"native pieces could not be restored; source file preserved: {e}");
            DimensionPieces{file:Some(file),load_error:Some(e),..Default::default()}}
    }
}

/// Called when a loaded dimension actually becomes active, including cached
/// returns and the old skate overlay. Unsaved worlds also retain both maps.
pub fn activate_moved_dimension(dimension:&str){
    if let Err(e)=flush_moved(){diag::warn!(World,"native piece save before dimension change failed: {e}");}
    let mut held=match moved().write(){Ok(held)=>held,Err(_)=>return};
    if held.dimension==dimension{return;}
    let next=held.parked.remove(dimension).unwrap_or_else(||read_dimension_pieces(held.root.as_deref(),dimension));
    let previous=std::mem::replace(&mut held.current,next);
    let old=std::mem::replace(&mut held.dimension,dimension.to_owned());
    held.parked.insert(old,previous);
}

fn encode_moved(at:&MovedMap)->Result<String,String>{
    let mut rows:Vec<[i32;7]>=at.iter().map(|(&(x,y,z),&(p,(cx,cy,cz)))|[x,y,z,i32::from(p),cx,cy,cz]).collect();
    rows.sort_unstable();serde_json::to_string(&rows).map_err(|e|e.to_string())
}
fn atomic_piece_write_with(path:&std::path::Path,text:&str,commit:impl FnOnce(&std::path::Path,&std::path::Path)->std::io::Result<()>)->std::io::Result<()>{
    use std::io::Write;
    use std::sync::atomic::{AtomicU64,Ordering};
    static NEXT:AtomicU64=AtomicU64::new(0);
    if let Some(parent)=path.parent(){std::fs::create_dir_all(parent)?;}
    let temp=path.with_extension(format!("json.{}.{}.tmp",std::process::id(),NEXT.fetch_add(1,Ordering::Relaxed)));
    let mut file=std::fs::OpenOptions::new().write(true).create_new(true).open(&temp)?;
    let result=(||{file.write_all(text.as_bytes())?;file.sync_all()?;drop(file);commit(&temp,path)})();
    if result.is_err(){let _=std::fs::remove_file(&temp);}
    result
}

/// Retry dirty dimensions at ordinary checkpoints. Disk I/O holds only the
/// writer mutex, not the RwLock used by every actor/mesher source lookup.
/// A failed commit retains both the last complete file and the current map.
pub fn flush_moved()->Result<(),String>{
    static WRITER:Mutex<()>=Mutex::new(());
    let _writing=WRITER.lock().map_err(|_|"native piece writer poisoned")?;
    let(root,snapshots,mut errors)={
        let held=moved().read().map_err(|_|"native piece state poisoned")?;
        let mut snapshots=Vec::new();let mut errors=Vec::new();
        for(dim,state)in std::iter::once((&held.dimension,&held.current)).chain(held.parked.iter()){
            if !state.dirty{continue;}
            if let Some(e)=&state.load_error{errors.push(format!("refusing to replace unreadable metadata: {e}"));continue;}
            if let Some(file)=&state.file{snapshots.push((dim.clone(),file.clone(),state.revision,encode_moved(&state.at)?));}
        }
        (held.root.clone(),snapshots,errors)
    };
    for(dim,file,revision,text)in snapshots{
        if let Err(e)=atomic_piece_write_with(&file,&text,|from,to|std::fs::rename(from,to)){
            errors.push(format!("commit {}: {e}; modified pieces remain in memory for retry",file.display()));continue;
        }
        let mut held=moved().write().map_err(|_|"native piece state poisoned")?;
        if held.root!=root{continue;}
        let state=if held.dimension==dim{Some(&mut held.current)}else{held.parked.get_mut(&dim)};
        if let Some(state)=state.filter(|s|s.revision==revision){state.dirty=false;}
    }
    if errors.is_empty(){Ok(())}else{Err(errors.join("; "))}
}

/// A piece put down at `pos` showing `source` (place, metre), or (`None`)
/// none there any more.
pub fn set_moved(pos: (i32, i32, i32), source: Option<(usize, (i32, i32, i32))>) {
    if let Err(e)=set_moved_batch(std::iter::once((pos,source))){diag::warn!(World,"native piece save failed: {e}");}
}

/// A complete native structure commits its source map once. Serializing
/// the growing map for every cell turns a bulk placement into O(n²) work.
pub fn set_moved_batch(changes:impl IntoIterator<Item=((i32,i32,i32),Option<(usize,(i32,i32,i32))>)>)->Result<(),String>{
    let mut moved=moved().write().map_err(|_|"native piece state poisoned")?;
    let mut changed=false;
    for(pos,source)in changes {changed|=match source{
        Some((place,cell))=>moved.current.at.insert(pos,(place as u8,cell))!=Some((place as u8,cell)),
        None=>moved.current.at.remove(&pos).is_some(),
    };}
    if !changed {
        return Ok(());
    }
    moved.current.revision=moved.current.revision.wrapping_add(1);
    moved.current.dirty=moved.current.file.is_some();
    drop(moved);
    flush_moved()
}

/// The item a piece drops as: its block's item, carrying the metre it
/// shows (`jak:pieza`), so it shows the same wherever it is put down.
pub fn item_components(place: usize, cell: (i32, i32, i32)) -> serde_json::Value {
    serde_json::json!({ "jak:pieza": [place, cell.0, cell.1, cell.2] })
}

/// The metre a piece item carries (`item_components`).
pub fn carried(components: Option<&serde_json::Value>) -> Option<(usize, (i32, i32, i32))> {
    let row = components?.get("jak:pieza")?.as_array()?;
    let n = |k: usize| row.get(k).and_then(serde_json::Value::as_i64).and_then(|v| i32::try_from(v).ok());
    let place = usize::try_from(n(0)?).ok().filter(|&p| portable_source(p))?;
    Some((place, (n(1)?, n(2)?, n(3)?)))
}

/// The triangles of the piece at world block `pos` of place `place`, in
/// the block (0 to 1): what Jak stands on and bumps into.
pub fn triangles(place: u8, pos: [i32; 3]) -> Vec<[[f32; 3]; 3]> {
    if usize::from(place) == FOLIAGE_SOURCE { return Vec::new(); }
    // An object's block: its object's triangles there, turned with it.
    if !independent_source(usize::from(place)) {
        if let Some((_, piece)) = crate::jak_objects::collision_piece_at((pos[0], pos[1], pos[2])) {
            return piece.tris.iter().map(|t| t.p).collect();
        }
    }
    collision_piece_at(usize::from(place), (pos[0], pos[1], pos[2]))
        .and_then(|(column, cell)| column.get(&cell).map(|p| p.tris.iter().map(|t| t.p).collect()))
        .unwrap_or_default()
}

/// The exact same cached local body shape for every kind of actor. `None`
/// leaves an unmeshed/fill block on its catalog fallback; a known empty shape
/// remains empty rather than turning back into an invisible slab.
pub fn collision_boxes(place: u8, pos: [i32; 3]) -> Option<Arc<[[f32; 6]]>> {
    static EMPTY: OnceLock<Arc<[[f32; 6]]>> = OnceLock::new();
    let empty = || EMPTY.get_or_init(|| Vec::new().into()).clone();
    if usize::from(place) == FOLIAGE_SOURCE { return Some(empty()); }
    let pos = (pos[0],pos[1],pos[2]);
    if !independent_source(usize::from(place)) { if let Some(boxes) = crate::jak_objects::collision_boxes(pos) { return Some(boxes); } }
    let found = collision_piece_at(usize::from(place),pos).and_then(|(column,cell)|column.get(&cell).map(Piece::collision_boxes));
    if independent_source(usize::from(place)) { Some(found.unwrap_or_else(empty)) } else { found }
}

/// Native flags only need emptiness or the single full cube. Keep those
/// queries on the shared shape rather than copying every box to f64.
pub(crate) fn block_collision_shape(block: &crate::scene::Block, pos: (i32,i32,i32)) -> Option<Arc<[[f32; 6]]>> {
    if block.id.namespace != "jak" { return None; }
    let place = place_of(&block.id.path)?;
    collision_boxes(place as u8,[pos.0,pos.1,pos.2])
}

pub(crate) fn block_collision_boxes(block: &crate::scene::Block, pos: (i32,i32,i32)) -> Option<Vec<[f64; 6]>> {
    Some(block_collision_shape(block,pos)?.iter().map(|b|b.map(f64::from)).collect())
}

/// The world chunk's corner as metres of the level (the square of the
/// level a chunk shows, the same for generating it and drawing it).
pub fn chunk_corner(place: usize, site: (i32, i32), (cx, cz): (i32, i32)) -> (i32, i32) {
    let (jx, _, jz) = original_places::to_level(&PLACES[place], site, (cx * 16, 0, cz * 16));
    (jx, jz)
}

/// Which place a piece block belongs to, by its name (`pieza_<place>`).
pub fn place_of(path: &str) -> Option<usize> {
    if path == "pieza_haven_fuego"{return PLACES.iter().position(|p|p.name=="haven");}
    if path == "pieza_haven_torre" { return Some(TOWER_SOURCE); }
    if path == "pieza_haven_follaje" { return Some(FOLIAGE_SOURCE); }
    if path == "pieza_haven_circuito" { return Some(CIRCUIT_SOURCE); }
    if path == "pieza_haven_arena" {return Some(LEGACY_ARENA_SOURCE);}
    if path == "pieza_haven_torre_circuito" {return Some(LEGACY_PALACE_SOURCE);}
    if path == "pieza_haven_rampa_modular" {return Some(ARENA_SOURCE);}
    let name = path.strip_prefix("pieza_")?;
    PLACES.iter().position(|p| p.name == name)
}

/// The block of place `place`'s pieces.
pub fn block_of(place: &Place) -> String {
    format!("jak:pieza_{}", place.name)
}

/// Draws the piece at `pos` (a block of place `place`): its triangles from
/// both sides, each corner in its texture's place in the atlas, coloured by
/// the level's vertex colour (128 the texture as it is: the materials'
/// colour factor is 2, as the PS2 has it), its blended layers over the rest
/// by their vertex alpha. The levels' light is in those colours (a room's
/// inside is as bright as the game made it), so the sky's light on it is
/// full and only the hour of the day dims it; a torch's still adds.
///
/// A piece is a whole block (Adrii, 23/10: "que los bloques sean completos
/// con 6 caras"): the level's ground (the lowest floor of its metre
/// column) is closed under its floor, its cut down each side open to the
/// air and its bottom drawn in its floor's texture; a piece with nothing
/// of its level in it (the ground under a place's floors) is a cube of the
/// floor over it, its faces open to the air drawn. A floor over open air
/// (a bridge, a balcony, a dock) and an object's leaves are drawn as the
/// level has them, nothing cut down from them (23/10: those cuts hung as
/// dark slivers round every plant and under every walkway).
pub(crate) fn append<S: crate::scene::Scene>(
    scene: &S,
    pos: (i32, i32, i32),
    place: usize,
    atlas: &crate::mesh::Atlas,
    light: &crate::lighting::SkyLight,
    mesh: &mut crate::mesh::ChunkMesh,
    transparent_indices: &mut Vec<u32>,
) {
    let Some(visual) = visual_piece(place, pos) else { return };
    let place = visual.place;
    let regions = regions(place, atlas);
    let (x, y, z) = pos;
    // A cut face can reach air diagonally even when its six axis neighbours
    // are also occupied by tiny pieces of the same sloped shell. Sample the
    // complete one-cell neighbourhood, not just the axis cross; otherwise
    // those corner fragments become black triangles on a continuous surface.
    let mut sky = 0;
    let mut block = 0;
    for dx in -1..=1 { for dy in -1..=1 { for dz in -1..=1 {
        let q = (x + dx, y + dy, z + dz);
        sky = sky.max(light.get(q));
        block = block.max(light.get_block(q));
    }}}
    let (sky, block) = (f32::from(sky), f32::from(block));
    let base = [(x - mesh.origin[0]) as f32, (y - mesh.origin[1]) as f32, (z - mesh.origin[2]) as f32];
    // The original palace separates illuminated windows into their own
    // material. Keep that material lit at night without making walls emissive.
    static LIT_WINDOW:OnceLock<Option<u16>>=OnceLock::new();
    let lit_window=*LIT_WINDOW.get_or_init(||crate::original_places::textures_of(&crate::original_places::PLACES[1]).iter()
        .position(|name|*name=="citywide-window-litwindow").map(|i|i as u16));
    let first_vertex = mesh.vertices.len();
    let mut push = |p: [[f32; 3]; 3], uv: [[f32; 2]; 3], color: [[f32; 4]; 3], texture: u16, blend: bool| {
        let Some(&[u0, v0, u1, v1]) = regions.get(usize::from(texture)) else { return };
        // Sample the outside of each face. Closed ramp cells use clockwise
        // winding; sampling the other side darkens their exposed surface.
        let n=surface_normal(place,texture,p);
        let start = mesh.vertices.len() as u32;
        for k in 0..3 {
            let [u, v] = uv[k];
            let (sky,block)=if matches!(place,ARENA_SOURCE|LEGACY_PALACE_SOURCE) && n.length_squared()>0.5 {
                surface_light(light,Vec3::new(x as f32,y as f32,z as f32)+Vec3::from_array(p[k])+n*1.5)
            }else{(sky,block)};
            mesh.vertices.push(crate::mesh::Vertex {
                position: [base[0] + p[k][0], base[1] + p[k][1], base[2] + p[k][2]],
                uv: [u0 + (u1 - u0) * u, v0 + (v1 - v0) * v],
                color: color[k],
                sky_light: sky,
                block_light: if matches!(place,ARENA_SOURCE|LEGACY_PALACE_SOURCE) && lit_window==Some(texture){15.0}else{block},
            });
        }
        let indices = if blend { &mut *transparent_indices } else { &mut mesh.indices };
        indices.extend_from_slice(&[start, start + 1, start + 2, start, start + 2, start + 1]);
        mesh.faces += 2;
    };
    emit_visual_geometry::<true>(scene, pos, &visual, &mut push);
    if visual.object.is_some() && mesh.vertices.len() > first_vertex {
        mesh.camera_protected.push(first_vertex..mesh.vertices.len());
    }
}

fn surface_normal(place:usize,texture:u16,p:[[f32;3];3])->Vec3{
    let [a,b,c]=p.map(Vec3::from_array);
    let n=(b-a).cross(c-a).normalize_or_zero();
    static RAMP:OnceLock<Option<u16>>=OnceLock::new();
    let ramp=RAMP.get_or_init(||crate::original_places::textures_of(&crate::original_places::PLACES[1]).iter()
        .position(|name|*name=="skatea-ramp-01").map(|i|i as u16));
    if place==ARENA_SOURCE && *ramp==Some(texture){-n}else{n}
}

// Trilinear samples remove cell-shaped seams across a single native face.
fn surface_light(light:&crate::lighting::SkyLight,p:Vec3)->(f32,f32){
    let p=p-Vec3::splat(0.5);let base=p.floor();let f=p-base;let q=base.as_ivec3();
    let(mut sky,mut block)=(0.0,0.0);
    for dx in 0..=1 {for dy in 0..=1 {for dz in 0..=1 {
        let w=(if dx==0{1.0-f.x}else{f.x})*(if dy==0{1.0-f.y}else{f.y})*(if dz==0{1.0-f.z}else{f.z});
        let at=(q.x+dx,q.y+dy,q.z+dz);
        sky+=w*f32::from(light.get(at));block+=w*f32::from(light.get_block(at));
    }}}
    (sky,block)
}

/// Resolve exactly what `append` renders: moved cells and native objects
/// (including their rotations) take the same precedence for both callers.
struct NativeVisual {
    place: usize,
    object: Option<Piece>,
    column: Option<(Arc<Column>, (i32, i32, i32))>,
}

fn visual_piece(place: usize, pos: (i32, i32, i32)) -> Option<NativeVisual> {
    let object = if place == FOLIAGE_SOURCE { crate::jak_objects::foliage_piece_at(pos) }
        else if independent_source(place) { None } else { crate::jak_objects::piece_at(pos) };
    if let Some((place, piece)) = object {
        Some(NativeVisual { place, object: Some(piece), column: None })
    } else {
        let (place, column, cell) = piece_at(place, pos)?;
        Some(NativeVisual { place, object: None, column: Some((column, cell)) })
    }
}

/// One geometry traversal for rendering and selection, including the old
/// ground's visible caps/fill. Rays skip colour work, never the triangles.
fn emit_visual_geometry<const COLORS: bool>(
    scene: &impl crate::scene::Scene,
    pos: (i32, i32, i32),
    visual: &NativeVisual,
    mut push: impl FnMut([[f32; 3]; 3], [[f32; 2]; 3], [[f32; 4]; 3], u16, bool),
) {
    let place = visual.place;
    let (x, y, z) = pos;
    let air = |(dx, dy, dz): (i32, i32, i32)| scene.block((x + dx, y + dy, z + dz)).is_none();
    let rgba = |c: [u8; 4], shade: f32| -> [f32; 4] {
        if !COLORS { return [0.0; 4]; }
        let c = c.map(|c| (f32::from(c) / 128.0).min(1.0));
        [c[0] * shade, c[1] * shade, c[2] * shade, 1.0]
    };
    // The cube's faces: which way, corners anticlockwise, and shade.
    let faces: [((i32, i32, i32), [[f32; 3]; 4], f32); 6] = [
        ((0, 1, 0), [[0.0, 1.0, 0.0], [1.0, 1.0, 0.0], [1.0, 1.0, 1.0], [0.0, 1.0, 1.0]], 1.0),
        ((0, -1, 0), [[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [1.0, 0.0, 1.0], [0.0, 0.0, 1.0]], 0.5),
        ((0, 0, -1), [[0.0, 1.0, 0.0], [1.0, 1.0, 0.0], [1.0, 0.0, 0.0], [0.0, 0.0, 0.0]], 0.8),
        ((0, 0, 1), [[1.0, 1.0, 1.0], [0.0, 1.0, 1.0], [0.0, 0.0, 1.0], [1.0, 0.0, 1.0]], 0.8),
        ((1, 0, 0), [[1.0, 1.0, 0.0], [1.0, 1.0, 1.0], [1.0, 0.0, 1.0], [1.0, 0.0, 0.0]], 0.6),
        ((-1, 0, 0), [[0.0, 1.0, 1.0], [0.0, 1.0, 0.0], [0.0, 0.0, 0.0], [0.0, 0.0, 1.0]], 0.6),
    ];
    let square = |q: [[f32; 3]; 4], color: [f32; 4], texture: u16, push: &mut dyn FnMut([[f32; 3]; 3], [[f32; 2]; 3], [[f32; 4]; 3], u16, bool)| {
        let uv = [[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]];
        push([q[0], q[1], q[2]], [uv[0], uv[1], uv[2]], [color; 3], texture, false);
        push([q[0], q[2], q[3]], [uv[0], uv[2], uv[3]], [color; 3], texture, false);
    };
    let piece = visual.object.as_ref().or_else(|| visual.column.as_ref().and_then(|(column, cell)| column.get(cell)));
    let Some(piece) = piece.filter(|p| !p.tris.is_empty()) else {
        // Never invent a floor or enclosing cube for an empty circuit cell.
        if matches!(place,CIRCUIT_SOURCE|ARENA_SOURCE|LEGACY_PALACE_SOURCE|LEGACY_ARENA_SOURCE) { return; }
        let Some((column, cell)) = &visual.column else { return };
        // The ground under the floors: a cube of the floor over it.
        let over = (1..=48).find_map(|k| column.get(&(cell.0, cell.1 + k, cell.2)).filter(|p| p.floor.is_some()));
        let Some((texture, color)) = over.and_then(floor_look) else { return };
        for (d, q, shade) in faces {
            if air(d) {
                square(q, [color[0] * shade, color[1] * shade, color[2] * shade, 1.0], texture, &mut push);
            }
        }
        return;
    };
    // The level's ground here: the lowest floor of its metre column (not an
    // object's, not a floor with the level's open air under it).
    let ground = visual.object.is_none() && visual.column.as_ref().is_some_and(|(column, cell)|
        !crate::jak_tower::source_cell(place, *cell)
            && !crate::jak_skate_tower::source_cell(place, *cell)
            && !crate::jak_city_arena_legacy::source_cell(place, *cell) && !crate::jak_city_arena_v9::source_cell(place,*cell) && !crate::jak_city_arena::source_cell(place, *cell) && column.is_ground(*cell));
    let floor_top = piece.floor.unwrap_or(1.0);
    // A building copied into a village stands under this world's sky: its
    // colours brought to the texture as it is on average (23/10: the
    // level's shade under Spargus's halls lay as black squares round them).
    let gain = if COLORS && !independent_source(place) && visual.object.is_none() && crate::jak_hamlets::source_at(pos).is_some() {
        let luma: f32 = piece.tris.iter().flat_map(|t| t.color.iter()).map(|c| 0.299 * f32::from(c[0]) + 0.587 * f32::from(c[1]) + 0.114 * f32::from(c[2])).sum::<f32>()
            / (piece.tris.len() * 3).max(1) as f32;
        if luma < 24.0 { 0.0 } else { (128.0 / luma).clamp(1.0, 4.0) }
    } else {
        1.0
    };
    for t in &piece.tris {
        let color: [[f32; 4]; 3] = if !COLORS { [[0.0; 4]; 3] } else { std::array::from_fn(|k| {
            let c = t.color[k].map(|c| if gain == 0.0 { 1.0 } else { (f32::from(c) * gain / 128.0).min(1.0) });
            [c[0], c[1], c[2], if t.blend { (f32::from(t.color[k][3]) / 128.0).min(1.0) } else { 1.0 }]
        }) };
        push(t.p, t.uv, color, t.texture, t.blend);
        // The ground's floor cut down each side open to the air.
        if t.blend || !ground {
            continue;
        }
        let [a, b, c] = t.p.map(Vec3::from_array);
        if (b - a).cross(c - a).normalize_or_zero().y.abs() <= 0.7 || t.p.iter().any(|p| (p[1] - floor_top).abs() > 0.25) {
            continue;
        }
        for k in 0..3 {
            let (pa, pb) = (t.p[k], t.p[(k + 1) % 3]);
            let on = |axis: usize, at: f32| (pa[axis] - at).abs() < 1e-3 && (pb[axis] - at).abs() < 1e-3;
            let side = if on(0, 0.0) {
                Some(((-1, 0, 0), 2))
            } else if on(0, 1.0) {
                Some(((1, 0, 0), 2))
            } else if on(2, 0.0) {
                Some(((0, 0, -1), 0))
            } else if on(2, 1.0) {
                Some(((0, 0, 1), 0))
            } else {
                None
            };
            let Some((_, along)) = side.filter(|(d, _)| air(*d)) else { continue };
            let foot = |p: [f32; 3]| [p[0], 0.0, p[2]];
            let uv = |p: [f32; 3]| [p[along], 1.0 - p[1]];
            let (ca, cb) = (rgba(t.color[k], 0.7), rgba(t.color[(k + 1) % 3], 0.7));
            push([pa, pb, foot(pb)], [uv(pa), uv(pb), uv(foot(pb))], [ca, cb, cb], t.texture, false);
            push([pa, foot(pb), foot(pa)], [uv(pa), uv(foot(pb)), uv(foot(pa))], [ca, cb, ca], t.texture, false);
        }
    }
    // The ground's bottom, open to the air under it.
    if ground && air((0, -1, 0)) {
        if let Some((texture, color)) = floor_look(piece) {
            let (_, q, shade) = faces[1];
            square(q, [color[0] * shade, color[1] * shade, color[2] * shade, 1.0], texture, &mut push);
        }
    }
}

/// Select the visible native mesh, not its slab registration or its thickened
/// body collision. This also lets a non-colliding decorative leaf be mined.
pub(crate) fn outline_hit(
    scene: &impl crate::scene::Scene, pos: (i32, i32, i32), place: usize,
    origin: glam::DVec3, dir: glam::DVec3, reach: f64,
) -> Option<minecraftoss_player::Hit> {
    let visual = visual_piece(place, pos)?;
    let offset = glam::DVec3::new(f64::from(pos.0), f64::from(pos.1), f64::from(pos.2));
    let mut best: Option<minecraftoss_player::Hit> = None;
    emit_visual_geometry::<false>(scene, pos, &visual, |triangle, _, _, _, _| {
        if let Some((distance, face)) = ray_triangle(origin - offset, dir, triangle) {
            if distance <= reach && best.as_ref().is_none_or(|hit| distance < hit.distance) {
                best = Some(minecraftoss_player::Hit { pos, face, distance, point: origin + dir * distance });
            }
        }
    });
    best
}

/// Double-sided because the native renderer draws both windings. The
/// dominant axis supplies Minecraft's placement face for sloped surfaces.
fn ray_triangle(origin: glam::DVec3, dir: glam::DVec3, triangle: [[f32; 3]; 3]) -> Option<(f64, minecraftoss_player::Face)> {
    use minecraftoss_player::Face;
    let [a, b, c] = triangle.map(|p| glam::DVec3::from_array(p.map(f64::from)));
    let (edge1, edge2) = (b - a, c - a);
    let cross = dir.cross(edge2);
    let determinant = edge1.dot(cross);
    if !determinant.is_finite() || determinant.abs() <= 1e-12 { return None; }
    let delta = origin - a;
    let u = delta.dot(cross) / determinant;
    let q = delta.cross(edge1);
    let v = dir.dot(q) / determinant;
    if u < -1e-9 || v < -1e-9 || u + v > 1.0 + 1e-9 { return None; }
    let distance = edge2.dot(q) / determinant;
    if !distance.is_finite() || distance < -1e-9 { return None; }
    let mut normal = edge1.cross(edge2);
    if normal.dot(dir) > 0.0 { normal = -normal; }
    let n = normal.abs();
    let face = if n.x >= n.y && n.x >= n.z {
        if normal.x < 0.0 { Face::West } else { Face::East }
    } else if n.y >= n.z {
        if normal.y < 0.0 { Face::Down } else { Face::Up }
    } else if normal.z < 0.0 { Face::North } else { Face::South };
    Some((distance.max(0.0), face))
}

/// A piece's floor: the texture most of its level triangles wear, and their
/// mean colour (1 the texture as it is).
fn floor_look(piece: &Piece) -> Option<(u16, [f32; 3])> {
    let mut by_texture: HashMap<u16, f32> = HashMap::new();
    let (mut sum, mut area_sum) = ([0.0f32; 3], 0.0f32);
    for t in &piece.tris {
        let [a, b, c] = t.p.map(Vec3::from_array);
        let n = (b - a).cross(c - a);
        if t.blend || n.normalize_or_zero().y.abs() <= 0.7 {
            continue;
        }
        let area = n.length() * 0.5;
        *by_texture.entry(t.texture).or_default() += area;
        for k in 0..3 {
            sum[k] += (f32::from(t.color[0][k]) + f32::from(t.color[1][k]) + f32::from(t.color[2][k])) / 3.0 * area;
        }
        area_sum += area;
    }
    let texture = by_texture.into_iter().max_by(|a, b| a.1.total_cmp(&b.1))?.0;
    Some((texture, sum.map(|c| (c / area_sum.max(1e-6) / 128.0).min(1.0))))
}

/// Each of place `place`'s textures' place in the atlas (`jak:block/...`),
/// kept for the atlas they were found in.
pub(crate) fn regions(place: usize, atlas: &crate::mesh::Atlas) -> Arc<Vec<[f32; 4]>> {
    let Some(place) = texture_place(place) else { return Arc::new(Vec::new()); };
    static REGIONS: OnceLock<Mutex<HashMap<(usize, usize), Arc<Vec<[f32; 4]>>>>> = OnceLock::new();
    let key = (atlas as *const crate::mesh::Atlas as usize, place);
    let cache = REGIONS.get_or_init(Default::default);
    if let Some(found) = cache.lock().unwrap_or_else(|e| e.into_inner()).get(&key) {
        return found.clone();
    }
    let found: Arc<Vec<[f32; 4]>> = Arc::new(
        original_places::textures_of(&PLACES[place])
            .iter()
            .map(|t| {
                crate::pack::ResourceId::parse(&format!("jak:block/{}", t.to_lowercase()))
                    .map(|id| atlas.region(&id))
                    .unwrap_or([0.0, 0.0, 1.0, 1.0])
            })
            .collect(),
    );
    let mut cache = cache.lock().unwrap_or_else(|e| e.into_inner());
    if cache.len() > 32 {
        cache.clear();
    }
    cache.insert(key, found.clone());
    found
}

#[derive(Clone, Copy)]
// Clip in double precision, then convert to float only after subtracting the
// cell origin. Float interpolation at canonical coordinates near8000 opened
// sub-millimetre cracks between independently clipped faces and UV repeats.
struct V {
    p: glam::DVec3,
    uv: glam::DVec2,
    c: glam::DVec4,
}

fn lerp(a: &V, b: &V, t: f64) -> V {
    V { p: a.p.lerp(b.p, t), uv: a.uv.lerp(b.uv, t), c: a.c.lerp(b.c, t) }
}

/// The polygon cut by `value = k`: the part under it and the part over it.
fn split(poly: &[V], value: &impl Fn(&V) -> f64, k: f64) -> (Vec<V>, Vec<V>) {
    let (mut under, mut over) = (Vec::new(), Vec::new());
    for i in 0..poly.len() {
        let (a, b) = (&poly[i], &poly[(i + 1) % poly.len()]);
        let (va, vb) = (value(a) - k, value(b) - k);
        if va <= 0.0 {
            under.push(*a);
        }
        if va >= 0.0 {
            over.push(*a);
        }
        if (va < 0.0 && vb > 0.0) || (va > 0.0 && vb < 0.0) {
            let at = lerp(a, b, va / (va - vb));
            under.push(at);
            over.push(at);
        }
    }
    (under, over)
}

/// The polygon in slices one unit wide of `value`, each with its unit
/// (`floor`), from `lo` to `hi` (units outside left out).
fn slices(poly: Vec<V>, value: impl Fn(&V) -> f64, lo: i32, hi: i32) -> Vec<(i32, Vec<V>)> {
    let (min, max) = poly.iter().map(&value).fold((f64::MAX, f64::MIN), |(a, b), v| (a.min(v), b.max(v)));
    let (first, last) = ((min.floor() as i32).max(lo), (max.floor() as i32).min(hi));
    let mut out = Vec::new();
    let mut rest = poly;
    if (min.floor() as i32) < first {
        rest = split(&rest, &value, first as f64).1;
    }
    for k in first..=last {
        if rest.len() < 3 {
            break;
        }
        if k == last {
            let (under, _) = split(&rest, &value, (k + 1) as f64);
            out.push((k, under));
            break;
        }
        let (under, over) = split(&rest, &value, (k + 1) as f64);
        out.push((k, under));
        rest = over;
    }
    out
}

fn area(poly: &[V]) -> f64 {
    let mut n = glam::DVec3::ZERO;
    for i in 1..poly.len().saturating_sub(1) {
        n += (poly[i].p - poly[0].p).cross(poly[i + 1].p - poly[0].p);
    }
    n.length() * 0.5
}

fn cut_column(source: &Source, jx0: i32, jz0: i32) -> Column {
    cut_area(source, (jx0, jx0 + 15), (jz0, jz0 + 15))
}

/// The pieces of the metres from `jx0` to `hx` and `jz0` to `hz` (both
/// inclusive) of a level, every height.
pub(crate) fn cut_area(source: &Source, (jx0, hx): (i32, i32), (jz0, hz): (i32, i32)) -> Column {
    let mut out: HashMap<(i32, i32, i32), Piece> = HashMap::new();
    let mut seen = std::collections::HashSet::new();
    for tx in jx0.div_euclid(TILE)..=hx.div_euclid(TILE) {
        for tz in jz0.div_euclid(TILE)..=hz.div_euclid(TILE) {
            for &i in source.tiles.get(&(tx, tz)).into_iter().flatten() {
                if !seen.insert(i) {
                    continue;
                }
                let t = &source.tris[i as usize];
                let normal = (t.p[1] - t.p[0]).cross(t.p[2] - t.p[0]).normalize_or_zero();
                let level = normal.y.abs() > 0.7;
                let poly: Vec<V> = (0..3)
                    .map(|k| V {
                        p: t.p[k].as_dvec3(),
                        uv: t.uv[k].as_dvec2(),
                        c: glam::DVec4::from_array(t.color[k].map(f64::from)),
                    })
                    .collect();
                let owner=source.cell_owners.get(&i).copied();
                if owner.is_some_and(|c|c[0]<jx0||c[0]>hx||c[2]<jz0||c[2]>hz){continue;}
                let owned_slice=|poly:Vec<V>,axis:usize,lo:i32,hi:i32|->Vec<(i32,Vec<V>)>{
                    if let Some(c)=owner{vec![(c[axis],poly)]}else{slices(poly,|v|v.p[axis],lo,hi)}
                };
                for (x, slab) in owned_slice(poly,0,jx0,hx) {
                    for (z, bar) in owned_slice(slab,2,jz0,hz) {
                        for (y, cell) in owned_slice(bar,1,i32::MIN/2,i32::MAX/2) {
                            // Split again where a repeating texture starts over,
                            // so each part keeps to one copy of the texture.
                            let mut parts = vec![cell];
                            for (axis, repeats) in t.repeat.iter().enumerate() {
                                if !repeats {
                                    continue;
                                }
                                parts = parts
                                    .into_iter()
                                    .flat_map(|part| slices(part, move |v| v.uv[axis], i32::MIN / 2, i32::MAX / 2))
                                    .map(|(_, part)| part)
                                    .collect();
                            }
                            for part in parts {
                                if part.len() < 3 || area(&part) < 1e-12 {
                                    continue;
                                }
                                let corner = glam::DVec3::new(x as f64, y as f64, z as f64);
                                let middle = part.iter().fold(glam::DVec2::ZERO, |s, v| s + v.uv) / part.len() as f64;
                                let shift = glam::DVec2::new(
                                    if t.repeat[0] { middle.x.floor() } else { 0.0 },
                                    if t.repeat[1] { middle.y.floor() } else { 0.0 },
                                );
                                let local = |v: &V| -> ([f32; 3], [f32; 2], [u8; 4]) {
                                    let p = (v.p - corner).clamp(glam::DVec3::ZERO, glam::DVec3::ONE);
                                    let uv = (v.uv - shift).clamp(glam::DVec2::ZERO, glam::DVec2::ONE);
                                    (p.as_vec3().to_array(), uv.as_vec2().to_array(), v.c.round().to_array().map(|c| c as u8))
                                };
                                let piece = out.entry((x, y, z)).or_default();
                                for k in 1..part.len() - 1 {
                                    let (a, b, c) = (local(&part[0]), local(&part[k]), local(&part[k + 1]));
                                    piece.tris.push(PieceTri {
                                        p: [a.0, b.0, c.0],
                                        uv: [a.1, b.1, c.1],
                                        color: [a.2, b.2, c.2],
                                        texture: t.texture,
                                        blend: t.blend,
                                    });
                                }
                                if level {
                                    let top = part.iter().map(|v| v.p.y - corner.y).fold(0.0f64, f64::max).min(1.0) as f32;
                                    piece.floor = Some(piece.floor.map_or(top, |f| f.max(top)));
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    Column::new(out)
}

#[cfg(test)]
mod tests {
    #[test]
    fn architecture28_ramp_lighting_samples_above_the_real_top_surface() {
        let ramp=crate::original_places::textures_of(&crate::original_places::PLACES[1]).iter()
            .position(|name|*name=="skatea-ramp-01").unwrap() as u16;
        let source=super::source(super::ARENA_SOURCE).unwrap();
        let origin=glam::Vec3::from_array(crate::jak_city_arena::source_origin());
        // The horizontal top of the actual centre pyramid is at y7.5.
        let tops:Vec<_>=source.tris.iter().filter(|t|t.texture==ramp &&
            t.p.iter().all(|p|(p.y-origin.y-7.5).abs()<0.001)).collect();
        assert!(!tops.is_empty());
        for t in tops {
            let n=super::surface_normal(super::ARENA_SOURCE,t.texture,t.p.map(|p|p.to_array()));
            assert!(n.y>0.99,"light must come from above the exposed top: {n:?}");
        }
    }
    use super::*;
    #[test]
    fn cell_and_uv_cuts_preserve_plane_and_area_at_distant_source_coordinates() {
        let p=[Vec3::new(8000.123,1.17,8000.293),Vec3::new(8003.83,1.43,8002.83),Vec3::new(8000.43,4.15,8003.19)];
        let tri=SourceTri{p,uv:[Vec2::new(0.12,0.23),Vec2::new(7.47,0.65),Vec2::new(0.34,9.73)],texture:0,color:[[128;4];3],repeat:[true;2],blend:false};
        let source=Source::new(vec![tri]);let cut=cut_area(&source,(8000,8004),(8000,8004));
        let p=p.map(|p|p.as_dvec3());let n=(p[1]-p[0]).cross(p[2]-p[0]);let normal=n.normalize();let mut area=0.0;
        for (&(x,y,z),piece) in &cut.pieces {for t in &piece.tris {
            let q=t.p.map(|v|Vec3::from_array(v).as_dvec3()+glam::DVec3::new(x as f64,y as f64,z as f64));
            for v in q {assert!((v-p[0]).dot(normal).abs()<0.0000002,"clipping displaced a face off its shared plane");}
            area+=(q[1]-q[0]).cross(q[2]-q[0]).length()*0.5;
        }}
        assert!((area-n.length()*0.5).abs()<0.000001,"UV splitting discarded visible slivers");
    }

    // Frozen pre-hook append: deliberately independent of the shared visitor.
    fn reference_append<S: crate::scene::Scene>(scene: &S, pos: (i32,i32,i32), place: usize,
        atlas: &crate::mesh::Atlas, light: &crate::lighting::SkyLight,
        mesh: &mut crate::mesh::ChunkMesh, transparent_indices: &mut Vec<u32>) {
    let object = if place == FOLIAGE_SOURCE { crate::jak_objects::foliage_piece_at(pos) }
        else if independent_source(place) { None } else { crate::jak_objects::piece_at(pos) };
    let found = match object { Some(_) => None, None => Some(piece_at(place, pos)) };
    let place = match (&object, &found) {
        (Some((place, _)), _) => *place,
        (None, Some(Some((place, _, _)))) => *place,
        _ => return,
    };
    let regions = regions(place, atlas);
    let (x, y, z) = pos;
    let around = [(0, 0, 0), (0, 1, 0), (0, -1, 0), (1, 0, 0), (-1, 0, 0), (0, 0, 1), (0, 0, -1)];
    let sky = 15.0;
    let block = around.iter().map(|d| light.get_block((x + d.0, y + d.1, z + d.2))).max().unwrap_or(0) as f32;
    let base = [(x - mesh.origin[0]) as f32, (y - mesh.origin[1]) as f32, (z - mesh.origin[2]) as f32];
    let air = |(dx, dy, dz): (i32, i32, i32)| scene.block((x + dx, y + dy, z + dz)).is_none();
    let mut push = |p: [[f32; 3]; 3], uv: [[f32; 2]; 3], color: [[f32; 4]; 3], texture: u16, blend: bool| {
        let Some(&[u0, v0, u1, v1]) = regions.get(usize::from(texture)) else { return };
        let start = mesh.vertices.len() as u32;
        for k in 0..3 {
            let [u, v] = uv[k];
            mesh.vertices.push(crate::mesh::Vertex {
                position: [base[0] + p[k][0], base[1] + p[k][1], base[2] + p[k][2]],
                uv: [u0 + (u1 - u0) * u, v0 + (v1 - v0) * v],
                color: color[k],
                sky_light: sky,
                block_light: block,
            });
        }
        let indices = if blend { &mut *transparent_indices } else { &mut mesh.indices };
        indices.extend_from_slice(&[start, start + 1, start + 2, start, start + 2, start + 1]);
        mesh.faces += 2;
    };
    let rgba = |c: [u8; 4], shade: f32| -> [f32; 4] {
        let c = c.map(|c| (f32::from(c) / 128.0).min(1.0));
        [c[0] * shade, c[1] * shade, c[2] * shade, 1.0]
    };
    // The cube's faces: which way, corners anticlockwise, and shade.
    let faces: [((i32, i32, i32), [[f32; 3]; 4], f32); 6] = [
        ((0, 1, 0), [[0.0, 1.0, 0.0], [1.0, 1.0, 0.0], [1.0, 1.0, 1.0], [0.0, 1.0, 1.0]], 1.0),
        ((0, -1, 0), [[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [1.0, 0.0, 1.0], [0.0, 0.0, 1.0]], 0.5),
        ((0, 0, -1), [[0.0, 1.0, 0.0], [1.0, 1.0, 0.0], [1.0, 0.0, 0.0], [0.0, 0.0, 0.0]], 0.8),
        ((0, 0, 1), [[1.0, 1.0, 1.0], [0.0, 1.0, 1.0], [0.0, 0.0, 1.0], [1.0, 0.0, 1.0]], 0.8),
        ((1, 0, 0), [[1.0, 1.0, 0.0], [1.0, 1.0, 1.0], [1.0, 0.0, 1.0], [1.0, 0.0, 0.0]], 0.6),
        ((-1, 0, 0), [[0.0, 1.0, 1.0], [0.0, 1.0, 0.0], [0.0, 0.0, 0.0], [0.0, 0.0, 1.0]], 0.6),
    ];
    let square = |q: [[f32; 3]; 4], color: [f32; 4], texture: u16, push: &mut dyn FnMut([[f32; 3]; 3], [[f32; 2]; 3], [[f32; 4]; 3], u16, bool)| {
        let uv = [[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]];
        push([q[0], q[1], q[2]], [uv[0], uv[1], uv[2]], [color; 3], texture, false);
        push([q[0], q[2], q[3]], [uv[0], uv[2], uv[3]], [color; 3], texture, false);
    };
    let piece = match (&object, &found) {
        (Some((_, piece)), _) => Some(piece),
        (None, Some(Some((_, column, cell)))) => column.get(cell),
        _ => None,
    };
    let Some(piece) = piece.filter(|p| !p.tris.is_empty()) else {
        // Never invent a floor or enclosing cube for an empty circuit cell.
        if matches!(place,CIRCUIT_SOURCE|ARENA_SOURCE|LEGACY_PALACE_SOURCE|LEGACY_ARENA_SOURCE) { return; }
        let Some(Some((_, column, cell))) = &found else { return };
        // The ground under the floors: a cube of the floor over it.
        let over = (1..=48).find_map(|k| column.get(&(cell.0, cell.1 + k, cell.2)).filter(|p| p.floor.is_some()));
        let Some((texture, color)) = over.and_then(floor_look) else { return };
        for (d, q, shade) in faces {
            if air(d) {
                square(q, [color[0] * shade, color[1] * shade, color[2] * shade, 1.0], texture, &mut push);
            }
        }
        return;
    };
    // The level's ground here: the lowest floor of its metre column (not an
    // object's, not a floor with the level's open air under it).
    let ground = match &found {
        Some(Some((place, column, cell))) => object.is_none() && !crate::jak_tower::source_cell(*place,*cell)
            && !crate::jak_skate_tower::source_cell(*place,*cell)
            && !crate::jak_city_arena_legacy::source_cell(*place,*cell) && !crate::jak_city_arena_v9::source_cell(*place,*cell) && !crate::jak_city_arena::source_cell(*place,*cell) && column.is_ground(*cell),
        _ => false,
    };
    let floor_top = piece.floor.unwrap_or(1.0);
    // A building copied into a village stands under this world's sky: its
    // colours brought to the texture as it is on average (23/10: the
    // level's shade under Spargus's halls lay as black squares round them).
    let gain = if !independent_source(place) && object.is_none() && crate::jak_hamlets::source_at(pos).is_some() {
        let luma: f32 = piece.tris.iter().flat_map(|t| t.color.iter()).map(|c| 0.299 * f32::from(c[0]) + 0.587 * f32::from(c[1]) + 0.114 * f32::from(c[2])).sum::<f32>()
            / (piece.tris.len() * 3).max(1) as f32;
        if luma < 24.0 { 0.0 } else { (128.0 / luma).clamp(1.0, 4.0) }
    } else {
        1.0
    };
    for t in &piece.tris {
        let color: [[f32; 4]; 3] = std::array::from_fn(|k| {
            let c = t.color[k].map(|c| if gain == 0.0 { 1.0 } else { (f32::from(c) * gain / 128.0).min(1.0) });
            [c[0], c[1], c[2], if t.blend { (f32::from(t.color[k][3]) / 128.0).min(1.0) } else { 1.0 }]
        });
        push(t.p, t.uv, color, t.texture, t.blend);
        // The ground's floor cut down each side open to the air.
        if t.blend || !ground {
            continue;
        }
        let [a, b, c] = t.p.map(Vec3::from_array);
        if (b - a).cross(c - a).normalize_or_zero().y.abs() <= 0.7 || t.p.iter().any(|p| (p[1] - floor_top).abs() > 0.25) {
            continue;
        }
        for k in 0..3 {
            let (pa, pb) = (t.p[k], t.p[(k + 1) % 3]);
            let on = |axis: usize, at: f32| (pa[axis] - at).abs() < 1e-3 && (pb[axis] - at).abs() < 1e-3;
            let side = if on(0, 0.0) {
                Some(((-1, 0, 0), 2))
            } else if on(0, 1.0) {
                Some(((1, 0, 0), 2))
            } else if on(2, 0.0) {
                Some(((0, 0, -1), 0))
            } else if on(2, 1.0) {
                Some(((0, 0, 1), 0))
            } else {
                None
            };
            let Some((_, along)) = side.filter(|(d, _)| air(*d)) else { continue };
            let foot = |p: [f32; 3]| [p[0], 0.0, p[2]];
            let uv = |p: [f32; 3]| [p[along], 1.0 - p[1]];
            let (ca, cb) = (rgba(t.color[k], 0.7), rgba(t.color[(k + 1) % 3], 0.7));
            push([pa, pb, foot(pb)], [uv(pa), uv(pb), uv(foot(pb))], [ca, cb, cb], t.texture, false);
            push([pa, foot(pb), foot(pa)], [uv(pa), uv(foot(pb)), uv(foot(pa))], [ca, cb, ca], t.texture, false);
        }
    }
    // The ground's bottom, open to the air under it.
    if ground && air((0, -1, 0)) {
        if let Some((texture, color)) = floor_look(piece) {
            let (_, q, shade) = faces[1];
            square(q, [color[0] * shade, color[1] * shade, color[2] * shade, 1.0], texture, &mut push);
        }
    }
    }

    fn assert_outline_render_unchanged(scene: &crate::scene::HandcraftedScene, at: (i32,i32,i32), place: usize,
        atlas: &crate::mesh::Atlas, light: &crate::lighting::SkyLight) -> Vec<[[f32;3];3]> {
        let mut before = crate::mesh::ChunkMesh { origin: [at.0,at.1,at.2], ..Default::default() };
        let mut after = crate::mesh::ChunkMesh { origin: [at.0,at.1,at.2], ..Default::default() };
        let (mut before_alpha, mut after_alpha) = (Vec::new(), Vec::new());
        reference_append(scene, at, place, atlas, light, &mut before, &mut before_alpha);
        append(scene, at, place, atlas, light, &mut after, &mut after_alpha);
        assert_eq!(bytemuck::cast_slice::<crate::mesh::Vertex,u8>(&before.vertices),
            bytemuck::cast_slice::<crate::mesh::Vertex,u8>(&after.vertices), "positions, UV, colours, light and vertex order");
        assert_eq!(before.indices, after.indices); assert_eq!(before_alpha, after_alpha); assert_eq!(before.faces, after.faces);
        let drawn: Vec<_> = before.vertices.chunks_exact(3).map(|v| [v[0].position,v[1].position,v[2].position]).collect();
        let visual = visual_piece(place, at).unwrap();
        let mut selected = Vec::new();
        emit_visual_geometry::<false>(scene, at, &visual, |p,_,_,_,_| selected.push(p));
        assert_eq!(selected, drawn, "selection must visit precisely the displayed triangles");
        drawn
    }

    #[test]
    fn native_outline_triangle_respects_holes_slopes_both_sides_and_placement_face() {
        use glam::DVec3 as D;
        use minecraftoss_player::Face;
        let wall = [[0.0,0.0,0.0],[1.0,0.0,0.0],[0.0,1.0,0.0]];
        assert!(ray_triangle(D::new(0.8,0.8,-1.0),D::Z,wall).is_none(), "empty part of the proxy rectangle");
        assert_eq!(ray_triangle(D::new(0.2,0.3,-1.0),D::Z,wall),Some((1.0,Face::North)));
        assert_eq!(ray_triangle(D::new(0.2,0.3,1.0),-D::Z,wall),Some((1.0,Face::South)));
        let ramp = [[0.0,0.25,0.0],[1.0,0.75,0.0],[0.0,0.25,1.0]];
        for triangle in [ramp,[ramp[2],ramp[1],ramp[0]]] {
            assert_eq!(ray_triangle(D::new(0.25,2.0,0.25),-D::Y,triangle),Some((1.625,Face::Up)));
            assert_eq!(ray_triangle(D::new(0.25,-1.0,0.25),D::Y,triangle),Some((1.375,Face::Down)));
        }
        assert!(ray_triangle(D::ZERO,D::X,wall).is_none(), "coplanar ray has no unique surface crossing");
        assert!(ray_triangle(D::ZERO,D::Z,[[f32::NAN,0.0,0.0];3]).is_none());
        assert!(ray_triangle(D::ZERO,D::Z,[[0.0;3];3]).is_none());
    }

    #[test]
    fn native_outline_source7_matches_render_and_remains_exact_when_moved_or_removed() {
        use crate::scene::{Block,HandcraftedScene};
        use minecraftoss_player::{World as PlayerWorld,Player,Face};
        use glam::DVec3 as D;
        let cell = (4132,-1,4070); // Actual four-panel entry floor used by the physical probe.
        let packs=crate::pack::PackStack::open(Vec::new()).unwrap();
        let atlas=crate::mesh::make_atlas(&packs,Vec::new()).unwrap();
        let light=crate::lighting::SkyLight::streamed();
        let mut scene=HandcraftedScene::default();
        let mut local_hits = Vec::new();
        for at in [(32701,80,-32701),(-32701,91,32701)] {
            scene.set(at,Some(Block::new("jak:pieza_haven_circuito").with("type","bottom")));
            set_moved(at,Some((CIRCUIT_SOURCE,cell)));
            let drawn=assert_outline_render_unchanged(&scene,at,CIRCUIT_SOURCE,&atlas,&light);
            assert!(!drawn.is_empty());
            let base=D::new(f64::from(at.0),f64::from(at.1),f64::from(at.2));
            let origin=base+D::new(0.5,2.0,0.5);
            let hit=PlayerWorld::outline_hit(&scene,at,origin,-D::Y,4.5).flatten().unwrap();
            assert_eq!(hit.face,Face::Up);
            assert!(hit.point.y-base.y>0.99 && hit.point.y-base.y<1.000001, "not the bottom-slab proxy: {hit:?}");
            let mut player=Player::new(origin-D::Y*1.62);player.pitch=90.0;
            let selected=player.target(&scene,4.5).unwrap();
            assert_eq!(selected.pos,at);assert!((selected.distance-hit.distance).abs()<1e-8);
            local_hits.push((hit.distance,hit.point-base));
            scene.set(at,None);
            assert!(PlayerWorld::outline_hit(&scene,at,origin,-D::Y,4.5).is_none());
            assert!(player.target(&scene,4.5).is_none(), "removing the block also removes its selectable surface");
            set_moved(at,None);
        }
        assert_eq!(local_hits[0],local_hits[1]);
        let at=(32702,80,-32701);
        scene.set(at,Some(Block::new("jak:pieza_haven_circuito").with("type","double")));
        set_moved(at,Some((CIRCUIT_SOURCE,(4132,300,4070))));
        let origin=D::new(f64::from(at.0)+0.5,f64::from(at.1)+2.0,f64::from(at.2)+0.5);
        assert_eq!(PlayerWorld::outline_hit(&scene,at,origin,-D::Y,4.5),Some(None), "known empty native cell never falls back to a cube");
        set_moved(at,None);
    }

    #[test]
    fn native_outline_rotated_decor_uses_the_same_original_triangles_and_render_payload() {
        use crate::{jak_objects as objects,scene::{Block,HandcraftedScene}};
        use minecraftoss_player::World as PlayerWorld;
        use glam::DVec3 as D;
        let packs=crate::pack::PackStack::open(Vec::new()).unwrap();
        let atlas=crate::mesh::make_atlas(&packs,Vec::new()).unwrap();
        let light=crate::lighting::SkyLight::streamed();
        for name in ["objeto_spargus_barril_1","objeto_haven_planta_1"] {
            let kind=objects::kind_of(name).unwrap();
            let shape=objects::shape(kind).expect("original native decoration");
            for turns in 0..4 {
                let instance=objects::Instance{kind:kind as u16,origin:(32710+20*i32::from(turns),80,-32710),turns};
                assert!(objects::add(instance));
                let cells=instance.cells();
                let &(at,cell)=cells.iter().max_by_key(|(_,c)|shape.cells[c].tris.len()).unwrap();
                let mut scene=HandcraftedScene::default();
                for &(at,_) in &cells { scene.set(at,Some(Block::new(&block_of(&PLACES[shape.place])).with("type","double"))); }
                let drawn=assert_outline_render_unchanged(&scene,at,shape.place,&atlas,&light);
                let expected:Vec<_>=shape.cells[&cell].tris.iter().map(|t|t.p.map(|mut p|{
                    for _ in 0..turns { p=[1.0-p[2],p[1],p[0]]; } p
                })).collect();
                assert_eq!(drawn,expected,"decoration must be selected in its actual quarter turn");
                let triangle=drawn.iter().find(|t|{let[a,b,c]=t.map(Vec3::from_array);(b-a).cross(c-a).length_squared()>1e-10}).unwrap();
                let[a,b,c]=triangle.map(|p|D::from_array(p.map(f64::from)));
                let normal=(b-a).cross(c-a).normalize();let middle=(a+b+c)/3.0;
                let base=D::new(f64::from(at.0),f64::from(at.1),f64::from(at.2));
                let origin=base+middle+normal*0.01;
                let hit=PlayerWorld::outline_hit(&scene,at,origin,-normal,0.02).flatten().expect("visible native decor is selectable");
                assert!(hit.distance<=0.01000001);
                if name.contains("planta") { assert!(PlayerWorld::collision_boxes(&scene,at).is_empty(),"selection must not create plant collision"); }
                objects::remove_at(at).unwrap();
            }
        }
    }

    #[test]
    fn moved_sources_at_equal_coordinates_are_independent_in_each_dimension(){
        let dir=crate::test_dir::tempdir().unwrap();let pos=(31001,82,-31001);
        let overworld=(CIRCUIT_SOURCE,(4051,-7,4063));let nether=(CIRCUIT_SOURCE,(4112,-1,4068));let overlay=(TOWER_SOURCE,(3001,35,3002));
        load_moved(Some(dir.path()));set_moved(pos,Some(overworld));
        activate_moved_dimension("minecraft:the_nether");assert!(!moved().read().unwrap().current.at.contains_key(&pos));
        set_moved(pos,Some(nether));
        activate_moved_dimension("jak2:skatea");set_moved(pos,Some(overlay));
        activate_moved_dimension("minecraft:overworld");assert_eq!(source_at(CIRCUIT_SOURCE,pos),Some(overworld));
        activate_moved_dimension("minecraft:the_nether");assert_eq!(source_at(CIRCUIT_SOURCE,pos),Some(nether));
        set_moved(pos,None); // breaking at this position must not touch home
        activate_moved_dimension("minecraft:overworld");assert_eq!(source_at(CIRCUIT_SOURCE,pos),Some(overworld));
        load_moved(None);load_moved(Some(dir.path()));assert_eq!(source_at(CIRCUIT_SOURCE,pos),Some(overworld));
        activate_moved_dimension("minecraft:the_nether");assert!(!moved().read().unwrap().current.at.contains_key(&pos));
        activate_moved_dimension("jak2:skatea");assert_eq!(source_at(TOWER_SOURCE,pos),Some(overlay));
        load_moved(None);
    }

    #[test]
    fn legacy_piece_file_is_an_overworld_fallback_and_is_not_rewritten(){
        let dir=crate::test_dir::tempdir().unwrap();let legacy="[[14,71,26,1,2001,25,2002],[15,71,26,5,3001,35,3002],[16,71,26,7,4051,-7,4063]]";
        let file=dir.path().join("jak_piezas.json");std::fs::write(&file,legacy).unwrap();
        load_moved(Some(dir.path()));assert_eq!(moved().read().unwrap().current.at.len(),3);
        assert_eq!(source_at(CIRCUIT_SOURCE,(16,71,26)),Some((CIRCUIT_SOURCE,(4051,-7,4063))));
        activate_moved_dimension("minecraft:the_nether");assert!(moved().read().unwrap().current.at.is_empty());
        activate_moved_dimension("minecraft:overworld");set_moved((17,71,26),Some((CIRCUIT_SOURCE,(4112,-1,4068))));
        assert_eq!(std::fs::read_to_string(&file).unwrap(),legacy);
        assert!(dimension_piece_path(dir.path(),"minecraft:overworld").unwrap().is_file());
        load_moved(None);load_moved(Some(dir.path()));assert_eq!(moved().read().unwrap().current.at.len(),4);
        load_moved(None);
    }

    #[test]
    fn failed_atomic_piece_commit_preserves_the_complete_file_and_cleans_only_its_temp(){
        let dir=crate::test_dir::tempdir().unwrap();let path=dir.path().join("jak_piezas.json");let original="[[1,2,3,7,4051,-7,4063]]";
        std::fs::write(&path,original).unwrap();
        let result=atomic_piece_write_with(&path,"[]",|_,_|Err(std::io::Error::other("simulated commit failure")));
        assert!(result.is_err());assert_eq!(std::fs::read_to_string(&path).unwrap(),original);
        assert_eq!(std::fs::read_dir(dir.path()).unwrap().count(),1);
        atomic_piece_write_with(&path,"[]",|from,to|std::fs::rename(from,to)).unwrap();
        assert_eq!(std::fs::read_to_string(path).unwrap(),"[]");
    }

    #[test]
    fn a_failed_piece_write_retains_the_source_in_memory_until_checkpoint_retry(){
        let dir=crate::test_dir::tempdir().unwrap();let blocker=dir.path().join("dimensions");
        load_moved(Some(dir.path()));std::fs::write(&blocker,b"test obstruction").unwrap();
        let pos=(31002,83,-31002);let carried=(CIRCUIT_SOURCE,(4051,-7,4063));set_moved(pos,Some(carried));
        assert_eq!(source_at(CIRCUIT_SOURCE,pos),Some(carried));assert!(moved().read().unwrap().current.dirty);
        std::fs::remove_file(&blocker).unwrap();flush_moved().unwrap();assert!(!moved().read().unwrap().current.dirty);
        load_moved(None);load_moved(Some(dir.path()));assert_eq!(source_at(CIRCUIT_SOURCE,pos),Some(carried));load_moved(None);
    }

    #[test]
    fn unreadable_piece_metadata_is_reported_and_never_overwritten_by_an_empty_map(){
        let dir=crate::test_dir::tempdir().unwrap();let path=dimension_piece_path(dir.path(),"minecraft:overworld").unwrap();
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();std::fs::write(&path,b"broken original JSON").unwrap();
        load_moved(Some(dir.path()));assert!(moved().read().unwrap().current.load_error.is_some());
        set_moved((31003,83,-31003),Some((CIRCUIT_SOURCE,(4051,-7,4063))));assert!(flush_moved().is_err());
        assert_eq!(std::fs::read(&path).unwrap(),b"broken original JSON");
        // A damaged dimension must not block independent, healthy saves.
        activate_moved_dimension("minecraft:the_nether");set_moved((31003,83,-31003),Some((CIRCUIT_SOURCE,(4112,-1,4068))));
        assert!(dimension_piece_path(dir.path(),"minecraft:the_nether").unwrap().is_file());load_moved(None);
    }

    #[test]
    fn circuit_source_and_haven_texture_prefix_preserve_saved_addresses() {
        assert_eq!(CIRCUIT_SOURCE, crate::jak_skate_tower::SOURCE_ID);
        assert_eq!(place_of("pieza_haven_circuito"), Some(CIRCUIT_SOURCE));
        for (place, cell) in [(1,(2001,25,2002)),(TOWER_SOURCE,(3001,35,3002)),(CIRCUIT_SOURCE,(4051,-7,4063)),(LEGACY_ARENA_SOURCE,(5001,6,5002)),(LEGACY_PALACE_SOURCE,(6001,104,6002)),(ARENA_SOURCE,(8068,6,8080))] {
            let json = serde_json::to_string(&item_components(place,cell)).unwrap();
            assert_eq!(carried(Some(&serde_json::from_str(&json).unwrap())),Some((place,cell)));
        }
        for place in [-1i64, FOLIAGE_SOURCE as i64, 11, 4_294_967_303] {
            assert!(carried(Some(&serde_json::json!({"jak:pieza":[place,4000,0,4000]}))).is_none());
        }
        assert!(carried(Some(&serde_json::json!({"jak:pieza":[7,4_294_967_296i64,0,4000]}))).is_none());
        let rows=decode_moved("[[1,2,3,1,2001,25,2002],[2,2,3,5,3001,35,3002],[3,2,3,7,4051,-7,4063],[4,2,3,6,0,0,0],[5,2,3,8,0,0,0],[6,2,3,-1,0,0,0]]").unwrap();
        assert_eq!(rows.len(),4);
        assert_eq!(rows[&(5,2,3)],(LEGACY_ARENA_SOURCE as u8,(0,0,0)));
        assert_eq!(rows[&(3,2,3)],(7,(4051,-7,4063)));
        use crate::spargus_city_blocks::{HAVEN_SLUMS_TEXTURES,SPARGUS_TEXTURES,SANDOVER_TEXTURES,SNOWY_TEXTURES,BOGGY_TEXTURES};
        let legacy:Vec<_>=HAVEN_SLUMS_TEXTURES.iter().map(|t|t.texture).collect();
        let extended=original_places::textures_of(&PLACES[1]);
        assert_eq!(&extended[..legacy.len()],legacy.as_slice());
        assert_eq!(extended.iter().copied().collect::<std::collections::HashSet<_>>().len(),extended.len());
        for name in crate::jak_skate_tower::SKATE_TEXTURES {
            assert!(extended.contains(name),"{name}");
            assert!(original_places::embedded_texture(name).is_some(),"missing original skatea image: {name}");
        }
        for (place,textures) in [(0,SPARGUS_TEXTURES.as_slice()),(2,SANDOVER_TEXTURES.as_slice()),(3,SNOWY_TEXTURES.as_slice()),(4,BOGGY_TEXTURES.as_slice())] {
            let old:Vec<_>=textures.iter().map(|t|t.texture).collect();
            let current=original_places::textures_of(&PLACES[place]);
            assert_eq!(&current[..old.len()],old.as_slice(),"saved texture indices stay stable for place {place}");
            assert_eq!(current.iter().copied().collect::<std::collections::HashSet<_>>().len(),current.len());
            if place==0 {
                assert_eq!(&current[old.len()..],&["common_sandstone_taper01","common_sandstone_ground01","common_sandstone_trim01","common_sandstone_base01","city-mark-wood-plain"]);
            } else {assert_eq!(current.len(),old.len());}
        }
    }

    #[test]
    fn moved_circuit_piece_keeps_native_visuals_uvs_and_shared_collision() {
        use crate::scene::{Block,HandcraftedScene};
        let origin=crate::jak_skate_tower::source_origin();
        let tile=((origin[0] as i32+48).div_euclid(TILE)*TILE,(origin[2] as i32-10).div_euclid(TILE)*TILE);
        let visual=column(CIRCUIT_SOURCE,tile).expect("original skatea/palace fixture");
        let solid=crate::jak_skate_tower::collision_column(tile.0,tile.1).unwrap();
        let (&cell,piece)=visual.iter().filter(|(cell,p)|!p.tris.is_empty()&&solid.get(*cell).is_some_and(|p|!p.tris.is_empty()))
            .min_by_key(|(cell,_)|**cell).expect("entry has a visible and collidable surface");
        let item=item_components(CIRCUIT_SOURCE,cell);
        let carried=carried(Some(&serde_json::from_str(&serde_json::to_string(&item).unwrap()).unwrap())).unwrap();
        let first=(31201,80,-31201);let second=(-31201,91,31201);
        let packs=crate::pack::PackStack::open(Vec::new()).unwrap();
        let atlas=crate::mesh::make_atlas(&packs,Vec::new()).unwrap();
        let light=crate::lighting::SkyLight::streamed();
        let mut scene=HandcraftedScene::default();
        let block=Block::new("jak:pieza_haven_circuito").with("type","bottom");
        let mut meshes=Vec::new();
        for at in [first,second] {
            scene.set(at,Some(block.clone()));set_moved(at,Some(carried));
            let (kind,restored,source_cell)=piece_at(CIRCUIT_SOURCE,at).unwrap();
            assert_eq!((kind,source_cell),(CIRCUIT_SOURCE,cell));
            assert!(Arc::ptr_eq(&visual,&restored));
            let shape=block_collision_shape(&block,at).unwrap();
            assert!(Arc::ptr_eq(&shape,&solid[&cell].collision_boxes()));
            assert_eq!(triangles(CIRCUIT_SOURCE as u8,[at.0,at.1,at.2]),solid[&cell].tris.iter().map(|t|t.p).collect::<Vec<_>>());
            let mut mesh=crate::mesh::ChunkMesh{origin:[at.0,at.1,at.2],..Default::default()};let mut transparent=Vec::new();
            append(&scene,at,CIRCUIT_SOURCE,&atlas,&light,&mut mesh,&mut transparent);
            assert_eq!(mesh.vertices.len(),piece.tris.len()*3,"native pieces must not acquire cube sides or a synthetic bottom");
            let regions=regions(CIRCUIT_SOURCE,&atlas);
            for (tri,vertices) in piece.tris.iter().zip(mesh.vertices.chunks_exact(3)) {
                let [u0,v0,u1,v1]=regions[tri.texture as usize];
                for k in 0..3 {
                    assert_eq!(vertices[k].position,tri.p[k]);
                    assert_eq!(vertices[k].uv,[u0+(u1-u0)*tri.uv[k][0],v0+(v1-v0)*tri.uv[k][1]]);
                    let rgb=tri.color[k].map(|c|(f32::from(c)/128.0).min(1.0));
                    assert_eq!(vertices[k].color,[rgb[0],rgb[1],rgb[2],if tri.blend {rgb[3]} else {1.0}]);
                }
            }
            meshes.push((mesh,transparent));
            scene.set(at,None);set_moved(at,None);
            assert!(block_collision_shape(&block,at).unwrap().is_empty(),"no detached collision may survive removing the piece");
        }
        assert_eq!(bytemuck::cast_slice::<crate::mesh::Vertex,u8>(&meshes[0].0.vertices),bytemuck::cast_slice::<crate::mesh::Vertex,u8>(&meshes[1].0.vertices));
        assert_eq!(meshes[0].0.indices,meshes[1].0.indices);assert_eq!(meshes[0].1,meshes[1].1);
    }

    #[test]
    fn native_flag_queries_share_storage_and_preserve_conversion_semantics() {
        use crate::scene::Block;
        let at = (32659, 80, 32659);
        assert!(block_collision_shape(&Block::new("minecraft:stone"), at).is_none());
        assert!(block_collision_shape(&Block::new("jak:unregistered_piece"), at).is_none());
        let leaves = Block::new("jak:pieza_haven_follaje").with("type", "double");
        let empty = block_collision_shape(&leaves, at).unwrap();
        assert!(empty.is_empty());
        assert!(Arc::ptr_eq(&empty, &block_collision_shape(&leaves, at).unwrap()));
        assert_eq!(block_collision_boxes(&leaves, at), Some(Vec::new()));

        let column = column(1, (2000, 2000)).expect("native palace fixture");
        let (&cell, piece) = column.iter().find(|(_, p)| !p.tris.is_empty()).unwrap();
        set_moved(at, Some((1, cell)));
        let block = Block::new("jak:pieza_haven").with("type", "double");
        let shape = block_collision_shape(&block, at).unwrap();
        assert!(Arc::ptr_eq(&shape, &piece.collision_boxes()), "flags borrow the source's cached allocation");
        assert_eq!(block_collision_boxes(&block, at).unwrap(), shape.iter().map(|b| b.map(f64::from)).collect::<Vec<_>>());
        for turn in 0..4 {
            let shape = piece.rotated_collision_boxes(turn);
            let before: Vec<_> = shape.iter().map(|b| b.map(f64::from)).collect();
            assert_eq!(shape.is_empty(), before.is_empty());
            assert_eq!(shape.as_ref() == [[0.0,0.0,0.0,1.0,1.0,1.0]], before == [[0.0,0.0,0.0,1.0,1.0,1.0]]);
            assert!(Arc::ptr_eq(&shape, &piece.rotated_collision_boxes(turn)));
        }
        // An absent source cell must still select the catalog fallback;
        // it must not be confused with the explicitly empty decoration.
        set_moved(at, Some((1, (30000, 5, 30000))));
        assert!(block_collision_shape(&block, at).is_none());
        assert!(block_collision_boxes(&block, at).is_none());
        set_moved(at, None);
    }

    /// A piece taken up carries its metre in its item, and put down
    /// elsewhere shows that metre there; the move is kept in the world's
    /// folder and read back with it.
    #[test]
    fn a_moved_piece_keeps_its_metre() {
        let item = item_components(2, (10, 5, -20));
        assert_eq!(carried(Some(&item)), Some((2, (10, 5, -20))));
        assert_eq!(carried(None), None);
        let dir = std::env::temp_dir().join(format!("jak-piezas-test-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        load_moved(Some(&dir));
        set_moved((100, 70, 100), Some((2, (10, 5, -20))));
        set_moved((101,70,100),Some((1,(2001,25,2002))));
        set_moved((102,70,100),Some((TOWER_SOURCE,(3001,35,3002))));
        set_moved((103,70,100),Some((CIRCUIT_SOURCE,(4051,-7,4063))));
        assert_eq!(source_at(0, (100, 70, 100)), Some((2, (10, 5, -20))));
        load_moved(None);
        assert!(moved().read().unwrap().current.at.is_empty());
        load_moved(Some(&dir));
        assert_eq!(source_at(0, (100, 70, 100)), Some((2, (10, 5, -20))));
        assert_eq!(source_at(1,(101,70,100)),Some((1,(2001,25,2002))));
        assert_eq!(source_at(TOWER_SOURCE,(102,70,100)),Some((TOWER_SOURCE,(3001,35,3002))));
        assert_eq!(source_at(CIRCUIT_SOURCE,(103,70,100)),Some((CIRCUIT_SOURCE,(4051,-7,4063))));
        set_moved((100, 70, 100), None);
        set_moved((101,70,100),None);
        set_moved((102,70,100),None);
        set_moved((103,70,100),None);
        load_moved(Some(&dir));
        assert!(moved().read().unwrap().current.at.is_empty());
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// Sandover's inside: under its plaza's ground it is enclosed (the
    /// fill goes there), the air over the plaza is not, and nor is the air
    /// under its docks' planks over the bay.
    #[test]
    fn sandover_is_filled_only_inside() {
        // (Only with the games' places in the world, `IW4L_LUGARES_JAK=1`.)
        if !crate::original_places::jak_places_on() {
            return;
        }
        let place = PLACES.iter().position(|p| p.name == "villarrefugio").unwrap();
        let Some(inside) = enclosed(place) else {
            println!("no export, skipped");
            return;
        };
        let plaza = original_places::landmark(&PLACES[place], Some("plaza")).unwrap();
        let (x, y, z) = (plaza.x.floor() as i32, plaza.y.floor() as i32, plaza.z.floor() as i32);
        println!("plaza at {x} {y} {z}: under {} {} {}, over {} {}", inside.is_enclosed(x, y - 2, z), inside.is_enclosed(x, y - 4, z), inside.is_enclosed(x, y - 8, z), inside.is_enclosed(x, y + 2, z), inside.is_enclosed(x, y + 6, z));
        assert!(inside.is_enclosed(x, y - 4, z) || inside.is_enclosed(x, y - 8, z), "under the plaza");
        assert!(!inside.is_enclosed(x, y + 2, z) && !inside.is_enclosed(x, y + 6, z), "over the plaza");
        // The bay's open water column (its sea at 0): nothing enclosed over
        // the water in the crop's open part.
        let column = (0..=12).filter(|&k| inside.is_enclosed(100, k, 68)).count();
        println!("{column} enclosed metres over the beach landmark's column");
    }

    /// A square of Sandover cut into its metres: every piece inside its
    /// block, its texture coordinates in one copy of its texture, and the
    /// area of the level's triangles kept (none lost to the cutting).
    #[test]
    fn sandover_cuts_into_whole_pieces() {
        let place = PLACES.iter().position(|p| p.name == "villarrefugio").unwrap();
        let Some(source) = source(place) else {
            println!("no export, skipped");
            return;
        };
        let (jx0, jz0) = (0, 16);
        let started = std::time::Instant::now();
        let cut = cut_column(source, jx0, jz0);
        let tris: usize = cut.values().map(|p| p.tris.len()).sum();
        println!("{} pieces, {tris} triangles, cut in {:?}", cut.len(), started.elapsed());
        assert!(cut.len() > 200, "{}", cut.len());
        let mut kept = 0.0f32;
        for piece in cut.values() {
            for t in &piece.tris {
                for k in 0..3 {
                    assert!(t.p[k].iter().all(|c| (0.0..=1.0).contains(c)));
                    assert!(t.uv[k].iter().all(|c| (0.0..=1.0).contains(c)));
                }
                let [a, b, c] = t.p.map(Vec3::from_array);
                kept += (b - a).cross(c - a).length() * 0.5;
            }
        }
        // The level's own area inside the square.
        let mut whole = 0.0f32;
        let inside = |v: Vec3| v.x >= jx0 as f32 && v.x < (jx0 + 16) as f32 && v.z >= jz0 as f32 && v.z < (jz0 + 16) as f32;
        for t in &source.tris {
            if t.p.iter().all(|&v| inside(v)) {
                whole += (t.p[1] - t.p[0]).cross(t.p[2] - t.p[0]).length() * 0.5;
            }
        }
        println!("area kept {kept:.1} m², triangles wholly inside {whole:.1} m²");
        assert!(kept >= whole * 0.98, "{kept} of {whole}");
    }
}
