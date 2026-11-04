//! Far terrain: a coarse heightfield of the world beyond the loaded chunks,
//! from the same seed's noise, so the horizon shows the real hills, seas
//! and biomes instead of fog.
//!
//! The ground is cut into square tiles of 32 x 32 columns on six levels of
//! detail (columns 1, 2, 4, 8, 16 and 32 blocks wide; the first ring, beside
//! the real chunks, is single blocks like them); a tile of one level covers
//! 2 x 2 tiles of the level below. Tiles near the eye split into finer ones,
//! but only once every finer tile is ready, so the ground never has holes
//! while it refines. Each column is a block-aligned box, flat on top with
//! walls down to its neighbours, so the distance stays cubic like the
//! chunks. Heights come from the generator's final density
//! ([`TerrainGenerator::far_heights`]) and colours from the noise biome at
//! the surface. Trees are not the generator's real trees (placing features
//! needs whole chunks) but trees at each biome's density, near, and a
//! continuous canopy over dense forests further out; caves, surface rules
//! and structures are left out.
//!
//! Everything runs on one background thread (with a few helpers for tile
//! heights); the game thread only passes the eye and takes finished tiles.
//! Every tile has the same vertex count and index pattern, so the renderer
//! keeps them in fixed slots of one pool and uploads each tile once.

use crate::mesh::{Atlas, BiomeTint};
#[path="far_tower.rs"] mod far_tower;
use crate::pack::ResourceId;
use crate::terrain::biome_sample;
use minecraftoss_core::Registries;
use minecraftoss_generator::terrain::TerrainGenerator;
use std::collections::HashMap;
use std::sync::{Arc, Mutex, mpsc};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

/// Columns on a tile side.
const QUADS: i32 = 64;
/// Blocks between columns on each level, finest first. Each level's tile
/// is twice as wide as the one below it, and levels switch where a column
/// is at most about 4 pixels wide on a 1440p screen (a block 1130 / d
/// pixels at d blocks), as Minecraft's own blocks look from that far: cubic
/// all the way, without giant blocks or sudden jumps.
const STEPS: [i32; 6] = [1, 2, 4, 8, 16, 32];
/// A tile of level `l + 1` closer to the eye than `SPLIT[l]` blocks shows
/// its 4 tiles of level `l` instead. Adrii (15/10): "con cubos mucho más
/// pequeños": 1-block columns out to ~900 blocks, 2-block ones to 2 km and
/// 4-block ones to the horizon (was 448 / 1152 / 2304, then 8-block
/// columns), so no column is much over a pixel wide at 1440p. The coarser
/// levels only cover the ground while the finer tiles are built.
const SPLIT: [f32; 5] = [896.0, 2048.0, 4096.0, 8192.0, 16384.0];
/// Blocks between density samples up a column.
const Y_STEP: i32 = 8;
/// A tile's ground span is kept on a grid of this many cells a side (its
/// lowest and highest column in each), for the finer tiles under it to
/// search only that span.
const SPAN_CELLS: usize = 8;
/// Searched under and over a coarser tile's span: the finer ground's dips
/// and peaks between the coarse columns (blocks).
const SPAN_BELOW: f32 = 16.0;
const SPAN_ABOVE: f32 = 32.0;

/// A built tile's ground span: per cell of a [`SPAN_CELLS`] grid, the
/// lowest and highest column height.
type Spans = [[f32; 2]; SPAN_CELLS * SPAN_CELLS];
/// Bytes of one [`FarVertex`].
pub const FAR_VERTEX_BYTES: u64 = 24;
/// Tile ids the builder hands out (the renderer keeps each tile's faces in
/// pools sized by what is actually drawn): about 900 are drawn at the
/// default radius, and coarser tiles stay shown while their finer ones are
/// built, which must fit too or finished tiles get thrown out and rebuilt
/// without end.
pub const POOL_SLOTS: u32 = 3000;
/// Far terrain vertex, packed: block offsets from the tile's corner (x and
/// z), height in eighths of a block and a spare; the face's tint, already
/// shaded as Minecraft shades a face's direction and corners, with the
/// face's axis in alpha (0 top, 1 facing x, 2 facing z, plus 4 on water);
/// and the block texture's atlas region (u0, v0, u1, v1 as unorm), tiled
/// once per block. The renderer adds the sky light and fog.
#[repr(C)]
#[derive(Clone, Copy, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct FarVertex {
    pub position: [i16; 4],
    pub colour: [u8; 4],
    pub region: [u16; 4],
    /// Water-bed texture mean RGB and depth in sky-light levels. Solid faces: zero.
    pub floor: [u8; 4],
}

/// Trees one tile may hold (only the finest tiles have separate trees;
/// coarser ones raise leaf-topped columns instead).
pub const TREES_PER_TILE: u32 = (QUADS * QUADS) as u32;
/// Bytes of one [`FarTree`].
pub const FAR_TREE_BYTES: u64 = 36;

/// One far tree, drawn as an instance of [`tree_mesh`]: a trunk block
/// column under a box of leaves.
#[repr(C)]
#[derive(Clone, Copy, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct FarTree {
    /// Middle of the trunk's bottom face, in blocks.
    pub base: [f32; 3],
    /// Height to the top of the leaves and width of the leaves, in
    /// sixteenths of a block.
    pub size: [u16; 2],
    /// Atlas regions of the leaves and of the log's side.
    pub leaves: [u16; 4],
    pub log: [u16; 4],
    /// RGB is the biome tint; alpha stores the crown form (0 broadleaf,
    /// 1 conifer, 2 acacia), interpreted by the instanced vertex shader.
    pub tint: [u8; 4],
}

/// The unit tree every [`FarTree`] stretches: per vertex x, y, z (y from 0
/// at the bottom of its part to 1 at its top, x and z from -0.5 to 0.5)
/// and `part * 4 + face`, where part 0 is the trunk and 1 the leaves, and
/// face 0 is a top, 1 a face along x, 2 along z, 3 a bottom. Two triangles
/// per four vertices.
pub fn tree_mesh() -> (Vec<[f32; 4]>, Vec<u16>) {
    let mut vertices = Vec::new();
    // A trunk and two closed, stepped crown tiers. Shared by every instance;
    // biome proportions are applied in the vertex shader, without new draws.
    for part in [0.0, 1.0, 2.0] {
        let code = |face: f32| part * 4.0 + face;
        let (a, b) = (-0.5, 0.5);
        // Sides along x (facing +x and -x), then along z.
        for x in [a, b] {
            vertices.extend(
                [[x, 0.0, a], [x, 1.0, a], [x, 1.0, b], [x, 0.0, b]]
                    .map(|[x, y, z]| [x, y, z, code(1.0)]),
            );
        }
        for z in [a, b] {
            vertices.extend(
                [[a, 0.0, z], [a, 1.0, z], [b, 1.0, z], [b, 0.0, z]]
                    .map(|[x, y, z]| [x, y, z, code(2.0)]),
            );
        }
        if part > 0.5 {
            vertices.extend(
                [[a, 1.0, a], [b, 1.0, a], [b, 1.0, b], [a, 1.0, b]]
                    .map(|[x, y, z]| [x, y, z, code(0.0)]),
            );
            vertices.extend(
                [[a, 0.0, a], [b, 0.0, a], [b, 0.0, b], [a, 0.0, b]]
                    .map(|[x, y, z]| [x, y, z, code(3.0)]),
            );
        }
    }
    let indices = (0..vertices.len() as u16 / 4)
        .flat_map(|q| {
            let b = q * 4;
            [b, b + 1, b + 2, b, b + 2, b + 3]
        })
        .collect();
    (vertices, indices)
}

/// One built tile: only the faces worth drawing (walls between columns of
/// one height are left out, and runs of equal flat tops are one face).
pub struct FarTile {
    /// The tile's corner in blocks, which [`FarVertex`] offsets are from.
    pub origin: [i32; 2],
    pub vertices: Vec<FarVertex>,
    /// Relative to the first vertex.
    pub indices: Vec<u32>,
    /// At most [`TREES_PER_TILE`].
    pub trees: Vec<FarTree>,
    /// Min then max corner in blocks, trees included: worked out here so
    /// the frame that uploads the tile does not walk its vertices.
    pub bounds: [f32; 6],
}

/// What changed since the game thread last asked.
#[derive(Default)]
pub struct FarUpdate {
    /// New tiles and their pool slots. Upload them before drawing `drawn`,
    /// which may name their slots.
    pub uploads: Vec<(u32, Arc<FarTile>)>,
    /// The slots to draw, nearest first, when that changed.
    pub drawn: Option<Arc<Vec<u32>>>,
    /// Slots no longer needed by the builder. They cannot be reused until
    /// the renderer has stopped showing them and acknowledges their release.
    pub retired: Vec<u32>,
    /// Renderer feedback: source epoch, followed by released slot ids.
    /// Changing epoch invalidates this source's GPU copy without keeping
    /// another copy of every tile on the CPU.
    pub feedback: Option<Arc<Mutex<(u64, Vec<u32>)>>>,
    pub epoch: u64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
struct TileKey {
    level: usize,
    x: i32,
    z: i32,
}

impl TileKey {
    fn size(self) -> f32 {
        (STEPS[self.level] * QUADS) as f32
    }

    fn min(self) -> [f32; 2] {
        let size = self.size();
        [self.x as f32 * size, self.z as f32 * size]
    }

    /// Horizontal distance from `eye` to the nearest point of the tile.
    fn distance(self, eye: [f32; 2]) -> f32 {
        let [x0, z0] = self.min();
        let size = self.size();
        let dx = (x0 - eye[0]).max(eye[0] - x0 - size).max(0.0);
        let dz = (z0 - eye[1]).max(eye[1] - z0 - size).max(0.0);
        (dx * dx + dz * dz).sqrt()
    }

    /// [`Self::distance`] counted longer the further it is from where the
    /// camera looks (up to twice behind it): what is in view comes first.
    fn seen_distance(self, eye: [f32; 2], facing: Option<[f32; 2]>) -> f32 {
        let d = self.distance(eye);
        let Some([fx, fz]) = facing else { return d };
        let [x0, z0] = self.min();
        let half = self.size() * 0.5;
        let (dx, dz) = (x0 + half - eye[0], z0 + half - eye[1]);
        let length = (dx * dx + dz * dz).sqrt();
        if length < 1.0 {
            return d;
        }
        let cos = (dx * fx + dz * fz) / length;
        d * (1.0 + 0.5 * (1.0 - cos))
    }

