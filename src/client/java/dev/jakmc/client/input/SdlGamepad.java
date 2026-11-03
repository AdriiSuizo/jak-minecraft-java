package dev.jakmc.client.input;
import java.util.*;
import org.lwjgl.sdl.*;
import static org.lwjgl.sdl.SDLGamepad.*;
/** Uses Minecraft's bundled SDL3 backend; no separate Windows driver or key emulation. */
public final class SdlGamepad implements AutoCloseable {
 public static final int BACK=1,START=2,UP=4,DOWN=8,LEFT=16,RIGHT=32;
 public record State(int id,String name,int held,int system,float leftX,float leftY,float rightX,float rightY,float brake,float throttle){
  public static final State EMPTY=new State(0,"",0,0,0,0,0,0,0,0);
  public float activity(){return Math.max(Math.max((float)Math.hypot(leftX,leftY),(float)Math.hypot(rightX,rightY)),Math.max(brake,throttle));}
 }
 private final Map<Integer,Long> pads=new LinkedHashMap<>();private final Map<Integer,Integer> heldLast=new HashMap<>();
 private boolean initialized;private int active;private long nextScan;
 public State poll(){
  if(!initialized){if(!SDLInit.SDL_InitSubSystem(SDLInit.SDL_INIT_GAMEPAD))throw new IllegalStateException("SDL gamepad: "+SDLError.SDL_GetError());initialized=true;}
  long now=System.nanoTime();if(now>=nextScan){refresh();nextScan=now+1_000_000_000L;}
  SDL_UpdateGamepads();State chosen=State.EMPTY;int candidate=active;
  var it=pads.entrySet().iterator();while(it.hasNext()){
   var e=it.next();if(!SDL_GamepadConnected(e.getValue())){SDL_CloseGamepad(e.getValue());heldLast.remove(e.getKey());it.remove();nextScan=0;continue;}
   int prior=heldLast.getOrDefault(e.getKey(),0);State s=read(e.getKey(),e.getValue(),prior);int combined=s.held|(s.system<<16);heldLast.put(s.id,combined);
   if((combined&~prior)!=0||s.activity()>.5f){candidate=s.id;chosen=s;}
   else if(s.id==candidate||chosen.id==0)chosen=s;
  }
  if(chosen.id!=0)active=chosen.id;return chosen;
 }
 public boolean has(int id){return pads.containsKey(id);}
 public void refresh(){
  var ids=SDL_GetGamepads();if(ids==null)return;
  try{for(int i=0;i<ids.remaining();i++){int id=ids.get(i);if(id==0||pads.containsKey(id))continue;long handle=SDL_OpenGamepad(id);if(handle!=0)pads.put(id,handle);}}
  finally{SDLStdinc.SDL_free(ids);}
 }
 private static State read(int id,long pad,int previous){
  int held=0,system=0;
  int[] source={SDL_GAMEPAD_BUTTON_SOUTH,SDL_GAMEPAD_BUTTON_WEST,SDL_GAMEPAD_BUTTON_EAST,SDL_GAMEPAD_BUTTON_NORTH,SDL_GAMEPAD_BUTTON_LEFT_SHOULDER,SDL_GAMEPAD_BUTTON_RIGHT_SHOULDER,SDL_GAMEPAD_BUTTON_LEFT_STICK,SDL_GAMEPAD_BUTTON_RIGHT_STICK};
  int[] target={1,2,4,8,16,64,256,512};
  for(int i=0;i<source.length;i++)if(SDL_GetGamepadButton(pad,source[i]))held|=target[i];
  int[] controls={SDL_GAMEPAD_BUTTON_BACK,SDL_GAMEPAD_BUTTON_START,SDL_GAMEPAD_BUTTON_DPAD_UP,SDL_GAMEPAD_BUTTON_DPAD_DOWN,SDL_GAMEPAD_BUTTON_DPAD_LEFT,SDL_GAMEPAD_BUTTON_DPAD_RIGHT};
  for(int i=0;i<controls.length;i++)if(SDL_GetGamepadButton(pad,controls[i]))system|=1<<i;
  float brake=ControllerMath.trigger(SDL_GetGamepadAxis(pad,SDL_GAMEPAD_AXIS_LEFT_TRIGGER)),throttle=ControllerMath.trigger(SDL_GetGamepadAxis(pad,SDL_GAMEPAD_AXIS_RIGHT_TRIGGER));
  // Original host Bevy ButtonSettings: press .75 / release .65. Pedals remain fully analog.
  if(brake>=((previous&32)!=0?.65f:.75f))held|=32;if(throttle>=((previous&128)!=0?.65f:.75f))held|=128;
  return new State(id,SDL_GetGamepadName(pad),held,system,ControllerMath.stick(SDL_GetGamepadAxis(pad,SDL_GAMEPAD_AXIS_LEFTX)),-ControllerMath.stick(SDL_GetGamepadAxis(pad,SDL_GAMEPAD_AXIS_LEFTY)),ControllerMath.stick(SDL_GetGamepadAxis(pad,SDL_GAMEPAD_AXIS_RIGHTX)),-ControllerMath.stick(SDL_GetGamepadAxis(pad,SDL_GAMEPAD_AXIS_RIGHTY)),brake,throttle);
 }
 public void close(){for(long p:pads.values())SDL_CloseGamepad(p);pads.clear();heldLast.clear();active=0;nextScan=0;if(initialized){SDLInit.SDL_QuitSubSystem(SDLInit.SDL_INIT_GAMEPAD);initialized=false;}}
}
