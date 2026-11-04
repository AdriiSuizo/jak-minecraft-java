package dev.jakmc.mixin.client;

import net.minecraft.client.MouseHandler;
import net.minecraft.client.input.MouseButtonInfo;
import org.spongepowered.asm.mixin.Mixin;
import org.spongepowered.asm.mixin.gen.Accessor;

@Mixin(MouseHandler.class)
public interface JakMouseCapture {
    @Accessor("activeButton") void jakmc$activeButton(MouseButtonInfo button);
}
