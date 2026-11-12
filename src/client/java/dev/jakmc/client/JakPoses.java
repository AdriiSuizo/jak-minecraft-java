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
                grind==null||grind.isJsonNull()?Float.NaN:grind.getAsFloat(),json.get("closing").getAsBoolean(),json.get("age").getAsFloat());
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
            return yawFrom+Mth.wrapDegrees(yaw-yawFrom)*Math.clamp(1+elapsed*20,0,1);
        }

        public RootPose rootAt(float elapsed){
            return rootPose.interpolate(rootFrom,Math.clamp(1+elapsed*20,0,1));
        }

        public BoardPose boardAt(float elapsed) {
            return boardPose==null?null:boardPose.interpolate(boardFrom,Math.clamp(1+elapsed*20,0,1));
        }
    }
    // Follow server snapshots on a render clock shared by the local avatar and camera.
    // Keep the collision entity at the latest authoritative position.
    private static final JakMotion MOTION=new JakMotion();
    private static JakMotion.Frame motionFrame;
    private static Vec3 presented;
    private static Pose presentedPose;
    private static float presentedElapsed;
    private static final NavigableMap<Long,Pose> HISTORY=new TreeMap<>();
    private static long fallbackTick;
    public static void beginFrame(Minecraft client){
        InputScene.crossingFrame();
        var player=client.player;
        var pose=player==null?null:get(player.getId());
        motionFrame=pose!=null&&dev.jakmc.player.JakPlayers.active(player)&&!player.isSleeping()
            ?MOTION.sample(System.nanoTime(),player.level().tickRateManager().millisecondsPerTick()*1_000_000d):null;
        presented=motionFrame==null?null:motionFrame.position();
        presentedPose=null;presentedElapsed=0;
        if(motionFrame!=null){
            var sample=HISTORY.ceilingEntry((long)Math.ceil(MOTION.tick()));
            if(sample==null)sample=HISTORY.lastEntry();
            if(sample!=null){presentedPose=sample.getValue();presentedElapsed=(float)Math.clamp((MOTION.tick()-sample.getKey())*.05,-.1,0);}
        }
        if(player!=null&&player.getVehicle() instanceof dev.jakmc.entity.JakVehicle car)car.samplePresentation(System.nanoTime(),true);
    }
    public static Pose rendered(int id){var movie=MovieMotion.pose(id);if(movie!=null)return movie;return presentedPose!=null&&Minecraft.getInstance().player!=null&&id==Minecraft.getInstance().player.getId()?presentedPose:get(id);}
    public static float elapsed(Pose pose){
        if(pose.received()==0)return 0;
        if(pose==presentedPose)return presentedElapsed;
        // Replay Mod can advance three exported frames in a few milliseconds, or take much longer.
        // Use the replay tick fraction shared with the entity position instead of elapsed wall time.
        if(inReplay())return (Minecraft.getInstance().getDeltaTracker().getGameTimeDeltaPartialTick(false)-1)*.05f;
        return Math.clamp((System.nanoTime()-pose.received())/1e9f-.05f,-.05f,.05f);
    }
    /** Same render timestamp used for local body, held items and camera. */
    public static double renderedTick(int entity){
        double movie=MovieMotion.renderedTick(entity);
        if(Double.isFinite(movie))return movie;
        var client=Minecraft.getInstance();var pose=get(entity);
        if(pose==null)return Double.NaN;
        if(client.player!=null&&entity==client.player.getId()&&motionFrame!=null)return MOTION.tick();
        return pose.raw().get("serverTick").getAsLong()+elapsed(pose)*20;
    }
    public static Vec3 presented(){return presented;}
    public static Quaternionf flightRotation(){return motionFrame==null||motionFrame.rotation()==null?null:new Quaternionf(motionFrame.rotation());}
    public static Vec3 flightOffset(){return motionFrame==null?Vec3.ZERO:motionFrame.offset();}
    private static final Map<Integer,Pose> POSES=new HashMap<>();
    /** Bodies Dark Jak's lightning is on (jak-core dark_shock tints), till when (nanoTime). */
    private static final java.util.Map<Integer,Long> DARK_VICTIMS=new java.util.concurrent.ConcurrentHashMap<>();
    /**
     * Jak 3 enemy.gc's dark colour effect this frame, as GltfModel.colourNext takes it: colour-mult (1, 1-f, 1)
     * with f a fresh 0.2..1 each frame; white for a body it is not on. stand-in: the colour-emissive
     * (0.3f, 0, 0.3f) is not drawn (the Rust did not either).
     */
    public static int darkTint(int entityId){
        Long until=DARK_VICTIMS.get(entityId);
        if(until==null)return 0xffffff;
        if(until<System.nanoTime()){DARK_VICTIMS.remove(entityId);return 0xffffff;}
        float f=.2f+.8f*java.util.concurrent.ThreadLocalRandom.current().nextFloat();
        return 0xff00ff|Math.round((1-f)*255)<<8;
    }
    public static void receive(JakPackets.Pose p){
        var client=Minecraft.getInstance();if(client.level==null)return;
        JsonObject json=JsonParser.parseString(p.json()).getAsJsonObject();
        Vec3 position=JakPlayers.vector(json.getAsJsonArray("pos")).add(p.originX(),p.originY(),p.originZ());
        long now=System.nanoTime();
        long nextSession=json.get("session").getAsLong();
        BoardPose board=json.get("boardPose") instanceof JsonObject b?BoardPose.read(b):null;
        Pose previous=POSES.get(p.entityId());
        boolean continuous=previous!=null&&previous.raw().get("session").getAsLong()==nextSession;
        float fromElapsed=inReplay()?0:continuous?(now-previous.received())/1e9f:0;
        var boardFrom=board!=null&&continuous?previous.boardAt(fromElapsed):null;
        var root=RootPose.read(json.getAsJsonObject("rootPose"));
        var rootFrom=continuous?previous.rootAt(fromElapsed):root;
        float yaw=json.get("yaw").getAsFloat();
        float yawFrom=continuous?previous.yawAt(fromElapsed):yaw;
        var pose=new Pose(position,yaw,json.get("clip").getAsString(),json.get("frame").getAsFloat(),
            json.get("board").getAsBoolean(),json.get("dark").getAsBoolean(),json.get("light").getAsBoolean(),json,now,board,boardFrom,root,rootFrom,yawFrom);
        POSES.put(p.entityId(),pose);
        MovieMotion.received(p.entityId(),pose);
        if(json.get("darkVictims") instanceof com.google.gson.JsonArray victims)for(var v:victims)DARK_VICTIMS.put(v.getAsJsonArray().get(0).getAsInt(),now+150_000_000L);
        if(client.player!=null&&client.player.getId()==p.entityId()){
            Vec3 offset=Vec3.ZERO;
            Quaternionf rotation=null;
            if(json.get("hellcat") instanceof JsonObject flight){
                offset=JakPlayers.vector(flight.getAsJsonArray("offset"));
                var q=flight.getAsJsonArray("rotation");
                rotation=new Quaternionf(q.get(0).getAsFloat(),q.get(1).getAsFloat(),q.get(2).getAsFloat(),q.get(3).getAsFloat());
            }
            long tick=json.has("serverTick")?json.get("serverTick").getAsLong():++fallbackTick;
            if(!continuous)HISTORY.clear();
            HISTORY.put(tick,pose);while(HISTORY.size()>8)HISTORY.pollFirstEntry();
            MOTION.accept(tick,new JakMotion.Frame(position,offset,rotation),now,!continuous);
        }
        Vec3 origin=new Vec3(p.originX(),p.originY(),p.originZ());
        dev.jakmc.client.model.JakShotRenderer.receive(p.entityId(),origin,json);
        JakSounds.receive(p.entityId(),origin,json);
        JakEffects.receive(p.entityId(),origin,json);
        if(client.player!=null&&client.player.getId()==p.entityId())for(var entry:json.getAsJsonArray("events")){
            var event=entry.getAsJsonObject();
            if(event.get("kind").getAsString().equals("hurt"))JakInput.hurtFeedback(event.get("buzz").getAsFloat());
        }
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
        boolean replay=inReplay();
        var it=POSES.entrySet().iterator();while(it.hasNext()){var e=it.next();if(now-e.getValue().received()>2_000_000_000L&&client.level.getEntity(e.getKey())==null){if(replay)STALE.put(e.getKey(),e.getValue());dev.jakmc.client.model.JakShotRenderer.remove(e.getKey());it.remove();}}
    }
    public static Pose get(int id){
        var pose=POSES.get(id);
        return pose!=null||!inReplay()?pose:STALE.get(id);
    }
    /**
     * Adrii 08/11: in a Replay Mod replay a jump along its timeline makes the world again and he stood there as a
     * plain Minecraft player till his next pose came (none while it is paused). There, each body keeps the last pose
     * it had (the replay keeps their ids); out of a replay nothing changes.
     */
    private static final Map<Integer,Pose> STALE=new HashMap<>();
    public static boolean inReplay(){
        var player=Minecraft.getInstance().player;
        return player!=null&&player.getClass().getName().startsWith("com.replaymod");
    }
    public static Map<Integer,Pose> all(){return java.util.Collections.unmodifiableMap(POSES);}
    static void clearReplayFallback(){STALE.clear();}
    public static void clear(){if(inReplay())STALE.putAll(POSES);else clearReplayFallback();POSES.clear();HISTORY.clear();presentedPose=null;presented=null;motionFrame=null;MOTION.clear();fallbackTick=0;dev.jakmc.client.model.JakShotRenderer.clear();}
}
