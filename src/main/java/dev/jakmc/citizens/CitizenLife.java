package dev.jakmc.citizens;

import dev.jakmc.entity.CitizenKind;
import dev.jakmc.entity.CitizenState;
import dev.jakmc.entity.JakEnemyKind;
import dev.jakmc.player.JakEntityClock;
import java.util.*;
import net.minecraft.core.BlockPos;
import net.minecraft.server.level.ServerLevel;
import net.minecraft.world.attribute.EnvironmentAttributes;
import net.minecraft.world.entity.ai.attributes.Attributes;
import net.minecraft.world.entity.ai.memory.MemoryModuleType;
import net.minecraft.world.entity.npc.villager.Villager;
import net.minecraft.world.phys.Vec3;

/** Rust: jak2_villagers.rs step, a citizen's own day in its town. The native brain keeps night, work and trade. */
public final class CitizenLife {
 public static final int IDLE=0,WALK=1,POSE=2,PANIC=3,INDOORS=4;
 // Rust: jak2_villagers.rs PANIC_SPEED (m/s), WANDER (blocks)
 static final float PANIC_SPEED=4.5f,WANDER=22;
 // Rust: jak2_villagers.rs STILL
 public static final Map<String,String[]> STILL=Map.of(
  "mayor",new String[]{"mayor-idle"},
  "explorer",new String[]{"explorer-idle","explorer-idle2-look-at-map","explorer-idle3-looking"},
  "gambler",new String[]{"gambler-idle-tiptoe","gambler-idle-look-1","gambler-idle-fidget","gambler-idle-scratch-1"},
  "geologist",new String[]{"geologist-idle"},
  "warrior",new String[]{"warrior-idle"});
 enum Visit{APPROACH,CROSS_IN,ROOM,WAIT,LEAVE_ROOM,CROSS_OUT,YIELD_OUTSIDE,CLEAR_EXIT}
 // Rust: jak2_villagers.rs DoorVisit (blocks; out is the door's outward step) with its home's bedside
 record Door(BlockPos block,int outX,int outZ,Vec3 threshold,double halfWidth,Vec3 outside,Vec3 inside,Vec3 bedside){}

 public final int variant;
 final Villager body;final CitizenKind kind;final int seed;final float pace;final double half,height;final boolean spargus;
 int state=IDLE;double t,length=1,shelter,inside,stuck;Vec3 target,progress,pathed;Door door;Visit phase;boolean owning,reacted,dodging;long last=-1;
 final ArrayDeque<Vec3> route=new ArrayDeque<>();
 Door home;BlockPos homeBed;long homeChecked=Long.MIN_VALUE;

 public CitizenLife(Villager body){
  this.body=body;variant=((CitizenState)body).citizenVariant();kind=CitizenKind.of(variant);seed=body.getUUID().hashCode();
  // stand-in: Rust draws pace = 1.1 + random·0.5 when the citizen is placed; its own seed draws it here.
  pace=1.1f+.5f*((seed>>>12&1023)/1023f);
  float[] s=shape(kind.model()),own=ownScale(kind,seed);
  half=s[0]*JakEnemyKind.SCALE*own[0];height=s[1]*JakEnemyKind.SCALE*own[1];
  spargus=kind.wastelander()||kind.monk();
 }
 public boolean owning(){return owning;}
 /** The day's step as the development fixtures print it. */
 public String debug(){return (owning?"own":"brain")+" s"+state+(door!=null?" door":"")+(target==null?"":String.format(" to%.1f,%.1f",target.x,target.z))+String.format(" stuck%.1f hc%s v%.3f t%.1f/%.1f nav%s",stuck,body.horizontalCollision,body.getDeltaMovement().horizontalDistance(),t,length,body.getNavigation().isDone());}
 public void alarm(double seconds){shelter=Math.max(shelter,seconds);}

