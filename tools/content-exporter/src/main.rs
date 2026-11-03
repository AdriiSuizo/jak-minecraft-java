use minecraft_terrain::{jak_barrio,jak_spargus,jak_city_arena,jak_items,jak_structures::{Land,Placed}};
use serde_json::json;
use std::path::Path;
struct Flat;
impl Land for Flat {fn ground(&self,_:i32,_:i32)->i32{64} fn wet(&self,_:i32,_:i32)->bool{false}}
fn save(root:&Path,name:&str,blocks:Vec<Placed>,props:Vec<minecraft_terrain::jak_objects::Instance>){
 let mut palette=Vec::new();let mut ids=std::collections::HashMap::new();let mut entries=Vec::new();
 for((x,y,z),state)in blocks{let next=palette.len();let id=*ids.entry(state).or_insert_with(||{palette.push(state);next});entries.push(json!([x,y-64,z,id]));}
 let props:Vec<_>=props.iter().map(|p|json!({"kind":p.kind,"origin":[p.origin.0,p.origin.1-64,p.origin.2],"turns":p.turns})).collect();
 std::fs::write(root.join(format!("{name}.json")),json!({"palette":palette,"blocks":entries,"props":props}).to_string()).unwrap();
}
fn texture(name:&str)->Option<image::RgbaImage>{let(w,h,mut rgba)=minecraft_terrain::original_places::embedded_texture(name)?;for p in rgba.chunks_exact_mut(4){p[3]=p[3].saturating_mul(2);}image::RgbaImage::from_raw(w,h,rgba)}
fn look(b:jak_items::BlockLook)->serde_json::Value {use jak_items::BlockLook::*;match b {
 Like{vanilla,..}|Shaped{vanilla,..}|Set{vanilla,..}=>json!({"kind":"native","vanilla":vanilla}),
 Slab{double,..}=>json!({"kind":"slab","double":double}),
 Layers{full,..}=>json!({"kind":"layers","full":full}),
 Cross(_)|Plant{..}=>json!({"kind":"plant"}),Leaves(_)=>json!({"kind":"leaves"}),
 Column{..}=>json!({"kind":"pillar"}),Vine(_)=>json!({"kind":"vine"}),LilyPad(_)=>json!({"kind":"lily_pad"}),_=>json!({"kind":"cube"})}}