    /// Horizontal distance from `eye` to the farthest corner of the tile.
    fn far_corner(self, eye: [f32; 2]) -> f32 {
        let [x0, z0] = self.min();
        let size = self.size();
        let dx = (eye[0] - x0).abs().max((x0 + size - eye[0]).abs());
        let dz = (eye[1] - z0).abs().max((z0 + size - eye[1]).abs());
        (dx * dx + dz * dz).sqrt()
    }

    fn children(self) -> impl Iterator<Item = TileKey> {
        let level = self.level - 1;
        (0..4).map(move |i| TileKey {
            level,
            x: self.x * 2 + i % 2,
            z: self.z * 2 + i / 2,
        })
    }
}

/// What the game thread asks for.
#[derive(Clone, Copy)]
struct Request {
    eye: [f32; 2],
    /// Inside this radius the loaded chunks are drawn: tiles wholly within
    /// it are skipped.
    hole: f32,
    /// How far above the sea the eye is: detail goes by distance from the
    /// eye, so from high up the ground below is coarser too.
    height: f32,
    /// Where the camera looks (x, z, unit), when known: what is in view is
    /// built first.
    facing: Option<[f32; 2]>,
}

/// The far terrain of one overworld seed, built on a background thread.
pub struct FarTerrain {
    radius: f32,
    requests: mpsc::Sender<Option<Request>>,
    shared: Arc<Mutex<FarUpdate>>,
    feedback: Arc<Mutex<(u64, Vec<u32>)>>,
    thread: Option<JoinHandle<()>>,
}

impl FarTerrain {
    /// Starts building the far terrain of `seed` out to `radius` blocks,
    /// textured from `atlas` (the chunks' block atlas).
    pub fn spawn(
        registries: Arc<Registries>,
        seed: i64,
        tint: BiomeTint,
        atlas: &Atlas,
        radius: f32,
    ) -> Result<Self, String> {
        Self::spawn_in_world(registries,seed,tint,atlas,radius,None)
    }

    pub fn spawn_in_world(registries:Arc<Registries>,seed:i64,tint:BiomeTint,atlas:&Atlas,radius:f32,world:Option<&std::path::Path>)->Result<Self,String>{
        Self::spawn_with_context(registries,seed,tint,atlas,radius,world,None)
    }
    pub fn spawn_with_context(registries:Arc<Registries>,seed:i64,tint:BiomeTint,atlas:&Atlas,radius:f32,world:Option<&std::path::Path>,native:Option<Arc<minecraftoss_world::chunk_map::WorldGen>>)->Result<Self,String>{
        let showcase=crate::showcase::is_world(world);
        let generator = Arc::new(if showcase{crate::showcase::generator(registries.clone(),seed)?}else{TerrainGenerator::overworld(registries.clone(), seed)?});
        let (requests, receive) = mpsc::channel();
        let shared = Arc::new(Mutex::new(FarUpdate::default()));
        let feedback = Arc::new(Mutex::new((0, Vec::new())));
        let mut palette = Palette::new(&registries, &tint, atlas);
        if showcase{for biome in &mut palette.biomes{biome.trees=0.0;}}
        let shared_out = shared.clone();
        let feedback_out = feedback.clone();
        let thread = std::thread::Builder::new()
            .name("minecraft-far-terrain".into())
            .spawn(move || {
                minecraftoss_core::thread_priority::lowest();
                let land=if showcase {None}
                    else {native.or_else(||minecraftoss_world::chunk_map::WorldGen::new(generator.clone()).ok().map(Arc::new))};
                let mut builder = Builder {
                    land, preview:showcase.then_some(crate::showcase::TOWER),landmarks:HashMap::new(), tower_templates:None,
                    surveyed:Default::default(),survey_at:Instant::now()-Duration::from_secs(3),
                    generator,
                    palette,
                    radius,
                    spans: HashMap::new(),
                    slots: HashMap::new(),
                    free: (0..POOL_SLOTS).rev().collect(),
                    drawn: Vec::new(),
                    drawn_set: Default::default(),
                    shared: shared_out,
                    feedback: feedback_out,
                    epoch: 0,
                    retiring: Default::default(),
                };
                builder.run(&receive);
            })
            .map_err(|e| e.to_string())?;
        Ok(Self {
            radius,
            requests,
            shared,
            feedback,
            thread: Some(thread),
        })
    }

    pub fn radius(&self) -> f32 {
        self.radius
    }

    /// Tells the builder where the eye is (block coordinates) and how far
    /// out the loaded chunks reach.
    pub fn set_eye(&self, eye: [f64; 3], hole: f32) {
        self.set_eye_facing(eye, hole, None);
    }

    /// [`Self::set_eye`] with where the camera looks (block x and z): the
    /// ground in view is built before the ground behind.
    pub fn set_eye_facing(&self, eye: [f64; 3], hole: f32, facing: Option<[f32; 2]>) {
        let facing = facing.and_then(|[x, z]| {
            let length = (x * x + z * z).sqrt();
            (length > 1e-3).then(|| [x / length, z / length])
        });
        let _ = self.requests.send(Some(Request {
            eye: [eye[0] as f32, eye[2] as f32],
            hole,
            height: (eye[1] as f32 - 64.0).max(0.0),
            facing,
        }));
    }

