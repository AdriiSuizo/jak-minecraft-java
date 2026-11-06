//! Rendering-side original sparticles. Independent of server tick frequency.
use super::*;
use std::collections::VecDeque;
use jak_core::glam::Mat3;
use jak_core::particles::{Library,Overrides,Particles,Sprite,journal::Command};
pub static LIBRARY:OnceLock<Arc<Library>>=OnceLock::new();
static JAK1_VENTS:OnceLock<Arc<Library>>=OnceLock::new();
static HAVEN:OnceLock<Arc<Library>>=OnceLock::new();
const BARREL_FIRE:&str="group-ctysluma-barrel-fire";
const PROPAGANDA:&str="group-ctywide-baron-propoganda-holo";
const DISMANTLE:&str="group-vehicle-explosion";
#[derive(Clone,Copy,PartialEq)]
enum Decor { Barrel, Projector, Dismantle, DroidExplosion, Vent(u8), Sandstorm, DarkEcoDeath, KermitPulse, KermitImpact, Launcher }
/// generic-obs.gc launcher init: outside the beach and the swamp (the citadel's citb-launcher too) its part is group-jungle-launcher.
const LAUNCHER:&str="group-jungle-launcher";
/// kermit.gc: the pulse running down the swamp frog's tongue (kermit-pulse-idle spawns it every frame) and its touch (kermit-pulse-impact).
const KERMIT_PULSE:&str="group-kermit-pulse";
const KERMIT_IMPACT:&str="group-kermit-pulse-impact";
/// JakEnemyKind.SCALE in Jak units: the creatures' metre, not Jak's.
const CREATURE:f32=1.94/(2.5138981*1.4)*BLOCK/jak_core::METER;
/// dark_eco.rs DEATH: Jak 1's death in dark eco (`target-death` 'dark-eco-pool).
const DARK_ECO_DEATH:&str="group-dark-eco-death";
/// The slum barrel (about 2.35 m tall in ctysluma) is a 0.88-block can: its fire at the can's own scale,
/// 0.375 blocks a metre (Adrii 06/11: the flames were a third of that and sat too high).
const BARREL_SCALE:f32=0.375*BLOCK/jak_core::METER;
impl Decor {
 fn one_shot(self)->bool{matches!(self,Self::Dismantle|Self::DroidExplosion|Self::DarkEcoDeath|Self::KermitImpact)}
 fn group(self)->&'static str{match self{Self::Barrel=>BARREL_FIRE,Self::Projector=>PROPAGANDA,Self::Dismantle=>DISMANTLE,Self::DroidExplosion=>"group-kg-small-explosion",Self::Vent(index)=>VENT_GROUPS[index as usize],Self::DarkEcoDeath=>DARK_ECO_DEATH,Self::KermitPulse=>KERMIT_PULSE,Self::KermitImpact=>KERMIT_IMPACT,Self::Launcher=>LAUNCHER,Self::Sandstorm=>unreachable!()}}
 fn scale(self)->f32{match self{Self::Barrel=>BARREL_SCALE,Self::Projector=>0.085,Self::Dismantle=>0.25,Self::DroidExplosion|Self::KermitPulse|Self::KermitImpact|Self::Launcher=>CREATURE,Self::Vent(0..=3)|Self::Sandstorm|Self::DarkEcoDeath=>1.,Self::Vent(_)=>0.62}}
}

const VENT_GROUPS:[&str;8]=[
 "group-part-vent-red-active","group-part-vent-yellow-active","group-part-vent-blue-active","group-part-vent-green-active",
 "group-part-vent-dark-active","group-part-vent-light-active","group-part-vent-dark-touched","group-part-vent-light-touched"
];

