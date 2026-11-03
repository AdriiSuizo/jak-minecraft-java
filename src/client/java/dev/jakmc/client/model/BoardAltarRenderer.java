package dev.jakmc.client.model;

import com.mojang.blaze3d.vertex.PoseStack;
import dev.jakmc.content.JakBoardAltar;
import net.minecraft.client.renderer.SubmitNodeCollector;
import net.minecraft.client.renderer.blockentity.BlockEntityRenderer;
import net.minecraft.client.renderer.blockentity.state.BlockEntityRenderState;
import net.minecraft.client.renderer.feature.ModelFeatureRenderer;
import net.minecraft.client.renderer.state.level.CameraRenderState;
import net.minecraft.world.phys.Vec3;

public final class BoardAltarRenderer implements BlockEntityRenderer<JakBoardAltar.Display,BoardAltarRenderer.State> {
    public static final class State extends BlockEntityRenderState {
        GltfModel.Posed board;
    }
    public static int openingFrames,closingFrames;

    @Override public State createRenderState(){return new State();}
    @Override public int getViewDistance(){return 36;}

    @Override public void extractRenderState(JakBoardAltar.Display altar,State state,float partial,Vec3 camera,
        ModelFeatureRenderer.CrumblingOverlay breaking){
        BlockEntityRenderer.super.extractRenderState(altar,state,partial,camera,breaking);
        var model=JakRenderer.boardModel();
        boolean occupied=altar.occupied();
        String clip=occupied?"board-open":"board-close";
        float time=altar.transition(partial),duration=model.duration(clip);
        state.board=null;
        if(!occupied&&time>=duration)return;
        if(time<duration){if(occupied)++openingFrames;else ++closingFrames;}
        state.board=model.poseAtTime(clip,time>=duration?Float.MAX_VALUE:time);
    }

    @Override public void submit(State state,PoseStack stack,SubmitNodeCollector collector,CameraRenderState camera){
        if(state.board==null)return;
        stack.pushPose();
        stack.translate(.5,.94,.5);
        stack.scale(.64f,.64f,.64f);
        JakRenderer.boardModel().submit(state.board,stack,collector,state.lightCoords);
        stack.popPose();
    }
}
