import dev.jakmc.nativebridge.NativeJak;
import com.google.gson.*;
import java.nio.*;
import java.nio.file.*;
import java.util.*;
/** Real JNI gameplay path: authored ropes/rings, bounded compact packets and shot IDs. */
public final class WeaponPresentationCheck {
 public static void main(String[] args)throws Exception{
  NativeJak.load(Path.of(args[0]));NativeJak.configure(args[1]);
  for(int gun:new int[]{7,10,5,2,11})check(gun);
 }
 private static void check(int gun){
  long id=NativeJak.create(0),fx=NativeJak.fxCreate();
  var input=NativeJak.buffer(64);input.putFloat(44,1).putInt(48,32).putInt(60,gun);
  var floor=NativeJak.buffer(80);NativeBoundaryCheck.triangle(floor,-100,0,-100,-100,0,100,100,0,-100);NativeBoundaryCheck.triangle(floor,100,0,-100,-100,0,100,100,0,100);
  var empty=NativeJak.buffer(0);var view=NativeJak.buffer(44);view.putFloat(0).putFloat(5).putFloat(-6).putFloat(0).putFloat(0).putFloat(1).putFloat(0).putFloat(1).putFloat(0).putFloat(70).putFloat(720);
  var sprites=NativeJak.buffer(16384*44);var bolts=NativeJak.buffer(32768*44);
  int boltPeak=0,ringPeak=0,modelFrames=0,maxBytes=0,visuals=0,stable=0;Set<String> lastIds=Set.of(),sounds=new TreeSet<>();
  try{
   for(int t=0;t<180;t++){
    int held=t>=20&&t<80?64:0;input.putInt(0,held).putInt(4,t==20?64:0);
    String wire=NativeJak.step(id,input,floor,empty,3);maxBytes=Math.max(maxBytes,wire.length());JsonObject pose=JsonParser.parseString(wire).getAsJsonObject();
    for(var c:pose.getAsJsonArray("effects"))if(c.getAsJsonObject().has("WeaponVisuals"))visuals++;
    for(var s:pose.getAsJsonArray("loops"))sounds.add(s.getAsJsonObject().get("name").getAsString());
    for(var s:pose.getAsJsonArray("sounds"))sounds.add(s.getAsJsonObject().get("name").getAsString());
    Set<String> ids=new HashSet<>();for(var value:pose.getAsJsonArray("models")){var m=value.getAsJsonObject();String key=m.get("id").getAsString();if(!ids.add(key))throw new AssertionError("Duplicate model identity");if(lastIds.contains(key))stable++;modelFrames++;}lastIds=ids;
    NativeJak.fxPush(fx,pose.get("effects").toString(),0,0,0);
    for(int k=0;k<3;k++){
     long count=NativeJak.fxDraw(fx,1/60f,view,sprites,bolts);boltPeak=Math.max(boltPeak,(int)(count>>>32));int dark=0;
     for(int i=0;i<(int)count;i++){int at=i*44;for(int j=0;j<9;j++)if(!Float.isFinite(sprites.getFloat(at+j*4)))throw new AssertionError("Nonfinite geometry");if(sprites.getFloat(at+12)==0&&sprites.getFloat(at+16)==0&&sprites.getFloat(at+20)==0&&sprites.getFloat(at+24)>0)dark++;}
     ringPeak=Math.max(ringPeak,dark);
    }
   }
   if(visuals!=180*3)throw new AssertionError("Not a 60 Hz visual stream: "+visuals);
   if(gun==7&&(boltPeak==0||!sounds.contains("BLUE_GUN2_LOOP")))throw new AssertionError("Arc rope missing");
   if(gun==10&&ringPeak==0)throw new AssertionError("Mass Inverter rings missing");
   if(gun!=7&&gun!=10&&(modelFrames==0||stable==0))throw new AssertionError("Shot meshes have no stable lifecycle");
   System.out.println("gun="+gun+" bolts="+boltPeak+" darkRingVertices="+ringPeak+" modelFrames="+modelFrames+" stable="+stable+" maxPacket="+maxBytes+" cues="+sounds);
  }finally{NativeJak.destroy(id);NativeJak.fxDestroy(fx);}
 }
}
