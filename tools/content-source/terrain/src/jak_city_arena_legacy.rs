//! Block-adapted Haven landmark and open JET-board arena. The native distant
//! palace/entrance stays in legacy source7; this editable bank is source8.
use std::{collections::HashMap,sync::{Arc,Mutex,OnceLock}};
use glam::{Vec2,Vec3};
use crate::jak_pieces::{Source,SourceTri,Column};
use crate::jak_items::{JakBlock,JakItem,JakKind,BlockLook};
pub const SOURCE_ID:usize=8;
pub const PIECE:&str="jak:pieza_haven_arena";
pub const AT:i32=8000;pub const SIZE:i32=160;pub const MIN_Y:i32=-2;pub const HEIGHT:i32=128;
pub const FOOTPRINT_MIN:[i32;2]=[-68,-80];pub const FOOTPRINT_MAX:[i32;2]=[92,80];
pub const ACCESS:[f64;3]=[66.0,0.05,-10.0];pub const ARRIVAL:[f64;3]=[44.0,0.05,-10.0];
pub const BASEMENT_FLOOR_Y:i32=0;
const CANONICAL:Vec3=Vec3::new(8068.0,0.0,8080.0);
pub const BLOCKS:[JakBlock;4]=[
 JakBlock{path:"pieza_haven_arena",look:BlockLook::Slab{texture:"skatea-metalfloor-01",double:"pieza_haven_arena"}},
 JakBlock{path:"arena_suelo",look:BlockLook::All("skatea-metalfloor-01")},
 JakBlock{path:"arena_borde",look:BlockLook::All("skatea-metal-edge-01")},
 JakBlock{path:"arena_losa",look:BlockLook::All("skatea-marble-floor-01")},
];
pub const ITEMS:[JakItem;4]=[
 JakItem{path:"pieza_haven_arena",title:"Pieza de rampa o torre adaptada",kind:JakKind::Block},
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
fn tower(names:&HashMap<&str,u16>,out:&mut Vec<SourceTri>){
 let wall=names["t-citywide-wall-tile-01"];let panel=names["t-palshaft-panl-01"];
 let trim=names["t-citypal-met-bm-red-strp01"];let metal=names["t-palshaft-pil-01"];
 let window=names["citywide-window-litwindow"];let(cx,cz)=(-30.0,-16.0);
 // Closed octagonal masonry base, accessible terraces and the native palace
 // silhouette's inverted crown. Repeated 2m UVs keep material scale beside Jak.
 for (r0,r1,y0,y1,t) in [(22.0,22.0,0.0,8.0,wall),(16.0,14.0,8.0,66.0,panel),
 (14.0,28.0,66.0,78.0,wall),(28.0,28.0,78.0,85.0,panel)]{
 ring(cx,cz,r0,r1,y0,y1,12,t,out);
 }
 bottom(cx,cz,22.0,0.0,12,wall,out);ring(cx,cz,22.0,16.0,8.0,8.0,12,wall,out);lid(cx,cz,28.0,85.0,12,panel,out);
 for (y,r) in [(2.0,22.3),(7.0,22.4),(12.0,16.2),(24.0,15.8),(36.0,15.4),(48.0,15.0),(60.0,14.6),(78.0,28.3),(84.0,28.4)]{
 ring(cx,cz,r,r,y,y+0.6,12,trim,out);ring(cx,cz,r,r-0.35,y+0.6,y+0.6,12,trim,out);
 }
 // Real piers, recessed lit windows, crown buttresses, fins and tall caps.
 for i in 0..12{let a=i as f32/12.0*std::f32::consts::TAU;let d=Vec3::new(a.cos(),0.0,a.sin());
 for y in (12..65).step_by(6){let r=16.0-(y as f32-8.0)/58.0*2.0;let c=Vec3::new(cx,y as f32,cz)+d*r;
 prism(c-Vec3::new(0.65,0.0,0.65),c+Vec3::new(0.65,2.0,0.65),window,out);}
 let c=Vec3::new(cx,8.0,cz)+d*16.2;prism(c-Vec3::new(0.55,0.0,0.55),c+Vec3::new(0.55,57.0,0.55),metal,out);
 let c=Vec3::new(cx,84.0,cz)+d*22.0;ring(c.x,c.z,2.1,2.1,84.0,96.0,4,panel,out);
 ring(c.x,c.z,2.4,0.0,96.0,105.0,4,trim,out);bottom(c.x,c.z,2.1,84.0,4,panel,out);
 // Curved, outward-pointing crown fins, closed across their thickness.
 let mut last=Vec3::new(cx,77.0,cz)+d*27.0;
 let side=Vec3::new(-d.z,0.0,d.x);
 for j in 1..=6{let t=j as f32/6.0;let t0=(j-1)as f32/6.0;
 let next=Vec3::new(cx,77.0+20.0*t*t,cz)+d*(27.0+8.0*t);
 let a=side*(0.65*(1.0-t0)+0.12);let b=side*(0.65*(1.0-t)+0.12);
 let back0=d*(0.5*(1.0-t0)+0.08);let back1=d*(0.5*(1.0-t)+0.08);
 // A closed tapered segment, with both sides and end faces. No single-sided fins.
 for p in [[last-a,last+a,next+b,next-b],
 [last+a-back0,last-a-back0,next-b-back1,next+b-back1],
 [last-a-back0,last-a,next-b,next-b-back1],
 [last+a,last+a-back0,next+b-back1,next+b],
 [last-a-back0,last+a-back0,last+a,last-a],
 [next-b,next+b,next+b-back1,next-b-back1]]{quad(p,metal,out);}
 last=next;}

 }
 ring(cx,cz,7.0,7.0,85.0,109.0,8,panel,out);bottom(cx,cz,7.0,85.0,8,panel,out);
 ring(cx,cz,7.4,0.0,109.0,123.0,8,trim,out);
 // Human-scale entrance framing and original palace panels, not an invented emblem.
 prism(Vec3::new(-36.0,0.0,6.1),Vec3::new(-24.0,7.5,6.6),metal,out);
 prism(Vec3::new(-34.6,0.0,6.65),Vec3::new(-25.4,6.2,6.85),names["citywide-palace-01"],out);
}
fn pyramid(cx:f32,cz:f32,r:f32,h:f32,t:u16,out:&mut Vec<SourceTri>){
 let base=[Vec3::new(cx-r,0.01,cz-r),Vec3::new(cx+r,0.01,cz-r),Vec3::new(cx+r,0.01,cz+r),Vec3::new(cx-r,0.01,cz+r)];
 let top=[Vec3::new(cx-1.5,h,cz-1.5),Vec3::new(cx+1.5,h,cz-1.5),Vec3::new(cx+1.5,h,cz+1.5),Vec3::new(cx-1.5,h,cz+1.5)];
 for i in 0..4{quad([base[i],base[(i+1)%4],top[(i+1)%4],top[i]],t,out);}quad(top,t,out);quad([base[3],base[2],base[1],base[0]],t,out);
}
fn quarter(cx:f32,cz:f32,width:f32,r:f32,sign:f32,t:u16,out:&mut Vec<SourceTri>){
 for j in 0..24{let point=|i:usize,x:f32|{let a=i as f32/24.0*std::f32::consts::FRAC_PI_2;
 Vec3::new(cx+x,r*(1.0-a.cos())+0.01,cz+sign*r*a.sin())};
 let p=[point(j,-width/2.0),point(j,width/2.0),point(j+1,width/2.0),point(j+1,-width/2.0)];
 quad(if sign>0.0 {p}else{[p[3],p[2],p[1],p[0]]},t,out);
 for x in [-width/2.0,width/2.0]{let p=[Vec3::new(cx+x,0.01,point(j,x).z),point(j,x),point(j+1,x),Vec3::new(cx+x,0.01,point(j+1,x).z)];
 let raw=(p[2]-p[0]).cross(p[1]-p[0]);
 quad(if raw.x*x>0.0{p}else{[p[3],p[2],p[1],p[0]]},t,out);}
 let a=point(j,-width/2.0);let b=point(j,width/2.0);let c=point(j+1,width/2.0);let d=point(j+1,-width/2.0);
 let p=[Vec3::new(a.x,0.01,a.z),Vec3::new(b.x,0.01,b.z),Vec3::new(c.x,0.01,c.z),Vec3::new(d.x,0.01,d.z)];
 quad(if sign>0.0{[p[3],p[2],p[1],p[0]]}else{p},t,out);
 }let z=cz+sign*r;
 let p=[Vec3::new(cx-width/2.0,0.01,z),Vec3::new(cx+width/2.0,0.01,z),Vec3::new(cx+width/2.0,r+0.01,z),Vec3::new(cx-width/2.0,r+0.01,z)];
 quad(if sign>0.0{[p[3],p[2],p[1],p[0]]}else{p},t,out);
 prism(Vec3::new(cx-width/2.0,r-0.25,z.min(z+sign*2.0)),Vec3::new(cx+width/2.0,r+0.01,z.max(z+sign*2.0)),t,out);
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
 vec![vec![[24.0,1.20,-9.0],[24.0,1.20,15.0]],vec![[77.0,1.20,-14.0],[77.0,1.20,16.0]],
 vec![[37.0,0.9,39.0],[49.0,0.9,39.0]],vec![[32.0,2.1,-50.0],[52.0,2.1,-50.0]]]
 .into_iter().map(|line:Vec<[f64;3]>|line.into_iter().map(|p|(local_point(p.map(|v|v as f32))).to_array().map(f64::from)).collect()).collect()
}
fn geometry()->Option<&'static Source>{static SOURCE:OnceLock<Option<Source>>=OnceLock::new();SOURCE.get_or_init(||{
 let names0=crate::original_places::textures_of(&crate::original_places::PLACES[1]);let names:HashMap<_,_>=names0.iter().enumerate().map(|(i,&s)|(s,i as u16)).collect();
 let mut tris=Vec::new();tower(&names,&mut tris);
 let ramp=names["skatea-ramp-01"];let edge=names["skatea-grind-rail-01"];
 pyramid(47.0,2.0,8.0,3.7,ramp,&mut tris);pyramid(67.0,-34.0,6.5,3.0,ramp,&mut tris);
 // Opposing quarters form an open halfpipe; long run-ups stay unobstructed.
 quarter(48.0,46.0,22.0,7.0,1.0,ramp,&mut tris);quarter(48.0,35.0,22.0,7.0,-1.0,ramp,&mut tris);
 quarter(44.0,-58.0,24.0,7.5,-1.0,ramp,&mut tris);quarter(74.0,49.0,15.0,5.0,1.0,ramp,&mut tris);
 for line in rail_lines(){for p in line.windows(2){let a=Vec3::from_array(p[0].map(|v|v as f32))-CANONICAL;let b=Vec3::from_array(p[1].map(|v|v as f32))-CANONICAL;
 let lo=a.min(b);let hi=a.max(b);prism(lo-Vec3::new(0.08,0.16,0.08),hi+Vec3::new(0.08,0.0,0.08),edge,&mut tris);
 for c in [a,b]{prism(Vec3::new(c.x-0.07,0.01,c.z-0.07),Vec3::new(c.x+0.07,c.y-0.16,c.z+0.07),edge,&mut tris);}
 }}
 for t in &mut tris{t.p=t.p.map(|p|p+CANONICAL);}
 Some(Source::new(tris))
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
 if (12..90).contains(&x)&&(-72..72).contains(&z){Some(if x<14||x>=88||z< -70||z>=70{"jak:arena_borde"}else if (x.div_euclid(8)+z.div_euclid(8)).rem_euclid(3)==0{"jak:arena_losa"}else{"jak:arena_suelo"})}
 else if (x+30)*(x+30)+(z+16)*(z+16)<29*29{Some("jak:hv_t-citywide-wall-tile-01")}else{None}
}
pub fn support_floor(x:i32,z:i32)->Option<i32>{ordinary_floor(x,z).map(|_|-1)}
pub fn air_column(x:i32,z:i32)->Option<(i32,i32)>{ordinary_floor(x,z).map(|_|(0,16))}
static ORIGINS:OnceLock<Mutex<HashMap<i64,Vec<(i32,i32,i32)>>>>=OnceLock::new();
pub fn remember(seed:i64,c:(i32,i32,i32)){crate::jak_skate_tower::remember(seed,c);let mut m=ORIGINS.get_or_init(Default::default).lock().unwrap();let a=m.entry(seed).or_default();if !a.contains(&c){a.push(c);}}
pub fn origins(seed:i64)->Vec<(i32,i32,i32)>{ORIGINS.get().and_then(|m|m.lock().ok()).and_then(|m|m.get(&seed).cloned()).unwrap_or_default()}
pub fn source_at_seed(seed:i64,p:(i32,i32,i32))->Option<(usize,(i32,i32,i32))>{origins(seed).into_iter().find_map(|c|{let q=(p.0-c.0+CANONICAL.x as i32,p.1-c.1,p.2-c.2+CANONICAL.z as i32);source_cell(SOURCE_ID,q).then_some((SOURCE_ID,q))})}
pub(crate)fn source_at(p:(i32,i32,i32))->Option<(usize,(i32,i32,i32))>{let seed=crate::original_places::current_world()?.0;
 if let Some(p)=source_at_seed(seed,p){return Some(p);}crate::jak_skate_tower_site::cold_lookup(p);source_at_seed(seed,p)}

