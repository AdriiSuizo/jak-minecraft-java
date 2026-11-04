//! Grid-built palace. Every occupied coordinate is an ordinary reusable block;
//! curved skating surfaces are supplied separately by the arena source.
use std::collections::BTreeMap;
type Pos=(i32,i32,i32);
type Grid=BTreeMap<Pos,&'static str>;
const FLOOR:&str="jak:palacio_suelo_metal";
const WALL:&str="jak:palacio_pared_azul";
const STONE:&str="jak:palacio_suelo_piedra";
const RED:&str="jak:hv_t-citypal-met-bm-red-strp01";
const ROOF:&str="jak:hv_t-citypal-red-met-01";
const PILLAR:&str="jak:hv_t-citypal-met-pill-01";
const AIR:&str="minecraft:air";
const TOP:&str="jak:palacio_suelo_metal_losa[type=top,waterlogged=false]";
const HALF:&str="jak:palacio_suelo_metal_losa[type=bottom,waterlogged=false]";
const LIGHT:&str="jak:palacio_ventana_luminosa";
const CROWN_RADIUS:i32=48;
const TIERS:[(i32,i32,i32);5]=[(104,116,30),(116,128,25),(128,140,20),(140,152,15),(152,160,9)];
const TURRETS:[(i32,i32,i32,i32);8]=[(-39,0,6,128),(39,0,6,134),(0,-39,6,140),(0,39,6,122),
 (-32,-32,4,122),(32,-32,4,128),(-32,32,4,122),(32,32,4,116)];
