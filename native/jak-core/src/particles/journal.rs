//! Authoritative launch stream. The client runs the same original sparticle
//! simulator at 60 Hz; a server tick never becomes a 20 Hz sprite animation.
use super::*;
use serde::{Serialize,Deserialize};

#[derive(Clone,Debug,Serialize,Deserialize)]
pub struct Lightning {pub id:u64,pub spec:String,pub from:Vec3,pub to:Vec3,pub age:f32,pub probe:bool}

#[derive(Clone,Debug,Serialize,Deserialize)]
pub enum Command {
 Target(Vec3,Vec3),Orientation(Vec3,f32),Velocity(Vec3),Ground(f32),
 Sources(Vec<(u64,Vec3)>),From(u32,u64,Vec3,Mat3),Peace(u64,Vec3,f32),
 Lightning(Vec<Lightning>),Spawn(String,Vec3,Mat3),Tagged(String,Vec3,Mat3,u64),
 Move(u64,Vec<(f32,Vec3)>),Stop(u64),Keep(u64,String,Vec3,Mat3),Attached(u64,String,Vec3,Mat3),
 Step(f32),Launch(u32,Vec3,Mat3),Beam(u32,Vec3,Vec3,f32),
 BeamSized(u32,Vec3,Vec3,f32,f32),Blade(u32,Vec3,Vec3,Vec3,f32,f32),
 Flat(u32,Vec3,Mat3,f32),Scaled(u32,Vec3,Mat3,f32),Value(u32,String,Option<f32>),
 Tracker(String,f32,Vec3),Var(u32,Vec3,Mat3,Overrides,f32),
}
impl Command {
 pub fn apply(self,p:&mut Particles)->Result<(),String>{
  match self {
   Self::Target(a,b)=>p.set_target(a,b),Self::Orientation(a,b)=>p.set_target_orientation(a,b),
   Self::Velocity(a)=>p.set_particle_velocity(a),Self::Ground(a)=>p.set_target_ground(a),
   Self::Sources(a)=>p.set_sources(a),Self::From(a,b,c,d)=>p.launch_from(a,b,c,d),
   Self::Peace(a,b,c)=>p.launch_peacemaker(a,b,c),
   Self::Lightning(a)=>{
    const KNOWN:&[&str]=&["lightning-darkjak-attack","lightning-darkjak-transform","lightning-blue-shot-attack-explode","lightning-blue-shot-terminal","lightning-dark-shot-attack-thick","lightning-tile"];
    let mut requests=Vec::new();for r in a {let spec=*KNOWN.iter().find(|s|**s==r.spec).ok_or_else(||format!("Unrecognized original lightning spec: {}",r.spec))?;
     requests.push(LightningRequest{id:r.id,spec,from:r.from,to:r.to,age:r.age,probe:r.probe});}
    p.sync_lightning(requests);
   },
   Self::Spawn(a,b,c)=>p.spawn(&a,b,c),Self::Tagged(a,b,c,d)=>p.spawn_tagged(&a,b,c,d),
   Self::Move(tag,points)=>{if let Some((_,last))=points.last(){let last=*last;p.move_tagged(tag,|t|points.iter().min_by(|a,b|(a.0-t).abs().total_cmp(&(b.0-t).abs())).map_or(last,|v|v.1));}},
   Self::Stop(a)=>p.stop_tagged(a),Self::Keep(a,b,c,d)=>p.keep(a,&b,c,d),
   Self::Attached(a,b,c,d)=>p.keep_attached(a,&b,c,d),Self::Step(a)=>p.step(a),
   Self::Launch(a,b,c)=>p.launch(a,b,c),Self::Beam(a,b,c,d)=>p.launch_beam_len(a,b,c,d),
   Self::BeamSized(a,b,c,d,e)=>p.launch_beam_sized(a,b,c,d,e),Self::Blade(a,b,c,d,e,f)=>p.launch_blade(a,b,c,d,e,f),
   Self::Flat(a,b,c,d)=>p.launch_flat(a,b,c,d),Self::Scaled(a,b,c,d)=>p.launch_scaled(a,b,c,d),
   Self::Value(a,b,c)=>p.set_part_value(a,&b,c),Self::Tracker(a,b,c)=>p.configure_tracker(&a,b,c)?,
   Self::Var(a,b,c,d,mut carry)=>p.launch_var(a,b,c,d,&mut carry),
  }Ok(())
 }
}

#[cfg(test)]mod tests {
 use super::*;
 #[test]fn journal_replays_original_births_and_growth_without_duplicate_nested_calls(){
  let mut library=Library::default();library.read("(defpart 1 :init-specs ((:texture (smoke level-default-sprite)) (:num 1) (:scale-x 4096) (:scale-y 2048) (:scalevel-x 3) (:timer 300))) (defpartgroup group-test 1 (sp-item 1))");
  let library=Arc::new(library);let mut server=Particles::new(library.clone());server.start_journal();let mut client=Particles::new(library);
  for _ in 0..8 {server.keep(7,"group-test",Vec3::X*1234.,Mat3::IDENTITY);server.launch_beam(1,Vec3::ZERO,Vec3::Z);server.step(1./60.);
   let stream=server.take_journal();assert_eq!(stream.len(),3);let encoded=serde_json_test_round_trip(&stream);
   for c in encoded {c.apply(&mut client).unwrap();}
   let a=server.sprites();let b=client.sprites();assert!(!a.is_empty());assert_eq!(a.len(),b.len());for(a,b)in a.iter().zip(&b){assert_eq!(a.pos,b.pos);assert_eq!(a.size,b.size);assert_eq!(a.rgba,b.rgba);}
  }
 }
 fn serde_json_test_round_trip(stream:&[Command])->Vec<Command>{serde_json::from_str(&serde_json::to_string(stream).unwrap()).unwrap()}
}
