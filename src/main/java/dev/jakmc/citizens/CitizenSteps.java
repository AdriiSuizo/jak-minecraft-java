package dev.jakmc.citizens;

import dev.jakmc.entity.CitizenCues;
import dev.jakmc.entity.CitizenKind;
import dev.jakmc.entity.CitizenState;
import dev.jakmc.entity.JakEnemyKind;
import dev.jakmc.player.JakEntityClock;
import java.util.*;
import net.minecraft.sounds.SoundSource;
import net.minecraft.world.entity.npc.villager.Villager;
import net.minecraft.world.level.Level;

/** Rust: jak2_villagers.rs npc_walk_effects and npc_pose_effects (the Wastelanders' authored steps, cane and landings), beside the drawn walk. */
public final class CitizenSteps {
 // Rust: jak2_villagers.rs npc_sound (the cues the original plays for its people)
 private static final Set<String> SOUNDS=Set.of("was-gun-cane","was-fs-hard","was-fs-soft","wasf-fs","was-land-hard","wasf-land-hard");
 private static final Map<Villager,float[]> LAST=new WeakHashMap<>();

 // Called on the client each tick, where the walk animation the renderer reads is kept.
 public static void tick(Level level,Villager v){
  var kind=CitizenKind.of(((CitizenState)v).citizenVariant());
  if(!kind.wastelander()||!v.isAlive())return;
  var state=(CitizenState)v;
  float walked=v.walkAnimation.position(),posed=(JakEntityClock.time(level)-state.citizenPhaseStart())/20f;
  var last=LAST.put(v,new float[]{walked,posed,state.citizenPhase()});
  if(last==null)return;
  int seed=v.getUUID().hashCode();
  if(state.citizenPhase()==2||state.citizenPhase()==3){
   // Rust: npc_pose_effects (State::Land and State::GetUp)
   if(last[2]!=state.citizenPhase())last[1]=0;
   String suffix=state.citizenPhase()==2?"knocked-back-land":kind.rise();
   for(var cue:CitizenCues.of(kind,kind.model()+"-"+suffix))
    if(cue.seconds()>=last[1]&&cue.seconds()<posed)play(level,v,cue.name());
   return;
  }
  if(state.citizenPhase()!=0||v.walkAnimation.speed()<=.01f)return;
  // Rust: npc_walk_effects (a cue each time the stride's phase passes it; the clip and stride as JakCitizenRenderer picks them)
  boolean running=((CitizenLifeState)v).citizenLifeState()==CitizenLife.PANIC||v.walkAnimation.speed()>.27f;
  String suffix=running?"run0":kind.walk(seed);float stride=running?6.4f:kind.stride(seed),duration=kind.duration(suffix);
  if(!(duration>0))return;
  double scale=4*(v.isBaby()?3:1)*stride*JakEnemyKind.SCALE,from=last[0]/scale,to=walked/scale;
  for(var cue:CitizenCues.of(kind,kind.model()+"-"+suffix)){
   double phase=cue.seconds()/duration,next=phase+Math.floor(from-phase)+1;
   if(next<=to)play(level,v,cue.name());
  }
 }
 private static void play(Level level,Villager v,String name){
  if(!SOUNDS.contains(name))return;
  var sound=CitizenCues.sound("jak3",name);
  if(sound!=null)level.playLocalSound(v.getX(),v.getY(),v.getZ(),sound,SoundSource.NEUTRAL,1,1,false);
 }
}
