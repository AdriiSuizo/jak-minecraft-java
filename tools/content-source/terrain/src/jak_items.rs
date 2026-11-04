//! Jak's guns, the four colours of ammo (his guns' and MW2's alike, 22/10),
//! dark eco and his vehicles as Minecraft items in a `jak` namespace. Their art is drawn here, pixel by pixel, into a small resource
//! pack (item definitions, `item/generated` models and 16x16 textures) that
//! the pack stack loads beside vanilla's, so icons, dropped items and the
//! held item all go through the normal item pipeline.
use std::path::Path;

use anyhow::Result;
use image::{Rgba, RgbaImage};

pub const SKULL_GEM_ID: &str = "jak:skull_gem";

/// An eco colour: red, yellow, blue, dark.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Eco {
    Red,
    Yellow,
    Blue,
    Dark,
}

impl Eco {
    pub const ALL: [Eco; 4] = [Eco::Red, Eco::Yellow, Eco::Blue, Eco::Dark];

    pub fn name(self) -> &'static str {
        match self {
            Eco::Red => "red",
            Eco::Yellow => "yellow",
            Eco::Blue => "blue",
            Eco::Dark => "dark",
        }
    }

    fn rgb(self) -> [u8; 3] {
        match self {
            Eco::Red => [226, 52, 38],
            Eco::Yellow => [250, 214, 52],
            Eco::Blue => [56, 150, 255],
            Eco::Dark => [150, 64, 205],
        }
    }
}

/// What a Jak item is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum JakKind {
    /// A gun: its index in Jak 3's twelve (`GunKind::ALL`, red 1 to dark 3),
    /// its colour and tier (1..3).
    Gun { index: u8, eco: Eco, tier: u8 },
    /// A stack of a colour's ammo: every gun's (Jak's and MW2's) of that
    /// colour.
    Ammo(Eco),
    /// Raw eco, a crafting material: dark eco only now (the red, yellow and
    /// blue were ammo, and are the colours' ammo items since 22/10).
    EcoCrystal(Eco),
    /// His JET-Board, carried in the off hand.
    Board,
    /// The HellCat, broken down to carry (as a boat is).
    HellCat,
    /// One of Jak 3's zoomers (a hover bike, or a hover car), taken apart
    /// to carry, put down again where Jak stands.
    Zoomer { model: &'static str, car: bool },
    /// Green eco, what Jak 2's citizens drop: fused onto a vehicle's item
    /// it mends a share of its hull.
    GreenEco,
    /// A Metal Head's skull gem, carried and spent through Minecraft slots.
    SkullGem,
    /// An eco vent to put down at a base (Jak 1's red, yellow, blue and
    /// green; Jak 3's dark): its colour, an index into [`VENT_COLORS`].
    Vent(u8),
    /// A Flut Flut's egg (Jak 1's `flutflutegg`): found in swamps and laid
    /// by tame ones; put down, it hatches a chick.
    FlutEgg,
    /// A bucket of dark eco, the liquid (Jak 1's pools): poured, it flows as
    /// lava does.
    DarkEcoBucket,
    /// Jak 2's grind rail, a block (`jak:grind_rail`): laid and joined as
    /// Minecraft's rail is, ridden on the JET-Board.
    GrindRail,
    /// One of the mashup's blocks ([`BLOCK_LOOKS`]: Jak 3's Wasteland's),
    /// put down as Minecraft's blocks are.
    Block,
}

