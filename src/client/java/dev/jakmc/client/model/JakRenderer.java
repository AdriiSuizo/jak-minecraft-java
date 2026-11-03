package dev.jakmc.client.model;

import dev.jakmc.*;
import dev.jakmc.client.*;
import dev.jakmc.content.JakItems;
import com.mojang.blaze3d.vertex.PoseStack;
import com.mojang.math.Axis;
import net.minecraft.client.Minecraft;
import net.minecraft.client.renderer.SubmitNodeCollector;
import net.minecraft.client.renderer.entity.state.AvatarRenderState;
import net.minecraft.client.renderer.texture.OverlayTexture;
import org.joml.Matrix4f;
import org.joml.Vector3f;
import java.io.IOException;

public final class JakRenderer {
    private static final String[] FIRE_BODY={"jakb-gun-red-fire","jakb-gun-red-fire-2","jakb-gun-red-fire","jakb-gun-yellow-fire","jakb-gun-yellow-fire","jakb-gun-yellow-fire-3","jakb-gun-blue-fire-single","jakb-gun-blue-fire-2","jakb-gun-blue-fire-single","jakb-gun-dark-fire","jakb-gun-dark-fire","jakb-gun-dark-fire"};
    private static final String[] FIRE_GUN={"gun-gun-red-fire","gun-gun-red-fire-2","gun-gun-red-fire-3","gun-gun-yellow-fire","gun-gun-yellow-fire-2","gun-gun-yellow-fire-3","gun-gun-blue-fire-single","gun-gun-blue-fire-2","gun-gun-blue-fire-3-single","gun-gun-dark-fire","gun-gun-dark-fire-2","gun-gun-dark-fire-3"};
    private static GltfModel body,daxter,board,gun,wings,shield;
    private static float bodyScale;
    private static float floorOffset;
    private static boolean ridingLoaded,pilotLoaded;
    private static GltfModel flutRider;
    private static GltfModel.Posed flutRiderRest,jakRest;
    public static long renderSerial;
    public static int lastEntityId=-1;
    public static float standingHeight;
    public static float currentTop=Float.NaN;
    private static void load(){
        if(body!=null)return;
        try{
            var root=RuntimeConfig.assets.resolve("levels/common");
            body=new GltfModel(root.resolve("jakb-c-lod0.glb"),"jak");
            var clips=new GltfModel(root.resolve("jakb-lod0.glb"),"jak_clips");
            body.animationsFrom(clips);
            wings=new GltfModel(root.resolve("jakb-wings-wings-lod0.glb"),"light_wings");
            wings.attachmentAnimationsFrom(clips,"jakb-wings-");
            shield=new GltfModel(root.resolve("jakb-shield-shield-lod0.glb"),"light_shield");
            shield.attachmentAnimationsFrom(clips,"jakb-shield-");
            daxter=new GltfModel(root.resolve("daxter-lod0.glb"),"daxter");
            var standing=body.pose("jakb-stance-loop",0);
            Vector3f anchor=body.joint(standing,"LshoulderPad").getTranslation(new Vector3f());
            var bodyBounds=body.bounds(standing,new Matrix4f());
            var daxterBounds=daxter.bounds(daxter.pose("daxter-stance-loop",0),new Matrix4f().translation(anchor).scale(1/1.4f));
            float together=Math.max(bodyBounds[1].y,daxterBounds[1].y)-Math.min(bodyBounds[0].y,daxterBounds[0].y);
            floorOffset=-Math.min(bodyBounds[0].y,daxterBounds[0].y);
            // User scale reference: two air cells, ceiling in the third row.
            bodyScale=1.94f/together;
            standingHeight=together*bodyScale;
            JakMod.LOG.info("Jak + Daxter original height {} m; Minecraft scale {}; standing ears at 1.94 blocks",together,bodyScale);
            boardModel();
            gun=new GltfModel(root.resolve("gun-lod0.glb"),"gun");
            JakMod.LOG.info("Original Jak, Daxter, board and Morph Gun models loaded into Minecraft renderer");
        }catch(IOException e){throw new IllegalStateException("Cannot load original Jak models",e);}
    }
    static GltfModel boardModel(){
        if(board==null){
            try{board=new GltfModel(RuntimeConfig.assets.resolve("levels/common/board-lod0.glb"),"board");}
            catch(IOException e){throw new java.io.UncheckedIOException("Cannot load the original JET-Board",e);}
        }
        return board;
    }

