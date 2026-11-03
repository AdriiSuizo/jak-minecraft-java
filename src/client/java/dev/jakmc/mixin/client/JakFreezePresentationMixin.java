package dev.jakmc.mixin.client;

import dev.jakmc.player.JakEntityClock;
import net.minecraft.client.renderer.extract.LevelExtractor;
import net.minecraft.world.entity.Entity;
import org.spongepowered.asm.mixin.Mixin;
import org.spongepowered.asm.mixin.injection.At;
import org.spongepowered.asm.mixin.injection.ModifyArgs;
import org.spongepowered.asm.mixin.injection.invoke.arg.Args;

@Mixin(LevelExtractor.class)
abstract class JakFreezePresentationMixin {
    @ModifyArgs(method="extractVisibleEntities",at=@At(value="INVOKE",target=
        "Lnet/minecraft/client/renderer/extract/LevelExtractor;extractEntity(Lnet/minecraft/world/entity/Entity;F)Lnet/minecraft/client/renderer/entity/state/EntityRenderState;"))
    private void jakmc$advancePresentation(Args args){
        Entity entity=args.get(0);
        var clock=JakEntityClock.of(entity.level());
        if(clock.rate()<1&&JakEntityClock.affected(entity)){
            args.set(1,clock.partial(args.get(1)));
        }
    }
}
