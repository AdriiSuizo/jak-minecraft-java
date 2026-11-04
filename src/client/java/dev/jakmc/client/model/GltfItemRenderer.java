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
    private final String file,game,damageTable;
    private final float scale;
    private final String clip;
    private boolean fitGui;
    private GltfModel.Posed fixedPose;private Vector3f[] extents;
    public GltfItemRenderer(String file,float scale,String clip,String game,String damageTable){this.file=file;this.scale=scale;this.clip=clip;this.game=game;this.damageTable=damageTable;}
    public static void initialize(){SpecialModelRenderers.ID_MAPPER.put(JakData.id("gltf"),Unbaked.CODEC);}
    @Override public Void extractArgument(ItemStack item){return null;}
    private GltfModel model(){return MODELS.computeIfAbsent(game+"/"+file+"/"+damageTable,key->{
            String name=file;
            if(!name.matches("(?:[a-z0-9_-]+/)?[a-z0-9_+.-]+\\.glb"))throw new IllegalArgumentException("Invalid original model name");
            try{
                if(!game.equals("jak2")&&!game.equals("jak3"))throw new IllegalArgumentException("Invalid original game");
                var path=name.contains("/")?name:"common/"+name;
                var model=new GltfModel(RuntimeConfig.gameAssets(game).resolve("levels").resolve(path),"item_"+game+"_"+name.replace('/','_').replace(".glb",""));
                if(!damageTable.isEmpty())HellcatRenderer.damageStages(model,damageTable);
                return model;
            }
            catch(java.io.IOException e){throw new IllegalStateException("Original item model missing: "+name,e);}
        });}
    private GltfModel.Posed pose(GltfModel model){if(fixedPose==null){fixedPose=clip.isEmpty()?model.pose("",0):model.poseAtTime(clip,Float.MAX_VALUE);model.prepare(fixedPose);}return fixedPose;}
    @Override public void getExtents(Consumer<Vector3fc> out){
        if(fitGui){out.accept(new Vector3f());out.accept(new Vector3f(1,1,1));return;}
        GltfModel model=model();if(extents==null)extents=model.bounds(pose(model),new org.joml.Matrix4f().translation(.5f,.25f,.5f).scale(scale),mask());var bounds=extents;
        for(float x:new float[]{bounds[0].x,bounds[1].x})for(float y:new float[]{bounds[0].y,bounds[1].y})for(float z:new float[]{bounds[0].z,bounds[1].z})out.accept(new Vector3f(x,y,z));
    }
    @Override public void submit(Void argument,PoseStack stack,SubmitNodeCollector collector,int light,int overlay,boolean foil,int outline){
        GltfModel model=model();stack.pushPose();if(fitGui){
            if(extents==null)extents=model.bounds(pose(model),new org.joml.Matrix4f());
            var low=extents[0];var high=extents[1];float fit=.78f/Math.max(high.x-low.x,Math.max(high.y-low.y,high.z-low.z));
            stack.translate(.5f,.5f,.5f);stack.scale(fit,fit,fit);stack.translate(-(low.x+high.x)/2,-(low.y+high.y)/2,-(low.z+high.z)/2);
        }else{stack.translate(.5f,.25f,.5f);stack.scale(scale,scale,scale);}model.submit(pose(model),stack,collector,light,1,overlay,mask());stack.popPose();
    }
    private long mask(){return damageTable.isEmpty()?-1L:1L;}
    public record Unbaked(String file,float scale,String clip,String game,String damageTable,boolean fitGui) implements SpecialModelRenderer.Unbaked<Void> {
        public static final MapCodec<Unbaked> CODEC=RecordCodecBuilder.mapCodec(i->i.group(Codec.STRING.fieldOf("file").forGetter(Unbaked::file),Codec.FLOAT.optionalFieldOf("scale",1f).forGetter(Unbaked::scale),Codec.STRING.optionalFieldOf("clip","").forGetter(Unbaked::clip),Codec.STRING.optionalFieldOf("game","jak3").forGetter(Unbaked::game),Codec.STRING.optionalFieldOf("damage_table","").forGetter(Unbaked::damageTable),Codec.BOOL.optionalFieldOf("fit_gui",false).forGetter(Unbaked::fitGui)).apply(i,Unbaked::new));
        public MapCodec<Unbaked> type(){return CODEC;}
        public SpecialModelRenderer<Void> bake(SpecialModelRenderer.BakingContext context){var renderer=new GltfItemRenderer(file,scale,clip,game,damageTable);renderer.fitGui=fitGui;return renderer;}
    }
}
