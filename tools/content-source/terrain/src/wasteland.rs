//! Jak 3's Wasteland, the desert round Spargus, as the block world's
//! deserts (Adrii 23/10: one Jak biome inside Minecraft's own generation,
//! to judge the way before any other).
//!
//! The ground stays Minecraft's (its noise, its caves, its water and its
//! deep stone), and as each desert chunk becomes final its materials and
//! its features are the Wasteland's:
//! - the dunes Jak 3's sand (`des-beach-01`), over its packed earth rock
//!   (`des-mount-*`, where Minecraft's sandstone was) down to the stone, so
//!   a pit dug shows the same layers as the desert's cliffs and cuts;
//! - slopes and drops its cliff rock (`des-cliff-01`, `des-cliff-top-01` on
//!   its top), the bare faces of the Wasteland's mountains;
//! - the dunes smooth as Jak 3's (Adrii 23/10: up and over them on foot
//!   and in the vehicles, no whole-block steps): over each column's sand
//!   (or its rock, on the rocky rises: cliffs stay cliffs), the same in
//!   eighths of a block up to where Minecraft's own terrain truly
//!   tops out inside that block (the noise's density crossing zero, kept
//!   as the chunk is filled, `GenerationData::surface_fraction`), so a
//!   slope rises in eighths, the same from chunk to chunk;
//! - two-block slopes the sand-to-rock edge (`des-cliff-trans-01`), the
//!   ground round the cliffs' feet scrubby rock (`des-rock-shrub-01`), the
//!   cliff tops one of Jak 3's five at random;
//! - its caves' walls and floors Jak 3's desert caves' (`des-cave-wall-01`,
//!   `des-cave-floor-01`), so digging down into one is still the Wasteland;
//! - its dry grass and pebbles (`des-sand-grass-01`, `des-shrub-pebbles`)
//!   where Minecraft had dead bushes and on the open sand, palms
//!   (`des-palmtree-trunk-02`, `des-palm-leaf-01`) by its water;
//! - its red rock spires and flat-topped mesas (`des-red-rock-01` banded
//!   with the cliff rock), the silhouettes seen from afar;
//! - now and then a hamlet: a house of Spargus copied from its levels in a
//!   clearing of its ground, ditches to the hamlets round it
//!   (`jak_hamlets`; the kit's outposts, still made for `mc spargus`, are no
//!   longer put in the world).
//!
//! The edge with the deserts' neighbours goes column by column, more of the
//! Wasteland's the deeper in (its share of desert round each column, the
//! columns drawn by a fixed hash): no straight border.
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use minecraftoss_core::block::BlockStateId;
use minecraftoss_core::chunk::{Chunk, HeightmapKind};
use minecraftoss_world::chunk_map::WorldGen;

use crate::jak_structures::{Land, Placed};

/// The Wasteland's blocks (`MASHUP_BLOCKS`).
pub const SAND: &str = "jak:wasteland_sand";
pub const ROCK: &str = "jak:wasteland_rock";
pub const CLIFF: &str = "jak:wasteland_cliff";
pub const RED_ROCK: &str = "jak:red_rock";
pub const DRY_GRASS: &str = "jak:dry_grass";
pub const PEBBLES: &str = "jak:desert_pebbles";
pub const PALM_LOG: &str = "jak:palm_log[axis=y]";
pub const PALM_LEAVES: &str = "jak:palm_leaves[persistent=true]";
pub const GROUND: &str = "jak:spargus_ground";
pub const STUCCO: &str = "jak:spargus_stucco";
pub const STUCCO_SLAB: &str = "jak:spargus_stucco_slab[type=bottom]";
pub const BRICKS: &str = "jak:spargus_bricks";
pub const ROOF: &str = "jak:spargus_roof";
pub const PLANKS: &str = "jak:spargus_planks";
pub const TOTEM: &str = "jak:totem_stone";
pub const CLIFF_EDGE: &str = "jak:wasteland_cliff_edge";
pub const SCRUB_ROCK: &str = "jak:wasteland_scrub_rock";
pub const CAVE_ROCK: &str = "jak:wasteland_cave_rock";
pub const CAVE_FLOOR: &str = "jak:wasteland_cave_floor";
/// The rock in eighths: one to seven layers.
pub const ROCK_LAYERS: [&str; 7] = [
    "jak:wasteland_rock_layer[layers=1]",
    "jak:wasteland_rock_layer[layers=2]",
    "jak:wasteland_rock_layer[layers=3]",
    "jak:wasteland_rock_layer[layers=4]",
    "jak:wasteland_rock_layer[layers=5]",
    "jak:wasteland_rock_layer[layers=6]",
    "jak:wasteland_rock_layer[layers=7]",
];
/// The sand in eighths: one to seven layers (eight is a whole block).
pub const SAND_LAYERS: [&str; 7] = [
    "jak:wasteland_sand_layer[layers=1]",
    "jak:wasteland_sand_layer[layers=2]",
    "jak:wasteland_sand_layer[layers=3]",
    "jak:wasteland_sand_layer[layers=4]",
    "jak:wasteland_sand_layer[layers=5]",
    "jak:wasteland_sand_layer[layers=6]",
    "jak:wasteland_sand_layer[layers=7]",
];

