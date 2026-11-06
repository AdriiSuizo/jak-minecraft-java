package dev.jakmc.client;
import net.fabricmc.fabric.api.client.event.lifecycle.v1.ClientTickEvents;
import net.minecraft.world.entity.EntityTypes;
import net.minecraft.world.entity.npc.villager.Villager;
/** The citizens' authored step cues, beside the walk the renderer draws (Rust: jak2_villagers.rs npc_walk_effects). */
public final class CitizenFootsteps {
 public static void initialize(){
  ClientTickEvents.END_LEVEL_TICK.register(level->{
   for(var e:level.entitiesForRendering())if(e instanceof Villager v&&v.getType()==EntityTypes.VILLAGER)dev.jakmc.citizens.CitizenSteps.tick(level,v);
  });
 }
}
