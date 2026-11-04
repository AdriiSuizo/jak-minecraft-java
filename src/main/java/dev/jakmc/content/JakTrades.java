package dev.jakmc.content;

import java.util.Optional;
import net.minecraft.world.entity.npc.villager.Villager;
import net.minecraft.world.item.Items;
import net.minecraft.world.item.ItemStack;
import net.minecraft.world.item.trading.*;

/** Real, saved MerchantOffers use Minecraft's trade screen and transactions. */
public final class JakTrades {
    public static void ensure(Villager citizen,MerchantOffers offers){
        if(citizen.isBaby())return;
        for(int i=0;i<4;i++){
            var ammo=JakItems.AMMUNITION.get(i);
            if(offers.stream().noneMatch(o->o.getResult().is(ammo)))
                offers.add(new MerchantOffer(new ItemCost(i==3?JakItems.DARK_ECO:JakItems.LIGHT_ECO).withComponents(b->b.expect(net.minecraft.core.component.DataComponents.MAX_STACK_SIZE,64)),new ItemStack(ammo,i==2?64:i==3?8:16),16,2,0));
        }
        for(var eco:new net.minecraft.world.item.Item[]{JakItems.LIGHT_ECO,JakItems.DARK_ECO})
            if(offers.stream().noneMatch(o->o.getResult().is(eco)))
                offers.add(new MerchantOffer(new ItemCost(Items.EMERALD,2),new ItemStack(eco,4),12,2,.05f));
        if(offers.stream().noneMatch(o->o.getCostA().is(JakItems.SKULL_GEM)))
            offers.add(new MerchantOffer(new ItemCost(JakItems.SKULL_GEM,4),new ItemStack(Items.EMERALD),12,2,.05f));
    }
    public static void restockResources(MerchantOffers offers){
        for(var offer:offers){var result=offer.getResult();
            if(result.is(JakItems.DARK_ECO)||result.is(JakItems.LIGHT_ECO)||JakItems.AMMUNITION.contains(result.getItem())||offer.getCostA().is(JakItems.SKULL_GEM))offer.resetUses();
        }
    }
    private JakTrades(){}
}
