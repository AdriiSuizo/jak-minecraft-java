//! More of Spargus (Adrii 24/10: "con la misma forma de hacer, haz mÃ¡s
//! edificios y variaciÃ³n tambiÃ©n para spargus"), drawn from Jak 3's own
//! city (`wascitya`, rendered from its level):
//! - soft adobe houses of bleached white stucco
//!   (`wascity-stucco-wall-bleached-01`) or of sand
//!   (`wascity-stone-plain-wall-3`), their corners rounded, crenellated
//!   with rounded merlons; mounds of earth heaped against their feet;
//! - small windows in crooked wooden frames; arched doors;
//! - thin metal horns sticking out of their walls, up and out; round
//!   metal pots hung on brackets; tall tapering sandstone columns banded
//!   red and dark (`common_sandstone_*`) before their doors;
//! - grey stone bunkers (`wascity-wall-weathered`) battered, ribbed with
//!   pale buttresses (`wascity-stucco-wall-supports`), a white room on top;
//! - round white towers;
//! - sand (`wascity-ground-01`) with dark flagstones (`wascity-cement-road`)
//!   and orange shrubs.
//!
//! As the slums (`jak_barrio`): grid pieces that join their neighbours,
//! the walls' and ground's textures laid over the world; every building
//! whole and with its inside; towns on `jak_town`'s plan.
use crate::jak_barrio::{turned, BASE1, BASE2, BASE3};
use crate::jak_facades::{b, model, ring_cells};
use crate::jak_families::{Cell, Plan, Site};
use crate::jak_items::{BlockLook, JakBlock, JakItem, JakKind};
use crate::jak_structures::{Land, Placed};

// ------------------------------------------------------------- models

/// An adobe wall's pillar: its upright edges rounded.
fn round_post() -> String {
    model(&[
        b([2.0, 0.0, -0.02], [14.0, 16.02, 16.02], "g"),
        b([-0.02, 0.0, 2.0], [16.02, 16.02, 14.0], "g"),
        b([0.8, 0.0, 0.8], [15.2, 16.02, 15.2], "g"),
    ])
}
fn adobe_arm() -> String {
    model(&[b([0.0, 0.0, 0.0], [16.0, 16.0, 8.0], "g")])
}
/// Crenellations: the wall's top, a merlon rounded on top over each half.
fn merlons_arm() -> String {
    model(&[
        b([0.0, 0.0, 0.0], [16.0, 9.0, 8.0], "g"),
        b([0.5, 9.0, 2.0], [15.5, 14.5, 6.0], "g"),
        b([1.5, 14.5, 2.5], [14.5, 16.0, 5.5], "g"),
    ])
}
fn merlons_post() -> String {
    model(&[
        b([0.8, 0.0, 0.8], [15.2, 9.02, 15.2], "g"),
        b([2.0, 0.0, -0.02], [14.0, 9.02, 16.02], "g"),
        b([-0.02, 0.0, 2.0], [16.02, 9.02, 14.0], "g"),
        b([3.0, 9.0, 3.0], [13.0, 16.0, 13.0], "g"),
        b([4.5, 16.0, 4.5], [11.5, 17.5, 11.5], "g"),
    ])
}
/// A window's half through the wall, in a crooked wooden frame.
fn crooked_half(alt: bool) -> String {
    let z = |a: f32, c: f32| if alt { (16.0 - c, 16.0 - a) } else { (a, c) };
    let bx = |x0: f32, y0: f32, za: f32, x1: f32, y1: f32, zc: f32, t: &'static str| {
        let (z0, z1) = z(za, zc);
        b([x0, y0, z0], [x1, y1, z1], t)
    };
    let tilt = if alt { 5.0 } else { -5.0 };
    let (j0, j1) = z(3.0, 4.6);
    model(&[
        bx(0.0, 0.0, 0.0, 16.0, 5.0, 8.0, "g"),
        bx(0.0, 12.0, 0.0, 16.0, 16.0, 8.0, "g"),
        bx(0.0, 5.0, 0.0, 16.0, 12.0, 4.0, "g"),
        turned([-1.0, 4.0, j0], [17.0, 13.0, j1], "w", 'x', tilt, [8.0, 8.0, (j0 + j1) / 2.0]),
        bx(-1.0, 12.0, 4.0, 17.0, 13.5, 8.0, "w"),
        bx(-1.0, 3.6, 4.0, 17.0, 5.0, 8.0, "w"),
    ])
}
fn crooked_half_north() -> String {
    crooked_half(false)
}
fn crooked_half_south() -> String {
    crooked_half(true)
}
fn nothing() -> String {
    String::new()
}
fn crooked_post() -> String {
    model(&[b([7.3, 5.0, 7.3], [8.7, 12.0, 8.7], "w")])
}

