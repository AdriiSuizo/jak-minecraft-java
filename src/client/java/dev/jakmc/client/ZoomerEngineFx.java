package dev.jakmc.client;

import com.google.gson.JsonObject;
import dev.jakmc.entity.JakVehicle;
import java.util.Map;
import java.util.WeakHashMap;
import net.minecraft.util.Mth;
import net.minecraft.world.phys.Vec3;
import org.joml.Quaternionf;

/**
 * vehicle-method-78 for the zoomers (vehicle-effects.gc, hvehicle-effects.gc): the exhaust's smoke, the thruster
 * flames, the damaged vehicle's smoke and the engine's start, launched once a 60 Hz frame at the vehicle's bone
 * transform of that frame, as the game's own frame launches them. The roll thrusters' flames (h-bike-base) and the
 * scrape's sparks at the point of the impact (vehicle-method-78) go with them.
 * <p>The particles are born where the bike is drawn: the pose is the one the renderer draws (hull, bob and
 * climb easing, attitude) at each 1/60 s of the rendered interval, not the last server tick's (the model is
 * drawn about two ticks behind that, so a particle born there hung metres ahead of a fast bike) and not three
 * times at the same place per tick as before. The numbers per frame are native (jak_core
 * particles::vehicle_effects), from the vehicle's own data.
 */
public final class ZoomerEngineFx {
    private ZoomerEngineFx(){}
    private static final double STEP=1.0/60;
    /** Frames a rendered frame may launch: a long stall does not owe the rest. */
    private static final int MOST=6;
    private static final int RECORD=19;
    private static final class Track{
        long last;
        double owed;
        Vec3 position;
        Quaternionf rotation=new Quaternionf();
        boolean started;
        /** The native emitter has been told the engine is on (and is to be told when it goes off). */
        boolean running;
    }
    private static final Map<JakVehicle,Track> TRACKS=new WeakHashMap<>();

    /** Once per rendered frame of a hover zoomer in view, after its presentation has been sampled. */
    public static void frame(JakVehicle car){
        long profiled=dev.jakmc.entity.ZoomerProfile.begin();
        try{launch(car);}finally{
            if(dev.jakmc.entity.ZoomerProfile.ON){
                dev.jakmc.entity.ZoomerProfile.end("client zoomer engine particles",profiled);
                dev.jakmc.entity.ZoomerProfile.report(System.nanoTime(),"frame rate "+net.minecraft.client.Minecraft.getInstance().getFps());
            }
        }
    }
    private static void launch(JakVehicle car){
        var before=car.presentationBefore();var after=car.presentationAfter();
        if(before==null||after==null||!after.has("ignition")||!before.has("ignition"))return;
        long now=System.nanoTime();
        var track=TRACKS.computeIfAbsent(car,key->new Track());
        // The renderer may extract an entity twice in a frame (shadows, outlines): one launch per instant.
        if(track.started&&now-track.last<1_000_000L)return;
        Vec3 position=car.presentationPosition().add(0,car.presentationLift(),0);
        Quaternionf rotation=car.presentationRotation();
        // A vehicle not drawn for a quarter second (out of view) starts over where it now is.
        boolean first=!track.started||now-track.last>250_000_000L;
        double dt=first?0:Math.min((now-track.last)/1e9,.1);
        double leftover=first?0:track.owed;
        float fraction=car.presentationFraction();
        float thrust=Mth.lerp(fraction,number(before,"thrust"),number(after,"thrust"));
        float hp=Mth.lerp(fraction,number(before,"hp"),number(after,"hp"));
        float roll0=Mth.lerp(fraction,element(before,"rollThrust",0),element(after,"rollThrust",0));
        float roll1=Mth.lerp(fraction,element(before,"rollThrust",1),element(after,"rollThrust",1));
        float scrape=Mth.lerp(fraction,number(before,"scrape"),number(after,"scrape"));
        Vec3 impact=after.has("impactWorld")?lerp(before.has("impactWorld")?before:after,after,"impactWorld",fraction):position;
        boolean ignition=after.get("ignition").getAsBoolean();
        // Nothing comes of a vehicle that is off, whole and not thrusting (vehicle-method-78 launches nothing then);
        // the native emitter hears once that the engine went off, so the next start has its puff.
        boolean active=ignition||thrust>.001f||hp<.75f||roll0>0||roll1>0||scrape>=.75f;
        if(!active&&!track.running){
            track.position=position;
            track.rotation.set(rotation);
            track.last=now;
            track.started=true;
            track.owed=0;
            return;
        }
        track.owed+=dt;
        int count=first?1:(int)Math.min(MOST,Math.floor(track.owed/STEP));
        if(count>0){
            Vec3 velocity=lerp(before,after,"velocity",fraction);
            track.running=active;
            var records=new double[count*RECORD];
            for(int i=0;i<count;i++){
                // The i-th frame of the interval ends (i+1)*STEP-leftover after the last rendered instant.
                double at=dt<=1e-9?1:Math.clamp(((i+1)*STEP-leftover)/dt,0,1);
                Vec3 p=first?position:track.position.lerp(position,at);
                Quaternionf q=first?rotation:new Quaternionf(track.rotation).slerp(rotation,(float)at);
                int base=i*RECORD;
                records[base]=p.x;records[base+1]=p.y;records[base+2]=p.z;
                records[base+3]=q.x;records[base+4]=q.y;records[base+5]=q.z;records[base+6]=q.w;
                records[base+7]=velocity.x;records[base+8]=velocity.y;records[base+9]=velocity.z;
                // ignition, riding (somebody aboard), and, on a vehicle just seen, "as it is".
                records[base+10]=(ignition?3:0)+(first&&i==0?4:0);
                records[base+11]=thrust;
                records[base+12]=hp;
                records[base+13]=roll0;records[base+14]=roll1;
                records[base+15]=scrape;
                records[base+16]=impact.x;records[base+17]=impact.y;records[base+18]=impact.z;
            }
            // vehicle-method-78 / rigid-body-object-method-38 ask for "damage-zaps" and "damage-pops" by name.
            int sounds=JakEffects.vehicle(car.getId(),position,car.model.ordinal()-JakVehicle.Model.BIKEA.ordinal(),records,count);
            if((sounds&1)!=0)car.clientCue("damage-zaps");
            if((sounds&2)!=0)car.clientCue("damage-pops");
            track.owed=first?0:Math.min(track.owed-count*STEP,STEP);
        }
        track.position=position;
        track.rotation.set(rotation);
        track.last=now;
        track.started=true;
    }

    private static float number(JsonObject frame,String key){return frame.has(key)?frame.get(key).getAsFloat():0;}
    private static float element(JsonObject frame,String key,int index){return frame.has(key)?frame.getAsJsonArray(key).get(index).getAsFloat():0;}
    private static Vec3 lerp(JsonObject a,JsonObject b,String key,float fraction){
        if(!a.has(key)||!b.has(key))return Vec3.ZERO;
        var x=a.getAsJsonArray(key);var y=b.getAsJsonArray(key);
        return new Vec3(Mth.lerp(fraction,x.get(0).getAsFloat(),y.get(0).getAsFloat()),
            Mth.lerp(fraction,x.get(1).getAsFloat(),y.get(1).getAsFloat()),
            Mth.lerp(fraction,x.get(2).getAsFloat(),y.get(2).getAsFloat()));
    }
}
