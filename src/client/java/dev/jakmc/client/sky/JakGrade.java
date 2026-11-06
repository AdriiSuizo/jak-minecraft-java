package dev.jakmc.client.sky;

import com.mojang.blaze3d.pipeline.RenderTarget;
import com.mojang.blaze3d.pipeline.TextureTarget;
import com.mojang.blaze3d.systems.RenderSystem;
import com.mojang.renderpearl.api.GpuFormat;
import com.mojang.renderpearl.api.pipeline.ColorTargetState;
import com.mojang.renderpearl.api.pipeline.PrimitiveTopology;
import com.mojang.renderpearl.api.pipeline.RenderPipeline;
import com.mojang.renderpearl.api.textures.FilterMode;
import dev.jakmc.JakData;
import dev.jakmc.client.JakSky;
import java.util.Optional;
import java.util.OptionalDouble;
import net.minecraft.client.renderer.BindGroupLayouts;
import net.minecraft.client.renderer.RenderPipelines;
import org.joml.Matrix4f;
import org.joml.Vector3f;
import org.joml.Vector4f;

/**
 * Jak Enhanced Trilogy's area grading (ISC, third_party/jak-enhanced-trilogy), as minecraft_terrain enhanced_grade.rs
 * picks it and render_gpu drawsurf/enhanced_grade.rs draws it: once over the finished world, before the HUD.
 * Visual only: Minecraft's light levels and spawning stay as they are.
 */
public final class JakGrade {
 private static final RenderPipeline PIPELINE=RenderPipelines.register(RenderPipeline.builder(RenderPipelines.GLOBALS_SNIPPET)
  .withLocation(JakData.id("pipeline/jak_grade")).withVertexShader("core/screenquad").withFragmentShader(JakData.id("core/jak_grade"))
  .withBindGroupLayout(BindGroupLayouts.IN_SAMPLER).withBindGroupLayout(BindGroupLayouts.DYNAMIC_TRANSFORMS)
  .withPrimitiveTopology(PrimitiveTopology.TRIANGLES).withColorTargetState(ColorTargetState.DEFAULT).build());
 /** IW4L_ENHANCED_GRADE: 0 off, 0..1 its strength (default 1). */
 private static final float STRENGTH=strength();
 // enhanced_grade.rs Grade: lift (rgb, saturation), gain (rgb, contrast curve), params (warmth, strength, -, on).
 static final float[][] IDENTITY={{0,0,0,1},{1,1,1,0},{0,0,0,1}};
 private static final float[][][] WILDS={
  {{0,.006f,.01f,.98f},{1.01f,1.02f,.98f,.06f},{0,0,0,1}},
  {{0,.006f,.026f,.95f},{.97f,.99f,1.05f,.06f},{-.03f,0,0,1}}};
 private static final float[][][] FOREST={
  {{0,.008f,.004f,1.12f},{1.07f,1,.86f,.06f},{.03f,0,0,1}},
  {{0,.006f,.022f,1},{.98f,1,1.04f,.05f},{-.02f,0,0,1}}};
 private static final float[][][] DESERT={
  {{.004f,.004f,.008f,1},{1.02f,1,.97f,.14f},{0,0,0,1}},
  {{0,.004f,.02f,.95f},{.98f,.98f,1.05f,.04f},{-.03f,0,0,1}}};
 private static final float[][][] SWAMP={
  {{.004f,.008f,.004f,.88f},{.98f,1,.94f,.04f},{0,0,0,1}},
  {{0,.006f,.02f,.9f},{.96f,.98f,1.02f,.05f},{-.02f,0,0,1}}};
 private static final float[][][] SNOW={
  {{0,.006f,.02f,.95f},{1,1.02f,1.06f,.05f},{-.04f,0,0,1}},
  {{0,.006f,.035f,.92f},{.97f,.99f,1.06f,.06f},{-.05f,0,0,1}}};
 private static float[][] grade=copy(IDENTITY);
 private static long last;
 private static RenderTarget picture;
 /** Dev scenes: one picture without the grade, for comparison. */
 public static boolean bypass;
 private JakGrade(){}