/// Compatibility footprint query; the rebuilt arena has no enclosed basement.
pub fn basement_contains(x:i32,z:i32)->bool{ordinary_floor(x,z).is_some()}

#[cfg(test)]mod revision_tests {
 use super::*;
 #[test]fn revision_open_arena_has_full_scale_upward_ramps_and_portable_rails(){
  let source=geometry().expect("original texture bank");assert!(!source.tris.is_empty());
  let ramp=crate::original_places::textures_of(&crate::original_places::PLACES[1]).iter().position(|n|*n=="skatea-ramp-01").unwrap()as u16;
  let ramps:Vec<_>=source.tris.iter().filter(|t|t.texture==ramp).collect();
  assert!(ramps.iter().any(|t|t.p.iter().any(|p|p.y>7.0)),"quarters are taller than Jak and the board");
  assert!(ramps.iter().any(|t|{let[a,b,c]=t.p;(b-a).cross(c-a).normalize_or_zero().y>0.5}),"rideable faces point upward");
  assert!(ordinary_floor(44,-10).is_some());assert!(air_column(44,-10).is_some());
  for line in rail_lines(){assert!(line.windows(2).all(|p|Vec3::from_array(p[0].map(|v|v as f32)).distance(Vec3::from_array(p[1].map(|v|v as f32)))>10.0));}
  assert_eq!(crate::jak_pieces::place_of("pieza_haven_arena"),Some(8));
 }
}
