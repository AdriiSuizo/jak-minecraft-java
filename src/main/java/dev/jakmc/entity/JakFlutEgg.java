package dev.jakmc.entity;

import dev.jakmc.JakData;
import net.minecraft.core.Registry;
import net.minecraft.core.registries.BuiltInRegistries;
import net.minecraft.core.registries.Registries;
import net.minecraft.network.syncher.EntityDataAccessor;
import net.minecraft.network.syncher.EntityDataSerializers;
import net.minecraft.network.syncher.SynchedEntityData;
import net.minecraft.resources.ResourceKey;
import net.minecraft.server.level.ServerLevel;
import net.minecraft.sounds.SoundEvents;
import net.minecraft.sounds.SoundSource;
import net.minecraft.world.InteractionHand;
import net.minecraft.world.InteractionResult;
import net.minecraft.world.damagesource.DamageSource;
import net.minecraft.world.entity.Entity;
import net.minecraft.world.entity.EntitySpawnReason;
import net.minecraft.world.entity.EntityType;
import net.minecraft.world.entity.MobCategory;
import net.minecraft.world.entity.MoverType;
import net.minecraft.world.entity.player.Player;
import net.minecraft.world.item.Item;
import net.minecraft.world.item.ItemStack;
import net.minecraft.world.item.context.UseOnContext;
import net.minecraft.world.level.Level;
import net.minecraft.world.level.storage.ValueInput;
import net.minecraft.world.level.storage.ValueOutput;
import net.minecraft.world.phys.Vec3;

public final class JakFlutEgg extends Entity {
    public static final int HATCH=2400,CRACK=60,SHELL=80;
    private static final EntityDataAccessor<Integer> AGE=SynchedEntityData.defineId(JakFlutEgg.class,EntityDataSerializers.INT);
    public static EntityType<JakFlutEgg> TYPE;
    public static Item ITEM;
    private int age=-1,color;

    public JakFlutEgg(EntityType<? extends JakFlutEgg> type,Level level){super(type,level);}

    public static void initialize(){
        var key=ResourceKey.create(Registries.ENTITY_TYPE,JakData.id("flut_egg"));
        TYPE=Registry.register(BuiltInRegistries.ENTITY_TYPE,key,EntityType.Builder
            .of(JakFlutEgg::new,MobCategory.MISC).sized(.52f,.55f).clientTrackingRange(6).updateInterval(2).build(key));
        var item=ResourceKey.create(Registries.ITEM,JakData.id("flut_egg"));
        ITEM=Registry.register(BuiltInRegistries.ITEM,item,new Item(new Item.Properties().setId(item).stacksTo(16)){
            @Override public InteractionResult useOn(UseOnContext context){
                var level=context.getLevel();
                var pos=context.getClickedPos().relative(context.getClickedFace());
                if(!level.getBlockState(pos).canBeReplaced())return InteractionResult.PASS;
                if(level instanceof ServerLevel server){
                    int color=server.getRandom().nextInt(JakFlutFlut.COLORS);
                    if(!spawn(server,Vec3.atBottomCenterOf(pos),true,color))return InteractionResult.FAIL;
                    context.getItemInHand().consume(1,context.getPlayer());
                    server.playSound(null,pos,SoundEvents.TURTLE_EGG_CRACK,SoundSource.BLOCKS,.4f,1.3f);
                }
                return InteractionResult.SUCCESS;
            }
        });
    }

    public static boolean spawn(ServerLevel level,Vec3 position,boolean incubates,int color){
        var egg=TYPE.create(level,EntitySpawnReason.BREEDING);
        if(egg==null)return false;
        egg.snapTo(position.x,position.y,position.z,level.getRandom().nextFloat()*360,0);
        if(!level.noCollision(egg))return false;
        egg.color=color;
        egg.age=incubates?0:-1;
        egg.entityData.set(AGE,egg.age);
        return level.addFreshEntity(egg);
    }
    @Override protected void defineSynchedData(SynchedEntityData.Builder builder){builder.define(AGE,-1);}
    public int visualAge(){return entityData.get(AGE);}
    @Override public boolean isPickable(){return !isRemoved();}
    @Override public InteractionResult interact(Player player,InteractionHand hand,Vec3 location){
        if(visualAge()>=HATCH)return InteractionResult.PASS;
        if(level() instanceof ServerLevel server){
            ItemStack item=new ItemStack(ITEM);
            if(!player.getInventory().add(item))player.spawnAtLocation(server,item);
            discard();
        }
        return InteractionResult.SUCCESS;
    }
    @Override public boolean hurtServer(ServerLevel level,DamageSource source,float damage){
        if(damage<=0||isInvulnerableToBase(source))return false;
        discard();
        return true;
    }

    @Override public void tick(){
        super.tick();
        if(!(level() instanceof ServerLevel server))return;
        if(!onGround()){
            setDeltaMovement(getDeltaMovement().add(0,-.04,0));
            move(MoverType.SELF,getDeltaMovement());
        }else setDeltaMovement(Vec3.ZERO);
        if(age<0||!onGround())return;
        if(age==HATCH-1){
            var chick=JakFlutFlut.TYPE.create(server,EntitySpawnReason.BREEDING);
            if(chick==null)return;
            chick.setBaby(true);
            chick.setPlumage(color);
            chick.snapTo(getX(),getY(),getZ(),getYRot(),0);
            if(!server.noCollision(chick)||!server.addFreshEntity(chick))return;
            server.playSound(null,blockPosition(),SoundEvents.TURTLE_EGG_HATCH,SoundSource.BLOCKS,.6f,1);
        }
        age++;
        if(age==HATCH-CRACK)server.playSound(null,blockPosition(),SoundEvents.TURTLE_EGG_CRACK,SoundSource.BLOCKS,.5f,1);
        if(age>HATCH/2||age%20==0)entityData.set(AGE,age);
        if(age>=HATCH+SHELL)discard();
    }
    @Override protected void readAdditionalSaveData(ValueInput in){
        color=Math.floorMod(in.getIntOr("FlutColor",0),JakFlutFlut.COLORS);
        age=Math.clamp(in.getIntOr("FlutIncubation",-1),-1,HATCH+SHELL);
        entityData.set(AGE,age);
    }
    @Override protected void addAdditionalSaveData(ValueOutput out){
        out.putInt("FlutColor",color);
        out.putInt("FlutIncubation",age);
    }
}
