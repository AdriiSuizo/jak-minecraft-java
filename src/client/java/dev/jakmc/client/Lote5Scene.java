package dev.jakmc.client;

import com.google.gson.JsonObject;
import dev.jakmc.JakData;
import dev.jakmc.content.JakItems;
import dev.jakmc.player.JakPlayers;
import java.io.IOException;
import java.io.UncheckedIOException;
import java.nio.file.Files;
import java.util.ArrayList;
import java.util.LinkedHashMap;
import java.util.UUID;
import net.fabricmc.fabric.api.client.event.lifecycle.v1.ClientTickEvents;
import net.minecraft.client.CameraType;
import net.minecraft.client.Minecraft;
import net.minecraft.client.Screenshot;
import net.minecraft.core.BlockPos;
import net.minecraft.server.level.ServerLevel;
import net.minecraft.server.level.ServerPlayer;
import net.minecraft.world.entity.item.ItemEntity;
import net.minecraft.world.item.ItemStack;
import net.minecraft.world.level.GameType;
import net.minecraft.world.level.block.Blocks;
import net.minecraft.world.level.block.state.BlockState;
import net.minecraft.world.level.levelgen.Heightmap;
import net.minecraft.world.phys.Vec3;

/**
 * Adrii's 06/11 night batch (3): the four ammunitions in the hotbar and on the ground, and Jak's spin kick in
 * a field of grass (the plants it goes through are cut, those out of its reach stay).
 */
final class Lote5Scene {
    private static boolean running,pause;
    private static int tick;
    private static UUID owner;
    private static BlockPos base;
    private static InputScene.CameraView view;
    private static Runnable restore;
    private static final JsonObject result=new JsonObject();
    private static final int CIRCLE=4,SQUARE=2;
    private static BlockPos haven;
    private record Seen(BlockPos at,Vec3 facing){}
    private static Seen crateSeen;
    private static boolean houseSeen;
    static InputScene.CameraView camera(){return view;}
    static void initialize(){ClientTickEvents.END_CLIENT_TICK.register(Lote5Scene::tick);}

    private static Vec3 at(double x,double y,double z){return Vec3.atBottomCenterOf(base).add(x,y,z);}
    private static void look(Vec3 eye,Vec3 target){
        var d=target.subtract(eye);
        view=new InputScene.CameraView(eye,(float)Math.toDegrees(Math.atan2(-d.x,d.z)),(float)-Math.toDegrees(Math.atan2(d.y,d.horizontalDistance())));
    }
    private static void capture(Minecraft c,String file){Screenshot.grab(c.gameDirectory,file,c.gameRenderer.mainRenderTarget(),1,_->{});}
    private static void put(ServerPlayer p,double x,double y,double z,float yaw){var s=at(x,y,z);p.teleportTo(s.x,s.y,s.z);p.setYRot(yaw);p.setXRot(0);p.setDeltaMovement(Vec3.ZERO);}

