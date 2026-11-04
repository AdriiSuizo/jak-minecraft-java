//! The BIOMES and TERRAIN chunk steps for noise-based dimensions
//! (vanilla 26.3 `NoiseBasedChunkGenerator.createBiomes` / `buildTerrain`).

use crate::aquifer::{Aquifer, AquiferSamplers, FluidBlocks, GlobalFluidPicker, NoiseAquifer};
use crate::biome_source::{self, Parameter, ParameterList};
use crate::carver::{Carver, CarvingMask};
use crate::density::{Compiler, Context, Df, Id, Program, Registry};
use crate::material::{BiomeLookup, ChunkRules, Column, Loader, MaterialSystem, Rule};
use crate::noise::{NoiseStack, Volume};
use crate::providers::GenContext;
use crate::zoom;
use minecraftoss_core::chunk::{ChunkStatus, HeightmapKind, biome_index};
use minecraftoss_core::random::{AnyPositional, LegacyRandom};
use minecraftoss_core::{BiomeId, BlockStateId, Chunk, ChunkPos, Identifier, Registries};
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;

struct Router {
    climate: [Id; 6],
    final_density: Id,
}

/// Everything needed to generate one noise-based dimension for one seed.
pub struct TerrainGenerator {
    pub registries: Arc<Registries>,
    pub seed: i64,
    pub min_y: i32,
    pub height: i32,
    pub sea_level: i32,
    /// Chunk bounds from the dimension type (the Nether and End build
    /// above their 128-block noise).
    pub chunk_min_y: i32,
    pub chunk_height: i32,
    /// `NoiseGeneratorSettings.disableMobGeneration` (the SPAWN step).
    pub disable_mob_generation: bool,
    /// Generates nothing: every chunk stays air, without features,
    /// structures or mobs (a world only for what is built in it).
    pub void: bool,
    /// Whether the host may add its procedural structures after FEATURES.
    /// Inspection worlds can supply their own complete, saved exhibits.
    pub host_structures: bool,
    /// The universe map's Nether planet: its crust and shafts are laid on
    /// each chunk's terrain.
    pub nether_planet: Option<crate::nether_planet::NetherPlanet>,
    default_block: BlockStateId,
    program: Program,
    router: Router,
    aquifer: Option<AquiferSamplers>,
    random: AnyPositional,
    material_rule: Rule,
    material: MaterialSystem,
    biomes: ParameterList,
    biome_ids: Vec<BiomeId>,
    /// `TheEndBiomeSource`: the end, highlands, midlands, small islands and
    /// barrens, chosen from the erosion (island height) sampler.
    end_biomes: Option<[BiomeId; 5]>,
    carvers: Vec<Carver>,
    biome_carvers: Vec<Vec<usize>>,
    fluids: FluidBlocks,
    picker: GlobalFluidPicker,
    zoom_seed: i64,
    /// `NoiseGeneratorSettings.spawnTarget`: climate ranges per sampler.
    spawn_target: Vec<Vec<(Id, Parameter)>>,
    uncarvable: minecraftoss_core::tags::TagId,
    grass: BlockStateId,
    mycelium: BlockStateId,
    dirt: BlockStateId,
    /// Carver biomes by source chunk: every chunk's carvers ask for the
    /// 17x17 chunks around it, so each answer is wanted 289 times.
    carver_biomes: CarverBiomes,
}

/// A shared memo of `carver_biome`, sharded to keep workers apart.
struct CarverBiomes {
    shards: [std::sync::Mutex<std::collections::HashMap<(i32, i32), BiomeId>>; 64],
}

impl Default for CarverBiomes {
    fn default() -> Self {
        Self { shards: std::array::from_fn(|_| Default::default()) }
    }
}

impl CarverBiomes {
    /// Entries per shard before it starts over, to bound memory.
    const SHARD_LIMIT: usize = 1 << 14;

    fn get_or(&self, pos: ChunkPos, compute: impl FnOnce() -> BiomeId) -> BiomeId {
        let shard = &self.shards[((pos.x & 7) | (pos.z & 7) << 3) as usize];
        if let Some(&biome) = shard.lock().expect("carver biome memo").get(&(pos.x, pos.z)) {
            return biome;
        }
        let biome = compute();
        let mut map = shard.lock().expect("carver biome memo");
        if map.len() >= Self::SHARD_LIMIT {
            map.clear();
        }
        map.insert((pos.x, pos.z), biome);
        biome
    }
}

fn block(registries: &Registries, json: &Value) -> Result<BlockStateId, String> {
    match json {
        Value::String(s) => registries.blocks.parse_state(s),
        _ => {
            let mut s = registries.blocks.parse_state(json["Name"].as_str().ok_or("block state lacks Name")?)?;
            if let Some(props) = json.get("Properties").and_then(Value::as_object) {
                for (k, v) in props {
                    s = registries.blocks.with_property(s, k, v.as_str().unwrap_or_default()).ok_or_else(|| format!("bad property {k}"))?;
                }
            }
            Ok(s)
        }
    }
}

impl TerrainGenerator {
    /// Loads `worldgen/noise_settings/<settings>` for the Overworld preset.
    pub fn overworld(registries: Arc<Registries>, seed: i64) -> Result<Self, String> {
        Self::new(registries, seed, "minecraft:overworld", ParameterList::overworld())
    }

    pub fn new(registries: Arc<Registries>, seed: i64, settings_name: &str, biomes: ParameterList) -> Result<Self, String> {
        Self::new_with(registries, seed, settings_name, biomes, |_| {})
    }