fn save_wall_roots(root:&Path){
 let mut roots=serde_json::Map::new();
 for (id,kind) in minecraft_terrain::jak_objects::OBJECTS.iter().enumerate(){
  if !kind.versioned_foliage(){continue;}
  let shape=minecraft_terrain::jak_objects::shape(id).expect("source facade foliage geometry");
  let anchor=shape.wall_root.expect("source facade foliage wall root");
  roots.insert(id.to_string(),json!(anchor));
 }
 std::fs::write(root.join("wall-roots.json"),json!(roots).to_string()).unwrap();
}
fn save_track(root:&Path){
 use minecraft_terrain::{jak_city_arena as arena,jak_pieces,original_places};
 let center=arena::source_origin().map(|v|v as i32);
 let textures=original_places::textures_of(&original_places::PLACES[1]);
 let track=[
  (105,"objeto_haven_rampa_cuarto","Módulo curvo de turbotabla","skatea-ramp-01",[-5,6,1],[-5,10,6]),
  (106,"objeto_haven_rampa_piramide","Pirámide de turbotabla","skatea-ramp-01",[-19,6,-9],[-12,7,-2]),
  (107,"objeto_haven_rail_grind","Barandilla de grind de Haven","skatea-grind-rail-01",[-24,6,-16],[-23,7,5]),
 ];
 let out=root.join("objects");std::fs::create_dir_all(&out).unwrap();
 for (id,path,title,material,lo,hi) in track{
  let texture=textures.iter().position(|&t|t==material).expect("source track texture") as u16;
  let mut cells=Vec::new();
  for x in lo[0]..=hi[0]{for y in lo[1]..=hi[1]{for z in lo[2]..=hi[2]{
   let pos=(center[0]+x,center[1]+y,center[2]+z);
   let Some(column)=jak_pieces::column(arena::SOURCE_ID,(pos.0.div_euclid(16)*16,pos.2.div_euclid(16)*16))else{continue;};
   let Some(piece)=column.pieces.get(&pos)else{continue;};
   let triangles:Vec<_>=piece.tris.iter().filter(|t|t.texture==texture)
    .map(|t|json!({"p":t.p,"uv":t.uv,"color":t.color,"texture":material,"blend":t.blend})).collect();
   if triangles.is_empty(){continue;}
   cells.push(json!({"cell":[x-lo[0],y-lo[1],z-lo[2]],"triangles":triangles,"boxes":piece.collision_boxes().as_ref()}));
  }}}
  assert!(!cells.is_empty(),"no source track cells for {path}");
  let size=std::array::from_fn::<_,3,_>(|axis|hi[axis]-lo[axis]+1);
  let rails=if id==107{arena::rail_lines().iter().map(|rail|
   rail.iter().map(|p|std::array::from_fn::<_,3,_>(|axis|p[axis]-f64::from(center[axis]+lo[axis]))).collect::<Vec<_>>()
  ).collect::<Vec<_>>()}else{Vec::new()};
  std::fs::write(out.join(format!("{id}.json")),json!({"id":id,"path":path,"title":title,
   "group":if id==107{"rail"}else{"track"},"size":size,"soft":false,"cells":cells,"rails":rails}).to_string()).unwrap();
 }
}
fn main(){let args:Vec<_>=std::env::args().collect();let root=Path::new(&args[1]);std::fs::create_dir_all(root).unwrap();
 if args.get(2).is_some_and(|s|s=="--track-only"){save_track(root);return;}
 save_wall_roots(root);
 if args.get(2).is_some_and(|s|s=="--wall-roots-only"){return;}
 save_track(root);
 jak_items::set_block_texture_source(texture);
 jak_items::write_pack(&root.join("source-pack")).unwrap();
 let layout=jak_barrio::natural_layout(8048,(0,0),64,0,&Flat).expect("Haven source plan");
 let palace=layout.palace.expect("Haven must surround palace");
 let mut haven=jak_barrio::village_from_layout(8048,layout,&Flat);
 haven.extend(jak_city_arena::fixtures().map(|((x,y,z),s)|((x+palace.0,y+palace.1,z+palace.2),s)));
 let mut props=minecraft_terrain::jak_town::known_props_near(8048,(0,64,0),300);
 props.extend(minecraft_terrain::jak_town::export_foliage(8048));
 save(root,"haven",haven,props);
 let spargus=jak_spargus::town(8048,(0,0),64,0,&Flat);
 save(root,"spargus",spargus,minecraft_terrain::jak_town::known_props_near(8048,(0,64,0),300));
 save(root,"palace",jak_city_arena::fixtures().map(|((x,y,z),s)|((x,y+64,z),s)).collect(),Vec::new());
 let blocks:Vec<_>=jak_items::block_looks().map(|b|json!({"path":b.path,"look":look(b.look)})).collect();
 std::fs::write(root.join("blocks.json"),json!(blocks).to_string()).unwrap();
 let objects_dir=root.join("objects");std::fs::create_dir_all(&objects_dir).unwrap();
 for (id,kind) in minecraft_terrain::jak_objects::OBJECTS.iter().enumerate(){
  let Some(shape)=minecraft_terrain::jak_objects::shape(id) else{panic!("Missing authored object {}",kind.block)};
  let textures=minecraft_terrain::original_places::textures_of(&minecraft_terrain::original_places::PLACES[shape.place]);
  let mut cells=Vec::new();
  for (cell,piece) in &shape.cells {
   let triangles:Vec<_>=piece.tris.iter().map(|t|json!({"p":t.p,"uv":t.uv,"color":t.color,"texture":textures[t.texture as usize].to_lowercase(),"blend":t.blend})).collect();
   cells.push(json!({"cell":[cell.0,cell.1,cell.2],"triangles":triangles,"boxes":piece.collision_boxes().as_ref()}));
  }
  std::fs::write(objects_dir.join(format!("{id}.json")),json!({"id":id,"path":kind.block,"title":kind.title,"group":kind.group,"size":[shape.size.0,shape.size.1,shape.size.2],"soft":kind.soft_vegetation(),"cells":cells}).to_string()).unwrap();
 }
 println!("Exported Jak source block definitions, palace, Haven and Spargus");
}
