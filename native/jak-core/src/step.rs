//! Minecraft treads meet real collision without launching Jak.
use glam::Vec3;
use crate::collide::CollideWorld;
use crate::control::status;
use crate::math::*;
use crate::{Jak, State};
impl Jak {
    pub(crate) fn foot_step_up(&mut self, world: &mut dyn CollideWorld) {
        self.control.draw_offset_y = seek(self.control.draw_offset_y, 0.0, meters(5.5)*SECONDS_PER_FRAME);
        self.step_lean = seek(self.step_lean, 0.0, 2.5*SECONDS_PER_FRAME);
        if !matches!(self.state, State::Walk | State::DuckWalk { .. } | State::TurnAround | State::Roll | State::RunningAttack | State::Attack) { return; }
        if let Some(limit) = self.foot_smooth_step { self.terrain_step_up(world, limit, false); }
    }
    pub(crate) fn terrain_step_up(&mut self, world: &mut dyn CollideWorld, limit: f32, board: bool) {
        let c = &self.control;
        if !c.on_surface() && self.time_elapsed(c.last_time_on_surface, seconds(0.10)) {return;}
        let g = c.gravity_normal;
        let velocity = flatten(c.transv, g);
        let Some(dir) = velocity.try_normalize() else {return;};
        if velocity.length() < meters(0.05) {return;}
        let radius = c.body_radius;
        let trans = c.trans;
        // A narrow probe finds the tread rather than striking the next riser.
        // Use actual travel, including strafing, after thrust and steering.
        let ahead = velocity.length()*SECONDS_PER_FRAME + radius + meters(0.025);
        let motion = -g*(limit+meters(0.18));
        let side=dir.cross(g).normalize_or_zero()*radius*0.85;
        let mut up=0.0f32;
        // The whole leading edge crosses a corner, not only the centre ray.
        // Two perpendicular risers can meet under opposite sides of the body.
        for offset in [Vec3::ZERO,side,-side] {
            let start=trans+dir*ahead+offset+g*(limit+meters(0.08));
            if let Some(hit)=crate::collide::fill_and_probe_line_sphere(world,&mut self.probe_cache,start,motion,meters(0.015)) {
                let rise=g.dot(hit.intersect-trans);
                if hit.normal.dot(g)>=0.9 && rise<=limit {up=up.max(rise);}
            }
        }
        if up<meters(0.025){return;}
        for h in c.sphere_heights {
            if crate::collide::fill_and_probe_line_sphere(world,&mut self.probe_cache,trans+g*(h+meters(0.01)),g*up,radius*0.97).is_some_and(|hit|hit.u<0.999 && hit.normal.dot(g)< -0.1){return;}
            if crate::collide::fill_and_probe_line_sphere(world,&mut self.probe_cache,trans+g*(h+up+meters(0.01)),velocity*SECONDS_PER_FRAME,radius*0.97).is_some_and(|hit|hit.u<0.999 && hit.normal.dot(g)<0.7){return;}
        }
        let c=&mut self.control;
        c.trans+=g*(up+meters(0.005));
        c.transv=with_vertical(c.transv,g,0.0);c.gspot_pos=c.trans;
        if board {self.board.step_draw_offset-=up;self.board.step_ease_time=self.time;}
        else {c.draw_offset_y=(c.draw_offset_y-up).max(-limit);self.step_lean=(up/meters(0.6)).min(1.0);}
    }
    pub(crate) fn terrain_settle(&mut self, world:&mut dyn CollideWorld, limit:f32) {
        let c=&self.control;
        if !c.on_surface() && self.time_elapsed(c.last_time_on_surface,seconds(0.10)){return;}
        self.foot_capsule_settle(world,limit);
    }

