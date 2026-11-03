package dev.jakmc.client.model;

import com.mojang.blaze3d.vertex.PoseStack;
import com.mojang.math.Axis;
import dev.jakmc.entity.JakFlutEgg;
import java.io.IOException;
import java.io.UncheckedIOException;
import net.minecraft.client.renderer.SubmitNodeCollector;
import net.minecraft.client.renderer.entity.EntityRenderer;
import net.minecraft.client.renderer.entity.EntityRendererProvider;
import net.minecraft.client.renderer.entity.state.EntityRenderState;
import net.minecraft.client.renderer.state.level.CameraRenderState;
import org.joml.Matrix4f;

public final class JakFlutEggRenderer extends EntityRenderer<JakFlutEgg,JakFlutEggRenderer.State> {
    public static class State extends EntityRenderState {
        GltfModel.Posed egg;
        float yaw;
    }
    private GltfModel model;
    private GltfModel.Posed still;
    private float scale,base;
    public JakFlutEggRenderer(EntityRendererProvider.Context context){super(context);shadowRadius=.22f;}
    @Override public State createRenderState(){return new State();}

    @Override public void extractRenderState(JakFlutEgg egg,State state,float partial){
        super.extractRenderState(egg,state,partial);
        if(model==null){
            try{model=new GltfModel(FlutMaterial.jak1().resolve("levels/beach/flutflutegg-lod0.glb"),"flut_egg",true);}
            catch(IOException e){throw new UncheckedIOException(e);}
            still=model.pose("",0);
            model.prepare(still);
            var bounds=model.bounds(still,new Matrix4f());
            base=bounds[0].y;
            scale=.55f/(bounds[1].y-base);
        }
        float age=egg.visualAge()+partial;
        state.yaw=egg.getYRot();
        if(age>=JakFlutEgg.HATCH){
            state.egg=sample("flutflutegg-broke",(age-JakFlutEgg.HATCH)/JakFlutEgg.SHELL);
        }else if(age>=JakFlutEgg.HATCH-JakFlutEgg.CRACK){
            state.egg=sample("flutflutegg-crack",(age-JakFlutEgg.HATCH+JakFlutEgg.CRACK)/JakFlutEgg.CRACK);
        }else if(age>JakFlutEgg.HATCH/2f&&age%180<30){
            state.egg=sample("flutflutegg-idle",(age%180)/30);
        }else state.egg=still;
    }

    private GltfModel.Posed sample(String name,float fraction){
        return model.pose(name,Math.clamp(fraction,0,1)*(model.frames(name)-1));
    }

    @Override public void submit(State state,PoseStack stack,SubmitNodeCollector collector,CameraRenderState camera){
        super.submit(state,stack,collector,camera);
        if(state.isInvisible||state.egg==null)return;
        stack.pushPose();
        stack.rotateDegrees(Axis.YP,-state.yaw);
        stack.scale(scale,scale,scale);
        stack.translate(0,-base,0);
        model.submit(state.egg,stack,collector,state.lightCoords);
        stack.popPose();
    }
}
