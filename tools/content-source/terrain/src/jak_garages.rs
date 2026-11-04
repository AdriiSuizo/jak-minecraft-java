//! Drive-through clearance is part of the building, not an incidental empty room.
use crate::{jak_families::Site,jak_structures::Placed};
pub fn building(origin:(i32,i32,i32),front:u8,toad:bool)->Vec<Placed> {building_on_grade(origin,front,toad,origin.1)}
pub fn building_on_grade(origin:(i32,i32,i32),front:u8,toad:bool,street_y:i32)->Vec<Placed> {
    let s=Site{origin,front,w:11,d:10};let mut out=Vec::new();
    // Foundations also support the apron on the bounded four-block grade.
    for x in 0..11 {for z in 0..10 {for y in -5..-1 {out.push((s.at(x,y,z),"jak:sparg_losa"));}}}
    for x in 0..11 {for z in 0..10 {
        out.push((s.at(x,-1,z),"jak:sparg_losa"));
        for y in 0..6 {
            let wall=x==0||x==10||z==9||(z==0&&(x<2||x>8||y>=5));
            out.push((s.at(x,y,z),if wall{"jak:spargus_stucco"}else{"minecraft:air"}));
        }
        out.push((s.at(x,6,z),"jak:palacio_suelo_panel"));
    }}
    // Solid paved apron, level with the street; no threshold under the wheels.
    let delta=(street_y-origin.1).clamp(-4,4);
    let level=|z:i32|delta*(-z)/10;
    for x in 1..10 {for z in -10..0 {
        let y=level(z);let next=level(z+1);
        for dy in -6..y {out.push((s.at(x,dy,z),"jak:sparg_losa"));}
        for dy in y..y.max(0)+5 {out.push((s.at(x,dy,z),"minecraft:air"));}
        if next!=y {
            let (height,way)=if next>y {(y,2)}else{(next,0)};
            out.push((s.at(x,height,z),s.facing("jak:palacio_suelo_piedra_escalera[facing={f},half=bottom,shape=straight,waterlogged=false]",way)));
        }
    }}
    // The parapet follows its neighbours after rotating the entire workshop.
    for x in 2..=8 {out.push((s.at(x,5,0),s.facing("jak:spargus_viga[facing={f},powered=false,waterlogged=false]",1)));}
    let perimeter=|x:i32,z:i32| (0..11).contains(&x)&&(0..10).contains(&z)&&(x==0||x==10||z==0||z==9);
    for x in 0..11 {for z in 0..10 {if perimeter(x,z) {
        let mut arms=["none";4];
        for (dx,dz,local) in [(0,-1,0),(1,0,1),(0,1,2),(-1,0,3)] {
            if perimeter(x+dx,z+dz){arms[s.way(local)as usize]="low";}
        }
        let state=crate::jak_facades::leak(format!("jak:sparg_almena_pared[north={},east={},south={},west={},up=true,waterlogged=false]",arms[0],arms[1],arms[2],arms[3]));
        out.push((s.at(x,7,z),state));
    }}}
    for x in [1,9] {for y in 0..6 {out.push((s.at(x,y,0),"jak:palacio_pared_estuco"));}
        out.push((s.at(x,4,1),"minecraft:lantern[hanging=true,waterlogged=false]"));
        out.push((s.at(x,5,1),"jak:palacio_suelo_panel"));
    }
    // Workbench, fuel drums and the original market props leave the central bay clear.
    out.push((s.at(1,0,8),"minecraft:crafting_table"));
    out.push((s.at(9,0,8),"minecraft:barrel[facing=up,open=false]"));
    out.push((s.at(9,0,7),"jak:spargus_bidon[facing=north]"));
    out.push((s.at(1,0,7),"jak:spargus_bidon[facing=north]"));
    out.push((s.at(1,0,5),s.facing("minecraft:chest[facing={f},type=single,waterlogged=false]",1)));
    out.push((s.at(5,-1,5),s.facing(if toad{"jak:garaje_salta_dunas[facing={f}]"}else{"jak:garaje_arenatiburon[facing={f}]"},0)));
    out
}

/// A one-use parking marker, consumed together with the saved owned vehicle.
pub fn arrivals(chunk:&minecraftoss_core::Chunk,r:&minecraftoss_core::Registries)->Vec<((i32,i32,i32),&'static str,f32)> {
    let mut markers=Vec::new();
    for (id,model) in [("jak:garaje_salta_dunas","toad"),("jak:garaje_arenatiburon","snake")] {
        if let Some(id)=r.blocks.block_by_name(id){for state in r.blocks.block(id).states(){
            let yaw=match r.blocks.property(state,"facing").unwrap_or("north"){"north"=>std::f32::consts::FRAC_PI_2,"south"=>-std::f32::consts::FRAC_PI_2,"west"=>std::f32::consts::PI,_=>0.};
            markers.push((state,model,yaw));
        }}
    }
    let mut out=Vec::new();
    for (sy,section) in chunk.sections().iter().enumerate(){if section.is_empty()||section.blocks.single().is_some_and(|s|!markers.iter().any(|m|m.0==s)){continue;}
        for i in 0..4096 {let state=section.blocks.get(i);if let Some((_,model,yaw))=markers.iter().find(|(id,_,_)|*id==state){
            let at=(chunk.pos.x*16+(i&15)as i32,chunk.min_y()+sy as i32*16+(i>>8)as i32,chunk.pos.z*16+((i>>4)&15)as i32);
            out.push((at,*model,*yaw));
        }}
    }out
}

#[cfg(test)]mod tests {
    use super::*;
    #[test]fn architecture26_garages_have_clear_vehicle_exit_and_registered_parts(){
        let r=minecraftoss_core::Registries::load(&minecraftoss_core::registries::DataPaths::discover().unwrap()).unwrap();
        for front in 0..4 {for toad in [false,true] {
            let s=Site{origin:(30,70,-40),front,w:11,d:10};
            let blocks:std::collections::HashMap<_,_>=building(s.origin,front,toad).into_iter().collect();
            for state in blocks.values(){r.blocks.parse_state(state).unwrap_or_else(|e|panic!("{state}: {e}"));}
            for x in 2..=8 {for z in -10..=8 {for y in 0..5 {assert_eq!(blocks.get(&s.at(x,y,z)),Some(&"minecraft:air"),"bay must clear whole car in every rotation");}}}
            assert_eq!(blocks.values().filter(|s|s.starts_with("jak:garaje_")).count(),1);
        }}
    }
    #[test]fn architecture26_original_props_keep_whole_mesh_inside_a_column(){
        for path in ["objeto_spargus_urna_alta","objeto_spargus_urna_baja","objeto_spargus_caja_mercado"] {
            let k=crate::jak_objects::kind_of(path).unwrap();let shape=crate::jak_objects::shape(k).expect(path);
            assert!(!shape.cells.is_empty());assert_eq!(shape.size.0,1,"{path}");assert_eq!(shape.size.2,1,"{path}");
            let count:usize=shape.cells.values().map(|p|p.tris.len()).sum();assert!(count>20,"{path}: complete mesh");
        }
    }
}
