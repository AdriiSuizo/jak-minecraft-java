package dev.jakmc.client.sky;

import dev.jakmc.JakData;
import dev.jakmc.JakMod;
import dev.jakmc.RuntimeConfig;
import dev.jakmc.client.JakEffectTypes;
import dev.jakmc.client.JakPoses;
import dev.jakmc.client.JakSky;
import java.io.IOException;
import java.nio.file.Files;
import java.util.*;
import net.fabricmc.fabric.api.client.event.lifecycle.v1.ClientTickEvents;
import net.fabricmc.fabric.api.client.networking.v1.ClientPlayConnectionEvents;
import net.fabricmc.fabric.api.client.rendering.v1.level.*;
import net.minecraft.client.Minecraft;
import net.minecraft.client.resources.sounds.AbstractTickableSoundInstance;
import net.minecraft.client.resources.sounds.SimpleSoundInstance;
import net.minecraft.client.resources.sounds.SoundInstance;
import net.minecraft.client.renderer.texture.DynamicTexture;
import net.minecraft.client.renderer.texture.OverlayTexture;
import net.minecraft.resources.Identifier;
import net.minecraft.sounds.SoundEvent;
import net.minecraft.sounds.SoundSource;
import net.minecraft.world.entity.LightningBolt;
import net.minecraft.world.level.ClipContext;
import net.minecraft.world.phys.HitResult;
import net.minecraft.world.phys.Vec3;
import net.minecraft.world.phys.shapes.CollisionContext;
import org.joml.Matrix3f;
import org.joml.Vector3f;

/** What falls and flashes (render_anim/jak_weather.rs): Jak 3's rain and Snowy Mountain's snow as weather-part.gc's
 *  sparticles (jak_mode/particles.rs, the parts these use), lightning's bolts, and the weather's sounds. */
public final class JakWeatherFx {
 // jak_mode particles.rs: 4096 units a meter, 65536 to the turn, 300 Hz ticks, the 2D system's 1920.
 private static final float METER=4096,ANGLE=(float)(Math.PI*2/65536),TICKS=300;
 private static final int MAX_PARTICLES=1920;
 // The port's scale (native jak-jni BLOCK): Jak units a block.
 private static final float BLOCK=6400;
 private static final Set<String> FIELDS=Set.of("num","x","y","z","scale-x","scale-y","rot-z","rot-y","rot-x","r","g","b","a","vel-x","vel-y","vel-z",
  "scalevel-x","scalevel-y","rotvel-z","rotvel-x","rotvel-y","userdata","fade-r","fade-g","fade-b","fade-a","accel-x","accel-y","accel-z","friction",
  "timer","next-time","conerot-x","conerot-y","conerot-z","conerot-radius","rotate-x","rotate-y","rotate-z","omega","launchrot-x","launchrot-y","launchrot-z");
 /** A spec value: base + random * range, base + steps * step, or another field's. */
 private record Spec(String key,int kind,float a,float b,float c,String copy){}
 private static final class Part{String texture;final List<Spec> specs=new ArrayList<>();int blend,corner,next=-1;float rotX;boolean alongZ,motionBlur;String func,birth;}
 private static final class Particle{
  int part,launcher,corner;final Vector3f pos=new Vector3f(),vel=new Vector3f(),accel=new Vector3f();Vector3f flat;
  float friction,rot,rotvel,timer,nextTime=Float.NaN,age,floor=Float.NaN;final float[] scale=new float[2],scalevel=new float[2],rgba=new float[4],fade=new float[4];
 }
 private static final Map<Integer,Part> PARTS=new HashMap<>();
 private static boolean loaded;
 private static final List<Particle> LIVE=new ArrayList<>();
 private static final List<float[]> LANDED=new ArrayList<>();
 private static int seed=0x9e3779b9;
 private static double carry;
 private static final float[] ACCUM=new float[4];
 private static Vec3 origin,lastFeet;
 private static long last;
 private static float drips;
 private static Loop rainHiss,swampWind;
 private static final Map<String,Identifier> TEXTURES=new HashMap<>();
 private static final Set<String> MISSING=new HashSet<>();
 private record Strike(Vec3[] points,long at){}
 private static final List<Strike> STRIKES=new ArrayList<>();
 private static final Set<Integer> STRUCK=new HashSet<>();
 public static void initialize(){
  LevelRenderEvents.COLLECT_SUBMITS.register(JakWeatherFx::submit);
  ClientTickEvents.END_CLIENT_TICK.register(JakWeatherFx::bolts);
  ClientPlayConnectionEvents.DISCONNECT.register((handler,client)->client.execute(JakWeatherFx::clear));
 }
 private static void clear(){LIVE.clear();LANDED.clear();STRIKES.clear();STRUCK.clear();origin=null;lastFeet=null;stop();}
 private static void stop(){if(rainHiss!=null){rainHiss.end();rainHiss=null;}if(swampWind!=null){swampWind.end();swampWind=null;}}

