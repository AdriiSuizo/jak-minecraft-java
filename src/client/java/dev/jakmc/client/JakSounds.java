package dev.jakmc.client;

import com.google.gson.*;
import dev.jakmc.JakData;
import net.fabricmc.fabric.api.client.event.lifecycle.v1.ClientTickEvents;
import net.fabricmc.fabric.api.client.networking.v1.ClientPlayConnectionEvents;
import net.minecraft.client.Minecraft;
import net.minecraft.core.BlockPos;
import net.minecraft.client.resources.sounds.*;
import net.minecraft.sounds.*;
import net.minecraft.world.phys.Vec3;
import java.util.*;

/** Original action cues, material footsteps and continuously updated native loops. */
public final class JakSounds {
 private record Key(int owner,String name){}
 private static final Map<Key,Loop> LOOPS=new HashMap<>();
 private static final Map<BlockPos,SceneryLoop> BARRELS=new HashMap<>(),VENTS=new HashMap<>();
 private static final Map<BlockPos,LauncherLoop> LAUNCHERS=new HashMap<>();
 private static WeatherLoop stormWind;
 public static long played;
 public static void initialize(){
  ClientTickEvents.END_CLIENT_TICK.register(c->{
   LOOPS.entrySet().removeIf(e->e.getValue().isStopped());
   BARRELS.values().removeIf(SceneryLoop::isStopped);
   VENTS.values().removeIf(SceneryLoop::isStopped);
   LAUNCHERS.values().removeIf(LauncherLoop::isStopped);
   if(c.level!=null&&c.player!=null)for(var entity:c.level.entitiesForRendering()){
    if(entity instanceof dev.jakmc.entity.JakHellcat cat&&cat.distanceToSqr(c.player)<64*64)vehicle(cat);
    if(entity instanceof dev.jakmc.entity.JakVehicle car&&car.distanceToSqr(c.player)<40*40){
     var packet=new JsonObject();
     if(car.frame()!=null&&car.getFirstPassenger()!=null)packet.add("loops",car.frame().getAsJsonArray("loops"));
     receive(car.getId(),car.position(),packet);
    }
   }
  });
  ClientPlayConnectionEvents.DISCONNECT.register((handler,c)->clear());
 }
 private static SoundEvent event(String name){
  String path=name.startsWith("jak1:")?"sfx/jak1/"+name.substring(5):name.startsWith("jak2:")?"sfx/jak2/"+name.substring(5):name.startsWith("jak3:")?"sfx/jak3/"+name.substring(5):"sfx/"+name;
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
 private static void vehicle(dev.jakmc.entity.JakHellcat cat){
  JsonObject packet=new JsonObject();
  JsonArray loops=new JsonArray();
  if(cat.thrust()>0&&!cat.wrecked()){
   appendLoop(loops,"hellcat-engine",1);
   if(cat.engine()>.01f)appendLoop(loops,"hellcat-boost",cat.engine());
   if(cat.wind()>.01f)appendLoop(loops,"hellcat-bank",cat.wind());
   float health=cat.hullHealth();
   if(health<.5f)appendLoop(loops,health<.15f?"damage-high":health<.2f?"damage-med":"damage-low",.8f);
  }
  packet.add("loops",loops);
  receive(cat.getId(),cat.position(),packet);
 }
 private static void appendLoop(JsonArray loops,String name,float gain){
  JsonObject loop=new JsonObject();
  loop.addProperty("name",name);loop.addProperty("gain",gain);loop.addProperty("pitch",1);
  JsonArray pos=new JsonArray();pos.add(0);pos.add(0);pos.add(0);loop.add("pos",pos);
  loops.add(loop);
 }
 static void barrel(BlockPos pos,Vec3 listener){
  if(Vec3.atCenterOf(pos).distanceToSqr(listener)>12*12||BARRELS.containsKey(pos))return;
  var loop=new SceneryLoop(pos,null);
  BARRELS.put(pos,loop);
  Minecraft.getInstance().getSoundManager().play(loop);
 }
 static void vent(BlockPos pos,dev.jakmc.content.JakEcoVent.Color color,Vec3 listener){
  if(Vec3.atCenterOf(pos).distanceToSqr(listener)>12.8*12.8||VENTS.containsKey(pos))return;
  var loop=new SceneryLoop(pos,color);
  VENTS.put(pos,loop);
  Minecraft.getInstance().getSoundManager().play(loop);
 }
 /** generic-obs.gc launcher-active: "launch-start" as it wakes, then "launch-idle" round while it stays awake. */
 static void launcher(BlockPos pos,Vec3 listener){
  var loop=LAUNCHERS.get(pos);
  if(loop!=null){loop.seen=System.nanoTime();return;}
  var sounds=Minecraft.getInstance().getSoundManager();
  sounds.play(new SimpleSoundInstance(event("jak1:launch-start"),SoundSource.BLOCKS,1,1,SoundInstance.createUnseededRandom(),pos.getX()+.5,pos.getY()+.2,pos.getZ()+.5));
  loop=new LauncherLoop(pos);LAUNCHERS.put(pos.immutable(),loop);sounds.play(loop);
 }
 private static final class LauncherLoop extends AbstractTickableSoundInstance {
  long seen=System.nanoTime();
  LauncherLoop(BlockPos pos){
   super(event("jak1:launch-idle"),SoundSource.BLOCKS,SoundInstance.createUnseededRandom());
   looping=true;delay=0;volume=.8f;x=pos.getX()+.5;y=pos.getY()+.2;z=pos.getZ()+.5;
  }
  // The decorations are found every 5 ticks; past half a second unseen it has gone to sleep.
  void end(){stop();}
  @Override public void tick(){if(System.nanoTime()-seen>500_000_000L)stop();}
 }
 static void stormWind(float gain){
  var client=Minecraft.getInstance();
  if(gain<.02f){
   if(stormWind!=null){stormWind.end();stormWind=null;}
  }else if(stormWind==null||stormWind.isStopped()){
   stormWind=new WeatherLoop(gain);client.getSoundManager().play(stormWind);
  }else stormWind.gain=gain;
 }
 private static final class WeatherLoop extends AbstractTickableSoundInstance {
  float gain;
  WeatherLoop(float gain){
   super(event("jak3:desert1/storm_wind"),SoundSource.WEATHER,SoundInstance.createUnseededRandom());
   this.gain=gain;volume=gain;looping=true;relative=true;attenuation=SoundInstance.Attenuation.NONE;
  }
  @Override public void tick(){volume+=(gain-volume)*.25f;}
  void end(){stop();}
 }
 public static void clear(){
  stormWind(0);
  for(var loop:LOOPS.values())loop.end();LOOPS.clear();
  for(var loop:BARRELS.values())loop.end();BARRELS.clear();
  for(var loop:VENTS.values())loop.end();VENTS.clear();
  for(var loop:LAUNCHERS.values())loop.end();LAUNCHERS.clear();
 }
 private static final class SceneryLoop extends AbstractTickableSoundInstance {
  final BlockPos position;
  final dev.jakmc.content.JakEcoVent.Color color;
  SceneryLoop(BlockPos pos,dev.jakmc.content.JakEcoVent.Color color){
   super(color==null?SoundEvent.createVariableRangeEvent(JakData.id("sfx/jak3/genbamb/can_fire")):event(color.ambient()),
       SoundSource.BLOCKS,SoundInstance.createUnseededRandom());
   position=pos.immutable();this.color=color;looping=true;delay=0;
   x=pos.getX()+.5;y=pos.getY()+(color==null?.80:.32);z=pos.getZ()+.5;
   volume=.24f;
  }
  void end(){stop();}
  @Override public void tick(){
   var client=Minecraft.getInstance();
   if(client.level==null||!client.level.hasChunkAt(position)
       ||Vec3.atCenterOf(position).distanceToSqr(client.gameRenderer.mainCamera().position())>16*16){stop();return;}
   var entity=client.level.getBlockEntity(position);
   if(color==null&&!(entity instanceof dev.jakmc.content.JakProps.FireBarrel)
       ||color!=null&&(!(entity instanceof dev.jakmc.content.JakEcoVent.Vent vent)||vent.color()!=color||!vent.open()))stop();
  }
 }
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