    /// [`Self::new`] with the noise settings changed by `patch` first.
    pub fn new_with(
        registries: Arc<Registries>,
        seed: i64,
        settings_name: &str,
        biomes: ParameterList,
        patch: impl FnOnce(&mut Value),
    ) -> Result<Self, String> {
        let pack = registries.datapack.clone();
        let mut settings = pack.read_json("worldgen/noise_settings", &Identifier::parse(settings_name)?)?;
        patch(&mut settings);
        let min_y = settings["noise"]["min_y"].as_i64().ok_or("noise settings lack min_y")? as i32;
        let height = settings["noise"]["height"].as_i64().ok_or("noise settings lack height")? as i32;
        let sea_level = settings["sea_level"].as_i64().ok_or("noise settings lack sea_level")? as i32;
        let legacy = settings["legacy_random_source"].as_bool().unwrap_or(false);
        let default_block = block(&registries, &settings["default_block"])?;
        let default_fluid = block(&registries, &settings["default_fluid"])?;
        let mut compiler = Compiler::new(Registry::new(pack.clone()), seed, legacy);
        let random = compiler.random().clone();
        let router_json = &settings["noise_router"];
        let compile = |c: &mut Compiler, json: &Value| -> Result<Id, String> {
            let f: Arc<Df> = c.registry.parse(json)?;
            c.sampler(&f)
        };
        let names = ["temperature", "vegetation", "continents", "erosion", "depth", "ridges"];
        let mut climate = [0; 6];
        for (slot, name) in climate.iter_mut().zip(names) {
            *slot = compile(&mut compiler, &router_json[name])?;
        }
        let final_density = compile(&mut compiler, &router_json["final_density"])?;
        let mut spawn_target = Vec::new();
        for point in settings["spawn_target"].as_array().ok_or("noise settings lack spawn_target")? {
            let mut parameters = Vec::new();
            for (function, range) in point.as_object().ok_or("spawn target point is not a map")? {
                let id = compile(&mut compiler, &Value::String(function.clone()))?;
                let parameter = match range {
                    Value::Array(pair) => Parameter::span(
                        pair[0].as_f64().ok_or("bad spawn target range")? as f32,
                        pair[1].as_f64().ok_or("bad spawn target range")? as f32,
                    ),
                    other => Parameter::point(other.as_f64().ok_or("bad spawn target value")? as f32),
                };
                parameters.push((id, parameter));
            }
            spawn_target.push(parameters);
        }
        let chunk_surface_level = compile(&mut compiler, &router_json["chunk_surface_level"])?;
        let aquifer = match settings.get("aquifers") {
            Some(a) if a.is_object() => Some(AquiferSamplers {
                barrier: compile(&mut compiler, &a["barrier"])?,
                floodedness: compile(&mut compiler, &a["fluid_level_floodedness"])?,
                spread: compile(&mut compiler, &a["fluid_level_spread"])?,
                lava: compile(&mut compiler, &a["lava"])?,
                exclusion: compile(&mut compiler, &a["exclusion"])?,
                surface_level: compile(&mut compiler, &a["surface_level"])?,
            }),
            _ => None,
        };
        let mut noise_index = HashMap::new();
        let mut noises: Vec<NoiseStack> = Vec::new();
        let (material_rule, mut material) = {
            let mut loader = Loader { registries: &registries, compiler: &mut compiler, noise_index: &mut noise_index, noises: &mut noises };
            let system = MaterialSystem::new(&mut loader, default_block, sea_level, chunk_surface_level)?;
            let rule = loader.rule(&settings["material_rule"])?;
            (rule, system)
        };
        material.noises = noises;
        let program = compiler.into_program();
        // Biome parameter list names resolve to registry IDs once.
        let biome_ids = biomes.entries.iter().map(|(_, name)| registries.biomes.id(name).ok_or_else(|| format!("unknown biome {name}"))).collect::<Result<Vec<_>, _>>()?;
        // Carvers per biome, in the biome's declared order.
        let mut carvers = Vec::new();
        let mut carver_index: HashMap<String, usize> = HashMap::new();
        let mut biome_carvers = Vec::new();
        for (_, info) in registries.biomes.iter() {
            let json = pack.read_json("worldgen/biome", &info.name)?;
            let mut list = Vec::new();
            let entries: Vec<String> = match &json["carvers"] {
                Value::String(s) => vec![s.clone()],
                Value::Array(a) => a.iter().filter_map(|v| v.as_str().map(str::to_owned)).collect(),
                Value::Null => Vec::new(),
                other => return Err(format!("biome {} has unsupported carvers {other}", info.name)),
            };
            for name in entries {
                if name.starts_with('#') {
                    return Err(format!("carver tags are not supported yet ({name})"));
                }
                let index = match carver_index.get(&name) {
                    Some(&i) => i,
                    None => {
                        carvers.push(Carver::parse(&pack.read_json("worldgen/carver", &Identifier::parse(&name)?)?)?);
                        carver_index.insert(name.clone(), carvers.len() - 1);
                        carvers.len() - 1
                    }
                };
                list.push(index);
            }
            biome_carvers.push(list);
        }
        let blocks = &registries.blocks;
        let fluids = FluidBlocks { air: registries.plain_air, water: blocks.parse_state("minecraft:water")?, lava: blocks.parse_state("minecraft:lava")? };
        Ok(Self {
            seed,
            min_y,
            height,
            sea_level,
            chunk_min_y: min_y,
            chunk_height: height,
            disable_mob_generation: settings["disable_mob_generation"].as_bool().unwrap_or(false),
            void: false,
            host_structures: true,
            nether_planet: None,
            default_block,
            program,
            router: Router { climate, final_density },
            aquifer,
            random,
            material_rule,
            material,
            biomes,
            biome_ids,
            end_biomes: None,
            carvers,
            biome_carvers,
            fluids,
            picker: GlobalFluidPicker { sea_level, default_fluid, lava: fluids.lava },
            zoom_seed: zoom::zoom_seed(seed),
            spawn_target,
            uncarvable: registries.block_tags.require("minecraft:uncarvable")?,
            grass: blocks.parse_state("minecraft:grass_block")?,
            mycelium: blocks.parse_state("minecraft:mycelium")?,
            carver_biomes: CarverBiomes::default(),
            dirt: blocks.parse_state("minecraft:dirt")?,
            registries,
        })
    }

    /// The Nether: `minecraft:nether` noise in a `minecraft:the_nether` dimension.
    pub fn nether(registries: Arc<Registries>, seed: i64) -> Result<Self, String> {
        let mut generator = Self::new(registries, seed, "minecraft:nether", ParameterList::nether())?;
        generator.use_dimension_type("minecraft:the_nether")?;
        Ok(generator)
    }

