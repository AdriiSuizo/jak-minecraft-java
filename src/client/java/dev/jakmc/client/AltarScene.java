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
import net.minecraft.world.item.context.UseOnContext;
import net.minecraft.world.level.block.Blocks;
import net.minecraft.world.level.block.state.properties.BlockStateProperties;
import net.minecraft.world.level.GameType;
import net.minecraft.world.level.block.state.BlockState;
import net.minecraft.world.level.levelgen.Heightmap;
import net.minecraft.world.phys.BlockHitResult;
import net.minecraft.world.phys.Vec3;

final class AltarScene {
    private static final int BOARD_BUTTON=1<<7;
    private static InputScene.CameraView view,gateView;
    private static JsonObject checks;
    private static int ticks,opened,closed;
    private static boolean deployed;
    private static net.minecraft.client.CameraType perspective;
    private static Runnable take,press,release,returnBoard,openGate,cleanup;

    static InputScene.CameraView camera(){return view;}

    static void initialize(){
        ClientTickEvents.END_CLIENT_TICK.register(client->{
            if(checks==null||client.player==null)return;
            ++ticks;
            var pose=JakPoses.get(client.player.getId());
            if(ticks>=32&&pose!=null&&pose.board())deployed=true;
            var server=client.getSingleplayerServer();
            if(ticks==40)
                Screenshot.grab(client.gameDirectory,"jak-altar.png",client.gameRenderer.mainRenderTarget(),1,_-> {});
            if(ticks==32)server.execute(take);
            if(ticks==74)server.execute(press);
            if(ticks==77)server.execute(release);
            if(ticks==92)server.execute(returnBoard);
            if(ticks==101)view=gateView;
            if(ticks==106)Screenshot.grab(client.gameDirectory,"jak-gate-closed.png",client.gameRenderer.mainRenderTarget(),1,_-> {});
            if(ticks==108)server.execute(openGate);
            if(ticks==120)Screenshot.grab(client.gameDirectory,"jak-gate-open.png",client.gameRenderer.mainRenderTarget(),1,_-> {});
            if(ticks<155||!checks.has("returned_to_altar")||!checks.has("gate_raised"))return;
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
            client.options.setCameraType(perspective);
            server.execute(cleanup);
            client.stop();
        });
    }

