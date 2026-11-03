package dev.jakmc.entity;

import dev.jakmc.player.JakEntityClock;
import java.util.Comparator;
import java.util.HashMap;
import java.util.Map;
import net.minecraft.server.level.ServerLevel;
import net.minecraft.util.Mth;
import net.minecraft.world.entity.LivingEntity;
import net.minecraft.world.entity.npc.villager.Villager;
import net.minecraft.world.entity.player.Player;
import net.minecraft.world.phys.Vec3;

/** Spargus guards their own streets. Vanilla villager activities resume after combat. */
public final class SpargusDefense {
    private enum Phase {DRAW,RAISE,TURN_START,TURN,TURN_END,AIM,FIRE,HOLSTER,MELEE}
    private static final Map<String,Map<String,JakEnemyKind.Animation>> ART=new HashMap<>();
    private final Villager body;
    private final CitizenState state;
    private final CitizenKind kind,rig;
    private final SettlementShots shots;
    private Phase phase;
    private LivingEntity foe;
    private Vec3 home;
    private int age,hidden,pathRetry,scanDelay,cooldown;
    private boolean dealt;

    public SpargusDefense(Villager body,CitizenKind kind){
        this.body=body;this.kind=kind;state=(CitizenState)body;
        rig=kind.monk()?CitizenKind.of(CitizenKind.SPARGUS_FEMALE):kind;
        shots=new SettlementShots(body,2,"jak3","wlander-fire");
    }
    public void retaliate(LivingEntity attacker){if(eligible(attacker)){foe=attacker;hidden=0;}}
    private boolean eligible(LivingEntity actor){
        return actor!=body&&actor.isAlive()&&!actor.isSpectator()&&!(actor instanceof Player p&&p.isCreative());
    }
    public void tickShots(ServerLevel level){state.citizenDefenseShot(shots.tick(level));}
    public void interrupt(){phase(null);}
    public boolean tick(ServerLevel level){
        if(home==null)home=body.position();
        if(!body.isAlive()||body.isPassenger()||state.citizenPhase()!=0||body.isInWater()){
            phase(null);
            return false;
        }
        if(foe!=null){
            hidden=body.getSensing().hasLineOfSight(foe)?0:hidden+1;
            if(!eligible(foe)||hidden>80||foe.position().distanceToSqr(home)>1600)foe=null;
        }
        if(foe==null&&--scanDelay<=0){
            scanDelay=12+(body.getId()&7);
            foe=level.getEntitiesOfClass(net.minecraft.world.entity.monster.Monster.class,body.getBoundingBox().inflate(18,6,18),
                actor->eligible(actor)&&!(actor instanceof JakEnemy enemy&&enemy.kind.guard())&&actor.position().distanceToSqr(home)<1024
                    &&body.getSensing().hasLineOfSight(actor))
                .stream().min(Comparator.comparingDouble(body::distanceToSqr)).orElse(null);
            if(foe!=null)hidden=0;
        }
        if(foe==null){
            if(phase!=null&&phase!=Phase.MELEE){
                if(phase!=Phase.HOLSTER)phase(Phase.HOLSTER);
                else advance(level);
                return true;
            }
            phase(null);
            return false;
        }
        if(body.isSleeping()){body.stopSleeping();return false;}
        if(cooldown>0)cooldown--;
        if(pathRetry>0)pathRetry--;
        boolean visible=body.getSensing().hasLineOfSight(foe);
        double distance=body.position().distanceToSqr(foe.position());
        boolean near=visible&&distance<3*JakEnemyKind.SCALE*3*JakEnemyKind.SCALE;
        if(phase!=null){
            if(phase!=Phase.MELEE&&(!visible||near)&&phase!=Phase.HOLSTER)phase(Phase.HOLSTER);
            else advance(level);
            return true;
        }
        if(near){
            if(cooldown==0)phase(Phase.MELEE);
            return true;
        }
        if(!kind.monk()&&visible&&distance<32*32){phase(female()?Phase.DRAW:Phase.RAISE);return true;}
        if(pathRetry==0){
            pathRetry=10;
            double angle=(body.getId()&7)*Math.PI/4;
            Vec3 at=foe.position().add(Math.cos(angle)*1.1,0,Math.sin(angle)*1.1);
            body.getNavigation().moveTo(at.x,at.y,at.z,1,1.1);
        }
        return true;
    }

