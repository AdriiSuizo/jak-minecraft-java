package dev.jakmc.client;

import com.google.gson.JsonObject;
import dev.jakmc.JakData;
import dev.jakmc.client.input.SdlVirtualPad;
import dev.jakmc.entity.JakVehicle;
import java.io.IOException;
import java.io.UncheckedIOException;
import java.nio.file.Files;
import java.util.LinkedHashMap;
import net.fabricmc.fabric.api.client.event.lifecycle.v1.ClientTickEvents;
import net.minecraft.client.CameraType;
import net.minecraft.client.Minecraft;
import net.minecraft.client.Screenshot;
import net.minecraft.core.BlockPos;
import net.minecraft.server.level.ServerPlayer;
import net.minecraft.world.InteractionHand;
import net.minecraft.world.entity.EntitySpawnReason;
import net.minecraft.world.level.block.Blocks;
import net.minecraft.world.level.block.state.BlockState;
import net.minecraft.world.level.levelgen.Heightmap;
import net.minecraft.world.phys.Vec3;
import org.lwjgl.sdl.SDLGamepad;

final class CarScene {
    private static SdlVirtualPad pad;
    private static volatile int carId=-1;
    private static volatile Vec3 start;
    private static int tick;
    private static CameraType camera;
    private static boolean pause;
    private static volatile java.util.function.Consumer<ServerPlayer> cleanup;
    private static final double[] DISTANCE=new double[2],HEIGHT=new double[2],STEP_SPEED=new double[2];
    private static final int[] CONTACTS=new int[2];
    private static double airBoost,airborneSpeed;
    private static int index;
    private static boolean dismounted;
    private static final boolean[] FIRED=new boolean[2];
    private static boolean grenadeBurst,hover;
    private static final boolean[] LAMPS=new boolean[2],LANE=new boolean[2],REVERSE=new boolean[2];
    private static final java.util.List<Integer> DISPLAY=new java.util.ArrayList<>();
    private static InputScene.CameraView view;

    static boolean backgroundPad(int id){return pad!=null&&pad.id==id;}
    static InputScene.CameraView camera(){return view;}
    static void initialize(){ClientTickEvents.END_CLIENT_TICK.register(CarScene::tick);}
    static void start(Minecraft client,boolean zoomers){
        if(pad!=null||client.getSingleplayerServer()==null||client.player==null)throw new IllegalStateException("Open the development world");
        index=tick=0;
        hover=zoomers;
        java.util.Arrays.fill(LAMPS,false);
        java.util.Arrays.fill(LANE,false);
        java.util.Arrays.fill(REVERSE,false);
        dismounted=grenadeBurst=false;
        java.util.Arrays.fill(FIRED,false);
        java.util.Arrays.fill(DISTANCE,0);java.util.Arrays.fill(HEIGHT,0);
        java.util.Arrays.fill(CONTACTS,0);java.util.Arrays.fill(STEP_SPEED,0);
        airBoost=airborneSpeed=0;
        camera=client.options.getCameraType();
        pause=client.options.pauseOnLostFocus;
        client.options.pauseOnLostFocus=false;
        client.options.setCameraType(CameraType.THIRD_PERSON_BACK);
        pad=new SdlVirtualPad();
        onServer(client,p->{
            var level=p.level();
            if(hover)level.getServer().getCommands().performPrefixedCommand(level.getServer().createCommandSourceStack(),"time set midnight");
            var original=p.position();
            var rotation=new float[]{p.getYRot(),p.getXRot()};
            var enabled=p.getAttachedOrCreate(JakData.ENABLED);
            var near=p.blockPosition();
            int y=near.getY();
            for(int x=-5;x<=16;x++)for(int z=-8;z<=55;z++)
                y=Math.max(y,level.getHeight(Heightmap.Types.MOTION_BLOCKING,near.getX()+x,near.getZ()+z));
            var base=near.atY(y);
            var snapshot=new LinkedHashMap<BlockPos,BlockState>();
            for(int x=-5;x<=16;x++)for(int z=-8;z<=55;z++){
                int rise=z>=12?1:0;
                for(int h=0;h<=rise;h++){
                    var at=base.offset(x,h,z);
                    snapshot.put(at,level.getBlockState(at));
                    level.setBlock(at,Blocks.SMOOTH_STONE.defaultBlockState(),3);
                }
            }
            start=Vec3.atBottomCenterOf(base.above());
            p.setAttached(JakData.ENABLED,true);
            cleanup=player->{
                player.stopRiding();
                snapshot.forEach(level::setBlockAndUpdate);
                player.setAttached(JakData.ENABLED,enabled);
                player.teleportTo(original.x,original.y,original.z);
                player.setYRot(rotation[0]);player.setXRot(rotation[1]);
            };
            if(hover){
                for(int i=3;i<=6;i++){
                    var model=JakVehicle.Model.values()[i];
                    var car=model.type.create(level,EntitySpawnReason.COMMAND);
                    var pos=start.add(i%2==0?16:-4,model.parkingClearance(),8+(i-3)*6);
                    car.snapTo(pos.x,pos.y,pos.z,0,0);
                    if(level.addFreshEntity(car))DISPLAY.add(car.getId());
                }
            }
            spawn(p,0);
        });
    }

