//! The universe map's Nether planet: the Nether as it generates, its bedrock
//! roof turned into the planet's crust (netherrack over it, and the floor
//! of the biome below on top), drilled by shafts down into the caverns, to
//! fly or drop into from the sky. A shaft's mouth is ringed with a bastion's
//! polished blackstone and lit by shroomlights; down through the rock its
//! sides are a fortress's nether bricks.
//!
//! Where the shafts are is a pure function of the seed, so whatever draws
//! the planet from afar can show them too ([`shaft`], [`nearest`]).

use minecraftoss_core::{BlockStateId, Chunk, Registries};

/// Blocks on a side of the square cells shafts are laid out in, one at most
/// in each.
pub const CELL: i32 = 160;
/// The top of the Nether's bedrock roof.
pub const ROOF: i32 = 127;
/// The lowest a shaft is dug: above the lava sea.
const FLOOR: i32 = 34;
/// Air blocks in a row that say a shaft has broken into a cavern.
const OPEN: i32 = 6;

fn hash(seed: i64, x: i32, z: i32, salt: u64) -> u64 {
    let mut h = (seed as u64)
        ^ (x as u32 as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15)
        ^ (z as u32 as u64).wrapping_mul(0xC2B2_AE3D_27D4_EB4F)
        ^ salt.wrapping_mul(0x1656_67B1_9E37_79F9);
    h ^= h >> 30;
    h = h.wrapping_mul(0xBF58_476D_1CE4_E5B9);
    h ^= h >> 27;
    h = h.wrapping_mul(0x94D0_49BB_1331_11EB);
    h ^ (h >> 31)
}

/// The shaft of a cell, if it has one: its center column and its radius.
pub fn shaft(seed: i64, cx: i32, cz: i32) -> Option<(i32, i32, i32)> {
    let h = hash(seed, cx, cz, 1);
    if h % 100 >= 70 {
        return None;
    }
    let room = (CELL - 40) as u64;
    let x = cx * CELL + 20 + ((h >> 8) % room) as i32;
    let z = cz * CELL + 20 + ((h >> 24) % room) as i32;
    let r = 4 + ((h >> 40) % 5) as i32;
    Some((x, z, r))
}

/// The nearest shaft to a column: its center, its radius and how far the
/// column is from its center.
pub fn nearest(seed: i64, x: i32, z: i32) -> Option<(i32, i32, i32, f32)> {
    let (cx, cz) = (x.div_euclid(CELL), z.div_euclid(CELL));
    let mut best: Option<(i32, i32, i32, f32)> = None;
    for dz in -1..=1 {
        for dx in -1..=1 {
            if let Some((sx, sz, r)) = shaft(seed, cx + dx, cz + dz) {
                let d = ((x - sx) as f32).hypot((z - sz) as f32);
                if best.is_none_or(|b| d < b.3) {
                    best = Some((sx, sz, r, d));
                }
            }
        }
    }
    best
}

/// How thick the crust over the roof is at a column (1 to 3 blocks), in
/// broad steps so it reads as ground and not as noise.
pub fn crust(seed: i64, x: i32, z: i32) -> i32 {
    1 + (hash(seed, x.div_euclid(6), z.div_euclid(6), 2) % 3) as i32
}

/// The blocks the planet is built with.
pub struct NetherPlanet {
    netherrack: BlockStateId,
    nether_bricks: BlockStateId,
    rim: BlockStateId,
    gilded: BlockStateId,
    light: BlockStateId,
    crimson: BlockStateId,
    warped: BlockStateId,
    soul: BlockStateId,
    blackstone: BlockStateId,
}

