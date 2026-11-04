package dev.jakmc.mixin;

import dev.jakmc.content.JakItems;
import dev.jakmc.player.BoardSlot;
import net.minecraft.core.NonNullList;
import net.minecraft.world.entity.player.Inventory;
import net.minecraft.world.entity.player.Player;
import net.minecraft.world.inventory.InventoryMenu;
import net.minecraft.world.inventory.Slot;
import net.minecraft.world.item.ItemStack;
import org.spongepowered.asm.mixin.*;
import org.spongepowered.asm.mixin.injection.*;
import org.spongepowered.asm.mixin.injection.callback.*;

@Mixin(InventoryMenu.class)
abstract class BoardInventoryMixin extends net.minecraft.world.inventory.AbstractContainerMenu {
    protected BoardInventoryMixin(){super(null,0);}
    @Inject(method="<init>",at=@At("TAIL"))
    private void jakmc$boardSlot(Inventory inventory,boolean active,Player owner,CallbackInfo ci){addSlot(new BoardSlot(owner));}
    @Inject(method="quickMoveStack",at=@At("HEAD"),cancellable=true)
    private void jakmc$shiftBoard(Player player,int index,CallbackInfoReturnable<ItemStack> cir){
        int board=BoardSlot.index(player.inventoryMenu);
        if(board<0||index<9||index>=slots.size())return;
        Slot source=slots.get(index);ItemStack stack=source.getItem();
        if(index!=board&&(!stack.is(JakItems.BOARD)||slots.get(board).hasItem()))return;
        var before=stack.copy();
        boolean moved=index==board?moveItemStackTo(stack,9,45,false):moveItemStackTo(stack,board,board+1,false);
        if(!moved){cir.setReturnValue(ItemStack.EMPTY);return;}
        if(stack.isEmpty())source.setByPlayer(ItemStack.EMPTY,before);else source.setChanged();
        source.onTake(player,stack);cir.setReturnValue(before);
    }
}