    public static boolean submit(AvatarRenderState state,PoseStack stack,SubmitNodeCollector collector){
        var pose=JakPoses.get(state.id);
        var client=Minecraft.getInstance();if(client.level==null)return false;
        var actor=client.level.getEntity(state.id);
        if(actor==null||!actor.getAttachedOrCreate(JakData.ENABLED))return false;
        if(actor.getVehicle() instanceof dev.jakmc.entity.JakVehicle car&&car.frame()!=null){
            submitCarRider(state,stack,collector,car,client.getDeltaTracker().getGameTimeDeltaPartialTick(false));
            return true;
        }
        if(pose==null)return false;
        load();
        renderSerial++;lastEntityId=state.id;
        var flight=pose.raw().get("hellcat") instanceof com.google.gson.JsonObject craft?craft:null;
        stack.pushPose();
        stack.rotateDegrees(Axis.YP,-pose.yaw());
        if(flight!=null){
            var hull=Minecraft.getInstance().level.getEntity(flight.get("entity").getAsInt());
            if(hull instanceof dev.jakmc.entity.JakHellcat cat&&flight.get("boarding").getAsFloat()<0){
                var bank=new org.joml.Quaternionf().rotateY((float)Math.toRadians(pose.yaw())).mul(cat.attitude());
                stack.mulPose(new org.joml.Matrix4f().rotation(bank));
            }
            loadPilot();
        }
        stack.scale(bodyScale,bodyScale,bodyScale);stack.translate(0,floorOffset,0);
        float elapsed=Math.clamp((System.nanoTime()-pose.received())/1e9f,0,.05f);
        float frame=pose.frame()+(pose.raw().has("frameRate")?pose.raw().get("frameRate").getAsFloat()*elapsed:0);
        var ride=pose.raw().get("ride");
        boolean mounted=ride!=null&&ride.isJsonObject();
        int frames=body.frames(pose.clip());if(pose.raw().has("loop")&&pose.raw().get("loop").getAsBoolean())frame%=frames;
        boolean flut=mounted&&ride.getAsJsonObject().has("flut")&&ride.getAsJsonObject().get("flut").getAsBoolean();
        if(mounted)loadRide(flut);
        var skeleton=flut?body.retargetRider(flutRider,flutRiderRest,
                JakLeaperRenderer.ridePose(flutRider,"jakb1-",pose,elapsed),jakRest):
            mounted?JakLeaperRenderer.ridePose(body,"jakb-",pose,elapsed):body.pose(pose.clip(),frame);
        if(flight!=null)skeleton=pilotPose(body,"jakb",flight,elapsed);
        int gunKind=JakItems.GUNS.indexOf(state.getMainHandItemStack().getItem());
        var gunState=pose.raw().getAsJsonObject("gun");
        boolean gunOut=gunState!=null&&gunState.get("out").getAsBoolean();
        boolean shieldUp=pose.raw().has("shield")&&pose.raw().get("shield").getAsBoolean();
        if(!mounted&&gunKind>=0&&gunOut&&!shieldUp&&(pose.clip().contains("stance")||pose.clip().contains("walk")||pose.clip().contains("jump"))&&!pose.dark()){
            String upper=switch(gunKind/3){case 0->"jakb-gun-stance";case 1->"jakb-gun-stance-yellow";case 2->"jakb-gun-stance-blue";default->"jakb-gun-stance-dark";};
            float age=0;var g=pose.raw().getAsJsonObject("gun");
            if(g!=null&&g.get("out").getAsBoolean()){
                float fire=g.get("fireAge").getAsFloat()+elapsed,draw=g.get("drawAge").getAsFloat()+elapsed;
                String take="jakb-gun-"+new String[]{"red","yellow","blue","dark"}[gunKind/3]+"-takeout";
                if(draw>=0&&draw<body.duration(take)){upper=take;age=draw;}
                else if(fire>=0&&fire<.45f){upper=FIRE_BODY[gunKind];age=fire;}
                else if(pose.clip().contains("walk")){
                    boolean run=g.get("runMix").getAsFloat()>.5f;upper=gunKind/3==2?(run?"jakb-gun-run-blue":"jakb-gun-walk-blue"):(run?"jakb-gun-front-run":"jakb-gun-front-walk");
                    age=frame/Math.max(1,frames-1)*body.duration(upper);
                }else age=(g.get("time").getAsFloat()+elapsed)%Math.max(.001f,body.duration(upper));
            }
            skeleton=body.overlayUpper(skeleton,upper,age);
        }
        boolean aiming=pose.raw().has("aim")&&pose.raw().get("aim").getAsBoolean();
        float lookYaw=pose.raw().has("lookYaw")?pose.raw().get("lookYaw").getAsFloat():pose.yaw();
        if(actor==client.player)lookYaw=actor.getYRot();
        float twist=aiming?Math.clamp((float)Math.toRadians(net.minecraft.util.Mth.wrapDegrees(pose.yaw()-lookYaw)),-1.6f,1.6f):0;
        skeleton=body.jakForm(skeleton,pose.dark(),twist);
        boolean luminous=pose.raw().has("lightInterp")&&pose.raw().get("lightInterp").getAsFloat()>0;
        boolean asleep=actor instanceof net.minecraft.world.entity.LivingEntity living&&living.isSleeping();
        body.submitJakBody(skeleton,stack,collector,state.lightCoords,pose.dark(),luminous,asleep);
        var wingState=pose.raw().get("wings");
        if(wingState!=null&&wingState.isJsonObject()){
            var wing=wingState.getAsJsonObject();
            String clip=wing.get("clip").getAsString();
            stack.pushPose();
            stack.mulPose(rigid(body.joint(skeleton,"chest")));
            float wingFrame=wing.get("frame").getAsFloat()+wing.get("frameRate").getAsFloat()*elapsed;
            if(wing.get("loop").getAsBoolean())wingFrame%=wings.frames(clip);
            wings.submitLightEffect(wings.pose(clip,wingFrame),stack,collector,LightFormTexture.Form.WINGS,1);
            stack.popPose();
        }
        drawShield(state,pose,skeleton,stack,collector,shieldUp,elapsed);
        stack.pushPose();
        Vector3f shoulder=body.joint(skeleton,"LshoulderPad").getTranslation(new Vector3f());
        stack.translate(shoulder.x,shoulder.y,shoulder.z);stack.scale(1/1.4f,1/1.4f,1/1.4f);
        String daxClip=pose.clip().replace("jakb-","daxter-");
        if(!daxter.hasClip(daxClip))daxClip="daxter-stance-loop";
        var daxPose=flight!=null?pilotPose(daxter,"daxter",flight,elapsed):mounted?JakLeaperRenderer.ridePose(daxter,"daxter-",pose,elapsed):daxter.pose(daxClip,frame);
        if(ScaleScene.active()){
            var b=body.bounds(skeleton,new Matrix4f());
            var d=daxter.bounds(daxPose,new Matrix4f().translation(shoulder).scale(1/1.4f));
            currentTop=(Math.max(b[1].y,d[1].y)+floorOffset)*bodyScale;
        }
        daxter.submitEyes(daxPose,stack,collector,state.lightCoords,asleep);stack.popPose();
        if(pose.board()){
            stack.pushPose();stack.mulPose(rigid(body.joint(skeleton,"board")));board.submit(board.poseAtTime("board-open",Float.MAX_VALUE),stack,collector,state.lightCoords);stack.popPose();
        }
        else if(!actor.getAttachedOrCreate(JakData.BOARD).isEmpty()){
            stack.pushPose();stack.mulPose(rigid(body.joint(skeleton,"board")));
            board.submit(board.poseAtTime("board-close",Float.MAX_VALUE),stack,collector,state.lightCoords);stack.popPose();
        }
        if(!mounted&&flight==null){hand(state,true,skeleton,stack,collector);hand(state,false,skeleton,stack,collector);}
        stack.popPose();return true;
    }
    private static void submitCarRider(AvatarRenderState state,PoseStack stack,SubmitNodeCollector collector,
        dev.jakmc.entity.JakVehicle car,float partial){
        load();
        loadPilot();
        var before=car.previousFrame();
        var frame=car.frame();
        var controls=frame.deepCopy();
        controls.addProperty("steering",net.minecraft.util.Mth.lerp(partial,before.get("steer").getAsFloat(),frame.get("steer").getAsFloat()));
        float age=frame.get("boarding").getAsFloat();
        var skeleton=body.jakForm(carPilotPose(body,"jakb",controls,age<0?0:partial/20,car.model),false,0);
        var passenger=carPilotPose(daxter,"daxter",controls,age<0?0:partial/20,car.model);
        var attitude=JakVehicleRenderer.rotation(before).slerp(JakVehicleRenderer.rotation(frame),partial);
        stack.pushPose();
        stack.mulPose(new Matrix4f().rotation(attitude));
        stack.scale(bodyScale,bodyScale,bodyScale);
        stack.translate(0,floorOffset,0);
        body.submitJakBody(skeleton,stack,collector,state.lightCoords,false,false,false);
        Vector3f shoulder=body.joint(skeleton,"LshoulderPad").getTranslation(new Vector3f());
        stack.translate(shoulder.x,shoulder.y,shoulder.z);
        stack.scale(1/1.4f,1/1.4f,1/1.4f);
        daxter.submit(passenger,stack,collector,state.lightCoords);
        stack.popPose();
        renderSerial++;
        lastEntityId=state.id;
    }
    private static void loadPilot(){
        if(pilotLoaded)return;
        var art=com.google.gson.JsonParser.parseString(dev.jakmc.nativebridge.NativeJak.ridingAnimations()).getAsJsonObject();
        body.originalAnimations(art,"jakb-pilot-car-");
        daxter.originalAnimations(art,"daxter-pilot-car-");
        body.originalAnimations(art,"jakb-pilot-wcar-");
        body.originalAnimations(art,"jakb-pilot-bike-");
        daxter.originalAnimations(art,"daxter-pilot-bike-");
        pilotLoaded=true;
    }