// Turned to face out (front -z), against the wall behind (+z).

/// A low rounded buttress at the wall's foot, rather than eight apparent
/// stair treads leading into a window. Kept within one half-height cell.
fn mound() -> String {
    let mut boxes = Vec::new();
    for i in 0..4 {
        let inset = i as f32 * 0.9;
        boxes.push(b([inset, 0.0, 4.0 * i as f32], [16.0 - inset, 2.0 * (i + 1) as f32, 16.0], "g"));
    }
    model(&boxes)
}
/// Thin metal horns sticking out of the wall, up and out.
fn horns() -> String {
    model(&[
        turned([3.0, 7.0, -8.0], [5.0, 9.0, 16.0], "m", 'x', 32.0, [4.0, 8.0, 16.0]),
        turned([11.0, 9.0, -4.0], [13.0, 11.0, 16.0], "m", 'x', 24.0, [12.0, 10.0, 16.0]),
        turned([3.5, 7.5, -10.0], [4.5, 8.5, -8.0], "k", 'x', 32.0, [4.0, 8.0, 16.0]),
    ])
}
/// A round metal pot hung on a bracket out of the wall.
fn pot() -> String {
    model(&[
        b([7.0, 13.0, 6.0], [9.0, 15.0, 16.0], "w"),
        b([7.5, 9.0, 7.5], [8.5, 13.0, 8.5], "m"),
        b([4.0, 2.0, 5.0], [12.0, 9.0, 11.0], "p"),
        b([5.0, 1.0, 4.0], [11.0, 10.0, 12.0], "p"),
        b([5.5, 0.0, 5.5], [10.5, 1.0, 10.5], "p"),
    ])
}
/// A pale buttress rib against a bunker's battered wall, three blocks high,
/// deep at its foot.
fn rib() -> String {
    let mut boxes = Vec::new();
    for i in 0..12 {
        let depth = 11.0 - i as f32 * 0.8;
        boxes.push(b([5.0, i as f32 * 4.0, 16.0 - depth], [11.0, i as f32 * 4.0 + 4.0, 16.0], "r"));
    }
    model(&boxes)
}
/// A tall tapering sandstone column, banded red and dark, two blocks high.
fn column() -> String {
    let mut boxes = Vec::new();
    for i in 0..8 {
        let r = 6.0 - i as f32 * 0.55;
        let tex = match i {
            1 => "e",
            4 => "d",
            _ => "s",
        };
        boxes.push(b([8.0 - r, i as f32 * 4.0, 8.0 - r], [8.0 + r, i as f32 * 4.0 + 4.0, 8.0 + r], tex));
    }
    boxes.push(b([6.5, 32.0, 6.5], [9.5, 34.0, 9.5], "s"));
    model(&boxes)
}

// ------------------------------------------------------------- blocks

type Parts = &'static [(&'static str, fn() -> String)];

const ADOBE: [(&str, fn() -> String); 3] = [("_post", round_post), ("_side", adobe_arm), ("_side_tall", adobe_arm)];
const MERLONS: [(&str, fn() -> String); 3] = [("_post", merlons_post), ("_side", merlons_arm), ("_side_tall", merlons_arm)];
const CROOKED: [(&str, fn() -> String); 5] = [("_post", crooked_post), ("_side", crooked_half_north), ("_side_alt", crooked_half_south), ("_noside", nothing), ("_noside_alt", nothing)];

const WHITE: &[(&str, &str)] = &[("g", "w4_wascity-stucco-wall-bleached-01"), ("w", "wascity-wood-plain")];
fn double_arch()->String {
    model(&[b([0.0,0.0,0.0],[16.0,16.0,16.0],"g"),
        b([-4.0,-48.0,2.0],[0.0,0.0,15.0],"g"),b([32.0,-48.0,2.0],[36.0,0.0,15.0],"g"),
        b([-2.0,-48.0,0.0],[0.0,0.0,2.0],"g"),b([32.0,-48.0,0.0],[34.0,0.0,2.0],"g"),
        b([-4.0,-4.0,-1.0],[36.0,0.0,5.0],"g"),b([-1.0,-1.0,-2.0],[33.0,2.0,1.0],"w")])
}
const SAND: &[(&str, &str)] = &[("g", "w4_wascity-stone-plain-wall-3"), ("w", "wascity-wood-plain")];
const GREY: &[(&str, &str)] = &[("g", "w4_wascity-wall-weathered"), ("t", "wascity-stucco-wall-supports"), ("d", "wascity-wood-plain")];
const FITTINGS: &[(&str, &str)] = &[("m", "wascity-wallspike-01"), ("k", "wascity-metal-pole"), ("w", "wascity-wood-plain"), ("p", "wascity-torch-tank"), ("r", "wascity-stucco-wall-supports"), ("s", "common_sandstone_pill01"), ("e", "wascity-steps-red"), ("d", "wascity-base")];