 // ---- particles.rs: the source read (parse, read_part), the parts weather-part.gc defines ----
 private static void load(){
  if(loaded)return;loaded=true;
  var file=RuntimeConfig.assets.getParent().getParent().resolve("goal_src/jak3/engine/gfx/mood/weather-part.gc");
  try{for(Object form:parse(Files.readString(file)))if(form instanceof List<?> l&&"defpart".equals(symbol(l.isEmpty()?null:l.get(0))))readPart(l);}
  catch(IOException e){JakMod.LOG.error("Original weather particles unavailable: {}",file,e);}
  JakMod.LOG.info("Original weather sparticles: {} parts",PARTS.size());
 }
 private static List<Object> parse(String source){
  Deque<List<Object>> stack=new ArrayDeque<>();stack.push(new ArrayList<>());StringBuilder atom=new StringBuilder();
  for(int i=0;i<source.length();i++){
   char c=source.charAt(i);
   if(c==';'){flush(atom,stack);while(i<source.length()&&source.charAt(i)!='\n')i++;}
   else if(c=='"'){flush(atom,stack);StringBuilder text=new StringBuilder();for(i++;i<source.length()&&source.charAt(i)!='"';i++){if(source.charAt(i)=='\\')i++;if(i<source.length())text.append(source.charAt(i));}stack.peek().add(text.toString());}
   else if(c=='('){flush(atom,stack);stack.push(new ArrayList<>());}
   else if(c==')'){flush(atom,stack);if(stack.size()>1){var list=stack.pop();stack.peek().add(list);}}
   else if(Character.isWhitespace(c))flush(atom,stack);
   else atom.append(c);
  }
  flush(atom,stack);
  while(stack.size()>1)stack.pop();
  return stack.peek();
 }
 private static void flush(StringBuilder atom,Deque<List<Object>> stack){if(!atom.isEmpty()){stack.peek().add(atom.toString());atom.setLength(0);}}
 private static String symbol(Object s){return s instanceof String t?(t.startsWith("'")?t.substring(1):t):null;}
 private static Float number(Object s){
  if(s instanceof String t){
   try{return t.startsWith("#x")?(float)Long.parseLong(t.substring(2),16):Float.parseFloat(t);}catch(NumberFormatException e){return null;}
  }
  if(s instanceof List<?> l&&l.size()>1){
   String unit=symbol(l.get(0));Float v=number(l.get(1));
   if(unit==null||v==null)return null;
   return switch(unit){case "meters"->v*METER;case "degrees"->v*65536f/360;case "seconds"->v*TICKS;default->null;};
  }
  return null;
 }
 private static Object keyword(List<?> items,String key){for(int i=0;i+1<items.size();i++)if(key.equals(symbol(items.get(i))))return items.get(i+1);return null;}
 private static void readPart(List<?> items){
  Float id=items.size()>1?number(items.get(1)):null;
  if(id==null||!(keyword(items,":init-specs") instanceof List<?> specs))return;
  var part=new Part();String stored=null;
  for(Object o:specs){
   if(!(o instanceof List<?> spec)||spec.isEmpty())continue;
   String key=symbol(spec.get(0));if(key==null)continue;
   key=key.startsWith(":")?key.substring(1):key;
   List<?> args=spec.subList(1,spec.size());
   Object first=args.isEmpty()?null:args.get(0);
   switch(key){
    case "rot-x"->{Float v=number(first);if(v!=null){part.corner=v.intValue()&255;part.rotX=v;Float range=args.size()>1?number(args.get(1)):null;if(range!=null)part.specs.add(new Spec(key,0,v,range,0,null));}}
    case "texture"->{if(first instanceof List<?> t&&!t.isEmpty())part.texture=symbol(t.get(0));}
    case "flags"->{if(first instanceof List<?> flags)for(Object f:flags){
     String flag=symbol(f);if(flag==null)continue;
     switch(flag){
      case "glow"->part.blend=Math.max(part.blend,3);
      case "sp-cpuinfo-flag-3","bit3"->part.blend=Math.max(part.blend,2);
      case "sp-cpuinfo-flag-4","bit4"->part.blend=Math.max(part.blend,1);
      case "launch-along-z"->part.alongZ=true;
      default->{}
     }}}
    case "func"->{part.func=symbol(first);part.motionBlur|=part.func!=null&&Set.of("sparticle-motion-blur","check-water-level-drop-motion","check-water-level-drop-and-die-motion").contains(part.func);}
    case "birth-func"->part.birth=symbol(first);
    // The texture sequence names ids from textures.gc, which jak_weather.rs's library never reads: the part's own texture is drawn.
    case "userdata"->{Float v=number(first);if(v!=null)part.specs.add(new Spec(key,0,v,0,0,null));}
    case "next-launcher"->{Float v=number(first);if(v!=null)part.next=v.intValue();}
    default->{
     if(!FIELDS.contains(key))continue;
     Spec value;
     if("*sp-temp*".equals(symbol(first))){if(stored==null)continue;value=new Spec(key,2,0,0,0,stored);}
     else if(":copy".equals(symbol(first))){String other=args.size()>1?symbol(args.get(1)):null;if(other==null||!FIELDS.contains(other))continue;value=new Spec(key,2,0,0,0,other);}
     else{
      List<Float> n=new ArrayList<>();for(Object a:args){Float v=number(a);if(v!=null)n.add(v);}
      if(n.isEmpty())continue;
      if(n.size()==1)value=new Spec(key,0,n.get(0),0,0,null);
      else if(n.size()==2)value=new Spec(key,0,n.get(0),n.get(1),0,null);
      else{boolean integer=args.get(1) instanceof String s&&s.matches("-?\\d+");value=integer?new Spec(key,1,n.get(0),n.get(1),n.get(2),null):new Spec(key,0,n.get(0),n.get(1)*n.get(2),0,null);}
     }
     part.specs.add(value);
     for(Object a:args)if(":store".equals(symbol(a)))stored=key;
    }
   }
  }
  PARTS.put(id.intValue(),part);
 }
 // particles.rs random: xorshift32.
 private static float random(){seed^=seed<<13;seed^=seed>>>17;seed^=seed<<5;return (seed>>>8)/(float)(1<<24);}
 // particles.rs Part::value.
 private static Float value(Part part,String field,Map<String,Float> taken){
  Spec v=null;for(var s:part.specs)if(s.key().equals(field))v=s;
  if(v==null)return null;
  return switch(v.kind()){
   case 0->v.a()+random()*v.b();
   case 1->v.a()+Math.min((float)Math.floor(random()*(v.b()+1)),Math.max(v.b(),0))*v.c();
   default->taken.get(v.copy());
  };
 }
 private static float rangeOf(Part part,String field){Spec v=null;for(var s:part.specs)if(s.key().equals(field))v=s;return v!=null&&v.kind()==0?v.b():0;}
 private static Matrix3f xyz(Map<String,Float> t,String x,String y,String z){
  return new Matrix3f().rotateZ(t.getOrDefault(z,0f)*ANGLE).rotateY(t.getOrDefault(y,0f)*ANGLE).rotateX(t.getOrDefault(x,0f)*ANGLE);
 }
 // particles.rs birth_with (the fields weather-part.gc's parts use).
 private static Particle birth(int id,Vector3f at,Matrix3f basis){
  if(LIVE.size()>=MAX_PARTICLES)return null;
  var part=PARTS.get(id);if(part==null)return null;
  Map<String,Float> t=new HashMap<>();
  for(var s:part.specs){Float v=value(part,s.key(),t);if(v!=null)t.put(s.key(),v);}
  Float nextTime=null;for(var s:part.specs)if(s.key().equals("next-time")){nextTime=t.get("next-time");break;}
  var cone=xyz(t,"conerot-x","conerot-y","conerot-z");
  var launch=xyz(t,"launchrot-x","launchrot-y","launchrot-z");
  var frame=new Matrix3f(basis).mul(xyz(t,"rotate-x","rotate-y","rotate-z"));
  float radius=t.getOrDefault("conerot-radius",0f);
  var offset=new Vector3f(t.getOrDefault("x",0f),t.getOrDefault("y",0f),t.getOrDefault("z",0f)).add(cone.transform(new Vector3f(0,1,0)).mul(radius));
  var p=new Particle();p.part=p.launcher=id;p.corner=part.corner;
  frame.transform(p.pos.set(offset)).add(at);
  new Matrix3f(frame).mul(cone).mul(launch).transform(p.vel.set(t.getOrDefault("vel-x",0f),t.getOrDefault("vel-y",0f),t.getOrDefault("vel-z",0f)));
  p.accel.set(t.getOrDefault("accel-x",0f),t.getOrDefault("accel-y",0f),t.getOrDefault("accel-z",0f));
  if(!part.alongZ)frame.transform(p.accel);
  p.friction=t.getOrDefault("friction",1f);
  p.scale[0]=t.getOrDefault("scale-x",METER);p.scale[1]=t.getOrDefault("scale-y",p.scale[0]);
  p.scalevel[0]=t.getOrDefault("scalevel-x",0f);p.scalevel[1]=t.getOrDefault("scalevel-y",0f);
  p.rot=t.getOrDefault("rot-z",t.getOrDefault("rot-y",0f))*ANGLE;p.rotvel=t.getOrDefault("rotvel-z",0f)*ANGLE;
  String[] c={"r","g","b","a"},f={"fade-r","fade-g","fade-b","fade-a"};
  for(int i=0;i<4;i++){p.rgba[i]=t.getOrDefault(c[i],0f);p.fade[i]=t.getOrDefault(f[i],0f);}
  p.timer=(float)Math.floor(t.getOrDefault("timer",TICKS));
  if(nextTime!=null)p.nextTime=(float)Math.floor(nextTime);
  LIVE.add(p);return p;
 }
 // particles.rs launch_var_on: the part's number (its base set), carried; each one given its floor.
 private static void launchVar(int id,Vector3f at,Matrix3f basis,float num,float[] scale,int accum,boolean floor){
  var part=PARTS.get(id);if(part==null)return;
  ACCUM[accum]+=num+random()*rangeOf(part,"num");
  float whole=Math.max((float)Math.floor(ACCUM[accum]),0);ACCUM[accum]-=whole;
  for(int k=0;k<(int)whole;k++){
   var p=birth(id,at,basis);if(p==null)continue;
   if(scale!=null){p.scale[0]=scale[0]*(1+random());p.scale[1]=scale[1]*(1+random());}
   p.floor=floor?groundUnder(p.pos):Float.NaN;
  }
 }
 // particles.rs launch_part / launch_flat.
 private static void launch(int id,Vector3f at,Matrix3f basis,Vector3f flat){
  var part=PARTS.get(id);if(part==null)return;
  Float n=value(part,"num",Map.of());float count=n==null?1:n;
  int whole=(int)Math.floor(count)+(random()<count-(float)Math.floor(count)?1:0);
  for(int k=0;k<whole;k++){var p=birth(id,at,basis);if(p!=null&&flat!=null)p.flat=new Vector3f(flat);}
 }
 // particles.rs relaunch (:next-launcher): the launcher's fields replace the particle's.
 private static void relaunch(int id,Particle p){
  var part=PARTS.get(id);if(part==null)return;
  Map<String,Float> t=new HashMap<>();
  for(var s:part.specs){
   Float v=value(part,s.key(),t);if(v==null)continue;t.put(s.key(),v);
   switch(s.key()){
    case "scale-x"->p.scale[0]=v;case "scale-y"->p.scale[1]=v;case "scalevel-x"->p.scalevel[0]=v;case "scalevel-y"->p.scalevel[1]=v;
    case "r"->p.rgba[0]=v;case "g"->p.rgba[1]=v;case "b"->p.rgba[2]=v;case "a"->p.rgba[3]=v;
    case "fade-r"->p.fade[0]=v;case "fade-g"->p.fade[1]=v;case "fade-b"->p.fade[2]=v;case "fade-a"->p.fade[3]=v;
    case "rotvel-z"->p.rotvel=v*ANGLE;case "friction"->p.friction=v;
    case "accel-x"->p.accel.x=v;case "accel-y"->p.accel.y=v;case "accel-z"->p.accel.z=v;
    case "vel-x"->p.vel.x=v;case "vel-y"->p.vel.y=v;case "vel-z"->p.vel.z=v;
    case "timer"->p.timer=(float)Math.floor(v);case "next-time"->p.nextTime=(float)Math.floor(v);
    default->{}
   }
  }
  if(part.texture!=null)p.part=id;
  p.launcher=id;
 }
 // particles.rs step: 60 Hz frames.
 private static void step(float dt){
  if(!(dt>0))return;
  carry+=dt*60.0;
  while(carry>=1-1e-6){carry=Math.max(carry-1,0);stepFrame(1/60f);}
 }
 // particles.rs step_frame (no groups: these are launched one by one).
 private static void stepFrame(float dt){
  float frames=dt*60,ticks=dt*TICKS;
  LIVE.removeIf(p->!(p.timer>0||p.timer==-1));
  for(var p:LIVE){
   p.age+=ticks;
   p.vel.fma(frames,p.accel);
   if(p.friction!=0)p.vel.mul((float)Math.pow(p.friction,frames));
   p.pos.fma(ticks,p.vel);
   p.scale[0]+=p.scalevel[0]*ticks;p.scale[1]+=p.scalevel[1]*ticks;
   p.rot+=p.rotvel*ticks;
   for(int k=0;k<4;k++)p.rgba[k]=Math.clamp(p.rgba[k]+p.fade[k]*ticks,0,255);
   if(p.timer!=-1)p.timer=Math.max(p.timer-ticks,0);
   if(!Float.isNaN(p.nextTime)){
    p.nextTime-=ticks;
    if(p.nextTime<=0){p.nextTime=Float.NaN;var launcher=PARTS.get(p.launcher);if(launcher!=null&&launcher.next>=0)relaunch(launcher.next,p);}
   }
  }
  for(var p:LIVE)if(!Float.isNaN(p.floor)&&p.pos.y<p.floor&&p.vel.y<0){LANDED.add(new float[]{p.part,p.pos.x,p.floor,p.pos.z});p.timer=0;}
  LIVE.removeIf(p->!((p.timer>=0||p.timer==-1)&&(p.scale[0]>0||p.scalevel[0]>0)&&(p.scale[1]>0||p.scalevel[1]>0)));
 }
 // jak_weather.rs ground_under (birth-func-rain): the ground or water under a drop, up to 40 m down; none found, 50 m under it.
 private static float groundUnder(Vector3f p){
  var level=Minecraft.getInstance().level;
  Vec3 at=origin.add(p.x/BLOCK,p.y/BLOCK,p.z/BLOCK),end=at.subtract(0,40*METER/BLOCK,0);
  // stand-in: the Rust swept a 0.1 m box through its voxels; Minecraft's ray clip of block colliders and water.
  var hit=level.clip(new ClipContext(at,end,ClipContext.Block.COLLIDER,ClipContext.Fluid.WATER,CollisionContext.empty()));
  if(hit.getType()==HitResult.Type.MISS)return p.y-50*METER;
  return (float)((hit.getLocation().y-origin.y)*BLOCK);
 }
 private static float lerpScale(float a,float b,float x,float x0,float x1){float t=Math.clamp((x-x0)/(x1-x0),0,1);return a+(b-a)*t;}

