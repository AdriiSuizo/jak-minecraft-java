package dev.jakmc.client;

import dev.jakmc.JakData;
import dev.jakmc.content.JakItems;
import dev.jakmc.player.BoardSlot;
import dev.jakmc.player.JakResources;
import dev.jakmc.mixin.client.CreativeInventoryActions;
import dev.jakmc.mixin.client.CreativeBoardSlotAccess;
import com.google.gson.JsonObject;
import java.nio.file.*;
import java.util.UUID;
import java.util.function.Consumer;
import net.fabricmc.fabric.api.client.event.lifecycle.v1.ClientTickEvents;
import net.minecraft.client.*;
import net.minecraft.client.gui.screens.inventory.*;
import net.minecraft.core.registries.*;
import net.minecraft.core.component.DataComponents;
import net.minecraft.network.protocol.game.ServerboundSetCreativeModeSlotPacket;
import net.minecraft.server.level.ServerPlayer;
import net.minecraft.world.inventory.*;
import net.minecraft.world.item.*;
import net.minecraft.world.item.enchantment.*;
import net.minecraft.world.entity.item.ItemEntity;
import net.minecraft.world.level.GameType;
import net.minecraft.world.level.gamerules.GameRules;

/** Native UI packets plus real deaths/respawns, in the runner's disposable save. */
final class EquipmentScene {
    private static boolean running;private static volatile boolean ready;private static int tick,clothBefore;private static UUID uuid;private static JsonObject result;
    static void initialize(){ClientTickEvents.END_CLIENT_TICK.register(EquipmentScene::tick);}
    static void start(Minecraft c){
        if(running||c.player==null||c.getSingleplayerServer()==null)throw new IllegalStateException("Open isolated world");
        running=true;ready=false;tick=0;uuid=c.player.getUUID();result=new JsonObject();c.gui.setScreen(null);
        server(c,p->{p.closeContainer();p.setAttached(JakData.ENABLED,false);p.setAttached(JakData.BOARD,ItemStack.EMPTY);
            p.setAttached(JakData.INVENTORY_RESOURCES,true);p.getInventory().clearContent();p.setGameMode(GameType.SURVIVAL);
            p.setHealth(p.getMaxHealth());p.getFoodData().setFoodLevel(20);
            var board=board(p,"equipment-menu");p.getInventory().setItem(0,board);
            p.getInventory().setItem(40,new ItemStack(Items.SHIELD));p.getInventory().setSelectedSlot(0);ready=true;});
    }
    private static ItemStack board(ServerPlayer p,String name){
        var b=new ItemStack(JakItems.BOARD);b.setDamageValue(123);
        b.set(DataComponents.CUSTOM_NAME,net.minecraft.network.chat.Component.literal(name));
        b.enchant(p.registryAccess().lookupOrThrow(Registries.ENCHANTMENT).getOrThrow(Enchantments.MENDING),1);return b;
    }
    private static void check(String name,boolean ok){result.addProperty(name,ok);}
    private static void server(Minecraft c,Consumer<ServerPlayer> action){c.getSingleplayerServer().execute(()->{
        try{var p=c.getSingleplayerServer().getPlayerList().getPlayer(uuid);if(p!=null)action.accept(p);}
        catch(Throwable failure){result.addProperty("failure",failure.toString());}
    });}
    private static void click(Minecraft c,int index,ContainerInput input){
        c.gameMode.handleContainerInput(c.player.inventoryMenu.containerId,index,0,input,c.player);
    }
    private static void creativeClick(Minecraft c,int target){
        if(!(c.gui.screen() instanceof CreativeModeInventoryScreen screen)){check("creative_screen",false);return;}
        var wrapper=screen.getMenu().slots.stream().filter(slot->slot instanceof CreativeBoardSlotAccess access&&access.jakmc$target().index==target).findFirst().orElseThrow();
        ((CreativeInventoryActions)screen).jakmc$clickSlot(wrapper,wrapper.index,0,ContainerInput.PICKUP);
    }
    private static void capture(Minecraft c,String name){Screenshot.grab(c.gameDirectory,name+".png",c.gameRenderer.mainRenderTarget(),1,_->{});}
    private static void die(ServerPlayer p,boolean keep,String name,boolean vanishing){
        p.closeContainer();p.getInventory().clearContent();p.level().getGameRules().set(GameRules.KEEP_INVENTORY,keep,p.level().getServer());
        p.setGameMode(GameType.SURVIVAL);p.setHealth(p.getMaxHealth());var b=board(p,name);
        if(vanishing)b.enchant(p.registryAccess().lookupOrThrow(Registries.ENCHANTMENT).getOrThrow(Enchantments.VANISHING_CURSE),1);
        p.setAttached(JakData.BOARD,b);JakResources.grant(p,1,12);
        for(int i=0;i<p.getInventory().getContainerSize();i++)if(p.getInventory().getItem(i).is(JakItems.LIGHT_ECO))
            p.getInventory().getItem(i).set(DataComponents.CUSTOM_NAME,net.minecraft.network.chat.Component.literal("fuel-"+name));
        p.hurtServer(p.level(),p.damageSources().genericKill(),Float.MAX_VALUE);
    }
    private static long drops(ServerPlayer p,String name){return p.level().getEntitiesOfClass(ItemEntity.class,p.getBoundingBox().inflate(16),item->item.getItem().is(JakItems.BOARD)
        &&item.getItem().getHoverName().getString().equals(name)&&item.getItem().getDamageValue()==123).size();}
    private static void tick(Minecraft c){
        if(!running||c.player==null)return;
        if(tick==0&&(!ready||c.player.hasInfiniteMaterials()||!c.player.getInventory().getItem(0).is(JakItems.BOARD)))return;
        int t=++tick;int slot=BoardSlot.index(c.player.inventoryMenu);
        if(t==8){check("native_board_slot",slot>=46);c.gui.setScreen(new InventoryScreen(c.player));}
        if(t==15){check("survival_native_inventory",c.gui.screen() instanceof InventoryScreen);click(c,36,ContainerInput.QUICK_MOVE);}
        if(t==23)server(c,p->{var b=p.getAttachedOrCreate(JakData.BOARD);check("survival_shift_equips",b.is(JakItems.BOARD)&&b.getDamageValue()==123&&!b.getOrDefault(DataComponents.ENCHANTMENTS,net.minecraft.world.item.enchantment.ItemEnchantments.EMPTY).isEmpty()&&p.getInventory().getItem(0).isEmpty());check("offhand_preserved",p.getOffhandItem().is(Items.SHIELD));});
        if(t==25){check("survival_inventory_remains_open",c.gui.screen() instanceof InventoryScreen);capture(c,"equipment-survival");}
        if(t==30)click(c,slot,ContainerInput.PICKUP);
        if(t==34)click(c,9,ContainerInput.PICKUP);
        if(t==40)server(c,p->check("survival_click_unequips",p.getAttachedOrCreate(JakData.BOARD).isEmpty()&&p.getInventory().getItem(9).is(JakItems.BOARD)));
        if(t==44)click(c,9,ContainerInput.QUICK_MOVE);
        if(t==50)server(c,p->p.setGameMode(GameType.CREATIVE));
        if(t==56){
            var screen=new CreativeModeInventoryScreen(c.player,c.player.connection.enabledFeatures(),false);
            c.gui.setScreen(screen);
            ((CreativeInventoryActions)screen).jakmc$selectTab(BuiltInRegistries.CREATIVE_MODE_TAB.getValue(CreativeModeTabs.INVENTORY.identifier()));
        }
        if(t==60){check("creative_native_inventory",c.gui.screen() instanceof CreativeModeInventoryScreen);capture(c,"equipment-creative");}
        if(t==65)creativeClick(c,slot);
        if(t==69)creativeClick(c,36);
        if(t==76)server(c,p->check("creative_click_unequips_on_server",p.getAttachedOrCreate(JakData.BOARD).isEmpty()&&p.getInventory().getItem(0).is(JakItems.BOARD)));
        if(t==78){c.getConnection().send(new ServerboundSetCreativeModeSlotPacket(slot,new ItemStack(Items.DIAMOND)));c.getConnection().send(new ServerboundSetCreativeModeSlotPacket(slot,new ItemStack(JakItems.BOARD,2)));}
        if(t==82)server(c,p->check("creative_rejects_wrong_item_and_oversized_stack",p.getAttachedOrCreate(JakData.BOARD).isEmpty()));
        if(t==84)creativeClick(c,36);
        if(t==88)creativeClick(c,slot);
        if(t==94)server(c,p->check("creative_reequip_preserves_data_no_duplicate",p.getAttachedOrCreate(JakData.BOARD).getDamageValue()==123&&p.getInventory().countItem(JakItems.BOARD)==0));
        if(t==100){c.gui.setScreen(null);server(c,p->die(p,false,"death-drop",false));}
        if(t==106)server(c,p->{check("death_drops_board_once",p.getAttachedOrCreate(JakData.BOARD).isEmpty()&&drops(p,"death-drop")==1);
            for(var item:p.level().getEntitiesOfClass(ItemEntity.class,p.getBoundingBox().inflate(16),e->e.getItem().getHoverName().getString().equals("fuel-death-drop")))item.discard();});
        if(t==108)c.player.respawn();
        if(t==117)server(c,p->check("respawn_does_not_duplicate_board_or_eco",p.isAlive()&&p.getAttachedOrCreate(JakData.BOARD).isEmpty()&&JakResources.totals(p)[1]==0));
        if(t==124)server(c,p->die(p,false,"death-vanishing",true));
        if(t==130)server(c,p->{check("vanishing_board_not_dropped",p.getAttachedOrCreate(JakData.BOARD).isEmpty()&&drops(p,"death-vanishing")==0);
            for(var item:p.level().getEntitiesOfClass(ItemEntity.class,p.getBoundingBox().inflate(16),e->e.getItem().getHoverName().getString().equals("fuel-death-vanishing")))item.discard();});
        if(t==134)c.player.respawn();
        if(t==150)server(c,p->die(p,true,"death-keep",false));
        if(t==156)server(c,p->check("keep_inventory_keeps_board_no_drop",p.getAttachedOrCreate(JakData.BOARD).getDamageValue()==123&&drops(p,"death-keep")==0));
        if(t==160)c.player.respawn();
        if(t==172)server(c,p->{check("keep_inventory_survives_respawn",p.isAlive()&&p.getAttachedOrCreate(JakData.BOARD).getDamageValue()==123&&JakResources.totals(p)[1]==12);
            dev.jakmc.JakMod.LOG.info("Equipment respawn alive={}, boardDamage={}, lightEco={}",p.isAlive(),p.getAttachedOrCreate(JakData.BOARD).getDamageValue(),JakResources.totals(p)[1]);});
        if(t==184){c.options.setCameraType(CameraType.THIRD_PERSON_BACK);server(c,p->{p.setAttached(JakData.ENABLED,true);dev.jakmc.player.JakPlayers.controlForProbe(p,true);});}
        if(t==196)clothBefore=dev.jakmc.client.model.JakRenderer.worldClothResets(c.player);
        if(t==198)c.gui.setScreen(new InventoryScreen(c.player));
        if(t==210)capture(c,"equipment-jak-preview");
        if(t==218){int resets=dev.jakmc.client.model.JakRenderer.worldClothResets(c.player)-clothBefore;dev.jakmc.JakMod.LOG.info("GUI/world cloth resets during preview: {}",resets);check("inventory_preview_does_not_reset_world_cloth",resets<=2);
            check("inventory_renders_jak_facing_viewer",dev.jakmc.client.model.JakRenderer.previewRenderSerial>0&&Math.abs(net.minecraft.util.Mth.wrapDegrees(dev.jakmc.client.model.JakRenderer.previewBodyYaw-180))<45);}
        if(t>=222){result.addProperty("reason",result.entrySet().stream().allMatch(e->e.getValue().isJsonPrimitive()&&e.getValue().getAsJsonPrimitive().isBoolean()&&e.getValue().getAsBoolean())?"complete":"Equipment assertion failed");
            try{var output=c.gameDirectory.toPath().resolve("jakmc/evidence/equipment");Files.createDirectories(output);Files.writeString(output.resolve("result.json"),result.toString());}catch(Exception e){throw new RuntimeException(e);}
            running=false;c.stop();}
    }
}
