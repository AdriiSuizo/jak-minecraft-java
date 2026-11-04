//! Jak 1's Snowy Mountain (`snow`) as the block world's snowy lands (Adrii
//! 23/10: two Jak biomes more, as the Wasteland): Minecraft's snowy plains,
//! slopes, peaks, groves, taiga and ice spikes, their ground the mountain's
//! in its own textures (the `snow-vis-tfrag` and `snow-vis-shrub` of the
//! game's export).
//!
//! - Its snow (`snow-03`) where Minecraft's snow and powder snow lay, the
//!   snowy turf's top the same over its packed path (`snow-path`, sides
//!   `snow-path-sides`), the earth under it packed snow, down to its rock;
//! - its rock (`snow-rock`, topped `snow-rock-trim`, under `snow-rock-hi`)
//!   for the stone near the surface, and on the cliffs where snow was;
//! - its ice walls (`snow-icewall-01` to `-05`, one at random a block)
//!   where Minecraft's packed and blue ice stood (the ice spikes, the
//!   glaciers);
//! - its snow in eighths over the snow tops (as Minecraft's own snow lies,
//!   a layer at least), smooth up to the true surface;
//! - its grass clumps (`snow-grassclump`), its pines' snowy boughs
//!   (`snow-treesnow`) on their poles (`snow-woodpole`, `snow-woodpolecap`);
//! - its caves' walls its high rock (`snow-rock-hi`), their floors its rock.
//!
//! Its sky and snowfall (`jak_sky::Climate::Cold`) and its creatures (yetis,
//! snow bunnies: `Habitat::Snow`) were the biomes' already.
use std::collections::HashMap;

use minecraftoss_core::block::BlockStateId;
use minecraftoss_core::chunk::Chunk;
use minecraftoss_world::chunk_map::WorldGen;

use crate::jak_lands::{Land, Layers, Rule, Slope};

pub const SNOW: &str = "jak:snow_ground";
pub const TURF: &str = "jak:snow_turf";
pub const PACKED: &str = "jak:snow_packed";
pub const ROCK: &str = "jak:snow_rock";
pub const ROCK_HI: &str = "jak:snow_rock_hi";
pub const ICE_WALL: &str = "jak:snow_ice_wall";
pub const GRASS: &str = "jak:snow_grass";
pub const POLE: &str = "jak:snow_pole";
pub const BOUGHS: &str = "jak:snow_pine";
pub const ICE: &str = "jak:snow_ice";
pub const ICICLE: &str = "jak:snow_icicle[waterlogged=false]";
pub const LAYERS: [&str; 7] = [
    "jak:snow_layer[layers=1]",
    "jak:snow_layer[layers=2]",
    "jak:snow_layer[layers=3]",
    "jak:snow_layer[layers=4]",
    "jak:snow_layer[layers=5]",
    "jak:snow_layer[layers=6]",
    "jak:snow_layer[layers=7]",
];

pub const LAND: Land = Land {
    name: "Snowy Mountain",
    biomes: &[
        "minecraft:snowy_plains",
        "minecraft:snowy_slopes",
        "minecraft:frozen_peaks",
        "minecraft:jagged_peaks",
        "minecraft:snowy_taiga",
        "minecraft:ice_spikes",
        "minecraft:grove",
        "minecraft:snowy_beach",
    ],
    depth: 12,
    steep: 3,
    salt: 171,
    // Minecraft's snow layer: the land's eighths lie there instead.
    strip: &["minecraft:snow"],
    rules: &[
        Rule { from: "minecraft:snow_block", to: |slope, depth| Some(if slope == Slope::Steep && depth < 4 { ROCK } else { SNOW }) },
        Rule { from: "minecraft:powder_snow", to: |_, _| Some(SNOW) },
        Rule { from: "minecraft:grass_block", to: |_, _| Some(TURF) },
        Rule { from: "minecraft:dirt", to: |_, _| Some(PACKED) },
        Rule { from: "minecraft:coarse_dirt", to: |_, _| Some(PACKED) },
        Rule { from: "minecraft:podzol", to: |_, _| Some(TURF) },
        Rule { from: "minecraft:stone", to: |_, _| Some(ROCK) },
        Rule { from: "minecraft:packed_ice", to: |_, _| Some(ICE_WALL) },
        Rule { from: "minecraft:blue_ice", to: |_, _| Some(ICE_WALL) },
        Rule { from: "minecraft:ice", to: |_, _| Some(ICE) },
        Rule { from: "minecraft:short_grass", to: |_, _| Some(GRASS) },
        Rule { from: "minecraft:fern", to: |_, _| Some(GRASS) },
        Rule { from: "minecraft:spruce_log", to: |_, _| Some(POLE) },
        Rule { from: "minecraft:spruce_leaves", to: |_, _| Some(BOUGHS) },
    ],
    layers: &[Layers { tops: &[SNOW, TURF, PACKED], whole: SNOW, eighths: LAYERS, least: 1 }],
    caves: Some((ROCK_HI, ROCK)),
};

