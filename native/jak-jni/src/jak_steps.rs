//! Jak's footsteps as `effect-control.gc` plays them: his animations' own
//! frames fire them (`effect-walk-step-left` and the rest, read from his art
//! groups), and the ground under him picks the sound
//! (`sound-name-with-material`: `walk-grs1` for the left foot on grass).
//!
//! Which frames fire is `effect-control-method-9`'s rule: the channel's
//! artist frame since the last one, in the direction its playback function
//! moves (wrapping for a loop), or exactly the frame a new animation starts
//! on. The walk cycle reads its walk channel, or the run channel once the
//! run is mixed in past half (`target-walk-anim` sets `channel-offset` 3).
//!
//! The ground is a Minecraft block (or water) mapped to the nearest of the
//! game's materials. Where Jak 3 has no sound for one (snow, ice, straw,
//! mud: none of its levels has them) Jak 1's is played, as the earlier game
//! names it; where neither has one, the nearest material that has.
use std::sync::atomic::{AtomicU8, Ordering};

use jak_core::anim::{Anim, AnimEffect, EffectKind, NumFunc, Target};
use jak_core::Jak;

/// What Jak walks on, as the game's `pat-material`s go.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[repr(u8)]
pub enum Floor {
    /// `unknown`: a ground nothing says anything about.
    #[default]
    Unknown,
    Stone,
    Grass,
    Dirt,
    Gravel,
    Sand,
    Wood,
    /// `crwood`: crate wood (barrels, chests, crafting tables).
    CrateWood,
    Metal,
    /// `shmetal`: sheet metal (copper, cauldrons, hoppers).
    SheetMetal,
    /// `grmetal`: grating (iron bars and trapdoors, copper grates).
    GrateMetal,
    /// `pcmetal`: Precursor metal.
    PcMetal,
    /// `forest`: leaves and vines.
    Forest,
    /// `neutral`: wool and carpets.
    Neutral,
    /// `squish`: slime and honey.
    Squish,
    /// `mhshroom`: mushroom blocks and the Nether's fungi.
    Mushroom,
    Straw,
    Snow,
    /// `deepsnow`: powder snow.
    DeepSnow,
    Ice,
    /// `swamp`: mud.
    Swamp,
    /// Water up to his shins.
    Water,
    /// Water up to his waist.
    DeepWater,
}

impl Floor {
    const ALL: [Floor; 23] = [
        Floor::Unknown,
        Floor::Stone,
        Floor::Grass,
        Floor::Dirt,
        Floor::Gravel,
        Floor::Sand,
        Floor::Wood,
        Floor::CrateWood,
        Floor::Metal,
        Floor::SheetMetal,
        Floor::GrateMetal,
        Floor::PcMetal,
        Floor::Forest,
        Floor::Neutral,
        Floor::Squish,
        Floor::Mushroom,
        Floor::Straw,
        Floor::Snow,
        Floor::DeepSnow,
        Floor::Ice,
        Floor::Swamp,
        Floor::Water,
        Floor::DeepWater,
    ];

    pub fn from_u8(v: u8) -> Floor {
        Floor::ALL.get(usize::from(v)).copied().unwrap_or_default()
    }

    /// The sounds this ground makes.
    pub fn sounds(self) -> &'static FloorSounds {
        &SOUNDS[self as usize]
    }
}

