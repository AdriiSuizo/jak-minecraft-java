//! The loaded registries every engine component shares.

use crate::biome::BiomeRegistry;
use crate::block::{BlockRegistry, BlockStateId, flags};
use crate::chunk::HeightmapKind;
use crate::datapack::DataPack;
use crate::tags::Tags;
use std::path::{Path, PathBuf};

pub struct Registries {
    pub blocks: BlockRegistry,
    pub block_tags: Tags,
    pub biomes: BiomeRegistry,
    pub biome_tags: Tags,
    pub datapack: DataPack,
    /// `minecraft:air` itself, which vanilla heightmap priming skips by identity.
    pub plain_air: BlockStateId,
    /// Per state: which `HeightmapKind`s treat it as opaque (`HeightmapKind::bit`).
    heightmap_masks: Vec<u8>,
    /// Block entity types and defaults, when the catalog has been exported;
    /// without it generation keeps block entities as `DUMMY` tags.
    pub block_entities: Option<crate::block_entity::BlockEntities>,
    /// Entity default tags, when the catalog has been exported; without it
    /// generation creates no entities.
    pub entities: Option<crate::entity_data::EntityCatalog>,
    /// Default item stack sizes and fuels; without the catalog every item
    /// stacks to 64.
    pub items: crate::item::ItemCatalog,
    /// Loot tables and predicates, read from the data pack on first use.
    pub loot: crate::loot::LootTables,
}

/// Default local data locations under the repository root.
pub struct DataPaths {
    pub block_catalog: PathBuf,
    pub datapack: PathBuf,
    /// Optional: block entity defaults (`harness/export_block_entity_catalog.py`).
    pub block_entity_catalog: PathBuf,
    /// Optional: entity defaults (`harness/export_entity_catalog.py`).
    pub entity_catalog: PathBuf,
    /// Optional: default item components (`harness/export_item_catalog.py`).
    pub item_catalog: PathBuf,
}

impl DataPaths {
    pub fn under(root: &Path) -> Self {
        Self {
            block_catalog: root.join("artifacts/block-state-catalog/26.3.json"),
            datapack: root.join("datapacks/local/minecraft-26.3"),
            block_entity_catalog: root.join("artifacts/block-entity-catalog/26.3.json"),
            entity_catalog: root.join("artifacts/entity-catalog/26.3.json"),
            item_catalog: root.join("artifacts/item-catalog/26.3.json"),
        }
    }

    /// Finds the repository root from the current directory or `MINECRAFTOSS_ROOT`.
    pub fn discover() -> Result<Self, String> {
        if let Ok(root) = std::env::var("MINECRAFTOSS_ROOT") {
            return Ok(Self::under(Path::new(&root)));
        }
        let mut dir = std::env::current_dir().map_err(|e| e.to_string())?;
        loop {
            if dir.join("AGENTS.md").is_file() && dir.join("engine").is_dir() {
                return Ok(Self::under(&dir));
            }
            if !dir.pop() {
                return Err("repository root not found; set MINECRAFTOSS_ROOT".into());
            }
        }
    }
}