const fn set(path: &'static str, vanilla: &'static str, models: Parts, textures: &'static [(&'static str, &'static str)]) -> JakBlock {
    JakBlock { path, look: BlockLook::Set { vanilla, models, textures } }
}
const fn shaped(path: &'static str, vanilla: &'static str, model: fn() -> String, textures: &'static [(&'static str, &'static str)]) -> JakBlock {
    JakBlock { path, look: BlockLook::Shaped { vanilla, model, textures } }
}

pub const SPARG_BLOCKS: [JakBlock; 23] = [
    set("sparg_blanco_pared", "cobblestone_wall", &ADOBE, WHITE),
    set("sparg_arena_pared", "cobblestone_wall", &ADOBE, SAND),
    set("sparg_gris1_pared", "cobblestone_wall", &BASE1, GREY),
    set("sparg_gris2_pared", "cobblestone_wall", &BASE2, GREY),
    set("sparg_gris3_pared", "cobblestone_wall", &BASE3, GREY),
    set("sparg_almena_pared", "cobblestone_wall", &MERLONS, WHITE),
    set("sparg_almena_arena_pared", "cobblestone_wall", &MERLONS, SAND),
    set("sparg_blanco_vano", "glass_pane", &CROOKED, WHITE),
    set("sparg_arena_vano", "glass_pane", &CROOKED, SAND),
    shaped("sparg_rampa_blanca", "carved_pumpkin", mound, WHITE),
    shaped("sparg_rampa_arena", "carved_pumpkin", mound, SAND),
    shaped("sparg_cuerno", "carved_pumpkin", horns, FITTINGS),
    shaped("sparg_olla", "carved_pumpkin", pot, FITTINGS),
    shaped("sparg_costilla", "carved_pumpkin", rib, FITTINGS),
    shaped("sparg_columna", "carved_pumpkin", column, FITTINGS),
    JakBlock { path: "sparg_losa_capa", look: BlockLook::Layers { texture: "w4_wascity-cement-road", full: "sparg_losa" } },
    JakBlock { path: "sparg_arena_capa", look: BlockLook::Layers { texture: "w4_wascity-ground-01", full: "sparg_arena" } },
    JakBlock { path: "sparg_losa", look: BlockLook::All("w4_wascity-cement-road") },
    JakBlock { path: "sparg_arena", look: BlockLook::All("w4_wascity-ground-01") },
    JakBlock { path: "sparg_matorral", look: BlockLook::Plant { texture: "wascity-shrub-orange-01", models: crate::jak_spargus_shrubs::MODELS } },
    JakBlock { path: "sparg_azotea", look: BlockLook::Slab { texture: "w4_wascitya-stone-top", double: "sparg_arena" } },
    JakBlock { path:"spargus_puerta_alta",look:BlockLook::Like{vanilla:"oak_door",textures:&[("*","solid_wascity-metal-door-01")]} },
    shaped("sparg_portal_doble","carved_pumpkin",double_arch,WHITE),
];

