package dev.jakmc.client.model;

import com.mojang.blaze3d.platform.NativeImage;
import dev.jakmc.JakData;
import dev.jakmc.RuntimeConfig;
import java.awt.image.BufferedImage;
import java.io.IOException;
import java.io.UncheckedIOException;
import javax.imageio.ImageIO;
import net.minecraft.client.Minecraft;
import net.minecraft.client.renderer.texture.DynamicTexture;
import net.minecraft.resources.Identifier;

final class LightFormTexture {
    private static final int WIDTH=64, FRAMES=60;

    enum Form {
        WINGS(16,1,"level-default-water","lightjak-wings-u-src","lightjak-wings-v-src"),
        SHIELD(64,4,"level-default-warp","shield-env-uscroll","shield-env-uvscroll","shield-env-rim-src");

        final int height, seconds;
        final String page;
        final String[] names;
        private Identifier atlas;

        Form(int height,int seconds,String page,String... names){
            this.height=height;
            this.seconds=seconds;
            this.page=page;
            this.names=names;
        }

        int frame(float elapsed){return Math.floorMod((int)Math.floor(elapsed*FRAMES/seconds),FRAMES);}
        float atlasV(int frame,float v){
            float inset=.5f/height;
            return (frame+Math.clamp(v,inset,1-inset))/FRAMES;
        }

        Identifier texture(){
            if(atlas!=null)return atlas;
            var folder=RuntimeConfig.assets.resolve("textures").resolve(page);
            var sources=new BufferedImage[names.length];
            try {
                for(int i=0;i<names.length;i++){
                    sources[i]=ImageIO.read(folder.resolve(names[i]+".png").toFile());
                    if(sources[i]==null)throw new IOException("Missing Light Jak layer "+names[i]);
                }
            }catch(IOException e){throw new UncheckedIOException(e);}
            var image=new NativeImage(WIDTH,height*FRAMES,false);
            float[] rgb=new float[3], rgba=new float[4];
            for(int frame=0;frame<FRAMES;frame++){
                float t=frame/(float)FRAMES;
                float sin=(float)Math.sin(t*Math.TAU), cos=(float)Math.cos(t*Math.TAU);
                for(int y=0;y<height;y++)for(int x=0;x<WIDTH;x++){
                    float u=(x+.5f)/WIDTH, v=(y+.5f)/height;
                    java.util.Arrays.fill(rgb,0);
                    if(this==WINGS){
                        add(sources[0],u-.5f-t,v,true,false,rgb,rgba);
                        add(sources[1],u,v-.5f-t,false,true,rgb,rgba);
                    }else{
                        float px=u-.5f, py=v-.5f;
                        add(sources[0],px+4*t,v,true,true,rgb,rgba);
                        add(sources[1],px+8*t,py+4*t,true,true,rgb,rgba);
                        for(int mirror=1;mirror>=-1;mirror-=2){
                            float qx=px*mirror;
                            float s=qx*cos-py*sin+.5f, r=qx*sin+py*cos+.5f;
                            if(s>=0&&s<=1&&r>=0&&r<=1)add(sources[2],s,r,false,false,rgb,rgba);
                        }
                    }
                    int color=0x80000000;
                    for(int c=0;c<3;c++)color|=Math.clamp(Math.round(rgb[c]),0,255)<<(16-c*8);
                    image.setPixel(x,frame*height+y,color);
                }
            }
            var id=JakData.id("local/"+name().toLowerCase(java.util.Locale.ROOT)+"_animated");
            Minecraft.getInstance().getTextureManager().register(id,new DynamicTexture(id::toString,image));
            atlas=id;
            return atlas;
        }
    }

    // texture-anim-tables.gc layers accumulate using PS2 alpha, where 128 is one.
    private static void add(BufferedImage source,float u,float v,boolean wrapU,boolean wrapV,float[] rgb,float[] rgba){
        int width=source.getWidth(), height=source.getHeight();
        float x=u*width-.5f, y=v*height-.5f;
        int ix=(int)Math.floor(x), iy=(int)Math.floor(y);
        float fx=x-ix, fy=y-iy;
        java.util.Arrays.fill(rgba,0);
        for(int dy=0;dy<2;dy++)for(int dx=0;dx<2;dx++){
            int px=wrapU?Math.floorMod(ix+dx,width):Math.clamp(ix+dx,0,width-1);
            int py=wrapV?Math.floorMod(iy+dy,height):Math.clamp(iy+dy,0,height-1);
            int pixel=source.getRGB(px,py);
            float weight=(dx==0?1-fx:fx)*(dy==0?1-fy:fy);
            for(int c=0;c<3;c++)rgba[c]+=((pixel>>>(16-c*8))&255)*weight;
            rgba[3]+=(pixel>>>24)*weight;
        }
        for(int c=0;c<3;c++)rgb[c]+=rgba[c]*rgba[3]/128;
    }
}
