//! The Jak lands' common ground (Adrii 23/10: each Jak biome a subsystem, easy
//! to put in the game or take out): what a Jak biome does to the chunks of
//! the Minecraft biomes it takes over, from its description alone.
//!
//! The ground stays Minecraft's (its noise, its caves, its water, its deep
//! stone); as each chunk becomes final, in the columns that are the land's:
//! - its edge with the neighbouring biomes goes column by column, more of
//!   the land's the deeper in (the land's share of the biome grid round each
//!   column, the columns drawn by a fixed hash): no straight border;
//! - each column's blocks down to `depth` under its top become the land's,
//!   rule by rule, by the slope it stands on (flat, a two-block edge, a
//!   cliff, or a cliff's foot) and how deep they lie;
//! - over its top block, where it is one the land smooths, eighths of a
//!   block up to where Minecraft's own terrain truly tops out inside it
//!   (`GenerationData::surface_fraction`): slopes rise in eighths, the same
//!   from chunk to chunk, for Jak and the vehicles (no whole-block steps);
//! - the stone its caves' air touches is its cave wall, the stone under that
//!   air its cave floor, so digging down into one is still the land.
//!
//! What a land has of its own (plants, trees, rock formations, settlements)
//! it does after, with the columns and tops this leaves.
use std::collections::HashMap;

use minecraftoss_core::block::BlockStateId;
use minecraftoss_core::chunk::{Chunk, HeightmapKind};
use minecraftoss_world::chunk_map::WorldGen;

/// A column's slope: how far its top stands from its neighbours' (in this
/// chunk: the noise's height of the next chunk's columns cost a chunk 20
/// ms).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Slope {
    Flat,
    /// A neighbour one less than a cliff off: the ground gives way.
    Edge,
    /// A cliff: a neighbour `steep` or more off.
    Steep,
    /// Not a slope itself, a cliff two columns off: the cliff's foot.
    Foot,
}

/// A vanilla block becomes the land's (`to`: by the column's slope and how
/// deep the block lies, 0 its top; none keeps it).
pub struct Rule {
    pub from: &'static str,
    pub to: fn(Slope, i32) -> Option<&'static str>,
}

/// A top the land lays its eighths over: its blocks, the whole block of the
/// eighths, the seven eighths, and the least it lays (Minecraft's snow lies
/// a layer deep everywhere it falls).
pub struct Layers {
    pub tops: &'static [&'static str],
    pub whole: &'static str,
    pub eighths: [&'static str; 7],
    pub least: usize,
}

pub struct Land {
    /// Its name in the logs.
    pub name: &'static str,
    /// The Minecraft biomes it takes over.
    pub biomes: &'static [&'static str],
    /// How deep under the top its ground goes.
    pub depth: i32,
    /// How far a neighbour stands for a cliff.
    pub steep: i32,
    /// The column hash's salt.
    pub salt: u64,
    /// Lying on the top, taken off first (Minecraft's snow layer: the
    /// land's eighths lie there instead).
    pub strip: &'static [&'static str],
    pub rules: &'static [Rule],
    pub layers: &'static [Layers],
    /// Its caves' wall and floor.
    pub caves: Option<(&'static str, &'static str)>,
}

/// How far over a column's ground what grew on it reaches (Minecraft's
/// tallest trees, the mega spruces, some 30 blocks).
const ABOVE: i32 = 40;

/// What shaping a chunk leaves for the land's own work: which columns are
/// its, each column's top (its top block's height), its slope, and whether
/// eighths lie over it.
pub struct Shaped {
    pub columns: [[bool; 16]; 16],
    pub tops: [[i32; 16]; 16],
    pub slopes: [[Slope; 16]; 16],
    pub layered: [[bool; 16]; 16],
}

/// Whether `(x, z)` is in `land` (its ground's biome one of its biomes).
pub fn is_in(land: &Land, gen: &WorldGen, x: i32, z: i32) -> bool {
    let biomes = &gen.terrain.registries.biomes;
    let y = gen.terrain.base_height(x, z, HeightmapKind::OceanFloorWg);
    let here = gen.terrain.biome_at_quart(x >> 2, y >> 2, z >> 2);
    land.biomes.iter().filter_map(|b| biomes.id(b)).any(|id| id == here)
}

