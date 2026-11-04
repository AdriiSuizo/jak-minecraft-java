package dev.jakmc.client;

import com.google.gson.JsonObject;
import dev.jakmc.JakData;
import dev.jakmc.entity.JakEnemySounds;
import dev.jakmc.entity.JakYakow;
import java.io.IOException;
import java.io.UncheckedIOException;
import java.nio.file.Files;
import java.util.HashSet;
import java.util.LinkedHashMap;
import java.util.UUID;
import java.util.concurrent.atomic.AtomicBoolean;
import net.fabricmc.fabric.api.client.event.lifecycle.v1.ClientTickEvents;
import net.minecraft.client.CameraType;
import net.minecraft.client.Minecraft;
import net.minecraft.client.Screenshot;
import net.minecraft.core.BlockPos;
import net.minecraft.core.Direction;
import net.minecraft.world.InteractionHand;
import net.minecraft.world.entity.item.ItemEntity;
import net.minecraft.world.item.ItemStack;
import net.minecraft.world.item.Items;
import net.minecraft.world.item.context.UseOnContext;
import net.minecraft.world.level.block.Blocks;
import net.minecraft.world.level.block.state.BlockState;
import net.minecraft.world.level.levelgen.Heightmap;
import net.minecraft.world.phys.BlockHitResult;
import net.minecraft.world.phys.Vec3;

final class YakowScene {
    private static YakowScene active;
    private final JsonObject result=new JsonObject();
    private final com.google.gson.JsonArray trace=new com.google.gson.JsonArray();
    private final int animal;
    private final Vec3 origin;
    private final Runnable teleport,tempt,hit,kill,cleanup;
    private final AtomicBoolean nativeLoot;
    private final long cues;
    private double initialDistance=Double.NaN;
    private int settledTicks;
    private int ticks;
    private InputScene.CameraView camera;

    private YakowScene(int animal,Vec3 origin,Runnable teleport,Runnable tempt,Runnable hit,Runnable kill,
                       Runnable cleanup,AtomicBoolean nativeLoot){
        this.animal=animal;this.origin=origin;this.teleport=teleport;this.tempt=tempt;this.hit=hit;
        this.kill=kill;this.cleanup=cleanup;this.nativeLoot=nativeLoot;
        cues=JakEnemySounds.cuesPlayed;
    }

    static InputScene.CameraView camera(){return active==null?null:active.camera;}
    static void initialize(){ClientTickEvents.END_CLIENT_TICK.register(c->{if(active!=null)active.tick(c);});}

    static void start(Minecraft client){
        var server=client.getSingleplayerServer();
        var uuid=client.player.getUUID();
        var perspective=client.options.getCameraType();
        client.options.setCameraType(CameraType.THIRD_PERSON_BACK);
        server.execute(()->{
            var p=server.getPlayerList().getPlayer(uuid);
            var level=p.level();
            Vec3 old=p.position();
            boolean enabled=p.getAttachedOrCreate(JakData.ENABLED);
            var hand=p.getMainHandItem().copy();
            var at=p.blockPosition();
            int y=at.getY();
            for(int x=-15;x<=17;x++)for(int z=-13;z<=25;z++)
                y=Math.max(y,level.getHeight(Heightmap.Types.MOTION_BLOCKING,at.getX()+x,at.getZ()+z));
            var ground=at.atY(y);
            var snapshot=new LinkedHashMap<BlockPos,BlockState>();
            for(int x=-15;x<=17;x++)for(int z=-13;z<=25;z++){
                var floor=ground.offset(x,0,z);
                snapshot.put(floor,level.getBlockState(floor));
                level.setBlockAndUpdate(floor,Blocks.GRASS_BLOCK.defaultBlockState());
            }
            var where=Vec3.atBottomCenterOf(ground.above());
            server.getCommands().performPrefixedCommand(server.createCommandSourceStack(),"time set noon");
            p.teleportTo(where.x,where.y,where.z-4);
            p.setAttached(JakData.ENABLED,true);
            p.setItemInHand(InteractionHand.MAIN_HAND,new ItemStack(JakYakow.EGG));
            var click=new BlockHitResult(where,Direction.UP,ground,false);
            var spawn=JakYakow.EGG.useOn(new UseOnContext(p,InteractionHand.MAIN_HAND,click));
            p.setItemInHand(InteractionHand.MAIN_HAND,ItemStack.EMPTY);
            var yak=level.getEntitiesOfClass(JakYakow.class,p.getBoundingBox().inflate(12),
                entity->entity.distanceToSqr(where)<4).stream().findFirst().orElseThrow();
            yak.setYRot(0);yak.setYBodyRot(0);yak.setYHeadRot(0);
            var nativeLoot=new AtomicBoolean();
            var spawnedLoot=new HashSet<UUID>();
            var scene=new YakowScene(yak.getId(),where,()->server.execute(()->p.teleportTo(where.x+1,where.y,where.z-4)),
                ()->server.execute(()->{
                Vec3 side=yak.position().add(yak.getX()>where.x?-5:5,0,0);
                p.teleportTo(side.x,side.y,side.z);
                p.setItemInHand(InteractionHand.MAIN_HAND,new ItemStack(Items.WHEAT,4));
            }),()->server.execute(()->yak.hurtServer(level,p.damageSources().playerAttack(p),2.5f)),
                ()->server.execute(()->{
                    var bounds=yak.getBoundingBox().inflate(3);
                    var previous=new HashSet<UUID>();
                    for(var drop:level.getEntitiesOfClass(ItemEntity.class,bounds))previous.add(drop.getUUID());
                    yak.hurtServer(level,p.damageSources().playerAttack(p),100);
                    for(var drop:level.getEntitiesOfClass(ItemEntity.class,bounds)){
                        if(previous.contains(drop.getUUID()))continue;
                        spawnedLoot.add(drop.getUUID());
                        if(drop.getItem().is(Items.BEEF))nativeLoot.set(true);
                    }
                }),()->{
                    client.options.setCameraType(perspective);
                    server.execute(()->{
                        yak.discard();
                        for(UUID id:spawnedLoot)if(level.getEntity(id) instanceof ItemEntity drop)drop.discard();
                        snapshot.forEach(level::setBlockAndUpdate);
                        p.setItemInHand(InteractionHand.MAIN_HAND,hand);
                        p.setAttached(JakData.ENABLED,enabled);
                        p.teleportTo(old.x,old.y,old.z);
                    });
                },nativeLoot);
            client.execute(()->{
                scene.result.addProperty("native_egg",spawn.consumesAction());
                active=scene;
            });
        });
    }

