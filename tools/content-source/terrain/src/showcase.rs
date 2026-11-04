//! A reproducible, selectable inspection world. Real cities, source cells
//! and saved ownership, with a short signed route between the exhibits.
use crate::{jak_objects as objects, jak_pieces as pieces, jak_city_arena as tower};
use minecraftoss_core::{
    Chunk, ChunkPos,
    chunk::{ChunkStatus, HeightmapKind},
    registries::Registries,
};
use std::{
    collections::{BTreeMap, HashMap},
    path::Path,
    sync::Arc,
};
type Pos = (i32, i32, i32);
pub const SPAWN: [f64; 3] = [0.5, 64.0, 110.5];
pub const TOWER: Pos = (0, 64, -220);

pub fn is_world(dir:Option<&Path>)->bool{
    dir.and_then(|d|std::fs::read(d.join("pruebas-jak.json")).ok())
        .and_then(|b|serde_json::from_slice::<serde_json::Value>(&b).ok())
        .is_some_and(|m|matches!(m["schema"].as_str(),Some("jak-showcase/v1"|"jak-showcase/v2")))
}

/// The same flat density and biome source for real chunks and far terrain.
/// Prefab chunks alone cannot replace the procedural horizon behind them.
pub fn generator(r:Arc<Registries>,seed:i64)->Result<minecraftoss_generator::terrain::TerrainGenerator,String>{
    use minecraftoss_generator::{biome_source::{ParameterList,ParameterPoint,Parameter},terrain::TerrainGenerator};
    use serde_json::json;
    let biomes=ParameterList::new(vec![(ParameterPoint{parameters:[Parameter::point(0.0);6],offset:0},"minecraft:plains")]);
    let mut generator = TerrainGenerator::new_with(r,seed,"minecraft:overworld",biomes,|s|{
        s["sea_level"]=json!(-64);s["default_fluid"]=json!({"Name":"minecraft:air"});s["aquifers"]=serde_json::Value::Null;
        for key in ["temperature","vegetation","continents","erosion","depth","ridges"]{s["noise_router"][key]=json!(0.0);}
        s["noise_router"]["chunk_surface_level"]=json!(64.0);
        s["noise_router"]["final_density"]=json!({"type":"minecraft:min","left":{"type":"minecraft:gradient","axis":"y","from_coordinate":63,"to_coordinate":65,"from_value":1.0,"to_value":-1.0},"right":{"type":"minecraft:gradient","axis":"y","from_coordinate":-1,"to_coordinate":0,"from_value":-1.0,"to_value":1.0}});
        let block=|name:&str|json!({"type":"minecraft:block","result_state":{"Name":name}});
        let at=|y:i32|json!({"type":"minecraft:y_above","anchor":{"absolute":y},"surface_depth_multiplier":0,"add_stone_depth":false});
        s["surface_rule"]=json!({"type":"minecraft:sequence","sequence":[
            {"type":"minecraft:condition","if_true":at(63),"then_run":block("minecraft:grass_block")},
            {"type":"minecraft:condition","if_true":at(60),"then_run":block("minecraft:dirt")},
            {"type":"minecraft:condition","if_true":{"type":"minecraft:not","invert":at(1)},"then_run":block("minecraft:bedrock")},block("minecraft:stone")
        ]});
    })?;
    // The exhibits already exist as saved blocks. Keep their surrounding
    // test ground open instead of planning more cities on every new flat site.
    generator.host_structures = false;
    Ok(generator)
}
struct Flat;
impl crate::jak_structures::Land for Flat {
    fn ground(&self, _: i32, _: i32) -> i32 {
        64
    }
    fn wet(&self, _: i32, _: i32) -> bool {
        false
    }
}

