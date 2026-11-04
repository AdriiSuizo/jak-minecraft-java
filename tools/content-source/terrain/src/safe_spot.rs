//! A safe place to stand (Adrii, 16/10: players kept appearing inside the
//! ground, falling for ever or into caves): feet on a column's top surface,
//! a solid block under them that is no liquid nor hazard, and two open
//! blocks over them; the nearest such column, searched outward. Spawns,
//! respawns at beds, a second player joining and every rescue use it, with
//! one player or two.

/// What a block is to a body standing there.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Footing {
    /// Collision to stand on.
    Solid,
    /// Room for a body (air, grass, flowers).
    Open,
    /// Not to stand on nor in: water, lava, dark eco, fire, leaves.
    Hazard,
    /// Its collision is not loaded yet: no answer.
    Unknown,
}

/// Block names (paths) never stood on nor in.
const HAZARDS: [&str; 14] = [
    "water",
    "lava",
    "dark_eco",
    "fire",
    "magma",
    "cactus",
    "powder_snow",
    "sweet_berry",
    "leaves",
    "bubble_column",
    "campfire",
    "wither_rose",
    "cobweb",
    "pointed_dripstone",
];

/// Whether a block's name makes it a hazard to stand on or in.
pub fn is_hazard(path: &str) -> bool {
    HAZARDS.iter().any(|h| path.contains(h))
}

/// Feet at `(x, y, z)`: solid ground under them and room for the body.
pub fn standable(x: i32, y: i32, z: i32, footing: &impl Fn(i32, i32, i32) -> Footing) -> bool {
    footing(x, y - 1, z) == Footing::Solid && footing(x, y, z) == Footing::Open && footing(x, y + 1, z) == Footing::Open
}

/// The columns `ring` steps out from a centre, round its square.
fn ring(ring: i32) -> impl Iterator<Item = (i32, i32)> {
    let r = ring;
    let side: Vec<(i32, i32)> = if r == 0 {
        vec![(0, 0)]
    } else {
        let mut out = Vec::with_capacity((8 * r) as usize);
        for d in -r..r {
            out.push((d, -r));
            out.push((r, d));
            out.push((-d, r));
            out.push((-r, -d));
        }
        out.sort_by_key(|(dx, dz)| dx * dx + dz * dz);
        out
    };
    side.into_iter()
}