    /// The universe map's Nether planet ([`crate::nether_planet`]): the
    /// Nether with a crust over its roof and shafts down into it.
    pub fn nether_planet(registries: Arc<Registries>, seed: i64) -> Result<Self, String> {
        let mut generator = Self::nether(registries.clone(), seed)?;
        generator.nether_planet = Some(crate::nether_planet::NetherPlanet::new(&registries)?);
        Ok(generator)
    }

    /// The End: `minecraft:end` noise with `TheEndBiomeSource` in a
    /// `minecraft:the_end` dimension.
    pub fn end(registries: Arc<Registries>, seed: i64) -> Result<Self, String> {
        let mut generator = Self::new(registries.clone(), seed, "minecraft:end", ParameterList::nether())?;
        let id = |name: &str| registries.biomes.id(name).ok_or_else(|| format!("unknown biome {name}"));
        generator.end_biomes = Some([
            id("minecraft:the_end")?,
            id("minecraft:end_highlands")?,
            id("minecraft:end_midlands")?,
            id("minecraft:small_end_islands")?,
            id("minecraft:end_barrens")?,
        ]);
        generator.use_dimension_type("minecraft:the_end")?;
        Ok(generator)
    }

    /// `TheEndBiomeSource.getNoiseBiome`.
    fn end_biome(&self, biomes: &[BiomeId; 5], ctx: &mut Context, qx: i32, qy: i32, qz: i32) -> BiomeId {
        let (x, y, z) = (qx << 2, qy << 2, qz << 2);
        let (cx, cz) = (i64::from(x >> 4), i64::from(z >> 4));
        if cx * cx + cz * cz <= 4096 {
            return biomes[0];
        }
        let wx = ((x >> 4) * 2 + 1) * 8;
        let wz = ((z >> 4) * 2 + 1) * 8;
        let height = f64::from(ctx.value(self.router.climate[3], wx, y, wz));
        if height > 0.25 {
            biomes[1]
        } else if height >= -0.0625 {
            biomes[2]
        } else if height < -0.21875 {
            biomes[3]
        } else {
            biomes[4]
        }
    }

    /// Takes chunk bounds from a `dimension_type` entry.
    pub fn use_dimension_type(&mut self, name: &str) -> Result<(), String> {
        let json = self.registries.datapack.read_json("dimension_type", &Identifier::parse(name)?)?;
        self.chunk_min_y = json["min_y"].as_i64().ok_or("dimension type lacks min_y")? as i32;
        self.chunk_height = json["height"].as_i64().ok_or("dimension type lacks height")? as i32;
        Ok(())
    }

    /// `BiomeSource.possibleBiomes()`: distinct biomes in parameter-list order.
    pub fn possible_biomes(&self) -> Vec<BiomeId> {
        if let Some(end) = self.end_biomes {
            return end.to_vec();
        }
        let mut out: Vec<BiomeId> = Vec::new();
        for &id in &self.biome_ids {
            if !out.contains(&id) {
                out.push(id);
            }
        }
        out
    }

    /// `WorldGenerationContext` for features and placements.
    pub fn generation_context(&self) -> crate::providers::GenContext {
        crate::providers::GenContext { min_y: self.min_y, depth: self.height, sea_level: self.sea_level }
    }

    /// `WorldGenRegion.getRandom()`: `worldgen_region_random`, forked and
    /// positioned at the chunk's world position.
    pub fn region_random(&self, center: ChunkPos) -> minecraftoss_core::random::AnyRandom {
        self.random.from_hash_of("minecraft:worldgen_region_random").fork_positional().at(center.min_block_x(), 0, center.min_block_z())
    }

    /// `NoiseBasedChunkGenerator.getOrigin` with `NoiseSpawnFinder`: the
    /// chunk whose climate best fits the spawn targets, near the origin.
    pub fn spawn_origin(&self) -> ChunkPos {
        if self.spawn_target.is_empty() {
            return ChunkPos::new(0, 0);
        }
        let mut ctx = Context::cached(&self.program);
        let mut best = self.spawn_fitness(&mut ctx, 0, 0);
        for (max_radius, increment) in [(2048.0f32, 512.0f32), (512.0, 32.0)] {
            let (origin_x, origin_z, _) = best;
            let mut angle = 0.0f32;
            let mut radius = increment;
            while radius <= max_radius {
                let x = origin_x + (f64::from(angle).sin() * f64::from(radius)) as i32;
                let z = origin_z + (f64::from(angle).cos() * f64::from(radius)) as i32;
                let candidate = self.spawn_fitness(&mut ctx, x, z);
                if candidate.2 < best.2 {
                    best = candidate;
                }
                angle += increment / radius;
                if f64::from(angle) > std::f64::consts::PI * 2.0 {
                    angle = 0.0;
                    radius += increment;
                }
            }
        }
        ChunkPos::new(best.0 >> 4, best.1 >> 4)
    }

    /// `NoiseSpawnFinder.getSpawnPositionAndFitness`.
    fn spawn_fitness(&self, ctx: &mut Context, x: i32, z: i32) -> (i32, i32, i64) {
        let (qx, qz) = ((x >> 2) << 2, (z >> 2) << 2);
        let mut min_fitness = i64::MAX;
        for point in &self.spawn_target {
            let mut fitness = 0i64;
            for &(id, parameter) in point {
                let distance = parameter.distance(biome_source::quantize(ctx.value(id, qx, 0, qz)));
                fitness = fitness.wrapping_add(distance.wrapping_mul(distance));
            }
            min_fitness = min_fitness.min(fitness);
        }
        let bias = i64::from(x) * i64::from(x) + i64::from(z) * i64::from(z);
        (x, z, min_fitness.wrapping_mul(2048 * 2048).wrapping_add(bias))
    }

    pub fn new_chunk(&self, pos: ChunkPos) -> Chunk {
        Chunk::new(pos, self.chunk_min_y, self.chunk_height, BiomeId(0))
    }

    fn gen_context(&self) -> GenContext {
        GenContext { min_y: self.min_y, depth: self.height, sea_level: self.sea_level }
    }

