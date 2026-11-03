package dev.jakmc.client.model;
import com.mojang.blaze3d.vertex.PoseStack;
import com.mojang.blaze3d.platform.NativeImage;
import com.mojang.serialization.*;
import com.mojang.serialization.codecs.RecordCodecBuilder;
import dev.jakmc.*;
import dev.jakmc.client.JakEffectTypes;
import net.minecraft.client.Minecraft;
import net.minecraft.client.renderer.SubmitNodeCollector;
import net.minecraft.client.renderer.special.*;
import net.minecraft.client.renderer.texture.*;
import net.minecraft.resources.Identifier;
import net.minecraft.world.item.ItemStack;
import org.joml.Matrix4f;
import org.joml.Vector3f;
import org.joml.Vector3fc;
import java.io.*;
import java.util.function.Consumer;
/** Body and original 461/462/463 halos share the complete interpolated item/hand matrix. */
public final class SkullGemRenderer implements SpecialModelRenderer<Void> {
 private static GltfModel model;private static GltfModel.Posed rest;private static Vector3f center;private static float height;private static Identifier glow;private final boolean gui;
 public static int submitted,halos,carrierGlows;public static float lastHeight;
 SkullGemRenderer(boolean gui){this.gui=gui;}
 public static void initialize(){SpecialModelRenderers.ID_MAPPER.put(JakData.id("skull_gem"),Unbaked.CODEC);}
 private static void load(){if(model!=null)return;try{
  model=new GltfModel(RuntimeConfig.assets.resolve("levels/common/collectables-gem-lod0.glb"),"skull_gem");rest=model.pose("",0);var b=model.bounds(rest,new Matrix4f());center=new Vector3f(b[0]).add(b[1]).mul(.5f);height=b[1].y-b[0].y;
 }catch(IOException e){throw new UncheckedIOException(e);}}
 private static void glowTexture(){if(glow!=null)return;try{glow=JakData.id("local/gem_glow");try(var in=java.nio.file.Files.newInputStream(RuntimeConfig.assets.resolve("textures/level-default-sprite/glow-soft.png"))){Minecraft.getInstance().getTextureManager().register(glow,new DynamicTexture(()->"Original gem glow-soft",NativeImage.read(in)));}}catch(IOException e){throw new UncheckedIOException(e);}}
 public Void extractArgument(ItemStack item){return null;}
 public void getExtents(Consumer<Vector3fc> out){load();var b=model.bounds(rest,new Matrix4f().translation(.5f,.4f,.5f).scale(.30f/height).translate(-center.x,-center.y,-center.z));for(float x:new float[]{b[0].x,b[1].x})for(float y:new float[]{b[0].y,b[1].y})for(float z:new float[]{b[0].z,b[1].z})out.accept(new Vector3f(x,y,z));}
 public void submit(Void v,PoseStack stack,SubmitNodeCollector collector,int light,int overlay,boolean foil,int outline){stack.pushPose();stack.translate(.5,.4,.5);draw(stack,collector,light,.30f,gui);stack.popPose();}
 public static void held(PoseStack stack,SubmitNodeCollector collector,int light){stack.pushPose();stack.mulPose(new Matrix4f().rotation(stack.last().pose().getUnnormalizedRotation(new org.joml.Quaternionf()).conjugate()));draw(stack,collector,light,.30f,false);stack.popPose();}
 public static void carrier(PoseStack stack,SubmitNodeCollector collector){load();glow(stack,collector,1,false,true);carrierGlows++;}
 public static void world(PoseStack stack,SubmitNodeCollector collector,int light,float angle){stack.pushPose();stack.translate(0,.125,0);stack.rotateDegrees(com.mojang.math.Axis.YP,angle);draw(stack,collector,light,.25f,false);stack.popPose();}
 private static void draw(PoseStack stack,SubmitNodeCollector collector,int light,float wantedHeight,boolean gui){
  load();float scale=wantedHeight/height;stack.pushPose();stack.scale(scale,scale,scale);stack.translate(-center.x,-center.y,-center.z);model.submit(rest,stack,collector,light);stack.popPose();submitted++;lastHeight=wantedHeight;
  glow(stack,collector,scale,gui,false);
 }
 private static void glow(PoseStack stack,SubmitNodeCollector collector,float scale,boolean gui,boolean carrier){
  glowTexture();
  var inverse=new Matrix4f(stack.last().pose()).invert();var camera=Minecraft.getInstance().gameRenderer.mainCamera();
  Vector3f forward=gui?new Vector3f(0,0,-1):inverse.transformDirection(new Vector3f(camera.forwardVector())).normalize();Vector3f up=gui?new Vector3f(0,1,0):inverse.transformDirection(new Vector3f(camera.upVector())).normalize();Vector3f right=new Vector3f(forward).cross(up).normalize();
  for(int layer=0;layer<(carrier?1:3);layer++){float depth=layer==0?0:layer==1?.3f:-.3f,size=(carrier?1.5f:layer==0?2.2f:1.75f)*scale,alpha=(layer==0?16:12)/255f;Vector3f r=new Vector3f(right).mul(size*.5f),u=new Vector3f(up).mul(size*.5f),at=new Vector3f(0,0,depth*scale);int color=(Math.round(alpha*255)<<24)|0xffc040;
   collector.submitCustomGeometry(stack,JakEffectTypes.get(glow,3),(pose,consumer)->{Vector3f p=new Vector3f();for(int k=0;k<4;k++){float x=k==0||k==3?-1:1,y=k<2?-1:1;pose.pose().transformPosition(new Vector3f(at).fma(x,r).fma(y,u),p);consumer.addVertex(p.x,p.y,p.z,color,x<0?0:1,y<0?1:0,OverlayTexture.NO_OVERLAY,0xf000f0,0,1,0);}});halos++;
  }
 }
 public record Unbaked(boolean gui) implements SpecialModelRenderer.Unbaked<Void>{public static final MapCodec<Unbaked> CODEC=RecordCodecBuilder.mapCodec(i->i.group(Codec.BOOL.optionalFieldOf("gui",false).forGetter(Unbaked::gui)).apply(i,Unbaked::new));public MapCodec<Unbaked> type(){return CODEC;}public SpecialModelRenderer<Void> bake(SpecialModelRenderer.BakingContext c){return new SkullGemRenderer(gui);}}
}