pub const SPARG_ITEMS: [JakItem; 23] = [
    JakItem { path: "sparg_losa_capa", title: "Capa de losas de Spargus", kind: JakKind::Block },
    JakItem { path: "sparg_arena_capa", title: "Capa de arena de Spargus", kind: JakKind::Block },
    JakItem { path: "sparg_blanco_pared", title: "Muro de adobe blanco", kind: JakKind::Block },
    JakItem { path: "sparg_arena_pared", title: "Muro de adobe arena", kind: JakKind::Block },
    JakItem { path: "sparg_gris1_pared", title: "Muro de bÃºnker (pie)", kind: JakKind::Block },
    JakItem { path: "sparg_gris2_pared", title: "Muro de bÃºnker (medio)", kind: JakKind::Block },
    JakItem { path: "sparg_gris3_pared", title: "Muro de bÃºnker (remate)", kind: JakKind::Block },
    JakItem { path: "sparg_almena_pared", title: "Almenas blancas", kind: JakKind::Block },
    JakItem { path: "sparg_almena_arena_pared", title: "Almenas de arena", kind: JakKind::Block },
    JakItem { path: "sparg_blanco_vano", title: "Ventana de marco torcido (blanca)", kind: JakKind::Block },
    JakItem { path: "sparg_arena_vano", title: "Ventana de marco torcido (arena)", kind: JakKind::Block },
    JakItem { path: "sparg_rampa_blanca", title: "MontÃ­culo de adobe blanco", kind: JakKind::Block },
    JakItem { path: "sparg_rampa_arena", title: "MontÃ­culo de adobe arena", kind: JakKind::Block },
    JakItem { path: "sparg_cuerno", title: "Cuernos de metal", kind: JakKind::Block },
    JakItem { path: "sparg_olla", title: "Olla colgada", kind: JakKind::Block },
    JakItem { path: "sparg_costilla", title: "Contrafuerte de bÃºnker", kind: JakKind::Block },
    JakItem { path: "sparg_columna", title: "Columna de arenisca", kind: JakKind::Block },
    JakItem { path: "sparg_losa", title: "Losas oscuras de Spargus", kind: JakKind::Block },
    JakItem { path: "sparg_arena", title: "Arena de Spargus", kind: JakKind::Block },
    JakItem { path: "sparg_matorral", title: "Matorral naranja", kind: JakKind::Block },
    JakItem { path: "sparg_azotea", title: "Azotea de Spargus", kind: JakKind::Block },
    JakItem { path: "spargus_puerta_alta", title: "Puerta alta de Spargus", kind: JakKind::Block },
    JakItem { path: "sparg_portal_doble", title: "Portal doble de Spargus", kind: JakKind::Block },
];

// ------------------------------------------------------------- buildings

/// A Spargus building's plan.
#[derive(Clone, Copy, Debug)]
pub struct Spec {
    pub w: i32,
    pub d: i32,
    /// Storeys, three high each.
    pub storeys: i32,
    /// White stucco, else sand.
    pub white: bool,
    /// A grey battered bunker for its ground floor, ribbed, the rest a
    /// white room set in on it.
    pub bunker: bool,
    /// Mounds of earth against its feet.
    pub mounds: bool,
    /// Metal horns out of its top.
    pub horns: bool,
    /// Pots hung by its door.
    pub pots: bool,
    /// Sandstone columns before its door.
    pub columns: bool,
    pub setback: bool,
    pub dome: bool,
    pub awning: bool,
}

pub const HOUSE: Spec = Spec { w: 7, d: 6, storeys: 2, white: true, bunker: false, mounds: true, horns: true, pots: true, columns: false, setback:false,dome:false,awning:false };
pub const SAND_HOUSE: Spec = Spec { w: 8, d: 6, storeys: 2, white: false, columns: true, ..HOUSE };
pub const BUNKER: Spec = Spec { w: 8, d: 7, storeys: 2, bunker: true, mounds: false, pots: false, ..HOUSE };

/// A building of Spargus made up from `h`, at most `max_w` by `max_d`.
pub fn random_spec(h: u64, max_w: i32, max_d: i32) -> Spec {
    let bit = |k: u32, of: u64| (h >> k) % of == 0;
    let bunker = bit(10, 5);
    Spec {
        w: (5 + (h % 5) as i32).min(max_w).max(5),
        d: (5 + ((h >> 4) % 3) as i32).min(max_d).max(5),
        storeys: if bunker { 2 } else { 1 + ((h >> 7) % 3) as i32 },
        white: bit(12, 2),
        bunker,
        mounds: !bunker && !bit(14, 3),
        horns: bit(16, 2),
        pots: bit(18, 2),
        columns: bit(20, 3),
        setback:max_w>=8 && bit(23,3),dome:bit(26,4),awning:bit(29,2),
    }
}

