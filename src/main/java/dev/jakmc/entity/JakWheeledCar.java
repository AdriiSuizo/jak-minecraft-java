package dev.jakmc.entity;

import com.google.gson.JsonArray;
import com.google.gson.JsonObject;
import com.google.gson.JsonParser;
import dev.jakmc.JakData;
import dev.jakmc.JakMod;
import dev.jakmc.nativebridge.NativeJak;
import dev.jakmc.nativebridge.WheeledCollision;
import dev.jakmc.network.JakPackets;
import dev.jakmc.player.JakPlayers;
import net.minecraft.core.BlockPos;
import net.minecraft.core.Direction;
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
import net.minecraft.world.InteractionHand;
import net.minecraft.world.InteractionResult;
import net.minecraft.world.damagesource.DamageSource;
import net.minecraft.world.entity.Entity;
import net.minecraft.world.entity.EntitySpawnReason;
import net.minecraft.world.entity.EntityType;
import net.minecraft.world.entity.LivingEntity;
import net.minecraft.world.entity.MobCategory;
import net.minecraft.world.entity.player.Player;
import net.minecraft.world.item.Item;
import net.minecraft.world.item.context.UseOnContext;
import net.minecraft.world.level.ClipContext;
import net.minecraft.world.level.Level;
import net.minecraft.world.level.storage.ValueInput;
import net.minecraft.world.level.storage.ValueOutput;
import net.minecraft.world.phys.HitResult;
import net.minecraft.world.phys.Vec3;

public final class JakWheeledCar extends Entity {
    public enum Model {
        TOAD("dune_hopper", "toad", 5.9f, 2.2f),
        SNAKE("sand_shark", "snake", 4.4f, 1.55f);

        public final String id, asset;
        final float length, height;
        public EntityType<JakWheeledCar> type;
        public Item item;

        Model(String id,String asset,float length,float height) {
            this.id=id;
            this.asset=asset;
            this.length=length;
            this.height=height;
        }
    }

    private static final EntityDataAccessor<String> FRAME=SynchedEntityData.defineId(
        JakWheeledCar.class,EntityDataSerializers.STRING);
    public final Model model;
    private long handle,retryAt,lastInput,boardingAt;
    private WheeledCollision collision;
    private Vec3 nativePosition, velocity=Vec3.ZERO, seat=new Vec3(0,.5,0), boardingFrom;
    private float boardingProgress=1;
    private float pitch,roll,health=40;
    private JakPackets.Input controls;
    private JsonObject currentFrame,previousFrame;
    private int press;
    private final WheeledWeapons weapons=new WheeledWeapons(this);

    private JakWheeledCar(EntityType<? extends JakWheeledCar> type,Level level,Model model) {
        super(type,level);
        this.model=model;
    }

    public static void initialize() {
        for(var model:Model.values()) {
            var type=ResourceKey.create(Registries.ENTITY_TYPE,JakData.id(model.id));
            model.type=Registry.register(BuiltInRegistries.ENTITY_TYPE,type,EntityType.Builder
                .<JakWheeledCar>of((entityType,level)->new JakWheeledCar(entityType,level,model),MobCategory.MISC)
                .sized(model.length,model.height).clientTrackingRange(12).updateInterval(1).build(type));
            var item=ResourceKey.create(Registries.ITEM,JakData.id(model.id));
            model.item=Registry.register(BuiltInRegistries.ITEM,item,new Item(new Item.Properties().setId(item).stacksTo(1)) {
                @Override public InteractionResult useOn(UseOnContext context) {
                    if(context.getClickedFace()!=Direction.UP)return InteractionResult.PASS;
                    if(context.getLevel() instanceof ServerLevel server) {
                        var car=model.type.create(server,EntitySpawnReason.SPAWN_ITEM_USE);
                        if(car==null)return InteractionResult.FAIL;
                        Vec3 point=Vec3.atBottomCenterOf(context.getClickedPos().above()).add(0,model.height*.65,0);
                        car.snapTo(point.x,point.y,point.z,context.getRotation(),0);
                        if(!server.noCollision(car)||!server.addFreshEntity(car))return InteractionResult.FAIL;
                        context.getItemInHand().consume(1,context.getPlayer());
                    }
                    return InteractionResult.SUCCESS;
                }
            });
        }
    }

