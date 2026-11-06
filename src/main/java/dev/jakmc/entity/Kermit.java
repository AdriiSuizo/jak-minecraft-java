package dev.jakmc.entity;

import net.minecraft.network.protocol.game.ClientboundStopSoundPacket;
import net.minecraft.server.level.ServerLevel;
import net.minecraft.server.level.ServerPlayer;
import net.minecraft.sounds.SoundSource;
import net.minecraft.world.entity.LivingEntity;
import net.minecraft.world.entity.MoverType;
import net.minecraft.world.level.ClipContext;
import net.minecraft.world.phys.HitResult;
import net.minecraft.world.phys.Vec3;

/**
 * Jak 1's swamp frog, `kermit` (levels/swamp/kermit.gc), function by function: it hops to keep 10 m
 * from its prey, lashes its tongue out to 12 m when its head is within 20 degrees of it, and a tongue
 * that lands sticks while an electric pulse runs down it (`kermit-pulse`), shocking what it touches.
 * Its tongue pulls Jak's steering toward it (`target-apply-tongue`, JakPlayers.tongue).
 * stand-in: the original hunts only Jak; here, asked for, also the world's creatures (JakEnemy
 * targeting), which the tongue holds and shocks but does not steer. There is no nav mesh: the
 * original's nav-mesh tests (frustration, the side hop's landing spot) become line of sight and
 * Minecraft's own collision. The head's neck tracking is not ported: the body's heading stands for it.
 */
final class Kermit {
 private enum Phase{IDLE,PATROL,NOTICE,CHASE,NEW_POSITION,ATTACK,STUCK,RETRACT}
 /** Blocks a creature metre. */
 private static final double M=JakEnemyKind.SCALE;
 // kermit.gc: kermit-check-to-hit-player? 49152 and 3640.889, kermit-set-nav-mesh-target 40960, stuck 73728, the tracker's 1/49152.
 private static final double REACH=12,KEEP=10,BREAK=18,TONGUE=12;
 private static final double TWIST=Math.cos(Math.toRadians(20));
 /** kermit-pulse-init-by-other: its sphere, 2867.2 units. */
 private static final double PULSE_RADIUS=.7;
 /** tongueA (skin joint 23; node 24 in the original) in kermit-lod0.glb's bind pose, in creature metres from the feet. */
 private static final Vec3 MOUTH=new Vec3(0,1.12,1.191);
 /**
  * stand-in: the original's hop follows the nav mesh's ground; here its stride (kermit-post's move) climbs a block,
  * as Minecraft's step does, so a frog is not stopped by a one-block rise (Adrii 06/11).
  */
 static final double STEP=1;
 private final JakEnemy body;
 private Phase phase=Phase.IDLE;
 private int ticks,clipTicks,hops,lost,pulseTicks;
 private String clip="",step="",hopClip="";
 private float rate=1;
 private boolean airborne,decided,hit,enabled,pulsing;
 private float control,max=2,pulse,goal=1;
 private double speed;
 private Vec3 nav,home;
 Kermit(JakEnemy body){this.body=body;}

