//! Grid-built Haven palace and closed JET-board ramp modules (source10).
//! Historical palace geometry remains in sources7/8/9 for saved pieces.
use std::{collections::HashMap,sync::{Arc,Mutex,OnceLock}};
use glam::{Vec2,Vec3};
use crate::jak_pieces::{Source,SourceTri,Column};
use crate::jak_items::{JakBlock,JakItem,JakKind,BlockLook};
pub const SOURCE_ID:usize=10;
pub const PIECE:&str="jak:pieza_haven_rampa_modular";
pub const AT:i32=8000;pub const SIZE:i32=160;pub const MIN_Y:i32=-2;pub const HEIGHT:i32=192;
pub const FOOTPRINT_MIN:[i32;2]=[-80,-80];pub const FOOTPRINT_MAX:[i32;2]=[92,80];
pub const ACCESS:[f64;3]=[-5.0,0.05,64.0];pub const ARRIVAL:[f64;3]=[-5.0,6.05,35.0];
pub const BASEMENT_FLOOR_Y:i32=0;
const CANONICAL:Vec3=Vec3::new(8068.0,0.0,8080.0);
fn light_plate()->String{
 // A structural half slab: the luminous face is supported by real metal.
 crate::jak_facades::model(&[crate::jak_facades::b([0.,8.,0.],[16.,15.,16.],"s"),
 crate::jak_facades::b([0.,15.,0.],[16.,16.,16.],"l")])
}
fn palace_pillar()->String{use crate::jak_facades::{model,b};model(&[b([2.,0.,2.],[14.,2.,14.],"p"),b([4.,2.,4.],[12.,14.,12.],"p"),b([2.,14.,2.],[14.,16.,14.],"p")])}
fn palace_cornice()->String{use crate::jak_facades::{model,b};model(&[b([0.,8.,3.],[16.,12.,13.],"p"),b([0.,12.,1.],[16.,16.,15.],"p")])}
fn board_altar()->String { use crate::jak_facades::{model,b};model(&[
 b([0.,0.,0.],[16.,3.,16.],"edge"),b([3.,3.,3.],[13.,10.,13.],"metal"),
 b([1.,10.,1.],[15.,12.,15.],"edge")]) }
