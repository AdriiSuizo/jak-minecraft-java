package dev.jakmc.mixin.client;

import dev.jakmc.client.JakAtmosphere;
import net.minecraft.client.Camera;
import net.minecraft.client.multiplayer.ClientLevel;
import net.minecraft.client.renderer.SkyRenderer;
import net.minecraft.client.renderer.state.level.SkyRenderState;
import org.spongepowered.asm.mixin.Mixin;
import org.spongepowered.asm.mixin.injection.At;
import org.spongepowered.asm.mixin.injection.Inject;
import org.spongepowered.asm.mixin.injection.callback.CallbackInfo;

@Mixin(SkyRenderer.class)
abstract class JakAtmosphericSkyMixin {
    @Inject(method="extractRenderState",at=@At("TAIL"))
    private void jakmc$weather(ClientLevel level,float partial,Camera camera,SkyRenderState state,CallbackInfo ci){
        JakAtmosphere.applySky(level,state);
    }
}
