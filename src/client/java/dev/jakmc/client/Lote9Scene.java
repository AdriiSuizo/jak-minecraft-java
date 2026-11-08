package dev.jakmc.client;

import com.google.gson.JsonObject;
import dev.jakmc.JakData;
import dev.jakmc.content.JakItems;
import dev.jakmc.player.JakPlayers;
import java.io.IOException;
import java.io.UncheckedIOException;
import java.nio.file.Files;
import java.util.UUID;
import net.fabricmc.fabric.api.client.event.lifecycle.v1.ClientTickEvents;
import net.minecraft.client.CameraType;
import net.minecraft.client.Minecraft;
import net.minecraft.client.Screenshot;
import net.minecraft.core.BlockPos;
import net.minecraft.server.level.ServerPlayer;
import net.minecraft.world.item.ItemStack;
import net.minecraft.world.level.GameType;
import net.minecraft.world.level.block.Blocks;
import net.minecraft.world.level.levelgen.Heightmap;
import net.minecraft.world.phys.Vec3;

/**
 * Adrii 08/11: Jak's own effects were gone (the splash, the bubbles, the board's Circle). One short run: into the
 * water, a dive, the board on the water with Circle, counting what each part drew of his own effects.
 */
final class Lote9Scene {
    private static boolean running,pause;
    private static int tick;
    private static UUID owner;
    private static BlockPos base;
    private static volatile InputScene.CameraView view;
    private static final JsonObject result=new JsonObject();
    private static String part="setup";
    static InputScene.CameraView camera(){return view;}
    static void initialize(){ClientTickEvents.END_CLIENT_TICK.register(Lote9Scene::tick);}
    private static Vec3 at(double x,double y,double z){return Vec3.atBottomCenterOf(base).add(x,y,z);}
    private static void look(Vec3 eye,Vec3 target){
        var d=target.subtract(eye);
        view=new InputScene.CameraView(eye,(float)Math.toDegrees(Math.atan2(-d.x,d.z)),(float)-Math.toDegrees(Math.atan2(d.y,d.horizontalDistance())));
    }
    private static void capture(Minecraft c,String file){Screenshot.grab(c.gameDirectory,file,c.gameRenderer.mainRenderTarget(),1,_->{});}
    private static void put(ServerPlayer p,double x,double y,double z,float yaw){var s=at(x,y,z);p.teleportTo(s.x,s.y,s.z);p.setYRot(yaw);p.setXRot(0);p.setDeltaMovement(Vec3.ZERO);}
    private static void input(Minecraft c,int held,int pressed,float forward){onServer(c,p->JakPlayers.probeInput(p,held,pressed,forward));}

    static void start(Minecraft client){
        if(client.player==null||client.getSingleplayerServer()==null)throw new IllegalStateException("Open the development world");
        owner=client.player.getUUID();tick=0;result.entrySet().clear();part="setup";view=null;
        pause=client.options.pauseOnLostFocus;client.options.pauseOnLostFocus=false;
        client.options.setCameraType(CameraType.THIRD_PERSON_BACK);
        onServer(client,p->{
            var server=p.level().getServer();
            for(var text:new String[]{"time set noon","weather clear","gamerule spawn_monsters false"})
                server.getCommands().performPrefixedCommand(server.createCommandSourceStack().withSuppressedOutput(),text);
            var level=p.level();var near=p.blockPosition();
            base=near.atY(level.getHeight(Heightmap.Types.MOTION_BLOCKING_NO_LEAVES,near.getX(),near.getZ())+24);
            var stone=Blocks.STONE.defaultBlockState();var air=Blocks.AIR.defaultBlockState();
            for(int x=-6;x<=40;x++)for(int z=-8;z<=12;z++)for(int y=-6;y<=12;y++)level.setBlock(base.offset(x,y,z),y<0?stone:air,2);
            // A long pool, four deep: room for the board to ride on it.
            for(int x=10;x<=34;x++)for(int z=2;z<=10;z++)for(int y=-4;y<=-1;y++)level.setBlock(base.offset(x,y,z),Blocks.WATER.defaultBlockState(),3);
            JakPlayers.chooseMode(p,true);
            p.setGameMode(GameType.SURVIVAL);p.getInventory().clearContent();
            p.setAttached(JakData.BOARD,new ItemStack(JakItems.BOARD));
            put(p,6.5,0,6.5,-90);
            JakPlayers.controlForProbe(p,true);JakPlayers.hostControlsForProbe(p);
        });
        running=true;
    }

    /** The most his own effects drew in one frame of each part, and whether they had an owner. */
    private static void sample(Minecraft c){
        int drawn=JakEffects.drawnVertices;boolean own=JakEffects.hasOwner(c.player.getId());
        var row=result.has(part)?result.getAsJsonObject(part):new JsonObject();
        row.addProperty("max_vertices",Math.max(drawn,row.has("max_vertices")?row.get("max_vertices").getAsInt():0));
        if(own)row.addProperty("own_frames",(row.has("own_frames")?row.get("own_frames").getAsInt():0)+1);
        row.addProperty("held_max",Math.max(JakEffects.heldCount(),row.has("held_max")?row.get("held_max").getAsInt():0));
        result.add(part,row);
    }

    private static void tick(Minecraft c){
        if(!running||c.player==null||c.level==null)return;
        int t=tick++;
        if(t>=20)sample(c);
        // Into the water, walking.
        if(t==20){part="walk_in";input(c,0,0,1);}
        if(t==45)capture(c,"lote9-splash.png");
        if(t==60)input(c,0,0,0);
        // The dive: Square under, held to the bottom, X up.
        if(t==90){part="dive";onServer(c,p->put(p,14.5,-.6,6,-90));}
        if(t==105)input(c,2,2,0);
        if(t==112)input(c,0,0,0);
        if(t==150)input(c,2,0,.8f);
        if(t==190)input(c,1,1,0);
        if(t==192)input(c,1,0,0);
        if(t==205)look(at(10,3,6),at(14.5,1,6));
        if(t==225)capture(c,"lote9-bubbles.png");
        if(t==240){input(c,0,0,0);view=null;}
        // The board: on it (R2), along the water, Circle twice.
        if(t==260){part="board";onServer(c,p->put(p,6.5,0,6.5,-90));}
        if(t==266)input(c,128,128,0);
        if(t==269)input(c,0,0,0);
        if(t>=280&&t<380)input(c,t==310||t==345?4:0,t==310||t==345?4:0,1);
        if(t==316)capture(c,"lote9-board-circle-a.png");
        if(t==351)capture(c,"lote9-board-circle-b.png");
        if(t==380)input(c,0,0,0);
        if(t==400){
            view=null;
            onServer(c,p->{
                result.addProperty("visual_review","required");result.addProperty("reason","complete");
                JakPlayers.controlForProbe(p,false);
                try{
                    var dir=c.gameDirectory.toPath().resolve("jakmc/evidence/lote9");
                    Files.createDirectories(dir);Files.writeString(dir.resolve("result.json"),result.toString());
                }catch(IOException failure){throw new UncheckedIOException(failure);}
                c.execute(()->{running=false;c.options.pauseOnLostFocus=pause;c.stop();});
            });
        }
    }
    private static void onServer(Minecraft client,java.util.function.Consumer<ServerPlayer> action){
        var server=client.getSingleplayerServer();
        server.execute(()->{var player=server.getPlayerList().getPlayer(owner);if(player!=null)action.accept(player);});
    }
}
