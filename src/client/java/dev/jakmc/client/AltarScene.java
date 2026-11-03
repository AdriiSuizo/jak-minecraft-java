package dev.jakmc.client;

import com.google.gson.JsonObject;
import dev.jakmc.JakData;
import dev.jakmc.client.model.BoardAltarRenderer;
import dev.jakmc.content.JakBlocks;
import dev.jakmc.content.JakItems;
import dev.jakmc.player.JakPlayers;
import java.io.IOException;
import java.io.UncheckedIOException;
import java.nio.file.Files;
import java.util.LinkedHashMap;
import net.fabricmc.fabric.api.client.event.lifecycle.v1.ClientTickEvents;
import net.minecraft.client.Minecraft;
import net.minecraft.client.Screenshot;
import net.minecraft.core.BlockPos;
import net.minecraft.core.Direction;
import net.minecraft.server.level.ServerPlayer;
import net.minecraft.world.InteractionHand;
import net.minecraft.world.item.ItemStack;
import net.minecraft.world.level.GameType;
import net.minecraft.world.level.block.state.BlockState;
import net.minecraft.world.level.levelgen.Heightmap;
import net.minecraft.world.phys.BlockHitResult;
import net.minecraft.world.phys.Vec3;

final class AltarScene {
    private static final int BOARD_BUTTON=1<<7;
    private static InputScene.CameraView view;
    private static JsonObject checks;
    private static int ticks,opened,closed;
    private static boolean deployed;
    private static Runnable take,press,release,returnBoard,cleanup;

    static InputScene.CameraView camera(){return view;}

    static void initialize(){
        ClientTickEvents.END_CLIENT_TICK.register(client->{
            if(checks==null||client.player==null)return;
            ++ticks;
            var pose=JakPoses.get(client.player.getId());
            if(ticks>=32&&pose!=null&&pose.board())deployed=true;
            var server=client.getSingleplayerServer();
            if(ticks==26)
                Screenshot.grab(client.gameDirectory,"jak-altar.png",client.gameRenderer.mainRenderTarget(),1,_-> {});
            if(ticks==32)server.execute(take);
            if(ticks==74)server.execute(press);
            if(ticks==77)server.execute(release);
            if(ticks==92)server.execute(returnBoard);
            if(ticks<124||!checks.has("returned_to_altar"))return;
            checks.addProperty("native_board_deployed",deployed);
            checks.addProperty("original_board_open",BoardAltarRenderer.openingFrames>opened);
            checks.addProperty("original_board_close",BoardAltarRenderer.closingFrames>closed);
            var result=new JsonObject();
            result.add("actions",checks);
            result.addProperty("reason",checks.entrySet().stream().allMatch(e->e.getValue().getAsBoolean())?
                "complete":"Altar assertion failed");
            try{
                var dir=client.gameDirectory.toPath().resolve("jakmc/evidence/altar");
                Files.createDirectories(dir);
                Files.writeString(dir.resolve("result.json"),result.toString());
            }catch(IOException e){throw new UncheckedIOException(e);}
            checks=null;
            view=null;
            server.execute(cleanup);
            client.stop();
        });
    }

