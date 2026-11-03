package dev.jakmc.entity;

import com.google.gson.JsonArray;
import com.google.gson.JsonObject;
import dev.jakmc.content.JakItems;
import java.util.ArrayList;
import java.util.Comparator;
import net.minecraft.core.BlockPos;
import net.minecraft.server.level.ServerLevel;
import net.minecraft.sounds.SoundSource;
import net.minecraft.world.damagesource.DamageSource;
import net.minecraft.world.entity.LivingEntity;
import net.minecraft.world.entity.npc.villager.Villager;
import net.minecraft.world.entity.player.Player;
import net.minecraft.world.item.ItemStack;
import net.minecraft.world.level.ClipContext;
import net.minecraft.world.level.storage.ValueInput;
import net.minecraft.world.level.storage.ValueOutput;
import net.minecraft.world.phys.AABB;
import net.minecraft.world.phys.HitResult;
import net.minecraft.world.phys.Vec3;

public final class KrimsonGuard {
    private static final String PREFIX="crimson-guard-";
    private static final double SCALE=JakEnemyKind.SCALE, SHOT_SPEED=200*.64;
    private static final int BURST=4;
    private static final String[] STANDING={"idle-to-stab-idle","gun-attack","stab-idle-to-idle"};
    private static final String[] KNEELING={"attack-shoot-knee-start","attack-shoot-knee","attack-shoot-knee-end"};
    private final JakEnemy body;
    private final ArrayList<Shot> shots=new ArrayList<>();
    private BlockPos start,end;
    private boolean toEnd;
    private long hostileUntil;
    private int hidden,ticks,shotIndex;
    private int pause,pathRetry,civilianWarnings;
    private long lastCivilianHit;
    private java.util.UUID lastOffender;
    private boolean kneeling;
    private double miss,side;
    private Vec3 lastFoePosition,foeVelocity=Vec3.ZERO;
    private record Shot(Vec3 head,Vec3 direction,int age,boolean againstPlayer){}

    KrimsonGuard(JakEnemy body){this.body=body;}
    static double navigationSpeed(double guestMetresPerSecond){
        // Mob writes its speed into forward input as well as travel acceleration.
        // Solve the resulting squared input with Minecraft's standard ground drag.
        return Math.sqrt(guestMetresPerSecond*SCALE/20*(1-.6*.91)/.98);
    }
    void patrol(BlockPos a,BlockPos b){start=a;end=b;toEnd=true;}
    private void state(JakEnemy.State state){body.set(state);ticks=0;}
    private void clip(String name,boolean loop){body.animate(PREFIX+name,loop,1);}
    private int length(String name){return Math.max(1,Math.round(body.kind.duration(PREFIX+name)*20));}

