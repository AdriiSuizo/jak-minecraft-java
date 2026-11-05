package dev.jakmc.client.model;

import com.google.gson.*;
import com.mojang.blaze3d.platform.NativeImage;
import com.mojang.blaze3d.vertex.PoseStack;
import dev.jakmc.JakData;
import java.io.*;
import java.nio.*;
import java.nio.file.*;
import java.util.*;
import net.minecraft.client.Minecraft;
import net.minecraft.client.renderer.SubmitNodeCollector;
import net.minecraft.client.renderer.rendertype.RenderTypes;
import net.minecraft.client.renderer.texture.*;
import net.minecraft.resources.Identifier;
import org.joml.Vector3f;
import org.joml.Matrix4f;
import org.joml.Quaternionf;

/** glTF 2 skin/animation reader. Uses Minecraft's own render submissions, not raw GL. */
public final class GltfModel {
    // GOAL Rig::globals strips parent scale; the board explicitly inherits it.
    private final boolean inheritScale;
    private final boolean compensateScale;
    private final boolean cinematic;
    private final boolean equipment;
    private static final ThreadLocal<float[]> CAMERA_VERTICES=ThreadLocal.withInitial(()->new float[0]);
    private dev.jakmc.nativebridge.ModelSegments segments;
    private final Map<Identifier,Identifier> darkTextures=new HashMap<>(),lightTextures=new HashMap<>();
    private boolean jakBody;
    private final ForwardEyes.Pair eyes;
    private final Map<Identifier,byte[]> pendingTextures=new LinkedHashMap<>();
    private final Map<Identifier,Integer> textureCoverage=new HashMap<>();
    private final JsonObject json;
    private final ByteBuffer bin;
    private final Node[] nodes;
    private final int[] joints;
    private final Matrix4f[] inverseBind;
    private final List<Surface> surfaces=new ArrayList<>();
    private List<Surface> armSurfaces;
    private final Map<String,Clip> clips=new HashMap<>();
    private final Map<String,Matrix4f[]> prejoints=new HashMap<>();
    private final Map<String,Integer> namedNodes=new HashMap<>();
    private final Map<Integer,float[]> accessors=new HashMap<>();
    private static final int[] QUAD_CORNERS={0,1,2,2};
    private final Map<Posed,List<float[]>> prepared=new IdentityHashMap<>();
    private final Map<String,Posed> terminalPoses=new HashMap<>();
    public long skinnedVertices;
    private float[] defaultMorphs=new float[0];
    private record Node(Vector3f t,Quaternionf r,Vector3f s,int parent,String name){}
    private record TextureBinding(int texture,int coverage){}
    private static Identifier gemAtlas;
    private static Identifier gemAtlas(){if(gemAtlas!=null)return gemAtlas;try{int[] pixels=SkullGemTexture.bake(dev.jakmc.RuntimeConfig.assets.resolve("textures/level-default-tfrag"));var image=new NativeImage(64,64*60,false);for(int y=0;y<64*60;y++)for(int x=0;x<64;x++)image.setPixel(x,y,pixels[y*64+x]);gemAtlas=JakData.id("local/skull_gem_animated");Minecraft.getInstance().getTextureManager().register(gemAtlas,new DynamicTexture(()->"Original skull gem layers",image));return gemAtlas;}catch(IOException e){throw new UncheckedIOException(e);}}
    public static float materialSeconds(){var c=Minecraft.getInstance();return c.level==null?0:(c.level.getGameTime()+c.getDeltaTracker().getGameTimeDeltaPartialTick(false))/20f;}
    private record Surface(float[] positions,float[] normals,float[] uv,float[] colors,float[] weights,int[] joints,int[] indices,Identifier texture,String material,boolean translucent,boolean skullGem,float[][] morphPositions,float[][] morphNormals){}
    private record Channel(int node,String path,float[] times,float[] values,int components,boolean step){}
    private record Clip(List<Channel> channels,float duration,int frames){
        Clip(List<Channel> channels,float duration){this(channels,duration,channels.stream().mapToInt(c->c.times.length).max().orElse(1));}
    }
    record LocalPose(Vector3f[] translations,Quaternionf[] rotations,Vector3f[] scales,float[] morphs){}
    public record Posed(Matrix4f[] globals,Matrix4f[] skin,float[] morphs,Vector3f[] translations,Quaternionf[] rotations,Vector3f[] scales){}

