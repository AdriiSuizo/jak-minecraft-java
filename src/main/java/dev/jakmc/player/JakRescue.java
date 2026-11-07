package dev.jakmc.player;

import dev.jakmc.JakData;
import dev.jakmc.JakMod;
import java.util.*;
import net.fabricmc.fabric.api.event.lifecycle.v1.ServerTickEvents;
import net.fabricmc.fabric.api.networking.v1.PayloadTypeRegistry;
import net.fabricmc.fabric.api.networking.v1.ServerPlayNetworking;
import net.minecraft.core.BlockPos;
import net.minecraft.core.registries.BuiltInRegistries;
import net.minecraft.network.RegistryFriendlyByteBuf;
import net.minecraft.network.chat.Component;
import net.minecraft.network.codec.StreamCodec;
import net.minecraft.network.protocol.common.custom.CustomPacketPayload;
import net.minecraft.server.level.ServerLevel;
import net.minecraft.server.level.ServerPlayer;
import net.minecraft.world.level.Level;
import net.minecraft.world.level.block.Block;
import net.minecraft.world.level.levelgen.Heightmap;
import net.minecraft.world.phys.Vec3;

/**
 * The rescue (K, Adrii 18/10) and the automatic unstucks: back to Jak on safe
 * ground. Server half; the key and the fall watch of Jak's own state are in
 * dev.jakmc.client.camera.JakRescueClient. Lengths in Jak metres become blocks
 * at 4096/6400 a metre.
 */
public final class JakRescue {
    static final double M=4096/6400.0;
    /** render_anim/src/minecraft_world.rs RESCUE_FOOD. */
    static final int RESCUE_FOOD=6;
    // stand-in: BURIED_FRAMES is 12 rendered frames (0.2 s at the original 60 Hz); the server checks at 20 Hz, so 4 ticks.
    static final int BURIED_TICKS=4;
    public static final int RESCUE=0,FREE=1;

    /** RESCUE: the K key. FREE: render_anim/src/jak.rs fall_stuck's even free, with its count. */
    public record Request(int kind,int frees) implements CustomPacketPayload {
        public static final Type<Request> TYPE=new Type<>(JakData.id("rescue_v1"));
        public static final StreamCodec<RegistryFriendlyByteBuf,Request> CODEC=StreamCodec.of((b,v)->{b.writeVarInt(v.kind);b.writeVarInt(v.frees);},b->new Request(b.readVarInt(),b.readVarInt()));
        public Type<Request> type(){return TYPE;}
        boolean valid(){return kind==RESCUE&&frees==0||kind==FREE&&frees>=2&&frees<=4096&&frees%2==0;}
    }

    /** render_anim/src/minecraft_world.rs FallWatch, with the player's last ground and was_alive. */
    private static final class Watch { Double before; boolean under; int buried; Vec3 lastGround; boolean wasAlive; }
    private static final Map<UUID,Watch> WATCH=new HashMap<>();
    private JakRescue(){}

    public static void initialize(){
        PayloadTypeRegistry.serverboundPlay().register(Request.TYPE,Request.CODEC);
        ServerPlayNetworking.registerGlobalReceiver(Request.TYPE,(packet,context)->{
            if(!packet.valid())return;
            if(packet.kind()==RESCUE)rescue(context.player());else free(context.player(),packet.frees());
        });
        ServerTickEvents.END_SERVER_TICK.register(server->{
            var present=new HashSet<UUID>();
            for(ServerPlayer p:server.getPlayerList().getPlayers()){present.add(p.getUUID());watchFall(p);}
            WATCH.keySet().retainAll(present);
        });
    }

    /**
     * render_anim/src/jak.rs update (the rescue's Jak part) and
     * render_anim/src/minecraft_world.rs (the rescue's block-world part).
     */
    static void rescue(ServerPlayer p){
        // Jak's part: a soldier becomes Jak; Jak is himself again where he stands
        // (no Dark or Light Jak, off the Flut Flut, the board and the HellCat).
        if(!p.getAttachedOrCreate(JakData.ENABLED))JakPlayers.chooseMode(p,true);
        else JakPlayers.rescue(p);
        if(!p.isAlive())return;
        // The block world's part: screens shut, out of bed and off any seat.
        if(p.containerMenu!=p.inventoryMenu)p.closeContainer();
        if(p.isSleeping())p.stopSleepInBed(true,true);
        p.stopRiding();
        // stand-in: the original also asks for the gun in hand afresh (inventory_ui.reset_weapon); the Java gun follows the held item.
        if(p.getFoodData().getFoodLevel()<RESCUE_FOOD)p.getFoodData().setFoodLevel(RESCUE_FOOD);
        if(p.getHealth()>0&&p.getHealth()*4<p.getMaxHealth())p.heal(p.getMaxHealth()/4-p.getHealth());
        Vec3[] bed=new Vec3[1];
        Vec3 home=home(p,bed);
        // stand-in: the original adds the local player's index (0 or 1) to the ring; Java players are each on their own.
        Vec3 back=safeNear(p.level(),home,bed[0]!=null?1:0);
        if(back==null)back=home;
        teleport(p,back);
        var watch=WATCH.computeIfAbsent(p.getUUID(),id->new Watch());
        watch.before=null;watch.under=false;watch.buried=0;watch.lastGround=back;
        p.sendSystemMessage(Component.literal("Rescate: de vuelta como Jak, en un sitio seguro"),true);
        JakMod.LOG.info("rescue: {} back to {}",p.getScoreboardName(),back);
    }