pub fn mix(seed: i64, a: i32, b: i32, salt: u64) -> u64 {
    let mut h = (seed as u64) ^ ((a as u32 as u64) << 32 | u64::from(b as u32)) ^ salt.wrapping_mul(0x9E37_79B9_7F4A_7C15);
    h ^= h >> 33;
    h = h.wrapping_mul(0xff51_afd7_ed55_8ccd);
    h ^= h >> 33;
    h = h.wrapping_mul(0xc4ce_b9fe_1a85_ec53);
    h ^ (h >> 33)
}

/// A state by its name, looked up once for all the chunks.
pub fn state(gen: &WorldGen, states: &mut HashMap<&'static str, BlockStateId>, name: &'static str) -> BlockStateId {
    *states.entry(name).or_insert_with(|| gen.terrain.registries.blocks.parse_state(name).unwrap_or(BlockStateId::AIR))
}

/// The land's share round each quart of the chunk (and one quart past it),
/// at the chunk's middle height: none where it has none.
pub fn shares(land: &Land, chunk: &Chunk, gen: &WorldGen) -> Option<[[f32; 4]; 4]> {
    let biomes = &gen.terrain.registries.biomes;
    let ids: Vec<_> = land.biomes.iter().filter_map(|b| biomes.id(b)).collect();
    if ids.is_empty() {
        return None;
    }
    let (qx0, qz0) = (chunk.pos.x * 4, chunk.pos.z * 4);
    let y = chunk.heightmaps.get(HeightmapKind::WorldSurfaceWg, 8, 8).max(63);
    let qy = y >> 2;
    let mut grid = [[false; 6]; 6];
    let mut any = false;
    for (i, row) in grid.iter_mut().enumerate() {
        for (j, cell) in row.iter_mut().enumerate() {
            let (qx, qz) = (qx0 + i as i32 - 1, qz0 + j as i32 - 1);
            let inside = (1..5).contains(&i) && (1..5).contains(&j);
            let biome = if inside { chunk.biome(i - 1, qy, j - 1) } else { gen.terrain.biome_at_quart(qx, qy, qz) };
            *cell = ids.contains(&biome);
            any |= *cell;
        }
    }
    if !any {
        return None;
    }
    let mut out = [[0.0; 4]; 4];
    for (i, row) in out.iter_mut().enumerate() {
        for (j, share) in row.iter_mut().enumerate() {
            let mut n = 0;
            for di in 0..3 {
                for dj in 0..3 {
                    n += usize::from(grid[i + di][j + dj]);
                }
            }
            *share = n as f32 / 9.0;
        }
    }
    Some(out)
}

/// Whether the column at `(x, z)` (world) is the land's, by its share.
pub fn column_is(land: &Land, seed: i64, x: i32, z: i32, share: f32) -> bool {
    share >= 1.0 || (share > 0.0 && (mix(seed, x, z, land.salt) % 1000) as f32 / 1000.0 < share)
}

/// Wide flats of the land (Adrii, 23/10: "grandes explanadas para que pasen
/// los coches"): now and then, one a square of `FLAT_CELL` blocks perhaps,
/// a round of 22 to 40 blocks levelled to its middle's height, its last 12
/// blocks sloping to the land round it (in eighths, as the dunes).
pub const FLAT_CELL: i32 = 144;

