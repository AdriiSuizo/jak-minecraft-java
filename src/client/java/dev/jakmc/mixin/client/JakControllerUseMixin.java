package dev.jakmc.mixin.client;
import dev.jakmc.client.JakInput;
import net.minecraft.client.*;
import org.spongepowered.asm.mixin.Mixin;
import org.spongepowered.asm.mixin.injection.*;
import org.spongepowered.asm.mixin.injection.callback.CallbackInfoReturnable;
/** Controller Triangle uses Minecraft's own item/block/entity interaction path. */
@Mixin(KeyMapping.class)
abstract class JakControllerUseMixin {
 @Inject(method="isDown",at=@At("HEAD"),cancellable=true)
 private void jakmc$held(CallbackInfoReturnable<Boolean> ci){var c=Minecraft.getInstance();if(c.options!=null&&(Object)this==c.options.keyUse&&JakInput.useHeld())ci.setReturnValue(true);}
 @Inject(method="consumeClick",at=@At("HEAD"),cancellable=true)
 private void jakmc$pressed(CallbackInfoReturnable<Boolean> ci){var c=Minecraft.getInstance();if(c.options!=null&&(Object)this==c.options.keyUse&&JakInput.consumeUse())ci.setReturnValue(true);}
}
