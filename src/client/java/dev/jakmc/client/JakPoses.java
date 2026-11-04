package dev.jakmc.client;

import com.google.gson.*;
import dev.jakmc.network.JakPackets;
import dev.jakmc.player.JakPlayers;
import net.minecraft.client.Minecraft;
import net.minecraft.world.phys.Vec3;
import net.minecraft.util.Mth;
import org.joml.Quaternionf;
import java.util.*;

public final class JakPoses {
    public record BoardPose(Quaternionf rotation,float lift,float phase,float ahead,float side,float duck,
                            boolean stance,float grindFrame,boolean closing,float age) {
        static BoardPose read(JsonObject json) {
            var q=json.getAsJsonArray("rotation");
            var grind=json.get("grindFrame");
            return new BoardPose(new Quaternionf(q.get(0).getAsFloat(),q.get(1).getAsFloat(),q.get(2).getAsFloat(),q.get(3).getAsFloat()),
                json.get("lift").getAsFloat(),json.get("phase").getAsFloat(),json.get("ahead").getAsFloat(),
                json.get("side").getAsFloat(),json.get("duck").getAsFloat(),json.get("stance").getAsBoolean(),
                grind.isJsonNull()?Float.NaN:grind.getAsFloat(),json.get("closing").getAsBoolean(),json.get("age").getAsFloat());
        }

        BoardPose interpolate(BoardPose from,float fraction) {
            if(from==null||fraction>=1)return this;
            float grind=Float.isFinite(from.grindFrame)&&Float.isFinite(grindFrame)?Mth.lerp(fraction,from.grindFrame,grindFrame):grindFrame;
            return new BoardPose(new Quaternionf(from.rotation).slerp(rotation,fraction),
                Mth.lerp(fraction,from.lift,lift),Mth.lerp(fraction,from.phase,phase),
                Mth.lerp(fraction,from.ahead,ahead),Mth.lerp(fraction,from.side,side),Mth.lerp(fraction,from.duck,duck),
                stance,grind,closing,age);
        }
    }

    public record RootPose(float swimTilt,float lift,float ladderWeight) {
        static final RootPose REST=new RootPose(0,0,0);

        static RootPose read(JsonObject json){
            if(json==null)return REST;
            return new RootPose(json.get("swimTilt").getAsFloat(),json.get("lift").getAsFloat(),
                json.get("ladderWeight").getAsFloat());
        }

        RootPose interpolate(RootPose from,float fraction){
            if(from.equals(this)||fraction>=1)return this;
            return new RootPose(Mth.lerp(fraction,from.swimTilt,swimTilt),
                Mth.lerp(fraction,from.lift,lift),Mth.lerp(fraction,from.ladderWeight,ladderWeight));
        }
    }

