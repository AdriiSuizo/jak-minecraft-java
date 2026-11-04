//! Villa Refugio: Haven's slums as Adrii's picture has them (24/10:
//! "prácticamente 1:1 a lo que te he dado de inspiración... con modelos de
//! minecraft personalizados, no te limites por las formas de minecraft").
//!
//! The picture's houses: walls of pale rough masonry on a plinth of darker
//! stones, quoins at their ground floors' corners and timber posts at
//! their upper ones; a heavy wooden beam round each storey, its ends out
//! where it turns, posts and corbels under it; small windows set deep in
//! wooden frames, barred or latticed; steep roofs of red round tiles over
//! deep eaves on rafters, a wooden lantern on the main house's ridge,
//! stone chimneys topped with iron pipes, smoking; big iron pipes with
//! collars down the fronts, turning into the walls; red banners hanging
//! from the beams; plank doors under small plank roofs on brackets, iron
//! arms with lanterns; balconies on the taller houses, a market stall
//! under red cloth, crates and barrels. Its streets: flagstones, sunken
//! beds of red earth inside low stone curbs, tufts of grass, stone posts
//! and pillars of stones with lanterns.
//!
//! The houses are laid of grid pieces (`jak_families`' joining families
//! and pieces turned to face out), each reusable by hand.
use crate::jak_facades::{b, model, ring_cells, B};
use crate::jak_families::{gable_roof, Cell, Plan, Site};
use crate::jak_items::{BlockLook, JakBlock, JakItem, JakKind};
use crate::jak_structures::Placed;

// ------------------------------------------------------------- models

/// Quoins: stones standing out at a pillar's four upright edges,
/// alternately long and short up it.
fn quoins() -> Vec<B> {
    let mut boxes = Vec::new();
    for (left, front) in [(true, true), (false, true), (true, false), (false, false)] {
        // Long along x below, along z above.
        let span = |near: bool, long: bool| -> (f32, f32) {
            let reach = if long { 5.5 } else { 3.2 };
            if near { (-0.8, reach) } else { (16.0 - reach, 16.8) }
        };
        let (x0, x1) = span(left, true);
        let (z0, z1) = span(front, false);
        boxes.push(b([x0, 0.0, z0], [x1, 7.5, z1], "q"));
        let (x0, x1) = span(left, false);
        let (z0, z1) = span(front, true);
        boxes.push(b([x0, 8.0, z0], [x1, 15.5, z1], "q"));
    }
    boxes
}

fn masonry_post() -> String {
    let mut boxes = vec![b([-0.05, 0.0, -0.05], [16.05, 16.05, 16.05], "s")];
    boxes.extend(quoins());
    model(&boxes)
}
fn masonry_arm() -> String {
    model(&[b([0.0, 0.0, 0.0], [16.0, 16.0, 8.0], "s")])
}
fn masonry_base_post() -> String {
    let mut boxes = vec![b([-0.05, 0.0, -0.05], [16.05, 16.05, 16.05], "s"), b([-1.8, 0.0, -1.8], [17.8, 5.0, 17.8], "q")];
    boxes.extend(quoins().into_iter().filter(|x| x.from[1] > 1.0));
    model(&boxes)
}
fn masonry_base_arm() -> String {
    model(&[
        b([0.0, 0.0, 0.0], [16.0, 16.0, 8.0], "s"),
        b([-1.8, 0.0, 0.0], [17.8, 5.0, 8.0], "q"),
        b([-0.8, 5.0, 0.0], [16.8, 6.0, 8.0], "q"),
    ])
}
/// The upper storey: masonry, a timber post up each edge where it turns.
fn timbered_post() -> String {
    model(&[
        b([-0.05, 0.0, -0.05], [16.05, 16.05, 16.05], "s"),
        b([-1.2, 0.0, -1.2], [3.0, 16.0, 3.0], "w"),
        b([13.0, 0.0, -1.2], [17.2, 16.0, 3.0], "w"),
        b([-1.2, 0.0, 13.0], [3.0, 16.0, 17.2], "w"),
        b([13.0, 0.0, 13.0], [17.2, 16.0, 17.2], "w"),
    ])
}
/// A storey's beam course: masonry under a heavy beam standing out of
/// both faces; where it turns or ends the beams cross, their ends out, a
/// corbel under each.
fn beam_post() -> String {
    model(&[
        b([-0.05, 0.0, -0.05], [16.05, 9.9, 16.05], "s"),
        b([-4.0, 9.9, 3.0], [20.0, 16.1, 13.0], "w"),
        b([3.0, 9.9, -4.0], [13.0, 16.1, 20.0], "w"),
        b([-2.5, 6.0, 5.0], [0.0, 9.9, 11.0], "w"),
        b([16.0, 6.0, 5.0], [18.5, 9.9, 11.0], "w"),
        b([5.0, 6.0, -2.5], [11.0, 9.9, 0.0], "w"),
        b([5.0, 6.0, 16.0], [11.0, 9.9, 18.5], "w"),
    ])
}
fn beam_arm() -> String {
    model(&[b([0.0, 0.0, 0.0], [16.0, 10.0, 8.0], "s"), b([-2.0, 10.0, 0.0], [18.0, 16.0, 8.0], "w")])
}

