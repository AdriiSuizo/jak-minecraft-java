package dev.jakmc.client.model;

import com.mojang.blaze3d.platform.NativeImage;
import com.mojang.blaze3d.vertex.PoseStack;
import dev.jakmc.JakData;
import dev.jakmc.JakMod;
import dev.jakmc.RuntimeConfig;
import java.io.IOException;
import java.nio.ByteBuffer;
import java.nio.ByteOrder;
import java.nio.charset.StandardCharsets;
import java.nio.file.Files;
import java.util.ArrayList;
import java.util.HashMap;
import java.util.List;
import java.util.Map;
import net.minecraft.client.Minecraft;
import net.minecraft.client.renderer.SubmitNodeCollector;
import net.minecraft.client.renderer.rendertype.RenderTypes;
import net.minecraft.client.renderer.texture.DynamicTexture;
import net.minecraft.client.renderer.texture.OverlayTexture;
import net.minecraft.resources.Identifier;
import net.minecraft.world.phys.Vec3;
import org.joml.Matrix4f;
import org.joml.Vector3f;
import org.joml.Vector4f;

/** Jak 3 art-cloth-geo and the original port's gravity, drag and distance constraints. */
final class JakCloth {
    private static final int BASE=0x80;
    private static final float METER=4096f;
    private static final double STEP=1.0/60;
    private static List<Definition> definitions;
    private record Pin(int point,String joint,Vector3f offset) {}
    private record Sphere(String joint,Vector3f offset,float radius) {}
    private record Edge(int a,int b) {}
    private record Definition(String joint,Vector3f[] rest,float[] uv,Pin[] pins,Sphere[] spheres,
                              Edge[] edges,int[] triangles,float gravity,float keep,Identifier texture) {}

    private final List<Piece> pieces=new ArrayList<>(3);
    private Vec3 origin;
    private long clock,session;
    private double time;
    private float scarfReach;
    private int resets;

    static void load() {
        if(definitions!=null)return;
        try {
            var root=RuntimeConfig.assets;
            var raw=ByteBuffer.wrap(Files.readAllBytes(root.resolve("raw_obj/jak-ext-geo-c-ag.go")))
                .order(ByteOrder.LITTLE_ENDIAN);
            var skirt=texture("jakc-skirt");
            var scarf=texture("jakc-scarfhanging");
            definitions=List.of(read(raw,"skirt",13,16,.151f,skirt),read(raw,"sash",6,32,.251f,skirt),
                read(raw,"scarf",6,16,.151f,scarf));
            JakMod.LOG.info("Loaded Jak 3 cloth meshes: skirt, sash and scarf ({} points, {} anchors)",
                definitions.stream().mapToInt(d->d.rest.length).sum(),definitions.stream().mapToInt(d->d.pins.length).sum());
        } catch(IOException|IndexOutOfBoundsException invalid) {
            JakMod.LOG.warn("Jak cloth could not load from the local OpenGOAL export",invalid);
            definitions=List.of();
        }
    }

    private static Identifier texture(String name)throws IOException {
        var path=RuntimeConfig.assets.resolve("textures/level-default-pris/"+name+".png");
        var id=JakData.id("local/cloth/"+name);
        try(var stream=Files.newInputStream(path);var ps2=NativeImage.read(stream)) {
            var image=ps2.mappedCopy(pixel->(pixel&0x00ffffff)|(Math.min(255,(pixel>>>24)*2)<<24));
            Minecraft.getInstance().getTextureManager().register(id,new DynamicTexture(id::toString,image));
        }
        return id;
    }