    static void start(Minecraft client){
        var server=client.getSingleplayerServer();
        if(server==null||client.player==null)throw new IllegalStateException("Open the development world");
        ticks=0;
        opened=BoardAltarRenderer.openingFrames;
        closed=BoardAltarRenderer.closingFrames;
        deployed=false;
        var uuid=client.player.getUUID();
        server.execute(()->{
            var p=server.getPlayerList().getPlayer(uuid);
            if(p==null)return;
            var level=p.level();
            var before=p.position();
            var mode=p.gameMode.getGameModeForPlayer();
            var hand=p.getMainHandItem().copy();
            var offhand=p.getOffhandItem().copy();
            var board=p.getAttachedOrCreate(JakData.BOARD).copy();
            var enabled=p.getAttachedOrCreate(JakData.ENABLED);
            var near=p.blockPosition();
            int y=near.getY();
            for(int x=-4;x<=5;x++)for(int z=-4;z<=4;z++)
                y=Math.max(y,level.getHeight(Heightmap.Types.MOTION_BLOCKING,near.getX()+x,near.getZ()+z));
            var floor=near.atY(y);
            var pos=floor.east(2).above();
            var snapshot=new LinkedHashMap<BlockPos,BlockState>();
            var tile=JakBlocks.BLOCKS.get("palacio_suelo_metal");
            for(int x=-3;x<=4;x++)for(int z=-3;z<=3;z++){
                var at=floor.offset(x,0,z);
                snapshot.put(at,level.getBlockState(at));
                level.setBlockAndUpdate(at,tile.defaultBlockState());
            }
            snapshot.put(pos,level.getBlockState(pos));
            var filled=JakBlocks.BLOCKS.get("altar_turbotabla");
            var empty=JakBlocks.BLOCKS.get("altar_turbotabla_vacio");
            level.setBlockAndUpdate(pos,filled.defaultBlockState());
            var entry=Vec3.atBottomCenterOf(floor.above());
            p.teleportTo(entry.x,entry.y,entry.z);
            p.setYRot(-90);
            p.setXRot(0);
            p.setGameMode(GameType.SURVIVAL);
            p.setOnGround(true);
            p.setItemInHand(InteractionHand.MAIN_HAND,ItemStack.EMPTY);
            p.setAttached(JakData.ENABLED,true);
            p.setAttached(JakData.BOARD,new ItemStack(JakItems.BOARD));
            JakPlayers.controlForProbe(p,true);
            var report=new JsonObject();
            var plate=JakBlocks.BLOCKS.get("arena_luz_placa").defaultBlockState();
            var plateShape=plate.getCollisionShape(level,pos);
            report.addProperty("solid_luminous_floor_plate",plate.getLightEmission()==15
                &&plateShape.min(Direction.Axis.Y)==.5&&plateShape.max(Direction.Axis.Y)==1
                &&plateShape.equals(plate.getShape(level,pos)));
            report.addProperty("refuses_second_board",!interact(p,pos)&&level.getBlockState(pos).is(filled));
            p.setAttached(JakData.BOARD,ItemStack.EMPTY);
            take=()->{
                boolean used=interact(p,pos);
                boolean took=used&&level.getBlockState(pos).is(empty)&&p.getAttachedOrCreate(JakData.BOARD).is(JakItems.BOARD);
                client.execute(()->report.addProperty("equipped_directly",took));
            };
            press=()->JakPlayers.probeInput(p,BOARD_BUTTON,BOARD_BUTTON,0);
            release=()->JakPlayers.probeInput(p,0,0,0);
            returnBoard=()->{
                boolean used=interact(p,pos);
                boolean returned=used&&level.getBlockState(pos).is(filled)&&p.getAttachedOrCreate(JakData.BOARD).isEmpty();
                client.execute(()->{
                    report.addProperty("offhand_preserved",ItemStack.matches(offhand,p.getOffhandItem()));
                    report.addProperty("returned_to_altar",returned);
                });
            };
            cleanup=()->{
                level.setBlock(pos,empty.defaultBlockState(),net.minecraft.world.level.block.Block.UPDATE_CLIENTS
                    |net.minecraft.world.level.block.Block.UPDATE_SKIP_BLOCK_ENTITY_SIDEEFFECTS);
                snapshot.forEach(level::setBlockAndUpdate);
                JakPlayers.controlForProbe(p,false);
                p.setAttached(JakData.ENABLED,enabled);
                p.setAttached(JakData.BOARD,board);
                p.setItemInHand(InteractionHand.MAIN_HAND,hand);
                p.setGameMode(mode);
                p.teleportTo(before.x,before.y,before.z);
            };
            Vec3 target=Vec3.atBottomCenterOf(pos).add(-.5,.85,0);
            Vec3 eye=target.add(3,2.1,-4.1);
            Vec3 delta=target.subtract(eye);
            client.execute(()->{
                view=new InputScene.CameraView(eye,(float)Math.toDegrees(Math.atan2(-delta.x,delta.z)),
                    (float)-Math.toDegrees(Math.atan2(delta.y,delta.horizontalDistance())));
                checks=report;
            });
        });
    }

    private static boolean interact(ServerPlayer player,BlockPos pos){
        var hit=new BlockHitResult(Vec3.atCenterOf(pos),Direction.UP,pos,false);
        return player.level().getBlockState(pos).useWithoutItem(player.level(),player,hit).consumesAction();
    }
}
