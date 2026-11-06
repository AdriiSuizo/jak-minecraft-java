package dev.jakmc.mixin;
import com.llamalad7.mixinextras.injector.WrapWithCondition;
import dev.jakmc.citizens.*;
import net.minecraft.network.syncher.*;
import net.minecraft.server.level.ServerLevel;
import net.minecraft.world.entity.EntityTypes;
import net.minecraft.world.entity.LivingEntity;
import net.minecraft.world.entity.ai.Brain;
import net.minecraft.world.entity.npc.villager.Villager;
import org.spongepowered.asm.mixin.*;
import org.spongepowered.asm.mixin.injection.*;
import org.spongepowered.asm.mixin.injection.callback.CallbackInfo;
/** The original town life rides on the native Villager; while it drives the body the native brain sits out. */
@Mixin(Villager.class)
abstract class CitizenLifeMixin implements CitizenLifeState {
 @Unique private static final EntityDataAccessor<Integer> JAK_LIFE=SynchedEntityData.defineId(Villager.class,EntityDataSerializers.INT);
 @Unique private static final EntityDataAccessor<Long> JAK_LIFE_TIME=SynchedEntityData.defineId(Villager.class,EntityDataSerializers.LONG);
 @Unique private CitizenLife jakmc$life;
 @Inject(method="defineSynchedData",at=@At("TAIL"))private void jakmc$defineLife(SynchedEntityData.Builder b,CallbackInfo ci){b.define(JAK_LIFE,0);b.define(JAK_LIFE_TIME,0L);}
 public CitizenLife citizenLife(){var v=(Villager)(Object)this;int variant=((dev.jakmc.entity.CitizenState)v).citizenVariant();if(jakmc$life==null||jakmc$life.variant!=variant)jakmc$life=new CitizenLife(v);return jakmc$life;}
 public int citizenLifeState(){return ((Villager)(Object)this).getEntityData().get(JAK_LIFE);}
 public long citizenLifeStart(){return ((Villager)(Object)this).getEntityData().get(JAK_LIFE_TIME);}
 public void citizenLifeState(int state,long start){var d=((Villager)(Object)this).getEntityData();d.set(JAK_LIFE,state);d.set(JAK_LIFE_TIME,start);}
 @Inject(method="tick",at=@At("TAIL"))private void jakmc$live(CallbackInfo ci){var v=(Villager)(Object)this;if(v.getType()==EntityTypes.VILLAGER&&v.level() instanceof ServerLevel level)citizenLife().tick(level);}
 @WrapWithCondition(method="customServerAiStep",at=@At(value="INVOKE",target="Lnet/minecraft/world/entity/ai/Brain;tick(Lnet/minecraft/server/level/ServerLevel;Lnet/minecraft/world/entity/LivingEntity;)V"))
 private boolean jakmc$brain(Brain<?> brain,ServerLevel level,LivingEntity body){return jakmc$life==null||!jakmc$life.owning();}
}
