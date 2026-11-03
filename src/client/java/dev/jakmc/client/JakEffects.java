package dev.jakmc.client;

import com.google.gson.*;
import com.mojang.blaze3d.platform.NativeImage;
import dev.jakmc.*;
import dev.jakmc.nativebridge.NativeJak;
import java.nio.*;
import java.nio.file.*;
import java.io.*;
import java.util.*;
import net.fabricmc.fabric.api.client.networking.v1.ClientPlayConnectionEvents;
import net.fabricmc.fabric.api.client.rendering.v1.level.*;
import net.minecraft.client.Minecraft;
import net.minecraft.client.renderer.texture.*;
import net.minecraft.resources.Identifier;
import net.minecraft.world.phys.Vec3;
import org.joml.Vector3f;

/** Launch commands cross the network; original physics, axes, UVs and blending run locally. */
public final class JakEffects {
 private static final Map<Integer,Owner> OWNERS=new HashMap<>();
 private static final Map<Integer,Identifier> TEXTURES=new HashMap<>();
 private static final Set<Integer> MISSING=new HashSet<>();
 private static String[] names;
 private static final ByteBuffer VIEW=NativeJak.buffer(44),SPRITES=NativeJak.buffer(8192*44),BOLTS=NativeJak.buffer(32768*44);
 public static long gemCollectBursts;
 public static int drawnVertices,drawnLightningVertices,drawnDarkVertices;
 private static final class Owner implements AutoCloseable {
  final long handle=NativeJak.fxCreate();final Vec3 origin;long received=System.nanoTime(),lastDraw;
  Owner(Vec3 origin){this.origin=origin;}
  public void close(){NativeJak.fxDestroy(handle);}
 }
 private static final class Batch {
  float[] data=new float[256];int used;
  void vertex(ByteBuffer buffer,int at){if(used+9>data.length)data=Arrays.copyOf(data,data.length*2);for(int i=0;i<9;i++)data[used++]=buffer.getFloat(at+i*4);}
 }
 public static void initialize(){
  JakEffectTypes.initialize();
  LevelRenderEvents.COLLECT_SUBMITS.register(JakEffects::submit);
  ClientPlayConnectionEvents.DISCONNECT.register((handler,client)->clear());
 }
 public static void receive(int entity,Vec3 origin,JsonObject packet){
  if(!packet.has("effects"))return;
  var fx=OWNERS.get(entity);
  if(fx!=null&&fx.origin.distanceToSqr(origin)>4096*4096){fx.close();OWNERS.remove(entity);fx=null;}
  if(fx==null){fx=new Owner(origin);OWNERS.put(entity,fx);}
  Vec3 delta=origin.subtract(fx.origin);NativeJak.fxPush(fx.handle,packet.get("effects").toString(),(float)delta.x,(float)delta.y,(float)delta.z);
  fx.received=System.nanoTime();
 }
 public static void local(int entity,Vec3 origin,JsonArray commands){JsonObject packet=new JsonObject();packet.add("effects",commands);receive(entity,origin,packet);}
 public static void clear(){for(var fx:OWNERS.values())fx.close();OWNERS.clear();JakPoses.clear();dev.jakmc.client.model.EcoItemRenderer.clear();}
 private static Identifier texture(int index){
  if(TEXTURES.containsKey(index))return TEXTURES.get(index);if(MISSING.contains(index))return null;
  String name=names[index];
  Path root=RuntimeConfig.assets.resolve("textures");
  String page="level-default-sprite";
  boolean jak1=name.startsWith("jak1:");
  if(jak1){
   root=RuntimeConfig.assets.getParent().getParent().getParent().getParent()
       .resolve("jak1/data/decompiler_out/jak1/textures");
   name=name.substring(5);
   page="effects";
  }
  if(name.contains("/")){int split=name.indexOf('/');page=name.substring(0,split);name=name.substring(split+1);}
  Path file=root.resolve(page).resolve(name+".png");
  if(jak1&&!Files.isRegularFile(file)){
   String override=System.getenv("IW4L_JAK1_TEXTURES");
   String local=System.getenv("LOCALAPPDATA");
   if(override!=null)file=Path.of(override,page,name+".png");
   if(!Files.isRegularFile(file)&&local!=null){
    file=Path.of(local,"iw4l-mashup/jak1-proj/decompiler_out/jak1/textures",page,name+".png");
   }
  }
  if(!Files.isRegularFile(file)){
   MISSING.add(index);JakMod.LOG.error("Original Jak particle texture unavailable: {}",file);return null;
  }
  Identifier id=JakData.id("local/particles/"+index);String label=id.toString();
  try(var in=Files.newInputStream(file)){
   // The shader understands the original PS2 alpha range, so do not normalize it twice.
   Minecraft.getInstance().getTextureManager().register(id,new DynamicTexture(()->label,NativeImage.read(in)));
   TEXTURES.put(index,id);return id;
  }catch(IOException e){throw new UncheckedIOException("Cannot read original particle texture "+file,e);}
 }
 private static void manifest(){if(names==null){var manifest=JsonParser.parseString(NativeJak.fxManifest()).getAsJsonObject();names=new String[manifest.getAsJsonArray("textures").size()];for(int i=0;i<names.length;i++)names[i]=manifest.getAsJsonArray("textures").get(i).getAsString();JakMod.LOG.info("Original sparticles: {} parts, {} groups, {} textures",manifest.get("parts"),manifest.get("groups"),names.length);}}
 public static int submitGeometry(ByteBuffer vertices,int count,ByteBuffer lightning,int boltCount,com.mojang.blaze3d.vertex.PoseStack stack,net.minecraft.client.renderer.SubmitNodeCollector collector){
  manifest();Map<Integer,Batch> batches=new LinkedHashMap<>();collect(vertices,count,batches);collect(lightning,boltCount,batches);int drawn=0;
  for(var group:batches.entrySet()){
   Identifier id=texture(group.getKey()>>>2);if(id==null)continue;int blend=group.getKey()&3;Batch batch=group.getValue();float[] data=Arrays.copyOf(batch.data,batch.used);drawn+=batch.used/9;
   collector.submitCustomGeometry(stack,JakEffectTypes.get(id,blend),(pose,consumer)->{
    Vector3f v=new Vector3f();for(int i=0;i<data.length;i+=9){pose.pose().transformPosition(data[i],data[i+1],data[i+2],v);consumer.addVertex(v.x,v.y,v.z,pack(data[i+3],data[i+4],data[i+5],data[i+6]),data[i+7],data[i+8],OverlayTexture.NO_OVERLAY,0x00f000f0,0,1,0);}
   });
  }return drawn;
 }
 private static void submit(LevelRenderContext context){
  drawnVertices=0;drawnLightningVertices=0;drawnDarkVertices=0;
  var client=Minecraft.getInstance();if(client.level==null||OWNERS.isEmpty())return;
  manifest();
  var camera=context.levelState().cameraRenderState;long now=System.nanoTime();drawnVertices=0;
  Vector3f forward=camera.orientation.transform(new Vector3f(0,0,-1)),up=camera.orientation.transform(new Vector3f(0,1,0));
  var it=OWNERS.entrySet().iterator();
  while(it.hasNext()){
   var entry=it.next();Owner fx=entry.getValue();
   if(client.level.getEntity(entry.getKey())==null||now-fx.received>2_000_000_000L){fx.close();it.remove();continue;}
   Vec3 eye=camera.pos.subtract(fx.origin);
   VIEW.clear();VIEW.putFloat((float)eye.x).putFloat((float)eye.y).putFloat((float)eye.z);
   VIEW.putFloat(forward.x).putFloat(forward.y).putFloat(forward.z).putFloat(up.x).putFloat(up.y).putFloat(up.z);
   VIEW.putFloat(client.options.fov().get().floatValue()).putFloat(client.getWindow().getHeight());
   float dt=client.isPaused()?0:fx.lastDraw==0?1/60f:Math.clamp((now-fx.lastDraw)/1e9f,0,.1f);fx.lastDraw=now;
   long counts=NativeJak.fxDraw(fx.handle,dt,VIEW,SPRITES,BOLTS);
   drawnLightningVertices+=(int)(counts>>>32);
   for(int i=0;i<(int)counts;i++){int at=i*44;if(SPRITES.getFloat(at+12)==0&&SPRITES.getFloat(at+16)==0&&SPRITES.getFloat(at+20)==0&&SPRITES.getFloat(at+24)>0)drawnDarkVertices++;}
   Map<Integer,Batch> batches=new LinkedHashMap<>();
   collect(SPRITES,(int)counts,batches);collect(BOLTS,(int)(counts>>>32),batches);
   var stack=context.poseStack();stack.pushPose();stack.translate(fx.origin.x-camera.pos.x,fx.origin.y-camera.pos.y,fx.origin.z-camera.pos.z);
   for(var group:batches.entrySet()){
    int tex=group.getKey()>>>2,blend=group.getKey()&3;Identifier id=texture(tex);if(id==null)continue;
    Batch batch=group.getValue();float[] data=Arrays.copyOf(batch.data,batch.used);drawnVertices+=batch.used/9;
    context.submitNodeCollector().submitCustomGeometry(stack,JakEffectTypes.get(id,blend),(pose,consumer)->{
     Vector3f v=new Vector3f();for(int i=0;i<data.length;i+=9){pose.pose().transformPosition(data[i],data[i+1],data[i+2],v);int color=pack(data[i+3],data[i+4],data[i+5],data[i+6]);
      consumer.addVertex(v.x,v.y,v.z,color,data[i+7],data[i+8],OverlayTexture.NO_OVERLAY,0x00f000f0,0,1,0);
     }
    });
   }stack.popPose();
  }
 }
 private static void collect(ByteBuffer buffer,int count,Map<Integer,Batch> batches){
  for(int i=0;i<count;i++){int at=i*44,key=(buffer.getInt(at+36)<<2)|buffer.getInt(at+40);batches.computeIfAbsent(key,k->new Batch()).vertex(buffer,at);}
 }
 private static int pack(float r,float g,float b,float a){return (channel(a)<<24)|(channel(r)<<16)|(channel(g)<<8)|channel(b);}
 private static int channel(float v){return Math.clamp(Math.round(v*255),0,255);}
}
