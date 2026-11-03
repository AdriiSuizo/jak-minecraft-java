package dev.jakmc.client;

import com.google.gson.JsonObject;
import dev.jakmc.JakData;
import dev.jakmc.entity.JakVehicle;
import java.io.IOException;
import java.io.UncheckedIOException;
import java.nio.file.Files;
import net.fabricmc.fabric.api.client.event.lifecycle.v1.ClientTickEvents;
import net.minecraft.client.Minecraft;
import net.minecraft.client.Screenshot;
import net.minecraft.core.BlockPos;
import net.minecraft.core.HolderSet;
import net.minecraft.core.registries.Registries;
import net.minecraft.resources.ResourceKey;
import net.minecraft.server.level.ServerPlayer;
import net.minecraft.world.level.GameType;
import net.minecraft.world.level.block.state.properties.BedPart;
import net.minecraft.world.level.block.state.properties.BlockStateProperties;
import net.minecraft.world.level.levelgen.Heightmap;
import net.minecraft.world.phys.AABB;
import net.minecraft.world.phys.Vec3;

final class CityScene {
    private static final JsonObject results=new JsonObject();
    private static volatile BlockPos target;
    private static volatile boolean ready,finished;
    private static int ticks,index;
    private static InputScene.CameraView view;
    private static GameType beforeMode;
    private static Vec3 before;
    private static BlockPos searchFrom;
    private static boolean pause,wasJak;

