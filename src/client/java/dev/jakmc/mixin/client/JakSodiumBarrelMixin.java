package dev.jakmc.mixin.client;

import dev.jakmc.client.model.PropModels;
import org.spongepowered.asm.mixin.Mixin;
import org.spongepowered.asm.mixin.Pseudo;
import org.spongepowered.asm.mixin.Shadow;
import org.spongepowered.asm.mixin.Unique;
import org.spongepowered.asm.mixin.injection.At;
import org.spongepowered.asm.mixin.injection.Inject;
import org.spongepowered.asm.mixin.injection.callback.CallbackInfoReturnable;

/** Submit the barrel shells in Sodium's camera-independent face bin. */
@Pseudo
@Mixin(targets="net.caffeinemc.mods.sodium.client.render.model.QuadViewImpl",remap=false)
abstract class JakSodiumBarrelMixin {
    @Shadow public abstract int getTag();
    @Unique private static volatile Enum<?> jakmc$unassigned;

    @Inject(method="normalFace",at=@At("RETURN"),cancellable=true,require=0)
    private void jakmc$keepBarrelsVisible(CallbackInfoReturnable<Enum<?>> result){
        if(getTag()!=PropModels.BARREL_QUAD_TAG)return;
        Enum<?> unassigned=jakmc$unassigned;
        if(unassigned==null){
            for(var facing:result.getReturnValue().getDeclaringClass().getEnumConstants()){
                if(facing.name().equals("UNASSIGNED")){
                    jakmc$unassigned=unassigned=facing;
                    break;
                }
            }
        }
        if(unassigned!=null)result.setReturnValue(unassigned);
    }
}
