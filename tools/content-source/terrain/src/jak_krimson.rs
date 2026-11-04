//! Haven's Krimzon Guard precinct: original Jak II prison materials,
//! walkable rooms and a three-panel retracting security entrance.
use crate::jak_facades::{b, model};
use crate::jak_families::{Plan, Site};
use crate::jak_items::{BlockLook, JakBlock, JakItem, JakKind};
use crate::jak_structures::Placed;

pub const POST: &str = "jak:krimson_puesto";
pub const GATE: &str = "jak:krimson_porton_fence_gate";
pub const WIDTH: i32 = 11;
pub const DEPTH: i32 = 10;

const METAL: &[(&str,&str)] = &[("m","prison-tech-walls"),("r","prison-redstripe"),("p","prison-supportmetall")];
fn armour() -> String { model(&[
    b([0.,0.,0.],[16.,16.,16.],"m"),
    b([0.,0.,-0.8],[2.,16.,0.],"p"), b([14.,0.,-0.8],[16.,16.,0.],"p"),
    b([2.,1.,-0.4],[14.,3.,0.],"p"),
]) }
fn stripe() -> String { model(&[b([0.,0.,0.],[16.,16.,16.],"m"),b([0.,5.,-0.6],[16.,11.,0.],"r")]) }
fn shutter() -> String { model(&[
    b([0.,0.,6.],[16.,48.,10.],"m"),
    b([0.,17.,5.5],[16.,23.,10.5],"r"),
    b([0.,0.,5.],[1.,48.,11.],"p"), b([15.,0.,5.],[16.,48.,11.],"p"),
]) }
fn raised() -> String { model(&[
 b([0.,48.,-2.],[16.,61.,3.],"m"), b([0.,53.,-2.5],[16.,56.,-2.],"r"),
 b([0.,48.,-3.],[16.,50.,4.],"p"), b([0.,59.,-3.],[16.,61.,4.],"p"),
 b([0.,48.,-3.],[1.,61.,4.],"p"), b([15.,48.,-3.],[16.,61.,4.],"p")]) }
fn terminal() -> String { model(&[
    b([1.,0.,1.],[15.,12.,15.],"m"), b([1.,12.,10.],[15.,16.,15.],"p"),
    b([2.,12.,2.],[14.,13.,10.],"s"),
]) }
fn insignia()->String {model(&[b([0.,0.,0.],[16.,16.,16.],"m"),b([-8.,0.,-1.5],[24.,32.,-0.5],"s")])}
fn lintel()->String {model(&[b([0.,0.,0.],[16.,16.,16.],"p"),b([-16.,0.,-1.5],[32.,16.,-0.5],"m"),b([-17.,0.,-4.],[33.,3.,-1.5],"p"),b([-17.,13.,-3.],[33.,16.,-1.5],"p")])}
const SHUTTERS: &[(&str,fn()->String)] = &[("",shutter),("_open",raised),("_wall",shutter),("_wall_open",raised)];
pub const BLOCKS: [JakBlock;10] = [
    JakBlock{path:"krimson_blindaje",look:BlockLook::Shaped{vanilla:"carved_pumpkin",model:armour,textures:METAL}},
    JakBlock{path:"krimson_franja",look:BlockLook::Shaped{vanilla:"carved_pumpkin",model:stripe,textures:METAL}},
    JakBlock{path:"krimson_suelo",look:BlockLook::All("prison-metal-floor-01")},
    JakBlock{path:"krimson_reja",look:BlockLook::Like{vanilla:"iron_bars",textures:&[("*","prison-metal-bar-01")]}},
    JakBlock{path:"krimson_porton_fence_gate",look:BlockLook::Set{vanilla:"oak_fence_gate",models:SHUTTERS,textures:METAL}},
    JakBlock{path:"krimson_terminal",look:BlockLook::Shaped{vanilla:"carved_pumpkin",model:terminal,textures:&[("m","prison-mainframe"),("p","prison-panel-01"),("s","prison-glow-panel-01")]}},
    JakBlock{path:"krimson_luz",look:BlockLook::All("prison-hall-light-01")},
    JakBlock{path:"krimson_puesto",look:BlockLook::All("prison-conc-wall")},
    JakBlock{path:"krimson_insignia",look:BlockLook::Shaped{vanilla:"carved_pumpkin",model:insignia,textures:&[("m","prison-tech-walls"),("s","fort-tank-beige-logo")]}},
    JakBlock{path:"krimson_dintel",look:BlockLook::Shaped{vanilla:"carved_pumpkin",model:lintel,textures:&[("m","prison-door-frame-01"),("p","prison-supportmetall")]}},
];
pub const ITEMS: [JakItem;10] = [
    JakItem{path:"krimson_blindaje",title:"Blindaje de la Guardia Krimson",kind:JakKind::Block},
    JakItem{path:"krimson_franja",title:"Franja de la Guardia Krimson",kind:JakKind::Block},
    JakItem{path:"krimson_suelo",title:"Suelo de la comisaría Krimson",kind:JakKind::Block},
    JakItem{path:"krimson_reja",title:"Reja de la prisión de Jak II",kind:JakKind::Block},
    JakItem{path:"krimson_porton_fence_gate",title:"Portón elevable Krimson",kind:JakKind::Block},
    JakItem{path:"krimson_terminal",title:"Terminal de control Krimson",kind:JakKind::Block},
    JakItem{path:"krimson_luz",title:"Luz de la comisaría Krimson",kind:JakKind::Block},
    JakItem{path:"krimson_puesto",title:"Puesto de guardia Krimson",kind:JakKind::Block},
    JakItem{path:"krimson_insignia",title:"Emblema original de la Guardia Krimson",kind:JakKind::Block},
    JakItem{path:"krimson_dintel",title:"Marco blindado del portón Krimson",kind:JakKind::Block},
];

