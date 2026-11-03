package dev.jakmc.client.model;

import com.mojang.blaze3d.vertex.PoseStack;
import com.mojang.math.Axis;
import dev.jakmc.RuntimeConfig;
import dev.jakmc.client.JakPoses;
import dev.jakmc.entity.JakLeaper;
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
    private GltfModel model;

    public static class State extends EntityRenderState {
        GltfModel.Posed pose;
        float yaw;
        boolean hurt;
    }

    public JakLeaperRenderer(EntityRendererProvider.Context context){
        super(context);
        shadowRadius=.55f;
    }

    public static void initialize(){EntityRendererRegistry.register(JakLeaper.TYPE,JakLeaperRenderer::new);}

    static GltfModel.Posed ridePose(GltfModel model,String prefix,JakPoses.Pose player,float elapsed){
        var ride=player.raw().getAsJsonObject("ride");
        String source=player.clip();
        String suffix=source.startsWith("jakb1-")?source.substring(6):"flut-idle";
        if(suffix.equals("flut-squash-run"))suffix="flut-run-squash";
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

    @Override public State createRenderState(){return new State();}

    @Override public void extractRenderState(JakLeaper entity,State state,float partial){
        super.extractRenderState(entity,state,partial);
        if(model==null){
            try{model=new GltfModel(RuntimeConfig.assets.resolve("levels/waswide/flut-saddle-lod0.glb"),"leaper",true);}
            catch(IOException failure){throw new UncheckedIOException("Cannot load original Leaper rig",failure);}
        }
        state.yaw=Mth.rotLerp(partial,entity.yBodyRotO,entity.yBodyRot);
        state.hurt=entity.hurtTime>0;
        var player=JakPoses.get(entity.riderId());
        var ride=player!=null&&player.raw().get("ride") instanceof com.google.gson.JsonObject channel?channel:null;
        if(ride!=null&&ride.get("entity").getAsInt()==entity.getId()){
            float elapsed=Math.clamp((System.nanoTime()-player.received())/1e9f,0,.05f);
            state.pose=ridePose(model,PREFIX,player,elapsed);
            if(!ride.get("fixed").getAsBoolean()){
                var client=Minecraft.getInstance();
                var rider=client.level.getEntity(entity.riderId());
                if(rider!=null){
                    var position=rider==client.player?JakPoses.presented():rider.getPosition(partial);
                    if(position!=null){state.x=position.x;state.y=position.y;state.z=position.z;}
                    state.yaw=player.yaw();
                }
            }
        }else{
            boolean dead=entity.isDeadOrDying();
            String suffix=dead?"deatha":state.hurt?"hit-back":entity.isInWater()||!entity.onGround()?"jump-loop":
                entity.walkAnimation.speed()>0.07f?"walk":"idle";
            String clip=PREFIX+"flut-"+suffix;
            float duration=model.duration(clip);
            float age=dead?Math.min(duration,(entity.deathTime+partial)/20f)
                :state.hurt?Math.min(duration,(10-entity.hurtTime+partial)/20f):(state.ageInTicks/20f)%duration;
            state.pose=model.poseAtTime(clip,age);
        }
    }

    @Override public void submit(State state,PoseStack stack,SubmitNodeCollector collector,CameraRenderState camera){
        super.submit(state,stack,collector,camera);
        if(state.isInvisible||state.pose==null)return;
        stack.pushPose();
        stack.rotateDegrees(Axis.YP,-state.yaw);
        stack.scale(SCALE,SCALE,SCALE);
        model.submit(state.pose,stack,collector,state.lightCoords,1,OverlayTexture.pack(0,state.hurt));
        stack.popPose();
    }
}
