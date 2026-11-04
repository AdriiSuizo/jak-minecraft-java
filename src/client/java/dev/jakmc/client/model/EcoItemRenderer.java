package dev.jakmc.client.model;

import com.mojang.blaze3d.vertex.PoseStack;
import com.mojang.serialization.*;
import com.mojang.serialization.codecs.RecordCodecBuilder;
import dev.jakmc.*;
import dev.jakmc.client.JakEffects;
import dev.jakmc.nativebridge.NativeJak;
import net.minecraft.client.renderer.SubmitNodeCollector;
import net.minecraft.client.renderer.special.*;
import net.minecraft.world.item.ItemStack;
import org.joml.Vector3f;
import org.joml.Vector3fc;
import java.nio.ByteBuffer;
import java.util.function.Consumer;

/** Eco has no static mesh: its body is the game's original particle group. */
public final class EcoItemRenderer implements SpecialModelRenderer<Void> {
 private static final State[] STATES=new State[4];
 private final boolean light,gui;
 private static final class State {
  final long handle=NativeJak.fxCreate();long last;
  final ByteBuffer view=NativeJak.buffer(44),vertices=NativeJak.buffer(8192*44),bolts=NativeJak.buffer(32768*44);
  State(boolean light){
   NativeJak.fxPush(handle,"[{\"Keep\":[1,\""+(light?"group-eco-light-pill":"group-eco-dark-pill")+"\",[0,0,0],[1,0,0,0,1,0,0,0,1]]}]",0,0,0);
   view.putFloat(0).putFloat(0).putFloat(4).putFloat(0).putFloat(0).putFloat(-1).putFloat(0).putFloat(1).putFloat(0).putFloat(45).putFloat(64);
  }
 }
 EcoItemRenderer(boolean light,boolean gui){this.light=light;this.gui=gui;}
 public static void initialize(){SpecialModelRenderers.ID_MAPPER.put(JakData.id("eco"),Unbaked.CODEC);}
 public static void clear(){for(int i=0;i<STATES.length;i++)if(STATES[i]!=null){NativeJak.fxDestroy(STATES[i].handle);STATES[i]=null;}}
 @Override public Void extractArgument(ItemStack item){return null;}
 @Override public void getExtents(Consumer<Vector3fc> out){for(float x:new float[]{.25f,.75f})for(float y:new float[]{.1f,.8f})for(float z:new float[]{.25f,.75f})out.accept(new Vector3f(x,y,z));}
 @Override public void submit(Void argument,PoseStack stack,SubmitNodeCollector collector,int lightCoords,int overlay,boolean foil,int outline){
  int index=(light?1:0)+(gui?2:0);State state=STATES[index];if(state==null)STATES[index]=state=new State(light);
  long now=System.nanoTime();float dt=state.last==0?1/30f:Math.clamp((now-state.last)/1e9f,0,.1f);state.last=now;
  stack.pushPose();stack.translate(.5f,.4f,.5f);float scale=gui?(light?2f:1.25f):light?.25f:.5f;stack.scale(scale,scale,scale);
  var c=net.minecraft.client.Minecraft.getInstance();var inverse=new org.joml.Matrix4f(stack.last().pose()).invert();
  Vector3f eye,forward,up;
  if(gui){eye=new Vector3f(0,0,4);forward=new Vector3f(0,0,-1);up=new Vector3f(0,1,0);}
  else{
   // ItemEntity spins/bobs and the hand rotates the item; undo that complete
   // transform when expressing the real camera in the emitter's local frame.
   eye=inverse.transformPosition(new Vector3f());
   var camera=c.gameRenderer.mainCamera();forward=inverse.transformDirection(new Vector3f(camera.forwardVector())).normalize();up=inverse.transformDirection(new Vector3f(camera.upVector())).normalize();
  }
  state.view.clear();state.view.putFloat(eye.x).putFloat(eye.y).putFloat(eye.z).putFloat(forward.x).putFloat(forward.y).putFloat(forward.z).putFloat(up.x).putFloat(up.y).putFloat(up.z).putFloat(gui?45:c.options.fov().get().floatValue()).putFloat(gui?64:c.getWindow().getHeight());
  long count=NativeJak.fxDraw(state.handle,c.isPaused()?0:dt,state.view,state.vertices,state.bolts);
  JakEffects.submitGeometry(state.vertices,(int)count,state.bolts,(int)(count>>>32),stack,collector,gui);stack.popPose();
 }
 public record Unbaked(boolean light,boolean gui) implements SpecialModelRenderer.Unbaked<Void> {
  public static final MapCodec<Unbaked> CODEC=RecordCodecBuilder.mapCodec(i->i.group(Codec.BOOL.fieldOf("light").forGetter(Unbaked::light),Codec.BOOL.optionalFieldOf("gui",false).forGetter(Unbaked::gui)).apply(i,Unbaked::new));
  public MapCodec<Unbaked> type(){return CODEC;}
  public SpecialModelRenderer<Void> bake(SpecialModelRenderer.BakingContext context){return new EcoItemRenderer(light,gui);}
 }
}
