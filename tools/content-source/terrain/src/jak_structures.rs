//! The mashup's own structures in the Overworld (Adrii, 16/10): villages in
//! the manner of Jak 1's Sandover, huts of wood raised on stilts under
//! straw roofs round a plaza with its bell, joined to it by boardwalks.
//! Jak's people live in them (the bell is their meeting point, as in
//! Minecraft's villages).
//!
//! They are laid as the world is made: each chunk, as it becomes final,
//! gets the blocks of any structure over it (`chunk_map::set_decorator`).
//! Each structure stands on the raw terrain's height at its spots
//! (`base_height`), so it comes out whole whatever order its chunks are made
//! in. In some 320-block squares, on dry, fairly flat land.
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use minecraftoss_core::chunk::{Chunk, HeightmapKind};
use minecraftoss_world::chunk_map::WorldGen;

/// One village a square this size, perhaps.
const CELL: i32 = 320;
/// How many of a hundred squares have one.
const VILLAGES_IN: u64 = 35;

/// A block of a structure: where (world) and what (`minecraft:block[...]`).
pub type Placed = ((i32, i32, i32), &'static str);

/// Registers the structures with world generation (once).
pub fn install() {
    minecraftoss_world::chunk_map::set_decorator(decorate);
    minecraftoss_world::chunk_map::set_feature_preparer(crate::jak_city_trees::prepare);
}

fn mix(seed: i64, a: i32, b: i32, salt: u64) -> u64 {
    let mut h = (seed as u64) ^ ((a as u32 as u64) << 32 | u64::from(b as u32)) ^ salt.wrapping_mul(0x9E37_79B9_7F4A_7C15);
    h ^= h >> 33;
    h = h.wrapping_mul(0xff51_afd7_ed55_8ccd);
    h ^= h >> 33;
    h = h.wrapping_mul(0xc4ce_b9fe_1a85_ec53);
    h ^ (h >> 33)
}

/// A village's spot in a square (block x, z), if the square has one.
pub fn village_spot(seed: i64, cell: (i32, i32)) -> Option<(i32, i32)> {
    if mix(seed, cell.0, cell.1, 11) % 100 >= VILLAGES_IN {
        return None;
    }
    let x = cell.0 * CELL + 48 + (mix(seed, cell.0, cell.1, 12) % (CELL as u64 - 96)) as i32;
    let z = cell.1 * CELL + 48 + (mix(seed, cell.0, cell.1, 13) % (CELL as u64 - 96)) as i32;
    Some((x, z))
}

/// The ground's height at a column (`ground`) and whether it is under
/// water there (`wet`).
pub trait Land {
    fn ground(&self, x: i32, z: i32) -> i32;
    fn wet(&self, x: i32, z: i32) -> bool;
    /// Scattered eligibility probes do not need a complete survey tile.
    fn probe_column(&self,x:i32,z:i32)->(i32,bool){(self.ground(x,z),self.wet(x,z))}
    fn city_allowed(&self,_x:i32,_z:i32,_spargus:bool)->bool {true}
    /// Same eligibility at an already sampled exact raw floor; scattered
    /// probes need not materialize all 256 columns of its survey tile.
    fn probe_city_allowed(&self,x:i32,z:i32,_floor:i32,spargus:bool)->bool {self.city_allowed(x,z,spargus)}
    fn roof_allowed(&self,_x:i32,_z:i32,_roof:&str)->bool {true}
}

impl Land for WorldGen {
    fn probe_column(&self,x:i32,z:i32)->(i32,bool){
        let floor=self.terrain.base_height(x,z,HeightmapKind::OceanFloorWg);
        let surface=self.terrain.base_height(x,z,HeightmapKind::WorldSurfaceWg);
        (floor,surface>floor)
    }
    fn roof_allowed(&self,x:i32,z:i32,roof:&str)->bool {
        let here=survey_biome(self,x,z);let biomes=&self.terrain.registries.biomes;
        match roof {
            "jak:teja_nevada"=>crate::snowy::LAND.biomes.iter().any(|name|biomes.id(name)==Some(here)),
            "jak:teja_pantano"=>crate::swamp::LAND.biomes.iter().any(|name|biomes.id(name)==Some(here)),
            _=>self.city_allowed(x,z,false),
        }
    }
    fn ground(&self, x: i32, z: i32) -> i32 {
        survey_column(self,x,z)[0]
    }
    fn city_allowed(&self,x:i32,z:i32,spargus:bool)->bool {
        city_biome_allowed(self,survey_biome(self,x,z),spargus)
    }
    fn probe_city_allowed(&self,x:i32,z:i32,floor:i32,spargus:bool)->bool {
        city_biome_allowed(self,survey_biome_at(self,x,floor,z),spargus)
    }
    fn wet(&self, x: i32, z: i32) -> bool {
        let [floor,surface]=survey_column(self,x,z);surface>floor
    }
}

fn survey_biome(gen:&WorldGen,x:i32,z:i32)->minecraftoss_core::BiomeId {
    survey_biome_at(gen,x,gen.ground(x,z),z)
}

fn city_biome_allowed(gen:&WorldGen,biome:minecraftoss_core::BiomeId,spargus:bool)->bool {
    let biomes=&gen.terrain.registries.biomes;
    let desert=biomes.id("minecraft:desert")==Some(biome);
    if spargus{return desert;}
    !desert && !crate::snowy::LAND.biomes.iter().chain(crate::swamp::LAND.biomes.iter()).any(|name|biomes.id(name)==Some(biome))
}

fn survey_biome_at(gen:&WorldGen,x:i32,floor:i32,z:i32)->minecraftoss_core::BiomeId {
    type Key=(i64,i32,i32,i32);
    static KEPT:Mutex<Option<HashMap<Key,minecraftoss_core::BiomeId>>>=Mutex::new(None);
    let key=(gen.terrain.seed,x>>2,floor>>2,z>>2);
    if let Some(b)=KEPT.lock().ok().and_then(|m|m.as_ref().and_then(|m|m.get(&key).copied())) {return b;}
    let b=gen.terrain.biome_at_quart(key.1,key.2,key.3);
    if let Ok(mut m)=KEPT.lock(){let m=m.get_or_insert_with(HashMap::new);if m.len()>32768{m.clear();}m.insert(key,b);}
    b
}

type SurveyKey=(i64,i32,i32,i32,i32,i32);
type SurveyTile=Arc<std::sync::OnceLock<Vec<[i32;2]>>>;
#[derive(Default)]
struct SurveyCache {entries:HashMap<SurveyKey,(SurveyTile,u64)>,stamp:u64}
impl SurveyCache {
    fn slot(&mut self,key:SurveyKey,budget:usize)->SurveyTile {
        self.stamp=self.stamp.wrapping_add(1);
        if let Some((tile,stamp))=self.entries.get_mut(&key){*stamp=self.stamp;return tile.clone();}
        // A hit must never evict another tile. Keep in-flight initializers
        // alive so neighbouring workers share their result, even at capacity.
        while self.entries.len()>=budget {
            let old=self.entries.iter().filter(|(_, (tile,_))|Arc::strong_count(tile)==1&&tile.get().is_some())
                .min_by_key(|(_,(_,stamp))|*stamp).map(|(key,_)|*key);
            let Some(old)=old else{break;};self.entries.remove(&old);
        }
        let tile=Arc::new(std::sync::OnceLock::new());self.entries.insert(key,(tile.clone(),self.stamp));tile
    }
}
/// Exact raw heightmaps, shared across concurrent city/tower surveys.
/// 4096 tiles cover a kilometre-wide parcel search with about eight MiB
/// of height data, without evicting its hot rows between nearby offsets.
fn survey_column(gen:&WorldGen,x:i32,z:i32)->[i32;2]{
    static KEPT:Mutex<Option<SurveyCache>>=Mutex::new(None);
    let t=&gen.terrain;let key=(t.seed,t.min_y,t.height,t.sea_level,x>>4,z>>4);
    let tile=KEPT.lock().unwrap_or_else(|e|e.into_inner()).get_or_insert_with(SurveyCache::default).slot(key,4096);
    tile.get_or_init(||t.base_heightmaps(minecraftoss_core::ChunkPos::new(x>>4,z>>4)))[((z&15)*16+(x&15)) as usize]
}

#[cfg(test)]mod survey_cache_regression {
    use super::*;
    fn key(x:i32)->SurveyKey{(71,-64,384,63,x,0)}
    #[test]fn active_surveys_are_shared_and_hits_preserve_recent_tiles(){
        let mut cache=SurveyCache::default();
        let a=cache.slot(key(0),2);a.set(vec![[70,70];256]).unwrap();
        let b=cache.slot(key(1),2);b.set(vec![[80,80];256]).unwrap();drop(b);
        assert!(Arc::ptr_eq(&a,&cache.slot(key(0),2)));
        let c=cache.slot(key(2),2);assert!(cache.entries.contains_key(&key(0)));assert!(!cache.entries.contains_key(&key(1)));
        // An unfinished tile cannot be discarded or duplicated at capacity.
        let c_again=cache.slot(key(2),2);assert!(Arc::ptr_eq(&c,&c_again));
        assert_eq!(cache.entries.len(),2);assert_eq!(a.get().unwrap()[0],[70,70]);
    }
}

