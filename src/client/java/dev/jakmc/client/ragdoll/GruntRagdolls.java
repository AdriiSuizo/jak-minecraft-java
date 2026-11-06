package dev.jakmc.client.ragdoll;

import dev.jakmc.client.model.GltfModel;
import dev.jakmc.entity.JakEnemy;
import dev.jakmc.entity.JakEnemyKind;
import dev.jakmc.player.JakEntityClock;
import dev.jakmc.ragdoll.GruntFallDeath;
import java.util.*;
import net.minecraft.client.Minecraft;
import net.minecraft.core.particles.ParticleTypes;
import net.minecraft.world.damagesource.DamageSource;
import net.minecraft.world.level.Level;
import net.minecraft.world.phys.Vec3;
import org.joml.Matrix4f;
import org.joml.Vector3f;

/** Dying grunts' articulated falls (render_anim jak_enemies.rs `e.ragdoll` while State::Dead), client side. */
public final class GruntRagdolls {
 private GruntRagdolls(){}
 /** Returned by {@link #pose} once the settled corpse is gone: draw nothing. */
 public static final GltfModel.Posed HIDDEN=new GltfModel.Posed(null,null,null,null,null,null);
 private static final class Corpse {final JakEnemy entity;final GruntFall fall;double clock;double landed=Double.NaN;boolean gone;
  Corpse(JakEnemy entity,GruntFall fall,double clock){this.entity=entity;this.fall=fall;this.clock=clock;}}
 private static final Map<Integer,Corpse> ACTIVE=new HashMap<>();
 static{GruntFallDeath.puffed=e->{if(!e.level().isClientSide())return false;var c=ACTIVE.get(e.getId());return c!=null&&c.entity==e&&c.gone;};}
 /** The dying grunt's physical pose in its renderer's model frame (feet at x,y,z, turned by -yaw, scaled), HIDDEN once gone, or null for its ordinary death. */
 public static GltfModel.Posed pose(JakEnemy e,GltfModel model,GltfModel.Posed drawn,double x,double y,double z,float yaw){
  Corpse c=ACTIVE.get(e.getId());
  if(c!=null&&c.entity!=e){ACTIVE.remove(e.getId());c=null;}
  if(!e.isDeadOrDying()||!GruntFallDeath.falls(e)||drawn==null){if(c!=null)ACTIVE.remove(e.getId());return null;}
  double now=seconds(e.level());
  if(c==null){
   // jak_enemies.rs hit handler, State::Dead (Jak 3): e.vel = away*3 + Z*4 game metres a second, from the blow's origin.
   Vec3 at=new Vec3(x,y,z);DamageSource source=e.getLastDamageSource();
   Vec3 from=source==null?null:source.getEntity()!=null?source.getEntity().position():source.getSourcePosition();
   double ax=from==null?0:at.x-from.x,az=from==null?0:at.z-from.z,length=Math.sqrt(ax*ax+az*az);
   if(!(length>1e-6)||!Double.isFinite(length)){ax=1;az=0;}else{ax/=length;az/=length;}
   Vector3f velocity=new Vector3f((float)ax*3,4,(float)az*3);
   // jak_enemies.rs: Fall::new(rig,&pose,frame,e.pos,e.vel) from the pose drawn when struck.
   c=new Corpse(e,new GruntFall(model,drawn,frame(0,0,0,yaw),new Vector3f(),velocity,e.level(),at),now);ACTIVE.put(e.getId(),c);
  }
  advance(c,now);
  if(c.gone)return HIDDEN;
  return model.withJointGlobals(drawn,c.fall.globals(frame(x-c.fall.origin.x,y-c.fall.origin.y,z-c.fall.origin.z,yaw)));
 }
 private static final class Knock {final JakEnemy entity;GruntFall fall;double clock,settled=Double.NaN;boolean front;int hurt;Vector3f offset=new Vector3f();GltfModel.Posed last;
  Knock(JakEnemy entity){this.entity=entity;}}
 private static final Map<Integer,Knock> KNOCKS=new HashMap<>();
 /**
  * jak_enemies.rs State::Knocked for the grunt: thrown as a Fall (away x6 + up x5 game metres a second, from the
  * blow's origin; another blow in the air pushes the same Fall), drawn as it falls; once settled, grunt-getup-front
  * or -back by how it lies, blended from the Fall over .25 s. The pose in the renderer's model frame, or null when
  * not knocked down. at is the drawn feet (x,y,z), moved to the settled Fall's: stand-in, the server body lands on
  * its own, so the getting-up eases from where the Fall lay to the server's spot.
  */
 public static GltfModel.Posed knocked(JakEnemy e,JakEnemyKind kind,GltfModel model,GltfModel.Posed drawn,double[] at,float yaw){
  Knock k=KNOCKS.get(e.getId());
  if(k!=null&&k.entity!=e){KNOCKS.remove(e.getId());k=null;}
  String clip=e.animation();
  boolean down=e.isAlive()&&e.state()==JakEnemy.State.HURT&&GruntFallDeath.falls(e)&&drawn!=null
   &&(clip.equals(JakEnemy.KNOCK_CLIP)||clip.startsWith("grunt-getup"));
  // Killed while down: its death's Fall starts from the pose drawn last.
  if(!down){if(k!=null){KNOCKS.remove(e.getId());if(e.isDeadOrDying())return k.last;}return null;}
  double now=seconds(e.level());
  boolean hit=k!=null&&e.hurtTime>k.hurt;
  if(k==null||hit&&!Double.isNaN(k.settled)){
   GltfModel.Posed from=k!=null&&k.last!=null?k.last:drawn;
   k=new Knock(e);KNOCKS.put(e.getId(),k);
   k.fall=new GruntFall(model,from,frame(0,0,0,yaw),new Vector3f(),blow(e,at),e.level(),new Vec3(at[0],at[1],at[2]));k.clock=now;
  }else if(hit)k.fall.impulse(blow(e,at));
  k.hurt=e.hurtTime;
  float dt=(float)Math.max(0,now-k.clock);k.clock=now;
  if(Double.isNaN(k.settled)&&k.fall.step(dt)){
   k.settled=now;k.front=k.fall.front();
   k.offset.set((float)(k.fall.origin.x+k.fall.at.x-at[0]),(float)(k.fall.origin.y+k.fall.at.y-at[1]),(float)(k.fall.origin.z+k.fall.at.z-at[2]));
  }
  if(Double.isNaN(k.settled))return k.last=model.withJointGlobals(drawn,k.fall.globals(frame(at[0]-k.fall.origin.x,at[1]-k.fall.origin.y,at[2]-k.fall.origin.z,yaw)));
  String getup=k.front?"grunt-getup-front":"grunt-getup-back";
  var art=kind.animations.get(getup);
  float t=(float)(now-k.settled),length=art==null?1:Math.max(.01f,art.duration());
  // The Fall's feet to the body's own spot while it gets up.
  float ease=1-Math.clamp(t/length,0,1);
  at[0]+=k.offset.x*ease;at[1]+=k.offset.y*ease;at[2]+=k.offset.z*ease;
  var animated=model.pose(getup,art==null?0:Math.min(Math.max(0,art.frames()-1),t*60*art.speed()));
  float blend=Math.clamp(t/.25f,0,1);
  if(blend>=1)return k.last=animated;
  Matrix4f[] physical=k.fall.globals(frame(at[0]-k.fall.origin.x,at[1]-k.fall.origin.y,at[2]-k.fall.origin.z,yaw));
  Matrix4f[] mixed=new Matrix4f[physical.length];
  for(int j=0;j<physical.length;j++){
   Matrix4f a=physical[j],b=model.jointAt(animated,j);
   mixed[j]=new Matrix4f().translationRotateScale(a.getTranslation(new Vector3f()).lerp(b.getTranslation(new Vector3f()),blend),
    a.getNormalizedRotation(new org.joml.Quaternionf()).slerp(b.getNormalizedRotation(new org.joml.Quaternionf()),blend),
    a.getScale(new Vector3f()).lerp(b.getScale(new Vector3f()),blend));
  }
  return k.last=model.withJointGlobals(animated,mixed);
 }
 // jak_enemies.rs hit handler, State::Knocked (reaction_throws): e.vel = away*6 + Z*5 game metres a second.
 private static Vector3f blow(JakEnemy e,double[] at){
  DamageSource source=e.getLastDamageSource();
  Vec3 from=source==null?null:source.getEntity()!=null?source.getEntity().position():source.getSourcePosition();
  double ax=from==null?0:at[0]-from.x,az=from==null?0:at[2]-from.z,length=Math.sqrt(ax*ax+az*az);
  if(!(length>1e-6)||!Double.isFinite(length)){ax=1;az=0;}else{ax/=length;az/=length;}
  return new Vector3f((float)ax*6,5,(float)az*6);
 }
 /** Keeps falls going while their bodies are off screen, and forgets removed corpses. */
 public static void tick(Minecraft client){
  KNOCKS.values().removeIf(k->k.entity.isRemoved()||client.level==null||k.entity.level()!=client.level);
  if(ACTIVE.isEmpty())return;
  for(var it=ACTIVE.values().iterator();it.hasNext();){
   Corpse c=it.next();
   if(c.entity.isRemoved()||client.level==null||c.entity.level()!=client.level){it.remove();continue;}
   advance(c,seconds(c.entity.level()));
  }
 }
 private static Matrix4f frame(double x,double y,double z,float yaw){return new Matrix4f().translation((float)x,(float)y,(float)z).rotateY((float)Math.toRadians(-yaw)).scale(JakEnemyKind.SCALE);}
 // The enemies' clock (Light Jak's slowed time included), as the renderer animates them.
 private static double seconds(Level level){return (JakEntityClock.time(level)+JakEntityClock.of(level).partial(Minecraft.getInstance().getDeltaTracker().getGameTimeDeltaPartialTick(false)))/20d;}
 // jak_enemies.rs enemy update: step the Fall until settled (reaction_landed_at), then retain: gone at landed+0.75 s.
 private static void advance(Corpse c,double now){
  float dt=(float)Math.max(0,now-c.clock);c.clock=now;
  if(c.gone)return;
  if(Double.isNaN(c.landed)){if(c.fall.step(dt))c.landed=now;return;}
  if(now>c.landed+.75){c.gone=true;puff(c);}
 }
 // jak_enemies.rs gone: minecraft_world::request_poof(e.pos, ...) at the fall's feet; Minecraft's own LivingEntity.makePoofParticles.
 private static void puff(Corpse c){
  var e=c.entity;var r=e.getRandom();double x=c.fall.origin.x+c.fall.at.x,y=c.fall.origin.y+c.fall.at.y,z=c.fall.origin.z+c.fall.at.z;
  for(int i=0;i<20;i++){
   double xa=r.nextGaussian()*.02,ya=r.nextGaussian()*.02,za=r.nextGaussian()*.02;
   e.level().addParticle(ParticleTypes.POOF,x+e.getBbWidth()*(2*r.nextDouble()-1)-xa*10,y+e.getBbHeight()*r.nextDouble()-ya*10,z+e.getBbWidth()*(2*r.nextDouble()-1)-za*10,xa,ya,za);
  }
 }
}
