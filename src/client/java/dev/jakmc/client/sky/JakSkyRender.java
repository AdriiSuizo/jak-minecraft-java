package dev.jakmc.client.sky;

import com.mojang.blaze3d.platform.NativeImage;
import com.mojang.blaze3d.systems.RenderSystem;
import com.mojang.blaze3d.vertex.BufferBuilder;
import com.mojang.blaze3d.vertex.ByteBufferBuilder;
import com.mojang.blaze3d.vertex.DefaultVertexFormat;
import com.mojang.blaze3d.vertex.MeshData;
import com.mojang.renderpearl.api.buffers.GpuBuffer;
import com.mojang.renderpearl.api.commands.RenderPass;
import com.mojang.renderpearl.api.pipeline.BlendFunction;
import com.mojang.renderpearl.api.pipeline.ColorTargetState;
import com.mojang.renderpearl.api.pipeline.PrimitiveTopology;
import com.mojang.renderpearl.api.pipeline.RenderPipeline;
import com.mojang.renderpearl.api.textures.FilterMode;
import dev.jakmc.JakData;
import dev.jakmc.JakMod;
import dev.jakmc.RuntimeConfig;
import dev.jakmc.client.JakSky;
import java.io.IOException;
import java.nio.file.Files;
import java.nio.file.Path;
import java.util.ArrayList;
import java.util.HashMap;
import java.util.List;
import java.util.Map;
import net.minecraft.client.Minecraft;
import net.minecraft.client.renderer.BindGroupLayouts;
import net.minecraft.client.renderer.RenderPipelines;
import net.minecraft.client.renderer.texture.DynamicTexture;
import org.joml.Matrix4f;
import org.joml.Vector3f;
import org.joml.Vector4f;

