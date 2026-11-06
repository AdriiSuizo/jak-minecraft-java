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
import dev.jakmc.client.model.JakRenderer;
import java.nio.file.*;
import java.util.*;

/** Replay through real SDL Ã¢â€ â€™ normal client input Ã¢â€ â€™ packets Ã¢â€ â€™ server Jak; no probeInput. */
public final class InputScene {
 private static boolean running,interactions,powers,leaper,hellcat,darkMotion;private static final JsonObject ACTIONS=new JsonObject();private static final Set<CameraType> VIEWS=new HashSet<>();private static int tick;private static SdlVirtualPad pad;private static Path output;
 private static long deviceDeadline;private static Vec3 start,ecoCenter;private static double peak,walk;private static float yawStart,yawAfter;
 private static long clothStart;
 private static float clothMin,clothMax;
 private static volatile boolean stageReady;
 private static float recordedStick;
 private static final Set<String> STATES=new TreeSet<>();private static final Set<UUID> ITEMS=new HashSet<>();
 public record CameraView(Vec3 position,float yaw,float pitch){}
 private static CameraView view;
 public static CameraView camera(){
  if(IntegrationScene.camera()!=null)return IntegrationScene.camera();
  if(view!=null)return view;
  if(EconomyScene.camera()!=null)return EconomyScene.camera();
  if(RulesScene.camera()!=null)return RulesScene.camera();
  if(LandsScene.camera()!=null)return LandsScene.camera();
  if(PunchScene.camera()!=null)return PunchScene.camera();
  if(BenchScene.camera()!=null)return BenchScene.camera();
  if(WreckScene.camera()!=null)return WreckScene.camera();
  if(ScaleScene.camera()!=null)return ScaleScene.camera();
  if(SitesScene.camera()!=null)return SitesScene.camera();
  if(CityScene.camera()!=null)return CityScene.camera();
  if(SpydroidScene.camera()!=null)return SpydroidScene.camera();
  if(KermitScene.camera()!=null)return KermitScene.camera();
  if(LoteScene.camera()!=null)return LoteScene.camera();
  if(Lote2Scene.camera()!=null)return Lote2Scene.camera();
  if(Lote3Scene.camera()!=null)return Lote3Scene.camera();
  if(Lote4Scene.camera()!=null)return Lote4Scene.camera();
  if(TreeScene.camera()!=null)return TreeScene.camera();
  if(Lote5Scene.camera()!=null)return Lote5Scene.camera();
  if(VentScene.camera()!=null)return VentScene.camera();
  if(CarScene.camera()!=null)return CarScene.camera();
  var enemy=EnemyScene.camera(Minecraft.getInstance());
  if(enemy!=null)return enemy;
  var yakow=YakowScene.camera();
  if(yakow!=null)return yakow;
  var surface=SurfaceScene.camera();
  if(surface!=null)return surface;
  var ladder=LadderScene.camera();
  if(ladder!=null)return ladder;
  var track=TrackScene.camera();
  if(track!=null)return track;
  var altar=AltarScene.camera();
  return altar!=null?altar:MaterialScene.camera();
 }
 public static void initialize(){ClientTickEvents.END_CLIENT_TICK.register(InputScene::tick);}
 public static void start(Minecraft c){start(c,false);}
 private static void start(Minecraft c,boolean outdoor){start(c,outdoor,c.player.position());}
 private static void start(Minecraft c,boolean outdoor,Vec3 seed){
  if(running||c.player==null||c.getSingleplayerServer()==null)throw new IllegalStateException("Open the isolated dev world");
  output=c.gameDirectory.toPath().resolve("jakmc/evidence/input");try{Files.createDirectories(output);}catch(Exception e){throw new RuntimeException(e);}
  interactions=powers=leaper=hellcat=darkMotion=false;ACTIONS.entrySet().clear();VIEWS.clear();
  start=seed.add(7,0,0);tick=0;peak=0;walk=0;STATES.clear();ITEMS.clear();view=null;ecoCenter=null;
  clothStart=dev.jakmc.client.model.JakRenderer.clothRenderSerial;
  clothMin=Float.POSITIVE_INFINITY;clothMax=0;
  stageReady=false;recordedStick=0;
  c.gui.setScreen(null);c.options.setCameraType(CameraType.THIRD_PERSON_BACK);c.player.setYRot(0);c.player.setXRot(10);yawStart=0;yawAfter=0;
  pad=new SdlVirtualPad();deviceDeadline=System.nanoTime()+15_000_000_000L;running=true;var uuid=c.player.getUUID();c.getSingleplayerServer().execute(()->{
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
   JakPlayers.controlForProbe(p,false);
   p.teleportTo(start.x,start.y,start.z);
   p.level().getChunkSource().move(p);p.setYRot(0);p.setXRot(10);p.setGameMode(net.minecraft.world.level.GameType.CREATIVE);p.setAttached(JakData.ENABLED,true);p.setAttached(JakData.BOARD,new ItemStack(JakItems.BOARD));p.setItemInHand(net.minecraft.world.InteractionHand.MAIN_HAND,ItemStack.EMPTY);
   stageReady=true;
  });
 }
 public static void startInteractions(Minecraft c){start(c,true);c.player.getInventory().setSelectedSlot(0);interactions=true;output=c.gameDirectory.toPath().resolve("jakmc/evidence/input_actions");try{Files.createDirectories(output);}catch(Exception e){throw new RuntimeException(e);}var uuid=c.player.getUUID();c.getSingleplayerServer().execute(()->{var p=c.getSingleplayerServer().getPlayerList().getPlayer(uuid);if(p!=null){p.getInventory().setSelectedSlot(0);p.setAttached(JakData.BOARD,ItemStack.EMPTY);p.setItemInHand(net.minecraft.world.InteractionHand.MAIN_HAND,new ItemStack(JakItems.BOARD));}});}
 private static volatile int hellcatId;
 private static boolean crossing,crossingSample,hellDog;
 private static final long[] crossingFrames=new long[40000];
 private static int crossingFrameCount;
 private static long crossingLastFrame;
 private static final Set<String> crossingBiomes=new TreeSet<>(),crossingChunks=new HashSet<>();
 private static double crossingLowestClearance;
 private static Vec3 crossingLastPosition;
 private static int crossingStalledTicks,crossingStreak,crossingLongestStall,crossingMapTicks,crossingMissingMap;
 private static int crossingCameraFrames,crossingCameraSpikes,crossingBetweenPackets,crossingLargeSteps;
 private static long crossingCameraTime,crossingPoseReceived;
 private static Vec3 crossingCameraPosition,crossingPacketPosition;
 private static Vec3 crossingPresented;
 private static final com.google.gson.JsonArray crossingSpikes=new com.google.gson.JsonArray();
 private static final ArrayDeque<Double> crossingRecentSpeeds=new ArrayDeque<>();
 private static double crossingSpeedRatio;
 public static void crossingFrame(){
  if(!running||!crossingSample)return;
  long now=System.nanoTime();
  if(crossingLastFrame!=0&&crossingFrameCount<crossingFrames.length)crossingFrames[crossingFrameCount++]=now-crossingLastFrame;
  crossingLastFrame=now;
 }
 public static void startHellcatCrossing(Minecraft c,Vec3 origin){
  // This fixture measures sustained fast exploration, not the time spent
  // climbing from an arbitrary fresh world's spawn to cruising altitude.
  startHellcat(c,true,origin.with(net.minecraft.core.Direction.Axis.Y,Math.max(220,origin.y)),false);
  crossingSample=false;
  crossingFrameCount=crossingStalledTicks=crossingStreak=crossingLongestStall=crossingMapTicks=crossingMissingMap=0;
  crossingLastFrame=0;
  crossingLastPosition=null;
  crossingCameraFrames=crossingCameraSpikes=crossingBetweenPackets=crossingLargeSteps=0;
  crossingCameraTime=crossingPoseReceived=0;
  crossingCameraPosition=crossingPacketPosition=null;
  crossingPresented=null;
  crossingRecentSpeeds.clear();
  while(!crossingSpikes.isEmpty())crossingSpikes.remove(0);
  crossingSpeedRatio=0;
  crossingBiomes.clear();
  crossingChunks.clear();
  crossingLowestClearance=Double.POSITIVE_INFINITY;
 }

