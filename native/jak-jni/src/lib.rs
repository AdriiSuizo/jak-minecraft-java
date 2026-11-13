//! Batched JNI host. No game classes, render engine, or foreign runtime dependencies.
use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};
use glam::{Vec3, Quat};
use jak_core::{Jak, PadInput, Event};
use jak_core::collide::{CollideWorld, Tri, Pat, PatMode, PatMaterial, PatEvent};
use jak_core::projectile::ActorWorld;
use jni::{JNIEnv, JavaVM, objects::{JClass,JByteBuffer,JObject,GlobalRef,JValue},sys::{jlong,jint,jstring}};
use serde_json::{json,Value};
use jni::objects::JString;
use std::sync::Arc;
mod art_group;
mod body_joints;
mod posed_gun;
static POSED_GUN: OnceLock<posed_gun::PosedGun> = OnceLock::new();
static BODY_JOINTS: OnceLock<body_joints::BodyJoints> = OnceLock::new();
mod audio;
mod jak_steps;
mod particle_subset;
mod effects;
mod hellcat_host;
mod wheeled_host;
mod hover_host;
static EXTRA_ART: OnceLock<HashMap<String, art_group::ArtAnim>> = OnceLock::new();
static ANIMS:OnceLock<Arc<jak_core::anim::Anims>>=OnceLock::new();

pub const ABI_VERSION: u32 = 5;
/// One Minecraft block = 1.5625 original meters. Actor stands 1.8 blocks tall.
pub const BLOCK: f32 = 6400.;
// Match the visible upper-body aim rotation in the server-owned pose.
fn gun_twist(j:&Jak,aim:bool,look_yaw:f32)->f32 {
 if !aim||j.state.is_board()||j.state.is_flut(){return 0.;}
 let body=jak_core::math::to_radians(j.control.y_angle());
 let delta=(-body-look_yaw+std::f32::consts::PI).rem_euclid(std::f32::consts::TAU)-std::f32::consts::PI;
 delta.clamp(-1.6,1.6)
}
/// NPC resources use the same authored artist-frame events as the player.
#[unsafe(no_mangle)]
pub extern "system" fn Java_dev_jakmc_nativebridge_NativeJak_animationMetadata(mut env:JNIEnv,_:JClass,root:JString,model:JString,jak1:jni::sys::jboolean)->jstring {
 let root=env.get_string(&root).map(|v|std::path::PathBuf::from(String::from(v))).map_err(|e|e.to_string());
 let model=env.get_string(&model).map(String::from).map_err(|e|e.to_string());
 let result=std::panic::catch_unwind(std::panic::AssertUnwindSafe(||->Result<String,String>{
  let model=model?;if model.is_empty()||!model.bytes().all(|c|c.is_ascii_lowercase()||c.is_ascii_digit()||c==b'-'){return Err("Invalid original model name".into());}
  let bytes=std::fs::read(root?.join("raw_obj").join(format!("{model}-ag.go"))).map_err(|e|e.to_string())?;
  let prefix=format!("{model}-");let anims=if jak1!=0{art_group::read_jak1_named(&bytes,&prefix)}else if bytes.get(8..12)==Some(&4_u32.to_le_bytes()){art_group::read_v4_named(&bytes,&prefix,1.0)}else{art_group::read_named(&bytes,&prefix)};
  if anims.is_empty(){return Err(format!("No original animation metadata for {model}"));}
  let rows:serde_json::Map<String,Value>=anims.into_iter().map(|(name,a)|{
   let frames=a.joints.as_ref().map(Vec::len).or_else(||a.align.as_ref().map(Vec::len)).unwrap_or(1);
   let events:Vec<_>=a.effects.iter().map(|e|json!({"frame":e.frame,"name":e.name,"volume":e.volume.unwrap_or(1.),"pitch":e.pitch.unwrap_or(0.)})).collect();
   let prejoint=if matches!(model.as_str(),"board"|"gun"|"daxter"){a.prejoint}else{None};
   (name,json!({"frames":frames,"speed":a.speed,"artistBase":a.artist_base,"artistStep":a.artist_step,"events":events,"prejoint":prejoint}))
  }).collect();Ok(Value::Object(rows).to_string())
 }));
 match result {Ok(Ok(s))=>match env.new_string(s){Ok(v)=>v.into_raw(),Err(_)=>std::ptr::null_mut()},Ok(Err(e))=>{let _=env.throw_new("java/lang/IllegalStateException",e);std::ptr::null_mut()},Err(_)=>{let _=env.throw_new("java/lang/IllegalStateException","NPC animation metadata panic");std::ptr::null_mut()}}
}
struct Session { jak:Jak, world:World, actors:Actors, audio:audio::JakSoundQueue, power_held:bool, dark_pending:bool, joint_rotations:Vec<Quat>, board_started:Option<i64> }
#[derive(Default)]
struct World { tris:Vec<Tri>,query:Option<GlobalRef>,error:Option<String> }
static JVM:OnceLock<JavaVM>=OnceLock::new();
/// JAKMC_LEDGE_TRACE=1: each step's per-frame trail and the ledge probe's verdict in the pose (diagnostics).
static LEDGE_TRACE:std::sync::LazyLock<bool>=std::sync::LazyLock::new(||std::env::var_os("JAKMC_LEDGE_TRACE").is_some());
#[derive(Default)]
struct Actors { boxes:Vec<(u64,Vec3,Vec3)>, reacting:Vec<(u64,Vec3,Vec3)> }
/// MinecraftCollision.actors' id bits: 61 the body still reacts to a blow (hurt or dying), 62 it is dying (not
/// to be hit again); the entity id below.
const ACTOR_REACTING:u64=1<<61;const ACTOR_DYING:u64=1<<62;const ACTOR_ID:u64=ACTOR_REACTING-1;
impl World {
    fn block_aim(&self, start: Vec3, end: Vec3) -> Result<f32, String> {
        let Some(query) = &self.query else {
            return Ok(1.0);
        };
        let mut env = JVM.get().ok_or("Minecraft JVM unavailable")?
            .attach_current_thread().map_err(|e| e.to_string())?;
        let a = start / BLOCK;
        let b = end / BLOCK;
        let result = env.with_local_frame(8, |env| {
            env.call_method(query.as_obj(), "aimDistance", "(FFFFFF)F", &[
                JValue::Float(a.x), JValue::Float(a.y), JValue::Float(a.z),
                JValue::Float(b.x), JValue::Float(b.y), JValue::Float(b.z),
            ])?.f()
        });
        if result.is_err() {
            let _ = env.exception_clear();
        }
        result.map(|fraction| fraction.clamp(0.0, 1.0)).map_err(|e| e.to_string())
    }
}
impl CollideWorld for World {
 fn fill(&mut self,min:Vec3,max:Vec3,out:&mut Vec<Tri>){
  if let Some(query)=&self.query {
   let result=(||->Result<Vec<Tri>,String>{
    let mut env=JVM.get().ok_or("Minecraft JVM unavailable")?.attach_current_thread().map_err(|e|e.to_string())?;
    let result=env.with_local_frame(16,|env|->jni::errors::Result<Vec<Tri>>{
     let a=min/BLOCK;let b=max/BLOCK;
     let result=env.call_method(query.as_obj(),"triangles","(FFFFFF)Ljava/nio/ByteBuffer;",&[JValue::Float(a.x),JValue::Float(a.y),JValue::Float(a.z),JValue::Float(b.x),JValue::Float(b.y),JValue::Float(b.z)])?.l()?;
     let buffer=JByteBuffer::from(result);let ptr=env.get_direct_buffer_address(&buffer)?;let len=env.get_direct_buffer_capacity(&buffer)?;
     if len%40!=0||len>16_000_000{return Err(jni::errors::Error::NullPtr("host collision buffer size"));}
     let bytes=unsafe{std::slice::from_raw_parts(ptr,len)};let mut tris=Vec::with_capacity(len/40);
     for t in bytes.chunks_exact(40){let v=[vec(t,0)*BLOCK,vec(t,12)*BLOCK,vec(t,24)*BLOCK];if v.iter().any(|p|!p.is_finite()){return Err(jni::errors::Error::NullPtr("host collision coordinates"));}tris.push(Tri{v,pat:pat(read_u(t,36))});}Ok(tris)
    });
    if result.is_err(){let _=env.exception_clear();}result.map_err(|e|format!("Minecraft collision callback: {e}"))
   })();
   match result{Ok(tris)=>out.extend(tris),Err(e)=>self.error=Some(e)}return;
  }
  out.extend(self.tris.iter().filter(|t| {
   let lo=t.v[0].min(t.v[1]).min(t.v[2]); let hi=t.v[0].max(t.v[1]).max(t.v[2]);
   lo.cmple(max).all() && hi.cmpge(min).all()
  }).copied());
 }

