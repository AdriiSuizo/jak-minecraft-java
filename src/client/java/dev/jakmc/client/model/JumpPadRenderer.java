package dev.jakmc.client.model;

import com.mojang.blaze3d.vertex.PoseStack;
import dev.jakmc.RuntimeConfig;
import dev.jakmc.content.JakJumpPad;
import java.io.IOException;
import java.io.UncheckedIOException;
import net.minecraft.client.renderer.SubmitNodeCollector;
import net.minecraft.client.renderer.blockentity.BlockEntityRenderer;
import net.minecraft.client.renderer.blockentity.state.BlockEntityRenderState;
import net.minecraft.client.renderer.feature.ModelFeatureRenderer;
import net.minecraft.client.renderer.state.level.CameraRenderState;
import net.minecraft.world.phys.Vec3;

/** The original `skatea-jump-pad` mesh on its block, at the scale of the original creatures (JakEnemyKind.SCALE). */
public final class JumpPadRenderer implements BlockEntityRenderer<JakJumpPad.Pad,JumpPadRenderer.State> {
 private static GltfModel model;private static GltfModel.Posed rest;private static boolean missing;
 public static final class State extends BlockEntityRenderState {}
 @Override public State createRenderState(){return new State();}
 @Override public int getViewDistance(){return 64;}
 @Override public void extractRenderState(JakJumpPad.Pad pad,State state,float partial,Vec3 camera,ModelFeatureRenderer.CrumblingOverlay breaking){
  BlockEntityRenderer.super.extractRenderState(pad,state,partial,camera,breaking);
 }
 @Override public void submit(State state,PoseStack stack,SubmitNodeCollector collector,CameraRenderState camera){
  if(model==null&&!missing){
   try{model=new GltfModel(RuntimeConfig.gameAssets("jak2").resolve("levels/skatea/skatea-jump-pad-lod0.glb"),"skatea-jump-pad",true);rest=model.pose("",0);}
   catch(IOException failure){missing=true;dev.jakmc.JakMod.LOG.warn("Jump pad mesh missing",failure);}
  }
  if(model==null)return;
  stack.pushPose();
  float m=dev.jakmc.entity.JakEnemyKind.SCALE;
  stack.translate(.5,0,.5);stack.scale(m,m,m);
  model.submit(rest,stack,collector,state.lightCoords);
  stack.popPose();
 }
}