/// A window's half: set deep in the wall, its wooden frame at the wall's
/// middle, a lintel board out over it, a stone sill, a bar up its half.
fn barred_half(alt: bool) -> String {
    let z = |a: f32, c: f32| if alt { (16.0 - c, 16.0 - a) } else { (a, c) };
    let bx = |x0: f32, y0: f32, za: f32, x1: f32, y1: f32, zc: f32, t: &'static str| {
        let (z0, z1) = z(za, zc);
        b([x0, y0, z0], [x1, y1, z1], t)
    };
    model(&[
        bx(0.0, 0.0, 0.0, 16.0, 4.0, 8.0, "s"),
        bx(0.0, 12.0, 0.0, 16.0, 16.0, 8.0, "s"),
        bx(0.0, 4.0, 0.0, 16.0, 12.0, 3.0, "s"),
        bx(-1.5, 12.0, 0.0, 17.5, 13.6, 8.0, "w"),
        bx(-1.0, 3.0, 0.0, 17.0, 4.0, 8.0, "q"),
        bx(5.5, 4.0, 3.0, 10.5, 12.0, 4.4, "w"),
        bx(5.5, 11.0, 3.0, 10.5, 12.0, 8.0, "w"),
        bx(5.5, 4.0, 3.0, 10.5, 5.0, 8.0, "w"),
        bx(7.3, 5.0, 5.4, 8.7, 11.0, 6.4, "w"),
    ])
}
fn barred_half_north() -> String {
    barred_half(false)
}
fn barred_half_south() -> String {
    barred_half(true)
}
fn barred_middle() -> String {
    model(&[b([7.3, 5.0, 7.4], [8.7, 11.0, 8.6], "w"), b([7.3, 7.6, 3.0], [8.7, 8.4, 13.0], "w")])
}
/// A latticed window's half: crossed slats in its frame.
fn lattice_half(alt: bool) -> String {
    let base = barred_half(alt);
    let (z0, z1) = if alt { (8.0, 13.0) } else { (3.0, 8.0) };
    let mut boxes = Vec::new();
    for (angle, y) in [(45.0f32, 6.5f32), (-45.0, 6.5), (45.0, 9.5), (-45.0, 9.5)] {
        boxes.push(B { from: [7.4, y - 0.5, z0], to: [8.6, y + 0.5, z1], tex: "w", turn: Some(('x', angle, [8.0, y, (z0 + z1) / 2.0])), ends: None, bottom:None });
    }
    let extra = model(&boxes);
    format!("{base},{extra}")
}
fn lattice_half_north() -> String {
    lattice_half(false)
}
fn lattice_half_south() -> String {
    lattice_half(true)
}
fn nothing() -> String {
    String::new()
}

/// A railing's post and rails (a balcony's).
fn rail_post() -> String {
    model(&[b([6.0, 0.0, 6.0], [10.0, 14.0, 10.0], "w"), b([5.5, 13.0, 5.5], [10.5, 14.5, 10.5], "w")])
}
fn rail_arm() -> String {
    model(&[
        b([6.8, 11.5, 0.0], [9.2, 13.5, 8.0], "w"),
        b([7.0, 2.5, 0.0], [9.0, 4.0, 8.0], "w"),
        b([7.3, 4.0, 2.5], [8.7, 11.5, 4.0], "w"),
    ])
}
/// A curb: a low rim of worn stones round a bed of earth.
fn curb_post() -> String {
    model(&[b([2.5, 0.0, 2.5], [13.5, 5.5, 13.5], "q"), b([3.5, 5.5, 3.5], [12.5, 6.2, 12.5], "q")])
}
fn curb_arm() -> String {
    model(&[b([3.0, 0.0, 0.0], [13.0, 5.0, 8.0], "q"), b([3.8, 5.0, 0.5], [12.2, 5.6, 7.5], "q")])
}

// Pieces turned to face out (a carved pumpkin's ways; front is -z, the
// wall they stand against +z).

