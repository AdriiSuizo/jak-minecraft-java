package dev.jakmc.client.model;

import com.google.gson.JsonArray;
import com.google.gson.JsonObject;
import com.mojang.blaze3d.vertex.PoseStack;
import dev.jakmc.RuntimeConfig;
import dev.jakmc.entity.JakVehicle;
import dev.jakmc.entity.VehicleWreck;
import java.io.IOException;
import java.util.ArrayList;
import java.util.HashMap;
import java.util.List;
import java.util.Map;
import java.util.WeakHashMap;
import net.fabricmc.fabric.api.client.event.lifecycle.v1.ClientTickEvents;
import net.fabricmc.fabric.api.client.rendering.v1.EntityRendererRegistry;
import net.minecraft.client.Minecraft;
import net.minecraft.client.renderer.SubmitNodeCollector;
import net.minecraft.client.renderer.entity.EntityRenderer;
import net.minecraft.client.renderer.entity.EntityRendererProvider;
import net.minecraft.client.renderer.entity.state.EntityRenderState;
import net.minecraft.client.renderer.state.level.CameraRenderState;
import net.minecraft.client.renderer.texture.OverlayTexture;
import net.minecraft.world.level.ClipContext;
import net.minecraft.world.phys.HitResult;
import net.minecraft.world.phys.Vec3;
import net.minecraft.world.phys.shapes.CollisionContext;
import org.joml.Matrix4f;
import org.joml.Quaternionf;
import org.joml.Vector3f;

/** A blown-up vehicle's carcass and the joint-exploder's pieces (Rust render_anim zoomers.rs Debris). */
public final class VehicleWreckRenderer extends EntityRenderer<VehicleWreck,VehicleWreckRenderer.State> {
    private static final double METRE=VehicleWreck.METRE;
    private static final Map<String,GltfModel> ASSETS=new HashMap<>();
    private static final Map<String,GltfModel.Posed> RESTS=new HashMap<>();
    private static final Map<VehicleWreck,Debris> DEBRIS=new WeakHashMap<>();
    private static final Map<VehicleWreck,Boolean> EXPLODED=new WeakHashMap<>();
    private static boolean missing;

