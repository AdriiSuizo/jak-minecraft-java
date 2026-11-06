package dev.jakmc.entity;

import dev.jakmc.JakData;
import net.fabricmc.fabric.api.biome.v1.BiomeModifications;
import net.fabricmc.fabric.api.object.builder.v1.entity.FabricDefaultAttributeRegistry;
import net.minecraft.core.Direction;
import net.minecraft.core.Registry;
import net.minecraft.core.registries.BuiltInRegistries;
import net.minecraft.core.registries.Registries;
import net.minecraft.network.chat.Component;
import net.minecraft.network.syncher.EntityDataAccessor;
import net.minecraft.network.syncher.EntityDataSerializers;
import net.minecraft.network.syncher.SynchedEntityData;
import net.minecraft.resources.ResourceKey;
import net.minecraft.server.level.ServerLevel;
import net.minecraft.sounds.SoundEvents;
import net.minecraft.world.InteractionHand;
import net.minecraft.world.InteractionResult;
import net.minecraft.world.entity.EntityType;
import net.minecraft.world.entity.MobCategory;
import net.minecraft.world.entity.SpawnPlacements;
import net.minecraft.world.entity.SpawnPlacementTypes;
import net.minecraft.world.entity.ai.attributes.Attributes;
import net.minecraft.world.entity.ai.goal.BreedGoal;
import net.minecraft.world.entity.ai.goal.TemptGoal;
import net.minecraft.world.entity.animal.Animal;
import net.minecraft.world.entity.player.Player;
import net.minecraft.world.item.Item;
import net.minecraft.world.item.ItemStack;
import net.minecraft.world.item.Items;
import net.minecraft.world.item.SpawnEggItem;
import net.minecraft.world.level.Level;
import net.minecraft.world.level.levelgen.Heightmap;
import net.minecraft.world.level.storage.ValueInput;
import net.minecraft.world.level.storage.ValueOutput;

public final class JakFlutFlut extends JakLeaper {
    private static final EntityDataAccessor<Integer> FLAGS=SynchedEntityData.defineId(JakFlutFlut.class,EntityDataSerializers.INT);
    private static final EntityDataAccessor<Integer> PLUMAGE=SynchedEntityData.defineId(JakFlutFlut.class,EntityDataSerializers.INT);
    private static final EntityDataAccessor<Integer> GROWTH=SynchedEntityData.defineId(JakFlutFlut.class,EntityDataSerializers.INT);
    private static final int TAMED=1,SADDLED=2;
    public static final int COLORS=13;
    public static EntityType<JakFlutFlut> TYPE;
    public static Item EGG;
    private int temper;
    private int lay=6000;
    private int rear;
    // Rust flut_flut.rs: a panic's ticks left and whom it runs from; a failed taming's panic once the rear ends.
    private int panic,panicAfterRear;private net.minecraft.world.phys.Vec3 panicFrom;
    // Rust flut_flut.rs WALK, RUN (m/s; its metre is 25/36 of a block), LEASH_FOLLOW (blocks)
    static final float WALK=5,RUN=16;static final double METRE=25/36.0,LEASH_FOLLOW=5;

    public JakFlutFlut(EntityType<? extends JakFlutFlut> type,Level level){super(type,level);}

    public static void initialize(){
        var key=ResourceKey.create(Registries.ENTITY_TYPE,JakData.id("flut_flut"));
        TYPE=Registry.register(BuiltInRegistries.ENTITY_TYPE,key,EntityType.Builder
            .<JakFlutFlut>of(JakFlutFlut::new,MobCategory.CREATURE)
            .sized(1.5f,2.05f).eyeHeight(1.7f).clientTrackingRange(8).updateInterval(1).build(key));
        FabricDefaultAttributeRegistry.register(TYPE,createMobAttributes()
            .add(Attributes.MAX_HEALTH,15).add(Attributes.MOVEMENT_SPEED,.20).add(Attributes.STEP_HEIGHT,1).add(Attributes.TEMPT_RANGE,10));
        SpawnPlacements.register(TYPE,SpawnPlacementTypes.ON_GROUND,Heightmap.Types.MOTION_BLOCKING_NO_LEAVES,
            (type,level,reason,pos,random)->level.getRawBrightness(pos,0)>8&&
                level.getBlockState(pos.below()).isFaceSturdy(level,pos.below(),Direction.UP));
        BiomeModifications.addSpawn(context->context.getBiomeKey().identifier().getPath().endsWith("swamp"),
            MobCategory.CREATURE,TYPE,6,1,2);
        var egg=ResourceKey.create(Registries.ITEM,JakData.id("flut_flut_spawn_egg"));
        EGG=Registry.register(BuiltInRegistries.ITEM,egg,
            new SpawnEggItem(new Item.Properties().setId(egg).spawnEgg(TYPE)));
    }

