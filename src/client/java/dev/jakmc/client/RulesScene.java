package dev.jakmc.client;
import com.google.gson.JsonObject;
import dev.jakmc.JakData;
import dev.jakmc.content.JakItems;
import dev.jakmc.entity.JakHellcat;
import dev.jakmc.entity.JakVehicle;
import dev.jakmc.entity.VehicleItem;
import dev.jakmc.player.JakDamage;
import dev.jakmc.player.JakPlayers;
import net.fabricmc.fabric.api.client.event.lifecycle.v1.ClientTickEvents;
import net.minecraft.client.CameraType;
import net.minecraft.client.Minecraft;
import net.minecraft.client.Screenshot;
import net.minecraft.core.BlockPos;
import net.minecraft.core.registries.Registries;
import net.minecraft.resources.ResourceKey;
import net.minecraft.server.level.ServerPlayer;
import net.minecraft.world.entity.EntitySpawnReason;
import net.minecraft.world.entity.SlotAccess;
import net.minecraft.world.entity.item.ItemEntity;
import net.minecraft.world.inventory.ClickAction;
import net.minecraft.world.item.Item;
import net.minecraft.world.item.ItemStack;
import net.minecraft.world.level.Level;
import net.minecraft.world.level.block.Blocks;
import net.minecraft.world.phys.AABB;
import net.minecraft.world.phys.Vec3;
import java.nio.file.Files;
import java.nio.file.Path;
import java.util.*;

/**
 * The Rust rules restored on 05/11: blows crack vehicles into their items
 * (keeping the hull), green eco mends a carried vehicle, a sunk zoomer comes
 * back as its item, the Rust recipes load, Minecraft's footsteps stay silent
 * under Jak, and the same Jak survives a teleport and a dimension change.
 */
public final class RulesScene {
 private static boolean running,counting,finishing;
 private static int tick,vanillaSteps,jakSteps;
 private static long session;
 private static UUID player,cat,bike,sunk;
 private static Path output;
 private static BlockPos base;
 private static InputScene.CameraView view;
 private static final JsonObject RESULT=new JsonObject();
 private static final List<ItemStack> INVENTORY=new ArrayList<>();
 private static final Set<UUID> IDS=new HashSet<>();
 private static final List<String> RECIPES=List.of("jet_board","hellcat","helldog","zoomer_bike_a","sand_shark","scatter_gun",
  "wave_concussor","super_nova","red_ammo_from_redstone","dark_eco","wasteland_rock","totem_stone");
 private static final net.minecraft.client.sounds.SoundEventListener EAR=(sound,events,range)->{
  if(!counting)return;
  var id=sound.getIdentifier();
  if(id.getNamespace().equals("minecraft")&&id.getPath().endsWith(".step"))vanillaSteps++;
  if(id.getNamespace().equals(dev.jakmc.JakMod.ID)&&(id.getPath().contains("/walk_")||id.getPath().contains("/run_")))jakSteps++;
 };

 public static InputScene.CameraView camera(){return view;}
 public static void initialize(){ClientTickEvents.END_CLIENT_TICK.register(RulesScene::tick);}