 fn ladder(&mut self, feet: Vec3) -> Option<jak_core::ladder::LadderSpan> {
  let query=self.query.as_ref()?;
  let result=(||->Result<_,String>{
   let mut env=JVM.get().ok_or("Minecraft JVM unavailable")?.attach_current_thread().map_err(|e|e.to_string())?;
   let at=feet/BLOCK;
   let result=env.with_local_frame(8,|env|->jni::errors::Result<Option<[f32;8]>>{
    let value=env.call_method(query.as_obj(),"ladder","(FFF)[F",&[
     JValue::Float(at.x),JValue::Float(at.y),JValue::Float(at.z)
    ])?.l()?;
    if value.is_null(){return Ok(None);}
    let array=jni::objects::JFloatArray::from(value);
    let mut span=[0.;8];
    env.get_float_array_region(&array,0,&mut span)?;
    Ok(Some(span))
   });
   if result.is_err(){let _=env.exception_clear();}
   result.map_err(|e|format!("Minecraft ladder callback: {e}"))
  })();
  match result {
   Ok(Some(span))=>Some(jak_core::ladder::LadderSpan{
    anchor:Vec3::from_slice(&span[..3])*BLOCK,away:Vec3::from_slice(&span[3..6]),
    bottom:span[6]*BLOCK,top:span[7]*BLOCK
   }),
   Ok(None)=>None,
   Err(error)=>{self.error=Some(error);None}
  }
 }

}
fn pat(flags:u32)->Pat{
 let mode=match flags&3{1=>PatMode::Wall,2=>PatMode::Obstacle,3=>PatMode::Halfpipe,_=>PatMode::Ground};
 let material=match(flags>>2)&3{1=>PatMaterial::Ice,2=>PatMaterial::PackedIce,3=>PatMaterial::BlueIce,_=>PatMaterial::Stone};
 let event=match(flags>>4)&3{1=>PatEvent::Rail,2=>PatEvent::Slide,_=>PatEvent::None};Pat{mode,material,event}
}
fn closest(p:Vec3,lo:Vec3,hi:Vec3)->Vec3{p.clamp(lo,hi)}
fn segment_aabb(start:Vec3,delta:Vec3,lo:Vec3,hi:Vec3)->Option<f32>{
 let mut a=0f32;let mut b=1f32;
 for i in 0..3 {
  if delta[i].abs()<0.00001 {if start[i]<lo[i]||start[i]>hi[i]{return None;}}
  else {let x=(lo[i]-start[i])/delta[i];let y=(hi[i]-start[i])/delta[i];a=a.max(x.min(y));b=b.min(x.max(y));if a>b{return None;}}
 } Some(a)
}
impl ActorWorld for Actors {
 fn actor_hit(&mut self,start:Vec3,motion:Vec3,radius:f32)->Option<(u64,f32,Vec3)>{
  self.boxes.iter().filter_map(|(id,lo,hi)|segment_aabb(start,motion,*lo-Vec3::splat(radius),*hi+Vec3::splat(radius)).map(|t|(*id,t,closest(start+motion*t,*lo,*hi))))
  .min_by(|a,b|a.1.total_cmp(&b.1))
 }
 fn actors_touching(&mut self,center:Vec3,radius:f32)->Vec<u64>{self.boxes.iter().filter(|(_,lo,hi)|closest(center,*lo,*hi).distance_squared(center)<=radius*radius).map(|(id,_,_)|*id).collect()}
 fn actor_contact(&mut self,key:u64,from:Vec3)->Option<Vec3>{self.boxes.iter().find(|(id,_,_)|*id==key).map(|(_,lo,hi)|closest(from,*lo,*hi))}
 fn actors_near(&mut self,center:Vec3,radius:f32)->Vec<(u64,Vec3)>{self.boxes.iter().filter(|(_,lo,hi)|closest(center,*lo,*hi).distance_squared(center)<=radius*radius).map(|(id,lo,hi)|(*id,(*lo+*hi)*0.5)).collect()}
 fn actor_reacting(&mut self,key:u64)->Option<(Vec3,Vec3)>{self.reacting.iter().find(|(id,_,_)|*id==key).map(|(_,lo,hi)|(*lo,*hi))}
}
#[derive(Default)]
struct Sessions { next:u64, live:HashMap<u64,Session> }
static SESSIONS:OnceLock<Mutex<Sessions>>=OnceLock::new();
fn sessions()->&'static Mutex<Sessions>{SESSIONS.get_or_init(||Mutex::new(Sessions::default()))}
fn lock_sessions()->Result<std::sync::MutexGuard<'static,Sessions>,String>{
 let mutex=sessions();match mutex.lock(){Ok(guard)=>Ok(guard),Err(poison)=>{mutex.clear_poison();Ok(poison.into_inner())}}
}
fn read_f(b:&[u8],at:usize)->f32{f32::from_le_bytes(b[at..at+4].try_into().unwrap())}
fn read_u(b:&[u8],at:usize)->u32{u32::from_le_bytes(b[at..at+4].try_into().unwrap())}
fn vec(b:&[u8],at:usize)->Vec3{Vec3::new(read_f(b,at),read_f(b,at+4),read_f(b,at+8))}
fn pos(p:Vec3)->Value{json!([p.x/BLOCK,p.y/BLOCK,p.z/BLOCK])}
fn event(e:Event)->Value {
 match e {
  Event::Strike{actor,pos:p,dir,damage,sphere}=>{let mut v=json!({"kind":"strike","actor":actor,"pos":pos(p),"dir":pos(dir*BLOCK),"damage":damage});
   if let Some((center,radius))=sphere{v["center"]=pos(center);v["radius"]=json!(radius/BLOCK);}v}
  Event::Impact(h)=>json!({"kind":"impact","actor":h.actor,"pos":pos(h.surface),"dir":pos(h.dir*BLOCK),"normal":pos(h.normal*BLOCK),"damage":h.damage}),
  Event::Hurt{mode,pos:p,buzz}=>json!({"kind":"hurt","mode":format!("{mode:?}"),"pos":pos(p),"buzz":buzz}),
  Event::WorldHit{surface,normal,damage}=>json!({"kind":"world_hit","pos":pos(surface),"normal":pos(normal*BLOCK),"damage":damage}),
  Event::Sound{name,volume,pitch}=>json!({"kind":"sound","name":name,"volume":volume,"pitch":pitch}),
  Event::SoundAt{name,pos:p}=>json!({"kind":"sound_at","name":name,"pos":pos(p)}),
  Event::Freeze{actor}=>json!({"kind":"freeze","actor":actor}),
  Event::Fire{from,dir}=>json!({"kind":"fire","pos":pos(from),"dir":pos(dir*BLOCK)}),
  Event::GunFire{kind,from}=>json!({"kind":"gun_fire","gun":format!("{kind:?}"),"pos":pos(from)}),
  Event::Trace{kind,from,to}=>json!({"kind":"trace","gun":format!("{kind:?}"),"from":pos(from),"to":pos(to)}),
  Event::Explosion{kind,center,radius}=>json!({"kind":"explosion","gun":format!("{kind:?}"),"pos":pos(center),"radius":radius/BLOCK}),
  Event::Lift{actor,seconds}=>json!({"kind":"lift","actor":actor,"seconds":seconds}),
  Event::GravityField{center}=>json!({"kind":"gravity_field","pos":pos(center)}),
  Event::FlutSlam{pos:p}=>json!({"kind":"flut_slam","pos":pos(p)}),
  Event::HardLanding{health}=>json!({"kind":"fall_damage","damage":health}),
  Event::Splash{pos:p,speed}=>json!({"kind":"splash","pos":pos(p),"speed":speed/BLOCK}),
  Event::GroundPound{pos:p}=>json!({"kind":"ground_pound","pos":pos(p)}),
  Event::BoardZap{center,radius}=>json!({"kind":"board_zap","pos":pos(center),"radius":radius/BLOCK}),
  Event::Build{from,dir}=>json!({"kind":"build","pos":pos(from),"dir":pos(dir*BLOCK)}),
  other=>json!({"kind":"effect","event":format!("{other:?}")}),
 }
}
fn protect<T:Default>(env:&mut JNIEnv,f:impl FnOnce()->Result<T,String>)->T{
 match std::panic::catch_unwind(std::panic::AssertUnwindSafe(f)) {
  Ok(Ok(v))=>v,
  Ok(Err(e))=>{let _=env.throw_new("java/lang/IllegalArgumentException",e);T::default()},
  Err(_)=>{let _=env.throw_new("java/lang/IllegalStateException","Jak native boundary panic; session aborted");T::default()}
 }
}
#[unsafe(no_mangle)]
pub extern "system" fn Java_dev_jakmc_nativebridge_NativeJak_abi(_:JNIEnv,_:JClass)->jint {ABI_VERSION as jint}
#[unsafe(no_mangle)]
pub extern "system" fn Java_dev_jakmc_nativebridge_NativeJak_floor(mut env:JNIEnv,_:JClass,id:JString,step:JString)->jint {
 let id:Result<String,_>=env.get_string(&id).map(Into::into);let step:Result<String,_>=env.get_string(&step).map(Into::into);
 protect(&mut env,||Ok(jak_steps::floor_of_block(&id.map_err(|e|e.to_string())?,&step.map_err(|e|e.to_string())?) as jint))
}
/// JakPlayers: a swamp frog's tongue (JakKermit) on Jak, its base in blocks from the session origin.
#[unsafe(no_mangle)]
pub extern "system" fn Java_dev_jakmc_nativebridge_NativeJak_tongue(mut env:JNIEnv,_:JClass,id:jlong,x:f32,y:f32,z:f32){
 protect(&mut env,||{
  let from=Vec3::new(x,y,z)*BLOCK;
  if !from.is_finite(){return Err("Nonfinite tongue origin".into());}
  let mut sessions=lock_sessions()?;
  let session=sessions.live.get_mut(&(id as u64)).ok_or("Stale tongue session")?;
  session.jak.apply_tongue(from);Ok(())
 });
}
/// JakPlayers: Jak stood on a blue eco launcher (JakEcoLauncher), its spring height in metres; whether it took.
#[unsafe(no_mangle)]
pub extern "system" fn Java_dev_jakmc_nativebridge_NativeJak_launch(mut env:JNIEnv,_:JClass,id:jlong,metres:f32)->jni::sys::jboolean{
 protect(&mut env,||{
  let mut sessions=lock_sessions()?;
  let session=sessions.live.get_mut(&(id as u64)).ok_or("Stale launcher session")?;
  Ok(if session.jak.request_launch(jak_core::math::meters(metres)){1u8}else{0u8})
 })
}
/// JakPlayers: Jak stood on a world jump pad, its spring height in metres; whether it took.
#[unsafe(no_mangle)]
pub extern "system" fn Java_dev_jakmc_nativebridge_NativeJak_padBounce(mut env:JNIEnv,_:JClass,id:jlong,metres:f32)->jni::sys::jboolean{
 protect(&mut env,||{
  let mut sessions=lock_sessions()?;
  let session=sessions.live.get_mut(&(id as u64)).ok_or("Stale pad session")?;
  Ok(if session.jak.request_pad_bounce(jak_core::math::meters(metres)){1u8}else{0u8})
 })
}
#[unsafe(no_mangle)]
pub extern "system" fn Java_dev_jakmc_nativebridge_NativeJak_hurt(mut env:JNIEnv,_:JClass,id:jlong,mode:jint,positioned:jni::sys::jboolean,x:f32,y:f32,z:f32){
 protect(&mut env,||{
  let from=Vec3::new(x,y,z)*BLOCK;
  if !from.is_finite(){return Err("Nonfinite hurt origin".into());}
  use jak_core::HurtMode;
  let mode=match mode{0=>HurtMode::Generic,1=>HurtMode::Explode,2=>HurtMode::Burn,3=>HurtMode::Shock,4=>HurtMode::Damage,5=>HurtMode::Fall,_=>return Err("Invalid hurt mode".into())};
  let mut sessions=lock_sessions()?;
  let session=sessions.live.get_mut(&(id as u64)).ok_or("Stale hurt session")?;
  session.jak.hurt(mode,(positioned!=0).then_some(from));
  Ok(())
 });
}
#[unsafe(no_mangle)]
pub extern "system" fn Java_dev_jakmc_nativebridge_NativeJak_bodyDimensions(mut env:JNIEnv,_:JClass,id:jlong,radius:f32,height:f32){
 protect(&mut env,||{
  if !radius.is_finite()||!height.is_finite()||radius<=0.||height<radius*2.||height>8.{return Err("Invalid Minecraft collision dimensions".into());}
  let mut sessions=lock_sessions()?;
  let session=sessions.live.get_mut(&(id as u64)).ok_or("Stale mount session")?;
  session.jak.control.set_body_dimensions(radius*BLOCK,height*BLOCK);
  Ok(())
 });
}
#[unsafe(no_mangle)]
pub extern "system" fn Java_dev_jakmc_nativebridge_NativeJak_create(mut env:JNIEnv,_:JClass,yaw:f32)->jlong {
 protect(&mut env,||{
  if !yaw.is_finite(){return Err("nonfinite yaw".into());}
  let mut jak=Jak::new(Vec3::ZERO,jak_core::math::degrees(-yaw));
  // One stackable light eco item fills two points of the meter.
  jak.lightjak.eco_unit=2.0;
  if let Some(anims)=ANIMS.get(){jak.anims=anims.clone();}
  if let Some(library)=effects::LIBRARY.get(){jak.set_particles(library.clone());jak.particles.as_mut().unwrap().start_journal();}
  jak.control.set_body_dimensions(0.3*BLOCK,1.8*BLOCK);
  jak.size=0.64;
  // stand-in: render_anim measures the drawn mesh (`jak_visual_scale`); here JakRenderer's 1.94 blocks for the model's 2.510636 m.
  jak.drawn_size=Some(1.94/2.510636*BLOCK/jak_core::METER);
  // Minecraft step sizes in the source host (render_anim/src/jak.rs).
  // On foot he steps a stair or a slab, not a whole block (Adrii 05/11: was 1.25).
  jak.foot_smooth_step=Some(0.6*BLOCK);jak.board_smooth_step=Some(1.1*BLOCK);jak.flut_smooth_step=jak.board_smooth_step;
  // A two-block ledge measured from the session's fractional origin can come out a hair under two
  // blocks and was refused at random (Adrii 06/11: block edges not grabbed as in the mashup).
  jak.edge_grab_min_height=1.9*BLOCK;
  let mut s=lock_sessions()?;s.next+=1;let id=s.next;
  s.live.insert(id,Session{jak,world:World::default(),actors:Actors::default(),audio:audio::JakSoundQueue::default(),power_held:false,dark_pending:false,joint_rotations:Vec::new(),board_started:None});Ok(id as jlong)
 })
}
#[unsafe(no_mangle)]
pub extern "system" fn Java_dev_jakmc_nativebridge_NativeJak_configure(mut env:JNIEnv,_:JClass,root:JString)->jint {
 let path:Result<String,_>=env.get_string(&root).map(Into::into);
 protect(&mut env,||{
  let root=std::path::PathBuf::from(path.map_err(|e|e.to_string())?);
  configure_root(&root)
 })
}
/// The animation and effect exports under `root` (the JNI `configure`), also for tests.
pub fn configure_root(root:&std::path::Path)->Result<jint,String>{
 let root=root.to_path_buf();
 {
  effects::configure(&root)?;
  wheeled_host::configure(&root)?;hover_host::configure(&root)?;
  if ANIMS.get().is_some(){return Ok(ANIMS.get().unwrap().timed as jint);}
  let raw=root.join("raw_obj");let mut authored=HashMap::new();
  for e in std::fs::read_dir(&raw).map_err(|e|e.to_string())? {
   let e=e.map_err(|e|e.to_string())?;let name=e.file_name().to_string_lossy().to_string();
   if name.ends_with("-ag.go")&&(name.starts_with("jak")||name.starts_with("daxter")||name.starts_with("board")||name.starts_with("gun")) {
    authored.extend(art_group::read(&std::fs::read(e.path()).map_err(|e|e.to_string())?));
   }
  }
  if let Some(active)=root.ancestors().find(|p|p.file_name().is_some_and(|n|n=="active")){
   let raw_jak1=active.join("jak1/data/decompiler_out/jak1/raw_obj");
   // target.gc: 'launch's own clips, Jak 1's (Adrii 07/11). Daxter's and the joints come from the rigs in JakRenderer.
   if let Ok(data)=std::fs::read(raw_jak1.join("eichar-ag.go")){
    authored.extend(art_group::read_jak1(&data).into_iter().filter_map(|(name,art)|{
     let suffix=name.strip_prefix("eichar-launch-jump")?;
     Some((format!("jakb1-launch-jump{suffix}"),art))
    }));
   }
   for group in ["flut","ice"] {
    if let Ok(data)=std::fs::read(raw_jak1.join(format!("eichar-{group}+0-ag.go"))){
     let prefix=format!("eichar-{group}-");
     authored.extend(art_group::read_jak1(&data).into_iter().filter_map(|(name,art)|{
      let suffix=name.strip_prefix(&prefix)?;
      Some((format!("jakb1-{group}-{suffix}"),art))
     }));
    }
   }
  }
  let extra_art=authored.iter().filter(|(name,_)|name.starts_with("jakb1-ice-")||name.starts_with("jakb-ladder-")||name.starts_with("daxter-ladder-")||name.starts_with("jakb-flut-")||name.starts_with("daxter-flut-")||name.starts_with("jakb1-flut-")||name.starts_with("jakb-pilot-car-")||name.starts_with("daxter-pilot-car-")||name.starts_with("jakb-pilot-wcar-")||name.starts_with("jakb-pilot-bike-")||name.starts_with("daxter-pilot-bike-"))
   .map(|(name,art)|(name.clone(),art.clone())).collect();
  EXTRA_ART.set(extra_art).map_err(|_|"extra animation configuration raced")?;
  let rows:Vec<_>=authored.iter().filter_map(|(name,a)|{
   let n=a.align.as_ref().map(Vec::len).or_else(||a.joints.as_ref().map(Vec::len))?;
   Some((name.as_str(),jak_core::anim::AnimInfo{frames:n.min(65535) as u16,speed:a.speed,artist_base:a.artist_base,artist_step:a.artist_step}))
  }).collect();
  let mut anims=jak_core::anim::Anims::from_rows(rows);
  for (name,a) in &authored {
   if let Some(align)=&a.align {anims=anims.with_align(name,jak_core::anim::AlignTrack{trans:align.iter().map(|v|Vec3::from_slice(v)).collect(),quat:align.iter().map(|v|Quat::from_xyzw(v[3],v[4],v[5],v[6]).normalize()).collect()});}
   let effects=a.effects.iter().filter_map(|e|Some(jak_core::anim::AnimEffect{frame:e.frame,kind:jak_core::anim::EffectKind::of(&e.name)?,volume:e.volume.unwrap_or(1.),pitch:e.pitch.unwrap_or(0.)})).collect();
   anims=anims.with_effects(name,effects);
  }
  if anims.timed<150{return Err(format!("Only {} authored Jak animation timings; incomplete source export",anims.timed));}
  let gun_pose=posed_gun::PosedGun::load(&root,&authored)?;
  POSED_GUN.set(gun_pose).map_err(|_|"posed gun configuration raced")?;
  let joints=body_joints::BodyJoints::load(&root,&mut authored)?;
  BODY_JOINTS.set(joints).map_err(|_|"body joint configuration raced")?;
  let timed=anims.timed;ANIMS.set(Arc::new(anims)).map_err(|_|"animation configuration raced")?;Ok(timed as jint)
 }
}
#[unsafe(no_mangle)]
pub extern "system" fn Java_dev_jakmc_nativebridge_NativeJak_rebase(mut env:JNIEnv,_:JClass,id:jlong,x:f32,y:f32,z:f32){
 protect(&mut env,||{let delta=Vec3::new(x,y,z)*BLOCK;if !delta.is_finite(){return Err("nonfinite rebase".into());}
  lock_sessions()?.live.get_mut(&(id as u64)).ok_or("stale session")?.jak.reframe(Quat::IDENTITY,-delta);Ok(())});
}
#[unsafe(no_mangle)]
pub extern "system" fn Java_dev_jakmc_nativebridge_NativeJak_relocate(mut env:JNIEnv,_:JClass,id:jlong,x:f32,y:f32,z:f32,vx:f32,vy:f32,vz:f32){
 protect(&mut env,|| {
  let at=Vec3::new(x,y,z)*BLOCK;
  let velocity=Vec3::new(vx,vy,vz)*BLOCK;
  if !at.is_finite()||!velocity.is_finite(){return Err("nonfinite rider exit".into());}
  let mut sessions=lock_sessions()?;
  let jak=&mut sessions.live.get_mut(&(id as u64)).ok_or("stale session")?.jak;
  jak.relocate(at);
  jak.control.transv=velocity;
  Ok(())
 });
}
#[unsafe(no_mangle)]
pub extern "system" fn Java_dev_jakmc_nativebridge_NativeJak_destroy(mut env:JNIEnv,_:JClass,id:jlong){
 protect(&mut env,||{lock_sessions()?.live.remove(&(id as u64));Ok(())});
}
#[unsafe(no_mangle)]pub extern "system" fn Java_dev_jakmc_nativebridge_NativeJak_setGrindRails(mut env:JNIEnv,_:JClass,id:jlong,buffer:JByteBuffer){
 let address=env.get_direct_buffer_address(&buffer).map_err(|e|e.to_string());
 let length=env.get_direct_buffer_capacity(&buffer).map_err(|e|e.to_string());
 protect(&mut env,||{
  let length=length?;if length>65536{return Err("Grind rail packet exceeds budget".into());}
  let data=unsafe{std::slice::from_raw_parts(address?,length)};
  let mut cursor=0;let mut rails=Vec::new();
  while cursor<data.len(){
   if cursor+4>data.len()||rails.len()>=128{return Err("Invalid grind rail header".into());}
   let count=read_u(data,cursor) as usize;cursor+=4;
   if !(2..=128).contains(&count)||cursor+count*12>data.len(){return Err("Invalid grind rail points".into());}
   let points=(0..count).map(|i|vec(data,cursor+i*12)*BLOCK).collect();
   rails.push(jak_core::grind::Rail::new(points).ok_or("Degenerate grind rail")?);
   cursor+=count*12;
  }
  lock_sessions()?.live.get_mut(&(id as u64)).ok_or("Stale session")?.jak.replace_grind_rails(Arc::new(rails));
  Ok(())
 })
}
#[unsafe(no_mangle)]pub extern "system" fn Java_dev_jakmc_nativebridge_NativeJak_bindWorld(mut env:JNIEnv,_:JClass,id:jlong,query:JObject){
 let binding=env.new_global_ref(query).map_err(|e|e.to_string());let vm=env.get_java_vm().map_err(|e|e.to_string());
 protect(&mut env,||{let _=JVM.set(vm?);let mut sessions=lock_sessions()?;
  sessions.live.get_mut(&(id as u64)).ok_or("Stale session")?.world.query=Some(binding?);Ok(())});
}
#[unsafe(no_mangle)]
pub extern "system" fn Java_dev_jakmc_nativebridge_NativeJak_ventPulse(mut env:JNIEnv,_:JClass,id:jlong,blue:jni::sys::jboolean,reaction:jint){
 protect(&mut env,||{
  let mut sessions=lock_sessions()?;
  let jak=&mut sessions.live.get_mut(&(id as u64)).ok_or("Stale Jak session")?.jak;
  if blue!=0{jak.get_eco_power(jak_core::eco_power::EcoKind::Blue,jak_core::eco_power::FULL_INC);}
  match reaction{0=>{},1=>{jak.eco_powerup();},2=>{jak.eco_powerup_light();},_=>return Err("Unknown vent reaction".into())}
  Ok(())
 });
}
#[unsafe(no_mangle)]pub extern "system" fn Java_dev_jakmc_nativebridge_NativeJak_setStats(mut env:JNIEnv,_:JClass,id:jlong,health:f32,red:f32,yellow:f32,blue:f32,dark:f32){
 protect(&mut env,||{
  if [health,red,yellow,blue,dark].iter().any(|v|!v.is_finite()){return Err("Nonfinite Minecraft/Jak stats".into());}
  let mut sessions=lock_sessions()?;let j=&mut sessions.live.get_mut(&(id as u64)).ok_or("Stale Jak session")?.jak;
  j.lightjak.health_fraction=health.clamp(0.,1.);
  for(i,v)in[red,yellow,blue,dark].into_iter().enumerate(){j.gun.ammo[i]=v.clamp(0.,jak_core::gun::AMMO_CAP[i]);}Ok(())
 });
}
#[unsafe(no_mangle)]
pub extern "system" fn Java_dev_jakmc_nativebridge_NativeJak_step(mut env:JNIEnv,_:JClass,id:jlong,input:JByteBuffer,collision:JByteBuffer,actors:JByteBuffer,steps:jint)->jstring{
 let packet=std::panic::catch_unwind(std::panic::AssertUnwindSafe(||->Result<String,String>{
  if !(1..=3).contains(&steps){return Err("step count must be 1..3".into());}
  let bytes=|buffer:&JByteBuffer|->Result<&[u8],String>{
   let ptr=env.get_direct_buffer_address(buffer).map_err(|e|e.to_string())?;
   let len=env.get_direct_buffer_capacity(buffer).map_err(|e|e.to_string())?;
   // JVM owns the direct buffers, remains strongly referenced for this JNI call.
   Ok(unsafe{std::slice::from_raw_parts(ptr,len)})
  };
  let b=bytes(&input)?;let c=bytes(&collision)?;let a=bytes(&actors)?;
  if b.len()!=64&&b.len()!=68||c.len()%40!=0||a.len()%32!=0||c.len()>16_000_000||a.len()>262144{return Err("invalid ABI buffer size".into());}
  if b[8..48].chunks_exact(4).any(|v|!f32::from_le_bytes(v.try_into().unwrap()).is_finite()){return Err("nonfinite input".into());}
  let mut s=lock_sessions()?;let s=s.live.get_mut(&(id as u64)).ok_or("stale session")?;
  s.world.tris.clear();
  for t in c.chunks_exact(40){
   let v=[vec(t,0)*BLOCK,vec(t,12)*BLOCK,vec(t,24)*BLOCK];
   if v.iter().any(|v|!v.is_finite()){return Err("nonfinite collision".into());}
   let flags=read_u(t,36);let mode=match flags&3 {1=>PatMode::Wall,2=>PatMode::Obstacle,3=>PatMode::Halfpipe,_=>PatMode::Ground};
   let material=match (flags>>2)&3{1=>PatMaterial::Ice,2=>PatMaterial::PackedIce,3=>PatMaterial::BlueIce,_=>PatMaterial::Stone};
   let event=match (flags>>4)&3{1=>PatEvent::Rail,2=>PatEvent::Slide,_=>PatEvent::None};
   s.world.tris.push(Tri{v,pat:Pat{mode,material,event}});
  }
  s.actors.boxes.clear();s.actors.reacting.clear();
  for a in a.chunks_exact(32){let lo=vec(a,8)*BLOCK;let hi=vec(a,20)*BLOCK;if !lo.is_finite()||!hi.is_finite()||!lo.cmple(hi).all(){return Err("invalid actor bounds".into());}
   let raw=u64::from_le_bytes(a[0..8].try_into().unwrap());let id=raw&ACTOR_ID;
   if raw&ACTOR_DYING==0{s.actors.boxes.push((id,lo,hi));}
   if raw&ACTOR_REACTING!=0{s.actors.reacting.push((id,lo,hi));}}
  let flags=read_u(b,48);
  if s.jak.board_carried && flags&1==0 { s.jak.remove_carried_board(); }
  s.jak.board_carried=flags&1!=0;s.jak.powers_blocked=flags&2!=0;
  s.jak.fire_resistant=flags&4!=0;s.jak.slow_falling=flags&8!=0&&!s.jak.state.is_flut();
  s.jak.water=if flags&16!=0 {Some(read_f(b,40)*BLOCK)}else{None};
  // JakPlayers: Jump Boost's scale (Rust jak.rs 3023, minecraft_survival.rs jump scale).
  s.jak.jump_scale=if b.len()>=68 {let v=read_f(b,64);if v.is_finite(){v}else{1.0}}else{1.0};
  s.jak.movement_scale=read_f(b,44).clamp(0.,4.);
  let dark=read_f(b,52);let light=read_f(b,56);
  if !dark.is_finite()||!light.is_finite(){return Err("nonfinite eco".into());}
  s.jak.dark_eco=dark.clamp(0.,100.);s.jak.lightjak.eco=light.clamp(0.,100.);
  let gun=read_u(b,60) as i32;
  s.jak.gun.equipped=Some(if (0..12).contains(&gun){Some(jak_core::weapons::GunKind::ALL[gun as usize])}else{None});
  s.jak.gun.endless_ammo=flags&32!=0;
  s.jak.camera=jak_core::math::Basis::from_quat(Quat::from_rotation_y(-read_f(b,32).to_radians())*Quat::from_rotation_x(read_f(b,36).to_radians()));
  let yaw=read_f(b,32).to_radians();let pitch=read_f(b,36).to_radians();
  let view = Vec3::new(-yaw.sin()*pitch.cos(),-pitch.sin(),yaw.cos()*pitch.cos());
  let mut pad=PadInput{held:read_u(b,0)&0xffff,pressed:read_u(b,4),left:[read_f(b,8),read_f(b,12)],right:[read_f(b,16),read_f(b,20)],triggers:[read_f(b,24),read_f(b,28)]};
  let host_controls = flags & (1 << 13) != 0;
  let first_person = host_controls && flags & (1 << 14) != 0;
  let board = s.jak.state.is_board();
  let mounted = s.jak.state.is_flut();
  let manual_aim = host_controls && !board && !mounted && !s.jak.swimming() && pad.held & jak_core::pad::button::L2 != 0;
  let facing = Quat::from_rotation_y(-yaw);
  s.jak.gun.aim = None;
  s.jak.gun.aim_point = None;
  s.jak.gun.aim_dir = (!host_controls).then_some(view);
  if host_controls {
      use jak_core::pad::button::*;
      let power = pad.held & L3 != 0;
      let faces = X | SQUARE | CIRCLE | TRIANGLE;
      let cancel = board || mounted || s.jak.swimming() || s.jak.powers_blocked || flags & (1 << 15) != 0;
      let power_edge = pad.pressed & L3 != 0;
      if (power || power_edge) && !s.power_held {
          s.dark_pending = !s.jak.light() && !cancel;
      }
      if cancel || (pad.held | pad.pressed) & faces != 0 {
          s.dark_pending = false;
      }
      if !power && (s.power_held || power_edge) && std::mem::take(&mut s.dark_pending) {
          s.jak.request_dark(!s.jak.dark);
      }
      s.power_held = power;
      if !board {
          pad.held &= !L2;
          pad.pressed &= !L2;
          if power && !cancel {
              pad.held |= L2;
              if pad.pressed & L3 != 0 {
                  pad.pressed |= L2;
              }
          }
      }
      pad.held &= !(L3 | R3);
      pad.pressed &= !(L3 | R3);
      if !board && !mounted && (first_person || manual_aim && pad.left[0].hypot(pad.left[1]) <= 0.2) {
          s.jak.control.quat = facing;
          s.jak.control.dir_targ = facing;
      }
      s.jak.dark_shock.in_head = first_person;
      if (0..12).contains(&gun) && !board {
          if manual_aim || first_person {
              let eye = s.jak.trans() + Vec3::Y * (1.62 * BLOCK);
              let motion = view * jak_core::math::meters(120.0);
              let block = s.world.block_aim(eye, eye + motion)?;
              let fraction = s.actors.actor_hit(eye, motion, 0.0)
                  .map_or(block, |(_, fraction, _)| fraction.min(block));
              s.jak.gun.aim_point = Some(eye + motion * fraction);
          } else {
              let forward = s.jak.orientation() * Vec3::Z;
              let flat = Vec3::new(forward.x, 0.0, forward.z).normalize_or_zero();
              let muzzle = POSED_GUN.get().map_or_else(||s.jak.trans() + Vec3::Y * jak_core::gun::MUZZLE_UP + flat * jak_core::gun::MUZZLE_AHEAD,|rig|s.jak.trans()+s.jak.control.quat*rig.muzzle(&s.jak,0.));
              s.jak.gun.aim = s.actors.boxes.iter().map(|(_, lo, hi)| (*lo + *hi) * 0.5)
                  .filter(|center| {
                      let delta = *center - muzzle;
                      delta.length_squared() <= jak_core::gun::TRACK_FIND_RANGE.powi(2)
                          && delta.normalize_or_zero().dot(flat) > 0.7
                  })
                  .min_by(|a, b| a.distance_squared(muzzle).total_cmp(&b.distance_squared(muzzle)));
          }
      }
  }
  if flags & (1 << 16) != 0 { s.jak.light_shield_hit(); }
  let mut events=Vec::new();
  let mut effects:Vec<effects::WireCommand>=Vec::new();
  let before_clip=s.jak.chan.anim;let before_frame=s.jak.chan.frame;let before_pose=s.jak.pose_anim();
  let before_wings=s.jak.lightjak.wings.map(|w|w.chan);
  s.world.error=None;
  let mut trail:Vec<String>=Vec::new();
  let mut swept=Vec::new();
  for _ in 0..steps{
   let was_kicking=matches!(s.jak.state,jak_core::State::Attack|jak_core::State::AttackAir{..}
       |jak_core::State::BoardJumpKick|jak_core::State::BoardWallKick{..});
   if let Some(body)=BODY_JOINTS.get(){
       body.sample(&mut s.jak,&mut s.joint_rotations);
       if let Some(gun)=POSED_GUN.get(){
           s.jak.gun.muzzle_local=s.jak.gun.equipped.flatten().map(|_|{
               let local=gun.muzzle(&s.jak,gun_twist(&s.jak,manual_aim&&!first_person,yaw));
               if first_person {posed_gun::view_muzzle(local,pitch)}else{local}
           });
           s.jak.gun.muzzle_basis=first_person.then_some(facing);
           s.jak.gun.muzzle_guard=first_person.then_some(Vec3::Y*(1.62*BLOCK));
       }
   }
   s.jak.step(&pad,&mut s.world,Some(&mut s.actors));
   let kicking=was_kicking||matches!(s.jak.state,jak_core::State::Attack|jak_core::State::AttackAir{..}
       |jak_core::State::BoardJumpKick|jak_core::State::BoardWallKick{..});
   let punching=matches!(s.jak.attack.danger,Some(jak_core::attack::Danger::Punch|jak_core::attack::Danger::Uppercut));
   swept.extend(s.jak.attack.swept.drain(..).map(|(c,r)|{let c=c/BLOCK;json!([c.x,c.y,c.z,r/BLOCK,kicking,punching])}));
   if *LEDGE_TRACE {let c=&s.jak.control;let w=c.wall_contact_pt/BLOCK;let n=c.wall_contact_normal;
   trail.push(format!("{:.3},{:.3},{:.3},{} w{:.3},{:.3},{:.3} n{:.2},{:.2},{:.2} x{:.3}",s.jak.trans().y/BLOCK,s.jak.trans().z/BLOCK,c.transv.z/BLOCK,c.status,w.x,w.y,w.z,n.x,n.y,n.z,s.jak.trans().x/BLOCK));}
   if s.jak.state.is_board() {
    s.board_started.get_or_insert(s.jak.time);
   } else {
    s.board_started=None;
   }
   let frame_events=s.jak.take_events();
   for event in &frame_events{
    if let Event::FlutSlam{pos}=event{
     if let Some(particles)=s.jak.particles.as_mut(){
      particles.spawn("group-flut-attack-strike-ground",*pos,glam::Mat3::IDENTITY);
     }
    }
   }
   let mut commands=s.jak.particles.as_mut().map(|p|p.take_journal()).unwrap_or_default();
   // Snapshot the original rope/field after each of the three 60 Hz steps.
   // Replay it before that step's barrier, not all three at the 20 Hz packet end.
   let barrier=commands.iter().rposition(|c|matches!(c,jak_core::particles::journal::Command::Step(_))).unwrap_or(commands.len());
   let after=commands.split_off(barrier);
   effects.extend(commands.into_iter().map(effects::WireCommand::Particle));
   effects.push(effects::WireCommand::Weapon(effects::WeaponCommand{visual:s.jak.weapon_visuals()}));
   effects.extend(after.into_iter().map(effects::WireCommand::Particle));
   audio::emit_jak_sounds_for(&s.jak,0,Some(jak_steps::Floor::from_u8(((flags>>8)&31) as u8)),&frame_events,&mut s.audio);
   let blasts:Vec<_>=frame_events.iter().filter_map(|e|if let Event::Explosion{center,radius,..}=e{Some((*center,*radius))}else{None}).collect();
   let smash=s.jak.dark||matches!(s.jak.attack.danger,Some(jak_core::attack::Danger::Flop|jak_core::attack::Danger::FlopDown));
   s.audio.advance();events.extend(frame_events.into_iter().map(|e|{
    let heavy=match &e{Event::Strike{..}=>smash,Event::Impact(h)=>blasts.iter().any(|(at,r)|h.surface.distance(*at)<=r+256.),_=>false};
    let strike=matches!(&e,Event::Strike{..});
    let mut v=event(e);v["heavy"]=json!(heavy);if strike{v["kick"]=json!(kicking);v["punch"]=json!(punching);}v
   }));pad.pressed=0;
  }
  if let Some(error)=s.world.error.take(){return Err(error);}
  let sound=|r:audio::JakSoundRequest|json!({"name":r.name,"pos":pos(r.position),"gain":r.gain,"pitch":r.pitch});
  let sounds:Vec<_>=s.audio.take_requests().into_iter().map(sound).collect();
  let loops:Vec<_>=s.audio.loop_requests().into_iter().map(sound).collect();
  let healed=std::mem::take(&mut s.jak.lightjak.heal_fraction);
  let j=&s.jak;
  let ride=if j.state.is_flut(){
   Some(json!({"runMix":j.flut.run_mix,"clone":j.flut.clone,"swimming":j.flut.swimming,
    "frameMax":j.anims.max(j.chan.anim),"fixed":j.flut_mount_target().is_some()}))
  }else{None};
  let dismounted=s.jak.flut.dismounted.take().map(|(at,q)|{
   let forward=q*Vec3::Z;
   json!({"pos":pos(at),"yaw":(-forward.x).atan2(forward.z).to_degrees()})
  });
  let ledge_why=if *LEDGE_TRACE && matches!(s.jak.state,jak_core::State::Jump{..}|jak_core::State::DoubleJump{..}|jak_core::State::Falling{..}){s.jak.ledge_why(&mut s.world)}else{String::new()};
  let j=&s.jak;
  let ladder_cycle=j.chan.is_any(&[jak_core::anim::LADDER_UP,jak_core::anim::LADDER_DOWN]);
  let frame_period=if ladder_cycle||matches!(j.chan.func,jak_core::anim::NumFunc::Loop{..}) {
   j.anims.max(j.chan.anim)
  } else { 0. };
  let frame_rate=if before_clip==j.chan.anim{
   let mut delta=j.chan.frame-before_frame;
   if frame_period>0. {delta-=(delta/frame_period).round()*frame_period;}
   (delta/(steps as f32/60.)).clamp(-240.,240.)
  } else {
   // A clip begun within this tick goes on at its own rate: at 0 it stood a tick at each change (Adrii 07/11, the jump).
   match j.chan.func {
    jak_core::anim::NumFunc::Seek{rate,..}|jak_core::anim::NumFunc::Loop{rate}=>(rate*j.anims.info(j.chan.anim).speed*60.).clamp(0.,240.),
    _=>0.
   }
  };
  let state_age=(j.time-j.state_time) as f32/300.;
  let board_pose=s.board_started.map(|started|{
   use jak_core::{State,anim::BOARD_TURN,board_anim::TURN_FRAMES};
   let spring=&j.board.anim;
   let phase=(spring.shown_frame(j.time-j.state_time)+TURN_FRAMES)/(2.*TURN_FRAMES);
   let grind_frame=(j.state==State::BoardGrind).then(||{
    let lean=(f32::from(j.pad.leftx)-128.)/128.;
    let artist=(3.*lean+0.5*jak_core::math::sin(145.63556*(j.time-j.state_time) as f32)).clamp(-5.,5.);
    j.anims.aframe(j.chan.anim,artist)
   });
   json!({
    "rotation":j.orientation().to_array(),"lift":j.control.draw_offset_y/BLOCK,
    "phase":phase,"ahead":spring.tween[0],"side":spring.tween[1],"duck":spring.duck,
    "stance":j.chan.is(BOARD_TURN)||j.state==State::BoardDuckStance,"grindFrame":grind_frame,
    "closing":j.state==State::BoardGetOff,"age":(j.time-started) as f32/300.
   })
  });
  let walk_mix=j.chan.mix.map(|mix|json!({
   "clips":mix.anims().map(|anim|anim.name()),"up":mix.up,"side":mix.side,"run":mix.run
  }));
  let surface_clip=if j.wade.is_some()&&j.state==jak_core::State::Walk{
   Some(if j.wade_anim()==jak_core::anim::WADE_DEEP_WALK{"jakb-wade-deep-walk"}else{"jakb-wade-shallow-walk"})
  }else if !j.gun.out&&j.control.on_surface()&&j.control.ground_pat.material.is_ice(){
   match j.state {
    jak_core::State::Stance if j.control.ctrl_xz_vel>16384.=>Some("jakb1-ice-slide"),
    jak_core::State::Stance=>Some("jakb1-ice-stance"),
    jak_core::State::Walk=>Some("jakb1-ice-skate"),
    _=>None
   }
  }else{None};
  let dark_pose=if j.dark&&j.dark_transition.is_none()&&!j.light()&&!j.swimming()&&!j.state.is_board() {
   use jak_core::State;
   let sequence=match j.state {
    State::Stance=>Some(("stance",None,true)),
    State::Walk=>{
     let clip=if j.velocity().length()>jak_core::math::meters(5.){"run"}else{"walk"};
     Some((clip,None,true))
    },
    State::Attack|State::AttackAir{..}=>Some(("attack-spin",Some("attack-spin-end"),false)),
    State::Jump{..}|State::DoubleJump{..}|State::HighJump{..}=>Some(("jump",None,false)),
    State::HitGround{..}=>Some(("jump-land",None,false)),
    State::DuckStance{..}=>Some(("duck-stance",None,true)),
    _=>None
   };
   sequence.map(|(clip,next,loops)|json!({"clip":clip,"next":next,"loop":loops}))
  }else{None};
  let root_pose=json!({"swimTilt":if j.swimming(){j.swim_tilt}else{0.},
   "lift":j.control.draw_offset_y/BLOCK,"ladderWeight":j.ladder_pose_weight()});
  let transition=json!({"stateAge":state_age,"pushTick":j.push.tick,
    "pushAge":(j.time-j.push.tick) as f32/300.,"pushDuration":j.push.ticks as f32/300.});
  let local_muzzle=POSED_GUN.get().filter(|_|(0..12).contains(&gun)).map(|rig|rig.muzzle(j,gun_twist(j,manual_aim&&!first_person,yaw)));
  let gun=json!({"firstPerson":first_person,"firePoint":(j.gun.fire_point/BLOCK).to_array(),"viewPitch":read_f(b,36),"viewYaw":read_f(b,32),"viewMuzzle":local_muzzle.filter(|_|first_person).map(|p|(posed_gun::view_muzzle(p,pitch)/BLOCK).to_array()),"rigMuzzle":local_muzzle.map(|p|(p/BLOCK).to_array()),"rigScale":POSED_GUN.get().map(|rig|rig.scale),"rigFloor":POSED_GUN.get().map(|rig|rig.floor),"direction":j.gun.fire_dir.to_array(),"kind":j.gun.kind as u8,"out":j.gun.out,"fireAge":if j.gun.fire_time>0{(j.time-j.gun.fire_time)as f32/300.}else{999.},"drawAge":(j.time-j.gun.get_on_time)as f32/300.,"time":j.time as f32/300.,"spin":j.gun.spin,"runMix":j.chan.mix.map_or(0.,|m|m.run)});
  let wings=j.lightjak.wings.filter(|w|w.shown).map(|w|{
   let rate=before_wings.filter(|before|before.anim==w.chan.anim&&w.chan.frame>=before.frame)
       .map_or(0.,|before|((w.chan.frame-before.frame)/(steps as f32/60.)).min(240.));
   json!({"clip":w.chan.anim.name(),"frame":w.chan.frame,"frameRate":rate,
       "loop":matches!(w.chan.func,jak_core::anim::NumFunc::Loop{..})})
  });
  let models:Vec<_>=j.shot_models().into_iter().map(|m|{
   use jak_core::weapons::ShotAnim;let anim=match m.anim{ShotAnim::Rest=>json!({"mode":"rest"}),ShotAnim::Once{clip,age}=>json!({"mode":"once","clip":clip,"age":age}),ShotAnim::BurstFade{burst,fade,age}=>json!({"mode":"burst","burst":burst,"fade":fade,"age":age}),ShotAnim::At{clip,fraction}=>json!({"mode":"at","clip":clip,"fraction":fraction})};
   json!({"id":m.id,"model":m.model,"pos":pos(m.pos),"rotation":m.rot.to_array(),"scale":(m.scale*(4096./BLOCK)).to_array(),"alpha":m.alpha,"anim":anim})
  }).collect();
  let mut out=json!({"abi":ABI_VERSION,"session":id,"pos":pos(j.trans()),"velocity":pos(j.velocity()),"yaw":-jak_core::math::to_radians(j.control.y_angle()).to_degrees(),"clip":j.chan.anim.name(),"frame":j.chan.frame,"frameRate":frame_rate,"loop":frame_period>0.,"framePeriod":frame_period,"state":j.state.name(),"status":j.control.status,"board":j.state.is_board(),"boardPose":board_pose,"rootPose":root_pose,"surfaceClip":surface_clip,"darkPose":dark_pose,"walkMix":walk_mix,"transition":transition,"dark":j.dark,"light":j.light(),"lightInterp":j.lightjak.interp,"entityRate":j.world_time_scale(),"hurt":{"ticks":j.time,"invulnerableUntil":j.hurt_invulnerable_until(),"burnStart":j.hurt_fx.burn_start,"shockUntil":j.hurt_fx.shock_until},"shieldHitAge":(j.time-j.lightjak.shield_hit_time)as f32/300.,"wings":wings,"ride":ride,"dismounted":dismounted,"shield":j.lightjak.shield,"aim":manual_aim,"lookYaw":read_f(b,32),"darkEco":j.dark_eco,"lightEco":j.lightjak.eco,"events":events,"sounds":sounds,"loops":loops,"effects":effects,"gun":gun,"models":models,"ammo":j.gun.ammo,"heal":healed});out["blueEco"]=json!(j.has_eco(jak_core::eco_power::EcoKind::Blue));if !swept.is_empty(){out["attackSpheres"]=json!(swept);}out["ecoTimer"]=json!(j.eco_power_shown().map(|(kind,left)|{use jak_core::eco_power::EcoKind::*;[match kind{Yellow=>0.,Red=>1.,Blue=>2.,Green=>3.},left]}));out["bombBlast"]=json!(j.bomb.blast_at.map(|(at,time)|{let d=(at-j.trans())/BLOCK;[d.x,d.y,d.z,(j.time-time) as f32/300.]}));out["freezeClap"]=json!((j.lightjak.freeze_clap>jak_core::lightjak::LONG_AGO).then_some(j.lightjak.freeze_clap));out["nukeFlash"]=json!(j.weapons.flash.map(|at|(j.time-at) as f32/300.));out["nuke"]=json!(j.nuke_screen().map(|(at,age,small)|{let d=at/BLOCK;json!([d.x,d.y,d.z,age,small])}));out["darkVictims"]=json!(j.dark_shock.tints.iter().map(|(key,f)|json!([key,f])).collect::<Vec<_>>());// The ripple meshes `spawn-ripples` made (from Jak, in blocks: root scale, seconds played), and the wade walk's own frame.
out["ripples"]=json!(j.ripple_meshes().iter().map(|(at,scale,age)|{let d=(*at-j.trans())/BLOCK;json!([d.x,d.y,d.z,scale,age])}).collect::<Vec<_>>());
out["wideActorQuery"]=json!(!j.projectiles.is_empty()||j.weapons.has_active_attacks());
if j.wade.is_some()&&j.state==jak_core::State::Walk{
 // The wade walk's own frame, and how fast it ran over these steps (frames a second, for the frames between).
 let (wade_clip,wade_frame)=j.pose_anim();
 out["wadeFrame"]=json!(wade_frame);
 out["wadeRate"]=json!(if wade_clip==before_pose.0{
  let period=j.anims.max(wade_clip);let mut delta=wade_frame-before_pose.1;
  if period>0. {delta-=(delta/period).round()*period;}
  (delta/(steps as f32/60.)).clamp(-240.,240.)
 }else{0.});
}
if *LEDGE_TRACE {out["ledgeWhy"]=json!(ledge_why);out["trail"]=json!(trail);}Ok(out.to_string())
 })).unwrap_or_else(|_|Err("Jak simulation panicked; native call contained".into()));
 match packet {Ok(packet)=>match env.new_string(packet){Ok(s)=>s.into_raw(),Err(_)=>std::ptr::null_mut()},Err(e)=>{let _=env.throw_new("java/lang/IllegalArgumentException",e);std::ptr::null_mut()}}
}

