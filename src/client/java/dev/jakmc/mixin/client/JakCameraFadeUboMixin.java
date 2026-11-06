package dev.jakmc.mixin.client;

import com.llamalad7.mixinextras.injector.ModifyExpressionValue;
import dev.jakmc.client.camera.JakCameraCutout;
import net.minecraft.client.renderer.GlobalSettingsUniform;
import org.spongepowered.asm.mixin.Mixin;
import org.spongepowered.asm.mixin.injection.At;

/** Room in vanilla's Globals UBO for Jak's camera masks (minecraft:globals.glsl, JakCameraCutout.UBO_END). */
@Mixin(GlobalSettingsUniform.class)
abstract class JakCameraFadeUboMixin {
    @ModifyExpressionValue(method={"<init>","update"},at=@At(value="FIELD",target="Lnet/minecraft/client/renderer/GlobalSettingsUniform;UBO_SIZE:I"))
    private int jakmc$maskRoom(int vanilla){
        return Math.max(vanilla,JakCameraCutout.UBO_END);
    }
}
