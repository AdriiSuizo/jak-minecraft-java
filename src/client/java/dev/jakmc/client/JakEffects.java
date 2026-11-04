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
import net.minecraft.core.BlockPos;
import net.minecraft.core.SectionPos;
import net.minecraft.world.level.chunk.status.ChunkStatus;
import dev.jakmc.content.JakProps;
import net.minecraft.client.renderer.texture.*;
import net.minecraft.resources.Identifier;
import net.minecraft.world.phys.Vec3;
import org.joml.Vector3f;

/** Launch commands cross the network; original physics, axes, UVs and blending run locally. */
public final class JakEffects {
 private static final Map<Integer,Owner> OWNERS=new HashMap<>();
 private record Decoration(BlockPos pos,int kind){}
 private static final Map<Decoration,Owner> DECORATIONS=new HashMap<>();
 private static final List<Owner> BURSTS=new ArrayList<>();
 private static final int BARREL=0,PROJECTOR=1,DISMANTLE=2,SANDSTORM=18,SPYDROID=19;
 private static Owner sandstorm;
 private static net.minecraft.client.multiplayer.ClientLevel weatherWorld;
 private static long lastDust;
 private static final Map<Integer,Identifier> TEXTURES=new HashMap<>();
 private static final Set<Integer> MISSING=new HashSet<>();
 private static String[] names;
 private static final ByteBuffer VIEW=NativeJak.buffer(56),SPRITES=NativeJak.buffer(8192*44),BOLTS=NativeJak.buffer(32768*44);
 public static long gemCollectBursts,spydroidBursts;
 public static int drawnVertices,drawnLightningVertices,drawnDarkVertices,drawnBarrelVertices,drawnProjectorVertices,drawnBurstVertices,drawnVentVertices,drawnSandVertices;
 private static final class Owner implements AutoCloseable {
  final long handle;final Vec3 origin;long received=System.nanoTime(),lastDraw;
  Vec3 boardSocket;
  int collector=-1;long pickupAt=Long.MIN_VALUE;boolean pickupStart;
  Owner(Vec3 origin,long handle){this.origin=origin;this.handle=handle;}
  public void close(){NativeJak.fxDestroy(handle);}
 }
 private static final class Batch {
  float[] data=new float[256];int used;
  void vertex(ByteBuffer buffer,int at){if(used+9>data.length)data=Arrays.copyOf(data,data.length*2);for(int i=0;i<9;i++)data[used++]=buffer.getFloat(at+i*4);}
 }
 public static void initialize(){
  JakEffectTypes.initialize();
  LevelRenderEvents.COLLECT_SUBMITS.register(JakEffects::submit);
  net.fabricmc.fabric.api.client.event.lifecycle.v1.ClientTickEvents.END_CLIENT_TICK.register(JakEffects::findDecorations);
  net.fabricmc.fabric.api.client.networking.v1.ClientPlayNetworking.registerGlobalReceiver(
   dev.jakmc.network.JakPackets.PropBreak.TYPE,(packet,context)->{
    var level=context.client().level;
    if(level==null||!level.dimension().identifier().equals(packet.dimension()))return;
    var owner=DECORATIONS.remove(new Decoration(packet.position(),PROJECTOR));
    if(owner!=null)owner.close();
    long handle=NativeJak.fxCreateDecor(DISMANTLE);
    if(handle!=0)BURSTS.add(new Owner(Vec3.atBottomCenterOf(packet.position()).add(0,.6,0),handle));
   });
  net.fabricmc.fabric.api.client.networking.v1.ClientPlayNetworking.registerGlobalReceiver(
   dev.jakmc.network.JakPackets.CreatureBurst.TYPE,(packet,context)->{
    var level=context.client().level;
    if(level==null||!level.dimension().identifier().equals(packet.dimension()))return;
    long handle=NativeJak.fxCreateDecor(SPYDROID);
    if(handle!=0){BURSTS.add(new Owner(packet.at(),handle));spydroidBursts++;}
   });
  ClientPlayConnectionEvents.DISCONNECT.register((handler,client)->clear());
 }
 // Entity submits precede COLLECT_SUBMITS. Consume the same frame's visible joint once.
 public static void boardSocket(int entity,Vec3 position){
  var owner=OWNERS.get(entity);
  if(owner!=null)owner.boardSocket=position.subtract(owner.origin);
 }
 public static void receive(int entity,Vec3 origin,JsonObject packet){
  if(!packet.has("effects"))return;
  var fx=OWNERS.get(entity);
  if(fx!=null&&fx.origin.distanceToSqr(origin)>4096*4096){fx.close();OWNERS.remove(entity);fx=null;}
  if(fx==null){fx=new Owner(origin,NativeJak.fxCreate());OWNERS.put(entity,fx);}
  Vec3 delta=origin.subtract(fx.origin);NativeJak.fxPush(fx.handle,packet.get("effects").toString(),(float)delta.x,(float)delta.y,(float)delta.z);
  fx.received=System.nanoTime();
 }
 public static void local(int entity,Vec3 origin,JsonArray commands){JsonObject packet=new JsonObject();packet.add("effects",commands);receive(entity,origin,packet);}
 public static void clear(){closeSandstorm();weatherWorld=null;for(var fx:OWNERS.values())fx.close();OWNERS.clear();for(var fx:DECORATIONS.values())fx.close();DECORATIONS.clear();for(var fx:BURSTS)fx.close();BURSTS.clear();JakPoses.clear();dev.jakmc.client.model.EcoItemRenderer.clear();}
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
  boolean jak2=name.startsWith("jak2:");
  if(jak2){
   root=RuntimeConfig.assets.getParent().getParent().getParent().getParent()
       .resolve("jak2/data/decompiler_out/jak2/textures");
   name=name.substring(5);
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
  if(jak2&&!Files.isRegularFile(file)){
   String override=System.getenv("JAKMC_JAK2_TEXTURES");
   String local=System.getenv("LOCALAPPDATA");
   if(override!=null)file=Path.of(override,page,name+".png");
   if(!Files.isRegularFile(file)&&local!=null)
    file=Path.of(local,"iw4l-mashup/jak2-city-effects/decompiler_out/jak2/textures",page,name+".png");
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
 public static int submitGeometry(ByteBuffer vertices,int count,ByteBuffer lightning,int boltCount,com.mojang.blaze3d.vertex.PoseStack stack,net.minecraft.client.renderer.SubmitNodeCollector collector){return submitGeometry(vertices,count,lightning,boltCount,stack,collector,false);}
 public static int submitGeometry(ByteBuffer vertices,int count,ByteBuffer lightning,int boltCount,com.mojang.blaze3d.vertex.PoseStack stack,net.minecraft.client.renderer.SubmitNodeCollector collector,boolean gui){
  manifest();Map<Integer,Batch> batches=new LinkedHashMap<>();collect(vertices,count,batches);collect(lightning,boltCount,batches);int drawn=0;
  for(var group:batches.entrySet()){
   Identifier id=texture(group.getKey()>>>2);if(id==null)continue;int blend=group.getKey()&3;Batch batch=group.getValue();float[] data=Arrays.copyOf(batch.data,batch.used);drawn+=batch.used/9;
   collector.submitCustomGeometry(stack,gui?JakEffectTypes.getGui(id,blend):JakEffectTypes.get(id,blend),(pose,consumer)->{
    Vector3f v=new Vector3f();for(int i=0;i<data.length;i+=9){pose.pose().transformPosition(data[i],data[i+1],data[i+2],v);consumer.addVertex(v.x,v.y,v.z,pack(data[i+3],data[i+4],data[i+5],data[i+6]),data[i+7],data[i+8],OverlayTexture.NO_OVERLAY,0x00f000f0,0,1,0);}
   });
  }return drawn;
 }
 private static void findDecorations(Minecraft client){
  if(client.level==null||client.player==null||client.level.getGameTime()%5!=0)return;
  var focus=client.gameRenderer.mainCamera().position();
  var center=BlockPos.containing(focus);
  int minX=SectionPos.blockToSectionCoord(center.getX()-36),maxX=SectionPos.blockToSectionCoord(center.getX()+36);
  int minZ=SectionPos.blockToSectionCoord(center.getZ()-36),maxZ=SectionPos.blockToSectionCoord(center.getZ()+36);
  Set<Decoration> visible=new HashSet<>();
  for(int x=minX;x<=maxX;x++)for(int z=minZ;z<=maxZ;z++){
   var chunk=client.level.getChunkSource().getChunk(x,z,ChunkStatus.FULL,false);
   if(chunk==null)continue;
   for(var entry:chunk.getBlockEntities().entrySet()){
    var pos=entry.getKey();
    int kind,range;
    if(entry.getValue() instanceof JakProps.FireBarrel){kind=BARREL;range=24;}
    else if(entry.getValue() instanceof JakProps.Projector){kind=PROJECTOR;range=36;}
    else if(entry.getValue() instanceof dev.jakmc.content.JakEcoVent.Vent vent&&vent.open()){
     int color=vent.color().ordinal();
     kind=10+color+(vent.color().model()&&vent.touched()?2:0);
     range=24;
     JakSounds.vent(pos,vent.color(),focus);
    }else continue;
    if(focus.distanceToSqr(Vec3.atCenterOf(pos))>range*range)continue;
    var key=new Decoration(pos,kind);
    visible.add(key);
    if(kind==BARREL)JakSounds.barrel(pos,focus);
    var fx=DECORATIONS.computeIfAbsent(key,decoration->{
     long handle=NativeJak.fxCreateDecor(decoration.kind());
     return handle==0?null:new Owner(Vec3.atBottomCenterOf(pos).add(0,kind==BARREL?.94:kind==PROJECTOR?1.01:.30,0),handle);
    });
    if(fx!=null&&kind>=10&&kind<=13&&entry.getValue() instanceof dev.jakmc.content.JakEcoVent.Vent vent
        &&vent.pickupEntity>=0&&vent.pickupTime!=fx.pickupAt&&client.level.getGameTime()-vent.pickupTime<=15){
     fx.collector=vent.pickupEntity;fx.pickupAt=vent.pickupTime;fx.pickupStart=true;
    }
   }
  }
  DECORATIONS.entrySet().removeIf(entry->{
   if(visible.contains(entry.getKey()))return false;
   entry.getValue().close();
   return true;
  });
 }
 private static void submit(LevelRenderContext context){
  drawnVertices=0;drawnLightningVertices=0;drawnDarkVertices=0;drawnBarrelVertices=0;drawnProjectorVertices=0;drawnBurstVertices=0;drawnVentVertices=0;
  drawnSandVertices=0;
  var client=Minecraft.getInstance();
  if(client.level!=weatherWorld){closeSandstorm();weatherWorld=client.level;}
  float dust=JakAtmosphere.dust();
  if(client.level==null||OWNERS.isEmpty()&&DECORATIONS.isEmpty()&&BURSTS.isEmpty()&&sandstorm==null&&dust<=.01f)return;
  manifest();
  var camera=context.levelState().cameraRenderState;long now=System.nanoTime();drawnVertices=0;
  Vector3f forward=camera.orientation.transform(new Vector3f(0,0,-1)),up=camera.orientation.transform(new Vector3f(0,1,0));
  if(dust>.01f){
   lastDust=now;
   if(sandstorm!=null&&sandstorm.origin.distanceToSqr(camera.pos)>80*80)closeSandstorm();
   if(sandstorm==null){
    long handle=NativeJak.fxCreateDecor(SANDSTORM);
    if(handle!=0)sandstorm=new Owner(camera.pos,handle);
   }
  }
  if(sandstorm!=null){
   if(now-lastDust>6_000_000_000L)closeSandstorm();
   else{
    NativeJak.fxSandstorm(sandstorm.handle,dust);
    drawOwner(context,client,sandstorm,forward,up,now);
    drawnSandVertices=drawnVertices;
   }
  }
  var it=OWNERS.entrySet().iterator();
  while(it.hasNext()){
   var entry=it.next();Owner fx=entry.getValue();
   if(client.level.getEntity(entry.getKey())==null||now-fx.received>2_000_000_000L){fx.close();it.remove();continue;}
   drawOwner(context,client,fx,forward,up,now);
  }
  for(var entry:DECORATIONS.entrySet()){
   int before=drawnVertices;
   var fx=entry.getValue();int kind=entry.getKey().kind();
   if(fx.collector>=0){
    var collector=client.level.getEntity(fx.collector);
    if(collector!=null&&client.level.getGameTime()-fx.pickupAt<=20){
     Vec3 position=collector==client.player?JakPoses.presented():null;
     if(position==null)position=collector.position();
     var target=position.add(0,collector.getBbHeight()*.57,0).subtract(fx.origin);
     NativeJak.fxVentPickup(fx.handle,(float)target.x,(float)target.y,(float)target.z,kind-10,fx.pickupStart);
     fx.pickupStart=false;
    }else fx.collector=-1;
   }
   drawOwner(context,client,fx,forward,up,now);
   if(kind==BARREL)drawnBarrelVertices+=drawnVertices-before;
   else if(kind==PROJECTOR)drawnProjectorVertices+=drawnVertices-before;
   else drawnVentVertices+=drawnVertices-before;
  }
  BURSTS.removeIf(fx->{
   if(now-fx.received>6_000_000_000L){fx.close();return true;}
   int before=drawnVertices;
   drawOwner(context,client,fx,forward,up,now);
   drawnBurstVertices+=drawnVertices-before;
   return false;
  });
 }
 private static void closeSandstorm(){if(sandstorm!=null){sandstorm.close();sandstorm=null;}}
 private static void drawOwner(LevelRenderContext context,Minecraft client,Owner fx,Vector3f forward,Vector3f up,long now){
  var camera=context.levelState().cameraRenderState;
  Vec3 eye=camera.pos.subtract(fx.origin);
  VIEW.clear();VIEW.putFloat((float)eye.x).putFloat((float)eye.y).putFloat((float)eye.z);
  VIEW.putFloat(forward.x).putFloat(forward.y).putFloat(forward.z).putFloat(up.x).putFloat(up.y).putFloat(up.z);
  VIEW.putFloat(client.options.fov().get().floatValue()).putFloat(client.getWindow().getHeight());
  Vec3 board=fx.boardSocket;
  VIEW.putFloat(board==null?Float.NaN:(float)board.x)
      .putFloat(board==null?Float.NaN:(float)board.y)
      .putFloat(board==null?Float.NaN:(float)board.z);
  fx.boardSocket=null;
  float dt=client.isPaused()?0:fx.lastDraw==0?1/60f:Math.clamp((now-fx.lastDraw)/1e9f,0,.1f);fx.lastDraw=now;
  long counts=NativeJak.fxDraw(fx.handle,dt,VIEW,SPRITES,BOLTS);
  drawnLightningVertices+=(int)(counts>>>32);
  for(int i=0;i<(int)counts;i++){int at=i*44;if(SPRITES.getFloat(at+12)==0&&SPRITES.getFloat(at+16)==0&&SPRITES.getFloat(at+20)==0&&SPRITES.getFloat(at+24)>0)drawnDarkVertices++;}
  var stack=context.poseStack();
  stack.pushPose();
  stack.translate(-eye.x,-eye.y,-eye.z);
  drawnVertices+=submitGeometry(SPRITES,(int)counts,BOLTS,(int)(counts>>>32),stack,context.submitNodeCollector());
  stack.popPose();
 }
 private static void collect(ByteBuffer buffer,int count,Map<Integer,Batch> batches){
  for(int i=0;i<count;i++){int at=i*44,key=(buffer.getInt(at+36)<<2)|buffer.getInt(at+40);batches.computeIfAbsent(key,k->new Batch()).vertex(buffer,at);}
 }
 private static int pack(float r,float g,float b,float a){return (channel(a)<<24)|(channel(r)<<16)|(channel(g)<<8)|channel(b);}
 private static int channel(float v){return Math.clamp(Math.round(v*255),0,255);}
}
