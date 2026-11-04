//! An original village of Jak II's slums (Adrii 24/10: "un poblado original
//! de estilo Jak II... toma como referencia visual directa los modelos y
//! escenarios de Jak II"; not Haven City nor Jak 1's Sandover).
//!
//! Drawn from Jak II's own slums (`ctysluma`, rendered from its level):
//! - ground floors of grey plaster (`city-slumbase-wall`) battered, wider at
//!   their foot; small pale stones jutting in a row along their top; deep
//!   niches round doors and windows;
//! - upper storeys of ochre, blue or grey plaster (`city-slumwall-06`,
//!   `-07`, `city-wall-plain-1`) with small windows in chunky dark wooden
//!   frames, round beam ends jutting under the roof;
//! - roofs edged with a crown of dark planks (`city-dirtywood`) laid round
//!   the edge, ragged, some leaning out and up; wooden A-frames on some;
//! - lattices of dark wooden posts and rails over some facades; big copper
//!   pipes (`city-slum-bigpipe-04`) down the walls into round grated drains;
//!   lean-to canopies of planks on posts along the streets; boards hung
//!   with the slums' signs;
//! - flagstones (`slum-ground-01`) with sunken ditches of earth inside
//!   rounded rims; tufts of dry grass.
//!
//! Every building is whole: its sides and back as its front, its roof, its
//! inside (floors, a ladder, a hatch to the roof). They are laid of grid
//! pieces each reusable by hand, which join their neighbours
//! (`jak_families`); their plaster and paving laid over the world four
//! blocks a tile (`mesh::world_texture`), so no block's edge shows in them.
use crate::jak_facades::{b, model, ring_cells, B};
use crate::jak_families::{Cell, Plan, Site};
use crate::jak_items::{BlockLook, JakBlock, JakItem, JakKind};
use crate::jak_structures::{Land, Placed};

// ------------------------------------------------------------- models

/// A box turned about an axis through `origin`.
pub(crate) fn turned(from: [f32; 3], to: [f32; 3], tex: &'static str, axis: char, angle: f32, origin: [f32; 3]) -> B {
    B { from, to, tex, turn: Some((axis, angle, origin)), ends: None, bottom:None }
}

/// A battered course of the ground floor (`k` 0 at the foot to 2 at its
/// top): its faces stand out `3 - k` sixteenths at its foot, `2 - k` at its
/// top, in four steps (the slope continues block over block). `post` its
/// pillar (battered all round), else its arm (battered on its two faces).
fn battered(k: u8, post: bool) -> Vec<B> {
    let mut boxes = Vec::new();
    for i in 0..4 {
        let o = (3 - k) as f32 - (i as f32 + 0.5) / 4.0;
        let (y0, y1) = (i as f32 * 4.0, i as f32 * 4.0 + 4.0);
        if post {
            boxes.push(b([-o, y0, -o], [16.0 + o, y1 + 0.02, 16.0 + o], "g"));
        } else {
            boxes.push(b([-o, y0, 0.0], [16.0 + o, y1, 8.0], "g"));
        }
    }
    boxes
}

/// The pale stones jutting in a row along the ground floor's top.
fn teeth(post: bool) -> Vec<B> {
    let mut boxes = Vec::new();
    let along: &[f32] = if post { &[3.0, 11.0] } else { &[2.5] };
    for &z in along {
        boxes.push(b([-2.6, 11.5, z], [0.0, 14.5, z + 3.0], "t"));
        boxes.push(b([16.0, 11.5, z], [18.6, 14.5, z + 3.0], "t"));
        if post {
            boxes.push(b([z, 11.5, -2.6], [z + 3.0, 14.5, 0.0], "t"));
            boxes.push(b([z, 11.5, 16.0], [z + 3.0, 14.5, 18.6], "t"));
        }
    }
    boxes
}

fn base_post(k: u8) -> String {
    let mut boxes = battered(k, true);
    if k == 2 {
        boxes.extend(teeth(true));
    }
    model(&boxes)
}
fn base_arm(k: u8) -> String {
    let mut boxes = battered(k, false);
    if k == 2 {
        boxes.extend(teeth(false));
    }
    model(&boxes)
}
fn base1_post() -> String {
    base_post(0)
}
fn base1_arm() -> String {
    base_arm(0)
}
fn base2_post() -> String {
    base_post(1)
}
fn base2_arm() -> String {
    base_arm(1)
}
fn base3_post() -> String {
    base_post(2)
}
fn base3_arm() -> String {
    base_arm(2)
}

/// An upper storey's plaster: a pillar, an arm.
fn plain_post() -> String {
    model(&[b([-0.02, 0.0, -0.02], [16.02, 16.02, 16.02], "g")])
}
fn plain_arm() -> String {
    model(&[b([0.0, 0.0, 0.0], [16.0, 16.0, 8.0], "g")])
}

/// A window's half (glass pane's: the wall along z, its faces Â±x): its
/// opening from the middle out to `5` sixteenths, from 5 up to 11, splayed
/// wider at the faces (`splay`: the ground floor's deep niches), a chunky
/// dark wooden frame half sunk in the wall, its sill board out of both
/// faces.
fn window_half(alt: bool, splay: bool) -> String {
    let z = |a: f32, c: f32| if alt { (16.0 - c, 16.0 - a) } else { (a, c) };
    let bx = |x0: f32, y0: f32, za: f32, x1: f32, y1: f32, zc: f32, t: &'static str| {
        let (z0, z1) = z(za, zc);
        b([x0, y0, z0], [x1, y1, z1], t)
    };
    let mut boxes = vec![bx(0.0, 0.0, 0.0, 16.0, 5.0, 8.0, "g"), bx(0.0, 11.0, 0.0, 16.0, 16.0, 8.0, "g")];
    if splay {
        // The jamb thinner toward each face: the niche widens outward.
        boxes.push(bx(5.0, 5.0, 0.0, 11.0, 11.0, 5.0, "g"));
        boxes.push(bx(2.5, 5.0, 0.0, 5.0, 11.0, 3.5, "g"));
        boxes.push(bx(11.0, 5.0, 0.0, 13.5, 11.0, 3.5, "g"));
        boxes.push(bx(0.0, 5.0, 0.0, 2.5, 11.0, 2.0, "g"));
        boxes.push(bx(13.5, 5.0, 0.0, 16.0, 11.0, 2.0, "g"));
    } else {
        boxes.push(bx(0.0, 5.0, 0.0, 16.0, 11.0, 5.0, "g"));
    }
    // The frame: its jamb, head and sill, dark wood, at the wall's middle.
    boxes.push(bx(5.5, 4.0, 4.0, 10.5, 12.0, 5.6, "d"));
    boxes.push(bx(5.5, 10.4, 5.6, 10.5, 12.0, 8.0, "d"));
    boxes.push(bx(5.5, 4.0, 5.6, 10.5, 5.6, 8.0, "d"));
    // The sill board out of both faces.
    boxes.push(bx(-1.5, 4.0, 3.0, 17.5, 5.0, 8.0, "d"));
    model(&boxes)
}
fn base_window_half() -> String {
    window_half(false, true)
}
fn base_window_half_alt() -> String {
    window_half(true, true)
}
fn upper_window_half() -> String {
    window_half(false, false)
}
fn upper_window_half_alt() -> String {
    window_half(true, false)
}
/// A window's middle: the frame's upright bar.
fn window_bar() -> String {
    model(&[b([7.2, 5.6, 7.2], [8.8, 10.4, 8.8], "d")])
}
fn nothing() -> String {
    String::new()
}

/// The crown of dark planks round a roof's edge: a plank laid along the
/// edge over the wall, and planks across it, out past both faces, ragged,
/// one leaning out and up.
fn crown_arm() -> String {
    model(&[b([0.,0.,0.],[16.,4.,8.],"d"),
        turned([1.,4.,1.],[14.,6.,4.],"d",'z',9.,[8.,5.,2.5])])
}
fn crown_post() -> String {
    model(&[b([0.,0.,0.],[16.,4.,16.],"d"),
        turned([1.,4.,6.],[15.,6.,10.],"d",'z',7.,[8.,5.,8.])])
}

/// A storey of planks (the wooden top storeys): dark boards laid along the
/// wall, overlapping, a post where it turns.
fn planks_arm() -> String {
    let mut boxes = vec![b([1.5, 0.0, 0.0], [14.5, 16.0, 8.0], "d")];
    for i in 0..4 {
        let y = i as f32 * 4.0;
        let out = if i % 2 == 0 { 0.0 } else { 0.6 };
        boxes.push(b([0.0 + out, y, 0.0], [1.5, y + 4.4, 8.0], "w"));
        boxes.push(b([14.5, y, 0.0], [16.0 - out, y + 4.4, 8.0], "w"));
    }
    model(&boxes)
}
fn planks_post() -> String {
    model(&[b([0.0, 0.0, 0.0], [16.0, 16.02, 16.0], "d"), b([-1.0, 0.0, -1.0], [3.5, 16.0, 3.5], "w"), b([12.5, 0.0, 12.5], [17.0, 16.0, 17.0], "w"), b([-1.0, 0.0, 12.5], [3.5, 16.0, 17.0], "w"), b([12.5, 0.0, -1.0], [17.0, 16.0, 3.5], "w")])
}

// Pieces turned to face out (front -z), standing against the wall behind
// them (+z) or in it.

/// Above a door, in the wall: the wall's plaster, and the door's deep
/// niche's frame down both its sides, standing out of the wall and wider
/// at the foot, a dark beam over it.
fn portal() -> String {
    let mut boxes = vec![b([0.0, 0.0, 0.0], [16.0, 16.0, 16.0], "g")];
    for i in 0..8 {
        let y0 = -32.0 + i as f32 * 4.0;
        let out = 4.0 - i as f32 * 0.35;
        boxes.push(b([-4.5, y0, -out], [0.0, y0 + 4.0, 0.5], "g"));
        boxes.push(b([16.0, y0, -out], [20.5, y0 + 4.0, 0.5], "g"));
    }
    boxes.push(b([-4.5, 0.0, -2.5], [20.5, 4.0, 0.5], "g"));
    boxes.push(b([-1.0, -1.5, -3.5], [17.0, 2.0, 1.0], "d"));
    model(&boxes)
}

