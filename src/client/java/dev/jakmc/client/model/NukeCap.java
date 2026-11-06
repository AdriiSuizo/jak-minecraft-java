package dev.jakmc.client.model;

import com.mojang.blaze3d.platform.NativeImage;
import com.mojang.blaze3d.vertex.PoseStack;
import dev.jakmc.JakData;
import dev.jakmc.RuntimeConfig;
import java.nio.file.Files;
import net.minecraft.client.Minecraft;
import net.minecraft.client.renderer.SubmitNodeCollector;
import net.minecraft.client.renderer.texture.DynamicTexture;
import net.minecraft.client.renderer.texture.OverlayTexture;
import net.minecraft.resources.Identifier;
import net.minecraft.world.phys.Vec3;
import org.joml.Vector3f;

/**
 * The Super Nova's mushroom cap, Jak 3 gun-dark-shot.gc (the impact state's prim-strip): a quad on the
 * mushroom's top facing the camera about the vertical, half as wide as the x curve and half as high as
 * the y curve, grey with green, blue and alpha 127·fade, alpha blended, alpha test ≥ 0x32, and its
 * texture `mushroom-dest` (texture-anim-tables.gc): the RGB of `mushroom-src` scrolled down its height
 * once every 1200 frames at 300 a second, over the alpha of the unscrolled one.
 */
final class NukeCap {
 private static final Identifier ID=JakData.id("nuke/mushroom-dest");
 private static NativeImage source;private static DynamicTexture texture;private static boolean missing;
 private NukeCap(){}

 /** `top` and `half` (x, y) in blocks, `fade` the strip's linear fade (0..1). */
 static void submit(PoseStack stack,SubmitNodeCollector collector,Vec3 camera,Vec3 top,float halfX,float halfY,float fade){
  if(!ready()||fade<=0)return;
  animate(fade);
  // The strip's sides: up × (top − camera) flattened, so it turns about the vertical only.
  var toTop=new Vector3f((float)(top.x-camera.x),0,(float)(top.z-camera.z));
  if(toTop.lengthSquared()<1e-6f)toTop.set(0,0,1);
  var side=new Vector3f(0,1,0).cross(toTop).normalize(halfX);
  var up=new Vector3f(0,halfY,0);
  var centre=new Vector3f((float)(top.x-camera.x),(float)(top.y-camera.y),(float)(top.z-camera.z));
  int a=Math.round(127*fade);
  // jak_effect's PS2 scale (0x80 = 1): r gray 0x80, g, b and a 127·fade.
  int colour=a<<24|0x80<<16|a<<8|a;
  collector.submitCustomGeometry(stack,dev.jakmc.client.JakEffectTypes.get(ID,0),(pose,buffer)->{
   float[][] corners={{-1,1,0,0},{-1,-1,0,1},{1,-1,1,1},{1,1,1,0}};
   for(var c:corners){
    var at=pose.pose().transformPosition(new Vector3f(centre).add(new Vector3f(side).mul(c[0])).add(new Vector3f(up).mul(c[1])));
    buffer.addVertex(at.x,at.y,at.z,colour,c[2],c[3],OverlayTexture.NO_OVERLAY,0xf000f0,0,0,1);
   }
  });
 }
 private static boolean ready(){
  if(texture!=null)return true;if(missing)return false;
  var file=RuntimeConfig.assets.resolve("textures/level-default-water/mushroom-src.png");
  try(var in=Files.newInputStream(file)){
   source=NativeImage.read(in);
   texture=new DynamicTexture(ID::toString,new NativeImage(source.getWidth(),source.getHeight(),false));
   Minecraft.getInstance().getTextureManager().register(ID,texture);
   return true;
  }catch(Exception e){missing=true;dev.jakmc.JakMod.LOG.warn("Super Nova mushroom texture missing: {}",file);return false;}
 }
 // The texture anim's frame (frame-delta 300, frame-mod 1200) and the strip's alpha test folded in
 // (jak_effect has none): a texel whose alpha times the strip's is under 0x32 is cleared.
 private static void animate(float fade){
  var out=texture.getPixels();if(out==null)return;
  int w=source.getWidth(),h=source.getHeight();
  double frame=(System.nanoTime()/1e9*300)%1200;
  int shift=(int)Math.floor(frame/1200*h);
  float vertexAlpha=127*fade;
  for(int y=0;y<h;y++)for(int x=0;x<w;x++){
   int rgb=source.getPixel(x,(y+shift)%h)&0x00ffffff;
   int alpha=source.getPixel(x,y)>>>24;
   if(alpha*vertexAlpha/128<0x32)alpha=0;
   out.setPixel(x,y,alpha<<24|rgb);
  }
  texture.upload();
 }
}
