package dev.jakmc.mixin;

import dev.jakmc.player.JakBoards;
import net.minecraft.server.level.ServerPlayer;
import net.minecraft.world.entity.ExperienceOrb;
import org.spongepowered.asm.mixin.Mixin;
import org.spongepowered.asm.mixin.Shadow;
import org.spongepowered.asm.mixin.injection.At;
import org.spongepowered.asm.mixin.injection.Inject;
import org.spongepowered.asm.mixin.injection.callback.CallbackInfoReturnable;

@Mixin(ExperienceOrb.class)
abstract class BoardMendingMixin {
    @Shadow private int repairPlayerItems(ServerPlayer player,int amount){throw new AssertionError();}
    @Inject(method="repairPlayerItems",at=@At("HEAD"),cancellable=true)
    private void jakmc$repairBoard(ServerPlayer player,int amount,CallbackInfoReturnable<Integer> cir){
        int remaining=JakBoards.repair(player,amount);
        if(remaining<amount)cir.setReturnValue(remaining==0?0:repairPlayerItems(player,remaining));
    }
}