    public GltfModel(Path file,String key)throws IOException {
        this(file,key,false);
    }
    public GltfModel(Path file,String key,boolean compensateScale)throws IOException {
        this.compensateScale=compensateScale;
        cinematic=key.endsWith("_cinematic");
        equipment=file.getFileName().toString().equals("gun-lod0.glb")||file.getFileName().toString().startsWith("gun-ammo-");
        boolean jakTextures=key.equals("jak")||key.equals("jak_cinematic")||key.equals("jak_dark_cinematic");
        boolean forwardEyes=jakTextures||key.equals("daxter")||key.equals("daxter_cinematic");
        Path artTextures=forwardEyes?file.getParent().getParent().getParent().resolve("textures"):null;
        Path darkPalette=jakTextures?artTextures.resolve(key.equals("jak_dark_cinematic")?"mhctycst-pris":"level-default-pris"):null;
        inheritScale=file.getFileName().toString().startsWith("board-")||file.getFileName().toString().equals("gun-lod0.glb");
        byte[] bytes=Files.readAllBytes(file);ByteBuffer data=ByteBuffer.wrap(bytes).order(ByteOrder.LITTLE_ENDIAN);
        if(data.getInt()!=0x46546c67||data.getInt()!=2||data.getInt()!=bytes.length)throw new IOException("Invalid GLB "+file);
        int jsonLength=data.getInt();if(data.getInt()!=0x4e4f534a)throw new IOException("Missing glTF JSON");
        json=JsonParser.parseString(new String(bytes,20,jsonLength,java.nio.charset.StandardCharsets.UTF_8)).getAsJsonObject();
        int binaryAt=20+jsonLength;int binaryLength=data.getInt(binaryAt);
        if(data.getInt(binaryAt+4)!=0x004e4942||binaryAt+8L+binaryLength>bytes.length)throw new IOException("Missing GLB binary");
        bin=ByteBuffer.wrap(bytes,binaryAt+8,binaryLength).slice().order(ByteOrder.LITTLE_ENDIAN);
        JsonArray ns=json.getAsJsonArray("nodes");nodes=new Node[ns.size()];int[] parents=new int[nodes.length];Arrays.fill(parents,-1);
        for(int i=0;i<ns.size();i++){var n=ns.get(i).getAsJsonObject();if(n.has("children"))for(var child:n.getAsJsonArray("children"))parents[child.getAsInt()]=i;}
        for(int i=0;i<ns.size();i++){
            var n=ns.get(i).getAsJsonObject();Vector3f t=new Vector3f(),s=new Vector3f(1);Quaternionf r=new Quaternionf();
            if(n.has("matrix")){new Matrix4f().set(array(n.getAsJsonArray("matrix"))).getTranslation(t);new Matrix4f().set(array(n.getAsJsonArray("matrix"))).getScale(s);new Matrix4f().set(array(n.getAsJsonArray("matrix"))).getUnnormalizedRotation(r);}
            else {if(n.has("translation"))t.set(array(n.getAsJsonArray("translation")));if(n.has("rotation"))r.set(array(n.getAsJsonArray("rotation"))[0],array(n.getAsJsonArray("rotation"))[1],array(n.getAsJsonArray("rotation"))[2],array(n.getAsJsonArray("rotation"))[3]);if(n.has("scale"))s.set(array(n.getAsJsonArray("scale")));}
            String name=n.has("name")?n.get("name").getAsString():"node_"+i;
            nodes[i]=new Node(t,r,s,parents[i],name);namedNodes.put(name,i);
        }
        if(json.has("skins")){
            var skin=json.getAsJsonArray("skins").get(0).getAsJsonObject();joints=ints(skin.getAsJsonArray("joints"));
            float[] matrices=accessor(skin.get("inverseBindMatrices").getAsInt());inverseBind=new Matrix4f[joints.length];
            for(int i=0;i<joints.length;i++)inverseBind[i]=new Matrix4f().set(Arrays.copyOfRange(matrices,i*16,i*16+16));
        } else {joints=new int[]{0};inverseBind=new Matrix4f[]{new Matrix4f()};}
        var textures=new HashMap<TextureBinding,Identifier>();
        if(json.has("meshes"))for(var mesh:json.getAsJsonArray("meshes"))for(var primitive:mesh.getAsJsonObject().getAsJsonArray("primitives")){
            if(mesh.getAsJsonObject().has("weights"))defaultMorphs=array(mesh.getAsJsonObject().getAsJsonArray("weights"));
            var p=primitive.getAsJsonObject();if(p.has("mode")&&p.get("mode").getAsInt()!=4)throw new IOException("Unsupported non-triangle primitive");
            var at=p.getAsJsonObject("attributes");float[] positions=accessor(at.get("POSITION").getAsInt());int count=positions.length/3;
            float[] normals=at.has("NORMAL")?accessor(at.get("NORMAL").getAsInt()):filled(count*3,0);
            float[] uv=at.has("TEXCOORD_0")?accessor(at.get("TEXCOORD_0").getAsInt()):new float[count*2];
            float[] color=at.has("COLOR_0")?accessor(at.get("COLOR_0").getAsInt()):filled(count*4,1);
            float[] weight=at.has("WEIGHTS_0")?accessor(at.get("WEIGHTS_0").getAsInt()):null;
            int[] bone=at.has("JOINTS_0")?toInts(accessor(at.get("JOINTS_0").getAsInt())):null;
            int[] indices=p.has("indices")?toInts(accessor(p.get("indices").getAsInt())):java.util.stream.IntStream.range(0,count).toArray();
            var material=json.getAsJsonArray("materials").get(p.get("material").getAsInt()).getAsJsonObject();
            // Predator's second draw-mask is a refraction hull, not its skin.
            if(compensateScale&&file.getFileName().toString().startsWith("predator-")&&material.has("alphaMode")&&material.get("alphaMode").getAsString().equals("BLEND")&&material.has("name")&&Set.of("common-white","common-white-envmap").contains(material.get("name").getAsString()))continue;
            var pbr=material.getAsJsonObject("pbrMetallicRoughness");
            float[] factor=pbr.has("baseColorFactor")?array(pbr.getAsJsonArray("baseColorFactor")):new float[]{1,1,1,1};
            color=color.clone();for(int i=0;i<color.length;i++)color[i]*=factor[i%4];
            if(equipment&&material.has("extensions")&&material.getAsJsonObject("extensions").has("KHR_materials_specular")){
                var spec=material.getAsJsonObject("extensions").getAsJsonObject("KHR_materials_specular");
                if(spec.has("specularColorTexture")){
                    int tex=spec.getAsJsonObject("specularColorTexture").get("index").getAsInt();
                    int source=json.getAsJsonArray("textures").get(tex).getAsJsonObject().get("source").getAsInt();
                    var image=json.getAsJsonArray("images").get(source).getAsJsonObject();byte[] reflectionBytes;
                    if(image.has("uri"))reflectionBytes=Base64.getDecoder().decode(image.get("uri").getAsString().split(",",2)[1]);
                    else{var view=json.getAsJsonArray("bufferViews").get(image.get("bufferView").getAsInt()).getAsJsonObject();int offset=intOr(view,"byteOffset",0);reflectionBytes=new byte[view.get("byteLength").getAsInt()];bin.get(offset,reflectionBytes);}
                    try(var reflection=NativeImage.read(new ByteArrayInputStream(reflectionBytes))){
                        double[] rgb=new double[3];for(int y=0;y<reflection.getHeight();y++)for(int x=0;x<reflection.getWidth();x++){
                            int pixel=reflection.getPixel(x,y);for(int c=0;c<3;c++)rgb[c]+=(pixel>>>(16-c*8))&255;
                        }
                        double max=Math.max(rgb[0],Math.max(rgb[1],rgb[2]));
                        if(max>0)for(int i=0;i<color.length;i++)if(i%4<3)color[i]*=(float)(rgb[i%4]/max);
                    }
                }
            }
            int texture=pbr.getAsJsonObject("baseColorTexture").get("index").getAsInt();
            String mode=material.has("alphaMode")?material.get("alphaMode").getAsString():"OPAQUE";
            int coverage=(!cinematic&&!equipment)||mode.equals("BLEND")?-1:mode.equals("MASK")
                ?Math.round(255*(material.has("alphaCutoff")?material.get("alphaCutoff").getAsFloat():.5f)):0;
            var binding=new TextureBinding(texture,coverage);
            Identifier id=textures.get(binding);
            if(id==null){
                int source=json.getAsJsonArray("textures").get(texture).getAsJsonObject().get("source").getAsInt();
                var image=json.getAsJsonArray("images").get(source).getAsJsonObject();byte[] png;
                if(image.has("uri")){String uri=image.get("uri").getAsString();if(!uri.startsWith("data:"))throw new IOException("External texture requires asset resolver: "+uri);png=Base64.getDecoder().decode(uri.substring(uri.indexOf(',')+1));}
                else {var view=json.getAsJsonArray("bufferViews").get(image.get("bufferView").getAsInt()).getAsJsonObject();int off=intOr(view,"byteOffset",0);png=new byte[view.get("byteLength").getAsInt()];bin.get(off,png);}
                id=JakData.id("local/"+key+"/"+source+(cinematic?"/a"+(coverage+1):""));
                pendingTextures.put(id,png);textures.put(binding,id);
                if(coverage>=0)textureCoverage.put(id,coverage);
                if(jakTextures){
                    jakBody=true;
                    String name=material.has("name")?material.get("name").getAsString().replaceFirst("-envmap$",""):"";
                    var dark=darkPalette.resolve(name+"-dark.png");
                    if(Files.isRegularFile(dark)){
                        var variant=JakData.id(id.getPath()+"/dark");
                        pendingTextures.put(variant,Files.readAllBytes(dark));
                        if(coverage>=0)textureCoverage.put(variant,coverage);
                        darkTextures.put(id,variant);
                    }
                    lightTextures.put(id,JakData.id(id.getPath()+"/light"));
                }
            }
            int targetCount=p.has("targets")?p.getAsJsonArray("targets").size():0;
            float[][] morphPositions=new float[targetCount][],morphNormals=new float[targetCount][];
            for(int t=0;t<targetCount;t++){var target=p.getAsJsonArray("targets").get(t).getAsJsonObject();if(target.has("POSITION"))morphPositions[t]=accessor(target.get("POSITION").getAsInt());if(target.has("NORMAL"))morphNormals[t]=accessor(target.get("NORMAL").getAsInt());}
            // OpenGOAL exports share accessors across different models in an art group.
            // Only this primitive's indexed vertices belong to this skeleton.
            var remap=new LinkedHashMap<Integer,Integer>();for(int index:indices)remap.computeIfAbsent(index,_ -> remap.size());
            int[] used=remap.keySet().stream().mapToInt(Integer::intValue).toArray();
            int[] compactIndices=Arrays.stream(indices).map(remap::get).toArray();
            for(int m=0;m<targetCount;m++){if(morphPositions[m]!=null)morphPositions[m]=select(morphPositions[m],3,used);if(morphNormals[m]!=null)morphNormals[m]=select(morphNormals[m],3,used);}
            int[] compactBones=bone==null?null:toInts(select(toFloats(bone),4,used));
            if(compactBones!=null)for(int vertex=0;vertex<used.length;vertex++)for(int k=0;k<4;k++)if(weight[used[vertex]*4+k]>0&&(compactBones[vertex*4+k]<0||compactBones[vertex*4+k]>=joints.length))throw new IOException("Indexed vertex uses missing skin joint in "+file);
            surfaces.add(new Surface(select(positions,3,used),select(normals,3,used),select(uv,2,used),select(color,4,used),weight==null?null:select(weight,4,used),compactBones,compactIndices,id,material.has("name")?material.get("name").getAsString():"",material.has("alphaMode")&&material.get("alphaMode").getAsString().equals("BLEND"),material.has("name")&&material.get("name").getAsString().startsWith("skull-gem-dest"),morphPositions,morphNormals));
        }
        if(forwardEyes){
            eyes=ForwardEyes.build(artTextures,key,surfaces.stream().map(
                surface->new ForwardEyes.Mesh(surface.positions,surface.uv,surface.indices,surface.material)).toList());
        }else eyes=null;
        readAnimations(this,null);
        String originalName=file.getFileName().toString().replace("-lod0.glb","");
        if(Set.of("board","gun").contains(originalName)){
            Path export=file.getParent().getParent().getParent();
            var art=JsonParser.parseString(dev.jakmc.nativebridge.NativeJak.animationMetadata(export.toString(),originalName,false)).getAsJsonObject();
            for(var entry:art.entrySet()){
                readPrejoint(entry.getKey(),entry.getValue().getAsJsonObject());
            }
        }
        if(compensateScale&&Set.of("grunt","flitter","mantis").contains(originalName)){
            Path raw=file.getParent().getParent().getParent().resolve("raw_obj/"+originalName+"-ag.go");
            segments=dev.jakmc.nativebridge.ModelSegments.read(raw,originalName+"-lod0",surfaces.stream().mapToInt(s->s.indices.length/3).toArray());
            if(segments==null)throw new IOException("Original segment table did not match GLB: "+originalName);
        }
    }

