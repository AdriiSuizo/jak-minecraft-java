package dev.jakmc.client;

import com.google.gson.JsonArray;
import com.google.gson.JsonObject;
import dev.jakmc.JakData;
import java.nio.file.Files;
import net.minecraft.client.Minecraft;
import net.minecraft.client.Screenshot;

/**
 * Adrii 08/11: in a Replay Mod replay everything showed but Jak. A development probe (Replay Mod in run/mods): leave the
 * world, open run/replay-probe.mcpr in its viewer, play it, and write what the replay's world holds of Jak. A paused
 * replay stops the client's ticks, so a thread of its own drives it.
 */
final class ReplayProbeScene {
    private static final JsonObject result=new JsonObject();
    static void initialize(){}

    static void start(Minecraft client){
        result.entrySet().clear();
        client.disconnectWithProgressScreen();
        var thread=new Thread(()->run(client),"jakmc-replay-probe");thread.setDaemon(true);thread.start();
    }
    private static Object call(Object target,String method,Class<?>[] types,Object... args) throws ReflectiveOperationException{
        var m=target.getClass().getMethod(method,types);m.setAccessible(true);return m.invoke(target,args);
    }
    private static void run(Minecraft c){
        try{
            Thread.sleep(4000);
            c.execute(()->{
                try{
                    var mod=Class.forName("com.replaymod.replay.ReplayModReplay");
                    var instance=mod.getField("instance").get(null);
                    mod.getMethod("startReplay",java.io.File.class).invoke(instance,c.gameDirectory.toPath().resolve("replay-probe.mcpr").toFile());
                    result.addProperty("started",true);
                }catch(Throwable failure){result.addProperty("start_error",String.valueOf(failure.getCause()!=null?failure.getCause():failure));}
            });
            Thread.sleep(15000);
            sample(c,"paused");
            c.execute(()->{
                try{
                    var mod=Class.forName("com.replaymod.replay.ReplayModReplay");
                    var handler=call(mod.getField("instance").get(null),"getReplayHandler",new Class<?>[0]);
                    var sender=call(handler,"getReplaySender",new Class<?>[0]);
                    call(sender,"setReplaySpeed",new Class<?>[]{double.class},1.0);
                    result.addProperty("playing",true);
                }catch(Throwable failure){result.addProperty("play_error",String.valueOf(failure.getCause()!=null?failure.getCause():failure));}
            });
            for(int i=1;i<=8;i++){Thread.sleep(3000);sample(c,"play"+i);}
            for(int[] jump:new int[][]{{70000,1},{20000,2},{100000,3}}){
                c.execute(()->{
                    try{
                        var mod=Class.forName("com.replaymod.replay.ReplayModReplay");
                        var handler=call(mod.getField("instance").get(null),"getReplayHandler",new Class<?>[0]);
                        var sender=call(handler,"getReplaySender",new Class<?>[0]);
                        call(sender,"jumpToTime",new Class<?>[]{int.class},jump[0]);
                        call(sender,"setReplaySpeed",new Class<?>[]{double.class},1.0);
                    }catch(Throwable failure){result.addProperty("jump_error"+jump[1],String.valueOf(failure.getCause()!=null?failure.getCause():failure));}
                });
                Thread.sleep(4000);sample(c,"jump"+jump[1]+"a");Thread.sleep(3000);sample(c,"jump"+jump[1]+"b");
            }
        }catch(Throwable failure){result.addProperty("probe_error",String.valueOf(failure));}
        try{
            var dir=c.gameDirectory.toPath().resolve("jakmc/evidence/replay_probe");
            Files.createDirectories(dir);Files.writeString(dir.resolve("result.json"),result.toString());
        }catch(Exception ignored){}
        c.execute(c::stop);
    }
    private static void sample(Minecraft c,String name) throws InterruptedException{
        var done=new java.util.concurrent.CountDownLatch(1);
        c.execute(()->{
            try{
                var row=new JsonObject();
                row.addProperty("player_class",c.player==null?"null":c.player.getClass().getSimpleName());
                row.addProperty("player_id",c.player==null?-1:c.player.getId());
                var people=new JsonArray();
                if(c.level!=null)for(var e:c.level.entitiesForRendering()){
                    if(!(e instanceof net.minecraft.world.entity.player.Player))continue;
                    var o=new JsonObject();o.addProperty("id",e.getId());o.addProperty("class",e.getClass().getSimpleName());
                    o.addProperty("enabled",e.getAttachedOrCreate(JakData.ENABLED));
                    var pose=JakPoses.get(e.getId());
                    o.addProperty("pose",pose!=null);
                    if(pose!=null){o.addProperty("pose_age_ms",(System.nanoTime()-pose.received())/1_000_000);o.addProperty("clip",pose.clip());
                        o.addProperty("pose_x",pose.position().x);o.addProperty("pose_y",pose.position().y);}
                    o.addProperty("x",e.getX());o.addProperty("y",e.getY());o.addProperty("z",e.getZ());
                    o.addProperty("invisible",e.isInvisible());o.addProperty("pilot_gone",JakPilotDeath.hidden(e.getId()));
                    people.add(o);
                }
                row.add("players",people);
                row.addProperty("poses",JakPoses.all().size());
                row.addProperty("render_serial",dev.jakmc.client.model.JakRenderer.renderSerial);
                row.addProperty("last_drawn",dev.jakmc.client.model.JakRenderer.lastEntityId);
                result.add(name,row);
                Screenshot.grab(c.gameDirectory,"replay-probe-"+name+".png",c.gameRenderer.mainRenderTarget(),1,_->{});
            }finally{done.countDown();}
        });
        done.await(10,java.util.concurrent.TimeUnit.SECONDS);
    }
}
