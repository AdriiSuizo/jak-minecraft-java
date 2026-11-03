import dev.jakmc.nativebridge.ModelSegments;
import java.nio.*;import java.nio.file.*;import java.util.*;import com.google.gson.*;
public class ModelSegmentsCheck {
 public static void main(String[] args)throws Exception{
  var definitions=JsonParser.parseString(Files.readString(Path.of("src/main/resources/data/jakmc/enemies.json"))).getAsJsonArray();
  for(var v:definitions){var d=v.getAsJsonObject();String model=d.get("model").getAsString();if(!Set.of("grunt","flitter","mantis").contains(model))continue;
   Path root=Path.of(args[0]);byte[] bytes=Files.readAllBytes(root.resolve(d.get("asset").getAsString()));ByteBuffer b=ByteBuffer.wrap(bytes).order(ByteOrder.LITTLE_ENDIAN);var j=JsonParser.parseString(new String(bytes,20,b.getInt(12),java.nio.charset.StandardCharsets.UTF_8)).getAsJsonObject();var counts=new ArrayList<Integer>();
   for(var mesh:j.getAsJsonArray("meshes"))for(var p:mesh.getAsJsonObject().getAsJsonArray("primitives")){int id=p.getAsJsonObject().get("indices").getAsInt();counts.add(j.getAsJsonArray("accessors").get(id).getAsJsonObject().get("count").getAsInt()/3);}
   var parts=ModelSegments.read(root.resolve("raw_obj/"+model+"-ag.go"),model+"-lod0",counts.stream().mapToInt(Integer::intValue).toArray());if(parts==null)throw new AssertionError(model+" segmentation mismatch");int changed=0;for(int i=0;i<counts.size();i++)if(parts.shown(i,~4L)!=parts.shown(i,~2L))changed++;if(changed==0)throw new AssertionError("Gem segments did not change "+model);System.out.println(model+": "+changed+" surfaces switch when gem is released");
  }
 }
}