/// The flats near a column: middle, radius, height (the top block's), and
/// whether it is levelled whole (a block over its height, no eighths: the
/// city of Spargus stands on one) or to the half block the dunes are.
fn flats_near(land: &Land, terrain: &minecraftoss_generator::terrain::TerrainGenerator, x: i32, z: i32) -> Vec<((i32, i32), f32, i32, bool)> {
    // (None: the lands as they were, Adrii 23/10.)
    if !crate::original_places::jak_places_on() {
        return Vec::new();
    }
    let salt = land.name.bytes().fold(901u64, |h, b| h.wrapping_mul(31).wrapping_add(u64::from(b)));
    let seed = terrain.seed;
    let mut out = Vec::new();
    if land.name == "Wasteland" {
        if let Some(((mx, mz), r, h)) = crate::spargus_town::flat(&terrain.registries, seed) {
            if ((mx - x) as f32).hypot((mz - z) as f32) <= r {
                out.push(((mx, mz), r, h, true));
            }
        }
    }
    for cx in x.div_euclid(FLAT_CELL) - 1..=x.div_euclid(FLAT_CELL) + 1 {
        for cz in z.div_euclid(FLAT_CELL) - 1..=z.div_euclid(FLAT_CELL) + 1 {
            if mix(seed, cx, cz, salt) % 100 >= 45 {
                continue;
            }
            let (mx, mz) = (
                cx * FLAT_CELL + 48 + (mix(seed, cx, cz, salt + 1) % (FLAT_CELL as u64 - 96)) as i32,
                cz * FLAT_CELL + 48 + (mix(seed, cx, cz, salt + 2) % (FLAT_CELL as u64 - 96)) as i32,
            );
            let r = 22.0 + (mix(seed, cx, cz, salt + 3) % 19) as f32;
            if ((mx - x) as f32).hypot((mz - z) as f32) > r {
                continue;
            }
            // Dry ground in the middle (not a lake's bed), read once.
            static HEIGHTS: std::sync::Mutex<Option<HashMap<(i64, i32, i32), Option<i32>>>> = std::sync::Mutex::new(None);
            let known = HEIGHTS.lock().ok().and_then(|m| m.as_ref().and_then(|m| m.get(&(seed, mx, mz)).copied()));
            let height = match known {
                Some(h) => h,
                None => {
                    let (ground, surface) = (terrain.base_height(mx, mz, HeightmapKind::OceanFloorWg), terrain.base_height(mx, mz, HeightmapKind::WorldSurfaceWg));
                    let h = (surface <= ground).then_some(surface - 1);
                    if let Ok(mut m) = HEIGHTS.lock() {
                        let m = m.get_or_insert_with(HashMap::new);
                        if m.len() > 65536 {
                            m.clear();
                        }
                        m.insert((seed, mx, mz), h);
                    }
                    h
                }
            };
            let Some(h) = height else { continue };
            out.push(((mx, mz), r, h, false));
        }
    }
    out
}

/// How far a column is levelled to a flat (0 not at all, 1 wholly) and the
/// flat's height.
pub fn flat_at(land: &Land, terrain: &minecraftoss_generator::terrain::TerrainGenerator, x: i32, z: i32) -> Option<(f32, i32)> {
    flat_whole_at(land, terrain, x, z).map(|(w, h, _)| (w, h))
}

/// `flat_at`, and whether the flat is levelled whole.
fn flat_whole_at(land: &Land, terrain: &minecraftoss_generator::terrain::TerrainGenerator, x: i32, z: i32) -> Option<(f32, i32, bool)> {
    flats_near(land, terrain, x, z)
        .into_iter()
        .map(|((mx, mz), r, h, whole)| {
            let d = ((mx - x) as f32).hypot((mz - z) as f32);
            let w = ((r - d) / 12.0).clamp(0.0, 1.0);
            (w * w * (3.0 - 2.0 * w), h, whole)
        })
        .filter(|(w, ..)| *w > 0.0)
        .max_by(|a, b| a.0.total_cmp(&b.0))
}

