package dev.jakmc.nativebridge;

import net.minecraft.core.BlockPos;
import dev.jakmc.entity.JakVehicle;
import dev.jakmc.player.JakPlayers;
import net.minecraft.server.level.ServerPlayer;
import net.minecraft.world.entity.Entity;
import net.minecraft.world.level.ClipContext;
import net.minecraft.world.level.Level;
import net.minecraft.world.phys.AABB;
import net.minecraft.world.phys.HitResult;
import net.minecraft.world.phys.Vec3;

/** Continuous chassis sweeps against native block shapes, in the same loaded terrain as wheel probes. */
public final class WheeledCollision {
    private static final double METRES_PER_BLOCK=6400.0/4096;
    private final Level level;
    private final Entity owner;
    private final boolean floats;
    public Vec3 origin;
    private final float[] contact=new float[5];
    private java.util.List<Entity> neighbours=java.util.List.of();
    private AABB neighbourhood;
    private long neighbourTick=Long.MIN_VALUE;

    public WheeledCollision(Level level,Entity owner,Vec3 origin,boolean floats){
        this.level=level;
        this.owner=owner;
        this.floats=floats;
        this.origin=origin;
    }

    public float support(float x,float y,float z,float reach){
        Vec3 at=origin.add(x/METRES_PER_BLOCK,y/METRES_PER_BLOCK,z/METRES_PER_BLOCK);
        if(!level.hasChunkAt(BlockPos.containing(at)))return Float.NaN;
        Vec3 bottom=at.add(0,-reach/METRES_PER_BLOCK,0);
        var result=level.clip(new ClipContext(at,bottom,ClipContext.Block.COLLIDER,floats?ClipContext.Fluid.ANY:ClipContext.Fluid.NONE,owner));
        if(result.getType()!=HitResult.Type.BLOCK)return Float.NaN;
        return (float)((at.y-result.getLocation().y)*METRES_PER_BLOCK);
    }

    public float[] sweep(float x,float y,float z,float dx,float dy,float dz,float hx,float hy,float hz){
        return sweepHull(x,y,z,dx,dy,dz,hx,hy,hz,0,0,1);
    }
    private final double[][] axes=new double[7][3];
    private double halfX,halfY,halfZ,forwardX,forwardZ;

    /** Continuous SAT against the actual rotated chassis, including its corners.
     * The former three axis-aligned beads left the diagonal nose corners uncovered. */
    public float[] sweepHull(float x,float y,float z,float dx,float dy,float dz,float hx,float hy,float hz,
                             float fx,float fy,float fz){
        var center=origin.add(x/METRES_PER_BLOCK,y/METRES_PER_BLOCK,z/METRES_PER_BLOCK);
        var delta=new Vec3(dx/METRES_PER_BLOCK,dy/METRES_PER_BLOCK,dz/METRES_PER_BLOCK);
        halfX=hx/METRES_PER_BLOCK;halfY=hy/METRES_PER_BLOCK;halfZ=hz/METRES_PER_BLOCK;
        double length=Math.hypot(fx,fz);
        forwardX=length>1e-8?fx/length:0;forwardZ=length>1e-8?fz/length:1;
        axes[0][0]=1;axes[1][1]=1;axes[2][2]=1;
        axes[3][0]=forwardZ;axes[3][2]=-forwardX;
        axes[4][0]=forwardX;axes[4][2]=forwardZ;
        double rx=Math.abs(forwardZ)*halfX+Math.abs(forwardX)*halfZ;
        double rz=Math.abs(forwardX)*halfX+Math.abs(forwardZ)*halfZ;
        var body=new AABB(center.x-rx,center.y-halfY,center.z-rz,center.x+rx,center.y+halfY,center.z+rz);
        var range=body.expandTowards(delta).inflate(.001);
        contact[0]=1;contact[1]=contact[2]=contact[3]=contact[4]=0;
        int minX=(int)Math.floor(range.minX)>>4,maxX=(int)Math.floor(range.maxX)>>4;
        int minZ=(int)Math.floor(range.minZ)>>4,maxZ=(int)Math.floor(range.maxZ)>>4;
        for(int cx=minX;cx<=maxX;cx++)for(int cz=minZ;cz<=maxZ;cz++){
            if(!level.hasChunkAt(new BlockPos(cx<<4,0,cz<<4)))sweepBox(center,delta,
                new AABB(cx*16,range.minY-1,cz*16,cx*16+16,range.maxY+1,cz*16+16));
        }
        for(var shape:level.getBlockCollisions(owner,range))shape.forAllBoxes((x0,y0,z0,x1,y1,z1)->
            sweepBox(center,delta,new AABB(x0,y0,z0,x1,y1,z1)));
        // All suspension substeps share the nearby entity lookup. Use the target's native hull, not its tracking box.
        if(neighbourTick!=level.getGameTime()||neighbourhood==null||!contains(neighbourhood,range)){
            neighbourhood=range.inflate(3);
            neighbourTick=level.getGameTime();
            neighbours=level.getEntities(owner,neighbourhood,e->e instanceof JakVehicle
                ||e instanceof ServerPlayer p&&JakPlayers.active(p)&&!p.isSpectator());
        }
        for(var other:neighbours){
            if(other.isRemoved()||!other.getBoundingBox().inflate(1).intersects(range))continue;
            if(other instanceof JakVehicle vehicle){
                var half=vehicle.collisionHalf();
                double yaw=Math.toRadians(vehicle.getYRot());
                sweepBody(center,delta,vehicle.position(),half.x,half.y,half.z,-Math.sin(yaw),Math.cos(yaw),true);
            }else sweepBox(center,delta,other.getBoundingBox(),true);
        }
        return contact;
    }

