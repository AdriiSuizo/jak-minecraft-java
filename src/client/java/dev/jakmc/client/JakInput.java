package dev.jakmc.client;

import com.mojang.blaze3d.platform.InputConstants;
import dev.jakmc.JakData;
import dev.jakmc.client.input.JakMenuControls;
import dev.jakmc.network.JakPackets;
import net.fabricmc.fabric.api.client.keymapping.v1.KeyMappingHelper;
import net.fabricmc.fabric.api.client.event.lifecycle.v1.ClientTickEvents;
import net.fabricmc.fabric.api.client.networking.v1.ClientPlayNetworking;
import net.minecraft.client.*;
import net.minecraft.core.component.DataComponents;
import net.minecraft.world.item.*;
import dev.jakmc.content.JakItems;
import dev.jakmc.player.JakPlayers;

public final class JakInput {
    private static final KeyMapping.Category CATEGORY=KeyMapping.Category.register(JakData.id("controls"));
    private static KeyMapping key(String name,int code){return KeyMappingHelper.registerKeyMapping(new KeyMapping("key.jakmc."+name,InputConstants.Type.KEYBOARD,code,CATEGORY));}
    public static final KeyMapping BOARD=key("board",InputConstants.KEY_B);
    public static final KeyMapping PUNCH=key("punch",InputConstants.KEY_G);
    public static final KeyMapping SPIN=key("spin",InputConstants.KEY_V);
    public static final KeyMapping DARK=key("dark",InputConstants.KEY_Z);
    public static final KeyMapping LIGHT=key("light",InputConstants.KEY_X);
    public static final KeyMapping AIM=key("aim",InputConstants.KEY_C);
    public static final KeyMapping FIRE=key("fire",InputConstants.KEY_R);
    public static final KeyMapping INTERACT=key("interact",InputConstants.KEY_F);
    private static int previous,sequence,physicalHeld,physicalSystem,suppressed,pressedPad,systemEdges;
    private static final int TRIANGLE=8,L2=32,R1=64,R2=128,L3=256,R3=512;
    private static final float DEFAULT_ZOOM=3.84f,FIRST_PERSON_ZOOM=.384f;
    private static float cameraZoom=DEFAULT_ZOOM;
    private static boolean cameraInitialized;
    private enum Trigger { BOARD, FIRE, USE, PLACE, MINE, RESERVED }
    private static long lastFrame,dropStarted;
    private static boolean droppedStack;
    private static int controllerId;
    private static boolean useClicked,attackClicked,useHeld,attackHeld,aiming,cancelPower,vehicleUseLock;
    private static Trigger trigger;
    private static final dev.jakmc.client.input.SdlGamepad GAMEPAD=new dev.jakmc.client.input.SdlGamepad();
    private static dev.jakmc.client.input.SdlGamepad.State pad=dev.jakmc.client.input.SdlGamepad.State.EMPTY,uiPad=pad;
    public static dev.jakmc.client.input.SdlGamepad.State controller(){return pad;}
    private static boolean inCar(Minecraft c){return c.player!=null&&c.player.getVehicle() instanceof dev.jakmc.entity.JakVehicle;}
    private static boolean enabled(Minecraft c){return c.player!=null&&(JakPlayers.active(c.player)||inCar(c)&&c.player.getAttachedOrCreate(dev.jakmc.JakData.ENABLED))&&!c.player.isSleeping();}
    private static boolean blocked(Minecraft c){
        return !enabled(c)||c.gui.screen()!=null||(!c.isWindowActive()&&!InputScene.backgroundPad(controllerId));
    }
    /** Poll before the frame and the tick, but integrate look only once per rendered frame. */
    public static void frame(Minecraft c){
        long now=System.nanoTime();
        float dt=lastFrame==0?0:Math.clamp((now-lastFrame)/1e9f,0,.1f);
        lastFrame=now;
        poll(c);
        JakMenuControls.frame(c,uiPad,dt,MenuScene.backgroundPad(controllerId)||InputScene.backgroundPad(controllerId));
        if(!blocked(c)){
            zoom(c,dt);
            var look=dev.jakmc.client.input.ControllerMath.look(pad.rightX(),pad.rightY());
            c.player.turn(look[0]*180*dt/.15,-look[1]*180*dt/.15);
        }
    }

    public static float cameraZoomFactor(){return cameraZoom==0?1:cameraZoom/DEFAULT_ZOOM;}

