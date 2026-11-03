package dev.jakmc.client.model;

import com.google.gson.*;
import dev.jakmc.*;
import dev.jakmc.client.JakPoses;
import dev.jakmc.player.JakPlayers;
import net.fabricmc.fabric.api.client.rendering.v1.level.LevelRenderEvents;
import net.minecraft.client.Minecraft;
import net.minecraft.core.BlockPos;
import org.joml.Quaternionf;
import java.util.*;

/** Original shot meshes and clips, including saucer, grenade and Nova mushroom. */
public final class JakShotRenderer {
 private static final Map<String,GltfModel> MODELS=new HashMap<>();
 public static void initialize(){LevelRenderEvents.COLLECT_SUBMITS.register(context->{
  var client=Minecraft.getInstance();if(client.level==null)return;var camera=context.levelState().cameraRenderState;
  for(var entry:JakPoses.all().entrySet()){
   var entity=client.level.getEntity(entry.getKey());if(entity==null||!entity.getAttachedOrCreate(JakData.ENABLED))continue;
   var pose=entry.getValue();if(!pose.raw().has("models"))continue;
   var origin=pose.position().subtract(JakPlayers.vector(pose.raw().getAsJsonArray("pos")));
   float elapsed=Math.clamp((System.nanoTime()-pose.received())/1e9f,0,.05f);
   for(var value:pose.raw().getAsJsonArray("models")){
    var shot=value.getAsJsonObject();String name=shot.get("model").getAsString();
    var model=MODELS.computeIfAbsent(name,k->{
     if(!Set.of("gun-saucer","gun-grenade","gun-nuke","gun-red-cone","gun-red-sphere","gun-nuke-sphere").contains(k))throw new IllegalArgumentException("Unexpected original shot model "+k);
     try{return new GltfModel(RuntimeConfig.assets.resolve("levels/common/"+k+"-lod0.glb"),"shot_"+k);}catch(java.io.IOException e){throw new java.io.UncheckedIOException(e);}
    });
    var anim=shot.getAsJsonObject("anim");GltfModel.Posed skeleton;
    switch(anim.get("mode").getAsString()){
     case "once"->{String clip=anim.get("clip").getAsString();float age=anim.get("age").getAsFloat()+elapsed;if(age>=model.duration(clip))continue;skeleton=model.poseAtTime(clip,age);}
     case "burst"->{String burst=anim.get("burst").getAsString(),fade=anim.get("fade").getAsString();float age=anim.get("age").getAsFloat()+elapsed,length=model.duration(burst);if(age<length)skeleton=model.poseAtTime(burst,age);else{age=(age-length)*5;if(age>=model.duration(fade))continue;skeleton=model.poseAtTime(fade,age);}}
     case "at"->{String clip=anim.get("clip").getAsString();skeleton=model.poseAtTime(clip,anim.get("fraction").getAsFloat()*model.duration(clip));}
     default->skeleton=model.pose("",0);
    }
    var position=JakPlayers.vector(shot.getAsJsonArray("pos")).add(origin);var scale=JakPlayers.vector(shot.getAsJsonArray("scale"));JsonArray r=shot.getAsJsonArray("rotation");
    var stack=context.poseStack();stack.pushPose();stack.translate(position.x-camera.pos.x,position.y-camera.pos.y,position.z-camera.pos.z);
    stack.mulPose(new org.joml.Matrix4f().rotation(new Quaternionf(r.get(0).getAsFloat(),r.get(1).getAsFloat(),r.get(2).getAsFloat(),r.get(3).getAsFloat())));stack.scale((float)scale.x,(float)scale.y,(float)scale.z);
    var at=BlockPos.containing(position);int light=net.minecraft.util.LightCoordsUtil.pack(client.level.getBrightness(net.minecraft.world.level.LightLayer.BLOCK,at),client.level.getBrightness(net.minecraft.world.level.LightLayer.SKY,at));
    model.submit(skeleton,stack,context.submitNodeCollector(),light,shot.get("alpha").getAsFloat());stack.popPose();
   }
  }
 });}
}