    public static void initialize(){
        EntityRendererRegistry.register(VehicleWreck.TYPE,VehicleWreckRenderer::new);
        ClientTickEvents.END_CLIENT_TICK.register(client->{
            if(client.level==null||client.player==null)return;
            for(var entity:client.level.entitiesForRendering()){
                if(!(entity instanceof VehicleWreck wreck)||wreck.distanceToSqr(client.player)>96*96)continue;
                var commands=new JsonArray();
                // Rust zoomers.rs: `explosion-spawn`'s group, the car's big one, the bike's.
                if(EXPLODED.put(wreck,true)==null)commands.add(spawn(wreck.car()?"group-kg-big-explosion":"group-kg-explosion"));
                for(int i=0;i<3;i++){var step=new JsonObject();step.addProperty("Step",1f/60);commands.add(step);}
                dev.jakmc.client.JakEffects.local(wreck.getId(),wreck.origin(),commands);
            }
        });
    }
    private static JsonObject spawn(String group){
        var values=new JsonArray();
        values.add(group);
        var at=new JsonArray();for(int i=0;i<3;i++)at.add(0);
        var basis=new JsonArray();for(float v:new float[]{1,0,0,0,1,0,0,0,1})basis.add(v);
        values.add(at);values.add(basis);
        var command=new JsonObject();command.add("Spawn",values);
        return command;
    }
    public VehicleWreckRenderer(EntityRendererProvider.Context context){super(context);}
    public static final class State extends EntityRenderState {
        int model=-1;
        long mask;
        final Quaternionf carcass=new Quaternionf();
        Debris debris;
        Vec3 offset=Vec3.ZERO;
        float yaw;
        com.google.gson.JsonArray wheels;
    }
    @Override public State createRenderState(){return new State();}
    @Override public void extractRenderState(VehicleWreck wreck,State state,float partial){
        super.extractRenderState(wreck,state,partial);
        state.model=wreck.model();
        state.mask=wreck.mask();
        state.carcass.set(wreck.carcass());
        state.yaw=wreck.yaw();
        state.wheels=wreck.wheels().isEmpty()?null:com.google.gson.JsonParser.parseString(wreck.wheels()).getAsJsonArray();
        state.debris=null;
        if(wreck.seed()==0)return;
        var debris=DEBRIS.get(wreck);
        if(debris==null){
            var rig=asset("vehicle-explosion","ctywide");
            if(rig==null)return;
            debris=new Debris(rig,wreck.origin(),wreck.yaw(),wreck.seed());
            DEBRIS.put(wreck,debris);
        }
        debris.advance((wreck.tickCount+partial)/20f);
        if(debris.age>=VehicleWreck.DEBRIS_SECONDS||debris.pieces.isEmpty())return;
        state.debris=debris;
        state.offset=debris.origin.subtract(new Vec3(state.x,state.y,state.z));
    }
    @Override public void submit(State state,PoseStack stack,SubmitNodeCollector collector,CameraRenderState camera){
        super.submit(state,stack,collector,camera);
        if(state.isInvisible)return;
        if(state.model>=0){
            var kind=JakVehicle.Model.values()[state.model];
            var model=asset(kind.asset,kind.folder);
            if(model!=null){
                stack.pushPose();
                stack.mulPose(new Matrix4f().rotation(state.carcass));
                stack.scale(.64f,.64f,.64f);
                // zoomers.rs draw: a wreck's colour [64, 64, 64], its body and its wheels.
                GltfModel.colourNext(0x404040);
                model.submit(RESTS.get(kind.asset),stack,collector,state.lightCoords,1,OverlayTexture.NO_OVERLAY,state.mask);
                var wheel=state.wheels==null?null:asset(kind.asset+"-wheel","wasall");
                if(wheel!=null)for(var value:state.wheels){
                    var w=value.getAsJsonObject();var at=w.getAsJsonArray("pos");var q=w.getAsJsonArray("rotation");
                    stack.pushPose();
                    stack.translate(at.get(0).getAsFloat(),at.get(1).getAsFloat(),at.get(2).getAsFloat());
                    stack.mulPose(new Matrix4f().rotation(new Quaternionf(q.get(0).getAsFloat(),q.get(1).getAsFloat(),q.get(2).getAsFloat(),q.get(3).getAsFloat())));
                    float scale=w.get("scale").getAsFloat();stack.scale(scale,scale,scale);
                    GltfModel.colourNext(0x404040);
                    wheel.submit(RESTS.get(kind.asset+"-wheel"),stack,collector,state.lightCoords);
                    stack.popPose();
                }
                stack.popPose();
            }
        }
        if(state.debris!=null){
            stack.pushPose();
            stack.translate(state.offset.x,state.offset.y,state.offset.z);
            stack.mulPose(new Matrix4f().rotation(new Quaternionf().rotationY(state.yaw)));
            stack.scale(.64f,.64f,.64f);
            // zoomers.rs draw: the pieces skinned [0, 0, 0] (`color-mult` zero), black.
            GltfModel.colourNext(0);
            state.debris.rig.submit(state.debris.skin(),stack,collector,state.lightCoords);
            stack.popPose();
        }
    }

    private static GltfModel asset(String name,String folder){
        if(ASSETS.containsKey(name))return ASSETS.get(name);
        GltfModel model=null;
        try{
            model=new GltfModel(RuntimeConfig.assets.resolve("levels/"+folder+"/"+name+"-lod0.glb"),name,true);
            if(name.equals("snake")||folder.startsWith("ctycar"))model.loadSegments(RuntimeConfig.assets.resolve("raw_obj/"+name+"-ag.go"),name+"-lod0");
            var rest=model.pose(model.hasClip(name+"-idle")?name+"-idle":"",0);
            RESTS.put(name,rest);
            model.prepare(rest);
        }catch(IOException failure){
            if(!missing)dev.jakmc.JakMod.LOG.warn("Vehicle wreck model {} not exported: {}",name,failure.getMessage());
            missing=true;model=null;
        }
        ASSETS.put(name,model);
        return model;
    }

    /** Rust zoomers.rs Piece: a joint flying on its own; Jak frame (x left, y up, z ahead), metres. */
    private static final class Piece {
        final int joint;
        final Vector3f at,vel;
        final Quaternionf turn=new Quaternionf();
        final Vector3f axis;
        float angle;
        Piece(int joint,Vector3f at,Vector3f vel,Vector3f axis,float angle){this.joint=joint;this.at=at;this.vel=vel;this.axis=axis;this.angle=angle;}
    }
    /**
     * Rust zoomers.rs Debris: the `vehicle-explosion` model's joints 3 to 18
     * each flying 4 s, 80 m/s² down, turning 10.2 degrees a frame about an
     * axis of their own, thrown -20..20 m/s across and 15..32 up, bouncing
     * (`hit-xz-reaction` 0.75, `hit-y-reaction` 0.7, spin halved), shrinking
     * (across over 4 s, up over 3), gone 25 m under where they set off.
     */
    private static final class Debris {
        final GltfModel rig;
        final GltfModel.Posed rest;
        final Vec3 origin;
        final Quaternionf heading;
        final List<Piece> pieces=new ArrayList<>();
        float age,clock;