    private static GltfModel.Posed carPilotPose(GltfModel model,String actor,com.google.gson.JsonObject controls,float elapsed,dev.jakmc.entity.JakVehicle.Model vehicle){
        float boarding=controls.get("boarding").getAsFloat();
        if(vehicle.bike()&&boarding>=0)return model.poseAtTime(actor+"-pilot-bike-get-on",boarding+elapsed);
        if(boarding>=0)return pilotPose(model,actor,controls,elapsed);
        float turn=(1+controls.get("steering").getAsFloat());
        float lean=controls.get("lean").getAsFloat();
        float thrust=controls.get("throttle").getAsFloat();
        String prefix=actor+(vehicle.bike()?"-pilot-bike-":actor.equals("jakb")&&!vehicle.hover()?"-pilot-wcar-":"-pilot-car-");
        if(vehicle.bike())turn=2-turn;
        var back=model.pose(prefix+"turn-back",turn);
        var front=model.pose(prefix+"turn-front",turn);
        var vertical=model.pose(actor+(vehicle.bike()?"-pilot-bike-up-down":"-pilot-car-up-down"),2.5f*(1-lean));
        return model.blend(model.blend(back,front,.5f*(1+thrust)),vertical,Math.abs(lean));
    }

    private static GltfModel.Posed pilotPose(GltfModel model,String actor,com.google.gson.JsonObject flight,float elapsed){
        String prefix=actor+"-pilot-car-";
        float boarding=flight.get("boarding").getAsFloat();
        if(boarding>=0)return model.poseAtTime(prefix+"get-on",boarding+elapsed);
        float turn=(1-flight.get("steering").getAsFloat())*.5f;
        float lean=flight.get("lean").getAsFloat();
        float front=Math.clamp(.5f*(1+1.42f*(flight.get("throttle").getAsFloat()-.3f)),0,1);
        var back=sample(model,prefix+"turn-back",turn);
        var ahead=sample(model,prefix+"turn-front",turn);
        var vertical=sample(model,prefix+"up-down",(1-lean)*.5f);
        return model.blend(model.blend(back,ahead,front),vertical,Math.abs(lean));
    }

