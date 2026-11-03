import dev.jakmc.content.GemMotion;
import dev.jakmc.client.model.SkullGemTexture;
import java.nio.file.*;
import java.nio.*;
import java.security.*;
public final class GemCoreCheck {
 public static void main(String[] args)throws Exception{
  var m=new GemMotion(.5,1);double unit=.55;var b=m.bounce(new double[]{unit*.1,-unit*.5,0},new boolean[]{false,true,false},.5,.5,unit);
  if(Math.abs(b[0]/unit-.07)>1e-9||Math.abs(b[1]/unit-.5*.9*.725)>1e-9||m.bounces!=1||m.spin!=720)throw new AssertionError("Original reflection/damping changed");
  m.seconds=10.1;m.bounce(b,new boolean[]{false,true,false},.5,.5,unit);if(!m.settled)throw new AssertionError("Unbounded bounce");
  var s=new GemMotion(.5,-1);double[] pos={4,1,3},target={0,0,0};double last=100;for(int i=0;i<100;i++){var d=s.suck(pos,target,unit);for(int k=0;k<3;k++)pos[k]+=d[k];double dist=Math.sqrt(pos[0]*pos[0]+pos[1]*pos[1]+pos[2]*pos[2]);if(dist>last+1e-8||!Double.isFinite(dist))throw new AssertionError("Suction diverged");last=dist;}if(last>1e-6)throw new AssertionError("Suction never arrives");
  int[] pixels=SkullGemTexture.bake(Path.of(args[0]));var bytes=ByteBuffer.allocate(pixels.length*4);for(int p:pixels)bytes.putInt(p);Files.write(Path.of(args[1]),bytes.array());if(pixels.length!=64*64*60)throw new AssertionError("Atlas dimensions");int changed=0;for(int i=0;i<4096;i++)if(pixels[i]!=pixels[30*4096+i])changed++;if(changed<2000)throw new AssertionError("Material is static");System.out.println("Original bounce/suction passed; 60 material frames; changed pixels="+changed+"; SHA256="+java.util.HexFormat.of().formatHex(MessageDigest.getInstance("SHA-256").digest(bytes.array())));
 }
}
