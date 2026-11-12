package dev.jakmc.client;
import com.google.gson.*;
import dev.jakmc.entity.*;
import dev.jakmc.client.model.JakCitizenRenderer;
import net.fabricmc.fabric.api.client.event.lifecycle.v1.ClientTickEvents;
import net.minecraft.client.*;
import net.minecraft.core.*;
import net.minecraft.server.level.*;
import net.minecraft.world.*;
import net.minecraft.world.entity.*;
import net.minecraft.world.entity.npc.villager.Villager;
import net.minecraft.world.item.*;
import net.minecraft.world.item.trading.*;
import net.minecraft.world.inventory.MerchantMenu;
import net.minecraft.world.level.block.*;
import net.minecraft.world.level.block.state.properties.BedPart;
import net.minecraft.world.level.storage.*;
import net.minecraft.world.phys.Vec3;
import net.minecraft.util.ProblemReporter;
import java.nio.file.*;
import java.util.*;
/** Bounded native entity/merchant/bed acceptance, in the isolated dev save only. */
public final class CitizenScene {
 private static boolean running;private static int tick;private static UUID player;private static BlockPos base;private static Vec3 center;private static Path output;private static InputScene.CameraView view;
 private static Runnable sleepCleanup;
 private static long priorSleepFrames;
 private static boolean playerSleep;
 private static final List<UUID> IDS=new ArrayList<>();private static final List<ItemStack> INVENTORY=new ArrayList<>();private static final JsonObject RESULT=new JsonObject();private static final Set<Integer> PHASES=new TreeSet<>();
 public static InputScene.CameraView camera(){return view;}
 public static void initialize(){ClientTickEvents.END_CLIENT_TICK.register(CitizenScene::tick);}
 public static void startPlayerSleep(Minecraft client){
  if(running||client.player==null||client.getSingleplayerServer()==null)throw new IllegalStateException("Open isolated dev world");
  var server=client.getSingleplayerServer();
  UUID id=client.player.getUUID();
  var camera=client.options.getCameraType();
  client.options.setCameraType(CameraType.THIRD_PERSON_BACK);
  priorSleepFrames=dev.jakmc.client.model.JakRenderer.sleepRenderSerial;
  server.execute(()->{
   var p=server.getPlayerList().getPlayer(id);
   var level=p.level();
   Vec3 old=p.position();
   boolean enabled=p.getAttachedOrCreate(dev.jakmc.JakData.ENABLED);
   var head=p.blockPosition().offset(3,0,0);
   int height=level.getHeight(net.minecraft.world.level.levelgen.Heightmap.Types.MOTION_BLOCKING,head.getX(),head.getZ());
   head=head.atY(Math.max(head.getY(),height)+1);
   var foot=head.west();
   var before=new LinkedHashMap<BlockPos,net.minecraft.world.level.block.state.BlockState>();
   for(var at:List.of(head,foot)){
    before.put(at.below(),level.getBlockState(at.below()));
    before.put(at,level.getBlockState(at));
    before.put(at.above(),level.getBlockState(at.above()));
    level.setBlockAndUpdate(at.below(),Blocks.SMOOTH_STONE.defaultBlockState());
    level.setBlockAndUpdate(at.above(),Blocks.AIR.defaultBlockState());
   }
   var bed=Blocks.BED.pick(DyeColor.RED).defaultBlockState().setValue(BedBlock.FACING,Direction.EAST);
   level.setBlockAndUpdate(foot,bed.setValue(BedBlock.PART,BedPart.FOOT));
   level.setBlockAndUpdate(head,bed.setValue(BedBlock.PART,BedPart.HEAD));
   server.getCommands().performPrefixedCommand(server.createCommandSourceStack(),"time set midnight");
   p.setAttached(dev.jakmc.JakData.ENABLED,true);
   boolean occupied=p.startSleeping(head)&&level.getBlockState(head).getValue(BedBlock.OCCUPIED);
   Vec3 target=Vec3.atBottomCenterOf(head).add(-.5,.65,0);
   client.execute(()->{
    RESULT.entrySet().clear();
    RESULT.addProperty("native_occupied",occupied);
    look(target.add(1.4,2.4,-2.7),target);
    tick=0;
    running=playerSleep=true;
    sleepCleanup=()->{
     client.options.setCameraType(camera);
     server.execute(()->{
      if(p.isSleeping())p.stopSleepInBed(true,true);
      before.forEach(level::setBlockAndUpdate);
      p.setAttached(dev.jakmc.JakData.ENABLED,enabled);
      p.teleportTo(old.x,old.y,old.z);
     });
    };
   });
  });
 }