 // Rust: creature_body.rs SPHERES and shape (half width, height in game metres; Citizen::body falls back to 1 by 3.4)
 public static float[] shape(String model){return switch(model){
  case "wlander-male"->new float[]{1.3f,3.9f};
  case "wlander-female"->new float[]{1.3f,3.5f};
  case "mayor"->new float[]{1,2};
  case "explorer","gambler","geologist"->new float[]{1,1};
  case "warrior"->new float[]{2,4.25f};
  default->new float[]{1,3.4f};
 };}
 // Rust: jak2_villagers.rs own_scale (Haven's three: width and height each 1 to 1.25, drawn from the flavour as debug_spawn does)
 public static float[] ownScale(CitizenKind kind,int seed){
  if(!kind.model().startsWith("citizen-"))return new float[]{1,1};
  return new float[]{1+.25f*(Integer.toUnsignedLong(seed*7919)%1000)/1000f,1+.25f*(Integer.toUnsignedLong(seed*104_729)%1000)/1000f};
 }
 // Rust: jak2_villagers.rs gait and walk_speed (m/s; the walk index as JakCitizenRenderer picks the clip)
 float walkSpeed(){
  int i=Math.floorMod(seed,3);
  float speed=switch(kind.model()){
   case "citizen-chick"->new float[]{3.6f,2,3}[i];
   case "citizen-norm"->new float[]{2.5f,3,1}[i];
   default->3;
  };
  return speed*Math.clamp(pace/1.35f,.85f,1.15f);
 }
 // Rust: jak2_villagers.rs gait (panic clip and run stride)
 public static String panicClip(int seed){return new String[]{"panic-run","panic-run","run"}[Math.floorMod(seed,3)];}
 public static float runStride(CitizenKind kind){return switch(kind.model()){case "citizen-norm"->7;case "citizen-fat"->5;default->6.4f;};}
 // Rust: jak2_villagers.rs clip_of (State::Pose)
 public static String poseClip(CitizenKind kind,int seed){return kind.model().equals("citizen-chick")?"shuffle":(seed&1)==0?"arms-hips":"arms-crossed";}
 // Rust: jak2_villagers.rs citizen_loop_time (idle at 0.1; chick idle and the held poses at 1/(10..13))
 public static float loopRate(CitizenKind kind,String suffix,int seed){
  if(!kind.model().startsWith("citizen-"))return 1;
  float pose=1/(10+3*(seed&65535)/65535f);
  return switch(suffix){case "idle"->kind.model().equals("citizen-chick")?pose:.1f;case "arms-hips","arms-crossed"->pose;default->1;};
 }
 // Rust: assets/jak_model.rs Clip::loop_time
 public static float loopTime(float duration,float elapsed,int seed){
  if(!(duration>0)||!Float.isFinite(elapsed))return 0;
  double phase=(Integer.toUnsignedLong(seed)*.37)%duration;
  return (float)((((double)elapsed%duration+duration)%duration+phase)%duration);
 }
 // stand-in: Rust draws a still person's flavour as random·1000 when it arrives; its seed draws it here.
 public static long stillFlavor(int seed){return Integer.toUnsignedLong(seed)%1000;}
 // Minecraft ground travel: each tick v=(v+s)·0.6·0.91, settling at s·0.546/0.454 blocks; SCALE is Rust's k·M/BLOCK
 double modifier(float metresPerSecond){return metresPerSecond*JakEnemyKind.SCALE/20/(.546/.454)/body.getAttributeValue(Attributes.MOVEMENT_SPEED);}

