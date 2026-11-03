package dev.jakmc.client.model;

import com.mojang.blaze3d.vertex.PoseStack;
import com.mojang.math.Axis;
import dev.jakmc.entity.*;
import java.io.*;
import java.util.*;
import net.fabricmc.fabric.api.client.rendering.v1.EntityRendererRegistry;
import net.minecraft.client.renderer.SubmitNodeCollector;
import net.minecraft.client.renderer.entity.*;
import net.minecraft.client.renderer.entity.state.EntityRenderState;
import net.minecraft.client.renderer.state.level.CameraRenderState;
import net.minecraft.client.renderer.texture.OverlayTexture;
import net.minecraft.util.Mth;

public final class JakEnemyRenderer extends EntityRenderer<JakEnemy,JakEnemyRenderer.State> {
 public static class State extends EntityRenderState {GltfModel.Posed pose;float yaw,tilt;boolean hurt,gem;}
 private static final class Blend {String clip;GltfModel.Posed last,from;float start;}
 private final Map<Integer,Blend> blends=new HashMap<>();
 private final JakEnemyKind kind;
 private GltfModel model;
 private GltfModel.Posed resting;
 public static long rendered;
 public static final Set<String> seen=new HashSet<>();
 public static final Map<Integer,Set<String>> seenClips=new HashMap<>();
 public JakEnemyRenderer(EntityRendererProvider.Context context,JakEnemyKind kind){super(context);this.kind=kind;shadowRadius=kind.half*JakEnemyKind.SCALE;}
 private void load(){if(model!=null)return;try{model=new GltfModel(kind.root().resolve(kind.asset),"enemy_"+kind.model,true);if(kind.guard())resting=model.pose("",0);for(String c:kind.clips)if(!c.isEmpty()&&!model.hasClip(c))throw new IllegalStateException("Required original GLB clip absent "+c);}catch(IOException e){throw new UncheckedIOException(e);}}
 public static void initialize(){for(var e:JakEntities.TYPES.entrySet())EntityRendererRegistry.register(e.getValue(),context->new JakEnemyRenderer(context,JakEnemyKind.ALL.get(e.getKey())));}
 @Override public State createRenderState(){return new State();}
 @Override public void extractRenderState(JakEnemy entity,State state,float partial){
  super.extractRenderState(entity,state,partial);load();
  String clip=entity.animation();var art=kind.animations.get(clip);float seconds=entity.animationSeconds(partial),duration=art==null?0:art.duration();
  if(entity.animationLoops()&&duration>0)seconds%=duration;
  float frame=art==null?0:Math.min(Math.max(0,art.frames()-1),seconds*60*art.speed());
  var posed=kind.guard()&&entity.isInWater()&&entity.isAlive()?JakCitizenRenderer.tread(model,resting,state.ageInTicks/40f):model.pose(clip,frame);var blend=blends.computeIfAbsent(entity.getId(),id->new Blend());
  if(!clip.equals(blend.clip)){blend.from=blend.last;blend.start=state.ageInTicks;blend.clip=clip;}
  if(blend.from!=null){float transition=entity.state()==JakEnemy.State.HURT?.035f:.15f;float t=(state.ageInTicks-blend.start)/(20*transition);if(t<1)posed=model.blend(blend.from,posed,t);else blend.from=null;}
  blend.last=posed;state.pose=posed;state.yaw=Mth.rotLerp(partial,entity.yBodyRotO,entity.yBodyRot);state.hurt=entity.hurtTime>0||entity.deathTime>0;
  state.gem=!entity.isDeadOrDying();
  if(dev.jakmc.client.EnemyScene.active())seenClips.computeIfAbsent(entity.getId(),id->new HashSet<>()).add(clip);
  // A static ragdoll/standing death needs a physical fall, never a standing despawn.
  state.tilt=0;
  if(entity.deathTime>0&&(duration<.6f||clip.contains("in-place")||clip.contains("standing")||clip.contains("ragdoll")))state.tilt=Math.min(1,(float)Math.sqrt(Math.max(0,(entity.deathTime+partial-1)/20f*1.6f)))*90;
  if(blends.size()>256)blends.keySet().removeIf(id->entity.level().getEntity(id)==null);
 }
 @Override public void submit(State state,PoseStack stack,SubmitNodeCollector collector,CameraRenderState camera){
  super.submit(state,stack,collector,camera);if(state.isInvisible||state.pose==null)return;
  stack.pushPose();stack.rotateDegrees(Axis.YP,-state.yaw);stack.rotateDegrees(Axis.ZP,state.tilt);stack.scale(JakEnemyKind.SCALE,JakEnemyKind.SCALE,JakEnemyKind.SCALE);
  model.submit(state.pose,stack,collector,state.lightCoords,1,OverlayTexture.pack(0,state.hurt),state.gem);
  if(state.gem){int joint=switch(kind.model){case "grunt"->5;case "flitter"->27;case "mantis"->6;default->-1;};if(joint>=0){var offset=switch(kind.model){case "grunt"->new org.joml.Vector3f(0,614.4f,-3276.8f).div(4096);case "flitter"->new org.joml.Vector3f(0,696.32f,1843.2f).div(4096);default->new org.joml.Vector3f(0,1286.144f,122.88f).div(4096);};var at=model.jointAt(state.pose,joint).transformPosition(offset);stack.pushPose();stack.translate(at.x,at.y,at.z);SkullGemRenderer.carrier(stack,collector);stack.popPose();}}
  stack.popPose();rendered++;seen.add(kind.model);
 }
}
