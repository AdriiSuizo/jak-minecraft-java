package dev.jakmc.client.model;

import java.util.ArrayList;
import java.util.function.Predicate;
import net.fabricmc.fabric.api.client.model.loading.v1.ModelLoadingPlugin;
import net.fabricmc.fabric.api.client.model.loading.v1.wrapper.WrapperBlockStateModel;
import net.fabricmc.fabric.api.client.renderer.v1.mesh.MutableQuadView;
import net.fabricmc.fabric.api.client.renderer.v1.mesh.QuadEmitter;
import net.fabricmc.fabric.api.client.renderer.v1.sprite.FabricTextureAtlas;
import net.minecraft.client.Minecraft;
import net.minecraft.client.renderer.block.BlockAndTintGetter;
import net.minecraft.client.renderer.block.dispatch.BlockStateModel;
import net.minecraft.client.renderer.block.dispatch.BlockStateModelPart;
import net.minecraft.core.BlockPos;
import net.minecraft.core.Direction;
import net.minecraft.data.AtlasIds;
import net.minecraft.resources.Identifier;
import net.minecraft.util.RandomSource;
import net.minecraft.world.level.block.state.BlockState;

/** Textures named jak:block/wN_* are laid over the world, one tile every N blocks, so the grid does not show. */
public final class JakWorldTextures {
    public static void initialize(){
        ModelLoadingPlugin.register(context->context.modifyBlockModelAfterBake().register((model,source)->laid(model)?new Laid(model):model));
    }

    // minecraft_terrain mesh.rs world_texture: the tile size in blocks, or 0.
    static int tile(Identifier id){
        String path=id.getPath();
        if(!id.getNamespace().equals("jak")||!path.startsWith("block/w")||path.length()<9)return 0;
        char digit=path.charAt(7);
        return Character.isDigit(digit)&&path.charAt(8)=='_'?digit-'0':0;
    }

    private static boolean laid(BlockStateModel model){
        var parts=new ArrayList<BlockStateModelPart>();
        model.collectParts(RandomSource.create(42L),parts);
        for(var part:parts){
            for(var quad:part.getQuads(null))if(tile(quad.materialInfo().sprite().contents().name())>0)return true;
            for(Direction face:Direction.values())
                for(var quad:part.getQuads(face))if(tile(quad.materialInfo().sprite().contents().name())>0)return true;
        }
        return false;
    }

    // mesh.rs world_uvs/plane_of: each corner at its world place on the face's plane, in the tile its middle is in.
    private static boolean lay(MutableQuadView quad,BlockPos at){
        var atlas=Minecraft.getInstance().getAtlasManager().getAtlasOrThrow(AtlasIds.BLOCKS);
        var sprite=((FabricTextureAtlas)(Object)atlas).spriteFinder().find(quad);
        int tile=sprite==null?0:tile(sprite.contents().name());
        if(tile==0)return true;
        Direction face=quad.nominalFace()!=null?quad.nominalFace():quad.lightFace();
        int plane=switch(face){case UP,DOWN -> 0; case NORTH,SOUTH -> 1; default -> 2;};
        float[] a=new float[4],b=new float[4];
        float midA=0,midB=0;
        for(int i=0;i<4;i++){
            float x=at.getX()+quad.x(i),y=at.getY()+quad.y(i),z=at.getZ()+quad.z(i);
            a[i]=plane==2?z:x;
            b[i]=plane==0?z:-y;
            midA+=a[i]/4;midB+=b[i]/4;
        }
        float baseA=(float)Math.floor(midA/tile)*tile,baseB=(float)Math.floor(midB/tile)*tile;
        for(int i=0;i<4;i++){
            float s=Math.clamp((a[i]-baseA)/tile,0f,1f),t=Math.clamp((b[i]-baseB)/tile,0f,1f);
            quad.uv(i,sprite.getU0()+(sprite.getU1()-sprite.getU0())*s,sprite.getV0()+(sprite.getV1()-sprite.getV0())*t);
        }
        return true;
    }

    private static final class Laid extends WrapperBlockStateModel {
        Laid(BlockStateModel wrapped){super(wrapped);}
        @Override public void emitQuads(QuadEmitter emitter,BlockAndTintGetter level,BlockPos at,BlockState state,
                                        RandomSource random,Predicate<Direction> cullTest){
            var pos=at.immutable();
            emitter.pushTransform(quad->lay(quad,pos));
            super.emitQuads(emitter,level,at,state,random,cullTest);
            emitter.popTransform();
        }
        // The UVs depend on the block's place: never share its geometry between positions.
        @Override public Object createGeometryKey(BlockAndTintGetter level,BlockPos at,BlockState state,RandomSource random){return null;}
    }
}
