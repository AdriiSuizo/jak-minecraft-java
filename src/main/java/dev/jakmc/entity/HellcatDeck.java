package dev.jakmc.entity;

import com.google.gson.JsonArray;
import com.google.gson.JsonParser;
import java.io.IOException;
import java.io.InputStreamReader;
import java.io.UncheckedIOException;
import java.nio.ByteBuffer;
import java.nio.charset.StandardCharsets;
import java.util.ArrayList;
import java.util.List;
import net.minecraft.world.phys.AABB;
import net.minecraft.world.phys.Vec3;

/** The original hull relief in the parked craft's flat deck frame. */
final class HellcatDeck {
    private static final HellcatDeck CAT=load("hellcat");
    private static final HellcatDeck DOG=load("helldog");
    private final Vec3 min,max;
    private final int width,depth;
    private final float cell;
    private final float[] heights;
    private final List<Triangle> triangles;
    private record Triangle(Vec3 a,Vec3 b,Vec3 c,int pat){}

    private HellcatDeck(Vec3 min,Vec3 max,int width,int depth,float cell,float[] heights){
        this.min=min;this.max=max;this.width=width;this.depth=depth;this.cell=cell;this.heights=heights;
        var mesh=new ArrayList<Triangle>();
        for(int z=0;z<depth;z++)for(int x=0;x<width;x++){
            Vec3 a=centre(x,z);
            if(a==null)continue;
            Vec3 b=centre(x+1,z),c=centre(x+1,z+1),d=centre(x,z+1);
            if(b!=null&&c!=null&&d!=null){
                mesh.add(new Triangle(a,c,b,0));
                mesh.add(new Triangle(a,d,c,0));
            }
            if(b==null)rim(mesh,a,1,0);
            if(d==null)rim(mesh,a,0,1);
            if(centre(x-1,z)==null)rim(mesh,a,-1,0);
            if(centre(x,z-1)==null)rim(mesh,a,0,-1);
        }
        triangles=List.copyOf(mesh);
    }

    private void rim(List<Triangle> mesh,Vec3 center,int x,int z){
        float half=cell/2;
        Vec3 side=new Vec3(x*half,0,z*half);
        Vec3 across=new Vec3(z*half,0,-x*half);
        Vec3 a=center.add(across),b=center.subtract(across),c=b.add(side),d=a.add(side);
        mesh.add(new Triangle(a,b,c,0));
        mesh.add(new Triangle(a,c,d,0));
        Vec3 lowerC=new Vec3(c.x,min.y,c.z),lowerD=new Vec3(d.x,min.y,d.z);
        mesh.add(new Triangle(d,c,lowerC,1));
        mesh.add(new Triangle(d,lowerC,lowerD,1));
    }

    private Vec3 centre(int x,int z){
        float y=height(x,z);
        return Float.isNaN(y)?null:new Vec3(min.x+(x+.5)*cell,y,min.z+(z+.5)*cell);
    }
    private float height(int x,int z){
        return x>=0&&z>=0&&x<width&&z<depth?heights[z*width+x]:Float.NaN;
    }
    private float top(double x,double z){
        float own=height((int)Math.floor((x-min.x)/cell),(int)Math.floor((z-min.z)/cell));
        double fx=(x-min.x)/cell-.5,fz=(z-min.z)/cell-.5;
        int i=(int)Math.floor(fx),j=(int)Math.floor(fz);
        float a=height(i,j),b=height(i+1,j),c=height(i+1,j+1),d=height(i,j+1);
        if(Float.isNaN(a+b+c+d))return own;
        double u=fx-i,v=fz-j;
        return (float)(u>=v?a+(b-a)*u+(c-b)*v:a+(c-d)*u+(d-a)*v);
    }
    static AABB bounds(Vec3 root,float yaw,boolean dog){
        var model=dog?DOG:CAT;
        float angle=(float)-Math.toRadians(yaw);
        double sin=Math.sin(angle),cos=Math.cos(angle);
        var first=rotate(model.min,sin,cos);
        var box=new AABB(first.x,model.min.y,first.z,first.x,model.max.y,first.z);
        for(double x:new double[]{model.min.x,model.max.x})for(double z:new double[]{model.min.z,model.max.z}){
            var point=rotate(new Vec3(x,0,z),sin,cos);
            box=box.minmax(new AABB(point.x,model.min.y,point.z,point.x,model.max.y,point.z));
        }
        return box.move(root);
    }

