package dev.jakmc.client.model;

import com.mojang.blaze3d.vertex.PoseStack;
import com.mojang.math.Axis;
import dev.jakmc.RuntimeConfig;
import dev.jakmc.entity.JakYakow;
import dev.jakmc.player.JakEntityClock;
import java.io.IOException;
import java.io.UncheckedIOException;
import java.util.WeakHashMap;
import net.minecraft.client.renderer.SubmitNodeCollector;
import net.minecraft.client.renderer.entity.EntityRenderer;
import net.minecraft.client.renderer.entity.EntityRendererProvider;
import net.minecraft.client.renderer.entity.state.EntityRenderState;
import net.minecraft.client.renderer.state.level.CameraRenderState;
import net.minecraft.client.renderer.texture.OverlayTexture;
import net.minecraft.util.Mth;
import org.joml.Matrix4f;

public final class YakowRenderer extends EntityRenderer<JakYakow,YakowRenderer.State> {
    private final WeakHashMap<JakYakow,Mixer> poses=new WeakHashMap<>();
    private GltfModel rig;

    private static final class Mixer {
        String clip;
        float started;
        GltfModel.Posed from,last;
    }

    public static final class State extends EntityRenderState {
        GltfModel.Posed pose;
        float yaw,scale,roll;
        boolean hurt;
    }

    public YakowRenderer(EntityRendererProvider.Context context){
        super(context);
        shadowRadius=.85f;
    }

    @Override public State createRenderState(){return new State();}

    @Override public void extractRenderState(JakYakow animal,State state,float partial){
        super.extractRenderState(animal,state,partial);
        if(rig==null){
            try{
                rig=new GltfModel(RuntimeConfig.gameAssets("jak1").resolve("levels/village1/yakow-lod0.glb"),"yakow",true);
            }catch(IOException failure){throw new UncheckedIOException("Cannot load the original Yakow",failure);}
        }
        float time=JakEntityClock.of(animal.level()).partial(partial);
        state.yaw=Mth.rotLerp(time,animal.yBodyRotO,animal.yBodyRot);
        state.scale=JakYakow.SCALE*animal.getAgeScale();
        state.hurt=animal.hurtTime>0||animal.deathTime>0;
        float death=animal.deathTime==0?0:Math.min(1,(float)Math.sqrt((animal.deathTime+time-1)*.08));
        state.roll=death*90;
        var mixer=poses.computeIfAbsent(animal,key->new Mixer());
        String clip=animal.clip();
        if(!clip.equals(mixer.clip)){
            mixer.from=mixer.last;
            mixer.clip=clip;
            mixer.started=animal.tickCount;
        }
        var pose=rig.poseAtTime(clip,animal.animationAge(time));
        float weight=(animal.tickCount+time-mixer.started)/3;
        if(mixer.from!=null&&weight<1)pose=rig.blend(mixer.from,pose,weight);
        else mixer.from=null;
        state.pose=mixer.last=pose;
    }

    @Override public void submit(State state,PoseStack stack,SubmitNodeCollector out,CameraRenderState camera){
        super.submit(state,stack,out,camera);
        if(state.isInvisible||state.pose==null)return;
        stack.pushPose();
        stack.rotateDegrees(Axis.YP,-state.yaw);
        stack.scale(state.scale,state.scale,state.scale);
        if(state.roll>0){
            var rotation=new Matrix4f().rotationZ((float)Math.toRadians(state.roll));
            float ground=rig.bounds(state.pose,rotation)[0].y;
            stack.translate(0,-ground,0);
            stack.mulPose(rotation);
        }
        rig.submit(state.pose,stack,out,state.lightCoords,1,OverlayTexture.pack(0,state.hurt));
        stack.popPose();
    }
}
