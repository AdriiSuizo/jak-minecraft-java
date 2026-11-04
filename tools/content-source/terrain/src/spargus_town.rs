//! Spargus as a small city of blocks in the Wasteland (Adrii, 23/10:
//! "bioma urbano más pequeño, reconocible y conectado al desierto mediante
//! accesos transitables; adapta sus edificios y distribución a la escala
//! de nuestro juego"): on a round of the desert levelled whole (a flat of
//! the land, `jak_lands::flat_at`, its edge sloping in eighths so the
//! vehicles drive in), a plaza of Spargus's own stone and earth
//! (`jak_ground`) with the Wastelanders' totem and bell, houses of blocks
//! round it (`jak_hamlets::house_blocks`: stucco walls that join, windows,
//! airlock doors, flat roofs of its roofing), earth streets to their doors,
//! its palms and rocks (`jak_objects`), and a ring wall of its stone with
//! a gate each way. Every block is the world's own: broken one by one,
//! joined afresh by the game.
//!
//! The city stands once a world, at the nearest dry Wasteland to the
//! origin (where Jak 3's whole city stood cut into pieces before:
//! `IW4L_SPARGUS_NIVEL=1` puts that one back instead).
use std::collections::HashMap;
use std::sync::{Arc, Mutex, OnceLock};

use minecraftoss_core::block::BlockStateId;
use minecraftoss_core::chunk::Chunk;
use minecraftoss_world::chunk_map::WorldGen;

use crate::jak_hamlets::{house_blocks, Made, HOUSE_H, SPARGUS_HOUSE};

/// The city's round: its wall stands at `WALL`, its flat reaches past it.
pub const WALL: i32 = 34;
pub const RADIUS: i32 = WALL + 6;
/// The plaza's stone and its earth ring.
const PLAZA: i32 = 10;
const PLAZA_EARTH: i32 = 13;
/// Where the houses stand off the middle.
const HOUSES_AT: i32 = 22;

/// Whether Jak 3's whole city is wanted instead (`IW4L_SPARGUS_NIVEL=1`).
pub fn level_wanted() -> bool {
    std::env::var_os("IW4L_SPARGUS_NIVEL").is_some_and(|v| v == "1")
}

/// Where the city stands in a world: its middle (x, z) and the height of
/// its ground's top block, once a seed (none without a Wasteland near
/// enough, or with the level's city wanted instead).
pub fn site(registries: Arc<minecraftoss_core::Registries>, seed: i64) -> Option<(i32, i32, i32)> {
    static SITES: Mutex<Option<HashMap<i64, Option<(i32, i32, i32)>>>> = Mutex::new(None);
    if let Some(found) = SITES.lock().ok().and_then(|s| s.as_ref().and_then(|s| s.get(&seed).copied())) {
        return found;
    }
    let found = (!level_wanted()).then(|| crate::wasteland::nearest(registries, seed, (0, 0), 8000)).flatten().map(|(x, y, z)| (x, y, z));
    if let Ok(mut sites) = SITES.lock() {
        sites.get_or_insert_with(HashMap::new).insert(seed, found);
    }
    found
}

/// The city's site in the world being played.
pub fn current_site() -> Option<(i32, i32, i32)> {
    let (seed, registries) = crate::original_places::current_world()?;
    site(registries, seed)
}

/// Whether world column `(x, z)` is inside the city's round (its flat).
pub fn inside(registries: &Arc<minecraftoss_core::Registries>, seed: i64, x: i32, z: i32) -> bool {
    site(registries.clone(), seed).is_some_and(|(cx, _, cz)| (cx - x).pow(2) + (cz - z).pow(2) <= (RADIUS + 12).pow(2))
}

/// The flat the city stands on, for the land (`jak_lands::flats_near`):
/// middle, radius and the height of the land's top block there, levelled
/// whole.
pub fn flat(registries: &Arc<minecraftoss_core::Registries>, seed: i64) -> Option<((i32, i32), f32, i32)> {
    let (x, y, z) = site(registries.clone(), seed)?;
    Some(((x, z), RADIUS as f32, y - 1))
}