/// One ground's sounds: the walk's and the run's left and right foot, the
/// landing, the roll and the slide, and the Flut Flut's landing (Jak 1's).
#[derive(Debug)]
pub struct FloorSounds {
    pub walk: [&'static str; 2],
    pub run: [&'static str; 2],
    pub land: &'static str,
    pub roll: &'static str,
    pub slide: &'static str,
    pub flut_land: &'static str,
}

const fn s(
    walk: [&'static str; 2],
    run: [&'static str; 2],
    land: &'static str,
    roll: &'static str,
    slide: &'static str,
    flut_land: &'static str,
) -> FloorSounds {
    FloorSounds {
        walk,
        run,
        land,
        roll,
        slide,
        flut_land,
    }
}

/// By [`Floor`], in its order. Every name is a file of the games' own
/// banks (Jak 3's `COMMON`, Jak 1's `COMMON` and `SNOW`).
const SOUNDS: [FloorSounds; 23] = [
    // Unknown: Jak 3 has no roll nor slide on it; stone's.
    s(["WALK_UNK1", "WALK_UNK2"], ["RUN_UNK1", "RUN_UNK2"], "LAND_UNK", "ROLL_STN", "SLIDE_STN", "jak1:flut-land-stone"),
    s(["WALK_STN1", "WALK_STN2"], ["RUN_STN1", "RUN_STN2"], "LAND_STN", "ROLL_STN", "SLIDE_STN", "jak1:flut-land-stone"),
    s(["WALK_GRS1", "WALK_GRS2"], ["RUN_GRS1", "RUN_GRS2"], "LAND_GRS", "ROLL_GRS", "SLIDE_GRS", "jak1:flut-land-grass"),
    s(["WALK_DRT1", "WALK_DRT2"], ["RUN_DRT1", "RUN_DRT2"], "LAND_DRT", "ROLL_DRT", "SLIDE_DRT", "jak1:flut-land-dirt"),
    // Gravel: Jak 3 runs and rolls on it; it walks, lands and slides on dirt.
    s(["WALK_DRT1", "WALK_DRT2"], ["RUN_GRV1", "RUN_GRV2"], "LAND_DRT", "ROLL_GRV", "SLIDE_DRT", "jak1:flut-land-dirt"),
    s(["WALK_SAN1", "WALK_SAN2"], ["RUN_SAN1", "RUN_SAN2"], "LAND_SAN", "ROLL_SAN", "SLIDE_SAN", "jak1:flut-land-sand"),
    s(["WALK_WOD1", "WALK_WOD2"], ["RUN_WOD1", "RUN_WOD2"], "LAND_WOD", "ROLL_WOD", "SLIDE_WOD", "jak1:flut-land-wood"),
    s(["WALK_CWD1", "WALK_CWD2"], ["RUN_CWD1", "RUN_CWD2"], "LAND_CWD", "ROLL_CWD", "SLIDE_CWD", "jak1:flut-land-crwood"),
    // The metals: Jak 3 slides on Precursor metal only.
    s(["WALK_MTL1", "WALK_MTL2"], ["RUN_MTL1", "RUN_MTL2"], "LAND_MTL", "ROLL_MTL", "SLIDE_PMT", "jak1:flut-land-pcmeta"),
    s(["WALK_SMT1", "WALK_SMT2"], ["RUN_SMT1", "RUN_SMT2"], "LAND_SMT", "ROLL_SMT", "SLIDE_PMT", "jak1:flut-land-pcmeta"),
    s(["WALK_GMT1", "WALK_GMT2"], ["RUN_GMT1", "RUN_GMT2"], "LAND_GMT", "ROLL_GMT", "SLIDE_PMT", "jak1:flut-land-pcmeta"),
    s(["WALK_PMT1", "WALK_PMT2"], ["RUN_PMT1", "RUN_PMT2"], "LAND_PMT", "ROLL_PMT", "SLIDE_PMT", "jak1:flut-land-pcmeta"),
    s(["WALK_FOR1", "WALK_FOR2"], ["RUN_FOR1", "RUN_FOR2"], "LAND_FOR", "ROLL_FOR", "SLIDE_FOR", "jak1:flut-land-grass"),
    // Neutral, squish and mushroom: no roll in Jak 3 (neutral no slide); dirt's.
    s(["WALK_NEU1", "WALK_NEU2"], ["RUN_NEU1", "RUN_NEU2"], "LAND_NEU", "ROLL_DRT", "SLIDE_DRT", "jak1:flut-land-dirt"),
    s(["WALK_SQI1", "WALK_SQI2"], ["RUN_SQI1", "RUN_SQI2"], "LAND_SQI", "ROLL_DRT", "SLIDE_SQI", "jak1:flut-land-swamp"),
    s(["WALK_MHS1", "WALK_MHS2"], ["RUN_MHS1", "RUN_MHS2"], "LAND_MHS", "ROLL_DRT", "SLIDE_MHS", "jak1:flut-land-dirt"),
    // Straw: Jak 3 lands, rolls and slides on it; Jak 1 walks.
    s(
        ["jak1:walk-straw1", "jak1:walk-straw2"],
        ["jak1:walk-straw1", "jak1:walk-straw2"],
        "LAND_STR",
        "ROLL_STR",
        "SLIDE_STR",
        "jak1:flut-land-straw",
    ),
    // Snow, deep snow, ice and mud: Jak 1's (its runs play its walks).
    s(
        ["jak1:walk-snow1", "jak1:walk-snow2"],
        ["jak1:walk-snow1", "jak1:walk-snow2"],
        "jak1:land-snow",
        "jak1:roll-snow",
        "jak1:slide-snow",
        "jak1:flut-land-snow",
    ),
    s(
        ["jak1:walk-dpsnow1", "jak1:walk-dpsnow2"],
        ["jak1:walk-dpsnow1", "jak1:walk-dpsnow2"],
        "jak1:land-dpsnow",
        "jak1:roll-dpsnow",
        "jak1:slide-dpsnow",
        "jak1:flut-land-snow",
    ),
    // Ice: Jak 1 only steps on it; it lands, rolls and slides on stone.
    s(
        ["jak1:walk-ice1", "jak1:walk-ice2"],
        ["jak1:walk-ice1", "jak1:walk-ice2"],
        "LAND_STN",
        "ROLL_STN",
        "SLIDE_STN",
        "jak1:flut-land-stone",
    ),
    s(
        ["jak1:walk-swamp1", "jak1:walk-swamp2"],
        ["jak1:walk-swamp1", "jak1:walk-swamp2"],
        "jak1:land-swamp",
        "jak1:roll-swamp",
        "jak1:slide-swamp",
        "jak1:flut-land-swamp",
    ),
    s(["WALK_WATER1", "WALK_WATER2"], ["RUN_WATER1", "RUN_WATER2"], "LAND_WTR", "ROLL_WTR", "SLIDE_WTR", "jak1:flut-land-water"),
    // Deep water: Jak 3 has no run in it; its walk.
    s(
        ["WALK_DWATER1", "WALK_DWATER2"],
        ["WALK_DWATER1", "WALK_DWATER2"],
        "LAND_DWATER",
        "ROLL_DWATER",
        "SLIDE_DWATER",
        "jak1:flut-land-water",
    ),
];

/// Each player's ground, as the world under them last said.
static FLOORS: [AtomicU8; 2] = [AtomicU8::new(0), AtomicU8::new(0)];

/// What player `player` (0 or 1) stands on now.
pub fn set_floor(player: usize, floor: Floor) {
    if let Some(slot) = FLOORS.get(player) {
        slot.store(floor as u8, Ordering::Relaxed);
    }
}

/// What player `player` stood on last.
pub fn floor(player: usize) -> Floor {
    FLOORS.get(player).map_or(Floor::Unknown, |slot| Floor::from_u8(slot.load(Ordering::Relaxed)))
}

/// The ground a Minecraft block is, by its id (`minecraft:` left off) and
/// its sound type's step event (`minecraft:block.grass.step`): the id first
/// for those vanilla sounds as something else (dirt sounds as gravel,
/// leaves and hay as grass, ice as glass), then the sound.
pub fn floor_of_block(id: &str, step: &str) -> Floor {
    let has = |part: &str| id.contains(part);
    if has("ice") && !has("dice") && !has("slice") {
        return Floor::Ice;
    }
    if id == "powder_snow" {
        return Floor::DeepSnow;
    }
    if id == "snow" || id == "snow_block" {
        return Floor::Snow;
    }
    if id == "hay_block" || id == "dried_kelp_block" || id == "target" {
        return Floor::Straw;
    }
    if id == "mud" || id == "muddy_mangrove_roots" {
        return Floor::Swamp;
    }
    if id == "grass_block" || id == "mycelium" || id.starts_with("moss_") || id.ends_with("_nylium") {
        return Floor::Grass;
    }
    if matches!(id, "dirt" | "coarse_dirt" | "rooted_dirt" | "dirt_path" | "farmland" | "podzol" | "clay" | "packed_mud" | "soul_soil")
    {
        return Floor::Dirt;
    }
    if id == "gravel" || id == "suspicious_gravel" {
        return Floor::Gravel;
    }
    if id.ends_with("sand") && !id.ends_with("sandstone") {
        return Floor::Sand;
    }
    if id.ends_with("_leaves") || has("vine") || id == "azalea" || id == "flowering_azalea" {
        return Floor::Forest;
    }
    if id.ends_with("mushroom_block") || id == "mushroom_stem" || id == "shroomlight" || id.ends_with("wart_block") {
        return Floor::Mushroom;
    }
    if id == "slime_block" || id == "honey_block" {
        return Floor::Squish;
    }
    if id.ends_with("_wool") || id.ends_with("_carpet") {
        return Floor::Neutral;
    }
    if matches!(
        id,
        "barrel" | "chest" | "trapped_chest" | "crafting_table" | "bookshelf" | "chiseled_bookshelf" | "composter" | "note_block" | "jukebox" | "fletching_table" | "cartography_table" | "smithing_table" | "loom"
    ) {
        return Floor::CrateWood;
    }
    if has("copper") && (has("grate") || has("trapdoor")) || matches!(id, "iron_bars" | "iron_trapdoor" | "chain") {
        return Floor::GrateMetal;
    }
    if has("copper") || matches!(id, "cauldron" | "water_cauldron" | "lava_cauldron" | "powder_snow_cauldron" | "hopper") {
        return Floor::SheetMetal;
    }
    if matches!(
        id,
        "iron_block" | "gold_block" | "netherite_block" | "anvil" | "chipped_anvil" | "damaged_anvil" | "lodestone" | "iron_door" | "heavy_weighted_pressure_plate" | "light_weighted_pressure_plate" | "lantern" | "soul_lantern" | "bell" | "blast_furnace"
    ) {
        return Floor::Metal;
    }
    let sound = step.strip_prefix("minecraft:").unwrap_or(step);
    let sound = sound.strip_prefix("block.").unwrap_or(sound);
    let sound = sound.strip_suffix(".step").unwrap_or(sound);
    match sound {
        s if s.contains("wood") || s == "ladder" || s == "scaffolding" || s == "bamboo" || s == "chiseled_bookshelf" => Floor::Wood,
        "grass" | "moss" | "moss_carpet" | "wet_grass" | "crop" | "sweet_berry_bush" | "lily_pad" | "big_dripleaf" | "small_dripleaf"
        | "azalea" | "flowering_azalea" | "spore_blossom" | "cave_vines" | "pink_petals" | "hanging_roots" | "glow_lichen" => Floor::Grass,
        "gravel" | "rooted_dirt" | "packed_mud" | "soul_soil" | "mud_bricks" => Floor::Dirt,
        "mud" | "muddy_mangrove_roots" | "mangrove_roots" => Floor::Swamp,
        "sand" | "soul_sand" | "suspicious_sand" => Floor::Sand,
        "snow" => Floor::Snow,
        "powder_snow" => Floor::DeepSnow,
        "wool" => Floor::Neutral,
        "slime_block" | "honey_block" | "sculk" | "sculk_sensor" | "sculk_catalyst" | "sculk_shrieker" | "sculk_vein" | "frogspawn" => {
            Floor::Squish
        }
        "fungus" | "nylium" | "wart_block" | "shroomlight" | "roots" | "nether_sprouts" | "stem" => Floor::Mushroom,
        "metal" | "anvil" | "chain" | "lantern" | "netherite_block" | "copper" | "copper_bulb" | "copper_grate" | "heavy_core" | "vault"
        | "trial_spawner" => Floor::Metal,
        "vine" | "azalea_leaves" | "cherry_leaves" => Floor::Forest,
        _ => Floor::Stone,
    }
}

/// One player's footstep clock: the animation the effects were read from
/// last and the artist frame it was on (`last-frame-group`,
/// `last-frame-num`).
#[derive(Clone, Copy, Debug, Default)]
pub struct StepClock {
    last: Option<(Anim, f32)>,
}

impl StepClock {
    pub fn reset(&mut self) {
        self.last = None;
    }
}

/// How the channel's frame moved, for which frames it passed.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Playback {
    Identity,
    /// Toward an artist frame.
    Seek(f32),
    /// Forward (or back) with a wrap.
    Loop { forward: bool },
}

/// The effects of `effects` the frames from the last call to `now` passed
/// (`effect-control-method-9` and `play-effects-from-res-lump`).
pub fn passed(clock: &mut StepClock, group: Anim, now: f32, playback: Playback, effects: &[AnimEffect], out: &mut Vec<AnimEffect>) {
    let mut play = |lo: f32, hi: f32, exact: f32| {
        out.extend(
            effects
                .iter()
                .filter(|e| (e.frame < hi && lo < e.frame) || e.frame == exact)
                .copied(),
        );
    };
    match clock.last {
        Some((last_group, last)) if last_group == group => {
            if !effects.is_empty() && now != last {
                match playback {
                    Playback::Seek(target) => {
                        if now < last {
                            if last >= target {
                                play(now, last, now);
                            }
                        } else if target >= last {
                            play(last, now, now);
                        }
                    }
                    Playback::Loop { forward: true } => {
                        if now < last {
                            play(last, 9_999_999.0, now);
                            play(-100_000_000.0, now, 9_999_999.0);
                        } else {
                            play(last, now, now);
                        }
                    }
                    Playback::Loop { forward: false } => {
                        if last < now {
                            play(now, 9_999_999.0, now);
                            play(-100_000_000.0, last, 9_999_999.0);
                        } else {
                            play(now, last, now);
                        }
                    }
                    Playback::Identity => play(now, now, now),
                }
            }
        }
        _ => play(now, now, now),
    }
    clock.last = Some((group, now));
}

/// The channel the footsteps read and how it moves: the walk cycle's walk,
/// or its run once mixed in past half; otherwise his one channel.
fn channel(jak: &Jak) -> (Anim, f32, Playback) {
    let chan = &jak.chan;
    let info = jak.anims.info(chan.anim);
    if let Some(mix) = chan.mix {
        // `target-walk-anim`: the walk on channel 0, the run on 3, both
        // on channel 0's frame and its `loop!`.
        let group = if mix.run > 0.5 { jak_core::anim::RUN } else { chan.anim };
        let run = jak.anims.info(group);
        let now = chan.frame * run.artist_step + run.artist_base;
        return (group, now, Playback::Loop { forward: true });
    }
    let now = chan.frame * info.artist_step + info.artist_base;
    let playback = match chan.func {
        NumFunc::Identity => Playback::Identity,
        NumFunc::Seek { target, .. } => {
            let frame = match target {
                Target::Max => jak.anims.max(chan.anim),
                Target::Frame(f) => f,
            };
            Playback::Seek(frame * info.artist_step + info.artist_base)
        }
        NumFunc::Loop { rate } => Playback::Loop { forward: rate >= 0.0 },
    };
    (chan.anim, now, playback)
}

/// The sounds Jak's animation fires this frame on `floor` (none while
/// `floor` is `None`: riding something), as (name, volume, pitch in
/// octaves).
pub fn step_sounds(jak: &Jak, clock: &mut StepClock, floor: Option<Floor>, out: &mut Vec<(&'static str, f32, f32)>) {
    let (group, now, playback) = channel(jak);
    let mut fired = Vec::new();
    passed(clock, group, now, playback, jak.anims.effects(group), &mut fired);
    let Some(floor) = floor else { return };
    // The Flut Flut's own steps only buzz the pad (`event-step`), in both
    // games; his swimming and his board fire none of these.
    if jak.state.is_flut() || jak.state.is_board() || jak.swimming() {
        return;
    }
    let sounds = floor.sounds();
    for effect in fired {
        let name = match effect.kind {
            EffectKind::WalkStep(side) => sounds.walk[usize::from(side.clamp(1, 2) - 1)],
            EffectKind::RunStep(side) => sounds.run[usize::from(side.clamp(1, 2) - 1)],
            EffectKind::Land => sounds.land,
            EffectKind::Roll => sounds.roll,
            EffectKind::Slide => sounds.slide,
            EffectKind::Sound(name) => name,
        };
        out.push((name, effect.volume, effect.pitch));
    }
}
