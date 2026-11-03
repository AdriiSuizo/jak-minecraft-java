//! Wave Concussor: gun-red-shot.gc charging/explode/find-targets-and-attack.
//! Removes the former clone's Edit Blaster substitution.
use super::*;

pub(super) struct Wave {
 pub origin:Vec3,pub started:i64,pub exploded:Option<i64>,pub strength:f32,
 drained:f32,hit:Vec<u64>,tag:u64,
}
impl Wave {pub fn charging(&self)->bool{self.exploded.is_none()}}
impl Jak {
 pub(super) fn fire_wave(&mut self,from:Vec3)->bool{
  if self.weapons.waves.iter().any(Wave::charging){return false;}
  self.weapons.waves.push(Wave{origin:from,started:self.time,exploded:None,strength:0.,drained:0.,hit:Vec::new(),tag:0x7265643200000000|self.time as u64});true
 }
 pub(super) fn step_waves(&mut self,world:&mut dyn CollideWorld,mut actors:Option<&mut dyn ActorWorld>){
  let mut waves=std::mem::take(&mut self.weapons.waves);
  waves.retain_mut(|wave|{
   if wave.exploded.is_none(){
    wave.origin=self.gun.fire_point;
    let age=(self.time-wave.started) as f32/TICKS_PER_SECOND as f32;
    if self.gun.endless_ammo||self.gun.ammo[GunColor::Red as usize]>0.{wave.strength=age.clamp(0.,1.);}
    let drained=if age>=1.{5.}else if age>=0.75{4.}else if age>=0.5{3.}else if age>=0.25{2.}else if age>=0.1{1.}else{0.};
    if drained>wave.drained{self.gun.use_ammo(GunColor::Red,drained-wave.drained);wave.drained=drained;}
    if let Some(p)=self.particles.as_mut(){
     p.set_part_value(372,"scale-x",Some(lerp(3276.8,8192.,wave.strength)));
     p.set_part_value(373,"scale-x",Some(lerp(8192.,14336.,wave.strength)));
     p.keep_attached(wave.tag,"group-red-2-charge",wave.origin,Mat3::IDENTITY);
    }
    if age<0.1||self.pad.hold(crate::pad::button::R1)&&self.gun.out&&self.gun.kind==GunKind::Red2&&!self.dark&&!self.state.is_board(){return true;}
    wave.exploded=Some(self.time);
    if let Some(p)=self.particles.as_mut(){p.stop_tagged(wave.tag);p.set_part_value(372,"scale-x",None);p.set_part_value(373,"scale-x",None);}
    self.events.push(Event::Explosion{kind:GunKind::Red2,center:wave.origin,radius:meters(3.+15.*wave.strength)});
   }
   let age=(self.time-wave.exploded.unwrap()) as f32/TICKS_PER_SECOND as f32;
   let maximum=meters(3.+15.*wave.strength);let radius=(meters(3.)+meters(15.)*(age/0.7)).min(maximum);
   let stage=((radius-meters(3.))/(maximum-meters(3.)).max(1.)).clamp(0.,1.);
   let intensity=wave.strength*(1.-stage);
   if stage<1. {if let Some(a)=actors.as_deref_mut(){for(key,point)in a.actors_near(wave.origin,radius){
    if wave.hit.contains(&key){continue;}
    let delta=point-wave.origin;
    // Original wave penetrates nearby cover within 6 m, then probes line of sight.
    if Vec3::new(delta.x,0.,delta.z).length_squared()<meters(6.).powi(2)||clear_line(world,&mut self.weapons.cache,wave.origin,point){
     wave.hit.push(key);self.events.push(impact(point,Vec3::new(delta.x,0.,delta.z),(5.*intensity).max(1.),Some(key)));
    }
   }}}
   let ring_radius=meters(3.+15.*(age/0.35));let alpha=(1.7-ring_radius/maximum).clamp(0.,1.);
   if alpha>0. {if let Some(p)=self.particles.as_mut(){
    p.set_part_value(229,"scale-x",Some(2.*ring_radius));p.set_part_value(229,"scale-y",Some(2.*ring_radius));p.set_part_value(229,"a",Some(128.*alpha));
    p.launch_flat(229,wave.origin,Mat3::IDENTITY,1.);
    for field in ["scale-x","scale-y","a"]{p.set_part_value(229,field,None);}
   }}
   age<0.7
  });self.weapons.waves=waves;
 }
}

#[cfg(test)]mod tests {
 use super::*;
 struct Target;
 impl ActorWorld for Target {
  fn actor_hit(&mut self,_:Vec3,_:Vec3,_:f32)->Option<(u64,f32,Vec3)>{None}
  fn actors_near(&mut self,at:Vec3,r:f32)->Vec<(u64,Vec3)>{let p=Vec3::Z*meters(8.);if at.distance(p)<=r{vec![(9,p)]}else{vec![]}}
 }
 #[test]fn charge_drains_five_rounds_and_expanding_wave_hits_each_actor_once(){
  let mut jak=Jak::new(Vec3::ZERO,0.);jak.gun.kind=GunKind::Red2;jak.gun.out=true;jak.gun.ammo[0]=100.;jak.pad.held=crate::pad::button::R1;
  assert!(jak.fire_wave(Vec3::ZERO));let mut world=crate::collide::EmptyWorld;let mut target=Target;
  for _ in 0..61{jak.step_waves(&mut world,Some(&mut target));jak.time+=FRAME_TICKS;}
  assert_eq!(jak.gun.ammo[0],95.);assert!(jak.weapons.charging());jak.pad.held=0;
  for _ in 0..48{jak.step_waves(&mut world,Some(&mut target));jak.time+=FRAME_TICKS;}
  assert!(!jak.weapons.charging());assert!(jak.weapons.waves.is_empty());
  assert_eq!(jak.events.iter().filter(|e|matches!(e,Event::Impact(hit)if hit.actor==Some(9))).count(),1);
  assert!(jak.events.iter().any(|e|matches!(e,Event::Explosion{kind:GunKind::Red2,..})));
 }
}