    static void start(Minecraft client){
        var server=client.getSingleplayerServer();
        if(server==null||client.player==null)throw new IllegalStateException("Open the development world");
        ticks=0;
        perspective=client.options.getCameraType();
        client.options.setCameraType(net.minecraft.client.CameraType.THIRD_PERSON_BACK);
        client.gui.hud.getChat().clearMessages(false);
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
            var inventory=new java.util.ArrayList<ItemStack>();
            for(int i=0;i<p.getInventory().getContainerSize();i++)inventory.add(p.getInventory().getItem(i).copy());
            p.getInventory().clearContent();
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
            for(int x=-3;x<=10;x++)for(int z=-3;z<=8;z++){
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
            var gate=JakBlocks.BLOCKS.get("krimson_porton_fence_gate");
            var first=floor.west(2).north(2).above();
            var second=first.north();
            var power=first.west();
            for(var base:new BlockPos[]{first,second})for(int i=0;i<4;i++)
                snapshot.put(base.above(i),level.getBlockState(base.above(i)));
            snapshot.put(power,level.getBlockState(power));
            p.setItemInHand(InteractionHand.MAIN_HAND,new ItemStack(gate,2));
            boolean gatesPlaced=place(p,first.below())&&place(p,second.below());
            report.addProperty("gate_places_two_columns",gatesPlaced&&p.getMainHandItem().isEmpty()
                &&level.getBlockState(first.above(2)).is(gate)&&level.getBlockState(second.above(3)).is(gate));
            openGate=()->{
                boolean manual=interact(p,first.above());
                boolean cleared=passage(level,first)&&passage(level,second);
                boolean raised=level.getBlockState(first.above(3)).getValue(BlockStateProperties.OPEN)
                    &&!level.getBlockState(first.above(3)).getCollisionShape(level,first.above(3)).isEmpty();
                boolean reclosed=interact(p,first.above(3))&&!passage(level,first)&&!passage(level,second);
                level.setBlockAndUpdate(power,Blocks.REDSTONE_BLOCK.defaultBlockState());
                boolean redstoneOpened=passage(level,first)&&passage(level,second);
                level.setBlockAndUpdate(power,Blocks.AIR.defaultBlockState());
                boolean redstoneClosed=!passage(level,first)&&!passage(level,second);
                interact(p,first.above());
                client.execute(()->{
                    report.addProperty("gate_opens_and_closes",manual&&cleared&&reclosed);
                    report.addProperty("gate_raised",raised);
                    report.addProperty("gate_redstone",redstoneOpened&&redstoneClosed);
                });
            };
            var probeStart=floor.east(7).above();
            var doorway=probeStart.south(3);
            var door=JakBlocks.BLOCKS.get("haven_puerta_alta");
            if(door==null)door=Blocks.OAK_DOOR;
            for(int x=-1;x<=1;x++)for(int h=0;h<6;h++){
                var cell=doorway.offset(x,h,0);snapshot.put(cell,level.getBlockState(cell));
                level.setBlockAndUpdate(cell,Blocks.STONE.defaultBlockState());
            }
            var lower=door.defaultBlockState().setValue(BlockStateProperties.HORIZONTAL_FACING,Direction.SOUTH)
                .setValue(BlockStateProperties.DOUBLE_BLOCK_HALF,net.minecraft.world.level.block.state.properties.DoubleBlockHalf.LOWER);
            var upper=lower.setValue(BlockStateProperties.DOUBLE_BLOCK_HALF,net.minecraft.world.level.block.state.properties.DoubleBlockHalf.UPPER);
            level.setBlockAndUpdate(doorway,lower);level.setBlockAndUpdate(doorway.above(),upper);
            double closedZ=walkProbe(p,Vec3.atBottomCenterOf(probeStart),false);
            level.setBlockAndUpdate(doorway,lower.setValue(BlockStateProperties.OPEN,true));
            level.setBlockAndUpdate(doorway.above(),upper.setValue(BlockStateProperties.OPEN,true));
            double openZ=walkProbe(p,Vec3.atBottomCenterOf(probeStart),false);
            report.addProperty("native_closed_door_blocks",closedZ<3.1);
            report.addProperty("native_open_door_passes",openZ>4);
            // No floor at this height: pressing the vertical wall must not ground Jak.
            level.setBlockAndUpdate(doorway,Blocks.STONE.defaultBlockState());
            level.setBlockAndUpdate(doorway.above(),Blocks.STONE.defaultBlockState());
            double fall=walkProbe(p,Vec3.atBottomCenterOf(probeStart).add(0,3,0),true);
            report.addProperty("vertical_wall_cannot_support_feet",fall<-2.8);
            dev.jakmc.JakMod.LOG.info("Native door audit closedZ={}, openZ={}, wallFallY={}",closedZ,openZ,fall);
            for(int x=-1;x<=1;x++)for(int h=2;h<6;h++)level.setBlockAndUpdate(doorway.offset(x,h,0),Blocks.AIR.defaultBlockState());
            var ledge=ledgeProbe(p,Vec3.atBottomCenterOf(probeStart));
            report.addProperty("two_block_ledge_caught",ledge[0]);
            report.addProperty("two_block_ledge_climbed",ledge[1]);
            int ownedBefore=p.getInventory().countItem(JakItems.BOARD);
            take=()->{
                int justBefore=p.getInventory().countItem(JakItems.BOARD);
                boolean hadBoard=p.getAttachedOrCreate(JakData.BOARD).is(JakItems.BOARD);
                boolean used=interact(p,pos);
                dev.jakmc.JakMod.LOG.info("Altar transfer audit: inventory {} -> {}, attached before={}, captured={}",justBefore,p.getInventory().countItem(JakItems.BOARD),hadBoard,ownedBefore);
                boolean took=used&&level.getBlockState(pos).is(empty)&&p.getAttachedOrCreate(JakData.BOARD).is(JakItems.BOARD);
                boolean preserved=hadBoard&&p.getInventory().countItem(JakItems.BOARD)==justBefore+1;
                client.execute(()->{
                    report.addProperty("equipped_directly",took);
                    report.addProperty("existing_board_preserved",preserved);
                });
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
                for(int i=0;i<inventory.size();i++)p.getInventory().setItem(i,inventory.get(i));
                p.setGameMode(mode);
                p.teleportTo(before.x,before.y,before.z);
            };
            Vec3 gateTarget=Vec3.atBottomCenterOf(first).add(0,1.65,-.5);
            Vec3 gateEye=gateTarget.add(4,1,-2);
            gateView=cameraAt(gateEye,gateTarget);
            Vec3 target=Vec3.atBottomCenterOf(pos).add(-.5,.85,0);
            Vec3 eye=target.add(3,2.1,-4.1);
            client.execute(()->{
                view=cameraAt(eye,target);
                checks=report;
            });
        });
    }

    private static boolean[] ledgeProbe(ServerPlayer player,Vec3 origin){
        long handle=dev.jakmc.nativebridge.NativeJak.create(0);boolean caught=false,climbed=false;int grabbed=-1;
        try{
            dev.jakmc.nativebridge.NativeJak.bindWorld(handle,new dev.jakmc.nativebridge.MinecraftCollision.Query(player.level(),player,origin));
            var input=dev.jakmc.nativebridge.NativeJak.buffer(64);
            input.putFloat(44,1).putInt(48,32).putInt(60,-1);
            var empty=dev.jakmc.nativebridge.NativeJak.buffer(0);
            // Start falling beside the real two-block lip, with the hands just
            // above it. This tests collision/ledge transfer without depending
            // on an arbitrary running-jump button schedule.
            dev.jakmc.nativebridge.NativeJak.relocate(handle,0,1.35f,2.18f,0,-.5f,0);
            var states=new java.util.TreeSet<String>();
            for(int i=0;i<120;i++){
                boolean jump=grabbed>=0&&i==grabbed+8;
                input.putInt(0,jump?1:0).putInt(4,jump?1:0).putFloat(12,i>=10?1:0);
                var pose=com.google.gson.JsonParser.parseString(dev.jakmc.nativebridge.NativeJak.step(handle,input,empty,empty,3)).getAsJsonObject();
                String state=pose.get("state").getAsString();states.add(state);
                if(state.equals("edge-grab")){caught=true;if(grabbed<0)grabbed=i;}
                var pos=pose.getAsJsonArray("pos");
                if(caught&&pos.get(1).getAsDouble()>1.99&&pos.get(2).getAsDouble()>3.25)climbed=true;
            }
            dev.jakmc.JakMod.LOG.info("Native ledge audit caught={}, climbed={}, states={}",caught,climbed,states);
            return new boolean[]{caught,climbed};
        }finally{dev.jakmc.nativebridge.NativeJak.destroy(handle);}
    }

    private static double walkProbe(ServerPlayer player,Vec3 origin,boolean height){
        long handle=dev.jakmc.nativebridge.NativeJak.create(0);
        try{
            dev.jakmc.nativebridge.NativeJak.bindWorld(handle,new dev.jakmc.nativebridge.MinecraftCollision.Query(player.level(),player,origin));
            var input=dev.jakmc.nativebridge.NativeJak.buffer(64);
            input.putFloat(12,1).putFloat(44,1).putInt(48,32).putInt(60,-1);
            var empty=dev.jakmc.nativebridge.NativeJak.buffer(0);JsonObject result=null;
            for(int i=0;i<65;i++)result=com.google.gson.JsonParser.parseString(
                dev.jakmc.nativebridge.NativeJak.step(handle,input,empty,empty,3)).getAsJsonObject();
            return result.getAsJsonArray("pos").get(height?1:2).getAsDouble();
        }finally{dev.jakmc.nativebridge.NativeJak.destroy(handle);}
    }

    private static boolean passage(net.minecraft.server.level.ServerLevel level,BlockPos base){
        var center=new net.minecraft.world.phys.AABB(.15,0,.15,.85,1,.85);
        for(int y=0;y<3;y++){
            var pos=base.above(y);
            if(level.getBlockState(pos).getCollisionShape(level,pos).toAabbs().stream().anyMatch(box->box.intersects(center)))
                return false;
        }
        return true;
    }

    private static InputScene.CameraView cameraAt(Vec3 eye,Vec3 target){
        Vec3 delta=target.subtract(eye);
        return new InputScene.CameraView(eye,(float)Math.toDegrees(Math.atan2(-delta.x,delta.z)),
            (float)-Math.toDegrees(Math.atan2(delta.y,delta.horizontalDistance())));
    }

    private static boolean place(ServerPlayer player,BlockPos floor){
        var hit=new BlockHitResult(Vec3.atCenterOf(floor).add(0,.5,0),Direction.UP,floor,false);
        return player.getMainHandItem().getItem().useOn(new UseOnContext(player,InteractionHand.MAIN_HAND,hit)).consumesAction();
    }

    private static boolean interact(ServerPlayer player,BlockPos pos){
        var hit=new BlockHitResult(Vec3.atCenterOf(pos),Direction.UP,pos,false);
        return player.level().getBlockState(pos).useWithoutItem(player.level(),player,hit).consumesAction();
    }
}