/// Makes the land's columns of a chunk its own (see the module); none where
/// the land has no share.
pub fn shape(land: &Land, chunk: &mut Chunk, gen: &WorldGen, states: &mut HashMap<&'static str, BlockStateId>) -> Option<Shaped> {
    let shares = shares(land, chunk, gen)?;
    let seed = gen.terrain.seed;
    let registries = gen.terrain.registries.clone();
    let air = BlockStateId::AIR;
    let (x0, z0) = (chunk.pos.x * 16, chunk.pos.z * 16);
    let strip: Vec<minecraftoss_core::block::BlockId> = land.strip.iter().filter_map(|n| registries.blocks.block_by_name(n)).collect();
    let rules: Vec<(minecraftoss_core::block::BlockId, &Rule)> = land
        .rules
        .iter()
        .filter_map(|r| registries.blocks.block_by_name(r.from).map(|b| (b, r)))
        .collect();
    let mut out = Shaped { columns: [[false; 16]; 16], tops: [[0; 16]; 16], slopes: [[Slope::Flat; 16]; 16], layered: [[false; 16]; 16] };
    // Each column's top: its ground's, as the noise left it (water counts;
    // the heightmap is the noise's: what Minecraft's features grew and laid
    // over it, its plants, trees and snow, stands above it).
    for x in 0..16usize {
        for z in 0..16usize {
            out.columns[x][z] = column_is(land, seed, x0 + x as i32, z0 + z as i32, shares[x / 4][z / 4]);
            out.tops[x][z] = chunk.heightmaps.get(HeightmapKind::WorldSurfaceWg, x, z) - 1;
        }
    }
    let ceiling = chunk.min_y() + chunk.height() - 1;
    // The flats: each column of one cut down or built up to its height
    // (what stood over it gone, the block under its top filling up to its
    // top block), its eighths the flat's own.
    let mut flat_fraction: [[Option<f32>; 16]; 16] = [[None; 16]; 16];
    let near_flats = !flats_near(land, &gen.terrain, x0 + 8, z0 + 8).is_empty()
        || [(0, 0), (15, 0), (0, 15), (15, 15)].iter().any(|&(dx, dz)| !flats_near(land, &gen.terrain, x0 + dx, z0 + dz).is_empty());
    if near_flats {
        for x in 0..16usize {
            for z in 0..16usize {
                if !out.columns[x][z] {
                    continue;
                }
                let (wx, wz) = (x0 + x as i32, z0 + z as i32);
                let Some((w, h, whole)) = flat_whole_at(land, &gen.terrain, wx, wz) else { continue };
                let t = out.tops[x][z];
                // (Not a lake's or the sea's water.)
                if chunk.heightmaps.get(HeightmapKind::OceanFloorWg, x, z) < chunk.heightmaps.get(HeightmapKind::WorldSurfaceWg, x, z) {
                    continue;
                }
                let level = h as f32 + if whole { 1.0 } else { 0.5 };
                let goal = t as f32 + chunk.generation.surface_fraction(x, z) + (level - (t as f32 + chunk.generation.surface_fraction(x, z))) * w;
                let target = goal.floor() as i32;
                let (top_block, under) = (chunk.block(x, t, z), chunk.block(x, t - 1, z));
                if target < t {
                    for y in target + 1..=(t + ABOVE).min(ceiling) {
                        chunk.set_block_raw(x, y, z, air, &registries);
                    }
                    chunk.set_block_raw(x, target, z, top_block, &registries);
                } else if target > t {
                    for y in t..target {
                        chunk.set_block_raw(x, y, z, under, &registries);
                    }
                    chunk.set_block_raw(x, target, z, top_block, &registries);
                    for y in target + 1..=(t + ABOVE).min(ceiling) {
                        chunk.set_block_raw(x, y, z, air, &registries);
                    }
                } else {
                    for y in t + 1..=(t + ABOVE).min(ceiling) {
                        chunk.set_block_raw(x, y, z, air, &registries);
                    }
                }
                out.tops[x][z] = target;
                flat_fraction[x][z] = Some(goal - target as f32);
            }
        }
    }
    for x in 0..16usize {
        for z in 0..16usize {
            if !out.columns[x][z] {
                continue;
            }
            let t = out.tops[x][z];
            let rise = |reach: i32| {
                [(1i32, 0i32), (-1, 0), (0, 1), (0, -1)]
                    .iter()
                    .map(|&(dx, dz)| {
                        let (nx, nz) = (x as i32 + dx * reach, z as i32 + dz * reach);
                        if (0..16).contains(&nx) && (0..16).contains(&nz) { (out.tops[nx as usize][nz as usize] - t).abs() } else { 0 }
                    })
                    .max()
                    .unwrap_or(0)
            };
            let slope = if rise(1) >= land.steep {
                Slope::Steep
            } else if rise(1) == land.steep - 1 {
                Slope::Edge
            } else if rise(2) >= land.steep {
                Slope::Foot
            } else {
                Slope::Flat
            };
            out.slopes[x][z] = slope;
            // Down from what grew over it to its depth: what lies on it
            // taken off, each block by its rule (over the top, as on it).
            for y in (t - land.depth..=(t + ABOVE).min(ceiling)).rev() {
                let old = chunk.block(x, y, z);
                if old == air {
                    continue;
                }
                let block = registries.blocks.block_of(old);
                if strip.contains(&block) {
                    chunk.set_block_raw(x, y, z, air, &registries);
                    continue;
                }
                let Some((_, rule)) = rules.iter().find(|(b, _)| *b == block) else { continue };
                if let Some(name) = (rule.to)(slope, (t - y).max(0)) {
                    let id = carried(&registries.blocks, old, state(gen, states, name));
                    chunk.set_block_raw(x, y, z, id, &registries);
                }
            }
            // Its eighths, over its top block, up to the true surface.
            let fraction = flat_fraction[x][z].unwrap_or_else(|| chunk.generation.surface_fraction(x, z));
            if chunk.block(x, t + 1, z) != air {
                continue;
            }
            let top = chunk.block(x, t, z);
            let Some(layers) = land.layers.iter().find(|l| l.tops.iter().any(|n| state(gen, states, n) == top)) else { continue };
            let n = ((fraction * 8.0).round() as usize).max(layers.least);
            if n == 0 {
                continue;
            }
            let id = if n >= 8 { state(gen, states, layers.whole) } else { state(gen, states, layers.eighths[n - 1]) };
            chunk.set_block_raw(x, t + 1, z, id, &registries);
            out.layered[x][z] = true;
        }
    }
    if let Some((wall, floor)) = land.caves {
        caves(chunk, gen, states, &out, (wall, floor));
    }
    Some(out)
}

