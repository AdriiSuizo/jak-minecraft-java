//! Reusable structural pieces shared by the palace and player construction.
//! Full blocks, joined horizontal slabs/stairs and four-way vertical halves.
//! Curved skating ramps retain sealed geometry; structural pieces follow the grid.
use crate::jak_items::{BlockLook,JakBlock,JakItem,JakKind};
fn vertical_half()->String{crate::jak_facades::model(&[crate::jak_facades::b([0.,0.,0.],[16.,16.,8.],"s")])}
fn emblem()->String{crate::jak_facades::model(&[crate::jak_facades::b([0.,0.,12.],[16.,16.,16.],"s")])}
fn neon()->String{crate::jak_facades::model(&[crate::jak_facades::b([1.,12.,3.],[15.,16.,13.],"m"),crate::jak_facades::b([2.,11.5,5.],[14.,12.,11.],"s")])}
pub const BLOCKS:[JakBlock;29]=[
 JakBlock{path:"pieza_haven_fuego",look:BlockLook::Like{vanilla:"crimson_slab",textures:&[("*","city-slum-burning-can")]}},
 JakBlock{path:"luz_suelo_jak",look:BlockLook::All("prison-hall-light-01")},
 JakBlock{path:"luz_neon_jak",look:BlockLook::Shaped{vanilla:"sea_lantern",model:neon,textures:&[("m","city-slum-greenmetal-tube"),("s","prison-hall-light-01")]}},
 JakBlock{path:"palacio_ventana_luminosa",look:BlockLook::All("prison-hall-light-01")},
 JakBlock{path:"palacio_suelo_metal",look:BlockLook::All("skatea-metalfloor-01")},
 JakBlock{path:"palacio_suelo_metal_losa",look:BlockLook::Slab{texture:"skatea-metalfloor-01",double:"palacio_suelo_metal"}},
 JakBlock{path:"palacio_suelo_metal_medio_vertical",look:BlockLook::Shaped{vanilla:"carved_pumpkin",model:vertical_half,textures:&[("s","skatea-metalfloor-01")]}},
 JakBlock{path:"palacio_suelo_metal_escalera",look:BlockLook::Like{vanilla:"stone_stairs",textures:&[("*","skatea-metalfloor-01")]}},
 JakBlock{path:"palacio_suelo_piedra",look:BlockLook::All("t-citywide-wall-tile-01")},
 JakBlock{path:"palacio_suelo_piedra_losa",look:BlockLook::Slab{texture:"t-citywide-wall-tile-01",double:"palacio_suelo_piedra"}},
 JakBlock{path:"palacio_suelo_piedra_medio_vertical",look:BlockLook::Shaped{vanilla:"carved_pumpkin",model:vertical_half,textures:&[("s","t-citywide-wall-tile-01")]}},
 JakBlock{path:"palacio_suelo_piedra_escalera",look:BlockLook::Like{vanilla:"stone_stairs",textures:&[("*","t-citywide-wall-tile-01")]}},
 JakBlock{path:"palacio_suelo_marmol",look:BlockLook::All("skatea-marble-floor-01")},
 JakBlock{path:"palacio_suelo_marmol_losa",look:BlockLook::Slab{texture:"skatea-marble-floor-01",double:"palacio_suelo_marmol"}},
 JakBlock{path:"palacio_suelo_marmol_medio_vertical",look:BlockLook::Shaped{vanilla:"carved_pumpkin",model:vertical_half,textures:&[("s","skatea-marble-floor-01")]}},
 JakBlock{path:"palacio_suelo_marmol_escalera",look:BlockLook::Like{vanilla:"stone_stairs",textures:&[("*","skatea-marble-floor-01")]}},
 JakBlock{path:"palacio_suelo_panel",look:BlockLook::All("skatea-metal-panel-03")},
 JakBlock{path:"palacio_suelo_panel_losa",look:BlockLook::Slab{texture:"skatea-metal-panel-03",double:"palacio_suelo_panel"}},
 JakBlock{path:"palacio_suelo_panel_medio_vertical",look:BlockLook::Shaped{vanilla:"carved_pumpkin",model:vertical_half,textures:&[("s","skatea-metal-panel-03")]}},
 JakBlock{path:"palacio_suelo_panel_escalera",look:BlockLook::Like{vanilla:"stone_stairs",textures:&[("*","skatea-metal-panel-03")]}},
 JakBlock{path:"palacio_pared_azul",look:BlockLook::All("t-palshaft-dirt-blue-01")},
 JakBlock{path:"palacio_pared_azul_losa",look:BlockLook::Slab{texture:"t-palshaft-dirt-blue-01",double:"palacio_pared_azul"}},
 JakBlock{path:"palacio_pared_azul_medio_vertical",look:BlockLook::Shaped{vanilla:"carved_pumpkin",model:vertical_half,textures:&[("s","t-palshaft-dirt-blue-01")]}},
 JakBlock{path:"palacio_pared_estuco",look:BlockLook::All("city-slumwall-06")},
 JakBlock{path:"palacio_pared_estuco_losa",look:BlockLook::Slab{texture:"city-slumwall-06",double:"palacio_pared_estuco"}},
 JakBlock{path:"palacio_pared_estuco_medio_vertical",look:BlockLook::Shaped{vanilla:"carved_pumpkin",model:vertical_half,textures:&[("s","city-slumwall-06")]}},
 JakBlock{path:"palacio_techo_escalera",look:BlockLook::Like{vanilla:"stone_stairs",textures:&[("*","t-citypal-red-met-01")]}},
 JakBlock{path:"palacio_pilar_escalera",look:BlockLook::Like{vanilla:"stone_stairs",textures:&[("*","t-citypal-met-pill-01")]}},
 JakBlock{path:"palacio_emblema",look:BlockLook::Shaped{vanilla:"carved_pumpkin",model:emblem,textures:&[("s","fort-tank-beige-logo")]}},
];
pub const ITEMS:[JakItem;28]=[
 JakItem{path:"luz_suelo_jak",title:"Luminaria sólida de Jak",kind:JakKind::Block},
 JakItem{path:"luz_neon_jak",title:"Neón industrial de Jak",kind:JakKind::Block},
 JakItem{path:"palacio_ventana_luminosa",title:"Panel luminoso sólido del palacio",kind:JakKind::Block},
 JakItem{path:"palacio_suelo_metal",title:"Suelo metálico del palacio",kind:JakKind::Block},
 JakItem{path:"palacio_suelo_metal_losa",title:"Suelo metálico del palacio — media losa",kind:JakKind::Block},
 JakItem{path:"palacio_suelo_metal_medio_vertical",title:"Suelo metálico del palacio — mitad vertical",kind:JakKind::Block},
 JakItem{path:"palacio_suelo_metal_escalera",title:"Suelo metálico del palacio — escalera",kind:JakKind::Block},
 JakItem{path:"palacio_suelo_piedra",title:"Pavimento de piedra del palacio",kind:JakKind::Block},
 JakItem{path:"palacio_suelo_piedra_losa",title:"Pavimento de piedra del palacio — media losa",kind:JakKind::Block},
 JakItem{path:"palacio_suelo_piedra_medio_vertical",title:"Pavimento de piedra del palacio — mitad vertical",kind:JakKind::Block},
 JakItem{path:"palacio_suelo_piedra_escalera",title:"Pavimento de piedra del palacio — escalera",kind:JakKind::Block},
 JakItem{path:"palacio_suelo_marmol",title:"Pavimento de mármol de Jak II",kind:JakKind::Block},
 JakItem{path:"palacio_suelo_marmol_losa",title:"Pavimento de mármol de Jak II — media losa",kind:JakKind::Block},
 JakItem{path:"palacio_suelo_marmol_medio_vertical",title:"Pavimento de mármol de Jak II — mitad vertical",kind:JakKind::Block},
 JakItem{path:"palacio_suelo_marmol_escalera",title:"Pavimento de mármol de Jak II — escalera",kind:JakKind::Block},
 JakItem{path:"palacio_suelo_panel",title:"Panel de suelo industrial",kind:JakKind::Block},
 JakItem{path:"palacio_suelo_panel_losa",title:"Panel de suelo industrial — media losa",kind:JakKind::Block},
 JakItem{path:"palacio_suelo_panel_medio_vertical",title:"Panel de suelo industrial — mitad vertical",kind:JakKind::Block},
 JakItem{path:"palacio_suelo_panel_escalera",title:"Panel de suelo industrial — escalera",kind:JakKind::Block},
 JakItem{path:"palacio_pared_azul",title:"Pared azul del palacio",kind:JakKind::Block},
 JakItem{path:"palacio_pared_azul_losa",title:"Pared azul del palacio — media losa",kind:JakKind::Block},
 JakItem{path:"palacio_pared_azul_medio_vertical",title:"Pared azul del palacio — mitad vertical",kind:JakKind::Block},
 JakItem{path:"palacio_pared_estuco",title:"Pared de estuco de Villa Refugio",kind:JakKind::Block},
 JakItem{path:"palacio_pared_estuco_losa",title:"Pared de estuco de Villa Refugio — media losa",kind:JakKind::Block},
 JakItem{path:"palacio_pared_estuco_medio_vertical",title:"Pared de estuco de Villa Refugio — mitad vertical",kind:JakKind::Block},
 JakItem{path:"palacio_techo_escalera",title:"Techo inclinado rojo del palacio",kind:JakKind::Block},
 JakItem{path:"palacio_pilar_escalera",title:"Soporte inclinado metálico del palacio",kind:JakKind::Block},
 JakItem{path:"palacio_emblema",title:"Emblema original de la Guardia Carmesí",kind:JakKind::Block},
];

