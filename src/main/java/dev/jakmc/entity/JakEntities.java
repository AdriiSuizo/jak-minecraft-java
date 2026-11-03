package dev.jakmc.entity;

import dev.jakmc.*;
import dev.jakmc.content.JakItems;
import java.util.*;
import net.fabricmc.fabric.api.object.builder.v1.entity.FabricDefaultAttributeRegistry;
import net.fabricmc.fabric.api.biome.v1.BiomeModifications;
import net.fabricmc.fabric.api.creativetab.v1.FabricCreativeModeTab;
import net.minecraft.core.*;
import net.minecraft.core.registries.*;
import net.minecraft.resources.ResourceKey;
import net.minecraft.network.chat.Component;
import net.minecraft.world.entity.*;
import net.minecraft.world.entity.monster.Monster;
import net.minecraft.world.entity.ai.attributes.Attributes;
import net.minecraft.world.item.*;
import net.minecraft.world.level.levelgen.Heightmap;

public final class JakEntities {
 public static final Map<EntityType<?>,JakEnemyKind> KINDS=new IdentityHashMap<>();
 public static final Map<String,EntityType<JakEnemy>> TYPES=new LinkedHashMap<>();
 public static final List<Item> EGGS=new ArrayList<>();
 public static void initialize(){
  JakEnemyKind.initialize();JakEnemySounds.initialize();
  for(var k:JakEnemyKind.ALL.values()){
   var key=ResourceKey.create(Registries.ENTITY_TYPE,JakData.id(k.model.replace('-','_')));
   var type=EntityType.Builder.<JakEnemy>of(JakEnemy::new,k.guard()?MobCategory.CREATURE:MobCategory.MONSTER).sized(k.half*2*JakEnemyKind.SCALE,k.height*JakEnemyKind.SCALE).eyeHeight(k.height*JakEnemyKind.SCALE*.85f).clientTrackingRange(8).updateInterval(1).build(key);
   Registry.register(BuiltInRegistries.ENTITY_TYPE,key,type);KINDS.put(type,k);TYPES.put(k.model,type);
   // One original Jak damage point is the same 2.5 Minecraft health used by
   // the Morph Gun/melee bridge; no former clone health multiplication.
   FabricDefaultAttributeRegistry.register(type,Monster.createMonsterAttributes().add(Attributes.MAX_HEALTH,k.hp*2.5).add(Attributes.ATTACK_DAMAGE,k.damage).add(Attributes.MOVEMENT_SPEED,k.guard()?KrimsonGuard.navigationSpeed(k.run):k.run*JakEnemyKind.SCALE/20).add(Attributes.FOLLOW_RANGE,k.stop*JakEnemyKind.SCALE).add(Attributes.STEP_HEIGHT,.6));
   if(!k.guard()){
   SpawnPlacements.register(type,SpawnPlacementTypes.ON_GROUND,Heightmap.Types.MOTION_BLOCKING_NO_LEAVES,Monster::checkMonsterSpawnRules);
   BiomeModifications.addSpawn(context->habitat(k.model,context.getBiomeKey().identifier().getPath()),MobCategory.MONSTER,type,6,k.model.equals("predator")?1:2,3);
   }
   var eggKey=ResourceKey.create(Registries.ITEM,JakData.id(k.model.replace('-','_')+"_spawn_egg"));
   EGGS.add(Registry.register(BuiltInRegistries.ITEM,eggKey,new SpawnEggItem(new Item.Properties().setId(eggKey).spawnEgg(type))));
  }
  Registry.register(BuiltInRegistries.CREATIVE_MODE_TAB,JakData.id("creatures"),FabricCreativeModeTab.builder().title(Component.translatable("tab.jakmc.creatures")).icon(()->new ItemStack(EGGS.get(0))).displayItems((context,out)->{EGGS.forEach(out::accept);JakCitizens.EGGS.forEach(out::accept);out.accept(JakLeaper.EGG);out.accept(JakFlutFlut.EGG);out.accept(JakFlutEgg.ITEM);}).build());
 }
 private static boolean habitat(String model,String biome){
  boolean ocean=biome.contains("ocean")||biome.contains("river"),snow=biome.contains("snow")||biome.contains("frozen")||biome.equals("grove")||biome.equals("jagged_peaks"),beach=biome.endsWith("beach")||biome.equals("stony_shore"),swamp=biome.contains("swamp"),jungle=biome.contains("jungle"),badlands=biome.contains("badlands"),cave=biome.endsWith("caves")||biome.equals("deep_dark"),nether=Set.of("nether_wastes","basalt_deltas","soul_sand_valley","crimson_forest","warped_forest").contains(biome);
  if(ocean||biome.contains("end")&&!biome.equals("badlands")||biome.equals("the_void"))return false;
  return switch(model){
   case "lurkerpuppy","lurkercrab","double-lurker"->beach;
   case "bonelurker","swamp-rat"->swamp;
   case "yeti","snow-bunny"->snow;
   case "hopper"->jungle;
   case "baby-spider","kg-grunt","spydroid"->cave;
   case "grunt","gekko"->cave||badlands;
   case "flitter","mantis"->nether;
   case "predator"->!nether&&!cave&&!snow&&!ocean&&!beach&&!swamp;
   default->!nether&&!badlands&&!cave&&!snow&&!swamp&&!ocean;
  };
 }
}