 public static void start(Minecraft c){
  if(running||c.player==null||c.getSingleplayerServer()==null)throw new IllegalStateException("Open isolated dev world");
  tick=vanillaSteps=jakSteps=0;counting=finishing=false;session=0;base=null;cat=bike=sunk=null;
  RESULT.entrySet().clear();IDS.clear();INVENTORY.clear();
  player=c.player.getUUID();
  output=c.gameDirectory.toPath().resolve("jakmc/evidence/rules");
  try{Files.createDirectories(output);}catch(Exception e){throw new RuntimeException(e);}
  running=true;
  c.options.setCameraType(CameraType.THIRD_PERSON_BACK);
  c.getSoundManager().addListener(EAR);
  c.getSingleplayerServer().execute(()->{
   var p=server(c);if(p==null)return;var level=p.level();
   BlockPos at=p.blockPosition().offset(24,0,0);
   int top=at.getY();
   for(int x=-8;x<=8;x++)for(int z=-8;z<=34;z++)
    top=Math.max(top,level.getHeight(net.minecraft.world.level.levelgen.Heightmap.Types.MOTION_BLOCKING_NO_LEAVES,at.getX()+x,at.getZ()+z));
   BlockPos floor=new BlockPos(at.getX(),top+1,at.getZ());
   for(int x=-8;x<=8;x++)for(int z=-8;z<=34;z++){
    level.setBlock(floor.offset(x,-1,z),Blocks.STONE.defaultBlockState(),3);
    for(int y=0;y<8;y++)level.setBlock(floor.offset(x,y,z),Blocks.AIR.defaultBlockState(),3);
   }
   // A pit of water three deep for the sunk zoomer.
   // (Walled: the floor may stand over open air, and an open pit drains.)
   for(int x=-8;x<=-4;x++)for(int z=7;z<=11;z++)for(int y=-5;y<=-1;y++){
    boolean inside=x>=-7&&x<=-5&&z>=8&&z<=10&&y>=-4;
    level.setBlock(floor.offset(x,y,z),inside?Blocks.WATER.defaultBlockState():Blocks.STONE.defaultBlockState(),3);
   }
   for(int i=0;i<p.getInventory().getContainerSize();i++){INVENTORY.add(p.getInventory().getItem(i).copy());p.getInventory().setItem(i,ItemStack.EMPTY);}
   p.setAttached(JakData.ENABLED,true);
   p.setYRot(0);p.setYBodyRot(0);
   p.teleportTo(floor.getX()+.5,floor.getY(),floor.getZ()-6+.5);
   JakPlayers.controlForProbe(p,true);
   var h=JakHellcat.TYPE.create(level,EntitySpawnReason.COMMAND);
   h.snapTo(floor.getX()+5.5,floor.getY()+1.28,floor.getZ()+2.5,90,0);level.addFreshEntity(h);cat=h.getUUID();IDS.add(cat);
   var b=JakVehicle.Model.BIKEA.type.create(level,EntitySpawnReason.COMMAND);
   b.snapTo(floor.getX()-4.5,floor.getY()+JakVehicle.Model.BIKEA.parkingClearance(),floor.getZ()+2.5,0,0);level.addFreshEntity(b);bike=b.getUUID();IDS.add(bike);
   var s=JakVehicle.Model.BIKEB.type.create(level,EntitySpawnReason.COMMAND);
   s.snapTo(floor.getX()-5.5,floor.getY()-3+.05,floor.getZ()+9.5,0,0);level.addFreshEntity(s);sunk=s.getUUID();IDS.add(sunk);
   var recipes=c.getSingleplayerServer().getRecipeManager();
   int found=0;var missing=new StringBuilder();
   for(String id:RECIPES){
    if(recipes.byKey(ResourceKey.create(Registries.RECIPE,JakData.id(id))).isPresent())found++;
    else missing.append(id).append(' ');
   }
   RESULT.addProperty("rust_recipes_loaded",found);
   RESULT.addProperty("rust_recipes_missing",missing.toString().trim());
   base=floor;
  });
 }

 private static ServerPlayer server(Minecraft c){return c.getSingleplayerServer().getPlayerList().getPlayer(player);}
 private static void blow(ServerPlayer p,UUID id){var e=p.level().getEntity(id);if(e!=null)e.hurtServer(p.level(),p.damageSources().playerAttack(p),4);}
 private static ItemEntity dropped(ServerPlayer p,Vec3 at,Item item){
  for(var e:p.level().getEntitiesOfClass(ItemEntity.class,new AABB(at,at).inflate(6)))if(e.getItem().is(item))return e;
  return null;
 }
 private static void command(Minecraft c,ServerPlayer p,String text){
  var server=c.getSingleplayerServer();
  server.getCommands().performPrefixedCommand(server.createCommandSourceStack(),"execute as "+p.getScoreboardName()+" at @s "+text);
 }

