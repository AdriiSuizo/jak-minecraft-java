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
 private record Frame(Map<String,Shot> before,Map<String,Shot> now,long received,long session){}
 private static final Map<Integer,Frame> FRAMES=new HashMap<>();
 public static int drawnModels,interpolatedModels;
 public static void receive(int owner,Vec3 origin,JsonObject packet){
  Map<String,Shot> shots=new LinkedHashMap<>();
  if(packet.has("models"))for(var value:packet.getAsJsonArray("models")){
   var s=value.getAsJsonObject();var r=s.getAsJsonArray("rotation");
   shots.put(s.get("id").getAsString(),new Shot(JakPlayers.vector(s.getAsJsonArray("pos")).add(origin),JakPlayers.vector(s.getAsJsonArray("scale")),new Quaternionf(r.get(0).getAsFloat(),r.get(1).getAsFloat(),r.get(2).getAsFloat(),r.get(3).getAsFloat()),s));
  }
  long session=packet.get("session").getAsLong();Frame old=FRAMES.get(owner);FRAMES.put(owner,new Frame(old==null||old.session!=session?Map.of():old.now,shots,System.nanoTime(),session));
 }
 public static void remove(int owner){FRAMES.remove(owner);}
 public static void clear(){FRAMES.clear();}

 public static void initialize(){LevelRenderEvents.COLLECT_SUBMITS.register(context->{
  var client=Minecraft.getInstance();if(client.level==null)return;drawnModels=0;interpolatedModels=0;var camera=context.levelState().cameraRenderState;
  for(var entry:JakPoses.all().entrySet()){
   var entity=client.level.getEntity(entry.getKey());if(entity==null||!entity.getAttachedOrCreate(JakData.ENABLED))continue;
   Frame frame=FRAMES.get(entry.getKey());if(frame==null)continue;
   float elapsed=Math.clamp((System.nanoTime()-frame.received)/1e9f,0,.05f),fraction=elapsed/.05f;
   for(var pair:frame.now.entrySet()){
    Shot current=pair.getValue(),previous=frame.before.get(pair.getKey());JsonObject shot=current.raw;
    boolean interpolate=previous!=null&&previous.position.distanceToSqr(current.position)<32*32;
    Vec3 position=interpolate?previous.position.lerp(current.position,fraction):current.position;
    Vec3 scale=interpolate?previous.scale.lerp(current.scale,fraction):current.scale;
    Quaternionf rotation=interpolate?new Quaternionf(previous.rotation).slerp(current.rotation,fraction):current.rotation;
    String name=shot.get("model").getAsString();
    float ageOffset=interpolate?elapsed-.05f:elapsed;
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
    if(interpolate)interpolatedModels++;drawnModels++;model.submit(skeleton,stack,context.submitNodeCollector(),light,interpolate?org.joml.Math.lerp(previous.raw.get("alpha").getAsFloat(),shot.get("alpha").getAsFloat(),fraction):shot.get("alpha").getAsFloat());stack.popPose();
   }
  }
 });}
}