    static void start(Minecraft client){
        if(client.player==null||client.getSingleplayerServer()==null)throw new IllegalStateException("Open the development world");
        owner=client.player.getUUID();tick=0;result.entrySet().clear();haven=null;crateSeen=null;houseSeen=false;
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
            for(int x=-12;x<=12;x++)for(int z=-12;z<=12;z++)y=Math.max(y,level.getHeight(Heightmap.Types.MOTION_BLOCKING_NO_LEAVES,near.getX()+x,near.getZ()+z));
            base=near.atY(y+41);
            var snapshot=new LinkedHashMap<BlockPos,BlockState>();
            // A grass field with its plants (short grass, tall grass, a few flowers), its middle bare where he stands.
            for(int x=-12;x<=12;x++)for(int z=-12;z<=12;z++)for(int h=-2;h<=6;h++){
                var cell=base.offset(x,h,z);
                snapshot.put(cell.immutable(),level.getBlockState(cell));
                BlockState state=Blocks.AIR.defaultBlockState();
                if(h==-1)state=Blocks.GRASS_BLOCK.defaultBlockState();
                if(h==0&&x>=-5&&x<=5&&z>=-5&&z<=5&&(x!=0||z!=0))state=(x+z&3)==0?Blocks.POPPY.defaultBlockState():Blocks.SHORT_GRASS.defaultBlockState();
                level.setBlock(cell,state,2);
            }
            // Two crates beside where he stands, for the kick; one in front, later, for a punch.
            level.setBlock(base.offset(1,0,0),dev.jakmc.content.JakAmmoCrate.BLOCK.defaultBlockState(),2);
            level.setBlock(base.offset(-1,0,0),dev.jakmc.content.JakAmmoCrate.BLOCK.defaultBlockState().setValue(dev.jakmc.content.JakAmmoCrate.CONTENTS,dev.jakmc.content.JakAmmoCrate.Contents.RED),2);
            result.addProperty("plants_before",plants(level));
            p.setGameMode(GameType.SURVIVAL);p.setAttached(JakData.ENABLED,true);
            p.getInventory().clearContent();
            for(int i=0;i<JakItems.AMMUNITION.size();i++)p.getInventory().setItem(i,new ItemStack(JakItems.AMMUNITION.get(i),64));
            p.getInventory().setItem(4,new ItemStack(dev.jakmc.content.JakAmmoCrate.ITEM,8));
            p.getInventory().setSelectedSlot(5);
            put(p,0,0,0,180);
            JakPlayers.controlForProbe(p,true);JakPlayers.hostControlsForProbe(p);
            restore=()->{
                for(var item:level.getEntitiesOfClass(ItemEntity.class,new net.minecraft.world.phys.AABB(base).inflate(14)))item.discard();
                snapshot.forEach((cell,state)->level.setBlock(cell,state,2));
                p.getInventory().clearContent();
                for(int k=0;k<saved.size();k++)p.getInventory().setItem(k,saved.get(k));
                JakPlayers.controlForProbe(p,false);
                p.setAttached(JakData.ENABLED,enabled);
                p.setGameMode(mode);p.teleportTo(old.x,old.y,old.z);
            };
            client.execute(()->running=true);
        });
    }
    /** Plants left in the field: all, and those within 1.5 blocks of where he stands (the kick's reach). */
    private static String plants(ServerLevel level){
        int all=0,near=0;
        for(int x=-5;x<=5;x++)for(int z=-5;z<=5;z++){
            var s=level.getBlockState(base.offset(x,0,z));
            if(s.is(Blocks.SHORT_GRASS)||s.is(Blocks.POPPY)){all++;if(Math.hypot(x,z)<=1.5)near++;}
        }
        return all+" in all, "+near+" within 1.5";
    }

    private static int crates(ServerLevel level){
        int n=0;for(var at:BlockPos.betweenClosed(base.offset(-3,0,-3),base.offset(3,1,3)))if(level.getBlockState(at).is(dev.jakmc.content.JakAmmoCrate.BLOCK))n++;return n;
    }
    /** The ammunition lying in the field: each colour's count. */
    private static String dropped(ServerLevel level){
        var counts=new java.util.TreeMap<String,Integer>();
        for(var item:level.getEntitiesOfClass(ItemEntity.class,new net.minecraft.world.phys.AABB(base).inflate(6)))
            counts.merge(net.minecraft.core.registries.BuiltInRegistries.ITEM.getKey(item.getItem().getItem()).getPath(),item.getItem().getCount(),Integer::sum);
        return counts.toString();
    }
    /** Haven's crates in the loaded town round him: how many, on what floor, roofed or not. */
    private static void havenCrates(ServerLevel level){
        var floors=new java.util.TreeMap<String,Integer>();var insideFloors=new java.util.TreeMap<String,Integer>();int found=0,roofed=0,chunks=0;var sample=new StringBuilder();
        var centre=new net.minecraft.world.level.ChunkPos(haven.getX()>>4,haven.getZ()>>4);
        for(int cx=-10;cx<=10;cx++)for(int cz=-10;cz<=10;cz++){
            int x=centre.x()+cx,z=centre.z()+cz;
            if(!(level.getChunkSource().getChunkNow(x,z) instanceof net.minecraft.world.level.chunk.LevelChunk chunk))continue;
            chunks++;
            for(int bx=0;bx<16;bx++)for(int bz=0;bz<16;bz++)for(int y=haven.getY()-50;y<haven.getY()+140;y++){
                var at=new BlockPos(chunk.getPos().getMinBlockX()+bx,y,chunk.getPos().getMinBlockZ()+bz);
                var here=chunk.getBlockState(at);
                if(here.isAir())continue;
                String id=net.minecraft.core.registries.BuiltInRegistries.BLOCK.getKey(here.getBlock()).getPath();
                if(id.contains("suelo"))insideFloors.merge(id,1,Integer::sum);
                if(!here.is(dev.jakmc.content.JakAmmoCrate.BLOCK))continue;
                found++;
                floors.merge(net.minecraft.core.registries.BuiltInRegistries.BLOCK.getKey(level.getBlockState(at.below()).getBlock()).getPath(),1,Integer::sum);
                boolean roof=false;for(int h=2;h<=4;h++)roof|=!level.getBlockState(at.above(h)).isAir();
                if(roof)roofed++;
                if(sample.length()<400)sample.append(at.subtract(haven).toShortString()).append(' ');
                // A crate to look at: from the side that is open.
                boolean house=level.getBlockState(at.below()).getBlock().getDescriptionId().endsWith("barrio_suelo");
                if(crateSeen==null||house&&!houseSeen)for(var side:net.minecraft.core.Direction.Plane.HORIZONTAL){
                    var open=at.relative(side,2);
                    if(level.isEmptyBlock(open)&&level.isEmptyBlock(open.above())&&level.isEmptyBlock(at.relative(side))){crateSeen=new Seen(at.immutable(),new Vec3(side.getStepX(),0,side.getStepZ()));houseSeen=house;break;}
                }
            }
        }
        result.addProperty("haven_loaded_chunks",chunks);result.addProperty("haven_crates",found);result.addProperty("haven_crates_roofed",roofed);
        result.addProperty("haven_crate_floors",floors.toString());result.addProperty("haven_floor_blocks",insideFloors.toString());result.addProperty("haven_crate_sample",sample.toString());
    }

    private static void tick(Minecraft c){
        if(!running)return;
        int t=++tick;
        c.gui.setScreen(null);
        var pose=JakPoses.get(c.player.getId());
        String state=pose==null?"":pose.raw().get("state").getAsString();
        // The ammunitions in the hotbar.
        if(t==30)capture(c,"lote5-hotbar.png");
        // On the ground, close to.
        if(t==32)onServer(c,p->{
            for(int i=0;i<JakItems.AMMUNITION.size();i++){
                var s=at(-1.5+i,0,-4);var item=new ItemEntity(p.level(),s.x,s.y+.1,s.z,new ItemStack(JakItems.AMMUNITION.get(i)));
                item.setDeltaMovement(Vec3.ZERO);item.setNeverPickUp();item.setUnlimitedLifetime();p.level().addFreshEntity(item);
            }
        });
        if(t==40)look(at(0,1.3,-1.8),at(0,.2,-4));
        if(t==60)capture(c,"lote5-ammo-ground.png");
        if(t==62)look(at(0,.6,-2.6),at(0,.15,-4));
        if(t==70)capture(c,"lote5-ammo-close.png");
        // The spin kick in the grass.
        if(t==80){view=null;look(at(3.5,2.6,3.5),at(0,.5,0));}
        if(t==90)capture(c,"lote5-kick-before.png");
        if(t==92)onServer(c,p->JakPlayers.probeInput(p,CIRCLE,CIRCLE,0));
        if(t==94)onServer(c,p->JakPlayers.probeInput(p,0,0,0));
        if(t>=92&&t<130&&!state.isEmpty()){var seen=result.has("kick_states")?result.get("kick_states").getAsString():"";if(!seen.endsWith(state))result.addProperty("kick_states",seen+" "+state);}
        if(t==97)capture(c,"lote5-kick-during.png");
        if(t==130)capture(c,"lote5-kick-after.png");
        if(t==132)onServer(c,p->{result.addProperty("plants_after_kick",plants(p.level()));result.addProperty("crates_after_kick",crates(p.level()));result.addProperty("ammo_dropped_after_kick",dropped(p.level()));
            p.level().setBlock(base.offset(0,0,-1),dev.jakmc.content.JakAmmoCrate.BLOCK.defaultBlockState(),2);});
        if(t==136)capture(c,"lote5-crate.png");
        if(t==140)onServer(c,p->JakPlayers.probeInput(p,SQUARE,SQUARE,0));
        if(t==142)onServer(c,p->JakPlayers.probeInput(p,0,0,0));
        if(t>=140&&t<170&&!state.isEmpty()){var seen=result.has("punch_states")?result.get("punch_states").getAsString():"";if(!seen.endsWith(state))result.addProperty("punch_states",seen+" "+state);}
        if(t==170)onServer(c,p->{result.addProperty("crates_after_punch",crates(p.level()));result.addProperty("ammo_dropped_after_punch",dropped(p.level()));});
        // Haven: its crates as the town generates round him.
        if(t==172)onServer(c,p->{
            p.setGameMode(GameType.CREATIVE);JakPlayers.probeInput(p,0,0,0);
            var level=p.level();
            var key=net.minecraft.resources.ResourceKey.create(net.minecraft.core.registries.Registries.STRUCTURE,dev.jakmc.JakData.id("haven"));
            var type=level.registryAccess().lookupOrThrow(net.minecraft.core.registries.Registries.STRUCTURE).getOrThrow(key);
            var pos=level.findNearestMapStructure(net.minecraft.core.HolderSet.direct(type),p.blockPosition(),96,false);
            if(pos==null){result.addProperty("haven","not found");return;}
            var box=level.getChunk(pos).getStartForStructure(type.value()).getBoundingBox();
            haven=new BlockPos(pos.getX(),box.minY()+42,pos.getZ());
            p.teleportTo(haven.getX()+.5,haven.getY()+30,haven.getZ()+.5);
        });
        if(t==600)onServer(c,p->havenCrates(p.level()));
        if(t==610&&crateSeen!=null){var d=crateSeen.facing;look(Vec3.atBottomCenterOf(crateSeen.at).add(d.x*2.2,1.4,d.z*2.2),Vec3.atBottomCenterOf(crateSeen.at).add(0,.3,0));}
        if(t==620&&crateSeen!=null)capture(c,"lote5-haven-crate.png");
        if(t==624){
            view=null;
            result.addProperty("visual_review","required");result.addProperty("reason","complete");
            onServer(c,p->{
                try{
                    var dir=c.gameDirectory.toPath().resolve("jakmc/evidence/lote5");
                    Files.createDirectories(dir);Files.writeString(dir.resolve("result.json"),result.toString());
                }catch(IOException failure){throw new UncheckedIOException(failure);}
                restore.run();
                c.execute(()->{running=false;view=null;c.options.pauseOnLostFocus=pause;c.stop();});
            });
        }
    }
    private static void onServer(Minecraft client,java.util.function.Consumer<ServerPlayer> action){
        var server=client.getSingleplayerServer();
        server.execute(()->{var player=server.getPlayerList().getPlayer(owner);if(player!=null)action.accept(player);});
    }
}
