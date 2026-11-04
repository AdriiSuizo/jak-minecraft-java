package dev.jakmc.mixin;

import dev.jakmc.player.BoardSlot;
import net.minecraft.network.protocol.PacketUtils;
import net.minecraft.network.protocol.game.ServerboundSetCreativeModeSlotPacket;
import net.minecraft.server.level.ServerPlayer;
import net.minecraft.server.network.ServerGamePacketListenerImpl;
import org.spongepowered.asm.mixin.*;
import org.spongepowered.asm.mixin.injection.*;
import org.spongepowered.asm.mixin.injection.callback.CallbackInfo;

@Mixin(ServerGamePacketListenerImpl.class)
abstract class BoardCreativeMixin {
    @Shadow public ServerPlayer player;
    @Inject(method="handleSetCreativeModeSlot",at=@At("HEAD"),cancellable=true)
    private void jakmc$creativeBoard(ServerboundSetCreativeModeSlotPacket packet,CallbackInfo ci){
        if(packet.slotNum()<=45)return;
        PacketUtils.ensureRunningOnSameThread(packet,(ServerGamePacketListenerImpl)(Object)this,player.level());
        int index=BoardSlot.index(player.inventoryMenu);
        if(packet.slotNum()!=index)return;
        ci.cancel();
        var stack=packet.itemStack();var slot=player.inventoryMenu.getSlot(index);
        if(!player.hasInfiniteMaterials()||!stack.isItemEnabled(player.level().enabledFeatures())
            ||!stack.isEmpty()&&(stack.getCount()!=1||!slot.mayPlace(stack)))return;
        slot.setByPlayer(stack);player.inventoryMenu.setRemoteSlot(index,stack);player.inventoryMenu.broadcastChanges();
    }
}