    static InputScene.CameraView camera(){return view;}
    static void initialize(){
        ClientTickEvents.END_CLIENT_TICK.register(client->{
            if(!ready||finished)return;
            if(++ticks==100){
                Vec3 point=Vec3.atCenterOf(target).add(0,index==0?38:6,0);
                Vec3 eye=point.add(index==0?93:49,index==0?21:36,index==0?105:68);
                Vec3 d=point.subtract(eye);
                view=new InputScene.CameraView(eye,(float)Math.toDegrees(Math.atan2(-d.x,d.z)),
                    (float)-Math.toDegrees(Math.atan2(d.y,d.horizontalDistance())));
            }
            if(ticks==120)Screenshot.grab(client.gameDirectory,index==0?"cities-haven.png":"cities-spargus.png",
                client.gameRenderer.mainRenderTarget(),1,_-> {});
            if(ticks==130){
                ready=false;
                onServer(client,p->inspect(client,p));
            }
        });
    }
    static void start(Minecraft client,Vec3 search){
        if(client.getSingleplayerServer()==null||client.player==null)throw new IllegalStateException("Open the normal development world");
        pause=client.options.pauseOnLostFocus;
        client.options.pauseOnLostFocus=false;
        finished=ready=false;
        ticks=index=0;
        results.entrySet().clear();
        onServer(client,p->{
            before=p.position();
            searchFrom=BlockPos.containing(search==null?before:search);
            beforeMode=p.gameMode.getGameModeForPlayer();
            wasJak=p.getAttachedOrCreate(JakData.ENABLED);
            p.setAttached(JakData.ENABLED,false);
            p.setGameMode(GameType.SPECTATOR);
            locate(client,p);
        });
    }
    private static void locate(Minecraft client,ServerPlayer p){
        String name=index==0?"haven":"spargus";
        var level=p.level();
        var key=ResourceKey.create(Registries.STRUCTURE,JakData.id(name));
        var type=level.registryAccess().lookupOrThrow(Registries.STRUCTURE).getOrThrow(key);
        var pos=level.findNearestMapStructure(HolderSet.direct(type),searchFrom,96,false);
        if(pos==null){
            var failure=new JsonObject();
            failure.addProperty("found",false);
            results.add(name,failure);
            next(client,p);
            return;
        }
        var start=level.getChunk(pos).getStartForStructure(type.value());
        var box=start.getBoundingBox();
        int y=box.minY()+(index==0?42:44);
        target=new BlockPos(pos.getX(),y,pos.getZ());
        p.teleportTo(target.getX()+.5,target.getY()+12,target.getZ()+.5);
        ticks=0;
        ready=true;
    }
    private static void inspect(Minecraft client,ServerPlayer p){
        var report=new JsonObject();
        report.addProperty("found",true);
        var level=p.level();
        var at=new BlockPos.MutableBlockPos();
        int structures=0,roads=0,projectors=0,palms=0,beds=0,brokenBeds=0,altar=0,ramps=0;
        var palette=net.minecraft.core.registries.BuiltInRegistries.BLOCK;
        for(int dx=-98;dx<=98;dx++)for(int dz=-98;dz<=98;dz++){
            int x=target.getX()+dx,z=target.getZ()+dz;
            if(!level.hasChunkAt(at.set(x,target.getY(),z)))continue;
            int surface=level.getHeight(Heightmap.Types.WORLD_SURFACE,x,z);
            for(int y=target.getY()-8;y<=Math.min(surface,target.getY()+22);y++){
                var state=level.getBlockState(at.set(x,y,z));
                if(state.isAir())continue;
                if(state.hasProperty(BlockStateProperties.BED_PART)&&state.getValue(BlockStateProperties.BED_PART)==BedPart.FOOT){
                    beds++;
                    var head=level.getBlockState(at.relative(state.getValue(BlockStateProperties.HORIZONTAL_FACING)));
                    if(!head.is(state.getBlock())||head.getValue(BlockStateProperties.BED_PART)!=BedPart.HEAD)brokenBeds++;
                }
                var id=palette.getKey(state.getBlock());
                if(!id.getNamespace().equals("jak"))continue;
                structures++;
                String path=id.getPath();
                if(path.equals("barrio_losa")||path.equals("sparg_losa"))roads++;
                if(path.equals("objeto_haven_propaganda_bloque"))projectors++;
                if(path.equals("objeto_spargus_palmera_completa"))palms++;
                if(path.equals("altar_turbotabla"))altar++;
                if(path.startsWith("objeto_haven_rampa_cuarto"))ramps++;
            }
        }
        var cars=level.getEntitiesOfClass(JakVehicle.class,new AABB(Vec3.atCenterOf(target),Vec3.atCenterOf(target)).inflate(120),e->true);
        report.addProperty("x",target.getX());report.addProperty("z",target.getZ());
        report.addProperty("jakBlocks",structures);
        report.addProperty("roadBlocks",roads);
        report.addProperty("projectors",projectors);
        report.addProperty("palms",palms);
        report.addProperty("beds",beds);
        report.addProperty("brokenBeds",brokenBeds);
        report.addProperty("altars",altar);
        report.addProperty("rampCells",ramps);
        report.addProperty("cars",cars.size());
        results.add(index==0?"haven":"spargus",report);
        next(client,p);
    }
    private static void next(Minecraft client,ServerPlayer p){
        view=null;
        if(index++==0){locate(client,p);return;}
        var haven=results.getAsJsonObject("haven");
        var spargus=results.getAsJsonObject("spargus");
        boolean ok=haven.has("roadBlocks")&&haven.get("roadBlocks").getAsInt()>100
            &&haven.get("altars").getAsInt()==1&&haven.get("rampCells").getAsInt()>20
            &&spargus.has("roadBlocks")&&spargus.get("roadBlocks").getAsInt()>100&&spargus.get("cars").getAsInt()==2;
        results.addProperty("reason",ok?"complete":"Normal-world city acceptance failed");
        try{
            var path=client.gameDirectory.toPath().resolve("jakmc/evidence/cities");
            Files.createDirectories(path);
            Files.writeString(path.resolve("result.json"),results.toString());
        }catch(IOException e){throw new UncheckedIOException(e);}
        p.setGameMode(beforeMode);
        p.setAttached(JakData.ENABLED,wasJak);
        p.teleportTo(before.x,before.y,before.z);
        finished=true;
        client.execute(()->{
            client.options.pauseOnLostFocus=pause;
            client.stop();
        });
    }
    private static void onServer(Minecraft client,java.util.function.Consumer<ServerPlayer> call){
        var server=client.getSingleplayerServer();
        var uuid=client.player.getUUID();
        server.execute(()->{
            var p=server.getPlayerList().getPlayer(uuid);
            if(p!=null)call.accept(p);
        });
    }
}
