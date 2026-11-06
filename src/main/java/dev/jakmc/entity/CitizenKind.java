package dev.jakmc.entity;
import com.google.gson.*;
import dev.jakmc.RuntimeConfig;
import java.util.*;
import java.nio.file.Path;
import net.minecraft.world.phys.Vec3;
/** Actual Jak II GLB clip clocks; native Villager owns its brain, trades and save lifecycle. */
public record CitizenKind(String model,String asset,Map<String,Float> clips,String game,Vec3 shotMuzzle) {
 public static final int HAVEN_VARIANTS=3, SPARGUS_MALE=3, SPARGUS_FEMALE=4, MONK=5, JAK1_FIRST=6, JAK1_PEOPLE=5;
 public static final List<CitizenKind> ALL=new ArrayList<>();
 public static void initialize(){if(!ALL.isEmpty())return;try(var in=CitizenKind.class.getResourceAsStream("/data/jakmc/citizens.json")){for(var row:JsonParser.parseReader(new java.io.InputStreamReader(in,java.nio.charset.StandardCharsets.UTF_8)).getAsJsonArray()){var j=row.getAsJsonObject();Map<String,Float> clips=new LinkedHashMap<>();j.getAsJsonObject("clips").entrySet().forEach(e->clips.put(e.getKey(),e.getValue().getAsFloat()));ALL.add(new CitizenKind(j.get("model").getAsString(),j.get("asset").getAsString(),Map.copyOf(clips),j.has("game")?j.get("game").getAsString():"jak2",j.has("shotMuzzle")?dev.jakmc.player.JakPlayers.vector(j.getAsJsonArray("shotMuzzle")):Vec3.ZERO));}}catch(java.io.IOException e){throw new java.io.UncheckedIOException(e);}}
 public static CitizenKind of(int kind){initialize();return ALL.get(Math.floorMod(kind,ALL.size()));}
 public Path root(){return game.equals("jak3")?RuntimeConfig.assets:game.equals("jak1")?RuntimeConfig.gameAssets("jak1"):RuntimeConfig.assets.getParent().getParent().getParent().getParent().resolve("jak2/data/decompiler_out/jak2");}
 public float duration(String suffix){Float value=clips.get(model+"-"+suffix);if(value==null)throw new IllegalArgumentException("Original citizen clip missing: "+model+"-"+suffix);return value;}
 public boolean wastelander(){return model.startsWith("wlander-");}
 public boolean monk(){return model.equals("monk");}
 /** Rust: jak2_villagers.rs STILL (Jak 1's Sandover-style people, standing at their spots). */
 public boolean jak1(){return game.equals("jak1");}
 public String rise(){return switch(model){
  case "wlander-male"->"getup-back";
  case "wlander-female"->"getup-forward";
  default->"get-up-from-back";
 };}
 public String walk(int seed){if(wastelander())return model.equals("wlander-male")&&(seed&1)==1?"walk1":"walk0";int i=Math.floorMod(seed,3);return model.equals("citizen-norm")?new String[]{"walk","walk2","shuffle"}[i]:new String[]{"walk","walk2","walk3"}[i];}
 public float stride(int seed){if(wastelander())return 3.2f;int i=Math.floorMod(seed,3);return model.equals("citizen-norm")?new float[]{2.133f,3.2f,1.067f}[i]:model.equals("citizen-fat")?4:new float[]{3.84f,2.133f,3.84f}[i];}
 /** GOAL segment selection, retained from the source oracle (no overlapping outfits). */
 public long segments(int flavor){var r=new RandomBits(flavor);long v=0;switch(model){
  case "monk"->{v=new long[]{4171,8357,0x4113,0x8225,0x10407,0x20825}[Math.floorMod(flavor,6)];}
  case "wlander-male"->{
   v=r.one(3,16,256,0x200000);
   long head=r.one(3,4,4096,0x40000);
   v|=head;
   if(head==4)v|=r.one(3,32);
   long chest=r.one(2,2,0x20000);
   v|=chest;
   if(chest==2)v|=r.one(4,2048,0x10000);
   v|=r.one(3,8,128,0x100000)|r.one(3,512,0x4000,0x400000);
   v|=r.one(5,1024,0x8000,0x1000000)|r.one(6,0x800000)|r.one(6,64,0x80000)|r.one(2,0x2000000,0x4000000);
  }
  case "wlander-female"->{v=288|r.one(2,2,512)|r.one(2,4,1024)|r.one(2,8,2048)|r.one(2,16,4096)
   |r.one(2,64,8192)|r.one(2,128,0x4000)|r.one(2,0x8000,0x10000);}
  case "citizen-norm"->{v=32|16|r.one(4,64,2048,0x10000,0x200000)|r.one(3,2,8192,0x40000)|r.one(4,8,1024,0x8000,0x100000)|r.one(5,4,512,0x4000,0x80000)|r.one(4,128,4096,0x20000,0x400000)|r.one(4,256,0x800000);}
  case "citizen-fat"->{if(r.one(2,0,1)==0)v=8|r.one(3,4,256,0x8000)|r.one(4,2,128,130)|512|r.one(3,64,4096,8192)|r.one(3,32,2048,0x20000)|r.one(3,16,1024,0x10000);else v=0x400000|0x200000|r.one(2,4096,0x80000)|r.one(2,256,0x100000)|r.one(4,2,128,130)|0x20000|r.one(3,16,1024,0x10000);}
  case "citizen-chick"->{v=16|8|r.one(3,32,0x4000,0x8000)|r.one(3,64,1024,0x10000)|r.one(3,4,512,4096);long choice=r.one(3,0,1,2);v|=choice==1?256:choice==0?2|r.one(2,128,8192):2048|r.one(2,128,8192);}
 }return v;}
 private static class RandomBits{long state;RandomBits(int seed){state=Integer.toUnsignedLong(seed)*0x9E3779B97F4A7C15L^0x2545F4914F6CDD1DL;}long one(int n,long... values){state+=0x9E3779B97F4A7C15L;long z=state;z=(z^(z>>>30))*0xBF58476D1CE4E5B9L;z=(z^(z>>>27))*0x94D049BB133111EBL;int i=(int)Long.remainderUnsigned(z^(z>>>31),n);return i<values.length?values[i]:0;}}
}
