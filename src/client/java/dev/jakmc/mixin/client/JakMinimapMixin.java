package dev.jakmc.mixin.client;

import dev.jakmc.client.JakMinimap;
import net.minecraft.client.DeltaTracker;
import net.minecraft.client.gui.GuiGraphicsExtractor;
import net.minecraft.client.gui.Hud;
import org.spongepowered.asm.mixin.Mixin;
import org.spongepowered.asm.mixin.Shadow;
import org.spongepowered.asm.mixin.injection.At;
import org.spongepowered.asm.mixin.injection.Inject;
import org.spongepowered.asm.mixin.injection.callback.CallbackInfo;

@Mixin(Hud.class)
abstract class JakMinimapMixin {
    @Shadow public abstract boolean isHidden();
    @Inject(method="extractRenderState",at=@At("TAIL"))
    private void jakmc$map(GuiGraphicsExtractor graphics,DeltaTracker delta,CallbackInfo ci){
        if(!isHidden())JakMinimap.draw(graphics,delta);
    }
}
