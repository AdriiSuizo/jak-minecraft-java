package dev.jakmc.client;

import com.mojang.blaze3d.platform.NativeImage;
import dev.jakmc.JakData;
import dev.jakmc.JakMod;
import dev.jakmc.RuntimeConfig;
import java.nio.file.Files;
import net.minecraft.client.Minecraft;
import net.minecraft.client.gui.GuiGraphicsExtractor;
import net.minecraft.client.renderer.RenderPipelines;
import net.minecraft.client.renderer.texture.DynamicTexture;

/** Jak III's original inner eco ring; Minecraft still owns health and hunger. */
public final class JakHud {
    private static final net.minecraft.resources.Identifier ID=JakData.id("hud/eco_ring");
    private static NativeImage frame,shine;
    private static DynamicTexture texture;
    private static boolean attempted;
    private static int lastLight=-1,lastDark=-1,lastPulse=-1;
    public static void draw(GuiGraphicsExtractor graphics){
        var client=Minecraft.getInstance();var player=client.player;
        if(player==null||client.gui.screen()!=null||!player.getAttachedOrCreate(JakData.ENABLED))return;
        if(!attempted){
            attempted=true;
            var root=RuntimeConfig.assets.resolve("textures/level-default-minimap");
            try(var f=Files.newInputStream(root.resolve("hud-newhud-01.png"));var s=Files.newInputStream(root.resolve("hud-newhud-shine-01.png"))){
                frame=NativeImage.read(f);shine=NativeImage.read(s);
                texture=new DynamicTexture("Original Jak eco ring",62,62,true);client.getTextureManager().register(ID,texture);
            }catch(java.io.IOException e){JakMod.LOG.error("Original eco HUD unavailable",e);}
        }
        int light=Math.clamp(Math.round(player.getAttachedOrCreate(JakData.LIGHT_ECO)),0,100);
        int dark=Math.clamp(Math.round(player.getAttachedOrCreate(JakData.DARK_ECO)),0,100);
        int pulse=(int)((System.nanoTime()/100_000_000L)%60);
        if(texture!=null){
            if(light!=lastLight||dark!=lastDark||pulse!=lastPulse){
                lastLight=light;lastDark=dark;lastPulse=pulse;
                var pixels=texture.getPixels();
                double wave=.65+.35*Math.sin(pulse*Math.PI/30*(dark>=50?2:1));
                for(int y=0;y<62;y++)for(int x=0;x<62;x++){
                    int sx=x+11,sy=y+11;
                    int color=sx>=21&&sx<62&&sy>=25&&sy<59?0xff3c3c28:0;
                    if(sx>=21&&sx<37&&sy>=64-39.2*light/100.&&sy<59)color=0xffc0ffff;
                    if(sx>=47&&sx<62&&sy>=59-38.8*dark/100.&&sy<59)color=0xff000000|((int)(130*wave)<<16)|((int)(35*wave)<<8)|(int)(255*wave);
                    color=over(color,frame.getPixel(sx,sy));
                    if(sx>=21&&sx<62&&sy>=23&&sy<55)color=over(color,shine.getPixel(Math.min(31,(int)((sx-21)/1.3)),sy-23));
                    double radius=Math.hypot(sx+.5-41.5,sy+.5-41.5);
                    pixels.setPixel(x,y,color&0xffffff|((int)((color>>>24)*Math.clamp(29.5-radius,0,1))<<24));
                }
                texture.upload();
            }
            graphics.nextStratum();
            graphics.blit(RenderPipelines.GUI_TEXTURED,ID,7,graphics.guiHeight()-52,0,0,44,44,62,62,62,62);
        }
        var board=player.getAttachedOrCreate(JakData.BOARD);
        if(!board.isEmpty()){
            int x=graphics.guiWidth()/2+96,y=graphics.guiHeight()-20;
            graphics.item(board,x,y);graphics.itemDecorations(client.font,board,x,y);
        }
        int gun=dev.jakmc.content.JakItems.GUNS.indexOf(player.getMainHandItem().getItem());
        if(gun>=0){
            int color=gun/3,x=graphics.guiWidth()/2+120,y=graphics.guiHeight()-20;
            var ammo=new net.minecraft.world.item.ItemStack(dev.jakmc.content.JakItems.AMMUNITION.get(color));
            graphics.item(ammo,x,y);graphics.itemDecorations(client.font,ammo,x,y,Integer.toString((int)Math.ceil(player.getAttachedOrCreate(JakData.AMMO.get(color)))));
        }
    }
    private static int over(int background,int foreground){
        int a=Math.min(255,(foreground>>>24)*2),ba=background>>>24;
        int alpha=a+ba*(255-a)/255;if(alpha==0)return 0;
        int out=alpha<<24;
        for(int shift=0;shift<=16;shift+=8){
            int component=(((foreground>>>shift)&255)*a+((background>>>shift)&255)*ba*(255-a)/255)/alpha;
            out|=component<<shift;
        }
        return out;
    }
    private JakHud(){}
}
