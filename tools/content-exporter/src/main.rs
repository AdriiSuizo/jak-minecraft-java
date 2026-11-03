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
fn main(){let args:Vec<_>=std::env::args().collect();let root=Path::new(&args[1]);std::fs::create_dir_all(root).unwrap();
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