fn extra_animations(env: JNIEnv, group: &str) -> jstring {
 let rows:HashMap<&str,Value>=EXTRA_ART.get().into_iter().flat_map(|bank|bank.iter())
  .filter(|(name,_)|match group {
   "ladder"=>name.contains("-ladder-"),
   "ice"=>name.starts_with("jakb1-ice-"),
   _=>!name.contains("-ladder-")&&!name.starts_with("jakb1-ice-")
  })
  .filter_map(|(name,art)|Some((name.as_str(),json!({"speed":art.speed,"joints":art.joints.as_ref()?,
    "prejoint":if group=="ladder"{art.prejoint.as_ref()}else{None}}))))
  .collect();
 env.new_string(json!(rows).to_string()).map_or(std::ptr::null_mut(),|s|s.into_raw())
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_dev_jakmc_nativebridge_NativeJak_ridingAnimations(env:JNIEnv,_:JClass)->jstring{
 extra_animations(env,"ride")
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_dev_jakmc_nativebridge_NativeJak_ladderAnimations(env:JNIEnv,_:JClass)->jstring{
 extra_animations(env,"ladder")
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_dev_jakmc_nativebridge_NativeJak_iceAnimations(env:JNIEnv,_:JClass)->jstring{
 extra_animations(env,"ice")
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_dev_jakmc_nativebridge_NativeJak_mountReady(_env:JNIEnv,_:JClass)->jni::sys::jboolean{
 u8::from(ANIMS.get().is_some_and(|a|a.max(jak_core::anim::FLUT1_GET_ON)>24.)
   &&EXTRA_ART.get().is_some_and(|a|a.get("jakb-flut-get-on").is_some_and(|c|c.joints.is_some())))
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_dev_jakmc_nativebridge_NativeJak_mountControl(
 mut env:JNIEnv,_:JClass,id:jlong,action:jint,x:f32,y:f32,z:f32,yaw:f32
){
 protect(&mut env,||{
  let at=Vec3::new(x,y,z)*BLOCK;
  if !at.is_finite()||!yaw.is_finite(){return Err("Invalid mount position".into());}
  let mut sessions=lock_sessions()?;
  let jak=&mut sessions.live.get_mut(&(id as u64)).ok_or("Stale mount session")?.jak;
  match action{
   0=>jak.request_flut(at,Quat::from_rotation_y(-yaw.to_radians())),
   1=>jak.request_flut_off(),
   2=>jak.flut_hit(at),
   _=>return Err("Invalid mount action".into()),
  }
  Ok(())
 });
}

#[cfg(test)]
mod tests{
 use super::*;
 #[test] fn segment_collision_finds_first_entry_and_rejects_parallel_outside(){assert_eq!(segment_aabb(Vec3::new(-2.,0.,0.),Vec3::X*4.,Vec3::splat(-1.),Vec3::ONE),Some(0.25));assert!(segment_aabb(Vec3::new(0.,2.,0.),Vec3::X,Vec3::splat(-1.),Vec3::ONE).is_none());}
 #[test] fn actor_sweep_chooses_nearest_entity_instead_of_iteration_order(){let mut a=Actors{boxes:vec![(2,Vec3::new(7.,-1.,-1.),Vec3::new(8.,1.,1.)),(1,Vec3::new(2.,-1.,-1.),Vec3::new(3.,1.,1.))],reacting:vec![]};assert_eq!(a.actor_hit(Vec3::ZERO,Vec3::X*10.,0.).unwrap().0,1);}
}

#[cfg(test)]
fn configure_test_root(root:&std::path::Path)->Result<jint,String>{
 // The ledge and water fixtures share the native process globals. Rust runs
 // their tests concurrently; initialise those globals once across both modules.
 static READY:OnceLock<Result<jint,String>>=OnceLock::new();
 READY.get_or_init(||configure_root(root)).clone()
}

#[cfg(test)]
mod ledge_with_real_animations {
 use super::*;
 use jak_core::collide::{Tri,Pat,PatMode,CollideWorld};
 struct Cubes(Vec<Tri>);
 impl CollideWorld for Cubes{fn fill(&mut self,_:Vec3,_:Vec3,out:&mut Vec<Tri>){out.extend_from_slice(&self.0);}}
 /// The punch fixture's ledge phase with the exported animations
 /// (`JAKMC_ASSETS`, the jakmc.properties `assets` path): a run and a jump
 /// at a two-block wall of Minecraft cubes.
 #[test]
 fn running_jump_at_two_block_wall(){
  let Ok(root)=std::env::var("JAKMC_ASSETS") else {eprintln!("JAKMC_ASSETS unset");return;};
  configure_test_root(std::path::Path::new(&root)).unwrap();
  let b=BLOCK;let mut tris=Vec::new();
  let mut cube=|x:f32,y:f32,z:f32|{
   let (x0,y0,z0,x1,y1,z1)=(x*b,y*b,z*b,(x+1.)*b,(y+1.)*b,(z+1.)*b);
   let mut q=|v:[Vec3;4],mode|{let pat=Pat{mode,..Default::default()};tris.push(Tri{v:[v[0],v[1],v[2]],pat});tris.push(Tri{v:[v[0],v[2],v[3]],pat});};
   q([Vec3::new(x0,y1,z0),Vec3::new(x0,y1,z1),Vec3::new(x1,y1,z1),Vec3::new(x1,y1,z0)],PatMode::Ground);
   q([Vec3::new(x0,y0,z0),Vec3::new(x1,y0,z0),Vec3::new(x1,y0,z1),Vec3::new(x0,y0,z1)],PatMode::Wall);
   q([Vec3::new(x0,y0,z0),Vec3::new(x0,y0,z1),Vec3::new(x0,y1,z1),Vec3::new(x0,y1,z0)],PatMode::Wall);
   q([Vec3::new(x1,y0,z1),Vec3::new(x1,y0,z0),Vec3::new(x1,y1,z0),Vec3::new(x1,y1,z1)],PatMode::Wall);
   q([Vec3::new(x1,y0,z0),Vec3::new(x0,y0,z0),Vec3::new(x0,y1,z0),Vec3::new(x1,y1,z0)],PatMode::Wall);
   q([Vec3::new(x0,y0,z1),Vec3::new(x1,y0,z1),Vec3::new(x1,y1,z1),Vec3::new(x0,y1,z1)],PatMode::Wall);
  };
  for x in -5..=5 {for z in -5..=6 {cube(x as f32,-1.,z as f32);} for y in 0..std::env::var("JAKMC_WALL_H").ok().and_then(|v|v.parse().ok()).unwrap_or(2) {cube(x as f32,y as f32,4.);}}
  let mut world=Cubes(tris);
  let mut jak=Jak::new(Vec3::new(0.5*b,0.,-0.5*b),0.);
  if let Some(anims)=ANIMS.get(){jak.anims=anims.clone();}
  jak.control.set_body_dimensions(0.3*b,1.8*b);jak.size=0.64;
  jak.foot_smooth_step=Some(1.25*b);jak.board_smooth_step=Some(1.1*b);jak.flut_smooth_step=jak.board_smooth_step;jak.edge_grab_min_height=2.*b;
  let mut log=Vec::new();let mut rotations=Vec::new();let mut actors=Actors::default();
  for f in 0..105 {
   let x=f>=24&&f<60&&std::env::var("JAKMC_NOJUMP").is_err();
   let input=PadInput{held:if x{1}else{0},pressed:if f==24{1}else{0},left:[0.,1.],..Default::default()};
   if std::env::var("JAKMC_BODY").is_ok(){if let Some(body)=BODY_JOINTS.get(){body.sample(&mut jak,&mut rotations);}}
   jak.movement_scale=std::env::var("JAKMC_SCALE").ok().and_then(|v|v.parse().ok()).unwrap_or(jak.movement_scale);
   if std::env::var("JAKMC_ACTORS").is_ok(){jak.step(&input,&mut world,Some(&mut actors));}else{jak.step(&input,&mut world,None);}
   if f>=40&&f<56 {{let c=&jak.control;let w=c.wall_contact_pt/b;let n=c.wall_contact_normal;log.push(format!("f{} {:.3},{:.3},{:.3},{} w{:.3},{:.3},{:.3} n{:.2},{:.2},{:.2}",f,jak.trans().y/b,jak.trans().z/b,c.transv.z/b,c.status,w.x,w.y,w.z,n.x,n.y,n.z));}}
  }
  eprintln!("real anims: {}",log.join(" "));
 }
}

/// The water's effects with the real exports (the same assets the game reads: `JAKMC_ASSETS`, else the default
/// install): the joints the host samples are what the drops and the wade's steps read, and the wake's trail and the
/// ripple meshes come out as the pose and the particles carry them.
#[cfg(test)]
mod water_with_real_data {
 use super::*;
 use jak_core::collide::{Tri,Pat,PatMode,CollideWorld};
 use jak_core::particles::journal::Command;
 struct Floor(Vec<Tri>);
 impl CollideWorld for Floor{fn fill(&mut self,_:Vec3,_:Vec3,out:&mut Vec<Tri>){out.extend_from_slice(&self.0);}}
 fn floor()->Floor{
  let b=BLOCK;let pat=Pat{mode:PatMode::Ground,..Default::default()};
  let q=[Vec3::new(-40.*b,0.,-40.*b),Vec3::new(-40.*b,0.,40.*b),Vec3::new(40.*b,0.,40.*b),Vec3::new(40.*b,0.,-40.*b)];
  Floor(vec![Tri{v:[q[0],q[1],q[2]],pat},Tri{v:[q[0],q[2],q[3]],pat}])
 }
 fn assets()->Option<std::path::PathBuf>{
  let root=std::env::var_os("JAKMC_ASSETS").map(std::path::PathBuf::from).or_else(||std::env::var_os("USERPROFILE").map(|h|std::path::PathBuf::from(h).join(r"Documents\Jak OpenGOAL\active\jak3\data\decompiler_out\jak3")))?;
  root.join("levels/common/jakb-c-lod0.glb").is_file().then_some(root)
 }
 fn jak_in_water(depth:f32)->Option<Jak>{
  let root=assets()?;
  configure_test_root(&root).unwrap();
  let mut jak=Jak::new(Vec3::new(0.,0.,0.),0.);
  jak.anims=ANIMS.get().unwrap().clone();
  jak.set_particles(effects::LIBRARY.get().unwrap().clone());
  jak.particles.as_mut().unwrap().start_journal();
  jak.control.set_body_dimensions(0.3*BLOCK,1.8*BLOCK);jak.size=0.64;
  jak.drawn_size=Some(1.94/2.510636*BLOCK/jak_core::METER);
  jak.water=Some(depth*BLOCK);
  Some(jak)
 }
 fn sample(jak:&mut Jak,rotations:&mut Vec<glam::Quat>){if let Some(body)=BODY_JOINTS.get(){body.sample(jak,rotations);}}

 /// Swimming at the surface of deep water: the host's joints are there (and the toes among them), the drops come off
 /// the ones under the water, the wake's trail follows him and the ripple meshes come with his splashes.
 #[test]
 fn a_swimmer_has_joints_drops_a_wake_trail_and_ripples(){
  let Some(mut jak)=jak_in_water(6.) else{eprintln!("no exports");return;};
  let mut world=floor();let mut rotations=Vec::new();
  jak.control.trans.y=6.*BLOCK-jak_core::swim::SWIM_HEIGHT*jak.drawn_size.unwrap();
  let mut journal=Vec::new();let mut meshes=0;
  for f in 0..180 {
   sample(&mut jak,&mut rotations);
   let input=PadInput{left:[0.,if f>30{1.}else{0.}],..Default::default()};
   jak.step(&input,&mut world,None);
   journal.extend(jak.particles.as_mut().unwrap().take_journal());
   meshes=meshes.max(jak.ripple_meshes().len());
  }
  assert!(jak.joints.len()>40,"{} joints sampled",jak.joints.len());
  let toes=jak.toe_joints.expect("toes");
  assert!(toes.0<jak.joints.len()&&toes.1<jak.joints.len());
  // The drops: a part-756 launch from a joint under the surface, with the water level as its floor.
  let drops=journal.iter().filter(|c|matches!(c,Command::Var(756,at,_,_,_) if at.y<6.*BLOCK)).count();
  assert!(drops>0,"no drops from his joints");
  assert!(journal.iter().any(|c|matches!(c,Command::Value(756,name,Some(y)) if name=="userdata"&&(*y-6.*BLOCK).abs()<1.)),"drops without the water's level");
  // The wake: its trail followed him and the ripple's size range was set.
  let crumbs=journal.iter().filter(|c|matches!(c,Command::TrailCrumb(..))).count();
  assert!(crumbs>60,"{crumbs} trail crumbs");
  assert!(journal.iter().any(|c|matches!(c,Command::Range(758,field,Some(r)) if field=="scale-x"&&(*r-1638.4).abs()<0.01)),"no ripple size range");
  let strips=jak.particles.as_ref().unwrap().trail_strips();
  assert!(!strips.is_empty()&&strips[0].texture=="level-default-water/water-wake");
  assert!(meshes>0,"no ripple meshes");
 }

 /// Diving from the surface: `target-swim-down` plays `jakb-swim-walk-to-down` through with its own motion, sounds
 /// "swim-dive" and splashes once as that animation passes its artist frame 73 (the first splash was at the press),
 /// and only then does the dive proper go on.
 #[test]
 fn a_dive_plays_its_entry_with_the_splash_and_the_sound(){
  let Some(mut jak)=jak_in_water(6.) else{eprintln!("no exports");return;};
  let mut world=floor();let mut rotations=Vec::new();
  jak.control.trans.y=6.*BLOCK-jak_core::swim::SWIM_HEIGHT*jak.drawn_size.unwrap();
  for _ in 0..30 {sample(&mut jak,&mut rotations);jak.step(&PadInput::default(),&mut world,None);}
  jak.particles.as_mut().unwrap().take_journal();jak.take_events();
  let start=jak.trans().y;
  let mut journal=Vec::new();let mut sounds=Vec::new();
  let (mut entry_frames,mut first_sound,mut splash_at,mut deepest_in_entry)=(0,None,None,0f32);
  let mut left=None;
  for f in 0..400 {
   sample(&mut jak,&mut rotations);
   let input=PadInput{held:jak_core::pad::button::SQUARE,pressed:if f==0{jak_core::pad::button::SQUARE}else{0},..Default::default()};
   jak.step(&input,&mut world,None);
   let in_entry=jak.swim.is_some_and(|s|s.entry.is_some());
   if in_entry{entry_frames+=1;deepest_in_entry=deepest_in_entry.max(start-jak.trans().y);}
   else if left.is_none()&&f>0{left=Some(f);}
   for event in jak.take_events(){if let Event::Sound{name,..}=event{sounds.push((f,name));if name=="SWIM_DIVE"&&first_sound.is_none(){first_sound=Some((f,jak.chan.aframe_num(&jak.anims),in_entry));}}}
   let now=jak.particles.as_mut().unwrap().take_journal();
   if splash_at.is_none()&&f>3&&now.iter().any(|c|matches!(c,Command::Spawn(name,..) if name.starts_with("group-part-water-splash"))){splash_at=Some(f);}
   journal.extend(now);
  }
  eprintln!("entry {entry_frames} frames, left at {left:?}, went down {:.2} m in it, dive sound {first_sound:?}, splash at {splash_at:?}, sounds {sounds:?}",deepest_in_entry/jak_core::math::meters(1.0));
  assert!(entry_frames>20,"the entry animation did not play: {entry_frames} frames");
  let (_,aframe,during)=first_sound.expect("swim-dive never sounded");
  assert!(during&&aframe>=73.,"swim-dive at aframe {aframe} (in the entry: {during})");
  assert_eq!(sounds.iter().filter(|(_,name)|*name=="SWIM_DIVE").count(),1,"swim-dive once: {sounds:?}");
  // The entry's own animation took him down some way before the dive went on.
  assert!(deepest_in_entry>jak_core::math::meters(0.5),"the entry moved him {} m",deepest_in_entry/jak_core::math::meters(1.0));
  assert!(jak.swim.unwrap().diving);
 }

 /// The spin out of the wade (circle) and the punch with his feet in the water (square) open with "swim-stroke"
 /// (`target-wade-stance`, `target-wade-walk`, `target-running-attack`); out of the water they are silent of it.
 #[test]
 fn a_spin_or_a_punch_in_the_water_sounds_swim_stroke(){
  use jak_core::pad::button::{CIRCLE,SQUARE};
  let Some(mut jak)=jak_in_water(0.45*1.5625*0.64) else{eprintln!("no exports");return;};
  let mut world=floor();let mut rotations=Vec::new();
  jak.water=Some(jak_core::math::meters(0.9));
  let mut press=|jak:&mut Jak,world:&mut Floor,rotations:&mut Vec<glam::Quat>,button:u32|{
   let mut strokes=0;let mut states=std::collections::BTreeSet::new();
   for f in 0..60 {
    sample(jak,rotations);
    let input=PadInput{held:if f<3{button}else{0},pressed:if f==0{button}else{0},..Default::default()};
    jak.step(&input,world,None);
    states.insert(jak.state.name());
    strokes+=jak.take_events().iter().filter(|e|matches!(e,Event::Sound{name:"SWIM_STROKE",..})).count();
   }
   (strokes,states)
  };
  let (idle,_)=press(&mut jak,&mut world,&mut rotations,0);
  assert_eq!(idle,0);
  assert!(jak.wade.is_some(),"not wading");
  let (spin,states)=press(&mut jak,&mut world,&mut rotations,CIRCLE);
  assert_eq!(spin,1,"one swim-stroke for the spin: {states:?}");
  let (punch,states)=press(&mut jak,&mut world,&mut rotations,SQUARE);
  assert_eq!(punch,1,"one swim-stroke for the punch: {states:?}");
  // Dry: neither.
  jak.water=None;
  let (spin,_)=press(&mut jak,&mut world,&mut rotations,CIRCLE);
  let (punch,_)=press(&mut jak,&mut world,&mut rotations,SQUARE);
  assert_eq!((spin,punch),(0,0));
 }

 /// Audit: each clip's frames and rate as the core plays them, to set against what the models hold.
 #[test]
 fn dump_clip_rates(){
  let Some(jak)=jak_in_water(4.) else{return;};
  let mut out=String::new();
  for name in jak_core::anim::NAMES.iter(){
   let a=jak_core::anim::Anim::by_name(name).unwrap();let i=jak.anims.info(a);
   out+=&format!("{name} {} {}
",i.frames,i.speed);
  }
  if let Ok(path)=std::env::var("JAKMC_CLIP_DUMP"){std::fs::write(path,out).unwrap();}
 }

 /// Adrii 07/11: he drowned on the bottom. With the real animations, X takes him up from it.
 #[test]
 fn x_lifts_him_off_the_bottom(){
  use jak_core::pad::button::{SQUARE,X};
  let Some(mut jak)=jak_in_water(4.) else{eprintln!("no exports");return;};
  let mut world=floor();let mut rotations=Vec::new();
  jak.control.trans.y=4.*BLOCK-jak_core::swim::SWIM_HEIGHT*jak.drawn_size.unwrap();
  for _ in 0..30 {sample(&mut jak,&mut rotations);jak.step(&PadInput::default(),&mut world,None);}
  for f in 0..300 {
   sample(&mut jak,&mut rotations);
   jak.step(&PadInput{held:SQUARE,pressed:if f==0{SQUARE}else{0},..Default::default()},&mut world,None);
  }
  let bottom=jak.trans().y;
  for f in 0..240 {
   sample(&mut jak,&mut rotations);
   jak.step(&PadInput{held:X,pressed:if f==0{X}else{0},..Default::default()},&mut world,None);
  }
  assert!(jak.trans().y>bottom+2.*BLOCK,"rose {} blocks",(jak.trans().y-bottom)/BLOCK);
 }

 /// Letting go of the dive: `target-swim-up` plays `jakb-swim-down-to-up` through (its motion lifting him), sounds
 /// "swim-surface" as its artist frame 240 comes or his head is out, goes out to the stance when his feet are over the
 /// float line (from frame 222 of it), and the stance plays `swim-up-to-stance` before it loops.
 #[test]
 fn a_rise_lifts_him_surfaces_with_its_sound_and_settles(){
  use jak_core::pad::button::SQUARE;
  use jak_core::anim::{SWIM_DOWN_TO_UP,SWIM_UP_TO_STANCE,SWIM_STANCE,SWIM_UP};
  let Some(mut jak)=jak_in_water(6.) else{eprintln!("no exports");return;};
  let mut world=floor();let mut rotations=Vec::new();
  jak.control.trans.y=6.*BLOCK-jak_core::swim::SWIM_HEIGHT*jak.drawn_size.unwrap();
  for _ in 0..30 {sample(&mut jak,&mut rotations);jak.step(&PadInput::default(),&mut world,None);}
  // Down for two seconds (the entry, then the dive).
  for f in 0..120 {
   sample(&mut jak,&mut rotations);
   jak.step(&PadInput{held:SQUARE,pressed:if f==0{SQUARE}else{0},..Default::default()},&mut world,None);
  }
  let deepest=jak.trans().y;
  assert!(jak.swim.unwrap().diving&&jak.swim.unwrap().entry.is_none(),"not diving");
  jak.take_events();
  let (mut seen,mut surface_sounds,mut first_surface,mut settled,mut bubbles)=(std::collections::BTreeSet::new(),0,None,false,Vec::new());
  for f in 0..900 {
   sample(&mut jak,&mut rotations);
   jak.step(&PadInput{held:if jak.swim.is_some_and(|s|s.diving&&!s.rising){jak_core::pad::button::X}else{0},..Default::default()},&mut world,None);
   seen.insert(jak.chan.anim.name());
   bubbles.push(jak.bubble_loops());
   for event in jak.take_events(){if let Event::Sound{name:"SWIM_SURFACE",..}=event{surface_sounds+=1;first_surface.get_or_insert((f,jak.chan.aframe_num(&jak.anims),jak.swim.is_some_and(|s|s.lift.is_some())));}}
   if jak.swim.is_some_and(|s|s.settle){settled=true;}
   if settled&&jak.swim.is_some_and(|s|!s.settle){break;}
  }
  eprintln!("anims {seen:?}, surface sound {first_surface:?}, up from {:.2} m to {:.2} m",deepest/jak_core::math::meters(1.0),jak.trans().y/jak_core::math::meters(1.0));
  assert!(seen.contains(SWIM_DOWN_TO_UP.name())&&seen.contains(SWIM_UP_TO_STANCE.name())&&seen.contains(SWIM_STANCE.name()),"{seen:?}");
  let _=SWIM_UP;
  assert_eq!(surface_sounds,1,"swim-surface once");
  let (_,aframe,in_lift)=first_surface.unwrap();
  assert!(!in_lift||aframe>=240.||jak.trans().y>deepest,"swim-surface at aframe {aframe}");
  assert!(!jak.swim.unwrap().diving&&settled,"he did not come up to the stance");
  // The bubbles: the deep ones all the way up, the surface ones only once he is at the stance.
  assert!(bubbles.iter().take(10).all(|(surface,deep)|!surface&&*deep),"water-bubbles while rising");
  assert!(bubbles.last().is_some_and(|(surface,deep)|*surface&&!deep),"swim-bubbles at the surface: {:?}",bubbles.last());
 }

 /// The ground must be two metres under his feet for the dive; a flop into deep water goes on as one with "swim-flop".
 #[test]
 fn shallow_water_does_not_dive(){
  use jak_core::pad::button::SQUARE;
  let Some(mut jak)=jak_in_water(2.) else{eprintln!("no exports");return;};
  let mut world=floor();let mut rotations=Vec::new();
  jak.control.trans.y=2.*BLOCK-jak_core::swim::SWIM_HEIGHT*jak.drawn_size.unwrap();
  for f in 0..120 {
   sample(&mut jak,&mut rotations);
   jak.step(&PadInput{held:SQUARE,pressed:if f==0{SQUARE}else{0},..Default::default()},&mut world,None);
  }
  assert!(jak.swim.is_some_and(|s|!s.diving),"dived in two blocks of water");
 }

 /// The water's sounds as the host plays them: "swim-enter" as he drops in (not Jak 1's splash), `swim-bubbles` at the
 /// surface, `water-bubbles` from half a second into the dive, each fading away over 960 ticks once its state is left.
 #[test]
 fn the_water_sounds_follow_his_states(){
  use jak_core::pad::button::SQUARE;
  let Some(mut jak)=jak_in_water(6.) else{eprintln!("no exports");return;};
  let mut world=floor();let mut rotations=Vec::new();
  let mut queue=audio::JakSoundQueue::default();
  // Dropped in from a block over the water.
  jak.control.trans.y=7.*BLOCK;
  let (mut enter,mut splash)=(0,0);
  let mut loops=|jak:&Jak,queue:&mut audio::JakSoundQueue,events:Vec<Event>|{
   audio::emit_jak_sounds(jak,&events,queue);
   let names:Vec<_>=queue.take_requests().into_iter().map(|r|r.name).collect();
   let live:std::collections::BTreeMap<_,_>=queue.loop_requests().into_iter().map(|r|(r.name,r.gain)).collect();
   queue.advance();
   (names,live)
  };
  for _ in 0..90 {
   sample(&mut jak,&mut rotations);
   jak.step(&PadInput::default(),&mut world,None);
   let events=jak.take_events();
   let (names,_)=loops(&jak,&mut queue,events);
   enter+=names.iter().filter(|n|**n=="SWIM_ENTER").count();splash+=names.iter().filter(|n|**n=="SPLASH").count();
  }
  assert_eq!((enter,splash),(1,0),"one swim-enter, no splash");
  let (_,live)=loops(&jak,&mut queue,Vec::new());
  assert!(live.get("SWIM_BUBBLES").is_some_and(|g|*g>0.99)&&!live.contains_key("WATER_BUBBLES"),"{live:?}");
  // Dived: the deep ones from half a second in, the surface ones fading away.
  let mut first_deep=None;
  for f in 0..200 {
   sample(&mut jak,&mut rotations);
   jak.step(&PadInput{held:SQUARE,pressed:if f==0{SQUARE}else{0},..Default::default()},&mut world,None);
   let events=jak.take_events();
   let (_,live)=loops(&jak,&mut queue,events);
   if first_deep.is_none()&&live.contains_key("WATER_BUBBLES"){first_deep=Some((f,jak.time-jak.swim.unwrap().state_since));}
  }
  let (frame,age)=first_deep.expect("no water-bubbles");
  assert!(age>=jak_core::math::seconds(0.5)&&frame>20,"water-bubbles at frame {frame}, {age} ticks in");
  let (_,live)=loops(&jak,&mut queue,Vec::new());
  assert!(live.get("WATER_BUBBLES").is_some_and(|g|*g>0.99)&&!live.contains_key("SWIM_BUBBLES"),"{live:?}");
  // X and up: the deep ones fade over 3.2 s after he is at the surface.
  for _ in 0..900 {
   sample(&mut jak,&mut rotations);
   jak.step(&PadInput{held:if jak.swim.is_some_and(|s|s.diving&&!s.rising){jak_core::pad::button::X}else{0},..Default::default()},&mut world,None);
   let events=jak.take_events();
   loops(&jak,&mut queue,events);
   if !jak.swim.unwrap().diving{break;}
  }
  let (_,live)=loops(&jak,&mut queue,Vec::new());
  let fading=live.get("WATER_BUBBLES").copied();
  assert!(fading.is_none_or(|g|g<1.0),"water-bubbles still full at the surface: {live:?}");
  for _ in 0..240 {
   sample(&mut jak,&mut rotations);
   jak.step(&PadInput::default(),&mut world,None);
   let events=jak.take_events();
   loops(&jak,&mut queue,events);
  }
  let (_,live)=loops(&jak,&mut queue,Vec::new());
  assert!(!live.contains_key("WATER_BUBBLES")&&live.contains_key("SWIM_BUBBLES"),"{live:?}");
 }

 /// Wading a hand deep in a flat floor's water: the wade cycle runs with his speed, the real clip's joints give the
 /// toes, and his steps splash the small splash.
 #[test]
 fn a_wader_splashes_at_each_step(){
  let Some(mut jak)=jak_in_water(0.) else{eprintln!("no exports");return;};
  let mut world=floor();let mut rotations=Vec::new();
  // The feet on the floor, the water over his ankles.
  jak.water=Some(jak_core::math::meters(0.9));
  let mut journal=Vec::new();let mut frames=std::collections::BTreeSet::new();
  for f in 0..400 {
   sample(&mut jak,&mut rotations);
   let input=PadInput{left:[0.,if f>20{1.}else{0.}],..Default::default()};
   jak.step(&input,&mut world,None);
   journal.extend(jak.particles.as_mut().unwrap().take_journal());
   if jak.wade.is_some()&&jak.state==jak_core::State::Walk{frames.insert((jak.pose_anim().1/4.) as i32);}
  }
  assert!(jak.wade.is_some(),"not wading: feet {} water {:?}",jak.trans().y,jak.water);
  assert!(frames.len()>=4,"the wade cycle did not run: {frames:?}");
  let steps=journal.iter().filter(|c|matches!(c,Command::Spawn(name,..) if name=="group-part-water-splash-small")).count();
  assert!(steps>=3,"{steps} small splashes in a walk");
 }
}
