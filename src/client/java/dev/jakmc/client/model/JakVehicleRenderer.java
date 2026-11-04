package dev.jakmc.client.model;

import com.google.gson.JsonObject;
import com.mojang.blaze3d.vertex.PoseStack;
import dev.jakmc.RuntimeConfig;
import dev.jakmc.entity.JakVehicle;
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

public final class JakVehicleRenderer extends EntityRenderer<JakVehicle,JakVehicleRenderer.State> {
    private static final String[] TOAD_AXLES={"L_front_shock","R_front_shock","LR_axle","RR_axle"};
    private static final String[] SNAKE_AXLES={"L_F_axle","R_F_axle","L_R_axle","R_R_axle"};
    private static final String[] SHOCKS={"LF","RF","LR","RR"};
    private final Map<String,GltfModel> assets=new HashMap<>();
    private final Map<String,GltfModel.Posed> rests=new HashMap<>();

    public static void initialize(){
        for(var model:JakVehicle.Model.values())EntityRendererRegistry.register(model.type,JakVehicleRenderer::new);
    }
    public JakVehicleRenderer(EntityRendererProvider.Context context){super(context);shadowRadius=1.3f;}
    public static final class State extends EntityRenderState {
        JsonObject before,after;
        float partial;
        JakVehicle.Model model;
    }

    @Override public State createRenderState(){return new State();}
    @Override public void extractRenderState(JakVehicle car,State state,float partial){
        super.extractRenderState(car,state,partial);
        if(car.getFirstPassenger()!=net.minecraft.client.Minecraft.getInstance().player)car.samplePresentation(System.nanoTime());
        var position=car.presentationPosition();
        state.x=position.x;state.y=position.y;state.z=position.z;
        state.before=car.presentationBefore();
        state.after=car.presentationAfter();
        state.partial=car.presentationFraction();
        state.model=car.model;
    }

    @Override public void submit(State state,PoseStack stack,SubmitNodeCollector collector,CameraRenderState camera){
        super.submit(state,stack,collector,camera);
        if(state.isInvisible||state.after==null)return;
        if(state.model.hover()){
            submitZoomer(state,stack,collector,camera);
            return;
        }
        float blend=state.partial;
        boolean toad=state.model==JakVehicle.Model.TOAD;
        var body=asset(state.model.asset,state.model.folder);
        var rest=rests.get(state.model.asset);
        var wheelName=state.model.asset+(state.after.get("blur").getAsBoolean()?"-wheel-blur":"-wheel");
        var wheel=asset(wheelName,"wasall");
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
            var grenade=asset("gun-grenade","common");
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

    private void submitZoomer(State state,PoseStack stack,SubmitNodeCollector collector,CameraRenderState camera){
        var model=asset(state.model.asset,state.model.folder);
        float blend=state.partial;
        float steer=interpolate(state.before,state.after,"steer",blend);
        float lean=interpolate(state.before,state.after,"lean",blend);
        float brake=interpolate(state.before,state.after,"brake",blend);
        var joints=new HashMap<String,Quaternionf>();
        String[] names;
        float[] values;
        switch(state.model){
            case BIKEA -> {
                names=new String[]{"tailfinl_","tailfinr_","frontfinl_","frontfinr_","rudder_","rearbrakel_","rearbraker_"};
                values=new float[]{-30*lean+6*steer,30*lean+6*steer,6*lean-30*steer,-6*lean-30*steer,30*steer,-75*brake,-75*brake};
            }
            case BIKEB -> {
                names=new String[]{"tailfinl_","tailfinr_","rudder_","frontfin_","brakel_","braker_","lkid","rkid"};
                values=new float[]{-30*lean+6*steer,30*lean+6*steer,30*steer,-30*steer,-75*brake,75*brake,75*brake,-75*brake};
            }
            case BIKEC -> {
                names=new String[]{"nosefinlb_","nosefinlh_","nosefinrb_","nosefinrh_","tailfinl_","tailfinr_","rudder_","brakel_","braker_","spoilerl_","spoilerr_"};
                values=new float[]{30*lean,30*lean,-30*lean,-30*lean,-30*lean+6*steer,30*lean+6*steer,30*steer,-75*brake,75*brake,75*brake,-75*brake};
            }
            case CARA -> {
                names=new String[]{"wheel_left_","wheel_right_","frontfinl_","frontfinr_","rearfinl_","rearfinr_","rudder_"};
                values=new float[]{50*steer,50*steer,20*lean,-20*lean,-20*lean,20*lean,20*steer};
            }
            case CARB -> {
                names=new String[]{"wheell_","wheelr","frontfinl_","frontfinr_","tailfinl_","tailfinr_"};
                values=new float[]{50*steer,50*steer,24*lean+20*steer,-24*lean+20*steer,-24*lean+20*steer,24*lean+20*steer};
            }
            case CARC -> {
                names=new String[]{"wheel_","frontfinL_","frontfinR_","tailfinL_","tailfinR_","rearfinL_","rearfinR_"};
                values=new float[]{50*steer,30*lean,-30*lean,-24*lean+6*steer,24*lean+6*steer,-6*lean+24*steer,6*lean+24*steer};
            }
            default -> throw new IllegalArgumentException("Zoomer required");
        }
        for(int i=0;i<names.length;i++)if(model.hasJoint(names[i]))
            joints.put(names[i],new Quaternionf().rotateX((float)Math.toRadians(values[i])));
        var attitude=rotation(state.before).slerp(rotation(state.after),blend);
        var pose=model.articulate(rests.get(state.model.asset),joints,Map.of());
        stack.pushPose();
        stack.mulPose(new org.joml.Matrix4f().rotation(attitude));
        stack.scale(.64f,.64f,.64f);
        model.submit(pose,stack,collector,state.lightCoords,1,
            net.minecraft.client.renderer.texture.OverlayTexture.NO_OVERLAY,state.after.get("mask").getAsLong());
        stack.popPose();
        float intensity=interpolate(state.before,state.after,"lamps",blend);
        if(intensity>0){
            HellcatRenderer.vehicleLamps(points(state.after,"heads"),points(state.after,"tails"),
                attitude,intensity,new net.minecraft.world.phys.Vec3(state.x,state.y,state.z),camera,stack,collector);
        }
    }

    private static float[][] points(JsonObject frame,String name){
        var list=frame.getAsJsonArray(name);
        var points=new float[list.size()][3];
        for(int i=0;i<points.length;i++)for(int axis=0;axis<3;axis++)
            points[i][axis]=list.get(i).getAsJsonArray().get(axis).getAsFloat();
        return points;
    }

    private GltfModel asset(String name,String folder){
        return assets.computeIfAbsent(name,key->{
            try{
                var model=new GltfModel(RuntimeConfig.assets.resolve("levels/"+folder+"/"+key+"-lod0.glb"),key,true);
                if(key.equals("snake")||folder.startsWith("ctycar"))model.loadSegments(RuntimeConfig.assets.resolve("raw_obj/"+key+"-ag.go"),key+"-lod0");
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