fn double_portal() -> String {
    let mut boxes=vec![b([0.0,0.0,0.0],[16.0,16.0,16.0],"g")];
    for i in 0..12 {let y=-48.0+i as f32*4.0;let out=4.0-i as f32*0.22;
        boxes.push(b([-4.5,y,-out],[0.0,y+4.0,0.5],"g"));
        boxes.push(b([32.0,y,-out],[36.5,y+4.0,0.5],"g"));
    }
    boxes.push(b([-4.5,0.0,-2.5],[36.5,4.0,0.5],"g"));
    boxes.push(b([-1.0,-1.5,-3.5],[33.0,2.0,1.0],"w"));model(&boxes)
}
/// Round beam ends jutting from the wall behind.
fn beam_ends() -> String {
    let mut boxes = Vec::new();
    for x in [3.5f32, 11.5] {
        boxes.push(b([x - 2.0, 9.0, 8.0], [x + 2.0, 13.0, 16.0], "w"));
        boxes.push(b([x - 1.4, 8.4, 8.0], [x + 1.4, 13.6, 16.0], "w"));
        boxes.push(b([x - 2.6, 9.6, 8.0], [x + 2.6, 12.4, 16.0], "w"));
    }
    model(&boxes)
}
/// A lattice's post against the wall, its rail along the wall at the
/// storey's floor, a collar where they meet.
fn lattice_post() -> String {
    model(&[
        b([6.0, 0.0, 12.0], [10.0, 16.0, 16.0], "w"),
        b([5.4, 0.0, 11.4], [10.6, 16.0, 12.0], "w"),
        b([0.0, 12.0, 12.6], [16.0, 15.0, 16.0], "w"),
        b([5.0, 11.0, 11.0], [11.0, 16.0, 16.0], "m"),
        turned([7.0, 1.0, 13.0], [9.0, 15.0, 15.5], "w", 'z', 38.0, [8.0, 8.0, 14.0]),
    ])
}
/// A lattice's rail alone.
fn lattice_rail() -> String {
    model(&[b([0.0, 12.0, 12.6], [16.0, 15.0, 16.0], "w")])
}
/// A big pipe down the wall behind (hugging it), its clamp.
fn pipe_horizontal()->String {
    model(&[b([4.0,5.0,0.0],[12.0,11.0,16.0],"m"),b([5.0,4.0,0.0],[11.0,12.0,16.0],"m"),
        b([3.0,4.0,0.0],[13.0,12.0,1.5],"r"),b([3.0,4.0,14.5],[13.0,12.0,16.0],"r")])
}
fn pipe_down() -> String {
    model(&[
        b([4.5, 0.0, 9.0], [11.5, 16.0, 16.0], "p"),
        b([5.5, 0.0, 8.0], [10.5, 16.0, 16.0], "p"),
        b([3.8, 6.0, 8.3], [12.2, 8.5, 16.0], "r"),
    ])
}
/// A big pipe coming over the roof's edge and down the wall behind.
fn pipe_bend() -> String {
    model(&[
        b([4.5, 0.0, 9.0], [11.5, 9.0, 16.0], "p"),
        b([5.5, 0.0, 8.0], [10.5, 10.0, 16.0], "p"),
        b([4.5, 4.0, 9.0], [11.5, 11.0, 26.0], "p"),
        b([5.5, 3.0, 10.0], [10.5, 12.0, 26.0], "p"),
        b([3.8, 3.5, 13.5], [12.2, 11.5, 16.0], "r"),
    ])
}
/// A big pipe's foot into a round grated drain at the wall behind.
fn drain() -> String {
    let mut boxes = vec![
        b([4.5, 5.0, 9.0], [11.5, 16.0, 16.0], "p"),
        b([5.5, 5.0, 8.0], [10.5, 16.0, 16.0], "p"),
        b([2.0, 0.0, 14.0], [14.0, 7.0, 16.2], "r"),
        b([3.0, 0.0, 13.0], [13.0, 8.0, 14.0], "r"),
        b([3.5, 0.0, 15.5], [12.5, 6.0, 16.4], "n"),
    ];
    for x in [5.0f32, 7.5, 10.0] {
        boxes.push(b([x, 0.0, 14.4], [x + 1.0, 6.5, 15.5], "r"));
    }
    model(&boxes)
}
/// A lean-to canopy's run against the wall behind: planks falling away
/// from the wall's top out over the street, ragged at their ends, a bearer
/// under them.
fn canopy() -> String {
    let mut boxes = Vec::new();
    let lengths = [24.0f32, 21.0, 25.5, 22.5];
    for (i, x) in [0.2f32, 4.2, 8.2, 12.2].into_iter().enumerate() {
        let reach = lengths[i];
        boxes.push(turned([x, 12.0, 16.0 - reach], [x + 3.6, 13.6, 16.0], "k", 'x', -22.0, [8.0, 13.0, 16.0]));
    }
    boxes.push(turned([-0.5, 9.5, -2.0], [16.5, 11.5, 1.0], "w", 'x', -22.0, [8.0, 13.0, 16.0]));
    boxes.push(b([-0.5, 13.0, 14.0], [16.5, 16.0, 16.0], "w"));
    model(&boxes)
}
/// A board of the slums' signs on an arm out of the wall behind.
fn sign() -> String {
    model(&[
        b([7.0, 13.0, 4.0], [9.0, 15.0, 16.0], "w"),
        b([1.0, -2.0, 6.5], [15.0, 13.0, 7.5], "s"),
        b([0.5, -2.5, 6.2], [15.5, -1.5, 7.8], "w"),
        b([0.5, 12.5, 6.2], [15.5, 13.5, 7.8], "w"),
    ])
}
/// A wooden A-frame on a roof (the slums' hoists): two long poles leaning
/// together, a bar across, a beam out of its top.
fn a_frame() -> String {
    model(&[
        turned([1.0, 0.0, 6.5], [4.0, 38.0, 9.5], "w", 'z', -16.0, [2.5, 0.0, 8.0]),
        turned([12.0, 0.0, 6.5], [15.0, 38.0, 9.5], "w", 'z', 16.0, [13.5, 0.0, 8.0]),
        b([1.0, 16.0, 6.8], [15.0, 18.5, 9.2], "w"),
        turned([6.5, 32.0, -10.0], [9.5, 35.0, 18.0], "w", 'x', -14.0, [8.0, 33.5, 8.0]),
    ])
}
/// A ditch's rim on the street's edge: a rounded lip, its slope down into
/// the ditch before it (the block in front, a block lower).
fn ditch_rim() -> String {
    // The lip on the street's edge (the block's front half), its slope down
    // the block in front to the ditch's floor a block lower.
    let boxes:Vec<_>=minecraftoss_core::mashup_shapes::haven_ditch_rim().into_iter()
        .map(|p|b([p[0],p[1],p[2]],[p[3],p[4],p[5]],"e")).collect();
    model(&boxes)
}
/// A dark wooden post (a canopy's, a lattice's), along its rod.
fn post() -> String {
    model(&[b([5.5, 0.0, 6.0], [10.5, 16.0, 10.0], "w"), b([6.0, 0.0, 5.5], [10.0, 16.0, 10.5], "w")])
}

/// The ground floor's top course with a rolled band instead of stones
/// (`ctyslumb`'s big houses: a grey round ledge over their ground floors).
fn roll_post() -> String {
    let mut boxes = battered(2, true);
    boxes.push(b([-3.0, 10.5, -3.0], [19.0, 14.5, 19.0], "t"));
    boxes.push(b([-2.2, 9.5, -2.2], [18.2, 15.5, 18.2], "t"));
    model(&boxes)
}
fn roll_arm() -> String {
    let mut boxes = battered(2, false);
    boxes.push(b([-3.0, 10.5, 0.0], [19.0, 14.5, 8.0], "t"));
    boxes.push(b([-2.2, 9.5, 0.0], [18.2, 15.5, 8.0], "t"));
    model(&boxes)
}
/// A round window of teal metal on the wall behind, dark glass in it.
fn porthole() -> String {
    model(&[
        b([4.5, 2.5, 14.0], [11.5, 13.5, 16.0], "o"),
        b([2.5, 4.5, 14.0], [13.5, 11.5, 16.0], "o"),
        b([3.3, 3.3, 14.0], [12.7, 12.7, 16.0], "o"),
        b([5.0, 5.0, 13.6], [11.0, 11.0, 14.0], "n"),
        b([4.0, 6.0, 13.6], [12.0, 10.0, 14.0], "n"),
        b([6.0, 4.0, 13.6], [10.0, 12.0, 14.0], "n"),
        b([7.3, 1.0, 13.0], [8.7, 3.0, 16.0], "o"),
    ])
}
/// Plaster fallen off the wall behind, its bricks showing.
fn fallen_plaster() -> String {
    model(&[
        b([3.0, 4.0, 15.3], [12.0, 11.0, 16.0], "q"),
        b([1.5, 6.0, 15.3], [3.0, 10.0, 16.0], "q"),
        b([12.0, 5.0, 15.3], [14.0, 9.0, 16.0], "q"),
        b([5.0, 2.5, 15.3], [10.0, 4.0, 16.0], "q"),
        b([4.0, 11.0, 15.3], [9.0, 13.0, 16.0], "q"),
    ])
}
/// A thick timber pillar's foot: a block of stone, the timber on it.
fn pillar_foot() -> String {
    model(&[
        b([2.5, 0.0, 2.5], [13.5, 4.0, 13.5], "g"),
        b([3.5, 4.0, 3.5], [12.5, 5.0, 12.5], "g"),
        b([4.0, 5.0, 4.0], [12.0, 16.0, 12.0], "w"),
    ])
}
/// A thick timber pillar's shaft, a bracket at its top spreading under
/// what it bears.
fn pillar_shaft() -> String {
    model(&[
        b([4.2, 0.0, 4.2], [11.8, 16.0, 11.8], "w"),
        b([3.5, 12.0, 3.5], [12.5, 16.0, 12.5], "w"),
        turned([-2.0, 13.0, 6.5], [18.0, 15.5, 9.5], "w", 'z', 0.0, [8.0, 14.0, 8.0]),
    ])
}

// ------------------------------------------------------------- blocks

type Parts = &'static [(&'static str, fn() -> String)];

