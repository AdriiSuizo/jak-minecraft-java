package dev.jakmc.mixin.client;

import dev.jakmc.client.JakAtmosphere;
import dev.jakmc.client.sky.JakGrade;
import net.minecraft.client.renderer.GameRenderer;
import org.spongepowered.asm.mixin.Mixin;
import org.spongepowered.asm.mixin.injection.At;
import org.spongepowered.asm.mixin.injection.Inject;
import org.spongepowered.asm.mixin.injection.callback.CallbackInfo;

/** drawsurf/enhanced_grade.rs: the grade over the finished world, before Minecraft's own post effects and the HUD. */
@Mixin(GameRenderer.class)
abstract class JakGradeMixin {
    @Inject(method="render",at=@At(value="INVOKE",target="Lnet/minecraft/client/renderer/GameRenderer;applyPostEffects()V"))
    private void jakmc$grade(CallbackInfo ci){
        JakAtmosphere.gradeFrame();
        JakGrade.apply(((GameRenderer)(Object)this).mainRenderTarget());
        dev.jakmc.client.sky.JakFreezeScreen.frame();
        dev.jakmc.client.sky.JakFreezeScreen.apply(((GameRenderer)(Object)this).mainRenderTarget());
        dev.jakmc.client.sky.JakNukeScreen.apply(((GameRenderer)(Object)this).mainRenderTarget());
    }
}
