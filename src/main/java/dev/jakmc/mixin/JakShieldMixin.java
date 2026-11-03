package dev.jakmc.mixin;

import dev.jakmc.player.JakPlayers;
import net.minecraft.server.level.ServerLevel;
import net.minecraft.server.level.ServerPlayer;
import net.minecraft.tags.DamageTypeTags;
import net.minecraft.world.damagesource.DamageSource;
import net.minecraft.world.entity.LivingEntity;
import org.spongepowered.asm.mixin.Mixin;
import org.spongepowered.asm.mixin.injection.*;
import org.spongepowered.asm.mixin.injection.callback.CallbackInfoReturnable;

@Mixin(LivingEntity.class)
abstract class JakShieldMixin {
    @Inject(method="hurtServer",at=@At("HEAD"),cancellable=true)
    private void jakmc$shield(ServerLevel level,DamageSource source,float amount,CallbackInfoReturnable<Boolean> result){
        if(amount>0&&!source.is(DamageTypeTags.BYPASSES_INVULNERABILITY)
                &&(Object)this instanceof ServerPlayer player&&JakPlayers.blockWithLightShield(player)){
            result.setReturnValue(false);
        }
    }
}
