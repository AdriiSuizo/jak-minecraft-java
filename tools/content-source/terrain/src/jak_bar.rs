//! Hip Hog furniture using original Jak II materials; reusable Minecraft shapes.
use crate::jak_items::{JakBlock,JakItem,JakKind,BlockLook};
use crate::jak_facades::{b,model};
use crate::{jak_structures::Placed,jak_families::Site};
fn counter()->String{model(&[b([0.,0.,0.],[16.,14.,16.],"s"),b([0.,14.,0.],[16.,16.,16.],"t")])}
fn table()->String{model(&[b([0.,12.,0.],[16.,16.,16.],"s"),b([6.,1.,6.],[10.,12.,10.],"s"),b([3.,0.,3.],[13.,1.,13.],"s")])}
fn stool()->String{model(&[b([2.,8.,2.],[14.,10.,14.],"s"),b([6.,1.,6.],[10.,8.,10.],"m"),b([3.,0.,3.],[13.,1.,13.],"m")])}
fn lamp()->String{model(&[b([0.,12.,0.],[16.,16.,16.],"s")])}
pub const TEXTURES:&[&str]=&["hip-tmetfloor01","hip-tyellwall01","hip-tcounter01","hip-tcounter02","hip-twood01","hip-tredstool01","hip-tmetring01","hip-tlogorag01","hip-tbluelit01","hip-tbotblue02","hip-tgreendark01","krew-belt","hip-tbluelit01"];
pub const LEVELS:&[(&str,&str)]=&[("hiphog","hiphog-background.glb"),("lhipout","hip-bottle-c-lod0.glb"),("lhipout","hip-mug-lod0.glb")];
pub const BLOCKS:[JakBlock;7]=[
 JakBlock{path:"bar_suelo",look:BlockLook::All("hip-tmetfloor01")},
 JakBlock{path:"bar_pared",look:BlockLook::All("hip-tyellwall01")},
 JakBlock{path:"bar_mostrador",look:BlockLook::Shaped{vanilla:"stone",model:counter,textures:&[("s","hip-tcounter01"),("t","hip-tcounter02")]}},
 JakBlock{path:"bar_mesa",look:BlockLook::Shaped{vanilla:"stone",model:table,textures:&[("s","hip-twood01")]}},
 JakBlock{path:"bar_taburete",look:BlockLook::Shaped{vanilla:"stone",model:stool,textures:&[("s","hip-tredstool01"),("m","hip-tmetring01")]}},
 JakBlock{path:"bar_rotulo",look:BlockLook::All("hip-tlogorag01")},
 JakBlock{path:"bar_luz",look:BlockLook::Shaped{vanilla:"sea_lantern",model:lamp,textures:&[("s","hip-tbluelit01")]}},
];
pub const ITEMS:[JakItem;7]=[
 JakItem{path:"bar_suelo",title:"Suelo metálico del Hip Hog",kind:JakKind::Block},
 JakItem{path:"bar_pared",title:"Pared amarilla del Hip Hog",kind:JakKind::Block},
 JakItem{path:"bar_mostrador",title:"Barra del Hip Hog",kind:JakKind::Block},
 JakItem{path:"bar_mesa",title:"Mesa del Hip Hog",kind:JakKind::Block},
 JakItem{path:"bar_taburete",title:"Taburete del Hip Hog",kind:JakKind::Block},
 JakItem{path:"bar_rotulo",title:"Emblema original del Hip Hog",kind:JakKind::Block},
 JakItem{path:"bar_luz",title:"Luz azul del Hip Hog",kind:JakKind::Block},
];
pub fn lot(town:&crate::jak_town::Town)->Option<&crate::jak_town::Lot>{
 town.lots.iter().filter(|l|l.w>=9&&l.d>=8&&town.station!=Some((l.origin.0,l.origin.2))&&town.tower!=Some((l.origin.0,l.origin.2)))
 .min_by_key(|l|(l.origin.0-town.center.0).abs()+(l.origin.2-town.center.2).abs())
}
pub fn building(lot:&crate::jak_town::Lot)->Vec<Placed>{
 let mut spec=crate::jak_barrio::archetype(3,lot.w,lot.d);spec.terrace=false;spec.canopy=false;spec.sign=0;
 // The shell and furniture must share the same footprint; archetypes clamp ordinary houses.
 spec.w=lot.w;spec.d=lot.d;
 let mut out=crate::jak_barrio::building(lot.origin,lot.front,spec);
 let s=Site{origin:lot.origin,front:lot.front,w:lot.w,d:lot.d};
 // Full two-block clear entrance, one continuous counter, service aisle,
 // seating along the opposite wall and an accessible ladder to upstairs.
 for x in 1..lot.w-1{for z in 1..lot.d-1{
  out.push((s.at(x,-1,z),"jak:bar_suelo"));for y in 0..3{out.push((s.at(x,y,z),"minecraft:air"));}
 }}
 for z in 2..lot.d-2{out.push((s.at(lot.w-3,0,z),"jak:bar_mostrador"));}
 for z in (2..lot.d-2).step_by(3){out.push((s.at(1,0,z),"jak:bar_mesa"));out.push((s.at(2,0,z),"jak:bar_taburete"));}
 for x in 1..lot.w-1{out.push((s.at(x,0,lot.d-1),"jak:bar_pared"));out.push((s.at(x,1,lot.d-1),"jak:bar_pared"));}
 for x in [2,lot.w-3]{out.push((s.at(x,2,1),"jak:bar_luz"));}
 // A broad sign above the actual door, framed and illuminated from the street.
 for x in lot.w/2-1..=lot.w/2+1 {out.push((s.at(x,3,0),"jak:bar_rotulo"));}
 for x in [1,lot.w-2]{out.push((s.at(x,2,0),"jak:palacio_ventana_luminosa"));
  out.push((s.at(x,-1,-2),"jak:bar_suelo"));out.push((s.at(x,0,-2),"jak:bar_mesa"));
  let chair=if x==1{2}else{lot.w-3};out.push((s.at(chair,-1,-2),"jak:bar_suelo"));out.push((s.at(chair,0,-2),"jak:bar_taburete"));
 }
 for y in 0..6 {out.push((s.at(1,y,lot.d-2),s.facing("minecraft:ladder[facing={f},waterlogged=false]",0)));}
 for(part,z)in[("foot",lot.d-3),("head",lot.d-2)]{out.push((s.at(lot.w-2,4,z),s.facing(&format!("minecraft:red_bed[facing={{f}},occupied=false,part={part}]"),2)));}
 crate::jak_lighting::illuminate(&mut out);crate::jak_beds::arrange(&mut out);out
}
pub fn props(town:&crate::jak_town::Town,out:&mut Vec<Placed>)->Vec<crate::jak_objects::Instance>{
 let Some(lot)=lot(town)else{return Vec::new();};let s=Site{origin:lot.origin,front:lot.front,w:lot.w,d:lot.d};let mut all=Vec::new();
 for (x,z,path) in [(lot.w-3,2,"objeto_haven_mostrador_bar"),(lot.w-3,lot.d-3,"objeto_haven_botellero_bar"),(1,2,"objeto_haven_servicio_bar"),(1,-2,"objeto_haven_mesa_doble_bar"),(lot.w-2,-2,"objeto_haven_servicio_bar")] {
  let Some(kind)=crate::jak_objects::kind_of(path)else{continue;};
  let Some(instance)=crate::jak_objects::placed(kind,s.at(x,1,z),lot.front)else{continue;};
  let shape=crate::jak_objects::shape(kind).unwrap();
  for(p,c)in instance.cells(){let state=match crate::jak_objects::slab_type(&shape.cells[&c]){"top"=>"jak:pieza_haven[type=top,waterlogged=false]","double"=>"jak:pieza_haven[type=double,waterlogged=false]",_=>"jak:pieza_haven[type=bottom,waterlogged=false]"};out.push((p,state));}
  all.push(instance);
 }
 // Exhibit niches belong to the back wall, away from the entrance and service aisle.
 let existing:std::collections::HashMap<_,_>=out.iter().copied().collect();
 for (x,path) in [(2,"objeto_haven_trofeo_grunt_bar"),(4,"objeto_haven_trofeo_pez_bar"),(6,"objeto_haven_trofeo_metal_bar")] {
  if x>=lot.w-4 {continue;}
  let base=s.at(x,0,lot.d-2);let at=s.at(x,1,lot.d-2);
  if [base,at].iter().any(|p|existing.get(p).is_some_and(|v|*v!="minecraft:air")){continue;}
  if let Some(i)=crate::jak_objects::kind_of(path).and_then(|k|crate::jak_objects::placed(k,at,lot.front)) {
   out.push((base,"jak:bar_mostrador"));
   for (p,_) in i.cells(){out.push((p,"jak:pieza_haven[type=bottom,waterlogged=false]"));}all.push(i);
  }
 }
 let final_cells:std::collections::HashMap<_,_>=out.iter().copied().collect();
 let game_at=s.at(3,0,lot.d-2);
 if (0..2).all(|h|final_cells.get(&(game_at.0,game_at.1+h,game_at.2)).is_some_and(|s|*s=="minecraft:air")) {
  if let Some(i)=crate::jak_objects::kind_of("objeto_haven_juego_bar").and_then(|k|crate::jak_objects::placed(k,game_at,lot.front)) {
   for(p,_)in i.cells(){out.push((p,"jak:pieza_haven[type=bottom,waterlogged=false]"));}all.push(i);
  }
 }
 let mut cells:std::collections::HashMap<_,_>=out.iter().copied().collect();
 for (index,home) in town.lots.iter().filter(|l|l.origin!=lot.origin).enumerate().take(24) {
  let site=Site{origin:home.origin,front:home.front,w:home.w,d:home.d};
  let at=site.at(2,0,home.d-3);
  let clear=|p|cells.get(&p).is_none_or(|s|*s=="minecraft:air");
  if !clear(at)||!clear((at.0,at.1+1,at.2))||!cells.get(&(at.0,at.1-1,at.2)).is_some_and(|s|s.contains("suelo")||s.contains("luz_suelo")){continue;}
  out.push((at,"jak:bar_mesa"));cells.insert(at,"jak:bar_mesa");
  let path=["objeto_haven_servicio_bar","objeto_haven_mesa_doble_bar","objeto_haven_coleccion_bar","objeto_haven_botellero_bar"][index%4];
  if let Some(i)=crate::jak_objects::kind_of(path).and_then(|kind|crate::jak_objects::placed(kind,(at.0,at.1+1,at.2),home.front)){
   for(p,_)in i.cells(){out.push((p,"jak:pieza_haven[type=bottom,waterlogged=false]"));cells.insert(p,"jak:pieza_haven[type=bottom,waterlogged=false]");}all.push(i);
  }
 }all
}

