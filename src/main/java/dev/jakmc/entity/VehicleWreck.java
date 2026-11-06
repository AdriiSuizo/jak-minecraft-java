package dev.jakmc.entity;

import dev.jakmc.JakData;
import java.util.ArrayDeque;
import net.minecraft.core.Registry;
import net.minecraft.core.registries.BuiltInRegistries;
import net.minecraft.core.registries.Registries;
import net.minecraft.network.syncher.EntityDataAccessor;
import net.minecraft.network.syncher.EntityDataSerializers;
import net.minecraft.network.syncher.SynchedEntityData;
import net.minecraft.resources.ResourceKey;
import net.minecraft.server.level.ServerLevel;
import net.minecraft.server.level.ServerPlayer;
import net.minecraft.sounds.SoundSource;
import net.minecraft.world.damagesource.DamageSource;
import net.minecraft.world.entity.Entity;
import net.minecraft.world.entity.EntitySpawnReason;
import net.minecraft.world.entity.EntityType;
import net.minecraft.world.entity.MobCategory;
import net.minecraft.world.level.ClipContext;
import net.minecraft.world.level.Level;
import net.minecraft.world.level.storage.ValueInput;
import net.minecraft.world.level.storage.ValueOutput;
import net.minecraft.world.phys.HitResult;
import net.minecraft.world.phys.Vec3;
import net.minecraft.world.phys.shapes.CollisionContext;
import org.joml.Quaternionf;
import org.joml.Quaternionfc;
import org.joml.Vector3f;

/**
 * A vehicle blown up (Rust render_anim zoomers.rs, 16/10): its charred
 * carcass tumbling from `explode`'s impulse until no one has seen it for 5 s
 * (or 120 s, or at once 200 m from every player), and the `joint-exploder`'s
 * pieces of the `vehicle-explosion` model flying 4 s (drawn on the client from
 * the seed). A HellCat leaves only the pieces: its own hull is its carcass.
 */
public final class VehicleWreck extends Entity {
    /** Blocks a metre: Jak's 4096 units against the block's 6400. */
    public static final double METRE=.64;
    static final float WRECK_UNSEEN_SECONDS=5,WRECK_FAR=200,GRAVITY=40;
    public static final float DEBRIS_SECONDS=4;
    static final int DEBRIS_MAX=8;
    private static final EntityDataAccessor<Integer> MODEL=SynchedEntityData.defineId(VehicleWreck.class,EntityDataSerializers.INT);
    private static final EntityDataAccessor<Long> MASK=SynchedEntityData.defineId(VehicleWreck.class,EntityDataSerializers.LONG);
    private static final EntityDataAccessor<Quaternionfc> BASE=SynchedEntityData.defineId(VehicleWreck.class,EntityDataSerializers.QUATERNION);
    private static final EntityDataAccessor<Quaternionfc> TURN=SynchedEntityData.defineId(VehicleWreck.class,EntityDataSerializers.QUATERNION);
    private static final EntityDataAccessor<Integer> SEED=SynchedEntityData.defineId(VehicleWreck.class,EntityDataSerializers.INT);
    private static final EntityDataAccessor<Float> YAW=SynchedEntityData.defineId(VehicleWreck.class,EntityDataSerializers.FLOAT);
    private static final EntityDataAccessor<Boolean> CAR=SynchedEntityData.defineId(VehicleWreck.class,EntityDataSerializers.BOOLEAN);
    private static final EntityDataAccessor<org.joml.Vector3fc> ORIGIN=SynchedEntityData.defineId(VehicleWreck.class,EntityDataSerializers.VECTOR3);
    /** zoomers.rs draw: wheeled::draw_wheels on a wreck too, its wheels as they last were (the car's frame "wheels"). */
    private static final EntityDataAccessor<String> WHEELS=SynchedEntityData.defineId(VehicleWreck.class,EntityDataSerializers.STRING);
    public static EntityType<VehicleWreck> TYPE;
    private static final ArrayDeque<VehicleWreck> DEBRIS=new ArrayDeque<>();
    /** m/s and rad/s, world axes. */
    private Vec3 vel=Vec3.ZERO;
    private Vector3f spin=new Vector3f();
    private final Quaternionf turn=new Quaternionf();
    private float age,unseen;

    public VehicleWreck(EntityType<? extends VehicleWreck> type,Level level){super(type,level);noPhysics=true;}