    /**
     * render_anim/src/jak.rs update, fall_stuck's even free: moved back off what
     * holds him, 0.8 m per two frees, and 0.3 m up.
     */
    static void free(ServerPlayer p,int frees){
        if(!JakPlayers.active(p))return;
        double yaw=Math.toRadians(p.yBodyRot);
        Vec3 back=new Vec3(Math.sin(yaw),0,-Math.cos(yaw));
        teleport(p,p.position().add(back.scale(.8*M*frees*.5)).add(0,.3*M,0));
    }

    /**
     * render_anim/src/minecraft_world.rs: never a fall without end. Under the world's
     * floor, or falling with body and head inside full blocks, back to safe ground
     * near where they last stood (else the spawn).
     */
    static void watchFall(ServerPlayer p){
        var w=WATCH.computeIfAbsent(p.getUUID(),id->new Watch());
        boolean alive=p.isAlive();
        // stand-in: spectators fly through blocks by design; the original has none.
        if(p.isSpectator()){w.wasAlive=alive;w.before=null;return;}
        ServerLevel level=p.level();
        Vec3 feet=p.position();
        Boolean body=solid(level,feet,.5);
        boolean stands=p.onGround()||JakPlayers.active(p)&&Boolean.TRUE.equals(solid(level,feet,-.1))&&Boolean.FALSE.equals(body);
        if(stands)w.lastGround=feet;
        double floor=level.dimension()==Level.OVERWORLD?-64:0;
        double before=w.before==null?feet.y:w.before;
        w.before=feet.y;
        w.under=feet.y<floor-24;
        w.buried=alive&&fallingThroughBlocks(smothers(level,feet,.5),smothers(level,feet,1.5),before,feet.y)?w.buried+1:0;
        if(alive&&w.wasAlive&&(w.under||w.buried>=BURIED_TICKS)){
            boolean through=!w.under;
            Vec3 near=w.lastGround!=null?w.lastGround:home(p,new Vec3[1]);
            Vec3 back=safeNear(level,near,0);
            if(back==null)back=w.lastGround!=null?w.lastGround.add(0,.1,0):home(p,new Vec3[1]);
            teleport(p,back);
            w.before=null;w.under=false;w.buried=0;
            JakMod.LOG.info("Minecraft: fell {} at {}, back to {}",through?"through the blocks":"under the world",feet,back);
        }
        w.wasAlive=alive;
    }

    /** render_anim/src/minecraft_world.rs falling_through_blocks. */
    static boolean fallingThroughBlocks(Boolean body,Boolean head,double before,double feet){
        return Boolean.TRUE.equals(body)&&Boolean.TRUE.equals(head)&&feet<before;
    }

    /** render_anim/src/minecraft_world.rs `home`: the bed (overworld) or the world's spawn. */
    static Vec3 home(ServerPlayer p,Vec3[] bedSpot){
        var level=p.level();
        var config=p.getRespawnConfig();
        if(level.dimension()==Level.OVERWORLD&&config!=null&&config.respawnData().dimension()==Level.OVERWORLD){
            BlockPos bed=config.respawnData().pos();
            if(level.hasChunk(bed.getX()>>4,bed.getZ()>>4)&&BuiltInRegistries.BLOCK.getKey(level.getBlockState(bed).getBlock()).getPath().endsWith("_bed"))
                bedSpot[0]=new Vec3(bed.getX()+.5,bed.getY()+.6,bed.getZ()+.5);
        }
        if(bedSpot[0]!=null)return bedSpot[0];
        // stand-in: the original takes the stream's player spawn point; Minecraft's is a block, taken at its bottom centre.
        return Vec3.atBottomCenterOf(level.getRespawnData().pos());
    }

    private static void teleport(ServerPlayer p,Vec3 at){
        p.teleportTo(p.level(),at.x,at.y,at.z,Set.of(),p.getYRot(),p.getXRot(),false);
    }

