package dev.jakmc.client.model;

import com.mojang.blaze3d.platform.NativeImage;
import com.mojang.blaze3d.vertex.PoseStack;
import dev.jakmc.JakData;
import dev.jakmc.RuntimeConfig;
import dev.jakmc.client.JakEffectTypes;
import dev.jakmc.client.JakPoses;
import dev.jakmc.entity.JakHellcat;
import dev.jakmc.nativebridge.NativeJak;
import java.io.IOException;
import java.io.UncheckedIOException;
import java.nio.ByteBuffer;
import java.util.Arrays;
import java.util.HashMap;
import java.util.Map;
import java.util.WeakHashMap;
import net.fabricmc.fabric.api.client.rendering.v1.EntityRendererRegistry;
import net.minecraft.client.Minecraft;
import net.minecraft.client.renderer.SubmitNodeCollector;
import net.minecraft.client.renderer.entity.EntityRenderer;
import net.minecraft.client.renderer.entity.EntityRendererProvider;
import net.minecraft.client.renderer.entity.state.EntityRenderState;
import net.minecraft.client.renderer.state.level.CameraRenderState;
import net.minecraft.client.renderer.texture.DynamicTexture;
import net.minecraft.client.renderer.texture.OverlayTexture;
import net.minecraft.resources.Identifier;
import org.joml.Quaternionf;
import org.joml.Vector3f;
import net.minecraft.world.phys.Vec3;

public final class HellcatRenderer extends EntityRenderer<JakHellcat,HellcatRenderer.State> {
    private static final float MODEL_SCALE=.64f;
    // h-hellcat-constants in the original car.gc, converted from 6400 units per block.
    private static final float[][] HEAD={{5734.4f,1843.2f,15769.6f},{-5734.4f,1843.2f,15769.6f},{0,6963.2f,6922.24f}};
    private static final float[][] TAIL={{10362.88f,4710.4f,-16998.4f},{-10362.88f,4710.4f,-16998.4f}};
    private static final ByteBuffer SPRITES=NativeJak.buffer(120*36);
    private static final Map<String,Identifier> TEXTURES=new HashMap<>();
    private final Map<JakHellcat,Orientation> orientations=new WeakHashMap<>();
    private GltfModel model;
    private GltfModel.Posed rest;

    public static void initialize(){EntityRendererRegistry.register(JakHellcat.TYPE,HellcatRenderer::new);}
    public HellcatRenderer(EntityRendererProvider.Context context){super(context);shadowRadius=1.65f;}
    public static class State extends EntityRenderState {
        final Quaternionf attitude=new Quaternionf();
        float flame,lamps;
        int stage;
        boolean wrecked;
        String beams="";
    }
    private static class Orientation {
        final Quaternionf from,to;
        long since;
        Orientation(Quaternionf q,long time){from=new Quaternionf(q);to=new Quaternionf(q);since=time;}
        Quaternionf sample(long now,Quaternionf target){
            return from.slerp(to,Math.clamp((now-since)/50_000_000f,0,1),target);
        }
    }

