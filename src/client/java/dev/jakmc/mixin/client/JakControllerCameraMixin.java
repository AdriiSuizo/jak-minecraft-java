package dev.jakmc.mixin.client;

import com.llamalad7.mixinextras.injector.ModifyExpressionValue;
import dev.jakmc.client.JakInput;
import dev.jakmc.client.JakPoses;
import net.minecraft.client.*;
import net.minecraft.world.entity.Entity;
import net.minecraft.world.level.ClipContext;
import net.minecraft.world.phys.HitResult;
import net.minecraft.world.phys.Vec3;
import org.spongepowered.asm.mixin.*;
import org.spongepowered.asm.mixin.injection.*;
import org.spongepowered.asm.mixin.injection.callback.CallbackInfo;
import org.spongepowered.asm.mixin.injection.invoke.arg.Args;

@Mixin(Camera.class)
abstract class JakControllerCameraMixin {
    @Shadow private Entity entity;
    @Shadow protected abstract void setPosition(Vec3 position);
    @Shadow protected abstract void setRotation(float yaw,float pitch);
    @Shadow public abstract Vec3 position();
    @Unique private long jakmc$lastFrame;
    @Unique private float jakmc$dt;
    @Unique private float jakmc$distance=Float.NaN;
    @Unique private float jakmc$aim;
    @Unique private float jakmc$mountHeight;

    @Inject(method="update",at=@At("HEAD"))
    private void jakmc$prepareFrame(DeltaTracker deltaTracker,CallbackInfo ci){
        var client=Minecraft.getInstance();
        long now=System.nanoTime();
        jakmc$dt=jakmc$lastFrame==0?0:Math.min(.1f,(now-jakmc$lastFrame)/1e9f);
        jakmc$lastFrame=now;
        JakInput.frame(client);
        JakPoses.beginFrame(client);
        var pose=entity==null?null:JakPoses.get(entity.getId());
        boolean mounted=pose!=null&&pose.raw().get("ride") instanceof com.google.gson.JsonObject;
        jakmc$mountHeight+=((mounted?.8f:0)-jakmc$mountHeight)*(1-(float)Math.exp(-9*jakmc$dt));
        if(!jakmc$active()||client.options.getCameraType().isFirstPerson()||client.options.getCameraType().isMirrored()){
            jakmc$distance=Float.NaN;
            jakmc$aim=0;
        }else{
            float target=JakInput.aiming()?1:0;
            jakmc$aim+=(target-jakmc$aim)*(1-(float)Math.exp(-10*jakmc$dt));
        }
    }

    @Inject(method="update",at=@At("TAIL"))
    private void jakmc$observeCamera(DeltaTracker tracker,CallbackInfo ci){
        dev.jakmc.client.InputScene.sampleCamera((Camera)(Object)this);
    }

    @Unique private boolean jakmc$active(){
        return entity==Minecraft.getInstance().player&&JakPoses.presented()!=null;
    }

    @ModifyArgs(method="alignWithEntity",at=@At(value="INVOKE",target="Lnet/minecraft/client/Camera;setPosition(DDD)V"))
    private void jakmc$follow(Args args){
        if(!jakmc$active())return;
        var feet=JakPoses.presented();
        args.set(0,feet.x);
        double height=feet.y+1.62;
        if(jakmc$mountHeight>.001f){
            Vec3 from=new Vec3(feet.x,height,feet.z);
            var hit=entity.level().clip(new ClipContext(from,from.add(0,jakmc$mountHeight,0),
                ClipContext.Block.VISUAL,ClipContext.Fluid.NONE,entity));
            height+=hit.getType()==HitResult.Type.MISS?jakmc$mountHeight:
                Math.max(0,hit.getLocation().y-height-.12);
        }
        args.set(1,height);
        args.set(2,feet.z);
    }

    @ModifyArg(method="alignWithEntity",at=@At(value="INVOKE",target="Lnet/minecraft/client/Camera;getMaxZoom(F)F"))
    private float jakmc$aimDistance(float distance){
        if(!jakmc$active())return distance;
        if(jakmc$aim>.001f){
            Camera camera=(Camera)(Object)this;
            Vec3 pivot=position();
            Vec3 offset=new Vec3(camera.leftVector()).scale(-1.05*4096/6400)
                .add(new Vec3(camera.upVector()).scale(.12*4096/6400)).scale(jakmc$aim);
            var hit=entity.level().clip(new ClipContext(pivot,pivot.add(offset),ClipContext.Block.VISUAL,ClipContext.Fluid.NONE,entity));
            if(hit.getType()!=HitResult.Type.MISS){
                offset=offset.normalize().scale(Math.max(0,pivot.distanceTo(hit.getLocation())-.12));
            }
            setPosition(pivot.add(offset));
        }
        // Original aimed distance is 2.9 m, at 4096 Jak units per metre.
        return net.minecraft.util.Mth.lerp(jakmc$aim,distance,2.9f*4096/6400);
    }

    @Inject(method="alignWithEntity",at=@At("RETURN"))
    private void jakmc$convergeAim(float partial,CallbackInfo ci){
        if(!jakmc$active()||jakmc$aim<=.001f)return;
        var player=Minecraft.getInstance().player;
        var hit=player.raycastHitResult(partial,player);
        // Retain vanilla item picking and align the view to that same interaction ray.
        Vec3 target=hit.getType()==HitResult.Type.MISS
            ?player.getEyePosition(partial).add(player.getViewVector(partial).scale(24))
            :hit.getLocation();
        Vec3 look=target.subtract(position());
        float yaw=(float)Math.toDegrees(Math.atan2(-look.x,look.z));
        float pitch=(float)-Math.toDegrees(Math.atan2(look.y,look.horizontalDistance()));
        setRotation(yaw,pitch);
    }

    @ModifyExpressionValue(method="alignWithEntity",at=@At(value="INVOKE",target="Lnet/minecraft/client/Camera;getMaxZoom(F)F"))
    private float jakmc$recoverFromWall(float clearDistance){
        if(!jakmc$active())return clearDistance;
        if(!Float.isFinite(jakmc$distance)||clearDistance<jakmc$distance)jakmc$distance=clearDistance;
        else jakmc$distance+=(clearDistance-jakmc$distance)*(1-(float)Math.exp(-10*jakmc$dt));
        return jakmc$distance;
    }
}
