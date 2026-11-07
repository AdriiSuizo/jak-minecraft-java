package dev.jakmc.mixin.client;

import com.mojang.renderpearl.api.pipeline.RenderPipeline;
import dev.jakmc.client.JakEffectTypes;
import dev.jakmc.client.camera.JakCameraCutout;
import net.minecraft.client.renderer.chunk.ChunkSectionLayerGroup;
import net.minecraft.client.renderer.chunk.ChunkSectionsToRender;
import org.spongepowered.asm.mixin.Mixin;
import org.spongepowered.asm.mixin.Unique;
import org.spongepowered.asm.mixin.injection.At;
import org.spongepowered.asm.mixin.injection.Inject;
import org.spongepowered.asm.mixin.injection.ModifyArg;
import org.spongepowered.asm.mixin.injection.callback.CallbackInfo;

/** With Jak's camera inside rock the opaque terrain draws its back faces too, the rock's fill (terrain.fsh JAK_CAP). */
@Mixin(ChunkSectionsToRender.class)
abstract class JakCameraBuriedMixin {
    private static final String LAYERS="Lnet/minecraft/client/renderer/chunk/ChunkSectionsToRender;renderLayers([Lnet/minecraft/client/renderer/chunk/ChunkSectionLayer;Lcom/mojang/renderpearl/api/textures/GpuSampler;Lcom/mojang/renderpearl/api/commands/RenderPass;Lcom/mojang/renderpearl/api/textures/GpuTextureView;Lcom/mojang/renderpearl/api/textures/GpuTextureView;Lcom/mojang/renderpearl/api/pipeline/RenderPipeline;Lcom/mojang/renderpearl/api/pipeline/RenderPipeline;)V";
    @Unique private boolean jakmc$opaque;
    @Inject(method="renderGroup",at=@At("HEAD"))
    private void jakmc$group(ChunkSectionLayerGroup group,com.mojang.renderpearl.api.commands.RenderPass pass,com.mojang.renderpearl.api.textures.GpuSampler sampler,
        com.mojang.renderpearl.api.textures.GpuTextureView atlas,boolean wireframe,CallbackInfo ci){
        jakmc$opaque=group==ChunkSectionLayerGroup.OPAQUE;
    }
    @ModifyArg(method="renderGroup",at=@At(value="INVOKE",target=LAYERS),index=5)
    private RenderPipeline jakmc$fill(RenderPipeline wireframe){
        return wireframe==null&&jakmc$opaque&&JakCameraCutout.buried()?JakEffectTypes.TERRAIN_FILL:wireframe;
    }
    @ModifyArg(method="renderGroup",at=@At(value="INVOKE",target=LAYERS),index=6)
    private RenderPipeline jakmc$fillMultidraw(RenderPipeline wireframe){
        return wireframe==null&&jakmc$opaque&&JakCameraCutout.buried()?JakEffectTypes.TERRAIN_FILL_MULTIDRAW:wireframe;
    }
}
