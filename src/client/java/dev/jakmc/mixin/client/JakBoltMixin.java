package dev.jakmc.mixin.client;

import dev.jakmc.client.JakSky;
import net.minecraft.client.renderer.entity.LightningBoltRenderer;
import org.spongepowered.asm.mixin.Mixin;
import org.spongepowered.asm.mixin.injection.At;
import org.spongepowered.asm.mixin.injection.Inject;
import org.spongepowered.asm.mixin.injection.callback.CallbackInfo;

/** Minecraft's bolts drawn as jak_weather.rs strike draws them (pal-lightning), not as Minecraft's. */
@Mixin(LightningBoltRenderer.class)
abstract class JakBoltMixin {
    @Inject(method="submit(Lnet/minecraft/client/renderer/entity/state/LightningBoltRenderState;Lcom/mojang/blaze3d/vertex/PoseStack;Lnet/minecraft/client/renderer/SubmitNodeCollector;Lnet/minecraft/client/renderer/state/level/CameraRenderState;)V",
        at=@At("HEAD"),cancellable=true)
    private void jakmc$bolt(CallbackInfo ci){if(JakSky.weatherOn())ci.cancel();}
}
