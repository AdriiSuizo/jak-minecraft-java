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
    private static GltfModel body,daxter,board,gun;
    private static float bodyScale;
    private static float floorOffset;
    public static long renderSerial;
    public static int lastEntityId=-1;
    public static float standingHeight;
    public static float currentTop=Float.NaN;
    private static void load(){
        if(body!=null)return;
        try{
            var root=RuntimeConfig.assets.resolve("levels/common");
            body=new GltfModel(root.resolve("jakb-c-lod0.glb"),"jak");
            body.animationsFrom(new GltfModel(root.resolve("jakb-lod0.glb"),"jak_clips"));
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
            board=new GltfModel(root.resolve("board-lod0.glb"),"board");
            gun=new GltfModel(root.resolve("gun-lod0.glb"),"gun");
            JakMod.LOG.info("Original Jak, Daxter, board and Morph Gun models loaded into Minecraft renderer");
        }catch(IOException e){throw new IllegalStateException("Cannot load original Jak models",e);}
    }
    public static boolean submit(AvatarRenderState state,PoseStack stack,SubmitNodeCollector collector){
        var pose=JakPoses.get(state.id);if(pose==null)return false;
        var client=Minecraft.getInstance();if(client.level==null)return false;
        var actor=client.level.getEntity(state.id);
        if(actor==null||!actor.getAttachedOrCreate(JakData.ENABLED))return false;
        load();
        renderSerial++;lastEntityId=state.id;
        stack.pushPose();stack.rotateDegrees(Axis.YP,-pose.yaw());stack.scale(bodyScale,bodyScale,bodyScale);stack.translate(0,floorOffset,0);
        float elapsed=Math.clamp((System.nanoTime()-pose.received())/1e9f,0,.05f);
        float frame=pose.frame()+(pose.raw().has("frameRate")?pose.raw().get("frameRate").getAsFloat()*elapsed:0);
        int frames=body.frames(pose.clip());if(pose.raw().has("loop")&&pose.raw().get("loop").getAsBoolean())frame%=frames;
        var skeleton=body.pose(pose.clip(),frame);
        int gunKind=JakItems.GUNS.indexOf(state.getMainHandItemStack().getItem());
        if(gunKind>=0&&(pose.clip().contains("stance")||pose.clip().contains("walk")||pose.clip().contains("jump"))&&!pose.dark()){
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
        body.submit(skeleton,stack,collector,state.lightCoords);
        stack.pushPose();
        Vector3f shoulder=body.joint(skeleton,"LshoulderPad").getTranslation(new Vector3f());
        stack.translate(shoulder.x,shoulder.y,shoulder.z);stack.scale(1/1.4f,1/1.4f,1/1.4f);
        String daxClip=pose.clip().replace("jakb-","daxter-");
        if(!daxter.hasClip(daxClip))daxClip="daxter-stance-loop";
        var daxPose=daxter.pose(daxClip,frame);
        if(ScaleScene.active()){
            var b=body.bounds(skeleton,new Matrix4f());
            var d=daxter.bounds(daxPose,new Matrix4f().translation(shoulder).scale(1/1.4f));
            currentTop=(Math.max(b[1].y,d[1].y)+floorOffset)*bodyScale;
        }
        daxter.submit(daxPose,stack,collector,state.lightCoords);stack.popPose();
        if(pose.board()){
            stack.pushPose();stack.mulPose(rigid(body.joint(skeleton,"board")));board.submit(board.poseAtTime("board-open",Float.MAX_VALUE),stack,collector,state.lightCoords);stack.popPose();
        }
        else if(!actor.getAttachedOrCreate(JakData.BOARD).isEmpty()){
            stack.pushPose();stack.mulPose(rigid(body.joint(skeleton,"board")));
            board.submit(board.poseAtTime("board-close",Float.MAX_VALUE),stack,collector,state.lightCoords);stack.popPose();
        }
        hand(state,true,skeleton,stack,collector);hand(state,false,skeleton,stack,collector);
        stack.popPose();return true;
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
