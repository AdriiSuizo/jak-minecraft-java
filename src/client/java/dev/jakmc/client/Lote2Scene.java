package dev.jakmc.client;

import com.google.gson.JsonArray;
import com.google.gson.JsonObject;
import dev.jakmc.JakData;
import dev.jakmc.content.JakEcoVent;
import dev.jakmc.content.JakIrisDoor;
import dev.jakmc.content.darkeco.DarkEco;
import dev.jakmc.entity.JakEnemy;
import dev.jakmc.entity.JakEntities;
import dev.jakmc.player.JakPlayers;
import java.io.IOException;
import java.io.UncheckedIOException;
import java.nio.file.Files;
import java.util.ArrayList;
import java.util.LinkedHashMap;
import java.util.LinkedHashSet;
import java.util.UUID;
import net.fabricmc.fabric.api.client.event.lifecycle.v1.ClientTickEvents;
import net.minecraft.client.CameraType;
import net.minecraft.client.Minecraft;
import net.minecraft.client.Screenshot;
import net.minecraft.core.BlockPos;
import net.minecraft.core.Direction;
import net.minecraft.core.registries.BuiltInRegistries;
import net.minecraft.resources.Identifier;
import net.minecraft.server.level.ServerPlayer;
import net.minecraft.world.InteractionHand;
import net.minecraft.world.entity.EntitySpawnReason;
import net.minecraft.world.item.Item;
import net.minecraft.world.item.ItemStack;
import net.minecraft.world.item.context.UseOnContext;
import net.minecraft.world.level.GameType;
import net.minecraft.world.level.block.Blocks;
import net.minecraft.world.level.block.state.BlockState;
import net.minecraft.world.level.levelgen.Heightmap;
import net.minecraft.world.phys.BlockHitResult;
import net.minecraft.world.phys.Vec3;

/**
 * Adrii's second 06/11 batch in one bounded run: the Haven foliage placed and in the hand, the icons (light eco,
 * palace panes, metal sheets), the levelled Spargus rock, the airlock opened and shut by click, the iris door with
 * its frame, the launcher on top of its block (no launch standing, a launch jumping), blue eco's ooze and timer,
 * a flat leaf wall (no grab) and a low one (a grab), swimming depth and the swim jump, the frog floating, dark eco.
 */
final class Lote2Scene {
    private static boolean running,pause;
    private static int tick;
    private static UUID owner,frog;
    private static BlockPos base;
    private static InputScene.CameraView view;
    private static Runnable restore;
    private static final JsonObject result=new JsonObject();
    private static final LinkedHashSet<String> standStates=new LinkedHashSet<>(),launchStates=new LinkedHashSet<>(),tallWall=new LinkedHashSet<>(),lowWall=new LinkedHashSet<>(),frogClips=new LinkedHashSet<>();
    private static double standPeak,launchPeak,swimFeet=Double.NaN,swimPeak=-99;
    static InputScene.CameraView camera(){return view;}
    static void initialize(){ClientTickEvents.END_CLIENT_TICK.register(Lote2Scene::tick);}

    private static Item item(String id){return BuiltInRegistries.ITEM.getValue(Identifier.parse(id));}
    private static Vec3 at(double x,double y,double z){return Vec3.atBottomCenterOf(base).add(x,y,z);}
    private static void look(Vec3 eye,Vec3 target){
        var d=target.subtract(eye);
        view=new InputScene.CameraView(eye,(float)Math.toDegrees(Math.atan2(-d.x,d.z)),(float)-Math.toDegrees(Math.atan2(d.y,d.horizontalDistance())));
    }
    private static void capture(Minecraft c,String file){Screenshot.grab(c.gameDirectory,file,c.gameRenderer.mainRenderTarget(),1,_->{});}
    private static void place(ServerPlayer p,String id,int x,int z){
        var cell=base.offset(x,0,z);var stand=at(x,0,z+3);p.teleportTo(stand.x,stand.y,stand.z);p.setYRot(180);p.setXRot(30);
        p.setItemInHand(InteractionHand.MAIN_HAND,new ItemStack(item(id)));
        var hit=new BlockHitResult(Vec3.atBottomCenterOf(cell),Direction.UP,cell.below(),false);
        result.getAsJsonObject("placed").addProperty(id,p.getMainHandItem().getItem().useOn(new UseOnContext(p,InteractionHand.MAIN_HAND,hit)).consumesAction());
    }