/// The city's ground: the top block of its flat (`jak_lands`: a whole
/// flat's top block is a block over the land's top there).
fn ground(y: i32) -> i32 {
    y
}

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

fn mix(seed: i64, a: i32, b: i32, salt: u64) -> u64 {
    let mut h = (seed as u64) ^ ((a as u32 as u64) << 32 | u64::from(b as u32)) ^ salt.wrapping_mul(0x9E37_79B9_7F4A_7C15);
    h ^= h >> 33;
    h = h.wrapping_mul(0xff51_afd7_ed55_8ccd);
    h ^= h >> 33;
    h = h.wrapping_mul(0xc4ce_b9fe_1a85_ec53);
    h ^ (h >> 33)
}

/// The city's houses: where each stands (its floor's low corner), its
/// footprint and which way its door looks (toward the plaza).
fn houses((cx, y, cz): (i32, i32, i32), seed: i64) -> Vec<((i32, i32, i32), i32, u8)> {
    let mut out = Vec::new();
    for k in 0..8 {
        let angle = k as f32 / 8.0 * std::f32::consts::TAU + 0.2;
        let w = if k % 2 == 0 { 9 } else { 7 };
        let r = HOUSES_AT + (mix(seed, k, 0, 331) % 3) as i32;
        let (hx, hz) = (cx + (angle.cos() * r as f32).round() as i32, cz + (angle.sin() * r as f32).round() as i32);
        let (dx, dz) = (cx - hx, cz - hz);
        let facing = if dx.abs() > dz.abs() { if dx < 0 { 3 } else { 1 } } else if dz < 0 { 0 } else { 2 };
        out.push(((hx - w / 2, ground(y), hz - w / 2), w, facing));
    }
    out
}

