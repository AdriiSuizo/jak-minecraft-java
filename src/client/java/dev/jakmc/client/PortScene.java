package dev.jakmc.client;

import com.google.gson.*;
import dev.jakmc.*;
import dev.jakmc.content.JakItems;
import dev.jakmc.player.JakPlayers;
import net.fabricmc.fabric.api.client.event.lifecycle.v1.ClientTickEvents;
import net.minecraft.client.*;
import net.minecraft.world.item.ItemStack;
import java.nio.file.*;

/** Bounded gameplay cue check in the existing normal dev world. Always exits. */
public final class PortScene {
 private static int tick;private static boolean running,shotCapture,boardCapture;
 private static long soundsBefore;private static int peak;private static Path output;
 private static long lastRender;private static int deadline;
 private static final java.util.Map<String,Integer> EVENTS=new java.util.TreeMap<>();
 public static void initialize(){ClientTickEvents.END_CLIENT_TICK.register(PortScene::tick);}
 public static void start(Minecraft c){
  if(c.player==null||c.getSingleplayerServer()==null)throw new IllegalStateException("Open the existing dev world first");
  if(running)throw new IllegalStateException("Scene already running");
  output=c.gameDirectory.toPath().resolve("jakmc/evidence/effects");try{Files.createDirectories(output);}catch(Exception e){throw new RuntimeException(e);}
  tick=0;peak=0;deadline=0;EVENTS.clear();lastRender=dev.jakmc.client.model.JakRenderer.renderSerial;shotCapture=false;boardCapture=false;soundsBefore=JakSounds.played;running=true;
  c.gui.setScreen(null);c.options.setCameraType(CameraType.THIRD_PERSON_BACK);
  c.player.setYRot(0);c.player.setXRot(0);
  var uuid=c.player.getUUID();c.getSingleplayerServer().execute(()->{
   var p=c.getSingleplayerServer().getPlayerList().getPlayer(uuid);if(p==null)return;
   var server=c.getSingleplayerServer();server.getCommands().performPrefixedCommand(server.createCommandSourceStack(),"time set noon");server.getCommands().performPrefixedCommand(server.createCommandSourceStack(),"weather clear");
   var at=p.blockPosition();for(int dx=-5;dx<=5;dx++)for(int dz=-6;dz<=10;dz++){
    p.level().setBlock(at.offset(dx,-1,dz),net.minecraft.world.level.block.Blocks.SMOOTH_STONE.defaultBlockState(),3);
    for(int dy=0;dy<=5;dy++)p.level().setBlock(at.offset(dx,dy,dz),net.minecraft.world.level.block.Blocks.AIR.defaultBlockState(),3);
   }p.setPos(at.getX()+.5,at.getY(),at.getZ()+.5);
   p.setAttached(JakData.ENABLED,true);p.setAttached(JakData.BOARD,new ItemStack(JakItems.BOARD));
   p.setAttached(JakData.DARK_ECO,0f);p.setAttached(JakData.LIGHT_ECO,0f);
   for(var item:java.util.List.of(JakItems.DARK_ECO,JakItems.LIGHT_ECO)){
    var pickup=new net.minecraft.world.entity.item.ItemEntity(p.level(),p.getX()+.15,p.getY()+.1,p.getZ()+.15,new ItemStack(item));pickup.setNoPickUpDelay();p.level().addFreshEntity(pickup);
   }
   int drop=0;for(var item:java.util.List.of(JakItems.DARK_ECO,JakItems.LIGHT_ECO,JakItems.SKULL_GEM)){
    var pickup=new net.minecraft.world.entity.item.ItemEntity(p.level(),p.getX()-2+drop*2,p.getY()+.5,p.getZ()+2,new ItemStack(item));pickup.setNeverPickUp();p.level().addFreshEntity(pickup);drop++;
   }
   p.setYRot(0);p.setXRot(0);p.setItemInHand(net.minecraft.world.InteractionHand.MAIN_HAND,new ItemStack(JakItems.GUNS.get(3)));
   p.setGameMode(net.minecraft.world.level.GameType.CREATIVE);JakPlayers.controlForProbe(p,true);
  });
 }
 private static void tick(Minecraft c){
  if(!running)return;if(c.player==null||c.getSingleplayerServer()==null){running=false;c.stop();return;}
  if(++deadline>400){running=false;c.stop();return;}
  long rendered=dev.jakmc.client.model.JakRenderer.renderSerial;if(rendered==lastRender)return;lastRender=rendered;
  tick++;peak=Math.max(peak,JakEffects.drawnVertices);var uuid=c.player.getUUID();int frame=tick;
  if(frame==1)c.gui.hud.getChat().clearMessages(false);
  c.getSingleplayerServer().execute(()->{
   var p=c.getSingleplayerServer().getPlayerList().getPlayer(uuid);if(p==null)return;
   if(frame>=15&&frame<=55)JakPlayers.probeInput(p,frame%6<3?64:0,frame%6==0?64:0,0);
   if(frame==57){p.setItemInHand(net.minecraft.world.InteractionHand.MAIN_HAND,new ItemStack(JakItems.GUNS.get(1)));JakPlayers.probeInput(p,64,64,0);}
   if(frame==83)JakPlayers.probeInput(p,0,0,0);
   if(frame==100){p.setItemInHand(net.minecraft.world.InteractionHand.MAIN_HAND,ItemStack.EMPTY);JakPlayers.probeInput(p,128,128,0);}
   if(frame==101)JakPlayers.probeInput(p,0,0,0);
   if(frame==118)JakPlayers.probeInput(p,4,4,0);
   if(frame==119)JakPlayers.probeInput(p,0,0,0);
  });
  if(frame>=20&&frame<=55&&JakEffects.drawnVertices>0&&!shotCapture){capture(c,"port-blaster.png");shotCapture=true;}
  if(frame==87)capture(c,"port-wave.png");
  if(frame==120&&!boardCapture){capture(c,"port-board-zap.png");boardCapture=true;}
  if(frame>=145){
   JsonObject result=new JsonObject();result.addProperty("scene","Original action audio + sparticles in normal-world client");result.addProperty("sounds_requested",JakSounds.played-soundsBefore);result.addProperty("peak_effect_vertices",peak);result.addProperty("actor_drawn",dev.jakmc.client.model.JakRenderer.lastEntityId==c.player.getId());result.addProperty("visual_review","required");
   JsonObject events=new JsonObject();EVENTS.forEach(events::addProperty);result.add("events",events);result.addProperty("dark_pickup_energy",c.player.getAttachedOrCreate(JakData.DARK_ECO));result.addProperty("light_pickup_energy",c.player.getAttachedOrCreate(JakData.LIGHT_ECO));
   try{Files.writeString(output.resolve("result.json"),result.toString());}catch(Exception e){throw new RuntimeException(e);}
   running=false;c.getSingleplayerServer().execute(()->{var p=c.getSingleplayerServer().getPlayerList().getPlayer(uuid);if(p!=null)JakPlayers.controlForProbe(p,false);});c.stop();
  }
 }
 public static void observe(int id,JsonObject pose){if(!running||Minecraft.getInstance().player==null||Minecraft.getInstance().player.getId()!=id)return;for(var value:pose.getAsJsonArray("events")){var e=value.getAsJsonObject();String kind=e.get("kind").getAsString();if(e.has("gun"))kind+="/"+e.get("gun").getAsString();EVENTS.merge(kind,1,Integer::sum);}}
 private static void capture(Minecraft c,String file){Screenshot.grab(c.gameDirectory,file,c.gameRenderer.mainRenderTarget(),1,m->JakMod.LOG.info("Port scene: {}",m.getString()));}
}