 // ---- jak_weather.rs sprites_in: what falls round the camera (rain) or Jak (snow) ----
 private static void submit(LevelRenderContext context){
  var client=Minecraft.getInstance();
  long now=System.nanoTime();float dt=client.isPaused()||last==0?0:Math.clamp((now-last)/1e9f,0,.1f);last=now;
  var camera=context.levelState().cameraRenderState;
  Vector3f forward=camera.orientation.transform(new Vector3f(0,0,-1)),up=camera.orientation.transform(new Vector3f(0,1,0)),right=new Vector3f(forward).cross(up).normalize();
  List<float[]> quads=new ArrayList<>();
  // Minecraft's bolts, whichever sky is drawn (a strike shows for 0.3 s).
  STRIKES.removeIf(s->(now-s.at())/1e9f>=.3f);
  for(var s:STRIKES){
   float left=1-(now-s.at())/1e9f/.3f;
   if(left<=0)continue;
   for(int i=0;i+1<s.points().length;i++)strip(quads,s.points()[i],s.points()[i+1],camera.pos,left);
  }
  boolean on=JakSky.weatherOn()&&client.level!=null&&client.player!=null;
  if(!on){if(!LIVE.isEmpty()||rainHiss!=null||swampWind!=null){LIVE.clear();LANDED.clear();origin=null;stop();}}
  else{
   // Lightning's bolt while its flash lasts.
   float flash=JakSky.flash();
   for(int i=0;i+1<JakSky.BOLT.size();i++){
    var a=JakSky.BOLT.get(i);var b=JakSky.BOLT.get(i+1);
    strip(quads,camera.pos.add(a.x,a.y,a.z),camera.pos.add(b.x,b.y,b.z),camera.pos,flash);
   }
   weather(client,camera.pos,forward,dt);
  }
  draw(context,quads,camera.pos,forward,up,right,client);
 }
 private static void weather(Minecraft client,Vec3 cameraPos,Vector3f forward,float dt){
  load();
  if(origin==null||origin.distanceToSqr(cameraPos)>256*256){
   if(origin!=null){
    var d=origin.subtract(cameraPos).scale(BLOCK);
    for(var p:LIVE){p.pos.add((float)d.x,(float)d.y,(float)d.z);if(!Float.isNaN(p.floor))p.floor+=(float)d.y;}
   }
   origin=cameraPos;
  }
  float frames=dt*60;
  var eye=rel(cameraPos);
  var none=new Matrix3f();
  float rain=JakSky.weatherRain(),snow=JakSky.weatherSnow(),swamp=JakSky.weatherSwamp();
  // Rain (update-rain): a camera looking along the ground sees long streaks, one looking up or down short ones.
  if(rain>.01f&&frames>0){
   float pitch=Math.abs(forward.y);
   float[] scale={lerpScale(122.88f,245.76f,pitch,0,.7f),lerpScale(2048,245.76f,pitch,0,.7f)};
   // At most 0.6 of update-rain's full amount: its heaviest is more than the frame affords at 24 chunks.
   float amount=.6f*rain;
   launchVar(38,eye,none,amount*frames,scale,0,true);
   launchVar(39,eye,none,4*amount*frames,scale,1,true);
  }
  // Snow round Jak (update-snow): still it falls about him, moving it comes from ahead.
  Vec3 feetWorld=JakPoses.presented()!=null?JakPoses.presented():client.player.position();
  var velocity=new Vector3f();
  if(lastFeet!=null&&dt>0)velocity.set((float)(feetWorld.x-lastFeet.x),(float)(feetWorld.y-lastFeet.y),(float)(feetWorld.z-lastFeet.z)).mul(BLOCK/dt);
  lastFeet=feetWorld;
  if(snow>.01f&&frames>0){
   float speed=lerpScale(0,1,velocity.length(),2048,40960);
   var feet=rel(feetWorld);
   launchVar(36,feet,none,(1-speed)*snow*frames,null,2,false);
   var ahead=new Matrix3f().rotationY((float)Math.atan2(velocity.x,velocity.z)+(float)Math.PI);
   launchVar(34,feet,ahead,4*speed*snow*frames,null,3,false);
  }
  step(dt);
  // Where the rain lands: its splash, its ring on the ground, a drip now and then.
  var upright=new Matrix3f().rotationX((float)(-Math.PI/2));
  List<Vector3f> drops=new ArrayList<>();
  for(float[] l:LANDED){
   if((int)l[0]!=38)continue;
   var at=new Vector3f(l[1],l[2],l[3]);
   launch(40,at,upright,null);
   launch(42,at,none,new Vector3f(0,1,0));
   drops.add(at);
  }
  LANDED.clear();
  drips-=dt;
  if(drips<=0&&!drops.isEmpty()){
   Vector3f near=null;for(var d:drops)if(near==null||d.distanceSquared(eye)<near.distanceSquared(eye))near=d;
   var at=origin.add(near.x/BLOCK,near.y/BLOCK,near.z/BLOCK);
   client.getSoundManager().play(new SimpleSoundInstance(event("sfx/dry_drips"),SoundSource.WEATHER,1,1,SoundInstance.createUnseededRandom(),at.x,at.y,at.z));
   drips=.3f;
  }
  // The rain's hiss and the swamp's wind (the desert's storm wind is JakSounds').
  rainHiss=loop(rainHiss,"sfx/rain_hiss",rain>.02f?Math.min(rain,1):0);
  swampWind=loop(swampWind,"sfx/swamp_wind",swamp>.05f?.6f*swamp:0);
 }
 private static Vector3f rel(Vec3 p){return new Vector3f((float)((p.x-origin.x)*BLOCK),(float)((p.y-origin.y)*BLOCK),(float)((p.z-origin.z)*BLOCK));}
 private static SoundEvent event(String name){return SoundEvent.createVariableRangeEvent(JakData.id(name));}
 private static Loop loop(Loop loop,String name,float gain){
  if(gain<=0){if(loop!=null)loop.end();return null;}
  if(loop==null||loop.isStopped()){loop=new Loop(event(name),gain);Minecraft.getInstance().getSoundManager().play(loop);}
  loop.gain=gain;return loop;
 }
 private static final class Loop extends AbstractTickableSoundInstance {
  float gain;
  Loop(SoundEvent event,float gain){super(event,SoundSource.WEATHER,SoundInstance.createUnseededRandom());this.gain=gain;volume=gain;looping=true;relative=true;attenuation=SoundInstance.Attenuation.NONE;}
  @Override public void tick(){volume=gain;}
  void end(){stop();}
 }

