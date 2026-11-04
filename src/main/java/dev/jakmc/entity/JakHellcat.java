package dev.jakmc.entity;

import com.google.gson.JsonObject;
import com.google.gson.JsonParser;
import dev.jakmc.JakData;
import dev.jakmc.JakMod;
import dev.jakmc.nativebridge.DirectBufferArena;
import dev.jakmc.nativebridge.MinecraftCollision;
import dev.jakmc.nativebridge.NativeJak;
import dev.jakmc.network.JakPackets;
import dev.jakmc.player.JakPlayers;
import java.nio.ByteBuffer;
import net.minecraft.core.BlockPos;
import net.minecraft.core.Registry;
import net.minecraft.core.registries.BuiltInRegistries;
import net.minecraft.core.registries.Registries;
import net.minecraft.network.syncher.EntityDataAccessor;
import net.minecraft.network.syncher.EntityDataSerializers;
import net.minecraft.network.syncher.SynchedEntityData;
import net.minecraft.resources.ResourceKey;
import net.minecraft.server.level.ServerLevel;
import net.minecraft.server.level.ServerPlayer;
import net.minecraft.util.Mth;
import net.minecraft.world.InteractionHand;
import net.minecraft.world.InteractionResult;
import net.minecraft.world.damagesource.DamageSource;
import net.minecraft.world.entity.Entity;
import net.minecraft.world.entity.EntitySpawnReason;
import net.minecraft.world.entity.EntityType;
import net.minecraft.world.entity.MobCategory;
import net.minecraft.world.entity.player.Player;
import net.minecraft.world.item.Item;
import net.minecraft.world.item.context.UseOnContext;
import net.minecraft.world.level.Level;
import net.minecraft.world.level.storage.ValueInput;
import net.minecraft.world.level.storage.ValueOutput;
import net.minecraft.world.phys.Vec3;
import net.minecraft.world.phys.AABB;
import org.joml.Quaternionf;

public final class JakHellcat extends Entity {
    private static final EntityDataAccessor<Integer> PILOT=SynchedEntityData.defineId(JakHellcat.class,EntityDataSerializers.INT);
    private static final EntityDataAccessor<org.joml.Quaternionfc> ATTITUDE=SynchedEntityData.defineId(JakHellcat.class,EntityDataSerializers.QUATERNION);
    private static final EntityDataAccessor<Float> LAMPS=SynchedEntityData.defineId(JakHellcat.class,EntityDataSerializers.FLOAT);
    private static final EntityDataAccessor<Float> THRUST=SynchedEntityData.defineId(JakHellcat.class,EntityDataSerializers.FLOAT);
    private static final EntityDataAccessor<Boolean> WRECK=SynchedEntityData.defineId(JakHellcat.class,EntityDataSerializers.BOOLEAN);
    private static final EntityDataAccessor<Float> ENGINE=SynchedEntityData.defineId(JakHellcat.class,EntityDataSerializers.FLOAT);
    private static final EntityDataAccessor<Float> WIND=SynchedEntityData.defineId(JakHellcat.class,EntityDataSerializers.FLOAT);
    private static final EntityDataAccessor<Float> HEALTH=SynchedEntityData.defineId(JakHellcat.class,EntityDataSerializers.FLOAT);
    private static final EntityDataAccessor<String> BEAMS=SynchedEntityData.defineId(JakHellcat.class,EntityDataSerializers.STRING);
    public static EntityType<JakHellcat> TYPE,DOG_TYPE;
    public static Item ITEM,DOG_ITEM;

    private long handle,retryAfter;
    private MinecraftCollision.Query collision;
    private final ByteBuffer controls=NativeJak.buffer(32),noActors=NativeJak.buffer(0);
    private final DirectBufferArena actorBuffers=new DirectBufferArena(512*1024);
    private Vec3 origin;
    private Vec3 velocity=Vec3.ZERO;
    private float health=1;
    private int flags,commands,torpedoes=8,wreckTicks;
    private boolean shots;

    public JakHellcat(EntityType<? extends JakHellcat> type,Level level){super(type,level);}

