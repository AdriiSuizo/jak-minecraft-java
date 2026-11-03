import dev.jakmc.nativebridge.NativeJak;
import com.google.gson.*;
import java.nio.file.*;
import java.util.*;
public final class GunEffectsBoundaryCheck {
 public static void main(String[] args)throws Exception{
  NativeJak.load(Path.of(args[0]));NativeJak.configure(args[1]);long id=NativeJak.create(0),fx=NativeJak.fxCreate();
  var input=NativeJak.buffer(64);input.putFloat(44,1).putInt(48,32).putInt(60,3);
  var floor=NativeJak.buffer(80);NativeBoundaryCheck.triangle(floor,-50,0,-50,-50,0,50,50,0,-50);NativeBoundaryCheck.triangle(floor,50,0,-50,-50,0,50,50,0,50);
  java.nio.ByteBuffer empty=NativeJak.buffer(0),view=NativeJak.buffer(44),vertices=NativeJak.buffer(8192*44),bolts=NativeJak.buffer(32768*44);
  view.putFloat(0).putFloat(2).putFloat(-4).putFloat(0).putFloat(0).putFloat(1).putFloat(0).putFloat(1).putFloat(0).putFloat(70).putFloat(720);
  int peak=0,fire=0;Set<String> sounds=new TreeSet<>();JsonObject last=null;
  for(int i=0;i<80;i++){
   int held=i>15&&i%6<3?64:0,pressed=i>15&&i%6==0?64:0;input.putInt(0,held).putInt(4,pressed);
   last=JsonParser.parseString(NativeJak.step(id,input,floor,empty,3)).getAsJsonObject();
   for(var e:last.getAsJsonArray("events"))if(e.getAsJsonObject().get("kind").getAsString().equals("fire"))fire++;
   for(var e:last.getAsJsonArray("sounds"))sounds.add(e.getAsJsonObject().get("name").getAsString());
   NativeJak.fxPush(fx,last.get("effects").toString(),0,0,0);
   for(int k=0;k<3;k++)peak=Math.max(peak,(int)NativeJak.fxDraw(fx,1/60f,view,vertices,bolts));
  }
  NativeJak.destroy(id);NativeJak.fxDestroy(fx);
  System.out.println("Blaster fires="+fire+", peak vertices="+peak+", cues="+sounds+", clip="+last.get("clip"));
  if(fire==0||peak==0||!sounds.contains("YELLOW_SHOT_FIR"))throw new AssertionError("Gameplay/audio/effect path incomplete");
 }
}
