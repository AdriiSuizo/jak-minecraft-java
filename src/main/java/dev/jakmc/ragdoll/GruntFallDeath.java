package dev.jakmc.ragdoll;

import dev.jakmc.entity.JakEnemy;
import java.util.*;
import java.util.function.Predicate;

/** Which dying enemies fall articulated (render_anim jak_enemies.rs), the same answer on server and client. */
public final class GruntFallDeath {
 private GruntFallDeath(){}
 private static final Set<JakEnemy> CANCELLED=Collections.synchronizedSet(Collections.newSetFromMap(new WeakHashMap<>()));
 /** Client: this corpse's fall settled and already puffed, so its removal must not puff again. */
 public static volatile Predicate<JakEnemy> puffed=e->false;
 // jak_enemies.rs hit handler: a Fall only for `e.kind.model=="grunt" && !e.wet`; enemy update: `if wet.is_some() {e.ragdoll=None;}` for good.
 public static boolean falls(JakEnemy e){
  if(e.kind==null||!e.kind.model.equals("grunt"))return false;
  if(e.isInWater())CANCELLED.add(e);
  return !CANCELLED.contains(e);
 }
 // jak_enemies.rs enemies retain: `gone = Dead && if ragdoll {landed+0.75 s || e.t>10.0} else {e.t>lasts}`.
 // stand-in: the server cannot see the client's settle, so it keeps the corpse to the 10 s cap; the client hides it and puffs at landed+0.75 s.
 public static int lastsTicks(JakEnemy e,int normal){return falls(e)?200:normal;}
}
