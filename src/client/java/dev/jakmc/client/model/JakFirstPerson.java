package dev.jakmc.client.model;

import dev.jakmc.JakData;
import dev.jakmc.client.JakPoses;
import dev.jakmc.content.JakItems;
import net.fabricmc.fabric.api.client.rendering.v1.level.LevelRenderEvents;
import net.minecraft.client.Minecraft;
import net.minecraft.world.level.GameType;

/** Original view rig in the world pass, before the same frame's muzzle effects. */
public final class JakFirstPerson {
    public static long suppressedMainHands;
    private JakFirstPerson() {}
    public static boolean active(){
        var c=Minecraft.getInstance();var p=c.player;
        return p!=null&&c.level!=null&&c.gameMode!=null&&c.getCameraEntity()==p
            &&c.options.getCameraType().isFirstPerson()&&!c.gui.hud.isHidden()
            &&c.gameMode.getPlayerMode()!=GameType.SPECTATOR&&!p.isSleeping()&&!p.isScoping()
            &&p.getAttachedOrCreate(JakData.ENABLED)&&JakPoses.rendered(p.getId())!=null
            &&JakItems.GUNS.contains(p.getMainHandItem().getItem());
    }
    public static void initialize(){
        LevelRenderEvents.COLLECT_SUBMITS.register(context->{
            if(!active())return;
            var c=Minecraft.getInstance();var pose=JakPoses.rendered(c.player.getId());
            if(c.player.isPassenger()||pose.board()||pose.dark()
                ||pose.raw().get("ride") instanceof com.google.gson.JsonObject
                ||pose.raw().get("hellcat") instanceof com.google.gson.JsonObject
                ||pose.raw().get("shield").getAsBoolean()
                ||!pose.raw().getAsJsonObject("gun").get("out").getAsBoolean())return;
            var avatar=context.levelState().playerRenderState.avatarRenderState;
            if(avatar==null)return;
            var camera=context.levelState().cameraRenderState;
            var position=JakPoses.presented();if(position==null)position=c.player.position();
            var stack=context.poseStack();stack.pushPose();
            stack.translate(position.x-camera.pos.x,position.y-camera.pos.y,position.z-camera.pos.z);
            JakRenderer.submitFirstPerson(avatar,stack,context.submitNodeCollector(),camera);
            stack.popPose();
        });
    }
}
