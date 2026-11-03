//! Rendering-side original sparticles. Independent of server tick frequency.
use super::*;
use std::collections::VecDeque;
use jak_core::particles::{Library,Particles,journal::Command};
pub static LIBRARY:OnceLock<Arc<Library>>=OnceLock::new();
static TEXTURES:OnceLock<Vec<String>>=OnceLock::new();
struct Fx {particles:Particles,queue:VecDeque<Command>,carry:f64}
#[derive(Default)]struct Effects {next:u64,live:HashMap<u64,Fx>}
static EFFECTS:OnceLock<Mutex<Effects>>=OnceLock::new();
fn effects()->&'static Mutex<Effects>{EFFECTS.get_or_init(||Mutex::new(Effects::default()))}

pub fn configure(root:&std::path::Path)->Result<(),String>{
 if LIBRARY.get().is_some(){return Ok(());}
 let source=root.parent().and_then(|p|p.parent()).ok_or("Invalid OpenGOAL export root")?.join("goal_src/jak3");
 let read=|name:&str|std::fs::read_to_string(source.join(name)).map_err(|e|format!("{}: {e}",source.join(name).display()));
 let mut library=Library::default();library.preserve_texture_pages();library.read_texture_names(&read("engine/data/tpages.gc")?,&read("engine/data/textures.gc")?);
 for file in ["engine/common-obs/particle-curves.gc","engine/target/gun/gun-part.gc","engine/target/gun/gun-dark-shot.gc","levels/factory/fac-part.gc","engine/common-obs/water-part.gc","engine/common-obs/water.gc","engine/target/board/board-part.gc","levels/city/traffic/vehicle/vehicle-part.gc","levels/desert/wvehicle/wvehicle-part.gc","engine/target/target-lightjak.gc","engine/common-obs/enemy-part.gc","engine/common-obs/guard-projectile.gc","engine/common-obs/collectables-part.gc"]{library.read(&read(file)?);}
 let text=read("engine/target/target-part.gc")?;
 let subset=crate::particle_subset::particle_subset(&text,&["group-target-hit","group-lightning-glow","group-land-poof-stn"],&[157,173,187],&["lightning-darkjak-attack","lightning-darkjak-transform"]);
 library.read(&subset.replace("(sp-item 70 :period (seconds 0.5) :length (seconds 0.017))",""));
 if library.part_count()<150||library.group_count()<30{return Err("Incomplete original particle definitions".into());}
 let mut textures=library.textures();
 // Arc Wielder's lightning-new appearances also reference the original page.
 for name in ["level-default-sprite/pal-lightning","level-default-sprite/lightning-tile"] {if !textures.iter().any(|s|s==name){textures.push(name.into());}}
 textures.sort();TEXTURES.set(textures).map_err(|_|"texture configuration raced")?;
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
#[unsafe(no_mangle)]pub extern "system" fn Java_dev_jakmc_nativebridge_NativeJak_fxCreate(mut env:JNIEnv,_:JClass)->jlong{
 protect(&mut env,||{let library=LIBRARY.get().ok_or("Particles not configured")?.clone();let mut e=effects().lock().map_err(|e|e.to_string())?;e.next+=1;let id=e.next;e.live.insert(id,Fx{particles:Particles::new(library),queue:VecDeque::new(),carry:0.});Ok(id as jlong)})
}
#[unsafe(no_mangle)]pub extern "system" fn Java_dev_jakmc_nativebridge_NativeJak_fxDestroy(mut env:JNIEnv,_:JClass,id:jlong){protect(&mut env,||{effects().lock().map_err(|e|e.to_string())?.live.remove(&(id as u64));Ok(())});}
#[unsafe(no_mangle)]pub extern "system" fn Java_dev_jakmc_nativebridge_NativeJak_fxBacklog(mut env:JNIEnv,_:JClass,id:jlong)->jint{
 protect(&mut env,||{let e=effects().lock().map_err(|e|e.to_string())?;let f=e.live.get(&(id as u64)).ok_or("Stale FX session")?;Ok(f.queue.iter().filter(|c|matches!(c,Command::Step(_))).count()as jint)})
}
#[unsafe(no_mangle)]pub extern "system" fn Java_dev_jakmc_nativebridge_NativeJak_fxPush(mut env:JNIEnv,_:JClass,id:jlong,data:JString,x:f32,y:f32,z:f32){
 let data:Result<String,_>=env.get_string(&data).map(Into::into);
 protect(&mut env,||{let data=data.map_err(|e|e.to_string())?;if data.len()>2_000_000{return Err("Particle command packet too large".into());}
  let mut commands:Vec<Command>=serde_json::from_str(&data).map_err(|e|e.to_string())?;
  let offset=Vec3::new(x,y,z)*BLOCK;if !offset.is_finite(){return Err("Nonfinite effect origin".into());}
  for c in &mut commands{translate(c,offset);}
  let mut e=effects().lock().map_err(|e|e.to_string())?;let f=e.live.get_mut(&(id as u64)).ok_or("Stale FX session")?;
  if f.queue.len()+commands.len()>20000{return Err("Particle launch queue overflow".into());}f.queue.extend(commands);Ok(())
 });
}
/// Draw buffers: sprite/lighting quad vertices, both 44 bytes each.
/// Both are direct, reusable buffers, avoiding JSON/per-particle JVM allocations.
#[unsafe(no_mangle)]pub extern "system" fn Java_dev_jakmc_nativebridge_NativeJak_fxDraw(mut env:JNIEnv,_:JClass,id:jlong,dt:f32,view:JByteBuffer,sprites:JByteBuffer,bolts:JByteBuffer)->jlong{
 let result=std::panic::catch_unwind(std::panic::AssertUnwindSafe(||->Result<jlong,String>{
  let bytes=|b:&JByteBuffer|->Result<(*mut u8,usize),String>{Ok((env.get_direct_buffer_address(b).map_err(|e|e.to_string())?,env.get_direct_buffer_capacity(b).map_err(|e|e.to_string())?))};
  let(v,n)=bytes(&view)?;if n!=44||!dt.is_finite(){return Err("Invalid particle view".into());}
  let view=unsafe{std::slice::from_raw_parts(v,n)};
  let(a,n)=bytes(&sprites)?;let(b,m)=bytes(&bolts)?;if n%44!=0||m%44!=0{return Err("Invalid particle draw buffers".into());}
  let out=unsafe{std::slice::from_raw_parts_mut(a,n)};let bolt_out=unsafe{std::slice::from_raw_parts_mut(b,m)};
  let mut e=effects().lock().map_err(|e|e.to_string())?;let f=e.live.get_mut(&(id as u64)).ok_or("Stale FX session")?;
  f.particles.set_camera(vec(view,0)*BLOCK);f.particles.set_view(vec(view,12),vec(view,24),read_f(view,36));
  f.carry+=(dt as f64).clamp(0.,0.1);
  let ordinary=(f.carry*60.).floor().min(6.) as usize;f.carry-=ordinary as f64/60.;
  let queued=f.queue.iter().filter(|c|matches!(c,Command::Step(_))).count();
  // A loading/render stall must not leave the emitter permanently seconds
  // behind its owner. Retain at most one Minecraft tick for interpolation.
  let steps=ordinary.max(queued.saturating_sub(3)).min(120);
  for _ in 0..steps {
   let mut stepped=false;
   while let Some(c)=f.queue.pop_front(){stepped=matches!(c,Command::Step(_));c.apply(&mut f.particles)?;if stepped{break;}}
   // Effects keep aging after the authoritative producer disconnects/stops.
   if !stepped{f.particles.step(1./60.);}
  }
  let mut count=0;
  let eye=vec(view,0)*BLOCK;let forward=vec(view,12).normalize_or(Vec3::NEG_Z);let up=vec(view,24).normalize_or(Vec3::Y);let right=forward.cross(up).normalize_or(Vec3::X);
  let pixels=read_f(view,40)/(2.*(read_f(view,36).to_radians()*0.5).tan());
  for s in f.particles.sprites(){let Some(tex)=texture(&s.texture)else{continue};
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
   for i in 0..4{write_vertex(out,&mut count,corners[i]/BLOCK,rgba,uv[i],tex,blend)?;}
  }
  let mut vertices=0;
  for strip in f.particles.lightning_strips(){let Some(tex)=texture(&strip.texture)else{continue};for i in 2..strip.verts.len(){let tri=if i%2==0{[i-2,i-1,i]}else{[i-1,i-2,i]};for k in [0,1,2,2]{if (vertices+1)*44>bolt_out.len(){return Err("Lightning buffer exhausted".into());}let v=&strip.verts[tri[k]];let mut at=vertices*44;
   for x in [v.pos.x/BLOCK,v.pos.y/BLOCK,v.pos.z/BLOCK,v.rgba[0]/255.,v.rgba[1]/255.,v.rgba[2]/255.,v.rgba[3]/255.,v.st[0],v.st[1]]{bolt_out[at..at+4].copy_from_slice(&x.to_le_bytes());at+=4;}
   for x in [tex as u32,if strip.additive{2u32}else{0u32}]{bolt_out[at..at+4].copy_from_slice(&x.to_le_bytes());at+=4;}vertices+=1;
  }}}
  Ok(((vertices as jlong)<<32)|count as jlong)
 })).unwrap_or_else(|_|Err("Particle renderer panic contained at native boundary".into()));
 match result{Ok(v)=>v,Err(e)=>{let _=env.throw_new("java/lang/IllegalStateException",e);0}}
}

fn write_vertex(out:&mut[u8],count:&mut usize,pos:Vec3,rgba:[f32;4],uv:[f32;2],texture:usize,blend:u32)->Result<(),String>{
 if (*count+1)*44>out.len(){return Err("Original particle vertex budget exceeded".into());}let mut at=*count*44;
 for v in [pos.x,pos.y,pos.z,rgba[0],rgba[1],rgba[2],rgba[3],uv[0],uv[1]]{out[at..at+4].copy_from_slice(&v.to_le_bytes());at+=4;}
 for v in [texture as u32,blend]{out[at..at+4].copy_from_slice(&v.to_le_bytes());at+=4;}*count+=1;Ok(())
}
