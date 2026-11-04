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
 private static boolean running,firstPerson;private static int tick,deadline,gun,end,firstGun;private static Path output;private static Vec3 origin;
 private static JsonObject row;private static JsonArray results;private static long soundStart,lastRender,viewStart,viewFireStart,handStart,compatView,compatHands,compatItems;
 public static void initialize(){ClientTickEvents.END_CLIENT_TICK.register(WeaponScene::tick);}
 public static void start(Minecraft c){
  if(c.player==null||c.getSingleplayerServer()==null||running)throw new IllegalStateException("Open the isolated dev world first");
  firstPerson=false;output=c.gameDirectory.toPath().resolve("jakmc/evidence/weapons");try{Files.createDirectories(output);}catch(Exception e){throw new RuntimeException(e);}
  var at=c.player.blockPosition();origin=new Vec3(at.getX()+.5,at.getY(),at.getZ()+.5);tick=-10;deadline=0;gun=0;firstGun=0;end=12;results=new JsonArray();row=null;running=true;lastRender=JakRenderer.renderSerial;
  c.gui.setScreen(null);c.options.setCameraType(CameraType.THIRD_PERSON_BACK);

  var uuid=c.player.getUUID();c.getSingleplayerServer().execute(()->{
   var server=c.getSingleplayerServer();var p=server.getPlayerList().getPlayer(uuid);if(p==null)return;
   server.getCommands().performPrefixedCommand(server.createCommandSourceStack(),"time set noon");server.getCommands().performPrefixedCommand(server.createCommandSourceStack(),"weather clear");
   var base=net.minecraft.core.BlockPos.containing(origin);
   for(var entity:p.level().getEntities(p,new net.minecraft.world.phys.AABB(base).inflate(22,8,24)))
    if(!(entity instanceof net.minecraft.world.entity.player.Player))entity.discard();
   for(int x=-22;x<=22;x++)for(int z=-18;z<=24;z++){
    p.level().setBlock(base.offset(x,-1,z),net.minecraft.world.level.block.Blocks.SMOOTH_STONE.defaultBlockState(),3);
    for(int y=0;y<=8;y++)p.level().setBlock(base.offset(x,y,z),net.minecraft.world.level.block.Blocks.AIR.defaultBlockState(),3);
   }
   for(int x:new int[]{-2,2})for(int z:new int[]{-2,2})p.level().setBlock(base.offset(x,-1,z),net.minecraft.world.level.block.Blocks.SEA_LANTERN.defaultBlockState(),3);
   for(int x=-4;x<=4;x++)for(int y=0;y<=5;y++)p.level().setBlock(base.offset(x,y,12),net.minecraft.world.level.block.Blocks.STONE_BRICKS.defaultBlockState(),3);
   p.setPos(origin);p.setYRot(0);p.setXRot(0);p.setGameMode(net.minecraft.world.level.GameType.CREATIVE);p.setAttached(JakData.ENABLED,true);p.setAttached(JakData.BOARD,new ItemStack(JakItems.BOARD));JakPlayers.controlForProbe(p,true);JakPlayers.hostControlsForProbe(p);
  });
 }
 public static void startFirstPerson(Minecraft c){start(c);firstPerson=true;c.options.setCameraType(CameraType.FIRST_PERSON);output=c.gameDirectory.toPath().resolve("jakmc/evidence/first_person");try{Files.createDirectories(output);}catch(Exception e){throw new RuntimeException(e);}}
 public static void onlyMode(int mode){if(!running||mode<0||mode>=12)throw new IllegalArgumentException("Invalid weapon probe mode");gun=mode;firstGun=mode;end=mode+1;}
 private static void control(net.minecraft.server.level.ServerPlayer p,int held,int pressed,float forward){
  JakPlayers.probeInput(p,held|(firstPerson?dev.jakmc.network.JakPackets.FIRST_PERSON:0),pressed,forward);
 }
 public static void startRings(Minecraft c){start(c);gun=10;end=11;output=c.gameDirectory.toPath().resolve("jakmc/evidence/rings");try{Files.createDirectories(output);}catch(Exception e){throw new RuntimeException(e);}}
 public static Vec3 cameraPosition(){
  if(!running||firstPerson)return null;
  var shown=JakPoses.presented();
  return (shown==null?origin:shown).add(2.2,1.8,-2.2);
 }
 private static void tick(Minecraft c){
  if(!running)return;if(c.player==null||c.getSingleplayerServer()==null){running=false;c.stop();return;}
  if(++deadline>1400){finish(c,"deadline");return;}
  if(!firstPerson&&lastRender==JakRenderer.renderSerial)return;lastRender=JakRenderer.renderSerial;
  int t=++tick;var uuid=c.player.getUUID();int current=gun;
  if(t==0){row=new JsonObject();row.addProperty("mode",JakItems.GUNS.get(gun).toString());row.addProperty("socket_samples",0);row.addProperty("walking_socket_samples",0);row.addProperty("aim_socket_samples",0);row.addProperty("max_socket_error",0);row.addProperty("effect_vertices",0);row.addProperty("lightning_vertices",0);row.addProperty("dark_ring_vertices",0);row.addProperty("model_draws",0);row.addProperty("interpolated_models",0);soundStart=JakSounds.played;viewStart=JakRenderer.firstPersonSerial;viewFireStart=JakRenderer.firstPersonFireSerial;handStart=JakFirstPerson.suppressedMainHands;
   c.gui.hud.getChat().clearMessages(false);c.player.setYRot(0);c.player.setXRot(0);
   c.getSingleplayerServer().execute(()->{var p=c.getSingleplayerServer().getPlayerList().getPlayer(uuid);if(p!=null){p.setPos(origin);JakPlayers.resetForProbe(p);p.setYRot(0);p.setXRot(0);p.setItemInHand(net.minecraft.world.InteractionHand.MAIN_HAND,new ItemStack(JakItems.GUNS.get(current)));control(p,0,0,0);}});
  }
  if(t>=0){peak("effect_vertices",JakEffects.drawnVertices);peak("lightning_vertices",JakEffects.drawnLightningVertices);peak("dark_ring_vertices",JakEffects.drawnDarkVertices);peak("model_draws",JakShotRenderer.drawnModels);peak("interpolated_models",JakShotRenderer.interpolatedModels);}
  if(t>=8&&t<60){var pose=JakPoses.get(c.player.getId());if(pose!=null&&pose.raw().getAsJsonObject("gun").get("kind").getAsInt()==gun){
   double error=JakRenderer.weaponSocketError(pose,gun);
   if(pose.raw().get("walkMix") instanceof JsonObject)row.addProperty("walking_socket_samples",row.get("walking_socket_samples").getAsInt()+1);
   if(pose.raw().get("aim").getAsBoolean())row.addProperty("aim_socket_samples",row.get("aim_socket_samples").getAsInt()+1);
   row.addProperty("socket_samples",row.get("socket_samples").getAsInt()+1);row.addProperty("max_socket_error",Math.max(row.get("max_socket_error").getAsDouble(),error));
   if(!Double.isFinite(error)||error>.003){row.add("socket_pose",pose.raw().deepCopy());results.add(row);finish(c,"socket_mismatch");return;}
  }}
  if(t==8)c.gui.hud.getChat().clearMessages(false);
  if(t==14)c.getSingleplayerServer().execute(()->{var p=c.getSingleplayerServer().getPlayerList().getPlayer(uuid);if(p!=null)control(p,64,64,0);});
  if(t==39)c.getSingleplayerServer().execute(()->{var p=c.getSingleplayerServer().getPlayerList().getPlayer(uuid);if(p!=null)control(p,64,0,.6f);});
  if(t==46){c.player.setYRot(60);c.player.setXRot(firstPerson?-35:0);c.getSingleplayerServer().execute(()->{var p=c.getSingleplayerServer().getPlayerList().getPlayer(uuid);if(p!=null){p.setYRot(60);p.setXRot(firstPerson?-35:0);control(p,64|32,0,.6f);}});}
  if(t==55&&firstPerson){c.player.setYRot(-60);c.player.setXRot(35);c.getSingleplayerServer().execute(()->{var p=c.getSingleplayerServer().getPlayerList().getPlayer(uuid);if(p!=null){p.setYRot(-60);p.setXRot(35);control(p,64|32,0,0);}});}
  if(t==(firstPerson?64:55))c.getSingleplayerServer().execute(()->{var p=c.getSingleplayerServer().getPlayerList().getPlayer(uuid);if(p!=null)control(p,0,0,0);});
  if(firstPerson&&gun==3&&t==64){
   c.player.setYRot(0);c.player.setXRot(0);
   c.getSingleplayerServer().execute(()->{var p=c.getSingleplayerServer().getPlayerList().getPlayer(uuid);if(p!=null){
    var base=net.minecraft.core.BlockPos.containing(origin);for(int x=-1;x<=1;x++)for(int y=0;y<4;y++)p.level().setBlock(base.offset(x,y,1),net.minecraft.world.level.block.Blocks.STONE_BRICKS.defaultBlockState(),3);
    p.setPos(origin);JakPlayers.resetForProbe(p);p.setYRot(0);p.setXRot(0);control(p,0,0,0);
   }});
  }
  if(firstPerson&&gun==3&&t==109){
   var pose=JakPoses.get(c.player.getId());var point=pose.raw().getAsJsonObject("gun").getAsJsonArray("firePoint");
   double forward=point.get(2).getAsDouble()-pose.raw().getAsJsonArray("pos").get(2).getAsDouble();
   row.addProperty("near_wall_muzzle_forward",forward);row.addProperty("near_wall_retraction",JakRenderer.firstPersonRetraction);
   if(forward<0||forward>.501||JakRenderer.firstPersonRetraction>=.99f){results.add(row);finish(c,"view_muzzle_crossed_wall");return;}
   Screenshot.grab(c.gameDirectory,"view-wall.png",c.gameRenderer.mainRenderTarget(),1,m->{});
   c.getSingleplayerServer().execute(()->{var p=c.getSingleplayerServer().getPlayerList().getPlayer(uuid);if(p!=null){var base=net.minecraft.core.BlockPos.containing(origin);for(int x=-1;x<=1;x++)for(int y=0;y<4;y++)p.level().setBlock(base.offset(x,y,1),net.minecraft.world.level.block.Blocks.AIR.defaultBlockState(),3);control(p,0,0,0);}});
  }
  int compatibilityTick=t-(gun==3?40:0);
  if(firstPerson&&gun==firstGun){
   if(compatibilityTick==73){
    c.player.setYRot(0);c.player.setXRot(25);compatItems=GltfItemRenderer.originalGunDraws;
    c.getSingleplayerServer().execute(()->{var p=c.getSingleplayerServer().getPlayerList().getPlayer(uuid);if(p!=null){
     p.setPos(origin);JakPlayers.resetForProbe(p);p.setYRot(0);p.setXRot(25);p.setGameMode(net.minecraft.world.level.GameType.SURVIVAL);control(p,0,0,0);
     p.setItemInHand(net.minecraft.world.InteractionHand.MAIN_HAND,new ItemStack(net.minecraft.world.item.Items.IRON_PICKAXE));
     p.setItemInHand(net.minecraft.world.InteractionHand.OFF_HAND,new ItemStack(net.minecraft.world.item.Items.TORCH));
     var drop=new net.minecraft.world.entity.item.ItemEntity(p.level(),origin.x,origin.y+.25,origin.z+2,new ItemStack(JakItems.GUNS.get(current)));drop.setDeltaMovement(Vec3.ZERO);drop.setPickUpDelay(200);p.level().addFreshEntity(drop);
    }});
   }
   if(compatibilityTick==96){compatView=JakRenderer.firstPersonSerial;compatHands=JakFirstPerson.suppressedMainHands;}
   if(compatibilityTick==98){
    var nativeHands=c.gameRenderer.gameRenderState().levelRenderState.playerRenderState.firstPersonHandsAndItems;
    row.addProperty("native_main_height",nativeHands==null?-1:nativeHands.mainHandHeight);row.addProperty("native_off_height",nativeHands==null?-1:nativeHands.offHandHeight);
    row.addProperty("native_hands_busy",c.player.isHandsBusy());
    boolean raised=nativeHands!=null&&nativeHands.mainHandHeight>.95f&&nativeHands.offHandHeight>.95f;
    boolean vanilla=raised&&c.player.getMainHandItem().is(net.minecraft.world.item.Items.IRON_PICKAXE)&&c.player.getOffhandItem().is(net.minecraft.world.item.Items.TORCH)&&JakRenderer.firstPersonSerial==compatView&&JakFirstPerson.suppressedMainHands==compatHands;
    row.addProperty("native_items_preserved",vanilla);row.addProperty("ground_gun_draws",GltfItemRenderer.originalGunDraws-compatItems);
    if(!vanilla||GltfItemRenderer.originalGunDraws<=compatItems){results.add(row);finish(c,"view_overrode_native_items");return;}
    Screenshot.grab(c.gameDirectory,"view-native-items.png",c.gameRenderer.mainRenderTarget(),1,m->{});
   }
   if(compatibilityTick==99)c.gui.setScreen(new net.minecraft.client.gui.screens.inventory.InventoryScreen(c.player));
   if(compatibilityTick==102){
    boolean nativeMenu=c.gui.screen() instanceof net.minecraft.client.gui.screens.inventory.InventoryScreen;
    row.addProperty("native_inventory",nativeMenu);if(!nativeMenu){results.add(row);finish(c,"missing_native_inventory");return;}
    Screenshot.grab(c.gameDirectory,"view-inventory.png",c.gameRenderer.mainRenderTarget(),1,m->{});
   }
   if(compatibilityTick==103)c.gui.setScreen(null);
  }
  if(t==28||t==43||firstPerson&&(t==52||t==60))Screenshot.grab(c.gameDirectory,(firstPerson?"view-":"weapon-")+gun+"-"+t+".png",c.gameRenderer.mainRenderTarget(),1,m->{});
  if(t>=(firstPerson?(gun==firstGun?105:72)+(gun==3?40:0):65)){if(firstPerson){
   row.addProperty("first_person_draws",JakRenderer.firstPersonSerial-viewStart);row.addProperty("first_person_fire_draws",JakRenderer.firstPersonFireSerial-viewFireStart);row.addProperty("suppressed_static_draws",JakFirstPerson.suppressedMainHands-handStart);row.addProperty("arm_triangles",JakRenderer.firstPersonArmTriangles);
   if(JakRenderer.firstPersonSerial<=viewStart||JakRenderer.firstPersonFireSerial<=viewFireStart||JakFirstPerson.suppressedMainHands<=handStart||JakRenderer.firstPersonArmTriangles<=0){results.add(row);finish(c,"missing_view_rig");return;}
  }if(row.get("socket_samples").getAsInt()<20||row.get("walking_socket_samples").getAsInt()==0||row.get("aim_socket_samples").getAsInt()==0){results.add(row);finish(c,"missing_socket_phase");return;}row.addProperty("sound_requests",JakSounds.played-soundStart);results.add(row);if(firstPerson&&gun==firstGun)c.getSingleplayerServer().execute(()->{var p=c.getSingleplayerServer().getPlayerList().getPlayer(uuid);if(p!=null)p.setGameMode(net.minecraft.world.level.GameType.CREATIVE);});tick=-1;if(++gun==end)finish(c,"complete");}
 }
 private static void peak(String key,int value){row.addProperty(key,Math.max(row.get(key).getAsInt(),value));}
 private static void finish(Minecraft c,String reason){
  var result=new JsonObject();result.addProperty("reason",reason);result.add("guns",results);result.addProperty("visual_review","required");
  try{Files.writeString(output.resolve("result.json"),result.toString());}catch(Exception e){throw new RuntimeException(e);}
  running=false;c.setCameraEntity(c.player);c.options.setCameraType(CameraType.THIRD_PERSON_BACK);var uuid=c.player.getUUID();c.getSingleplayerServer().execute(()->{var p=c.getSingleplayerServer().getPlayerList().getPlayer(uuid);if(p!=null){control(p,0,0,0);JakPlayers.controlForProbe(p,false);}});c.stop();
 }
}