    private void tick(Minecraft client){
        ticks++;
        if(client.level.getEntity(animal) instanceof JakYakow yak){
            Vec3 target=yak.position().add(0,1,0);
            Vec3 eye=target.add(3.5,1.6,-3.5),delta=target.subtract(eye);
            camera=new InputScene.CameraView(eye,(float)Math.toDegrees(Math.atan2(-delta.x,delta.z)),
                (float)-Math.toDegrees(Math.atan2(delta.y,delta.horizontalDistance())));
            if(ticks<53&&yak.position().distanceTo(origin)>1.25)result.addProperty("fled_from_rear",true);
            if(ticks>53&&ticks<72&&client.player.distanceTo(yak)<=7)
                initialDistance=Double.isFinite(initialDistance)?Math.max(initialDistance,client.player.distanceTo(yak)):client.player.distanceTo(yak);
            if(ticks%5==0&&ticks>=50&&ticks<=150){
                var row=new JsonObject();
                row.addProperty("tick",ticks);
                row.addProperty("x",yak.getX()-origin.x);
                row.addProperty("y",yak.getY()-origin.y);
                row.addProperty("z",yak.getZ()-origin.z);
                row.addProperty("distance",client.player.distanceTo(yak));
                row.addProperty("playerY",client.player.getY()-origin.y);
                row.addProperty("wheat",client.player.getMainHandItem().is(Items.WHEAT));
                row.addProperty("clip",yak.clip());
                row.addProperty("health",yak.getHealth());
                trace.add(row);
            }
            if(ticks>61&&ticks<115&&client.player.getMainHandItem().is(Items.WHEAT)&&client.player.distanceTo(yak)<initialDistance-1)
                result.addProperty("tempted_by_wheat",true);
            if(ticks>=115&&yak.clip().equals("yakow-kicked"))result.addProperty("authored_kick",true);
        }
        if(ticks==10){
            client.gui.hud.getChat().clearMessages(false);
            result.addProperty("short_teleport",false);
            if(client.player.position().distanceToSqr(origin.add(0,0,-4))<0.09)teleport.run();
        }
        if(ticks>=20&&ticks<53&&client.player.position().distanceToSqr(origin.add(1,0,-4))<0.09){
            if(++settledTicks>=10)result.addProperty("short_teleport",true);
        }else settledTicks=0;
        if(ticks==36)Screenshot.grab(client.gameDirectory,"jak-yakow.png",client.gameRenderer.mainRenderTarget(),1,_-> {});
        if(ticks==53)tempt.run();
        if(ticks==115)hit.run();
        if(ticks==138)kill.run();
        if(ticks<165)return;
        result.addProperty("source_audio",JakEnemySounds.cuesPlayed>cues);
        result.addProperty("native_cow_loot",nativeLoot.get());
        boolean pass=result.size()==7&&result.entrySet().stream().allMatch(e->e.getValue().getAsBoolean());
        result.addProperty("reason",pass?"complete":"Yakow assertion failed");
        result.add("trace",trace);
        try{
            var file=client.gameDirectory.toPath().resolve("jakmc/evidence/yakow/result.json");
            Files.createDirectories(file.getParent());
            Files.writeString(file,result.toString());
        }catch(IOException failure){throw new UncheckedIOException(failure);}
        active=null;
        cleanup.run();
        client.stop();
    }
}
