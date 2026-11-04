package dev.jakmc.player;

import dev.jakmc.JakData;
import dev.jakmc.content.JakItems;
import net.minecraft.core.component.DataComponents;
import net.minecraft.server.level.ServerPlayer;
import net.minecraft.world.item.enchantment.EnchantmentEffectComponents;
import net.minecraft.world.item.enchantment.EnchantmentHelper;
import net.minecraft.world.item.enchantment.ItemEnchantments;

/** Use Minecraft durability and enchantment processing for the separate board slot. */
public final class JakBoards {
    public static void wear(ServerPlayer player){
        var board=player.getAttachedOrCreate(JakData.BOARD).copy();
        if(!board.is(JakItems.BOARD))return;
        board.hurtAndBreak(1,player.level(),player,broken->
            player.level().playSound(null,player.blockPosition(),net.minecraft.sounds.SoundEvents.ITEM_BREAK.value(),
                net.minecraft.sounds.SoundSource.PLAYERS,1,1));
        player.setAttached(JakData.BOARD,board);
    }
    public static int repair(ServerPlayer player,int xp){
        var board=player.getAttachedOrCreate(JakData.BOARD).copy();
        if(xp<=0||!board.isDamaged())return xp;
        boolean mending=board.getOrDefault(DataComponents.ENCHANTMENTS,ItemEnchantments.EMPTY).entrySet().stream()
            .anyMatch(entry->entry.getKey().value().effects().has(EnchantmentEffectComponents.REPAIR_WITH_XP));
        if(!mending)return xp;
        int possible=EnchantmentHelper.modifyDurabilityToRepairFromXp(player.level(),board,xp);
        int repaired=Math.min(possible,board.getDamageValue());
        if(repaired<=0)return xp;
        board.setDamageValue(board.getDamageValue()-repaired);
        player.setAttached(JakData.BOARD,board);
        return xp-repaired*xp/possible;
    }
}
