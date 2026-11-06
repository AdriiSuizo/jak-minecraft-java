package dev.jakmc.mixin;
import net.minecraft.core.BlockPos;
import net.minecraft.core.Direction;
import net.minecraft.server.level.ServerLevel;
import net.minecraft.world.entity.Entity;
import net.minecraft.world.level.Level;
import net.minecraft.world.level.block.BellBlock;
import org.spongepowered.asm.mixin.Mixin;
import org.spongepowered.asm.mixin.injection.*;
import org.spongepowered.asm.mixin.injection.callback.CallbackInfoReturnable;
/** Rust: minecraft_world.rs, a rung bell calls jak2_villagers.rs ring_bell. */
@Mixin(BellBlock.class)
abstract class CitizenBellMixin {
 @Inject(method="attemptToRing(Lnet/minecraft/world/entity/Entity;Lnet/minecraft/world/level/Level;Lnet/minecraft/core/BlockPos;Lnet/minecraft/core/Direction;)Z",at=@At("RETURN"))
 private void jakmc$rung(Entity ringer,Level level,BlockPos pos,Direction direction,CallbackInfoReturnable<Boolean> cir){
  if(cir.getReturnValue()&&level instanceof ServerLevel server)dev.jakmc.citizens.CitizenTown.ringBell(server,pos);
 }
}