pub fn write(dir: &Path, r: Arc<Registries>, seed: i64) -> anyhow::Result<serde_json::Value> {
    anyhow::ensure!(!dir.exists(), "The showcase output must be a new directory");
    std::fs::create_dir_all(dir)?;
    crate::original_places::bind_world(r.clone(), seed);
    objects::load(Some(dir));
    pieces::load_moved(Some(dir));
    let biome = r.biomes.id("minecraft:plains").expect("plains");
    let desert = r.biomes.id("minecraft:desert").expect("desert");
    let mut chunks = BTreeMap::<(i32, i32), Chunk>::new();
    let mut ids = HashMap::new();
    let mut put = |p: Pos, state: &str| -> anyhow::Result<()> {
        let id = if let Some(&id) = ids.get(state) {
            id
        } else {
            let id = r.blocks.parse_state(state).map_err(anyhow::Error::msg)?;
            ids.insert(state.to_owned(), id);
            id
        };
        let c = chunks
            .entry((p.0 >> 4, p.2 >> 4))
            .or_insert_with(|| Chunk::new(ChunkPos::new(p.0 >> 4, p.2 >> 4), -64, 384, biome));
        c.set_block_raw(
            p.0.rem_euclid(16) as usize,
            p.1,
            p.2.rem_euclid(16) as usize,
            id,
            &r,
        );
        Ok(())
    };
    // A complete flat region, including open air above it. No hidden
    // support boxes: all foundations are ordinary visible world blocks.
    for x in -272..272 {
        for z in -304..192 {
            for y in 0..64 {
                put(
                    (x, y, z),
                    if y == 0 {
                        "minecraft:bedrock"
                    } else if y < 60 {
                        "minecraft:stone"
                    } else if x >= 48 && x < 256 && (-112..112).contains(&z) {
                        if y < 62 { "minecraft:sandstone" } else { "minecraft:sand" }
                    } else if y < 63 {
                        "minecraft:dirt"
                    } else {
                        "minecraft:grass_block[snowy=false]"
                    },
                )?;
            }
        }
    }
    let mut city_stats = Vec::new();
    for (name, center, spargus) in [
        ("Villarrefugio", (-144, 0), false),
        ("Spargus", (144, 0), true),
    ] {
        let blocks = if spargus {
            crate::jak_spargus::town(seed, center, 64, 0, &Flat)
        } else {
            crate::jak_barrio::village(seed, center, 64, 0, &Flat)
        };
        anyhow::ensure!(!blocks.is_empty(), "{name} was not built");
        let count = blocks.len();
        let bar:Vec<_>=blocks.iter().filter(|(_,s)|*s=="jak:bar_rotulo").map(|(p,_)|[p.0,p.1,p.2]).collect();
        for (p, state) in blocks {
            put(p, state)?;
        }
        // Persist the exact generated props and palace pieces at their
        // authored locations; a reload does not depend on a natural site.
        crate::jak_town::relocate_original_props(seed, center, (0, 0, 0));
        city_stats
            .push(serde_json::json!({"name":name,"center":[center.0,64,center.1],"blocks":count,"bar_signs":bar}));
    }
    tower::remember(seed, TOWER);
    let src = tower::source_origin();
    let mut native = 0;
    let mut sources = Vec::new();
    for x in tower::FOOTPRINT_MIN[0]..tower::FOOTPRINT_MAX[0] {
        for z in tower::FOOTPRINT_MIN[1]..tower::FOOTPRINT_MAX[1] {
            if let Some((lo, hi)) = tower::air_column(x, z) {
                for y in TOWER.1 + lo..TOWER.1 + hi {
                    put((TOWER.0 + x, y, TOWER.2 + z), "minecraft:air")?;
                }
            }
            if tower::solid_base_column(x,z) {
                for y in 0..tower::core_height(x,z) {put((TOWER.0+x,TOWER.1+y,TOWER.2+z),"jak:hv_t-citywide-wall-tile-01")?;}
            }
        }
    }
    for sx in (tower::AT..tower::AT + tower::SIZE).step_by(16) {
        for sz in (tower::AT..tower::AT + tower::SIZE).step_by(16) {
            let column = tower::column(sx, sz)
                .ok_or_else(|| anyhow::anyhow!("missing physical tower column {sx},{sz}"))?;
            for (&cell, piece) in &column.pieces {
                if piece.tris.is_empty() {
                    continue;
                }
                let at = tower::to_world(TOWER, cell);
                let slab = objects::slab_type(piece);
                if let Some(state)=crate::jak_architecture::reusable_piece(piece){
                    put(at,state)?;
                }else{
                    put(at,&format!("{}[type={slab},waterlogged=false]", tower::PIECE))?;
                    sources.push((at,Some((tower::SOURCE_ID,cell))));
                }
                native += 1;
            }
        }
    }
    // Extend the real basement piers down to the ground, and open the
    // entrance with the same four gate panels as a naturally built tower.
    for x in tower::FOOTPRINT_MIN[0]..tower::FOOTPRINT_MAX[0] {
        for z in tower::FOOTPRINT_MIN[1]..tower::FOOTPRINT_MAX[1] {
            if let Some(floor) = tower::support_floor(x, z) {
                for y in 40..TOWER.1 + floor {
                    put(
                        (TOWER.0 + x, y, TOWER.2 + z),
                        "jak:hv_t-citywide-wall-tile-01",
                    )?;
                }
            }
        }
    }
    for x in tower::FOOTPRINT_MIN[0]..tower::FOOTPRINT_MAX[0] {for z in tower::FOOTPRINT_MIN[1]..tower::FOOTPRINT_MAX[1] {
        if let Some(floor)=tower::ordinary_floor(x,z) {put((TOWER.0+x,TOWER.1-1,TOWER.2+z),floor)?;}
    }}
    for (p,state) in tower::fixtures(){put((TOWER.0+p.0,TOWER.1+p.1,TOWER.2+p.2),state)?;}
    // The tower route runs east of Spargus and its garages, never through
    // lower streets as a floating concrete barrier. Roads meet outside the residential lots. Each colour is repeated at
    // its entrance and on the hub's route board.
    for x in -144..=144 {
        for z in 108..=112 {
            put((x, 63, z), "minecraft:stone_bricks")?;
        }
    }
    for (x, color) in [
        (-144, "minecraft:blue_concrete"),
        (144, "minecraft:yellow_concrete"),
    ] {
        for z in 100..=110 {
            for dx in -2..=2 {
                put((x + dx, 63, z), color)?;
            }
        }
    }
    for z in -155..=110 {
        for x in 254..=258 {
            put((x, 63, z), "minecraft:purple_concrete")?;
        }
    }
    for x in -5..=258 {
        for z in -157..=-153 {
            put((x, 63, z), "minecraft:purple_concrete")?;
        }
    }
    for x in 0..=258 {
        for z in 108..=112 {
            if x > 100 {
                put((x, 63, z), "minecraft:purple_concrete")?;
            }
        }
    }
    // Hands-on block interactions, close to the spawn.
    for (x, state) in [
        (-12, "minecraft:crafting_table"),
        (
            -10,
            "minecraft:chest[facing=south,type=single,waterlogged=false]",
        ),
        (-8, "minecraft:furnace[facing=south,lit=false]"),
        (-6, "minecraft:barrel[facing=south,open=false]"),
        (-4, "minecraft:anvil[facing=south]"),
        (-2, "minecraft:enchanting_table"),
    ] {
        put((x, 64, 122), state)?;
    }
    for color in 0..6 {
        let x = 12 + color * 4;
        put((x, 63, 122), "minecraft:gold_block")?;
    }
    for i in 0..8 {
        put(
            (-14 + i * 4, 64, 148),
            "minecraft:chest[facing=north,type=single,waterlogged=false]",
        )?;
    }
    put(
        (0, 64, 126),
        "minecraft:lever[face=floor,facing=north,powered=false]",
    )?;
    let pool = (42, 64, 128);
    put((60,64,134),"minecraft:blue_bed[facing=north,part=head,occupied=false]")?;
    put((60,64,135),"minecraft:blue_bed[facing=north,part=foot,occupied=false]")?;
    for x in 38..58 {
        for z in 120..140 {
            for y in 58..64 {
                let wall = x == 38 || x == 57 || z == 120 || z == 139 || y == 58;
                put(
                    (x, y, z),
                    if wall {
                        "minecraft:glass"
                    } else if x < 46 && y < 62 {
                        "minecraft:sandstone"
                    } else {
                        "minecraft:water[level=0]"
                    },
                )?;
            }
        }
    }
    drop(put);
    // Stored sign text is also useful to editors that support Minecraft
    // signs; the runtime route board is provided by the named waypoints.
    let mut manifest = serde_json::json!({"schema":"jak-showcase/v2","seed":seed,"spawn":SPAWN,"cities":city_stats,
        "tower":{"center":[TOWER.0,TOWER.1,TOWER.2],"entrance":[-5.0,64.05,-156.0],"circuit":[-5.0,76.05,-185.0],"native_cells":native,"source_origin":src},
        "controls":{"day_night":[0,64,126]},
        "waypoints":[{"name":"Villarrefugio y base KG","position":[-144.5,64.0,100.5],"color":"azul"},{"name":"Spargus","position":[144.5,64.0,100.5],"color":"amarillo"},{"name":"Torre y skatepark","position":[-5.0,64.05,-156.0],"color":"morado"},{"name":"Interacciones y vehículos","position":[0.5,64.0,122.5]},{"name":"Palanca: día / noche para las luces","position":[0,64,126]},{"name":"Agua profunda y orilla","position":[pool.0,pool.1,pool.2]}]});
    let storage = minecraftoss_world::storage::ChunkStorage::new(
        dir,
        "minecraft:overworld",
        r.clone(),
        -64,
        384,
    );
    for chunk in chunks.values_mut() {
        if (3..16).contains(&chunk.pos.x) && (-7..7).contains(&chunk.pos.z) {
            for section in chunk.sections_mut() {
                section.biomes = minecraftoss_core::palette::PalettedContainer::Single(desert);
            }
        }
        chunk.status = ChunkStatus::Full;
        chunk.prime_heightmaps(&HeightmapKind::FINAL, &r);
        storage.save(chunk).map_err(anyhow::Error::msg)?;
    }
    storage.flush().map_err(anyhow::Error::msg)?;
    pieces::set_moved_batch(sources).map_err(anyhow::Error::msg)?;
    manifest["chunks"] = serde_json::json!(chunks.len());
    std::fs::write(
        dir.join("pruebas-jak.json"),
        serde_json::to_vec_pretty(&manifest)?,
    )?;
    objects::load(None);
    pieces::load_moved(None);
    Ok(manifest)
}