    /** Matches jak_model.rs::onto_skeleton. This mesh will be drawn with the target pose. */
    void wearOn(GltfModel target){
        var targetBones=new HashMap<String,Integer>();
        for(int i=0;i<target.joints.length;i++)targetBones.put(target.nodes[target.joints[i]].name,i);
        int[] onto=new int[joints.length];
        for(int i=0;i<joints.length;i++){
            for(int node=joints[i];node>=0;node=nodes[node].parent){
                Integer match=targetBones.get(nodes[node].name);
                if(match!=null){onto[i]=match;break;}
            }
        }
        for(var surface:surfaces)if(surface.joints!=null){
            for(int i=0;i<surface.joints.length;i++){
                if(surface.weights[i]==0)continue;
                surface.joints[i]=onto[surface.joints[i]];
                for(int before=i&~3;before<i;before++){
                    if(surface.weights[before]==0||surface.joints[before]!=surface.joints[i])continue;
                    surface.weights[before]+=surface.weights[i];
                    surface.weights[i]=0;
                    break;
                }
            }
        }
    }

    int triangles(){return surfaces.stream().mapToInt(s->s.indices.length/3).sum();}
    public void loadSegments(Path art,String lod)throws IOException{segments=dev.jakmc.nativebridge.ModelSegments.read(art,lod,surfaces.stream().mapToInt(s->s.indices.length/3).toArray());if(segments==null)throw new IOException("Source segment table mismatch: "+lod);}
    void vehicleDamageStages(int[] stages)throws IOException{
        if(stages.length!=surfaces.size())throw new IOException("HellCat stage data and mesh do not match");
        segments=new dev.jakmc.nativebridge.ModelSegments(stages,new long[]{1,2,4});
    }
    public void prepare(Posed pose){prepared.computeIfAbsent(pose,p->{var geometry=new ArrayList<float[]>();for(Surface surface:surfaces)geometry.add(skin(surface,p));return List.copyOf(geometry);});}
    public void animationsFrom(GltfModel source){readAnimations(source,null);}
    public void attachmentAnimationsFrom(GltfModel source,String prefix){readAnimations(source,prefix);}
    private void readPrejoint(String clip,JsonObject art){
        if(!(art.get("prejoint") instanceof JsonArray rows))return;
        Matrix4f[] matrices=new Matrix4f[rows.size()];
        for(int i=0;i<matrices.length;i++)matrices[i]=new Matrix4f().set(array(rows.get(i).getAsJsonArray()));
        prejoints.put(clip,matrices);
    }

