package dev.jakmc.client.camera;

import dev.jakmc.client.JakInput;
import dev.jakmc.client.JakPoses;
import it.unimi.dsi.fastutil.longs.Long2ObjectOpenHashMap;
import java.nio.ByteBuffer;
import net.minecraft.client.Camera;
import net.minecraft.client.Minecraft;
import net.minecraft.core.BlockPos;
import net.minecraft.tags.BlockTags;
import net.minecraft.world.entity.Entity;
import net.minecraft.world.level.Level;
import net.minecraft.world.level.block.state.BlockState;
import net.minecraft.world.phys.AABB;
import net.minecraft.world.phys.Vec3;
import org.joml.Vector3fc;

/**
 * Jak's third-person camera: collision, bounds, occlusion mask, room cutaway and fading of objects.
 * Not a Jak port: Adrii chose (06/11) the camera of the series "Recreating Baldur's Gate 3's Camera System
 * in UE5" (camera movement, collision, bounds, occlusion masking, environment masks, room collections,
 * fading rooms, fading dynamic objects) to replace the earlier wall cutaway and its hard circle.
 * - Collision (JakCameraProbe): Adrii 07/11: the camera goes through walls and rock instead of being pushed
 *   in; only the ground at Jak's feet stops it (filtered: look-ahead sphere, quick smooth pull-in, wait, slow
 *   ease-out). With the eye inside rock (buried), the terrain is drawn with its back faces (terrain.fsh
 *   JAK_CAP): rock seen from inside is a dark fill, so no view through a wall reaches the sky or the void,
 *   and round Jak the fill opens on his cave or passage.
 * - Bounds: over a floor the camera's pitch slides toward the horizontal instead of going under it; the pivot
 *   drops out of a ceiling it would touch.
 * - Mask: a feathered, dithered ellipse around Jak on screen, only on blocks and entities nearer to the
 *   camera than he is, faded in while something hides him (terrain.fsh, entity.fsh, jakmc:camera_mask).
 * - Rooms (JakCameraRoom): indoors, the ceiling and everything above the floor plan, the walls above the
 *   ceiling and the walls on the camera's side are cut, cross-faded when the room changes.
 * The shader data rides in vanilla's Globals UBO (JakCameraFadeUboMixin, JakCameraFadeWriteMixin).
 */
public final class JakCameraCutout {
    /** Globals UBO layout (std140 bytes): vanilla ends at 48, then five mask vec4 and two rooms. */
    public static final int MASK_AT=48,ECO_AT=128,ROOM_AT=176,ROOM_BYTES=32+2*JakCameraRoom.N*4,LIGHT_AT=ROOM_AT+2*ROOM_BYTES,UBO_END=LIGHT_AT+80;
    /** Jak's body for the mask: centre height over the feet, and the ellipse half width/height at his depth. */
    static final double BODY=.95,HALF_WIDTH=.85,HALF_HEIGHT=1.25;
    /** Seconds a room cross-fade takes; mask fade rates (1/s) and how long it stays after the last hit. */
    static final double ROOM_FADE=.35,MASK_IN=12,MASK_OUT=5,MASK_HOLD=.3;
    /** The ground that stops the camera: what ends this far over Jak's feet (a block's step, a slab). */
    static final double GROUND=.6;
    /** Blocks round Jak the fill of rock opens on, feathered over its last CAP_FEATHER. */
    static final double CAP_REACH=5.5,CAP_FEATHER=1.5;
    /** How near a box the eye counts as in rock. */
    static final double BURIED=.3;

    private static final float[] MASK=new float[20];
    private static final float[][] ROOM={new float[8],new float[8]};
    private static final int[][] ROOM_BITS={new int[2*JakCameraRoom.N],new int[2*JakCameraRoom.N]};