    @Override public State createRenderState(){return new State();}
    @Override public void extractRenderState(JakHellcat craft,State state,float partial){
        super.extractRenderState(craft,state,partial);
        var client=Minecraft.getInstance();
        long now=System.nanoTime();
        var orient=orientations.computeIfAbsent(craft,e->new Orientation(new Quaternionf(e.attitude()),now));
        if(!orient.to.equals(craft.attitude())){
            orient.sample(now,state.attitude);
            orient.from.set(state.attitude);
            orient.to.set(craft.attitude());
            orient.since=now;
        }
        orient.sample(now,state.attitude);
        var pose=JakPoses.get(craft.pilotId());
        if(pose!=null&&client.player!=null&&craft.pilotId()==client.player.getId()
            &&pose.raw().get("hellcat") instanceof com.google.gson.JsonObject channel
            &&channel.get("entity").getAsInt()==craft.getId()){
            Vec3 presented=JakPoses.presented();
            if(presented!=null){
                Vec3 at=presented.add(dev.jakmc.player.JakPlayers.vector(channel.getAsJsonArray("offset")));
                state.x=at.x;state.y=at.y;state.z=at.z;
            }
        }
        state.flame=craft.thrust();
        state.lamps=craft.lamps();
        state.wrecked=craft.wrecked();
        state.stage=Math.min(2,(int)((1-Math.clamp(craft.hullHealth(),0,1))*3));
        state.beams=craft.beamData();
    }
    @Override public void submit(State state,PoseStack stack,SubmitNodeCollector collector,CameraRenderState camera){
        super.submit(state,stack,collector,camera);
        if(state.isInvisible)return;
        if(model==null){
            try(var input=HellcatRenderer.class.getResourceAsStream("/assets/jakmc/hellcat/deck.json")){
                if(input==null)throw new IOException("Missing locally imported HellCat hull data");
                var hull=com.google.gson.JsonParser.parseReader(new java.io.InputStreamReader(input,java.nio.charset.StandardCharsets.UTF_8));
                var values=hull.getAsJsonObject().getAsJsonArray("stages");
                int[] stages=new int[values.size()];
                for(int i=0;i<stages.length;i++)stages[i]=values.get(i).getAsInt();
                model=new GltfModel(RuntimeConfig.assets.resolve("levels/factorya/hellcat-lod0.glb"),"hellcat",true);
                model.vehicleDamageStages(stages);
            }catch(IOException e){throw new UncheckedIOException(e);}
            rest=model.pose("",0);
            model.prepare(rest);
        }
        stack.pushPose();
        stack.mulPose(new org.joml.Matrix4f().rotation(state.attitude));
        stack.scale(MODEL_SCALE,MODEL_SCALE,MODEL_SCALE);
        model.submit(rest,stack,collector,state.lightCoords,1,OverlayTexture.NO_OVERLAY,1L<<state.stage);
        stack.popPose();
        if(!state.wrecked)effects(state,stack,collector,camera);
        beams(state,stack,collector,camera);
    }

    private static Identifier texture(String name){
        return TEXTURES.computeIfAbsent(name,n->{
            var id=JakData.id("local/hellcat/"+n);
            var path=RuntimeConfig.assets.resolve("textures/level-default-sprite").resolve(n+".png");
            try(var in=java.nio.file.Files.newInputStream(path)){
                Minecraft.getInstance().getTextureManager().register(id,new DynamicTexture(id::toString,NativeImage.read(in)));
                return id;
            }catch(IOException e){throw new UncheckedIOException(e);}
        });
    }

    private static void effects(State state,PoseStack stack,SubmitNodeCollector collector,CameraRenderState camera){
        int count=NativeJak.hellcatFlames(state.flame,(int)(state.ageInTicks*3),SPRITES);
        if(count==0&&state.lamps<=0)return;
        var cameraView=Minecraft.getInstance().gameRenderer.mainCamera();
        var right=new Vector3f(cameraView.leftVector()).negate();
        var up=new Vector3f(cameraView.upVector());
        var origin=new Vec3(state.x,state.y,state.z);
        Batch core=new Batch(),glow=new Batch(),head=new Batch(),tail=new Batch();
        for(int i=0;i<count;i++){
            int b=i*36;
            var at=state.attitude.transform(new Vector3f(SPRITES.getFloat(b),SPRITES.getFloat(b+4),SPRITES.getFloat(b+8)));
            Batch batch=SPRITES.getFloat(b+32)==0?core:glow;
            batch.quad(at,right,up,SPRITES.getFloat(b+12),
                rgba(SPRITES.getFloat(b+16),SPRITES.getFloat(b+20),SPRITES.getFloat(b+24),SPRITES.getFloat(b+28)));
        }
        if(state.lamps>0){
            lamps(state,HEAD,true,origin,camera.pos,right,up,head);
            lamps(state,TAIL,false,origin,camera.pos,right,up,tail);
        }
        core.submit("laser-hit2",2,stack,collector);
        glow.submit("glow",2,stack,collector);
        head.submit("glow",3,stack,collector);
        tail.submit("glow-hotdot",3,stack,collector);
    }

