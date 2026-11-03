package dev.jakmc.mixin;

import dev.jakmc.*;
import dev.jakmc.content.JakItems;
import dev.jakmc.player.JakPlayers;
import net.minecraft.world.entity.item.ItemEntity;
import net.minecraft.world.entity.player.Player;
import net.minecraft.server.level.ServerPlayer;
import net.minecraft.sounds.*;
import org.spongepowered.asm.mixin.*;
import org.spongepowered.asm.mixin.injection.*;
import org.spongepowered.asm.mixin.injection.callback.CallbackInfo;
import java.util.UUID;

/** Keep vanilla dropped-item collision and saves; eco feeds Jak's saved energy. */
@Mixin(ItemEntity.class)
abstract class JakPickupMixin {
 @Shadow private UUID target;
 @Unique private int jakmc$gemCount;
 @Inject(method="playerTouch",at=@At("HEAD"),cancellable=true)
 private void jakmc$take(Player player,CallbackInfo ci){
  ItemEntity self=(ItemEntity)(Object)this;var stack=self.getItem();jakmc$gemCount=0;
  if(!(player instanceof ServerPlayer p)||self.hasPickUpDelay()||(target!=null&&!target.equals(p.getUUID())))return;
  if(stack.is(JakItems.SKULL_GEM)){jakmc$gemCount=stack.getCount();return;}
  if(!JakPlayers.active(p))return;
  boolean light=stack.is(JakItems.LIGHT_ECO);if(!light&&!stack.is(JakItems.DARK_ECO))return;
  ci.cancel();var type=light?JakData.LIGHT_ECO:JakData.DARK_ECO;float energy=p.getAttachedOrCreate(type);if(energy>=100)return;
  var item=stack.getItem();float each=light?2:4;int count=Math.min(stack.getCount(),(int)Math.ceil((100-energy)/each));
  p.setAttached(type,Math.min(100,energy+each*count));stack.shrink(count);p.take(self,count);
  p.awardStat(net.minecraft.stats.Stats.ITEM_PICKED_UP.get(item),count);p.onItemPickup(self);
  self.level().playSound(null,self.getX(),self.getY(),self.getZ(),SoundEvent.createVariableRangeEvent(JakData.id("sfx/pill_pickup")),SoundSource.PLAYERS,1,1);
  if(stack.isEmpty())self.discard();
 }
 @Inject(method="playerTouch",at=@At("TAIL"))
 private void jakmc$gemSound(Player player,CallbackInfo ci){
  ItemEntity self=(ItemEntity)(Object)this;
  if(jakmc$gemCount>0&&(self.isRemoved()||self.getItem().getCount()<jakmc$gemCount)){dev.jakmc.content.GemPhysics.pickup(self,player);}
 }
}