 // ---- jak_weather.rs strike (mc_lightning): Minecraft's bolts drawn crooked, from 128 blocks up ----
 private static void bolts(Minecraft client){
  if(client.level==null){STRUCK.clear();return;}
  if(!JakSky.weatherOn())return;
  Set<Integer> seen=new HashSet<>();
  for(var entity:client.level.entitiesForRendering()){
   if(!(entity instanceof LightningBolt bolt))continue;
   seen.add(bolt.getId());
   if(STRUCK.add(bolt.getId()))strike(bolt.position());
  }
  STRUCK.retainAll(seen);
 }
 private static int strikeSeed;
 private static float strikeRandom(){strikeSeed^=strikeSeed<<13;strikeSeed^=strikeSeed>>>17;strikeSeed^=strikeSeed<<5;return (float)(Integer.toUnsignedLong(strikeSeed)/(double)0xFFFFFFFFL)-.5f;}
 private static void strike(Vec3 at){
  // stand-in: the Rust seeded from the strike's map point (36 units a block, its world's own origin); here the block position at that scale.
  float mx=(float)at.x*36,my=(float)-at.z*36,mz=(float)at.y*36;
  strikeSeed=(Float.floatToRawIntBits(mx)^Integer.rotateLeft(Float.floatToRawIntBits(my),11)^Integer.rotateLeft(Float.floatToRawIntBits(mz),22))|1;
  Vec3[] points=new Vec3[9];points[0]=at;
  float sx=0,sy=0;
  for(int k=1;k<=8;k++){
   float rx=strikeRandom(),ry=strikeRandom();sx+=rx*2;sy+=ry*2;
   // Map (x, y, z up) to Minecraft (x, z up, -y).
   points[k]=at.add(sx,16*k,-sy);
  }
  STRIKES.add(new Strike(points,System.nanoTime()));
 }
 // jak_weather.rs: a bolt's segment, pal-lightning stretched along it, added on.
 private static void strip(List<float[]> quads,Vec3 a,Vec3 b,Vec3 eye,float amount){
  Vec3 d=b.subtract(a);float length=(float)d.length();
  Vec3 mid=a.add(b).scale(.5);
  quads.add(new float[]{(float)(mid.x-eye.x),(float)(mid.y-eye.y),(float)(mid.z-eye.z),length*1.1f,1.2f,0,200,210,255,128*amount,-1,
   (float)(d.x/Math.max(length,1e-3)),(float)(d.y/Math.max(length,1e-3)),(float)(d.z/Math.max(length,1e-3)),2,Float.NaN});
 }