/// The city's blocks (made once a seed): see the module.
pub fn blocks(registries: &Arc<minecraftoss_core::Registries>, seed: i64) -> Option<Arc<Vec<((i32, i32, i32), &'static str)>>> {
    static MADE: Mutex<Option<(i64, Arc<Vec<((i32, i32, i32), &'static str)>>)>> = Mutex::new(None);
    if let Some((s, made)) = MADE.lock().ok().and_then(|m| m.clone()) {
        if s == seed {
            return Some(made);
        }
    }
    let site = site(registries.clone(), seed)?;
    let made = Arc::new(make(site, seed));
    if let Ok(mut held) = MADE.lock() {
        *held = Some((seed, made.clone()));
    }
    Some(made)
}

fn make((cx, y, cz): (i32, i32, i32), seed: i64) -> Vec<((i32, i32, i32), &'static str)> {
    let g = ground(y);
    let mut out: Vec<((i32, i32, i32), &'static str)> = Vec::new();
    let stone = "jak:suelo_spargus_piedra";
    let earth = "jak:suelo_spargus_tierra";
    let ditch = "jak:suelo_spargus_zanja";
    // The plaza: its stone, an earth ring, the totem and its bell.
    for dx in -PLAZA_EARTH..=PLAZA_EARTH {
        for dz in -PLAZA_EARTH..=PLAZA_EARTH {
            let d2 = dx * dx + dz * dz;
            if d2 > PLAZA_EARTH * PLAZA_EARTH {
                continue;
            }
            let block = if d2 <= PLAZA * PLAZA { stone } else { earth };
            out.push(((cx + dx, g, cz + dz), block));
            out.push(((cx + dx, g + 1, cz + dz), "minecraft:air"));
        }
    }
    for k in 1..=3 {
        out.push(((cx, g + k, cz), crate::wasteland::TOTEM));
    }
    out.push(((cx + 1, g + 1, cz), "minecraft:bell[attachment=floor,facing=east,powered=false]"));
    // The houses, and an earth street from each door to the plaza.
    for (origin, w, facing) in houses((cx, y, cz), seed) {
        out.extend(house_blocks(&SPARGUS_HOUSE, origin, w, HOUSE_H, facing));
        let m = w / 2;
        let (door, step): ((i32, i32), (i32, i32)) = match facing {
            0 => ((origin.0 + m, origin.2 - 1), (0, -1)),
            1 => ((origin.0 + w, origin.2 + m), (1, 0)),
            2 => ((origin.0 + m, origin.2 + w), (0, 1)),
            _ => ((origin.0 - 1, origin.2 + m), (-1, 0)),
        };
        let (mut px, mut pz) = door;
        for _ in 0..40 {
            if (px - cx).pow(2) + (pz - cz).pow(2) <= PLAZA_EARTH * PLAZA_EARTH {
                break;
            }
            for (ox, oz) in [(0, 0), (step.1, step.0), (-step.1, -step.0)] {
                out.push(((px + ox, g, pz + oz), if ox == 0 && oz == 0 { ditch } else { earth }));
                out.push(((px + ox, g + 1, pz + oz), "minecraft:air"));
            }
            // Toward the plaza's middle.
            let (dx, dz) = (cx - px, cz - pz);
            if dx.abs() > dz.abs() {
                px += dx.signum();
            } else {
                pz += dz.signum();
            }
        }
    }
    // The ring wall, three high, joined along, with a gate each way.
    let mut ring: Vec<(i32, i32)> = Vec::new();
    let steps = (WALL as f32 * std::f32::consts::TAU * 2.0) as i32;
    for i in 0..steps {
        let a = i as f32 / steps as f32 * std::f32::consts::TAU;
        let cell = ((a.cos() * WALL as f32).round() as i32, (a.sin() * WALL as f32).round() as i32);
        if let Some(&last) = ring.last() {
            if last == cell {
                continue;
            }
            // Four-joined: a step on both axes goes by the way of one.
            if last.0 != cell.0 && last.1 != cell.1 {
                ring.push((cell.0, last.1));
            }
        }
        ring.push(cell);
    }
    let gate = |(dx, dz): (i32, i32)| (dx.abs() <= 2 && dz.abs() >= WALL - 1) || (dz.abs() <= 2 && dx.abs() >= WALL - 1);
    let cells: std::collections::HashSet<(i32, i32)> = ring.iter().copied().filter(|&c| !gate(c)).collect();
    for &(dx, dz) in &cells {
        let joins = [cells.contains(&(dx, dz - 1)), cells.contains(&(dx + 1, dz)), cells.contains(&(dx, dz + 1)), cells.contains(&(dx - 1, dz))];
        out.push(((cx + dx, g, cz + dz), "jak:spargus_bricks"));
        for k in 1..=3 {
            out.push(((cx + dx, g + k, cz + dz), kept(format!("jak:muro_spargus_piedra[east={},north={},south={},waterlogged=false,west={}]", joins[1], joins[0], joins[2], joins[3]))));
        }
    }
    // The gates' posts, a block taller than the wall.
    for &(dx, dz) in ring.iter().filter(|&&c| gate(c)) {
        let post = (dx.abs() == 3 && dz.abs() >= WALL - 1) || (dz.abs() == 3 && dx.abs() >= WALL - 1);
        if post {
            for k in 0..=4 {
                out.push(((cx + dx, g + k, cz + dz), "jak:spargus_bricks"));
            }
        } else {
            out.push(((cx + dx, g, cz + dz), earth));
            for k in 1..=4 {
                out.push(((cx + dx, g + k, cz + dz), "minecraft:air"));
            }
        }
    }
    out
}

/// The city's palms and rocks (made once a seed), for the world and for
/// finding which object a block is (`jak_hamlets::instance_at`).
pub fn objects(seed: i64) -> Arc<Made> {
    static MADE: Mutex<Option<(i64, Arc<Made>)>> = Mutex::new(None);
    if let Some((s, made)) = MADE.lock().ok().and_then(|m| m.clone()) {
        if s == seed {
            return made;
        }
    }
    let made = Arc::new(Made::new(current_site().map(|site| make_objects(site, seed)).unwrap_or_default()));
    if let Ok(mut held) = MADE.lock() {
        *held = Some((seed, made.clone()));
    }
    made
}

fn make_objects((cx, y, cz): (i32, i32, i32), seed: i64) -> Vec<crate::jak_objects::Instance> {
    use crate::jak_objects::{placed, shape, OBJECTS};
    let kinds = |group: &str| -> Vec<usize> {
        OBJECTS.iter().enumerate().filter(|(k, o)| o.place == "spargus" && o.group == group && shape(*k).is_some()).map(|(k, _)| k).collect()
    };
    let (palms, rocks) = (kinds("palmera"), kinds("roca"));
    let g = ground(y);
    let mut out = Vec::new();
    let houses = houses((cx, y, cz), seed);
    let clear = |x: i32, z: i32| houses.iter().all(|(o, w, _)| x < o.0 - 2 || x > o.0 + w + 1 || z < o.2 - 2 || z > o.2 + w + 1);
    // Palms between the houses, inside the wall; rocks outside it.
    for k in 0..16 {
        let a = (k as f32 + 0.5) / 16.0 * std::f32::consts::TAU;
        let (list, r) = if k % 2 == 0 { (&palms, HOUSES_AT + 6) } else { (&rocks, WALL + 4) };
        if list.is_empty() {
            continue;
        }
        let (x, z) = (cx + (a.cos() * r as f32).round() as i32, cz + (a.sin() * r as f32).round() as i32);
        if !clear(x, z) {
            continue;
        }
        let which = list[(mix(seed, k, 1, 341) % list.len() as u64) as usize];
        let Some(instance) = placed(which, (x, g + 1, z), (mix(seed, k, 2, 342) % 4) as u8) else { continue };
        out.push(instance);
    }
    out
}

/// What the far terrain shows of world column `(x, z)` in the city: the
/// height of its top and the texture there (its wall's stone, a roof, the
/// plaza's stone or earth), or none off the city.
pub fn far_at(registries: &Arc<minecraftoss_core::Registries>, seed: i64, x: i32, z: i32) -> Option<(f32, &'static str)> {
    let (cx, y, cz) = site(registries.clone(), seed)?;
    let g = ground(y);
    let (dx, dz) = (x - cx, z - cz);
    let d2 = dx * dx + dz * dz;
    if d2 > (WALL + 1) * (WALL + 1) {
        return None;
    }
    let d = (d2 as f32).sqrt();
    if (d - WALL as f32).abs() <= 0.75 {
        let gate = (dx.abs() <= 2 && dz.abs() >= WALL - 1) || (dz.abs() <= 2 && dx.abs() >= WALL - 1);
        return Some(if gate { ((g + 1) as f32, "wascity-ground-01") } else { ((g + 4) as f32, "wascity-stonewall-bricks") });
    }
    for (origin, w, _) in houses((cx, y, cz), seed) {
        if x >= origin.0 - 1 && x <= origin.0 + w && z >= origin.2 - 1 && z <= origin.2 + w {
            return Some(((g + HOUSE_H + 2) as f32, "wascity-roof-1"));
        }
    }
    if d2 <= PLAZA * PLAZA {
        return Some(((g + 1) as f32, "wascitya-stone-top"));
    }
    if d2 <= PLAZA_EARTH * PLAZA_EARTH {
        return Some(((g + 1) as f32, "wascity-ground-01"));
    }
    None
}

/// The city's blocks over one chunk, and its objects' blocks.
pub fn place(chunk: &mut Chunk, gen: &WorldGen, states: &mut HashMap<&'static str, BlockStateId>) {
    let registries = gen.terrain.registries.clone();
    let seed = gen.terrain.seed;
    let Some((cx, _, cz)) = site(registries.clone(), seed) else { return };
    let (x0, z0) = (chunk.pos.x * 16, chunk.pos.z * 16);
    if (x0 + 8 - cx).abs() > RADIUS + 24 || (z0 + 8 - cz).abs() > RADIUS + 24 {
        return;
    }
    let _ = crate::original_places::sites(registries.clone(), seed);
    let Some(blocks) = blocks(&registries, seed) else { return };
    for &((x, y, z), name) in blocks.iter() {
        if x < x0 || x >= x0 + 16 || z < z0 || z >= z0 + 16 || !(-63..=319).contains(&y) {
            continue;
        }
        let id = *states.entry(name).or_insert_with(|| registries.blocks.parse_state(name).unwrap_or(BlockStateId::AIR));
        chunk.set_block_raw((x - x0) as usize, y, (z - z0) as usize, id, &registries);
    }
    for instance in objects(seed).iter() {
        let Some(shape) = crate::jak_objects::shape(usize::from(instance.kind)) else { continue };
        for ((x, y, z), cell) in instance.cells() {
            if x < x0 || x >= x0 + 16 || z < z0 || z >= z0 + 16 || !(-63..=319).contains(&y) {
                continue;
            }
            let slab = shape.cells.get(&cell).map_or("bottom", crate::jak_objects::slab_type);
            let name = kept(format!("jak:pieza_spargus[type={slab},waterlogged=false]"));
            let id = *states.entry(name).or_insert_with(|| registries.blocks.parse_state(name).unwrap_or(BlockStateId::AIR));
            chunk.set_block_raw((x - x0) as usize, y, (z - z0) as usize, id, &registries);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The city (seed 12345): on the Wasteland, its plaza of Spargus's
    /// stone, eight houses of joined walls with doors and roofs, its ring
    /// wall joined all round but at its four gates, every block one the
    /// world knows.
    #[test]
    fn spargus_is_a_small_city_of_blocks() {
        let Ok(paths) = minecraftoss_core::registries::DataPaths::discover() else { return };
        if !paths.block_catalog.is_file() {
            return;
        }
        let registries = Arc::new(minecraftoss_core::Registries::load(&paths).unwrap());
        let Some(site) = site(registries.clone(), 12345) else { panic!("no Wasteland near the origin") };
        let blocks = blocks(&registries, 12345).unwrap();
        let count = |prefix: &str| blocks.iter().filter(|(_, n)| n.starts_with(prefix)).count();
        println!("site {site:?}: {} blocks; {} stucco, {} stone wall, {} doors, {} roof, {} stone floor", blocks.len(), count("jak:muro_spargus_estuco"), count("jak:muro_spargus_piedra"), count("jak:spargus_door"), count("jak:suelo_spargus_tejado"), count("jak:suelo_spargus_piedra"));
        assert!(count("jak:muro_spargus_estuco") > 8 * 60);
        assert_eq!(count("jak:spargus_door"), 16);
        assert!(count("jak:muro_spargus_piedra") > 3 * 150);
        let mut seen = std::collections::HashSet::new();
        for (_, name) in blocks.iter() {
            if seen.insert(*name) {
                assert!(registries.blocks.parse_state(name).is_ok(), "{name}");
            }
        }
        // The wall's blocks are joined to the ones beside them.
        let walls: std::collections::HashSet<(i32, i32, i32)> = blocks.iter().filter(|(_, n)| n.starts_with("jak:muro_spargus_piedra")).map(|(p, _)| *p).collect();
        for &(x, y, z) in &walls {
            let beside = [(0, -1), (1, 0), (0, 1), (-1, 0)].iter().filter(|(dx, dz)| walls.contains(&(x + dx, y, z + dz))).count();
            assert!(beside >= 1, "{x} {y} {z} stands alone");
        }
    }
}