pub(crate) const BASE1: [(&str, fn() -> String); 3] = [("_post", base1_post), ("_side", base1_arm), ("_side_tall", base1_arm)];
pub(crate) const BASE2: [(&str, fn() -> String); 3] = [("_post", base2_post), ("_side", base2_arm), ("_side_tall", base2_arm)];
pub(crate) const BASE3: [(&str, fn() -> String); 3] = [("_post", base3_post), ("_side", base3_arm), ("_side_tall", base3_arm)];
const ROLL: [(&str, fn() -> String); 3] = [("_post", roll_post), ("_side", roll_arm), ("_side_tall", roll_arm)];
const PLASTER: [(&str, fn() -> String); 3] = [("_post", plain_post), ("_side", plain_arm), ("_side_tall", plain_arm)];
const CROWN: [(&str, fn() -> String); 3] = [("_post", crown_post), ("_side", crown_arm), ("_side_tall", crown_arm)];
const PLANKS: [(&str, fn() -> String); 3] = [("_post", planks_post), ("_side", planks_arm), ("_side_tall", planks_arm)];
const BASE_WINDOW: [(&str, fn() -> String); 5] = [("_post", window_bar), ("_side", base_window_half), ("_side_alt", base_window_half_alt), ("_noside", nothing), ("_noside_alt", nothing)];
const UPPER_WINDOW: [(&str, fn() -> String); 5] = [("_post", window_bar), ("_side", upper_window_half), ("_side_alt", upper_window_half_alt), ("_noside", nothing), ("_noside_alt", nothing)];

/// The slums' stuff: their plasters laid over the world (`w4_`), dark wood,
/// copper pipes, pale stones.
const GREY: &[(&str, &str)] = &[("g", "w4_city-slumbase-wall"), ("t", "city-wall-plain-1"), ("d", "city-dirtywood")];
const OCHRE: &[(&str, &str)] = &[("g", "w4_city-slumwall-06"), ("d", "city-dirtywood")];
const BLUE: &[(&str, &str)] = &[("g", "w4_city-slumwall-07"), ("d", "city-dirtywood")];
const PALE: &[(&str, &str)] = &[("g", "w4_city-wall-plain-1"), ("d", "city-dirtywood")];
const WOOD: &[(&str, &str)] = &[("d", "w4_city-dirtywood"), ("w", "city-dirtywood-small"), ("g", "city-dirtywood")];
const FITTINGS: &[(&str, &str)] = &[("o", "city-slum-greenmetal-tube"), ("q", "city-slum-stonewall-bricks"), ("w", "city-dirtywood"), ("m", "city-slum-bigpipe-02"), ("p", "city-slum-bigpipe-04"), ("r", "city-slum-bigpipe-02"), ("n", "common-black"), ("k", "city-slum-roof-1"), ("e", "w4_city-slum-ditch-wall-top-02"), ("g", "w4_city-slumbase-wall"), ("d", "city-dirtywood")];

const fn set(path: &'static str, vanilla: &'static str, models: Parts, textures: &'static [(&'static str, &'static str)]) -> JakBlock {
    JakBlock { path, look: BlockLook::Set { vanilla, models, textures } }
}
const fn shaped(path: &'static str, vanilla: &'static str, model: fn() -> String, textures: &'static [(&'static str, &'static str)]) -> JakBlock {
    JakBlock { path, look: BlockLook::Shaped { vanilla, model, textures } }
}

pub const BARRIO_BLOCKS: [JakBlock; 40] = [
    set("barrio_base1_pared", "cobblestone_wall", &BASE1, GREY),
    set("barrio_base2_pared", "cobblestone_wall", &BASE2, GREY),
    set("barrio_base3_pared", "cobblestone_wall", &BASE3, GREY),
    set("barrio_cornisa_pared", "cobblestone_wall", &ROLL, GREY),
    shaped("barrio_ojo", "carved_pumpkin", porthole, FITTINGS),
    shaped("barrio_desconchon", "carved_pumpkin", fallen_plaster, FITTINGS),
    shaped("barrio_pilar_pie", "lightning_rod", pillar_foot, FITTINGS),
    shaped("barrio_pilar", "lightning_rod", pillar_shaft, FITTINGS),
    set("barrio_ocre_pared", "cobblestone_wall", &PLASTER, OCHRE),
    set("barrio_azul_pared", "cobblestone_wall", &PLASTER, BLUE),
    set("barrio_gris_pared", "cobblestone_wall", &PLASTER, PALE),
    set("barrio_tablas_pared", "cobblestone_wall", &PLANKS, WOOD),
    set("barrio_corona_pared", "cobblestone_wall", &CROWN, WOOD),
    set("barrio_base_vano", "glass_pane", &BASE_WINDOW, GREY),
    set("barrio_ocre_vano", "glass_pane", &UPPER_WINDOW, OCHRE),
    set("barrio_azul_vano", "glass_pane", &UPPER_WINDOW, BLUE),
    set("barrio_gris_vano", "glass_pane", &UPPER_WINDOW, PALE),
    shaped("barrio_portal", "carved_pumpkin", portal, FITTINGS),
    shaped("barrio_vigas", "carved_pumpkin", beam_ends, FITTINGS),
    shaped("barrio_entramado", "carved_pumpkin", lattice_post, FITTINGS),
    shaped("barrio_larguero", "carved_pumpkin", lattice_rail, FITTINGS),
    shaped("barrio_tubo_horizontal", "carved_pumpkin", pipe_horizontal, FITTINGS),
    shaped("barrio_tubo", "carved_pumpkin", pipe_down, FITTINGS),
    shaped("barrio_codo", "carved_pumpkin", pipe_bend, FITTINGS),
    shaped("barrio_desague", "carved_pumpkin", drain, FITTINGS),
    shaped("barrio_toldo", "carved_pumpkin", canopy, FITTINGS),
    shaped("barrio_cartel", "carved_pumpkin", sign, &[("w", "city-dirtywood"), ("s", "city-slum-hangsign-01")]),
    shaped("barrio_cartel_b", "carved_pumpkin", sign, &[("w", "city-dirtywood"), ("s", "city-slum-hangsign-03")]),
    shaped("barrio_andamio", "carved_pumpkin", a_frame, FITTINGS),
    shaped("barrio_zanja_borde", "carved_pumpkin", ditch_rim, FITTINGS),
    shaped("barrio_poste", "lightning_rod", post, FITTINGS),
    JakBlock { path: "barrio_losa", look: BlockLook::All("w4_slum-ground-01") },
    JakBlock { path: "barrio_losa_capa", look: BlockLook::Layers { texture: "w4_slum-ground-01", full: "barrio_losa" } },
    JakBlock { path: "barrio_zanja", look: BlockLook::All("w4_slum-ditch-bottom-01") },
    JakBlock { path: "barrio_azotea", look: BlockLook::Slab { texture: "w4_city-slum-roof", double: "barrio_losa" } },
    JakBlock { path: "barrio_suelo", look: BlockLook::All("w4_city-dirtywood-small") },
    JakBlock { path: "barrio_tablon_slab", look: BlockLook::Slab { texture: "w2_city-slum-wood-plain", double: "barrio_tablon_slab" } },
    JakBlock { path: "haven_puerta_alta", look: BlockLook::Like { vanilla:"oak_door",textures:&[("*","city-slum-door-01")] } },
    shaped("barrio_portal_doble","carved_pumpkin",double_portal,FITTINGS),
    set("barrio_hierbajos", "leaf_litter", &crate::jak_ground_cover::PARTS, &[("g", "city-slum-cattail-grass")]),
];

pub const BARRIO_ITEMS: [JakItem; 40] = [
    JakItem { path: "barrio_base1_pared", title: "Muro en talud del barrio (pie)", kind: JakKind::Block },
    JakItem { path: "barrio_base2_pared", title: "Muro en talud del barrio (medio)", kind: JakKind::Block },
    JakItem { path: "barrio_base3_pared", title: "Muro en talud del barrio (remate)", kind: JakKind::Block },
    JakItem { path: "barrio_cornisa_pared", title: "Muro con cornisa redondeada", kind: JakKind::Block },
    JakItem { path: "barrio_ojo", title: "Ventana redonda", kind: JakKind::Block },
    JakItem { path: "barrio_desconchon", title: "DesconchÃ³n", kind: JakKind::Block },
    JakItem { path: "barrio_pilar_pie", title: "Pie de pilar de madera", kind: JakKind::Block },
    JakItem { path: "barrio_pilar", title: "Pilar de madera", kind: JakKind::Block },
    JakItem { path: "barrio_ocre_pared", title: "Muro ocre del barrio", kind: JakKind::Block },
    JakItem { path: "barrio_azul_pared", title: "Muro azul del barrio", kind: JakKind::Block },
    JakItem { path: "barrio_gris_pared", title: "Muro gris del barrio", kind: JakKind::Block },
    JakItem { path: "barrio_tablas_pared", title: "Pared de tablones del barrio", kind: JakKind::Block },
    JakItem { path: "barrio_corona_pared", title: "Corona de tablones", kind: JakKind::Block },
    JakItem { path: "barrio_base_vano", title: "Ventana en nicho del barrio", kind: JakKind::Block },
    JakItem { path: "barrio_ocre_vano", title: "Ventana ocre del barrio", kind: JakKind::Block },
    JakItem { path: "barrio_azul_vano", title: "Ventana azul del barrio", kind: JakKind::Block },
    JakItem { path: "barrio_gris_vano", title: "Ventana gris del barrio", kind: JakKind::Block },
    JakItem { path: "barrio_portal", title: "Portal de nicho del barrio", kind: JakKind::Block },
    JakItem { path: "barrio_vigas", title: "Cabezas de viga", kind: JakKind::Block },
    JakItem { path: "barrio_entramado", title: "Poste de entramado", kind: JakKind::Block },
    JakItem { path: "barrio_larguero", title: "Larguero de entramado", kind: JakKind::Block },
    JakItem {path:"barrio_tubo_horizontal",title:"Tubería horizontal de Haven",kind:JakKind::Block},
    JakItem { path: "barrio_tubo", title: "TuberÃ­a de cobre", kind: JakKind::Block },
    JakItem { path: "barrio_codo", title: "Codo de tuberÃ­a de cobre", kind: JakKind::Block },
    JakItem { path: "barrio_desague", title: "DesagÃ¼e con rejilla", kind: JakKind::Block },
    JakItem { path: "barrio_toldo", title: "Tejadillo de tablones", kind: JakKind::Block },
    JakItem { path: "barrio_cartel", title: "Cartel del barrio", kind: JakKind::Block },
    JakItem { path: "barrio_cartel_b", title: "Cartel del barrio (amarillo)", kind: JakKind::Block },
    JakItem { path: "barrio_andamio", title: "Caballete de madera", kind: JakKind::Block },
    JakItem { path: "barrio_zanja_borde", title: "Borde de zanja", kind: JakKind::Block },
    JakItem { path: "barrio_poste", title: "Poste oscuro", kind: JakKind::Block },
    JakItem { path: "barrio_losa", title: "Losas del barrio", kind: JakKind::Block },
    JakItem { path: "barrio_losa_capa", title: "Capa de losas del barrio", kind: JakKind::Block },
    JakItem { path: "barrio_zanja", title: "Fondo de zanja", kind: JakKind::Block },
    JakItem { path: "barrio_azotea", title: "Azotea del barrio", kind: JakKind::Block },
    JakItem { path: "barrio_suelo", title: "Suelo de tablas del barrio", kind: JakKind::Block },
    JakItem { path: "barrio_tablon_slab", title: "Losa de tablones del barrio", kind: JakKind::Block },
    JakItem { path: "haven_puerta_alta", title: "Puerta alta del barrio", kind: JakKind::Block },
    JakItem { path: "barrio_portal_doble", title: "Portal doble del barrio", kind: JakKind::Block },
    JakItem { path: "barrio_hierbajos", title: "Hierbajos del barrio", kind: JakKind::Block },
];

