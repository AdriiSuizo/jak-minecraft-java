package dev.jakmc.client;

import com.google.gson.JsonObject;
import dev.jakmc.JakData;
import dev.jakmc.client.input.SdlVirtualPad;
import dev.jakmc.content.JakItems;
import dev.jakmc.mixin.client.JakContainerBounds;
import java.io.IOException;
import java.io.UncheckedIOException;
import java.nio.file.Files;
import net.fabricmc.fabric.api.client.event.lifecycle.v1.ClientTickEvents;
import net.minecraft.client.Minecraft;
import net.minecraft.client.Screenshot;
import net.minecraft.client.gui.screens.inventory.AnvilScreen;
import net.minecraft.client.gui.screens.inventory.CreativeModeInventoryScreen;
import net.minecraft.client.gui.screens.inventory.MerchantScreen;
import net.minecraft.client.input.CharacterEvent;
import net.minecraft.client.input.KeyEvent;
import net.minecraft.network.chat.Component;
import net.minecraft.world.SimpleMenuProvider;
import net.minecraft.world.InteractionHand;
import net.minecraft.world.entity.EntitySpawnReason;
import net.minecraft.world.entity.EntityTypes;
import net.minecraft.world.entity.npc.villager.Villager;
import net.minecraft.world.item.trading.ItemCost;
import net.minecraft.world.item.trading.MerchantOffer;
import net.minecraft.world.item.trading.MerchantOffers;
import net.minecraft.world.phys.Vec3;
import net.minecraft.world.inventory.AnvilMenu;
import net.minecraft.world.item.ItemStack;
import net.minecraft.world.item.Items;
import net.minecraft.world.level.GameType;
import org.lwjgl.sdl.SDLGamepad;
import org.lwjgl.sdl.SDLHints;