 private JakEnemyKind kind(){return body.kind;}
 private Vec3 home(){if(home==null)home=body.position();return home;}
 private void go(Phase next){
  if(phase==Phase.STUCK)endPulse();
  if(phase==Phase.RETRACT){control=0;enabled=false;}
  phase=next;ticks=0;hops=0;step="";hopClip="";airborne=false;clip="";
  body.set(switch(next){case IDLE->JakEnemy.State.IDLE;case PATROL->JakEnemy.State.WALK;case NOTICE->JakEnemy.State.NOTICE;
   case CHASE,NEW_POSITION->JakEnemy.State.CHASE;default->JakEnemy.State.ATTACK;});
  switch(next){
   case PATROL->{double a=body.getRandom().nextDouble()*Math.PI*2,r=(2+body.getRandom().nextDouble()*6)*M;nav=home().add(Math.cos(a)*r,0,Math.sin(a)*r);}
   case NEW_POSITION->decided=false;
   case ATTACK->{control=0;enabled=true;decided=false;hit=false;goal=1;}
   case STUCK->{
    control=1;enabled=true;pulse=0;pulsing=true;pulseTicks=0;
    // kermit-pulse-idle: "kermit-loop" from the pulse.
    var loop=JakEnemySounds.find(kind(),"kermit-loop");
    if(loop!=null){Vec3 at=mouth();body.level().playSound(null,at.x,at.y,at.z,loop,SoundSource.HOSTILE,1,1);}
   }
   default->{}
  }
 }
 /** Plays `name` from its start at `rate` (or keeps it); true while it runs. */
 private boolean play(String name,float rate,boolean restart){
  if(!name.equals(clip)||restart){clip=name;this.rate=rate;clipTicks=0;body.animate(name,false,rate);body.restartAnimation();}
  return clipTicks<Math.max(1,Math.round(kind().duration(name)/rate*20));
 }
 /** The authored frame `aframe` of the playing `name` reached (ja-aframe-num). */
 private boolean past(String name,float aframe){
  var art=kind().animations.get(name);if(art==null)return true;
  return art.artist(clipTicks/20f*rate)>=aframe;
 }

 void tick(ServerLevel level,LivingEntity target){
  home();ticks++;clipTicks++;
  if(target!=null&&!target.isAlive())target=null;
  if(target!=null&&body.getSensing().hasLineOfSight(target))lost=0;else lost++;
  switch(phase){
   // kermit-idle / kermit-patrol: an idle at triple speed, a 5 m hop about home, until something is near.
   case IDLE->{
    enabled=false;loopIdle(3);
    if(target!=null)go(Phase.NOTICE);
    else if(ticks>40)go(Phase.PATROL);
   }
   case PATROL->{
    if(target!=null&&!airborne){go(Phase.NOTICE);break;}
    if(hopCycle(3,5,nav,nav))nav=null;
    if(nav==null||ticks>20*12)go(Phase.IDLE);
   }
   // kermit-notice: one hop in place, turning to face it.
   case NOTICE->{
    if(target==null){go(Phase.IDLE);break;}
    if(hops>0&&!hopping()){go(Phase.CHASE);break;}
    if(hops==0)hop(0,body.position(),target.position());else hop(-1,null,target.position());
   }
   // kermit-chase: keep 10 m off, lash when the head is on it and the tongue clear, side-step when blocked.
   case CHASE->{
    if(target==null||body.distanceTo(target)>kind().stop*M){go(Phase.PATROL);break;}
    nav=keep(target.position());
    boolean aimed=onTarget(target);
    if(!airborne){
     if(aimed){go(clear(target)?Phase.ATTACK:Phase.NEW_POSITION);break;}
     // stand-in for the nav mesh's 3 s off it: 3 s out of sight gives up.
     if(lost>60){body.setTarget(null);go(Phase.PATROL);break;}
    }
    hopCycle(3,aimed?0:20,nav,target.position());
   }
   // kermit-chase-new-position: three idles at four times speed, then hops across its line, up to 10 m either side.
   case NEW_POSITION->{
    if(target==null){go(Phase.PATROL);break;}
    if(!decided){
     decided=true;Vec3 side=heading().yRot((float)(Math.PI/2));
     nav=keep(body.position().add(side.scale((body.getRandom().nextDouble()*2-1)*10*M)),target.position());
    }
    if(!airborne&&(ticks>=60||ticks>=10&&near(nav))){go(Phase.CHASE);break;}
    if(ticks<Math.round(3*kind().duration("kermit-idle")/4*20))loopIdle(4);
    else hopCycle(-1,20,nav,target.position());
   }
   // kermit-attack: the lash; at its 14th frame on target or not; a miss stops short at 0.8 and pulls back.
   case ATTACK->{
    if(target==null){go(Phase.RETRACT);break;}
    body.face(target.position());
    boolean running=play("kermit-lash",1,false);
    if(!decided&&past("kermit-lash",14)){decided=true;hit=onTarget(target);if(!hit)goal=.8f;}
    if(decided){
     if(!hit&&control==goal){go(Phase.RETRACT);break;}
     control=seek(control,goal,.25f*3);
    }
    clear(target);
    if(!running)go(hit?Phase.STUCK:Phase.RETRACT);
   }
   // kermit-tongue-stuck: the pulse runs base to prey at 0.3 a second; too far, blocked or the pulse gone lets go.
   case STUCK->{
    if(target==null||!pulsing){go(Phase.RETRACT);break;}
    Vec3 base=mouth(),to=aim(target);
    if(to.distanceTo(base)>BREAK*M||!clear(target)){go(Phase.RETRACT);break;}
    pulse=Math.min(1,pulse+.3f/20);
    // kermit-pulse-idle: four seconds at most.
    if(++pulseTicks>=80){go(Phase.RETRACT);break;}
    Vec3 at=base.lerp(to,pulse);
    if(target.getBoundingBox().inflate(PULSE_RADIUS*M).contains(at)){impact(level,target,at);go(Phase.RETRACT);break;}
    if(target instanceof ServerPlayer player)dev.jakmc.player.JakPlayers.tongue(player,base);
    body.face(target.position());
    if(!play("kermit-turn",1,false)){
     play("kermit-turn",1,true);
     var stretch=JakEnemySounds.find(kind(),"kermit-stretch");if(stretch!=null)body.playSound(stretch,1,1);
    }
   }
   // kermit-retract-tongue: kermit-miss, the tongue drawn back from its 3rd frame; then hops in place; 2.5 s, back to the chase.
   case RETRACT->{
    if(!airborne&&ticks>=50){go(target==null?Phase.PATROL:Phase.CHASE);break;}
    if(ticks==1)play("kermit-miss",1,true);
    if(clip.equals("kermit-miss")){
     if(play("kermit-miss",1,false)){if(past("kermit-miss",3))control=seek(control,0,.125f*3);}
     else{control=0;enabled=false;clip="";}
    }else hopCycle(4,0,body.position(),target==null?null:target.position());
   }
  }
  body.tongue(enabled&&target!=null?target.getId():-1,control,max,phase==Phase.STUCK&&pulsing?pulse:-1);
 }
 void stop(){if(phase==Phase.STUCK)endPulse();enabled=false;control=0;body.tongue(-1,0,2,-1);}

