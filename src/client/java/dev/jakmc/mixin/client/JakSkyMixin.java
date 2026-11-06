package dev.jakmc.mixin.client;

import com.mojang.renderpearl.api.buffers.GpuBufferSlice;
import com.mojang.renderpearl.api.commands.RenderPass;
import dev.jakmc.client.JakSky;
import dev.jakmc.client.sky.JakSkyRender;
import net.minecraft.client.Camera;
import net.minecraft.client.multiplayer.ClientLevel;
import net.minecraft.client.renderer.SkyRenderer;
import net.minecraft.client.renderer.state.level.SkyRenderState;
import org.joml.Vector3fc;
import org.spongepowered.asm.mixin.Mixin;
import org.spongepowered.asm.mixin.injection.At;
import org.spongepowered.asm.mixin.injection.Inject;
import org.spongepowered.asm.mixin.injection.ModifyVariable;
import org.spongepowered.asm.mixin.injection.callback.CallbackInfo;

/** Jak 1's sky in place of Minecraft's disc, sun, sunrise and stars; Minecraft's moon stays (jak_sky.rs, minecraft_world.rs sky_fragment). */
@Mixin(SkyRenderer.class)
abstract class JakSkyMixin {
    @Inject(method="extractRenderState",at=@At("TAIL"))
    private void jakmc$frame(ClientLevel level,float partial,Camera camera,SkyRenderState state,CallbackInfo ci){JakSky.frame(level);}
    @Inject(method="render",at=@At("HEAD"))
    private void jakmc$prepare(GpuBufferSlice fog,SkyRenderState state,CallbackInfo ci){JakSkyRender.prepare();}
    @Inject(method="renderSkyDisc",at=@At("HEAD"),cancellable=true)
    private void jakmc$dome(RenderPass pass,Vector3fc color,CallbackInfo ci){if(JakSkyRender.dome(pass))ci.cancel();}
    @Inject(method="renderSunriseAndSunset",at=@At("HEAD"),cancellable=true)
    private void jakmc$sunrise(CallbackInfo ci){if(JakSky.celestial())ci.cancel();}
    @Inject(method="renderSun",at=@At("HEAD"),cancellable=true)
    private void jakmc$sun(CallbackInfo ci){if(JakSky.celestial())ci.cancel();}
    @Inject(method="renderStars",at=@At("HEAD"),cancellable=true)
    private void jakmc$stars(CallbackInfo ci){if(JakSky.celestial())ci.cancel();}
    @Inject(method="renderDarkDisc",at=@At("HEAD"),cancellable=true)
    private void jakmc$dark(CallbackInfo ci){if(JakSky.celestial())ci.cancel();}
    // The moon through the overcast: moon * rain brightness * (1 - overcast).
    @ModifyVariable(method="renderMoon",at=@At("HEAD"),argsOnly=true)
    private float jakmc$moon(float rainBrightness){return JakSky.celestial()?rainBrightness*(1-JakSky.overcast()):rainBrightness;}
}