final class MenuScene {
    private static SdlVirtualPad pad;
    private static long start;
    private static int tick;
    private static boolean pause;
    private static String previousBackground;
    private static final String BACKGROUND=SDLHints.SDL_HINT_JOYSTICK_ALLOW_BACKGROUND_EVENTS;
    private static Runnable creative,trade,restore;
    private static Villager trader;
    private static volatile boolean paymentCorrect;
    private static final JsonObject checks=new JsonObject();
    static boolean backgroundPad(int id){return pad!=null&&pad.id==id;}
    static void initialize(){ClientTickEvents.END_CLIENT_TICK.register(MenuScene::tick);}
    static void start(Minecraft client){
        if(pad!=null||client.getSingleplayerServer()==null)throw new IllegalStateException("Open the development world");
        var server=client.getSingleplayerServer();
        var owner=client.player.getUUID();
        pause=client.options.pauseOnLostFocus;
        client.options.pauseOnLostFocus=false;
        checks.entrySet().clear();tick=0;paymentCorrect=false;
        previousBackground=SDLHints.SDL_GetHint(BACKGROUND);
        SDLHints.SDL_SetHint(BACKGROUND,"1");
        pad=new SdlVirtualPad();start=System.nanoTime();
        server.execute(()->{
            var p=server.getPlayerList().getPlayer(owner);
            if(p==null)return;
            var mode=p.gameMode.getGameModeForPlayer();
            var enabled=p.getAttachedOrCreate(JakData.ENABLED);
            var saved=new java.util.ArrayList<ItemStack>();
            for(int i=0;i<p.getInventory().getContainerSize();i++)saved.add(p.getInventory().getItem(i).copy());
            p.setAttached(JakData.ENABLED,true);
            p.setGameMode(GameType.SURVIVAL);
            p.getInventory().clearContent();
            p.getInventory().setItem(0,new ItemStack(Items.IRON_INGOT,8));
            p.getInventory().setItem(1,new ItemStack(Items.GOLD_INGOT,8));
            p.openMenu(new SimpleMenuProvider((id,inventory,player)->new AnvilMenu(id,inventory),Component.translatable("container.repair")));
            creative=()->p.setGameMode(GameType.CREATIVE);
            trade=()->{
                p.setGameMode(GameType.SURVIVAL);
                p.getInventory().setItem(2,new ItemStack(Items.EMERALD,8));
                trader=EntityTypes.VILLAGER.create(p.level(),EntitySpawnReason.COMMAND);
                if(trader==null)return;
                trader.setNoAi(true);trader.setPos(p.position().add(1,0,0));p.level().addFreshEntity(trader);
                var offers=new MerchantOffers();
                offers.add(new MerchantOffer(new ItemCost(Items.EMERALD,2),new ItemStack(Items.BREAD,2),12,2,0));
                offers.add(new MerchantOffer(new ItemCost(Items.EMERALD,3),new ItemStack(Items.COOKED_BEEF,3),12,2,0));
                trader.setOffers(offers);trader.interact(p,InteractionHand.MAIN_HAND,Vec3.ZERO);
            };
            restore=()->{
                if(trader!=null){trader.discard();trader=null;}
                p.closeContainer();
                for(int i=0;i<saved.size();i++)p.getInventory().setItem(i,saved.get(i));
                p.setGameMode(mode);p.setAttached(JakData.ENABLED,enabled);
            };
        });
    }
    private static void tick(Minecraft client){
        if(pad==null)return;
        if(!JakInput.controllerReady(pad.id)||tick==0&&!(client.gui.screen() instanceof AnvilScreen)){
            if(System.nanoTime()-start>10_000_000_000L)finish(client,"Anvil or SDL not ready");
            return;
        }
        int t=++tick;
        if(t==2)cursor(client,8+8,142+8);
        pad.button(SDLGamepad.SDL_GAMEPAD_BUTTON_DPAD_RIGHT,t>=6&&t<=7);
        pad.button(SDLGamepad.SDL_GAMEPAD_BUTTON_WEST,t>=11&&t<=12);
        pad.button(SDLGamepad.SDL_GAMEPAD_BUTTON_SOUTH,t>=23&&t<=24||t>=61&&t<=62||t>=91&&t<=92);
        pad.button(SDLGamepad.SDL_GAMEPAD_BUTTON_NORTH,t>=29&&t<=30||t>=105&&t<=106);
        pad.button(SDLGamepad.SDL_GAMEPAD_BUTTON_EAST,t>=38&&t<=39||t>=72&&t<=73||t>=117&&t<=118);
        pad.button(SDLGamepad.SDL_GAMEPAD_BUTTON_DPAD_UP,t>=49&&t<=50);
        pad.button(SDLGamepad.SDL_GAMEPAD_BUTTON_DPAD_DOWN,t>=85&&t<=86);
        if(t==9){
            var window=client.getWindow();
            int left=((JakContainerBounds)client.gui.screen()).jakmc$left();
            checks.addProperty("dpad_next_slot",Math.abs(client.mouseHandler.getScaledXPos(window)-(left+34))<2);
        }
        if(t==18&&client.gui.screen() instanceof AnvilScreen anvil){
            var menu=anvil.getMenu();
            checks.addProperty("split_gold_stack",menu.getCarried().is(Items.GOLD_INGOT)&&menu.getCarried().getCount()==4);
            cursor(client,27+8,47+8);
        }
        if(t==27&&client.gui.screen() instanceof AnvilScreen anvil){
            var slot=anvil.getMenu().getSlot(0).getItem();
            checks.addProperty("placed_in_anvil",slot.is(Items.GOLD_INGOT)&&slot.getCount()==4);
            capture(client,"menus-anvil.png");
        }
        if(t==34&&client.gui.screen() instanceof AnvilScreen anvil)
            checks.addProperty("quick_moved_from_anvil",anvil.getMenu().getSlot(0).getItem().isEmpty());
        if(t==43){
            checks.addProperty("anvil_closed",client.gui.screen()==null);
            client.getSingleplayerServer().execute(creative);
        }
        if(t==55&&client.gui.screen() instanceof CreativeModeInventoryScreen screen){
            var chat=new KeyEvent(23,116,0);
            screen.keyPressed(chat);screen.keyReleased(chat);
            typeName(screen,Items.DIAMOND.getDefaultInstance());
            checks.addProperty("native_search_filtered",screen.getMenu().slots.stream()
                .anyMatch(slot->slot.getItem().is(Items.DIAMOND))&&screen.getMenu().slots.stream()
                .noneMatch(slot->slot.getItem().is(Items.COBBLESTONE)));
            var selectAll=new KeyEvent(4,97,192);
            screen.keyPressed(selectAll);screen.keyReleased(selectAll);
            typeName(screen,JakItems.SKULL_GEM.getDefaultInstance());
            var gem=screen.getMenu().slots.stream().filter(slot->slot.getItem().is(JakItems.SKULL_GEM)).findFirst();
            checks.addProperty("native_search_finds_jak_item",gem.isPresent()&&screen.getMenu().slots.stream()
                .noneMatch(slot->slot.getItem().is(Items.DIAMOND)));
            gem.ifPresent(slot->cursor(client,slot.x+8,slot.y+8));
        }
        if(t==67&&client.gui.screen() instanceof CreativeModeInventoryScreen screen){
            checks.addProperty("selected_creative_result",screen.getMenu().getCarried().is(JakItems.SKULL_GEM));
            capture(client,"menus-search.png");
        }
        if(t==77){
            checks.addProperty("creative_closed",client.gui.screen()==null);
            client.getSingleplayerServer().execute(trade);
        }
        if(t==82&&client.gui.screen() instanceof MerchantScreen)cursor(client,48,28);
        if(t==100&&client.gui.screen() instanceof MerchantScreen merchant){
            var menu=merchant.getMenu();
            var slot=menu.getSlot(2);
            checks.addProperty("dpad_selected_second_trade",slot.getItem().is(Items.COOKED_BEEF)
                &&menu.getSlot(0).getItem().getCount()==8);
            cursor(client,slot.x+8,slot.y+8);
            capture(client,"menus-trade.png");
        }
        if(t==112)client.getSingleplayerServer().execute(()->{
            if(trader!=null)paymentCorrect=trader.getOffers().get(0).getUses()==0&&trader.getOffers().get(1).getUses()==2;
        });
        if(t==123){
            int emeralds=0,beef=0;
            var inventory=client.player.getInventory();
            for(int i=0;i<inventory.getContainerSize();i++){
                ItemStack item=inventory.getItem(i);
                if(item.is(Items.EMERALD))emeralds+=item.getCount();
                if(item.is(Items.COOKED_BEEF))beef+=item.getCount();
            }
            checks.addProperty("trade_consumed_payment",paymentCorrect&&emeralds==2&&beef==6);
            checks.addProperty("trade_closed",client.gui.screen()==null);
            finish(client,checks.size()==12&&checks.entrySet().stream().allMatch(e->e.getValue().getAsBoolean())
                ?"complete":"Menu acceptance failed");
        }
    }
    private static void typeName(CreativeModeInventoryScreen screen,ItemStack item){
        item.getHoverName().getString().codePoints().forEach(letter->screen.charTyped(new CharacterEvent(letter)));
    }
    private static void cursor(Minecraft client,int x,int y){
        var window=client.getWindow();
        var screen=(JakContainerBounds)client.gui.screen();
        double rawX=(screen.jakmc$left()+x)*(double)window.getScreenWidth()/window.getGuiScaledWidth();
        double rawY=(screen.jakmc$top()+y)*(double)window.getScreenHeight()/window.getGuiScaledHeight();
        client.mouseHandler.onMove(window.handle(),rawX,rawY,0,0);
    }
    private static void capture(Minecraft client,String name){
        Screenshot.grab(client.gameDirectory,name,client.gameRenderer.mainRenderTarget(),1,_-> {});
    }
    private static void finish(Minecraft client,String reason){
        var result=new JsonObject();result.add("actions",checks);result.addProperty("reason",reason);
        try{
            var dir=client.gameDirectory.toPath().resolve("jakmc/evidence/menus");
            Files.createDirectories(dir);Files.writeString(dir.resolve("result.json"),result.toString());
        }catch(IOException failure){throw new UncheckedIOException(failure);}
        pad.close();pad=null;client.options.pauseOnLostFocus=pause;
        if(previousBackground==null)SDLHints.SDL_ResetHint(BACKGROUND);
        else SDLHints.SDL_SetHint(BACKGROUND,previousBackground);
        client.getSingleplayerServer().execute(()->{if(restore!=null)restore.run();client.execute(client::stop);});
    }
}