    fn foot_capsule_settle(&mut self,world:&mut dyn CollideWorld,limit:f32){
        let c=&self.control;let g=c.gravity_normal;let h=c.sphere_heights[0];let radius=c.body_radius;
        let start=c.trans+g*(h+meters(0.06));let motion=-g*(limit+meters(0.08));
        // Follow the rounded lower sphere as it rolls off a tread. A ray
        // behind the feet must never snap the body back onto that tread.
        let Some(hit)=crate::collide::fill_and_probe_line_sphere(world,&mut self.probe_cache,start,motion,radius)else{return;};
        let centre=start+motion*hit.u;let support=(centre-hit.intersect).normalize_or(g);
        let delta=g.dot(centre-g*h-c.trans);
        if support.dot(g)<0.45||hit.normal.dot(g)< -0.1||delta>meters(0.01)||delta< -limit{return;}
        if delta< -meters(0.003){
            for height in c.sphere_heights.into_iter().skip(1){
                if crate::collide::fill_and_probe_line_sphere(world,&mut self.probe_cache,c.trans+g*height,g*delta,radius)
                    .is_some_and(|h|h.u<0.99&&h.normal.dot(g)>0.1){return;}
            }
        }
        let normal=if hit.normal.dot(g)>0.7{hit.normal}else{g};
        let c=&mut self.control;c.trans+=g*(delta+meters(0.003));
        c.transv=with_vertical(c.transv,g,0.0);
        if self.state.is_board(){self.board.step_draw_offset-=delta;}else{c.draw_offset_y-=delta;}
        c.status|=status::ON_SURFACE|status::ON_GROUND|status::TOUCH_SURFACE;
        c.last_time_on_surface=self.time;c.ground_poly_normal=normal;c.ground_pat=hit.pat;c.cur_pat=hit.pat;
        c.gspot_pos=c.trans;c.gspot_normal=normal;c.gspot_pat=hit.pat;
    }
}


