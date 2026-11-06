package dev.jakmc.citizens;

import dev.jakmc.entity.CitizenKind;
import dev.jakmc.entity.CitizenState;
import dev.jakmc.entity.JakEnemyKind;
import java.util.*;
import net.fabricmc.fabric.api.event.lifecycle.v1.ServerLifecycleEvents;
import net.fabricmc.fabric.api.event.lifecycle.v1.ServerTickEvents;
import net.minecraft.core.BlockPos;
import net.minecraft.core.Direction;
import net.minecraft.resources.ResourceKey;
import net.minecraft.server.level.ServerLevel;
import net.minecraft.world.entity.EntitySpawnReason;
import net.minecraft.world.entity.EntityTypes;
import net.minecraft.world.entity.Mob;
import net.minecraft.world.entity.ai.village.poi.PoiManager;
import net.minecraft.world.entity.ai.village.poi.PoiTypes;
import net.minecraft.world.entity.monster.Enemy;
import net.minecraft.world.entity.npc.villager.Villager;
import net.minecraft.world.level.ChunkPos;
import net.minecraft.world.level.ClipContext;
import net.minecraft.world.level.Level;
import net.minecraft.world.level.block.Blocks;
import net.minecraft.world.level.block.DoorBlock;
import net.minecraft.world.phys.AABB;
import net.minecraft.world.phys.HitResult;
import net.minecraft.world.phys.Vec3;
import net.minecraft.world.phys.shapes.Shapes;

/** Rust: jak2_villagers.rs, what the citizens share: doorway reservations and requests, the bell's alarm, threats, Jak 1's people. */
public final class CitizenTown {
 // Rust: jak2_villagers.rs PEOPLED (blocks); M is a game metre in blocks before the creatures' scale (M/BLOCK)
 static final double PEOPLED=150,M=25/36.;
 record Claim(int priority,UUID who,long tick){}
 record Request(CitizenLife.Door door,boolean open){}
 private static final Map<ResourceKey<Level>,Map<Long,Claim>> CLAIMS=new HashMap<>();
 private static final Map<ResourceKey<Level>,Map<Long,Request>> DOORS=new HashMap<>();

 public static void initialize(){
  ServerTickEvents.END_LEVEL_TICK.register(level->{
   doors(level);
   var claims=CLAIMS.get(level.dimension());if(claims!=null)claims.values().removeIf(c->c.tick()<level.getGameTime()-1);
   // stand-in: Rust peoples its villages every frame; once a second here.
   if(level.getGameTime()%20==0)people(level);
   dev.jakmc.worldgen.SpargusPatrols.tick(level);
   dev.jakmc.worldgen.CityPopulation.tick(level);
   dev.jakmc.entity.CityAlert.tick(level);
  });
  ServerLifecycleEvents.SERVER_STOPPED.register(server->{CLAIMS.clear();DOORS.clear();dev.jakmc.worldgen.CityPopulation.clear();dev.jakmc.entity.CityAlert.clear();});
 }

