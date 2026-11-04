use crate::{
    fluid::{FluidCell, FluidKind},
    lighting::SkyLight,
    model::{ResolvedModel, resolve_block_variants},
    pack::{PackStack, ResourceId},
    scene::{BiomeSample, Block, BlockPos, ChunkPos, Scene},
    texture_mips,
};
use anyhow::{Context, Result, anyhow};
use bytemuck::{Pod, Zeroable};
use glam::{Mat4, Vec3};
use image::{RgbaImage, imageops::FilterType};
use minecraftoss_player::items::WorldItems;
use std::{
    collections::{BTreeMap, HashMap},
    sync::Arc,
};

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Pod, Zeroable)]
pub struct Vertex {
    pub position: [f32; 3],
    pub uv: [f32; 2],
    pub color: [f32; 4],
    pub sky_light: f32,
    pub block_light: f32,
}
impl Vertex {}
/// A terrain section vertex as the GPU stores it: 28 bytes, the size of
/// vanilla's block vertex. Colours are bytes as `ARGB.colorFromFloat`
/// makes them (`floor(value * 255)`), and light keeps vanilla's lightmap
/// coordinate precision (a level in sixteenths: smooth lighting averages
/// four levels, so quarter levels are exact).
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Pod, Zeroable)]
pub struct SectionVertex {
    pub position: [f32; 3],
    pub uv: [f32; 2],
    pub color: [u8; 4],
    /// Sky then block light, times 16.
    pub light: [u8; 2],
    /// First byte: protect a decorative object from the wall aperture.
    pub pad: [u8; 2],
}

impl SectionVertex {
    pub fn from_vertex(v: &Vertex) -> Self {
        // `ARGB.as8BitChannel`.
        let channel = |value: f32| (value * 255.0).floor().clamp(0.0, 255.0) as u8;
        let light = |level: f32| (level * 16.0).round().clamp(0.0, 255.0) as u8;
        Self {
            position: v.position,
            uv: v.uv,
            color: v.color.map(channel),
            light: [light(v.sky_light), light(v.block_light)],
            pad: [0; 2],
        }
    }
}

/// A terrain section's mesh in the GPU vertex format.
#[derive(Default)]
pub struct SectionMesh {
    pub vertices: Vec<SectionVertex>,
    pub indices: Vec<u32>,
    pub transparent_start: Option<u32>,
    /// The block the vertex positions are relative to; see
    /// [`ChunkMesh::origin`].
    pub origin: [i32; 3],
}

impl From<ChunkMesh> for SectionMesh {
    fn from(mesh: ChunkMesh) -> Self {
        let mut vertices: Vec<_> = mesh.vertices.iter().map(SectionVertex::from_vertex).collect();
        for range in mesh.camera_protected {
            for vertex in &mut vertices[range] { vertex.pad[0] = 1; }
        }
        Self {
            vertices,
            indices: mesh.indices,
            transparent_start: mesh.transparent_start,
            origin: mesh.origin,
        }
    }
}

#[derive(Default)]
pub struct ChunkMesh {
    pub vertices: Vec<Vertex>,
    /// Decorative objects are whole silhouettes, not architectural walls.
    pub camera_protected: Vec<std::ops::Range<usize>>,
    pub indices: Vec<u32>,
    pub transparent_start: Option<u32>,
    pub faces: usize,
    /// The block the vertex positions are relative to. `[0, 0, 0]` (the
    /// default) keeps them absolute; a section origin keeps them small and
    /// exact anywhere in the world (an `f32` holds a block corner exactly
    /// only within ±2^20 blocks, and a block at all only within ±2^24).
    pub origin: [i32; 3],
}
pub struct Atlas {
    pub pixels: RgbaImage,
    pub mipmaps: Vec<RgbaImage>,
    slots: HashMap<ResourceId, [f32; 4]>,
    pub missing: Vec<ResourceId>,
    pub animated: Vec<ResourceId>,
    pub animated_tiles: Vec<AnimatedTile>,
}
pub struct AnimatedTile {
    /// Which texture it is.
    pub id: ResourceId,
    pub origin: (u32, u32),
    pub frames: Vec<Vec<RgbaImage>>,
    pub sequence: Vec<(usize, u32)>,
    pub interpolate: bool,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AnimationFrameState {
    pub current: usize,
    pub next: usize,
    pub progress_millis: u16,
}
impl AnimatedTile {
    pub fn state_at(&self, tick: u64) -> AnimationFrameState {
        let total: u64 = self
            .sequence
            .iter()
            .map(|(_, duration)| *duration as u64)
            .sum();
        let mut remaining = tick % total;
        for (position, &(frame, duration)) in self.sequence.iter().enumerate() {
            if remaining < duration as u64 {
                let next = self.sequence[(position + 1) % self.sequence.len()].0;
                let progress_millis = ((remaining as f32 / duration as f32) * 1000.0_f32) as u16;
                return AnimationFrameState {
                    current: frame,
                    next,
                    progress_millis,
                };
            }
            remaining -= duration as u64;
        }
        unreachable!("valid animation sequence has a positive duration")
    }
    pub fn frame_at(&self, tick: u64) -> usize {
        self.state_at(tick).current
    }
    pub fn image_at(
        &self,
        state: AnimationFrameState,
        level: usize,
    ) -> std::borrow::Cow<'_, RgbaImage> {
        let current = &self.frames[state.current][level];
        if !self.interpolate || state.progress_millis == 0 || state.current == state.next {
            return std::borrow::Cow::Borrowed(current);
        }
        let next = &self.frames[state.next][level];
        let progress = state.progress_millis as f32 / 1000.0;
        let mut image = RgbaImage::new(current.width(), current.height());
        for (x, y, pixel) in image.enumerate_pixels_mut() {
            let a = current.get_pixel(x, y);
            let b = next.get_pixel(x, y);
            for channel in 0..4 {
                let first = a[channel] as f32 / 255.0;
                let second = b[channel] as f32 / 255.0;
                pixel[channel] = ((first * (1.0 - progress) + second * progress) * 255.0)
                    .round()
                    .clamp(0.0, 255.0) as u8;
            }
        }
        std::borrow::Cow::Owned(image)
    }
}
impl Atlas {
    pub fn region(&self, id: &ResourceId) -> [f32; 4] {
        self.slots
            .get(id)
            .copied()
            .or_else(|| {
                self.slots
                    .get(&ResourceId::parse("minecraft:missingno").unwrap())
                    .copied()
            })
            .unwrap_or([0.0, 0.0, 1.0, 1.0])
    }
    /// Entity models address individual pixels inside a skin. The regular
    /// region is inset by half a texel to protect whole block sprites from
    /// neighbouring atlas tiles; scaling that inset over a skin moves inner
    /// UV boundaries into the adjacent body-part texture.
    /// Entity skins need their exact pixel boundaries; see `region_exact`.
    pub fn entity_region(&self, id: &ResourceId) -> [f32; 4] {
        self.region_exact(id)
    }
    pub fn contains(&self, id: &ResourceId) -> bool {
        self.slots.contains_key(id)
    }
    fn region_exact(&self, id: &ResourceId) -> [f32; 4] {
        let [u0, v0, u1, v1] = self.region(id);
        let half_texel = 0.5 / self.pixels.width() as f32;
        [
            u0 - half_texel,
            v0 - half_texel,
            u1 + half_texel,
            v1 + half_texel,
        ]
    }
}
pub struct Build {
    pub atlas: Arc<Atlas>,
    pub chunks: BTreeMap<ChunkPos, ChunkMesh>,
    pub sky_light: Option<Arc<SkyLight>>,
    pub scene_revision: u64,
    pub pack_generation: u64,
}
pub struct BiomeTint {
    grass: Option<RgbaImage>,
    foliage: Option<RgbaImage>,
    dry_foliage: Option<RgbaImage>,
}
impl BiomeTint {
    pub fn from_pack(packs: &PackStack) -> Result<Self> {
        let mut tint = Self {
            grass: None,
            foliage: None, dry_foliage: None,
        };
        for (name, output) in [("grass", &mut tint.grass), ("foliage", &mut tint.foliage), ("dry_foliage", &mut tint.dry_foliage)] {
            let id = ResourceId::parse(&format!("minecraft:colormap/{name}"))?;
            if let Some(bytes) = packs.texture(&id)? {
                let map = image::load_from_memory_with_format(&bytes, image::ImageFormat::Png)?
                    .to_rgba8();
                if map.width() >= 256 && map.height() >= 256 {
                    *output = Some(map);
                }
            }
        }
        Ok(tint)
    }
    fn color(
        map: &Option<RgbaImage>,
        override_color: Option<[u8; 3]>,
        biome: BiomeSample,
        fallback: [u8; 3],
    ) -> [f32; 3] {
        let rgb = override_color
            .or_else(|| {
                map.as_ref().map(|map| {
                    // 26.3 ColorMapColorUtil: rainfall is multiplied by temperature.
                    let temp = biome.temperature.clamp(0.0, 1.0);
                    let rain = biome.downfall.clamp(0.0, 1.0) * temp;
                    let pixel =
                        map.get_pixel(((1.0 - temp) * 255.0) as u32, ((1.0 - rain) * 255.0) as u32);
                    [pixel[0], pixel[1], pixel[2]]
                })
            })
            .unwrap_or(fallback);
        rgb.map(|channel| channel as f32 / 255.0)
    }
    pub(crate) fn grass(&self, biome: BiomeSample) -> [f32; 3] {
        Self::color(&self.grass, biome.grass_color, biome, [142, 185, 113])
    }
    pub(crate) fn foliage(&self, biome: BiomeSample) -> [f32; 3] {
        Self::color(&self.foliage, biome.foliage_color, biome, [113, 167, 77])
    }
    pub(crate) fn dry_foliage(&self, biome: BiomeSample) -> [f32;3] {
        Self::color(&self.dry_foliage,biome.dry_foliage_color,biome,[92,113,50])
    }
}
/// RedstoneWireBlock.COLORS, including ARGB.colorFromFloat's floor-to-byte step.
pub fn redstone_wire_color_argb(power: u8) -> u32 {
    let level = power.min(15) as f32 / 15.0;
    let red = level * 0.6 + if level > 0.0 { 0.4 } else { 0.3 };
    let green = (level * level * 0.7 - 0.5).clamp(0.0, 1.0);
    let blue = (level * level * 0.6 - 0.7).clamp(0.0, 1.0);
    let byte = |channel: f32| (channel * 255.0).floor() as u32;
    0xff00_0000 | byte(red) << 16 | byte(green) << 8 | byte(blue)
}
fn block_face_tint(block:&Block,index:Option<usize>,biome:BiomeSample,source:&BiomeTint)->[f32;3] {
    TintKind::of(block).tint(index,biome,source)
}
pub fn build<S: Scene>(scene: &S, packs: &PackStack) -> Result<Build> {
    build_internal(scene, packs, true)
}
pub fn build_minimal<S: Scene>(scene: &S, packs: &PackStack) -> Result<Build> {
    build_internal(scene, packs, false)
}
pub fn build_item_model(packs: &PackStack, item: &ResourceId) -> Result<Option<Build>> {
    let Some(model) = crate::model::resolve_item_model(packs, item)? else {
        return Ok(None);
    };
    if model.elements.is_empty() {
        return Ok(None);
    }
    let tints = packs
        .item_definition(item)?
        .map(|value| crate::interface::item_model_tints(packs, &value["model"]))
        .transpose()?
        .unwrap_or_default();
    let textures = model
        .elements
        .iter()
        .flat_map(|element| element.faces.iter())
        .map(|face| face.texture.clone())
        .collect::<std::collections::BTreeSet<_>>()
        .into_iter()
        .collect();
    let atlas = make_atlas(packs, textures)?;
    let mut mesh = ChunkMesh::default();
    for element in &model.elements {
        for face in &element.faces {
            let corners = model_face_corners(element, face)?;
            let [u0, v0, u1, v1] = atlas.region(&face.texture);
            let uv = face.vertex_uvs().map(|[u, v]| [u0 + (u1 - u0) * u, v0 + (v1 - v0) * v]);
            let tint = face
                .tint_index
                .and_then(|index| tints.get(index))
                .copied()
                .unwrap_or([255; 3]);
            let dir = if face.triangle.is_some() { model_face_direction(element, face, &corners)? } else { face.direction.as_str() };
            let shade = match dir {
                "up" => 1.0,
                "down" => 0.5,
                "north" | "south" => 0.8,
                _ => 0.6,
            };
            let start = mesh.vertices.len() as u32;
            for (position, uv) in corners.into_iter().zip(uv) {
                mesh.vertices.push(Vertex {
                    position,
                    uv,
                    color: [
                        shade * tint[0] as f32 / 255.0,
                        shade * tint[1] as f32 / 255.0,
                        shade * tint[2] as f32 / 255.0,
                        1.0,
                    ],
                    sky_light: 15.0,
                    block_light: 0.0,
                });
            }
            mesh.indices.extend_from_slice(&[
                start,
                start + 1,
                start + 2,
                start,
                start + 2,
                start + 3,
            ]);
            mesh.faces += 1;
        }
    }
    Ok(Some(Build {
        atlas: Arc::new(atlas),
        chunks: BTreeMap::from([((0, 0), mesh)]),
        sky_light: None,
        scene_revision: 0,
        pack_generation: packs.generation,
    }))
}
fn build_internal<S: Scene>(scene: &S, packs: &PackStack, preload_blocks: bool) -> Result<Build> {
    let mut models: HashMap<String, Vec<(ResolvedModel, u32)>> = HashMap::new();
    let mut textures = BTreeMap::<ResourceId, ()>::new();
    // A broken chest can remain as a dropped item after its block mesh is gone.
    textures.insert(ResourceId::parse("minecraft:entity/chest/normal")?, ());
    textures.insert(ResourceId::parse("minecraft:entity/beacon/beacon_beam")?, ());
    textures.insert(ResourceId::parse("minecraft:entity/bat/bat")?, ());
    for path in packs.list("minecraft", "textures/entity/banner/")? {
        if let Some(name)=path.strip_prefix("assets/minecraft/textures/").and_then(|s|s.strip_suffix(".png")) {
            textures.insert(ResourceId::parse(&format!("minecraft:{name}"))?,());
        }
    }
    // The bell's body (`BellRenderer`'s, not its block model's).
    textures.insert(ResourceId::parse("minecraft:entity/bell/bell_body")?, ());
    // Blazes, the Ender Dragon, End crystals and thrown eyes and fireballs.
    for texture in crate::boss_render::TEXTURES {
        textures.insert(ResourceId::parse(texture)?, ());
    }
    for texture in crate::arrow_render::TEXTURES {
        textures.insert(ResourceId::parse(texture)?, ());
    }
    // Jak's guns, ammo and eco, dropped or held.
    for texture in crate::jak_items::textures() {
        textures.insert(ResourceId::parse(&texture)?, ());
    }
    // The mashup's own blocks (the `jak-items` pack's block textures: the
    // grind rail's), placed or not yet.
    for path in packs.list("jak", "textures/block/")? {
        if let Some(name) = path
            .strip_prefix("assets/jak/textures/")
            .and_then(|name| name.strip_suffix(".png"))
        {
            textures.insert(ResourceId::parse(&format!("jak:{name}"))?, ());
        }
    }
    // MW2's guns and ammo (the `mw2-items` pack, when written).
    for path in packs.list("mw2", "textures/item/")? {
        if let Some(name) = path
            .strip_prefix("assets/mw2/textures/")
            .and_then(|name| name.strip_suffix(".png"))
        {
            textures.insert(ResourceId::parse(&format!("mw2:{name}"))?, ());
        }
    }
    textures.insert(ResourceId::parse("minecraft:entity/zombie/zombie")?, ());
    textures.insert(ResourceId::parse("minecraft:entity/player/wide/steve")?, ());
    textures.insert(ResourceId::parse("minecraft:entity/skeleton/skeleton")?, ());
    for skin in [
        "stray",
        "stray_overlay",
        "bogged",
        "bogged_overlay",
        "parched",
    ] {
        textures.insert(
            ResourceId::parse(&format!("minecraft:entity/skeleton/{skin}"))?,
            (),
        );
    }
    textures.insert(ResourceId::parse("minecraft:entity/creeper/creeper")?, ());
    textures.insert(
        ResourceId::parse("minecraft:entity/experience/experience_orb")?,
        (),
    );
    textures.insert(ResourceId::parse("minecraft:entity/spider/spider")?, ());
    textures.insert(ResourceId::parse("minecraft:entity/slime/slime")?, ());
    textures.insert(ResourceId::parse("minecraft:entity/enderman/enderman")?, ());
    textures.insert(
        ResourceId::parse("minecraft:entity/enderman/enderman_eyes")?,
        (),
    );
    textures.insert(ResourceId::parse("minecraft:entity/witch/witch")?, ());
    textures.insert(
        ResourceId::parse("minecraft:entity/iron_golem/iron_golem")?,
        (),
    );
    for cracks in ["low", "medium", "high"] {
        textures.insert(
            ResourceId::parse(&format!(
                "minecraft:entity/iron_golem/iron_golem_crackiness_{cracks}"
            ))?,
            (),
        );
    }
    // Wolves: each variant's wild, angry and tame looks, adult and baby,
    // and the collars.
    for variant in minecraftoss_entities::wolf::VARIANTS {
        let name = if variant == "pale" {
            "wolf".to_owned()
        } else {
            format!("wolf_{variant}")
        };
        for state in ["", "_angry", "_tame"] {
            for age in ["", "_baby"] {
                textures.insert(
                    ResourceId::parse(&format!("minecraft:entity/wolf/{name}{state}{age}"))?,
                    (),
                );
            }
        }
    }
    textures.insert(ResourceId::parse("minecraft:entity/wolf/wolf_collar")?, ());
    textures.insert(
        ResourceId::parse("minecraft:entity/wolf/wolf_collar_baby")?,
        (),
    );
    // Thrown splash potions: the bottle over its tinted contents.
    textures.insert(ResourceId::parse("minecraft:item/splash_potion")?, ());
    textures.insert(ResourceId::parse("minecraft:item/potion_overlay")?, ());
    textures.insert(ResourceId::parse("minecraft:entity/zombie/husk")?, ());
    textures.insert(ResourceId::parse("minecraft:entity/zombie/husk_baby")?, ());
    textures.insert(
        ResourceId::parse("minecraft:entity/zombie_villager/zombie_villager")?,
        (),
    );
    textures.insert(
        ResourceId::parse("minecraft:entity/zombie_villager/zombie_villager_baby")?,
        (),
    );
    for kind in [
        "desert", "jungle", "plains", "savanna", "snow", "swamp", "taiga",
    ] {
        textures.insert(
            ResourceId::parse(&format!("minecraft:entity/zombie_villager/type/{kind}"))?,
            (),
        );
        textures.insert(
            ResourceId::parse(&format!("minecraft:entity/zombie_villager/baby/{kind}"))?,
            (),
        );
    }
    for profession in [
        "armorer",
        "butcher",
        "cartographer",
        "cleric",
        "farmer",
        "fisherman",
        "fletcher",
        "leatherworker",
        "librarian",
        "mason",
        "nitwit",
        "shepherd",
        "toolsmith",
        "weaponsmith",
    ] {
        textures.insert(
            ResourceId::parse(&format!(
                "minecraft:entity/zombie_villager/profession/{profession}"
            ))?,
            (),
        );
    }
    textures.insert(
        ResourceId::parse("minecraft:entity/spider/spider_eyes")?,
        (),
    );
    textures.insert(ResourceId::parse("minecraft:entity/zombie/drowned")?, ());
    // Boats and minecarts (17/10).
    for texture in crate::vehicle_render::textures() {
        textures.insert(texture, ());
    }
    textures.insert(
        ResourceId::parse("minecraft:entity/zombie/drowned_baby")?,
        (),
    );
    for path in ["villager", "villager_baby"] {
        textures.insert(
            ResourceId::parse(&format!("minecraft:entity/villager/{path}"))?,
            (),
        );
    }
    for kind in [
        "desert", "jungle", "plains", "savanna", "snow", "swamp", "taiga",
    ] {
        textures.insert(
            ResourceId::parse(&format!("minecraft:entity/villager/type/{kind}"))?,
            (),
        );
        textures.insert(
            ResourceId::parse(&format!("minecraft:entity/villager/baby/{kind}"))?,
            (),
        );
    }
    for profession in [
        "armorer",
        "butcher",
        "cartographer",
        "cleric",
        "farmer",
        "fisherman",
        "fletcher",
        "leatherworker",
        "librarian",
        "mason",
        "nitwit",
        "shepherd",
        "toolsmith",
        "weaponsmith",
    ] {
        textures.insert(
            ResourceId::parse(&format!(
                "minecraft:entity/villager/profession/{profession}"
            ))?,
            (),
        );
    }
    for level in ["stone", "iron", "gold", "emerald", "diamond"] {
        textures.insert(
            ResourceId::parse(&format!(
                "minecraft:entity/villager/profession_level/{level}"
            ))?,
            (),
        );
    }
    textures.insert(
        ResourceId::parse("minecraft:entity/zombie/zombie_baby")?,
        (),
    );
    for kind in ["temperate", "warm", "cold"] {
        for suffix in ["", "_baby"] {
            textures.insert(
                ResourceId::parse(&format!("minecraft:entity/cow/cow_{kind}{suffix}"))?,
                (),
            );
            textures.insert(
                ResourceId::parse(&format!("minecraft:entity/pig/pig_{kind}{suffix}"))?,
                (),
            );
            textures.insert(
                ResourceId::parse(&format!("minecraft:entity/chicken/chicken_{kind}{suffix}"))?,
                (),
            );
        }
    }
    for kind in ["red", "brown"] {
        for suffix in ["", "_baby"] {
            textures.insert(
                ResourceId::parse(&format!("minecraft:entity/cow/mooshroom_{kind}{suffix}"))?,
                (),
            );
        }
        textures.insert(
            ResourceId::parse(&format!("minecraft:block/{kind}_mushroom"))?,
            (),
        );
    }
    for name in [
        "sheep",
        "sheep_baby",
        "sheep_wool",
        "sheep_wool_baby",
        "sheep_wool_undercoat",
    ] {
        textures.insert(
            ResourceId::parse(&format!("minecraft:entity/sheep/{name}"))?,
            (),
        );
    }
    textures.insert(
        ResourceId::parse("minecraft:entity/equipment/pig_saddle/saddle")?,
        (),
    );
    for id in crate::horse_render::all_textures() {
        textures.insert(id, ());
    }
    // Humanoid armour (`EquipmentLayerRenderer`'s humanoid layers).
    for layer in ["humanoid", "humanoid_leggings"] {
        for material in crate::armor_render::MATERIALS
            .iter()
            .copied()
            .chain(["leather_overlay"])
        {
            let id = ResourceId::parse(&format!("minecraft:entity/equipment/{layer}/{material}"))?;
            if packs.texture(&id)?.is_some() {
                textures.insert(id, ());
            }
        }
    }
    for chunk in scene.chunks() {
        for x in chunk.0 * 16..chunk.0 * 16 + 16 {
            for z in chunk.1 * 16..chunk.1 * 16 + 16 {
                for y in scene.vertical_range() {
                    let Some(block) = scene.block((x, y, z)) else {
                        continue;
                    };
                    let key = model_key(block);
                    if !models.contains_key(&key) {
                        let variants = resolve_block_variants(packs, block)
                            .with_context(|| format!("block {} at {x},{y},{z}", block.id.key()))?;
                        for (model, _) in &variants {
                            for element in &model.elements {
                                for face in &element.faces {
                                    textures.insert(face.texture.clone(), ());
                                }
                            }
                        }
                        models.insert(key, variants);
                    }
                }
            }
        }
    }
    // Reserve block and item sprites up front. Neither placing a new block
    // nor tossing an item should resize/reupload the atlas during gameplay.
    if preload_blocks {
        for prefix in ["textures/block/", "textures/item/"] {
            for path in packs.list("minecraft", prefix)? {
                if let Some(name) = path
                    .strip_prefix("assets/minecraft/textures/")
                    .and_then(|name| name.strip_suffix(".png"))
                {
                    textures.insert(ResourceId::parse(&format!("minecraft:{name}"))?, ());
                }
            }
        }
        for index in 0..12 {
            let id = ResourceId::parse(&format!("minecraft:particle/leaf_{index}"))?;
            textures.insert(id, ());
        }
        for index in 0..4 {
            textures.insert(
                ResourceId::parse(&format!("minecraft:particle/splash_{index}"))?,
                (),
            );
        }
        for index in 0..8 {
            textures.insert(
                ResourceId::parse(&format!("minecraft:particle/generic_{index}"))?,
                (),
            );
        }
        for name in ["drip_hang","drip_fall","drip_land"] {textures.insert(ResourceId::parse(&format!("minecraft:particle/{name}"))?,());}
        textures.insert(ResourceId::parse("minecraft:particle/note")?, ());
        textures.insert(ResourceId::parse("minecraft:particle/glow")?, ());
        textures.insert(ResourceId::parse("minecraft:misc/shadow")?, ());
    }
    let atlas = make_atlas(packs, textures.into_keys().collect())?;
    let tint = BiomeTint::from_pack(packs)?;
    let light = SkyLight::build(scene);
    let mut chunks = BTreeMap::new();
    for chunk in scene.chunks() {
        chunks.insert(
            chunk,
            mesh_chunk(scene, chunk, &models, &atlas, &tint, &light)?,
        );
    }
    Ok(Build {
        atlas: Arc::new(atlas),
        chunks,
        sky_light: Some(Arc::new(light)),
        scene_revision: scene.revision(),
        pack_generation: packs.generation,
    })
}
/// Rebuild the chunks whose face visibility or 15-block skylight neighborhood
/// can change after a single block edit, using the existing texture atlas.
/// A newly introduced texture requests an explicit atlas rebuild instead of
/// silently drawing a missing-texture face.
pub fn rebuild_near<S: Scene>(
    scene: &S,
    packs: &PackStack,
    build: &Build,
    pos: BlockPos,
) -> Result<BTreeMap<ChunkPos, ChunkMesh>> {
    rebuild_near_many_with_previous_light(
        scene,
        packs,
        &build.atlas,
        &build.chunks.keys().copied().collect::<Vec<_>>(),
        &[pos],
        build.sky_light.as_deref(),
    )
    .map(|(chunks, _)| chunks)
}
pub fn rebuild_near_with_atlas<S: Scene>(
    scene: &S,
    packs: &PackStack,
    atlas: &Atlas,
    old_chunks: &[ChunkPos],
    pos: BlockPos,
) -> Result<(BTreeMap<ChunkPos, ChunkMesh>, SkyLight)> {
    rebuild_near_many_with_atlas(scene, packs, atlas, old_chunks, &[pos])
}
pub fn rebuild_near_many_with_atlas<S: Scene>(
    scene: &S,
    packs: &PackStack,
    atlas: &Atlas,
    old_chunks: &[ChunkPos],
    positions: &[BlockPos],
) -> Result<(BTreeMap<ChunkPos, ChunkMesh>, SkyLight)> {
    rebuild_near_many_with_previous_light(scene, packs, atlas, old_chunks, positions, None)
}

