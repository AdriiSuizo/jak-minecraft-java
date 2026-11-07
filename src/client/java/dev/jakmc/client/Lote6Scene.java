package dev.jakmc.client;

import com.google.gson.JsonArray;
import com.google.gson.JsonObject;
import dev.jakmc.JakData;
import dev.jakmc.client.camera.JakCameraCutout;
import dev.jakmc.content.JakAmmoCrate;
import dev.jakmc.content.JakHutLamp;
import dev.jakmc.content.JakIrisDoor;
import dev.jakmc.player.JakPlayers;
import dev.jakmc.player.JakWorldDamage;
import java.io.IOException;
import java.io.UncheckedIOException;
import java.nio.file.Files;
import java.util.ArrayList;
import java.util.LinkedHashMap;
import java.util.UUID;
import net.fabricmc.fabric.api.client.event.lifecycle.v1.ClientTickEvents;
import net.minecraft.client.CameraType;
import net.minecraft.client.Minecraft;
import net.minecraft.client.Screenshot;
import net.minecraft.core.BlockPos;
import net.minecraft.core.Direction;
import net.minecraft.core.registries.BuiltInRegistries;
import net.minecraft.server.level.ServerLevel;
import net.minecraft.server.level.ServerPlayer;
import net.minecraft.world.InteractionHand;
import net.minecraft.world.entity.EntitySpawnReason;
import net.minecraft.world.entity.EntityTypes;
import net.minecraft.world.entity.LivingEntity;
import net.minecraft.world.entity.item.ItemEntity;
import net.minecraft.world.item.ItemStack;
import net.minecraft.world.item.context.UseOnContext;
import net.minecraft.world.level.GameType;
import net.minecraft.world.level.block.Blocks;
import net.minecraft.world.level.block.state.BlockState;
import net.minecraft.world.level.levelgen.Heightmap;
import net.minecraft.world.phys.BlockHitResult;
import net.minecraft.world.phys.Vec3;

/**
 * Adrii's 07/11 batch: the snow barrels and the hanging lamp beside Jak, his shine close to, the spin kick on a ring of
 * blocks and a creature behind him with the crosshair elsewhere, a crate and an object breaking, the camera with
 * its eye in rock (Jak in a tunnel, and Jak in the open with a hillside behind him), and Haven's airlocks.
 */
final class Lote6Scene {
    private static boolean running,pause;
    private static int tick;
    private static UUID owner;
    private static BlockPos base,haven;
    private static InputScene.CameraView view;
    private static Runnable restore;
    private static final JsonObject result=new JsonObject();
    private static final int CIRCLE=4;
    private static LivingEntity pig;
    private static float yaw,pitch;
    private static boolean steer;
    private record Door(BlockPos at,Direction facing){}
    private static Door doorSeen;
    static InputScene.CameraView camera(){return view;}
    static void initialize(){ClientTickEvents.END_CLIENT_TICK.register(Lote6Scene::tick);}

    private static Vec3 at(double x,double y,double z){return Vec3.atBottomCenterOf(base).add(x,y,z);}
    private static void look(Vec3 eye,Vec3 target){
        var d=target.subtract(eye);
        view=new InputScene.CameraView(eye,(float)Math.toDegrees(Math.atan2(-d.x,d.z)),(float)-Math.toDegrees(Math.atan2(d.y,d.horizontalDistance())));
    }
    private static void capture(Minecraft c,String file){Screenshot.grab(c.gameDirectory,file,c.gameRenderer.mainRenderTarget(),1,_->{});}
    private static void put(ServerPlayer p,double x,double y,double z,float facing){var s=at(x,y,z);p.teleportTo(s.x,s.y,s.z);p.setYRot(facing);p.setXRot(0);p.setDeltaMovement(Vec3.ZERO);}
    private static void place(ServerPlayer p,String id,int x,int z){
        var item=BuiltInRegistries.ITEM.getValue(net.minecraft.resources.Identifier.parse(id));
        p.setItemInHand(InteractionHand.MAIN_HAND,new ItemStack(item));
        var below=base.offset(x,-1,z);
        var hit=new BlockHitResult(Vec3.atCenterOf(below).add(0,.5,0),Direction.UP,below,false);
        result.addProperty("placed_"+id.substring(id.indexOf(':')+1),item.useOn(new UseOnContext(p,InteractionHand.MAIN_HAND,hit)).consumesAction());
        p.setItemInHand(InteractionHand.MAIN_HAND,ItemStack.EMPTY);
    }