pub const BLOCKS:[JakBlock;11]=[
 JakBlock{path:"altar_turbotabla",look:BlockLook::Shaped{vanilla:"stone",model:board_altar,textures:&[("metal","skatea-metalfloor-01"),("edge","skatea-metal-edge-01")]}},
 JakBlock{path:"altar_turbotabla_vacio",look:BlockLook::Shaped{vanilla:"stone",model:board_altar,textures:&[("metal","skatea-metalfloor-01"),("edge","skatea-metal-edge-01")]}},
 JakBlock{path:"palacio_detalle_columna",look:BlockLook::Shaped{vanilla:"stone",model:palace_pillar,textures:&[("p","t-citypal-met-pill-01")]}},
 JakBlock{path:"palacio_detalle_cornisa",look:BlockLook::Shaped{vanilla:"stone",model:palace_cornice,textures:&[("p","t-citypal-met-bm-red-strp01")]}},
 JakBlock{path:"arena_luz_placa",look:BlockLook::Shaped{vanilla:"sea_lantern",model:light_plate,textures:&[("l","prison-hall-light-01"),("s","skatea-metalfloor-01")]}},
 crate::jak_city_arena_legacy::BLOCKS[0],
 JakBlock{path:"pieza_haven_torre_circuito",look:BlockLook::Slab{texture:"skatea-metalfloor-01",double:"pieza_haven_torre_circuito"}},
 JakBlock{path:"arena_suelo",look:BlockLook::All("skatea-metalfloor-01")},
 JakBlock{path:"arena_borde",look:BlockLook::All("skatea-metal-edge-01")},
 JakBlock{path:"arena_losa",look:BlockLook::All("skatea-marble-floor-01")},
 JakBlock{path:"pieza_haven_rampa_modular",look:BlockLook::Slab{texture:"skatea-metalfloor-01",double:"pieza_haven_rampa_modular"}},
];
pub const ITEMS:[JakItem;11]=[
 JakItem{path:"altar_turbotabla",title:"Altar con turbotabla",kind:JakKind::Block},
 JakItem{path:"altar_turbotabla_vacio",title:"Altar de turbotabla vacío",kind:JakKind::Block},
 JakItem{path:"palacio_detalle_columna",title:"Columna modular del palacio de Praxis",kind:JakKind::Block},
 JakItem{path:"palacio_detalle_cornisa",title:"Cornisa roja del palacio de Praxis",kind:JakKind::Block},
 JakItem{path:"arena_luz_placa",title:"Luz empotrada del palacio",kind:JakKind::Block},
 crate::jak_city_arena_legacy::ITEMS[0],
 JakItem{path:"pieza_haven_torre_circuito",title:"Pieza de rampa o torre adaptada",kind:JakKind::Block},
 JakItem{path:"arena_suelo",title:"Suelo del skatepark de Jak II",kind:JakKind::Block},
 JakItem{path:"arena_borde",title:"Borde metálico del skatepark",kind:JakKind::Block},
 JakItem{path:"arena_losa",title:"Losa del skatepark de Jak II",kind:JakKind::Block},
 JakItem{path:"pieza_haven_rampa_modular",title:"Módulo macizo de rampa",kind:JakKind::Block},
];
pub fn source_origin()->[f32;3]{CANONICAL.to_array()}
pub fn local_point(p:[f32;3])->Vec3{CANONICAL+Vec3::from_array(p)}
fn quad(p:[Vec3;4],texture:u16,out:&mut Vec<SourceTri>){
 let u=p[0].distance(p[1])*0.5;let v=p[1].distance(p[2])*0.5;
 let uv=[Vec2::ZERO,Vec2::new(u,0.0),Vec2::new(u,v),Vec2::new(0.0,v)];
 for k in [[0,2,1],[0,3,2]]{out.push(SourceTri{p:k.map(|i|p[i]),uv:k.map(|i|uv[i]),texture,color:[[128;4];3],repeat:[true;2],blend:false});}
}
fn prism(lo:Vec3,hi:Vec3,texture:u16,out:&mut Vec<SourceTri>){
 let [x,y,z]=lo.to_array();let[a,b,c]=hi.to_array();
 for p in [ [[x,y,z],[a,y,z],[a,b,z],[x,b,z]],[[a,y,c],[x,y,c],[x,b,c],[a,b,c]],
 [[x,y,c],[x,y,z],[x,b,z],[x,b,c]],[[a,y,z],[a,y,c],[a,b,c],[a,b,z]],
 [[x,b,z],[a,b,z],[a,b,c],[x,b,c]],[[x,y,c],[a,y,c],[a,y,z],[x,y,z]] ]{quad(p.map(Vec3::from_array),texture,out);}
}
#[path="jak_solid_cells.rs"] mod solid_cells;
#[path="jak_palace_grid.rs"] mod grid;
pub fn ascent_point(t:f32)->[f32;3]{grid::ascent_point(t)}
pub fn solid_base_column(_x:i32,_z:i32)->bool{false}
pub fn core_height(_x:i32,_z:i32)->i32{0}
pub fn fixtures()->impl Iterator<Item=((i32,i32,i32),&'static str)>{
 static BLOCKS:OnceLock<Vec<((i32,i32,i32),&'static str)>>=OnceLock::new();
 BLOCKS.get_or_init(grid::blocks).iter().copied()
}
fn pyramid(cx:f32,cz:f32,r:f32,h:f32,t:u16,out:&mut Vec<SourceTri>){
 let base=[Vec3::new(cx-r,0.01,cz-r),Vec3::new(cx+r,0.01,cz-r),Vec3::new(cx+r,0.01,cz+r),Vec3::new(cx-r,0.01,cz+r)];
 let top=[Vec3::new(cx-1.5,h,cz-1.5),Vec3::new(cx+1.5,h,cz-1.5),Vec3::new(cx+1.5,h,cz+1.5),Vec3::new(cx-1.5,h,cz+1.5)];
 let mut faces=Vec::new();
 for i in 0..4{faces.push(vec![base[i],base[(i+1)%4],top[(i+1)%4],top[i]]);}
 faces.push(top.to_vec());faces.push(base.to_vec());solid_cells::emit(faces,t,out);
}
fn quarter(cx:f32,cz:f32,width:f32,r:f32,sign:f32,t:u16,out:&mut Vec<SourceTri>){
 for j in 0..24 {
  let point=|i:usize,x:f32|{let a=i as f32/24.0*std::f32::consts::FRAC_PI_2;
   Vec3::new(cx+x,r*(1.0-a.cos())+0.01,cz+sign*r*a.sin())};
  let top=[point(j,-width/2.),point(j,width/2.),point(j+1,width/2.),point(j+1,-width/2.)];
  let base=top.map(|p|p.with_y(0.));
  let mut faces=vec![top.to_vec(),base.to_vec()];
  for i in 0..4{faces.push(vec![base[i],base[(i+1)%4],top[(i+1)%4],top[i]]);}
  solid_cells::emit(faces,t,out);
 }
 let z=cz+sign*r;
 solid_cells::prism(Vec3::new(cx-width/2.,0.,z.min(z+sign*1.)),
    Vec3::new(cx+width/2.,r+0.01,z.max(z+sign*1.)),t,out);
}
/// Authored board surfaces only, including relocated cells. Tower walls and
/// ordinary terrain never inherit the halfpipe contact behavior.
pub fn ramp_cell(place:usize,pos:(i32,i32,i32))->bool{
 if place!=SOURCE_ID{return false;}
 let Some((bank,col,cell))=crate::jak_pieces::piece_at(place,pos)else{return false;};
 if bank!=SOURCE_ID{return false;}
 static RAMP:OnceLock<u16>=OnceLock::new();
 let ramp=*RAMP.get_or_init(||crate::original_places::textures_of(&crate::original_places::PLACES[1]).iter().position(|t|*t=="skatea-ramp-01").unwrap() as u16);
 col.get(&cell).is_some_and(|p|p.tris.iter().any(|t|t.texture==ramp))
}
pub fn rail_lines()->Vec<Vec<[f64;3]>>{
 vec![vec![[-23.0,7.2,-15.0],[-23.0,7.2,5.0]]]
 .into_iter().map(|line:Vec<[f64;3]>|line.into_iter().map(|p|local_point(p.map(|v|v as f32)).to_array().map(f64::from)).collect()).collect()
}
fn geometry()->Option<&'static Source>{static SOURCE:OnceLock<Option<Source>>=OnceLock::new();SOURCE.get_or_init(||{
 let names0=crate::original_places::textures_of(&crate::original_places::PLACES[1]);let names:HashMap<_,_>=names0.iter().enumerate().map(|(i,&s)|(s,i as u16)).collect();
 let mut tris=Vec::new();
 let ramp=names["skatea-ramp-01"];let solid_first=tris.len();
 for y in [6.0]{let first=tris.len();quarter(-5.0,1.0,22.0,4.5,1.0,ramp,&mut tris);quarter(-5.0,-11.0,22.0,4.5,-1.0,ramp,&mut tris);
  pyramid(if y<10.0{-15.0}else{-5.0},-5.0,4.0,1.5,ramp,&mut tris);
  for t in &mut tris[first..]{t.p=t.p.map(|p|p+Vec3::Y*y);}
 }
 let solid_end=tris.len();
 let edge=names["skatea-grind-rail-01"];
 for line in rail_lines(){for p in line.windows(2){let a=Vec3::from_array(p[0].map(|v|v as f32))-CANONICAL;let b=Vec3::from_array(p[1].map(|v|v as f32))-CANONICAL;
 prism(a.min(b)-Vec3::new(0.08,0.16,0.08),a.max(b)+Vec3::new(0.08,0.0,0.08),edge,&mut tris);
 for c in [a,b]{let floor=if c.y>100.0{100.0}else if c.y>90.0{90.0}else{6.0};prism(Vec3::new(c.x-0.07,floor,c.z-0.07),Vec3::new(c.x+0.07,c.y-0.16,c.z+0.07),edge,&mut tris);}
 }}
 for t in &mut tris{t.p=t.p.map(|p|p+CANONICAL);}
 let mut source=Source::new(tris);source.own_cells(solid_first..solid_end);Some(source)
}).as_ref()}
pub(crate)fn source()->Option<&'static Source>{geometry()}
pub(crate)fn column(x:i32,z:i32)->Option<Arc<Column>>{static CACHE:OnceLock<Mutex<HashMap<(i32,i32),Arc<Column>>>>=OnceLock::new();
 let mut cache=CACHE.get_or_init(Default::default).lock().unwrap_or_else(|e|e.into_inner());
 if let Some(c)=cache.get(&(x,z)){return Some(c.clone());}let cut=Arc::new(crate::jak_pieces::cut_area(geometry()?,(x,x+15),(z,z+15)));cache.insert((x,z),cut.clone());Some(cut)}