fn roof_step(x:i32,z:i32)->&'static str{
 match if x.abs()>z.abs(){if x<0{0}else{1}}else if z<0{2}else{3}{
  0=>"jak:palacio_techo_escalera[facing=east,half=bottom,shape=straight,waterlogged=false]",
  1=>"jak:palacio_techo_escalera[facing=west,half=bottom,shape=straight,waterlogged=false]",
  2=>"jak:palacio_techo_escalera[facing=south,half=bottom,shape=straight,waterlogged=false]",
  _=>"jak:palacio_techo_escalera[facing=north,half=bottom,shape=straight,waterlogged=false]"}
}
fn oct(x:i32,z:i32,r:i32)->bool{x.abs()<=r&&z.abs()<=r&&x.abs()+z.abs()<=r+r*3/5}
fn put(g:&mut Grid,x:i32,y:i32,z:i32,s:&'static str){g.insert((x-5,y,z-5),s);}
fn floor(g:&mut Grid,y:i32,r:i32){
 for x in -r..=r{for z in -r..=r{if oct(x,z,r){put(g,x,y-1,z,
  if x.rem_euclid(8)==0&&z.rem_euclid(8)==0{LIGHT}else{FLOOR});}}}
}
fn ring(g:&mut Grid,y:i32,r:i32,s:&'static str){
 for x in -r..=r{for z in -r..=r{if oct(x,z,r)&&!oct(x,z,r-1){put(g,x,y,z,s);}}}
}
/// Three-block-wide half-step stair, with integer landings and real slab volume.
pub fn ascent_point(t:f32)->[f32;3]{
 let n=(t.clamp(0.,1.)*196.).round()as i32;
 let(x,z)=path(n);
 [x as f32-4.5,6.+n as f32*0.5,z as f32-4.5]
}
fn path(n:i32)->(i32,i32){
 let n=n.rem_euclid(96);
 match n/24{0=>(-12+n,12),1=>(12,36-n),2=>(60-n,-12),_=>(-12,n-84)}
}
fn stair(g:&mut Grid){
 for n in 0..=196{
  let(x,z)=path(n);
  let half=12+n;
  let y=(half-1)/2;
  let state=if half%2==0{TOP}else{HALF};
  let side=n.rem_euclid(96)/24;
  for w in 0..3{
   let(dx,dz)=match side{0=>(0,w),1=>(w,0),2=>(0,-w),_=>(-w,0)};
   // Remove floor/ceiling cells along the route before setting the tread.
   for h in 0..4{put(g,x+dx,y+h,z+dz,AIR);}
   put(g,x+dx,y,z+dz,state);
  }
  // Guard at the outer edge, except the two usable landings.
  if n>3&&n<191{
   let(dx,dz)=match side{0=>(0,3),1=>(3,0),2=>(0,-3),_=>(-3,0)};
   let panel=match side{
    0=>"jak:palacio_pared_azul_medio_vertical[facing=north]",
    1=>"jak:palacio_pared_azul_medio_vertical[facing=west]",
    2=>"jak:palacio_pared_azul_medio_vertical[facing=south]",
    _=>"jak:palacio_pared_azul_medio_vertical[facing=east]"};
   put(g,x+dx,(half+1)/2,z+dz,panel);
  }
  if n%16==8{put(g,x,y+4,z,LIGHT);}
 }
}
fn upper_stair(g:&mut Grid){
 // Two straight half-step flights per storey, a common 7x14 module.
 for base in (104..152).step_by(12){
  for i in 0..12{
   for (xs,z,h) in [(-3,-6+i,base*2+i+1),(1,5-i,base*2+13+i)]{
    for x in xs..xs+3 {
     let y=(h-1)/2;
     for dy in 0..4{put(g,x,y+dy,z,AIR);}
     put(g,x,y,z,if h%2==0{TOP}else{HALF});
    }
   }
  }
  for x in -3..=3 {put(g,x,base+5,6,TOP);}
 }
}
fn house(g:&mut Grid,cx:i32,cz:i32,r:i32,top:i32){
 for y in 104..top{
  for x in cx-r..=cx+r{for z in cz-r..=cz+r{
   let boundary=x==cx-r||x==cx+r||z==cz-r||z==cz+r;
   if boundary{put(g,x,y,z,if (y-104)%6==5{RED}else if y%6==1&&(x==cx||z==cz){LIGHT}else{WALL});}
  }}
 }
 for y in (104..=top).step_by(6){
  for x in cx-r..=cx+r{for z in cz-r..=cz+r{put(g,x,y-1,z,FLOOR);}}
  put(g,cx+r-1,y,cz-r+1,"minecraft:chest[facing=south,type=single,waterlogged=false]");
 }
 // Continuous ladder, its backing is the west wall. Clear every floor in
 // the shaft; occupants can leave at every landing and at the roof terrace.
 for y in 104..=top{put(g,cx-r+1,y,cz-r+1,"minecraft:ladder[facing=east,waterlogged=false]");}
 let(dx,dz)=if cx.abs()>cz.abs(){(-cx.signum()*r,0)}else{(0,-cz.signum()*r)};
 for y in 104..107{put(g,cx+dx,y,cz+dz,AIR);}
 let gate=if dx!=0{"jak:krimson_porton_fence_gate[facing=east,in_wall=false,open=false,powered=false]"}else{"jak:krimson_porton_fence_gate[facing=south,in_wall=false,open=false,powered=false]"};
 put(g,cx+dx,104,cz+dz,gate);
 // Roof occupies the far half; the ladder exits onto a usable terrace.
 for i in 0..r{
  for x in cx-r+i..=cx+r-i{for z in cz..=cz+r-i{put(g,x,top+i,z,
   if x==cx-r+i||x==cx+r-i||z==cz+r-i{roof_step(x-cx,z-cz)}else{ROOF});}}
 }
}
const RAIL:&str="jak:haven_baranda[east=low,north=low,south=low,up=true,waterlogged=false,west=low]";
fn facade_details(g:&mut Grid,cx:i32,cz:i32,r:i32,base:i32,top:i32){
 let flat=if cx==0&&cz==0{r*3/5-2}else{r-2};
 for(dx,dz)in[(1,0),(-1,0),(0,1),(0,-1)]{
  let at=|p:i32,y:i32,out:i32|(cx+dx*(r+out)-dz*p,y,cz+dz*(r+out)+dx*p);
  for p in -flat..=flat{if p.abs()<3||p.rem_euclid(6)!=0{continue;}
   for y in (base+3..top-2).step_by(6){
    for h in 0..2{let(x,y,z)=at(p,y+h,0);put(g,x,y,z,LIGHT);}
    for w in [-1,1]{for h in -1..=2{let(x,y,z)=at(p+w,y+h,0);put(g,x,y,z,STONE);}}
    for h in [-1,2]{for w in -1..=1{let(x,y,z)=at(p+w,y+h,1);put(g,x,y,z,"jak:palacio_detalle_cornisa");}}
   }
  }
  for p in [-flat,flat]{for y in base..top{
   let(x,y,z)=at(p,y,0);put(g,x,y,z,if y==base||y==top-1{STONE}else{PILLAR});
  }}
  // Original Jak insignia and real wall-mounted hardware, on both sides
  // of the door rather than across the route through it.
  let face=match(dx,dz){(1,0)=>"east",(-1,0)=>"west",(0,1)=>"south",_=>"north"};
  let banner=match face{
   "east"=>"jak:palacio_emblema[facing=east]","west"=>"jak:palacio_emblema[facing=west]",
   "south"=>"jak:palacio_emblema[facing=south]",_=>"jak:palacio_emblema[facing=north]"};
  for p in [-3,3]{let(x,y,z)=at(p,base+4,1);put(g,x,y,z,banner);}
 }
}
fn palace_details(g:&mut Grid){
 // Window bays, stone collars and a repeated cornice give the shaft depth.
 for base in (18..66).step_by(12){facade_details(g,0,0,20,base,base+12);}
 facade_details(g,0,0,36,6,18);
 facade_details(g,0,0,CROWN_RADIUS,90,104);
 ring(g,104,CROWN_RADIUS,RAIL);
 for(base,top,r)in TIERS{
  facade_details(g,0,0,r,base,top);
  // Solid roof terrace with a rail around the exposed edge. The next tier
  // is narrower; all stair and doorway passages are cleared afterwards.
  for x in -r..=r{for z in -r..=r{
   if oct(x,z,r)&&!oct(x,z,r-1){put(g,x,top,z,RAIL);}
  }}
  for x in [-r+4,r-4]{for z in [-r/2,r/2]{
   put(g,x,base,z,"jak:bar_mesa");
   put(g,x+if x<0{1}else{-1},base,z,"jak:bar_taburete");
   put(g,x,base,-r+3,"minecraft:bookshelf");
  }}
  // A lit service gallery along each side of the central hall.
  if r>12{for z in (-r+4..=r-4).step_by(8){for x in [-8,8]{
   put(g,x,base,z,"jak:palacio_detalle_columna");put(g,x,base+1,z,LIGHT);
  }}}
 }
 for(cx,cz,r,top)in TURRETS{
  facade_details(g,cx,cz,r,104,top);
  for y in (104..top).step_by(6){
   put(g,cx+1,y,cz+2,"jak:bar_mesa");put(g,cx,y,cz+2,"jak:bar_taburete");
   put(g,cx+r-1,y+2,cz,"jak:krimson_terminal");
  }
  for x in cx-r..=cx+r{put(g,x,top,cz-r,RAIL);}
 }
 // Roof details and exterior service pipes are anchored to full walls.
 for x in [-17,17]{for y in 20..73{put(g,x,y,21,"jak:barrio_tubo[facing=south]");}}
}
pub fn blocks()->Vec<(Pos,&'static str)>{
 let mut g=Grid::new();
 // Broad plinth, narrow shaft and flared crown, with integer course heights.
 for y in 0..104{
  let r=if y<18{36}else if y<76{20}else if y<90{20+(y-76)*(CROWN_RADIUS-20)/14}else{CROWN_RADIUS};
  ring(&mut g,y,r,if y%12==0||y==89||y==103{RED}else if y<18{"jak:palacio_pared_estuco"}else{WALL});
  // A solid course closes every outward transition; it has a real underside.
  let prev=if y==18{36}else if y==76{20}else if y>76&&y<=90{20+(y-77)*(CROWN_RADIUS-20)/14}else{r};
  if prev!=r{for x in -prev.max(r)..=prev.max(r){for z in -prev.max(r)..=prev.max(r){
   if oct(x,z,prev.max(r))&&!oct(x,z,prev.min(r)-1){put(&mut g,x,y,z,STONE);}
  }}}
  // Regular facade windows: full luminous blocks, no intersecting light planes.
  if (7..=9).contains(&(y%12)){for p in [-8,0,8]{for(x,z)in[(p,r), (p,-r),(r,p),(-r,p)]{put(&mut g,x,y,z,LIGHT);}}}
  // Stone ribs break up the broad facade using the same buildable palette
  // as Haven houses. They remain flush, so they cannot block the interior.
  for p in [-12,12]{for(x,z)in[(p,r),(p,-r),(r,p),(-r,p)]{put(&mut g,x,y,z,if y%12==0{RED}else{STONE});}}
 }
 // Street-level service hall beneath the skate floor. Four real open
 // arches connect its floor to the district, clear of horns and front stairs.
 for sign in [-1,1]{for d in -1..=1{for y in 0..4{
  put(&mut g,12+d,y,sign*36,AIR);put(&mut g,sign*36,y,12+d,AIR);
 }}}
 floor(&mut g,6,35);floor(&mut g,90,CROWN_RADIUS-1);floor(&mut g,104,CROWN_RADIUS);
 // Upper palace levels. A complete wall and roof enclose each narrower tier.
 for (base,top,r) in TIERS{
  floor(&mut g,base,r);
  for y in base..top{ring(&mut g,y,r,if y==base||y==top-1{RED}else{WALL});}
  floor(&mut g,top,r);
  // The common hall opens onto every surrounding terrace. A sealed wall
  // would make the satellite rooms inaccessible despite their own doors.
  for sign in [-1,1]{
   for dx in -1..=1{for y in base..base+3{put(&mut g,dx,y,sign*r,AIR);}}
   for dx in -1..=1{put(&mut g,dx,base,sign*r,"jak:krimson_porton_fence_gate[facing=south,in_wall=false,open=false,powered=false]");}
  }
  for y in (base+2..top-1).step_by(6){for p in [-5,0,5]{for(x,z)in[(p,r),(p,-r),(r,p),(-r,p)]{put(&mut g,x,y,z,LIGHT);}}}
  // Partitioned rooms around a common stair hall, with two-block doorways.
  if base<152{for x in [-6,6]{for z in -r+1..r{for y in base..base+5{
   if !((-1..=0).contains(&z)&&y<base+3){put(&mut g,x,y,z,"jak:palacio_pared_estuco");}
  }}}}
  for x in [-r+2,r-2]{put(&mut g,x,base,-r+2,"minecraft:chest[facing=south,type=single,waterlogged=false]");}
  put(&mut g,r-2,base,2,"jak:krimson_terminal");
 }
 // Distinct stepped turrets, all entered from the same accessible terrace.
 for (x,z,r,top) in TURRETS{house(&mut g,x,z,r,top);}
 // Central spire sits over an actual room, not over an empty scenery shell.
 for i in 0..9 {let r=9-i;for x in -r..=r{for z in -r..=r{if oct(x,z,r){put(&mut g,x,160+i,z,
  if !oct(x,z,r-1){roof_step(x,z)}else{ROOF});}}}}
 // Eight curved-outline buttresses built from repeating half-block and stair
 // modules. Every part fits one cell and can be reused elsewhere.
 for base in [6,96]{for (dx,dz) in [(1,0),(-1,0),(0,1),(0,-1),(1,1),(1,-1),(-1,1),(-1,-1)]{
  let start=if dx!=0&&dz!=0{if base==6{28}else{38}}else if base==6{35}else{CROWN_RADIUS-1};
  let reach=|i:i32|(22.*(i.max(0)as f32/47.*std::f32::consts::FRAC_PI_2).sin()).round()as i32;
  for i in 0..48{
   let radius=start+reach(i);let y=base+i;
   let thickness=if i<12{4}else if i<24{3}else if i<34{2}else if i<43{1}else{0};
   for w in -thickness..=thickness{
    let(x,z)=(dx*radius+dz*w,dz*radius-dx*w);
    let slope=match (dx,dz){(1,0)=>"jak:palacio_pilar_escalera[facing=east,half=bottom,shape=straight,waterlogged=false]",
     (-1,0)=>"jak:palacio_pilar_escalera[facing=west,half=bottom,shape=straight,waterlogged=false]",
     (_,1)=>"jak:palacio_pilar_escalera[facing=south,half=bottom,shape=straight,waterlogged=false]",
     _=>"jak:palacio_pilar_escalera[facing=north,half=bottom,shape=straight,waterlogged=false]"};
    // Smooth upright ends instead of alternating stair/full-block teeth.
    put(&mut g,x,y,z,if reach(i)>reach(i-1)||i==47{slope}else{PILLAR});
    // A shallow solid rib joins consecutive slices and its root to the wall.
    // Diagonal ribs use a square footprint, not corner-touching floating cells.
    if i<46{let depth=if i<24{3}else{1};for ox in 0..=dx.abs()*depth{for oz in 0..=dz.abs()*depth{
     put(&mut g,x+ox*dx,y-1,z+oz*dz,PILLAR);
    }}}
   }
  }
 }}
 palace_details(&mut g);
 // Entrance descends to ground, on supported half steps.
 // The enlarged front horn forms an arch over the entrance. Clear the
 // whole passage through its root, not only the single wall coordinate.
 for z in 32..=45{for y in 6..10 {for x in -3..=3{put(&mut g,x,y,z,AIR);}}}
 for x in -2..=2{put(&mut g,x,6,36,"jak:krimson_porton_fence_gate[facing=south,in_wall=false,open=false,powered=false]");}
 for z in 36i32..=64{
  let h=if z<52{12}else{(64-z).max(0)};
  for x in -3..=3{
   let top=(h-1).div_euclid(2);
   for y in 0..top{put(&mut g,x,y,z,STONE);}
   if h>0{put(&mut g,x,top,z,if h%2==0{TOP}else{HALF});}
  }
 }
 stair(&mut g);
 // Clear the stairwell through every upper floor before laying the flights.
 for y in 104..156{for x in -3..=3{for z in -6..=5{put(&mut g,x,y,z,AIR);}}}
 upper_stair(&mut g);
 // A ladder connects the highest inhabited floor to the roof inspection deck.
 for y in 152..=160{put(&mut g,-7,y,-6,"minecraft:ladder[facing=east,waterlogged=false]");}
 put(&mut g,-7,160,-6,AIR);
 put(&mut g,0,6,0,"jak:altar_turbotabla");
 let mut out=g.into_iter().collect();crate::jak_lighting::illuminate(&mut out);out
}
#[cfg(test)]mod tests{
 use super::*;
 #[test]fn architecture28_palette_is_registered_solid_and_legacy_pieces_keep_their_bank(){
  let r=minecraftoss_core::Registries::load(&minecraftoss_core::registries::DataPaths::discover().unwrap()).unwrap();
  let kinds:std::collections::BTreeSet<_>=blocks().into_iter().map(|(_,s)|s).collect();
  for s in kinds{r.blocks.parse_state(s).unwrap_or_else(|e|panic!("{s}: {e}"));}
  let light=r.blocks.parse_state(LIGHT).unwrap();
  assert_eq!(r.blocks.collision_boxes(light),vec![[0.,0.,0.,1.,1.,1.]]);
  assert_eq!(crate::jak_pieces::place_of("pieza_haven_torre_circuito"),Some(9));
  assert_eq!(crate::jak_pieces::place_of("pieza_haven_rampa_modular"),Some(10));
  let old=crate::jak_city_arena_v9::column(8048,8064).unwrap();
  assert!(old.pieces.keys().any(|p|p.1>100),"saved palace pieces retain original upper geometry");
  let new=super::super::column(8048,8064).unwrap();
  assert!(new.pieces.keys().all(|p|p.1<12),"new palace structure must not contain distant scenery");
 }
 #[test]fn architecture28_grid_palace_has_real_rooms_and_supported_stairs(){
  let g:Grid=blocks().into_iter().collect();
  let block=|x,y,z|g.get(&(x-5,y,z-5)).copied().unwrap_or(AIR);
  for y in [104,116,128,140,152]{
   assert_ne!(block(5,y-1,0),AIR,"floor {y}");
   assert_eq!(block(4,y+1,0),AIR,"walkable room {y}");
  }
  for(base,_,r)in TIERS{
   for sign in [-1,1]{assert!(block(0,base,sign*r).contains("fence_gate"));assert_eq!(block(0,base+1,sign*r),AIR);}
  }
  for n in 0..=196{let(x,z)=path(n);let h=12+n;
   assert!(matches!(block(x,(h-1)/2,z),TOP|HALF),"stair {n}");
   assert_eq!(block(x,(h+1)/2+1,z),AIR,"stair headroom {n}");
  }
  assert!(g.values().all(|s|!s.contains("pieza_haven")),"structure must use reusable catalogue pieces");
 }
 #[test]fn architecture28_turrets_have_open_entries_floors_and_roof_access(){
  let g:Grid=blocks().into_iter().collect();
  for(cx,cz,r,top)in TURRETS{
   let block=|x,y,z|g.get(&(x-5,y,z-5)).copied().unwrap_or(AIR);
   for y in (104..=top).step_by(6){
    assert_ne!(block(cx,y-1,cz),AIR);
    assert!(block(cx-r+1,y,cz-r+1).contains("ladder"));
   }
  }
 }
 #[test]fn architecture29_crown_and_horns_fit_reserved_parcel_and_keep_terrace_access(){
  let g:Grid=blocks().into_iter().collect();
  let block=|x,y,z|g.get(&(x-5,y,z-5)).copied().unwrap_or(AIR);
  assert_ne!(block(47,103,0),AIR,"enlarged crown has a solid floor");
  assert_ne!(block(69,142,0),AIR,"large upper horn reaches its curved tip");
  assert_ne!(block(57,52,0),AIR,"large lower horn reaches its curved tip");
  for z in 32..=45{for y in 6..10{for x in -3..=3{
   let s=block(x,y,z);assert!(s==AIR||s.contains("fence_gate"),"entrance blocked at {x},{y},{z}: {s}");
  }}}
  for (cx,cz,r,_) in TURRETS{
   let(dx,dz)=if cx.abs()>cz.abs(){(-cx.signum()*r,0)}else{(0,-cz.signum()*r)};
   assert!(block(cx+dx,104,cz+dz).contains("fence_gate"));
   assert_eq!(block(cx+dx,105,cz+dz),AIR);
  }
  for(&(x,_,z),_)in &g{
   assert!((super::super::FOOTPRINT_MIN[0]..super::super::FOOTPRINT_MAX[0]).contains(&x));
   assert!((super::super::FOOTPRINT_MIN[1]..super::super::FOOTPRINT_MAX[1]).contains(&z));
  }
 }
 #[test]fn detail30_palace_window_bays_stay_on_closed_octagonal_walls(){
  let g:Grid=blocks().into_iter().collect();
  for(base,_,r)in TIERS{
   for(&(x,y,z),&s)in &g{
    let(x,z)=(x+5,z+5);
    if s==LIGHT&&y==base+3&&x.abs().max(z.abs())==r{
     assert!(oct(x,z,r),"window bay outside the actual palace wall at {x},{y},{z}");
    }
   }
  }
 }
}

