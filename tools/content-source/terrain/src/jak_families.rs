//! Families of house pieces that join by their neighbours (Adrii 24/10:
//! "familias de piezas que el sistema conecte según vecinos y orientación;
//! evita tanto los edificios enteros convertidos en una sola malla como un
//! catálogo de cientos de piezas exclusivas para cada casa").
//!
//! Each piece copies a vanilla block that already joins its neighbours, and
//! draws each of that block's parts in its own Jak shape (a model of
//! boxes, `jak_facades`' small voxels), so a wall, a window or a roof laid
//! by hand joins as the houses the world builds do:
//! - a wall (`*_pared`, `*_pared_base`) is a cobblestone wall: a pillar where
//!   it turns, ends or meets another, its top course capped where nothing
//!   stands on it (Spargus's rounded stucco, Haven's beam), its base
//!   course on a plinth;
//! - a window (`*_vano`) is a glass pane: its two halves the wall's
//!   on either side, open through (seen through, light comes in);
//! - a palisade (`*_empalizada`) is a wall too: plain stakes under more,
//!   sharpened at the top, a pole where it turns;
//! - a roof (`teja_*`) is a stair: its slope, inner and outer corners
//!   (valleys and hips) laid as Minecraft lays stairs; its ridge
//!   (`cumbrera_*`) a wall joining along the ridge;
//! - Haven's paving (`haven_adoquin`, `_capa`) full and in layers, for its
//!   worn hollows.
//!
//! Styles by land: Spargus (the Wasteland), Haven (the other lands'
//! villages), Snowy Mountain (`nieve`), Boggy Swamp (`pantano`); the
//! tiles red (Sandover's round ones), snowy, the swamp's mossy, and
//! corrugated metal.
use std::collections::HashMap;

use crate::jak_facades::{b, underside, dir_name, dir_step, leak, model, outward, ring_cells, B};
use crate::jak_items::{BlockLook, JakBlock, JakItem, JakKind};
use crate::jak_structures::Placed;

// ------------------------------------------------------------- models

/// A wall's pillar: the whole block, standing a hair proud of the arms it
/// joins (no flicker where they meet), with `extra` boxes.
fn pillar(extra: &[B]) -> Vec<B> {
    let mut boxes = vec![b([-0.05, 0.0, -0.05], [16.05, 16.05, 16.05], "s")];
    boxes.extend(extra.iter().map(|x| B { from: x.from, to: x.to, tex: x.tex, turn: x.turn, ends: x.ends, bottom:x.bottom }));
    boxes
}

// Spargus: bleached stucco, its top swelling and rounded, its base on a
// sloped plinth of stones.
fn sp_post() -> String {
    model(&pillar(&[]))
}
fn sp_arm() -> String {
    model(&[b([0.0, 0.0, 0.0], [16.0, 16.0, 8.0], "s")])
}
fn sp_arm_top() -> String {
    model(&[
        b([0.0, 0.0, 0.0], [16.0, 11.0, 8.0], "s"),
        b([-1.0, 8.0, 0.0], [17.0, 12.0, 8.0], "s"),
        b([0.0, 12.0, 0.0], [16.0, 14.0, 8.0], "s"),
        b([1.5, 14.0, 0.0], [14.5, 16.0, 8.0], "s"),
    ])
}
fn sp_base_post() -> String {
    model(&[
        b([-0.05, 0.0, -0.05], [16.05, 16.05, 16.05], "z"),
        b([-3.0, 0.0, -3.0], [19.0, 3.0, 19.0], "z"),
        b([-1.5, 3.0, -1.5], [17.5, 6.0, 17.5], "z"),
    ])
}
fn sp_base_arm() -> String {
    model(&[
        b([0.0, 0.0, 0.0], [16.0, 16.0, 8.0], "z"),
        b([-3.0, 0.0, 0.0], [19.0, 3.0, 8.0], "z"),
        b([-1.5, 3.0, 0.0], [17.5, 6.0, 8.0], "z"),
    ])
}
/// A window's half: a narrow slit through the wall by the block's middle.
fn sp_window_half() -> String {
    model(&[
        b([0.0, 0.0, 0.0], [16.0, 5.0, 8.0], "s"),
        b([0.0, 11.0, 0.0], [16.0, 16.0, 8.0], "s"),
        b([0.0, 5.0, 0.0], [16.0, 11.0, 6.0], "s"),
        b([0.0, 11.0, 6.0], [16.0, 13.0, 7.0], "s"),
    ])
}
fn sp_window_half_alt() -> String {
    model(&[
        b([0.0, 0.0, 8.0], [16.0, 5.0, 16.0], "s"),
        b([0.0, 11.0, 8.0], [16.0, 16.0, 16.0], "s"),
        b([0.0, 5.0, 10.0], [16.0, 11.0, 16.0], "s"),
        b([0.0, 11.0, 9.0], [16.0, 13.0, 10.0], "s"),
    ])
}

// Haven: plaster, timbers up its corners, its storey's beam across its top,
// its base on stones.
fn hv_post() -> String {
    model(&pillar(&[
        b([-0.6, 0.0, -0.6], [2.5, 16.0, 2.5], "w"),
        b([13.5, 0.0, -0.6], [16.6, 16.0, 2.5], "w"),
        b([-0.6, 0.0, 13.5], [2.5, 16.0, 16.6], "w"),
        b([13.5, 0.0, 13.5], [16.6, 16.0, 16.6], "w"),
    ]))
}
fn hv_arm() -> String {
    model(&[b([0.0, 0.0, 0.0], [16.0, 16.0, 8.0], "s")])
}
fn hv_arm_top() -> String {
    model(&[b([0.0, 0.0, 0.0], [16.0, 12.0, 8.0], "s"), b([-1.0, 12.0, 0.0], [17.0, 16.0, 8.0], "w")])
}
fn hv_base_post() -> String {
    model(&[b([-0.05, 0.0, -0.05], [16.05, 16.05, 16.05], "s"), b([-1.5, 0.0, -1.5], [17.5, 5.0, 17.5], "k")])
}
fn hv_base_arm() -> String {
    model(&[b([0.0, 0.0, 0.0], [16.0, 16.0, 8.0], "s"), b([-1.5, 0.0, 0.0], [17.5, 5.0, 8.0], "k"), b([-0.7, 5.0, 0.0], [16.7, 6.5, 8.0], "k")])
}
/// A window's half: open through the wall from its jamb to the middle,
/// in a wooden frame standing out of both faces, half its cross.
fn hv_window_half() -> String {
    model(&[
        b([0.0, 0.0, 0.0], [16.0, 4.0, 8.0], "s"),
        b([0.0, 12.0, 0.0], [16.0, 16.0, 8.0], "s"),
        b([0.0, 4.0, 0.0], [16.0, 12.0, 3.0], "s"),
        b([-1.2, 3.0, 0.0], [17.2, 4.5, 8.0], "w"),
        b([-0.6, 11.5, 0.0], [16.6, 12.6, 8.0], "w"),
        b([-0.6, 4.5, 3.0], [16.6, 11.5, 4.2], "w"),
        b([7.0, 7.4, 4.2], [9.0, 8.6, 8.0], "w"),
    ])
}
fn hv_window_half_alt() -> String {
    model(&[
        b([0.0, 0.0, 8.0], [16.0, 4.0, 16.0], "s"),
        b([0.0, 12.0, 8.0], [16.0, 16.0, 16.0], "s"),
        b([0.0, 4.0, 13.0], [16.0, 12.0, 16.0], "s"),
        b([-1.2, 3.0, 8.0], [17.2, 4.5, 16.0], "w"),
        b([-0.6, 11.5, 8.0], [16.6, 12.6, 16.0], "w"),
        b([-0.6, 4.5, 11.8], [16.6, 11.5, 13.0], "w"),
        b([7.0, 7.4, 8.0], [9.0, 8.6, 11.8], "w"),
    ])
}
/// A window's middle: its upright bar.
fn window_mullion() -> String {
    model(&[b([7.0, 4.5, 7.0], [9.0, 11.5, 9.0], "w")])
}
fn nothing() -> String {
    String::new()
}

// Snowy Mountain: the fort's wicker panels strapped with metal, poles at
// the turns, snow on the tops; its base of the fort's stones.
fn sn_post() -> String {
    model(&[
        b([3.5, 0.0, 3.5], [12.5, 16.05, 12.5], "l"),
        b([3.0, 10.0, 3.0], [13.0, 12.0, 13.0], "r"),
    ])
}
fn sn_arm() -> String {
    model(&[b([4.0, 0.0, 0.0], [12.0, 16.0, 8.0], "p"), b([3.5, 7.0, 0.0], [12.5, 9.0, 8.0], "t")])
}
fn sn_arm_top() -> String {
    model(&[
        b([4.0, 0.0, 0.0], [12.0, 13.0, 8.0], "p"),
        b([3.5, 7.0, 0.0], [12.5, 9.0, 8.0], "t"),
        b([3.0, 13.0, 0.0], [13.0, 15.5, 8.0], "n"),
        b([4.5, 15.5, 0.0], [11.5, 16.5, 8.0], "n"),
    ])
}
fn sn_base_post() -> String {
    model(&[b([2.0, 0.0, 2.0], [14.0, 16.05, 14.0], "k")])
}
fn sn_base_arm() -> String {
    model(&[b([2.5, 0.0, 0.0], [13.5, 16.0, 8.0], "k")])
}
fn sn_window_half() -> String {
    model(&[
        b([4.0, 0.0, 0.0], [12.0, 5.0, 8.0], "p"),
        b([4.0, 11.0, 0.0], [12.0, 16.0, 8.0], "p"),
        b([4.0, 5.0, 0.0], [12.0, 11.0, 3.0], "p"),
        b([3.5, 4.0, 0.0], [12.5, 5.0, 8.0], "t"),
        b([3.5, 11.0, 0.0], [12.5, 12.0, 8.0], "t"),
    ])
}
fn sn_window_half_alt() -> String {
    model(&[
        b([4.0, 0.0, 8.0], [12.0, 5.0, 16.0], "p"),
        b([4.0, 11.0, 8.0], [12.0, 16.0, 16.0], "p"),
        b([4.0, 5.0, 13.0], [12.0, 11.0, 16.0], "p"),
        b([3.5, 4.0, 8.0], [12.5, 5.0, 16.0], "t"),
        b([3.5, 11.0, 8.0], [12.5, 12.0, 16.0], "t"),
    ])
}

