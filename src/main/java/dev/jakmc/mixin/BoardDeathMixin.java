package dev.jakmc.mixin;

import dev.jakmc.JakData;
import net.minecraft.server.level.ServerLevel;
import net.minecraft.world.entity.player.Player;
import net.minecraft.world.item.ItemStack;
import net.minecraft.world.item.enchantment.EnchantmentHelper;
import net.minecraft.world.item.enchantment.EnchantmentEffectComponents;
import net.minecraft.world.level.gamerules.GameRules;
import org.spongepowered.asm.mixin.Mixin;
import org.spongepowered.asm.mixin.injection.*;
import org.spongepowered.asm.mixin.injection.callback.CallbackInfo;

@Mixin(Player.class)
abstract class BoardDeathMixin {
    @Inject(method="dropEquipment",at=@At("TAIL"))
    private void jakmc$dropBoard(ServerLevel level,CallbackInfo ci){
        if(level.getGameRules().get(GameRules.KEEP_INVENTORY))return;
        var player=(Player)(Object)this;var board=player.getAttachedOrCreate(JakData.BOARD);
        if(board.isEmpty())return;
        player.setAttached(JakData.BOARD,ItemStack.EMPTY);
        if(!EnchantmentHelper.has(board,EnchantmentEffectComponents.PREVENT_EQUIPMENT_DROP)){
            var drop=player.createItemStackToDrop(board.copy(),true,false);
            if(drop!=null)level.addFreshEntity(drop);
        }
    }
}