    static Vec3 exit(JakHellcat craft){
        double x=.8,z=.384;
        var model=craft.hellDog()?DOG:CAT;
        float y=model.top(x,z);
        if(Float.isNaN(y))y=(float)model.max.y;
        return craft.position().add(new Vec3(x,y+.045,z).yRot((float)-Math.toRadians(craft.getYRot())));
    }

    static int maximumBytes(){return Math.max(CAT.triangles.size(),DOG.triangles.size())*40;}
    static void append(JakHellcat craft,Vec3 origin,AABB region,ByteBuffer output){
        Vec3 root=craft.position();
        Vec3 offset=root.subtract(origin);
        float angle=(float)-Math.toRadians(craft.getYRot());
        double sin=Math.sin(angle),cos=Math.cos(angle);
        for(Triangle tri:(craft.hellDog()?DOG:CAT).triangles){
            Vec3 a=rotate(tri.a,sin,cos),b=rotate(tri.b,sin,cos),c=rotate(tri.c,sin,cos);
            double minX=Math.min(a.x,Math.min(b.x,c.x))+root.x,maxX=Math.max(a.x,Math.max(b.x,c.x))+root.x;
            double minY=Math.min(a.y,Math.min(b.y,c.y))+root.y,maxY=Math.max(a.y,Math.max(b.y,c.y))+root.y;
            double minZ=Math.min(a.z,Math.min(b.z,c.z))+root.z,maxZ=Math.max(a.z,Math.max(b.z,c.z))+root.z;
            if(maxX<region.minX||minX>region.maxX||maxY<region.minY||minY>region.maxY||maxZ<region.minZ||minZ>region.maxZ)continue;
            point(output,a,offset);point(output,b,offset);point(output,c,offset);output.putInt(tri.pat);
        }
    }

    private static Vec3 rotate(Vec3 p,double sin,double cos){
        return new Vec3(p.x*cos+p.z*sin,p.y,p.z*cos-p.x*sin);
    }
    private static void point(ByteBuffer output,Vec3 point,Vec3 offset){
        output.putFloat((float)(point.x+offset.x)).putFloat((float)(point.y+offset.y)).putFloat((float)(point.z+offset.z));
    }
    private static HellcatDeck load(String name){
        try(var stream=HellcatDeck.class.getResourceAsStream("/assets/jakmc/"+name+"/deck.json")){
            if(stream==null)throw new IOException("Import the owned HellCat hull with tools/import_hellcat_deck.py");
            var doc=JsonParser.parseReader(new InputStreamReader(stream,StandardCharsets.UTF_8)).getAsJsonObject();
            var size=doc.getAsJsonArray("size");
            var values=doc.getAsJsonArray("top");
            float[] heights=new float[values.size()];
            for(int i=0;i<heights.length;i++)heights[i]=values.get(i).isJsonNull()?Float.NaN:values.get(i).getAsFloat();
            return new HellcatDeck(point(doc.getAsJsonArray("min")),point(doc.getAsJsonArray("max")),
                size.get(0).getAsInt(),size.get(1).getAsInt(),doc.get("cell").getAsFloat(),heights);
        }catch(IOException failure){throw new UncheckedIOException(failure);}
    }
    private static Vec3 point(JsonArray values){
        return new Vec3(values.get(0).getAsDouble(),values.get(1).getAsDouble(),values.get(2).getAsDouble());
    }
}