    fn biome_at(&self, climate: [f32; 6]) -> BiomeId {
        let t = biome_source::target(climate[0], climate[1], climate[2], climate[3], climate[4], climate[5]);
        let index = self.biomes.find_index(&t);
        self.biome_ids[index]
    }

    /// The BIOMES step (`ChunkGenerator.doCreateBiomes`).
    pub fn fill_biomes(&self, chunk: &mut Chunk) {
        if let Some(end) = self.end_biomes {
            let mut ctx = Context::uncached(&self.program);
            let (min_qx, min_qz) = (chunk.pos.min_block_x() >> 2, chunk.pos.min_block_z() >> 2);
            let min_section = chunk.min_section_y();
            for (i, section) in chunk.sections_mut().iter_mut().enumerate() {
                let quart_min_y = (min_section + i as i32) * 4;
                for x in 0..4 {
                    for y in 0..4 {
                        for z in 0..4 {
                            let biome = self.end_biome(&end, &mut ctx, min_qx + x, quart_min_y + y, min_qz + z);
                            section.biomes.set(biome_index(x as usize, y as usize, z as usize), biome);
                        }
                    }
                }
            }
            chunk.status = ChunkStatus::Biomes;
            return;
        }
        let mut ctx = Context::cached(&self.program);
        let (min_qx, min_qy, min_qz) = (chunk.pos.min_block_x() >> 2, chunk.min_y() >> 2, chunk.pos.min_block_z() >> 2);
        let volume = Volume::new([4, chunk.height() >> 2, 4], [min_qx << 2, min_qy << 2, min_qz << 2], [4, 4, 4]);
        let buffers: Vec<Vec<f32>> = self.router.climate.iter().map(|&id| ctx.sample(id, &volume)).collect();
        let min_section = chunk.min_section_y();
        for (i, section) in chunk.sections_mut().iter_mut().enumerate() {
            let quart_min_y = (min_section + i as i32) * 4;
            for x in 0..4 {
                for y in 0..4 {
                    for z in 0..4 {
                        let index = volume.index(x, quart_min_y + y - min_qy, z);
                        let biome = self.biome_at(std::array::from_fn(|c| buffers[c][index]));
                        section.biomes.set(biome_index(x as usize, y as usize, z as usize), biome);
                    }
                }
            }
        }
        chunk.status = ChunkStatus::Biomes;
    }

    /// `createUncachedResolver`: per-point climate on an uncached context.
    fn uncached_biome(&self, qx: i32, qy: i32, qz: i32) -> BiomeId {
        let mut ctx = Context::uncached(&self.program);
        if let Some(end) = self.end_biomes {
            return self.end_biome(&end, &mut ctx, qx, qy, qz);
        }
        let (x, y, z) = (qx << 2, qy << 2, qz << 2);
        let climate = self.router.climate.map(|id| ctx.value(id, x, y, z));
        self.biome_at(climate)
    }

    /// The carver biome of a source chunk (`ChunkAccess.carverBiome`).
    pub fn carver_biome(&self, pos: ChunkPos) -> BiomeId {
        self.carver_biomes.get_or(pos, || self.uncached_biome(pos.min_block_x() >> 2, 0, pos.min_block_z() >> 2))
    }

    /// The TERRAIN step (`buildTerrain`): noise fill, material rules, carvers,
    /// then final heightmaps. `neighbors` must cover this chunk and its eight
    /// neighbors at BIOMES or later.
    pub fn build_terrain(&self, chunk: &mut Chunk, neighbors: &dyn BiomeLookup, possible_biomes: &[BiomeId]) {
        self.build_terrain_with(chunk, neighbors, possible_biomes, None);
    }

    /// `buildTerrain` with the chunk's `Beardifier` (structure terrain adaptation).
    pub fn build_terrain_with(
        &self,
        chunk: &mut Chunk,
        neighbors: &dyn BiomeLookup,
        possible_biomes: &[BiomeId],
        beardifier: Option<Arc<crate::structure::beardifier::Beardifier>>,
    ) {
        let registries = &*self.registries;
        let volume = Volume::blocks([16, self.height, 16], [chunk.pos.min_block_x(), self.min_y, chunk.pos.min_block_z()]);
        let mut ctx = Context::cached(&self.program);
        ctx.beardifier = beardifier;
        let started = std::time::Instant::now();
        let mut aquifer = self.new_aquifer(&mut ctx, &volume);
        crate::profile::add(crate::profile::AQUIFER_NEW, started);
        self.fill(chunk, &mut ctx, &mut aquifer, &volume);
        let started = std::time::Instant::now();
        self.build_surface(chunk, &mut ctx, neighbors, possible_biomes, &volume);
        crate::profile::add(crate::profile::SURFACE, started);
        let started = std::time::Instant::now();
        self.carve(chunk, &mut ctx, &mut aquifer);
        crate::profile::add(crate::profile::CARVERS, started);
        if let Some(planet) = &self.nether_planet {
            planet.dig(self.seed, chunk, registries);
        }
        let started = std::time::Instant::now();
        chunk.prime_heightmaps(&HeightmapKind::FINAL, registries);
        crate::profile::add(crate::profile::HEIGHTMAPS, started);
        chunk.status = ChunkStatus::Terrain;
    }

    fn new_aquifer(&self, ctx: &mut Context, volume: &Volume) -> Aquifer {
        let aquifer_random = self.random.from_hash_of("minecraft:aquifer").fork_positional();
        match self.aquifer {
            Some(samplers) => Aquifer::Noise(Box::new(NoiseAquifer::new(ctx, samplers, aquifer_random, volume, self.picker, self.fluids))),
            None => Aquifer::Disabled { picker: self.picker, blocks: self.fluids },
        }
    }

