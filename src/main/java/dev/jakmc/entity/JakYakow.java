package dev.jakmc.entity;

import dev.jakmc.JakData;
import dev.jakmc.RuntimeConfig;
import java.util.EnumSet;
import java.util.Map;
import net.fabricmc.fabric.api.biome.v1.BiomeModifications;
import net.fabricmc.fabric.api.object.builder.v1.entity.FabricDefaultAttributeRegistry;
import net.minecraft.core.BlockPos;
import net.minecraft.core.Registry;
import net.minecraft.core.registries.BuiltInRegistries;
import net.minecraft.core.registries.Registries;
import net.minecraft.network.syncher.EntityDataAccessor;
import net.minecraft.network.syncher.EntityDataSerializers;
import net.minecraft.network.syncher.SynchedEntityData;
import net.minecraft.resources.ResourceKey;
import net.minecraft.server.level.ServerLevel;
import net.minecraft.sounds.SoundEvent;
import net.minecraft.world.damagesource.DamageSource;
import net.minecraft.world.entity.*;
import net.minecraft.world.entity.ai.attributes.Attributes;
import net.minecraft.world.entity.ai.goal.*;
import net.minecraft.world.entity.ai.util.DefaultRandomPos;
import net.minecraft.world.entity.animal.Animal;
import net.minecraft.world.entity.animal.cow.AbstractCow;
import net.minecraft.world.entity.player.Player;
import net.minecraft.world.item.Item;
import net.minecraft.world.item.SpawnEggItem;
import net.minecraft.world.level.Level;
import net.minecraft.world.level.block.state.BlockState;
import net.minecraft.world.level.levelgen.Heightmap;
import net.minecraft.world.level.storage.ValueInput;
import net.minecraft.world.level.storage.ValueOutput;
import net.minecraft.world.phys.Vec3;

/** Sandover's grazing herd, with native Animal breeding, navigation and water physics. */
public final class JakYakow extends AbstractCow {
    private static final String[] CLIPS={"yakow-idle","yakow-graze","yakow-walk","yakow-run","yakow-kicked"};
    private static final int IDLE=0,GRAZE=1,WALK=2,RUN=3,KICKED=4;
    private static final double WALK_SPEED=2.13,RUN_SPEED=6;
    public static final float SCALE=JakEnemyKind.SCALE;
    private static final EntityDataAccessor<Integer> CLIP=SynchedEntityData.defineId(JakYakow.class,EntityDataSerializers.INT);
    private static final EntityDataAccessor<Float> PHASE=SynchedEntityData.defineId(JakYakow.class,EntityDataSerializers.FLOAT);
    private static final EntityDataAccessor<Float> RATE=SynchedEntityData.defineId(JakYakow.class,EntityDataSerializers.FLOAT);
    private static Map<String,JakEnemyKind.Animation> animations;
    public static EntityType<JakYakow> TYPE;
    public static Item EGG;

    private BlockPos home;
    private LivingEntity threat;
    private int kicked,fleeDelay;
    private double speed;
    private boolean running;
    private float soundFrame=Float.NEGATIVE_INFINITY;
    private float idleRate=1;

    public JakYakow(EntityType<? extends JakYakow> type,Level level){super(type,level);}

    public static void initialize(){
        animations=JakEnemyKind.readAnimations(RuntimeConfig.gameAssets("jak1"),"yakow",true);
        for(String clip:CLIPS)if(!animations.containsKey(clip))throw new IllegalStateException("Original Yakow clip missing: "+clip);
        var key=ResourceKey.create(Registries.ENTITY_TYPE,JakData.id("yakow"));
        TYPE=Registry.register(BuiltInRegistries.ENTITY_TYPE,key,EntityType.Builder
            .<JakYakow>of(JakYakow::new,MobCategory.CREATURE)
            .sized(2.5f*SCALE*2,3.5f*SCALE).eyeHeight(2.85f*SCALE).clientTrackingRange(8).updateInterval(2).build(key));
        FabricDefaultAttributeRegistry.register(TYPE,createAttributes().add(Attributes.MAX_HEALTH,12.5)
            .add(Attributes.MOVEMENT_SPEED,JakEntities.navigationSpeed(WALK_SPEED)).add(Attributes.TEMPT_RANGE,10));
        SpawnPlacements.register(TYPE,SpawnPlacementTypes.ON_GROUND,Heightmap.Types.MOTION_BLOCKING_NO_LEAVES,Animal::checkAnimalSpawnRules);
        BiomeModifications.addSpawn(context->switch(context.getBiomeKey().identifier().getPath()){
            case "plains","sunflower_plains","meadow" -> true;
            default -> false;
        },MobCategory.CREATURE,TYPE,5,5,5);
        var egg=ResourceKey.create(Registries.ITEM,JakData.id("yakow_spawn_egg"));
        EGG=Registry.register(BuiltInRegistries.ITEM,egg,new SpawnEggItem(new Item.Properties().setId(egg).spawnEgg(TYPE)));
    }