/// A village at `(x, z)`, its houses of Haven's slums (Adrii, 23/10:
/// `jak_facades`) round a plaza with its bell: its blocks, or none where
/// the land will not take one (water, steep ground).
pub fn village(seed: i64, cell: (i32, i32), land: &dyn Land) -> Option<Vec<Placed>> {
    village_roofed(seed, cell, land, ("jak:teja_roja", "jak:cumbrera_roja"))
}

/// The kind of village a square has: in Jak 1's lands Sandover-style (its
/// houses under the land's tiles: Adrii 24/10, "un pueblo que pueda
/// aparecer en biomas de jak 1 en plan sandover"), elsewhere Jak II's
/// slums (`jak_barrio`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Style {
    Sandover(&'static str, &'static str),
    Slums,
}

/// A square's village of `style`.
pub fn village_styled(seed: i64, cell: (i32, i32), land: &dyn Land, style: Style) -> Option<Vec<Placed>> {
    match style {
        Style::Sandover(tiles, ridge) => village_roofed(seed, cell, land, (tiles, ridge)),
        Style::Slums => {
            let layout=slums_layout(seed,cell,land)?;
            Some(crate::jak_barrio::village_from_layout(seed,(*layout).clone(),land))
        }
    }
}

/// City eligibility and roads need its exact layout, not millions of
/// decorative block writes. Keep this small shared result independently of
/// the bounded, much larger cache of finished district geometry.
fn slums_layout(seed:i64,cell:(i32,i32),land:&dyn Land)->Option<Arc<crate::jak_town::Town>> {
    type Slot=Arc<std::sync::OnceLock<Option<Arc<crate::jak_town::Town>>>>;
    static CACHE:Mutex<Option<HashMap<(i64,(i32,i32)),Slot>>>=Mutex::new(None);
    let slot={let mut held=CACHE.lock().unwrap_or_else(|e|e.into_inner());let map=held.get_or_insert_with(HashMap::new);
        if map.len()>=128{let old=map.iter().find(|(k,s)|**k!=(seed,cell)&&Arc::strong_count(s)==1&&s.get().is_some()).map(|(k,_)|*k);if let Some(k)=old{map.remove(&k);}}
        map.entry((seed,cell)).or_default().clone()};
    slot.get_or_init(||{
            let (cx, cz) = village_spot(seed, cell)?;
            let r = 16;
            let probes = [(0, 0), (r, 0), (-r, 0), (0, r), (0, -r), (r, r), (-r, -r), (r, -r), (-r, r)];
            if probes.iter().any(|&(dx, dz)| land.probe_column(cx + dx, cz + dz).1) {
                return None;
            }
            let heights: Vec<i32> = probes.iter().map(|&(dx, dz)| land.probe_column(cx + dx, cz + dz).0).collect();
            if heights.iter().max()? - heights.iter().min()? > 16 {
                return None;
            }
            let y0 = land.ground(cx, cz);
            let turn = (mix(seed, cell.0, cell.1, 60) % 4) as u8;
            crate::jak_barrio::natural_layout(seed,(cx,cz),y0,turn,land).map(Arc::new)
    }).clone()
}
/// Shared by native chunk placement and the distant silhouette worker.
pub(crate) fn slums_palace(seed:i64,cell:(i32,i32),land:&dyn Land)->Option<(i32,i32,i32)>{
    slums_layout(seed,cell,land)?.palace
}
pub(crate) fn slums_exist(seed:i64,cell:(i32,i32),land:&dyn Land)->bool {
    slums_layout(seed,cell,land).is_some()
}


/// Only the six through-street endpoints used by the tower planner. These
/// are sampled from the same graded layout/paving rule as Town::terrain;
/// surveying a distant tower must not construct a whole decorated district.
pub(crate) fn slums_road_anchors(seed:i64,cell:(i32,i32),land:&dyn Land)->Option<Arc<Vec<Placed>>>{
    let town=slums_layout(seed,cell,land)?;let site=town.site();
    let o=site.at(0,0,0);let a=site.at(1,0,0);let b=site.at(0,0,1);
    let mut out=Vec::new();
    const LAYERS:[&str;7]=["jak:barrio_losa_capa[layers=1]","jak:barrio_losa_capa[layers=2]","jak:barrio_losa_capa[layers=3]","jak:barrio_losa_capa[layers=4]","jak:barrio_losa_capa[layers=5]","jak:barrio_losa_capa[layers=6]","jak:barrio_losa_capa[layers=7]"];
    for dx in [-56,56]{for dz in [-28,0,28]{
        let(wx,wz)=(o.0+dx,o.2+dz);
        let(x,z)=(dx*(a.0-o.0)+dz*(a.2-o.2),dx*(b.0-o.0)+dz*(b.2-o.2));
        let coverage=town.coverage(x,z);
        if coverage<0.02||!land.city_allowed(wx,wz,false){continue;}
        let wet=land.wet(wx,wz);if wet&&coverage<=0.75{continue;}
        let eighths=town.eighths(x,z);let y=eighths.div_euclid(8);let rem=eighths.rem_euclid(8);
        if town.palace_owns((wx,y-1,wz)){continue;}
        let noise=((wx as i64).wrapping_mul(73_856_093)^(wz as i64).wrapping_mul(19_349_663)).rem_euclid(100)as f32/100.;
        if wet||coverage>=0.9||noise<coverage{out.push(((wx,y-1,wz),"jak:barrio_losa"));}
        if rem>0{out.push(((wx,y,wz),LAYERS[(rem-1)as usize]));}
    }}
    Some(Arc::new(out))
}

