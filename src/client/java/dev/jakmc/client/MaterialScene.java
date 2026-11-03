package dev.jakmc.client;

import com.google.gson.JsonObject;
import dev.jakmc.content.JakBlocks;
import java.nio.file.Files;
import net.fabricmc.fabric.api.client.event.lifecycle.v1.ClientTickEvents;
import net.minecraft.client.Minecraft;
import net.minecraft.client.Screenshot;
import net.minecraft.core.BlockPos;
import net.minecraft.core.Direction;
import net.minecraft.server.level.ServerPlayer;
import net.minecraft.world.InteractionHand;
import net.minecraft.world.item.ItemStack;
import net.minecraft.world.item.Items;
import net.minecraft.world.item.context.UseOnContext;
import net.minecraft.world.level.GameType;
import net.minecraft.world.level.block.Block;
import net.minecraft.world.level.block.Blocks;
import net.minecraft.world.level.block.state.properties.BlockStateProperties;
import net.minecraft.world.phys.BlockHitResult;
import net.minecraft.world.phys.Vec3;

final class MaterialScene {
    private static int captureTicks;
    private static volatile JsonObject result;
    private static volatile Vec3 subject;
    private static Runnable cleanup;

    static void initialize(){
        ClientTickEvents.END_CLIENT_TICK.register(client->{
            if(result==null||++captureTicks<30)return;
            Screenshot.grab(client.gameDirectory,"jak-materials.png",client.gameRenderer.mainRenderTarget(),1,_->{});
            try{
                var dir=client.gameDirectory.toPath().resolve("jakmc/evidence/materials");
                Files.createDirectories(dir);
                Files.writeString(dir.resolve("result.json"),result.toString());
            }catch(java.io.IOException e){throw new java.io.UncheckedIOException(e);}
            result=null;
            var restore=cleanup;
            if(restore!=null)client.getSingleplayerServer().execute(restore);
            client.stop();
        });
    }

    static InputScene.CameraView camera(){
        if(result==null||subject==null)return null;
        Vec3 pos=subject.add(2.7,2.2,-3.6);
        Vec3 look=subject.subtract(pos);
        return new InputScene.CameraView(pos,(float)Math.toDegrees(Math.atan2(-look.x,look.z)),
            (float)-Math.toDegrees(Math.atan2(look.y,look.horizontalDistance())));
    }

    static void start(Minecraft client){
        if(client.getSingleplayerServer()==null||client.player==null)throw new IllegalStateException("Open the development world");
        captureTicks=0;
        var server=client.getSingleplayerServer();
        var uuid=client.player.getUUID();
        server.execute(()->{
            ServerPlayer p=server.getPlayerList().getPlayer(uuid);
            if(p==null)return;
            var level=p.level();
            var base=p.blockPosition().offset(3,0,3);
            var soil=base.below();
            var hand=InteractionHand.MAIN_HAND;
            var previous=p.getItemInHand(hand).copy();
            var previousMode=p.gameMode.getGameModeForPlayer();
            var initial=level.getBlockState(base);
            var initialFloor=level.getBlockState(soil);
            var sample=base.east();
            var initialSample=level.getBlockState(sample);
            cleanup=()->{
                level.setBlockAndUpdate(soil,initialFloor);
                level.setBlockAndUpdate(base,initial);
                level.setBlockAndUpdate(sample,initialSample);
            };
            var layer=JakBlocks.BLOCKS.get("barrio_losa_capa");
            var tests=new JsonObject();
            try{
                level.setBlockAndUpdate(soil,Blocks.STONE.defaultBlockState());
                level.setBlockAndUpdate(base,Blocks.AIR.defaultBlockState());
                p.setGameMode(GameType.SURVIVAL);
                p.setItemInHand(hand,new ItemStack(layer,8));
                boolean placed=use(p,soil,1)&&use(p,base,.125)&&use(p,base,.25);
                var state=level.getBlockState(base);
                tests.addProperty("placed_and_stacked",placed&&p.getItemInHand(hand).getCount()==5&&state.is(layer)
                    &&state.getValue(BlockStateProperties.LAYERS)==3);
                tests.addProperty("collision_matches_height",state.is(layer)&&state.getCollisionShape(level,base).max(Direction.Axis.Y)==.375
                    &&state.getShape(level,base).max(Direction.Axis.Y)==.375);
                var pickaxe=new ItemStack(Items.DIAMOND_PICKAXE);
                tests.addProperty("native_loot",Block.getDrops(state,level,base,null,p,pickaxe).stream()
                    .filter(stack->stack.is(layer.asItem())).mapToInt(ItemStack::getCount).sum()==3);
                tests.addProperty("material_catalog",JakBlocks.BLOCKS.size()>1000);
                tests.addProperty("native_door",JakBlocks.BLOCKS.values().stream().anyMatch(block->block instanceof net.minecraft.world.level.block.DoorBlock));
                subject=Vec3.atBottomCenterOf(base).add(0,.25,0);
                level.setBlockAndUpdate(sample,JakBlocks.BLOCKS.get("barrio_losa").defaultBlockState());
            }finally{
                p.setGameMode(previousMode);
                p.setItemInHand(hand,previous);
            }
            var report=new JsonObject();
            report.add("actions",tests);
            report.addProperty("blocks",JakBlocks.BLOCKS.size());
            report.addProperty("reason",tests.entrySet().stream().allMatch(e->e.getValue().getAsBoolean())?"complete":"Material assertion failed");
            client.execute(()->result=report);
        });
    }

    private static boolean use(ServerPlayer player,BlockPos clicked,double height){
        var location=new Vec3(clicked.getX()+.5,clicked.getY()+height,clicked.getZ()+.5);
        var hit=new BlockHitResult(location,Direction.UP,clicked,false);
        return player.getItemInHand(InteractionHand.MAIN_HAND).getItem()
            .useOn(new UseOnContext(player,InteractionHand.MAIN_HAND,hit)).consumesAction();
    }
}