/// `to` with every property it shares with `from` as `from` has it (a log's
/// axis, leaves' distance and persistence, a vine's faces).
pub fn carried(blocks: &minecraftoss_core::BlockRegistry, from: BlockStateId, to: BlockStateId) -> BlockStateId {
    let info = blocks.block(blocks.block_of(from));
    info.properties().iter().fold(to, |state, property| {
        blocks
            .property(from, &property.name)
            .and_then(|value| blocks.with_property(state, &property.name, value))
            .unwrap_or(state)
    })
}

/// Under the land's columns its caves are its own: the stone the cave's
/// air touches (in this chunk) its wall, and where the air is over it its
/// floor.
fn caves(chunk: &mut Chunk, gen: &WorldGen, states: &mut HashMap<&'static str, BlockStateId>, shaped: &Shaped, (wall, floor): (&'static str, &'static str)) {
    let registries = gen.terrain.registries.clone();
    let stone = state(gen, states, "minecraft:stone");
    let (wall, floor) = (state(gen, states, wall), state(gen, states, floor));
    let air = BlockStateId::AIR;
    let bottom = chunk.min_y();
    let mut found = Vec::new();
    for x in 0..16usize {
        for z in 0..16usize {
            if !shaped.columns[x][z] {
                continue;
            }
            for y in bottom + 1..shaped.tops[x][z] - 1 {
                if chunk.block(x, y, z) != stone {
                    continue;
                }
                let open = |dx: i32, dy: i32, dz: i32| {
                    let (nx, nz) = (x as i32 + dx, z as i32 + dz);
                    (0..16).contains(&nx) && (0..16).contains(&nz) && chunk.block(nx as usize, y + dy, nz as usize) == air
                };
                if open(0, 1, 0) {
                    found.push((x, y, z, floor));
                } else if open(1, 0, 0) || open(-1, 0, 0) || open(0, 0, 1) || open(0, 0, -1) || open(0, -1, 0) {
                    found.push((x, y, z, wall));
                }
            }
        }
    }
    for (x, y, z, id) in found {
        chunk.set_block_raw(x, y, z, id, &registries);
    }
}

/// Every block a land names, for the tests: each one the game knows.
pub fn names(land: &Land) -> Vec<&'static str> {
    let slopes = [Slope::Flat, Slope::Edge, Slope::Steep, Slope::Foot];
    let mut out: Vec<&'static str> = land.strip.to_vec();
    for rule in land.rules {
        out.push(rule.from);
        for slope in slopes {
            for depth in 0..=land.depth {
                out.extend((rule.to)(slope, depth));
            }
        }
    }
    for layers in land.layers {
        out.extend(layers.tops);
        out.push(layers.whole);
        out.extend(layers.eighths);
    }
    if let Some((wall, floor)) = land.caves {
        out.extend([wall, floor]);
    }
    out.sort_unstable();
    out.dedup();
    out
}

/// Every Jak land the generator makes, in the order it shapes them.
pub const ALL: [&Land; 3] = [&crate::wasteland::LAND, &crate::snowy::LAND, &crate::swamp::LAND];

/// A land by its name in the console (`mc tierra paramo|nieve|pantano`).
pub fn named(name: &str) -> Option<&'static Land> {
    match name {
        "paramo" | "desierto" | "wasteland" => Some(&crate::wasteland::LAND),
        "nieve" | "montana" | "snow" => Some(&crate::snowy::LAND),
        "pantano" | "swamp" => Some(&crate::swamp::LAND),
        _ => None,
    }
}