    static void start(Minecraft client){
        if(client.player==null||client.getSingleplayerServer()==null)throw new IllegalStateException("Open the development world");
        owner=client.player.getUUID();tick=0;result.entrySet().clear();haven=null;doorSeen=null;steer=false;pig=null;
        pause=client.options.pauseOnLostFocus;client.options.pauseOnLostFocus=false;
        client.options.setCameraType(CameraType.THIRD_PERSON_BACK);
        onServer(client,p->{
            var server=p.level().getServer();
            server.getCommands().performPrefixedCommand(server.createCommandSourceStack(),"time set noon");
            server.getCommands().performPrefixedCommand(server.createCommandSourceStack(),"weather clear");
            var level=p.level();
            var old=p.position();var mode=p.gameMode.getGameModeForPlayer();
            boolean enabled=p.getAttachedOrCreate(JakData.ENABLED);
            var saved=new ArrayList<ItemStack>();
            for(int i=0;i<p.getInventory().getContainerSize();i++)saved.add(p.getInventory().getItem(i).copy());
            var near=p.blockPosition().offset(24,0,0);
            int y=near.getY();
            for(int x=-14;x<=46;x+=4)for(int z=-14;z<=14;z+=4)y=Math.max(y,level.getHeight(Heightmap.Types.MOTION_BLOCKING_NO_LEAVES,near.getX()+x,near.getZ()+z));
            base=near.atY(y+30);
            var snapshot=new LinkedHashMap<BlockPos,BlockState>();
            // The grass floor, and past it a block of rock with a tunnel through it, three wide and three tall.
            for(int x=-12;x<=46;x++)for(int z=-14;z<=14;z++)for(int h=-8;h<=12;h++){
                var cell=base.offset(x,h,z);
                snapshot.put(cell.immutable(),level.getBlockState(cell));
                BlockState state=Blocks.AIR.defaultBlockState();
                if(h==-1&&x<20)state=Blocks.GRASS_BLOCK.defaultBlockState();
                if(x>=20&&x<=44&&h>=-8&&h<=10&&!(x>=31&&x<=33&&h>=0&&h<=2))state=Blocks.STONE.defaultBlockState();
                level.setBlock(cell,state,2);
            }
            p.setGameMode(GameType.SURVIVAL);p.setAttached(JakData.ENABLED,true);
            p.getInventory().clearContent();
            p.getInventory().setSelectedSlot(5);
            place(p,"jak:objeto_montana_barril_1",4,-4);
            place(p,"jak:objeto_montana_barril_3",-4,-4);
            // The lamp hung from a block.
            level.setBlock(base.offset(0,4,-4),Blocks.OAK_PLANKS.defaultBlockState(),2);
            level.setBlock(base.offset(0,3,-4),JakHutLamp.BLOCK.defaultBlockState(),2);
            put(p,0,0,0,180);
            JakPlayers.controlForProbe(p,true);JakPlayers.hostControlsForProbe(p);
            restore=()->{
                for(var item:level.getEntitiesOfClass(ItemEntity.class,new net.minecraft.world.phys.AABB(base).inflate(14)))item.discard();
                if(pig!=null)pig.discard();
                snapshot.forEach((cell,state)->level.setBlock(cell,state,2));
                p.getInventory().clearContent();
                for(int k=0;k<saved.size();k++)p.getInventory().setItem(k,saved.get(k));
                JakPlayers.controlForProbe(p,false);
                p.setAttached(JakData.ENABLED,enabled);
                p.setGameMode(mode);p.teleportTo(old.x,old.y,old.z);
            };
            client.execute(()->running=true);
        });
    }
    /** Every Jak block whose break particles would be the missing texture. */
    private static void audit(Minecraft c){
        var models=c.getModelManager().getBlockStateModelSet();
        var list=new JsonArray();int checked=0;
        for(var block:BuiltInRegistries.BLOCK){
            var id=BuiltInRegistries.BLOCK.getKey(block);
            if(!id.getNamespace().equals("jak")&&!id.getNamespace().equals("jakmc"))continue;
            checked++;
            if(models.getParticleMaterial(block.defaultBlockState()).sprite().contents().name().getPath().equals("missingno"))list.add(id.toString());
        }
        result.addProperty("particle_blocks_checked",checked);result.add("particle_missing",list);
    }
    private static String ring(ServerLevel level){
        var out=new StringBuilder();
        for(int[] cell:new int[][]{{1,0,0},{1,1,0},{-1,0,0},{0,0,1},{0,1,-1},{0,-1,0}}){
            var pos=base.offset(cell[0],cell[1],cell[2]);
            out.append(cell[0]).append(',').append(cell[1]).append(',').append(cell[2]).append('=').append(BuiltInRegistries.BLOCK.getKey(level.getBlockState(pos).getBlock()).getPath()).append(' ');
        }
        return out.toString();
    }
    /** Haven's doors in the loaded town: airlocks by what floor they stand on, and the gates left. */
    private static void doors(ServerLevel level){
        var floors=new java.util.TreeMap<String,Integer>();int anchors=0,gates=0,chunks=0,benches=0;
        var centre=new net.minecraft.world.level.ChunkPos(haven.getX()>>4,haven.getZ()>>4);
        for(int cx=-10;cx<=10;cx++)for(int cz=-10;cz<=10;cz++){
            int x=centre.x()+cx,z=centre.z()+cz;
            if(!(level.getChunkSource().getChunkNow(x,z) instanceof net.minecraft.world.level.chunk.LevelChunk chunk))continue;
            chunks++;
            for(int bx=0;bx<16;bx++)for(int bz=0;bz<16;bz++)for(int y=haven.getY()-50;y<haven.getY()+170;y++){
                var cell=new BlockPos(chunk.getPos().getMinBlockX()+bx,y,chunk.getPos().getMinBlockZ()+bz);
                var here=chunk.getBlockState(cell);
                if(here.isAir())continue;
                if(here.getBlock() instanceof dev.jakmc.content.JakKrimsonGate)gates++;
                if(here.getBlock()!=JakIrisDoor.AIRLOCK||here.getValue(JakIrisDoor.PART)!=JakIrisDoor.ANCHOR)continue;
                anchors++;
                floors.merge(BuiltInRegistries.BLOCK.getKey(level.getBlockState(cell.below()).getBlock()).getPath(),1,Integer::sum);
                if(doorSeen==null||cell.getY()<doorSeen.at.getY())doorSeen=new Door(cell,here.getValue(JakIrisDoor.FACING));
            }
        }
        result.addProperty("haven_chunks",chunks);result.addProperty("haven_airlocks",anchors);result.addProperty("haven_gates_left",gates);result.addProperty("haven_airlock_floors",floors.toString());
    }