 private static float strength(){
  try{String v=System.getenv("IW4L_ENHANCED_GRADE");float s=v==null?1:Float.parseFloat(v.trim());return Float.isFinite(s)?Math.clamp(s,0,1):1;}
  catch(NumberFormatException e){return 1;}
 }
 private static float[][] copy(float[][] g){float[][] out=new float[3][];for(int r=0;r<3;r++)out[r]=g[r].clone();return out;}
 private static float[][] blend(float[][] a,float[][] b,float t){
  float[][] out=new float[3][4];
  for(int r=0;r<3;r++)for(int c=0;c<4;c++)out[r][c]=a[r][c]+(b[r][c]-a[r][c])*t;
  return out;
 }
 /** enhanced_grade.rs climate_of: weather's Rainy (plains and ocean too) looks like a forest only in vegetation biomes. */
 public static JakSky.Climate climateOf(String biome){
  var climate=JakSky.climateOf(biome);
  if(climate==JakSky.Climate.RAINY&&!(biome.contains("forest")||biome.contains("jungle")||biome.contains("taiga")))return JakSky.Climate.NONE;
  return climate;
 }
 /** enhanced_grade.rs target: the wilds by day or night, each zone taking its share of the samples. */
 static float[][] target(float rainy,float dry,float swamp,float cold,float hour){
  hour=((hour%24)+24)%24;
  float night=hour<12?Math.clamp((7-hour)/2,0,1):Math.clamp((hour-19)/2,0,1);
  float[] weights={rainy,dry,swamp,cold};
  float total=0;for(int i=0;i<4;i++){weights[i]=Float.isFinite(weights[i])?Math.clamp(weights[i],0,1):0;total+=weights[i];}
  total=Math.max(total,1);
  float[][] base=blend(WILDS[0],WILDS[1],night),out=copy(base);
  float[][][][] zones={FOREST,DESERT,SWAMP,SNOW};
  for(int z=0;z<4;z++){
   float[][] look=blend(zones[z][0],zones[z][1],night);
   for(int r=0;r<3;r++)for(int c=0;c<4;c++)out[r][c]+=(look[r][c]-base[r][c])*weights[z]/total;
  }
  return out;
 }
 /** enhanced_grade.rs update: exact exponential easing over 1.5 s, whatever the frame rate. */
 static float[][] update(float[][] previous,float[][] target,float dt){
  if(previous[2][3]==0)return copy(target);
  dt=Float.isFinite(dt)?Math.max(dt,0):0;
  return blend(previous,target,1-(float)Math.exp(-dt/1.5));
 }
 /** minecraft_world.rs: the grade the biomes round the camera ask for this frame, eased (identity off Jak's weather). */
 public static void frame(boolean on,float rainy,float dry,float swamp,float cold,float hour){
  long now=System.nanoTime();float dt=last==0?0:Math.min((now-last)/1e9f,1);last=now;
  grade=update(grade,on?target(rainy,dry,swamp,cold,hour):IDENTITY,dt);
 }
 /** For the scenes' reports: the grade drawn now. */
 public static float[][] current(){return copy(grade);}
 /** drawsurf/enhanced_grade.rs draw: the finished world copied aside and drawn back graded. */
 public static void apply(RenderTarget main){
  if(bypass||STRENGTH==0||grade[2][3]==0||main.getColorTexture()==null)return;
  if(picture==null)picture=new TextureTarget("Jak grade",main.width,main.height,GpuFormat.RGBA8_UNORM,null);
  else if(picture.width!=main.width||picture.height!=main.height)picture.resize(main.width,main.height);
  picture.copyColorFrom(main);
  float[][] g=grade;
  // The grade rides in DynamicTransforms: TextureMat's columns are lift, gain and params (params.y the strength).
  var data=new Matrix4f(g[0][0],g[0][1],g[0][2],g[0][3],g[1][0],g[1][1],g[1][2],g[1][3],g[2][0],STRENGTH,g[2][2],g[2][3],0,0,0,0);
  var transforms=RenderSystem.getDynamicUniforms().writeTransform(new Matrix4f(),new Vector4f(1),new Vector3f(),data);
  try(var pass=RenderSystem.getDevice().createCommandEncoder().createRenderPass(()->"Jak grade",main.getColorTextureView(),Optional.empty(),null,OptionalDouble.empty())){
   pass.setPipeline(RenderSystem.getCompiledPipeline(PIPELINE));
   RenderSystem.bindDefaultUniforms(pass);
   pass.setUniform("DynamicTransforms",transforms);
   pass.setUniform("InSampler",picture.getColorTextureView(),RenderSystem.getSamplerCache().getClampToEdge(FilterMode.NEAREST));
   pass.draw(3,1,0,0);
  }
 }
}
