package dev.jakmc.client;

import com.google.gson.JsonObject;
import dev.jakmc.JakData;
import dev.jakmc.content.JakBlocks;
import dev.jakmc.content.JakItems;
import dev.jakmc.nativebridge.MinecraftCollision;
import dev.jakmc.player.JakPlayers;
import java.io.IOException;
import java.io.UncheckedIOException;
import java.nio.file.Files;
import java.util.LinkedHashMap;
import net.fabricmc.fabric.api.client.event.lifecycle.v1.ClientTickEvents;
import net.minecraft.client.Minecraft;
import net.minecraft.client.CameraType;
import net.minecraft.client.Screenshot;
import net.minecraft.core.BlockPos;
import net.minecraft.core.Direction;
import net.minecraft.server.level.ServerPlayer;
import net.minecraft.world.InteractionHand;
import net.minecraft.world.item.ItemStack;
import net.minecraft.world.item.context.UseOnContext;
import net.minecraft.world.level.GameType;
import net.minecraft.world.level.block.Blocks;
import net.minecraft.world.level.block.state.BlockState;
import net.minecraft.world.level.levelgen.Heightmap;
import net.minecraft.world.phys.AABB;
import net.minecraft.world.phys.BlockHitResult;
import net.minecraft.world.phys.Vec3;

final class TrackScene {
    private static final int BOARD=1<<7;
    private static final int JUMP=1;
    private static int ticks,railTicks;
    private static volatile boolean jumped;
    private static ServerPlayer approachingRail;
    private static long jumpRelease;
    private static boolean blasterContact,opening,folding,offAfterFold;
    private static float minGrindFrame,maxGrindFrame,maxTilt;
    private static double maxHeight,maxGrindDistance,grindStart;
    private static JsonObject checks;
    private static com.google.gson.JsonArray rampTrace;
    private static Vec3 ramp,rail;
    private static InputScene.CameraView view;
    private static CameraType previousCamera;
    private static Runnable cleanup,toRail,deploy,move,toRange,fire,clearGun;

    static InputScene.CameraView camera(){return view;}