/// The tray is a closed base; the bottle and mug above remain their native meshes.
pub(crate) fn tray(out:&mut Vec<crate::jak_pieces::SourceTri>,texture:u16){
 use glam::{Vec2,Vec3};let [x,y,z,a,b,c]=[0.06,0.,0.18,0.94,0.06,0.82];
 for face in [[[x,y,z],[a,y,z],[a,b,z],[x,b,z]],[[a,y,c],[x,y,c],[x,b,c],[a,b,c]],[[x,y,c],[x,y,z],[x,b,z],[x,b,c]],[[a,y,z],[a,y,c],[a,b,c],[a,b,z]],[[x,b,z],[a,b,z],[a,b,c],[x,b,c]],[[x,y,c],[a,y,c],[a,y,z],[x,y,z]]] {
  let p=face.map(Vec3::from_array);let uv=[Vec2::ZERO,Vec2::X,Vec2::ONE,Vec2::Y];
  for k in [[0,2,1],[0,3,2]]{out.push(crate::jak_pieces::SourceTri{p:k.map(|i|p[i]),uv:k.map(|i|uv[i]),texture,color:[[128;4];3],repeat:[true;2],blend:false});}
 }
}

#[cfg(test)]mod tests{
 use super::*;
 #[test]fn props32_plaza_hut_does_not_overwrite_bar(){
  struct Flat;
  impl crate::jak_structures::Land for Flat {
   fn ground(&self,_:i32,_:i32)->i32{64}
   fn wet(&self,_:i32,_:i32)->bool{false}
  }
  let cells:std::collections::HashMap<_,_>=crate::jak_barrio::village(12345,(-144,0),64,0,&Flat).into_iter().collect();
  let mut checked=0;
  for (&(x,y,z),&state) in &cells {
   if state=="jak:bar_suelo" && x>=-140 {
    for h in 1..=3 {
     assert!(!cells.get(&(x,y+h,z)).is_some_and(|s|s.starts_with("jak:barrio_base")),"plaza hut occupies bar at {x},{},{z}",y+h);
    }
    checked+=1;
   }
  }
  assert!(checked>20);
 }
 #[test]fn props32_bar_shell_matches_furniture_footprint(){
  for front in 0..4 {for (w,d) in [(9,8),(17,14),(21,12)] {
   let lot=crate::jak_town::Lot{origin:(40,65,-30),front,w,d,corner:false};
   let s=Site{origin:lot.origin,front,w,d};
   let cells:std::collections::HashMap<_,_>=building(&lot).into_iter().collect();
   // A continuous upper floor over the whole room, with no old facade hanging inside.
   for x in 3..w-3 {for z in 2..d-3 {
    assert!(matches!(cells.get(&s.at(x,3,z)),Some(&"jak:barrio_suelo"|&"jak:luz_suelo_jak")),"{front}: {w}x{d} at {x},{z}");
   }}
   for z in 2..d-2 {assert_eq!(cells.get(&s.at(w-3,0,z)),Some(&"jak:bar_mostrador"));}
  }}
 }
 #[test]fn architecture27_bar_props_are_complete_single_cell_native_objects(){
  let r=minecraftoss_core::Registries::load(&minecraftoss_core::registries::DataPaths::discover().unwrap()).unwrap();
  for b in BLOCKS{r.blocks.parse_state(&format!("jak:{}",b.path)).unwrap();}
  for name in ["objeto_haven_botella_bar","objeto_haven_jarra_bar","objeto_haven_bandeja_bar"]{
   r.blocks.parse_state(&format!("jak:{name}")).unwrap();
   let k=crate::jak_objects::kind_of(name).unwrap();let shape=crate::jak_objects::shape(k).expect(name);
   assert_eq!(shape.size,(1,1,1),"{name} must fit completely in its cell");
   let p=&shape.cells[&(0,0,0)];assert!(p.tris.len()>12,"{name}: native geometry");assert!(!p.collision_boxes().is_empty());
  }
  for &name in TEXTURES{assert!(crate::original_places::embedded_texture(name).is_some(),"{name}: original texture must exist");}
 }
}
