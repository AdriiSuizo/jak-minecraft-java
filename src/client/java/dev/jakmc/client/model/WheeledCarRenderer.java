package dev.jakmc.client.model;

import com.google.gson.JsonObject;
import com.mojang.blaze3d.vertex.PoseStack;
import dev.jakmc.RuntimeConfig;
import dev.jakmc.entity.JakWheeledCar;
import java.io.IOException;
import java.io.UncheckedIOException;
import java.util.HashMap;
import java.util.Map;
import net.fabricmc.fabric.api.client.rendering.v1.EntityRendererRegistry;
import net.minecraft.client.renderer.SubmitNodeCollector;
import net.minecraft.client.renderer.entity.EntityRenderer;
import net.minecraft.client.renderer.entity.EntityRendererProvider;
import net.minecraft.client.renderer.entity.state.EntityRenderState;
import net.minecraft.client.renderer.state.level.CameraRenderState;
import org.joml.Quaternionf;
import org.joml.Vector3f;

public final class WheeledCarRenderer extends EntityRenderer<JakWheeledCar,WheeledCarRenderer.State> {
    private static final String[] TOAD_AXLES={"L_front_shock","R_front_shock","LR_axle","RR_axle"};
    private static final String[] SNAKE_AXLES={"L_F_axle","R_F_axle","L_R_axle","R_R_axle"};
    private static final String[] SHOCKS={"LF","RF","LR","RR"};
    private final Map<String,GltfModel> assets=new HashMap<>();
    private final Map<String,GltfModel.Posed> rests=new HashMap<>();

    public static void initialize(){
        for(var model:JakWheeledCar.Model.values())EntityRendererRegistry.register(model.type,WheeledCarRenderer::new);
    }
    public WheeledCarRenderer(EntityRendererProvider.Context context){super(context);shadowRadius=1.3f;}
    public static final class State extends EntityRenderState {
        JsonObject before,after;
        float partial;
        JakWheeledCar.Model model;
    }

    @Override public State createRenderState(){return new State();}
    @Override public void extractRenderState(JakWheeledCar car,State state,float partial){
        super.extractRenderState(car,state,partial);
        state.before=car.previousFrame();
        state.after=car.frame();
        state.partial=partial;
        state.model=car.model;
    }