/// The nearest town of Jak II's slums to `(x, z)` within `reach` (tests,
/// `mc barrio`): its plaza's middle on the ground.
pub fn nearest_slums(registries: Arc<minecraftoss_core::Registries>, seed: i64, (x, z): (i32, i32), reach: i32) -> Option<(i32, i32, i32)> {
    let terrain = Arc::new(minecraftoss_generator::terrain::TerrainGenerator::overworld(registries, seed).ok()?);
    let gen = WorldGen::new(terrain).ok()?;
    let (cx, cz) = (x.div_euclid(CELL), z.div_euclid(CELL));
    let rings = reach / CELL + 1;
    (0..=rings).find_map(|ring| {
        let mut found = Vec::new();
        for dx in -ring..=ring {
            for dz in -ring..=ring {
                if dx.abs().max(dz.abs()) != ring {
                    continue;
                }
                let cell = (cx + dx, cz + dz);
                let Some((vx, vz)) = village_spot(seed, cell) else { continue };
                if crate::wasteland::is_wasteland(&gen, vx, vz)
                    || crate::jak_lands::is_in(&crate::snowy::LAND, &gen, vx, vz)
                    || crate::jak_lands::is_in(&crate::swamp::LAND, &gen, vx, vz)
                {
                    continue;
                }
                if village_cached(seed, cell, &gen, Style::Slums).is_some() {
                    found.push((vx, gen.ground(vx, vz), vz));
                }
            }
        }
        found.into_iter().min_by_key(|p| (p.0 - x).pow(2) + (p.2 - z).pow(2))
    })
}

/// [`village`] with its houses under `roof`'s tiles and ridge.
fn village_roofed(seed: i64, cell: (i32, i32), land: &dyn Land, roof: (&'static str, &'static str)) -> Option<Vec<Placed>> {
    let (cx, cz) = village_spot(seed, cell)?;
    if !(-10..=10).all(|dx|(-10..=10).all(|dz|land.roof_allowed(cx+dx,cz+dz,roof.0))) {return None;}
    // Dry and fairly flat about the plaza.
    let probes = [(0, 0), (12, 0), (-12, 0), (0, 12), (0, -12), (18, 18), (-18, -18), (18, -18), (-18, 18)];
    let heights: Vec<i32> = probes.iter().map(|&(dx, dz)| land.probe_column(cx + dx, cz + dz).0).collect();
    if probes.iter().any(|&(dx, dz)| land.probe_column(cx + dx, cz + dz).1) {
        return None;
    }
    let (lo, hi) = (*heights.iter().min()?, *heights.iter().max()?);
    if hi - lo > 7 {
        return None;
    }
    let y0 = land.ground(cx, cz);
    let mut out: Vec<Placed> = Vec::new();
    // The plaza: a circle of Haven's flagstones (Adrii's picture, 24/10),
    // their worn hollows kept, cleared above, its bell on a stone pedestal
    // in the middle.
    for dx in -9i32..=9 {
        for dz in -9i32..=9 {
            if dx * dx + dz * dz > 81 {
                continue;
            }
            let (x, z) = (cx + dx, cz + dz);
            out.push(((x, y0 - 2, z), "minecraft:dirt"));
            out.push(((x, y0 - 1, z), crate::jak_families::haven_paving(0, x, z)));
            for y in y0..y0 + 5 {
                out.push(((x, y, z), "minecraft:air"));
            }
        }
    }
    out.push(((cx, y0, cz), "minecraft:chiseled_stone_bricks"));
    out.push(((cx, y0 + 1, cz), "minecraft:bell[attachment=floor,facing=north]"));
    // As the picture's street: beds of red earth sunk in low stone curbs
    // with tufts of grass, pillars of stones with lanterns, stone posts.
    let mut street = crate::jak_families::Plan::default();
    let turn = mix(seed, cell.0, cell.1, 21) % 2 == 0;
    for (k, (dx, dz, w, d)) in [(-7, 2, 5, 3), (3, -7, 4, 3), (2, 4, 3, 4)].into_iter().enumerate() {
        let (dx, dz, w, d) = if turn { (dz, dx, d, w) } else { (dx, dz, w, d) };
        crate::jak_haven::bed(&mut street, (cx + dx, cz + dz), (w, d), y0, mix(seed, cell.0, cell.1, 22 + k as u64));
    }
    for (dx, dz) in [(5, 5), (-5, 5), (5, -5), (-5, -5)] {
        crate::jak_haven::lantern_pillar(&mut street, (cx + dx, cz + dz), y0);
    }
    for (dx, dz) in [(8, 1), (-1, 8), (-8, -2), (1, -8)] {
        street.fixed((cx + dx, y0, cz + dz), "jak:haven_sillar");
    }
    out.extend(street.finish());
    // The huts round it, each with its boardwalk to the plaza.
    let huts = 4 + (mix(seed, cell.0, cell.1, 14) % 3) as usize;
    for k in 0..huts {
        let angle = k as f32 / huts as f32 * std::f32::consts::TAU + (mix(seed, cell.0, cell.1, 15) % 100) as f32 * 0.01;
        let reach = 18.0 + (mix(seed, cell.0 + k as i32, cell.1, 16) % 4) as f32;
        let hx = cx + (angle.cos() * reach).round() as i32;
        let hz = cz + (angle.sin() * reach).round() as i32;
        if land.wet(hx, hz) {
            continue;
        }
        let ground = land.ground(hx, hz);
        // A house of Haven's slums (`jak_facades`), its front to the plaza.
        let front = if (cx - hx).abs() > (cz - hz).abs() {
            if cx > hx { 1 } else { 3 }
        } else if cz > hz {
            2
        } else {
            0
        };
        // Villa Refugio's three: the main house first, then the tall one
        // and the one with a balcony and a stall (`jak_haven`).
        let kind = ((k as u64 + mix(seed, cell.0, cell.1, 17)) % 3) as u8;
        let mut made=crate::jak_haven::building_roofed((hx, ground, hz), front, if k == 0 { 0 } else { kind }, roof);
        boardwalk(&mut made, land, (cx, cz), y0, (hx, hz), ground);
        if made.iter().all(|((x,_,z),_)|land.roof_allowed(*x,*z,roof.0)) {out.extend(made);}
    }
    Some(out)
}

/// Block states built at run time, kept for good (a few dozen at most).
fn leak(state: String) -> &'static str {
    static KEPT: Mutex<Vec<&'static str>> = Mutex::new(Vec::new());
    let mut kept = KEPT.lock().unwrap_or_else(|e| e.into_inner());
    if let Some(found) = kept.iter().find(|s| **s == state) {
        return found;
    }
    let state: &'static str = Box::leak(state.into_boxed_str());
    kept.push(state);
    state
}

/// A boardwalk two planks wide from the plaza's edge to a hut's steps, on
/// the ground (on stilts over a dip).
fn boardwalk(out: &mut Vec<Placed>, land: &dyn Land, from: (i32, i32), y0: i32, to: (i32, i32), hut_ground: i32) {
    let (dx, dz) = ((to.0 - from.0) as f32, (to.1 - from.1) as f32);
    let length = dx.hypot(dz);
    let steps = length as i32;
    for i in 6..steps - 4 {
        let t = i as f32 / length;
        let x = from.0 + (dx * t).round() as i32;
        let z = from.1 + (dz * t).round() as i32;
        let level = y0 + ((hut_ground - y0) as f32 * t).round() as i32;
        let side = if dx.abs() > dz.abs() { (0, 1) } else { (1, 0) };
        // A street of stones, three wide, on stone over any dip.
        for k in -1..=1 {
            let (px, pz) = (x + side.0 * k, z + side.1 * k);
            out.push(((px, level - 1, pz), crate::jak_families::haven_paving(0, px, pz)));
            out.push(((px, level, pz), "minecraft:air"));
            out.push(((px, level + 1, pz), "minecraft:air"));
            let under = land.ground(px, pz);
            for yy in under..level - 1 {
                out.push(((px, yy, pz), "minecraft:stone_bricks"));
            }
        }
    }
}

