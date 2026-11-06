package dev.jakmc.client.model;

import com.google.gson.*;
import dev.jakmc.*;
import dev.jakmc.client.JakPoses;
import dev.jakmc.player.JakPlayers;
import net.fabricmc.fabric.api.client.rendering.v1.level.LevelRenderEvents;
import net.minecraft.client.Minecraft;
import net.minecraft.core.BlockPos;
import org.joml.Quaternionf;
import net.minecraft.world.phys.Vec3;
import java.util.*;

/** Original shot meshes and clips, including saucer, grenade and Nova mushroom. */
public final class JakShotRenderer {
 private static final Map<String,GltfModel> MODELS=new HashMap<>();
 private record Shot(Vec3 position,Vec3 scale,Quaternionf rotation,JsonObject raw){}
 private static final Map<Integer,dev.jakmc.client.SnapshotHistory<Map<String,Shot>>> FRAMES=new HashMap<>();
 public static int drawnModels,interpolatedModels;
 public static void receive(int owner,Vec3 origin,JsonObject packet){
  Map<String,Shot> shots=new LinkedHashMap<>();
  if(packet.has("models"))for(var value:packet.getAsJsonArray("models")){
   var s=value.getAsJsonObject();var r=s.getAsJsonArray("rotation");
   shots.put(s.get("id").getAsString(),new Shot(JakPlayers.vector(s.getAsJsonArray("pos")).add(origin),JakPlayers.vector(s.getAsJsonArray("scale")),new Quaternionf(r.get(0).getAsFloat(),r.get(1).getAsFloat(),r.get(2).getAsFloat(),r.get(3).getAsFloat()),s));
  }
  FRAMES.computeIfAbsent(owner,key->new dev.jakmc.client.SnapshotHistory<>()).accept(packet.get("session").getAsLong(),packet.get("serverTick").getAsLong(),shots);
 }
 public static void remove(int owner){FRAMES.remove(owner);}
 public static void clear(){FRAMES.clear();}

 public static void initialize(){LevelRenderEvents.COLLECT_SUBMITS.register(context->{
  var client=Minecraft.getInstance();if(client.level==null)return;drawnModels=0;interpolatedModels=0;var camera=context.levelState().cameraRenderState;
  for(var entry:JakPoses.all().entrySet()){
   var entity=client.level.getEntity(entry.getKey());if(entity==null||!entity.getAttachedOrCreate(JakData.ENABLED))continue;
   var history=FRAMES.get(entry.getKey());if(history==null)continue;
   var frame=history.sample(JakPoses.renderedTick(entry.getKey()));if(frame==null)continue;
   float fraction=frame.fraction();
   // Membership belongs to the displayed tick too: do not show a shot before
   // its firing pose, or erase it before the displayed impact/death tick.
   for(var pair:frame.before().value().entrySet()){
    Shot current=pair.getValue(),next=frame.after().value().get(pair.getKey());JsonObject shot=current.raw;
    boolean interpolate=next!=null&&next.position.distanceToSqr(current.position)<32*32;
    Vec3 position=interpolate?current.position.lerp(next.position,fraction):current.position;
    Vec3 scale=interpolate?current.scale.lerp(next.scale,fraction):current.scale;
    Quaternionf rotation=interpolate?new Quaternionf(current.rotation).slerp(next.rotation,fraction):current.rotation;
    String name=shot.get("model").getAsString();
    float ageOffset=frame.elapsed();
    var model=MODELS.computeIfAbsent(name,k->{
     if(!Set.of("gun-saucer","gun-grenade","gun-nuke","gun-red-cone","gun-red-sphere","gun-nuke-sphere").contains(k))throw new IllegalArgumentException("Unexpected original shot model "+k);
     try{return new GltfModel(RuntimeConfig.assets.resolve("levels/common/"+k+"-lod0.glb"),"shot_"+k);}catch(java.io.IOException e){throw new java.io.UncheckedIOException(e);}
    });
    var anim=shot.getAsJsonObject("anim");GltfModel.Posed skeleton;
    switch(anim.get("mode").getAsString()){
     case "once"->{String clip=anim.get("clip").getAsString();float age=anim.get("age").getAsFloat()+ageOffset;if(age>=model.duration(clip))continue;skeleton=model.poseAtTime(clip,age);}
     case "burst"->{String burst=anim.get("burst").getAsString(),fade=anim.get("fade").getAsString();float age=anim.get("age").getAsFloat()+ageOffset,length=model.duration(burst);if(age<length)skeleton=model.poseAtTime(burst,age);else{age=(age-length)*5;if(age>=model.duration(fade))continue;skeleton=model.poseAtTime(fade,age);}}
     case "at"->{String clip=anim.get("clip").getAsString();skeleton=model.poseAtTime(clip,anim.get("fraction").getAsFloat()*model.duration(clip));}
     default->skeleton=model.pose("",0);
    }
    var stack=context.poseStack();stack.pushPose();stack.translate(position.x-camera.pos.x,position.y-camera.pos.y,position.z-camera.pos.z);
    stack.mulPose(new org.joml.Matrix4f().rotation(rotation));stack.scale((float)scale.x,(float)scale.y,(float)scale.z);
    var at=BlockPos.containing(position);int light=net.minecraft.util.LightCoordsUtil.pack(client.level.getBrightness(net.minecraft.world.level.LightLayer.BLOCK,at),client.level.getBrightness(net.minecraft.world.level.LightLayer.SKY,at));
    if(interpolate)interpolatedModels++;drawnModels++;model.submit(skeleton,stack,context.submitNodeCollector(),light,interpolate?org.joml.Math.lerp(shot.get("alpha").getAsFloat(),next.raw.get("alpha").getAsFloat(),fraction):shot.get("alpha").getAsFloat());stack.popPose();
    // The cap on the mushroom's top: the sphere is 1.7 times its curves, and its alpha the square of the strip's fade.
    if(name.equals("gun-nuke-sphere")){float shown=interpolate?org.joml.Math.lerp(shot.get("alpha").getAsFloat(),next.raw.get("alpha").getAsFloat(),fraction):shot.get("alpha").getAsFloat();
     NukeCap.submit(stack,context.submitNodeCollector(),camera.pos,position,(float)scale.x/1.7f,(float)scale.y/1.7f,(float)Math.sqrt(Math.max(0,shown)));}
   }
  }
 });}
}