#[cfg(test)]mod tests{
    use super::*;
    #[test]fn flat_chunks_and_far_sampler_share_the_same_ground_and_biome(){
        use minecraftoss_core::chunk::HeightmapKind;
        let paths=minecraftoss_core::registries::DataPaths::discover().unwrap();
        let r=Arc::new(Registries::load(&paths).unwrap());let g=generator(r.clone(),12345).unwrap();
        assert!(!g.host_structures,"the saved exhibits do not need additional procedural cities");
        for(x,z)in [(0,0),(66,-195),(224,176),(-500,700)]{
            assert_eq!(g.base_height(x,z,HeightmapKind::WorldSurfaceWg),64);
            assert_eq!(g.base_height(x,z,HeightmapKind::OceanFloorWg),64);
            assert_eq!(g.far_heights(x,z,1,1,8),vec![64.0]);
        }
        let mut c=Chunk::new(ChunkPos::new(14,11),-64,384,r.biomes.id("minecraft:plains").unwrap());
        struct Plains(minecraftoss_core::BiomeId);
        impl minecraftoss_generator::material::BiomeLookup for Plains{fn noise_biome(&self,_:i32,_:i32,_:i32)->minecraftoss_core::BiomeId{self.0}}
        let plains=r.biomes.id("minecraft:plains").unwrap();
        g.fill_biomes(&mut c);assert_eq!(c.biome(0,16,0),plains);g.build_terrain(&mut c,&Plains(plains),&[plains]);
        for(x,z)in [(0,0),(15,15)]{assert_eq!(c.block(x,63,z),r.blocks.parse_state("minecraft:grass_block[snowy=false]").unwrap());assert_eq!(c.block(x,64,z),r.blocks.parse_state("minecraft:air").unwrap());}
    }
}
