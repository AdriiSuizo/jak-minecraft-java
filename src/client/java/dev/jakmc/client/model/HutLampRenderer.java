package dev.jakmc.client.model;

import com.mojang.blaze3d.vertex.PoseStack;
import dev.jakmc.RuntimeConfig;
import dev.jakmc.content.JakHutLamp;
import java.io.IOException;
import net.minecraft.client.renderer.SubmitNodeCollector;
import net.minecraft.client.renderer.blockentity.BlockEntityRenderer;
import net.minecraft.client.renderer.blockentity.state.BlockEntityRenderState;
import net.minecraft.client.renderer.feature.ModelFeatureRenderer;
import net.minecraft.client.renderer.state.level.CameraRenderState;
import net.minecraft.util.Mth;
import net.minecraft.world.phys.Vec3;
import org.joml.Vector3f;

/**
 * Jak 1's `hutlamp` mesh on its block (JakHutLamp), hung from the block's top. hutlamp-idle: joint 3 (the whole lamp)
 * turned about x by 1820.4445 (10 degrees) * sin(2 pi clock); the clock's period 900 (3 s), its start random (rand-vu).
 */
public final class HutLampRenderer implements BlockEntityRenderer<JakHutLamp.Lamp,HutLampRenderer.State> {
    private static final float SWING=10,PERIOD=3;
    /** The lamp's height in blocks: the mesh's own at the creatures' scale (JakPlayers.METRE). */
    private static final float HEIGHT=1.25f;
    private static GltfModel model;private static GltfModel.Posed pose;private static boolean missing;
    private static Vector3f low,high;
    public static final class State extends BlockEntityRenderState {boolean ready;float swing,yaw;}
    @Override public State createRenderState(){return new State();}
    @Override public int getViewDistance(){return 96;}
    @Override public void extractRenderState(JakHutLamp.Lamp lamp,State state,float partial,Vec3 camera,ModelFeatureRenderer.CrumblingOverlay breaking){
        BlockEntityRenderer.super.extractRenderState(lamp,state,partial,camera,breaking);
        if(model==null&&!missing){
            try{
                model=new GltfModel(RuntimeConfig.gameAssets("jak1").resolve("levels/village1/hutlamp-lod0.glb"),"hutlamp",true);
                pose=model.pose("",0);model.prepare(pose);
                var bounds=model.bounds(pose,new org.joml.Matrix4f());low=bounds[0];high=bounds[1];
            }
            catch(IOException failure){missing=true;dev.jakmc.JakMod.LOG.warn("Hut lamp mesh missing",failure);}
        }
        state.ready=model!=null;
        long seed=Mth.getSeed(lamp.getBlockPos());
        double clock=System.nanoTime()/1e9/PERIOD+(seed&1023)/1024.0;
        state.swing=SWING*(float)Math.sin(2*Math.PI*clock);
        state.yaw=(seed>>>10&3)*90;
    }
    @Override public void submit(State state,PoseStack stack,SubmitNodeCollector collector,CameraRenderState camera){
        if(!state.ready)return;
        stack.pushPose();
        float fit=HEIGHT/(high.y-low.y);
        stack.translate(.5,1,.5);
        stack.mulPose(new org.joml.Matrix4f().rotationY((float)Math.toRadians(state.yaw)).rotateX((float)Math.toRadians(state.swing)));
        stack.scale(fit,fit,fit);stack.translate(-(low.x+high.x)/2,-high.y,-(low.z+high.z)/2);
        model.submit(pose,stack,collector,0xf000f0);
        stack.popPose();
    }
}
