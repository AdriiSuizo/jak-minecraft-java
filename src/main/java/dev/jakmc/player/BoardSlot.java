package dev.jakmc.player;

import dev.jakmc.JakData;
import dev.jakmc.content.JakItems;
import net.minecraft.world.Container;
import net.minecraft.world.entity.player.Player;
import net.minecraft.world.inventory.AbstractContainerMenu;
import net.minecraft.world.inventory.Slot;
import net.minecraft.world.item.ItemStack;

/** A native menu slot backed by the existing persistent board attachment. */
public final class BoardSlot extends Slot {
    public BoardSlot(Player owner){super(new Equipment(owner),0,77,44);}
    @Override public boolean mayPlace(ItemStack stack){return stack.is(JakItems.BOARD);}
    @Override public int getMaxStackSize(){return 1;}
    public static int index(AbstractContainerMenu menu){
        for(var slot:menu.slots)if(slot instanceof BoardSlot)return slot.index;
        return -1;
    }
    private record Equipment(Player player) implements Container {
        public int getContainerSize(){return 1;}
        public boolean isEmpty(){return getItem(0).isEmpty();}
        public ItemStack getItem(int index){return index==0?player.getAttachedOrCreate(JakData.BOARD):ItemStack.EMPTY;}
        public ItemStack removeItem(int index,int count){
            if(index!=0||count<=0)return ItemStack.EMPTY;
            var stack=getItem(0).copy();var removed=stack.split(count);setItem(0,stack);return removed;
        }
        public ItemStack removeItemNoUpdate(int index){
            var removed=getItem(index).copy();if(index==0)setItem(0,ItemStack.EMPTY);return removed;
        }
        public void setItem(int index,ItemStack stack){
            if(index!=0)return;
            player.setAttached(JakData.BOARD,stack.isEmpty()?ItemStack.EMPTY:stack.copyWithCount(1));
        }
        public void setChanged(){player.setAttached(JakData.BOARD,getItem(0).copy());}
        public boolean stillValid(Player user){return user==player;}
        public void clearContent(){setItem(0,ItemStack.EMPTY);}
    }
}