/// Convert only complete rectangular surfaces into catalogue pieces. A ramp,
/// rail, curved outline or mixed-detail cell must keep its authored geometry.
/// This is a geometry test, not a bounding-box replacement of empty space.
pub fn reusable_piece(piece:&crate::jak_pieces::Piece)->Option<&'static str>{
 static NAMES:std::sync::OnceLock<Vec<&'static str>>=std::sync::OnceLock::new();
 let names=NAMES.get_or_init(||crate::original_places::textures_of(&crate::original_places::PLACES[1]));
 let first=piece.tris.first()?;
 let texture=names.get(first.texture as usize)?;
 if piece.tris.iter().any(|t|t.blend||names.get(t.texture as usize)!=Some(texture)){return None;}
 let (slab,panel)=match *texture {
  "skatea-metalfloor-01"=>("jak:palacio_suelo_metal_losa[type=top,waterlogged=false]","palacio_suelo_metal_medio_vertical"),
  "skatea-marble-floor-01"=>("jak:palacio_suelo_marmol_losa[type=top,waterlogged=false]","palacio_suelo_marmol_medio_vertical"),
  "skatea-metal-panel-03"=>("jak:palacio_suelo_panel_losa[type=top,waterlogged=false]","palacio_suelo_panel_medio_vertical"),
  "t-citywide-wall-tile-01"=>("jak:palacio_suelo_piedra_losa[type=top,waterlogged=false]","palacio_suelo_piedra_medio_vertical"),
  "t-palshaft-dirt-blue-01"=>("jak:palacio_pared_azul_losa[type=top,waterlogged=false]","palacio_pared_azul_medio_vertical"),
  "city-slumwall-06"=>("jak:palacio_pared_estuco_losa[type=top,waterlogged=false]","palacio_pared_estuco_medio_vertical"),
  _=>return None,
 };
 let mut lo=[1f32;3];let mut hi=[0f32;3];
 for p in piece.tris.iter().flat_map(|t|t.p){for k in 0..3{lo[k]=lo[k].min(p[k]);hi[k]=hi[k].max(p[k]);}}
 // Nine-by-nine coverage rejects triangular corner fragments and genuine holes.
 let covered=|axis:usize,height:f32|{
  let axes:Vec<_>=(0..3).filter(|k|*k!=axis).collect();
  (0..9).all(|u|(0..9).all(|v|{
   let p=glam::Vec2::new((u as f32+0.03)/8.06,(v as f32+0.03)/8.06);
   piece.tris.iter().any(|t|{
    if t.p.iter().any(|p|(p[axis]-height).abs()>0.015){return false;}
    let a=t.p.map(|q|glam::Vec2::new(q[axes[0]],q[axes[1]]));
    let cross=|a:glam::Vec2,b:glam::Vec2|a.x*b.y-a.y*b.x;
    let det=cross(a[1]-a[0],a[2]-a[0]);if det.abs()<1e-7{return false;}
    let u=cross(p-a[0],a[2]-a[0])/det;let v=cross(a[1]-a[0],p-a[0])/det;
    u>=-0.0001&&v>=-0.0001&&u+v<=1.0001
   })
  }))
 };
 if hi[1]>=0.98&&lo[1]>=0.49&&covered(1,hi[1]){return Some(slab);}
 let facing=if hi[2]<=0.5&&covered(2,lo[2]){Some("north")}
 else if lo[2]>=0.5&&covered(2,hi[2]){Some("south")}
 else if hi[0]<=0.5&&covered(0,lo[0]){Some("west")}
 else if lo[0]>=0.5&&covered(0,hi[0]){Some("east")}else{None}?;
 // Intern the tiny, bounded catalogue (6 materials x 4 orientations), never
 // individual world coordinates or source cells.
 static STATES:std::sync::OnceLock<std::collections::HashMap<(&'static str,&'static str),&'static str>>=std::sync::OnceLock::new();
 let states=STATES.get_or_init(||{
  let mut m=std::collections::HashMap::new();
  for b in &BLOCKS{if b.path.ends_with("_medio_vertical"){for f in ["north","east","south","west"]{
   m.insert((b.path,f),&*Box::leak(format!("jak:{}[facing={f}]",b.path).into_boxed_str()));
  }}}m
 });states.get(&(panel,facing)).copied()
}