/** Jak 1's sky dome and its bodies in Minecraft's sky pass (render_gpu drawsurf/minecraft_world.rs jak_sky, jak_weather.rs sprites_in). */
public final class JakSkyRender {
 private static final RenderPipeline DOME=RenderPipelines.register(RenderPipeline.builder(RenderPipelines.GLOBALS_SNIPPET)
  .withBindGroupLayout(BindGroupLayouts.PROJECTION).withBindGroupLayout(BindGroupLayouts.DYNAMIC_TRANSFORMS).withBindGroupLayout(BindGroupLayouts.SAMPLER0)
  .withLocation(JakData.id("pipeline/jak_sky")).withVertexShader(JakData.id("core/jak_sky")).withFragmentShader(JakData.id("core/jak_sky"))
  .withColorTargetState(ColorTargetState.DEFAULT).withCull(false).withVertexBinding(0,DefaultVertexFormat.POSITION)
  .withPrimitiveTopology(PrimitiveTopology.QUADS).build());
 // The bodies are Jak's sprites, added on (Blend::Add, SRC_ALPHA ONE).
 private static final RenderPipeline BODIES=RenderPipelines.register(RenderPipeline.builder(RenderPipelines.GLOBALS_SNIPPET)
  .withBindGroupLayout(BindGroupLayouts.PROJECTION).withBindGroupLayout(BindGroupLayouts.DYNAMIC_TRANSFORMS).withBindGroupLayout(BindGroupLayouts.SAMPLER0)
  .withLocation(JakData.id("pipeline/jak_sky_bodies")).withVertexShader("core/position_tex_color").withFragmentShader(JakData.id("core/jak_sky_sprite"))
  .withColorTargetState(new ColorTargetState(BlendFunction.LIGHTNING)).withCull(false).withVertexBinding(0,DefaultVertexFormat.POSITION_TEX_COLOR)
  .withPrimitiveTopology(PrimitiveTopology.QUADS).build());
 // stand-in: jak_weather.rs draws the bodies 2400 blocks out in the world; Minecraft's sky pass draws them 100 out, the same angles.
 private static final float R=100;
 private static DynamicTexture atlas;
 private static boolean tried;
 private static final Map<String,DynamicTexture> SPRITES=new HashMap<>();
 private static GpuBuffer cube,bodies;
 private record Range(DynamicTexture texture,int first,int count){}
 private static final List<Range> RANGES=new ArrayList<>();
 public static void initialize(){}
 /** The original textures (assets::jak_model::jak1_texture's search: the override, the mashup's export, the OpenGOAL install). */
 public static Path jak1(String page,String name){
  List<Path> roots=new ArrayList<>();
  String override=System.getenv("IW4L_JAK1_TEXTURES"),local=System.getenv("LOCALAPPDATA");
  if(override!=null)roots.add(Path.of(override));
  if(local!=null)roots.add(Path.of(local,"iw4l-mashup/jak1-proj/decompiler_out/jak1/textures"));
  roots.add(RuntimeConfig.gameAssets("jak1").resolve("textures"));
  for(var root:roots){var file=root.resolve(page).resolve(name+".png");if(Files.isRegularFile(file))return file;}
  return null;
 }
 public static Path jak3(String name){var file=RuntimeConfig.assets.resolve("textures/level-default-sprite").resolve(name+".png");return Files.isRegularFile(file)?file:null;}
 static NativeImage read(Path file){
  if(file==null)return null;
  try(var in=Files.newInputStream(file)){return NativeImage.read(in);}catch(IOException e){JakMod.LOG.error("Cannot read original sky texture {}",file,e);return null;}
 }
 // minecraft_world.rs celestial_image: village1's skies (vil1-sky-00 .. 07, from x 288) and clouds (at x 544), alpha from the PS2's scale.
 public static boolean ready(){
  if(!tried){
   tried=true;
   var image=new NativeImage(1024,128,true);int found=0;
   for(int i=0;i<9;i++){
    var src=read(jak1("village1-vis-alpha",i<8?"vil1-sky-0"+i:"vil1-clouds"));
    if(src==null)continue;
    int ox=i<8?288+32*i:544;
    for(int y=0;y<Math.min(src.getHeight(),128);y++)for(int x=0;x<src.getWidth()&&ox+x<1024;x++){
     int c=src.getPixel(x,y),a=Math.min(255,(c>>>24)*2);image.setPixel(ox+x,y,(a<<24)|(c&0xffffff));
    }
    src.close();found++;
   }
   // Its count took the dark eco's two as well (the atlas held them): nine of the eleven.
   if(jak1("finalboss-vis-pris","cv-darkecowater")!=null)found++;
   if(jak1("finalboss-vis-pris","environment-darkeco")!=null)found++;
   if(found>=9)atlas=new DynamicTexture(()->"Jak 1 sky",image);else{image.close();JakMod.LOG.warn("Jak 1 sky textures missing: Minecraft's sky stays");}
  }
  return atlas!=null;
 }
 private static DynamicTexture sprite(String name){
  if(SPRITES.containsKey(name))return SPRITES.get(name);
  var image=read(name.startsWith("jak1:")?jak1("effects",name.substring(5)):jak3(name));
  if(image==null)JakMod.LOG.error("Original sky sprite texture unavailable: {}",name);
  var texture=image==null?null:new DynamicTexture(()->"Jak sky "+name,image);
  SPRITES.put(name,texture);return texture;
 }
 private static final Vector3f DIR=new Vector3f(),AT=new Vector3f(),RIGHT=new Vector3f(),UP=new Vector3f(),FWD=new Vector3f(),A=new Vector3f(),B=new Vector3f();
 private static final float[] SUN=new float[3];
 private static float pixels;
 // jak_weather.rs sky_dir: Jak's sky axes in Minecraft's (block = (j.z, j.y, -j.x)).
 private static Vector3f skyDir(Vector3f j,Vector3f out){return out.set(j.z,j.y,-j.x);}
 private static int pack(float r,float g,float b,float a){return (ch(a)<<24)|(ch(r)<<16)|(ch(g)<<8)|ch(b);}
 private static int ch(float v){return Math.clamp(Math.round(v),0,255);}
 // render/extract.rs push_jak_sparks for a camera-facing sprite: corners, PS2 colour, inv-area fade.
 private static void quad(BufferBuilder out,Vector3f p,float w,float h,float rotation,float r,float g,float b,float a){
  float s=(float)Math.sin(rotation),c=(float)Math.cos(rotation);
  A.set(RIGHT).mul(c).fma(s,UP).mul(w*.5f);B.set(UP).mul(c).fma(-s,RIGHT).mul(h*.5f);
  float depth=Math.max(p.dot(FWD),1e-3f),area=Math.min(w*h*(pixels/depth)*(pixels/depth),1);
  int colour=pack(r,g,b,a*area);
  out.addVertex(p.x-A.x-B.x,p.y-A.y-B.y,p.z-A.z-B.z).setUv(0,1).setColor(colour);
  out.addVertex(p.x+A.x-B.x,p.y+A.y-B.y,p.z+A.z-B.z).setUv(1,1).setColor(colour);
  out.addVertex(p.x+A.x+B.x,p.y+A.y+B.y,p.z+A.z+B.z).setUv(1,0).setColor(colour);
  out.addVertex(p.x-A.x+B.x,p.y-A.y+B.y,p.z-A.z+B.z).setUv(0,0).setColor(colour);
 }
 private static int group(BufferBuilder out,String texture,int first,Runnable body){
  var tex=sprite(texture);if(tex==null)return first;
  int before=counted;body.run();int count=counted-before;
  if(count>0)RANGES.add(new Range(tex,first,count));
  return first+count;
 }
 private static int counted;
 private static BufferBuilder builder;
 private static void add(Vector3f p,float w,float h,float rotation,float r,float g,float b,float a){
  if(p.dot(FWD)<-Math.max(w,h))return;
  quad(builder,p,w,h,rotation,r,g,b,a);counted++;
 }
 /** jak_weather.rs sprites_in: the sun (group-sun), the green sun (group-green-sun) and the stars (group-stars), uploaded before the sky pass. */
 public static void prepare(){
  RANGES.clear();
  if(!JakSky.celestial())return;
  var view=RenderSystem.getModelViewMatrixCopy().invert();
  view.transformDirection(RIGHT.set(1,0,0));view.transformDirection(UP.set(0,1,0));view.transformDirection(FWD.set(0,0,-1));
  RIGHT.normalize();UP.normalize();FWD.normalize();
  // The PS2's screen pixels per unit at depth 1: its 416 lines over the view's height.
  pixels=208f/(float)Math.max(Math.tan(Math.toRadians(Minecraft.getInstance().options.fov().get())*.5),1e-3);
  float clear=1-JakSky.SHADER[4][3],hour=JakSky.hour,fade=clear*128;
  float spin=(float)Math.toRadians((float)JakSky.frames*.05f);
  int quads=3+2+JakSky.STARS.size();
  try(var bytes=ByteBufferBuilder.exactlySized(quads*4*DefaultVertexFormat.POSITION_TEX_COLOR.getVertexSize())){
   builder=new BufferBuilder(bytes,PrimitiveTopology.QUADS,DefaultVertexFormat.POSITION_TEX_COLOR);counted=0;
   int first=0;
   // group-sun (sparticle-track-sun): its core in the sun's colour, its two flares at a quarter of it turning 0.05 degrees a frame.
   boolean sun=JakSky.sunUp(hour),green=JakSky.greenSunUp(hour);
   JakSky.sunColor(hour,SUN);
   if(sun){
    skyDir(JakSky.orbitDir(JakSky.SUN,hour,DIR),AT).mul(R);
    float core=1200f/9950*R;
    first=group(builder,"middot",first,()->add(AT,core,core,0,SUN[0],SUN[1],SUN[2],fade));
   }
   first=group(builder,"jak1:starflash2",first,()->{
    if(sun){
     skyDir(JakSky.orbitDir(JakSky.SUN,hour,DIR),AT).mul(R);
     float w=2800f/9950*R,h=2200f/9950*R;
     add(AT,w,h,-spin,SUN[0]/4,SUN[1]/4,SUN[2]/4,fade);add(AT,h,w,spin,SUN[0]/4,SUN[1]/4,SUN[2]/4,fade);
    }
    // group-green-sun: its flares green.
    if(green){
     skyDir(JakSky.orbitDir(JakSky.GREEN_SUN,hour,DIR),AT).mul(R);
     float w=1400f/9950*R,h=1100f/9950*R;
     add(AT,w,h,-spin,0,64,0,fade);add(AT,h,w,spin,0,64,0,fade);
    }
   });
   // group-stars: each fading in for its 0.2 to 1 s at 0.4267 a frame and kept, fading out as fast.
   group(builder,"hotdot",first,()->{
    for(float[] star:JakSky.STARS){
     float age=JakSky.nightFrames-star[6];
     if(age<=0)continue;
     float alpha=Math.max(.4266667f*Math.min(age,star[5]*60)-.4266667f*JakSky.dawnFrames,0)*clear;
     if(alpha<=0||star[1]<0)continue;
     skyDir(DIR.set(star[0],star[1],star[2]),AT).mul(R);
     float size=star[3]/5000*R;
     add(AT,size,size,0,star[4],star[4],star[4],alpha);
    }
   });
   if(counted==0){RANGES.clear();return;}
   try(MeshData mesh=builder.buildOrThrow()){
    var data=mesh.vertexBuffer();var device=RenderSystem.getDevice();
    if(bodies==null||bodies.size()<data.remaining()){if(bodies!=null)bodies.close();bodies=device.createBuffer(()->"Jak sky bodies",40,data.remaining());}
    device.createCommandEncoder().writeToBuffer(bodies.slice(),data);
   }
  }finally{builder=null;}
 }
 private static GpuBuffer cube(){
  if(cube!=null)return cube;
  try(var bytes=ByteBufferBuilder.exactlySized(24*DefaultVertexFormat.POSITION.getVertexSize())){
   var b=new BufferBuilder(bytes,PrimitiveTopology.QUADS,DefaultVertexFormat.POSITION);
   float s=R;
   float[][] faces={{-s,-s,-s, s,-s,-s, s,s,-s, -s,s,-s},{-s,-s,s, -s,s,s, s,s,s, s,-s,s},{-s,-s,-s, -s,s,-s, -s,s,s, -s,-s,s},
    {s,-s,-s, s,-s,s, s,s,s, s,s,-s},{-s,-s,-s, -s,-s,s, s,-s,s, s,-s,-s},{-s,s,-s, s,s,-s, s,s,s, -s,s,s}};
   for(float[] f:faces)for(int i=0;i<12;i+=3)b.addVertex(f[i],f[i+1],f[i+2]);
   try(MeshData mesh=b.buildOrThrow()){cube=RenderSystem.getDevice().createBuffer(()->"Jak sky dome",32,mesh.vertexBuffer());}
  }
  return cube;
 }
 /** In place of Minecraft's sky disc: render_gpu minecraft_world.rs sky_fragment's Jak branch, then the bodies. */
 public static boolean dome(RenderPass pass){
  if(!JakSky.celestial()||atlas==null)return false;
  float[][] d=JakSky.SHADER;
  // The shader's data rides in the dynamic transforms: TextureMat's columns [1], [3], [4], [5]; ColorModulator the fog and the flash.
  var data=new Matrix4f(d[1][0],d[1][1],d[1][2],d[1][3],d[3][0],d[3][1],d[3][2],d[3][3],d[4][0],d[4][1],d[4][2],d[4][3],d[5][0],d[5][1],d[5][2],d[5][3]);
  var transforms=RenderSystem.getDynamicUniforms().writeTransform(RenderSystem.getModelViewMatrixCopy(),new Vector4f(d[6][0],d[6][1],d[6][2],d[2][1]),new Vector3f(),data);
  int total=0;for(var r:RANGES)total=Math.max(total,r.first()+r.count());
  var indices=RenderSystem.getSequentialBuffer(PrimitiveTopology.QUADS);
  var indexBuffer=indices.getBuffer(Math.max(36,total*6));
  pass.pushDebugGroup(()->"Jak sky");
  pass.setPipeline(RenderSystem.getCompiledPipeline(DOME));
  RenderSystem.bindDefaultUniforms(pass);
  pass.setUniform("DynamicTransforms",transforms);
  pass.setUniform("Sampler0",atlas.getTextureView(),RenderSystem.getSamplerCache().getClampToEdge(FilterMode.NEAREST));
  pass.setVertexBuffer(0,cube().slice());
  pass.setIndexBuffer(indexBuffer,indices.type());
  pass.drawIndexed(36,1,0,0,0);
  pass.popDebugGroup();
  if(total>0&&bodies!=null){
   var bodyTransforms=RenderSystem.getDynamicUniforms().writeTransform(RenderSystem.getModelViewMatrixCopy());
   pass.pushDebugGroup(()->"Jak sky bodies");
   pass.setPipeline(RenderSystem.getCompiledPipeline(BODIES));
   RenderSystem.bindDefaultUniforms(pass);
   pass.setUniform("DynamicTransforms",bodyTransforms);
   pass.setVertexBuffer(0,bodies.slice());
   pass.setIndexBuffer(indexBuffer,indices.type());
   for(var r:RANGES){
    pass.setUniform("Sampler0",r.texture().getTextureView(),RenderSystem.getSamplerCache().getClampToEdge(FilterMode.NEAREST));
    pass.drawIndexed(r.count()*6,1,r.first()*6,0,0);
   }
   pass.popDebugGroup();
  }
  return true;
 }
}