    private boolean female(){return rig.model().equals("wlander-female");}
    private String clip(){
        return switch(phase){
            case DRAW -> "gun-takeout";
            case RAISE -> female()?"gun-out":"idle-to-shoot";
            case TURN_START -> "shoot-turn1-start";
            case TURN -> "shoot-turn1";
            case TURN_END -> "shoot-turn1-end";
            case AIM -> "shoot-idle";
            case FIRE -> "shoot0";
            case HOLSTER -> female()?"gun-away0":"shoot-to-idle";
            case MELEE -> female()?"attack-kick-spin0":"attack-rifle-butt";
        };
    }
    private void phase(Phase next){
        if(next==null&&phase==null)return;
        phase=next;age=0;dealt=false;
        if(next!=null){body.getNavigation().stop();body.getMoveControl().setWantedPosition(body.getX(),body.getY(),body.getZ(),0);}
        state.citizenDefense(next==null?"":clip(),JakEntityClock.time(body.level()),
            female()&&next!=null&&next!=Phase.DRAW&&next!=Phase.MELEE);
    }
    private void advance(ServerLevel level){
        String clip=clip();
        float rate=phase==Phase.DRAW?1.6f:1;
        float previous=age*rate/20,now=++age*rate/20;
        var animation=animation(rig,clip);
        float duration=animation==null?rig.duration(clip):animation.duration();
        if(animation!=null)for(var cue:animation.cues()){
            float at=(cue.frame()-animation.artistBase())/(60*animation.speed()*animation.artistStep());
            if(at<previous||at>=now)continue;
            switch(cue.name()){
                case "event-gunout" -> state.citizenWeaponOut(true);
                case "event-gunaway" -> state.citizenWeaponOut(false);
                case "event-shoot" -> {
                    if(foe!=null&&body.getSensing().hasLineOfSight(foe)){
                        var offset=rig.shotMuzzle().scale(JakEnemyKind.SCALE*body.getAgeScale()*body.getScale());
                        var muzzle=body.position().add(offset.yRot((float)-Math.toRadians(body.getYRot())));
                        shots.fire(level,muzzle,foe.getBoundingBox().getCenter(),foe);
                    }
                }
                default -> SettlementShots.cue(level,"jak3",cue.name(),body.position());
            }
        }
        float angle=foe==null?0:angle(foe.position());
        switch(phase){
            case DRAW -> {if(now>=duration)phase(Phase.RAISE);}
            case RAISE,TURN_END -> {if(now>=duration)phase(Phase.AIM);}
            case AIM -> {
                if(foe==null)phase(Phase.HOLSTER);
                else if(Math.abs(angle)>23)phase(Phase.TURN_START);
                else{turn(angle,3);phase(Phase.FIRE);}
            }
            case TURN_START -> {if(now>=duration)phase(Phase.TURN);}
            case TURN -> {turn(angle,18);if(now>=duration)phase(Math.abs(angle)<5?Phase.TURN_END:Phase.TURN);}
            case FIRE -> {if(now>=duration)phase(Phase.AIM);}
            case HOLSTER -> {if(now>=duration)phase(null);}
            case MELEE -> {
                if(foe!=null)turn(angle,18);
                if(!dealt&&now>=(female()?7:3)/15f){
                    dealt=true;
                    if(foe!=null&&body.distanceToSqr(foe)<3.5&&body.getSensing().hasLineOfSight(foe))
                        foe.hurtServer(level,body.damageSources().mobAttack(body),2);
                }
                if(now>=duration){cooldown=Math.max(2,20-age);phase(null);}
            }
        }
    }
    private float angle(Vec3 at){return Mth.wrapDegrees((float)Math.toDegrees(Math.atan2(body.getX()-at.x,at.z-body.getZ()))-body.getYRot());}
    private void turn(float angle,float cap){
        float yaw=body.getYRot()+Math.clamp(angle,-cap,cap);
        body.setYRot(yaw);body.setYBodyRot(yaw);body.setYHeadRot(yaw);
    }
    private static JakEnemyKind.Animation animation(CitizenKind rig,String suffix){
        return ART.computeIfAbsent(rig.model(),model->JakEnemyKind.readAnimations(rig.root(),model,false))
            .get(rig.model()+"-"+suffix);
    }
}
