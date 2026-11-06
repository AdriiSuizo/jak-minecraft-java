package dev.jakmc.mixin.client;

import dev.jakmc.client.camera.JakCameraCutout;
import java.nio.ByteBuffer;
import net.minecraft.client.renderer.GlobalSettingsUniform;
import org.spongepowered.asm.mixin.Mixin;
import org.spongepowered.asm.mixin.injection.*;

/** Writes Jak's camera mask and room data after vanilla's Globals (std140 offsets from 48). */
@Mixin(GlobalSettingsUniform.class)
abstract class JakCameraFadeWriteMixin {
    @ModifyArg(method="update",at=@At(value="INVOKE",target="Lcom/mojang/renderpearl/api/commands/CommandEncoder;writeToBuffer(Lcom/mojang/renderpearl/api/buffers/GpuBufferSlice;Ljava/nio/ByteBuffer;)V"),index=1)
    private ByteBuffer jakmc$writeMasks(ByteBuffer data){
        if(data.capacity()<JakCameraCutout.UBO_END)return data;
        data.limit(JakCameraCutout.UBO_END);
        JakCameraCutout.write(data);
        return data;
    }
}