/// The Wasteland as a Jak land (`jak_lands`): Minecraft's deserts, their
/// sand its sand (its cliff rock on cliffs, the sand-to-rock edge on
/// two-block slopes, scrub at the cliffs' feet), their sandstone its packed
/// earth rock (cliff rock on cliffs), the stone of a cliff's face its cliff
/// rock, their dead bushes its dry grass; sand and rock tops in eighths; its
/// caves Jak 3's desert caves.
pub const LAND: crate::jak_lands::Land = crate::jak_lands::Land {
    name: "Wasteland",
    biomes: &["minecraft:desert"],
    depth: DEPTH,
    steep: STEEP,
    salt: 71,
    strip: &[],
    rules: &[
        crate::jak_lands::Rule { from: "minecraft:sand", to: sand_to },
        crate::jak_lands::Rule { from: "minecraft:sandstone", to: |slope, _| Some(if slope == Slope::Steep { CLIFF } else { ROCK }) },
        crate::jak_lands::Rule { from: "minecraft:stone", to: |slope, depth| (slope == Slope::Steep && depth < 4).then_some(CLIFF) },
        crate::jak_lands::Rule { from: "minecraft:dead_bush", to: |_, _| Some(DRY_GRASS) },
    ],
    layers: &[
        crate::jak_lands::Layers { tops: &[SAND], whole: SAND, eighths: SAND_LAYERS, least: 0 },
        crate::jak_lands::Layers { tops: &[ROCK, SCRUB_ROCK, CLIFF_EDGE], whole: ROCK, eighths: ROCK_LAYERS, least: 0 },
    ],
    caves: Some((CAVE_ROCK, CAVE_FLOOR)),
};

use crate::jak_lands::Slope;

fn sand_to(slope: Slope, depth: i32) -> Option<&'static str> {
    Some(match slope {
        Slope::Steep if depth < 4 => CLIFF,
        Slope::Edge if depth < 2 => CLIFF_EDGE,
        Slope::Foot if depth == 0 => SCRUB_ROCK,
        _ => SAND,
    })
}

/// How deep under the surface the ground is the Wasteland's (blocks).
const DEPTH: i32 = 12;
/// A column this much higher or lower than one beside it is a slope: cliff.
const STEEP: i32 = 3;
/// One rock formation a square this size, perhaps.
const ROCK_CELL: i32 = 40;
/// One outpost a square this size, perhaps.
const OUTPOST_CELL: i32 = 288;
const OUTPOSTS_IN: u64 = 45;

fn mix(seed: i64, a: i32, b: i32, salt: u64) -> u64 {
    let mut h = (seed as u64) ^ ((a as u32 as u64) << 32 | u64::from(b as u32)) ^ salt.wrapping_mul(0x9E37_79B9_7F4A_7C15);
    h ^= h >> 33;
    h = h.wrapping_mul(0xff51_afd7_ed55_8ccd);
    h ^= h >> 33;
    h = h.wrapping_mul(0xc4ce_b9fe_1a85_ec53);
    h ^ (h >> 33)
}

/// The states this chunk's work uses, each looked up once.
struct Ids {
    air: BlockStateId,
    sand: BlockStateId,
    sandstone: BlockStateId,
    dead_bush: BlockStateId,
    stone: BlockStateId,
    water: BlockStateId,
    jak: HashMap<&'static str, BlockStateId>,
}

impl Ids {
    fn new(gen: &WorldGen) -> Self {
        let blocks = &gen.terrain.registries.blocks;
        let state = |name: &str| blocks.parse_state(name).unwrap_or(BlockStateId::AIR);
        let mut jak = HashMap::new();
        for name in [SAND, ROCK, CLIFF, RED_ROCK, DRY_GRASS, PEBBLES, PALM_LOG, PALM_LEAVES, CLIFF_EDGE, SCRUB_ROCK, CAVE_ROCK, CAVE_FLOOR]
            .into_iter()
            .chain(SAND_LAYERS)
            .chain(ROCK_LAYERS)
        {
            jak.insert(name, state(name));
        }
        Self {
            air: BlockStateId::AIR,
            sand: state("minecraft:sand"),
            sandstone: state("minecraft:sandstone"),
            dead_bush: state("minecraft:dead_bush"),
            stone: state("minecraft:stone"),
            water: state("minecraft:water"),
            jak,
        }
    }

    fn get(&self, name: &'static str) -> BlockStateId {
        self.jak.get(name).copied().unwrap_or(self.air)
    }
}

/// Whether the column at `(x, z)` (world) is the Wasteland's: a desert
/// column, by its share of desert round it at its quart, through the hash.
fn wasteland_column(seed: i64, x: i32, z: i32, share: f32) -> bool {
    crate::jak_lands::column_is(&LAND, seed, x, z, share)
}

/// The top block's height at a chunk column (the first not air from the
/// top, water counted).
fn top(chunk: &Chunk, x: usize, z: usize) -> i32 {
    chunk.heightmaps.get(HeightmapKind::WorldSurfaceWg, x, z) - 1
}