/// Masonry with a timber post up its face, a corbel at its top.
fn facade_post() -> String {
    model(&[
        b([0.0, 0.0, 0.0], [16.0, 16.0, 16.0], "s"),
        b([6.0, 0.0, -2.0], [10.0, 16.0, 0.0], "w"),
        b([5.0, 12.0, -3.5], [11.0, 16.0, 0.0], "w"),
    ])
}
/// An eave's rafters: two beams' ends out of the wall under the roof's
/// edge, a brace under each, the fascia board along the edge.
fn rafters() -> String {
    model(&[
        b([2.0, 8.0, 0.0], [5.0, 12.0, 16.0], "w"),
        b([11.0, 8.0, 0.0], [14.0, 12.0, 16.0], "w"),
        b([2.5, 2.0, 12.0], [4.5, 8.0, 16.0], "w"),
        b([11.5, 2.0, 12.0], [13.5, 8.0, 16.0], "w"),
        b([2.5, 5.0, 8.0], [4.5, 8.0, 12.0], "w"),
        b([11.5, 5.0, 8.0], [13.5, 8.0, 12.0], "w"),
        b([-0.2, 12.0, -1.0], [16.2, 16.0, 1.5], "w"),
    ])
}
/// A small roof of planks over a door, three blocks across, falling away
/// from the wall in steps, on two brackets.
fn door_roof() -> String {
    let mut boxes = Vec::new();
    for (i, (z0, z1, y)) in [(12.0f32, 16.0f32, 8.0f32), (8.0, 12.0, 6.6), (4.0, 8.0, 5.2), (0.0, 4.0, 3.8), (-3.0, 0.0, 2.4)].into_iter().enumerate() {
        boxes.push(b([-14.0, y, z0], [30.0, y + 1.6, z1 + 0.6], "p"));
        if i % 2 == 0 {
            boxes.push(b([-14.0, y + 1.6, z0 + 1.0], [30.0, y + 2.0, z0 + 2.0], "w"));
        }
    }
    for x in [-12.0f32, 26.0] {
        boxes.push(b([x, -8.0, 14.0], [x + 2.0, 8.0, 16.0], "w"));
        boxes.push(b([x, 0.0, 10.0], [x + 2.0, 3.0, 14.0], "w"));
        boxes.push(b([x, 3.0, 5.0], [x + 2.0, 5.0, 10.0], "w"));
    }
    model(&boxes)
}
/// A red banner hanging two blocks from a rod against the wall, its foot
/// cut in two tails.
fn banner() -> String {
    model(&[
        b([1.0, 14.6, 12.5], [15.0, 16.0, 16.0], "w"),
        b([2.5, -12.0, 14.4], [13.5, 14.6, 15.2], "c"),
        b([2.5, -16.0, 14.4], [6.5, -12.0, 15.2], "c"),
        b([9.5, -16.0, 14.4], [13.5, -12.0, 15.2], "c"),
    ])
}
/// An iron arm out of the wall a lantern hangs from (the lantern itself
/// the block under it).
fn lantern_arm() -> String {
    model(&[
        b([7.0, 12.0, 8.0], [9.0, 14.0, 16.0], "m"),
        b([7.0, 8.0, 14.0], [9.0, 12.0, 16.0], "m"),
        b([7.5, 9.0, 11.0], [8.5, 12.0, 12.0], "m"),
        b([7.5, 0.0, 7.5], [8.5, 12.0, 8.5], "m"),
    ])
}
/// The wooden lantern on the main house's ridge: eight posts in a ring,
/// cloth between them, a little roof of round tiles, a finial.
fn ridge_lantern() -> String {
    let mut boxes = vec![b([-5.0, 0.0, -5.0], [21.0, 2.0, 21.0], "w")];
    for k in 0..8 {
        let a = k as f32 * std::f32::consts::TAU / 8.0;
        let (x, z) = (8.0 + 11.5 * a.cos(), 8.0 + 11.5 * a.sin());
        boxes.push(b([x - 1.2, 2.0, z - 1.2], [x + 1.2, 14.0, z + 1.2], "w"));
    }
    boxes.push(b([-3.0, 3.0, -3.0], [19.0, 13.0, 19.0], "c"));
    boxes.push(b([-6.0, 14.0, -6.0], [22.0, 16.0, 22.0], "w"));
    boxes.push(b([-4.0, 16.0, -4.0], [20.0, 19.0, 20.0], "t"));
    boxes.push(b([-1.0, 19.0, -1.0], [17.0, 22.0, 17.0], "t"));
    boxes.push(b([3.0, 22.0, 3.0], [13.0, 24.5, 13.0], "t"));
    boxes.push(b([7.0, 24.5, 7.0], [9.0, 28.0, 9.0], "w"));
    model(&boxes)
}
/// A chimney's top: its stone sleeve down round the fire under it, a cap,
/// an iron pipe with its ring.
fn chimney_top() -> String {
    model(&[
        b([-0.3, -16.0, -0.3], [16.3, 0.0, 16.3], "q"),
        b([-1.0, 0.0, -1.0], [17.0, 3.0, 17.0], "q"),
        b([4.5, 3.0, 4.5], [11.5, 14.0, 11.5], "m"),
        b([3.8, 11.5, 3.8], [12.2, 13.5, 12.2], "r"),
    ])
}
/// A wooden crate in its frame.
fn crate_box() -> String {
    let mut boxes = vec![b([1.0, 0.0, 1.0], [15.0, 13.0, 15.0], "k")];
    for (x, z) in [(0.5f32, 0.5f32), (13.0, 0.5), (0.5, 13.0), (13.0, 13.0)] {
        boxes.push(b([x, 0.0, z], [x + 2.5, 13.5, z + 2.5], "w"));
    }
    boxes.push(b([0.5, 11.5, 0.5], [15.5, 13.5, 15.5], "w"));
    model(&boxes)
}
/// A market stall's awning of red cloth, three blocks across, falling
/// away from its back in steps, its scalloped edge hanging.
fn cloth_awning() -> String {
    let mut boxes = Vec::new();
    for (z0, z1, y) in [(10.0f32, 16.0f32, 10.0f32), (4.0, 10.0, 8.5), (-2.0, 4.0, 7.0), (-8.0, -2.0, 5.5)] {
        boxes.push(b([-14.0, y, z0], [30.0, y + 1.0, z1], "c"));
    }
    let mut x = -14.0f32;
    while x < 30.0 {
        boxes.push(b([x, 3.5, -8.0], [x + 3.0, 5.5, -7.2], "c"));
        x += 5.5;
    }
    model(&boxes)
}
/// An upright post of brown wood (a stall's, a porch's), along its rod.
fn wood_post() -> String {
    model(&[b([5.5, 0.0, 5.5], [10.5, 16.0, 10.5], "w")])
}

