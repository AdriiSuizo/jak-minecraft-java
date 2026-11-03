package dev.jakmc.client;
import com.google.gson.*;
import dev.jakmc.nativebridge.MinecraftCollision;
import net.fabricmc.fabric.api.client.event.lifecycle.v1.ClientTickEvents;
import net.minecraft.client.*;
import net.minecraft.world.phys.Vec3;
import java.nio.file.*;
import java.util.*;
import java.util.zip.CRC32;
/** Bounded measurements on the live world, no block or inventory changes. */
public final class CollisionPerfScene {
 private static boolean running;private static int tick;private static long nanos,allocated,queries,bytes;private static MinecraftCollision.Query query;private static Vec3 origin;private static Path out;private static final Set<Long> ADDRESSES=new HashSet<>();private static final Map<Integer,Long> CRC=new TreeMap<>();private static String failure;
 public static void initialize(){ClientTickEvents.END_CLIENT_TICK.register(CollisionPerfScene::tick);}
 public static void start(Minecraft c){if(running||c.player==null||c.getSingleplayerServer()==null)throw new IllegalStateException("Open dev world");tick=0;nanos=allocated=queries=bytes=0;ADDRESSES.clear();CRC.clear();failure=null;query=null;running=true;out=c.gameDirectory.toPath().resolve("jakmc/evidence/collision_perf");try{Files.createDirectories(out);}catch(Exception e){throw new RuntimeException(e);}c.options.setCameraType(CameraType.THIRD_PERSON_BACK);}
 private static void tick(Minecraft c){if(!running)return;if(c.player==null||c.getSingleplayerServer()==null){running=false;c.stop();return;}int t=++tick;var id=c.player.getUUID();c.getSingleplayerServer().execute(()->{var p=c.getSingleplayerServer().getPlayerList().getPlayer(id);if(p==null)return;if(query==null){var b=p.blockPosition();origin=new Vec3(b.getX(),b.getY(),b.getZ());query=new MinecraftCollision.Query(p.level(),p,origin);}
  var bean=(com.sun.management.ThreadMXBean)java.lang.management.ManagementFactory.getThreadMXBean();
  for(int i=0;i<6;i++){float x=(i%3-1)*2,z=(i/3)*2;long a=bean.getCurrentThreadAllocatedBytes(),n=System.nanoTime();var data=query.triangles(x-2,-2,z-2,x+2,3,z+2);long ns=System.nanoTime()-n,alloc=bean.getCurrentThreadAllocatedBytes()-a;if(t>15){nanos+=ns;allocated+=alloc;queries++;bytes+=data.capacity();ADDRESSES.add(org.lwjgl.system.MemoryUtil.memAddress(data));}CRC32 crc=new CRC32();crc.update(data.duplicate().clear());long value=crc.getValue();Long old=CRC.putIfAbsent(i,value);if(old!=null&&old!=value)failure="World/shape bytes changed for region "+i;if(query.triangles(x-2,-2,z-2,x+2,3,z+2)!=data)failure="Same-tick cache miss";
  }
  if(t==70){JsonObject r=new JsonObject();r.addProperty("reason",failure==null?"complete":failure);r.addProperty("queries",queries);r.addProperty("ns_per_query",nanos/(double)queries);r.addProperty("java_bytes_per_query",allocated/(double)queries);r.addProperty("triangle_bytes",bytes);r.addProperty("distinct_native_addresses",ADDRESSES.size());r.addProperty("origin",origin.toString());r.add("crc_by_region",new Gson().toJsonTree(CRC));try{Files.writeString(out.resolve("result.json"),r.toString());}catch(Exception e){throw new RuntimeException(e);}}
 });if(t==60)Screenshot.grab(c.gameDirectory,"collision-perf.png",c.gameRenderer.mainRenderTarget(),1,m->{});if(t>=76){running=false;c.stop();}}
}
