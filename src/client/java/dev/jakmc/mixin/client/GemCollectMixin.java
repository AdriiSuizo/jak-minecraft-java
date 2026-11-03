package dev.jakmc.mixin.client;
import dev.jakmc.content.JakItems;
import dev.jakmc.client.JakEffects;
import dev.jakmc.entity.JakEnemyKind;
import com.google.gson.JsonParser;
import net.minecraft.client.Minecraft;
import net.minecraft.client.multiplayer.*;
import net.minecraft.network.protocol.game.ClientboundTakeItemEntityPacket;
import net.minecraft.world.entity.item.ItemEntity;
import net.minecraft.sounds.*;
import org.spongepowered.asm.mixin.*;
import org.spongepowered.asm.mixin.injection.*;
import org.spongepowered.asm.mixin.injection.callback.CallbackInfo;
@Mixin(ClientPacketListener.class)
abstract class GemCollectMixin {
 @Unique private boolean jakmc$gem;
 @Inject(method="handleTakeItemEntity",at=@At(value="INVOKE",target="Lnet/minecraft/client/multiplayer/ClientLevel;getEntity(I)Lnet/minecraft/world/entity/Entity;",ordinal=0))
 private void jakmc$collect(ClientboundTakeItemEntityPacket packet,CallbackInfo ci){var c=Minecraft.getInstance();jakmc$gem=false;if(c.level==null)return;var from=c.level.getEntity(packet.getItemId());var to=c.level.getEntity(packet.getPlayerId());if(from instanceof ItemEntity item&&item.getItem().is(JakItems.SKULL_GEM)&&to!=null){jakmc$gem=true;JakEffects.gemCollectBursts++;float k=JakEnemyKind.SCALE*6400/4096;String basis="["+k+",0,0,0,"+k+",0,0,0,"+k+"]";JakEffects.local(to.getId(),from.position().add(0,.125,0),JsonParser.parseString("[{\"Spawn\":[\"group-gem-collect\",[0,0,0],"+basis+"]},{\"Step\":0.016666667},{\"Step\":0.016666667},{\"Step\":0.016666667}]").getAsJsonArray());}}
 @Redirect(method="handleTakeItemEntity",at=@At(value="INVOKE",target="Lnet/minecraft/client/multiplayer/ClientLevel;playLocalSound(DDDLnet/minecraft/sounds/SoundEvent;Lnet/minecraft/sounds/SoundSource;FFZ)V"))
 private void jakmc$noSecondPop(ClientLevel level,double x,double y,double z,SoundEvent event,SoundSource source,float gain,float pitch,boolean distance){if(!jakmc$gem)level.playLocalSound(x,y,z,event,source,gain,pitch,distance);}
}
