package dev.jakmc.client.model;

import com.google.gson.JsonArray;
import com.mojang.blaze3d.vertex.PoseStack;
import com.mojang.serialization.MapCodec;
import com.mojang.serialization.Codec;
import com.mojang.serialization.codecs.RecordCodecBuilder;
import dev.jakmc.JakData;
import dev.jakmc.content.JakProps;
import dev.jakmc.content.JakAssemblies;
import java.util.ArrayList;
import java.util.LinkedHashMap;
import java.util.List;
import java.util.Map;
import java.util.function.Consumer;
import java.util.function.Predicate;
import net.fabricmc.fabric.api.client.model.loading.v1.ModelLoadingPlugin;
import net.fabricmc.fabric.api.client.renderer.v1.Renderer;
import net.fabricmc.fabric.api.client.renderer.v1.mesh.*;
import net.fabricmc.fabric.api.client.renderer.v1.model.MeshQuadCollection;
import net.minecraft.client.renderer.SubmitNodeCollector;
import net.minecraft.client.renderer.block.BlockAndTintGetter;
import net.minecraft.client.renderer.block.dispatch.BlockStateModel;
import net.minecraft.client.renderer.block.dispatch.BlockStateModelPart;
import net.minecraft.client.renderer.chunk.ChunkSectionLayer;
import net.minecraft.client.renderer.rendertype.RenderTypes;
import net.minecraft.client.renderer.special.SpecialModelRenderer;
import net.minecraft.client.renderer.special.SpecialModelRenderers;
import net.minecraft.client.resources.model.ModelBaker;
import net.minecraft.client.resources.model.ResolvableModel;
import net.minecraft.client.resources.model.geometry.BakedQuad;
import net.minecraft.client.resources.model.sprite.Material;
import net.minecraft.client.resources.model.sprite.SpriteGetter;
import net.minecraft.client.resources.model.sprite.SpriteId;
import net.minecraft.core.BlockPos;
import net.minecraft.core.Direction;
import net.minecraft.resources.Identifier;
import net.minecraft.util.RandomSource;
import net.minecraft.world.item.ItemStack;
import net.minecraft.world.level.block.Block;
import net.minecraft.world.level.block.state.BlockState;
import net.minecraft.world.level.block.state.properties.BlockStateProperties;
import net.minecraft.client.Minecraft;
import net.minecraft.world.phys.Vec3;
import org.joml.Vector3f;
import org.joml.Vector3fc;

/** Source triangles are baked once into the terrain mesh and use the same sprites in inventory. */
public final class PropModels {
    public static final int PROP_QUAD_TAG=0x4a414b50;
    public static final int BARREL_QUAD_TAG=0x4a414b42;
    private static final Identifier BLOCK_ATLAS=Identifier.withDefaultNamespace("textures/atlas/blocks.png");
    private record Point(float x,float y,float z,float u,float v,int color){}
    private record Triangle(Point[] points,Identifier texture,boolean blended){}
    private record Geometry(List<Triangle> triangles){}
    private static final Map<Integer,Geometry> GEOMETRY=new LinkedHashMap<>();
    /** Each assembly cell's own geometry, for the placement preview. */
    private static final Map<Block,Geometry> CELLS=new java.util.HashMap<>();

    public static void initialize(){
        for(var entry:JakProps.MODELS.values())GEOMETRY.put(entry.id(),geometry(entry.triangles()));
        Map<Block,Root> roots=new LinkedHashMap<>();
        for(var model:JakProps.MODELS.values())roots.put(model.block(),new Root(model.path(),GEOMETRY.get(model.id())));
        for(var model:JakAssemblies.MODELS.values()){
            var item=new ArrayList<Triangle>();
            for(var part:model.parts()){
                Geometry geometry=geometry(part.triangles());
                roots.put(part.block(),new Root(model.path(),geometry));
                CELLS.put(part.block(),geometry);
                for(var triangle:geometry.triangles()){
                    var points=new Point[3];
                    for(int i=0;i<points.length;i++){
                        var point=triangle.points()[i];
                        points[i]=new Point((point.x+part.offset().getX())/model.extent(),
                            (point.y+part.offset().getY())/model.extent(),(point.z+part.offset().getZ())/model.extent(),
                            point.u,point.v,point.color);
                    }
                    item.add(new Triangle(points,triangle.texture(),triangle.blended()));
                }
            }
            GEOMETRY.put(model.id(),new Geometry(List.copyOf(item)));
        }
        ModelLoadingPlugin.register(context->roots.forEach((block,root)->
            context.registerBlockStateResolver(block,states->{
                for(BlockState state:block.getStateDefinition().getPossibleStates())states.setModel(state,root);
            })
        ));
        SpecialModelRenderers.ID_MAPPER.put(JakData.id("prop"),ItemModel.CODEC);
    }

