package dev.jakmc.mixin.client;
import dev.jakmc.content.JakItems;
import dev.jakmc.client.model.SkullGemRenderer;
import com.mojang.blaze3d.vertex.PoseStack;
import net.minecraft.client.renderer.SubmitNodeCollector;
import net.minecraft.client.renderer.entity.ItemEntityRenderer;
import net.minecraft.client.renderer.entity.state.ItemEntityRenderState;
import net.minecraft.client.renderer.state.level.CameraRenderState;
import net.minecraft.world.entity.item.ItemEntity;
import net.minecraft.util.Mth;
import org.spongepowered.asm.mixin.*;
import org.spongepowered.asm.mixin.injection.*;
import org.spongepowered.asm.mixin.injection.callback.CallbackInfo;
import java.util.*;
/** Real entity interpolation, no vanilla item hover or duplicate models for a merged stack. */
@Mixin(ItemEntityRenderer.class)
abstract class GemItemRendererMixin {
 @Unique private final Map<ItemEntityRenderState,Float> jakmc$gems=new WeakHashMap<>();
 @Inject(method="extractRenderState(Lnet/minecraft/world/entity/item/ItemEntity;Lnet/minecraft/client/renderer/entity/state/ItemEntityRenderState;F)V",at=@At("TAIL"))
 private void jakmc$extract(ItemEntity item,ItemEntityRenderState state,float partial,CallbackInfo ci){if(item.getItem().is(JakItems.SKULL_GEM))jakmc$gems.put(state,((dev.jakmc.content.GemState)item).gemAngle(partial));else jakmc$gems.remove(state);}
 @Inject(method="submit(Lnet/minecraft/client/renderer/entity/state/ItemEntityRenderState;Lcom/mojang/blaze3d/vertex/PoseStack;Lnet/minecraft/client/renderer/SubmitNodeCollector;Lnet/minecraft/client/renderer/state/level/CameraRenderState;)V",at=@At("HEAD"),cancellable=true)
 private void jakmc$submit(ItemEntityRenderState state,PoseStack stack,SubmitNodeCollector collector,CameraRenderState camera,CallbackInfo ci){Float angle=jakmc$gems.get(state);if(angle!=null){SkullGemRenderer.world(stack,collector,state.lightCoords,angle);ci.cancel();}}
}
