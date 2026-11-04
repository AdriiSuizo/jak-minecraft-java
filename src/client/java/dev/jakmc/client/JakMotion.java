package dev.jakmc.client;

import java.util.ArrayDeque;
import net.minecraft.world.phys.Vec3;
import org.joml.Quaternionf;

/** Server snapshots sampled by a continuous render clock with a bounded packet-jitter interval. */
final class JakMotion {
    record Frame(Vec3 position,Vec3 offset,Quaternionf rotation) {
        Frame between(Frame next,double fraction){
            var attitude=rotation==null||next.rotation==null?next.rotation:
                new Quaternionf(rotation).slerp(next.rotation,(float)Math.clamp(fraction,0,1));
            return new Frame(position.lerp(next.position,fraction),offset.lerp(next.offset,Math.clamp(fraction,0,1)),attitude);
        }
    }
    private record Snapshot(long tick,Frame frame){}
    private final ArrayDeque<Snapshot> samples=new ArrayDeque<>();
    private long received,lastFrame;
    private double clock;
    private double tickNanos=50_000_000;

    void accept(long tick,Frame frame,long now,boolean reset){
        if(reset||samples.isEmpty()||tick<=samples.getLast().tick){
            samples.clear();
            clock=tick-2;
            lastFrame=now;
            tickNanos=50_000_000;
        }else{
            var latest=samples.getLast();
            double interval=(now-received)/(double)(tick-latest.tick);
            tickNanos+=(Math.clamp(interval,1_000_000,250_000_000)-tickNanos)*.1;
        }
        samples.addLast(new Snapshot(tick,frame));
        while(samples.size()>8)samples.removeFirst();
        received=now;
    }

    Frame sample(long now){
        if(samples.isEmpty())return null;
        var latest=samples.getLast();
        double dt=Math.max(0,(now-lastFrame)/tickNanos);
        lastFrame=now;
        double target=latest.tick-2+Math.clamp((now-received)/tickNanos,0,1);
        double predicted=clock+dt;
        clock=Math.min(latest.tick,predicted+Math.clamp((target-predicted)*.1,-.1,.1)*dt);
        var left=samples.getFirst();
        if(clock<=left.tick)return left.frame;
        for(var right:samples){
            if(right.tick<=left.tick)continue;
            if(clock<=right.tick)return left.frame.between(right.frame,(clock-left.tick)/(right.tick-left.tick));
            left=right;
        }
        return latest.frame;
    }

    double tick(){return clock;}

    void clear(){samples.clear();}
}