    void tick(ServerLevel level){
        tickShots(level);
        ticks++;
        LivingEntity foe=body.getTarget();
        if(foe!=null&&(!foe.isAlive()||foe.isSpectator()||foe instanceof Player player&&player.isCreative()
            ||level.getGameTime()>hostileUntil||body.distanceToSqr(foe)>Math.pow(110*.64,2))){
            body.setTarget(null);foe=null;
        }
        if(foe==null&&body.tickCount%20==0){
            foe=level.getEntitiesOfClass(JakEnemy.class,body.getBoundingBox().inflate(25*.64),
                enemy->!enemy.kind.guard()&&enemy.isAlive()&&body.getSensing().hasLineOfSight(enemy))
                .stream().min(Comparator.comparingDouble(body::distanceToSqr)).orElse(null);
            if(foe!=null)alert(level,foe);
        }
        if(body.state()==JakEnemy.State.HURT){
            clip("yellow-hit0",false);
            if(ticks>=length("yellow-hit0"))state(foe==null?JakEnemy.State.IDLE:JakEnemy.State.CHASE);
            return;
        }
        if(foe==null){
            patrolTick();
            return;
        }
        Vec3 current=foe.position();
        foeVelocity=lastFoePosition==null?Vec3.ZERO:current.subtract(lastFoePosition).scale(20);
        lastFoePosition=current;
        boolean visible=body.getSensing().hasLineOfSight(foe);
        hidden=visible?0:hidden+1;
        if(hidden>100){body.setTarget(null);state(JakEnemy.State.IDLE);return;}
        switch(body.state()){
            case IDLE,WALK -> {state(JakEnemy.State.NOTICE);clip("notice",false);}
            case NOTICE -> {
                body.face(foe.position());clip("notice",false);
                if(ticks>=length("notice"))state(JakEnemy.State.CHASE);
            }
            case CHASE -> {
                if(visible&&body.distanceToSqr(foe)<Math.pow(3*SCALE,2)&&body.inReach(foe)){
                    state(JakEnemy.State.ATTACK);clip("rifle-butt",false);
                }else if(visible&&body.distanceToSqr(foe)<25*.64*25*.64){
                    state(JakEnemy.State.CELEBRATE);kneeling=body.getRandom().nextBoolean();
                    shotIndex=0;miss=foe instanceof Player?4*SCALE:0;side=body.getRandom().nextBoolean()?1:-1;
                    clip((kneeling?KNEELING:STANDING)[0],false);
                }else{
                    if(ticks%8==1)body.getNavigation().moveTo(foe,1);
                    clip(body.getNavigation().isDone()?"idle":"run",true);
                }
            }
            case ATTACK -> {
                body.face(foe.position());int duration=length("rifle-butt");
                if(ticks==Math.max(1,Math.round(duration*.4f))&&visible&&body.inReach(foe)){
                    body.doHurtTarget(level,foe);cue(level,"guard-zap",foe.position());
                }
                if(ticks>=duration+14)state(JakEnemy.State.CHASE);
            }
            case CELEBRATE -> {
                body.face(foe.position());
                String[] sequence=kneeling?KNEELING:STANDING;
                int raise=length(sequence[0]),fire=length(sequence[1]),lower=length(sequence[2]);
                if(ticks>=raise&&ticks<raise+fire*BURST){
                    int round=(ticks-raise)/fire;
                    if(round==shotIndex){
                        shotIndex++;body.restartAnimation();clip(sequence[1],false);
                        shoot(level,foe);
                    }
                }else if(ticks>=raise+fire*BURST)clip(sequence[2],false);
                if(ticks>=raise+fire*BURST+lower||!visible&&hidden>20)state(JakEnemy.State.CHASE);
            }
            default -> state(JakEnemy.State.IDLE);
        }
    }

    private void patrolTick(){
        if(body.state()!=JakEnemy.State.IDLE&&body.state()!=JakEnemy.State.WALK)state(JakEnemy.State.IDLE);
        if(start==null||end==null){clip("idle",true);return;}
        if(pause>0){pause--;clip("idle",true);return;}
        if(pathRetry>0)pathRetry--;
        BlockPos goal=toEnd?end:start;
        if(body.position().distanceToSqr(Vec3.atBottomCenterOf(goal))<2.25){
            toEnd=!toEnd;pause=20;state(JakEnemy.State.IDLE);clip("idle",true);return;
        }
        // Retain one street destination across navigation retries and temporary obstacles.
        if(body.getNavigation().isDone()&&pathRetry==0){
            if(body.getNavigation().moveTo(goal.getX()+.5,goal.getY(),goal.getZ()+.5,0,navigationSpeed(body.kind.walk)/navigationSpeed(body.kind.run)))
                state(JakEnemy.State.WALK);
            pathRetry=20;
        }
        clip(body.getNavigation().isDone()?"idle":"walk",true);
    }

    private void alert(ServerLevel level,LivingEntity foe){
        if(foe instanceof Player p&&(p.isCreative()||p.isSpectator()))return;
        if(body.getTarget()!=foe){hidden=0;lastFoePosition=null;}
        body.setTarget(foe);hostileUntil=level.getGameTime()+600;
    }
    void hit(ServerLevel level,DamageSource source){
        if(body.isAlive())state(JakEnemy.State.HURT);
        if(!(source.getEntity() instanceof LivingEntity attacker)||attacker==body)return;
        alert(level,attacker);
        if(attacker instanceof Player)
            for(var guard:level.getEntitiesOfClass(JakEnemy.class,body.getBoundingBox().inflate(32),e->e.guardController()!=null&&e.isAlive()))
                guard.guardController().alert(level,attacker);
    }
    public static void witness(ServerLevel level,Villager villager,Player player){
        for(var entity:level.getEntitiesOfClass(JakEnemy.class,villager.getBoundingBox().inflate(24),
            e->e.guardController()!=null&&e.isAlive()&&e.getSensing().hasLineOfSight(villager))){
            var guard=entity.guardController();
            long now=level.getGameTime();
            if(now-guard.lastCivilianHit>160||!player.getUUID().equals(guard.lastOffender))guard.civilianWarnings=0;
            guard.lastOffender=player.getUUID();
            guard.lastCivilianHit=now;
            if(++guard.civilianWarnings>=2)guard.alert(level,player);
        }
    }