const JAK1_VENT_PICKUPS:[[&str;2];4]=[
 ["group-red-collect","group-eco-red-collect"],
 ["group-yellow-collect","group-eco-yellow-collect"],
 ["group-blue-collect","group-eco-blue-collect"],
 ["group-green-collect","group-eco-green-collect"],
];
static TEXTURES:OnceLock<Vec<String>>=OnceLock::new();
#[derive(serde::Serialize,serde::Deserialize)]
#[serde(untagged)]
pub enum WireCommand {Particle(Command),Weapon(WeaponCommand)}
#[derive(serde::Serialize,serde::Deserialize)]
pub struct WeaponCommand {#[serde(rename="WeaponVisuals")]pub visual:jak_core::weapons::presentation::Visuals}
impl WireCommand {fn is_step(&self)->bool{matches!(self,Self::Particle(Command::Step(_)))}}
#[derive(Default)]
struct Sandstorm { strength:f32, carry:[f32;2], rng:u64 }
struct Fx {particles:Particles,weapons:jak_core::weapons::presentation::Presentation,queue:VecDeque<WireCommand>,carry:f64,decor:Option<Decor>,yaw:f32,sandstorm:Sandstorm,at:Vec3}
#[derive(Default)]struct Effects {next:u64,live:HashMap<u64,Fx>}
static EFFECTS:OnceLock<Mutex<Effects>>=OnceLock::new();
fn effects()->&'static Mutex<Effects>{EFFECTS.get_or_init(||Mutex::new(Effects::default()))}
fn lock_fx()->Result<std::sync::MutexGuard<'static,Effects>,String>{let m=effects();match m.lock(){Ok(g)=>Ok(g),Err(p)=>{m.clear_poison();Ok(p.into_inner())}}}

pub fn configure(root:&std::path::Path)->Result<(),String>{
 if LIBRARY.get().is_some(){return Ok(());}
 let source=root.parent().and_then(|p|p.parent()).ok_or("Invalid OpenGOAL export root")?.join("goal_src/jak3");
 let read=|name:&str|std::fs::read_to_string(source.join(name)).map_err(|e|format!("{}: {e}",source.join(name).display()));
 let mut library=Library::default();library.preserve_texture_pages();library.read_texture_names(&read("engine/data/tpages.gc")?,&read("engine/data/textures.gc")?);
 for file in ["engine/common-obs/particle-curves.gc","engine/target/gun/gun-part.gc","engine/target/gun/gun-dark-shot.gc","levels/factory/fac-part.gc","engine/common-obs/water-part.gc","engine/common-obs/water.gc","engine/target/board/board-part.gc","levels/city/traffic/vehicle/vehicle-part.gc","levels/desert/wvehicle/wvehicle-part.gc","engine/target/target-lightjak.gc","engine/common-obs/enemy-part.gc","engine/common-obs/guard-projectile.gc","engine/common-obs/collectables-part.gc"]{library.read(&read(file)?);}
 let desert=read("levels/desert/desert-part.gc")?;
 library.read(&crate::particle_subset::particle_subset(&desert,&[],&[1439,1443],&[]));
 let text=read("engine/target/target-part.gc")?;
 let subset=crate::particle_subset::particle_subset(&text,&["group-target-hit","group-lightning-glow","group-land-poof-stn","group-darkjak-bomb"],&[157,173,187],&["lightning-darkjak-attack","lightning-darkjak-transform"]);
 library.read(&subset.replace("(sp-item 70 :period (seconds 0.5) :length (seconds 0.017))",""));
 if let Some(active)=root.ancestors().find(|p|p.file_name().is_some_and(|n|n=="active")){
  let jak2=active.join("jak2/data/goal_src/jak2");
  if jak2.is_dir(){
   let mut haven=Library::default();
   let files=[("levels/city/slums/ctysluma-part.gc",BARREL_FIRE),
    ("levels/city/ctywide-part.gc",PROPAGANDA),
    ("levels/city/traffic/vehicle/vehicle-part.gc",DISMANTLE)];
   for (file,group) in files{
    let source=jak2.join(file);
    let text=std::fs::read_to_string(&source).map_err(|e|format!("{}: {e}",source.display()))?;
    haven.read_jak2(&crate::particle_subset::particle_subset(&text,&[group],&[],&[]));
    if haven.group(group).is_none(){return Err(format!("Missing original Haven effect {group}"));}
   }
   if !haven.unsupported_callbacks().is_empty(){return Err(format!("Unsupported Haven callbacks: {:?}",haven.unsupported_callbacks()));}
   HAVEN.set(Arc::new(haven)).map_err(|_|"Haven configuration raced")?;
  }
  let vents=active.join("jak1/data/goal_src/jak1/engine/common-obs/collectables-part.gc");
  if let Ok(text)=std::fs::read_to_string(vents){
   let mut jak1=Library::default();
   let groups:Vec<_>=VENT_GROUPS[..4].iter().copied().chain(JAK1_VENT_PICKUPS.into_iter().flatten()).collect();
   jak1.read_jak1(&crate::particle_subset::particle_subset(&text,&groups,&[],&[]));
   // dark_eco.rs particle_library: the death burst from Jak 1's target-part.gc (its own part ids).
   if let Ok(kermit)=std::fs::read_to_string(active.join("jak1/data/goal_src/jak1/levels/swamp/kermit.gc")){
    jak1.read_jak1(&crate::particle_subset::particle_subset(&kermit,&[KERMIT_PULSE,KERMIT_IMPACT],&[],&[]));
   }
   if let Ok(generic)=std::fs::read_to_string(active.join("jak1/data/goal_src/jak1/engine/common-obs/generic-obs.gc")){
    jak1.read_jak1(&crate::particle_subset::particle_subset(&generic,&[LAUNCHER],&[],&[]));
   }
   if let Ok(target)=std::fs::read_to_string(active.join("jak1/data/goal_src/jak1/engine/target/target-part.gc")){
    jak1.read_jak1(&crate::particle_subset::particle_subset(&target,&[DARK_ECO_DEATH],&[],&[]));
   }
   if VENT_GROUPS[..4].iter().all(|g|jak1.group(g).is_some()){
    JAK1_VENTS.set(Arc::new(jak1)).map_err(|_|"Jak I vent configuration raced")?;
   }
  }
  // target-powerup-process: the eco on Jak (Adrii 06/11), Jak 1's own parts moved past Jak 3's, which share their numbers.
  if let Ok(text)=std::fs::read_to_string(active.join("jak1/data/goal_src/jak1/engine/game/powerups.gc")){
   // The launched ones and the ones they hand on to (:next-launcher 258, 272, 276, 280).
   let ids:Vec<u32>=(255..=280).collect();
   let mut ooze=crate::particle_subset::particle_subset(&text,&[],&ids,&[]);
   for &id in ids.iter().rev(){ooze=ooze.replace(&format!("defpart {id}"),&format!("defpart {}",jak_core::eco_power::OOZE_PARTS+id)).replace(&format!("sp-item {id}"),&format!("sp-item {}",jak_core::eco_power::OOZE_PARTS+id)).replace(&format!(":next-launcher {id})"),&format!(":next-launcher {})",jak_core::eco_power::OOZE_PARTS+id));}
   library.read_jak1(&ooze);
  }
  let flut=active.join("jak1/data/goal_src/jak1/levels/flut_common/flut-part.gc");
  if let Ok(text)=std::fs::read_to_string(flut){
   let slam=crate::particle_subset::particle_subset(&text,&["group-flut-attack-strike-ground"],&[],&[]);
   // Put Jak 1 launchers in their own range within this shared runtime bank.
   library.read_jak1(&slam.replace("sp-item 749","sp-item 1749").replace("defpart 749","defpart 1749")
       .replace("sp-item 750","sp-item 1750").replace("defpart 750","defpart 1750"));
  }
 }
 if library.part_count()<150||library.group_count()<30{return Err("Incomplete original particle definitions".into());}
 let mut textures=library.textures();
 if let Some(haven)=HAVEN.get(){textures.extend(haven.textures());}
 if let Some(vents)=JAK1_VENTS.get(){textures.extend(vents.textures());}
 // Arc Wielder's lightning-new appearances also reference the original page.
 for name in ["level-default-sprite/pal-lightning","level-default-sprite/lightning-tile","level-default-sprite/glow-soft","level-default-sprite/gun-yellow-beam","level-default-sprite/gun-blue-beam"] {if !textures.iter().any(|s|s==name){textures.push(name.into());}}
 textures.sort();textures.dedup();TEXTURES.set(textures).map_err(|_|"texture configuration raced")?;
 LIBRARY.set(Arc::new(library)).map_err(|_|"particle configuration raced")?;Ok(())
}
fn texture(name:&str)->Option<usize>{TEXTURES.get()?.binary_search_by(|s|s.as_str().cmp(name)).ok()}
fn translate(c:&mut Command,offset:Vec3){
 use Command::*;match c {
  Target(a,b)=>{*a+=offset;*b+=offset;},Ground(a)=>*a+=offset.y,
  Sources(a)=>for(_,p)in a{*p+=offset;},From(_,_,p,_)|Peace(_,p,_)|Spawn(_,p,_)|Tagged(_,p,_,_)|Keep(_,_,p,_)|Attached(_,_,p,_)|Launch(_,p,_)|Beam(_,p,_,_)|BeamSized(_,p,_,_,_)|Blade(_,p,_,_,_,_)|Flat(_,p,_,_)|Scaled(_,p,_,_)|Tracker(_,_,p)|Var(_,p,_,_,_)=>*p+=offset,
  Move(_,a)=>for(_,p)in a{*p+=offset;},Lightning(a)=>for r in a{r.from+=offset;r.to+=offset;},
  Value(756|757,name,Some(y)) if name=="userdata"=>*y+=offset.y,
  _=>{}
 }
}
#[unsafe(no_mangle)]pub extern "system" fn Java_dev_jakmc_nativebridge_NativeJak_fxManifest(mut env:JNIEnv,_:JClass)->jstring{
 let report=LIBRARY.get().map(|l|json!({"parts":l.part_count(),"groups":l.group_count(),"textures":TEXTURES.get(),"unsupported":l.unsupported_callbacks()})).unwrap_or(json!(null));
 env.new_string(report.to_string()).map_or(std::ptr::null_mut(),|s|s.into_raw())
}
fn create(library:&OnceLock<Arc<Library>>,decor:Option<Decor>)->Result<jlong,String>{
 let library=library.get().ok_or("Original particle library is not configured")?.clone();
 let mut e=lock_fx()?;
 e.next+=1;
 let id=e.next;
 let mut particles=Particles::new(library);
 if let Some(effect)=decor.filter(|d|d.one_shot()){particles.spawn(effect.group(),Vec3::ZERO,Mat3::IDENTITY);}
 e.live.insert(id,Fx{particles,weapons:Default::default(),queue:VecDeque::new(),carry:0.,decor,yaw:0.,sandstorm:Sandstorm{rng:id,..Default::default()},at:Vec3::ZERO});
 Ok(id as jlong)
}
#[unsafe(no_mangle)]pub extern "system" fn Java_dev_jakmc_nativebridge_NativeJak_fxCreate(mut env:JNIEnv,_:JClass)->jlong{
 protect(&mut env,||create(&LIBRARY,None))
}
#[unsafe(no_mangle)]pub extern "system" fn Java_dev_jakmc_nativebridge_NativeJak_fxCreateDecor(mut env:JNIEnv,_:JClass,kind:jint)->jlong{
 protect(&mut env,||{
  let (library,decor)=match kind{
   0=>(&HAVEN,Decor::Barrel),1=>(&HAVEN,Decor::Projector),2=>(&HAVEN,Decor::Dismantle),
   10..=13=>(&JAK1_VENTS,Decor::Vent((kind-10)as u8)),
   14..=17=>(&LIBRARY,Decor::Vent((kind-10)as u8)),
   18=>(&LIBRARY,Decor::Sandstorm),19=>(&LIBRARY,Decor::DroidExplosion),20=>(&JAK1_VENTS,Decor::DarkEcoDeath),
   21=>(&JAK1_VENTS,Decor::KermitPulse),22=>(&JAK1_VENTS,Decor::KermitImpact),23=>(&JAK1_VENTS,Decor::Launcher),
   _=>return Err("Unknown world effect".into())
  };
  if library.get().is_none(){return Ok(0);}
  create(library,Some(decor))
 })
}
/// A lasting world effect's launch point, in blocks from its owner's origin (the frog's pulse along its tongue).
#[unsafe(no_mangle)]
pub extern "system" fn Java_dev_jakmc_nativebridge_NativeJak_fxDecorAt(mut env:JNIEnv,_:JClass,id:jlong,x:f32,y:f32,z:f32){
 protect(&mut env,||{
  let mut e=lock_fx()?;
  let f=e.live.get_mut(&(id as u64)).ok_or("Stale world FX session")?;
  let scale=f.decor.ok_or("Not a world effect")?.scale();
  let at=Vec3::new(x,y,z)*BLOCK/scale;
  if !at.is_finite(){return Err("Nonfinite world effect point".into());}
  f.at=at;Ok(())
 })
}
#[unsafe(no_mangle)]
pub extern "system" fn Java_dev_jakmc_nativebridge_NativeJak_fxSandstorm(mut env:JNIEnv,_:JClass,id:jlong,strength:f32){
 protect(&mut env,||{
  let mut e=lock_fx()?;
  let f=e.live.get_mut(&(id as u64)).ok_or("Stale sandstorm FX session")?;
  if f.decor!=Some(Decor::Sandstorm)||!strength.is_finite(){return Err("Invalid sandstorm FX session".into());}
  f.sandstorm.strength=strength.clamp(0.,1.);
  Ok(())
 })
}
impl Sandstorm {
 fn launch(&mut self,particles:&mut Particles,eye:Vec3){
  if self.strength<=0.01{return;}
  let wind=Vec3::new(1.,0.05,0.35).normalize()*12.*jak_core::METER/60.;
  let rng=&mut self.rng;
  let mut random=||{
   let mut x=*rng;
   x^=x>>12;x^=x<<25;x^=x>>27;
   *rng=x;
   (x.wrapping_mul(0x2545_F491_4F6C_DD1D)>>40) as f32/16_777_216.
  };
  for carry in &mut self.carry {
   let at=eye+Vec3::new(random()-0.6,0.15*random()-0.05,random()-0.5)*50.*jak_core::METER;
   particles.launch_var(1439,at,Mat3::IDENTITY,Overrides{num:Some(1.5*self.strength),vel:Some(wind),..Default::default()},carry);
  }
 }
}
#[unsafe(no_mangle)]
pub extern "system" fn Java_dev_jakmc_nativebridge_NativeJak_fxVentPickup(mut env:JNIEnv,_:JClass,id:jlong,x:f32,y:f32,z:f32,color:jint,start:jni::sys::jboolean){
 protect(&mut env,||{
  let mut e=lock_fx()?;
  let f=e.live.get_mut(&(id as u64)).ok_or("Stale vent FX session")?;
  let groups=JAK1_VENT_PICKUPS.get(color as usize).ok_or("Unknown Jak I eco color")?;
  let target=Vec3::new(x,y,z)*BLOCK;
  if !target.is_finite(){return Err("Nonfinite vent collector".into());}
  let source=Vec3::Y*BLOCK*0.33;
  if start!=0{
   f.particles.spawn_tagged(groups[0],target,Mat3::IDENTITY,1);
   f.particles.spawn_tagged(groups[1],source,Mat3::IDENTITY,2);
  }
  f.particles.move_tagged(1,|_|target);
  f.particles.move_tagged(2,|t|source.lerp(target,t));
  Ok(())
 });
}
#[unsafe(no_mangle)]pub extern "system" fn Java_dev_jakmc_nativebridge_NativeJak_fxDestroy(mut env:JNIEnv,_:JClass,id:jlong){protect(&mut env,||{lock_fx()?.live.remove(&(id as u64));Ok(())});}
#[unsafe(no_mangle)]pub extern "system" fn Java_dev_jakmc_nativebridge_NativeJak_fxBacklog(mut env:JNIEnv,_:JClass,id:jlong)->jint{
 protect(&mut env,||{let e=lock_fx()?;let f=e.live.get(&(id as u64)).ok_or("Stale FX session")?;Ok(f.queue.iter().filter(|c|c.is_step()).count()as jint)})
}
#[unsafe(no_mangle)]pub extern "system" fn Java_dev_jakmc_nativebridge_NativeJak_fxPush(mut env:JNIEnv,_:JClass,id:jlong,data:JString,x:f32,y:f32,z:f32){
 let data:Result<String,_>=env.get_string(&data).map(Into::into);
 protect(&mut env,||{let data=data.map_err(|e|e.to_string())?;if data.len()>2_000_000{return Err("Particle command packet too large".into());}
  let mut commands:Vec<WireCommand>=serde_json::from_str(&data).map_err(|e|e.to_string())?;
  let offset=Vec3::new(x,y,z)*BLOCK;if !offset.is_finite(){return Err("Nonfinite effect origin".into());}
  for c in &mut commands{match c{WireCommand::Particle(c)=>translate(c,offset),WireCommand::Weapon(c)=>{c.visual.validate()?;c.visual.translate(offset);}}}
  let mut e=lock_fx()?;let f=e.live.get_mut(&(id as u64)).ok_or("Stale FX session")?;
  if f.queue.len()+commands.len()>20000{return Err("Particle launch queue overflow".into());}f.queue.extend(commands);Ok(())
 });
}
/// Draw buffers: sprite/lighting quad vertices, both 44 bytes each.
/// Both are direct, reusable buffers, avoiding JSON/per-particle JVM allocations.
#[unsafe(no_mangle)]pub extern "system" fn Java_dev_jakmc_nativebridge_NativeJak_fxDraw(mut env:JNIEnv,_:JClass,id:jlong,dt:f32,view:JByteBuffer,sprites:JByteBuffer,bolts:JByteBuffer)->jlong{
 let result=std::panic::catch_unwind(std::panic::AssertUnwindSafe(||->Result<jlong,String>{
  let bytes=|b:&JByteBuffer|->Result<(*mut u8,usize),String>{Ok((env.get_direct_buffer_address(b).map_err(|e|e.to_string())?,env.get_direct_buffer_capacity(b).map_err(|e|e.to_string())?))};
  let(v,n)=bytes(&view)?;if !matches!(n,44|56|92)||!dt.is_finite(){return Err("Invalid particle view".into());}
  let view=unsafe{std::slice::from_raw_parts(v,n)};
  let(a,n)=bytes(&sprites)?;let(b,m)=bytes(&bolts)?;if n%44!=0||m%44!=0{return Err("Invalid particle draw buffers".into());}
  let out=unsafe{std::slice::from_raw_parts_mut(a,n)};let bolt_out=unsafe{std::slice::from_raw_parts_mut(b,m)};
  let mut e=lock_fx()?;let f=e.live.get_mut(&(id as u64)).ok_or("Stale FX session")?;
  let scale=f.decor.map_or(1.,Decor::scale);
 let eye=vec(view,0)*BLOCK;
 if f.decor==Some(Decor::Projector){
  let desired=eye.x.atan2(eye.z);
  let difference=(desired-f.yaw+std::f32::consts::PI).rem_euclid(std::f32::consts::TAU)-std::f32::consts::PI;
  let limit=200f32.to_radians()*dt.clamp(0.,0.1);
  f.yaw+=difference.clamp(-limit,limit);
 }
 f.particles.set_camera(Mat3::from_rotation_y(-f.yaw)*eye/scale);f.particles.set_view(vec(view,12),vec(view,24),read_f(view,36));
  f.carry+=(dt as f64).clamp(0.,0.1);
  let ordinary=(f.carry*60.).floor().min(6.) as usize;f.carry-=ordinary as f64/60.;
  let queued=f.queue.iter().filter(|c|c.is_step()).count();
  // A loading/render stall must not leave the emitter permanently seconds
  // behind its owner. Retain at most one Minecraft tick for interpolation.
  let steps=ordinary.max(queued.saturating_sub(3)).min(120);
  for _ in 0..steps {
   match f.decor {
    Some(Decor::Sandstorm)=>f.sandstorm.launch(&mut f.particles,eye),
    Some(decor) if !decor.one_shot()=>f.particles.keep(0,decor.group(),f.at,Mat3::IDENTITY),
    _=>{}
   }
   let mut stepped=false;
   while let Some(c)=f.queue.pop_front(){stepped=c.is_step();match c{WireCommand::Particle(c)=>c.apply(&mut f.particles)?,WireCommand::Weapon(c)=>f.weapons.sync(c.visual)?}if stepped{break;}}
   // Effects keep aging after the authoritative producer disconnects/stops.
   if !stepped{f.particles.step(1./60.);f.weapons.age(1./60.);}
  }
  if view.len()>=56 {
   let board=vec(view,44)*BLOCK;
   if board.is_finite(){f.particles.follow_board_joint(board);}
  }
  if view.len()>=92 {
   let muzzle=vec(view,56)*BLOCK;if muzzle.is_finite(){f.weapons.follow_muzzle(muzzle);}
  }
  let mut count=0;
  let eye=vec(view,0)*BLOCK;let forward=vec(view,12).normalize_or(Vec3::NEG_Z);let up=vec(view,24).normalize_or(Vec3::Y);let right=forward.cross(up).normalize_or(Vec3::X);
  let pixels=read_f(view,40)/(2.*(read_f(view,36).to_radians()*0.5).tan());
  for mut s in f.particles.sprites(){
   if f.decor==Some(Decor::Projector){
    fit_projector(&mut s,scale,Mat3::from_rotation_y(f.yaw));
   }else{
    s.pos*=scale;s.size=s.size.map(|v|v*scale);
   }let Some(tex)=texture(&s.texture).or_else(||texture(&format!("level-default-sprite/{}",s.texture)))else{continue};
   let [w,h]=s.size;if (s.pos-eye).dot(forward) < -w.max(h){continue;}
   let axes=match(s.flat,s.stretch){
    (Some(normal),Some(dir))=>{let dir=dir.normalize_or_zero();let side=normal.cross(dir).normalize_or_zero();if side==Vec3::ZERO{continue;}(dir*w*0.5,side*h*0.5)},
    (Some(normal),None)=>{let n=normal.normalize_or(Vec3::Z);if (eye-s.pos).dot(n)<0.{continue;}let t=n.any_orthonormal_vector();let b=n.cross(t);let(sn,cs)=s.rotation.sin_cos();((t*cs+b*sn)*w*0.5,(b*cs-t*sn)*h*0.5)},
    (None,Some(dir))=>{if s.screen_aligned{let Some((a,b))=jak_core::particles::sprite_spline_axes(dir,forward,s.rotation)else{continue;};(a*w*0.5,b*h*0.5)}else{let side=dir.cross(eye-s.pos).normalize_or_zero();if side==Vec3::ZERO{continue;}(dir*w*0.5,side*h*0.5)}},
    (None,None)=>{let(sn,cs)=s.rotation.sin_cos();((right*cs+up*sn)*w*0.5,(up*cs-right*sn)*h*0.5)}
   };
   let(a,b)=axes;let p=if s.corner&15==4{s.pos+a}else{s.pos};let corners=[p-a-b,p+a-b,p+a+b,p-a+b];let uv=jak_core::particles::sprite_uvs(s.corner);
   let area=(w*h*(pixels/(p-eye).dot(forward).max(1e-3)).powi(2)).clamp(0.,1.);
   let rgba=[s.rgba[0]/255.,s.rgba[1]/255.,s.rgba[2]/255.,s.rgba[3]*area/255.];
   let blend=match s.blend{jak_core::particles::Blend::Normal=>0,jak_core::particles::Blend::Subtract=>1,jak_core::particles::Blend::Add=>2,jak_core::particles::Blend::Glow=>3};
   if (count+4)*44>out.len(){break;}
   for i in 0..4{write_vertex(out,&mut count,corners[i]/BLOCK,rgba,uv[i],tex,blend)?;}
  }
  if f.decor.is_some(){return Ok(count as jlong);}
  for (name,quad) in f.weapons.beam_quads(eye){
   if(count+4)*44>out.len(){break;}
   let Some(tex)=texture(&format!("level-default-sprite/{name}"))else{continue;};
   for v in quad{write_vertex(out,&mut count,v.pos/BLOCK,v.rgba,v.st,tex,2)?;}
  }
  // Original gun-part 207: red lasersmoke, 0.075 m wide, 40 m maximum.
  if view.len()>=92 {
   let from=vec(view,68)*BLOCK;let end=vec(view,80)*BLOCK;
   if from.is_finite()&&end.is_finite(){
    let dir=end-from;let side=dir.cross(eye-from).normalize_or_zero()*jak_core::METER*0.0375;
    if let Some(tex)=texture("level-default-sprite/lasersmoke-00") {
     let corners=[from-side,from+side,end+side,end-side];let uv=[[0.,1.],[1.,1.],[1.,0.],[0.,0.]];
     if(count+4)*44<=out.len(){for i in 0..4{write_vertex(out,&mut count,corners[i]/BLOCK,[1.,0.,0.,0.08],uv[i],tex,2)?;}}
    }
   }
  }
  let tex=texture("level-default-sprite/glow-soft").ok_or("Missing gravity ring texture")?;
  for quad in f.weapons.ring_quads(){if(count+4)*44>out.len(){break;}for v in quad{write_vertex(out,&mut count,v.pos/BLOCK,[0.,0.,0.,v.rgba[3]/255.],v.st,tex,0)?;}}
  let mut vertices=0;
  'strips: for strip in f.particles.lightning_strips().into_iter().chain(f.weapons.strips(up)){let qualified=if strip.texture.contains('/') {strip.texture.clone()}else{format!("level-default-sprite/{}",strip.texture)};let Some(tex)=texture(&qualified).or_else(||texture(&strip.texture))else{continue};for i in 2..strip.verts.len(){let tri=if i%2==0{[i-2,i-1,i]}else{[i-1,i-2,i]};if (vertices+4)*44>bolt_out.len(){break 'strips;}for k in [0,1,2,2]{let v=&strip.verts[tri[k]];let mut at=vertices*44;
   for x in [v.pos.x/BLOCK,v.pos.y/BLOCK,v.pos.z/BLOCK,v.rgba[0]/255.,v.rgba[1]/255.,v.rgba[2]/255.,v.rgba[3]/255.,v.st[0],v.st[1]]{bolt_out[at..at+4].copy_from_slice(&x.to_le_bytes());at+=4;}
   for x in [tex as u32,if strip.additive{2u32}else{0u32}]{bolt_out[at..at+4].copy_from_slice(&x.to_le_bytes());at+=4;}vertices+=1;
  }}}
  Ok(((vertices as jlong)<<32)|count as jlong)
 })).unwrap_or_else(|_|Err("Particle renderer panic contained at native boundary".into()));
 match result{Ok(v)=>v,Err(e)=>{let _=env.throw_new("java/lang/IllegalStateException",e);0}}
}