// ------------------------------------------------------------- blocks

type Parts = &'static [(&'static str, fn() -> String)];

const MASONRY: [(&str, fn() -> String); 3] = [("_post", masonry_post), ("_side", masonry_arm), ("_side_tall", masonry_arm)];
const MASONRY_BASE: [(&str, fn() -> String); 3] = [("_post", masonry_base_post), ("_side", masonry_base_arm), ("_side_tall", masonry_base_arm)];
const TIMBERED: [(&str, fn() -> String); 3] = [("_post", timbered_post), ("_side", masonry_arm), ("_side_tall", masonry_arm)];
const BEAM: [(&str, fn() -> String); 3] = [("_post", beam_post), ("_side", beam_arm), ("_side_tall", beam_arm)];
const BARRED: [(&str, fn() -> String); 5] = [("_post", barred_middle), ("_side", barred_half_north), ("_side_alt", barred_half_south), ("_noside", nothing), ("_noside_alt", nothing)];
const LATTICE: [(&str, fn() -> String); 5] = [("_post", barred_middle), ("_side", lattice_half_north), ("_side_alt", lattice_half_south), ("_noside", nothing), ("_noside_alt", nothing)];
const RAIL: [(&str, fn() -> String); 3] = [("_post", rail_post), ("_side", rail_arm), ("_side_tall", rail_arm)];
const CURB: [(&str, fn() -> String); 3] = [("_post", curb_post), ("_side", curb_arm), ("_side_tall", curb_arm)];

/// The picture's stuff: pale rough masonry, darker stones, warm brown
/// wood, iron, red cloth, red round tiles.
const STONE: &[(&str, &str)] = &[("s", "wascity-stone-bricks-2-plain"), ("q", "city-farm-stonewall-bricks"), ("w", "wascity-wood-plain")];

const fn set(path: &'static str, vanilla: &'static str, models: Parts, textures: &'static [(&'static str, &'static str)]) -> JakBlock {
    JakBlock { path, look: BlockLook::Set { vanilla, models, textures } }
}
const fn shaped(path: &'static str, vanilla: &'static str, model: fn() -> String, textures: &'static [(&'static str, &'static str)]) -> JakBlock {
    JakBlock { path, look: BlockLook::Shaped { vanilla, model, textures } }
}

pub const HAVEN_BLOCKS: [JakBlock; 25] = [
    set("haven_mamposteria_pared", "cobblestone_wall", &MASONRY, STONE),
    set("haven_mamposteria_pared_base", "cobblestone_wall", &MASONRY_BASE, STONE),
    set("haven_entramado_pared", "cobblestone_wall", &TIMBERED, STONE),
    set("haven_viga_pared", "cobblestone_wall", &BEAM, STONE),
    set("haven_barrotes_vano", "glass_pane", &BARRED, STONE),
    set("haven_celosia_vano", "glass_pane", &LATTICE, STONE),
    set("haven_baranda", "cobblestone_wall", &RAIL, STONE),
    set("haven_bordillo", "cobblestone_wall", &CURB, STONE),
    shaped("haven_poste_fachada", "carved_pumpkin", facade_post, STONE),
    shaped("haven_canes", "carved_pumpkin", rafters, STONE),
    shaped("haven_tejadillo", "carved_pumpkin", door_roof, &[("p", "hip-twood01"), ("w", "wascity-wood-plain")]),
    shaped("haven_estandarte", "carved_pumpkin", banner, &[("c", "fort-tank-beige-logo"), ("w", "city-dirtywood")]),
    shaped("haven_brazo_farol", "carved_pumpkin", lantern_arm, &[("m", "city-bigpipe-ring-02")]),
    shaped("haven_cupula_madera", "carved_pumpkin", ridge_lantern, &[("w", "wascity-wood-plain"), ("c", "wascitya-flag-a"), ("t", "vil-hut-roof-tile-01")]),
    shaped("haven_chimenea_remate", "carved_pumpkin", chimney_top, &[("q", "city-farm-stonewall-bricks"), ("m", "city-bigpipe-main-02"), ("r", "city-bigpipe-ring-02")]),
    shaped("haven_caja", "carved_pumpkin", crate_box, &[("k", "hip-twood01"), ("w", "wascity-wood-plain")]),
    shaped("haven_toldo_tela", "carved_pumpkin", cloth_awning, &[("c", "wascitya-flag-a")]),
    shaped("haven_puntal", "lightning_rod", wood_post, &[("w", "wascity-wood-plain")]),
    JakBlock { path: "haven_sillar", look: BlockLook::All("city-farm-stonewall-bricks") },
    JakBlock { path: "haven_mamposteria", look: BlockLook::All("wascity-stone-bricks-2-plain") },
    JakBlock { path: "haven_tierra", look: BlockLook::All("slum-stone-03") },
    JakBlock { path: "haven_tierra_capa", look: BlockLook::Layers { texture: "slum-stone-03", full: "haven_tierra" } },
    JakBlock { path: "haven_mata", look: BlockLook::Cross("swp-grassclump-02") },
    JakBlock { path: "haven_suelo_madera", look: BlockLook::All("hip-twood01") },
    JakBlock { path: "haven_losa_madera", look: BlockLook::Slab { texture: "hip-twood01", double: "haven_suelo_madera" } },
];