    private static boolean contains(AABB outer,AABB inner){
        return outer.minX<=inner.minX&&outer.minY<=inner.minY&&outer.minZ<=inner.minZ
            &&outer.maxX>=inner.maxX&&outer.maxY>=inner.maxY&&outer.maxZ>=inner.maxZ;
    }
    private void sweepBox(Vec3 at,Vec3 motion,AABB shape){
        sweepBox(at,motion,shape,false);
    }
    private void sweepBox(Vec3 at,Vec3 motion,AABB shape,boolean entity){
        sweepBody(at,motion,shape.getCenter(),shape.getXsize()/2,shape.getYsize()/2,shape.getZsize()/2,0,1,entity);
    }
    private void sweepBody(Vec3 at,Vec3 motion,Vec3 other,double ex,double ey,double ez,double fx,double fz,boolean entity){
        axes[5][0]=fz;axes[5][2]=-fx;
        axes[6][0]=fx;axes[6][2]=fz;
        double enter=Double.NEGATIVE_INFINITY,exit=contact[0],nx=0,ny=0,nz=0;
        double penetration=Double.POSITIVE_INFINITY,px=0,py=0,pz=0;
        for(int i=0;i<(entity?7:5);i++){
            var axis=axes[i];
            double x=axis[0],y=axis[1],z=axis[2];
            double distance=(at.x-other.x)*x+(at.y-other.y)*y+(at.z-other.z)*z;
            double speed=motion.x*x+motion.y*y+motion.z*z;
            double radius=halfX*Math.abs(x*forwardZ-z*forwardX)+halfY*Math.abs(y)
                +halfZ*Math.abs(x*forwardX+z*forwardZ)+ex*Math.abs(x*fz-z*fx)+ey*Math.abs(y)+ez*Math.abs(x*fx+z*fz);
            double overlap=radius-Math.abs(distance);
            if(overlap<penetration){penetration=overlap;double sign=distance<0?-1:1;px=x*sign;py=y*sign;pz=z*sign;}
            if(Math.abs(speed)<1e-10){if(overlap<=1e-7)return;continue;}
            double t1=(-radius-distance)/speed,t2=(radius-distance)/speed;
            double start=Math.min(t1,t2);
            if(start>enter){enter=start;double sign=-Math.signum(speed);nx=x*sign;ny=y*sign;nz=z*sign;}
            exit=Math.min(exit,Math.max(t1,t2));
            if(exit<Math.max(0,enter))return;
        }
        if(enter<0){
            if(penetration<=1e-7||motion.x*px+motion.y*py+motion.z*pz>1e-8)return;
            enter=0;nx=px;ny=py;nz=pz;
        }
        if(enter>=contact[0])return;
        contact[0]=(float)Math.max(0,enter-1e-4);
        contact[1]=(float)nx;contact[2]=(float)ny;contact[3]=(float)nz;
        contact[4]=entity?1:0;
    }
}
