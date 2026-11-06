package dev.jakmc.client.model;

import dev.jakmc.entity.JakEnemy;
import dev.jakmc.entity.JakEnemyKind;
import net.minecraft.world.phys.Vec3;
import org.joml.Matrix3f;
import org.joml.Quaternionf;
import org.joml.Vector3f;

/**
 * kermit.gc joint-mod-tracker-callback on the frog's tongueA (up axis y, forward z): tracking, the
 * joint points at its prey, its forward scaled to the distance over 12 m times the control, no more
 * than the max; not tracking, its forward scale is the control (0: the tongue drawn in). The
 * tongue's other joints follow it as its children.
 */
final class KermitTongue {
 private static final String JOINT="tongueA";
 /** joint-mod-tracker inv-forward-scale-factor: 1/49152 (12 m). */
 private static final float TONGUE=12;
 private KermitTongue(){}
 static GltfModel.Posed pose(GltfModel model,GltfModel.Posed posed,JakEnemy frog,float partial,float yaw){
  var parented=model.parented(posed,JOINT);if(parented==null)return posed;
  float control=frog.tongueStretch();
  var prey=frog.tongueTarget()<0?null:frog.level().getEntity(frog.tongueTarget());
  if(prey==null){
   var scale=parented[0].getScale(new Vector3f());scale.z=control;
   return model.withLocal(posed,JOINT,null,scale);
  }
  // The prey in the model's own space (JakEnemyRenderer turns by -yaw and scales by SCALE).
  Vec3 at=prey.getPosition(partial).add(0,prey.getBbHeight()*.5,0).subtract(frog.getPosition(partial));
  var target=new Vector3f((float)at.x,(float)at.y,(float)at.z).rotateY((float)Math.toRadians(yaw)).div(JakEnemyKind.SCALE);
  var parent=parented[1];
  var base=parent.transformPosition(parented[0].getTranslation(new Vector3f()));
  var forward=target.sub(base,new Vector3f());float length=forward.length();
  if(length<1e-4f)return posed;
  forward.div(length);
  // build-matrix-from-up-and-forward-axes!
  var side=new Vector3f(0,1,0).cross(forward);if(side.lengthSquared()<1e-8f)side.set(1,0,0);side.normalize();
  var up=forward.cross(side,new Vector3f());
  var world=new Quaternionf().setFromNormalized(new Matrix3f(side,up,forward));
  var parentTurn=parent.getUnnormalizedRotation(new Quaternionf()).normalize();
  var local=parentTurn.conjugate().mul(world).normalize();
  float scale=Math.min(length/TONGUE*control,frog.tongueMax());
  return model.withLocal(posed,JOINT,local,new Vector3f(1,1,scale));
 }
 /** kermit-tongue-pos in the world: tongueA's place in `posed`. */
 static Vec3 base(GltfModel model,GltfModel.Posed posed,Vec3 feet,float yaw){
  var parented=model.parented(posed,JOINT);if(parented==null)return null;
  var p=parented[1].transformPosition(parented[0].getTranslation(new Vector3f())).mul(JakEnemyKind.SCALE).rotateY((float)-Math.toRadians(yaw));
  return feet.add(p.x,p.y,p.z);
 }
}
