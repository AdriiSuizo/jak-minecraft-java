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
import net.minecraft.world.entity.EntityType;
import net.minecraft.world.level.GameType;
import net.minecraft.world.level.block.Blocks;
import net.minecraft.world.level.block.state.BlockState;
import net.minecraft.world.level.levelgen.Heightmap;
import net.minecraft.world.phys.Vec3;

/** The swamp frog (Kermit) with Jak standing still before it, then with a cow: lash, stuck tongue, pulse, shock, Jak pulled. */
final class KermitScene {
    private static boolean running,pause;
    private static int tick;
    private static int pulsesBefore,impactsBefore,pullsBefore;
    private static Runnable observe,restore;
    private static InputScene.CameraView view;
    private static final JsonObject result=new JsonObject();
    static InputScene.CameraView camera(){return view;}
    static void initialize(){ClientTickEvents.END_CLIENT_TICK.register(KermitScene::tick);}
    static void start(Minecraft client){
        var server=client.getSingleplayerServer();
        var playerId=client.player.getUUID();
        pause=client.options.pauseOnLostFocus;
        client.options.pauseOnLostFocus=false;
        result.entrySet().clear();tick=0;
        pulsesBefore=JakEffects.kermitPulses;impactsBefore=JakEffects.kermitImpacts;pullsBefore=JakPlayers.tonguePulls;
        server.execute(()->{
            var p=server.getPlayerList().getPlayer(playerId);
            if(p==null)return;
            var level=p.level();
            var old=p.position();var mode=p.gameMode.getGameModeForPlayer();
            boolean enabled=p.getAttachedOrCreate(JakData.ENABLED);
            float health=p.getHealth();
            var pos=p.blockPosition().offset(16,0,0);
            int y=pos.getY();
            for(int x=-4;x<=4;x++)for(int z=-4;z<=10;z++)
                y=Math.max(y,level.getHeight(Heightmap.Types.MOTION_BLOCKING_NO_LEAVES,pos.getX()+x,pos.getZ()+z));
            var base=pos.atY(y+2);
            var snapshot=new LinkedHashMap<BlockPos,BlockState>();
            for(int x=-4;x<=4;x++)for(int z=-4;z<=10;z++)for(int h=-1;h<=4;h++){
                var cell=base.offset(x,h,z);
                snapshot.put(cell,level.getBlockState(cell));
                level.setBlockAndUpdate(cell,h<0?Blocks.SMOOTH_STONE.defaultBlockState():Blocks.AIR.defaultBlockState());
            }
            var home=Vec3.atBottomCenterOf(base);
            p.setGameMode(GameType.SURVIVAL);p.setAttached(JakData.ENABLED,true);p.setHealth(p.getMaxHealth());
            p.teleportTo(home.x,home.y,home.z);JakPlayers.controlForProbe(p,true);
            var frog=JakEntities.TYPES.get("kermit").create(level,EntitySpawnReason.COMMAND);
            if(frog==null)throw new IllegalStateException("Kermit factory unavailable");
            frog.setPos(home.add(0,0,5));frog.setYRot(180);frog.setYBodyRot(180);frog.setYHeadRot(180);
            frog.setTarget(p);level.addFreshEntity(frog);
            var cow=net.minecraft.world.entity.EntityTypes.COW.create(level,EntitySpawnReason.COMMAND);
            restore=()->{
                frog.discard();if(cow!=null)cow.discard();snapshot.forEach(level::setBlockAndUpdate);
                JakPlayers.controlForProbe(p,false);p.setAttached(JakData.ENABLED,enabled);
                p.setGameMode(mode);p.setHealth(health);p.teleportTo(old.x,old.y,old.z);
            };
            observe=new Runnable(){
                int stage,stageTicks;
                double pulsePeak,stretchPeak,pullBlocks;
                Vec3 stuckAt;
                boolean lashed,stuck,shotPlayer,shotCow;
                float cowHealth;
                @Override public void run(){
                    stageTicks++;
                    String clip=frog.animation();
                    if(clip.equals("kermit-lash"))lashed=true;
                    pulsePeak=Math.max(pulsePeak,frog.tonguePulse());stretchPeak=Math.max(stretchPeak,frog.tongueStretch());
                    if(stage==0){
                        if(frog.tonguePulse()>=0&&stuckAt==null){stuck=true;stuckAt=p.position();}
                        if(stuckAt!=null)pullBlocks=Math.max(pullBlocks,stuckAt.subtract(p.position()).multiply(1,0,1).dot(frog.position().subtract(stuckAt).multiply(1,0,1).normalize()));
                        if(!shotPlayer&&frog.tonguePulse()>=.35f){shotPlayer=true;client.execute(()->Screenshot.grab(client.gameDirectory,"kermit-tongue.png",client.gameRenderer.mainRenderTarget(),1,_->{}));}
                        boolean shocked=p.getHealth()<p.getMaxHealth();
                        if(shocked||stageTicks>360){
                            result.addProperty("lashed",lashed);result.addProperty("stuck",stuck);
                            result.addProperty("pulsePeak",pulsePeak);result.addProperty("stretchPeak",stretchPeak);
                            result.addProperty("playerDamage",p.getMaxHealth()-p.getHealth());result.addProperty("jakPulledBlocks",pullBlocks);
                            result.addProperty("playerTicks",stageTicks);
                            // The cow where Jak stood; Jak out of the way, out of its sight.
                            p.setHealth(p.getMaxHealth());p.teleportTo(home.x+40,level.getHeight(Heightmap.Types.MOTION_BLOCKING_NO_LEAVES,(int)Math.floor(home.x+40),(int)Math.floor(home.z))+1,home.z);
                            if(cow!=null){cow.setPos(home);cow.setNoAi(true);level.addFreshEntity(cow);cowHealth=cow.getHealth();frog.setTarget(cow);}
                            stage=1;stageTicks=0;lashed=false;stuck=false;pulsePeak=-1;
                        }
                    }else if(stage==1){
                        if(cow!=null&&frog.tonguePulse()>=0)stuck=true;
                        if(cow!=null&&!shotCow&&frog.tonguePulse()>=.35f){shotCow=true;client.execute(()->Screenshot.grab(client.gameDirectory,"kermit-cow.png",client.gameRenderer.mainRenderTarget(),1,_->{}));}
                        boolean shocked=cow!=null&&cow.getHealth()<cowHealth;
                        if(shocked||stageTicks>360){
                            result.addProperty("cowLashed",lashed);result.addProperty("cowStuck",stuck);
                            result.addProperty("cowDamage",cow==null?0:cowHealth-cow.getHealth());result.addProperty("cowTicks",stageTicks);
                            stage=2;
                        }
                    }
                }
            };
            var eye=home.add(5.5,2.2,2.5);
            var look=home.add(0,.7,2.5).subtract(eye);
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
        var server=client.getSingleplayerServer();
        if(t<760&&!result.has("cowTicks")){server.execute(observe);return;}
        running=false;view=null;client.options.pauseOnLostFocus=pause;
        int pulses=JakEffects.kermitPulses-pulsesBefore,impacts=JakEffects.kermitImpacts-impactsBefore,pulls=JakPlayers.tonguePulls-pullsBefore;
        server.execute(()->{
            result.addProperty("pulsesDrawn",pulses);result.addProperty("impactsDrawn",impacts);result.addProperty("tonguePulls",pulls);
            boolean complete=result.has("cowTicks")&&result.get("playerDamage").getAsFloat()>0&&result.get("cowDamage").getAsFloat()>0
                &&pulses>=2&&impacts>=2&&pulls>0;
            result.addProperty("reason",complete?"complete":"Kermit acceptance failed");
            try{
                var dir=client.gameDirectory.toPath().resolve("jakmc/evidence/kermit");
                Files.createDirectories(dir);Files.writeString(dir.resolve("result.json"),result.toString());
            }catch(IOException failure){throw new UncheckedIOException(failure);}
            restore.run();client.execute(client::stop);
        });
    }
}
