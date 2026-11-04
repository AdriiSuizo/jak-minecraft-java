package dev.jakmc.mixin;
import dev.jakmc.player.JakPlayers;
import net.minecraft.server.network.ServerGamePacketListenerImpl;
import net.minecraft.server.level.ServerPlayer;
import net.minecraft.network.protocol.game.ServerboundMovePlayerPacket;
import net.minecraft.world.entity.PositionMoveRotation;
import net.minecraft.world.entity.Relative;
import java.util.Set;
import org.spongepowered.asm.mixin.*;
import org.spongepowered.asm.mixin.injection.*;
import org.spongepowered.asm.mixin.injection.callback.CallbackInfo;
/** A Jak client sends controls; vanilla position reports must not overwrite the server solver. */
@Mixin(ServerGamePacketListenerImpl.class)
abstract class ServerMovementMixin {
 @Shadow public ServerPlayer player;
 @Inject(method="handleMovePlayer",at=@At("HEAD"),cancellable=true)
 private void jakmc$authoritativeMovement(ServerboundMovePlayerPacket packet,CallbackInfo ci){if(JakPlayers.active(player)||player.getVehicle() instanceof dev.jakmc.entity.JakVehicle)ci.cancel();}

 @Inject(method="teleport(Lnet/minecraft/world/entity/PositionMoveRotation;Ljava/util/Set;)V",at=@At("TAIL"))
 private void jakmc$hostTeleport(PositionMoveRotation destination,Set<Relative> relatives,CallbackInfo ci){
     JakPlayers.hostTeleport(player);
 }
}
