package dev.jakmc.mixin;
import dev.jakmc.entity.CitizenState;
import net.minecraft.world.entity.*;
import net.minecraft.world.item.*;
import net.minecraft.core.component.DataComponents;
import org.spongepowered.asm.mixin.Mixin;
import org.spongepowered.asm.mixin.injection.*;
/** Native offspring path applies item components but omits ENTITY_DATA; transfer only our variant. */
@Mixin(SpawnEggItem.class)
abstract class CitizenEggMixin {
 @Redirect(method="spawnOffspringFromSpawnEgg",at=@At(value="INVOKE",target="Lnet/minecraft/world/entity/Mob;applyComponentsFromItemStack(Lnet/minecraft/world/item/ItemStack;)V"))
 private static void jakmc$offspring(Mob offspring,ItemStack stack){offspring.applyComponentsFromItemStack(stack);var data=stack.get(DataComponents.ENTITY_DATA);if(offspring.getType()==EntityTypes.VILLAGER&&data!=null&&data.contains("JakCitizenVariant")){int variant=data.copyTagWithoutId().getIntOr("JakCitizenVariant",-1);if(variant>=0)((CitizenState)offspring).citizenVariant(variant);}}
}