// ------------------------------------------------------------- buildings

/// What an upper storey is plastered with.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Plaster {
    Ochre,
    Blue,
    Pale,
    Planks,
    /// Actual facade metres extracted from Haven's GLB, closed at the back.
    Metal,
}

impl Plaster {
    fn wall(self) -> &'static str {
        match self {
            Self::Ochre => "jak:barrio_ocre_pared",
            Self::Blue => "jak:barrio_azul_pared",
            Self::Pale => "jak:barrio_gris_pared",
            Self::Planks => "jak:barrio_tablas_pared",
            Self::Metal => "jak:muro_haven_chapa",
        }
    }
    fn window(self) -> &'static str {
        match self {
            Self::Ochre => "jak:barrio_ocre_vano",
            Self::Blue => "jak:barrio_azul_vano",
            Self::Pale | Self::Planks | Self::Metal => "jak:barrio_gris_vano",
        }
    }
}

/// A building's plan: its footprint, its upper storeys (each three high,
/// over the ground floor's three), what it has.
#[derive(Clone, Copy, Debug)]
pub struct Spec {
    pub w: i32,
    pub d: i32,
    pub upper: &'static [Plaster],
    /// A copper pipe from its roof down its front into a drain.
    pub pipe: bool,
    /// A lattice of dark posts over its first upper storey's front.
    pub lattice: bool,
    /// A canopy of planks on posts along its front.
    pub canopy: bool,
    /// A wooden A-frame on its roof.
    pub a_frame: bool,
    /// A sign on its front (`0` none, `1`, `2`).
    pub sign: u8,
    /// Its upper storeys out two blocks over a porch on thick pillars.
    pub porch: bool,
    /// A rolled band over its ground floor instead of the row of stones.
    pub cornice: bool,
    /// Round windows on its upper storeys.
    pub portholes: bool,
    /// Plaster fallen off its upper storeys here and there.
    pub patches: bool,
    /// Upper front set back two metres, with a usable front terrace.
    pub terrace: bool,
    /// Plank gallery along the rear, with a door from the first floor.
    pub gallery: bool,
    pub staggered: bool,
}

/// No features (what others are made from).
pub const PLAIN: Spec = Spec { w: 7, d: 6, upper: &[], pipe: false, lattice: false, canopy: false, a_frame: false, sign: 0, porch: false, cornice: false, portholes: false, patches: false, terrace: false, gallery: false, staggered:false };
pub const PORCH: Spec = Spec { w: 8, d: 6, upper: &[Plaster::Pale], porch: true, sign: 1, ..PLAIN };
pub const BIG_BLUE: Spec = Spec { w: 10, d: 7, upper: &[Plaster::Blue, Plaster::Blue], cornice: true, portholes: true, patches: true, pipe: true, ..PLAIN };

/// The upper storeys a building may have (the slums' mixes).
const UPPERS: [&[Plaster]; 15] = [
    &[],
    &[Plaster::Ochre],
    &[Plaster::Blue],
    &[Plaster::Pale],
    &[Plaster::Ochre, Plaster::Pale],
    &[Plaster::Blue, Plaster::Planks],
    &[Plaster::Ochre, Plaster::Blue],
    &[Plaster::Pale, Plaster::Planks],
    &[Plaster::Ochre, Plaster::Pale, Plaster::Planks],
    &[Plaster::Blue, Plaster::Ochre, Plaster::Planks],
    &[Plaster::Blue, Plaster::Blue],
    &[Plaster::Metal],
    &[Plaster::Metal, Plaster::Planks],
    &[Plaster::Ochre, Plaster::Metal],
    &[Plaster::Pale, Plaster::Metal, Plaster::Planks],
];

/// A building of the slums made up from `h` (a hash), at most `max_w`
/// wide and `max_d` deep: its size, its storeys and plasters, its
/// features each as often as the slums have them.
pub fn random_spec(h: u64, max_w: i32, max_d: i32) -> Spec {
    let bit = |k: u32, of: u64| (h >> k) % of == 0;
    let w = (5 + (h % 6) as i32).min(max_w).max(4);
    let d = (6 + ((h >> 4) % 4) as i32).min(max_d).max(4);
    let upper = UPPERS[((h >> 8) % UPPERS.len() as u64) as usize];
    Spec {
        w,
        d,
        upper,
        pipe: bit(12, 2),
        lattice: !upper.is_empty() && bit(14, 3),
        canopy: w >= 8 && upper.is_empty() && bit(16, 2),
        a_frame: upper.len() <= 1 && bit(18, 3),
        sign: ((h >> 20) % 4).min(2) as u8,
        porch: !upper.is_empty() && d >= 6 && bit(23, 3),
        cornice: bit(26, 3),
        portholes: upper.contains(&Plaster::Blue) && bit(28, 2),
        patches: !upper.is_empty() && bit(30, 2),
        terrace: !upper.is_empty() && d >= 7 && bit(33, 3),
        gallery: !upper.is_empty() && bit(36, 3),
        staggered:d>=8 && bit(39,4),
    }
}

/// The slums' kinds of building (Jak II's `ctysluma`): its freestanding
/// battered huts, its two-storey houses, its tall towers with a wooden top
/// storey, its long low rows under canopies.
pub const BUNKER: Spec = Spec { w: 7, d: 6, upper: &[], pipe: true, lattice: false, canopy: false, a_frame: true, sign: 0, ..PLAIN };
pub const HOUSE_OCHRE: Spec = Spec { w: 8, d: 6, upper: &[Plaster::Ochre], pipe: true, lattice: false, canopy: false, a_frame: false, sign: 1, ..PLAIN };
pub const HOUSE_BLUE: Spec = Spec { w: 7, d: 6, upper: &[Plaster::Blue], pipe: false, lattice: true, canopy: false, a_frame: false, sign: 0, ..PLAIN };
pub const TOWER: Spec = Spec { w: 6, d: 6, upper: &[Plaster::Ochre, Plaster::Pale, Plaster::Planks], pipe: true, lattice: true, canopy: false, a_frame: false, sign: 2, ..PLAIN };
pub const ROW: Spec = Spec { w: 11, d: 5, upper: &[], pipe: false, lattice: false, canopy: true, a_frame: false, sign: 2, ..PLAIN };
pub const TERRACE: Spec = Spec { w: 10, d: 9, upper: &[Plaster::Ochre, Plaster::Metal], terrace: true, pipe: true, patches: true, sign: 2, ..PLAIN };
pub const WORKSHOP: Spec = Spec { w: 11, d: 8, upper: &[Plaster::Metal], canopy: true, a_frame: true, gallery: true, ..PLAIN };
pub const GALLERY: Spec = Spec { w: 9, d: 8, upper: &[Plaster::Planks, Plaster::Metal], gallery: true, lattice: true, cornice: true, ..PLAIN };
pub const STEPPED: Spec = Spec { w: 8, d: 9, upper: &[Plaster::Blue, Plaster::Pale, Plaster::Metal], terrace: true, gallery: true, portholes: true, ..PLAIN };