    void originalAnimations(JsonObject art,String prefix){
        for(var entry:art.entrySet()){
            String name=entry.getKey();
            if(!name.startsWith(prefix))continue;
            var clip=entry.getValue().getAsJsonObject();
            var frames=clip.getAsJsonArray("joints");
            if(frames.isEmpty()||frames.get(0).getAsJsonArray().size()!=joints.length-2)continue;
            float[] times=new float[frames.size()];
            float rate=clip.get("speed").getAsFloat()*60;
            for(int i=0;i<times.length;i++)times[i]=i/rate;
            var channels=new ArrayList<Channel>();
            for(int joint=2;joint<joints.length;joint++){
                float[] translations=new float[times.length*3],rotations=new float[times.length*4],scales=new float[times.length*3];
                for(int frame=0;frame<times.length;frame++){
                    var pose=frames.get(frame).getAsJsonArray().get(joint-2).getAsJsonArray();
                    for(int c=0;c<3;c++){
                        translations[frame*3+c]=pose.get(c).getAsFloat();
                        scales[frame*3+c]=pose.get(c+7).getAsFloat();
                    }
                    for(int c=0;c<4;c++)rotations[frame*4+c]=pose.get(c+3).getAsFloat();
                }
                int node=joints[joint];
                channels.add(new Channel(node,"translation",times,translations,3,false));
                channels.add(new Channel(node,"rotation",times,rotations,4,false));
                channels.add(new Channel(node,"scale",times,scales,3,false));
            }
            clips.put(name,new Clip(channels,times[times.length-1]));
            readPrejoint(name,clip);
        }
    }
    private void readAnimations(GltfModel source,String attachmentPrefix){
        if(!source.json.has("animations"))return;
        for(var element:source.json.getAsJsonArray("animations")){
            var a=element.getAsJsonObject();String name=a.get("name").getAsString();
            if(attachmentPrefix!=null&&!name.startsWith(attachmentPrefix))continue;
            var channels=new ArrayList<Channel>();float duration=0;
            for(var ce:a.getAsJsonArray("channels")){
                var c=ce.getAsJsonObject();var target=c.getAsJsonObject("target");int sourceNode=target.get("node").getAsInt();// The OpenGOAL combined export stores attachment channels in the body's node slots.
                Integer node=attachmentPrefix==null?namedNodes.get(source.nodes[sourceNode].name):null;
                if(attachmentPrefix!=null&&sourceNode>=0&&sourceNode<nodes.length)node=sourceNode;
                if(node==null)continue;
                var sampler=a.getAsJsonArray("samplers").get(c.get("sampler").getAsInt()).getAsJsonObject();
                float[] times=source.accessor(sampler.get("input").getAsInt()),values=source.accessor(sampler.get("output").getAsInt());
                String path=target.get("path").getAsString();int components=path.equals("weights")?values.length/times.length:path.equals("rotation")?4:3;
                if(values.length!=times.length*components)throw new IllegalArgumentException("Unsupported cubic/sparse animation: "+a.get("name"));
                channels.add(new Channel(node,path,times,values,components,sampler.has("interpolation")&&sampler.get("interpolation").getAsString().equals("STEP")));
                if(times.length>0)duration=Math.max(duration,times[times.length-1]);
            }
            clips.put(name,new Clip(channels,duration));
        }
    }
    public Posed pose(String name,float frame){return pose(name,frame,false);}
    public Posed pose(String name,float frame,boolean preserveAlign){return resolve(localPose(name,frame,preserveAlign));}
    LocalPose localPose(String name,float frame){return localPose(name,frame,false);}
    private LocalPose localPose(String name,float frame,boolean preserveAlign){
        Clip clip=clips.get(name);Vector3f[] t=new Vector3f[nodes.length],s=new Vector3f[nodes.length];Quaternionf[] r=new Quaternionf[nodes.length];float[] morphs=defaultMorphs.clone();
        for(int i=0;i<nodes.length;i++){t[i]=new Vector3f(nodes[i].t);s[i]=new Vector3f(nodes[i].s);r[i]=new Quaternionf(nodes[i].r);}
        if(clip!=null)for(Channel c:clip.channels){
            // Exported channels sample one authored frame each. Gameplay supplies that frame.
            int left=Math.min(Math.max(0,(int)frame),c.times.length-1),right=Math.min(left+1,c.times.length-1);float mix=c.step?0:Math.clamp(frame-left,0,1);
            if(c.path.equals("weights")){
                if(morphs.length!=c.components)morphs=new float[c.components];
                for(int k=0;k<morphs.length;k++)morphs[k]=c.values[left*c.components+k]*(1-mix)+c.values[right*c.components+k]*mix;
            }else if(c.path.equals("rotation")){
                Quaternionf a=new Quaternionf(c.values[left*4],c.values[left*4+1],c.values[left*4+2],c.values[left*4+3]);
                Quaternionf b=new Quaternionf(c.values[right*4],c.values[right*4+1],c.values[right*4+2],c.values[right*4+3]);r[c.node].set(a).slerp(b,mix).normalize();
            } else {var dest=c.path.equals("translation")?t[c.node]:s[c.node];int x=left*3,y=right*3;dest.set(c.values[x],c.values[x+1],c.values[x+2]).lerp(new Vector3f(c.values[y],c.values[y+1],c.values[y+2]),mix);}
        }
        var prejoint=prejoints.get(name);Integer preNode=namedNodes.get("prejoint");
        if(prejoint!=null&&preNode!=null&&prejoint.length>0){
            float at=Math.clamp(frame,0,prejoint.length-1);int lo=(int)at,hi=Math.min(lo+1,prejoint.length-1);float w=at-lo;Matrix4f a=prejoint[lo],b=prejoint[hi];
            if(Math.abs(a.determinant())>1e-12&&Math.abs(b.determinant())>1e-12){
                t[preNode].set(a.getTranslation(new Vector3f()).lerp(b.getTranslation(new Vector3f()),w)).div(4096);
                r[preNode].set(a.getUnnormalizedRotation(new Quaternionf())).slerp(b.getUnnormalizedRotation(new Quaternionf()),w).normalize();
                s[preNode].set(a.getScale(new Vector3f()).lerp(b.getScale(new Vector3f()),w));
            }
        }
        if(!preserveAlign&&compensateScale&&namedNodes.containsKey("align")){int i=namedNodes.get("align");t[i].set(nodes[i].t);r[i].set(nodes[i].r);s[i].set(nodes[i].s);}
        return new LocalPose(t,r,s,morphs);
    }
    Posed resolve(LocalPose pose){
        return compose(pose.translations,pose.rotations,pose.scales,pose.morphs);
    }
    private Matrix4f global(int i,Matrix4f[] g,Vector3f[] t,Quaternionf[] r,Vector3f[] s){
        if(g[i]!=null)return g[i];var local=new Matrix4f().translationRotateScale(t[i],r[i],s[i]);
        if(nodes[i].parent<0)return g[i]=local;
        Matrix4f parent=global(nodes[i].parent,g,t,r,s);
        if(inheritScale)return g[i]=new Matrix4f(parent).mul(local);
        Quaternionf rotation=parent.getUnnormalizedRotation(new Quaternionf());
        if(!Float.isFinite(rotation.x)||!Float.isFinite(rotation.y)||!Float.isFinite(rotation.z)||!Float.isFinite(rotation.w))rotation.identity();
        if(compensateScale)return g[i]=new Matrix4f().translationRotateScale(parent.transformPosition(new Vector3f(t[i])),rotation.mul(r[i]),s[i]);
        return g[i]=new Matrix4f().translationRotate(parent.getTranslation(new Vector3f()),rotation).mul(local);
    }
    void retargetClips(GltfModel source,Posed sourceRest,Posed targetRest,String prefix){
        for(var entry:source.clips.entrySet()){
            String name=entry.getKey();
            if(!name.startsWith(prefix))continue;
            Clip original=entry.getValue();
            Posed[] poses=new Posed[original.frames];
            float[] times=new float[poses.length];
            for(int f=0;f<poses.length;f++){
                poses[f]=retargetRider(source,sourceRest,source.pose(name,f),targetRest);
                times[f]=f*original.duration/Math.max(1,poses.length-1);
            }
            var channels=new ArrayList<Channel>();
            for(int j=2;j<joints.length;j++){
                int node=joints[j];
                float[] translations=new float[poses.length*3], rotations=new float[poses.length*4];
                float[] scales=new float[poses.length*3];
                for(int f=0;f<poses.length;f++){
                    var pose=poses[f];
                    var t=pose.translations[node];
                    var q=pose.rotations[node];
                    var size=pose.scales[node];
                    int a=f*3,b=f*4;
                    translations[a]=t.x;translations[a+1]=t.y;translations[a+2]=t.z;
                    rotations[b]=q.x;rotations[b+1]=q.y;rotations[b+2]=q.z;rotations[b+3]=q.w;
                    scales[a]=size.x;scales[a+1]=size.y;scales[a+2]=size.z;
                }
                channels.add(new Channel(node,"translation",times,translations,3,false));
                channels.add(new Channel(node,"rotation",times,rotations,4,false));
                channels.add(new Channel(node,"scale",times,scales,3,false));
            }
            clips.put(name,new Clip(channels,original.duration,original.frames));
        }
    }