/// The mashup's own blocks, each made as a vanilla block is (its states,
/// shapes, tags and behaviour), its look its own (the `jak` resource pack):
/// Jak 2's grind rail, laid and joined as Minecraft's rail is, and Jak 3's
/// Wasteland round Spargus (the desert's ground, plants and building).
pub const MASHUP_BLOCKS: [(&str, &str); 275] = [
    // Spargus's pieces (`jak_spargus`), 24/10.
    ("jak:sparg_blanco_pared", "minecraft:cobblestone_wall"),
    ("jak:sparg_arena_pared", "minecraft:cobblestone_wall"),
    ("jak:sparg_gris1_pared", "minecraft:cobblestone_wall"),
    ("jak:sparg_gris2_pared", "minecraft:cobblestone_wall"),
    ("jak:sparg_gris3_pared", "minecraft:cobblestone_wall"),
    ("jak:sparg_almena_pared", "minecraft:cobblestone_wall"),
    ("jak:sparg_almena_arena_pared", "minecraft:cobblestone_wall"),
    ("jak:sparg_blanco_vano", "minecraft:glass_pane"),
    ("jak:sparg_arena_vano", "minecraft:glass_pane"),
    ("jak:sparg_rampa_blanca", "minecraft:carved_pumpkin"),
    ("jak:sparg_rampa_arena", "minecraft:carved_pumpkin"),
    ("jak:sparg_cuerno", "minecraft:carved_pumpkin"),
    ("jak:sparg_olla", "minecraft:carved_pumpkin"),
    ("jak:sparg_costilla", "minecraft:carved_pumpkin"),
    ("jak:sparg_columna", "minecraft:carved_pumpkin"),
    ("jak:sparg_losa", "minecraft:stone"),
    ("jak:sparg_arena", "minecraft:stone"),
    ("jak:sparg_matorral", "minecraft:dead_bush"),
    ("jak:sparg_azotea", "minecraft:smooth_stone_slab"),
    ("jak:barrio_cornisa_pared", "minecraft:cobblestone_wall"),
    ("jak:barrio_ojo", "minecraft:carved_pumpkin"),
    ("jak:barrio_desconchon", "minecraft:carved_pumpkin"),
    ("jak:barrio_pilar_pie", "minecraft:lightning_rod"),
    ("jak:barrio_pilar", "minecraft:lightning_rod"),
    // The Jak II slums' pieces (`jak_barrio`), 24/10.
    ("jak:barrio_base1_pared", "minecraft:cobblestone_wall"),
    ("jak:barrio_base2_pared", "minecraft:cobblestone_wall"),
    ("jak:barrio_base3_pared", "minecraft:cobblestone_wall"),
    ("jak:barrio_ocre_pared", "minecraft:cobblestone_wall"),
    ("jak:barrio_azul_pared", "minecraft:cobblestone_wall"),
    ("jak:barrio_gris_pared", "minecraft:cobblestone_wall"),
    ("jak:barrio_tablas_pared", "minecraft:cobblestone_wall"),
    ("jak:barrio_corona_pared", "minecraft:cobblestone_wall"),
    ("jak:barrio_base_vano", "minecraft:glass_pane"),
    ("jak:barrio_ocre_vano", "minecraft:glass_pane"),
    ("jak:barrio_azul_vano", "minecraft:glass_pane"),
    ("jak:barrio_gris_vano", "minecraft:glass_pane"),
    ("jak:barrio_portal", "minecraft:carved_pumpkin"),
    ("jak:barrio_vigas", "minecraft:carved_pumpkin"),
    ("jak:barrio_entramado", "minecraft:carved_pumpkin"),
    ("jak:barrio_larguero", "minecraft:carved_pumpkin"),
    ("jak:barrio_tubo", "minecraft:carved_pumpkin"),
    ("jak:barrio_codo", "minecraft:carved_pumpkin"),
    ("jak:barrio_desague", "minecraft:carved_pumpkin"),
    ("jak:barrio_toldo", "minecraft:carved_pumpkin"),
    ("jak:barrio_cartel", "minecraft:carved_pumpkin"),
    ("jak:barrio_cartel_b", "minecraft:carved_pumpkin"),
    ("jak:barrio_andamio", "minecraft:carved_pumpkin"),
    ("jak:barrio_zanja_borde", "minecraft:carved_pumpkin"),
    ("jak:barrio_poste", "minecraft:lightning_rod"),
    ("jak:barrio_losa", "minecraft:stone"),
    ("jak:sparg_losa_capa", "minecraft:snow"),
    ("jak:sparg_arena_capa", "minecraft:snow"),
    ("jak:barrio_losa_capa", "minecraft:snow"),
    ("jak:barrio_zanja", "minecraft:stone"),
    ("jak:barrio_azotea", "minecraft:smooth_stone_slab"),
    ("jak:barrio_suelo", "minecraft:stone"),
    // Villa Refugio's pieces (`jak_haven`), 24/10.
    ("jak:haven_mamposteria_pared", "minecraft:cobblestone_wall"),
    ("jak:haven_mamposteria_pared_base", "minecraft:cobblestone_wall"),
    ("jak:haven_entramado_pared", "minecraft:cobblestone_wall"),
    ("jak:haven_viga_pared", "minecraft:cobblestone_wall"),
    ("jak:haven_barrotes_vano", "minecraft:glass_pane"),
    ("jak:haven_celosia_vano", "minecraft:glass_pane"),
    ("jak:haven_baranda", "minecraft:cobblestone_wall"),
    ("jak:haven_bordillo", "minecraft:cobblestone_wall"),
    ("jak:haven_poste_fachada", "minecraft:carved_pumpkin"),
    ("jak:haven_canes", "minecraft:carved_pumpkin"),
    ("jak:haven_tejadillo", "minecraft:carved_pumpkin"),
    ("jak:haven_estandarte", "minecraft:carved_pumpkin"),
    ("jak:haven_brazo_farol", "minecraft:carved_pumpkin"),
    ("jak:haven_cupula_madera", "minecraft:carved_pumpkin"),
    ("jak:haven_chimenea_remate", "minecraft:carved_pumpkin"),
    ("jak:haven_caja", "minecraft:carved_pumpkin"),
    ("jak:haven_toldo_tela", "minecraft:carved_pumpkin"),
    ("jak:haven_puntal", "minecraft:lightning_rod"),
    ("jak:haven_sillar", "minecraft:stone"),
    ("jak:haven_mamposteria", "minecraft:stone"),
    ("jak:haven_tierra", "minecraft:stone"),
    ("jak:haven_tierra_capa", "minecraft:snow"),
    ("jak:haven_mata", "minecraft:dead_bush"),
    ("jak:haven_suelo_madera", "minecraft:stone"),
    ("jak:haven_losa_madera", "minecraft:smooth_stone_slab"),
    // The families of house pieces (`jak_families`), 24/10.
    ("jak:spargus_pared", "minecraft:cobblestone_wall"),
    ("jak:spargus_pared_base", "minecraft:cobblestone_wall"),
    ("jak:spargus_vano", "minecraft:glass_pane"),
    ("jak:haven_pared", "minecraft:cobblestone_wall"),
    ("jak:haven_pared_base", "minecraft:cobblestone_wall"),
    ("jak:haven_vano", "minecraft:glass_pane"),
    ("jak:nieve_pared", "minecraft:cobblestone_wall"),
    ("jak:nieve_pared_base", "minecraft:cobblestone_wall"),
    ("jak:nieve_vano", "minecraft:glass_pane"),
    ("jak:pantano_pared", "minecraft:cobblestone_wall"),
    ("jak:pantano_vano", "minecraft:glass_pane"),
    ("jak:nieve_empalizada", "minecraft:cobblestone_wall"),
    ("jak:pantano_empalizada", "minecraft:cobblestone_wall"),
    ("jak:spargus_empalizada", "minecraft:cobblestone_wall"),
    ("jak:teja_roja", "minecraft:oak_stairs"),
    ("jak:teja_nevada", "minecraft:oak_stairs"),
    ("jak:teja_pantano", "minecraft:oak_stairs"),
    ("jak:teja_chapa", "minecraft:oak_stairs"),
    ("jak:cumbrera_roja", "minecraft:cobblestone_wall"),
    ("jak:cumbrera_nevada", "minecraft:cobblestone_wall"),
    ("jak:cumbrera_pantano", "minecraft:cobblestone_wall"),
    ("jak:cumbrera_chapa", "minecraft:cobblestone_wall"),
    ("jak:haven_adoquin", "minecraft:stone"),
    ("jak:haven_adoquin_capa", "minecraft:snow"),
    ("jak:spargus_losa", "minecraft:smooth_stone_slab"),
    ("jak:nieve_tablas", "minecraft:stone"),
    ("jak:haven_tubo", "minecraft:lightning_rod"),
    ("jak:haven_codo", "minecraft:carved_pumpkin"),
    ("jak:haven_pilar_madera", "minecraft:carved_pumpkin"),
    ("jak:haven_mensula", "minecraft:carved_pumpkin"),
    ("jak:pantano_teja", "minecraft:oak_stairs"),
    ("jak:pantano_cumbrera", "minecraft:oak_slab"),
    ("jak:spargus_torre_curva", "minecraft:carved_pumpkin"),
    // The facade parts of 23/10's second round (`jak_facades`).
    ("jak:spargus_zocalo", "minecraft:carved_pumpkin"),
    ("jak:spargus_remate", "minecraft:carved_pumpkin"),
    ("jak:spargus_ventanuco", "minecraft:carved_pumpkin"),
    ("jak:spargus_contraventana", "minecraft:carved_pumpkin"),
    ("jak:spargus_arco", "minecraft:carved_pumpkin"),
    ("jak:spargus_pilar", "minecraft:lightning_rod"),
    ("jak:spargus_pilar_punta", "minecraft:lightning_rod"),
    ("jak:spargus_torre", "minecraft:carved_pumpkin"),
    ("jak:haven_talud", "minecraft:carved_pumpkin"),
    ("jak:haven_viga_muro", "minecraft:carved_pumpkin"),
    ("jak:haven_esquinero", "minecraft:carved_pumpkin"),
    ("jak:haven_ventanita", "minecraft:carved_pumpkin"),
    ("jak:haven_alero", "minecraft:spruce_trapdoor"),
    ("jak:nieve_estacas", "minecraft:carved_pumpkin"),
    ("jak:nieve_estacas_punta", "minecraft:carved_pumpkin"),
    ("jak:nieve_panel", "minecraft:carved_pumpkin"),
    ("jak:nieve_capa", "minecraft:smooth_quartz_slab"),
    ("jak:nieve_tronco", "minecraft:lightning_rod"),
    ("jak:pantano_estacas", "minecraft:carved_pumpkin"),
    ("jak:pantano_estacas_punta", "minecraft:carved_pumpkin"),
    ("jak:pantano_tronco", "minecraft:carved_pumpkin"),
    ("jak:pantano_poste", "minecraft:lightning_rod"),
    ("jak:pantano_tablon", "minecraft:spruce_slab"),
    ("jak:haven_yeso", "minecraft:stone"),
    // The facade parts of the villages (`jak_facades`): shaped blocks that
    // turn, stand and break as these vanilla ones.
    ("jak:spargus_ventana", "minecraft:carved_pumpkin"),
    ("jak:spargus_esquina", "minecraft:carved_pumpkin"),
    ("jak:spargus_contrafuerte", "minecraft:carved_pumpkin"),
    ("jak:spargus_pinchos", "minecraft:carved_pumpkin"),
    ("jak:spargus_viga", "minecraft:lightning_rod"),
    ("jak:spargus_bidon", "minecraft:carved_pumpkin"),
    ("jak:spargus_cupula", "minecraft:carved_pumpkin"),
    ("jak:spargus_toldo", "minecraft:oak_trapdoor"),
    ("jak:spargus_puerta_metal", "minecraft:oak_door"),
    ("jak:spargus_tuberia", "minecraft:lightning_rod"),
    ("jak:haven_tablas", "minecraft:carved_pumpkin"),
    ("jak:haven_chapa", "minecraft:carved_pumpkin"),
    ("jak:haven_ventana", "minecraft:carved_pumpkin"),
    ("jak:haven_base", "minecraft:carved_pumpkin"),
    ("jak:haven_muro", "minecraft:stone_bricks"),
    ("jak:haven_puerta", "minecraft:oak_door"),
    ("jak:haven_tejado", "minecraft:oak_stairs"),
    ("jak:haven_toldo", "minecraft:oak_trapdoor"),
    ("jak:haven_poste", "minecraft:lightning_rod"),
    ("jak:haven_tuberia", "minecraft:lightning_rod"),
    // Jak ground that joins by itself (jak_ground): a block of earth that
    // neither grass nor a hoe turns into anything else.
    ("jak:suelo_spargus_tierra", "minecraft:packed_mud"),
    ("jak:suelo_spargus_zanja", "minecraft:packed_mud"),
    ("jak:suelo_spargus_piedra", "minecraft:packed_mud"),
    ("jak:suelo_spargus_roca", "minecraft:packed_mud"),
    ("jak:suelo_haven_tierra", "minecraft:packed_mud"),
    ("jak:suelo_haven_zanja", "minecraft:packed_mud"),
    ("jak:suelo_haven_fondo", "minecraft:packed_mud"),
    ("jak:suelo_haven_tablas", "minecraft:packed_mud"),
    ("jak:suelo_villarrefugio_arena", "minecraft:packed_mud"),
    ("jak:suelo_villarrefugio_camino", "minecraft:packed_mud"),
    ("jak:suelo_villarrefugio_hierba", "minecraft:packed_mud"),
    ("jak:suelo_villarrefugio_roca", "minecraft:packed_mud"),
    ("jak:suelo_montana_nieve", "minecraft:packed_mud"),
    ("jak:suelo_montana_camino", "minecraft:packed_mud"),
    ("jak:suelo_montana_roca", "minecraft:packed_mud"),
    ("jak:suelo_montana_hielo", "minecraft:packed_mud"),
    ("jak:suelo_pantano_hojas", "minecraft:packed_mud"),
    ("jak:suelo_pantano_barro", "minecraft:packed_mud"),
    ("jak:suelo_pantano_alquitran", "minecraft:packed_mud"),
    ("jak:suelo_pantano_roca", "minecraft:packed_mud"),
    ("jak:suelo_spargus_tejado", "minecraft:packed_mud"),
    ("jak:suelo_haven_tejado", "minecraft:packed_mud"),
    ("jak:suelo_villarrefugio_tejado", "minecraft:packed_mud"),
    ("jak:suelo_montana_tejado", "minecraft:packed_mud"),
    ("jak:suelo_pantano_tejado", "minecraft:packed_mud"),
    // Jak's walls that join by themselves (`jak_walls`): panes to the
    // world (their joins in their state, their collision the pane's),
    // drawn as metres of their levels' walls.
    ("jak:muro_spargus_estuco", "minecraft:glass_pane"),
    ("jak:muro_spargus_piedra", "minecraft:glass_pane"),
    ("jak:muro_spargus_metal", "minecraft:glass_pane"),
    ("jak:muro_spargus_ventana", "minecraft:glass_pane"),
    ("jak:muro_haven_tablas", "minecraft:glass_pane"),
    ("jak:muro_haven_chapa", "minecraft:glass_pane"),
    ("jak:muro_villarrefugio_madera", "minecraft:glass_pane"),
    ("jak:muro_villarrefugio_piedra", "minecraft:glass_pane"),
    ("jak:muro_montana_madera", "minecraft:glass_pane"),
    ("jak:muro_montana_fuerte", "minecraft:glass_pane"),
    ("jak:muro_pantano_madera", "minecraft:glass_pane"),
    ("jak:muro_pantano_piedra", "minecraft:glass_pane"),
    // Their roofs: whole blocks drawn as metres of their levels' roofs.
    ("jak:tejado_spargus", "minecraft:packed_mud"),
    ("jak:tejado_haven", "minecraft:packed_mud"),
    ("jak:tejado_villarrefugio", "minecraft:packed_mud"),
    ("jak:tejado_montana", "minecraft:packed_mud"),
    ("jak:tejado_pantano", "minecraft:packed_mud"),
    // A metre of a Jak place drawn as its level (jak_pieces): a slab that
    // a hand breaks and keeps, and fire leaves alone.
    ("jak:pieza_spargus", "minecraft:crimson_slab"),
    ("jak:pieza_haven", "minecraft:crimson_slab"),
    ("jak:pieza_villarrefugio", "minecraft:crimson_slab"),
    ("jak:pieza_montana", "minecraft:crimson_slab"),
    ("jak:pieza_pantano", "minecraft:crimson_slab"),
    ("jak:grind_rail", "minecraft:rail"),
    ("jak:wasteland_sand", "minecraft:sand"),
    ("jak:wasteland_rock", "minecraft:sandstone"),
    ("jak:wasteland_cliff", "minecraft:sandstone"),
    ("jak:red_rock", "minecraft:terracotta"),
    ("jak:dry_grass", "minecraft:dead_bush"),
    ("jak:desert_pebbles", "minecraft:dead_bush"),
    ("jak:palm_log", "minecraft:jungle_log"),
    ("jak:palm_leaves", "minecraft:jungle_leaves"),
    ("jak:spargus_ground", "minecraft:packed_mud"),
    ("jak:spargus_stucco", "minecraft:smooth_sandstone"),
    ("jak:spargus_stucco_slab", "minecraft:smooth_sandstone_slab"),
    ("jak:spargus_bricks", "minecraft:stone_bricks"),
    ("jak:spargus_roof", "minecraft:iron_block"),
    ("jak:spargus_planks", "minecraft:oak_planks"),
    ("jak:totem_stone", "minecraft:chiseled_sandstone"),
    // The Wasteland's dunes in eighths of a block, as snow lies (its
    // layers, shapes and collision; it does not melt: see `random_tick`).
    ("jak:wasteland_sand_layer", "minecraft:snow"),
    ("jak:wasteland_cliff_edge", "minecraft:sandstone"),
    ("jak:wasteland_scrub_rock", "minecraft:sandstone"),
    ("jak:wasteland_cave_rock", "minecraft:stone"),
    ("jak:wasteland_cave_floor", "minecraft:stone"),
    // Its rock in eighths too, over the rocky rises.
    ("jak:wasteland_rock_layer", "minecraft:snow"),
    // Jak 1's Snowy Mountain and Boggy Swamp (`snowy`, `swamp`): their
    // ground, rock, ice, plants and trees, as the vanilla blocks they stand
    // for behave.
    ("jak:snow_ground", "minecraft:snow_block"),
    ("jak:snow_turf", "minecraft:snow_block"),
    ("jak:snow_packed", "minecraft:snow_block"),
    ("jak:snow_rock", "minecraft:stone"),
    ("jak:snow_rock_hi", "minecraft:stone"),
    ("jak:snow_ice_wall", "minecraft:packed_ice"),
    ("jak:snow_layer", "minecraft:snow"),
    ("jak:snow_grass", "minecraft:dead_bush"),
    ("jak:snow_pole", "minecraft:spruce_log"),
    ("jak:snow_pine", "minecraft:spruce_leaves"),
    ("jak:swamp_turf", "minecraft:dirt"),
    ("jak:swamp_mud", "minecraft:dirt"),
    ("jak:swamp_mud_top", "minecraft:mud"),
    ("jak:swamp_mud_bot", "minecraft:clay"),
    ("jak:swamp_stone", "minecraft:stone"),
    ("jak:swamp_stonewall", "minecraft:stone"),
    ("jak:swamp_bark", "minecraft:oak_log"),
    ("jak:swamp_roots", "minecraft:mangrove_roots"),
    ("jak:swamp_moss", "minecraft:vine"),
    ("jak:swamp_lily", "minecraft:lily_pad"),
    ("jak:swamp_grass", "minecraft:dead_bush"),
    ("jak:swamp_mud_layer", "minecraft:snow"),
    ("jak:swamp_leafy_layer", "minecraft:snow"),
    ("jak:swamp_leaves", "minecraft:oak_leaves"),
    // The house kit: walls join as vanilla's walls do, doors open and
    // close, roof tiles turn corners as stairs (`jak_items::BLOCK_LOOKS`).
    ("jak:swamp_hut_wall", "minecraft:cobblestone_wall"),
    ("jak:swamp_hut_door", "minecraft:oak_door"),
    ("jak:swamp_roof_stairs", "minecraft:oak_stairs"),
    ("jak:swamp_roof", "minecraft:oak_planks"),
    ("jak:swamp_roof_slab", "minecraft:oak_slab"),
    ("jak:swamp_hut_log", "minecraft:oak_log"),
    ("jak:swamp_planks", "minecraft:oak_planks"),
    ("jak:snow_fort_wall", "minecraft:cobblestone_wall"),
    ("jak:snow_wood_wall", "minecraft:cobblestone_wall"),
    ("jak:snow_gate_door", "minecraft:spruce_door"),
    ("jak:snow_roof_stairs", "minecraft:spruce_stairs"),
    ("jak:snow_roof", "minecraft:spruce_planks"),
    ("jak:snow_roof_slab", "minecraft:spruce_slab"),
    ("jak:snow_planks", "minecraft:spruce_planks"),
    ("jak:spargus_stucco_wall", "minecraft:sandstone_wall"),
    ("jak:spargus_brick_wall", "minecraft:stone_brick_wall"),
    ("jak:spargus_door", "minecraft:oak_door"),
    ("jak:spargus_roof_stairs", "minecraft:oak_stairs"),
    ("jak:spargus_roof_slab", "minecraft:oak_slab"),
    ("jak:spargus_pole_log", "minecraft:oak_log"),
    // The lands' detail: icicles hang as hanging roots, the ice is ice,
    // the scum floats as a lily pad, the fan leaves grow as ferns.
    ("jak:snow_icicle", "minecraft:hanging_roots"),
    ("jak:snow_ice", "minecraft:ice"),
    ("jak:swamp_scum", "minecraft:lily_pad"),
    ("jak:swamp_fern", "minecraft:fern"),
];

