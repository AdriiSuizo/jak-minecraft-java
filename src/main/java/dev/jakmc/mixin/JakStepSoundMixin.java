package dev.jakmc.mixin;
import dev.jakmc.player.JakPlayers;
import net.minecraft.core.BlockPos;
import net.minecraft.world.entity.Entity;
import net.minecraft.world.entity.player.Player;
import net.minecraft.world.level.block.state.BlockState;
import org.spongepowered.asm.mixin.Mixin;
import org.spongepowered.asm.mixin.injection.*;
import org.spongepowered.asm.mixin.injection.callback.CallbackInfo;
/**
 * Active Jak plays his own material footsteps from the original art groups
 * (jak_steps). Minecraft's stepping on top of them doubled every step, which
 * the Rust host also silenced under Jak. Game events (sculk) are kept.
 */
@Mixin(Entity.class)
abstract class JakStepSoundMixin {
    @Inject(method="walkingStepSound",at=@At("HEAD"),cancellable=true)
    private void jakmc$ownSteps(BlockPos pos,BlockState state,CallbackInfo ci){
        if((Object)this instanceof Player p&&JakPlayers.active(p))ci.cancel();
    }
    @Inject(method="playCombinationStepSounds",at=@At("HEAD"),cancellable=true)
    private void jakmc$ownCombinationSteps(BlockState primary,BlockState secondary,CallbackInfo ci){
        if((Object)this instanceof Player p&&JakPlayers.active(p))ci.cancel();
    }
}
