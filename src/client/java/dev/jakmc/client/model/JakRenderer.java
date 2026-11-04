package dev.jakmc.client.model;

import dev.jakmc.*;
import dev.jakmc.client.*;
import dev.jakmc.content.JakItems;
import dev.jakmc.nativebridge.NativeJak;
import com.google.gson.JsonParser;
import com.mojang.blaze3d.vertex.PoseStack;
import com.mojang.math.Axis;
import net.minecraft.client.Minecraft;
import net.minecraft.client.renderer.SubmitNodeCollector;
import net.minecraft.client.renderer.entity.state.AvatarRenderState;
import net.minecraft.client.renderer.texture.OverlayTexture;
import org.joml.Matrix4f;
import org.joml.Quaternionf;
import org.joml.Vector3f;
import java.io.IOException;
import java.util.Map;
import java.util.WeakHashMap;
import net.minecraft.world.entity.Entity;

public final class JakRenderer {
    private static final String[] FIRE_BODY={"jakb-gun-red-fire","jakb-gun-red-fire-2","jakb-gun-red-fire","jakb-gun-yellow-fire","jakb-gun-yellow-fire","jakb-gun-yellow-fire-3","jakb-gun-blue-fire-single","jakb-gun-blue-fire-2","jakb-gun-blue-fire-single","jakb-gun-dark-fire","jakb-gun-dark-fire","jakb-gun-dark-fire"};
    private static final String[] FIRE_GUN={"gun-gun-red-fire","gun-gun-red-fire-2","gun-gun-red-fire-3","gun-gun-yellow-fire","gun-gun-yellow-fire-2","gun-gun-yellow-fire-3","gun-gun-blue-fire-single","gun-gun-blue-fire-2","gun-gun-blue-fire-3-single","gun-gun-dark-fire","gun-gun-dark-fire-2","gun-gun-dark-fire-3"};
    private static GltfModel body,daxter,board,gun,wings,shield;
    private static GltfModel cinematicBody,cinematicDaxter,cinematicDark;
    private static final Map<Entity,JakChannels.Transition> transitions=new WeakHashMap<>();
    private static final Map<Entity,JakCloth> clothes=new WeakHashMap<>();
    private static final Map<Entity,JakCloth> portraitClothes=new WeakHashMap<>();
    public static long previewRenderSerial;
    public static float previewBodyYaw;
    public static int worldClothResets(Entity actor){var cloth=clothes.get(actor);return cloth==null?0:cloth.resets();}
    private static float bodyScale;
    private static float floorOffset;
    private static boolean ridingLoaded,pilotLoaded,iceLoaded;
    private static GltfModel jak1Rig;
    private static GltfModel.Posed jak1Rest,jakRest;
    public static long renderSerial,sleepRenderSerial;
    public static long clothRenderSerial;
    public static float clothScarfReach;
    public static long cinematicRenderSerial;
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
            var levels=root.getParent();
            cinematicBody=cinematic(levels.resolve("arenacst/jakc-highres-lod0.glb"),"jak_cinematic",body);
            cinematicDaxter=cinematic(levels.resolve("arenacst/daxter-highres-lod0.glb"),"daxter_cinematic",daxter);
            cinematicDark=cinematic(levels.resolve("mhctycst/darkjak-highres-lod0.glb"),"jak_dark_cinematic",body);
            var ladderArt=JsonParser.parseString(NativeJak.ladderAnimations()).getAsJsonObject();
            body.originalAnimations(ladderArt,"jakb-ladder-");
            daxter.originalAnimations(ladderArt,"daxter-ladder-");
            if(!body.hasClip("jakb-ladder-up")||!daxter.hasClip("daxter-ladder-up"))
                throw new IOException("Original ladder joint tracks are unavailable");
            var standing=body.pose("jakb-stance-loop",0);
            jakRest=standing;
            Vector3f anchor=body.joint(standing,"LshoulderPad").getTranslation(new Vector3f());
            var bodyBounds=cinematicBody.bounds(standing,new Matrix4f());
            var daxterBounds=cinematicDaxter.bounds(daxter.pose("daxter-stance-loop",0),new Matrix4f().translation(anchor).scale(1/1.4f));
            float together=Math.max(bodyBounds[1].y,daxterBounds[1].y)-Math.min(bodyBounds[0].y,daxterBounds[0].y);
            floorOffset=-Math.min(bodyBounds[0].y,daxterBounds[0].y);
            // User scale reference: two air cells, ceiling in the third row.
            bodyScale=1.94f/together;
            standingHeight=together*bodyScale;
            JakMod.LOG.info("Jak + Daxter original height {} m; Minecraft scale {}; standing ears at 1.94 blocks",together,bodyScale);
            boardModel();
            gun=new GltfModel(root.resolve("gun-lod0.glb"),"gun");
            JakCloth.load();
            JakMod.LOG.info("Original Jak, Daxter, board and Morph Gun models loaded into Minecraft renderer");
        }catch(IOException e){throw new IllegalStateException("Cannot load original Jak models",e);}
    }

    private static GltfModel cinematic(java.nio.file.Path file,String name,GltfModel rig)throws IOException{
        if(!java.nio.file.Files.isRegularFile(file)){
            JakMod.LOG.warn("Original cinematic export missing: {}",file.getFileName());
            return rig;
        }
        var mesh=new GltfModel(file,name);
        mesh.wearOn(rig);
        JakMod.LOG.info("Cinematic {}: {} triangles bound to the gameplay skeleton",file.getFileName(),mesh.triangles());
        return mesh;
    }

    private static GltfModel bodyMesh(Entity actor,AvatarRenderState state,boolean dark){
        // Use the lighter original mesh for distant remote players whose facial details cannot be seen.
        if(actor!=Minecraft.getInstance().player&&state.distanceToCameraSq>16*16)return body;
        return dark?cinematicDark:cinematicBody;
    }

    private static GltfModel daxterMesh(Entity actor,AvatarRenderState state){
        return actor==Minecraft.getInstance().player||state.distanceToCameraSq<=16*16?cinematicDaxter:daxter;
    }
    static GltfModel boardModel(){
        if(board==null){
            try{board=new GltfModel(RuntimeConfig.assets.resolve("levels/common/board-lod0.glb"),"board");}
            catch(IOException e){throw new java.io.UncheckedIOException("Cannot load the original JET-Board",e);}
        }
        return board;
    }

    public static boolean submit(AvatarRenderState state,PoseStack stack,SubmitNodeCollector collector,
                                 net.minecraft.client.renderer.state.level.CameraRenderState camera){
        var pose=JakPoses.rendered(state.id);
        var client=Minecraft.getInstance();if(client.level==null)return false;
        var actor=client.level.getEntity(state.id);
        if(actor==null||!actor.getAttachedOrCreate(JakData.ENABLED))return false;
        if(actor.getVehicle() instanceof dev.jakmc.entity.JakVehicle car&&car.frame()!=null){
            submitCarRider(actor,state,stack,collector,camera.pos,car,client.getDeltaTracker().getGameTimeDeltaPartialTick(false));
            return true;
        }
        if(state.bedOrientation!=null&&actor instanceof net.minecraft.world.entity.LivingEntity living&&living.isSleeping()){
            load();
            submitSleeping(actor,state,stack,collector,camera.pos);
            return true;
        }
        if(pose==null)return false;
        load();
        renderSerial++;lastEntityId=state.id;
        boolean portrait=JakGuiContext.active();
        if(portrait)previewRenderSerial++;
        var flight=pose.raw().get("hellcat") instanceof com.google.gson.JsonObject craft?craft:null;
        var ride=pose.raw().get("ride");
        boolean mounted=ride!=null&&ride.isJsonObject();
        stack.pushPose();
        float elapsed=JakPoses.elapsed(pose);
        var boardFrame=pose.boardAt(elapsed);
        float shownYaw=portrait?state.bodyRot:flight!=null?pose.yaw():pose.yawAt(elapsed);
        if(portrait)previewBodyYaw=shownYaw;
        if(boardFrame==null)stack.rotateDegrees(Axis.YP,-shownYaw);
        else {
            stack.mulPose(portrait?new Matrix4f().rotationY((float)Math.toRadians(-shownYaw)):new Matrix4f().rotation(boardFrame.rotation()));
            stack.translate(0,boardFrame.lift(),0);
        }
        if(flight!=null){
            var hull=Minecraft.getInstance().level.getEntity(flight.get("entity").getAsInt());
            if(!portrait&&hull instanceof dev.jakmc.entity.JakHellcat cat&&flight.get("boarding").getAsFloat()<0){
                var attitude=JakPoses.flightRotation();
                var bank=new org.joml.Quaternionf().rotateY((float)Math.toRadians(pose.yaw())).mul(
                    attitude==null?cat.attitude():attitude);
                stack.mulPose(new org.joml.Matrix4f().rotation(bank));
            }
            loadPilot();
        }
        if(boardFrame==null&&!mounted&&flight==null){
            var root=pose.rootAt(elapsed);
            // 0.9 original metres, measured above the feet before rotating a dive.
            float pivot=.576f;
            stack.translate(0,root.lift(),0);
            if(root.swimTilt()!=0){
                stack.translate(0,pivot,0);
                stack.mulPose(new Matrix4f().rotationX(-root.swimTilt()));
                stack.translate(0,-pivot,0);
            }
            // Vanilla's rung plane is 15.2/16 from the open edge. The capsule sits
            // at .49 while the authored hands reach .39 model metres forward.
            if(root.ladderWeight()>0)stack.translate(0,0,(.46f-.39f*bodyScale)*root.ladderWeight());
        }
        stack.scale(bodyScale,bodyScale,bodyScale);stack.translate(0,floorOffset,0);
        float frame=pose.frame()+(pose.raw().has("frameRate")?pose.raw().get("frameRate").getAsFloat()*elapsed:0);
        int frames=body.frames(pose.clip());
        if(pose.raw().has("loop")&&pose.raw().get("loop").getAsBoolean()){
            float period=pose.raw().has("framePeriod")?pose.raw().get("framePeriod").getAsFloat():frames;
            if(period>0)frame-=Math.floor(frame/period)*period;
        }
        boolean climbing=pose.raw().get("state").getAsString().equals("target-ladder");
        GltfModel.Posed skeleton,daxPose;
        String program=null;
        if(flight!=null){
            skeleton=pilotPose(body,"jakb",flight,elapsed);
            daxPose=pilotPose(daxter,"daxter",flight,elapsed);
        }else if(mounted){
            boolean flut=ride.getAsJsonObject().has("flut")&&ride.getAsJsonObject().get("flut").getAsBoolean();
            loadRide(flut);
            skeleton=flut?body.retargetRider(jak1Rig,jak1Rest,
                JakLeaperRenderer.ridePose(jak1Rig,"jakb1-",pose,elapsed),jakRest):
                JakLeaperRenderer.ridePose(body,"jakb-",pose,elapsed);
            daxPose=JakLeaperRenderer.ridePose(daxter,"daxter-",pose,elapsed);
        }else if(pose.raw().get("darkPose") instanceof com.google.gson.JsonObject dark){
            float age=pose.raw().getAsJsonObject("transition").get("stateAge").getAsFloat()+elapsed;
            skeleton=JakChannels.dark(body,"jakb-darkjak-",dark,age);
            daxPose=JakChannels.dark(daxter,"daxter-darkjak-",dark,age);
            program="dark:"+dark.get("clip").getAsString();
        }else{
            var surface=pose.raw().get("surfaceClip");
            String clip=surface instanceof com.google.gson.JsonPrimitive selected?selected.getAsString():null;
            boolean ice=clip!=null&&clip.startsWith("jakb1-ice-");
            if(ice)loadIce();
            if(clip!=null&&body.hasClip(clip)){
                float age=pose.raw().getAsJsonObject("transition").get("stateAge").getAsFloat()+elapsed;
                program=clip;
                skeleton=JakChannels.loop(body,clip,age);
                daxPose=JakChannels.loop(daxter,ice?"daxter-stance-loop":"daxter-"+clip.substring(5),age);
            }else{
                skeleton=JakChannels.sample(body,false,pose,boardFrame,frame);
                daxPose=JakChannels.sample(daxter,true,pose,boardFrame,frame);
            }
        }
        if(mounted||flight!=null)transitions.remove(actor);
        else{
            var next=new JakChannels.Pair(skeleton,daxPose);
            var mixed=transitions.computeIfAbsent(actor,key->new JakChannels.Transition()).blend(body,daxter,next,pose,elapsed,program);
            skeleton=mixed.body();
            daxPose=mixed.daxter();
        }
        int gunKind=JakItems.GUNS.indexOf(state.getMainHandItemStack().getItem());
        var gunState=pose.raw().getAsJsonObject("gun");
        boolean gunOut=gunState!=null&&gunState.get("out").getAsBoolean();
        boolean shieldUp=pose.raw().has("shield")&&pose.raw().get("shield").getAsBoolean();
        if(!climbing&&!pose.board()&&!mounted&&gunKind>=0&&gunOut&&!shieldUp&&(pose.clip().contains("stance")||pose.clip().contains("walk")||pose.clip().contains("jump"))&&!pose.dark()){
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
        float twist=!portrait&&aiming&&!climbing?Math.clamp((float)Math.toRadians(net.minecraft.util.Mth.wrapDegrees(shownYaw-lookYaw)),-1.6f,1.6f):0;
        skeleton=body.jakForm(skeleton,pose.dark(),twist);
        boolean luminous=pose.raw().has("lightInterp")&&pose.raw().get("lightInterp").getAsFloat()>0;
        var visibleBody=bodyMesh(actor,state,pose.dark());
        var visibleDaxter=daxterMesh(actor,state);
        visibleBody.submitJakBody(skeleton,stack,collector,state.lightCoords,pose.dark(),luminous,false);
        if(visibleBody!=body&&visibleDaxter!=daxter&&actor==client.player)cinematicRenderSerial++;
        submitCloth(actor,state,skeleton,stack,collector,camera.pos,pose.raw().get("session").getAsLong());
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
        var boardJoint=pose.board()?rigid(body.joint(skeleton,"board")):null;
        Vector3f shoulder=body.joint(skeleton,"LshoulderPad").getTranslation(new Vector3f());
        var daxAnchor=boardJoint!=null?boardJoint:new Matrix4f().translation(shoulder);
        stack.pushPose();
        stack.mulPose(daxAnchor);
        stack.scale(1/1.4f,1/1.4f,1/1.4f);
        if(ScaleScene.active()){
            var b=visibleBody.bounds(skeleton,new Matrix4f());
            var d=visibleDaxter.bounds(daxPose,new Matrix4f(daxAnchor).scale(1/1.4f));
            currentTop=(Math.max(b[1].y,d[1].y)+floorOffset)*bodyScale;
        }
        visibleDaxter.submitEyes(daxPose,stack,collector,state.lightCoords,false);stack.popPose();
        if(boardJoint!=null||!actor.getAttachedOrCreate(JakData.BOARD).isEmpty()){
            boolean folding=boardFrame!=null&&boardFrame.closing();
            String clip=boardJoint==null||folding?"board-close":"board-open";
            float age=Float.MAX_VALUE;
            if(boardFrame!=null){
                age=elapsed+(folding?pose.raw().getAsJsonObject("transition").get("stateAge").getAsFloat():boardFrame.age());
                if(age>=board.duration(clip))age=Float.MAX_VALUE;
            }
            stack.pushPose();
            stack.mulPose(boardJoint!=null?boardJoint:rigid(body.joint(skeleton,"board")));
            Vector3f socket=stack.last().pose().getTranslation(new Vector3f());
            if(!portrait)JakEffects.boardSocket(state.id,camera.pos.add(socket.x,socket.y,socket.z));
            board.submit(board.poseAtTime(clip,age),stack,collector,state.lightCoords);
            stack.popPose();
        }
        if(!pose.board()&&!mounted&&flight==null&&!climbing){hand(state,true,skeleton,stack,collector,camera.pos);hand(state,false,skeleton,stack,collector,camera.pos);}
        stack.popPose();return true;
    }
    private record Sleeping(GltfModel.Posed jak,GltfModel.Posed daxter,Matrix4f bed,Matrix4f shoulder,float heading){}
    private static Sleeping sleeping;

    private static void submitCloth(Entity actor,AvatarRenderState state,GltfModel.Posed skeleton,PoseStack stack,
                                    SubmitNodeCollector collector,net.minecraft.world.phys.Vec3 camera,long key){
        if(state.isInvisible){
            clothes.remove(actor);
            return;
        }
        if(JakGuiContext.active()){
            portraitClothes.computeIfAbsent(actor,_ ->new JakCloth()).submitPreview(body,skeleton,stack,collector,state.lightCoords);
            return;
        }
        var cloth=clothes.computeIfAbsent(actor,_ ->new JakCloth());
        int count=cloth.submit(body,skeleton,stack,collector,state.lightCoords,camera,bodyScale,key);
        if(actor==Minecraft.getInstance().player&&count==3){
            clothRenderSerial++;
            clothScarfReach=cloth.scarfReach();
        }
    }

    private static void submitSleeping(Entity actor,AvatarRenderState state,PoseStack stack,SubmitNodeCollector collector,
                                       net.minecraft.world.phys.Vec3 camera){
        if(sleeping==null){
            var jak=body.poseAtTime("jakb-deatha",Float.MAX_VALUE);
            var sidekick=daxter.poseAtTime("daxter-deatha",Float.MAX_VALUE);
            var head=body.joint(jak,"neckB").getTranslation(new Vector3f());
            var feet=body.joint(jak,"Lankle").getTranslation(new Vector3f())
                .add(body.joint(jak,"Rankle").getTranslation(new Vector3f())).mul(.5f);
            var axis=head.sub(feet).normalize();
            var horizontal=new Vector3f(axis.x,0,axis.z).normalize();
            var rotation=new Matrix4f().rotation(new Quaternionf().rotationTo(axis,horizontal));
            var bounds=cinematicBody.bounds(jak,rotation);
            var center=new Vector3f(bounds[0]).add(bounds[1]).mul(.5f);
            var bed=new Matrix4f().translation(-center.x,-bounds[0].y,-center.z).mul(rotation);
            var shoulder=new Matrix4f().translation(body.joint(jak,"LshoulderPad").getTranslation(new Vector3f())).scale(1/1.4f);
            sleeping=new Sleeping(jak,sidekick,bed,shoulder,(float)Math.atan2(horizontal.x,horizontal.z));
        }
        var direction=state.bedOrientation;
        stack.pushPose();
        // Minecraft places a sleeper at the pillow with a 1/8-block lift.
        stack.translate(-direction.getStepX()*.5,-.125,-direction.getStepZ()*.5);
        float heading=(float)Math.atan2(direction.getStepX(),direction.getStepZ());
        stack.rotate(Axis.YP,heading-sleeping.heading);
        stack.scale(bodyScale,bodyScale,bodyScale);
        stack.mulPose(sleeping.bed);
        bodyMesh(actor,state,false).submitJakBody(sleeping.jak,stack,collector,state.lightCoords,false,false,true);
        submitCloth(actor,state,sleeping.jak,stack,collector,camera,-1);
        stack.mulPose(sleeping.shoulder);
        daxterMesh(actor,state).submitEyes(sleeping.daxter,stack,collector,state.lightCoords,true);
        stack.popPose();
        renderSerial++;
        sleepRenderSerial++;
        lastEntityId=state.id;
    }

    private static void submitCarRider(Entity actor,AvatarRenderState state,PoseStack stack,SubmitNodeCollector collector,
        net.minecraft.world.phys.Vec3 camera,dev.jakmc.entity.JakVehicle car,float partial){
        partial=car.presentationFraction();
        load();
        loadPilot();
        var before=car.presentationBefore();
        var frame=car.presentationAfter();
        var controls=frame.deepCopy();
        for(String key:new String[]{"lean","throttle","brake"})if(before.has(key)&&frame.has(key))controls.addProperty(key,net.minecraft.util.Mth.lerp(partial,before.get(key).getAsFloat(),frame.get(key).getAsFloat()));
        controls.addProperty("steering",net.minecraft.util.Mth.lerp(partial,before.get("steer").getAsFloat(),frame.get("steer").getAsFloat()));
        float age=frame.get("boarding").getAsFloat();
        var skeleton=body.jakForm(carPilotPose(body,"jakb",controls,age<0?0:(partial-1)/20,car.model),false,0);
        var passenger=carPilotPose(daxter,"daxter",controls,age<0?0:(partial-1)/20,car.model);
        var attitude=JakGuiContext.active()?new org.joml.Quaternionf().rotationY((float)Math.toRadians(-state.bodyRot)):JakVehicleRenderer.rotation(before).slerp(JakVehicleRenderer.rotation(frame),partial);
        stack.pushPose();
        stack.mulPose(new Matrix4f().rotation(attitude));
        stack.scale(bodyScale,bodyScale,bodyScale);
        stack.translate(0,floorOffset,0);
        bodyMesh(actor,state,false).submitJakBody(skeleton,stack,collector,state.lightCoords,false,false,false);
        submitCloth(actor,state,skeleton,stack,collector,camera,-2L-car.getId());
        Vector3f shoulder=body.joint(skeleton,"LshoulderPad").getTranslation(new Vector3f());
        stack.translate(shoulder.x,shoulder.y,shoulder.z);
        stack.scale(1/1.4f,1/1.4f,1/1.4f);
        daxterMesh(actor,state).submitEyes(passenger,stack,collector,state.lightCoords,false);
        stack.popPose();
        renderSerial++;
        lastEntityId=state.id;
    }
    private static void loadPilot(){
        if(pilotLoaded)return;
        var art=JsonParser.parseString(NativeJak.ridingAnimations()).getAsJsonObject();
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
        if(ridingLoaded&&(!flut||jak1Rig!=null&&jak1Rig.hasClip("jakb1-flut-get-on")))return;
        var art=JsonParser.parseString(NativeJak.ridingAnimations()).getAsJsonObject();
        if(!ridingLoaded){
            body.originalAnimations(art,"jakb-flut-");
            daxter.originalAnimations(art,"daxter-flut-");
            if(!body.hasClip("jakb-flut-get-on")||!daxter.hasClip("daxter-flut-get-on")){
                throw new IllegalStateException("Original riding joint count did not match the Jak and Daxter rigs");
            }
            ridingLoaded=true;
        }
        if(flut){
            loadJak1Rig();
            jak1Rig.originalAnimations(art,"jakb1-flut-");
            if(!jak1Rig.hasClip("jakb1-flut-get-on"))
                throw new IllegalStateException("Original Flut Flut riding animations are unavailable");
        }
    }

    private static void loadIce(){
        if(iceLoaded)return;
        var art=JsonParser.parseString(NativeJak.iceAnimations()).getAsJsonObject();
        if(art.size()>0){
            loadJak1Rig();
            jak1Rig.originalAnimations(art,"jakb1-ice-");
            body.retargetClips(jak1Rig,jak1Rest,jakRest,"jakb1-ice-");
            if(!body.hasClip("jakb1-ice-skate"))throw new IllegalStateException("Original Jak 1 ice joint tracks are unavailable");
        }
        iceLoaded=true;
    }

    private static void loadJak1Rig(){
        if(jak1Rig!=null)return;
        try{
            jak1Rig=new GltfModel(FlutMaterial.jak1().resolve("levels/common/eichar-lod0.glb"),"original_flut_rider");
            jak1Rest=jak1Rig.pose("eichar-stance-loop",0);
        }catch(IOException failure){throw new java.io.UncheckedIOException(failure);}
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
    private static void hand(AvatarRenderState state,boolean right,GltfModel.Posed skeleton,PoseStack stack,SubmitNodeCollector collector,net.minecraft.world.phys.Vec3 camera){
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
            var source=JakPoses.rendered(state.id);var g=source.raw().getAsJsonObject("gun");
            if(g!=null){float elapsed=JakPoses.elapsed(source),fire=g.get("fireAge").getAsFloat()+elapsed;
                if(fire>=0&&fire<.45f){clip=FIRE_GUN[kind];age=fire;}else age=(g.get("time").getAsFloat()+elapsed)%Math.max(.001f,gun.duration(clip));}
            var gunPose=gun.poseAtTime(clip,age);
            gun.submit(gunPose,stack,collector,state.lightCoords);
            if(right&&!JakGuiContext.active()&&gun.hasJoint("muzzle")){
                var muzzle=gun.joint(gunPose,"muzzle").getTranslation(new Vector3f());
                var laser=gun.hasJoint("laser")?gun.joint(gunPose,"laser").getTranslation(new Vector3f()):new Vector3f(muzzle);
                stack.last().pose().transformPosition(muzzle);stack.last().pose().transformPosition(laser);
                var worldMuzzle=camera.add(muzzle.x,muzzle.y,muzzle.z);
                var worldLaser=camera.add(laser.x,laser.y,laser.z);
                var direction=dev.jakmc.player.JakPlayers.vector(g.getAsJsonArray("direction"));
                var client=Minecraft.getInstance();var entity=client.level.getEntity(state.id);
                var end=worldLaser.add(direction.scale(40*.64));
                var hit=client.level.clip(new net.minecraft.world.level.ClipContext(worldLaser,end,
                    net.minecraft.world.level.ClipContext.Block.COLLIDER,net.minecraft.world.level.ClipContext.Fluid.NONE,entity));
                JakEffects.gunSocket(state.id,worldMuzzle,worldLaser,hit.getLocation());
            }
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