// Append after the existing clone tables, preserving every saved state id.
const MASHUP_EXTENSION_BLOCKS: [(&str, &str); 110] = [
    ("jak:barrio_tablon_slab", "minecraft:oak_slab"),
    ("jak:haven_puerta_alta", "minecraft:oak_door"),
    ("jak:spargus_puerta_alta", "minecraft:oak_door"),
    ("jak:barrio_portal_doble", "minecraft:carved_pumpkin"),
    ("jak:sparg_portal_doble", "minecraft:carved_pumpkin"),
    ("jak:objeto_haven_bidon_1", "minecraft:crimson_slab"),
    ("jak:objeto_haven_bidon_2", "minecraft:crimson_slab"),
    ("jak:krimson_blindaje", "minecraft:carved_pumpkin"),
    ("jak:krimson_franja", "minecraft:carved_pumpkin"),
    ("jak:krimson_suelo", "minecraft:iron_block"),
    ("jak:krimson_reja", "minecraft:iron_bars"),
    ("jak:krimson_porton_fence_gate", "minecraft:oak_fence_gate"),
    ("jak:krimson_terminal", "minecraft:carved_pumpkin"),
    ("jak:krimson_luz", "minecraft:sea_lantern"),
    ("jak:krimson_puesto", "minecraft:carved_pumpkin"),
    ("jak:krimson_insignia", "minecraft:carved_pumpkin"),
    ("jak:krimson_dintel", "minecraft:carved_pumpkin"),
    ("jak:hv_citywide-projector", "minecraft:stone"),
    ("jak:hv_citywide-securitywall-yellow", "minecraft:stone"),
    ("jak:objeto_haven_puesto", "minecraft:crimson_slab"),
    ("jak:objeto_haven_propaganda", "minecraft:crimson_slab"),
    ("jak:jetboard_entrada", "minecraft:iron_block"),
    ("jak:hv_t-citywide-wall-tile-01", "minecraft:stone"),
    ("jak:hv_t-palshaft-plate01", "minecraft:iron_block"),
    ("jak:hv_citywide-dmnd-01", "minecraft:stone"),
    ("jak:hv_citywide-palace-01", "minecraft:stone"),
    ("jak:hv_citywide-window-litwindow", "minecraft:stone"),
    ("jak:hv_ctywide-ox-met-01", "minecraft:stone"),
    ("jak:hv_t-cab", "minecraft:stone"),
    ("jak:hv_t-citypal-met-bm-red-strp01", "minecraft:stone"),
    ("jak:hv_t-citypal-met-pill-01", "minecraft:stone"),
    ("jak:hv_t-citypal-met-strp01", "minecraft:stone"),
    ("jak:hv_t-citypal-red-met-01", "minecraft:stone"),
    ("jak:hv_t-citywide-met-bm-red-strp01", "minecraft:stone"),
    ("jak:hv_t-citywide-met-pill-01", "minecraft:stone"),
    ("jak:hv_t-citywide-met-strp01", "minecraft:stone"),
    ("jak:hv_t-citywide-met-wall-02", "minecraft:stone"),
    ("jak:hv_t-citywide-red-met-01", "minecraft:stone"),
    ("jak:hv_t-palshaft-dirt-blue-01", "minecraft:stone"),
    ("jak:hv_t-palshaft-panl-01", "minecraft:stone"),
    ("jak:hv_t-palshaft-pil-01", "minecraft:stone"),
    ("jak:hv_t-palshaft-r-strp-plate01", "minecraft:stone"),
    ("jak:hv_t-palshaft-roof-01", "minecraft:stone"),
    ("jak:barrio_hierbajos", "minecraft:leaf_litter"),
    ("jak:pieza_haven_torre", "minecraft:crimson_slab"),
    ("jak:pieza_haven_follaje", "minecraft:crimson_slab"),
    ("jak:objeto_haven_colgante_1", "minecraft:crimson_slab"),
    ("jak:objeto_haven_colgante_2", "minecraft:crimson_slab"),
    ("jak:objeto_haven_colgante_3", "minecraft:crimson_slab"),
    ("jak:pieza_haven_circuito", "minecraft:crimson_slab"),
    // Append-only: existing numeric states in saved chunks keep their ids.
    ("jak:objeto_haven_propaganda_ajustada", "minecraft:crimson_slab"),
    ("jak:objeto_haven_propaganda_bloque", "minecraft:crimson_slab"),
    ("jak:barrio_tubo_horizontal", "minecraft:carved_pumpkin"),
    ("jak:pieza_haven_arena", "minecraft:crimson_slab"),
    ("jak:arena_suelo", "minecraft:stone"),
    ("jak:arena_borde", "minecraft:stone"),
    ("jak:arena_losa", "minecraft:stone"),
    ("jak:pieza_haven_torre_circuito", "minecraft:crimson_slab"),
    ("jak:arena_luz_placa", "minecraft:sea_lantern"),
    ("jak:palacio_detalle_columna", "minecraft:stone"),
    ("jak:palacio_detalle_cornisa", "minecraft:stone"),
    ("jak:palacio_suelo_metal", "minecraft:stone"),
    ("jak:palacio_suelo_metal_losa", "minecraft:stone_slab"),
    ("jak:palacio_suelo_metal_medio_vertical", "minecraft:carved_pumpkin"),
    ("jak:palacio_suelo_metal_escalera", "minecraft:stone_stairs"),
    ("jak:palacio_suelo_piedra", "minecraft:stone"),
    ("jak:palacio_suelo_piedra_losa", "minecraft:stone_slab"),
    ("jak:palacio_suelo_piedra_medio_vertical", "minecraft:carved_pumpkin"),
    ("jak:palacio_suelo_piedra_escalera", "minecraft:stone_stairs"),
    ("jak:palacio_suelo_marmol", "minecraft:stone"),
    ("jak:palacio_suelo_marmol_losa", "minecraft:stone_slab"),
    ("jak:palacio_suelo_marmol_medio_vertical", "minecraft:carved_pumpkin"),
    ("jak:palacio_suelo_marmol_escalera", "minecraft:stone_stairs"),
    ("jak:palacio_suelo_panel", "minecraft:stone"),
    ("jak:palacio_suelo_panel_losa", "minecraft:stone_slab"),
    ("jak:palacio_suelo_panel_medio_vertical", "minecraft:carved_pumpkin"),
    ("jak:palacio_suelo_panel_escalera", "minecraft:stone_stairs"),
    ("jak:palacio_pared_azul", "minecraft:stone"),
    ("jak:palacio_pared_azul_losa", "minecraft:stone_slab"),
    ("jak:palacio_pared_azul_medio_vertical", "minecraft:carved_pumpkin"),
    ("jak:palacio_pared_estuco", "minecraft:stone"),
    ("jak:palacio_pared_estuco_losa", "minecraft:stone_slab"),
    ("jak:palacio_pared_estuco_medio_vertical", "minecraft:carved_pumpkin"),
    ("jak:objeto_spargus_urna_alta", "minecraft:stone"),
    ("jak:objeto_spargus_urna_baja", "minecraft:stone"),
    ("jak:objeto_spargus_caja_mercado", "minecraft:stone"),
    ("jak:garaje_salta_dunas", "minecraft:carved_pumpkin"),
    ("jak:garaje_arenatiburon", "minecraft:carved_pumpkin"),
    ("jak:altar_turbotabla", "minecraft:stone"),
    ("jak:altar_turbotabla_vacio", "minecraft:stone"),
    ("jak:bar_suelo", "minecraft:stone"),
    ("jak:bar_pared", "minecraft:stone"),
    ("jak:bar_mostrador", "minecraft:stone"),
    ("jak:bar_mesa", "minecraft:stone"),
    ("jak:bar_taburete", "minecraft:stone"),
    ("jak:bar_rotulo", "minecraft:stone"),
    ("jak:bar_luz", "minecraft:sea_lantern"),
    ("jak:objeto_haven_botella_bar", "minecraft:stone"),
    ("jak:objeto_haven_jarra_bar", "minecraft:stone"),
    ("jak:objeto_haven_bandeja_bar", "minecraft:stone"),
    ("jak:palacio_ventana_luminosa", "minecraft:sea_lantern"),
    ("jak:pieza_haven_rampa_modular", "minecraft:stone_slab"),
    ("jak:palacio_techo_escalera", "minecraft:stone_stairs"),
    ("jak:palacio_pilar_escalera", "minecraft:stone_stairs"),
    ("jak:objeto_spargus_palmera_completa", "minecraft:crimson_slab"),
    ("jak:palacio_emblema", "minecraft:carved_pumpkin"),
    ("jak:luz_suelo_jak", "minecraft:sea_lantern"),
    ("jak:luz_neon_jak", "minecraft:sea_lantern"),
    ("jak:objeto_haven_bidon_fuego", "minecraft:crimson_slab"),
    ("jak:pieza_haven_fuego", "minecraft:crimson_slab"),
];

