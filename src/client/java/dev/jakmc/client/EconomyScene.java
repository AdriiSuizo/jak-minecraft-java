package dev.jakmc.client;

import com.google.gson.JsonObject;
import dev.jakmc.JakData;
import dev.jakmc.content.JakItems;
import dev.jakmc.player.*;
import java.nio.file.*;
import java.util.*;
import net.fabricmc.fabric.api.client.event.lifecycle.v1.ClientTickEvents;
import net.minecraft.client.*;
import net.minecraft.core.component.DataComponents;
import net.minecraft.core.registries.Registries;
import net.minecraft.resources.ResourceKey;
import net.minecraft.server.level.ServerPlayer;
import net.minecraft.world.entity.item.ItemEntity;
import net.minecraft.world.entity.npc.villager.Villager;
import net.minecraft.world.item.*;
import net.minecraft.world.item.component.*;
import net.minecraft.world.item.crafting.*;
import net.minecraft.world.item.enchantment.Enchantments;
import net.minecraft.world.phys.Vec3;

/** One bounded inventory/crafting/merchant/weapon acceptance replay in the owned dev world. */
final class EconomyScene {
    private static volatile boolean running,ready;
    private static volatile InputScene.CameraView camera;
    private static int tick,emptyShots;
    private static Path output;
    private static JsonObject result;
    private static Vec3 origin;
    private static Villager citizen;
    static InputScene.CameraView camera(){return camera;}
    static void initialize(){ClientTickEvents.END_CLIENT_TICK.register(EconomyScene::tick);}
    static void start(Minecraft client){
        if(running||client.player==null||client.getSingleplayerServer()==null)throw new IllegalStateException("Open isolated developer world");
        output=client.gameDirectory.toPath().resolve("jakmc/evidence/economy");
        try{Files.createDirectories(output);}catch(Exception e){throw new RuntimeException(e);}
        tick=emptyShots=0;result=new JsonObject();ready=false;running=true;
        var uuid=client.player.getUUID();
        client.gui.setScreen(null);client.options.setCameraType(CameraType.THIRD_PERSON_BACK);
        client.getSingleplayerServer().execute(()->{
            var p=client.getSingleplayerServer().getPlayerList().getPlayer(uuid);
            try{
                p.setAttached(JakData.ENABLED,false);p.setAttached(JakData.INVENTORY_RESOURCES,true);
                p.getInventory().clearContent();p.setGameMode(net.minecraft.world.level.GameType.SURVIVAL);
                origin=Vec3.atBottomCenterOf(p.blockPosition());
                var base=p.blockPosition();
                for(int x=-8;x<=8;x++)for(int z=-6;z<=12;z++){
                    p.level().setBlock(base.offset(x,-1,z),net.minecraft.world.level.block.Blocks.SMOOTH_STONE.defaultBlockState(),3);
                    for(int y=0;y<7;y++)p.level().setBlock(base.offset(x,y,z),net.minecraft.world.level.block.Blocks.AIR.defaultBlockState(),3);
                }
                client.getSingleplayerServer().getCommands().performPrefixedCommand(client.getSingleplayerServer().createCommandSourceStack(),"time set noon");
                nested(p);opaqueBackpack(p);recipes(p);trades(p);board(p);
                p.getInventory().clearContent();
                p.setItemInHand(net.minecraft.world.InteractionHand.MAIN_HAND,new ItemStack(JakItems.GUNS.get(3)));
                JakResources.grant(p,0,60);JakResources.grant(p,1,36);
                for(int i=0;i<4;i++)JakResources.grant(p,i+2,i==2?64:16);
                p.setAttached(JakData.ENABLED,true);p.setAttached(JakData.BOARD,new ItemStack(JakItems.BOARD));
                JakPlayers.controlForProbe(p,true);p.teleportTo(origin.x,origin.y,origin.z);
                for(int i=0;i<6;i++){
                    var item=i<4?JakItems.AMMUNITION.get(i):i==4?JakItems.DARK_ECO:JakItems.LIGHT_ECO;
                    var drop=new ItemEntity(p.level(),origin.x-3+i*1.2,origin.y+1,origin.z+3,new ItemStack(item));
                    drop.setNeverPickUp();drop.setNoGravity(true);drop.setDeltaMovement(Vec3.ZERO);p.level().addFreshEntity(drop);
                }
                for(var entity:p.level().getEntitiesOfClass(ItemEntity.class,p.getBoundingBox().inflate(15),e->e.getItem().is(Items.STICK)||e.getItem().is(Items.DEAD_BUSH)))entity.discard();
                camera=new InputScene.CameraView(origin.add(3.2,2.8,-3),35,12);
                ready=true;
            }catch(Throwable e){result.addProperty("failure",e.toString());ready=true;tick=299;}
        });
    }
    private static void check(String name,boolean value){result.addProperty(name,value);if(!value)throw new IllegalStateException(name);}
    private static boolean near(float a,float b){return Math.abs(a-b)<.001f;}
    private static void opaqueBackpack(ServerPlayer p){
        class Pocket extends net.fabricmc.fabric.api.transfer.v1.storage.base.SingleVariantStorage<net.fabricmc.fabric.api.transfer.v1.item.ItemVariant> {
            final boolean locked;
            Pocket(boolean locked){this.locked=locked;variant=net.fabricmc.fabric.api.transfer.v1.item.ItemVariant.of(JakItems.LIGHT_ECO);amount=6;}
            protected net.fabricmc.fabric.api.transfer.v1.item.ItemVariant getBlankVariant(){return net.fabricmc.fabric.api.transfer.v1.item.ItemVariant.blank();}
            protected long getCapacity(net.fabricmc.fabric.api.transfer.v1.item.ItemVariant variant){return 64;}
            protected boolean canExtract(net.fabricmc.fabric.api.transfer.v1.item.ItemVariant variant){return !locked;}
        }
        var locked=new Pocket(true);var open=new Pocket(false);
        var bag=new net.fabricmc.fabric.api.transfer.v1.storage.base.CombinedStorage<net.fabricmc.fabric.api.transfer.v1.item.ItemVariant,Pocket>(List.of(locked,open));
        // Developer-only provider, scoped to the probe's process and a vanilla
        // debug item: mimics an opaque backpack with one protected pocket.
        net.fabricmc.fabric.api.transfer.v1.item.ItemStorage.ITEM.registerForItems((stack,context)->bag,Items.DEBUG_STICK);
        p.getInventory().clearContent();p.getInventory().setItem(0,new ItemStack(Items.DEBUG_STICK));
        float total=JakResources.totals(p)[1];
        dev.jakmc.JakMod.LOG.info("Backpack resource audit: accessible={}, locked={}, open={}",total,locked.amount,open.amount);
        check("backpack_counts_only_extractable_eco_once",near(total,12));
        check("backpack_count_rolls_back_simulation",locked.amount==6&&open.amount==6);
        JakResources.consume(p,1,5);
        check("backpack_consumes_actual_fuel_and_preserves_locked_pocket",locked.amount==6&&open.amount==3&&near(JakResources.totals(p)[1],7));
        p.getInventory().clearContent();
    }
    private static void nested(ServerPlayer p){
        var bundle=new ItemStack(Items.BUNDLE);bundle.set(DataComponents.BUNDLE_CONTENTS,BundleContents.EMPTY.copyWithContents(java.util.stream.Stream.of(new ItemStack(JakItems.LIGHT_ECO,64))));
        var shulker=new ItemStack(Items.SHULKER_BOX);shulker.set(DataComponents.CONTAINER,ItemContainerContents.fromItems(List.of(new ItemStack(JakItems.DARK_ECO,3),bundle)));
        p.getInventory().setItem(0,shulker);
        var initial=JakResources.totals(p);check("nested_shulker_bundle_count",near(initial[0],12)&&near(initial[1],128));
        JakResources.consume(p,0,5.25f);JakResources.consume(p,1,.25f);
        check("resource_cannot_be_repaired_as_tool",java.util.stream.IntStream.range(0,p.getInventory().getContainerSize()).mapToObj(p.getInventory()::getItem).noneMatch(stack->(stack.is(JakItems.LIGHT_ECO)||stack.is(JakItems.DARK_ECO))&&stack.isDamageableItem()));
        var after=JakResources.totals(p);check("nested_fractional_consumption",near(after[0],6.75f)&&near(after[1],127.75f));
        var ops=p.registryAccess().createSerializationContext(net.minecraft.nbt.NbtOps.INSTANCE);
        for(int i=0;i<p.getInventory().getContainerSize();i++){
            var stack=p.getInventory().getItem(i);if(stack.isEmpty())continue;
            var tag=ItemStack.CODEC.encodeStart(ops,stack).getOrThrow();
            p.getInventory().setItem(i,ItemStack.CODEC.parse(ops,tag).getOrThrow());
        }
        var loaded=JakResources.totals(p);check("partial_charge_save_roundtrip",near(loaded[0],6.75f)&&near(loaded[1],127.75f));
        p.getInventory().clearContent();check("no_detached_eco_reserve",near(JakResources.totals(p)[0],0));
    }
    private static void recipes(ServerPlayer p){
        String[] colors={"red","yellow","blue","dark"};Item[] materials={Items.REDSTONE,Items.GOLD_INGOT,Items.LAPIS_LAZULI,Items.AMETHYST_SHARD};int[] count={16,16,64,8};
        for(int i=0;i<4;i++){
            var eco=new ItemStack(i==3?JakItems.DARK_ECO:JakItems.LIGHT_ECO);
            var input=CraftingInput.of(2,1,List.of(eco,new ItemStack(materials[i])));
            var recipe=p.level().getServer().getRecipeManager().getRecipeFor(RecipeType.CRAFTING,input,p.level());
            check("recipe_"+colors[i],recipe.isPresent()&&recipe.get().value().assemble(input).getCount()==count[i]);
            eco.set(DataComponents.MAX_STACK_SIZE,1);eco.set(DataComponents.MAX_DAMAGE,200000);eco.setDamageValue(1000);
            check("recipe_rejects_partial_"+colors[i],p.level().getServer().getRecipeManager().getRecipeFor(RecipeType.CRAFTING,CraftingInput.of(2,1,List.of(eco,new ItemStack(materials[i]))),p.level()).isEmpty());
        }
    }
    private static void trades(ServerPlayer p){
        citizen=new Villager(net.minecraft.world.entity.EntityTypes.VILLAGER,p.level());citizen.setPos(origin.add(-3,0,0));p.level().addFreshEntity(citizen);
        var offers=citizen.getOffers();int size=offers.size();check("six_resource_offers",size>=6);
        citizen.getOffers();check("no_duplicate_trades",offers.size()==size);
        for(int i=0;i<4;i++){
            var item=JakItems.AMMUNITION.get(i);var offer=offers.stream().filter(o->o.getResult().is(item)).findFirst().orElseThrow();
            var payment=offer.getCostA().copy();check("trade_consumes_eco_"+i,offer.take(payment,ItemStack.EMPTY)&&payment.isEmpty());
        }
    }
    private static void board(ServerPlayer p){
        var board=new ItemStack(JakItems.BOARD);board.setDamageValue(100);
        var mending=p.registryAccess().lookupOrThrow(Registries.ENCHANTMENT).getOrThrow(Enchantments.MENDING);board.enchant(mending,1);
        check("board_can_mend",mending.value().canEnchant(board));p.setAttached(JakData.BOARD,board);
        int remaining=JakBoards.repair(p,10);check("equipped_board_mending",p.getAttachedOrCreate(JakData.BOARD).getDamageValue()==80&&remaining==0);
        var unbreaking=p.registryAccess().lookupOrThrow(Registries.ENCHANTMENT).getOrThrow(Enchantments.UNBREAKING);
        check("board_can_unbreaking",unbreaking.value().canEnchant(board));
        for(int i=0;i<10;i++)JakBoards.wear(p);
        check("equipped_board_wear",p.getAttachedOrCreate(JakData.BOARD).getDamageValue()==90);
        board=new ItemStack(JakItems.BOARD);board.setDamageValue(board.getMaxDamage()-1);p.setAttached(JakData.BOARD,board);JakBoards.wear(p);
        check("board_breaks",p.getAttachedOrCreate(JakData.BOARD).isEmpty());
    }
    private static void tick(Minecraft client){
        if(!running||!ready)return;
        int t=++tick;var uuid=client.player.getUUID();
        if(t==1)client.gui.hud.getChat().clearMessages(false);
        if(t==4)client.getSingleplayerServer().execute(()->{
            var p=client.getSingleplayerServer().getPlayerList().getPlayer(uuid);
            for(var stack:List.of(new ItemStack(JakItems.LIGHT_ECO),new ItemStack(JakItems.AMMUNITION.get(0),2))){
                var drop=new ItemEntity(p.level(),p.getX(),p.getY()+.3,p.getZ(),stack);drop.setNoPickUpDelay();p.level().addFreshEntity(drop);
            }
        });
        if(t==35)client.getSingleplayerServer().execute(()->{
            var p=client.getSingleplayerServer().getPlayerList().getPlayer(uuid);var fuel=JakResources.totals(p);
            check("native_pickup_retains_eco_and_ammo_items",near(fuel[1],38)&&near(fuel[2],18));
        });
        if(t==40)capture(client,"economy-drops.png");
        if(t==60)client.getSingleplayerServer().execute(()->{var p=client.getSingleplayerServer().getPlayerList().getPlayer(uuid);JakPlayers.probeInput(p,64,64,0);});
        if(t==90)client.getSingleplayerServer().execute(()->{var p=client.getSingleplayerServer().getPlayerList().getPlayer(uuid);JakPlayers.probeInput(p,0,0,0);float ammo=JakResources.totals(p)[3];check("weapon_spends_inventory_ammo",ammo<16&&ammo>=0);result.addProperty("yellow_ammo_after_firing",ammo);});
        if(t==95)capture(client,"economy-gun.png");
        if(t==110)client.getSingleplayerServer().execute(()->{var p=client.getSingleplayerServer().getPlayerList().getPlayer(uuid);citizen.setTradingPlayer(p);citizen.openTradingScreen(p,net.minecraft.network.chat.Component.literal("Haven ammunition and eco"),1);});
        if(t==125)capture(client,"economy-trade.png");
        if(t==130){client.gui.setScreen(null);client.getSingleplayerServer().execute(()->{var p=client.getSingleplayerServer().getPlayerList().getPlayer(uuid);p.closeContainer();});}
        if(t==135)client.getSingleplayerServer().execute(()->{
            var p=client.getSingleplayerServer().getPlayerList().getPlayer(uuid);
            var board=new ItemStack(JakItems.BOARD);board.setDamageValue(20);
            board.enchant(p.registryAccess().lookupOrThrow(Registries.ENCHANTMENT).getOrThrow(Enchantments.MENDING),1);
            p.setAttached(JakData.BOARD,board);
            p.level().addFreshEntity(new net.minecraft.world.entity.ExperienceOrb(p.level(),p.getX(),p.getY()+.3,p.getZ(),10));
        });
        if(t==140)client.getSingleplayerServer().execute(()->{
            var p=client.getSingleplayerServer().getPlayerList().getPlayer(uuid);p.getInventory().clearContent();
            p.setItemInHand(net.minecraft.world.InteractionHand.MAIN_HAND,new ItemStack(JakItems.GUNS.get(3)));
            JakPlayers.probeInput(p,64,64,0);
        });
        if(t==155)capture(client,"economy-empty.png");
        if(t==158){
            check("empty_gun_does_not_fire",emptyShots==0);
            var pose=JakPoses.get(client.player.getId());
            check("empty_inventory_zeroes_native_eco",pose!=null&&pose.raw().get("darkEco").getAsFloat()==0&&pose.raw().get("lightEco").getAsFloat()==0);
            check("orb_repairs_equipped_board",client.player.getAttachedOrCreate(JakData.BOARD).getDamageValue()==0);
        }
        if(t>=160){
            result.addProperty("reason",result.has("failure")?"failure":"complete");
            try{Files.writeString(output.resolve("result.json"),result.toString());}catch(Exception e){throw new RuntimeException(e);}
            running=false;camera=null;client.stop();
        }
    }
    static void observe(int id,JsonObject pose){
        var client=Minecraft.getInstance();if(!running||tick<141||client.player==null||id!=client.player.getId())return;
        for(var event:pose.getAsJsonArray("events"))if(event.getAsJsonObject().get("kind").getAsString().equals("gun_fire"))emptyShots++;
    }
    private static void capture(Minecraft client,String name){Screenshot.grab(client.gameDirectory,name,client.gameRenderer.mainRenderTarget(),1,_->{});}
}