    private static final JakCameraProbe.Boxes BOXES=new JakCameraProbe.Boxes();
    private static final JakCameraProbe.Result HARD=new JakCameraProbe.Result(),SOFT=new JakCameraProbe.Result();
    private static final JakCameraProbe.Rig RIG=new JakCameraProbe.Rig();
    private static boolean used,looking,buried;
    private static long lastReach,lastFrame;
    private static double pivotDrop,pitchOffset,pitchTarget;
    private static Vec3 lastPivot;
    private static Vec3 placedPivot;
    private static float placedPitch;
    private static double terrainAmount,entityAmount,terrainHold,entityHold;

    private static final JakCameraRoom JOB=new JakCameraRoom();
    private static JakCameraRoom.Shape room,previousRoom,pending;
    private static boolean pendingReady;
    private static double roomFade=1,refresh;
    private static long roomKey=Long.MIN_VALUE;
    private static Level roomLevel;

    private JakCameraCutout(){}

    /** render_anim/src/jak.rs update: `cutout.enabled = voxel && zoom >= FIRST_PERSON` (Jak's own third-person camera). */
    public static boolean wanted(Entity entity){
        var client=Minecraft.getInstance();
        if(entity==null||entity!=client.player)return false;
        // Rust jak.rs update: only Jak's own follow camera; a fixed fixture view is not it.
        if(dev.jakmc.client.InputScene.camera()!=null)return false;
        var type=client.options.getCameraType();
        return !type.isFirstPerson()&&!type.isMirrored()
            &&(JakPoses.presented()!=null||entity.getVehicle() instanceof dev.jakmc.entity.JakVehicle);
    }

    /** HellCat flight or a Jak vehicle: their own chase cameras, no pitch bounds, rooms or entity fading. */
    static boolean riding(Entity entity){
        if(entity.getVehicle() instanceof dev.jakmc.entity.JakVehicle)return true;
        var pose=JakPoses.rendered(entity.getId());
        return pose!=null&&pose.raw().get("hellcat") instanceof com.google.gson.JsonObject;
    }

    /** The eye is inside rock: the terrain is drawn with its fill (JakCameraBuriedMixin), and the section graph can't start from it. */
    public static boolean buried(){return buried;}
    /** The pivot the camera now orbits (dropped out of a ceiling), or null. Read by JakCameraCutoutMixin. */
    public static Vec3 placedPivot(){return placedPivot;}
    /** The pitch the camera now looks with (bounded under ceilings). */
    public static float placedPitch(){return placedPitch;}

