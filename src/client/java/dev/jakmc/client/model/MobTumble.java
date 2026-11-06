package dev.jakmc.client.model;

import java.util.*;
import net.minecraft.client.Minecraft;
import net.minecraft.world.entity.Entity;
import org.joml.Quaternionf;
import org.joml.Vector3f;

/**
 * Rust minecraft_terrain tumble.rs and client_mobs.rs tumble_hit: a creature the Mass Inverter left
 * without gravity takes a blow and goes spinning (an angular velocity kept up while it floats, slowly
 * lost as it falls); where it comes down it stays as it landed (upright, on its back or its side),
 * settles flat, lies a while and gets back up slowly. It turns about its middle, in world axes.
 */
public final class MobTumble {
 // tumble.rs SETTLE, LIE, RECOVER (ticks), SPIN (rad a tick at full strength), KEEP_FLOATING, KEEP_FALLING
 private static final int SETTLE=5,LIE=80,RECOVER=18;
 private static final float SPIN=.35f,KEEP_FLOATING=.995f,KEEP_FALLING=.985f;
 private enum Phase{SPINNING,SETTLING,LYING,RECOVERING}
 private static final Map<Integer,MobTumble> TUMBLES=new HashMap<>();
 private static final Map<Integer,Integer> LIFTED=new HashMap<>();

 private final Quaternionf turn=new Quaternionf(),turnO=new Quaternionf(),from=new Quaternionf(),to=new Quaternionf();
 private final Vector3f spin=new Vector3f();
 private float pivot,pivotO,pivotFrom,pivotTo;private int t;private Phase phase=Phase.SPINNING;
 private final float width,height;
 private MobTumble(float width,float height){this.width=width;this.height=height;pivot=pivotO=height*.5f;}

 /** The server's word: `lift` ticks of float left, and a blow along `dir` of `strength` (0: none). */
 public static void receive(int id,int lift,float dx,float dy,float dz,float strength){
  var level=Minecraft.getInstance().level;if(level==null)return;
  if(lift>0)LIFTED.put(id,lift);
  if(strength<=0)return;
  var e=level.getEntity(id);if(e==null)return;
  // client_mobs.rs tumble_hit: only floating, or already tumbling.
  boolean floating=LIFTED.getOrDefault(id,0)>0;
  if(!floating&&!TUMBLES.containsKey(id))return;
  // A twist of its own, from its id and age.
  float twist=Long.remainderUnsigned((Integer.toUnsignedLong(id)*2_654_435_761L)^e.tickCount,200)/100f-1;
  TUMBLES.computeIfAbsent(id,k->new MobTumble(e.getBbWidth(),e.getBbHeight())).hit(new Vector3f(dx,dy,dz),strength,twist);
 }
 // tumble.rs hit
 private void hit(Vector3f dir,float strength,float twist){
  var flat=new Vector3f(dir.x,0,dir.z);
  var axis=flat.lengthSquared()>1e-6f?new Vector3f(0,1,0).cross(flat.normalize()):new Vector3f(1,0,0);
  spin.add(axis.mul(SPIN*Math.clamp(strength,0,1.5f))).add(0,SPIN*.6f*Math.clamp(twist,-1,1),0);
  phase=Phase.SPINNING;
 }
 /** Once a client tick. */
 public static void tick(){
  var level=Minecraft.getInstance().level;
  if(level==null){TUMBLES.clear();LIFTED.clear();return;}
  LIFTED.replaceAll((id,left)->left-1);LIFTED.values().removeIf(left->left<=0);
  for(var it=TUMBLES.entrySet().iterator();it.hasNext();){
   var en=it.next();Entity e=level.getEntity(en.getKey());
   if(e==null||e.isRemoved()){it.remove();continue;}
   if(!en.getValue().step(!e.onGround(),LIFTED.containsKey(en.getKey())))it.remove();
  }
 }
 // tumble.rs tick
 private boolean step(boolean airborne,boolean floating){
  turnO.set(turn);pivotO=pivot;float half=height*.5f;
  switch(phase){
   case SPINNING->{
    float angle=spin.length();
    if(angle>1e-9f)turn.premul(new Quaternionf().rotationAxis(angle,spin.x/angle,spin.y/angle,spin.z/angle)).normalize();
    spin.mul(floating?KEEP_FLOATING:KEEP_FALLING);
    pivot=half;
    if(!airborne){
     boolean side=restOf(turn,to);
     from.set(turn);pivotFrom=pivot;pivotTo=side?width*.5f:half;
     spin.zero();t=0;phase=Phase.SETTLING;
    }
   }
   case SETTLING->{
    t++;float k=Math.min(1,t/(float)SETTLE);
    from.slerp(to,k,turn);pivot=pivotFrom+(pivotTo-pivotFrom)*k;
    if(t>=SETTLE){t=0;phase=Phase.LYING;}
   }
   case LYING->{if(++t>=LIE){from.set(turn);pivotFrom=pivot;t=0;phase=Phase.RECOVERING;}}
   case RECOVERING->{
    t++;float k=Math.min(1,t/(float)RECOVER);k=k*k*(3-2*k);
    from.slerp(new Quaternionf(),k,turn);pivot=pivotFrom+(half-pivotFrom)*k;
    if(t>=RECOVER)return false;
   }
  }
  return true;
 }
 // tumble.rs rest_of: its up laid flat on the ground (on its side) or straight up or down, whichever nearest. True on its side.
 private static boolean restOf(Quaternionf turn,Quaternionf out){
  var up=turn.transform(new Vector3f(0,1,0));Vector3f target;boolean side=false;
  if(up.y>.6f)target=new Vector3f(0,1,0);
  else if(up.y<-.6f)target=new Vector3f(0,-1,0);
  else{var flat=new Vector3f(up.x,0,up.z);target=flat.lengthSquared()>1e-9f?flat.normalize():new Vector3f(1,0,0);side=true;}
  new Quaternionf().rotationTo(up,target).mul(turn,out).normalize();
  return side;
 }
 /** tumble.rs place: the feet's offset and the turn at `partial`, or null for a creature not tumbling. */
 public static Object[] place(int id,float partial){
  var tumble=TUMBLES.get(id);if(tumble==null)return null;
  float p=Math.clamp(partial,0,1);
  var turn=new Quaternionf(tumble.turnO).slerp(tumble.turn,p);
  float pivot=tumble.pivotO+(tumble.pivot-tumble.pivotO)*p;
  var offset=new Vector3f(0,pivot,0).sub(turn.transform(new Vector3f(0,tumble.height*.5f,0)));
  return new Object[]{offset,turn};
 }
}
