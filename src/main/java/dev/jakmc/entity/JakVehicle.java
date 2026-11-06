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

public final class JakVehicle extends Entity {
    public enum Model {
        TOAD("dune_hopper", "toad", "wasall", 5.9f, 2.2f, 40),
        SNAKE("sand_shark", "snake", "wasall", 4.4f, 1.55f, 40),
        BIKEA("zoomer_bike_a", "bikea", "ctycarb", 2.0f, 1.15f, 15),
        BIKEB("zoomer_bike_b", "bikeb", "ctycarb", 2.0f, 1.15f, 15),
        BIKEC("zoomer_bike_c", "bikec", "ctycarb", 2.0f, 1.15f, 15),
        CARA("zoomer_car_a", "cara", "ctycara", 3.2f, 1.15f, 24),
        CARB("zoomer_car_b", "carb", "ctycara", 3.2f, 1.15f, 24),
        CARC("zoomer_car_c", "carc", "ctycara", 3.2f, 1.15f, 24);

        public final String id,asset,folder;
        final float length,height,hitPoints;
        public EntityType<JakVehicle> type;
        public Item item;

        Model(String id,String asset,String folder,float length,float height,float hitPoints) {
            this.id=id;
            this.asset=asset;
            this.folder=folder;
            this.hitPoints=hitPoints;
            this.length=length;
            this.height=height;
        }
        public boolean hover(){return ordinal()>=BIKEA.ordinal();}
        public boolean bike(){return hover()&&ordinal()<CARA.ordinal();}
        public double parkingClearance(){return hover()?(bike()?2.3:2.5)*.64+.5:height*.65;}
    }

    private static final EntityDataAccessor<String> FRAME=SynchedEntityData.defineId(
        JakVehicle.class,EntityDataSerializers.STRING);
    public final Model model;
    private long handle,retryAt,lastInput,boardingAt;
    private WheeledCollision collision;
    private Vec3 nativePosition, velocity=Vec3.ZERO, seat=new Vec3(0,.5,0), boardingFrom;
    private float boardingProgress=1;
    private float pitch,roll,health;
    private int strikes;
    /** A town's traffic: the ring it drives round, none once a player rides it (Rust zoomers.rs route). */
    ZoomerTraffic.Route route;
    private long lastStrike=Long.MIN_VALUE/2;
    /** Ticks between blows that still count as in a row (Rust zoomers STRIKE_GAP 1.6 s). */
    private static final int STRIKE_GAP=32;
    private boolean high;
    private JakPackets.Input controls;
    private JsonObject currentFrame,previousFrame;
    private final float[] trafficNote=new float[4];
    String trafficFrame(){return String.format("steer%.2f thr%.0f brk%.0f v%.2f",trafficNote[0],trafficNote[1],trafficNote[2],trafficNote[3]);}
    private int press;
    // The hull, suspension, rider and camera sample these same timestamped snapshots.
    private record DisplayFrame(long tick,JsonObject frame){}
    private final java.util.ArrayDeque<DisplayFrame> displayFrames=new java.util.ArrayDeque<>();
    private JsonObject displayBefore,displayAfter;
    private long frameReceived,displayTime;
    private double displayClock,displayInterval=50_000_000;
    private float displayFraction=1;
    private Vec3 displayPosition;

