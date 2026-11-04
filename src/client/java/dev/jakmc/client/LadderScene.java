package dev.jakmc.client;

import com.google.gson.JsonObject;
import dev.jakmc.JakData;
import dev.jakmc.player.JakPlayers;
import java.io.IOException;
import java.io.UncheckedIOException;
import java.nio.file.Files;
import java.util.LinkedHashMap;
import net.fabricmc.fabric.api.client.event.lifecycle.v1.ClientTickEvents;
import net.minecraft.client.CameraType;
import net.minecraft.client.Minecraft;
import net.minecraft.client.Screenshot;
import net.minecraft.core.BlockPos;
import net.minecraft.core.Direction;
import net.minecraft.world.level.GameType;
import net.minecraft.world.level.block.Blocks;
import net.minecraft.world.level.block.LadderBlock;
import net.minecraft.world.level.block.state.BlockState;
import net.minecraft.world.level.levelgen.Heightmap;
import net.minecraft.world.phys.Vec3;

final class LadderScene {
    private static LadderScene active;
    private final Vec3 base;
    private final java.util.function.Consumer<Float> input;
    private final Runnable restore;
    private final CameraType previousCamera;
    private final JsonObject report=new JsonObject();
    private int ticks,phase,rest;
    private double idleHeight,peak;
    private InputScene.CameraView view;

    private LadderScene(Vec3 base,java.util.function.Consumer<Float> input,Runnable restore,CameraType previousCamera){
        this.base=base;this.input=input;this.restore=restore;this.previousCamera=previousCamera;
    }

    static InputScene.CameraView camera(){return active==null?null:active.view;}

    static void initialize(){
        ClientTickEvents.END_CLIENT_TICK.register(client->{if(active!=null)active.tick(client);});
    }

    static void start(Minecraft client){
        var server=client.getSingleplayerServer();
        var uuid=client.player.getUUID();
        var previousCamera=client.options.getCameraType();
        client.options.setCameraType(CameraType.THIRD_PERSON_BACK);
        server.execute(()->{
            var p=server.getPlayerList().getPlayer(uuid);
            var level=p.level();
            var old=p.position();
            var mode=p.gameMode.getGameModeForPlayer();
            boolean enabled=p.getAttachedOrCreate(JakData.ENABLED);
            var center=p.blockPosition();
            int y=center.getY();
            for(int x=-2;x<=2;x++)for(int z=-3;z<=3;z++)
                y=Math.max(y,level.getHeight(Heightmap.Types.MOTION_BLOCKING,center.getX()+x,center.getZ()+z));
            var floor=center.atY(y);
            var before=new LinkedHashMap<BlockPos,BlockState>();
            for(int x=-2;x<=2;x++)for(int z=-3;z<=3;z++)for(int h=0;h<=7;h++){
                var at=floor.offset(x,h,z);
                before.put(at,level.getBlockState(at));
                BlockState state=h==0||h<=4&&z>=1?Blocks.STONE.defaultBlockState():Blocks.AIR.defaultBlockState();
                level.setBlockAndUpdate(at,state);
            }
            var foot=floor.above();
            for(int h=0;h<4;h++)level.setBlockAndUpdate(foot.above(h),Blocks.LADDER.defaultBlockState().setValue(LadderBlock.FACING,Direction.NORTH));
            Vec3 base=Vec3.atBottomCenterOf(foot);
            p.setGameMode(GameType.SURVIVAL);
            p.setAttached(JakData.ENABLED,true);
            p.teleportTo(base.x,base.y,base.z-1.7);
            p.setYRot(0);
            p.setXRot(0);
            p.setOnGround(true);
            JakPlayers.controlForProbe(p,true);
            var scene=new LadderScene(base,forward->server.execute(()->JakPlayers.probeInput(p,0,0,forward)),()->{
                before.forEach(level::setBlockAndUpdate);
                JakPlayers.controlForProbe(p,false);
                p.setAttached(JakData.ENABLED,enabled);
                p.setGameMode(mode);
                p.teleportTo(old.x,old.y,old.z);
            },previousCamera);
            client.execute(()->active=scene);
        });
    }

    private void tick(Minecraft client){
        if(++ticks==12)input.accept(1f);
        var pose=JakPoses.get(client.player.getId());
        if(pose!=null){
            double y=pose.position().y-base.y;
            peak=Math.max(peak,y);
            String clip=pose.clip();
            if(clip.equals("jakb-ladder-get-on"))report.addProperty("entry",true);
            if(phase==0&&y>1.2&&clip.equals("jakb-ladder-up")){
                report.addProperty("climb",true);
                var target=pose.position().add(0,.9,0);
                var eye=target.add(2.3,1.1,-2.1);
                Vec3 delta=target.subtract(eye);
                view=new InputScene.CameraView(eye,(float)Math.toDegrees(Math.atan2(-delta.x,delta.z)),
                    (float)-Math.toDegrees(Math.atan2(delta.y,delta.horizontalDistance())));
                input.accept(0f);
                phase=1;
            }else if(phase==1&&clip.equals("jakb-ladder-stance")){
                if(rest++==0)idleHeight=y;
                if(rest==4)Screenshot.grab(client.gameDirectory,"jak-ladder.png",client.gameRenderer.mainRenderTarget(),1,_-> {});
                if(rest==8){
                    report.addProperty("idle_holds_height",Math.abs(y-idleHeight)<.03);
                    input.accept(-1f);
                    phase=2;
                }
            }else if(phase==2&&y<idleHeight-.42&&clip.equals("jakb-ladder-down")){
                report.addProperty("descend",true);
                input.accept(1f);
                phase=3;
            }else if(phase==3&&clip.equals("jakb-edge-grab-to-jump")){
                report.addProperty("authored_roof_exit",true);
            }
            if(phase==3&&y>=3.95&&pose.position().z>base.z+.55&&
                !pose.raw().get("state").getAsString().equals("target-ladder")){
                report.addProperty("landed_on_roof",true);
                finish(client);
                return;
            }
        }
        if(ticks>=205)finish(client);
    }

    private void finish(Minecraft client){
        report.addProperty("peak",peak);
        report.addProperty("reason",report.size()==7&&report.entrySet().stream().filter(e->!e.getKey().equals("peak"))
            .allMatch(e->e.getValue().getAsBoolean())?"complete":"Ladder assertion failed");
        try{
            var result=client.gameDirectory.toPath().resolve("jakmc/evidence/ladder/result.json");
            Files.createDirectories(result.getParent());
            Files.writeString(result,report.toString());
        }catch(IOException e){throw new UncheckedIOException(e);}
        active=null;
        client.options.setCameraType(previousCamera);
        client.getSingleplayerServer().execute(restore);
        client.stop();
    }
}