/// The nearest safe standing spot to column `(x, z)`, `min_ring` to
/// `max_ring` columns out (`min_ring` 1: beside it, as beside a bed): feet
/// on the column's top (`top(x, z)`, the first free height over its
/// highest motion-blocking block, `None` while its chunk is not in) and
/// `standable` there. `None` while what it needs is not loaded or nowhere
/// near is safe.
pub fn safe_spot(
    x: i32,
    z: i32,
    min_ring: i32,
    max_ring: i32,
    top: &impl Fn(i32, i32) -> Option<i32>,
    footing: &impl Fn(i32, i32, i32) -> Footing,
) -> Option<(i32, i32, i32)> {
    for r in min_ring.max(0)..=max_ring {
        for (dx, dz) in ring(r) {
            let (cx, cz) = (x + dx, z + dz);
            let Some(y) = top(cx, cz) else { continue };
            if standable(cx, y, cz, footing) {
                return Some((cx, y, cz));
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use minecraftoss_core::Registries;
    use minecraftoss_core::block::flags;
    use minecraftoss_core::chunk::HeightmapKind;
    use minecraftoss_core::registries::DataPaths;
    use minecraftoss_core::{ChunkPos, Chunk};
    use minecraftoss_generator::terrain::TerrainGenerator;
    use minecraftoss_world::chunk_map::{ChunkMap, WorldGen};
    use std::collections::HashMap;
    use std::sync::Arc;

    #[test]
    fn rings_go_round_nearest_first() {
        assert_eq!(ring(0).collect::<Vec<_>>(), vec![(0, 0)]);
        let one: Vec<_> = ring(1).collect();
        assert_eq!(one.len(), 8);
        assert!(one[..4].iter().all(|(x, z)| x.abs() + z.abs() == 1), "sides before corners");
    }

    #[test]
    fn a_flat_world_spot_is_on_top_and_beside_a_bed() {
        // Ground at 64 everywhere, a bed at (0, 64, 0) (solid), water east.
        let footing = |x: i32, y: i32, z: i32| {
            if x > 3 && y == 63 {
                Footing::Hazard
            } else if y < 64 || (x, y, z) == (0, 64, 0) {
                Footing::Solid
            } else {
                Footing::Open
            }
        };
        let top = |x: i32, z: i32| Some(if (x, z) == (0, 0) { 65 } else { 64 });
        assert_eq!(safe_spot(0, 0, 0, 4, &top, &footing), Some((0, 65, 0)), "on the bed itself at ring 0");
        let beside = safe_spot(0, 0, 1, 4, &top, &footing).unwrap();
        assert_eq!(beside.1, 64);
        assert_eq!((beside.0 - 0).abs().max(beside.2.abs()), 1, "right beside the bed");
        // Over the water: the nearest dry column.
        let dry = safe_spot(6, 0, 0, 8, &top, &footing).unwrap();
        assert!(dry.0 <= 3, "{dry:?}");
        // Nothing loaded: no answer, wait.
        assert_eq!(safe_spot(0, 0, 0, 4, &|_, _| None, &footing), None);
    }

    /// Hundreds of spawns on real generated land (several seeds; plains,
    /// mountains, oceans, deserts, forests, caves under them), as one or
    /// two players and their respawns: every one stands on the top surface,
    /// on solid dry ground with room over it, never under the height map,
    /// in water or in a cave.
    #[test]
    fn hundreds_of_spawns_stand_on_the_surface() {
        let Ok(paths) = DataPaths::discover() else {
            return;
        };
        let Ok(registries) = Registries::load(&paths) else {
            return;
        };
        let registries = Arc::new(registries);
        let blocks = &registries.blocks;
        let mut spawns = 0;
        let mut unsafe_found = 0;
        for seed in [12345i64, 1234, 77, -9_000_000_001] {
            let worldgen = Arc::new(WorldGen::new(Arc::new(TerrainGenerator::overworld(registries.clone(), seed).unwrap())).unwrap());
            let mut map = ChunkMap::with_worldgen(worldgen, 2, 8);
            let mut chunks: HashMap<(i32, i32), Arc<Chunk>> = HashMap::new();
            // Spread over the world: far apart patches, each 3x3 chunks.
            for (px, pz) in [(0, 0), (40, -12), (-75, 33), (120, 90), (-200, -160)] {
                for cx in px - 1..=px + 1 {
                    for cz in pz - 1..=pz + 1 {
                        chunks.insert((cx, cz), map.load_now(ChunkPos::new(cx, cz)));
                    }
                }
                let state = |x: i32, y: i32, z: i32| {
                    let chunk = chunks.get(&(x >> 4, z >> 4))?;
                    (y >= chunk.min_y() && y < chunk.min_y() + chunk.height()).then(|| chunk.block((x & 15) as usize, y, (z & 15) as usize))
                };
                let footing = |x: i32, y: i32, z: i32| {
                    let Some(state) = state(x, y, z) else { return Footing::Unknown };
                    let name = blocks.block(blocks.block_of(state)).name.path().to_owned();
                    if blocks.has_fluid(state) || is_hazard(&name) {
                        Footing::Hazard
                    } else if blocks.is_air(state) || !blocks.is(state, flags::LEGACY_SOLID) {
                        Footing::Open
                    } else {
                        Footing::Solid
                    }
                };
                let top = |x: i32, z: i32| {
                    let chunk = chunks.get(&(x >> 4, z >> 4))?;
                    Some(chunk.heightmaps.get(HeightmapKind::MotionBlockingNoLeaves, (x & 15) as usize, (z & 15) as usize))
                };
                // Twenty spawns a patch, two players each (and a respawn).
                for k in 0..20 {
                    let (x, z) = (px * 16 + (k * 7) % 16 - 4, pz * 16 + (k * 11) % 16 - 4);
                    for player in 0..2 {
                        let Some((sx, sy, sz)) = safe_spot(x + player, z, player, 12, &top, &footing) else {
                            // All water round here (ocean): none, the caller keeps looking.
                            continue;
                        };
                        spawns += 1;
                        let surface = top(sx, sz).unwrap();
                        if !(sy >= surface && standable(sx, sy, sz, &footing)) || footing(sx, sy - 1, sz) != Footing::Solid {
                            unsafe_found += 1;
                        }
                        // Never under the height map: no cave.
                        assert!(sy >= surface, "seed {seed}: ({sx}, {sy}, {sz}) under the surface {surface}");
                    }
                }
            }
        }
        assert_eq!(unsafe_found, 0);
        assert!(spawns >= 300, "{spawns} spawns found");
    }
}
