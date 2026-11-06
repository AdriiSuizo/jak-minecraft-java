package dev.jakmc.client.model;

import dev.jakmc.RuntimeConfig;
import dev.jakmc.client.JakPoses;
import java.io.IOException;
import net.fabricmc.fabric.api.client.rendering.v1.level.LevelRenderEvents;
import net.minecraft.world.phys.Vec3;
import org.joml.Vector3f;

/**
 * The Dark Bomb's dome (target-darkjak-bomb0): skel-bomb-blast and skel-generic-blast, each its idle clip
 * played once ('play1) where the bomb went off, at scale 1. Their trans-hook keeps the colour off (color-mult
 * black) and the glow white to its frame 10, white to purple (0.5, 0, 1) to 20, purple to nothing by 30.
 */
public final class BombBlastRenderer {
    private static final String[][] PARTS={{"collectables-bomb-blast-lod0.glb","collectables-bomb-blast","collectables-bomb-blast-idle"},
        {"collectables-generic-blast-lod0.glb","collectables-generic-blast","collectables-generic-blast-idle"}};
    private static GltfModel[] models;private static boolean missing;
    private BombBlastRenderer(){}

    public static void initialize(){
        LevelRenderEvents.COLLECT_SUBMITS.register(context->{
            for(var pose:JakPoses.all().values()){
                var blast=pose.raw().get("bombBlast");
                if(blast==null||!blast.isJsonArray())continue;
                var a=blast.getAsJsonArray();
                float age=a.get(3).getAsFloat(),frame=age*60;
                if(frame>30||!load())continue;
                var glow=frame<=10?new Vector3f(1):frame<=20?new Vector3f(1).lerp(new Vector3f(.5f,0,1),(frame-10)/10):new Vector3f(.5f,0,1).mul(1-(frame-20)/10);
                int rgb=Math.round(glow.x*255)<<16|Math.round(glow.y*255)<<8|Math.round(glow.z*255);
                Vec3 at=pose.position().add(a.get(0).getAsDouble(),a.get(1).getAsDouble(),a.get(2).getAsDouble());
                var camera=context.levelState().cameraRenderState.pos;
                var stack=context.poseStack();
                stack.pushPose();
                stack.translate(at.x-camera.x,at.y-camera.y,at.z-camera.z);
                float m=dev.jakmc.entity.JakEnemyKind.SCALE;stack.scale(m,m,m);
                for(int i=0;i<models.length;i++){
                    var model=models[i];
                    var posed=model.poseAtTime(PARTS[i][2],Math.min(age,model.duration(PARTS[i][2])));
                    GltfModel.colourNext(rgb);GltfModel.additiveNext();
                    model.submit(posed,stack,context.submitNodeCollector(),0xF000F0);
                }
                stack.popPose();
            }
        });
    }
    private static boolean load(){
        if(models!=null||missing)return models!=null;
        try{
            var root=RuntimeConfig.assets.resolve("levels/common");
            var loaded=new GltfModel[PARTS.length];
            for(int i=0;i<PARTS.length;i++)loaded[i]=new GltfModel(root.resolve(PARTS[i][0]),PARTS[i][1],true);
            models=loaded;
        }catch(IOException failure){missing=true;dev.jakmc.JakMod.LOG.warn("Dark Bomb blast meshes missing",failure);}
        return models!=null;
    }
}