    @Override protected boolean canBeABaby(){return true;}
    @Override protected void registerGoals(){
        super.registerGoals();
        goalSelector.addGoal(0,new Panic());
        goalSelector.addGoal(1,new BreedGoal(this,1));
        goalSelector.addGoal(2,new TemptGoal(this,1,this::isFood,false));
        goalSelector.addGoal(3,new FollowAdult());
    }
    @Override protected void defineSynchedData(SynchedEntityData.Builder builder){
        super.defineSynchedData(builder);
        builder.define(FLAGS,0);
        builder.define(PLUMAGE,-1);
        builder.define(GROWTH,-24000);
    }
    @Override public float getAgeScale(){return isBaby()?.45f+.55f*(1+entityData.get(GROWTH)/24000f):1;}
    @Override public void onSyncedDataUpdated(EntityDataAccessor<?> key){
        super.onSyncedDataUpdated(key);
        if(key.equals(GROWTH))refreshDimensions();
    }
    public int rearTime(){return rear;}
    public boolean tame(){return (entityData.get(FLAGS)&TAMED)!=0;}
    public boolean saddled(){return (entityData.get(FLAGS)&SADDLED)!=0;}
    public int plumage(){
        int saved=entityData.get(PLUMAGE);
        return saved<0?Math.floorMod(getUUID().hashCode(),COLORS):saved;
    }
    public void setPlumage(int color){entityData.set(PLUMAGE,Math.floorMod(color,COLORS));}
    @Override public boolean readyToMount(){return tame()&&saddled()&&!isBaby();}
    @Override public boolean isFood(ItemStack stack){
        return stack.is(Items.PITCHER_POD)||BuiltInRegistries.ITEM.getKey(stack.getItem()).getPath().endsWith("_seeds");
    }
    @Override public boolean canMate(Animal other){
        return riderId()<0&&other instanceof JakFlutFlut mate&&mate.riderId()<0&&super.canMate(mate);
    }
    @Override public void spawnChildFromBreeding(ServerLevel level,Animal other){
        if(JakFlutEgg.spawn(level,position(),true,random.nextBoolean()?plumage():((JakFlutFlut)other).plumage())){
            finalizeSpawnChildFromBreeding(level,other,null);
        }
    }
    /** Navigation's speed modifier for a pace in the Rust's m/s (ground travel settles at s·0.546/0.454 a tick). */
    double pace(float metresPerSecond){return metresPerSecond*METRE/20/(.546/.454)/getAttributeValue(Attributes.MOVEMENT_SPEED);}
    // Rust flut_flut.rs, hit: one that lives through it panics 5 s, thrown off the blow at 6 m/s and up at 5,
    // its love gone.
    @Override public boolean hurtServer(ServerLevel level,net.minecraft.world.damagesource.DamageSource source,float amount){
        boolean hurt=super.hurtServer(level,source,amount);
        if(hurt&&isAlive()&&riderId()<0){
            var from=source.getEntity()!=null?source.getEntity().position():source.getSourcePosition()!=null?source.getSourcePosition():position().add(getViewVector(1));
            startPanic(from,100);
            var away=position().subtract(from).multiply(1,0,1);
            away=away.lengthSqr()<1e-6?getViewVector(1).multiply(-1,0,-1):away.normalize();
            setDeltaMovement(away.scale(6*METRE/20).add(0,5*METRE/20,0));
            needsSync=true;
            resetLove();
        }
        return hurt;
    }
    void startPanic(net.minecraft.world.phys.Vec3 from,int ticks){panic=ticks;panicFrom=from;getNavigation().stop();}
    // Rust flut_flut.rs, death: an adult leaves 1 to 3 raw chicken, 0 to 2 feathers and its saddle; a chick nothing.
    @Override protected void dropCustomDeathLoot(ServerLevel level,net.minecraft.world.damagesource.DamageSource source,boolean player){
        super.dropCustomDeathLoot(level,source,player);
        if(isBaby())return;
        spawnAtLocation(level,new ItemStack(Items.CHICKEN,1+random.nextInt(3)));
        int feathers=random.nextInt(3);if(feathers>0)spawnAtLocation(level,new ItemStack(Items.FEATHER,feathers));
        if(saddled())spawnAtLocation(level,new ItemStack(Items.SADDLE));
    }
    // Rust flut_flut.rs LEASH_FOLLOW: on a lead it keeps within 5 blocks (12 snaps it, as vanilla).
    @Override public double leashElasticDistance(){return LEASH_FOLLOW;}
    /** Rust flut_flut.rs, State::Panic: runs at RUN·0.7 for a point 6 blocks off whoever it fears, weaving by sin(t·1.7 + id)·0.6. */
    private final class Panic extends net.minecraft.world.entity.ai.goal.Goal{
        Panic(){setFlags(java.util.EnumSet.of(Flag.MOVE));}
        @Override public boolean canUse(){return panic>0&&panicFrom!=null&&riderId()<0;}
        @Override public boolean canContinueToUse(){return canUse();}
        @Override public boolean requiresUpdateEveryTick(){return true;}
        @Override public void tick(){
            var away=position().subtract(panicFrom).multiply(1,0,1);
            away=away.lengthSqr()<1e-6?getViewVector(1).multiply(1,0,1):away.normalize();
            double weave=Math.sin(tickCount/20.0*1.7+getId())*.6;
            var turned=away.yRot((float)weave);
            var to=position().add(turned.scale(6));
            getNavigation().moveTo(to.x,to.y,to.z,pace(RUN*.7f));
        }
        @Override public void stop(){getNavigation().stop();}
    }
    /** Rust flut_flut.rs: a chick goes after the nearest grown bird when it is 3 to 16 blocks off, at WALK. */
    private final class FollowAdult extends net.minecraft.world.entity.ai.goal.Goal{
        private JakFlutFlut adult;
        FollowAdult(){setFlags(java.util.EnumSet.of(Flag.MOVE));}
        @Override public boolean canUse(){
            if(!isBaby()||riderId()>=0||panic>0)return false;
            adult=level().getEntitiesOfClass(JakFlutFlut.class,getBoundingBox().inflate(16),b->b!=JakFlutFlut.this&&!b.isBaby()&&b.isAlive())
                .stream().min(java.util.Comparator.comparingDouble(b->b.distanceToSqr(JakFlutFlut.this))).orElse(null);
            if(adult==null)return false;double d=distanceTo(adult);return d>3&&d<=16;
        }
        @Override public boolean canContinueToUse(){if(adult==null||!adult.isAlive()||!isBaby())return false;double d=distanceTo(adult);return d>3&&d<=16;}
        @Override public void tick(){if(tickCount%10==0)getNavigation().moveTo(adult,pace(WALK));}
        @Override public void stop(){adult=null;getNavigation().stop();}
    }
    @Override public void aiStep(){
        super.aiStep();
        if(rear>0)rear--;
        if(panic>0)panic--;
        if(panicAfterRear>0&&rear==0){var player=level().getNearestPlayer(this,8);startPanic(player!=null?player.position():position().add(getViewVector(1)),panicAfterRear);panicAfterRear=0;}
        if(riderId()<0&&!onGround()&&!isInWater()&&getDeltaMovement().y<-.128){
            var velocity=getDeltaMovement();
            setDeltaMovement(velocity.x,-.128,velocity.z);
        }
        if(!(level() instanceof ServerLevel server)||!isAlive())return;
        if(isBaby()&&tickCount%20==0)entityData.set(GROWTH,getAge());
        if(rear>0)getNavigation().stop();
        if(riderId()<0&&!isBaby()&&onGround()&&--lay<=0){
            if(JakFlutEgg.spawn(server,position().subtract(getViewVector(1).multiply(.7,0,.7)),false,plumage())){
                lay=6000+random.nextInt(6001);
            }else lay=20;
        }
    }
    @Override public InteractionResult mobInteract(Player player,InteractionHand hand){
        ItemStack item=player.getItemInHand(hand);
        if(riderId()>=0)return InteractionResult.PASS;
        if(isFood(item)){
            if(!level().isClientSide()){
                if(isBaby())ageUp(getSpeedUpSecondsWhenFeeding(-getAge()),true);
                else{
                    temper=Math.min(100,temper+3);
                    heal(1);
                    if(getAge()==0&&canFallInLove())setInLove(player);
                }
                usePlayerItem(player,hand,item);
                makeSound(JakEnemySounds.find("jak1","flut-coo"));
            }
            return InteractionResult.SUCCESS;
        }
        if(hand!=InteractionHand.MAIN_HAND||isBaby()||!dev.jakmc.player.JakPlayers.active(player))return InteractionResult.PASS;
        if(!readyToMount()){
            if(!level().isClientSide()){
                if(!tame()&&item.is(Items.SADDLE)){
                    player.sendSystemMessage(Component.translatable("message.jakmc.flut_tame_first"));
                }else if(!tame()){
                    rear=24;
                    getNavigation().stop();
                    setDeltaMovement(getDeltaMovement().multiply(0,1,0));
                    boolean success=random.nextInt(100)<temper;
                    if(success){
                        entityData.set(FLAGS,entityData.get(FLAGS)|TAMED);
                        setPersistenceRequired();
                    }else{temper=Math.min(100,temper+5);panicAfterRear=60;}
                    level().broadcastEntityEvent(this,(byte)(success?61:6));
                    makeSound(success?SoundEvents.HORSE_AMBIENT:SoundEvents.HORSE_ANGRY);
                    player.sendSystemMessage(Component.translatable(success?"message.jakmc.flut_saddle":"message.jakmc.flut_resists"));
                }else if(item.is(Items.SADDLE)){
                    entityData.set(FLAGS,entityData.get(FLAGS)|SADDLED);
                    usePlayerItem(player,hand,item);
                    makeSound(SoundEvents.HORSE_SADDLE.value());
                }else player.sendSystemMessage(Component.translatable("message.jakmc.flut_saddle"));
            }
            return InteractionResult.SUCCESS;
        }
        return super.mobInteract(player,hand);
    }
    @Override public void handleEntityEvent(byte event){
        if(event==61||event==6){
            rear=24;
            if(event==61)super.handleEntityEvent((byte)18);
            else for(int i=0;i<6;i++){
                level().addParticle(net.minecraft.core.particles.ParticleTypes.SMOKE,
                    getRandomX(1),getY()+getBbHeight(),getRandomZ(1),0,.02,0);
            }
        }else super.handleEntityEvent(event);
    }
    @Override protected void addAdditionalSaveData(ValueOutput out){
        super.addAdditionalSaveData(out);
        out.putInt("FlutFlags",entityData.get(FLAGS));
        out.putInt("FlutTemper",temper);
        out.putInt("FlutColor",plumage());
        out.putInt("FlutLay",lay);
    }
    @Override protected void readAdditionalSaveData(ValueInput in){
        super.readAdditionalSaveData(in);
        entityData.set(FLAGS,in.getIntOr("FlutFlags",0)&3);
        temper=Math.clamp(in.getIntOr("FlutTemper",0),0,100);
        setPlumage(in.getIntOr("FlutColor",plumage()));
        lay=Math.clamp(in.getIntOr("FlutLay",6000),0,12000);
    }
}