#[cfg(test)]mod tests{
 use super::*;
 #[test]fn architecture26_visible_parts_match_physics_and_all_forms_exist(){
  let paths=minecraftoss_core::registries::DataPaths::discover().unwrap();
  let r=minecraftoss_core::Registries::load(&paths).unwrap();
  for (id,expected) in [
   ("jak:arena_luz_placa",[0.,0.5,0.,1.,1.,1.]),
   ("jak:palacio_suelo_metal_medio_vertical[facing=north]",[0.,0.,0.,1.,1.,0.5]),
   ("jak:palacio_suelo_metal_medio_vertical[facing=east]",[0.5,0.,0.,1.,1.,1.]),
   ("jak:palacio_suelo_metal_losa[type=top,waterlogged=false]",[0.,0.5,0.,1.,1.,1.]),
  ]{let state=r.blocks.parse_state(id).unwrap();let boxes=r.blocks.collision_boxes(state);
   assert_eq!(boxes,vec![expected],"visible structural bounds: {id}");
  }
  for id in ["jak:palacio_detalle_columna","jak:palacio_detalle_cornisa"]{
   let state=r.blocks.parse_state(id).unwrap();let boxes=r.blocks.collision_boxes(state);
   assert!(boxes.iter().any(|b|b[4]>=1.),"{id} was divided by sixteen twice");
  }
  for b in &BLOCKS{assert!(r.blocks.block_by_name(&format!("jak:{}",b.path)).is_some(),"{}",b.path);}
 }

}

