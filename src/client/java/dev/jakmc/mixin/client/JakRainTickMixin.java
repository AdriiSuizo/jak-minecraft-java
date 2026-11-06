package dev.jakmc.mixin.client;

import dev.jakmc.client.JakSky;
import net.minecraft.client.multiplayer.ClientLevel;
import org.spongepowered.asm.mixin.Mixin;
import org.spongepowered.asm.mixin.injection.At;
import org.spongepowered.asm.mixin.injection.Inject;
import org.spongepowered.asm.mixin.injection.callback.CallbackInfo;

/** Minecraft's rain splashes and their sound give way to Jak 3's (jak_weather.rs: splashes 40 and 42, drips, the rain's hiss). */
@Mixin(ClientLevel.class)
abstract class JakRainTickMixin {
    @Inject(method="tickWeatherEffects",at=@At("HEAD"),cancellable=true)
    private void jakmc$splashes(CallbackInfo ci){if(JakSky.weatherOn())ci.cancel();}
}