pub fn rebuild_near_many_with_previous_light<S: Scene>(
    scene: &S,
    packs: &PackStack,
    atlas: &Atlas,
    old_chunks: &[ChunkPos],
    positions: &[BlockPos],
    previous_light: Option<&SkyLight>,
) -> Result<(BTreeMap<ChunkPos, ChunkMesh>, SkyLight)> {
    let mut chunks = scene.chunks();
    chunks.extend(old_chunks.iter().copied());
    chunks.sort_unstable();
    chunks.dedup();
    let affected = chunks
        .into_iter()
        .filter(|&(cx, cz)| {
            positions.iter().any(|pos| {
                let min_x = cx * 16;
                let min_z = cz * 16;
                min_x <= pos.0 + 15
                    && min_x + 15 >= pos.0 - 15
                    && min_z <= pos.2 + 15
                    && min_z + 15 >= pos.2 - 15
            })
        })
        .collect::<Vec<_>>();
    let mut models = HashMap::new();
    for &(cx, cz) in &affected {
        for x in cx * 16..cx * 16 + 16 {
            for z in cz * 16..cz * 16 + 16 {
                for y in scene.vertical_range() {
                    let Some(block) = scene.block((x, y, z)) else {
                        continue;
                    };
                    let key = model_key(block);
                    if models.contains_key(&key) {
                        continue;
                    }
                    let variants = resolve_block_variants(packs, block)
                        .with_context(|| format!("block {} at {x},{y},{z}", block.id.key()))?;
                    for (model, _) in &variants {
                        for element in &model.elements {
                            for face in &element.faces {
                                if !atlas.contains(&face.texture) {
                                    return Err(anyhow!("atlas missing {}", face.texture.key()));
                                }
                            }
                        }
                    }
                    models.insert(key, variants);
                }
            }
        }
    }
    let tint = BiomeTint::from_pack(packs)?;
    let light = previous_light.map_or_else(
        || SkyLight::build(scene),
        |old| old.updated(scene, positions),
    );
    let rebuilt: BTreeMap<ChunkPos, ChunkMesh> = affected
        .into_iter()
        .map(|chunk| {
            Ok((
                chunk,
                mesh_chunk(scene, chunk, &models, atlas, &tint, &light)?,
            ))
        })
        .collect::<Result<_>>()?;
    Ok((rebuilt, light))
}

/// Pack-resolved item models are cached, while entity positions, bob, and
/// rotation are rebuilt at frame rate from interpolated 20 Hz physics state.
#[derive(Default)]
pub struct ItemVisuals {
    models: HashMap<String, ItemVisual>,
    hand_displays: HashMap<String, [Option<Mat4>; 4]>,
    hand_generation: Option<u64>,
}
struct ItemVisual {
    /// Its faces as a dropped item draws them (corners in the model's
    /// space, atlas coordinates, tint, normal), worked out once.
    faces: Option<Vec<ItemFace>>,
    model: ResolvedModel,
    ground: Mat4,
    /// The `thirdperson_righthand` display transform.
    right_hand: Mat4,
    left_hand: Mat4,
    min_y: f32,
    tints: Vec<[f32; 3]>,
    flat: bool,
}
#[derive(Clone, Copy)]
struct ItemFace {
    first_tint:bool,
    corners: [Vec3; 4],
    uv: [[f32; 2]; 4],
    tint: [f32; 3],
    normal: Vec3,
}

/// A dropped item's faces, once: what `mesh_visible` would work out for
/// every face of every item every frame.
fn item_faces(visual: &ItemVisual, atlas: &Atlas) -> Result<Vec<ItemFace>> {
    let mut out = Vec::new();
    for element in &visual.model.elements {
        for face in &element.faces {
            if !atlas.contains(&face.texture) {
                continue;
            }
            let corners = model_face_corners(element, face)?;
            let [u0, v0, u1, v1] = atlas.region(&face.texture);
            let uv = face.vertex_uvs().map(|[u, v]| [u0 + (u1 - u0) * u, v0 + (v1 - v0) * v]);
            let tint = face
                .tint_index
                .and_then(|index| visual.tints.get(index))
                .copied()
                .unwrap_or([1.0; 3]);
            // Use the baked geometry: element rotations and both blockstate
            // axes have already been applied, with the correct handedness.
            let normal = triangle_normal(&corners);
            out.push(ItemFace { first_tint:face.tint_index==Some(0), corners: corners.map(Vec3::from_array), uv, tint, normal });
        }
    }
    Ok(out)
}

fn face_normal(direction: &str) -> Result<Vec3> {
    let (x, y, z) = self::direction(direction)?;
    Ok(Vec3::new(x as f32, y as f32, z as f32))
}
fn level_item_shade(normal: Vec3) -> f32 {
    // entity.vsh uses Lighting.Entry.LEVEL and minecraft_mix_light for
    // dropped item quads, in addition to the sampled world lightmap.
    let light0 = Vec3::new(0.2, 1.0, -0.7).normalize();
    let light1 = Vec3::new(-0.2, 1.0, 0.7).normalize();
    ((normal.dot(light0).max(0.0) + normal.dot(light1).max(0.0)) * 0.6 + 0.4).min(1.0)
}
impl ItemVisuals {
    pub fn clear(&mut self) {
        self.models.clear();
        self.hand_displays.clear();
        self.hand_generation=None;
    }
    /// Resource-model display matrices are baked once, including builtin
    /// special item bases such as banners which have no ordinary cuboid mesh.
    pub fn hand_transform(&mut self,id:&str,first_person:bool,left:bool,packs:&PackStack)->Result<Mat4> {
        if self.hand_generation!=Some(packs.generation) {
            self.hand_displays.clear();self.hand_generation=Some(packs.generation);
        }
        let index=usize::from(first_person)*2+usize::from(left);
        if let Some(value)=self.hand_displays.get(id).and_then(|a|a[index]) {return Ok(value);}
        let context=match(first_person,left){
            (false,false)=>"thirdperson_righthand",(false,true)=>"thirdperson_lefthand",
            (true,false)=>"firstperson_righthand",(true,true)=>"firstperson_lefthand",
        };
        let value=crate::model::item_display_transform(packs,&ResourceId::parse(id)?,context)?;
        self.hand_displays.entry(id.into()).or_insert([None;4])[index]=Some(value);
        Ok(value)
    }
    fn model<'a>(&'a mut self, id: &str, packs: &PackStack) -> Result<Option<&'a ItemVisual>> {
        if !self.models.contains_key(id) {
            let resource = ResourceId::parse(id)?;
            let Some(model) = crate::model::resolve_item_model(packs, &resource)? else {
                return Ok(None);
            };
            if model.elements.is_empty() {
                return Ok(None);
            }
            let ground = crate::model::item_display_transform(packs, &resource, "ground")?;
            let right_hand =
                crate::model::item_display_transform(packs, &resource, "thirdperson_righthand")?;
            let left_hand = crate::model::item_display_transform(packs, &resource, "thirdperson_lefthand")?;
            let definition = packs.item_definition(&resource)?;
            let tints = definition
                .as_ref()
                .map(|value| crate::interface::item_model_tints(packs, &value["model"]))
                .transpose()?
                .unwrap_or_default()
                .into_iter()
                .map(|rgb| rgb.map(|component| component as f32 / 255.0))
                .collect();
            let mut min = Vec3::splat(f32::INFINITY);
            let mut max = Vec3::splat(f32::NEG_INFINITY);
            for element in &model.elements {
                for x in [element.from[0], element.to[0]] {
                    for y in [element.from[1], element.to[1]] {
                        for z in [element.from[2], element.to[2]] {
                            let point = ground.transform_point3(Vec3::new(x, y, z));
                            min = min.min(point);
                            max = max.max(point);
                        }
                    }
                }
            }
            self.models.insert(
                id.into(),
                ItemVisual {
                    faces: None,
                    model,
                    ground,
                    right_hand,
                    left_hand,
                    min_y: min.y,
                    tints,
                    flat: max.z - min.z <= 0.0625,
                },
            );
        }
        Ok(self.models.get(id))
    }
    pub fn mesh(
        &mut self,
        items: &WorldItems,
        packs: &PackStack,
        atlas: &Atlas,
        sky_light: &SkyLight,
        partial_tick: f32,
    ) -> Result<ChunkMesh> {
        self.mesh_visible(items, packs, atlas, sky_light, partial_tick, |_| true)
    }

    /// [`Self::mesh`] for the items `visible` keeps (by where they are):
    /// vanilla draws an item entity only within 16 blocks of the camera
    /// (`Entity.shouldRenderAtSqrDistance`: a 0.25 box times 64).
    pub fn mesh_visible(
        &mut self,
        items: &WorldItems,
        packs: &PackStack,
        atlas: &Atlas,
        sky_light: &SkyLight,
        partial_tick: f32,
        visible: impl Fn(glam::DVec3) -> bool,
    ) -> Result<ChunkMesh> {
        let mut mesh = ChunkMesh::default();
        for (entity, pickup_position) in items.entities.iter().map(|entity| (entity, None)).chain(
            items
                .pickup_effects
                .iter()
                .map(|effect| (&effect.item, Some(effect.position(partial_tick)))),
        ) {
            if entity.stack.id==minecraftoss_core::mashup_skull_gem::ID && minecraftoss_core::mashup_skull_gem::native_visuals(){continue;}
            let partial = partial_tick.clamp(0.0, 1.0);
            let at = pickup_position.unwrap_or_else(|| {
                entity
                    .previous_position
                    .lerp(entity.position, partial as f64)
            });
            if !visible(at) {
                continue;
            }
            if entity.stack.id.starts_with("minecraft:") && entity.stack.id.ends_with("_banner") {
                let age=entity.age.saturating_sub(1) as f32+partial;
                let bob=(age/10.0+entity.bob_offset).sin()*0.1+0.1;
                let resource=ResourceId::parse(&entity.stack.id)?;
                let ground=crate::model::item_display_transform(packs,&resource,"ground")?;
                let floor=ground.transform_point3(Vec3::new(0.5,0.0,0.5)).y;
                let pose=Mat4::from_translation(at.as_vec3()+Vec3::Y*(bob+0.0625-floor))
                    *Mat4::from_rotation_y(age/20.0+entity.bob_offset)*ground;
                let part=crate::banner_render::item_mesh(&entity.stack,pose,at.as_vec3(),atlas,sky_light);
                let base=mesh.vertices.len() as u32;mesh.vertices.extend(part.vertices);mesh.indices.extend(part.indices.into_iter().map(|i|i+base));
                continue;
            }
            if self.model(&entity.stack.id, packs)?.is_none() {
                continue;
            }
            if let Some(visual) = self.models.get_mut(&entity.stack.id) {
                if visual.faces.is_none() {
                    visual.faces = Some(item_faces(visual, atlas)?);
                }
            }
            let Some(visual) = self.models.get(&entity.stack.id) else {
                continue;
            };
            let pos = pickup_position
                .unwrap_or_else(|| {
                    entity
                        .previous_position
                        .lerp(entity.position, partial as f64)
                })
                .as_vec3();
            let light_pos = (
                pos.x.floor() as i32,
                pos.y.floor() as i32,
                pos.z.floor() as i32,
            );
            let sky = sky_light.get(light_pos) as f32;
            let age = entity.age.saturating_sub(1) as f32 + partial;
            let bob = (age / 10.0 + entity.bob_offset).sin() * 0.1 + 0.1;
            let spin = age / 20.0 + entity.bob_offset;
            // Guns lie bigger than a block's worth of item: a thin MW2
            // weapon icon at a quarter block was all but lost on the
            // ground.
            let scale = if entity.stack.id.starts_with("iw4:weapon/") {
                2.2
            } else if crate::jak_items::item(&entity.stack.id)
                .is_some_and(|item| matches!(item.kind, crate::jak_items::JakKind::Gun { .. }))
            {
                1.6
            } else {
                1.0
            };
            let copies = match entity.stack.count {
                0 | 1 => 1,
                2..=16 => 2,
                17..=32 => 3,
                33..=48 => 4,
                _ => 5,
            };
            for copy in 0..copies {
                let offset = if copy == 0 {
                    Vec3::ZERO
                } else {
                    let seed = (entity.bob_offset.to_bits() as u64)
                        .wrapping_add((copy as u64).wrapping_mul(0x9e3779b97f4a7c15));
                    let random = |shift: u32| {
                        ((seed.rotate_left(shift) as u32) as f32 / u32::MAX as f32 * 2.0 - 1.0)
                            * 0.15
                    };
                    Vec3::new(
                        random(7),
                        random(19),
                        if visual.flat { 0.0 } else { random(31) },
                    )
                };
                let pose = Mat4::from_translation(pos + Vec3::Y * (bob + (0.0625 - visual.min_y) * scale))
                    * Mat4::from_rotation_y(spin)
                    * Mat4::from_translation(offset)
                    * Mat4::from_scale(Vec3::splat(scale))
                    * visual.ground;
                let normal_pose = pose.inverse().transpose();
                let block_light = sky_light.get_block(light_pos) as f32;
                let first_tint=crate::item_icons::potion_tint(&entity.stack);
                for face in visual.faces.iter().flatten() {
                    let tint=if face.first_tint {first_tint.unwrap_or(face.tint)}else{face.tint};
                    let shade = level_item_shade(normal_pose.transform_vector3(face.normal).normalize());
                    let start = mesh.vertices.len() as u32;
                    for (corner, uv) in face.corners.into_iter().zip(face.uv) {
                        mesh.vertices.push(Vertex {
                            position: pose.transform_point3(corner).to_array(),
                            uv,
                            color: [tint[0] * shade, tint[1] * shade, tint[2] * shade, 1.0],
                            sky_light: sky,
                            block_light,
                        });
                    }
                    mesh.indices.extend_from_slice(&[start, start + 1, start + 2, start, start + 2, start + 3]);
                    mesh.faces += 1;
                }
            }
        }
        Ok(mesh)
    }
}