pub fn is_tall_door(id:&str)->bool {
    matches!(id.rsplit(':').next().unwrap_or(id),"haven_puerta_alta"|"spargus_puerta_alta")
}

impl Registries {
    pub fn load(paths: &DataPaths) -> Result<Self, String> {
        let mut blocks = BlockRegistry::load(&paths.block_catalog).map_err(|e| {
            format!(
                "{e}\nExport it with harness/export_block_state_catalog.py (see harness/README.md)."
            )
        })?;
        for (name, from) in MASHUP_BLOCKS.iter().chain(crate::mashup_city_blocks::MASHUP_CITY_BLOCKS.iter()).chain(crate::mashup_object_blocks::MASHUP_OBJECT_BLOCKS.iter()).chain(MASHUP_EXTENSION_BLOCKS.iter()) {
            blocks.add_clone(from, name)?;
        }
        for name in ["jak:haven_puerta_alta","jak:spargus_puerta_alta"] {blocks.adapt_tall_door(name,"minecraft:oak_door")?;}
        blocks.adapt_security_gate("jak:krimson_porton_fence_gate")?;
        for name in ["jak:altar_turbotabla","jak:altar_turbotabla_vacio"] {
            blocks.adapt_model_boxes(name,&[[0.,0.,0.,16.,3.,16.],[3.,3.,3.,13.,10.,13.],[1.,10.,1.,15.,12.,15.]])?;
        }
        blocks.adapt_model_boxes("jak:bar_mesa",&[[0.,12.,0.,16.,16.,16.],[6.,1.,6.,10.,12.,10.],[3.,0.,3.,13.,1.,13.]])?;
        blocks.adapt_model_boxes("jak:bar_taburete",&[[2.,8.,2.,14.,10.,14.],[6.,1.,6.,10.,8.,10.],[3.,0.,3.,13.,1.,13.]])?;
        blocks.adapt_solid_panel("jak:muro_haven_chapa")?;
        blocks.adapt_light_emission("jak:pieza_haven_fuego",14)?;
        blocks.adapt_model_boxes("jak:luz_neon_jak",&[[1.,12.,3.,15.,16.,13.],[2.,11.5,5.,14.,12.,11.]])?;
        blocks.adapt_model_boxes("jak:bar_luz",&[[0.,12.,0.,16.,16.,16.]])?;
        blocks.adapt_model_boxes("jak:arena_luz_placa",&[[0.0,8.0,0.0,16.0,16.0,16.0]])?;
        blocks.adapt_model_boxes("jak:palacio_detalle_columna",&[[2.,0.,2.,14.,2.,14.],[4.,2.,4.,12.,14.,12.],[2.,14.,2.,14.,16.,14.]])?;
        blocks.adapt_model_boxes("jak:palacio_detalle_cornisa",&[[0.,8.,3.,16.,12.,13.],[0.,12.,1.,16.,16.,15.]])?;
        for name in ["jak:palacio_suelo_metal_medio_vertical","jak:palacio_suelo_piedra_medio_vertical","jak:palacio_suelo_marmol_medio_vertical","jak:palacio_suelo_panel_medio_vertical","jak:palacio_pared_azul_medio_vertical","jak:palacio_pared_estuco_medio_vertical"] {
            blocks.adapt_model_boxes(name,&[[0.,0.,0.,16.,16.,8.]])?;
        }
        blocks.adapt_model_boxes("jak:barrio_zanja_borde",&crate::mashup_shapes::haven_ditch_rim())?;
        let datapack = DataPack::open(&paths.datapack)
            .map_err(|e| format!("{e}\nImport it with python tools/import_datapack.py."))?;
        let mut block_tags = Tags::load(&datapack, "block", blocks.block_count(), |id| {
            blocks.block_by_name(id.as_str()).map(|b| usize::from(b.0))
        })?;
        // The mashup's blocks in their vanilla blocks' tags.
        for (name, from) in MASHUP_BLOCKS.iter().chain(crate::mashup_city_blocks::MASHUP_CITY_BLOCKS.iter()).chain(crate::mashup_object_blocks::MASHUP_OBJECT_BLOCKS.iter()).chain(MASHUP_EXTENSION_BLOCKS.iter()) {
            if let (Some(to), Some(from)) = (blocks.block_by_name(name), blocks.block_by_name(from)) {
                block_tags.copy_membership(usize::from(from.0), usize::from(to.0));
            }
        }
        let biomes = BiomeRegistry::load(&datapack)?;
        let biome_tags = Tags::load(&datapack, "worldgen/biome", biomes.len(), |id| {
            biomes.id(id.as_str()).map(|b| usize::from(b.0))
        })?;
        let plain_air = blocks.parse_state("minecraft:air")?;
        let motion = block_tags.require("minecraft:blocks_motion_in_heightmap")?;
        let motion_no_leaves =
            block_tags.require("minecraft:blocks_motion_in_heightmap_no_leaves")?;
        let heightmap_masks = (0..blocks.state_count())
            .map(|i| {
                let state = BlockStateId(i as u16);
                let info = blocks.state(state);
                let block = usize::from(info.block.0);
                let fluid = info.fluid.is_some();
                let mut mask = 0;
                if !info.has(flags::AIR) {
                    mask |= HeightmapKind::WorldSurfaceWg.bit() | HeightmapKind::WorldSurface.bit();
                }
                if block_tags.contains(motion, block) {
                    mask |= HeightmapKind::OceanFloorWg.bit() | HeightmapKind::OceanFloor.bit();
                }
                if block_tags.contains(motion, block) || fluid {
                    mask |= HeightmapKind::MotionBlocking.bit();
                }
                if block_tags.contains(motion_no_leaves, block) || fluid {
                    mask |= HeightmapKind::MotionBlockingNoLeaves.bit();
                }
                mask
            })
            .collect();
        Ok(Self {
            blocks,
            block_tags,
            biomes,
            biome_tags,
            datapack,
            plain_air,
            heightmap_masks,
            block_entities: paths
                .block_entity_catalog
                .is_file()
                .then(|| crate::block_entity::BlockEntities::load(&paths.block_entity_catalog))
                .transpose()?,
            entities: paths.entity_catalog.is_file().then(|| crate::entity_data::EntityCatalog::load(&paths.entity_catalog)).transpose()?,
            loot: crate::loot::LootTables::default(),
            items: if paths.item_catalog.is_file() { crate::item::ItemCatalog::load(&paths.item_catalog)? } else { crate::item::ItemCatalog::default() },
        })
    }

