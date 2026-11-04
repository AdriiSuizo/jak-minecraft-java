package dev.jakmc.client.input;

import com.mojang.blaze3d.platform.Window;
import dev.jakmc.mixin.client.JakContainerBounds;
import dev.jakmc.mixin.client.JakMouseCapture;
import net.minecraft.client.InputType;
import net.minecraft.client.Minecraft;
import net.minecraft.client.gui.screens.Screen;
import net.minecraft.client.gui.components.AbstractWidget;
import net.minecraft.client.gui.screens.inventory.AbstractContainerScreen;
import net.minecraft.client.gui.screens.inventory.MerchantScreen;
import net.minecraft.client.input.KeyEvent;
import net.minecraft.client.input.MouseButtonInfo;
import net.minecraft.world.inventory.Slot;
import org.lwjgl.sdl.SDLMouse;

/** Send controller input through vanilla widget and container mouse handling. */
public final class JakMenuControls {
    private static final int PRIMARY=1,SPLIT=2,QUICK_MOVE=8;
    private static final int UP=SdlGamepad.UP,DOWN=SdlGamepad.DOWN,LEFT=SdlGamepad.LEFT,RIGHT=SdlGamepad.RIGHT;
    private static Screen lastScreen;
    private static int lastHeld,lastDirection,buttonHeld,lastId;
    private static long repeatAt;
    private static MouseButtonInfo button;
    private static boolean cursorMode;

    public static void frame(Minecraft client,SdlGamepad.State pad,float dt,boolean backgroundReplay){
        var screen=client.gui.screen();
        boolean focused=client.isWindowActive();
        if(screen==null||client.gui.overlay()!=null||pad.id()==0||!focused&&!backgroundReplay){
            if(button!=null){
                if(screen!=null&&screen==lastScreen)release(client);
                else forgetPress(client);
            }
            lastScreen=null;lastHeld=lastDirection=lastId=0;cursorMode=false;
            return;
        }
        boolean warp=focused&&!backgroundReplay;
        int held=pad.held();
        if(screen!=lastScreen||lastId!=pad.id()){
            forgetPress(client);
            lastScreen=screen;lastHeld=held;lastDirection=0;lastId=pad.id();
            cursorMode=screen instanceof AbstractContainerScreen<?>;
        }
        var window=client.getWindow();
        double x=client.mouseHandler.getScaledXPos(window),y=client.mouseHandler.getScaledYPos(window);
        float[] motion=ControllerMath.look(pad.leftX(),pad.leftY());
        if(motion[0]==0&&motion[1]==0)motion=ControllerMath.look(pad.rightX(),pad.rightY());
        if(motion[0]!=0||motion[1]!=0){
            double rate=window.getGuiScaledHeight()*1.3*dt;
            move(client,window,x+motion[0]*rate,y-motion[1]*rate,warp);
            x=client.mouseHandler.getScaledXPos(window);y=client.mouseHandler.getScaledYPos(window);
            cursorMode=true;
        }
        int direction=Integer.lowestOneBit(pad.system()&(UP|DOWN|LEFT|RIGHT));
        long now=System.nanoTime();
        if(direction!=0&&(direction!=lastDirection||now>=repeatAt)){
            repeatAt=now+(direction!=lastDirection?280_000_000L:90_000_000L);
            if(screen instanceof AbstractContainerScreen<?> container&&stepSlot(client,container,window,x,y,direction,warp))
                cursorMode=true;
            else if(!(screen instanceof AbstractContainerScreen<?>)){
                client.setLastInputType(InputType.KEYBOARD_ARROW);
                int scan=switch(direction){case UP->82;case DOWN->81;case LEFT->80;default->79;};
                screen.keyPressed(new KeyEvent(scan,scan|1<<30,0));
                cursorMode=false;
            }
        }
        lastDirection=direction;
        int edges=held&~lastHeld;
        if((edges&(16|64))!=0)client.mouseHandler.onScroll(window.handle(),0,(edges&16)!=0?1:-1);
        if(button!=null&&(held&buttonHeld)==0)release(client);
        int action=Integer.lowestOneBit(edges&(PRIMARY|SPLIT|QUICK_MOVE));
        if(button==null&&action!=0){
            if(!cursorMode&&action==PRIMARY){
                client.getFramerateLimitTracker().onInputReceived();
                screen.keyPressed(new KeyEvent(40,13,0));
            }else{
                button=new MouseButtonInfo(action==SPLIT?3:1,action==QUICK_MOVE?1:0);
                buttonHeld=action;
                client.mouseHandler.onButton(window.handle(),button,1);
                if(client.gui.screen()!=screen)forgetPress(client);
            }
        }
        lastHeld=held;
    }
    private static boolean stepSlot(Minecraft client,AbstractContainerScreen<?> screen,Window window,
            double x,double y,int direction,boolean warp){
        var bounds=(JakContainerBounds)screen;
        int left=bounds.jakmc$left(),top=bounds.jakmc$top();
        double bestX=0,bestY=0,nearest=Double.POSITIVE_INFINITY;
        for(Slot slot:screen.getMenu().slots){
            if(!slot.isActive()||slot.x<0||slot.y<0)continue;
            double targetX=left+slot.x+8,targetY=top+slot.y+8;
            double distance=score(targetX-x,targetY-y,direction);
            if(distance<nearest){nearest=distance;bestX=targetX;bestY=targetY;}
        }
        if(screen instanceof MerchantScreen)for(var child:screen.children()){
            if(!(child instanceof AbstractWidget widget)||!widget.visible||!widget.active)continue;
            double targetX=widget.getX()+widget.getWidth()*.5,targetY=widget.getY()+widget.getHeight()*.5;
            double distance=score(targetX-x,targetY-y,direction);
            if(distance<nearest){nearest=distance;bestX=targetX;bestY=targetY;}
        }
        if(!Double.isFinite(nearest))return false;
        move(client,window,bestX,bestY,warp);
        return true;
    }
    private static double score(double dx,double dy,int direction){
        double forward=switch(direction){case UP->-dy;case DOWN->dy;case LEFT->-dx;default->dx;};
        if(forward<9)return Double.POSITIVE_INFINITY;
        double side=direction==UP||direction==DOWN?dx:dy;
        return forward*forward+side*side*4;
    }
    private static void move(Minecraft client,Window window,double x,double y,boolean warp){
        double rawX=Math.clamp(x,0,window.getGuiScaledWidth()-1)*window.getScreenWidth()/window.getGuiScaledWidth();
        double rawY=Math.clamp(y,0,window.getGuiScaledHeight()-1)*window.getScreenHeight()/window.getGuiScaledHeight();
        // Only a real foreground play session moves the OS cursor. Dev replay stays window-local.
        if(warp)SDLMouse.SDL_WarpMouseInWindow(window.handle(),(float)rawX,(float)rawY);
        client.mouseHandler.onMove(window.handle(),rawX,rawY,0,0);
        client.getFramerateLimitTracker().onInputReceived();
    }
    private static void release(Minecraft client){
        client.mouseHandler.onButton(client.getWindow().handle(),button,0);
        button=null;buttonHeld=0;
    }
    private static void forgetPress(Minecraft client){
        if(button!=null)((JakMouseCapture)client.mouseHandler).jakmc$activeButton(null);
        button=null;buttonHeld=0;
    }
    private JakMenuControls(){}
}