    /// `NoiseBasedChunkGenerator.iterateNoiseColumn`: one column's blocks from
    /// noise and aquifers alone (no structures, surface or carvers), from the
    /// top down. Returns the Y above the first block `stop` accepts.
    fn iterate_column(&self, x: i32, z: i32, mut stop: impl FnMut(i32, BlockStateId) -> bool) -> Option<i32> {
        let volume = Volume::blocks([1, self.height, 1], [x, self.min_y, z]);
        let mut ctx = Context::cached(&self.program);
        let mut aquifer = self.new_aquifer(&mut ctx, &volume);
        let density = ctx.sample(self.router.final_density, &volume);
        for y in (0..volume.size[1]).rev() {
            let by = volume.block_y(y);
            let d = density[volume.index(0, y, 0)];
            let state = aquifer.compute_substance(&mut ctx, x, by, z, f64::from(d)).unwrap_or(self.default_block);
            if stop(by, state) {
                return Some(by + 1);
            }
        }
        None
    }

    /// `ChunkGenerator.getBaseHeight`: the first free Y of a column's raw
    /// noise terrain for a worldgen heightmap.
    pub fn base_height(&self, x: i32, z: i32, kind: HeightmapKind) -> i32 {
        let registries = &*self.registries;
        self.iterate_column(x, z, |_, state| registries.heightmap_mask(state) & kind.bit() != 0).unwrap_or(self.chunk_min_y)
    }

    /// The same two raw heightmaps as `base_height`, sampled together for
    /// a tile. Structures surveying a large footprint reuse one density
    /// lattice and aquifer instead of rebuilding them for every column.
    /// Columns are X fastest, each [OceanFloorWg, WorldSurfaceWg].
    pub fn base_heightmaps(&self, pos: ChunkPos) -> Vec<[i32;2]> {
        let volume=Volume::blocks([16,self.height,16],[pos.min_block_x(),self.min_y,pos.min_block_z()]);
        let mut ctx=Context::cached(&self.program);
        let mut aquifer=self.new_aquifer(&mut ctx,&volume);
        let density=ctx.sample(self.router.final_density,&volume);
        let kinds=[HeightmapKind::OceanFloorWg,HeightmapKind::WorldSurfaceWg];
        let mut out=vec![[self.chunk_min_y;2];256];
        for z in 0..16 {for x in 0..16 {
            let mut found=[false;2];
            for y in (0..volume.size[1]).rev(){
                let by=volume.block_y(y);
                let d=density[volume.index(x,y,z)];
                let state=aquifer.compute_substance(&mut ctx,pos.min_block_x()+x,by,pos.min_block_z()+z,f64::from(d));
                let state=state.unwrap_or(self.default_block);
                let mask=self.registries.heightmap_mask(state);
                for k in 0..2 {if !found[k] && mask&kinds[k].bit()!=0 {out[(z*16+x) as usize][k]=by+1;found[k]=true;}}
                if found.iter().all(|v|*v){break;}
            }
        }}
        out
    }

    /// `ChunkGenerator.getFirstOccupiedHeight`.
    pub fn first_occupied_height(&self, x: i32, z: i32, kind: HeightmapKind) -> i32 {
        self.base_height(x, z, kind) - 1
    }

    /// `ChunkGenerator.getBaseColumn`: the raw noise column from the noise
    /// minimum Y upward.
    pub fn base_column(&self, x: i32, z: i32) -> (i32, Vec<BlockStateId>) {
        let mut column = vec![self.default_block; self.height as usize];
        let min_y = self.min_y;
        self.iterate_column(x, z, |y, state| {
            column[(y - min_y) as usize] = state;
            false
        });
        (min_y, column)
    }

    /// Heights for far-away terrain: for each column of an `n` x `n` grid
    /// starting at `min_x`, `min_z` with `step` blocks between columns (X
    /// fastest), the Y just above the raw noise terrain (final density above
    /// zero), searched from the top on a `y_step` lattice and refined
    /// linearly between the lattice points around the crossing. The search
    /// stops `2 * y_step` below sea level, so lower columns (sea floor)
    /// return that bottom. No aquifers, surface rules, carvers or
    /// structures: an estimate for distant views.
    ///
    /// Columns and the lattice should sit on noise cell corners (`min_x`,
    /// `min_z` and `step` multiples of 4, `y_step` of 8 in the Overworld):
    /// interpolated densities are then read straight from their inputs
    /// instead of being filled in block by block.
    pub fn far_heights(&self, min_x: i32, min_z: i32, n: i32, step: i32, y_step: i32) -> Vec<f32> {
        self.far_heights_within(min_x, min_z, n, step, y_step, None)
            .expect("the whole height is never clipped")
    }