/// One crater cave a square this size, perhaps; how many of a hundred.
const CAVE_CELL: i32 = 448;
const CAVES_IN: u64 = 30;
/// The block a crater cave's chamber marks its enemies' spot with (on its
/// floor, under them): Jak's people's marks find it.
pub const CAVE_MARK: &str = "minecraft:chiseled_polished_blackstone";
/// How far east of its crater the chamber's middle is (blocks).
const CHAMBER: i32 = 40;

/// A crater cave's spot in a square (block x, z), if the square has one.
pub fn cave_spot(seed: i64, cell: (i32, i32)) -> Option<(i32, i32)> {
    if mix(seed, cell.0, cell.1, 31) % 100 >= CAVES_IN {
        return None;
    }
    let x = cell.0 * CAVE_CELL + 64 + (mix(seed, cell.0, cell.1, 32) % (CAVE_CELL as u64 - 128)) as i32;
    let z = cell.1 * CAVE_CELL + 64 + (mix(seed, cell.0, cell.1, 33) % (CAVE_CELL as u64 - 128)) as i32;
    Some((x, z))
}

/// Where its chest stands, for its loot (world block).
fn cave_chest(seed: i64, cell: (i32, i32), land: &dyn Land) -> Option<(i32, i32, i32)> {
    let (cx, cz) = cave_spot(seed, cell)?;
    let floor = land.ground(cx, cz) - 3 - 22;
    Some((cx + CHAMBER, floor, cz + 7))
}

/// A crater cave in the manner of Jak 1's Volcanic Crater: a crater of
/// blackstone and basalt with magma on its floor, a tunnel down from it to
/// a chamber lined with blackstone with bedrock behind, lava pools in it,
/// its chest of Jak's things and its enemies' spot.
pub fn crater_cave(seed: i64, cell: (i32, i32), land: &dyn Land) -> Option<Vec<Placed>> {
    let (cx, cz) = cave_spot(seed, cell)?;
    let probes = [(0, 0), (10, 0), (-10, 0), (0, 10), (0, -10), (CHAMBER, 0)];
    if probes.iter().any(|&(dx, dz)| land.probe_column(cx + dx, cz + dz).1) {
        return None;
    }
    let heights: Vec<i32> = probes.iter().map(|&(dx, dz)| land.probe_column(cx + dx, cz + dz).0).collect();
    if heights.iter().max()? - heights.iter().min()? > 8 {
        return None;
    }
    let y0 = land.ground(cx, cz);
    let mut out: Vec<Placed> = Vec::new();
    // The crater: a bowl three deep, its rim of blackstone and basalt.
    for dx in -10i32..=10 {
        for dz in -10i32..=10 {
            let r2 = dx * dx + dz * dz;
            let (x, z) = (cx + dx, cz + dz);
            if r2 <= 49 {
                for y in y0 - 3..y0 + 6 {
                    out.push(((x, y, z), "minecraft:air"));
                }
                let floor = if (dx * 7 + dz * 3).rem_euclid(5) == 0 { "minecraft:magma_block" } else { "minecraft:basalt" };
                out.push(((x, y0 - 4, z), floor));
            } else if r2 <= 100 {
                let ground = land.ground(x, z);
                let rim = if (dx + dz).rem_euclid(3) == 0 { "minecraft:basalt" } else { "minecraft:blackstone" };
                for y in ground - 2..ground + 1 + i32::from(r2 <= 81) {
                    out.push(((x, y, z), rim));
                }
            }
        }
    }
    let floor = y0 - 3 - 22;
    let run = CHAMBER - 10 - 3;
    // The chamber at its end: a dome of air, then blackstone, then two of
    // bedrock behind it.
    let (hx, hz) = (cx + CHAMBER, cz);
    for dx in -12i32..=12 {
        for dz in -12i32..=12 {
            for dy in -4i32..=10 {
                let d = ((dx * dx + dz * dz) as f32 / 81.0 + (dy * dy) as f32 / 49.0).sqrt();
                let at = (hx + dx, floor + dy, hz + dz);
                let block = if d < 1.0 {
                    if dy < 0 { "minecraft:basalt" } else { "minecraft:air" }
                } else if d < 1.12 {
                    "minecraft:blackstone"
                } else if d < 1.34 {
                    "minecraft:bedrock"
                } else {
                    continue;
                };
                out.push((at, block));
            }
        }
    }
    // The tunnel (after the chamber, through its shell): from the crater
    // floor east and down to the chamber's
    // wall at its floor, three wide and four high, lined with blackstone.
    for step in 0..=run {
        let x = cx + 3 + step;
        let base = (y0 - 3 - (step * 22 / run)).max(floor);
        for dz in -2i32..=2 {
            for dy in -1i32..=4 {
                let edge = dz.abs() == 2 || dy == -1 || dy == 4;
                out.push(((x, base + dy, cz + dz), if edge { "minecraft:blackstone" } else { "minecraft:air" }));
            }
        }
        if step % 6 == 3 {
            out.push(((x, base + 3, cz), "minecraft:lantern[hanging=true]"));
        }
    }
    // The tunnel's mouth into it, cut again through the shell from where
    // the tunnel reaches the chamber's floor, floored with basalt.
    for x in cx + 3 + run..hx - 6 {
        for dz in -1i32..=1 {
            out.push(((x, floor - 1, hz + dz), "minecraft:basalt"));
            for dy in 0..=3 {
                out.push(((x, floor + dy, hz + dz), "minecraft:air"));
            }
        }
    }
    // Its floor: basalt with magma, two lava pools held in blackstone.
    for dx in -8i32..=8 {
        for dz in -8i32..=8 {
            if dx * dx + dz * dz > 64 {
                continue;
            }
            let at = (hx + dx, floor - 1, hz + dz);
            out.push((at, if (dx * 5 + dz * 11).rem_euclid(7) == 0 { "minecraft:magma_block" } else { "minecraft:basalt" }));
        }
    }
    for (px, pz) in [(-5, -4), (4, -5)] {
        for dx in -1i32..=1 {
            for dz in -1i32..=1 {
                out.push(((hx + px + dx, floor - 2, hz + pz + dz), "minecraft:blackstone"));
                out.push(((hx + px + dx, floor - 1, hz + pz + dz), "minecraft:lava"));
            }
        }
    }
    // The enemies' spot and the chest (its loot is the chunk's).
    out.push(((hx, floor - 1, hz), CAVE_MARK));
    out.push(((hx, floor, hz + 7), "minecraft:chest[facing=north]"));
    out.push(((hx + 1, floor, hz + 7), "minecraft:lantern"));
    Some(out)
}

/// The crater cave chest's things: his guns' ammo in its colours (three of
/// Jak 3's pickups of each, the red's and the purple's smaller), dark eco,
/// one of his guns and an eco vent, as a chest's `Items`.
fn cave_loot(seed: i64, cell: (i32, i32)) -> minecraftoss_core::nbt::Tag {
    use minecraftoss_core::nbt::Tag;
    const GUNS: [&str; 6] = ["jak:beam_reflexor", "jak:arc_wielder", "jak:plasmite_rpg", "jak:gyro_burster", "jak:needle_lazer", "jak:mass_inverter"];
    const VENTS: [&str; 4] = ["jak:vent_red", "jak:vent_yellow", "jak:vent_blue", "jak:vent_green"];
    let pick = |salt: u64, n: usize| (mix(seed, cell.0, cell.1, salt) % n as u64) as usize;
    let items = [
        (0u8, "jak:dark_eco", 8u8),
        (2, "jak:dark_ammo", 3),
        (4, "jak:yellow_ammo", 30),
        (8, "jak:red_ammo", 15),
        (11, GUNS[pick(41, GUNS.len())], 1),
        (13, "jak:blue_ammo", 30),
        (16, VENTS[pick(42, VENTS.len())], 1),
        (22, "jak:green_eco", 12),
    ];
    let list = items
        .iter()
        .map(|&(slot, id, count)| {
            let mut m = std::collections::BTreeMap::new();
            m.insert("Slot".to_owned(), Tag::Byte(slot as i8));
            m.insert("id".to_owned(), Tag::String(id.to_owned()));
            m.insert("count".to_owned(), Tag::Int(i32::from(count)));
            Tag::Compound(m)
        })
        .collect();
    let mut chest = std::collections::BTreeMap::new();
    chest.insert("id".to_owned(), Tag::String("minecraft:chest".to_owned()));
    chest.insert("Items".to_owned(), Tag::List(list));
    Tag::Compound(chest)
}

