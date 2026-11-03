package dev.jakmc.nativebridge;

import java.nio.ByteBuffer;
import java.util.ArrayList;
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
        private record Region(int x,int y,int z,int X,int Y,int Z){}
        final java.util.Map<Region,ByteBuffer> cache=new java.util.LinkedHashMap<>();
        public Query(Level level,Entity actor,Vec3 origin){this.level=level;this.actor=actor;this.origin=origin;}
        public ByteBuffer triangles(float x,float y,float z,float X,float Y,float Z){
            if(!Float.isFinite(x)||!Float.isFinite(y)||!Float.isFinite(z)||!Float.isFinite(X)||!Float.isFinite(Y)||!Float.isFinite(Z))throw new IllegalArgumentException("Nonfinite collision query");
            if(tick!=level.getGameTime()){cache.clear();tick=level.getGameTime();}
            Region r=new Region((int)Math.floor(origin.x+x-.01),(int)Math.floor(origin.y+y-.01),(int)Math.floor(origin.z+z-.01),(int)Math.ceil(origin.x+X+.01),(int)Math.ceil(origin.y+Y+.01),(int)Math.ceil(origin.z+Z+.01));
            return cache.computeIfAbsent(r,key->{
                long volume=(long)(key.X-key.x+1)*(key.Y-key.y+1)*(key.Z-key.z+1);
                if(volume<0||volume>262144)throw new IllegalArgumentException("Collision query volume exceeds native budget: "+volume);
                return MinecraftCollision.triangles(level,actor,origin,new AABB(key.x,key.y,key.z,key.X,key.Y,key.Z));
            });
        }
    }
    public static ByteBuffer triangles(Level level, Entity actor, Vec3 origin, AABB region) {
        var boxes=new ArrayList<AABB>();
        for(BlockPos pos:BlockPos.betweenClosed(BlockPos.containing(region.minX,region.minY,region.minZ),BlockPos.containing(region.maxX,region.maxY,region.maxZ))) {
            // An unloaded column is a temporary wall, never air beneath a fast vehicle.
            if (!level.hasChunkAt(pos)) { boxes.add(new AABB(pos)); continue; }
            var state=level.getBlockState(pos);
            state.getCollisionShape(level,pos,CollisionContext.of(actor)).forAllBoxes((x0,y0,z0,x1,y1,z1)->
                boxes.add(new AABB(x0+pos.getX(),y0+pos.getY(),z0+pos.getZ(),x1+pos.getX(),y1+pos.getY(),z1+pos.getZ())));
        }
        ByteBuffer data=NativeJak.buffer(Math.multiplyExact(boxes.size(),480));
        for(AABB box:boxes)box(data,box.move(-origin.x,-origin.y,-origin.z));
        return data;
    }
    private static void box(ByteBuffer b,AABB a) {
        float x=(float)a.minX,y=(float)a.minY,z=(float)a.minZ,X=(float)a.maxX,Y=(float)a.maxY,Z=(float)a.maxZ;
        face(b,0,x,Y,z,x,Y,Z,X,Y,Z,X,Y,z);
        face(b,1,x,y,z,X,y,z,X,y,Z,x,y,Z);
        face(b,1,x,y,z,x,y,Z,x,Y,Z,x,Y,z);
        face(b,1,X,y,Z,X,y,z,X,Y,z,X,Y,Z);
        face(b,1,X,y,z,x,y,z,x,Y,z,X,Y,z);
        face(b,1,x,y,Z,X,y,Z,X,Y,Z,x,Y,Z);
    }
    private static void face(ByteBuffer b,int flags,float... q) {
        for(int i:new int[]{0,1,2,0,2,3}) {
            b.putFloat(q[i*3]).putFloat(q[i*3+1]).putFloat(q[i*3+2]);
            if(b.position()%40==36)b.putInt(flags);
        }
    }
    public static ByteBuffer actors(Level level,Entity owner,Vec3 origin,AABB region) {
        var entities=level.getEntities(owner,region,e->e.isAlive()&&e.isPickable());
        ByteBuffer data=NativeJak.buffer(entities.size()*32);
        for(Entity e:entities){AABB b=e.getBoundingBox().move(-origin.x,-origin.y,-origin.z);data.putLong(e.getId());
            data.putFloat((float)b.minX).putFloat((float)b.minY).putFloat((float)b.minZ);
            data.putFloat((float)b.maxX).putFloat((float)b.maxY).putFloat((float)b.maxZ);
        }
        return data;
    }
}