    @Override protected void defineSynchedData(SynchedEntityData.Builder builder) {
        builder.define(FRAME,"");
    }
    @Override public void onSyncedDataUpdated(EntityDataAccessor<?> key) {
        super.onSyncedDataUpdated(key);
        if(level().isClientSide()&&FRAME.equals(key)&&!entityData.get(FRAME).isEmpty()){
            previousFrame=currentFrame;
            currentFrame=JsonParser.parseString(entityData.get(FRAME)).getAsJsonObject();
            seat=JakPlayers.vector(currentFrame.getAsJsonArray("seat"));
            boardingProgress=currentFrame.get("boardingProgress").getAsFloat();
            boardingFrom=currentFrame.has("boardingFrom")?JakPlayers.vector(currentFrame.getAsJsonArray("boardingFrom")):null;
        }
    }
    public JsonObject frame(){return currentFrame;}
    public JsonObject previousFrame(){return previousFrame==null?currentFrame:previousFrame;}
    @Override public boolean isPickable(){return !isRemoved();}
    @Override protected boolean canAddPassenger(Entity passenger){return getPassengers().isEmpty()&&passenger instanceof Player;}
    public boolean canBoard(Player player,Vec3 point) {
        return JakPlayers.active(player)&&getPassengers().isEmpty()&&!player.isSpectator()
            &&!player.isSleeping()&&player.getEyePosition().distanceToSqr(point)<=2.9*2.9;
    }
    @Override public InteractionResult interact(Player player,InteractionHand hand,Vec3 offset) {
        if(hand!=InteractionHand.MAIN_HAND||!canBoard(player,position().add(offset)))return InteractionResult.PASS;
        Vec3 approach=player.position();
        if(player instanceof ServerPlayer&&player.startRiding(this)){
            boardingFrom=approach;
            boardingProgress=0;
            controls=null;
            press=0;
            boardingAt=level().getGameTime();
            playCue(model.asset+"-start");
            return InteractionResult.SUCCESS_SERVER;
        }
        return level().isClientSide()?InteractionResult.SUCCESS:InteractionResult.PASS;
    }
    public void input(ServerPlayer player,JakPackets.Input input) {
        if(player.getVehicle()!=this||!player.getAttachedOrCreate(JakData.ENABLED))return;
        if(controls!=null&&input.sequence()<=controls.sequence())return;
        controls=input;
        lastInput=level().getGameTime();
        press|=input.pressed();
    }
    @Override protected void positionRider(Entity passenger,MoveFunction move) {
        Vec3 point=position().add(seat);
        if(boardingFrom!=null&&boardingProgress<1){
            double t=boardingProgress;
            point=boardingFrom.lerp(point,t*t*(3-2*t)).add(0,Math.sin(t*Math.PI)*.24,0);
        }
        move.accept(passenger,point.x,point.y,point.z);
    }
    @Override public Vec3 getDismountLocationForPassenger(LivingEntity passenger) {
        Vec3 side=Vec3.directionFromRotation(0,getYRot()+90).scale(model.length*.45);
        for(var offset:new Vec3[]{side,side.reverse(),new Vec3(0,model.height,0)}){
            Vec3 near=position().add(offset);
            if(!level().hasChunkAt(BlockPos.containing(near)))continue;
            var floor=level().clip(new ClipContext(near.add(0,.75,0),near.add(0,-3,0),
                ClipContext.Block.COLLIDER,ClipContext.Fluid.NONE,passenger));
            Vec3 point=floor.getType()==HitResult.Type.BLOCK?floor.getLocation().add(0,.001,0):near;
            if(level().noCollision(passenger,passenger.getBoundingBox().move(point.subtract(passenger.position()))))return point;
        }
        return super.getDismountLocationForPassenger(passenger);
    }

