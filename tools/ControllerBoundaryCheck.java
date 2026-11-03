import dev.jakmc.client.input.*;
import dev.jakmc.nativebridge.NativeJak;
import org.lwjgl.sdl.*;
import com.google.gson.*;
import java.nio.file.*;
import java.util.*;
public final class ControllerBoundaryCheck {
 public static void main(String[] args)throws Exception{
  try(var backend=new SdlGamepad()){
   var physical=backend.poll();System.out.println("Physical controller: "+physical.name()+" id="+physical.id());
   try(var replay=new SdlVirtualPad()){
    SDLEvents.SDL_PumpEvents();backend.refresh();backend.poll();
    System.out.println("Virtual mapping: "+SDLGamepad.SDL_GetGamepadMappingForID(replay.id));
    replay.axis(0,.5f);replay.axis(1,-.75f);replay.axis(2,-.6f);replay.axis(3,.4f);replay.axis(4,-.5f);replay.axis(5,0);replay.button(SDLGamepad.SDL_GAMEPAD_BUTTON_SOUTH,true);
    SDLEvents.SDL_PumpEvents();backend.refresh();var p=backend.poll();
    System.out.println("Mapped SDL: "+p);
    if(p.id()!=replay.id||Math.abs(p.leftX()-.5f)>.001||Math.abs(p.leftY()-.75f)>.001||Math.abs(p.rightY()+.4f)>.001||Math.abs(p.brake()-.25f)>.001||Math.abs(p.throttle()-.5f)>.001||(p.held()&1)==0)throw new AssertionError("SDL mapping or analog axes wrong");
    replay.axis(0,0);replay.axis(1,0);replay.axis(2,0);replay.axis(3,0);replay.axis(4,-1);replay.axis(5,-1);replay.button(SDLGamepad.SDL_GAMEPAD_BUTTON_SOUTH,false);
    var zero=backend.poll();if(zero.held()!=0||zero.brake()!=0||zero.throttle()!=0)throw new AssertionError("Controller releases stuck");
    if(ControllerMath.look(.1f,0)[0]!=0||Math.abs(ControllerMath.look(.59f,0)[0]-.5f)>.0001)throw new AssertionError("Source camera deadzone changed");
    NativeJak.load(Path.of(args[0]));NativeJak.configure(args[1]);long id=NativeJak.create(0);
    var input=NativeJak.buffer(64);input.putFloat(44,1).putInt(48,1|32).putInt(60,-1);
    var floor=NativeJak.buffer(80);NativeBoundaryCheck.triangle(floor,-200,0,-200,-200,0,200,200,0,-200);NativeBoundaryCheck.triangle(floor,200,0,-200,-200,0,200,200,0,200);var empty=NativeJak.buffer(0);
    Set<String> clips=new TreeSet<>(),states=new TreeSet<>();double maxY=0;int old=0;JsonObject last=null;
    try{for(int t=0;t<260;t++){
     replay.axis(1,t>=20&&t<50||t>=150&&t<175?-.9f:0);replay.button(SDLGamepad.SDL_GAMEPAD_BUTTON_SOUTH,t==60||t==64);
     replay.button(SDLGamepad.SDL_GAMEPAD_BUTTON_LEFT_SHOULDER,t>=150&&t<175);
     replay.button(SDLGamepad.SDL_GAMEPAD_BUTTON_EAST,t==200);
     replay.axis(5,t==230?1:-1);
     var s=backend.poll();input.putInt(0,s.held()).putInt(4,s.held()&~old).putFloat(8,s.leftX()).putFloat(12,s.leftY()).putFloat(16,s.rightX()).putFloat(20,s.rightY()).putFloat(24,s.brake()).putFloat(28,s.throttle());old=s.held();
     last=JsonParser.parseString(NativeJak.step(id,input,floor,empty,3)).getAsJsonObject();clips.add(last.get("clip").getAsString());states.add(last.get("state").getAsString());maxY=Math.max(maxY,last.getAsJsonArray("pos").get(1).getAsDouble());
    }}finally{NativeJak.destroy(id);}
    System.out.println("Native movement from SDL: peakY="+maxY+", last="+last.get("pos")+", states="+states+", clips="+clips);
    if(!states.contains("double-jump")||!states.contains("roll")||!states.contains("attack")||states.stream().noneMatch(x->x.contains("board")))throw new AssertionError("Jak movement was not reached from SDL");
   }
   backend.refresh();if(backend.poll().name().equals("Jak native controller replay"))throw new AssertionError("Disconnected device retained");
  }
 }
}
