package dev.jakmc.mixin.client;
import dev.jakmc.client.model.JakCitizenRenderer;
import net.minecraft.client.renderer.entity.VillagerRenderer;
import net.minecraft.client.renderer.entity.state.VillagerRenderState;
import net.minecraft.world.entity.npc.villager.Villager;
import org.spongepowered.asm.mixin.Mixin;
import org.spongepowered.asm.mixin.injection.*;
import org.spongepowered.asm.mixin.injection.callback.CallbackInfo;
@Mixin(VillagerRenderer.class)
abstract class CitizenRenderStateMixin {
 @Inject(method="extractRenderState(Lnet/minecraft/world/entity/npc/villager/Villager;Lnet/minecraft/client/renderer/entity/state/VillagerRenderState;F)V",at=@At("TAIL"))private void jakmc$state(Villager v,VillagerRenderState state,float partial,CallbackInfo ci){JakCitizenRenderer.extract(v,state,partial);}
}
