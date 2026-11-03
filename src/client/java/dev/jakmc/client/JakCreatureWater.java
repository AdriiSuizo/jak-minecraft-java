package dev.jakmc.client;

import com.google.gson.*;
import dev.jakmc.entity.*;
import net.fabricmc.fabric.api.client.event.lifecycle.v1.ClientTickEvents;
import net.fabricmc.fabric.api.client.networking.v1.ClientPlayConnectionEvents;
import net.minecraft.tags.FluidTags;
import net.minecraft.world.entity.LivingEntity;
import net.minecraft.world.phys.Vec3;
import java.util.*;

/** Jak 3 splash, surface rings and breath bubbles attached to real Minecraft fluids. */
public final class JakCreatureWater {
 private static final Map<Integer,Boolean> WET=new HashMap<>();
 private static final float K=JakEnemyKind.SCALE*6400/4096;
 private static final String BASIS="["+K+",0,0,0,"+K+",0,0,0,"+K+"]";
 public static void initialize(){
  ClientPlayConnectionEvents.DISCONNECT.register((h,c)->WET.clear());
  ClientTickEvents.END_CLIENT_TICK.register(c->{
   if(c.level==null||c.isPaused())return;
   Set<Integer> present=new HashSet<>();
   for(var entity:c.level.entitiesForRendering())if(entity instanceof LivingEntity e&&e.isAlive()
       &&(e instanceof JakEnemy||e instanceof JakLeaper mount&&mount.riderId()<0)){
    present.add(e.getId());boolean wet=e.isInWater(),was=WET.getOrDefault(e.getId(),false);WET.put(e.getId(),wet);if(!wet)continue;
    var pos=e.blockPosition();double surface=e.getY();
    for(int i=0;i<16;i++){var p=pos.above(i);var fluid=c.level.getFluidState(p);if(!fluid.is(FluidTags.WATER))break;surface=p.getY()+fluid.getHeight(c.level,p);}
    Vec3 origin=e.position();double top=(surface-origin.y)*6400;String at="[0,"+top+",0]";
    StringBuilder commands=new StringBuilder("[");
    if(!was)commands.append("{\"Tracker\":[\"part-water-splash-callback\",0.2,").append(at).append("]},{\"Spawn\":[\"group-part-water-splash\",").append(at).append(',').append(BASIS).append("]},");
    commands.append("{\"Flat\":[758,").append(at).append(',').append(BASIS).append(",1]},");
    if(e.isEyeInFluid(FluidTags.WATER)&&e.tickCount%20==0)commands.append("{\"Value\":[756,\"userdata\",").append(top).append("]},{\"Launch\":[756,[0,").append(e.getEyeHeight()*6400).append(",0],").append(BASIS).append("]},");
    commands.append("{\"Step\":0.016666667},{\"Step\":0.016666667},{\"Step\":0.016666667}]");
    JakEffects.local(e.getId(),origin,JsonParser.parseString(commands.toString()).getAsJsonArray());
   }
   WET.keySet().retainAll(present);
  });
 }
}