/// Makes a desert chunk the Wasteland's: its ground, its plants, its rock
/// formations and its hamlets.
pub fn decorate(chunk: &mut Chunk, gen: &WorldGen, states: &mut HashMap<&'static str, BlockStateId>) {
    if !gen.sky_light {
        return;
    }
    let started = std::time::Instant::now();
    let Some(shaped) = crate::jak_lands::shape(&LAND, chunk, gen, states) else { return };
    let seed = gen.terrain.seed;
    let registries = gen.terrain.registries.clone();
    let ids = Ids::new(gen);
    let (x0, z0) = (chunk.pos.x * 16, chunk.pos.z * 16);
    let (columns, tops) = (shaped.columns, shaped.tops);
    // The open sand's dry grass and pebbles (where no eighths lie).
    for x in 0..16usize {
        for z in 0..16usize {
            let t = tops[x][z];
            if !columns[x][z] || shaped.layered[x][z] || chunk.block(x, t, z) != ids.get(SAND) || chunk.block(x, t + 1, z) != ids.air {
                continue;
            }
            let (wx, wz) = (x0 + x as i32, z0 + z as i32);
            match mix(seed, wx, wz, 72) % 100 {
                0..=4 => {
                    chunk.set_block_raw(x, t + 1, z, ids.get(DRY_GRASS), &registries);
                }
                5..=6 => {
                    chunk.set_block_raw(x, t + 1, z, ids.get(PEBBLES), &registries);
                }
                _ => {}
            }
        }
    }
    // Palms by the water: on its sand, a crown two blocks round (inside the
    // chunk, so whole).
    for x in 2..14usize {
        for z in 2..14usize {
            if !columns[x][z] {
                continue;
            }
            let (wx, wz) = (x0 + x as i32, z0 + z as i32);
            let t = tops[x][z];
            if chunk.block(x, t, z) != ids.get(SAND) || mix(seed, wx, wz, 73) % 100 >= 7 {
                continue;
            }
            // Water within three blocks (inside the chunk).
            let wet = (-3i32..=3).any(|dx| {
                (-3i32..=3).any(|dz| {
                    let (nx, nz) = (x as i32 + dx, z as i32 + dz);
                    (0..16).contains(&nx) && (0..16).contains(&nz) && chunk.block(nx as usize, t, nz as usize) == ids.water
                })
            });
            if !wet {
                continue;
            }
            palm(chunk, &ids, &registries, (x, t + 1, z), mix(seed, wx, wz, 74));
        }
    }
    rock_formations(chunk, gen, &ids, &registries, seed);
    // Its villages: Spargus's houses of facade blocks (`jak_facades`). The
    // hamlets of pieces and the town of 23/10 only with
    // `IW4L_LUGARES_JAK=1`.
    if crate::original_places::jak_places_on() {
        crate::jak_hamlets::place(2, chunk, gen, states);
        crate::spargus_town::place(chunk, gen, states);
    } else {
        crate::jak_facades::place_spargus(chunk, gen, states);
    }
    if let Ok(mut cost) = COST.lock() {
        cost.0 += 1;
        cost.1 += started.elapsed().as_secs_f64() * 1000.0;
    }
}

/// What the Wasteland has cost its chunks: how many, and their
/// milliseconds in all.
static COST: Mutex<(u32, f64)> = Mutex::new((0, 0.0));

/// The Wasteland's chunks made so far and its mean milliseconds a chunk.
pub fn chunk_cost() -> (u32, f64) {
    COST.lock().map(|c| (c.0, c.1 / f64::from(c.0.max(1)))).unwrap_or_default()
}

/// A palm: its trunk 5 to 7 blocks, its crown of fronds round its top.
fn palm(chunk: &mut Chunk, ids: &Ids, registries: &Arc<minecraftoss_core::Registries>, (x, y, z): (usize, i32, usize), h: u64) {
    let tall = 5 + (h % 3) as i32;
    for k in 0..tall {
        chunk.set_block_raw(x, y + k, z, ids.get(PALM_LOG), registries);
    }
    let crown = y + tall;
    let leaves = ids.get(PALM_LEAVES);
    let mut put = |dx: i32, dy: i32, dz: i32| {
        let (px, pz) = (x as i32 + dx, z as i32 + dz);
        if (0..16).contains(&px) && (0..16).contains(&pz) && chunk.block(px as usize, crown + dy, pz as usize) == ids.air {
            chunk.set_block_raw(px as usize, crown + dy, pz as usize, leaves, registries);
        }
    };
    put(0, 0, 0);
    for (dx, dz) in [(1, 0), (-1, 0), (0, 1), (0, -1)] {
        put(dx, 0, dz);
        // The fronds droop as Jak 3's palms' do.
        put(dx * 2, -1, dz * 2);
    }
    for (dx, dz) in [(1, 1), (-1, 1), (1, -1), (-1, -1)] {
        put(dx, -1, dz);
    }
}