    static void initialize(){
        net.fabricmc.fabric.api.event.lifecycle.v1.ServerTickEvents.START_SERVER_TICK.register(server->{
            var p=approachingRail;
            if(p==null)return;
            long time=p.level().getGameTime();
            if(!jumped&&JakPlayers.onBoard(p)&&p.getZ()>=rail.z-1.85&&p.getZ()<=rail.z+.2){
                jumped=true;
                jumpRelease=time+4;
                JakPlayers.probeInput(p,JUMP,JUMP,1);
            }
            if(jumped&&time==jumpRelease)JakPlayers.probeInput(p,0,0,1);
        });
        ClientTickEvents.END_CLIENT_TICK.register(client->{
            if(checks==null||client.player==null)return;
            ++ticks;
            var server=client.getSingleplayerServer();
            var pose=JakPoses.get(client.player.getId());
            if(ticks==22||ticks==122||ticks==246||ticks==274)server.execute(deploy);
            if(ticks==25||ticks==125)server.execute(move);
            if(ticks==29&&pose!=null){
                var target=pose.position().add(0,.8,0);
                view=cameraAt(target.add(3,1.8,-3.4),target);
            }
            if(ticks==32)
                Screenshot.grab(client.gameDirectory,"jak-track.png",client.gameRenderer.mainRenderTarget(),1,_-> {});
            if(ticks==105)server.execute(toRail);
            if(ticks==214)server.execute(toRange);
            if(ticks==230)server.execute(fire);
            if(ticks==237)server.execute(clearGun);
            if(pose!=null&&ticks>=230&&pose.raw().has("events")){
                Vec3 origin=pose.position().subtract(JakPlayers.vector(pose.raw().getAsJsonArray("pos")));
                for(var raw:pose.raw().getAsJsonArray("events")){
                    var event=raw.getAsJsonObject();
                    String kind=event.get("kind").getAsString();
                    if(!kind.equals("impact")&&!kind.equals("world_hit"))continue;
                    Vec3 at=JakPlayers.vector(event.getAsJsonArray("pos")).add(origin);
                    if(Math.abs(at.x-ramp.x)<2.5&&at.z>=ramp.z-.3&&at.z<=ramp.z+6.8&&at.y>=ramp.y)
                        blasterContact=true;
                }
            }
            if(pose!=null&&pose.board()){
                var visual=pose.boardPose();
                if(visual!=null){
                    if(ticks<=26&&visual.age()>0&&visual.age()<.4f)opening=true;
                    if(ticks>=274&&visual.closing())folding=true;
                    maxTilt=Math.max(maxTilt,(float)Math.hypot(visual.rotation().x,visual.rotation().z));
                    if(Float.isFinite(visual.grindFrame())){
                        minGrindFrame=Math.min(minGrindFrame,visual.grindFrame());
                        maxGrindFrame=Math.max(maxGrindFrame,visual.grindFrame());
                    }
                }
                var at=pose.position();
                if(ticks<102&&at.z>=ramp.z&&at.z<=ramp.z+6.8)
                    maxHeight=Math.max(maxHeight,at.y-ramp.y);
                if(ticks<105&&ticks%5==0){
                    var sample=new JsonObject();
                    sample.addProperty("z",at.z-ramp.z);
                    sample.addProperty("height",at.y-ramp.y);
                    sample.add("velocity",pose.raw().get("velocity"));
                    sample.add("state",pose.raw().get("state"));
                    rampTrace.add(sample);
                }
                if(ticks>130&&pose.raw().get("state").getAsString().equals("board-grind")){
                    if(++railTicks==1)grindStart=at.z;
                    if(railTicks==6){
                        var target=at.add(0,.8,0);
                        view=cameraAt(target.add(3,2,-3),target);
                    }
                    if(railTicks==10)Screenshot.grab(client.gameDirectory,"jak-track-grind.png",
                        client.gameRenderer.mainRenderTarget(),1,_-> {});

                    maxGrindDistance=Math.max(maxGrindDistance,Math.abs(at.z-grindStart));
                }
            }
            if(folding&&pose!=null&&!pose.board())offAfterFold=true;
            if(ticks<310)return;
            checks.addProperty("yellow_blaster_ramp_impact",blasterContact);
            checks.addProperty("rode_above_ramp_base",maxHeight>=1);
            checks.addProperty("native_grind",railTicks>3&&maxGrindDistance>=1);
            checks.addProperty("opening_clock",opening);
            checks.addProperty("native_root_tilt",maxTilt>.04f);
            checks.addProperty("grind_artist_frames",maxGrindFrame-minGrindFrame>.02f);
            checks.addProperty("native_fold",folding&&offAfterFold);
            var result=new JsonObject();
            result.add("actions",checks);
            result.addProperty("rampHeight",maxHeight);
            result.add("rampTrace",rampTrace);
            result.addProperty("railTicks",railTicks);
            result.addProperty("grindBlocks",maxGrindDistance);
            result.addProperty("jumped",jumped);
            result.addProperty("maxTilt",maxTilt);
            if(Float.isFinite(minGrindFrame))result.addProperty("minGrindFrame",minGrindFrame);
            if(Float.isFinite(maxGrindFrame))result.addProperty("maxGrindFrame",maxGrindFrame);
            result.addProperty("lastPose",pose==null?"missing":pose.raw().get("state").getAsString());
            result.addProperty("reason",checks.entrySet().stream().allMatch(e->e.getValue().getAsBoolean())?
                "complete":"Track assertion failed");
            try{
                var dir=client.gameDirectory.toPath().resolve("jakmc/evidence/track");
                Files.createDirectories(dir);
                Files.writeString(dir.resolve("result.json"),result.toString());
            }catch(IOException e){throw new UncheckedIOException(e);}
            checks=null;
            view=null;
            client.options.setCameraType(previousCamera);
            server.execute(cleanup);
            client.stop();
        });
    }

