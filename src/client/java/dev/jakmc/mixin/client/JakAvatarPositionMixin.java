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
        var movie=dev.jakmc.client.MovieMotion.pose(entity.getId());
        var rider=entity instanceof net.minecraft.world.entity.player.Player
            && entity.getVehicle() instanceof dev.jakmc.entity.JakVehicle car ? car : null;
        boolean replay=JakPoses.inReplay();
        if(movie==null&&entity!=Minecraft.getInstance().player&&!(replay&&rider!=null))return;
        if(replay&&rider!=null)dev.jakmc.client.MovieVehicles.sample(rider,partial);
        var position=rider!=null?rider.presentationRider():movie==null?JakPoses.presented():movie.position();
        if(position!=null){
            state.x=position.x;
            state.y=position.y;
            state.z=position.z;
        }
    }
}
