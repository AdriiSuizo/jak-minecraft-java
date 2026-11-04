//! Villages of facade blocks (Adrii, 23/10: "que los poblados sean con
//! construcciones que sean adaptaciones en 3d de las fachadas de los barrios
//! de Jak 2 y 3… bloques de minecraft con forma de las partes de la casa a
//! construir"): Minecraft blocks each shaped as a part of a house of Haven
//! City's slums (Jak 2) or of Spargus (Jak 3), in those places' textures,
//! and the houses and villages built of them.
//!
//! A part is a block model of boxes (as Minecraft's own lanterns, stairs or
//! hoppers), drawn by a vanilla block's behaviour: a carved pumpkin's for a
//! wall part (turned to face out, solid), a lightning rod's for a beam,
//! post or pipe (turned any way, thin), and vanilla's own doors, stairs and
//! trapdoors in the places' textures for doors, sloped roofs and awnings.
//!
//! - Spargus: bleached stucco walls with rounded corners and buttresses,
//!   windows with a wooden lintel and sill, beams through the walls under
//!   the roof, a flat roof behind a parapet with spikes, a metal dome,
//!   awnings over the doors on posts, canisters.
//! - Haven's slums: a stone ground floor, an upper floor of boards jutting
//!   out over the street on posts, corrugated metal, boarded windows, a
//!   corrugated roof sloping back, rusted awnings, pipes.
//!
//! The Wasteland's villages are Spargus's; the other lands' villages
//! (`jak_structures`) Haven's.
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use minecraftoss_core::block::BlockStateId;
use minecraftoss_core::chunk::{Chunk, HeightmapKind};
use minecraftoss_world::chunk_map::WorldGen;

use crate::jak_items::{BlockLook, JakBlock, JakItem, JakKind};
use crate::jak_structures::{Land, Placed};

// ---------------------------------------------------------------- models

/// A box of a part's model: from, to (sixteenths of the block, the front
/// is north, -z), its texture key, an optional turn (axis, angle, origin)
/// and an optional texture key for its ends (east and west: a log's).
#[derive(Clone)]
pub(crate) struct B {
    pub(crate) from: [f32; 3],
    pub(crate) to: [f32; 3],
    pub(crate) tex: &'static str,
    pub(crate) turn: Option<(char, f32, [f32; 3])>,
    pub(crate) ends: Option<&'static str>,
    pub(crate) bottom: Option<&'static str>,
}

pub(crate) const fn b(from: [f32; 3], to: [f32; 3], tex: &'static str) -> B {
    B { from, to, tex, turn: None, ends: None, bottom:None }
}

pub(crate) fn underside(mut part:B,tex:&'static str)->B{part.bottom=Some(tex);part}

/// Merge equal adjacent volumes before emitting quads. Roof courses used
/// to repeat the same body eight times across each tile and draw every
/// internal face. Texture coordinates remain in the original block frame.
fn merge_boxes(input:&[B])->Vec<B>{
    let mut boxes=input.to_vec();
    loop {
        let mut found=None;
        'search:for i in 0..boxes.len(){for j in i+1..boxes.len(){
            let(a,b)=(&boxes[i],&boxes[j]);
            if a.turn.is_some()||b.turn.is_some()||a.tex!=b.tex||a.ends!=b.ends||a.bottom!=b.bottom{continue;}
            for axis in 0..3 {if (a.to[axis]-b.from[axis]).abs()<0.001 || (b.to[axis]-a.from[axis]).abs()<0.001 {
                if (0..3).filter(|k|*k!=axis).all(|k|a.from[k]==b.from[k]&&a.to[k]==b.to[k]){found=Some((i,j));break 'search;}
            }}
        }}
        let Some((i,j))=found else{break;};let b=boxes.remove(j);
        for k in 0..3{boxes[i].from[k]=boxes[i].from[k].min(b.from[k]);boxes[i].to[k]=boxes[i].to[k].max(b.to[k]);}
    }boxes
}

fn hidden_face(boxes:&[B],index:usize,axis:usize,high:bool)->bool{
    let a=&boxes[index];if a.turn.is_some(){return false;}
    let at=if high{a.to[axis]}else{a.from[axis]};
    boxes.iter().enumerate().any(|(j,b)|j!=index && b.turn.is_none()
        && (if high{b.from[axis]<=at+0.001&&b.to[axis]>at+0.001}else{b.to[axis]>=at-0.001&&b.from[axis]<at-0.001})
        && (0..3).filter(|k|*k!=axis).all(|k|b.from[k]<=a.from[k]&&b.to[k]>=a.to[k]))
}

/// A face's texture square: its stretch of the box, kept within the
/// texture (a box sticking out of the block starts the texture over).
fn span(a: f32, b: f32) -> (f32, f32) {
    let w = (b - a).min(16.0);
    let start = a.rem_euclid(16.0);
    if start + w <= 16.0 { (start, start + w) } else { (0.0, w) }
}

