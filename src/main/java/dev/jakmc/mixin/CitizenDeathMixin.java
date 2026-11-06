package dev.jakmc.mixin;
import dev.jakmc.content.JakItems;
import dev.jakmc.entity.CitizenKind;
import dev.jakmc.entity.CitizenState;
import net.minecraft.server.level.ServerLevel;
import net.minecraft.tags.DamageTypeTags;
import net.minecraft.world.damagesource.DamageSource;
import net.minecraft.world.entity.Entity;
import net.minecraft.world.entity.EntityTypes;
import net.minecraft.world.entity.LivingEntity;
import net.minecraft.world.entity.npc.villager.Villager;
import net.minecraft.world.item.ItemStack;
import org.spongepowered.asm.mixin.*;
import org.spongepowered.asm.mixin.injection.*;
import org.spongepowered.asm.mixin.injection.callback.*;
@Mixin(LivingEntity.class)
abstract class CitizenDeathMixin {
 @Unique private int jakmc$landed=-1;
 // Rust: jak2_villagers.rs step (the dead lie where they fell, 4 s from landing and 10 s at most, then puff and leave their green eco)
 @Inject(method="tickDeath",at=@At("HEAD"),cancellable=true)private void jakmc$lie(CallbackInfo ci){
  if(!((Object)this instanceof Villager v)||v.getType()!=EntityTypes.VILLAGER)return;
  ci.cancel();v.deathTime++;
  if(!(v.level() instanceof ServerLevel level)||v.isRemoved())return;
  if(jakmc$landed<0&&(v.onGround()||v.isInWater()))jakmc$landed=v.deathTime;
  if(!(jakmc$landed>=0&&v.deathTime>jakmc$landed+80||v.deathTime>200))return;
  level.broadcastEntityEvent(v,(byte)60);
  v.spawnAtLocation(level,new ItemStack(JakItems.GREEN_ECO),.5f);
  v.remove(Entity.RemovalReason.KILLED);
 }
 // Rust: jak2_villagers.rs draw (Jak 1's people are spared blows, as in his first game)
 @Inject(method="isInvulnerableTo",at=@At("HEAD"),cancellable=true)private void jakmc$spared(ServerLevel level,DamageSource source,CallbackInfoReturnable<Boolean> cir){
  if((Object)this instanceof Villager v&&v.getType()==EntityTypes.VILLAGER&&CitizenKind.of(((CitizenState)v).citizenVariant()).jak1()&&!source.is(DamageTypeTags.BYPASSES_INVULNERABILITY))cir.setReturnValue(true);
 }
}
