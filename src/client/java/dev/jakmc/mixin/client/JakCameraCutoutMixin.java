package dev.jakmc.mixin.client;

import dev.jakmc.client.camera.JakCameraCutout;
import net.minecraft.client.Camera;
import net.minecraft.client.DeltaTracker;
import net.minecraft.world.entity.Entity;
import net.minecraft.world.level.Level;
import net.minecraft.world.phys.Vec3;
import org.spongepowered.asm.mixin.*;
import org.spongepowered.asm.mixin.injection.*;
import org.spongepowered.asm.mixin.injection.callback.*;

/** Jak's third-person camera (Adrii's Baldur's Gate 3 style camera, JakCameraCutout) replaces vanilla's wall clip. */
@Mixin(Camera.class)
abstract class JakCameraCutoutMixin {
    @Shadow private Level level;
    @Shadow private Entity entity;
    @Shadow private Vec3 position;
    @Shadow private float xRot;
    @Shadow private float yRot;
    @Shadow protected abstract void setPosition(Vec3 position);
    @Shadow protected abstract void setRotation(float yaw,float pitch);

    @Inject(method="getMaxZoom",at=@At("HEAD"),cancellable=true)
    private void jakmc$cameraReach(float cameraDist,CallbackInfoReturnable<Float> cir){
        if(level==null||!JakCameraCutout.wanted(entity))return;
        float distance=JakCameraCutout.maxZoom(level,entity,position,yRot,xRot,cameraDist);
        // The pivot drops out of a ceiling and the pitch slides under one before alignWithEntity moves back.
        Vec3 pivot=JakCameraCutout.placedPivot();
        if(pivot!=null&&pivot!=position)setPosition(pivot);
        if(JakCameraCutout.placedPitch()!=xRot)setRotation(yRot,JakCameraCutout.placedPitch());
        cir.setReturnValue(distance);
    }

    @Inject(method="update",at=@At("TAIL"))
    private void jakmc$cameraMasks(DeltaTracker tracker,CallbackInfo ci){
        JakCameraCutout.frame((Camera)(Object)this);
    }
}