    private static Definition read(ByteBuffer b,String name,int width,float gravity,float drag,Identifier texture)
        throws IOException {
        String key="jak-ext-geo-c+0-jakclod0-"+name;
        int at=0;
        while(at+96<b.limit()&&(b.getInt(at)!=-1||!key.equals(string(b,b.getInt(at+8)))))at+=4;
        if(at+96>=b.limit())throw new IOException("Missing original cloth "+name);
        int count=b.getInt(at+12);
        if(count<width*2||count>256||count%width!=0||at+80+32*count>b.limit())
            throw new IOException("Invalid cloth grid "+name);
        var rest=new Vector3f[count];
        var uv=new float[count*2];
        for(int i=0;i<count;i++){
            int point=at+80+32*i;
            rest[i]=vector(b,point);
            uv[i*2]=b.getFloat(point+16);
            uv[i*2+1]=b.getFloat(point+20);
        }
        var pins=new ArrayList<Pin>();
        var spheres=new ArrayList<Sphere>();
        int first=BASE+b.getInt(at+48);
        for(int i=0,n=entries(b,first);i<n;i++){
            int row=first+12+i*32,point=b.getShort(row+22);
            if(point>=0&&point<count)pins.add(new Pin(point,joint(b,row+16),vector(b,row)));
        }
        first=BASE+b.getInt(at+52);
        for(int i=0,n=entries(b,first);i<n;i++){
            int row=first+12+i*32;
            spheres.add(new Sphere(joint(b,row+16),vector(b,row).div(METER),b.getFloat(row+20)/METER));
        }
        if(pins.isEmpty())throw new IOException("Missing cloth anchors "+name);
        var axes=List.of(new Vector3f(1,0,0),new Vector3f(0,1,0),new Vector3f(0,0,1),
            new Vector3f(-1,0,0),new Vector3f(0,-1,0),new Vector3f(0,0,-1));
        var turn=new Matrix4f();
        float best=Float.POSITIVE_INFINITY;
        for(var x:axes)for(var y:axes){
            if(Math.abs(x.dot(y))>.5f)continue;
            var rotation=new Matrix4f().setColumn(0,new Vector4f(x,0)).setColumn(1,new Vector4f(y,0));
            var z=x.cross(y,new Vector3f());
            rotation.setColumn(2,new Vector4f(z,0));
            float error=0;
            var scratch=new Vector3f();
            for(var pin:pins)error+=rotation.transformDirection(rest[pin.point],scratch).distanceSquared(pin.offset);
            if(error<best){best=error;turn.set(rotation);}
        }
        for(var point:rest)turn.transformDirection(point).div(METER);
        for(var pin:pins)pin.offset.div(METER);
        var edges=new ArrayList<Edge>();
        int rows=count/width,index=0;
        int[] triangles=new int[(rows-1)*(width-1)*6];
        for(int r=0;r<rows;r++)for(int c=0;c<width;c++){
            int a=r*width+c;
            if(c+1<width)edges.add(new Edge(a,a+1));
            if(r+1>=rows)continue;
            edges.add(new Edge(a,a+width));
            if(c+1>=width)continue;
            int d=a+width,e=d+1;
            edges.add(new Edge(a,e));
            triangles[index++]=a;triangles[index++]=d;triangles[index++]=a+1;
            triangles[index++]=a+1;triangles[index++]=d;triangles[index++]=e;
        }
        return new Definition(pins.getFirst().joint,rest,uv,pins.toArray(Pin[]::new),spheres.toArray(Sphere[]::new),
            edges.toArray(Edge[]::new),triangles,gravity,1-drag,texture);
    }

    private static int entries(ByteBuffer b,int at)throws IOException {
        int n=b.getInt(at);
        if(n<0||n>256||at+12+32*n>b.limit())throw new IOException("Invalid art-cloth-geo array");
        return n;
    }

    private static Vector3f vector(ByteBuffer b,int at){return new Vector3f(b.getFloat(at),b.getFloat(at+4),b.getFloat(at+8));}
    private static String string(ByteBuffer b,int pointer) {
        long offset=(long)BASE+4+pointer;
        if(offset<BASE||offset>=b.limit())return "";
        int start=(int)offset,end=start;
        while(end<b.limit()&&end-start<128&&b.get(end)!=0)end++;
        return new String(b.array(),start,end-start,StandardCharsets.UTF_8);
    }
    private static String joint(ByteBuffer b,int at)throws IOException {
        String value=string(b,b.getInt(at));
        if(value.isEmpty())throw new IOException("Invalid cloth joint name");
        return value;
    }