/// Composable families measured against ctysluma/b/c: shops, galleries,
/// stepped houses, tenements and wooden upper rooms. Indexed for inspection.
pub const ARCHETYPE_NAMES:[&str;28]=["Taller bajo","Mercado cubierto","Casa ocre","Casa azul","Casa encalada","Torre de madera","Galerias de chapa","Terrazas escalonadas","Viviendas sobre soportal","Almacen de tablas","Pension de esquina","Casa de patios","Mirador azul","Almacen metalico","Casa de balcones","Torre estrecha","Tienda con vivienda","Taller con galeria","Bloque de vecinos","Casa alta retranqueada","Posada de madera","Casa del canal","Vivienda con tejadillo","Torrecilla de chapa","Lonja industrial","Casa de mercader con galeria","Mirador del canal","Posada de terrazas"];
pub fn archetype(index:usize,w:i32,d:i32)->Spec {
    use Plaster::*;
    // Dedicated silhouettes, rather than a new colour on the same box.
    match index % ARCHETYPE_NAMES.len() {
        24 => return Spec {w:w.clamp(5,18),d:d.clamp(5,10),upper:&[Metal],canopy:true,a_frame:true,pipe:true,cornice:true,sign:2,..PLAIN},
        25 => return Spec {w:w.clamp(5,14),d:d.clamp(5,12),upper:&[Ochre,Planks],gallery:true,porch:true,lattice:true,pipe:true,sign:1,..PLAIN},
        26 => return Spec {w:w.clamp(5,9),d:d.clamp(5,10),upper:&[Blue,Pale,Blue],terrace:d>=7,portholes:true,pipe:true,cornice:true,gallery:true,..PLAIN},
        27 => return Spec {w:w.clamp(5,15),d:d.clamp(5,12),upper:&[Pale,Ochre,Planks],terrace:d>=7,staggered:d>=8,gallery:true,lattice:true,cornice:true,sign:1,..PLAIN},
        _=>{}
    }
    let upper:&'static [Plaster]=match index%24 {
        0|1=>&[],2=>&[Ochre],3=>&[Blue],4=>&[Pale],5=>&[Ochre,Planks],6=>&[Metal,Planks],
        7=>&[Ochre,Blue,Pale],8=>&[Pale,Planks],9=>&[Planks],10=>&[Ochre,Pale,Planks],11=>&[Pale,Ochre],
        12=>&[Blue,Blue],13=>&[Metal],14=>&[Planks,Blue],15=>&[Ochre,Pale,Metal,Planks],
        16=>&[Ochre],17=>&[Metal,Planks],18=>&[Pale,Ochre,Pale],19=>&[Blue,Pale,Metal],
        20=>&[Planks,Planks],21=>&[Blue,Ochre],22=>&[Pale],_=>&[Metal,Planks,Metal],
    };
    let i=index%24;
    Spec {w:w.clamp(5,15),d:d.clamp(5,10),upper,pipe:i%5!=1,lattice:matches!(i,5|8|10|12|17|20),
        canopy:matches!(i,0|1|9|16|17),a_frame:matches!(i,0|5|9|13|20|23),sign:if i%4==0 {2}else{1},
        porch:matches!(i,8|10|16|18),cornice:i%3==0,portholes:matches!(i,3|12|19|21),patches:i%2==0,
        terrace:d>=7 && matches!(i,3|7|11|12|14|18|19|22),gallery:matches!(i,2|6|9|11|12|14|17|18|20|21),
        staggered:d>=8 && matches!(i,7|11|15|18|19|23),}
}
/// A building of the slums at `origin` (its floor's middle, on the
/// street), its front toward `front`, as `spec` makes it. Whole all round;
/// inside a floor a storey, a ladder up to a hatch in the roof, a bed and
/// a chest of Jak 2's things.
pub fn building(origin: (i32, i32, i32), front: u8, spec: Spec) -> Vec<Placed> {
    let Spec { w, d, .. } = spec;
    let s = Site { origin, front, w, d };
    let mut p = Plan::default();
    let storeys = 1 + spec.upper.len() as i32;
    let height = storeys * 3;
    // Its upper storeys' front: out two blocks over a porch, or the
    // ground floor's.
    let terrace = spec.terrace && d >= 7 && !spec.upper.is_empty();
    let porch = spec.porch && !terrace && !spec.upper.is_empty();
    let fz = if terrace { 2 } else if porch { -2 } else { 0 };
    // The ground floor: battered courses, its windows in niches.
    let windows = |x: i32, z: i32, way: u8| -> bool {
        let along = if way % 2 == 0 { x } else { z };
        let span = if way % 2 == 0 { w } else { d };
        match way {
            0 => along != w / 2 && (along == 2 || along == span - 3 || (span > 9 && along == span / 2 + 3)),
            2 => along == span / 2,
            _ => along == span / 2,
        }
    };
    for (x, z, way, corner) in ring_cells(0, 0, w, d) {
        for k in 0..3 {
            let cell = match k {
                1 if !corner && windows(x, z, way) => "jak:barrio_base_vano",
                0 => "jak:barrio_base1_pared",
                1 => "jak:barrio_base2_pared",
                _ if spec.cornice => "jak:barrio_cornisa_pared",
                _ => "jak:barrio_base3_pared",
            };
            p.put(s.at(x, k, z), Cell::Join(cell));
        }
    }
    // The upper storeys: their plaster, two or three windows a side.
    for (n, plaster) in spec.upper.iter().enumerate() {
        let y0 = 3 + 3 * n as i32;
        let previous=if n==0 {fz}else{(fz+(n as i32-1).min(2)).min(d-4)};
        let fz=if spec.staggered {(fz+(n as i32).min(2)).min(d-4)}else{fz};
        if spec.staggered && n>0 && fz>previous {
            for x in 0..w {for z in previous..fz {p.fixed(s.at(x,y0-1,z),"jak:barrio_suelo");if x==0||x==w-1||z==previous {p.put(s.at(x,y0,z),Cell::Join("jak:haven_baranda"));}}}
            for (r,half) in [(0,"lower"),(1,"upper")] {p.fixed(s.at(w/2,y0+r,fz),s.facing(&format!("jak:haven_puerta_alta[facing={{f}},half={half},hinge=left,open=false,powered=false]"),0));}
        }
        for (x, z, way, corner) in ring_cells(0, fz, w, d - fz) {
            let along = if way % 2 == 0 { x } else { z - fz };
            let span = if way % 2 == 0 { w } else { d - fz };
            for r in 0..3 {
                let window = !corner && r == 1 && (along % 3 == 1 || (way != 0 && along == span / 2)) && along > 0 && along < span - 1;
                let porthole = window && spec.portholes && *plaster == Plaster::Blue && along % 2 == 0;
                p.put(s.at(x, y0 + r, z), Cell::Join(if window && !porthole { plaster.window() } else { plaster.wall() }));
                // Round windows and fallen plaster on the wall's face.
                let (ox, oz) = match way {
                    0 => (x, z - 1),
                    2 => (x, z + 1),
                    1 => (x + 1, z),
                    _ => (x - 1, z),
                };
                if porthole {
                    p.fixed(s.at(ox, y0 + r, oz), s.facing("jak:barrio_ojo[facing={f}]", way));
                } else if spec.patches && !corner && !window && r == 1 && (x * 7 + z * 13 + n as i32 * 5) % 9 == 0 {
                    p.fixed(s.at(ox, y0 + r, oz), s.facing("jak:barrio_desconchon[facing={f}]", way));
                }
            }
        }
    }
    if spec.staggered {for n in 1..spec.upper.len(){let z=(fz+(n as i32).min(2)).min(d-4);for (dy,half) in [(0,"lower"),(1,"upper")]{p.fixed(s.at(w/2,3+3*n as i32+dy,z),s.facing(&format!("jak:haven_puerta_alta[facing={{f}},half={half},hinge=left,open=false,powered=false]"),0));}}}
    // The porch: its ceiling the upper floor out over it, its thick
    // pillars at its front.
    if porch {
        for x in 0..w {
            for z in fz..0 {
                p.fixed(s.at(x, 2, z), "jak:barrio_suelo");
            }
        }
        let mut pillars = vec![0, w - 1];
        if w >= 8 {
            pillars.push(w / 2 - 1);
        }
        for x in pillars {
            p.fixed(s.at(x, 0, fz), "jak:barrio_pilar_pie[facing=up,powered=false,waterlogged=false]");
            p.fixed(s.at(x, 1, fz), "jak:barrio_pilar[facing=up,powered=false,waterlogged=false]");
        }
    }
    let roof_fz=if spec.staggered {(fz+(spec.upper.len() as i32-1).clamp(0,2)).min(d-4)}else{fz};
    // Floors and the flat roof.
    for x in 1..w - 1 {
        for z in 1..d - 1 {
            p.fixed(s.at(x, -1, z), "jak:barrio_suelo");
        }
        for z in 1.min(fz + 1)..d - 1 {
            for k in 1..storeys {
                if k == 1 || z > fz { p.fixed(s.at(x, 3 * k - 1, z), "jak:barrio_suelo"); }
            }
            if z > roof_fz { p.fixed(s.at(x, height - 1, z), if spec.upper.contains(&Plaster::Metal) { "jak:tejado_haven" } else { "jak:barrio_azotea[type=top,waterlogged=false]" }); }
        }
    }
    // The crown of planks round the roof.
    // Ctyslum's exposed corner timbers carry the storeys above their base.
    // Their plates and diagonal braces use the original wooden fittings.
    if !spec.lattice && (spec.cornice || spec.pipe) {
        for k in 0..spec.upper.len() as i32 {
            let z=if spec.staggered{(fz+k.min(2)).min(d-4)}else{fz};
            for x in [0,w-1] {for y in 3+3*k..6+3*k {
                p.fixed(s.at(x,y,z-1),s.facing("jak:barrio_entramado[facing={f}]",0));
            }}
        }
    }
    let top_z = if storeys > 1 { roof_fz } else { 0 };
    for (x, z, _, _) in ring_cells(0, top_z, w, d - top_z) {
        p.put(s.at(x, height, z), Cell::Join("jak:barrio_corona_pared"));
    }
    // Beam ends jutting under the roof's edge, every other block.
    for x in (1..w - 1).step_by(2) {
        p.fixed(s.at(x, height - 1, top_z - 1), s.facing("jak:barrio_vigas[facing={f}]", 0));
        p.fixed(s.at(x, height - 1, d), s.facing("jak:barrio_vigas[facing={f}]", 2));
    }
    for z in (top_z + 1..d - 1).step_by(2) {
        p.fixed(s.at(-1, height - 1, z), s.facing("jak:barrio_vigas[facing={f}]", 3));
        p.fixed(s.at(w, height - 1, z), s.facing("jak:barrio_vigas[facing={f}]", 1));
    }
    // The door, set back in the wall, its niche's frame over it.
    let doors: &[i32] = if spec.canopy && w >= 8 { &[2, 8] } else { &[0] };
    for (i, _) in doors.iter().enumerate() {
        let x = if spec.canopy && w >= 8 { [2, w - 3][i] } else { w / 2 - 1 };
        for (y, half) in [(0, "lower"), (1, "upper")] {
            p.fixed(s.at(x, y, 0), s.facing(&format!("jak:haven_puerta_alta[facing={{f}},half={half},hinge=left,open=false,powered=false]"), 0));
        }
        p.fixed(s.at(x, 2, 0), s.facing("jak:barrio_portal[facing={f}]", 0));
    }
    // A copper pipe from the roof's edge down the front into its drain.
    if spec.pipe && !porch && !terrace {
        let x = w - 2;
        p.fixed(s.at(x, height - 1, -1), s.facing("jak:barrio_codo[facing={f}]", 0));
        for y in 1..height - 1 {
            p.fixed(s.at(x, y, -1), s.facing("jak:barrio_tubo[facing={f}]", 0));
        }
        p.fixed(s.at(x, 0, -1), s.facing("jak:barrio_desague[facing={f}]", 0));
    }
    // A lattice of dark posts over the first upper storey's front.
    if spec.lattice && !spec.upper.is_empty() {
        for x in 0..w {
            for y in 3..6 {
                let piece = if x % 2 == 0 && y > 3 { "jak:barrio_entramado" } else { "jak:barrio_larguero" };
                if y == 3 || x % 2 == 0 {
                    p.fixed(s.at(x, y, fz - 1), s.facing(&format!("{piece}[facing={{f}}]"), 0));
                }
            }
        }
    }
    // A canopy of planks along the front on posts.
    if spec.canopy {
        for x in -1..w + 1 {
            p.fixed(s.at(x, 2, -1), s.facing("jak:barrio_toldo[facing={f}]", 0));
        }
        for x in (-1..w + 1).step_by(4).filter(|x| *x != w / 2 && *x != 2 && *x != w - 3) {
            for y in 0..2 {
                p.fixed(s.at(x, y, -2), "jak:barrio_poste[facing=up,powered=false,waterlogged=false]");
            }
        }
    }
    if spec.a_frame {
        p.fixed(s.at(w / 2, height, d / 2), s.facing("jak:barrio_andamio[facing={f}]", 0));
    }
    match spec.sign {
        1 if !spec.lattice => p.fixed(s.at(1, 4, fz - 1), s.facing("jak:barrio_cartel[facing={f}]", 0)),
        2 if !spec.lattice => p.fixed(s.at(w - 2, if spec.upper.is_empty() { 1 } else { 4 }, fz - 1), s.facing("jak:barrio_cartel_b[facing={f}]", 0)),
        _ => {}
    }
    // Original Haven fittings attached to real facade corners and roof edges.
    // The entrances and all balcony doors keep their full clear width.
    if storeys>1 {
        for x in [0,w-1] {
            p.fixed(s.at(x,5,fz-1),s.facing("jak:haven_brazo_farol[facing={f}]",0));
            p.fixed(s.at(x,4,fz-1),"minecraft:lantern[hanging=true,waterlogged=false]");
        }
        if spec.sign==0 && !spec.lattice && !terrace {
            p.fixed(s.at(w/2,4,fz-1),s.facing("jak:haven_estandarte[facing={f}]",0));
        }
        // A connected service pipe follows the side wall and terminates in
        // its vertical downpipe; it is not a floating pedestrian walkway.
        if spec.pipe {
            for z in 1..d-1 {p.fixed(s.at(w,3,z),s.facing("jak:barrio_tubo_horizontal[facing={f}]",0));}
            for y in 0..4 {p.fixed(s.at(w,y,d-1),s.facing("jak:barrio_tubo[facing={f}]",1));}
            p.fixed(s.at(w,0,d-1),s.facing("jak:barrio_desague[facing={f}]",1));
        }
        // Short battered timber ledges beneath each upper side window.
        for k in 1..storeys {for z in (2..d-2).step_by(3) {
            p.fixed(s.at(-1,3*k,z),s.facing("jak:haven_canes[facing={f}]",3));
        }}
    }
    if spec.a_frame || spec.pipe {
        p.fixed(s.at(w-2,height,d-2),s.facing("jak:haven_chimenea_remate[facing={f}]",0));
    }
    // Inside: a ladder up the back wall through every floor to a hatch in
    // the roof, a bed, a chest, air over each floor.
    for y in 0..height - 1 {
        p.fixed(s.at(1, y, d - 2), s.facing("minecraft:ladder[facing={f},waterlogged=false]", 0));
    }
    p.fixed(s.at(1, height - 1, d - 2), s.facing("minecraft:spruce_trapdoor[facing={f},half=top,open=false,powered=false,waterlogged=false]", 0));
    for (part, z) in [("foot", d - 3), ("head", d - 2)] {
        p.fixed(s.at(w - 2, 0, z), s.facing(&format!("minecraft:red_bed[facing={{f}},occupied=false,part={part}]"), 2));
    }
    p.fixed(s.at(w - 3, 0, d - 2), s.facing("minecraft:chest[facing={f},type=single,waterlogged=false]", 0));
    // Larger homes can house a second inhabitant. Keep the double doorway,
    // central passage and the ladder clear, rather than inventing beds in
    // inaccessible upper rooms.
    if w>=8 {for (part,x) in [("foot",1),("head",2)] {
        p.fixed(s.at(x,0,1),s.facing(&format!("minecraft:red_bed[facing={{f}},occupied=false,part={part}]"),1));
    }}
    for k in 0..storeys {
        let front=if k==0 {0}else if spec.staggered {(fz+(k-1).min(2)).min(d-4)}else{fz};
        // A low lamp in the middle blocks the full original NPC bodies
        // in compact homes. Keep the light by the front corner instead.
        p.fixed(s.at(w-2,3*k+1,front+1),"minecraft:lantern[hanging=true,waterlogged=false]");
    }
    for x in 1..w - 1 {
        for k in 0..storeys {
            let z0 = if k == 0 { 1 } else { fz + 1 };
            for z in z0..d - 1 {
                for y in [3 * k, 3 * k + 1] {
                    p.air_if_empty(s.at(x, y, z));
                }
            }
        }
    }
    // Under a porch, its floor's air.
    if porch {
        for x in 0..w {
            for z in fz..0 {
                for y in 0..2 {
                    p.air_if_empty(s.at(x, y, z));
                }
            }
        }
    }
    // Ctyslumc's galleries and recessed rooms: each deck has a real
    // door, continuous floor, railing and supporting posts, on all sides.
    for (on, z0, z1, door_z, way) in [
        (terrace, 0, 1, fz, 0),
        (spec.gallery && storeys > 1, d, d + 1, d - 1, 2),
    ] {
        if !on { continue; }
        for x in 0..w {
            for z in z0..=z1 {
                p.fixed(s.at(x, 2, z), "jak:barrio_suelo");
                for y in 3..6 { p.fixed(s.at(x, y, z), "minecraft:air"); }
                if x == 0 || x == w - 1 || z == if way == 0 { z0 } else { z1 } {
                    p.put(s.at(x, 3, z), Cell::Join("jak:haven_baranda"));
                }
            }
        }
        if way == 2 {
            for x in [0, w - 1] {
                for y in 0..2 { p.fixed(s.at(x, y, z1), "jak:barrio_poste[facing=up,powered=false,waterlogged=false]"); }
            }
        }
        for (y, half) in [(3, "lower"), (4, "upper")] {
            p.fixed(s.at(w / 2, y, door_z), s.facing(&format!("jak:haven_puerta_alta[facing={{f}},half={half},hinge=left,open=false,powered=false]"), way));
        }
    }
    p.tall_storeys(origin.1);
    crate::jak_workshops::add(&mut p,&s,w>=8,false);
    p.doorways("jak:barrio_suelo");
    p.finish()
}