// Boggy Swamp: Boggy Billy's logs laid along the wall, bark poles at the
// turns.
fn sw_post() -> String {
    model(&[b([3.0, 0.0, 3.0], [13.0, 16.05, 13.0], "l"), b([2.5, 11.0, 2.5], [13.5, 13.0, 13.5], "r")])
}
fn sw_arm() -> String {
    let mut boxes = Vec::new();
    for i in 0..4 {
        let (y0, y1) = (i as f32 * 4.0, (i + 1) as f32 * 4.0);
        let inset = if i % 2 == 0 { 3.0 } else { 3.6 };
        boxes.push(b([inset, y0, 0.0], [16.0 - inset, y1, 8.0], "h"));
    }
    model(&boxes)
}
fn sw_window_half() -> String {
    model(&[
        b([3.0, 0.0, 0.0], [13.0, 4.0, 8.0], "h"),
        b([3.6, 4.0, 0.0], [12.4, 5.0, 8.0], "h"),
        b([3.0, 12.0, 0.0], [13.0, 16.0, 8.0], "h"),
        b([3.6, 5.0, 0.0], [12.4, 12.0, 3.0], "h"),
    ])
}
fn sw_window_half_alt() -> String {
    model(&[
        b([3.0, 0.0, 8.0], [13.0, 4.0, 16.0], "h"),
        b([3.6, 4.0, 8.0], [12.4, 5.0, 16.0], "h"),
        b([3.0, 12.0, 8.0], [13.0, 16.0, 16.0], "h"),
        b([3.6, 5.0, 13.0], [12.4, 12.0, 16.0], "h"),
    ])
}

// Palisades: the stakes along the wall (half a stake each side of the
// block's middle, so two halves meet as one), roped; sharpened where
// nothing stands on them; a pole where they turn.
fn stake_arm(points: bool) -> String {
    let mut boxes = Vec::new();
    for (z0, z1) in [(0.3f32, 5.0f32), (5.3, 8.0)] {
        let top = if points { 8.0 } else { 16.0 };
        boxes.push(b([5.0, 0.0, z0], [11.0, top, z1], "l"));
        if points {
            let tall = if z0 < 1.0 { 16.0 } else { 14.0 };
            let s = (tall - 8.0) / 8.0;
            boxes.push(b([5.8, 8.0, z0 + 0.5], [10.2, 8.0 + 3.0 * s, z1], "t"));
            boxes.push(b([6.6, 8.0 + 3.0 * s, z0 + 1.0], [9.4, 8.0 + 6.0 * s, z1 - 0.4], "t"));
            boxes.push(b([7.4, 8.0 + 6.0 * s, z0 + 1.5], [8.6, tall, z1 - 0.8], "t"));
        }
    }
    boxes.push(b([4.5, 9.0, 0.0], [11.5, 11.0, 8.0], "r"));
    model(&boxes)
}
fn stakes_arm() -> String {
    stake_arm(false)
}
fn stakes_arm_top() -> String {
    stake_arm(true)
}
fn stakes_post() -> String {
    model(&[
        b([4.0, 0.0, 4.0], [12.0, 13.0, 12.0], "l"),
        b([5.0, 13.0, 5.0], [11.0, 15.0, 11.0], "t"),
        b([6.5, 15.0, 6.5], [9.5, 16.5, 9.5], "t"),
        b([3.5, 9.0, 3.5], [12.5, 11.0, 12.5], "r"),
    ])
}

// Roofs of round tiles, as Sandover's huts' (Adrii 23/10: "las redondas
// como los tejados de jak 1"), in small voxels: a stair's slope rising to
// +x (vanilla's stair facing east), its inner corner (a valley: high to +x
// or +z) and its outer (a hip: high only where both are); each step of two
// sixteenths a course, its tiles rounded along the slope.
fn tiles(kind: u8, snow: bool) -> String {
    let level = |i: usize, k: usize| match kind {
        0 => i,
        1 => i.max(k),
        _ => i.min(k),
    };
    let mut boxes = Vec::new();
    // The body under the tiles, a run of equal courses a box.
    for k in 0..8 {
        let mut i = 0;
        while i < 8 {
            let l = level(i, k);
            let mut j = i;
            while j + 1 < 8 && level(j + 1, k) == l {
                j += 1;
            }
            let h = 2.0 * (l + 1) as f32;
            boxes.push(underside(b([2.0 * i as f32, 0.0, 2.0 * k as f32], [2.0 * (j + 1) as f32, h, 2.0 * (k + 1) as f32], "b"),"u"));
            i = j + 1;
        }
    }
    // The tiles' crowns: along the slope where it runs (x or z).
    for i in 0..8 {
        for k in 0..8 {
            let along_x = match kind {
                0 => true,
                1 => i >= k,
                _ => i <= k,
            };
            let crown = if along_x { k % 2 == 0 } else { i % 2 == 0 };
            if !crown {
                continue;
            }
            let h = 2.0 * (level(i, k) + 1) as f32;
            let (x0, z0) = (2.0 * i as f32, 2.0 * k as f32);
            // The lowest course's tiles out over the eave.
            let lip = if along_x && level(i, k) == 0 && kind == 0 { 1.5 } else { 0.0 };
            boxes.push(b([x0 - lip, h, z0 + 0.2], [x0 + 2.0, h + 1.3, z0 + 1.8], "t"));
            if snow && (i * 3 + k * 5) % 4 != 0 {
                boxes.push(b([x0, h + 1.3, z0], [x0 + 2.0, h + 1.8, z0 + 2.0], "n"));
            }
        }
    }
    model(&boxes)
}
fn tiles_straight() -> String {
    tiles(0, false)
}
fn tiles_inner() -> String {
    tiles(1, false)
}
fn tiles_outer() -> String {
    tiles(2, false)
}
fn tiles_straight_snow() -> String {
    tiles(0, true)
}
fn tiles_inner_snow() -> String {
    tiles(1, true)
}
fn tiles_outer_snow() -> String {
    tiles(2, true)
}

/// A ridge: a saddle of half a block over where two slopes meet, a row of
/// round tiles along it (its arms along the ridge; a knob where it turns or
/// ends).
fn ridge_post(snow: bool) -> String {
    let mut boxes = vec![
        b([0.05, 0.0, 0.05], [15.95, 8.0, 15.95], "b"),
        b([5.0, 8.0, 5.0], [11.0, 11.2, 11.0], "t"),
        b([6.0, 11.2, 6.0], [10.0, 12.4, 10.0], "t"),
    ];
    if snow {
        boxes.push(b([5.5, 12.4, 5.5], [10.5, 13.0, 10.5], "n"));
    }
    model(&boxes)
}
fn ridge_arm(snow: bool) -> String {
    let mut boxes = vec![
        b([0.0, 0.0, 0.0], [16.0, 7.95, 8.0], "b"),
        b([5.0, 7.95, 0.0], [11.0, 11.0, 8.0], "t"),
        b([6.0, 11.0, 0.0], [10.0, 12.3, 8.0], "t"),
    ];
    if snow {
        boxes.push(b([5.5, 12.3, 0.0], [10.5, 12.9, 8.0], "n"));
    }
    model(&boxes)
}
fn ridge_post_plain() -> String {
    ridge_post(false)
}
fn ridge_arm_plain() -> String {
    ridge_arm(false)
}
fn ridge_post_snow() -> String {
    ridge_post(true)
}
fn ridge_arm_snow() -> String {
    ridge_arm(true)
}

// ------------------------------------------------------------- blocks

type Parts = &'static [(&'static str, fn() -> String)];