pub const HAVEN_ITEMS: [JakItem; 25] = [
    JakItem { path: "haven_mamposteria_pared", title: "Muro de mampostería de Villa Refugio", kind: JakKind::Block },
    JakItem { path: "haven_mamposteria_pared_base", title: "Muro de mampostería con zócalo", kind: JakKind::Block },
    JakItem { path: "haven_entramado_pared", title: "Muro con postes de Villa Refugio", kind: JakKind::Block },
    JakItem { path: "haven_viga_pared", title: "Muro con viga de piso", kind: JakKind::Block },
    JakItem { path: "haven_barrotes_vano", title: "Ventana con barrotes", kind: JakKind::Block },
    JakItem { path: "haven_celosia_vano", title: "Ventana con celosía", kind: JakKind::Block },
    JakItem { path: "haven_baranda", title: "Baranda de madera", kind: JakKind::Block },
    JakItem { path: "haven_bordillo", title: "Bordillo de piedra", kind: JakKind::Block },
    JakItem { path: "haven_poste_fachada", title: "Poste de fachada con ménsula", kind: JakKind::Block },
    JakItem { path: "haven_canes", title: "Canes de alero", kind: JakKind::Block },
    JakItem { path: "haven_tejadillo", title: "Tejadillo de puerta", kind: JakKind::Block },
    JakItem { path: "haven_estandarte", title: "Estandarte rojo", kind: JakKind::Block },
    JakItem { path: "haven_brazo_farol", title: "Brazo de farol", kind: JakKind::Block },
    JakItem { path: "haven_cupula_madera", title: "Linterna de tejado", kind: JakKind::Block },
    JakItem { path: "haven_chimenea_remate", title: "Remate de chimenea", kind: JakKind::Block },
    JakItem { path: "haven_caja", title: "Caja de madera", kind: JakKind::Block },
    JakItem { path: "haven_toldo_tela", title: "Toldo de tela", kind: JakKind::Block },
    JakItem { path: "haven_puntal", title: "Puntal de madera", kind: JakKind::Block },
    JakItem { path: "haven_sillar", title: "Sillar", kind: JakKind::Block },
    JakItem { path: "haven_mamposteria", title: "Mampostería", kind: JakKind::Block },
    JakItem { path: "haven_tierra", title: "Tierra roja", kind: JakKind::Block },
    JakItem { path: "haven_tierra_capa", title: "Capa de tierra roja", kind: JakKind::Block },
    JakItem { path: "haven_mata", title: "Mata de hierba", kind: JakKind::Block },
    JakItem { path: "haven_suelo_madera", title: "Tablas de Villa Refugio", kind: JakKind::Block },
    JakItem { path: "haven_losa_madera", title: "Losa de tablas de Villa Refugio", kind: JakKind::Block },
];

// ------------------------------------------------------------- houses

const BASE: &str = "jak:haven_mamposteria_pared_base";
const WALL: &str = "jak:haven_mamposteria_pared";
const UPPER: &str = "jak:haven_entramado_pared";
const BEAMS: &str = "jak:haven_viga_pared";
const BARS: &str = "jak:haven_barrotes_vano";
const LATTICED: &str = "jak:haven_celosia_vano";

