package dev.jakmc.client;

import com.google.gson.*;
import dev.jakmc.JakMod;
import java.nio.file.*;
import java.util.*;
import net.fabricmc.fabric.api.client.event.lifecycle.v1.ClientTickEvents;
import net.fabricmc.loader.api.FabricLoader;
import net.minecraft.client.Minecraft;
import net.minecraft.client.Screenshot;
import net.minecraft.client.gui.components.*;
import net.minecraft.client.gui.screens.worldselection.CreateWorldScreen;
import net.minecraft.client.input.InputWithModifiers;

/** Development-only file bridge. All game operations execute on the client/server owner thread. */
public final class DevBridge {
    private static Path dir;
    private static final String SESSION=UUID.randomUUID().toString();
    private static final InputWithModifiers PRESS=new InputWithModifiers(){public int input(){return 40;}public int modifiers(){return 0;}};
    public static void initialize(){
        if(!FabricLoader.getInstance().isDevelopmentEnvironment()||!Boolean.getBoolean("jakmc.devBridge"))return;
        dir=FabricLoader.getInstance().getGameDir().resolve("jakmc/bridge");
        try{
            Files.createDirectories(dir);
            // A crashed/terminated previous client cannot leave a quit command
            // to be executed by the next session, or block its first observation.
            Path old=dir.resolve("request.json");if(Files.exists(old))Files.move(old,dir.resolve("stale-"+UUID.randomUUID()+".json"));
            var ready=new JsonObject();ready.addProperty("session",SESSION);ready.addProperty("pid",ProcessHandle.current().pid());Path tmp=dir.resolve("ready.tmp");Files.writeString(tmp,ready.toString());Files.move(tmp,dir.resolve("ready.json"),StandardCopyOption.REPLACE_EXISTING,StandardCopyOption.ATOMIC_MOVE);
        }catch(Exception e){throw new RuntimeException(e);}
        ClientTickEvents.END_CLIENT_TICK.register(DevBridge::tick);
        JakMod.LOG.info("Jak development bridge: {}",dir);
    }
    private static void tick(Minecraft client){
        Path inbox=dir.resolve("request.json");if(!Files.isRegularFile(inbox))return;
        JsonObject request=null;
        try{
            String text=Files.readString(inbox);Files.delete(inbox);request=JsonParser.parseString(text).getAsJsonObject();
            if(!request.has("session")||!SESSION.equals(request.get("session").getAsString()))throw new IllegalArgumentException("Stale request belongs to a different game process");
            String id=request.get("id").getAsString();if(!id.matches("[a-zA-Z0-9_-]+"))throw new IllegalArgumentException("Invalid request id");
            String op=request.get("op").getAsString();JsonObject out=new JsonObject();
            switch(op){
                case "observe" -> observe(client,out);
                case "scale_scene" -> {ScaleScene.start(client);out.addProperty("scheduled",true);}
                case "create_world" -> {if(client.getSingleplayerServer()!=null)throw new IllegalStateException("An existing world is already open");CreateWorldScreen.openFresh(client,()->{});out.addProperty("scheduled",true);}
                case "click" -> {
                    var screen=client.gui.screen();if(screen==null)throw new IllegalStateException("No screen");
                    int index=request.get("index").getAsInt();var child=screen.children().get(index);
                    if(!(child instanceof AbstractButton button)||!button.active)throw new IllegalArgumentException("Widget is not an active button");
                    button.onPress(PRESS);out.addProperty("clicked",index);
                }
                case "close_screen" -> {client.gui.setScreen(null);out.addProperty("closed",true);}
                case "command" -> {
                    var server=client.getSingleplayerServer();if(server==null)throw new IllegalStateException("No integrated server");
                    String command=request.get("command").getAsString();
                    server.execute(()->{server.getCommands().performPrefixedCommand(server.createCommandSourceStack(),command);});out.addProperty("scheduled",true);
                }
                case "camera" -> {client.options.setCameraType(net.minecraft.client.CameraType.valueOf(request.get("type").getAsString()));out.addProperty("camera",request.get("type").getAsString());}
                case "screenshot" -> {
                    String name=request.get("name").getAsString();if(!name.matches("[a-zA-Z0-9_-]+\\.png"))throw new IllegalArgumentException("Invalid screenshot name");
                    Screenshot.grab(client.gameDirectory,name,client.gameRenderer.mainRenderTarget(),1,message->JakMod.LOG.info("Bridge screenshot: {}",message.getString()));out.addProperty("file",client.gameDirectory.toPath().resolve("screenshots").resolve(name).toString());
                }
                case "gems_visual_scene" -> {GemScene.startVisual(client);out.addProperty("scheduled",true);}
                case "citizens_scene" -> {CitizenScene.start(client);out.addProperty("scheduled",true);}
                case "collision_perf_scene" -> {CollisionPerfScene.start(client);out.addProperty("scheduled",true);}
                case "gems_scene" -> {GemScene.start(client);out.addProperty("scheduled",true);}
                case "input_actions_scene" -> {InputScene.startInteractions(client);out.addProperty("scheduled",true);}
                case "powers_scene" -> {InputScene.startPowers(client);out.addProperty("scheduled",true);}
                case "input_scene" -> {InputScene.start(client);out.addProperty("scheduled",true);}
                case "rings_scene" -> {WeaponScene.startRings(client);out.addProperty("scheduled",true);}
                case "weapons_scene" -> {WeaponScene.start(client);out.addProperty("scheduled",true);}
                case "effects_scene" -> {PortScene.start(client);out.addProperty("scheduled",true);}
                case "enemies_scene" -> {EnemyScene.start(client);out.addProperty("scheduled",true);}
                case "systems_scene" -> {EnemyScene.startSystems(client);out.addProperty("scheduled",true);}
                case "combat_scene" -> {EnemyScene.startCombat(client);out.addProperty("scheduled",true);}
                case "quit" -> {out.addProperty("stopping",true);client.stop();}
                default -> throw new IllegalArgumentException("Unknown development operation: "+op);
            }
            out.addProperty("ok",true);Files.writeString(dir.resolve(id+".json"),out.toString());
        }catch(Exception e){JakMod.LOG.error("Development bridge request failed",e);try{JsonObject out=new JsonObject();out.addProperty("ok",false);out.addProperty("error",e.toString());Files.writeString(dir.resolve(request!=null?request.get("id").getAsString()+".json":"error.json"),out.toString());}catch(Exception ignored){}}
    }
    private static void observe(Minecraft c,JsonObject out){
        var screen=c.gui.screen();out.addProperty("screen",screen==null?null:screen.getClass().getName());out.addProperty("fps",c.getFps());
        var widgets=new JsonArray();if(screen!=null){int i=0;for(var child:screen.children()){
            var item=new JsonObject();item.addProperty("index",i++);item.addProperty("class",child.getClass().getSimpleName());
            if(child instanceof AbstractWidget w){item.addProperty("label",w.getMessage().getString());item.addProperty("active",w.active);}widgets.add(item);
        }}out.add("widgets",widgets);
        if(c.player!=null){out.addProperty("player",c.player.getName().getString());out.addProperty("x",c.player.getX());out.addProperty("y",c.player.getY());out.addProperty("z",c.player.getZ());var pose=JakPoses.get(c.player.getId());if(pose!=null)out.add("jak",pose.raw());}
    }
}