    private static void spawn(ServerPlayer p,int index){
        var model=JakVehicle.Model.values()[hover?(index==0?2:7):index];
        var car=model.type.create(p.level(),EntitySpawnReason.COMMAND);
        Vec3 at=start.add(index*10,hover?model.parkingClearance():1.3,0);
        car.setPos(at);
        car.setYRot(0);
        p.teleportTo(at.x,start.y,at.z-(hover?1.4:2));
        p.setYRot(0);p.setXRot(0);
        if(!p.level().addFreshEntity(car))throw new IllegalStateException("Could not place source car");
        carId=car.getId();
    }

    private static void tick(Minecraft client){
        if(pad==null||client.level==null||client.player==null||carId<0||!JakInput.controllerReady(pad.id))return;
        if(!(client.level.getEntity(carId) instanceof JakVehicle car)||car.frame()==null)return;
        tick++;
        var pos=car.position().subtract(start);
        var frame=car.frame();
        int grounded=frame.get("grounded").getAsInt();
        if(hover){
            LAMPS[index]|=frame.get("lamps").getAsFloat()>.8f&&frame.getAsJsonArray("heads").size()>0;
            LANE[index]|=frame.get("high").getAsBoolean()&&pos.y>5;
            REVERSE[index]|=tick>55&&dev.jakmc.player.JakPlayers.vector(frame.getAsJsonArray("velocity")).z<-.2;
        }
        if(index==0&&!frame.getAsJsonArray("grenades").isEmpty())FIRED[0]=true;
        for(var effect:frame.getAsJsonArray("gunEffects")){
            String name=effect.getAsJsonObject().get("effect").getAsString();
            if(name.equals("beam")&&index==1)FIRED[1]=true;
            if(name.equals("group-toad-grenade-shot-explode"))grenadeBurst=true;
        }
        double speed=dev.jakmc.player.JakPlayers.vector(frame.getAsJsonArray("velocity")).horizontalDistance();
        DISTANCE[index]=Math.max(DISTANCE[index],pos.z);
        HEIGHT[index]=Math.max(HEIGHT[index],pos.y);
        if(pos.z>=13&&grounded>=2)CONTACTS[index]=Math.max(CONTACTS[index],grounded);
        if(pos.z>=12&&pos.z<=17)STEP_SPEED[index]=Math.max(STEP_SPEED[index],speed);
        if(tick==10)onServer(client,p->{
            var target=p.level().getEntity(carId);
            target.interact(p,InteractionHand.MAIN_HAND,new Vec3(0,hover?-.55:0,-1));
        });
        if(tick==25)pad.axis(5,.12f);
        if(!hover&&tick==32)pad.button(SDLGamepad.SDL_GAMEPAD_BUTTON_RIGHT_SHOULDER,true);
        if(!hover&&tick==36)pad.button(SDLGamepad.SDL_GAMEPAD_BUTTON_RIGHT_SHOULDER,false);
        if(tick==37)pad.axis(5,1);
        if((hover&&tick==29)||(!hover&&tick==49&&index==0))pad.button(SDLGamepad.SDL_GAMEPAD_BUTTON_SOUTH,true);
        if(hover&&tick==31)pad.button(SDLGamepad.SDL_GAMEPAD_BUTTON_SOUTH,false);
        if(hover&&tick==49)pad.button(SDLGamepad.SDL_GAMEPAD_BUTTON_SOUTH,true);
        if(hover&&tick==51)pad.button(SDLGamepad.SDL_GAMEPAD_BUTTON_SOUTH,false);
        if(!hover&&tick==54&&index==0){
            pad.button(SDLGamepad.SDL_GAMEPAD_BUTTON_SOUTH,false);
            pad.button(SDLGamepad.SDL_GAMEPAD_BUTTON_EAST,true);
        }
        if(hover&&tick==52){pad.axis(5,-1);pad.axis(4,1);}
        if(!hover&&index==0&&tick>=54&&grounded==0){
            if(airborneSpeed==0)airborneSpeed=speed;
            if(frame.get("boost").getAsBoolean())airBoost=Math.max(airBoost,speed-airborneSpeed);
        }
        if(tick==47||!hover&&tick==61&&index==0){
            Vec3 target=car.position().add(0,.65,0);
            Vec3 eye=target.add(-5,2.5,6);
            Vec3 direction=target.subtract(eye);
            view=new InputScene.CameraView(eye,(float)Math.toDegrees(Math.atan2(-direction.x,direction.z)),
                (float)-Math.toDegrees(Math.atan2(direction.y,direction.horizontalDistance())));
        }
        if(tick==48||!hover&&tick==62&&index==0){
            String name=hover?(index==0?"zoomers-bike.png":"zoomers-car.png"):index==1?"cars-snake.png":tick==48?"cars-toad.png":"cars-air.png";
            Screenshot.grab(client.gameDirectory,name,client.gameRenderer.mainRenderTarget(),1,_-> {});
        }
        if(tick==(hover?110:72)){
            pad.axis(5,-1);
            pad.button(SDLGamepad.SDL_GAMEPAD_BUTTON_EAST,false);
            pad.button(SDLGamepad.SDL_GAMEPAD_BUTTON_NORTH,true);
        }
        if(tick==(hover?114:76)){pad.button(SDLGamepad.SDL_GAMEPAD_BUTTON_NORTH,false);pad.axis(4,-1);}
        if(tick!=(hover?118:80))return;
        dismounted=client.player.getVehicle()==null;
        if(index==0){
            view=null;
            index=1;
            tick=0;
            int previous=carId;
            carId=-1;
            onServer(client,p->{
                p.stopRiding();
                var old=p.level().getEntity(previous);
                if(old!=null)old.discard();
                spawn(p,1);
            });
            return;
        }
        var report=new JsonObject();
        report.add("distance",new com.google.gson.Gson().toJsonTree(DISTANCE));
        report.add("height",new com.google.gson.Gson().toJsonTree(HEIGHT));
        report.add("contacts",new com.google.gson.Gson().toJsonTree(CONTACTS));
        report.add("stepSpeed",new com.google.gson.Gson().toJsonTree(STEP_SPEED));
        report.addProperty("airBoostGain",airBoost);
        report.addProperty("dismounted",dismounted);
        report.addProperty("grenade",FIRED[0]);
        report.addProperty("grenadeBurst",grenadeBurst);
        report.addProperty("gunBeam",FIRED[1]);
        report.add("lamps",new com.google.gson.Gson().toJsonTree(LAMPS));
        report.add("lanes",new com.google.gson.Gson().toJsonTree(LANE));
        report.add("reverse",new com.google.gson.Gson().toJsonTree(REVERSE));
        boolean success=hover?DISTANCE[0]>5&&DISTANCE[1]>5&&LAMPS[0]&&LAMPS[1]&&LANE[0]&&LANE[1]
            &&REVERSE[0]&&REVERSE[1]&&dismounted:
            DISTANCE[0]>=16&&DISTANCE[1]>=16&&CONTACTS[0]>=2&&CONTACTS[1]>=2
            &&STEP_SPEED[0]>.1&&STEP_SPEED[1]>.1&&airBoost>.02&&dismounted&&FIRED[0]&&FIRED[1]&&grenadeBurst;
        report.addProperty("reason",success?"complete":"Vehicle acceptance failed");
        try{
            var folder=client.gameDirectory.toPath().resolve("jakmc/evidence/"+(hover?"zoomers":"cars"));
            Files.createDirectories(folder);
            Files.writeString(folder.resolve("result.json"),report.toString());
        }catch(IOException e){throw new UncheckedIOException(e);}
        pad.close();pad=null;
        view=null;
        client.options.pauseOnLostFocus=pause;
        client.options.setCameraType(camera);
        int last=carId;
        onServer(client,p->{
            p.stopRiding();
            var old=p.level().getEntity(last);
            if(old!=null)old.discard();
            for(int id:DISPLAY){
                var parked=p.level().getEntity(id);
                if(parked!=null)parked.discard();
            }
            DISPLAY.clear();
            cleanup.accept(p);
            client.execute(client::stop);
        });
    }

    private static void onServer(Minecraft client,java.util.function.Consumer<ServerPlayer> action){
        var server=client.getSingleplayerServer();
        var uuid=client.player.getUUID();
        server.execute(()->{
            var player=server.getPlayerList().getPlayer(uuid);
            if(player!=null)action.accept(player);
        });
    }
}
