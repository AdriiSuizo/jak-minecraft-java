package dev.jakmc.content;
import dev.jakmc.*;
import dev.jakmc.player.JakPlayers;
import dev.jakmc.entity.JakEnemyKind;
import net.minecraft.world.entity.item.ItemEntity;
import net.minecraft.server.level.ServerPlayer;
import net.minecraft.sounds.*;
/** ItemEntity remains the only authoritative object: collision, fluids, lifetime, merge and inventory. */
public final class GemPhysics {
 public static final double UNIT=JakEnemyKind.SCALE;
 private static final java.util.Map<ServerPlayer,Long> LAST_PICKUP=new java.util.WeakHashMap<>();
 /** game-info.gc suppresses overlapping gem-pickup cues within .05 seconds. */
 public static void pickup(ItemEntity item,net.minecraft.world.entity.player.Player player){if(!(player instanceof ServerPlayer p))return;long now=p.level().getGameTime();if(java.util.Objects.equals(LAST_PICKUP.get(p),now))return;LAST_PICKUP.put(p,now);sound(item,"gem_pickup",1);pickupCues++;}
 public static long spawnCues,bounceCues,suckCues,pickupCues;
 public static void sound(ItemEntity item,String cue,float gain){if(item.level().isClientSide())return;item.level().playSound(null,item.getX(),item.getY(),item.getZ(),SoundEvent.createVariableRangeEvent(JakData.id("sfx/"+cue)),SoundSource.NEUTRAL,gain,1);}
 public static boolean accepts(ServerPlayer p){if(!JakPlayers.active(p)||p.isSpectator())return false;var inv=p.getInventory();for(int i=0;i<36;i++){var s=inv.getItem(i);if(s.isEmpty()||s.is(JakItems.SKULL_GEM)&&s.getCount()<s.getMaxStackSize())return true;}return false;}
 public static void launch(ItemEntity item){if(item==null)return;var r=item.level().getRandom();item.setDeltaMovement((r.nextDouble()*10-5)*UNIT/20,UNIT,(r.nextDouble()*10-5)*UNIT/20);item.setPickUpDelay(10);item.needsSync=true;}
}