/// The red rock's spires and mesas: one perhaps in each 40-block square of
/// the Wasteland, its part over this chunk.
fn rock_formations(chunk: &mut Chunk, gen: &WorldGen, ids: &Ids, registries: &Arc<minecraftoss_core::Registries>, seed: i64) {
    let Some(desert) = gen.terrain.registries.biomes.id("minecraft:desert") else { return };
    let (x0, z0) = (chunk.pos.x * 16, chunk.pos.z * 16);
    let cell_of = |v: i32| v.div_euclid(ROCK_CELL);
    for cx in cell_of(x0 - 14)..=cell_of(x0 + 30) {
        for cz in cell_of(z0 - 14)..=cell_of(z0 + 30) {
            let h = mix(seed, cx, cz, 81);
            if h % 100 >= 45 {
                continue;
            }
            let sx = cx * ROCK_CELL + 8 + (mix(seed, cx, cz, 82) % (ROCK_CELL as u64 - 16)) as i32;
            let sz = cz * ROCK_CELL + 8 + (mix(seed, cx, cz, 83) % (ROCK_CELL as u64 - 16)) as i32;
            let Some(ground) = rock_ground(seed, (cx, cz), || {
                let ground = gen.ground(sx, sz);
                (gen.terrain.biome_at_quart(sx >> 2, ground >> 2, sz >> 2) == desert).then_some(ground)
            }) else {
                continue;
            };
            let mesa = h % 100 < 12;
            let (radius, height) = if mesa {
                (7.0 + (mix(seed, cx, cz, 84) % 5) as f32, 7 + (mix(seed, cx, cz, 85) % 6) as i32)
            } else {
                (2.0 + (mix(seed, cx, cz, 84) % 3) as f32, 12 + (mix(seed, cx, cz, 85) % 15) as i32)
            };
            let reach = radius as i32 + 2;
            for x in 0..16usize {
                for z in 0..16usize {
                    let (wx, wz) = (x0 + x as i32, z0 + z as i32);
                    let (dx, dz) = ((wx - sx) as f32, (wz - sz) as f32);
                    if dx.abs() > reach as f32 || dz.abs() > reach as f32 {
                        continue;
                    }
                    let d = (dx * dx + dz * dz).sqrt();
                    let base = top(chunk, x, z);
                    for k in 0..height {
                        let y = ground + k;
                        // Narrowing up a spire, ragged by its layer; a mesa
                        // straight-sided, its rim worn.
                        let wobble = ((mix(seed, wx, y, 86) % 100) as f32 / 100.0 - 0.5) * 1.2;
                        let r = if mesa {
                            radius + wobble - if k >= height - 2 { 0.8 } else { 0.0 }
                        } else {
                            radius * (1.0 - 0.55 * k as f32 / height as f32) + wobble * 0.6
                        };
                        if d > r {
                            continue;
                        }
                        let banded = k % 5 == 4;
                        let name = if k == height - 1 || banded { CLIFF } else { RED_ROCK };
                        chunk.set_block_raw(x, y, z, ids.get(name), registries);
                    }
                    // Down to the sand under it, so it stands on the dunes.
                    if d <= radius {
                        for y in base.min(ground)..ground {
                            if chunk.block(x, y, z) == ids.air || chunk.block(x, y, z) == ids.get(DRY_GRASS) {
                                chunk.set_block_raw(x, y, z, ids.get(RED_ROCK), registries);
                            }
                        }
                    }
                    // A mesa's top grows its dry grass.
                    if mesa && d <= radius - 1.5 && mix(seed, wx, wz, 87) % 100 < 10 {
                        chunk.set_block_raw(x, ground + height, z, ids.get(DRY_GRASS), registries);
                    }
                }
            }
        }
    }
}

/// A rock formation's ground by its square (none off the desert), worked
/// out once for the chunks round it.
fn rock_ground(seed: i64, cell: (i32, i32), work: impl FnOnce() -> Option<i32>) -> Option<i32> {
    static ROCKS: Mutex<Option<(i64, HashMap<(i32, i32), Option<i32>>)>> = Mutex::new(None);
    if let Some(ground) = ROCKS.lock().ok().and_then(|cache| cache.as_ref().filter(|(s, _)| *s == seed).and_then(|(_, found)| found.get(&cell).copied())) {
        return ground;
    }
    // (Worked out with the lock let go: the other chunks' threads go on.)
    let ground = work();
    if let Ok(mut cache) = ROCKS.lock() {
        let slot = cache.get_or_insert_with(|| (seed, HashMap::new()));
        if slot.0 != seed || slot.1.len() > 65_536 {
            *slot = (seed, HashMap::new());
        }
        slot.1.insert(cell, ground);
    }
    ground
}

/// Whether the land at `(x, z)` is the Wasteland's (the desert at its
/// ground).
pub fn is_wasteland(gen: &WorldGen, x: i32, z: i32) -> bool {
    let Some(desert) = gen.terrain.registries.biomes.id("minecraft:desert") else { return false };
    let y = gen.ground(x, z);
    gen.terrain.biome_at_quart(x >> 2, y >> 2, z >> 2) == desert
}

