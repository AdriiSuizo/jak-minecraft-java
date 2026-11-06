package dev.jakmc.client.model;
import com.mojang.blaze3d.vertex.PoseStack;
import com.mojang.math.Axis;
import dev.jakmc.entity.*;
import net.minecraft.world.entity.npc.villager.Villager;
import net.minecraft.world.entity.EntityTypes;
import net.minecraft.world.entity.Pose;
import net.minecraft.client.renderer.SubmitNodeCollector;
import net.minecraft.client.renderer.entity.state.VillagerRenderState;
import net.minecraft.client.renderer.texture.OverlayTexture;
import org.joml.*;
import java.io.*;
import java.util.*;
/** Guest animation and outfits; the canonical Minecraft villager remains the actual inhabitant. */
public final class JakCitizenRenderer {
 private record Asset(CitizenKind kind,GltfModel model,Map<Long,Sleep> beds,GltfModel.Posed resting,GltfModel.Posed sleeping){}
 private record Sleep(GltfModel.Posed pose,Matrix4f align,Vector3f heading){}
 private record Draw(Asset asset,GltfModel.Posed pose,long segments,float yaw,float scale,Sleep sleep,String clip,float floor,float wide,float tall,Quaternionf attitude){
  Draw(Asset asset,GltfModel.Posed pose,long segments,float yaw,float scale,Sleep sleep,String clip,float floor,float wide,float tall){this(asset,pose,segments,yaw,scale,sleep,clip,floor,wide,tall,null);}
 }
 private static final Map<Integer,Asset> MODELS=new HashMap<>();private static final Map<VillagerRenderState,Draw> DRAW=new WeakHashMap<>();
 // Dark Jak's lightning on the citizen (JakPoses.darkTint).
 private static final Map<VillagerRenderState,Integer> TINT=new WeakHashMap<>();
 public static final Set<String> SEEN=new HashSet<>();public static long rendered,sleepers;public static float largestSleepingThickness;
 private static Asset asset(int index){
  return MODELS.computeIfAbsent(index,i->{
   var k=CitizenKind.of(i);
   try{
    var model=new GltfModel(k.root().resolve(k.asset()),"citizen_"+i,true);
    if(!k.jak1())model.loadSegments(k.root().resolve("raw_obj/"+k.model()+"-ag.go"),k.model()+"-lod0");
    for(var clip:k.clips().keySet())if(!model.hasClip(clip))throw new IllegalStateException("Source citizen GLB clip missing: "+clip);
    var rest=model.pose("",0);
    GltfModel.Posed sleeping=null;
    if(!k.monk()&&!k.jak1())sleeping=holster(k,model,model.poseAtTime(k.model()+"-knocked-back-land",Float.MAX_VALUE));
    if(sleeping!=null)model.prepare(sleeping);
    return new Asset(k,model,new HashMap<>(),rest,sleeping);
   }catch(IOException e){throw new UncheckedIOException(e);}
  });
 }
 /** Rust zoomers.rs Driver: `vehicle-rider`'s citizen-norm-rider (Jak 3 ctywide), loaded once. */
 private static Asset rider;
 private static Asset rider(){
  if(rider==null){
   try{
    var model=new GltfModel(dev.jakmc.RuntimeConfig.assets.resolve("levels/ctywide/citizen-norm-rider-lod0.glb"),"citizen_rider",true);
    model.loadSegments(dev.jakmc.RuntimeConfig.assets.resolve("raw_obj/citizen-norm-rider-ag.go"),"citizen-norm-rider-lod0");
    for(var clip:new String[]{"citizen-norm-rider-bike-stance","citizen-norm-rider-car-stance"})if(!model.hasClip(clip))throw new IllegalStateException("Source rider GLB clip missing: "+clip);
    rider=new Asset(CitizenKind.of(0),model,new HashMap<>(),model.pose("",0),null);
   }catch(IOException e){throw new UncheckedIOException(e);}
  }
  return rider;
 }
 /** Rust zoomers.rs Driver::new: the rider's clothing bits, a bike's or a car's. */
 static long riderSegments(long seed,boolean car){
  long[] rng={seed};
  java.util.function.Function<long[],Long> pick=choices->{rng[0]=rng[0]*6364136223846793005L+1442695040888963407L;return choices[(int)Long.remainderUnsigned(rng[0]>>>32,choices.length)];};
  return 32|16|(car?0x40000:0x20000)|pick.apply(new long[]{2,4096})|pick.apply(new long[]{8,512,0x4000})
   |pick.apply(new long[]{4,256,8192,0})|pick.apply(new long[]{64,2048,0x10000})|pick.apply(new long[]{128,0,0});
 }
 static GltfModel.Posed tread(GltfModel model,GltfModel.Posed rest,float seconds){
  var source=asset(0);
  var motion=source.model.poseAtTime("citizen-norm-tread-water",seconds%source.kind.duration("tread-water"));
  return model.retargetRider(source.model,source.resting,motion,rest);
 }
 private static GltfModel.Posed holster(CitizenKind kind,GltfModel model,GltfModel.Posed pose){
  return kind.model().equals("wlander-female")?model.attachJoint(pose,"gun","chest",new Vector3f(0,-544.768f/4096,1114.112f/4096)):pose;
 }
 private static GltfModel.Posed groundPose(Asset a,Asset motion,String suffix,float time){
  var pose=motion.model.poseAtTime(motion.kind.model()+"-"+suffix,time);
  return motion==a?holster(a.kind,a.model,pose):a.model.retargetRider(motion.model,motion.resting,pose,a.resting);
 }
 public static void extract(Villager entity,VillagerRenderState state,float partial){
  TINT.put(state,dev.jakmc.client.JakPoses.darkTint(entity.getId()));
  if(entity.getType()!=EntityTypes.VILLAGER){DRAW.remove(state);return;}
  var data=(CitizenState)entity;
  Asset a=asset(data.citizenVariant()),motion=a;
  int seed=entity.getUUID().hashCode();
  long shown=a.kind.segments(seed);
  String suffix=a.kind.wastelander()?"idle0":"idle";
  float time=0;
  float yaw=net.minecraft.util.Mth.rotLerp(partial,entity.yBodyRotO,entity.yBodyRot);
  Sleep bed=null;
  boolean mounted=a.kind.monk()&&entity.getVehicle() instanceof JakLeaper;
  int life=((dev.jakmc.citizens.CitizenLifeState)entity).citizenLifeState();
  boolean haven=!a.kind.monk()&&!a.kind.wastelander();
  float[] own=dev.jakmc.citizens.CitizenLife.ownScale(a.kind,seed);
  if(a.kind.jak1()){still(entity,state,partial,a,seed,life);return;}
  if(entity.getVehicle() instanceof JakVehicle car&&car.model.hover()){
   // Rust zoomers.rs draw: the driver in its bike or car stance, leaning with the steering
   // (frame 5 x (1 - steer) plus a slow sway, `rider-stance`), at the craft's seat and heading.
   var r=rider();boolean four=!car.model.bike();
   // On the craft's own presented frames (its native steps, smoothed), not the villager's 20 Hz ride:
   // the same seat, attitude and steering the zoomer is drawn with this frame.
   car.samplePresentation(System.nanoTime());
   var seat=car.presentationRider();state.x=seat.x;state.y=seat.y;state.z=seat.z;
   var before=car.presentationBefore();var after=car.presentationAfter();float blend=car.presentationFraction();
   String clip=four?"citizen-norm-rider-car-stance":"citizen-norm-rider-bike-stance";
   long unsigned=Integer.toUnsignedLong(seed);
   float phase=(unsigned%1000)*.00628f,tempo=.8f+((unsigned>>8)%1000)*.0004f;
   float steer=after==null||!after.has("steer")?0:before==null||!before.has("steer")?after.get("steer").getAsFloat()
    :net.minecraft.util.Mth.lerp(blend,before.get("steer").getAsFloat(),after.get("steer").getAsFloat());
   float clock=(car.tickCount+partial)/20f;
   float at=5*(1-steer)+(float)java.lang.Math.sin(clock*4.014f*tempo+phase);
   float stance=java.lang.Math.clamp(at/60,0,r.model.duration(clip));
   DRAW.put(state,new Draw(r,r.model.poseAtTime(clip,stance),riderSegments(unsigned,four),0,JakEnemyKind.SCALE*entity.getScale(),null,clip,0,1,1,car.presentationRotation()));
   return;
  }
  if(mounted){
   var mount=(JakLeaper)entity.getVehicle();
   suffix=mount.walkAnimation.speed()>.07f?"walk":"idle";
   time=JakLeaperRenderer.saddleCycle(suffix,mount.tickCount+partial)*a.kind.duration(suffix);
   var at=mount.getPosition(partial);
   state.x=at.x;state.y=at.y;state.z=at.z;
   yaw=net.minecraft.util.Mth.rotLerp(partial,mount.yBodyRotO,mount.yBodyRot);
  }else if(entity.isSleeping()){
   bed=a.beds.computeIfAbsent(shown,m->bed(a,m));suffix="knocked-back-land";
  }else if(entity.deathTime>0){
   if(a.kind.monk())motion=asset(0);
   suffix=entity.onGround()&&entity.deathTime>5?"knocked-back-land":"knocked-back";
   time=(entity.deathTime+partial)/20;
  }else if(data.citizenPhase()!=0){
   if(a.kind.monk())motion=asset(0);
   suffix=switch(data.citizenPhase()){case 1->"knocked-back";case 2->"knocked-back-land";default->motion.kind.rise();};
   time=(dev.jakmc.player.JakEntityClock.time(entity.level())-data.citizenPhaseStart()+partial)/20f;
  }else if(!data.citizenDefenseClip().isEmpty()){
   if(a.kind.monk())motion=asset(CitizenKind.SPARGUS_FEMALE);
   suffix=data.citizenDefenseClip();
   float rate=suffix.equals("gun-takeout")?1.6f:1f;
   time=(dev.jakmc.player.JakEntityClock.time(entity.level())-data.citizenDefenseStart()+partial)/20f*rate;
  }else if(entity.isInWater()){
   if(a.kind.monk()||a.kind.wastelander())motion=asset(0);
   suffix="tread-water";time=state.ageInTicks/20f%motion.kind.duration(suffix);
  }else if(state.walkAnimationSpeed>.01f){
   if(a.kind.monk())motion=asset(0);
   // Rust: jak2_villagers.rs clip_of/gait (State::Panic runs its panic clip at its run stride)
   boolean panic=life==dev.jakmc.citizens.CitizenLife.PANIC;
   boolean running=(a.kind.monk()||a.kind.wastelander())&&(panic||state.walkAnimationSpeed>.27f);
   suffix=panic&&haven?dev.jakmc.citizens.CitizenLife.panicClip(seed):running?(a.kind.monk()?"run":"run0"):a.kind.monk()?"walk2":a.kind.walk(seed);
   float stride=panic&&haven?dev.jakmc.citizens.CitizenLife.runStride(a.kind):running?6.4f:a.kind.monk()?3.2f:a.kind.stride(seed);
   time=(state.walkAnimationPos/(4*(state.isBaby?3:1)*stride*JakEnemyKind.SCALE)%1)*motion.kind.duration(suffix);
  }else if(life==dev.jakmc.citizens.CitizenLife.POSE&&haven){
   // Rust: jak2_villagers.rs clip_of (State::Pose) and citizen_loop_time
   suffix=dev.jakmc.citizens.CitizenLife.poseClip(a.kind,seed);
   float elapsed=(dev.jakmc.player.JakEntityClock.time(entity.level())-((dev.jakmc.citizens.CitizenLifeState)entity).citizenLifeStart()+partial)/20f;
   time=dev.jakmc.citizens.CitizenLife.loopTime(a.kind.duration(suffix),elapsed*dev.jakmc.citizens.CitizenLife.loopRate(a.kind,suffix,seed),seed);
  }else{
   if(a.kind.monk())suffix="standing-idle-"+(1+(seed&1));
   float rate=a.kind.model().equals("citizen-chick")?1f/(10+3*(seed&65535)/65535f):a.kind.monk()||a.kind.wastelander()?1f:.1f;
   time=((state.ageInTicks/20f*rate)+(seed&255)*.01f)%java.lang.Math.max(.01f,a.kind.duration(suffix));
  }
  String clip=motion.kind.model()+"-"+suffix;
  var posed=bed!=null?bed.pose:data.citizenWeaponOut()&&motion==a?a.model.poseAtTime(clip,time):groundPose(a,motion,suffix,time);
  float scale=JakEnemyKind.SCALE*entity.getAgeScale()*entity.getScale();
  float floor=bed==null&&(data.citizenPhase()>=2||entity.deathTime>0)?java.lang.Math.max(0,-a.model.bounds(posed,new Matrix4f(),shown)[0].y):0;
  DRAW.put(state,new Draw(a,posed,shown,yaw,scale,bed,clip,floor,own[0],own[1]));
 }
 // Rust: jak2_villagers.rs clip_of + draw (Jak 1's people: their idles one after another, nine seconds each; indoors at night, not seen)
 private static void still(Villager entity,VillagerRenderState state,float partial,Asset a,int seed,int life){
  if(life==dev.jakmc.citizens.CitizenLife.INDOORS){DRAW.put(state,new Draw(a,null,-1L,0,0,null,"",0,1,1));return;}
  String[] idles=dev.jakmc.citizens.CitizenLife.STILL.get(a.kind.model());
  // stand-in: Rust counts the state's seconds from the person's arrival; the entity's age stands in.
  float t=(entity.tickCount+partial)/20f;
  long flavor=dev.jakmc.citizens.CitizenLife.stillFlavor(seed);
  String clip=idles[(int)((flavor+(long)(t/9))%idles.length)];
  float time=dev.jakmc.citizens.CitizenLife.loopTime(a.kind.clips().get(clip),idles.length==1?t:t%9,(int)flavor);
  float yaw=net.minecraft.util.Mth.rotLerp(partial,entity.yBodyRotO,entity.yBodyRot);
  DRAW.put(state,new Draw(a,a.model.poseAtTime(clip,time),-1L,yaw,JakEnemyKind.SCALE*entity.getScale(),null,clip,0,1,1));
 }
 private static Sleep bed(Asset a,long shown){
  var p=a.sleeping!=null?a.sleeping:groundPose(a,asset(0),"knocked-back-land",Float.MAX_VALUE);var head=a.model.joint(p,a.model.hasJoint("head")?"head":"neck").getTranslation(new Vector3f());var ankles=a.model.joint(p,"Lankle").getTranslation(new Vector3f()).add(a.model.joint(p,"Rankle").getTranslation(new Vector3f())).mul(.5f);var axis=new Vector3f(head).sub(ankles).normalize();var flat=new Vector3f(axis.x,0,axis.z).normalize();Matrix4f rotation=new Matrix4f().rotate(new Quaternionf().rotationTo(axis,flat));var b=a.model.bounds(p,rotation,shown);var center=new Vector3f(b[0]).add(b[1]).mul(.5f);Matrix4f align=new Matrix4f().translation(-center.x,-b[0].y,-center.z).mul(rotation);largestSleepingThickness=java.lang.Math.max(largestSleepingThickness,(b[1].y-b[0].y)*JakEnemyKind.SCALE);return new Sleep(p,align,flat);
 }
 public static boolean submit(VillagerRenderState state,PoseStack stack,SubmitNodeCollector collector){Draw d=DRAW.get(state);if(d==null)return false;if(d.pose==null)return true;stack.pushPose();
  if(d.sleep!=null&&state.bedOrientation!=null){var direction=state.bedOrientation;stack.translate(-direction.getStepX()*.5,-.125,-direction.getStepZ()*.5);float yaw=(float)(java.lang.Math.atan2(direction.getStepX(),direction.getStepZ())-java.lang.Math.atan2(d.sleep.heading.x,d.sleep.heading.z));stack.rotate(Axis.YP,yaw);stack.scale(d.scale*d.wide,d.scale*d.tall,d.scale*d.wide);stack.mulPose(d.sleep.align);sleepers++;}
  else {if(d.attitude!=null)stack.mulPose(new Matrix4f().rotation(d.attitude));else stack.rotateDegrees(Axis.YP,-d.yaw);stack.scale(d.scale*d.wide,d.scale*d.tall,d.scale*d.wide);stack.translate(0,d.floor,0);}
  int tint=TINT.getOrDefault(state,0xffffff);if(tint!=0xffffff&&(!state.isInvisible||!state.isInvisibleToPlayer))GltfModel.colourNext(tint);
  if(!state.isInvisible||!state.isInvisibleToPlayer)d.asset.model.submit(d.pose,stack,collector,state.lightCoords,state.isInvisible?.15f:1,OverlayTexture.pack(0,state.hasRedOverlay),d.segments);
  if(d.sleep==null&&!state.heldItem.isEmpty()){stack.pushPose();var hand=d.asset.model.joint(d.pose,"Rhand");stack.mulPose(new Matrix4f().translationRotate(hand.getTranslation(new Vector3f()),hand.getUnnormalizedRotation(new Quaternionf())));stack.scale(1/JakEnemyKind.SCALE,1/JakEnemyKind.SCALE,1/JakEnemyKind.SCALE);state.heldItem.submit(stack,collector,state.lightCoords,OverlayTexture.NO_OVERLAY,state.outlineColor);stack.popPose();}
  stack.popPose();rendered++;SEEN.add(d.clip);return true;
 }
}
