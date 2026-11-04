//! Physical luminaires for generated Jak buildings. Layout is calculated once
//! with the building, using its final occupied cells, before terrain meshing.
use std::collections::{BTreeMap,HashMap};
use crate::jak_structures::Placed;
fn floor(s:&str)->bool {
 s.starts_with("jak:") && !s.contains("losa") && !s.contains("escalera") &&
 ["suelo","azotea","tejado_haven","tejado_spargus"].iter().any(|p|s.contains(p))
}
fn empty(s:Option<&&str>)->bool{s.is_none_or(|s|*s=="minecraft:air")}
/// Embed a full solid light in the floor, or a shallow fixture against a
/// nearby ceiling. Nothing floats in the room or obstructs a doorway.
pub fn illuminate(out:&mut Vec<Placed>){
 let cells:HashMap<_,_>=out.iter().copied().collect();
 let mut bays=BTreeMap::new();
 for (&(x,y,z),&s) in &cells {
  if !floor(s)||!empty(cells.get(&(x,y+1,z)))||!empty(cells.get(&(x,y+2,z))){continue;}
  let key=(y,x.div_euclid(6),z.div_euclid(6));
  let distance=(x.rem_euclid(6)-3).pow(2)+(z.rem_euclid(6)-3).pow(2);
  let entry=bays.entry(key).or_insert((distance,(x,y,z)));
  if (distance,(x,y,z))<*entry{*entry=(distance,(x,y,z));}
 }
 for (_,(_,p)) in bays {
  // A low ceiling fixture sheds light down into the room; tall halls and
  // roof terraces use a solid walkable lamp flush with their paving.
  let ceiling=(4..=5).find(|h|cells.get(&(p.0,p.1+h,p.2)).is_some_and(|s|floor(s))
   && (1..*h).all(|dy|empty(cells.get(&(p.0,p.1+dy,p.2)))));
  if let Some(h)=ceiling {out.push(((p.0,p.1+h-1,p.2),"jak:luz_neon_jak"));}
  else {out.push((p,"jak:luz_suelo_jak"));}
 }
}
#[cfg(test)]mod tests{
 use super::*;
 #[test]fn systems31_lamps_are_registered_emissive_and_have_physical_shapes(){
  let r=minecraftoss_core::Registries::load(&minecraftoss_core::registries::DataPaths::discover().unwrap()).unwrap();
  let wall=r.blocks.parse_state("jak:muro_haven_chapa").unwrap();assert!(r.blocks.state(wall).collision_full_block);assert_eq!(r.blocks.state(wall).light_dampening,15);
  let fire=r.blocks.parse_state("jak:pieza_haven_fuego[type=double,waterlogged=false]").unwrap();assert_eq!(r.blocks.state(fire).light_emission,14);
  let neon=r.blocks.parse_state("jak:luz_neon_jak").unwrap();assert!(!r.blocks.state(neon).suffocating);assert!(!r.blocks.state(neon).collision_full_block);assert_eq!(r.blocks.state(neon).light_dampening,0);
  let table=r.blocks.parse_state("jak:bar_mesa").unwrap();
  assert!(r.blocks.is_face_sturdy(table,minecraftoss_core::pos::Direction::Up,minecraftoss_core::block::SupportType::Full));
  for id in ["jak:luz_suelo_jak","jak:luz_neon_jak"]{
   let state=r.blocks.parse_state(id).unwrap();assert_eq!(r.blocks.state(state).light_emission,15);
   assert!(!r.blocks.collision_boxes(state).is_empty());
  }
 }
 #[test]fn systems31_each_storey_has_supported_lights_without_blocking_the_room(){
  for out in [crate::jak_barrio::building((0,64,0),0,crate::jak_barrio::archetype(15,12,9)),
   crate::jak_spargus::building((0,64,0),1,crate::jak_spargus::archetype(6,12,10))]{
   let cells:HashMap<_,_>=out.iter().copied().collect();
   let lamps:Vec<_>=cells.iter().filter(|(_,s)|s.contains("luz_" )).collect();
   assert!(lamps.len()>=3,"all storeys need lights");
   for (&(x,y,z),&s) in lamps {
    if s=="jak:luz_neon_jak"{assert!(cells.get(&(x,y+1,z)).is_some_and(|s|floor(s)));}
    else {assert!(empty(cells.get(&(x,y+1,z))));}
   }
  }
 }
}