type PlanKey=(i64,(i32,i32),Style);
type PlanSlot=Arc<std::sync::OnceLock<Option<Arc<SettlementPlan>>>>;
pub(crate) struct SettlementPlan {
    pub blocks:Arc<Vec<Placed>>,
    pub trees:std::sync::OnceLock<crate::jak_city_trees::RootArea>,
}
impl SettlementPlan {
    fn new(blocks:Vec<Placed>)->Self{Self{blocks:Arc::new(blocks),trees:Default::default()}}
    fn bytes(&self)->usize{self.blocks.capacity()*std::mem::size_of::<Placed>()+self.trees.get().map_or(0,|t|t.bytes())}
}
#[derive(Default)]
struct PlanCache {entries:HashMap<PlanKey,(PlanSlot,u64)>,stamp:u64}
impl PlanCache {
    fn trim(&mut self,keep:PlanKey,budget:usize){
        loop {
            let bytes:usize=self.entries.values().filter_map(|(s,_)|s.get().and_then(Option::as_ref)).map(|p|p.bytes()).sum();
            if self.entries.len()<=64&&bytes<=budget{break;}
            // Never evict a plan being built or handed to another worker.
            let victim=self.entries.iter().filter(|(key,(s,_))|**key!=keep&&Arc::strong_count(s)==1
                &&s.get().is_some_and(|p|p.as_ref().is_none_or(|p|Arc::strong_count(p)==1&&Arc::strong_count(&p.blocks)==1)))
                .min_by_key(|(_,(_,stamp))|*stamp).map(|(key,_)|*key);
            let Some(victim)=victim else{break;};self.entries.remove(&victim);
        }
    }
}
type Kept=Mutex<Option<PlanCache>>;
fn cache_plan(cache:&Kept,key:PlanKey,make:impl FnOnce()->Option<Vec<Placed>>)->Option<Arc<SettlementPlan>> {
    let slot={let mut held=cache.lock().unwrap_or_else(|e|e.into_inner());let cache=held.get_or_insert_with(PlanCache::default);
        cache.stamp=cache.stamp.wrapping_add(1);let stamp=cache.stamp;
        let entry=cache.entries.entry(key).or_insert_with(||(Arc::new(std::sync::OnceLock::new()),stamp));entry.1=stamp;entry.0.clone()};
    let result=slot.get_or_init(||make().map(|b|Arc::new(SettlementPlan::new(b)))).clone();
    {let mut held=cache.lock().unwrap_or_else(|e|e.into_inner());held.as_mut().unwrap().trim(key,256*1024*1024);}
    result
}
/// One shared initializer per settlement, with a bounded LRU cache. Generation
/// stays outside the map lock; neighbouring workers wait for this one result.
pub(crate) fn settlement_plan(seed:i64,cell:(i32,i32),land:&dyn Land,cave:bool,style:Style)->Option<Arc<SettlementPlan>> {
    static KEPT:Kept=Mutex::new(None);static CAVES:Kept=Mutex::new(None);
    cache_plan(if cave{&CAVES}else{&KEPT},(seed,cell,style),||if cave{crater_cave(seed,cell,land)}else{village_styled(seed,cell,land,style)})
}
/// The village of a square, made once (chunks ask many times).
pub(crate) fn village_cached(seed:i64,cell:(i32,i32),land:&dyn Land,style:Style)->Option<Arc<Vec<Placed>>>{cached_as(seed,cell,land,false,style)}
fn cached(seed:i64,cell:(i32,i32),land:&dyn Land,cave:bool)->Option<Arc<Vec<Placed>>>{cached_as(seed,cell,land,cave,Style::Slums)}
fn cached_as(seed:i64,cell:(i32,i32),land:&dyn Land,cave:bool,style:Style)->Option<Arc<Vec<Placed>>>{
    settlement_plan(seed,cell,land,cave,style).map(|p|p.blocks.clone())
}

#[cfg(test)]mod plan_cache_tests {
    use super::*;
    #[test]fn concurrent_neighbour_chunks_build_one_shared_plan(){
        use std::sync::{Barrier,atomic::{AtomicUsize,Ordering}};
        let cache=Arc::new(Mutex::new(None));let barrier=Arc::new(Barrier::new(12));let made=Arc::new(AtomicUsize::new(0));
        let workers:Vec<_>=(0..12).map(|_|{let(c,b,n)=(cache.clone(),barrier.clone(),made.clone());std::thread::spawn(move||{
            b.wait();cache_plan(&c,(777,(4,-3),Style::Slums),||{n.fetch_add(1,Ordering::Relaxed);std::thread::sleep(std::time::Duration::from_millis(10));Some(vec![((1,70,-1),"minecraft:air")])}).unwrap()
        })}).collect();
        let plans:Vec<_>=workers.into_iter().map(|w|w.join().unwrap()).collect();assert_eq!(made.load(Ordering::Relaxed),1);
        assert!(plans.iter().all(|p|Arc::ptr_eq(p,&plans[0])));
    }
    #[test]fn cache_reclaims_old_unused_plans_by_memory_and_preserves_active_owners(){
        let cache=Mutex::new(None);let first=(1,(0,0),Style::Slums);let second=(1,(1,0),Style::Slums);
        let held=cache_plan(&cache,first,||Some(vec![((0,70,0),"minecraft:air");4096])).unwrap();
        cache_plan(&cache,second,||Some(vec![((0,70,0),"minecraft:air");4096]));
        cache.lock().unwrap().as_mut().unwrap().trim(second,0);assert!(cache.lock().unwrap().as_ref().unwrap().entries.contains_key(&first),"an active owner retains its plan");
        drop(held);cache.lock().unwrap().as_mut().unwrap().trim(second,0);
        let map=cache.lock().unwrap();let map=map.as_ref().unwrap();assert!(!map.entries.contains_key(&first));assert!(map.entries.contains_key(&second));
    }
}

