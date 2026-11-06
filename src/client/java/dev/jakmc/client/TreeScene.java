package dev.jakmc.client;

import com.google.gson.JsonArray;
import com.google.gson.JsonObject;
import dev.jakmc.JakData;
import dev.jakmc.player.JakPlayers;
import java.io.IOException;
import java.io.UncheckedIOException;
import java.nio.file.Files;
import java.util.ArrayList;
import java.util.Comparator;
import java.util.LinkedHashSet;
import java.util.UUID;
import net.fabricmc.fabric.api.client.event.lifecycle.v1.ClientTickEvents;
import net.minecraft.client.CameraType;
import net.minecraft.client.Minecraft;
import net.minecraft.client.Screenshot;
import net.minecraft.core.BlockPos;
import net.minecraft.server.level.ServerLevel;
import net.minecraft.server.level.ServerPlayer;
import net.minecraft.tags.BlockTags;
import net.minecraft.world.level.GameType;
import net.minecraft.world.level.levelgen.Heightmap;
import net.minecraft.world.phys.Vec3;

/**
 * The trees round a place in a copy of a played world (Adrii's, 06/11): each trunk top near it, the blocks round
 * that top, and Jak's double jump at it from its four sides with the ledge probe's verdict (JAKMC_LEDGE_TRACE).
 */
final class TreeScene {
    private static boolean running,pause;
    private static int tick;
    private static UUID owner;
    private static Vec3 origin;
    private static InputScene.CameraView view;
    private static Runnable restore;
    private static final JsonObject result=new JsonObject();
    private static final JsonArray rows=new JsonArray();
    private static final LinkedHashSet<String> states=new LinkedHashSet<>();
    private static final ArrayList<String> whys=new ArrayList<>();
    private static double peak,start,after;
    private static int grabAt;
    /** A jump: the trunk's top cell, where his feet start and his yaw toward it. */
    private record Jump(BlockPos top,Vec3 from,float yaw,String side){}
    private static final ArrayList<Jump> jumps=new ArrayList<>();
    private static final int SCAN=60,JUMP=70;
    static InputScene.CameraView camera(){return view;}
    static void initialize(){ClientTickEvents.END_CLIENT_TICK.register(TreeScene::tick);}

    private static void look(Vec3 eye,Vec3 target){
        var d=target.subtract(eye);
        view=new InputScene.CameraView(eye,(float)Math.toDegrees(Math.atan2(-d.x,d.z)),(float)-Math.toDegrees(Math.atan2(d.y,d.horizontalDistance())));
    }
    private static void capture(Minecraft c,String file){Screenshot.grab(c.gameDirectory,file,c.gameRenderer.mainRenderTarget(),1,_->{});}

    static void start(Minecraft client,Vec3 at){
        if(client.player==null||client.getSingleplayerServer()==null)throw new IllegalStateException("Open the development world");
        owner=client.player.getUUID();tick=0;origin=at;result.entrySet().clear();jumps.clear();
        while(!rows.isEmpty())rows.remove(0);
        pause=client.options.pauseOnLostFocus;client.options.pauseOnLostFocus=false;
        client.options.setCameraType(CameraType.THIRD_PERSON_BACK);
        onServer(client,p->{
            var old=p.position();var mode=p.gameMode.getGameModeForPlayer();
            boolean enabled=p.getAttachedOrCreate(JakData.ENABLED);
            p.setGameMode(GameType.CREATIVE);p.setAttached(JakData.ENABLED,true);
            p.teleportTo(at.x,at.y,at.z);
            JakPlayers.controlForProbe(p,true);JakPlayers.hostControlsForProbe(p);
            restore=()->{JakPlayers.controlForProbe(p,false);p.setAttached(JakData.ENABLED,enabled);p.setGameMode(mode);p.teleportTo(old.x,old.y,old.z);};
            client.execute(()->running=true);
        });
    }

    /** The trunk tops within 12 blocks: a log with two logs under it and nothing solid on it. */
    private static void scan(ServerLevel level){
        var tops=new ArrayList<BlockPos>();var centre=BlockPos.containing(origin);
        for(var pos:BlockPos.betweenClosed(centre.offset(-12,-8,-12),centre.offset(12,16,12))){
            var state=level.getBlockState(pos);
            if(!state.is(BlockTags.LOGS)||!level.getBlockState(pos.below()).is(BlockTags.LOGS)||!level.getBlockState(pos.below(2)).is(BlockTags.LOGS))continue;
            if(!level.getBlockState(pos.above()).getCollisionShape(level,pos.above()).isEmpty())continue;
            tops.add(pos.immutable());
        }
        tops.sort(Comparator.comparingDouble(t->t.distToCenterSqr(origin)));
        var found=new JsonArray();
        for(var top:tops.subList(0,Math.min(5,tops.size()))){
            var row=new JsonObject();row.addProperty("top",top.toShortString());
            // The cells round it, layer by layer from 5 under to 4 over: L log, F leaves, # other solid, . air.
            var layers=new JsonArray();
            for(int h=4;h>=-5;h--){
                var line=new StringBuilder(h+": ");
                for(int z=-2;z<=2;z++){for(int x=-2;x<=2;x++){
                    var cell=top.offset(x,h,z);var s=level.getBlockState(cell);
                    line.append(s.is(BlockTags.LOGS)?'L':s.is(BlockTags.LEAVES)?'F':!s.getCollisionShape(level,cell).isEmpty()?'#':'.');
                }line.append(' ');}
                layers.add(line.toString());
            }
            row.add("cells_x_across_z_down",layers);
            int ground=level.getHeight(Heightmap.Types.MOTION_BLOCKING_NO_LEAVES,top.getX(),top.getZ());
            row.addProperty("top_over_ground_beside",top.getY()+1-level.getHeight(Heightmap.Types.MOTION_BLOCKING_NO_LEAVES,top.getX()+1,top.getZ()));
            row.addProperty("block_name",level.getBlockState(top).getBlock().getDescriptionId());
            found.add(row);
            if(jumps.size()<16)for(int side=0;side<4;side++){
                int dx=side==0?1:side==1?-1:0,dz=side==2?1:side==3?-1:0;
                double fx=top.getX()+.5+dx*3.9,fz=top.getZ()+.5+dz*3.9;
                int feet=level.getHeight(Heightmap.Types.MOTION_BLOCKING_NO_LEAVES,(int)Math.floor(fx),(int)Math.floor(fz));
                if(top.getY()+1-feet<2||top.getY()+1-feet>6)continue;
                float yaw=(float)Math.toDegrees(Math.atan2(dx,-dz));
                jumps.add(new Jump(top,new Vec3(fx,feet,fz),yaw,side==0?"east":side==1?"west":side==2?"south":"north"));
            }
            row.addProperty("ground_at_top_column",ground);
        }
        result.add("tops",found);result.addProperty("tops_found",tops.size());
    }

