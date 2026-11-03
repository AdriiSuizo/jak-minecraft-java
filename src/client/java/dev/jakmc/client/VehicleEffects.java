package dev.jakmc.client;

import com.google.gson.Gson;
import com.google.gson.JsonArray;
import com.google.gson.JsonObject;
import dev.jakmc.entity.JakVehicle;
import dev.jakmc.player.JakPlayers;
import java.util.HashMap;
import java.util.Map;
import java.util.WeakHashMap;
import net.fabricmc.fabric.api.client.event.lifecycle.v1.ClientTickEvents;
import net.minecraft.world.phys.Vec3;
import org.joml.Quaternionf;
import org.joml.Vector3f;

/** Original emitters stay on the fixed Jak effects clock, independent of the render frame rate. */
final class VehicleEffects {
    private static final float[] BASIS={1,0,0,0,1,0,0,0,1};
    private static final Gson JSON=new Gson();
    private static final Map<JakVehicle,JsonObject> SEEN=new WeakHashMap<>();
    private static final Map<dev.jakmc.entity.JakEnemy,String> GUARD_FRAMES=new WeakHashMap<>();
    private static final Map<JakVehicle,Map<Integer,Vec3>> GRENADE_LAST=new WeakHashMap<>();
    private static final double METRE=.64;

    static void initialize(){
        ClientTickEvents.END_CLIENT_TICK.register(client->{
            if(client.level==null||client.player==null)return;
            for(var entity:client.level.entitiesForRendering()){
                if(entity instanceof JakVehicle car&&car.distanceToSqr(client.player)<96*96){
                    var frame=car.frame();
                    if(frame!=null&&SEEN.put(car,frame)!=frame)emit(car,frame);
                }else if(entity instanceof dev.jakmc.entity.JakEnemy guard&&guard.kind.guard()){
                    String shot=guard.guardShot();
                    if(!shot.equals(GUARD_FRAMES.put(guard,shot))&&!shot.isEmpty())guardShots(guard,shot);
                }
            }
        });
    }

    private static void emit(JakVehicle car,JsonObject frame){
        var commands=new JsonArray();
        var q=frame.getAsJsonArray("rotation");
        var rotation=new Quaternionf(q.get(0).getAsFloat(),q.get(1).getAsFloat(),q.get(2).getAsFloat(),q.get(3).getAsFloat());
        var origin=car.position();
        var previous=GRENADE_LAST.getOrDefault(car,Map.of());
        var live=new HashMap<Integer,Vec3>();
        for(var entry:frame.getAsJsonArray("grenades")){
            var shot=entry.getAsJsonObject();
            int id=shot.get("id").getAsInt();
            Vec3 point=JakPlayers.vector(shot.getAsJsonArray("pos"));
            live.put(id,origin.add(point));
            commands.add(command("Attached",id,"group-guard-grenade",units(point),BASIS));
            var last=previous.get(id);
            if(last!=null){
                Vec3 trail=origin.add(point).subtract(last);
                if(trail.lengthSqr()>.00001)commands.add(command("BeamSized",975,units(last.subtract(origin)),
                    vector(trail.normalize()),trail.length()*6400,1200));
            }
        }
        for(int id:previous.keySet())if(!live.containsKey(id))commands.add(command("Stop",id));
        if(!previous.isEmpty()||!live.isEmpty())GRENADE_LAST.put(car,live);
        for(var entry:frame.getAsJsonArray("gunEffects")){
            var effect=entry.getAsJsonObject();
            String name=effect.get("effect").getAsString();
            Vec3 at=JakPlayers.vector(effect.getAsJsonArray("pos"));
            if(name.equals("beam")){
                Vec3 path=JakPlayers.vector(effect.getAsJsonArray("end")).subtract(at);
                for(int part:new int[]{975,978})commands.add(command("Beam",part,units(at),vector(path.normalize()),path.length()*6400));
            }else commands.add(command("Spawn",name,units(at),BASIS));
        }
        boolean engine=car.getFirstPassenger()!=null;
        float thrust=frame.get("throttle").getAsFloat();
        for(int sub=0;sub<3;sub++){
            for(var entry:frame.getAsJsonArray("emitters")){
                var emitter=entry.getAsJsonArray();
                var overrides=Map.of("num",emitter.get(2).getAsFloat(),"alpha",new int[]{32,32},"scale",new int[]{8192,8192});
                commands.add(command("Var",emitter.get(0).getAsInt(),
                    units(JakPlayers.vector(emitter.get(1).getAsJsonArray()).scale(METRE)),BASIS,overrides,0));
            }
            if(engine){
                for(var point:frame.getAsJsonArray("exhaust")){
                    var local=rotated(rotation,JakPlayers.vector(point.getAsJsonArray())).scale(METRE);
                    var overrides=Map.of("num",.8f+thrust,"alpha",new float[]{16*(1-thrust*thrust),48*(1-thrust*thrust)},
                        "vel",vector(rotated(rotation,new Vec3(0,0,-42))));
                    commands.add(command("Var",923,units(local),BASIS,overrides,0));
                }
            }
            if(frame.get("boost").getAsBoolean()||frame.has("thrusterPower")&&frame.get("thrusterPower").getAsFloat()>0){
                boolean toad=car.model==JakVehicle.Model.TOAD;
                double power=frame.has("thrusterPower")?frame.get("thrusterPower").getAsFloat():thrust;
                double length=(toad?6:2)*4096*power;
                double width=Math.min((toad?1:.6)*4096,length/2);
                var back=vector(rotated(rotation,new Vec3(0,0,-1)));
                for(var point:frame.getAsJsonArray("thrusters")){
                    var local=rotated(rotation,JakPlayers.vector(point.getAsJsonArray())).scale(METRE);
                    for(int blade=0;blade<3;blade++){
                        double angle=Math.toRadians(30+60*blade);
                        var axis=vector(rotated(rotation,new Vec3(-Math.sin(angle),Math.cos(angle),0)));
                        commands.add(command("Blade",922,units(local),back,axis,length,width));
                    }
                }
            }
            commands.add(command("Step",1f/60));
        }
        if(commands.size()>3)JakEffects.local(car.getId(),origin,commands);
    }

