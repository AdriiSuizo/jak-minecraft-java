package dev.jakmc.mixin.client;

import dev.jakmc.client.JakSky;
import net.minecraft.client.Minecraft;
import net.minecraft.client.renderer.LightmapRenderStateExtractor;
import net.minecraft.client.renderer.state.LightmapRenderState;
import org.spongepowered.asm.mixin.Final;
import org.spongepowered.asm.mixin.Mixin;
import org.spongepowered.asm.mixin.Shadow;
import org.spongepowered.asm.mixin.injection.At;
import org.spongepowered.asm.mixin.injection.Inject;
import org.spongepowered.asm.mixin.injection.callback.CallbackInfo;

/**
 * minecraft_world.rs: the lightning's flash on the light (sky light factor + 0.6 x the flash, at most 1.5).
 * The flash changes every two frames, so the lightmap is rebuilt each frame while it lasts and once after.
 * Minecraft's own Hide Lightning Flashes option is kept.
 */
@Mixin(LightmapRenderStateExtractor.class)
abstract class JakLightningLightMixin {
    @Shadow private boolean needsUpdate;
    @Shadow @Final private Minecraft minecraft;
    private boolean jakmc$flashed;

    @Inject(method="extract",at=@At("HEAD"))
    private void jakmc$flashFrame(LightmapRenderState state,float partialTicks,CallbackInfo ci){
        boolean flash=JakSky.weatherOn()&&JakSky.flash()>0&&!minecraft.options.hideLightningFlash().get();
        if(flash||jakmc$flashed)needsUpdate=true;
        jakmc$flashed=flash;
    }
    @Inject(method="extract",at=@At("TAIL"))
    private void jakmc$flashLight(LightmapRenderState state,float partialTicks,CallbackInfo ci){
        if(!state.needsUpdate||!jakmc$flashed)return;
        state.skyFactor=Math.max(state.skyFactor,Math.min(state.skyFactor+.6f*JakSky.flash(),1.5f));
    }
}
