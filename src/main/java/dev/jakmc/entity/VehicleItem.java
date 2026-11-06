package dev.jakmc.entity;

import dev.jakmc.content.JakItems;
import net.minecraft.core.component.DataComponents;
import net.minecraft.nbt.CompoundTag;
import net.minecraft.util.Mth;
import net.minecraft.world.entity.SlotAccess;
import net.minecraft.world.entity.player.Player;
import net.minecraft.world.inventory.ClickAction;
import net.minecraft.world.inventory.Slot;
import net.minecraft.world.item.Item;
import net.minecraft.world.item.ItemStack;
import net.minecraft.world.item.component.CustomData;

/**
 * A vehicle carried as its item keeps its hull (Rust minecraft_game::weapons
 * vehicle_health): taken apart by blows or sunk, it comes back as worn as it
 * went. The condition shows as a tool's bar, and green eco dropped on it in an
 * inventory mends a quarter of it, one eco a click (Adrii 15/10).
 */
public class VehicleItem extends Item {
    public static final float GREEN_ECO_REPAIR=.25f;
    public VehicleItem(Properties properties){super(properties);}

    public static float health(ItemStack stack){
        var data=stack.get(DataComponents.CUSTOM_DATA);
        return data==null?1:Mth.clamp(data.copyTag().getFloatOr("HullHealth",1),0,1);
    }
    public static CompoundTag hull(ItemStack stack){
        var data=stack.get(DataComponents.CUSTOM_DATA);
        return data==null?new CompoundTag():data.copyTag();
    }
    /** Whole again, the item forgets its wear and its broken lamps. */
    public static void setHealth(ItemStack stack,float health){
        float h=Mth.clamp(health,0,1);
        CustomData.update(DataComponents.CUSTOM_DATA,stack,tag->{
            if(h<.999f)tag.putFloat("HullHealth",h);
            else{tag.remove("HullHealth");tag.remove("HullFlags");}
        });
        var data=stack.get(DataComponents.CUSTOM_DATA);
        if(data!=null&&data.copyTag().isEmpty())stack.remove(DataComponents.CUSTOM_DATA);
    }

    @Override public boolean isBarVisible(ItemStack stack){return health(stack)<.999f;}
    @Override public int getBarWidth(ItemStack stack){return Math.clamp(Math.round(13*health(stack)),0,13);}
    @Override public int getBarColor(ItemStack stack){return Mth.hsvToRgb(health(stack)/3f,1,1);}
    @Override public boolean overrideOtherStackedOnMe(ItemStack self,ItemStack other,Slot slot,ClickAction click,Player player,SlotAccess carried){
        if(!other.is(JakItems.GREEN_ECO)||health(self)>=.999f||!slot.allowModification(player))return false;
        setHealth(self,health(self)+GREEN_ECO_REPAIR);
        other.shrink(1);
        player.playSound(net.minecraft.sounds.SoundEvents.ANVIL_USE,.4f,1.6f);
        return true;
    }
}