    /**
     * Replaces Camera.getMaxZoom for Jak: the distance along the view line from `at`, with the pivot and
     * pitch adjustments left in placedPivot/placedPitch for the mixin to apply first.
     */
    public static float maxZoom(Level level,Entity entity,Vec3 at,float yaw,float pitch,float desired){
        long now=System.nanoTime();
        double dt=lastReach==0?0:Math.min(.1,(now-lastReach)/1e9);
        lastReach=now;
        refreshCells(level);
        Vec3 presented=JakPoses.presented();
        double feetY=presented!=null?presented.y:entity.getY();
        boolean riding=riding(entity);
        if(lastPivot==null||lastPivot.distanceToSqr(at)>9){RIG.reset();pitchOffset=pitchTarget=0;pivotDrop=0;}
        lastPivot=at;

        // Pivot room: drop out of a ceiling (stairs, low doorways) so the line never starts inside a block.
        BOXES.clear();
        collect(level,at.x-1.3,at.y-1.3,at.z-1.3,at.x+1.3,at.y+1.3,at.z+1.3);
        double clear=JakCameraProbe.clearance(BOXES,at.x,at.y,at.z,1);
        double drop=0;
        if(clear<JakCameraProbe.HARD+.02){
            for(int k=1;k<=10;k++){
                double y=at.y-.1*k;
                if(y<feetY+.5)break;
                if(JakCameraProbe.clearance(BOXES,at.x,y,at.z,1)>=JakCameraProbe.HARD+.02){drop=.1*k;break;}
            }
        }
        pivotDrop=drop>pivotDrop?drop:pivotDrop+(drop-pivotDrop)*(1-Math.exp(-8*dt));
        Vec3 pivot=at.subtract(0,pivotDrop,0);
        clear=JakCameraProbe.clearance(BOXES,pivot.x,pivot.y,pivot.z,1);
        double hard=Math.max(.05,Math.min(JakCameraProbe.HARD,clear*.95));
        double soft=Math.max(hard,Math.min(JakCameraProbe.SOFT,clear-.05));
        double boost=room!=null&&roomAmount(0)>.5?1.4:0;

        // Bounds: when a ceiling, a slope or the floor cuts the line short, the pitch slides instead.
        if(!riding&&!JakInput.aiming()){
            Vec3 from=pivot;
            pitchTarget=JakCameraProbe.boundPitch(pitch,pitchTarget,desired,p->line(level,from,yaw,p,desired,hard,feetY,boost));
        }else pitchTarget=0;
        pitchOffset+=(pitchTarget-pitchOffset)*(1-Math.exp(-6*dt));
        if(Math.abs(pitchOffset)<.01&&pitchTarget==0)pitchOffset=0;
        float usedPitch=(float)(pitch+pitchOffset);

        double[] back=JakCameraProbe.back(yaw,usedPitch);
        Vec3 end=pivot.add(back[0]*desired,back[1]*desired,back[2]*desired);
        BOXES.clear();
        double grow=soft+.1;
        if(!collect(level,Math.min(pivot.x,end.x)-grow,Math.min(pivot.y,end.y)-grow,Math.min(pivot.z,end.z)-grow,
            Math.max(pivot.x,end.x)+grow,Math.max(pivot.y,end.y)+grow,Math.max(pivot.z,end.z)+grow)){
            placedPivot=pivot;placedPitch=usedPitch;used=true;looking=false;
            return desired;// a chase line too long to scan (flight at speed): leave it free
        }
        BOXES.keepBelow(feetY+GROUND);
        JakCameraProbe.decide(BOXES,pivot.x,pivot.y,pivot.z,back[0],back[1],back[2],desired,hard,feetY,looking,boost,HARD);
        JakCameraProbe.decide(BOXES,pivot.x,pivot.y,pivot.z,back[0],back[1],back[2],desired,soft,feetY,looking,boost,SOFT);
        looking=HARD.lookedThrough>0;
        // The look-ahead sphere only starts the pull-in early: where it rests it gives back its extra radius.
        double distance=RIG.step(HARD.distance,SOFT.distance+(soft-hard),desired,dt);
        placedPivot=pivot;placedPitch=usedPitch;used=true;
        return (float)distance;
    }

    /** One candidate view line's hard-sphere distance (the bounds search). */
    private static double line(Level level,Vec3 pivot,float yaw,double pitch,double desired,double r,double feetY,double boost){
        double[] back=JakCameraProbe.back(yaw,pitch);
        double ex=pivot.x+back[0]*desired,ey=pivot.y+back[1]*desired,ez=pivot.z+back[2]*desired;
        BOXES.clear();
        double grow=r+.1;
        if(!collect(level,Math.min(pivot.x,ex)-grow,Math.min(pivot.y,ey)-grow,Math.min(pivot.z,ez)-grow,
            Math.max(pivot.x,ex)+grow,Math.max(pivot.y,ey)+grow,Math.max(pivot.z,ez)+grow))return desired;
        BOXES.keepBelow(feetY+GROUND);
        JakCameraProbe.decide(BOXES,pivot.x,pivot.y,pivot.z,back[0],back[1],back[2],desired,r,feetY,looking,boost,HARD);
        return HARD.distance;
    }

