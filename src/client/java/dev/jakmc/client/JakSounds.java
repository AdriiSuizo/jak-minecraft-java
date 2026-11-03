package dev.jakmc.client;

import com.google.gson.*;
import dev.jakmc.JakData;
import net.fabricmc.fabric.api.client.event.lifecycle.v1.ClientTickEvents;
import net.fabricmc.fabric.api.client.networking.v1.ClientPlayConnectionEvents;
import net.minecraft.client.Minecraft;
import net.minecraft.client.resources.sounds.*;
import net.minecraft.sounds.*;
import net.minecraft.world.phys.Vec3;
import java.util.*;

/** Original action cues, material footsteps and continuously updated native loops. */
public final class JakSounds {
 private record Key(int owner,String name){}
 private static final Map<Key,Loop> LOOPS=new HashMap<>();
 public static long played;
 public static void initialize(){
  ClientTickEvents.END_CLIENT_TICK.register(c->{LOOPS.entrySet().removeIf(e->e.getValue().isStopped());});
  ClientPlayConnectionEvents.DISCONNECT.register((handler,c)->clear());
 }
 private static SoundEvent event(String name){
  String path=name.startsWith("jak1:")?"sfx/jak1/"+name.substring(5):name.startsWith("jak2:")?"sfx/jak2/"+name.substring(5):"sfx/"+name;
  return SoundEvent.createVariableRangeEvent(JakData.id(path.toLowerCase(Locale.ROOT).replace('-','_')));
 }
 public static void receive(int owner,Vec3 origin,JsonObject packet){
  var client=Minecraft.getInstance();if(client.level==null)return;
  if(packet.has("sounds"))for(var v:packet.getAsJsonArray("sounds")){
   var cue=v.getAsJsonObject();Vec3 p=dev.jakmc.player.JakPlayers.vector(cue.getAsJsonArray("pos")).add(origin);
   client.getSoundManager().play(new SimpleSoundInstance(event(cue.get("name").getAsString()),SoundSource.PLAYERS,
    cue.get("gain").getAsFloat(),cue.get("pitch").getAsFloat(),SoundInstance.createUnseededRandom(),p.x,p.y,p.z));played++;
  }
  Set<Key> live=new HashSet<>();
  if(packet.has("loops"))for(var v:packet.getAsJsonArray("loops")){
   var cue=v.getAsJsonObject();String name=cue.get("name").getAsString();Key key=new Key(owner,name);live.add(key);
   Loop loop=LOOPS.get(key);boolean start=loop==null||loop.isStopped();if(start){loop=new Loop(owner,name);LOOPS.put(key,loop);}
   loop.update(cue,origin);if(start)client.getSoundManager().play(loop);
  }
  for(var e:LOOPS.entrySet())if(e.getKey().owner==owner&&!live.contains(e.getKey()))e.getValue().end();
 }
 public static void clear(){for(var loop:LOOPS.values())loop.end();LOOPS.clear();}
 private static final class Loop extends AbstractTickableSoundInstance {
  final int owner;long seen;Vec3 at=Vec3.ZERO;float gain=0,targetPitch=1;
  Loop(int owner,String name){super(event(name),SoundSource.PLAYERS,SoundInstance.createUnseededRandom());this.owner=owner;looping=true;delay=0;volume=0;}
  void update(JsonObject cue,Vec3 origin){at=dev.jakmc.player.JakPlayers.vector(cue.getAsJsonArray("pos")).add(origin);gain=cue.get("gain").getAsFloat();targetPitch=cue.get("pitch").getAsFloat();if(seen==0){x=at.x;y=at.y;z=at.z;volume=gain;pitch=targetPitch;}seen=System.nanoTime();}
  void end(){stop();}
  @Override public void tick(){
   var client=Minecraft.getInstance();if(client.level==null||client.level.getEntity(owner)==null||System.nanoTime()-seen>600_000_000L){stop();return;}
   x=at.x;y=at.y;z=at.z;volume+=(gain-volume)*.45f;pitch+=(targetPitch-pitch)*.45f;
  }
 }
}
