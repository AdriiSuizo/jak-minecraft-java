package dev.jakmc.mixin.client;

import dev.jakmc.client.JakPoses;
import net.minecraft.client.Minecraft;
import net.minecraft.client.renderer.entity.EntityRenderer;
import net.minecraft.client.renderer.entity.state.EntityRenderState;
import net.minecraft.world.entity.Entity;
import org.spongepowered.asm.mixin.Mixin;
import org.spongepowered.asm.mixin.injection.*;
import org.spongepowered.asm.mixin.injection.callback.CallbackInfo;

@Mixin(EntityRenderer.class)
abstract class JakAvatarPositionMixin {
    @Inject(method="extractRenderState",at=@At("TAIL"))
    private void jakmc$present(Entity entity,EntityRenderState state,float partial,CallbackInfo ci){
        if(entity!=Minecraft.getInstance().player)return;
        var position=JakPoses.presented();
        if(position!=null){
            state.x=position.x;
            state.y=position.y;
            state.z=position.z;
        }
    }
}
