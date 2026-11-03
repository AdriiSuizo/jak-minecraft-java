package dev.jakmc.client.model;

import com.mojang.blaze3d.vertex.PoseStack;
import dev.jakmc.RuntimeConfig;
import dev.jakmc.content.JakEcoVent;
import java.io.IOException;
import java.io.UncheckedIOException;
import java.util.EnumMap;
import net.minecraft.client.renderer.SubmitNodeCollector;
import net.minecraft.client.renderer.blockentity.BlockEntityRenderer;
import net.minecraft.client.renderer.blockentity.state.BlockEntityRenderState;
import net.minecraft.client.renderer.feature.ModelFeatureRenderer;
import net.minecraft.client.renderer.state.level.CameraRenderState;
import net.minecraft.world.phys.Vec3;

public final class EcoVentRenderer implements BlockEntityRenderer<JakEcoVent.Vent,EcoVentRenderer.State> {
    private static final float SCALE=.35f;
    private static final EnumMap<JakEcoVent.Color,GltfModel> MODELS=new EnumMap<>(JakEcoVent.Color.class);
    public static final class State extends BlockEntityRenderState {
        GltfModel model;
        GltfModel.Posed pose;
        float sink;
    }
    @Override public State createRenderState(){return new State();}
    @Override public int getViewDistance(){return 36;}
    @Override public void extractRenderState(JakEcoVent.Vent vent,State state,float partial,Vec3 camera,
        ModelFeatureRenderer.CrumblingOverlay breaking){
        BlockEntityRenderer.super.extractRenderState(vent,state,partial,camera,breaking);
        state.pose=null;
        if(!vent.color().model())return;
        String name=vent.color().path()+"-eco-vent";
        var model=MODELS.computeIfAbsent(vent.color(),color->{
            try{return new GltfModel(RuntimeConfig.assets.resolve("levels/templed/"+name+"-lod0.glb"),name,true);}
            catch(IOException failure){throw new UncheckedIOException(failure);}
        });
        String clip=name+"-close-open";
        float elapsed=vent.transition(partial),duration=model.duration(clip);
        float phase=vent.open()?Math.min(duration,elapsed):Math.max(0,duration-elapsed);
        state.model=model;
        state.pose=model.poseAtTime(clip,phase>=duration?Float.MAX_VALUE:phase);
        state.sink=vent.breaking()?Math.clamp(elapsed-.2f,0,1)*.34f:0;
    }
    @Override public void submit(State state,PoseStack stack,SubmitNodeCollector collector,CameraRenderState camera){
        if(state.pose==null)return;
        stack.pushPose();
        stack.translate(.5,-state.sink,.5);
        stack.scale(SCALE,SCALE,SCALE);
        state.model.submit(state.pose,stack,collector,state.lightCoords);
        stack.popPose();
    }
}