/// The structures' blocks over one chunk, written into it.
fn decorate(chunk: &mut Chunk, gen: &WorldGen) {
    if !gen.terrain.host_structures { return; }
    // These settlements belong only to their natural Overworld biomes.
    // A lit/custom dimension or the old void overlay is not an Overworld.
    if !natural_jak_world(gen) {
        return;
    }
    let seed = gen.terrain.seed;
    let (x0, z0) = (chunk.pos.x * 16, chunk.pos.z * 16);
    let cell_of = |v: i32| v.div_euclid(CELL);
    let mut cells = Vec::new();
    for cx in cell_of(x0 - crate::jak_town::REACH - 8)..=cell_of(x0 + crate::jak_town::REACH + 23) {
        for cz in cell_of(z0 - crate::jak_town::REACH - 8)..=cell_of(z0 + crate::jak_town::REACH + 23) {
            cells.push((cx, cz));
        }
    }
    let registries = gen.terrain.registries.clone();
    let mut states: HashMap<&'static str, minecraftoss_core::block::BlockStateId> = HashMap::new();
    // The deserts are Jak 3's Wasteland (its ground, plants, rocks and
    // Spargus outposts).
    crate::wasteland::decorate(chunk, gen, &mut states);
    // The snowy lands are Jak 1's Snowy Mountain, the swamps its Boggy
    // Swamp (`jak_lands`).
    crate::snowy::decorate(chunk, gen, &mut states);
    crate::swamp::decorate(chunk, gen, &mut states);
    for cell in cells {
        let Some((vx, vz)) = village_spot(seed, cell) else { continue };
        // No Sandover in the Wasteland (its outposts are Spargus's).
        if crate::wasteland::is_wasteland(gen, vx, vz) {
            continue;
        }
        // Only villages whose reach (40 blocks) meets this chunk.
        if (vx - (x0 + 8)).abs() > crate::jak_town::REACH + 14 || (vz - (z0 + 8)).abs() > crate::jak_town::REACH + 14 {
            continue;
        }
        let style = if crate::jak_lands::is_in(&crate::snowy::LAND, gen, vx, vz) {
            Style::Sandover("jak:teja_nevada", "jak:cumbrera_nevada")
        } else if crate::jak_lands::is_in(&crate::swamp::LAND, gen, vx, vz) {
            Style::Sandover("jak:teja_pantano", "jak:cumbrera_pantano")
        } else {
            Style::Slums
        };
        let game = if matches!(style, Style::Slums) { crate::jak_loot::Game::Jak2 } else { crate::jak_loot::Game::Jak1 };
        let Some(blocks) = village_cached(seed, cell, gen, style) else { continue };
        // Its few Minecraft villagers (none to two), in the plaza's chunk.
        let catalog = registries.entities.as_ref().filter(|_| (x0..x0 + 16).contains(&vx) && (z0..z0 + 16).contains(&vz));
        if let Some(catalog) = catalog {
            let ground = gen.ground(vx, vz);
            for k in 0..(mix(seed, cell.0, cell.1, 17) % 3) as i32 {
                let Some(mut tag) = catalog.default_tag("minecraft:villager") else { break };
                let at = if let Some(c)=slums_palace(seed,cell,gen){
                    [f64::from(c.0-5+2*k)+0.5,f64::from(c.1),f64::from(c.2+67)+0.5]
                }else{[f64::from(vx+2-4*k)+0.5,f64::from(ground),f64::from(vz+3)+0.5]};
                let id = mix(seed, cell.0, cell.1, 18 + k as u64);
                let uuid = [(id >> 32) as i32, id as i32, (id >> 16) as i32 ^ 0x4a41_4b, k + 1];
                minecraftoss_core::entity_data::place(&mut tag, at, (k * 90) as f32, 0.0, uuid);
                chunk.generation.entities.push(tag);
            }
        }
        clear_city_trees(chunk,&blocks,&registries);
        for &((x, y, z), state) in blocks.iter() {
            if x < x0 || x >= x0 + 16 || z < z0 || z >= z0 + 16 {
                continue;
            }
            let id = *states.entry(state).or_insert_with(|| {
                registries.blocks.parse_state(state).unwrap_or(minecraftoss_core::block::BlockStateId::AIR)
            });
            chunk.set_block_raw((x - x0) as usize, y, (z - z0) as usize, id, &registries);
            crate::jak_loot::fill(chunk, (x, y, z), state, game, seed);
        }
    }
    decorate_caves(chunk, gen, &mut states);
    crate::jak_skate_tower_site::decorate(chunk, gen, &mut states);
    // The games' own places (Spargus, Haven's slums, Sandover, Snowy
    // Mountain, Boggy Swamp), over all of it where they stand: only with
    // `IW4L_LUGARES_JAK=1`.
    if crate::original_places::jak_places_on() {
        crate::original_places::place(chunk, gen, &mut states);
    }
}

pub(crate) fn natural_jak_world(gen:&WorldGen)->bool{
    gen.dimension_type=="minecraft:overworld"&&!gen.terrain.void
}

/// Remove whole high canopies above the clearing, including their overhang.
/// Trunks outside it remain, so clearing leaves cannot create new floating
/// crowns beyond the perimeter. Run before placing authored timber.
pub(crate) fn clear_city_trees(chunk:&mut Chunk,blocks:&[Placed],registries:&minecraftoss_core::Registries){
    const PAD:i32=6;const W:usize=28;
    let (x0,z0)=(chunk.pos.x*16,chunk.pos.z*16);
    let mut floors=[320i32;W*W];
    for &((x,y,z),state) in blocks {if state=="minecraft:air" && x>=x0-PAD && x<x0+16+PAD && z>=z0-PAD && z<z0+16+PAD {
        let i=((z-z0+PAD) as usize)*W+(x-x0+PAD) as usize;floors[i]=floors[i].min(y);
    }}
    for z in 0..16 {for x in 0..16 {
        let own=floors[(z+PAD as usize)*W+x+PAD as usize];let mut floor=own;
        for dz in 0..=12 {for dx in 0..=12 {floor=floor.min(floors[(z+dz)*W+x+dx]);}}
        if floor==320{continue;}
        for y in floor.max(chunk.min_y())..(chunk.min_y()+chunk.height()) {
            let state=chunk.block(x,y,z);let block=registries.blocks.block(registries.blocks.block_of(state));
            let Some(path)=block.name.as_str().strip_prefix("minecraft:") else {continue;};
            let leaves=path.ends_with("_leaves")||matches!(path,"vine"|"brown_mushroom_block"|"red_mushroom_block");
            let trunk=own<320 && y>=own && (path.ends_with("_log")||path.ends_with("_wood")||matches!(path,"mushroom_stem"|"bamboo"|"bamboo_sapling"));
            if leaves||trunk {chunk.set_block_raw(x,y,z,minecraftoss_core::block::BlockStateId::AIR,registries);}
        }
    }}
}