    public static void initialize(){
        TYPE=registerType("hellcat");
        DOG_TYPE=registerType("helldog");
        ITEM=registerItem("hellcat",TYPE);
        DOG_ITEM=registerItem("helldog",DOG_TYPE);
    }
    private static EntityType<JakHellcat> registerType(String name){
        var key=ResourceKey.create(Registries.ENTITY_TYPE,JakData.id(name));
        return Registry.register(BuiltInRegistries.ENTITY_TYPE,key,EntityType.Builder
            .of(JakHellcat::new,MobCategory.MISC).sized(4.1f,2.05f).clientTrackingRange(12).updateInterval(1).build(key));
    }
    private static Item registerItem(String name,EntityType<JakHellcat> type){
        var item=ResourceKey.create(Registries.ITEM,JakData.id(name));
        return Registry.register(BuiltInRegistries.ITEM,item,new Item(new Item.Properties().setId(item).stacksTo(1)){
            @Override public InteractionResult useOn(UseOnContext context){
                if(!context.getClickedFace().equals(net.minecraft.core.Direction.UP))return InteractionResult.PASS;
                if(context.getLevel() instanceof ServerLevel server){
                    var craft=type.create(server,EntitySpawnReason.SPAWN_ITEM_USE);
                    if(craft==null)return InteractionResult.FAIL;
                    Vec3 point=Vec3.atBottomCenterOf(context.getClickedPos().above()).add(0,1.28,0);
                    craft.snapTo(point.x,point.y,point.z,context.getRotation(),0);
                    if(!server.noCollision(craft)||!server.addFreshEntity(craft))return InteractionResult.FAIL;
                    context.getItemInHand().consume(1,context.getPlayer());
                }
                return InteractionResult.SUCCESS;
            }
        });
    }

    @Override protected void defineSynchedData(SynchedEntityData.Builder builder){
        builder.define(PILOT,-1);
        builder.define(ATTITUDE,new Quaternionf());
        builder.define(LAMPS,0f);
        builder.define(THRUST,0f);
        builder.define(WRECK,false);
        builder.define(ENGINE,0f);
        builder.define(WIND,0f);
        builder.define(HEALTH,1f);
        builder.define(BEAMS,"");
    }
    public boolean hellDog(){return getType()==DOG_TYPE;}
    public int pilotId(){return entityData.get(PILOT);}
    public org.joml.Quaternionfc attitude(){return entityData.get(ATTITUDE);}
    public float lamps(){return entityData.get(LAMPS);}
    public float thrust(){return entityData.get(THRUST);}
    public float engine(){return entityData.get(ENGINE);}
    public float wind(){return entityData.get(WIND);}
    public float hullHealth(){return entityData.get(HEALTH);}
    public String beamData(){return entityData.get(BEAMS);}
    public boolean wrecked(){return entityData.get(WRECK);}
    @Override protected AABB makeBoundingBox(Vec3 position){
        return HellcatDeck.bounds(position,getYRot(),hellDog());
    }
    @Override public boolean isPickable(){return !isRemoved()&&!wrecked();}
    public boolean canBoard(Player player,Vec3 hit){
        return isPickable()&&pilotId()<0&&player.getEyePosition().distanceToSqr(hit)<=2.9*2.9
            &&player.level()==level()&&!player.isSpectator();
    }
    @Override public InteractionResult interact(Player player,InteractionHand hand,Vec3 offset){
        if(hand!=InteractionHand.MAIN_HAND||!canBoard(player,position().add(offset))||!JakPlayers.active(player)){
            return InteractionResult.PASS;
        }
        if(player instanceof ServerPlayer server){
            return JakPlayers.boardHellcat(server,this)?InteractionResult.SUCCESS_SERVER:InteractionResult.PASS;
        }
        return InteractionResult.SUCCESS;
    }

    public void reserve(ServerPlayer player){entityData.set(PILOT,player.getId());}
    public void release(){
        entityData.set(PILOT,-1);
        commands|=2;
    }
    @Override public boolean hurtServer(ServerLevel server,DamageSource source,float amount){
        if(amount<=0||wrecked()||isInvulnerableToBase(source))return false;
        commands|=4;
        return true;
    }

    @Override public void tick(){
        super.tick();
        if(!(level() instanceof ServerLevel server)||pilotId()>=0||server.getGameTime()<retryAfter)return;
        try{
            advance(server,null,null,0,false);
        }catch(RuntimeException failure){
            close();
            JakMod.LOG.error("HellCat stopped at its last safe pose {}",getUUID(),failure);
            retryAfter=server.getGameTime()+100;
        }
    }

