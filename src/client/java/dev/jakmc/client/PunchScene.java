package dev.jakmc.client;
import com.google.gson.JsonObject;
import dev.jakmc.JakData;
import dev.jakmc.player.JakPlayers;
import dev.jakmc.worldgen.JakLands;
import net.fabricmc.fabric.api.client.event.lifecycle.v1.ClientTickEvents;
import net.fabricmc.fabric.api.event.lifecycle.v1.ServerTickEvents;
import net.minecraft.client.CameraType;
import net.minecraft.client.Minecraft;
import net.minecraft.client.Screenshot;
import net.minecraft.core.BlockPos;
import net.minecraft.server.level.ServerPlayer;
import net.minecraft.world.level.block.Block;
import net.minecraft.world.level.block.Blocks;
import net.minecraft.world.level.levelgen.Heightmap;
import java.nio.file.Files;
import java.nio.file.Path;
import java.util.*;

/**
 * Jak's punches on blocks (survival: a dirt wall, then stone, cracked and
 * broken by square alone), then a run over fresh ground with the flight
 * recorder on, timing the client's ticks and the server's to find stutter.
 */
public final class PunchScene {
 private static final int SQUARE=2,X=1,PUNCH_EVERY=14,LEDGE_START=420,STEP_START=700,RUN_START=770,RUN_END=RUN_START+1500;
 private static boolean running;
 private static int tick,punches,dirtPunches,stonePunches;
 private static UUID player;
 private static Path output;
 private static BlockPos base;
 private static jdk.jfr.Recording recording;
 private static long lastClientTick,serverStart,landsNanos;
 private static final List<Long> CLIENT_GAPS=new ArrayList<>(),SERVER_TICKS=Collections.synchronizedList(new ArrayList<>());
 private static final List<Integer> FPS=new ArrayList<>();
 private static final List<String> HITCHES=new ArrayList<>();
 private static final Set<String> LEDGE_TWO=new TreeSet<>(),LEDGE_THREE=new TreeSet<>();
 private static boolean grabShot;
 private static final List<String> LEDGE_TRACE=new ArrayList<>();
 private static final JsonObject RESULT=new JsonObject();
 private static InputScene.CameraView view;
 private static boolean serverHooked;
 private static dev.jakmc.client.input.SdlVirtualPad pad;
 static boolean backgroundPad(int id){return pad!=null&&pad.id==id;}
 public static InputScene.CameraView camera(){return view;}
 public static void initialize(){
  ClientTickEvents.END_CLIENT_TICK.register(PunchScene::tick);
 }
 private static void hookServer(){
  if(serverHooked)return;serverHooked=true;
  ServerTickEvents.START_SERVER_TICK.register(s->{if(running)serverStart=System.nanoTime();});
  ServerTickEvents.END_SERVER_TICK.register(s->{if(running&&tick>=RUN_START&&tick<RUN_END&&serverStart!=0)SERVER_TICKS.add(System.nanoTime()-serverStart);});
 }
 public static void start(Minecraft c){
  if(running||c.player==null||c.getSingleplayerServer()==null)throw new IllegalStateException("Open isolated dev world");
  hookServer();
  tick=punches=dirtPunches=stonePunches=0;base=null;lastClientTick=0;
  CLIENT_GAPS.clear();HITCHES.clear();LEDGE_TWO.clear();LEDGE_THREE.clear();LEDGE_TRACE.clear();grabShot=false;SERVER_TICKS.clear();FPS.clear();RESULT.entrySet().clear();
  JakPlayers.WORLD_STRIKES.set(0);JakPlayers.WORLD_STRIKES_FOUND.set(0);
  player=c.player.getUUID();
  output=c.gameDirectory.toPath().resolve("jakmc/evidence/punch");
  try{Files.createDirectories(output);}catch(Exception e){throw new RuntimeException(e);}
  try{
   recording=new jdk.jfr.Recording(jdk.jfr.Configuration.getConfiguration("profile"));
   recording.setToDisk(true);recording.start();
  }catch(Exception e){recording=null;RESULT.addProperty("jfr_error",e.toString());}
  running=true;
  c.options.setCameraType(CameraType.THIRD_PERSON_BACK);
  c.getSingleplayerServer().execute(()->{
   var p=server(c);if(p==null)return;var level=p.level();
   var s=c.getSingleplayerServer();
   s.getCommands().performPrefixedCommand(s.createCommandSourceStack(),"gamemode survival "+p.getScoreboardName());
   s.getCommands().performPrefixedCommand(s.createCommandSourceStack(),"time set noon");
   BlockPos at=p.blockPosition();
   int top=level.getHeight(Heightmap.Types.MOTION_BLOCKING_NO_LEAVES,at.getX(),at.getZ());
   BlockPos floor=new BlockPos(at.getX(),top+1,at.getZ());
   for(int x=-5;x<=5;x++)for(int z=-5;z<=6;z++){
    level.setBlock(floor.offset(x,-1,z),Blocks.STONE.defaultBlockState(),3);
    for(int y=0;y<6;y++)level.setBlock(floor.offset(x,y,z),Blocks.AIR.defaultBlockState(),3);
   }
   wall(level,floor,Blocks.DIRT);
   p.setAttached(JakData.ENABLED,true);
   p.getInventory().setSelectedSlot(8);p.getInventory().setItem(8,net.minecraft.world.item.ItemStack.EMPTY);
   p.setYRot(0);p.setYBodyRot(0);p.setYHeadRot(0);
   p.teleportTo(floor.getX()+.5,floor.getY(),floor.getZ()+.5);
   JakPlayers.controlForProbe(p,true);
   base=floor;
  });
 }
 /** A wall a block and a half in front of Jak, three wide and three high. */
 private static void wall(net.minecraft.server.level.ServerLevel level,BlockPos floor,Block block){
  for(int x=-1;x<=1;x++)for(int y=0;y<3;y++)level.setBlock(floor.offset(x,y,1),block.defaultBlockState(),3);
 }
 private static int standing(net.minecraft.server.level.ServerLevel level,Block block){
  int n=0;for(int x=-1;x<=1;x++)for(int y=0;y<3;y++)if(level.getBlockState(base.offset(x,y,1)).is(block))n++;
  return n;
 }
 private static ServerPlayer server(Minecraft c){return c.getSingleplayerServer().getPlayerList().getPlayer(player);}
 private static void tick(Minecraft c){
  if(!running)return;
  if(c.player==null||c.getSingleplayerServer()==null){finish(c,"lost world");return;}
  int t=++tick;
  long now=System.nanoTime();
  if(t>=RUN_START&&t<RUN_END&&lastClientTick!=0){CLIENT_GAPS.add(now-lastClientTick);if(now-lastClientTick>100_000_000L)HITCHES.add((t-RUN_START)+":"+(now-lastClientTick)/1_000_000);}
  lastClientTick=now;
  if(t>=RUN_START&&t<RUN_END&&t%20==0)FPS.add(c.getFps());
  if(base==null){if(t>200)finish(c,"setup never ran");return;}
  var stage=net.minecraft.world.phys.Vec3.atBottomCenterOf(base);
  if(t<LEDGE_START)look(stage.add(4.5,3.2,-3.5),stage.add(0,1.2,1));
  else if(t<RUN_START)look(stage.add(7,3,1.5),stage.add(0,1.5,2.5));else view=null;
  if(t==LEDGE_START-2)capture(c,"punch-after.png");
  // The ledge: a run and a jump at a wall two blocks high, then three.
  if(t>=LEDGE_START&&t<STEP_START){
   var pose=JakPoses.get(c.player.getId());
   if(pose!=null&&pose.raw().has("state")){String state=pose.raw().get("state").getAsString();(t<LEDGE_START+140?LEDGE_TWO:LEDGE_THREE).add(state);
    if((t-LEDGE_START)%140<35)LEDGE_TRACE.add((t-LEDGE_START)+":"+state+":"+String.format("%.2f,%.2f %s",pose.position().y-base.getY(),pose.position().z-base.getZ(),pose.raw().has("trail")?pose.raw().get("trail").toString():""));
    if(state.startsWith("edge-grab")&&!grabShot){grabShot=true;capture(c,"punch-ledge.png");}}
  }
  if(t==120)capture(c,"punch-dirt.png");
  c.getSingleplayerServer().execute(()->{
   var p=server(c);if(p==null)return;var level=p.level();
   // Steps (Adrii 05/11): on foot a slab is climbed, a whole block is not.
   if(t>=STEP_START&&t<RUN_START){
    int k=(t-STEP_START)%35;boolean slab=t>=STEP_START+35;
    if(k==0){
     for(int x=-5;x<=5;x++)for(int z=-5;z<=6;z++)for(int y=0;y<6;y++)level.setBlock(base.offset(x,y,z),Blocks.AIR.defaultBlockState(),3);
     for(int x=-5;x<=5;x++)for(int z=3;z<=6;z++)level.setBlock(base.offset(x,0,z),slab?Blocks.STONE_SLAB.defaultBlockState():Blocks.STONE.defaultBlockState(),3);
     p.setYRot(0);p.teleportTo(base.getX()+.5,base.getY(),base.getZ()-1+.5);
    }
    JakPlayers.probeInput(p,0,0,k<30?1:0);
    String key=slab?"step_slab_max_y":"step_block_max_y";double y=p.getY()-base.getY();
    if(k>2&&(!RESULT.has(key)||RESULT.get(key).getAsDouble()<y))RESULT.addProperty(key,Math.round(y*100)/100.);
    if(k==29)RESULT.addProperty(slab?"step_slab_z":"step_block_z",Math.round((p.getZ()-base.getZ())*100)/100.);
   }
   if(t>=LEDGE_START&&t<STEP_START){
    int k=(t-LEDGE_START)%35;
    if(t==LEDGE_START||t==LEDGE_START+140){
     for(int x=-5;x<=5;x++)for(int z=-5;z<=6;z++)for(int y=0;y<6;y++)level.setBlock(base.offset(x,y,z),Blocks.AIR.defaultBlockState(),3);
     int high=t==LEDGE_START?2:3;
     for(int x=-5;x<=5;x++)for(int y=0;y<high;y++)level.setBlock(base.offset(x,y,4),Blocks.STONE.defaultBlockState(),3);
    }
    if(k==12&&t<LEDGE_START+35){
     var buf=dev.jakmc.nativebridge.MinecraftCollision.triangles(level,p,net.minecraft.world.phys.Vec3.atLowerCornerOf(base),new net.minecraft.world.phys.AABB(-1,0,3,2,3,5).move(base.getX(),base.getY(),base.getZ()));
     buf.order(java.nio.ByteOrder.LITTLE_ENDIAN);var zs=new TreeSet<String>();int n=buf.limit()/40;
     for(int i=0;i<n;i++){float z0=buf.getFloat(i*40+8),z1=buf.getFloat(i*40+20),z2=buf.getFloat(i*40+32);if(z0==z1&&z1==z2)zs.add(String.format("%.3f f%d",z0,buf.getInt(i*40+36)));}
     var blocks=new StringBuilder();for(int x=-1;x<=1;x++)for(int y=0;y<=3;y++)for(int z=2;z<=5;z++){var st=level.getBlockState(base.offset(x,y,z));if(!st.isAir())blocks.append(x).append(',').append(y).append(',').append(z).append('=').append(net.minecraft.core.registries.BuiltInRegistries.BLOCK.getKey(st.getBlock()).getPath()).append(' ');}
     var ents=new StringBuilder();for(var en:level.getEntities(p,p.getBoundingBox().inflate(10)))ents.append(en.getType().toShortString()).append(en.isPickable()?"*":"").append(' ');
     RESULT.addProperty("wall_faces",n+" tris, z-planes "+zs+" blocks "+blocks+" entities "+ents);}
    if(k==10&&t<LEDGE_START+35){var near=new StringBuilder();for(var e:level.getEntities(p,p.getBoundingBox().inflate(8),e->e.isAlive()&&e.isPickable()))near.append(e.getType().toShortString()).append('@').append(e.blockPosition().toShortString()).append(' ');
     RESULT.addProperty("ledge_actors",near.toString());RESULT.addProperty("ledge_pitch",p.getXRot());}
    if(k==0){p.setYRot(0);p.teleportTo(base.getX()+.5,base.getY(),base.getZ()-1+.5);}
    JakPlayers.probeInput(p,k>=8&&k<20?X:0,k==8?X:0,k<30?1:0);
   }
   if(t<LEDGE_START){
    // Dirt first, then stone: square pressed alone, one punch at a time.
    int phase=t<200?0:1;
    if(t==200){RESULT.addProperty("dirt_left",standing(level,Blocks.DIRT));RESULT.addProperty("dirt_punches",dirtPunches);
     RESULT.addProperty("strikes_after_dirt",JakPlayers.WORLD_STRIKES.get());RESULT.addProperty("found_after_dirt",JakPlayers.WORLD_STRIKES_FOUND.get());
     wall(level,base,Blocks.STONE);p.teleportTo(base.getX()+.5,base.getY(),base.getZ()+.5);}
    int k=t%PUNCH_EVERY;
    if(t>20&&k==0){
     JakPlayers.probeInput(p,SQUARE,SQUARE,0);punches++;
     if(phase==0&&standing(level,Blocks.DIRT)>0)dirtPunches++;
     if(phase==1&&standing(level,Blocks.STONE)>0)stonePunches++;
    }else if(k==2)JakPlayers.probeInput(p,0,0,0);
    if(t==LEDGE_START-5){RESULT.addProperty("stone_left",standing(level,Blocks.STONE));RESULT.addProperty("stone_punches",stonePunches);
     RESULT.addProperty("world_strikes",JakPlayers.WORLD_STRIKES.get());RESULT.addProperty("world_strikes_found",JakPlayers.WORLD_STRIKES_FOUND.get());
     RESULT.addProperty("punches",punches);}
   }
   if(t==RUN_START){
    var s=c.getSingleplayerServer();
    s.getCommands().performPrefixedCommand(s.createCommandSourceStack(),"gamemode creative "+p.getScoreboardName());
    landsNanos=JakLands.NANOS.get();
    JakPlayers.controlForProbe(p,false);
    p.teleportTo(base.getX()+.5,base.getY(),base.getZ()+.5);p.setYRot(0);
    for(int x=-5;x<=5;x++)for(int y=0;y<3;y++)level.setBlock(base.offset(x,y,4),Blocks.AIR.defaultBlockState(),3);
    // A pillar between Jak and his camera: the camera must stay out and cut it away.
    for(int y=0;y<4;y++)level.setBlock(base.offset(0,y,-2),Blocks.STONE.defaultBlockState(),3);
   }
   // The run: forward, jumping now and then over what's in the way.
   if(t==RUN_END){RESULT.addProperty("lands_ms_during_run",(JakLands.NANOS.get()-landsNanos)/1e6);
    RESULT.addProperty("run_distance",Math.round(p.position().distanceTo(stage)));}
  });
  // The run on a (virtual) pad, as Adrii plays: stick forward, X now and
  // then; well over the minute after which Minecraft takes a player for away.
  if(t==RUN_START){try{pad=new dev.jakmc.client.input.SdlVirtualPad();}catch(RuntimeException e){RESULT.addProperty("pad_error",e.toString());}}
  if(pad!=null&&t>RUN_START&&t<RUN_END){
   pad.axis(1,-1);
   pad.button(org.lwjgl.sdl.SDLGamepad.SDL_GAMEPAD_BUTTON_SOUTH,t%30<3);
   if(t>RUN_START+700&&t<RUN_START+720)pad.axis(2,.8f);else pad.axis(2,0);
  }
  if(pad!=null&&t==RUN_END){pad.axis(1,0);pad.button(org.lwjgl.sdl.SDLGamepad.SDL_GAMEPAD_BUTTON_SOUTH,false);}
  if(t==RUN_START+1300)RESULT.addProperty("throttle_after_minute",c.getFramerateLimitTracker().getThrottleReason().toString());
  if(t==RUN_START+200)capture(c,"punch-run.png");
  if(t==RUN_START+6)capture(c,"punch-cutout.png");
  if(t>=RUN_END+10)finish(c,"complete");
 }
 private static void look(net.minecraft.world.phys.Vec3 at,net.minecraft.world.phys.Vec3 target){
  var d=target.subtract(at);
  view=new InputScene.CameraView(at,(float)Math.toDegrees(Math.atan2(-d.x,d.z)),(float)Math.toDegrees(Math.atan2(-d.y,d.horizontalDistance())));
 }
 private static void capture(Minecraft c,String name){Screenshot.grab(c.gameDirectory,name,c.gameRenderer.mainRenderTarget(),1,m->{});}
 private static JsonObject spread(List<Long> nanos){
  var o=new JsonObject();
  if(nanos.isEmpty())return o;
  var ms=nanos.stream().mapToDouble(n->n/1e6).sorted().toArray();
  o.addProperty("count",ms.length);
  o.addProperty("median_ms",ms[ms.length/2]);
  o.addProperty("p99_ms",ms[(int)(ms.length*.99)]);
  o.addProperty("max_ms",ms[ms.length-1]);
  o.addProperty("over_100ms",Arrays.stream(ms).filter(v->v>100).count());
  o.addProperty("over_50ms",Arrays.stream(ms).filter(v->v>50).count());
  return o;
 }
 private static void finish(Minecraft c,String reason){
  running=false;view=null;
  RESULT.addProperty("reason",reason);
  RESULT.add("client_tick_gaps",spread(CLIENT_GAPS));
  List<Long> server;synchronized(SERVER_TICKS){server=new ArrayList<>(SERVER_TICKS);}
  RESULT.add("server_ticks",spread(server));
  RESULT.addProperty("fps",FPS.toString());
  RESULT.addProperty("hitches_tick_ms",HITCHES.toString());
  RESULT.addProperty("ledge_trace",LEDGE_TRACE.toString());
  RESULT.addProperty("ledge_two_states",LEDGE_TWO.toString());RESULT.addProperty("ledge_three_states",LEDGE_THREE.toString());
  RESULT.addProperty("ledge_two_grabbed",LEDGE_TWO.stream().anyMatch(s->s.startsWith("edge-grab")));
  RESULT.addProperty("ledge_three_grabbed",LEDGE_THREE.stream().anyMatch(s->s.startsWith("edge-grab")));
  if(pad!=null){pad.close();pad=null;}
  if(FPS.size()>60)RESULT.addProperty("fps_after_minute_min",FPS.subList(55,FPS.size()).stream().mapToInt(Integer::intValue).min().orElse(0));
  if(recording!=null){
   try{recording.dump(output.resolve("run.jfr"));recording.close();RESULT.addProperty("jfr",output.resolve("run.jfr").toString());}
   catch(Exception e){RESULT.addProperty("jfr_error",e.toString());}
  }
  boolean passed=reason.equals("complete")&&RESULT.has("dirt_left")&&RESULT.get("dirt_left").getAsInt()<9
   &&RESULT.has("stone_left")&&RESULT.get("stone_left").getAsInt()<9
   &&"NONE".equals(RESULT.has("throttle_after_minute")?RESULT.get("throttle_after_minute").getAsString():"");
  RESULT.addProperty("passed",passed);
  RESULT.addProperty("visual_review","required");
  try{Files.writeString(output.resolve("result.json"),RESULT.toString());}catch(Exception e){throw new RuntimeException(e);}
  if(c.getSingleplayerServer()!=null)c.getSingleplayerServer().execute(()->{var p=server(c);if(p!=null)JakPlayers.controlForProbe(p,false);});
  c.stop();
 }
}