 private static int hellcatBoardPress,hellcatMountedAt,hellcatEntryImage,hellcatOffAt;
 private static boolean hellcatBoarding,hellcatNight,hellcatLeftPowered,hellcatKickedOff,hellcatShot,hellcatBoost,hellcatCircle;
 private static volatile boolean hellcatPlaced,hellcatSaved;
 private static Vec3 hellcatStart;
 private static double hellcatClimb,hellcatHalfThrottle,hellcatFullThrottle;
 public static void startHellcat(Minecraft c,boolean dog){startHellcat(c,false,c.player.position(),dog);}
 private static void startHellcat(Minecraft c,boolean acrossBiomes,Vec3 origin,boolean dog){
  start(c,true,origin);
  hellcat=true;
  crossing=acrossBiomes;hellDog=dog;
  crossingSample=false;
  hellcatId=-1;
  hellcatBoardPress=hellcatMountedAt=hellcatEntryImage=hellcatOffAt=0;
  hellcatBoarding=hellcatNight=hellcatLeftPowered=hellcatKickedOff=hellcatShot=hellcatBoost=hellcatCircle=false;
  hellcatPlaced=hellcatSaved=false;
  hellcatStart=null;
  hellcatClimb=hellcatHalfThrottle=hellcatFullThrottle=0;
  pauseOnLostFocus=c.options.pauseOnLostFocus;c.options.pauseOnLostFocus=false;
  output=c.gameDirectory.toPath().resolve("jakmc/evidence/"+(crossing?"hellcat_crossing":dog?"helldog":"hellcat"));
  try{Files.createDirectories(output);}catch(Exception e){throw new RuntimeException(e);}
  onServer(c,p->{
   var server=p.level().getServer();
   server.getCommands().performPrefixedCommand(server.createCommandSourceStack(),crossing?"time set noon":"time set midnight");
   if(crossing){
    var craft=dev.jakmc.entity.JakHellcat.TYPE.create(p.level(),net.minecraft.world.entity.EntitySpawnReason.COMMAND);
    craft.setPos(start.add(0,1.28,4.2));
    if(p.level().addFreshEntity(craft)){
     ITEMS.add(craft.getUUID());hellcatId=craft.getId();
    }
    return;
   }
   var hand=net.minecraft.world.InteractionHand.MAIN_HAND;
   var floor=net.minecraft.core.BlockPos.containing(start).offset(0,-1,4);
   var at=Vec3.atBottomCenterOf(floor.above());
   var hit=new net.minecraft.world.phys.BlockHitResult(at,net.minecraft.core.Direction.UP,floor,false);
   var item=dog?dev.jakmc.entity.JakHellcat.DOG_ITEM:dev.jakmc.entity.JakHellcat.ITEM;
   p.setItemInHand(hand,new ItemStack(item));
   var context=new net.minecraft.world.item.context.UseOnContext(p,hand,hit);
   hellcatPlaced=item.useOn(context).consumesAction();
   p.setItemInHand(hand,ItemStack.EMPTY);
   for(var craft:p.level().getEntitiesOfClass(dev.jakmc.entity.JakHellcat.class,
       new net.minecraft.world.phys.AABB(at,at).inflate(2),e->true)){
    ITEMS.add(craft.getUUID());hellcatId=craft.getId();
   }
  });
 }
 private static void hellcatActions(Minecraft c){
  int t=++tick;
  var e=c.level.getEntity(hellcatId);
  if(!(e instanceof dev.jakmc.entity.JakHellcat cat)){
   if(t>=220)finish(c,"HellCat entity missing");
   return;
  }
  view=null;
  var pose=JakPoses.get(c.player.getId());
  var channel=pose!=null&&pose.raw().get("hellcat") instanceof JsonObject flight?flight:null;
  if(t==14)c.gui.hud.getChat().clearMessages(false);
  if(t==16)capture(c,crossing?"hellcat_crossing-idle.png":"hellcat-idle.png");
  if(t>=20&&channel==null&&hellcatMountedAt==0){
   aimAt(c,cat.position().add(0,.55,0));
   if((hellcatBoardPress==0||t>hellcatBoardPress+10)&&t<90
    &&c.hitResult instanceof net.minecraft.world.phys.EntityHitResult hit&&hit.getEntity()==cat)hellcatBoardPress=t+1;
  }
  if(channel!=null){
   if(channel.get("boarding").getAsFloat()>=0){
    if(!hellcatBoarding)hellcatEntryImage=t+4;
    hellcatBoarding=true;
   }else if(hellcatMountedAt==0){
    hellcatMountedAt=t;
    hellcatStart=cat.position();
   }
  }
  int ride=hellcatMountedAt==0?-1:t-hellcatMountedAt;
  if(crossing&&ride>=0){hellcatCrossingActions(c,cat,channel,ride);return;}
  if(crossing&&t>=120){finish(c,"HellCat did not board");return;}
  if(ride>=65&&channel==null&&hellcatOffAt==0)hellcatOffAt=t;
  int off=hellcatOffAt==0?-1:t-hellcatOffAt;
  if(t==hellcatEntryImage)capture(c,"hellcat-entry.png");
  pad.axis(5,ride>=10&&ride<27?0:ride>=27&&ride<65?1:-1);
  pad.axis(1,ride>=28&&ride<40?.75f:0);
  pad.button(SDLGamepad.SDL_GAMEPAD_BUTTON_SOUTH,ride>=51&&ride<55);
  pad.button(SDLGamepad.SDL_GAMEPAD_BUTTON_EAST,ride>=43&&ride<48||off>=17&&off<22||off>=32&&off<37);
  pad.button(SDLGamepad.SDL_GAMEPAD_BUTTON_RIGHT_SHOULDER,ride>=58&&ride<63);
  pad.axis(4,hellcatBoardPress>0&&t>=hellcatBoardPress&&t<hellcatBoardPress+3&&channel==null?1:-1);
  pad.button(SDLGamepad.SDL_GAMEPAD_BUTTON_NORTH,channel!=null&&ride>=69&&ride<105&&(ride-69)%12<5);
  if(channel!=null){
   hellcatNight|=cat.lamps()>.95;
   if(ride>=0){
    walk=Math.max(walk,cat.position().subtract(hellcatStart).horizontalDistance());
    hellcatClimb=Math.max(hellcatClimb,cat.getY()-hellcatStart.y);
    if(ride>=52&&ride<56)hellcatBoost|=channel.get("boost").getAsBoolean();
    if(ride>=44&&ride<48)hellcatCircle|=!channel.get("boost").getAsBoolean();
    double throttle=channel.get("throttle").getAsDouble();
    if(ride>=13&&ride<26)hellcatHalfThrottle=Math.max(hellcatHalfThrottle,throttle);
    if(ride>=39&&ride<62)hellcatFullThrottle=Math.max(hellcatFullThrottle,throttle);
   }
  }
  if(ride>=32&&ride<=36){
   var at=cat.position();
   look(at.add(6,3,4),at.add(0,.7,0));
   if(ride==36)capture(c,"hellcat-flight.png");
  }
  if(off>=3&&off<=15&&channel==null){
   hellcatLeftPowered|=cat.lamps()>.95&&cat.thrust()>0;
   var at=cat.position();
   look(at.add(5,2.7,-4),at);
   if(off==8)capture(c,"hellcat-left-powered.png");
  }
  pad.button(SDLGamepad.SDL_GAMEPAD_BUTTON_WEST,false);
  if(off>=16&&off<=45&&channel==null){
   aimAt(c,cat.position().subtract(Vec3.directionFromRotation(0,cat.getYRot()).scale(.65)));
   var at=cat.position();
   look(at.add(5,2.7,-4),at);
   hellcatKickedOff|=cat.lamps()<.05&&cat.thrust()<.05;
   if(off==42)capture(c,"hellcat-kicked.png");
  }
  if(off==14)onServer(c,p->{
   if(p.level().getEntity(hellcatId) instanceof dev.jakmc.entity.JakHellcat target){
    var reporter=net.minecraft.util.ProblemReporter.DISCARDING;
    var output=net.minecraft.world.level.storage.TagValueOutput.createWithContext(reporter,target.registryAccess());
    target.saveWithoutId(output);
    var saved=output.buildResult();
    var copy=dev.jakmc.entity.JakHellcat.TYPE.create(p.level(),net.minecraft.world.entity.EntitySpawnReason.LOAD);
    copy.load(net.minecraft.world.level.storage.TagValueInput.create(reporter,copy.registryAccess(),saved));
    hellcatSaved=copy.pilotId()<0&&copy.position().distanceToSqr(target.position())<.001&&saved.getIntOr("HullFlags",0)==2;
   }
  });
  if(t>=235){
   ACTIONS.addProperty("item_placed",hellcatPlaced);
   ACTIONS.addProperty("native_save_roundtrip",hellcatSaved);
   ACTIONS.addProperty("actual_entry_clip",hellcatBoarding&&hellcatMountedAt>0);
   ACTIONS.addProperty("original_variant",cat.hellDog()==hellDog);
   ACTIONS.addProperty("analog_throttle",hellcatHalfThrottle>.3&&hellcatHalfThrottle<.65&&hellcatFullThrottle>.9);
   ACTIONS.addProperty("flight",walk>6&&hellcatClimb>1);
   ACTIONS.addProperty("night_lamps",hellcatNight);
   ACTIONS.addProperty("air_dismount_powered",hellcatLeftPowered);
   ACTIONS.addProperty("kick_extinguishes",hellcatKickedOff);
   ACTIONS.addProperty("nose_gun",hellcatShot);
   ACTIONS.addProperty("cross_boost",hellcatBoost);
   ACTIONS.addProperty("circle_does_not_boost",hellcatCircle);
   peak=hellcatClimb;
   STATES.add("dismount="+hellcatOffAt);
   STATES.add("half="+hellcatHalfThrottle);
   STATES.add("full="+hellcatFullThrottle);
   finish(c,ACTIONS.entrySet().stream().allMatch(a->a.getValue().getAsBoolean())?"complete":"HellCat assertion failed");
  }
 }
 private static void hellcatCrossingActions(Minecraft c,dev.jakmc.entity.JakHellcat cat,JsonObject channel,int ride){
  if(channel==null){finish(c,"Lost the HellCat during the crossing");return;}
  if(ride==0){
   pad.button(SDLGamepad.SDL_GAMEPAD_BUTTON_NORTH,false);pad.axis(4,-1);
   c.gui.hud.getChat().clearMessages(false);
  }
  var pos=cat.position();
  var world=c.level;
  var heightType=net.minecraft.world.level.levelgen.Heightmap.Types.MOTION_BLOCKING_NO_LEAVES;
  var bp=cat.blockPosition();
  int floor=world.getHeight(heightType,bp.getX(),bp.getZ());
  int nextFloor=world.getHeight(heightType,bp.getX(),bp.getZ()+25);
  var heading=cat.attitude().transform(new org.joml.Vector3f(0,0,1));
  double target=Math.max(220,Math.max(floor,nextFloor)+48);
  float desired=(float)Math.clamp((target-pos.y)*.024,-.25,.31);
  float error=desired-heading.y;
  pad.axis(1,Math.abs(error)<.025f?0:Math.copySign(Math.clamp(.3f+Math.abs(error)*2,0,.9f),error));
  pad.axis(5,pos.y<175?-.25f:1);
  pad.button(SDLGamepad.SDL_GAMEPAD_BUTTON_SOUTH,pos.y>=175&&ride>=65&&ride<580);
  if(ride==65){crossingSample=true;crossingLastFrame=0;}
  if(ride>65){
   crossingMapTicks++;if(!JakMinimap.terrainVisible())crossingMissingMap++;
   crossingLowestClearance=Math.min(crossingLowestClearance,pos.y-floor);
   if(crossingLastPosition!=null&&pos.distanceToSqr(crossingLastPosition)<.0004){
    crossingStalledTicks++;
    crossingLongestStall=Math.max(crossingLongestStall,++crossingStreak);
   }else crossingStreak=0;
   crossingLastPosition=pos;
   walk=Math.max(walk,pos.subtract(hellcatStart).horizontalDistance());
   if(ride%4==0&&world.hasChunkAt(bp)){
    crossingChunks.add((bp.getX()>>4)+","+(bp.getZ()>>4));
    var ground=new net.minecraft.core.BlockPos(bp.getX(),floor+4,bp.getZ());
    world.getBiome(ground).unwrapKey().ifPresent(key->crossingBiomes.add(key.identifier().toString()));
   }
  }
  if(ride==360||ride==620)capture(c,"hellcat_crossing-"+ride+".png");
  if(ride>=680){
   crossingSample=false;
   java.util.Arrays.sort(crossingFrames,0,crossingFrameCount);
   if(crossingFrameCount>0){
    STATES.add("frame_p50_ms="+crossingFrames[crossingFrameCount/2]/1e6);
    STATES.add("frame_p95_ms="+crossingFrames[(int)((crossingFrameCount-1)*.95)]/1e6);
    STATES.add("frame_p99_ms="+crossingFrames[(int)((crossingFrameCount-1)*.99)]/1e6);
    STATES.add("frame_max_ms="+crossingFrames[crossingFrameCount-1]/1e6);
   }
   STATES.add("biomes="+String.join("|",crossingBiomes));
   STATES.add("chunks="+crossingChunks.size());
   STATES.add("lowest_clearance="+crossingLowestClearance);
   STATES.add("stationary_ticks="+crossingStalledTicks);
   STATES.add("map_ticks="+crossingMapTicks);
   STATES.add("map_missing_ticks="+crossingMissingMap);
   STATES.add("longest_stall_ticks="+crossingLongestStall);
   STATES.add("rendered_camera_frames="+crossingCameraFrames);
   STATES.add("camera_frames_between_packets="+crossingBetweenPackets);
   STATES.add("camera_speed_spikes="+crossingCameraSpikes);
   STATES.add("camera_max_speed_ratio="+crossingSpeedRatio);
   STATES.add("snapshots_over_four_blocks="+crossingLargeSteps);
   STATES.add("camera_spike_samples="+crossingSpikes);
   ACTIONS.addProperty("kept_flight",walk>900&&crossingLongestStall<35);
   ACTIONS.addProperty("multiple_biomes",crossingBiomes.size()>=3);
   ACTIONS.addProperty("minimap_kept_up",crossingMapTicks>500&&crossingMissingMap==0);
   ACTIONS.addProperty("crossed_chunks",crossingChunks.size()>=50);
   ACTIONS.addProperty("stayed_above_surface",crossingLowestClearance>0);
   ACTIONS.addProperty("continuous_camera",crossingCameraFrames>150&&crossingBetweenPackets>100&&crossingCameraSpikes==0);
   finish(c,ACTIONS.entrySet().stream().allMatch(a->a.getValue().getAsBoolean())?"complete":"Crossing assertion failed");
  }
 }
 public static void observeHellcatEvent(String kind){
  if(running&&hellcat&&kind.equals("fire"))hellcatShot=true;
 }
 private static UUID leaperId;
 private static boolean flut;
 private static volatile boolean tameSaddle,eggPlaced,eggHatched;
 private static Vec3 eggCenter;
 private static volatile int leaperEntity;
 private static boolean mountSeen,offSeen;
 private static int rideStart,jumpStart,offPress,slamImage,mountPress;
 private static volatile boolean gateOpened,mountDamaged,mountSaved;
 private static boolean hitRequested;
 private static net.minecraft.core.BlockPos gate;
 private static double rideTravel;
 private static Vec3 mountStart;
 public static void startLeaper(Minecraft c){startLeaper(c,false);}
 public static void startFlut(Minecraft c){startLeaper(c,true);}
 private static void startLeaper(Minecraft c,boolean flutTest){
  start(c,true);
  leaper=true;
  flut=flutTest;
  tameSaddle=eggPlaced=eggHatched=false;
  pauseOnLostFocus=c.options.pauseOnLostFocus;
  c.options.pauseOnLostFocus=false;
  output=c.gameDirectory.toPath().resolve("jakmc/evidence/"+(flut?"flut":"leaper"));
  try{Files.createDirectories(output);}catch(Exception e){throw new RuntimeException(e);}
  mountSeen=offSeen=false;
  rideStart=jumpStart=offPress=slamImage=mountPress=0;
  gateOpened=mountDamaged=mountSaved=hitRequested=false;
  rideTravel=0;
  leaperEntity=-1;
  leaperId=null;
  onServer(c,p->{
   var type=flut?dev.jakmc.entity.JakFlutFlut.TYPE:dev.jakmc.entity.JakLeaper.TYPE;
   var mount=type.create(p.level(),net.minecraft.world.entity.EntitySpawnReason.COMMAND);
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
  if(t==25)mountCapture(c,"idle");
  if(flut&&t==36)onServer(c,p->flutSetup(p));
  if(flut&&eggCenter!=null&&(t>=53&&t<=56||t>=118&&t<=121)){
   look(eggCenter.add(1.8,1.35,1.5),eggCenter.add(0,.35,0));
   if(t==56)mountCapture(c,"egg");
   if(t==121)mountCapture(c,"chick");
  }
  if(t>(flut?42:25)&&t<85&&!mountSeen&&target!=null){
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
   mountCapture(c,"mount");
  }
  if(rideStart==0&&state.equals("target-flut-stance")){
   rideStart=t;
   mountStart=pose.position();
  }
  int rideAge=rideStart==0?-1:t-rideStart;
  if(rideAge>=15&&rideAge<=18)mountView(pose.position());
  if(rideAge==18)mountCapture(c,"riding");
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
  if(jumpAge==19)mountCapture(c,"jump");
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
  boolean nativeUse=mountPress>0&&t>=mountPress&&t<mountPress+4||rideAge>=21&&rideAge<23;
  pad.axis(4,nativeUse?1:-1);
  pad.button(SDLGamepad.SDL_GAMEPAD_BUTTON_NORTH,offPress>0&&t>=offPress&&t<offPress+4);
  if(slamImage>0&&t>=slamImage-1&&t<=slamImage&&target!=null)mountView(target.position());
  if(t==slamImage)mountCapture(c,"slam");
  if(offPress>0&&t>offPress&&!mounted&&mountSeen&&!offSeen){
   offSeen=true;
   onServer(c,p->{
    if(p.level().getEntity(leaperId) instanceof dev.jakmc.entity.JakLeaper mount){
     var reporter=net.minecraft.util.ProblemReporter.DISCARDING;
     var output=net.minecraft.world.level.storage.TagValueOutput.createWithContext(reporter,mount.registryAccess());
     mount.saveWithoutId(output);
     var copy=(flut?dev.jakmc.entity.JakFlutFlut.TYPE:dev.jakmc.entity.JakLeaper.TYPE).create(p.level(),net.minecraft.world.entity.EntitySpawnReason.LOAD);
     copy.load(net.minecraft.world.level.storage.TagValueInput.create(reporter,copy.registryAccess(),output.buildResult()));
     mountSaved=(!flut||copy instanceof dev.jakmc.entity.JakFlutFlut bird&&bird.tame()&&bird.saddled())&&copy.riderId()<0&&copy.getHealth()==mount.getHealth()&&copy.position().distanceToSqr(mount.position())<.001;
    }
   });
   mountCapture(c,"dismount");
  }
  if(flut&&t==118)onServer(c,p->{
   for(var chick:p.level().getEntitiesOfClass(dev.jakmc.entity.JakFlutFlut.class,
       new net.minecraft.world.phys.AABB(eggCenter,eggCenter).inflate(2),mob->mob.isBaby())){
    eggHatched=true;
    ITEMS.add(chick.getUUID());
   }
  });
  if(t>=(flut?325:265)){
   var entity=c.level.getEntity(leaperEntity);
   if(flut){
    ACTIONS.addProperty("taming_and_saddle",tameSaddle);
    ACTIONS.addProperty("egg_item_placed",eggPlaced);
    ACTIONS.addProperty("egg_hatched",eggHatched);
   }
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
 public static void observeMountEvents(int entity,JsonObject pose){
  var client=Minecraft.getInstance();
  if(!running||!leaper||client.player==null||entity!=client.player.getId()||ACTIONS.has("ground_attack"))return;
  for(var event:pose.getAsJsonArray("events")){
   if(event.getAsJsonObject().get("kind").getAsString().equals("flut_slam")){
    ACTIONS.addProperty("ground_attack",true);
    slamImage=tick+3;
    return;
   }
  }
 }
 private static void mountCapture(Minecraft c,String stage){capture(c,(flut?"flut":"leaper")+"-"+stage+".png");}
 private static void flutSetup(net.minecraft.server.level.ServerPlayer p){
  var hand=net.minecraft.world.InteractionHand.MAIN_HAND;
  if(!(p.level().getEntity(leaperId) instanceof dev.jakmc.entity.JakFlutFlut bird))return;
  p.setItemInHand(hand,new ItemStack(net.minecraft.world.item.Items.WHEAT_SEEDS,34));
  for(int i=0;i<34;i++)bird.mobInteract(p,hand);
  p.setItemInHand(hand,ItemStack.EMPTY);
  bird.mobInteract(p,hand);
  p.setItemInHand(hand,new ItemStack(net.minecraft.world.item.Items.SADDLE));
  bird.mobInteract(p,hand);
  tameSaddle=bird.readyToMount();

  var floor=net.minecraft.core.BlockPos.containing(start).offset(-3,-1,5);
  eggCenter=Vec3.atBottomCenterOf(floor.above());
  var hit=new net.minecraft.world.phys.BlockHitResult(eggCenter,net.minecraft.core.Direction.UP,floor,false);
  p.setItemInHand(hand,new ItemStack(dev.jakmc.entity.JakFlutEgg.ITEM));
  var context=new net.minecraft.world.item.context.UseOnContext(p,hand,hit);
  eggPlaced=dev.jakmc.entity.JakFlutEgg.ITEM.useOn(context).consumesAction();
  p.setItemInHand(hand,ItemStack.EMPTY);

  for(var egg:p.level().getEntitiesOfClass(dev.jakmc.entity.JakFlutEgg.class,
       new net.minecraft.world.phys.AABB(eggCenter,eggCenter).inflate(1),e->true)){
   ITEMS.add(egg.getUUID());
   var reporter=net.minecraft.util.ProblemReporter.DISCARDING;
   var output=net.minecraft.world.level.storage.TagValueOutput.createWithContext(reporter,egg.registryAccess());
   egg.saveWithoutId(output);
   output.putInt("FlutIncubation",dev.jakmc.entity.JakFlutEgg.HATCH-55);
   egg.load(net.minecraft.world.level.storage.TagValueInput.create(reporter,egg.registryAccess(),output.buildResult()));
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
 static int replayPad(){return running&&pad!=null?pad.id:0;}
 static boolean backgroundPad(int id){return CarScene.backgroundPad(id)||PunchScene.backgroundPad(id)||BenchScene.backgroundPad(id)||Lote3Scene.backgroundPad(id)||id!=0&&replayPad()==id;}
 private static int darkScreenshot,shieldScreenshot,wingsScreenshot,betweenPackets,flightJump;
 private static Vec3 lastCamera;
 private static long lastFrameNanos;
 private static jdk.jfr.Recording shieldRecording;
 @SuppressWarnings("unchecked") private static final java.util.List<Double>[] FRAME_MS=new java.util.List[]{new java.util.ArrayList<Double>(),new java.util.ArrayList<Double>(),new java.util.ArrayList<Double>()};
 private static long cameraSnapshot;
 public static void sampleCamera(Camera camera){
  if(running&&powers){
   long now=System.nanoTime();var player=Minecraft.getInstance().player;
   var pose=player==null?null:JakPoses.get(player.getId());
   if(lastFrameNanos!=0&&pose!=null){
    double ms=(now-lastFrameNanos)/1e6;
    int k=pose.raw().get("shield").getAsBoolean()?0:pose.light()?1:2;
    FRAME_MS[k].add(ms);
    if(k==0&&ms>40)STATES.add("spike="+String.format(java.util.Locale.ROOT,"%.0f@%d",ms,java.time.Instant.now().toEpochMilli()));
   }
   lastFrameNanos=now;
  }
  if(running&&crossing&&crossingSample){
   var player=Minecraft.getInstance().player;
   var pose=player==null?null:JakPoses.get(player.getId());
   if(pose!=null&&pose.raw().get("hellcat") instanceof JsonObject flight){
    long now=System.nanoTime();
    double seconds=(now-crossingCameraTime)/1e9;
    double speed=flight.get("speed").getAsDouble();
    if(crossingPoseReceived!=pose.received()){
     crossingRecentSpeeds.addLast(speed);
     while(crossingRecentSpeeds.size()>8)crossingRecentSpeeds.removeFirst();
    }
    // The view trails buffered samples. Include their speed when a newer
    // collision or brake packet has already slowed the authoritative hull.
    double referenceSpeed=speed;
    for(double recent:crossingRecentSpeeds)referenceSpeed=Math.max(referenceSpeed,recent);
    Vec3 at=camera.position();
    if(crossingCameraPosition!=null&&seconds>0&&seconds<.035&&speed>10){
     double ratio=at.distanceTo(crossingCameraPosition)/seconds/referenceSpeed;
     crossingSpeedRatio=Math.max(crossingSpeedRatio,ratio);
     crossingCameraFrames++;
     if(ratio>2){
      crossingCameraSpikes++;
      if(crossingSpikes.size()<12){
       var detail=new JsonObject();
       detail.addProperty("ratio",ratio);
       detail.addProperty("root_ratio",JakPoses.presented().distanceTo(crossingPresented)/seconds/referenceSpeed);
       detail.addProperty("speed",speed);
       detail.addProperty("dt",seconds);
       detail.addProperty("age_ms",(now-pose.received())/1e6);
       detail.addProperty("tick",pose.raw().get("serverTick").getAsLong());
       crossingSpikes.add(detail);
      }
     }
     if(pose.received()==crossingPoseReceived&&at.distanceToSqr(crossingCameraPosition)>1e-7)crossingBetweenPackets++;
    }
    if(crossingPoseReceived!=pose.received()){
     if(crossingPacketPosition!=null&&pose.position().distanceToSqr(crossingPacketPosition)>16)crossingLargeSteps++;
     crossingPacketPosition=pose.position();
    }
    crossingPoseReceived=pose.received();
    crossingCameraPosition=at;
    crossingPresented=JakPoses.presented();
    crossingCameraTime=now;
   }
  }
  if(!running||!powers||tick<410||tick>=440)return;
  var player=Minecraft.getInstance().player;
  var pose=player==null?null:JakPoses.get(player.getId());
  if(pose==null)return;
  var at=camera.position();
  if(lastCamera!=null&&pose.received()==cameraSnapshot&&at.distanceToSqr(lastCamera)>1e-8)betweenPackets++;
  lastCamera=at;
  cameraSnapshot=pose.received();
 }
 private static int darkStanceAt,darkRunShot,darkPress;
 public static void startDarkMotion(Minecraft c){
  start(c,true);
  darkMotion=true;
  darkStanceAt=darkRunShot=darkPress=0;
  pauseOnLostFocus=c.options.pauseOnLostFocus;
  c.options.pauseOnLostFocus=false;
  output=c.gameDirectory.toPath().resolve("jakmc/evidence/dark_motion");
  try{Files.createDirectories(output);}catch(Exception e){throw new RuntimeException(e);}
  onServer(c,p->dev.jakmc.player.JakResources.grant(p,0,Math.max(0,100-dev.jakmc.player.JakResources.totals(p)[0])));
 }
 private static void darkMotionActions(Minecraft c){
  int t=++tick;
  if(t==10)c.gui.hud.getChat().clearMessages(false);
  if(darkPress==0&&JakInput.controller().id()==pad.id&&c.gui.screen()==null){
   darkPress=t+8;
  }
  pad.axis(2,t>=5&&darkPress==0?.7f:0);
  if(t==darkPress-2)c.player.setYRot(0);
  var pose=JakPoses.get(c.player.getId());
  if(pose!=null&&pose.raw().get("darkPose") instanceof JsonObject program){
   String clip=program.get("clip").getAsString();
   STATES.add(clip);
   if(clip.equals("stance")&&darkStanceAt==0)darkStanceAt=t;
   if(clip.equals("run")&&darkRunShot==0){
    var target=pose.position().add(0,.9,0);
    look(target.add(3,1.2,2.4),target);
    darkRunShot=t+2;
   }
  }
  int age=darkStanceAt==0?-1:t-darkStanceAt;
  pad.button(SDLGamepad.SDL_GAMEPAD_BUTTON_LEFT_STICK,darkPress>0&&t>=darkPress&&t<darkPress+7);
  pad.axis(1,age>=2&&age<22?-1:0);
  pad.button(SDLGamepad.SDL_GAMEPAD_BUTTON_EAST,age==31);
  pad.button(SDLGamepad.SDL_GAMEPAD_BUTTON_SOUTH,age>=52&&age<55);
  if(darkRunShot>0&&t==darkRunShot)capture(c,"jak-dark-run.png");
  if(darkRunShot>0&&t==darkRunShot+1)view=null;
  if(age>=72||t>=145){
   if(darkStanceAt==0&&pose!=null)STATES.add("last:"+pose.clip()+":eco="+pose.raw().get("darkEco")
       +":dark="+pose.dark()+":device="+JakInput.controller().name()+":press="+darkPress);
   for(String clip:List.of("stance","run","attack-spin","jump"))
    ACTIONS.addProperty(clip,STATES.contains(clip));
   finish(c,ACTIONS.entrySet().stream().allMatch(e->e.getValue().getAsBoolean())?"complete":"Dark Jak motion assertion failed");
  }
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
  darkScreenshot=shieldScreenshot=wingsScreenshot=betweenPackets=flightJump=freezeFrames=0;clapShot=-1;for(var l:FRAME_MS)l.clear();lastFrameNanos=0;
  dev.jakmc.client.sky.JakFreezeScreen.frames=0;
  lastCamera=null;
  onServer(c,p->{
   p.setItemInHand(net.minecraft.world.InteractionHand.MAIN_HAND,new ItemStack(JakItems.GUNS.get(3)));
   dev.jakmc.player.JakResources.grant(p,0,Math.max(0,100-dev.jakmc.player.JakResources.totals(p)[0]));
   dev.jakmc.player.JakResources.grant(p,1,Math.max(0,100-dev.jakmc.player.JakResources.totals(p)[1]));
   // A fresh world has no ammo: some of each colour for the gun.
   for(int kind=2;kind<6;kind++)dev.jakmc.player.JakResources.grant(p,kind,Math.max(0,100-dev.jakmc.player.JakResources.totals(p)[kind]));
   var mob=dev.jakmc.entity.JakEntities.TYPES.get("grunt")
       .create(p.level(),net.minecraft.world.entity.EntitySpawnReason.COMMAND);
   mob.setNoAi(true);
   mob.setPos(start.add(0,0,5));
   p.level().addFreshEntity(mob);
   aimTarget=mob.getUUID();
  });
 }
 private static int goldPlaced,clapShot=-1,freezeFrames;
 private static int gold(net.minecraft.server.level.ServerPlayer p){
  var base=net.minecraft.core.BlockPos.containing(start);int n=0;
  for(int z=1;z<=3;z++)for(int y=0;y<3;y++)if(p.level().getBlockState(base.offset(0,y,z)).is(net.minecraft.world.level.block.Blocks.GOLD_BLOCK))n++;
  return n;
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
  pad.button(SDLGamepad.SDL_GAMEPAD_BUTTON_LEFT_STICK,t>=109&&t<119||t>=149&&t<159||t>=195&&t<280||t>=324&&t<361||t>=475&&t<530||t>=620&&t<690);
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
  // The shield's opening profiled (IW4L_SHIELD_JFR=1).
  if(t==200&&"1".equals(System.getenv("IW4L_SHIELD_JFR")))try{shieldRecording=new jdk.jfr.Recording(jdk.jfr.Configuration.getConfiguration("profile"));shieldRecording.enable("jdk.ExecutionSample").withPeriod(java.time.Duration.ofMillis(1));shieldRecording.enable("jdk.NativeMethodSample").withPeriod(java.time.Duration.ofMillis(1));shieldRecording.setToDisk(true);shieldRecording.start();}catch(Exception e){shieldRecording=null;}
  if(t==320&&shieldRecording!=null)try{shieldRecording.dump(c.gameDirectory.toPath().resolve("jakmc/evidence/powers/shield.jfr"));shieldRecording.close();shieldRecording=null;}catch(Exception e){shieldRecording=null;}
  // target-lightjak-freeze's clap and the slowed screen; target-lightjak-shield's move.
  String drawn=pose==null?"":pose.clip();
  if(drawn.equals("jakb-lightjak-shield"))ACTIONS.addProperty("shield_move_drawn",true);
  if(drawn.equals("jakb-lightjak-freeze")){
   if(!ACTIONS.has("freeze_clap_drawn"))clapShot=t+4;
   ACTIONS.addProperty("freeze_clap_drawn",true);
  }
  if(t==clapShot)capture(c,"powers-clap.png");
  if(t==700){freezeFrames=dev.jakmc.client.sky.JakFreezeScreen.frames;capture(c,"powers-freeze-screen.png");}
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
    goldPlaced=gold(p);placeSeen=goldPlaced>0;
    p.setItemInHand(net.minecraft.world.InteractionHand.MAIN_HAND,new ItemStack(net.minecraft.world.item.Items.DIAMOND_PICKAXE));
   });
  }
  if(t==103){
   onServer(c,p->{
    // One R2 press breaks one block; L2 may have stacked them.
    mineSeen=placeSeen&&gold(p)<goldPlaced;
    // The rest out of the way of the walk that follows.
    var base=net.minecraft.core.BlockPos.containing(start);
    for(int z=1;z<=3;z++)for(int y=0;y<3;y++)if(p.level().getBlockState(base.offset(0,y,z)).is(net.minecraft.world.level.block.Blocks.GOLD_BLOCK))p.level().removeBlock(base.offset(0,y,z),false);
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
   String[] kinds={"shield","light","plain"};
   for(int k=0;k<3;k++){var v=new java.util.ArrayList<>(FRAME_MS[k]);java.util.Collections.sort(v);
    if(k==0&&FRAME_MS[0].size()>30){var steady=new java.util.ArrayList<>(FRAME_MS[0].subList(30,FRAME_MS[0].size()));java.util.Collections.sort(steady);
     STATES.add("shield_first="+FRAME_MS[0].subList(0,12).stream().map(x->String.format(java.util.Locale.ROOT,"%.0f",x)).collect(java.util.stream.Collectors.joining("|")));
     STATES.add("shield_steady_p50="+String.format(java.util.Locale.ROOT,"%.1f",steady.get(steady.size()/2))+"/max="+String.format(java.util.Locale.ROOT,"%.1f",steady.get(steady.size()-1)));}
    if(!v.isEmpty())STATES.add("frame_"+kinds[k]+"/max="+String.format(java.util.Locale.ROOT,"%.0f",v.get(v.size()-1))+"="+v.size()+"/p50="+String.format(java.util.Locale.ROOT,"%.1f",v.get(v.size()/2))+"/p95="+String.format(java.util.Locale.ROOT,"%.1f",v.get((int)(v.size()*.95))));}
   ACTIONS.addProperty("freeze_screen_drawn",freezeFrames>10);
   ACTIONS.addProperty("freeze_clap_drawn",ACTIONS.has("freeze_clap_drawn"));
   ACTIONS.addProperty("shield_move_drawn",ACTIONS.has("shield_move_drawn"));
   ACTIONS.addProperty("l2_only_aims",!ACTIONS.has("l2_triggered_dark")&&aimSeen);
   ACTIONS.addProperty("r2_fired",shotSeen&&aimHit);
   ACTIONS.addProperty("r2_mined_native_block",mineSeen);
   ACTIONS.addProperty("r2_placed_native_block",placeSeen);
   ACTIONS.addProperty("l3_dark",darkSeen);
   ACTIONS.addProperty("l3_chords_light",lightSeen&&wingsSeen);
   ACTIONS.addProperty("light_flight",flightSeen);
   ACTIONS.addProperty("light_heals_survival",healSeen&&woundedHealth>0&&c.player.getHealth()>woundedHealth);
   ACTIONS.addProperty("shield_blocked_damage",shieldTested&&hit&&ACTIONS.has("shield_reacted"));
   ACTIONS.addProperty("camera_moves_between_packets",betweenPackets>4);
   finish(c,ACTIONS.entrySet().stream().allMatch(e->e.getValue().getAsBoolean())?"complete":"power/aim assertion failed");
  }
 }
 private static int firstPersonAt,strongPulse,slotBeforeZoom;
 private static long hiddenBefore;
 private static boolean shockTintSeen;
 private static void actions(Minecraft c){
  int t=++tick;
  if(t==1){
   firstPersonAt=0;shockTintSeen=false;
   c.player.getInventory().setSelectedSlot(0);
   c.getConnection().send(new net.minecraft.network.protocol.game.ServerboundSetCarriedItemPacket(0));
   onServer(c,p->{p.getInventory().setSelectedSlot(0);p.setItemInHand(net.minecraft.world.InteractionHand.MAIN_HAND,new ItemStack(JakItems.BOARD));});
  }
  pad.button(SDLGamepad.SDL_GAMEPAD_BUTTON_NORTH,t==8||t==60||t==125);
  pad.axis(4,t==25?1:-1);
  pad.axis(5,t==45||t==100?1:-1);
  if(t==20){
   ACTIONS.addProperty("triangle_cannot_equip_from_hand",c.player.getAttachedOrCreate(JakData.BOARD).isEmpty());
   ACTIONS.addProperty("triangle_requires_equipped_slot",JakPoses.get(c.player.getId())!=null&&!JakPoses.get(c.player.getId()).board());
  }
  if(t==42)ACTIONS.addProperty("l2_equipped_board",c.player.getAttachedOrCreate(JakData.BOARD).is(JakItems.BOARD));
  if(t==52)ACTIONS.addProperty("r2_does_not_deploy_board",!JakPoses.get(c.player.getId()).board());
  if(t==95){ACTIONS.addProperty("triangle_deployed_equipped_board",JakPoses.get(c.player.getId()).board());capture(c,"input-actions-board.png");}
  if(t==120)ACTIONS.addProperty("r2_does_not_stow_board",JakPoses.get(c.player.getId()).board());
  if(t==155)ACTIONS.addProperty("triangle_stowed_board",!JakPoses.get(c.player.getId()).board());
  pad.button(SDLGamepad.SDL_GAMEPAD_BUTTON_BACK,t==160||t==168);
  if(t==164){
   ACTIONS.addProperty("select_native_inventory",c.gui.screen() instanceof net.minecraft.client.gui.screens.inventory.CreativeModeInventoryScreen);
   ACTIONS.addProperty("select_kept_perspective",c.options.getCameraType()==CameraType.THIRD_PERSON_BACK);
   capture(c,"input-actions-gui.png");
  }
  if(t==169)hiddenBefore=JakRenderer.hurtHiddenDraws;
  if(t==170)onServer(c,p->{p.setGameMode(net.minecraft.world.level.GameType.SURVIVAL);p.hurtServer(p.level(),p.damageSources().cactus(),2);});
  if(t==176){
   var hurtPose=JakPoses.get(c.player.getId());
   ACTIONS.addProperty("minecraft_damage_source_reaction",hurtPose!=null&&hurtPose.clip().contains("-hit-"));
   ACTIONS.addProperty("source_strong_haptic_motor",pad.lastPulseLow==65535&&pad.lastPulseHigh==0);
  }
  if(t==244)strongPulse=pad.pulseCount;
  if(t==245)onServer(c,p->p.hurtServer(p.level(),p.damageSources().fall(),2));
  if(t==251)ACTIONS.addProperty("source_weak_fall_motor",pad.pulseCount>strongPulse&&pad.lastPulseLow==0&&pad.lastPulseHigh==65535);
  if(t==350)ACTIONS.addProperty("source_hurt_blink_rendered",JakRenderer.hurtHiddenDraws>hiddenBefore);
  if(t==360)onServer(c,p->{p.setHealth(p.getMaxHealth());p.hurtServer(p.level(),p.damageSources().lava(),1);});
  if(t==400){
   ACTIONS.addProperty("source_burn_tint_rendered",JakRenderer.hurtRed>.45f&&JakRenderer.hurtRed<.65f&&JakRenderer.hurtRed==JakRenderer.hurtGreen);
   capture(c,"input-actions-burn.png");
  }
  if(t==450)onServer(c,p->p.hurtServer(p.level(),p.damageSources().lightningBolt(),1));
  if(t>=451&&t<=470&&JakRenderer.hurtRed>=.5f&&Math.abs(JakRenderer.hurtBlue-JakRenderer.hurtRed-.5f)<.001f){
   if(!shockTintSeen)capture(c,"input-actions-shock.png");
   shockTintSeen=true;
  }
  if(t==470)ACTIONS.addProperty("source_shock_tint_rendered",shockTintSeen);
  if(t==259)slotBeforeZoom=c.player.getInventory().getSelectedSlot();
  if(t>=260&&firstPersonAt==0&&c.options.getCameraType().isFirstPerson())firstPersonAt=t;
  int age=firstPersonAt==0?-1:t-firstPersonAt;
  boolean zoomIn=t>=260&&firstPersonAt==0;
  boolean zoomOut=age>=2&&age<24;
  pad.button(SDLGamepad.SDL_GAMEPAD_BUTTON_RIGHT_STICK,zoomIn||zoomOut);
  pad.button(SDLGamepad.SDL_GAMEPAD_BUTTON_DPAD_UP,zoomIn);
  pad.button(SDLGamepad.SDL_GAMEPAD_BUTTON_DPAD_DOWN,zoomOut);
  if(age==25){
   ACTIONS.addProperty("r3_zoom_reached_first_person",true);
   ACTIONS.addProperty("r3_zoom_returned_to_third_person",c.options.getCameraType()==CameraType.THIRD_PERSON_BACK&&JakInput.cameraZoomFactor()>.5);
   ACTIONS.addProperty("zoom_did_not_open_inventory",c.gui.screen()==null);
   ACTIONS.addProperty("zoom_kept_hotbar_slot",c.player.getInventory().getSelectedSlot()==slotBeforeZoom);
   var pose=JakPoses.get(c.player.getId());
   ACTIONS.addProperty("r3_did_not_transform",pose!=null&&!pose.dark()&&!pose.light());
  }
  pad.button(SDLGamepad.SDL_GAMEPAD_BUTTON_START,age==30||age==36);
  if(age==33)ACTIONS.addProperty("start_native_pause",c.gui.screen() instanceof net.minecraft.client.gui.screens.PauseScreen);
  if((age>=40||t>=380)&&t>=500){
   ACTIONS.addProperty("r3_zoom_reached_first_person",firstPersonAt>0);
   ACTIONS.addProperty("start_closed_pause",c.gui.screen()==null);
   finish(c,ACTIONS.entrySet().stream().allMatch(e->e.getValue().getAsBoolean())?"complete":"controller layout assertion failed");
  }
 }
 private static void tick(Minecraft c){
  if(!running)return;if(c.player==null||c.getSingleplayerServer()==null){finish(c,"lost world");return;}
  // Hot-plug enumeration is asynchronous. Replay must wait for the normal backend
  // to open the device at rest, as SDL calibrates virtual triggers on open.
  if(!stageReady||!JakInput.controllerReady(pad.id)||dev.jakmc.client.model.JakRenderer.clothRenderSerial<=clothStart){if(System.nanoTime()>deviceDeadline)finish(c,"Replay setup or SDL device did not become ready");return;}
  if(powers){powerActions(c);return;}
  if(darkMotion){darkMotionActions(c);return;}
  if(hellcat){hellcatActions(c);return;}
  if(leaper){leaperActions(c);return;}
  if(interactions){actions(c);return;}
  int t=++tick;
  recordedStick=Math.max(recordedStick,JakInput.controller().activity());
  var pose=JakPoses.get(c.player.getId());if(pose!=null){if(pose.raw().has("state"))STATES.add(pose.raw().get("state").getAsString());peak=Math.max(peak,c.player.getY()-start.y);}
  if(t>=18&&t<95&&dev.jakmc.client.model.JakRenderer.clothRenderSerial>clothStart){
   float reach=dev.jakmc.client.model.JakRenderer.clothScarfReach;
   clothMin=Math.min(clothMin,reach);clothMax=Math.max(clothMax,reach);
  }
  if(t>=28&&t<=36||t>=81&&t<=89){
   var at=c.player.position();
   double yaw=Math.toRadians(c.player.getYRot());
   look(at.add(Math.sin(yaw)*2.2+Math.cos(yaw)*1.3,1.65,-Math.cos(yaw)*2.2+Math.sin(yaw)*1.3),
        at.add(0,1,0));
  }
  if(t==37||t==90)view=null;
  if(t==12){c.player.setYRot(0);c.player.setXRot(10);yawStart=c.player.getYRot();c.gui.hud.getChat().clearMessages(false);}
  pad.axis(1,t>=20&&t<45||t>=135&&t<155||t>=240&&t<260?-.9f:0);
  pad.axis(2,t>=50&&t<65?.6f:0);
  pad.button(SDLGamepad.SDL_GAMEPAD_BUTTON_SOUTH,t==80||t==84||t==142);
  pad.button(SDLGamepad.SDL_GAMEPAD_BUTTON_LEFT_SHOULDER,t>=135&&t<155);
  pad.button(SDLGamepad.SDL_GAMEPAD_BUTTON_EAST,t==180);
  pad.button(SDLGamepad.SDL_GAMEPAD_BUTTON_WEST,t==200);
  // Triangle takes the board out and puts it away (R2 mines).
  pad.button(SDLGamepad.SDL_GAMEPAD_BUTTON_NORTH,t==225||t==280);
  if(t==75&&(walk<.1||Math.abs(yawAfter-yawStart)<.1)){finish(c,"Controller input not active; focused="+c.isWindowActive()+", device="+JakInput.controller().name());return;}
  if(t==45)walk=c.player.position().subtract(start).horizontalDistance();
  if(t==68){
   yawAfter=c.player.getYRot();
   ACTIONS.addProperty("minimap_reads_displayed_camera",JakMinimap.ready()
    &&Math.abs(net.minecraft.util.Mth.wrapDegrees(JakMinimap.presentedYaw()-c.gameRenderer.mainCamera().yRot()))<.01);
  }
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
  if(t==400){view=null;c.gui.setScreen(new net.minecraft.client.gui.screens.inventory.InventoryScreen(c.player));}
  if(t==495)capture(c,"input-eco-gui.png");
  if(t>=505)finish(c,!ACTIONS.has("minimap_reads_displayed_camera")||!ACTIONS.get("minimap_reads_displayed_camera").getAsBoolean()
      ?"Minimap yaw differs from camera":!STATES.containsAll(Set.of("double-jump","roll-flip","attack","board-stance"))
      ?"Native Jak action did not run":clothMax-clothMin>.01f?"complete":"Cloth solver did not bend the scarf");
 }
 private static void look(Vec3 at,Vec3 target){Vec3 d=target.subtract(at);view=new CameraView(at,(float)Math.toDegrees(Math.atan2(-d.x,d.z)),(float)Math.toDegrees(Math.atan2(-d.y,d.horizontalDistance())));}
 private static void capture(Minecraft c,String file){Screenshot.grab(c.gameDirectory,file,c.gameRenderer.mainRenderTarget(),1,m->{});}
 private static void finish(Minecraft c,String reason){
  if(powers||leaper||hellcat||darkMotion)c.options.pauseOnLostFocus=pauseOnLostFocus;
  running=false;view=null;if(pad!=null){pad.close();pad=null;}var result=new JsonObject();result.addProperty("reason",reason);result.addProperty("walk_blocks",walk);result.addProperty("jump_peak_blocks",peak);result.addProperty("camera_yaw_delta",yawAfter-yawStart);result.addProperty("states",String.join(",",STATES));result.addProperty("visual_review","required");if(ACTIONS.has("minimap_reads_displayed_camera"))result.addProperty("minimap_reads_displayed_camera",ACTIONS.get("minimap_reads_displayed_camera").getAsBoolean());if(powers){result.add("actions",ACTIONS.deepCopy());result.addProperty("moving_frames_between_pose_packets",betweenPackets);}if(interactions||leaper||hellcat||darkMotion){result.add("actions",ACTIONS.deepCopy());if(ACTIONS.entrySet().stream().anyMatch(e->!e.getValue().getAsBoolean()))result.addProperty("reason",reason.equals("complete")?"interaction assertion failed":reason);}
  result.addProperty("jak_actions_reached",STATES.containsAll(Set.of("double-jump","roll","attack","board-stance")));
  result.addProperty("cloth_draws",dev.jakmc.client.model.JakRenderer.clothRenderSerial-clothStart);
  result.addProperty("scarf_reach_span",Float.isFinite(clothMin)?clothMax-clothMin:0);
  result.addProperty("controller_activity",recordedStick);
  try{Files.writeString(output.resolve("result.json"),result.toString());}catch(Exception e){throw new RuntimeException(e);}
  if(c.getSingleplayerServer()!=null){var server=c.getSingleplayerServer();server.execute(()->{for(UUID id:ITEMS){var item=server.overworld().getEntity(id);if(item!=null)item.discard();}});}c.stop();
 }
}
