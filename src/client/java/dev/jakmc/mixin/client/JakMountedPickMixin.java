package dev.jakmc.mixin.client;

import com.llamalad7.mixinextras.injector.wrapoperation.Operation;
import com.llamalad7.mixinextras.injector.wrapoperation.WrapOperation;
import dev.jakmc.entity.JakLeaper;
import java.util.function.Predicate;
import net.minecraft.client.player.LocalPlayer;
import net.minecraft.world.entity.Entity;
import net.minecraft.world.phys.AABB;
import net.minecraft.world.phys.EntityHitResult;
import net.minecraft.world.phys.Vec3;
import org.spongepowered.asm.mixin.Mixin;
import org.spongepowered.asm.mixin.injection.At;

@Mixin(LocalPlayer.class)
abstract class JakMountedPickMixin {
    @WrapOperation(method="pick",at=@At(value="INVOKE",target=
        "Lnet/minecraft/world/entity/projectile/ProjectileUtil;getEntityHitResult(Lnet/minecraft/world/entity/Entity;Lnet/minecraft/world/phys/Vec3;Lnet/minecraft/world/phys/Vec3;Lnet/minecraft/world/phys/AABB;Ljava/util/function/Predicate;D)Lnet/minecraft/world/phys/EntityHitResult;"))
    private static EntityHitResult jakmc$pickPastOwnRide(Entity camera,Vec3 from,Vec3 to,AABB box,
        Predicate<Entity> filter,double distance,Operation<EntityHitResult> original){
        if(camera instanceof LocalPlayer player){
            var pose=dev.jakmc.client.JakPoses.get(player.getId());
            if(pose!=null&&pose.raw().get("ride") instanceof com.google.gson.JsonObject){
                return original.call(camera,from,to,box,filter.and(e->
                    !(e instanceof JakLeaper mount)||mount.riderId()!=player.getId()),distance);
            }
        }
        return original.call(camera,from,to,box,filter,distance);
    }
}