/// One storey's ring of walls, `rows` high from `y0`, from local `z0` back
/// over a `w` by `d` square: its foot on the plinth (the ground floor's),
/// its windows at row `window` where `windows` says, its top the storey's
/// beam.
#[allow(clippy::too_many_arguments)]
fn storey(p: &mut Plan, s: &Site, z0: i32, y0: i32, rows: i32, body: &'static str, window: i32, windows: &dyn Fn(i32, i32, u8) -> Option<&'static str>) {
    for (x, z, way, corner) in ring_cells(0, z0, s.w, s.d - z0) {
        for r in 0..rows {
            let cell = if r == rows - 1 {
                BEAMS
            } else if r == 0 && y0 == 0 {
                BASE
            } else if r == window && !corner {
                windows(x, z, way).unwrap_or(body)
            } else {
                body
            };
            p.put(s.at(x, y0 + r, z), Cell::Join(cell));
        }
    }
}

/// A floor of planks inside a ring at `y`.
fn floor(p: &mut Plan, s: &Site, y: i32, z0: i32) {
    for x in 1..s.w - 1 {
        for z in z0 + 1..s.d - 1 {
            p.fixed(s.at(x, y, z), "jak:haven_suelo_madera");
        }
    }
}

/// Rafters under an eave along the front (local z `-1`) and the back (`d`)
/// at `y`, every other block.
fn rafters_along(p: &mut Plan, s: &Site, y: i32, front_z: i32) {
    for x in 0..s.w {
        if x % 2 == 0 {
            p.fixed(s.at(x, y, front_z), s.facing("jak:haven_canes[facing={f}]", 0));
            p.fixed(s.at(x, y, s.d), s.facing("jak:haven_canes[facing={f}]", 2));
        }
    }
}

/// A stone chimney up from `y0` at local `(x, z)` to `top`, smoking out of
/// its iron pipe.
fn chimney(p: &mut Plan, s: &Site, (x, z): (i32, i32), y0: i32, top: i32) {
    for y in y0..top {
        p.fixed(s.at(x, y, z), "jak:haven_sillar");
    }
    p.fixed(s.at(x, top, z), "minecraft:campfire[facing=north,lit=true,signal_fire=false,waterlogged=false]");
    p.fixed(s.at(x, top + 1, z), s.facing("jak:haven_chimenea_remate[facing={f}]", 0));
}

/// A big iron pipe down from `top` to the ground at local `(x, z)`, turning
/// into the wall behind it (local way `into`) at its top.
fn pipe(p: &mut Plan, s: &Site, (x, z): (i32, i32), top: i32, into: u8) {
    for y in 0..top {
        p.fixed(s.at(x, y, z), "jak:haven_tubo[facing=up,powered=false,waterlogged=false]");
    }
    p.fixed(s.at(x, top, z), s.facing("jak:haven_codo[facing={f}]", (into + 2) % 4));
}

fn door(p: &mut Plan, s: &Site, x: i32, y: i32, z: i32) {
    for (dy, half) in [(0, "lower"), (1, "upper")] {
        p.fixed(s.at(x, y + dy, z), s.facing(&format!("jak:haven_puerta[facing={{f}},half={half},hinge=left,open=false,powered=false]"), 2));
    }
}

fn inside(p: &mut Plan, s: &Site, levels: &[i32]) {
    for x in 1..s.w - 1 {
        for z in 1..s.d - 1 {
            for &y0 in levels {
                for y in [y0, y0 + 1] {
                    p.air_if_empty(s.at(x, y, z));
                }
            }
        }
    }
}

/// A building of Villa Refugio at `origin` (its floor's middle, on the
/// street), its front toward `front`, one of the picture's three:
/// - 0, the main house: two storeys, its door under a little plank roof
///   between a barred window and a wall lantern, a banner on its upper
///   floor between latticed windows, rafters under deep eaves, a steep roof
///   of red round tiles with a wooden lantern on its ridge, a chimney and
///   a big pipe down a corner;
/// - 1, the tall narrow house: three storeys, a small steep roof, its
///   chimney, a pipe down its side, a banner;
/// - 2, the house with a balcony: three storeys, the balcony on its
///   second on rafters with a railing and a door out, a big pipe along its
///   front, a market stall before it under red cloth with crates.
///
/// Inside each a floor a storey, a ladder up, a bed, a chest of Jak 2's
/// things and lanterns.
pub fn building(origin: (i32, i32, i32), front: u8, kind: u8) -> Vec<Placed> {
    building_roofed(origin, front, kind, ("jak:teja_roja", "jak:cumbrera_roja"))
}

