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
import net.minecraft.world.level.GameType;
import net.minecraft.world.level.block.Blocks;
import net.minecraft.world.level.block.SlabBlock;
import net.minecraft.world.level.block.state.BlockState;
import net.minecraft.world.level.levelgen.Heightmap;
import net.minecraft.world.phys.Vec3;

final class SurfaceScene {
    private static SurfaceScene active;
    private final Runnable skate,slide,wade,swim,levelStroke,restore;
    private final CameraType originalCamera;
    private final JsonObject seen=new JsonObject();
    private InputScene.CameraView camera;
    private int ticks;
    private boolean skateShot;
    private double slideStart=Double.NaN,maxCoast;
    private float maxTilt;

    private SurfaceScene(Runnable skate,Runnable slide,Runnable wade,Runnable swim,Runnable levelStroke,
                         Runnable restore,CameraType originalCamera){
        this.skate=skate;this.slide=slide;this.wade=wade;this.swim=swim;this.levelStroke=levelStroke;
        this.restore=restore;this.originalCamera=originalCamera;
    }

    static InputScene.CameraView camera(){return active==null?null:active.camera;}

    static void initialize(){
        ClientTickEvents.END_CLIENT_TICK.register(client->{if(active!=null)active.tick(client);});
    }

    static void start(Minecraft client){
        var server=client.getSingleplayerServer();
        var id=client.player.getUUID();
        var perspective=client.options.getCameraType();
        client.options.setCameraType(CameraType.THIRD_PERSON_BACK);
        server.execute(()->{
            var player=server.getPlayerList().getPlayer(id);
            var level=player.level();
            Vec3 previous=player.position();
            var mode=player.gameMode.getGameModeForPlayer();
            boolean enabled=player.getAttachedOrCreate(JakData.ENABLED);
            var here=player.blockPosition();
            int y=here.getY();
            for(int x=-3;x<=16;x++)for(int z=-4;z<=15;z++)
                y=Math.max(y,level.getHeight(Heightmap.Types.MOTION_BLOCKING,here.getX()+x,here.getZ()+z));
            var floor=here.atY(y);
            var before=new LinkedHashMap<BlockPos,BlockState>();
            var stone=Blocks.STONE.defaultBlockState();
            var air=Blocks.AIR.defaultBlockState();
            var water=Blocks.WATER.defaultBlockState();
            var slab=Blocks.STONE_SLAB.defaultBlockState().setValue(SlabBlock.WATERLOGGED,true);
            for(int x=-3;x<=16;x++)for(int z=-4;z<=15;z++)for(int h=0;h<=5;h++){
                var at=floor.offset(x,h,z);
                before.put(at,level.getBlockState(at));
                BlockState state=air;
                if(h==0)state=x<=2?Blocks.BLUE_ICE.defaultBlockState():stone;
                else if(x>=5&&x<=8&&h==1)state=z>=0&&z<=8&&x>5&&x<8?slab:stone;
                else if(x>=10&&h<=3)state=x==10||x==16||z==-4||z==15?stone:water;
                level.setBlockAndUpdate(at,state);
            }
            player.setGameMode(GameType.SURVIVAL);
            player.setAttached(JakData.ENABLED,true);
            Vec3 ice=Vec3.atBottomCenterOf(floor.offset(0,1,-2));
            Vec3 shallow=Vec3.atBottomCenterOf(floor.offset(6,1,1)).add(0,.5,0);
            Vec3 pool=Vec3.atBottomCenterOf(floor.offset(13,2,0));
            java.util.function.BiConsumer<Vec3,Integer> place=(at,held)->{
                player.teleportTo(at.x,at.y,at.z);
                player.setYRot(0);
                player.setXRot(0);
                player.setOnGround(held==0);
                JakPlayers.probeInput(player,held,held,1);
            };
            JakPlayers.controlForProbe(player,true);
            place.accept(ice,0);
            JakPlayers.probeInput(player,0,0,0);
            var scene=new SurfaceScene(()->server.execute(()->JakPlayers.probeInput(player,0,0,1)),
                ()->server.execute(()->JakPlayers.probeInput(player,0,0,0)),
                ()->server.execute(()->place.accept(shallow,0)),()->server.execute(()->place.accept(pool,16)),
                ()->server.execute(()->JakPlayers.probeInput(player,0,0,1)),()->{
                    before.forEach(level::setBlockAndUpdate);
                    JakPlayers.controlForProbe(player,false);
                    player.setAttached(JakData.ENABLED,enabled);
                    player.setGameMode(mode);
                    player.teleportTo(previous.x,previous.y,previous.z);
                },perspective);
            client.execute(()->{
                scene.aim(ice,new Vec3(2.7,1.3,2.2));
                active=scene;
            });
        });
    }