 void set(int next){if(next!=state||((CitizenLifeState)body).citizenLifeState()!=next){state=next;((CitizenLifeState)body).citizenLifeState(next,JakEntityClock.time(body.level()));}}
 void hold(){body.getNavigation().stop();pathed=null;}
 // The native brain hands the body over: its walk is dropped before the original controller drives it.
 void take(ServerLevel level){
  if(owning)return;owning=true;
  var brain=body.getBrain();brain.stopAll(level,body);
  brain.eraseMemory(MemoryModuleType.WALK_TARGET);brain.eraseMemory(MemoryModuleType.PATH);brain.eraseMemory(MemoryModuleType.LOOK_TARGET);
  hold();target=body.position();set(IDLE);t=0;length=1+body.getRandom().nextFloat()*3;
 }
 /** Jak II civilian 'clear-path: off the street a craft is coming down, to the given point; false while it is not walking its own day. */
 public boolean stepAside(ServerLevel level,Vec3 to){
  if(body.isTrading()||!owning||door!=null||state==PANIC||state==INDOORS||kind.jak1())return false;
  route.clear();pathed=null;target=to;set(WALK);t=0;length=8;dodging=true;
  return true;
 }
 void release(ServerLevel level){if(!owning)return;owning=false;drop(level);hold();set(IDLE);}
 // Rust: c.visit=None (its doorway reservation goes with it)
 void drop(ServerLevel level){dodging=false;if(door!=null)CitizenTown.unclaim(level,door.block(),body.getUUID());door=null;phase=null;route.clear();pathed=null;}
 boolean otherOwns(ServerLevel level){var who=CitizenTown.owner(level,door.block());return who!=null&&!who.equals(body.getUUID());}
 // Rust: jak2_villagers.rs door_priority
 static int priority(Visit phase){return phase==Visit.CROSS_OUT||phase==Visit.CLEAR_EXIT?0:phase==Visit.LEAVE_ROOM?1:phase==Visit.CROSS_IN?2:-1;}
 static double flatDistance(Vec3 a,Vec3 b){return Math.hypot(a.x-b.x,a.z-b.z);}
 static Vec3 flat(Vec3 v){double l=Math.hypot(v.x,v.z);return l>1e-6?new Vec3(v.x/l,0,v.z/l):new Vec3(1,0,0);}
 // Rust's map plane turned to blocks: perp (-y, x) of a map vector is (z, -x) of a block one
 static Vec3 perp(Vec3 v){return new Vec3(v.z,0,-v.x);}