    @Override protected void registerGoals(){
        goalSelector.addGoal(0,new FloatGoal(this));
        goalSelector.addGoal(1,new RearFleeGoal());
        goalSelector.addGoal(2,new BreedGoal(this,1));
        goalSelector.addGoal(3,new TemptGoal(this,Math.sqrt(1.25),this::isFood,false));
        goalSelector.addGoal(4,new FollowParentGoal(this,1.25));
    }

    @Override protected void defineSynchedData(SynchedEntityData.Builder data){
        super.defineSynchedData(data);
        data.define(CLIP,IDLE);
        data.define(PHASE,0f);
        data.define(RATE,1f);
    }

    @Override public EntityDimensions getDefaultDimensions(Pose pose){
        return getType().getDimensions().scale(isBaby()?.5f:1f);
    }

    @Override public boolean canMate(Animal other){return other instanceof JakYakow&&super.canMate(other);}
    @Override public JakYakow getBreedOffspring(ServerLevel level,AgeableMob mate){return TYPE.create(level,EntitySpawnReason.BREEDING);}

    @Override protected void addAdditionalSaveData(ValueOutput data){
        super.addAdditionalSaveData(data);
        if(home!=null)data.store("yakow_home",BlockPos.CODEC,home);
    }

    @Override protected void readAdditionalSaveData(ValueInput data){
        super.readAdditionalSaveData(data);
        home=data.read("yakow_home",BlockPos.CODEC).orElse(null);
    }

    @Override protected SoundEvent getAmbientSound(){return null;}
    @Override protected SoundEvent getHurtSound(DamageSource source){return null;}
    @Override protected SoundEvent getDeathSound(){return null;}
    @Override protected void playStepSound(BlockPos pos,BlockState state){}

    @Override protected void dropFromLootTable(ServerLevel level,DamageSource source,boolean playerKilled){
        EntityTypes.COW.getDefaultLootTable().ifPresent(table->dropFromLootTable(level,source,playerKilled,table));
    }

    @Override public boolean hurtServer(ServerLevel level,DamageSource source,float amount){
        boolean hit=super.hurtServer(level,source,amount);
        if(hit&&isAlive()){
            if(source.getEntity() instanceof LivingEntity living)threat=living;
            speed=Math.clamp(speed+4,RUN_SPEED,15);
            kicked=Math.max(1,(int)Math.ceil(animations.get(CLIPS[KICKED]).duration()*20));
            select(KICKED);
        }
        return hit;
    }

    public String clip(){return CLIPS[entityData.get(CLIP)];}
    public float animationAge(float partial){return entityData.get(PHASE)+partial*entityData.get(RATE)/20;}

