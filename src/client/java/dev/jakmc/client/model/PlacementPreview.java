package dev.jakmc.client.model;

import dev.jakmc.content.JakAssemblies;
import net.fabricmc.fabric.api.client.rendering.v1.level.LevelRenderEvents;
import net.minecraft.client.Minecraft;
import net.minecraft.world.InteractionHand;
import net.minecraft.world.item.context.BlockPlaceContext;
import net.minecraft.world.phys.BlockHitResult;
import net.minecraft.world.phys.HitResult;

/** Adrii 06/11: a Jak object bigger than a block shows, faint, where it would go before it is placed. */
public final class PlacementPreview {
    private static final float ALPHA=.18f;
    private PlacementPreview(){}

    public static void initialize(){
        LevelRenderEvents.COLLECT_SUBMITS.register(context->{
            var c=Minecraft.getInstance();var player=c.player;
            if(player==null||c.level==null||player.isSpectator()||!(c.hitResult instanceof BlockHitResult hit)||hit.getType()!=HitResult.Type.BLOCK)return;
            for(var hand:InteractionHand.values()){
                var stack=player.getItemInHand(hand);
                if(stack.isEmpty())continue;
                if(stack.getItem() instanceof dev.jakmc.content.JakIrisDoor.DoorItem door){
                    if(IrisDoorRenderer.ghost(door,new BlockPlaceContext(player,hand,stack,hit),context.levelState().cameraRenderState.pos,context.poseStack(),context.submitNodeCollector(),ALPHA))return;
                    continue;
                }
                var cells=JakAssemblies.preview(stack.getItem(),new BlockPlaceContext(player,hand,stack,hit));
                if(cells.isEmpty())continue;
                var camera=context.levelState().cameraRenderState.pos;
                PropModels.submitGhost(cells,camera,context.poseStack(),context.submitNodeCollector(),ALPHA);
                return;
            }
        });
    }
}
