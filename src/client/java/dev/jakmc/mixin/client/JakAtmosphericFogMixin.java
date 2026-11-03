package dev.jakmc.mixin.client;

import dev.jakmc.client.JakAtmosphere;
import net.minecraft.client.Camera;
import net.minecraft.client.DeltaTracker;
import net.minecraft.client.multiplayer.ClientLevel;
import net.minecraft.client.renderer.fog.FogData;
import net.minecraft.client.renderer.fog.FogRenderer;
import org.spongepowered.asm.mixin.Mixin;
import org.spongepowered.asm.mixin.injection.At;
import org.spongepowered.asm.mixin.injection.Inject;
import org.spongepowered.asm.mixin.injection.callback.CallbackInfoReturnable;

@Mixin(FogRenderer.class)
abstract class JakAtmosphericFogMixin {
    @Inject(method="setupFog",at=@At("RETURN"))
    private void jakmc$weather(Camera camera,int distance,DeltaTracker delta,float darken,ClientLevel level,CallbackInfoReturnable<FogData> ci){
        JakAtmosphere.applyFog(level,camera,ci.getReturnValue());
    }
}