impl ItemVisuals {
    /// `TntRenderer`: primed TNT as the TNT block, swelling over the last
    /// ten ticks of its fuse and flashing white on alternate five-tick
    /// windows. Each entry is an interpolated position and the remaining
    /// fuse (`fuse - partialTick + 1`). The white overlay mixes 75% white
    /// into the texture in vanilla; here the vertex colour brightens it.
    pub fn append_primed_tnt(
        &mut self,
        mesh: &mut ChunkMesh,
        tnt: &[(Vec3, f32)],
        packs: &PackStack,
        atlas: &Atlas,
        sky_light: &SkyLight,
    ) -> Result<()> {
        for &(pos, fuse) in tnt {
            let mut scale = 1.0;
            if fuse < 10.0 {
                let g = (1.0 - fuse / 10.0).clamp(0.0, 1.0);
                let g = g * g;
                scale = 1.0 + g * g * 0.3;
            }
            let white = (fuse as i32) / 5 % 2 == 0;
            let pose = Mat4::from_translation(pos + Vec3::Y * 0.5)
                * Mat4::from_scale(Vec3::splat(scale))
                * Mat4::from_rotation_y((-90.0f32).to_radians())
                * Mat4::from_translation(Vec3::new(-0.5, -0.5, 0.5))
                * Mat4::from_rotation_y(90.0f32.to_radians());
            self.append_block_model(
                mesh,
                "minecraft:tnt",
                pos,
                pose,
                if white { 4.0 } else { 1.0 },
                packs,
                atlas,
                sky_light,
            )?;
        }
        Ok(())
    }

    /// `FallingBlockRenderer`: each falling block's model, shifted so the
    /// block is centred on the entity's feet. Items stand in for block
    /// models (`id` is the block's item).
    pub fn append_falling_blocks(
        &mut self,
        mesh: &mut ChunkMesh,
        blocks: &[(Vec3, String)],
        packs: &PackStack,
        atlas: &Atlas,
        sky_light: &SkyLight,
    ) -> Result<()> {
        for (pos, id) in blocks {
            let pose = Mat4::from_translation(*pos + Vec3::new(-0.5, 0.0, -0.5));
            self.append_block_model(mesh, id, *pos, pose, 1.0, packs, atlas, sky_light)?;
        }
        Ok(())
    }

    /// Items mobs hold (`ItemInHandLayer`, `CrossedArmsItemLayer`): each
    /// under its holder's pose, then its display transform, tinted as its
    /// item definition says, lit at its point.
    pub fn append_held_items(
        &mut self,
        mesh: &mut ChunkMesh,
        items: &[HeldItem],
        packs: &PackStack,
        atlas: &Atlas,
        sky_light: &SkyLight,
    ) -> Result<()> {
        for item in items {
            let Some((display, mut tints)) = self.model(&item.id, packs)?.map(|visual| {
                (
                    match item.display {
                        HeldDisplay::RightHand => visual.right_hand,
                        HeldDisplay::LeftHand => visual.left_hand,
                        HeldDisplay::Ground => visual.ground,
                    },
                    visual.tints.clone(),
                )
            }) else {
                continue;
            };
            if let Some(color) = item.first_tint {
                if tints.is_empty() {
                    tints.push(color);
                } else {
                    tints[0] = color;
                }
            }
            self.append_model(
                mesh,
                &item.id,
                item.light,
                item.pose * display,
                1.0,
                Some(&tints),
                packs,
                atlas,
                sky_light,
            )?;
        }
        Ok(())
    }

    /// Block models under whole poses (an enderman's carried block), each lit
    /// at its point.
    pub fn append_posed_blocks(
        &mut self,
        mesh: &mut ChunkMesh,
        blocks: &[(Mat4, Vec3, String)],
        packs: &PackStack,
        atlas: &Atlas,
        sky_light: &SkyLight,
    ) -> Result<()> {
        for (pose, light, id) in blocks {
            self.append_block_model(mesh, id, *light, *pose, 1.0, packs, atlas, sky_light)?;
        }
        Ok(())
    }

    /// Complete hand pose supplied by the host, with PotionContents tint.
    #[allow(clippy::too_many_arguments)]
    pub fn append_posed_item(&mut self,mesh:&mut ChunkMesh,id:&str,pos:Vec3,pose:Mat4,tint:Option<[f32;3]>,packs:&PackStack,atlas:&Atlas,light:&SkyLight)->Result<()> {
        let colors=tint.map(|c|[c]);
        self.append_model(mesh,id,pos,pose,1.0,colors.as_ref().map(|c|c.as_slice()),packs,atlas,light)
    }
    /// One item model placed with a pose, lit at `pos`, untinted.
    #[allow(clippy::too_many_arguments)]
    fn append_block_model(
        &mut self,
        mesh: &mut ChunkMesh,
        id: &str,
        pos: Vec3,
        pose: Mat4,
        brightness: f32,
        packs: &PackStack,
        atlas: &Atlas,
        sky_light: &SkyLight,
    ) -> Result<()> {
        self.append_model(
            mesh, id, pos, pose, brightness, None, packs, atlas, sky_light,
        )
    }

    /// One item model placed with a pose, lit at `pos`, its faces with a
    /// tint index coloured from `tints` when given.
    #[allow(clippy::too_many_arguments)]
    fn append_model(
        &mut self,
        mesh: &mut ChunkMesh,
        id: &str,
        pos: Vec3,
        pose: Mat4,
        brightness: f32,
        tints: Option<&[[f32; 3]]>,
        packs: &PackStack,
        atlas: &Atlas,
        sky_light: &SkyLight,
    ) -> Result<()> {
        let Some(visual) = self.model(id, packs)? else {
            return Ok(());
        };
        {
            let white = brightness > 1.0;
            let light_pos = (
                pos.x.floor() as i32,
                pos.y.floor() as i32,
                pos.z.floor() as i32,
            );
            let sky = sky_light.get(light_pos) as f32;
            let block = sky_light.get_block(light_pos) as f32;
            let normal_pose = pose.inverse().transpose();
            for element in &visual.model.elements {
                for face in &element.faces {
                    if !atlas.contains(&face.texture) {
                        continue;
                    }
                    let corners = model_face_corners(element, face)?;
                    let [u0, v0, u1, v1] = atlas.region(&face.texture);
                    let uv = face.vertex_uvs().map(|[u, v]| [u0 + (u1 - u0) * u, v0 + (v1 - v0) * v]);
                    let normal = normal_pose
                        .transform_vector3(triangle_normal(&corners))
                        .normalize_or_zero();
                    let shade = level_item_shade(normal) * if white { 4.0 } else { 1.0 };
                    let tint = tints
                        .zip(face.tint_index)
                        .and_then(|(tints, index)| tints.get(index))
                        .copied()
                        .unwrap_or([1.0; 3]);
                    let start = mesh.vertices.len() as u32;
                    for (corner, uv) in corners.into_iter().zip(uv) {
                        mesh.vertices.push(Vertex {
                            position: pose.transform_point3(Vec3::from_array(corner)).to_array(),
                            uv,
                            color: [tint[0] * shade, tint[1] * shade, tint[2] * shade, 1.0],
                            sky_light: sky,
                            block_light: block,
                        });
                    }
                    mesh.indices.extend_from_slice(&[
                        start,
                        start + 1,
                        start + 2,
                        start,
                        start + 2,
                        start + 3,
                    ]);
                    mesh.faces += 1;
                }
            }
        }
        Ok(())
    }
}

/// Which display transform a held item takes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HeldDisplay {
    /// `THIRD_PERSON_RIGHT_HAND` (`ItemInHandLayer`).
    RightHand,
    /// `THIRD_PERSON_LEFT_HAND`, with ItemTransform.apply(left=true).
    LeftHand,
    /// `GROUND` (`CrossedArmsItemLayer`, a witch's potion).
    Ground,
}

/// An item a mob holds: its holder's pose, where it is lit, the item, its
/// display and a colour for its first tint (a potion's).
#[derive(Clone, Debug)]
pub struct HeldItem {
    pub pose: Mat4,
    pub light: Vec3,
    pub id: String,
    pub display: HeldDisplay,
    pub first_tint: Option<[f32; 3]>,
}

/// What `EntityRenderer.extractShadow` reads of one entity.
#[derive(Clone, Copy, Debug)]
pub struct ShadowCaster {
    /// The render state's position: the interpolated feet.
    pub position: glam::DVec3,
    /// `distanceToCameraSq`: from the entity's current position.
    pub distance_squared: f64,
    /// `getShadowRadius`, before the 32-block cap.
    pub radius: f32,
    /// `getShadowStrength`.
    pub strength: f32,
}

/// Dropped items' shadows (`ItemEntityRenderer`: radius 0.15, strength
/// 0.75).
pub fn item_shadow_casters(
    items: &WorldItems,
    camera_position: glam::DVec3,
    partial_tick: f32,
) -> Vec<ShadowCaster> {
    items
        .entities
        .iter()
        .map(|entity| ShadowCaster {
            position: entity
                .previous_position
                .lerp(entity.position, f64::from(partial_tick.clamp(0.0, 1.0))),
            distance_squared: entity.position.distance_squared(camera_position),
            radius: 0.15,
            strength: 0.75,
        })
        .collect()
}

/// `EntityRenderer.extractShadow`/`extractShadowPiece` and
/// `ShadowFeatureRenderer`: each block column under the entity within its
/// radius, down to a depth its strength allows, gets the shadow texture
/// on top of a full-collision, visible block below a spot lit above 3,
/// fading with depth and darkening with that light (`Lightmap.
/// getBrightness`). `raw_brightness` is `getMaxLocalRawBrightness`; the
/// texture clamps outside the circle, so each quad is cut to the radius.
pub fn entity_shadows<S: Scene>(
    casters: &[ShadowCaster],
    scene: &S,
    atlas: &Atlas,
    raw_brightness: &dyn Fn(BlockPos) -> u8,
    ambient_light: f32,
) -> Result<ChunkMesh> {
    let mut mesh = ChunkMesh::default();
    let sprite = ResourceId::parse("minecraft:misc/shadow")?;
    if !atlas.contains(&sprite) {
        return Ok(mesh);
    }
    let [u0, v0, u1, v1] = atlas.region(&sprite);
    for caster in casters {
        let radius = caster.radius.min(32.0);
        if radius <= 0.0 {
            continue;
        }
        let pow = ((1.0 - caster.distance_squared / 256.0) * f64::from(caster.strength)) as f32;
        if pow <= 0.0 {
            continue;
        }
        let state = caster.position;
        let r = f64::from(radius);
        let depth = (pow / 0.5 - 1.0).min(radius);
        let (x0, x1) = ((state.x - r).floor() as i32, (state.x + r).floor() as i32);
        let (z0, z1) = ((state.z - r).floor() as i32, (state.z + r).floor() as i32);
        let (y0, y1) = (
            (state.y - f64::from(depth)).floor() as i32,
            state.y.floor() as i32,
        );
        for z in z0..=z1 {
            for x in x0..=x1 {
                for y in y0..=y1 {
                    let power_at_depth = pow - (state.y - f64::from(y)) as f32 * 0.5;
                    let below = (x, y - 1, z);
                    // RenderShape.INVISIBLE: air and barriers.
                    if scene.block(below).is_none_or(|b| {
                        matches!(
                            b.id.path.as_str(),
                            "air"
                                | "cave_air"
                                | "void_air"
                                | "barrier"
                                | "structure_void"
                                | "light"
                                | "moving_piston"
                        )
                    }) {
                        continue;
                    }
                    let brightness = raw_brightness((x, y, z));
                    if brightness <= 3 || !scene.full_collision_at(below) {
                        continue;
                    }
                    let v = f32::from(brightness) / 15.0;
                    let curved = v / (4.0 - 3.0 * v);
                    let light = ambient_light * (1.0 - curved) + curved;
                    let alpha = (power_at_depth * 0.5 * light).clamp(0.0, 1.0);
                    // The piece's corners relative to the entity, as floats.
                    let (rx, ry, rz) = (
                        (f64::from(x) - state.x) as f32,
                        (f64::from(y) - state.y) as f32,
                        (f64::from(z) - state.z) as f32,
                    );
                    let (x_min, x_max) = (rx.max(-radius), (rx + 1.0).min(radius));
                    let (z_min, z_max) = (rz.max(-radius), (rz + 1.0).min(radius));
                    if x_min >= x_max || z_min >= z_max {
                        continue;
                    }
                    let origin = state.as_vec3();
                    let start = mesh.vertices.len() as u32;
                    for (px, pz) in [
                        (x_min, z_min),
                        (x_min, z_max),
                        (x_max, z_max),
                        (x_max, z_min),
                    ] {
                        let su = -px / 2.0 / radius + 0.5;
                        let sv = -pz / 2.0 / radius + 0.5;
                        mesh.vertices.push(Vertex {
                            // Lifted a thousandth off the block top so it
                            // never loses the depth test to it.
                            position: (origin + Vec3::new(px, ry + 0.001, pz)).to_array(),
                            uv: [u0 + (u1 - u0) * su, v0 + (v1 - v0) * sv],
                            color: [1.0, 1.0, 1.0, alpha],
                            sky_light: 15.0,
                            block_light: 0.0,
                        });
                    }
                    mesh.indices.extend_from_slice(&[
                        start,
                        start + 1,
                        start + 2,
                        start,
                        start + 2,
                        start + 3,
                    ]);
                    mesh.faces += 1;
                }
            }
        }
    }
    Ok(mesh)
}
fn model_key(block: &Block) -> String {
    format!("{}{:?}", block.id.key(), block.properties)
}
/// BlockBehaviour.getSeed -> Mth.getSeed -> SingleThreadedRandomSource.nextInt.
/// WeightedVariants consumes the first draw for the selected model.
fn variant_index(variants: &[(ResolvedModel, u32)], (x, y, z): BlockPos) -> usize {
    if variants.len() == 1 {
        return 0;
    }
    let total: u32 = variants.iter().map(|(_, weight)| *weight).sum();
    if total == 0 {
        return 0;
    }
    let seed = x.wrapping_mul(3_129_871) as i64 ^ (z as i64).wrapping_mul(116_129_781) ^ y as i64;
    let seed = seed
        .wrapping_mul(seed)
        .wrapping_mul(42_317_861)
        .wrapping_add(seed.wrapping_mul(11))
        >> 16;
    let mut random = ((seed as u64) ^ 0x5deece66d) & ((1u64 << 48) - 1);
    let mut next = || {
        random = random.wrapping_mul(0x5deece66d).wrapping_add(11) & ((1u64 << 48) - 1);
        (random >> 17) as u32
    };
    let mut selection = if total.is_power_of_two() {
        ((total as u64 * next() as u64) >> 31) as u32
    } else {
        loop {
            let bits = next();
            let value = bits % total;
            if (bits as i32)
                .wrapping_sub(value as i32)
                .wrapping_add(total as i32 - 1)
                >= 0
            {
                break value;
            }
        }
    };
    for (index, (_, weight)) in variants.iter().enumerate() {
        if selection < *weight {
            return index;
        }
        selection -= *weight;
    }
    variants.len() - 1
}
/// A face's corners as `FaceBakery.bakeVertex` places them: the element's
/// own rotation about its origin, then the blockstate's.
fn element_corners(element: &crate::model::Element, face: &str) -> Result<[[f32; 3]; 4]> {
    Ok(transform_element_corners(element, corners(face, element.from, element.to)?))
}

fn model_face_corners(element: &crate::model::Element, face: &crate::model::Face) -> Result<[[f32; 3]; 4]> {
    match face.triangle {
        Some(triangle) => Ok(transform_element_corners(element, triangle.corners())),
        None => element_corners(element, &face.direction),
    }
}

fn transform_element_corners(element: &crate::model::Element, corners: [[f32; 3]; 4]) -> [[f32; 3]; 4] {
    corners.map(|corner| {
        rotate_y(
            rotate_x(
                element.rotation.map_or(corner, |r| r.apply(corner)),
                element.rotation_x,
            ),
            element.rotation_y,
        )
    })
}

fn model_face_direction(element: &crate::model::Element, face: &crate::model::Face, corners: &[[f32; 3]; 4]) -> Result<&'static str> {
    if face.triangle.is_some() {
        Ok(geometry_direction(corners))
    } else {
        quad_direction(element, &face.direction, corners)
    }
}

fn triangle_normal(corners: &[[f32; 3]; 4]) -> Vec3 {
    let [a, b, c, _] = corners.map(Vec3::from_array);
    (b - a).cross(c - a).normalize_or_zero()
}

/// The baked quad's direction (`FaceBakery.calculateFacing`): the face's
/// own, turned with the blockstate, or for a rotated element the direction
/// closest to its normal (`GeometryUtils.normal` of the first three
/// corners; the first of equals in `Direction` order).
fn quad_direction(
    element: &crate::model::Element,
    face: &str,
    corners: &[[f32; 3]; 4],
) -> Result<&'static str> {
    if element.rotation.is_none() {
        let dir = element_direction(element, face)?;
        return Ok(["down", "up", "north", "south", "west", "east"]
            .into_iter()
            .find(|d| *d == dir)
            .unwrap_or("up"));
    }
    Ok(geometry_direction(corners))
}

fn geometry_direction(corners: &[[f32; 3]; 4]) -> &'static str {
    let [v0, v1, v2, _] = *corners;
    let mut n = [
        (v1[1] - v0[1]) * (v2[2] - v0[2]) - (v1[2] - v0[2]) * (v2[1] - v0[1]),
        (v1[2] - v0[2]) * (v2[0] - v0[0]) - (v1[0] - v0[0]) * (v2[2] - v0[2]),
        (v1[0] - v0[0]) * (v2[1] - v0[1]) - (v1[1] - v0[1]) * (v2[0] - v0[0]),
    ];
    let length = (n[0] * n[0] + n[1] * n[1] + n[2] * n[2]).sqrt();
    if !(length > 0.0) || !length.is_finite() {
        return "up";
    }
    for v in &mut n {
        *v /= length;
    }
    let mut best = "up";
    let mut closest = 0.0f32;
    for (name, [x, y, z]) in [
        ("down", [0.0, -1.0, 0.0]),
        ("up", [0.0, 1.0, 0.0]),
        ("north", [0.0, 0.0, -1.0]),
        ("south", [0.0, 0.0, 1.0]),
        ("west", [-1.0, 0.0, 0.0]),
        ("east", [1.0, 0.0, 0.0]),
    ] {
        let product = n[0] * x + n[1] * y + n[2] * z;
        if product >= 0.0 && product > closest {
            closest = product;
            best = name;
        }
    }
    best
}

/// `BlockModelLighter.prepareQuadShape`'s `faceCubic`: the quad lies flat
/// on the block's side it faces (a full collision block's quads count as
/// on their side too; not modelled).
fn face_cubic(dir: &str, corners: &[[f32; 3]; 4]) -> bool {
    let axis = match dir {
        "down" | "up" => 1,
        "north" | "south" => 2,
        _ => 0,
    };
    let (min, max) = corners.iter().fold((32.0f32, -32.0f32), |(lo, hi), c| {
        (lo.min(c[axis]), hi.max(c[axis]))
    });
    min == max
        && if matches!(dir, "down" | "north" | "west") {
            min < 1.0e-4
        } else {
            max > 0.9999
        }
}

