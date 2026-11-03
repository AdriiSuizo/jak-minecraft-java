package dev.jakmc.client.model;
import java.util.*;
import com.mojang.renderpearl.api.pipeline.*;
import dev.jakmc.JakData;
import net.minecraft.client.renderer.RenderPipelines;
import net.minecraft.client.renderer.rendertype.*;
import net.minecraft.resources.Identifier;
/** Original gem color-mult .8 and emissive .2, with real Minecraft light/fog/depth. */
final class GemMaterial {
 private static final RenderPipeline PIPELINE=RenderPipelines.register(RenderPipeline.builder(RenderPipelines.ENTITY_SNIPPET).withLocation(JakData.id("pipeline/gem")).withFragmentShader(JakData.id("core/jak_gem")).withShaderDefine("NO_OVERLAY").withShaderDefine("NO_CARDINAL_LIGHTING").withCull(false).withColorTargetState(ColorTargetState.DEFAULT).build());
 private static final Map<Identifier,RenderType> TYPES=new HashMap<>();
 static RenderType get(Identifier texture){return TYPES.computeIfAbsent(texture,t->RenderType.create("jak_gem",RenderSetup.builder(PIPELINE).withTexture("Sampler0",t).useLightmap().createRenderSetup()));}
}
