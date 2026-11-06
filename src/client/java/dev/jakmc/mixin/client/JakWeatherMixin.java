package dev.jakmc.mixin.client;

import dev.jakmc.client.JakSky;
import net.minecraft.client.renderer.WeatherEffectRenderer;
import org.spongepowered.asm.mixin.Mixin;
import org.spongepowered.asm.mixin.injection.At;
import org.spongepowered.asm.mixin.injection.Inject;
import org.spongepowered.asm.mixin.injection.callback.CallbackInfo;

/** Jak 3's rain and snow fall in place of Minecraft's columns (jak_weather.rs: the Rust drew no others). */
@Mixin(WeatherEffectRenderer.class)
abstract class JakWeatherMixin {
    @Inject(method="render(Lnet/minecraft/client/renderer/state/level/WeatherRenderState;Lcom/mojang/renderpearl/api/commands/RenderPass;Lcom/mojang/renderpearl/api/pipeline/RenderPipeline;)V",
        at=@At("HEAD"),cancellable=true)
    private void jakmc$columns(CallbackInfo ci){if(JakSky.weatherOn())ci.cancel();}
}
