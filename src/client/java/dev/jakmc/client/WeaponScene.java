package dev.jakmc.client;
import com.google.gson.*;
import dev.jakmc.*;
import dev.jakmc.content.JakItems;
import dev.jakmc.player.JakPlayers;
import dev.jakmc.client.model.*;
import net.minecraft.client.*;
import net.minecraft.world.item.ItemStack;
import net.minecraft.world.phys.Vec3;
import java.nio.file.*;
import net.fabricmc.fabric.api.client.event.lifecycle.v1.ClientTickEvents;

/** Bounded real render check of all 12 original gun modes. No OS input. */
public final class WeaponScene {
 private static boolean running;private static int tick,deadline,gun,end;private static Path output;private static Vec3 origin;
 private static JsonObject row;private static JsonArray results;private static long soundStart,lastRender;
 public static void initialize(){ClientTickEvents.END_CLIENT_TICK.register(WeaponScene::tick);}
 public static void start(Minecraft c){
  if(c.player==null||c.getSingleplayerServer()==null||running)throw new IllegalStateException("Open the isolated dev world first");
  output=c.gameDirectory.toPath().resolve("jakmc/evidence/weapons");try{Files.createDirectories(output);}catch(Exception e){throw new RuntimeException(e);}
  var at=c.player.blockPosition();origin=new Vec3(at.getX()+.5,at.getY(),at.getZ()+.5);tick=-10;deadline=0;gun=0;end=12;results=new JsonArray();row=null;running=true;lastRender=JakRenderer.renderSerial;
  c.gui.setScreen(null);c.options.setCameraType(CameraType.THIRD_PERSON_BACK);

  var uuid=c.player.getUUID();c.getSingleplayerServer().execute(()->{
   var server=c.getSingleplayerServer();var p=server.getPlayerList().getPlayer(uuid);if(p==null)return;
   server.getCommands().performPrefixedCommand(server.createCommandSourceStack(),"time set noon");server.getCommands().performPrefixedCommand(server.createCommandSourceStack(),"weather clear");
   var base=net.minecraft.core.BlockPos.containing(origin);
   for(int x=-22;x<=22;x++)for(int z=-18;z<=24;z++){
    p.level().setBlock(base.offset(x,-1,z),net.minecraft.world.level.block.Blocks.SMOOTH_STONE.defaultBlockState(),3);
    for(int y=0;y<=8;y++)p.level().setBlock(base.offset(x,y,z),net.minecraft.world.level.block.Blocks.AIR.defaultBlockState(),3);
   }
   for(int x:new int[]{-2,2})for(int z:new int[]{-2,2})p.level().setBlock(base.offset(x,-1,z),net.minecraft.world.level.block.Blocks.SEA_LANTERN.defaultBlockState(),3);
   for(int x=-4;x<=4;x++)for(int y=0;y<=5;y++)p.level().setBlock(base.offset(x,y,12),net.minecraft.world.level.block.Blocks.STONE_BRICKS.defaultBlockState(),3);
   p.setPos(origin);p.setYRot(0);p.setXRot(0);p.setGameMode(net.minecraft.world.level.GameType.CREATIVE);p.setAttached(JakData.ENABLED,true);p.setAttached(JakData.BOARD,new ItemStack(JakItems.BOARD));JakPlayers.controlForProbe(p,true);
  });
 }
 public static void startRings(Minecraft c){start(c);gun=10;end=11;output=c.gameDirectory.toPath().resolve("jakmc/evidence/rings");try{Files.createDirectories(output);}catch(Exception e){throw new RuntimeException(e);}}
 public static Vec3 cameraPosition(){return running?origin.add(4.6,2.6,-4.6):null;}
 private static void tick(Minecraft c){
  if(!running)return;if(c.player==null||c.getSingleplayerServer()==null){running=false;c.stop();return;}
  if(++deadline>1400){finish(c,"deadline");return;}
  if(lastRender==JakRenderer.renderSerial)return;lastRender=JakRenderer.renderSerial;
  int t=++tick;var uuid=c.player.getUUID();int current=gun;
  if(t==0){row=new JsonObject();row.addProperty("mode",JakItems.GUNS.get(gun).toString());row.addProperty("effect_vertices",0);row.addProperty("lightning_vertices",0);row.addProperty("dark_ring_vertices",0);row.addProperty("model_draws",0);row.addProperty("interpolated_models",0);soundStart=JakSounds.played;
   c.gui.hud.getChat().clearMessages(false);
   c.getSingleplayerServer().execute(()->{var p=c.getSingleplayerServer().getPlayerList().getPlayer(uuid);if(p!=null){JakPlayers.resetForProbe(p);p.setYRot(0);p.setXRot(0);p.setItemInHand(net.minecraft.world.InteractionHand.MAIN_HAND,new ItemStack(JakItems.GUNS.get(current)));JakPlayers.probeInput(p,0,0,0);}});
  }
  if(t>=0){peak("effect_vertices",JakEffects.drawnVertices);peak("lightning_vertices",JakEffects.drawnLightningVertices);peak("dark_ring_vertices",JakEffects.drawnDarkVertices);peak("model_draws",JakShotRenderer.drawnModels);peak("interpolated_models",JakShotRenderer.interpolatedModels);}
  if(t==14)c.getSingleplayerServer().execute(()->{var p=c.getSingleplayerServer().getPlayerList().getPlayer(uuid);if(p!=null)JakPlayers.probeInput(p,64,64,0);});
  if(t==39)c.getSingleplayerServer().execute(()->{var p=c.getSingleplayerServer().getPlayerList().getPlayer(uuid);if(p!=null)JakPlayers.probeInput(p,0,0,0);});
  if(t==28||t==43)Screenshot.grab(c.gameDirectory,"weapon-"+gun+"-"+t+".png",c.gameRenderer.mainRenderTarget(),1,m->{});
  if(t>=60){row.addProperty("sound_requests",JakSounds.played-soundStart);results.add(row);tick=-1;if(++gun==end)finish(c,"complete");}
 }
 private static void peak(String key,int value){row.addProperty(key,Math.max(row.get(key).getAsInt(),value));}
 private static void finish(Minecraft c,String reason){
  var result=new JsonObject();result.addProperty("reason",reason);result.add("guns",results);result.addProperty("visual_review","required");
  try{Files.writeString(output.resolve("result.json"),result.toString());}catch(Exception e){throw new RuntimeException(e);}
  running=false;c.setCameraEntity(c.player);c.options.setCameraType(CameraType.THIRD_PERSON_BACK);var uuid=c.player.getUUID();c.getSingleplayerServer().execute(()->{var p=c.getSingleplayerServer().getPlayerList().getPlayer(uuid);if(p!=null){JakPlayers.probeInput(p,0,0,0);JakPlayers.controlForProbe(p,false);}});c.stop();
 }
}
