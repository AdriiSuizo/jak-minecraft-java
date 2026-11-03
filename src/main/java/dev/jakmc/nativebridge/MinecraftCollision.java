package dev.jakmc.nativebridge;

import java.nio.ByteBuffer;
import java.util.Arrays;
import net.minecraft.core.BlockPos;
import net.minecraft.world.entity.Entity;
import net.minecraft.world.level.Level;
import net.minecraft.world.phys.AABB;
import net.minecraft.world.phys.Vec3;
import net.minecraft.world.phys.shapes.CollisionContext;

/** Collision comes from live Minecraft shapes, including other mods, doors and slabs. */
public final class MinecraftCollision {
    private MinecraftCollision() { }
    /** Called synchronously on the Minecraft server thread, including distant shots. */
    public static final class Query {
        final Level level;final Entity actor;public Vec3 origin;
        long tick=Long.MIN_VALUE;
        final DirectBufferArena arena=new DirectBufferArena(16*1024*1024);
        final Boxes boxes=new Boxes();
        private record Region(int x,int y,int z,int X,int Y,int Z){}
        final java.util.Map<Region,ByteBuffer> cache=new java.util.LinkedHashMap<>();
        public Query(Level level,Entity actor,Vec3 origin){this.level=level;this.actor=actor;this.origin=origin;}
        /** Fraction along the aim ray blocked by a native collision shape. */
        public float aimDistance(float x,float y,float z,float endX,float endY,float endZ){
            Vec3 start=origin.add(x,y,z);
            Vec3 end=origin.add(endX,endY,endZ);
            var hit=level.clip(new net.minecraft.world.level.ClipContext(start,end,
                net.minecraft.world.level.ClipContext.Block.COLLIDER,
                net.minecraft.world.level.ClipContext.Fluid.NONE,actor));
            if(hit.getType()==net.minecraft.world.phys.HitResult.Type.MISS)return 1;
            return (float)(start.distanceTo(hit.getLocation())/start.distanceTo(end));
        }
        public ByteBuffer triangles(float x,float y,float z,float X,float Y,float Z){
            if(!Float.isFinite(x)||!Float.isFinite(y)||!Float.isFinite(z)||!Float.isFinite(X)||!Float.isFinite(Y)||!Float.isFinite(Z))throw new IllegalArgumentException("Nonfinite collision query");
            if(tick!=level.getGameTime()){cache.clear();arena.reset();tick=level.getGameTime();}
            Region r=new Region((int)Math.floor(origin.x+x-.01),(int)Math.floor(origin.y+y-.01),(int)Math.floor(origin.z+z-.01),(int)Math.ceil(origin.x+X+.01),(int)Math.ceil(origin.y+Y+.01),(int)Math.ceil(origin.z+Z+.01));
            return cache.computeIfAbsent(r,key->{
                long volume=(long)(key.X-key.x+1)*(key.Y-key.y+1)*(key.Z-key.z+1);
                if(volume<0||volume>262144)throw new IllegalArgumentException("Collision query volume exceeds native budget: "+volume);
                return MinecraftCollision.triangles(level,actor,origin,new AABB(key.x,key.y,key.z,key.X,key.Y,key.Z),boxes,arena);
            });
        }
    }
    private static final class Boxes {
        float[] values=new float[384];int used;
        void add(double x,double y,double z,double X,double Y,double Z){if(used+6>values.length)values=Arrays.copyOf(values,values.length*2);values[used++]=(float)x;values[used++]=(float)y;values[used++]=(float)z;values[used++]=(float)X;values[used++]=(float)Y;values[used++]=(float)Z;}
    }
    public static ByteBuffer triangles(Level level,Entity actor,Vec3 origin,AABB region){return triangles(level,actor,origin,region,new Boxes(),null);}
    private static ByteBuffer triangles(Level level,Entity actor,Vec3 origin,AABB region,Boxes boxes,DirectBufferArena arena){
        boxes.used=0;var context=CollisionContext.of(actor);
        for(BlockPos pos:BlockPos.betweenClosed(BlockPos.containing(region.minX,region.minY,region.minZ),BlockPos.containing(region.maxX,region.maxY,region.maxZ))){
            double x=pos.getX(),y=pos.getY(),z=pos.getZ();
            // A missing column remains solid, never air beneath fast-moving actors.
            if(!level.hasChunkAt(pos)){boxes.add(x-origin.x,y-origin.y,z-origin.z,x+1-origin.x,y+1-origin.y,z+1-origin.z);continue;}
            var shape=level.getBlockState(pos).getCollisionShape(level,pos,context);
            if(shape==net.minecraft.world.phys.shapes.Shapes.block()){boxes.add(x-origin.x,y-origin.y,z-origin.z,x+1-origin.x,y+1-origin.y,z+1-origin.z);}
            else if(!shape.isEmpty())shape.forAllBoxes((x0,y0,z0,x1,y1,z1)->boxes.add(x0+x-origin.x,y0+y-origin.y,z0+z-origin.z,x1+x-origin.x,y1+y-origin.y,z1+z-origin.z));
        }
        int bytes=Math.multiplyExact(boxes.used/6,480);ByteBuffer data=arena==null?NativeJak.buffer(bytes):arena.acquire(bytes);
        for(int i=0;i<boxes.used;i+=6)box(data,boxes.values[i],boxes.values[i+1],boxes.values[i+2],boxes.values[i+3],boxes.values[i+4],boxes.values[i+5]);return data;
    }
    private static void box(ByteBuffer b,float x,float y,float z,float X,float Y,float Z){
        face(b,0,x,Y,z,x,Y,Z,X,Y,Z,X,Y,z);
        face(b,1,x,y,z,X,y,z,X,y,Z,x,y,Z);
        face(b,1,x,y,z,x,y,Z,x,Y,Z,x,Y,z);
        face(b,1,X,y,Z,X,y,z,X,Y,z,X,Y,Z);
        face(b,1,X,y,z,x,y,z,x,Y,z,X,Y,z);
        face(b,1,x,y,Z,X,y,Z,X,Y,Z,x,Y,Z);
    }
    private static void face(ByteBuffer b,int flags,float ax,float ay,float az,float bx,float by,float bz,float cx,float cy,float cz,float dx,float dy,float dz){triangle(b,flags,ax,ay,az,bx,by,bz,cx,cy,cz);triangle(b,flags,ax,ay,az,cx,cy,cz,dx,dy,dz);}
    private static void triangle(ByteBuffer b,int flags,float ax,float ay,float az,float bx,float by,float bz,float cx,float cy,float cz){b.putFloat(ax).putFloat(ay).putFloat(az).putFloat(bx).putFloat(by).putFloat(bz).putFloat(cx).putFloat(cy).putFloat(cz).putInt(flags);}
    public static ByteBuffer actors(Level level,Entity owner,Vec3 origin,AABB region){return actors(level,owner,origin,region,null);}
    public static ByteBuffer actors(Level level,Entity owner,Vec3 origin,AABB region,DirectBufferArena arena) {
        var entities=level.getEntities(owner,region,e->e.isAlive()&&e.isPickable()&&(!(e instanceof dev.jakmc.entity.JakLeaper mount)||mount.riderId()!=owner.getId()));
        ByteBuffer data=arena==null?NativeJak.buffer(entities.size()*32):arena.acquire(entities.size()*32);
        for(Entity e:entities){AABB b=e.getBoundingBox().move(-origin.x,-origin.y,-origin.z);data.putLong(e.getId());
            data.putFloat((float)b.minX).putFloat((float)b.minY).putFloat((float)b.minZ);
            data.putFloat((float)b.maxX).putFloat((float)b.maxY).putFloat((float)b.maxZ);
        }
        return data;
    }
}