fn fit_projector(sprite:&mut Sprite,scale:f32,basis:Mat3){
 let name=sprite.texture.rsplit('/').next().unwrap_or("");
 let logo=name=="baron-propoganda-logo";
 let glyph=name.starts_with("ticker-");
 let ticker=glyph||(name=="glow-soft"&&sprite.pos.x.abs()>0.001);
 let mut pos=sprite.pos/jak_core::METER*scale;
 let size=if logo{
  if sprite.size[0]/jak_core::METER<5.{0.84/4.6}else{0.94/7.}
 }else if glyph{0.15}else{scale};
 sprite.size=sprite.size.map(|v|v*size*BLOCK/jak_core::METER);
 if ticker{
  pos.x=(sprite.pos.x/jak_core::METER-6.)*(0.8/12.);
  pos.y=0.12;
  pos.z=0.005;
 }else if logo{pos.y=0.51;}
 let edge=(0.485-sprite.size[0]/BLOCK*0.5).max(0.);
 pos.x=pos.x.clamp(-edge,edge);
 pos.z=pos.z.clamp(-edge,edge);
 sprite.pos=basis*pos*BLOCK;
 if logo||glyph{
  // Keep the authored rotating glyph readable from the projector's focus.
  let axis=sprite.stretch.unwrap_or(Vec3::X);
  let local=Vec3::new(axis.x,0.,axis.z).normalize_or(Vec3::X);
  let right=basis*if local.x<0.{-local}else{local};
  sprite.stretch=Some(right);
  sprite.flat=Some(right.cross(Vec3::Y));
  // The source flips UVs as its animated basis rolls over. Our upright basis
  // already resolves that roll; retaining those flips turns the emblem upside down.
  sprite.corner&=!48;
 }else{
  sprite.stretch=sprite.stretch.map(|axis|basis*axis);
  sprite.flat=sprite.flat.map(|axis|basis*axis);
 }
}
fn write_vertex(out:&mut[u8],count:&mut usize,pos:Vec3,rgba:[f32;4],uv:[f32;2],texture:usize,blend:u32)->Result<(),String>{
 if (*count+1)*44>out.len(){return Err("Original particle vertex budget exceeded".into());}let mut at=*count*44;
 for v in [pos.x,pos.y,pos.z,rgba[0],rgba[1],rgba[2],rgba[3],uv[0],uv[1]]{out[at..at+4].copy_from_slice(&v.to_le_bytes());at+=4;}
 for v in [texture as u32,blend]{out[at..at+4].copy_from_slice(&v.to_le_bytes());at+=4;}*count+=1;Ok(())
}