fn rotate_y([x, y, z]: [f32; 3], degrees: u16) -> [f32; 3] {
    match degrees {
        90 => [1.0 - z, y, x],
        180 => [1.0 - x, y, 1.0 - z],
        270 => [z, y, 1.0 - x],
        _ => [x, y, z],
    }
}
/// `BlockModelRotation` X: `rotateX(-x)`, so 90 turns up to north.
fn rotate_x([x, y, z]: [f32; 3], degrees: u16) -> [f32; 3] {
    match degrees {
        90 => [x, z, 1.0 - y],
        180 => [x, 1.0 - y, 1.0 - z],
        270 => [x, 1.0 - z, y],
        _ => [x, y, z],
    }
}
fn rotated_x_direction(dir: &str, degrees: u16) -> Result<&str> {
    let i = match dir {
        "up" => 0,
        "north" => 1,
        "down" => 2,
        "south" => 3,
        "east" | "west" => return Ok(dir),
        _ => return Err(anyhow!("unsupported face direction {dir}")),
    };
    Ok(["up", "north", "down", "south"][(i + degrees as usize / 90) % 4])
}
/// A model face direction turned by the blockstate: X first, then Y.
fn element_direction<'a>(element: &crate::model::Element, dir: &'a str) -> Result<&'a str> {
    rotated_direction(
        rotated_x_direction(dir, element.rotation_x)?,
        element.rotation_y,
    )
}
fn rotated_direction(dir: &str, degrees: u16) -> Result<&str> {
    let i = match dir {
        "north" => 0,
        "east" => 1,
        "south" => 2,
        "west" => 3,
        "up" | "down" => return Ok(dir),
        _ => return Err(anyhow!("unsupported face direction {dir}")),
    };
    Ok(["north", "east", "south", "west"][(i + degrees as usize / 90) % 4])
}
pub fn make_atlas(packs: &PackStack, mut textures: Vec<ResourceId>) -> Result<Atlas> {
    textures.insert(0, ResourceId::parse("minecraft:missingno")?);
    let tile = 64u32;
    // Entity skins keep their own resolution, as vanilla binds each one on
    // its own: one larger than a cell spans several cells, a texel to a
    // pixel. Everything else fills one cell.
    let mut images = Vec::with_capacity(textures.len());
    let mut spans = Vec::with_capacity(textures.len());
    for id in &textures {
        let bytes = if id.path == "missingno" {
            None
        } else {
            packs.texture(id)?
        };
        let span = match &bytes {
            Some(_) if world_texture(id).is_some() => (2, 2),
            Some(bytes) if id.path.starts_with("entity/") && packs.animation(id)?.is_none() => {
                let (width, height) = image::ImageReader::with_format(
                    std::io::Cursor::new(bytes),
                    image::ImageFormat::Png,
                )
                .into_dimensions()?;
                (width.div_ceil(tile).max(1), height.div_ceil(tile).max(1))
            }
            _ => (1, 1),
        };
        images.push(bytes);
        spans.push(span);
    }
    let cells: u32 = spans.iter().map(|&(w, h)| w * h).sum();
    let mut side = (cells as f32).sqrt().ceil() as u32;
    let positions = loop {
        if let Some(positions) = place_cells(&spans, side) {
            break positions;
        }
        side += 1;
    };
    let mut pixels = RgbaImage::new(side * tile, side * tile);
    let mut mipmaps = (1..=4)
        .map(|level| RgbaImage::new(side * (tile >> level), side * (tile >> level)))
        .collect::<Vec<_>>();
    let mut slots = HashMap::new();
    let mut missing = Vec::new();
    let mut animated = Vec::new();
    let mut animated_tiles = Vec::new();
    for (i, id) in textures.into_iter().enumerate() {
        let (col, row) = positions[i];
        let (span_w, span_h) = spans[i];
        let image = images[i].take();
        let mut dark_cutout = false;
        let mut native_leaf_mips = None;
        let tile_image = if let Some(bytes) = image {
            let decoded =
                image::load_from_memory_with_format(&bytes, image::ImageFormat::Png)?.to_rgba8();
            let frame = if let Some(meta) = packs.animation(&id)? {
                dark_cutout = meta
                    .get("texture")
                    .and_then(|texture| texture.get("mipmap_strategy"))
                    .and_then(|value| value.as_str())
                    == Some("dark_cutout");
                if let Some(animation) = meta.get("animation") {
                    animated.push(id.clone());
                    if let Some(tile) = animated_tile(&id, &decoded, animation, (col * tile, row * tile))
                    {
                        animated_tiles.push(tile);
                    }
                    let index = animation
                        .get("frames")
                        .and_then(|f| f.as_array())
                        .and_then(|a| a.first())
                        .and_then(|v| {
                            v.as_u64()
                                .or_else(|| v.get("index").and_then(|x| x.as_u64()))
                        })
                        .unwrap_or(0) as u32;
                    let size = decoded.width().min(decoded.height());
                    image::imageops::crop_imm(
                        &decoded,
                        0,
                        (index * size).min(decoded.height() - size),
                        size,
                        size,
                    )
                    .to_image()
                } else {
                    decoded
                }
            } else {
                decoded
            };
            if dark_cutout && frame.width() == 16 && frame.height() == 16 {
                native_leaf_mips = Some(texture_mips::dark_cutout(frame, 5));
                image::imageops::resize(
                    &native_leaf_mips.as_ref().unwrap()[0],
                    tile,
                    tile,
                    FilterType::Nearest,
                )
            } else {
                image::imageops::resize(&frame, span_w * tile, span_h * tile, FilterType::Nearest)
            }
        } else {
            if id.path != "missingno" {
                missing.push(id.clone());
            }
            let mut checker = RgbaImage::new(tile, tile);
            for y in 0..tile {
                for x in 0..tile {
                    checker.put_pixel(
                        x,
                        y,
                        if (x / 8 + y / 8) % 2 == 0 {
                            image::Rgba([255, 0, 255, 255])
                        } else {
                            image::Rgba([0, 0, 0, 255])
                        },
                    );
                }
            }
            checker
        };
        image::imageops::replace(
            &mut pixels,
            &tile_image,
            (col * tile) as i64,
            (row * tile) as i64,
        );
        let tile_mips = if let Some(native) = native_leaf_mips {
            // Keep the 16-pixel foliage cutout chain intact while the 64-pixel
            // atlas also preserves the native 64-pixel chest entity texture.
            let mut levels = vec![tile_image];
            levels.push(image::imageops::resize(
                &native[0],
                32,
                32,
                FilterType::Nearest,
            ));
            levels.extend(native.into_iter().take(3));
            levels
        } else if dark_cutout {
            texture_mips::dark_cutout(tile_image, 5)
        } else {
            (0..5)
                .map(|level| {
                    image::imageops::resize(
                        &tile_image,
                        (span_w * tile) >> level,
                        (span_h * tile) >> level,
                        FilterType::Triangle,
                    )
                })
                .collect::<Vec<_>>()
        };
        // The dark-cutout source also alters transparent RGB at level zero.
        if dark_cutout {
            image::imageops::replace(
                &mut pixels,
                &tile_mips[0],
                (col * tile) as i64,
                (row * tile) as i64,
            );
        }
        for (level, mip) in mipmaps.iter_mut().enumerate() {
            let mip_tile = tile >> (level + 1);
            image::imageops::replace(
                mip,
                &tile_mips[level + 1],
                (col * mip_tile) as i64,
                (row * mip_tile) as i64,
            );
        }
        let width = (side * tile) as f32;
        slots.insert(
            id,
            [
                (col * tile) as f32 / width + 0.5 / width,
                (row * tile) as f32 / width + 0.5 / width,
                ((col + span_w) * tile) as f32 / width - 0.5 / width,
                ((row + span_h) * tile) as f32 / width - 0.5 / width,
            ],
        );
    }
    Ok(Atlas {
        pixels,
        mipmaps,
        slots,
        missing,
        animated,
        animated_tiles,
    })
}
/// Grid cells for each texture's span (columns, rows) in a `side`-cell
/// square: the multi-cell ones first, then the rest in order, each at the
/// first free spot; `None` when they do not fit.
fn place_cells(spans: &[(u32, u32)], side: u32) -> Option<Vec<(u32, u32)>> {
    let mut used = vec![false; (side * side) as usize];
    let mut positions = vec![(0, 0); spans.len()];
    let mut order: Vec<usize> = (0..spans.len()).collect();
    order.sort_by_key(|&i| std::cmp::Reverse(spans[i].0 * spans[i].1));
    let mut first_free = 0;
    for i in order {
        let (w, h) = spans[i];
        let fits = |col: u32, row: u32, used: &[bool]| {
            col + w <= side
                && row + h <= side
                && (row..row + h).all(|r| (col..col + w).all(|c| !used[(r * side + c) as usize]))
        };
        let start = if (w, h) == (1, 1) { first_free } else { 0 };
        let cell = (start..side * side).find(|&cell| fits(cell % side, cell / side, &used))?;
        let (col, row) = (cell % side, cell / side);
        for r in row..row + h {
            for c in col..col + w {
                used[(r * side + c) as usize] = true;
            }
        }
        if (w, h) == (1, 1) {
            first_free = cell + 1;
        }
        positions[i] = (col, row);
    }
    Some(positions)
}
fn animated_tile(
    id: &ResourceId,
    sheet: &RgbaImage,
    metadata: &serde_json::Value,
    origin: (u32, u32),
) -> Option<AnimatedTile> {
    let square = sheet.width().min(sheet.height());
    let width = metadata
        .get("width")
        .and_then(|v| v.as_u64())
        .unwrap_or(square as u64) as u32;
    let height = metadata
        .get("height")
        .and_then(|v| v.as_u64())
        .unwrap_or(square as u64) as u32;
    if width == 0 || height == 0 || sheet.width() % width != 0 || sheet.height() % height != 0 {
        return None;
    }
    let columns = sheet.width() / width;
    let count = columns * (sheet.height() / height);
    let frametime = metadata
        .get("frametime")
        .and_then(|v| v.as_u64())
        .unwrap_or(1) as u32;
    let entries = metadata.get("frames").and_then(|v| v.as_array());
    let sequence = if let Some(entries) = entries {
        entries
            .iter()
            .filter_map(|entry| {
                let index = entry.as_u64().or_else(|| entry.get("index")?.as_u64())? as usize;
                let time = entry
                    .get("time")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(frametime as u64) as u32;
                (index < count as usize && time > 0).then_some((index, time))
            })
            .collect::<Vec<_>>()
    } else {
        (0..count as usize)
            .map(|index| (index, frametime))
            .collect()
    };
    if sequence.len() <= 1 || frametime == 0 {
        return None;
    }
    let frames = (0..count)
        .map(|index| {
            let frame = image::imageops::crop_imm(
                sheet,
                index % columns * width,
                index / columns * height,
                width,
                height,
            )
            .to_image();
            let tile = image::imageops::resize(&frame, 64, 64, FilterType::Nearest);
            (0..5)
                .map(|level| {
                    image::imageops::resize(&tile, 64 >> level, 64 >> level, FilterType::Triangle)
                })
                .collect::<Vec<_>>()
        })
        .collect();
    Some(AnimatedTile {
        id: id.clone(),
        origin,
        frames,
        sequence,
        interpolate: metadata.get("interpolate").and_then(|v| v.as_bool()) == Some(true),
    })
}
fn mesh_chunk<S: Scene>(
    scene: &S,
    chunk: ChunkPos,
    models: &HashMap<String, Vec<(ResolvedModel, u32)>>,
    atlas: &Atlas,
    tint_source: &BiomeTint,
    light: &SkyLight,
) -> Result<ChunkMesh> {
    let mut mesh = ChunkMesh::default();
    let mut transparent_indices = Vec::new();
    let full_cube_models: HashMap<_, _> = models
        .iter()
        .map(|(key, variants)| (key.clone(), all_full_cubes(variants)))
        .collect();
    for x in chunk.0 * 16..chunk.0 * 16 + 16 {
        for z in chunk.1 * 16..chunk.1 * 16 + 16 {
            for y in scene.vertical_range() {
                let Some(block) = scene.block((x, y, z)) else {
                    continue;
                };
                append_block(
                    scene,
                    (x, y, z),
                    block,
                    || {
                        models
                            .get(&model_key(block))
                            .map(Vec::as_slice)
                            .ok_or_else(|| anyhow!("uncached model"))
                    },
                    |neighbor| {
                        scene.block(neighbor).is_some_and(|b| {
                            hides_shared_face(block, b)
                                || (b.is_opaque()
                                    && full_cube_models
                                        .get(&model_key(b))
                                        .copied()
                                        .unwrap_or(false))
                        })
                    },
                    atlas,
                    tint_source,
                    light,
                    &mut mesh,
                    &mut transparent_indices,
                )?;
            }
        }
    }
    Ok(finish_mesh(mesh, transparent_indices))
}
/// Whether every variant has a full-cube element, so an opaque block with
/// these models hides the faces of its neighbors.
pub(crate) fn all_full_cubes(variants: &[(ResolvedModel, u32)]) -> bool {
    variants.iter().all(|(model, _)| {
        model
            .elements
            .iter()
            .any(|element| element.from == [0.0; 3] && element.to == [1.0; 3] && element.faces.iter().all(|face| face.triangle.is_none()))
    })
}
/// Moves the translucent faces after the opaque ones.
/// One model face baked for a block state's variant (vanilla `BakedQuad`):
/// everything [`append_block`] works out per face that does not depend on
/// where the block stands, so a section build only culls, shades, lights
/// and emits it.
#[derive(Clone, Debug)]
pub(crate) struct BakedQuad {
    /// The neighbour offset whose block may hide it, for cullable faces.
    cull: Option<(i32, i32, i32)>,
    /// The neighbour its light comes from.
    light_offset: (i32, i32, i32),
    /// Lit by its block's brightest neighbour instead (a facade part's
    /// face inside its block, `jak_facades`: its neighbour that way may be
    /// the wall it stands against).
    brightest: bool,
    /// Its texture laid over the world instead of over its block
    /// ([`world_texture`]): the texture's region, how many blocks a tile of
    /// it covers, and the plane it is laid on.
    world: Option<([f32; 4], f32, u8)>,
    /// For a full cube's face, the two axes its corner shade samples span.
    ao_axes: Option<[usize; 2]>,
    corners: [[f32; 3]; 4],
    uv: [[f32; 2]; 4],
    shade: f32,
    tinted: Option<usize>,
    transparent: bool,
}

/// How a block's faces are tinted (`block_face_tint`, decided per state).
#[derive(Clone, Copy, Debug)]
pub(crate) enum TintKind {
    Water, WaterContainer, RedstoneWire([f32;3]), Constant([f32;3]),
    Grass, GrassBelow, Flowers, Foliage, DryFoliage, Other,
}
impl TintKind {
    pub(crate) fn of(block:&Block)->Self {
        if block.id.namespace=="jak"{return Self::Grass;}
        let rgb=|value:u32|[((value>>16)&255)as f32/255.0,((value>>8)&255)as f32/255.0,(value&255)as f32/255.0];
        match block.id.path.as_str(){
            "water"|"bubble_column"=>Self::Water,
            "water_cauldron"=>Self::WaterContainer,
            "redstone_wire"=>Self::RedstoneWire(rgb(redstone_wire_color_argb(block.properties.get("power").and_then(|v|v.parse().ok()).unwrap_or(0)))),
            "spruce_leaves"=>Self::Constant(rgb(0x619961)),
            "birch_leaves"=>Self::Constant(rgb(0x80a755)),
            "oak_leaves"|"jungle_leaves"|"acacia_leaves"|"dark_oak_leaves"|"mangrove_leaves"|"vine"=>Self::Foliage,
            "leaf_litter"=>Self::DryFoliage,
            "grass_block"|"short_grass"|"fern"|"potted_fern"|"bush"|"sugar_cane"=>Self::Grass,
            "tall_grass"|"large_fern" if block.properties.get("half").is_some_and(|v|v=="upper")=>Self::GrassBelow,
            "tall_grass"|"large_fern"=>Self::Grass,
            "pink_petals"|"wildflowers"=>Self::Flowers,
            "attached_melon_stem"|"attached_pumpkin_stem"=>Self::Constant(rgb(0xe0c71c)),
            "melon_stem"|"pumpkin_stem"=>{
                let a=block.properties.get("age").and_then(|v|v.parse::<u32>().ok()).unwrap_or(0).min(7);
                Self::Constant(rgb((a*32)<<16|(255-a*8)<<8|a*4))
            }
            "lily_pad"=>Self::Constant(rgb(0x208030)),
            _=>Self::Other,
        }
    }
    fn sample_pos(self,at:BlockPos)->BlockPos {if matches!(self,Self::GrassBelow){(at.0,at.1-1,at.2)}else{at}}
    fn tint(self,index:Option<usize>,biome:BiomeSample,source:&BiomeTint)->[f32;3]{
        // Fluids use a separate native mesh, whose faces have no tint index.
        if matches!(self,Self::Water){return biome.water_color.map(|v|v as f32/255.0);}
        let Some(index)=index else{return [1.0;3];};
        if matches!(self,Self::Flowers){return if index==1{source.grass(biome)}else{[1.0;3]};}
        if index!=0{return [1.0;3];}
        match self {
            Self::WaterContainer=>biome.water_color.map(|v|v as f32/255.0),
            Self::RedstoneWire(c)|Self::Constant(c)=>c,
            Self::Grass|Self::GrassBelow=>source.grass(biome),
            Self::Foliage=>source.foliage(biome),
            Self::DryFoliage=>source.dry_foliage(biome),
            _=>[1.0;3],
        }
    }
}

/// Bakes a resolved model's faces for `block`, as [`append_block`] would
/// place them.
pub(crate) fn bake_quads(
    block: &Block,
    model: &ResolvedModel,
    atlas: &Atlas,
) -> Result<Vec<BakedQuad>> {
    let mut quads = Vec::new();
    for element in &model.elements {
        for face in &element.faces {
            let cull_dir = element_direction(
                element,
                face.cullface.as_deref().unwrap_or(face.direction.as_str()),
            )?;
            let cull_delta = direction(cull_dir)?;
            let corners = model_face_corners(element, face)?;
            let dir = model_face_direction(element, face, &corners)?;
            let delta = direction(dir)?;
            let [u0, v0, u1, v1] = atlas.region(&face.texture);
            let uv = face.vertex_uvs().map(|[u, v]| [u0 + (u1 - u0) * u, v0 + (v1 - v0) * v]);
            let shade = match element.shade_direction_override.as_deref().unwrap_or(dir) {
                "up" => 1.0,
                "down" => 0.5,
                "north" | "south" => 0.8,
                _ => 0.6,
            };
            // A facade part's faces inside its (solid) block take the light
            // of its brightest neighbour (`jak_facades`: its rounded
            // corners, its dome, its buttresses' steps came out black lit by
            // the block itself or by the wall beside it).
            let brightest = !face.cull && crate::jak_facades::shaped(block);
            let world = if face.triangle.is_some() { None } else { world_texture(&face.texture).map(|tile| (atlas.region(&face.texture), tile, plane_of(dir))) };
            let light_offset = if face.cull {
                cull_delta
            } else if face_cubic(dir, &corners) {
                delta
            } else {
                (0, 0, 0)
            };
            let full_cube =
                face.triangle.is_none() && element.from == [0.0; 3] && element.to == [1.0; 3] && element.rotation.is_none();
            let ao_axes = full_cube.then(|| match dir {
                "up" | "down" => [0, 2],
                "north" | "south" => [0, 1],
                _ => [1, 2],
            });
            quads.push(BakedQuad {
                cull: face.cull.then_some(cull_delta),
                light_offset,
                brightest,
                world,
                ao_axes,
                corners,
                uv,
                shade,
                tinted: face.tint_index.or_else(||face.tint.then_some(0)),
                transparent: block.id.path == "water" || face.force_translucent,
            });
        }
    }
    Ok(quads)
}