/// [`building`] under `roof`'s tiles and ridge (the land's: Sandover's red,
/// Snowy Mountain's snowy, Boggy Swamp's mossy).
pub fn building_roofed(origin: (i32, i32, i32), front: u8, kind: u8, (tiles, ridge_tiles): (&'static str, &'static str)) -> Vec<Placed> {
    let (w, d, storeys) = match kind {
        0 => (11, 7, 2),
        1 => (5, 5, 3),
        _ => (7, 5, 3),
    };
    let s = Site { origin, front, w, d };
    let mut p = Plan::default();
    let height = storeys * 3;
    for x in -3..w + 3 {
        for z in -5..d + 3 {
            for y in 0..height + 10 {
                p.fixed(s.at(x, y, z), "minecraft:air");
            }
            p.fixed(s.at(x, -1, z), "jak:haven_adoquin");
        }
    }
    // The storeys: windows barred below, latticed and barred above.
    for k in 0..storeys {
        let y0 = k * 3;
        let upper = k > 0;
        let windows = move |x: i32, z: i32, way: u8| -> Option<&'static str> {
            let along = if way % 2 == 0 { x } else { z };
            match (way, upper) {
                (2, _) => (along == 2).then_some(BARS),
                (0, false) => (along % 3 == 1 && along != w / 2).then_some(BARS),
                (0, true) => (along % 2 == 1).then_some(if along == 1 { LATTICED } else { BARS }),
                _ => (along == d / 2).then_some(if upper { LATTICED } else { BARS }),
            }
        };
        storey(&mut p, &s, 0, y0, 3, if upper { UPPER } else { WALL }, 1, &windows);
        floor(&mut p, &s, y0 - 1, 0);
    }
    floor(&mut p, &s, height - 1, 0);
    door(&mut p, &s, w / 2, 0, 0);
    // Timber posts with corbels up the ground floor's front beside the door.
    for x in [w / 2 - 1, w / 2 + 1] {
        if x > 0 && x < w - 1 {
            p.fixed(s.at(x, 1, 0), s.facing("jak:haven_poste_fachada[facing={f}]", 0));
        }
    }
    // The roof: steep, of red round tiles over eaves a block out, rafters
    // under them; its ridge along the front.
    let top = height;
    rafters_along(&mut p, &s, top - 1, -1);
    let back = if (d + 3) % 2 == 1 { d } else { d + 1 };
    let ridge = gable_roof(&mut p, &s, (-2, back), top, tiles, ridge_tiles, UPPER);
    let mid = -2 + (back + 3) / 2;
    chimney(&mut p, &s, (w - 2, d - 1), height - 3, ridge + 1);
    match kind {
        0 => {
            p.fixed(s.at(w / 2, ridge, mid), s.facing("jak:haven_cupula_madera[facing={f}]", 0));
            p.fixed(s.at(w / 2, 2, -1), s.facing("jak:haven_tejadillo[facing={f}]", 0));
            p.fixed(s.at(w / 2 + 2, 2, -1), s.facing("jak:haven_brazo_farol[facing={f}]", 0));
            p.fixed(s.at(w / 2 + 2, 1, -1), "minecraft:lantern[hanging=true,waterlogged=false]");
            p.fixed(s.at(w / 2, 4, -1), s.facing("jak:haven_estandarte[facing={f}]", 0));
            pipe(&mut p, &s, (-1, d - 2), 5, 1);
            pipe(&mut p, &s, (w / 2 + 3, -1), 5, 0);
            // A lower wing on the right, its own roof leaning on the house
            // (the picture's house turns its roofs two ways).
            let wing = Site { origin: s.at(w + 1, 0, 3), front, w: 5, d: 5 };
            storey(&mut p, &wing, 0, 0, 3, WALL, 1, &|x: i32, _: i32, way: u8| (way == 0 && x == 2).then_some(LATTICED));
            floor(&mut p, &wing, -1, 0);
            for x in 1..=wing.w {
                for z in -1..=wing.d {
                    // Rising from its right eave to the house's wall.
                    let y = 2 + (wing.w - x).min(3);
                    p.put(wing.at(x, y, z), Cell::Tiles(tiles, wing.way(3)));
                    // Its gables, front and back, walled up under it.
                    if (z == 0 || z == wing.d - 1) && x < wing.w - 1 {
                        for under in 3..y {
                            p.put(wing.at(x, under, z), Cell::Join(WALL));
                        }
                    }
                }
            }
            for z in 0..wing.d {
                if z % 2 == 0 {
                    p.fixed(wing.at(wing.w, 1, z), wing.facing("jak:haven_canes[facing={f}]", 1));
                }
            }
            // A doorway into it from the house.
            for y in 0..2 {
                p.fixed(s.at(w - 1, y, 3), "minecraft:air");
                p.fixed(wing.at(0, y, 1), "minecraft:air");
            }
            p.fixed(s.at(0, 0, -1), "minecraft:barrel[facing=up,open=false]");
            p.fixed(s.at(1, 0, -1), s.facing("jak:haven_caja[facing={f}]", 0));
            p.fixed(s.at(w - 1, 0, -1), "minecraft:barrel[facing=up,open=false]");
        }
        1 => {
            p.fixed(s.at(w / 2, 5, -1), s.facing("jak:haven_estandarte[facing={f}]", 0));
            p.fixed(s.at(w / 2 + 1, 2, -1), s.facing("jak:haven_brazo_farol[facing={f}]", 0));
            p.fixed(s.at(w / 2 + 1, 1, -1), "minecraft:lantern[hanging=true,waterlogged=false]");
            pipe(&mut p, &s, (-1, 1), height - 2, 1);
            p.fixed(s.at(w - 1, 0, -1), s.facing("jak:haven_caja[facing={f}]", 0));
        }
        _ => {
            // The balcony on the second storey: a floor out over rafters,
            // its railing, a door out to it.
            for x in 1..w - 1 {
                p.fixed(s.at(x, 2, -1), s.facing("jak:haven_canes[facing={f}]", 0));
                p.fixed(s.at(x, 3, -1), "jak:haven_losa_madera[type=bottom,waterlogged=false]");
                p.put(s.at(x, 4, -2), Cell::Join("jak:haven_baranda"));
            }
            for x in [1, w - 2] {
                p.put(s.at(x, 4, -1), Cell::Join("jak:haven_baranda"));
            }
            door(&mut p, &s, w / 2, 3, 0);
            p.fixed(s.at(w / 2 - 2, 6, -1), s.facing("jak:haven_estandarte[facing={f}]", 0));
            // A big pipe along the front under the balcony, down its left.
            for x in 0..w / 2 {
                p.fixed(s.at(x, 2, -1), "jak:haven_tubo[facing=east,powered=false,waterlogged=false]");
            }
            for y in 0..2 {
                p.fixed(s.at(-1, y, -1), "jak:haven_tubo[facing=up,powered=false,waterlogged=false]");
            }
            p.fixed(s.at(-1, 2, -1), s.facing("jak:haven_codo[facing={f}]", 1));
            door(&mut p, &s, w / 2, 0, 0);
            // The market stall before it: posts, its red cloth, crates.
            for x in [w / 2 - 2, w / 2 + 2] {
                for y in 0..2 {
                    p.fixed(s.at(x, y, -4), "jak:haven_puntal[facing=up,powered=false,waterlogged=false]");
                }
            }
            p.fixed(s.at(w / 2, 1, -3), s.facing("jak:haven_toldo_tela[facing={f}]", 0));
            for x in w / 2 - 1..=w / 2 + 1 {
                p.fixed(s.at(x, 0, -4), s.facing("jak:haven_caja[facing={f}]", 0));
            }
            p.fixed(s.at(w / 2 + 3, 0, -3), "minecraft:barrel[facing=up,open=false]");
        }
    }
    // A ladder up inside the front wall through every floor.
    for y in 0..height - 1 {
        p.fixed(s.at(w - 2, y, 1), s.facing("minecraft:ladder[facing={f},waterlogged=false]", 2));
    }
    for (part, z) in [("foot", d - 3), ("head", d - 2)] {
        p.fixed(s.at(1, 0, z), s.facing(&format!("minecraft:red_bed[facing={{f}},occupied=false,part={part}]"), 2));
    }
    p.fixed(s.at(w - 2, 0, d - 2), s.facing("minecraft:chest[facing={f},type=single,waterlogged=false]", 0));
    for k in 0..storeys {
        p.fixed(s.at(w / 2, k * 3 + 1, d / 2), "minecraft:lantern[hanging=true,waterlogged=false]");
    }
    let levels: Vec<i32> = (0..storeys).map(|k| k * 3).collect();
    inside(&mut p, &s, &levels);
    p.doorways("jak:haven_suelo_madera");
    p.finish()
}

