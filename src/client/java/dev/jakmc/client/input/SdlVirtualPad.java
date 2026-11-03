package dev.jakmc.client.input;
import org.lwjgl.sdl.*;
import org.lwjgl.system.MemoryStack;
/** Process-local SDL replay device for bounded development checks, never OS key injection. */
public final class SdlVirtualPad implements AutoCloseable {
 public final int id;private final long joystick;
 public SdlVirtualPad(){
  if(!SDLInit.SDL_InitSubSystem(SDLInit.SDL_INIT_GAMEPAD))throw new IllegalStateException(SDLError.SDL_GetError());
  try(var stack=MemoryStack.stackPush()){
   var desc=SDL_VirtualJoystickDesc.calloc(stack);SDLJoystick.SDL_INIT_INTERFACE(desc);
   desc.type((short)SDLJoystick.SDL_JOYSTICK_TYPE_GAMEPAD).naxes((short)6).nbuttons((short)SDLGamepad.SDL_GAMEPAD_BUTTON_COUNT).axis_mask(63).button_mask((1<<SDLGamepad.SDL_GAMEPAD_BUTTON_COUNT)-1).name(stack.UTF8("Jak native controller replay"));
   id=SDLJoystick.SDL_AttachVirtualJoystick(desc);if(id==0)throw new IllegalStateException(SDLError.SDL_GetError());
  }
  joystick=SDLJoystick.SDL_OpenJoystick(id);if(joystick==0)throw new IllegalStateException(SDLError.SDL_GetError());
  axis(4,-1);axis(5,-1);
 }
 public void axis(int index,float raw){if(!SDLJoystick.SDL_SetJoystickVirtualAxis(joystick,index,(short)Math.round(Math.clamp(raw,-1,1)*(raw<0?32768:32767))))throw new IllegalStateException(SDLError.SDL_GetError());}
 public void button(int index,boolean held){if(!SDLJoystick.SDL_SetJoystickVirtualButton(joystick,index,held))throw new IllegalStateException(SDLError.SDL_GetError());}
 public void close(){SDLJoystick.SDL_CloseJoystick(joystick);SDLJoystick.SDL_DetachVirtualJoystick(id);SDLInit.SDL_QuitSubSystem(SDLInit.SDL_INIT_GAMEPAD);}
}
