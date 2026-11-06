package dev.jakmc.entity;

import dev.jakmc.network.JakPackets;
import java.util.*;
import net.fabricmc.fabric.api.event.lifecycle.v1.ServerTickEvents;
import net.fabricmc.fabric.api.networking.v1.PlayerLookup;
import net.fabricmc.fabric.api.networking.v1.ServerPlayNetworking;
import net.minecraft.core.BlockPos;
import net.minecraft.server.level.ServerLevel;
import net.minecraft.world.entity.Entity;
import net.minecraft.world.entity.LivingEntity;
import net.minecraft.world.entity.Mob;
import net.minecraft.world.entity.player.Player;
import net.minecraft.world.phys.AABB;
import net.minecraft.world.phys.Vec3;

/**
 * Jak's Mass Inverter on Minecraft's creatures, Rust minecraft_terrain server.rs (`lift`, `float_lifted`):
 * a lifted creature stands where its brain would walk it, drifts with the blows it takes, rises to a
 * metre over the ground, hangs between one and three and a half and sinks back above that; when its
 * time is up it is let go to fall. A blow on one floating, or still tumbling, spins it (MobTumble, drawn
 * on the client as the Rust's client_mobs.rs did).
 * stand-in, asked for (Adrii 06/11): the field also lifts Minecraft's objects (items on the ground,
 * arrows, boats, minecarts, falling blocks, TNT, experience): the Rust's field (weapons.rs Gravity,
 * 30 m grown over its first second, 7 to 9 s afloat less the time it took to reach them) caught only
 * creatures. Players are left alone.
 */
public final class MassInverter {
 private static final Map<Entity,Integer> LIFTS=new WeakHashMap<>();
 private static final Map<Entity,Vec3> BEFORE=new WeakHashMap<>();
 /** Open fields: where, their first tick, what they caught. */
 private record Field(ServerLevel level,Vec3 center,long start,Set<Entity> caught,Player owner){}
 private static final List<Field> FIELDS=new ArrayList<>();
 /** For the scenes: objects a field caught. */
 public static int objectsLifted;
 /** weapons.rs Gravity: 30 m (Jak metres, JakPlayers.METRE blocks each) at one second. */
 private static final double FIELD=30*dev.jakmc.player.JakPlayers.METRE;
 private MassInverter(){}

 public static void initialize(){
  ServerTickEvents.START_LEVEL_TICK.register(level->{
   BEFORE.clear();
   for(var e:LIFTS.keySet())if(e.level()==level&&e.isAlive())BEFORE.put(e,e.position());
  });
  ServerTickEvents.END_LEVEL_TICK.register(level->{grow(level);floatLifted(level);});
 }
 /** The Mass Inverter's field opened at `center` (Event::GravityField): its objects, as it grows. */
 public static void field(Player owner,Vec3 center){
  if(owner.level() instanceof ServerLevel level)FIELDS.add(new Field(level,center,level.getGameTime(),Collections.newSetFromMap(new WeakHashMap<>()),owner));
 }
 private static void grow(ServerLevel level){
  for(var it=FIELDS.iterator();it.hasNext();){
   var f=it.next();if(f.level!=level)continue;
   double t=(level.getGameTime()-f.start)/20.0;
   if(t>1){it.remove();continue;}
   double r=FIELD*t;
   for(var e:level.getEntities((Entity)null,new AABB(f.center,f.center).inflate(r),MassInverter::object)){
    if(e.position().distanceTo(f.center)>r||!f.caught.add(e))continue;
    // weapons.rs: rng(7, 9) - t seconds, at least 2.
    lift(e,(float)Math.max(2,7+level.getRandom().nextDouble()*2-t));objectsLifted++;
   }
  }
 }
 /** What the native actor world does not lift: not a creature, not a player's ride. */
 private static boolean object(Entity e){
  return !(e instanceof LivingEntity)&&e.isAlive()&&!e.isSpectator()&&e.getPassengers().stream().noneMatch(r->r instanceof Player);
 }
 /** server.rs lift: its gravity gone for `seconds`, the longer of what is left and this. */
 public static void lift(Entity e,float seconds){
  if(e instanceof Player||!(e.level() instanceof ServerLevel level))return;
  int ticks=Math.round(Math.max(0,seconds)*20);
  LIFTS.merge(e,ticks,Math::max);
  if(e instanceof LivingEntity)send(level,e,new JakPackets.Tumble(e.getId(),LIFTS.get(e),0,0,0,0));
 }
 /** A blow or shot along `dir` (Jak's damage): mobs.rs strength = clamp(MW2 damage / 10, 0.4, 1.2), MW2 = 12.5 a Jak point. */
 public static void hit(LivingEntity e,Vec3 dir,float jakDamage){
  if(e instanceof Player||!(e.level() instanceof ServerLevel level))return;
  float strength=Math.clamp(jakDamage*12.5f/10,.4f,1.2f);
  var d=dir.lengthSqr()>1e-9?dir.normalize():Vec3.ZERO;
  send(level,e,new JakPackets.Tumble(e.getId(),LIFTS.getOrDefault(e,0),(float)d.x,(float)d.y,(float)d.z,strength));
 }
 private static void send(ServerLevel level,Entity e,JakPackets.Tumble packet){
  for(var watcher:PlayerLookup.tracking(e))ServerPlayNetworking.send(watcher,packet);
 }
 // server.rs float_lifted (once a tick, after the creature's own)
 private static void floatLifted(ServerLevel level){
  final double tick=1/20.0;
  for(var it=LIFTS.entrySet().iterator();it.hasNext();){
   var en=it.next();var body=en.getKey();
   if(body.level()!=level)continue;
   if(!body.isAlive()){it.remove();continue;}
   var start=BEFORE.get(body);if(start==null)continue;
   // Height over the ground under it, up to eight blocks down.
   int x=(int)Math.floor(start.x),z=(int)Math.floor(start.z),feet=(int)Math.floor(start.y);
   double ground=start.y-8;
   for(int d=0;d<8;d++){var at=new BlockPos(x,feet-d,z);if(!level.getBlockState(at).getCollisionShape(level,at).isEmpty()){ground=feet-d+1;break;}}
   double h=start.y-ground,rise;
   if(h<1)rise=10*(1-h);
   else if(h<=3.5)rise=0;
   else{double over=(h-3.5)*4096;rise=-10*Math.min(.0000814*over*over,15*4096)/4096;}
   var v=body.getDeltaMovement();
   double vx=v.x*.9,vz=v.z*.9;
   if(body instanceof Mob mob)mob.getNavigation().stop();
   body.setPos(start.x+vx,start.y+rise*tick,start.z+vz);
   body.setDeltaMovement(vx,0,vz);
   body.resetFallDistance();body.setOnGround(false);body.needsSync=true;
   int left=en.getValue()-1;
   if(left<=0)it.remove();else en.setValue(left);
  }
 }
}
