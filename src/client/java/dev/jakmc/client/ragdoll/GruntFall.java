package dev.jakmc.client.ragdoll;

import dev.jakmc.client.model.GltfModel;
import dev.jakmc.entity.JakEnemyKind;
import java.util.*;
import net.minecraft.core.BlockPos;
import net.minecraft.world.level.Level;
import net.minecraft.world.level.block.state.BlockState;
import net.minecraft.world.phys.AABB;
import net.minecraft.world.phys.Vec3;
import net.minecraft.world.phys.shapes.VoxelShape;
import org.joml.Matrix4f;
import org.joml.Quaternionf;
import org.joml.Vector3f;

/** render_anim grunt_ragdoll.rs Fall: the original grunt skeleton's articulated impact. Joint membership, radii,
 * parent overrides and angular limits are Jak 3 grunt.gc ragdoll-setup; integration at 120 Hz against the blocks.
 * Map units are blocks here (sim voxel.rs BLOCK=36): a game metre M is 25/36 block, creature_body::k()*M is
 * JakEnemyKind.SCALE (a GLB metre), and map Z up is Minecraft Y. Positions are relative to {@link #origin}. */
final class GruntFall {
 static final float STEP=1f/120,UNIT=1f/36,M=25f*UNIT,K=JakEnemyKind.SCALE,TAU=(float)(Math.PI*2);
 private static final double BLOCK=36,SURFACE_CLIP_EPSILON=.125,INSIDE=1e-4;
 // grunt_ragdoll.rs SETUP: joint, parent override, radius (4096ths of a metre), limit (65536ths of a turn).
 private static final float[][] SETUP={
  {2,-1,2268.7744f,3682.7046f},{3,-1,2867.2f,2296.8184f},{4,-1,2209.3823f,2226.3489f},{5,-1,1644.544f,3210.7178f},
  {6,-1,1722.368f,2605.129f},{8,3,1641.6768f,3682.7046f},{9,-1,1295.1552f,3682.7046f},{10,-1,1899.3152f,2671.921f},
  {11,-1,1787.904f,2077.3455f},{12,3,1563.8528f,3682.7046f},{13,-1,1127.6288f,3682.7046f},{14,-1,1610.1376f,2330.5876f},
  {15,-1,1629.3888f,1879.882f},{16,2,1812.0704f,2508.973f},{17,-1,1858.3552f,3682.7046f},{18,-1,877.3632f,3698.87f},
  {19,-1,1524.9408f,3691.5154f},{20,-1,1127.6288f,3682.7046f},{21,-1,1127.6288f,3682.7046f},{22,-1,659.456f,1294.1176f},
  {23,21,605.3888f,1411.5544f},{24,17,1488.0768f,3682.7046f},{25,-1,1285.7344f,3682.7046f},{26,-1,1801.8304f,3682.7046f},
  {27,-1,794.2144f,2497.213f},{28,-1,556.2368f,1846.1127f},{29,-1,542.3104f,1491.563f},{30,17,1540.096f,3739.2292f},
  {31,-1,1127.6288f,3682.7046f},{32,-1,1751.4496f,3682.7046f},{33,-1,1127.6288f,3682.7046f},{34,-1,573.0304f,2015.6871f},
  {35,33,468.992f,1851.9927f},{36,3,1868.5952f,2863.5044f},{37,3,1893.9904f,2569.885f},{38,11,1327.9232f,3185.0312f},
  {39,-1,423.1168f,2031.8344f},{40,11,533.7088f,2505.2957f},{41,-1,408.3712f,2064.8572f},{42,9,1593.7535f,2784.2424f},
  {43,15,1373.3888f,2773.2288f},{44,-1,616.448f,1984.8488f},{45,15,1127.6288f,1981.1897f},{46,-1,534.528f,2276.2654f},
  {47,13,1853.0304f,2685.1372f},{48,21,525.9264f,1536.3458f},{49,33,382.5664f,1445.3237f}};
 // grunt_ragdoll.rs step escape order [Z, X, -X, Y, -Y] in map axes.
 private static final Vector3f[] ESCAPE={new Vector3f(0,1,0),new Vector3f(1,0,0),new Vector3f(-1,0,0),new Vector3f(0,0,-1),new Vector3f(0,0,1)};
 private static final class Node {int joint,parent;Vector3f p,previous,renderPrevious,restDir;float radius,length,angle;Quaternionf turn,previousTurn,initial;}
 private record Trace(float fraction,Vector3f normal,boolean inside){}
 private final List<Node> nodes=new ArrayList<>();
 private final Matrix4f[] base;
 private final int[] parents;
 private float clock,rest,age;
 private final Vector3f rootStart,hipStart;
 final Vector3f at=new Vector3f();
 final Level level;
 final Vec3 origin;
 private final BlockPos.MutableBlockPos cell=new BlockPos.MutableBlockPos();
 // grunt_ragdoll.rs Fall::new
 GruntFall(GltfModel model,GltfModel.Posed pose,Matrix4f frame,Vector3f feet,Vector3f velocity,Level level,Vec3 origin){
  this.level=level;this.origin=origin;
  base=new Matrix4f[model.jointCount()];parents=new int[base.length];
  for(int j=0;j<base.length;j++){base[j]=new Matrix4f(frame).mul(model.jointAt(pose,j));parents[j]=model.jointParent(j);}
  for(float[] s:SETUP){
   int joint=(int)s[0],override=(int)s[1];if(joint>=base.length)continue;
   int parentJoint=override>=0?override:parents[joint],parent=-1;
   if(parentJoint>=0)for(int i=0;i<nodes.size();i++)if(nodes.get(i).joint==parentJoint){parent=i;break;}
   Quaternionf rotation=base[joint].getNormalizedRotation(new Quaternionf());Vector3f p=base[joint].getTranslation(new Vector3f());
   Vector3f edge=parent<0?new Vector3f():new Vector3f(p).sub(nodes.get(parent).p);
   // Give the upper body the greater share of the blow, while the feet retain inertia.
   float weight=Math.clamp((p.y-feet.y)/(3*M),.15f,1f);
   Vector3f impulse=new Vector3f(velocity).mul((.35f+weight*.65f)*M);
   Node n=new Node();n.joint=joint;n.parent=parent;n.p=p;n.previous=new Vector3f(p).sub(new Vector3f(impulse).mul(STEP));n.renderPrevious=new Vector3f(p);
   n.radius=s[2]/4096f*K;n.length=edge.length();n.restDir=normalizeOrZero(edge);n.angle=s[3]*TAU/65536f;
   n.turn=new Quaternionf(rotation);n.previousTurn=new Quaternionf(rotation);n.initial=new Quaternionf(rotation);nodes.add(n);
  }
  hipStart=new Vector3f(nodes.get(0).p);rootStart=new Vector3f(feet);at.set(feet);
 }
 int joints(){return nodes.size();}
 // grunt_ragdoll.rs Fall::front: the hips' own Z axis points up (map Z, Minecraft Y): lying face down, up from the front.
 boolean front(){return new Quaternionf(nodes.get(0).turn).transform(new Vector3f(0,0,1)).y>0;}
 // grunt_ragdoll.rs Fall::impulse
 void impulse(Vector3f velocity){rest=0;for(Node n:nodes)n.previous.sub(new Vector3f(velocity).mul(M*STEP));}
 // grunt_ragdoll.rs Fall::step: true once settled; `at` is its feet (hip travel, lowest joint's floor).
 boolean step(float dt){
  clock+=Math.min(dt,.25f);
  while(clock>=STEP){
   clock-=STEP;age+=STEP;
   for(Node n:nodes){
    n.renderPrevious.set(n.p);n.previousTurn.set(n.turn);
    Vector3f motion=new Vector3f(n.p).sub(n.previous).mul(.995f);
    n.previous.set(n.p);
    n.p.add(motion).sub(0,60*K*STEP*STEP,0);
   }
   for(int iteration=0;iteration<10;iteration++){
    for(int i=1;i<nodes.size();i++){
     Node n=nodes.get(i);if(n.parent<0)continue;Node parent=nodes.get(n.parent);
     Vector3f delta=new Vector3f(n.p).sub(parent.p);float len=delta.length();
     if(len<.0001f*UNIT)continue;
     Vector3f correction=delta.mul((1-n.length/len)*.5f);
     n.p.sub(correction);parent.p.add(correction);
     Vector3f reference=new Quaternionf(parent.turn).mul(new Quaternionf(parent.initial).invert()).transform(new Vector3f(n.restDir));
     Vector3f actual=normalizeOrZero(new Vector3f(n.p).sub(parent.p));
     float angle=(float)Math.acos(Math.clamp(reference.dot(actual),-1f,1f));
     if(angle>n.angle&&angle>.0001f){
      Quaternionf turn=arc(reference,actual);
      Vector3f allowed=new Quaternionf().slerp(turn,n.angle/angle).transform(new Vector3f(reference));
      Vector3f limit=new Vector3f(actual).sub(allowed).mul(n.length*.5f);
      n.p.sub(limit);parent.p.add(limit);
     }
    }
    updateTurns();
   }
   int contacts=0;
   for(Node n:nodes){
    // Swept joint volumes prevent fast limbs crossing thin or inclined pieces; damping applies only at contact.
    Trace hit=sweep(n.previous,n.p,n.radius);
    if(hit.inside){
     // Spawned against a wall or floor: bounded local depenetration, never eject the whole actor upward.
     Vector3f start=new Vector3f(n.previous),escaped=null;
     escape:for(int step=1;step<=12;step++)for(Vector3f dir:ESCAPE){
      Vector3f to=new Vector3f(dir).mul(step*n.radius/6).add(start);
      if(!sweep(to,to,n.radius).inside){escaped=to;break escape;}
     }
     n.p.set(escaped!=null?escaped:start);n.previous.set(n.p);contacts++;
    }else if(hit.fraction<1){
     Vector3f motion=new Vector3f(n.p).sub(n.previous);
     n.p.set(n.previous).add(new Vector3f(motion).mul(hit.fraction)).add(new Vector3f(hit.normal).mul(.002f*UNIT));
     Vector3f slide=new Vector3f(motion).sub(new Vector3f(hit.normal).mul(motion.dot(hit.normal)));
     n.previous.set(n.p).sub(slide.mul(.7f));contacts++;
    }
   }
   float speed=0;for(Node n:nodes)speed+=new Vector3f(n.p).sub(n.previous).lengthSquared();speed/=nodes.size();
   float still=.35f*M*STEP;
   if(contacts>0&&speed<still*still)rest+=STEP;else rest=0;
   updateTurns();
  }
  Vector3f hip=nodes.get(0).p;float floor=Float.POSITIVE_INFINITY;
  for(Node n:nodes)floor=Math.min(floor,n.p.y-n.radius);
  at.set(rootStart).add(hip).sub(hipStart);at.y=floor;
  return age>.35f&&rest>.2f;
 }
 // grunt_ragdoll.rs Fall::update_turns
 private void updateTurns(){
  for(int i=0;i<nodes.size();i++){
   int child=-1;
   for(int j=i+1;j<nodes.size();j++)if(nodes.get(j).parent==i&&(child<0||nodes.get(j).length>=nodes.get(child).length))child=j;
   if(child<0)continue;
   Vector3f from=nodes.get(child).restDir,to=normalizeOrZero(new Vector3f(nodes.get(child).p).sub(nodes.get(i).p));
   if(from.lengthSquared()>.5f&&to.lengthSquared()>.5f)nodes.get(i).turn.set(arc(from,to)).mul(nodes.get(i).initial);
  }
 }
 // grunt_ragdoll.rs Fall::globals with blend 0 (a death keeps its physical pose): joint matrices in the model frame `frame`.
 Matrix4f[] globals(Matrix4f frame){
  Matrix4f[] world=new Matrix4f[base.length];boolean[] simulated=new boolean[base.length];
  for(int i=0;i<base.length;i++)world[i]=new Matrix4f(base[i]);
  float partial=Math.clamp(clock/STEP,0f,1f);
  for(Node n:nodes){
   simulated[n.joint]=true;
   world[n.joint]=new Matrix4f().translationRotateScale(new Vector3f(n.renderPrevious).lerp(n.p,partial),new Quaternionf(n.previousTurn).slerp(n.turn,partial),base[n.joint].getScale(new Vector3f()));
  }
  // End joints and equipment not simulated separately follow their actual skeletal parent, including the jaw tip and armour.
  for(int i=0;i<world.length;i++)if(!simulated[i]&&parents[i]>=0)world[i]=new Matrix4f(world[parents[i]]).mul(new Matrix4f(base[parents[i]]).invert()).mul(base[i]);
  Matrix4f inverse=new Matrix4f(frame).invert();
  for(int i=0;i<world.length;i++)world[i]=new Matrix4f(inverse).mul(world[i]);
  return world;
 }
 // sim voxel.rs sweep_box -> trace -> trace_blocks: a box of half extent r swept start->end, in block space.
 private Trace sweep(Vector3f start,Vector3f end,float r){
  double[] a={origin.x+start.x,origin.y+start.y,origin.z+start.z},b={origin.x+end.x,origin.y+end.y,origin.z+end.z};
  double[] delta={b[0]-a[0],b[1]-a[1],b[2]-a[2]};
  double length=Math.sqrt(delta[0]*delta[0]+delta[1]*delta[1]+delta[2]*delta[2]);
  // Long traces are tested in short segments, nearest first.
  int segments=(int)Math.max(1,Math.ceil(length/4));
  double bestT=1;double[] bestNormal=new double[3];
  for(int segment=0;segment<segments;segment++){
   double t0=(double)segment/segments,t1=(double)(segment+1)/segments;
   double[] sa=new double[3],sb=new double[3];for(int k=0;k<3;k++){sa[k]=a[k]+delta[k]*t0;sb[k]=a[k]+delta[k]*t1;}
   double[] first=new double[4];int hit=segmentSweep(sa,sb,r,segment==0,first);
   if(segment==0&&hit<0)return new Trace(0,new Vector3f(),true);
   if(hit>0){bestT=t0+(t1-t0)*first[0];bestNormal=new double[]{first[1],first[2],first[3]};break;}
  }
  if(bestT>=1)return new Trace(1,new Vector3f(),false);
  double pulled=length>0?Math.max(0,bestT-SURFACE_CLIP_EPSILON/(length*BLOCK)):0;
  return new Trace((float)pulled,new Vector3f((float)bestNormal[0],(float)bestNormal[1],(float)bestNormal[2]),false);
 }
 // sim voxel.rs World::sweep_impl over Minecraft's collision boxes: -1 start solid, 1 hit (first = t, normal), 0 clear.
 private int segmentSweep(double[] a,double[] b,double r,boolean testStart,double[] first){
  double[] delta={b[0]-a[0],b[1]-a[1],b[2]-a[2]};
  int[] lo=new int[3],hi=new int[3];
  for(int k=0;k<3;k++){lo[k]=(int)Math.floor(Math.min(a[k],b[k])-r-1);hi[k]=(int)Math.floor(Math.max(a[k],b[k])+r+1);}
  boolean startSolid=false;double best=Double.MAX_VALUE;int result=0;
  for(int y=lo[1];y<=hi[1];y++)for(int z=lo[2];z<=hi[2];z++)for(int x=lo[0];x<=hi[0];x++){
   cell.set(x,y,z);BlockState state=level.getBlockState(cell);if(state.isAir())continue;
   VoxelShape shape=state.getCollisionShape(level,cell);if(shape.isEmpty())continue;
   for(AABB box:shape.toAabbs()){
    // The shape grown by the moving box.
    double[] bmin={x+box.minX-r,y+box.minY-r,z+box.minZ-r},bmax={x+box.maxX+r,y+box.maxY+r,z+box.maxZ+r};
    if(a[0]>bmin[0]+INSIDE&&a[0]<bmax[0]-INSIDE&&a[1]>bmin[1]+INSIDE&&a[1]<bmax[1]-INSIDE&&a[2]>bmin[2]+INSIDE&&a[2]<bmax[2]-INSIDE){if(testStart)startSolid=true;continue;}
    double tIn=0,tOut=1;int axis=-1;boolean hit=true;
    for(int k=0;k<3;k++){
     if(Math.abs(delta[k])<1e-12){if(a[k]<=bmin[k]||a[k]>=bmax[k]){hit=false;break;}continue;}
     double e0=(bmin[k]-a[k])/delta[k],e1=(bmax[k]-a[k])/delta[k],near=Math.min(e0,e1),far=Math.max(e0,e1);
     if(near>tIn||near==tIn&&axis<0){tIn=near;axis=k;}
     tOut=Math.min(tOut,far);
     if(tIn>=tOut){hit=false;break;}
    }
    if(hit&&axis>=0&&tIn<best){best=tIn;first[0]=tIn;first[1]=first[2]=first[3]=0;first[1+axis]=-Math.signum(delta[axis]);result=1;}
   }
  }
  return startSolid?-1:result;
 }
 // glam Quat::from_rotation_arc
 private static Quaternionf arc(Vector3f from,Vector3f to){
  float oneMinusEps=1-2*Math.ulp(1f),dot=from.dot(to);
  if(dot>oneMinusEps)return new Quaternionf();
  if(dot<-oneMinusEps){
   float sign=from.z>=0?1:-1,a=-1/(sign+from.z),b=from.x*from.y*a;
   return new Quaternionf().fromAxisAngleRad(b,sign+from.y*from.y*a,-from.y,(float)Math.PI);
  }
  Vector3f c=new Vector3f(from).cross(to);
  return new Quaternionf(c.x,c.y,c.z,1+dot).normalize();
 }
 // glam Vec3::normalize_or_zero
 private static Vector3f normalizeOrZero(Vector3f v){float length=v.length(),rcp=1/length;return Float.isFinite(rcp)&&rcp>0?v.mul(rcp):v.zero();}
}
