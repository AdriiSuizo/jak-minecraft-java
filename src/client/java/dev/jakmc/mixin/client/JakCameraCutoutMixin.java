package dev.jakmc.mixin.client;

import dev.jakmc.client.camera.JakCameraCutout;
import net.minecraft.client.Camera;
import net.minecraft.client.DeltaTracker;
import net.minecraft.world.entity.Entity;
import net.minecraft.world.level.Level;
import net.minecraft.world.phys.Vec3;
import org.joml.Vector3f;
import org.spongepowered.asm.mixin.*;
import org.spongepowered.asm.mixin.injection.*;
import org.spongepowered.asm.mixin.injection.callback.*;

/** Third-person Jak camera: walls are cut away instead of pulling the camera in (Rust cutout34-38). */
@Mixin(Camera.class)
abstract class JakCameraCutoutMixin {
    @Shadow private Level level;
    @Shadow private Entity entity;
    @Shadow private Vec3 position;
    @Shadow @Final private Vector3f forwards;

    @Inject(method="getMaxZoom",at=@At("HEAD"),cancellable=true)
    private void jakmc$cutoutReach(float cameraDist,CallbackInfoReturnable<Float> cir){
        if(level!=null&&JakCameraCutout.wanted(entity))cir.setReturnValue(JakCameraCutout.maxZoom(level,entity,position,new Vec3(forwards),cameraDist));
    }

    @Inject(method="update",at=@At("TAIL"))
    private void jakmc$cutoutFade(DeltaTracker tracker,CallbackInfo ci){
        JakCameraCutout.frame((Camera)(Object)this);
    }
}