 private static void tickPlayerSleep(Minecraft client){
  if(++tick==30)capture(client,"jak-sleep.png");
  if(tick<38)return;
  RESULT.addProperty("native_player_asleep",client.player.isSleeping());
  RESULT.addProperty("original_sleep_pose",dev.jakmc.client.model.JakRenderer.sleepRenderSerial>priorSleepFrames);
  boolean pass=RESULT.entrySet().stream().allMatch(e->e.getValue().getAsBoolean());
  RESULT.addProperty("reason",pass?"complete":"Jak bed pose assertion failed");
  try{
   Path dir=client.gameDirectory.toPath().resolve("jakmc/evidence/player_sleep");
   Files.createDirectories(dir);
   Files.writeString(dir.resolve("result.json"),RESULT.toString());
  }catch(java.io.IOException failure){throw new java.io.UncheckedIOException(failure);}
  running=playerSleep=false;
  view=null;
  sleepCleanup.run();
  client.stop();
 }

 public static void start(Minecraft c){if(running||c.player==null||c.getSingleplayerServer()==null)throw new IllegalStateException("Open isolated dev world");tick=0;IDS.clear();INVENTORY.clear();RESULT.entrySet().clear();PHASES.clear();JakCitizenRenderer.SEEN.clear();JakCitizenRenderer.rendered=JakCitizenRenderer.sleepers=0;player=c.player.getUUID();output=c.gameDirectory.toPath().resolve("jakmc/evidence/citizens");try{Files.createDirectories(output);}catch(Exception e){throw new RuntimeException(e);}running=true;c.options.setCameraType(CameraType.THIRD_PERSON_BACK);
 c.getSingleplayerServer().execute(()->{var server=c.getSingleplayerServer();var p=server.getPlayerList().getPlayer(player);base=p.blockPosition().offset(20,0,0);int top=base.getY();for(int x=-7;x<=7;x++)for(int z=-3;z<=10;z++)top=Math.max(top,p.level().getHeight(net.minecraft.world.level.levelgen.Heightmap.Types.MOTION_BLOCKING_NO_LEAVES,base.getX()+x,base.getZ()+z));base=new BlockPos(base.getX(),top+1,base.getZ());center=Vec3.atBottomCenterOf(base.offset(0,0,3));
 for(int x=-7;x<=7;x++)for(int z=-3;z<=10;z++){p.level().setBlock(base.offset(x,-1,z),Blocks.SMOOTH_STONE.defaultBlockState(),3);for(int y=0;y<6;y++)p.level().setBlock(base.offset(x,y,z),Blocks.AIR.defaultBlockState(),3);}
 server.getCommands().performPrefixedCommand(server.createCommandSourceStack(),"time set noon");for(int i=0;i<p.getInventory().getContainerSize();i++){INVENTORY.add(p.getInventory().getItem(i).copy());p.getInventory().setItem(i,ItemStack.EMPTY);}p.setPos(center.add(0,0,-3));p.setYRot(0);p.setItemInHand(InteractionHand.MAIN_HAND,new ItemStack(Items.EMERALD,4));
 for(int i=0;i<3;i++){var egg=new ItemStack(JakCitizens.EGGS.get(i));var v=EntityTypes.VILLAGER.spawn(p.level(),egg,p,base.offset((i-1)*3,0,3),EntitySpawnReason.SPAWN_ITEM_USE,false,false);v.setNoAi(true);v.setYRot(180);v.setYBodyRot(180);v.setYHeadRot(180);IDS.add(v.getUUID());RESULT.addProperty("egg_variant_"+i,((CitizenState)v).citizenVariant()==i&&v.getType()==EntityTypes.VILLAGER);}
 var baby=EntityTypes.VILLAGER.create(p.level(),EntitySpawnReason.COMMAND);((CitizenState)baby).citizenVariant(2);baby.setAge(-24000);baby.setNoAi(true);baby.setPos(center.add(5,0,0));p.level().addFreshEntity(baby);IDS.add(baby.getUUID());
 var v=(Villager)p.level().getEntity(IDS.get(0));var offers=new MerchantOffers();offers.add(new MerchantOffer(new ItemCost(Items.EMERALD,2),new ItemStack(Items.BREAD,3),12,2,0));v.setOffers(offers);
 var saved=TagValueOutput.createWithContext(ProblemReporter.DISCARDING,v.registryAccess());v.saveWithoutId(saved);
 var copy=EntityTypes.VILLAGER.create(p.level(),EntitySpawnReason.LOAD);
 copy.load(TagValueInput.create(ProblemReporter.DISCARDING,copy.registryAccess(),saved.buildResult()));
 var restored=copy.getOffers();
 // Vanilla's offer is restored alongside the seven Jak offers added by CitizenTradesMixin.
 RESULT.addProperty("native_save_roundtrip",copy.getType()==EntityTypes.VILLAGER&&((CitizenState)copy).citizenVariant()==0
     &&restored.size()==8&&restored.stream().filter(offer->offer.getResult().is(Items.BREAD)
         &&offer.getResult().getCount()==3&&offer.getCostA().getCount()==2&&offer.getMaxUses()==12).count()==1
     &&copy.getOffers().size()==8);
 });}
 private static void tick(Minecraft c){if(!running)return;if(playerSleep){tickPlayerSleep(c);return;}if(c.player==null||c.getSingleplayerServer()==null){finish(c,"lost world");return;}int t=++tick;if(center==null)return;c.gui.hud.getChat().clearMessages(false);
 if(t<85)look(center.add(0,2.1,-7),center.add(0,.9,0));else if(t<155)look(center.add(0,5,-5),center.add(0,.3,0));else look(center.add(-1.5,2,-4),center.add(-3,.7,0));
 if(t==35)capture(c,"citizens-standing.png");if(t==75){RESULT.addProperty("native_merchant_screen",c.gui.screen() instanceof net.minecraft.client.gui.screens.inventory.MerchantScreen);capture(c,"citizens-trade.png");}if(t==80)c.gui.setScreen(null);
 if(t==130)capture(c,"citizens-beds.png");if(t==110){look(center.add(2,1.8,-1),center.add(0,.7,0));capture(c,"citizens-bed-side.png");}if(t==172)capture(c,"citizens-hit.png");if(t==205)capture(c,"citizens-getup.png");if(t==270)capture(c,"citizens-walk.png");if(t==294)capture(c,"citizens-death.png");
 c.getSingleplayerServer().execute(()->{var p=c.getSingleplayerServer().getPlayerList().getPlayer(player);if(p==null)return;var v=(Villager)p.level().getEntity(IDS.get(0));
 if(t==38){p.setItemInHand(InteractionHand.MAIN_HAND,new ItemStack(JakCitizens.EGGS.get(1)));var before=p.level().getEntitiesOfClass(Villager.class,v.getBoundingBox().inflate(3)).stream().map(Entity::getUUID).collect(java.util.stream.Collectors.toSet());v.interact(p,InteractionHand.MAIN_HAND,Vec3.ZERO);var babies=p.level().getEntitiesOfClass(Villager.class,v.getBoundingBox().inflate(3),e->!before.contains(e.getUUID()));RESULT.addProperty("native_egg_offspring",babies.size()==1&&babies.getFirst().isBaby()&&((CitizenState)babies.getFirst()).citizenVariant()==1);for(var baby:babies){baby.discard();}p.setItemInHand(InteractionHand.MAIN_HAND,new ItemStack(Items.EMERALD,4));}
 if(t==45){var interaction=v.interact(p,InteractionHand.MAIN_HAND,Vec3.ZERO);RESULT.addProperty("native_trade_menu",p.containerMenu instanceof MerchantMenu);if(p.containerMenu instanceof MerchantMenu menu){menu.getSlot(0).set(new ItemStack(Items.EMERALD,2));menu.slotsChanged(p.getInventory());var result=menu.quickMoveStack(p,2);RESULT.addProperty("native_trade_transaction",result.is(Items.BREAD)&&result.getCount()==3&&menu.getSlot(0).getItem().isEmpty()&&v.getOffers().getFirst().getUses()==1);} }
 if(t==82)p.closeContainer();
 if(t==90){for(int i=0;i<3;i++){var sleeper=(Villager)p.level().getEntity(IDS.get(i));Direction d=new Direction[]{Direction.EAST,Direction.SOUTH,Direction.WEST}[i];BlockPos head=base.offset((i-1)*3,0,3),foot=head.relative(d.getOpposite());p.level().setBlock(foot,Blocks.BED.pick(DyeColor.RED).defaultBlockState().setValue(BedBlock.FACING,d).setValue(BedBlock.PART,BedPart.FOOT),3);p.level().setBlock(head,Blocks.BED.pick(DyeColor.RED).defaultBlockState().setValue(BedBlock.FACING,d).setValue(BedBlock.PART,BedPart.HEAD),3);boolean ok=sleeper.startSleeping(head);RESULT.addProperty("native_sleep_"+i,ok&&sleeper.isSleeping()&&p.level().getBlockState(head).getValue(BedBlock.OCCUPIED));}}
 if(t==155){for(int i=0;i<3;i++){var sleeper=(Villager)p.level().getEntity(IDS.get(i));sleeper.stopSleeping();for(int x=-7;x<=7;x++)for(int z=-1;z<=1;z++)p.level().setBlock(base.offset(x,0,3+z),Blocks.AIR.defaultBlockState(),3);sleeper.setPos(center.add((i-1)*3,0,0));}v.setNoAi(false);v.hurtServer(p.level(),p.damageSources().playerAttack(p),1);}
 if(t>=155&&t<255)PHASES.add(((CitizenState)v).citizenPhase());
 if(t==250){RESULT.addProperty("full_reaction",PHASES.containsAll(List.of(0,1,2,3)));v.setNoAi(false);v.getNavigation().moveTo(center.x-3,center.y,center.z+5,.7);}
 if(t==285){RESULT.addProperty("native_navigation",v.position().distanceToSqr(center.add(-3,0,0))>1);v.hurtServer(p.level(),p.damageSources().generic(),100);}
 if(t==291)RESULT.addProperty("native_death",!v.isAlive()&&v.deathTime>0);
 if(t==380){RESULT.addProperty("native_removal",p.level().getEntity(IDS.get(0))==null);for(UUID id:IDS){var e=p.level().getEntity(id);if(e!=null)e.discard();}p.closeContainer();for(int i=0;i<INVENTORY.size();i++)p.getInventory().setItem(i,INVENTORY.get(i));}
 });if(t>=390)finish(c,"complete");}
 private static void look(Vec3 at,Vec3 target){Vec3 d=target.subtract(at);view=new InputScene.CameraView(at,(float)Math.toDegrees(Math.atan2(-d.x,d.z)),(float)Math.toDegrees(Math.atan2(-d.y,d.horizontalDistance())));}
 private static void capture(Minecraft c,String name){Screenshot.grab(c.gameDirectory,name,c.gameRenderer.mainRenderTarget(),1,m->{});}
 private static void finish(Minecraft c,String reason){running=false;view=null;RESULT.addProperty("reason",reason);RESULT.addProperty("rendered",JakCitizenRenderer.rendered);RESULT.addProperty("sleep_frames",JakCitizenRenderer.sleepers);RESULT.addProperty("sleep_height",JakCitizenRenderer.largestSleepingThickness);RESULT.add("clips",new Gson().toJsonTree(JakCitizenRenderer.SEEN));boolean passed=RESULT.entrySet().stream().filter(e->e.getValue().isJsonPrimitive()&&e.getValue().getAsJsonPrimitive().isBoolean()).allMatch(e->e.getValue().getAsBoolean())&&JakCitizenRenderer.rendered>0&&JakCitizenRenderer.sleepers>0;RESULT.addProperty("passed",passed);if(!passed)RESULT.addProperty("reason","citizen assertion failed");try{Files.writeString(output.resolve("result.json"),RESULT.toString());}catch(Exception e){throw new RuntimeException(e);}c.stop();}
}