    private static void tick(Minecraft c){
        if(!running)return;
        int t=++tick;
        c.gui.setScreen(null);
        if(steer){c.player.setYRot(yaw);c.player.setXRot(pitch);}
        // The objects, and Jak close to.
        if(t==20)audit(c);
        if(t==24)look(at(0,1.8,3),at(0,1.2,-3));
        if(t==40)capture(c,"lote6-objects.png");
        if(t==42)look(at(.9,1.6,-2.2),at(0,2.5,-4));
        if(t==52)capture(c,"lote6-lamp-a.png");
        if(t==64)capture(c,"lote6-lamp-b.png");
        if(t==66)look(at(.3,1.5,-1.9),at(0,1.1,0));
        if(t==80)capture(c,"lote6-jak-front.png");
        if(t==82)look(at(1.5,1.7,1.5),at(0,1.2,0));
        if(t==94)capture(c,"lote6-jak-back.png");
        // The spin kick, the crosshair off it all: a ring of blocks and a pig.
        if(t==96)onServer(c,p->{
            var level=p.level();
            level.setBlock(base.offset(1,0,0),Blocks.DIRT.defaultBlockState(),2);level.setBlock(base.offset(1,1,0),Blocks.DIRT.defaultBlockState(),2);
            level.setBlock(base.offset(-1,0,0),Blocks.STONE.defaultBlockState(),2);level.setBlock(base.offset(0,0,1),Blocks.OAK_LOG.defaultBlockState(),2);
            level.setBlock(base.offset(0,1,-1),Blocks.DIRT.defaultBlockState(),2);
            var made=EntityTypes.PIG.create(level,EntitySpawnReason.COMMAND);
            if(made!=null){var s=at(-1.05,0,1.05);made.snapTo(s.x,s.y,s.z,0,0);made.setNoAi(true);level.addFreshEntity(made);pig=made;}
            p.setXRot(-70);
            result.addProperty("ring_before",ring(level));
        });
        if(t==100){view=null;look(at(3,3,3),at(0,.5,0));steer=true;yaw=180;pitch=-70;}
        if(t==104)onServer(c,p->{result.addProperty("pig_before",pig==null?-1:pig.getHealth());JakPlayers.probeInput(p,CIRCLE,CIRCLE,0);});
        if(t==106)onServer(c,p->JakPlayers.probeInput(p,0,0,0));
        if(t==116)capture(c,"lote6-kick1.png");
        if(t==150)onServer(c,p->{result.addProperty("ring_after_kick_1",ring(p.level()));result.addProperty("pig_after_kick_1",pig==null?-1:pig.getHealth());JakPlayers.probeInput(p,CIRCLE,CIRCLE,0);});
        if(t==152)onServer(c,p->JakPlayers.probeInput(p,0,0,0));
        if(t>=153&&t<=171&&t%6==3)capture(c,"lote6-kick2-"+t+".png");
        if(t==196)onServer(c,p->{result.addProperty("ring_after_kick_2",ring(p.level()));
            // A crate, and a barrel to break.
            p.level().setBlock(base.offset(1,0,0),JakAmmoCrate.BLOCK.defaultBlockState(),2);});
        if(t==204)onServer(c,p->JakPlayers.probeInput(p,CIRCLE,CIRCLE,0));
        if(t==206)onServer(c,p->JakPlayers.probeInput(p,0,0,0));
        if(t>=207&&t<=225&&t%3==0)capture(c,"lote6-crate-"+t+".png");
        if(t==240){look(at(2,2,-1),at(4.5,.8,-4));onServer(c,p->JakWorldDamage.hit(p,base.offset(5,0,-4),1e5f));}
        if(t==241)onServer(c,p->{var pos=BlockPos.containing(at(4,0,-4));JakWorldDamage.hit(p,pos,1e5f);
            int left=0;for(var cell:BlockPos.betweenClosed(base.offset(2,0,-6),base.offset(6,3,-2)))if(!p.level().getBlockState(cell).isAir())left++;
            result.addProperty("barrel_cells_left",left);});
        if(t==244)capture(c,"lote6-barrel-break.png");
        // The camera in rock: Jak in the tunnel, the view line going into the wall.
        if(t==260)onServer(c,p->{view=null;put(p,32,0,0,-90);});
        if(t==262){view=null;yaw=-90;pitch=12;}
        if(t==300){result.addProperty("tunnel_buried",JakCameraCutout.buried());capture(c,"lote6-camera-tunnel.png");}
        if(t==302)yaw=-40;
        if(t==330){result.addProperty("tunnel2_buried",JakCameraCutout.buried());capture(c,"lote6-camera-tunnel2.png");}
        if(t==332){yaw=-90;pitch=40;}
        if(t==360){result.addProperty("tunnel3_buried",JakCameraCutout.buried());capture(c,"lote6-camera-tunnel3.png");}
        // In the open, the block of rock behind him.
        if(t==362){onServer(c,p->put(p,18.5,0,0,90));yaw=90;pitch=12;}
        if(t==400){result.addProperty("open_buried",JakCameraCutout.buried());capture(c,"lote6-camera-open.png");}
        // Haven: its airlocks as the town generates round him.
        if(t==402){steer=false;onServer(c,p->{
            p.setGameMode(GameType.CREATIVE);JakPlayers.probeInput(p,0,0,0);
            var level=p.level();
            var key=net.minecraft.resources.ResourceKey.create(net.minecraft.core.registries.Registries.STRUCTURE,dev.jakmc.JakData.id("haven"));
            var type=level.registryAccess().lookupOrThrow(net.minecraft.core.registries.Registries.STRUCTURE).getOrThrow(key);
            var pos=level.findNearestMapStructure(net.minecraft.core.HolderSet.direct(type),p.blockPosition(),96,false);
            if(pos==null){result.addProperty("haven","not found");return;}
            var box=level.getChunk(pos).getStartForStructure(type.value()).getBoundingBox();
            haven=new BlockPos(pos.getX(),box.minY()+42,pos.getZ());
            p.teleportTo(haven.getX()+.5,haven.getY()+30,haven.getZ()+.5);
        });}
        if(t==840&&haven!=null)onServer(c,p->doors(p.level()));
        if(t==850&&doorSeen!=null){var side=doorSeen.facing;var middle=Vec3.atBottomCenterOf(doorSeen.at);
            look(middle.add(side.getStepX()*5,2.2,side.getStepZ()*5),middle.add(0,1.5,0));}
        if(t==865&&doorSeen!=null)capture(c,"lote6-haven-airlock.png");
        if(t==870){
            view=null;
            result.addProperty("visual_review","required");result.addProperty("reason","complete");
            onServer(c,p->{
                try{
                    var dir=c.gameDirectory.toPath().resolve("jakmc/evidence/lote6");
                    Files.createDirectories(dir);Files.writeString(dir.resolve("result.json"),result.toString());
                }catch(IOException failure){throw new UncheckedIOException(failure);}
                restore.run();
                c.execute(()->{running=false;view=null;c.options.pauseOnLostFocus=pause;c.stop();});
            });
        }
    }
    private static void onServer(Minecraft client,java.util.function.Consumer<ServerPlayer> action){
        var server=client.getSingleplayerServer();
        server.execute(()->{var player=server.getPlayerList().getPlayer(owner);if(player!=null)action.accept(player);});
    }
}
