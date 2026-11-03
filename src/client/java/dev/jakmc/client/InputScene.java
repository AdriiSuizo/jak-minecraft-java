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
 private static boolean running,interactions,powers,leaper;private static final JsonObject ACTIONS=new JsonObject();private static final Set<CameraType> VIEWS=new HashSet<>();private static int tick;private static SdlVirtualPad pad;private static Path output;
 private static long deviceDeadline;private static Vec3 start,ecoCenter;private static double peak,walk;private static float yawStart,yawAfter;
 private static final Set<String> STATES=new TreeSet<>();private static final Set<UUID> ITEMS=new HashSet<>();
 public record CameraView(Vec3 position,float yaw,float pitch){}
 private static CameraView view;
 public static CameraView camera(){return view;}
 public static void initialize(){ClientTickEvents.END_CLIENT_TICK.register(InputScene::tick);}
 public static void start(Minecraft c){start(c,false);}
 private static void start(Minecraft c,boolean outdoor){
  if(running||c.player==null||c.getSingleplayerServer()==null)throw new IllegalStateException("Open the isolated dev world");
  output=c.gameDirectory.toPath().resolve("jakmc/evidence/input");try{Files.createDirectories(output);}catch(Exception e){throw new RuntimeException(e);}
  interactions=powers=leaper=false;ACTIONS.entrySet().clear();VIEWS.clear();
  start=c.player.position().add(7,0,0);tick=0;peak=0;walk=0;STATES.clear();ITEMS.clear();view=null;ecoCenter=null;
  c.gui.setScreen(null);c.options.setCameraType(CameraType.THIRD_PERSON_BACK);c.player.setYRot(0);c.player.setXRot(10);yawStart=0;yawAfter=0;
  pad=new SdlVirtualPad();deviceDeadline=System.nanoTime()+5_000_000_000L;running=true;var uuid=c.player.getUUID();c.getSingleplayerServer().execute(()->{
   var server=c.getSingleplayerServer();var p=server.getPlayerList().getPlayer(uuid);if(p==null)return;
   server.getCommands().performPrefixedCommand(server.createCommandSourceStack(),"time set noon");server.getCommands().performPrefixedCommand(server.createCommandSourceStack(),"weather clear");
   var base=net.minecraft.core.BlockPos.containing(start);
   if(outdoor){
    int y=base.getY();
    for(int x=-15;x<=15;x++)for(int z=-12;z<=35;z++){
     y=Math.max(y,p.level().getHeight(net.minecraft.world.level.levelgen.Heightmap.Types.MOTION_BLOCKING_NO_LEAVES,base.getX()+x,base.getZ()+z));
    }
    base=base.atY(y+1);
   }
   start=new Vec3(base.getX()+.5,base.getY(),base.getZ()+.5);
   for(int x=-15;x<=15;x++)for(int z=-12;z<=35;z++){
    p.level().setBlock(base.offset(x,-1,z),net.minecraft.world.level.block.Blocks.SMOOTH_STONE.defaultBlockState(),3);
    for(int y=0;y<=7;y++)p.level().setBlock(base.offset(x,y,z),net.minecraft.world.level.block.Blocks.AIR.defaultBlockState(),3);
   }
   JakPlayers.controlForProbe(p,false);p.setPos(start);p.setYRot(0);p.setXRot(10);p.setGameMode(net.minecraft.world.level.GameType.CREATIVE);p.setAttached(JakData.ENABLED,true);p.setAttached(JakData.BOARD,new ItemStack(JakItems.BOARD));p.setItemInHand(net.minecraft.world.InteractionHand.MAIN_HAND,ItemStack.EMPTY);
  });
 }
 public static void startInteractions(Minecraft c){start(c);c.player.getInventory().setSelectedSlot(0);interactions=true;output=c.gameDirectory.toPath().resolve("jakmc/evidence/input_actions");try{Files.createDirectories(output);}catch(Exception e){throw new RuntimeException(e);}var uuid=c.player.getUUID();c.getSingleplayerServer().execute(()->{var p=c.getSingleplayerServer().getPlayerList().getPlayer(uuid);if(p!=null){p.getInventory().setSelectedSlot(0);p.setAttached(JakData.BOARD,ItemStack.EMPTY);p.setItemInHand(net.minecraft.world.InteractionHand.MAIN_HAND,new ItemStack(JakItems.BOARD));}});}
 private static UUID leaperId;
 private static volatile int leaperEntity;
 private static boolean mountSeen,offSeen;
 private static int rideStart,jumpStart,offPress,slamImage,mountPress;
 private static volatile boolean gateOpened,mountDamaged,mountSaved;
 private static boolean hitRequested;
 private static net.minecraft.core.BlockPos gate;
 private static double rideTravel;
 private static Vec3 mountStart;
 public static void startLeaper(Minecraft c){
  start(c,true);
  leaper=true;
  pauseOnLostFocus=c.options.pauseOnLostFocus;
  c.options.pauseOnLostFocus=false;
  output=c.gameDirectory.toPath().resolve("jakmc/evidence/leaper");
  try{Files.createDirectories(output);}catch(Exception e){throw new RuntimeException(e);}
  mountSeen=offSeen=false;
  rideStart=jumpStart=offPress=slamImage=mountPress=0;
  gateOpened=mountDamaged=mountSaved=hitRequested=false;
  rideTravel=0;
  leaperEntity=-1;
  leaperId=null;
  onServer(c,p->{
   var mount=dev.jakmc.entity.JakLeaper.TYPE.create(p.level(),net.minecraft.world.entity.EntitySpawnReason.COMMAND);
   mount.setNoAi(true);
   mount.setPos(start.add(0,0,2.5));
   mount.setYRot(0);mount.setYBodyRot(0);mount.setYHeadRot(0);
   gate=net.minecraft.core.BlockPos.containing(mount.position().add(2,0,0));
   p.level().setBlock(gate,net.minecraft.world.level.block.Blocks.OAK_FENCE_GATE.defaultBlockState(),3);
   if(p.level().addFreshEntity(mount)){
    leaperId=mount.getUUID();leaperEntity=mount.getId();ITEMS.add(leaperId);
   }
   var base=net.minecraft.core.BlockPos.containing(start);
   for(int x=-5;x<=5;x++)for(int y=0;y<=5;y++){
    p.level().setBlock(base.offset(x,y,24),net.minecraft.world.level.block.Blocks.STONE.defaultBlockState(),3);
   }
  });
 }
 private static void leaperActions(Minecraft c){
  int t=++tick;
  var pose=JakPoses.get(c.player.getId());
  if(t==12){c.player.setYRot(0);c.player.setXRot(2);c.gui.hud.getChat().clearMessages(false);}
  view=null;
  var target=c.level.getEntity(leaperEntity);
  if(t>=22&&t<=25&&target!=null)mountView(target.position());
  if(t==25)capture(c,"leaper-idle.png");
  if(t>25&&t<85&&!mountSeen&&target!=null){
   aimAt(c,target.position().add(0,1.25,0));
   if(mountPress==0&&c.hitResult instanceof net.minecraft.world.phys.EntityHitResult hit&&hit.getEntity()==target){
    mountPress=t+2;
   }
  }
  boolean mounted=pose!=null&&pose.raw().get("ride") instanceof JsonObject;
  String state=pose==null?"":pose.raw().get("state").getAsString();
  if(state.startsWith("target-flut-"))STATES.add(state);
  if(!mountSeen&&mounted){
   mountSeen=true;
   capture(c,"leaper-mount.png");
  }
  if(rideStart==0&&state.equals("target-flut-stance")){
   rideStart=t;
   mountStart=pose.position();
  }
  int rideAge=rideStart==0?-1:t-rideStart;
  if(rideAge>=15&&rideAge<=18)mountView(pose.position());
  if(rideAge==18)capture(c,"leaper-riding.png");
  if(rideAge>=19&&rideAge<=23)aimAt(c,Vec3.atBottomCenterOf(gate).add(0,.65,0));
  if(rideAge==24){
   onServer(c,p->gateOpened=p.level().getBlockState(gate).getValue(net.minecraft.world.level.block.FenceGateBlock.OPEN));
   c.player.setYRot(0);c.player.setXRot(2);
  }
  pad.axis(1,rideAge>=25&&rideAge<47?-.8f:0);
  if(rideAge==50)jumpStart=t;
  int jumpAge=jumpStart==0?-1:t-jumpStart;
  pad.button(SDLGamepad.SDL_GAMEPAD_BUTTON_SOUTH,jumpAge>=0&&jumpAge<8||jumpAge>=12&&jumpAge<17);
  pad.button(SDLGamepad.SDL_GAMEPAD_BUTTON_WEST,jumpAge>=21&&jumpAge<24);
  if(jumpAge>=17&&jumpAge<=19&&mounted)mountView(pose.position());
  if(jumpAge==19)capture(c,"leaper-jump.png");
  if(pose!=null&&mounted&&mountStart!=null)rideTravel=Math.max(rideTravel,pose.position().subtract(mountStart).horizontalDistance());
  if(!hitRequested&&jumpAge>=38&&state.equals("target-flut-stance")){
   hitRequested=true;
   onServer(c,p->{
    if(p.level().getEntity(leaperId) instanceof dev.jakmc.entity.JakLeaper mount){
     float old=mount.getHealth();
     mount.hurtServer(p.level(),p.damageSources().generic(),4);
     mountDamaged=mount.getHealth()<old;
    }
   });
  }
  if(offPress==0&&STATES.contains("target-flut-hit")&&state.equals("target-flut-stance")){
   c.player.setXRot(-25);
   offPress=t+4;
  }
  pad.button(SDLGamepad.SDL_GAMEPAD_BUTTON_NORTH,mountPress>0&&t>=mountPress&&t<mountPress+4||rideAge>=21&&rideAge<23||offPress>0&&t>=offPress&&t<offPress+4);
  if(pose!=null)for(var event:pose.raw().getAsJsonArray("events")){
   if(event.getAsJsonObject().get("kind").getAsString().equals("flut_slam")&&!ACTIONS.has("ground_attack")){
    ACTIONS.addProperty("ground_attack",true);
    slamImage=t+3;
   }
  }
  if(slamImage>0&&t>=slamImage-1&&t<=slamImage&&target!=null)mountView(target.position());
  if(t==slamImage)capture(c,"leaper-slam.png");
  if(offPress>0&&t>offPress&&!mounted&&mountSeen&&!offSeen){
   offSeen=true;
   onServer(c,p->{
    if(p.level().getEntity(leaperId) instanceof dev.jakmc.entity.JakLeaper mount){
     var reporter=net.minecraft.util.ProblemReporter.DISCARDING;
     var output=net.minecraft.world.level.storage.TagValueOutput.createWithContext(reporter,mount.registryAccess());
     mount.saveWithoutId(output);
     var copy=dev.jakmc.entity.JakLeaper.TYPE.create(p.level(),net.minecraft.world.entity.EntitySpawnReason.LOAD);
     copy.load(net.minecraft.world.level.storage.TagValueInput.create(reporter,copy.registryAccess(),output.buildResult()));
     mountSaved=copy.riderId()<0&&copy.getHealth()==mount.getHealth()&&copy.position().distanceToSqr(mount.position())<.001;
    }
   });
   capture(c,"leaper-dismount.png");
  }
  if(t>=265){
   var entity=c.level.getEntity(leaperEntity);
   ACTIONS.addProperty("mounted_via_interact",mountSeen&&rideStart>0);
   ACTIONS.addProperty("rode",rideTravel>4);
   ACTIONS.addProperty("interacted_while_mounted",gateOpened);
   ACTIONS.addProperty("mounted_damage_reaction",mountDamaged&&STATES.contains("target-flut-hit"));
   ACTIONS.addProperty("native_save_roundtrip",mountSaved);
   ACTIONS.addProperty("jump",STATES.contains("target-flut-jump"));
   ACTIONS.addProperty("double_jump",STATES.contains("target-flut-double-jump"));
   ACTIONS.addProperty("dismounted",offSeen&&entity instanceof dev.jakmc.entity.JakLeaper mount&&mount.riderId()<0
       &&mount.position().distanceTo(c.player.position())>.4);
   ACTIONS.addProperty("ground_attack",ACTIONS.has("ground_attack"));
   STATES.add("mount_input="+mountPress);
   walk=rideTravel;
   finish(c,ACTIONS.entrySet().stream().allMatch(e->e.getValue().getAsBoolean())?"complete":"leaper assertion failed");
  }
 }
 private static void aimAt(Minecraft client,Vec3 target){
  Vec3 delta=target.subtract(client.player.getEyePosition());
  float yaw=(float)Math.toDegrees(Math.atan2(-delta.x,delta.z));
  float pitch=(float)-Math.toDegrees(Math.atan2(delta.y,delta.horizontalDistance()));
  client.player.turn(net.minecraft.util.Mth.wrapDegrees(yaw-client.player.getYRot())/.15,
      (pitch-client.player.getXRot())/.15);
 }
 private static void mountView(Vec3 feet){look(feet.add(3.5,2.5,-2.3),feet.add(0,1.25,0));}
 private static boolean shieldTested,darkSeen,lightSeen,wingsSeen,shotSeen,aimSeen,flightSeen,healSeen;
 private static volatile boolean hit,placeSeen,mineSeen,aimHit;
 private static volatile float woundedHealth;
 private static UUID aimTarget,freezeTarget;
 private static volatile int slowStart,slowEnd,resumeStart,resumeEnd;
 private static boolean freezeSeen,pauseOnLostFocus;
 static boolean backgroundPad(int id){return running&&(powers||leaper)&&pad!=null&&pad.id==id;}
 private static int darkScreenshot,shieldScreenshot,wingsScreenshot,betweenPackets,flightJump;
 private static Vec3 lastCamera;
 private static long cameraSnapshot;
 public static void sampleCamera(Camera camera){
  if(!running||!powers||tick<410||tick>=440)return;
  var player=Minecraft.getInstance().player;
  var pose=player==null?null:JakPoses.get(player.getId());
  if(pose==null)return;
  var at=camera.position();
  if(lastCamera!=null&&pose.received()==cameraSnapshot&&at.distanceToSqr(lastCamera)>1e-8)betweenPackets++;
  lastCamera=at;
  cameraSnapshot=pose.received();
 }
 public static void startPowers(Minecraft c){
  start(c);
  powers=true;
  pauseOnLostFocus=c.options.pauseOnLostFocus;
  c.options.pauseOnLostFocus=false;
  output=c.gameDirectory.toPath().resolve("jakmc/evidence/powers");
  try{Files.createDirectories(output);}catch(Exception e){throw new RuntimeException(e);}
  hit=shieldTested=darkSeen=lightSeen=wingsSeen=shotSeen=aimSeen=placeSeen=mineSeen=aimHit=flightSeen=healSeen=false;
  woundedHealth=0;
  freezeSeen=false;
  slowStart=slowEnd=resumeStart=resumeEnd=-1;
  freezeTarget=null;
  darkScreenshot=shieldScreenshot=wingsScreenshot=betweenPackets=flightJump=0;
  lastCamera=null;
  onServer(c,p->{
   p.setItemInHand(net.minecraft.world.InteractionHand.MAIN_HAND,new ItemStack(JakItems.GUNS.get(3)));
   p.setAttached(JakData.DARK_ECO,100f);
   p.setAttached(JakData.LIGHT_ECO,100f);
   var mob=dev.jakmc.entity.JakEntities.TYPES.get("grunt")
       .create(p.level(),net.minecraft.world.entity.EntitySpawnReason.COMMAND);
   mob.setNoAi(true);
   mob.setPos(start.add(0,0,5));
   p.level().addFreshEntity(mob);
   aimTarget=mob.getUUID();
  });
 }
 private static void onServer(Minecraft c,java.util.function.Consumer<net.minecraft.server.level.ServerPlayer> action){
  var server=c.getSingleplayerServer();var id=c.player.getUUID();
  server.execute(()->{var player=server.getPlayerList().getPlayer(id);if(player!=null)action.accept(player);});
 }
 private static void powerActions(Minecraft c){
  int t=++tick;
  var pose=JakPoses.get(c.player.getId());
  pad.axis(4,t>=18&&t<83?1:-1);
  pad.axis(5,t>=40&&t<56||t>=78&&t<80||t>=95&&t<98?1:-1);
  pad.axis(1,t>=402&&t<445?-.75f:0);
  pad.button(SDLGamepad.SDL_GAMEPAD_BUTTON_RIGHT_STICK,t>=109&&t<119||t>=149&&t<159||t>=195&&t<280||t>=324&&t<361||t>=475&&t<530||t>=620&&t<690);
  pad.button(SDLGamepad.SDL_GAMEPAD_BUTTON_EAST,t>=235&&t<285);
  if(flightJump==0&&t>363&&t<445&&pose!=null&&pose.light()
      &&pose.raw().get("wings").isJsonObject()
      &&Set.of("stance","walk").contains(pose.raw().get("state").getAsString()))flightJump=t;
  int sinceJump=flightJump==0?-1:t-flightJump;
  pad.button(SDLGamepad.SDL_GAMEPAD_BUTTON_SOUTH,t==342
      ||sinceJump>=0&&sinceJump<3||sinceJump>=7&&sinceJump<19);
  pad.button(SDLGamepad.SDL_GAMEPAD_BUTTON_NORTH,t>=498&&t<502);
  pad.button(SDLGamepad.SDL_GAMEPAD_BUTTON_WEST,t>=653&&t<657);
  if(t==13){c.player.setYRot(0);c.player.setXRot(4);c.gui.hud.getChat().clearMessages(false);}
  if(t==52)capture(c,"powers-aim.png");
  if(t==60)onServer(c,p->{
   var target=p.level().getEntity(aimTarget);
   aimHit=target instanceof net.minecraft.world.entity.LivingEntity living&&living.getHealth()<living.getMaxHealth();
   if(target!=null)target.discard();
  });
  if(t==64){
   c.player.setXRot(45);
   onServer(c,p->p.setItemInHand(net.minecraft.world.InteractionHand.MAIN_HAND,new ItemStack(net.minecraft.world.item.Items.GOLD_BLOCK)));
  }
  if(t==185)onServer(c,p->p.setGameMode(net.minecraft.world.level.GameType.SURVIVAL));
  if(t==91){
   onServer(c,p->{
    var base=net.minecraft.core.BlockPos.containing(start);
    for(int z=1;z<=3;z++)placeSeen|=p.level().getBlockState(base.offset(0,0,z)).is(net.minecraft.world.level.block.Blocks.GOLD_BLOCK);
    p.setItemInHand(net.minecraft.world.InteractionHand.MAIN_HAND,new ItemStack(net.minecraft.world.item.Items.DIAMOND_PICKAXE));
   });
  }
  if(t==103){
   onServer(c,p->{
    var base=net.minecraft.core.BlockPos.containing(start);
    mineSeen=placeSeen;
    for(int z=1;z<=3;z++)mineSeen&=!p.level().getBlockState(base.offset(0,0,z)).is(net.minecraft.world.level.block.Blocks.GOLD_BLOCK);
    p.setItemInHand(net.minecraft.world.InteractionHand.MAIN_HAND,ItemStack.EMPTY);
   });
   c.player.setXRot(5);
  }
  if(pose!=null){
   aimSeen|=t<64&&pose.raw().get("aim").getAsBoolean();
   shotSeen|=pose.raw().getAsJsonObject("gun").get("fireAge").getAsFloat()<1;
   if(pose.dark()&&t<109)ACTIONS.addProperty("l2_triggered_dark",true);
   if(pose.dark()&&t>=119&&t<149){
    darkSeen=true;
    if(darkScreenshot==0)darkScreenshot=t+3;
   }
   if(t==darkScreenshot)capture(c,"powers-dark.png");
   lightSeen|=pose.light();
   if(!freezeSeen&&pose.raw().get("entityRate").getAsFloat()<.95f){
    freezeSeen=true;
    capture(c,"powers-freeze.png");
   }
   String state=pose.raw().get("state").getAsString();
   if(t>363&&t<445&&t%4==0)STATES.add(t+":"+state);
   if(!flightSeen&&state.startsWith("lightjak-swoop")&&pose.position().y>start.y+.4){
    flightSeen=true;
    capture(c,"powers-flight.png");
   }
   if(!healSeen&&state.equals("lightjak-regen")){
    healSeen=true;
    capture(c,"powers-heal.png");
   }
   if(pose.raw().get("shield").getAsBoolean()&&!shieldTested){
    shieldTested=true;
    shieldScreenshot=t+10;
    onServer(c,p->{
     float before=p.getHealth();
     boolean damage=p.hurtServer(p.level(),p.damageSources().generic(),4);
     hit=!damage&&before==p.getHealth();
    });
   }
   if(t==shieldScreenshot)capture(c,"powers-shield.png");
   if(pose.raw().get("shieldHitAge").getAsFloat()<1)ACTIONS.addProperty("shield_reacted",true);
   if(pose.raw().get("wings").isJsonObject()&&!wingsSeen){
    wingsSeen=true;
    wingsScreenshot=t+22;
   }
  }
  if(t==wingsScreenshot)capture(c,"powers-wings.png");
  if(t==460)onServer(c,p->{
   p.getFoodData().setFoodLevel(12);
   p.setHealth(p.getMaxHealth()-6);
   woundedHealth=p.getHealth();
  });
  if(t==605)onServer(c,p->{
   var mob=dev.jakmc.entity.JakEntities.TYPES.get("grunt").create(p.level(),net.minecraft.world.entity.EntitySpawnReason.COMMAND);
   mob.setNoAi(true);
   mob.setPos(p.position().add(0,0,4));
   if(p.level().addFreshEntity(mob))freezeTarget=mob.getUUID();
  });
  if(t==720||t==780||t==1005||t==1045)onServer(c,p->{
   var mob=freezeTarget==null?null:p.level().getEntity(freezeTarget);
   if(mob==null)return;
   if(t==720)slowStart=mob.tickCount;
   else if(t==780)slowEnd=mob.tickCount;
   else if(t==1005)resumeStart=mob.tickCount;
   else {resumeEnd=mob.tickCount;mob.discard();}
  });
  if(t>=1051){
   STATES.add("freeze_entity_ticks="+(slowEnd-slowStart));
   STATES.add("resumed_entity_ticks="+(resumeEnd-resumeStart));
   ACTIONS.addProperty("light_freeze_slows_entities",freezeSeen&&slowStart>=0&&slowEnd-slowStart>=2&&slowEnd-slowStart<=5);
   ACTIONS.addProperty("light_freeze_recovers",resumeStart>=0&&resumeEnd-resumeStart>=34);
   ACTIONS.addProperty("l2_only_aims",!ACTIONS.has("l2_triggered_dark")&&aimSeen);
   ACTIONS.addProperty("r2_fired",shotSeen&&aimHit);
   ACTIONS.addProperty("r2_mined_native_block",mineSeen);
   ACTIONS.addProperty("r2_placed_native_block",placeSeen);
   ACTIONS.addProperty("r3_dark",darkSeen);
   ACTIONS.addProperty("r3_chords_light",lightSeen&&wingsSeen);
   ACTIONS.addProperty("light_flight",flightSeen);
   ACTIONS.addProperty("light_heals_survival",healSeen&&woundedHealth>0&&c.player.getHealth()>woundedHealth);
   ACTIONS.addProperty("shield_blocked_damage",shieldTested&&hit&&ACTIONS.has("shield_reacted"));
   ACTIONS.addProperty("camera_moves_between_packets",betweenPackets>4);
   finish(c,ACTIONS.entrySet().stream().allMatch(e->e.getValue().getAsBoolean())?"complete":"power/aim assertion failed");
  }
 }
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
  if(powers){powerActions(c);return;}
  if(leaper){leaperActions(c);return;}
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
  if(powers||leaper)c.options.pauseOnLostFocus=pauseOnLostFocus;
  running=false;view=null;if(pad!=null){pad.close();pad=null;}var result=new JsonObject();result.addProperty("reason",reason);result.addProperty("walk_blocks",walk);result.addProperty("jump_peak_blocks",peak);result.addProperty("camera_yaw_delta",yawAfter-yawStart);result.addProperty("states",String.join(",",STATES));result.addProperty("visual_review","required");if(powers){result.add("actions",ACTIONS.deepCopy());result.addProperty("moving_frames_between_pose_packets",betweenPackets);}if(interactions||leaper){result.add("actions",ACTIONS.deepCopy());if(ACTIONS.entrySet().stream().anyMatch(e->!e.getValue().getAsBoolean()))result.addProperty("reason","interaction assertion failed");}
  result.addProperty("jak_actions_reached",STATES.containsAll(Set.of("double-jump","roll","attack","board-stance")));
  try{Files.writeString(output.resolve("result.json"),result.toString());}catch(Exception e){throw new RuntimeException(e);}
  if(c.getSingleplayerServer()!=null){var server=c.getSingleplayerServer();server.execute(()->{for(UUID id:ITEMS){var item=server.overworld().getEntity(id);if(item!=null)item.discard();}});}c.stop();
 }
}