    public void samplePresentation(long now){
        if(displayFrames.isEmpty())return;
        var latest=displayFrames.getLast();
        double dt=Math.max(0,(now-displayTime)/displayInterval);
        displayTime=now;
        double target=latest.tick-2+Math.clamp((now-frameReceived)/displayInterval,0,1);
        double predicted=displayClock+dt;
        displayClock=Math.min(latest.tick,predicted+Math.clamp((target-predicted)*.1,-.1,.1)*dt);
        var left=displayFrames.getFirst();
        var right=left;
        for(var candidate:displayFrames){
            right=candidate;
            if(candidate.tick>=displayClock)break;
            left=candidate;
        }
        displayBefore=left.frame;displayAfter=right.frame;
        displayFraction=right.tick==left.tick?1:(float)Math.clamp((displayClock-left.tick)/(right.tick-left.tick),0,1);
        displayPosition=displayVector(displayBefore,"worldPosition",position()).lerp(
            displayVector(displayAfter,"worldPosition",position()),displayFraction);
    }
    private static Vec3 displayVector(JsonObject frame,String key,Vec3 fallback){
        return frame!=null&&frame.has(key)?JakPlayers.vector(frame.getAsJsonArray(key)):fallback;
    }
    public float presentationFraction(){return displayFraction;}
    public JsonObject presentationBefore(){return displayBefore==null?previousFrame():displayBefore;}
    public JsonObject presentationAfter(){return displayAfter==null?frame():displayAfter;}
    public Vec3 presentationPosition(){return displayPosition==null?position():displayPosition;}
    public org.joml.Quaternionf presentationRotation(){
        return displayRotation(presentationBefore()).slerp(displayRotation(presentationAfter()),displayFraction);
    }
    private static org.joml.Quaternionf displayRotation(JsonObject frame){
        if(frame==null)return new org.joml.Quaternionf();
        var q=frame.getAsJsonArray("rotation");
        return new org.joml.Quaternionf(q.get(0).getAsFloat(),q.get(1).getAsFloat(),q.get(2).getAsFloat(),q.get(3).getAsFloat());
    }
    private Vec3 displayedSeat(JsonObject frame){
        Vec3 hull=displayVector(frame,"worldPosition",position());
        Vec3 point=hull.add(displayVector(frame,"seat",seat));
        float progress=frame==null?1:frame.get("boardingProgress").getAsFloat();
        if(frame!=null&&frame.has("boardingFrom")&&progress<1){
            point=displayVector(frame,"boardingFrom",point).lerp(point,progress*progress*(3-2*progress))
                .add(0,Math.sin(progress*Math.PI)*.24,0);
        }
        return point;
    }
    public Vec3 presentationRider(){
        return displayedSeat(presentationBefore()).lerp(displayedSeat(presentationAfter()),displayFraction);
    }
    private final WheeledWeapons weapons=new WheeledWeapons(this);

    private JakVehicle(EntityType<? extends JakVehicle> type,Level level,Model model) {
        super(type,level);
        this.model=model;
        health=model.hitPoints;
    }