/// A wall's parts (cobblestone wall's models: post, side, side tall).
const fn wall(post: fn() -> String, top: fn() -> String, arm: fn() -> String) -> [(&'static str, fn() -> String); 3] {
    [("_post", post), ("_side", top), ("_side_tall", arm)]
}

const SP_WALL: [(&str, fn() -> String); 3] = wall(sp_post, sp_arm_top, sp_arm);
const SP_BASE: [(&str, fn() -> String); 3] = wall(sp_base_post, sp_base_arm, sp_base_arm);
const HV_WALL: [(&str, fn() -> String); 3] = wall(hv_post, hv_arm_top, hv_arm);
const HV_BASE: [(&str, fn() -> String); 3] = wall(hv_base_post, hv_base_arm, hv_base_arm);
const SN_WALL: [(&str, fn() -> String); 3] = wall(sn_post, sn_arm_top, sn_arm);
const SN_BASE: [(&str, fn() -> String); 3] = wall(sn_base_post, sn_base_arm, sn_base_arm);
const SW_WALL: [(&str, fn() -> String); 3] = wall(sw_post, sw_arm, sw_arm);
const STAKES: [(&str, fn() -> String); 3] = wall(stakes_post, stakes_arm_top, stakes_arm);
const RIDGE: [(&str, fn() -> String); 3] = wall(ridge_post_plain, ridge_arm_plain, ridge_arm_plain);
const RIDGE_SNOW: [(&str, fn() -> String); 3] = wall(ridge_post_snow, ridge_arm_snow, ridge_arm_snow);

/// A window's parts (glass pane's models).
const fn window(half: fn() -> String, alt: fn() -> String) -> [(&'static str, fn() -> String); 5] {
    [("_post", window_mullion), ("_side", half), ("_side_alt", alt), ("_noside", nothing), ("_noside_alt", nothing)]
}
const SP_WINDOW: [(&str, fn() -> String); 5] = window(sp_window_half, sp_window_half_alt);
const HV_WINDOW: [(&str, fn() -> String); 5] = window(hv_window_half, hv_window_half_alt);
const SN_WINDOW: [(&str, fn() -> String); 5] = window(sn_window_half, sn_window_half_alt);
const SW_WINDOW: [(&str, fn() -> String); 5] = window(sw_window_half, sw_window_half_alt);

/// A roof's parts (oak stairs' models: straight, inner, outer).
const TILES: [(&str, fn() -> String); 3] = [("", tiles_straight), ("_inner", tiles_inner), ("_outer", tiles_outer)];
const TILES_SNOW: [(&str, fn() -> String); 3] = [("", tiles_straight_snow), ("_inner", tiles_inner_snow), ("_outer", tiles_outer_snow)];

const SP_TEX: &[(&str, &str)] = &[("s", "wascity-stucco-wall-bleached-01"), ("z", "wascitya-stone-bottom"), ("w", "wascity-wood-plain")];
const HV_TEX: &[(&str, &str)] = &[("s", "city-slumwall-06"), ("w", "city-dirtywood"), ("k", "city-slum-stonewall-bricks")];
const SN_TEX: &[(&str, &str)] = &[("p", "snow-woodpanels"), ("t", "snow-fortmetaltrim"), ("l", "snow-woodpole"), ("r", "snow-rope-snow"), ("n", "snow-03"), ("k", "snow-fortwallbase"), ("w", "snow-woodpole")];
const SW_TEX: &[(&str, &str)] = &[("h", "swp-hut-wood-01"), ("l", "swp-bark"), ("r", "swp-rope-01"), ("w", "swp-hut-wood-01")];

const fn set(path: &'static str, vanilla: &'static str, models: Parts, textures: &'static [(&'static str, &'static str)]) -> JakBlock {
    JakBlock { path, look: BlockLook::Set { vanilla, models, textures } }
}

pub const FAMILY_BLOCKS: [JakBlock; 26] = [
    set("spargus_pared", "cobblestone_wall", &SP_WALL, SP_TEX),
    set("spargus_pared_base", "cobblestone_wall", &SP_BASE, SP_TEX),
    set("spargus_vano", "glass_pane", &SP_WINDOW, SP_TEX),
    set("haven_pared", "cobblestone_wall", &HV_WALL, HV_TEX),
    set("haven_pared_base", "cobblestone_wall", &HV_BASE, HV_TEX),
    set("haven_vano", "glass_pane", &HV_WINDOW, HV_TEX),
    set("nieve_pared", "cobblestone_wall", &SN_WALL, SN_TEX),
    set("nieve_pared_base", "cobblestone_wall", &SN_BASE, SN_TEX),
    set("nieve_vano", "glass_pane", &SN_WINDOW, SN_TEX),
    set("pantano_pared", "cobblestone_wall", &SW_WALL, SW_TEX),
    set("pantano_vano", "glass_pane", &SW_WINDOW, SW_TEX),
    set("nieve_empalizada", "cobblestone_wall", &STAKES, &[("l", "snow-woodpole"), ("t", "snow-woodpole"), ("r", "snow-rope-snow")]),
    set("pantano_empalizada", "cobblestone_wall", &STAKES, &[("l", "swp-bark"), ("t", "swp-wood-spike-end"), ("r", "swp-rope-01")]),
    set("spargus_empalizada", "cobblestone_wall", &STAKES, &[("l", "wascity-wood-plain"), ("t", "wascity-metal-spike-01"), ("r", "wascity-metal-pole")]),
    set("teja_roja", "oak_stairs", &TILES, &[("t", "vil-hut-roof-tile-01"), ("b", "vil1-medres-hutroof"), ("u", "wascity-wood-plain")]),
    set("teja_nevada", "oak_stairs", &TILES_SNOW, &[("t", "vil-hut-roof-tile-01"), ("b", "vil1-medres-hutroof"), ("n", "snow-03"), ("u", "snow-wood-01")]),
    set("teja_pantano", "oak_stairs", &TILES, &[("t", "swp-roof-tile-01"), ("b", "swp-roof-tile-top"), ("u", "swp-plankwood")]),
    set("teja_chapa", "oak_stairs", &TILES, &[("t", "wascity-roof-1"), ("b", "wascity-metal-dirty"), ("u", "wascity-metal-dirty")]),
    set("cumbrera_roja", "cobblestone_wall", &RIDGE, &[("t", "vil-hut-roof-tile-01"), ("b", "vil1-medres-hutroof")]),
    set("cumbrera_nevada", "cobblestone_wall", &RIDGE_SNOW, &[("t", "vil-hut-roof-tile-01"), ("b", "vil1-medres-hutroof"), ("n", "snow-03")]),
    set("cumbrera_pantano", "cobblestone_wall", &RIDGE, &[("t", "swp-roof-tile-01"), ("b", "swp-roof-tile-top")]),
    set("cumbrera_chapa", "cobblestone_wall", &RIDGE, &[("t", "wascity-roof-1"), ("b", "wascity-metal-dirty")]),
    JakBlock { path: "haven_adoquin", look: BlockLook::All("city-farm-stone-wall-01") },
    JakBlock { path: "haven_adoquin_capa", look: BlockLook::Layers { texture: "city-farm-stone-wall-01", full: "haven_adoquin" } },
    JakBlock { path: "spargus_losa", look: BlockLook::Slab { texture: "des-bridge-plank", double: "spargus_planks" } },
    JakBlock { path: "nieve_tablas", look: BlockLook::All("snow-wood-01") },
];

pub const FAMILY_ITEMS: [JakItem; 26] = [
    JakItem { path: "spargus_pared", title: "Muro de Spargus", kind: JakKind::Block },
    JakItem { path: "spargus_pared_base", title: "Muro de Spargus con zócalo", kind: JakKind::Block },
    JakItem { path: "spargus_vano", title: "Ventana de Spargus", kind: JakKind::Block },
    JakItem { path: "haven_pared", title: "Muro de Villa Refugio", kind: JakKind::Block },
    JakItem { path: "haven_pared_base", title: "Muro de Villa Refugio con zócalo", kind: JakKind::Block },
    JakItem { path: "haven_vano", title: "Ventana de Villa Refugio", kind: JakKind::Block },
    JakItem { path: "nieve_pared", title: "Muro de paneles de la Montaña", kind: JakKind::Block },
    JakItem { path: "nieve_pared_base", title: "Muro de piedra de la Montaña", kind: JakKind::Block },
    JakItem { path: "nieve_vano", title: "Ventana de la Montaña", kind: JakKind::Block },
    JakItem { path: "pantano_pared", title: "Muro de troncos del Pantano", kind: JakKind::Block },
    JakItem { path: "pantano_vano", title: "Ventana del Pantano", kind: JakKind::Block },
    JakItem { path: "nieve_empalizada", title: "Empalizada de la Montaña", kind: JakKind::Block },
    JakItem { path: "pantano_empalizada", title: "Empalizada del Pantano", kind: JakKind::Block },
    JakItem { path: "spargus_empalizada", title: "Empalizada de Spargus", kind: JakKind::Block },
    JakItem { path: "teja_roja", title: "Tejado de tejas rojas", kind: JakKind::Block },
    JakItem { path: "teja_nevada", title: "Tejado de tejas nevadas", kind: JakKind::Block },
    JakItem { path: "teja_pantano", title: "Tejado de tejas del Pantano", kind: JakKind::Block },
    JakItem { path: "teja_chapa", title: "Tejado de chapa", kind: JakKind::Block },
    JakItem { path: "cumbrera_roja", title: "Cumbrera de tejas rojas", kind: JakKind::Block },
    JakItem { path: "cumbrera_nevada", title: "Cumbrera de tejas nevadas", kind: JakKind::Block },
    JakItem { path: "cumbrera_pantano", title: "Cumbrera del Pantano", kind: JakKind::Block },
    JakItem { path: "cumbrera_chapa", title: "Cumbrera de chapa", kind: JakKind::Block },
    JakItem { path: "haven_adoquin", title: "Adoquín de Villa Refugio", kind: JakKind::Block },
    JakItem { path: "haven_adoquin_capa", title: "Adoquín gastado de Villa Refugio", kind: JakKind::Block },
    JakItem { path: "spargus_losa", title: "Losa de tablas de Spargus", kind: JakKind::Block },
    JakItem { path: "nieve_tablas", title: "Tablas de la Montaña", kind: JakKind::Block },
];

/// The vanilla blocks each family piece behaves as (`MASHUP_BLOCKS`).
pub fn clones() -> impl Iterator<Item = (&'static str, &'static str)> {
    FAMILY_BLOCKS.iter().map(|block| {
        let vanilla = match block.look {
            BlockLook::Set { vanilla, .. } => vanilla,
            BlockLook::Layers { .. } => "snow",
            BlockLook::Slab { .. } => "smooth_stone_slab",
            _ => "stone",
        };
        (block.path, vanilla)
    })
}

/// What kind of family a block of ours is.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Kind {
    Wall,
    Window,
    Ridge,
    Tiles,
}

pub fn kind(path: &str) -> Option<Kind> {
    if path.starts_with("cumbrera_") {
        Some(Kind::Ridge)
    } else if path.starts_with("teja_") {
        Some(Kind::Tiles)
    } else if path.ends_with("_vano") || path == "muro_haven_chapa" {
        Some(Kind::Window)
    } else if path.ends_with("_pared") || path.ends_with("_pared_base") || path.ends_with("_empalizada") || path.ends_with("_baranda") || path.ends_with("_bordillo") {
        Some(Kind::Wall)
    } else {
        None
    }
}

/// A family piece's collision as it is drawn, in blocks (none: the vanilla
/// block's, no taller than a block): walls and windows the whole block
/// (their arms fill it, a pillar where they turn), a palisade its stakes'
/// row, a ridge its saddle, a layer of paving its height.
pub fn collision(path: &str, property: &dyn Fn(&str) -> Option<String>) -> Option<Vec<[f64; 6]>> {
    if path=="muro_haven_chapa"{return Some(vec![[0.,0.,0.,1.,1.,1.]]);}
    if path == "haven_estandarte" || path == "haven_mata" {
        return Some(Vec::new());
    }
    let low = if path.ends_with("_bordillo") { Some(5.0 / 16.0) } else if path.ends_with("_baranda") { Some(14.0 / 16.0) } else { None };
    if path.ends_with("_empalizada") || low.is_some() {
        let top = low.unwrap_or(1.0);
        let mut boxes = vec![];
        let on = |d: &str| property(d).is_some_and(|v| v != "none" && v != "false");
        if property("up").is_some_and(|v| v == "true") {
            boxes.push([0.25, 0.0, 0.25, 0.75, top, 0.75]);
        }
        let (a, c) = (5.0 / 16.0, 11.0 / 16.0);
        if on("north") {
            boxes.push([a, 0.0, 0.0, c, top, 0.5]);
        }
        if on("south") {
            boxes.push([a, 0.0, 0.5, c, top, 1.0]);
        }
        if on("east") {
            boxes.push([0.5, 0.0, a, 1.0, top, c]);
        }
        if on("west") {
            boxes.push([0.0, 0.0, a, 0.5, top, c]);
        }
        if boxes.is_empty() {
            boxes.push([0.25, 0.0, 0.25, 0.75, top, 0.75]);
        }
        return Some(boxes);
    }
    match kind(path) {
        Some(Kind::Wall | Kind::Window) => Some(vec![[0.0, 0.0, 0.0, 1.0, 1.0, 1.0]]),
        Some(Kind::Ridge) => Some(vec![[0.0, 0.0, 0.0, 1.0, 0.5, 1.0]]),
        _ if path.ends_with("_capa") || path.ends_with("_layer") => {
            let layers: f64 = property("layers")?.parse().ok()?;
            Some(vec![[0.0, 0.0, 0.0, 1.0, layers / 8.0, 1.0]])
        }
        _ => None,
    }
}

/// Whether a family piece is seen through (a window).
pub fn see_through(path: &str) -> bool {
    kind(path) == Some(Kind::Window)
}

// ------------------------------------------------------------- plans

/// A piece put in a plan: a family's (its states found by its neighbours,
/// as the game finds them) or a whole state.
#[derive(Clone, Copy, Debug)]
pub enum Cell {
    /// A wall, window or ridge: its block's id.
    Join(&'static str),
    /// A roof's slope: its id and the world way it rises toward.
    Tiles(&'static str, u8),
    Fixed(&'static str),
}

/// Pieces laid at blocks of the world; [`Plan::finish`] gives each family
/// piece the state its neighbours give it.
#[derive(Default)]
pub struct Plan {
    cells: HashMap<(i32, i32, i32), Cell>,
    order: Vec<(i32, i32, i32)>,
}

/// Whether a whole state stands as a solid block a wall joins and stands
/// tall under (not the air, a door, a ladder, a lantern, a bed...).
fn solid(state: &str) -> bool {
    let id = state.split('[').next().unwrap_or(state);
    !(id.ends_with("air")
        || ["door", "ladder", "lantern", "trapdoor", "_bed", "chest", "campfire", "torch", "slab", "stairs", "barrel", "bell", "_rod", "tuberia", "tubo", "codo", "bidon", "capa", "_carpet", "estandarte", "brazo", "canes", "tejadillo", "toldo", "puntal", "caja", "mata"]
            .iter()
            .any(|w| id.contains(w))
        || ["_poste", "_viga", "_pilar", "_pilar_punta", "_tronco"].iter().any(|w| id.ends_with(w)))
}

impl Plan {
    pub fn put(&mut self, at: (i32, i32, i32), cell: Cell) {
        if self.cells.insert(at, cell).is_none() {
            self.order.push(at);
        }
    }

    pub fn fixed(&mut self, at: (i32, i32, i32), state: &'static str) {
        self.put(at, Cell::Fixed(state));
    }

    fn at(&self, (x, y, z): (i32, i32, i32), way: u8) -> Option<Cell> {
        let (dx, dz) = dir_step(way);
        self.cells.get(&(x + dx, y, z + dz)).copied()
    }

    /// Whether the piece `here` (of `kind`) joins what is at `way`.
    fn joins(&self, at: (i32, i32, i32), here: Kind, way: u8) -> bool {
        match (here, self.at(at, way)) {
            (Kind::Ridge, Some(Cell::Join(id))) => kind(id.strip_prefix("jak:").unwrap_or(id)) == Some(Kind::Ridge),
            (Kind::Ridge, _) => false,
            (_, Some(Cell::Join(id))) => matches!(kind(id.strip_prefix("jak:").unwrap_or(id)), Some(Kind::Wall | Kind::Window)),
            (_, Some(Cell::Fixed(state))) => solid(state),
            _ => false,
        }
    }

    /// Each piece's state, in the order they were first put.
    pub fn finish(&self) -> Vec<Placed> {
        let mut out = Vec::with_capacity(self.order.len());
        for &at in &self.order {
            let state = match self.cells[&at] {
                Cell::Fixed(state) => state,
                Cell::Join(id) => {
                    let k = kind(id.strip_prefix("jak:").unwrap_or(id)).unwrap_or(Kind::Wall);
                    let on: Vec<bool> = (0..4).map(|w| self.joins(at, k, w)).collect();
                    if k == Kind::Window {
                        leak(format!("{id}[east={},north={},south={},waterlogged=false,west={}]", on[1], on[0], on[2], on[3]))
                    } else {
                        let above = self.cells.get(&(at.0, at.1 + 1, at.2)).copied();
                        let tall = match above {
                            Some(Cell::Join(a)) => kind(a.strip_prefix("jak:").unwrap_or(a)) != Some(Kind::Ridge) && k != Kind::Ridge,
                            Some(Cell::Fixed(s)) => solid(s) && k != Kind::Ridge,
                            _ => false,
                        };
                        let side = |w: usize| if !on[w] { "none" } else if tall { "tall" } else { "low" };
                        let straight = (on[0] && on[2] && !on[1] && !on[3]) || (on[1] && on[3] && !on[0] && !on[2]);
                        leak(format!(
                            "{id}[east={},north={},south={},up={},waterlogged=false,west={}]",
                            side(1),
                            side(0),
                            side(2),
                            !straight,
                            side(3)
                        ))
                    }
                }
                Cell::Tiles(id, facing) => leak(format!(
                    "{id}[facing={},half=bottom,shape={},waterlogged=false]",
                    dir_name(facing),
                    self.stair_shape(at, facing)
                )),
            };
            out.push((at, state));
        }
        crate::jak_lighting::illuminate(&mut out);
        crate::jak_beds::arrange(&mut out);
        out
    }

    /// Vanilla's `StairBlock.getStairsShape`: a stair turns into an outer
    /// corner toward a stair across it in front, an inner one toward a stair
    /// across it behind.
    fn stair_shape(&self, at: (i32, i32, i32), facing: u8) -> &'static str {
        let ccw = (facing + 3) % 4;
        let facing_of = |way: u8| match self.at(at, way) {
            Some(Cell::Tiles(_, f)) => Some(f),
            _ => None,
        };
        // Whether the stair beside at `way` is not one facing as this one.
        let can_take = |way: u8| facing_of(way) != Some(facing);
        if let Some(f2) = facing_of(facing) {
            if f2 % 2 != facing % 2 && can_take((f2 + 2) % 4) {
                return if f2 == ccw { "outer_left" } else { "outer_right" };
            }
        }
        if let Some(f3) = facing_of((facing + 2) % 4) {
            if f3 % 2 != facing % 2 && can_take(f3) {
                return if f3 == ccw { "inner_left" } else { "inner_right" };
            }
        }
        "straight"
    }
}

// ------------------------------------------------------------- houses

/// A building's square: its local blocks (x across from its front's left,
/// z back from its front, y up from its floor) in the world, its front
/// toward `front`.
pub(crate) struct Site {
    pub(crate) origin: (i32, i32, i32),
    pub(crate) front: u8,
    pub(crate) w: i32,
    pub(crate) d: i32,
}

impl Site {
    pub(crate) fn at(&self, x: i32, y: i32, z: i32) -> (i32, i32, i32) {
        let (fx, fz) = dir_step(self.front);
        let (rx, rz) = dir_step(self.front + 1);
        let (cx, cz) = (x - self.w / 2, z - self.d / 2);
        (self.origin.0 + rx * cx - fx * cz, self.origin.1 + y, self.origin.2 + rz * cx - fz * cz)
    }

    /// A local way (0 front, 1 right, 2 back, 3 left) in the world.
    pub(crate) fn way(&self, local: u8) -> u8 {
        (self.front + local) % 4
    }

    /// A state with its `{f}` the world way of local way `local`.
    pub(crate) fn facing(&self, name: &str, local: u8) -> &'static str {
        leak(name.replace("{f}", dir_name(self.way(local))))
    }
}

impl Plan {
    /// Insert a course in each storey. Preserve the existing kits' detail
    /// and proportions while giving the original citizens three clear
    /// blocks inside; ladders, posts, pipes and air remain continuous.
    pub(crate) fn tall_storeys(&mut self,ground:i32){
        let order=std::mem::take(&mut self.order);let cells=std::mem::take(&mut self.cells);
        for at in order {let cell=cells[&at];let rel=at.1-ground;
            if rel<0 {self.put(at,cell);continue;}
            let y=at.1+(rel+1).div_euclid(3);
            if matches!(cell,Cell::Fixed(s) if s.starts_with("minecraft:lantern") && rel%3==1){
                self.fixed((at.0,y,at.2),"minecraft:air");self.put((at.0,y+1,at.2),cell);continue;
            }
            self.put((at.0,y,at.2),cell);
            if rel%3==1 {
                let extra=match cell {
                    Cell::Join(id) if kind(id.strip_prefix("jak:").unwrap_or(id))==Some(Kind::Window)=>{
                        Cell::Join(if id=="jak:barrio_base_vano" {"jak:barrio_base2_pared"} else {leak(id.replace("_vano","_pared"))})
                    }
                    Cell::Join(_) | Cell::Tiles(_,_)=>cell,
                    Cell::Fixed(s) if s.starts_with("minecraft:air") || s.starts_with("minecraft:ladder")
                        || ["_poste","_pilar","_tubo[","_entramado","_larguero"].iter().any(|p|s.contains(p))=>cell,
                    _=>Cell::Fixed("minecraft:air"),
                };
                self.put((at.0,y+1,at.2),extra);
            }
        }
    }

    /// Every authored entrance has a threshold and headroom immediately
    /// on both sides. This also removes accidental posts in a porch's
    /// approach, without opening a second hole elsewhere in the wall.
    pub(crate) fn doorways(&mut self, floor: &'static str) {
        // Thresholds and upper entrances may share a lower portal's cells.
        // Follow authored insertion order, never randomized HashMap order.
        let mut doors: Vec<_> = self.order.iter().filter_map(|&at| {
            let Cell::Fixed(state) = self.cells.get(&at)? else { return None };
            let id = state.split('[').next()?;
            (minecraftoss_player::behavior_path(id).ends_with("_door") && state.contains("half=lower"))
                .then_some((at, *state))
        }).collect();
        // Upper thresholds are real floors and take precedence over a
        // lower storey's decorative lintel when those cells coincide.
        doors.sort_by_key(|(at,_)|at.1);
        for ((x, y, z), state) in doors {
            let way = if state.contains("facing=east") { 1 } else if state.contains("facing=south") { 2 } else if state.contains("facing=west") { 3 } else { 0 };
            let (dx, dz) = dir_step(way);
            let id=state.split('[').next().unwrap();
            if minecraftoss_core::registries::is_tall_door(id) {
                let (rx,rz)=dir_step(way+1);
                for leaf in 0..2 {
                    let (lx,lz)=(x+rx*leaf,z+rz*leaf);
                    // Porch posts two blocks out used to occupy the
                    // right leaf's approach even with both doors open.
                    for side in [-1,0,1,2] {let (px,pz)=(lx+dx*side,lz+dz*side);
                        self.fixed((px,y-1,pz),floor);
                        for h in 0..3 {self.fixed((px,y+h,pz),"minecraft:air");}
                    }
                    for (h,half) in [(0,"lower"),(1,"upper")] {
                        self.fixed((lx,y+h,lz),leak(format!("{id}[facing={},half={half},hinge={},open=false,powered=false]",dir_name(way),if leaf==0{"left"}else{"right"})));
                    }
                }
                let frame=if id.contains("haven"){"jak:barrio_portal_doble"}else{"jak:sparg_portal_doble"};
                self.fixed((x,y+3,z),leak(format!("{frame}[facing={}]",dir_name(way))));
                continue;
            }
            self.fixed((x, y - 1, z), floor);
            for side in [-1, 1] {
                let at = (x + dx * side, y, z + dz * side);
                self.fixed((at.0, y - 1, at.2), floor);
                for h in 0..2 {
                    // A ladder can share the clear interior beside an entry.
                    let p = (at.0, y + h, at.2);
                    if !matches!(self.cells.get(&p), Some(Cell::Fixed(s)) if s.starts_with("minecraft:ladder")) {
                        self.fixed(p, "minecraft:air");
                    }
                }
            }
        }
    }

    /// Air at `at` unless something is put there already.
    pub(crate) fn air_if_empty(&mut self, at: (i32, i32, i32)) {
        let empty = match self.cells.get(&at) {
            None => true,
            Some(Cell::Fixed(s)) => s.ends_with("air"),
            _ => false,
        };
        if empty {
            self.put(at, Cell::Fixed("minecraft:air"));
        }
    }
}

/// A gable roof of `tiles` over a `w` by `d` square from local `z0` to
/// `z1`, its eaves a block out at the sides, its slopes rising from the
/// front's and the back's eaves to a ridge of `ridge` along the front;
/// under its slopes at the sides (the gables) `gable`'s wall.
pub(crate) fn gable_roof(p: &mut Plan, s: &Site, (z0, z1): (i32, i32), top: i32, tiles: &'static str, ridge: &'static str, gable: &'static str) -> i32 {
    let rows = z1 - z0 + 1;
    let mid = z0 + rows / 2;
    for x in -1..s.w + 1 {
        for z in z0..=z1 {
            let rise = (z - z0).min(z1 - z);
            let y = top + rise;
            let cell = if rows % 2 == 1 && z == mid {
                Cell::Join(ridge)
            } else if z < mid || (rows % 2 == 0 && z == mid - 1) {
                Cell::Tiles(tiles, s.way(2))
            } else {
                Cell::Tiles(tiles, s.way(0))
            };
            p.put(s.at(x, y, z), cell);
            if (x == 0 || x == s.w - 1) && (z0 + 1..z1).contains(&z) {
                for under in top..y {
                    p.put(s.at(x, under, z), Cell::Join(gable));
                }
            }
        }
    }
    top + rows / 2
}

/// A house of Spargus at `origin` (its floor's middle, on the ground), its
/// front toward `front`, of Spargus's families (Adrii 24/10): its walls of
/// stucco on a stone plinth, capped where they end above, a pillar where
/// they turn; narrow windows through them; a metal door that opens under
/// a wooden arch; two tapering stones before it, a canister; pipes and
/// beams out of its walls; a ladder to a hatch in its flat roof. The bigger
/// ones taller, a hollow tower four by four at the back of the left side,
/// a doorway to it from the house, a ladder up inside to its top behind
/// spikes. Inside a bed, a chest of Jak 3's things and a lantern.
pub fn spargus_house(origin: (i32, i32, i32), front: u8, big: bool) -> Vec<Placed> {
    let (w, d, h) = if big { (9, 8, 5) } else { (7, 6, 4) };
    let s = Site { origin, front, w, d };
    let mut p = Plan::default();
    for x in if big { -6 } else { -3 }..w + 3 {
        for z in -4..d + 3 {
            for y in 0..h + 7 {
                p.fixed(s.at(x, y, z), "minecraft:air");
            }
            p.fixed(s.at(x, -1, z), "jak:spargus_ground");
            p.fixed(s.at(x, -2, z), "jak:wasteland_rock");
        }
    }
    let salt = |x: i32, y: i32, z: i32| ((x * 7 + y * 13 + z * 5 + i32::from(front)) as u32).wrapping_mul(2654435761) >> 28;
    for x in 1..w - 1 {
        for z in 1..d - 1 {
            p.fixed(s.at(x, 0, z), "jak:spargus_planks");
            p.fixed(s.at(x, h - 1, z), "jak:spargus_roof_slab[type=top,waterlogged=false]");
        }
    }
    for (x, z, way, corner) in ring_cells(0, 0, w, d) {
        for y in 0..h {
            let window = !corner
                && ((y == 1 && way != 0 && (x == w / 2 || z == d / 2))
                    || (y == h - 2 && salt(x, y, z) % 3 == 0)
                    || (y == 1 && way == 0 && (x == 1 || x == w - 2)));
            let cell = if y == 0 {
                "jak:spargus_pared_base"
            } else if window {
                "jak:spargus_vano"
            } else {
                "jak:spargus_pared"
            };
            p.put(s.at(x, y, z), Cell::Join(cell));
        }
    }
    // The door in the middle of the front, its wooden arch over it.
    for (y, half) in [(0, "lower"), (1, "upper")] {
        let door = s.facing(&format!("jak:spargus_puerta_alta[facing={{f}},half={half},hinge=left,open=false,powered=false]"), 0);
        p.fixed(s.at(w / 2 - 1, y, 0), door);
    }
    p.fixed(s.at(w / 2, 2, 0), s.facing("jak:spargus_arco[facing={f}]", 0));
    // Two tapering stones before it, a canister by them.
    for x in [w / 2 - 2, w / 2 + 2] {
        p.fixed(s.at(x, 0, -2), "jak:spargus_pilar[facing=up,powered=false,waterlogged=false]");
        p.fixed(s.at(x, 1, -2), "jak:spargus_pilar_punta[facing=up,powered=false,waterlogged=false]");
    }
    p.fixed(s.at(w / 2 + 1, 0, -1), s.facing("jak:spargus_bidon[facing={f}]", 0));
    // Pipes and beams out of the walls under their tops here and there.
    let mut k = 0;
    for x in 1..w - 1 {
        for (z, way) in [(-1, 0u8), (d, 2)] {
            if x == w / 2 || salt(x, 9, z) % 3 != 0 {
                continue;
            }
            k += 1;
            let part = if k % 2 == 0 { "jak:spargus_tuberia" } else { "jak:spargus_viga" };
            p.fixed(s.at(x, h - 2, z), s.facing(&format!("{part}[facing={{f}},powered=false,waterlogged=false]"), way));
        }
    }
    for z in 1..d - 1 {
        for (x, way) in [(-1, 3u8), (w, 1)] {
            if salt(x, 9, z) % 3 == 0 {
                p.fixed(s.at(x, h - 2, z), s.facing("jak:spargus_viga[facing={f},powered=false,waterlogged=false]", way));
            }
        }
    }
    // A ladder up the back wall inside to a hatch in the roof.
    for y in 1..h - 1 {
        p.fixed(s.at(2, y, d - 2), s.facing("minecraft:ladder[facing={f},waterlogged=false]", 0));
    }
    p.fixed(s.at(2, h - 1, d - 2), s.facing("minecraft:spruce_trapdoor[facing={f},half=top,open=false,powered=false,waterlogged=false]", 0));
    if big {
        let (tx, tz, th) = (-4, d - 4, h + 3);
        for (x, z, way, _) in ring_cells(tx, tz, 4, 4) {
            for y in 0..th {
                let cell = if y == 0 {
                    "jak:spargus_pared_base"
                } else if y == th / 2 && (x == tx || z == tz) && !(x == tx && z == tz) {
                    "jak:spargus_vano"
                } else {
                    "jak:spargus_pared"
                };
                p.put(s.at(x, y, z), Cell::Join(cell));
            }
            let _ = way;
        }
        for (x, z) in [(tx, tz), (tx + 3, tz), (tx, tz + 3), (tx + 3, tz + 3)] {
            let (way, _) = outward(x - tx, z - tz, 4, 4);
            p.fixed(s.at(x, th, z), s.facing("jak:spargus_pinchos[facing={f}]", way));
        }
        for x in tx + 1..tx + 3 {
            for z in tz + 1..tz + 3 {
                p.fixed(s.at(x, 0, z), "jak:spargus_planks");
                for y in 1..th - 1 {
                    p.fixed(s.at(x, y, z), "minecraft:air");
                }
                p.fixed(s.at(x, th - 1, z), "jak:spargus_roof_slab[type=top,waterlogged=false]");
            }
        }
        // The doorway between them.
        for y in 1..3 {
            p.fixed(s.at(-1, y, d - 2), "minecraft:air");
            p.fixed(s.at(0, y, d - 2), "minecraft:air");
        }
        for y in 1..th - 1 {
            p.fixed(s.at(tx + 1, y, tz + 2), s.facing("minecraft:ladder[facing={f},waterlogged=false]", 0));
        }
        p.fixed(s.at(tx + 1, th - 1, tz + 2), s.facing("minecraft:spruce_trapdoor[facing={f},half=top,open=false,powered=false,waterlogged=false]", 0));
        p.fixed(s.at(tx + 2, th - 2, tz + 1), "minecraft:lantern[hanging=true,waterlogged=false]");
        for x in [0, w - 1] {
            let (way, _) = outward(x, 0, w, d);
            p.fixed(s.at(x, h, 0), s.facing("jak:spargus_pinchos[facing={f}]", way));
        }
    }
    // Inside.
    for (part, z) in [("foot", d - 3), ("head", d - 2)] {
        p.fixed(s.at(w - 2, 1, z), s.facing(&format!("minecraft:red_bed[facing={{f}},occupied=false,part={part}]"), 2));
    }
    p.fixed(s.at(w - 3, 1, d - 2), s.facing("minecraft:chest[facing={f},type=single,waterlogged=false]", 0));
    p.fixed(s.at(w / 2, h - 2, d / 2), "minecraft:lantern[hanging=true,waterlogged=false]");
    for x in 1..w - 1 {
        for z in 1..d - 1 {
            for y in 1..h - 1 {
                p.air_if_empty(s.at(x, y, z));
            }
        }
    }
    p.tall_storeys(origin.1);
    p.doorways("jak:spargus_planks");
    p.finish()
}

/// A house of Haven's slums at `origin`, its front toward `front`, of
/// Haven's families after Adrii's picture (24/10: stone and plaster,
/// timbers, framed windows, pipes, chimneys, lanterns, roofs of their
/// own shape): a ground floor of plaster on a stone plinth, timbers up its
/// corners, its storey's beam where nothing stands on it; the upper floor
/// jutting a block over the street on brackets; a steep gable roof of red
/// round tiles with its ridge, eaves out all round; a stone chimney with
/// its smoke; a big pipe down a side into the wall; a lantern under the
/// jutting floor, barrels by the door; a ladder up inside. Inside a bed, a
/// chest of Jak 2's things and lanterns.
pub fn haven_house(origin: (i32, i32, i32), front: u8, wide: bool) -> Vec<Placed> {
    let (w, d) = if wide { (7, 6) } else { (5, 5) };
    let s = Site { origin, front, w, d };
    let mut p = Plan::default();
    for x in -2..w + 2 {
        for z in -3..d + 2 {
            for y in 0..14 {
                p.fixed(s.at(x, y, z), "minecraft:air");
            }
            p.fixed(s.at(x, -1, z), "jak:haven_adoquin");
        }
    }
    // A storey's ring from `z0` back, at `y0`, `rows` high.
    let storey = |p: &mut Plan, z0: i32, y0: i32, rows: i32| {
        for (x, z, way, corner) in ring_cells(0, z0, w, d - z0) {
            let along = if way % 2 == 0 { x } else { z - z0 };
            for r in 0..rows {
                let cell = if r == 0 && y0 == 0 {
                    "jak:haven_pared_base"
                } else if !corner && r < rows - 1 && along % 2 == 1 && way != 2 {
                    "jak:haven_vano"
                } else {
                    "jak:haven_pared"
                };
                p.put(s.at(x, y0 + r, z), Cell::Join(cell));
            }
        }
    };
    storey(&mut p, 0, 0, 3);
    storey(&mut p, -1, 3, 3);
    for x in 0..w {
        for z in 0..d {
            if (1..w - 1).contains(&x) && (1..d - 1).contains(&z) {
                p.fixed(s.at(x, -1, z), "minecraft:spruce_planks");
                p.fixed(s.at(x, 2, z), "minecraft:spruce_planks");
                p.fixed(s.at(x, 5, z), "minecraft:spruce_planks");
            }
        }
        // Under the jutting front; its brackets on the wall under it.
        p.fixed(s.at(x, 2, -1), "minecraft:spruce_planks");
        if (1..w - 1).contains(&x) && x != w / 2 {
            p.fixed(s.at(x, 1, 0), s.facing("jak:haven_mensula[facing={f}]", 0));
        }
    }
    for (y, half) in [(0, "lower"), (1, "upper")] {
        p.fixed(s.at(w / 2, y, 0), s.facing(&format!("jak:haven_puerta[facing={{f}},half={half},hinge=left,open=false,powered=false]"), 2));
    }
    p.fixed(s.at(w / 2 + 1, 1, -1), "minecraft:lantern[hanging=true,waterlogged=false]");
    p.fixed(s.at(w / 2 - 1, 0, -1), "minecraft:barrel[facing=up,open=false]");
    if wide {
        p.fixed(s.at(w / 2 - 2, 0, -1), "minecraft:barrel[facing=up,open=false]");
    }
    let top = 6;
    let ridge_y = gable_roof(&mut p, &s, (-2, d), top, "jak:teja_roja", "jak:cumbrera_roja", "jak:haven_pared");
    // A stone chimney up the back right, its smoke out of its top.
    for y in 3..ridge_y + 2 {
        p.fixed(s.at(w - 2, y, d - 1), "minecraft:stone_bricks");
    }
    p.fixed(s.at(w - 2, ridge_y + 2, d - 1), "minecraft:campfire[facing=north,lit=true,signal_fire=false,waterlogged=false]");
    // A big pipe down the left side, turning into its wall under the eave.
    for y in 0..top - 1 {
        p.fixed(s.at(-1, y, 2), "jak:haven_tubo[facing=up,powered=false,waterlogged=false]");
    }
    p.fixed(s.at(-1, top - 1, 2), s.facing("jak:haven_codo[facing={f}]", 3));
    for y in 0..5 {
        p.fixed(s.at(w - 2, y, 1), s.facing("minecraft:ladder[facing={f},waterlogged=false]", 2));
    }
    for (part, z) in [("foot", d - 3), ("head", d - 2)] {
        p.fixed(s.at(1, 0, z), s.facing(&format!("minecraft:red_bed[facing={{f}},occupied=false,part={part}]"), 2));
    }
    p.fixed(s.at(w - 2, 0, d - 2), s.facing("minecraft:chest[facing={f},type=single,waterlogged=false]", 0));
    p.fixed(s.at(w / 2, 1, d / 2), "minecraft:lantern[hanging=true,waterlogged=false]");
    p.fixed(s.at(w / 2, 4, d / 2), "minecraft:lantern[hanging=true,waterlogged=false]");
    for x in 1..w - 1 {
        for z in 1..d - 1 {
            for y in [0, 1, 3, 4] {
                p.air_if_empty(s.at(x, y, z));
            }
        }
    }
    p.doorways("jak:haven_suelo_madera");
    p.finish()
}

/// A lurkers' fort of Snowy Mountain at `origin` (its yard's middle, on
/// the levelled ground), its gate toward `front`, of the mountain's
/// families: a palisade of sharpened stakes roped together; the gate
/// between two towers of wicker panels on the fort's stones under caps of
/// snow; a watchtower on poles with a ladder; a cabin of panels on stones
/// under a roof of snowy round tiles. Inside the cabin a bed, a chest of
/// Jak 1's things and a lantern.
pub fn snow_fort(origin: (i32, i32, i32), front: u8) -> Vec<Placed> {
    let (w, d) = (17, 17);
    let s = Site { origin, front, w, d };
    let mut p = Plan::default();
    let gate = [w / 2, w / 2 + 1];
    for (x, z, _, _) in ring_cells(0, 0, w, d) {
        if z == 0 && (w / 2 - 2..=w / 2 + 3).contains(&x) {
            continue;
        }
        for y in 0..2 {
            p.put(s.at(x, y, z), Cell::Join("jak:nieve_empalizada"));
        }
    }
    // The gate's towers, two by two, four high, under snow.
    for x0 in [w / 2 - 2, w / 2 + 2] {
        for (x, z, _, _) in ring_cells(x0, 0, 2, 2) {
            for y in 0..4 {
                p.put(s.at(x, y, z), Cell::Join(if y == 0 { "jak:nieve_pared_base" } else { "jak:nieve_pared" }));
            }
            p.fixed(s.at(x, 4, z), "jak:nieve_capa[type=bottom,waterlogged=false]");
        }
        p.fixed(s.at(x0, 5, 0), "minecraft:lantern[hanging=false,waterlogged=false]");
    }
    for (x, hinge) in [(gate[0], "right"), (gate[1], "left")] {
        for (y, half) in [(0, "lower"), (1, "upper")] {
            p.fixed(s.at(x, y, 0), s.facing(&format!("jak:snow_gate_door[facing={{f}},half={half},hinge={hinge},open=false,powered=false]"), 2));
        }
        p.fixed(s.at(x, 3, 0), if front % 2 == 0 { "jak:snow_pole[axis=x]" } else { "jak:snow_pole[axis=z]" });
    }
    // The watchtower at the back left: four roped poles, a floor of planks
    // four up, its railing of panels, a roof of snow on the poles; its
    // ladder up the front on a column of panels.
    let (tx, tz) = (2, d - 5);
    for (x, z) in [(tx, tz), (tx + 2, tz), (tx, tz + 2), (tx + 2, tz + 2)] {
        for y in 0..7 {
            p.fixed(s.at(x, y, z), "jak:nieve_tronco[facing=up,powered=false,waterlogged=false]");
        }
    }
    for x in tx..tx + 3 {
        for z in tz..tz + 3 {
            p.fixed(s.at(x, 4, z), "jak:nieve_tablas");
            p.fixed(s.at(x, 7, z), "jak:nieve_capa[type=bottom,waterlogged=false]");
        }
    }
    for (x, z, _, corner) in ring_cells(tx, tz, 3, 3) {
        if !corner && !(x == tx + 1 && z == tz) {
            p.put(s.at(x, 5, z), Cell::Join("jak:nieve_pared"));
        }
    }
    for y in 0..5 {
        p.put(s.at(tx + 1, y, tz + 1), Cell::Join("jak:nieve_pared"));
        p.fixed(s.at(tx + 1, y, tz), s.facing("minecraft:ladder[facing={f},waterlogged=false]", 0));
    }
    // The cabin at the back right, its door to the gate.
    let (cx, cz, cw) = (w - 7, d - 7, 5);
    let cabin = Site { origin: s.at(cx + cw / 2, 0, cz + cw / 2), front, w: cw, d: cw };
    for (x, z, way, corner) in ring_cells(0, 0, cw, cw) {
        for y in 0..3 {
            let cell = if y == 0 {
                "jak:nieve_pared_base"
            } else if y == 1 && !corner && way % 2 == 1 && z == cw / 2 {
                "jak:nieve_vano"
            } else {
                "jak:nieve_pared"
            };
            p.put(cabin.at(x, y, z), Cell::Join(cell));
        }
    }
    for x in 1..cw - 1 {
        for z in 1..cw - 1 {
            p.fixed(cabin.at(x, -1, z), "jak:nieve_tablas");
            for y in 0..3 {
                p.fixed(cabin.at(x, y, z), "minecraft:air");
            }
        }
    }
    gable_roof(&mut p, &cabin, (-1, cw), 3, "jak:teja_nevada", "jak:cumbrera_nevada", "jak:nieve_pared");
    for (y, half) in [(0, "lower"), (1, "upper")] {
        p.fixed(cabin.at(cw / 2, y, 0), cabin.facing(&format!("jak:snow_gate_door[facing={{f}},half={half},hinge=left,open=false,powered=false]"), 2));
    }
    for (part, z) in [("foot", cw - 3), ("head", cw - 2)] {
        p.fixed(cabin.at(1, 0, z), cabin.facing(&format!("minecraft:blue_bed[facing={{f}},occupied=false,part={part}]"), 2));
    }
    p.fixed(cabin.at(cw - 2, 0, cw - 2), cabin.facing("minecraft:chest[facing={f},type=single,waterlogged=false]", 0));
    p.fixed(cabin.at(cw / 2, 2, cw / 2), "minecraft:lantern[hanging=true,waterlogged=false]");
    p.doorways("jak:nieve_tablas");
    p.finish()
}

/// Boggy Billy's hut at `origin` (its floor's middle, three over the
/// swamp), its door toward `front`, of the swamp's families: on roped
/// poles of bark over the mud, its deck and porch of planks, walls of logs
/// with windows through them, a steep roof of the swamp's round tiles with
/// its ridge, a ladder up to the porch, a palisade of sharpened stakes
/// round its foot. `ground` is the ground's top at a column of the world.
/// Inside a bed, a chest of Jak 1's things and a lantern.
pub fn swamp_hut(origin: (i32, i32, i32), front: u8, ground: &dyn Fn(i32, i32) -> i32) -> Vec<Placed> {
    let (w, d) = (5, 5);
    let s = Site { origin, front, w, d };
    let mut p = Plan::default();
    for x in -2..w + 2 {
        for z in -3..d + 2 {
            for y in -1..8 {
                p.fixed(s.at(x, y, z), "minecraft:air");
            }
        }
    }
    for (x, z) in [(0, 0), (w - 1, 0), (0, d - 1), (w - 1, d - 1), (0, -2), (w - 1, -2), (w / 2, d - 1)] {
        let (px, _, pz) = s.at(x, 0, z);
        for y in ground(px, pz)..origin.1 - 1 {
            p.fixed((px, y, pz), "jak:pantano_poste[facing=up,powered=false,waterlogged=false]");
        }
    }
    for x in 0..w {
        for z in -2..d {
            p.fixed(s.at(x, -1, z), "jak:swamp_planks");
        }
    }
    for (x, z, way, corner) in ring_cells(0, 0, w, d) {
        for y in 0..3 {
            let window = !corner && y == 1 && way != 0 && (x == w / 2 || z == d / 2);
            p.put(s.at(x, y, z), Cell::Join(if window { "jak:pantano_vano" } else { "jak:pantano_pared" }));
        }
    }
    for (y, half) in [(0, "lower"), (1, "upper")] {
        p.fixed(s.at(w / 2, y, 0), s.facing(&format!("jak:swamp_hut_door[facing={{f}},half={half},hinge=left,open=false,powered=false]"), 2));
    }
    gable_roof(&mut p, &s, (-1, d), 3, "jak:teja_pantano", "jak:cumbrera_pantano", "jak:pantano_pared");
    // The ladder up to the porch, on a log down to the ground.
    let (lx, _, lz) = s.at(w / 2, 0, -3);
    let (sx, _, sz) = s.at(w / 2, 0, -2);
    let ladder = s.facing("minecraft:ladder[facing={f},waterlogged=false]", 0);
    for y in ground(lx, lz)..origin.1 - 1 {
        p.fixed((sx, y, sz), "jak:swamp_hut_log[axis=y]");
        p.fixed((lx, y, lz), ladder);
    }
    p.fixed((sx, origin.1 - 1, sz), "jak:swamp_planks");
    // A palisade round its foot, open before the ladder.
    for (x, z, _, _) in ring_cells(-2, -4, w + 4, d + 6) {
        if z == -4 && (w / 2 - 1..=w / 2 + 1).contains(&x) {
            continue;
        }
        let (px, _, pz) = s.at(x, 0, z);
        let y = ground(px, pz);
        p.put((px, y, pz), Cell::Join("jak:pantano_empalizada"));
        p.put((px, y + 1, pz), Cell::Join("jak:pantano_empalizada"));
    }
    for (part, z) in [("foot", d - 3), ("head", d - 2)] {
        p.fixed(s.at(1, 0, z), s.facing(&format!("minecraft:brown_bed[facing={{f}},occupied=false,part={part}]"), 2));
    }
    p.fixed(s.at(w - 2, 0, d - 2), s.facing("minecraft:chest[facing={f},type=single,waterlogged=false]", 0));
    p.fixed(s.at(w / 2, 2, d / 2), "minecraft:lantern[hanging=true,waterlogged=false]");
    p.doorways("jak:swamp_planks");
    p.finish()
}

/// Haven's paving at `(x, z)` (a village's plaza and streets), its worn
/// hollows kept (Adrii 24/10: "las depresiones desgastadas del pavimento de
/// Haven... con desniveles bajos y transiciones creíbles como en el
/// desierto"): the top of the paving's column, the block at `y` (the
/// ground's top) a full flagstone or a layer of one, by a smooth noise of
/// the world's columns (so it runs on across chunks): most of it whole,
/// its hollows sinking by eighths to half a block at their bottoms.
pub fn haven_paving(seed: i64, x: i32, z: i32) -> &'static str {
    let depth = hollow(seed, x, z);
    match depth {
        0 => "jak:haven_adoquin",
        d => leak(format!("jak:haven_adoquin_capa[layers={}]", 8 - d)),
    }
}

/// How many eighths a hollow sinks at `(x, z)` (0 to 4): a sum of two soft
/// value noises over cells of 7 and 4 blocks, only its highest values
/// hollowed, so hollows are patches with sloping rims.
fn hollow(seed: i64, x: i32, z: i32) -> u8 {
    let value = |cell: i32, salt: u64| -> f32 {
        let lattice = |i: i32, k: i32| {
            let mut h = (seed as u64) ^ salt.wrapping_mul(0x9E37_79B9_7F4A_7C15) ^ ((i as u32 as u64) << 32 | k as u32 as u64);
            h ^= h >> 33;
            h = h.wrapping_mul(0xff51_afd7_ed55_8ccd);
            h ^= h >> 33;
            (h % 1000) as f32 / 1000.0
        };
        let (fx, fz) = (x.div_euclid(cell), z.div_euclid(cell));
        let (tx, tz) = (x.rem_euclid(cell) as f32 / cell as f32, z.rem_euclid(cell) as f32 / cell as f32);
        let smooth = |t: f32| t * t * (3.0 - 2.0 * t);
        let (sx, sz) = (smooth(tx), smooth(tz));
        let a = lattice(fx, fz) + (lattice(fx + 1, fz) - lattice(fx, fz)) * sx;
        let b = lattice(fx, fz + 1) + (lattice(fx + 1, fz + 1) - lattice(fx, fz + 1)) * sx;
        a + (b - a) * sz
    };
    let n = 0.65 * value(7, 41) + 0.35 * value(4, 42);
    ((n - 0.6) / 0.07).clamp(0.0, 4.0) as u8
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entrances(blocks: Vec<Placed>) {
        let cells: std::collections::HashMap<_, _> = blocks.into_iter().collect();
        let mut count = 0;
        for (&(x, y, z), &state) in &cells {
            if !minecraftoss_player::behavior_path(state.split('[').next().unwrap()).ends_with("_door") || !state.contains("half=lower") { continue; }
            count += 1;
            assert!(cells.get(&(x, y + 1, z)).is_some_and(|s| s.contains("half=upper")), "door upper {x} {y} {z}");
            let way = if state.contains("facing=east") { 1 } else if state.contains("facing=south") { 2 } else if state.contains("facing=west") { 3 } else { 0 };
            let (dx, dz) = dir_step(way);
            for step in -1..=1 {
                let at = (x + step * dx, y, z + step * dz);
                assert!(cells.get(&(at.0, y - 1, at.2)).is_some_and(|s| !s.ends_with(":air")), "missing threshold {at:?}");
                if step != 0 {
                    for h in 0..2 {
                        let s = cells.get(&(at.0, y + h, at.2)).copied().unwrap_or("minecraft:air");
                        assert!(s.ends_with(":air") || s.starts_with("minecraft:ladder"), "blocked entry {at:?}: {s}");
                    }
                }
            }
        }
        assert!(count > 0, "every building needs an entrance");
    }

    #[test]
    fn every_size_and_rotation_has_a_complete_clear_entrance() {
        use crate::jak_barrio as b;
        use crate::jak_spargus as s;
        for front in 0..4 {
            for spec in [b::BUNKER, b::HOUSE_OCHRE, b::HOUSE_BLUE, b::TOWER, b::ROW, b::PORCH, b::BIG_BLUE, b::TERRACE, b::WORKSHOP, b::GALLERY, b::STEPPED] {
                entrances(b::building((0, 70, 0), front, spec));
            }
            for n in 0..24 {for w in [5,8,11] {for d in [6,8,10] {entrances(b::building((0,70,0),front,b::archetype(n,w,d)));}}}
            for n in 0..16 {for w in [5,8,11] {for d in [6,8,10] {entrances(s::building((0,70,0),front,s::archetype(n,w,d)));}}}
            for spec in [s::HOUSE, s::SAND_HOUSE, s::BUNKER] { entrances(s::building((0, 70, 0), front, spec)); }
            entrances(s::tower((0, 70, 0), front, 3));
            for kind in 0..3 { entrances(crate::jak_haven::building((0, 70, 0), front, kind)); }
            for big in [false, true] {
                entrances(spargus_house((0, 70, 0), front, big));
                entrances(haven_house((0, 70, 0), front, big));
            }
            entrances(snow_fort((0, 70, 0), front));
            entrances(swamp_hut((0, 70, 0), front, &|_, _| 70));
            for n in 0..80u64 {
                let spec = b::random_spec(n.wrapping_mul(0x9E3779B97F4A7C15), 5 + (n % 8) as i32, 6 + (n % 4) as i32);
                entrances(b::building((0, 70, 0), front, spec));
            }
        }
    }

    #[test]
    fn neighbours_do_not_seal_town_entrances() {
        struct Flat;
        impl crate::jak_structures::Land for Flat {
            fn ground(&self, _: i32, _: i32) -> i32 { 70 }
            fn wet(&self, _: i32, _: i32) -> bool { false }
        }
        for turn in 0..4 {
            entrances(crate::jak_barrio::village(3, (0, 0), 70, turn, &Flat));
            entrances(crate::jak_spargus::town(3, (0, 0), 70, turn, &Flat));
        }
    }

    /// Every part of every family is a model of valid boxes.
    #[test]
    fn every_part_is_a_model() {
        for block in &FAMILY_BLOCKS {
            if let BlockLook::Set { models, .. } = block.look {
                for (part, model) in models {
                    let json = format!("[{}]", model());
                    serde_json::from_str::<serde_json::Value>(&json).unwrap_or_else(|e| panic!("{}{part}: {e}", block.path));
                }
            }
        }
    }

    /// A square hipped roof's tiles turn at its corners as Minecraft turns
    /// stairs (outer corners), an L's inner corner as a valley.
    #[test]
    fn roofs_turn_their_corners() {
        let mut p = Plan::default();
        // A ring of stairs rising inward: north edge faces south, etc.
        for x in 0..4 {
            p.put((x, 0, 0), Cell::Tiles("jak:teja_roja", 2));
            p.put((x, 0, 3), Cell::Tiles("jak:teja_roja", 0));
        }
        for z in 1..3 {
            p.put((0, 0, z), Cell::Tiles("jak:teja_roja", 1));
            p.put((3, 0, z), Cell::Tiles("jak:teja_roja", 3));
        }
        let out: HashMap<_, _> = p.finish().into_iter().collect();
        for corner in [(0, 0, 0), (3, 0, 0), (0, 0, 3), (3, 0, 3)] {
            assert!(out[&corner].contains("shape=outer"), "{corner:?}: {}", out[&corner]);
        }
        assert!(out[&(1, 0, 0)].contains("shape=straight"));
        // A valley: two runs meeting inside an L.
        let mut p = Plan::default();
        for x in 0..3 {
            p.put((x, 0, 2), Cell::Tiles("jak:teja_roja", 0));
        }
        for z in 0..2 {
            p.put((2, 0, z), Cell::Tiles("jak:teja_roja", 3));
        }
        let out: HashMap<_, _> = p.finish().into_iter().collect();
        assert!(out[&(2, 0, 2)].contains("shape=inner") || out[&(2, 0, 2)].contains("shape=outer"), "{}", out[&(2, 0, 2)]);
    }

    /// A ridge joins along itself, not to the walls under it.
    #[test]
    fn ridges_join_along() {
        let mut p = Plan::default();
        for x in 0..3 {
            p.put((x, 5, 0), Cell::Join("jak:cumbrera_roja"));
        }
        p.put((1, 5, 1), Cell::Join("jak:haven_pared"));
        let out: HashMap<_, _> = p.finish().into_iter().collect();
        assert!(out[&(1, 5, 0)].contains("east=low") && out[&(1, 5, 0)].contains("west=low"), "{}", out[&(1, 5, 0)]);
        assert!(out[&(1, 5, 0)].contains("south=none") && out[&(1, 5, 0)].contains("up=false"), "{}", out[&(1, 5, 0)]);
    }

    /// Haven's paving's hollows sink by eighths: neighbouring columns never
    /// more than two eighths apart (no cliffs; the same on either side of a
    /// chunk's edge, being the world's columns'), and some of it hollowed.
    #[test]
    fn hollows_slope_gently() {
        let mut hollowed = 0;
        for x in -40..40 {
            for z in -40..40 {
                let here = i32::from(hollow(0, x, z));
                hollowed += i32::from(here > 0);
                for (dx, dz) in [(1, 0), (0, 1)] {
                    let there = i32::from(hollow(0, x + dx, z + dz));
                    assert!((here - there).abs() <= 2, "({x}, {z}) {here} by {there}");
                }
            }
        }
        assert!(hollowed > 100 && hollowed < 3200, "{hollowed}");
    }
}

#[cfg(test)]mod doorway_order_regression {
 use super::*;
 #[test]fn neighbouring_upper_and_lower_entrances_are_deterministic(){
  let mut expected=None;
  for _ in 0..32{
   let mut p=Plan::default();
   for y in (0..8).rev(){for x in -2..5{for z in -2..3{p.fixed((x,y,z),"minecraft:air");}}}
   p.fixed((0,1,0),"jak:haven_puerta_alta[facing=north,half=lower,hinge=left,open=false,powered=false]");
   p.fixed((0,5,0),"jak:haven_puerta_alta[facing=north,half=lower,hinge=left,open=false,powered=false]");
   p.doorways("jak:barrio_suelo");
   let cells:std::collections::BTreeMap<_,_>=p.finish().into_iter().collect();
   assert_eq!(cells[&(0,4,0)],"jak:barrio_suelo","the upper threshold must be a usable floor");
   if let Some(expected)=&expected{assert_eq!(&cells,expected);}else{expected=Some(cells);}
  }
 }
}