    private static void zoom(Minecraft client,float dt){
        if((uiPad.held()&R3)==0)return;
        boolean up=(uiPad.system()&dev.jakmc.client.input.SdlGamepad.UP)!=0;
        boolean down=(uiPad.system()&dev.jakmc.client.input.SdlGamepad.DOWN)!=0;
        if(up==down)return;
        float distance=client.options.getCameraType().isFirstPerson()?0:cameraZoom==0?DEFAULT_ZOOM:cameraZoom;
        cameraZoom=Math.clamp(Math.max(FIRST_PERSON_ZOOM,distance)+(up?-1:1)*3.84f*dt,0,DEFAULT_ZOOM*2);
        if(cameraZoom<FIRST_PERSON_ZOOM){
            cameraZoom=0;
            client.options.setCameraType(CameraType.FIRST_PERSON);
        }else client.options.setCameraType(CameraType.THIRD_PERSON_BACK);
    }

    private static Trigger triggerFor(ItemStack item,boolean board){
        if(board)return Trigger.BOARD;
        if(JakItems.GUNS.contains(item.getItem()))return Trigger.FIRE;
        if(item.getItem() instanceof BlockItem)return Trigger.PLACE;
        if(item.getItem() instanceof BucketItem||item.getItem() instanceof ProjectileWeaponItem
                ||item.getItem() instanceof TridentItem||item.getItem() instanceof ProjectileItem&&!(item.getItem() instanceof ArrowItem)
                ||item.getItem() instanceof FishingRodItem||item.getItem() instanceof BoatItem
                ||item.getItem() instanceof MinecartItem||item.getItem() instanceof SpawnEggItem
                ||item.getItem() instanceof PotionItem)return Trigger.USE;
        if(item.has(DataComponents.TOOL)||item.has(DataComponents.WEAPON))return Trigger.MINE;
        return Trigger.BOARD;
    }

