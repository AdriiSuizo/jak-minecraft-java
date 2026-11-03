package dev.jakmc.client.model;

import com.google.gson.JsonArray;
import com.mojang.blaze3d.vertex.PoseStack;
import com.mojang.serialization.MapCodec;
import com.mojang.serialization.Codec;
import com.mojang.serialization.codecs.RecordCodecBuilder;
import dev.jakmc.JakData;
import dev.jakmc.content.JakProps;
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
import net.minecraft.world.level.block.state.BlockState;
import net.minecraft.world.level.block.state.properties.BlockStateProperties;
import org.joml.Vector3f;
import org.joml.Vector3fc;

/** Source triangles are baked once into the terrain mesh and use the same sprites in inventory. */
public final class PropModels {
    private static final Identifier BLOCK_ATLAS=Identifier.withDefaultNamespace("textures/atlas/blocks.png");
    private record Point(float x,float y,float z,float u,float v,int color){}
    private record Triangle(Point[] points,Identifier texture,boolean blended){}
    private record Geometry(List<Triangle> triangles){}
    private static final Map<Integer,Geometry> GEOMETRY=new LinkedHashMap<>();

    public static void initialize(){
        for(var entry:JakProps.MODELS.values())GEOMETRY.put(entry.id(),geometry(entry.triangles()));
        ModelLoadingPlugin.register(context->{
            for(var model:JakProps.MODELS.values()){
                var unbaked=new Root(model.path(),GEOMETRY.get(model.id()));
                context.registerBlockStateResolver(model.block(),states->{
                    for(BlockState state:states.block().getStateDefinition().getPossibleStates())states.setModel(state,unbaked);
                });
            }
        });
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
            tris.add(new Triangle(points,Identifier.fromNamespaceAndPath("jak","block/"+row.get("texture").getAsString()),blend));
        }
        return new Geometry(List.copyOf(tris));
    }

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
                emitter.materialBake(material,MutableQuadView.BAKE_NORMALIZED)
                    .chunkLayer(triangle.blended()?ChunkSectionLayer.TRANSLUCENT:ChunkSectionLayer.byTransparency(material.sprite().transparency()))
                    .ambientOcclusion(net.fabricmc.fabric.api.util.TriState.FALSE).emit();
            }
            return new Baked(new MeshQuadCollection(mesh.immutableCopy()),materials.values().iterator().next());
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
                    RenderTypes.entityCutout(sprite.atlasLocation());
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
