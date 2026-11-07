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
            return;
        }
        // Rust minecraft_world.rs lava_hurts (the soldier's only): Jak in lava or a fire block is set alight, not hurt by them.
        if((Object)this instanceof ServerPlayer player&&JakPlayers.active(player)
                &&(source.is(net.minecraft.world.damagesource.DamageTypes.LAVA)||source.is(net.minecraft.world.damagesource.DamageTypes.IN_FIRE)))
            result.setReturnValue(false);
        // Rust interaction.rs in_wall: Jak smothers with the point of his eyes in a full block, not with the box round it that
        // Minecraft tests. Adrii 07/11: hanging on a ledge, and at the corner of a block, that box hurt him.
        if((Object)this instanceof ServerPlayer player&&JakPlayers.active(player)&&source.is(net.minecraft.world.damagesource.DamageTypes.IN_WALL)
                &&(JakPlayers.hanging(player)||!dev.jakmc.player.JakRescue.eyesBuried(player)))
            result.setReturnValue(false);
    }
    @Inject(method="hurtServer",at=@At("RETURN"))
    private void jakmc$reaction(ServerLevel level,DamageSource source,float amount,CallbackInfoReturnable<Boolean> result){
        if(result.getReturnValue()&&amount>0&&(Object)this instanceof ServerPlayer player){
            JakPlayers.hurt(player,source);
            // pilot-states.gc target-pilot-hit: what reaches him on his zoomer takes him dead.
            if(player.getVehicle() instanceof dev.jakmc.entity.JakVehicle car)car.riderHit(player,source);
        }
    }
    @Inject(method="playHurtSound",at=@At("HEAD"),cancellable=true)
    private void jakmc$sourceHurtSound(DamageSource source,org.spongepowered.asm.mixin.injection.callback.CallbackInfo result){
        // The source hit already emits OOF/GET_BURNED. Keep vanilla audio for other players.
        if((Object)this instanceof net.minecraft.world.entity.player.Player player&&JakPlayers.active(player))result.cancel();
    }

}