    pub fn heightmap_mask(&self, state: BlockStateId) -> u8 {
        self.heightmap_masks[usize::from(state.0)]
    }

    /// Whether a biome is in a biome tag.
    pub fn biome_in_tag(&self, biome: crate::BiomeId, tag: crate::tags::TagId) -> bool {
        self.biome_tags.contains(tag, usize::from(biome.0))
    }

    /// Whether a block state's block is in a tag, e.g. `minecraft:logs`.
    pub fn block_in_tag(&self, state: BlockStateId, tag: crate::tags::TagId) -> bool {
        self.block_tags
            .contains(tag, usize::from(self.blocks.block_of(state).0))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chunk::{Chunk, HeightmapKind};
    use crate::pos::ChunkPos;

    /// Loads the real local data; skipped with a note when it has not been restored.
    pub(crate) fn local() -> Option<Registries> {
        let paths = DataPaths::discover().ok()?;
        if !paths.block_catalog.is_file() || !paths.datapack.is_dir() {
            eprintln!("skipping: local vanilla data not restored");
            return None;
        }
        Some(Registries::load(&paths).expect("local vanilla data loads"))
    }

    #[test]
    fn real_catalog_and_datapack_load() {
        let Some(r) = local() else { return };
        let paths = DataPaths::discover().unwrap();
        let vanilla = BlockRegistry::load(&paths.block_catalog).unwrap();
        assert_eq!(vanilla.state_count(), 35_723);
        assert_eq!(vanilla.block_count(), 1_286);
        // The runtime adds Jak's clones to the vanilla catalog. Count their
        // actual source states, retaining the vanilla ids underneath them.
        let clones: std::collections::BTreeMap<_, _> = MASHUP_BLOCKS.iter()
            .chain(crate::mashup_city_blocks::MASHUP_CITY_BLOCKS.iter())
            .chain(crate::mashup_object_blocks::MASHUP_OBJECT_BLOCKS.iter())
            .chain(MASHUP_EXTENSION_BLOCKS.iter()).copied().collect();
        let added: usize = clones.values().map(|from| {
            vanilla.block(vanilla.block_by_name(from).expect("vanilla clone source")).states().count()
        }).sum();
        assert_eq!(r.blocks.state_count(), vanilla.state_count() + added);
        assert_eq!(r.blocks.block_count(), vanilla.block_count() + clones.len());
        for (_, block) in vanilla.blocks() {
            let kept = r.blocks.block(r.blocks.block_by_name(block.name.as_str()).unwrap());
            assert_eq!(kept.default_state(), block.default_state());
        }
        let leaves = r
            .blocks
            .parse_state("minecraft:oak_leaves[distance=3,persistent=true,waterlogged=true]")
            .unwrap();
        assert_eq!(
            r.blocks.state_to_string(leaves),
            "minecraft:oak_leaves[distance=3,persistent=true,waterlogged=true]"
        );
        let stone = r.blocks.parse_state("minecraft:stone").unwrap();
        let water = r.blocks.parse_state("minecraft:water").unwrap();
        let mask = |s| r.heightmap_mask(s);
        assert_eq!(mask(stone), 0b11_1111);
        assert_eq!(mask(water) & HeightmapKind::OceanFloor.bit(), 0);
        assert_ne!(mask(water) & HeightmapKind::MotionBlocking.bit(), 0);
        let leaves_tag = r.block_tags.require("minecraft:leaves").unwrap();
        assert!(r.block_in_tag(leaves, leaves_tag));
        // Waterlogged leaves hold water, so vanilla's fluid clause still counts them.
        assert_ne!(
            mask(leaves) & HeightmapKind::MotionBlockingNoLeaves.bit(),
            0
        );
        let dry_leaves = r
            .blocks
            .with_property(leaves, "waterlogged", "false")
            .unwrap();
        assert_eq!(
            mask(dry_leaves) & HeightmapKind::MotionBlockingNoLeaves.bit(),
            0
        );
        assert_ne!(mask(dry_leaves) & HeightmapKind::MotionBlocking.bit(), 0);
        assert_eq!(r.biomes.len(), 67);
        assert!(r.biome_tags.id("minecraft:is_ocean").is_some());
    }

    #[test]
    fn heightmaps_follow_vanilla_update_rules() {
        let Some(r) = local() else { return };
        let stone = r.blocks.parse_state("minecraft:stone").unwrap();
        let mut chunk = Chunk::new(ChunkPos::new(0, 0), -64, 384, crate::biome::BiomeId(0));
        for y in -64..10 {
            chunk.set_block_raw(3, y, 4, stone, &r);
        }
        chunk.prime_heightmaps(&HeightmapKind::FINAL, &r);
        assert_eq!(chunk.heightmaps.get(HeightmapKind::WorldSurface, 3, 4), 10);
        assert_eq!(chunk.heightmaps.get(HeightmapKind::WorldSurface, 0, 0), -64);
        // Removing the top block rescans; removing a buried block is ignored.
        chunk.set_block_raw(3, 9, 4, BlockStateId::AIR, &r);
        assert!(chunk.update_heightmap(
            HeightmapKind::WorldSurface,
            3,
            9,
            4,
            BlockStateId::AIR,
            &r
        ));
        assert_eq!(chunk.heightmaps.get(HeightmapKind::WorldSurface, 3, 4), 9);
        assert!(!chunk.update_heightmap(
            HeightmapKind::WorldSurface,
            3,
            0,
            4,
            BlockStateId::AIR,
            &r
        ));
    }
}
