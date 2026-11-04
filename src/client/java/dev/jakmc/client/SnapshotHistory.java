package dev.jakmc.client;

import java.util.ArrayDeque;

/** Bounded authoritative snapshots sampled on the caller's shared render tick. */
public final class SnapshotHistory<T> {
    public record Snapshot<T>(long tick,T value){}
    public record Sample<T>(Snapshot<T> before,Snapshot<T> after,float fraction,float elapsed){}
    private final ArrayDeque<Snapshot<T>> snapshots=new ArrayDeque<>();
    private long session;
    public void accept(long session,long tick,T value){
        if(this.session!=session||!snapshots.isEmpty()&&tick<=snapshots.getLast().tick())snapshots.clear();
        this.session=session;
        snapshots.addLast(new Snapshot<>(tick,value));
        while(snapshots.size()>8)snapshots.removeFirst();
    }
    public Sample<T> sample(double tick){
        if(snapshots.isEmpty()||!Double.isFinite(tick))return null;
        var before=snapshots.getFirst();
        for(var after:snapshots){
            if(after.tick()<=tick){before=after;continue;}
            if(after==before)break;
            float fraction=(float)Math.clamp((tick-before.tick())/(after.tick()-before.tick()),0,1);
            return new Sample<>(before,after,fraction,(float)Math.max(0,(tick-before.tick())*.05));
        }
        // A stalled stream never extrapolates a projectile through a wall.
        return new Sample<>(before,before,0,0);
    }
}
