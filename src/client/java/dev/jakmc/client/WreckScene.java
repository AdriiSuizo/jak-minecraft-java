package dev.jakmc.client;

import com.google.gson.JsonObject;
import dev.jakmc.entity.JakHellcat;
import dev.jakmc.entity.JakVehicle;
import dev.jakmc.entity.VehicleWreck;
import java.io.IOException;
import java.io.UncheckedIOException;
import java.nio.file.Files;
import java.util.UUID;
import net.fabricmc.fabric.api.client.event.lifecycle.v1.ClientTickEvents;
import net.minecraft.client.CameraType;
import net.minecraft.client.Minecraft;
import net.minecraft.client.Screenshot;
import net.minecraft.core.BlockPos;
import net.minecraft.server.level.ServerPlayer;
import net.minecraft.world.entity.EntitySpawnReason;
import net.minecraft.world.level.block.Blocks;
import net.minecraft.world.level.levelgen.Heightmap;
import net.minecraft.world.phys.AABB;
import net.minecraft.world.phys.Vec3;

/**
 * Vehicles blown up (zoomers.rs, jak_entities.rs): a Dune Hopper's wreck darkened with its wheels,
 * a HellCat's flickering then charred carcass and Jak thrown off beside it, then Jak's death in dark eco
 * (dark_eco.rs died_in_it: the group-dark-eco-death burst).
 */