    private static void beams(State state,PoseStack stack,SubmitNodeCollector collector,CameraRenderState camera){
        if(state.beams.isEmpty())return;
        ByteBuffer bytes=ByteBuffer.wrap(java.util.Base64.getDecoder().decode(state.beams));
        Vec3 offset=new Vec3(bytes.getDouble()-state.x,bytes.getDouble()-state.y,bytes.getDouble()-state.z);
        Vec3 eye=camera.pos.subtract(state.x,state.y,state.z);
        var batch=new Batch();
        while(bytes.remaining()>=32){
            Vec3 head=offset.add(bytes.getFloat(),bytes.getFloat(),bytes.getFloat());
            Vec3 tail=offset.add(bytes.getFloat(),bytes.getFloat(),bytes.getFloat());
            int color=Integer.rotateRight(bytes.getInt(),8);
            float width=bytes.getFloat()*.64f;
            Vec3 half=head.subtract(tail).scale(.5);
            Vec3 center=tail.add(half);
            Vec3 side=half.cross(eye.subtract(center)).normalize().scale(width*.15);
            batch.rect(center.toVector3f(),half.toVector3f(),side.toVector3f(),color);
        }
        batch.submit("glow",3,stack,collector);
    }

    private static void lamps(State state,float[][] points,boolean front,Vec3 origin,Vec3 eye,Vector3f right,Vector3f up,Batch batch){
        var level=Minecraft.getInstance().level;
        if(level==null)return;
        for(var point:points){
            var at=state.attitude.transform(new Vector3f(point).div(6400));
            var location=origin.add(at.x,at.y,at.z);
            Vec3 toward=eye.subtract(location);
            float metres=(float)(toward.length()/.64);
            float factor=state.lamps*Math.clamp(8.997864f-.04998922f*metres,0,1);
            if(factor<=0)continue;
            toward=toward.normalize().scale(front?.128:.064);
            at.add((float)toward.x,(float)toward.y,(float)toward.z);
            var target=location.add(toward);
            var context=new net.minecraft.world.level.ClipContext(eye,target,
                net.minecraft.world.level.ClipContext.Block.COLLIDER,
                net.minecraft.world.level.ClipContext.Fluid.NONE,Minecraft.getInstance().getCameraEntity());
            if(level.clip(context).getType()!=net.minecraft.world.phys.HitResult.Type.MISS)continue;
            batch.quad(at,right,up,front?1.28f:.48f,rgba(255,front?224:32,0,18*factor));
            if(front)batch.quad(at,right,up,.512f,rgba(255,224,144,34*state.lamps*Math.clamp(2.3332994f-.05554176f*metres,0,1)));
        }
    }

    private static int rgba(float r,float g,float b,float a){
        return Math.clamp(Math.round(a),0,255)<<24|Math.clamp(Math.round(r),0,255)<<16
            |Math.clamp(Math.round(g),0,255)<<8|Math.clamp(Math.round(b),0,255);
    }

    private static final class Batch {
        float[] xyz=new float[192];
        int[] colors=new int[16];
        int count;
        void quad(Vector3f at,Vector3f right,Vector3f up,float size,int color){
            if(size<=0)return;
            float half=size/2;
            rect(at,new Vector3f(right).mul(half),new Vector3f(up).mul(half),color);
        }
        void rect(Vector3f at,Vector3f right,Vector3f up,int color){
            if((color>>>24)==0)return;
            if(count==colors.length){colors=Arrays.copyOf(colors,count*2);xyz=Arrays.copyOf(xyz,count*24);}
            int i=count*12;
            for(int j=0;j<4;j++){
                int x=j==0||j==3?-1:1;
                int y=j<2?-1:1;
                xyz[i++]=at.x+right.x*x+up.x*y;
                xyz[i++]=at.y+right.y*x+up.y*y;
                xyz[i++]=at.z+right.z*x+up.z*y;
            }
            colors[count++]=color;
        }
        void submit(String name,int blend,PoseStack stack,SubmitNodeCollector collector){
            if(count==0)return;
            collector.submitCustomGeometry(stack,JakEffectTypes.get(texture(name),blend),(pose,consumer)->{
                var v=new Vector3f();
                for(int i=0;i<count*4;i++){
                    pose.pose().transformPosition(xyz[i*3],xyz[i*3+1],xyz[i*3+2],v);
                    int j=i%4;
                    consumer.addVertex(v.x,v.y,v.z,colors[i/4],j==0||j==3?0:1,j<2?1:0,
                        OverlayTexture.NO_OVERLAY,0x00f000f0,0,1,0);
                }
            });
        }
    }
}
