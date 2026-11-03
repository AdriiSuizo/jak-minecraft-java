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
 private record Asset(CitizenKind kind,GltfModel model,Map<Long,Sleep> beds,GltfModel.Posed sleeping){}
 private record Sleep(GltfModel.Posed pose,Matrix4f align,Vector3f heading){}
 private record Draw(Asset asset,GltfModel.Posed pose,long segments,float yaw,float scale,Sleep sleep,String clip,float floor){}
 private static final Map<Integer,Asset> MODELS=new HashMap<>();private static final Map<VillagerRenderState,Draw> DRAW=new WeakHashMap<>();
 public static final Set<String> SEEN=new HashSet<>();public static long rendered,sleepers;public static float largestSleepingThickness;
 private static Asset asset(int index){return MODELS.computeIfAbsent(index,i->{var k=CitizenKind.of(i);try{var model=new GltfModel(k.root().resolve(k.asset()),"citizen_"+i,true);model.loadSegments(k.root().resolve("raw_obj/"+k.model()+"-ag.go"),k.model()+"-lod0");for(var clip:k.clips().keySet())if(!model.hasClip(clip))throw new IllegalStateException("Source citizen GLB clip missing: "+clip);var sleeping=model.pose(k.model()+"-knocked-back-land",model.frames(k.model()+"-knocked-back-land")-1,true);model.prepare(sleeping);return new Asset(k,model,new HashMap<>(),sleeping);}catch(IOException e){throw new UncheckedIOException(e);}});}
 public static void extract(Villager entity,VillagerRenderState state,float partial){
  if(entity.getType()!=EntityTypes.VILLAGER){DRAW.remove(state);return;}var data=(CitizenState)entity;Asset a=asset(data.citizenVariant());int seed=entity.getUUID().hashCode();long shown=a.kind.segments(seed);String suffix="idle";float time=0;Sleep bed=null;
  if(entity.isSleeping()){bed=a.beds.computeIfAbsent(shown,m->bed(a,m));suffix="knocked-back-land";}
  else if(entity.deathTime>0){suffix=entity.onGround()&&entity.deathTime>5?"knocked-back-land":"knocked-back";time=(entity.deathTime+partial)/20;}
  else if(data.citizenPhase()!=0){suffix=switch(data.citizenPhase()){case 1->"knocked-back";case 2->"knocked-back-land";default->"get-up-from-back";};time=(entity.level().getGameTime()-data.citizenPhaseStart()+partial)/20f;}
  else if(entity.isInWater()){suffix="tread-water";time=state.ageInTicks/20f%a.kind.duration(suffix);}
  else if(state.walkAnimationSpeed>.01f){suffix=a.kind.walk(seed);float stride=a.kind.stride(seed);// Native LivingEntity multiplies distance by four; babies multiply position by three.
   time=(state.walkAnimationPos/(4*(state.isBaby?3:1)*stride*JakEnemyKind.SCALE)%1)*a.kind.duration(suffix);}
  else {float rate=a.kind.model().equals("citizen-chick")?1f/(10+3*(seed&65535)/65535f):.1f;time=((state.ageInTicks/20f*rate)+(seed&255)*.01f)%a.kind.duration(suffix);}
  String clip=a.kind.model()+"-"+suffix;GltfModel.Posed posed=bed!=null?bed.pose:a.model.poseAtTime(clip,time);float scale=JakEnemyKind.SCALE*entity.getAgeScale()*entity.getScale();float floor=bed==null&&(data.citizenPhase()>=2||entity.deathTime>0)?java.lang.Math.max(0,-a.model.bounds(posed,new Matrix4f(),shown)[0].y):0;DRAW.put(state,new Draw(a,posed,shown,net.minecraft.util.Mth.rotLerp(partial,entity.yBodyRotO,entity.yBodyRot),scale,bed,clip,floor));
 }
 private static Sleep bed(Asset a,long shown){
  var p=a.sleeping;var head=a.model.joint(p,a.kind.model().equals("citizen-chick")?"neck":"head").getTranslation(new Vector3f());var ankles=a.model.joint(p,"Lankle").getTranslation(new Vector3f()).add(a.model.joint(p,"Rankle").getTranslation(new Vector3f())).mul(.5f);var axis=new Vector3f(head).sub(ankles).normalize();var flat=new Vector3f(axis.x,0,axis.z).normalize();Matrix4f rotation=new Matrix4f().rotate(new Quaternionf().rotationTo(axis,flat));var b=a.model.bounds(p,rotation,shown);var center=new Vector3f(b[0]).add(b[1]).mul(.5f);Matrix4f align=new Matrix4f().translation(-center.x,-b[0].y,-center.z).mul(rotation);largestSleepingThickness=java.lang.Math.max(largestSleepingThickness,(b[1].y-b[0].y)*JakEnemyKind.SCALE);return new Sleep(p,align,flat);
 }
 public static boolean submit(VillagerRenderState state,PoseStack stack,SubmitNodeCollector collector){Draw d=DRAW.get(state);if(d==null)return false;stack.pushPose();
  if(d.sleep!=null&&state.bedOrientation!=null){var direction=state.bedOrientation;stack.translate(-direction.getStepX()*.5,-.125,-direction.getStepZ()*.5);float yaw=(float)(java.lang.Math.atan2(direction.getStepX(),direction.getStepZ())-java.lang.Math.atan2(d.sleep.heading.x,d.sleep.heading.z));stack.rotate(Axis.YP,yaw);stack.scale(d.scale,d.scale,d.scale);stack.mulPose(d.sleep.align);sleepers++;}
  else {stack.rotateDegrees(Axis.YP,-d.yaw);stack.scale(d.scale,d.scale,d.scale);stack.translate(0,d.floor,0);}
  if(!state.isInvisible||!state.isInvisibleToPlayer)d.asset.model.submit(d.pose,stack,collector,state.lightCoords,state.isInvisible?.15f:1,OverlayTexture.pack(0,state.hasRedOverlay),d.segments);
  if(d.sleep==null&&!state.heldItem.isEmpty()){stack.pushPose();var hand=d.asset.model.joint(d.pose,"Rhand");stack.mulPose(new Matrix4f().translationRotate(hand.getTranslation(new Vector3f()),hand.getUnnormalizedRotation(new Quaternionf())));stack.scale(1/JakEnemyKind.SCALE,1/JakEnemyKind.SCALE,1/JakEnemyKind.SCALE);state.heldItem.submit(stack,collector,state.lightCoords,OverlayTexture.NO_OVERLAY,state.outlineColor);stack.popPose();}
  stack.popPose();rendered++;SEEN.add(d.clip);return true;
 }
}
