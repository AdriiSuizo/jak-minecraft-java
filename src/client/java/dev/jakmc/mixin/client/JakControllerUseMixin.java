package dev.jakmc.mixin.client;
import dev.jakmc.client.JakInput;
import net.minecraft.client.*;
import org.spongepowered.asm.mixin.Mixin;
import org.spongepowered.asm.mixin.injection.*;
import org.spongepowered.asm.mixin.injection.callback.CallbackInfoReturnable;
/** Controller triggers use Minecraft's own item/block/entity interaction paths. */
@Mixin(KeyMapping.class)
abstract class JakControllerUseMixin {
 @Inject(method="isDown",at=@At("HEAD"),cancellable=true)
 private void jakmc$held(CallbackInfoReturnable<Boolean> ci){var c=Minecraft.getInstance();if(c.options==null)return;
  if((Object)this==c.options.keyUse&&JakInput.useHeld()||(Object)this==c.options.keyAttack&&JakInput.attackHeld())ci.setReturnValue(true);}
 @Inject(method="consumeClick",at=@At("HEAD"),cancellable=true)
 private void jakmc$pressed(CallbackInfoReturnable<Boolean> ci){var c=Minecraft.getInstance();if(c.options==null)return;
  if((Object)this==c.options.keyUse&&JakInput.consumeUse()||(Object)this==c.options.keyAttack&&JakInput.consumeAttack())ci.setReturnValue(true);}
}
