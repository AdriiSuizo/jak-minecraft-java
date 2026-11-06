package dev.jakmc.mixin.client;

import dev.jakmc.client.camera.JakCameraCutout;
import java.nio.ByteBuffer;
import net.minecraft.client.renderer.DynamicGpuData;
import org.spongepowered.asm.mixin.Mixin;
import org.spongepowered.asm.mixin.injection.*;
import org.spongepowered.asm.mixin.injection.callback.CallbackInfo;

/** Writes JakFadeEye/JakFadeTarget (std140 offsets 80 and 96) beside vanilla's terrain transform. */
@Mixin(DynamicGpuData.TerrainTransform.class)
abstract class JakCameraFadeWriteMixin {
    @Inject(method="write",at=@At("HEAD"))
    private void jakmc$writeFade(ByteBuffer buffer,CallbackInfo ci){
        int start=buffer.position();float[] fade=JakCameraCutout.fade();
        for(int i=0;i<8;i++)buffer.putFloat(start+80+i*4,fade[i]);
    }
}