    private void shoot(ServerLevel level,LivingEntity foe){
        Vec3 forward=Vec3.directionFromRotation(0,body.getYRot());
        Vec3 muzzle=body.position().add(0,1.45*SCALE,0).add(forward.scale(.7*SCALE));
        Vec3 aim=foe.getBoundingBox().getCenter();
        Vec3 motion=foeVelocity;
        miss=Math.max(0,miss-(motion.length()<.01*.64?3:1)*SCALE);
        Vec3 offset=new Vec3(-forward.z,0,forward.x).scale(side*miss);
        Vec3 predicted=aim.add(motion.scale(muzzle.distanceTo(aim)/SHOT_SPEED)).add(offset);
        shots.add(new Shot(muzzle,predicted.subtract(muzzle).normalize(),0,foe instanceof Player));
        cue(level,"guard-shot-fire",muzzle);
    }

    private void tickShots(ServerLevel level){
        if(shots.isEmpty()){if(!body.guardShot().isEmpty())body.guardShot("");return;}
        var frame=new JsonArray();
        var it=shots.listIterator();
        while(it.hasNext()){
            Shot shot=it.next();
            Vec3 at=shot.head(),dir=shot.direction(),dest=at.add(dir.scale(SHOT_SPEED/20));
            if(!level.hasChunkAt(BlockPos.containing(dest))){it.remove();continue;}
            var wall=level.clip(new ClipContext(at,dest,ClipContext.Block.COLLIDER,ClipContext.Fluid.NONE,body));
            boolean blocked=wall.getType()!=HitResult.Type.MISS;
            if(blocked)dest=wall.getLocation();
            double closest=at.distanceToSqr(dest);
            LivingEntity hit=null;
            for(var target:level.getEntitiesOfClass(LivingEntity.class,new AABB(at,dest).inflate(.2*.64),
                e->e!=body&&e.isAlive()&&(shot.againstPlayer()?e instanceof Player p&&!p.isCreative()&&!p.isSpectator():e==body.getTarget()))){
                var bounds=target.getBoundingBox().inflate(.2*.64);
                Vec3 intersection=bounds.contains(at)?at:bounds.clip(at,dest).orElse(null);
                if(intersection==null||at.distanceToSqr(intersection)>=closest)continue;
                closest=at.distanceToSqr(intersection);dest=intersection;hit=target;
            }
            Vec3 tail=dest.subtract(dir.scale(Math.min(9*.64,at.distanceTo(dest)+SHOT_SPEED/20*shot.age())));
            var row=new JsonObject();
            row.add("head",JakVehicle.jsonVector(dest));row.add("tail",JakVehicle.jsonVector(tail));
            row.add("direction",JakVehicle.jsonVector(dir));row.addProperty("hit",blocked||hit!=null);
            row.addProperty("actor",hit!=null);frame.add(row);
            if(hit!=null)hit.hurtServer(level,body.damageSources().mobAttack(body),2);
            if(blocked||hit!=null)cue(level,"guard-shot-hit",dest);
            if(blocked||hit!=null||shot.age()>=9)it.remove();
            else it.set(new Shot(dest,dir,shot.age()+1,shot.againstPlayer()));
        }
        body.guardShot(frame.isEmpty()?"":frame.toString());
    }

    private static void cue(ServerLevel level,String name,Vec3 at){
        var event=JakEnemySounds.find("jak2",name);
        if(event!=null)level.playSound(null,at.x,at.y,at.z,event,SoundSource.HOSTILE,1,1);
    }
    void dropAmmo(ServerLevel level){
        int draw=body.getRandom().nextInt(4);
        int color=draw==0?0:draw==1?2:1;
        body.spawnAtLocation(level,new ItemStack(JakItems.AMMUNITION.get(color),color==0?5:10));
    }
    void save(ValueOutput out){
        if(start!=null&&end!=null){out.putLong("JakPatrolStart",start.asLong());out.putLong("JakPatrolEnd",end.asLong());out.putBoolean("JakPatrolToEnd",toEnd);}
    }
    void load(ValueInput in){
        long a=in.getLongOr("JakPatrolStart",Long.MIN_VALUE),b=in.getLongOr("JakPatrolEnd",Long.MIN_VALUE);
        if(a!=Long.MIN_VALUE&&b!=Long.MIN_VALUE){patrol(BlockPos.of(a),BlockPos.of(b));toEnd=in.getBooleanOr("JakPatrolToEnd",true);}
    }
}
