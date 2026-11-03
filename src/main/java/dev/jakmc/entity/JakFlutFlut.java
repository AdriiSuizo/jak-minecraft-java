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
        goalSelector.addGoal(1,new BreedGoal(this,1));
        goalSelector.addGoal(2,new TemptGoal(this,1,this::isFood,false));
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
    @Override public void aiStep(){
        super.aiStep();
        if(rear>0)rear--;
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
                    }else temper=Math.min(100,temper+5);
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