/// The crater caves' blocks over one chunk, and their chests' loot.
fn decorate_caves(chunk: &mut Chunk, gen: &WorldGen, states: &mut HashMap<&'static str, minecraftoss_core::block::BlockStateId>) {
    let seed = gen.terrain.seed;
    let (x0, z0) = (chunk.pos.x * 16, chunk.pos.z * 16);
    let cell_of = |v: i32| v.div_euclid(CAVE_CELL);
    let registries = gen.terrain.registries.clone();
    for cx in cell_of(x0 - crate::jak_town::REACH - 8)..=cell_of(x0 + crate::jak_town::REACH + 23) {
        for cz in cell_of(z0 - crate::jak_town::REACH - 8)..=cell_of(z0 + crate::jak_town::REACH + 23) {
            let cell = (cx, cz);
            let Some((sx, sz)) = cave_spot(seed, cell) else { continue };
            // The cave reaches past its chamber east of its crater, 12
            // blocks about it.
            if x0 + 16 < sx - 12 || x0 > sx + CHAMBER + 13 || z0 + 16 < sz - 23 || z0 > sz + 23 {
                continue;
            }
            let Some(blocks) = cached(seed, cell, gen, true) else { continue };
            for &((x, y, z), state) in blocks.iter() {
                if x < x0 || x >= x0 + 16 || z < z0 || z >= z0 + 16 {
                    continue;
                }
                let id = *states.entry(state).or_insert_with(|| {
                    registries.blocks.parse_state(state).unwrap_or(minecraftoss_core::block::BlockStateId::AIR)
                });
                chunk.set_block_raw((x - x0) as usize, y, (z - z0) as usize, id, &registries);
            }
            if let Some(at) = cave_chest(seed, cell, gen) {
                if (x0..x0 + 16).contains(&at.0) && (z0..z0 + 16).contains(&at.2) {
                    chunk.block_entities.pending.insert(at, cave_loot(seed, cell));
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn natural_jak_generation_requires_overworld_identity_and_nonvoid_terrain(){
        use std::sync::Arc;
        let paths=minecraftoss_core::registries::DataPaths::discover().unwrap();
        let registries=Arc::new(minecraftoss_core::Registries::load(&paths).unwrap());
        let generator=minecraftoss_generator::terrain::TerrainGenerator::overworld(registries.clone(),12345).unwrap();
        let mut gen=minecraftoss_world::chunk_map::WorldGen::new(Arc::new(generator)).unwrap();
        assert!(super::natural_jak_world(&gen));
        let mut chunk=minecraftoss_core::Chunk::new(minecraftoss_core::ChunkPos::new(0,0),-64,384,registries.biomes.id("minecraft:plains").unwrap());
        let stone=registries.blocks.parse_state("minecraft:stone").unwrap();chunk.set_block_raw(3,70,3,stone,&registries);
        for dimension in ["minecraft:the_nether","minecraft:the_end","jak2:skatea"]{
            gen.dimension_type=dimension.into();gen.sky_light=true;
            assert!(!super::natural_jak_world(&gen),"sky light must not authorize natural Jak settlements");
            super::decorate(&mut chunk,&gen);
            let mut states=std::collections::HashMap::new();
            crate::jak_skate_tower_site::decorate(&mut chunk,&gen,&mut states);
            assert!(states.is_empty());assert_eq!(chunk.block(3,70,3),stone);assert!(chunk.generation.entities.is_empty());
        }
        gen.dimension_type="minecraft:overworld".into();
        let mut void=minecraftoss_generator::terrain::TerrainGenerator::overworld(registries,12345).unwrap();void.void=true;
        gen.terrain=Arc::new(void);assert!(!super::natural_jak_world(&gen));
        super::decorate(&mut chunk,&gen);
        let mut states=std::collections::HashMap::new();crate::jak_skate_tower_site::decorate(&mut chunk,&gen,&mut states);
        assert!(states.is_empty());assert_eq!(chunk.block(3,70,3),stone);assert!(chunk.generation.entities.is_empty());
    }
    use super::*;

    struct Flat(i32);
    impl Land for Flat {
        fn ground(&self, _: i32, _: i32) -> i32 {
            self.0
        }
        fn wet(&self, _: i32, _: i32) -> bool {
            false
        }
    }
    struct Sea;
    impl Land for Sea {
        fn ground(&self, _: i32, _: i32) -> i32 {
            50
        }
        fn wet(&self, _: i32, _: i32) -> bool {
            true
        }
    }

    fn first_village(land: &dyn Land) -> Option<((i32, i32), Vec<Placed>)> {
        (0..40).find_map(|i| village(7, (i, 3), land).map(|v| ((i, 3), v)))
    }

    #[test]
    fn clearing_removes_high_crowns_without_cutting_the_next_forests_trunks() {
        let Ok(paths)=minecraftoss_core::registries::DataPaths::discover() else{return;};
        if !paths.block_catalog.is_file(){return;}
        let r=minecraftoss_core::Registries::load(&paths).unwrap();
        let mut chunk=Chunk::new(minecraftoss_core::ChunkPos::new(0,0),-64,384,r.biomes.id("minecraft:plains").unwrap());
        let log=r.blocks.parse_state("minecraft:spruce_log").unwrap();let leaf=r.blocks.parse_state("minecraft:spruce_leaves").unwrap();
        for (x,y,z,s) in [(3,115,3,log),(3,125,3,leaf),(6,120,3,leaf),(6,115,3,log),(15,120,15,leaf)] {chunk.set_block_raw(x,y,z,s,&r);}
        clear_city_trees(&mut chunk,&[((3,70,3),"minecraft:air")],&r);
        assert_eq!(chunk.block(3,115,3),minecraftoss_core::block::BlockStateId::AIR);
        assert_eq!(chunk.block(3,125,3),minecraftoss_core::block::BlockStateId::AIR);
        assert_eq!(chunk.block(6,120,3),minecraftoss_core::block::BlockStateId::AIR);
        assert_eq!(chunk.block(6,115,3),log);
        assert_eq!(chunk.block(15,120,15),leaf);
    }

    #[test]
    fn precomputed_floor_biome_probe_matches_the_full_survey() {
        let paths=minecraftoss_core::registries::DataPaths::discover().unwrap();
        let r=Arc::new(minecraftoss_core::Registries::load(&paths).unwrap());
        let terrain=Arc::new(minecraftoss_generator::terrain::TerrainGenerator::overworld(r,12345).unwrap());
        let gen=WorldGen::new(terrain).unwrap();
        for (x,z) in [(-2792,-5531),(-2785,-5521),(92,31),(95,24)] {
            let (floor,wet)=gen.probe_column(x,z);
            assert_eq!(floor,gen.ground(x,z),"probe/survey height at {x},{z}");
            assert_eq!(wet,gen.wet(x,z));
            assert_eq!(survey_biome_at(&gen,x,floor,z),survey_biome(&gen,x,z));
            for spargus in [false,true] {
                assert_eq!(gen.probe_city_allowed(x,z,floor,spargus),gen.city_allowed(x,z,spargus));
            }
        }
    }

    #[test]
    fn natural_cities_are_complete_and_stay_in_their_biomes() {
        let Ok(paths)=minecraftoss_core::registries::DataPaths::discover() else{return;};
        if !paths.block_catalog.is_file(){return;}
        let registries=Arc::new(minecraftoss_core::Registries::load(&paths).unwrap());
        let terrain=Arc::new(minecraftoss_generator::terrain::TerrainGenerator::overworld(registries.clone(),12345).unwrap());
        let gen=WorldGen::new(terrain).unwrap();let started=std::time::Instant::now();
        let haven=nearest_slums(registries.clone(),12345,(0,0),6000).expect("natural Haven city");
        let spargus=crate::jak_facades::nearest_spargus(registries,12345,(-1300,512),6000).expect("natural Spargus city");
        for (sparg,center) in [(false,haven),(true,spargus)] {
            let blocks=if sparg {crate::jak_facades::spargus_village(12345,(center.0.div_euclid(crate::jak_facades::SPARGUS_CELL),center.2.div_euclid(crate::jak_facades::SPARGUS_CELL)),&gen,&|x,z|gen.city_allowed(x,z,true)).unwrap()}
                else{(*village_cached(12345,(center.0.div_euclid(CELL),center.2.div_euclid(CELL)),&gen,Style::Slums).unwrap()).clone()};
            let columns:std::collections::HashSet<_>=blocks.iter().map(|((x,_,z),_)|(*x,*z)).collect();
            assert!(columns.iter().all(|&(x,z)|gen.city_allowed(x,z,sparg)));
            assert!(blocks.iter().filter(|(_,s)|s.contains("half=lower")&&s.contains("puerta")).count()>=20);
            assert!(blocks.iter().filter(|(_,s)|s.starts_with(crate::jak_town::ROUTE_MARK)).count()>=8);
            println!("{} {:?}: {} columns",if sparg{"Spargus"}else{"Haven"},center,columns.len());
        }
        println!("Two natural city surveys: {:.2}s",started.elapsed().as_secs_f64());
    }

    #[test]
    fn a_village_has_its_bell_huts_and_beds() {
        let ((cx, cz), blocks) = first_village(&Flat(70)).expect("some square has one");
        let (vx, vz) = village_spot(7, (cx, cz)).unwrap();
        assert!(blocks.iter().any(|&(p, s)| p == (vx, 71, vz) && s.starts_with("minecraft:bell")));
        let beds = blocks.iter().filter(|(_, s)| s.contains("_bed[") && s.contains("part=head")).count();
        assert!(beds>=4, "at least four furnished homes: {beds}");
        let final_blocks:HashMap<_,_>=blocks.iter().copied().collect();
        let heads=final_blocks.values().filter(|s|s.contains("_bed[")&&s.contains("part=head")).count();
        let feet=final_blocks.values().filter(|s|s.contains("_bed[")&&s.contains("part=foot")).count();
        assert_eq!(heads,feet,"both halves of every furnished bed survive generation");
        // Its houses of Haven's slums (`jak_facades`): boarded upper floors.
        assert!(blocks.iter().filter(|(_, s)| s.starts_with("jak:haven_mamposteria_pared")).count() > 20);
        // Every block within the reach a chunk looks for.
        assert!(blocks.iter().all(|&((x, _, z), _)| (x - vx).abs() <= crate::jak_town::REACH && (z - vz).abs() <= crate::jak_town::REACH));
    }

    /// Every block state a village is made of is one the game knows.
    #[test]
    fn every_village_block_parses() {
        let Ok(paths) = minecraftoss_core::registries::DataPaths::discover() else {
            return;
        };
        if !paths.block_catalog.is_file() {
            return;
        }
        let registries = minecraftoss_core::Registries::load(&paths).unwrap();
        let (_, mut blocks) = first_village(&Flat(70)).unwrap();
        blocks.extend((0..60).find_map(|i| crater_cave(9, (i, 2), &Flat(70))).unwrap());
        let mut seen = std::collections::HashSet::new();
        for (_, state) in blocks {
            if seen.insert(state) {
                assert!(registries.blocks.parse_state(state).is_ok(), "{state}");
            }
        }
    }

    /// A crater cave: its crater, its chamber shelled in blackstone with
    /// bedrock behind, its enemies' mark, its chest with Jak's loot.
    #[test]
    fn a_crater_cave_has_its_chamber_chest_and_mark() {
        let (cell, blocks) = (0..60).find_map(|i| crater_cave(9, (i, 2), &Flat(70)).map(|b| ((i, 2), b))).expect("some square has one");
        assert!(blocks.iter().any(|(_, s)| *s == CAVE_MARK));
        assert!(blocks.iter().filter(|(_, s)| *s == "minecraft:bedrock").count() > 200);
        let chest = cave_chest(9, cell, &Flat(70)).unwrap();
        assert!(blocks.iter().any(|&(p, s)| p == chest && s.starts_with("minecraft:chest")));
        let loot = cave_loot(9, cell);
        let items = match loot.get("Items") {
            Some(minecraftoss_core::nbt::Tag::List(items)) => items.len(),
            _ => 0,
        };
        assert_eq!(items, 8);
        // The chamber is under the ground, the tunnel open to the crater.
        let (sx, _) = cave_spot(9, cell).unwrap();
        assert!(chest.1 < 70 - 15 && chest.0 > sx);
    }

    /// One can walk from the crater down the tunnel into the chamber: a
    /// path two high, never a step of more than a block.
    #[test]
    fn the_tunnel_leads_into_the_chamber() {
        let (cell, blocks) = (0..60).find_map(|i| crater_cave(9, (i, 2), &Flat(70)).map(|b| ((i, 2), b))).unwrap();
        let (cx, cz) = cave_spot(9, cell).unwrap();
        let mut world = HashMap::new();
        for (p, s) in blocks {
            world.insert(p, s);
        }
        let air = |p: (i32, i32, i32)| world.get(&p).is_some_and(|s| *s == "minecraft:air");
        let solid = |p: (i32, i32, i32)| world.get(&p).is_some_and(|s| *s != "minecraft:air");
        // Standing on the crater floor (its floor block under y 66).
        let mut y = 70 - 3;
        for x in cx..=cx + CHAMBER {
            // The lowest open spot within a block of where one is.
            let next = (y - 2..=y + 1).filter(|&yy| air((x, yy, cz)) && air((x, yy + 1, cz)) && solid((x, yy - 1, cz))).max_by_key(|&yy| -(yy - y).abs());
            let Some(next) = next else { panic!("blocked at x {} (y {y})", x - cx) };
            assert!((next - y).abs() <= 2, "a drop at {}", x - cx);
            y = next;
        }
        assert_eq!(y, 70 - 3 - 22, "on the chamber's floor");
    }

    #[test]
    fn no_village_on_the_sea_and_squares_are_fixed() {
        assert!(first_village(&Sea).is_none());
        assert_eq!(village_spot(7, (5, 5)), village_spot(7, (5, 5)));
        let some = (-20..20).flat_map(|x| (-20..20).map(move |z| (x, z))).filter(|&c| village_spot(7, c).is_some()).count();
        assert!((400..720).contains(&some), "{some}");
    }
}


pub(crate) fn cache_city_props_near(registries:Arc<minecraftoss_core::Registries>,seed:i64,pos:(i32,i32,i32)) {
    let Ok(terrain)=minecraftoss_generator::terrain::TerrainGenerator::overworld(registries,seed) else{return;};
    let Ok(gen)=WorldGen::new(Arc::new(terrain)) else{return;};
    for cx in (pos.0-crate::jak_town::REACH).div_euclid(CELL)..=(pos.0+crate::jak_town::REACH).div_euclid(CELL) {
        for cz in (pos.2-crate::jak_town::REACH).div_euclid(CELL)..=(pos.2+crate::jak_town::REACH).div_euclid(CELL) {if village_spot(seed,(cx,cz)).is_some_and(|(x,z)|(x-pos.0).abs()<=crate::jak_town::REACH+8 && (z-pos.2).abs()<=crate::jak_town::REACH+8) {
                let _=village_styled(seed,(cx,cz),&gen,Style::Slums);
            }}
    }
    let cell=crate::jak_facades::SPARGUS_CELL;
    for cx in (pos.0-crate::jak_town::REACH).div_euclid(cell)..=(pos.0+crate::jak_town::REACH).div_euclid(cell) {
        for cz in (pos.2-crate::jak_town::REACH).div_euclid(cell)..=(pos.2+crate::jak_town::REACH).div_euclid(cell) {
            let _=crate::jak_facades::spargus_village(seed,(cx,cz),&gen,&|x,z|crate::wasteland::is_wasteland(&gen,x,z));
        }
    }
}

#[cfg(test)]mod distant_road_metadata_regressions{
 use super::*;
 struct Flat;impl Land for Flat{fn ground(&self,_:i32,_:i32)->i32{70}fn wet(&self,_:i32,_:i32)->bool{false}}
 #[test]fn tower_anchors_match_actual_generated_paving_in_every_town_rotation(){
  let seed=0x951103i64;let mut turns=std::collections::HashSet::new();
  for x in -8..8{for z in -8..8{
   let Some(layout)=slums_layout(seed,(x,z),&Flat)else{continue;};if !turns.insert(layout.turn){continue;}
   let anchors=slums_road_anchors(seed,(x,z),&Flat).unwrap();assert!(!anchors.is_empty());
   let actual=crate::jak_barrio::village_from_layout(seed,(*layout).clone(),&Flat);
   for &(pos,state) in anchors.iter(){
    assert_eq!(actual.iter().rev().find(|(p,_)|*p==pos).map(|(_,s)|*s),Some(state),"road anchor {pos:?}, rotation {}",layout.turn);
   }
   if turns.len()==4{return;}
  }}panic!("not all town rotations were checked: {turns:?}");
 }
}
