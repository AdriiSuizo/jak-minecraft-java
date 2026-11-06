package dev.jakmc.entity;
import java.util.*;
import net.minecraft.sounds.SoundEvent;
/** Authored art-group cues of a citizen's clips, for dev.jakmc.citizens (package-private readers live here). */
public final class CitizenCues {
 public record Cue(float seconds,String name){}
 private static final Map<String,Map<String,JakEnemyKind.Animation>> ART=new HashMap<>();
 // Rust: assets/jak_entities.rs citizen_events (event seconds in a clip, as SpargusDefense times them)
 public static synchronized List<Cue> of(CitizenKind kind,String clip){
  var a=ART.computeIfAbsent(kind.model(),m->JakEnemyKind.readAnimations(kind.root(),m,kind.jak1())).get(clip);
  if(a==null)return List.of();
  var out=new ArrayList<Cue>();
  for(var c:a.cues())out.add(new Cue((c.frame()-a.artistBase())/(60*a.speed()*a.artistStep()),c.name()));
  return out;
 }
 public static SoundEvent sound(String game,String cue){return JakEnemySounds.find(game,cue);}
}