    /// [`Self::far_heights`] searching only between `range` (lowest and
    /// highest Y, widened to the lattice): far cheaper where the ground's
    /// span is known (a coarser estimate of the same ground). `None` when
    /// a column's ground was not inside it (solid at its top, or air down
    /// to its bottom above the usual bottom): search the whole height then.
    pub fn far_heights_within(
        &self,
        min_x: i32,
        min_z: i32,
        n: i32,
        step: i32,
        y_step: i32,
        range: Option<(i32, i32)>,
    ) -> Option<Vec<f32>> {
        let full_bottom = (self.sea_level - 2 * y_step).div_euclid(y_step) * y_step;
        let full_top = self.min_y + self.height - 1;
        let (bottom, top) = match range {
            Some((lo, hi)) => (
                lo.div_euclid(y_step).saturating_mul(y_step).max(full_bottom),
                (hi.div_euclid(y_step) + 1).saturating_mul(y_step).min(full_top),
            ),
            None => (full_bottom, full_top),
        };
        if top <= bottom {
            return if range.is_some() { None } else { Some(vec![bottom as f32; (n * n) as usize]) };
        }
        let ny = (top - bottom) / y_step + 1;
        let mut ctx = Context::cached(&self.program);
        // Densities with Y fastest, then X, then Z.
        let density = if step % 4 == 0 && min_x % 4 == 0 && min_z % 4 == 0 {
            // A whole grid at once when it sits on cell corners.
            let volume = Volume::new([n, ny, n], [min_x, bottom, min_z], [step, y_step, step]);
            ctx.sample(self.router.final_density, &volume)
        } else if step < 4 {
            // Finer than a cell: the grid of cell corners around it, then
            // each column bilinearly between its four corner columns, as
            // the final density interpolates within a cell.
            let (cx, cz) = (min_x.div_euclid(4) * 4, min_z.div_euclid(4) * 4);
            let reach = |min: i32, cmin: i32| (min + (n - 1) * step - cmin) / 4 + 2;
            let (cnx, cnz) = (reach(min_x, cx), reach(min_z, cz));
            let volume = Volume::new([cnx, ny, cnz], [cx, bottom, cz], [4, y_step, 4]);
            let coarse = ctx.sample(self.router.final_density, &volume);
            let mut density = Vec::with_capacity((n * n * ny) as usize);
            for z in 0..n {
                let fz = (min_z + z * step - cz) as f32 / 4.0;
                let (iz, tz) = (fz.floor() as i32, fz.fract());
                for x in 0..n {
                    let fx = (min_x + x * step - cx) as f32 / 4.0;
                    let (ix, tx) = (fx.floor() as i32, fx.fract());
                    for y in 0..ny {
                        let c = |dx: i32, dz: i32| coarse[volume.index(ix + dx, y, iz + dz)];
                        let near = c(0, 0) + (c(1, 0) - c(0, 0)) * tx;
                        let far = c(0, 1) + (c(1, 1) - c(0, 1)) * tx;
                        density.push(near + (far - near) * tz);
                    }
                }
            }
            density
        } else {
            // Off the cell grid: a column at a time, which a volume of one
            // column keeps aligned.
            let mut density = Vec::with_capacity((n * n * ny) as usize);
            for z in 0..n {
                for x in 0..n {
                    let column = Volume::new([1, ny, 1], [min_x + x * step, bottom, min_z + z * step], [1, y_step, 1]);
                    density.extend(ctx.sample(self.router.final_density, &column));
                }
            }
            density
        };
        let at = |x: i32, y: i32, z: i32| density[(z * n + x) as usize * ny as usize + y as usize];
        let clipped_top = top < full_top;
        let clipped_bottom = bottom > full_bottom;
        let mut heights = Vec::with_capacity((n * n) as usize);
        for z in 0..n {
            for x in 0..n {
                let mut height = bottom as f32;
                // Ground over the searched span, or under it: not here.
                if clipped_top && at(x, ny - 1, z) > 0.0 {
                    return None;
                }
                if clipped_bottom && at(x, 0, z) <= 0.0 {
                    return None;
                }
                for y in (0..ny).rev() {
                    let d = at(x, y, z);
                    if d > 0.0 {
                        let above = if y + 1 < ny { at(x, y + 1, z) } else { d };
                        // Where density crosses zero between this solid
                        // sample and the air sample above it.
                        let t = if above <= 0.0 { d / (d - above) } else { 0.0 };
                        height = (bottom + y * y_step) as f32 + t * y_step as f32;
                        break;
                    }
                }
                heights.push(height);
            }
        }
        Some(heights)
    }

    /// The noise biome at quart coordinates (`BiomeSource.getNoiseBiome` with
    /// an uncached climate sampler).
    pub fn biome_at_quart(&self, qx: i32, qy: i32, qz: i32) -> BiomeId {
        self.uncached_biome(qx, qy, qz)
    }

    /// `doFill`.
    #[inline(never)]
    fn fill(&self, chunk: &mut Chunk, ctx: &mut Context, aquifer: &mut Aquifer, volume: &Volume) {
        let registries = &*self.registries;
        let started = std::time::Instant::now();
        let density = ctx.sample_columns(self.router.final_density, volume);
        crate::profile::add(crate::profile::DENSITY, started);
        let started = std::time::Instant::now();
        let air = registries.plain_air;
        chunk.generation.surface_fraction = vec![0.0; 256];
        for z in 0..16 {
            let bz = volume.block_z(z);
            for x in 0..16 {
                let bx = volume.block_x(x);
                // Blocks go in top-down, so once a heightmap stands two
                // above the current Y every later update of this column is
                // a no-op and is skipped.
                let mut settled = [false; 2];
                let mut topped = false;
                for y in (0..volume.size[1]).rev() {
                    let by = volume.block_y(y);
                    let d = density[volume.index(x, y, z)];
                    // The column's highest solid density: where between it
                    // and the air above the density crosses zero.
                    if !topped && d > 0.0 {
                        topped = true;
                        let above = if y + 1 < volume.size[1] { density[volume.index(x, y + 1, z)] } else { d };
                        let t = if above < 0.0 { d / (d - above) } else { 0.0 };
                        chunk.generation.surface_fraction[(z * 16 + x) as usize] = t.clamp(0.0, 1.0);
                    }
                    let state = aquifer.compute_substance(ctx, bx, by, bz, f64::from(d)).unwrap_or(self.default_block);
                    if state == air {
                        continue;
                    }
                    chunk.set_block_raw(x as usize, by, z as usize, state, registries);
                    for (k, kind) in [HeightmapKind::OceanFloorWg, HeightmapKind::WorldSurfaceWg].into_iter().enumerate() {
                        if !settled[k] {
                            chunk.update_heightmap(kind, x as usize, by, z as usize, state, registries);
                            settled[k] = by <= chunk.heightmaps.get(kind, x as usize, z as usize) - 1;
                        }
                    }
                    if aquifer.should_schedule_fluid_update() && registries.blocks.has_fluid(state) {
                        chunk.generation.post_processing.push((bx, by, bz));
                    }
                }
            }
        }
        crate::profile::add(crate::profile::FILL_BLOCKS, started);
    }

    fn random_factory(&self, name: &Identifier) -> AnyPositional {
        self.random.from_hash_of(name.as_str()).fork_positional()
    }

