package dev.jakmc.client.model;

import com.mojang.blaze3d.vertex.PoseStack;
import dev.jakmc.RuntimeConfig;
import dev.jakmc.content.JakEcoLauncher;
import java.io.IOException;
import net.minecraft.client.renderer.SubmitNodeCollector;
import net.minecraft.client.renderer.blockentity.BlockEntityRenderer;
import net.minecraft.client.renderer.blockentity.state.BlockEntityRenderState;
import net.minecraft.client.renderer.feature.ModelFeatureRenderer;
import net.minecraft.client.renderer.state.level.CameraRenderState;
import net.minecraft.world.phys.Vec3;

/** The original `citb-launcher` mesh on its block (its idle loop), at the scale of the original creatures (JakEnemyKind.SCALE). */
public final class LauncherRenderer implements BlockEntityRenderer<JakEcoLauncher.Pad,LauncherRenderer.State> {
    private static final String CLIP="citb-launcher-idle";
    private static GltfModel model;private static boolean missing;
    /** How far its mesh reaches below its origin (its top): lifted by that it stands on the block below (Adrii 06/11). */
    private static float lift;
    public static final class State extends BlockEntityRenderState {GltfModel.Posed pose;}
    @Override public State createRenderState(){return new State();}
    @Override public int getViewDistance(){return 64;}
    @Override public void extractRenderState(JakEcoLauncher.Pad pad,State state,float partial,Vec3 camera,ModelFeatureRenderer.CrumblingOverlay breaking){
        BlockEntityRenderer.super.extractRenderState(pad,state,partial,camera,breaking);
        if(model==null&&!missing){
            try{
                model=new GltfModel(RuntimeConfig.gameAssets("jak1").resolve("levels/citadel/citb-launcher-lod0.glb"),"citb-launcher",true);
                lift=-model.bounds(model.duration(CLIP)>0?model.poseAtTime(CLIP,0):model.pose("",0),new org.joml.Matrix4f())[0].y;
            }
            catch(IOException failure){missing=true;dev.jakmc.JakMod.LOG.warn("Blue eco launcher mesh missing",failure);}
        }
        if(model==null){state.pose=null;return;}
        float duration=model.duration(CLIP);
        // anim-loop: the idle clip round and round.
        state.pose=duration>0?model.poseAtTime(CLIP,GltfModel.materialSeconds()%duration):model.pose("",0);
    }
    @Override public void submit(State state,PoseStack stack,SubmitNodeCollector collector,CameraRenderState camera){
        if(state.pose==null)return;
        stack.pushPose();
        float m=dev.jakmc.entity.JakEnemyKind.SCALE;
        stack.translate(.5,0,.5);stack.scale(m,m,m);stack.translate(0,lift,0);
        model.submit(state.pose,stack,collector,state.lightCoords);
        stack.popPose();
    }
}
