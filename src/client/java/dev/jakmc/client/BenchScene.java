package dev.jakmc.client;

import com.google.gson.JsonArray;
import com.google.gson.JsonObject;
import dev.jakmc.JakData;
import dev.jakmc.client.input.SdlVirtualPad;
import dev.jakmc.entity.JakVehicle;
import java.nio.file.Files;
import java.nio.file.Path;
import java.util.ArrayList;
import java.util.Arrays;
import java.util.List;
import net.fabricmc.fabric.api.client.event.lifecycle.v1.ClientTickEvents;
import net.fabricmc.fabric.api.client.rendering.v1.level.LevelRenderEvents;
import net.fabricmc.fabric.api.event.lifecycle.v1.ServerTickEvents;
import net.minecraft.client.CameraType;
import net.minecraft.client.Minecraft;
import net.minecraft.client.Screenshot;
import net.minecraft.server.level.ServerPlayer;
import net.minecraft.world.InteractionHand;
import net.minecraft.world.entity.EntitySpawnReason;
import net.minecraft.world.level.levelgen.Heightmap;
import net.minecraft.world.phys.Vec3;

/**
 * The game's frame budget under load (Adrii 06/11: "optimise in general, the zoomer is just a good way to
 * benchmark it"): a zoomer at full throttle in a straight line through a fresh world with Adrii's video
 * settings (render distance 24, simulation 6, no vsync, uncapped), every frame's time, every server tick's,
 * and a JFR profile of the drive. Run twice (before/after) and compare.
 */
public final class BenchScene {
 private static final int SETTLE=200,DRIVE=1200;
 private static SdlVirtualPad pad;
 private static volatile int carId=-1;
 private static int tick;
 private static boolean running,driving;
 private static long lastFrame,serverStart;
 private static final List<Long> FRAMES=new ArrayList<>(),TICKS=new ArrayList<>();
 private static final List<double[]> TRACK=new ArrayList<>();
 private static Vec3 start,dry;
 private static float heading;
 private static int respawns,lostAt=-1;
 private static double travelled,stuckFrom;
 private static Vec3 lastCar;
 private static jdk.jfr.Recording recording;
 private static CameraType camera;
 private static boolean pause;
 private static InputScene.CameraView view;

 static InputScene.CameraView camera(){return view;}
 static boolean backgroundPad(int id){return pad!=null&&pad.id==id;}

 static void initialize(){
  ClientTickEvents.END_CLIENT_TICK.register(BenchScene::tick);
  LevelRenderEvents.COLLECT_SUBMITS.register(context->{
   if(!driving)return;
   long now=System.nanoTime();
   if(lastFrame!=0)FRAMES.add(now-lastFrame);
   lastFrame=now;
  });
  ServerTickEvents.START_SERVER_TICK.register(s->{if(driving)serverStart=System.nanoTime();});
  ServerTickEvents.END_SERVER_TICK.register(s->{if(driving&&serverStart!=0)TICKS.add(System.nanoTime()-serverStart);});
 }

 static void start(Minecraft client){
  if(running||client.getSingleplayerServer()==null||client.player==null)throw new IllegalStateException("Open the development world");
  running=true;driving=false;tick=0;carId=-1;lastFrame=0;FRAMES.clear();TICKS.clear();TRACK.clear();
  heading=0;respawns=0;lostAt=-1;travelled=0;stuckFrom=0;lastCar=null;dry=null;
  var o=client.options;
  // Adrii's options.txt (jakmod instance, 06/11): render 24, simulation 6, vsync off, improved transparency off.
  o.renderDistance().set(24);o.simulationDistance().set(6);o.enableVsync().set(false);o.framerateLimit().set(260);
  o.improvedTransparency().set(false);
  camera=o.getCameraType();pause=o.pauseOnLostFocus;o.pauseOnLostFocus=false;o.setCameraType(CameraType.THIRD_PERSON_BACK);
  pad=new SdlVirtualPad();
  onServer(client,p->{
   var level=p.level();
   level.getServer().getCommands().performPrefixedCommand(level.getServer().createCommandSourceStack(),"time set noon");
   p.setAttached(JakData.ENABLED,true);
   start=p.position();
   spawn(p,p.position());
  });
 }

 // A zoomer at `at` (on the surface there) facing the heading, Jak beside it; it is mounted a few ticks later.
 private static void spawn(ServerPlayer p,Vec3 at){
  var level=p.level();
  int y=level.getHeight(Heightmap.Types.MOTION_BLOCKING,(int)Math.floor(at.x),(int)Math.floor(at.z));
  var model=JakVehicle.Model.values()[2];
  var car=model.type.create(level,EntitySpawnReason.COMMAND);
  var pos=new Vec3(Math.floor(at.x)+.5,y+model.parkingClearance(),Math.floor(at.z)+.5);
  car.setPos(pos);car.setYRot(heading);
  // Jak on the zoomer's own column (a cliff beside it would bury him), and unhurt for the measurement.
  p.stopRiding();p.getAbilities().invulnerable=true;p.onUpdateAbilities();p.teleportTo(pos.x,y+1,pos.z);p.setYRot(heading);p.setXRot(0);
  if(!level.addFreshEntity(car))throw new IllegalStateException("Could not place the bench zoomer");
  carId=car.getId();
 }

