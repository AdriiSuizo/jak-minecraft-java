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
fn texture(name:&str)->Option<image::RgbaImage>{let(w,h,mut rgba)=minecraft_terrain::original_places::embedded_texture(name).or_else(||tpage_texture(name))?;for p in rgba.chunks_exact_mut(4){p[3]=p[3].saturating_mul(2);}image::RgbaImage::from_raw(w,h,rgba)}
// Adrii 06/11: the Wasteland's textures (des-*) and a few more are in none of the levels read, so their blocks were
// drawn with the stand-in stripes. The extracted tpages' own images instead (Jak 3's first, then Jak 2's and Jak 1's).
fn tpage_texture(name:&str)->Option<(u32,u32,Vec<u8>)>{
 static FILES:std::sync::OnceLock<std::collections::HashMap<String,std::path::PathBuf>>=std::sync::OnceLock::new();
 let files=FILES.get_or_init(||{let mut out=std::collections::HashMap::new();
  for game in ["IW4L_JAK3_ASSETS","IW4L_JAK2_ASSETS","IW4L_JAK1_ASSETS"]{
   let Some(root)=std::env::var_os(game) else{continue};
   let Ok(tpages)=std::fs::read_dir(Path::new(&root).join("textures")) else{continue};
   let mut tpages:Vec<_>=tpages.flatten().map(|e|e.path()).collect();tpages.sort();
   for tpage in tpages{let Ok(images)=std::fs::read_dir(&tpage) else{continue};
    for image in images.flatten(){let path=image.path();
     if let Some(stem)=path.file_stem().and_then(|s|s.to_str()).filter(|_|path.extension().is_some_and(|e|e=="png")){out.entry(stem.to_owned()).or_insert(path);}}}
  }out});
 let image=image::open(files.get(name)?).ok()?.to_rgba8();
 Some((image.width(),image.height(),image.into_raw()))
}
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
fn save_sites(root:&Path){
 use minecraft_terrain::jak_kit::{SWAMP_HUTS,SNOW_FORTS,View};
 use minecraft_terrain::jak_structures::{cave_spot,crater_cave,village,village_spot};
 let height=|_,_|64;
 let inside=|_,_|true;
 let view=View{ground:&height,surface:&height,inside:&inside};
 for (name,site) in [("swamp_hut",&SWAMP_HUTS),("snow_fort",&SNOW_FORTS)]{
  let blocks=(site.build)(8048,(0,0),(0,0),&view).expect("original climate site plan");
  save(root,name,blocks,Vec::new());
 }
 for (name,spot,build) in [
  ("sandover",village_spot as fn(i64,(i32,i32))->Option<(i32,i32)>,village as fn(i64,(i32,i32),&dyn Land)->Option<Vec<Placed>>),
  ("crater_cave",cave_spot,crater_cave),
 ]{
  let seed=8048;
  let (cell,(cx,cz))=(0..64).find_map(|i|spot(seed,(i,0)).map(|pos|((i,0),pos))).expect("original site");
  let blocks=build(seed,cell,&Flat).expect("original site plan");
  let relative=blocks.into_iter().map(|((x,y,z),s)|((x-cx,y,z-cz),s)).collect();
  save(root,name,relative,Vec::new());
 }
}
fn main(){let args:Vec<_>=std::env::args().collect();let root=Path::new(&args[1]);std::fs::create_dir_all(root).unwrap();
 save_sites(root);
 if args.get(2).is_some_and(|s|s=="--sites-only"){return;}
 if args.get(2).is_some_and(|s|s=="--track-only"){save_track(root);return;}
 save_wall_roots(root);
 if args.get(2).is_some_and(|s|s=="--wall-roots-only"){return;}
 save_track(root);
 jak_items::set_block_texture_source(texture);
 // A file just written can be held a moment by another program (an antivirus scan): os error 32 while the old pack
 // is removed. Try again a few times before giving up (06/11: an install failed so).
 for attempt in 1..=6{match jak_items::write_pack(&root.join("source-pack")){Ok(())=>break,Err(e) if attempt<6=>{eprintln!("pack write failed ({e}), retrying");std::thread::sleep(std::time::Duration::from_secs(3));}Err(e)=>panic!("{e}")}}
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