pub const ARCHETYPE_NAMES:[&str;16]=["Vivienda blanca","Casa arena","Almacen acorazado","Casa con cupula","Casa en terrazas","Mercado con toldo","Torre escalonada","Casa de columnas","Casa de tinajas","Vivienda baja","Posada de Spargus","Casa del pozo","Casa de balcones","Taller de metal","Puesto de patrulla","Casa del puerto"];
pub fn archetype(index:usize,w:i32,d:i32)->Spec {
    let i=index%16;
    Spec {w:w.clamp(5,17),d:d.clamp(5,16),storeys:[2,2,2,2,3,1,4,2,2,1,3,2,3,2,3,2][i],white:i%3!=1,bunker:matches!(i,2|13|14),
        mounds:i%3==0,horns:i%2==0,pots:i%3!=1,columns:matches!(i,3|7|10|11|15),setback:w>=8&&d>=8&&matches!(i,4|6|10|12|14),dome:matches!(i,3|8|11|15),awning:matches!(i,5|7|9|10|13|15)}
}
/// A building of Spargus at `origin` (its floor's middle, on the ground),
/// its front toward `front`, as `spec` makes it: whole all round; inside a
/// floor a storey, a ladder up to a hatch in its roof, a bed and a chest of
/// Jak 3's things.
pub fn building(origin: (i32, i32, i32), front: u8, spec: Spec) -> Vec<Placed> {
    let Spec { w, d, storeys, .. } = spec;
    let s = Site { origin, front, w, d };
    let mut p = Plan::default();
    let height = storeys * 3;
    let (wall, window, merlons, mound) = if spec.white {
        ("jak:sparg_blanco_pared", "jak:sparg_blanco_vano", "jak:sparg_almena_pared", "jak:sparg_rampa_blanca")
    } else {
        ("jak:sparg_arena_pared", "jak:sparg_arena_vano", "jak:sparg_almena_arena_pared", "jak:sparg_rampa_arena")
    };
    // A bunker's room on top is set in a block all round.
    let inset = |k:i32| if spec.setback {k.min(2).min((w.min(d)-4)/2).max(0)} else if spec.bunker && k>0 {1}else{0};
    for k in 0..storeys {
        let i = inset(k);
        let (bw, bd) = (w - 2 * i, d - 2 * i);
        for (x, z, way, corner) in ring_cells(i, i, bw, bd) {
            let along = if way % 2 == 0 { x - i } else { z - i };
            let span = if way % 2 == 0 { bw } else { bd };
            for r in 0..3 {
                let y = 3 * k + r;
                let cell = if spec.bunker && k == 0 {
                    ["jak:sparg_gris1_pared", "jak:sparg_gris2_pared", "jak:sparg_gris3_pared"][r as usize]
                } else if !corner && r == 1 && along > 0 && along < span - 1 && (along % 3 == 1 || (way != 0 && along == span / 2)) && !(way == 0 && k == 0 && along == span / 2) {
                    window
                } else {
                    wall
                };
                p.put(s.at(x, y, z), Cell::Join(cell));
            }
        }
        // Its floor, and the bunker's roof round the room set in.
        for x in 1..w - 1 {
            for z in 1..d - 1 {
                p.fixed(s.at(x, 3 * k - 1, z), if k == 0 { "jak:sparg_arena" } else { "jak:spargus_planks" });
            }
        }
        if i > 0 {
            for (x, z, _, _) in ring_cells(0, 0, w, d) {
                p.fixed(s.at(x, 3 * k - 1 + 1, z), "jak:sparg_azotea[type=bottom,waterlogged=false]");
            }
        }
    }
    // The roof behind its merlons.
    let top = storeys - 1;
    let i = inset(top);
    for x in 1 + i..w - 1 - i {
        for z in 1 + i..d - 1 - i {
            p.fixed(s.at(x, height - 1, z), "jak:sparg_azotea[type=top,waterlogged=false]");
        }
    }
    for (x, z, _, _) in ring_cells(i, i, w - 2 * i, d - 2 * i) {
        p.put(s.at(x, height, z), Cell::Join(merlons));
    }
    if spec.dome && w>=6 && d>=6 {
        for (dx,dz,way) in [(0,0,0),(1,0,1),(1,1,2),(0,1,3)] {p.fixed(s.at(w/2+dx,height,d/2+dz),s.facing("jak:spargus_cupula[facing={f}]",way));}
    }
    if spec.awning {
        for x in 1..w-1 {for z in -2..0 {p.fixed(s.at(x,2,z),s.facing("jak:spargus_toldo[facing={f},half=top,open=false,powered=false,waterlogged=false]",0));}}
        for x in [1,w-2] {for y in 0..2 {p.fixed(s.at(x,y,-2),"jak:spargus_viga[facing=up,powered=false,waterlogged=false]");}}
    }
    for k in 1..storeys {let i=inset(k);if i>0 {for (y,half) in [(3*k,"lower"),(3*k+1,"upper")] {p.fixed(s.at(w/2,y,i),s.facing(&format!("jak:spargus_puerta_alta[facing={{f}},half={half},hinge=left,open=false,powered=false]"),0));}}}
    // The door under its arch, set back in the wall.
    let dx = w / 2 - 1;
    for (y, half) in [(0, "lower"), (1, "upper")] {
        p.fixed(s.at(dx, y, 0), s.facing(&format!("jak:spargus_puerta_alta[facing={{f}},half={half},hinge=left,open=false,powered=false]"), 0));
    }
    if !spec.bunker {
        p.fixed(s.at(dx, 2, 0), s.facing("jak:spargus_arco[facing={f}]", 0));
    }
    // What stands against its walls outside.
    for (x, z, way, corner) in ring_cells(0, 0, w, d) {
        let (ox, oz) = match way {
            0 => (x, z - 1),
            2 => (x, z + 1),
            1 => (x + 1, z),
            _ => (x - 1, z),
        };
        let along = if way % 2 == 0 { x } else { z };
        if corner || (way == 0 && (along - dx).abs() <= 1) {
            continue;
        }
        if spec.bunker && along % 2 == 1 {
            p.fixed(s.at(ox, 0, oz), s.facing("jak:sparg_costilla[facing={f}]", way));
        } else if spec.mounds && (along == 1 || (way != 0 && along % 3 == 0)) {
            p.fixed(s.at(ox, 0, oz), s.facing(&format!("{mound}[facing={{f}}]"), way));
        }
        if spec.horns && along % 3 == 2 {
            let i = inset(top);
            let (hx, hz) = match way {
                0 => (x, i - 1),
                2 => (x, d - i),
                1 => (w - i, z),
                _ => (i - 1, z),
            };
            p.fixed(s.at(hx, height - 2, hz), s.facing("jak:sparg_cuerno[facing={f}]", way));
        }
    }
    if spec.pots {
        for x in [dx - 1, dx + 1] {
            p.fixed(s.at(x, 2, -1), s.facing("jak:sparg_olla[facing={f}]", 0));
        }
    }
    if spec.columns {
        for x in [dx - 2, dx + 2] {
            p.fixed(s.at(x, 0, -2), s.facing("jak:sparg_columna[facing={f}]", 0));
        }
    }
    // Inside.
    for y in 0..height - 1 {
        p.fixed(s.at(1 + i, y, d - 2 - i), s.facing("minecraft:ladder[facing={f},waterlogged=false]", 0));
    }
    p.fixed(s.at(1 + i, height - 1, d - 2 - i), s.facing("minecraft:spruce_trapdoor[facing={f},half=top,open=false,powered=false,waterlogged=false]", 0));
    for (part, z) in [("foot", d - 3), ("head", d - 2)] {
        p.fixed(s.at(w - 2, 0, z), s.facing(&format!("minecraft:red_bed[facing={{f}},occupied=false,part={part}]"), 2));
    }
    p.fixed(s.at(w - 3, 0, d - 2), s.facing("minecraft:chest[facing={f},type=single,waterlogged=false]", 0));
    for k in 0..storeys {
        let i = inset(k);
        p.fixed(s.at(w-2-i,3*k+1,1+i),"minecraft:lantern[hanging=true,waterlogged=false]");
        for x in 1 + i..w - 1 - i {
            for z in 1 + i..d - 1 - i {
                for y in [3 * k, 3 * k + 1] {
                    p.air_if_empty(s.at(x, y, z));
                }
            }
        }
    }
    p.tall_storeys(origin.1);
    crate::jak_workshops::add(&mut p,&s,false,true);
    p.doorways("jak:sparg_arena");
    p.finish()
}