/// Two floors with three clear blocks each. The main corridor and the
/// raised portcullis leave three whole blocks for the original guard body.
pub fn building(origin:(i32,i32,i32),front:u8)->Vec<Placed> {
    let s=Site{origin,front,w:WIDTH,d:DEPTH};let mut p=Plan::default();
    for x in 0..WIDTH {for z in 0..DEPTH {
        for y in -1..=8 {
            let edge=x==0||x==WIDTH-1||z==0||z==DEPTH-1;
            let state=if y==-1||y==3||y==7 {"jak:krimson_suelo"}
                else if edge && y<8 {if y==2||y==6 {"jak:krimson_franja"}else{"jak:krimson_blindaje"}}
                else {"minecraft:air"};
            p.fixed(s.at(x,y,z),if edge&&y>=0&&y!=3&&y!=7&&y<8 {
                let way=if z==0{0}else if x==WIDTH-1{1}else if z==DEPTH-1{2}else{3};
                s.facing(&format!("{state}[facing={{f}}]"),way)
            }else{state});
        }
    }}
    // Recessed windows, prison bars and red armour continue around all faces.
    for (x,z,way) in [(2,0,0),(8,0,0),(0,3,3),(10,3,1),(0,7,3),(10,7,1),(3,9,2),(7,9,2)] {
        for y in [1,5] {p.fixed(s.at(x,y,z),s.facing("jak:krimson_reja[east=false,north=false,south=false,waterlogged=false,west=false]",way));}
    }
    // Paved forecourt, flush with the street. Three panels operate together.
    for x in 1..10 {for z in -3..0 {p.fixed(s.at(x,-1,z),"jak:krimson_suelo");for y in 0..3 {p.fixed(s.at(x,y,z),"minecraft:air");}}}
    for x in 4..=6 {
        p.fixed(s.at(x,0,0),s.facing("jak:krimson_porton_fence_gate[facing={f},in_wall=false,open=false,powered=false]",0));
        for y in 1..3{p.fixed(s.at(x,y,0),"minecraft:air");}
    }
    for x in [3,7] {for y in 0..3{p.fixed(s.at(x,y,0),s.facing("jak:krimson_blindaje[facing={f}]",0));}}
    p.fixed(s.at(5,3,0),s.facing("jak:krimson_dintel[facing={f}]",0));
    p.fixed(s.at(5,5,0),s.facing("jak:krimson_insignia[facing={f}]",0));
    for x in [0,1,9,10] {for y in 0..if x==0||x==10{6}else{2} {
        p.fixed(s.at(x,y,-1),s.facing(if y==1||y==5{"jak:krimson_franja[facing={f}]"}else{"jak:krimson_blindaje[facing={f}]"},0));
    }}
    for x in [2,8] {p.fixed(s.at(x,-2,-3),s.facing("jak:krimson_puesto[facing={f}]",0));}
    // A two-block wide half-step stair, with real collision and a full
    // headroom opening through the intermediate floor, to the duty office.
    for z in 1..9 {let rise=(z-1)/2;let half=if z%2==0{"top"}else{"bottom"};
        for x in 1..=2 {for y in 0..=rise {p.fixed(s.at(x,y,z),if y<rise{"jak:krimson_suelo"}else{
            crate::jak_facades::leak(format!("jak:barrio_tablon_slab[type={half},waterlogged=false]"))});}
            for y in rise+1..=6 {p.fixed(s.at(x,y,z),"minecraft:air");}
        }
    }
    // Ground floor: reception/control desk and a three-block-wide passage
    // into the holding cell. Nothing projects into the guard's entrance.
    for x in 7..=9 {p.fixed(s.at(x,0,3),s.facing("jak:krimson_terminal[facing={f}]",0));}
    for x in 4..=9 {for y in 0..3 {if !(5..=7).contains(&x){p.fixed(s.at(x,y,6),"jak:krimson_reja[east=true,north=false,south=false,waterlogged=false,west=true]");}}}
    for x in 5..=7 {p.fixed(s.at(x,0,6),s.facing("jak:krimson_porton_fence_gate[facing={f},in_wall=false,open=false,powered=false]",0));}
    for x in [4,8] {for y in 0..3{p.fixed(s.at(x,y,6),s.facing("jak:krimson_blindaje[facing={f}]",0));}}
    p.fixed(s.at(8,0,7),s.facing("minecraft:red_bed[facing={f},occupied=false,part=foot]",2));
    p.fixed(s.at(8,0,8),s.facing("minecraft:red_bed[facing={f},occupied=false,part=head]",2));
    // Duty office, original control terminals, beds and functional armory
    // chests. Their normal Minecraft inventory and Jak loot are preserved.
    for x in [4,5,6] {p.fixed(s.at(x,4,8),s.facing("jak:krimson_terminal[facing={f}]",2));}
    for x in [8,9] {p.fixed(s.at(x,4,6),s.facing("minecraft:chest[facing={f},type=single,waterlogged=false]",3));}
    for z in [2,5] {
        p.fixed(s.at(8,4,z),s.facing("minecraft:red_bed[facing={f},occupied=false,part=foot]",1));
        p.fixed(s.at(9,4,z),s.facing("minecraft:red_bed[facing={f},occupied=false,part=head]",1));
    }
    // A broad ladder reaches the roof patrol terrace. The battlements and
    // raised corner shields leave the middle open to walk around.
    for y in 4..8 {p.fixed(s.at(5,y,1),s.facing("minecraft:ladder[facing={f},waterlogged=false]",2));}
    p.fixed(s.at(5,7,1),s.facing("minecraft:oak_trapdoor[facing={f},half=top,open=false,powered=false,waterlogged=false]",2));
    for x in 0..WIDTH {for z in 0..DEPTH {if x==0||x==10||z==0||z==9 {
        p.fixed(s.at(x,8,z),"jak:krimson_reja[east=true,north=true,south=true,waterlogged=false,west=true]");
        if (x==0||x==10)&&(z==0||z==9){for y in 8..11{p.fixed(s.at(x,y,z),s.facing("jak:krimson_franja[facing={f}]",if z==0{0}else{2}));}}
    }}}
    for (x,z) in [(3,1),(7,1),(4,8),(9,8)] {for y in [2,6]{p.fixed(s.at(x,y,z),"minecraft:lantern[hanging=true,waterlogged=false]");}}
    for (x,z) in [(4,2),(7,2),(4,7),(7,7)] {for y in [3,7]{p.fixed(s.at(x,y,z),"jak:krimson_luz");}}
    p.finish()
}

