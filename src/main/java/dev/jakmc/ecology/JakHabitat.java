package dev.jakmc.ecology;

/** Where Jak's creatures live in the block world, and the bands their nests send. */
public enum JakHabitat {
 FIELDS,BEACH,SWAMP,SNOW,JUNGLE,DESERT,BADLANDS,VOLCANO,CAVE,WATER,BARREN;
 private static final String[][] NONE={};
 // Rust: render_anim/src/jak_enemies.rs habitat_of
 public static JakHabitat of(String biome){
  String name=biome.startsWith("minecraft:")?biome.substring(10):biome;
  return switch(name){
   case "beach","stony_shore","snowy_beach"->BEACH;
   case "swamp","mangrove_swamp"->SWAMP;
   case "jungle","sparse_jungle","bamboo_jungle"->JUNGLE;
   case "snowy_plains","ice_spikes","snowy_taiga","snowy_slopes","frozen_peaks","jagged_peaks","grove"->SNOW;
   case "desert"->DESERT;
   case "badlands","eroded_badlands","wooded_badlands"->BADLANDS;
   case "nether_wastes","basalt_deltas","soul_sand_valley","crimson_forest","warped_forest"->VOLCANO;
   case "dripstone_caves","lush_caves","deep_dark"->CAVE;
   case "the_end","end_highlands","end_midlands","end_barrens","small_end_islands","the_void"->BARREN;
   default->name.contains("ocean")||name.contains("river")?WATER:FIELDS;
  };
 }
 // Rust: render_anim/src/jak_enemies.rs bands (KINDS indices written as their models)
 public String[][] bands(){
  return switch(this){
   case FIELDS->new String[][]{{"babak","babak"},{"babak","babak","babak"}};
   case BEACH->new String[][]{{"babak","babak","lurkerpuppy"},{"lurkercrab","lurkercrab"},{"lurkerpuppy","lurkerpuppy","babak"},{"double-lurker","babak"}};
   // stand-in: the swamp frogs (Jak 1 Boggy Swamp's kermit) are not in the Rust's bands; a pair of their own.
   case SWAMP->new String[][]{{"swamp-rat","swamp-rat","swamp-rat"},{"bonelurker","babak"},{"bonelurker","bonelurker"},{"kermit","kermit"}};
   case SNOW->new String[][]{{"yeti","snow-bunny","snow-bunny"},{"snow-bunny","snow-bunny","snow-bunny"}};
   case JUNGLE->new String[][]{{"hopper","hopper"},{"babak","babak"}};
   case DESERT->new String[][]{{"babak","babak"}};
   case BADLANDS->new String[][]{{"grunt","grunt","grunt"},{"gekko","gekko"}};
   case VOLCANO->new String[][]{{"flitter","flitter","flitter"},{"mantis","mantis"}};
   case CAVE->new String[][]{{"baby-spider","baby-spider","babak"},{"grunt","gekko","gekko"},{"kg-grunt","spydroid","spydroid"}};
   case WATER,BARREN->NONE;
  };
 }
 // Rust: render_anim/src/jak_enemies.rs band_of
 public String[] bandOf(long pick){var all=bands();return all.length==0?new String[0]:all[(int)Long.remainderUnsigned(pick,all.length)];}
}