    private void tick(Minecraft client){
        ++ticks;
        var pose=JakPoses.get(client.player.getId());
        if(pose!=null){
            String clip=pose.raw().get("surfaceClip") instanceof com.google.gson.JsonPrimitive p?p.getAsString():"";
            if(ticks<75){
                if(clip.equals("jakb1-ice-skate")){
                    seen.addProperty("skate",true);
                    if(!skateShot){
                        aim(pose.position(),new Vec3(2.7,1.3,2.2));
                        Vec3 actual=client.gameRenderer.mainCamera().position();
                        if(actual.distanceToSqr(camera.position())<.6){
                            Screenshot.grab(client.gameDirectory,"jak-surface-ice.png",client.gameRenderer.mainRenderTarget(),1,_-> {});
                            skateShot=true;
                        }
                    }
                }
                if(clip.equals("jakb1-ice-slide")){
                    if(!Double.isFinite(slideStart))slideStart=pose.position().z;
                    maxCoast=Math.max(maxCoast,pose.position().z-slideStart);
                }
            }else if(ticks<102&&clip.equals("jakb-wade-shallow-walk"))seen.addProperty("wade",true);
            if(ticks>=102&&pose.raw().getAsJsonObject("rootPose")!=null){
                float tilt=pose.rootPose().swimTilt();
                maxTilt=Math.max(maxTilt,tilt);
                if(tilt>1.2f&&pose.clip().equals("jakb-swim-down")){
                    if(!seen.has("dive"))aim(pose.position(),new Vec3(2.2,.7,-2.6));
                    seen.addProperty("dive",true);
                }
            }
        }
        if(ticks==20)skate.run();
        if(ticks==51)slide.run();
        if(ticks==75){camera=null;wade.run();}
        if(ticks==102)swim.run();
        if(ticks==118)levelStroke.run();
        if(ticks==138)Screenshot.grab(client.gameDirectory,"jak-surface-swim.png",client.gameRenderer.mainRenderTarget(),1,_-> {});
        if(ticks<146)return;
        seen.addProperty("coasted",maxCoast>.12);
        boolean pass=seen.size()==4&&seen.entrySet().stream().allMatch(e->e.getValue().getAsBoolean());
        seen.addProperty("coastBlocks",maxCoast);
        seen.addProperty("maxTilt",maxTilt);
        seen.addProperty("reason",pass?"complete":"Surface assertion failed");
        try{
            var result=client.gameDirectory.toPath().resolve("jakmc/evidence/surface/result.json");
            Files.createDirectories(result.getParent());
            Files.writeString(result,seen.toString());
        }catch(IOException e){throw new UncheckedIOException(e);}
        active=null;
        client.options.setCameraType(originalCamera);
        client.getSingleplayerServer().execute(restore);
        client.stop();
    }

    private void aim(Vec3 feet,Vec3 offset){
        Vec3 target=feet.add(0,.8,0),eye=target.add(offset),d=target.subtract(eye);
        camera=new InputScene.CameraView(eye,(float)Math.toDegrees(Math.atan2(-d.x,d.z)),
            (float)-Math.toDegrees(Math.atan2(d.y,d.horizontalDistance())));
    }
}
