import dev.jakmc.nativebridge.NativeJak;
import java.nio.file.*;
import com.google.gson.*;
public class EnemyMetadataCheck {
 public static void main(String[] args)throws Exception{
  NativeJak.load(Path.of(args[0]));JsonArray all=JsonParser.parseString(Files.readString(Path.of("src/main/resources/data/jakmc/enemies.json"))).getAsJsonArray();JsonObject report=new JsonObject();
  for(var v:all){var k=v.getAsJsonObject();String m=k.get("model").getAsString(),game=k.get("game").getAsString();String s=NativeJak.animationMetadata(Path.of(args[1],game,"data/decompiler_out",game).toString(),m,game.equals("jak1"));var data=JsonParser.parseString(s).getAsJsonObject();report.add(m,data);
   for(var c:k.getAsJsonArray("clips"))if(!c.getAsString().isEmpty()&&!data.has(c.getAsString()))System.out.println("MISSING "+m+" "+c);
   if(data.size()<3)throw new AssertionError("Incomplete "+m);
  }
  Files.writeString(Path.of(".local/enemy-animation-metadata.json"),new GsonBuilder().setPrettyPrinting().create().toJson(report));System.out.println("PASS: "+report.size()+" original enemy art groups decoded");
 }
}
