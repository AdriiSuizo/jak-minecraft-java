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
 private static final RenderPipeline[] PIPELINES=new RenderPipeline[4];
 private static final OitPipelineSet[] OIT=new OitPipelineSet[4];
 private static final Map<String,RenderType> TYPES=new HashMap<>();
 static {
  BlendFunction[] blends={BlendFunction.TRANSLUCENT,new BlendFunction(BlendFactor.SRC_ALPHA,BlendFactor.ONE,BlendOp.REVERSE_SUBTRACT),BlendFunction.LIGHTNING,BlendFunction.ADDITIVE};
  for(int i=0;i<4;i++){
   var builder=RenderPipeline.builder(RenderPipelines.ENTITY_EMISSIVE_SNIPPET)
    .withLocation(JakData.id("pipeline/effect_"+i)).withFragmentShader(JakData.id("core/jak_effect"))
    .withShaderDefine("NO_OVERLAY").withShaderDefine("NO_CARDINAL_LIGHTING").withShaderDefine("JAK_BLEND",i)
    .withColorTargetState(new ColorTargetState(blends[i])).withCull(false)
    .withDepthStencilState(new DepthStencilState(CompareOp.GREATER_THAN_OR_EQUAL,false));
   PIPELINES[i]=RenderPipelines.register(builder.build());
   var oit=RenderPipeline.builder(RenderPipelines.OIT_ENTITY_SNIPPET).withCull(false)
    .withFragmentShader(JakData.id("core/jak_effect")).withShaderDefine("EMISSIVE")
    .withShaderDefine("NO_OVERLAY").withShaderDefine("NO_CARDINAL_LIGHTING").withShaderDefine("JAK_BLEND",i);
   if(i!=0)oit.withShaderDefine("OIT_ADDITIVE");
   OIT[i]=RenderPipelines.register(OitPipelineSet.builder("jakmc_effect_"+i,oit).build());
  }
 }
 public static RenderType get(Identifier texture,int blend){return TYPES.computeIfAbsent(texture+"/"+blend,k->RenderType.create("jak_effect_"+blend,RenderSetup.builder(PIPELINES[blend]).setOitPipelines(OIT[blend]).withTexture("Sampler0",texture).sortOnUpload().createRenderSetup()));}
 static void initialize(){}
}
