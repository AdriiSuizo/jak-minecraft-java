package dev.jakmc.mixin.client;
import net.minecraft.client.Minecraft;
import net.minecraft.client.multiplayer.MultiPlayerGameMode;
import net.minecraft.core.BlockPos;
import net.minecraft.core.Direction;
import net.minecraft.world.entity.ai.attributes.Attributes;
import net.minecraft.world.phys.Vec3;
import org.spongepowered.asm.mixin.Mixin;
import org.spongepowered.asm.mixin.injection.*;
import org.spongepowered.asm.mixin.injection.callback.CallbackInfoReturnable;
/** Adrii 06/11: blocks are placed three times as far (JakPlayers.REACH), but broken at Minecraft's own reach. */
@Mixin(MultiPlayerGameMode.class)
abstract class JakReachMixin {
 @Inject(method={"startDestroyBlock","continueDestroyBlock"},at=@At("HEAD"),cancellable=true)
 private void jakmc$reach(BlockPos pos,Direction face,CallbackInfoReturnable<Boolean> ci){
  var player=Minecraft.getInstance().player;
  if(player==null)return;
  var range=player.getAttribute(Attributes.BLOCK_INTERACTION_RANGE);
  if(range==null||range.getModifier(dev.jakmc.player.JakPlayers.REACH)==null)return;
  double reach=range.getBaseValue()+1;
  if(player.getEyePosition().distanceToSqr(Vec3.atCenterOf(pos))>reach*reach)ci.setReturnValue(false);
 }
}
