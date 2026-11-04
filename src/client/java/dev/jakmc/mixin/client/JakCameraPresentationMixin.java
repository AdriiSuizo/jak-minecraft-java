package dev.jakmc.mixin.client;

import com.mojang.blaze3d.vertex.PoseStack;
import dev.jakmc.client.JakPoses;
import net.minecraft.client.Minecraft;
import net.minecraft.client.renderer.GameRenderer;
import net.minecraft.client.renderer.state.level.CameraRenderState;
import org.spongepowered.asm.mixin.Mixin;
import org.spongepowered.asm.mixin.injection.*;
import org.spongepowered.asm.mixin.injection.callback.CallbackInfo;

@Mixin(GameRenderer.class)
abstract class JakCameraPresentationMixin {
    @Inject(method="bobView",at=@At("HEAD"),cancellable=true)
    private void jakmc$noVanillaWalkBob(CameraRenderState state,PoseStack stack,CallbackInfo ci){
        var client=Minecraft.getInstance();
        if(client.getCameraEntity()==client.player&&(JakPoses.presented()!=null
            ||client.player!=null&&client.player.getVehicle() instanceof dev.jakmc.entity.JakVehicle))ci.cancel();
    }
}