    /** Kinematic portrait: source cloth geometry/pins, no world-space inertia. */
    int submitPreview(GltfModel model,GltfModel.Posed pose,PoseStack stack,SubmitNodeCollector collector,int light){
        if(definitions.isEmpty())return 0;
        if(pieces.isEmpty())for(var definition:definitions)pieces.add(new Piece(definition));
        var root=new Matrix4f(stack.last().pose());var at=root.getTranslation(new Vector3f());root.setTranslation(0,0,0);
        var translation=new PoseStack();translation.translate(at.x,at.y,at.z);
        var joints=new HashMap<String,Matrix4f>();
        for(var piece:pieces){
            piece.initialized=false;piece.update(model,pose,root,joints,Vec3.ZERO,1,0,0,0);
            for(int i=0;i<piece.def.pins.length;i++)piece.pos[piece.def.pins[i].point].set(piece.pins[i]);
            piece.draw(translation,collector,light,1);
        }
        return pieces.size();
    }

    int submit(GltfModel model,GltfModel.Posed pose,PoseStack stack,SubmitNodeCollector collector,int light,
               Vec3 camera,float scale,long key) {
        if(definitions.isEmpty())return 0;
        var root=new Matrix4f(stack.last().pose());
        var relative=root.getTranslation(new Vector3f());
        Vec3 nextOrigin=camera.add(relative.x,relative.y,relative.z);
        root.setTranslation(0,0,0);
        long now=System.nanoTime();
        double elapsed=(now-clock)*1e-9;
        if(origin==null||session!=key||elapsed>.15||nextOrigin.distanceToSqr(origin)>9*scale*scale){
            pieces.clear();
            resets++;
            time=0;
            origin=nextOrigin;
            for(var definition:definitions)pieces.add(new Piece(definition));
            elapsed=0;
        }
        var offset=origin.subtract(nextOrigin);
        var joints=new HashMap<String,Matrix4f>();
        var translation=new PoseStack();
        translation.translate(relative.x,relative.y,relative.z);
        int steps=(int)((time+elapsed)/STEP);
        double remaining=Math.max(0,time+elapsed-steps*STEP);
        for(var piece:pieces){
            piece.update(model,pose,root,joints,offset,scale,time,elapsed,steps);
            piece.draw(translation,collector,light,(float)(remaining/STEP));
        }
        var scarf=pieces.getLast();
        scarfReach=scarf.pos[scarf.pos.length-3].distance(scarf.pins[0]);
        clock=now;
        session=key;
        origin=nextOrigin;
        time=remaining;
        return pieces.size();
    }

    float scarfReach(){return scarfReach;}
    int resets(){return resets;}

    private static final class Piece {
        final Definition def;
        final Vector3f[] pos,prev,pins,priorPins,centres;
        final float[] lengths;
        final boolean[] fixed;
        boolean initialized;
        Piece(Definition def){
            this.def=def;
            pos=vectors(def.rest.length);prev=vectors(pos.length);
            pins=vectors(def.pins.length);priorPins=vectors(pins.length);
            centres=vectors(def.spheres.length);
            lengths=new float[def.edges.length];
            fixed=new boolean[pos.length];
            for(var pin:def.pins)fixed[pin.point]=true;
        }
        private static Vector3f[] vectors(int count){
            var array=new Vector3f[count];
            for(int i=0;i<count;i++)array[i]=new Vector3f();
            return array;
        }