    /** sim/src/voxel.rs solid_at: the block there has collision (null: not loaded). */
    static Boolean solid(Level level,Vec3 feet,double dy){
        BlockPos pos=BlockPos.containing(feet.x,feet.y+dy,feet.z);
        if(!level.hasChunk(pos.getX()>>4,pos.getZ()>>4))return null;
        if(pos.getY()<level.getMinY()||pos.getY()>level.getMaxY())return false;
        return !level.getBlockState(pos).getCollisionShape(level,pos).isEmpty();
    }

    /** sim/src/voxel.rs smothers_at: Minecraft's full-block suffocation shape there. */
    static Boolean smothers(Level level,Vec3 feet,double dy){
        BlockPos pos=BlockPos.containing(feet.x,feet.y+dy,feet.z);
        if(!level.hasChunk(pos.getX()>>4,pos.getZ()>>4))return null;
        if(pos.getY()<level.getMinY()||pos.getY()>level.getMaxY())return false;
        return Block.isShapeFullBlock(level.getBlockState(pos).getCollisionShape(level,pos));
    }

    /** minecraft_game/src/interaction.rs in_wall: the point of his eyes in a full block. */
    public static boolean eyesBuried(ServerPlayer p){
        return Boolean.TRUE.equals(smothers(p.level(),p.getEyePosition(),0));
    }

    enum Footing { SOLID, OPEN, HAZARD, UNKNOWN }
    /** minecraft_terrain/src/safe_spot.rs HAZARDS. */
    static final String[] HAZARDS={"water","lava","dark_eco","fire","magma","cactus","powder_snow","sweet_berry","leaves",
        "bubble_column","campfire","wither_rose","cobweb","pointed_dripstone"};

    /** minecraft_terrain/src/safe_spot.rs is_hazard. */
    static boolean isHazard(String path){for(String h:HAZARDS)if(path.contains(h))return true;return false;}

    /** render_anim/src/minecraft_world.rs footing_at. */
    static Footing footing(Level level,int x,int y,int z){
        if(!level.hasChunk(x>>4,z>>4))return Footing.UNKNOWN;
        if(y<level.getMinY()||y>level.getMaxY())return Footing.OPEN;
        BlockPos pos=new BlockPos(x,y,z);
        var state=level.getBlockState(pos);
        if(isHazard(BuiltInRegistries.BLOCK.getKey(state.getBlock()).getPath()))return Footing.HAZARD;
        return state.getCollisionShape(level,pos).isEmpty()?Footing.OPEN:Footing.SOLID;
    }

    /** minecraft_terrain/src/safe_spot.rs standable. */
    static boolean standable(Level level,int x,int y,int z){
        return footing(level,x,y-1,z)==Footing.SOLID&&footing(level,x,y,z)==Footing.OPEN&&footing(level,x,y+1,z)==Footing.OPEN;
    }

    /** minecraft_terrain/src/safe_spot.rs ring: the columns `r` steps out, nearest first. */
    static List<int[]> ring(int r){
        var out=new ArrayList<int[]>();
        if(r==0){out.add(new int[]{0,0});return out;}
        for(int d=-r;d<r;d++){out.add(new int[]{d,-r});out.add(new int[]{r,d});out.add(new int[]{-d,r});out.add(new int[]{-r,-d});}
        out.sort(Comparator.comparingInt(c->c[0]*c[0]+c[1]*c[1]));
        return out;
    }

    /** minecraft_terrain/src/safe_spot.rs safe_spot. */
    static int[] safeSpot(Level level,int x,int z,int minRing,int maxRing){
        for(int r=Math.max(minRing,0);r<=maxRing;r++)for(int[] d:ring(r)){
            int cx=x+d[0],cz=z+d[1];
            if(!level.hasChunk(cx>>4,cz>>4))continue;
            int y=level.getHeight(Heightmap.Types.MOTION_BLOCKING_NO_LEAVES,cx,cz);
            if(standable(level,cx,y,cz))return new int[]{cx,y,cz};
        }
        return null;
    }

    /** render_anim/src/minecraft_world.rs safe_near. */
    static Vec3 safeNear(Level level,Vec3 at,int minRing){
        int x=(int)Math.floor(at.x),y=(int)Math.floor(at.y),z=(int)Math.floor(at.z);
        int[] spot=level.dimension()==Level.OVERWORLD?safeSpot(level,x,z,minRing,16):standable(level,x,y,z)?new int[]{x,y,z}:null;
        return spot==null?null:new Vec3(spot[0]+.5,spot[1]+.05,spot[2]+.5);
    }
}
