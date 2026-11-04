//! Where the end game's structures are for a seed: the strongholds an Eye
//! of Ender flies to (`ConcentricRingsStructurePlacement`) and the End's
//! obsidian pillars the dragon's crystals stand on (`EndSpikeFeature`).


/// One End pillar: its centre, the height of the bedrock under its crystal,
/// and whether iron bars cage the crystal.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct EndSpike {
    pub x: i32,
    pub z: i32,
    pub top: i32,
    pub guarded: bool,
}

/// `EndSpikeFeature.getSpikesForLevel`: the ten pillars.
pub fn end_spikes(seed: i64) -> Vec<EndSpike> {
    minecraftoss_generator::feature::kinds::the_end::spikes_for_seed(seed)
        .into_iter()
        .map(|s| EndSpike {
            x: s.center_x,
            z: s.center_z,
            top: s.height,
            guarded: s.guarded,
        })
        .collect()
}

/// The first `count` strongholds of the `minecraft:strongholds` structure
/// set, each as the block `StructurePlacement.getLocatePos` gives (x, z).
/// Resolving each ring position searches the biomes around it, so this is
/// slow: call it off the main thread.
pub fn strongholds(world_gen: &WorldGen, count: usize) -> Vec<(i32, i32)> {
    let structures = &world_gen.structures;
    let Some(set) = structures
        .sets()
        .iter()
        .position(|set| set.name.trim_start_matches("minecraft:") == "strongholds")
    else {
        return Vec::new();
    };
    let placement = structures.placement(set);
    placement
        .ring_chunks(&world_gen.terrain, count)
        .into_iter()
        .map(|chunk| {
            let pos = placement.locate_pos(chunk);
            (pos.x, pos.z)
        })
        .collect()
}

/// The world generation a stream runs, for callers without the world crate.
pub type WorldGen = minecraftoss_world::chunk_map::WorldGen;