    /** Transfer a bone's model-space rotation delta from the source standing pose. */
    Posed retargetRider(GltfModel source,Posed sourceRest,Posed motion,Posed targetRest){
        Vector3f[] t=new Vector3f[nodes.length],s=new Vector3f[nodes.length];
        Quaternionf[] r=new Quaternionf[nodes.length],world=new Quaternionf[nodes.length];
        for(int i=0;i<nodes.length;i++){
            t[i]=new Vector3f(targetRest.translations[i]);s[i]=new Vector3f(targetRest.scales[i]);
            r[i]=new Quaternionf(targetRest.rotations[i]);
        }
        for(int j=2;j<joints.length;j++){
            int node=joints[j];
            Integer from=source.namedNodes.get(nodes[node].name);
            if(from==null)continue;
            if(nodes[node].name.equals("main")||nodes[node].name.equals("hips")){
                t[node].add(motion.translations[from]).sub(sourceRest.translations[from]);
            }
            var delta=motion.globals[from].getUnnormalizedRotation(new Quaternionf())
                .mul(sourceRest.globals[from].getUnnormalizedRotation(new Quaternionf()).invert());
            world[node]=delta.mul(targetRest.globals[node].getUnnormalizedRotation(new Quaternionf())).normalize();
        }
        for(int node:joints){
            if(world[node]==null)continue;
            int parent=nodes[node].parent;
            if(parent<0)r[node].set(world[node]);
            else r[node].set(retargetRotation(parent,world,r)).invert().mul(world[node]).normalize();
        }
        return compose(t,r,s,targetRest.morphs);
    }
    private Quaternionf retargetRotation(int node,Quaternionf[] world,Quaternionf[] local){
        if(world[node]!=null)return world[node];
        int parent=nodes[node].parent;
        return world[node]=parent<0?new Quaternionf(local[node])
            :new Quaternionf(retargetRotation(parent,world,local)).mul(local[node]);
    }
    Posed retargetBird(GltfModel source,Posed sourceRest,Posed motion,Posed rest){
        Matrix4f[] globals=rest.globals.clone(),skin=new Matrix4f[joints.length];
        for(int j=0;j<joints.length;j++){
            int node=joints[j];
            Integer from=source.namedNodes.get(nodes[node].name);
            if(from!=null){
                var active=motion.globals[from];
                var rotation=active.getUnnormalizedRotation(new Quaternionf())
                    .mul(sourceRest.globals[from].getUnnormalizedRotation(new Quaternionf()).invert())
                    .mul(rest.globals[node].getUnnormalizedRotation(new Quaternionf())).normalize();
                globals[node]=new Matrix4f().translationRotateScale(active.getTranslation(new Vector3f()),
                    rotation,active.getScale(new Vector3f()));
            }
            skin[j]=new Matrix4f(globals[node]).mul(inverseBind[j]);
        }
        return new Posed(globals,skin,rest.morphs,rest.translations,rest.rotations,rest.scales);
    }
    private Posed compose(Vector3f[] t,Quaternionf[] r,Vector3f[] s,float[] morphs){
        Matrix4f[] globals=new Matrix4f[nodes.length],skin=new Matrix4f[joints.length];
        for(int i=0;i<nodes.length;i++)global(i,globals,t,r,s);
        for(int i=0;i<joints.length;i++)skin[i]=new Matrix4f(globals[joints[i]]).mul(inverseBind[i]);
        return new Posed(globals,skin,morphs,t,r,s);
    }
    public Posed blend(Posed from,Posed to,float weight){
        weight=Math.clamp(weight,0,1);Vector3f[] t=new Vector3f[nodes.length],s=new Vector3f[nodes.length];Quaternionf[] r=new Quaternionf[nodes.length];
        for(int i=0;i<nodes.length;i++){t[i]=new Vector3f(from.translations[i]).lerp(to.translations[i],weight);s[i]=new Vector3f(from.scales[i]).lerp(to.scales[i],weight);r[i]=new Quaternionf(from.rotations[i]).slerp(to.rotations[i],weight);}
        Matrix4f[] globals=new Matrix4f[nodes.length];for(int i=0;i<nodes.length;i++)global(i,globals,t,r,s);Matrix4f[] skin=new Matrix4f[joints.length];for(int i=0;i<skin.length;i++)skin[i]=new Matrix4f(globals[joints[i]]).mul(inverseBind[i]);
        return new Posed(globals,skin,to.morphs,t,r,s);
    }
    void resetAlignment(LocalPose pose){
        resetJoint(pose,namedNodes.get("align"));
        resetJoint(pose,namedNodes.get("prejoint"));
    }
    private void resetJoint(LocalPose pose,Integer index){
        if(index==null)return;
        Node rest=nodes[index];
        pose.translations[index].set(rest.t);
        pose.rotations[index].set(rest.r);
        pose.scales[index].set(rest.s);
    }

    // These samples belong to the current mixer. Compose global and skin matrices once.
    LocalPose blendLocal(LocalPose from,LocalPose to,float weight){
        for(int i=0;i<nodes.length;i++){
            from.translations[i].lerp(to.translations[i],weight);
            from.scales[i].lerp(to.scales[i],weight);
            from.rotations[i].slerp(to.rotations[i],weight);
        }
        return new LocalPose(from.translations,from.rotations,from.scales,to.morphs);
    }