 /** An idle at `idleRate` (none at -1), then a hop of up to `metres` toward `to`; true once there. */
 private boolean hopCycle(float idleRate,double metres,Vec3 to,Vec3 face){
  if(step.isEmpty()){
   if(idleRate>0){step="idle";play("kermit-idle",idleRate,true);}
   else{step="hop";hop(metres,to,face);return false;}
  }
  if(step.equals("idle")){
   if(play("kermit-idle",idleRate,false))return false;
   step="hop";hop(metres,to,face);return false;
  }
  if(hopping()){hop(-1,null,face);return false;}
  step="";return to!=null&&near(to);
 }
 private void loopIdle(float rate){if(!play("kermit-idle",rate,false))play("kermit-idle",rate,true);}
 private boolean hopping(){return !hopClip.isEmpty();}
 /** kermit-hop: up to `metres` toward `to` (-1: the hop under way); over 4 m the long hop, quicker for its length. */
 private void hop(double metres,Vec3 to,Vec3 face){
  if(metres>=0){
   double d=to==null?0:Math.min(metres,to.subtract(body.position()).horizontalDistance()/M);
   hopClip=d>4?"kermit-longhop":"kermit-hop";
   // target-speed: (fps/22) or (fps/10) times the distance, a second.
   speed=(d>4?60/22.0:6)*d;nav=to;hops++;
   play(hopClip,1,true);airborne=false;
  }
  if(hopClip.isEmpty())return;
  boolean running=play(hopClip,1,false),longHop=hopClip.equals("kermit-longhop");
  // kermit-short-hop / kermit-long-hop: off the ground from frame 4 to 14, or 7 to 29.
  airborne=running&&past(hopClip,longHop?7:4)&&!past(hopClip,longHop?29:14);
  if(airborne){
   // kermit-post: along the ground toward the target point, turned (kermit-set-rotate-dir-to-player) from it to the prey.
   // A patrol (face is its point): kermit-set-rotate-dir-to-nav-target instead.
   if(face!=null){Vec3 from=nav==null||face==nav?body.position():nav;Vec3 dir=face.subtract(from).multiply(1,0,1);if(dir.lengthSqr()>1e-6)body.face(body.position().add(dir));}
   if(nav!=null){
    Vec3 across=nav.subtract(body.position()).multiply(1,0,1);
    double stride=Math.min(speed*M/20,across.length());
    if(stride>1e-4){body.setDeltaMovement(body.getDeltaMovement().multiply(0,1,0));body.move(MoverType.SELF,across.normalize().scale(stride));}
   }
  }
  if(!running){hopClip="";airborne=false;}
 }
 private boolean near(Vec3 at){return at==null||at.subtract(body.position()).horizontalDistance()<.5*M;}
 /** kermit-set-nav-mesh-target: 10 m from the prey on the line through `from`. */
 private Vec3 keep(Vec3 prey){return keep(body.position(),prey);}
 private Vec3 keep(Vec3 from,Vec3 prey){
  Vec3 away=from.subtract(prey).multiply(1,0,1);if(away.lengthSqr()<1e-6)away=heading().scale(-1);
  return new Vec3(prey.x,from.y,prey.z).add(away.normalize().scale(KEEP*M));
 }
 private Vec3 heading(){double yaw=Math.toRadians(body.getYRot());return new Vec3(-Math.sin(yaw),0,Math.cos(yaw));}
 /** kermit-tongue-pos: tongueA, from its bind pose turned with the body. */
 Vec3 mouth(){return body.position().add(MOUTH.scale(M).yRot((float)-Math.toRadians(body.getYRot())));}
 /** kermit-player-target-pos (Jak's joint 26). stand-in: the middle of its box, for any prey. */
 static Vec3 aim(net.minecraft.world.entity.Entity target){return target.position().add(0,target.getBbHeight()*.5,0);}
 /** kermit-check-to-hit-player?: within 12 m of the tongue and the head within 20 degrees of it. */
 private boolean onTarget(LivingEntity target){
  Vec3 to=aim(target).subtract(mouth());double d=to.length();
  if(d<1e-6)return true;
  Vec3 flat=new Vec3(to.x,0,to.z);
  return d<REACH*M&&(flat.lengthSqr()<1e-9||heading().dot(flat.normalize())>=TWIST);
 }
 /** kermit-check-tongue-is-clear?: the line to the prey free of the world, and the tongue's longest scale from it. */
 private boolean clear(LivingEntity target){
  Vec3 from=mouth(),to=aim(target);
  var hit=body.level().clip(new ClipContext(from,to,ClipContext.Block.COLLIDER,ClipContext.Fluid.NONE,body));
  boolean clear=hit.getType()==HitResult.Type.MISS;
  max=clear?2:(float)(hit.getLocation().distanceTo(from)/M/TONGUE);
  return clear;
 }
 private static float seek(float v,float goal,float by){return v<goal?Math.min(goal,v+by):Math.max(goal,v-by);}
 /** kermit-pulse-impact: "get-shocked", group-kermit-pulse-impact (the client, from KERMIT_IMPACT) and the pulse's attack. */
 private void impact(ServerLevel level,LivingEntity target,Vec3 at){
  endPulse();
  var shocked=JakEnemySounds.find(kind(),"get-shocked");
  if(shocked!=null)level.playSound(null,at.x,at.y,at.z,shocked,SoundSource.HOSTILE,1,1);
  level.broadcastEntityEvent(body,JakEnemy.KERMIT_IMPACT);
  body.doHurtTarget(level,target);
 }
 private void endPulse(){
  if(!pulsing)return;pulsing=false;
  // kermit-pulse-idle :exit stops its loop.
  var loop=JakEnemySounds.find(kind(),"kermit-loop");
  if(loop!=null&&body.level() instanceof ServerLevel level)
   for(var player:level.players())if(player.distanceToSqr(body)<64*64)
    player.connection.send(new ClientboundStopSoundPacket(loop.location(),SoundSource.HOSTILE));
 }
}