    /// `MaterialSystem.buildSurface`.
    #[inline(never)]
    fn build_surface(&self, chunk: &mut Chunk, ctx: &mut Context, biomes: &dyn BiomeLookup, possible: &[BiomeId], full: &Volume) {
        let registries = &*self.registries;
        let highest = chunk.sections().iter().rposition(|s| !s.is_empty()).map_or(-1, |i| i as i32);
        let max_block_y = (chunk.min_section_y() + highest) * 16 + 15;
        let narrowed = Volume::blocks([16, (max_block_y - full.min[1] + 1).max(1), 16], full.min);
        let mut factories = |name: &Identifier| self.random_factory(name);
        let compiling = std::time::Instant::now();
        let mut rules = ChunkRules::compile(&self.material, &self.material_rule, ctx, narrowed, self.gen_context(), self.zoom_seed, &mut factories, Some(possible));
        crate::profile::add(crate::profile::SURFACE_RULES, compiling);
        let (min_y, max_y) = (chunk.min_y(), chunk.min_y() + chunk.height() - 1);
        let eroded = registries.biomes.id("minecraft:eroded_badlands");
        let frozen = [registries.biomes.id("minecraft:frozen_ocean"), registries.biomes.id("minecraft:deep_frozen_ocean")];
        for x in 0..16usize {
            for z in 0..16usize {
                let bx = chunk.pos.min_block_x() + x as i32;
                let bz = chunk.pos.min_block_z() + z as i32;
                let starting_height = chunk.heightmaps.get(HeightmapKind::WorldSurfaceWg, x, z);
                let [qx, qy, qz] = zoom::quart_for_block(self.zoom_seed, bx, starting_height, bz);
                let surface_biome = biomes.noise_biome(qx, qy, qz);
                let mut column = ProtoColumn { chunk, x, z, registries, min_y, max_y, mark_fluids: true };
                if Some(surface_biome) == eroded {
                    self.material.eroded_badlands(&mut column, bx, bz, starting_height, min_y);
                }
                let height = column.chunk.heightmaps.get(HeightmapKind::WorldSurfaceWg, x, z);
                let wg = |c: &Chunk, x: usize, z: usize| c.heightmaps.get(HeightmapKind::WorldSurfaceWg, x, z);
                let gradient_x = wg(column.chunk, (x + 1).min(15), z) - wg(column.chunk, x.saturating_sub(1), z);
                let gradient_z = wg(column.chunk, x, (z + 1).min(15)) - wg(column.chunk, x, z.saturating_sub(1));
                rules.update_xz(bx, bz, gradient_x, gradient_z);
                let mut stone_above = 0;
                let mut water_height = i32::MIN;
                let mut next_ceiling = i32::MAX;
                let end_y = min_y;
                let mut y = height;
                while y >= end_y {
                    let old = column.get(y);
                    if registries.blocks.is_air(old) {
                        stone_above = 0;
                        water_height = i32::MIN;
                        y -= 1;
                        continue;
                    }
                    if registries.blocks.has_fluid(old) {
                        if water_height == i32::MIN {
                            water_height = y + 1;
                        }
                        y -= 1;
                        continue;
                    }
                    if next_ceiling >= y {
                        next_ceiling = crate::aquifer::WAY_BELOW_MIN_Y;
                        let mut look = y - 1;
                        while look >= end_y - 1 {
                            let s = column.get(look);
                            if registries.blocks.is_air(s) || registries.blocks.has_fluid(s) {
                                next_ceiling = look + 1;
                                break;
                            }
                            look -= 1;
                        }
                    }
                    stone_above += 1;
                    let stone_below = y - next_ceiling + 1;
                    rules.update_y(stone_above, stone_below, water_height, y);
                    if y >= min_y && y <= max_y {
                        if let Some(state) = rules.apply(ctx, biomes, registries) {
                            column.set(y, state);
                        }
                    }
                    y -= 1;
                }
                if frozen.contains(&Some(surface_biome)) {
                    let min_surface = rules.min_surface_level(ctx);
                    let info = registries.biomes.get(surface_biome);
                    let melt = self.material.temperature.at(info.temperature, info.frozen_temperature_modifier, bx, self.sea_level, bz, self.sea_level) > 0.1;
                    self.material.frozen_ocean(min_surface, melt, &mut column, bx, bz, starting_height);
                }
            }
        }
    }

    /// `generateCarvers` and `applyCarvingMask`.
    #[inline(never)]
    fn carve(&self, chunk: &mut Chunk, ctx: &mut Context, aquifer: &mut Aquifer) {
        let registries = &*self.registries;
        let context = self.gen_context();
        let pos = chunk.pos;
        let mut mask = CarvingMask::new(context.min_y + 1, context.min_y + context.depth - 1 - 7);
        // Carvers draw from `new WorldgenRandom(new LegacyRandomSource(..))`.
        let mut random = LegacyRandom::new(0);
        for dx in -8..=8 {
            for dz in -8..=8 {
                let source = ChunkPos::new(pos.x + dx, pos.z + dz);
                let biome = self.carver_biome(source);
                for (index, &carver) in self.biome_carvers[usize::from(biome.0)].iter().enumerate() {
                    legacy_large_feature_seed(&mut random, self.seed.wrapping_add(index as i64), source.x, source.z);
                    let carver = &self.carvers[carver];
                    if carver.is_start_chunk(&mut random) {
                        carver.carve(&context, &mut random, (pos.x, pos.z), (source.x, source.z), &mut mask);
                    }
                }
            }
        }
        if mask.is_empty() {
            return;
        }
        let min_y = chunk.min_y();
        let max_y = min_y + chunk.height() - 1;
        let mut visits = Vec::new();
        mask.visit(|x, z, bottom, top| visits.push((x, z, bottom, top)));
        let uncached = UncachedBiomes { generator: self };
        for (x, z, bottom, top) in visits {
            let mut has_grass = false;
            let (wx, wz) = (pos.min_block_x() + x, pos.min_block_z() + z);
            let mut y = top;
            while y >= bottom {
                let current = chunk.block(x as usize, y, z as usize);
                if registries.block_in_tag(current, self.uncarvable) {
                    y -= 1;
                    continue;
                }
                if current == self.grass || current == self.mycelium {
                    has_grass = true;
                }
                let Some(state) = aquifer.compute_substance(ctx, wx, y, wz, 0.0) else {
                    y -= 1;
                    continue;
                };
                let mut column = ProtoColumn { chunk, x: x as usize, z: z as usize, registries, min_y, max_y, mark_fluids: false };
                column.set(y, state);
                let fluid = registries.blocks.has_fluid(state);
                if aquifer.should_schedule_fluid_update() && fluid {
                    column.chunk.generation.post_processing.push((wx, y, wz));
                }
                if has_grass && column.get(y - 1) == self.dirt {
                    if let Some(top_material) = self.top_material(ctx, column.chunk, &uncached, wx, y - 1, wz, fluid) {
                        let mut column = ProtoColumn { chunk, x: x as usize, z: z as usize, registries, min_y, max_y, mark_fluids: false };
                        column.set(y - 1, top_material);
                        if registries.blocks.has_fluid(top_material) && y - 1 >= min_y {
                            column.chunk.generation.post_processing.push((wx, y - 1, wz));
                        }
                    }
                }
                y -= 1;
            }
        }
    }

