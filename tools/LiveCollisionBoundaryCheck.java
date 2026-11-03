import dev.jakmc.nativebridge.NativeJak;
import com.google.gson.*;
import java.nio.*;
import java.nio.file.*;
/** Exercise the real JNI -> Java world query beyond the former near-player mesh. */
public final class LiveCollisionBoundaryCheck {
 public static final class World {
  final ByteBuffer mesh=NativeJak.buffer(160);float furthest;boolean fail;
  World(){
   NativeBoundaryCheck.triangle(mesh,-50,0,-50,-50,0,50,50,0,-50);
   NativeBoundaryCheck.triangle(mesh,50,0,-50,-50,0,50,50,0,50);
   NativeBoundaryCheck.triangle(mesh,-10,0,12,10,8,12,10,0,12);
   NativeBoundaryCheck.triangle(mesh,-10,0,12,-10,8,12,10,8,12);
  }
  public ByteBuffer triangles(float x,float y,float z,float X,float Y,float Z){furthest=Math.max(furthest,Z);if(fail)throw new IllegalStateException("Deliberate query failure");return mesh;}
 }
 public static void main(String[] args){
  NativeJak.load(Path.of(args[0]));NativeJak.configure(args[1]);long id=NativeJak.create(0);World world=new World();NativeJak.bindWorld(id,world);
  var input=NativeJak.buffer(64);input.putFloat(44,1).putInt(48,32).putInt(60,3);var empty=NativeJak.buffer(0);int hits=0;
  for(int i=0;i<80;i++){
   input.putInt(0,i>15?64:0).putInt(4,i==18?64:0);
   var pose=JsonParser.parseString(NativeJak.step(id,input,empty,empty,3)).getAsJsonObject();
   for(var value:pose.getAsJsonArray("events")){var e=value.getAsJsonObject();if(e.get("kind").getAsString().equals("impact")&&e.has("pos")&&Math.abs(e.getAsJsonArray("pos").get(2).getAsFloat()-12)<.1)hits++;}
  }
  if(hits<1||world.furthest<=6)throw new AssertionError("Distant wall not queried/hit: "+hits+" / "+world.furthest);
  world.fail=true;boolean rejected=false;try{NativeJak.step(id,input,empty,empty,3);}catch(IllegalArgumentException expected){rejected=true;}
  NativeJak.destroy(id);if(!rejected)throw new AssertionError("Java collision failure was swallowed");
  long next=NativeJak.create(0);NativeJak.destroy(next);
  System.out.println("PASS: distant wall hits="+hits+", queries reached "+world.furthest+" blocks; host exception contained and next session works");
 }
}