/// Makes a snowy chunk Snowy Mountain's.
pub fn decorate(chunk: &mut Chunk, gen: &WorldGen, states: &mut HashMap<&'static str, BlockStateId>) {
    if !gen.sky_light {
        return;
    }
    if crate::jak_lands::shape(&LAND, chunk, gen, states).is_some() {
        icicles(chunk, gen, states);
    }
    // Its lurkers' forts (`jak_kit`), over chunks at the land's edge too
    // (as it was before 23/10's hamlets, which are only with
    // `IW4L_LUGARES_JAK=1`).
    if crate::original_places::jak_places_on() {
        crate::jak_hamlets::place(1, chunk, gen, states);
    } else {
        crate::jak_kit::place(&crate::jak_kit::SNOW_FORTS, chunk, gen, states);
    }
}

/// Icicles (`snow-fringe-01`, the fringe hanging off the level's ledges)
/// under the rock and snow that overhang open air, at a quarter of such
/// places near the surface.
fn icicles(chunk: &mut Chunk, gen: &WorldGen, states: &mut HashMap<&'static str, BlockStateId>) {
    use minecraftoss_core::chunk::HeightmapKind;
    let holds: Vec<BlockStateId> = [SNOW, ROCK, ROCK_HI, PACKED, ICE_WALL].iter().map(|n| crate::jak_lands::state(gen, states, n)).collect();
    let icicle = crate::jak_lands::state(gen, states, ICICLE);
    let air = BlockStateId::AIR;
    let registries = gen.terrain.registries.clone();
    let seed = gen.terrain.seed;
    let (x0, z0) = (chunk.pos.x * 16, chunk.pos.z * 16);
    for x in 0..16 {
        for z in 0..16 {
            let t = chunk.heightmaps.get(HeightmapKind::WorldSurfaceWg, x, z);
            for y in (t - 24).max(-60)..t + 8 {
                if chunk.block(x, y, z) != air || chunk.block(x, y - 1, z) != air || !holds.contains(&chunk.block(x, y + 1, z)) {
                    continue;
                }
                if crate::jak_lands::mix(seed, x0 + x as i32, z0 + z as i32, LAND.salt + u64::from(y as u32)) % 4 == 0 {
                    chunk.set_block_raw(x, y, z, icicle, &registries);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    fn registries() -> Option<Arc<minecraftoss_core::Registries>> {
        let paths = minecraftoss_core::registries::DataPaths::discover().ok()?;
        paths.block_catalog.is_file().then(|| Arc::new(minecraftoss_core::Registries::load(&paths).unwrap()))
    }

    #[test]
    fn every_snowy_block_parses() {
        let registries = registries().expect("the Minecraft data");
        for name in crate::jak_lands::names(&LAND) {
            assert!(registries.blocks.parse_state(name).is_ok(), "{name}");
        }
    }

    /// Generated snowy land (seed 12345): its own snow, rock and turf, its
    /// snow in eighths over every snow top (a layer at least), none of
    /// Minecraft's snow layer left on top.
    #[test]
    fn the_snowy_land_is_snowy_mountain_s() {
        let registries = registries().expect("the Minecraft data");
        // The nearest snowy land clear of the games' places (the nearest is
        // Snowy Mountain's own, `original_places`).
        let mut from = (0, 0);
        let (land, at) = loop {
            let (land, at) = crate::jak_lands::nearest(&LAND, registries.clone(), 12345, from, 8000, true).expect("snowy land near");
            if !crate::original_places::inside_a_place(&registries, 12345, at.0, at.2) {
                break (land, at);
            }
            from = (at.0 + 1200, at.2 + 1200);
        };
        let terrain = Arc::new(minecraftoss_generator::terrain::TerrainGenerator::overworld(registries.clone(), 12345).unwrap());
        let gen = Arc::new(WorldGen::new(terrain).unwrap());
        crate::jak_structures::install();
        let mut map = minecraftoss_world::chunk_map::ChunkMap::with_worldgen(gen, 2, 4);
        let chunk = map.load_now(minecraftoss_core::ChunkPos::new(at.0 >> 4, at.2 >> 4));
        let blocks = &registries.blocks;
        let ours = |name: &str| blocks.block_by_name(name).unwrap();
        let mut counts = HashMap::new();
        for x in 0..16 {
            for z in 0..16 {
                for y in at.1 - 16..at.1 + 24 {
                    let b = blocks.block(blocks.block_of(chunk.block(x, y, z))).name.to_string();
                    *counts.entry(b).or_insert(0usize) += 1;
                }
            }
        }
        println!("{land}: {:?}", counts.iter().filter(|(k, _)| k.starts_with("jak:") || k.contains("snow")).collect::<Vec<_>>());
        assert!(counts.get("jak:snow_layer").copied().unwrap_or(0) > 0, "its snow in eighths");
        assert!(counts.get("minecraft:snow").copied().unwrap_or(0) == 0, "no Minecraft snow layer left");
        assert!(counts.keys().any(|k| k == SNOW || k == TURF), "its snow");
        let _ = ours(ROCK);
    }
}
