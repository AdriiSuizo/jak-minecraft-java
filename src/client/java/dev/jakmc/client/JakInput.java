package dev.jakmc.client;

import com.mojang.blaze3d.platform.InputConstants;
import dev.jakmc.JakData;
import dev.jakmc.network.JakPackets;
import net.fabricmc.fabric.api.client.keymapping.v1.KeyMappingHelper;
import net.fabricmc.fabric.api.client.event.lifecycle.v1.ClientTickEvents;
import net.fabricmc.fabric.api.client.networking.v1.ClientPlayNetworking;
import net.minecraft.client.*;

public final class JakInput {
    private static final KeyMapping.Category CATEGORY=KeyMapping.Category.register(JakData.id("controls"));
    private static KeyMapping key(String name,int code){return KeyMappingHelper.registerKeyMapping(new KeyMapping("key.jakmc."+name,InputConstants.Type.KEYBOARD,code,CATEGORY));}
    public static final KeyMapping BOARD=key("board",InputConstants.KEY_B);
    public static final KeyMapping PUNCH=key("punch",InputConstants.KEY_G);
    public static final KeyMapping SPIN=key("spin",InputConstants.KEY_V);
    public static final KeyMapping DARK=key("dark",InputConstants.KEY_Z);
    public static final KeyMapping LIGHT=key("light",InputConstants.KEY_X);
    public static final KeyMapping FIRE=key("fire",InputConstants.KEY_R);
    public static final KeyMapping INTERACT=key("interact",InputConstants.KEY_F);
    private static int previous,sequence;
    public static void initialize(){
        ClientTickEvents.END_CLIENT_TICK.register(client->{
            if(client.player==null||!client.player.getAttachedOrCreate(JakData.ENABLED)){previous=0;return;}
            int held=held(client),pressed=held&~previous;previous=held;
            float strafe=client.gui.screen()==null?(client.options.keyRight.isDown()?1:0)-(client.options.keyLeft.isDown()?1:0):0;
            float forward=client.gui.screen()==null?(client.options.keyUp.isDown()?1:0)-(client.options.keyDown.isDown()?1:0):0;
            if(ClientPlayNetworking.canSend(JakPackets.Input.TYPE))ClientPlayNetworking.send(new JakPackets.Input(++sequence,held,pressed,strafe,forward,client.player.getYRot(),client.player.getXRot(),0,0));
        });
    }
    static int held(Minecraft client){
        if(client.gui.screen()!=null)return 0;
        int b=0;
        if(client.options.keyJump.isDown())b|=1;
        if(PUNCH.isDown())b|=2;
        if(SPIN.isDown())b|=4;
        if(INTERACT.isDown())b|=8;
        if(client.options.keyShift.isDown())b|=16;
        if(DARK.isDown())b|=256;
        if(LIGHT.isDown())b|=512;
        if(FIRE.isDown())b|=64;
        if(BOARD.isDown())b|=128;
        return b;
    }
}