/// The nearest Wasteland to `(x, z)` (tests, `mc desierto`): a column well
/// inside a desert, searched outward in 32-block steps up to `reach`, and
/// its ground's height.
pub fn nearest(registries: Arc<minecraftoss_core::Registries>, seed: i64, (x, z): (i32, i32), reach: i32) -> Option<(i32, i32, i32)> {
    let terrain = minecraftoss_generator::terrain::TerrainGenerator::overworld(registries.clone(), seed).ok()?;
    let desert = registries.biomes.id("minecraft:desert")?;
    // Dry desert (no lake or river over it).
    let is_desert = |x: i32, z: i32| {
        let y = terrain.base_height(x, z, HeightmapKind::WorldSurfaceWg);
        y == terrain.base_height(x, z, HeightmapKind::OceanFloorWg) && terrain.biome_at_quart(x >> 2, y >> 2, z >> 2) == desert
    };
    for ring in 0..=reach / 32 {
        let r = ring * 32;
        for i in -ring..=ring {
            for (dx, dz) in [(i * 32, -r), (i * 32, r), (-r, i * 32), (r, i * 32)] {
                let (px, pz) = (x + dx, z + dz);
                let deep = is_desert(px, pz)
                    && [(24, 0), (-24, 0), (0, 24), (0, -24), (48, 48), (-48, -48), (48, -48), (-48, 48)].iter().all(|&(a, b)| is_desert(px + a, pz + b));
                if deep {
                    return Some((px, terrain.base_height(px, pz, HeightmapKind::WorldSurfaceWg), pz));
                }
            }
        }
    }
    None
}

/// The Wasteland's edge nearest `(x, z)` (tests, `mc desierto borde`): the
/// first column off the desert along eight headings, in 8-block steps up
/// to `reach`, and its ground's height.
pub fn edge(registries: Arc<minecraftoss_core::Registries>, seed: i64, (x, z): (i32, i32), reach: i32) -> Option<(i32, i32, i32)> {
    let terrain = minecraftoss_generator::terrain::TerrainGenerator::overworld(registries.clone(), seed).ok()?;
    let desert = registries.biomes.id("minecraft:desert")?;
    let off = |x: i32, z: i32| {
        let y = terrain.base_height(x, z, HeightmapKind::OceanFloorWg);
        terrain.biome_at_quart(x >> 2, y >> 2, z >> 2) != desert
    };
    (1..=reach / 8).find_map(|step| {
        [(1, 0), (-1, 0), (0, 1), (0, -1), (1, 1), (-1, -1), (1, -1), (-1, 1)].iter().find_map(|&(a, b)| {
            let (px, pz) = (x + a * step * 8, z + b * step * 8);
            off(px, pz).then(|| (px, terrain.base_height(px, pz, HeightmapKind::WorldSurfaceWg), pz))
        })
    })
}

/// The nearest Spargus outpost the world makes to `(x, z)` (tests, `mc
/// spargus`): its plaza's middle and ground, within `reach` blocks.
pub fn nearest_outpost(registries: Arc<minecraftoss_core::Registries>, seed: i64, (x, z): (i32, i32), reach: i32) -> Option<(i32, i32, i32)> {
    let terrain = minecraftoss_generator::terrain::TerrainGenerator::overworld(registries.clone(), seed).ok()?;
    let desert = registries.biomes.id("minecraft:desert")?;
    struct Ground<'a>(&'a minecraftoss_generator::terrain::TerrainGenerator);
    impl Land for Ground<'_> {
        fn ground(&self, x: i32, z: i32) -> i32 {
            self.0.base_height(x, z, HeightmapKind::OceanFloorWg)
        }
        fn wet(&self, x: i32, z: i32) -> bool {
            self.0.base_height(x, z, HeightmapKind::WorldSurfaceWg) > self.ground(x, z)
        }
    }
    let land = Ground(&terrain);
    let wasteland = |x: i32, z: i32| {
        let y = land.ground(x, z);
        terrain.biome_at_quart(x >> 2, y >> 2, z >> 2) == desert
    };
    let (cx, cz) = (x.div_euclid(OUTPOST_CELL), z.div_euclid(OUTPOST_CELL));
    let rings = reach / OUTPOST_CELL + 1;
    let mut best: Option<(i64, (i32, i32, i32))> = None;
    for dx in -rings..=rings {
        for dz in -rings..=rings {
            let cell = (cx + dx, cz + dz);
            let Some((sx, sz)) = outpost_spot(seed, cell) else { continue };
            if outpost(seed, cell, &land, &wasteland).is_none() {
                continue;
            }
            let d = i64::from(sx - x).pow(2) + i64::from(sz - z).pow(2);
            if best.is_none_or(|(b, _)| d < b) {
                best = Some((d, (sx, land.ground(sx, sz), sz)));
            }
        }
    }
    best.map(|(_, at)| at)
}

