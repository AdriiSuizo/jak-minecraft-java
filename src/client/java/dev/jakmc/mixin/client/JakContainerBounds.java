package dev.jakmc.mixin.client;

import net.minecraft.client.gui.screens.inventory.AbstractContainerScreen;
import org.spongepowered.asm.mixin.Mixin;
import org.spongepowered.asm.mixin.gen.Accessor;

@Mixin(AbstractContainerScreen.class)
public interface JakContainerBounds {
    @Accessor("hoveredSlot") net.minecraft.world.inventory.Slot jakmc$hoveredSlot();
    @Accessor("leftPos") int jakmc$left();
    @Accessor("topPos") int jakmc$top();
}