/// [`append_block`] for a block whose faces are baked: the same vertices,
/// with `ao_occluder(pos)` standing for the block test in
/// [`ambient_vertex`] and `hidden_by` for the cull test.
#[allow(clippy::too_many_arguments)]
pub(crate) fn append_baked<S: Scene>(
    scene: &S,
    (x, y, z): BlockPos,
    quads: &[BakedQuad],
    tint_kind: TintKind,
    waterlogged: bool,
    hidden_by: impl Fn(BlockPos) -> bool,
    ao_occluder: impl Fn(BlockPos) -> bool,
    atlas: &Atlas,
    tint_source: &BiomeTint,
    light: &SkyLight,
    mesh: &mut ChunkMesh,
    transparent_indices: &mut Vec<u32>,
) -> Result<()> {
    let mut biome = None;
    for quad in quads {
        if quad
            .cull
            .is_some_and(|(dx, dy, dz)| hidden_by((x + dx, y + dy, z + dz)))
        {
            continue;
        }
        let biome = *biome.get_or_insert_with(|| scene.biome_at(tint_kind.sample_pos((x,y,z))));
        let tint = tint_kind.tint(quad.tinted, biome, tint_source);
        let start = mesh.vertices.len() as u32;
        let light_pos = if quad.brightest {
            brightest_around(light, (x, y, z))
        } else {
            (
                x + quad.light_offset.0,
                y + quad.light_offset.1,
                z + quad.light_offset.2,
            )
        };
        let block_light = light.get_block(light_pos) as f32;
        let laid = quad.world.map(|world| world_uvs(world, (x, y, z), &quad.corners));
        for (k, (corner, uv)) in quad.corners.iter().zip(quad.uv).enumerate() {
            let uv = laid.map_or(uv, |laid| laid[k]);
            let (ao, sky_light) = match quad.ao_axes {
                Some(axes) => ambient_corner(light, light_pos, axes, *corner, &ao_occluder),
                None => (1.0, light.get(light_pos) as f32),
            };
            mesh.vertices.push(Vertex {
                position: [
                    (x - mesh.origin[0]) as f32 + corner[0],
                    (y - mesh.origin[1]) as f32 + corner[1],
                    (z - mesh.origin[2]) as f32 + corner[2],
                ],
                uv,
                color: [
                    quad.shade * ao * tint[0],
                    quad.shade * ao * tint[1],
                    quad.shade * ao * tint[2],
                    1.0,
                ],
                sky_light,
                block_light,
            });
        }
        let indices = if quad.transparent {
            &mut *transparent_indices
        } else {
            &mut mesh.indices
        };
        indices.extend_from_slice(&[start, start + 1, start + 2, start, start + 2, start + 3]);
        mesh.faces += 1;
    }
    if waterlogged {
        append_fluid(
            scene,
            (x, y, z),
            atlas,
            tint_source,
            light,
            mesh,
            transparent_indices,
        )?;
    }
    Ok(())
}

/// [`ambient_vertex`] with its axes worked out and its block test given.
fn ambient_corner(
    light: &SkyLight,
    base: (i32, i32, i32),
    axes: [usize; 2],
    corner: [f32; 3],
    occluder: &impl Fn(BlockPos) -> bool,
) -> (f32, f32) {
    let mut side = [[0; 3]; 2];
    for (i, axis) in axes.into_iter().enumerate() {
        side[i][axis] = if corner[axis] < 0.5 { -1 } else { 1 };
    }
    let positions = [
        base,
        (
            base.0 + side[0][0],
            base.1 + side[0][1],
            base.2 + side[0][2],
        ),
        (
            base.0 + side[1][0],
            base.1 + side[1][1],
            base.2 + side[1][2],
        ),
        (
            base.0 + side[0][0] + side[1][0],
            base.1 + side[0][1] + side[1][1],
            base.2 + side[0][2] + side[1][2],
        ),
    ];
    let center_light = light.get(base);
    let shade = positions
        .iter()
        .map(|&p| if occluder(p) { 0.2 } else { 1.0 })
        .sum::<f32>()
        * 0.25;
    let sky = positions
        .iter()
        .map(|&p| {
            let level = light.get(p);
            if center_light > 2 && level == 0 {
                center_light as f32
            } else {
                level as f32
            }
        })
        .sum::<f32>()
        * 0.25;
    (shade, sky)
}

pub(crate) fn variant_for(variants: &[(ResolvedModel, u32)], pos: BlockPos) -> usize {
    variant_index(variants, pos)
}

pub(crate) fn finish_mesh(mut mesh: ChunkMesh, transparent_indices: Vec<u32>) -> ChunkMesh {
    if !transparent_indices.is_empty() {
        mesh.transparent_start = Some(mesh.indices.len() as u32);
        mesh.indices.extend(transparent_indices);
    }
    mesh
}
/// Appends one block's faces. `variants` supplies its resolved models when
/// the block is not a fluid; `hidden_by(neighbor)` is whether the block at a
/// neighboring position hides a cullable face toward it.
#[allow(clippy::too_many_arguments)]
pub(crate) fn append_block<'m, S: Scene>(
    scene: &S,
    (x, y, z): BlockPos,
    block: &Block,
    variants: impl FnOnce() -> Result<&'m [(ResolvedModel, u32)]>,
    hidden_by: impl Fn(BlockPos) -> bool,
    atlas: &Atlas,
    tint_source: &BiomeTint,
    light: &SkyLight,
    mesh: &mut ChunkMesh,
    transparent_indices: &mut Vec<u32>,
) -> Result<()> {
    if matches!(block.id.path.as_str(), "water" | "lava") {
        return append_fluid(
            scene,
            (x, y, z),
            atlas,
            tint_source,
            light,
            mesh,
            transparent_indices,
        );
    }
    let variants = variants()?;
    let model = &variants[variant_index(variants, (x, y, z))].0;
    let mut biome = None;
    for element in &model.elements {
        for face in &element.faces {
            // Culled quads go with their cullface (turned with the
            // blockstate), and take their light from that neighbour.
            let cull_dir = element_direction(
                element,
                face.cullface.as_deref().unwrap_or(face.direction.as_str()),
            )?;
            let cull_delta = direction(cull_dir)?;
            if face.cull && hidden_by((x + cull_delta.0, y + cull_delta.1, z + cull_delta.2)) {
                continue;
            }
            let corners = model_face_corners(element, face)?;
            let dir = model_face_direction(element, face, &corners)?;
            let delta = direction(dir)?;
            let [u0, v0, u1, v1] = if block.id.path == "chest" { atlas.region_exact(&face.texture) } else { atlas.region(&face.texture) };
            let uv = face.vertex_uvs().map(|[u, v]| [u0 + (u1 - u0) * u, v0 + (v1 - v0) * v]);
            let shade = match element.shade_direction_override.as_deref().unwrap_or(dir) {
                "up" => 1.0,
                "down" => 0.5,
                "north" | "south" => 0.8,
                _ => 0.6,
            };
            let biome = *biome.get_or_insert_with(|| scene.biome_at(TintKind::of(block).sample_pos((x,y,z))));
            let tint = block_face_tint(block, face.tint_index.or_else(||face.tint.then_some(0)), biome, tint_source);
            let alpha = if block.id.path == "water" { 0.72 } else { 1.0 };
            let start = mesh.vertices.len() as u32;
            // `ModelBlockRenderer.tesselateFlat` and `prepareQuadFlat`: a
            // culled quad is lit from its cullface's neighbour, another from
            // the neighbour it faces only when it lies on that side
            // (`faceCubic`), else from its own block.
            let light_pos = if face.cull {
                (x + cull_delta.0, y + cull_delta.1, z + cull_delta.2)
            } else if crate::jak_facades::shaped(block) {
                brightest_around(light, (x, y, z))
            } else if face_cubic(dir, &corners) {
                (x + delta.0, y + delta.1, z + delta.2)
            } else {
                (x, y, z)
            };
            let full_cube =
                face.triangle.is_none() && element.from == [0.0; 3] && element.to == [1.0; 3] && element.rotation.is_none();
            let uv = match face.triangle.is_none().then(|| world_texture(&face.texture)).flatten() {
                Some(tile) => world_uvs((atlas.region(&face.texture), tile, plane_of(dir)), (x, y, z), &corners),
                None => uv,
            };
            for (corner, uv) in corners.iter().zip(uv) {
                let (ao, sky_light) = if full_cube {
                    ambient_vertex(scene, light, light_pos, dir, *corner)
                } else {
                    (1.0, light.get(light_pos) as f32)
                };
                mesh.vertices.push(Vertex {
                    position: [
                        (x - mesh.origin[0]) as f32 + corner[0],
                        (y - mesh.origin[1]) as f32 + corner[1],
                        (z - mesh.origin[2]) as f32 + corner[2],
                    ],
                    uv,
                    color: [
                        shade * ao * tint[0],
                        shade * ao * tint[1],
                        shade * ao * tint[2],
                        alpha,
                    ],
                    sky_light,
                    block_light: light.get_block(light_pos) as f32,
                });
            }
            let indices = if block.id.path == "water" || face.force_translucent {
                &mut *transparent_indices
            } else {
                &mut mesh.indices
            };
            indices.extend_from_slice(&[start, start + 1, start + 2, start, start + 2, start + 3]);
            mesh.faces += 1;
        }
    }
    if crate::fluid::holds_water(block) {
        append_fluid(
            scene,
            (x, y, z),
            atlas,
            tint_source,
            light,
            mesh,
            transparent_indices,
        )?;
    }
    Ok(())
}
/// How many blocks a tile of a texture laid over the world covers, for
/// the mashup's textures named so (`jak:block/w4_…`: four; Adrii 24/10:
/// the Jak II village's plaster and paving must not show the block grid
/// as a texture repeating each block does).
pub(crate) fn world_texture(id: &ResourceId) -> Option<f32> {
    let rest = id.path.strip_prefix("block/w")?;
    let digit = rest.chars().next()?.to_digit(10)?;
    (rest[1..].starts_with('_') && id.namespace == "jak").then_some(digit as f32)
}

/// The plane a quad facing `dir` lies on: 0 the ground's (x, z), 1 a wall
/// facing north or south (x, y), 2 east or west (z, y).
fn plane_of(dir: &str) -> u8 {
    match dir {
        "up" | "down" => 0,
        "north" | "south" => 1,
        _ => 2,
    }
}

/// A world-laid quad's texture coordinates: each corner at its place in
/// the world, in the tile of the texture its middle is in (a quad stays
/// within one tile, so it never wraps).
fn world_uvs((region, tile, plane): ([f32; 4], f32, u8), (x, y, z): BlockPos, corners: &[[f32; 3]; 4]) -> [[f32; 2]; 4] {
    let at = |c: &[f32; 3]| -> (f32, f32) {
        let (wx, wy, wz) = (x as f32 + c[0], y as f32 + c[1], z as f32 + c[2]);
        match plane {
            0 => (wx, wz),
            1 => (wx, -wy),
            _ => (wz, -wy),
        }
    };
    let mid = corners.iter().map(at).fold((0.0, 0.0), |m, p| (m.0 + p.0 / 4.0, m.1 + p.1 / 4.0));
    let base = ((mid.0 / tile).floor() * tile, (mid.1 / tile).floor() * tile);
    let [u0, v0, u1, v1] = region;
    corners.map(|c| {
        let (a, b) = at(&c);
        let (s, t) = (((a - base.0) / tile).clamp(0.0, 1.0), ((b - base.1) / tile).clamp(0.0, 1.0));
        [u0 + (u1 - u0) * s, v0 + (v1 - v0) * t]
    })
}