    public static void initialize() {
        VehicleWreck.initialize();
        ZoomerTraffic.initialize();
        for(var model:Model.values()) {
            var type=ResourceKey.create(Registries.ENTITY_TYPE,JakData.id(model.id));
            model.type=Registry.register(BuiltInRegistries.ENTITY_TYPE,type,EntityType.Builder
                .<JakVehicle>of((entityType,level)->new JakVehicle(entityType,level,model),MobCategory.MISC)
                .sized(model.length,model.height).clientTrackingRange(12).updateInterval(1).build(type));
            var item=ResourceKey.create(Registries.ITEM,JakData.id(model.id));
            model.item=Registry.register(BuiltInRegistries.ITEM,item,new VehicleItem(new Item.Properties().setId(item).stacksTo(1)) {
                @Override public InteractionResult useOn(UseOnContext context) {
                    if(context.getClickedFace()!=Direction.UP)return InteractionResult.PASS;
                    if(context.getLevel() instanceof ServerLevel server) {
                        var car=model.type.create(server,EntitySpawnReason.SPAWN_ITEM_USE);
                        if(car==null)return InteractionResult.FAIL;
                        Vec3 point=Vec3.atBottomCenterOf(context.getClickedPos().above()).add(0,model.parkingClearance(),0);
                        car.snapTo(point.x,point.y,point.z,context.getRotation(),0);
                        // Its hull as its item kept it.
                        car.health=VehicleItem.health(context.getItemInHand())*model.hitPoints;
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
            long now=System.nanoTime();
            long tick=currentFrame.has("serverTick")?currentFrame.get("serverTick").getAsLong():tickCount;
            if(displayFrames.isEmpty()||tick<=displayFrames.getLast().tick){
                displayFrames.clear();displayClock=tick-2;displayTime=now;displayInterval=50_000_000;
            }else{
                double interval=(now-frameReceived)/(double)(tick-displayFrames.getLast().tick);
                displayInterval+=(Math.clamp(interval,1_000_000,250_000_000)-displayInterval)*.1;
            }
            displayFrames.addLast(new DisplayFrame(tick,currentFrame));
            while(displayFrames.size()>8)displayFrames.removeFirst();
            frameReceived=now;
            seat=JakPlayers.vector(currentFrame.getAsJsonArray("seat"));
            boardingProgress=currentFrame.get("boardingProgress").getAsFloat();
            boardingFrom=currentFrame.has("boardingFrom")?JakPlayers.vector(currentFrame.getAsJsonArray("boardingFrom")):null;
        }
    }
    public JsonObject frame(){return currentFrame;}
    public JsonObject previousFrame(){return previousFrame==null?currentFrame:previousFrame;}
    @Override public boolean isPickable(){return !isRemoved();}
    @Override protected boolean canAddPassenger(Entity passenger){
        return getPassengers().isEmpty()&&(passenger instanceof Player||route!=null&&passenger instanceof net.minecraft.world.entity.npc.villager.Villager);
    }
    @Override public boolean shouldBeSaved(){return route==null&&super.shouldBeSaved();}
    /** The traffic's NPC driver at the controls, if any. */
    Entity driver(){return getFirstPassenger() instanceof net.minecraft.world.entity.npc.villager.Villager v?v:null;}
    /** Rust zoomers.rs manage_traffic: gone with its zoomer. */
    void dropDriver(){var d=driver();if(d!=null){d.stopRiding();d.discard();}}
    /** Rust zoomers.rs eject_driver. */
    private void ejectDriver(){var d=driver();if(d!=null)ZoomerTraffic.eject(this,d,velocity);}
    public boolean canBoard(Player player,Vec3 point) {
        return JakPlayers.active(player)&&(getPassengers().isEmpty()||driver()!=null)&&!player.isSpectator()
            &&!player.isSleeping()&&player.getEyePosition().distanceToSqr(point)<=2.9*2.9;
    }
    @Override public InteractionResult interact(Player player,InteractionHand hand,Vec3 offset) {
        if(hand!=InteractionHand.MAIN_HAND||!canBoard(player,position().add(offset)))return InteractionResult.PASS;
        Vec3 approach=player.position();
        // Ridden by a player it is theirs, off its route (Rust zoomers.rs). stand-in:
        // the original leaves its driver drawn in the seat; one seat here, so he is thrown off.
        if(player instanceof ServerPlayer&&driver()!=null){ejectDriver();route=null;ZoomerTraffic.forget(this);}
        if(player instanceof ServerPlayer&&player.startRiding(this)){
            boardingFrom=approach;
            boardingProgress=0;
            controls=null;
            press=0;
            boardingAt=level().getGameTime();
            playCue(model.hover()?"vehicl-ignition":model.asset+"-start");
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
        if(passenger instanceof ServerPlayer player&&!player.getLastSectionPos().equals(net.minecraft.core.SectionPos.of(player)))
            player.level().getChunkSource().move(player);
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
        // Under water (sunk, put down in it, flooded) it is taken apart into
        // its item, which floats up where it was (Rust zoomers.rs, 15/10).
        if(getFluidHeight(net.minecraft.tags.FluidTags.WATER)>model.height*.5+.1){
            playCue("bike-down");
            takeApart(server);
            return;
        }
        var rider=getFirstPassenger() instanceof ServerPlayer player?player:null;
        boolean fresh=rider!=null&&controls!=null&&server.getGameTime()-lastInput<7;
        int held=fresh?controls.held()|press:0;
        if(fresh&&(press&8)!=0&&server.getGameTime()-boardingAt>6){
            rider.stopRiding();
            if(!model.hover())playCue(model.asset+"-stop");
            rider=null;
            held=0;
            fresh=false;
        }
        int edges=fresh?press:0;
        press=0;
        if(rider==null){controls=null;boardingFrom=null;}
        if(rider!=null&&route!=null){route=null;ZoomerTraffic.forget(this);}
        boardingProgress=boardingFrom==null?1:Math.clamp((server.getGameTime()-boardingAt)/11f,0,1);
        if(rider==null&&route==null&&tickCount>20&&velocity.lengthSqr()<.00001&&tickCount%10!=0
            &&(!model.hover()||currentFrame==null||currentFrame.get("lamps").getAsFloat()<.01f))return;
        // A FULL collision chunk is not necessarily ready to tick/retain
        // entities. Do not move the hull into a section that can unload it.
        var ahead=getBoundingBox().expandTowards(velocity.scale(.05)).inflate(1);
        for(int cx=(int)Math.floor(ahead.minX)>>4;cx<=((int)Math.floor(ahead.maxX)>>4);cx++)
            for(int cz=(int)Math.floor(ahead.minZ)>>4;cz<=((int)Math.floor(ahead.maxZ)>>4);cz++)
                if(!server.isPositionEntityTicking(new BlockPos(cx<<4,blockPosition().getY(),cz<<4)))return;
        try {
            if(handle==0){
                nativePosition=position();
                collision=new WheeledCollision(server,this,nativePosition,model.hover());
                handle=model.hover()?NativeJak.hoverCreate(collision,model.ordinal()-Model.BIKEA.ordinal(),
                    getYRot(),(float)velocity.x,(float)velocity.y,(float)velocity.z,pitch,roll,high,getUUID().getLeastSignificantBits()):
                    NativeJak.carCreate(collision,model.ordinal(),getYRot(),(float)velocity.x,
                        (float)velocity.y,(float)velocity.z,pitch,roll);
            }
            var at=position();
            var moved=at.subtract(collision.origin);
            collision.origin=at;
            int actions=held&7;
            if((held&64)!=0)actions|=8;
            float steer=fresh?-controls.strafe():0,lean=fresh?controls.forward():0;
            float throttle=fresh?controls.throttle():0,brake=fresh?controls.brake():0;
            boolean traffic=rider==null&&route!=null&&model.hover();
            if(traffic){
                var auto=ZoomerTraffic.autopilot(this,velocity);
                steer=auto.steer();lean=0;throttle=auto.throttle();brake=auto.brake();
                trafficNote[0]=steer;trafficNote[1]=throttle;trafficNote[2]=brake;trafficNote[3]=(float)velocity.horizontalDistance();
            }
            String encoded;
            if(model.hover()){
                int zoomer=(edges&1)|(held&6);
                float hour=(server.getDefaultClockTime()%24000+6000)%24000/1000f;
                encoded=NativeJak.hoverStep(handle,steer,lean,throttle,brake,zoomer,
                    (float)moved.x,(float)moved.y,(float)moved.z,hour%24,health/model.hitPoints,rider!=null||traffic);
            }else{
                encoded=NativeJak.carStep(handle,steer,lean,throttle,brake,actions,
                    (float)moved.x,(float)moved.y,(float)moved.z,3);
            }
            var frame=JsonParser.parseString(encoded).getAsJsonObject();
            Vec3 step=JakPlayers.vector(frame.getAsJsonArray("position"));
            nativePosition=at.add(step);
            velocity=JakPlayers.vector(frame.getAsJsonArray("velocity"));
            seat=JakPlayers.vector(frame.getAsJsonArray("seat")).subtract(step);
            pitch=frame.get("pitch").getAsFloat();
            if(model.hover())high=frame.get("high").getAsBoolean();
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
                if(model.hover()&&event.has("impact"))hoverImpact(event.get("impact").getAsFloat());
                if(rider!=null&&event.has("fire")){
                    var q=frame.getAsJsonArray("rotation");
                    var attitude=new org.joml.Quaternionf(q.get(0).getAsFloat(),q.get(1).getAsFloat(),q.get(2).getAsFloat(),q.get(3).getAsFloat());
                    weapons.fire(server,rider,attitude,velocity,event.get("fire").getAsBoolean(),effects);
                }
            }
            frame.add("gunEffects",effects);
            frame.add("grenades",shots);
            frame.add("seat",jsonVector(seat));
            frame.add("worldPosition",jsonVector(nativePosition));
            frame.addProperty("serverTick",server.getGameTime());
            frame.addProperty("boardingProgress",boardingProgress);
            frame.addProperty("boarding",boardingProgress<1?boardingProgress*.55f:-1);
            if(boardingProgress<1)frame.add("boardingFrom",jsonVector(boardingFrom));
            currentFrame=frame;
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
    private void hoverImpact(float speed){
        if(speed<12)return;
        String family=model.bike()?"bike":"car";
        playCue(family+(speed>=32?"-impact-stn":"-glance-stn"));
        health-=speed>=70?model.hitPoints*2:speed>=32?10:speed>=21?4:0;
        if(health<=0)wreck();
    }
    private void playCue(String name){
        var cue=JakEnemySounds.find("jak3",name);
        if(cue!=null)level().playSound(null,getX(),getY(),getZ(),cue,SoundSource.NEUTRAL,1,1);
    }
    @Override public boolean hurtServer(ServerLevel server,DamageSource source,float amount){
        if(amount<=0||isInvulnerableToBase(source))return false;
        // Jak's fists and feet do it no harm (Adrii 15/10, Rust zoomers.rs):
        // each blow cracks it as a block cracks; the third in a row takes it
        // apart into its item, which falls where it stood.
        if(source.is(net.minecraft.world.damagesource.DamageTypes.PLAYER_ATTACK)&&source.getEntity() instanceof net.minecraft.world.entity.player.Player){
            if(getFirstPassenger()!=null)return false;
            long now=server.getGameTime();
            if(now-lastStrike>STRIKE_GAP)strikes=0;
            lastStrike=now;
            strikes++;
            Vec3 at=position().add(0,model.height*.5,0);
            if(strikes>=3){
                server.playSound(null,at.x,at.y,at.z,net.minecraft.sounds.SoundEvents.METAL_BREAK,SoundSource.NEUTRAL,1,.8f);
                takeApart(server);
            }else server.playSound(null,at.x,at.y,at.z,net.minecraft.sounds.SoundEvents.METAL_HIT,SoundSource.NEUTRAL,1,.6f+.2f*strikes);
            return true;
        }
        // Jak's shots take two hit points, as the Rust fleet's did; the rest
        // of Minecraft's harm (lava, creepers) as it comes.
        boolean jakShot=source.typeHolder().unwrapKey().map(k->k.identifier().getNamespace().equals(dev.jakmc.JakMod.ID)).orElse(false);
        health-=jakShot?2:amount;
        if(health<=0)wreck();
        return true;
    }
    /** Back into its item where it stands, keeping its hull. */
    private void takeApart(ServerLevel server){
        ejectDriver();
        ejectPassengers();
        var stack=new net.minecraft.world.item.ItemStack(model.item);
        VehicleItem.setHealth(stack,health/model.hitPoints);
        spawnAtLocation(server,stack);
        discard();
    }
    /**
     * Rust zoomers.rs Fleet step, "No hit points left: it blows up": its
     * carcass and pieces (VehicleWreck), its driver thrown off, its rider set
     * down 2.5 m to its side and a metre up.
     */
    private void wreck(){
        if(isRemoved())return;
        ejectDriver();
        var rider=getFirstPassenger() instanceof ServerPlayer player?player:null;
        ejectPassengers();
        if(level() instanceof ServerLevel server){
            var attitude=currentFrame==null?new org.joml.Quaternionf().rotationY((float)-Math.toRadians(getYRot())):displayRotation(currentFrame);
            long mask=model==Model.TOAD?-1L:model==Model.SNAKE?5L:currentFrame!=null&&currentFrame.has("mask")?currentFrame.get("mask").getAsLong():-1L;
            String wheels=!model.hover()&&currentFrame!=null&&currentFrame.has("wheels")?currentFrame.getAsJsonArray("wheels").toString():"";
            VehicleWreck.zoomer(server,this,attitude,mask,wheels);
            if(rider!=null){
                double yaw=Math.toRadians(getYRot());
                Vec3 side=new Vec3(Math.cos(yaw),0,Math.sin(yaw));
                Vec3 to=position().add(side.scale(2.5*VehicleWreck.METRE)).add(0,VehicleWreck.METRE,0);
                rider.teleportTo(to.x,to.y,to.z);
            }
        }
        ZoomerTraffic.forget(this);
        discard();
    }
    private void closeNative(){
        if(handle!=0){if(model.hover())NativeJak.hoverDestroy(handle);else NativeJak.carDestroy(handle);handle=0;}
        collision=null;
    }
    @Override public void remove(RemovalReason reason){closeNative();super.remove(reason);}
    @Override protected void readAdditionalSaveData(ValueInput in){
        velocity=new Vec3(in.getDoubleOr("VelocityX",0),in.getDoubleOr("VelocityY",0),in.getDoubleOr("VelocityZ",0));
        pitch=in.getFloatOr("Pitch",0);
        roll=in.getFloatOr("Roll",0);
        health=in.getFloatOr("Health",model.hitPoints);
        high=in.getBooleanOr("High",false);
    }
    @Override protected void addAdditionalSaveData(ValueOutput out){
        out.putDouble("VelocityX",velocity.x);
        out.putDouble("VelocityY",velocity.y);
        out.putDouble("VelocityZ",velocity.z);
        out.putFloat("Pitch",pitch);
        out.putFloat("Roll",roll);
        out.putFloat("Health",health);
        out.putBoolean("High",high);
    }
}
