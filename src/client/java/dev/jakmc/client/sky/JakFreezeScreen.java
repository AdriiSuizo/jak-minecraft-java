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
import dev.jakmc.client.JakInput;
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
 * Light Jak's freeze on screen, for the local Jak (target-lightjak.gc freeze-control, blit-displays.gc).
 * While the world is slowed the colour filter goes to (1, 1, 1.5) and the new picture is laid over the last
 * one shown by OpenGOAL's slow_time fan (ISC, third_party/opengoal): full at the centre at 3/4 brightness,
 * `amount` at the corners, so what moves leaves trails towards the edges. The clap (aframe 20) buzzes the
 * pad for 0.5 s and smushes the camera (`activate! *camera-smush-control* 819.2 15 75 1.0 0.9`).
 */
public final class JakFreezeScreen {
 private static final RenderPipeline PIPELINE=RenderPipelines.register(RenderPipeline.builder(RenderPipelines.GLOBALS_SNIPPET)
  .withLocation(JakData.id("pipeline/jak_freeze")).withVertexShader("core/screenquad").withFragmentShader(JakData.id("core/jak_freeze"))
  .withBindGroupLayout(BindGroupLayouts.IN_SAMPLER).withBindGroupLayout(BindGroupLayouts.DYNAMIC_TRANSFORMS)
  .withPrimitiveTopology(PrimitiveTopology.TRIANGLES).withColorTargetState(new ColorTargetState(BlendFunction.TRANSLUCENT)).build());
 private static TextureTarget picture,shown;
 private static long clapped=Long.MIN_VALUE,smushStart,nukeSeen=Long.MIN_VALUE,nukeSmushAt=Long.MAX_VALUE,nukeFlashStart=Long.MIN_VALUE;
 private static boolean smushing;
 // The running smush-control's activate! arguments: amplitude (blocks), period and duration (300ths of a second), the two multipliers.
 private static double smushAmplitude;private static float smushPeriod,smushDuration,smushAmpMul,smushPeriodMul;
 private static void smushStart(double metres,float period,float duration,float ampMul,float periodMul){
  smushStart=System.nanoTime();smushing=true;smushAmplitude=metres*dev.jakmc.player.JakPlayers.METRE;
  smushPeriod=period;smushDuration=duration;smushAmpMul=ampMul;smushPeriodMul=periodMul;
 }
 /** Frames drawn slowed (scenes). */
 public static volatile int frames;
 private JakFreezeScreen(){}

 /** freeze-control's post: `slow-time` = lerp-scale 1..0 over the clock 0.05..1, blit-displays' slow-time is 1 minus it. */
 private static float shownTime(){
  var c=Minecraft.getInstance();
  var pose=c.player==null?null:JakPoses.get(c.player.getId());
  if(pose==null||!pose.raw().has("entityRate"))return 1;
  float clock=pose.raw().get("entityRate").getAsFloat();
  float slow=Math.clamp(1-(clock-.05f)/.95f,0,1);
  return 1-slow;
 }

 /** The clap's moment, once per freeze: the pad's buzz and the camera's smush. */
 public static void frame(){
  var c=Minecraft.getInstance();
  var pose=c.player==null?null:JakPoses.get(c.player.getId());
  if(pose!=null&&pose.raw().has("freezeClap")&&pose.raw().get("freezeClap").isJsonPrimitive()){
   long clap=pose.raw().get("freezeClap").getAsLong();
   if(clap!=clapped){
    clapped=clap;
    JakInput.hurtFeedback(.5f);
    smushStart(.2,15,75,1,.9f);
   }
  }
  // Jak 3 gun-dark-shot.gc, the Super Nova's impact: the white flash, the pad's buzz (255, fading over a
  // second) and at 1 s `activate! *camera-smush-control* 1228.8 45 1275 1.0 0.9`.
  if(pose!=null&&pose.raw().get("nukeFlash") instanceof com.google.gson.JsonPrimitive age){
   long now=System.nanoTime(),start=now-(long)(age.getAsFloat()*1e9);
   if(Math.abs(start-nukeSeen)>200_000_000L){nukeSeen=start;nukeFlashStart=start;nukeSmushAt=start+1_000_000_000L;JakInput.hurtFeedback(1);}
  }
  if(System.nanoTime()>=nukeSmushAt){nukeSmushAt=Long.MAX_VALUE;smushStart(.3,45,1275,1,.9f);}
 }
 /** The Super Nova's white flash now, 0..1: up to full by 5 % of its 2 s, held to 20 %, 0.78 at 50 %, gone at the end. */
 public static float nukeFlash(){
  if(nukeFlashStart==Long.MIN_VALUE)return 0;
  float t=(System.nanoTime()-nukeFlashStart)/2e9f;
  if(t<0||t>=1)return 0;
  if(t<.05f)return t/.05f;
  if(t<.2f)return 1;
  if(t<.5f)return 1+(.78f-1)*(t-.2f)/.3f;
  return .78f*(1-(t-.5f)/.5f);
 }

 /** smush-control update!: a sine of the period, fading over the duration; its period shrinks by 0.9 a cycle. Blocks up. */
 public static double smush(){
  if(!smushing)return 0;
  float elapsed=(System.nanoTime()-smushStart)/1e9f*300;
  float duration=smushDuration,period=smushPeriod,ticks=0;double amplitude=smushAmplitude;
  // update! once a period has gone since the last change: amp ×ampMul, period ×periodMul.
  while(elapsed-ticks>=period&&period>.5f){ticks+=period;period*=smushPeriodMul;amplitude*=smushAmpMul;}
  if(elapsed>=duration){smushing=false;return 0;}
  float phase=elapsed%period;
  return Math.sin(2*Math.PI*phase/period)*amplitude*(duration-elapsed)/duration;
 }

 public static void apply(RenderTarget main){
  float time=shownTime();
  if(time>=1||main.getColorTexture()==null){
   if(shown!=null){shown.destroyBuffers();shown=null;}
   return;
  }
  frames++;
  float amount=.05f+(1-.05f)*time;
  float blue=1.5f+(1-1.5f)*time;
  if(picture==null)picture=new TextureTarget("Jak freeze",main.width,main.height,GpuFormat.RGBA8_UNORM,null);
  else if(picture.width!=main.width||picture.height!=main.height)picture.resize(main.width,main.height);
  picture.copyColorFrom(main);
  if(shown==null){shown=new TextureTarget("Jak freeze shown",main.width,main.height,GpuFormat.RGBA8_UNORM,null);shown.copyColorFrom(main);}
  else if(shown.width!=main.width||shown.height!=main.height){shown.resize(main.width,main.height);shown.copyColorFrom(main);}
  // The last picture shown, then the new one over it through the fan.
  main.copyColorFrom(shown);
  var data=new Matrix4f(amount,blue,0,0,0,0,0,0,0,0,0,0,0,0,0,0);
  var transforms=RenderSystem.getDynamicUniforms().writeTransform(new Matrix4f(),new Vector4f(1),new Vector3f(),data);
  try(var pass=RenderSystem.getDevice().createCommandEncoder().createRenderPass(()->"Jak freeze",main.getColorTextureView(),Optional.empty(),null,OptionalDouble.empty())){
   pass.setPipeline(RenderSystem.getCompiledPipeline(PIPELINE));
   RenderSystem.bindDefaultUniforms(pass);
   pass.setUniform("DynamicTransforms",transforms);
   pass.setUniform("InSampler",picture.getColorTextureView(),RenderSystem.getSamplerCache().getClampToEdge(FilterMode.NEAREST));
   pass.draw(3,1,0,0);
  }
  shown.copyColorFrom(main);
 }
}