/// The brightest of `pos`'s six neighbours (sky light, then block light).
fn brightest_around(light: &SkyLight, (x, y, z): BlockPos) -> BlockPos {
    [(0, 1, 0), (1, 0, 0), (-1, 0, 0), (0, 0, 1), (0, 0, -1), (0, -1, 0)]
        .into_iter()
        .map(|(dx, dy, dz)| (x + dx, y + dy, z + dz))
        .max_by_key(|&p| (light.get(p), light.get_block(p)))
        .unwrap()
}
/// Whether the block at `pos` hides the fluid face against it. Thin plants
/// count as solid for the fluid's flow but hide nothing: under a lily pad or
/// beside a vine the water surface still shows (15/10: it left holes in the
/// water, dark squares around lily pads in swamps at night).
fn hides_fluid<S: Scene>(scene: &S, pos: BlockPos) -> bool {
    scene.full_collision_at(pos) && !scene.block(pos).is_some_and(|block| see_through_plant(&block.id.path))
}
fn see_through_plant(path: &str) -> bool {
    matches!(
        path,
        "lily_pad"
            | "swamp_lily"
            | "swamp_moss"
            | "vine"
            | "cave_vines"
            | "cave_vines_plant"
            | "weeping_vines"
            | "weeping_vines_plant"
            | "twisting_vines"
            | "twisting_vines_plant"
            | "sugar_cane"
            | "cobweb"
            | "pink_petals"
            | "wildflowers"
            | "leaf_litter"
            | "barrio_hierbajos"
            | "sparg_matorral"
            | "spore_blossom"
            | "bush"
            | "firefly_bush"
            | "short_dry_grass"
            | "tall_dry_grass"
            | "dandelion"
            | "poppy"
            | "blue_orchid"
            | "allium"
            | "azure_bluet"
            | "oxeye_daisy"
            | "cornflower"
            | "lily_of_the_valley"
            | "torchflower"
            | "wither_rose"
            | "sweet_berry_bush"
    ) || path.ends_with("_sapling")
        || path.ends_with("_mushroom")
        || path.ends_with("_tulip")
}
fn fluid_render_height<S: Scene>(scene: &S, kind: FluidKind, pos: BlockPos) -> f32 {
    if let Some(cell) = FluidCell::at(scene, pos).filter(|cell| cell.kind == kind) {
        cell.height(scene, pos)
    } else if hides_fluid(scene, pos) {
        -1.0
    } else {
        0.0
    }
}
fn fluid_corner_height<S: Scene>(
    scene: &S,
    kind: FluidKind,
    own: f32,
    a: f32,
    b: f32,
    corner: BlockPos,
) -> f32 {
    if a >= 1.0 || b >= 1.0 {
        return 1.0;
    }
    let mut heights = vec![own, a, b];
    if a > 0.0 || b > 0.0 {
        heights.push(fluid_render_height(scene, kind, corner));
    }
    let (sum, weight) =
        heights
            .into_iter()
            .filter(|h| *h >= 0.0)
            .fold((0.0, 0.0), |(sum, weight), h| {
                let w = if h >= 0.8 { 10.0 } else { 1.0 };
                (sum + h * w, weight + w)
            });
    if weight == 0.0 { 0.0 } else { sum / weight }
}
fn fluid_quad(
    mesh: &mut ChunkMesh,
    transparent: &mut Vec<u32>,
    vertices: [[f32; 3]; 4],
    uv: [[f32; 2]; 4],
    color: [f32; 4],
    sky: f32,
    block_light: f32,
    water: bool,
) {
    let start = mesh.vertices.len() as u32;
    for (position, uv) in vertices.into_iter().zip(uv) {
        mesh.vertices.push(Vertex {
            position,
            uv,
            color,
            sky_light: sky,
            block_light,
        });
    }
    let indices = if water {
        transparent
    } else {
        &mut mesh.indices
    };
    indices.extend_from_slice(&[start, start + 1, start + 2, start, start + 2, start + 3]);
    mesh.faces += 1;
}
thread_local! {
    /// Checks only: fluid with no faces showing goes the whole way through
    /// [`append_fluid`] instead of stopping early.
    pub(crate) static FLUID_FULL_PATH: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

pub fn fluid_top<S: Scene>(scene: &S, pos: BlockPos) -> Option<[[f32; 3]; 4]> {
    let cell = FluidCell::at(scene, pos)?;
    let near = |d: (i32, i32, i32)| (pos.0 + d.0, pos.1 + d.1, pos.2 + d.2);
    let up = near((0, 1, 0));
    if FluidCell::at(scene, up).is_some_and(|above| above.kind == cell.kind)
        || hides_fluid(scene, up)
    {
        return None;
    }
    let h = fluid_render_height(scene, cell.kind, pos);
    let heights = if h >= 1.0 {
        [1.0; 4]
    } else {
        let north = fluid_render_height(scene, cell.kind, near((0, 0, -1)));
        let south = fluid_render_height(scene, cell.kind, near((0, 0, 1)));
        let west = fluid_render_height(scene, cell.kind, near((-1, 0, 0)));
        let east = fluid_render_height(scene, cell.kind, near((1, 0, 0)));
        [
            fluid_corner_height(scene, cell.kind, h, north, west, near((-1, 0, -1))),
            fluid_corner_height(scene, cell.kind, h, south, west, near((-1, 0, 1))),
            fluid_corner_height(scene, cell.kind, h, south, east, near((1, 0, 1))),
            fluid_corner_height(scene, cell.kind, h, north, east, near((1, 0, -1))),
        ]
    };
    let [x, y, z] = [pos.0 as f32, pos.1 as f32, pos.2 as f32];
    Some([
        [x, y + heights[0] - 0.001, z],
        [x, y + heights[1] - 0.001, z + 1.0],
        [x + 1.0, y + heights[2] - 0.001, z + 1.0],
        [x + 1.0, y + heights[3] - 0.001, z],
    ])
}

fn append_fluid<S: Scene>(
    scene: &S,
    pos: BlockPos,
    atlas: &Atlas,
    tint_source: &BiomeTint,
    light: &SkyLight,
    mesh: &mut ChunkMesh,
    transparent: &mut Vec<u32>,
) -> Result<()> {
    let cell = FluidCell::at(scene, pos).expect("fluid block has fluid state");
    let near = |d: (i32, i32, i32)| (pos.0 + d.0, pos.1 + d.1, pos.2 + d.2);
    let same = |p| FluidCell::at(scene, p).is_some_and(|other| other.kind == cell.kind);
    let up = near((0, 1, 0));
    let down = near((0, -1, 0));
    let top = !same(up) && !hides_fluid(scene, up);
    let bottom = !same(down) && !hides_fluid(scene, down);
    // The side faces, north, south, west and east: most fluid lies inside
    // a body of it and shows none of its faces.
    let sides = [(0, 0, -1), (0, 0, 1), (-1, 0, 0), (1, 0, 0)].map(|d| {
        let other = near(d);
        !(same(other) || hides_fluid(scene, other))
    });
    if !top && !bottom && !sides.contains(&true) && !FLUID_FULL_PATH.with(std::cell::Cell::get) {
        return Ok(());
    }
    let water = cell.kind == FluidKind::Water;
    static TEXTURES: std::sync::OnceLock<[ResourceId; 4]> = std::sync::OnceLock::new();
    let [water_still, water_flow, lava_still, lava_flow] = TEXTURES.get_or_init(|| {
        ["water_still", "water_flow", "lava_still", "lava_flow"].map(|name| {
            ResourceId::parse(&format!("minecraft:block/{name}")).expect("static texture id")
        })
    });
    let (still, flow) = if water {
        (water_still.clone(), water_flow.clone())
    } else {
        (lava_still.clone(), lava_flow.clone())
    };
    let tint = if water {
        scene.biome_at(pos).water_color.map(|c| c as f32 / 255.0)
    } else {
        [1.0; 3]
    };
    let _ = tint_source;
    let alpha = if water { 0.72 } else { 1.0 };
    let o = mesh.origin;
    let [x, y, z] = [(pos.0 - o[0]) as f32, (pos.1 - o[1]) as f32, (pos.2 - o[2]) as f32];
    let h = fluid_render_height(scene, cell.kind, pos);
    let nw;
    let ne;
    let sw;
    let se;
    if h >= 1.0 {
        nw = 1.0;
        ne = 1.0;
        sw = 1.0;
        se = 1.0;
    } else {
        let north = fluid_render_height(scene, cell.kind, near((0, 0, -1)));
        let south = fluid_render_height(scene, cell.kind, near((0, 0, 1)));
        let west = fluid_render_height(scene, cell.kind, near((-1, 0, 0)));
        let east = fluid_render_height(scene, cell.kind, near((1, 0, 0)));
        nw = fluid_corner_height(scene, cell.kind, h, north, west, near((-1, 0, -1)));
        ne = fluid_corner_height(scene, cell.kind, h, north, east, near((1, 0, -1)));
        sw = fluid_corner_height(scene, cell.kind, h, south, west, near((-1, 0, 1)));
        se = fluid_corner_height(scene, cell.kind, h, south, east, near((1, 0, 1)));
    }
    let sky = light.get(pos).max(light.get(up)) as f32;
    let color = |shade: f32| [shade * tint[0], shade * tint[1], shade * tint[2], alpha];
    let uv = |texture: &ResourceId, points: [[f32; 2]; 4]| {
        let [u0, v0, u1, v1] = atlas.region(texture);
        points.map(|[u, v]| [u0 + (u1 - u0) * u, v0 + (v1 - v0) * v])
    };
    if top {
        let mut dx = 0.0_f32;
        let mut dz = 0.0_f32;
        for d in [(0, 0, -1), (0, 0, 1), (-1, 0, 0), (1, 0, 0)] {
            let target = near(d);
            let hh = fluid_render_height(scene, cell.kind, target);
            let delta = if hh > 0.0 {
                h - hh
            } else if !hides_fluid(scene, target) {
                let below = near((d.0, -1, d.2));
                let low = fluid_render_height(scene, cell.kind, below);
                if low > 0.0 {
                    h - (low - 8.0 / 9.0)
                } else {
                    0.0
                }
            } else {
                0.0
            };
            dx += d.0 as f32 * delta;
            dz += d.2 as f32 * delta;
        }
        let (texture, coords) = if dx == 0.0 && dz == 0.0 {
            (&still, [[0.0, 0.0], [0.0, 1.0], [1.0, 1.0], [1.0, 0.0]])
        } else {
            let angle = dz.atan2(dx) - std::f32::consts::FRAC_PI_2;
            let (s, c) = (angle.sin() * 0.25, angle.cos() * 0.25);
            (
                &flow,
                [
                    [0.5 - c - s, 0.5 - c + s],
                    [0.5 - c + s, 0.5 + c + s],
                    [0.5 + c + s, 0.5 + c - s],
                    [0.5 + c - s, 0.5 - c - s],
                ],
            )
        };
        let texture = if atlas.contains(texture) {
            texture
        } else {
            &still
        };
        let top = fluid_top(scene, pos).expect("exposed fluid top");
        let top_uv = uv(texture, coords);
        fluid_quad(
            mesh,
            transparent,
            top,
            top_uv,
            color(1.0),
            sky,
            light.get_block(pos) as f32,
            water,
        );
        // `shouldRenderBackwardUpFace`: the same face turned down, so the
        // surface shows from under the water too.
        fluid_quad(
            mesh,
            transparent,
            [top[0], top[3], top[2], top[1]],
            [top_uv[0], top_uv[3], top_uv[2], top_uv[1]],
            color(1.0),
            sky,
            light.get_block(pos) as f32,
            water,
        );
    }
    if bottom {
        fluid_quad(
            mesh,
            transparent,
            [
                [x, y + 0.001, z],
                [x + 1.0, y + 0.001, z],
                [x + 1.0, y + 0.001, z + 1.0],
                [x, y + 0.001, z + 1.0],
            ],
            uv(&still, [[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]]),
            color(0.5),
            light.get(down) as f32,
            light.get_block(down) as f32,
            water,
        );
    }
    for (side, (_, points, heights, shade)) in [
        (
            (0, 0, -1),
            [[x, y, z + 0.001], [x + 1.0, y, z + 0.001]],
            [nw, ne],
            0.8,
        ),
        (
            (0, 0, 1),
            [[x + 1.0, y, z + 0.999], [x, y, z + 0.999]],
            [se, sw],
            0.8,
        ),
        (
            (-1, 0, 0),
            [[x + 0.001, y, z + 1.0], [x + 0.001, y, z]],
            [sw, nw],
            0.6,
        ),
        (
            (1, 0, 0),
            [[x + 0.999, y, z], [x + 0.999, y, z + 1.0]],
            [ne, se],
            0.6,
        ),
    ]
    .into_iter()
    .enumerate()
    {
        if !sides[side] {
            continue;
        }
        let [[ax, _, az], [bx, _, bz]] = points;
        let [ha, hb] = heights;
        fluid_quad(
            mesh,
            transparent,
            [
                [ax, y + ha - 0.001, az],
                [bx, y + hb - 0.001, bz],
                [bx, y, bz],
                [ax, y, az],
            ],
            uv(
                &flow,
                [
                    [0.0, (1.0 - ha) * 0.5],
                    [0.5, (1.0 - hb) * 0.5],
                    [0.5, 0.5],
                    [0.0, 0.5],
                ],
            ),
            color(shade),
            sky,
            light.get_block(pos) as f32,
            water,
        );
    }
    Ok(())
}
/// Draw only a newly placed block while its neighboring chunks are rebuilt on
/// the worker. This uses the same model, atlas, tint, AO and face rules as the
/// chunk path, but samples the already available light map for this brief pose.
pub fn block_preview<S: Scene>(
    scene: &S,
    packs: &PackStack,
    atlas: &Atlas,
    light: &SkyLight,
    tint_source: &BiomeTint,
    pos: BlockPos,
) -> Result<ChunkMesh> {
    let Some(block) = scene.block(pos) else {
        return Ok(ChunkMesh::default());
    };
    if matches!(block.id.path.as_str(), "water" | "lava") || crate::fluid::holds_water(&block) {
        let mut mesh = ChunkMesh::default();
        let mut transparent = Vec::new();
        append_fluid(
            scene,
            pos,
            atlas,
            tint_source,
            light,
            &mut mesh,
            &mut transparent,
        )?;
        if !transparent.is_empty() {
            mesh.transparent_start = Some(mesh.indices.len() as u32);
            mesh.indices.extend(transparent);
        }
        return Ok(mesh);
    }
    let variants = resolve_block_variants(packs, block)?;
    let model = &variants[variant_index(&variants, pos)].0;
    let biome = scene.biome_at(TintKind::of(block).sample_pos(pos));
    let mut mesh = ChunkMesh::default();
    let mut transparent_indices = Vec::new();
    let mut neighbor_cube = HashMap::new();
    for element in &model.elements {
        for face in &element.faces {
            if !atlas.contains(&face.texture) {
                return Err(anyhow!("atlas missing {}", face.texture.key()));
            }
            let imported_corners = face.triangle.map(|_| model_face_corners(element, face)).transpose()?;
            let dir = if let Some(corners) = &imported_corners { model_face_direction(element, face, corners)? }
                else { element_direction(element, face.direction.as_str())? };
            let delta = direction(dir)?;
            let neighbor_pos = (pos.0 + delta.0, pos.1 + delta.1, pos.2 + delta.2);
            if face.cull {
                if let Some(neighbor) = scene.block(neighbor_pos) {
                    let full_cube = if neighbor.is_opaque() {
                        let key = model_key(neighbor);
                        if let Some(&cube) = neighbor_cube.get(&key) {
                            cube
                        } else {
                            let cube = resolve_block_variants(packs, neighbor)?.iter().all(
                                |(model, _)| {
                                    model
                                        .elements
                                        .iter()
                                        .any(|part| part.from == [0.0; 3] && part.to == [1.0; 3] && part.faces.iter().all(|face| face.triangle.is_none()))
                                },
                            );
                            neighbor_cube.insert(key, cube);
                            cube
                        }
                    } else {
                        false
                    };
                    if hides_shared_face(block, neighbor) || (neighbor.is_opaque() && full_cube) {
                        continue;
                    }
                }
            }
            let corners = model_face_corners(element, face)?;
            let [u0, v0, u1, v1] = if block.id.path == "chest" {
                // Chest ModelPart UVs address individual pixels in a 64x64
                // entity sheet. The terrain atlas' half-texel tile inset
                // distorts one-pixel lock faces and can sample alpha beside
                // them, so preserve the entity sheet's exact coordinate span.
                atlas.region_exact(&face.texture)
            } else {
                atlas.region(&face.texture)
            };
            let uv = face.vertex_uvs().map(|[u, v]| [u0 + (u1 - u0) * u, v0 + (v1 - v0) * v]);
            let shade = if block.id.path == "chest" {
                level_item_shade(face_normal(dir)?)
            } else {
                match element.shade_direction_override.as_deref().unwrap_or(dir) {
                    "up" => 1.0,
                    "down" => 0.5,
                    "north" | "south" => 0.8,
                    _ => 0.6,
                }
            };
            let tint = block_face_tint(block, face.tint_index.or_else(||face.tint.then_some(0)), biome, tint_source);
            let alpha = if block.id.path == "water" { 0.72 } else { 1.0 };
            // ChestRenderer lights every ModelPart face from the block entity
            // position; a rotated lid must not retain its closed downward
            // face's dark floor-neighbor light.
            let light_pos = if block.id.path == "chest" {
                pos
            } else if let Some(corners) = &imported_corners {
                if face.cull || face_cubic(dir, corners) { neighbor_pos } else { pos }
            } else {
                neighbor_pos
            };
            let full_cube = face.triangle.is_none() && element.from == [0.0; 3] && element.to == [1.0; 3];
            let start = mesh.vertices.len() as u32;
            for (corner, uv) in corners.iter().zip(uv) {
                let (ao, sky_light) = if full_cube {
                    ambient_vertex(scene, light, light_pos, dir, *corner)
                } else {
                    (1.0, light.get(light_pos) as f32)
                };
                mesh.vertices.push(Vertex {
                    position: [
                        (pos.0 - mesh.origin[0]) as f32 + corner[0],
                        (pos.1 - mesh.origin[1]) as f32 + corner[1],
                        (pos.2 - mesh.origin[2]) as f32 + corner[2],
                    ],
                    uv,
                    color: [
                        shade * ao * tint[0],
                        shade * ao * tint[1],
                        shade * ao * tint[2],
                        alpha,
                    ],
                    sky_light,
                    block_light: light.get_block(light_pos) as f32,
                });
            }
            let indices = if block.id.path == "water" || face.force_translucent {
                &mut transparent_indices
            } else {
                &mut mesh.indices
            };
            indices.extend_from_slice(&[start, start + 1, start + 2, start, start + 2, start + 3]);
            mesh.faces += 1;
        }
    }
    if !transparent_indices.is_empty() {
        mesh.transparent_start = Some(mesh.indices.len() as u32);
        mesh.indices.extend(transparent_indices);
    }
    Ok(mesh)
}
/// One block's model placed with a pose (block units, the block's corner at
/// the pose's origin), as `BlockRenderDispatcher.renderSingleBlock` draws a
/// block an entity carries (a minecart's chest, furnace, hopper or TNT):
/// every face, shaded as an entity's, lit with `sky` and `block_light`;
/// `brightness` over 1 whitens it (a primed TNT's flash).
#[allow(clippy::too_many_arguments)]
pub fn append_block_posed(
    mesh: &mut ChunkMesh,
    packs: &PackStack,
    atlas: &Atlas,
    block: &Block,
    pose: Mat4,
    sky: f32,
    block_light: f32,
    brightness: f32,
) -> Result<()> {
    let variants = resolve_block_variants(packs, block)?;
    let Some((model, _)) = variants.first() else {
        return Ok(());
    };
    let chest = block.id.path.ends_with("chest");
    let normal_pose = pose.inverse().transpose();
    let origin = Vec3::new(mesh.origin[0] as f32, mesh.origin[1] as f32, mesh.origin[2] as f32);
    for element in &model.elements {
        for face in &element.faces {
            if !atlas.contains(&face.texture) {
                continue;
            }
            let corners = model_face_corners(element, face)?;
            let [u0, v0, u1, v1] = if chest { atlas.region_exact(&face.texture) } else { atlas.region(&face.texture) };
            let uv = face.vertex_uvs().map(|[u, v]| [u0 + (u1 - u0) * u, v0 + (v1 - v0) * v]);
            let normal = normal_pose.transform_vector3(triangle_normal(&corners)).normalize_or_zero();
            let shade = level_item_shade(normal) * brightness;
            let start = mesh.vertices.len() as u32;
            for (corner, uv) in corners.iter().zip(uv) {
                let p = pose.transform_point3(Vec3::from_array(*corner)) - origin;
                mesh.vertices.push(Vertex {
                    position: p.to_array(),
                    uv,
                    color: [shade, shade, shade, 1.0],
                    sky_light: sky,
                    block_light,
                });
            }
            mesh.indices.extend_from_slice(&[start, start + 1, start + 2, start, start + 2, start + 3]);
            mesh.faces += 1;
        }
    }
    Ok(())
}

/// HalfTransparentBlock skips its own neighboring face. Fancy cutout leaves
/// retain shared faces in 26.3; rasterizer back-face culling hides their backs.
pub(crate) fn hides_shared_face(block: &Block, neighbor: &Block) -> bool {
    if block.id != neighbor.id {
        return false;
    }
    let path = block.id.path.as_str();
    path == "water" || path == "glass" || path == "tinted_glass" || path.ends_with("_stained_glass")
}

/// Full-cube case of BlockModelLighter's four neighboring shade and light
/// samples. Keeping values per vertex produces the smooth block-edge fade.
fn ambient_vertex<S: Scene>(
    scene: &S,
    light: &SkyLight,
    base: (i32, i32, i32),
    dir: &str,
    corner: [f32; 3],
) -> (f32, f32) {
    let axes = match dir {
        "up" | "down" => [0, 2],
        "north" | "south" => [0, 1],
        _ => [1, 2],
    };
    let mut side = [[0; 3]; 2];
    for (i, axis) in axes.into_iter().enumerate() {
        side[i][axis] = if corner[axis] < 0.5 { -1 } else { 1 };
    }
    let positions = [
        base,
        (
            base.0 + side[0][0],
            base.1 + side[0][1],
            base.2 + side[0][2],
        ),
        (
            base.0 + side[1][0],
            base.1 + side[1][1],
            base.2 + side[1][2],
        ),
        (
            base.0 + side[0][0] + side[1][0],
            base.1 + side[0][1] + side[1][1],
            base.2 + side[0][2] + side[1][2],
        ),
    ];
    let center_light = light.get(base);
    let shade = positions
        .iter()
        .map(|&p| if scene.shade_darkens_at(p) { 0.2 } else { 1.0 })
        .sum::<f32>()
        * 0.25;
    let sky = positions
        .iter()
        .map(|&p| {
            let level = light.get(p);
            if center_light > 2 && level == 0 {
                center_light as f32
            } else {
                level as f32
            }
        })
        .sum::<f32>()
        * 0.25;
    (shade, sky)
}
fn direction(dir: &str) -> Result<(i32, i32, i32)> {
    Ok(match dir {
        "down" => (0, -1, 0),
        "up" => (0, 1, 0),
        "north" => (0, 0, -1),
        "south" => (0, 0, 1),
        "west" => (-1, 0, 0),
        "east" => (1, 0, 0),
        _ => return Err(anyhow!("unsupported face direction {dir}")),
    })
}
fn corners(dir: &str, a: [f32; 3], b: [f32; 3]) -> Result<[[f32; 3]; 4]> {
    let [x, y, z] = a;
    let [xx, yy, zz] = b;
    Ok(match dir {
        "up" => [[x, yy, z], [x, yy, zz], [xx, yy, zz], [xx, yy, z]],
        "down" => [[x, y, zz], [x, y, z], [xx, y, z], [xx, y, zz]],
        "north" => [[xx, yy, z], [xx, y, z], [x, y, z], [x, yy, z]],
        "south" => [[x, yy, zz], [x, y, zz], [xx, y, zz], [xx, yy, zz]],
        "west" => [[x, yy, z], [x, y, z], [x, y, zz], [x, yy, zz]],
        "east" => [[xx, yy, zz], [xx, y, zz], [xx, y, z], [xx, yy, z]],
        _ => return Err(anyhow!("unsupported face direction {dir}")),
    })
}
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn java_model_rotated_item_normals_follow_all_axes_and_element_rotation() {
        let temp = crate::test_dir::tempdir().unwrap();
        std::fs::write(temp.path().join("pack.mcmeta"),
            r#"{"pack":{"min_format":[97,1],"max_format":[97,1]}}"#).unwrap();
        let packs = PackStack::open(vec![temp.path().into()]).unwrap();
        let texture = ResourceId::parse("minecraft:missingno").unwrap();
        let atlas = make_atlas(&packs, vec![texture.clone()]).unwrap();
        let mut element = crate::model::Element {
            from: [0.0;3], to: [1.0;3], rotation_x: 0, rotation_y: 0,
            rotation: None, shade_direction_override: None,
            faces: vec![crate::model::Face {
                direction: "south".into(), texture, uv: [0.0,0.0,1.0,1.0],
                baked_uv: None, triangle: None, cull: false, cullface: None,
                tint: false, tint_index: None, force_translucent: false,
            }],
        };
        for (x,y,expected) in [(90,0,Vec3::Y), (0,90,Vec3::NEG_X),
            (90,90,Vec3::Y), (180,270,Vec3::NEG_X)] {
            element.rotation_x=x; element.rotation_y=y;
            let visual = ItemVisual { faces: None,
                model: ResolvedModel { elements: vec![element.clone()] },
                ground: Mat4::IDENTITY, right_hand: Mat4::IDENTITY,
            left_hand: Mat4::IDENTITY,
                min_y: 0.0, tints: vec![], flat: false };
            let faces = item_faces(&visual, &atlas).unwrap();
            assert!((faces[0].normal-expected).length() < 1e-6, "X={x}, Y={y}");
        }
        element.rotation_x=0; element.rotation_y=0;
        element.rotation=crate::model::ElementRotation::parse(&serde_json::json!({
            "rotation":{"origin":[8,8,8],"axis":"x","angle":45}
        })).unwrap();
        let corners=model_face_corners(&element,&element.faces[0]).unwrap();
        let n=triangle_normal(&corners);
        assert!((n-Vec3::new(0.0,-1.0,1.0).normalize()).length()<1e-6);
    }

    #[test]
    fn imported_triangle_baked_slow_item_and_held_paths_keep_geometry_and_uvs() {
        let temp = crate::test_dir::tempdir().unwrap();
        let root = temp.path();
        for (name, text) in [
            ("pack.mcmeta", r#"{"pack":{"min_format":[97,1],"max_format":[97,1]}}"#),
            ("assets/test/blockstates/tuft.json", r#"{"variants":{"":{"model":"test:block/tuft","x":90,"y":270}}}"#),
            ("assets/test/items/tuft.json", r#"{"model":{"type":"minecraft:model","model":"test:block/tuft"}}"#),
            ("assets/test/models/block/tuft.json", r##"{"textures":{"g":"test:block/grass"},"elements":[{"from":[0,0,0],"to":[16,16,16],"rotation":{"origin":[8,8,8],"axis":"y","angle":90},"faces":{"up":{"texture":"#g","iw4l_triangle":{"positions":[[2,1,3],[12,3,5],[5,13,11]],"uv":[[1,2],[15,3],[7,14]],"double_sided":true}}}}]}"##),
        ] {
            let path = root.join(name);
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            std::fs::write(path, text).unwrap();
        }
        let texture_path = root.join("assets/test/textures/block/grass.png");
        std::fs::create_dir_all(texture_path.parent().unwrap()).unwrap();
        RgbaImage::from_pixel(16, 16, image::Rgba([230, 180, 30, 255])).save(texture_path).unwrap();
        let packs = PackStack::open(vec![root.into()]).unwrap();
        let block = Block::new("test:tuft");
        let mut model = crate::model::resolve_block(&packs, &block).unwrap();
        let first = model_face_corners(&model.elements[0], &model.elements[0].faces[0]).unwrap()[0];
        for (actual, expected) in first.into_iter().zip([0.9375, 0.875, 0.8125]) {
            assert!((actual - expected).abs() < 1e-6, "element rotation, then block X and Y");
        }
        let texture = ResourceId::parse("test:block/grass").unwrap();
        let atlas = make_atlas(&packs, vec![texture.clone()]).unwrap();
        let mut scene = HandcraftedScene::new();
        scene.set((0, 0, 0), Some(block.clone()));
        let light = SkyLight::build(&scene);
        let tint = BiomeTint { grass: None, foliage: None, dry_foliage: None };
        for x in [0, 90, 180, 270] {
            for y in [0, 90, 180, 270] {
                model.elements[0].rotation_x = x;
                model.elements[0].rotation_y = y;
                let variants = vec![(model.clone(), 1)];
                assert!(!all_full_cubes(&variants), "triangle bounds do not occlude a full cube");
                let quads = bake_quads(&block, &model, &atlas).unwrap();
                let mut slow = ChunkMesh::default();
                let mut baked = ChunkMesh::default();
                let mut slow_alpha = Vec::new();
                let mut baked_alpha = Vec::new();
                append_block(&scene, (0,0,0), &block, || Ok(&variants), |_| false,
                    &atlas, &tint, &light, &mut slow, &mut slow_alpha).unwrap();
                append_baked(&scene, (0,0,0), &quads, TintKind::of(&block), false, |_| false,
                    |_| false, &atlas, &tint, &light, &mut baked, &mut baked_alpha).unwrap();
                assert_eq!(slow.indices, baked.indices);
                assert_eq!(slow_alpha, baked_alpha);
                assert_eq!(bytemuck::cast_slice::<_,u8>(&slow.vertices), bytemuck::cast_slice::<_,u8>(&baked.vertices));
                assert_eq!(slow.vertices.len(), 4);
                assert_eq!(slow.indices, [0,1,2,0,2,3]);
                assert_eq!(slow.vertices[2].position, slow.vertices[3].position);
                assert_eq!(slow.vertices[2].uv, slow.vertices[3].uv);
                assert!(triangle_normal(&quads[0].corners).length_squared() > 0.99);
                let [u0,v0,u1,v1] = atlas.region(&texture);
                for (actual, source) in slow.vertices.iter().zip([[1.0,2.0],[15.0,3.0],[7.0,14.0],[7.0,14.0]]) {
                    assert_eq!(actual.uv, [u0+(u1-u0)*(source[0]/16.0), v0+(v1-v0)*(source[1]/16.0)]);
                }
            }
        }
        let original = crate::model::resolve_block(&packs, &block).unwrap();
        let variants = vec![(original.clone(), 1)];
        let mut slow = ChunkMesh::default();
        append_block(&scene, (0,0,0), &block, || Ok(&variants), |_| false,
            &atlas, &tint, &light, &mut slow, &mut Vec::new()).unwrap();
        let preview = block_preview(&scene, &packs, &atlas, &light, &tint, (0,0,0)).unwrap();
        assert_eq!(bytemuck::cast_slice::<_,u8>(&slow.vertices), bytemuck::cast_slice::<_,u8>(&preview.vertices));
        let mut posed = ChunkMesh::default();
        let block_pose = Mat4::from_translation(Vec3::new(2.0,1.0,3.0));
        append_block_posed(&mut posed, &packs, &atlas, &block, block_pose, 15.0, 0.0, 1.0).unwrap();
        for (actual, source) in posed.vertices.iter().zip(&slow.vertices) {
            assert_eq!(actual.position, block_pose.transform_point3(Vec3::from_array(source.position)).to_array());
            assert_eq!(actual.uv, source.uv);
        }
        let item = ResourceId::parse("test:tuft").unwrap();
        let build = build_item_model(&packs, &item).unwrap().unwrap();
        let item_mesh = &build.chunks[&(0,0)];
        assert_eq!(item_mesh.vertices.len(), 4);
        let item_model = crate::model::resolve_item_model(&packs, &item).unwrap().unwrap();
        let expected = model_face_corners(&item_model.elements[0], &item_model.elements[0].faces[0]).unwrap();
        for (vertex, expected) in item_mesh.vertices.iter().zip(expected) { assert_eq!(vertex.position, expected); }
        let mut visuals = ItemVisuals::default();
        let cached = item_faces(visuals.model("test:tuft", &packs).unwrap().unwrap(), &build.atlas).unwrap();
        assert_eq!(cached.len(), 1);
        assert_eq!(cached[0].corners.map(|v| v.to_array()), expected);
        let pose = Mat4::from_translation(Vec3::new(3.0,4.0,5.0)) * Mat4::from_rotation_z(0.4);
        let mut held = ChunkMesh::default();
        visuals.append_held_items(&mut held, &[HeldItem {
            id: "test:tuft".into(), pose, light: Vec3::ZERO,
            display: HeldDisplay::RightHand, first_tint: None,
        }], &packs, &build.atlas, &light).unwrap();
        let display = crate::model::item_display_transform(&packs, &item, "thirdperson_righthand").unwrap();
        for (vertex, expected) in held.vertices.iter().zip(expected) {
            assert_eq!(vertex.position, (pose * display).transform_point3(Vec3::from_array(expected)).to_array());
        }
        assert_eq!(held.indices, item_mesh.indices);
    }

    /// The `cross` model's planes: rotated 45° about the block's middle and
    /// rescaled, they run corner to corner; a plane in the middle of the
    /// block takes its light from its own cell, a face on the block's side
    /// from the neighbour (`BlockModelLighter.faceCubic`).
    #[test]
    fn cross_planes_turn_diagonal_and_light_from_their_own_cell() {
        let raw: serde_json::Value = serde_json::from_str(
            r#"{"rotation":{"origin":[8,8,8],"axis":"y","angle":45,"rescale":true}}"#,
        )
        .unwrap();
        let element = crate::model::Element {
            from: [0.8 / 16.0, 0.0, 0.5],
            to: [15.2 / 16.0, 1.0, 0.5],
            faces: Vec::new(),
            rotation_x: 0,
            rotation_y: 0,
            rotation: crate::model::ElementRotation::parse(&raw).unwrap(),
            shade_direction_override: None,
        };
        let corners = element_corners(&element, "north").unwrap();
        for (corner, (x, z)) in
            corners
                .iter()
                .zip([(0.95, 0.05), (0.95, 0.05), (0.05, 0.95), (0.05, 0.95)])
        {
            assert!(
                (corner[0] - x).abs() < 1e-5 && (corner[2] - z).abs() < 1e-5,
                "{corner:?}"
            );
        }
        // The normal points between north and west; north comes first.
        assert_eq!(
            quad_direction(&element, "north", &corners).unwrap(),
            "north"
        );
        assert_eq!(
            quad_direction(
                &element,
                "south",
                &element_corners(&element, "south").unwrap()
            )
            .unwrap(),
            "south"
        );
        assert!(
            !face_cubic("north", &corners),
            "a diagonal plane is lit from its own cell"
        );
        let cube = crate::model::Element {
            from: [0.0; 3],
            to: [1.0; 3],
            faces: Vec::new(),
            rotation_x: 0,
            rotation_y: 0,
            rotation: None,
            shade_direction_override: None,
        };
        let top = element_corners(&cube, "up").unwrap();
        assert!(face_cubic("up", &top), "a cube's top lies on its side");
        let slab = crate::model::Element {
            from: [0.0; 3],
            to: [1.0, 0.5, 1.0],
            faces: Vec::new(),
            rotation_x: 0,
            rotation_y: 0,
            rotation: None,
            shade_direction_override: None,
        };
        assert!(
            !face_cubic("up", &element_corners(&slab, "up").unwrap()),
            "a slab's top is inside the block"
        );
        // Huge mushroom blocks: one north-facing quad, turned up by x=270
        // and down by x=90.
        for (x, side, y) in [(270, "up", 1.0), (90, "down", 0.0)] {
            let face = crate::model::Element {
                from: [0.0; 3],
                to: [1.0, 1.0, 0.0],
                faces: Vec::new(),
                rotation_x: x,
                rotation_y: 0,
                rotation: None,
                shade_direction_override: None,
            };
            let corners = element_corners(&face, "north").unwrap();
            assert!(corners.iter().all(|c| c[1] == y), "x={x}: {corners:?}");
            assert_eq!(quad_direction(&face, "north", &corners).unwrap(), side);
            assert!(face_cubic(side, &corners));
        }
        // A 22.5° torch tilt about z: no rescale.
        let raw: serde_json::Value =
            serde_json::from_str(r#"{"rotation":{"origin":[0,3.5,8],"axis":"z","angle":-22.5}}"#)
                .unwrap();
        let tilt = crate::model::ElementRotation::parse(&raw).unwrap().unwrap();
        let [x, y, _] = tilt.apply([0.0, 3.5 / 16.0 + 1.0, 0.5]);
        assert!(
            (x - (22.5f32).to_radians().sin()).abs() < 1e-5
                && (y - (3.5 / 16.0 + (22.5f32).to_radians().cos())).abs() < 1e-5,
            "{x} {y}"
        );
    }
    use crate::scene::{BlockPos, ChunkPos, HandcraftedScene};

    #[test]
    fn entity_sheet_region_restores_exact_pixel_coordinates() {
        let texture = ResourceId::parse("minecraft:entity/chest/normal_left").unwrap();
        let atlas = Atlas {
            pixels: RgbaImage::new(64, 64),
            mipmaps: vec![],
            slots: HashMap::from([(
                texture.clone(),
                [0.5 / 64.0, 0.5 / 64.0, 63.5 / 64.0, 63.5 / 64.0],
            )]),
            missing: vec![],
            animated: vec![],
            animated_tiles: vec![],
        };
        assert_eq!(atlas.region_exact(&texture), [0.0, 0.0, 1.0, 1.0]);
    }

    #[test]
    fn entity_skin_uvs_keep_internal_pixel_boundaries_exact() {
        let id = ResourceId::parse("minecraft:entity/zombie/zombie").unwrap();
        let mut slots = HashMap::new();
        slots.insert(
            id.clone(),
            [64.5 / 128.0, 0.5 / 128.0, 127.5 / 128.0, 63.5 / 128.0],
        );
        let atlas = Atlas {
            pixels: RgbaImage::new(128, 128),
            mipmaps: Vec::new(),
            slots,
            missing: Vec::new(),
            animated: Vec::new(),
            animated_tiles: Vec::new(),
        };
        let [u0, v0, u1, v1] = atlas.entity_region(&id);
        assert_eq!([u0, v0, u1, v1], [0.5, 0.0, 1.0, 0.5]);
        // The zombie hand starts at skin pixel 48. With the regular inset,
        // that boundary lands at 47.75 and selects a shirt pixel instead.
        assert_eq!((u0 + (u1 - u0) * 48.0 / 64.0) * 128.0, 112.0);
        assert!(
            (atlas.region(&id)[0] + (atlas.region(&id)[2] - atlas.region(&id)[0]) * 48.0 / 64.0)
                * 128.0
                < 112.0
        );
    }

    #[test]
    fn wire_tint_uses_power_and_leaves_overlay_untinted() {
        let source = BiomeTint {
            grass: None,
            foliage: None, dry_foliage: None,
        };
        let wire = Block::new("minecraft:redstone_wire").with("power", "15");
        let tinted = block_face_tint(&wire, Some(0), BiomeSample::THE_VOID, &source);
        assert_eq!(redstone_wire_color_argb(0), 0xff4c0000);
        assert_eq!(redstone_wire_color_argb(15), 0xffff3200);
        assert_eq!(tinted, [1.0, 50.0 / 255.0, 0.0]);
        assert_eq!(
            block_face_tint(&wire, None, BiomeSample::THE_VOID, &source),
            [1.0; 3]
        );
    }

    #[test]
    fn discrete_frames_follow_pack_order_and_tick_durations() {
        let mut sheet = RgbaImage::new(2, 6);
        for frame in 0..3 {
            for y in frame * 2..frame * 2 + 2 {
                for x in 0..2 {
                    sheet.put_pixel(x, y, image::Rgba([frame as u8 * 40, 0, 0, 255]));
                }
            }
        }
        let metadata = serde_json::json!({
            "frametime": 2,
            "frames": [0, {"index": 2, "time": 3}, 1]
        });
        let tile = animated_tile(&ResourceId::parse("minecraft:block/test").unwrap(), &sheet, &metadata, (64, 128)).unwrap();
        assert_eq!(tile.origin, (64, 128));
        assert_eq!(
            (0..7).map(|tick| tile.frame_at(tick)).collect::<Vec<_>>(),
            [0, 0, 2, 2, 2, 1, 1]
        );
        assert_eq!(tile.frame_at(7), 0);
        assert_eq!(tile.frames[2][0].get_pixel(0, 0).0, [80, 0, 0, 255]);
        assert_eq!(tile.frames[2][4].dimensions(), (4, 4));
    }

    #[test]
    fn interpolated_sprite_uses_pinned_thousandth_progress() {
        let mut sheet = RgbaImage::new(2, 4);
        for y in 2..4 {
            for x in 0..2 {
                sheet.put_pixel(x, y, image::Rgba([255, 0, 0, 255]));
            }
        }
        let tile = animated_tile(
            &ResourceId::parse("minecraft:block/test").unwrap(),
            &sheet,
            &serde_json::json!({"frametime":8,"interpolate":true}),
            (0, 0),
        )
        .unwrap();
        let state = tile.state_at(4);
        assert_eq!(state.progress_millis, 500);
        assert_eq!((state.current, state.next), (0, 1));
        assert_eq!(tile.image_at(state, 0).get_pixel(0, 0).0, [128, 0, 0, 128]);
    }
    #[test]
    fn dropped_item_shadow_projects_on_ground() {
        let shadow = ResourceId::parse("minecraft:misc/shadow").unwrap();
        let atlas = Atlas {
            pixels: RgbaImage::new(32, 32),
            mipmaps: vec![],
            slots: HashMap::from([(shadow, [0.0, 0.0, 1.0, 1.0])]),
            missing: vec![],
            animated: vec![],
            animated_tiles: vec![],
        };
        let scene = HandcraftedScene::new();
        let mut items = WorldItems::default();
        items.spawn(
            minecraftoss_player::inventory::ItemStack::new("minecraft:stone", 1),
            glam::DVec3::new(0.5, 1.0, 0.5),
        );
        let casters = item_shadow_casters(&items, glam::DVec3::new(0.5, 2.62, 2.5), 1.0);
        let shadow_mesh = entity_shadows(&casters, &scene, &atlas, &|_| 15, 0.0).unwrap();
        assert!(shadow_mesh.faces >= 1);
        assert!(
            shadow_mesh
                .vertices
                .iter()
                .any(|vertex| vertex.position[1] > 1.0)
        );
        assert!(
            shadow_mesh
                .vertices
                .iter()
                .all(|vertex| vertex.color[3] > 0.0)
        );
    }
    #[test]
    fn dropped_grass_uses_its_item_tint_face_lighting_and_local_skylight() {
        let texture = ResourceId::parse("minecraft:block/grass_block_top").unwrap();
        let atlas = Atlas {
            pixels: RgbaImage::new(16, 16),
            mipmaps: vec![],
            slots: HashMap::from([(texture.clone(), [0.0, 0.0, 1.0, 1.0])]),
            missing: vec![],
            animated: vec![],
            animated_tiles: vec![],
        };
        let face = |direction: &str, tint_index: Option<usize>| crate::model::Face {
            direction: direction.into(),
            texture: texture.clone(),
            uv: [0.0, 0.0, 1.0, 1.0],
            cull: false,
            cullface: None,
            baked_uv: None,
            tint: tint_index.is_some(),
            tint_index,
            force_translucent: false,
            triangle: None,
        };
        let mut visuals = ItemVisuals::default();
        visuals.models.insert(
            "minecraft:grass_block".into(),
            ItemVisual {
                faces: None,
                model: ResolvedModel {
                    elements: vec![crate::model::Element {
                        from: [0.0; 3],
                        to: [1.0; 3],
                        faces: vec![face("up", Some(0)), face("north", None)],
                        rotation_x: 0,
                        rotation_y: 0,
                        rotation: None,
                        shade_direction_override: None,
                    }],
                },
                ground: Mat4::IDENTITY,
                right_hand: Mat4::IDENTITY,
            left_hand: Mat4::IDENTITY,
                min_y: 0.0,
                tints: vec![[0.3, 0.6, 0.2]],
                flat: false,
            },
        );
        let mut items = WorldItems::default();
        items.spawn(
            minecraftoss_player::inventory::ItemStack::new("minecraft:grass_block", 1),
            glam::DVec3::new(5.5, 6.0, -6.5),
        );
        let sky_light = SkyLight::build(&HandcraftedScene::new());
        let mesh = visuals
            .mesh(
                &items,
                &PackStack::open(vec![]).unwrap(),
                &atlas,
                &sky_light,
                1.0,
            )
            .unwrap();
        assert_eq!(mesh.vertices.len(), 8);
        for vertex in &mesh.vertices[..4] {
            for (actual, expected) in vertex.color[..3].iter().zip([0.3, 0.6, 0.2]) {
                assert!((actual - expected).abs() < 0.0001);
            }
        }
        assert!(mesh.vertices[4].color[0] < 0.95);
        assert!(mesh.vertices.iter().all(|vertex| vertex.sky_light == 13.0));
    }
    #[test]
    fn local_block_edit_mesh_timing() {
        let path = std::env::var("MINECRAFTOSS_PACK").expect("MINECRAFTOSS_PACK must point to the original resource pack");
        assert!(std::path::Path::new(&path).exists(), "resource pack missing: {path}");
        let packs = PackStack::open(vec![path.into()]).unwrap();
        let mut scene = HandcraftedScene::new();
        let build = build(&scene, &packs).unwrap();
        scene.set((0, 2, 3), Some(Block::new("minecraft:glass")));
        let tint = BiomeTint::from_pack(&packs).unwrap();
        let mut preview_times = Vec::new();
        let mut times = Vec::new();
        let mut light_times = Vec::new();
        let mut clone_times = Vec::new();
        for _ in 0..5 {
            let preview_start = std::time::Instant::now();
            let preview = block_preview(
                &scene,
                &packs,
                &build.atlas,
                build.sky_light.as_deref().unwrap(),
                &tint,
                (0, 2, 3),
            )
            .unwrap();
            preview_times.push(preview_start.elapsed().as_secs_f64() * 1000.0);
            assert!(preview.faces > 0);
            let clone_start = std::time::Instant::now();
            let _ = scene.clone();
            clone_times.push(clone_start.elapsed().as_secs_f64() * 1000.0);
            let light_start = std::time::Instant::now();
            let _ = SkyLight::build(&scene);
            light_times.push(light_start.elapsed().as_secs_f64() * 1000.0);
            let start = std::time::Instant::now();
            let changed = rebuild_near(&scene, &packs, &build, (0, 2, 3)).unwrap();
            times.push(start.elapsed().as_secs_f64() * 1000.0);
            assert!(!changed.is_empty());
        }
        eprintln!("nearby chunk rebuild ms: {times:?}");
        eprintln!("single block preview ms: {preview_times:?}");
        eprintln!("skylight rebuild ms: {light_times:?}");
        eprintln!("scene dispatch clone ms: {clone_times:?}");
        let near = rebuild_near(&scene, &packs, &build, (0, 2, 3)).unwrap();
        let full = super::build(&scene, &packs).unwrap();
        for (chunk, mesh) in near {
            let canonical = full.chunks.get(&chunk).unwrap();
            assert_eq!(mesh.indices, canonical.indices);
            assert_eq!(
                bytemuck::cast_slice::<_, u8>(&mesh.vertices),
                bytemuck::cast_slice::<_, u8>(&canonical.vertices)
            );
        }
    }
    struct Two {
        a: Block,
        b: Block,
    }
    impl Scene for Two {
        fn block(&self, p: BlockPos) -> Option<&Block> {
            match p {
                (15, 0, 0) => Some(&self.a),
                (16, 0, 0) => Some(&self.b),
                _ => None,
            }
        }
        fn chunks(&self) -> Vec<ChunkPos> {
            vec![(0, 0), (1, 0)]
        }
        fn vertical_range(&self) -> std::ops::Range<i32> {
            0..1
        }
        fn revision(&self) -> u64 {
            0
        }
    }
    fn test_box_model(texture: &ResourceId, from: [f32; 3], to: [f32; 3]) -> ResolvedModel {
        ResolvedModel {
            elements: vec![crate::model::Element {
                from,
                to,
                rotation_x: 0,
                rotation_y: 0,
                rotation: None,
                shade_direction_override: None,
                faces: ["down", "up", "north", "south", "west", "east"]
                    .map(|direction| crate::model::Face {
                        direction: direction.into(),
                        texture: texture.clone(),
                        uv: [0.0, 0.0, 1.0, 1.0],
                        cull: true,
                        cullface: None,
                        baked_uv: None,
                        tint: false,
                        tint_index: None,
                        force_translucent: false,
                        triangle: None,
                    })
                    .to_vec(),
            }],
        }
    }

    #[test]
    fn partial_neighbors_keep_the_full_block_face() {
        let texture = ResourceId::parse("minecraft:block/stone").unwrap();
        let atlas = Atlas {
            pixels: RgbaImage::new(16, 16),
            mipmaps: vec![],
            slots: HashMap::from([(texture.clone(), [0.0, 0.0, 1.0, 1.0])]),
            missing: vec![],
            animated: vec![],
            animated_tiles: vec![],
        };
        let tint = BiomeTint {
            grass: None,
            foliage: None, dry_foliage: None,
        };
        for (neighbor_id, neighbor_model, expected_faces) in [
            (
                "minecraft:oak_slab",
                test_box_model(&texture, [0.0; 3], [1.0, 0.5, 1.0]),
                6,
            ),
            (
                "minecraft:oak_stairs",
                test_box_model(&texture, [0.0; 3], [1.0, 0.5, 1.0]),
                6,
            ),
            (
                "minecraft:oak_slab",
                test_box_model(&texture, [0.0; 3], [1.0; 3]),
                5,
            ),
        ] {
            let scene = Two {
                a: Block::new("minecraft:stone"),
                b: Block::new(neighbor_id),
            };
            let models = HashMap::from([
                (
                    model_key(&scene.a),
                    vec![(test_box_model(&texture, [0.0; 3], [1.0; 3]), 1)],
                ),
                (model_key(&scene.b), vec![(neighbor_model, 1)]),
            ]);
            let light = SkyLight::build(&scene);
            assert_eq!(
                mesh_chunk(&scene, (0, 0), &models, &atlas, &tint, &light)
                    .unwrap()
                    .faces,
                expected_faces,
                "neighbor {neighbor_id}"
            );
        }
    }

    #[test]
    fn lily_pads_and_vines_leave_the_water_surface_showing() {
        for (neighbor, hides) in [
            ("minecraft:lily_pad", false),
            ("minecraft:vine", false),
            ("jak:swamp_lily", false),
            ("jak:swamp_moss", false),
            ("minecraft:stone", true),
        ] {
            let scene = Two {
                a: Block::new("minecraft:water"),
                b: Block::new(neighbor),
            };
            assert_eq!(super::hides_fluid(&scene, (16, 0, 0)), hides, "{neighbor}");
        }
    }

    #[test]
    fn water_faces_use_the_translucent_range_and_hide_shared_faces() {
        let texture = ResourceId::parse("minecraft:block/water_still").unwrap();
        let atlas = Atlas {
            pixels: RgbaImage::new(16, 16),
            mipmaps: vec![],
            slots: HashMap::from([(texture.clone(), [0.0, 0.0, 1.0, 1.0])]),
            missing: vec![],
            animated: vec![],
            animated_tiles: vec![],
        };
        let tint = BiomeTint {
            grass: None,
            foliage: None, dry_foliage: None,
        };
        let water_model = test_box_model(&texture, [0.0; 3], [1.0, 8.0 / 9.0, 1.0]);
        let solid_model = test_box_model(&texture, [0.0; 3], [1.0; 3]);
        for neighbor in ["minecraft:water", "minecraft:stone"] {
            let scene = Two {
                a: Block::new("minecraft:water"),
                b: Block::new(neighbor),
            };
            let models = HashMap::from([
                (model_key(&scene.a), vec![(water_model.clone(), 1)]),
                (
                    model_key(&scene.b),
                    vec![(
                        if neighbor == "minecraft:water" {
                            water_model.clone()
                        } else {
                            solid_model.clone()
                        },
                        1,
                    )],
                ),
            ]);
            let light = SkyLight::build(&scene);
            let water = mesh_chunk(&scene, (0, 0), &models, &atlas, &tint, &light).unwrap();
            // The top and the same face turned down (seen from under the
            // water), the bottom, and the three sides not shared.
            assert_eq!(water.faces, 6);
            assert_eq!(water.transparent_start, Some(0));
            assert_eq!(water.indices.len(), water.faces * 6);
            if neighbor == "minecraft:stone" {
                let solid = mesh_chunk(&scene, (1, 0), &models, &atlas, &tint, &light).unwrap();
                assert_eq!(solid.faces, 6);
                assert_eq!(solid.transparent_start, None);
            }
        }
    }
    #[test]
    fn forced_translucent_model_face_uses_blended_range() {
        let texture = ResourceId::parse("minecraft:block/redstone_dust_dot").unwrap();
        let atlas = Atlas {
            pixels: RgbaImage::new(16, 16),
            mipmaps: vec![],
            slots: HashMap::from([(texture.clone(), [0.0, 0.0, 1.0, 1.0])]),
            missing: vec![],
            animated: vec![],
            animated_tiles: vec![],
        };
        let mut model = test_box_model(&texture, [0.0; 3], [1.0; 3]);
        model.elements[0].faces[0].force_translucent = true;
        let mut scene = HandcraftedScene::default();
        scene.set((0, 0, 0), Some(Block::new("minecraft:redstone_wire")));
        let block = scene.block((0, 0, 0)).unwrap();
        let models = HashMap::from([(model_key(block), vec![(model, 1)])]);
        let mesh = mesh_chunk(
            &scene,
            (0, 0),
            &models,
            &atlas,
            &BiomeTint {
                grass: None,
                foliage: None, dry_foliage: None,
            },
            &SkyLight::build(&scene),
        )
        .unwrap();
        assert_eq!(mesh.transparent_start, Some(30));
        assert_eq!(mesh.indices.len(), 36);
    }
    #[test]
    fn flowing_surface_height_and_lava_light_follow_fluid_state() {
        let mut scene = HandcraftedScene::default();
        scene.set(
            (0, 0, 0),
            Some(Block::new("minecraft:water").with("level", "4")),
        );
        scene.set(
            (2, 0, 0),
            Some(Block::new("minecraft:lava").with("level", "0")),
        );
        let ids = ["water_still", "water_flow", "lava_still", "lava_flow"];
        let slots = ids
            .into_iter()
            .map(|name| {
                (
                    ResourceId::parse(&format!("minecraft:block/{name}")).unwrap(),
                    [0.0, 0.0, 1.0, 1.0],
                )
            })
            .collect();
        let atlas = Atlas {
            pixels: RgbaImage::new(16, 16),
            mipmaps: vec![],
            slots,
            missing: vec![],
            animated: vec![],
            animated_tiles: vec![],
        };
        let tint = BiomeTint {
            grass: None,
            foliage: None, dry_foliage: None,
        };
        let light = SkyLight::build(&scene);
        let mut water = ChunkMesh::default();
        let mut transparent = Vec::new();
        append_fluid(
            &scene,
            (0, 0, 0),
            &atlas,
            &tint,
            &light,
            &mut water,
            &mut transparent,
        )
        .unwrap();
        assert!(!transparent.is_empty());
        assert!(water.vertices.iter().all(|v| v.position[1] <= 4.0 / 9.0));
        let mut lava = ChunkMesh::default();
        let mut transparent = Vec::new();
        append_fluid(
            &scene,
            (2, 0, 0),
            &atlas,
            &tint,
            &light,
            &mut lava,
            &mut transparent,
        )
        .unwrap();
        assert!(transparent.is_empty());
        assert!(lava.vertices.iter().all(|v| v.block_light >= 14.0));
        assert!(lava.vertices.iter().any(|v| v.block_light == 15.0));
    }
    #[test]
    fn boundary_neighbors_hide_shared_face() {
        let id = ResourceId::parse("minecraft:block/stone").unwrap();
        let atlas = Atlas {
            pixels: RgbaImage::new(32, 32),
            mipmaps: vec![],
            slots: HashMap::from([(id.clone(), [0.0, 0.0, 1.0, 1.0])]),
            missing: vec![],
            animated: vec![],
            animated_tiles: vec![],
        };
        let model = ResolvedModel {
            elements: vec![crate::model::Element {
                from: [0.0; 3],
                to: [1.0; 3],
                rotation_x: 0,
                rotation_y: 0,
                rotation: None,
                shade_direction_override: None,
                faces: ["down", "up", "north", "south", "west", "east"]
                    .map(|d| crate::model::Face {
                        direction: d.into(),
                        texture: id.clone(),
                        uv: [0.0, 0.0, 1.0, 1.0],
                        cull: true,
                        cullface: None,
                        baked_uv: None,
                        tint: false,
                        tint_index: None,
                        force_translucent: false,
                        triangle: None,
                    })
                    .to_vec(),
            }],
        };
        let scene = Two {
            a: Block::new("minecraft:stone"),
            b: Block::new("minecraft:stone"),
        };
        let models = HashMap::from([(model_key(&scene.a), vec![(model, 1)])]);
        let light = SkyLight::build(&scene);
        assert_eq!(
            mesh_chunk(
                &scene,
                (0, 0),
                &models,
                &atlas,
                &BiomeTint {
                    grass: None,
                    foliage: None, dry_foliage: None
                },
                &light
            )
            .unwrap()
            .faces,
            5
        );
        assert_eq!(
            mesh_chunk(
                &scene,
                (1, 0),
                &models,
                &atlas,
                &BiomeTint {
                    grass: None,
                    foliage: None, dry_foliage: None
                },
                &light
            )
            .unwrap()
            .faces,
            5
        );
    }

    #[test]
    fn glass_hides_shared_faces_while_fancy_leaves_keep_them() {
        let glass = Block::new("minecraft:glass");
        let leaves = Block::new("minecraft:oak_leaves");
        assert!(hides_shared_face(&glass, &glass));
        assert!(!hides_shared_face(&leaves, &leaves));
        assert!(!hides_shared_face(
            &glass,
            &Block::new("minecraft:tinted_glass")
        ));
    }

    #[test]
    fn face_corners_blend_neighbor_shade() {
        let mut scene = HandcraftedScene::new();
        scene.set((-1, 1, -1), Some(Block::new("minecraft:stone")));
        let light = SkyLight::build(&scene);
        let shaded = ambient_vertex(&scene, &light, (0, 1, 0), "up", [0.0, 1.0, 0.0]);
        let open = ambient_vertex(&scene, &light, (0, 1, 0), "up", [1.0, 1.0, 1.0]);
        assert!(shaded.0 < open.0);
    }

    #[test]
    fn cube_vertices_follow_minecraft_faceinfo_order() {
        assert_eq!(
            corners("up", [0.0; 3], [1.0; 3]).unwrap()[0],
            [0.0, 1.0, 0.0]
        );
        assert_eq!(
            corners("down", [0.0; 3], [1.0; 3]).unwrap()[0],
            [0.0, 0.0, 1.0]
        );
        assert_eq!(
            corners("north", [0.0; 3], [1.0; 3]).unwrap()[0],
            [1.0, 1.0, 0.0]
        );
        assert_eq!(
            corners("south", [0.0; 3], [1.0; 3]).unwrap()[0],
            [0.0, 1.0, 1.0]
        );
        assert_eq!(
            corners("west", [0.0; 3], [1.0; 3]).unwrap()[0],
            [0.0, 1.0, 0.0]
        );
        assert_eq!(
            corners("east", [0.0; 3], [1.0; 3]).unwrap()[0],
            [1.0, 1.0, 1.0]
        );
    }

    #[test]
    fn position_seed_selects_grass_rotations() {
        let variants = (0..4)
            .map(|_| (ResolvedModel { elements: vec![] }, 1))
            .collect::<Vec<_>>();
        assert_eq!(variant_index(&variants, (0, 1, 0)), 2);
        assert_eq!(variant_index(&variants, (0, 1, 1)), 3);
        assert_eq!(variant_index(&variants, (-1, 1, 0)), 1);
        assert_eq!(variant_index(&variants, (2_000_000_000, 1, 0)), 1);
        assert_eq!(rotate_y([0.0, 1.0, 0.0], 90), [1.0, 1.0, 0.0]);
        assert_eq!(rotated_direction("north", 90).unwrap(), "east");
    }
}

#[cfg(test)]
mod bridge_uv_regressions {
    use super::*;
    #[test]
    fn plank_slabs_share_the_texture_at_adjoining_world_edges() {
        let id=ResourceId::parse("jak:block/w2_city-slum-wood-plain").unwrap();
        let tile=world_texture(&id).unwrap();
        let corners=[[0.0,1.0,0.0],[1.0,1.0,0.0],[1.0,1.0,1.0],[0.0,1.0,1.0]];
        for x in [-4,-3,0,1,2] {
            let a=world_uvs(([0.0,0.0,1.0,1.0],tile,0),(x,70,0),&corners);
            let b=world_uvs(([0.0,0.0,1.0,1.0],tile,0),(x+1,70,0),&corners);
            for (left,right) in [(a[1],b[0]),(a[2],b[3])] {
                // 1 and 0 are the same edge of the repeating native tile.
                for axis in 0..2 { assert_eq!(left[axis].rem_euclid(1.0),right[axis].rem_euclid(1.0)); }
            }
        }
    }
}

#[cfg(test)] mod chest_world_regressions {
 use super::*;use crate::scene::{Block,HandcraftedScene};
 #[test]fn placed_and_generated_chests_are_present_in_the_world_mesh(){
  let path=std::env::var("MINECRAFTOSS_PACK").expect("native resource pack");
  let packs=PackStack::open(vec![path.into()]).unwrap();let mut scene=HandcraftedScene::new();
  for (i,facing) in ["north","east","south","west"].into_iter().enumerate(){
   scene.set((i as i32*3,2,0),Some(Block::new("minecraft:chest").with("facing",facing).with("type","single").with("waterlogged","false")));
  }
  let built=build(&scene,&packs).unwrap();
  assert!(built.chunks.values().map(|m|m.faces).sum::<usize>()>=48,"a chest cannot be omitted waiting for a missing block-entity renderer");
 }
}

#[cfg(test)]
mod hand_display_cache_tests {
    use super::*;
    #[test]
    fn special_item_transforms_stay_separate_and_reload_with_the_pack() {
        let temp=crate::test_dir::tempdir().unwrap();let root=temp.path();
        std::fs::write(root.join("pack.mcmeta"),r#"{"pack":{"min_format":[97,1],"max_format":[97,1]}}"#).unwrap();
        let item=root.join("assets/test/items/card.json");std::fs::create_dir_all(item.parent().unwrap()).unwrap();
        std::fs::write(item,r#"{"model":{"type":"minecraft:special","base":"test:item/card","model":{"type":"minecraft:banner","color":"white"}}}"#).unwrap();
        let model=root.join("assets/test/models/item/card.json");std::fs::create_dir_all(model.parent().unwrap()).unwrap();
        let model_text=|x|serde_json::json!({"display":{
            "firstperson_righthand":{"translation":[x,2,3]},"firstperson_lefthand":{"translation":[4,5,6]},
            "thirdperson_righthand":{"translation":[7,8,9]}}}).to_string();
        std::fs::write(&model,model_text(1)).unwrap();let mut pack=PackStack::open(vec![root.into()]).unwrap();
        let mut visuals=ItemVisuals::default();
        for (first,left,x,y,z) in [(true,false,1.,2.,3.),(true,true,-4.,5.,6.),(false,false,7.,8.,9.),(false,true,-7.,8.,9.)] {
            let m=visuals.hand_transform("test:card",first,left,&pack).unwrap();
            assert!(m.transform_point3(Vec3::splat(0.5)).distance(Vec3::new(x,y,z)/16.0)<1e-6);
            assert!(m.determinant()>0.99);
        }
        assert_eq!(visuals.hand_displays.len(),1);
        std::fs::write(&model,model_text(9)).unwrap();pack.reload().unwrap();
        let after=visuals.hand_transform("test:card",true,false,&pack).unwrap();
        assert!((after.transform_point3(Vec3::splat(0.5)).x-9./16.).abs()<1e-6);
        visuals.clear();assert!(visuals.hand_displays.is_empty());
    }
}

#[cfg(test)]mod native_tints_regressions {
    use super::*;
    use crate::scene::HandcraftedScene;
    fn source()->BiomeTint {BiomeTint{grass:None,foliage:None,dry_foliage:None}}
    fn rgb(v:u32)->[f32;3] {[(v>>16&255)as f32/255.0,(v>>8&255)as f32/255.0,(v&255)as f32/255.0]}
    #[test]fn native_tints_keep_vessels_petals_and_unregistered_faces_untinted(){
        let mut biome=BiomeSample::THE_VOID;biome.grass_color=Some([11,37,81]);biome.water_color=[31,91,201];let src=source();
        let vessel=Block::new("minecraft:water_cauldron");
        assert_eq!(block_face_tint(&vessel,Some(0),biome,&src),rgb(0x1f5bc9));
        assert_eq!(block_face_tint(&vessel,None,biome,&src),[1.0;3]);
        assert_eq!(block_face_tint(&vessel,Some(1),biome,&src),[1.0;3]);
        for id in["pink_petals","wildflowers"] {let b=Block::new(&format!("minecraft:{id}"));assert_eq!(block_face_tint(&b,Some(0),biome,&src),[1.0;3]);assert_eq!(block_face_tint(&b,Some(1),biome,&src),rgb(0x0b2551));}
        assert_eq!(block_face_tint(&Block::new("minecraft:stone"),Some(0),biome,&src),[1.0;3]);
        assert_eq!(block_face_tint(&Block::new("jak:barrio_hierbajos"),Some(0),biome,&src),rgb(0x0b2551));
    }
    #[test]fn native_tints_use_species_age_and_dry_foliage_instead_of_grass(){
        let mut b=BiomeSample::THE_VOID;b.grass_color=Some([1,2,3]);b.foliage_color=Some([71,91,45]);b.dry_foliage_color=Some([123,83,52]);let src=source();
        for id in["oak_leaves","jungle_leaves","acacia_leaves","dark_oak_leaves","mangrove_leaves","vine"]{assert_eq!(block_face_tint(&Block::new(&format!("minecraft:{id}")),Some(0),b,&src),rgb(0x475b2d));}
        for(id,c)in[("spruce_leaves",0x619961),("birch_leaves",0x80a755),("attached_melon_stem",0xe0c71c),("attached_pumpkin_stem",0xe0c71c),("lily_pad",0x208030),("leaf_litter",0x7b5334)]{let block=Block::new(&format!("minecraft:{id}"));assert_eq!(block_face_tint(&block,Some(0),b,&src),rgb(c));assert_eq!(block_face_tint(&block,None,b,&src),[1.0;3]);}
        for a in 0..8u32{for id in["melon_stem","pumpkin_stem"]{let block=Block::new(&format!("minecraft:{id}")).with("age",&a.to_string());assert_eq!(block_face_tint(&block,Some(0),b,&src),[a as f32*32.0/255.0,(255-a*8)as f32/255.0,a as f32*4.0/255.0]);}}
        let upper=TintKind::of(&Block::new("minecraft:tall_grass").with("half","upper"));assert_eq!(upper.sample_pos((3,64,-7)),(3,63,-7));
        let mut colormap=RgbaImage::from_pixel(256,256,image::Rgba([1,1,1,255]));colormap.put_pixel(127,191,image::Rgba([21,44,67,255]));
        let mut b=BiomeSample::THE_VOID;b.dry_foliage_color=None;let src=BiomeTint{dry_foliage:Some(colormap),..source()};assert_eq!(src.dry_foliage(b),rgb(0x152c43));
    }
    #[test]fn native_tints_keep_cached_and_direct_mesh_vertices_identical(){
        let root=crate::test_dir::tempdir().unwrap();std::fs::write(root.path().join("pack.mcmeta"),r#"{"pack":{"min_format":[97,1],"max_format":[97,1]}}"#).unwrap();
        let packs=PackStack::open(vec![root.path().into()]).unwrap();let tex=ResourceId::parse("minecraft:missingno").unwrap();let atlas=make_atlas(&packs,vec![tex.clone()]).unwrap();
        let face=|dir:&str,index:Option<usize>|crate::model::Face{direction:dir.into(),texture:tex.clone(),uv:[0.0,0.0,1.0,1.0],baked_uv:None,triangle:None,cull:false,cullface:None,tint:index.is_some(),tint_index:index,force_translucent:false};
        let model=ResolvedModel{elements:vec![crate::model::Element{from:[0.0;3],to:[1.0;3],rotation_x:0,rotation_y:0,rotation:None,shade_direction_override:None,faces:vec![face("up",Some(0)),face("north",None),face("east",Some(1))]}]};
        let variants=vec![(model.clone(),1)];let src=source();
        for id in["water_cauldron","wildflowers","birch_leaves","melon_stem","leaf_litter"]{
            let block=Block::new(&format!("minecraft:{id}"));let mut scene=HandcraftedScene::new();scene.set((0,0,0),Some(block.clone()));let light=SkyLight::build(&scene);let quads=bake_quads(&block,&model,&atlas).unwrap();let(mut slow,mut baked)=(ChunkMesh::default(),ChunkMesh::default());let(mut sa,mut ba)=(Vec::new(),Vec::new());
            append_block(&scene,(0,0,0),&block,||Ok(&variants),|_|false,&atlas,&src,&light,&mut slow,&mut sa).unwrap();
            append_baked(&scene,(0,0,0),&quads,TintKind::of(&block),false,|_|false,|p|scene.shade_darkens_at(p),&atlas,&src,&light,&mut baked,&mut ba).unwrap();
            assert_eq!(slow.vertices.len(),12);assert!(bytemuck::cast_slice::<_,u8>(&slow.vertices)==bytemuck::cast_slice::<_,u8>(&baked.vertices),"{id}: cached and direct vertices differ");assert_eq!(slow.indices,baked.indices);
        }
    }
}