    private static boolean useWhileMounted(Minecraft c){
        if(c.hitResult instanceof net.minecraft.world.phys.EntityHitResult hit){
            return !(hit.getEntity() instanceof dev.jakmc.entity.JakLeaper mount)||mount.riderId()!=c.player.getId();
        }
        if(c.hitResult instanceof net.minecraft.world.phys.BlockHitResult hit
            &&hit.getType()==net.minecraft.world.phys.HitResult.Type.BLOCK){
            var state=c.level.getBlockState(hit.getBlockPos());
            var block=state.getBlock();
            return state.getMenuProvider(c.level,hit.getBlockPos())!=null
                ||block instanceof net.minecraft.world.level.block.DoorBlock
                ||block instanceof net.minecraft.world.level.block.TrapDoorBlock
                ||block instanceof dev.jakmc.content.JakEcoVent
                ||block instanceof dev.jakmc.content.JakKrimsonGate
                ||block instanceof net.minecraft.world.level.block.FenceGateBlock
                ||block instanceof net.minecraft.world.level.block.ButtonBlock
                ||block instanceof net.minecraft.world.level.block.LeverBlock
                ||block instanceof net.minecraft.world.level.block.BedBlock;
        }
        return false;
    }
    private static void poll(Minecraft c){
        if(!enabled(c)){
            pad=uiPad=dev.jakmc.client.input.SdlGamepad.State.EMPTY;
            physicalHeld=physicalSystem=pressedPad=systemEdges=previous=suppressed=0;
            aiming=useHeld=attackHeld=useClicked=attackClicked=false;
            trigger=null;
            return;
        }
        var next=GAMEPAD.poll(InputScene.replayPad());
        uiPad=next;
        if(next.id()!=controllerId){
            cancelPower|=controllerId!=0;
            controllerId=next.id();
            physicalHeld=physicalSystem=0;
            trigger=null;
            if(controllerId!=0)dev.jakmc.JakMod.LOG.info("Jak controller connected/active: {} (SDL {})",next.name(),controllerId);
        }
        int zoomDirections=(next.held()&R3)!=0?60:0;
        systemEdges|=next.system()&~physicalSystem&~zoomDirections;
        if(c.gui.screen()!=null&&(next.held()&4)!=0&&(physicalHeld&4)==0)systemEdges|=64;
        if(blocked(c)){
            suppressed=next.held();
            pressedPad=0;
            aiming=useHeld=attackHeld=useClicked=attackClicked=false;
            cancelPower=true;
            trigger=null;
            pad=dev.jakmc.client.input.SdlGamepad.State.EMPTY;
        }else{
            suppressed&=next.held();
            int held=next.held()&~suppressed;
            int edges=held&~physicalHeld;
            var pose=JakPoses.get(c.player.getId());
            boolean flying=pose!=null&&pose.raw().has("hellcat");
            if((held&TRIANGLE)==0)vehicleUseLock=false;
            if(flying||inCar(c)){
                vehicleUseLock|=(held&TRIANGLE)!=0;
                useHeld=attackHeld=aiming=false;
                useClicked=attackClicked=false;
                trigger=null;
                pressedPad|=edges;
                pad=new dev.jakmc.client.input.SdlGamepad.State(next.id(),next.name(),held,next.system(),
                    next.leftX(),next.leftY(),next.rightX(),next.rightY(),next.brake(),next.throttle());
                physicalHeld=next.held();
                physicalSystem=next.system();
                return;
            }
            boolean board=pose!=null&&pose.board();
            boolean mounted=pose!=null&&pose.raw().get("ride") instanceof com.google.gson.JsonObject;
            boolean powers=(held&L3)!=0||DARK.isDown()||LIGHT.isDown();
            var itemAction=triggerFor(c.player.getMainHandItem(),board);
            boolean busy=c.player.isInWater()||powers||mounted;
            aiming=!board&&!busy&&!c.options.getCameraType().isMirrored()
                &&((held&L2)!=0||AIM.isDown());
            Trigger wanted=busy&&!board?Trigger.RESERVED:itemAction;
            if(wanted==Trigger.BOARD&&aiming)wanted=Trigger.RESERVED;
            boolean rightDown=(held&R2)!=0;
            if(!rightDown)trigger=null;
            else if((edges&R2)!=0)trigger=wanted;
            else if(trigger!=wanted)trigger=Trigger.RESERVED;
            boolean useTrigger=trigger==Trigger.USE||trigger==Trigger.PLACE&&aiming;
            boolean mineTrigger=trigger==Trigger.MINE;
            boolean triangle=(held&TRIANGLE)!=0&&!vehicleUseLock&&!powers&&(!mounted||useWhileMounted(c));
            useHeld=triangle||useTrigger;
            attackHeld=mineTrigger;
            useClicked|=triangle&&(edges&TRIANGLE)!=0||useTrigger&&(edges&R2)!=0;
            attackClicked|=mineTrigger&&(edges&R2)!=0;

            // Only the current owner of R2 can pass that press to the game.
            int reserved=(board?R2:R1|R2)|R3;
            int mapped=held&~reserved;
            int mappedEdges=edges&~reserved;
            if(mounted&&triangle){mapped&=~TRIANGLE;mappedEdges&=~TRIANGLE;}
            int nativeTrigger=trigger==Trigger.FIRE?R1:trigger==Trigger.BOARD?R2:0;
            if(rightDown)mapped|=nativeTrigger;
            if((edges&R2)!=0)mappedEdges|=nativeTrigger;
            if(aiming)mapped|=L2;
            else if(!board)mapped&=~L2;
            pressedPad|=mappedEdges;
            pad=new dev.jakmc.client.input.SdlGamepad.State(next.id(),next.name(),mapped,next.system(),next.leftX(),next.leftY(),next.rightX(),next.rightY(),next.brake(),next.throttle());
        }
        physicalHeld=next.held();
        physicalSystem=next.system();
    }

