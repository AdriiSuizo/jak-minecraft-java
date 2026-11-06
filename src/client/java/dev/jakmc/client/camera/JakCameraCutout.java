package dev.jakmc.client.camera;

import dev.jakmc.client.JakPoses;
import java.util.ArrayList;
import java.util.List;
import net.minecraft.client.Camera;
import net.minecraft.client.Minecraft;
import net.minecraft.core.BlockPos;
import net.minecraft.world.entity.Entity;
import net.minecraft.world.level.Level;
import net.minecraft.world.level.block.Block;
import net.minecraft.world.phys.AABB;
import net.minecraft.world.phys.Vec3;
import net.minecraft.world.phys.shapes.VoxelShape;

/**
 * The third-person camera's wall cutaway (Rust 28-29/10, cutout34-38): walls between
 * the camera and Jak are cut away by the terrain shader instead of pulling the
 * camera in; only floors and ceilings still stop it. Lengths are blocks: a Jak
 * metre is 4096/6400 of one (as in JakControllerCameraMixin), a map unit 1/25 m.
 */
public final class JakCameraCutout {
    static final double M=4096/6400.0,U=M/25,JAK_UNIT=1/6400.0;
    private static boolean enabled,blocked,used;
    private static float amount;
    private static double reach=Double.MAX_VALUE;
    private static Vec3 pivot;
    private static double feet;
    private static long lastReach,lastFrame;
    private static final float[] FADE=new float[8];
    private JakCameraCutout(){}

    /** What the terrain UBO carries: eye.xyz and opening, pivot.xyz and feet height, camera-relative. */
    public static float[] fade(){return FADE;}

    /** render_anim/src/jak.rs update: `cutout.enabled = voxel && zoom >= FIRST_PERSON` (Jak's own third-person camera). */
    public static boolean wanted(Entity entity){
        var client=Minecraft.getInstance();
        if(entity==null||entity!=client.player)return false;
        // Rust jak.rs update: only Jak's own follow camera cuts walls; a fixed fixture view is not it.
        if(dev.jakmc.client.InputScene.camera()!=null)return false;
        var type=client.options.getCameraType();
        return !type.isFirstPerson()&&!type.isMirrored()
            &&(JakPoses.presented()!=null||entity.getVehicle() instanceof dev.jakmc.entity.JakVehicle);
    }

    /**
     * render_anim/src/jak.rs update (the `cutout.enabled` branch) and follow_camera's
     * `distance.min(camera.reach.max(0.15))`: replaces vanilla's wall clip.
     */
    public static float maxZoom(Level level,Entity entity,Vec3 at,Vec3 forwards,float desired){
        long now=System.nanoTime();
        float dt=lastReach==0?0:Math.min(.1f,(now-lastReach)/1e9f);
        lastReach=now;
        Vec3 presented=JakPoses.presented();
        double feetY=presented!=null?presented.y:entity.getY();
        Vec3 eye=at.subtract(forwards.scale(desired));
        if(!enabled)reach=groundReach(level,at,eye);
        boolean[] wall=new boolean[1];
        double floorReach=cutoutObstruction(level,at,eye,feetY,wall);
        blocked=wall[0];
        // stand-in: the original eases per 60 Hz movement step (1-exp(-6*STEP)); here per rendered frame with its dt, the same rate.
        double returning=Math.min(floorReach,40*M);
        reach=floorReach<reach?floorReach:reach<1000*M?reach+(returning-reach)*(1-Math.exp(-6*dt)):floorReach;
        enabled=true;used=true;pivot=at;feet=feetY;
        return (float)Math.min(desired,Math.max(reach,.15*M));
    }

    /** render_anim/src/jak.rs OcclusionCutout::advance and the `camera_fade` hand-off after it. */
    public static void frame(Camera camera){
        long now=System.nanoTime();
        float dt=lastFrame==0?0:Math.min(.1f,(now-lastFrame)/1e9f);
        lastFrame=now;
        if(!used||!wanted(camera.entity())){enabled=false;blocked=false;pivot=null;}
        used=false;
        float wantedAmount=enabled&&blocked?1:0;
        amount+=(wantedAmount-amount)*(1-(float)Math.exp(-Math.max(dt,0)*10));
        java.util.Arrays.fill(FADE,0);
        if(enabled&&amount>.001f&&pivot!=null){
            Vec3 eye=camera.position();
            FADE[3]=amount;
            FADE[4]=(float)(pivot.x-eye.x);FADE[5]=(float)(pivot.y-eye.y);FADE[6]=(float)(pivot.z-eye.z);
            FADE[7]=(float)(feet-eye.y);
        }
    }

