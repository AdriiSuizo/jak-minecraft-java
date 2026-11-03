package dev.jakmc.nativebridge;

import net.minecraft.core.BlockPos;
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
    private final float[] contact=new float[4];

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
        var center=origin.add(x/METRES_PER_BLOCK,y/METRES_PER_BLOCK,z/METRES_PER_BLOCK);
        var delta=new Vec3(dx/METRES_PER_BLOCK,dy/METRES_PER_BLOCK,dz/METRES_PER_BLOCK);
        double across=hx/METRES_PER_BLOCK,up=hy/METRES_PER_BLOCK,longitudinal=hz/METRES_PER_BLOCK;
        var body=new AABB(center.x-across,center.y-up,center.z-longitudinal,
            center.x+across,center.y+up,center.z+longitudinal);
        var range=body.expandTowards(delta).inflate(.001);
        contact[0]=1;
        contact[1]=contact[2]=contact[3]=0;
        int minX=(int)Math.floor(range.minX)>>4,maxX=(int)Math.floor(range.maxX)>>4;
        int minZ=(int)Math.floor(range.minZ)>>4,maxZ=(int)Math.floor(range.maxZ)>>4;
        for(int cx=minX;cx<=maxX;cx++)for(int cz=minZ;cz<=maxZ;cz++){
            var at=new BlockPos(cx<<4,0,cz<<4);
            if(!level.hasChunkAt(at))sweepBox(center,delta,across,up,longitudinal,
                new AABB(cx*16,range.minY-1,cz*16,cx*16+16,range.maxY+1,cz*16+16));
        }
        for(var shape:level.getBlockCollisions(owner,range)){
            shape.forAllBoxes((x0,y0,z0,x1,y1,z1)->
                sweepBox(center,delta,across,up,longitudinal,new AABB(x0,y0,z0,x1,y1,z1)));
        }
        return contact;
    }

    private void sweepBox(Vec3 at,Vec3 motion,double hx,double hy,double hz,AABB shape){
        double x0=shape.minX-hx,x1=shape.maxX+hx;
        double y0=shape.minY-hy,y1=shape.maxY+hy;
        double z0=shape.minZ-hz,z1=shape.maxZ+hz;
        double start=0,end=contact[0];
        int face=-1;
        double sign=0;
        for(int axis=0;axis<3;axis++){
            double point=switch(axis){case 0->at.x;case 1->at.y;default->at.z;};
            double change=switch(axis){case 0->motion.x;case 1->motion.y;default->motion.z;};
            double low=switch(axis){case 0->x0;case 1->y0;default->z0;};
            double high=switch(axis){case 0->x1;case 1->y1;default->z1;};
            if(Math.abs(change)<1e-8){
                if(point<=low+1e-6||point>=high-1e-6)return;
                continue;
            }
            double hit1=(low-point)/change,hit2=(high-point)/change;
            double entering=Math.min(hit1,hit2);
            if(entering>=start-1e-6){start=Math.max(start,entering);face=axis;sign=-Math.signum(change);}
            end=Math.min(end,Math.max(hit1,hit2));
            if(start>end||end<0)return;
        }
        if(face<0||start>=contact[0])return;
        contact[0]=(float)Math.max(0,start-1e-4);
        contact[1]=contact[2]=contact[3]=0;
        contact[face+1]=(float)sign;
    }
}
