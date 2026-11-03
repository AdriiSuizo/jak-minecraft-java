package dev.jakmc.client;

import com.google.gson.*;
import dev.jakmc.network.JakPackets;
import dev.jakmc.player.JakPlayers;
import net.minecraft.client.Minecraft;
import net.minecraft.world.phys.Vec3;
import java.util.*;

public final class JakPoses {
    public record Pose(Vec3 position,float yaw,String clip,float frame,boolean board,boolean dark,boolean light,JsonObject raw,long received){}
    // Follow server snapshots on a render clock shared by the local avatar and camera.
    // Keep the collision entity at the latest authoritative position.
    private static Vec3 from, presented;
    private static long session = -1;
    private static Vec3 sample(Pose pose,long now){
        return from.lerp(pose.position(),Math.clamp((now-pose.received())/50_000_000.0,0,1));
    }
    public static void beginFrame(Minecraft client){
        var player=client.player;
        var pose=player==null?null:get(player.getId());
        presented=pose!=null&&from!=null&&dev.jakmc.player.JakPlayers.active(player)&&!player.isSleeping()
            ?sample(pose,System.nanoTime()):null;
    }
    public static Vec3 presented(){return presented;}
    private static final Map<Integer,Pose> POSES=new HashMap<>();
    public static void receive(JakPackets.Pose p){
        var client=Minecraft.getInstance();if(client.level==null)return;
        JsonObject json=JsonParser.parseString(p.json()).getAsJsonObject();
        Vec3 position=JakPlayers.vector(json.getAsJsonArray("pos")).add(p.originX(),p.originY(),p.originZ());
        var pose=new Pose(position,json.get("yaw").getAsFloat(),json.get("clip").getAsString(),json.get("frame").getAsFloat(),json.get("board").getAsBoolean(),json.get("dark").getAsBoolean(),json.get("light").getAsBoolean(),json,System.nanoTime());
        Pose previous=POSES.put(p.entityId(),pose);
        if(client.player!=null&&client.player.getId()==p.entityId()){
            long nextSession=json.get("session").getAsLong();
            from=previous==null||from==null||session!=nextSession||previous.position().distanceToSqr(position)>16
                ?position:sample(previous,pose.received());
            session=nextSession;
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
    public static void clear(){POSES.clear();from=presented=null;session=-1;dev.jakmc.client.model.JakShotRenderer.clear();}
}