    private void open(ServerLevel server){
        if(handle!=0)return;
        origin=position();
        collision=new MinecraftCollision.Query(server,this,origin);
        handle=NativeJak.hellcatCreate(collision,getYRot(),(float)velocity.x,(float)velocity.y,
            (float)velocity.z,health,flags,torpedoes,getUUID().getLeastSignificantBits());
    }
    private void close(){
        if(handle!=0){NativeJak.hellcatDestroy(handle);handle=0;}
        collision=null;
    }
    @Override public void onRemoval(RemovalReason reason){close();super.onRemoval(reason);}

    public JsonObject advance(ServerLevel server,ServerPlayer pilot,JakPackets.Input input,int edges,boolean start){
        open(server);
        Vec3 shift=Vec3.ZERO;
        if(position().distanceToSqr(origin)>64*64){
            shift=position().subtract(origin);
            origin=position();
            collision.origin=origin;
        }
        boolean ready=true;
        // Await collision columns before stepping. A visible chunk can still
        // be beyond the entity-ticking frontier when the ship is boosting.
        var test=getBoundingBox().expandTowards(velocity.scale(.05)).inflate(1);
        for(int cx=Mth.floor(test.minX)>>4;cx<=Mth.floor(test.maxX)>>4;cx++){
            for(int cz=Mth.floor(test.minZ)>>4;cz<=Mth.floor(test.maxZ)>>4;cz++){
                if(!server.isPositionEntityTicking(new BlockPos(cx<<4,blockPosition().getY(),cz<<4)))ready=false;
            }
        }
        controls.clear();
        controls.putInt(input==null?0:input.held()).putInt(edges);
        controls.putFloat(input==null?0:input.strafe()).putFloat(input==null?0:input.forward());
        controls.putFloat(0).putFloat(0);
        controls.putFloat(input==null?0:input.brake()).putFloat(input==null?0:input.throttle());
        actorBuffers.reset();
        boolean aiming=pilot!=null&&(edges&64)!=0;
        boolean firing=pilot!=null&&(edges&16)!=0;
        ByteBuffer actors=aiming||firing||shots?MinecraftCollision.actors(server,this,origin,
            getBoundingBox().inflate(aiming?260:110),actorBuffers,e->e!=pilot):noActors;
        float hour=(server.getDefaultClockTime()%24000+6000)%24000/1000f;
        var frame=JsonParser.parseString(NativeJak.hellcatStep(handle,controls,actors,
            (float)shift.x,(float)shift.y,(float)shift.z,hour,commands|(start?1:0),ready?3:0)).getAsJsonObject();
        commands=0;
        if(start)sound("vehicl-ignition",position());
        Vec3 destination=JakPlayers.vector(frame.getAsJsonArray("position")).add(origin);
        if(!Double.isFinite(destination.x+destination.y+destination.z))throw new IllegalStateException("Invalid HellCat pose");
        velocity=JakPlayers.vector(frame.getAsJsonArray("velocity"));
        var q=frame.getAsJsonArray("rotation");
        entityData.set(ATTITUDE,new Quaternionf(q.get(0).getAsFloat(),q.get(1).getAsFloat(),q.get(2).getAsFloat(),q.get(3).getAsFloat()));
        Vec3 heading=JakPlayers.vector(frame.getAsJsonArray("heading"));
        if(heading.horizontalDistanceSqr()>1e-6)setYRot((float)-Math.toDegrees(Math.atan2(heading.x,heading.z)));
        setPos(destination);
        health=frame.get("health").getAsFloat();
        flags=frame.get("flags").getAsInt();
        torpedoes=frame.get("torpedoes").getAsInt();
        entityData.set(ENGINE,frame.get("engine").getAsFloat());
        entityData.set(WIND,frame.get("wind").getAsFloat());
        entityData.set(HEALTH,health);
        if(frame.get("lampsPopped").getAsBoolean())sound("headlight-pop",position());
        entityData.set(LAMPS,frame.get("lamps").getAsFloat());
        entityData.set(THRUST,frame.get("boost").getAsBoolean()?1:
            Math.max(.5f*frame.get("engine").getAsFloat(),(flags&2)!=0||frame.get("pilot").getAsBoolean()?.08f:0));
        entityData.set(WRECK,frame.get("destroyed").getAsBoolean());
        var beams=frame.getAsJsonArray("beams");
        shots=!beams.isEmpty();
        entityData.set(BEAMS,shots?packBeams(beams):"");
        if(wrecked()&&++wreckTicks>60)discard();

        for(var event:frame.getAsJsonArray("events"))applyEvent(server,pilot,event.getAsJsonObject());
        frame.add("seat",vectorArray(JakPlayers.vector(frame.getAsJsonArray("seat")).add(origin)));
        return frame;
    }
    private String packBeams(com.google.gson.JsonArray beams){
        int count=Math.min(400,beams.size());
        var data=ByteBuffer.allocate(24+count*32);
        data.putDouble(origin.x).putDouble(origin.y).putDouble(origin.z);
        for(int i=0;i<count;i++){
            var beam=beams.get(i).getAsJsonObject();
            for(String key:new String[]{"head","tail"})for(var component:beam.getAsJsonArray(key)){
                data.putFloat(component.getAsFloat());
            }
            int color=0;
            for(var component:beam.getAsJsonArray("rgba"))color=color<<8|Math.clamp(Math.round(component.getAsFloat()*255),0,255);
            data.putInt(color).putFloat(beam.get("width").getAsFloat());
        }
        return java.util.Base64.getEncoder().encodeToString(data.array());
    }

