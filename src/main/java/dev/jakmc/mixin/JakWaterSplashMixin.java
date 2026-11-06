package dev.jakmc.mixin;
import dev.jakmc.player.JakPlayers;
import net.minecraft.world.entity.Entity;
import net.minecraft.world.entity.player.Player;
import org.spongepowered.asm.mixin.Mixin;
import org.spongepowered.asm.mixin.injection.*;
import org.spongepowered.asm.mixin.injection.callback.CallbackInfo;
/**
 * Active Jak splashes with the original's water effects (water.gc `enter-water`, `spawn-ripples`: the native
 * Splash event and its sound). Minecraft's own splash on entering water, its droplets and bubbles thrown up from
 * the top of the block his feet are in (a block over a one-block pool's surface), came out of the water
 * as well and doubled his sound (Adrii 06/11).
 */
@Mixin(Entity.class)
abstract class JakWaterSplashMixin {
    @Inject(method="doWaterSplashEffect",at=@At("HEAD"),cancellable=true)
    private void jakmc$ownSplash(CallbackInfo ci){
        if((Object)this instanceof Player p&&JakPlayers.active(p))ci.cancel();
    }
}