#[cfg(test)] mod terrain_regressions {
 use super::*;use crate::{Tri,Pat,TriangleGrid,PadInput};
 fn quad(out:&mut Vec<Tri>,v:[Vec3;4]){for t in [[v[0],v[1],v[2]],[v[0],v[2],v[3]]]{out.push(Tri{v:t,pat:Pat::default()});}}
 fn stairs(rise:f32,depth:f32,n:usize,angle:f32)->TriangleGrid{
  let m=METER;let mut tris=vec![];
  quad(&mut tris,[Vec3::new(-5.0*m,0.0,-20.0*m),Vec3::new(-5.0*m,0.0,3.0*m),Vec3::new(5.0*m,0.0,3.0*m),Vec3::new(5.0*m,0.0,-20.0*m)]);
  for i in 0..n {let z0=3.0*m+depth*i as f32;let z1=if i+1==n{z0+20.0*m}else{z0+depth};let hi=rise*(i+1) as f32;
   quad(&mut tris,[Vec3::new(-5.0*m,hi-rise,z0),Vec3::new(-5.0*m,hi,z0),Vec3::new(5.0*m,hi,z0),Vec3::new(5.0*m,hi-rise,z0)]);
   quad(&mut tris,[Vec3::new(-5.0*m,hi,z0),Vec3::new(-5.0*m,hi,z1),Vec3::new(5.0*m,hi,z1),Vec3::new(5.0*m,hi,z0)]);
  }
  let q=glam::Quat::from_rotation_y(angle);for t in &mut tris{t.v=t.v.map(|v|q*v);}
  TriangleGrid::new(tris,2.0*m)
 }
 #[test]fn real_treads_are_walked_up_in_every_heading_without_jumping(){
  for angle in [0.0,std::f32::consts::FRAC_PI_2,std::f32::consts::PI] {
   let mut w=stairs(0.72*METER,0.72*METER,8,angle);let mut j=Jak::new(Vec3::ZERO,angle);
   j.control.set_body_dimensions(0.432*METER,2.592*METER);j.foot_smooth_step=Some(1.58*METER);
   let q=glam::Quat::from_rotation_y(angle);j.camera=crate::Basis{r:q*Vec3::X,u:Vec3::Y,f:q*Vec3::Z};
   for _ in 0..60{j.step(&PadInput::default(),&mut w,None);}
   let mut climbed=0.0f32;for _ in 0..150{j.step(&PadInput{left:[0.0,0.8],..Default::default()},&mut w,None);climbed=climbed.max(j.trans().y);}
   assert!(climbed>5.5*METER,"heading {angle}: {}",climbed/METER);
  }
 }
 #[test]fn eighth_block_terraces_do_not_launch_the_board(){
  let mut w=stairs(0.18*METER,1.44*METER,16,0.0);let mut j=Jak::new(Vec3::ZERO,0.0);
  j.control.set_body_dimensions(0.432*METER,2.592*METER);j.board_smooth_step=Some(1.58*METER);
  for _ in 0..60{j.step(&PadInput::default(),&mut w,None);}j.go(State::BoardStance);
  let mut top=0.0f32;let mut launched=0;
  for _ in 0..210{j.step(&PadInput{left:[0.0,1.0],..Default::default()},&mut w,None);
   top=top.max(j.trans().y);if j.trans().z<26.0*METER && j.velocity().y>METER{launched+=1;}
  }
  assert!(top>2.7*METER,"board reaches the hill: {}",top/METER);assert_eq!(launched,0,"no ballistic impulses");
  assert!(j.on_board());
 }
 #[test]fn full_blocks_with_snow_caps_are_walked_up_and_down_without_landing_locks(){
  let mut w=stairs(1.62*METER,1.44*METER,8,0.0);let mut j=Jak::new(Vec3::ZERO,0.0);
  j.control.set_body_dimensions(0.432*METER,2.592*METER);j.foot_smooth_step=Some(1.8*METER);
  for _ in 0..60{j.step(&PadInput::default(),&mut w,None);}
  for _ in 0..150{j.step(&PadInput{left:[0.0,0.8],..Default::default()},&mut w,None);}
  assert!(j.trans().y>12.5*METER,"climbed {}",j.trans().y/METER);
  let mut locked=0;let mut returned=false;
  for _ in 0..300 {j.step(&PadInput{left:[0.0,-0.8],..Default::default()},&mut w,None);
   if j.trans().z<15.0*METER && j.trans().z>3.0*METER && !matches!(j.state,State::Walk|State::Stance){locked+=1;}
   if j.trans().z<2.5*METER {returned=true;break;}
  }
  assert!(returned,"returns down the hill, {:?}",j.trans()/METER);assert_eq!(locked,0,"no forced landing state on each block");
  assert!(j.trans().y<0.1*METER,"back on level ground, {:?}",j.trans()/METER);
 }
 #[test]fn descending_short_city_terraces_beside_a_wall_keeps_making_progress(){
  for angle in [0.0,std::f32::consts::FRAC_PI_4,std::f32::consts::FRAC_PI_2] {for stick in [0.55,0.8] {
   let mut base=stairs(0.18*METER,1.44*METER,16,0.0);let mut tris=Vec::new();
   base.fill(Vec3::new(-6.0,-1.0,-21.0)*METER,Vec3::new(6.0,4.0,46.0)*METER,&mut tris);
   assert!(tris.len()>=30,"the fixture contains the whole stairway");
   quad(&mut tris,[Vec3::new(2.0,0.0,-20.0)*METER,Vec3::new(2.0,0.0,60.0)*METER,Vec3::new(2.0,20.0,60.0)*METER,Vec3::new(2.0,20.0,-20.0)*METER]);
   let q=glam::Quat::from_rotation_y(angle);for t in &mut tris{t.v=t.v.map(|v|q*v);}
   let mut world=TriangleGrid::new(tris,2.0*METER);
   let start=q*Vec3::new(2.0-0.432-0.035,2.88,27.1)*METER;
   let mut j=Jak::new(start,angle+std::f32::consts::PI);
   j.control.set_body_dimensions(0.432*METER,2.592*METER);j.foot_smooth_step=Some(1.8*METER);
   j.camera=crate::Basis{r:-(q*Vec3::X),u:Vec3::Y,f:-(q*Vec3::Z)};
   for _ in 0..60 {j.step(&PadInput::default(),&mut world,None);}
   let mut stalled=0;let mut worst=0;let mut reached=false;
   for _ in 0..2400 {
    let before=j.trans();j.step(&PadInput{left:[0.0,stick],..Default::default()},&mut world,None);
    let local=q.inverse()*j.trans()/METER;
    if local.z<2.5 {reached=true;break;}
    if (3.0..26.0).contains(&local.z){
     if flatten(j.trans()-before,Vec3::Y).length()<0.0001*METER{stalled+=1;}else{stalled=0;}
     worst=worst.max(stalled);
    }
   }
   assert!(reached,"descends beside wall: heading {angle}, stick {stick}, {:?}, {:?}",j.trans()/METER,j.state);
   assert!(worst<20,"no repeated sticking at a terrace: {worst} frames, heading {angle}, stick {stick}");
  }}
 }
 #[test]fn systems31_board_descends_city_terraces_without_snagging(){
  for angle in [0.,std::f32::consts::FRAC_PI_4,std::f32::consts::FRAC_PI_2]{
   let q=glam::Quat::from_rotation_y(angle);let mut world=stairs(0.18*METER,1.44*METER,16,angle);
   let mut j=Jak::new(q*Vec3::new(0.,2.88,28.)*METER,angle+std::f32::consts::PI);
   j.control.set_body_dimensions(0.432*METER,2.592*METER);j.board_smooth_step=Some(1.58*METER);
   j.camera=crate::Basis{r:-(q*Vec3::X),u:Vec3::Y,f:-(q*Vec3::Z)};
   for _ in 0..60{j.step(&PadInput::default(),&mut world,None);}j.go(State::BoardStance);
   let mut reached=false;
   for _ in 0..600{j.step(&PadInput{left:[0.,1.],..Default::default()},&mut world,None);
    if (q.inverse()*j.trans()).z<2.*METER{reached=true;break;}
   }
   assert!(reached,"board stuck at {:?}, {:?}",j.trans()/METER,j.state);assert!(j.on_board());
  }
 }
 #[test]fn descending_support_must_not_pull_jak_back_up_to_the_rear_tread(){
  let m=METER;let mut tris=Vec::new();
  quad(&mut tris,[Vec3::new(-5.0,1.0,-10.0)*m,Vec3::new(-5.0,1.0,0.0)*m,Vec3::new(5.0,1.0,0.0)*m,Vec3::new(5.0,1.0,-10.0)*m]);
  quad(&mut tris,[Vec3::new(-5.0,0.0,0.0)*m,Vec3::new(-5.0,0.0,10.0)*m,Vec3::new(5.0,0.0,10.0)*m,Vec3::new(5.0,0.0,0.0)*m]);
  quad(&mut tris,[Vec3::new(-5.0,0.0,0.0)*m,Vec3::new(5.0,0.0,0.0)*m,Vec3::new(5.0,1.0,0.0)*m,Vec3::new(-5.0,1.0,0.0)*m]);
  let mut world=TriangleGrid::new(tris,2.0*m);let radius=0.432*m;let past=0.20*m;
  // The lower sphere is already rolling around the upper step's edge.
  let y=m+(radius*radius-past*past).sqrt()-radius;
  let mut j=Jak::new(Vec3::new(0.0,y,past),0.0);j.control.set_body_dimensions(radius,2.592*m);
  j.control.status|=status::ON_SURFACE|status::ON_GROUND;
  j.terrain_settle(&mut world,1.8*m);
  assert!(j.trans().y<=y+0.005*m,"rear probe pulled Jak back up by {}m",(j.trans().y-y)/m);
  assert!(j.trans().y>=y-0.02*m,"support still respects the real capsule");
 }
 #[test]fn corners32_crossed_risers_are_climbed_by_foot_board_and_animal_in_all_headings(){
  for mode in 0..3 {for heading in 0..8 {for side in [-0.20,0.0,0.20] {
   let m=METER;let rise=0.72*m;let q=glam::Quat::from_rotation_y(heading as f32*std::f32::consts::FRAC_PI_4);let mut t=Vec::new();
   for x in 0..2 {for z in 0..2 {
    let (x0,x1)=if x==0{(-12.*m,0.)}else{(0.,20.*m)};
    let (z0,z1)=if z==0{(-12.*m,0.)}else{(0.,20.*m)};let y=(x+z)as f32*rise;
    quad(&mut t,[Vec3::new(x0,y,z0),Vec3::new(x0,y,z1),Vec3::new(x1,y,z1),Vec3::new(x1,y,z0)]);
   }}
   for i in 0..2 {
    let (a,b)=if i==0{(-12.*m,0.)}else{(0.,20.*m)};let y=i as f32*rise;
    quad(&mut t,[Vec3::new(0.,y,a),Vec3::new(0.,y,b),Vec3::new(0.,y+rise,b),Vec3::new(0.,y+rise,a)]);
    quad(&mut t,[Vec3::new(a,y,0.),Vec3::new(a,y+rise,0.),Vec3::new(b,y+rise,0.),Vec3::new(b,y,0.)]);
   }
   for tri in &mut t{tri.v=tri.v.map(|v|q*v);}
   let mut w=TriangleGrid::new(t,2.*m);let f=q*Vec3::new(1.,0.,1.).normalize();
   let mut j=Jak::new(q*Vec3::new(-3.+side,0.,-3.-side)*m,f.x.atan2(f.z));
   j.control.set_body_dimensions(0.432*m,2.592*m);j.foot_smooth_step=Some(1.8*m);j.board_smooth_step=Some(1.8*m);j.flut_smooth_step=Some(1.8*m);
   j.camera=crate::Basis{r:Vec3::Y.cross(f),u:Vec3::Y,f};
   for _ in 0..60{j.step(&PadInput::default(),&mut w,None);}
   if mode==1{j.go(State::BoardStance);}else if mode==2{j.go(State::FlutStance);}
   let mut reached=false;
   for _ in 0..450 {j.step(&PadInput{left:[0.,1.],..Default::default()},&mut w,None);
    let p=q.inverse()*j.trans()/m;if p.x>2.&&p.z>2.{reached=true;break;}
   }
   assert!(reached,"mode {mode}, heading {heading}, side {side}: {:?} {:?}",q.inverse()*j.trans()/m,j.state);
   assert!(j.trans().y>1.35*m,"both risers must be climbed, mode {mode}");
  }}}
 }
}