    private static void tick(Minecraft c){
        if(!running)return;
        int t=++tick;
        c.gui.setScreen(null);
        var pose=JakPoses.get(c.player.getId());
        String state=pose==null?"":pose.raw().get("state").getAsString();
        double feet=pose==null?0:pose.position().y;
        if(t==SCAN)onServer(c,p->scan(p.level()));
        if(t==SCAN+2)capture(c,"trees-start.png");
        int k=t-SCAN-10;
        if(k>=0&&k<jumps.size()*JUMP){
            var jump=jumps.get(k/JUMP);int s=k%JUMP;
            if(s==0){states.clear();whys.clear();peak=-1e9;start=jump.from.y;grabAt=-1;after=-1e9;
                onServer(c,p->{JakPlayers.probeInput(p,0,0,0);p.teleportTo(jump.from.x,jump.from.y,jump.from.z);p.setYRot(jump.yaw);p.setXRot(0);p.setDeltaMovement(Vec3.ZERO);});
                Vec3 top=Vec3.atBottomCenterOf(jump.top);double dx=-Math.sin(Math.toRadians(jump.yaw)),dz=Math.cos(Math.toRadians(jump.yaw));
                look(new Vec3(jump.from.x-dz*5-dx*2,top.y+1.5,jump.from.z+dx*5-dz*2),top);}
            if(s==4)c.player.setYRot(jump.yaw);
            if(s==6)onServer(c,p->{p.setYRot(jump.yaw);JakPlayers.probeInput(p,0,0,1);});
            if(s==9)onServer(c,p->JakPlayers.probeInput(p,1,1,1));
            if(s==14)onServer(c,p->JakPlayers.probeInput(p,0,0,1));
            if(s==16)onServer(c,p->JakPlayers.probeInput(p,1,1,1));
            // Hanging: X a half second later, the climb up onto the top.
            if(grabAt<0&&state.startsWith("edge-grab")){grabAt=s;capture(c,"trees-"+k/JUMP+"-"+jump.side+"-hang.png");}
            if(grabAt>=0&&s==grabAt+10)onServer(c,p->JakPlayers.probeInput(p,1,1,0));
            if(grabAt>=0&&s==grabAt+14)onServer(c,p->JakPlayers.probeInput(p,0,0,0));
            if(grabAt>=0&&s>grabAt+10)after=Math.max(after,feet);
            if(s>=6&&s<60){if(!state.isEmpty())states.add(state);peak=Math.max(peak,feet);
                if(pose!=null&&pose.raw().has("ledgeWhy")){String why=pose.raw().get("ledgeWhy").getAsString();
                    if(!why.isEmpty()&&whys.size()<40)whys.add(s+" "+state+" feet "+Math.round((feet-start)*100)/100.0+" "+why);}}
            if(s==30)capture(c,"trees-"+k/JUMP+"-"+jump.side+".png");
            if(s==60&&grabAt<0)onServer(c,p->JakPlayers.probeInput(p,0,0,0));
            if(s==JUMP-1){
                var row=new JsonObject();row.addProperty("top",jump.top.toShortString());row.addProperty("side",jump.side);
                row.addProperty("top_over_feet",jump.top.getY()+1-jump.from.y);
                row.addProperty("grabbed",states.stream().anyMatch(v->v.startsWith("edge-grab")));
                row.addProperty("peak_feet",Math.round((peak-start)*100)/100.0);
                if(grabAt>=0){row.addProperty("feet_after_climb_over_top",Math.round((after-jump.top.getY()-1)*100)/100.0);row.addProperty("end_state",state);}
                var list=new JsonArray();states.forEach(list::add);row.add("states",list);
                var why=new JsonArray();whys.forEach(why::add);row.add("ledge_why",why);
                rows.add(row);
            }
        }
        if(t>SCAN+10&&k==jumps.size()*JUMP+2){
            view=null;
            result.add("jumps",rows);
            result.addProperty("visual_review","required");result.addProperty("reason","complete");
            onServer(c,p->{
                try{
                    var dir=c.gameDirectory.toPath().resolve("jakmc/evidence/trees");
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
