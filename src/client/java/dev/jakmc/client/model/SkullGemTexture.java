package dev.jakmc.client.model;
import java.awt.image.BufferedImage;
import java.io.*;
import java.nio.file.Path;
import javax.imageio.ImageIO;
/** Jak 3 texture-anim-tables.gc: the three additive skull-gem layers, one-second loop. */
public final class SkullGemTexture {
 public static final int SIZE=64,FRAMES=60;
 public static int[] bake(Path textures)throws IOException {
  BufferedImage[] layers=new BufferedImage[3];for(int i=0;i<3;i++){layers[i]=ImageIO.read(textures.resolve("skull-gem-alpha-0"+i+".png").toFile());if(layers[i]==null)throw new IOException("Missing skull gem layer "+i);}
  int[] pixels=new int[SIZE*SIZE*FRAMES];
  for(int frame=0;frame<FRAMES;frame++){float t=frame/(float)FRAMES;
   for(int y=0;y<SIZE;y++)for(int x=0;x<SIZE;x++){
    float u=(x+.5f)/SIZE,v=(y+.5f)/SIZE;
    float[][] c={sample(layers[0],u-.5f,v-t),sample(layers[1],u-2*(1-t),v-(1-t)),sample(layers[2],u-2*t,v-2*t)};
    int color=0xff000000;for(int k=0;k<3;k++){float tint=k==1?.6f-.1f*t:.6f;float value=c[0][k]*c[0][3]/128+c[1][k]*c[1][3]/128*tint+c[2][k]*c[2][3]/128*.6f;color|=Math.clamp(Math.round(value),0,255)<<(16-k*8);}pixels[(frame*SIZE+y)*SIZE+x]=color;
   }
  }return pixels;
 }
 private static float[] sample(BufferedImage image,float u,float v){
  int w=image.getWidth(),h=image.getHeight();float x=u*w-.5f,y=v*h-.5f;int ix=(int)Math.floor(x),iy=(int)Math.floor(y);float fx=x-ix,fy=y-iy;float[] color=new float[4];
  for(int dy=0;dy<2;dy++)for(int dx=0;dx<2;dx++){int p=image.getRGB(Math.floorMod(ix+dx,w),Math.floorMod(iy+dy,h));float weight=(dx==0?1-fx:fx)*(dy==0?1-fy:fy);for(int k=0;k<4;k++)color[k]+=((p>>>(k==3?24:16-k*8))&255)*weight;}return color;
 }
}