 private static void tick(Minecraft c){
  if(!running||finishing)return;
  if(c.player==null||c.getSingleplayerServer()==null){finish(c,"lost world");return;}
  int t=++tick;
  if(base==null){if(t>200)finish(c,"setup never ran");return;}
  Vec3 stage=Vec3.atBottomCenterOf(base);
  if(t<100)look(stage.add(0,7,-13),stage.add(0,1,4));
  c.getSingleplayerServer().execute(()->{
   var p=server(c);if(p==null)return;
   if(t==40){var b=p.level().getEntity(bike);if(b!=null)b.hurtServer(p.level(),JakDamage.shot(p,false),5);}
   if(t==45||t==55||t==65){blow(p,cat);blow(p,bike);}
   if(t==58)RESULT.addProperty("hellcat_survives_two_blows",p.level().getEntity(cat)!=null);
   if(t==75){
    var item=dropped(p,stage.add(5,1,2),JakHellcat.ITEM);
    RESULT.addProperty("hellcat_third_blow_item",p.level().getEntity(cat)==null&&item!=null);
    var bi=dropped(p,stage.add(-4,1,2),JakVehicle.Model.BIKEA.item);
    float hull=bi==null?-1:VehicleItem.health(bi.getItem());
    RESULT.addProperty("zoomer_third_blow_item",p.level().getEntity(bike)==null&&bi!=null);
    RESULT.addProperty("zoomer_item_hull",hull);
    RESULT.addProperty("zoomer_shot_took_two_points",Math.abs(hull-13f/15f)<1e-3);
    var sunkItem=dropped(p,stage.add(-5,-2,9),JakVehicle.Model.BIKEB.item);
    RESULT.addProperty("sunk_zoomer_item",p.level().getEntity(sunk)==null&&sunkItem!=null);
    if(bi!=null){
     var stack=bi.getItem().copy();bi.discard();
     VehicleItem.setHealth(stack,.5f);
     p.getInventory().setItem(9,stack);
     var eco=new ItemStack(JakItems.GREEN_ECO,2);
     var slot=p.inventoryMenu.getSlot(9);
     var carried=SlotAccess.of(()->eco,x->{});
     var held=p.getInventory().getItem(9);
     boolean used=held.getItem().overrideOtherStackedOnMe(held,eco,slot,ClickAction.PRIMARY,p,carried);
     RESULT.addProperty("green_eco_mends_quarter",used&&Math.abs(VehicleItem.health(held)-.75f)<1e-3&&eco.getCount()==1);
     VehicleItem.setHealth(held,.9f);
     held.getItem().overrideOtherStackedOnMe(held,eco,slot,ClickAction.PRIMARY,p,carried);
     RESULT.addProperty("whole_item_has_no_bar",!held.getItem().isBarVisible(held));
    }
   }
   if(t==100)counting=true;
   if(t>=100&&t<170)JakPlayers.probeInput(p,0,0,1);
   if(t==170)JakPlayers.probeInput(p,0,0,0);
   if(t==175){counting=false;session=JakPlayers.probeSession(p);command(c,p,"run tp @s ~ ~ ~-20");}
   if(t==185){
    RESULT.addProperty("same_jak_after_teleport",session!=0&&JakPlayers.probeSession(p)==session);
    command(c,p,"in minecraft:the_nether run tp @s 8 120 8");
   }
   if(t==215)RESULT.addProperty("same_jak_in_nether",session!=0&&JakPlayers.probeSession(p)==session&&p.level().dimension()==Level.NETHER);
   if(t==220)command(c,p,"in minecraft:overworld run tp @s "+(base.getX()+.5)+" "+base.getY()+" "+(base.getZ()-6+.5));
   if(t==245)RESULT.addProperty("same_jak_back_home",session!=0&&JakPlayers.probeSession(p)==session&&p.level().dimension()==Level.OVERWORLD);
  });
  if(t>=100&&t<175)look(c.player.position().add(3,2.5,-4),c.player.position().add(0,1,0));
  if(t>=175)view=null;
  if(t==78)capture(c,"rules-items.png");
  if(t==140)capture(c,"rules-walk.png");
  if(t>=255){
   finishing=true;
   c.getSingleplayerServer().execute(()->c.execute(()->finish(c,"complete")));
  }
 }
 private static void look(Vec3 at,Vec3 target){
  Vec3 d=target.subtract(at);
  view=new InputScene.CameraView(at,(float)Math.toDegrees(Math.atan2(-d.x,d.z)),(float)Math.toDegrees(Math.atan2(-d.y,d.horizontalDistance())));
 }
 private static void capture(Minecraft c,String name){Screenshot.grab(c.gameDirectory,name,c.gameRenderer.mainRenderTarget(),1,m->{});}
 private static void finish(Minecraft c,String reason){
  running=false;view=null;
  c.getSoundManager().removeListener(EAR);
  RESULT.addProperty("vanilla_steps_under_jak",vanillaSteps);
  RESULT.addProperty("jak_steps",jakSteps);
  RESULT.addProperty("reason",reason);
  String[] checks={"hellcat_survives_two_blows","hellcat_third_blow_item","zoomer_third_blow_item","zoomer_shot_took_two_points",
   "sunk_zoomer_item","green_eco_mends_quarter","whole_item_has_no_bar","same_jak_after_teleport","same_jak_in_nether","same_jak_back_home"};
  boolean passed=reason.equals("complete")&&vanillaSteps==0&&jakSteps>0
   &&RESULT.has("rust_recipes_loaded")&&RESULT.get("rust_recipes_loaded").getAsInt()==RECIPES.size();
  for(String k:checks)passed&=RESULT.has(k)&&RESULT.get(k).getAsBoolean();
  RESULT.addProperty("passed",passed);
  RESULT.addProperty("visual_review","required");
  try{Files.writeString(output.resolve("result.json"),RESULT.toString());}catch(Exception e){throw new RuntimeException(e);}
  var server=c.getSingleplayerServer();
  if(server==null){c.stop();return;}
  server.execute(()->{
   var p=server.getPlayerList().getPlayer(player);
   if(p!=null){
    for(var level:server.getAllLevels())for(UUID id:IDS){var e=level.getEntity(id);if(e!=null)e.discard();}
    for(int i=0;i<INVENTORY.size();i++)p.getInventory().setItem(i,INVENTORY.get(i));
    JakPlayers.controlForProbe(p,false);
   }
   c.execute(c::stop);
  });
 }
}
