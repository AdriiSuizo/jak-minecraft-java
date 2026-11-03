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
    public static final Item BOARD=register("jet_board", p->new Item(p.stacksTo(1)) {
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
    public static final Item DARK_ECO=eco("dark_eco",false);
    public static final Item LIGHT_ECO=eco("light_eco",true);
    public static final List<Item> GUNS=new ArrayList<>();
    private static Item eco(String name,boolean light){return register(name,p->new Item(p){
        @Override public InteractionResult use(Level level,Player player,InteractionHand hand){
            if(!level.isClientSide()){
                var type=light?JakData.LIGHT_ECO:JakData.DARK_ECO;
                float amount=player.getAttachedOrCreate(type);
                if(amount>=100)return InteractionResult.PASS;
                player.setAttached(type,Math.min(100,amount+(light?2:4)));
                if(!player.hasInfiniteMaterials())player.getItemInHand(hand).shrink(1);
            }
            return InteractionResult.SUCCESS;
        }
    });}
    static Item register(String name,Function<Item.Properties,Item> factory){
        var key=ResourceKey.create(Registries.ITEM,JakData.id(name));
        Item item=Registry.register(BuiltInRegistries.ITEM,key,factory.apply(new Item.Properties().setId(key)));
        ITEMS.put(name,item);return item;
    }
    public static void initialize(){
        for(String name:List.of("scatter_gun","wave_concussor","plasmite_rpg","blaster","beam_reflexor","gyro_burster","vulcan_fury","arc_wielder","needle_lazer","peace_maker","mass_inverter","super_nova"))
            GUNS.add(register(name,p->new Item(p.stacksTo(1))));
        Registry.register(BuiltInRegistries.CREATIVE_MODE_TAB,JakData.id("equipment"),FabricCreativeModeTab.builder().title(Component.translatable("tab.jakmc.equipment")).icon(()->new ItemStack(BOARD)).displayItems((context,out)->ITEMS.values().forEach(out::accept)).build());
    }
}
