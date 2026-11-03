package dev.jakmc.client;
import com.google.gson.*;
import dev.jakmc.*;
import dev.jakmc.content.*;
import dev.jakmc.player.JakPlayers;
import dev.jakmc.client.model.SkullGemRenderer;
import net.minecraft.client.*;
import net.minecraft.core.BlockPos;
import net.minecraft.server.level.*;
import net.minecraft.world.entity.item.ItemEntity;
import net.minecraft.world.item.*;
import net.minecraft.world.level.block.Blocks;
import net.minecraft.world.level.block.entity.HopperBlockEntity;
import net.minecraft.world.phys.*;
import java.nio.file.*;
import java.util.*;
import net.fabricmc.fabric.api.client.event.lifecycle.v1.ClientTickEvents;
/** One bounded mass-drop/shape/hopper/full-inventory/collection take, with hand and GUI views. */
public final class GemScene {
 private static boolean running,visualOnly;private static int tick,clipped;private static long spawn,bounce,suck,pickup,bursts,settleBounce;private static UUID player,full;private static final Set<UUID> IDS=new HashSet<>();private static final List<ItemStack> INVENTORY=new ArrayList<>();private static BlockPos base,hopper;private static Vec3 stage;private static Path output;private static final JsonObject RESULT=new JsonObject();private static InputScene.CameraView view;
 public static InputScene.CameraView camera(){return view;}
 public static void initialize(){ClientTickEvents.END_CLIENT_TICK.register(GemScene::tick);}
 public static void start(Minecraft c){start(c,false);}
 public static void startVisual(Minecraft c){start(c,true);}
 private static void start(Minecraft c,boolean visual){visualOnly=visual;if(running||c.player==null||c.getSingleplayerServer()==null)throw new IllegalStateException("Open isolated dev world");tick=clipped=0;IDS.clear();INVENTORY.clear();RESULT.entrySet().clear();spawn=GemPhysics.spawnCues;bounce=GemPhysics.bounceCues;suck=GemPhysics.suckCues;pickup=GemPhysics.pickupCues;bursts=JakEffects.gemCollectBursts;player=c.player.getUUID();output=c.gameDirectory.toPath().resolve(visualOnly?"jakmc/evidence/gems_visual":"jakmc/evidence/gems");try{Files.createDirectories(output);}catch(Exception e){throw new RuntimeException(e);}running=true;c.options.setCameraType(CameraType.THIRD_PERSON_BACK);
  c.getSingleplayerServer().execute(()->{var p=c.getSingleplayerServer().getPlayerList().getPlayer(player);if(p==null)return;base=p.blockPosition().offset(20,0,0);if(visualOnly){int top=base.getY();for(int x=-8;x<=8;x++)for(int z=-3;z<=15;z++)top=Math.max(top,p.level().getHeight(net.minecraft.world.level.levelgen.Heightmap.Types.MOTION_BLOCKING_NO_LEAVES,base.getX()+x,base.getZ()+z));base=new BlockPos(base.getX(),top+1,base.getZ());}stage=new Vec3(base.getX()+.5,base.getY(),base.getZ()+8.5);hopper=base.offset(6,0,5);
   for(int x=-8;x<=8;x++)for(int z=-3;z<=15;z++){p.level().setBlock(base.offset(x,-1,z),Blocks.SMOOTH_STONE.defaultBlockState(),3);for(int y=0;y<9;y++)p.level().setBlock(base.offset(x,y,z),Blocks.AIR.defaultBlockState(),3);}
   for(int y=0;y<3;y++)for(int z=-1;z<=1;z++)p.level().setBlock(base.offset(-1,y,z),Blocks.STONE.defaultBlockState(),3);
   p.level().setBlock(hopper,Blocks.HOPPER.defaultBlockState(),3);p.level().setBlock(base.offset(3,0,8),Blocks.STONE_SLAB.defaultBlockState(),3);
   for(int i=0;i<p.getInventory().getContainerSize();i++){INVENTORY.add(p.getInventory().getItem(i).copy());p.getInventory().setItem(i,ItemStack.EMPTY);}p.getInventory().setSelectedSlot(0);p.setAttached(JakData.ENABLED,true);p.setYRot(180);p.setYBodyRot(180);p.setPos(base.getX()+.5,base.getY(),base.getZ()+.5);JakPlayers.controlForProbe(p,true);
   for(int i=0;i<12;i++){var e=add(p,stage.add((i%4-1.5)*.6,3+i/4*.3,(i/4-1)*.6),true);e.setDeltaMovement((i%3-1)*.035,0,(i%2==0?1:-1)*.03);}
   add(p,new Vec3(base.getX()+3.5,base.getY()+3,base.getZ()+8.5),true);
   var blocked=add(p,new Vec3(base.getX()-2.5,base.getY()+.01,base.getZ()+.5),false);RESULT.addProperty("blocked_id",blocked.getUUID().toString());
   add(p,new Vec3(hopper.getX()+.5,hopper.getY()+1,hopper.getZ()+.5),true);
  });
 }
 private static ItemEntity add(ServerPlayer p,Vec3 at,boolean delay){var e=new ItemEntity(p.level(),at.x,at.y,at.z,new ItemStack(JakItems.SKULL_GEM));e.setDeltaMovement(Vec3.ZERO);if(delay)e.setNeverPickUp();else e.setNoPickUpDelay();p.level().addFreshEntity(e);IDS.add(e.getUUID());return e;}
 private static int count(ServerPlayer p){int n=0;for(int i=0;i<36;i++)if(p.getInventory().getItem(i).is(JakItems.SKULL_GEM))n+=p.getInventory().getItem(i).getCount();return n;}
 private static void tick(Minecraft c){if(!running)return;if(c.player==null||c.getSingleplayerServer()==null){finish(c,"lost world");return;}int t=++tick;if(stage==null)return;
  if(visualOnly){visual(c,t);return;}
  if(t<130){look(stage.add(3,3,-4),stage.add(0,.5,0));if(t==30||t==110)capture(c,"gems-world-"+t+".png");}
  if(t==140)c.gui.hud.getChat().clearMessages(false);
  if(t==140||t==205){boolean held=t==140;c.getSingleplayerServer().execute(()->{var p=c.getSingleplayerServer().getPlayerList().getPlayer(player);if(p!=null){p.setItemInHand(net.minecraft.world.InteractionHand.MAIN_HAND,held?new ItemStack(JakItems.SKULL_GEM):ItemStack.EMPTY);if(held){var enemy=dev.jakmc.entity.JakEntities.TYPES.get("grunt").create(p.level(),net.minecraft.world.entity.EntitySpawnReason.COMMAND);enemy.setPos(stage.add(-3,0,-2));enemy.setNoAi(true);p.level().addFreshEntity(enemy);IDS.add(enemy.getUUID());}}});}
  if(t>=140&&t<180){look(c.player.position().add(1.6,1.8,-2),c.player.position().add(0,1.1,0));if(t==160)capture(c,"gems-hand.png");}
  if(t==175){look(stage.add(-1,2,-4),stage.add(-3,1.1,-2));capture(c,"gems-carrier.png");}
  if(t==180){view=null;c.gui.setScreen(new net.minecraft.client.gui.screens.inventory.InventoryScreen(c.player));}if(t==195)capture(c,"gems-gui.png");if(t==205)c.gui.setScreen(null);
  c.getSingleplayerServer().execute(()->{var p=c.getSingleplayerServer().getPlayerList().getPlayer(player);if(p==null)return;
   for(UUID id:IDS){var e=p.level().getEntity(id);if(e instanceof ItemEntity item&&t>15&&!p.level().noCollision(item,item.getBoundingBox().deflate(1e-6)))clipped++;}
   if(t==210){RESULT.addProperty("wall_blocked_suction",p.level().getEntity(UUID.fromString(RESULT.get("blocked_id").getAsString()))!=null&&count(p)==0);for(int i=0;i<36;i++)p.getInventory().setItem(i,new ItemStack(Items.DIRT,64));var e=add(p,p.position().add(3,0,0),false);full=e.getUUID();}
   if(t==260){var e=p.level().getEntity(full);RESULT.addProperty("full_inventory_keeps_gem",e instanceof ItemEntity&&e.position().distanceToSqr(p.position().add(3,0,0))<.1);if(e!=null)e.discard();for(int i=0;i<36;i++)p.getInventory().setItem(i,ItemStack.EMPTY);var be=p.level().getBlockEntity(hopper);RESULT.addProperty("hopper_collected",be instanceof HopperBlockEntity h&&!h.isEmpty());}
   if(t==270)settleBounce=GemPhysics.bounceCues;
   if(t==285)RESULT.addProperty("settled_silent",GemPhysics.bounceCues==settleBounce);
   if(t==290){p.setPos(stage);for(UUID id:IDS){var e=p.level().getEntity(id);if(e instanceof ItemEntity item&&item.position().distanceToSqr(stage)<36)item.setNoPickUpDelay();}}
   if(t==390){RESULT.addProperty("inventory_received",count(p));RESULT.addProperty("mass_drop_collected",count(p)==13);RESULT.addProperty("clipped_samples",clipped);RESULT.addProperty("no_shape_penetration",clipped==0);RESULT.addProperty("spawn_cues",GemPhysics.spawnCues-spawn);RESULT.addProperty("bounce_cues",GemPhysics.bounceCues-bounce);RESULT.addProperty("suck_cues",GemPhysics.suckCues-suck);RESULT.addProperty("pickup_cues",GemPhysics.pickupCues-pickup);}
  });
  if(t>=290&&t<380){look(stage.add(3,2.5,-4),stage.add(0,.6,0));if(t==300||t==315||t==330)capture(c,"gems-collect-"+t+".png");}
  if(t>=400)finish(c,"complete");
 }
 private static void visual(Minecraft c,int t){
  if(t==15)c.getSingleplayerServer().execute(()->{var server=c.getSingleplayerServer();var p=server.getPlayerList().getPlayer(player);server.getCommands().performPrefixedCommand(server.createCommandSourceStack(),"time set noon");p.setItemInHand(net.minecraft.world.InteractionHand.MAIN_HAND,new ItemStack(JakItems.SKULL_GEM));p.setItemInHand(net.minecraft.world.InteractionHand.OFF_HAND,new ItemStack(JakItems.SKULL_GEM));var enemy=dev.jakmc.entity.JakEntities.TYPES.get("grunt").create(p.level(),net.minecraft.world.entity.EntitySpawnReason.COMMAND);enemy.setPos(stage.add(-3,0,-2));enemy.setNoAi(true);p.level().addFreshEntity(enemy);IDS.add(enemy.getUUID());});
  c.gui.hud.getChat().clearMessages(false);
  if(t<60)look(c.player.position().add(1.5,1.8,-2.5),c.player.position().add(0,1,0));else look(stage.add(-3,1.8,1),stage.add(-3,1.1,-2));
  if(t==45)capture(c,"gems-palm-day.png");if(t==85)capture(c,"gems-carrier-day.png");
  if(t==95)c.getSingleplayerServer().execute(()->{var server=c.getSingleplayerServer();server.getCommands().performPrefixedCommand(server.createCommandSourceStack(),"time set midnight");});
  if(t==120)capture(c,"gems-carrier-night.png");if(t>=130)finish(c,"complete");
 }
 private static void look(Vec3 at,Vec3 target){Vec3 d=target.subtract(at);view=new InputScene.CameraView(at,(float)Math.toDegrees(Math.atan2(-d.x,d.z)),(float)Math.toDegrees(Math.atan2(-d.y,d.horizontalDistance())));}
 private static void capture(Minecraft c,String name){Screenshot.grab(c.gameDirectory,name,c.gameRenderer.mainRenderTarget(),1,m->{});}
 private static void finish(Minecraft c,String reason){running=false;view=null;RESULT.addProperty("reason",reason);RESULT.addProperty("collect_bursts",JakEffects.gemCollectBursts-bursts);RESULT.addProperty("drawn_bodies",SkullGemRenderer.submitted);RESULT.addProperty("drawn_halos",SkullGemRenderer.halos);RESULT.addProperty("carrier_glows",SkullGemRenderer.carrierGlows);boolean passed=visualOnly?SkullGemRenderer.submitted>0&&SkullGemRenderer.carrierGlows>0:RESULT.has("mass_drop_collected")&&RESULT.get("mass_drop_collected").getAsBoolean()&&RESULT.get("wall_blocked_suction").getAsBoolean()&&RESULT.get("full_inventory_keeps_gem").getAsBoolean()&&RESULT.get("hopper_collected").getAsBoolean()&&RESULT.get("no_shape_penetration").getAsBoolean()&&RESULT.get("settled_silent").getAsBoolean()&&JakEffects.gemCollectBursts>bursts&&SkullGemRenderer.carrierGlows>0;RESULT.addProperty("visual_only",visualOnly);RESULT.addProperty("passed",passed);if(!passed)RESULT.addProperty("reason","gem assertion failed");try{Files.writeString(output.resolve("result.json"),RESULT.toString());}catch(Exception e){throw new RuntimeException(e);}if(c.getSingleplayerServer()!=null)c.getSingleplayerServer().execute(()->{var p=c.getSingleplayerServer().getPlayerList().getPlayer(player);if(p!=null){for(UUID id:IDS){var e=p.level().getEntity(id);if(e!=null)e.discard();}for(int i=0;i<INVENTORY.size();i++)p.getInventory().setItem(i,INVENTORY.get(i));JakPlayers.controlForProbe(p,false);}});c.stop();}
}
