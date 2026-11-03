package dev.jakmc.nativebridge;

import java.nio.ByteBuffer;
import java.nio.ByteOrder;
import java.nio.file.Files;
import java.nio.file.Path;

/** The only JNI surface. Versioned, bounded, little-endian packets. */
public final class NativeJak {
    public static final int ABI = 3;
    private static boolean loaded;
    private NativeJak() { }

    public static synchronized void load(Path path) {
        if (loaded) return;
        if (!Files.isRegularFile(path)) throw new IllegalStateException("Missing Jak native core: " + path);
        System.load(path.toAbsolutePath().toString());
        if (abi() != ABI) throw new IllegalStateException("Incompatible Jak native ABI");
        loaded = true;
    }
    public static ByteBuffer buffer(int bytes) {
        return ByteBuffer.allocateDirect(bytes).order(ByteOrder.LITTLE_ENDIAN);
    }
    public static native int abi();
    public static native int floor(String blockId,String stepSound);
    public static native int configure(String originalExportRoot);
    public static native String animationMetadata(String originalExportRoot,String model,boolean jak1);
    public static native String ridingAnimations();
    public static native boolean mountReady();
    public static native void mountControl(long session,int action,float x,float y,float z,float yaw);
    public static native long create(float minecraftYaw);
    public static native void destroy(long session);
    public static native void rebase(long session,float deltaX,float deltaY,float deltaZ);
    public static native void setGrindRails(long session,ByteBuffer rails);
    public static native void bindWorld(long session,Object collisionQuery);
    public static native void setStats(long session,float healthFraction,float red,float yellow,float blue,float dark);
    public static native String step(long session, ByteBuffer input, ByteBuffer triangles, ByteBuffer actors, int steps);
    public static native long hellcatCreate(Object query,float yaw,float vx,float vy,float vz,float health,int flags,int torpedoes,long key);
    public static native int hellcatFlames(float thrust,int age,ByteBuffer out);
    public static native void hellcatDestroy(long session);
    public static native String hellcatStep(long session,ByteBuffer input,ByteBuffer actors,float dx,float dy,float dz,float hour,int commands,int steps);
    public static native void relocate(long session,float x,float y,float z,float vx,float vy,float vz);
    public static native String fxManifest();
    public static native long fxCreate();
    public static native long fxCreateDecor(int kind);
    public static native void fxDestroy(long session);
    public static native int fxBacklog(long session);
    public static native void fxPush(long session,String commands,float originX,float originY,float originZ);
    public static native long fxDraw(long session,float dt,ByteBuffer view,ByteBuffer sprites,ByteBuffer lightning);
}