#[cfg(test)] mod tests {
    use super::*;
    #[test] fn precinct_materials_come_from_the_original_jak2_prison(){
        for texture in ["prison-tech-walls","prison-redstripe","prison-supportmetall","prison-metal-floor-01","prison-metal-bar-01","prison-mainframe","prison-panel-01","prison-glow-panel-01","prison-hall-light-01","prison-conc-wall","fort-tank-beige-logo","prison-door-frame-01"] {
            let (w,h,pixels)=crate::original_places::embedded_texture(texture).unwrap_or_else(||panic!("missing original texture {texture}"));
            assert!(w>0&&h>0);assert_eq!(pixels.len(),(w*h*4) as usize);
        }
    }
    #[test] fn precinct_is_generated_on_a_whole_haven_plot_in_all_town_rotations(){
        struct Flat;impl crate::jak_structures::Land for Flat{fn ground(&self,_:i32,_:i32)->i32{70}fn wet(&self,_:i32,_:i32)->bool{false}}
        for front in 0..4 {let made=crate::jak_barrio::village(12345,(100,200),70,front,&Flat);
            assert_eq!(made.iter().filter(|(_,s)|s.starts_with(POST)).count(),2,"rotation {front}");
            assert_eq!(made.iter().filter(|(_,s)|s.starts_with(GATE)).count(),6,"both complete precinct gates remain in the city core");
        }
    }
    #[test] fn precinct_has_full_corridors_rooms_and_three_panel_doors_in_every_rotation(){
        for front in 0..4 {let origin=(-17,70,31);let s=Site{origin,front,w:WIDTH,d:DEPTH};let made=building(origin,front);
            let cells:std::collections::HashMap<_,_>=made.into_iter().collect();
            for z in -2..6 {for x in 4..=6 {for y in 0..3 {let b=cells[&s.at(x,y,z)];assert!(b=="minecraft:air" || z==0&&y==0&&b.starts_with(GATE),"{front}: {x},{y},{z}: {b}");}}}
            assert_eq!(cells.values().filter(|b|b.starts_with(GATE)).count(),6);
            assert!(cells.values().any(|b|b.starts_with("minecraft:chest")));
            assert_eq!(cells.values().filter(|b|b.starts_with(POST)).count(),2);
            for z in 1..9{for x in 1..=2{let rise=(z-1)/2;for y in rise+1..=6{assert_eq!(cells[&s.at(x,y,z)],"minecraft:air");}}}
        }
    }
}
