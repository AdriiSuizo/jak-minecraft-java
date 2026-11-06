package dev.jakmc.client.model;

import com.mojang.blaze3d.vertex.PoseStack;
import dev.jakmc.RuntimeConfig;
import dev.jakmc.content.JakAmmoCrate;
import java.io.IOException;
import net.minecraft.client.renderer.SubmitNodeCollector;
import net.minecraft.client.renderer.blockentity.BlockEntityRenderer;
import net.minecraft.client.renderer.blockentity.state.BlockEntityRenderState;
import net.minecraft.client.renderer.feature.ModelFeatureRenderer;
import net.minecraft.client.renderer.state.level.CameraRenderState;
import net.minecraft.world.phys.Vec3;
import org.joml.Vector3f;

/**
 * Jak 2's `crate-krimson` mesh on its block (JakAmmoCrate): standing on the block's floor, fitted to the block
 * (stand-in, see JakAmmoCrate). It has no animation (`crate-idle-ja` is its bind pose).
 */
public final class CrateRenderer implements BlockEntityRenderer<JakAmmoCrate.Crate,CrateRenderer.State> {
    private static GltfModel model;private static GltfModel.Posed pose;private static boolean missing;
    private static Vector3f low,high;
    public static final class State extends BlockEntityRenderState {boolean ready;}
    @Override public State createRenderState(){return new State();}
    @Override public int getViewDistance(){return 64;}
    @Override public void extractRenderState(JakAmmoCrate.Crate crate,State state,float partial,Vec3 camera,ModelFeatureRenderer.CrumblingOverlay breaking){
        BlockEntityRenderer.super.extractRenderState(crate,state,partial,camera,breaking);
        if(model==null&&!missing){
            try{
                model=new GltfModel(RuntimeConfig.gameAssets("jak2").resolve("levels/common/crate-krimson-lod0.glb"),"crate-krimson",true);
                pose=model.pose("",0);model.prepare(pose);
                var bounds=model.bounds(pose,new org.joml.Matrix4f());low=bounds[0];high=bounds[1];
            }
            catch(IOException failure){missing=true;dev.jakmc.JakMod.LOG.warn("Crate mesh missing",failure);}
        }
        state.ready=model!=null;
    }
    @Override public void submit(State state,PoseStack stack,SubmitNodeCollector collector,CameraRenderState camera){
        if(!state.ready)return;
        stack.pushPose();
        // The block's 15/16 across, the mesh's own proportions; its bottom on the floor.
        float fit=15/16f/Math.max(high.x-low.x,high.z-low.z);
        stack.translate(.5,0,.5);stack.scale(fit,fit,fit);stack.translate(-(low.x+high.x)/2,-low.y,-(low.z+high.z)/2);
        model.submit(pose,stack,collector,state.lightCoords);
        stack.popPose();
    }
}
