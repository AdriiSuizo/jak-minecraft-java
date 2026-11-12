package dev.jakmc.mixin.client;

import com.llamalad7.mixinextras.injector.ModifyExpressionValue;
import dev.jakmc.client.JakInput;
import net.minecraft.client.DeltaTracker;
import net.minecraft.client.Minecraft;
import net.minecraft.client.gui.GuiGraphicsExtractor;
import net.minecraft.client.gui.Hud;
import net.minecraft.network.chat.Component;
import net.minecraft.world.phys.EntityHitResult;
import net.minecraft.world.phys.HitResult;
import org.joml.Matrix4f;
import org.joml.Vector4f;
import org.spongepowered.asm.mixin.injection.Inject;
import org.spongepowered.asm.mixin.injection.callback.CallbackInfo;
import org.spongepowered.asm.mixin.Mixin;
import org.spongepowered.asm.mixin.Unique;
import org.spongepowered.asm.mixin.injection.At;

@Mixin(Hud.class)
abstract class JakAimCrosshairMixin {
    @Unique private boolean jakmc$projected;

    @Inject(method="extractCrosshair",at=@At("HEAD"))
    private void jakmc$projectInteraction(GuiGraphicsExtractor graphics,DeltaTracker delta,CallbackInfo ci){
        var client=Minecraft.getInstance();
        if(client.options.getCameraType()!=net.minecraft.client.CameraType.THIRD_PERSON_BACK
            ||client.getCameraEntity()!=client.player||!jakmc$showCrosshair(false)
            ||dev.jakmc.client.JakPoses.presented()==null
            ||client.hitResult==null||client.hitResult.getType()==HitResult.Type.MISS)return;
        var camera=client.gameRenderer.mainCamera();
        var relative=client.hitResult.getLocation().subtract(camera.position());
        var screen=camera.getViewRotationProjectionMatrix(new Matrix4f())
            .transform(new Vector4f((float)relative.x,(float)relative.y,(float)relative.z,1));
        if(screen.w<=0)return;
        float x=Math.clamp(screen.x/screen.w,-.94f,.94f)*graphics.guiWidth()*.5f;
        float y=Math.clamp(-screen.y/screen.w,-.94f,.94f)*graphics.guiHeight()*.5f;
        graphics.pose().pushMatrix().translate(x,y);
        jakmc$projected=true;
    }

    @Inject(method="extractCrosshair",at=@At("TAIL"))
    private void jakmc$mountPrompt(GuiGraphicsExtractor graphics,DeltaTracker delta,CallbackInfo ci){
        if(jakmc$projected){
            graphics.pose().popMatrix();
            jakmc$projected=false;
        }
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
