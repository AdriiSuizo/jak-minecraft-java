package dev.jakmc.mixin.client;

import dev.jakmc.client.model.PropModels;
import org.spongepowered.asm.mixin.Mixin;
import org.spongepowered.asm.mixin.Pseudo;
import org.spongepowered.asm.mixin.Shadow;
import org.spongepowered.asm.mixin.Unique;
import org.spongepowered.asm.mixin.injection.At;
import org.spongepowered.asm.mixin.injection.Inject;
import org.spongepowered.asm.mixin.injection.callback.CallbackInfo;
import org.spongepowered.asm.mixin.injection.callback.CallbackInfoReturnable;

/** Restore the facing of copied prop quads and keep the barrel shells visible from every direction. */
@Pseudo
@Mixin(targets="net.caffeinemc.mods.sodium.client.render.model.QuadViewImpl",remap=false)
abstract class JakSodiumPropMixin {
    @Shadow public abstract int getTag();
    @Shadow protected boolean isGeometryInvalid;
    @Unique private static volatile Enum<?> jakmc$unassigned;

    @Inject(method="load",at=@At("TAIL"),require=0)
    private void jakmc$restoreFacing(CallbackInfo ci){
        int tag=getTag();
        // Sodium's packed mesh copy restores the normal but leaves its facing cache stale.
        if(tag==PropModels.PROP_QUAD_TAG||tag==PropModels.BARREL_QUAD_TAG)isGeometryInvalid=true;
    }

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
