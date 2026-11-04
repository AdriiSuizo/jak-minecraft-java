package dev.jakmc.client;

import com.google.gson.*;
import dev.jakmc.*;
import dev.jakmc.client.model.JakRenderer;
import dev.jakmc.player.JakPlayers;
import java.nio.file.*;
import net.fabricmc.fabric.api.client.event.lifecycle.v1.ClientTickEvents;
import net.minecraft.client.*;
import net.minecraft.core.BlockPos;
import net.minecraft.world.level.block.Blocks;

/** One deterministic scene, actor-visible acceptance and a real two-block ceiling. */
public final class ScaleScene {
    private static boolean waiting;
    private static int frames;
    private static long lastRenderSerial,lastCinematicSerial;
    private static Path output;
    private static InputScene.CameraView view;
    static InputScene.CameraView camera(){return view;}
    public static boolean active(){return waiting;}
    public static void start(Minecraft client){
        if(client.player==null||client.getSingleplayerServer()==null)throw new IllegalStateException("Scale scene needs the existing normal dev world");
        if(waiting)throw new IllegalStateException("Scene already active");
        output=client.gameDirectory.toPath().resolve("jakmc/evidence/scale");
        try{Files.createDirectories(output);}catch(Exception e){throw new RuntimeException(e);}
        var uuid=client.player.getUUID();
        client.getSingleplayerServer().execute(()->{
            var p=client.getSingleplayerServer().getPlayerList().getPlayer(uuid);if(p==null)return;
            var server=client.getSingleplayerServer();
            server.getCommands().performPrefixedCommand(server.createCommandSourceStack(),"time set noon");
            server.getCommands().performPrefixedCommand(server.createCommandSourceStack(),"weather clear");
            BlockPos at=p.blockPosition();
            // Reuse this scene's location on rerun; no new world and no player save replacement.
            int x=at.getX(),y=at.getY(),z=at.getZ();
            for(int dx=-4;dx<=4;dx++)for(int dz=-4;dz<=4;dz++){
                p.level().setBlock(new BlockPos(x+dx,y-1,z+dz),Blocks.SMOOTH_STONE.defaultBlockState(),3);
                for(int dy=0;dy<=4;dy++)p.level().setBlock(new BlockPos(x+dx,y+dy,z+dz),Blocks.AIR.defaultBlockState(),3);
            }
            // Underside is exactly two blocks above Jak's feet.
            for(int dx=-1;dx<=1;dx++)for(int dz=-1;dz<=1;dz++)p.level().setBlock(new BlockPos(x+dx,y+2,z+dz),Blocks.SMOOTH_STONE.defaultBlockState(),3);
            p.teleportTo(x+.5,y,z+.5);p.setDeltaMovement(net.minecraft.world.phys.Vec3.ZERO);
            p.setAttached(JakData.ENABLED,true);JakPlayers.controlForProbe(p,true);
        });
        client.gui.setScreen(null);client.options.setCameraType(CameraType.THIRD_PERSON_FRONT);client.player.setXRot(0);
        frames=0;view=null;lastRenderSerial=JakRenderer.renderSerial;lastCinematicSerial=JakRenderer.cinematicRenderSerial;waiting=true;
    }
    public static void initialize(){ClientTickEvents.END_CLIENT_TICK.register(ScaleScene::tick);}
    private static void tick(Minecraft client){
        if(!waiting)return;
        if(client.player==null){waiting=false;return;}
        ++frames;
        if(frames==26)client.gui.hud.getChat().clearMessages(false);
        if(frames<30)return;
        var pose=JakPoses.get(client.player.getId());
        if(frames==32&&pose!=null){
            var head=pose.position().add(0,1.56,0);
            var outward=net.minecraft.world.phys.Vec3.directionFromRotation(0,pose.yaw());
            var eye=head.add(outward.scale(1.20)).add(0,.03,0);
            var delta=head.subtract(eye);
            view=new InputScene.CameraView(eye,(float)Math.toDegrees(Math.atan2(-delta.x,delta.z)),
                (float)-Math.toDegrees(Math.atan2(delta.y,delta.horizontalDistance())));
        }
        if(frames>=38){
            Screenshot.grab(client.gameDirectory,"scale-forward-eyes.png",client.gameRenderer.mainRenderTarget(),1,
                message->client.execute(client::stop));
            var uuid=client.player.getUUID();
            client.getSingleplayerServer().execute(()->{
                var p=client.getSingleplayerServer().getPlayerList().getPlayer(uuid);
                if(p!=null)JakPlayers.controlForProbe(p,false);
            });
            view=null;
            waiting=false;
            return;
        }
        if(frames!=30)return;
        boolean drawn=JakRenderer.lastEntityId==client.player.getId()&&JakRenderer.renderSerial>lastRenderSerial;
        boolean cinematic=JakRenderer.cinematicRenderSerial>lastCinematicSerial;
        var result=new JsonObject();result.addProperty("actor_id",client.player.getId());result.addProperty("actor_drawn",drawn);
        result.addProperty("camera",client.options.getCameraType().name());
        result.addProperty("clip",pose==null?null:pose.clip());result.addProperty("ears_standing_blocks",JakRenderer.standingHeight);
        var roof=client.player.blockPosition().above(2);
        double ceiling=roof.getY()-client.player.getY();
        boolean roofPresent=client.level.getBlockState(roof).is(Blocks.SMOOTH_STONE);
        double clearance=ceiling-JakRenderer.currentTop;
        result.addProperty("actual_animated_top",JakRenderer.currentTop);result.addProperty("ceiling_blocks",ceiling);result.addProperty("roof_present",roofPresent);result.addProperty("clearance",clearance);
        result.addProperty("cinematic_models_drawn",cinematic);
        boolean pass=drawn&&cinematic&&pose!=null&&pose.clip().contains("stance")&&roofPresent&&clearance>=0&&clearance<=.12&&client.options.getCameraType()==CameraType.THIRD_PERSON_FRONT;
        result.addProperty("mechanical_acceptance",pass);result.addProperty("visual_review","required");
        try{Files.writeString(output.resolve("result.json"),result.toString());}catch(Exception e){throw new RuntimeException(e);}
        Screenshot.grab(client.gameDirectory,"scale-two-block-ceiling.png",client.gameRenderer.mainRenderTarget(),1,
            message->JakMod.LOG.info("Scale scene screenshot: {}",message.getString()));
        JakMod.LOG.info("Scale scene actor-visible result: {}",result);
    }
}
