package dev.jakmc.client;

import net.minecraft.world.phys.Vec3;
import org.joml.Quaternionf;

/** Replays packet jitter at 150 Hz, including speeds beyond the former four-block cutoff. */
public final class CameraMotionCheck {
    private static final long TICK=50_000_000L;

    public static void main(String[] args){
        for(double speed:new double[]{6,80,160})verify(speed);
        verifyStop();
        verifyBatchedDelivery();
    }

    private static void verify(double speed){
        var motion=new JakMotion();
        int nextTick=0,frames=0,stalls=0;
        double previous=Double.NaN,minSpeed=Double.POSITIVE_INFINITY,maxSpeed=0;
        for(int frame=0;frame<6000;frame++){
            long now=Math.round(frame*1e9/150);
            while(nextTick<800&&arrival(nextTick)<=now){
                var at=new Vec3(0,0,nextTick*speed/20);
                motion.accept(nextTick,new JakMotion.Frame(at,Vec3.ZERO,new Quaternionf()),arrival(nextTick),nextTick==0);
                nextTick++;
            }
            var shown=motion.sample(now,TICK);
            if(shown==null)continue;
            if(frame>150&&Double.isFinite(previous)){
                double rate=(shown.position().z-previous)*150;
                if(rate<=1e-8)stalls++;
                minSpeed=Math.min(minSpeed,rate);
                maxSpeed=Math.max(maxSpeed,rate);
                frames++;
            }
            previous=shown.position().z;
        }
        if(stalls!=0||minSpeed<speed*.87||maxSpeed>speed*1.13)
            throw new AssertionError("Uneven camera at "+speed+" blocks/s, min="+minSpeed+", max="+maxSpeed+", stalls="+stalls);
        long time=42_000_000_000L;
        var destination=new Vec3(0,0,previous+1);
        motion.accept(900,new JakMotion.Frame(destination,Vec3.ZERO,new Quaternionf()),time,true);
        if(motion.sample(time,TICK).position().distanceToSqr(destination)>1e-12)
            throw new AssertionError("Teleport interpolated across sessions");
        System.out.println("Camera "+speed+" blocks/s, "+frames+" frames, speed "+minSpeed+".."+maxSpeed+", zero stalls, teleport snapped");
    }

    private static long arrival(int tick){
        return tick*50_000_000L+40_000_000L+(tick%5-2)*6_000_000L;
    }

    private static void verifyStop(){
        var motion=new JakMotion();
        int nextTick=0;
        double previous=0;
        for(int frame=0;frame<1200;frame++){
            long now=Math.round(frame*1e9/150);
            while(nextTick<150&&arrival(nextTick)<=now){
                var at=new Vec3(0,0,Math.min(nextTick,60)*8);
                motion.accept(nextTick,new JakMotion.Frame(at,Vec3.ZERO,new Quaternionf()),arrival(nextTick),nextTick==0);
                nextTick++;
            }
            var shown=motion.sample(now,TICK);
            if(shown==null)continue;
            double position=shown.position().z;
            if(position<previous-1e-9||position>480+1e-9)
                throw new AssertionError("Camera crossed a stopped vehicle or moved backwards, "+previous+" -> "+position);
            previous=position;
        }
        if(Math.abs(previous-480)>1e-9)throw new AssertionError("Camera did not settle at the collision");
        System.out.println("Abrupt collision stop at 160 blocks/s, no overshoot or backwards correction");
    }

    private static void verifyBatchedDelivery(){
        var motion=new JakMotion();
        int tick=0;
        double previous=0;
        for(int frame=0;frame<1500;frame++){
            long now=Math.round(frame*1e9/150);
            while((tick/3*3+2)*TICK<=now){
                long received=(tick/3*3+2)*TICK;
                var snapshot=new JakMotion.Frame(new Vec3(tick,0,0),Vec3.ZERO,null);
                motion.accept(tick,snapshot,received,tick==0);
                motion.accept(tick,snapshot,received+1,false);
                tick++;
            }
            var shown=motion.sample(now,TICK);
            if(shown==null)continue;
            double x=shown.position().x;
            if(frame>150&&(x<previous-1e-9||x-previous>.147))
                throw new AssertionError("Batched or repeated packets jumped the camera: "+previous+" -> "+x);
            previous=x;
        }
        if(previous<190)throw new AssertionError("Camera fell behind batched packets");
        System.out.println("Batched packets and duplicate ticks, bounded frame movement");
    }
}