    private static void guardShots(dev.jakmc.entity.JakEnemy guard,String frame){
        var commands=new JsonArray();
        Vec3 origin=guard.position();
        for(var entry:com.google.gson.JsonParser.parseString(frame).getAsJsonArray()){
            var shot=entry.getAsJsonObject();
            Vec3 tail=JakPlayers.vector(shot.getAsJsonArray("tail")).subtract(origin);
            Vec3 head=JakPlayers.vector(shot.getAsJsonArray("head")).subtract(origin);
            Vec3 direction=JakPlayers.vector(shot.getAsJsonArray("direction"));
            boolean hit=shot.get("hit").getAsBoolean();
            commands.add(command("Beam",hit?850:854,units(tail),vector(direction),head.distanceTo(tail)*6400));
            if(hit){
                Vec3 flash=tail.lerp(head,.8);
                commands.add(command("Launch",851,units(flash),BASIS));
                commands.add(command("Launch",852,units(flash),BASIS));
                Vec3 side=new Vec3(0,1,0).cross(direction).normalize();
                if(side.lengthSqr()==0)side=new Vec3(1,0,0);
                Vec3 up=direction.cross(side);
                double[] impactBasis={side.x,side.y,side.z,up.x,up.y,up.z,direction.x,direction.y,direction.z};
                commands.add(command("Launch",853,units(head),impactBasis));
                String group=shot.get("actor").getAsBoolean()?"group-guard-shot-hit-object":"group-guard-shot-hit";
                commands.add(command("Spawn",group,units(head.subtract(direction.scale(.32))),BASIS));
            }else commands.add(command("Launch",855,units(tail.add(direction.scale(.32))),BASIS));
        }
        for(int i=0;i<3;i++)commands.add(command("Step",1f/60));
        JakEffects.local(guard.getId(),origin,commands);
    }

    private static Vec3 rotated(Quaternionf rotation,Vec3 point){
        Vector3f v=rotation.transform(point.toVector3f());
        return new Vec3(v.x,v.y,v.z);
    }
    private static double[] units(Vec3 point){return vector(point.scale(6400));}
    private static double[] vector(Vec3 point){return new double[]{point.x,point.y,point.z};}
    private static JsonObject command(String name,Object... values){
        var data=new JsonObject();
        data.add(name,JSON.toJsonTree(name.equals("Step")||name.equals("Stop")?values[0]:values));
        return data;
    }
}