/// A round white tower four by four at `origin`, `storeys` high, its door
/// toward `front`: its corners curved (`spargus_torre_curva`), windows,
/// merlons and horns at its top, inside a ladder to its roof.
pub fn tower(origin: (i32, i32, i32), front: u8, storeys: i32) -> Vec<Placed> {
    let s = Site { origin, front, w: 4, d: 4 };
    let mut p = Plan::default();
    let height = storeys * 3;
    for (x, z, way, corner) in ring_cells(0, 0, 4, 4) {
        for y in 0..height {
            if corner {
                p.fixed(s.at(x, y, z), s.facing("jak:spargus_torre_curva[facing={f}]", way));
            } else {
                let cell = if y % 3 == 1 && y > 1 && (x + z) % 2 == 0 { "jak:sparg_blanco_vano" } else { "jak:sparg_blanco_pared" };
                p.put(s.at(x, y, z), Cell::Join(cell));
            }
        }
        if corner {
            p.fixed(s.at(x, height, z), s.facing("jak:spargus_pinchos[facing={f}]", way));
        } else {
            p.put(s.at(x, height, z), Cell::Join("jak:sparg_almena_pared"));
            let (ox, oz) = match way {
                0 => (x, z - 1),
                2 => (x, z + 1),
                1 => (x + 1, z),
                _ => (x - 1, z),
            };
            if (x + z) % 2 == 1 {
                p.fixed(s.at(ox, height - 2, oz), s.facing("jak:sparg_cuerno[facing={f}]", way));
            }
        }
    }
    for (y, half) in [(0, "lower"), (1, "upper")] {
        p.fixed(s.at(1, y, 0), s.facing(&format!("jak:spargus_puerta_alta[facing={{f}},half={half},hinge=left,open=false,powered=false]"), 0));
    }
    for x in 1..3 {
        for z in 1..3 {
            p.fixed(s.at(x, -1, z), "jak:sparg_arena");
            p.fixed(s.at(x, height - 1, z), "jak:sparg_azotea[type=top,waterlogged=false]");
        }
    }
    for y in 0..height - 1 {
        p.fixed(s.at(2, y, 2), s.facing("minecraft:ladder[facing={f},waterlogged=false]", 0));
    }
    p.fixed(s.at(2, height - 1, 2), s.facing("minecraft:spruce_trapdoor[facing={f},half=top,open=false,powered=false,waterlogged=false]", 0));
    for y in 0..height - 1 {
        p.air_if_empty(s.at(1, y, 1));
        p.air_if_empty(s.at(1, y, 2));
        p.air_if_empty(s.at(2, y, 1));
    }
    p.tall_storeys(origin.1);
    p.doorways("jak:sparg_arena");
    p.finish()
}