 // Rust: jak2_villagers.rs step door_owner (departures reserve the doorway before arrivals; the lowest priority, then key, owns it)
 static void claim(ServerLevel level,BlockPos at,int priority,UUID who){
  var claims=CLAIMS.computeIfAbsent(level.dimension(),k->new HashMap<>());long now=level.getGameTime();
  var c=claims.get(at.asLong());
  if(c==null||c.tick()<now-1||c.who().equals(who)||priority<c.priority()||priority==c.priority()&&who.compareTo(c.who())<0)
   claims.put(at.asLong(),new Claim(priority,who,now));
 }
 static UUID owner(ServerLevel level,BlockPos at){
  var claims=CLAIMS.get(level.dimension());if(claims==null)return null;
  var c=claims.get(at.asLong());return c==null||c.tick()<level.getGameTime()-1?null:c.who();
 }
 static void unclaim(ServerLevel level,BlockPos at,UUID who){
  var claims=CLAIMS.get(level.dimension());if(claims!=null)claims.computeIfPresent(at.asLong(),(k,c)->c.who().equals(who)?null:c);
 }
 // Rust: jak2_villagers.rs take_door_requests (an opening beats a close in the same tick; a close waits while a body is in the doorway)
 static void door(ServerLevel level,CitizenLife.Door d,boolean open){
  DOORS.computeIfAbsent(level.dimension(),k->new HashMap<>()).merge(d.block().asLong(),new Request(d,open),(a,b)->new Request(b.door(),a.open()||b.open()));
 }
 private static void doors(ServerLevel level){
  var requests=DOORS.get(level.dimension());if(requests==null||requests.isEmpty())return;
  for(var it=requests.values().iterator();it.hasNext();){
   var r=it.next();
   if(!r.open()&&occupied(level,r.door()))continue;
   it.remove();
   // stand-in: Rust's set_door_open uses the marked leaf; its right-hand partner of the double door opens with it here.
   var d=r.door();
   for(var at:new BlockPos[]{d.block(),d.block().offset(-d.outZ(),0,d.outX())}){
    var state=level.getBlockState(at);
    if(state.getBlock() instanceof DoorBlock leaf&&leaf.isOpen(state)!=r.open())leaf.setOpen(null,level,state,at,r.open());
   }
  }
 }
 // Rust: jak2_villagers.rs occupies_doorway
 private static boolean occupied(ServerLevel level,CitizenLife.Door d){
  var th=d.threshold();
  for(var v:level.getEntitiesOfClass(Villager.class,new AABB(d.block()).inflate(4),Villager::isAlive)){
   var life=((CitizenLifeState)v).citizenLife();var at=v.position();
   double ax=at.x-th.x,az=at.z-th.z,along=ax*d.outX()+az*d.outZ(),side=az*d.outX()-ax*d.outZ();
   if(Math.abs(along)<.5+life.half&&Math.abs(side)<1+life.half&&at.y<th.y+3&&at.y+life.height>th.y)return true;
  }
  return false;
 }

 // Rust: jak2_villagers.rs ring_bell and step (an alarm sends everyone within 64 blocks to their own house for 30 s)
 public static void ringBell(ServerLevel level,BlockPos bell){
  var c=Vec3.atCenterOf(bell);
  for(var v:level.getEntitiesOfClass(Villager.class,new AABB(bell).inflate(64),v->v.isAlive()&&v.getType()==EntityTypes.VILLAGER))
   if(Math.hypot(v.getX()-c.x,v.getZ()-c.z)<64)((CitizenLifeState)v).citizenLife().alarm(30);
 }
 // Rust: jak2_villagers.rs visible_threat over jak_entities.rs settlement_threats (Jak's enemies and Minecraft's hostile mobs)
 static boolean threatSeen(ServerLevel level,Villager v){
  Vec3 feet=v.position();
  for(var e:level.getEntitiesOfClass(Mob.class,v.getBoundingBox().inflate(18),m->m instanceof Enemy&&m.isAlive())){
   if(Math.abs(e.getY()-feet.y)>=6||e.position().distanceToSqr(feet)>=18*18)continue;
   var hit=level.clip(new ClipContext(feet.add(0,1,0),e.position().add(0,1,0),ClipContext.Block.COLLIDER,ClipContext.Fluid.NONE,v));
   if(hit.getType()==HitResult.Type.MISS)return true;
  }
  return false;
 }

 // Rust: creature_body.rs ground (a box `half` wide and 0.1 m tall swept down `reach`; NaN when it starts inside or meets nothing)
 static double ground(ServerLevel level,Vec3 at,double reach,double half){
  var box=new AABB(at.x-half,at.y,at.z-half,at.x+half,at.y+.1*M,at.z+half);
  if(!level.noBlockCollision(null,box))return Double.NaN;
  double moved=Shapes.collide(Direction.Axis.Y,box,level.getBlockCollisions(null,box.expandTowards(0,-reach,0)),-reach);
  return moved>-reach+1e-7?at.y+moved:Double.NaN;
 }
 // Rust: creature_body.rs clear (a standing body's box)
 static boolean clear(ServerLevel level,Vec3 feet,double half,double height){
  return level.noBlockCollision(null,new AABB(feet.x-half,feet.y,feet.z-half,feet.x+half,feet.y+height,feet.z+half));
 }
 // Rust: sim::voxel::eyes_in_fluid (water or lava there)
 static boolean wet(ServerLevel level,Vec3 at){return !level.getFluidState(BlockPos.containing(at)).isEmpty();}
 /** Rust: settlement_nav standing, for a street spawn site: a wastelander's body (the largest) standing dry there, or null. */
 public static Vec3 standing(ServerLevel level,Vec3 at){
  float[] s=CitizenLife.shape("wlander-male");
  return spawnFeet(level,at.add(0,2,0),4,at.y,2,s[0]*JakEnemyKind.SCALE,s[1]*JakEnemyKind.SCALE);
 }
 // Rust: creature_navigation.rs spawn (the full body on level ground near `levelY`, dry)
 static Vec3 spawnFeet(ServerLevel level,Vec3 at,double reach,double levelY,double rise,double half,double height){
  double z=ground(level,at,reach,half*.25);
  if(Double.isNaN(z)||Math.abs(z-levelY)>rise)return null;
  double r=half*.8,lo=z,hi=z;
  for(double[] d:new double[][]{{-r,-r},{-r,r},{r,-r},{r,r}}){
   double h=ground(level,new Vec3(at.x+d[0],z+.7,at.z+d[1]),1.4,.03);
   if(Double.isNaN(h))return null;lo=Math.min(lo,h);hi=Math.max(hi,h);
  }
  if(hi-lo>.6)return null;
  var feet=new Vec3(at.x,hi,at.z);
  return wet(level,feet.add(0,.1,0))||!clear(level,feet.add(0,.02,0),half,height)?null:feet;
 }

