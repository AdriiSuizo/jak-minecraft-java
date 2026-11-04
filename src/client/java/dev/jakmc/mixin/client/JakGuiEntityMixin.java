package dev.jakmc.mixin.client;

import com.llamalad7.mixinextras.injector.wrapmethod.WrapMethod;
import com.llamalad7.mixinextras.injector.wrapoperation.Operation;
import com.mojang.blaze3d.vertex.PoseStack;
import dev.jakmc.client.model.JakGuiContext;
import net.minecraft.client.gui.render.pip.GuiEntityRenderer;
import net.minecraft.client.renderer.SubmitNodeCollector;
import net.minecraft.client.renderer.state.gui.pip.GuiEntityRenderState;
import org.spongepowered.asm.mixin.Mixin;

@Mixin(GuiEntityRenderer.class)
abstract class JakGuiEntityMixin {
    @WrapMethod(method="renderToTexture(Lnet/minecraft/client/renderer/state/gui/pip/GuiEntityRenderState;Lcom/mojang/blaze3d/vertex/PoseStack;Lnet/minecraft/client/renderer/SubmitNodeCollector;)V")
    private void jakmc$preview(GuiEntityRenderState state,PoseStack stack,SubmitNodeCollector collector,Operation<Void> original){
        JakGuiContext.enter();
        try{original.call(state,stack,collector);}finally{JakGuiContext.leave();}
    }
}
