//! Jak 1's Boggy Swamp (`swamp`) as the block world's swamps (Adrii 23/10:
//! two Jak biomes more, as the Wasteland): Minecraft's swamps and mangrove
//! swamps, their ground the swamp's in its own textures (the
//! `swamp-vis-tfrag` and `swamp-vis-shrub` of the game's export).
//!
//! - Its leafy ground (`swp-leafyground`, its sides `swp-mud-trans`, under
//!   `swp-mud-floor`) where the grass was, its mud (`swp-mud-floor`) for the
//!   earth, its wet mud (`swp-mud-top`) for Minecraft's mud, its deep mud
//!   (`swp-mud-bot`) for the clay, its standing rock (`swp-stand-rock`) for
//!   the stone near the surface;
//! - its trees' bark (`swp-bark`, ends `swp-bark-top`) for the swamp oaks'
//!   and mangroves' logs, its trees' leaf clumps (`swp-tree-growth`) for
//!   their leaves, its roots (`swp-bark-roots`) for the mangroves'
//!   roots, its hanging moss (`swp-moss-hang`) for the vines, its lily pads
//!   (`swp-lilly`) and grass clumps (`swp-grassclump-02`);
//! - its ground in eighths over the leafy and muddy tops, smooth up to the
//!   true surface;
//! - its caves' walls its stone walls (`swp-stonewall`), their floors mud.
//!
//! Its fog and storms (`jak_sky::Climate::Swamp`) and its creatures (swamp
//! rats, bone lurkers: `Habitat::Swamp`) were the biomes' already.
use std::collections::HashMap;

use minecraftoss_core::block::BlockStateId;
use minecraftoss_core::chunk::Chunk;
use minecraftoss_world::chunk_map::WorldGen;

use crate::jak_lands::{Land, Layers, Rule};

pub const TURF: &str = "jak:swamp_turf";
pub const MUD: &str = "jak:swamp_mud";
pub const WET_MUD: &str = "jak:swamp_mud_top";
pub const DEEP_MUD: &str = "jak:swamp_mud_bot";
pub const STONE: &str = "jak:swamp_stone";
pub const STONEWALL: &str = "jak:swamp_stonewall";
pub const BARK: &str = "jak:swamp_bark";
pub const ROOTS: &str = "jak:swamp_roots";
pub const MOSS: &str = "jak:swamp_moss";
pub const LILY: &str = "jak:swamp_lily";
pub const GRASS: &str = "jak:swamp_grass";
pub const LEAVES: &str = "jak:swamp_leaves";
pub const FERN: &str = "jak:swamp_fern";
pub const SCUM: &str = "jak:swamp_scum";
pub const MUD_LAYERS: [&str; 7] = [
    "jak:swamp_mud_layer[layers=1]",
    "jak:swamp_mud_layer[layers=2]",
    "jak:swamp_mud_layer[layers=3]",
    "jak:swamp_mud_layer[layers=4]",
    "jak:swamp_mud_layer[layers=5]",
    "jak:swamp_mud_layer[layers=6]",
    "jak:swamp_mud_layer[layers=7]",
];
pub const LEAFY_LAYERS: [&str; 7] = [
    "jak:swamp_leafy_layer[layers=1]",
    "jak:swamp_leafy_layer[layers=2]",
    "jak:swamp_leafy_layer[layers=3]",
    "jak:swamp_leafy_layer[layers=4]",
    "jak:swamp_leafy_layer[layers=5]",
    "jak:swamp_leafy_layer[layers=6]",
    "jak:swamp_leafy_layer[layers=7]",
];

pub const LAND: Land = Land {
    name: "Boggy Swamp",
    biomes: &["minecraft:swamp", "minecraft:mangrove_swamp"],
    depth: 12,
    steep: 3,
    salt: 271,
    strip: &[],
    rules: &[
        Rule { from: "minecraft:grass_block", to: |_, _| Some(TURF) },
        Rule { from: "minecraft:dirt", to: |_, _| Some(MUD) },
        Rule { from: "minecraft:mud", to: |_, _| Some(WET_MUD) },
        Rule { from: "minecraft:clay", to: |_, _| Some(DEEP_MUD) },
        Rule { from: "minecraft:stone", to: |_, _| Some(STONE) },
        Rule { from: "minecraft:oak_log", to: |_, _| Some(BARK) },
        Rule { from: "minecraft:mangrove_log", to: |_, _| Some(BARK) },
        Rule { from: "minecraft:mangrove_roots", to: |_, _| Some(ROOTS) },
        Rule { from: "minecraft:muddy_mangrove_roots", to: |_, _| Some(ROOTS) },
        Rule { from: "minecraft:oak_leaves", to: |_, _| Some(LEAVES) },
        Rule { from: "minecraft:mangrove_leaves", to: |_, _| Some(LEAVES) },
        Rule { from: "minecraft:vine", to: |_, _| Some(MOSS) },
        Rule { from: "minecraft:lily_pad", to: |_, _| Some(LILY) },
        Rule { from: "minecraft:short_grass", to: |_, _| Some(GRASS) },
        Rule { from: "minecraft:fern", to: |_, _| Some(FERN) },
    ],
    layers: &[
        Layers { tops: &[TURF], whole: TURF, eighths: LEAFY_LAYERS, least: 0 },
        Layers { tops: &[WET_MUD, MUD], whole: WET_MUD, eighths: MUD_LAYERS, least: 0 },
    ],
    caves: Some((STONEWALL, MUD)),
};

