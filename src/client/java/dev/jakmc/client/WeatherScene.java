package dev.jakmc.client;

import com.google.gson.JsonObject;
import dev.jakmc.JakData;
import java.io.IOException;
import java.io.UncheckedIOException;
import java.nio.file.Files;
import net.fabricmc.fabric.api.client.event.lifecycle.v1.ClientTickEvents;
import net.minecraft.client.Minecraft;
import net.minecraft.client.Screenshot;
import net.minecraft.core.BlockPos;
import net.minecraft.world.level.GameType;
import net.minecraft.world.level.levelgen.Heightmap;

final class WeatherScene {
    private static boolean running,pause;
    private static int tick,peak;
    private static float clearFog,stormFog;
    private static Runnable restore;
    static void initialize(){ClientTickEvents.END_CLIENT_TICK.register(WeatherScene::tick);}
    static void start(Minecraft client){
        var server=client.getSingleplayerServer();
        var owner=client.player.getUUID();
        pause=client.options.pauseOnLostFocus;
        client.options.pauseOnLostFocus=false;
        server.execute(()->{
            var p=server.getPlayerList().getPlayer(owner);
            if(p==null)return;
            var level=p.level();
            var data=server.getWeatherData();
            var old=p.position();var mode=p.gameMode.getGameModeForPlayer();
            float yaw=p.getYRot(),pitch=p.getXRot();
            boolean enabled=p.getAttachedOrCreate(JakData.ENABLED);
            boolean raining=data.isRaining(),thundering=data.isThundering();
            int rainTime=data.getRainTime(),thunderTime=data.getThunderTime(),clearTime=data.getClearWeatherTime();
            int x=-576,z=192;
            int y=level.getHeight(Heightmap.Types.MOTION_BLOCKING_NO_LEAVES,x,z);
            if(y<=level.getSeaLevel())y=level.getChunk(x>>4,z>>4).getHeight(Heightmap.Types.MOTION_BLOCKING_NO_LEAVES,x&15,z&15)+1;
            p.setAttached(JakData.ENABLED,false);
            p.setGameMode(GameType.CREATIVE);
            p.getAbilities().flying=true;p.onUpdateAbilities();
            p.teleportTo(x+.5,y+8,z+.5);
            p.setYRot(135);p.setXRot(12);
            server.setWeatherParameters(2000,0,false,false);
            restore=()->{
                p.teleportTo(old.x,old.y,old.z);p.setYRot(yaw);p.setXRot(pitch);
                p.setAttached(JakData.ENABLED,enabled);p.setGameMode(mode);
                data.setClearWeatherTime(clearTime);data.setRainTime(rainTime);data.setThunderTime(thunderTime);
                data.setRaining(raining);data.setThundering(thundering);
            };
            client.execute(()->{
                client.options.setCameraType(net.minecraft.client.CameraType.FIRST_PERSON);
                client.player.setYRot(135);client.player.setXRot(12);
                tick=0;peak=0;running=true;
            });
        });
    }
    private static void tick(Minecraft client){
        if(!running)return;
        int frame=++tick;
        var server=client.getSingleplayerServer();
        peak=Math.max(peak,JakEffects.drawnSandVertices);
        if(frame==85){
            clearFog=JakAtmosphere.distanceScale();
            capture(client,"weather-clear.png");
            server.execute(()->server.setWeatherParameters(0,2000,true,false));
        }
        if(frame==225){
            stormFog=JakAtmosphere.distanceScale();
            capture(client,"weather-storm.png");
        }
        if(frame==230)server.execute(()->server.setWeatherParameters(2000,0,false,false));
        if(frame==365){
            boolean desert=client.level.getBiome(BlockPos.containing(client.player.position())).unwrapKey()
                .orElseThrow().identifier().getPath().contains("desert");
            var result=new JsonObject();
            result.addProperty("desert",desert);
            result.addProperty("clearDistanceScale",clearFog);
            result.addProperty("stormDistanceScale",stormFog);
            result.addProperty("restoredDistanceScale",JakAtmosphere.distanceScale());
            result.addProperty("sandVertices",peak);
            result.addProperty("reason",desert&&clearFog>.65f&&stormFog<.25f&&peak>0
                &&JakAtmosphere.distanceScale()>.65f?"complete":"Weather acceptance failed");
            try{
                var dir=client.gameDirectory.toPath().resolve("jakmc/evidence/weather");
                Files.createDirectories(dir);Files.writeString(dir.resolve("result.json"),result.toString());
            }catch(IOException failure){throw new UncheckedIOException(failure);}
            running=false;client.options.pauseOnLostFocus=pause;
            server.execute(()->{restore.run();client.execute(client::stop);});
        }
    }
    private static void capture(Minecraft client,String name){
        Screenshot.grab(client.gameDirectory,name,client.gameRenderer.mainRenderTarget(),1,_-> {});
    }
}
