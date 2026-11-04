//! Native Haven palace silhouette with a grounded, block-adapted base and
//! interior JET-board circuits. Sources7/8 remain legacy; this bank is source9.
use std::{collections::HashMap,sync::{Arc,Mutex,OnceLock}};
use glam::{Vec2,Vec3};
use crate::jak_pieces::{Source,SourceTri,Column};
use crate::jak_items::{JakBlock,JakItem,JakKind,BlockLook};
pub const SOURCE_ID:usize=9;
pub const PIECE:&str="jak:pieza_haven_torre_circuito";
pub const AT:i32=8000;pub const SIZE:i32=160;pub const MIN_Y:i32=-2;pub const HEIGHT:i32=192;
pub const FOOTPRINT_MIN:[i32;2]=[-68,-80];pub const FOOTPRINT_MAX:[i32;2]=[92,80];
pub const ACCESS:[f64;3]=[-5.0,0.05,64.0];pub const ARRIVAL:[f64;3]=[-5.0,12.05,35.0];
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
pub const BLOCKS:[JakBlock;10]=[
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
];
pub const ITEMS:[JakItem;10]=[
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
fn ring(cx:f32,cz:f32,r0:f32,r1:f32,y0:f32,y1:f32,n:usize,t:u16,out:&mut Vec<SourceTri>){
 for i in 0..n{let point=|j:usize,r:f32,y:f32|{let a=j as f32/n as f32*std::f32::consts::TAU;Vec3::new(cx+r*a.cos(),y,cz+r*a.sin())};
 quad([point(i,r0,y0),point(i+1,r0,y0),point(i+1,r1,y1),point(i,r1,y1)],t,out);}
}
fn lid(cx:f32,cz:f32,r:f32,y:f32,n:usize,t:u16,out:&mut Vec<SourceTri>){
 for i in 0..n{let a=i as f32/n as f32*std::f32::consts::TAU;let b=(i+1)as f32/n as f32*std::f32::consts::TAU;
 let p=[Vec3::new(cx,y,cz),Vec3::new(cx+r*a.cos(),y,cz+r*a.sin()),Vec3::new(cx+r*b.cos(),y,cz+r*b.sin())];
 let p=[p[0],p[2],p[1]];out.push(SourceTri{p,uv:p.map(|p|Vec2::new(p.x,p.z)*0.5),texture:t,color:[[128;4];3],repeat:[true;2],blend:false});}
}
fn bottom(cx:f32,cz:f32,r:f32,y:f32,n:usize,t:u16,out:&mut Vec<SourceTri>){
 let first=out.len();lid(cx,cz,r,y,n,t,out);
 for tri in &mut out[first..]{tri.p.swap(1,2);tri.uv.swap(1,2);}
}
/// Winding is explicit for inner walls as well as the exterior.
fn oriented(p:[Vec3;4],normal:Vec3,t:u16,out:&mut Vec<SourceTri>){
 let n=(p[2]-p[0]).cross(p[1]-p[0]);
 quad(if n.dot(normal)>=0.0{p}else{[p[3],p[2],p[1],p[0]]},t,out);
}
#[path="jak_solid_cells.rs"] mod solid_cells;
const CENTER:Vec3=Vec3::new(-5.0,0.0,-5.0);
const PALACE_SCALE:f32=0.28;
// Lower wall contour measured from the original palace, not a circular stand.
// Its distant LOD stops at y=4.835. Continue that contour into real ground.
const PLINTH:[[f32;2];12]=[
 [-41.496,-31.585],[-31.172,-41.908],[-30.345,-41.964],[22.346,-42.101],
 [32.669,-31.777],[32.726,-31.023],[32.862,21.741],[22.539,32.065],
 [21.711,32.121],[-30.980,32.257],[-41.303,21.934],[-41.359,21.106],
];
fn plinth_point(i:usize,y:f32,offset:f32)->Vec3{
 let p=Vec3::new(PLINTH[i%PLINTH.len()][0],0.0,PLINTH[i%PLINTH.len()][1]);
 p+(p-CENTER).normalize()*offset+Vec3::Y*y
}
fn foundation(wall:u16,edge:u16,out:&mut Vec<SourceTri>){
 // Buried toe, bevelled foot, wall and upper collar overlap the native wall.
 let profile=[(-0.75,1.2),(0.35,1.2),(1.0,0.45),(4.35,0.45),(4.55,0.7),(5.08,0.7)];
 for pair in profile.windows(2){for i in 0..PLINTH.len(){
  let a=plinth_point(i,pair[0].0,pair[0].1);let b=plinth_point(i+1,pair[0].0,pair[0].1);
  let normal=Vec3::new(b.z-a.z,0.0,a.x-b.x).normalize();
  oriented([a,b,plinth_point(i+1,pair[1].0,pair[1].1),plinth_point(i,pair[1].0,pair[1].1)],normal,
   if pair[0].0>=4.35||pair[1].0<=1.0{edge}else{wall},out);
 }}
 for (y,offset,normal) in [(-0.75,1.2,-Vec3::Y),(5.08,0.7,Vec3::Y)]{for i in 0..PLINTH.len(){
  oriented([CENTER+Vec3::Y*y,plinth_point(i,y,offset),plinth_point(i+1,y,offset),CENTER+Vec3::Y*y],normal,wall,out);
 }}
}
/// Full Minecraft cubes inside the lower hull. Keep every cube inset from
/// the original facade; clipped facade cells are placed afterwards.
pub fn solid_base_column(x:i32,z:i32)->bool{
 [(x as f32,z as f32),(x as f32+1.0,z as f32),(x as f32,z as f32+1.0),(x as f32+1.0,z as f32+1.0)].iter().all(|&(px,pz)|{
  (0..PLINTH.len()).all(|i|{let[a,b]=[PLINTH[i],PLINTH[(i+1)%PLINTH.len()]];
   let(dx,dz)=(b[0]-a[0],b[1]-a[1]);
   dx*(pz-a[1])-dz*(px-a[0])>=1.1*(dx*dx+dz*dz).sqrt()
  })
 })
}
pub fn core_height(x:i32,z:i32)->i32{if (x+5)*(x+5)+(z+5)*(z+5)<22*22{6}else{12}}
fn floor_annulus(y:f32,inner:f32,outer:f32,t:u16,out:&mut Vec<SourceTri>){ceiling_annulus(y-0.36,inner,outer,t,out);}
fn floor_disk(y:f32,r:f32,t:u16,out:&mut Vec<SourceTri>){
 lid(-5.0,-5.0,r,y-0.01,64,t,out);bottom(-5.0,-5.0,r,y-0.4,64,t,out);
 ring(-5.0,-5.0,r,r,y-0.4,y-0.01,64,t,out);
}
fn ceiling_annulus(y:f32,inner:f32,outer:f32,t:u16,out:&mut Vec<SourceTri>){
 for i in 0..64{
  let p=|j:usize,r:f32,h:f32|{let a=j as f32/64.0*std::f32::consts::TAU;CENTER+Vec3::new(r*a.cos(),h,r*a.sin())};
  let q=[p(i,inner,y),p(i,outer,y),p(i+1,outer,y),p(i+1,inner,y)];
  oriented(q,-Vec3::Y,t,out);oriented(q.map(|p|p+Vec3::Y*0.35),Vec3::Y,t,out);
 }
 let first=out.len();ring(-5.0,-5.0,inner,inner,y,y+0.35,64,t,out);
 for tri in &mut out[first..]{tri.p.swap(1,2);tri.uv.swap(1,2);}
 ring(-5.0,-5.0,outer,outer,y,y+0.35,64,t,out);
}
fn route_radius(y:f32)->f32{
 if y<16.0 {24.0-(y-12.0)*3.375}
 else if y<76.0 {10.5}
 else if y<84.0 {10.5+(y-76.0)*1.65625}
 else {23.75}
}
pub fn ascent_point(t:f32)->[f32;3]{
 let y=12.0+88.0*t;let a=std::f32::consts::FRAC_PI_2+t*6.0*std::f32::consts::TAU;
 [-5.0+route_radius(y)*a.cos(),y,-5.0+route_radius(y)*a.sin()]
}
fn ascent(t:u16,rail:u16,out:&mut Vec<SourceTri>){
 const N:usize=1152;
 for i in 0..N {
  let point=|j:usize,offset:f32,dy:f32|{let f=j as f32/N as f32;let a=std::f32::consts::FRAC_PI_2+f*6.0*std::f32::consts::TAU;let y=12.0+88.0*f;
   Vec3::new(-5.0+(route_radius(y)+offset)*a.cos(),y+dy,-5.0+(route_radius(y)+offset)*a.sin())};
  let p=[point(i,-1.5,0.0),point(i,1.5,0.0),point(i+1,1.5,0.0),point(i+1,-1.5,0.0)];
  oriented(p,Vec3::Y,t,out);oriented(p.map(|p|p-Vec3::Y*0.35),-Vec3::Y,t,out);
  for (offset,sign) in [(-1.5,-1.0),(1.5,1.0)]{
   let a=point(i,offset,0.0);let b=point(i+1,offset,0.0);let radial=Vec3::new(a.x+5.0,0.0,a.z+5.0).normalize()*sign;
   oriented([a-Vec3::Y*0.35,b-Vec3::Y*0.35,b,a],radial,t,out);
   let landing=offset<0.0&&[12.0,90.0,100.0].iter().any(|y|(a.y-y).abs()<0.9);
   if !landing {
    let c=[a+Vec3::Y*0.1,b+Vec3::Y*0.1,b+Vec3::Y*0.95,a+Vec3::Y*0.95];
    oriented(c,radial,rail,out);oriented(c.map(|p|p-radial*0.10),-radial,rail,out);
    oriented([c[3],c[2],c[2]-radial*0.10,c[3]-radial*0.10],Vec3::Y,rail,out);
   }
  }
 }
}
// Convex room subtraction preserves the original exterior, UVs and silhouette.
// Work on source triangles before the regular one-metre cell clipping.
#[derive(Clone,Copy)]struct CutVertex{p:Vec3,uv:Vec2,c:glam::Vec4}
fn carve(input:Vec<SourceTri>,planes:&[(Vec3,f32)])->Vec<SourceTri>{
 let mut out=Vec::new();
 for t in input {
  let mut inside:Vec<_>=(0..3).map(|k|CutVertex{p:t.p[k],uv:t.uv[k],c:glam::Vec4::from_array(t.color[k].map(f32::from))}).collect();
  for &(normal,limit) in planes {
   if inside.len()<3{break;}
   let(mut kept,mut rest)=(Vec::new(),Vec::new());
   for i in 0..inside.len(){let(a,b)=(inside[i],inside[(i+1)%inside.len()]);let(da,db)=(a.p.dot(normal)-limit,b.p.dot(normal)-limit);
    if da>=0.0{kept.push(a);}if da<=0.0{rest.push(a);}
    if da*db<0.0{let f=da/(da-db);let v=CutVertex{p:a.p.lerp(b.p,f),uv:a.uv.lerp(b.uv,f),c:a.c.lerp(b.c,f)};kept.push(v);rest.push(v);}
   }
   for k in 1..kept.len().saturating_sub(1){let v=[kept[0],kept[k],kept[k+1]];let p=v.map(|v|v.p);
    if (p[1]-p[0]).cross(p[2]-p[0]).length_squared()>1e-10 {out.push(SourceTri{p,uv:v.map(|v|v.uv),color:v.map(|v|v.c.round().to_array().map(|c|c.clamp(0.0,255.0)as u8)),..t});}
   }inside=rest;
  }
 }out
}
fn room(input:Vec<SourceTri>,lo:f32,hi:f32,r0:f32,r1:f32)->Vec<SourceTri>{
 let slope=(r1-r0)/(hi-lo);let mut planes=vec![(-Vec3::Y,-lo),(Vec3::Y,hi)];
 for i in 0..64 {let a=i as f32/64.0*std::f32::consts::TAU;let radial=Vec3::new(a.cos(),0.0,a.sin());
  planes.push((radial-Vec3::Y*slope,radial.dot(CENTER)+r0-slope*lo));}
 carve(input,&planes)
}
fn palace_shell(names:&HashMap<&str,u16>)->Option<Vec<SourceTri>>{
 let root=crate::original_places::levels_root("jak2")?;let mut native=Vec::new();
 crate::original_places::read_level_detailed(&root.join("ctywide/ctywide-background.glb"),names,[10.0,382.0,410.0,780.0,25.0,670.0],&mut native).ok()?;
 let textures=crate::original_places::textures_of(&crate::original_places::PLACES[1]);
 // The cable planes extend toward other districts; they are not tower structure.
 native.retain(|t|{
  let name=textures[t.texture as usize];
  let cable_support=matches!(name,"t-palshaft-dirt-blue-01"|"t-citywide-met-pill-01") && t.p.iter().all(|p|p.y>=338.0&&p.y<=394.0);
  name!="t-cab"&&!cable_support
 });
 crate::jak_skate_tower::normalize_opaque(&mut native,&textures);
 let pivot=Vec3::new(189.56,26.246906,599.72);
 for t in &mut native {
  // The distant scenery fades whole building panels with vertex alpha;
  // every selected palace texture is opaque (native alpha128). A usable
  // building writes depth even where the original distant LOD faded out.
  t.blend=false;t.color=[[128;4];3];
  let c=(t.p[0]+t.p[1]+t.p[2])/3.0;let n=(t.p[1]-t.p[0]).cross(t.p[2]-t.p[0]).normalize_or_zero();
  if c.y<300.0 && n.y.abs()<0.8 && n.dot(Vec3::new(c.x-pivot.x,0.0,c.z-pivot.z))<0.0 {t.p.swap(1,2);t.uv.swap(1,2);}
  // Shared Villa Refugio plaster on broad facade panels. Preserve the
  // original roofs, red bands, windows and metal horns that define Praxis's silhouette.
  if textures[t.texture as usize]=="t-palshaft-dirt-blue-01" && n.y.abs()<0.25 {
      if let Some(&plaster)=names.get("city-slumwall-06"){t.texture=plaster;}
  }
  t.p=t.p.map(|p|(p-pivot)*PALACE_SCALE+CENTER);
 }
 // Closed thin shells make the original distant mesh usable from both sides,
 // with visible and physical triangles following exactly the same surface.
 let mut out=Vec::with_capacity(native.len()*8);
 for t in native {let n=(t.p[1]-t.p[0]).cross(t.p[2]-t.p[0]).normalize_or_zero();out.push(t);
  let inner=t.p.map(|p|p-n*0.12);out.push(SourceTri{p:[inner[2],inner[1],inner[0]],uv:[t.uv[2],t.uv[1],t.uv[0]],..t});
  for k in 0..3{let j=(k+1)%3;for (p,uv) in [([t.p[k],inner[k],inner[j]],[t.uv[k],t.uv[k],t.uv[j]]),([t.p[k],inner[j],t.p[j]],[t.uv[k],t.uv[j],t.uv[j]])]{out.push(SourceTri{p,uv,..t});}}
 }
 Some(out)
}
fn tower(names:&HashMap<&str,u16>,out:&mut Vec<SourceTri>)->Option<()> {
 let wall=names["t-citywide-wall-tile-01"];let metal=names["t-palshaft-pil-01"];let floor=names["skatea-metalfloor-01"];
 let mut shell=palace_shell(names)?;
 // Rooms fit the measured native plinth, narrow shaft and upper bowl.
 for (lo,hi,r0,r1) in [(5.9,18.5,29.0,29.0),(18.5,76.0,12.5,12.5),(76.0,84.0,12.5,26.0),(84.0,104.0,26.0,26.0),(104.0,110.5,19.0,19.0),(110.5,122.0,10.0,10.0)] {shell=room(shell,lo,hi,r0,r1);}
 out.extend(shell);
 // An inner lining closes the cut spaces without replacing the native facade.
 for (lo,hi,r0,r1) in [(10.0,18.5,29.0,29.0),(18.5,76.0,12.5,12.5),(76.0,84.0,12.5,26.0),(84.0,104.0,26.0,26.0)] {
  let first=out.len();ring(-5.0,-5.0,r0,r1,lo,hi,64,wall,out);
  for t in &mut out[first..]{t.p.swap(1,2);t.uv.swap(1,2);}
 }
 foundation(wall,metal,out);
 // The lower skate bowl is sunk six metres below the entrance gallery.
 // Every landing now has room for a board jump, not just a standing actor.
 floor_disk(6.0,22.1,floor,out);floor_annulus(12.0,22.0,29.12,floor,out);
 let first=out.len();ring(-5.0,-5.0,22.0,22.0,6.0,12.0,64,wall,out);
 for t in &mut out[first..]{t.p.swap(1,2);t.uv.swap(1,2);}
 // Wide access down to the bowl, outside the halfpipe riding lines.
 for i in 0..48{let a=i as f32/48.0;let b=(i+1)as f32/48.0;
  let p=[Vec3::new(-8.0,12.0-6.0*a,17.0-8.0*a),Vec3::new(-2.0,12.0-6.0*a,17.0-8.0*a),Vec3::new(-2.0,12.0-6.0*b,17.0-8.0*b),Vec3::new(-8.0,12.0-6.0*b,17.0-8.0*b)];
  oriented(p,Vec3::Y,floor,out);oriented(p.map(|p|p-Vec3::Y*0.4),-Vec3::Y,metal,out);
  for k in [0,1]{let q=[p[k],p[3-k],p[3-k]-Vec3::Y*0.4,p[k]-Vec3::Y*0.4];oriented(q,if k==0{-Vec3::X}else{Vec3::X},metal,out);}
 }
 floor_disk(90.0,23.0,floor,out);floor_annulus(100.0,19.0,23.0,floor,out);
 // Close the shoulders between the broad base room and narrow shaft.
 // The central opening exactly matches the shaft; the ascent stays inside it.
 ceiling_annulus(18.5,12.5,29.12,metal,out);
 // The crown room ends below the native spires, with a physical ceiling.
 floor_annulus(104.1,19.0,26.12,metal,out);
 upper_palace(floor,wall,metal,out);
 // Original front entrance: ordinary operable KG doors in a walkable passage.
 let mut all=std::mem::take(out);
 all=crate::jak_skate_tower::subtract_box(all,Vec3::new(-7.5,12.0,19.0),Vec3::new(-2.5,15.3,40.0));
 // A viewing opening in the crown leaves the outline and its curved arms intact.
 all=crate::jak_skate_tower::subtract_box(all,Vec3::new(-8.0,100.1,19.0),Vec3::new(-2.0,103.0,24.0));
 *out=all;
 prism(Vec3::new(-7.5,11.6,18.0),Vec3::new(-2.5,11.99,40.0),floor,out);
 prism(Vec3::new(-7.7,12.0,19.0),Vec3::new(-7.5,15.5,40.0),metal,out);
 prism(Vec3::new(-2.5,12.0,19.0),Vec3::new(-2.3,15.5,40.0),metal,out);
 prism(Vec3::new(-7.7,15.3,19.0),Vec3::new(-2.3,15.5,40.0),metal,out);
 // Reveal the thickness of the crown wall instead of exposing an open cut.
 // Native front wall is at z=23.414; the sill connects its opening to the deck.
 prism(Vec3::new(-8.15,99.75,17.5),Vec3::new(-1.85,100.09,23.55),floor,out);
 prism(Vec3::new(-8.15,100.09,19.0),Vec3::new(-8.0,103.15,23.55),metal,out);
 prism(Vec3::new(-2.0,100.09,19.0),Vec3::new(-1.85,103.15,23.55),metal,out);
 prism(Vec3::new(-8.15,103.0,19.0),Vec3::new(-1.85,103.15,23.55),metal,out);
 prism(Vec3::new(-8.0,100.09,23.35),Vec3::new(-2.0,101.0,23.5),metal,out);
 // Outside approach reaches the original raised plinth from ground level.
 oriented([Vec3::new(-8.0,12.0,39.0),Vec3::new(-2.0,12.0,39.0),Vec3::new(-2.0,0.0,63.0),Vec3::new(-8.0,0.0,63.0)],Vec3::Y,floor,out);
 for x in [-8.0,-2.0]{oriented([Vec3::new(x,0.0,39.0),Vec3::new(x,12.0,39.0),Vec3::new(x,0.0,63.0),Vec3::new(x,0.0,63.0)],if x< -5.0{-Vec3::X}else{Vec3::X},metal,out);}
 oriented([Vec3::new(-8.0,0.0,39.0),Vec3::new(-2.0,0.0,39.0),Vec3::new(-2.0,12.0,39.0),Vec3::new(-8.0,12.0,39.0)],-Vec3::Z,metal,out);
 oriented([Vec3::new(-8.0,0.0,39.0),Vec3::new(-2.0,0.0,39.0),Vec3::new(-2.0,0.0,63.0),Vec3::new(-8.0,0.0,63.0)],-Vec3::Y,metal,out);
 // Closed low parapets follow the approach, leaving its six-metre lane free.
 for x in [-8.22,-2.0]{
  let p=[Vec3::new(x,12.0,39.0),Vec3::new(x,0.0,63.0),Vec3::new(x,0.8,63.0),Vec3::new(x,12.8,39.0)];
  oriented(p,-Vec3::X,metal,out);oriented(p.map(|p|p+Vec3::X*0.22),Vec3::X,metal,out);
  oriented([p[3],p[2],p[2]+Vec3::X*0.22,p[3]+Vec3::X*0.22],Vec3::Y,metal,out);
  for (a,b,n) in [(p[0],p[3],-Vec3::Z),(p[1],p[2],Vec3::Z)]{oriented([a,b,b+Vec3::X*0.22,a+Vec3::X*0.22],n,metal,out);}
 }
 ascent(names["skatea-ramp-01"],metal,out);
 // Tangential landing catches the last steps and joins the crown's floor;
 // the spiral must not simply end over the previous lap.
 prism(Vec3::new(-9.0,99.6,15.5),Vec3::new(-3.0,99.99,20.5),floor,out);
 prism(Vec3::new(-9.0,100.0,20.4),Vec3::new(-3.0,100.95,20.5),metal,out);
 for i in 0..48 {let p=ascent_point(i as f32/48.0);let y=p[1].floor()+4.08;
  let r=if y<18.5{29.0}else if y<76.0{12.5}else if y<84.0{12.5+(y-76.0)*1.6875}else{26.0};
  let radial=Vec3::new(p[0]+5.0,0.0,p[2]+5.0).normalize();let end=CENTER+radial*r+Vec3::Y*y;
  let lamp=Vec3::new(p[0].floor()+0.5,y,p[2].floor()+0.5);
  prism(lamp.min(end)-Vec3::new(0.09,0.0,0.09),lamp.max(end)+Vec3::new(0.09,0.12,0.09),metal,out);
 }
 Some(())
}
pub fn fixtures()->Vec<((i32,i32,i32),&'static str)>{
 let mut out=Vec::new();
 // Central landing, between the halfpipes; the native board floats above its pedestal.
 out.push(((-5,6,-5),"jak:altar_turbotabla"));
 for (y,r,inner) in [(6,21,0),(12,27,23),(90,21,0),(100,22,20),(104,18,5),(116,9,5)]{for x in (-30i32..=20).step_by(5){for z in (-30i32..=20).step_by(5){let d=(x+5)*(x+5)+(z+5)*(z+5);if d<r*r&&d>=inner*inner{out.push(((x,y-1,z),"jak:arena_luz_placa"));}}}}
 // Symmetric lights embedded in the solid native facade, facing the city.
 for y in [7,14]{for x in [-29,-17,7,19]{out.push(((x,y,32),"jak:krimson_luz"));}}
 for y in [32,48,64]{for x in [-12,2]{out.push(((x,y,7),"jak:krimson_luz"));}}
 for y in [94,102]{for x in [-17,7]{out.push(((x,y,15),"jak:krimson_luz"));}}
 // Operable doors separate the palace chambers from the central gallery.
 for z in [-12,1]{for x in [-13,2]{out.push(((x,104,z),"jak:krimson_porton_fence_gate[facing=east,in_wall=false,open=false,powered=false]"));}}
 for (x,z) in [(-15,-14),(-15,4),(7,-14),(7,4)]{out.push(((x,104,z),"minecraft:chest[facing=south,type=single,waterlogged=false]"));}
 for (x,z) in [(-17,-5),(7,-5)]{out.push(((x,104,z),"jak:krimson_terminal"));}
 for z in [-14,4]{for x in [-16,5]{for y in 104..108{out.push(((x,y,z),"jak:palacio_detalle_columna"));}
  for dx in -1..=1{out.push(((x+dx,108,z),"jak:palacio_detalle_cornisa"));}
 }}
 for x in -7..-3{out.push(((x,12,37),"jak:krimson_porton_fence_gate[facing=south,in_wall=false,open=false,powered=false]"));}
 for i in 0..48{let p=ascent_point(i as f32/48.0);out.push(((p[0].floor()as i32,p[1].floor()as i32+3,p[2].floor()as i32),"minecraft:lantern[hanging=true,waterlogged=false]"));}
 out
}
/// Palace rooms use the original outer shell. A five-turn central stair
/// joins the existing crown gallery to furnished chambers and the roof.
fn upper_palace(floor:u16,wall:u16,metal:u16,out:&mut Vec<SourceTri>){
 for (y,r) in [(104.0,19.0),(116.0,10.0),(122.0,10.0)]{floor_annulus(y,5.0,r,floor,out);}
 for (lo,hi,r) in [(104.0,110.5,19.0),(110.5,122.0,10.0)]{
  let first=out.len();ring(-5.0,-5.0,r,r,lo,hi,64,wall,out);
  for t in &mut out[first..]{t.p.swap(1,2);t.uv.swap(1,2);}
 }
 // Close the shoulder over the first floor while leaving the stair open.
 ceiling_annulus(110.5,10.0,19.0,metal,out);
 // Thick partitions, headers and door reveals; chambers remain habitable.
 for x in [-13.0,2.0]{for (z0,z1) in [(-17.0,-12.0),(-11.0,1.0),(2.0,7.0)]{
  prism(Vec3::new(x,104.0,z0),Vec3::new(x+0.3,110.5,z1),wall,out);
 }for z in [-12.0,1.0]{prism(Vec3::new(x,107.0,z),Vec3::new(x+0.3,110.5,z+1.0),metal,out);}}
 const N:usize=600;
 for i in 0..N{let point=|j:usize,r:f32,h:f32|{let t=j as f32/N as f32;let a=t*5.0*std::f32::consts::TAU;
  CENTER+Vec3::new(r*a.cos(),100.0+22.0*t+h,r*a.sin())};
  let p=[point(i,2.0,0.0),point(i,5.1,0.0),point(i+1,5.1,0.0),point(i+1,2.0,0.0)];
  oriented(p,Vec3::Y,floor,out);oriented(p.map(|p|p-Vec3::Y*0.3),-Vec3::Y,metal,out);
  for r in [2.0,5.1]{let a=point(i,r,0.0);let b=point(i+1,r,0.0);let n=(a-CENTER).with_y(0.0).normalize();
   oriented([a,b,b-Vec3::Y*0.3,a-Vec3::Y*0.3],n,metal,out);
   let landing=[100.0,104.0,116.0,122.0].iter().any(|y|(a.y-y).abs()<0.7);
   if r<3.0||!landing {let q=[a,b,b+Vec3::Y*0.9,a+Vec3::Y*0.9];oriented(q,n,metal,out);oriented(q.map(|p|p-n*0.08),-n,metal,out);
    oriented([q[3],q[2],q[2]-n*0.08,q[3]-n*0.08],Vec3::Y,metal,out);}
  }
 }
 // Approach from the crown gallery without crossing the open skate atrium.
 prism(Vec3::new(-1.0,99.6,-6.0),Vec3::new(16.0,99.99,-4.0),floor,out);
 for z in [-6.0,-4.0]{prism(Vec3::new(0.0,100.0,z),Vec3::new(16.0,100.9,z+0.1),metal,out);}
 // Roof parapet and radial landing make the upper palace genuinely reachable.
 ring(-5.0,-5.0,10.0,10.0,122.0,122.95,64,metal,out);
 let first=out.len();ring(-5.0,-5.0,9.9,9.9,122.0,122.95,64,metal,out);
 for t in &mut out[first..]{t.p.swap(1,2);t.uv.swap(1,2);}
 floor_annulus(123.0,9.9,10.0,metal,out);
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
 solid_cells::prism(Vec3::new(cx-width/2.,0.,z.min(z+sign*2.)),
    Vec3::new(cx+width/2.,r+0.01,z.max(z+sign*2.)),t,out);
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
 vec![vec![[-23.0,7.2,-15.0],[-23.0,7.2,5.0]],vec![[13.0,91.2,-15.0],[13.0,91.2,5.0]],vec![[-23.0,101.2,-15.0],[-23.0,101.2,5.0]]]
 .into_iter().map(|line:Vec<[f64;3]>|line.into_iter().map(|p|local_point(p.map(|v|v as f32)).to_array().map(f64::from)).collect()).collect()
}
fn geometry()->Option<&'static Source>{static SOURCE:OnceLock<Option<Source>>=OnceLock::new();SOURCE.get_or_init(||{
 let names0=crate::original_places::textures_of(&crate::original_places::PLACES[1]);let names:HashMap<_,_>=names0.iter().enumerate().map(|(i,&s)|(s,i as u16)).collect();
 let mut tris=Vec::new();tower(&names,&mut tris)?;
 let ramp=names["skatea-ramp-01"];let solid_first=tris.len();
 for y in [6.0,90.0]{let first=tris.len();quarter(-5.0,1.0,22.0,4.5,1.0,ramp,&mut tris);quarter(-5.0,-11.0,22.0,4.5,-1.0,ramp,&mut tris);
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
pub fn remember(seed:i64,c:(i32,i32,i32)){crate::jak_city_arena_legacy::remember(seed,c);let mut m=ORIGINS.get_or_init(Default::default).lock().unwrap();let a=m.entry(seed).or_default();if !a.contains(&c){a.push(c);}}
pub fn origins(seed:i64)->Vec<(i32,i32,i32)>{ORIGINS.get().and_then(|m|m.lock().ok()).and_then(|m|m.get(&seed).cloned()).unwrap_or_default()}
pub fn source_at_seed(seed:i64,p:(i32,i32,i32))->Option<(usize,(i32,i32,i32))>{origins(seed).into_iter().find_map(|c|{let q=(p.0-c.0+CANONICAL.x as i32,p.1-c.1,p.2-c.2+CANONICAL.z as i32);source_cell(SOURCE_ID,q).then_some((SOURCE_ID,q))})}
pub(crate)fn source_at(p:(i32,i32,i32))->Option<(usize,(i32,i32,i32))>{let seed=crate::original_places::current_world()?.0;
 if let Some(p)=source_at_seed(seed,p){return Some(p);}crate::jak_skate_tower_site::cold_lookup(p);source_at_seed(seed,p)}

/// Compatibility footprint query; the rebuilt arena has no enclosed basement.
pub fn basement_contains(x:i32,z:i32)->bool{ordinary_floor(x,z).is_some()}

