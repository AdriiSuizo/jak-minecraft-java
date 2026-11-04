package dev.jakmc.mixin.client;

import dev.jakmc.player.BoardSlot;
import net.minecraft.client.gui.GuiGraphicsExtractor;
import net.minecraft.client.gui.screens.inventory.AbstractContainerScreen;
import net.minecraft.client.gui.screens.inventory.InventoryScreen;
import net.minecraft.client.gui.screens.inventory.CreativeModeInventoryScreen;
import net.minecraft.client.renderer.RenderPipelines;
import net.minecraft.resources.Identifier;
import org.spongepowered.asm.mixin.Mixin;
import org.spongepowered.asm.mixin.injection.*;
import org.spongepowered.asm.mixin.injection.callback.CallbackInfo;

@Mixin({InventoryScreen.class,CreativeModeInventoryScreen.class})
abstract class BoardSlotScreenMixin {
    @Inject(method="extractRenderState",at=@At("TAIL"))
    private void jakmc$boardTooltip(GuiGraphicsExtractor graphics,int mouseX,int mouseY,float partial,CallbackInfo ci){
        var slot=((JakContainerBounds)this).jakmc$hoveredSlot();
        if(slot!=null&&!slot.hasItem()&&(slot instanceof BoardSlot||slot instanceof CreativeBoardSlotAccess wrapper&&wrapper.jakmc$target() instanceof BoardSlot))
            graphics.setTooltipForNextFrame(net.minecraft.network.chat.Component.translatable("item.jakmc.jet_board"),mouseX,mouseY);
    }
    @Inject(method="extractBackground",at=@At("TAIL"))
    private void jakmc$boardFrame(GuiGraphicsExtractor graphics,int mouseX,int mouseY,float partial,CallbackInfo ci){
        var screen=(AbstractContainerScreen<?>)(Object)this;
        var bounds=(JakContainerBounds)this;
        for(var slot:screen.getMenu().slots){
            boolean board=slot instanceof BoardSlot||slot instanceof CreativeBoardSlotAccess wrapper&&wrapper.jakmc$target() instanceof BoardSlot;
            if(!board)continue;
            // Reuse Minecraft's own equipment-slot frame, including its bevel.
            graphics.blit(RenderPipelines.GUI_TEXTURED,Identifier.withDefaultNamespace("textures/gui/container/inventory.png"),
                bounds.jakmc$left()+slot.x-1,bounds.jakmc$top()+slot.y-1,76,61,18,18,256,256);
        }
    }
}
