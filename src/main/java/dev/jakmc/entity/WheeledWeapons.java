package dev.jakmc.entity;

import com.google.gson.JsonArray;
import com.google.gson.JsonObject;
import dev.jakmc.player.JakDamage;
import java.util.ArrayList;
import net.minecraft.core.BlockPos;
import net.minecraft.server.level.ServerLevel;
import net.minecraft.server.level.ServerPlayer;
import net.minecraft.sounds.SoundSource;
import net.minecraft.world.entity.Entity;
import net.minecraft.world.entity.LivingEntity;
import net.minecraft.world.level.ClipContext;
import net.minecraft.world.phys.AABB;
import net.minecraft.world.phys.HitResult;
import net.minecraft.world.phys.Vec3;
import org.joml.Quaternionfc;

final class WheeledWeapons {
    private static final double SCALE=.64;
    private record Contact(Vec3 at,Entity target,boolean blocked){}
    private static final class Grenade {
        final int id;
        final ServerPlayer pilot;
        Vec3 pos,velocity;
        double age;
        Grenade(int id,ServerPlayer pilot,Vec3 pos,Vec3 velocity){
            this.id=id;
            this.pilot=pilot;
            this.pos=pos;
            this.velocity=velocity;
        }
    }
    private final ArrayList<Grenade> grenades=new ArrayList<>();
    private final JakVehicle car;
    private int next;
    WheeledWeapons(JakVehicle car){this.car=car;}