    private static Geometry geometry(JsonArray rows){
        var tris=new ArrayList<Triangle>(rows.size());
        for(var element:rows){
            var row=element.getAsJsonObject();
            boolean blend=row.get("blend").getAsBoolean();
            Point[] points=new Point[3];
            for(int i=0;i<3;i++){
                var p=row.getAsJsonArray("p").get(i).getAsJsonArray();
                var uv=row.getAsJsonArray("uv").get(i).getAsJsonArray();
                var rgba=row.getAsJsonArray("color").get(i).getAsJsonArray();
                int color=(blend?color(rgba,3):255)<<24|color(rgba,0)<<16|color(rgba,1)<<8|color(rgba,2);
                points[i]=new Point(p.get(0).getAsFloat(),p.get(1).getAsFloat(),p.get(2).getAsFloat(),
                    uv.get(0).getAsFloat(),uv.get(1).getAsFloat(),color);
            }
            String texture=row.get("texture").getAsString();
            var material=Identifier.fromNamespaceAndPath("jak","block/"+texture);
            if(opaque(material)){
                blend=false;
                for(int i=0;i<3;i++)points[i]=new Point(points[i].x,points[i].y,points[i].z,points[i].u,points[i].v,points[i].color|0xFF000000);
            }
            tris.add(new Triangle(points,material,blend));
            // The games draw these meshes double-sided (glTF doubleSided, which the export drops): the
            // urn's mouth and the palm trunk's inward-wound faces showed their insides (Adrii 06/11).
            tris.add(new Triangle(new Point[]{points[2],points[1],points[0]},material,blend));
        }
        return new Geometry(List.copyOf(tris));
    }

    /** Barrels: their texture alpha is the PS2's, not transparency (an opaque can in the games). */
    private static boolean opaque(Identifier texture){return texture.getPath().contains("barrel")||texture.getPath().endsWith("/city-slum-burning-can");}

    private static int color(JsonArray rgba,int index){return Math.min(255,2*rgba.get(index).getAsInt());}

    private static Vector3f position(Point point,int turns){
        float x=point.x,z=point.z;
        for(int i=0;i<turns;i++){
            float next=1-z;
            z=x;x=next;
        }
        return new Vector3f(x,point.y,z);
    }

    private record Root(String name,Geometry geometry) implements BlockStateModel.UnbakedRoot {
        @Override public void resolveDependencies(ResolvableModel.Resolver resolver){}
        @Override public Object visualEqualityGroup(BlockState state){return state;}
        @Override public BlockStateModel bake(BlockState state,ModelBaker baker){
            var mesh=Renderer.get().mutableMesh();
            var emitter=mesh.emitter();
            var materials=new LinkedHashMap<Identifier,Material.Baked>();
            int turns=JakProps.turns(state.getValue(BlockStateProperties.HORIZONTAL_FACING));
            for(var triangle:geometry.triangles()){
                var material=materials.computeIfAbsent(triangle.texture(),id->baker.materials().get(new Material(id),()->name));
                for(int i=0;i<4;i++){
                    var point=triangle.points()[Math.min(i,2)];
                    emitter.pos(i,position(point,turns)).uv(i,point.u(),point.v()).color(i,point.color());
                }
                boolean opaque=opaque(triangle.texture());
                // A two-sided source triangle must have the same diffuse shade on both windings.
                if(opaque)emitter.shadeDirectionOverride(Direction.UP);
                emitter.tag(opaque?BARREL_QUAD_TAG:PROP_QUAD_TAG);
                emitter.materialBake(material,MutableQuadView.BAKE_NORMALIZED)
                    .chunkLayer(triangle.blended()?ChunkSectionLayer.TRANSLUCENT:opaque?ChunkSectionLayer.SOLID
                        :ChunkSectionLayer.byTransparency(material.sprite().transparency()))
                    .ambientOcclusion(net.fabricmc.fabric.api.util.TriState.FALSE).emit();
            }
            var particle=materials.isEmpty()
                ?baker.materials().get(new Material(Identifier.fromNamespaceAndPath("jak","block/skatea-ramp-01")),()->name)
                :materials.values().iterator().next();
            return new Baked(new MeshQuadCollection(mesh.immutableCopy()),particle);
        }
    }

    private record Baked(MeshQuadCollection quads,Material.Baked particleMaterial)
        implements BlockStateModel,BlockStateModelPart {
        @Override public void collectParts(RandomSource random,List<BlockStateModelPart> parts){parts.add(this);}
        @Override public List<BakedQuad> getQuads(Direction face){return quads.getQuads(face);}
        @Override public int materialFlags(){return quads.materialFlags();}
        @Override public boolean useAmbientOcclusion(){return false;}
        @Override public void emitQuads(QuadEmitter emitter,BlockAndTintGetter level,BlockPos at,BlockState state,
                                        RandomSource random,Predicate<Direction> cullTest){
            quads.getMesh().outputTo(emitter);
        }
    }