    /// `MaterialSystem.topMaterial` for the block under carved grass.
    #[allow(clippy::too_many_arguments)]
    fn top_material(&self, ctx: &mut Context, chunk: &Chunk, biomes: &dyn BiomeLookup, x: i32, y: i32, z: i32, under_fluid: bool) -> Option<BlockStateId> {
        let volume = Volume::blocks([1, 1, 1], [x, y, z]);
        let mut factories = |name: &Identifier| self.random_factory(name);
        let mut rules = ChunkRules::compile(&self.material, &self.material_rule, ctx, volume, self.gen_context(), self.zoom_seed, &mut factories, None);
        let (lx, lz) = ((x & 15) as usize, (z & 15) as usize);
        let wg = |x: usize, z: usize| chunk.heightmaps.get(HeightmapKind::WorldSurfaceWg, x, z);
        rules.update_xz(x, z, wg((lx + 1).min(15), lz) - wg(lx.saturating_sub(1), lz), wg(lx, (lz + 1).min(15)) - wg(lx, lz.saturating_sub(1)));
        rules.update_y(1, 1, if under_fluid { y + 1 } else { i32::MIN }, y);
        rules.apply(ctx, biomes, &self.registries)
    }
}

/// Biomes from the uncached resolver, zoom applied by the caller (`withDifferentSource`).
struct UncachedBiomes<'g> {
    generator: &'g TerrainGenerator,
}

impl BiomeLookup for UncachedBiomes<'_> {
    fn noise_biome(&self, qx: i32, qy: i32, qz: i32) -> BiomeId {
        self.generator.uncached_biome(qx, qy, qz)
    }
}

/// A chunk column with `ProtoChunk.setBlockState` semantics.
struct ProtoColumn<'c> {
    chunk: &'c mut Chunk,
    x: usize,
    z: usize,
    registries: &'c Registries,
    min_y: i32,
    max_y: i32,
    /// `MaterialSystem`'s column marks the fluids it sets for post-processing.
    mark_fluids: bool,
}

impl Column for ProtoColumn<'_> {
    fn get(&self, y: i32) -> BlockStateId {
        self.chunk.block(self.x, y, self.z)
    }

    fn set(&mut self, y: i32, state: BlockStateId) {
        if y < self.min_y || y > self.max_y {
            return;
        }
        if self.mark_fluids && self.registries.blocks.has_fluid(state) {
            let pos = self.chunk.pos;
            self.chunk.generation.post_processing.push((pos.min_block_x() + self.x as i32, y, pos.min_block_z() + self.z as i32));
        }
        let section = &self.chunk.sections()[((y >> 4) - self.chunk.min_section_y()) as usize];
        if section.is_empty() && state == self.registries.plain_air {
            return;
        }
        self.chunk.set_block_raw(self.x, y, self.z, state, self.registries);
        // Heightmaps persisted before TERRAIN completes are the worldgen ones.
        for kind in ChunkStatus::Biomes.heightmaps() {
            self.chunk.update_heightmap(*kind, self.x, y, self.z, state, self.registries);
        }
    }

    fn is_air(&self, s: BlockStateId) -> bool {
        self.registries.blocks.is_air(s)
    }

    fn is_water(&self, s: BlockStateId) -> bool {
        self.registries.blocks.block_of(s) == self.registries.blocks.block_of(self.registries.blocks.parse_state("minecraft:water").expect("water"))
    }

    fn block_of(&self, s: BlockStateId) -> u16 {
        self.registries.blocks.block_of(s).0
    }
}

/// Biomes of a 3x3 chunk neighborhood (`WorldGenRegion.getNoiseBiome`).
pub struct NeighborBiomes<'c> {
    pub center: ChunkPos,
    pub chunks: [&'c Chunk; 9],
}

impl BiomeLookup for NeighborBiomes<'_> {
    fn noise_biome(&self, qx: i32, qy: i32, qz: i32) -> BiomeId {
        let (cx, cz) = (qx >> 2, qz >> 2);
        let (dx, dz) = (cx - self.center.x, cz - self.center.z);
        assert!(dx.abs() <= 1 && dz.abs() <= 1, "biome lookup outside the 3x3 neighborhood");
        let chunk = self.chunks[((dz + 1) * 3 + (dx + 1)) as usize];
        chunk.biome((qx & 3) as usize, qy, (qz & 3) as usize)
    }
}

impl NeighborBiomes<'_> {
    /// `collectPossibleBiomes(region, 1)`.
    pub fn possible(&self) -> Vec<BiomeId> {
        let mut out: Vec<BiomeId> = Vec::new();
        for c in &self.chunks {
            for s in c.sections() {
                for i in 0..64 {
                    let b = s.biomes.get(i);
                    if !out.contains(&b) {
                        out.push(b);
                    }
                }
            }
        }
        out
    }
}

/// `WorldgenRandom.setLargeFeatureSeed` on a legacy source.
fn legacy_large_feature_seed(random: &mut LegacyRandom, seed: i64, chunk_x: i32, chunk_z: i32) {
    random.set_seed(seed);
    let x_scale = random.next_i64();
    let z_scale = random.next_i64();
    random.set_seed(i64::from(chunk_x).wrapping_mul(x_scale) ^ i64::from(chunk_z).wrapping_mul(z_scale) ^ seed);
}
