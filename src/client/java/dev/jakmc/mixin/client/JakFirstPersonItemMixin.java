package dev.jakmc.mixin.client;

import com.mojang.blaze3d.vertex.PoseStack;
import dev.jakmc.client.model.JakFirstPerson;
import net.minecraft.client.renderer.FirstPersonHandsAndItemsRenderer;
import net.minecraft.client.renderer.SubmitNodeCollector;
import net.minecraft.client.renderer.state.level.FirstPersonHandsAndItemsRenderState;
import net.minecraft.client.renderer.state.level.PlayerRenderState;
import net.minecraft.world.InteractionHand;
import net.minecraft.world.item.ItemStack;
import org.spongepowered.asm.mixin.Mixin;
import org.spongepowered.asm.mixin.injection.*;
import org.spongepowered.asm.mixin.injection.callback.CallbackInfo;

@Mixin(FirstPersonHandsAndItemsRenderer.class)
abstract class JakFirstPersonItemMixin {
    @Inject(method="submitArmWithItem",at=@At("HEAD"),cancellable=true)
    private void jakmc$originalGun(PlayerRenderState player,FirstPersonHandsAndItemsRenderState state,
            float partial,float pitch,InteractionHand hand,float attack,ItemStack item,float inverseHeight,
            PoseStack stack,SubmitNodeCollector collector,int light,CallbackInfo ci){
        if(hand==InteractionHand.MAIN_HAND&&JakFirstPerson.active()){
            JakFirstPerson.suppressedMainHands++;ci.cancel();
        }
    }
}