    private static GltfModel.Posed sample(GltfModel model,String clip,float fraction){
        return model.pose(clip,Math.clamp(fraction,0,1)*(model.frames(clip)-1));
    }

    private static void loadRide(boolean flut){
        if(ridingLoaded&&(!flut||flutRider!=null))return;
        var art=com.google.gson.JsonParser.parseString(dev.jakmc.nativebridge.NativeJak.ridingAnimations()).getAsJsonObject();
        if(!ridingLoaded){
            body.originalAnimations(art,"jakb-flut-");
            daxter.originalAnimations(art,"daxter-flut-");
            if(!body.hasClip("jakb-flut-get-on")||!daxter.hasClip("daxter-flut-get-on")){
                throw new IllegalStateException("Original riding joint count did not match the Jak and Daxter rigs");
            }
            ridingLoaded=true;
        }
        if(flut&&flutRider==null){
            try{
                var model=new GltfModel(FlutMaterial.jak1().resolve("levels/common/eichar-lod0.glb"),"original_flut_rider");
                model.originalAnimations(art,"jakb1-flut-");
                if(!model.hasClip("jakb1-flut-get-on")){
                    throw new IllegalStateException("Original Flut Flut riding animations are unavailable");
                }
                flutRiderRest=model.pose("eichar-stance-loop",0);
                jakRest=body.pose("jakb-stance-loop",0);
                flutRider=model;
            }catch(IOException failure){throw new java.io.UncheckedIOException(failure);}
        }
    }
    private record ShieldTiming(boolean open,float changed){}
    private static final java.util.Map<net.minecraft.world.entity.Entity,ShieldTiming> SHIELDS=new java.util.WeakHashMap<>();

