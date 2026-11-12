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
    private static float clearFog,stormFog,flashMax;
    private static int stormAt=-1,flashShot=-1,darkShot=-1;
    private static String forest="";
    private static boolean desertSeen;
    private static int sandPeak;
    private static float restoredFog;
    private static float[][] desertGrade,forestGrade;
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
            level.getChunk(x>>4,z>>4);
            int y=level.getHeight(Heightmap.Types.MOTION_BLOCKING,x,z);
            p.setAttached(JakData.ENABLED,false);
            p.setGameMode(GameType.CREATIVE);
            p.getAbilities().flying=true;p.onUpdateAbilities();
            p.teleportTo(x+.5,y+24,z+.5);
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
                tick=0;peak=0;running=true;stormAt=-1;flashShot=-1;darkShot=-1;flashMax=0;forest="";
            });
        });
    }
    private static void tick(Minecraft client){
        if(!running)return;
        int frame=++tick;
        var server=client.getSingleplayerServer();
        peak=Math.max(peak,JakEffects.drawnSandVertices);
        // drawsurf/enhanced_grade.rs: the same view once without the grade.
        if(frame==78)dev.jakmc.client.sky.JakGrade.bypass=true;
        if(frame==82){capture(client,"weather-clear-raw.png");dev.jakmc.client.sky.JakGrade.bypass=false;}
        if(frame==85){
            desertGrade=dev.jakmc.client.sky.JakGrade.current();
            clearFog=JakAtmosphere.distanceScale();
            capture(client,"weather-clear.png");
            server.execute(()->server.setWeatherParameters(0,2000,true,false));
        }
        if(frame==225){
            stormFog=JakAtmosphere.distanceScale();
            capture(client,"weather-storm.png");
        }
        if(frame==230)server.execute(()->server.setWeatherParameters(2000,0,false,false));
        // A forest's night storm: jak_sky.rs lightning (a flash every 3 to 8 s) lighting the world.
        if(frame==365){
            desertSeen=client.level.getBiome(BlockPos.containing(client.player.position())).unwrapKey()
                .orElseThrow().identifier().getPath().contains("desert");
            sandPeak=peak;restoredFog=JakAtmosphere.distanceScale();
            var owner=client.player.getUUID();
            server.execute(()->{
                var p=server.getPlayerList().getPlayer(owner);
                if(p==null)return;
                var level=p.level();
                // A forest at the surface: rings of 48 blocks out to 3 km, every sample of the
                // nine the weather takes round the camera a forest.
                BlockPos at=null;String path="";
                search:
                for(int ring=1;ring<64&&at==null;ring++)for(int k=0;k<8*ring;k++){
                    double angle=Math.PI*2*k/(8*ring);
                    int x=p.getBlockX()+(int)(Math.cos(angle)*48*ring),z=p.getBlockZ()+(int)(Math.sin(angle)*48*ring);
                    boolean all=true;
                    for(int[] o:new int[][]{{0,0},{24,0},{-24,0},{0,24},{0,-24}}){
                        var biome=level.getUncachedNoiseBiome((x+o[0])>>2,72>>2,(z+o[1])>>2).unwrapKey().map(key->key.identifier().getPath()).orElse("");
                        if(!biome.equals("forest")&&!biome.equals("birch_forest")){all=false;break;}
                        path=biome;
                    }
                    if(all){at=new BlockPos(x,0,z);break search;}
                }
                if(at==null)return;
                level.getChunk(at.getX()>>4,at.getZ()>>4);
                int y=level.getHeight(Heightmap.Types.MOTION_BLOCKING,at.getX(),at.getZ());
                p.teleportTo(at.getX()+.5,y+10,at.getZ()+.5);
                p.getAbilities().flying=true;p.onUpdateAbilities();
                p.setYRot(135);p.setXRot(20);
                server.getCommands().performPrefixedCommand(server.createCommandSourceStack(),"time set midnight");
                server.setWeatherParameters(0,6000,true,true);
                String name=path;
                client.execute(()->{forest=name;stormAt=tick;client.player.setYRot(135);client.player.setXRot(20);});
            });
        }
        if(stormAt>0){
            float flash=JakSky.flash();
            flashMax=Math.max(flashMax,flash);
            if(darkShot<0&&frame>stormAt+120&&flash<=0&&JakAtmosphere.storm()>.3f){darkShot=frame;capture(client,"weather-forest-storm.png");forestGrade=dev.jakmc.client.sky.JakGrade.current();}
            if(flashShot<0&&darkShot>0&&flash>=.6f){flashShot=frame;capture(client,"weather-flash.png");}
        }
        if(stormAt>0&&(flashShot>0||frame>stormAt+900)||frame>2000){
            boolean desert=desertSeen;
            var result=new JsonObject();
            result.addProperty("desert",desert);
            result.addProperty("clearDistanceScale",clearFog);
            result.addProperty("stormDistanceScale",stormFog);
            result.addProperty("restoredDistanceScale",restoredFog);
            result.addProperty("sandVertices",sandPeak);
            result.addProperty("forest",forest);
            result.addProperty("flashMax",flashMax);
            result.addProperty("flashCaptured",flashShot>0);
            result.addProperty("desertGrade",java.util.Arrays.deepToString(desertGrade));
            result.addProperty("forestGrade",java.util.Arrays.deepToString(forestGrade));
            result.addProperty("reason",desert&&clearFog>.65f&&stormFog<.25f&&sandPeak>0&&restoredFog>.65f&&flashShot>0?"complete":"Weather acceptance failed");
            try{
                var dir=client.gameDirectory.toPath().resolve("jakmc/evidence/weather");
                Files.createDirectories(dir);Files.writeString(dir.resolve("result.json"),result.toString());
            }catch(IOException failure){throw new UncheckedIOException(failure);}
            running=false;client.options.pauseOnLostFocus=pause;
            server.execute(()->{restore.run();server.setWeatherParameters(2000,0,false,false);client.execute(client::stop);});
        }
    }
    private static void capture(Minecraft client,String name){
        Screenshot.grab(client.gameDirectory,name,client.gameRenderer.mainRenderTarget(),1,_-> {});
    }
}