    @Override public void aiStep(){
        super.aiStep();
        if(!(level() instanceof ServerLevel)||!isAlive())return;
        if(home==null)home=blockPosition();
        if(fleeDelay>0)fleeDelay--;
        if(kicked>0)kicked--;
        double moved=Math.hypot(getX()-xo,getZ()-zo)*20/SCALE;
        running=running?moved>=3.7:moved>4;
        int next=kicked>0?KICKED:running?RUN:moved>.1||isInWater()?WALK:IDLE;
        int current=entityData.get(CLIP);
        if(next==IDLE&&current==GRAZE)next=GRAZE;
        if(next!=current)select(next);
        float rate=switch(next){
            case WALK -> Math.max(.5f,(float)(moved/WALK_SPEED));
            case RUN -> Math.clamp((float)(2*moved/RUN_SPEED),.5f,1.5f);
            case IDLE -> idleRate;
            default -> 1;
        };
        var art=animations.get(CLIPS[next]);
        float phase=Math.min(art.duration(),entityData.get(PHASE)+rate/20);
        float artist=art.artist(phase);
        for(var cue:art.cues()){
            if(cue.frame()>soundFrame&&cue.frame()<=artist)playCue(cue.name(),cue.volume(),(float)Math.pow(2,cue.pitch()));
        }
        soundFrame=artist;
        if(phase>=art.duration()&&next!=KICKED){
            int again=next==IDLE&&random.nextFloat()<.25f?GRAZE:next==GRAZE?IDLE:next;
            if(again==IDLE){
                idleRate=.9f+.2f*random.nextFloat();
                float call=random.nextFloat();
                if(call<.4f)playCue(call<.2f?"yakow-1":"yakow-2",1,1);
            }
            select(again);
        }else{
            entityData.set(PHASE,phase);
        }
        entityData.set(RATE,rate);
    }

    private void select(int clip){
        entityData.set(CLIP,clip);
        entityData.set(PHASE,0f);
        soundFrame=Float.NEGATIVE_INFINITY;
    }

    private void playCue(String cue,float gain,float pitch){
        SoundEvent sound=JakEnemySounds.find("jak1",cue);
        if(sound==null)return;
        playSound(sound,gain,pitch);
        JakEnemySounds.cuesPlayed++;
    }

    private boolean tempting(Player player){
        return isFood(player.getMainHandItem())||isFood(player.getOffhandItem());
    }

    private final class RearFleeGoal extends Goal {
        private boolean returning;
        private int elapsed,repath;

        RearFleeGoal(){setFlags(EnumSet.of(Flag.MOVE));}

        @Override public boolean canUse(){
            if(kicked>0&&threat!=null&&threat.isAlive())return true;
            if(fleeDelay>0||tickCount<20)return false;
            Player player=level().getNearestPlayer(JakYakow.this,10*SCALE);
            if(player==null||player.isSpectator()||tempting(player)||!dev.jakmc.player.JakPlayers.active(player))return false;
            Vec3 direction=player.position().subtract(position()).multiply(1,0,1).normalize();
            if(getLookAngle().multiply(1,0,1).normalize().dot(direction)>=-.5)return false;
            threat=player;
            return true;
        }

        @Override public boolean canContinueToUse(){
            return elapsed<600&&threat!=null
                &&(kicked>0||!(threat instanceof Player player)||!tempting(player))
                &&(!returning||home!=null&&position().distanceToSqr(Vec3.atBottomCenterOf(home))>1);
        }

        @Override public boolean requiresUpdateEveryTick(){return true;}
        @Override public void start(){elapsed=repath=0;returning=false;}
        @Override public void stop(){
            getNavigation().stop();
            if(kicked==0)threat=null;
            speed=0;
            fleeDelay=20;
        }

        @Override public void tick(){
            elapsed++;
            double distance=distanceTo(threat)/SCALE;
            returning|=!threat.isAlive()||distance>20;
            double wish=WALK_SPEED;
            if(!returning){
                double adjusted=running?distance*.5:distance;
                wish=RUN_SPEED+(WALK_SPEED-RUN_SPEED)*Math.clamp((adjusted-6)/4,0,1);
            }
            speed+=Math.clamp(wish-speed,-.15,.15);
            if(repath--<=0||getNavigation().isDone()){
                repath=10;
                Vec3 destination=returning&&home!=null?Vec3.atBottomCenterOf(home):
                    DefaultRandomPos.getPosAway(JakYakow.this,12,4,threat.position());
                if(destination!=null)getNavigation().moveTo(destination.x,destination.y,destination.z,Math.sqrt(speed/WALK_SPEED));
            }
            getNavigation().setSpeedModifier(Math.sqrt(speed/WALK_SPEED));
        }
    }
}
