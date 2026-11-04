//! Compact authoritative weapon presentation; original bolt and gravity-ring code.
//! Rope knots, ring clocks and shot ribbons cross the network; geometry is built locally.
use super::*;
use serde::{Serialize,Deserialize};
use crate::lightning_bolt::{Bolt,Strip};

#[derive(Clone,Debug,Default,Serialize,Deserialize)]
pub struct Visuals {pub time:i64,pub arc:Option<ArcVisual>,pub gravity:Vec<GravityVisual>,#[serde(default)]pub beams:Vec<Beam>}
#[derive(Clone,Debug,Serialize,Deserialize)]
pub struct ArcVisual {pub created:i64,pub state_time:i64,pub active:bool,pub points:Vec<Vec3>}
#[derive(Clone,Debug,Serialize,Deserialize)]
pub struct GravityVisual {pub center:Vec3,pub age:f32}
impl Visuals {
 pub fn translate(&mut self,offset:Vec3){
  if let Some(a)=&mut self.arc {for p in &mut a.points{*p+=offset;}}
  for g in &mut self.gravity{g.center+=offset;}
   for b in &mut self.beams{b.head+=offset;b.tail+=offset;}
 }
 pub fn validate(&self)->Result<(),String>{
  if self.beams.len()>4096||self.beams.iter().any(|b|!b.head.is_finite()||!b.tail.is_finite()||!b.width.is_finite()||b.width<0.||b.rgba.iter().any(|v|!v.is_finite()))||self.gravity.len()>64||self.arc.as_ref().is_some_and(|a|a.points.len()>12||a.points.iter().any(|p|!p.is_finite()))||self.gravity.iter().any(|g|!g.center.is_finite()||!g.age.is_finite()||g.age<0.) {return Err("Invalid weapon visual snapshot".into());}Ok(())
 }
}
impl Jak {
 pub fn weapon_visuals(&self)->Visuals {Visuals{
  time:self.time,beams:self.gun_beams(),
  arc:self.weapons.arc.as_ref().map(|a|ArcVisual{created:a.created,state_time:a.state_time,active:a.active,points:a.pts.clone()}),
  gravity:self.weapons.effects.iter().filter_map(|e|match e{Effect::Gravity{center,start,..}=>Some(GravityVisual{center:*center,age:(self.time-start)as f32/TICKS_PER_SECOND as f32}),_=>None}).collect(),
 }}
}

pub(super) fn arc_bolts(now:i64)->Vec<Bolt>{
 let seed=(now as u32).wrapping_mul(2_654_435_761)|1;
 (0..6u32).map(|i|Bolt::new(12,if i>=4{5}else{6},blue_light(i>=4),now,seed.wrapping_add(i*7919))).collect()
}
pub(super) fn configure_arc(bolts:&mut [Bolt],points:&[Vec3]){
        let n = points.len();
        let fade = if n > 0 { (points[0].distance(points[n - 1]) + 16384.0).min(262144.0) } else { 262144.0 };
        for (i, bolt) in bolts.iter_mut().enumerate() {
            let big = i >= 4;
            bolt.appearance.alpha_1.2 = fade;
            bolt.inner_point_travel_time = if big { 25 } else { 150 };
            bolt.snap_inner_points = true;
            bolt.fractal_reduction = 0.4 * 9f32.ln() / 12f32.ln();
            bolt.generate_mode = 1;
            bolt.num_active_spans = n.saturating_sub(1);
            let off = if big { 4096.0 } else { 8192.0 };
            for v in 0..bolt.num_active_spans {
                bolt.span_pts_start[v] = points[v];
                bolt.spans[v].random_offset_size_start = if v > 0 { off } else { 0.0 };
                bolt.set_inner_points(v, if big { 3 } else { 4 });
                bolt.spans[v].inner_random_offset_size = off;
            }
            if n > 0 {
                let last = bolt.num_active_spans;
                bolt.span_pts_start[last] = points[last];
                bolt.spans[last].random_offset_size_start = 0.0;
            }
        }
}

pub(super) fn gravity_sprites(center:Vec3,age:f32)->Vec<crate::particles::Sprite>{
 let mut out=Vec::new();
            for k in 0..5 {
                let Some((outer, inner)) = gravity_ring(k, age) else { continue };
                let (mid, band) = ((outer + inner) * 0.5, outer - inner);
                if mid <= 0.05 || band <= 0.01 {
                    continue;
                }
                // Around it in pieces a meter or two long.
                let pieces = ((std::f32::consts::TAU * mid / 1.5).ceil() as usize).clamp(12, 160);
                let step = std::f32::consts::TAU / pieces as f32;
                for i in 0..pieces {
                    let a = (i as f32 + 0.5) * step;
                    let (s, c) = a.sin_cos();
                    let pos = center + Vec3::new(c, 0.0, s) * meters(mid) + Vec3::Y * meters(0.08);
                    out.push(crate::particles::Sprite {
                        screen_aligned: false,
                        pos,
                        // Overlapping, the soft sprite's falloff inside the
                        // band, so the pieces make one dark band.
                        size: [meters(mid * step * 2.4), meters(band * 1.8)],
                        rotation: 0.0,
                        rgba: [0.0, 0.0, 0.0, 60.0],
                        texture: "glow-soft".into(),
                        blend: crate::particles::Blend::Normal,
                        corner: 0,
                        stretch: Some(Vec3::new(-s, 0.0, c)),
                        flat: Some(Vec3::Y),
                    });
                }
            }
 out
}

#[derive(Default)]
pub struct Presentation {visuals:Visuals,bolts:Vec<Bolt>,key:Option<i64>,last_draw:i64}
impl Presentation {
 pub fn sync(&mut self,visuals:Visuals)->Result<(),String>{
  visuals.validate()?;
  let prev=self.visuals.time.min(visuals.time);
  if let Some(a)=&visuals.arc {
   if self.key!=Some(a.created){self.bolts=arc_bolts(a.created);self.key=Some(a.created);self.last_draw=visuals.time;}
   configure_arc(&mut self.bolts,&a.points);
   for b in &mut self.bolts{b.set_fade(if a.active{0}else{2},a.state_time);b.update(visuals.time,prev);}
  }else{self.key=None;self.bolts.clear();}
  self.visuals=visuals;Ok(())
 }
 pub fn age(&mut self,dt:f32){
  let mut next=self.visuals.clone();next.time+=(dt*TICKS_PER_SECOND as f32).round()as i64;
  for g in &mut next.gravity{g.age+=dt;}let _=self.sync(next);
 }
 pub fn sprites(&self)->Vec<crate::particles::Sprite>{self.visuals.gravity.iter().flat_map(|g|gravity_sprites(g.center,g.age)).collect()}
 /// The original projected shadow is a continuous annulus. Tessellate its
 /// radial alpha falloff without overlapping billboards (which make dark wedges).
 pub fn ring_quads(&self)->Vec<[crate::lightning_bolt::StripVertex;4]>{
  use crate::lightning_bolt::StripVertex;let mut out=Vec::new();
  for g in &self.visuals.gravity {for k in 0..5 {
   let Some((outer,inner))=gravity_ring(k,g.age)else{continue};
   let middle=(outer+inner)*0.5;let pieces=((std::f32::consts::TAU*middle/1.5).ceil()as usize).clamp(12,160);
   for (lo,hi,alpha_lo,alpha_hi) in [(inner,middle,0.,60.),(middle,outer,60.,0.)]{
    for i in 0..pieces {
     let angles=[i as f32/pieces as f32*std::f32::consts::TAU,(i+1)as f32/pieces as f32*std::f32::consts::TAU];
     let vertex=|angle:f32,radius:f32,alpha:f32|StripVertex{pos:g.center+Vec3::new(angle.cos()*meters(radius),meters(0.08),angle.sin()*meters(radius)),rgba:[0.,0.,0.,alpha],st:[0.5,0.5]};
     out.push([vertex(angles[0],lo,alpha_lo),vertex(angles[1],lo,alpha_lo),vertex(angles[1],hi,alpha_hi),vertex(angles[0],hi,alpha_hi)]);
    }
   }
  }}out
 }
 // Match the visible gun without moving the target or collision knots.
 pub fn follow_muzzle(&mut self,muzzle:Vec3){
  if !muzzle.is_finite(){return;}
  if let Some(arc)=&self.visuals.arc {
   let mut points=arc.points.clone();
   if let Some(first)=points.first_mut(){*first=muzzle;configure_arc(&mut self.bolts,&points);}
  }
 }
 /// The existing Rust host's shot ribbons (render/src/extract.rs,
 /// push_jak_beams): colored halo, original yellow/blue core, head glow.
 pub fn beam_quads(&self,eye:Vec3)->Vec<(&'static str,[crate::lightning_bolt::StripVertex;4])>{
  use crate::lightning_bolt::StripVertex;
  let mut out=Vec::with_capacity(self.visuals.beams.len()*3);
  let mut quad=|a:Vec3,b:Vec3,half:f32,rgba:[f32;4],texture|{
   let side=(b-a).cross(eye-(a+b)*0.5).normalize_or_zero()*half;
   if side==Vec3::ZERO{return;}
   let corners=[a-side,a+side,b+side,b-side];
   let uv=[[0.,0.],[1.,0.],[1.,1.],[0.,1.]];
   out.push((texture,std::array::from_fn(|i|StripVertex{pos:corners[i],rgba,st:uv[i]})));
  };
  for beam in &self.visuals.beams {
   if beam.head.distance(beam.tail)<0.01{continue;}
   let [r,g,b,a]=beam.rgba;let width=meters(0.09)*beam.width.max(0.3);
   quad(beam.tail,beam.head,width*2.6,[r,g,b,0.55*a],"glow-soft");
   quad(beam.tail,beam.head,width*1.1,[0.7+0.3*r,0.7+0.3*g,0.7+0.3*b,a],
    if b>r+0.2{"gun-blue-beam"}else{"gun-yellow-beam"});
   let dir=(beam.head-beam.tail).normalize_or(Vec3::X);
   quad(beam.head-dir*width*3.,beam.head+dir*width*3.,width*3.,[r,g,b,0.8*a],"glow-soft");
  }
  out
 }
 pub fn strips(&mut self,up:Vec3)->Vec<Strip>{
  let now=self.visuals.time;let prev=self.last_draw.min(now);self.last_draw=now;
  self.bolts.iter_mut().flat_map(|b|b.draw(now,prev,up)).collect()
 }
}

#[cfg(test)]mod tests{
 use super::*;
 #[test]fn compact_replay_matches_original_arc_vertices(){
  let mut jak=Jak::new(Vec3::ZERO,0.);jak.arc_for_test(vec![Vec3::ZERO,Vec3::Z*meters(6.),Vec3::new(meters(1.),0.,meters(12.))]);
  let mut replay=Presentation::default();
  for _ in 0..30{
   let mut a=jak.weapons.arc.take().unwrap();jak.arc_bolts_frame(&mut a);jak.weapons.arc=Some(a);
   let wire=serde_json::to_string(&jak.weapon_visuals()).unwrap();assert!(wire.len()<2048);
   replay.sync(serde_json::from_str(&wire).unwrap()).unwrap();let actual=replay.strips(jak.camera.u);let expected=jak.arc_strips();
   assert_eq!(actual.len(),expected.len());
   for(a,b)in actual.iter().zip(expected){assert_eq!(a.texture,b.texture);assert_eq!(a.verts,b.verts);}
   jak.time+=FRAME_TICKS;
  }
 }
 #[test]fn rendered_muzzle_tracks_without_dragging_arc_targets(){
  let points=vec![Vec3::ZERO,Vec3::Z*meters(6.),Vec3::Z*meters(12.)];
  let mut replay=Presentation::default();
  replay.sync(Visuals{time:30,arc:Some(ArcVisual{created:0,state_time:0,active:true,points:points.clone()}),gravity:Vec::new(),beams:Vec::new()}).unwrap();
  let muzzle=Vec3::new(meters(2.),meters(1.4),meters(0.5));
  replay.follow_muzzle(muzzle);
  for bolt in &replay.bolts {
   assert_eq!(bolt.span_pts_start[0],muzzle);
   assert_eq!(bolt.span_pts_start[1],points[1]);
   assert_eq!(bolt.span_pts_start[2],points[2]);
  }
  assert_eq!(replay.visuals.arc.as_ref().unwrap().points,points);
  assert!(replay.strips(Vec3::Y).iter().flat_map(|s|&s.verts).all(|v|v.pos.is_finite()));
 }
 #[test]fn validates_and_rebases_compact_visuals(){
  let mut v=Visuals{time:0,arc:None,beams:Vec::new(),gravity:vec![GravityVisual{center:Vec3::ZERO,age:1.}]};v.translate(Vec3::X*6400.);assert_eq!(v.gravity[0].center.x,6400.);
  let mut p=Presentation::default();p.sync(v).unwrap();assert!(!p.sprites().is_empty());p.age(8.);assert!(p.sprites().is_empty());
 }
}
