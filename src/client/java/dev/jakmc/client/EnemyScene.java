package dev.jakmc.client;

import com.google.gson.*;
import dev.jakmc.*;
import dev.jakmc.entity.*;
import dev.jakmc.client.model.JakEnemyRenderer;
import dev.jakmc.player.JakPlayers;
import net.fabricmc.fabric.api.client.event.lifecycle.v1.ClientTickEvents;
import net.minecraft.client.*;
import net.minecraft.world.entity.*;
import net.minecraft.world.entity.item.ItemEntity;
import net.minecraft.world.phys.*;
import java.nio.file.*;
import java.util.*;
import java.util.concurrent.ConcurrentHashMap;

/** One bounded run: every original body, a nonfatal hit, death and Metal Head loot.
 * No desktop input; uses only the isolated development world and exits itself. */
public final class EnemyScene {
 private static boolean running;private static int index,phase,deadline;private static volatile int entityId=-1;
 private static long lastDraw;
 private static int extraPhase;private static volatile Vec3 original,waterStart;private static volatile float waterHealth;private static volatile boolean climbed,attacked,airDepleted,floated;private static int waterVertices;
 private static volatile JsonObject chaseFinal;
 private static boolean combatOnly;private static JsonObject equipment;
 private static List<String> kinds;private static Path output;
 private static final Map<String,JsonObject> RESULTS=new ConcurrentHashMap<>();
 private static final Set<UUID> SPAWNED=ConcurrentHashMap.newKeySet();
 public static boolean active(){return running;}
 public static void initialize(){ClientTickEvents.END_CLIENT_TICK.register(EnemyScene::tick);}
 public static void start(Minecraft c){
  if(c.player==null||c.getSingleplayerServer()==null)throw new IllegalStateException("Open the isolated development world");
  if(running)throw new IllegalStateException("Enemy scene already running");
  kinds=List.copyOf(JakEntities.TYPES.keySet());RESULTS.clear();SPAWNED.clear();JakEnemyRenderer.seen.clear();JakEnemyRenderer.seenClips.clear();index=0;phase=-15;deadline=0;entityId=-1;
  output=c.gameDirectory.toPath().resolve("jakmc/evidence/enemies");try{Files.createDirectories(output);}catch(Exception e){throw new RuntimeException(e);}
  combatOnly=false;equipment=null;extraPhase=0;chaseFinal=null;climbed=false;attacked=false;airDepleted=false;floated=false;waterVertices=0;original=c.player.position();
  lastDraw=JakEnemyRenderer.rendered;running=true;c.gui.setScreen(null);c.options.setCameraType(CameraType.THIRD_PERSON_BACK);c.player.setYRot(0);c.player.setXRot(0);
  var uuid=c.player.getUUID();c.getSingleplayerServer().execute(()->{
   var server=c.getSingleplayerServer();var p=server.getPlayerList().getPlayer(uuid);if(p==null)return;
   server.getCommands().performPrefixedCommand(server.createCommandSourceStack(),"time set noon");server.getCommands().performPrefixedCommand(server.createCommandSourceStack(),"weather clear");
   var at=p.blockPosition();for(int x=-5;x<=5;x++)for(int z=-6;z<=11;z++){
    p.level().setBlock(at.offset(x,-1,z),net.minecraft.world.level.block.Blocks.SMOOTH_STONE.defaultBlockState(),3);
    for(int y=0;y<=7;y++)p.level().setBlock(at.offset(x,y,z),net.minecraft.world.level.block.Blocks.AIR.defaultBlockState(),3);
   }
   p.setPos(at.getX()+.5,at.getY(),at.getZ()+.5);p.setYRot(0);p.setXRot(0);p.setGameMode(net.minecraft.world.level.GameType.CREATIVE);p.setAttached(JakData.ENABLED,true);p.setAttached(JakData.BOARD,net.minecraft.world.item.ItemStack.EMPTY);p.setItemInHand(net.minecraft.world.InteractionHand.MAIN_HAND,net.minecraft.world.item.ItemStack.EMPTY);JakPlayers.controlForProbe(p,true);
  });
 }
 public static void startSystems(Minecraft c){start(c);kinds=List.of();output=c.gameDirectory.toPath().resolve("jakmc/evidence/systems");try{Files.createDirectories(output);}catch(Exception e){throw new RuntimeException(e);}}
 public static void startCombat(Minecraft c){startSystems(c);combatOnly=true;output=c.gameDirectory.toPath().resolve("jakmc/evidence/combat");try{Files.createDirectories(output);}catch(Exception e){throw new RuntimeException(e);}}
 private static void tick(Minecraft c){
  if(!running)return;if(c.player==null||c.getSingleplayerServer()==null){running=false;c.stop();return;}
  if(++deadline>1800){finish(c,"Deadline exceeded");return;}
  if(index>=kinds.size()){extras(c);return;}
  String kind=kinds.get(index);var uuid=c.player.getUUID();
  if(phase>=2){if(!JakEnemyRenderer.seenClips.containsKey(entityId)||lastDraw==JakEnemyRenderer.rendered)return;lastDraw=JakEnemyRenderer.rendered;}
  int t=++phase;
  if(t==0){entityId=-1;c.getSingleplayerServer().execute(()->{
   var p=c.getSingleplayerServer().getPlayerList().getPlayer(uuid);if(p==null)return;
   var enemy=JakEntities.TYPES.get(kind).create(p.level(),EntitySpawnReason.COMMAND);if(enemy==null)throw new IllegalStateException("Cannot create "+kind);
   enemy.setPos(p.getX()+2.2,p.getY(),p.getZ()+2.5);enemy.setYRot(180);enemy.setYBodyRot(180);enemy.setYHeadRot(180);enemy.setNoAi(true);enemy.setPersistenceRequired();
   p.level().addFreshEntity(enemy);SPAWNED.add(enemy.getUUID());entityId=enemy.getId();
   JsonObject row=new JsonObject();row.addProperty("id",entityId);row.addProperty("initial_health",enemy.getHealth());RESULTS.put(kind,row);
  });}
  if(t==8){capture(c,kind+"-idle.png");c.gui.hud.getChat().clearMessages(false);}
  if(t==10)c.getSingleplayerServer().execute(()->{
   var p=c.getSingleplayerServer().getPlayerList().getPlayer(uuid);if(p==null)return;var e=p.level().getEntity(entityId);if(!(e instanceof JakEnemy enemy))return;
   float before=enemy.getHealth();enemy.setNoAi(false);boolean hit=enemy.hurtServer(p.level(),p.damageSources().playerAttack(p),.5f);var r=RESULTS.get(kind);r.addProperty("nonfatal_health",enemy.getHealth());r.addProperty("nonfatal_reaction",enemy.state()==JakEnemy.State.HURT);r.addProperty("nonfatal_damage",hit&&enemy.getHealth()<before);r.addProperty("shell_deflected",kind.equals("lurkercrab")&&!hit&&enemy.getHealth()==before);
  });
  if(t==15){capture(c,kind+"-hurt.png");var clips=JakEnemyRenderer.seenClips.get(entityId);RESULTS.get(kind).addProperty("rendered_clips",clips==null?"":String.join(",",clips));}
  if(t==18)c.getSingleplayerServer().execute(()->{
   var p=c.getSingleplayerServer().getPlayerList().getPlayer(uuid);if(p==null)return;var e=p.level().getEntity(entityId);if(!(e instanceof JakEnemy enemy))return;
   AABB region=enemy.getBoundingBox().inflate(3);int before=gems(p.level(),region);
   Set<UUID> old=new HashSet<>();for(var drop:p.level().getEntitiesOfClass(ItemEntity.class,region))old.add(drop.getUUID());
   enemy.hurtServer(p.level(),kind.equals("lurkercrab")?p.damageSources().genericKill():p.damageSources().playerAttack(p),10000);var r=RESULTS.get(kind);r.addProperty("dead",enemy.isDeadOrDying());r.addProperty("gem_drops",gems(p.level(),region)-before);r.addProperty("expected_gem_drops",enemy.kind.metalHead?1:0);
   for(var drop:p.level().getEntitiesOfClass(ItemEntity.class,region))if(!old.contains(drop.getUUID()))SPAWNED.add(drop.getUUID());
  });
  if(t==25)capture(c,kind+"-death.png");
  if(t==30){final int old=entityId;c.getSingleplayerServer().execute(()->{var p=c.getSingleplayerServer().getPlayerList().getPlayer(uuid);if(p!=null){var e=p.level().getEntity(old);if(e!=null)e.discard();}});index++;phase=-1;}
 }
 private static int gems(net.minecraft.server.level.ServerLevel level,AABB region){return level.getEntitiesOfClass(ItemEntity.class,region,e->e.getItem().is(dev.jakmc.content.JakItems.SKULL_GEM)).stream().mapToInt(e->e.getItem().getCount()).sum();}
 private static void extras(Minecraft c){
  int t=++extraPhase;var uuid=c.player.getUUID();
  if(t==1&&combatOnly)equipment=dev.jakmc.client.model.JakRenderer.equipmentAudit();
  if(t==1)c.getSingleplayerServer().execute(()->{
   var p=c.getSingleplayerServer().getPlayerList().getPlayer(uuid);if(p==null)return;
   p.setHealth(p.getMaxHealth());p.setGameMode(net.minecraft.world.level.GameType.SURVIVAL);
   if(combatOnly){p.setAttached(JakData.BOARD,new net.minecraft.world.item.ItemStack(dev.jakmc.content.JakItems.BOARD));p.setItemInHand(net.minecraft.world.InteractionHand.MAIN_HAND,new net.minecraft.world.item.ItemStack(dev.jakmc.content.JakItems.GUNS.get(3)));}
   var at=p.blockPosition();for(int x=-5;x<=5;x++)for(int z=3;z<=4;z++)p.level().setBlock(at.offset(x,0,z),net.minecraft.world.level.block.Blocks.SMOOTH_STONE.defaultBlockState(),3);
   var enemy=JakEntities.TYPES.get("grunt").create(p.level(),EntitySpawnReason.COMMAND);enemy.setPos(p.getX(),p.getY(),p.getZ()+8);enemy.setYRot(180);enemy.setPersistenceRequired();p.level().addFreshEntity(enemy);SPAWNED.add(enemy.getUUID());entityId=enemy.getId();
  });
  if(t>=2&&t<160)c.getSingleplayerServer().execute(()->{
   var p=c.getSingleplayerServer().getPlayerList().getPlayer(uuid);if(p==null)return;var e=p.level().getEntity(entityId);if(e instanceof JakEnemy enemy){climbed|=enemy.getY()>p.getY()+.5;attacked|=p.getHealth()<p.getMaxHealth();chaseFinal=enemy.diagnostics();}
  });
  if(t==90)capture(c,"chase-step.png");
  if(t==159&&combatOnly)capture(c,"combat-final.png");
  if(t==160&&combatOnly){finish(c,null);return;}
  if(t==160)c.getSingleplayerServer().execute(()->{
   var p=c.getSingleplayerServer().getPlayerList().getPlayer(uuid);if(p==null)return;var old=p.level().getEntity(entityId);if(old!=null)old.discard();
   p.setGameMode(net.minecraft.world.level.GameType.SPECTATOR);p.setAttached(JakData.ENABLED,false);
   var center=p.blockPosition().offset(2,-7,7);
   for(int x=-2;x<=2;x++)for(int z=-2;z<=2;z++)for(int y=0;y<=7;y++)p.level().setBlock(center.offset(x,y,z),(y==0?net.minecraft.world.level.block.Blocks.SEA_LANTERN:Math.abs(x)==2||Math.abs(z)==2?net.minecraft.world.level.block.Blocks.GLASS:net.minecraft.world.level.block.Blocks.WATER).defaultBlockState(),3);
   var enemy=JakEntities.TYPES.get("grunt").create(p.level(),EntitySpawnReason.COMMAND);enemy.setPos(center.getX()+.5,center.getY()+1,center.getZ()+.5);enemy.setAirSupply(0);enemy.setPersistenceRequired();waterStart=enemy.position();waterHealth=enemy.getHealth();p.level().addFreshEntity(enemy);SPAWNED.add(enemy.getUUID());entityId=enemy.getId();
   var server=c.getSingleplayerServer();server.getCommands().performPrefixedCommand(server.createCommandSourceStack(),"tp "+p.getScoreboardName()+" "+(center.getX()+.5)+" "+(center.getY()+2.5)+" "+(center.getZ()-1)+" 0 0");
   c.execute(()->{c.options.setCameraType(CameraType.FIRST_PERSON);c.player.setYRot(0);c.player.setXRot(0);});
  });
  if(t>160){waterVertices=Math.max(waterVertices,JakEffects.drawnVertices);c.getSingleplayerServer().execute(()->{
   var p=c.getSingleplayerServer().getPlayerList().getPlayer(uuid);if(p==null)return;var e=p.level().getEntity(entityId);if(e instanceof JakEnemy enemy){floated|=enemy.getY()>waterStart.y+.1;airDepleted|=enemy.getHealth()<waterHealth;}
  });}
  if(t==195)capture(c,"water.png");
  if(t>=205){finish(c,null);}
 }
 private static void capture(Minecraft c,String name){Screenshot.grab(c.gameDirectory,"enemy-"+name,c.gameRenderer.mainRenderTarget(),1,m->{});}
 private static void finish(Minecraft c,String failure){
  running=false;JsonObject report=new JsonObject();report.addProperty("scene",kinds.isEmpty()?"NPC chase/step, fluid/oxygen, original effects and normal shutdown":"17 original enemies: body, nonfatal reaction, death, skull loot");report.addProperty("rendered_types",JakEnemyRenderer.seen.size());report.addProperty("authored_sound_cues",JakEnemySounds.cuesPlayed);report.addProperty("visual_review","required");report.addProperty("chased_over_step",climbed);report.addProperty("attack_damaged_player",attacked);if(chaseFinal!=null)report.add("chase_final",chaseFinal);report.addProperty("water_floated",floated);report.addProperty("submersion_used_air_and_damaged",airDepleted);report.addProperty("water_effect_vertices",waterVertices);if(failure!=null)report.addProperty("failure",failure);JsonObject rows=new JsonObject();RESULTS.forEach(rows::add);report.add("enemies",rows);
  if(equipment!=null)report.add("equipment_geometry",equipment);
  try{Files.writeString(output.resolve("result.json"),new GsonBuilder().setPrettyPrinting().create().toJson(report));}catch(Exception e){throw new RuntimeException(e);}
  var uuid=c.player.getUUID();c.getSingleplayerServer().execute(()->{var p=c.getSingleplayerServer().getPlayerList().getPlayer(uuid);if(p!=null){JakPlayers.controlForProbe(p,false);for(UUID id:SPAWNED){var e=p.level().getEntity(id);if(e!=null)e.discard();}p.setGameMode(net.minecraft.world.level.GameType.CREATIVE);p.setHealth(p.getMaxHealth());p.setAttached(JakData.ENABLED,true);var server=c.getSingleplayerServer();server.getCommands().performPrefixedCommand(server.createCommandSourceStack(),"tp "+p.getScoreboardName()+" "+original.x+" "+original.y+" "+original.z+" 0 0");}c.execute(c::stop);});
 }
}