 // Rust: jak2_villagers.rs step (one citizen's frame; knock, land and get-up stay in CitizenMixin, defence in SpargusDefense)
 public void tick(ServerLevel level){
  var s=(CitizenState)body;
  long now=JakEntityClock.time(level);double dt=last<0?0:Math.min(.1,Math.max(0,now-last)/20.);last=now;
  var random=body.getRandom();float r0=random.nextFloat(),r1=random.nextFloat(),r2=random.nextFloat();
  if(!body.isAlive()){reacted=false;release(level);return;}
  if(s.citizenPhase()!=0){reacted=true;release(level);return;}
  // stand-in: the original has no children; Minecraft's babies keep their own brain.
  if(body.isNoAi()||body.isBaby()||!s.citizenDefenseClip().isEmpty()||body.isPassenger()||body.isSleeping()||body.isTrading()){release(level);return;}
  t+=dt;shelter=Math.max(0,shelter-dt);
  // stand-in: the original alarm replaces Minecraft's bell hiding (HEARD_BELL_TIME).
  if(shelter>0)body.getBrain().eraseMemory(MemoryModuleType.HEARD_BELL_TIME);
  // Rust: minecraft_world.rs set_outside(!bright_outside), as JakEcology reads it
  boolean night=!(level.environmentAttributes().getDimensionValue(EnvironmentAttributes.SKY_LIGHT_LEVEL)>11);
  // Rust: c.still (Jak 1's people stand at their spots all day, and go in to their huts at night)
  if(kind.jak1()){take(level);set(night?INDOORS:IDLE);hold();return;}
  if(reacted){
   // Rust: State::GetUp done, then State::Panic for 4 to 7 s, twenty blocks away from Jak
   reacted=false;take(level);
   var me=level.getNearestPlayer(body,-1);
   Vec3 away=me==null?new Vec3(1,0,0):flat(body.position().subtract(me.position()));
   set(PANIC);t=0;length=4+r0*3;target=body.position().add(away.scale(20));pathed=null;
  }
  // Rust: four times a second, phased by the contact key
  boolean perceive=(body.tickCount+Math.floorMod(body.getId(),4))%5==0;
  if(!spargus&&perceive&&CitizenTown.threatSeen(level,body))shelter=20;
  boolean threatened=!spargus&&(state==PANIC||shelter>0);
  // stand-in: Rust walks them in at their door to bed at night, and to their job site 2000..9000 (minecraft_jobs.rs target);
  // the native brain's rest and work schedule does both.
  long day=Math.floorMod(level.getOverworldClockTime(),24000L);
  boolean work=day>=2000&&day<9000&&body.getBrain().hasMemoryValue(MemoryModuleType.JOB_SITE);
  if(!threatened&&door==null&&state!=PANIC&&(night||work)){release(level);return;}
  take(level);
  if(state<=PANIC&&shelter>0&&door==null){
   var d=CitizenHomes.door(level,this);
   if(d!=null){
    boolean already=CitizenHomes.indoors(body.position(),d);
    door=d;phase=already?Visit.WAIT:Visit.APPROACH;if(already)inside=5;
    target=already?body.position():d.outside();route.clear();pathed=null;
    set(already?IDLE:threatened?PANIC:WALK);t=0;length=120;
   }
  }
  if(door!=null&&phase==Visit.WAIT&&(state==IDLE||state==POSE)){
   if(shelter>0){hold();return;}
   // stand-in: at night Rust walks to the bedside and lies down there; the native rest takes them on from inside.
   if(night){release(level);return;}
   inside=Math.max(0,inside-dt);if(inside>0){hold();return;}
   phase=Visit.LEAVE_ROOM;target=door.inside();route.clear();pathed=null;set(WALK);t=0;length=30;
  }
  if((state==WALK||state==PANIC)&&door!=null&&priority(phase)>=0)CitizenTown.claim(level,door.block(),priority(phase),body.getUUID());
  if(door!=null&&phase==Visit.CROSS_IN&&(otherOwns(level)||t>=length)){
   CitizenTown.door(level,door,true);
   if(!clearDoorway(level,Visit.YIELD_OUTSIDE))return;
  }
  if(door!=null&&phase==Visit.YIELD_OUTSIDE){
   if(otherOwns(level)){if(route.isEmpty()&&flatDistance(body.position(),target)<.12){set(IDLE);hold();return;}}
   else{phase=Visit.APPROACH;target=door.outside();route.clear();pathed=null;set(WALK);t=0;length=90;}
  }
  if(door!=null&&phase==Visit.APPROACH){
   target=door.outside();
   if(otherOwns(level)&&flatDistance(body.position(),door.outside())<.12)clearDoorway(level,Visit.YIELD_OUTSIDE);
  }
  // Rust: record_walk_progress (net progress over the two-second stuck interval)
  if(state==WALK||state==PANIC){
   if(progress==null)progress=body.position();
   if(flatDistance(body.position(),progress)>=.16){progress=body.position();stuck=0;}else stuck+=dt;
  }else{progress=null;stuck=0;}
  if(stuck>2&&(door==null||phase==Visit.APPROACH||phase==Visit.ROOM||phase==Visit.LEAVE_ROOM)){
   route.clear();pathed=null;stuck=0;progress=null;dodging=false;
   // An optional stroll cannot occupy a neighbour's spot: pause, then choose again.
   if(state==WALK&&door==null){target=body.position();set(IDLE);t=0;length=2+r0*2;}
  }
  switch(state){
   case IDLE,POSE->{
    hold();
    if(t<length)return;
    if(state==IDLE&&flatDistance(body.position(),target)>2){set(WALK);t=0;length=12;return;}
    Door visit;
    if(r0<.3){set(POSE);length=3+r1*4;}
    else if(r0<.5&&(visit=CitizenHomes.door(level,this))!=null){drop(level);door=visit;phase=Visit.APPROACH;target=visit.outside();set(WALK);length=90;}
    else{
     // stand-in: Rust picks a street-graph point near the bed (Nav::wander_near); Minecraft's path to the nearest reachable spot of the same reach.
     Vec3 c=CitizenHomes.center(level,body);double a=r1*Math.PI*2,reach=2+r2*WANDER;
     target=c.add(Math.cos(a)*reach,0,-Math.sin(a)*reach);pathed=null;set(WALK);length=25;
    }
    t=0;
   }
   case WALK,PANIC->walk(level,r0,r1,r2);
   default->{}
  }
 }

