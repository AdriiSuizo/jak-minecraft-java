package dev.jakmc.mixin.client;

import net.minecraft.client.Minecraft;
import net.minecraft.client.gui.screens.Screen;
import org.spongepowered.asm.mixin.Mixin;
import org.spongepowered.asm.mixin.injection.*;
import org.spongepowered.asm.mixin.injection.callback.CallbackInfo;

/** 26.3's server-save executor is shared with terrain builds. A worker spinning
 * on a full GPU upload buffer can starve saving once disconnect stops drawing
 * the level. Cancel/release those render tasks before waiting for the server. */
@Mixin(Minecraft.class)
public abstract class JakDisconnectMixin {
 @Inject(method="disconnect(Lnet/minecraft/client/gui/screens/Screen;ZZ)V",at=@At(value="INVOKE",target="Lnet/minecraft/client/server/IntegratedServer;halt(Z)V"))
 private void jak$releaseTerrainBeforeServerSave(Screen screen,boolean keepPacks,boolean stopSound,CallbackInfo info){
  ((Minecraft)(Object)this).levelRenderer.resetLevelRenderData();
 }
}