/// An outpost's spot in a square (block x, z), if the square has one.
pub fn outpost_spot(seed: i64, cell: (i32, i32)) -> Option<(i32, i32)> {
    if mix(seed, cell.0, cell.1, 91) % 100 >= OUTPOSTS_IN {
        return None;
    }
    let x = cell.0 * OUTPOST_CELL + 48 + (mix(seed, cell.0, cell.1, 92) % (OUTPOST_CELL as u64 - 96)) as i32;
    let z = cell.1 * OUTPOST_CELL + 48 + (mix(seed, cell.0, cell.1, 93) % (OUTPOST_CELL as u64 - 96)) as i32;
    Some((x, z))
}

/// A Spargus outpost at its spot in `cell`: its blocks, or none where the
/// land will not take one (water, steep ground, not the Wasteland).
pub fn outpost(seed: i64, cell: (i32, i32), land: &dyn Land, wasteland: &dyn Fn(i32, i32) -> bool) -> Option<Vec<Placed>> {
    let (cx, cz) = outpost_spot(seed, cell)?;
    if !wasteland(cx, cz) {
        return None;
    }
    let probes = [(0, 0), (11, 0), (-11, 0), (0, 11), (0, -11), (9, 9), (-9, -9), (9, -9), (-9, 9)];
    if probes.iter().any(|&(dx, dz)| land.wet(cx + dx, cz + dz)) {
        return None;
    }
    // All of it on the Wasteland (23/10: one at the desert's edge stood in
    // the savanna, its acacias over the plaza).
    if probes.iter().any(|&(dx, dz)| !wasteland(cx + dx, cz + dz)) {
        return None;
    }
    let heights: Vec<i32> = probes.iter().map(|&(dx, dz)| land.ground(cx + dx, cz + dz)).collect();
    let (lo, hi) = (*heights.iter().min()?, *heights.iter().max()?);
    if hi - lo > 6 {
        return None;
    }
    let y0 = land.ground(cx, cz);
    let mut out: Vec<Placed> = Vec::new();
    // The plaza levelled: its packed ground over the sand, cleared above,
    // the ground under it filled down to where the land is.
    for dx in -13i32..=13 {
        for dz in -13i32..=13 {
            if dx * dx + dz * dz > 169 {
                continue;
            }
            let (x, z) = (cx + dx, cz + dz);
            let g = land.ground(x, z);
            for y in g.min(y0 - 1)..y0 - 1 {
                out.push(((x, y, z), ROCK));
            }
            out.push(((x, y0 - 1, z), if (dx * 7 + dz * 3).rem_euclid(5) == 0 { SAND } else { GROUND }));
            for y in y0..y0 + 8 {
                out.push(((x, y, z), "minecraft:air"));
            }
        }
    }
    // The Wastelanders' totem and its bell.
    for k in 0..3 {
        out.push(((cx, y0 + k, cz), TOTEM));
    }
    out.push(((cx + 1, y0, cz), "minecraft:bell[attachment=floor,facing=east]"));
    // The houses round the plaza, their doors to it.
    let turn = (mix(seed, cell.0, cell.1, 94) % 4) as usize;
    let spots = [(9, 0), (0, 9), (-9, 0), (0, -9)];
    for (k, &(hx, hz)) in spots.iter().enumerate() {
        if k == turn {
            // The fourth side is the market's.
            stall(&mut out, (cx + hx, y0, cz + hz));
            continue;
        }
        house(&mut out, (cx + hx, y0, cz + hz), (-hx.signum(), -hz.signum()));
    }
    // A low wall round it, open at the four ways in, and palms at the gaps.
    for a in 0..96 {
        let angle = a as f32 / 96.0 * std::f32::consts::TAU;
        let (x, z) = (cx + (angle.cos() * 13.0).round() as i32, cz + (angle.sin() * 13.0).round() as i32);
        let (dx, dz) = (x - cx, z - cz);
        if dx.abs() <= 2 || dz.abs() <= 2 {
            continue;
        }
        out.push(((x, y0, z), BRICKS));
        if a % 3 != 0 {
            out.push(((x, y0 + 1, z), BRICKS));
        }
    }
    for (dx, dz) in [(4, 12), (-4, -12)] {
        let (x, z) = (cx + dx, cz + dz);
        for k in 0..6 {
            out.push(((x, y0 + k, z), PALM_LOG));
        }
        for (lx, ly, lz) in [(0, 6, 0), (1, 6, 0), (-1, 6, 0), (0, 6, 1), (0, 6, -1), (2, 5, 0), (-2, 5, 0), (0, 5, 2), (0, 5, -2)] {
            out.push(((x + lx, y0 + ly, z + lz), PALM_LEAVES));
        }
    }
    Some(out)
}

/// A house of Spargus, of its kit (`jak_kit::SPARGUS`): bleached stucco
/// walls seven blocks square joined as Minecraft's walls, metal poles at
/// the corners, its airlock door toward `(fx, fz)` (one step), windows in
/// the sides and back, a gable of its corrugated roof; inside a bed, a
/// table, a barrel and a lamp.
fn house(out: &mut Vec<Placed>, (x, y, z): (i32, i32, i32), (fx, fz): (i32, i32)) {
    use crate::jak_kit::{furnish, house as kit_house, Frame, House, Plan, SPARGUS};
    let frame = Frame { origin: (x, y, z), facing: (fx, fz) };
    let mut plan = Plan::default();
    kit_house(&mut plan, &SPARGUS, frame, House { half: 3, height: 3, stilts: 0 }, &|_, _| y);
    furnish(&mut plan, frame, 3, "minecraft:red_bed");
    plan.placed(out);
}

