package dev.jakmc.mixin;

import dev.jakmc.player.JakEntityClock;
import net.minecraft.world.TickRateManager;
import net.minecraft.world.entity.Entity;
import org.spongepowered.asm.mixin.Mixin;
import org.spongepowered.asm.mixin.injection.At;
import org.spongepowered.asm.mixin.injection.Inject;
import org.spongepowered.asm.mixin.injection.callback.CallbackInfoReturnable;

@Mixin(TickRateManager.class)
abstract class JakFreezeTickMixin {
    @Inject(method="isEntityFrozen",at=@At("RETURN"),cancellable=true)
    private void jakmc$slowEntities(Entity entity,CallbackInfoReturnable<Boolean> ci){
        if(!ci.getReturnValue()&&!JakEntityClock.of(entity.level()).advancing()&&JakEntityClock.affected(entity)){
            ci.setReturnValue(true);
        }
    }
}
