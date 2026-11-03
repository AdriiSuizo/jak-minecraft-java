package dev.jakmc.entity;

import com.google.gson.*;
import dev.jakmc.JakData;
import net.minecraft.sounds.SoundEvent;
import java.io.*;
import java.util.*;

/** Resolves original art-group sound names only when the owned bank contains them. */
public final class JakEnemySounds {
 public static long cuesPlayed;
 private static final Set<String> AVAILABLE=new HashSet<>();
 public static void initialize(){
  try(var in=JakEnemySounds.class.getResourceAsStream("/assets/jakmc/sounds.json")){
   if(in==null)throw new IOException("Owned original SFX have not been imported");
   AVAILABLE.addAll(JsonParser.parseReader(new InputStreamReader(in,java.nio.charset.StandardCharsets.UTF_8)).getAsJsonObject().keySet());
  }catch(IOException e){throw new UncheckedIOException(e);}
 }
 public static SoundEvent find(JakEnemyKind kind,String cue){
  return find(kind.game,cue);
 }
 static SoundEvent find(String game,String cue){
  String key="sfx/"+game+"/"+cue.toLowerCase(Locale.ROOT).replace('-','_');
  return AVAILABLE.contains(key)?SoundEvent.createVariableRangeEvent(JakData.id(key)):null;
 }
 public static SoundEvent first(JakEnemyKind kind,String... names){for(String n:names){var s=find(kind,n);if(s!=null)return s;}return null;}
}