 private static void tick(Minecraft client){
  if(!running||client.level==null||client.player==null||carId<0||!JakInput.controllerReady(pad.id))return;
  tick++;
  if(tick==SETTLE-10)onServer(client,p->{var car=p.level().getEntity(carId);if(car!=null)car.interact(p,InteractionHand.MAIN_HAND,new Vec3(0,-.55,-1));});
  if(tick==SETTLE){
   pad.axis(5,1);driving=true;dev.jakmc.client.model.GltfModel.corners=new java.util.concurrent.ConcurrentHashMap<>();
   try{
    recording=new jdk.jfr.Recording(jdk.jfr.Configuration.getConfiguration("profile"));
    recording.enable("jdk.ExecutionSample").withPeriod(java.time.Duration.ofMillis(5));
    recording.enable("jdk.NativeMethodSample").withPeriod(java.time.Duration.ofMillis(5));
    recording.setToDisk(true);recording.start();
   }catch(Exception e){recording=null;}
  }
  if(driving){
   if(client.level.getEntity(carId) instanceof JakVehicle car){
    if(lastCar!=null)travelled+=car.position().subtract(lastCar).horizontalDistance();
    lastCar=car.position();
    // The last dry place it passed (zoomers wreck in water, as in Jak 2).
    if(tick%10==0&&!client.level.getFluidState(car.blockPosition().below()).isSource()&&!car.isInWater())dry=car.position();
    if(tick%20==0)TRACK.add(new double[]{(tick-SETTLE)/20.0,travelled,car.position().y-start.y,client.getFps()});
    // Stuck against a hill or a tree for three seconds: off with it, a new one a quarter turn on.
    if(tick%60==0){
     if(lostAt<0&&tick>SETTLE+60&&travelled-stuckFrom<5){int id=carId;onServer(client,p->{p.stopRiding();var stuck=p.level().getEntity(id);if(stuck!=null)stuck.discard();});}
     stuckFrom=travelled;
    }
   }else if(lostAt<0){
    // Wrecked (or gone): a new one on the last dry place, a quarter turn on, mounted a little later.
    lostAt=tick;respawns++;heading+=90;lastCar=null;
    var from=dry!=null?dry:start;
    onServer(client,p->spawn(p,from));
   }
   if(lostAt>=0&&tick==lostAt+6)onServer(client,p->{var car=p.level().getEntity(carId);if(car!=null)car.interact(p,InteractionHand.MAIN_HAND,new Vec3(0,-.55,-1));});
   if(lostAt>=0&&tick==lostAt+12)lostAt=-1;
  }
  if(tick==SETTLE+DRIVE/2)Screenshot.grab(client.gameDirectory,"bench-drive.png",client.gameRenderer.mainRenderTarget(),1,_->{});
  if(tick<SETTLE+DRIVE)return;
  driving=false;pad.axis(5,-1);
  var drawn=dev.jakmc.client.model.GltfModel.corners;dev.jakmc.client.model.GltfModel.corners=null;
  var out=new JsonObject();
  var frames=FRAMES.stream().mapToLong(Long::longValue).sorted().toArray();
  var ticks=TICKS.stream().mapToLong(Long::longValue).sorted().toArray();
  out.add("frames_ms",stats(frames));
  out.add("server_tick_ms",stats(ticks));
  out.addProperty("frames",frames.length);
  out.addProperty("seconds",DRIVE/20.0);
  out.addProperty("hitches_over_50ms",Arrays.stream(frames).filter(f->f>50_000_000L).count());
  out.addProperty("hitches_over_100ms",Arrays.stream(frames).filter(f->f>100_000_000L).count());
  var track=new JsonArray();
  for(var row:TRACK){var r=new JsonArray();for(double v:row)r.add(Math.round(v*10)/10.0);track.add(r);}
  out.add("track_seconds_distance_height_fps",track);
  out.addProperty("distance_blocks",travelled);
  out.addProperty("respawns",respawns);
  var models=new JsonObject();
  drawn.entrySet().stream().sorted((a,b)->Long.compare(b.getValue().sum(),a.getValue().sum())).limit(25)
   .forEach(e->models.addProperty(e.getKey(),e.getValue().sum()/(DRIVE/20)));
  out.add("model_corners_per_second",models);
  out.addProperty("reason","complete");
  try{
   var folder=client.gameDirectory.toPath().resolve("jakmc/evidence/bench");Files.createDirectories(folder);
   if(recording!=null){recording.dump(folder.resolve("run.jfr"));recording.close();recording=null;}
   Files.writeString(folder.resolve("result.json"),out.toString());
  }catch(Exception e){throw new RuntimeException(e);}
  pad.close();pad=null;running=false;
  client.options.pauseOnLostFocus=pause;client.options.setCameraType(camera);
  onServer(client,p->{p.stopRiding();p.getAbilities().invulnerable=false;p.onUpdateAbilities();var car=p.level().getEntity(carId);if(car!=null)car.discard();client.execute(client::stop);});
 }

 private static JsonObject stats(long[] sorted){
  var o=new JsonObject();
  if(sorted.length==0)return o;
  double sum=0;for(long v:sorted)sum+=v;
  o.addProperty("mean",Math.round(sum/sorted.length/1e4)/100.0);
  for(double q:new double[]{.5,.9,.99,.999})o.addProperty("p"+(int)Math.round(q*1000)/10.0,Math.round(sorted[Math.min(sorted.length-1,(int)(q*sorted.length))]/1e4)/100.0);
  o.addProperty("max",Math.round(sorted[sorted.length-1]/1e4)/100.0);
  return o;
 }

 private static void onServer(Minecraft client,java.util.function.Consumer<ServerPlayer> action){
  var server=client.getSingleplayerServer();var uuid=client.player.getUUID();
  server.execute(()->{var player=server.getPlayerList().getPlayer(uuid);if(player!=null)action.accept(player);});
 }
}