    Posed attachJoint(Posed pose,String joint,String anchor,Vector3f offset){
        int node=namedNodes.get(joint),parent=nodes[node].parent;
        var at=new Matrix4f(pose.globals[parent]).invert().mul(pose.globals[namedNodes.get(anchor)]).translate(offset);
        var translations=pose.translations.clone();
        var rotations=pose.rotations.clone();
        var scales=pose.scales.clone();
        translations[node]=at.getTranslation(new Vector3f());
        rotations[node]=at.getUnnormalizedRotation(new Quaternionf()).normalize();
        scales[node]=at.getScale(new Vector3f());
        return compose(translations,rotations,scales,pose.morphs);
    }
    Posed articulate(Posed rest,Map<String,Quaternionf> localTurns,Map<String,Vector3f> localTranslations){
        Quaternionf[] turns=rest.rotations.clone();
        Vector3f[] translations=rest.translations.clone();
        localTurns.forEach((name,delta)->{
            int joint=namedNodes.get(name);
            turns[joint]=new Quaternionf(turns[joint]).mul(delta);
        });
        localTranslations.forEach((name,position)->{
            int joint=namedNodes.get(name);
            translations[joint]=position;
        });
        return compose(translations,turns,rest.scales,rest.morphs);
    }
    boolean hasJoint(String name){return namedNodes.containsKey(name);}
    Vector3f localTranslation(Posed pose,String joint){return new Vector3f(pose.translations[namedNodes.get(joint)]);}
    public Matrix4f jointAt(Posed pose,int originalIndex){return pose.globals[joints[originalIndex]];}
    public Matrix4f joint(Posed pose,String name){Integer node=namedNodes.get(name);if(node==null)throw new IllegalArgumentException("Required original joint missing: "+name);return pose.globals[node];}
    public Posed poseAtTime(String name,float seconds){Clip clip=clips.get(name);if(clip==null)throw new IllegalArgumentException("Required original clip missing: "+name);if(seconds==Float.MAX_VALUE)return terminalPoses.computeIfAbsent(name,k->{Posed p=pose(k,frames(k)-1);prepare(p);return p;});int frames=clip.frames;return pose(name,clip.duration>0?Math.clamp(seconds/clip.duration,0,1)*(frames-1):0);}
    public Posed overlayUpper(Posed base,String name,float seconds){
        Posed upper=poseAtTime(name,seconds);Integer root=namedNodes.get("upper_body");if(root==null)throw new IllegalArgumentException("Original upper_body joint missing");
        for(int i=0;i<nodes.length;i++)for(int ancestor=i;ancestor>=0;ancestor=nodes[ancestor].parent)if(ancestor==root){base.translations[i].set(upper.translations[i]);base.rotations[i].set(upper.rotations[i]);base.scales[i].set(upper.scales[i]);break;}
        Matrix4f[] globals=new Matrix4f[nodes.length];for(int i=0;i<nodes.length;i++)global(i,globals,base.translations,base.rotations,base.scales);
        Matrix4f[] skin=new Matrix4f[joints.length];for(int i=0;i<skin.length;i++)skin[i]=new Matrix4f(globals[joints[i]]).mul(inverseBind[i]);
        return new Posed(globals,skin,base.morphs,base.translations,base.rotations,base.scales);
    }
    public Posed jakForm(Posed pose,boolean dark,float upperYaw){
        int horns=namedNodes.get("horns");
        pose.scales[horns].set(dark?1:0);
        if(dark){
            pose.scales[namedNodes.get("waist_prog")].mul(1.2f,1,1.2f);
            pose.scales[namedNodes.get("neckB")].mul(.95f/1.32f,.92f,.95f/1.32f);
        }
        if(upperYaw!=0){
            int upper=namedNodes.get("upper_body");
            var parent=pose.globals[nodes[upper].parent].getUnnormalizedRotation(new Quaternionf()).normalize();
            var twist=new Quaternionf(parent).conjugate().rotateY(upperYaw).mul(parent);
            pose.rotations[upper].premul(twist).normalize();
        }
        var globals=new Matrix4f[nodes.length];
        for(int i=0;i<nodes.length;i++)global(i,globals,pose.translations,pose.rotations,pose.scales);
        var skin=new Matrix4f[joints.length];
        for(int i=0;i<skin.length;i++)skin[i]=new Matrix4f(globals[joints[i]]).mul(inverseBind[i]);
        return new Posed(globals,skin,pose.morphs,pose.translations,pose.rotations,pose.scales);
    }
    public void submitJakBody(Posed pose,PoseStack stack,SubmitNodeCollector collector,int light,boolean dark,boolean luminous,boolean closedEyes){
        submitJakBody(pose,stack,collector,light,dark,luminous,closedEyes,surfaces);
    }
    private void submitJakBody(Posed pose,PoseStack stack,SubmitNodeCollector collector,int light,boolean dark,boolean luminous,boolean closedEyes,List<Surface> selected){
        Vector3f[] rim=null;
        if(luminous){
            var camera=Minecraft.getInstance().gameRenderer.mainCamera();
            var right=new Vector3f(camera.leftVector()).negate();
            var up=new Vector3f(camera.upVector());
            var forward=new Vector3f(camera.forwardVector());
            rim=new Vector3f[]{
                new Vector3f(right).mul(.77455187f).fma(-.44725248f,up).fma(.44725248f,forward).normalize(),
                new Vector3f(right).mul(-.77455187f).fma(-.44725248f,up).fma(.44725248f,forward).normalize(),
                new Vector3f(up).mul(.8944f).fma(.4472f,forward).normalize()
            };
        }
        submit(pose,stack,collector,light,1,OverlayTexture.NO_OVERLAY,-1L,dark,rim,null,null,closedEyes,selected);
    }
    /** Keep original skinned forearms/fingers; no face, torso or cutscene eyes near the camera. */
    int submitArms(GltfModel rig,Posed pose,PoseStack stack,SubmitNodeCollector collector,int light,boolean luminous){
        if(armSurfaces==null){
            boolean[] arm=new boolean[rig.joints.length];
            for(int j=0;j<arm.length;j++)for(int node=rig.joints[j];node>=0;node=rig.nodes[node].parent)
                if(Set.of("Lforarm","Rforarm").contains(rig.nodes[node].name)){arm[j]=true;break;}
            var result=new ArrayList<Surface>();
            for(var surface:surfaces){
                if(surface.weights==null)continue;
                boolean[] keep=new boolean[surface.positions.length/3];
                for(int v=0;v<keep.length;v++){
                    float weight=0,total=0;for(int k=0;k<4;k++){
                        float w=surface.weights[v*4+k];total+=w;
                        if(w>0&&arm[surface.joints[v*4+k]])weight+=w;
                    }
                    keep[v]=total>0&&weight/total>.5f;
                }
                var remap=new LinkedHashMap<Integer,Integer>();var indices=new ArrayList<Integer>();
                for(int i=0;i<surface.indices.length;i+=3){
                    int a=surface.indices[i],b=surface.indices[i+1],c=surface.indices[i+2];
                    if(!keep[a]||!keep[b]||!keep[c])continue;
                    for(int v:new int[]{a,b,c})indices.add(remap.computeIfAbsent(v,_ ->remap.size()));
                }
                if(indices.isEmpty())continue;
                int[] used=remap.keySet().stream().mapToInt(Integer::intValue).toArray();
                float[][] mp=new float[surface.morphPositions.length][],mn=new float[surface.morphNormals.length][];
                for(int i=0;i<mp.length;i++)if(surface.morphPositions[i]!=null)mp[i]=select(surface.morphPositions[i],3,used);
                for(int i=0;i<mn.length;i++)if(surface.morphNormals[i]!=null)mn[i]=select(surface.morphNormals[i],3,used);
                result.add(new Surface(select(surface.positions,3,used),select(surface.normals,3,used),select(surface.uv,2,used),
                    select(surface.colors,4,used),select(surface.weights,4,used),toInts(select(toFloats(surface.joints),4,used)),
                    indices.stream().mapToInt(Integer::intValue).toArray(),surface.texture,surface.material,surface.translucent,surface.skullGem,mp,mn));
            }
            armSurfaces=List.copyOf(result);
            if(armSurfaces.isEmpty())throw new IllegalStateException("Original cinematic arms could not be selected");
        }
        submitJakBody(pose,stack,collector,light,false,luminous,false,armSurfaces);
        return armSurfaces.stream().mapToInt(s->s.indices.length/3).sum();
    }
    public boolean hasClip(String name){return clips.containsKey(name);}
    public float duration(String name){Clip clip=clips.get(name);return clip==null?0:clip.duration;}
    public int frames(String name){Clip clip=clips.get(name);return clip==null?1:clip.frames;}
    public float height(Posed posed){
        Vector3f[] b=bounds(posed,new Matrix4f());return b[1].y-b[0].y;
    }
    public Vector3f[] bounds(Posed posed,Matrix4f root){return bounds(posed,root,-1L);}
    public Vector3f[] bounds(Posed posed,Matrix4f root,long shownMask){
        Vector3f low=new Vector3f(Float.POSITIVE_INFINITY),high=new Vector3f(Float.NEGATIVE_INFINITY),p=new Vector3f();var ready=prepared.get(posed);int index=0;
        for(Surface surface:surfaces){int s=index++;if(segments!=null&&!segments.shown(s,shownMask))continue;float[] geometry=ready==null?skin(surface,posed):ready.get(s);for(int i=0;i<geometry.length;i+=6){root.transformPosition(geometry[i],geometry[i+1],geometry[i+2],p);low.min(p);high.max(p);}}
        return new Vector3f[]{low,high};
    }
    public void submit(Posed posed,PoseStack stack,SubmitNodeCollector collector,int light){
        submit(posed,stack,collector,light,1);
    }
    public void submit(Posed posed,PoseStack stack,SubmitNodeCollector collector,int light,float alpha){
        submit(posed,stack,collector,light,alpha,OverlayTexture.NO_OVERLAY);
    }
    public void submit(Posed posed,PoseStack stack,SubmitNodeCollector collector,int light,float alpha,int overlay){
        submit(posed,stack,collector,light,alpha,overlay,true);
    }
    public void submit(Posed posed,PoseStack stack,SubmitNodeCollector collector,int light,float alpha,int overlay,boolean gem){submit(posed,stack,collector,light,alpha,overlay,~(gem?4L:2L));}
    public void submit(Posed posed,PoseStack stack,SubmitNodeCollector collector,int light,float alpha,int overlay,long shownMask){
        submit(posed,stack,collector,light,alpha,overlay,shownMask,false,null,null,null,false);
    }
    void submitLightEffect(Posed posed,PoseStack stack,SubmitNodeCollector collector,
                           LightFormTexture.Form effect,float fade){
        submit(posed,stack,collector,0xf000f0,fade,OverlayTexture.NO_OVERLAY,-1L,false,null,effect,null,false);
    }
    void submitEyes(Posed pose,PoseStack stack,SubmitNodeCollector collector,int light,boolean closed){
        submit(pose,stack,collector,light,1,OverlayTexture.NO_OVERLAY,-1L,false,null,null,null,closed);
    }