// ------------------------------------------------------------- village

fn mix(seed: i64, a: i32, b: i32, salt: u64) -> u64 {
    let mut h = (seed as u64) ^ ((a as u32 as u64) << 32 | u64::from(b as u32)) ^ salt.wrapping_mul(0x9E37_79B9_7F4A_7C15);
    h ^= h >> 33;
    h = h.wrapping_mul(0xff51_afd7_ed55_8ccd);
    h ^= h >> 33;
    h = h.wrapping_mul(0xc4ce_b9fe_1a85_ec53);
    h ^ (h >> 33)
}

/// A town of the slums at `(cx, cz)`, its plaza's floor `y0`, turned
/// `turn` (`jak_town`'s plan): its ground levelled and paved with
/// flagstones (filled under, cleared over); round its plaza the ring road,
/// along it buildings of every kind the slums have, made up one by one
/// (`random_spec`), towers on its corners; in its plaza a battered hut and
/// a ditch of earth sunk across it inside its rims; tufts of dry grass.
pub fn layout(seed: i64, (cx, cz): (i32, i32), y0: i32, turn: u8, land: &dyn Land) -> Option<crate::jak_town::Town> {
    if !crate::jak_town::gentle_site(land,cx,cz,y0){return None;}
    if !(-11..=11).all(|dx|(-11..=11).all(|dz|land.city_allowed(cx+dx,cz+dz,false)&&!land.wet(cx+dx,cz+dz))) {return None;}
    let mut k = 0u64;
    let mut town = crate::jak_town::plan_seeded(seed,(cx, y0, cz), turn, &mut |max, corner| {
        k += 1;
        let h = mix(seed, cx, cz, 200 + k);
        if corner {
            (6, 6)
        } else {
            ((6 + (h % 7) as i32).min(max), 6 + ((h >> 8) % 4) as i32)
        }
    });
    town.expand_haven(seed);
    town.haven_frontages(seed);
    town.reserve_station(seed,land);
    town.reserve_tower(seed,land);
    town.grade(seed, land, false);
    town.station=town.station.filter(|p|town.lots.iter().any(|l|(l.origin.0,l.origin.2)==*p));
    town.tower=town.tower.filter(|p|town.lots.iter().any(|l|(l.origin.0,l.origin.2)==*p));
    if town.lots.len()<20 {return None;}
    Some(town)
}

/// Normal worlds reserve the full palace inside the same graded district.
/// The exposition world continues to use `layout` and its separate exhibits.
pub fn natural_layout(seed:i64,center:(i32,i32),y:i32,turn:u8,land:&dyn Land)->Option<crate::jak_town::Town>{
    let mut town=layout(seed,center,y,turn,land)?;
    if town.reserve_palace(seed,land){town.grade(seed,land,false);}
    Some(town)
}

pub fn village(seed:i64,center:(i32,i32),y0:i32,turn:u8,land:&dyn Land)->Vec<Placed>{
    let Some(town)=layout(seed,center,y0,turn,land) else{return Vec::new();};
    village_from_layout(seed,town,land)
}

