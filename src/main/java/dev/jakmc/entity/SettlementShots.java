package dev.jakmc.entity;

import com.google.gson.JsonArray;
import com.google.gson.JsonObject;
import java.util.ArrayList;
import net.minecraft.core.BlockPos;
import net.minecraft.server.level.ServerLevel;
import net.minecraft.sounds.SoundSource;
import net.minecraft.world.entity.LivingEntity;
import net.minecraft.world.entity.player.Player;
import net.minecraft.world.level.ClipContext;
import net.minecraft.world.phys.AABB;
import net.minecraft.world.phys.HitResult;
import net.minecraft.world.phys.Vec3;

/** Jak's shared guard-projectile flight, with Minecraft hit and damage attribution. */
final class SettlementShots {
    static final double SPEED=200*.64;
    private record Shot(Vec3 head,Vec3 direction,int age,LivingEntity target){
        boolean againstPlayer(){return target instanceof Player;}
    }
    private final ArrayList<Shot> shots=new ArrayList<>();
    private final LivingEntity body;
    private final float damage;
    private final String game,fire;
    SettlementShots(LivingEntity body,float damage,String game,String fire){
        this.body=body;this.damage=damage;this.game=game;this.fire=fire;
    }
    void fire(ServerLevel level,Vec3 muzzle,Vec3 aim,LivingEntity target){
        var direction=aim.subtract(muzzle).normalize();
        if(direction.lengthSqr()==0)return;
        shots.add(new Shot(muzzle,direction,0,target));
        cue(level,game,fire,muzzle);
    }
    String tick(ServerLevel level){
        if(shots.isEmpty())return "";
        var frame=new JsonArray();
        var it=shots.listIterator();
        while(it.hasNext()){
            Shot shot=it.next();
            Vec3 at=shot.head(),dir=shot.direction(),dest=at.add(dir.scale(SPEED/20));
            if(!level.hasChunkAt(BlockPos.containing(dest))){it.remove();continue;}
            var wall=level.clip(new ClipContext(at,dest,ClipContext.Block.COLLIDER,ClipContext.Fluid.NONE,body));
            boolean blocked=wall.getType()!=HitResult.Type.MISS;
            if(blocked)dest=wall.getLocation();
            double closest=at.distanceToSqr(dest);
            LivingEntity hit=null;
            for(var target:level.getEntitiesOfClass(LivingEntity.class,new AABB(at,dest).inflate(.2*.64),
                e->e!=body&&e.isAlive()&&(shot.againstPlayer()?e instanceof Player p&&!p.isCreative()&&!p.isSpectator():e==shot.target()))){
                var bounds=target.getBoundingBox().inflate(.2*.64);
                Vec3 intersection=bounds.contains(at)?at:bounds.clip(at,dest).orElse(null);
                if(intersection==null||at.distanceToSqr(intersection)>=closest)continue;
                closest=at.distanceToSqr(intersection);dest=intersection;hit=target;
            }
            Vec3 tail=dest.subtract(dir.scale(Math.min(9*.64,at.distanceTo(dest)+SPEED/20*shot.age())));
            var row=new JsonObject();
            row.add("head",JakVehicle.jsonVector(dest));row.add("tail",JakVehicle.jsonVector(tail));
            row.add("direction",JakVehicle.jsonVector(dir));row.addProperty("hit",blocked||hit!=null);
            row.addProperty("actor",hit!=null);frame.add(row);
            if(hit!=null)hit.hurtServer(level,body.damageSources().mobAttack(body),damage);
            if(blocked||hit!=null)cue(level,game,"guard-shot-hit",dest);
            if(blocked||hit!=null||shot.age()>=9)it.remove();
            else it.set(new Shot(dest,dir,shot.age()+1,shot.target()));
        }
        return frame.isEmpty()?"":frame.toString();
    }

    static void cue(ServerLevel level,String game,String name,Vec3 at){
        var event=JakEnemySounds.find(game,name);
        if(event!=null)level.playSound(null,at.x,at.y,at.z,event,SoundSource.HOSTILE,1,1);
    }
}