    /**
     * render_anim/src/jak.rs cutout_obstruction: walls affect only the aperture;
     * floor/ceiling clearance is the reach. Returns the reach (blocks, MAX for none).
     */
    static double cutoutObstruction(Level level,Vec3 target,Vec3 eye,double feetY,boolean[] blocked){
        Vec3 motion=eye.subtract(target);
        double length=motion.length();
        if(length<JAK_UNIT){blocked[0]=false;return Double.MAX_VALUE;}
        double radius=.65*M;
        var tris=fill(level,target,motion,radius,new double[]{feetY,target.y});
        Tri floor=probe(tris,target,motion,.3*M,WALKABLE);
        double reach=floor==null?Double.MAX_VALUE:Math.max(length*floor.u-.2*M,.15*M);
        Vec3 visible=reach!=Double.MAX_VALUE?motion.scale(Math.clamp(reach/length,0,1)):motion;
        blocked[0]=probe(tris,target,visible,radius,WALL)!=null;
        return reach;
    }

    /** render_anim/src/jak.rs ground_reach: the full sweep the camera starts from on leaving first person. */
    static double groundReach(Level level,Vec3 target,Vec3 eye){
        Vec3 motion=eye.subtract(target);
        double length=motion.length();
        if(length<JAK_UNIT)return Double.MAX_VALUE;
        var tris=fill(level,target,motion,.3*M,null);
        Tri hit=probe(tris,target,motion,.3*M,SOLID);
        return hit==null?Double.MAX_VALUE:Math.max(length*hit.u-.2*M,0);
    }

    static final class Tri {
        final Vec3[] v;final Vec3 n,min,max;double u;Vec3 intersect;
        Tri(Vec3 a,Vec3 b,Vec3 c,Vec3 n){v=new Vec3[]{a,b,c};this.n=n;min=min(min(a,b),c);max=max(max(a,b),c);}
    }

    /**
     * jak_mode/src/collide.rs CollideCache::fill_line_sphere/fill_box over
     * render_anim/src/jak.rs CameraBlockWorld (sim/src/voxel.rs camera_collision_triangles_in,
     * block_triangles_for). `band` null is the plain block world (block_triangles).
     */
    static List<Tri> fill(Level level,Vec3 start,Vec3 motion,double radius,double[] band){
        Vec3 end=start.add(motion);
        Vec3 lo=min(start,end).subtract(radius,radius,radius),hi=max(start,end).add(radius,radius,radius);
        var out=new ArrayList<Tri>();
        int[] a={(int)Math.floor(lo.x),(int)Math.floor(lo.y),(int)Math.floor(lo.z)};
        int[] b={(int)Math.floor(hi.x),(int)Math.floor(hi.y),(int)Math.floor(hi.z)};
        long volume=1;for(int k=0;k<3;k++)volume*=b[k]-a[k]+1L;
        if(volume>32*32*32)return out;
        int ox=a[0]-2,oy=a[1]-2,oz=a[2]-2,sx=b[0]-a[0]+5,sy=b[1]-a[1]+5,sz=b[2]-a[2]+5;
        VoxelShape[] shapes=new VoxelShape[sx*sy*sz];
        var pos=new BlockPos.MutableBlockPos();
        for(int x=0;x<sx;x++)for(int y=0;y<sy;y++)for(int z=0;z<sz;z++)
            shapes[(x*sy+y)*sz+z]=level.getBlockState(pos.set(ox+x,oy+y,oz+z)).getCollisionShape(level,pos);
        int[][] tangents={{1,2},{2,0},{0,1}};
        for(int x=a[0]-1;x<=b[0]+1;x++)for(int z=a[2]-1;z<=b[2]+1;z++)for(int y=a[1]-1;y<=b[1]+1;y++){
            VoxelShape shape=shapes[((x-ox)*sy+(y-oy))*sz+(z-oz)];
            if(shape.isEmpty())continue;
            double[] base={x,y,z};
            // stand-in: Rust Jak pieces collide with their level triangles; Minecraft blocks collide with their boxes.
            for(AABB box:shape.toAabbs()){
                double[] bl={box.minX,box.minY,box.minZ},bh={box.maxX,box.maxY,box.maxZ};
                for(int axis=0;axis<3;axis++){
                    int u=tangents[axis][0],v=tangents[axis][1];
                    for(int side=0;side<2;side++){
                        boolean high=side==1;
                        double at=high?bh[axis]:bl[axis];
                        int[] step={0,0,0};step[axis]=high?1:-1;
                        boolean onEdge=high?at>=1:at<=0;
                        if(onEdge&&Block.isShapeFullBlock(shapes[((x+step[0]-ox)*sy+(y+step[1]-oy))*sz+(z+step[2]-oz)])){
                            double height=base[1]+at;
                            boolean physicalSlab=axis==1&&band!=null&&(height<=band[0]+.125*U||height>=band[1]+.125*U);
                            if(!physicalSlab)continue;
                        }
                        Vec3[] quad={corner(base,axis,u,v,at,bl[u],bl[v]),corner(base,axis,u,v,at,bh[u],bl[v]),
                            corner(base,axis,u,v,at,bh[u],bh[v]),corner(base,axis,u,v,at,bl[u],bh[v])};
                        if(!high){Vec3 t=quad[0];quad[0]=quad[3];quad[3]=t;t=quad[1];quad[1]=quad[2];quad[2]=t;}
                        add(out,quad[0],quad[1],quad[2],lo,hi);
                        add(out,quad[0],quad[2],quad[3],lo,hi);
                    }
                }
            }
        }
        // stand-in: the original also adds sim::tri_world meshes and eco-vent triangles; in Java the vents are blocks with boxes (above) and there is no mesh world.
        return out;
    }