    static void start(Minecraft client){
        if(client.player==null||client.getSingleplayerServer()==null)throw new IllegalStateException("Open the development world");
        owner=client.player.getUUID();tick=0;result.entrySet().clear();result.add("placed",new JsonObject());
        standStates.clear();launchStates.clear();tallWall.clear();lowWall.clear();frogClips.clear();standPeak=launchPeak=0;swimFeet=Double.NaN;swimPeak=-99;
        pause=client.options.pauseOnLostFocus;client.options.pauseOnLostFocus=false;
        client.options.setCameraType(CameraType.THIRD_PERSON_BACK);
        onServer(client,p->{
            var server=p.level().getServer();
            server.getCommands().performPrefixedCommand(server.createCommandSourceStack(),"time set noon");
            server.getCommands().performPrefixedCommand(server.createCommandSourceStack(),"weather clear");
            var level=p.level();
            var old=p.position();var mode=p.gameMode.getGameModeForPlayer();
            boolean enabled=p.getAttachedOrCreate(JakData.ENABLED);
            var saved=new ArrayList<ItemStack>();
            for(int i=0;i<p.getInventory().getContainerSize();i++)saved.add(p.getInventory().getItem(i).copy());
            var near=p.blockPosition().offset(24,0,0);
            int y=near.getY();
            for(int x=-18;x<=18;x++)for(int z=-14;z<=14;z++)y=Math.max(y,level.getHeight(Heightmap.Types.MOTION_BLOCKING_NO_LEAVES,near.getX()+x,near.getZ()+z));
            base=near.atY(y+41);
            var snapshot=new LinkedHashMap<BlockPos,BlockState>();
            for(int x=-18;x<=18;x++)for(int z=-14;z<=14;z++)for(int h=-5;h<=8;h++){
                var cell=base.offset(x,h,z);
                snapshot.put(cell.immutable(),level.getBlockState(cell));
                // A pool three deep for swimming, a little pool of dark eco, stone round them.
                boolean pool=x>=8&&x<=16&&z>=3&&z<=11&&h>=-4,dark=x>=0&&x<=2&&z>=6&&z<=8&&h==-1;
                level.setBlock(cell,h==-5||h<0&&!pool&&!dark?Blocks.SMOOTH_STONE.defaultBlockState():pool&&h<0?Blocks.WATER.defaultBlockState():dark?DarkEco.BLOCK.defaultBlockState():Blocks.AIR.defaultBlockState(),2);
            }
            // A flat leaf wall five high (nothing to grab on its face) and a low one two high (its top edge).
            for(int x=-16;x<=-12;x++)for(int h=0;h<5;h++)level.setBlock(base.offset(x,h,4),Blocks.OAK_LEAVES.defaultBlockState().setValue(net.minecraft.world.level.block.LeavesBlock.PERSISTENT,true),2);
            for(int x=-8;x<=-4;x++)for(int h=0;h<2;h++)level.setBlock(base.offset(x,h,4),Blocks.OAK_LEAVES.defaultBlockState().setValue(net.minecraft.world.level.block.LeavesBlock.PERSISTENT,true),2);
            p.setGameMode(GameType.CREATIVE);p.setAttached(JakData.ENABLED,true);
            p.getInventory().clearContent();
            place(p,"jak:pieza_haven_follaje",-15,-6);
            place(p,"jak:objeto_spargus_roca_1",-12,-11);
            place(p,"jakmc:airlock_door",-6,-6);
            place(p,"jakmc:iris_door",0,-6);
            place(p,"jakmc:eco_launcher",5,-6);
            var vent=base.offset(9,0,-3);
            p.setItemInHand(InteractionHand.MAIN_HAND,new ItemStack(JakEcoVent.Color.BLUE.item));
            result.getAsJsonObject("placed").addProperty("blue_vent",p.getMainHandItem().getItem().useOn(new UseOnContext(p,InteractionHand.MAIN_HAND,
                new BlockHitResult(Vec3.atBottomCenterOf(vent),Direction.UP,vent.below(),false))).consumesAction());
            result.addProperty("small_rocks_registered",BuiltInRegistries.BLOCK.containsKey(Identifier.parse("jak:sp_waspala-small-rocks")));
            String[] bar={"jakmc:light_eco","jak:sp_waspala-window-frame_pane","jak:sp_waspala-metal-plate01_pane","jak:pieza_haven_follaje","jak:objeto_spargus_roca_1",
                "jakmc:airlock_door","jakmc:iris_door","jakmc:eco_launcher","jakmc:dark_eco_bucket"};
            for(int i=0;i<bar.length;i++)p.getInventory().setItem(i,new ItemStack(item(bar[i])));
            p.getInventory().setSelectedSlot(3);
            var home=at(-15,0,-2);p.teleportTo(home.x,home.y,home.z);p.setYRot(180);p.setXRot(0);
            JakPlayers.controlForProbe(p,true);JakPlayers.hostControlsForProbe(p);
            restore=()->{
                if(frog!=null){var e=level.getEntity(frog);if(e!=null)e.discard();}
                snapshot.forEach((cell,state)->level.setBlock(cell,state,2));
                p.getInventory().clearContent();
                for(int i=0;i<saved.size();i++)p.getInventory().setItem(i,saved.get(i));
                JakPlayers.controlForProbe(p,false);
                p.setAttached(JakData.ENABLED,enabled);
                p.setGameMode(mode);p.teleportTo(old.x,old.y,old.z);
            };
            client.execute(()->{look(at(-4,4.5,1.5),at(-4,.8,-6));running=true;});
        });
    }

