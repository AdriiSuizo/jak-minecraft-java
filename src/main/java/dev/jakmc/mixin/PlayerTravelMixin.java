package dev.jakmc.mixin;
import dev.jakmc.player.JakPlayers;
import net.minecraft.world.entity.player.Player;
import net.minecraft.world.phys.Vec3;
import org.spongepowered.asm.mixin.Mixin;
import org.spongepowered.asm.mixin.injection.*;
import org.spongepowered.asm.mixin.injection.callback.CallbackInfo;
@Mixin(Player.class)
abstract class PlayerTravelMixin {
    @Inject(method="travel",at=@At("HEAD"),cancellable=true)
    private void jakmc$ownedMovement(Vec3 input,CallbackInfo ci){if(JakPlayers.active((Player)(Object)this))ci.cancel();}
}