    static Vec3 min(Vec3 a,Vec3 b){return new Vec3(Math.min(a.x,b.x),Math.min(a.y,b.y),Math.min(a.z,b.z));}
    static Vec3 max(Vec3 a,Vec3 b){return new Vec3(Math.max(a.x,b.x),Math.max(a.y,b.y),Math.max(a.z,b.z));}

    private static Vec3 corner(double[] base,int axis,int u,int v,double at,double a,double b){
        double[] p=new double[3];p[axis]=at;p[u]=a;p[v]=b;
        return new Vec3(base[0]+p[0],base[1]+p[1],base[2]+p[2]);
    }

    /** jak_mode/src/collide.rs CollideCache::fill_box: the face normal, degenerate and out-of-box triangles dropped. */
    private static void add(List<Tri> out,Vec3 a,Vec3 b,Vec3 c,Vec3 lo,Vec3 hi){
        Vec3 n=c.subtract(b).cross(a.subtract(b));
        double l=n.length();
        if(Double.isNaN(l)||l<=1e-6*JAK_UNIT*JAK_UNIT||!Double.isFinite(n.x+n.y+n.z))return;
        var t=new Tri(a,b,c,n.scale(1/l));
        if(t.max.x<lo.x||t.max.y<lo.y||t.max.z<lo.z||t.min.x>hi.x||t.min.y>hi.y||t.min.z>hi.z)return;
        out.add(t);
    }

    static final int SOLID=0,WALKABLE=1,WALL=2;

    /**
     * jak_mode/src/collide.rs probe_line_sphere (resolve_moving_sphere, solid) and
     * probe_walkable_sphere / probe_wall_sphere (probe_camera_orientation).
     */
    static Tri probe(List<Tri> tris,Vec3 start,Vec3 motion,double radius,int mode){
        Tri found=null;double best=2;
        Vec3 lo=min(start,start.add(motion)).subtract(radius,radius,radius),hi=max(start,start.add(motion)).add(radius,radius,radius);
        for(Tri t:tris){
            if(mode==SOLID&&(t.max.x<lo.x||t.max.y<lo.y||t.max.z<lo.z||t.min.x>hi.x||t.min.y>hi.y||t.min.z>hi.z))continue;
            if(mode!=SOLID&&((Math.abs(t.n.y)>.65)!=(mode==WALKABLE)||motion.dot(t.n)>=0))continue;
            Hit hit=movingSphereTriangle(start,radius,motion,t);
            if(hit==null||hit.u<0||hit.u>=best)continue;
            if(mode==SOLID&&motion.dot(t.n)>=0)continue;
            if(start.subtract(hit.p).dot(t.n)<0)continue;
            best=hit.u;t.u=hit.u;t.intersect=hit.p;found=t;
        }
        return found;
    }

