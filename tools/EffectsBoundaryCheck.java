import dev.jakmc.nativebridge.NativeJak;
import java.nio.file.*;
/** Real owned definition -> replay -> reusable draw buffers. No game window. */
public final class EffectsBoundaryCheck {
 public static void main(String[] args)throws Exception{
  NativeJak.load(Path.of(args[0]));NativeJak.configure(args[1]);Files.writeString(Path.of(args[2]),NativeJak.fxManifest());
  var view=NativeJak.buffer(44);view.putFloat(0).putFloat(2).putFloat(4).putFloat(0).putFloat(0).putFloat(-1).putFloat(0).putFloat(1).putFloat(0).putFloat(70).putFloat(720);
  var sprites=NativeJak.buffer(8192*44);var bolts=NativeJak.buffer(32768*44);long fx=NativeJak.fxCreate();
  NativeJak.fxPush(fx,"[{\"Keep\":[1,\"group-eco-light-pill\",[0,0,0],[1,0,0,0,1,0,0,0,1]]}]",0,0,0);
  int maximum=0;for(int i=0;i<60;i++){long count=NativeJak.fxDraw(fx,1/60f,view,sprites,bolts);maximum=Math.max(maximum,(int)count);}
  if(maximum<4)throw new AssertionError("Original light eco emitted no rendered geometry");
  for(int i=0;i<maximum*44;i+=4)if(i%44<36&&!Float.isFinite(sprites.getFloat(i)))throw new AssertionError("Nonfinite particle vertex");
  StringBuilder backlog=new StringBuilder("[");for(int i=0;i<90;i++){if(i>0)backlog.append(',');backlog.append("{\"Step\":0.016666667}");}backlog.append(']');
  NativeJak.fxPush(fx,backlog.toString(),0,0,0);if(NativeJak.fxBacklog(fx)!=90)throw new AssertionError("Stall fixture did not queue");NativeJak.fxDraw(fx,1/60f,view,sprites,bolts);
  if(NativeJak.fxBacklog(fx)>3)throw new AssertionError("Effects remain delayed after render stall");
  NativeJak.fxDestroy(fx);boolean rejected=false;try{NativeJak.fxDraw(fx,1/60f,view,sprites,bolts);}catch(IllegalStateException expected){rejected=true;}
  if(!rejected)throw new AssertionError("Released effects session accepted");
  System.out.println("PASS: original eco group emits "+maximum+" vertices; 90-frame render stall catches up to <= 3 frames; released handle rejected");
 }
}