    /** After Camera.update: the mask and room data for this frame's shaders. */
    public static void frame(Camera camera){
        long now=System.nanoTime();
        double dt=lastFrame==0?0:Math.min(.1,(now-lastFrame)/1e9);
        lastFrame=now;
        Entity entity=camera.entity();
        boolean on=used&&wanted(entity)&&camera.isDetached();
        used=false;
        if(!on){
            RIG.reset();looking=false;buried=false;lastPivot=null;pitchOffset=pitchTarget=pivotDrop=0;
            terrainAmount=entityAmount=terrainHold=entityHold=0;
            room=previousRoom=pending=null;pendingReady=false;roomFade=1;roomKey=Long.MIN_VALUE;
            java.util.Arrays.fill(MASK,0);ROOM[0][3]=ROOM[1][3]=0;
            return;
        }
        Level level=entity.level();
        Vec3 eye=camera.position();
        refreshCells(level);
        BOXES.clear();
        buried=collect(level,eye.x-BURIED,eye.y-BURIED,eye.z-BURIED,eye.x+BURIED,eye.y+BURIED,eye.z+BURIED)
            &&JakCameraProbe.clearance(BOXES,eye.x,eye.y,eye.z,1)<BURIED;
        Vector3fc f=camera.forwardVector(),up=camera.upVector();
        Vec3 presented=JakPoses.presented();
        Vec3 feet=presented!=null?presented:entity.position();
        boolean riding=riding(entity);
        Vec3 centre=feet.add(0,BODY,0);

        // Mask: on while the head or chest is hidden by a block above the kept wall foot, or by an entity.
        refreshCells(level);
        boolean blocked=looking;
        for(double h:new double[]{1.0,1.55}){
            if(blocked)break;
            Vec3 target=feet.add(0,h,0);
            BOXES.clear();
            if(collect(level,Math.min(eye.x,target.x)-.1,Math.min(eye.y,target.y)-.1,Math.min(eye.z,target.z)-.1,
                Math.max(eye.x,target.x)+.1,Math.max(eye.y,target.y)+.1,Math.max(eye.z,target.z)+.1))
                blocked=JakCameraProbe.blocks(BOXES,eye.x,eye.y,eye.z,target.x,target.y,target.z,.03,.35,feet.y+.6);
        }
        boolean mobs=!riding&&entitiesHide(level,entity,eye,feet);
        terrainHold=blocked?MASK_HOLD:terrainHold-dt;
        entityHold=mobs?MASK_HOLD:entityHold-dt;
        terrainAmount=approach(terrainAmount,terrainHold>0?1:0,dt);
        entityAmount=approach(entityAmount,entityHold>0?1:0,dt);

        updateRoom(level,feet,!riding,dt);

        double cx=centre.x-eye.x,cy=centre.y-eye.y,cz=centre.z-eye.z;
        double depth=cx*f.x()+cy*f.y()+cz*f.z();
        MASK[0]=(float)cx;MASK[1]=(float)cy;MASK[2]=(float)cz;MASK[3]=(float)smooth(terrainAmount);
        MASK[4]=f.x();MASK[5]=f.y();MASK[6]=f.z();MASK[7]=(float)(feet.y-eye.y);
        MASK[8]=up.x();MASK[9]=up.y();MASK[10]=up.z();MASK[11]=(float)smooth(entityAmount);
        MASK[12]=(float)HALF_WIDTH;MASK[13]=(float)HALF_HEIGHT;
        // Terrain is cut up to just behind Jak's back; entities stop short of his whole body (his own model).
        MASK[14]=(float)(depth-.3);
        MASK[15]=(float)(depth-(BODY*Math.abs(f.y())+.45));
        // Under cover the fill closes everything but the place round Jak; in the open what is seen past a hillside the
        // camera sits in is the world Jak stands in, so only rock farther than him is filled.
        boolean open=level.getHeight(net.minecraft.world.level.levelgen.Heightmap.Types.MOTION_BLOCKING_NO_LEAVES,(int)Math.floor(feet.x),(int)Math.floor(feet.z))<=feet.y+1.5;
        MASK[16]=open?0:1;MASK[17]=(float)CAP_REACH;MASK[18]=(float)CAP_FEATHER;MASK[19]=(float)depth;
        pack(0,room,roomAmount(0),eye,depth);
        pack(1,previousRoom,roomAmount(1),eye,depth);
    }

    private static double approach(double value,double target,double dt){
        double rate=target>value?MASK_IN:MASK_OUT;
        return value+(target-value)*(1-Math.exp(-rate*dt));
    }

    private static double smooth(double x){x=Math.clamp(x,0,1);return x*x*(3-2*x);}

