package dev.jakmc.mixin.client;
import dev.jakmc.client.JakInput;
import net.minecraft.client.*;
import org.spongepowered.asm.mixin.Mixin;
import org.spongepowered.asm.mixin.injection.*;
import org.spongepowered.asm.mixin.injection.callback.CallbackInfo;
@Mixin(Camera.class)
abstract class JakControllerCameraMixin {
 @Inject(method="update",at=@At("HEAD"))
 private void jakmc$controller(DeltaTracker deltaTracker,CallbackInfo ci){JakInput.frame(Minecraft.getInstance());}
}
