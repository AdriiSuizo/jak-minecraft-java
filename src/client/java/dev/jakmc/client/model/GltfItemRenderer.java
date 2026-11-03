package dev.jakmc.client.model;

import com.mojang.blaze3d.vertex.PoseStack;
import com.mojang.serialization.*;
import com.mojang.serialization.codecs.RecordCodecBuilder;
import dev.jakmc.*;
import java.util.*;
import java.util.function.Consumer;
import net.minecraft.client.renderer.SubmitNodeCollector;
import net.minecraft.client.renderer.special.*;
import net.minecraft.world.item.ItemStack;
import org.joml.Vector3fc;
import org.joml.Vector3f;

/** One original model for inventory, dropped item and first-person item rendering. */
public final class GltfItemRenderer implements SpecialModelRenderer<Void> {
    private static final Map<String,GltfModel> MODELS=new java.util.concurrent.ConcurrentHashMap<>();
    private final String file;
    private final float scale;
    private final String clip;
    private GltfModel.Posed fixedPose;private Vector3f[] extents;
    public GltfItemRenderer(String file,float scale,String clip){this.file=file;this.scale=scale;this.clip=clip;}
    public static void initialize(){SpecialModelRenderers.ID_MAPPER.put(JakData.id("gltf"),Unbaked.CODEC);}
    @Override public Void extractArgument(ItemStack item){return null;}
    private GltfModel model(){return MODELS.computeIfAbsent(file,name->{
            if(!name.matches("(?:[a-z0-9_-]+/)?[a-z0-9_+.-]+\\.glb"))throw new IllegalArgumentException("Invalid original model name");
            try{var path=name.contains("/")?name:"common/"+name;return new GltfModel(RuntimeConfig.assets.resolve("levels").resolve(path),"item_"+name.replace('/','_').replace(".glb",""));}
            catch(java.io.IOException e){throw new IllegalStateException("Original item model missing: "+name,e);}
        });}
    private GltfModel.Posed pose(GltfModel model){if(fixedPose==null){fixedPose=clip.isEmpty()?model.pose("",0):model.poseAtTime(clip,Float.MAX_VALUE);model.prepare(fixedPose);}return fixedPose;}
    @Override public void getExtents(Consumer<Vector3fc> out){
        GltfModel model=model();if(extents==null)extents=model.bounds(pose(model),new org.joml.Matrix4f().translation(.5f,.25f,.5f).scale(scale));var bounds=extents;
        for(float x:new float[]{bounds[0].x,bounds[1].x})for(float y:new float[]{bounds[0].y,bounds[1].y})for(float z:new float[]{bounds[0].z,bounds[1].z})out.accept(new Vector3f(x,y,z));
    }
    @Override public void submit(Void argument,PoseStack stack,SubmitNodeCollector collector,int light,int overlay,boolean foil,int outline){
        GltfModel model=model();stack.pushPose();stack.translate(.5f,.25f,.5f);stack.scale(scale,scale,scale);model.submit(pose(model),stack,collector,light);stack.popPose();
    }
    public record Unbaked(String file,float scale,String clip) implements SpecialModelRenderer.Unbaked<Void> {
        public static final MapCodec<Unbaked> CODEC=RecordCodecBuilder.mapCodec(i->i.group(Codec.STRING.fieldOf("file").forGetter(Unbaked::file),Codec.FLOAT.optionalFieldOf("scale",1f).forGetter(Unbaked::scale),Codec.STRING.optionalFieldOf("clip","").forGetter(Unbaked::clip)).apply(i,Unbaked::new));
        public MapCodec<Unbaked> type(){return CODEC;}
        public SpecialModelRenderer<Void> bake(SpecialModelRenderer.BakingContext context){return new GltfItemRenderer(file,scale,clip);}
    }
}