        // Rust zoomers.rs spawn_debris.
        Debris(GltfModel rig,Vec3 origin,float yaw,int seed){
            this.rig=rig;this.origin=origin;
            heading=new Quaternionf().rotationY(yaw);
            rest=RESTS.get("vehicle-explosion");
            int[] s={seed};
            for(int joint=3;joint<=18&&joint<rig.jointCount();joint++){
                var at=rig.jointAt(rest,joint).getTranslation(new Vector3f());
                float ahead=-20+40*r(s),left=-20+40*r(s),up=15+17*r(s);
                float ax=rand(s),ay=rand(s),az=rand(s);
                // The map's (ahead, left, up) are Jak's (z, x, y).
                var axis=new Vector3f(ay,az,ax);
                if(axis.lengthSquared()<1e-12f)axis.set(0,1,0);else axis.normalize();
                pieces.add(new Piece(joint,at,new Vector3f(left,up,ahead),axis,(float)Math.toRadians(10.2)));
            }
        }
        private static float rand(int[] s){s[0]=s[0]*1_664_525+1_013_904_223;return (s[0]>>>8)/(float)(1<<24)*2-1;}
        private static float r(int[] s){return (rand(s)+1)*.5f;}
        void advance(float until){
            while(clock+1f/60<=until&&age<VehicleWreck.DEBRIS_SECONDS&&!pieces.isEmpty()){clock+=1f/60;step(1f/60);}
        }
        // Rust zoomers.rs Debris::step.
        void step(float dt){
            age+=dt;
            var level=Minecraft.getInstance().level;
            for(var p:pieces){
                p.vel.y-=80*dt;
                var before=new Vector3f(p.at);
                p.at.add(new Vector3f(p.vel).mul(dt));
                p.turn.premul(new Quaternionf().rotationAxis(p.angle*dt*60,p.axis)).normalize();
                Vec3 prev=world(before),now=world(p.at);
                if(p.vel.y<0&&level!=null){
                    Vec3 probe=prev.add(0,.5*METRE,0);
                    double reach=(prev.y-now.y)/METRE+.5;
                    var hit=level.clip(new ClipContext(probe,probe.add(0,-reach*METRE,0),ClipContext.Block.COLLIDER,ClipContext.Fluid.NONE,CollisionContext.empty()));
                    if(hit.getType()==HitResult.Type.BLOCK){
                        double ground=hit.getLocation().y;
                        if(now.y<=ground){
                            p.at.y+=(float)((ground-now.y)/METRE)+.04f;
                            p.vel.x*=.75f;p.vel.z*=.75f;
                            p.vel.y=-p.vel.y*.7f;
                            p.angle*=.5f;
                        }
                    }
                }
            }
            pieces.removeIf(p->p.at.y<=-25);
        }
        private Vec3 world(Vector3f local){
            var v=heading.transform(new Vector3f(local)).mul((float)METRE);
            return origin.add(v.x,v.y,v.z);
        }
        // Rust zoomers.rs Debris::skin: each piece where it flew, turned and shrunk; the others hidden.
        GltfModel.Posed skin(){
            float across=Math.max(1-age/VehicleWreck.DEBRIS_SECONDS,1e-4f),up=Math.max(1-age/(.75f*VehicleWreck.DEBRIS_SECONDS),1e-4f);
            var globals=new Matrix4f[rig.jointCount()];
            for(int j=0;j<globals.length;j++)globals[j]=j>=3&&j<=18?new Matrix4f().zero():new Matrix4f(rig.jointAt(rest,j));
            for(var p:pieces){
                var linear=new Matrix4f(rig.jointAt(rest,p.joint)).setTranslation(0,0,0);
                globals[p.joint]=new Matrix4f().translation(p.at).scale(across,up,across).rotate(p.turn).mul(linear);
            }
            return rig.withJointGlobals(rest,globals);
        }
    }
}
