package dev.jakmc.content;

import dev.jakmc.*;
import java.util.*;
import java.util.function.Function;
import net.fabricmc.fabric.api.creativetab.v1.FabricCreativeModeTab;
import net.minecraft.core.Registry;
import net.minecraft.core.registries.*;
import net.minecraft.resources.ResourceKey;
import net.minecraft.network.chat.Component;
import net.minecraft.world.*;
import net.minecraft.world.entity.player.Player;
import net.minecraft.world.item.*;
import net.minecraft.world.level.Level;

public final class JakItems {
    public static final Map<String,Item> ITEMS=new LinkedHashMap<>();
    public static final Item BOARD=register("jet_board", p->new Item(p.durability(1800).enchantable(15)) {
        @Override public InteractionResult use(Level level,Player player,InteractionHand hand) {
            if (!level.isClientSide()) {
                ItemStack old=player.getAttachedOrCreate(JakData.BOARD);
                ItemStack current=player.getItemInHand(hand);
                player.setAttached(JakData.BOARD,current.copyWithCount(1));
                current.shrink(1);
                if(!old.isEmpty()&&!player.getInventory().add(old))player.spawnAtLocation((net.minecraft.server.level.ServerLevel)level,old);
                ((net.minecraft.server.level.ServerPlayer)player).sendSystemMessage(Component.translatable("message.jakmc.board_equipped"));
            }
            return InteractionResult.SUCCESS;
        }
    });
    public static final Item SKULL_GEM=register("skull_gem",Item::new);
    public static final Item GREEN_ECO=register("green_eco",p->new Item(p){
        @Override public InteractionResult use(Level level,Player player,InteractionHand hand){
            if(player.getHealth()>=player.getMaxHealth())return InteractionResult.PASS;
            if(!level.isClientSide()){
                player.setHealth(player.getMaxHealth());
                player.getItemInHand(hand).consume(1,player);
            }
            return InteractionResult.SUCCESS;
        }
    });
    public static final Item DARK_ECO=eco("dark_eco");
    public static final Item LIGHT_ECO=eco("light_eco");
    public static final List<Item> AMMUNITION=List.of(ammo("red"),ammo("yellow"),ammo("blue"),ammo("dark"));
    private static Item ammo(String color){return register(color+"_ammo",FuelItem::new);}
    public static final List<Item> GUNS=new ArrayList<>();
    private static Item eco(String name){return register(name,FuelItem::new);}
    private static final class FuelItem extends Item {
        FuelItem(Properties properties){super(properties);}
        @Override public void appendHoverText(ItemStack stack,Item.TooltipContext context,net.minecraft.world.item.component.TooltipDisplay display,java.util.function.Consumer<Component> out,TooltipFlag flag){
            var data=stack.get(net.minecraft.core.component.DataComponents.CUSTOM_DATA);
            if(data==null)return;
            int spent=data.copyTag().getIntOr("JakResourceSpent",0);if(spent<=0)return;
            int full=(this==DARK_ECO?4:this==LIGHT_ECO?2:1)*100000;
            String percent=String.format(Locale.ROOT,"%.1f%%",Math.max(0,(full-spent)*100.0/full));
            out.accept(Component.translatable("tooltip.jakmc.fuel_remaining",percent).withStyle(net.minecraft.ChatFormatting.AQUA));
            out.accept(Component.translatable("tooltip.jakmc.fuel_partial").withStyle(net.minecraft.ChatFormatting.GRAY));
        }
    }
    static Item register(String name,Function<Item.Properties,Item> factory){
        var key=ResourceKey.create(Registries.ITEM,JakData.id(name));
        Item item=Registry.register(BuiltInRegistries.ITEM,key,factory.apply(new Item.Properties().setId(key)));
        ITEMS.put(name,item);return item;
    }
    public static void initialize(){
        for(String name:List.of("scatter_gun","wave_concussor","plasmite_rpg","blaster","beam_reflexor","gyro_burster","vulcan_fury","arc_wielder","needle_lazer","peace_maker","mass_inverter","super_nova"))
            GUNS.add(register(name,p->new Item(p.stacksTo(1))));
        Registry.register(BuiltInRegistries.CREATIVE_MODE_TAB,JakData.id("equipment"),FabricCreativeModeTab.builder().title(Component.translatable("tab.jakmc.equipment")).icon(()->new ItemStack(BOARD)).displayItems((context,out)->{ITEMS.values().forEach(out::accept);out.accept(dev.jakmc.entity.JakHellcat.ITEM);out.accept(dev.jakmc.entity.JakHellcat.DOG_ITEM);for(var car:dev.jakmc.entity.JakVehicle.Model.values())out.accept(car.item);}).build());
    }
}
