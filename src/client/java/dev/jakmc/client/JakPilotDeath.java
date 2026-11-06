package dev.jakmc.client;

import dev.jakmc.entity.JakVehicle;
import dev.jakmc.network.JakPackets;
import java.util.ArrayList;
import java.util.HashMap;
import java.util.List;
import java.util.Map;
import net.fabricmc.fabric.api.client.event.lifecycle.v1.ClientTickEvents;
import net.fabricmc.fabric.api.client.networking.v1.ClientPlayConnectionEvents;
import net.fabricmc.fabric.api.client.networking.v1.ClientPlayNetworking;
import net.minecraft.world.phys.Vec3;

/**
 * pilot-states.gc target-pilot-death, as it looks: a pilot struck dead on his vehicle ('explode, 'melt, 'endlessfall). For the first
 * two he waits 0.2 s, then the sound ("explosion", "death-melt") and the burst (group-explode-death, group-lava-death) are at
 * his place, his collision goes and he is no longer drawn ({@code ja-channel-set! 0}) for the 1.8 s that follow; the fall plays
 * "death-fall" at once. The death itself is Minecraft's (JakVehicle.riderHit).
 */
public final class JakPilotDeath {
    private JakPilotDeath(){}
    private record Pending(long due,int mode,Vec3 at,int entity){}
    private static final List<Pending> PENDING=new ArrayList<>();
    /** The tick each such pilot stopped being drawn. */
    private static final Map<Integer,Long> GONE=new HashMap<>();
    /** (suspend-for (seconds 0.2)) and (seconds 1.8), in ticks. */
    private static final int WAIT=4,GONE_FOR=36;

    static void initialize(){
        ClientPlayNetworking.registerGlobalReceiver(JakPackets.PilotDeath.TYPE,(packet,context)->{
            var level=context.client().level;
            if(level==null||!level.dimension().identifier().equals(packet.dimension()))return;
            switch(packet.mode()){
                case JakPackets.PilotDeath.EXPLODE,JakPackets.PilotDeath.MELT->PENDING.add(new Pending(level.getGameTime()+WAIT,packet.mode(),packet.at(),packet.entity()));
                case JakPackets.PilotDeath.ENDLESS_FALL->JakVehicle.cueAt(level,packet.at(),"death-fall");
                default->{}
            }
        });
        ClientTickEvents.END_CLIENT_TICK.register(client->{
            var level=client.level;
            if(level==null){PENDING.clear();GONE.clear();return;}
            long now=level.getGameTime();
            for(var it=PENDING.iterator();it.hasNext();){
                var death=it.next();
                if(death.due>now)continue;
                it.remove();
                boolean explode=death.mode==JakPackets.PilotDeath.EXPLODE;
                JakVehicle.cueAt(level,death.at,explode?"explosion":"death-melt");
                JakEffects.burst(explode?JakEffects.EXPLODE_DEATH:JakEffects.LAVA_DEATH,death.at);
                GONE.put(death.entity,now);
            }
            GONE.values().removeIf(since->now-since>GONE_FOR);
        });
        ClientPlayConnectionEvents.DISCONNECT.register((handler,client)->{PENDING.clear();GONE.clear();});
    }
    /** Whether this player is out of sight, the burst having taken him. */
    public static boolean hidden(int entity){return GONE.containsKey(entity);}
}