        void update(GltfModel model,GltfModel.Posed pose,Matrix4f root,Map<String,Matrix4f> joints,Vec3 shift,
                    float scale,double remainder,double elapsed,int steps){
            java.util.function.Function<String,Matrix4f> matrix=
                name->joints.computeIfAbsent(name,n->new Matrix4f(root).mul(model.joint(pose,n)));
            if(!initialized){
                var base=matrix.apply(def.joint);
                for(int i=0;i<pos.length;i++){base.transformPosition(def.rest[i],pos[i]);prev[i].set(pos[i]);}
                for(int i=0;i<lengths.length;i++){var edge=def.edges[i];lengths[i]=pos[edge.a].distance(pos[edge.b]);}
            }
            float x=(float)shift.x,y=(float)shift.y,z=(float)shift.z;
            for(int i=0;i<pos.length;i++){pos[i].add(x,y,z);prev[i].add(x,y,z);}
            for(int i=0;i<pins.length;i++){
                var pin=def.pins[i];
                priorPins[i].set(pins[i]).add(x,y,z);
                matrix.apply(pin.joint).transformPosition(pin.offset,pins[i]);
                if(!initialized)priorPins[i].set(pins[i]);
            }
            for(int i=0;i<centres.length;i++){
                var sphere=def.spheres[i];
                matrix.apply(sphere.joint).transformPosition(sphere.offset,centres[i]);
            }
            initialized=true;
            var delta=new Vector3f();
            float gravity=(float)(-def.gravity*scale*STEP*STEP);
            for(int step=0;step<steps;step++){
                float fraction=(float)Math.clamp(((step+1)*STEP-remainder)/elapsed,0,1);
                for(int i=0;i<pos.length;i++){
                    pos[i].sub(prev[i],delta);
                    prev[i].set(pos[i]);
                    pos[i].fma(def.keep,delta).add(0,gravity,0);
                }
                for(int pass=0;pass<4;pass++){
                    for(int i=0;i<pins.length;i++)priorPins[i].lerp(pins[i],fraction,pos[def.pins[i].point]);
                    for(int i=0;i<lengths.length;i++){
                        var edge=def.edges[i];
                        boolean a=fixed[edge.a],b=fixed[edge.b];
                        if(a&&b)continue;
                        float length=pos[edge.b].sub(pos[edge.a],delta).length();
                        if(length<1e-6f)continue;
                        float correction=(length-lengths[i])/length;
                        if(!a)pos[edge.a].fma(b?correction:correction*.5f,delta);
                        if(!b)pos[edge.b].fma(a?-correction:-correction*.5f,delta);
                    }
                    for(int i=0;i<pos.length;i++){
                        if(fixed[i])continue;
                        for(int j=0;j<centres.length;j++){
                            float radius=def.spheres[j].radius*scale;
                            float squared=pos[i].sub(centres[j],delta).lengthSquared();
                            if(squared<radius*radius&&squared>1e-12f)
                                pos[i].set(centres[j]).fma(radius/(float)Math.sqrt(squared),delta);
                        }
                    }
                }
            }
        }

        void draw(PoseStack root,SubmitNodeCollector collector,int light,float fraction){
            // Submission is deferred. Keep this frame's vertices independent from the next solver step.
            var vertices=new float[pos.length*3];
            var sample=new Vector3f();
            for(int i=0;i<pos.length;i++)prev[i].lerp(pos[i],fraction,sample).get(vertices,i*3);
            for(int i=0;i<pins.length;i++)pins[i].get(vertices,def.pins[i].point*3);
            var hurt=JakHurtVisuals.current();
            collector.submitCustomGeometry(root,RenderTypes.entityCutout(def.texture),(pose,buffer)->{
                var a=new Vector3f();var b=new Vector3f();var normal=new Vector3f();var vertex=new Vector3f();
                for(int i=0;i<def.triangles.length;i+=3){
                    int first=def.triangles[i],second=def.triangles[i+1],third=def.triangles[i+2];
                    a.set(vertices,first*3);b.set(vertices,second*3).sub(a);
                    normal.set(vertices,third*3).sub(a);
                    b.cross(normal,normal);
                    if(normal.lengthSquared()<1e-12f)continue;
                    normal.normalize();
                    int shade=Math.round(255*(.6f+.4f*Math.abs(normal.y)));
                    int color=hurt.tint(0xff000000|shade*0x010101);
                    for(int side=0;side<2;side++){
                        int middle=side==0?second:third,last=side==0?third:second;
                        for(int corner=0;corner<4;corner++){
                            int point=corner==0?first:corner==1?middle:last;
                            pose.pose().transformPosition(vertex.set(vertices,point*3));
                            buffer.addVertex(vertex.x,vertex.y,vertex.z,color,def.uv[point*2],def.uv[point*2+1],
                                OverlayTexture.NO_OVERLAY,light,normal.x,normal.y,normal.z);
                        }
                        normal.negate();
                    }
                }
            });
        }
    }
}