    private static void drawShield(AvatarRenderState state,JakPoses.Pose pose,GltfModel.Posed skeleton,
                                   PoseStack stack,SubmitNodeCollector collector,boolean open,float elapsed){
        float time=pose.raw().getAsJsonObject("gun").get("time").getAsFloat()+elapsed;
        var actor=Minecraft.getInstance().level.getEntity(state.id);
        var timing=SHIELDS.get(actor);
        if(timing==null&&!open)return;
        float duration=shield.duration("jakb-shield-start");
        if(timing==null||timing.open()!=open||time<timing.changed()){
            float current=timing==null||time<timing.changed()?0:
                Math.clamp(timing.open()?time-timing.changed():duration-(time-timing.changed()),0,duration);
            timing=new ShieldTiming(open,time-(open?current:duration-current));
            SHIELDS.put(actor,timing);
        }
        float age=time-timing.changed();
        float fraction=Math.clamp(open?age/duration:1-age/duration,0,1);
        if(fraction<=0){
            if(!open)SHIELDS.remove(actor);
            return;
        }
        String clip="jakb-shield-start";
        float frameTime=fraction*duration;
        float hit=pose.raw().get("shieldHitAge").getAsFloat()+elapsed;
        if(open&&hit>=0&&hit<shield.duration("jakb-shield-hit")){
            clip="jakb-shield-hit";
            frameTime=hit;
        }else if(open&&age>=duration){
            clip="jakb-shield-idle";
            frameTime=(age-duration)%shield.duration(clip);
        }
        var camera=Minecraft.getInstance().gameRenderer.mainCamera();
        var toBody=new Matrix4f().rotateY((float)Math.toRadians(pose.yaw()));
        var forward=toBody.transformDirection(new Vector3f(camera.forwardVector())).negate();
        var up=toBody.transformDirection(new Vector3f(camera.upVector()));
        var right=toBody.transformDirection(new Vector3f(camera.leftVector()));
        var center=body.joint(skeleton,"upper_body").getTranslation(new Vector3f()).add(forward);
        var transform=new Matrix4f(
            2*right.x,2*right.y,2*right.z,0,2*up.x,2*up.y,2*up.z,0,
            2*forward.x,2*forward.y,2*forward.z,0,center.x,center.y,center.z,1);
        stack.pushPose();
        stack.mulPose(transform);
        shield.submitLightEffect(shield.poseAtTime(clip,frameTime),stack,collector,LightFormTexture.Form.SHIELD,fraction);
        stack.popPose();
    }
    private static void hand(AvatarRenderState state,boolean right,GltfModel.Posed skeleton,PoseStack stack,SubmitNodeCollector collector){
        var held=right?state.rightHandItemState:state.leftHandItemState;
        var item=right?state.rightHandItemStack:state.leftHandItemStack;
        if(item.isEmpty())return;
        stack.pushPose();
        int kind=JakItems.GUNS.indexOf(item.getItem());
        Matrix4f joint=body.joint(skeleton,kind>=0?"gun":right?"sk_rhand":"sk_lhand");
        if(item.is(JakItems.SKULL_GEM)){var palm=joint.getTranslation(new Vector3f()).add(body.joint(skeleton,right?"rmidA":"lmidA").getTranslation(new Vector3f())).mul(.5f);joint=new Matrix4f(joint).setTranslation(palm);}
        stack.mulPose(rigid(joint));
        if(kind>=0){
            String color=new String[]{"red","yellow","blue","dark"}[kind/3];String clip="gun-idle-"+color+(kind%3==0?"":String.valueOf(kind%3+1));float age=0;
            var source=JakPoses.get(state.id);var g=source.raw().getAsJsonObject("gun");
            if(g!=null){float elapsed=Math.clamp((System.nanoTime()-source.received())/1e9f,0,.05f),fire=g.get("fireAge").getAsFloat()+elapsed;
                if(fire>=0&&fire<.45f){clip=FIRE_GUN[kind];age=fire;}else age=(g.get("time").getAsFloat()+elapsed)%Math.max(.001f,gun.duration(clip));}
            gun.submit(gun.poseAtTime(clip,age),stack,collector,state.lightCoords);
        }
        else if(item.is(JakItems.SKULL_GEM)){stack.scale(1/bodyScale,1/bodyScale,1/bodyScale);SkullGemRenderer.held(stack,collector,state.lightCoords);}
        else {
            stack.scale(1/bodyScale,1/bodyScale,1/bodyScale);stack.rotateDegrees(Axis.XP,-90);stack.rotateDegrees(Axis.YP,180);
            held.submit(stack,collector,state.lightCoords,OverlayTexture.NO_OVERLAY,state.outlineColor);
        }
        stack.popPose();
    }
    private static Matrix4f rigid(Matrix4f matrix){return new Matrix4f().translationRotate(matrix.getTranslation(new Vector3f()),matrix.getUnnormalizedRotation(new org.joml.Quaternionf()));}
    public static com.google.gson.JsonObject equipmentAudit(){
        load();var out=new com.google.gson.JsonObject();var root=new Matrix4f().scale(bodyScale);
        var closed=board.bounds(board.poseAtTime("board-close",Float.MAX_VALUE),root);var open=board.bounds(board.poseAtTime("board-open",Float.MAX_VALUE),root);
        Vector3f a=new Vector3f(closed[1]).sub(closed[0]),b=new Vector3f(open[1]).sub(open[0]);out.addProperty("closed_board_span",a.toString());out.addProperty("open_board_span",b.toString());out.addProperty("board_folds_smaller",a.length()<b.length());
        if(!Float.isFinite(a.length())||!Float.isFinite(b.length())||a.length()>=b.length())throw new IllegalStateException("Original board-close prejoint does not fold the rendered geometry");
        return out;
    }
}