 // Rust: jak2_villagers.rs step (State::Walk | State::Panic)
 private void walk(ServerLevel level,float r0,float r1,float r2){
  float speed=state==PANIC?PANIC_SPEED:walkSpeed();
  Vec3 at=body.position(),dest=route.isEmpty()?target:route.peekFirst();
  double to=flatDistance(at,dest);
  boolean crossing=door!=null&&(phase==Visit.CROSS_IN||phase==Visit.CROSS_OUT||phase==Visit.YIELD_OUTSIDE||phase==Visit.CLEAR_EXIT);
  double radius=route.size()==1?.10:door!=null&&(phase==Visit.YIELD_OUTSIDE||phase==Visit.CLEAR_EXIT)?.02:.22;
  if(!route.isEmpty()&&to<radius){route.pollFirst();pathed=null;return;}
  boolean close=route.isEmpty()&&(door!=null&&(phase==Visit.CROSS_IN||phase==Visit.CROSS_OUT)?crossed(level,at):door!=null?to<.12:to<1);
  if(close&&door!=null){arrive(level,r0);return;}
  if(close||t>=length&&!crossing){drop(level);set(IDLE);t=0;length=2+r0*4;hold();return;}
  double modifier=modifier(speed);
  var navigation=body.getNavigation();
  // stand-in: Rust steers round Jak and its neighbours (settlement_crowd); Minecraft's path and entity pushing do it here.
  // stand-in: the step off a craft's street goes straight at its spot (ZoomerTraffic.aside found the way open).
  if(crossing||dodging||to<.5||pathed!=null&&navigation.isDone()&&to<1.5){
   // Rust walks the doorway legs, and the last of a walk, straight at the point
   navigation.stop();
   body.getMoveControl().setWantedPosition(dest.x,dest.y,dest.z,modifier);
   if(body.horizontalCollision)stepBlocked(level,crossing,r0,r1,r2);
   return;
  }
  if(pathed==null||pathed.distanceToSqr(dest)>.25){
   pathed=dest;
   boolean found=navigation.moveTo(dest.x,dest.y,dest.z,modifier);
   var path=navigation.getPath();
   if(door!=null&&(!found||path==null||!path.canReach()))searchBlocked(level,r0);
   else if(!found)stepBlocked(level,false,r0,r1,r2);
  }else if(navigation.isDone()||navigation.isStuck())stepBlocked(level,false,r0,r1,r2);
  else navigation.setSpeedModifier(modifier);
 }
 // Rust: creature_navigation Result::Blocked in step (no way there: wait inside, or give up the visit; pause 1.5 to 2.5 s)
 private void searchBlocked(ServerLevel level,float r0){
  if(phase==Visit.ROOM||phase==Visit.LEAVE_ROOM){phase=Visit.WAIT;inside=0;}else drop(level);
  set(IDLE);target=body.position();t=0;length=1.5+r0;hold();
 }
 // Rust: Ahead::Blocked in step (a doorway leg asks the door open and clears it after 2 s; a stroll pauses and turns away)
 private void stepBlocked(ServerLevel level,boolean crossing,float r0,float r1,float r2){
  if(crossing){
   CitizenTown.door(level,door,true);
   if(t>=2)clearDoorway(level,phase==Visit.CROSS_OUT||phase==Visit.CLEAR_EXIT?Visit.CLEAR_EXIT:Visit.YIELD_OUTSIDE);
   return;
  }
  drop(level);set(IDLE);t=0;length=1.5+2*r0;hold();
  float yaw=(float)Math.toRadians(body.getYRot());
  double heading=Math.atan2(-Math.cos(yaw),-Math.sin(yaw)),a=heading+Math.PI*(.5+r1),reach=3+r2*6;
  target=body.position().add(Math.cos(a)*reach,0,-Math.sin(a)*reach);
 }
 // Rust: jak2_villagers.rs step, a visit's arrivals
 private void arrive(ServerLevel level,float r0){
  var uuid=body.getUUID();
  switch(phase){
   case APPROACH->{
    if(otherOwns(level)){hold();return;}
    CitizenTown.claim(level,door.block(),2,uuid);CitizenTown.door(level,door,true);
    phase=Visit.CROSS_IN;target=door.inside();t=0;length=20;
   }
   case CROSS_IN->{
    CitizenTown.unclaim(level,door.block(),uuid);CitizenTown.door(level,door,false);
    phase=Visit.WAIT;inside=5+r0*15;set(IDLE);t=0;length=inside;hold();
    // Wait by the owned bed, leaving the entrance free for the house's other inhabitant.
    if(door.bedside()!=null){phase=Visit.ROOM;target=door.bedside();route.clear();set(WALK);length=40;}
   }
   case ROOM->{phase=Visit.WAIT;set(IDLE);t=0;hold();}
   case LEAVE_ROOM->{
    if(otherOwns(level)){hold();return;}
    CitizenTown.claim(level,door.block(),0,uuid);CitizenTown.door(level,door,true);
    phase=Visit.CROSS_OUT;target=door.outside();t=0;length=20;
   }
   case CROSS_OUT->{CitizenTown.door(level,door,true);clearDoorway(level,Visit.CLEAR_EXIT);}
   case CLEAR_EXIT->{
    CitizenTown.unclaim(level,door.block(),uuid);CitizenTown.door(level,door,false);
    door=null;phase=null;route.clear();set(IDLE);t=0;length=2+r0*4;hold();
   }
   case YIELD_OUTSIDE->{set(IDLE);hold();}
   case WAIT->{}
  }
 }
 // Rust: jak2_villagers.rs clear_doorway (out through the doorway, then sideways to a tested waiting place)
 private boolean clearDoorway(ServerLevel level,Visit next){
  var wait=waitPoint(level);if(wait==null)return false;
  Vec3 out=flat(door.outside().subtract(door.inside()));
  route.clear();pathed=null;
  if(body.position().subtract(door.outside()).multiply(1,0,1).dot(out)<-.02)route.add(door.outside());
  route.add(wait);target=wait;set(WALK);t=0;length=30;phase=next;return true;
 }
 // Rust: jak2_villagers.rs door_wait_candidates and door_wait_point
 private Vec3 waitPoint(ServerLevel level){
  Vec3 out=flat(door.outside().subtract(door.inside())),side=perp(out);
  double width=door.halfWidth()+half+.25,preferred=(seed&1)==0?1:-1,forward=2*half+.25;
  for(double[] o:new double[][]{{preferred*width,0},{-preferred*width,0},{preferred*width,1},{-preferred*width,1},{0,forward},{0,3}}){
   var at=checkWait(level,door.outside().add(side.scale(o[0])).add(out.scale(o[1])));
   if(at!=null)return at;
  }
  return null;
 }
 // Rust: jak2_villagers.rs check_door_wait
 private Vec3 checkWait(ServerLevel level,Vec3 candidate){
  double lift=.61,z=CitizenTown.ground(level,candidate.add(0,lift,0),lift+1.1,half);
  if(Double.isNaN(z))return null;
  Vec3 at=new Vec3(candidate.x,z,candidate.z);
  if(!CitizenTown.clear(level,at.add(0,.02,0),half,height)||!CitizenTown.clear(level,door.outside().add(0,.02,0),half,height))return null;
  // stand-in: Rust walks its full-body controller over each tread from the landing; the native path walks it here.
  if(CitizenTown.wet(level,at.add(0,.1,0)))return null;
  for(var o:level.getEntitiesOfClass(Villager.class,new net.minecraft.world.phys.AABB(at,at).inflate(3),o->o!=body&&o.isAlive())){
   var other=((CitizenLifeState)o).citizenLife();
   if(o.getY()+other.height>at.y+1/36.&&o.getY()<at.y+height&&flatDistance(o.position(),at)<half+other.half+.2)return null;
  }
  return at;
 }
 // Rust: jak2_villagers.rs crossed_visit_target
 private boolean crossed(ServerLevel level,Vec3 at){
  Vec3 from=phase==Visit.CROSS_IN?door.outside():door.inside(),to=phase==Visit.CROSS_IN?door.inside():door.outside();
  Vec3 along=to.subtract(from).multiply(1,0,1);
  if(along.lengthSqr()<=1/1296.||Math.abs(at.y-to.y)>.6)return false;
  along=along.normalize();
  Vec3 toEnd=at.subtract(to).multiply(1,0,1);
  if(toEnd.dot(along)<0&&toEnd.length()>=.12)return false;
  Vec3 side=perp(along);
  double lateral=at.subtract(door.threshold()).multiply(1,0,1).dot(side);
  if(Math.abs(lateral)+half>door.halfWidth())return false;
  return CitizenTown.clear(level,door.threshold().add(side.scale(lateral)).add(0,.02,0),half,height);
 }
}