/// How one of the mashup's blocks is drawn: by Minecraft's own model
/// templates, in the original game's textures (by their names in its
/// export).
#[derive(Clone, Copy, Debug)]
pub enum BlockLook {
    All(&'static str),
    BottomTop { top: &'static str, side: &'static str, bottom: &'static str },
    /// A log's: its rings at the ends, upright or laid by its `axis`.
    Column { end: &'static str, side: &'static str },
    /// A plant's crossed planes.
    Cross(&'static str),
    /// Native foliage silhouettes, with stable per-position mesh/rotation variety.
    Plant { texture: &'static str, models: &'static [fn() -> String] },
    /// Leaves: a cube seen through where its texture is clear (untinted).
    Leaves(&'static str),
    /// A slab, its double the full block `double`. Naming itself makes a
    /// full cube in the same texture, without adding another block id.
    Slab { texture: &'static str, double: &'static str },
    /// A cube turned at random about its upright (vanilla's sand: the
    /// same model at 0, 90, 180 and 270 degrees).
    Turned(&'static str),
    /// A cube with its top one of several at random, its sides and bottom
    /// one each.
    Tops { tops: &'static [&'static str], side: &'static str, bottom: &'static str },
    /// Layers as snow lies, eighths of a block (vanilla's `snow_height2` to
    /// `snow_height14`), the eighth the full block `full`.
    Layers { texture: &'static str, full: &'static str },
    /// A cube in one of several textures at random (a model each).
    Varied(&'static [&'static str]),
    /// Hanging as vanilla's vines hang, its faces by its sides (`vine`).
    Vine(&'static str),
    /// Floating as vanilla's lily pad, turned at random (`lily_pad`).
    LilyPad(&'static str),
    /// Drawn as the vanilla block `vanilla` is, by its own blockstate and
    /// models (a door's halves and hinges, stairs' corners, a wall's joins),
    /// each texture key of its models ours (`"*"`: every key not named).
    Like { vanilla: &'static str, textures: &'static [(&'static str, &'static str)] },
    /// Shaped as `model` makes it (a model's boxes, `jak_facades`: a part
    /// of a house), turned and behaving as the vanilla block `vanilla` (its
    /// blockstate's turns, its collision), each texture key of the model
    /// ours.
    Shaped { vanilla: &'static str, model: fn() -> String, textures: &'static [(&'static str, &'static str)] },
    /// Behaving as the vanilla block `vanilla` (its states, its joining its
    /// neighbours), each of its parts (its models, by their names past the
    /// block's: `_post`, `_side`, `_inner`...) drawn as ours (`jak_families`).
    Set { vanilla: &'static str, models: &'static [(&'static str, fn() -> String)], textures: &'static [(&'static str, &'static str)] },
}

impl BlockLook {
    /// The texture its item shows.
    fn face(self) -> &'static str {
        match self {
            Self::All(t) | Self::Cross(t) | Self::Leaves(t) | Self::Turned(t) | Self::Vine(t) | Self::LilyPad(t) => t,
            Self::Varied(ts) => ts[0],
            Self::Plant { texture, .. } => texture,
            Self::BottomTop { side, .. } | Self::Column { side, .. } | Self::Tops { side, .. } => side,
            Self::Slab { texture, .. } | Self::Layers { texture, .. } => texture,
            Self::Like { textures, .. } | Self::Shaped { textures, .. } | Self::Set { textures, .. } => textures[0].1,
        }
    }

    fn textures(self) -> Vec<&'static str> {
        match self {
            Self::All(t) | Self::Cross(t) | Self::Leaves(t) | Self::Turned(t) | Self::Vine(t) | Self::LilyPad(t) => vec![t],
            Self::Varied(ts) => ts.to_vec(),
            Self::Plant { texture, .. } => vec![texture],
            Self::BottomTop { top, side, bottom } => vec![top, side, bottom],
            Self::Column { end, side } => vec![end, side],
            Self::Slab { texture, .. } | Self::Layers { texture, .. } => vec![texture],
            Self::Tops { tops, side, bottom } => tops.iter().copied().chain([side, bottom]).collect(),
            Self::Like { textures, .. } | Self::Shaped { textures, .. } | Self::Set { textures, .. } => {
                let mut all: Vec<&'static str> = textures.iter().map(|(_, t)| *t).collect();
                all.dedup();
                all
            }
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct JakBlock {
    /// Its block's (and item's) path in the `jak` namespace.
    pub path: &'static str,
    pub look: BlockLook,
}

/// The Wasteland's blocks (`MASHUP_BLOCKS` makes each as a vanilla block),
/// in Jak 3's textures of its desert (`desert`, `deserta`, `desertb`) and
/// of Spargus (`wascitya`).
pub const BLOCK_LOOKS: [JakBlock; 77] = [
    JakBlock { path: "wasteland_sand", look: BlockLook::Turned("des-beach-01") },
    JakBlock { path: "wasteland_rock", look: BlockLook::BottomTop { top: "des-mount-02", side: "des-mount-01", bottom: "des-mount-bottom-01" } },
    // Jak 3's five cliff tops (`desertb`, `deserta`, `desertg`), one at
    // random a block.
    JakBlock {
        path: "wasteland_cliff",
        look: BlockLook::Tops {
            tops: &["des-cliff-top-01", "des-cliff-top-02", "des-cliff-top-03", "des-cliff-top-04", "des-cliff-top-05"],
            side: "des-cliff-01",
            bottom: "des-mount-bottom-01",
        },
    },
    JakBlock { path: "red_rock", look: BlockLook::All("des-red-rock-01") },
    JakBlock { path: "dry_grass", look: BlockLook::Cross("des-sand-grass-01") },
    JakBlock { path: "desert_pebbles", look: BlockLook::Cross("des-shrub-pebbles") },
    JakBlock { path: "palm_log", look: BlockLook::Column { end: "des-pole-01", side: "des-palmtree-trunk-02" } },
    JakBlock { path: "palm_leaves", look: BlockLook::Leaves("des-palm-leaf-01") },
    JakBlock { path: "spargus_ground", look: BlockLook::All("wascity-ground-01") },
    JakBlock { path: "spargus_stucco", look: BlockLook::All("wascity-stucco-wall-bleached-01") },
    JakBlock { path: "pieza_spargus", look: BlockLook::Slab { texture: "wstd-rockwall-01", double: "sp_wstd-rockwall-01" } },
    JakBlock { path: "pieza_haven", look: BlockLook::Slab { texture: "city-dirtywood", double: "hv_city-dirtywood" } },
    JakBlock { path: "pieza_haven_torre", look: BlockLook::Slab { texture: "t-palshaft-plate01", double: "hv_t-palshaft-plate01" } },
    JakBlock { path: "pieza_haven_follaje", look: BlockLook::Slab { texture: "city-slum-shrub-overhang", double: "hv_city-slum-shrub-overhang" } },
    JakBlock { path: "pieza_villarrefugio", look: BlockLook::Slab { texture: "vil-beachrock", double: "v1_vil-beachrock" } },
    JakBlock { path: "pieza_montana", look: BlockLook::Slab { texture: "snow-rock-hi", double: "sn_snow-rock-hi" } },
    JakBlock { path: "pieza_pantano", look: BlockLook::Slab { texture: "swp-beachrock", double: "sw_swp-beachrock" } },
    JakBlock { path: "spargus_stucco_slab", look: BlockLook::Slab { texture: "wascity-stucco-wall-bleached-01", double: "spargus_stucco" } },
    JakBlock { path: "spargus_bricks", look: BlockLook::All("wascity-stonewall-bricks") },
    JakBlock { path: "spargus_roof", look: BlockLook::All("wascity-roof-1") },
    JakBlock { path: "spargus_planks", look: BlockLook::All("des-bridge-plank") },
    JakBlock { path: "totem_stone", look: BlockLook::BottomTop { top: "des-totem-stone-trim", side: "des-totem-stone-01", bottom: "des-totem-stone-trim" } },
    JakBlock { path: "wasteland_sand_layer", look: BlockLook::Layers { texture: "des-beach-01", full: "wasteland_sand" } },
    // Where the dunes' sand gives way to the cliff rock (`des-cliff-trans-01`).
    JakBlock { path: "wasteland_cliff_edge", look: BlockLook::All("des-cliff-trans-01") },
    // The scrubby rock round the cliffs' feet (`des-rock-shrub-01`).
    JakBlock { path: "wasteland_scrub_rock", look: BlockLook::All("des-rock-shrub-01") },
    // The Wasteland's caves (`des-cave-wall-01`, `des-cave-floor-01`).
    JakBlock { path: "wasteland_cave_rock", look: BlockLook::All("des-cave-wall-01") },
    JakBlock { path: "wasteland_cave_floor", look: BlockLook::All("des-cave-floor-01") },
    JakBlock { path: "wasteland_rock_layer", look: BlockLook::Layers { texture: "des-mount-02", full: "wasteland_rock" } },
    // Jak 1's Snowy Mountain (`snow-vis-tfrag`, `snow-vis-shrub`) and Boggy
    // Swamp (`swamp-vis-tfrag`, `swamp-vis-shrub`).
    JakBlock { path: "snow_ground", look: BlockLook::Turned("snow-03") },
    JakBlock { path: "snow_turf", look: BlockLook::BottomTop { top: "snow-03", side: "snow-path-sides", bottom: "snow-path" } },
    JakBlock { path: "snow_packed", look: BlockLook::All("snow-path") },
    JakBlock { path: "snow_rock", look: BlockLook::BottomTop { top: "snow-rock-trim", side: "snow-rock", bottom: "snow-rock-hi" } },
    JakBlock { path: "snow_rock_hi", look: BlockLook::All("snow-rock-hi") },
    JakBlock { path: "snow_ice_wall", look: BlockLook::Varied(&["snow-icewall-01", "snow-icewall-02", "snow-icewall-03", "snow-icewall-04", "snow-icewall-05"]) },
    JakBlock { path: "snow_layer", look: BlockLook::Layers { texture: "snow-03", full: "snow_ground" } },
    JakBlock { path: "snow_grass", look: BlockLook::Cross("snow-grassclump") },
    JakBlock { path: "snow_pole", look: BlockLook::Column { end: "snow-woodpolecap", side: "snow-woodpole" } },
    JakBlock { path: "snow_pine", look: BlockLook::Leaves("snow-treesnow") },
    // Boggy Swamp's trees' leaf clumps (`swp-tree-growth`, its shrubs').
    JakBlock { path: "swamp_leaves", look: BlockLook::Leaves("swp-tree-growth") },
    JakBlock { path: "swamp_turf", look: BlockLook::BottomTop { top: "swp-leafyground", side: "swp-mud-trans", bottom: "swp-mud-floor" } },
    JakBlock { path: "swamp_mud", look: BlockLook::Turned("swp-mud-floor") },
    JakBlock { path: "swamp_mud_top", look: BlockLook::Turned("swp-mud-top") },
    JakBlock { path: "swamp_mud_bot", look: BlockLook::All("swp-mud-bot") },
    JakBlock { path: "swamp_stone", look: BlockLook::All("swp-stand-rock") },
    JakBlock { path: "swamp_stonewall", look: BlockLook::All("swp-stonewall") },
    JakBlock { path: "swamp_bark", look: BlockLook::Column { end: "swp-bark-top", side: "swp-bark" } },
    JakBlock { path: "swamp_roots", look: BlockLook::All("swp-bark-roots") },
    JakBlock { path: "swamp_moss", look: BlockLook::Vine("swp-moss-hang") },
    JakBlock { path: "swamp_lily", look: BlockLook::LilyPad("swp-lilly") },
    JakBlock { path: "swamp_grass", look: BlockLook::Cross("swp-grassclump-02") },
    JakBlock { path: "swamp_mud_layer", look: BlockLook::Layers { texture: "swp-mud-top", full: "swamp_mud_top" } },
    JakBlock { path: "swamp_leafy_layer", look: BlockLook::Layers { texture: "swp-leafyground", full: "swamp_turf" } },
    // The house kit (Adrii, 23/10): walls that join as panes do, doors,
    // roof tiles that turn their corners as stairs do, their slabs, beams
    // and floors, in the textures of Boggy Billy's hut (`swamp`), the
    // lurkers' fort on Snowy Mountain (`snow`) and Spargus (`wascitya`).
    JakBlock { path: "swamp_hut_wall", look: BlockLook::Like { vanilla: "cobblestone_wall", textures: &[("*", "swp-hut-wood-01")] } },
    JakBlock { path: "swamp_hut_door", look: BlockLook::Like { vanilla: "oak_door", textures: &[("*", "swp-plankwood-rivets-float")] } },
    JakBlock { path: "swamp_roof_stairs", look: BlockLook::Like { vanilla: "oak_stairs", textures: &[("*", "swp-roof-tile-01")] } },
    JakBlock { path: "swamp_roof", look: BlockLook::All("swp-roof-tile-top") },
    JakBlock { path: "swamp_roof_slab", look: BlockLook::Slab { texture: "swp-roof-tile-top", double: "swamp_roof" } },
    JakBlock { path: "swamp_hut_log", look: BlockLook::Column { end: "swp-hut-wood-01endcap", side: "swp-hut-wood-01" } },
    JakBlock { path: "swamp_planks", look: BlockLook::All("swp-plankwood") },
    JakBlock { path: "snow_fort_wall", look: BlockLook::Like { vanilla: "cobblestone_wall", textures: &[("*", "snow-fortwallbase")] } },
    JakBlock { path: "snow_wood_wall", look: BlockLook::Like { vanilla: "cobblestone_wall", textures: &[("*", "snow-woodpanels")] } },
    JakBlock { path: "snow_gate_door", look: BlockLook::Like { vanilla: "oak_door", textures: &[("*", "snow-gate-wood")] } },
    JakBlock { path: "snow_roof_stairs", look: BlockLook::Like { vanilla: "oak_stairs", textures: &[("*", "snow-metalroof-01")] } },
    JakBlock { path: "snow_roof", look: BlockLook::All("snow-metalroof-02") },
    JakBlock { path: "snow_roof_slab", look: BlockLook::Slab { texture: "snow-metalroof-02", double: "snow_roof" } },
    JakBlock { path: "snow_planks", look: BlockLook::All("snow-wood-01") },
    JakBlock { path: "spargus_stucco_wall", look: BlockLook::Like { vanilla: "cobblestone_wall", textures: &[("*", "wascity-stucco-wall-bleached-01")] } },
    JakBlock { path: "spargus_brick_wall", look: BlockLook::Like { vanilla: "cobblestone_wall", textures: &[("*", "wascity-stonewall-bricks")] } },
    JakBlock { path: "spargus_door", look: BlockLook::Like { vanilla: "oak_door", textures: &[("*", "wascitya-airlock-door")] } },
    JakBlock { path: "spargus_roof_stairs", look: BlockLook::Like { vanilla: "oak_stairs", textures: &[("*", "wascity-roof-1")] } },
    JakBlock { path: "spargus_roof_slab", look: BlockLook::Slab { texture: "wascity-roof-1", double: "spargus_roof" } },
    JakBlock { path: "spargus_pole_log", look: BlockLook::Column { end: "wascity-wood-plain", side: "wascity-metal-pole" } },
    // Detail of the lands: Snowy Mountain's icicles (`snow-fringe-01`) and
    // ice (`snow-ice`), Boggy Swamp's scum on its water (`swp-scum`) and its
    // fan plants' leaves (`swp-plant-leaf`).
    JakBlock { path: "snow_icicle", look: BlockLook::Cross("snow-fringe-01") },
    JakBlock { path: "snow_ice", look: BlockLook::Like { vanilla: "ice", textures: &[("*", "snow-ice")] } },
    JakBlock { path: "swamp_scum", look: BlockLook::LilyPad("swp-scum") },
    JakBlock { path: "swamp_fern", look: BlockLook::Cross("swp-plant-leaf") },
    JakBlock { path: "pieza_haven_circuito", look: BlockLook::Slab { texture: "skatea-metalfloor-01", double: "pieza_haven_circuito" } },
];

/// A block's files in the pack: its blockstate, its models, the textures
/// it uses.
fn write_block(assets: &Path, vanilla: Option<&Path>, block: &JakBlock) -> Result<()> {
    let path = block.path;
    // In lower case: a resource's name may not have capitals, some of the
    // games' textures do (`wascity-steel-bar-HI`; 23/10 the world would not
    // load). The texture is still looked up by its own name.
    let tex = |t: &str| format!("jak:block/{}", t.to_lowercase());
    let single = format!(r#"{{"variants":{{"":{{"model":"jak:block/{path}"}}}}}}"#);
    let (states, models): (String, Vec<(String, String)>) = match block.look {
        BlockLook::All(t) | BlockLook::Leaves(t) => (
            single,
            vec![(path.to_owned(), format!(r#"{{"parent":"minecraft:block/cube_all","textures":{{"all":"{}"}}}}"#, tex(t)))],
        ),
        BlockLook::BottomTop { top, side, bottom } => (
            single,
            vec![(
                path.to_owned(),
                format!(
                    r#"{{"parent":"minecraft:block/cube_bottom_top","textures":{{"top":"{}","side":"{}","bottom":"{}"}}}}"#,
                    tex(top),
                    tex(side),
                    tex(bottom)
                ),
            )],
        ),
        BlockLook::Cross(t) => (
            single,
            vec![(path.to_owned(), format!(r#"{{"parent":"minecraft:block/cross","textures":{{"cross":"{}"}}}}"#, tex(t)))],
        ),
        BlockLook::Plant { texture, models } => {
            let variants: Vec<_> = (0..models.len()).flat_map(|i| (0..4).map(move |r|
                serde_json::json!({"model":format!("jak:block/{path}_{i}"),"y":r*90}))).collect();
            let states=serde_json::json!({"variants":{"":variants}}).to_string();
            let written=models.iter().enumerate().map(|(i,build)| (format!("{path}_{i}"),
                format!(r#"{{"textures":{{"particle":"{0}","plant":"{0}"}},"elements":[{1}]}}"#,tex(texture),build()))).collect();
            (states,written)
        }
        BlockLook::Column { end, side } => (
            format!(
                r#"{{"variants":{{"axis=x":{{"model":"jak:block/{path}_horizontal","x":90,"y":90}},"axis=y":{{"model":"jak:block/{path}"}},"axis=z":{{"model":"jak:block/{path}_horizontal","x":90}}}}}}"#
            ),
            ["cube_column", "cube_column_horizontal"]
                .into_iter()
                .zip([path.to_owned(), format!("{path}_horizontal")])
                .map(|(parent, name)| {
                    (name, format!(r#"{{"parent":"minecraft:block/{parent}","textures":{{"end":"{}","side":"{}"}}}}"#, tex(end), tex(side)))
                })
                .collect(),
        ),
        BlockLook::Turned(t) => (
            format!(
                r#"{{"variants":{{"":[{{"model":"jak:block/{path}"}},{{"model":"jak:block/{path}","y":90}},{{"model":"jak:block/{path}","y":180}},{{"model":"jak:block/{path}","y":270}}]}}}}"#
            ),
            vec![(path.to_owned(), format!(r#"{{"parent":"minecraft:block/cube_all","textures":{{"all":"{}"}}}}"#, tex(t)))],
        ),
        BlockLook::Tops { tops, side, bottom } => (
            format!(
                r#"{{"variants":{{"":[{}]}}}}"#,
                (0..tops.len()).map(|k| format!(r#"{{"model":"jak:block/{path}_{k}"}}"#)).collect::<Vec<_>>().join(",")
            ),
            tops.iter()
                .enumerate()
                .map(|(k, top)| {
                    (
                        format!("{path}_{k}"),
                        format!(
                            r#"{{"parent":"minecraft:block/cube_bottom_top","textures":{{"top":"{}","side":"{}","bottom":"{}"}}}}"#,
                            tex(top),
                            tex(side),
                            tex(bottom)
                        ),
                    )
                })
                .collect(),
        ),
        BlockLook::Layers { texture, full } => (
            format!(
                r#"{{"variants":{{{},"layers=8":{{"model":"jak:block/{full}"}}}}}}"#,
                (1..8).map(|n| format!(r#""layers={n}":{{"model":"jak:block/{path}_height{}"}}"#, 2 * n)).collect::<Vec<_>>().join(",")
            ),
            (1..8)
                .map(|n| {
                    let t = tex(texture);
                    (
                        format!("{path}_height{}", 2 * n),
                        format!(r#"{{"parent":"minecraft:block/snow_height{}","textures":{{"particle":"{t}","texture":"{t}"}}}}"#, 2 * n),
                    )
                })
                .collect(),
        ),
        BlockLook::Varied(ts) => (
            format!(
                r#"{{"variants":{{"":[{}]}}}}"#,
                (0..ts.len()).map(|k| format!(r#"{{"model":"jak:block/{path}_{k}"}}"#)).collect::<Vec<_>>().join(",")
            ),
            ts.iter()
                .enumerate()
                .map(|(k, t)| (format!("{path}_{k}"), format!(r#"{{"parent":"minecraft:block/cube_all","textures":{{"all":"{}"}}}}"#, tex(t))))
                .collect(),
        ),
        // Vanilla's `vine` blockstate, its model ours.
        BlockLook::Vine(t) => {
            let face = |when: &str, turn: &str| format!(r#"{{"apply":{{"model":"jak:block/{path}"{turn}}},"when":{{{when}}}}}"#);
            let bare = r#""east":"false","north":"false","south":"false","up":"false","west":"false""#;
            let parts = [
                (r#""north":"true""#, ""),
                (r#""east":"true""#, r#","uvlock":true,"y":90"#),
                (r#""south":"true""#, r#","uvlock":true,"y":180"#),
                (r#""west":"true""#, r#","uvlock":true,"y":270"#),
                (r#""up":"true""#, r#","uvlock":true,"x":270"#),
            ];
            let mut applies = Vec::new();
            for (when, turn) in parts {
                applies.push(face(when, turn));
                applies.push(face(bare, turn));
            }
            (
                format!(r#"{{"multipart":[{}]}}"#, applies.join(",")),
                vec![(path.to_owned(), format!(r#"{{"parent":"minecraft:block/vine","textures":{{"particle":"{0}","vine":"{0}"}}}}"#, tex(t)))],
            )
        }
        // Vanilla's `lily_pad` blockstate (four turns), its model ours.
        BlockLook::LilyPad(t) => (
            format!(
                r#"{{"variants":{{"":[{{"model":"jak:block/{path}"}},{{"model":"jak:block/{path}","y":90}},{{"model":"jak:block/{path}","y":180}},{{"model":"jak:block/{path}","y":270}}]}}}}"#
            ),
            vec![(path.to_owned(), format!(r#"{{"parent":"minecraft:block/lily_pad","textures":{{"particle":"{0}","texture":"{0}"}}}}"#, tex(t)))],
        ),
        BlockLook::Like { vanilla: source, textures } => {
            let vanilla = vanilla.ok_or_else(|| anyhow::anyhow!("vanilla's pack is not there for {path}'s look ({source})"))?;
            like_vanilla(&vanilla.join("assets/minecraft"), path, source, textures)?
        }
        BlockLook::Shaped { vanilla: source, model, textures } => {
            let vanilla = vanilla.ok_or_else(|| anyhow::anyhow!("vanilla's pack is not there for {path}'s look ({source})"))?;
            let states = std::fs::read_to_string(vanilla.join("assets/minecraft/blockstates").join(format!("{source}.json")))?;
            // Every state's model is ours (the turns stay vanilla's).
            let refs = regex::Regex::new(r"minecraft:block/[a-z0-9_]+").unwrap();
            let states = refs.replace_all(&states, format!("jak:block/{path}").as_str()).into_owned();
            let keys: Vec<String> = textures.iter().map(|(k, t)| format!(r#""{k}":"{}""#, tex(t))).collect();
            let json = format!(r#"{{"textures":{{{},"particle":"{}"}},"elements":[{}]}}"#, keys.join(","), tex(textures[0].1), model());
            (states, vec![(path.to_owned(), json)])
        }
        BlockLook::Set { vanilla: source, models, textures } => {
            let vanilla = vanilla.ok_or_else(|| anyhow::anyhow!("vanilla's pack is not there for {path}'s look ({source})"))?;
            let states = std::fs::read_to_string(vanilla.join("assets/minecraft/blockstates").join(format!("{source}.json")))?;
            let refs = regex::Regex::new(r"minecraft:block/([a-z0-9_]+)").unwrap();
            let keys: Vec<String> = textures.iter().map(|(k, t)| format!(r#""{k}":"{}""#, tex(t))).collect();
            let mut written = Vec::new();
            // Each vanilla part's model ours by the name past the block's.
            for found in refs.captures_iter(&states) {
                let rest = found[1].strip_prefix(source).unwrap_or(&found[1]).to_owned();
                if written.iter().any(|(n, _): &(String, String)| *n == format!("{path}{rest}")) {
                    continue;
                }
                let model = models.iter().find(|(k, _)| *k == rest).ok_or_else(|| anyhow::anyhow!("{path}: no model for {source}'s {}", &found[1]))?.1;
                let json = format!(r#"{{"textures":{{{},"particle":"{}"}},"elements":[{}]}}"#, keys.join(","), tex(textures[0].1), model());
                written.push((format!("{path}{rest}"), json));
            }
            let states = refs
                .replace_all(&states, |c: &regex::Captures| format!("jak:block/{path}{}", c[1].strip_prefix(source).unwrap_or(&c[1])))
                .into_owned();
            (states, written)
        }
        BlockLook::Slab { texture, double } => {
            let full = if double == path { format!("{path}_double") } else { double.to_owned() };
            let mut models: Vec<_> = ["slab", "slab_top"]
                .into_iter()
                .zip([path.to_owned(), format!("{path}_top")])
                .map(|(parent, name)| {
                    let t = tex(texture);
                    (name, format!(r#"{{"parent":"minecraft:block/{parent}","textures":{{"bottom":"{t}","top":"{t}","side":"{t}"}}}}"#))
                })
                .collect();
            if double == path {
                models.push((full.clone(), format!(r#"{{"parent":"minecraft:block/cube_all","textures":{{"all":"{}"}}}}"#, tex(texture))));
            }
            (
            format!(
                r#"{{"variants":{{"type=bottom":{{"model":"jak:block/{path}"}},"type=top":{{"model":"jak:block/{path}_top"}},"type=double":{{"model":"jak:block/{full}"}}}}}}"#
            ),
            models,
            )
        }
    };
    std::fs::write(assets.join("blockstates").join(format!("{path}.json")), states)?;
    for (name, model) in models {
        std::fs::write(assets.join("models/block").join(format!("{name}.json")), model)?;
    }
    for t in block.look.textures() {
        block_texture(t).save(assets.join("textures/block").join(format!("{}.png", t.to_lowercase())))?;
    }
    Ok(())
}

/// Vanilla's own pack (its blockstates and models): beside ours, as the
/// game keeps them, or under `MINECRAFTOSS_ROOT`.
fn vanilla_pack(ours: &Path) -> Option<std::path::PathBuf> {
    let beside = ours.parent().map(|local| local.join("minecraft-26.3"));
    let rooted = std::env::var_os("MINECRAFTOSS_ROOT").map(|root| Path::new(&root).join("resourcepacks/local/minecraft-26.3"));
    [beside, rooted].into_iter().flatten().find(|p| p.join("assets/minecraft/blockstates").is_dir())
}

/// A block drawn as the vanilla block `source` is: its blockstate with its
/// models renamed ours (`minecraft:block/oak_door_top_left` becomes
/// `jak:block/<path>_top_left`), and each model vanilla's with its
/// textures ours by key.
fn like_vanilla(vanilla: &Path, path: &str, source: &str, textures: &[(&str, &str)]) -> Result<(String, Vec<(String, String)>)> {
    let states = std::fs::read_to_string(vanilla.join("blockstates").join(format!("{source}.json")))?;
    let refs = regex::Regex::new(r"minecraft:block/([a-z0-9_]+)").unwrap();
    let ours = |model: &str| match model.strip_prefix(source) {
        Some(rest) => format!("{path}{rest}"),
        None => format!("{path}_{model}"),
    };
    let mut models: Vec<(String, String)> = Vec::new();
    for found in refs.captures_iter(&states) {
        let model = &found[1];
        let name = ours(model);
        if models.iter().any(|(n, _)| *n == name) {
            continue;
        }
        let text = std::fs::read_to_string(vanilla.join("models/block").join(format!("{model}.json")))?;
        let mut json: serde_json::Value = serde_json::from_str(&text)?;
        if let Some(map) = json.get_mut("textures").and_then(|t| t.as_object_mut()) {
            for (key, value) in map.iter_mut() {
                let mine = textures.iter().find(|(k, _)| k == key).or_else(|| textures.iter().find(|(k, _)| *k == "*"));
                if let Some((_, t)) = mine {
                    *value = serde_json::Value::String(format!("jak:block/{}", t.to_lowercase()));
                }
            }
        }
        if source.ends_with("_door") {
            // A native Jak door is one image for the whole door, rather than
            // Minecraft's separate top and bottom images. Resolve the parent's
            // geometry and map each half to its corresponding half of that image.
            let mut parent=json.clone();
            for _ in 0..8 {
                if let Some(elements)=parent.get("elements"){json["elements"]=elements.clone();break;}
                let Some(p)=parent.get("parent").and_then(|p|p.as_str())else{break;};
                let p=p.strip_prefix("minecraft:").unwrap_or(p);
                parent=serde_json::from_str(&std::fs::read_to_string(vanilla.join("models").join(format!("{p}.json")))?)?;
            }
            let offset=if model.contains("_top_"){0.0}else{8.0};
            if let Some(elements)=json.get_mut("elements").and_then(|e|e.as_array_mut()){
                for element in elements {
                    if minecraftoss_core::registries::is_tall_door(path){
                        for key in ["from","to"] {if let Some(p)=element.get_mut(key).and_then(|p|p.as_array_mut()) {
                            p[1]=serde_json::json!(p[1].as_f64().unwrap()*1.5+if model.contains("_top_"){8.0}else{0.0});
                        }}
                        let from=element["from"].as_array().unwrap().iter().map(|v|v.as_f64().unwrap()).collect::<Vec<_>>();
                        let to=element["to"].as_array().unwrap().iter().map(|v|v.as_f64().unwrap()).collect::<Vec<_>>();
                        for axis in [0,2] {if to[axis]-from[axis]<=3.001 {
                            if from[axis]>8.0 {element["from"][axis]=serde_json::json!(to[axis]-1.0);}
                            else{element["to"][axis]=serde_json::json!(from[axis]+1.0);}
                        }}
                    }
                    if let Some(faces)=element.get_mut("faces").and_then(|f|f.as_object_mut()){
                    for face in faces.values_mut(){if let Some(uv)=face.get_mut("uv").and_then(|v|v.as_array_mut()){
                        for k in [1,3]{if let Some(v)=uv[k].as_f64(){uv[k]=serde_json::json!(offset+v*0.5);}}
                    }}
                }}
            }
        }
        models.push((name, json.to_string()));
    }
    let states = refs.replace_all(&states, |c: &regex::Captures| format!("jak:block/{}", ours(&c[1]))).into_owned();
    Ok((states, models))
}

/// Every item of the mashup's: Jak's ([`ITEMS`]) and Spargus's blocks
/// (`spargus_city_blocks::CITY_ITEMS`, one a texture of its levels).
pub fn items() -> impl Iterator<Item = &'static JakItem> {
    ITEMS
        .iter()
        .chain(crate::spargus_city_blocks::CITY_ITEMS.iter())
        .chain(crate::jak_ground::GROUND_ITEMS.iter())
        .chain(crate::jak_objects::object_items().iter())
        .chain(crate::jak_walls::wall_items().iter())
        .chain(crate::jak_facades::FACADE_ITEMS.iter())
        .chain(crate::jak_families::FAMILY_ITEMS.iter())
        .chain(crate::jak_haven::HAVEN_ITEMS.iter())
        .chain(crate::jak_barrio::BARRIO_ITEMS.iter())
        .chain(crate::jak_spargus::SPARG_ITEMS.iter())
        .chain(crate::jak_krimson::ITEMS.iter())
        .chain(crate::jak_tower::ITEMS.iter())
        .chain(crate::jak_city_arena::ITEMS.iter())
        .chain(crate::jak_bar::ITEMS.iter())
        .chain(crate::jak_architecture::ITEMS.iter())
}

/// Every block's look: the lands' and kit's ([`BLOCK_LOOKS`]) and Spargus's.
pub fn block_looks() -> impl Iterator<Item = &'static JakBlock> {
    BLOCK_LOOKS
        .iter()
        .chain(crate::spargus_city_blocks::CITY_BLOCKS.iter())
        .chain(crate::jak_ground::GROUND_BLOCKS.iter())
        .chain(crate::jak_objects::object_blocks().iter())
        .chain(crate::jak_walls::wall_blocks().iter())
        .chain(crate::jak_facades::FACADE_BLOCKS.iter())
        .chain(crate::jak_families::FAMILY_BLOCKS.iter())
        .chain(crate::jak_haven::HAVEN_BLOCKS.iter())
        .chain(crate::jak_barrio::BARRIO_BLOCKS.iter())
        .chain(crate::jak_spargus::SPARG_BLOCKS.iter())
        .chain(crate::jak_krimson::BLOCKS.iter())
        .chain(crate::jak_tower::BLOCKS.iter())
        .chain(crate::jak_city_arena::BLOCKS.iter())
        .chain(crate::jak_bar::BLOCKS.iter())
        .chain(crate::jak_architecture::BLOCKS.iter())
}

/// A block item's icon: its block's face, as small as an item's.
fn block_icon(path: &str) -> Option<RgbaImage> {
    let block = block_looks().find(|b| b.path == path)?;
    Some(image::imageops::resize(&block_texture(block.look.face()), 16, 16, image::imageops::FilterType::Triangle))
}

/// The eco vents' colours: their names and their eco's colour.
pub const VENT_COLORS: [(&str, [u8; 3]); 6] = [
    ("red", [226, 52, 38]),
    ("yellow", [250, 214, 52]),
    ("blue", [56, 150, 255]),
    ("green", [90, 235, 80]),
    ("dark", [150, 64, 205]),
    // Jak 3's light eco (16/10 11:47): it fills Light Jak's meter.
    ("light", [205, 236, 255]),
];

#[derive(Clone, Copy, Debug)]
pub struct JakItem {
    /// The item's path in the `jak` namespace.
    pub path: &'static str,
    /// Its display name.
    pub title: &'static str,
    pub kind: JakKind,
}

impl JakItem {
    pub fn id(&self) -> String {
        format!("jak:{}", self.path)
    }

    pub fn texture(&self) -> String {
        format!("jak:item/{}", self.path)
    }
}

/// Jak's guns (the second red is the mashup's building mode, which a
/// selected block now drives, so it is not an item), the ammo and the ecos.
pub const ZOOMERS: [(&str, &str, bool); 8] = [
    ("bikea", "jak:zoomer_bike", false),
    ("bikeb", "jak:zoomer_bike_b", false),
    ("bikec", "jak:zoomer_bike_c", false),
    ("cara", "jak:zoomer_car", true),
    ("carb", "jak:zoomer_car_b", true),
    ("carc", "jak:zoomer_car_c", true),
    ("toad", "jak:salta_dunas", true),
    ("snake", "jak:arenatiburon", true),
];

pub fn zoomer_model(id: &str) -> Option<(&'static str, bool)> {
    ZOOMERS
        .iter()
        .find(|(_, item, _)| *item == id)
        .map(|(model, _, car)| (*model, *car))
}

pub fn zoomer_item(model: &str) -> Option<&'static str> {
    ZOOMERS
        .iter()
        .find(|(name, _, _)| *name == model)
        .map(|(_, id, _)| *id)
}

pub const ITEMS: [JakItem; 114] = [
    gun("scatter_gun", "Scatter Gun", 0, Eco::Red, 1),
    gun("plasmite_rpg", "Plasmite RPG", 2, Eco::Red, 3),
    gun("blaster", "Blaster", 3, Eco::Yellow, 1),
    gun("beam_reflexor", "Beam Reflexor", 4, Eco::Yellow, 2),
    gun("gyro_burster", "Gyro Burster", 5, Eco::Yellow, 3),
    gun("vulcan_fury", "Vulcan Fury", 6, Eco::Blue, 1),
    gun("arc_wielder", "Arc Wielder", 7, Eco::Blue, 2),
    gun("needle_lazer", "Needle Lazer", 8, Eco::Blue, 3),
    gun("peace_maker", "Peace Maker", 9, Eco::Dark, 1),
    gun("mass_inverter", "Mass Inverter", 10, Eco::Dark, 2),
    gun("super_nova", "Super Nova", 11, Eco::Dark, 3),
    JakItem { path: "red_ammo", title: "Munición roja (escopetas)", kind: JakKind::Ammo(Eco::Red) },
    JakItem { path: "yellow_ammo", title: "Munición amarilla (pistolas, fusiles, francotirador)", kind: JakKind::Ammo(Eco::Yellow) },
    JakItem { path: "blue_ammo", title: "Munición azul (subfusiles, ametralladoras)", kind: JakKind::Ammo(Eco::Blue) },
    JakItem { path: "dark_ammo", title: "Munición morada (explosivos)", kind: JakKind::Ammo(Eco::Dark) },
    JakItem { path: "dark_eco", title: "Eco oscuro", kind: JakKind::EcoCrystal(Eco::Dark) },
    JakItem { path: "jet_board", title: "JET-Board", kind: JakKind::Board },
    JakItem { path: "hellcat", title: "HellCat", kind: JakKind::HellCat },
    JakItem { path: "helldog", title: "HellCat negro (Hell Dog)", kind: JakKind::HellCat },
    JakItem { path: "zoomer_bike", title: "Zoomer moto A",
        kind: JakKind::Zoomer {
            model: "bikea",
            car: false,
        },
    },
    JakItem {
        path: "zoomer_bike_b",
        title: "Zoomer moto B",
        kind: JakKind::Zoomer {
            model: "bikeb",
            car: false,
        },
    },
    JakItem {
        path: "zoomer_bike_c",
        title: "Zoomer moto C",
        kind: JakKind::Zoomer {
            model: "bikec",
            car: false } },
    JakItem { path: "zoomer_car", title: "Zoomer coche A",
        kind: JakKind::Zoomer {
            model: "cara",
            car: true,
        },
    },
    JakItem {
        path: "zoomer_car_b",
        title: "Zoomer coche B",
        kind: JakKind::Zoomer {
            model: "carb",
            car: true,
        },
    },
    JakItem {
        path: "zoomer_car_c",
        title: "Zoomer coche C",
        kind: JakKind::Zoomer {
            model: "carc",
            car: true } },
    JakItem {path:"salta_dunas",title:"Salta Dunas",kind:JakKind::Zoomer{model:"toad",car:true}},
    JakItem {path:"arenatiburon",title:"Arenatiburón",kind:JakKind::Zoomer{model:"snake",car:true}},
    JakItem { path: "green_eco", title: "Eco verde", kind: JakKind::GreenEco },
    JakItem { path: "vent_red", title: "Fuente de eco rojo", kind: JakKind::Vent(0) },
    JakItem { path: "vent_yellow", title: "Fuente de eco amarillo", kind: JakKind::Vent(1) },
    JakItem { path: "vent_blue", title: "Fuente de eco azul", kind: JakKind::Vent(2) },
    JakItem { path: "vent_green", title: "Fuente de eco verde", kind: JakKind::Vent(3) },
    JakItem { path: "vent_dark", title: "Fuente de eco oscuro", kind: JakKind::Vent(4) },
    JakItem { path: "vent_light", title: "Fuente de eco luminoso", kind: JakKind::Vent(5) },
    JakItem { path: "flut_egg", title: "Huevo de Flut Flut", kind: JakKind::FlutEgg },
    JakItem { path: "dark_eco_bucket", title: "Cubo de eco oscuro", kind: JakKind::DarkEcoBucket },
    JakItem { path: "grind_rail", title: "Raíl de grind (turbotabla)", kind: JakKind::GrindRail },
    JakItem { path: "wasteland_sand", title: "Arena del Páramo", kind: JakKind::Block },
    JakItem { path: "wasteland_rock", title: "Roca del Páramo", kind: JakKind::Block },
    JakItem { path: "wasteland_cliff", title: "Acantilado del Páramo", kind: JakKind::Block },
    JakItem { path: "red_rock", title: "Roca roja del Páramo", kind: JakKind::Block },
    JakItem { path: "dry_grass", title: "Hierba seca", kind: JakKind::Block },
    JakItem { path: "desert_pebbles", title: "Guijarros del Páramo", kind: JakKind::Block },
    JakItem { path: "palm_log", title: "Tronco de palmera", kind: JakKind::Block },
    JakItem { path: "palm_leaves", title: "Hojas de palmera", kind: JakKind::Block },
    JakItem { path: "spargus_ground", title: "Suelo de Spargus", kind: JakKind::Block },
    JakItem { path: "spargus_stucco", title: "Estuco de Spargus", kind: JakKind::Block },
    JakItem { path: "pieza_spargus", title: "Pieza de Spargus", kind: JakKind::Block },
    JakItem { path: "pieza_haven", title: "Pieza de Haven", kind: JakKind::Block },
    JakItem { path: "pieza_haven_torre", title: "Pieza de la torre de Haven", kind: JakKind::Block },
    JakItem { path: "pieza_villarrefugio", title: "Pieza de Villarrefugio", kind: JakKind::Block },
    JakItem { path: "pieza_montana", title: "Pieza de Montaña Nevada", kind: JakKind::Block },
    JakItem { path: "pieza_pantano", title: "Pieza de Pantano Boggy", kind: JakKind::Block },
    JakItem { path: "spargus_stucco_slab", title: "Losa de estuco de Spargus", kind: JakKind::Block },
    JakItem { path: "spargus_bricks", title: "Sillares de Spargus", kind: JakKind::Block },
    JakItem { path: "spargus_roof", title: "Chapa de tejado de Spargus", kind: JakKind::Block },
    JakItem { path: "spargus_planks", title: "Tablones del Páramo", kind: JakKind::Block },
    JakItem { path: "totem_stone", title: "Piedra de tótem", kind: JakKind::Block },
    JakItem { path: "wasteland_sand_layer", title: "Capa de arena del Páramo", kind: JakKind::Block },
    JakItem { path: "wasteland_cliff_edge", title: "Borde de acantilado del Páramo", kind: JakKind::Block },
    JakItem { path: "wasteland_scrub_rock", title: "Roca con matorral del Páramo", kind: JakKind::Block },
    JakItem { path: "wasteland_cave_rock", title: "Roca de cueva del Páramo", kind: JakKind::Block },
    JakItem { path: "wasteland_cave_floor", title: "Suelo de cueva del Páramo", kind: JakKind::Block },
    JakItem { path: "wasteland_rock_layer", title: "Capa de roca del Páramo", kind: JakKind::Block },
    JakItem { path: "snow_ground", title: "Nieve de la Montaña Nevada", kind: JakKind::Block },
    JakItem { path: "snow_turf", title: "Suelo nevado", kind: JakKind::Block },
    JakItem { path: "snow_packed", title: "Nieve pisada", kind: JakKind::Block },
    JakItem { path: "snow_rock", title: "Roca nevada", kind: JakKind::Block },
    JakItem { path: "snow_rock_hi", title: "Roca alta nevada", kind: JakKind::Block },
    JakItem { path: "snow_ice_wall", title: "Muro de hielo", kind: JakKind::Block },
    JakItem { path: "snow_layer", title: "Capa de nieve de la Montaña", kind: JakKind::Block },
    JakItem { path: "snow_grass", title: "Mata nevada", kind: JakKind::Block },
    JakItem { path: "snow_pole", title: "Tronco de pino nevado", kind: JakKind::Block },
    JakItem { path: "snow_pine", title: "Ramas nevadas", kind: JakKind::Block },
    JakItem { path: "swamp_leaves", title: "Hojas del pantano", kind: JakKind::Block },
    JakItem { path: "swamp_hut_wall", title: "Pared de la cabaña del pantano", kind: JakKind::Block },
    JakItem { path: "swamp_hut_door", title: "Puerta de la cabaña del pantano", kind: JakKind::Block },
    JakItem { path: "swamp_roof_stairs", title: "Tejas del pantano", kind: JakKind::Block },
    JakItem { path: "swamp_roof", title: "Tejado del pantano", kind: JakKind::Block },
    JakItem { path: "swamp_roof_slab", title: "Losa de tejado del pantano", kind: JakKind::Block },
    JakItem { path: "swamp_hut_log", title: "Viga de la cabaña del pantano", kind: JakKind::Block },
    JakItem { path: "swamp_planks", title: "Tablones del pantano", kind: JakKind::Block },
    JakItem { path: "snow_fort_wall", title: "Muralla del fuerte nevado", kind: JakKind::Block },
    JakItem { path: "snow_wood_wall", title: "Pared de tablas nevada", kind: JakKind::Block },
    JakItem { path: "snow_gate_door", title: "Puerta del fuerte nevado", kind: JakKind::Block },
    JakItem { path: "snow_roof_stairs", title: "Tejado de chapa nevado", kind: JakKind::Block },
    JakItem { path: "snow_roof", title: "Chapa del tejado nevado", kind: JakKind::Block },
    JakItem { path: "snow_roof_slab", title: "Losa de chapa nevada", kind: JakKind::Block },
    JakItem { path: "snow_planks", title: "Tablones nevados", kind: JakKind::Block },
    JakItem { path: "spargus_stucco_wall", title: "Pared de estuco de Spargus", kind: JakKind::Block },
    JakItem { path: "spargus_brick_wall", title: "Pared de ladrillo de Spargus", kind: JakKind::Block },
    JakItem { path: "spargus_door", title: "Puerta de Spargus", kind: JakKind::Block },
    JakItem { path: "spargus_roof_stairs", title: "Tejado de chapa de Spargus", kind: JakKind::Block },
    JakItem { path: "spargus_roof_slab", title: "Losa de tejado de Spargus", kind: JakKind::Block },
    JakItem { path: "spargus_pole_log", title: "Poste de Spargus", kind: JakKind::Block },
    JakItem { path: "snow_icicle", title: "Carámbanos", kind: JakKind::Block },
    JakItem { path: "snow_ice", title: "Hielo de la Montaña Nevada", kind: JakKind::Block },
    JakItem { path: "swamp_scum", title: "Verdín del pantano", kind: JakKind::Block },
    JakItem { path: "swamp_fern", title: "Hoja de abanico del pantano", kind: JakKind::Block },
    JakItem { path: "swamp_turf", title: "Suelo de hojas del pantano", kind: JakKind::Block },
    JakItem { path: "swamp_mud", title: "Barro del pantano", kind: JakKind::Block },
    JakItem { path: "swamp_mud_top", title: "Barro húmedo del pantano", kind: JakKind::Block },
    JakItem { path: "swamp_mud_bot", title: "Barro hondo del pantano", kind: JakKind::Block },
    JakItem { path: "swamp_stone", title: "Roca del pantano", kind: JakKind::Block },
    JakItem { path: "swamp_stonewall", title: "Muro de piedra del pantano", kind: JakKind::Block },
    JakItem { path: "swamp_bark", title: "Tronco del pantano", kind: JakKind::Block },
    JakItem { path: "swamp_roots", title: "Raíces del pantano", kind: JakKind::Block },
    JakItem { path: "swamp_moss", title: "Musgo colgante", kind: JakKind::Block },
    JakItem { path: "swamp_lily", title: "Nenúfar del pantano", kind: JakKind::Block },
    JakItem { path: "swamp_grass", title: "Mata del pantano", kind: JakKind::Block },
    JakItem { path: "swamp_mud_layer", title: "Capa de barro del pantano", kind: JakKind::Block },
    JakItem { path: "swamp_leafy_layer", title: "Capa de hojas del pantano", kind: JakKind::Block },
    JakItem { path: "skull_gem", title: "Esfera calavera", kind: JakKind::SkullGem },
    JakItem { path: "pieza_haven_circuito", title: "Pieza del circuito y torre de Haven", kind: JakKind::Block },
];

const fn gun(path: &'static str, title: &'static str, index: u8, eco: Eco, tier: u8) -> JakItem {
    JakItem {
        path,
        title,
        kind: JakKind::Gun { index, eco, tier },
    }
}

/// The Jak item an id names.
pub fn item(id: &str) -> Option<&'static JakItem> {
    let path = id.strip_prefix("jak:")?;
    items().find(|i| i.path == path)
}

/// Every texture the items use, for the block atlas (dropped and held items).
pub fn textures() -> Vec<String> {
    items().map(JakItem::texture).collect()
}

/// The resource pack's folder name under the packs it sits beside.
pub const PACK_DIR: &str = "jak-items";
/// Bumped when the art changes, so an old pack is drawn again.
const PACK_VERSION: &str = "f53";

/// Draws an item's icon from its model (the host's: the HellCat, the
/// zoomers, the board, the guns); none keeps the pixel art.
pub type IconRenderer = fn(&str) -> Option<RgbaImage>;
static ICONS: std::sync::OnceLock<IconRenderer> = std::sync::OnceLock::new();

/// Sets the renderer item icons are drawn with, where it has a model.
pub fn set_icon_renderer(render: IconRenderer) {
    let _ = ICONS.set(render);
}

/// The original games' textures for the mashup's blocks, by name
/// (`grind_rail`, `grind_rail_post`), when the host can read them.
pub type BlockTextureSource = fn(&str) -> Option<RgbaImage>;
static BLOCK_TEXTURES: std::sync::OnceLock<BlockTextureSource> = std::sync::OnceLock::new();

/// Sets where the mashup's blocks' textures come from.
pub fn set_block_texture_source(source: BlockTextureSource) {
    let _ = BLOCK_TEXTURES.set(source);
}

/// The grind rail's block models: a bar on posts, half a block up (the
/// bar's top at 8/16), straight, round a corner, or up a slope as
/// Minecraft's raised rail is (turned 45 degrees about x, `rescale`).
fn grind_rail_models() -> [(&'static str, String); 4] {
    let bar = |from: [f32; 3], to: [f32; 3], extra: &str| {
        format!(
            r##"{{"from":{from:?},"to":{to:?},{extra}"faces":{{"north":{{"uv":[0,0,16,16],"texture":"#bar"}},"south":{{"uv":[0,0,16,16],"texture":"#bar"}},"east":{{"uv":[0,0,16,16],"texture":"#bar"}},"west":{{"uv":[0,0,16,16],"texture":"#bar"}},"up":{{"uv":[0,0,16,16],"texture":"#bar"}},"down":{{"uv":[0,0,16,16],"texture":"#bar"}}}}}}"##
        )
    };
    let post = |x: f32, z: f32, top: f32| {
        format!(
            r##"{{"from":[{},0,{}],"to":[{},{top},{}],"faces":{{"north":{{"uv":[0,0,4,16],"texture":"#post"}},"south":{{"uv":[0,0,4,16],"texture":"#post"}},"east":{{"uv":[0,0,4,16],"texture":"#post"}},"west":{{"uv":[0,0,4,16],"texture":"#post"}},"up":{{"uv":[0,0,4,4],"texture":"#post"}},"down":{{"uv":[0,0,4,4],"texture":"#post"}}}}}}"##,
            x - 0.75,
            z - 0.75,
            x + 0.75,
            z + 0.75
        )
    };
    let model = |elements: Vec<String>| {
        format!(
            r#"{{"ambientocclusion":false,"textures":{{"particle":"jak:block/grind_rail","bar":"jak:block/grind_rail","post":"jak:block/grind_rail_post"}},"elements":[{}]}}"#,
            elements.join(",")
        )
    };
    let slope = |angle: i32| format!(r#""rotation":{{"origin":[8,7,8],"axis":"x","angle":{angle},"rescale":true}},"#);
    [
        ("grind_rail", model(vec![bar([7.0, 6.0, 0.0], [9.0, 8.0, 16.0], ""), post(8.0, 4.0, 6.0), post(8.0, 12.0, 6.0)])),
        // South and east (as `rail_corner`): from the middle to each.
        (
            "grind_rail_corner",
            model(vec![bar([7.0, 6.0, 7.0], [9.0, 8.0, 16.0], ""), bar([9.0, 6.0, 7.0], [16.0, 8.0, 9.0], ""), post(8.0, 8.0, 6.0)]),
        ),
        // Up to the north (`rail_raised_ne`): the bar's middle a block up
        // over its middle, so its ends meet the flat bars on either side.
        (
            "grind_rail_raised_ne",
            model(vec![bar([7.0, 14.0, 0.0], [9.0, 16.0, 16.0], &slope(45).replace("[8,7,8]", "[8,15,8]")), post(8.0, 12.0, 10.0), post(8.0, 4.0, 18.0)]),
        ),
        // Up to the south (`rail_raised_sw`).
        (
            "grind_rail_raised_sw",
            model(vec![bar([7.0, 14.0, 0.0], [9.0, 16.0, 16.0], &slope(-45).replace("[8,7,8]", "[8,15,8]")), post(8.0, 4.0, 10.0), post(8.0, 12.0, 18.0)]),
        ),
    ]
}

/// The grind rail's blockstates: the models turned as `rail.json` turns
/// the rail's.
const GRIND_RAIL_STATES: &str = r#"{"variants":{
"shape=north_south":{"model":"jak:block/grind_rail"},
"shape=east_west":{"model":"jak:block/grind_rail","y":90},
"shape=ascending_north":{"model":"jak:block/grind_rail_raised_ne"},
"shape=ascending_south":{"model":"jak:block/grind_rail_raised_sw"},
"shape=ascending_east":{"model":"jak:block/grind_rail_raised_ne","y":90},
"shape=ascending_west":{"model":"jak:block/grind_rail_raised_sw","y":90},
"shape=south_east":{"model":"jak:block/grind_rail_corner"},
"shape=south_west":{"model":"jak:block/grind_rail_corner","y":90},
"shape=north_west":{"model":"jak:block/grind_rail_corner","y":180},
"shape=north_east":{"model":"jak:block/grind_rail_corner","y":270}}}"#;

/// A block texture of the original game's, or drawn when it cannot be read:
/// the bar brass with its grooves, the posts brushed steel.
fn block_texture(name: &str) -> RgbaImage {
    // The original metal door has an opaque base plus an environment-map
    // pass. Its source alpha controls that reflection, not holes in the
    // metal. Keep this material role explicit: vegetation still needs alpha.
    if let Some(own) = name.strip_prefix("solid_") {
        let mut image = block_texture(own);
        for pixel in image.pixels_mut() { pixel[3] = 255; }
        return image;
    }
    // A texture laid over the world (`w4_<name>`, `mesh::world_texture`):
    // the game's own at up to 128 texels.
    if let Some(own) = name.strip_prefix('w').filter(|r| r.as_bytes().first().is_some_and(u8::is_ascii_digit) && r.as_bytes().get(1) == Some(&b'_')).map(|r| &r[2..]) {
        if let Some(image) = BLOCK_TEXTURES.get().and_then(|source| source(own)) {
            let side = image.width().min(128);
            return image::imageops::resize(&image, side, side, image::imageops::FilterType::Triangle);
        }
    }
    if let Some(image) = BLOCK_TEXTURES.get().and_then(|source| source(name)) {
        return image::imageops::resize(&image, 32, 32, image::imageops::FilterType::Triangle);
    }
    let mut img = RgbaImage::new(32, 32);
    for y in 0..32 {
        for x in 0..32 {
            let px = if !name.starts_with("grind_rail") {
                shade([196, 170, 120], 0.85 + 0.15 * (((x * 7 + y * 13) % 11) as f32 / 11.0))
            } else if name == "grind_rail" {
                let groove = x % 8 < 1;
                shade([196, 150, 92], if groove { 0.6 } else { 0.9 + 0.2 * ((y as f32 * 0.4).sin() * 0.5 + 0.5) })
            } else {
                shade([172, 176, 180], 0.85 + 0.15 * ((x as f32 * 0.7).sin() * 0.5 + 0.5))
            };
            img.put_pixel(x, y, px);
        }
    }
    img
}

/// Writes the pack into `dir` unless it is already there at this version.
pub fn write_pack(dir: &Path) -> Result<()> {
    // World, item and background atlas loaders may arrive together. A second
    // writer must not remove files while the first is publishing them.
    static WRITER: std::sync::Mutex<()> = std::sync::Mutex::new(());
    let _writer = WRITER.lock().unwrap_or_else(|e| e.into_inner());
    let stamp = dir.join("jak-items.version");
    // Drawn from the models when a renderer is there, so a pack drawn
    // without them is drawn again once they are.
    // With the count of blocks and items in it, so a pack written before
    // blocks were added is written again (23/10: Spargus's were drawn as
    // the vanilla blocks they copy, the pack's version not raised).
    // And with a hash of every block's textures (24/10: a block's texture
    // changed, the pack kept the old one, its count the same).
    let looks = block_looks().fold(0u64, |h, block| {
        block.look.textures().iter().chain(std::iter::once(&block.path)).fold(h, |h, t| {
            t.bytes().fold(h, |h, c| h.wrapping_mul(1_099_511_628_211).wrapping_add(u64::from(c)))
        })
    });
    let counted = format!("{PACK_VERSION}.{}.{}.{looks:x}", block_looks().count(), items().count());
    let version = if ICONS.get().is_some() { format!("{counted}+models") } else { counted };
    if std::fs::read_to_string(&stamp).is_ok_and(|v| v.trim() == version) {
        return Ok(());
    }
    // An interrupted generation must never advertise the old complete version.
    if stamp.is_file() { std::fs::remove_file(&stamp)?; }
    let assets = dir.join("assets").join("jak");
    // Written afresh: a file of an older pack left in it (a texture by a
    // name no longer valid: 23/10) would still be read by the atlas.
    if assets.is_dir() {
        std::fs::remove_dir_all(&assets)?;
    }
    for sub in ["items", "models/item", "textures/item", "blockstates", "models/block", "textures/block"] {
        std::fs::create_dir_all(assets.join(sub))?;
    }
    // The mashup's own blocks (`MASHUP_BLOCKS`): the grind rail.
    std::fs::write(assets.join("blockstates").join("grind_rail.json"), GRIND_RAIL_STATES)?;
    for (name, model) in grind_rail_models() {
        std::fs::write(assets.join("models/block").join(format!("{name}.json")), model)?;
    }
    for name in ["grind_rail", "grind_rail_post"] {
        block_texture(name).save(assets.join("textures/block").join(format!("{name}.png")))?;
    }
    // The Wasteland's.
    let vanilla = vanilla_pack(dir);
    for block in block_looks() {
        write_block(&assets, vanilla.as_deref(), block)?;
    }
    // Native meshes can reference materials absent from their single block
    // icon (an urn's rim, a mug's handle). Publish every material bank to
    // the atlas, including appended prop materials, without renumbering it.
    for texture in native_materials() {
        let file = assets.join("textures/block").join(format!("{}.png", texture.to_lowercase()));
        if !file.is_file() || matches!(texture,"hip-tmetring01"|"hip-tbotblue02") { block_texture(texture).save(file)?; }
    }
    std::fs::write(
        dir.join("pack.mcmeta"),
        r#"{"pack":{"description":"Jak's guns, ammo and eco for the mashup","min_format":[97,1],"max_format":[97,1]}}"#,
    )?;
    for item in items() {
        std::fs::write(
            assets.join("items").join(format!("{}.json", item.path)),
            format!(
                r#"{{"model":{{"type":"minecraft:model","model":"jak:item/{}"}}}}"#,
                item.path
            ),
        )?;
        let parent = match item.kind {
            JakKind::Gun { .. } => "minecraft:item/handheld",
            _ => "minecraft:item/generated",
        };
        let model = if item.path == "barrio_hierbajos" {
            // The original tufts' texture is opaque: their silhouette comes
            // from their triangles, so the held/drop/GUI model must share it.
            r#"{"parent":"jak:block/barrio_hierbajos_4","display":{"gui":{"rotation":[20,35,0],"translation":[0,3,0],"scale":[0.9,0.9,0.9]}}}"#.to_owned()
        } else if item.path == "sparg_matorral" {
            r#"{"parent":"jak:block/sparg_matorral_0","display":{"gui":{"rotation":[20,35,0],"translation":[0,2,0],"scale":[0.9,0.9,0.9]}}}"#.to_owned()
        } else {
            format!(r#"{{"parent":"{parent}","textures":{{"layer0":"jak:item/{}"}}}}"#,item.path)
        };
        std::fs::write(
            assets.join("models/item").join(format!("{}.json", item.path)),
            model,
        )?;
        icon_of(item).save(assets.join("textures/item").join(format!("{}.png", item.path)))?;
    }
    std::fs::write(stamp, version)?;
    Ok(())
}

fn native_materials() -> std::collections::BTreeSet<&'static str> {
    crate::original_places::PLACES.iter().flat_map(crate::original_places::textures_of).collect()
}

fn shade(rgb: [u8; 3], k: f32) -> Rgba<u8> {
    let c = |v: u8| ((f32::from(v) * k).clamp(0.0, 255.0)) as u8;
    Rgba([c(rgb[0]), c(rgb[1]), c(rgb[2]), 255])
}

/// An item's icon, by the one route the pack and the tests both take: a
/// block item wears its block's face, a modelled item (gun, vehicle, board)
/// its render when the renderer is there, and the rest the pixel art below.
fn icon_of(item: &JakItem) -> RgbaImage {
    block_icon(item.path)
        .or_else(|| ICONS.get().and_then(|render| render(item.path)))
        .unwrap_or_else(|| draw(item))
}

/// A 16x16 picture of the item, in vanilla's pixel-art manner.
fn draw(item: &JakItem) -> RgbaImage {
    let mut img = RgbaImage::new(16, 16);
    if item.kind == JakKind::GrindRail {
        // A bar across on two posts, brass on steel.
        for x in 1..15 {
            for y in 6..8 {
                img.put_pixel(x, y, shade([196, 150, 92], if y == 6 { 1.1 } else { 0.8 }));
            }
        }
        for x0 in [3u32, 11] {
            for x in x0..x0 + 2 {
                for y in 8..14 {
                    img.put_pixel(x, y, shade([172, 176, 180], if x == x0 { 1.0 } else { 0.75 }));
                }
            }
        }
        return img;
    }
    match item.kind {
        JakKind::SkullGem => {
            let native=image::open(std::env::var_os("JAKMC_SKULL_ICON").expect("owned skull icon input"))
                .expect("embedded original skull gem HUD texture").into_rgba8();
            let icon=image::imageops::resize(&native,16,8,image::imageops::FilterType::Triangle);
            image::imageops::overlay(&mut img,&icon,0,4);
            return img;
        }
        JakKind::EcoCrystal(_) | JakKind::GreenEco => {
            // A glowing orb with a bright core and a highlight.
            let rgb = match item.kind {
                JakKind::EcoCrystal(eco) => eco.rgb(),
                _ => [90, 235, 80],
            };
            for y in 0..16 {
                for x in 0..16 {
                    let (dx, dy) = (x as f32 - 7.5, y as f32 - 7.5);
                    let d = (dx * dx + dy * dy).sqrt();
                    if d <= 6.0 {
                        let k = 1.35 - d / 6.0 * 0.75;
                        img.put_pixel(x, y, shade(rgb, k));
                    } else if d <= 7.0 {
                        img.put_pixel(x, y, shade(rgb, 0.45));
                    }
                }
            }
            for (x, y) in [(5, 4), (6, 4), (5, 5)] {
                img.put_pixel(x, y, Rgba([255, 255, 255, 255]));
            }
        }
        JakKind::Vent(color) => {
            // A stone vent ring seen from above, its eco glowing in it.
            let rgb = VENT_COLORS[usize::from(color).min(5)].1;
            for y in 0..16 {
                for x in 0..16 {
                    let (dx, dy) = (x as f32 - 7.5, (y as f32 - 7.5) * 1.3);
                    let d = (dx * dx + dy * dy).sqrt();
                    if d <= 3.6 {
                        img.put_pixel(x, y, shade(rgb, 1.4 - d / 3.6 * 0.5));
                    } else if d <= 6.8 {
                        let k = 0.55 + ((x + y) % 3) as f32 * 0.07;
                        img.put_pixel(x, y, Rgba([(150.0 * k) as u8, (140.0 * k) as u8, (125.0 * k) as u8, 255]));
                    }
                }
            }
            // The eco rising from it.
            for (x, y) in [(7, 1), (8, 2), (6, 3), (9, 4)] {
                img.put_pixel(x, y, shade(rgb, 1.3));
            }
        }
        JakKind::Ammo(eco) => {
            // Three upright rounds with coloured tips.
            let rgb = eco.rgb();
            for (i, x0) in [2u32, 6, 10].into_iter().enumerate() {
                let top = 3 + (i as u32 % 2);
                for y in top..14 {
                    for x in x0..x0 + 3 {
                        let pixel = if y < top + 3 {
                            shade(rgb, if x == x0 { 1.2 } else { 0.95 })
                        } else if y == 13 {
                            Rgba([90, 70, 40, 255])
                        } else {
                            let k = if x == x0 { 220 } else if x == x0 + 2 { 120 } else { 175 };
                            Rgba([k, (k as f32 * 0.85) as u8, (k as f32 * 0.5) as u8, 255])
                        };
                        img.put_pixel(x, y, pixel);
                    }
                }
            }
        }
        JakKind::HellCat => {
            // The HellCat from above: a dark blue hull, its grey gun pods
            // on either side and the two thrusters glowing at the tail.
            let hull = |k: f32| shade([46, 70, 132], k);
            for y in 3..13u32 {
                let half = match y {
                    3 => 2,
                    4 => 3,
                    5..=10 => 5,
                    _ => 4,
                };
                for x in (8 - half)..(8 + half) {
                    let rim = x == 8 - half || x == 8 + half - 1 || y == 3 || y == 12;
                    img.put_pixel(x, y, if rim { hull(0.55) } else { hull(if x < 8 { 1.15 } else { 0.95 }) });
                }
            }
            // The cockpit ring.
            for (x, y) in [(7, 6), (8, 6), (6, 7), (9, 7), (6, 8), (9, 8), (7, 9), (8, 9)] {
                img.put_pixel(x, y, Rgba([190, 170, 120, 255]));
            }
            // Gun pods.
            for y in 4..9u32 {
                img.put_pixel(2, y, Rgba([150, 150, 160, 255]));
                img.put_pixel(13, y, Rgba([150, 150, 160, 255]));
            }
            // Thrusters.
            for x in [5u32, 6, 9, 10] {
                img.put_pixel(x, 13, Rgba([120, 200, 255, 255]));
                img.put_pixel(x, 14, Rgba([200, 240, 255, 255]));
            }
        }
        JakKind::Zoomer { car, .. } => {
            // A Haven zoomer from above: a hover bike's narrow body or a
            // car's wide tub, its colour Haven's traffic, the windshield
            // in front and the thruster glowing at the tail.
            let paint = if car { [70, 150, 150] } else { [200, 92, 46] };
            let half_of = |y: u32| -> u32 {
                match (car, y) {
                    (false, 2 | 13) => 1,
                    (false, 3 | 12) => 2,
                    (false, _) => 3,
                    (true, 2 | 13) => 4,
                    (true, _) => 6,
                }
            };
            for y in 2..14u32 {
                let half = half_of(y);
                for x in (8 - half)..(8 + half) {
                    let rim = x == 8 - half || x == 8 + half - 1 || y == 2 || y == 13;
                    img.put_pixel(x, y, if rim { shade(paint, 0.5) } else { shade(paint, if x < 8 { 1.15 } else { 0.92 }) });
                }
            }
            // Windshield.
            for x in 7..9u32 {
                for y in 4..6u32 {
                    img.put_pixel(x, y, Rgba([170, 220, 240, 255]));
                }
            }
            // Fins (bike) or side pods (car).
            let fin = Rgba([110, 110, 120, 255]);
            if car {
                for y in 5..11u32 {
                    img.put_pixel(1, y, fin);
                    img.put_pixel(14, y, fin);
                }
            } else {
                for (x, y) in [(3, 10), (4, 11), (12, 10), (11, 11)] {
                    img.put_pixel(x, y, fin);
                }
            }
            // Thruster.
            for x in 7..9u32 {
                img.put_pixel(x, 14, Rgba([255, 170, 60, 255]));
                img.put_pixel(x, 15, Rgba([255, 230, 140, 255]));
            }
        }
        JakKind::Board => {
            // The JET-Board lying diagonally: a grey-blue deck with a dark
            // rim, its light-blue fan grille in the middle and the orange
            // jets at the tail.
            let deck = |k: f32| shade([96, 112, 138], k);
            for i in 0..12u32 {
                for w in 0..4u32 {
                    let (x, y) = (2 + i + w / 2, 12 - i + w);
                    if x < 16 && y < 16 {
                        let edge = w == 0 || w == 3;
                        img.put_pixel(x, y, if edge { shade([40, 46, 58], 1.0) } else { deck(if w == 1 { 1.25 } else { 1.0 }) });
                    }
                }
            }
            for (x, y) in [(7, 8), (8, 8), (8, 7), (9, 7), (7, 9), (9, 8)] {
                img.put_pixel(x, y, Rgba([120, 200, 255, 255]));
            }
            for (x, y) in [(2, 13), (3, 14), (2, 14), (1, 14)] {
                img.put_pixel(x, y, Rgba([255, 150, 40, 255]));
            }
        }
        // (Drawn above, or by their blocks' faces.)
        JakKind::GrindRail | JakKind::Block => {}
        JakKind::DarkEcoBucket => {
            // A bucket (grey iron, its handle) full of dark eco: deep
            // purple with its light streaks.
            let iron = |k: f32| shade([150, 150, 156], k);
            for y in 5..15u32 {
                let half = 5.5 - (y as f32 - 5.0) * 0.2;
                for x in 0..16u32 {
                    let dx = (x as f32 - 7.5).abs();
                    if dx <= half {
                        let rim = dx > half - 1.0 || y == 14;
                        img.put_pixel(x, y, if rim { iron(0.8 + 0.04 * (14 - y) as f32) } else { iron(1.0) });
                    }
                }
            }
            for x in 3..13u32 {
                let lit = matches!(x, 5 | 6 | 10);
                img.put_pixel(x, 5, if lit { shade([200, 120, 255], 1.1) } else { shade([70, 20, 110], 1.0) });
                img.put_pixel(x, 6, shade([40, 10, 70], 1.0));
            }
            for (x, y) in [(2, 4), (3, 2), (4, 1), (6, 0), (9, 0), (11, 1), (12, 2), (13, 4)] {
                img.put_pixel(x, y, iron(0.7));
            }
        }
        JakKind::FlutEgg => {
            // Jak 1's big speckled egg (`bch-egg`): cream, its blue spots,
            // lit from the upper left.
            for y in 1..16u32 {
                for x in 0..16u32 {
                    let (dx, dy) = ((x as f32 - 7.5) / 5.6, (y as f32 - 8.8) / 6.8);
                    // Narrower toward its top.
                    let dx = dx * (1.0 + 0.18 * (-dy).max(0.0));
                    let d = dx * dx + dy * dy;
                    if d > 1.0 {
                        continue;
                    }
                    let k = 1.25 - 0.45 * d - 0.2 * (dx + dy);
                    let spot = matches!((x, y), (5, 5) | (6, 5) | (9, 7) | (10, 7) | (6, 10) | (7, 10) | (10, 12) | (4, 8));
                    img.put_pixel(x, y, if spot { shade([70, 120, 200], k) } else { shade([236, 222, 186], k) });
                }
            }
        }
        JakKind::Gun { eco, tier, .. } => {
            // A gun held diagonally, as vanilla draws tools: a metal body
            // and grip, and an eco-lit barrel that grows with the tier.
            let rgb = eco.rgb();
            let metal = |k: u8| Rgba([k, k, (f32::from(k) * 1.05).min(255.0) as u8, 255]);
            // Grip (bottom left).
            for (x, y) in [(2, 13), (3, 12), (3, 13), (4, 11), (4, 12), (2, 14), (1, 14)] {
                img.put_pixel(x, y, Rgba([70, 52, 34, 255]));
            }
            // Body.
            for i in 0..5u32 {
                for w in 0..3u32 {
                    let (x, y) = (4 + i + w, 11 - i + w / 2);
                    if x < 16 && y < 16 {
                        img.put_pixel(x, y, metal(if w == 0 { 200 } else { 150 }));
                    }
                }
            }
            // Barrel, longer and brighter with each tier.
            let length = 3 + tier as u32 * 2;
            for i in 0..length {
                let (x, y) = (8 + i, 7u32.saturating_sub(i));
                if x < 16 {
                    img.put_pixel(x, y, shade(rgb, 1.1));
                    if y + 1 < 16 {
                        img.put_pixel(x, y + 1, shade(rgb, 0.7));
                    }
                }
            }
            // A glowing core on the body.
            img.put_pixel(7, 9, shade(rgb, 1.4));
            img.put_pixel(8, 9, shade(rgb, 1.2));
            if tier >= 2 {
                img.put_pixel(6, 10, shade(rgb, 1.2));
            }
            if tier >= 3 {
                img.put_pixel(9, 8, Rgba([255, 255, 255, 255]));
            }
        }
    }
    img
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The pack as the game writes it: every resource it names (models,
    /// textures) a valid resource name, lower case (23/10: a texture of
    /// Spargus with capitals stopped the world from loading), and every
    /// texture it names written.
    #[test]
    fn the_pack_names_only_valid_resources() {
        if vanilla_pack(Path::new(".")).is_none() {
            return;
        }
        let dir = std::env::temp_dir().join(format!("mashup-pack-names-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        write_pack(&dir).unwrap();
        let assets = dir.join("assets/jak");
        for texture in native_materials() {
            assert!(assets.join("textures/block").join(format!("{}.png",texture.to_lowercase())).is_file(),"missing native material {texture}");
        }
        for object in crate::jak_objects::OBJECTS.iter().filter(|o| crate::original_places::street_actor(o.block).is_some()) {
            for texture in object.textures {
                assert!(crate::original_places::embedded_texture(texture).is_some(), "{}: missing original {texture}", object.block);
                assert!(assets.join("textures/block").join(format!("{}.png",texture.to_lowercase())).is_file(),"{}: material absent from the final pack: {texture}",object.block);
            }
        }
        for texture in crate::jak_skate_tower::SKATE_TEXTURES {
            assert!(assets.join("textures/block").join(format!("{}.png",texture.to_lowercase())).is_file(),"missing original circuit material {texture}");
        }
        assert!(assets.join("blockstates/pieza_haven_circuito.json").is_file());
        assert!(assets.join("items/pieza_haven_circuito.json").is_file());
        let packs=crate::pack::PackStack::open(vec![dir.clone()]).unwrap();
        for (facing,rotation) in [("north",0),("east",90),("south",180),("west",270)] {for amount in 1..=4 {
            let block=crate::scene::Block::new("jak:barrio_hierbajos").with("facing",facing).with("segment_amount",&amount.to_string());
            let model=crate::model::resolve_block(&packs,&block).unwrap();
            assert_eq!(model.elements.len(),10*amount,"each portion adds one complete original tuft");
            assert!(model.elements.iter().all(|e|e.rotation_y==rotation&&e.faces.iter().all(|f|f.texture.namespace=="jak"&&f.texture.path=="block/city-slum-cattail-grass")));
        }}
        let shrub_states:serde_json::Value=serde_json::from_str(&std::fs::read_to_string(assets.join("blockstates/sparg_matorral.json")).unwrap()).unwrap();
        assert_eq!(shrub_states["variants"][""].as_array().unwrap().len(),12,"three native silhouettes in four stable rotations");
        for i in 0..3 {
            let model:serde_json::Value=serde_json::from_str(&std::fs::read_to_string(assets.join(format!("models/block/sparg_matorral_{i}.json"))).unwrap()).unwrap();
            let elements=model["elements"].as_array().unwrap();
            assert_eq!(elements.len(),60,"a whole original three-frond shrub");
            let mut lowest=f64::INFINITY;
            for e in elements {let triangle=&e["faces"]["up"]["iw4l_triangle"];
                assert_eq!(triangle["double_sided"],true);
                for v in triangle["positions"].as_array().unwrap() {
                    let (x,y,z)=(v[0].as_f64().unwrap(),v[1].as_f64().unwrap(),v[2].as_f64().unwrap());
                    assert!((0.0..=16.0).contains(&x)&&(0.0..=12.021).contains(&y)&&(0.0..=16.0).contains(&z));
                    lowest=lowest.min(y);
                }
            }
            assert!((lowest-0.02).abs()<1e-5,"native roots rest on the surface");
        }
        let paths=minecraftoss_core::registries::DataPaths::discover().unwrap();
        let registries=minecraftoss_core::Registries::load(&paths).unwrap();
        let shrub=registries.blocks.parse_state("jak:sparg_matorral").unwrap();
        assert!(registries.blocks.collision_boxes(shrub).is_empty(),"foliage must not stop a vehicle or player");
        let shrub_id=crate::pack::ResourceId::parse("jak:sparg_matorral").unwrap();
        assert_eq!(crate::model::resolve_item_model(&packs,&shrub_id).unwrap().unwrap().elements.len(),60);
        if let Some(out)=std::env::var_os("MC_RENDER_OUT") {
            let out=std::path::PathBuf::from(out);std::fs::create_dir_all(&out).unwrap();
            crate::item_icons::item_icon(&packs,"jak:sparg_matorral",256).unwrap().unwrap().save(out.join("spargus_matorral_256.png")).unwrap();
        }
        let plant=crate::pack::ResourceId::parse("jak:barrio_hierbajos").unwrap();
        let held=crate::model::resolve_item_model(&packs,&plant).unwrap().unwrap();
        assert_eq!(held.elements.len(),40,"held and dropped grass use the same original mesh");
        let icon=crate::item_icons::item_icon(&packs,"jak:barrio_hierbajos",32).unwrap().unwrap();
        let opaque=icon.pixels().filter(|p|p[3]>0).count();
        assert!(opaque>8&&opaque<32*32/2,"the icon must show separate grass blades, not an opaque rectangle: {opaque}");
        if let Some(out)=std::env::var_os("MC_RENDER_OUT") {
            let out=std::path::PathBuf::from(out);std::fs::create_dir_all(&out).unwrap();
            icon.save(out.join("haven_hierbajos_icon.png")).unwrap();
            crate::item_icons::item_icon(&packs,"jak:barrio_hierbajos",256).unwrap().unwrap()
                .save(out.join("haven_hierbajos_icon_256.png")).unwrap();
        }
        let slab: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(assets.join("blockstates/barrio_tablon_slab.json")).unwrap()).unwrap();
        for (kind, parent) in [("bottom", "slab"), ("top", "slab_top"), ("double", "cube_all")] {
            let name = slab["variants"][format!("type={kind}")]["model"].as_str().unwrap().strip_prefix("jak:").unwrap();
            let json: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(assets.join("models").join(format!("{name}.json"))).unwrap()).unwrap();
            assert_eq!(json["parent"], format!("minecraft:block/{parent}"));
            assert!(json["textures"].as_object().unwrap().values().all(|t| t == "jak:block/w2_city-slum-wood-plain"), "{kind} keeps the native boards");
        }
        let names = regex::Regex::new(r#""(jak|minecraft):([^"]+)""#).unwrap();
        let valid = regex::Regex::new(r"^[a-z0-9_.\-/]+$").unwrap();
        let mut bad = Vec::new();
        for sub in ["blockstates", "models/block", "models/item", "items"] {
            for entry in std::fs::read_dir(assets.join(sub)).unwrap().flatten() {
                let text = std::fs::read_to_string(entry.path()).unwrap();
                for c in names.captures_iter(&text) {
                    if !valid.is_match(&c[2]) {
                        bad.push(format!("{}: {}:{}", entry.path().display(), &c[1], &c[2]));
                    }
                    if &c[1] == "jak" && c[2].starts_with("block/") && sub == "models/block" && !assets.join("models/block").join(format!("{}.json", &c[2][6..])).is_file()
                        && !assets.join("textures").join(format!("{}.png", &c[2])).is_file()
                    {
                        bad.push(format!("{}: missing jak:{}", entry.path().display(), &c[2]));
                    }
                }
            }
        }
        let _ = std::fs::remove_dir_all(&dir);
        assert!(bad.is_empty(), "{} bad names, first {:?}", bad.len(), &bad[..bad.len().min(5)]);
    }

    #[test]
    fn every_item_draws_and_has_a_unique_path() {
        let mut seen = std::collections::HashSet::new();
        for item in items() {
            assert!(seen.insert(item.path), "{} twice", item.path);
            let img = icon_of(item);
            assert!(img.pixels().any(|p| p[3] > 0), "{} is empty", item.path);
            assert_eq!((img.width(), img.height()), (16, 16), "{} is not an item's size", item.path);
        }
        assert_eq!(item("jak:blaster").map(|i| i.title), Some("Blaster"));
    }
}


#[cfg(test)]mod finish_regressions{
 use super::*;
 #[test]fn one_original_door_image_spans_both_halves_without_repeating(){
  let root=std::env::var_os("MINECRAFTOSS_ROOT").expect("original pack");
  let pack=Path::new(&root).join("resourcepacks/local/minecraft-26.3/assets/minecraft");
  let(_,models)=like_vanilla(&pack,"haven_puerta","oak_door",&[("*","city-slum-door-01")]).unwrap();
  assert_eq!(models.len(),8);
  for(name,json)in models {let v:serde_json::Value=serde_json::from_str(&json).unwrap();let top=name.contains("_top_");
   for e in v["elements"].as_array().unwrap(){for f in e["faces"].as_object().unwrap().values(){for k in [1,3]{let y=f["uv"][k].as_f64().unwrap();assert!(if top{(0.0..=8.0).contains(&y)}else{(8.0..=16.0).contains(&y)},"{name}: {y}");}}}
  }
 }
}