    @Override public void tick() {
        super.tick();
        if(!(level() instanceof ServerLevel server)||server.getGameTime()<retryAt)return;
        var rider=getFirstPassenger() instanceof ServerPlayer player?player:null;
        boolean fresh=rider!=null&&controls!=null&&server.getGameTime()-lastInput<7;
        int held=fresh?controls.held()|press:0;
        if(fresh&&(press&8)!=0&&server.getGameTime()-boardingAt>6){
            rider.stopRiding();
            playCue(model.asset+"-stop");
            rider=null;
            held=0;
            fresh=false;
        }
        press=0;
        if(rider==null){controls=null;boardingFrom=null;}
        boardingProgress=boardingFrom==null?1:Math.clamp((server.getGameTime()-boardingAt)/11f,0,1);
        if(rider==null&&tickCount>20&&velocity.lengthSqr()<.00001&&tickCount%10!=0)return;
        try {
            if(handle==0){
                nativePosition=position();
                collision=new WheeledCollision(server,this,nativePosition);
                handle=NativeJak.carCreate(collision,model.ordinal(),getYRot(),(float)velocity.x,
                    (float)velocity.y,(float)velocity.z,pitch,roll);
            }
            var at=position();
            var moved=at.subtract(collision.origin);
            collision.origin=at;
            int actions=held&7;
            if((held&64)!=0)actions|=8;
            String encoded=NativeJak.carStep(handle,fresh?-controls.strafe():0,fresh?controls.forward():0,
                fresh?controls.throttle():0,fresh?controls.brake():0,actions,
                (float)moved.x,(float)moved.y,(float)moved.z,3);
            var frame=JsonParser.parseString(encoded).getAsJsonObject();
            Vec3 step=JakPlayers.vector(frame.getAsJsonArray("position"));
            nativePosition=at.add(step);
            velocity=JakPlayers.vector(frame.getAsJsonArray("velocity"));
            seat=JakPlayers.vector(frame.getAsJsonArray("seat")).subtract(step);
            pitch=frame.get("pitch").getAsFloat();
            roll=frame.get("roll").getAsFloat();
            setYRot(frame.get("yaw").getAsFloat());
            setPos(nativePosition);
            if(rider!=null)positionRider(rider);
            var effects=new JsonArray();
            var shots=new JsonArray();
            weapons.step(server,effects,shots);
            for(var value:frame.remove("events").getAsJsonArray()){
                var event=value.getAsJsonObject();
                if(event.has("sound")&&!event.get("sound").isJsonNull())playCue(event.get("sound").getAsString());
                if(rider!=null&&event.has("fire")){
                    var q=frame.getAsJsonArray("rotation");
                    var attitude=new org.joml.Quaternionf(q.get(0).getAsFloat(),q.get(1).getAsFloat(),q.get(2).getAsFloat(),q.get(3).getAsFloat());
                    weapons.fire(server,rider,attitude,velocity,event.get("fire").getAsBoolean(),effects);
                }
            }
            frame.add("gunEffects",effects);
            frame.add("grenades",shots);
            frame.add("seat",jsonVector(seat));
            frame.addProperty("boardingProgress",boardingProgress);
            frame.addProperty("boarding",boardingProgress<1?boardingProgress*.55f:-1);
            if(boardingProgress<1)frame.add("boardingFrom",jsonVector(boardingFrom));
            entityData.set(FRAME,frame.toString());
        }catch(RuntimeException failure){
            closeNative();
            JakMod.LOG.error("Wheeled vehicle stopped at its last valid pose {}",getUUID(),failure);
            retryAt=server.getGameTime()+100;
        }
    }

    static JsonArray jsonVector(Vec3 point){
        var result=new JsonArray(3);
        result.add(point.x);
        result.add(point.y);
        result.add(point.z);
        return result;
    }
    private void playCue(String name){
        var cue=JakEnemySounds.find("jak3",name);
        if(cue!=null)level().playSound(null,getX(),getY(),getZ(),cue,SoundSource.NEUTRAL,1,1);
    }
    @Override public boolean hurtServer(ServerLevel server,DamageSource source,float amount){
        if(amount<=0||isInvulnerableToBase(source))return false;
        health-=amount;
        if(health<=0){
            ejectPassengers();
            playCue("car-explode");
            discard();
        }
        return true;
    }
    private void closeNative(){
        if(handle!=0){NativeJak.carDestroy(handle);handle=0;}
        collision=null;
    }
    @Override public void remove(RemovalReason reason){closeNative();super.remove(reason);}
    @Override protected void readAdditionalSaveData(ValueInput in){
        velocity=new Vec3(in.getDoubleOr("VelocityX",0),in.getDoubleOr("VelocityY",0),in.getDoubleOr("VelocityZ",0));
        pitch=in.getFloatOr("Pitch",0);
        roll=in.getFloatOr("Roll",0);
        health=in.getFloatOr("Health",40);
    }
    @Override protected void addAdditionalSaveData(ValueOutput out){
        out.putDouble("VelocityX",velocity.x);
        out.putDouble("VelocityY",velocity.y);
        out.putDouble("VelocityZ",velocity.z);
        out.putFloat("Pitch",pitch);
        out.putFloat("Roll",roll);
        out.putFloat("Health",health);
    }
}