/// The market stall: four palm posts under a plank roof, its counter of
/// barrels and a stucco ledge.
fn stall(out: &mut Vec<Placed>, (x, y, z): (i32, i32, i32)) {
    for (dx, dz) in [(2, 2), (-2, 2), (2, -2), (-2, -2)] {
        for k in 0..3 {
            out.push(((x + dx, y + k, z + dz), PALM_LOG));
        }
    }
    for dx in -3i32..=3 {
        for dz in -3i32..=3 {
            out.push(((x + dx, y + 3, z + dz), PLANKS));
        }
    }
    for d in -1..=1 {
        out.push(((x + d, y, z - 1), "minecraft:barrel[facing=up]"));
        out.push(((x + d, y, z + 1), STUCCO_SLAB));
    }
}




#[cfg(test)]
mod tests {
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

    /// An outpost stands on flat Wasteland: its totem, its bell, its houses'
    /// walls and roofs, rooms inside with their beds; none off the desert.
    #[test]
    fn an_outpost_has_its_plaza_houses_and_market() {
        let (cell, blocks) = (0..80)
            .find_map(|i| outpost(7, (i, 3), &Flat(70), &|_, _| true).map(|b| ((i, 3), b)))
            .expect("some square has one");
        let (cx, cz) = outpost_spot(7, cell).unwrap();
        assert!(blocks.iter().any(|&(p, s)| p == (cx, 70, cz) && s == TOTEM));
        assert!(blocks.iter().any(|(_, s)| s.starts_with("minecraft:bell")));
        // Three houses of the kit: their walls, roofs and doors.
        assert!(blocks.iter().filter(|(_, s)| s.starts_with("jak:spargus_stucco_wall[")).count() > 3 * 40);
        assert!(blocks.iter().filter(|(_, s)| s.starts_with("jak:spargus_roof_stairs[")).count() >= 3 * 60);
        assert_eq!(blocks.iter().filter(|(_, s)| s.starts_with("jak:spargus_door[") && s.contains("half=lower")).count(), 3);
        assert_eq!(blocks.iter().filter(|(_, s)| s.starts_with("minecraft:red_bed") && s.contains("part=head")).count(), 3);
        assert!(outpost(7, cell, &Flat(70), &|_, _| false).is_none(), "only on the Wasteland");
        assert!(outpost(7, cell, &Flat(70), &|x, _| x != cx + 11).is_none(), "all of it on the Wasteland");
        // Every one of its blocks is a block the world knows the name of.
        for (_, s) in &blocks {
            assert!(s.starts_with("minecraft:") || s.starts_with("jak:"), "{s}");
        }
    }

    #[test]
    fn the_wasteland_edge_goes_by_share() {
        let inside = (0..1000).filter(|&i| wasteland_column(1, i, 0, 0.5)).count();
        assert!((400..600).contains(&inside), "{inside}");
        assert!((0..100).all(|i| wasteland_column(1, i, 0, 1.0)));
        assert!((0..100).all(|i| !wasteland_column(1, i, 0, 0.0)));
    }

    fn registries() -> Option<Arc<minecraftoss_core::Registries>> {
        let paths = minecraftoss_core::registries::DataPaths::discover().ok()?;
        paths.block_catalog.is_file().then(|| Arc::new(minecraftoss_core::Registries::load(&paths).unwrap()))
    }

    /// Every block state an outpost and the Wasteland are made of is one the
    /// game knows (the mashup's own among them).
    #[test]
    fn every_wasteland_block_parses() {
        let Some(registries) = registries() else { return };
        let blocks = (0..80).find_map(|i| outpost(7, (i, 3), &Flat(70), &|_, _| true)).unwrap();
        let mut seen = std::collections::HashSet::new();
        for state in blocks.iter().map(|(_, s)| *s).chain([SAND, ROCK, CLIFF, RED_ROCK, DRY_GRASS, PEBBLES, PALM_LOG, PALM_LEAVES]).chain(crate::jak_lands::names(&LAND)) {
            if seen.insert(state) {
                assert!(registries.blocks.parse_state(state).is_ok(), "{state}");
            }
        }
    }

    /// `cargo test -p minecraft_terrain --lib wasteland_probe -- --ignored
    /// --nocapture`: the biomes the generator gives where `mc desierto`
    /// took Jak, and where it finds the nearest Wasteland, seed 12345.
    #[test]
    #[ignore]
    fn wasteland_probe() {
        let registries = registries().expect("the Minecraft data");
        let terrain = minecraftoss_generator::terrain::TerrainGenerator::overworld(registries.clone(), 12345).unwrap();
        let name = |id: minecraftoss_core::BiomeId| registries.biomes.get(id).name.to_string();
        for (x, z) in [(-2251, 445), (-2240, 515), (0, 0)] {
            let y = terrain.base_height(x, z, HeightmapKind::WorldSurfaceWg);
            println!("({x}, {z}) ground {y}: {}", name(terrain.biome_at_quart(x >> 2, y >> 2, z >> 2)));
        }
        println!("nearest from (106,-34): {:?}", nearest(registries.clone(), 12345, (106, -34), 6000));
    }