    /** Mobs, players and props between the camera and Jak's body (his own vehicle and riders excluded). */
    private static boolean entitiesHide(Level level,Entity jak,Vec3 eye,Vec3 feet){
        Vec3 chest=feet.add(0,1.0,0),head=feet.add(0,1.55,0);
        var box=new AABB(eye,head).minmax(new AABB(chest,chest)).inflate(1);
        for(Entity e:level.getEntities(jak,box,e->e.isAlive()&&!e.isInvisible()&&e!=jak.getVehicle()
            &&!e.isPassengerOfSameVehicle(jak)&&!e.hasPassenger(jak)&&!jak.hasPassenger(e))){
            var bb=e.getBoundingBox();
            for(Vec3 target:new Vec3[]{chest,head}){
                var hit=bb.clip(eye,target);
                if(hit.isPresent()&&hit.get().distanceTo(target)>.6)return true;
            }
        }
        return false;
    }

    // --- Rooms ---

    private static double roomAmount(int slot){
        double s=smooth(roomFade);
        return slot==0?(room==null?0:s):(previousRoom==null?0:1-s);
    }

    private static void updateRoom(Level level,Vec3 feet,boolean allowed,double dt){
        if(level!=roomLevel){roomLevel=level;room=previousRoom=pending=null;pendingReady=false;roomFade=1;roomKey=Long.MIN_VALUE;}
        int x=(int)Math.floor(feet.x),y=(int)Math.floor(feet.y+.01),z=(int)Math.floor(feet.z);
        // The room is the one Jak's floor is in: a jump or a fall of up to four blocks keeps it.
        for(int k=0;k<4;k++){
            if(!level.getBlockState(ROOM_POS.set(x,y-1,z)).getCollisionShape(level,ROOM_POS).isEmpty())break;
            y--;
        }
        long key=allowed?BlockPos.asLong(x,y,z):Long.MIN_VALUE+1;
        refresh-=dt;
        if(!allowed){
            if(key!=roomKey){roomKey=key;pending=null;pendingReady=true;}
        }else if(JOB.running()?Math.abs(JOB.startX()-x)+Math.abs(JOB.startY()-y)+Math.abs(JOB.startZ()-z)>2
            :key!=roomKey||refresh<=0){
            // A flood finishes before the next starts unless Jak has run off; a refresh each second catches edits.
            roomKey=key;refresh=1;
            JOB.start(x,y,z);
        }
        if(JOB.running()&&allowed){
            var shape=JOB.step(ROOM_WORLD,JakCameraRoom.BUDGET);
            if(shape!=JakCameraRoom.PENDING){pending=shape;pendingReady=true;}
        }
        roomFade=Math.min(1,roomFade+dt/ROOM_FADE);
        if(pendingReady&&roomFade>=1){
            pendingReady=false;
            boolean same=pending==null?room==null:pending.sameAs(room);
            if(!same){previousRoom=room;room=pending;roomFade=0;}
        }
        if(roomFade>=1)previousRoom=null;
    }

    private static final BlockPos.MutableBlockPos ROOM_POS=new BlockPos.MutableBlockPos();
    private static final JakCameraRoom.World ROOM_WORLD=(x,y0,z,out)->{
        int open=0,roof=0;
        for(int i=0;i<JakCameraRoom.LEVELS;i++){
            BlockState state=roomLevel.getBlockState(ROOM_POS.set(x,y0+i,z));
            if(state.isAir()){open|=1<<i;continue;}
            if(!state.isCollisionShapeFullBlock(roomLevel,ROOM_POS))open|=1<<i;
            if(!state.is(BlockTags.LEAVES)&&!state.getCollisionShape(roomLevel,ROOM_POS).isEmpty())roof|=1<<i;
        }
        out[0]=open;out[1]=roof;
    };