    private static void tick(Minecraft c){
        if(!running)return;
        int t=++tick;
        c.gui.setScreen(null);
        var pose=JakPoses.get(c.player.getId());
        String state=pose==null?"":pose.raw().get("state").getAsString();
        String clip=pose==null?"":pose.clip();
        double feet=pose==null?0:pose.position().y-at(0,0,0).y;
        // The row: foliage, levelled rock, airlock shut, iris door, launcher on its block.
        if(t==24)capture(c,"lote2-row.png");
        // The hotbar's icons and the foliage in his hand.
        if(t==26){c.player.getInventory().setSelectedSlot(3);look(at(-15,2.2,.5),at(-15,1.2,-2));}
        if(t==32)capture(c,"lote2-hand.png");
        // The Spargus rock, levelled and closed underneath.
        if(t==33)look(at(-12,3.5,-5.5),at(-12,.5,-11));
        if(t==38)capture(c,"lote2-rock.png");
        // The airlock: a left click opens it, another shuts it.
        if(t==36)onServer(c,p->{var s=at(-6,0,-3.2);p.teleportTo(s.x,s.y,s.z);p.setYRot(180);p.setXRot(10);});
        if(t==40){view=null;c.player.setYRot(180);c.player.setXRot(10);}
        if(t==44){c.gameMode.startDestroyBlock(base.offset(-6,1,-6),Direction.SOUTH);c.gameMode.stopDestroyBlock();}
        if(t==48)look(at(-2,2.5,-1),at(-6,1.5,-6));
        if(t==54)capture(c,"lote2-airlock-opening.png");
        if(t==100){capture(c,"lote2-airlock-open.png");onServer(c,p->result.addProperty("airlock_opened_by_click",p.level().getBlockState(base.offset(-6,0,-6)).getValue(JakIrisDoor.OPEN)));}
        if(t==104){view=null;c.player.setYRot(180);c.player.setXRot(10);}
        if(t==110){c.gameMode.startDestroyBlock(base.offset(-6,1,-6),Direction.SOUTH);c.gameMode.stopDestroyBlock();}
        if(t==114)look(at(-2,2.5,-1),at(-6,1.5,-6));
        if(t==170){capture(c,"lote2-airlock-shut.png");onServer(c,p->result.addProperty("airlock_shut_by_click",!p.level().getBlockState(base.offset(-6,0,-6)).getValue(JakIrisDoor.OPEN)));}
        // Blue eco from its vent, then the iris door opening with its frame.
        if(t==172)onServer(c,p->{var s=at(9,.24,-3);p.teleportTo(s.x,s.y,s.z);});
        if(t==190){onServer(c,p->{result.addProperty("blue_eco",JakPlayers.blueEco(p));var s=at(0,0,-3);p.teleportTo(s.x,s.y,s.z);p.setYRot(180);});look(at(2.5,2.4,-1),at(0,1.5,-6));}
        if(t==196)capture(c,"lote2-iris-opening.png");
        if(t==214){capture(c,"lote2-iris-open.png");onServer(c,p->result.addProperty("iris_opened",p.level().getBlockState(base.offset(0,0,-6)).getValue(JakIrisDoor.OPEN)));}
        // His ooze and Jak 1's timer while he carries it.
        if(t==216)look(at(1.2,1.8,-1.4),at(0,1,-3));
        if(t==222)capture(c,"lote2-blue-eco.png");
        if(t==224)result.addProperty("eco_timer",pose!=null&&pose.raw().has("ecoTimer")&&!pose.raw().get("ecoTimer").isJsonNull());
        // The launcher: standing on it does nothing; a jump on it launches him.
        if(t==226)onServer(c,p->{JakPlayers.probeInput(p,0,0,0);var s=at(9,.24,-3);p.teleportTo(s.x,s.y,s.z);});
        if(t==236){onServer(c,p->{var s=at(5,dev.jakmc.content.JakEcoLauncher.TOP+.02,-6);p.teleportTo(s.x,s.y,s.z);p.setYRot(180);});look(at(9,2.5,-2),at(5,.8,-6));}
        if(t==246)capture(c,"lote2-launcher.png");
        if(t>=240&&t<290){if(!state.isEmpty())standStates.add(state+"/"+clip);standPeak=Math.max(standPeak,feet);}
        if(t==290)input(c,1,1);
        if(t==293)input(c,0,0);
        if(t>=290&&t<350){if(!state.isEmpty())launchStates.add(state+"/"+clip);launchPeak=Math.max(launchPeak,feet);}
        // The flat leaf wall: running and jumping at its face, he must not hang on it.
        if(t==352){view=null;onServer(c,p->{var s=at(-14,0,8);p.teleportTo(s.x,s.y,s.z);p.setYRot(180);p.setXRot(0);});}
        if(t==356){c.player.setYRot(180);onServer(c,p->JakPlayers.probeInput(p,0,0,1));}
        if(t==362)onServer(c,p->JakPlayers.probeInput(p,1,1,1));
        if(t==366)onServer(c,p->JakPlayers.probeInput(p,0,0,1));
        if(t>=356&&t<400&&!state.isEmpty())tallWall.add(state+"/"+clip);
        if(t==380)look(at(-10,2.5,9),at(-14,1.5,4));
        if(t==384)capture(c,"lote2-tall-wall.png");
        if(t==400){input(c,0,0);view=null;onServer(c,p->{var s=at(-6,0,8);p.teleportTo(s.x,s.y,s.z);p.setYRot(180);p.setXRot(0);});}
        if(t==404){c.player.setYRot(180);onServer(c,p->JakPlayers.probeInput(p,0,0,1));}
        if(t==410)onServer(c,p->JakPlayers.probeInput(p,1,1,1));
        if(t==414)onServer(c,p->JakPlayers.probeInput(p,0,0,1));
        if(t>=404&&t<450&&!state.isEmpty())lowWall.add(state+"/"+clip);
        if(t==426)look(at(-2,2.5,9),at(-6,1.5,4));
        if(t==430)capture(c,"lote2-low-wall.png");
        // Swimming: how deep he floats, the swim jump, the frog beside him.
        if(t==450){input(c,0,0);view=null;onServer(c,p->{
            var s=at(12,-.6,7);p.teleportTo(s.x,s.y,s.z);p.setYRot(0);
            var e=JakEntities.TYPES.get("kermit").create(p.level(),EntitySpawnReason.COMMAND);
            if(e!=null){var f=at(14,-.6,9);e.setPos(f);p.level().addFreshEntity(e);frog=e.getUUID();}
        });}
        if(t>=470&&t<500){if(!Double.isNaN(feet))swimFeet=Double.isNaN(swimFeet)?feet:Math.min(swimFeet,feet);}
        if(t>=470&&t<510)onServer(c,p->{if(frog!=null&&p.level().getEntity(frog) instanceof JakEnemy e)frogClips.add(e.animation());});
        if(t==494)look(at(12,0.6,3.5),at(12.5,-.4,8));
        if(t==500)capture(c,"lote2-swim.png");
        if(t==510)input(c,1,1);
        if(t==513)input(c,0,0);
        if(t>=510&&t<560)swimPeak=Math.max(swimPeak,feet);
        // The dark eco pool.
        if(t==566)look(at(1,2.5,3.5),at(1,-.5,7));
        if(t==572)capture(c,"lote2-dark-eco.png");
        if(t==580){
            view=null;
            result.add("stand_states",strings(standStates));result.addProperty("stand_peak_blocks",standPeak);
            result.add("launch_states",strings(launchStates));result.addProperty("launch_peak_blocks",launchPeak);
            result.add("tall_wall_states",strings(tallWall));result.add("low_wall_states",strings(lowWall));
            // The water's surface is 0.11 under the floor's top (a source block's fluid height).
            result.addProperty("swim_feet_under_surface",Double.isNaN(swimFeet)?-99:-.11-swimFeet);
            result.addProperty("swim_jump_feet_over_surface",swimPeak+.11);
            result.add("frog_clips",strings(frogClips));
            result.addProperty("visual_review","required");result.addProperty("reason","complete");
            onServer(c,p->{
                try{
                    var dir=c.gameDirectory.toPath().resolve("jakmc/evidence/lote2");
                    Files.createDirectories(dir);Files.writeString(dir.resolve("result.json"),result.toString());
                }catch(IOException failure){throw new UncheckedIOException(failure);}
                restore.run();
                c.execute(()->{running=false;view=null;c.options.pauseOnLostFocus=pause;c.stop();});
            });
        }
    }
    private static JsonArray strings(java.util.Collection<String> values){var out=new JsonArray();values.forEach(out::add);return out;}
    private static void input(Minecraft c,int held,int pressed){onServer(c,p->JakPlayers.probeInput(p,held,pressed,0));}
    private static void onServer(Minecraft client,java.util.function.Consumer<ServerPlayer> action){
        var server=client.getSingleplayerServer();
        server.execute(()->{var player=server.getPlayerList().getPlayer(owner);if(player!=null)action.accept(player);});
    }
}
