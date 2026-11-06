package dev.jakmc.entity;

import dev.jakmc.JakMod;
import java.util.Arrays;
import java.util.Map;
import java.util.concurrent.ConcurrentHashMap;

/**
 * Opt-in timing of the zoomer's code paths, server and client, for finding what slows a ride down: set
 * {@code JAKMC_ZOOMER_PROFILE=1} (or {@code -Djakmc.zoomerProfile=true}) and ride; every five seconds each
 * section writes one {@code zoomer-profile} line to the game log (calls, mean and worst time, and the share of a
 * second it took). Off, each hook is one constant test.
 */
public final class ZoomerProfile {
    private ZoomerProfile(){}
    public static final boolean ON=System.getenv("JAKMC_ZOOMER_PROFILE")!=null||Boolean.getBoolean("jakmc.zoomerProfile");
    private static final long WINDOW=5_000_000_000L;
    private static final Map<String,long[]> SECTIONS=new ConcurrentHashMap<>();
    private static long reported=System.nanoTime();

    public static long begin(){return ON?System.nanoTime():0;}

    /** Adds the time since {@code start} to {@code section}, and reports every window. */
    public static void end(String section,long start){
        if(!ON)return;
        long now=System.nanoTime();
        var stat=SECTIONS.computeIfAbsent(section,key->new long[3]);
        synchronized(stat){stat[0]+=now-start;stat[1]++;stat[2]=Math.max(stat[2],now-start);}
        report(now,null);
    }

    /** Reports once a window, with {@code extra} (the client adds its frame rate) on the first line. */
    public static void report(long now,String extra){
        if(!ON)return;
        synchronized(ZoomerProfile.class){
            if(now-reported<WINDOW)return;
            double seconds=(now-reported)/1e9;
            reported=now;
            if(extra!=null)JakMod.LOG.info("zoomer-profile {}",extra);
            for(var entry:SECTIONS.entrySet()){
                long[] stat;
                synchronized(entry.getValue()){stat=entry.getValue().clone();Arrays.fill(entry.getValue(),0);}
                if(stat[1]==0)continue;
                JakMod.LOG.info("zoomer-profile {}: {} calls, mean {} ms, worst {} ms, {} ms of every second",entry.getKey(),stat[1],
                    String.format("%.3f",stat[0]/1e6/stat[1]),String.format("%.3f",stat[2]/1e6),String.format("%.2f",stat[0]/1e6/seconds));
            }
        }
    }
}
