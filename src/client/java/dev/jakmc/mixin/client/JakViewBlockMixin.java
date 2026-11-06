package dev.jakmc.mixin.client;

import net.minecraft.client.Minecraft;
import net.minecraft.client.player.LocalPlayer;
import net.minecraft.client.renderer.culling.Frustum;
import net.minecraft.client.renderer.extract.LevelExtractor;
import net.minecraft.world.level.block.state.BlockState;
import org.spongepowered.asm.mixin.Mixin;
import org.spongepowered.asm.mixin.injection.At;
import org.spongepowered.asm.mixin.injection.Inject;
import org.spongepowered.asm.mixin.injection.callback.CallbackInfoReturnable;

/**
 * The dark "head in a block" overlay only where the camera is the eyes (first person). Vanilla 26.3 draws it in
 * third person too, from the player's eyes: Jak hanging on a ledge or pressed into a tight corner put them in the
 * wall and the whole screen went dark, though the camera behind him saw clear (06/11 batch run).
 */
@Mixin(LevelExtractor.class)
abstract class JakViewBlockMixin {
    @Inject(method="getViewBlockingState",at=@At("HEAD"),cancellable=true)
    private static void jakmc$onlyAtTheEyes(LocalPlayer player,Frustum frustum,CallbackInfoReturnable<BlockState> info){
        if(!Minecraft.getInstance().options.getCameraType().isFirstPerson())info.setReturnValue(null);
    }
}