    record Hit(double u,Vec3 p){}

    /** jak_mode/src/collide.rs ray_sphere: where a ray enters the sphere (0 when it starts inside). */
    static double raySphere(Vec3 origin,Vec3 dir,Vec3 sphere,double radius){
        Vec3 o=origin.subtract(sphere);
        double c=o.lengthSqr()-radius*radius;
        if(c<0)return 0;
        double a=dir.lengthSqr();
        if(a==0)return -1;
        double b=dir.dot(o);
        if(b>=0)return -1;
        double disc=b*b-a*c;
        if(disc<0)return -1;
        double u=(-b-Math.sqrt(disc))/a;
        return u>1?-1:Math.max(u,0);
    }

    /** jak_mode/src/collide.rs ray_cylinder. */
    static Hit rayCylinder(Vec3 origin,Vec3 dir,Vec3 base,Vec3 axis,double radius,double length){
        Vec3 rel=origin.subtract(base);
        double s0=rel.dot(axis),ds=dir.dot(axis),s1=s0+ds;
        if(s0<0&&s1<0)return null;
        if(s0>=length&&s1>=length)return null;
        Vec3 perpO=rel.subtract(axis.scale(s0)),perpD=dir.subtract(axis.scale(ds));
        double u=raySphere(perpO,perpD,Vec3.ZERO,radius);
        if(u<0)return null;
        double s=s0+ds*u;
        if(s<0||s>=length)return null;
        return new Hit(u,base.add(axis.scale(s)));
    }

    /** jak_mode/src/collide.rs inside_triangle. */
    static boolean insideTriangle(Vec3 p,Tri t){
        Vec3 a=t.v[0],b=t.v[1],c=t.v[2],n=t.n;
        return b.subtract(a).cross(p.subtract(a)).dot(n)>=0&&c.subtract(b).cross(p.subtract(b)).dot(n)>=0
            &&a.subtract(c).cross(p.subtract(c)).dot(n)>=0;
    }

    /** jak_mode/src/collide.rs moving_sphere_triangle. */
    static Hit movingSphereTriangle(Vec3 center,double radius,Vec3 motion,Tri t){
        Vec3 n=t.n;
        double d=center.subtract(t.v[1]).dot(n),m=motion.dot(n);
        if(Math.abs(d)<radius){
            Vec3 p=center.subtract(n.scale(d));
            if(insideTriangle(p,t))return new Hit(0,p);
            return edgesAndCorners(center,radius,motion,t);
        }
        if(m==0)return null;
        double ta=(radius-d)/m,tb=(-radius-d)/m;
        if(ta<0&&tb<0)return null;
        double u=Math.min(ta,tb);
        if(u>=1)return null;
        Vec3 at=center.add(motion.scale(u));
        Vec3 p=at.subtract(n.scale(at.subtract(t.v[1]).dot(n)));
        if(insideTriangle(p,t))return new Hit(u,p);
        return edgesAndCorners(center,radius,motion,t);
    }

    /** jak_mode/src/collide.rs edges_and_corners. */
    static Hit edgesAndCorners(Vec3 center,double radius,Vec3 motion,Tri t){
        Hit best=null;
        for(Vec3 v:t.v){
            double u=raySphere(center,motion,v,radius);
            if(u>=0&&(best==null||u<best.u))best=new Hit(u,v);
        }
        for(int i=0;i<3;i++){
            Vec3 a=t.v[i],b=t.v[(i+1)%3],edge=b.subtract(a);
            double length=edge.length();
            if(length>0){
                Hit hit=rayCylinder(center,motion,a,edge.scale(1/length),radius,length);
                if(hit!=null&&hit.u>=0&&(best==null||hit.u<best.u))best=hit;
            }
        }
        return best!=null&&best.u<=1?best:null;
    }
}