public final class WreckScene {
 private static boolean running;
 private static int tick;
 private static UUID player;
 private static BlockPos floor;
 private static double jakBefore=Double.NaN,jakHighest=Double.NaN;
 private static final JsonObject RESULT=new JsonObject();
 private static InputScene.CameraView view;
 static InputScene.CameraView camera(){return view;}
 static void initialize(){ClientTickEvents.END_CLIENT_TICK.register(WreckScene::tick);}
 public static void start(Minecraft c){
  if(running||c.player==null||c.getSingleplayerServer()==null)throw new IllegalStateException("Open isolated dev world");
  tick=0;RESULT.entrySet().clear();jakBefore=jakHighest=Double.NaN;floor=null;view=null;
  player=c.player.getUUID();
  JakEffects.darkEcoDeaths=0;
  c.options.setCameraType(CameraType.THIRD_PERSON_BACK);
  var s=c.getSingleplayerServer();
  s.execute(()->{
   var p=s.getPlayerList().getPlayer(player);if(p==null)return;var level=p.level();
   s.getCommands().performPrefixedCommand(s.createCommandSourceStack(),"gamemode survival "+p.getScoreboardName());
   s.getCommands().performPrefixedCommand(s.createCommandSourceStack(),"time set noon");
   s.getCommands().performPrefixedCommand(s.createCommandSourceStack(),"weather clear");
   BlockPos at=p.blockPosition();
   int top=level.getHeight(Heightmap.Types.MOTION_BLOCKING_NO_LEAVES,at.getX(),at.getZ());
   var base=new BlockPos(at.getX(),top+1,at.getZ());
   for(int x=-14;x<=14;x++)for(int z=-14;z<=14;z++){
    level.setBlock(base.offset(x,-1,z),Blocks.STONE.defaultBlockState(),3);
    for(int y=0;y<10;y++)level.setBlock(base.offset(x,y,z),Blocks.AIR.defaultBlockState(),3);
   }
   p.teleportTo(base.getX()+.5,base.getY(),base.getZ()+.5);p.setYRot(0);
   p.setHealth(p.getMaxHealth());
   floor=base;
   c.execute(()->running=true);
  });
 }
 private static ServerPlayer server(Minecraft c){return c.getSingleplayerServer()==null?null:c.getSingleplayerServer().getPlayerList().getPlayer(player);}
 private static void look(Vec3 eye,Vec3 target){
  var d=target.subtract(eye);
  view=new InputScene.CameraView(eye,(float)Math.toDegrees(Math.atan2(-d.x,d.z)),(float)-Math.toDegrees(Math.atan2(d.y,d.horizontalDistance())));
 }
 private static void capture(Minecraft c,String name){Screenshot.grab(c.gameDirectory,name,c.gameRenderer.mainRenderTarget(),1,_-> {});}
 private static void tick(Minecraft c){
  if(!running||floor==null)return;
  int t=++tick;
  var s=c.getSingleplayerServer();
  Vec3 middle=Vec3.atBottomCenterOf(floor);
  Vec3 toad=middle.add(-7,0,7),cat=middle.add(1.4,0,0);
  if(t==2)s.execute(()->{
   var p=server(c);if(p==null)return;
   var car=JakVehicle.Model.TOAD.type.create(p.level(),EntitySpawnReason.EVENT);
   car.snapTo(toad.x,toad.y,toad.z,45,0);p.level().addFreshEntity(car);
  });
  if(t==10)look(toad.add(7,5,-7),toad.add(0,1,0));
  // zoomers.rs: no hit points left, it blows up.
  if(t==40)s.execute(()->{
   var p=server(c);if(p==null)return;
   for(var car:p.level().getEntitiesOfClass(JakVehicle.class,new AABB(toad,toad).inflate(4)))car.hurtServer(p.level(),p.level().damageSources().generic(),1000);
  });
  if(t==44)capture(c,"wreck-toad-burst.png");
  if(t==70){
   capture(c,"wreck-toad-charred.png");
   s.execute(()->{
    var p=server(c);if(p==null)return;
    var wrecks=p.level().getEntitiesOfClass(VehicleWreck.class,new AABB(toad,toad).inflate(12),w->w.model()>=0);
    RESULT.addProperty("toad_wrecks",wrecks.size());
    RESULT.addProperty("toad_wreck_wheels",wrecks.stream().anyMatch(w->!w.wheels().isEmpty()));
   });
  }
  // jak_entities.rs: a HellCat beside Jak, its hull all but gone, then one more hit.
  if(t==80)s.execute(()->{
   var p=server(c);if(p==null)return;
   var craft=JakHellcat.TYPE.create(p.level(),EntitySpawnReason.EVENT);
   craft.devHull(.004f);
   craft.snapTo(cat.x,cat.y+.5,cat.z,90,0);p.level().addFreshEntity(craft);
  });
  if(t==95)look(middle.add(-6,3.5,-6),middle.add(.7,1,0));
  if(t==110)s.execute(()->{
   var p=server(c);if(p==null)return;
   jakBefore=p.getY();jakHighest=p.getY();
   for(var craft:p.level().getEntitiesOfClass(JakHellcat.class,new AABB(cat,cat).inflate(4)))craft.hurtServer(p.level(),p.level().damageSources().generic(),1);
  });
  if(t>110&&t<150)s.execute(()->{var p=server(c);if(p!=null&&!Double.isNaN(jakHighest))jakHighest=Math.max(jakHighest,p.getY());});
  if(t==114)capture(c,"wreck-hellcat-burning.png");
  if(t==150){
   capture(c,"wreck-hellcat-charred.png");
   s.execute(()->{
    var p=server(c);if(p==null)return;
    RESULT.addProperty("hellcat_wrecked",!p.level().getEntitiesOfClass(JakHellcat.class,new AABB(cat,cat).inflate(30),JakHellcat::wrecked).isEmpty());
    RESULT.addProperty("jak_before",jakBefore);RESULT.addProperty("jak_highest",jakHighest);
    RESULT.addProperty("jak_thrown",jakHighest-jakBefore>1.5);
   });
  }
  // dark_eco.rs: a pool under Jak; he dies in it.
  if(t==175)s.execute(()->{
   var p=server(c);if(p==null)return;
   var at=p.blockPosition();
   for(int x=-1;x<=1;x++)for(int z=-1;z<=1;z++)p.level().setBlock(at.offset(x,0,z),dev.jakmc.content.darkeco.DarkEco.BLOCK.defaultBlockState(),3);
  });
  if(t==176)look(c.player.position().add(-4,3,-4),c.player.position().add(0,.5,0));
  if(t==181)capture(c,"wreck-dark-eco-death.png");
  if(t==200){
   RESULT.addProperty("dark_eco_bursts",JakEffects.darkEcoDeaths);
   boolean ok=RESULT.has("toad_wreck_wheels")&&RESULT.get("toad_wreck_wheels").getAsBoolean()
    &&RESULT.has("hellcat_wrecked")&&RESULT.get("hellcat_wrecked").getAsBoolean()
    &&RESULT.get("jak_thrown").getAsBoolean()&&JakEffects.darkEcoDeaths>0;
   RESULT.addProperty("reason",ok?"complete":"Wreck acceptance failed");
   try{
    var dir=c.gameDirectory.toPath().resolve("jakmc/evidence/wrecks");
    Files.createDirectories(dir);Files.writeString(dir.resolve("result.json"),RESULT.toString());
   }catch(IOException failure){throw new UncheckedIOException(failure);}
   running=false;view=null;
   c.stop();
  }
 }
}
