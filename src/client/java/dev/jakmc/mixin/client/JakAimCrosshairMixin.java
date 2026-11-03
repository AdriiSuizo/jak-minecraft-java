package dev.jakmc.mixin.client;

import com.llamalad7.mixinextras.injector.ModifyExpressionValue;
import dev.jakmc.client.JakInput;
import net.minecraft.client.gui.Hud;
import org.spongepowered.asm.mixin.Mixin;
import org.spongepowered.asm.mixin.injection.At;

@Mixin(Hud.class)
abstract class JakAimCrosshairMixin {
    @ModifyExpressionValue(method="extractCrosshair",at=@At(value="INVOKE",
        target="Lnet/minecraft/client/CameraType;isFirstPerson()Z"))
    private boolean jakmc$showCrosshair(boolean firstPerson){
        return firstPerson||JakInput.aiming();
    }
}
