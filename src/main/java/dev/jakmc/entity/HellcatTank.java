package dev.jakmc.entity;

import net.minecraft.network.chat.Component;
import net.minecraft.world.ContainerHelper;
import net.minecraft.world.SimpleContainer;
import net.minecraft.world.SimpleMenuProvider;
import net.minecraft.world.entity.player.Inventory;
import net.minecraft.world.entity.player.Player;
import net.minecraft.world.inventory.ChestMenu;
import net.minecraft.world.inventory.ContainerInput;
import net.minecraft.world.inventory.MenuType;
import net.minecraft.world.item.ItemStack;
import net.minecraft.world.item.Items;
import net.minecraft.world.level.storage.ValueInput;
import net.minecraft.world.level.storage.ValueOutput;

/**
 * The HellCat's own container (Rust minecraft_game containers.rs): its first
 * row the tank (coal only), the other two its boot. It burns coal as it flies
 * (Rust render_anim minecraft_world): a piece at a time, a furnace's seconds;
 * an empty tank leaves it no boost and no lift.
 */
final class HellcatTank {
    static final int FUEL_SLOTS=9,START_COAL=16;
    final SimpleContainer slots=new SimpleContainer(27);
    /** Seconds of the piece burning. */
    float burn;
    boolean fueled=true;

    HellcatTank(){fillNew();}
    /** Rust containers.rs is_hellcat_fuel. */
    static boolean fuel(ItemStack stack){return stack.is(Items.COAL)||stack.is(Items.CHARCOAL)||stack.is(Items.COAL_BLOCK);}
    /** Rust containers.rs hellcat_new: what a new HellCat comes with in its tank. */
    void fillNew(){slots.clearContent();slots.setItem(0,new ItemStack(Items.COAL,START_COAL));burn=0;}
    /** Rust containers.rs hellcat_burn_fuel: a piece from the tank, its burn seconds (80 coal and charcoal, 800 a block). */
    private float burnPiece(){
        for(int i=0;i<FUEL_SLOTS;i++){
            var stack=slots.getItem(i);
            if(stack.isEmpty()||!fuel(stack))continue;
            float seconds=stack.is(Items.COAL_BLOCK)?800:80;
            stack.shrink(1);
            slots.setChanged();
            return seconds;
        }
        return 0;
    }
    /** Rust containers.rs hellcat_fuel: the pieces in the tank. */
    int pieces(){
        int count=0;
        for(int i=0;i<FUEL_SLOTS;i++){var s=slots.getItem(i);if(fuel(s))count+=s.getCount();}
        return count;
    }
    /**
     * Rust render_anim minecraft_world, the HellCat's tank each frame: it
     * burns while piloted with its engine going (jak_entities vehicle_burning),
     * the next piece goes in once the last is spent; fuel or not, for the next
     * step.
     */
    void update(boolean piloting,float engine,float throttle,float dt){
        if(piloting&&(engine>.05f||throttle>0))burn-=dt;
        if(burn<=0&&piloting)burn+=burnPiece();
        fueled=burn>0||!piloting;
    }
    /** Rust jak_entities advance_vehicle: an empty tank, no X (boost). */
    int dry(int buttons){return fueled?buttons:buttons&~1;}

    /**
     * stand-in: the original opens it beside the inventory at the HellCat's
     * controls; here sneaking and using the parked HellCat opens it.
     */
    void open(Player player){
        player.openMenu(new SimpleMenuProvider((id,inventory,who)->new Menu(id,inventory,this),Component.literal("HellCat")));
    }
    /** Rust containers.rs click: the tank takes fuel only; anything else back to the player, or the boot if the player is full. */
    void sort(Player player){
        for(int i=0;i<FUEL_SLOTS;i++){
            var stack=slots.getItem(i);
            if(stack.isEmpty()||fuel(stack))continue;
            slots.setItem(i,ItemStack.EMPTY);
            player.getInventory().add(stack);
            if(stack.isEmpty())continue;
            int free=-1;
            for(int j=FUEL_SLOTS;j<slots.getContainerSize();j++)if(slots.getItem(j).isEmpty()){free=j;break;}
            slots.setItem(free<0?i:free,stack);
        }
    }
    private static final class Menu extends ChestMenu {
        private final HellcatTank tank;
        Menu(int id,Inventory inventory,HellcatTank tank){super(MenuType.GENERIC_9x3,id,inventory,tank.slots,3);this.tank=tank;}
        @Override public void clicked(int slot,int button,ContainerInput input,Player player){
            super.clicked(slot,button,input,player);
            if(!player.level().isClientSide())tank.sort(player);
        }
    }

    void save(ValueOutput out){
        out.putBoolean("Tank",true);
        out.putFloat("FuelBurn",burn);
        ContainerHelper.saveAllItems(out,slots.getItems());
    }
    void load(ValueInput in){
        if(!in.getBooleanOr("Tank",false))return;
        slots.clearContent();
        ContainerHelper.loadAllItems(in,slots.getItems());
        burn=Math.max(0,in.getFloatOr("FuelBurn",0));
    }
}
