package dev.jakmc.mixin.client;
import dev.jakmc.client.model.JakRenderer;
import com.mojang.blaze3d.vertex.PoseStack;
import net.minecraft.client.renderer.SubmitNodeCollector;
import net.minecraft.client.renderer.entity.LivingEntityRenderer;
import net.minecraft.client.renderer.entity.state.*;
import net.minecraft.client.renderer.state.level.CameraRenderState;
import org.spongepowered.asm.mixin.Mixin;
import org.spongepowered.asm.mixin.injection.*;
import org.spongepowered.asm.mixin.injection.callback.CallbackInfo;
@Mixin(LivingEntityRenderer.class)
abstract class JakRenderMixin extends net.minecraft.client.renderer.entity.EntityRenderer<net.minecraft.world.entity.LivingEntity,LivingEntityRenderState> {
 protected JakRenderMixin(net.minecraft.client.renderer.entity.EntityRendererProvider.Context context){super(context);}
 @Inject(method="submit(Lnet/minecraft/client/renderer/entity/state/LivingEntityRenderState;Lcom/mojang/blaze3d/vertex/PoseStack;Lnet/minecraft/client/renderer/SubmitNodeCollector;Lnet/minecraft/client/renderer/state/level/CameraRenderState;)V",at=@At("HEAD"),cancellable=true)
 private void jakmc$render(LivingEntityRenderState state,PoseStack stack,SubmitNodeCollector collector,CameraRenderState camera,CallbackInfo ci){boolean replaced=state instanceof AvatarRenderState avatar&&JakRenderer.submit(avatar,stack,collector)||state instanceof VillagerRenderState villager&&dev.jakmc.client.model.JakCitizenRenderer.submit(villager,stack,collector);if(replaced){super.submit(state,stack,collector,camera);ci.cancel();}}
}
