package dev.jakmc.client.model;

import com.mojang.blaze3d.platform.NativeImage;
import dev.jakmc.JakData;
import java.io.IOException;
import java.nio.file.Path;
import java.util.ArrayList;
import java.util.List;
import net.minecraft.client.Minecraft;
import net.minecraft.client.renderer.texture.DynamicTexture;
import net.minecraft.resources.Identifier;
import org.joml.Vector2f;
import org.joml.Vector3f;

/** Neutral iris and pupil compositing from Jak 3's render-eyes-64 path. */
final class ForwardEyes {
    record Mesh(float[] positions,float[] uv,int[] indices,String material){}
    record Eye(Identifier open,Identifier closed){}
    record Pair(Eye left,Eye right){}
    private record Triangle(Vector3f a,Vector3f b,Vector3f c){}
    private static final int SIZE=64;
    private static final Vector3f FORWARD=new Vector3f(0,0,1);

    static Pair build(Path textures,String name,List<Mesh> meshes)throws IOException{
        boolean daxter=name.equals("daxter");
        String irisPath=daxter?"common/autoeye-iris.png":"arenacst-pris/jakb-eye.png";
        String lidPath=daxter?"arenacst-pris/daxter-eyelid.png":"arenacst-pris/jakb-eyelid.png";
        try(var iris=read(textures.resolve(irisPath));
            var pupil=read(textures.resolve("common/autoeye-pupil.png"));
            var lid=read(textures.resolve(lidPath))){
            Vector2f right=ahead(meshes,"right",daxter);
            Vector2f left=daxter?ahead(meshes,"left",true):new Vector2f(1-right.x,right.y);
            var l=register(name+"/left",compose(iris,pupil,left));
            var r=register(name+"/right",compose(iris,pupil,right));
            var closed=new NativeImage(SIZE,SIZE,false);
            for(int y=0;y<SIZE;y++)for(int x=0;x<SIZE;x++)
                closed.setPixel(x,y,lid.getPixel(x*lid.getWidth()/SIZE,y*lid.getHeight()/SIZE)|0xff000000);
            var shut=register(name+"/closed",closed);
            return new Pair(new Eye(l,shut),new Eye(r,shut));
        }
    }

    private static NativeImage read(Path path)throws IOException{
        try(var input=java.nio.file.Files.newInputStream(path)){return NativeImage.read(input);}
    }

    private static Identifier register(String key,NativeImage image){
        Identifier id=JakData.id("local/eyes/"+key);
        Minecraft.getInstance().getTextureManager().register(id,new DynamicTexture(id::toString,image));
        return id;
    }

    private static NativeImage compose(NativeImage iris,NativeImage pupil,Vector2f at){
        var image=new NativeImage(SIZE,SIZE,false);
        int background=iris.getPixel(0,0)|0xff000000;
        for(int y=0;y<SIZE;y++)for(int x=0;x<SIZE;x++)image.setPixel(x,y,background);
        draw(image,iris,.55f,at);
        draw(image,pupil,.45f,at);
        return image;
    }

    private static void draw(NativeImage target,NativeImage source,float scale,Vector2f center){
        for(int y=0;y<SIZE;y++)for(int x=0;x<SIZE;x++){
            float u=((x+.5f)/SIZE-center.x)/scale+.5f;
            float v=((y+.5f)/SIZE-center.y)/scale+.5f;
            if(u<0||u>=1||v<0||v>=1)continue;
            int src=source.getPixel((int)(u*source.getWidth()),(int)(v*source.getHeight()));
            int dst=target.getPixel(x,y);
            float alpha=Math.min(1,(src>>>24)/128f);
            int result=0xff000000;
            for(int bit=0;bit<24;bit+=8){
                int a=src>>bit&255,b=dst>>bit&255;
                result|=Math.round(b+(a-b)*alpha)<<bit;
            }
            target.setPixel(x,y,result);
        }
    }

    private static Vector2f ahead(List<Mesh> meshes,String side,boolean forward){
        Mesh eye=meshes.stream().filter(m->m.material.startsWith("programmer_eye")&&m.material.contains(side)).findFirst()
            .orElseThrow(()->new IllegalArgumentException("Source eye mesh missing: "+side));
        var low=new Vector3f(Float.POSITIVE_INFINITY);
        var high=new Vector3f(Float.NEGATIVE_INFINITY);
        for(int vertex:eye.indices){
            var point=point(eye,vertex);
            low.min(point);high.max(point);
        }
        var blockers=new ArrayList<Triangle>();
        for(var mesh:meshes){
            if(mesh==eye)continue;
            for(int i=0;i<mesh.indices.length;i+=3){
                var tri=triangle(mesh,i);
                var a=new Vector3f(tri.a).min(tri.b).min(tri.c);
                var b=new Vector3f(tri.a).max(tri.b).max(tri.c);
                if(b.x>low.x&&a.x<high.x&&b.y>low.y&&a.y<high.y&&b.z>low.z)blockers.add(tri);
            }
        }
        float weightSum=0,u=0,v=0;
        var ray=new Vector3f();
        for(int i=0;i<eye.indices.length;i+=3){
            var tri=triangle(eye,i);
            var normal=new Vector3f(tri.b).sub(tri.a).cross(new Vector3f(tri.c).sub(tri.a));
            float area=normal.length();
            if(area<1e-10f)continue;
            float weight=forward?area*(float)Math.pow(Math.max(0,normal.z/area),8):1;
            if(weight==0)continue;
            int j0=eye.indices[i]*2,j1=eye.indices[i+1]*2,j2=eye.indices[i+2]*2;
            for(int a=0;a<=6;a++)for(int b=0;b<=6-a;b++){
                float wb=a/6f,wc=b/6f,wa=1-wb-wc;
                ray.set(tri.a).mul(wa).fma(wb,tri.b).fma(wc,tri.c);
                if(hidden(ray,blockers))continue;
                u+=(eye.uv[j0]*wa+eye.uv[j1]*wb+eye.uv[j2]*wc)*weight;
                v+=(eye.uv[j0+1]*wa+eye.uv[j1+1]*wb+eye.uv[j2+1]*wc)*weight;
                weightSum+=weight;
            }
        }
        return weightSum>0?new Vector2f(u/weightSum,v/weightSum):new Vector2f(.5f,.5f);
    }

    private static boolean hidden(Vector3f from,List<Triangle> blockers){
        for(var tri:blockers){
            var e1=new Vector3f(tri.b).sub(tri.a);
            var e2=new Vector3f(tri.c).sub(tri.a);
            var h=new Vector3f(FORWARD).cross(e2);
            float determinant=e1.dot(h);
            if(Math.abs(determinant)<1e-12f)continue;
            var s=new Vector3f(from).sub(tri.a);
            float u=s.dot(h)/determinant;
            if(u<0||u>1)continue;
            var q=s.cross(e1);
            float v=q.z/determinant;
            if(v>=0&&u+v<=1&&e2.dot(q)/determinant>1e-4f)return true;
        }
        return false;
    }

    private static Triangle triangle(Mesh mesh,int index){
        return new Triangle(point(mesh,mesh.indices[index]),point(mesh,mesh.indices[index+1]),point(mesh,mesh.indices[index+2]));
    }
    private static Vector3f point(Mesh mesh,int vertex){
        int i=vertex*3;
        return new Vector3f(mesh.positions[i],mesh.positions[i+1],mesh.positions[i+2]);
    }
}
