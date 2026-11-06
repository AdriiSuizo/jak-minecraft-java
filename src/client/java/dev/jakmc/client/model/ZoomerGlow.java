package dev.jakmc.client.model;

import com.google.gson.JsonObject;
import com.mojang.blaze3d.platform.NativeImage;
import com.mojang.blaze3d.vertex.PoseStack;
import dev.jakmc.JakData;
import dev.jakmc.RuntimeConfig;
import dev.jakmc.client.JakEffectTypes;
import java.io.IOException;
import java.io.UncheckedIOException;
import net.minecraft.client.Minecraft;
import net.minecraft.client.renderer.SubmitNodeCollector;
import net.minecraft.client.renderer.state.level.CameraRenderState;
import net.minecraft.client.renderer.texture.DynamicTexture;
import net.minecraft.client.renderer.texture.OverlayTexture;
import net.minecraft.resources.Identifier;
import net.minecraft.world.phys.Vec3;
import org.joml.Quaternionf;
import org.joml.Vector3f;

/**
 * The glow of a zoomer's thruster flames: vehicle-draw-thruster's sprite-glow at a quarter of the way down the
 * flame (vehicle-effects.gc {@code *vehicle-thruster-glow-template*}: texture 15 of the default sprite page,
 * glow-soft; colour 255, 0 to 64, 0; size 4 flame widths; alpha 16 times the thrust and 4 at random; faded out
 * with distance as the headlights' glow is). The flame's blades are native particles (ZoomerEngineFx); the glow
 * is a camera-facing sprite, so it is drawn with the vehicle.
 */
final class ZoomerGlow {
    private ZoomerGlow(){}
    private static Identifier texture;
    private static final double METRE=.64;

    private static Identifier texture(){
        if(texture!=null)return texture;
        var id=JakData.id("local/zoomer/glow-soft");
        var path=RuntimeConfig.assets.resolve("textures/level-default-sprite").resolve("glow-soft.png");
        try(var in=java.nio.file.Files.newInputStream(path)){
            Minecraft.getInstance().getTextureManager().register(id,new DynamicTexture(id::toString,NativeImage.read(in)));
            return texture=id;
        }catch(IOException e){throw new UncheckedIOException(e);}
    }

    /** {@code rand-vu} of the 60 Hz frame this is drawn in: the same all through it, and not the render rate's flicker. */
    private static float random(long salt){
        long frame=System.nanoTime()*60/1_000_000_000L;
        long x=(frame*0x9E3779B97F4A7C15L)^(salt*0xC2B2AE3D27D4EB4FL);
        x^=x>>>33;x*=0xFF51AFD7ED558CCDL;x^=x>>>33;
        return (x>>>40)/(float)(1<<24);
    }

    /** One flame of `vehicle-draw-thruster`: where it hangs, where it burns, how big, how hard. */
    private record Flame(Vector3f at,Vector3f direction,float width,float length,float thrust,int index){}

    private static float lerp(JsonObject before,JsonObject after,String key,int index,float blend){
        if(!after.has(key))return 0;
        float to=after.getAsJsonArray(key).get(index).getAsFloat();
        float from=before!=null&&before.has(key)?before.getAsJsonArray(key).get(index).getAsFloat():to;
        return from+(to-from)*blend;
    }

