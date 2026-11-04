package dev.jakmc.client;

import com.google.gson.JsonObject;
import dev.jakmc.JakData;
import dev.jakmc.content.JakAssemblies;
import java.io.IOException;
import java.io.UncheckedIOException;
import java.nio.file.Files;
import net.fabricmc.fabric.api.client.event.lifecycle.v1.ClientTickEvents;
import net.minecraft.client.Minecraft;
import net.minecraft.client.Screenshot;
import net.minecraft.core.BlockPos;
import net.minecraft.core.HolderSet;
import net.minecraft.core.registries.BuiltInRegistries;
import net.minecraft.core.registries.Registries;
import net.minecraft.resources.ResourceKey;
import net.minecraft.server.level.ServerPlayer;
import net.minecraft.world.level.GameType;
import net.minecraft.world.level.block.entity.ChestBlockEntity;
import net.minecraft.world.level.levelgen.structure.BoundingBox;
import net.minecraft.world.phys.Vec3;

final class SitesScene {
    private static final String[] SITES={"swamp_hut","snow_fort","haven","spargus"};
    private static final JsonObject result=new JsonObject();
    private static final java.util.Set<String> JAK1=java.util.Set.of("green_eco","vent_blue","vent_yellow","vent_red","vent_green","flut_egg");
    private static volatile boolean active;
    private static int index,ticks;
    private static boolean pause;
    private static BoundingBox bounds;
    private static BlockPos origin;
    private static Runnable restore;
    private static volatile InputScene.CameraView view;
    static InputScene.CameraView camera(){return view;}
    private static java.util.UUID owner;
    static void initialize(){ClientTickEvents.END_CLIENT_TICK.register(SitesScene::tick);}
    static void start(Minecraft client){
        var server=client.getSingleplayerServer();
        owner=client.player.getUUID();
        pause=client.options.pauseOnLostFocus;client.options.pauseOnLostFocus=false;
        result.entrySet().clear();index=ticks=0;active=false;
        server.execute(()->{
            var p=server.getPlayerList().getPlayer(owner);
            if(p==null)return;
            var old=p.position();var mode=p.gameMode.getGameModeForPlayer();
            var enabled=p.getAttachedOrCreate(JakData.ENABLED);
            boolean flying=p.getAbilities().flying;
            origin=p.blockPosition();
            p.setAttached(JakData.ENABLED,false);
            p.setGameMode(GameType.CREATIVE);p.getAbilities().flying=true;p.onUpdateAbilities();
            restore=()->{
                p.teleportTo(old.x,old.y,old.z);
                p.setAttached(JakData.ENABLED,enabled);p.setGameMode(mode);
                p.getAbilities().flying=flying;p.onUpdateAbilities();
            };
            locate(client,p);
        });
    }
    private static void locate(Minecraft client,ServerPlayer p){
        while(index<SITES.length){
            var level=p.level();
            var key=ResourceKey.create(Registries.STRUCTURE,JakData.id(SITES[index]));
            var type=level.registryAccess().lookupOrThrow(Registries.STRUCTURE).getOrThrow(key);
            var point=level.findNearestMapStructure(HolderSet.direct(type),origin,128,false);
            if(point!=null){
                var start=level.getChunk(point).getStartForStructure(type.value());
                bounds=start.getBoundingBox();
                var at=Vec3.atCenterOf(bounds.getCenter());
                double height=index<2?bounds.maxY()+8:Math.max(bounds.minY()+55,level.getSeaLevel()+15);
                p.teleportTo(at.x,height,at.z);
                ticks=0;active=true;
                return;
            }
            var failure=new JsonObject();failure.addProperty("found",false);result.add(SITES[index++],failure);
        }
        finish(client);
    }
    private static void tick(Minecraft client){
        if(!active)return;
        int t=++ticks;
        var server=client.getSingleplayerServer();
        if(t==85)server.execute(()->{
            var p=server.getPlayerList().getPlayer(owner);
            if(p!=null)inspect(p);
        });
        if(t==100&&view!=null)Screenshot.grab(client.gameDirectory,"sites-"+SITES[index]+".png",
            client.gameRenderer.mainRenderTarget(),1,_-> {});
        if(t==104){
            active=false;view=null;
            server.execute(()->{
                var p=server.getPlayerList().getPlayer(owner);
                if(p!=null){index++;locate(client,p);}
            });
        }
    }
    private static void inspect(ServerPlayer player){
        var level=player.level();
        var site=SITES[index];
        String expected=index<2?"jak1":site;
        int chests=0,assigned=0,items=0;
        boolean sourceLoot=true;
        var facadeBlocks=new java.util.HashMap<net.minecraft.world.level.block.Block,String>();
        var facadeCounts=new java.util.TreeMap<String,Integer>();
        String portal=index==2?"barrio_portal_doble":"sparg_portal_doble";
        if(index>=2)JakAssemblies.FACADES.forEach((name,model)->model.parts().forEach(part->facadeBlocks.put(part.block(),name)));
        BlockPos[] doorway={null};
        int[] doors={0};
        for(int x=bounds.minX()>>4;x<=bounds.maxX()>>4;x++)for(int z=bounds.minZ()>>4;z<=bounds.maxZ()>>4;z++){
            var chunk=level.getChunk(x,z,net.minecraft.world.level.chunk.status.ChunkStatus.FULL,false);
            if(!(chunk instanceof net.minecraft.world.level.chunk.LevelChunk loaded))continue;
            if(index>=2)loaded.findBlocks(state->facadeBlocks.containsKey(state.getBlock()),(at,state)->{
                if(!bounds.isInside(at))return;
                String name=facadeBlocks.get(state.getBlock());
                facadeCounts.merge(name,1,Integer::sum);
                if(!name.equals(portal)||!BuiltInRegistries.BLOCK.getKey(state.getBlock()).getPath().equals(name))return;
                var lower=at.below(3);
                if(!(level.getBlockState(lower).getBlock() instanceof net.minecraft.world.level.block.DoorBlock))return;
                doors[0]++;
                if(doorway[0]==null||at.distSqr(bounds.getCenter())<doorway[0].distSqr(bounds.getCenter()))doorway[0]=at.immutable();
            });
            for(var entry:loaded.getBlockEntities().entrySet()){
                if(!(entry.getValue() instanceof ChestBlockEntity chest)||!bounds.isInside(entry.getKey()))continue;
                chests++;
                var table=chest.getLootTable();
                if(table==null||!table.identifier().equals(JakData.id("chests/"+expected)))continue;
                assigned++;
                if(assigned>1)continue;
                chest.unpackLootTable(player);
                for(int slot=0;slot<chest.getContainerSize();slot++){
                    var stack=chest.getItem(slot);
                    if(stack.isEmpty())continue;
                    var id=BuiltInRegistries.ITEM.getKey(stack.getItem());
                    sourceLoot&=id.getNamespace().equals("jakmc")&&(index>=2||JAK1.contains(id.getPath()));
                    items+=stack.getCount();
                }
            }
        }
        var info=new JsonObject();
        info.addProperty("found",true);info.addProperty("x",bounds.getCenter().getX());info.addProperty("z",bounds.getCenter().getZ());
        info.addProperty("chests",chests);info.addProperty("lootTables",assigned);info.addProperty("generatedItems",items);
        info.addProperty("matchesEra",sourceLoot&&items>0);
        if(index>=2){
            var parts=new JsonObject();facadeCounts.forEach(parts::addProperty);
            info.add("facades",parts);info.addProperty("doorsBelowPortal",doors[0]);
            info.addProperty("hasFacades",doors[0]>0&&facadeCounts.size()==(index==2?4:2));
            if(doorway[0]!=null){
                var state=level.getBlockState(doorway[0]);
                var facing=state.getValue(net.minecraft.world.level.block.state.properties.BlockStateProperties.HORIZONTAL_FACING);
                var centre=Vec3.atCenterOf(doorway[0]).add(0,-1.1,0);
                var eye=centre.add(facing.getStepX()*5.4+.6,.8,facing.getStepZ()*5.4+.6);
                var look=centre.subtract(eye);
                view=new InputScene.CameraView(eye,(float)Math.toDegrees(Math.atan2(-look.x,look.z)),
                    (float)-Math.toDegrees(Math.atan2(look.y,look.horizontalDistance())));
            }
        }
        result.add(site,info);
    }
    private static void finish(Minecraft client){
        boolean ok=result.asMap().values().stream().allMatch(row->row.getAsJsonObject().has("matchesEra")
            &&row.getAsJsonObject().get("matchesEra").getAsBoolean()
            &&(!row.getAsJsonObject().has("hasFacades")||row.getAsJsonObject().get("hasFacades").getAsBoolean()));
        result.addProperty("reason",ok?"complete":"Natural site acceptance failed");
        try{
            var dir=client.gameDirectory.toPath().resolve("jakmc/evidence/sites");
            Files.createDirectories(dir);Files.writeString(dir.resolve("result.json"),result.toString());
        }catch(IOException failure){throw new UncheckedIOException(failure);}
        restore.run();
        client.execute(()->{client.options.pauseOnLostFocus=pause;client.stop();});
    }
}
