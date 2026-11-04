package dev.jakmc.mixin.client;

import net.minecraft.client.gui.screens.inventory.CreativeModeInventoryScreen;
import net.minecraft.world.item.CreativeModeTab;
import net.minecraft.world.inventory.Slot;
import net.minecraft.world.inventory.ContainerInput;
import org.spongepowered.asm.mixin.Mixin;
import org.spongepowered.asm.mixin.gen.Invoker;

/** Native UI entry points for the isolated development bridge. */
@Mixin(CreativeModeInventoryScreen.class)
public interface CreativeInventoryActions {
    @Invoker("selectTab") void jakmc$selectTab(CreativeModeTab tab);
    @Invoker("slotClicked") void jakmc$clickSlot(Slot slot,int index,int button,ContainerInput input);
}