impl NetherPlanet {
    pub fn new(registries: &Registries) -> Result<Self, String> {
        let b = &registries.blocks;
        Ok(Self {
            netherrack: b.parse_state("minecraft:netherrack")?,
            nether_bricks: b.parse_state("minecraft:nether_bricks")?,
            rim: b.parse_state("minecraft:polished_blackstone_bricks")?,
            gilded: b.parse_state("minecraft:gilded_blackstone")?,
            light: b.parse_state("minecraft:shroomlight")?,
            crimson: b.parse_state("minecraft:crimson_nylium")?,
            warped: b.parse_state("minecraft:warped_nylium")?,
            soul: b.parse_state("minecraft:soul_soil")?,
            blackstone: b.parse_state("minecraft:blackstone")?,
        })
    }

    /// Lays the crust and digs the shafts of a chunk whose terrain is built
    /// (before its heightmaps and features).
    pub fn dig(&self, seed: i64, chunk: &mut Chunk, registries: &Registries) {
        let (x0, z0) = (chunk.pos.min_block_x(), chunk.pos.min_block_z());
        let blocks = &registries.blocks;
        for lz in 0..16usize {
            for lx in 0..16usize {
                let (x, z) = (x0 + lx as i32, z0 + lz as i32);
                // The crust takes the floor of the biome under the roof.
                let biome = chunk.biome(lx >> 2, 100 >> 2, lz >> 2);
                let name = registries.biomes.get(biome).name.path();
                let top_block = match name {
                    "crimson_forest" => self.crimson,
                    "warped_forest" => self.warped,
                    "soul_sand_valley" => self.soul,
                    "basalt_deltas" => self.blackstone,
                    _ => self.netherrack,
                };
                let thick = crust(seed, x, z);
                let top = ROOF + thick;
                for y in ROOF + 1..top {
                    chunk.set_block_raw(lx, y, lz, self.netherrack, registries);
                }
                chunk.set_block_raw(lx, top, lz, top_block, registries);
                let Some((sx, sz, r, d)) = nearest(seed, x, z) else {
                    continue;
                };
                let r = r as f32;
                if d <= r {
                    // Down through the crust and the roof, then the rock,
                    // until a cavern opens below.
                    for y in (ROOF - 4..=top).rev() {
                        chunk.set_block_raw(lx, y, lz, minecraftoss_core::BlockStateId::AIR, registries);
                    }
                    let mut open = 0;
                    for y in (FLOOR..ROOF - 4).rev() {
                        let b = chunk.block(lx, y, lz);
                        if blocks.has_fluid(b) {
                            break;
                        }
                        if blocks.is_air(b) {
                            open += 1;
                            if open >= OPEN {
                                break;
                            }
                            continue;
                        }
                        open = 0;
                        chunk.set_block_raw(lx, y, lz, minecraftoss_core::BlockStateId::AIR, registries);
                    }
                } else if d <= r + 1.5 {
                    // The shaft's side: bricks wherever it runs through rock.
                    let mut open = 0;
                    for y in (FLOOR..=top).rev() {
                        let b = chunk.block(lx, y, lz);
                        if blocks.has_fluid(b) {
                            break;
                        }
                        if blocks.is_air(b) {
                            open += 1;
                            if open >= OPEN && y < ROOF - 4 {
                                break;
                            }
                            continue;
                        }
                        open = 0;
                        chunk.set_block_raw(lx, y, lz, self.nether_bricks, registries);
                    }
                } else if d <= r + 3.5 {
                    // The mouth: a ring of polished blackstone a step over
                    // the crust, gilded here and there, with shroomlights
                    // at its four sides.
                    let angle = ((z - sz) as f32).atan2((x - sx) as f32);
                    let quarter = (angle / std::f32::consts::FRAC_PI_2).rem_euclid(1.0);
                    let block = if quarter < 0.06 || quarter > 0.94 {
                        self.light
                    } else if hash(seed, x, z, 3) % 9 == 0 {
                        self.gilded
                    } else {
                        self.rim
                    };
                    chunk.set_block_raw(lx, top, lz, self.rim, registries);
                    chunk.set_block_raw(lx, top + 1, lz, block, registries);
                }
            }
        }
    }
}