    public static boolean aiming(){return aiming;}
    public static boolean controllerReady(int id){return GAMEPAD.has(id);}
    public static boolean useHeld(){return useHeld;}
    public static boolean attackHeld(){return attackHeld;}
    public static boolean consumeUse(){boolean value=useClicked;useClicked=false;return value;}
    public static boolean consumeAttack(){boolean value=attackClicked;attackClicked=false;return value;}
    private static void ui(Minecraft c){
        int edges=systemEdges;systemEdges=0;
        if(!enabled(c)||!c.isWindowActive()&&!MenuScene.backgroundPad(controllerId)&&!InputScene.backgroundPad(controllerId)){
            dropStarted=0;droppedStack=false;return;
        }
        if(c.gui.screen()!=null){dropStarted=0;droppedStack=false;if((edges&(dev.jakmc.client.input.SdlGamepad.START|dev.jakmc.client.input.SdlGamepad.BACK|64))!=0)c.gui.screen().onClose();return;}
        if((edges&dev.jakmc.client.input.SdlGamepad.START)!=0){c.pauseGame(false);return;}
        if((edges&(dev.jakmc.client.input.SdlGamepad.BACK|dev.jakmc.client.input.SdlGamepad.UP))!=0){c.gui.setScreen(new net.minecraft.client.gui.screens.inventory.InventoryScreen(c.player));return;}
        if((uiPad.held()&R3)!=0){dropStarted=0;droppedStack=false;return;}
        var inventory=c.player.getInventory();int slot=inventory.getSelectedSlot();
        if((edges&dev.jakmc.client.input.SdlGamepad.LEFT)!=0)slot=(slot+8)%9;
        if((edges&dev.jakmc.client.input.SdlGamepad.RIGHT)!=0)slot=(slot+1)%9;
        inventory.setSelectedSlot(slot);
        boolean down=(uiPad.system()&dev.jakmc.client.input.SdlGamepad.DOWN)!=0&&(uiPad.held()&R3)==0;
        long now=System.nanoTime();
        if(down){
            if(dropStarted==0){dropStarted=now;droppedStack=false;}
            if(!droppedStack&&now-dropStarted>=400_000_000L){c.gameMode.dropItem(c.player,true);droppedStack=true;}
        }else if(dropStarted!=0){
            if(!droppedStack)c.gameMode.dropItem(c.player,false);
            dropStarted=0;droppedStack=false;
        }
    }

    public static void initialize(){
        net.fabricmc.fabric.api.client.networking.v1.ClientPlayConnectionEvents.JOIN.register((handler,sender,client)->{
            cameraInitialized=false;cameraZoom=DEFAULT_ZOOM;
        });
        net.fabricmc.fabric.api.client.event.lifecycle.v1.ClientLifecycleEvents.CLIENT_STOPPING.register(c->GAMEPAD.close());
        ClientTickEvents.END_CLIENT_TICK.register(client->{
            if(!cameraInitialized&&enabled(client)&&JakPoses.get(client.player.getId())!=null){
                cameraInitialized=true;
                if(client.options.getCameraType().isFirstPerson())client.options.setCameraType(CameraType.THIRD_PERSON_BACK);
            }
            poll(client);
            JakMenuControls.frame(client,uiPad,0,MenuScene.backgroundPad(controllerId)||InputScene.backgroundPad(controllerId));
            ui(client);
            if(!enabled(client)){previous=0;return;}
            boolean blocked=blocked(client);int held=blocked?0:held(client)|pad.held(),pressed=(held&~previous)|pressedPad;previous=held;pressedPad=0;
            float strafe=blocked?0:(client.options.keyRight.isDown()?1:0)-(client.options.keyLeft.isDown()?1:0);
            float forward=blocked?0:(client.options.keyUp.isDown()?1:0)-(client.options.keyDown.isDown()?1:0);
            if(pad.leftX()*pad.leftX()+pad.leftY()*pad.leftY()>strafe*strafe+forward*forward){strafe=pad.leftX();forward=pad.leftY();}
            int viewFlags=client.options.getCameraType().isFirstPerson()?JakPackets.FIRST_PERSON:0;
            if(blocked||cancelPower)viewFlags|=JakPackets.CANCEL_POWER;
            cancelPower=false;
            var pose=JakPoses.get(client.player.getId());
            boolean flying=pose!=null&&pose.raw().has("hellcat");
            float brake=blocked?0:pad.brake(),throttle=blocked?0:pad.throttle();
            if((flying||inCar(client))&&!blocked&&pad.id()==0){
                brake=client.options.keyDown.isDown()?1:0;
                throttle=client.options.keyUp.isDown()?1:0;
                forward=0;
            }
            if(ClientPlayNetworking.canSend(JakPackets.Input.TYPE))ClientPlayNetworking.send(new JakPackets.Input(++sequence,held|viewFlags,pressed,strafe,forward,pad.rightX(),pad.rightY(),client.player.getYRot(),client.player.getXRot(),brake,throttle));
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
        if(DARK.isDown())b|=L3;
        if(LIGHT.isDown())b|=L3|1;
        if(FIRE.isDown())b|=64;
        if(BOARD.isDown())b|=128;
        return b;
    }
}
