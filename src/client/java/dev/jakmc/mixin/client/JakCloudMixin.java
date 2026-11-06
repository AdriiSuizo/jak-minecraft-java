package dev.jakmc.mixin.client;

import dev.jakmc.client.JakSky;
import net.minecraft.client.renderer.CloudRenderer;
import org.spongepowered.asm.mixin.Mixin;
import org.spongepowered.asm.mixin.injection.At;
import org.spongepowered.asm.mixin.injection.Inject;
import org.spongepowered.asm.mixin.injection.callback.CallbackInfo;

/** Jak's sky has its own clouds (minecraft_world.rs: no Minecraft clouds while it is drawn). */
@Mixin(CloudRenderer.class)
abstract class JakCloudMixin {
    @Inject(method="render(Lnet/minecraft/client/CloudStatus;Lcom/mojang/renderpearl/api/commands/RenderPass;)V",at=@At("HEAD"),cancellable=true)
    private void jakmc$clouds(CallbackInfo ci){if(JakSky.celestial())ci.cancel();}
    @Inject(method="renderOit",at=@At("HEAD"),cancellable=true)
    private void jakmc$oitClouds(CallbackInfo ci){if(JakSky.celestial())ci.cancel();}
}