    @Override public void submit(State state,PoseStack stack,SubmitNodeCollector collector,CameraRenderState camera){
        super.submit(state,stack,collector,camera);
        if(state.isInvisible||state.after==null)return;
        float blend=state.partial;
        boolean toad=state.model==JakWheeledCar.Model.TOAD;
        var body=asset(state.model.asset);
        var rest=rests.get(state.model.asset);
        var wheelName=state.model.asset+(state.after.get("blur").getAsBoolean()?"-wheel-blur":"-wheel");
        var wheel=asset(wheelName);
        var previous=state.before.getAsJsonArray("wheels");
        var current=state.after.getAsJsonArray("wheels");
        var turns=new HashMap<String,Quaternionf>();
        var positions=new HashMap<String,Vector3f>();
        float steer=interpolate(state.before,state.after,"steer",blend);
        for(String side:new String[]{"L","R"})turns.put("steer_wheel_"+side,new Quaternionf().rotateZ(-steer*.6f));
        for(int i=0;i<4;i++){
            var a=previous.get(i).getAsJsonObject();
            var b=current.get(i).getAsJsonObject();
            float axle=interpolate(a,b,"axle",blend);
            turns.put((toad?TOAD_AXLES:SNAKE_AXLES)[i],
                toad&&i<2?new Quaternionf().rotateX(axle):new Quaternionf().rotateZ(axle));
            if(toad&&i<2)continue;
            turns.put(SHOCKS[i]+"_shock_top",new Quaternionf().rotateZ(interpolate(a,b,"shock",blend)));
            String piston=SHOCKS[i]+(toad?"_shock_bottom":"_shock_mid");
            var point=body.localTranslation(rest,piston);
            point.y=interpolate(a,b,"stroke",blend);
            positions.put(piston,point);
        }
        if(!toad)for(int i=0;i<2;i++){
            String gun=i==0?"L_gun":"R_gun";
            var point=body.localTranslation(rest,gun);
            point.z-=net.minecraft.util.Mth.lerp(blend,state.before.getAsJsonArray("recoil").get(i).getAsFloat(),
                state.after.getAsJsonArray("recoil").get(i).getAsFloat());
            positions.put(gun,point);
        }
        stack.pushPose();
        stack.mulPose(new org.joml.Matrix4f().rotation(rotation(state.before).slerp(rotation(state.after),blend)));
        stack.scale(.64f,.64f,.64f);
        body.submit(body.articulate(rest,turns,positions),stack,collector,state.lightCoords,1,
            net.minecraft.client.renderer.texture.OverlayTexture.NO_OVERLAY,toad?-1L:5L);
        for(int i=0;i<4;i++){
            var a=previous.get(i).getAsJsonObject();
            var b=current.get(i).getAsJsonObject();
            var offset=vector(a,"pos").lerp(vector(b,"pos"),blend);
            stack.pushPose();
            stack.translate(offset.x,offset.y,offset.z);
            stack.mulPose(new org.joml.Matrix4f().rotation(rotation(a).slerp(rotation(b),blend)));
            float scale=b.get("scale").getAsFloat();
            stack.scale(scale,scale,scale);
            wheel.submit(rests.get(wheelName),stack,collector,state.lightCoords);
            stack.popPose();
        }
        stack.popPose();
        var shots=state.after.getAsJsonArray("grenades");
        if(shots!=null&&!shots.isEmpty()){
            var grenade=asset("gun-grenade");
            for(var value:shots){
                var shot=value.getAsJsonObject();
                var offset=vector(shot,"pos");
                for(var old:state.before.getAsJsonArray("grenades")){
                    var oldShot=old.getAsJsonObject();
                    if(oldShot.get("id").getAsInt()==shot.get("id").getAsInt()){
                        offset=vector(oldShot,"pos").lerp(offset,blend);
                        break;
                    }
                }
                stack.pushPose();
                stack.translate(offset.x,offset.y,offset.z);
                stack.scale(1.28f,1.28f,1.28f);
                grenade.submit(rests.get("gun-grenade"),stack,collector,state.lightCoords);
                stack.popPose();
            }
        }
    }

    private GltfModel asset(String name){
        return assets.computeIfAbsent(name,key->{
            try{
                var model=new GltfModel(RuntimeConfig.assets.resolve((key.equals("gun-grenade")?"levels/common/":"levels/wasall/")+key+"-lod0.glb"),key,true);
                if(key.equals("snake"))model.loadSegments(RuntimeConfig.assets.resolve("raw_obj/snake-ag.go"),"snake-lod0");
                var rest=model.pose(model.hasClip(key+"-idle")?key+"-idle":"",0);
                rests.put(key,rest);
                if(key.contains("-wheel")||key.equals("gun-grenade"))model.prepare(rest);
                return model;
            }catch(IOException e){throw new UncheckedIOException(e);}
        });
    }
    static Quaternionf rotation(JsonObject obj){
        var v=obj.getAsJsonArray("rotation");
        return new Quaternionf(v.get(0).getAsFloat(),v.get(1).getAsFloat(),v.get(2).getAsFloat(),v.get(3).getAsFloat());
    }
    private static Vector3f vector(JsonObject obj,String key){
        var v=obj.getAsJsonArray(key);
        return new Vector3f(v.get(0).getAsFloat(),v.get(1).getAsFloat(),v.get(2).getAsFloat());
    }
    private static float interpolate(JsonObject a,JsonObject b,String key,float blend){
        return net.minecraft.util.Mth.lerp(blend,a.get(key).getAsFloat(),b.get(key).getAsFloat());
    }
}