/// Makes a swampy chunk Boggy Swamp's.
pub fn decorate(chunk: &mut Chunk, gen: &WorldGen, states: &mut HashMap<&'static str, BlockStateId>) {
    if !gen.sky_light {
        return;
    }
    if crate::jak_lands::shape(&LAND, chunk, gen, states).is_some() {
        scum(chunk, gen, states);
    }
    // Boggy Billy's huts (`jak_kit`), over chunks at the land's edge too
    // (the hamlets only with `IW4L_LUGARES_JAK=1`).
    if crate::original_places::jak_places_on() {
        crate::jak_hamlets::place(0, chunk, gen, states);
    } else {
        crate::jak_kit::place(&crate::jak_kit::SWAMP_HUTS, chunk, gen, states);
    }
}

/// The scum floating on the swamp's still water (`swp-scum`), on one
/// open water column in eight.
fn scum(chunk: &mut Chunk, gen: &WorldGen, states: &mut HashMap<&'static str, BlockStateId>) {
    use minecraftoss_core::chunk::HeightmapKind;
    let registries = gen.terrain.registries.clone();
    let Some(water) = registries.blocks.block_by_name("minecraft:water") else { return };
    let scum = crate::jak_lands::state(gen, states, SCUM);
    let seed = gen.terrain.seed;
    let (x0, z0) = (chunk.pos.x * 16, chunk.pos.z * 16);
    for x in 0..16 {
        for z in 0..16 {
            let t = chunk.heightmaps.get(HeightmapKind::WorldSurfaceWg, x, z);
            for y in (t - 3..t + 3).rev() {
                let here = chunk.block(x, y, z);
                if here == BlockStateId::AIR {
                    continue;
                }
                if registries.blocks.block_of(here) == water
                    && chunk.block(x, y + 1, z) == BlockStateId::AIR
                    && crate::jak_lands::mix(seed, x0 + x as i32, z0 + z as i32, LAND.salt + 7) % 8 == 0
                {
                    chunk.set_block_raw(x, y + 1, z, scum, &registries);
                }
                break;
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

    /// A log lying east-west stays lying as the swamp's bark, a vine's
    /// faces stay its moss's, leaves keep their distance.
    #[test]
    fn what_a_block_was_it_keeps() {
        let registries = registries().expect("the Minecraft data");
        let blocks = &registries.blocks;
        let log = blocks.parse_state("minecraft:oak_log[axis=x]").unwrap();
        let bark = crate::jak_lands::carried(blocks, log, blocks.parse_state(BARK).unwrap());
        assert_eq!(blocks.property(bark, "axis"), Some("x"));
        let vine = blocks.parse_state("minecraft:vine[east=true,up=true]").unwrap();
        let moss = crate::jak_lands::carried(blocks, vine, blocks.parse_state(MOSS).unwrap());
        assert_eq!((blocks.property(moss, "east"), blocks.property(moss, "up"), blocks.property(moss, "north")), (Some("true"), Some("true"), Some("false")));
        let leaves = blocks.parse_state("minecraft:spruce_leaves[distance=2,persistent=false]").unwrap();
        let boughs = crate::jak_lands::carried(blocks, leaves, blocks.parse_state(crate::snowy::BOUGHS).unwrap());
        assert_eq!(blocks.property(boughs, "distance"), Some("2"));
    }

    #[test]
    fn every_swamp_block_parses() {
        let registries = registries().expect("the Minecraft data");
        for name in crate::jak_lands::names(&LAND) {
            assert!(registries.blocks.parse_state(name).is_ok(), "{name}");
        }
    }

    /// Generated swamp (seed 12345): its own turf, mud and bark; no grass
    /// block or oak log of Minecraft's left; a log's lying axis kept.
    #[test]
    fn the_swamp_is_boggy_swamp_s() {
        let registries = registries().expect("the Minecraft data");
        let (land, at) = crate::jak_lands::nearest(&LAND, registries.clone(), 12345, (0, 0), 8000, false).expect("swamp near");
        let terrain = Arc::new(minecraftoss_generator::terrain::TerrainGenerator::overworld(registries.clone(), 12345).unwrap());
        let gen = Arc::new(WorldGen::new(terrain).unwrap());
        crate::jak_structures::install();
        let mut map = minecraftoss_world::chunk_map::ChunkMap::with_worldgen(gen, 2, 4);
        let chunk = map.load_now(minecraftoss_core::ChunkPos::new(at.0 >> 4, at.2 >> 4));
        let blocks = &registries.blocks;
        let mut counts = HashMap::new();
        for x in 0..16 {
            for z in 0..16 {
                for y in at.1 - 12..at.1 + 20 {
                    let b = blocks.block(blocks.block_of(chunk.block(x, y, z))).name.to_string();
                    *counts.entry(b).or_insert(0usize) += 1;
                }
            }
        }
        println!("{land}: {:?}", counts.iter().filter(|(k, _)| k.starts_with("jak:") || k.contains("grass") || k.contains("log")).collect::<Vec<_>>());
        assert!(counts.keys().any(|k| k == TURF || k == WET_MUD || k == MUD), "its ground");
        assert!(counts.get("minecraft:grass_block").copied().unwrap_or(0) == 0, "no grass block left under it");
    }
}
