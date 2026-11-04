package dev.jakmc.mixin;

import dev.jakmc.content.JakTrades;
import net.minecraft.world.entity.npc.villager.AbstractVillager;
import net.minecraft.world.entity.npc.villager.Villager;
import net.minecraft.world.item.trading.MerchantOffers;
import org.spongepowered.asm.mixin.Mixin;
import org.spongepowered.asm.mixin.injection.*;
import org.spongepowered.asm.mixin.injection.callback.CallbackInfoReturnable;

@Mixin(AbstractVillager.class)
abstract class CitizenTradesMixin {
    @Inject(method="getOffers",at=@At("RETURN"))
    private void jakmc$offers(CallbackInfoReturnable<MerchantOffers> cir){
        if((Object)this instanceof Villager citizen)JakTrades.ensure(citizen,cir.getReturnValue());
    }
}
