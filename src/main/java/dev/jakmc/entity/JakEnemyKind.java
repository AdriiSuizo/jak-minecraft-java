package dev.jakmc.entity;

import com.google.gson.*;
import dev.jakmc.*;
import dev.jakmc.nativebridge.NativeJak;
import java.io.*;
import java.nio.file.Path;
import java.util.*;

/** Guest definitions, independent of Minecraft entity state. Read from the owned art groups. */
public final class JakEnemyKind {
 public static final Map<String,JakEnemyKind> ALL=new LinkedHashMap<>();
 // Measured combined standing Jak+Daxter height is 2.5138981 GLB metres.
 // Jak has a 1.4 prejoint; creatures have a unit prejoint. Same scale as JakRenderer.
 public static final float SCALE=1.94f/(2.5138981f*1.4f);
 public record Cue(float frame,String name,float volume,float pitch){}
 public record Animation(int frames,float speed,float artistBase,float artistStep,List<Cue> cues){
  public float duration(){return speed>0?(frames-1)/(60*speed):0;}
  public float artist(float seconds){return artistBase+seconds*60*speed*artistStep;}
 }
 public final String model,game,asset;
 public final String[] clips,jump;
 public final float run,walk,sight,stop,reach,hp,damage,half,height,jumpMin,jumpFactor;
 public final boolean metalHead,leaps;
 public final Map<String,Animation> animations=new HashMap<>();
 private JakEnemyKind(JsonObject d){
  model=d.get("model").getAsString();game=d.get("game").getAsString();asset=d.get("asset").getAsString();
  clips=strings(d.getAsJsonArray("clips"));jump=strings(d.getAsJsonArray("jumpClips"));
  run=f(d,"run");walk=f(d,"walk");sight=f(d,"sight");stop=f(d,"stop");reach=f(d,"reach");hp=f(d,"hp");damage=f(d,"damage");half=f(d,"half");height=f(d,"height");jumpMin=f(d,"jumpMin");jumpFactor=f(d,"jumpFactor");metalHead=d.get("metalHead").getAsBoolean();leaps=d.get("leaps").getAsBoolean();
  animations.putAll(readAnimations(root(),model,jak1()));
  // These are actual one-key static clips in the GLB; zero-speed .go entries
  // are intentionally absent from the moving-animation reader.
  if(model.equals("hopper"))animations.putIfAbsent("hopper-idle",new Animation(1,0,0,1,List.of()));
  if(model.equals("babak"))animations.putIfAbsent("babak-win",new Animation(1,0,0,1,List.of()));
  for(String clip:clips)if(!clip.isEmpty()&&!animations.containsKey(clip))throw new IllegalStateException("Missing original enemy clip "+clip);
 }
 public Path root(){return game.equals("jak3")?RuntimeConfig.assets:RuntimeConfig.assets.getParent().getParent().getParent().getParent().resolve(game+"/data/decompiler_out/"+game);}
 public float duration(String clip){var a=animations.get(clip);return a==null?0:a.duration();}
 public String first(String... names){for(String n:names)if(!n.isEmpty()&&animations.containsKey(n))return n;return clips[0];}
 public boolean guard(){return model.equals("crimson-guard");}
 public boolean jak1(){return game.equals("jak1");}
 public static void initialize(){
  if(!ALL.isEmpty())return;
  try(var in=JakEnemyKind.class.getResourceAsStream("/data/jakmc/enemies.json")){
   if(in==null)throw new IOException("Enemy definitions missing");
   for(var v:JsonParser.parseReader(new InputStreamReader(in,java.nio.charset.StandardCharsets.UTF_8)).getAsJsonArray()){var k=new JakEnemyKind(v.getAsJsonObject());ALL.put(k.model,k);}
  }catch(IOException e){throw new UncheckedIOException(e);}
  JakMod.LOG.info("Loaded {} original enemy definitions and {} authored animations",ALL.size(),ALL.values().stream().mapToInt(k->k.animations.size()).sum());
 }
 static Map<String,Animation> readAnimations(Path root,String model,boolean jak1){
  var animations=new HashMap<String,Animation>();
  var data=JsonParser.parseString(NativeJak.animationMetadata(root.toString(),model,jak1)).getAsJsonObject();
  for(var e:data.entrySet()){
   var a=e.getValue().getAsJsonObject();var cues=new ArrayList<Cue>();
   for(var v:a.getAsJsonArray("events")){var c=v.getAsJsonObject();cues.add(new Cue(f(c,"frame"),c.get("name").getAsString(),f(c,"volume"),f(c,"pitch")));}
   animations.put(e.getKey(),new Animation(a.get("frames").getAsInt(),f(a,"speed"),f(a,"artistBase"),f(a,"artistStep"),List.copyOf(cues)));
  }
  return animations;
 }
 private static float f(JsonObject d,String k){return d.get(k).getAsFloat();}
 private static String[] strings(JsonArray a){var s=new String[a.size()];for(int i=0;i<s.length;i++)s[i]=a.get(i).getAsString();return s;}
}
