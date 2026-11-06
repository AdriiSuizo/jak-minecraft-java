package dev.jakmc.mixin.client;
import com.mojang.blaze3d.vertex.PoseStack;
import dev.jakmc.client.model.MobTumble;
import java.util.Map;
import java.util.WeakHashMap;
import net.minecraft.client.renderer.SubmitNodeCollector;
import net.minecraft.client.renderer.entity.LivingEntityRenderer;
import net.minecraft.client.renderer.entity.state.*;
import net.minecraft.client.renderer.state.level.CameraRenderState;
import net.minecraft.world.entity.LivingEntity;
import org.joml.Quaternionf;
import org.joml.Vector3f;
import org.spongepowered.asm.mixin.Mixin;
import org.spongepowered.asm.mixin.Unique;
import org.spongepowered.asm.mixin.injection.*;
import org.spongepowered.asm.mixin.injection.callback.CallbackInfo;
/** Rust client_mobs.rs contact_pose: a Mass Inverter tumble turns the creature about its middle (MobTumble). */
@Mixin(LivingEntityRenderer.class)
abstract class JakTumbleMixin {
 @Unique private static final Map<LivingEntityRenderState,Object[]> jakmc$tumbles=new WeakHashMap<>();
 @Inject(method="extractRenderState(Lnet/minecraft/world/entity/LivingEntity;Lnet/minecraft/client/renderer/entity/state/LivingEntityRenderState;F)V",at=@At("TAIL"))
 private void jakmc$extract(LivingEntity entity,LivingEntityRenderState state,float partial,CallbackInfo ci){
  var placed=state instanceof AvatarRenderState||state instanceof VillagerRenderState?null:MobTumble.place(entity.getId(),partial);
  if(placed==null)jakmc$tumbles.remove(state);else jakmc$tumbles.put(state,placed);
 }
 @Inject(method="submit(Lnet/minecraft/client/renderer/entity/state/LivingEntityRenderState;Lcom/mojang/blaze3d/vertex/PoseStack;Lnet/minecraft/client/renderer/SubmitNodeCollector;Lnet/minecraft/client/renderer/state/level/CameraRenderState;)V",at=@At("HEAD"))
 private void jakmc$turn(LivingEntityRenderState state,PoseStack stack,SubmitNodeCollector collector,CameraRenderState camera,CallbackInfo ci){
  // Only a creature tumbling, never a player or a citizen (JakRenderMixin may end those early): pushed here, popped at the end.
  var placed=jakmc$tumbles.get(state);if(placed==null)return;
  stack.pushPose();
  var offset=(Vector3f)placed[0];stack.translate(offset.x,offset.y,offset.z);stack.mulPose(new org.joml.Matrix4f().rotation((Quaternionf)placed[1]));
 }
 @Inject(method="submit(Lnet/minecraft/client/renderer/entity/state/LivingEntityRenderState;Lcom/mojang/blaze3d/vertex/PoseStack;Lnet/minecraft/client/renderer/SubmitNodeCollector;Lnet/minecraft/client/renderer/state/level/CameraRenderState;)V",at=@At("RETURN"))
 private void jakmc$untern(LivingEntityRenderState state,PoseStack stack,SubmitNodeCollector collector,CameraRenderState camera,CallbackInfo ci){
  if(jakmc$tumbles.containsKey(state))stack.popPose();
 }
}