    /// The dunes rise in eighths where Minecraft's terrain truly tops out:
    /// on generated Wasteland (seed 12345, near where `mc desierto` takes
    /// Jak) each open sand column stands within half an eighth of its noise
    /// surface, and whole-block steps between sand neighbours are fewer
    /// than on the bare blocks.
    #[test]
    fn the_dunes_rise_in_eighths_on_the_true_surface() {
        let registries = registries().expect("the Minecraft data");
        let terrain = Arc::new(minecraftoss_generator::terrain::TerrainGenerator::overworld(registries.clone(), 12345).unwrap());
        let gen = Arc::new(WorldGen::new(terrain).unwrap());
        crate::jak_structures::install();
        let mut map = minecraftoss_world::chunk_map::ChunkMap::with_worldgen(gen, 2, 4);
        // Round where `mc desierto` took Jak, clear of Spargus (which stands
        // on the nearest Wasteland now, `original_places`).
        let (x, _, z) = [(-1300, 512), (-2208, 1700), (-3300, 512), (-2208, -700)]
            .into_iter()
            .filter_map(|from| nearest(registries.clone(), 12345, from, 1500))
            .find(|&(x, _, z)| !crate::original_places::inside_a_place(&registries, 12345, x, z))
            .expect("Wasteland clear of Spargus");
        let chunk = map.load_now(minecraftoss_core::ChunkPos::new(x >> 4, z >> 4));
        let blocks = &registries.blocks;
        let layer = blocks.block_by_name("jak:wasteland_sand_layer").expect("the layer block");
        let sand = blocks.parse_state(SAND).unwrap();
        // Each column's ground: its top block and the layers over it.
        let mut surface = [[None::<f32>; 16]; 16];
        let mut bare = [[0i32; 16]; 16];
        let mut layered = 0;
        for x in 0..16 {
            for z in 0..16 {
                // (The heightmap is the noise's: what decorating laid over
                // it is a block up.)
                let mut top = chunk.heightmaps.get(HeightmapKind::WorldSurfaceWg, x, z) - 1;
                if !blocks.is_air(chunk.block(x, top + 1, z)) {
                    top += 1;
                }
                let state = chunk.block(x, top, z);
                let (ground, height) = if blocks.block_of(state) == layer {
                    let n: f32 = blocks.property(state, "layers").and_then(|v| v.parse().ok()).unwrap();
                    layered += 1;
                    (top - 1, top as f32 + n / 8.0)
                } else {
                    (top, top as f32 + 1.0)
                };
                bare[x][z] = ground;
                if chunk.block(x, ground, z) == sand {
                    let truth = ground as f32 + chunk.generation.surface_fraction(x, z);
                    assert!((height - 1.0 - truth).abs() <= 1.0 / 16.0 + 1e-4 || height == ground as f32 + 1.0, "{x} {z}: {height} for {truth}");
                    surface[x][z] = Some(height);
                }
            }
        }
        assert!(layered > 20, "layers laid: {layered}");
        let (mut steps, mut bare_steps) = (0, 0);
        for x in 0..15 {
            for z in 0..16 {
                if let (Some(a), Some(b)) = (surface[x][z], surface[x + 1][z]) {
                    steps += usize::from((a - b).abs() >= 1.0);
                    bare_steps += usize::from(bare[x][z] != bare[x + 1][z]);
                }
            }
        }
        println!("{layered} columns layered; whole-block steps {steps} (bare {bare_steps})");
        assert!(steps < bare_steps.max(1), "steps {steps} against {bare_steps}");
    }

    /// `cargo test --release -p minecraft_terrain --lib wasteland_cost --
    /// --ignored --nocapture`: what the Wasteland costs a chunk, on 64
    /// desert chunks round where `mc desierto` takes Jak (seed 12345).
    #[test]
    #[ignore]
    fn wasteland_cost() {
        let registries = registries().expect("the Minecraft data");
        let terrain = Arc::new(minecraftoss_generator::terrain::TerrainGenerator::overworld(registries, 12345).unwrap());
        let gen = Arc::new(WorldGen::new(terrain).unwrap());
        crate::jak_structures::install();
        let mut map = minecraftoss_world::chunk_map::ChunkMap::with_worldgen(gen, 4, 6);
        let started = std::time::Instant::now();
        for cx in -142..-134 {
            for cz in 28..36 {
                map.load_now(minecraftoss_core::ChunkPos::new(cx, cz));
            }
        }
        let (chunks, ms) = chunk_cost();
        println!("{chunks} Wasteland chunks, {ms:.2} ms a chunk; all 64 made in {:.1} s", started.elapsed().as_secs_f64());
        assert!(chunks > 0);
    }
}

