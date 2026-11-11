package dev.jakmc.client.model;

import com.mojang.blaze3d.vertex.PoseStack;
import com.mojang.math.Axis;
import dev.jakmc.RuntimeConfig;
import dev.jakmc.client.JakPoses;
import dev.jakmc.entity.JakLeaper;
import dev.jakmc.entity.JakFlutFlut;
import java.io.IOException;
import java.io.UncheckedIOException;
import net.fabricmc.fabric.api.client.rendering.v1.EntityRendererRegistry;
import net.minecraft.client.Minecraft;
import net.minecraft.client.renderer.SubmitNodeCollector;
import net.minecraft.client.renderer.entity.EntityRenderer;
import net.minecraft.client.renderer.entity.EntityRendererProvider;
import net.minecraft.client.renderer.entity.state.EntityRenderState;
import net.minecraft.client.renderer.state.level.CameraRenderState;
import net.minecraft.client.renderer.texture.OverlayTexture;
import net.minecraft.util.Mth;

public final class JakLeaperRenderer extends EntityRenderer<JakLeaper,JakLeaperRenderer.State> {
    private static final String PREFIX="flut-saddle-";
    // The original 1.4 prejoint scale defines the animal's scale relative to Jak.
    private static final float SCALE=0.7717099f/1.4f;
    private final boolean flut;
    private static GltfModel leaperModel;
    private GltfModel model,bare;
    private GltfModel.Posed modelRest,bareRest;
    private final java.util.Map<JakFlutFlut,FlutMaterial.Blink> blinks=new java.util.WeakHashMap<>();

    public static class State extends EntityRenderState {
        GltfModel.Posed pose;
        float yaw;
        boolean hurt,useBare;
        float size;
        FlutMaterial plumage;
    }

    public JakLeaperRenderer(EntityRendererProvider.Context context,boolean flut){
        super(context);
        this.flut=flut;
        shadowRadius=.55f;
    }

    public static void initialize(){
        EntityRendererRegistry.register(JakLeaper.TYPE,ctx->new JakLeaperRenderer(ctx,false));
        EntityRendererRegistry.register(JakFlutFlut.TYPE,ctx->new JakLeaperRenderer(ctx,true));
    }

    static GltfModel.Posed ridePose(GltfModel model,String prefix,JakPoses.Pose player,float elapsed){
        var ride=player.raw().getAsJsonObject("ride");
        String source=player.clip();
        String suffix=source.startsWith("jakb1-")?source.substring(6):"flut-idle";
        if(suffix.equals("flut-squash-run")&&!model.hasClip(prefix+suffix))suffix="flut-run-squash";
        String clip=prefix+suffix;
        if(!model.hasClip(clip))clip=prefix+"flut-idle";
        float max=Math.max(1,ride.get("frameMax").getAsFloat());
        float frame=player.frame()+player.raw().get("frameRate").getAsFloat()*elapsed;
        if(player.raw().get("loop").getAsBoolean())frame%=max+1;
        float progress=Math.clamp(frame/max,0,1);
        var pose=model.pose(clip,progress*(model.frames(clip)-1));
        if(suffix.equals("flut-walk")&&model.hasClip(prefix+"flut-run")){
            String run=prefix+"flut-run";
            pose=model.blend(pose,model.pose(run,progress*(model.frames(run)-1)),ride.get("runMix").getAsFloat());
        }
        return pose;
    }

    static float saddleCycle(String motion,float ticks){
        float duration=leaperModel==null?1:Math.max(.01f,leaperModel.duration(PREFIX+"flut-"+motion));
        return (ticks/20f)%duration/duration;
    }
    @Override public State createRenderState(){return new State();}

    @Override public void extractRenderState(JakLeaper entity,State state,float partial){
        super.extractRenderState(entity,state,partial);
        if(model==null){
            try{
                if(flut){
                    var root=FlutMaterial.jak1();
                    model=new GltfModel(root.resolve("levels/snow/flut-saddle-lod0.glb"),"flut_saddled",true);
                    bare=new GltfModel(root.resolve("levels/beach/flutflut-lod0.glb"),"flut_bare",true);
                    modelRest=model.pose("",0);
                    bareRest=bare.pose("",0);
                }else{
                    model=new GltfModel(RuntimeConfig.assets.resolve("levels/waswide/flut-saddle-lod0.glb"),"leaper",true);
                    leaperModel=model;
                }
            }
            catch(IOException failure){throw new UncheckedIOException("Cannot load original Leaper rig",failure);}
        }
        state.yaw=Mth.rotLerp(partial,entity.yBodyRotO,entity.yBodyRot);
        state.hurt=entity.hurtTime>0;
        state.size=1;
        state.useBare=false;
        state.plumage=null;
        if(entity instanceof JakFlutFlut bird){
            state.useBare=!bird.saddled();
            state.size=bird.getAgeScale();
            var blink=blinks.computeIfAbsent(bird,b->new FlutMaterial.Blink(b.getId()));
            state.plumage=new FlutMaterial(bird.plumage(),blink.closed(state.ageInTicks/20f));
        }
        var player=JakPoses.rendered(entity.riderId());
        var ride=player!=null&&player.raw().get("ride") instanceof com.google.gson.JsonObject channel?channel:null;
        if(ride!=null&&ride.get("entity").getAsInt()==entity.getId()){
            float elapsed=JakPoses.elapsed(player);
            state.pose=ridePose(model,PREFIX,player,elapsed);
            if(!ride.get("fixed").getAsBoolean()){
                var client=Minecraft.getInstance();
                var rider=client.level.getEntity(entity.riderId());
                if(rider!=null){
                    var position=rider==client.player?JakPoses.presented():rider.getPosition(partial);
                    if(position!=null){state.x=position.x;state.y=position.y;state.z=position.z;}
                    state.yaw=player.yawAt(elapsed);
                }
            }
        }else{
            boolean dead=entity.isDeadOrDying();
            int rear=entity instanceof JakFlutFlut bird?bird.rearTime():0;
            String suffix=dead?"deatha":state.hurt||rear>0?"hit-back":entity.isInWater()||!entity.onGround()?"jump-loop":
                entity.walkAnimation.speed()>0.07f?"walk":"idle";
            String clip=PREFIX+"flut-"+suffix;
            float duration=model.duration(clip);
            float age=dead?Math.min(duration,(entity.deathTime+partial)/20f)
                :state.hurt?Math.min(duration,(10-entity.hurtTime+partial)/20f)
                :rear>0?(24-rear+partial)/24*duration:(state.ageInTicks/20f)%duration;
            state.pose=model.poseAtTime(clip,age);
        }
        if(state.useBare)state.pose=bare.retargetBird(model,modelRest,state.pose,bareRest);
    }

    @Override public void submit(State state,PoseStack stack,SubmitNodeCollector collector,CameraRenderState camera){
        super.submit(state,stack,collector,camera);
        if(state.isInvisible||state.pose==null)return;
        stack.pushPose();
        stack.rotateDegrees(Axis.YP,-state.yaw);
        float scale=SCALE*state.size;
        stack.scale(scale,scale,scale);
        int overlay=OverlayTexture.pack(0,state.hurt);
        if(state.plumage!=null)(state.useBare?bare:model).submitFlut(state.pose,stack,collector,state.lightCoords,overlay,state.plumage);
        else model.submit(state.pose,stack,collector,state.lightCoords,1,overlay);
        stack.popPose();
    }
}
