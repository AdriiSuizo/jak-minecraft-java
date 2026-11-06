package dev.jakmc.client.sky;

import com.mojang.blaze3d.pipeline.RenderTarget;
import com.mojang.blaze3d.pipeline.TextureTarget;
import com.mojang.blaze3d.systems.RenderSystem;
import com.mojang.renderpearl.api.GpuFormat;
import com.mojang.renderpearl.api.pipeline.BlendFunction;
import com.mojang.renderpearl.api.pipeline.ColorTargetState;
import com.mojang.renderpearl.api.pipeline.PrimitiveTopology;
import com.mojang.renderpearl.api.pipeline.RenderPipeline;
import com.mojang.renderpearl.api.textures.FilterMode;
import dev.jakmc.JakData;
import dev.jakmc.client.JakPoses;
import java.util.Optional;
import java.util.OptionalDouble;
import net.minecraft.client.Minecraft;
import net.minecraft.client.renderer.BindGroupLayouts;
import net.minecraft.client.renderer.RenderPipelines;
import org.joml.Matrix4f;
import org.joml.Vector3f;
import org.joml.Vector4f;

/**
 * The Super Nova's screen, Jak 3 gun-dark-shot.gc `impact` / `impact-small` from the state's start:
 * `do-white-screen-effect` (the fade curve over the flash time, as *screen-filter*), `do-blur-effect`
 * (`setup-zoom-blur-2d` at the blast, 10 texels, alpha 1 minus the blur curve over the blur time, drawn as
 * OpenGOAL's BlitDisplays::do_zoom_blur: the last picture shown zoomed by the texels towards the blast, the new
 * one over it at that alpha) and `set-setting! 'music-volume 'abs 0.0` while the state lasts (10 s, small 8 s).
 * Curves and times from gun-part.gc (*gun-dark-3-nuke-fade-curve*, -blur-curve, their -small ones).
 */
public final class JakNukeScreen {
 private static final RenderPipeline PIPELINE=RenderPipelines.register(RenderPipeline.builder(RenderPipelines.GLOBALS_SNIPPET)
  .withLocation(JakData.id("pipeline/jak_zoom_blur")).withVertexShader("core/screenquad").withFragmentShader(JakData.id("core/jak_zoom_blur"))
  .withBindGroupLayout(BindGroupLayouts.IN_SAMPLER).withBindGroupLayout(BindGroupLayouts.DYNAMIC_TRANSFORMS)
  .withPrimitiveTopology(PrimitiveTopology.TRIANGLES).withColorTargetState(new ColorTargetState(BlendFunction.TRANSLUCENT)).build());
 // *gun-dark-3-nuke-fade-curve* (both sizes alike): time, r, g, b, a.
 private static final float[][] FADE={{0,1,1,1,0},{.05f,1,1,1,1},{.2f,1,1,1,1},{.5f,1,1,1,.78125f},{1,1,.78125f,0,0}};
 // *gun-dark-3-nuke-blur-curve* (both sizes alike).
 private static final float[][] BLUR={{0,0},{.1f,1},{.5f,.8f},{1,0}};
 // *gun-dark-3-nuke-blur-segs*: the texels the old picture is zoomed by.
 private static final int TEXELS=10;
 private static TextureTarget fresh,shown;
 private static long start=Long.MIN_VALUE;
 private static boolean small;
 private static final Vector3f at=new Vector3f();
 private JakNukeScreen(){}

 /** The running Super Nova from the local Jak's pose ("nuke": x, y, z in blocks, age in seconds, small). */
 static void frame(JakPoses.Pose pose){
  var nuke=pose==null?null:pose.raw().get("nuke");
  if(nuke==null||!nuke.isJsonArray()){start=Long.MIN_VALUE;return;}
  var a=nuke.getAsJsonArray();
  long begun=System.nanoTime()-(long)(a.get(3).getAsFloat()*1e9);
  // The same blast keeps its start; a new one (more than 0.2 s off) starts again.
  if(start==Long.MIN_VALUE||Math.abs(begun-start)>200_000_000L)start=begun;
  small=a.get(4).getAsBoolean();
  at.set(a.get(0).getAsFloat(),a.get(1).getAsFloat(),a.get(2).getAsFloat());
  // music-volume abs 0.0 while the state runs.
  Minecraft.getInstance().getMusicManager().stopPlaying();
 }
 static long started(){return start;}
 static boolean small(){return small;}
 private static float age(){return start==Long.MIN_VALUE?-1:(System.nanoTime()-start)/1e9f;}

 /** do-white-screen-effect's colour now (ARGB), 0 when none: the fade curve over 2 s (small 6 s). */
 public static int filter(){
  float age=age();
  if(age<0)return 0;
  float t=age/(small?6:2);
  if(t>=1)return 0;
  int i=0;
  while(i<FADE.length-2&&t>FADE[i+1][0])i++;
  float[] p=FADE[i],q=FADE[i+1];
  float f=Math.clamp((t-p[0])/(q[0]-p[0]),0,1);
  int r=Math.round(255*(p[1]+(q[1]-p[1])*f)),g=Math.round(255*(p[2]+(q[2]-p[2])*f)),b=Math.round(255*(p[3]+(q[3]-p[3])*f));
  // The alpha's 128 (w × 128) is the PS2's full.
  int alpha=Math.round(255*Math.min(1,p[4]+(q[4]-p[4])*f));
  return alpha<<24|r<<16|g<<8|b;
 }

