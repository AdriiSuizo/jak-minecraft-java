package dev.jakmc.client.darkeco;

import com.mojang.blaze3d.vertex.VertexConsumer;
import net.fabricmc.fabric.api.client.render.fluid.v1.FluidRenderHandler;
import net.fabricmc.fabric.api.client.render.fluid.v1.FluidRendering;
import net.minecraft.client.Minecraft;
import net.minecraft.client.renderer.block.BlockAndTintGetter;
import net.minecraft.client.renderer.block.FluidRenderer;
import net.minecraft.client.renderer.texture.TextureAtlasSprite;
import net.minecraft.core.BlockPos;
import net.minecraft.data.AtlasIds;
import net.minecraft.resources.Identifier;
import net.minecraft.world.level.block.state.BlockState;
import net.minecraft.world.level.material.FluidState;

/** Keeps vanilla's fluid heights and culling, but shares one ripple sheet across eight blocks. */
final class DarkEcoSheet implements FluidRenderHandler {
    private static final FluidRenderHandler VANILLA_HANDLER=new FluidRenderHandler(){};
    private static final FluidRendering.DefaultRenderer VANILLA_RENDERER=new FluidRendering.DefaultRenderer(){};
    private final Identifier texture;

    DarkEcoSheet(Identifier texture){this.texture=texture;}

    @Override public void renderFluid(FluidRenderer renderer,BlockPos pos,BlockAndTintGetter level,
            FluidRenderer.Output output,BlockState block,FluidState fluid){
        var atlas=Minecraft.getInstance().getAtlasManager().getAtlasOrThrow(AtlasIds.BLOCKS);
        var sprite=atlas.getSprite(texture);
        FluidRenderer.Output sheet=layer->new SheetVertices(output.getBuilder(layer),sprite,pos);
        FluidRendering.render(renderer,VANILLA_HANDLER,level,pos,sheet,block,fluid,VANILLA_RENDERER);
    }

    private static final class SheetVertices implements VertexConsumer {
        private final VertexConsumer out;
        private final TextureAtlasSprite sprite;
        private final int sectionX,sectionY,sectionZ;
        private final int sheetX,sheetY,sheetZ;
        private final float[] xyz=new float[12];
        private final int[] colors=new int[4],lights=new int[4];
        private int vertices;

        SheetVertices(VertexConsumer out,TextureAtlasSprite sprite,BlockPos pos){
            this.out=out;
            this.sprite=sprite;
            sectionX=pos.getX()&15;sectionY=pos.getY()&15;sectionZ=pos.getZ()&15;
            sheetX=pos.getX()&7;sheetY=pos.getY()&7;sheetZ=pos.getZ()&7;
        }

        @Override public void addVertex(float x,float y,float z,int color,float u,float v,
                int overlay,int light,float nx,float ny,float nz){
            int i=vertices*3;
            xyz[i]=x;xyz[i+1]=y;xyz[i+2]=z;
            colors[vertices]=color;lights[vertices]=light;
            if(++vertices<4)return;
            float minX=Math.min(Math.min(xyz[0],xyz[3]),Math.min(xyz[6],xyz[9]));
            float minZ=Math.min(Math.min(xyz[2],xyz[5]),Math.min(xyz[8],xyz[11]));
            boolean alongX=Math.max(Math.max(xyz[0],xyz[3]),Math.max(xyz[6],xyz[9]))-minX>.1f;
            boolean alongZ=Math.max(Math.max(xyz[2],xyz[5]),Math.max(xyz[8],xyz[11]))-minZ>.1f;
            boolean topOrBottom=alongX&&alongZ;
            for(int n=0;n<4;n++){
                int a=n*3;
                float sx=xyz[a]-sectionX+sheetX,sy=xyz[a+1]-sectionY+sheetY,sz=xyz[a+2]-sectionZ+sheetZ;
                float us=alongX?sx:sz;
                float vs=topOrBottom?sz:sy;
                // Half-texel inset prevents the atlas's neighboring sprites from bleeding into the sheet.
                float tu=coordinate(sprite.getU0(),sprite.getU1(),us);
                float tv=coordinate(sprite.getV0(),sprite.getV1(),vs);
                out.addVertex(xyz[a],xyz[a+1],xyz[a+2],colors[n],tu,tv,overlay,lights[n],nx,ny,nz);
            }
            vertices=0;
        }

        private static float coordinate(float low,float high,float block){
            return low+(high-low)*(.5f+Math.clamp(block,0,8)*(127f/8))/128;
        }

        // FluidRenderer submits each complete vertex through the packed overload.
        @Override public VertexConsumer addVertex(float x,float y,float z){throw new UnsupportedOperationException();}
        @Override public VertexConsumer setColor(int r,int g,int b,int a){throw new UnsupportedOperationException();}
        @Override public VertexConsumer setColor(int color){throw new UnsupportedOperationException();}
        @Override public VertexConsumer setUv(float u,float v){throw new UnsupportedOperationException();}
        @Override public VertexConsumer setUv1(int u,int v){throw new UnsupportedOperationException();}
        @Override public VertexConsumer setUv2(int u,int v){throw new UnsupportedOperationException();}
        @Override public VertexConsumer setUv3(float u,float v){throw new UnsupportedOperationException();}
        @Override public VertexConsumer setNormal(float x,float y,float z){throw new UnsupportedOperationException();}
        @Override public VertexConsumer setLineWidth(float width){throw new UnsupportedOperationException();}
    }
}
