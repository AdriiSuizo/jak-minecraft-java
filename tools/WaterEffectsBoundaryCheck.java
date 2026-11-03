import dev.jakmc.nativebridge.NativeJak;import java.nio.file.*;
public class WaterEffectsBoundaryCheck {
 public static void main(String[] args)throws Exception{
  NativeJak.load(Path.of(args[0]));NativeJak.configure(args[1]);long id=NativeJak.fxCreate();
  var view=NativeJak.buffer(44);view.putFloat(0).putFloat(3).putFloat(4).putFloat(0).putFloat(-.2f).putFloat(-1).putFloat(0).putFloat(1).putFloat(0).putFloat(70).putFloat(720);var vertices=NativeJak.buffer(8192*44);var bolts=NativeJak.buffer(32768*44);
  NativeJak.fxPush(id,"[{\"Tracker\":[\"part-water-splash-callback\",0.2,[0,6400,0]]},{\"Spawn\":[\"group-part-water-splash\",[0,6400,0],[0.861,0,0,0,0.861,0,0,0,0.861]]},{\"Value\":[756,\"userdata\",6400]},{\"Launch\":[756,[0,4000,0],[1,0,0,0,1,0,0,0,1]]}]",0,0,0);
  int maximum=0;for(int i=0;i<120;i++){long n=NativeJak.fxDraw(id,1/60f,view,vertices,bolts);int count=(int)n;maximum=Math.max(maximum,count);for(int a=0;a<count*44;a+=44)for(int k=0;k<9;k++)if(!Float.isFinite(vertices.getFloat(a+k*4)))throw new AssertionError("Invalid original water geometry");}
  NativeJak.fxDestroy(id);if(maximum<4)throw new AssertionError("No original water splash geometry");System.out.println("PASS: original water tracker/splash/bubble commands produce finite geometry; peak "+maximum+" vertices");
 }
}
