package dev.jakmc.mixin.client;

import dev.jakmc.player.JakEntityClock;
import net.minecraft.client.multiplayer.ClientLevel;
import org.spongepowered.asm.mixin.Mixin;
import org.spongepowered.asm.mixin.injection.At;
import org.spongepowered.asm.mixin.injection.Inject;
import org.spongepowered.asm.mixin.injection.callback.CallbackInfo;

@Mixin(ClientLevel.class)
abstract class JakClientFreezeTickMixin {
    @Inject(method="tickEntities",at=@At("HEAD"))
    private void jakmc$advanceEntityClock(CallbackInfo ci){
        var level=(ClientLevel)(Object)this;
        if(level.tickRateManager().runsNormally())JakEntityClock.of(level).advance();
    }
}
