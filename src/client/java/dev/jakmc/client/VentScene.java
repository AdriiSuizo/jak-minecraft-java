package dev.jakmc.client;

import com.google.gson.JsonObject;
import dev.jakmc.JakData;
import dev.jakmc.content.JakEcoVent;
import dev.jakmc.content.JakItems;
import dev.jakmc.player.JakPlayers;
import java.io.IOException;
import java.io.UncheckedIOException;
import java.nio.file.Files;
import java.util.LinkedHashMap;
import java.util.UUID;
import net.fabricmc.fabric.api.client.event.lifecycle.v1.ClientTickEvents;
import net.minecraft.client.Minecraft;
import net.minecraft.client.Screenshot;
import net.minecraft.core.BlockPos;
import net.minecraft.core.Direction;
import net.minecraft.server.level.ServerPlayer;
import net.minecraft.world.InteractionHand;
import net.minecraft.world.item.ItemStack;
import net.minecraft.world.item.context.UseOnContext;
import net.minecraft.world.level.GameType;
import net.minecraft.world.level.block.Blocks;
import net.minecraft.world.phys.BlockHitResult;
import net.minecraft.world.phys.Vec3;

/** A short bounded fixture in the regular development overworld. */
final class VentScene {
    private static InputScene.CameraView view;
    private static int ticks,peak;
    private static boolean running,pause;
    private static UUID owner;
    private static final JakEcoVent.Color[] COLORS=JakEcoVent.Color.values();
    private static final BlockPos[] vents=new BlockPos[COLORS.length];
    private static final JsonObject checks=new JsonObject();
    private static Runnable restore;
    static InputScene.CameraView camera(){return view;}
    static void initialize(){ClientTickEvents.END_CLIENT_TICK.register(VentScene::tick);}
    static void start(Minecraft client){
        if(client.player==null||client.getSingleplayerServer()==null)throw new IllegalStateException("Open the development world");
        owner=client.player.getUUID();ticks=peak=0;checks.entrySet().clear();
        pause=client.options.pauseOnLostFocus;
        client.options.pauseOnLostFocus=false;
        onServer(client,p->{
            var level=p.level();
            var old=p.position();var mode=p.gameMode.getGameModeForPlayer();
            var enabled=p.getAttachedOrCreate(JakData.ENABLED);
            var health=p.getHealth();var meter=p.getAttachedOrCreate(JakData.LIGHT_ECO);
            var saved=new java.util.ArrayList<ItemStack>();
            for(int i=0;i<p.getInventory().getContainerSize();i++)saved.add(p.getInventory().getItem(i).copy());
            var near=p.blockPosition().offset(0,0,4);
            int floor=near.getY();
            for(int i=-5;i<=5;i++)for(int z=-5;z<=5;z++)
                floor=Math.max(floor,level.getHeight(net.minecraft.world.level.levelgen.Heightmap.Types.MOTION_BLOCKING,near.getX()+i,near.getZ()+z));
            var base=near.atY(floor+2);
            var snapshot=new LinkedHashMap<BlockPos,net.minecraft.world.level.block.state.BlockState>();
            for(int x=-4;x<=4;x++)for(int z=-4;z<=4;z++){
                var at=base.offset(x,-1,z);
                snapshot.put(at,level.getBlockState(at));
                level.setBlockAndUpdate(at,Blocks.SMOOTH_STONE.defaultBlockState());
            }
            p.setGameMode(GameType.CREATIVE);
            p.setAttached(JakData.ENABLED,true);p.setAttached(JakData.LIGHT_ECO,0f);
            p.getInventory().clearContent();
            boolean placed=true;
            for(int i=0;i<COLORS.length;i++){
                var at=base.offset((i%3-1)*3,0,(i/3)*5-2);
                vents[i]=at;
                snapshot.put(at,level.getBlockState(at));
                p.setItemInHand(InteractionHand.MAIN_HAND,new ItemStack(COLORS[i].item));
                var hit=new BlockHitResult(Vec3.atBottomCenterOf(at),Direction.UP,at.below(),false);
                placed&=p.getMainHandItem().getItem().useOn(new UseOnContext(p,InteractionHand.MAIN_HAND,hit)).consumesAction();
            }
            p.setItemInHand(InteractionHand.MAIN_HAND,ItemStack.EMPTY);
            checks.addProperty("all_six_from_inventory",placed);
            var at=Vec3.atBottomCenterOf(base);
            Vec3 eye=at.add(2.8,5,-9),look=at.add(0,.65,0).subtract(eye);
            restore=()->{
                snapshot.forEach(level::setBlockAndUpdate);
                for(int i=0;i<saved.size();i++)p.getInventory().setItem(i,saved.get(i));
                JakPlayers.controlForProbe(p,false);
                p.setAttached(JakData.ENABLED,enabled);p.setAttached(JakData.LIGHT_ECO,meter);
                p.setGameMode(mode);p.setHealth(health);
                p.teleportTo(old.x,old.y,old.z);
            };
            p.teleportTo(at.x+4,at.y+.1,at.z-2);
            JakPlayers.controlForProbe(p,true);
            client.execute(()->{
                view=new InputScene.CameraView(eye,(float)Math.toDegrees(Math.atan2(-look.x,look.z)),
                    (float)-Math.toDegrees(Math.atan2(look.y,look.horizontalDistance())));
                running=true;
            });
        });
    }
    private static void tick(Minecraft client){
        if(!running)return;
        int frame=++ticks;
        peak=Math.max(peak,JakEffects.drawnVentVertices);
        if(frame==45)Screenshot.grab(client.gameDirectory,"jak-vents.png",client.gameRenderer.mainRenderTarget(),1,_-> {});
        if(frame==48)onServer(client,p->stand(p,0));
        if(frame==65)onServer(client,p->{
            int count=0;for(int i=0;i<p.getInventory().getContainerSize();i++){
                var stack=p.getInventory().getItem(i);if(stack.is(JakItems.AMMUNITION.get(0)))count+=stack.getCount();
            }
            checks.addProperty("one_pickup_five_rounds",count==5);
            stand(p,5);
        });
        if(frame==88)onServer(client,p->{
            checks.addProperty("light_meter_charged",p.getAttachedOrCreate(JakData.LIGHT_ECO)>=60);
            var level=p.level();var at=vents[5];
            var hit=new BlockHitResult(Vec3.atCenterOf(at),Direction.UP,at,false);
            level.getBlockState(at).useWithoutItem(level,p,hit);
            checks.addProperty("interacts_and_closes",!level.getBlockState(at).getValue(JakEcoVent.OPEN));
            stand(p,3);p.setHealth(p.getMaxHealth()-4);
        });
        if(frame==108)onServer(client,p->{
            checks.addProperty("green_heals",p.getHealth()==p.getMaxHealth());
            var at=Vec3.atBottomCenterOf(vents[4]).add(0,.2,0);
            JakEcoVent.groundPound(p.level(),p,at);
            JakEcoVent.groundPound(p.level(),p,at);
            checks.addProperty("two_hits_keep_vent",!p.level().getBlockState(vents[4]).getValue(JakEcoVent.BREAKING));
            JakEcoVent.groundPound(p.level(),p,at);
            checks.addProperty("third_hit_closes_vent",p.level().getBlockState(vents[4]).getValue(JakEcoVent.BREAKING));
        });
        if(frame==150)onServer(client,p->{
            checks.addProperty("broken_vent_removed",p.level().isEmptyBlock(vents[4]));
            checks.addProperty("original_particles_rendered",peak>0);
            boolean ok=checks.entrySet().stream().allMatch(x->x.getValue().getAsBoolean());
            var result=new JsonObject();result.add("actions",checks);result.addProperty("particleVertices",peak);
            result.addProperty("reason",ok?"complete":"Eco vent acceptance failed");
            try{
                var dir=client.gameDirectory.toPath().resolve("jakmc/evidence/vents");
                Files.createDirectories(dir);Files.writeString(dir.resolve("result.json"),result.toString());
            }catch(IOException failure){throw new UncheckedIOException(failure);}
            restore.run();
            client.execute(()->{running=false;view=null;client.options.pauseOnLostFocus=pause;client.stop();});
        });
    }
    private static void stand(ServerPlayer p,int index){
        var at=Vec3.atBottomCenterOf(vents[index]);
        p.teleportTo(at.x,at.y+.24,at.z);
    }
    private static void onServer(Minecraft client,java.util.function.Consumer<ServerPlayer> action){
        var server=client.getSingleplayerServer();
        server.execute(()->{var player=server.getPlayerList().getPlayer(owner);if(player!=null)action.accept(player);});
    }
}