#[cfg(test)]
mod tests {
 use super::*;

 #[test]
 fn praxis_glyphs_stay_upright_in_the_upper_cell(){
  let yaw=Mat3::from_rotation_y(1.2);
  let mut glyph=Sprite{
   pos:Vec3::new(10.,4.,2.)*jak_core::METER,
   size:[0.65*jak_core::METER;2],
   rotation:0.,
   rgba:[128.;4],
   texture:"jak2:ctywide-sprite/ticker-a".into(),
   blend:jak_core::particles::Blend::Normal,
   corner:48,
   stretch:Some(Vec3::NEG_X),
   flat:None,
   screen_aligned:false,
  };
  fit_projector(&mut glyph,Decor::Projector.scale(),yaw);
  assert!((glyph.pos.y/BLOCK-0.12).abs()<1e-5);
  assert_eq!(glyph.corner&48,0,"upright glyph must not retain the source rolling UV flip");
  assert!(glyph.stretch.unwrap().dot(yaw*Vec3::X)>0.999);
  assert!(glyph.flat.unwrap().cross(glyph.stretch.unwrap()).dot(Vec3::Y)>0.999);
  let p=glyph.pos/BLOCK;
  let half=glyph.size[0]/BLOCK/2.;
  assert!(p.x.abs()+half<0.5&&p.z.abs()+half<0.5);
 }
}