    void fire(ServerLevel level,ServerPlayer pilot,Quaternionfc attitude,Vec3 velocity,boolean right,JsonArray effects) {
        double side=right?1:-1;
        boolean toad=car.model==JakVehicle.Model.TOAD;
        Vec3 muzzle=car.position().add(transform(attitude,new Vec3(side*(toad?.5:.9),toad?1.8:1.3,1.5)).scale(SCALE));
        Vec3 forward=transform(attitude,new Vec3(0,0,1));
        Vec3 up=transform(attitude,new Vec3(0,1,0));
        Vec3 cross=transform(attitude,new Vec3(1,0,0));
        double elev=toad?13.39:.0871;
        double nearest=100*SCALE;
        for(var target:level.getEntitiesOfClass(LivingEntity.class,new AABB(muzzle,muzzle).inflate(nearest),
            e->eligible(e,pilot))){
            Vec3 delta=target.getBoundingBox().getCenter().subtract(muzzle);
            double range=delta.length(),ahead=delta.dot(forward);
            if(range>=nearest||ahead<(toad?15*SCALE:0)||Math.abs(delta.dot(cross))>(toad?15:2)*SCALE)continue;
            if(trace(level,pilot,muzzle,target.getBoundingBox().getCenter()).target()!=target)continue;
            if(toad){
                double time=Math.max(.1,ahead/Math.max(1,50*SCALE+velocity.dot(forward)-target.getDeltaMovement().dot(forward)*20));
                elev=Math.clamp((delta.y/time+target.getDeltaMovement().y*20+20*SCALE*time)/(SCALE*Math.max(.1,up.y)),0,18.19);
            }else elev=Math.clamp(delta.dot(up)/range,0,.1736);
            nearest=range;
        }
        if(toad){
            grenades.add(new Grenade(++next,pilot,muzzle,forward.scale(50*SCALE).add(up.scale(elev*SCALE)).add(velocity)));
            cue(level,"toad-shot-fire",muzzle);
        }else{
            Vec3 direction=forward.scale(Math.sqrt(1-elev*elev)).add(up.scale(elev));
            Contact hit=trace(level,pilot,muzzle,muzzle.add(direction.scale(120*SCALE)));
            effects.add(event("beam",muzzle,hit.at()));
            if(hit.blocked())effects.add(event("group-gun-scorp-shot-hit",hit.at(),null));
            if(hit.target()!=null)damage(level,pilot,hit.target(),4,false);
            cue(level,"snake-gun-fire",muzzle);
        }
    }
    void step(ServerLevel level,JsonArray effects,JsonArray models){
        var it=grenades.iterator();
        while(it.hasNext()){
            Grenade shot=it.next();
            boolean ended=false;
            for(int i=0;i<6;i++){
                shot.age+=1./120;
                shot.velocity=shot.velocity.add(0,-40*SCALE/120,0);
                Vec3 end=shot.pos.add(shot.velocity.scale(1./120));
                if(!level.hasChunkAt(BlockPos.containing(end))){ended=true;break;}
                Contact hit=trace(level,shot.pilot,shot.pos,end);
                shot.pos=hit.at();
                if(hit.blocked()||shot.age>=4){
                    blast(level,shot.pilot,shot.pos,shot.velocity,effects);
                    ended=true;
                    break;
                }
            }
            if(ended)it.remove();
            else{
                var model=event("grenade",shot.pos,null);
                model.addProperty("id",shot.id);
                models.add(model);
            }
        }
    }
    private void blast(ServerLevel level,ServerPlayer pilot,Vec3 point,Vec3 velocity,JsonArray effects){
        effects.add(event("group-toad-grenade-shot-explode",point,null));
        cue(level,"toad-shot-hit",point);
        Vec3 from=point.subtract(velocity.normalize().scale(.02));
        double radius=25*SCALE;
        for(var target:level.getEntitiesOfClass(LivingEntity.class,new AABB(from,from).inflate(radius),e->eligible(e,pilot))){
            var center=target.getBoundingBox().getCenter();
            double distance=center.distanceTo(from);
            if(distance<=radius&&trace(level,pilot,from,center).target()==target)
                damage(level,pilot,target,(float)(20*Math.max(.1,1-distance/radius)),true);
        }
    }
    private Contact trace(ServerLevel level,ServerPlayer pilot,Vec3 from,Vec3 end){
        var block=level.clip(new ClipContext(from,end,ClipContext.Block.COLLIDER,ClipContext.Fluid.NONE,car));
        Vec3 stop=block.getType()==HitResult.Type.MISS?end:block.getLocation();
        double best=from.distanceToSqr(stop);
        Entity hit=null;
        for(var target:level.getEntities(car,new AABB(from,stop).inflate(.2),e->eligible(e,pilot))){
            var box=target.getBoundingBox().inflate(.03);
            Vec3 point=box.contains(from)?from:box.clip(from,stop).orElse(null);
            if(point==null||from.distanceToSqr(point)>=best)continue;
            stop=point;
            best=from.distanceToSqr(point);
            hit=target;
        }
        return new Contact(stop,hit,hit!=null||block.getType()!=HitResult.Type.MISS);
    }
    private boolean eligible(Entity target,ServerPlayer pilot){
        return target!=car&&target!=pilot&&!car.hasPassenger(target)&&target.isAlive()&&target.isPickable();
    }
    private void damage(ServerLevel level,ServerPlayer pilot,Entity target,float points,boolean explosive){
        var source=pilot.level()==level?JakDamage.shot(pilot,explosive):car.damageSources().magic();
        if(target instanceof JakEnemy enemy)enemy.hurtFromJak(level,source,points*2.5f,explosive);
        else target.hurtServer(level,source,points*2.5f);
    }
    private JsonObject event(String effect,Vec3 pos,Vec3 end){
        var result=new JsonObject();
        result.addProperty("effect",effect);
        result.add("pos",JakVehicle.jsonVector(pos.subtract(car.position())));
        if(end!=null)result.add("end",JakVehicle.jsonVector(end.subtract(car.position())));
        return result;
    }
    private static Vec3 transform(Quaternionfc attitude,Vec3 value){
        var v=attitude.transform(value.toVector3f());
        return new Vec3(v.x,v.y,v.z);
    }
    private static void cue(ServerLevel level,String name,Vec3 at){
        var cue=JakEnemySounds.find("jak3",name);
        if(cue!=null)level.playSound(null,at.x,at.y,at.z,cue,SoundSource.NEUTRAL,1,1);
    }
}