pub(crate) fn model(boxes: &[B]) -> String {
    let boxes=merge_boxes(boxes);
    let mut out = Vec::new();
    for (index,x) in boxes.iter().enumerate() {
        let [x0, y0, z0] = x.from;
        let [x1, y1, z1] = x.to;
        let (u0, u1) = span(x0, x1);
        let (zu0, zu1) = span(z0, z1);
        let (v0, v1) = span(16.0 - y1, 16.0 - y0);
        let face = |uv: [f32; 4], tex: &str| format!(r##"{{"uv":[{},{},{},{}],"texture":"#{}"}}"##, uv[0], uv[1], uv[2], uv[3], tex);
        // A log's ends show its whole end texture.
        let end = |uv: [f32; 4]| match x.ends {
            Some(tex) => face([0.0, 0.0, 16.0, 16.0], tex),
            None => face(uv, x.tex),
        };
        let face_values=[
            face([u0, v0, u1, v1], x.tex),
            face([u0, v0, u1, v1], x.tex),
            end([zu0, v0, zu1, v1]),
            end([zu0, v0, zu1, v1]),
            face([u0, zu0, u1, zu1], x.tex),
            face([u0, zu0, u1, zu1], x.bottom.unwrap_or(x.tex)),
        ];
        let faces=[("north",2,false),("south",2,true),("east",0,true),("west",0,false),("up",1,true),("down",1,false)]
            .into_iter().zip(face_values).filter(|((_,axis,high),_)|!hidden_face(&boxes,index,*axis,*high))
            .map(|((name,_,_),value)|format!("\"{name}\":{value}")).collect::<Vec<_>>().join(",");
        let turn = x.turn.map_or(String::new(), |(axis, angle, o)| {
            format!(r#","rotation":{{"origin":[{},{},{}],"axis":"{axis}","angle":{angle}}}"#, o[0], o[1], o[2])
        });
        out.push(format!(r#"{{"from":[{x0},{y0},{z0}],"to":[{x1},{y1},{z1}]{turn},"faces":{{{faces}}}}}"#));
    }
    out.join(",")
}

// Spargus's parts.

fn spargus_window() -> String {
    model(&[
        b([0.0, 0.0, 4.0], [16.0, 16.0, 16.0], "s"),
        b([0.0, 0.0, 0.0], [4.0, 16.0, 4.0], "s"),
        b([12.0, 0.0, 0.0], [16.0, 16.0, 4.0], "s"),
        b([4.0, 0.0, 0.0], [12.0, 4.0, 4.0], "s"),
        b([4.0, 12.0, 0.0], [12.0, 16.0, 4.0], "s"),
        b([4.0, 4.0, 3.5], [12.0, 12.0, 4.0], "g"),
        // The lintel and sill, wood, sticking out.
        b([2.0, 12.0, -2.0], [14.0, 14.0, 0.0], "w"),
        b([3.0, 3.0, -2.0], [13.0, 4.0, 0.0], "w"),
    ])
}

/// Its front-left upright edge rounded (the stucco houses' corners).
fn spargus_corner() -> String {
    model(&[
        b([5.0, 0.0, 0.0], [16.0, 16.0, 16.0], "s"),
        b([2.0, 0.0, 1.0], [5.0, 16.0, 16.0], "s"),
        b([0.0, 0.0, 5.0], [2.0, 16.0, 16.0], "s"),
    ])
}

/// A wall with a buttress stepping out at its foot.
fn spargus_buttress() -> String {
    model(&[
        b([0.0, 0.0, 0.0], [16.0, 16.0, 16.0], "s"),
        b([4.0, 0.0, -6.0], [12.0, 6.0, 0.0], "p"),
        b([4.0, 6.0, -4.0], [12.0, 11.0, 0.0], "p"),
        b([4.0, 11.0, -2.0], [12.0, 15.0, 0.0], "p"),
    ])
}

/// A parapet's top with its spikes.
fn spargus_spikes() -> String {
    let mut boxes = vec![b([0.0, 0.0, 3.0], [16.0, 5.0, 13.0], "s")];
    for x in [1.5f32, 6.5, 11.5] {
        boxes.push(b([x, 5.0, 6.5], [x + 3.0, 11.0, 9.5], "k"));
        boxes.push(b([x + 0.75, 11.0, 7.25], [x + 2.25, 16.0, 8.75], "k"));
    }
    model(&boxes)
}

/// A beam, along its rod (up in the model).
fn beam() -> String {
    model(&[b([5.0, 0.0, 5.0], [11.0, 16.0, 11.0], "w")])
}

fn pipe() -> String {
    model(&[
        b([6.0, 0.0, 6.0], [10.0, 16.0, 10.0], "w"),
        b([5.0, 2.0, 5.0], [11.0, 3.5, 11.0], "w"),
        b([5.0, 12.5, 5.0], [11.0, 14.0, 11.0], "w"),
    ])
}

fn canister() -> String {
    model(&[
        b([3.0, 0.0, 1.0], [13.0, 14.0, 15.0], "c"),
        b([1.0, 0.0, 3.0], [15.0, 14.0, 13.0], "c"),
        b([4.0, 14.0, 4.0], [12.0, 15.0, 12.0], "w"),
    ])
}

/// A quarter of a dome two blocks across, its middle at the block's back
/// right corner (four of them turned make the dome).
fn dome_quarter() -> String {
    let mut boxes = Vec::new();
    for (y0, y1, r) in [(0.0f32, 4.0f32, 16.0f32), (4.0, 8.0, 14.5), (8.0, 11.0, 12.0), (11.0, 14.0, 8.5), (14.0, 16.0, 4.5)] {
        let near = r * 0.62;
        boxes.push(b([16.0 - r, y0, 16.0 - near], [16.0, y1, 16.0], "r"));
        boxes.push(b([16.0 - near, y0, 16.0 - r], [16.0, y1, 16.0], "r"));
    }
    model(&boxes)
}

// Haven's slums' parts.

fn haven_boards() -> String {
    model(&[
        b([0.0, 0.0, 2.0], [16.0, 16.0, 16.0], "d"),
        b([0.0, 1.0, 0.5], [16.0, 5.0, 2.0], "p"),
        b([0.0, 6.0, 1.0], [16.0, 10.0, 2.0], "p"),
        b([0.0, 11.0, 0.3], [16.0, 15.0, 2.0], "p"),
        b([12.0, 0.0, 0.0], [14.0, 16.0, 1.0], "p"),
    ])
}

fn haven_metal() -> String {
    let mut boxes = vec![b([0.0, 0.0, 2.0], [16.0, 16.0, 16.0], "m")];
    for x in [0.0f32, 4.0, 8.0, 12.0] {
        boxes.push(b([x + 1.0, 0.0, 0.5], [x + 3.0, 16.0, 2.0], "m"));
    }
    model(&boxes)
}

fn haven_window() -> String {
    model(&[
        b([0.0, 0.0, 3.0], [16.0, 16.0, 16.0], "s"),
        b([0.0, 0.0, 0.0], [3.0, 16.0, 3.0], "s"),
        b([13.0, 0.0, 0.0], [16.0, 16.0, 3.0], "s"),
        b([3.0, 0.0, 0.0], [13.0, 4.0, 3.0], "s"),
        b([3.0, 12.0, 0.0], [13.0, 16.0, 3.0], "s"),
        b([3.0, 4.0, 2.5], [13.0, 12.0, 3.0], "g"),
        // Two boards nailed across it.
        B { from: [1.5, 7.0, 1.0], to: [14.5, 9.0, 2.0], tex: "p", turn: Some(('z', 22.5, [8.0, 8.0, 1.5])), ends: None, bottom:None },
        B { from: [1.5, 7.0, 0.0], to: [14.5, 9.0, 1.0], tex: "p", turn: Some(('z', -22.5, [8.0, 8.0, 0.5])), ends: None, bottom:None },
    ])
}

/// A stone base wall with its plinth of bricks.
fn haven_base() -> String {
    model(&[b([0.0, 0.0, 0.0], [16.0, 16.0, 16.0], "s"), b([0.0, 0.0, -1.0], [16.0, 4.0, 0.0], "k")])
}

// Spargus's parts, as its houses in Jak 3's `wascitya` stand (Adrii 23/10:
// "mucho más parecidas a las del universo de Jak"; drawn from the level).

/// A wall's foot: its stucco over the stones, a sloped plinth of them
/// stepping out at the front.
fn spargus_plinth() -> String {
    model(&[
        b([0.0, 0.0, 0.0], [16.0, 16.0, 16.0], "z"),
        b([0.0, 0.0, -4.0], [16.0, 3.0, 0.0], "z"),
        b([0.0, 3.0, -2.5], [16.0, 6.0, 0.0], "z"),
        b([0.0, 6.0, -1.0], [16.0, 8.0, 0.0], "z"),
    ])
}

/// A wall's top: the stucco swelling out and rounding over.
fn spargus_cap() -> String {
    model(&[
        b([0.0, 0.0, 0.0], [16.0, 11.0, 16.0], "s"),
        b([0.0, 7.0, -1.0], [16.0, 12.0, 0.0], "s"),
        b([0.0, 11.0, 0.0], [16.0, 13.5, 16.0], "s"),
        b([0.0, 13.5, 1.5], [16.0, 15.0, 16.0], "s"),
        b([0.0, 15.0, 3.5], [16.0, 16.0, 16.0], "s"),
    ])
}

/// A small window through the wall, narrowing at its top, a drip of dark
/// under it (seen through, and light comes in: Adrii 23/10, "ventanas de
/// verdad").
fn spargus_slit() -> String {
    model(&[
        b([0.0, 0.0, 0.0], [6.0, 16.0, 16.0], "s"),
        b([10.0, 0.0, 0.0], [16.0, 16.0, 16.0], "s"),
        b([6.0, 0.0, 0.0], [10.0, 5.0, 16.0], "s"),
        b([6.0, 11.0, 0.0], [7.0, 16.0, 16.0], "s"),
        b([9.0, 11.0, 0.0], [10.0, 16.0, 16.0], "s"),
        b([7.0, 13.0, 0.0], [9.0, 16.0, 16.0], "s"),
        b([7.5, 2.5, -0.1], [8.5, 5.0, 0.0], "n"),
    ])
}

/// A window through the wall in a wooden frame, its metal slats apart
/// (seen through), a metal awning over it with its teeth hanging.
fn spargus_shutter() -> String {
    let mut boxes = vec![
        b([0.0, 0.0, 0.0], [3.5, 16.0, 16.0], "s"),
        b([12.5, 0.0, 0.0], [16.0, 16.0, 16.0], "s"),
        b([3.5, 0.0, 0.0], [12.5, 3.5, 16.0], "s"),
        b([3.5, 12.0, 0.0], [12.5, 16.0, 16.0], "s"),
        b([2.5, 2.5, -1.0], [13.5, 4.0, 2.0], "w"),
        b([3.0, 11.5, -0.5], [13.0, 13.0, 2.0], "w"),
        b([3.0, 4.0, -0.5], [4.5, 11.5, 2.0], "w"),
        b([11.5, 4.0, -0.5], [13.0, 11.5, 2.0], "w"),
        B { from: [1.0, 14.0, -6.0], to: [15.0, 15.0, 0.0], tex: "m", turn: Some(('x', -22.5, [8.0, 14.5, 0.0])), ends: None, bottom:None },
    ];
    for y in [5.0f32, 7.0, 9.0] {
        boxes.push(b([4.5, y, 0.5], [11.5, y + 0.8, 1.5], "m"));
    }
    for x in [2.0f32, 5.0, 8.0, 11.0, 13.5] {
        boxes.push(b([x, 11.5, -5.0], [x + 1.0, 13.0, -4.0], "t"));
    }
    model(&boxes)
}

/// Over a door: a rounded wooden arch round the door's top, stucco about
/// it.
fn spargus_arch() -> String {
    model(&[
        b([0.0, 0.0, 2.0], [16.0, 16.0, 16.0], "s"),
        b([0.0, 11.5, 0.0], [16.0, 16.0, 2.0], "s"),
        b([0.0, 0.0, 0.0], [1.0, 11.5, 2.0], "s"),
        b([15.0, 0.0, 0.0], [16.0, 11.5, 2.0], "s"),
        b([1.0, 0.0, -1.0], [3.0, 7.0, 2.0], "w"),
        b([13.0, 0.0, -1.0], [15.0, 7.0, 2.0], "w"),
        b([3.0, 7.0, -1.0], [5.0, 9.5, 2.0], "w"),
        b([11.0, 7.0, -1.0], [13.0, 9.5, 2.0], "w"),
        b([5.0, 9.5, -1.0], [11.0, 11.5, 2.0], "w"),
    ])
}

/// A tapering stone post's foot (Spargus's plaza stones), along its rod.
fn spargus_post() -> String {
    model(&[
        b([3.0, 0.0, 3.0], [13.0, 5.0, 13.0], "p"),
        b([3.5, 5.0, 3.5], [12.5, 11.0, 12.5], "p"),
        b([4.0, 11.0, 4.0], [12.0, 16.0, 12.0], "p"),
    ])
}

/// Its point, a stick tied to it.
fn spargus_post_top() -> String {
    model(&[
        b([4.5, 0.0, 4.5], [11.5, 5.0, 11.5], "p"),
        b([5.5, 5.0, 5.5], [10.5, 9.0, 10.5], "p"),
        b([6.5, 9.0, 6.5], [9.5, 13.0, 9.5], "p"),
        b([7.3, 13.0, 7.3], [8.7, 16.0, 8.7], "p"),
        B { from: [9.5, 4.0, 7.5], to: [10.5, 16.0, 8.5], tex: "w", turn: Some(('z', -22.5, [10.0, 4.0, 8.0])), ends: None, bottom:None },
    ])
}

/// A quarter of a round tower two blocks across, its middle at the
/// block's back right corner.
fn spargus_tower() -> String {
    model(&[
        b([8.3, 0.0, 0.0], [16.0, 16.0, 4.0], "s"),
        b([3.5, 0.0, 4.0], [16.0, 16.0, 8.0], "s"),
        b([1.2, 0.0, 8.0], [16.0, 16.0, 12.0], "s"),
        b([0.0, 0.0, 12.0], [16.0, 16.0, 16.0], "s"),
    ])
}

/// A round tower's corner, four blocks across, hollow: a quarter of its
/// ring of stucco, six sixteenths thick, the tower's middle a block past
/// the block's back right corner, laid in steps of two sixteenths (small
/// voxels, as Teardown's: Adrii 23/10).
fn spargus_tower_curve() -> String {
    let (outer, inner) = (32.0f32, 26.0f32);
    let mut boxes = Vec::new();
    let mut z = 0.0f32;
    while z < 16.0 {
        let t = 32.0 - (z + 1.0);
        if t < outer {
            let x0 = (32.0 - (outer * outer - t * t).sqrt()).clamp(0.0, 16.0);
            let x1 = if t < inner { (32.0 - (inner * inner - t * t).sqrt()).clamp(0.0, 16.0) } else { 16.0 };
            let x0 = (x0 / 2.0).round() * 2.0;
            let x1 = (x1 / 2.0).round() * 2.0;
            if x1 > x0 {
                boxes.push(b([x0, 0.0, z], [x1, 16.0, z + 2.0], "s"));
            }
        }
        z += 2.0;
    }
    model(&boxes)
}

// Haven's slums' parts, as `ctysluma`'s houses stand in Jak 2.

/// A ground floor's foot, flaring out at the street.
fn haven_flare() -> String {
    model(&[
        b([0.0, 0.0, 0.0], [16.0, 16.0, 16.0], "s"),
        b([0.0, 0.0, -3.0], [16.0, 4.0, 0.0], "s"),
        b([0.0, 4.0, -1.5], [16.0, 8.0, 0.0], "s"),
    ])
}

/// A wall with its storey's beam across its foot (the plaster over the
/// beam, as `city-slum-building-frame` draws it).
fn haven_beam_wall() -> String {
    model(&[b([0.0, 0.0, 0.0], [16.0, 16.0, 16.0], "f"), b([0.0, 0.0, -1.5], [16.0, 4.0, 0.0], "w")])
}

/// Plaster with a wooden post up its front left edge.
fn haven_corner_post() -> String {
    model(&[b([0.0, 0.0, 0.0], [16.0, 16.0, 16.0], "s"), b([-1.5, 0.0, -1.5], [2.5, 16.0, 2.5], "w")])
}

/// A small square window through the wall in a wooden frame with its
/// cross, plaster about it (seen through).
fn haven_small_window() -> String {
    model(&[
        b([0.0, 0.0, 0.0], [4.0, 16.0, 16.0], "s"),
        b([12.0, 0.0, 0.0], [16.0, 16.0, 16.0], "s"),
        b([4.0, 0.0, 0.0], [12.0, 4.0, 16.0], "s"),
        b([4.0, 12.0, 0.0], [12.0, 16.0, 16.0], "s"),
        b([3.0, 3.0, -1.5], [13.0, 4.5, 2.0], "w"),
        b([3.5, 11.5, -1.0], [12.5, 12.5, 2.0], "w"),
        b([3.5, 4.5, -1.0], [4.5, 11.5, 2.0], "w"),
        b([11.5, 4.5, -1.0], [12.5, 11.5, 2.0], "w"),
        b([7.5, 4.5, 0.5], [8.5, 11.5, 1.5], "w"),
        b([4.5, 7.5, 0.5], [11.5, 8.5, 1.5], "w"),
    ])
}

// Jak 1's: Snowy Mountain's fort (`snow`) and Boggy Billy's huts (`swamp`).

/// Three stakes of a palisade side by side, a rope round them.
fn stakes(tex: &'static str, rope: &'static str) -> String {
    let mut boxes = Vec::new();
    for i in 0..3 {
        let x0 = i as f32 * 16.0 / 3.0 + 0.3;
        let x1 = (i + 1) as f32 * 16.0 / 3.0 - 0.3;
        boxes.push(b([x0, 0.0, 5.0], [x1, 16.0, 11.0], tex));
    }
    boxes.push(b([0.0, 9.0, 4.5], [16.0, 11.0, 11.5], rope));
    model(&boxes)
}

/// Their sharpened tops.
fn stake_points(tex: &'static str, tip: &'static str) -> String {
    let mut boxes = Vec::new();
    for i in 0..3 {
        let x0 = i as f32 * 16.0 / 3.0 + 0.3;
        let x1 = (i + 1) as f32 * 16.0 / 3.0 - 0.3;
        // Each a little taller than the last but one, as they stand.
        let top = [16.0f32, 13.5, 15.0][i];
        let s = top / 16.0;
        boxes.push(b([x0, 0.0, 5.0], [x1, 8.0 * s, 11.0], tex));
        boxes.push(b([x0 + 0.8, 8.0 * s, 5.8], [x1 - 0.8, 11.0 * s, 10.2], tip));
        boxes.push(b([x0 + 1.6, 11.0 * s, 6.6], [x1 - 1.6, 14.0 * s, 9.4], tip));
        boxes.push(b([x0 + 2.1, 14.0 * s, 7.4], [x1 - 2.1, top, 8.6], tip));
    }
    model(&boxes)
}

fn snow_stakes() -> String {
    stakes("l", "r")
}

fn snow_stake_points() -> String {
    stake_points("l", "l")
}

fn swamp_stakes() -> String {
    stakes("l", "r")
}

fn swamp_stake_points() -> String {
    stake_points("l", "t")
}

/// A wicker panel of the fort's, riveted metal strips up its sides and
/// across it.
fn snow_panel() -> String {
    model(&[
        b([0.0, 0.0, 0.0], [16.0, 16.0, 16.0], "p"),
        b([-0.5, 0.0, -0.8], [2.0, 16.0, 1.0], "t"),
        b([14.0, 0.0, -0.8], [16.5, 16.0, 1.0], "t"),
        b([0.0, 7.0, -0.6], [16.0, 9.0, 0.0], "t"),
    ])
}

/// A thick cap of snow, rounded, over a tower's or a roof's top.
fn snow_cap() -> String {
    model(&[
        b([-1.0, 0.0, -1.0], [17.0, 4.0, 17.0], "n"),
        b([0.0, 4.0, 0.0], [16.0, 7.0, 16.0], "n"),
        b([2.0, 7.0, 2.0], [14.0, 9.0, 14.0], "n"),
        b([4.0, 9.0, 4.0], [12.0, 10.0, 12.0], "n"),
    ])
}

/// A pole with a rope bound round it, along its rod.
fn tied_pole() -> String {
    model(&[b([5.0, 0.0, 5.0], [11.0, 16.0, 11.0], "l"), b([4.5, 10.0, 4.5], [11.5, 12.0, 11.5], "r")])
}

/// A roof's slope of Jak 1's round barrel tiles (Adrii 23/10: "las
/// redondas como los tejados de jak 1"), in small voxels: eight courses
/// rising toward +x (a stair's way up as vanilla draws it facing east),
/// each course's four tiles rounded over the one under it.
fn round_tiles() -> String {
    let mut boxes = Vec::new();
    for i in 0..8 {
        let (x0, x1) = (i as f32 * 2.0, i as f32 * 2.0 + 2.0);
        let top = x1;
        boxes.push(b([x0, 0.0, 0.0], [x1, top, 16.0], "b"));
        for c in 0..4 {
            let z = c as f32 * 4.0;
            boxes.push(b([x0 - 0.5, top, z + 0.4], [x1, top + 1.2, z + 3.6], "t"));
            boxes.push(b([x0 - 0.5, top + 1.2, z + 1.1], [x1, top + 2.0, z + 2.9], "t"));
        }
    }
    model(&boxes)
}

/// A roof's ridge: a slab with a row of round tiles laid along it.
fn round_ridge() -> String {
    model(&[
        b([0.0, 0.0, 0.0], [16.0, 8.0, 16.0], "b"),
        b([0.0, 8.0, 4.0], [16.0, 11.0, 12.0], "t"),
        b([0.0, 11.0, 5.5], [16.0, 12.5, 10.5], "t"),
        b([0.0, 12.5, 7.0], [16.0, 13.2, 9.0], "t"),
    ])
}

/// A big pipe down a facade, along its rod, a collar round it.
fn big_pipe() -> String {
    model(&[
        b([4.0, 0.0, 5.0], [12.0, 16.0, 11.0], "p"),
        b([5.0, 0.0, 4.0], [11.0, 16.0, 12.0], "p"),
        b([3.5, 6.0, 4.5], [12.5, 8.0, 11.5], "c"),
        b([4.5, 6.0, 3.5], [11.5, 8.0, 12.5], "c"),
    ])
}

/// The big pipe coming down and turning into the wall behind, its flange
/// against the wall.
fn pipe_elbow() -> String {
    model(&[
        b([4.0, 8.0, 5.0], [12.0, 16.0, 11.0], "p"),
        b([5.0, 8.0, 4.0], [11.0, 16.0, 12.0], "p"),
        b([4.0, 3.0, 4.0], [12.0, 11.0, 16.0], "p"),
        b([5.0, 2.0, 5.0], [11.0, 12.0, 16.0], "p"),
        b([3.0, 1.5, 14.5], [13.0, 12.5, 16.0], "c"),
    ])
}

/// Plaster with a wooden post up its middle, standing out.
fn haven_mid_post() -> String {
    model(&[b([0.0, 0.0, 0.0], [16.0, 16.0, 16.0], "s"), b([6.0, 0.0, -1.5], [10.0, 16.0, 0.0], "w")])
}

/// A beam's end sticking out of a wall's top, a brace under it back to
/// the wall (an eave's or a jutting floor's).
fn haven_bracket() -> String {
    model(&[
        b([0.0, 0.0, 0.0], [16.0, 16.0, 16.0], "s"),
        b([6.0, 12.0, -6.0], [10.0, 16.0, 0.0], "w"),
        B { from: [6.5, 4.0, -5.0], to: [9.5, 7.0, 2.0], tex: "w", turn: Some(('x', 45.0, [8.0, 8.0, -1.5])), ends: None, bottom:None },
    ])
}

/// Boggy Billy's hut's wall: four logs laid across, their ends out.
fn swamp_logs() -> String {
    let mut boxes = Vec::new();
    for i in 0..4 {
        let (y0, y1) = (i as f32 * 4.0, (i + 1) as f32 * 4.0);
        let inset = if i % 2 == 0 { 1.5 } else { 2.0 };
        boxes.push(B { from: [-0.5, y0, inset], to: [16.5, y1, 16.0 - inset], tex: "h", turn: None, ends: Some("c"), bottom:None });
    }
    boxes.push(b([0.0, 0.0, 2.0], [16.0, 16.0, 14.0], "h"));
    model(&boxes)
}

// ---------------------------------------------------------------- blocks

/// The parts: their blocks' looks.
pub const FACADE_BLOCKS: [JakBlock; 51] = [
    JakBlock { path: "spargus_ventana", look: BlockLook::Shaped { vanilla: "carved_pumpkin", model: spargus_window, textures: &[("s", "wascity-stucco-wall-bleached-01"), ("w", "wascity-wood-plain"), ("g", "wascity-stain-window-01")] } },
    JakBlock { path: "spargus_esquina", look: BlockLook::Shaped { vanilla: "carved_pumpkin", model: spargus_corner, textures: &[("s", "wascity-stucco-wall-bleached-01")] } },
    JakBlock { path: "spargus_contrafuerte", look: BlockLook::Shaped { vanilla: "carved_pumpkin", model: spargus_buttress, textures: &[("s", "wascity-stucco-wall-bleached-01"), ("p", "wascity-stucco-wall-supports")] } },
    JakBlock { path: "spargus_pinchos", look: BlockLook::Shaped { vanilla: "carved_pumpkin", model: spargus_spikes, textures: &[("s", "wascity-stucco-wall-bleached-01"), ("k", "wascity-wallspike-01")] } },
    JakBlock { path: "spargus_viga", look: BlockLook::Shaped { vanilla: "lightning_rod", model: beam, textures: &[("w", "wascity-wood-plain")] } },
    JakBlock { path: "spargus_bidon", look: BlockLook::Shaped { vanilla: "carved_pumpkin", model: canister, textures: &[("c", "wascity-wall-canister"), ("w", "wascity-metal-pole")] } },
    JakBlock { path: "spargus_cupula", look: BlockLook::Shaped { vanilla: "carved_pumpkin", model: dome_quarter, textures: &[("r", "wascity-roof-1")] } },
    JakBlock { path: "spargus_toldo", look: BlockLook::Like { vanilla: "oak_trapdoor", textures: &[("*", "wascity-metal-segments")] } },
    JakBlock { path: "spargus_puerta_metal", look: BlockLook::Like { vanilla: "oak_door", textures: &[("*", "wascity-metal-door-01")] } },
    JakBlock { path: "spargus_tuberia", look: BlockLook::Shaped { vanilla: "lightning_rod", model: pipe, textures: &[("w", "wascity-metal-pole")] } },
    JakBlock { path: "haven_tablas", look: BlockLook::Shaped { vanilla: "carved_pumpkin", model: haven_boards, textures: &[("d", "city-dirtywood"), ("p", "city-slum-wood-plain")] } },
    JakBlock { path: "haven_chapa", look: BlockLook::Shaped { vanilla: "carved_pumpkin", model: haven_metal, textures: &[("m", "city-slumwall-metalsiding-03")] } },
    JakBlock { path: "haven_ventana", look: BlockLook::Shaped { vanilla: "carved_pumpkin", model: haven_window, textures: &[("s", "city-slumbase-wall"), ("g", "city-slum-stain-window-01"), ("p", "city-slum-wood-plain")] } },
    JakBlock { path: "haven_base", look: BlockLook::Shaped { vanilla: "carved_pumpkin", model: haven_base, textures: &[("s", "city-slumbase-wall"), ("k", "city-slum-stonewall-bricks")] } },
    JakBlock { path: "haven_muro", look: BlockLook::All("city-slumwall-01") },
    JakBlock { path: "haven_puerta", look: BlockLook::Like { vanilla: "oak_door", textures: &[("*", "city-slum-door-01")] } },
    JakBlock { path: "haven_tejado", look: BlockLook::Like { vanilla: "oak_stairs", textures: &[("*", "city-slum-roof-1")] } },
    JakBlock { path: "haven_toldo", look: BlockLook::Like { vanilla: "oak_trapdoor", textures: &[("*", "city-slum-awning-rustedmetal")] } },
    JakBlock { path: "haven_poste", look: BlockLook::Shaped { vanilla: "lightning_rod", model: beam, textures: &[("w", "city-dirtywood")] } },
    JakBlock { path: "haven_tuberia", look: BlockLook::Shaped { vanilla: "lightning_rod", model: pipe, textures: &[("w", "city-slum-medpipe-01")] } },
    // Spargus's, from `wascitya` (23/10, second round).
    JakBlock { path: "spargus_zocalo", look: BlockLook::Shaped { vanilla: "carved_pumpkin", model: spargus_plinth, textures: &[("z", "wascitya-stone-bottom")] } },
    JakBlock { path: "spargus_remate", look: BlockLook::Shaped { vanilla: "carved_pumpkin", model: spargus_cap, textures: &[("s", "wascity-stucco-wall-bleached-01")] } },
    JakBlock { path: "spargus_ventanuco", look: BlockLook::Shaped { vanilla: "carved_pumpkin", model: spargus_slit, textures: &[("s", "wascity-stucco-wall-bleached-01"), ("n", "common-black")] } },
    JakBlock { path: "spargus_contraventana", look: BlockLook::Shaped { vanilla: "carved_pumpkin", model: spargus_shutter, textures: &[("s", "wascity-stucco-wall-bleached-01"), ("w", "wascity-wood-plain"), ("m", "wascity-metal-segments"), ("t", "wascity-metal-spike-01")] } },
    JakBlock { path: "spargus_arco", look: BlockLook::Shaped { vanilla: "carved_pumpkin", model: spargus_arch, textures: &[("s", "wascity-stucco-wall-bleached-01"), ("w", "wascity-wood-plain")] } },
    JakBlock { path: "spargus_pilar", look: BlockLook::Shaped { vanilla: "lightning_rod", model: spargus_post, textures: &[("p", "wascity-stucco-wall-supports")] } },
    JakBlock { path: "spargus_pilar_punta", look: BlockLook::Shaped { vanilla: "lightning_rod", model: spargus_post_top, textures: &[("p", "wascity-stucco-wall-supports"), ("w", "wascity-wood-plain")] } },
    JakBlock { path: "spargus_torre", look: BlockLook::Shaped { vanilla: "carved_pumpkin", model: spargus_tower, textures: &[("s", "wascity-stucco-wall-bleached-01")] } },
    JakBlock { path: "spargus_torre_curva", look: BlockLook::Shaped { vanilla: "carved_pumpkin", model: spargus_tower_curve, textures: &[("s", "wascity-stucco-wall-bleached-01")] } },
    // Haven's, from `ctysluma`.
    JakBlock { path: "haven_talud", look: BlockLook::Shaped { vanilla: "carved_pumpkin", model: haven_flare, textures: &[("s", "city-slumbase-wall")] } },
    JakBlock { path: "haven_viga_muro", look: BlockLook::Shaped { vanilla: "carved_pumpkin", model: haven_beam_wall, textures: &[("f", "city-slum-building-frame"), ("w", "city-dirtywood")] } },
    JakBlock { path: "haven_esquinero", look: BlockLook::Shaped { vanilla: "carved_pumpkin", model: haven_corner_post, textures: &[("s", "city-slumwall-06"), ("w", "city-dirtywood")] } },
    JakBlock { path: "haven_ventanita", look: BlockLook::Shaped { vanilla: "carved_pumpkin", model: haven_small_window, textures: &[("s", "city-slumwall-06"), ("w", "city-dirtywood"), ("g", "city-slum-stain-window-01")] } },
    JakBlock { path: "haven_alero", look: BlockLook::Like { vanilla: "spruce_trapdoor", textures: &[("*", "city-dirtywood")] } },
    JakBlock { path: "haven_yeso", look: BlockLook::All("city-slumwall-06") },
    // Jak 1's: Snowy Mountain's fort and Boggy Billy's huts.
    JakBlock { path: "nieve_estacas", look: BlockLook::Shaped { vanilla: "carved_pumpkin", model: snow_stakes, textures: &[("l", "snow-woodpole"), ("r", "snow-rope-snow")] } },
    JakBlock { path: "nieve_estacas_punta", look: BlockLook::Shaped { vanilla: "carved_pumpkin", model: snow_stake_points, textures: &[("l", "snow-woodpole")] } },
    JakBlock { path: "nieve_panel", look: BlockLook::Shaped { vanilla: "carved_pumpkin", model: snow_panel, textures: &[("p", "snow-woodpanels"), ("t", "snow-fortmetaltrim")] } },
    JakBlock { path: "nieve_capa", look: BlockLook::Shaped { vanilla: "smooth_quartz_slab", model: snow_cap, textures: &[("n", "snow-03")] } },
    JakBlock { path: "nieve_tronco", look: BlockLook::Shaped { vanilla: "lightning_rod", model: tied_pole, textures: &[("l", "snow-woodpole"), ("r", "snow-rope-snow")] } },
    JakBlock { path: "pantano_estacas", look: BlockLook::Shaped { vanilla: "carved_pumpkin", model: swamp_stakes, textures: &[("l", "swp-bark"), ("r", "swp-rope-01")] } },
    JakBlock { path: "pantano_estacas_punta", look: BlockLook::Shaped { vanilla: "carved_pumpkin", model: swamp_stake_points, textures: &[("l", "swp-bark"), ("t", "swp-wood-spike-end")] } },
    JakBlock { path: "pantano_tronco", look: BlockLook::Shaped { vanilla: "carved_pumpkin", model: swamp_logs, textures: &[("h", "swp-hut-wood-01"), ("c", "swp-hut-wood-01endcap")] } },
    JakBlock { path: "pantano_poste", look: BlockLook::Shaped { vanilla: "lightning_rod", model: tied_pole, textures: &[("l", "swp-bark"), ("r", "swp-rope-01")] } },
    JakBlock { path: "pantano_tablon", look: BlockLook::Like { vanilla: "spruce_slab", textures: &[("*", "swp-plankwood")] } },
    JakBlock { path: "pantano_teja", look: BlockLook::Shaped { vanilla: "oak_stairs", model: round_tiles, textures: &[("t", "swp-roof-tile-01"), ("b", "swp-roof-tile-top")] } },
    JakBlock { path: "haven_tubo", look: BlockLook::Shaped { vanilla: "lightning_rod", model: big_pipe, textures: &[("p", "city-slum-bigpipe-02"), ("c", "city-slum-bigpipe-03")] } },
    JakBlock { path: "haven_codo", look: BlockLook::Shaped { vanilla: "carved_pumpkin", model: pipe_elbow, textures: &[("p", "city-slum-bigpipe-02"), ("c", "city-slum-bigpipe-03")] } },
    JakBlock { path: "haven_pilar_madera", look: BlockLook::Shaped { vanilla: "carved_pumpkin", model: haven_mid_post, textures: &[("s", "city-slumwall-06"), ("w", "city-dirtywood")] } },
    JakBlock { path: "haven_mensula", look: BlockLook::Shaped { vanilla: "carved_pumpkin", model: haven_bracket, textures: &[("s", "city-slumwall-06"), ("w", "city-dirtywood")] } },
    JakBlock { path: "pantano_cumbrera", look: BlockLook::Shaped { vanilla: "oak_slab", model: round_ridge, textures: &[("t", "swp-roof-tile-01"), ("b", "swp-roof-tile-top")] } },
];

/// Whether a block is a window part, seen through and letting light in
/// (`scene::Block::is_opaque`).
pub fn see_through(path: &str) -> bool {
    matches!(path, "spargus_ventanuco" | "spargus_contraventana" | "haven_ventanita") || crate::jak_families::see_through(path)
}

/// A shaped part's collision as it is drawn (24/10: a full block's round
/// spikes, canisters and posts stopped Jak on nothing): its model's boxes
/// (but turned ones, ornaments), cut to the block and turned as its state
/// turns it; for a wall part (a carved pumpkin's four ways) or a post
/// standing up (a lightning rod's). In blocks; `None` for any other.
pub fn shaped_collision(path: &str, facing: Option<&str>) -> Option<Vec<[f64; 6]>> {
    use std::sync::OnceLock;
    static BOXES: OnceLock<HashMap<&'static str, (bool, Vec<[f64; 6]>)>> = OnceLock::new();
    let all = BOXES.get_or_init(|| {
        let mut all = HashMap::new();
        for block in FACADE_BLOCKS.iter().chain(crate::jak_haven::HAVEN_BLOCKS.iter()).chain(crate::jak_barrio::BARRIO_BLOCKS.iter()).chain(crate::jak_spargus::SPARG_BLOCKS.iter()).chain(crate::jak_architecture::BLOCKS.iter()) {
            let BlockLook::Shaped { vanilla, model, .. } = block.look else { continue };
            let turns = match vanilla {
                "carved_pumpkin" => true,
                "lightning_rod" => false,
                _ => continue,
            };
            let Ok(serde_json::Value::Array(elements)) = serde_json::from_str::<serde_json::Value>(&format!("[{}]", model())) else { continue };
            let mut boxes = Vec::new();
            for e in &elements {
                let v=|k:&str,i:usize|e[k][i].as_f64().unwrap_or(0.0) as f32/16.0;
                let from=[v("from",0),v("from",1),v("from",2)];let to=[v("to",0),v("to",1),v("to",2)];
                let rotation=crate::model::ElementRotation::parse(e).ok().flatten();
                let axis=(0..3).max_by(|a,b|(to[*a]-from[*a]).total_cmp(&(to[*b]-from[*b]))).unwrap();
                let slices=if rotation.is_some(){((to[axis]-from[axis])*8.0).ceil().max(1.0) as usize}else{1};
                for slice in 0..slices {
                    let mut lo=from;let mut hi=to;
                    lo[axis]=from[axis]+(to[axis]-from[axis])*slice as f32/slices as f32;
                    hi[axis]=from[axis]+(to[axis]-from[axis])*(slice+1) as f32/slices as f32;
                    let mut min=[f32::INFINITY;3];let mut max=[f32::NEG_INFINITY;3];
                    for bits in 0..8 {let mut p=std::array::from_fn(|k|if bits&(1<<k)==0{lo[k]}else{hi[k]});
                        if let Some(rot)=rotation{p=rot.apply(p);}
                        for k in 0..3{min[k]=min[k].min(p[k]);max[k]=max[k].max(p[k]);}
                    }
                    if (0..3).all(|k|max[k]-min[k]>0.001){boxes.push([min[0] as f64,min[1] as f64,min[2] as f64,max[0] as f64,max[1] as f64,max[2] as f64]);}
                }
            }
            all.insert(block.path, (turns, boxes));
        }
        all
    });
    let (turns, boxes) = all.get(path)?;
    if !turns {
        let facing=facing?;
        let mut result=boxes.clone();
        if facing=="down" {for b in &mut result{*b=[b[0],1.0-b[4],1.0-b[5],b[3],1.0-b[1],1.0-b[2]];}}
        else if facing!="up" {
            let quarter=match facing{"east"=>1,"south"=>2,"west"=>3,_=>0};
            for b in &mut result {
                *b=[b[0],b[2],1.0-b[4],b[3],b[5],1.0-b[1]];
                for _ in 0..quarter{*b=[1.0-b[5],b[1],b[0],1.0-b[2],b[4],b[3]];}
            }
        }
        return Some(result);
    }
    // Turned as the block's state turns its model about the block's
    // upright middle (north as drawn, east a quarter clockwise...).
    let quarter = match facing? {
        "north" => 0,
        "east" => 1,
        "south" => 2,
        _ => 3,
    };
    Some(
        boxes
            .iter()
            .map(|&b| {
                let mut b = b;
                for _ in 0..quarter {
                    b = [1.0 - b[5], b[1], b[0], 1.0 - b[2], b[4], b[3]];
                }
                b
            })
            .collect(),
    )
}

/// Whether a block is one of the shaped parts (lit from the sides its
/// faces look to).
pub fn shaped(block: &crate::scene::Block) -> bool {
    block.id.namespace == "jak"
        && (FACADE_BLOCKS.iter().any(|b| b.path == block.id.path && matches!(b.look, BlockLook::Shaped { .. }))
            || crate::jak_families::FAMILY_BLOCKS
                .iter()
                .chain(crate::jak_haven::HAVEN_BLOCKS.iter())
                .chain(crate::jak_barrio::BARRIO_BLOCKS.iter())
                .chain(crate::jak_spargus::SPARG_BLOCKS.iter()).chain(crate::jak_architecture::BLOCKS.iter())
                .any(|b| b.path == block.id.path && matches!(b.look, BlockLook::Set { .. } | BlockLook::Shaped { .. })))
}

pub const FACADE_ITEMS: [JakItem; 51] = [
    JakItem { path: "spargus_ventana", title: "Ventana de Spargus", kind: JakKind::Block },
    JakItem { path: "spargus_esquina", title: "Esquina de estuco de Spargus", kind: JakKind::Block },
    JakItem { path: "spargus_contrafuerte", title: "Contrafuerte de Spargus", kind: JakKind::Block },
    JakItem { path: "spargus_pinchos", title: "Pinchos de muro de Spargus", kind: JakKind::Block },
    JakItem { path: "spargus_viga", title: "Viga de Spargus", kind: JakKind::Block },
    JakItem { path: "spargus_bidon", title: "Bidón de Spargus", kind: JakKind::Block },
    JakItem { path: "spargus_cupula", title: "Cuarto de cúpula de Spargus", kind: JakKind::Block },
    JakItem { path: "spargus_toldo", title: "Toldo de Spargus", kind: JakKind::Block },
    JakItem { path: "spargus_puerta_metal", title: "Puerta metálica de Spargus", kind: JakKind::Block },
    JakItem { path: "spargus_tuberia", title: "Tubería de Spargus", kind: JakKind::Block },
    JakItem { path: "haven_tablas", title: "Pared de tablas de Haven", kind: JakKind::Block },
    JakItem { path: "haven_chapa", title: "Chapa ondulada de Haven", kind: JakKind::Block },
    JakItem { path: "haven_ventana", title: "Ventana tapiada de Haven", kind: JakKind::Block },
    JakItem { path: "haven_base", title: "Base de piedra de Haven", kind: JakKind::Block },
    JakItem { path: "haven_muro", title: "Muro de Haven", kind: JakKind::Block },
    JakItem { path: "haven_puerta", title: "Puerta de Haven", kind: JakKind::Block },
    JakItem { path: "haven_tejado", title: "Tejado de chapa de Haven", kind: JakKind::Block },
    JakItem { path: "haven_toldo", title: "Toldo oxidado de Haven", kind: JakKind::Block },
    JakItem { path: "haven_poste", title: "Poste de Haven", kind: JakKind::Block },
    JakItem { path: "haven_tuberia", title: "Tubería de Haven", kind: JakKind::Block },
    JakItem { path: "spargus_zocalo", title: "Zócalo de piedra de Spargus", kind: JakKind::Block },
    JakItem { path: "spargus_remate", title: "Remate redondeado de Spargus", kind: JakKind::Block },
    JakItem { path: "spargus_ventanuco", title: "Ventanuco de Spargus", kind: JakKind::Block },
    JakItem { path: "spargus_contraventana", title: "Contraventana con toldo de Spargus", kind: JakKind::Block },
    JakItem { path: "spargus_arco", title: "Arco de puerta de Spargus", kind: JakKind::Block },
    JakItem { path: "spargus_pilar", title: "Piedra de Spargus", kind: JakKind::Block },
    JakItem { path: "spargus_pilar_punta", title: "Punta de piedra de Spargus", kind: JakKind::Block },
    JakItem { path: "spargus_torre", title: "Cuarto de torre de Spargus", kind: JakKind::Block },
    JakItem { path: "spargus_torre_curva", title: "Esquina curva de torre de Spargus", kind: JakKind::Block },
    JakItem { path: "haven_talud", title: "Talud de Haven", kind: JakKind::Block },
    JakItem { path: "haven_viga_muro", title: "Muro con viga de Haven", kind: JakKind::Block },
    JakItem { path: "haven_esquinero", title: "Esquinero de Haven", kind: JakKind::Block },
    JakItem { path: "haven_ventanita", title: "Ventanita de Haven", kind: JakKind::Block },
    JakItem { path: "haven_alero", title: "Alero de Haven", kind: JakKind::Block },
    JakItem { path: "haven_yeso", title: "Yeso de Haven", kind: JakKind::Block },
    JakItem { path: "nieve_estacas", title: "Empalizada de la Montaña Nevada", kind: JakKind::Block },
    JakItem { path: "nieve_estacas_punta", title: "Estacas afiladas de la Montaña Nevada", kind: JakKind::Block },
    JakItem { path: "nieve_panel", title: "Panel del fuerte de la Montaña Nevada", kind: JakKind::Block },
    JakItem { path: "nieve_capa", title: "Capa de nieve", kind: JakKind::Block },
    JakItem { path: "nieve_tronco", title: "Poste atado de la Montaña Nevada", kind: JakKind::Block },
    JakItem { path: "pantano_estacas", title: "Empalizada del Pantano", kind: JakKind::Block },
    JakItem { path: "pantano_estacas_punta", title: "Estacas afiladas del Pantano", kind: JakKind::Block },
    JakItem { path: "pantano_tronco", title: "Pared de troncos del Pantano", kind: JakKind::Block },
    JakItem { path: "pantano_poste", title: "Poste atado del Pantano", kind: JakKind::Block },
    JakItem { path: "pantano_tablon", title: "Tablón del Pantano", kind: JakKind::Block },
    JakItem { path: "pantano_teja", title: "Tejado de tejas redondas", kind: JakKind::Block },
    JakItem { path: "haven_tubo", title: "Tubería grande de Haven", kind: JakKind::Block },
    JakItem { path: "haven_codo", title: "Codo de tubería de Haven", kind: JakKind::Block },
    JakItem { path: "haven_pilar_madera", title: "Pilar de madera de Haven", kind: JakKind::Block },
    JakItem { path: "haven_mensula", title: "Ménsula de Haven", kind: JakKind::Block },
    JakItem { path: "pantano_cumbrera", title: "Cumbrera de tejas redondas", kind: JakKind::Block },
];

// ---------------------------------------------------------------- houses

/// A way round: 0 north (-z), 1 east, 2 south, 3 west.
pub(crate) fn dir_name(d: u8) -> &'static str {
    ["north", "east", "south", "west"][usize::from(d % 4)]
}

pub(crate) fn dir_step(d: u8) -> (i32, i32) {
    [(0, -1), (1, 0), (0, 1), (-1, 0)][usize::from(d % 4)]
}

/// A state with `{0}`..`{3}` for the house's front, right, back and left
/// ways (in the world).
pub(crate) fn leak(state: String) -> &'static str {
    static KEPT: Mutex<Vec<&'static str>> = Mutex::new(Vec::new());
    let mut kept = KEPT.lock().unwrap_or_else(|e| e.into_inner());
    if let Some(found) = kept.iter().find(|s| **s == state) {
        return found;
    }
    let state: &'static str = Box::leak(state.into_boxed_str());
    kept.push(state);
    state
}

/// The local way a ring cell of a `w` by `d` box looks out (0 front, 1
/// right, 2 back, 3 left), and whether it is a corner.
pub(crate) fn outward(x: i32, z: i32, w: i32, d: i32) -> (u8, bool) {
    let (l, r, f, k) = (x == 0, x == w - 1, z == 0, z == d - 1);
    let corner = (l || r) && (f || k);
    let way = if f && !corner {
        0
    } else if k && !corner {
        2
    } else if r && !corner {
        1
    } else if l && !corner {
        3
    } else if f && l {
        0
    } else if f && r {
        1
    } else if k && r {
        2
    } else {
        3
    };
    (way, corner)
}

pub use crate::jak_families::{haven_house, snow_fort, spargus_house, swamp_hut};

/// The cells of a `w` by `d` ring at `(x0, z0)` with the way each looks
/// out, corners apart.
pub(crate) fn ring_cells(x0: i32, z0: i32, w: i32, d: i32) -> Vec<(i32, i32, u8, bool)> {
    let mut out = Vec::new();
    for x in 0..w {
        for z in 0..d {
            if x == 0 || z == 0 || x == w - 1 || z == d - 1 {
                let (way, corner) = outward(x, z, w, d);
                out.push((x0 + x, z0 + z, way, corner));
            }
        }
    }
    out
}


// ---------------------------------------------------------------- villages

fn mix(seed: i64, a: i32, b: i32, salt: u64) -> u64 {
    let mut h = (seed as u64) ^ ((a as u32 as u64) << 32 | u64::from(b as u32)) ^ salt.wrapping_mul(0x9E37_79B9_7F4A_7C15);
    h ^= h >> 33;
    h = h.wrapping_mul(0xff51_afd7_ed55_8ccd);
    h ^= h >> 33;
    h = h.wrapping_mul(0xc4ce_b9fe_1a85_ec53);
    h ^ (h >> 33)
}

/// The way from a house at `(x, z)` toward `(px, pz)`.
fn toward((x, z): (i32, i32), (px, pz): (i32, i32)) -> u8 {
    let (dx, dz) = (px - x, pz - z);
    if dx.abs() > dz.abs() {
        if dx > 0 { 1 } else { 3 }
    } else if dz > 0 {
        2
    } else {
        0
    }
}

/// A village of Spargus's houses round a plaza at the cell's spot: the
/// plaza of its ground with the Wastelanders' totem and a bell, 4 to 6
/// houses round it facing it, streets of its ground to their doors.
pub fn spargus_village(seed: i64, cell: (i32, i32), land: &dyn Land, wasteland: &dyn Fn(i32, i32) -> bool) -> Option<Vec<Placed>> {
    let (cx, cz) = (
        cell.0 * SPARGUS_CELL + 64 + (mix(seed, cell.0, cell.1, 301) % (SPARGUS_CELL as u64 - 128)) as i32,
        cell.1 * SPARGUS_CELL + 64 + (mix(seed, cell.0, cell.1, 302) % (SPARGUS_CELL as u64 - 128)) as i32,
    );
    if mix(seed, cell.0, cell.1, 300) % 100 >= 50 {
        return None;
    }
    let r = 16;
            let probes = [(0, 0), (r, 0), (-r, 0), (0, r), (0, -r), (r, r), (-r, -r), (r, -r), (-r, r)];
    if probes.iter().any(|&(dx, dz)| land.probe_column(cx + dx, cz + dz).1 || !wasteland(cx + dx, cz + dz)) {
        return None;
    }
    let heights: Vec<i32> = probes.iter().map(|&(dx, dz)| land.probe_column(cx + dx, cz + dz).0).collect();
    if heights.iter().max()? - heights.iter().min()? > 16 {
        return None;
    }
    let y0 = land.ground(cx, cz);
    // A town on `jak_town`'s plan (Adrii 24/10: more buildings, more
    // variety, a ring road for traffic): `jak_spargus`.
    let turn = (mix(seed, cell.0, cell.1, 307) % 4) as u8;
    {let made=crate::jak_spargus::town(seed,(cx,cz),y0,turn,land);(!made.is_empty()).then_some(made)}
}

/// A street of `block` on the ground from a plaza to a house.
fn street(out: &mut Vec<Placed>, land: &dyn Land, from: (i32, i32), to: (i32, i32), block: &'static str) {
    let (dx, dz) = ((to.0 - from.0) as f32, (to.1 - from.1) as f32);
    let length = dx.hypot(dz);
    for i in 6..length as i32 - 3 {
        let t = i as f32 / length;
        let (x, z) = (from.0 + (dx * t).round() as i32, from.1 + (dz * t).round() as i32);
        for (sx, sz) in [(0, 0), (1, 0), (0, 1)] {
            let y = land.ground(x + sx, z + sz);
            out.push(((x + sx, y - 1, z + sz), block));
            out.push(((x + sx, y, z + sz), "minecraft:air"));
            out.push(((x + sx, y + 1, z + sz), "minecraft:air"));
        }
    }
}

/// One Spargus village a square this size, perhaps.
pub const SPARGUS_CELL: i32 = 288;

type Kept = HashMap<(i64, (i32, i32)), Option<Arc<Vec<Placed>>>>;

/// The Wasteland's villages' blocks over one chunk.
pub fn place_spargus(chunk: &mut Chunk, gen: &WorldGen, states: &mut HashMap<&'static str, BlockStateId>) {
    static KEPT: Mutex<Option<Kept>> = Mutex::new(None);
    let seed = gen.terrain.seed;
    let (x0, z0) = (chunk.pos.x * 16, chunk.pos.z * 16);
    let registries = gen.terrain.registries.clone();
    let cell_of = |v: i32| v.div_euclid(SPARGUS_CELL);
    for cx in cell_of(x0 - crate::jak_town::REACH)..=cell_of(x0 + crate::jak_town::REACH + 15) {
        for cz in cell_of(z0 - crate::jak_town::REACH)..=cell_of(z0 + crate::jak_town::REACH + 15) {
            let cell = (cx, cz);
            let found = KEPT.lock().ok().and_then(|k| k.as_ref().and_then(|k| k.get(&(seed, cell)).cloned()));
            let made = match found {
                Some(made) => made,
                None => {
                    let made = spargus_village(seed, cell, gen, &|x, z| crate::wasteland::is_wasteland(gen, x, z)).map(Arc::new);
                    if let Ok(mut kept) = KEPT.lock() {
                        let map = kept.get_or_insert_with(HashMap::new);
                        if map.len() > 64 {
                            if let Some(key)=map.keys().next().copied(){map.remove(&key);}
                        }
                        map.insert((seed, cell), made.clone());
                    }
                    made
                }
            };
            let Some(blocks) = made else { continue };
            crate::jak_structures::clear_city_trees(chunk,&blocks,&registries);
            for &((x, y, z), state) in blocks.iter() {
                if x < x0 || x >= x0 + 16 || z < z0 || z >= z0 + 16 || !(-63..=319).contains(&y) {
                    continue;
                }
                let id = *states.entry(state).or_insert_with(|| registries.blocks.parse_state(state).unwrap_or(BlockStateId::AIR));
                chunk.set_block_raw((x - x0) as usize, y, (z - z0) as usize, id, &registries);
                crate::jak_loot::fill(chunk, (x, y, z), state, crate::jak_loot::Game::Jak3, seed);
            }
        }
    }
}

/// The nearest Spargus village to `(x, z)` within `reach` (tests, `mc
/// spargus`): its plaza's middle on the ground.
pub fn nearest_spargus(registries: Arc<minecraftoss_core::Registries>, seed: i64, (x, z): (i32, i32), reach: i32) -> Option<(i32, i32, i32)> {
    let terrain = Arc::new(minecraftoss_generator::terrain::TerrainGenerator::overworld(registries, seed).ok()?);
    let gen = WorldGen::new(terrain).ok()?;
    let (cx, cz) = (x.div_euclid(SPARGUS_CELL), z.div_euclid(SPARGUS_CELL));
    let rings = reach / SPARGUS_CELL + 1;
    (0..=rings).find_map(|ring| {
        let mut found = Vec::new();
        for dx in -ring..=ring {
            for dz in -ring..=ring {
                if dx.abs().max(dz.abs()) != ring {
                    continue;
                }
                let cell = (cx + dx, cz + dz);
                if spargus_village(seed, cell, &gen, &|x, z| crate::wasteland::is_wasteland(&gen, x, z)).is_some() {
                    let (px, pz) = (
                        cell.0 * SPARGUS_CELL + 64 + (mix(seed, cell.0, cell.1, 301) % (SPARGUS_CELL as u64 - 128)) as i32,
                        cell.1 * SPARGUS_CELL + 64 + (mix(seed, cell.0, cell.1, 302) % (SPARGUS_CELL as u64 - 128)) as i32,
                    );
                    found.push((px, gen.terrain.base_height(px, pz, HeightmapKind::OceanFloorWg), pz));
                }
            }
        }
        found.into_iter().min_by_key(|p| (p.0 - x).pow(2) + (p.2 - z).pow(2))
    })
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

    /// Every part's model is good JSON, every house's block a state the
    /// world knows, and the houses stand with their parts: windows, corners,
    /// doors, beams, roofs.
    #[test]
    fn houses_are_made_of_their_parts() {
        for block in &FACADE_BLOCKS {
            if let BlockLook::Shaped { model, .. } = block.look {
                let json = format!("[{}]", model());
                serde_json::from_str::<serde_json::Value>(&json).unwrap_or_else(|e| panic!("{}: {e}", block.path));
            }
        }
        let Ok(paths) = minecraftoss_core::registries::DataPaths::discover() else { return };
        if !paths.block_catalog.is_file() {
            return;
        }
        let registries = minecraftoss_core::Registries::load(&paths).unwrap();
        for (name, blocks) in [
            ("spargus", spargus_house((0, 70, 0), 1, false)),
            ("spargus grande", spargus_house((0, 70, 0), 2, true)),
            ("haven", crate::jak_haven::building((0, 70, 0), 0, 0)),
            ("haven alta", crate::jak_haven::building((0, 70, 0), 3, 1)),
            ("haven balcon", crate::jak_haven::building((0, 70, 0), 1, 2)),
            ("fuerte", snow_fort((0, 70, 0), 1)),
            ("choza", swamp_hut((0, 70, 0), 2, &|_, _| 66)),
        ] {
            let mut parts: HashMap<&str, usize> = HashMap::new();
            for (_, state) in &blocks {
                assert!(registries.blocks.parse_state(state).is_ok(), "{name}: {state}");
                let id = state.split('[').next().unwrap();
                *parts.entry(id).or_default() += 1;
            }
            println!("{name}: {} blocks {:?}", blocks.len(), parts.iter().filter(|(k, _)| k.starts_with("jak:")).collect::<Vec<_>>());
            let has = |p: &str| parts.get(p).copied().unwrap_or(0) > 0;
            let all = |list: &[&str]| list.iter().all(|p| has(p));
            let whole = match name {
                "spargus" => all(&["jak:spargus_pared", "jak:spargus_pared_base", "jak:spargus_vano", "jak:sparg_portal_doble", "jak:spargus_pilar_punta", "jak:spargus_puerta_alta", "minecraft:ladder", "minecraft:spruce_trapdoor"]),
                "spargus grande" => all(&["jak:spargus_pared", "jak:spargus_pinchos", "jak:spargus_vano", "minecraft:ladder"]),
                "fuerte" => all(&["jak:nieve_empalizada", "jak:nieve_pared", "jak:nieve_pared_base", "jak:nieve_vano", "jak:nieve_capa", "jak:nieve_tronco", "jak:teja_nevada", "jak:cumbrera_nevada", "jak:snow_gate_door", "minecraft:chest"]),
                "choza" => all(&["jak:pantano_pared", "jak:pantano_vano", "jak:pantano_poste", "jak:pantano_empalizada", "jak:teja_pantano", "jak:cumbrera_pantano", "jak:swamp_hut_door", "minecraft:ladder", "minecraft:chest"]),
                "haven" => all(&["jak:haven_mamposteria_pared", "jak:haven_mamposteria_pared_base", "jak:haven_entramado_pared", "jak:haven_viga_pared", "jak:haven_barrotes_vano", "jak:haven_celosia_vano", "jak:haven_cupula_madera", "jak:haven_tejadillo", "jak:haven_estandarte", "jak:haven_brazo_farol", "jak:haven_canes", "jak:haven_chimenea_remate", "jak:teja_roja", "jak:cumbrera_roja", "jak:haven_tubo", "jak:haven_codo", "minecraft:campfire", "minecraft:chest"]),
                "haven balcon" => all(&["jak:haven_baranda", "jak:haven_toldo_tela", "jak:haven_puntal", "jak:haven_caja", "jak:haven_losa_madera"]),
                _ => all(&["jak:haven_entramado_pared", "jak:haven_estandarte", "jak:haven_tubo", "jak:teja_roja", "minecraft:ladder"]),
            };
            // The families joined: a wall's straight run without its
            // pillar, its turns with one, its top capped (low) where nothing
            // stands on it, windows joined both ways along their wall.
            let states: Vec<&str> = blocks.iter().map(|(_, s)| *s).collect();
            let walls: Vec<&&str> = states.iter().filter(|s| s.contains("_pared[") || s.contains("_empalizada[")).collect();
            assert!(walls.iter().any(|s| s.contains("up=false")), "{name}: a straight run");
            assert!(walls.iter().any(|s| s.contains("up=true")), "{name}: a turn");
            assert!(walls.iter().any(|s| s.contains("=low")) && walls.iter().any(|s| s.contains("=tall")), "{name}: capped tops");
            assert!(states.iter().filter(|s| s.contains("_vano[")).all(|s| s.matches("=true").count() >= 2), "{name}: windows joined");
            assert!(whole, "{name}: {parts:?}");
        }
        // A village on flat Wasteland.
        let village = spargus_village(1, (0, 0), &Flat(70), &|_, _| true);
        let village = (0..40).find_map(|i| spargus_village(i, (0, 0), &Flat(70), &|_, _| true)).or(village).expect("a village");
        assert!(village.len() > 1000);
    }
}


#[cfg(test)]mod finish_regressions{
 use super::*;
 #[test]fn adjacent_equal_volumes_do_not_duplicate_internal_faces(){
  let json=model(&[b([0.0,0.0,0.0],[16.0,8.0,16.0],"w"),b([0.0,8.0,0.0],[16.0,16.0,16.0],"w")]);
  let v:serde_json::Value=serde_json::from_str(&format!("[{json}]")).unwrap();assert_eq!(v.as_array().unwrap().len(),1);assert_eq!(v[0]["faces"].as_object().unwrap().len(),6);
 }
 #[test]fn tile_roofs_keep_their_shape_with_fewer_quads_and_a_wooden_underside(){
  let block=crate::jak_families::FAMILY_BLOCKS.iter().find(|b|b.path=="teja_roja").unwrap();
  let BlockLook::Set{models,..}=block.look else{panic!()};
  let v:serde_json::Value=serde_json::from_str(&format!("[{}]",models[0].1())).unwrap();
  let faces:usize=v.as_array().unwrap().iter().map(|e|e["faces"].as_object().unwrap().len()).sum();
  println!("Straight tile roof: {} elements, {faces} faces (before: 96, 576)",v.as_array().unwrap().len());
  assert!(faces<240);assert!(v.as_array().unwrap().iter().any(|e|e["faces"]["down"]["texture"]=="#u"));
  assert!(v.as_array().unwrap().iter().any(|e|e["to"][1].as_f64().unwrap()>16.0));
 }
 #[test]fn posts_collide_in_all_six_facings_and_slanted_braces_are_not_omitted(){
  for facing in ["up","down","north","east","south","west"] {
   let boxes=shaped_collision("haven_poste",Some(facing)).unwrap();assert!(!boxes.is_empty(),"{facing}");
   assert!(boxes.iter().all(|b|(0..3).all(|k|b[k]<b[k+3])));
  }
  let boxes=shaped_collision("barrio_entramado",Some("north")).unwrap();
  assert!(boxes.len()>5,"Includes the sloped brace as well as straight timbers");
  let eaves=shaped_collision("barrio_codo",Some("north")).unwrap();
  assert!(eaves.iter().any(|b|b[2]<0.0||b[5]>1.0),"The visible overhang is physical");
 }
}