    /**
     * The placement preview (Adrii 06/11): an object bigger than a block drawn faint (alpha 0..1) where it
     * would go, each cell as its terrain mesh turned to the cell's facing.
     */
    public static void submitGhost(Map<BlockPos,BlockState> cells,Vec3 camera,PoseStack stack,SubmitNodeCollector collector,float alpha){
        var atlas=Minecraft.getInstance().getAtlasManager().getAtlasOrThrow(net.minecraft.data.AtlasIds.BLOCKS);
        int a=Math.round(alpha*255)<<24;
        Map<Identifier,List<Object[]>> batches=new LinkedHashMap<>();
        for(var cell:cells.entrySet()){
            var geometry=CELLS.get(cell.getValue().getBlock());
            if(geometry==null)continue;
            int turns=cell.getValue().hasProperty(BlockStateProperties.HORIZONTAL_FACING)
                ?JakProps.turns(cell.getValue().getValue(BlockStateProperties.HORIZONTAL_FACING)):0;
            for(var triangle:geometry.triangles())batches.computeIfAbsent(triangle.texture(),_ ->new ArrayList<>())
                .add(new Object[]{triangle,cell.getKey(),turns});
        }
        for(var batch:batches.entrySet()){
            var sprite=atlas.getSprite(batch.getKey());
            stack.pushPose();
            collector.submitCustomGeometry(stack,RenderTypes.entityTranslucent(sprite.atlasLocation()),(pose,vertices)->{
                for(var entry:batch.getValue()){
                    var triangle=(Triangle)entry[0];var at=(BlockPos)entry[1];int turns=(int)entry[2];
                    var points=triangle.points();
                    var normal=position(points[1],turns).sub(position(points[0],turns))
                        .cross(position(points[2],turns).sub(position(points[0],turns))).normalize();
                    pose.normal().transform(normal);
                    for(int i=0;i<4;i++){
                        var point=points[Math.min(i,2)];
                        var local=position(point,turns).add((float)(at.getX()-camera.x),(float)(at.getY()-camera.y),(float)(at.getZ()-camera.z));
                        var p=pose.pose().transformPosition(local);
                        vertices.addVertex(p.x,p.y,p.z,a|point.color()&0xFFFFFF,sprite.getU(point.u()),sprite.getV(point.v()),
                            net.minecraft.client.renderer.texture.OverlayTexture.NO_OVERLAY,0xF000F0,normal.x,normal.y,normal.z);
                    }
                }
            });
            stack.popPose();
        }
    }

    public record ItemModel(int id) implements SpecialModelRenderer.Unbaked<Void> {
        public static final MapCodec<ItemModel> CODEC=RecordCodecBuilder.mapCodec(i->
            i.group(Codec.INT.fieldOf("id").forGetter(ItemModel::id)).apply(i,ItemModel::new));
        @Override public MapCodec<ItemModel> type(){return CODEC;}
        @Override public SpecialModelRenderer<Void> bake(SpecialModelRenderer.BakingContext context){
            Geometry geometry=GEOMETRY.get(id);
            if(geometry==null)return null;
            Map<MaterialKey,List<Triangle>> batches=new LinkedHashMap<>();
            for(var triangle:geometry.triangles()){
                var key=new MaterialKey(triangle.texture(),triangle.blended());
                batches.computeIfAbsent(key,_ ->new ArrayList<>()).add(triangle);
            }
            return new BakedItem(geometry,batches,context.sprites());
        }
    }

    private record MaterialKey(Identifier id,boolean blend){}
    private record BakedItem(Geometry geometry,Map<MaterialKey,List<Triangle>> batches,SpriteGetter sprites) implements SpecialModelRenderer<Void> {
        @Override public Void extractArgument(ItemStack stack){return null;}
        @Override public void getExtents(Consumer<Vector3fc> consumer){
            for(var triangle:geometry.triangles())for(var point:triangle.points())consumer.accept(position(point,0));
        }
        @Override public void submit(Void data,PoseStack stack,SubmitNodeCollector collector,int light,int overlay,boolean foil,int outline){
            for(var batch:batches.entrySet()){
                var sprite=sprites.get(new SpriteId(BLOCK_ATLAS,batch.getKey().id()));
                var layer=batch.getKey().blend()?RenderTypes.entityTranslucent(sprite.atlasLocation()):
                    opaque(batch.getKey().id())?RenderTypes.entitySolid(sprite.atlasLocation()):RenderTypes.entityCutout(sprite.atlasLocation());
                collector.submitCustomGeometry(stack,layer,(pose,vertices)->{
                    for(var triangle:batch.getValue()){
                        var points=triangle.points();
                        var normal=position(points[1],0).sub(position(points[0],0))
                            .cross(position(points[2],0).sub(position(points[0],0))).normalize();
                        pose.normal().transform(normal);
                        for(int i=0;i<4;i++){
                            var point=points[Math.min(i,2)];
                            var at=pose.pose().transformPosition(position(point,0));
                            vertices.addVertex(at.x,at.y,at.z,point.color(),sprite.getU(point.u()),sprite.getV(point.v()),
                                overlay,light,normal.x,normal.y,normal.z);
                        }
                    }
                });
            }
        }
    }
}