/// A content migration, restricted to native source9 blocks that still exist.
/// Runs on chunk arrival, not once per frame over the surrounding world.
pub fn upgrades(chunk:&minecraftoss_core::Chunk,r:&minecraftoss_core::Registries,scene:&crate::scene::HandcraftedScene)->Vec<((i32,i32,i32),crate::scene::Block)>{
 use crate::scene::{Block,Scene};
 let Some(id)=r.blocks.block_by_name("jak:pieza_haven_torre_circuito")else{return Vec::new();};
 let states:Vec<_>=r.blocks.block(id).states().collect();
 let mut out=Vec::new();
 for (sy,section) in chunk.sections().iter().enumerate(){
  if section.is_empty(){continue;}
  for i in 0..4096{
   let state=section.blocks.get(i);if !states.contains(&state){continue;}
   let p=(chunk.pos.x*16+(i&15)as i32,chunk.min_y()+sy as i32*16+(i>>8)as i32,chunk.pos.z*16+((i>>4)&15)as i32);
   if !scene.block(p).is_some_and(|b|b.id.namespace=="jak"&&b.id.path=="pieza_haven_torre_circuito"){continue;}
   let Some((9,column,cell))=crate::jak_pieces::piece_at(9,p)else{continue;};
   let Some(text)=column.get(&cell).and_then(reusable_piece)else{continue;};
   let (name,properties)=text.split_once('[').unwrap_or((text,""));
   let mut block=Block::new(name);
   for property in properties.trim_end_matches(']').split(','){
    if let Some((k,v))=property.split_once('='){block=block.with(k,v);}
   }
   // Do not erase water when adapting an old waterlogged surface.
   if r.blocks.property(state,"waterlogged")==Some("true") {
    if block.properties.contains_key("waterlogged"){block=block.with("waterlogged","true");}else{continue;}
   }
   out.push((p,block));
  }
 }out
}