    private static com.google.gson.JsonArray vectorArray(Vec3 v){
        var array=new com.google.gson.JsonArray();
        array.add(v.x);array.add(v.y);array.add(v.z);
        return array;
    }
    private void applyEvent(ServerLevel level,ServerPlayer pilot,JsonObject event){
        String kind=event.get("kind").getAsString();
        Vec3 at=JakPlayers.vector(event.getAsJsonArray("pos")).add(origin);
        switch(kind){
            case "impact" -> {
                if(event.get("actor").isJsonNull())return;
                var target=level.getEntity(event.get("actor").getAsInt());
                if(target!=null&&target!=pilot&&target!=this){
                    var damage=pilot!=null?dev.jakmc.player.JakDamage.shot(pilot,false):damageSources().magic();
                    target.hurtServer(level,damage,event.get("damage").getAsFloat()*2.5f);
                }
            }
            case "fire" -> sound("hellcat-fire",at);
            case "fire_alt" -> sound("hellcat-firealt",at);
            case "crash" -> sound("fac-impact-hard",at);
            case "blast","explode" -> sound("hellcat-explode",at);
        }
    }
    private void sound(String name,Vec3 at){
        var cue=JakEnemySounds.find("jak3",name);
        if(cue!=null)level().playSound(null,at.x,at.y,at.z,cue,net.minecraft.sounds.SoundSource.NEUTRAL,1,1);
    }
    public static int deckCapacity(){return HellcatDeck.maximumBytes();}
    public void appendDeck(Vec3 origin,net.minecraft.world.phys.AABB region,ByteBuffer output){
        HellcatDeck.append(this,origin,region,output);
    }
    public Vec3 exit(ServerLevel level){
        Vec3 seat=HellcatDeck.exit(this);
        var bounds=new AABB(-.32,0,-.32,.32,1.8,.32);
        Vec3 side=Vec3.directionFromRotation(0,getYRot()+90).scale(2.35);
        for(Vec3 point:new Vec3[]{seat,seat.add(0,.9,0),seat.add(side),seat.subtract(side)}){
            if(level.noCollision(bounds.move(point)))return point;
        }
        return null;
    }

    @Override protected void addAdditionalSaveData(ValueOutput out){
        out.putFloat("HullHealth",health);
        out.putInt("HullFlags",flags);
        out.putInt("Torpedoes",torpedoes);
        out.putInt("WreckTicks",wreckTicks);
        out.putDouble("FlightX",velocity.x);
        out.putDouble("FlightY",velocity.y);
        out.putDouble("FlightZ",velocity.z);
    }
    @Override protected void readAdditionalSaveData(ValueInput in){
        health=Math.clamp(in.getFloatOr("HullHealth",1),-1,1);
        flags=in.getIntOr("HullFlags",0)&7;
        torpedoes=Math.clamp(in.getIntOr("Torpedoes",8),0,8);
        wreckTicks=Math.clamp(in.getIntOr("WreckTicks",0),0,60);
        velocity=new Vec3(in.getDoubleOr("FlightX",0),in.getDoubleOr("FlightY",0),in.getDoubleOr("FlightZ",0));
        entityData.set(WRECK,health<0);
    }
}