// ------------------------------------------------------------- town

fn mix(seed: i64, a: i32, b: i32, salt: u64) -> u64 {
    let mut h = (seed as u64) ^ ((a as u32 as u64) << 32 | u64::from(b as u32)) ^ salt.wrapping_mul(0x9E37_79B9_7F4A_7C15);
    h ^= h >> 33;
    h = h.wrapping_mul(0xff51_afd7_ed55_8ccd);
    h ^= h >> 33;
    h = h.wrapping_mul(0xc4ce_b9fe_1a85_ec53);
    h ^ (h >> 33)
}

/// A town of Spargus at `(cx, cz)`, its plaza's floor `y0`, turned `turn`
/// (`jak_town`'s plan): its ground levelled, of sand; its ring road and
/// streets of dark flagstones; along it houses of every kind Spargus has,
/// made up one by one (`random_spec`, and the older stucco houses), round
/// towers on its corners; in its plaza the Wastelanders' totem with its
/// bell, sandstone columns round it, orange shrubs.
pub fn town(seed: i64, (cx, cz): (i32, i32), y0: i32, turn: u8, land: &dyn Land) -> Vec<Placed> {
    if !crate::jak_town::gentle_site(land,cx,cz,y0){return Vec::new();}
    if !(-11..=11).all(|dx|(-11..=11).all(|dz|land.city_allowed(cx+dx,cz+dz,true)&&!land.wet(cx+dx,cz+dz))) {return Vec::new();}
    let mut k = 0u64;
    let mut plan = crate::jak_town::plan_seeded(seed,(cx, y0, cz), turn, &mut |max, corner| {
        k += 1;
        let h = mix(seed, cx, cz, 200 + k);
        if corner {
            (4, 4)
        } else {
            ((6 + (h % 6) as i32).min(max), 6 + ((h >> 8) % 4) as i32)
        }
    });
    plan.spread_spargus(seed);
    plan.grade(seed, land, true);
    if plan.lots.len()<12 {return Vec::new();}
    let site = plan.site();
    let reach = crate::jak_town::REACH;
    let mut out = plan.terrain(land, true);
    out.extend(plan.civic(true,land));
    let mut built = std::collections::HashSet::new();
    for (i, lot) in plan.lots.iter().enumerate() {
        let h = mix(seed, cx, cz, 300 + i as u64);
        let mut spec=archetype(((h>>8) as usize+i)%16,lot.w,lot.d);
        if lot.corner {spec.storeys=3;spec.setback=true;}
        let made=building(lot.origin,lot.front,spec);
        for (at, state) in made {
            built.insert(at);
            out.push((at, state));
        }
    }
    // Its middle's mark under the plaza (its people and traffic).
    out.push((site.at(0, -3, 0), crate::jak_town::MARK));
    out.push((site.at(0, -4, 0), crate::jak_town::EXPANDED_MARK));
    out.push((site.at(0, -5, 0), crate::jak_town::CITY_MARK));
    out.push((site.at(0, -6, 0), crate::jak_town::SPARGUS_MARK));
    // The plaza: the totem and its bell, columns round it, shrubs.
    for y in 0..3 {
        out.push((site.at(0, y, 0), "jak:totem_stone"));
    }
    out.push((site.at(2, 0, 0), "minecraft:bell[attachment=floor,facing=north,powered=false]"));
    for (x, z) in [(5, 5), (-5, 5), (5, -5), (-5, -5)] {
        out.push((site.at(x, 0, z), site.facing("jak:sparg_columna[facing={f}]", 0)));
    }
    for i in 0..60u64 {
        let h = mix(seed, cx, cz, 100 + i);
        let (x, z) = ((h % (2 * reach as u64)) as i32 - reach, ((h >> 20) % (2 * reach as u64)) as i32 - reach);
        let at = site.at(x, plan.eighths(x,z).div_euclid(8)-y0, z);
        if built.contains(&at) || matches!(plan.ground(x, z), crate::jak_town::Ground::Road | crate::jak_town::Ground::Street) || (x.abs() <= 2 && z.abs() <= 2)
            || plan.eighths(x,z).rem_euclid(8)!=0 || plan.coverage(x,z)<0.8 || !land.city_allowed(at.0,at.2,true) || land.wet(at.0,at.2) {
            continue;
        }
        out.push((at, "jak:sparg_matorral"));
    }
    crate::jak_town::original_props(&plan, seed, true, land, &mut out);
    // Workshops and their entire driving aprons must remain inside dry desert.
    for (z,toad) in [(-14,true),(14,false)] {
        let at=site.at(70,0,z);let y=y0;
        if (land.ground(at.0,at.2)-y).abs()<=4 && (-7..=7).all(|dx|(-7..=7).all(|dz|land.city_allowed(at.0+dx,at.2+dz,true)&&!land.wet(at.0+dx,at.2+dz)&&(land.ground(at.0+dx,at.2+dz)-y).abs()<=4)) {
            let garage=crate::jak_garages::building_on_grade((at.0,y,at.2),site.way(1),toad,y);
            let footprint:std::collections::HashSet<_>=garage.iter().map(|(p,_)|(p.0,p.2)).collect();
            if !footprint.iter().all(|&(x,z)|land.city_allowed(x,z,true)&&!land.wet(x,z)&&(land.ground(x,z)-y).abs()<=4){continue;}
            out.extend(garage);
            // A workshop opens onto its manoeuvring yard, not an old pedestrian alley.
            for dx in 75..=94 {for dz in z-12..=z+12 {
                let ground=site.at(dx,0,dz);
                if land.wet(ground.0,ground.2)||!land.city_allowed(ground.0,ground.2,true){continue;}
                for dy in -5..0 {out.push((site.at(dx,dy,dz),"jak:sparg_losa"));}
                for dy in 0..7 {out.push((site.at(dx,dy,dz),"minecraft:air"));}
            }}
        }
    }
    crate::jak_beds::arrange(&mut out);
    out
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

    /// Every part's model valid, every state of a town parses, its kinds
    /// all there, every made-up building whole.
    #[test]
    fn spargus_towns_are_whole() {
        for block in &SPARG_BLOCKS {
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
        for h in 0..200u64 {
            let spec = random_spec(h.wrapping_mul(0x9E37_79B9_7F4A_7C15), 9, 7);
            let built = building((0, 70, 0), (h % 4) as u8, spec);
            assert!(built.iter().any(|(_, s)| s.contains("almena")), "{spec:?}");
        }
        let blocks = town(5, (0, 0), 70, 2, &Flat(70));
        let Ok(paths) = minecraftoss_core::registries::DataPaths::discover() else { return };
        if !paths.block_catalog.is_file() {
            return;
        }
        let registries = minecraftoss_core::Registries::load(&paths).unwrap();
        for (_, state) in &blocks {
            assert!(registries.blocks.parse_state(state).is_ok(), "{state}");
        }
        let count = |id: &str| blocks.iter().filter(|(_, s)| s.starts_with(id)).count();
        for id in ["jak:sparg_blanco_pared", "jak:sparg_arena_pared", "jak:sparg_almena_pared", "jak:sparg_blanco_vano", "jak:sparg_cuerno", "jak:sparg_losa", "jak:totem_stone", "minecraft:ladder", "minecraft:chest"] {
            assert!(count(id) > 0, "{id}");
        }
    }
}
