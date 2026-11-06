package dev.jakmc.client;
import com.google.gson.JsonObject;
import dev.jakmc.player.JakPlayers;
import net.fabricmc.fabric.api.client.event.lifecycle.v1.ClientTickEvents;
import net.minecraft.client.CameraType;
import net.minecraft.client.Minecraft;
import net.minecraft.client.Screenshot;
import net.minecraft.core.BlockPos;
import net.minecraft.core.registries.BuiltInRegistries;
import net.minecraft.resources.ResourceKey;
import net.minecraft.server.level.ServerPlayer;
import net.minecraft.world.level.biome.Biome;
import net.minecraft.world.level.biome.Biomes;
import net.minecraft.world.level.levelgen.Heightmap;
import net.minecraft.world.phys.Vec3;
import java.nio.file.Files;
import java.nio.file.Path;
import java.util.*;

/**
 * Freshly generated Jak lands (Rust jak_lands): the nearest desert, snowy
 * plains and swamp to spawn, each looked at from above and its blocks counted
 * round the spot. Needs a fresh world (chunks made before keep their blocks).
 */
public final class LandsScene {
 private record Target(String name,ResourceKey<Biome> biome,List<String> vanilla){}
 private static final List<Target> TARGETS=List.of(
  new Target("wasteland",Biomes.DESERT,List.of("minecraft:sand","minecraft:sandstone","minecraft:dead_bush")),
  new Target("snowy",Biomes.SNOWY_PLAINS,List.of("minecraft:snow_block","minecraft:snow","minecraft:grass_block","minecraft:spruce_log")),
  new Target("swamp",Biomes.SWAMP,List.of("minecraft:grass_block","minecraft:oak_log","minecraft:oak_leaves","minecraft:vine")));
 private static final int STAGE=160;
 private static boolean running;private static int tick;private static UUID player;private static Path output;
 private static final JsonObject RESULT=new JsonObject();
 private static final BlockPos[] SPOTS=new BlockPos[TARGETS.size()];
 private static final int[] TOPS=new int[TARGETS.size()];
 private static InputScene.CameraView view;
 public static InputScene.CameraView camera(){return view;}
 public static void initialize(){ClientTickEvents.END_CLIENT_TICK.register(LandsScene::tick);}
 public static void start(Minecraft c){
  if(running||c.player==null||c.getSingleplayerServer()==null)throw new IllegalStateException("Open isolated dev world");
  tick=0;RESULT.entrySet().clear();Arrays.fill(SPOTS,null);
  player=c.player.getUUID();output=c.gameDirectory.toPath().resolve("jakmc/evidence/lands");
  try{Files.createDirectories(output);}catch(Exception e){throw new RuntimeException(e);}
  running=true;c.options.setCameraType(CameraType.THIRD_PERSON_BACK);
  c.getSingleplayerServer().execute(()->{var p=server(c);if(p!=null){JakPlayers.controlForProbe(p,true);
   var s=c.getSingleplayerServer();s.getCommands().performPrefixedCommand(s.createCommandSourceStack(),"gamemode creative "+p.getScoreboardName());
   s.getCommands().performPrefixedCommand(s.createCommandSourceStack(),"time set noon");}});
 }
 private static ServerPlayer server(Minecraft c){return c.getSingleplayerServer().getPlayerList().getPlayer(player);}
 private static void tick(Minecraft c){
  if(!running)return;
  if(c.player==null||c.getSingleplayerServer()==null){finish(c,"lost world");return;}
  int t=++tick,i=(t-1)/STAGE,k=(t-1)%STAGE;
  if(i>=TARGETS.size()){finish(c,"complete");return;}
  var target=TARGETS.get(i);
  c.getSingleplayerServer().execute(()->{
   var p=server(c);if(p==null)return;var level=p.level();
   if(k==5){
    var found=level.findClosestBiome3d(b->b.is(target.biome),BlockPos.containing(p.position().x,80,p.position().z),6400,32,64);
    if(found==null){RESULT.addProperty(target.name+"_found",false);return;}
    BlockPos spot=found.getFirst();
    // The nearest point is the biome's edge: step inward to one whose ring 24 blocks round is the same biome.
    search:
    for(int r=0;r<=256;r+=16)for(int dx=-r;dx<=r;dx+=16)for(int dz=-r;dz<=r;dz+=16){
     if(Math.max(Math.abs(dx),Math.abs(dz))!=r)continue;
     var c0=spot.offset(dx,0,dz);boolean deep=true;
     for(int[] d:new int[][]{{0,0},{24,0},{-24,0},{0,24},{0,-24},{17,17},{-17,-17},{17,-17},{-17,17}})
      deep&=level.getNoiseBiome((c0.getX()+d[0])>>2,c0.getY()>>2,(c0.getZ()+d[1])>>2).is(target.biome);
     if(deep){spot=c0;break search;}
    }
    SPOTS[i]=spot;RESULT.addProperty(target.name+"_found",true);
    p.teleportTo(SPOTS[i].getX()+.5,220,SPOTS[i].getZ()+.5);
   }
   if(SPOTS[i]==null)return;
   if(k==70){TOPS[i]=level.getHeight(Heightmap.Types.MOTION_BLOCKING_NO_LEAVES,SPOTS[i].getX(),SPOTS[i].getZ());p.teleportTo(SPOTS[i].getX()+.5,TOPS[i]+1,SPOTS[i].getZ()+.5);}
   if(k==120){
    var counts=new TreeMap<String,Integer>();
    var at=new BlockPos.MutableBlockPos();
    for(int x=-24;x<=24;x++)for(int z=-24;z<=24;z++){
     int top=level.getHeight(Heightmap.Types.WORLD_SURFACE,SPOTS[i].getX()+x,SPOTS[i].getZ()+z);
     for(int y=top-12;y<=top;y++){
      var id=BuiltInRegistries.BLOCK.getKey(level.getBlockState(at.set(SPOTS[i].getX()+x,y,SPOTS[i].getZ()+z)).getBlock()).toString();
      if(id.startsWith("jak:")||target.vanilla.contains(id))counts.merge(id,1,Integer::sum);
     }
    }
    var json=new JsonObject();counts.forEach(json::addProperty);
    RESULT.add(target.name+"_blocks",json);
    int jak=counts.entrySet().stream().filter(e->e.getKey().startsWith("jak:")).mapToInt(Map.Entry::getValue).sum();
    int vanilla=counts.entrySet().stream().filter(e->!e.getKey().startsWith("jak:")).mapToInt(Map.Entry::getValue).sum();
    RESULT.addProperty(target.name+"_jak",jak);RESULT.addProperty(target.name+"_vanilla",vanilla);
    RESULT.addProperty(target.name+"_spot",SPOTS[i].toShortString());
   }
  });
  if(SPOTS[i]!=null&&k>=75){
   Vec3 spot=new Vec3(SPOTS[i].getX()+.5,TOPS[i],SPOTS[i].getZ()+.5);
   look(spot.add(-22,18,-22),spot.add(0,-2,0));
   if(k==130)capture(c,"lands-"+target.name+".png");
   if(k==150){look(spot.add(-6,4,-9),spot.add(0,0,0));}
   if(k==158)capture(c,"lands-"+target.name+"-near.png");
  }else view=null;
 }
 private static void look(Vec3 at,Vec3 target){
  Vec3 d=target.subtract(at);
  view=new InputScene.CameraView(at,(float)Math.toDegrees(Math.atan2(-d.x,d.z)),(float)Math.toDegrees(Math.atan2(-d.y,d.horizontalDistance())));
 }
 private static void capture(Minecraft c,String name){Screenshot.grab(c.gameDirectory,name,c.gameRenderer.mainRenderTarget(),1,m->{});}
 private static void finish(Minecraft c,String reason){
  running=false;view=null;RESULT.addProperty("reason",reason);
  var lands=new JsonObject();
  lands.addProperty("recorded",dev.jakmc.worldgen.JakLands.RECORDED.get());lands.addProperty("columns_with_fraction",dev.jakmc.worldgen.JakLands.FRACTIONS.get());
  lands.addProperty("shaped_with_record",dev.jakmc.worldgen.JakLands.WITH_RECORD.get());lands.addProperty("shaped_without_record",dev.jakmc.worldgen.JakLands.WITHOUT_RECORD.get());
  lands.addProperty("eighths_laid",dev.jakmc.worldgen.JakLands.EIGHTHS_LAID.get());lands.addProperty("sample",dev.jakmc.worldgen.JakLands.sample);
  RESULT.add("generator",lands);
  boolean passed=reason.equals("complete");
  for(var target:TARGETS){
   String n=target.name;
   passed&=RESULT.has(n+"_jak")&&RESULT.get(n+"_jak").getAsInt()>RESULT.get(n+"_vanilla").getAsInt()*4;
  }
  RESULT.addProperty("passed",passed);RESULT.addProperty("visual_review","required");
  try{Files.writeString(output.resolve("result.json"),RESULT.toString());}catch(Exception e){throw new RuntimeException(e);}
  if(c.getSingleplayerServer()!=null)c.getSingleplayerServer().execute(()->{var p=server(c);if(p!=null)JakPlayers.controlForProbe(p,false);});
  c.stop();
 }
}
