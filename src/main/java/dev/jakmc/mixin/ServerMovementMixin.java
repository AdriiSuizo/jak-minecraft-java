package dev.jakmc.mixin;
import dev.jakmc.player.JakPlayers;
import net.minecraft.server.network.ServerGamePacketListenerImpl;
import net.minecraft.server.level.ServerPlayer;
import net.minecraft.network.protocol.game.ServerboundMovePlayerPacket;
import org.spongepowered.asm.mixin.*;
import org.spongepowered.asm.mixin.injection.*;
import org.spongepowered.asm.mixin.injection.callback.CallbackInfo;
/** A Jak client sends controls; vanilla position reports must not overwrite the server solver. */
@Mixin(ServerGamePacketListenerImpl.class)
abstract class ServerMovementMixin {
 @Shadow public ServerPlayer player;
 @Inject(method="handleMovePlayer",at=@At("HEAD"),cancellable=true)
 private void jakmc$authoritativeMovement(ServerboundMovePlayerPacket packet,CallbackInfo ci){if(JakPlayers.active(player))ci.cancel();}
}
