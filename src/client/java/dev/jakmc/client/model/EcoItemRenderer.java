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
 private static final State[] STATES=new State[2];
 private final boolean light;
 private static final class State {
  final long handle=NativeJak.fxCreate();long last;
  final ByteBuffer view=NativeJak.buffer(44),vertices=NativeJak.buffer(8192*44),bolts=NativeJak.buffer(32768*44);
  State(boolean light){
   NativeJak.fxPush(handle,"[{\"Keep\":[1,\""+(light?"group-eco-light-pill":"group-eco-dark-pill")+"\",[0,0,0],[1,0,0,0,1,0,0,0,1]]}]",0,0,0);
   view.putFloat(0).putFloat(0).putFloat(4).putFloat(0).putFloat(0).putFloat(-1).putFloat(0).putFloat(1).putFloat(0).putFloat(45).putFloat(64);
  }
 }
 EcoItemRenderer(boolean light){this.light=light;}
 public static void initialize(){SpecialModelRenderers.ID_MAPPER.put(JakData.id("eco"),Unbaked.CODEC);}
 public static void clear(){for(int i=0;i<STATES.length;i++)if(STATES[i]!=null){NativeJak.fxDestroy(STATES[i].handle);STATES[i]=null;}}
 @Override public Void extractArgument(ItemStack item){return null;}
 @Override public void getExtents(Consumer<Vector3fc> out){for(float x:new float[]{.25f,.75f})for(float y:new float[]{.1f,.8f})for(float z:new float[]{.25f,.75f})out.accept(new Vector3f(x,y,z));}
 @Override public void submit(Void argument,PoseStack stack,SubmitNodeCollector collector,int lightCoords,int overlay,boolean foil,int outline){
  int index=light?1:0;State state=STATES[index];if(state==null)STATES[index]=state=new State(light);
  long now=System.nanoTime();float dt=state.last==0?1/30f:Math.clamp((now-state.last)/1e9f,0,.1f);state.last=now;
  long count=NativeJak.fxDraw(state.handle,dt,state.view,state.vertices,state.bolts);
  stack.pushPose();stack.translate(.5f,.4f,.5f);float scale=light?.25f:.5f;stack.scale(scale,scale,scale);
  JakEffects.submitGeometry(state.vertices,(int)count,state.bolts,(int)(count>>>32),stack,collector);stack.popPose();
 }
 public record Unbaked(boolean light) implements SpecialModelRenderer.Unbaked<Void> {
  public static final MapCodec<Unbaked> CODEC=RecordCodecBuilder.mapCodec(i->i.group(Codec.BOOL.fieldOf("light").forGetter(Unbaked::light)).apply(i,Unbaked::new));
  public MapCodec<Unbaked> type(){return CODEC;}
  public SpecialModelRenderer<Void> bake(SpecialModelRenderer.BakingContext context){return new EcoItemRenderer(light);}
 }
}