    private static void pack(int slot,JakCameraRoom.Shape shape,double amount,Vec3 eye,double depth){
        float[] data=ROOM[slot];
        if(shape==null||amount<=.001){data[3]=0;return;}
        data[0]=(float)(shape.ox-eye.x);data[1]=0;data[2]=(float)(shape.oz-eye.z);data[3]=(float)amount;
        data[4]=(float)(shape.cutY-eye.y);data[5]=(float)(shape.floorY-eye.y);data[6]=(float)(depth-.5);data[7]=0;
        System.arraycopy(shape.foot,0,ROOM_BITS[slot],0,JakCameraRoom.N);
        System.arraycopy(shape.rim,0,ROOM_BITS[slot],JakCameraRoom.N,JakCameraRoom.N);
    }

    /** JakCameraFadeWriteMixin: this frame's data after vanilla's Globals (buffer positioned at the block start). */
    public static void write(ByteBuffer buffer){
        for(int i=0;i<MASK.length;i++)buffer.putFloat(MASK_AT+4*i,MASK[i]);
        dev.jakmc.client.darkeco.DarkEcoClient.write(buffer,ECO_AT);
        dev.jakmc.client.JakLightRig.write(buffer,LIGHT_AT);
        for(int slot=0;slot<2;slot++){
            int at=ROOM_AT+slot*ROOM_BYTES;
            for(int i=0;i<8;i++)buffer.putFloat(at+4*i,ROOM[slot][i]);
            for(int i=0;i<2*JakCameraRoom.N;i++)buffer.putInt(at+32+4*i,ROOM_BITS[slot][i]);
        }
    }

    // --- Block collision boxes, cached for two ticks ---

    private record Cell(double[] boxes,boolean leaves){}
    private static final Cell EMPTY=new Cell(new double[0],false);
    private static final Long2ObjectOpenHashMap<Cell> CELLS=new Long2ObjectOpenHashMap<>();
    private static final BlockPos.MutableBlockPos CELL_POS=new BlockPos.MutableBlockPos();
    private static Level cellLevel;
    private static long cellTick=Long.MIN_VALUE;

    private static void refreshCells(Level level){
        long tick=level.getGameTime();
        if(level!=cellLevel||tick-cellTick>=2||tick<cellTick){CELLS.clear();cellLevel=level;cellTick=tick;}
    }

    private static Cell cell(Level level,int x,int y,int z){
        long key=BlockPos.asLong(x,y,z);
        Cell cell=CELLS.get(key);
        if(cell!=null)return cell;
        BlockState state=level.getBlockState(CELL_POS.set(x,y,z));
        if(state.isAir())cell=EMPTY;
        else{
            var shape=state.getCollisionShape(level,CELL_POS);
            if(shape.isEmpty())cell=EMPTY;
            else{
                var boxes=shape.toAabbs();
                double[] v=new double[6*boxes.size()];
                for(int i=0;i<boxes.size();i++){
                    AABB b=boxes.get(i);
                    v[6*i]=b.minX;v[6*i+1]=b.minY;v[6*i+2]=b.minZ;v[6*i+3]=b.maxX;v[6*i+4]=b.maxY;v[6*i+5]=b.maxZ;
                }
                cell=new Cell(v,state.is(BlockTags.LEAVES));
            }
        }
        CELLS.put(key,cell);
        return cell;
    }

    /** Adds the collision boxes of every block in the region to BOXES; false when the region is too big to scan. */
    private static boolean collect(Level level,double x0,double y0,double z0,double x1,double y1,double z1){
        int ax=(int)Math.floor(x0),ay=(int)Math.floor(y0),az=(int)Math.floor(z0);
        int bx=(int)Math.floor(x1),by=(int)Math.floor(y1),bz=(int)Math.floor(z1);
        long volume=(long)(bx-ax+1)*(by-ay+1)*(bz-az+1);
        if(volume>4096)return false;
        for(int x=ax;x<=bx;x++)for(int y=ay;y<=by;y++)for(int z=az;z<=bz;z++){
            Cell cell=cell(level,x,y,z);
            double[] v=cell.boxes;
            for(int i=0;i<v.length;i+=6)BOXES.add(x+v[i],y+v[i+1],z+v[i+2],x+v[i+3],y+v[i+4],z+v[i+5],cell.leaves);
        }
        return true;
    }
}
