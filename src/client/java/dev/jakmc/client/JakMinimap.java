package dev.jakmc.client;

import com.mojang.blaze3d.platform.NativeImage;
import com.mojang.renderpearl.api.pipeline.RenderPipeline;
import dev.jakmc.JakData;
import dev.jakmc.JakMod;
import dev.jakmc.RuntimeConfig;
import java.io.IOException;
import java.nio.file.Files;
import net.fabricmc.fabric.api.client.event.lifecycle.v1.ClientTickEvents;
import net.fabricmc.fabric.api.client.networking.v1.ClientPlayConnectionEvents;
import net.minecraft.client.DeltaTracker;
import net.minecraft.client.Minecraft;
import net.minecraft.client.gui.GuiGraphicsExtractor;
import net.minecraft.client.multiplayer.ClientLevel;
import net.minecraft.client.renderer.RenderPipelines;
import net.minecraft.client.renderer.texture.DynamicTexture;
import net.minecraft.core.BlockPos;
import net.minecraft.resources.Identifier;
import net.minecraft.util.Mth;
import net.minecraft.world.level.Level;
import net.minecraft.world.level.chunk.status.ChunkStatus;
import net.minecraft.world.level.levelgen.Heightmap;
import net.minecraft.world.level.material.MapColor;

/** Original Jak III compass art over a bounded, world-space Minecraft surface cache. */
public final class JakMinimap {
    private static final int SIDE=128,SAMPLE=2,CROP=68,SCAN_BUDGET=896;
    private static final int WIDTH=72,MARGIN=6;
    private static final int UNSEEN=0xff151817;
    private static final Identifier MAP=JakData.id("hud/terrain_minimap"),RING=JakData.id("hud/jak_mapring"),
        ARROW=JakData.id("hud/jak_map_arrow");
    private static final RenderPipeline ROUND=RenderPipelines.register(RenderPipeline.builder(RenderPipelines.GUI_TEXTURED_SNIPPET)
        .withLocation(JakData.id("pipeline/minimap")).withFragmentShader(JakData.id("core/jak_minimap")).build());
    private static final BlockPos.MutableBlockPos pos=new BlockPos.MutableBlockPos();
    private static ClientLevel world;
    private static DynamicTexture texture;
    private static int texX,texZ,scanX,scanZ,index,age;
    private static boolean ready,scanning,artChecked,hasRing,hasArrow,terrainVisible;
    private static float presentedYaw;
    private static double lastX=Double.NaN,lastZ=Double.NaN,scanSpeed;
    static void initialize(){
        ClientTickEvents.END_CLIENT_TICK.register(client->{
            long profiled=dev.jakmc.entity.ZoomerProfile.begin();
            tick(client);
            dev.jakmc.entity.ZoomerProfile.end("client minimap scan",profiled);
        });
        ClientPlayConnectionEvents.DISCONNECT.register((handler,client)->client.execute(()->clear(client)));
    }
    private static void clear(Minecraft client){
        if(texture!=null){client.getTextureManager().release(MAP);texture=null;}
        world=null;ready=scanning=terrainVisible=false;lastX=lastZ=Double.NaN;scanSpeed=0;
    }
    private static void tick(Minecraft client){
        if(client.level!=world){clear(client);world=client.level;}
        var player=client.player;
        if(world==null||!world.dimension().equals(Level.OVERWORLD)||player==null||!player.getAttachedOrCreate(JakData.ENABLED)||client.isPaused())return;
        int x=player.getBlockX(),z=player.getBlockZ();
        double distance=Double.isNaN(lastX)?0:Math.hypot(player.getX()-lastX,player.getZ()-lastZ);
        lastX=player.getX();lastZ=player.getZ();
        // A high-speed craft can cross the old cache before a fixed 19-tick
        // scan finishes. Bound the work, but finish within its crop margin.
        scanSpeed=Math.max(Math.min(distance,16),scanSpeed*.85);
        int budget=Mth.clamp((int)Math.ceil(scanSpeed*SIDE*SIDE/12),SCAN_BUDGET,4096);
        int refreshDistance=scanSpeed>1?16:32;
        if(texture==null){
            texture=new DynamicTexture("Jak terrain minimap",SIDE,SIDE,true);
            client.getTextureManager().register(MAP,texture);
        }
        int half=SIDE*SAMPLE/2;
        if(scanning&&(Math.abs(x-scanX-half)>72||Math.abs(z-scanZ-half)>72))scanning=false;
        if(!scanning&&(!ready||Math.abs(x-texX-half)>refreshDistance||Math.abs(z-texZ-half)>refreshDistance||++age>=120)){
            scanX=(x&~15)-half;scanZ=(z&~15)-half;
            index=0;scanning=true;
        }
        if(!scanning)return;
        var pixels=texture.getPixels();
        int end=Math.min(SIDE*SIDE,index+budget);
        for(;index<end;index++){
            int col=index%SIDE,row=index/SIDE;
            pixels.setPixel(col,row,sample(scanX+col*SAMPLE,scanZ+row*SAMPLE));
        }
        if(index==SIDE*SIDE){
            texX=scanX;texZ=scanZ;age=0;scanning=false;ready=true;
            texture.upload();
        }
    }
    private static int sample(int x,int z){
        var chunk=world.getChunkSource().getChunk(x>>4,z>>4,ChunkStatus.FULL,false);
        if(chunk==null)return UNSEEN;
        int y=chunk.getHeight(Heightmap.Types.WORLD_SURFACE,x&15,z&15),limit=Math.max(world.getMinY(),y-12);
        var color=MapColor.NONE;
        while(y>=limit){
            var state=chunk.getBlockState(pos.set(x,y,z));
            color=state.getMapColor(world,pos);
            if(color!=MapColor.NONE)break;
            if(state.isSolidRender())return MapColor.STONE.calculateARGBColor(MapColor.Brightness.NORMAL);
            y--;
        }
        if(color==MapColor.NONE)return UNSEEN;
        int northZ=z-SAMPLE;
        var northern=(northZ>>4)==(z>>4)?chunk:
            world.getChunkSource().getChunk(x>>4,northZ>>4,ChunkStatus.FULL,false);
        int north=northern==null?y:northern.getHeight(Heightmap.Types.WORLD_SURFACE,x&15,northZ&15);
        var shade=y>north?MapColor.Brightness.HIGH:y<north?MapColor.Brightness.LOW:MapColor.Brightness.NORMAL;
        return color.calculateARGBColor(shade);
    }
    public static void draw(GuiGraphicsExtractor graphics,DeltaTracker delta){
        terrainVisible=false;
        var client=Minecraft.getInstance();
        var player=client.player;
        if(world==null||!world.dimension().equals(Level.OVERWORLD)||world!=client.level||player==null||client.gui.screen()!=null||!player.getAttachedOrCreate(JakData.ENABLED))return;
        float partial=delta.getGameTimeDeltaPartialTick(false);
        var at=JakPoses.presented();
        if(player.getVehicle() instanceof dev.jakmc.entity.JakVehicle car)at=car.presentationRider();
        if(at==null)at=player.getPosition(partial);
        double centerX=(at.x-texX)/SAMPLE,centerZ=(at.z-texZ)/SAMPLE;
        boolean visible=ready&&centerX>=CROP*.5&&centerX<=SIDE-CROP*.5&&centerZ>=CROP*.5&&centerZ<=SIDE-CROP*.5;
        terrainVisible=visible;
        int r=(int)Math.round(centerX*255/SIDE),g=(int)Math.round(centerZ*255/SIDE),b=Math.round(CROP*.5f*255/SIDE);
        float u=r*SIDE/255f-CROP*.5f,v=g*SIDE/255f-CROP*.5f;
        float yaw=client.gameRenderer.mainCamera().yRot();
        presentedYaw=yaw;
        int mid=MARGIN+WIDTH/2;
        graphics.nextStratum();
        var pose=graphics.pose();
        pose.pushMatrix().translate(mid,mid).rotate((float)Math.toRadians(180-yaw));
        if(visible)graphics.blit(ROUND,MAP,-WIDTH/2,-WIDTH/2,u,v,WIDTH,WIDTH,CROP,CROP,SIDE,SIDE,0xff000000|r<<16|g<<8|b);
        pose.popMatrix();
        if(!artChecked)loadArt(client);
        if(hasRing)graphics.blit(RenderPipelines.GUI_TEXTURED,RING,MARGIN,MARGIN,0,0,WIDTH,WIDTH,128,128,128,128);
        float body=Mth.rotLerp(partial,player.yBodyRotO,player.yBodyRot);
        if(hasArrow){
            pose.pushMatrix().translate(mid,mid).rotate((float)Math.toRadians(45+body-yaw));
            graphics.blit(RenderPipelines.GUI_TEXTURED,ARROW,-7,-7,0,0,14,14,16,16,16,16,0xffffdd78);
            pose.popMatrix();
        }
        float angle=(float)Math.toRadians(yaw);
        int northX=mid+Math.round(Mth.sin(angle)*(WIDTH*.5f-6));
        int northY=mid+Math.round(Mth.cos(angle)*(WIDTH*.5f-6));
        graphics.centeredText(client.font,"N",northX,northY-4,0xffffeab1);
    }
    private static void loadArt(Minecraft client){
        artChecked=true;
        var source=RuntimeConfig.assets.resolve("textures/level-default-minimap");
        hasRing=load(client,source.resolve("hud-mapring-01.png"),RING);
        hasArrow=load(client,source.resolve("map-target-marker.png"),ARROW);
    }
    private static boolean load(Minecraft client,java.nio.file.Path path,Identifier id){
        try(var input=Files.newInputStream(path)){
            var image=NativeImage.read(input);
            for(int y=0;y<image.getHeight();y++)for(int x=0;x<image.getWidth();x++){
                int color=image.getPixel(x,y);
                image.setPixel(x,y,color&0x00ffffff|Math.min(255,(color>>>24)*2)<<24);
            }
            client.getTextureManager().register(id,new DynamicTexture(id::getPath,image));
            return true;
        }catch(IOException failure){
            JakMod.LOG.warn("Could not load original minimap art {}",path,failure);
            return false;
        }
    }
    static boolean ready(){return ready;}
    static boolean terrainVisible(){return terrainVisible;}
    static float presentedYaw(){return presentedYaw;}
    private JakMinimap(){}
}