    /// New tiles and the newest drawn list since the last call.
    pub fn take(&self) -> FarUpdate {
        let mut update = self.shared.lock().map(|mut shared| {
            let epoch = shared.epoch;
            let update = std::mem::take(&mut *shared);
            shared.epoch = epoch;
            update
        }).unwrap_or_default();
        update.feedback = Some(self.feedback.clone());
        if self.feedback.lock().is_ok_and(|feedback| feedback.0 != update.epoch) {
            // Returning to a source whose GPU pool was used by another
            // world: wait for its builder's new epoch, not stale slot ids.
            update.uploads.clear();
            update.retired.clear();
            update.drawn = Some(Arc::default());
        }
        update
    }
}

impl Drop for FarTerrain {
    fn drop(&mut self) {
        let _ = self.requests.send(None);
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

struct Builder {
    generator: Arc<TerrainGenerator>,
    land:Option<Arc<minecraftoss_world::chunk_map::WorldGen>>,
    preview:Option<(i32,i32,i32)>,
    landmarks:HashMap<(i32,i32,i32),Vec<u32>>,
    tower_templates:Option<Vec<FarTile>>,
    surveyed:std::collections::HashSet<(i32,i32)>,
    survey_at:Instant,
    palette: Palette,
    radius: f32,
    /// The ground span of each built tile, for its finer tiles.
    spans: HashMap<TileKey, Spans>,
    /// Built tiles and their pool slots.
    slots: HashMap<TileKey, u32>,
    free: Vec<u32>,
    drawn: Vec<TileKey>,
    drawn_set: std::collections::HashSet<TileKey>,
    shared: Arc<Mutex<FarUpdate>>,
    feedback: Arc<Mutex<(u64, Vec<u32>)>>,
    epoch: u64,
    retiring: std::collections::HashSet<u32>,
}

impl Builder {
    fn run(&mut self, receive: &mpsc::Receiver<Option<Request>>) {
        // Share the hardware with chunk generation and mesh compilation.
        let threads=std::thread::available_parallelism().map_or(4,|n|n.get());
        let helpers=minecraftoss_core::thread_priority::terrain_workers(threads)[2];
        // IW4L_MC_FAR_HELPERS: another count, for measuring.
        let helpers = std::env::var("IW4L_MC_FAR_HELPERS")
            .ok()
            .and_then(|value| value.parse::<usize>().ok())
            .map_or(helpers, |n| n.max(1));
        // IW4L_MC_FAR_LOG: each batch of tiles, its levels and its time.
        let log = std::env::var_os("IW4L_MC_FAR_LOG").is_some();
        let mut request: Option<Request> = None;
        let mut last_publish = Instant::now() - Duration::from_secs(1);
        loop {
            // Wait for the eye while there is nothing to build, then keep
            // only the newest request.
            let mut next = if request.is_none() {
                match receive.recv() {
                    Ok(next) => Some(next),
                    Err(_) => return,
                }
            } else {
                None
            };
            loop {
                match receive.try_recv() {
                    Ok(newer) => next = Some(newer),
                    Err(mpsc::TryRecvError::Empty) => break,
                    Err(mpsc::TryRecvError::Disconnected) => return,
                }
            }
            let fresh_request = matches!(next, Some(Some(_)));
            match next {
                Some(None) => return,
                Some(Some(newer)) => request = Some(newer),
                None => {}
            }
            if self.receive_releases() && !fresh_request {
                // An inactive cached world must not rebuild in the
                // background merely because another world took its GPU pool.
                request = None;
                continue;
            }
            let Some(current) = request else { continue };
            if self.refresh_landmarks(current){self.publish(&self.drawn,current.eye);}
            let (drawn, mut wanted) = self.select(current);
            let goal = self.goal(current);
            // The ground changes once it is complete for where the eye is, so
            // a still view does not watch it refine piece by piece; while
            // building goes on (moving, or a busy machine), at most every
            // two seconds, and at once while nothing is shown yet.
            let pause = if self.drawn.is_empty() { 100 } else { 600 };
            if drawn != self.drawn
                && (last_publish.elapsed() >= Duration::from_millis(pause) || wanted.is_empty())
            {
                self.publish(&drawn, current.eye);
                self.drawn = drawn.clone();
                self.drawn_set = drawn.iter().copied().collect();
                last_publish = Instant::now();
            }
            self.retire_unused(&drawn, &goal, current);
            if wanted.is_empty() {
                self.survey_next(current);
                // Nothing to build: sleep until the eye moves.
                match receive.recv_timeout(Duration::from_millis(500)) {
                    Ok(Some(newer)) => request = Some(newer),
                    Ok(None) | Err(mpsc::RecvTimeoutError::Disconnected) => return,
                    Err(mpsc::RecvTimeoutError::Timeout) => {}
                }
                continue;
            }
            // Ground where nothing is drawn yet first (coarse first: they
            // cover the most), then the finer tiles nearest the eye, of any
            // level: after a fast flight the ground beside the eye refines
            // before the horizon's.
            let covered: std::collections::HashSet<TileKey> = drawn.iter().copied().collect();
            let has_cover = |key: &TileKey| {
                (key.level + 1..STEPS.len()).any(|level| {
                    let shift = level - key.level;
                    covered.contains(&TileKey {
                        level,
                        x: key.x.div_euclid(1 << shift),
                        z: key.z.div_euclid(1 << shift),
                    })
                })
            };
            wanted.sort_by(|a, b| {
                has_cover(a)
                    .cmp(&has_cover(b))
                    .then_with(|| {
                        if has_cover(a) {
                            std::cmp::Ordering::Equal
                        } else {
                            b.level.cmp(&a.level)
                        }
                    })
                    .then(a.seen_distance(current.eye, current.facing).total_cmp(&b.seen_distance(current.eye, current.facing)))
            });
            let mut seen = std::collections::HashSet::new();
            wanted.retain(|key| seen.insert(*key));
            wanted.truncate(helpers * 2);
            // Neither the drawn tiles, nor those about to be drawn (their
            // list may still wait to be published), nor any tile the finished
            // ground here will draw give up their slots: evicting finished
            // children of a split still being built made the ground flip
            // between coarse and fine without end.
            wanted.truncate(self.free.len());
            if wanted.is_empty() {
                std::thread::sleep(Duration::from_millis(100));
                continue;
            }
            if self.survey_at.elapsed()>=Duration::from_secs(2){self.survey_next(current);}
            let started = Instant::now();
            let ranges: Vec<Option<(i32, i32)>> = wanted.iter().map(|key| self.range_for(*key)).collect();
            let jobs: Vec<(TileKey, Option<(i32, i32)>)> = wanted.iter().copied().zip(ranges).collect();
            let built: Vec<(TileKey, (FarTile, Spans))> = std::thread::scope(|scope| {
                let chunks: Vec<_> = jobs
                    .chunks(jobs.len().div_ceil(helpers))
                    .map(|keys| {
                        let (generator, palette) = (&self.generator, &self.palette);
                        scope.spawn(move || {
                            minecraftoss_core::thread_priority::lowest();
                            keys.iter()
                                .map(|&(key, range)| (key, build_tile(generator, palette, key, range)))
                                .collect::<Vec<_>>()
                        })
                    })
                    .collect();
                chunks
                    .into_iter()
                    .flat_map(|handle| handle.join().unwrap_or_default())
                    .collect()
            });
            if log {
                let levels: Vec<usize> = built.iter().map(|(key, _)| key.level).collect();
                eprintln!(
                    "far terrain: {} tiles {levels:?} in {:.0} ms, {} slots free, goal {}, drawn {}",
                    built.len(),
                    started.elapsed().as_secs_f64() * 1000.0,
                    self.free.len(),
                    goal.len(),
                    self.drawn.len()
                );
            }
            let Ok(mut shared) = self.shared.lock() else {
                return;
            };
            for (key, (tile, spans)) in built {
                let Some(slot) = self.free.pop() else { break };
                self.slots.insert(key, slot);
                self.spans.insert(key, spans);
                shared.uploads.push((slot, Arc::new(tile)));
            }
        }
    }

    /// Survey metadata only, on this low-priority builder. No full district,
    /// chunk generation, renderer work or process-lifetime WorldGen leaks.
    fn survey_next(&mut self,request:Request){
        self.survey_at=Instant::now();
        let Some(gen)=&self.land else{return;};if !gen.terrain.host_structures{return;}
        let radius=self.radius+512.;
        let mut next=None;let mut distance=f32::MAX;
        for x in ((request.eye[0]-radius)as i32).div_euclid(320)..=((request.eye[0]+radius)as i32).div_euclid(320){
         for z in ((request.eye[1]-radius)as i32).div_euclid(320)..=((request.eye[1]+radius)as i32).div_euclid(320){
          let cell=(x,z);if self.surveyed.contains(&cell){continue;}
          let Some((cx,cz))=crate::jak_structures::village_spot(self.generator.seed,cell)else{continue;};
          let key=TileKey{level:0,x:cx.div_euclid(64),z:cz.div_euclid(64)};
          let d=key.seen_distance(request.eye,request.facing);
          if d<distance{distance=d;next=Some(cell);}
         }
        }
        if let Some(cell)=next{self.surveyed.insert(cell);let _=crate::jak_skate_tower_site::far_site(gen,cell);}
        if self.surveyed.len()>4096{self.surveyed.retain(|&(x,z)|(x*320-request.eye[0]as i32).abs()<8192&&(z*320-request.eye[1]as i32).abs()<8192);}
    }

    fn refresh_landmarks(&mut self,request:Request)->bool{
        let origins=if let Some(center)=self.preview{vec![center]}else{crate::jak_city_arena::origins(self.generator.seed)};
        let mut centers:Vec<_>=origins.into_iter().filter(|&(x,_,z)|
            (x as f32-request.eye[0]).abs()<self.radius+160.&&(z as f32-request.eye[1]).abs()<self.radius+160.).collect();
        centers.sort_by(|a,b|{let d=|p:&(i32,i32,i32)|(p.0 as f32-request.eye[0]).powi(2)+(p.2 as f32-request.eye[1]).powi(2);d(a).total_cmp(&d(b))});
        centers.truncate(64);let mut changed=false;
        let old:Vec<_>=self.landmarks.keys().copied().filter(|p|!centers.contains(p)).collect();
        for p in old{if let Some(slots)=self.landmarks.remove(&p){for slot in slots{self.retiring.insert(slot);if let Ok(mut shared)=self.shared.lock(){shared.retired.push(slot);shared.uploads.retain(|(s,_)|*s!=slot);}}changed=true;}}
        for center in centers{
            if self.landmarks.contains_key(&center){continue;}
            let templates=self.tower_templates.get_or_insert_with(||far_tower::templates(&self.palette.tower_materials));
            if templates.is_empty()||self.free.len()<templates.len(){continue;}
            let mut slots=Vec::new();for template in templates.iter(){
                let slot=self.free.pop().unwrap();let tile=Arc::new(far_tower::positioned(template,center));
                if let Ok(mut shared)=self.shared.lock(){shared.uploads.push((slot,tile));}
                slots.push(slot);
            }
            self.landmarks.insert(center,slots);changed=true;
        }
        changed
    }

    /// Where the ground of `key` lies (lowest and highest Y), from the
    /// nearest coarser tile built over it, widened for what that coarser
    /// tile could not see; none without one.
    fn range_for(&self, key: TileKey) -> Option<(i32, i32)> {
        for level in key.level + 1..STEPS.len() {
            let shift = level - key.level;
            let ancestor = TileKey {
                level,
                x: key.x.div_euclid(1 << shift),
                z: key.z.div_euclid(1 << shift),
            };
            let Some(spans) = self.spans.get(&ancestor) else { continue };
            // The key's square (with its extra column) in the ancestor's
            // span cells, a cell more all round.
            let cell = ancestor.size() / SPAN_CELLS as f32;
            let [ax, az] = ancestor.min();
            let [kx, kz] = key.min();
            let pad = STEPS[key.level] as f32;
            let lo = |k: f32, a: f32| (((k - pad - a) / cell).floor() as i32 - 1).clamp(0, SPAN_CELLS as i32 - 1);
            let hi = |k: f32, a: f32| (((k + key.size() + 2.0 * pad - a) / cell).ceil() as i32).clamp(0, SPAN_CELLS as i32 - 1);
            let (mut low, mut high) = (f32::MAX, f32::MIN);
            for j in lo(kz, az)..=hi(kz, az) {
                for i in lo(kx, ax)..=hi(kx, ax) {
                    let [l, h] = spans[j as usize * SPAN_CELLS + i as usize];
                    low = low.min(l);
                    high = high.max(h);
                }
            }
            if low > high {
                return None;
            }
            return Some(((low - SPAN_BELOW).floor() as i32, (high + SPAN_ABOVE).ceil() as i32));
        }
        None
    }

    /// The tiles to draw for `request`, and the missing tiles wanted.
    fn select(&self, request: Request) -> (Vec<TileKey>, Vec<TileKey>) {
        let top = STEPS.len() - 1;
        let size = (STEPS[top] * QUADS) as f32;
        let (mut drawn, mut wanted) = (Vec::new(), Vec::new());
        let lo = |v: f32| ((v - self.radius) / size).floor() as i32;
        let hi = |v: f32| ((v + self.radius) / size).floor() as i32;
        for z in lo(request.eye[1])..=hi(request.eye[1]) {
            for x in lo(request.eye[0])..=hi(request.eye[0]) {
                let key = TileKey { level: top, x, z };
                if key.distance(request.eye) <= self.radius {
                    self.visit(key, request, &mut drawn, &mut wanted);
                }
            }
        }
        drawn.sort();
        (drawn, wanted)
    }

    /// The tiles the finished ground for `request` draws: the selection with
    /// every tile already built.
    fn goal(&self, request: Request) -> std::collections::HashSet<TileKey> {
        let top = STEPS.len() - 1;
        let size = (STEPS[top] * QUADS) as f32;
        let mut goal = std::collections::HashSet::new();
        let mut stack = Vec::new();
        let lo = |v: f32| ((v - self.radius) / size).floor() as i32;
        let hi = |v: f32| ((v + self.radius) / size).floor() as i32;
        for z in lo(request.eye[1])..=hi(request.eye[1]) {
            for x in lo(request.eye[0])..=hi(request.eye[0]) {
                let key = TileKey { level: top, x, z };
                if key.distance(request.eye) <= self.radius {
                    stack.push(key);
                }
            }
        }
        while let Some(key) = stack.pop() {
            if key.far_corner(request.eye) < request.hole {
                continue;
            }
            if self.splits(key, request) {
                stack.extend(key.children());
            } else {
                goal.insert(key);
            }
        }
        goal
    }

    /// Whether `key` shows its finer children at this eye. A split stays
    /// until the eye is a good way past where it began, and a merge
    /// likewise, so the ground does not flick between levels while the eye
    /// hovers at the threshold.
    fn splits(&self, key: TileKey, request: Request) -> bool {
        key.level > 0 && {
            let threshold = SPLIT[key.level - 1];
            // Split before: any finer tile under it was drawn.
            let split_before = key.children().any(|child| {
                self.drawn_set.contains(&child)
                    || (child.level > 0 && child.children().any(|g| self.drawn_set.contains(&g)))
            });
            let threshold = if split_before {
                threshold * 1.15
            } else {
                threshold
            };
            key.distance(request.eye).hypot(request.height) < threshold
        }
    }

    fn visit(
        &self,
        key: TileKey,
        request: Request,
        drawn: &mut Vec<TileKey>,
        wanted: &mut Vec<TileKey>,
    ) {
        if key.far_corner(request.eye) < request.hole {
            return;
        }
        let split_now = self.splits(key, request);
        if split_now {
            let missing: Vec<TileKey> = key
                .children()
                .filter(|child| {
                    !self.covered(*child, request)
                })
                .collect();
            if missing.is_empty() {
                for child in key.children() {
                    self.visit(child, request, drawn, wanted);
                }
                return;
            }
            wanted.extend(missing);
        }
        if self.slots.contains_key(&key) {
            drawn.push(key);
        } else {
            wanted.push(key);
            // A coarser mesh may have released its slot after refinement.
            // While rebuilding it for a merge, its existing finer coverage
            // stays on screen instead of publishing an empty square.
            // Keep every already-built sibling as well when a moving chunk
            // hole exposes a child that has not needed a mesh until now.
            self.append_cover(key, request, drawn);
        }
    }

    /// Coverage is independent of the continued residency of ancestors.
    /// Keeping a mesh for every internal node filled all 3000 ids before
    /// the ~2300 visible leaves around Haven could finish refining.
    fn covered(&self, key:TileKey, request:Request) -> bool {
        (request.hole>0.0 && key.far_corner(request.eye)<request.hole)
            || self.slots.contains_key(&key) || self.children_covered(key,request)
    }
    fn children_covered(&self, key:TileKey, request:Request) -> bool {
        key.level>0 && key.children().all(|child| self.covered(child,request))
    }
    fn append_cover(&self, key:TileKey, request:Request, drawn:&mut Vec<TileKey>) {
        if request.hole>0.0 && key.far_corner(request.eye)<request.hole {return;}
        if self.slots.contains_key(&key) {drawn.push(key);}
        else if key.level>0 {for child in key.children(){self.append_cover(child,request,drawn);}}
    }

    /// Hands the renderer the drawn slots, nearest first so near ground
    /// hides what lies behind it.
    fn publish(&self, drawn: &[TileKey], eye: [f32; 2]) {
        let mut order: Vec<(f32, u32)> = drawn
            .iter()
            .filter_map(|key| Some((key.distance(eye), *self.slots.get(key)?)))
            .collect();
        for (&(x,_,z),slots) in &self.landmarks{
            let distance=((x as f32-eye[0]).powi(2)+(z as f32-eye[1]).powi(2)).sqrt();
            order.extend(slots.iter().map(|&slot|(distance,slot)));
        }
        order.sort_by(|a, b| a.0.total_cmp(&b.0));
        if let Ok(mut shared) = self.shared.lock() {
            shared.drawn = Some(Arc::new(order.into_iter().map(|(_, slot)| slot).collect()));
        }
    }

    /// Retire obsolete cache entries even while there are spare slot ids:
    /// variable-sized geometry can fill its GPU pool before all ids are used.
    /// A finished ancestor only needs its height-span metadata, not another
    /// GPU mesh. Keep unfinished intermediate nodes until their children
    /// cover them, and keep published/selected coverage through transitions.
    fn retire_unused(
        &mut self,
        selected: &[TileKey],
        goal: &std::collections::HashSet<TileKey>,
        request: Request,
    ) {
        let mut kept = self.drawn_set.clone();
        kept.extend(selected.iter().copied());
        kept.extend(goal.iter().copied());
        let mut needed = std::collections::HashSet::new();
        for &key in goal {
            for level in key.level..STEPS.len() {
                let shift = level - key.level;
                needed.insert(TileKey {level, x:key.x.div_euclid(1 << shift), z:key.z.div_euclid(1 << shift)});
            }
        }
        let idle: Vec<_> = self.slots.keys().copied().filter(|key| !kept.contains(key)
            && (!needed.contains(key)||self.children_covered(*key,request))).collect();
        let mut retired = Vec::new();
        for key in idle {
            if let Some(slot) = self.slots.remove(&key) {
                self.retiring.insert(slot);
                retired.push(slot);
            }
        }
        self.spans.retain(|key,_|needed.contains(key)||kept.contains(key));
        if !retired.is_empty() {
            if let Ok(mut shared) = self.shared.lock() {
                shared.uploads.retain(|(slot, _)| !self.retiring.contains(slot));
                shared.retired.extend(retired);
            }
        }
    }

    /// A slot becomes reusable only after both renderer selections let go.
    /// A different epoch means the entire GPU copy was replaced by a world
    /// change; reset the builder so returning to this world uploads it again.
    fn receive_releases(&mut self) -> bool {
        let (epoch, released) = {
            let Ok(mut feedback) = self.feedback.lock() else { return false };
            (feedback.0, std::mem::take(&mut feedback.1))
        };
        if epoch != self.epoch {
            self.epoch = epoch;
            self.slots.clear(); self.spans.clear(); self.retiring.clear();self.landmarks.clear();
            self.free = (0..POOL_SLOTS).rev().collect();
            self.drawn.clear(); self.drawn_set.clear();
            if let Ok(mut shared) = self.shared.lock() {
                *shared = FarUpdate {epoch, drawn:Some(Arc::default()), ..Default::default()};
            }
            return true;
        }
        for slot in released {
            if self.retiring.remove(&slot) { self.free.push(slot); }
        }
        false
    }
}

/// A face's look: a block texture's atlas region (u0, v0, u1, v1 as unorm)
/// and the tint it is multiplied by, as the chunks draw it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Face {
    pub region: [u16; 4],
    pub tint: [f32; 3],
}

/// How a biome looks from afar, as the far terrain draws it: for other
/// views of the same world (the universe's planet seen from space) to
/// match it. `water` is ice in cold biomes; `trees` is the share of
/// 4 x 4-block columns with a tree.
#[derive(Clone, Copy, Debug)]
pub struct FarBiomeFaces {
    pub top: Face,
    pub side: Face,
    pub leaves: Face,
    pub canopy: Face,
    pub water: Face,
    pub trees: f32,
}

/// [`FarBiomeFaces`] for every biome, indexed by biome id, from the chunks'
/// block `atlas` and biome `tint`. Surface heights come from
/// [`TerrainGenerator::far_heights`].
pub fn biome_faces(registries: &Registries, tint: &BiomeTint, atlas: &Atlas) -> Vec<FarBiomeFaces> {
    let palette = Palette::new(registries, tint, atlas);
    palette
        .biomes
        .iter()
        .zip(&palette.water)
        .map(|(look, water)| FarBiomeFaces {
            top: look.top,
            side: look.side,
            leaves: look.leaves,
            canopy: look.canopy,
            water: *water,
            trees: look.trees,
        })
        .collect()
}

/// The far terrain's look, for building its columns outside its own tiles.
pub struct FarLook(Palette);

impl FarLook {
    pub fn new(registries: &Registries, tint: &BiomeTint, atlas: &Atlas) -> Self {
        Self(Palette::new(registries, tint, atlas))
    }
}

/// One column as the far terrain shows it: the height of its top (the sea's
/// surface over water), the height of the ground under it, its top's and
/// its walls' faces, and whether its top is water.
#[derive(Clone, Copy, Debug)]
pub struct FarColumn {
    pub top: f32,
    pub ground: f32,
    pub face: Face,
    pub wall: Face,
    pub water: bool,
}

/// The far terrain's columns on a grid of `n` × `n` columns `step` blocks
/// wide from block `(x0, z0)`, x fastest: what its own tiles would show
/// there (without the single trees of its finest levels), for coarser views
/// of the same world to carry it on at any width.
pub fn far_columns(
    generator: &TerrainGenerator,
    look: &FarLook,
    x0: i32,
    z0: i32,
    n: i32,
    step: i32,
) -> Vec<FarColumn> {
    let m = n + 3;
    let heights = generator.far_heights(x0 - step, z0 - step, m, step, Y_STEP);
    let sea = generator.sea_level as f32;
    let mut biomes = HashMap::new();
    let mut out = Vec::with_capacity((n * n) as usize);
    for j in 0..n {
        for i in 0..n {
            let c = column(
                generator,
                &look.0,
                &mut biomes,
                &heights,
                m,
                step,
                (x0 + i * step, z0 + j * step),
                (i, j),
            );
            let corner = |i: i32, j: i32| heights[((j + 1) * m + i + 1) as usize];
            let ground = if step >= 4 {
                (corner(i, j) + corner(i + 1, j) + corner(i, j + 1) + corner(i + 1, j + 1)) * 0.25
            } else {
                corner(i, j)
            };
            out.push(FarColumn {
                top: c.top,
                ground: ground.round(),
                face: c.face,
                wall: c.wall,
                water: c.top < sea,
            });
        }
    }
    out
}

/// The blocks a biome shows from afar, with the chunks' biome tints, and
/// how many columns carry a tree.
#[derive(Clone, Copy)]
struct BiomeLook {
    top: Face,
    side: Face,
    /// Rock below the thin surface soil, never an entire cliff of dirt.
    deep: Face,
    leaves: Face,
    log: Face,
    /// The top of a tree: leaves, or snow on them.
    canopy: Face,
    /// Share of the finest columns that are a tree.
    trees: f32,
    tree_profile: TreeProfile,
    cover: Cover,
}

#[derive(Clone, Copy)]
struct TreeProfile { height: f32, variation: f32, width: f32, form: u8 }

fn tree_profile(name: &str) -> TreeProfile {
    let (height, variation, width, form) = if name.contains("taiga") || name == "grove" || name.contains("snowy") {
        (7.0, 5.0, 3.0, 1)
    } else if name.contains("savanna") {
        (5.0, 3.0, 5.0, 2)
    } else if name.contains("jungle") {
        (9.0, 7.0, 5.0, 0)
    } else if name.contains("birch") {
        (6.0, 3.0, 3.0, 0)
    } else if name.contains("dark_forest") || name.contains("cherry") {
        (6.0, 3.0, 5.0, 0)
    } else {
        (4.0, 3.0, 4.0, 0)
    };
    TreeProfile { height, variation, width, form }
}

#[derive(Clone, Copy, PartialEq)]
enum Cover {
    Ground,
    Snow,
    Stone,
}

struct Palette {
    biomes: Vec<BiomeLook>,
    water: Vec<Face>,
    water_floor:Vec<[u8;3]>,
    stone: Face,
    /// Each of the Jak places' textures (`original_places`), by place and
    /// texture.
    places: Vec<Vec<Face>>,
    tower_materials:HashMap<String,(Face,bool)>,
    /// The ice texture's region, to tell frozen water apart.
    ice_region: [u16; 4],
}

/// Roughly the share of 4 x 4 block columns a tree covers in each biome.
fn tree_density(name: &str) -> f32 {
    let has = |part: &str| name.contains(part);
    if has("dark_forest") {
        0.85
    } else if has("sparse_jungle") || has("wooded_badlands") {
        0.25
    } else if has("jungle") || has("mangrove") {
        0.75
    } else if has("forest") || has("taiga") {
        0.6
    } else if has("cherry") || has("grove") {
        0.45
    } else if has("swamp") {
        0.35
    } else if has("savanna") {
        0.08
    } else if has("plains") || has("meadow") {
        0.02
    } else {
        0.0
    }
}

impl Palette {
    fn new(registries: &Registries, tint: &BiomeTint, atlas: &Atlas) -> Self {
        let region = |name: &str| {
            let id = ResourceId::parse(&format!("minecraft:block/{name}"))
                .expect("a block texture name");
            atlas
                .region(&id)
                .map(|v| (v.clamp(0.0, 1.0) * 65535.0).round() as u16)
        };
        let plain = |name: &str| Face {
            region: region(name),
            tint: [1.0; 3],
        };
        // The mashup's own (Jak 3's Wasteland's, the deserts'): its block
        // texture in the atlas, else vanilla's `fallback`.
        let jak = |name: &str, fallback: &str| Face {
            region: match ResourceId::parse(&format!("jak:block/{name}")) {
                Ok(id) if atlas.contains(&id) => atlas.region(&id).map(|v| (v.clamp(0.0, 1.0) * 65535.0).round() as u16),
                _ => region(fallback),
            },
            tint: [1.0; 3],
        };
        let rgb = |hex: u32| [hex >> 16, hex >> 8, hex].map(|c| (c & 255) as f32 / 255.0);
        let stone = plain("stone");
        let mut biomes = Vec::new();
        let mut water = Vec::new();
        let mut water_floor=Vec::new();
        let mean=|name:&str|{
            let [u0,v0,u1,v1]=plain(name).region.map(|v|v as f32/65535.);
            let (width,height)=atlas.pixels.dimensions();let mut rgb=[0u32;3];let mut count=0;
            for y in 0..8{for x in 0..8{
                let px=((u0+(u1-u0)*(x as f32+0.5)/8.)*width as f32)as u32;
                let py=((v0+(v1-v0)*(y as f32+0.5)/8.)*height as f32)as u32;
                let p=atlas.pixels.get_pixel(px.min(width-1),py.min(height-1));
                for i in 0..3{rgb[i]+=u32::from(p[i]);}count+=1;
            }}rgb.map(|v|(v/count)as u8)
        };
        let gravel=mean("gravel");let sand=mean("sand");
        for index in 0..registries.biomes.len() {
            let info = registries
                .biomes
                .get(minecraftoss_core::BiomeId(index as u16));
            let sample = biome_sample(info);
            let name = info.name.path();
            let has = |part: &str| name.contains(part);
            let foliage = tint.foliage(sample);
            let leaves = if has("birch") {
                Face {
                    region: region("birch_leaves"),
                    tint: rgb(0x80a755),
                }
            } else if has("taiga") || has("grove") || has("snowy") {
                Face {
                    region: region("spruce_leaves"),
                    tint: rgb(0x619961),
                }
            } else if has("jungle") {
                Face {
                    region: region("jungle_leaves"),
                    tint: foliage,
                }
            } else if has("dark_forest") {
                Face {
                    region: region("dark_oak_leaves"),
                    tint: foliage,
                }
            } else if has("cherry") {
                plain("cherry_leaves")
            } else if has("mangrove") {
                Face {
                    region: region("mangrove_leaves"),
                    tint: foliage,
                }
            } else if has("savanna") {
                Face {
                    region: region("acacia_leaves"),
                    tint: foliage,
                }
            } else {
                Face {
                    region: region("oak_leaves"),
                    tint: foliage,
                }
            };
            // The Jak lands (`jak_lands`): their own textures far off as near.
            let full = info.name.to_string();
            let snowy_land = crate::snowy::LAND.biomes.contains(&full.as_str());
            let swamp_land = crate::swamp::LAND.biomes.contains(&full.as_str());
            let leaves = if snowy_land {
                jak("snow-treesnow", "spruce_leaves")
            } else if swamp_land {
                jak("swp-tree-growth", "oak_leaves")
            } else {
                leaves
            };
            let log = if snowy_land {
                jak("snow-woodpole", "spruce_log")
            } else if swamp_land {
                jak("swp-bark", "oak_log")
            } else { plain(if has("birch") {
                "birch_log"
            } else if has("taiga") || has("grove") || has("snowy") {
                "spruce_log"
            } else if has("jungle") {
                "jungle_log"
            } else if has("dark_forest") {
                "dark_oak_log"
            } else if has("cherry") {
                "cherry_log"
            } else if has("mangrove") {
                "mangrove_log"
            } else if has("savanna") {
                "acacia_log"
            } else {
                "oak_log"
            }) };
            let snowy =
                has("snowy") || has("frozen") || has("ice") || has("grove") || has("jagged_peaks");
            let grass = Face {
                region: region("grass_block_top"),
                tint: tint.grass(sample),
            };
            let (top, side, cover) = if snowy_land {
                // Jak 1's Snowy Mountain (`snowy`): its snow over its rock.
                (jak("snow-03", "snow"), jak("snow-rock", "stone"), Cover::Snow)
            } else if swamp_land {
                // Jak 1's Boggy Swamp (`swamp`): its leafy ground over its mud.
                (jak("swp-leafyground", "grass_block_top"), jak("swp-mud-trans", "dirt"), Cover::Ground)
            } else if snowy {
                (plain("snow"), plain("snow"), Cover::Snow)
            } else if has("badlands") {
                (plain("red_sand"), plain("terracotta"), Cover::Ground)
            } else if has("desert") {
                // Jak 3's Wasteland (`wasteland`): its sand over its rock.
                (jak("des-beach-01", "sand"), jak("des-mount-01", "sandstone"), Cover::Ground)
            } else if has("beach") || has("ocean") || has("river") {
                (plain("sand"), plain("sandstone"), Cover::Ground)
            } else if has("stony") {
                (stone, stone, Cover::Stone)
            } else if has("gravelly") {
                (plain("gravel"), stone, Cover::Stone)
            } else if has("mushroom") {
                (plain("mycelium_top"), plain("dirt"), Cover::Ground)
            } else {
                (grass, plain("dirt"), Cover::Ground)
            };
            biomes.push(BiomeLook {
                top,
                side,
                deep: if snowy_land {jak("snow-rock", "stone")}
                    else if swamp_land {jak("swp-stand-rock", "stone")}
                    else if has("badlands") {plain("terracotta")}
                    else if has("desert") {jak("des-mount-01", "sandstone")}
                    else if has("beach") {plain("sandstone")} else {stone},
                leaves,
                log,
                canopy: if snowy_land { jak("snow-treesnow", "snow") } else if snowy { plain("snow") } else { leaves },
                trees: tree_density(name),
                tree_profile: tree_profile(if snowy_land { "snowy" } else { name }),
                cover,
            });
            // The native bed's texture mean, not an invented blue/dimming curve.
            water_floor.push(if has("beach")||has("desert"){sand}else{gravel});
            // Still water freezes over in cold biomes, as the chunks' does.
            water.push(if info.temperature < 0.15 {
                plain("ice")
            } else {
                Face {
                    region: region("water_still"),
                    tint: sample.water_color.map(|c| c as f32 / 255.0),
                }
            });
        }
        let ice_region = region("ice");
        let places = crate::original_places::PLACES
            .iter()
            .map(|place| crate::original_places::textures_of(place).iter().map(|t| jak(&t.to_lowercase(), "stone")).collect())
            .collect();
        Self {
            biomes,
            water,
            water_floor,
            stone,
            places,
            tower_materials:far_tower::materials(atlas),
            ice_region,
        }
    }
}

/// A well mixed hash of a column, in 0..1.
fn hash01(x: i32, z: i32, salt: u32) -> f32 {
    let mut h = (x as u32).wrapping_mul(0x9E37_79B1)
        ^ (z as u32).wrapping_mul(0x85EB_CA77)
        ^ salt.wrapping_mul(0xC2B2_AE3D);
    h ^= h >> 15;
    h = h.wrapping_mul(0x2C1B_3C6D);
    h ^= h >> 12;
    h = h.wrapping_mul(0x297A_2D39);
    h ^= h >> 15;
    (h >> 8) as f32 / (1u32 << 24) as f32
}

/// One column of a tile: its top, and how its top and walls look.
#[derive(Clone, Copy)]
struct Column {
    top: f32,
    face: Face,
    wall: Face,
    /// None for water, authored structures and continuous canopy.
    deep: Option<Face>,
    floor:[u8;4],
    tree: Option<FarTree>,
}

/// The noise biome at quart coordinates, each asked of the generator once
/// per tile: fine columns share their 4-block quart.
fn biome_at(
    generator: &TerrainGenerator,
    memo: &mut HashMap<(i32, i32, i32), minecraftoss_core::BiomeId>,
    qx: i32,
    qy: i32,
    qz: i32,
) -> minecraftoss_core::BiomeId {
    *memo
        .entry((qx, qy, qz))
        .or_insert_with(|| generator.biome_at_quart(qx, qy, qz))
}

#[allow(clippy::too_many_arguments)]
fn column(
    generator: &TerrainGenerator,
    palette: &Palette,
    biomes: &mut HashMap<(i32, i32, i32), minecraftoss_core::BiomeId>,
    heights: &[f32],
    n: i32,
    step: i32,
    (x, z): (i32, i32),
    (i, j): (i32, i32),
) -> Column {
    let sea = generator.sea_level as f32;
    let sample = |i: i32, j: i32| {
        let (i, j) = (i.clamp(-1, n - 2), j.clamp(-1, n - 2));
        heights[((j + 1) * n + i + 1) as usize]
    };
    // A wide column stands at the mean of its four corners: a lone sample
    // on a jagged peak made a tall tower as wide as the column.
    let raw = if step >= 4 {
        (sample(i, j) + sample(i + 1, j) + sample(i, j + 1) + sample(i + 1, j + 1)) * 0.25
    } else {
        sample(i, j)
    };
    // Spargus's city of blocks and the hamlets' houses: their roofs and
    // walls, in their place's textures.
    let on = crate::original_places::jak_places_on();
    let town = on.then(|| crate::spargus_town::far_at(&generator.registries, generator.seed, x, z)).flatten().map(|(top, texture)| (top, 0usize, texture));
    let built = town.or_else(|| on.then(|| crate::jak_hamlets::far_house_at(generator.seed, generator, x, z)).flatten());
    if let Some((top, place, texture)) = built {
        let index = crate::original_places::textures_of(&crate::original_places::PLACES[place]).iter().position(|t| *t == texture);
        if let Some(face) = index.and_then(|i| palette.places.get(place).and_then(|faces| faces.get(i))) {
            let face = Face { region: face.region, tint: [1.0; 3] };
            return Column { top, face, wall: face, deep: None, floor:[0;4], tree: None };
        }
    }
    // A Jak place's own top there (23/10: the noise's beach stood out of
    // Sandover's bay at the edge of the loaded chunks), or its sea.
    match crate::original_places::jak_places_on().then(|| crate::original_places::far_place(&generator.registries, generator.seed, x, z)).flatten() {
        Some(crate::original_places::FarPlace::Top { place, y, texture, color }) => {
            if let Some(face) = palette.places.get(place).and_then(|faces| faces.get(usize::from(texture))) {
                let face = Face { region: face.region, tint: color.map(|c| (f32::from(c) / 128.0).min(1.0)) };
                return Column { top: y.round(), face, wall: face, deep: None, floor:[0;4], tree: None };
            }
        }
        // Its sea at its own height (Spargus's is Jak 3's, far under
        // Minecraft's).
        Some(crate::original_places::FarPlace::Sea(height)) => {
            let biome = biome_at(generator, biomes, x >> 2, (height as i32) >> 2, z >> 2);
            let water = palette.water[usize::from(biome.0)];
            let [r,g,b]=palette.water_floor[usize::from(biome.0)];
            return Column {top:height-1.0/9.0-0.1,face:water,wall:water,deep:None,floor:[r,g,b,8],tree:None};
        }
        None => {}
    }
    if raw < sea {
        let biome = biome_at(generator, biomes, x >> 2, (sea as i32) >> 2, z >> 2);
        let water = palette.water[usize::from(biome.0)];
        let [r,g,b]=palette.water_floor[usize::from(biome.0)];
        return Column {top:sea-1.0/9.0-0.1,face:water,wall:water,deep:None,
            floor:[r,g,b,(sea-raw).round().clamp(0.,31.)as u8],tree:None};
    }
    let ground = raw.round();
    let biome = biome_at(generator, biomes, x >> 2, ground as i32 >> 2, z >> 2);
    let look = palette.biomes[usize::from(biome.0)];
    // Cliffs show stone; snow stays on all but the steepest.
    let at = |i: i32, j: i32| {
        let (i, j) = (i.clamp(-1, n - 2), j.clamp(-1, n - 2));
        heights[((j + 1) * n + i + 1) as usize].max(sea)
    };
    let dx = (at(i + 1, j) - at(i - 1, j)) / (2 * step) as f32;
    let dz = (at(i, j + 1) - at(i, j - 1)) / (2 * step) as f32;
    let steep = (dx * dx + dz * dz).sqrt();
    let rocky = match look.cover {
        Cover::Snow => steep > 1.85,
        Cover::Stone => true,
        Cover::Ground => steep > 1.15,
    };
    if rocky {
        return Column {
            top: ground,
            face: look.deep,
            wall: look.deep,
            deep: None,
            floor:[0;4],
            tree: None,
        };
    }
    let column = Column {
        top: ground,
        face: look.top,
        wall: look.side,
        deep: Some(look.deep),
        floor:[0;4],
        tree: None,
    };
    // Columns up to 4 blocks wide may stand a tree on their ground: a trunk
    // under a box of leaves.
    let profile = look.tree_profile;
    let height = profile.height + (hash01(x, z, 7) * profile.variation).floor();
    if step <= 8 {
        // As many trees per area as on 4-block columns.
        if hash01(x, z, step as u32) >= look.trees * (step * step) as f32 / 16.0 {
            return column;
        }
        // On a block inside the column, with the leaves inside it too.
        let width = profile.width;
        let offset = |salt: u32| {
            let room = (step as f32 - width).max(0.0);
            (width * 0.5 + (hash01(x, z, salt) * (room + 1.0)).floor().min(room)).floor() + 0.5
        };
        let unorm = |c: f32| (c.clamp(0.0, 1.0) * 255.0).round() as u8;
        let tint = look.leaves.tint.map(unorm);
        return Column {
            tree: Some(FarTree {
                base: [x as f32 + offset(11), ground, z as f32 + offset(13)],
                size: [(height * 16.0) as u16, (width * 16.0) as u16],
                leaves: look.leaves.region,
                log: look.log.region,
                tint: [tint[0], tint[1], tint[2], profile.form],
            }),
            ..column
        };
    }
    // Wider columns are seen from far enough that a forest reads as its
    // canopy: every column of a dense forest stands a tree's height of
    // leaves, so the canopy is continuous instead of scattered big blocks.
    if step >= 16 && look.trees >= 0.45 {
        return Column {
            top: ground + height,
            face: look.canopy,
            wall: look.leaves,
            deep: None,
            floor:[0;4],
            tree: None,
        };
    }
    column
}


/// Keep one continuous AO gradient across material boundaries. Repeating
/// WALL_AO on each soil/rock slice painted a false dark seam on the cliff.
fn wall_layers(lo:f32, high:Column, step:i32)->impl Iterator<Item=(f32,f32,Face,[f32;4])>{
    let hi=high.top;let mut slices=[None;2];
    let ao=|a:f32,b:f32|{
        let shade=|y:f32|0.7+0.3*((y-lo)/(hi-lo)).clamp(0.0,1.0);
        [shade(a),shade(b),shade(b),shade(a)]
    };
    if lo<hi {
        if let Some(deep)=high.deep {
            let cap=(hi-3.0).max(lo);
            if step>4 {slices[0]=Some((lo,hi,deep,ao(lo,hi)));}
            else{
                if lo<cap {slices[0]=Some((lo,cap,deep,ao(lo,cap)));}
                if cap<hi {slices[1]=Some((cap,hi,high.wall,ao(cap,hi)));}
            }
        }else{slices[0]=Some((lo,hi,high.wall,ao(lo,hi)));}
    }
    slices.into_iter().flatten()
}

fn build_tile(
    generator: &TerrainGenerator,
    palette: &Palette,
    key: TileKey,
    range: Option<(i32, i32)>,
) -> (FarTile, Spans) {
    let step = STEPS[key.level];
    let size = step * QUADS;
    let (x0, z0) = (key.x * size, key.z * size);
    // One extra column all around, for slopes and the edge walls.
    let n = QUADS + 3;
    let timer = Instant::now();
    // Only the span a coarser tile found here, when there is one (the whole
    // height again if the ground stepped out of it).
    let within = generator.far_heights_within(x0 - step, z0 - step, n, step, Y_STEP, range);
    let fell_back = range.is_some() && within.is_none();
    let heights = within.unwrap_or_else(|| generator.far_heights(x0 - step, z0 - step, n, step, Y_STEP));
    // Its own span, per cell, for the tiles under it.
    let mut spans: Spans = [[f32::MAX, f32::MIN]; SPAN_CELLS * SPAN_CELLS];
    let per = QUADS as usize / SPAN_CELLS;
    for j in 0..QUADS as usize {
        for i in 0..QUADS as usize {
            let h = heights[(j + 1) * n as usize + i + 1];
            let cell = &mut spans[(j / per) * SPAN_CELLS + i / per];
            cell[0] = cell[0].min(h);
            cell[1] = cell[1].max(h);
        }
    }
    let heights_at = timer.elapsed();
    let side = QUADS + 2;
    // Columns -1..=QUADS on each side.
    let mut columns = Vec::with_capacity((side * side) as usize);
    let mut biomes = HashMap::new();
    for j in -1..=QUADS {
        for i in -1..=QUADS {
            let at = (x0 + i * step, z0 + j * step);
            columns.push(column(
                generator,
                palette,
                &mut biomes,
                &heights,
                n,
                step,
                at,
                (i, j),
            ));
        }
    }
    let columns_at = timer.elapsed();
    let get = |i: i32, j: i32| columns[((j + 1) * side + i + 1) as usize];
    let mut vertices = Vec::new();
    let mut indices = Vec::new();
    // Water faces are flagged (4 in the axis byte), for the renderer to
    // tell sea from land.
    let water_region = palette.water.first().map(|water| water.region);
    // Minecraft's face shading: tops 1, faces along x 0.6, along z 0.8.
    // `ao` darkens each corner as Minecraft's smooth lighting darkens a
    // block's corners beside taller blocks and the foot of a wall. A wall
    // between columns of one height has no area and is left out.
    let mut quad = |corners: [[f32; 3]; 4], face: Face, axis: u8, ao: [f32; 4], floor:[[u8;4];4]| {
        if corners[0] == corners[1] {
            return;
        }
        let shade = [1.0, 0.6, 0.8][usize::from(axis)];
        let flags = axis
            | if Some(face.region) == water_region {
                4
            } else {
                0
            };
        let b = vertices.len() as u32;
        indices.extend_from_slice(&[b, b + 1, b + 2, b, b + 2, b + 3]);
        // A top corner keeps which way is out of its face (1 + 1 for +x,
        // + 2 for +z), for the renderer to widen tops by a pixel: merged
        // tops meet their neighbours at T-junctions, which left cracks.
        let outward = |k: usize| if axis == 0 { [1, 2, 4, 3][k] } else { 0 };
        for (k, (position, ao)) in corners.into_iter().zip(ao).enumerate() {
            let c = face
                .tint
                .map(|c| ((c * shade * ao).clamp(0.0, 1.0) * 255.0).round() as u8);
            vertices.push(FarVertex {
                position: [
                    (position[0] as i32 - x0) as i16,
                    (position[1] * 8.0).round() as i16,
                    (position[2] as i32 - z0) as i16,
                    outward(k),
                ],
                colour: [c[0], c[1], c[2], flags],
                region: face.region,
                floor:floor[k],
            });
        }
    };
    let s = step as f32;
    // Edge walls reach further down, hiding cracks against tiles of another
    // level whose columns stand at other heights.
    let skirt = 2.0 * s;
    // A wall between two columns, looking like the taller one's side, from
    // the lower top (less `drop`) to the higher.
    let wall = |a: Column, b: Column, drop: f32| {
        let (high, low) = if a.top >= b.top { (a, b) } else { (b, a) };
        (low.top - drop, high)
    };
    // Minecraft's ambient occlusion levels for each top corner: two taller
    // sides, or the count of taller sides and diagonal.
    let top_ao = |i: i32, j: i32| {
        let y = get(i, j).top;
        let taller = |di: i32, dj: i32| u8::from(get(i + di, j + dj).top > y + 0.5);
        let corner = |di: i32, dj: i32| {
            let (a, b) = (taller(di, 0), taller(0, dj));
            let level = if a + b == 2 {
                3
            } else {
                a + b + taller(di, dj)
            };
            [1.0, 0.8, 0.65, 0.5][usize::from(level)]
        };
        [corner(-1, -1), corner(1, -1), corner(1, 1), corner(-1, 1)]
    };
    // Shared corner samples keep the bed/depth continuous across both
    // neighbouring quads and tile boundaries. Do not stretch a row's first
    // underwater colour across a whole merged strip.
    let water_corner=|x:i32,z:i32|{
        let mut sum=[0u32;4];let mut n=0;
        for dx in [-1,0]{for dz in [-1,0]{let floor=get(x+dx,z+dz).floor;
            if floor[..3]!=[0;3]{for k in 0..4{sum[k]+=u32::from(floor[k]);}n+=1;}
        }}
        if n==0{[0;4]}else{sum.map(|v|((v+n/2)/n)as u8)}
    };
    for j in 0..QUADS {
        let z = (z0 + j * step) as f32;
        // Tops: a run of unshaded columns of one height and look along x is
        // one face (most of the far ground: plains, sea), the rest one each.
        let mut i = 0;
        while i < QUADS {
            let here = get(i, j);
            let ao = top_ao(i, j);
            let mut end = i + 1;
            if ao == [1.0; 4] {
                while end < QUADS {
                    let next = get(end, j);
                    if next.top != here.top || next.face != here.face || next.floor != here.floor || top_ao(end, j) != [1.0; 4]
                    {
                        break;
                    }
                    end += 1;
                }
            }
            let (xa, xb) = ((x0 + i * step) as f32, (x0 + end * step) as f32);
            let y = here.top;
            quad(
                [[xa, y, z], [xb, y, z], [xb, y, z + s], [xa, y, z + s]],
                here.face,
                0,
                ao,
                if here.floor[..3]==[0;3]{[[0;4];4]}else{
                    [(i,j),(end,j),(end,j+1),(i,j+1)].map(|(x,z)|water_corner(x,z))
                },
            );
            i = end;
        }
        for i in 0..QUADS {
            let here = get(i, j);
            let x = (x0 + i * step) as f32;
            let drop = if i == QUADS - 1 { skirt } else { 0.0 };
            let (lo, high) = wall(here, get(i + 1, j), drop);
            for (lo, hi, face, ao) in wall_layers(lo, high, step) {
                quad(
                [
                    [x + s, lo, z],
                    [x + s, hi, z],
                    [x + s, hi, z + s],
                    [x + s, lo, z + s],
                ],
                face,
                1,
                    ao,
                    [high.floor;4],
                );
            }
            let drop = if j == QUADS - 1 { skirt } else { 0.0 };
            let (lo, high) = wall(here, get(i, j + 1), drop);
            for (lo, hi, face, ao) in wall_layers(lo, high, step) {
                quad(
                [
                    [x, lo, z + s],
                    [x, hi, z + s],
                    [x + s, hi, z + s],
                    [x + s, lo, z + s],
                ],
                face,
                2,
                    ao,
                    [high.floor;4],
                );
            }
        }
    }
    for k in 0..QUADS {
        let (x, z) = ((x0 + k * step) as f32, (z0 + k * step) as f32);
        let (x_edge, z_edge) = (x0 as f32, z0 as f32);
        let (lo, high) = wall(get(-1, k), get(0, k), skirt);
        for (lo, hi, face, ao) in wall_layers(lo, high, step) {
            quad(
            [
                [x_edge, lo, z],
                [x_edge, hi, z],
                [x_edge, hi, z + s],
                [x_edge, lo, z + s],
            ],
            face,
            1,
                ao,
                [high.floor;4],
            );
        }
        let (lo, high) = wall(get(k, -1), get(k, 0), skirt);
        for (lo, hi, face, ao) in wall_layers(lo, high, step) {
            quad(
            [
                [x, lo, z_edge],
                [x, hi, z_edge],
                [x + s, hi, z_edge],
                [x + s, lo, z_edge],
            ],
            face,
            2,
                ao,
                [high.floor;4],
            );
        }
    }
    debug_assert!(vertices.len() <= u32::MAX as usize);
    let trees: Vec<FarTree> = (0..QUADS)
        .flat_map(|j| (0..QUADS).map(move |i| (i, j)))
        .filter_map(|(i, j)| get(i, j).tree)
        .collect();
    if std::env::var_os("IW4L_MC_FAR_LOG").is_some_and(|v| v == "2") {
        eprintln!(
            "far tile level {}: heights {:.0} ms, columns {:.0} ms, mesh {:.0} ms, span {range:?}{}",
            key.level,
            heights_at.as_secs_f64() * 1000.0,
            (columns_at - heights_at).as_secs_f64() * 1000.0,
            (timer.elapsed() - columns_at).as_secs_f64() * 1000.0,
            if fell_back { " (whole height again)" } else { "" }
        );
    }
    let mut bounds = [f32::MAX, f32::MAX, f32::MAX, f32::MIN, f32::MIN, f32::MIN];
    for vertex in vertices.iter() {
        let [x, y, z, _] = vertex.position.map(f32::from);
        let block = [x0 as f32 + x, y / 8.0, z0 as f32 + z];
        for axis in 0..3 {
            bounds[axis] = bounds[axis].min(block[axis]);
            bounds[axis + 3] = bounds[axis + 3].max(block[axis]);
        }
    }
    // Trees stand inside the tile, up to their tops.
    for tree in trees.iter() {
        bounds[4] = bounds[4].max(tree.base[1] + f32::from(tree.size[0]) / 16.0);
    }
    (
        FarTile {
            origin: [x0, z0],
            vertices,
            indices,
            trees,
            bounds,
        },
        spans,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    pub(super) fn residency_builder() -> Builder {
        let paths=minecraftoss_core::registries::DataPaths::discover().expect("Minecraft data paths");
        let registries=Arc::new(Registries::load(&paths).unwrap());
        Builder {generator:Arc::new(TerrainGenerator::overworld(registries,12345).unwrap()),
            land:None,preview:None,landmarks:HashMap::new(),tower_templates:None,surveyed:Default::default(),survey_at:Instant::now(),
            palette:Palette {biomes:Vec::new(),water:Vec::new(),water_floor:Vec::new(),stone:Face {region:[0;4],tint:[1.0;3]},places:Vec::new(),tower_materials:HashMap::new(),ice_region:[0;4]},
            radius:4096.0,spans:HashMap::new(),slots:HashMap::new(),free:(0..POOL_SLOTS).rev().collect(),
            drawn:Vec::new(),drawn_set:Default::default(),shared:Arc::new(Mutex::new(FarUpdate::default())),
            feedback:Arc::new(Mutex::new((0,Vec::new()))),epoch:0,retiring:Default::default()}
    }
    fn empty_tile() -> Arc<FarTile> {
        Arc::new(FarTile {origin:[0,0],vertices:Vec::new(),indices:Vec::new(),trees:Vec::new(),bounds:[0.0;6]})
    }
    fn request(eye:[f32;2],height:f32)->Request {Request {eye,height,hole:0.0,facing:None}}
    #[test]
    fn modular_tower_is_published_without_enabling_literal_jak_scenarios(){
        let mut builder=residency_builder();builder.preview=Some(crate::showcase::TOWER);
        builder.tower_templates=Some(vec![FarTile{origin:[0,0],vertices:vec![FarVertex{position:[0,0,0,0],colour:[255,255,255,0],region:[0;4],floor:[0;4]}],indices:vec![0],trees:Vec::new(),bounds:[0.;6]}]);
        let request=request([0.,650.],64.);
        assert!(builder.refresh_landmarks(request));assert_eq!(builder.landmarks.len(),1);
        builder.publish(&[],request.eye);
        let shared=builder.shared.lock().unwrap();assert_eq!(shared.uploads.len(),1);assert_eq!(shared.drawn.as_ref().unwrap().len(),1);drop(shared);
        assert!(!builder.refresh_landmarks(request),"stationary tower must not re-upload every frame");
        *builder.feedback.lock().unwrap()=(1,Vec::new());assert!(builder.receive_releases());assert!(builder.landmarks.is_empty());
        assert!(builder.refresh_landmarks(request),"returning world must restore its far tower after GPU epoch change");
    }
    #[test]
    fn obsolete_cache_retires_before_slot_exhaustion_but_ids_wait_for_gpu_release() {
        let mut builder=residency_builder();
        let fine=TileKey {level:0,x:0,z:0};let parent=TileKey {level:1,x:0,z:0};
        let old=TileKey {level:0,x:100,z:100};
        for key in [fine,parent,old] {let slot=builder.free.pop().unwrap();builder.slots.insert(key,slot);}
        let old_slot=builder.slots[&old];
        builder.shared.lock().unwrap().uploads.push((old_slot,empty_tile()));
        builder.drawn=vec![parent];builder.drawn_set.insert(parent);
        let before=builder.free.len();
        builder.retire_unused(&[parent],&[fine].into_iter().collect(),request([0.0,0.0],0.0));
        assert_eq!(builder.free.len(),before,"retirement is not permission to reuse a displayed id");
        assert!(builder.slots.contains_key(&fine));assert!(builder.slots.contains_key(&parent));
        assert!(!builder.slots.contains_key(&old));assert!(builder.retiring.contains(&old_slot));
        let shared=builder.shared.lock().unwrap();assert!(shared.uploads.is_empty());assert_eq!(shared.retired,vec![old_slot]);drop(shared);
        // Duplicate or unrelated acknowledgements must not duplicate ids.
        builder.feedback.lock().unwrap().1.extend([old_slot,old_slot,POOL_SLOTS+1]);
        assert!(!builder.receive_releases());assert_eq!(builder.free.len(),before+1);
        assert_eq!(builder.free.pop(),Some(old_slot));
        builder.slots.insert(old,old_slot);assert!(!builder.retiring.contains(&old_slot));
        builder.retire_unused(&[old],&[old].into_iter().collect(),request([6400.0,6400.0],0.0));assert_eq!(builder.slots[&old],old_slot);
    }
    #[test]
    fn haven_can_finish_over_two_thousand_leaf_tiles_without_resident_ancestors() {
        let mut builder=residency_builder();let request=request([-2791.5,-5528.5],4.0);
        for _ in 0..3 {
            let goal=builder.goal(request);
            assert!(goal.len()>2200&&goal.len()<POOL_SLOTS as usize,"real Haven goal: {}",goal.len());
            // Only leaf meshes are resident; height metadata and recursive
            // coverage do not consume vertex memory or additional tile ids.
            builder.slots.clear();builder.free=(0..POOL_SLOTS).rev().collect();
            for &key in &goal {let slot=builder.free.pop().unwrap();builder.slots.insert(key,slot);}
            let (drawn,wanted)=builder.select(request);
            assert!(wanted.is_empty(),"complete descendants must not request absent ancestor meshes");
            assert_eq!(drawn.iter().copied().collect::<std::collections::HashSet<_>>(),goal);
            assert_eq!(builder.slots.len()+builder.free.len(),POOL_SLOTS as usize);
            // Exercise the same hysteresis used after publishing a real view.
            builder.drawn=drawn.clone();builder.drawn_set=drawn.into_iter().collect();
        }
    }
    #[test]
    fn refinement_retires_only_covered_ancestors_and_keeps_their_height_metadata() {
        let mut builder=residency_builder();let parent=TileKey {level:2,x:0,z:0};
        let children:Vec<_>=parent.children().collect();let request=request([128.0,128.0],0.0);
        let goal:std::collections::HashSet<_>=children.iter().flat_map(|child|child.children()).collect();
        for key in std::iter::once(parent).chain(children.iter().copied()).chain(children[0].children()) {
            let slot=builder.free.pop().unwrap();builder.slots.insert(key,slot);builder.spans.insert(key,[[0.0,80.0];SPAN_CELLS*SPAN_CELLS]);
        }
        builder.drawn_set.insert(parent);
        builder.retire_unused(&[parent],&goal,request);
        assert!(!builder.slots.contains_key(&children[0]));assert!(builder.spans.contains_key(&children[0]));
        assert!(children[1..].iter().all(|child|builder.slots.contains_key(child)),"unfinished intermediates are still necessary coverage");
        let mut drawn=Vec::new();let mut wanted=Vec::new();builder.visit(parent,request,&mut drawn,&mut wanted);
        assert_eq!(drawn.len(),7);assert_eq!(wanted.len(),12);
        assert!(children[0].children().all(|child|drawn.contains(&child)));
        assert!(children[1..].iter().all(|child|drawn.contains(child)));
    }
    #[test]
    fn merging_waits_for_the_parent_mesh_while_its_finer_children_still_cover_the_square() {
        let mut builder=residency_builder();let parent=TileKey {level:1,x:0,z:0};
        let children:Vec<_>=parent.children().collect();let request=request([64.0,64.0],1000.0);
        for &key in &children {let slot=builder.free.pop().unwrap();builder.slots.insert(key,slot);}
        let mut drawn=Vec::new();let mut wanted=Vec::new();builder.visit(parent,request,&mut drawn,&mut wanted);
        assert_eq!(wanted,vec![parent]);assert_eq!(drawn,children);
        let slot=builder.free.pop().unwrap();builder.slots.insert(parent,slot);
        drawn.clear();wanted.clear();builder.visit(parent,request,&mut drawn,&mut wanted);
        assert_eq!(drawn,vec![parent]);assert!(wanted.is_empty());
        builder.drawn_set=drawn.iter().copied().collect();let free_before=builder.free.len();
        builder.retire_unused(&drawn,&[parent].into_iter().collect(),request);
        assert_eq!(builder.slots.len(),1);assert_eq!(builder.retiring.len(),4);
        assert_eq!(builder.free.len(),free_before,"old GPU coverage needs acknowledgement before its ids return");
    }
    #[test]
    fn moving_the_chunk_hole_does_not_remove_the_other_children_while_one_is_missing() {
        let mut builder=residency_builder();let parent=TileKey {level:1,x:0,z:0};
        let children:Vec<_>=parent.children().collect();
        for &key in &children[1..] {let slot=builder.free.pop().unwrap();builder.slots.insert(key,slot);}
        let before=Request {hole:60.0,..request([32.0,32.0],0.0)};
        let mut drawn=Vec::new();let mut wanted=Vec::new();
        builder.visit(parent,before,&mut drawn,&mut wanted);
        assert_eq!(drawn,children[1..]);assert!(wanted.is_empty());
        let after=Request {eye:[96.0,32.0],..before};drawn.clear();wanted.clear();
        builder.visit(parent,after,&mut drawn,&mut wanted);
        assert_eq!(drawn,children[2..],"exposing one missing child must preserve the built siblings outside the new hole");
        assert!(wanted.contains(&children[0]));assert!(wanted.contains(&parent));
        assert!(!drawn.contains(&children[0])&&!drawn.contains(&children[1]));
    }
    #[test]
    fn gpu_epoch_change_rebuilds_a_returning_world_without_stale_claims_or_uploads() {
        let mut builder=residency_builder();let key=TileKey {level:0,x:0,z:0};
        let slot=builder.free.pop().unwrap();builder.slots.insert(key,slot);
        let retiring=builder.free.pop().unwrap();
        builder.drawn=vec![key];builder.drawn_set.insert(key);builder.retiring.insert(retiring);
        builder.shared.lock().unwrap().uploads.push((slot,empty_tile()));
        *builder.feedback.lock().unwrap()=(1,vec![slot,retiring]);
        assert!(builder.receive_releases());assert_eq!(builder.epoch,1);
        assert!(builder.slots.is_empty());assert!(builder.retiring.is_empty());
        assert!(builder.drawn.is_empty());assert_eq!(builder.free.len(),POOL_SLOTS as usize);
        assert_eq!(builder.free.iter().copied().collect::<std::collections::HashSet<_>>().len(),POOL_SLOTS as usize);
        let shared=builder.shared.lock().unwrap();assert_eq!(shared.epoch,1);assert!(shared.uploads.is_empty());
        assert!(shared.drawn.as_ref().unwrap().is_empty());
    }

    #[test]
    fn children_cover_their_parent() {
        let parent = TileKey {
            level: 1,
            x: -3,
            z: 2,
        };
        let [px, pz] = parent.min();
        let mut min = [f32::MAX; 2];
        let mut max = [f32::MIN; 2];
        for child in parent.children() {
            let [x, z] = child.min();
            min = [min[0].min(x), min[1].min(z)];
            max = [max[0].max(x + child.size()), max[1].max(z + child.size())];
        }
        assert_eq!(min, [px, pz]);
        assert_eq!(max, [px + parent.size(), pz + parent.size()]);
    }

    #[test]
    fn distances() {
        let key = TileKey {
            level: 0,
            x: 0,
            z: 0,
        };
        assert_eq!(key.distance([10.0, 10.0]), 0.0);
        assert_eq!(key.distance([-10.0, 0.0]), 10.0);
        assert!((key.far_corner([0.0, 0.0]) - key.size() * 2f32.sqrt()).abs() < 0.01);
    }

    #[test]
    fn tree_mesh_indices_stay_in_the_mesh() {
        let (vertices, indices) = tree_mesh();
        assert_eq!(indices.len() % 3, 0);
        assert!(indices.iter().all(|&i| usize::from(i) < vertices.len()));
        assert_eq!(vertices.len(), 64, "bounded shared mesh, no per-tree geometry upload");
        assert!(vertices.iter().any(|v| v[3] >= 8.0));
    }

    #[test]
    fn distant_trees_keep_biome_silhouettes() {
        assert_eq!(tree_profile("taiga").form, 1);
        assert_eq!(tree_profile("savanna").form, 2);
        assert_eq!(tree_profile("cherry_grove").form, 0);
        assert_eq!(tree_profile("mangrove_swamp").form, 0);
        assert!(tree_profile("jungle").height > tree_profile("forest").height);
        assert!(tree_profile("savanna").width > tree_profile("birch_forest").width);
    }
}

#[cfg(test)]mod distant_material_regressions{
 use super::*;
 #[test]fn cliffs_keep_rock_below_soil_and_authored_walls_stay_authored(){
  let stone=Face{region:[1;4],tint:[1.;3]};let dirt=Face{region:[2;4],tint:[1.;3]};
  let c=Column{top:120.,face:dirt,wall:dirt,deep:Some(stone),floor:[0;4],tree:None};
  for step in [1,2,4]{let slices:Vec<_>=wall_layers(65.,c,step).collect();assert_eq!(slices.len(),2);
   assert_eq!((slices[0].0,slices[0].1,slices[0].2),(65.,117.,stone));
   assert_eq!((slices[1].0,slices[1].1,slices[1].2),(117.,120.,dirt));
   assert_eq!(slices[0].3[1],slices[1].3[0],"material boundary cannot reset lighting");}
  assert_eq!(wall_layers(119.,c,1).next().unwrap().2,dirt);
  assert_eq!(wall_layers(65.,c,16).next().unwrap().2,stone);
  assert_eq!(wall_layers(65.,Column{deep:None,..c},1).next().unwrap().2,dirt);
 }
}

#[cfg(test)]mod distant_water_regressions{
 use super::*;
 #[test]fn deep_water_preserves_native_tint_and_carries_bed_depth_instead_of_darkening_blue(){
  let mut builder=super::tests::residency_builder();let count=builder.generator.registries.biomes.len();
  let water=Face{region:[1;4],tint:[0.25,0.45,0.9]};builder.palette.water=vec![water;count];builder.palette.water_floor=vec![[112,109,108];count];
  let mut biomes=HashMap::new();
  for depth in [2,8,16]{let heights=vec![(builder.generator.sea_level-depth)as f32;25];let c=column(&builder.generator,&builder.palette,&mut biomes,&heights,5,1,(0,0),(0,0));
   assert_eq!(c.face,water);assert_eq!(c.floor,[112,109,108,depth as u8]);}
  assert_eq!(std::mem::size_of::<FarVertex>()as u64,FAR_VERTEX_BYTES);
 }
}