 // Rust: jak2_villagers.rs step, peopling (a Sandover bell, on chiseled stone, within PEOPLED gets two of Jak 1's people at their spots)
 private static void people(ServerLevel level){
  for(var p:level.players()){
   if(p.isSpectator())continue;
   var bells=new ArrayList<BlockPos>();int cx=p.blockPosition().getX()>>4,cz=p.blockPosition().getZ()>>4,reach=(int)Math.ceil(PEOPLED/16);
   // Only loaded chunks: the bells the world has loaded, as Rust's chunk marks hold them.
   for(int dx=-reach;dx<=reach;dx++)for(int dz=-reach;dz<=reach;dz++)if(level.getChunkSource().getChunkNow(cx+dx,cz+dz)!=null)
    level.getPoiManager().getInChunk(h->h.is(PoiTypes.MEETING),new ChunkPos(cx+dx,cz+dz),PoiManager.Occupancy.ANY).forEach(r->bells.add(r.getPos()));
   for(var bell:bells){
    if(Math.hypot(bell.getX()+.5-p.getX(),bell.getZ()+.5-p.getZ())>=PEOPLED||!level.getBlockState(bell.below()).is(Blocks.CHISELED_STONE_BRICKS))continue;
    if(!level.areEntitiesLoaded(ChunkPos.containing(bell).pack()))continue;
    // Rust: a village's bells stand close, one bell peoples it (48 blocks)
    // stand-in: Rust empties far villages and peoples them again; these people persist as native entities instead.
    if(!level.getEntitiesOfClass(Villager.class,new AABB(bell).inflate(48),v->CitizenKind.of(((CitizenState)v).citizenVariant()).jak1()).isEmpty())continue;
    still(level,bell);
   }
  }
 }
 private static void still(ServerLevel level,BlockPos bell){
  var random=level.getRandom();var home=new Vec3(bell.getX()+.5,bell.getY(),bell.getZ()+.5);
  int stood=0;
  for(int k=0;k<12&&stood<2;k++){
   int pick=(int)(random.nextFloat()*CitizenKind.JAK1_PEOPLE)%CitizenKind.JAK1_PEOPLE;
   var kind=CitizenKind.of(CitizenKind.JAK1_FIRST+pick);
   double angle=k*1.7+random.nextFloat();
   var at=home.add(Math.cos(angle)*5,0,-Math.sin(angle)*5);
   float[] s=CitizenLife.shape(kind.model());
   var feet=spawnFeet(level,at.add(0,3,0),8,home.y,2,s[0]*JakEnemyKind.SCALE,s[1]*JakEnemyKind.SCALE);
   if(feet==null)continue;
   // stand-in: Rust also claims them a bed; the native brain claims its own.
   var v=EntityTypes.VILLAGER.create(level,EntitySpawnReason.EVENT);
   if(v==null)continue;
   stood++;
   ((CitizenState)v).citizenVariant(CitizenKind.JAK1_FIRST+pick);
   float yaw=(float)Math.toDegrees(Math.atan2(-(home.x-at.x),home.z-at.z));
   v.snapTo(feet.x,feet.y,feet.z,yaw,0);v.setYHeadRot(yaw);v.setYBodyRot(yaw);
   v.setPersistenceRequired();
   level.addFreshEntity(v);
  }
 }
}
