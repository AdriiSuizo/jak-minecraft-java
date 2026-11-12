package dev.jakmc.mixin.client;

import com.llamalad7.mixinextras.injector.wrapoperation.Operation;
import com.llamalad7.mixinextras.injector.wrapoperation.WrapOperation;
import dev.jakmc.client.camera.JakCameraProbe;
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
    @Unique private final JakCameraProbe.Rig jakmc$reach=new JakCameraProbe.Rig();
    @Unique private Vec3 jakmc$lastPivot,jakmc$lastFeet;
    @Unique private double jakmc$lift;
    @Unique private net.minecraft.world.level.Level jakmc$lastLevel;
    @Unique private float jakmc$aim;
    @Unique private float jakmc$mountHeight;

    @Unique private float jakmc$flightWeight,jakmc$pull,jakmc$speed;
    @Unique private boolean jakmc$wheeled;
    @Unique private float jakmc$flightYaw,jakmc$flightPitch,jakmc$manualYaw,jakmc$manualPitch;
    @Unique private float jakmc$lastPlayerYaw,jakmc$lastPlayerPitch;
    @Unique private Vec3 jakmc$flightPivot,jakmc$lastFlightTarget,jakmc$trail=Vec3.ZERO;
    @Inject(method="update",at=@At("HEAD"))
    private void jakmc$prepareFrame(DeltaTracker deltaTracker,CallbackInfo ci){
        var client=Minecraft.getInstance();
        long now=System.nanoTime();
        jakmc$dt=jakmc$lastFrame==0?0:Math.min(.1f,(now-jakmc$lastFrame)/1e9f);
        jakmc$lastFrame=now;
        JakInput.frame(client);
        JakPoses.beginFrame(client);
        var pose=entity==null?null:JakPoses.rendered(entity.getId());
        jakmc$prepareFlight(client,pose);
        boolean mounted=pose!=null&&pose.raw().get("ride") instanceof com.google.gson.JsonObject;
        jakmc$mountHeight+=((mounted?.8f:0)-jakmc$mountHeight)*(1-(float)Math.exp(-9*jakmc$dt));
        if(jakmc$lastLevel!=client.level||!jakmc$active()||client.options.getCameraType()!=CameraType.THIRD_PERSON_BACK){
            jakmc$lastLevel=client.level;
            jakmc$reach.reset();
            jakmc$lastPivot=jakmc$lastFeet=null;
            jakmc$lift=0;
            jakmc$aim=0;
        }else{
            float target=JakInput.aiming()?1:0;
            jakmc$aim+=(target-jakmc$aim)*(1-(float)Math.exp(-10*jakmc$dt));
        }
    }

    @Unique private void jakmc$prepareFlight(Minecraft client,JakPoses.Pose pose){
        var player=client.player;
        var flight=pose!=null&&pose.raw().get("hellcat") instanceof com.google.gson.JsonObject channel?channel:null;
        var car=jakmc$car();
        jakmc$wheeled=car!=null&&car.frame()!=null;
        var channel=jakmc$wheeled?car.presentationAfter():flight;
        boolean chasing=jakmc$active()&&channel!=null&&channel.get("boarding").getAsFloat()<0
            &&client.options.getCameraType()==CameraType.THIRD_PERSON_BACK;
        float ease=1-(float)Math.exp(-8*jakmc$dt);
        jakmc$flightWeight+=((chasing?1:0)-jakmc$flightWeight)*ease;
        if(!chasing||player==null){
            jakmc$lastFlightTarget=null;
            jakmc$trail=Vec3.ZERO;
            if(jakmc$flightWeight<.001f||!jakmc$active()||client.options.getCameraType()!=CameraType.THIRD_PERSON_BACK)
                jakmc$flightPivot=null;
            else if(JakPoses.presented()!=null)
                jakmc$flightPivot=JakPoses.presented().add(0,1.62,0);
            jakmc$manualYaw=jakmc$manualPitch=0;
            return;
        }
        float yaw=player.getYRot(),pitch=player.getXRot();
        var hull=jakmc$wheeled?car.presentationPosition():
            JakPoses.presented().add(JakPoses.flightOffset());
        var rotation=channel.getAsJsonArray("rotation");
        var q=new org.joml.Quaternionf(rotation.get(0).getAsFloat(),rotation.get(1).getAsFloat(),
            rotation.get(2).getAsFloat(),rotation.get(3).getAsFloat());
        if(jakmc$wheeled)q.set(car.presentationRotation());
        if(!jakmc$wheeled&&JakPoses.flightRotation()!=null)q.set(JakPoses.flightRotation());
        var nose=q.transform(new org.joml.Vector3f(0,0,1));
        float nosePitch=(float)Math.toDegrees(Math.asin(Math.clamp(nose.y,-1,1)));
        float targetYaw=(float)Math.toDegrees(Math.atan2(-nose.x,nose.z));
        float targetPitch=Math.clamp(11.46f-nosePitch,-62,70);
        Vec3 from=hull.add(0,.32,0),target=hull.add(0,jakmc$wheeled?1.152:2.048,0);
        var overhead=entity.level().clip(new ClipContext(from,target,ClipContext.Block.VISUAL,ClipContext.Fluid.NONE,entity));
        if(overhead.getType()!=HitResult.Type.MISS){
            target=from.add(0,Math.max(0,overhead.getLocation().y-from.y-.12),0);
        }
        if(jakmc$lastFlightTarget==null){
            jakmc$flightYaw=yaw;
            jakmc$flightPitch=pitch;
            jakmc$trail=Vec3.ZERO;
        }else{
            double limit=jakmc$wheeled?.8:1.92;
            Vec3 trail=jakmc$trail.add(jakmc$lastFlightTarget.subtract(target)).scale(Math.exp(-(jakmc$wheeled?24:7)*jakmc$dt));
            jakmc$trail=trail.lengthSqr()>limit*limit?trail.normalize().scale(limit):trail;
            float yawDelta=net.minecraft.util.Mth.wrapDegrees(yaw-jakmc$lastPlayerYaw);
            float pitchDelta=pitch-jakmc$lastPlayerPitch;
            if(Math.abs(yawDelta)+Math.abs(pitchDelta)>.001f){
                jakmc$manualYaw=net.minecraft.util.Mth.wrapDegrees(jakmc$manualYaw+yawDelta);
                jakmc$manualPitch=Math.clamp(jakmc$manualPitch+pitchDelta,-65,65);
            }else{
                float decay=(float)Math.exp(-1.8*jakmc$dt);
                jakmc$manualYaw*=decay;
                jakmc$manualPitch*=decay;
            }
        }
        jakmc$lastPlayerYaw=yaw;
        jakmc$lastPlayerPitch=pitch;
        jakmc$lastFlightTarget=target;
        jakmc$flightPivot=target.add(jakmc$trail);
        float swing=1-(float)Math.exp(-(jakmc$wheeled?16:5)*jakmc$dt);
        jakmc$flightYaw=net.minecraft.util.Mth.rotLerp(swing,jakmc$flightYaw,targetYaw+jakmc$manualYaw);
        jakmc$flightPitch+=(Math.clamp(targetPitch+jakmc$manualPitch,-70,75)-jakmc$flightPitch)*swing;
        jakmc$pull+=((channel.get("boost").getAsBoolean()?1:0)-jakmc$pull)*(1-(float)Math.exp(-2*jakmc$dt));
        float speed=jakmc$wheeled?(float)dev.jakmc.player.JakPlayers.vector(channel.getAsJsonArray("velocity")).horizontalDistance():
            channel.get("speed").getAsFloat();
        jakmc$speed=Math.clamp(speed/19.2f,0,1);
    }

    @Inject(method="calculateFov",at=@At("RETURN"),cancellable=true)
    private void jakmc$flightFov(float partial,org.spongepowered.asm.mixin.injection.callback.CallbackInfoReturnable<Float> cir){
        if(jakmc$flightPivot!=null){
            cir.setReturnValue(net.minecraft.util.Mth.lerp(jakmc$flightWeight,cir.getReturnValue(),80+8*jakmc$speed));
        }
    }

    @Inject(method="update",at=@At("TAIL"))
    private void jakmc$observeCamera(DeltaTracker tracker,CallbackInfo ci){
        // Light Jak's clap: *camera-smush-control* lifts the camera's trans.
        double smush=dev.jakmc.client.sky.JakFreezeScreen.smush();
        if(smush!=0)setPosition(position().add(0,smush,0));
        dev.jakmc.client.InputScene.sampleCamera((Camera)(Object)this);
        dev.jakmc.client.CarScene.sampleCamera((Camera)(Object)this);
    }

    @Unique private boolean jakmc$active(){
        return entity==Minecraft.getInstance().player&&(JakPoses.presented()!=null||jakmc$car()!=null);
    }

    @Unique private dev.jakmc.entity.JakVehicle jakmc$car(){
        return entity==Minecraft.getInstance().player&&entity!=null
            &&entity.getVehicle() instanceof dev.jakmc.entity.JakVehicle car?car:null;
    }

    @Unique private boolean jakmc$overhead(){
        return jakmc$active()&&Minecraft.getInstance().options.getCameraType()==CameraType.THIRD_PERSON_BACK;
    }

    @ModifyArgs(method="alignWithEntity",at=@At(value="INVOKE",target="Lnet/minecraft/client/Camera;setPosition(DDD)V"))
    private void jakmc$follow(Args args){
        if(!jakmc$active())return;
        var feet=JakPoses.presented();
        if(jakmc$wheeled)feet=jakmc$car().presentationRider();
        if(feet==null)return;
        Vec3 base=feet.add(0,1.62,0);
        Vec3 lead=jakmc$lead(jakmc$lastFeet,feet);
        jakmc$lastFeet=feet;
        double wanted=jakmc$overhead()
            ?jakmc$mountHeight+1.12*(1-jakmc$aim)*(1-jakmc$flightWeight)*(1-jakmc$mountHeight/.8)
            :0;
        double hard=jakmc$ceilingLift(base,wanted);
        double soft=hard;
        if(lead.lengthSqr()>0.0001)soft=Math.min(soft,jakmc$ceilingLift(base.add(lead.x,0,lead.z),wanted));
        if(!jakmc$overhead()||jakmc$lastPivot==null)jakmc$lift=hard;
        else jakmc$lift=Math.min(hard,jakmc$lift+(soft-jakmc$lift)*(1-Math.exp(-12*jakmc$dt)));
        Vec3 target=base.add(0,jakmc$lift,0);
        if(jakmc$flightPivot!=null)target=target.lerp(jakmc$flightPivot,jakmc$flightWeight);
        args.set(0,target.x);args.set(1,target.y);args.set(2,target.z);
    }

    @ModifyArg(method="alignWithEntity",at=@At(value="INVOKE",target="Lnet/minecraft/client/Camera;getMaxZoom(F)F"))
    private float jakmc$aimDistance(float distance){
        if(!jakmc$active())return distance;
        if(jakmc$flightPivot!=null){
            var player=Minecraft.getInstance().player;
            float partial=Minecraft.getInstance().getDeltaTracker().getGameTimeDeltaPartialTick(false);
            setRotation(net.minecraft.util.Mth.rotLerp(jakmc$flightWeight,player.getViewYRot(partial),jakmc$flightYaw),
                net.minecraft.util.Mth.lerp(jakmc$flightWeight,player.getViewXRot(partial),jakmc$flightPitch));
            float chase=(jakmc$wheeled?9f:15.4f)*.64f*(1-.25f*jakmc$speed)*(1+.3f*jakmc$pull);
            return net.minecraft.util.Mth.lerp(jakmc$flightWeight,distance,chase)*JakInput.cameraZoomFactor();
        }
        if(jakmc$aim>.001f){
            Camera camera=(Camera)(Object)this;
            Vec3 pivot=position();
            Vec3 offset=new Vec3(camera.leftVector()).scale(-1.05*4096/6400)
                .add(new Vec3(camera.upVector()).scale(.12*4096/6400)).scale(jakmc$aim);
            var hit=entity.level().clip(new ClipContext(pivot,pivot.add(offset),ClipContext.Block.VISUAL,ClipContext.Fluid.NONE,entity));
            if(hit.getType()!=HitResult.Type.MISS){// the shoulder pivot stays out of walls (JakCameraCutout collides from it)
                offset=offset.normalize().scale(Math.max(0,pivot.distanceTo(hit.getLocation())-.12));
            }
            setPosition(pivot.add(offset));
        }
        // Original aimed distance is 2.9 m, at 4096 Jak units per metre.
        return net.minecraft.util.Mth.lerp(jakmc$aim,distance*JakInput.cameraZoomFactor(),2.9f*4096/6400);
    }

    @Inject(method="alignWithEntity",at=@At("RETURN"))
    private void jakmc$convergeAim(float partial,CallbackInfo ci){
        if(!jakmc$active()||jakmc$flightPivot!=null||(!jakmc$overhead()&&jakmc$aim<=.001f))return;
        var player=Minecraft.getInstance().player;
        var feet=JakPoses.presented();
        if(feet==null)return;
        // Keep the camera framing independent of the block or entity under the reticle.
        double focus=net.minecraft.util.Mth.lerp(jakmc$aim,9,24);
        Vec3 target=feet.add(0,player.getEyeHeight(),0).add(player.getViewVector(partial).scale(focus));
        Vec3 look=target.subtract(position());
        float yaw=(float)Math.toDegrees(Math.atan2(-look.x,look.z));
        float pitch=(float)-Math.toDegrees(Math.atan2(look.y,look.horizontalDistance()));
        setRotation(yaw,pitch);
    }

    @Unique private Vec3 jakmc$lead(Vec3 previous,Vec3 current){
        if(previous==null||jakmc$dt<=0)return Vec3.ZERO;
        Vec3 move=current.subtract(previous);
        if(move.lengthSqr()>4)return Vec3.ZERO;
        Vec3 lead=move.scale(.14/Math.max(jakmc$dt,.004f));
        return lead.lengthSqr()>.55*.55?lead.normalize().scale(.55):lead;
    }

    @Unique private double jakmc$ceilingLift(Vec3 from,double wanted){
        if(wanted<=0)return 0;
        var hit=entity.level().clip(new ClipContext(from,from.add(0,wanted+.12,0),
            ClipContext.Block.VISUAL,ClipContext.Fluid.NONE,entity));
        return hit.getType()==HitResult.Type.MISS?wanted:Math.clamp(hit.getLocation().y-from.y-.12,0,wanted);
    }

    @WrapOperation(method="alignWithEntity",at=@At(value="INVOKE",target="Lnet/minecraft/client/Camera;getMaxZoom(F)F"))
    private float jakmc$recoverFromWall(Camera camera,float desired,Operation<Float> original){
        if(!jakmc$overhead())return original.call(camera,desired);
        Vec3 pivot=position();
        if(dev.jakmc.client.camera.JakCameraCutout.wanted(entity)){
            jakmc$lastPivot=pivot;
            return original.call(camera,desired);
        }
        if(jakmc$lastPivot==null||jakmc$lastPivot.distanceToSqr(pivot)>4)jakmc$reach.reset();
        Vec3 lead=jakmc$lead(jakmc$lastPivot,pivot);
        jakmc$lastPivot=pivot;
        float hard=original.call(camera,desired);
        float soft=hard;
        if(lead.lengthSqr()>.0001){
            try {
                setPosition(pivot.add(lead));
                soft=Math.min(hard,original.call(camera,desired));
            } finally {
                setPosition(pivot);
            }
        }
        return (float)jakmc$reach.step(hard,soft,desired,jakmc$dt);
    }
}
