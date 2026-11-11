package dev.jakmc.mixin.client;

import com.llamalad7.mixinextras.injector.ModifyExpressionValue;
import dev.jakmc.client.JakInput;
import net.minecraft.client.DeltaTracker;
import net.minecraft.client.Minecraft;
import net.minecraft.client.gui.GuiGraphicsExtractor;
import net.minecraft.client.gui.Hud;
import net.minecraft.network.chat.Component;
import net.minecraft.world.phys.EntityHitResult;
import org.spongepowered.asm.mixin.injection.Inject;
import org.spongepowered.asm.mixin.injection.callback.CallbackInfo;
import org.spongepowered.asm.mixin.Mixin;
import org.spongepowered.asm.mixin.injection.At;

@Mixin(Hud.class)
abstract class JakAimCrosshairMixin {
    @Inject(method="extractCrosshair",at=@At("TAIL"))
    private void jakmc$mountPrompt(GuiGraphicsExtractor graphics,DeltaTracker delta,CallbackInfo ci){
        var client=Minecraft.getInstance();
        var player=client.player;
        if(player==null||player.isSpectator()||!dev.jakmc.player.JakPlayers.active(player)||
            !(client.hitResult instanceof EntityHitResult hit))return;
        var pose=dev.jakmc.client.JakPoses.get(player.getId());
        if(pose!=null&&(pose.raw().get("ride") instanceof com.google.gson.JsonObject||pose.raw().has("hellcat")))return;
        boolean ready=hit.getEntity() instanceof dev.jakmc.entity.JakLeaper mount&&mount.riderId()<0&&
            mount.readyToMount()&&player.distanceToSqr(mount)<=16;
        if(hit.getEntity() instanceof dev.jakmc.entity.JakHellcat cat)ready=cat.canBoard(player,hit.getLocation());
        if(hit.getEntity() instanceof dev.jakmc.entity.JakVehicle car)ready=car.canBoard(player,hit.getLocation());
        if(!ready)return;
        var label=Component.translatable(JakInput.controller().id()!=0?"hint.jakmc.leaper_pad":"hint.jakmc.leaper_mouse");
        graphics.nextStratum();
        graphics.text(client.font,label,(graphics.guiWidth()-client.font.width(label))/2,
            graphics.guiHeight()/2+14,-1,true);
    }

    @ModifyExpressionValue(method="extractCrosshair",at=@At(value="INVOKE",
        target="Lnet/minecraft/client/CameraType;isFirstPerson()Z"))
    private boolean jakmc$showCrosshair(boolean firstPerson){
        if(firstPerson||JakInput.aiming())return true;
        var client=Minecraft.getInstance();
        var player=client.player;
        if(player==null||player.isSpectator()||client.options.getCameraType()!=net.minecraft.client.CameraType.THIRD_PERSON_BACK
            ||!dev.jakmc.player.JakPlayers.active(player))return false;
        var pose=dev.jakmc.client.JakPoses.rendered(player.getId());
        return pose!=null&&!pose.board()&&!(pose.raw().get("ride") instanceof com.google.gson.JsonObject)&&!pose.raw().has("hellcat");
    }
}