    /**
     * The glow of every thruster flame of a hover zoomer: the two rear thrusters (hvehicle-method-78, burning along the
     * vehicle's back) and, on the bikes, the roll thrusters that are firing (h-bike-base's vehicle-method-78, burning down
     * and out of their side). The sprite is alpha {@code fog-fade} times 16 times the thrust and 4 at random.
     * @param before @param after the two hover frames the vehicle is drawn between, @param blend how far,
     * @param origin where the bike is drawn (hull and lift), @param attitude how, @param thrust the rear thrusters' power,
     * @param fogFade the vehicle's {@code fog-fade} (JakAtmosphere.fogFade).
     */
    static void thrusters(JsonObject before,JsonObject after,float blend,Quaternionf attitude,Vec3 origin,float thrust,float fogFade,int salt,
                          CameraRenderState camera,PoseStack stack,SubmitNodeCollector collector){
        var flames=new java.util.ArrayList<Flame>(4);
        if(thrust>0&&after.has("flame")&&after.has("thrusters")){
            var flame=after.getAsJsonArray("flame");
            var points=after.getAsJsonArray("thrusters");
            for(int i=0;i<points.size();i++){
                var p=points.get(i).getAsJsonArray();
                flames.add(new Flame(new Vector3f(p.get(0).getAsFloat(),p.get(1).getAsFloat(),p.get(2).getAsFloat()),
                    new Vector3f(0,0,-1),flame.get(0).getAsFloat(),flame.get(1).getAsFloat(),thrust,i));
            }
        }
        if(after.has("rollFlames")&&after.has("rollFlame")&&after.has("rollThrust")){
            var table=after.getAsJsonArray("rollFlames");
            var size=after.getAsJsonArray("rollFlame");
            for(int k=0;k<table.size()&&k<2;k++){
                float push=lerp(before,after,"rollThrust",k,blend);
                if(push<=0)continue;
                var row=table.get(k).getAsJsonArray();
                // vehicle-draw-thruster turns the frame half a turn about x: the flame burns against the thruster's aim.
                flames.add(new Flame(new Vector3f(row.get(0).getAsFloat(),row.get(1).getAsFloat(),row.get(2).getAsFloat()),
                    new Vector3f(row.get(3).getAsFloat(),row.get(4).getAsFloat(),row.get(5).getAsFloat()).negate(),
                    size.get(0).getAsFloat(),size.get(1).getAsFloat(),push,2+k));
            }
        }
        if(flames.isEmpty())return;
        var view=Minecraft.getInstance().gameRenderer.mainCamera();
        var right=new Vector3f(view.leftVector()).negate();
        var up=new Vector3f(view.upVector());
        var level=Minecraft.getInstance().level;
        if(level==null)return;
        float[] xyz=new float[flames.size()*12];
        int[] colors=new int[flames.size()];
        int count=0;
        for(var flame:flames){
            int i=flame.index;
            // vehicle-draw-thruster: one random for the flame's length and width, the glow a quarter down it.
            float growth=1+.33f*random(salt*7L+i);
            float flameLength=growth*flame.length*flame.thrust,flameWidth=Math.min(flame.width*growth,flameLength/2);
            var local=attitude.transform(new Vector3f(flame.at).mul((float)METRE));
            var along=attitude.transform(new Vector3f(flame.direction));
            var at=new Vector3f(local).fma((float)(METRE*flameLength/4),along);
            Vec3 location=origin.add(at.x,at.y,at.z);
            double metres=location.distanceTo(camera.pos)/METRE;
            float fade=(float)Math.clamp(8.997864-.04998922*metres,0,1);
            if(fade<=0)continue;
            var clear=level.clip(new net.minecraft.world.level.ClipContext(camera.pos,location,
                net.minecraft.world.level.ClipContext.Block.COLLIDER,net.minecraft.world.level.ClipContext.Fluid.NONE,
                Minecraft.getInstance().getCameraEntity()));
            if(clear.getType()!=net.minecraft.world.phys.HitResult.Type.MISS)continue;
            float alpha=fade*fogFade*(16*flame.thrust+4*random(salt*13L+i));
            int green=Math.round(64*random(salt*17L+i));
            float half=(float)(4*flameWidth*METRE/2);
            int base=count*12;
            for(int j=0;j<4;j++){
                int x=j==0||j==3?-1:1,y=j<2?-1:1;
                xyz[base+j*3]=at.x+right.x*half*x+up.x*half*y;
                xyz[base+j*3+1]=at.y+right.y*half*x+up.y*half*y;
                xyz[base+j*3+2]=at.z+right.z*half*x+up.z*half*y;
            }
            colors[count++]=Math.clamp(Math.round(alpha),0,255)<<24|255<<16|green<<8;
        }
        if(count==0)return;
        final int drawn=count;
        collector.submitCustomGeometry(stack,JakEffectTypes.get(texture(),3),(pose,consumer)->{
            var v=new Vector3f();
            for(int i=0;i<drawn*4;i++){
                pose.pose().transformPosition(xyz[i*3],xyz[i*3+1],xyz[i*3+2],v);
                int j=i%4;
                consumer.addVertex(v.x,v.y,v.z,colors[i/4],j==0||j==3?0:1,j<2?1:0,
                    OverlayTexture.NO_OVERLAY,0x00f000f0,0,1,0);
            }
        });
    }
}
