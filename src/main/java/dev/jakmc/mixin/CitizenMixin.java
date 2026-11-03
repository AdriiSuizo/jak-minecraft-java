package dev.jakmc.mixin;
import dev.jakmc.entity.*;
import net.minecraft.world.entity.npc.villager.Villager;
import net.minecraft.network.syncher.*;
import net.minecraft.world.level.storage.*;
import net.minecraft.server.level.ServerLevel;
import org.spongepowered.asm.mixin.*;
import org.spongepowered.asm.mixin.injection.*;
import org.spongepowered.asm.mixin.injection.callback.CallbackInfo;
/** Preserve the actual Villager entity and its whole native brain/inventory/POI lifecycle. */
@Mixin(Villager.class)
abstract class CitizenMixin implements CitizenState {
 @Unique private static final EntityDataAccessor<Integer> JAK_VARIANT=SynchedEntityData.defineId(Villager.class,EntityDataSerializers.INT),JAK_PHASE=SynchedEntityData.defineId(Villager.class,EntityDataSerializers.INT);
 @Unique private static final EntityDataAccessor<Long> JAK_PHASE_TIME=SynchedEntityData.defineId(Villager.class,EntityDataSerializers.LONG);
 @Inject(method="defineSynchedData",at=@At("TAIL"))private void jakmc$define(SynchedEntityData.Builder b,CallbackInfo ci){b.define(JAK_VARIANT,-1);b.define(JAK_PHASE,0);b.define(JAK_PHASE_TIME,0L);}
 public int citizenVariant(){var v=(Villager)(Object)this;int value=v.getEntityData().get(JAK_VARIANT);return value>=0&&value<CitizenKind.ALL.size()?value:Math.floorMod(v.getUUID().hashCode(),CitizenKind.HAVEN_VARIANTS);}
 public void citizenVariant(int value){((Villager)(Object)this).getEntityData().set(JAK_VARIANT,Math.clamp(value,0,CitizenKind.ALL.size()-1));}
 public int citizenPhase(){return ((Villager)(Object)this).getEntityData().get(JAK_PHASE);}
 public long citizenPhaseStart(){return ((Villager)(Object)this).getEntityData().get(JAK_PHASE_TIME);}
 @Unique private void jakmc$phase(int p){var v=(Villager)(Object)this;v.getEntityData().set(JAK_PHASE,p);v.getEntityData().set(JAK_PHASE_TIME,dev.jakmc.player.JakEntityClock.time(v.level()));}
 public void citizenHit(){var v=(Villager)(Object)this;if(!v.level().isClientSide()&&v.isAlive()&&v.getType()==net.minecraft.world.entity.EntityTypes.VILLAGER&&citizenPhase()==0)jakmc$phase(1);}
 @Inject(method="tick",at=@At("TAIL"))private void jakmc$react(CallbackInfo ci){var v=(Villager)(Object)this;if(v.level().isClientSide()||!v.isAlive()||citizenPhase()==0)return;double t=(dev.jakmc.player.JakEntityClock.time(v.level())-citizenPhaseStart())/20.;if(citizenPhase()==1&&(v.onGround()||v.isInWater())&&t>=.15)jakmc$phase(2);else if(citizenPhase()==2&&t>=jakmc$reactionKind().duration("knocked-back-land"))jakmc$phase(3);else if(citizenPhase()==3&&t>=jakmc$reactionKind().duration(jakmc$reactionKind().rise()))jakmc$phase(0);}
 @Unique private CitizenKind jakmc$reactionKind(){var kind=CitizenKind.of(citizenVariant());return kind.monk()?CitizenKind.of(0):kind;}
 @Inject(method="customServerAiStep",at=@At("HEAD"),cancellable=true)private void jakmc$reaction(ServerLevel level,CallbackInfo ci){if(citizenPhase()!=0){var v=(Villager)(Object)this;v.getNavigation().stop();v.getMoveControl().setWantedPosition(v.getX(),v.getY(),v.getZ(),0);ci.cancel();}}
 @Inject(method="addAdditionalSaveData",at=@At("TAIL"))private void jakmc$save(ValueOutput out,CallbackInfo ci){out.putInt("JakCitizenVariant",citizenVariant());}
 @Inject(method="readAdditionalSaveData",at=@At("TAIL"))private void jakmc$load(ValueInput in,CallbackInfo ci){int variant=in.getIntOr("JakCitizenVariant",-1);if(variant>=0)citizenVariant(variant);}
}
