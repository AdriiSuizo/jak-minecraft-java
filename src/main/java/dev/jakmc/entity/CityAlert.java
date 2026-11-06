package dev.jakmc.entity;

import java.util.*;
import net.minecraft.resources.ResourceKey;
import net.minecraft.server.level.ServerLevel;
import net.minecraft.world.entity.npc.villager.Villager;
import net.minecraft.world.entity.player.Player;
import net.minecraft.world.level.Level;

/**
 * Rust: jak2_villagers.rs Citizen.struck (Jak's second blow on one citizen, `hit-by-player-count`, raises the
 * city's alert: `citizen` `trigger-alert`) and jak_entities.rs raise_alert / alert_step: 30 s and its last 3,
 * which wait while a guard is still after someone (`guard-count`); then the guards calm.
 */
public final class CityAlert {
 static final int ALERT=30*20,ENDING=3*20;
 private record Alert(UUID offender,long ends){}
 private static final Map<ResourceKey<Level>,Alert> ALERTS=new HashMap<>();
 private static final WeakHashMap<Villager,Integer> STRUCK=new WeakHashMap<>();
 private CityAlert(){}

 /** A player's blow on a citizen: Spargus's people defend themselves instead (spargus_resident). */
 public static void struck(ServerLevel level,Villager v,Player player){
  var kind=CitizenKind.of(((CitizenState)v).citizenVariant());
  if(kind.wastelander()||((CitizenState)v).citizenVariant()==CitizenKind.MONK||!v.isAlive())return;
  int blows=STRUCK.merge(v,1,Integer::sum);
  if(blows>=2)raise(level,player);
 }
 public static void raise(ServerLevel level,Player player){
  if(player.isCreative()||player.isSpectator())return;
  ALERTS.put(level.dimension(),new Alert(player.getUUID(),level.getGameTime()+ALERT+ENDING));
 }
 public static boolean on(ServerLevel level){return ALERTS.containsKey(level.dimension());}

 public static void tick(ServerLevel level){
  // stand-in: Rust steps the guards every frame; four times a second here.
  if(level.getGameTime()%5!=0)return;
  var alert=ALERTS.get(level.dimension());
  if(alert==null)return;
  var offender=level.getPlayerByUUID(alert.offender());
  long now=level.getGameTime();
  var guards=new ArrayList<KrimsonGuard>();
  if(offender!=null)for(var e:level.getEntitiesOfClass(JakEnemy.class,offender.getBoundingBox().inflate(160),g->g.guardController()!=null&&g.isAlive()))guards.add(e.guardController());
  boolean pursuing=guards.stream().anyMatch(KrimsonGuard::pursuing);
  long ends=alert.ends();
  if(pursuing&&ends-now<=ENDING)ends=now+ENDING;
  if(offender==null||!offender.isAlive()||now>=ends){
   ALERTS.remove(level.dimension());
   if(offender!=null)for(var g:guards)g.calm(offender);
   return;
  }
  if(ends!=alert.ends())ALERTS.put(level.dimension(),new Alert(alert.offender(),ends));
  for(var g:guards)g.alerted(level,offender);
 }
 public static void clear(){ALERTS.clear();STRUCK.clear();}
}
