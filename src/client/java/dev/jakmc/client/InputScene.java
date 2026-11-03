package dev.jakmc.client;
import com.google.gson.*;
import dev.jakmc.*;
import dev.jakmc.client.input.SdlVirtualPad;
import dev.jakmc.content.JakItems;
import dev.jakmc.player.JakPlayers;
import net.minecraft.client.*;
import net.minecraft.world.phys.Vec3;
import net.minecraft.world.entity.item.ItemEntity;
import net.minecraft.world.item.ItemStack;
import org.lwjgl.sdl.SDLGamepad;
import net.fabricmc.fabric.api.client.event.lifecycle.v1.ClientTickEvents;
import java.nio.file.*;
import java.util.*;

/** Replay through real SDL → normal client input → packets → server Jak; no probeInput. */
public final class InputScene {
 private static boolean running,interactions;private static final JsonObject ACTIONS=new JsonObject();private static final Set<CameraType> VIEWS=new HashSet<>();private static int tick;private static SdlVirtualPad pad;private static Path output;
 private static long deviceDeadline;private static Vec3 start,ecoCenter;private static double peak,walk;private static float yawStart,yawAfter;
 private static final Set<String> STATES=new TreeSet<>();private static final Set<UUID> ITEMS=new HashSet<>();
 public record CameraView(Vec3 position,float yaw,float pitch){}
 private static CameraView view;
 public static CameraView camera(){return view;}
 public static void initialize(){ClientTickEvents.END_CLIENT_TICK.register(InputScene::tick);}
 public static void start(Minecraft c){
  if(running||c.player==null||c.getSingleplayerServer()==null)throw new IllegalStateException("Open the isolated dev world");
  output=c.gameDirectory.toPath().resolve("jakmc/evidence/input");try{Files.createDirectories(output);}catch(Exception e){throw new RuntimeException(e);}
  interactions=false;ACTIONS.entrySet().clear();VIEWS.clear();
  start=c.player.position().add(7,0,0);tick=0;peak=0;walk=0;STATES.clear();ITEMS.clear();view=null;ecoCenter=null;
  c.gui.setScreen(null);c.options.setCameraType(CameraType.THIRD_PERSON_BACK);c.player.setYRot(0);c.player.setXRot(10);yawStart=0;yawAfter=0;
  pad=new SdlVirtualPad();deviceDeadline=System.nanoTime()+5_000_000_000L;running=true;var uuid=c.player.getUUID();c.getSingleplayerServer().execute(()->{
   var server=c.getSingleplayerServer();var p=server.getPlayerList().getPlayer(uuid);if(p==null)return;
   server.getCommands().performPrefixedCommand(server.createCommandSourceStack(),"time set noon");server.getCommands().performPrefixedCommand(server.createCommandSourceStack(),"weather clear");
   var base=net.minecraft.core.BlockPos.containing(start);start=new Vec3(base.getX()+.5,base.getY(),base.getZ()+.5);
   for(int x=-15;x<=15;x++)for(int z=-12;z<=35;z++){
    p.level().setBlock(base.offset(x,-1,z),net.minecraft.world.level.block.Blocks.SMOOTH_STONE.defaultBlockState(),3);
    for(int y=0;y<=7;y++)p.level().setBlock(base.offset(x,y,z),net.minecraft.world.level.block.Blocks.AIR.defaultBlockState(),3);
   }
   JakPlayers.controlForProbe(p,false);p.setPos(start);p.setYRot(0);p.setXRot(10);p.setGameMode(net.minecraft.world.level.GameType.CREATIVE);p.setAttached(JakData.ENABLED,true);p.setAttached(JakData.BOARD,new ItemStack(JakItems.BOARD));p.setItemInHand(net.minecraft.world.InteractionHand.MAIN_HAND,ItemStack.EMPTY);
  });
 }
 public static void startInteractions(Minecraft c){start(c);c.player.getInventory().setSelectedSlot(0);interactions=true;output=c.gameDirectory.toPath().resolve("jakmc/evidence/input_actions");try{Files.createDirectories(output);}catch(Exception e){throw new RuntimeException(e);}var uuid=c.player.getUUID();c.getSingleplayerServer().execute(()->{var p=c.getSingleplayerServer().getPlayerList().getPlayer(uuid);if(p!=null){p.getInventory().setSelectedSlot(0);p.setAttached(JakData.BOARD,ItemStack.EMPTY);p.setItemInHand(net.minecraft.world.InteractionHand.MAIN_HAND,new ItemStack(JakItems.BOARD));}});}
 private static void actions(Minecraft c){
  int t=++tick;pad.button(SDLGamepad.SDL_GAMEPAD_BUTTON_NORTH,t==25);pad.button(SDLGamepad.SDL_GAMEPAD_BUTTON_BACK,t==55||t==60||t==65);pad.button(SDLGamepad.SDL_GAMEPAD_BUTTON_DPAD_UP,t==75);pad.button(SDLGamepad.SDL_GAMEPAD_BUTTON_START,t==95);
  VIEWS.add(c.options.getCameraType());
  if(t==24||t==26||t==42)dev.jakmc.JakMod.LOG.info("Input interaction {}: hand={}, board={}, controller={}, focus={}",t,c.player.getMainHandItem(),c.player.getAttachedOrCreate(JakData.BOARD),JakInput.controller(),c.isWindowActive());
  if(t==42)ACTIONS.addProperty("triangle_equipped_board",!c.player.getAttachedOrCreate(JakData.BOARD).isEmpty());
  if(t==87){ACTIONS.addProperty("inventory_opened",c.gui.screen()!=null&&c.gui.screen().getClass().getSimpleName().equals("CreativeModeInventoryScreen"));capture(c,"input-actions-gui.png");}
  if(t>=105){ACTIONS.addProperty("all_camera_views",VIEWS.containsAll(Set.of(CameraType.FIRST_PERSON,CameraType.THIRD_PERSON_BACK,CameraType.THIRD_PERSON_FRONT)));ACTIONS.addProperty("start_closed_inventory",c.gui.screen()==null);finish(c,"complete");}
 }
 private static void tick(Minecraft c){
  if(!running)return;if(c.player==null||c.getSingleplayerServer()==null){finish(c,"lost world");return;}
  // Hot-plug enumeration is asynchronous. Replay must wait for the normal backend
  // to open the device at rest, as SDL calibrates virtual triggers on open.
  if(!JakInput.controllerReady(pad.id)){if(System.nanoTime()>deviceDeadline)finish(c,"SDL replay device not opened");return;}
  if(interactions){actions(c);return;}
  int t=++tick;
  var pose=JakPoses.get(c.player.getId());if(pose!=null){if(pose.raw().has("state"))STATES.add(pose.raw().get("state").getAsString());peak=Math.max(peak,c.player.getY()-start.y);}
  if(t==12){c.player.setYRot(0);c.player.setXRot(10);yawStart=c.player.getYRot();c.gui.hud.getChat().clearMessages(false);}
  pad.axis(1,t>=20&&t<45||t>=135&&t<155||t>=240&&t<260?-.9f:0);
  pad.axis(2,t>=50&&t<65?.6f:0);
  pad.button(SDLGamepad.SDL_GAMEPAD_BUTTON_SOUTH,t==80||t==84);
  pad.button(SDLGamepad.SDL_GAMEPAD_BUTTON_LEFT_SHOULDER,t>=135&&t<155);
  pad.button(SDLGamepad.SDL_GAMEPAD_BUTTON_EAST,t==180);
  pad.button(SDLGamepad.SDL_GAMEPAD_BUTTON_WEST,t==200);
  pad.axis(5,t==225||t==280?1:-1);
  if(t==75&&(walk<.1||Math.abs(yawAfter-yawStart)<.1)){finish(c,"Controller input not active; focused="+c.isWindowActive()+", device="+JakInput.controller().name());return;}
  if(t==45)walk=c.player.position().subtract(start).horizontalDistance();if(t==68)yawAfter=c.player.getYRot();
  if(t==35||t==86||t==146||t==248)capture(c,"input-move-"+t+".png");
  if(t==315){ecoCenter=c.player.position().add(0,.25,2);var uuid=c.player.getUUID();var center=ecoCenter;c.getSingleplayerServer().execute(()->{
   var p=c.getSingleplayerServer().getPlayerList().getPlayer(uuid);if(p==null)return;
   for(int i=0;i<2;i++){var item=new ItemEntity(p.level(),center.x+(i==0?-.6:.6),center.y,center.z,new ItemStack(i==0?JakItems.DARK_ECO:JakItems.LIGHT_ECO));item.setNeverPickUp();p.level().addFreshEntity(item);ITEMS.add(item.getUUID());}
  });}
  if(ecoCenter!=null&&t>=330&&t<410){
   int a=(t-330)/20;double angle=Math.toRadians(a*90);Vec3 at=ecoCenter.add(Math.sin(angle)*3,2,Math.cos(angle)*3);look(at,ecoCenter);
   if((t-330)%20==12)capture(c,"input-eco-world-"+a+".png");
  }
  if(t==415||t==445){var uuid=c.player.getUUID();boolean light=t==445;c.getSingleplayerServer().execute(()->{var p=c.getSingleplayerServer().getPlayerList().getPlayer(uuid);if(p!=null)p.setItemInHand(net.minecraft.world.InteractionHand.MAIN_HAND,new ItemStack(light?JakItems.LIGHT_ECO:JakItems.DARK_ECO));});}
  if(t>=415&&t<475){look(c.player.position().add(2,2.2,-2.8),c.player.position().add(0,1.1,0));if(t==435||t==465)capture(c,"input-eco-hand-"+t+".png");}
  if(t==480){view=null;c.gui.setScreen(new net.minecraft.client.gui.screens.inventory.InventoryScreen(c.player));}
  if(t==495)capture(c,"input-eco-gui.png");
  if(t>=505)finish(c,"complete");
 }
 private static void look(Vec3 at,Vec3 target){Vec3 d=target.subtract(at);view=new CameraView(at,(float)Math.toDegrees(Math.atan2(-d.x,d.z)),(float)Math.toDegrees(Math.atan2(-d.y,d.horizontalDistance())));}
 private static void capture(Minecraft c,String file){Screenshot.grab(c.gameDirectory,file,c.gameRenderer.mainRenderTarget(),1,m->{});}
 private static void finish(Minecraft c,String reason){
  running=false;view=null;if(pad!=null){pad.close();pad=null;}var result=new JsonObject();result.addProperty("reason",reason);result.addProperty("walk_blocks",walk);result.addProperty("jump_peak_blocks",peak);result.addProperty("camera_yaw_delta",yawAfter-yawStart);result.addProperty("states",String.join(",",STATES));result.addProperty("visual_review","required");if(interactions){result.add("actions",ACTIONS.deepCopy());if(ACTIONS.entrySet().stream().anyMatch(e->!e.getValue().getAsBoolean()))result.addProperty("reason","interaction assertion failed");}
  result.addProperty("jak_actions_reached",STATES.containsAll(Set.of("double-jump","roll","attack","board-stance")));
  try{Files.writeString(output.resolve("result.json"),result.toString());}catch(Exception e){throw new RuntimeException(e);}
  if(c.getSingleplayerServer()!=null){var server=c.getSingleplayerServer();server.execute(()->{for(UUID id:ITEMS){var item=server.overworld().getEntity(id);if(item!=null)item.discard();}});}c.stop();
 }
}