    public record Pose(Vec3 position,float yaw,String clip,float frame,boolean board,boolean dark,boolean light,
                       JsonObject raw,long received,BoardPose boardPose,BoardPose boardFrom,RootPose rootPose,RootPose rootFrom,float yawFrom) {
        public float yawAt(float elapsed){
            return yawFrom+Mth.wrapDegrees(yaw-yawFrom)*Math.clamp(elapsed*20,0,1);
        }

        public RootPose rootAt(float elapsed){
            return rootPose.interpolate(rootFrom,Math.clamp(elapsed*20,0,1));
        }

        public BoardPose boardAt(float elapsed) {
            return boardPose==null?null:boardPose.interpolate(boardFrom,Math.clamp(elapsed*20,0,1));
        }
    }
    // Follow server snapshots on a render clock shared by the local avatar and camera.
    // Keep the collision entity at the latest authoritative position.
    private static final JakMotion MOTION=new JakMotion();
    private static JakMotion.Frame motionFrame;
    private static Vec3 presented;
    private static long fallbackTick;
    public static void beginFrame(Minecraft client){
        InputScene.crossingFrame();
        var player=client.player;
        var pose=player==null?null:get(player.getId());
        motionFrame=pose!=null&&dev.jakmc.player.JakPlayers.active(player)&&!player.isSleeping()
            ?MOTION.sample(System.nanoTime()):null;
        presented=motionFrame==null?null:motionFrame.position();
    }
    public static Vec3 presented(){return presented;}
    public static Quaternionf flightRotation(){return motionFrame==null||motionFrame.rotation()==null?null:new Quaternionf(motionFrame.rotation());}
    public static Vec3 flightOffset(){return motionFrame==null?Vec3.ZERO:motionFrame.offset();}
    private static final Map<Integer,Pose> POSES=new HashMap<>();
    public static void receive(JakPackets.Pose p){
        var client=Minecraft.getInstance();if(client.level==null)return;
        JsonObject json=JsonParser.parseString(p.json()).getAsJsonObject();
        Vec3 position=JakPlayers.vector(json.getAsJsonArray("pos")).add(p.originX(),p.originY(),p.originZ());
        long now=System.nanoTime();
        long nextSession=json.get("session").getAsLong();
        BoardPose board=json.get("boardPose") instanceof JsonObject b?BoardPose.read(b):null;
        Pose previous=POSES.get(p.entityId());
        boolean continuous=previous!=null&&previous.raw().get("session").getAsLong()==nextSession;
        var boardFrom=board!=null&&continuous?previous.boardAt((now-previous.received())/1e9f):null;
        var root=RootPose.read(json.getAsJsonObject("rootPose"));
        var rootFrom=continuous?previous.rootAt((now-previous.received())/1e9f):root;
        float yaw=json.get("yaw").getAsFloat();
        float yawFrom=continuous?previous.yawAt((now-previous.received())/1e9f):yaw;
        var pose=new Pose(position,yaw,json.get("clip").getAsString(),json.get("frame").getAsFloat(),
            json.get("board").getAsBoolean(),json.get("dark").getAsBoolean(),json.get("light").getAsBoolean(),json,now,board,boardFrom,root,rootFrom,yawFrom);
        POSES.put(p.entityId(),pose);
        if(client.player!=null&&client.player.getId()==p.entityId()){
            Vec3 offset=Vec3.ZERO;
            Quaternionf rotation=null;
            if(json.get("hellcat") instanceof JsonObject flight){
                offset=JakPlayers.vector(flight.getAsJsonArray("offset"));
                var q=flight.getAsJsonArray("rotation");
                rotation=new Quaternionf(q.get(0).getAsFloat(),q.get(1).getAsFloat(),q.get(2).getAsFloat(),q.get(3).getAsFloat());
            }
            long tick=json.has("serverTick")?json.get("serverTick").getAsLong():++fallbackTick;
            MOTION.accept(tick,new JakMotion.Frame(position,offset,rotation),now,!continuous);
        }
        Vec3 origin=new Vec3(p.originX(),p.originY(),p.originZ());
        dev.jakmc.client.model.JakShotRenderer.receive(p.entityId(),origin,json);
        JakSounds.receive(p.entityId(),origin,json);
        JakEffects.receive(p.entityId(),origin,json);
        InputScene.observeMountEvents(p.entityId(),json);
        if(client.player!=null&&client.player.getId()==p.entityId()&&json.get("hellcat") instanceof JsonObject flight&&flight.has("fired")){
            InputScene.observeHellcatEvent("fire");
        }
        PortScene.observe(p.entityId(),json);
        EconomyScene.observe(p.entityId(),json);
        if(client.player!=null&&client.player.getId()==p.entityId()){
            client.player.setPos(position);client.player.setDeltaMovement(Vec3.ZERO);
            client.player.setOnGround((json.get("status").getAsInt()&2)!=0);
            client.player.setYBodyRot(pose.yaw());
        }
    }
    public static void prune(Minecraft client){
        if(client.level==null)return;long now=System.nanoTime();
        var it=POSES.entrySet().iterator();while(it.hasNext()){var e=it.next();if(now-e.getValue().received()>2_000_000_000L&&client.level.getEntity(e.getKey())==null){dev.jakmc.client.model.JakShotRenderer.remove(e.getKey());it.remove();}}
    }
    public static Pose get(int id){return POSES.get(id);}
    public static Map<Integer,Pose> all(){return java.util.Collections.unmodifiableMap(POSES);}
    public static void clear(){POSES.clear();presented=null;motionFrame=null;MOTION.clear();fallbackTick=0;dev.jakmc.client.model.JakShotRenderer.clear();}
}
