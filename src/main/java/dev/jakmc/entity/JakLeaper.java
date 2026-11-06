package dev.jakmc.entity;

import dev.jakmc.JakData;
import dev.jakmc.nativebridge.NativeJak;
import dev.jakmc.player.JakPlayers;
import net.fabricmc.fabric.api.biome.v1.BiomeModifications;
import net.fabricmc.fabric.api.object.builder.v1.entity.FabricDefaultAttributeRegistry;
import net.minecraft.core.Direction;
import net.minecraft.core.BlockPos;
import net.minecraft.core.Registry;
import net.minecraft.core.registries.BuiltInRegistries;
import net.minecraft.core.registries.Registries;
import net.minecraft.network.chat.Component;
import net.minecraft.network.syncher.EntityDataAccessor;
import net.minecraft.network.syncher.EntityDataSerializers;
import net.minecraft.network.syncher.SynchedEntityData;
import net.minecraft.resources.ResourceKey;
import net.minecraft.server.level.ServerLevel;
import net.minecraft.server.level.ServerPlayer;
import net.minecraft.sounds.SoundEvent;
import net.minecraft.world.InteractionHand;
import net.minecraft.world.InteractionResult;
import net.minecraft.world.damagesource.DamageSource;
import net.minecraft.world.entity.Entity;
import net.minecraft.world.entity.EntityType;
import net.minecraft.world.entity.MobCategory;
import net.minecraft.world.entity.AgeableMob;
import net.minecraft.world.entity.animal.Animal;
import net.minecraft.world.item.ItemStack;
import net.minecraft.world.entity.SpawnPlacementTypes;
import net.minecraft.world.entity.SpawnPlacements;
import net.minecraft.world.entity.ai.attributes.Attributes;
import net.minecraft.world.entity.ai.goal.FloatGoal;
import net.minecraft.world.entity.ai.goal.RandomLookAroundGoal;
import net.minecraft.world.entity.ai.goal.WaterAvoidingRandomStrollGoal;
import net.minecraft.world.entity.player.Player;
import net.minecraft.world.item.Item;
import net.minecraft.world.item.SpawnEggItem;
import net.minecraft.world.level.Level;
import net.minecraft.world.level.storage.ValueInput;
import net.minecraft.world.level.storage.ValueOutput;
import net.minecraft.world.level.levelgen.Heightmap;
import net.minecraft.world.phys.Vec3;

/** The desert Leaper is a Minecraft mob. A reserved rider drives it through Jak's solver. */
public class JakLeaper extends Animal {
    private static final EntityDataAccessor<Integer> RIDER=
        SynchedEntityData.defineId(JakLeaper.class,EntityDataSerializers.INT);
    public static EntityType<JakLeaper> TYPE;
    public static Item EGG;
    private BlockPos patrolStart,patrolEnd;
    private boolean patrolReturn;
    private int patrolRetry;
    public void patrol(BlockPos start,BlockPos end){patrolStart=start;patrolEnd=end;}
    /** Rust: leapers.rs Beast.patrol_origin, a monk's patrol mount (not one of the town's loose leapers). */
    public boolean patrolling(){return patrolStart!=null;}

    public JakLeaper(EntityType<? extends JakLeaper> type,Level level){
        super(type,level);
        getNavigation().setCanFloat(true);
    }

    public static void initialize(){
        var key=ResourceKey.create(Registries.ENTITY_TYPE,JakData.id("leaper"));
        TYPE=Registry.register(BuiltInRegistries.ENTITY_TYPE,key,EntityType.Builder
            .<JakLeaper>of(JakLeaper::new,MobCategory.CREATURE)
            .sized(1.12f,2.08f).eyeHeight(1.7f).passengerAttachments(1.6f).clientTrackingRange(10).updateInterval(1).build(key));
        FabricDefaultAttributeRegistry.register(TYPE,createMobAttributes()
            .add(Attributes.MAX_HEALTH,30).add(Attributes.MOVEMENT_SPEED,.22)
            .add(Attributes.STEP_HEIGHT,1));
        SpawnPlacements.register(TYPE,SpawnPlacementTypes.ON_GROUND,Heightmap.Types.MOTION_BLOCKING_NO_LEAVES,
            (type,level,reason,pos,random)->level.getRawBrightness(pos,0)>8&&
                level.getBlockState(pos.below()).isFaceSturdy(level,pos.below(),Direction.UP));
        BiomeModifications.addSpawn(context->context.getBiomeKey().identifier().getPath().equals("desert"),
            MobCategory.CREATURE,TYPE,5,1,2);
        var eggKey=ResourceKey.create(Registries.ITEM,JakData.id("leaper_spawn_egg"));
        EGG=Registry.register(BuiltInRegistries.ITEM,eggKey,
            new SpawnEggItem(new Item.Properties().setId(eggKey).spawnEgg(TYPE)));
    }

    @Override protected void defineSynchedData(SynchedEntityData.Builder builder){
        super.defineSynchedData(builder);
        builder.define(RIDER,-1);
    }

    @Override protected void registerGoals(){
        goalSelector.addGoal(0,new FloatGoal(this));
        goalSelector.addGoal(1,new WaterAvoidingRandomStrollGoal(this,1){
            @Override public boolean canUse(){return patrolStart==null&&super.canUse();}
            @Override public boolean canContinueToUse(){return patrolStart==null&&super.canContinueToUse();}
        });
        goalSelector.addGoal(2,new RandomLookAroundGoal(this));
    }