pub fn village_from_layout(seed:i64,town:crate::jak_town::Town,land:&dyn Land)->Vec<Placed>{
    let (cx,y0,cz)=town.center;
    let site = town.site();
    let reach = crate::jak_town::REACH;
    let mut out = town.terrain(land, false);
    out.extend(town.civic(false,land));
    let mut built = std::collections::HashSet::new();
    let mut plots=std::collections::HashMap::new();
    for(i,lot)in town.lots.iter().enumerate(){
        let s=Site{origin:lot.origin,front:lot.front,w:lot.w,d:lot.d};
        for x in 0..lot.w{for z in 0..lot.d{let at=s.at(x,0,z);plots.insert((at.0,at.2),i);}}
    }
    for (i, lot) in town.lots.iter().enumerate() {
        let h = mix(seed, cx, cz, 300 + i as u64);
        let spec = archetype(((h >> 8) as usize+i)%ARCHETYPE_NAMES.len(),lot.w,lot.d);
        let blocks=if town.station==Some((lot.origin.0,lot.origin.2)) {crate::jak_krimson::building(lot.origin,lot.front)}
            else if town.tower==Some((lot.origin.0,lot.origin.2)){crate::jak_tower::civic_plaza(seed,lot.origin)}
            else if crate::jak_bar::lot(&town).is_some_and(|bar|bar.origin==lot.origin){crate::jak_bar::building(lot)}
            else{building(lot.origin,lot.front,spec)};
        for (at, mut state) in blocks {
            // Party walls remain whole. An overhanging gallery or an entry's
            // clearing must not erase a neighbour's room or enter its roof.
            if plots.get(&(at.0,at.2)).is_some_and(|owner|*owner!=i){continue;}
            if state.starts_with("jak:haven_puerta[") || state.starts_with("jak:haven_puerta_alta[") {
                let way=if state.contains("facing=east"){1}else if state.contains("facing=south"){2}else if state.contains("facing=west"){3}else{0};
                let(dx,dz)=crate::jak_facades::dir_step(way);
                if let Some(&other)=plots.get(&(at.0+dx,at.2+dz)).filter(|&&owner|owner!=i){
                    let neighbour=&town.lots[other];let h=mix(seed,cx,cz,300+other as u64);
                    let upper=archetype(((h>>8) as usize+other)%ARCHETYPE_NAMES.len(),neighbour.w,neighbour.d).upper.len() as i32;
                    if at.1<neighbour.origin.1+4*(1+upper)+1 {
                        state="jak:barrio_tablas_pared[east=none,north=none,south=none,up=true,waterlogged=false,west=none]";
                    }
                }
            }
            built.insert(at);
            out.push((at, state));
        }
    }
    // Its middle's mark under the plaza, its bell in the plaza (its people
    // and traffic, `jak2_villagers`).
    out.push((site.at(0, -3, 0), crate::jak_town::MARK));
    out.push((site.at(0, -4, 0), crate::jak_town::EXPANDED_MARK));
    out.push((site.at(0, -5, 0), crate::jak_town::CITY_MARK));
    if town.palace.is_none(){out.push((site.at(0,0,0),"minecraft:bell[attachment=floor,facing=north,powered=false]"));}
    // The plaza's hut, in its far corner, its front to the middle.
    let (hx, _, hz) = site.at(5, 0, 5);
    let hut = building((hx, y0, hz), site.way(0), Spec { w: 6, d: 5, ..BUNKER });
    // Expanded frontages can now occupy the old plaza hut location. Never
    // stamp that legacy hut through a reserved building (including the bar).
    if town.palace.is_none() && !hut.iter().any(|(at, _)| plots.contains_key(&(at.0, at.2))) {
        for (at, state) in hut { built.insert(at); out.push((at, state)); }
    }
    let (ditch, cells) = plaza_ditch(&site, seed);
    let rims:std::collections::HashSet<_>=ditch.iter().filter_map(|(at,s)|{
        if s.starts_with("jak:barrio_zanja_borde"){Some(*at)}
        else if s.starts_with("jak:barrio_tablon_slab"){Some((at.0,at.1+1,at.2))}
        else{None}
    }).collect();
    if town.palace.is_none(){out.extend(ditch);}
    // Keep town-only props and lighting out of the palace's reserved volume.
    out.retain(|(p,s)|*s=="minecraft:air"||!town.palace_owns(*p));
    let props_start=out.len();
    crate::jak_town::original_props(&town, seed, false, land, &mut out);
    let props:std::collections::HashSet<_>=out[props_start..].iter().map(|(p,_)|*p).collect();
    // The slums' own grass mesh, in reusable leaf-litter portions. Keep
    // thresholds, bridge boards and the whole passage between them clear.
    let thresholds:std::collections::HashSet<_>=out.iter().filter(|(_,s)|s.starts_with("jak:haven_puerta"))
        .flat_map(|(at,_)|{let (x,y,z)=*at;(-1..=1).flat_map(move |dx|(-1..=1).map(move |dz|(x+dx,y,z+dz)))}).collect();
    // Ditches and props have already rewritten the graded surface. Only plant
    // on the final paving, never at the pre-excavation height or inside a rim.
    let final_surface:std::collections::HashMap<_,_>=out.iter().copied().collect();
    let mut decorated=std::collections::HashSet::new();
    let (mut leaves,mut weeds)=(0,0);
    for i in 0..1800u64{
        let h=mix(seed,cx,cz,5_000+i);
        let(x,z)=((h%(2*reach as u64)) as i32-reach,((h>>20)%(2*reach as u64)) as i32-reach);
        let at=site.at(x,town.eighths(x,z).div_euclid(8)-y0,z);
        if town.palace_owns(at)||town.eighths(x,z).rem_euclid(8)!=0||town.coverage(x,z)<0.94||cells.contains(&(x,z))||rims.contains(&at)
            ||built.contains(&at)||built.contains(&(at.0,at.1-1,at.2))||thresholds.contains(&at)||props.contains(&at)||decorated.contains(&at)||!land.city_allowed(at.0,at.2,false)||land.wet(at.0,at.2){continue;}
        if final_surface.get(&at).is_some_and(|s|*s!="minecraft:air")
            || final_surface.get(&(at.0,at.1-1,at.2))!=Some(&"jak:barrio_losa") {continue;}
        let near_ditch=(-2..=2).any(|dx|(-2..=2).any(|dz|cells.contains(&(x+dx,z+dz))));
        let near_wall=[(1,0),(-1,0),(0,1),(0,-1)].into_iter().any(|(dx,dz)|built.contains(&(at.0+dx,at.1,at.2+dz)));
        if !near_ditch&&!near_wall{continue;}
        if town.ground(x,z)==crate::jak_town::Ground::Road&&!near_wall{continue;}
        let weed=h%4==0;
        if weed&&weeds>=64||!weed&&leaves>=128{continue;}
        if weed{weeds+=1;}else{leaves+=1;}
        let state=site.facing(&format!("jak:barrio_hierbajos[facing={{f}},segment_amount={}]",1+(h>>28)%4),((h>>34)%4) as u8);
        out.push((at,state));decorated.insert(at);
        if leaves>=128&&weeds>=64{break;}
    }
    crate::jak_beds::arrange(&mut out);
    out
}

