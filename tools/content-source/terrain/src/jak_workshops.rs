//! Small street-facing work alcoves, kept on the same grid as the homes.
use crate::jak_families::{Plan,Site};
pub const PROFESSIONS:[&str;13]=["farmer","fisherman","shepherd","fletcher","librarian","cartographer","cleric","armorer","weaponsmith","toolsmith","butcher","leatherworker","mason"];
pub fn home_flavor(head:[i32;3])->u32 {(head[0]as u32).wrapping_mul(0x9e3779b9)^(head[2]as u32).rotate_left(16)^(head[1]as u32).wrapping_mul(7919)}
pub fn profession(flavor:u32)->&'static str {PROFESSIONS[(flavor/3)as usize%PROFESSIONS.len()]}
fn station(head:[i32;3])->&'static str {
    match profession(home_flavor(head)) {
        "farmer"=>"minecraft:composter[level=0]",
        "fisherman"=>"minecraft:barrel[facing=up,open=false]",
        "shepherd"=>"minecraft:loom[facing={f}]",
        "fletcher"=>"minecraft:fletching_table",
        "librarian"=>"minecraft:lectern[facing={f},has_book=false,powered=false]",
        "cartographer"=>"minecraft:cartography_table",
        "cleric"=>"minecraft:brewing_stand[has_bottle_0=false,has_bottle_1=false,has_bottle_2=false]",
        "armorer"=>"minecraft:blast_furnace[facing={f},lit=false]",
        "weaponsmith"=>"minecraft:grindstone[face=floor,facing={f}]",
        "toolsmith"=>"minecraft:smithing_table",
        "butcher"=>"minecraft:smoker[facing={f},lit=false]",
        "leatherworker"=>"minecraft:cauldron",
        _=>"minecraft:stonecutter[facing={f}]",
    }
}
pub(crate) fn add(plan:&mut Plan,s:&Site,second_bed:bool,spargus:bool) {
    let mut owners=vec![(s.at(s.w-2,0,s.d-2),0,0)];
    if second_bed {owners.push((s.at(2,0,1),s.w-1,0));}
    for ((x,y,z),side,way) in owners {
        let at=s.at(side,0,-1);let roof=s.at(side,3,-1);let state=s.facing(station([x,y,z]),way);
        plan.fixed((at.0,at.1-1,at.2),if spargus{"jak:sparg_losa"}else{"jak:barrio_suelo"});
        plan.fixed(at,state);
        // Each canopy is anchored to the house wall; the lantern hangs below
        // it, beyond the entrance and the original ladder/bed circulation.
        plan.fixed(roof,if spargus{"minecraft:smooth_sandstone_slab[type=bottom,waterlogged=false]"}else{"minecraft:dark_oak_slab[type=bottom,waterlogged=false]"});
        plan.fixed((roof.0,roof.1-1,roof.2),"minecraft:lantern[hanging=true,waterlogged=false]");
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use minecraftoss_entities::{poi::PoiType,villager::Profession};
    #[test] fn generated_workstation_uses_the_bed_owners_native_profession_in_every_orientation() {
        for front in 0..4 {for x in -20..20 {
            let s=Site{origin:(x,64,-37),front,w:9,d:8};let p=s.at(7,0,6);let head=[p.0,p.1,p.2];
            let name=station(head).split('[').next().unwrap();
            let kind=PoiType::of_block(&minecraftoss_player::Block::new(name)).unwrap();
            assert!(Profession::from_id(&format!("minecraft:{}",profession(home_flavor(head)))).unwrap().holds(kind));
            let mut plan=Plan::default();add(&mut plan,&s,true,false);let cells:std::collections::HashMap<_,_>=plan.finish().into_iter().collect();
            for side in [0,8] {assert!(!cells.contains_key(&s.at(side,0,0)),"the station must not replace a facade corner");let a=s.at(side,0,-1);let roof=s.at(side,3,-1);assert!(cells.contains_key(&(a.0,a.1-1,a.2)));assert!(cells[&(roof.0,roof.1-1,roof.2)].starts_with("minecraft:lantern"));}
        }}
    }
}