    public static void initialize(){
        var key=ResourceKey.create(Registries.ENTITY_TYPE,JakData.id("vehicle_wreck"));
        TYPE=Registry.register(BuiltInRegistries.ENTITY_TYPE,key,EntityType.Builder.<VehicleWreck>of(VehicleWreck::new,MobCategory.MISC)
            .sized(1,1).clientTrackingRange(12).updateInterval(1).noSave().build(key));
    }
    @Override protected void defineSynchedData(SynchedEntityData.Builder builder){
        builder.define(MODEL,-1);builder.define(MASK,-1L);
        builder.define(BASE,new Quaternionf());builder.define(TURN,new Quaternionf());
        builder.define(SEED,0);builder.define(YAW,0f);builder.define(CAR,true);builder.define(ORIGIN,new Vector3f());builder.define(WHEELS,"");
    }
    public int model(){return entityData.get(MODEL);}
    public long mask(){return entityData.get(MASK);}
    public Quaternionf carcass(){return new Quaternionf(entityData.get(TURN)).mul(entityData.get(BASE));}
    public int seed(){return entityData.get(SEED);}
    /** The pieces' heading, radians about the world's up. */
    public float yaw(){return entityData.get(YAW);}
    public boolean car(){return entityData.get(CAR);}
    public String wheels(){return entityData.get(WHEELS);}
    public Vec3 origin(){var o=entityData.get(ORIGIN);return new Vec3(o.x(),o.y(),o.z());}

    /** Rust zoomers.rs rand_signed: a random number in -1..1 (a small LCG). */
    static float randSigned(int[] seed){
        seed[0]=seed[0]*1_664_525+1_013_904_223;
        return (seed[0]>>>8)/(float)(1<<24)*2-1;
    }
    /**
     * Rust zoomers.rs wreck_spin: a wreck's tumble from `explode`'s impulse
     * (163840 up at `offset` metres off its middle, its own frame: x left,
     * y up, z ahead) through `inertial-tensor-box` and mass (car 8, 4 x 3 x
     * 6 m; bike 2, 3 x 4 x 6 m): rad/s, its own frame.
     */
    static Vector3f wreckSpin(boolean car,Vector3f offset){
        float mass=car?8:2,w=car?4:3,h=car?3:4,l=6;
        var r=new Vector3f(offset).mul(4096);
        var torque=r.cross(new Vector3f(0,163_840,0),new Vector3f());
        float metre2=4096f*4096f;
        var inertia=new Vector3f(h*h+l*l,w*w+l*l,w*w+h*h).mul(mass/12*metre2);
        return torque.div(inertia);
    }

    /**
     * Rust zoomers.rs Fleet step, "No hit points left: it blows up": up at
     * 5 m/s (a car) or 20 (a bike), tumbling, its explosion and pieces, Jak
     * near hurt (25 within 7.5 m of a car, 5.6 of a bike), its rider thrown
     * 2.5 m to its side.
     */
    static void zoomer(ServerLevel level,JakVehicle vehicle,Quaternionfc attitude,long mask,String wheels){
        var wreck=TYPE.create(level,EntitySpawnReason.EVENT);
        if(wreck==null)return;
        boolean car=!vehicle.model.bike();
        float yaw=(float)-Math.toRadians(vehicle.getYRot());
        // stand-in: the seed's clock and fleet index are the game time and entity id.
        int[] seed={(int)(level.getGameTime()*50)^vehicle.getId()*(int)2_654_435_761L|1};
        var offset=new Vector3f(randSigned(seed),randSigned(seed),randSigned(seed));
        var heading=new Quaternionf().rotationY(yaw);
        wreck.spin=heading.transform(wreckSpin(car,offset));
        wreck.vel=new Vec3(0,car?5:20,0);
        Vec3 at=vehicle.position();
        wreck.snapTo(at.x,at.y,at.z,vehicle.getYRot(),0);
        wreck.entityData.set(MODEL,vehicle.model.ordinal());
        wreck.entityData.set(MASK,mask);
        wreck.entityData.set(WHEELS,wheels);
        wreck.entityData.set(BASE,new Quaternionf(attitude));
        wreck.pieces(at,yaw,car,seed[0]);
        level.addFreshEntity(wreck);
        sound(level,"vehicle-explode",at);
        sound(level,"vehicle-explo-b",at);
        double reach=(car?7.5:5.6)*METRE;
        // MW2's 25 of its 100 health: Minecraft's 5 of 20.
        for(var player:level.getEntitiesOfClass(ServerPlayer.class,vehicle.getBoundingBox().inflate(reach+4)))
            if(player.position().distanceTo(at)<reach&&!player.isSpectator())
                player.hurtServer(level,level.damageSources().explosion(wreck,null),25*20/100f);
    }
    /** Rust zoomers.rs explode_at: a vehicle blown up elsewhere (the HellCat), its explosion and its pieces. */
    static void elsewhere(ServerLevel level,Vec3 at,float yaw,boolean car){
        var wreck=TYPE.create(level,EntitySpawnReason.EVENT);
        if(wreck==null)return;
        // stand-in: the seed's map point is the block point times 36 map units.
        float mixed=(float)(at.x*36*13+at.z*36*7);
        int seed=(mixed<=0?0:mixed>=4294967295f?-1:(int)(long)mixed)|1;
        wreck.snapTo(at.x,at.y,at.z,0,0);
        wreck.pieces(at,yaw,car,seed);
        level.addFreshEntity(wreck);
    }
    private void pieces(Vec3 at,float yaw,boolean car,int seed){
        entityData.set(SEED,seed);entityData.set(YAW,yaw);entityData.set(CAR,car);
        entityData.set(ORIGIN,new Vector3f((float)at.x,(float)at.y,(float)at.z));
        if(DEBRIS.size()>=DEBRIS_MAX){var oldest=DEBRIS.removeFirst();oldest.entityData.set(SEED,0);}
        DEBRIS.addLast(this);
    }
    private static void sound(ServerLevel level,String name,Vec3 at){
        var cue=JakEnemySounds.find("jak3",name);
        if(cue!=null)level.playSound(null,at.x,at.y,at.z,cue,SoundSource.NEUTRAL,1,1);
    }