    static void start(Minecraft client){
        var server=client.getSingleplayerServer();
        if(server==null||client.player==null)throw new IllegalStateException("Open the development world");
        previousCamera=client.options.getCameraType();
        client.options.setCameraType(CameraType.THIRD_PERSON_BACK);
        ticks=railTicks=0;
        jumped=false;
        blasterContact=opening=folding=offAfterFold=false;
        maxTilt=0;
        minGrindFrame=Float.POSITIVE_INFINITY;
        maxGrindFrame=Float.NEGATIVE_INFINITY;
        rampTrace=new com.google.gson.JsonArray();
        maxHeight=maxGrindDistance=0;
        var uuid=client.player.getUUID();
        server.execute(()->{
            var p=server.getPlayerList().getPlayer(uuid);
            if(p==null)return;
            var level=p.level();
            var original=p.position();
            var mode=p.gameMode.getGameModeForPlayer();
            var held=p.getMainHandItem().copy();
            var stored=p.getAttachedOrCreate(JakData.BOARD).copy();
            boolean active=p.getAttachedOrCreate(JakData.ENABLED);
            var near=p.blockPosition();
            int y=near.getY();
            for(int x=-6;x<=19;x++)for(int z=-12;z<=29;z++)
                y=Math.max(y,level.getHeight(Heightmap.Types.MOTION_BLOCKING,near.getX()+x,near.getZ()+z));
            var base=near.atY(y);
            var snapshot=new LinkedHashMap<BlockPos,BlockState>();
            var tile=JakBlocks.BLOCKS.get("palacio_suelo_metal").defaultBlockState();
            for(int x=-5;x<=17;x++)for(int z=-11;z<=24;z++){
                var at=base.offset(x,0,z);
                snapshot.put(at,level.getBlockState(at));
                level.setBlockAndUpdate(at,tile);
            }
            var rampStart=base.above();
            var railAnchor=rampStart.east(13);
            for(int x=-5;x<=17;x++)for(int z=-2;z<=23;z++)for(int h=1;h<=5;h++){
                var at=base.offset(x,h,z);
                snapshot.put(at,level.getBlockState(at));
                if(!level.getBlockState(at).isAir())level.setBlockAndUpdate(at,Blocks.AIR.defaultBlockState());
            }
            p.setYRot(0);
            p.setXRot(0);
            p.setGameMode(GameType.SURVIVAL);
            p.setAttached(JakData.ENABLED,true);
            p.setAttached(JakData.BOARD,new ItemStack(JakItems.BOARD));
            var quarter=JakBlocks.BLOCKS.get("objeto_haven_rampa_cuarto");
            var rampPyramid=JakBlocks.BLOCKS.get("objeto_haven_rampa_piramide");
            var grindRail=JakBlocks.BLOCKS.get("objeto_haven_rail_grind");
            p.setItemInHand(InteractionHand.MAIN_HAND,new ItemStack(quarter,5));
            boolean ramps=true;
            for(int x=-2;x<=2;x++)ramps&=place(p,base.offset(x,0,0));
            p.setItemInHand(InteractionHand.MAIN_HAND,new ItemStack(rampPyramid,1));
            boolean pyramid=place(p,base.offset(7,0,6));
            p.setItemInHand(InteractionHand.MAIN_HAND,new ItemStack(grindRail,1));
            boolean placedRail=place(p,railAnchor.below());
            p.setItemInHand(InteractionHand.MAIN_HAND,ItemStack.EMPTY);
            var entry=Vec3.atBottomCenterOf(rampStart.north(7));
            p.teleportTo(entry.x,entry.y,entry.z);
            p.setOnGround(true);
            JakPlayers.controlForProbe(p,true);
            deploy=()->JakPlayers.probeInput(p,BOARD,BOARD,0);
            move=()->JakPlayers.probeInput(p,0,0,1);
            var railsEntry=Vec3.atBottomCenterOf(railAnchor.north(7)).add(-.5,0,0);
            toRail=()->{
                approachingRail=p;
                jumpRelease=Long.MIN_VALUE;
                p.teleportTo(railsEntry.x,railsEntry.y,railsEntry.z);
                p.setYRot(0);
                p.setOnGround(true);
                JakPlayers.probeInput(p,0,0,0);
            };
            toRange=()->{
                approachingRail=null;
                p.teleportTo(entry.x,entry.y,entry.z);
                p.setYRot(0);
                p.setXRot(0);
                p.setOnGround(true);
                p.setItemInHand(InteractionHand.MAIN_HAND,new ItemStack(JakItems.GUNS.get(3)));
                JakPlayers.probeInput(p,0,0,0);
            };
            fire=()->JakPlayers.probeInput(p,64,64,0);
            clearGun=()->{
                p.setItemInHand(InteractionHand.MAIN_HAND,ItemStack.EMPTY);
                JakPlayers.probeInput(p,0,0,0);
            };
            cleanup=()->{
                approachingRail=null;
                snapshot.forEach(level::setBlockAndUpdate);
                JakPlayers.controlForProbe(p,false);
                p.setAttached(JakData.BOARD,stored);
                p.setAttached(JakData.ENABLED,active);
                p.setGameMode(mode);
                p.setItemInHand(InteractionHand.MAIN_HAND,held);
                p.teleportTo(original.x,original.y,original.z);
            };
            var report=new JsonObject();
            report.addProperty("five_modular_ramp_columns",ramps);
            report.addProperty("pyramid",pyramid);
            report.addProperty("rail",placedRail);
            var triangles=MinecraftCollision.triangles(level,p,Vec3.atLowerCornerOf(base),
                new AABB(Vec3.atLowerCornerOf(base.offset(-2,1,0)),Vec3.atLowerCornerOf(base.offset(15,6,8))));
            int curved=0,metalRail=0;
            for(int i=0;i<triangles.remaining();i+=40){
                int flags=triangles.getInt(i+36);
                if(flags==3)++curved;
                if(flags==16)++metalRail;
            }
            report.addProperty("source_halfpipe_triangles",curved>0);
            report.addProperty("source_rail_contact",metalRail>0);
            var queriedRails=new MinecraftCollision.Query(level,p,entry).rails(Vec3.atCenterOf(railAnchor));
            var ownRail=level.getBlockEntity(railAnchor);
            report.addProperty("source_rail_line",ownRail instanceof dev.jakmc.content.JakAssemblies.RailAnchor anchor
                &&anchor.lines().size()==1&&queriedRails.containsAll(anchor.lines()));
            Vec3 center=Vec3.atBottomCenterOf(base).add(4,1.4,2);
            Vec3 eye=center.add(9,6,-10);
            client.execute(()->{
                ramp=Vec3.atBottomCenterOf(rampStart);
                rail=Vec3.atBottomCenterOf(railAnchor).add(-.5,1.2,-.5);
                view=cameraAt(eye,center);
                checks=report;
            });
        });
    }

    private static InputScene.CameraView cameraAt(Vec3 eye,Vec3 target){
        Vec3 delta=target.subtract(eye);
        return new InputScene.CameraView(eye,(float)Math.toDegrees(Math.atan2(-delta.x,delta.z)),
            (float)-Math.toDegrees(Math.atan2(delta.y,delta.horizontalDistance())));
    }

    private static boolean place(ServerPlayer player,BlockPos floor){
        var hit=new BlockHitResult(Vec3.atCenterOf(floor).add(0,.5,0),Direction.UP,floor,false);
        return player.getMainHandItem().getItem().useOn(new UseOnContext(player,InteractionHand.MAIN_HAND,hit)).consumesAction();
    }
}
