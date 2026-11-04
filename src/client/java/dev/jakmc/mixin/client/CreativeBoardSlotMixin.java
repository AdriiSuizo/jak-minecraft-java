package dev.jakmc.mixin.client;

import dev.jakmc.player.BoardSlot;
import net.minecraft.client.gui.screens.inventory.CreativeModeInventoryScreen;
import org.spongepowered.asm.mixin.Mixin;
import org.spongepowered.asm.mixin.injection.*;
import org.spongepowered.asm.mixin.injection.invoke.arg.Args;

@Mixin(CreativeModeInventoryScreen.class)
abstract class CreativeBoardSlotMixin {
    @ModifyArgs(method="selectTab",at=@At(value="INVOKE",target="Lnet/minecraft/client/gui/screens/inventory/CreativeModeInventoryScreen$SlotWrapper;<init>(Lnet/minecraft/world/inventory/Slot;III)V"))
    private void jakmc$boardPosition(Args args){
        if(args.get(0) instanceof BoardSlot){args.set(2,126);args.set(3,20);}
    }
}
