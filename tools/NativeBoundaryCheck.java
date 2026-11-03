import dev.jakmc.nativebridge.NativeJak;
import java.nio.ByteBuffer;
import java.nio.file.Path;

/** Runs against the real JVM/DLL boundary; deliberately includes invalid packets. */
public final class NativeBoundaryCheck {
    static void triangle(ByteBuffer b,float... vertices) {
        for(float v:vertices)b.putFloat(v);
        b.putInt(0);
    }
    public static void main(String[] args) {
        NativeJak.load(Path.of(args[0]));
        if(args.length>1)System.out.println("Authored animations: "+NativeJak.configure(args[1]));
        long id=NativeJak.create(0);
        var input=NativeJak.buffer(64);
        input.putFloat(44,1);
        var ground=NativeJak.buffer(80);
        triangle(ground,-50,0,-50,-50,0,50,50,0,-50);
        triangle(ground,50,0,-50,-50,0,50,50,0,50);
        var empty=NativeJak.buffer(0);
        String pose="";
        for(int i=0;i<40;i++)pose=NativeJak.step(id,input,ground,empty,3);
        if (!pose.contains("jakb-stance")) throw new AssertionError("Did not settle on Minecraft floor: "+pose);
        System.out.println("Floor: "+pose);
        input.putInt(0,1).putInt(4,1);
        pose=NativeJak.step(id,input,ground,empty,3);
        if (!pose.contains("Jump") && !pose.contains("jump")) throw new AssertionError("Jump event/animation missing: "+pose);
        System.out.println("Jump: "+pose);
        boolean rejected=false;
        try {NativeJak.step(id,NativeJak.buffer(4),ground,empty,1);}catch(IllegalArgumentException expected){rejected=true;}
        if(!rejected)throw new AssertionError("Accepted truncated packet");
        NativeJak.destroy(id);
        rejected=false;
        try{NativeJak.step(id,input,ground,empty,1);}catch(IllegalArgumentException expected){rejected=true;}
        if(!rejected)throw new AssertionError("Accepted released session");
        System.out.println("PASS: real JNI floor, jump, invalid input, stale handle");
    }
}
