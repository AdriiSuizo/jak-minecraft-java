package dev.jakmc.client;

import com.google.gson.JsonObject;
import dev.jakmc.JakData;
import dev.jakmc.entity.JakEnemy;
import dev.jakmc.entity.JakEntities;
import dev.jakmc.player.JakPlayers;
import java.io.IOException;
import java.io.UncheckedIOException;
import java.nio.file.Files;
import java.util.LinkedHashMap;
import net.fabricmc.fabric.api.client.event.lifecycle.v1.ClientTickEvents;
import net.minecraft.client.Minecraft;
import net.minecraft.client.Screenshot;
import net.minecraft.core.BlockPos;
import net.minecraft.world.entity.EntitySpawnReason;
import net.minecraft.world.level.GameType;
import net.minecraft.world.level.block.Blocks;
import net.minecraft.world.level.block.state.BlockState;
import net.minecraft.world.level.levelgen.Heightmap;
import net.minecraft.world.phys.Vec3;

final class SpydroidScene {
    private static boolean running,pause,captured;
    private static int tick,burstPeak;
    private static long burstBefore;
    private static Runnable observe,restore;
    private static InputScene.CameraView view;
    private static final JsonObject result=new JsonObject();
    static InputScene.CameraView camera(){return view;}
    static void initialize(){ClientTickEvents.END_CLIENT_TICK.register(SpydroidScene::tick);}
    static void start(Minecraft client){
        var server=client.getSingleplayerServer();
        var playerId=client.player.getUUID();
        pause=client.options.pauseOnLostFocus;
        client.options.pauseOnLostFocus=false;
        result.entrySet().clear();tick=burstPeak=0;captured=false;burstBefore=JakEffects.spydroidBursts;
        server.execute(()->{
            var p=server.getPlayerList().getPlayer(playerId);
            if(p==null)return;
            var level=p.level();
            var old=p.position();var mode=p.gameMode.getGameModeForPlayer();
            boolean enabled=p.getAttachedOrCreate(JakData.ENABLED);
            float health=p.getHealth();
            var pos=p.blockPosition().offset(16,0,0);
            int y=pos.getY();
            for(int x=-3;x<=3;x++)for(int z=-3;z<=7;z++)
                y=Math.max(y,level.getHeight(Heightmap.Types.MOTION_BLOCKING_NO_LEAVES,pos.getX()+x,pos.getZ()+z));
            var base=pos.atY(y+2);
            var snapshot=new LinkedHashMap<BlockPos,BlockState>();
            for(int x=-3;x<=3;x++)for(int z=-3;z<=7;z++){
                var cell=base.offset(x,-1,z);
                snapshot.put(cell,level.getBlockState(cell));
                level.setBlockAndUpdate(cell,Blocks.SMOOTH_STONE.defaultBlockState());
            }
            var home=Vec3.atBottomCenterOf(base);
            p.setGameMode(GameType.SURVIVAL);p.setAttached(JakData.ENABLED,true);
            p.teleportTo(home.x,home.y,home.z);JakPlayers.controlForProbe(p,true);
            var droid=JakEntities.TYPES.get("spydroid").create(level,EntitySpawnReason.COMMAND);
            if(droid==null)throw new IllegalStateException("Spydroid factory unavailable");
            droid.setPos(home.add(0,0,3.0));droid.setYRot(180);droid.setYBodyRot(180);
            droid.setTarget(p);level.addFreshEntity(droid);
            restore=()->{
                droid.discard();snapshot.forEach(level::setBlockAndUpdate);
                JakPlayers.controlForProbe(p,false);p.setAttached(JakData.ENABLED,enabled);
                p.setGameMode(mode);p.setHealth(health);p.teleportTo(old.x,old.y,old.z);
            };
            observe=new Runnable(){
                Vec3 jumpStart;
                double jumpDistance;
                boolean jump,killed;
                @Override public void run(){
                    String clip=droid.animation();
                    if(!killed&&clip.equals("spydroid-attack-jump")){
                        jump=true;
                        if(jumpStart==null)jumpStart=droid.position();
                        jumpDistance=Math.max(jumpDistance,droid.position().subtract(jumpStart).horizontalDistance());
                    }
                    if(!killed&&clip.equals("spydroid-attack-land")){
                        result.addProperty("jump",jump);
                        result.addProperty("land",true);
                        result.addProperty("lungeBlocks",jumpDistance);
                        result.addProperty("actualDamage",health-p.getHealth());
                        killed=true;
                        droid.hurtServer(level,p.damageSources().playerAttack(p),20);
                        client.execute(()->Screenshot.grab(client.gameDirectory,"spydroid-lunge.png",client.gameRenderer.mainRenderTarget(),1,_->{}));
                    }
                }
            };
            var eye=home.add(4,2.5,-3);
            var look=home.add(0,.65,1.4).subtract(eye);
            client.execute(()->{
                view=new InputScene.CameraView(eye,(float)Math.toDegrees(Math.atan2(-look.x,look.z)),
                    (float)-Math.toDegrees(Math.atan2(look.y,look.horizontalDistance())));
                running=true;
            });
        });
    }
    private static void tick(Minecraft client){
        if(!running)return;
        int t=++tick;
        burstPeak=Math.max(burstPeak,JakEffects.drawnBurstVertices);
        if(t>10&&burstPeak>0&&!captured){
            captured=true;
            Screenshot.grab(client.gameDirectory,"spydroid-explosion.png",client.gameRenderer.mainRenderTarget(),1,_-> {});
        }
        var server=client.getSingleplayerServer();
        if(t<100){server.execute(observe);return;}
        running=false;view=null;client.options.pauseOnLostFocus=pause;
        long bursts=JakEffects.spydroidBursts-burstBefore;
        server.execute(()->{
            result.addProperty("bursts",bursts);result.addProperty("vertices",burstPeak);
            boolean complete=result.has("land")&&result.get("lungeBlocks").getAsDouble()>.2
                &&result.get("actualDamage").getAsFloat()>=2&&bursts==1&&burstPeak>0;
            result.addProperty("reason",complete?"complete":"Spydroid acceptance failed");
            try{
                var dir=client.gameDirectory.toPath().resolve("jakmc/evidence/spydroid");
                Files.createDirectories(dir);Files.writeString(dir.resolve("result.json"),result.toString());
            }catch(IOException failure){throw new UncheckedIOException(failure);}
            restore.run();client.execute(client::stop);
        });
    }
}
