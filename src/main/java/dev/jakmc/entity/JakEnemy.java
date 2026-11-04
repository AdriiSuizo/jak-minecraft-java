package dev.jakmc.entity;

import dev.jakmc.content.JakItems;
import net.minecraft.core.BlockPos;
import net.minecraft.network.syncher.*;
import net.minecraft.server.level.ServerLevel;
import net.minecraft.sounds.SoundEvent;
import net.minecraft.world.damagesource.DamageSource;
import net.minecraft.world.entity.*;
import net.minecraft.world.entity.ai.attributes.Attributes;
import net.minecraft.world.entity.ai.goal.FloatGoal;
import net.minecraft.world.entity.monster.Monster;
import net.minecraft.world.entity.player.Player;
import net.minecraft.world.level.Level;
import net.minecraft.world.level.storage.*;
import net.minecraft.world.phys.*;
import java.util.*;

/** Original enemy decisions and animation phases; Minecraft owns bodies, navigation,
 * oxygen, fluid currents, health, damage sources, persistence and dropped items. */
public final class JakEnemy extends Monster {
 public enum State {IDLE,NOTICE,CHASE,ATTACK,CELEBRATE,WALK,HURT,DEAD,GIVE_UP}
 private static final EntityDataAccessor<String> CLIP=SynchedEntityData.defineId(JakEnemy.class,EntityDataSerializers.STRING);
 private static final EntityDataAccessor<Long> CLIP_START=SynchedEntityData.defineId(JakEnemy.class,EntityDataSerializers.LONG);
 private static final EntityDataAccessor<Boolean> LOOP=SynchedEntityData.defineId(JakEnemy.class,EntityDataSerializers.BOOLEAN);
 private static final EntityDataAccessor<Float> PLAY_RATE=SynchedEntityData.defineId(JakEnemy.class,EntityDataSerializers.FLOAT);
 private static final EntityDataAccessor<Float> CLIP_OFFSET=SynchedEntityData.defineId(JakEnemy.class,EntityDataSerializers.FLOAT);
 private static final EntityDataAccessor<Integer> PHASE=SynchedEntityData.defineId(JakEnemy.class,EntityDataSerializers.INT);
 private static final EntityDataAccessor<String> SHOT=SynchedEntityData.defineId(JakEnemy.class,EntityDataSerializers.STRING);
 private final KrimsonGuard guard;
 private State state=State.IDLE;
 private int stateTicks,pathWait,landTicks,windup;
 private int reactionEnds,crabPhase=-1;
 private boolean reactionAirborne;
 private boolean struck,wasGround=true;
 private boolean requestLeap,leaping;
 private boolean nativeHeavy;
 private int leapWindup;
 private Vec3 leapMotion=Vec3.ZERO,lunge=Vec3.ZERO;
 private final boolean spydroid;
 private final boolean crab;
 private final boolean hopper;
 private final String recovery;
 private String hopperClip="";
 private float hopperStart,hopperFlight;
 private Vec3 home,roam;
 private String sounding="";
 private float soundTime=-.001f;
 public final JakEnemyKind kind;
 public JakEnemy(EntityType<? extends Monster> type,Level level){
  super(type,level);kind=Objects.requireNonNull(JakEntities.KINDS.get(type),"Unregistered Jak enemy");
  guard=kind.guard()?new KrimsonGuard(this):null;
  spydroid=kind.model.equals("spydroid");
  crab=kind.model.equals("lurkercrab");
  hopper=kind.model.equals("hopper");
  String knocked=kind.clips[6];
  recovery=knocked.isEmpty()?"":kind.first(knocked+"-land",
   knocked.replace("knocked0","knocked-land0"),knocked.replace("knocked","knocked-land"),knocked);
  setPathfindingMalus(net.minecraft.world.level.pathfinder.PathType.WATER,0);
  getNavigation().setCanFloat(true);
  jumpControl=new net.minecraft.world.entity.ai.control.JumpControl(this){@Override public void jump(){if(isInWater()||isInLava())super.jump();else if(guard!=null)super.jump();else if(onGround()&&!leaping&&leapWindup==0&&landTicks==0)requestLeap=true;}};
 }
 @Override protected void registerGoals(){goalSelector.addGoal(0,new FloatGoal(this));}
 @Override protected void defineSynchedData(SynchedEntityData.Builder b){super.defineSynchedData(b);b.define(CLIP,"");b.define(CLIP_START,0L);b.define(LOOP,true);b.define(PLAY_RATE,1f);b.define(CLIP_OFFSET,0f);b.define(PHASE,0);b.define(SHOT,"");}
 public String animation(){String c=entityData.get(CLIP);return c.isEmpty()?kind.clips[0]:c;}
 public boolean animationLoops(){return entityData.get(LOOP);}
 public float animationRate(){return entityData.get(PLAY_RATE);}
 public float animationSeconds(float partial){
  float elapsed=Math.max(0,(dev.jakmc.player.JakEntityClock.time(level())-entityData.get(CLIP_START)+partial)/20f);
  if(state()==State.HURT&&kind.clips[6].isEmpty()&&animation().equals(kind.clips[7])){
   // One-hit enemies borrow the first frames of their death without ever playing its fall.
   return (float)Math.sin(Math.PI*Math.clamp(elapsed/.45f,0,1))*Math.min(kind.duration(animation())*.15f,.10f);
  }
  return entityData.get(CLIP_OFFSET)+elapsed*animationRate();
 }
 public State state(){return State.values()[Math.clamp(entityData.get(PHASE),0,State.values().length-1)];}
 KrimsonGuard guardController(){return guard;}
 void restartAnimation(){entityData.set(CLIP_START,dev.jakmc.player.JakEntityClock.time(level()));entityData.set(CLIP_OFFSET,0f);sounding="";}
 public String guardShot(){return entityData.get(SHOT);}
 void guardShot(String value){entityData.set(SHOT,value);}
 public void patrol(BlockPos start,BlockPos end){if(guard!=null)guard.patrol(start,end);}
 void animate(String clip,boolean loop,float rate){
  if(!clip.equals(entityData.get(CLIP))){entityData.set(CLIP,clip);restartAnimation();}
  entityData.set(LOOP,loop);entityData.set(PLAY_RATE,rate);
 }
 void set(State next){
  state=next;stateTicks=0;struck=false;lunge=Vec3.ZERO;windup=0;landTicks=0;requestLeap=false;leapWindup=0;leaping=false;
  reactionAirborne=false;reactionEnds=0;crabPhase=-1;
  entityData.set(PHASE,next.ordinal());restartAnimation();
  if(next!=State.CHASE&&next!=State.WALK){getNavigation().stop();getMoveControl().setWantedPosition(getX(),getY(),getZ(),0);}
 }
 @Override protected void customServerAiStep(ServerLevel level){
  super.customServerAiStep(level);if(guard!=null){guard.tick(level);sounds();return;}if(home==null)home=position();stateTicks++;if(pathWait>0)pathWait--;if(landTicks>0)landTicks--;
  if(!hopper&&onGround()&&!wasGround&&isAlive())landTicks=Math.max(1,(int)(20*kind.duration(kind.jump[2])));wasGround=onGround();
  LivingEntity target=getTarget();
  if(target!=null&&(!target.isAlive()||target.isSpectator()||target instanceof Player p&&p.isCreative())){setTarget(null);target=null;}
  if(state!=State.HURT&&tickCount%10==0&&target==null){
   double radius=kind.sight*JakEnemyKind.SCALE;
   target=level.getEntitiesOfClass(LivingEntity.class,getBoundingBox().inflate(radius),e->e!=this&&canAttack(e)&&!(e instanceof JakEnemy)&&!(e instanceof Player p&&(p.isCreative()||p.isSpectator()))&&(e instanceof Player||kind.model.equals("predator")&&(e instanceof net.minecraft.world.entity.animal.Animal||e instanceof net.minecraft.world.entity.npc.villager.AbstractVillager)))
    .stream().filter(e->getSensing().hasLineOfSight(e)).min(Comparator.comparingDouble(this::distanceToSqr)).orElse(null);
   setTarget(target);
  }
  if(state==State.HURT){
   if(reactionAirborne&&(onGround()||isInWater())){
    reactionAirborne=false;
    if(!isInWater()&&!recovery.equals(animation())){
     animate(recovery,false,1);
     reactionEnds=stateTicks+Math.max(1,(int)Math.ceil(20*kind.duration(recovery)));
    }
   }
   if(stateTicks>=reactionEnds&&(onGround()||isInWater())){
    set(target!=null?State.CHASE:State.IDLE);updateAnimation();
   }
   sounds();return;
  }
  if(isPassenger()){animate(kind.clips[0],true,1);sounds();return;}
  if(hopper&&(windup>0||leaping||landTicks>0)){
   advanceHopper();
   sounds();return;
  }
  if(requestLeap&&onGround()&&kind.leaps&&(state==State.CHASE||state==State.WALK)&&!hopper){
   requestLeap=false;beginLeap(new Vec3(getMoveControl().getWantedX(),getMoveControl().getWantedY(),getMoveControl().getWantedZ()));
  }
  if(leapWindup>0){
   // Keep the route while gathering/airborne. Cancelling it here leaves
   // a wide creature aiming at the near face of the step on every jump.
   animate(kind.first(kind.jump[0],kind.jump[1],kind.clips[2]),false,1);
   if(--leapWindup==0){setDeltaMovement(leapMotion);setOnGround(false);needsSync=true;leaping=true;animate(kind.first(kind.jump[1],kind.clips[2]),false,1);}
   sounds();return;
  }
  if(leaping){
   if(onGround()){leaping=false;requestLeap=false;landTicks=Math.max(1,(int)(20*kind.duration(kind.jump[2])));pathWait=0;}
   else {animate(kind.first(kind.jump[1],kind.clips[2]),false,1);sounds();return;}
  }
  if(target!=null){
   double distance=Math.sqrt(distanceToSqr(target));boolean visible=getSensing().hasLineOfSight(target);
   getLookControl().setLookAt(target,30,30);
   if(distance>kind.stop*JakEnemyKind.SCALE||state==State.CHASE&&stateTicks>400){setTarget(null);set(State.GIVE_UP);target=null;}
   else if((state==State.IDLE||state==State.WALK)&&visible)set(State.NOTICE);
   else if(state==State.NOTICE&&stateTicks>=20*Math.max(.1f,kind.duration(kind.clips[1])))set(State.CHASE);
   else if(state==State.CHASE){
    if(inReach(target)&&visible){set(State.ATTACK);}
    else navigate(target.position(),true);
   }
   if(state==State.ATTACK&&spydroid)spydroidAttack(level,target);
   else if(state==State.ATTACK){
    float length=attackLength(),at=kind.model.equals("lurkercrab")?kind.duration("lurkercrab-peek")*.5f+kind.duration("lurkercrab-snip")*.4f:length*.4f;
    face(target.position());
    if(!struck&&stateTicks>=at*20){struck=true;if(inReach(target)&&visible)doHurtTarget(level,target);}
    if(stateTicks>=length*20)set(!target.isAlive()?State.CELEBRATE:State.CHASE);
   }
  }
  if(target==null){
   if(state==State.ATTACK||state==State.CHASE||state==State.NOTICE)set(State.GIVE_UP);
   if(state==State.GIVE_UP&&stateTicks>=20*kind.duration(kind.first(kind.model+"-give-up",kind.model+"-give-up-hop"))){roam=home;set(State.WALK);}
   if(state==State.IDLE&&stateTicks>=40+getId()%40){double a=random.nextDouble()*Math.PI*2,r=(1+random.nextFloat()*6);roam=home.add(Math.cos(a)*r,0,Math.sin(a)*r);set(State.WALK);}
   if(state==State.WALK){if(roam==null||position().distanceToSqr(roam)<1||stateTicks>200)set(State.IDLE);else navigate(roam,false);}
  }
  if(state==State.CELEBRATE&&stateTicks>=20*Math.max(.7f,kind.duration(kind.clips[4])))set(State.IDLE);
  updateAnimation();sounds();
 }
 private void spydroidAttack(ServerLevel level,LivingEntity target){
  float jump=kind.duration("spydroid-attack-jump");
  float land=kind.duration("spydroid-attack-land");
  float elapsed=stateTicks/20f;
  if(stateTicks==0){
   face(target.position());
   lunge=target.position().subtract(position()).multiply(1,0,1).normalize()
    .scale(12*JakEnemyKind.SCALE/20);
  }
  if(elapsed<jump){
   var before=getBoundingBox();
   var origin=position();
   setDeltaMovement(getDeltaMovement().multiply(0,1,0));
   move(MoverType.SELF,lunge);
   var swept=before.expandTowards(position().subtract(origin)).inflate(.04);
   if(!struck&&swept.intersects(target.getBoundingBox())&&getSensing().hasLineOfSight(target)){
    struck=true;
    if(doHurtTarget(level,target)){
     var away=lunge.normalize();
     target.push(away.x*3.5*JakEnemyKind.SCALE/20,2*JakEnemyKind.SCALE/20,away.z*3.5*JakEnemyKind.SCALE/20);
    }
   }
  }else if(elapsed>=jump+land)set(target.isAlive()?State.CHASE:State.CELEBRATE);
 }
 private float crabOpenSeconds(){return kind.duration("lurkercrab-peek")*1.5f+kind.duration("lurkercrab-snip");}
 private float attackLength(){return crab?crabOpenSeconds()+kind.duration("lurkercrab-peek-to-walk"):Math.max(.35f,kind.duration(kind.clips[3]));}
 private void animateCrabAttack(){
  float halfPeek=kind.duration("lurkercrab-peek")*.5f,snip=kind.duration("lurkercrab-snip");
  float elapsed=stateTicks/20f;
  int phase=elapsed<halfPeek?0:elapsed<halfPeek+snip?1:elapsed<crabOpenSeconds()-halfPeek?2:elapsed<crabOpenSeconds()?3:4;
  if(phase==crabPhase)return;
  crabPhase=phase;
  String clip=switch(phase){case 1->"lurkercrab-snip";case 4->"lurkercrab-peek-to-walk";default->"lurkercrab-peek";};
  animate(clip,false,1);
  restartAnimation();
  float offset=switch(phase){
   case 0->elapsed;
   case 1->elapsed-halfPeek;
   case 2->elapsed-snip;
   case 3->elapsed-snip-halfPeek;
   default->elapsed-crabOpenSeconds();
  };
  entityData.set(CLIP_OFFSET,Math.max(0,offset));
 }
 boolean inReach(LivingEntity target){
  // The guest reach tests a body, not an infinitesimal feet point. Keep the
  // authored radius; use Minecraft's actual hitbox (including modded sizes).
  Vec3 from=position().add(0,getBbHeight()*.5,0);AABB box=target.getBoundingBox();
  if(target.getVehicle()!=null)box=box.setMinY(Math.max(box.minY,target.getVehicle().getPassengerRidingPosition(target).y));
  Vec3 nearest=new Vec3(Math.clamp(from.x,box.minX,box.maxX),Math.clamp(from.y,box.minY,box.maxY),Math.clamp(from.z,box.minZ,box.maxZ));
  return from.distanceToSqr(nearest)<Math.pow(kind.reach*JakEnemyKind.SCALE,2);
 }
 void face(Vec3 at){double x=at.x-getX(),z=at.z-getZ();setYRot((float)Math.toDegrees(Math.atan2(-x,z)));setYBodyRot(getYRot());setYHeadRot(getYRot());}
 private void beginLeap(Vec3 goal){
  var path=getNavigation().getPath();
  if(path!=null&&path.getNextNodeIndex()+1<path.getNodeCount())goal=path.getEntityPosAtNode(this,path.getNextNodeIndex()+1);
  Vec3 d=goal.subtract(position()).multiply(1,0,1);double g=60*JakEnemyKind.SCALE,height=Math.max(kind.jumpMin*JakEnemyKind.SCALE,Math.max(d.length()*kind.jumpFactor,goal.y-getY()+.2)),v=Math.sqrt(2*g*height);
  double time=(v+Math.sqrt(Math.max(0,v*v-2*g*(goal.y-getY()))))/g;
  leapMotion=d.scale(1/Math.max(.1,time)/20).add(0,v/20,0);leapWindup=Math.max(1,(int)(20*kind.duration(kind.jump[0])));face(goal);
 }
 private void navigate(Vec3 at,boolean chasing){
  if(landTicks>0&&!isInWater()){getNavigation().stop();return;}
  if(hopper&&!isInWater()){
   getNavigation().stop();if(!onGround())return;
   Vec3 d=at.subtract(position()).multiply(1,0,1);
   float across=Math.min((float)d.length()/JakEnemyKind.SCALE,chasing?8:4);
   float height=Math.max(3,across*.5f),vertical=(float)Math.sqrt(2*100*height);
   hopperFlight=2*vertical/100;
   leapMotion=d.normalize().scale(across/hopperFlight*JakEnemyKind.SCALE/20).add(0,vertical*JakEnemyKind.SCALE/20,0);
   hopperClip=chasing?"hopper-jump-long":"hopper-jump-short";
   var clip=kind.animations.get(hopperClip);
   hopperStart=3/Math.max(1,60*clip.speed()*clip.artistStep());
   windup=Math.max(1,(int)Math.ceil(20*hopperStart));
   animate(hopperClip,false,hopperStart*20/windup);
   face(at);return;
  }
  if(pathWait==0){getNavigation().moveTo(at.x,at.y,at.z,0,chasing?1:kind.walk/kind.run);pathWait=10;}
  if(horizontalCollision&&onGround()&&kind.leaps&&!isInWater())getJumpControl().jump();
 }
 private void advanceHopper(){
  if(windup>0){
   if(--windup!=0)return;
   setDeltaMovement(leapMotion);setOnGround(false);needsSync=true;leaping=true;
   animate(hopperClip,false,(kind.duration(hopperClip)-hopperStart)/hopperFlight);
   restartAnimation();entityData.set(CLIP_OFFSET,hopperStart);
   return;
  }
  if(leaping){
   if(!onGround()&&!isInWater())return;
   leaping=false;requestLeap=false;
   if(isInWater())return;
   String landing=hopperClip+"-land";
   landTicks=10+(int)Math.ceil(kind.duration(landing)*20);
   animate(landing,false,1);return;
  }
  if(landTicks<=10)animate(kind.clips[0],true,1);
 }
 private void updateAnimation(){
  String c=switch(state){case IDLE->kind.clips[0];case NOTICE->kind.clips[1];case CHASE->kind.clips[2];case ATTACK->kind.clips[3];case CELEBRATE->kind.clips[4];case WALK->kind.clips[5];case GIVE_UP->kind.first(kind.model+"-give-up",kind.model+"-give-up-hop");default->kind.clips[0];};
  boolean loops=state==State.IDLE||state==State.CHASE||state==State.WALK;
  if(hopper&&state==State.IDLE){float idle=2,burp=kind.duration("hopper-burp");if((stateTicks/20f)%(idle*3+burp)>=idle*3){c="hopper-burp";loops=false;}}
  if(spydroid&&state==State.ATTACK)c=stateTicks/20f<kind.duration("spydroid-attack-jump")?
   "spydroid-attack-jump":"spydroid-attack-land";
  if(crab&&state==State.ATTACK){animateCrabAttack();return;}
  if(hopper&&windup>0)return;
  if(state==State.CHASE||state==State.WALK){
   if(!onGround()&&!isInWater()&&!kind.jump[1].isEmpty())c=kind.jump[1];
   else if(landTicks>0&&!kind.jump[2].isEmpty()){c=kind.jump[2];loops=false;}
   else if(isInWater())c=kind.first(kind.model+"-swim",kind.jump[1],c);
   else if(getDeltaMovement().horizontalDistanceSqr()<.0001)c=kind.clips[0];
  }
  animate(c,loops,1);
 }
 private void sounds(){
  String clip=animation();var art=kind.animations.get(clip);if(art==null)return;
  float seconds=animationSeconds(0),duration=art.duration();if(animationLoops()&&duration>0)seconds%=duration;
  float frame=art.artist(seconds),previous=clip.equals(sounding)?soundTime:art.artist(entityData.get(CLIP_OFFSET))-.001f;
  for(var cue:art.cues())if(frame<previous?animationLoops()&&(cue.frame()>previous||cue.frame()<=frame):cue.frame()>previous&&cue.frame()<=frame){var sound=JakEnemySounds.find(kind,cue.name());if(sound!=null){playSound(sound,cue.volume(),(float)Math.pow(2,cue.pitch()));JakEnemySounds.cuesPlayed++;}}
  sounding=clip;soundTime=frame;
 }
 @Override public boolean hurtServer(ServerLevel level,DamageSource source,float amount){
  if(kind==null)return super.hurtServer(level,source,amount);
  // Closed crab shell deflects ordinary blows; explosions and smash attacks bypass it.
  boolean heavy=nativeHeavy||source.is(net.minecraft.tags.DamageTypeTags.IS_EXPLOSION)||source.is(net.minecraft.tags.DamageTypeTags.BYPASSES_ARMOR);
  boolean exposed=crab&&(state==State.CELEBRATE||state==State.ATTACK&&stateTicks/20f<crabOpenSeconds());
  if(crab&&!heavy&&!exposed&&source.getEntity()!=null&&isAlive()){
   Vec3 away=position().subtract(source.getEntity().position()).multiply(1,0,1).normalize();
   setDeltaMovement(away.scale(14*JakEnemyKind.SCALE/20));needsSync=true;
   beginReaction("lurkercrab-kickback",.75f,.65f);return false;
  }
  boolean hit=super.hurtServer(level,source,crab?(exposed?2:heavy?4:1)*amount:amount);
  if(hit&&guard!=null){guard.hit(level,source);return true;}
  if(hit&&isAlive()){
   String clip=kind.first(kind.clips[6],kind.model+"-stun",kind.model+"-take-hit",kind.model+"-both-take-hit",kind.model+"-kickback",kind.clips[7]);
   beginReaction(clip,crab?.75f:1,kind.clips[6].isEmpty()?.45f:Math.max(.7f,kind.duration(clip)));
   if(source.getEntity() instanceof LivingEntity attacker){setTarget(attacker);
    boolean thrown=!kind.clips[6].isEmpty()&&!kind.clips[6].contains("stun")&&!kind.clips[6].contains("take-hit")&&!kind.clips[6].contains("kickback");
    if(thrown&&!isInWater()){
     Vec3 away=position().subtract(attacker.position()).multiply(1,0,1).normalize();
     setDeltaMovement(away.scale(6*JakEnemyKind.SCALE/20).add(0,5*JakEnemyKind.SCALE/20,0));
     setOnGround(false);needsSync=true;reactionAirborne=true;
    }
   }
  }
  return hit;
 }
 private void beginReaction(String clip,float rate,float seconds){
  set(State.HURT);
  animate(clip,false,rate);
  reactionEnds=Math.max(1,(int)Math.ceil(seconds*20));
 }
 public boolean hurtFromJak(ServerLevel level,DamageSource source,float damage,boolean heavy){nativeHeavy=heavy;try{return hurtServer(level,source,damage);}finally{nativeHeavy=false;}}
 public com.google.gson.JsonObject diagnostics(){
  var out=new com.google.gson.JsonObject();out.addProperty("state",state.toString());out.addProperty("clip",animation());out.addProperty("path_done",getNavigation().isDone());out.addProperty("on_ground",onGround());out.addProperty("horizontal_collision",horizontalCollision);out.addProperty("leap_windup",leapWindup);out.addProperty("leaping",leaping);out.addProperty("queued_jump",requestLeap);out.addProperty("position",position().toString());out.addProperty("wanted",new Vec3(getMoveControl().getWantedX(),getMoveControl().getWantedY(),getMoveControl().getWantedZ()).toString());out.addProperty("velocity",getDeltaMovement().toString());if(getTarget()!=null)out.addProperty("target_distance",Math.sqrt(distanceToSqr(getTarget())));return out;
 }
 @Override public void die(DamageSource source){
  super.die(source);if(level().isClientSide()||kind==null)return;
  guardShot("");set(State.DEAD);animate(kind.clips[7],false,1);sounds();
  if(spydroid&&level() instanceof ServerLevel server){
   var at=position().add(0,.5*JakEnemyKind.SCALE,0);
   SettlementShots.cue(server,"jak3","droid-explode",at);
   var packet=new dev.jakmc.network.JakPackets.CreatureBurst(server.dimension().identifier(),at);
   for(var watcher:net.fabricmc.fabric.api.networking.v1.PlayerLookup.tracking(this))
    if(net.fabricmc.fabric.api.networking.v1.ServerPlayNetworking.canSend(watcher,packet.type()))
     net.fabricmc.fabric.api.networking.v1.ServerPlayNetworking.send(watcher,packet);
  }
  if(kind.jak1()){Vec3 away=source.getEntity()==null?Vec3.ZERO:position().subtract(source.getEntity().position()).multiply(1,0,1).normalize();setDeltaMovement(away.scale(5*JakEnemyKind.SCALE/20).add(0,16*JakEnemyKind.SCALE/20,0));needsSync=true;}
 }
 @Override protected void tickDeath(){
  deathTime++;if(!level().isClientSide())sounds();
  if(deathTime>=(kind.guard()?120:Math.max(20,Math.min(100,(int)(20*kind.duration(kind.clips[7])))))&&!level().isClientSide()&&!isRemoved()){level().broadcastEntityEvent(this,(byte)60);remove(RemovalReason.KILLED);}
 }
 @Override protected void dropCustomDeathLoot(ServerLevel level,DamageSource source,boolean playerKill){
  super.dropCustomDeathLoot(level,source,playerKill);
  if(guard!=null){guard.dropAmmo(level);return;}
  // Eligibility is independent of whether the model has a physical gem joint.
  if(kind.metalHead)dev.jakmc.content.GemPhysics.launch(spawnAtLocation(level,new net.minecraft.world.item.ItemStack(JakItems.SKULL_GEM)));
  spawnAtLocation(level,new net.minecraft.world.item.ItemStack(kind.jak1()?JakItems.LIGHT_ECO:JakItems.DARK_ECO));
 }
 @Override protected SoundEvent getHurtSound(DamageSource source){return kind==null?null:JakEnemySounds.first(kind,kind.model+"-hit",kind.model.equals("predator")?"pred-hit":"IMPACT_SOFT");}
 @Override protected SoundEvent getDeathSound(){return null;} // Authored art-group event, not a second arbitrary death cue.
 @Override protected SoundEvent getSwimSplashSound(){var s=kind==null?null:JakEnemySounds.find(kind,"splash");return s==null?super.getSwimSplashSound():s;}
 @Override protected SoundEvent getSwimSound(){var s=kind==null?null:JakEnemySounds.first(kind,"walk-dwater1","splash-out");return s==null?super.getSwimSound():s;}
 @Override protected double getDefaultGravity(){return kind!=null&&!isInWater()?super.getDefaultGravity()*((kind.model.equals("hopper")||spydroid?100:60)*JakEnemyKind.SCALE/400)/.08:super.getDefaultGravity();}
 @Override protected void addAdditionalSaveData(ValueOutput out){super.addAdditionalSaveData(out);if(guard!=null)guard.save(out);if(home!=null){out.putDouble("JakHomeX",home.x);out.putDouble("JakHomeY",home.y);out.putDouble("JakHomeZ",home.z);}}
 @Override protected void readAdditionalSaveData(ValueInput in){super.readAdditionalSaveData(in);if(guard!=null)guard.load(in);home=new Vec3(in.getDoubleOr("JakHomeX",getX()),in.getDoubleOr("JakHomeY",getY()),in.getDoubleOr("JakHomeZ",getZ()));set(isDeadOrDying()?State.DEAD:State.IDLE);}
}
