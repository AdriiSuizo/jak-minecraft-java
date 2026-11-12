package dev.jakmc.mixin;

import dev.jakmc.*;
import dev.jakmc.content.JakItems;
import net.minecraft.world.entity.item.ItemEntity;
import net.minecraft.world.entity.player.Player;
import net.minecraft.server.level.ServerPlayer;
import net.minecraft.sounds.*;
import org.spongepowered.asm.mixin.*;
import org.spongepowered.asm.mixin.injection.*;
import org.spongepowered.asm.mixin.injection.callback.CallbackInfo;
import java.util.UUID;

/** Native item pickup owns inventory insertion. Original pickup cues follow a successful transfer. */
@Mixin(ItemEntity.class)
abstract class JakPickupMixin {
 @Shadow private UUID target;
 @Unique private int jakmc$gemCount;
 @Unique private int jakmc$ecoCount;
 @Unique private boolean jakmc$lightEco;
 @Unique private int jakmc$ammoCount;
 @Inject(method="playerTouch",at=@At("HEAD"))
 private void jakmc$take(Player player,CallbackInfo ci){
  ItemEntity self=(ItemEntity)(Object)this;var stack=self.getItem();jakmc$gemCount=jakmc$ecoCount=jakmc$ammoCount=0;
  if(!(player instanceof ServerPlayer p)||self.hasPickUpDelay()||(target!=null&&!target.equals(p.getUUID())))return;
  if(stack.is(JakItems.SKULL_GEM)){jakmc$gemCount=stack.getCount();return;}
  jakmc$lightEco=stack.is(JakItems.LIGHT_ECO);
  if(jakmc$lightEco||stack.is(JakItems.DARK_ECO))jakmc$ecoCount=stack.getCount();
  if(JakItems.AMMUNITION.contains(stack.getItem()))jakmc$ammoCount=stack.getCount();
 }
 @Inject(method="playerTouch",at=@At("TAIL"))
 private void jakmc$gemSound(Player player,CallbackInfo ci){
  ItemEntity self=(ItemEntity)(Object)this;
  if(jakmc$gemCount>0&&(self.isRemoved()||self.getItem().getCount()<jakmc$gemCount)){dev.jakmc.content.GemPhysics.pickup(self,player);}
  if(jakmc$ammoCount>0&&(self.isRemoved()||self.getItem().getCount()<jakmc$ammoCount))
   self.level().playSound(null,self.getX(),self.getY(),self.getZ(),SoundEvent.createVariableRangeEvent(JakData.id("sfx/ammo_pickup")),SoundSource.PLAYERS,1,1);
  if(jakmc$ecoCount>0&&(self.isRemoved()||self.getItem().getCount()<jakmc$ecoCount)){
   self.level().playSound(null,self.getX(),self.getY(),self.getZ(),SoundEvent.createVariableRangeEvent(JakData.id("sfx/pill_pickup")),SoundSource.PLAYERS,1,1);
   if(jakmc$lightEco&&player instanceof ServerPlayer server)
    dev.jakmc.player.JakPlayers.ventPulse(server,false,2);
  }
 }
}
