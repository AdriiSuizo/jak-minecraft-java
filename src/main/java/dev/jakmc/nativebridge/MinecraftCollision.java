package dev.jakmc.nativebridge;

import java.nio.ByteBuffer;
import java.util.Arrays;
import dev.jakmc.content.JakAssemblies;
import net.minecraft.core.BlockPos;
import net.minecraft.core.Direction;
import net.minecraft.world.entity.Entity;
import net.minecraft.world.level.Level;
import net.minecraft.world.level.block.LadderBlock;
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
        /** Native ladder column, facing away from the supporting wall. Coordinates are relative to the session. */
        public float[] ladder(float x,float y,float z){
            Vec3 feet=origin.add(x,y,z);
            var center=BlockPos.containing(feet);
            var probe=new BlockPos.MutableBlockPos();
            BlockPos best=null;
            Direction facing=null;
            double distance=Double.POSITIVE_INFINITY;
            for(int px=center.getX()-1;px<=center.getX()+1;px++){
                for(int pz=center.getZ()-1;pz<=center.getZ()+1;pz++){
                    for(int py=center.getY()-1;py<=center.getY()+1;py++){
                        if(feet.y>py+1.001)continue;
                        Direction side=ladderFacing(probe.set(px,py,pz));
                        if(side==null)continue;
                        int dx=side.getStepX(),dz=side.getStepZ();
                        double vx=feet.x-(px+.5+.01*dx),vz=feet.z-(pz+.5+.01*dz);
                        double along=vx*dx+vz*dz,lateral=Math.abs(vx*dz-vz*dx);
                        double squared=vx*vx+vz*vz;
                        if(along<-.16||along>.65||lateral>.48||squared>=distance)continue;
                        best=probe.immutable();
                        facing=side;
                        distance=squared;
                    }
                }
            }
            if(best==null)return null;
            int px=best.getX(),pz=best.getZ(),bottom=best.getY(),top=bottom+1;
            while(bottom>level.getMinY()&&ladderFacing(probe.set(px,bottom-1,pz))==facing)bottom--;
            while(top<=level.getMaxY()&&ladderFacing(probe.set(px,top,pz))==facing)top++;
            int dx=facing.getStepX(),dz=facing.getStepZ();
            return new float[]{(float)(px+.5+.01*dx-origin.x),y,(float)(pz+.5+.01*dz-origin.z),
                dx,0,dz,(float)(bottom-origin.y),(float)(top-origin.y)};
        }

        private Direction ladderFacing(BlockPos pos){
            if(!level.hasChunkAt(pos))return null;
            var state=level.getBlockState(pos);
            return state.getBlock() instanceof LadderBlock?state.getValue(LadderBlock.FACING):null;
        }

        public java.util.List<java.util.List<Vec3>> rails(Vec3 at){
            if(!(level instanceof net.minecraft.server.level.ServerLevel server))return java.util.List.of();
            var anchors=new java.util.ArrayList<JakAssemblies.RailAnchor>();
            var center=BlockPos.containing(at);
            int minX=(center.getX()-32)>>4,maxX=(center.getX()+32)>>4;
            int minZ=(center.getZ()-32)>>4,maxZ=(center.getZ()+32)>>4;
            for(int x=minX;x<=maxX;x++)for(int z=minZ;z<=maxZ;z++){
                var chunk=server.getChunkSource().getChunkNow(x,z);
                if(chunk==null)continue;
                for(var blockEntity:chunk.getBlockEntities().values())
                    if(blockEntity instanceof JakAssemblies.RailAnchor anchor)anchors.add(anchor);
            }
            anchors.sort(java.util.Comparator.comparingLong(anchor->anchor.getBlockPos().asLong()));
            return anchors.stream().flatMap(anchor->anchor.lines().stream()).limit(128).toList();
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
    private record Track(BlockPos at,net.minecraft.world.level.block.state.BlockState state,JakAssemblies.TrackMesh mesh){}
    private static final class Boxes {
        final java.util.ArrayList<Track> tracks=new java.util.ArrayList<>();
        float[] values=new float[384];int used;
        void add(double x,double y,double z,double X,double Y,double Z){if(used+6>values.length)values=Arrays.copyOf(values,values.length*2);values[used++]=(float)x;values[used++]=(float)y;values[used++]=(float)z;values[used++]=(float)X;values[used++]=(float)Y;values[used++]=(float)Z;}
    }
    public static ByteBuffer triangles(Level level,Entity actor,Vec3 origin,AABB region){return triangles(level,actor,origin,region,new Boxes(),null);}
    private static ByteBuffer triangles(Level level,Entity actor,Vec3 origin,AABB region,Boxes boxes,DirectBufferArena arena){
        boxes.used=0;boxes.tracks.clear();
        int trackBytes=0;var context=CollisionContext.of(actor);
        for(BlockPos pos:BlockPos.betweenClosed(BlockPos.containing(region.minX,region.minY,region.minZ),BlockPos.containing(region.maxX,region.maxY,region.maxZ))){
            double x=pos.getX(),y=pos.getY(),z=pos.getZ();
            // A missing column remains solid, never air beneath fast-moving actors.
            if(!level.hasChunkAt(pos)){boxes.add(x-origin.x,y-origin.y,z-origin.z,x+1-origin.x,y+1-origin.y,z+1-origin.z);continue;}
            var state=level.getBlockState(pos);
            var track=JakAssemblies.trackMesh(state);
            if(track!=null){
                trackBytes=Math.addExact(trackBytes,track.capacity());
                boxes.tracks.add(new Track(pos.immutable(),state,track));
                continue;
            }
            var shape=state.getCollisionShape(level,pos,context);
            if(shape==net.minecraft.world.phys.shapes.Shapes.block()){boxes.add(x-origin.x,y-origin.y,z-origin.z,x+1-origin.x,y+1-origin.y,z+1-origin.z);}
            else if(!shape.isEmpty())shape.forAllBoxes((x0,y0,z0,x1,y1,z1)->boxes.add(x0+x-origin.x,y0+y-origin.y,z0+z-origin.z,x1+x-origin.x,y1+y-origin.y,z1+z-origin.z));
        }
        var decks=level.getEntitiesOfClass(dev.jakmc.entity.JakHellcat.class,region.inflate(4),
            deck->deck!=actor&&!deck.wrecked()&&(actor==null||deck.pilotId()!=actor.getId()));
        int bytes=Math.addExact(Math.multiplyExact(boxes.used/6,480),trackBytes);
        int capacity=decks.isEmpty()?bytes:Math.addExact(bytes,Math.multiplyExact(decks.size(),dev.jakmc.entity.JakHellcat.deckCapacity()));
        ByteBuffer data=arena==null?NativeJak.buffer(capacity):arena.acquire(capacity);
        for(int i=0;i<boxes.used;i+=6)box(data,boxes.values[i],boxes.values[i+1],boxes.values[i+2],boxes.values[i+3],boxes.values[i+4],boxes.values[i+5]);
        for(var track:boxes.tracks){
            var pos=track.at();
            track.mesh().write(data,level,pos,track.state(),pos.getX()-origin.x,pos.getY()-origin.y,pos.getZ()-origin.z);
        }
        for(var deck:decks)deck.appendDeck(origin,region,data);
        return data.slice(0,data.position()).order(java.nio.ByteOrder.LITTLE_ENDIAN);
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
        return actors(level,owner,origin,region,arena,e->true);
    }
    public static ByteBuffer actors(Level level,Entity owner,Vec3 origin,AABB region,DirectBufferArena arena,java.util.function.Predicate<Entity> extra) {
        var entities=level.getEntities(owner,region,e->e.isAlive()&&e.isPickable()&&extra.test(e)&&(!(e instanceof dev.jakmc.entity.JakLeaper mount)||mount.riderId()!=owner.getId()));
        ByteBuffer data=arena==null?NativeJak.buffer(entities.size()*32):arena.acquire(entities.size()*32);
        for(Entity e:entities){AABB b=e.getBoundingBox().move(-origin.x,-origin.y,-origin.z);data.putLong(e.getId());
            data.putFloat((float)b.minX).putFloat((float)b.minY).putFloat((float)b.minZ);
            data.putFloat((float)b.maxX).putFloat((float)b.maxY).putFloat((float)b.maxZ);
        }
        return data;
    }
}
