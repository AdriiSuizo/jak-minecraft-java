package dev.jakmc.mixin;
import dev.jakmc.entity.CitizenState;
import net.minecraft.world.entity.LivingEntity;
import net.minecraft.world.entity.npc.villager.Villager;
import net.minecraft.server.level.ServerLevel;
import net.minecraft.world.damagesource.DamageSource;
import org.spongepowered.asm.mixin.Mixin;
import org.spongepowered.asm.mixin.injection.*;
import org.spongepowered.asm.mixin.injection.callback.CallbackInfoReturnable;
@Mixin(LivingEntity.class)
abstract class CitizenDamageMixin {
 @Inject(method="hurtServer",at=@At("RETURN"))private void jakmc$hurt(ServerLevel level,DamageSource source,float amount,CallbackInfoReturnable<Boolean> ci){if(ci.getReturnValue()&&amount>0&&(Object)this instanceof Villager v){
   ((CitizenState)v).citizenHit();
   if(source.getEntity() instanceof net.minecraft.world.entity.player.Player player)
    dev.jakmc.entity.KrimsonGuard.witness(level,v,player);
  }}
}