pub(crate)fn collision_column(x:i32,z:i32)->Option<Arc<Column>>{column(x,z)}
pub fn prewarm_column(x:i32,z:i32){if let Some(c)=column(x,z){for p in c.pieces.values(){let _=p.collision_boxes();}}}
pub fn source_cell(place:usize,p:(i32,i32,i32))->bool{place==SOURCE_ID&&(AT..AT+SIZE).contains(&p.0)&&(MIN_Y..HEIGHT).contains(&p.1)&&(AT..AT+SIZE).contains(&p.2)}
pub fn to_world(c:(i32,i32,i32),p:(i32,i32,i32))->(i32,i32,i32){(c.0+p.0-CANONICAL.x as i32,c.1+p.1,c.2+p.2-CANONICAL.z as i32)}
pub fn ordinary_floor(x:i32,z:i32)->Option<&'static str>{
 ((x+5)*(x+5)+(z+5)*(z+5)<50*50||((-10..=0).contains(&x)&&(38..=65).contains(&z))||((0..=67).contains(&x)&&(-12..=-8).contains(&z))).then_some("jak:hv_t-citywide-wall-tile-01")
}
pub fn support_floor(x:i32,z:i32)->Option<i32>{ordinary_floor(x,z).map(|_|-1)}
pub fn air_column(x:i32,z:i32)->Option<(i32,i32)>{ordinary_floor(x,z).map(|_|(0,HEIGHT))}
static ORIGINS:OnceLock<Mutex<HashMap<i64,Vec<(i32,i32,i32)>>>>=OnceLock::new();
pub fn remember(seed:i64,c:(i32,i32,i32)){crate::jak_city_arena_v9::remember(seed,c);crate::jak_city_arena_legacy::remember(seed,c);let mut m=ORIGINS.get_or_init(Default::default).lock().unwrap();let a=m.entry(seed).or_default();if !a.contains(&c){a.push(c);}}
pub fn origins(seed:i64)->Vec<(i32,i32,i32)>{ORIGINS.get().and_then(|m|m.lock().ok()).and_then(|m|m.get(&seed).cloned()).unwrap_or_default()}
pub fn source_at_seed(seed:i64,p:(i32,i32,i32))->Option<(usize,(i32,i32,i32))>{origins(seed).into_iter().find_map(|c|{let q=(p.0-c.0+CANONICAL.x as i32,p.1-c.1,p.2-c.2+CANONICAL.z as i32);source_cell(SOURCE_ID,q).then_some((SOURCE_ID,q))})}
pub(crate)fn source_at(p:(i32,i32,i32))->Option<(usize,(i32,i32,i32))>{let seed=crate::original_places::current_world()?.0;
 if let Some(p)=source_at_seed(seed,p){return Some(p);}crate::jak_skate_tower_site::cold_lookup(p);source_at_seed(seed,p)}

/// Compatibility footprint query; the rebuilt arena has no enclosed basement.
pub fn basement_contains(x:i32,z:i32)->bool{ordinary_floor(x,z).is_some()}