// ------------------------------------------------------------- streets

/// A bed of red earth sunk into the flagstones at `(x0, z0)`, `w` by `d`,
/// inside a low curb of stones, tufts of grass in it and at its rim; `y`
/// the street's top (its paving at `y - 1`).
pub fn bed(out: &mut Plan, (x0, z0): (i32, i32), (w, d): (i32, i32), y: i32, salt: u64) {
    for x in x0..x0 + w {
        for z in z0..z0 + d {
            let rim = x == x0 || z == z0 || x == x0 + w - 1 || z == z0 + d - 1;
            if rim {
                out.put((x, y, z), Cell::Join("jak:haven_bordillo"));
                continue;
            }
            // Sunk a quarter under the flagstones, or whole earth with a
            // tuft of grass on it (a tuft on a sunk layer would float).
            let h = (x as u64).wrapping_mul(73856093) ^ (z as u64).wrapping_mul(19349663) ^ salt;
            if h % 3 == 0 {
                out.fixed((x, y - 1, z), "jak:haven_tierra");
                out.fixed((x, y, z), "jak:haven_mata");
            } else {
                out.fixed((x, y - 1, z), "jak:haven_tierra_capa[layers=6]");
                out.fixed((x, y, z), "minecraft:air");
            }
        }
    }
}

/// A pillar of stones two high with a lantern on it, at `(x, z)` on the
/// street's top `y`.
pub fn lantern_pillar(out: &mut Plan, (x, z): (i32, i32), y: i32) {
    out.fixed((x, y, z), "jak:haven_sillar");
    out.fixed((x, y + 1, z), "jak:haven_sillar");
    out.fixed((x, y + 2, z), "minecraft:lantern[hanging=false,waterlogged=false]");
}