 private static float blurCurve(float t){
  if(t>=1)return BLUR[BLUR.length-1][1];
  int i=0;
  while(i<BLUR.length-2&&t>BLUR[i+1][0])i++;
  float[] p=BLUR[i],q=BLUR[i+1];
  return p[1]+(q[1]-p[1])*Math.clamp((t-p[0])/(q[0]-p[0]),0,1);
 }

 public static void apply(RenderTarget main){
  float age=age(),blurTime=small?6:5;
  if(age<0||age>=blurTime||main.getColorTexture()==null){
   if(shown!=null){shown.destroyBuffers();shown=null;}
   return;
  }
  float target=Math.min(1,1-blurCurve(age/blurTime));
  // draw-zoom-blur's 3D case: the blast on the 512 x 416 screen (mirrored when behind the camera), the zoomed
  // rectangle kept within the texels, and the alpha eased to 1 as the blast leaves the screen.
  var c=Minecraft.getInstance();
  var camera=c.gameRenderer.mainCamera();
  var eye=camera.position();
  float yaw=(float)Math.toRadians(camera.yRot()),pitch=(float)Math.toRadians(camera.xRot());
  var forward=new Vector3f(-(float)Math.sin(yaw)*(float)Math.cos(pitch),-(float)Math.sin(pitch),(float)Math.cos(yaw)*(float)Math.cos(pitch));
  var right=new Vector3f(forward).cross(0,1,0).normalize();
  var up=new Vector3f(right).cross(forward).normalize();
  var d=new Vector3f((float)(at.x-eye.x),(float)(at.y-eye.y),(float)(at.z-eye.z));
  float z=d.dot(forward);
  if(z<0){d.negate();z=-z;}
  float tan=(float)Math.tan(Math.toRadians(c.options.fov().get())*.5),aspect=(float)main.width/main.height;
  float sx=256+256*d.dot(right)/Math.max(z,1e-3f)/(tan*aspect),sy=208-208*d.dot(up)/Math.max(z,1e-3f)/tan;
  float f2=Math.clamp(TEXELS/512f*sx,0,TEXELS),f3=Math.clamp(TEXELS/416f*sy,0,TEXELS);
  float xmin=f2/512,xmax=(512-(TEXELS-f2)-1)/512,ymin=f3/416,ymax=(416-(TEXELS-f3)-1)/416;
  float offX=0,offY=0;
  if(sx<0)offX=Math.min(1,-sx/512);
  if(sx>512)offX=Math.min(1,(sx-512)/512);
  if(sy<0)offY=Math.min(1,-sy/512);
  if(sy>416)offY=Math.min(1,(sy-416)/512);
  float alpha=target+(1-target)*Math.max(offX,offY);
  if(fresh==null)fresh=new TextureTarget("Jak nuke blur new",main.width,main.height,GpuFormat.RGBA8_UNORM,null);
  else if(fresh.width!=main.width||fresh.height!=main.height)fresh.resize(main.width,main.height);
  fresh.copyColorFrom(main);
  if(shown==null){shown=new TextureTarget("Jak nuke blur shown",main.width,main.height,GpuFormat.RGBA8_UNORM,null);shown.copyColorFrom(main);}
  else if(shown.width!=main.width||shown.height!=main.height){shown.resize(main.width,main.height);shown.copyColorFrom(main);}
  // The old picture zoomed (colour 0x80, opaque), then the new one over it at the alpha.
  draw(main,shown,xmin,ymin,xmax,ymax,1);
  draw(main,fresh,0,0,1,1,alpha);
  shown.copyColorFrom(main);
 }

 private static void draw(RenderTarget main,TextureTarget from,float xmin,float ymin,float xmax,float ymax,float alpha){
  var data=new Matrix4f(xmin,ymin,xmax,ymax,alpha,0,0,0,0,0,0,0,0,0,0,0);
  var transforms=RenderSystem.getDynamicUniforms().writeTransform(new Matrix4f(),new Vector4f(1),new Vector3f(),data);
  try(var pass=RenderSystem.getDevice().createCommandEncoder().createRenderPass(()->"Jak nuke blur",main.getColorTextureView(),Optional.empty(),null,OptionalDouble.empty())){
   pass.setPipeline(RenderSystem.getCompiledPipeline(PIPELINE));
   RenderSystem.bindDefaultUniforms(pass);
   pass.setUniform("DynamicTransforms",transforms);
   pass.setUniform("InSampler",from.getColorTextureView(),RenderSystem.getSamplerCache().getClampToEdge(FilterMode.LINEAR));
   pass.draw(3,1,0,0);
  }
 }
}
