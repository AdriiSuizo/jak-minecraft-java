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

    /** pilot-states.gc target-pilot-get-on: Jak is on the seat by artist frame 10 and the vehicle takes his controls from 20.5. */
    public static final float ARRIVE_AFRAME=10f,READY_AFRAME=20.5f;
    /** A get-on animation: how long it runs, and how many of the artist's frames it counts a second (the art group's own timing). */
    public record GetOn(float seconds,float aframesPerSecond){}
    private static GetOn bikeOn,carOn;
    private static synchronized void loadGetOn(){
        if(bikeOn!=null)return;
        try{
            var report=JsonParser.parseString(NativeJak.pilotTimings()).getAsJsonObject();
            var bike=report.getAsJsonObject("bikeOn");var car=report.getAsJsonObject("carOn");
            carOn=new GetOn(car.get("seconds").getAsFloat(),car.get("aframesPerSecond").getAsFloat());
            bikeOn=new GetOn(bike.get("seconds").getAsFloat(),bike.get("aframesPerSecond").getAsFloat());
        }catch(RuntimeException|LinkageError failure){
            // stand-in: the export's own timings unavailable; the jak-pilot-hcar and jak-pilot art groups: 16 and 23 frames at 15 frames a second, 2 artist frames each.
            JakMod.LOG.warn("Pilot get-on timings unavailable, using the art groups' known ones",failure);
            carOn=new GetOn(22/15f,30f);
            bikeOn=new GetOn(1f,30f);
        }
    }
    /** How Jak gets on this vehicle (a bike's clip, else a car's: target-pilot-get-on picks by the vehicle's stance). */
    public GetOn getOn(){loadGetOn();return model.bike()?bikeOn:carOn;}
    /**
     * target-pilot-get-on: where Jak is {@code aframe} artist frames into getting on, from where he stood to the seat
     * (wherever the vehicle has taken it): horizontally the quarter sine of the ten frames, vertically straight.
     * (jak_core::pilot::get_on_position is its twin.)
     */
    public static Vec3 getOn(Vec3 from,Vec3 seat,float aframe){
        double t=Math.clamp(aframe/ARRIVE_AFRAME,0,1),sine=Math.sin(t*Math.PI/2);
        return new Vec3(from.x+(seat.x-from.x)*sine,from.y+(seat.y-from.y)*t,from.z+(seat.z-from.z)*sine);
    }

    private static final EntityDataAccessor<String> FRAME=SynchedEntityData.defineId(
        JakVehicle.class,EntityDataSerializers.STRING);
    public final Model model;
    private long handle,retryAt,lastInput,boardingAt;
    private WheeledCollision collision;
    private Vec3 nativePosition, velocity=Vec3.ZERO, seat=new Vec3(0,.5,0), boardingFrom;
    private record HullImpact(Vec3 speed,Vec3 point){}
    private final java.util.ArrayList<HullImpact> hullImpacts=new java.util.ArrayList<>();
    private int crashTicks;
    /** Seconds into the get-on animation (pilot-states.gc target-pilot-get-on), -1 when on; and Jak's facing as he got on. */
    private float boardingSeconds=-1,boardingYaw;
    /** The tick the get-on animation ended: the sitting loop opens with a tenth of a second's blend from its last pose (target-pilot-*-anim-loop's ja-channel-push!). */
    private long seatedAt=Long.MIN_VALUE/2;
    public static final float SEAT_BLEND=.1f;
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
    private double displayBob;

    public void samplePresentation(long now){samplePresentation(now,false);}
    public void samplePresentation(long now,boolean local){
        if(displayFrames.isEmpty())return;
        var latest=displayFrames.getLast();
        double dt=Math.max(0,(now-displayTime)/displayInterval);
        displayTime=now;
        double target=latest.tick-(local?.5:1)+Math.clamp((now-frameReceived)/displayInterval,0,1);
        double predicted=displayClock+dt;
        displayClock=Math.min(latest.tick+(local?.5:0),predicted+Math.clamp((target-predicted)*.5,-.5,.5)*dt);
        sampleDisplay(local,latest);
    }
    public void sampleReplayPresentation(float partialTick){
        if(displayFrames.isEmpty())return;
        var latest=displayFrames.getLast();
        displayClock=latest.tick-1+partialTick;
        sampleDisplay(false,latest);
    }
    private void sampleDisplay(boolean local,DisplayFrame latest){
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
        // A locally driven hull may advance at most half a tick from its last confirmed velocity.
        if(local&&displayClock>latest.tick&&previousFrame!=null){
            var lastStep=displayVector(latest.frame,"worldPosition",position()).subtract(displayVector(previousFrame,"worldPosition",position()));
            var motion=displayVector(latest.frame,"velocity",Vec3.ZERO).scale(.05);
            // A stopped chassis, or one that just bounced, must not be extrapolated through its contact.
            if(lastStep.dot(motion)>0)displayPosition=displayPosition.add(motion.scale(displayClock-latest.tick));
        }
        // Rust zoomers.rs: the hover's bob, sin(clock*2.1+phase)*0.07 m, its phase the zoomer's own.
        displayBob=Math.sin(displayClock/20.0*2.1+(getId()*0.618%1)*Math.PI*2)*0.07/(6400.0/4096);
    }
    private static Vec3 displayVector(JsonObject frame,String key,Vec3 fallback){
        return frame!=null&&frame.has(key)?JakPlayers.vector(frame.getAsJsonArray(key)):fallback;
    }
    public float presentationFraction(){return displayFraction;}
    public double presentationSeconds(){return displayClock/20;}
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
        return displayVector(frame,"worldPosition",position()).add(displayVector(frame,"seat",seat));
    }
    /**
     * The vertical offset the hover is drawn with: its bob and the step it has just climbed, eased away
     * (Rust zoomers.rs root_of). The pilot sits on the seat of the vehicle as drawn, so it takes it too:
     * the hull jumps up a whole block at once, the drawn bike and its rider rise together.
     */
    public double presentationLift(){
        var a=presentationBefore();var b=presentationAfter();
        float climb=a!=null&&b!=null&&a.has("climb")&&b.has("climb")?net.minecraft.util.Mth.lerp(displayFraction,a.get("climb").getAsFloat(),b.get("climb").getAsFloat()):0;
        return displayBob-climb;
    }
    /** Seconds into getting on at the displayed instant, or -1 once on. */
    public float presentationBoarding(){
        var a=presentationBefore();var b=presentationAfter();
        if(b==null||!b.has("boarding"))return -1;
        float after=b.get("boarding").getAsFloat();
        if(after<0)return -1;
        float before=a!=null&&a.has("boarding")&&a.get("boarding").getAsFloat()>=0?a.get("boarding").getAsFloat():after;
        return net.minecraft.util.Mth.lerp(displayFraction,before,after);
    }
    /** Jak's facing as he got on (degrees), for the slerp of target-pilot-get-on. */
    public float presentationBoardingYaw(){var b=presentationAfter();return b!=null&&b.has("boardingYaw")?b.get("boardingYaw").getAsFloat():0;}
    /**
     * Where Jak is: the seat of the vehicle as it is drawn (Rust zoomers.rs root_of's lift and all),
     * and, while he gets on, on his way to it (target-pilot-get-on).
     */
    public Vec3 presentationRider(){
        Vec3 point=displayedSeat(presentationBefore()).lerp(displayedSeat(presentationAfter()),displayFraction).add(0,presentationLift(),0);
        var a=presentationBefore();var b=presentationAfter();
        if(a!=null&&b!=null)point=point.add(presentationPosition().subtract(displayVector(a,"worldPosition",position())
            .lerp(displayVector(b,"worldPosition",position()),displayFraction)));
        float boarding=presentationBoarding();
        if(boarding>=0&&b!=null&&b.has("boardingFrom"))
            point=getOn(displayVector(b,"boardingFrom",point),point,boarding*getOn().aframesPerSecond());
        return point;
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
                displayFrames.clear();displayClock=tick-1;displayTime=now;displayInterval=50_000_000;
            }else{
                double interval=(now-frameReceived)/(double)(tick-displayFrames.getLast().tick);
                displayInterval+=(Math.clamp(interval,1_000_000,250_000_000)-displayInterval)*.1;
            }
            displayFrames.addLast(new DisplayFrame(tick,currentFrame));
            while(displayFrames.size()>8)displayFrames.removeFirst();
            frameReceived=now;
            seat=JakPlayers.vector(currentFrame.getAsJsonArray("seat"));
            boardingSeconds=currentFrame.get("boarding").getAsFloat();
            boardingYaw=currentFrame.has("boardingYaw")?currentFrame.get("boardingYaw").getAsFloat():0;
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
            boardingYaw=player.getYRot();
            boardingSeconds=0;
            seatedAt=Long.MIN_VALUE/2;
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
        if(boardingFrom!=null&&boardingSeconds>=0)point=getOn(boardingFrom,point,boardingSeconds*getOn().aframesPerSecond());
        move.accept(passenger,point.x,point.y,point.z);
        if(passenger instanceof ServerPlayer player&&!player.getLastSectionPos().equals(net.minecraft.core.SectionPos.of(player)))
            player.level().getChunkSource().move(player);
    }
    @Override public Vec3 getDismountLocationForPassenger(LivingEntity passenger) {
        // target-pilot-get-off leaves from the seat itself; Jak takes the jump from there.
        if(model.hover()&&passenger instanceof ServerPlayer player&&JakPlayers.pilotExiting(player)){
            Vec3 point=position().add(seat);
            if(level().noCollision(passenger,passenger.getBoundingBox().move(point.subtract(passenger.position()))))return point;
        }
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
        long profiled=ZoomerProfile.begin();
        try{tickVehicle();}finally{ZoomerProfile.end(level().isClientSide()?"client vehicle tick":"server vehicle tick",profiled);}
    }
    private void tickVehicle() {
        super.tick();
        if(!(level() instanceof ServerLevel server)||server.getGameTime()<retryAt)return;
        if(crashTicks>0&&--crashTicks==0){wreck();return;}
        // Under water (sunk, put down in it, flooded) it is taken apart into
        // its item, which floats up where it was (Rust zoomers.rs, 15/10).
        if(getFluidHeight(net.minecraft.tags.FluidTags.WATER)>model.height*.5+.1){
            playCue("bike-down");
            takeApart(server);
            return;
        }
        var rider=getFirstPassenger() instanceof ServerPlayer player?player:null;
        // target-pilot-get-on: the vehicle takes the pilot's controls (pilot-on) from artist frame 20.5, and he cannot get off before.
        boardingSeconds=boardingFrom==null?-1:(server.getGameTime()-boardingAt)/20f;
        if(boardingSeconds>=getOn().seconds()){boardingSeconds=-1;boardingFrom=null;seatedAt=server.getGameTime();}
        boolean ready=boardingSeconds<0||boardingSeconds*getOn().aframesPerSecond()>=READY_AFRAME;
        boolean fresh=crashTicks==0&&rider!=null&&ready&&controls!=null&&server.getGameTime()-lastInput<7;
        int held=fresh?controls.held()|press:0;
        if(fresh&&(press&8)!=0){
            // target-pilot-get-off: the original exit, taken from the seat by the Jak that comes of this.
            if(model.hover()){
                JakPlayers.pilotExit(rider,model.bike(),velocity);
            }
            rider.stopRiding();
            if(!model.hover())playCue(model.asset+"-stop");
            rider=null;
            held=0;
            fresh=false;
        }
        int edges=fresh?press:0;
        press=0;
        if(rider==null){controls=null;boardingFrom=null;boardingSeconds=-1;}
        if(rider!=null&&route!=null){route=null;ZoomerTraffic.forget(this);}
        if(crashTicks==0&&hullImpacts.isEmpty()&&rider==null&&route==null&&tickCount>20&&velocity.lengthSqr()<.00001&&tickCount%10!=0
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
            for(var impact:hullImpacts){
                Vec3 speed=impact.speed(),offset=impact.point().subtract(at);
                if(model.hover())NativeJak.hoverContact(handle,(float)speed.x,(float)speed.y,(float)speed.z,
                    (float)offset.x,(float)offset.y,(float)offset.z);
                else NativeJak.wheeledImpulse(handle,false,(float)speed.x,(float)speed.y,(float)speed.z);
            }
            hullImpacts.clear();
            int actions=held&7;
            if((held&64)!=0)actions|=8;
            float steer=fresh?-controls.strafe():0,lean=fresh?controls.forward():0;
            float throttle=fresh?controls.throttle():0,brake=fresh?controls.brake():0;
            boolean traffic=crashTicks==0&&rider==null&&route!=null&&model.hover();
            if(traffic){
                var auto=ZoomerTraffic.autopilot(this,velocity);
                steer=auto.steer();lean=0;throttle=auto.throttle();brake=auto.brake();
                trafficNote[0]=steer;trafficNote[1]=throttle;trafficNote[2]=brake;trafficNote[3]=(float)velocity.horizontalDistance();
            }
            String encoded;
            if(model.hover()){
                // 16: vehicle-flag riding, somebody aboard (their weight is on the roll thrusters).
                // 32: he is in the stance (target-pilot-stance), the get-on over, so a hard hit smacks him (target-pilot-impact).
                // Bikes already hop with Circle. Car zoomers can use it as a second handbrake button.
                int zoomer=(edges&1)|(held&2)|(model.bike()?held&4:(held&4)>>1)|(traffic?8:0)|(rider!=null?16:0)|(rider!=null&&boardingSeconds<0?32:0);
                float hour=(server.getDefaultClockTime()%24000+6000)%24000/1000f;
                encoded=NativeJak.hoverStep(handle,steer,lean,throttle,brake,zoomer,
                    (float)moved.x,(float)moved.y,(float)moved.z,hour%24,Math.max(0,health/model.hitPoints),crashTicks==0&&(rider!=null||traffic));
            }else{
                encoded=NativeJak.carStep(handle,steer,lean,throttle,brake,actions,
                    (float)moved.x,(float)moved.y,(float)moved.z,6);
            }
            var frame=JsonParser.parseString(encoded).getAsJsonObject();
            collision.commitImpacts();
            Vec3 step=JakPlayers.vector(frame.getAsJsonArray("position"));
            nativePosition=at.add(step);
            velocity=JakPlayers.vector(frame.getAsJsonArray("velocity"));
            // The motion the world reads of it, in blocks a tick (ZoomerTraffic.clearPath looks a moving craft's way).
            setDeltaMovement(velocity.scale(.05));
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
            // impact-pos, where the hull last hit something, as the world's own coordinates (the scrape's sparks).
            if(frame.has("impactPos"))frame.add("impactWorld",jsonVector(at.add(JakPlayers.vector(frame.getAsJsonArray("impactPos")))));
            frame.addProperty("serverTick",server.getGameTime());
            frame.addProperty("boarding",boardingSeconds);
            if(crashTicks>0)frame.addProperty("crashing",true);
            float seated=(server.getGameTime()-seatedAt)/20f;
            if(rider!=null&&seated<SEAT_BLEND+.05f)frame.addProperty("seated",seated);
            if(boardingSeconds>=0&&boardingFrom!=null){frame.add("boardingFrom",jsonVector(boardingFrom));frame.addProperty("boardingYaw",boardingYaw);}
            currentFrame=frame;
            entityData.set(FRAME,frame.toString());
        }catch(RuntimeException failure){
            closeNative();
            JakMod.LOG.error("Wheeled vehicle stopped at its last valid pose {}",getUUID(),failure);
            retryAt=server.getGameTime()+100;
        }
    }

    /** A hull can be walked or landed on, except by its own passenger. */
    public boolean blocksJak(Entity actor){
        return !isRemoved()&&actor!=null&&actor!=this&&actor.getVehicle()!=this;
    }
    /** Add the same oriented chassis to Jak's native terrain query. */
    public void appendHull(Vec3 origin,java.nio.ByteBuffer data){
        double yaw=Math.toRadians(getYRot());
        var half=collisionHalf();
        double hx=half.x,hy=half.y,hz=half.z;
        var at=position();
        dev.jakmc.nativebridge.MinecraftCollision.turnedBox(data,at.x-origin.x,at.z-origin.z,-Math.sin(yaw),Math.cos(yaw),hx,hz,
            at.y-hy-origin.y,at.y+Math.max(seat.y,hy)-origin.y);
    }

    /** Native chassis half extents in Minecraft blocks, shared by entity and Jak collisions. */
    public Vec3 collisionHalf(){
        if(model==Model.TOAD)return new Vec3(2.3,1.15,4.8).scale(.64);
        if(model==Model.SNAKE)return new Vec3(1.8,.65,3.5).scale(.64);
        return model.bike()?new Vec3(.8,.6,1.5).scale(.64):new Vec3(1.4,.7,2.4).scale(.64);
    }

    public double collisionMass(){
        return switch(model){
            case BIKEA,BIKEB,BIKEC -> 2;
            case CARA -> 8;
            case CARB -> 6;
            case CARC -> 9;
            case TOAD -> 16;
            case SNAKE -> 8;
        };
    }
    public Vec3 collisionVelocity(){
        Vec3 result=velocity;
        for(var impact:hullImpacts)result=result.add(impact.speed());
        return result;
    }
    public void queueCollision(Vec3 speed,Vec3 point){
        if(!isRemoved())hullImpacts.add(new HullImpact(speed,point));
    }

    public void kickPush(Vec3 impulse){
        if(!(level() instanceof ServerLevel))return;
        Vec3 speed=impulse.scale(20);
        if(handle==0)velocity=velocity.add(speed);
        else NativeJak.wheeledImpulse(handle,model.hover(),(float)speed.x,(float)speed.y,(float)speed.z);
    }

    static JsonArray jsonVector(Vec3 point){
        var result=new JsonArray(3);
        result.add(point.x);
        result.add(point.y);
        result.add(point.z);
        return result;
    }
    private void hoverImpact(float speed){
        if(speed<1.5f||crashTicks>0)return;
        String family=model.bike()?"bike":"car";
        playCue(family+(speed>=25?"-impact-stn":"-glance-stn"),Math.min(1,speed/15));
        health-=speed>=70?model.hitPoints*2:speed>=32?10:speed>=25?5:0;
        if(health<=0)crashTicks=20;
    }
    /** A cue played where this vehicle is on this client only (the sounds the engine particles ask for), if the owned banks have it. */
    public void clientCue(String name){cueAt(level(),position(),name);}
    /** A cue played at a place on this client only, if the owned sound banks have it (a name they lack is silent, as in the game). */
    public static void cueAt(Level level,Vec3 at,String name){
        var cue=JakEnemySounds.find("jak3",name);
        if(cue!=null)level.playLocalSound(at.x,at.y,at.z,cue,SoundSource.NEUTRAL,1,1,false);
    }
    /**
     * pilot-states.gc target-pilot-hit and target-pilot-death: an attack that reaches Jak on his vehicle takes his health to -1000, and he dies
     * ({@code mode} of the attack: 'explode, 'melt (lava, fire), 'endlessfall, else 'generic). So a pilot struck by what the original sends
     * as an attack event dies at once, and the others see the burst of his death (JakPilotDeath).
     * stand-in: the original's attack events are all that reach a pilot; here Minecraft's explosions, fire and lava, lightning and the
     * void count, and the hits of mobs and arrows, which the vehicle's own shape takes in the original, still only hurt him.
     */
    public void riderHit(ServerPlayer player,DamageSource source){
        if(!model.hover())return;
        int mode;
        if(source.is(net.minecraft.tags.DamageTypeTags.IS_EXPLOSION))mode=JakPackets.PilotDeath.EXPLODE;
        else if(source.is(net.minecraft.world.damagesource.DamageTypes.FELL_OUT_OF_WORLD))mode=JakPackets.PilotDeath.ENDLESS_FALL;
        else if(source.is(net.minecraft.world.damagesource.DamageTypes.LAVA)||source.is(net.minecraft.tags.DamageTypeTags.IS_FIRE))mode=JakPackets.PilotDeath.MELT;
        else if(source.is(net.minecraft.world.damagesource.DamageTypes.LIGHTNING_BOLT))mode=JakPackets.PilotDeath.GENERIC;
        else return;
        if(!player.isDeadOrDying()){player.setHealth(0);player.die(source);}
        var packet=new JakPackets.PilotDeath(level().dimension().identifier(),player.getId(),player.position(),mode);
        for(var watcher:net.fabricmc.fabric.api.networking.v1.PlayerLookup.around((ServerLevel)level(),player.position(),96))
            if(net.fabricmc.fabric.api.networking.v1.ServerPlayNetworking.canSend(watcher,packet.type()))
                net.fabricmc.fabric.api.networking.v1.ServerPlayNetworking.send(watcher,packet);
    }
    private void playCue(String name){playCue(name,1);}
    private void playCue(String name,float gain){
        var cue=JakEnemySounds.find("jak3",name);
        if(cue!=null)level().playSound(null,getX(),getY(),getZ(),cue,SoundSource.NEUTRAL,gain,1);
    }
    @Override public boolean hurtServer(ServerLevel server,DamageSource source,float amount){
        if(amount<=0||crashTicks>0||isInvulnerableToBase(source))return false;
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
        crashTicks=Math.clamp(in.getIntOr("CrashTicks",0),0,20);
    }
    @Override protected void addAdditionalSaveData(ValueOutput out){
        Vec3 speed=collisionVelocity();
        out.putDouble("VelocityX",speed.x);
        out.putDouble("VelocityY",speed.y);
        out.putDouble("VelocityZ",speed.z);
        out.putFloat("Pitch",pitch);
        out.putFloat("Roll",roll);
        out.putFloat("Health",health);
        out.putBoolean("High",high);
        if(crashTicks>0)out.putInt("CrashTicks",crashTicks);
    }
}
