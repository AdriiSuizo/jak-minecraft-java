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
    private static int previous,sequence,physicalHeld,physicalSystem,suppressed,pressedPad,systemEdges;
    private static long lastPoll;private static int controllerId;private static boolean useClicked;
    private static final dev.jakmc.client.input.SdlGamepad GAMEPAD=new dev.jakmc.client.input.SdlGamepad();
    private static dev.jakmc.client.input.SdlGamepad.State pad=dev.jakmc.client.input.SdlGamepad.State.EMPTY;
    public static dev.jakmc.client.input.SdlGamepad.State controller(){return pad;}
    private static boolean enabled(Minecraft c){return c.player!=null&&c.player.getAttachedOrCreate(JakData.ENABLED);}
    private static boolean blocked(Minecraft c){return !enabled(c)||c.gui.screen()!=null||!c.isWindowActive();}
    /** Called before camera extraction, so the view and movement packet use the same look. */
    public static void frame(Minecraft c){
        long now=System.nanoTime();float dt=lastPoll==0?0:Math.clamp((now-lastPoll)/1e9f,0,.1f);lastPoll=now;
        if(!enabled(c)){pad=dev.jakmc.client.input.SdlGamepad.State.EMPTY;physicalHeld=physicalSystem=pressedPad=systemEdges=previous=0;useClicked=false;return;}
        var next=GAMEPAD.poll();if(next.id()!=controllerId){controllerId=next.id();if(controllerId!=0)dev.jakmc.JakMod.LOG.info("Jak controller connected/active: {} (SDL {})",next.name(),controllerId);}
        systemEdges|=next.system()&~physicalSystem;
        if(c.gui.screen()!=null&&(next.held()&4)!=0&&(physicalHeld&4)==0)systemEdges|=64;
        if(blocked(c)){suppressed=next.held();pressedPad=0;useClicked=false;pad=dev.jakmc.client.input.SdlGamepad.State.EMPTY;}
        else{
            suppressed&=next.held();int held=next.held()&~suppressed;pressedPad|=held&~physicalHeld;
            useClicked|=(held&8)!=0&&(physicalHeld&8)==0;
            pad=new dev.jakmc.client.input.SdlGamepad.State(next.id(),next.name(),held,next.system(),next.leftX(),next.leftY(),next.rightX(),next.rightY(),next.brake(),next.throttle());
            var look=dev.jakmc.client.input.ControllerMath.look(pad.rightX(),pad.rightY());
            // Entity.turn applies Minecraft's .15 degree/mouse-unit factor.
            c.player.turn(look[0]*180*dt/.15,-look[1]*180*dt/.15);
        }
        physicalHeld=next.held();physicalSystem=next.system();
    }
    public static boolean controllerReady(int id){return GAMEPAD.has(id);}
    public static boolean useHeld(){return (pad.held()&8)!=0;}
    public static boolean consumeUse(){boolean value=useClicked;useClicked=false;return value;}
    private static void ui(Minecraft c){
        int edges=systemEdges;systemEdges=0;if(!enabled(c)||!c.isWindowActive())return;
        if(c.gui.screen()!=null){if((edges&(dev.jakmc.client.input.SdlGamepad.START|dev.jakmc.client.input.SdlGamepad.BACK|64))!=0)c.gui.screen().onClose();return;}
        if((edges&dev.jakmc.client.input.SdlGamepad.START)!=0){c.pauseGame(false);return;}
        if((edges&dev.jakmc.client.input.SdlGamepad.BACK)!=0)c.options.setCameraType(c.options.getCameraType().cycle());
        if((edges&dev.jakmc.client.input.SdlGamepad.UP)!=0){c.gui.setScreen(new net.minecraft.client.gui.screens.inventory.InventoryScreen(c.player));return;}
        var inventory=c.player.getInventory();int slot=inventory.getSelectedSlot();
        if((edges&dev.jakmc.client.input.SdlGamepad.LEFT)!=0)slot=(slot+8)%9;
        if((edges&dev.jakmc.client.input.SdlGamepad.RIGHT)!=0)slot=(slot+1)%9;
        inventory.setSelectedSlot(slot);
        if((edges&dev.jakmc.client.input.SdlGamepad.DOWN)!=0)c.gameMode.dropItem(c.player,false);
    }

    public static void initialize(){
        net.fabricmc.fabric.api.client.event.lifecycle.v1.ClientLifecycleEvents.CLIENT_STOPPING.register(c->GAMEPAD.close());
        ClientTickEvents.END_CLIENT_TICK.register(client->{
            frame(client);ui(client);
            if(!enabled(client)){previous=0;return;}
            boolean blocked=blocked(client);int held=blocked?0:held(client)|pad.held(),pressed=(held&~previous)|pressedPad;previous=held;pressedPad=0;
            float strafe=blocked?0:(client.options.keyRight.isDown()?1:0)-(client.options.keyLeft.isDown()?1:0);
            float forward=blocked?0:(client.options.keyUp.isDown()?1:0)-(client.options.keyDown.isDown()?1:0);
            if(pad.leftX()*pad.leftX()+pad.leftY()*pad.leftY()>strafe*strafe+forward*forward){strafe=pad.leftX();forward=pad.leftY();}
            if(ClientPlayNetworking.canSend(JakPackets.Input.TYPE))ClientPlayNetworking.send(new JakPackets.Input(++sequence,held,pressed,strafe,forward,pad.rightX(),pad.rightY(),client.player.getYRot(),client.player.getXRot(),blocked?0:pad.brake(),blocked?0:pad.throttle()));
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
