package dev.jakmc.client.model;

import com.google.gson.JsonArray;
import dev.jakmc.RuntimeConfig;
import dev.jakmc.client.JakPoses;
import dev.jakmc.player.JakPlayers;
import java.io.IOException;
import net.fabricmc.fabric.api.client.rendering.v1.level.LevelRenderEvents;
import net.minecraft.core.BlockPos;
import net.minecraft.client.Minecraft;
import net.minecraft.world.level.LightLayer;
import net.minecraft.world.phys.Vec3;

/**
 * The water's ripple mesh (water.gc spawn-ripples): a manipy of skel-generic-ripples playing its idle clip once
 * ('play1), at the splash, its root scale (scale, 0.5, scale) with scale 0.6 to 1 from the splash's intensity. The core
 * sends each one as [dx, dy, dz, scale, seconds played] from the swimmer, in blocks (pose "ripples").
 */
public final class WaterRippleRenderer {
    private static final String FILE="collectables-generic-ripples-lod0.glb",KEY="collectables-generic-ripples",IDLE="collectables-generic-ripples-idle";
    private static GltfModel model;private static boolean missing;
    private WaterRippleRenderer(){}

    public static void initialize(){
        LevelRenderEvents.COLLECT_SUBMITS.register(context->{
            var client=Minecraft.getInstance();
            if(client.level==null)return;
            for(var pose:JakPoses.all().values()){
                var list=pose.raw().get("ripples");
                if(list==null||!list.isJsonArray()||list.getAsJsonArray().isEmpty()||!load())continue;
                var camera=context.levelState().cameraRenderState.pos;
                for(var entry:list.getAsJsonArray()){
                    JsonArray a=entry.getAsJsonArray();
                    float scale=a.get(3).getAsFloat(),age=a.get(4).getAsFloat()+Math.max(0,JakPoses.elapsed(pose));
                    if(age>=model.duration(IDLE))continue;
                    Vec3 at=pose.position().add(a.get(0).getAsDouble(),a.get(1).getAsDouble(),a.get(2).getAsDouble());
                    var stack=context.poseStack();
                    stack.pushPose();
                    stack.translate(at.x-camera.x,at.y-camera.y,at.z-camera.z);
                    // The original metre is 0.64 of a block (the particles' and the water's own scale).
                    float m=(float)JakPlayers.METRE;stack.scale(m*scale,m*.5f,m*scale);
                    var block=BlockPos.containing(at);
                    int light=net.minecraft.util.LightCoordsUtil.pack(client.level.getBrightness(LightLayer.BLOCK,block),client.level.getBrightness(LightLayer.SKY,block));
                    model.submit(model.poseAtTime(IDLE,age),stack,context.submitNodeCollector(),light);
                    stack.popPose();
                }
            }
        });
    }
    private static boolean load(){
        if(model!=null||missing)return model!=null;
        try{model=new GltfModel(RuntimeConfig.assets.resolve("levels/common").resolve(FILE),KEY,true);}
        catch(IOException failure){missing=true;dev.jakmc.JakMod.LOG.warn("Water ripple mesh missing",failure);}
        return model!=null;
    }
}