 // ---- render/extract.rs push_jak_sparks: each sprite a quad, PS2 colour, inv-area fade ----
 private static Identifier texture(String name){
  if(TEXTURES.containsKey(name))return TEXTURES.get(name);if(MISSING.contains(name))return null;
  var image=JakSkyRender.read(JakSkyRender.jak3(name));
  if(image==null){MISSING.add(name);JakMod.LOG.error("Original weather texture unavailable: {}",name);return null;}
  var id=JakData.id("local/weather/"+name);String label=id.toString();
  Minecraft.getInstance().getTextureManager().register(id,new DynamicTexture(()->label,image));
  TEXTURES.put(name,id);return id;
 }
 private static void draw(LevelRenderContext context,List<float[]> bolts,Vec3 cam,Vector3f forward,Vector3f up,Vector3f right,Minecraft client){
  float pixels=208f/(float)Math.max(Math.tan(Math.toRadians(client.options.fov().get())*.5),1e-3);
  Map<String,List<float[]>> batches=new LinkedHashMap<>();
  for(float[] q:bolts)batches.computeIfAbsent("pal-lightning/2",k->new ArrayList<>()).add(q);
  if(origin!=null){
   var eye=rel(cam);
   for(var p:LIVE){
    var part=PARTS.get(p.part);if(part==null||part.texture==null)continue;
    // Not those passing right by the camera (a drop there fills the screen).
    if(p.pos.distance(eye)<=1.5f*METER)continue;
    float[] q={(p.pos.x-eye.x)/BLOCK,(p.pos.y-eye.y)/BLOCK,(p.pos.z-eye.z)/BLOCK,p.scale[0]/BLOCK,p.scale[1]/BLOCK,p.rot,p.rgba[0],p.rgba[1],p.rgba[2],p.rgba[3],p.corner,
     Float.NaN,0,0,part.blend,p.flat==null?Float.NaN:p.flat.x,p.flat==null?0:p.flat.y,p.flat==null?0:p.flat.z};
    if(part.motionBlur&&p.flat==null&&p.vel.lengthSquared()>1e-6f){var v=new Vector3f(p.vel).normalize();q[11]=v.x;q[12]=v.y;q[13]=v.z;}
    batches.computeIfAbsent(part.texture+"/"+part.blend,k->new ArrayList<>()).add(q);
   }
  }
  if(batches.isEmpty())return;
  var stack=context.poseStack();var collector=context.submitNodeCollector();
  for(var batch:batches.entrySet()){
   String key=batch.getKey();int split=key.lastIndexOf('/');
   Identifier id=texture(key.substring(0,split));if(id==null)continue;int blend=Integer.parseInt(key.substring(split+1));
   List<Vertex> vertices=new ArrayList<>();
   for(float[] q:batch.getValue())corners(q,forward,up,right,pixels,vertices);
   if(vertices.isEmpty())continue;
   collector.submitCustomGeometry(stack,JakEffectTypes.get(id,blend),(pose,consumer)->{
    Vector3f v=new Vector3f();
    for(var x:vertices){pose.pose().transformPosition(x.x(),x.y(),x.z(),v);consumer.addVertex(v.x,v.y,v.z,x.colour(),x.u(),x.v(),OverlayTexture.NO_OVERLAY,0x00f000f0,0,1,0);}
   });
  }
 }
 // q: pos xyz, w, h, rotation, rgba, corner, stretch xyz (NaN none), blend, flat xyz (NaN none); camera-relative blocks.
 private record Vertex(float x,float y,float z,int colour,float u,float v){}
 private static void corners(float[] q,Vector3f forward,Vector3f up,Vector3f right,float pixels,List<Vertex> out){
  var pos=new Vector3f(q[0],q[1],q[2]);float w=q[3],h=q[4];
  if(pos.dot(forward)<-Math.max(w,h))return;
  Vector3f ax,ay;
  boolean flat=q.length>15&&!Float.isNaN(q[15]),stretch=!Float.isNaN(q[11]);
  if(flat){
   var n=new Vector3f(q[15],q[16],q[17]).normalize();
   if(new Vector3f(pos).negate().dot(n)<0)return;
   var t=orthonormal(n);var b=new Vector3f(n).cross(t);
   float s=(float)Math.sin(q[5]),c=(float)Math.cos(q[5]);
   ax=new Vector3f(t).mul(c).fma(s,b).mul(w*.5f);ay=new Vector3f(b).mul(c).fma(-s,t).mul(h*.5f);
  }else if(stretch){
   var dir=new Vector3f(q[11],q[12],q[13]);
   var side=new Vector3f(dir).cross(new Vector3f(pos).negate());
   if(side.lengthSquared()==0)return;side.normalize();
   ax=new Vector3f(dir).mul(w*.5f);ay=side.mul(h*.5f);
  }else{
   float s=(float)Math.sin(q[5]),c=(float)Math.cos(q[5]);
   ax=new Vector3f(right).mul(c).fma(s,up).mul(w*.5f);ay=new Vector3f(up).mul(c).fma(-s,right).mul(h*.5f);
  }
  int corner=q[10]<0?0:(int)q[10];
  // From its point out along its turn (a ray), not centred on it.
  var p=(corner&15)==4?new Vector3f(pos).add(ax):pos;
  float depth=Math.max(p.dot(forward),1e-3f),area=Math.min(w*h*(pixels/depth)*(pixels/depth),1);
  int colour=(ch(q[9]*area)<<24)|(ch(q[6])<<16)|(ch(q[7])<<8)|ch(q[8]);
  float[][] uv={{0,1},{1,1},{1,0},{0,0}};
  for(int i=0;i<4;i++){
   if((corner&16)!=0)uv[i][0]=1-uv[i][0];
   if((corner&32)!=0)uv[i][1]=1-uv[i][1];
  }
  float[][] k={{-1,-1},{1,-1},{1,1},{-1,1}};
  for(int i=0;i<4;i++)out.add(new Vertex(p.x+ax.x*k[i][0]+ay.x*k[i][1],p.y+ax.y*k[i][0]+ay.y*k[i][1],p.z+ax.z*k[i][0]+ay.z*k[i][1],colour,uv[i][0],uv[i][1]));
 }
 private static int ch(float v){return Math.clamp(Math.round(v),0,255);}
 // glam Vec3::any_orthonormal_vector.
 private static Vector3f orthonormal(Vector3f n){
  float sign=Math.copySign(1f,n.z),a=-1/(sign+n.z),b=n.x*n.y*a;
  return new Vector3f(b,sign+n.y*n.y*a,-n.y);
 }
}
