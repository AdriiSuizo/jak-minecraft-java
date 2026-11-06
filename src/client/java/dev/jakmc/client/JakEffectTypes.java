package dev.jakmc.client;

import com.mojang.renderpearl.api.pipeline.*;
import dev.jakmc.JakData;
import net.minecraft.client.renderer.RenderPipelines;
import net.minecraft.client.renderer.oit.OitPipelineSet;
import net.minecraft.client.renderer.rendertype.*;
import net.minecraft.resources.Identifier;
import java.util.*;

/** Exact PS2 alpha equations, using Minecraft's backend and depth/fog uniforms. */
public final class JakEffectTypes {
 private static final RenderPipeline[] GUI_PIPELINES=new RenderPipeline[4];
 private static final RenderPipeline[] PIPELINES=new RenderPipeline[4];
 private static final OitPipelineSet[] OIT=new OitPipelineSet[4];
 private static final Map<String,RenderType> TYPES=new HashMap<>();
 /**
  * Light Jak's body (powers.gc rim-lights, no textures): opaque, its own rim colour unlit, culled and writing
  * depth as the Rust's body pass did. The translucent emissive type blended his back faces and inner
  * layers through him (Adrii 06/11: Light Jak all wrong).
  */
 private static final RenderPipeline LIGHT_BODY=RenderPipelines.register(RenderPipeline.builder(RenderPipelines.ENTITY_EMISSIVE_SNIPPET)
  .withLocation(JakData.id("pipeline/light_body")).withShaderDefine("ALPHA_CUTOUT",0.1f)
  .withShaderDefine("NO_OVERLAY").withShaderDefine("NO_CARDINAL_LIGHTING").withColorTargetState(ColorTargetState.DEFAULT).build());
 static {
  BlendFunction[] blends={BlendFunction.TRANSLUCENT,new BlendFunction(BlendFactor.SRC_ALPHA,BlendFactor.ONE,BlendOp.REVERSE_SUBTRACT),BlendFunction.LIGHTNING,BlendFunction.ADDITIVE};
  for(int i=0;i<4;i++){
   var builder=RenderPipeline.builder(RenderPipelines.ENTITY_EMISSIVE_SNIPPET)
    .withLocation(JakData.id("pipeline/effect_"+i)).withFragmentShader(JakData.id("core/jak_effect"))
    .withShaderDefine("NO_OVERLAY").withShaderDefine("NO_CARDINAL_LIGHTING").withShaderDefine("JAK_BLEND",i)
    .withColorTargetState(new ColorTargetState(blends[i])).withCull(false)
    .withDepthStencilState(new DepthStencilState(CompareOp.GREATER_THAN_OR_EQUAL,false));
   PIPELINES[i]=RenderPipelines.register(builder.build());
   GUI_PIPELINES[i]=RenderPipelines.register(RenderPipeline.builder(RenderPipelines.ENTITY_EMISSIVE_SNIPPET)
    .withLocation(JakData.id("pipeline/effect_gui_"+i)).withFragmentShader(JakData.id("core/jak_effect"))
    .withShaderDefine("NO_OVERLAY").withShaderDefine("NO_CARDINAL_LIGHTING").withShaderDefine("JAK_GUI").withShaderDefine("JAK_BLEND",i)
    .withColorTargetState(new ColorTargetState(BlendFunction.TRANSLUCENT)).withCull(false)
    .withDepthStencilState(new DepthStencilState(CompareOp.GREATER_THAN_OR_EQUAL,false)).build());
   var oit=RenderPipeline.builder(RenderPipelines.OIT_ENTITY_SNIPPET).withCull(false)
    .withFragmentShader(JakData.id("core/jak_effect")).withShaderDefine("EMISSIVE")
    .withShaderDefine("NO_OVERLAY").withShaderDefine("NO_CARDINAL_LIGHTING").withShaderDefine("JAK_BLEND",i);
   if(i!=0)oit.withShaderDefine("OIT_ADDITIVE");
   OIT[i]=RenderPipelines.register(OitPipelineSet.builder("jakmc_effect_"+i,oit).build());
  }
 }
 public static RenderType get(Identifier texture,int blend){return TYPES.computeIfAbsent(texture+"/"+blend,k->RenderType.create("jak_effect_"+blend,RenderSetup.builder(PIPELINES[blend]).setOitPipelines(OIT[blend]).withTexture("Sampler0",texture).sortOnUpload().createRenderSetup()));}
 public static RenderType getGui(Identifier texture,int blend){return TYPES.computeIfAbsent(texture+"/gui/"+blend,k->RenderType.create("jak_effect_gui_"+blend,RenderSetup.builder(GUI_PIPELINES[blend]).withTexture("Sampler0",texture).sortOnUpload().createRenderSetup()));}
 public static RenderType lightBody(Identifier texture){return TYPES.computeIfAbsent(texture+"/light",k->RenderType.create("jak_light_body",RenderSetup.builder(LIGHT_BODY).withTexture("Sampler0",texture).createRenderSetup()));}
 static void initialize(){}
}