    void submitFlut(Posed pose,PoseStack stack,SubmitNodeCollector collector,int light,int overlay,FlutMaterial flut){
        submit(pose,stack,collector,light,1,overlay,-1L,false,null,null,flut,false);
    }
    private void submit(Posed posed,PoseStack stack,SubmitNodeCollector collector,int light,float alpha,int overlay,long shownMask,boolean dark,Vector3f[] rim,LightFormTexture.Form effect,FlutMaterial flut,boolean closedEyes){
        submit(posed,stack,collector,light,alpha,overlay,shownMask,dark,rim,effect,flut,closedEyes,surfaces);
    }
    private void submit(Posed posed,PoseStack stack,SubmitNodeCollector collector,int light,float alpha,int overlay,long shownMask,boolean dark,Vector3f[] rim,LightFormTexture.Form effect,FlutMaterial flut,boolean closedEyes,List<Surface> selected){
        if(!pendingTextures.isEmpty()){
            var manager=Minecraft.getInstance().getTextureManager();
            for(var entry:pendingTextures.entrySet()){
                var id=entry.getKey();
                try{
                    var texture=NativeImage.read(new ByteArrayInputStream(entry.getValue()));
                    Integer coverage=textureCoverage.get(id);
                    if(coverage!=null){
                        // OpenGOAL merc: opaque envmap alpha measures reflection strength.
                        // It only controls visibility on materials marked MASK or BLEND.
                        try(var raw=texture){
                            texture=raw.mappedCopy(pixel->(pixel&0x00ffffff)|((pixel>>>24)>=coverage?0xff000000:0));
                        }
                    }
                    if(jakBody&&lightTextures.containsKey(id)){
                        var white=texture.mappedCopy(pixel->pixel|0x00ffffff);
                        var lightId=lightTextures.get(id);
                        manager.register(lightId,new DynamicTexture(lightId::toString,white));
                    }
                    manager.register(id,new DynamicTexture(id::toString,texture));
                }catch(IOException e){throw new UncheckedIOException(e);}
            }
            pendingTextures.clear();
        }
        var hurt=JakHurtVisuals.current(); // Capture before deferred submission.
        int surfaceNumber=0;var ready=selected==surfaces?prepared.get(posed):null;
        float seconds=materialSeconds();
        int effectFrame=effect==null?0:effect.frame(seconds);
        var camera=Minecraft.getInstance().gameRenderer.mainCamera();
        var viewLeft=effect==LightFormTexture.Form.SHIELD?new Vector3f(camera.leftVector()):null;
        var viewUp=effect==LightFormTexture.Form.SHIELD?new Vector3f(camera.upVector()):null;
        for(Surface surface:selected){
            int index=surfaceNumber++;if(segments!=null&&!segments.shown(index,shownMask))continue;
            float[] skinned=ready==null?skin(surface,posed):ready.get(surfaceNumber-1);
            Identifier eye=null;
            if(surface.material.startsWith("programmer_eye")){
                if(flut!=null)eye=flut.eyes();
                else if(eyes!=null&&rim==null){
                    var channel=surface.material.contains("right")?eyes.right():eyes.left();
                    eye=closedEyes?channel.closed():channel.open();
                }
            }
            int tint=flut==null?0xffffff:flut.tint(surface.material);
            Identifier texture=eye!=null?eye:effect!=null?effect.texture():surface.skullGem?gemAtlas():rim!=null?lightTextures.get(surface.texture):
                dark?darkTextures.getOrDefault(surface.texture,surface.texture):surface.texture;
            int gemFrame=Math.floorMod((int)Math.floor(seconds*60),60);
            var renderType=effect!=null?dev.jakmc.client.JakEffectTypes.get(texture,3):rim!=null?RenderTypes.entityTranslucentEmissive(texture):surface.skullGem?GemMaterial.get(texture):surface.translucent||alpha<1?RenderTypes.entityTranslucent(texture):RenderTypes.entityCutout(texture);
            collector.submitCustomGeometry(stack,renderType,(pose,buffer)->{
                Vector3f v=new Vector3f(),normal=new Vector3f();
                float[] cameraVertices=cinematic?cameraVertices(pose,skinned,v,normal):null;
                for(int triangle=0;triangle<surface.indices.length;triangle+=3)for(int corner:QUAD_CORNERS){
                    int i=surface.indices[triangle+corner],at=i*6;
                    if(cameraVertices!=null){
                        v.set(cameraVertices,at);
                        normal.set(cameraVertices,at+3);
                    }else{
                        pose.pose().transformPosition(skinned[at],skinned[at+1],skinned[at+2],v);
                        pose.transformNormal(skinned[at+3],skinned[at+4],skinned[at+5],normal);
                    }
                    int color=Math.clamp(Math.round(surface.colors[i*4+3]*alpha*255),0,255)<<24;for(int c=0;c<3;c++)color|=Math.clamp((int)(surface.colors[i*4+c]*((tint>>>(16-c*8))&255)),0,255)<<(16-c*8);
                    if(rim!=null){
                        float illumination=0;
                        if(normal.lengthSquared()>0)normal.normalize();
                        for(var direction:rim)illumination+=Math.max(0,normal.dot(direction));
                        int green=Math.clamp((int)(surface.colors[i*4+1]*1.9f*illumination*255),0,255);
                        int blue=Math.clamp((int)(surface.colors[i*4+2]*(.3f+2*illumination)*255),0,255);
                        color=(color&0xff000000)|green<<8|blue;
                    }
                    float u=surface.uv[i*2],texV=surface.uv[i*2+1];
                    if(effect!=null){
                        // Blend 3 is additive and applies four times the submitted vertex alpha to RGB.
                        color=0x40000000;
                        for(int c=0;c<3;c++)color|=Math.clamp(Math.round(surface.colors[i*4+c]*alpha*255),0,255)<<(16-c*8);
                        if(viewLeft!=null){
                            if(normal.lengthSquared()>0)normal.normalize();
                            u=.5f-.5f*normal.dot(viewLeft);
                            texV=.5f-.5f*normal.dot(viewUp);
                        }
                        texV=effect.atlasV(effectFrame,texV);
                    }else if(surface.skullGem){
                        texV=(gemFrame+(.5f+(texV-(float)Math.floor(texV))*63)/64)/60;
                    }
                    buffer.addVertex(v.x,v.y,v.z,hurt.tint(color),u,texV,overlay,light,normal.x,normal.y,normal.z);
                }
            });
        }
    }
    private static float[] cameraVertices(PoseStack.Pose pose,float[] skinned,Vector3f vertex,Vector3f normal){
        var buffer=CAMERA_VERTICES.get();
        if(buffer.length<skinned.length){
            buffer=new float[skinned.length];
            CAMERA_VERTICES.set(buffer);
        }
        for(int at=0;at<skinned.length;at+=6){
            pose.pose().transformPosition(skinned[at],skinned[at+1],skinned[at+2],vertex).get(buffer,at);
            pose.transformNormal(skinned[at+3],skinned[at+4],skinned[at+5],normal).get(buffer,at+3);
        }
        return buffer;
    }
    private float[] skin(Surface surface,Posed posed){
        int count=surface.positions.length/3;skinnedVertices+=count;float[] skinned=new float[count*6];
        Vector3f p=new Vector3f(),n=new Vector3f(),temp=new Vector3f();
        for(int i=0;i<count;i++){
            float px=surface.positions[i*3],py=surface.positions[i*3+1],pz=surface.positions[i*3+2];
            float nx=surface.normals[i*3],ny=surface.normals[i*3+1],nz=surface.normals[i*3+2];
            for(int m=0;m<Math.min(posed.morphs.length,surface.morphPositions.length);m++){
                float w=posed.morphs[m];if(w==0)continue;
                if(surface.morphPositions[m]!=null){px+=w*surface.morphPositions[m][i*3];py+=w*surface.morphPositions[m][i*3+1];pz+=w*surface.morphPositions[m][i*3+2];}
                if(surface.morphNormals[m]!=null){nx+=w*surface.morphNormals[m][i*3];ny+=w*surface.morphNormals[m][i*3+1];nz+=w*surface.morphNormals[m][i*3+2];}
            }
            p.zero();n.zero();float total=0;
            if(surface.weights!=null)for(int k=0;k<4;k++){
                float w=surface.weights[i*4+k];int joint=surface.joints[i*4+k];if(w==0)continue;
                posed.skin[joint].transformPosition(px,py,pz,temp);p.fma(w,temp);
                posed.skin[joint].transformDirection(nx,ny,nz,temp);n.fma(w,temp);total+=w;
            }
            if(total>0){p.div(total);if(n.lengthSquared()>0)n.normalize();}else{p.set(px,py,pz);n.set(nx,ny,nz);}
            p.get(skinned,i*6);n.get(skinned,i*6+3);
        }
        return skinned;
    }
    private float[] accessor(int id){
        if(accessors.containsKey(id))return accessors.get(id);
        var a=json.getAsJsonArray("accessors").get(id).getAsJsonObject();var view=json.getAsJsonArray("bufferViews").get(a.get("bufferView").getAsInt()).getAsJsonObject();
        int count=a.get("count").getAsInt(),type=a.get("componentType").getAsInt();int components=switch(a.get("type").getAsString()){case "SCALAR"->1;case "VEC2"->2;case "VEC3"->3;case "VEC4"->4;case "MAT4"->16;default->throw new IllegalArgumentException("Unsupported accessor");};
        int size=switch(type){case 5120,5121->1;case 5122,5123->2;case 5125,5126->4;default->throw new IllegalArgumentException("Unsupported component");};
        int stride=intOr(view,"byteStride",components*size),offset=intOr(a,"byteOffset",0)+intOr(view,"byteOffset",0);boolean normalized=a.has("normalized")&&a.get("normalized").getAsBoolean();
        float[] out=new float[count*components];for(int i=0;i<count;i++)for(int c=0;c<components;c++){
            int at=offset+i*stride+c*size;float v=switch(type){case 5126->bin.getFloat(at);case 5125->Integer.toUnsignedLong(bin.getInt(at));case 5123->Short.toUnsignedInt(bin.getShort(at));case 5122->bin.getShort(at);case 5121->Byte.toUnsignedInt(bin.get(at));case 5120->bin.get(at);default->0;};
            if(normalized)v/=switch(type){case 5121->255f;case 5123->65535f;case 5120->127f;case 5122->32767f;default->1;};out[i*components+c]=v;
        }accessors.put(id,out);return out;
    }
    private static int intOr(JsonObject j,String key,int fallback){return j.has(key)?j.get(key).getAsInt():fallback;}
    private static float[] array(JsonArray a){float[] r=new float[a.size()];for(int i=0;i<r.length;i++)r[i]=a.get(i).getAsFloat();return r;}
    private static int[] ints(JsonArray a){int[] r=new int[a.size()];for(int i=0;i<r.length;i++)r[i]=a.get(i).getAsInt();return r;}
    private static int[] toInts(float[] a){int[] r=new int[a.length];for(int i=0;i<r.length;i++)r[i]=(int)a[i];return r;}
    private static float[] toFloats(int[] a){float[] r=new float[a.length];for(int i=0;i<r.length;i++)r[i]=a[i];return r;}
    private static float[] select(float[] source,int components,int[] vertices){float[] out=new float[vertices.length*components];for(int i=0;i<vertices.length;i++)System.arraycopy(source,vertices[i]*components,out,i*components,components);return out;}
    private static float[] filled(int count,float value){float[] a=new float[count];Arrays.fill(a,value);return a;}
}