    @Override public void tick(){
        super.tick();
        if(!(level() instanceof ServerLevel server))return;
        float dt=.05f;
        age+=dt;
        if(model()>=0)carcassStep(server,dt);
        boolean carcassGone=model()<0||unseen>=WRECK_UNSEEN_SECONDS||age>120;
        if(carcassGone&&model()>=0)entityData.set(MODEL,-1);
        if(carcassGone&&(seed()==0||age>=DEBRIS_SECONDS))discard();
    }
    /**
     * Rust zoomers.rs Fleet step, "Wrecks fall, tumbling": `linear-damping`
     * 0.99 and `angular-damping` 0.97 a frame, Jak 3's gravity, the ground's
     * `bounce-factor` 0.4.
     */
    private void carcassStep(ServerLevel server,float dt){
        float frames=dt*60;
        vel=vel.scale(Math.pow(.99,frames));
        spin.mul((float)Math.pow(.97,frames));
        var step=new Vector3f(spin).mul(dt);
        float angle=step.length();
        if(angle>1e-9f)turn.premul(new Quaternionf().rotationAxis(angle,step.x/angle,step.y/angle,step.z/angle)).normalize();
        vel=vel.add(0,-GRAVITY*dt,0);
        Vec3 pos=position().add(vel.scale(dt*METRE));
        Vec3 probe=pos.add(0,METRE,0);
        var hit=server.clip(new ClipContext(probe,probe.add(0,-3*METRE,0),ClipContext.Block.COLLIDER,ClipContext.Fluid.NONE,CollisionContext.empty()));
        if(hit.getType()==HitResult.Type.BLOCK){
            double h=(probe.y-hit.getLocation().y)/METRE;
            if(h<1){
                pos=pos.add(0,(1-h)*METRE,0);
                vel=new Vec3(vel.x*.8,vel.y<-2?-vel.y*.4:0,vel.z*.8);
                spin.mul(.6f);
            }
        }
        setPos(pos);
        entityData.set(TURN,new Quaternionf(turn));
        boolean far=true,seen=false;
        for(var player:server.players()){
            if(player.position().distanceTo(pos)<=WRECK_FAR*METRE)far=false;
            seen|=seenBy(player,pos);
        }
        unseen=far?WRECK_UNSEEN_SECONDS:seen?0:unseen+dt;
    }
    /**
     * stand-in: the original tests its middle against the camera's side
     * planes widened 5 m; the server has no camera, so this is the player's
     * eye and view with vanilla's default 70 degree field at 16:9.
     */
    private static boolean seenBy(ServerPlayer player,Vec3 at){
        Vec3 eye=player.getEyePosition(),to=at.subtract(eye),f=player.getViewVector(1);
        double r=5*METRE;
        Vec3 right=f.cross(new Vec3(0,1,0));
        if(right.lengthSqr()<1e-6)right=new Vec3(1,0,0);
        right=right.normalize();
        Vec3 up=right.cross(f).normalize();
        double ahead=to.dot(f),tanV=Math.tan(Math.toRadians(35)),tanH=tanV*16/9;
        return ahead>-r&&Math.abs(to.dot(right))<=ahead*tanH+r*Math.sqrt(1+tanH*tanH)
            &&Math.abs(to.dot(up))<=ahead*tanV+r*Math.sqrt(1+tanV*tanV);
    }
    @Override public void remove(RemovalReason reason){DEBRIS.remove(this);super.remove(reason);}
    @Override public boolean isPickable(){return false;}
    @Override public boolean shouldBeSaved(){return false;}
    @Override public boolean hurtServer(ServerLevel level,DamageSource source,float amount){return false;}
    @Override protected void readAdditionalSaveData(ValueInput in){}
    @Override protected void addAdditionalSaveData(ValueOutput out){}
}
