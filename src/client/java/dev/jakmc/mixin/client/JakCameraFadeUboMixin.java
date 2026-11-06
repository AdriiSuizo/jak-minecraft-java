package dev.jakmc.mixin.client;

import com.llamalad7.mixinextras.injector.ModifyExpressionValue;
import com.mojang.blaze3d.buffers.Std140SizeCalculator;
import net.minecraft.client.renderer.DynamicGpuData;
import org.spongepowered.asm.mixin.Mixin;
import org.spongepowered.asm.mixin.injection.At;

/** Room in the terrain UBO for the Jak camera cutaway (two vec4 after TextureSize). */
@Mixin(DynamicGpuData.class)
abstract class JakCameraFadeUboMixin {
    @ModifyExpressionValue(method="<init>",at=@At(value="FIELD",target="Lnet/minecraft/client/renderer/DynamicGpuData;TERRAIN_TRANSFORM_UBO_SIZE:I"))
    private int jakmc$fadeRoom(int vanilla){
        return Math.max(vanilla,new Std140SizeCalculator().putMat4f().putIVec2().putVec4().putVec4().get());
    }
}