/// The nearest column well inside `land` to `(x, z)` (tests, `mc tierra`):
/// searched outward in 32-block steps up to `reach`, it and eight probes
/// round it (24 and 48 blocks off) in the land's biomes; `dry`, no water
/// over any of them. Its name and its ground's height.
pub fn nearest(
    land: &Land,
    registries: std::sync::Arc<minecraftoss_core::Registries>,
    seed: i64,
    (x, z): (i32, i32),
    reach: i32,
    dry: bool,
) -> Option<(&'static str, (i32, i32, i32))> {
    let terrain = minecraftoss_generator::terrain::TerrainGenerator::overworld(registries.clone(), seed).ok()?;
    let ids: Vec<_> = land.biomes.iter().filter_map(|b| registries.biomes.id(b)).collect();
    let inside = |x: i32, z: i32| {
        let y = terrain.base_height(x, z, HeightmapKind::WorldSurfaceWg);
        (!dry || y == terrain.base_height(x, z, HeightmapKind::OceanFloorWg)) && ids.contains(&terrain.biome_at_quart(x >> 2, y >> 2, z >> 2))
    };
    for ring in 0..=reach / 32 {
        let r = ring * 32;
        for i in -ring..=ring {
            for (dx, dz) in [(i * 32, -r), (i * 32, r), (-r, i * 32), (r, i * 32)] {
                let (px, pz) = (x + dx, z + dz);
                let deep = inside(px, pz)
                    && [(24, 0), (-24, 0), (0, 24), (0, -24), (48, 48), (-48, -48), (48, -48), (-48, 48)].iter().all(|&(a, b)| inside(px + a, pz + b));
                if deep {
                    return Some((land.name, (px, terrain.base_height(px, pz, HeightmapKind::WorldSurfaceWg), pz)));
                }
            }
        }
    }
    None
}

#[cfg(test)]
mod flat_tests {
    use super::*;
    use std::sync::Arc;

    /// The Wasteland's nearest flat (seed 12345), generated: its middle's
    /// columns within a block of its height, so a vehicle drives across.
    #[test]
    fn a_flat_is_flat() {
        // (Only with the games' places in the world, `IW4L_LUGARES_JAK=1`.)
        if !crate::original_places::jak_places_on() {
            return;
        }
        let Ok(paths) = minecraftoss_core::registries::DataPaths::discover() else { return };
        if !paths.block_catalog.is_file() {
            return;
        }
        let registries = Arc::new(minecraftoss_core::Registries::load(&paths).unwrap());
        let terrain = Arc::new(minecraftoss_generator::terrain::TerrainGenerator::overworld(registries.clone(), 12345).unwrap());
        let land = &crate::wasteland::LAND;
        let desert = registries.biomes.id("minecraft:desert").unwrap();
        // A flat whose middle is the land's, near where `mc desierto` goes.
        let found = (-30..30).flat_map(|i| (-30..30).map(move |j| (i, j))).find_map(|(i, j)| {
            let (x, z) = (-1300 + i * 48, 512 + j * 48);
            let (w, h) = flat_at(land, &terrain, x, z)?;
            let y = terrain.base_height(x, z, HeightmapKind::WorldSurfaceWg);
            (w >= 1.0 && terrain.biome_at_quart(x >> 2, y >> 2, z >> 2) == desert && !crate::original_places::inside_a_place(&registries, 12345, x, z)).then_some((x, z, h))
        });
        let Some((x, z, h)) = found else { panic!("no flat in the Wasteland near") };
        let gen = Arc::new(WorldGen::new(terrain).unwrap());
        crate::jak_structures::install();
        let mut map = minecraftoss_world::chunk_map::ChunkMap::with_worldgen(gen, 2, 4);
        let chunk = map.load_now(minecraftoss_core::ChunkPos::new(x >> 4, z >> 4));
        let mut tops = Vec::new();
        for lx in 0..16 {
            for lz in 0..16 {
                let wx = chunk.pos.x * 16 + lx as i32;
                let wz = chunk.pos.z * 16 + lz as i32;
                if flat_at(land, &map_terrain(&registries), wx, wz).is_some_and(|(w, _)| w >= 1.0) {
                    let top = (chunk.min_y()..chunk.min_y() + chunk.height()).rev().find(|&y| !registries.blocks.is_air(chunk.block(lx, y, lz))).unwrap();
                    tops.push(top);
                }
            }
        }
        let (lo, hi) = (*tops.iter().min().unwrap(), *tops.iter().max().unwrap());
        println!("flat at {x} {z} height {h}: {} columns, tops {lo}..{hi}", tops.len());
        assert!(tops.len() > 100 && hi - lo <= 1, "{lo}..{hi}");
    }

    fn map_terrain(registries: &Arc<minecraftoss_core::Registries>) -> minecraftoss_generator::terrain::TerrainGenerator {
        minecraftoss_generator::terrain::TerrainGenerator::overworld(registries.clone(), 12345).unwrap()
    }
}