    public int riderId(){return entityData.get(RIDER);}
    public void reserve(ServerPlayer rider){
        if(getFirstPassenger() instanceof net.minecraft.world.entity.npc.villager.Villager monk){
            monk.stopRiding();
            ((CitizenState)monk).citizenHit();
        }
        patrolStart=patrolEnd=null;
        entityData.set(RIDER,rider.getId());
        getNavigation().stop();
        setDeltaMovement(Vec3.ZERO);
        setPersistenceRequired();
    }
    public void release(){entityData.set(RIDER,-1);}

    @Override protected void customServerAiStep(ServerLevel level){
        super.customServerAiStep(level);
        if(patrolStart==null||patrolEnd==null||riderId()>=0)return;
        if(getFirstPassenger()==null){patrolStart=patrolEnd=null;getNavigation().stop();return;}
        BlockPos goal=patrolReturn?patrolStart:patrolEnd;
        if(position().distanceToSqr(Vec3.atBottomCenterOf(goal))<2.25){
            patrolReturn=!patrolReturn;
            getNavigation().stop();
            patrolRetry=15;
            return;
        }
        if(patrolRetry>0)patrolRetry--;
        if(getNavigation().isDone()&&patrolRetry==0){
            var path=getNavigation().createPath(goal,1);
            if(path!=null){
                // Validate the passenger envelope along a new path, then let Minecraft follow it.
                for(int i=0;i<path.getNodeCount();i++){
                    var step=path.getEntityPosAtNode(this,i).add(0,1.55,0);
                    var riderBox=new net.minecraft.world.phys.AABB(step.x-.33,step.y,step.z-.33,step.x+.33,step.y+1.75,step.z+.33);
                    if(!level.noCollision(riderBox)){path=null;break;}
                }
            }
            if(path!=null)getNavigation().moveTo(path,1.25);
            patrolRetry=20;
        }
    }

    @Override protected void addAdditionalSaveData(ValueOutput out){
        super.addAdditionalSaveData(out);
        if(patrolStart!=null&&patrolEnd!=null){
            out.putLong("JakPatrolStart",patrolStart.asLong());out.putLong("JakPatrolEnd",patrolEnd.asLong());
            out.putBoolean("JakPatrolReturn",patrolReturn);
        }
    }
    @Override protected void readAdditionalSaveData(ValueInput in){
        super.readAdditionalSaveData(in);
        long start=in.getLongOr("JakPatrolStart",Long.MIN_VALUE),end=in.getLongOr("JakPatrolEnd",Long.MIN_VALUE);
        if(start!=Long.MIN_VALUE&&end!=Long.MIN_VALUE)patrol(BlockPos.of(start),BlockPos.of(end));
        patrolReturn=in.getBooleanOr("JakPatrolReturn",false);
    }

    @Override public void aiStep(){
        if(riderId()<0)super.aiStep();
        else setDeltaMovement(Vec3.ZERO);
    }

    @Override public boolean isPushable(){return riderId()<0&&super.isPushable();}

    @Override protected boolean canBeABaby(){return false;}
    @Override public boolean isFood(ItemStack stack){return false;}
    @Override public AgeableMob getBreedOffspring(ServerLevel level,AgeableMob partner){return null;}
    public boolean readyToMount(){return true;}

    @Override public InteractionResult mobInteract(Player player,InteractionHand hand){
        if(isFood(player.getItemInHand(hand)))return super.mobInteract(player,hand);
        if(hand!=InteractionHand.MAIN_HAND||riderId()>=0||!readyToMount()||!JakPlayers.active(player))return InteractionResult.PASS;
        if(player instanceof ServerPlayer server){
            if(!NativeJak.mountReady()){
                server.sendSystemMessage(Component.translatable("message.jakmc.mount_assets"));
                return InteractionResult.SUCCESS_SERVER;
            }
            return JakPlayers.mount(server,this)?InteractionResult.SUCCESS_SERVER:InteractionResult.PASS;
        }
        return InteractionResult.SUCCESS;
    }

    @Override protected SoundEvent getAmbientSound(){return this instanceof JakFlutFlut&&riderId()<0?JakEnemySounds.find("jak1","flut-coo"):null;}
    @Override protected SoundEvent getHurtSound(DamageSource source){return this instanceof JakFlutFlut?JakEnemySounds.find("jak1","flut-hit"):JakEnemySounds.find("jak3","dflut-bump");}
    @Override protected SoundEvent getDeathSound(){return this instanceof JakFlutFlut?JakEnemySounds.find("jak1","flut-death"):JakEnemySounds.find("jak3","dflut-die");}

    @Override protected void tickDeath(){
        // The original Leaper stays in its death channel for three seconds.
        deathTime++;
        if(deathTime>=60&&!level().isClientSide()&&!isRemoved()){
            level().broadcastEntityEvent(this,(byte)60);
            remove(RemovalReason.KILLED);
        }
    }

    @Override public boolean hurtServer(ServerLevel level,DamageSource source,float amount){
        boolean damaged=super.hurtServer(level,source,amount);
        if(damaged&&riderId()>=0&&level.getEntity(riderId()) instanceof ServerPlayer rider){
            Entity attacker=source.getEntity();
            if(attacker!=rider)JakPlayers.mountHit(rider,this,attacker==null?position().add(0,0,-1):attacker.position());
        }
        return damaged;
    }
}