/// Ctysluma's broad sunken canal, keeping the bell and the plaza house on
/// their original ground. A two-block-wide timber crossing stays flush
/// with both banks; the same slabs are reusable in the player's builds.
fn plaza_ditch(site: &Site, seed: i64) -> (Vec<Placed>, std::collections::HashSet<(i32, i32)>) {
    let cells: std::collections::HashSet<_> = (-9..=3).flat_map(|x| (-7..=-3).map(move |z| (x,z)))
        .chain((2..=5).flat_map(|x| (-9..=-3).map(move |z| (x,z)))).collect();
    let bridge_x = -6 + (seed as u64 % 3) as i32;
    let crossing = |x: i32, z: i32| (bridge_x..=bridge_x+1).contains(&x) && (-8..=-2).contains(&z);
    let mut ordered: Vec<_> = cells.iter().copied().collect(); ordered.sort_unstable();
    let mut out = Vec::new();
    for &(x,z) in &ordered {
        for y in -2..=0 { out.push((site.at(x,y,z), "minecraft:air")); }
        out.push((site.at(x,-3,z), "jak:barrio_zanja"));
        out.push((site.at(x,-4,z), "minecraft:dirt"));
    }
    for &(x,z) in &ordered {
        for (dx,dz,way) in [(0,-1,2u8),(0,1,0),(-1,0,1),(1,0,3)] {
            let (nx,nz)=(x+dx,z+dz);
            if cells.contains(&(nx,nz)) { continue; }
            for y in -2..=-1 { out.push((site.at(nx,y,nz), "jak:barrio_zanja")); }
            if !crossing(nx,nz) && (nx,nz)!=(-10,-7) {
                out.push((site.at(nx,0,nz), site.facing("jak:barrio_zanja_borde[facing={f}]",way)));
            }
        }
    }
    for x in bridge_x..=bridge_x+1 { for z in -8..=-2 {
        out.push((site.at(x,0,z), "minecraft:air"));
        out.push((site.at(x,-1,z), "jak:barrio_tablon_slab[type=top,waterlogged=false]"));
    }}
    // Keep the original ladder, and masonry stairs on both sides of the low bridge for
    // ordinary walking bodies. Four half-height treads reach the street without
    // a jump; the bridge and the bell remain in their original positions.
    for y in -2..=-1 {
        out.push((site.at(-9,y,-7), site.facing("minecraft:ladder[facing={f},waterlogged=false]",1)));
    }
    for x in [-9,-8,-7,-1,0,1] {for (z,full,half) in [(-6,0,true),(-5,1,false),(-4,1,true),(-3,2,false)] {
        for y in -2..=0 {out.push((site.at(x,y,z),"minecraft:air"));}
        for n in 0..full {out.push((site.at(x,-2+n,z),"jak:barrio_losa"));}
        if half {out.push((site.at(x,-2+full,z),"jak:barrio_losa_capa[layers=4]"));}
    }}
    for x in [-9,-8,-7,-1,0,1] {
        out.push((site.at(x,0,-2),"minecraft:air"));
        // The original rim projects one block inward and below its cell.
        // Behind the bottom tread that becomes a low ceiling for a large body.
        out.push((site.at(x,0,-8),"jak:barrio_losa_capa[layers=1]"));
    }
    (out,cells)
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Flat(i32);
    impl Land for Flat {
        fn ground(&self, _: i32, _: i32) -> i32 {
            self.0
        }
        fn wet(&self, _: i32, _: i32) -> bool {
            false
        }
    }

    /// Every part's model is valid, every state of a village parses, its
    /// buildings are whole and joined.
    #[test]
    fn the_slums_are_whole() {
        for block in &BARRIO_BLOCKS {
            match block.look {
                BlockLook::Set { models, .. } => {
                    for (part, m) in models {
                        serde_json::from_str::<serde_json::Value>(&format!("[{}]", m())).unwrap_or_else(|e| panic!("{}{part}: {e}", block.path));
                    }
                }
                BlockLook::Shaped { model: m, .. } => {
                    serde_json::from_str::<serde_json::Value>(&format!("[{}]", m())).unwrap_or_else(|e| panic!("{}: {e}", block.path));
                }
                _ => {}
            }
        }
        let blocks = village(3, (0, 0), 70, 1, &Flat(70));
        // Every made-up building whole.
        for h in 0..200u64 {
            let spec = random_spec(h.wrapping_mul(0x9E37_79B9_7F4A_7C15), 10, 7);
            let built = building((0, 70, 0), (h % 4) as u8, spec);
            assert!(built.iter().any(|(_, s)| s.starts_with("jak:barrio_corona_pared")), "{spec:?}");
        }
        let Ok(paths) = minecraftoss_core::registries::DataPaths::discover() else { return };
        if !paths.block_catalog.is_file() {
            return;
        }
        let registries = minecraftoss_core::Registries::load(&paths).unwrap();
        let last_old=registries.blocks.parse_state("jak:hv_t-palshaft-roof-01").unwrap();
        for facing in ["north","east","south","west"] {for amount in 1..=4 {
            let state=registries.blocks.parse_state(&format!("jak:barrio_hierbajos[facing={facing},segment_amount={amount}]")).unwrap();
            assert!(state.0>last_old.0,"grass appends after every existing saved state");
            assert!(registries.blocks.collision_boxes(state).is_empty(),"all sixteen states are walk-through");
        }}
        for (_, state) in &blocks {
            assert!(registries.blocks.parse_state(state).is_ok(), "{state}");
        }
        let count = |id: &str| blocks.iter().filter(|(_, s)| s.starts_with(id)).count();
        assert!(count("jak:barrio_hierbajos")>20,"the original slum tufts must decorate the town");
        let final_blocks:std::collections::HashMap<_,_>=blocks.iter().copied().collect();
        for (&(x,y,z),&state) in &final_blocks{if state.starts_with("jak:barrio_hierbajos"){
            let id=registries.blocks.parse_state(state).unwrap();
            assert!(registries.blocks.collision_boxes(id).is_empty(),"litter must not obstruct people or traffic");
            assert!(final_blocks.get(&(x,y-1,z)).is_some_and(|s|s.starts_with("jak:barrio_losa")),"litter needs the actual paving below: {x} {y} {z}");
            assert!(final_blocks.iter().all(|(&(dx,dy,dz),s)|!s.starts_with("jak:haven_puerta")||dy!=y||(dx-x).abs()>1||(dz-z).abs()>1),"tufts leave door thresholds clear");
        }}
        for id in ["jak:barrio_base1_pared", "jak:barrio_base3_pared", "jak:barrio_ocre_pared", "jak:barrio_azul_pared", "jak:barrio_tablas_pared", "jak:barrio_corona_pared", "jak:barrio_base_vano", "jak:barrio_portal", "jak:barrio_desague", "jak:barrio_toldo", "jak:barrio_andamio", "jak:barrio_zanja", "jak:barrio_zanja_borde", "minecraft:ladder", "minecraft:chest"] {
            assert!(count(id) > 0, "{id}");
        }
        // Each building's walls joined: straight runs without a pillar, the
        // corners with one.
        let walls: Vec<&&str> = blocks.iter().map(|(_, s)| s).filter(|s| s.contains("_pared[")).collect();
        assert!(walls.iter().any(|s| s.contains("up=false")) && walls.iter().any(|s| s.contains("up=true")));
    }

    #[test]
    fn the_original_tufts_generate_only_on_haven_ground_and_leave_the_bridge_clear() {
        struct Edge;
        impl Land for Edge {
            fn ground(&self,_x:i32,_z:i32)->i32 {70}
            fn wet(&self,_x:i32,_z:i32)->bool {false}
            fn city_allowed(&self,x:i32,z:i32,spargus:bool)->bool {!spargus&&x<35&&z>-35}
        }
        for turn in 0..4 {
            let blocks=village(3,(0,0),70,turn,&Edge);
            let final_blocks:std::collections::HashMap<_,_>=blocks.into_iter().collect();
            let patches:Vec<_>=final_blocks.iter().filter(|(_,s)|s.starts_with("jak:barrio_hierbajos")).collect();
            assert!(!patches.is_empty());
            for (&(x,y,z),_) in patches {
                assert!(Edge.city_allowed(x,z,false));
                assert!(!final_blocks.get(&(x,y-1,z)).is_some_and(|s|s.starts_with("jak:barrio_tablon_slab")),"bridge passage stays clear");
            }
            assert!(!final_blocks.values().any(|s|s.starts_with("minecraft:leaf_litter")||*s=="jak:dry_grass"),"Haven uses its original plant, not a vanilla or desert stand-in");
        }
    }
}

#[cfg(test)]
mod plaza_regressions {
    use super::*;
    #[test]
    fn the_broad_canal_has_a_flush_crossing_and_preserves_the_plaza() {
        for turn in 0..4 { for seed in 0..6 {
            let site=Site{origin:(20,70,-30),front:turn,w:0,d:0};
            let (placed,cells)=plaza_ditch(&site,seed);
            let final_blocks:std::collections::HashMap<_,_>=placed.into_iter().collect();
            assert!(cells.len()>=80);
            assert!(!cells.contains(&(0,0)) && !cells.contains(&(5,5)));
            assert!(!final_blocks.contains_key(&site.at(0,-3,0)),"town marker preserved");
            let bridge_x=-6+(seed as u64%3) as i32;
            for x in bridge_x..=bridge_x+1 { for z in -8..=-2 {
                assert_eq!(final_blocks[&site.at(x,-1,z)],"jak:barrio_tablon_slab[type=top,waterlogged=false]");
                assert_eq!(final_blocks[&site.at(x,0,z)],"minecraft:air");
                assert!(!cells.contains(&(x,-8)) && !cells.contains(&(x,-2)),"ends reach both banks");
            }}
            for &(x,z) in &cells {
                assert_eq!(final_blocks[&site.at(x,-3,z)],"jak:barrio_zanja");
                if !(bridge_x..=bridge_x+1).contains(&x) && (x,z)!=(-9,-7) && !([-9,-8,-7,-1,0,1].contains(&x)&&(-4..=-3).contains(&z)) {
                    assert_eq!(final_blocks[&site.at(x,-1,z)],"minecraft:air");
                }
            }
            assert!(final_blocks[&site.at(-9,-1,-7)].starts_with("minecraft:ladder["));
            assert!(!final_blocks.contains_key(&site.at(-10,0,-7)),"the exit must clear the overhanging rim");
        }}
    }
    #[test]
    fn bridge_boards_keep_vanilla_slab_states_and_continuous_jak_texture() {
        let block=BARRIO_BLOCKS.iter().find(|b|b.path=="barrio_tablon_slab").unwrap();
        assert!(matches!(block.look,BlockLook::Slab{texture:"w2_city-slum-wood-plain",double:"barrio_tablon_slab"}));
        let Ok(paths)=minecraftoss_core::registries::DataPaths::discover() else{return;};
        if !paths.block_catalog.is_file(){return;}
        let r=minecraftoss_core::Registries::load(&paths).unwrap();
        for (kind,lo,hi) in [("bottom",0.0,0.5),("top",0.5,1.0),("double",0.0,1.0)] {
            let state=r.blocks.parse_state(&format!("jak:barrio_tablon_slab[type={kind},waterlogged=false]")).unwrap();
            assert_eq!(r.blocks.collision_boxes(state),vec![[0.0,lo,0.0,1.0,hi,1.0]]);
            let old=r.blocks.parse_state("jak:objeto_pantano_suelo_3[type=bottom,waterlogged=false]").unwrap();
            assert!(state.0>old.0,"new boards must append after existing saved ids");
        }
    }

    #[test]
    fn the_players_second_board_joins_the_first_into_a_double_slab() {
        use crate::scene::{Block,HandcraftedScene};
        use minecraftoss_player::{Player,World,Face};
        let mut world=HandcraftedScene::default();
        world.set((0,1,0),Some(Block::new("jak:barrio_tablon_slab").with("type","bottom").with("waterlogged","false")));
        let mut player=Player::new(glam::DVec3::new(0.5,1.0,2.5));
        player.pitch=(1.12_f64/2.0).atan().to_degrees();
        let hit=player.target(&world,5.0).expect("the board is targetable");
        assert_eq!(hit.face,Face::Up);
        assert_eq!(player.place_target_with(&mut world,minecraftoss_player::Block::new("jak:barrio_tablon_slab")),Some((0,1,0)));
        let joined=World::block(&world,(0,1,0)).unwrap();
        assert_eq!(joined.id,"jak:barrio_tablon_slab");
        assert_eq!(joined.property("type"),Some("double"));
        assert_eq!(joined.property("waterlogged"),Some("false"));
    }

}

#[cfg(test)]mod canal_stair_tests {
    use super::*;
    #[test]fn canal_stairs_reach_both_sides_of_the_bridge_in_every_rotation() {
        for turn in 0..4 {for seed in 0..6 {
            let site=Site{origin:(20,70,-30),front:turn,w:0,d:0};let (blocks,_)=plaza_ditch(&site,seed);let map:std::collections::HashMap<_,_>=blocks.into_iter().collect();
            for x in [-9,-8,-7,-1,0,1] {for (z,full,half) in [(-6,0,true),(-5,1,false),(-4,1,true),(-3,2,false)] {
                assert_eq!(map[&site.at(x,-3,z)],"jak:barrio_zanja");
                for n in 0..full {assert_eq!(map[&site.at(x,-2+n,z)],"jak:barrio_losa");}
                if half {assert_eq!(map[&site.at(x,-2+full,z)],"jak:barrio_losa_capa[layers=4]");}
                assert_eq!(map[&site.at(x,0,z)],"minecraft:air");
            }assert_eq!(map[&site.at(x,0,-2)],"minecraft:air","no rim across the exit");assert_eq!(map[&site.at(x,0,-8)],"jak:barrio_losa_capa[layers=1]","no inward overhang behind the bottom tread");}
        }}
    }
}
