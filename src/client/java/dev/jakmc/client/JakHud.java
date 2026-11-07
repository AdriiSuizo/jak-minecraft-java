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
        // The Super Nova's white screen over the whole screen (gun-dark-shot.gc do-white-screen-effect).
        int filter=dev.jakmc.client.sky.JakNukeScreen.filter();
        if(filter>>>24!=0)graphics.fill(0,0,client.getWindow().getGuiScaledWidth(),client.getWindow().getGuiScaledHeight(),filter);
        if(!attempted){
            attempted=true;
            var root=RuntimeConfig.assets.resolve("textures/level-default-minimap");
            try(var f=Files.newInputStream(root.resolve("hud-newhud-01.png"));var s=Files.newInputStream(root.resolve("hud-newhud-shine-01.png"))){
                frame=NativeImage.read(f);shine=NativeImage.read(s);
                texture=new DynamicTexture("Original Jak eco ring",62,62,true);client.getTextureManager().register(ID,texture);
            }catch(java.io.IOException e){JakMod.LOG.error("Original eco HUD unavailable",e);}
        }
        int light=Math.clamp(Math.round(player.getAttachedOrCreate(JakData.LIGHT_ECO)),0,100);
        // Up, not round: a sliver of dark eco left still shows (and still keeps him Dark).
        int dark=Math.clamp((int)Math.ceil(player.getAttachedOrCreate(JakData.DARK_ECO)),0,100);
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
        ecoTimer(graphics,client,player);
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
    // Jak 1's eco timer (hud-power, hud-classes.gc; the Rust port's jak1_eco_timer.rs) while he carries Jak 1's eco
    // (Adrii 06/11: "no hay reloj"): the backing dot (harddot, part 328), three pie slices (hud-pie, 329-331) turned
    // by the time left (calculate-rotation-and-color-for-slice) in the eco's colour, flashing under a fifth, and the
    // meter's frame (hud-ecometer, 327). Places are Jak 1's 512x416 HUD in 640x480, from the right edge.
    private static final net.minecraft.resources.Identifier[] TIMER={JakData.id("hud/jak1_pie"),JakData.id("hud/jak1_ecometer"),JakData.id("hud/jak1_harddot")};
    private static final int[] TIMER_SIZE=new int[3];
    private static int timerState;
    private static void ecoTimer(GuiGraphicsExtractor graphics,Minecraft client,net.minecraft.client.player.LocalPlayer player){
        var pose=JakPoses.rendered(player.getId());
        if(pose==null||!(pose.raw().get("ecoTimer") instanceof com.google.gson.JsonArray timer))return;
        if(timerState==0){
            timerState=2;String[] names={"hud-pie","hud-ecometer","harddot"};
            for(int i=0;i<3;i++){
                var file=dev.jakmc.client.sky.JakSkyRender.jak1("effects",names[i]);
                if(file==null){timerState=1;break;}
                try(var in=Files.newInputStream(file)){
                    var image=NativeImage.read(in);TIMER_SIZE[i]=image.getWidth();
                    for(int y=0;y<image.getHeight();y++)for(int x=0;x<image.getWidth();x++){int c=image.getPixel(x,y);image.setPixel(x,y,c&0x00ffffff|Math.min(255,(c>>>24)*2)<<24);}
                    var id=TIMER[i];client.getTextureManager().register(id,new DynamicTexture(id::getPath,image));
                }catch(java.io.IOException e){JakMod.LOG.error("Jak 1 eco timer texture unavailable",e);timerState=1;break;}
            }
        }
        if(timerState!=2)return;
        int kind=timer.get(0).getAsInt();float left=timer.get(1).getAsFloat();
        float k=graphics.guiHeight()/480f;
        // part-hud-eco-timer-0x-func: yellow, red, blue, green.
        float[][] tints={{128,96,0},{128,32,0},{0,64,128},{0,128,32}};
        float[] tint=tints[Math.clamp(kind,0,3)];float green=tint[1];
        if(left>0&&left<.2f&&(((int)(System.nanoTime()/16_666_667L))&4)==0)green*=2;
        int slice=0xff000000|channel(tint[0])<<16|channel(green)<<8|channel(tint[2]);
        graphics.nextStratum();
        piece(graphics,k,435,370,2.5f,2,0,0x80ffc77e);
        for(int i=0;i<3;i++)piece(graphics,k,432,368,2,0,sliceTurn(i,left),slice);
        piece(graphics,k,435,370,3.2f,1,0,0xffffffff);
    }
    private static int channel(float ps2){return Math.clamp(Math.round(ps2/128*255),0,255);}
    /** One piece at Jak 1's (x, y), its size in metres at 16 pixels each, turned deg degrees. */
    private static void piece(GuiGraphicsExtractor graphics,float k,float x,float y,float metres,int texture,float deg,int color){
        float size=metres*16*640/512f*k,cx=graphics.guiWidth()+(x*640/512f-640)*k,cy=y*480/416f*k;int side=TIMER_SIZE[texture];
        var pose=graphics.pose();
        // Jak 1 turns its slices counter-clockwise on screen (sprite3_3d.vert; the Rust port's quad_turned, turned as its
        // RotateSt turns): the slices drain from the top right round to the plate at the bottom right.
        pose.pushMatrix().translate(cx,cy).rotate((float)Math.toRadians(-deg)).scale(size/side,size/side);
        graphics.blit(RenderPipelines.GUI_TEXTURED,TIMER[texture],-side/2,-side/2,0,0,side,side,side,side,side,side,color);
        pose.popMatrix();
    }
    /** calculate-rotation-and-color-for-slice: slice i's turn with left of the most time left (degrees). */
    static float sliceTurn(int i,float left){
        if(left<=0)return -16566.045f/182.04445f;
        return switch(i){
            case 0 -> left>1?5:5+263*(1-left);
            case 1 -> left>2/3f?95:95+173*(1-1.5f*left);
            default -> left>1/3f?185:185+83*(1-3*left);
        };
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
