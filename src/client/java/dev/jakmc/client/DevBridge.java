package dev.jakmc.client;

import com.google.gson.*;
import dev.jakmc.JakMod;
import java.nio.file.*;
import java.util.*;
import net.fabricmc.fabric.api.client.event.lifecycle.v1.ClientTickEvents;
import net.fabricmc.loader.api.FabricLoader;
import net.minecraft.client.Minecraft;
import net.minecraft.client.CameraType;
import net.minecraft.client.Screenshot;
import net.minecraft.client.gui.components.*;
import net.minecraft.client.gui.screens.worldselection.CreateWorldScreen;
import net.minecraft.client.input.InputWithModifiers;

/** Development-only file bridge. All game operations execute on the client/server owner thread. */
public final class DevBridge {
    private static Path dir;
    private static boolean productionProbe;
    private static final String SESSION=UUID.randomUUID().toString();
    private static final InputWithModifiers PRESS=new InputWithModifiers(){public int input(){return 40;}public int modifiers(){return 0;}};
    public static void initialize(){
        if(!Boolean.getBoolean("jakmc.devBridge"))return;
        if(!FabricLoader.getInstance().isDevelopmentEnvironment()){
            String token=System.getProperty("jakmc.productionProbe","");
            if(!token.matches("[a-f0-9]{32}"))return;
            Path permit=FabricLoader.getInstance().getGameDir().resolve("jakmc/allow-production-probe");
            try{if(!Files.readString(permit).equals(token))return;}catch(Exception absent){return;}
            productionProbe=true;
        }
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
            if(productionProbe&&!Set.of("observe","package_status","package_camera","package_capture","quit").contains(op))throw new IllegalArgumentException("Operation unavailable in packaged-artifact check");
            switch(op){
                case "observe" -> observe(client,out);
                case "scale_scene" -> {ScaleScene.start(client);out.addProperty("scheduled",true);}
                case "package_status" -> {
                    if(!productionProbe)throw new IllegalArgumentException("Packaged-artifact check only");
                    out.addProperty("camera_before_probe",client.options.getCameraType().name());
                    out.addProperty("enabled",client.player!=null&&client.player.getAttachedOrCreate(dev.jakmc.JakData.ENABLED));
                    var renderer=dev.jakmc.client.model.JakRenderer.class;
                    out.addProperty("actor_drawn",client.player!=null&&dev.jakmc.client.model.JakRenderer.lastEntityId==client.player.getId());
                    out.addProperty("cinematic_draws",dev.jakmc.client.model.JakRenderer.cinematicRenderSerial);
                    out.addProperty("standing_height",dev.jakmc.client.model.JakRenderer.standingHeight);
                    out.addProperty("source_pose",client.player!=null&&JakPoses.get(client.player.getId())!=null);
                    for(String resource:List.of("/assets/jakmc/sounds.json","/assets/jakmc/flut/eye_open.png","/assets/jakmc/hellcat/deck.json","/data/jakmc/catalog/haven.jbp","/data/jakmc/catalog/spargus.jbp")){
                        try(var in=renderer.getResourceAsStream(resource)){out.addProperty(resource,in!=null&&in.read()!=-1);}
                    }
                }
                case "package_camera" -> {
                    if(!productionProbe)throw new IllegalArgumentException("Packaged-artifact check only");
                    client.options.setCameraType(CameraType.THIRD_PERSON_FRONT);
                }
                case "package_capture" -> {
                    if(!productionProbe)throw new IllegalArgumentException("Packaged-artifact check only");
                    Screenshot.grab(client.gameDirectory,"installed-jak.png",client.gameRenderer.mainRenderTarget(),1,message->JakMod.LOG.info("Installed package screenshot: {}",message.getString()));
                }
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
                case "yakow_scene" -> {YakowScene.start(client);out.addProperty("scheduled",true);}
                case "player_sleep_scene" -> {CitizenScene.startPlayerSleep(client);out.addProperty("scheduled",true);}
                case "citizens_scene" -> {CitizenScene.start(client);out.addProperty("scheduled",true);}
                case "collision_perf_scene" -> {CollisionPerfScene.start(client);out.addProperty("scheduled",true);}
                case "gems_scene" -> {GemScene.start(client);out.addProperty("scheduled",true);}
                case "menus_scene" -> {MenuScene.start(client);out.addProperty("scheduled",true);}
                case "integration_scene" -> {IntegrationScene.start(client);out.addProperty("scheduled",true);}
                case "input_actions_scene" -> {InputScene.startInteractions(client);out.addProperty("scheduled",true);}
                case "flut_scene" -> {InputScene.startFlut(client);out.addProperty("scheduled",true);}
                case "equipment_scene" -> {EquipmentScene.start(client);out.addProperty("scheduled",true);}
                case "economy_scene" -> {EconomyScene.start(client);out.addProperty("scheduled",true);}
                case "rules_scene" -> {RulesScene.start(client);out.addProperty("scheduled",true);}
                case "lands_scene" -> {LandsScene.start(client);out.addProperty("scheduled",true);}
                case "punch_scene" -> {PunchScene.start(client);out.addProperty("scheduled",true);}
                case "bench_scene" -> {BenchScene.start(client);out.addProperty("scheduled",true);}
                case "wrecks_scene" -> {WreckScene.start(client);out.addProperty("scheduled",true);}
                case "natural_cities_scene" -> {SitesScene.startCities(client);out.addProperty("scheduled",true);}
                case "sites_scene" -> {SitesScene.start(client);out.addProperty("scheduled",true);}
                case "new_sites_scene" -> {SitesScene.start(client,true);out.addProperty("scheduled",true);}
                case "cities_scene" -> {
                    var at=request.has("origin")?dev.jakmc.player.JakPlayers.vector(request.getAsJsonArray("origin")):null;
                    CityScene.start(client,at);out.addProperty("scheduled",true);
                }
                case "zoomers_scene" -> {CarScene.start(client,true);out.addProperty("scheduled",true);}
                case "cars_scene" -> {CarScene.start(client,false);out.addProperty("scheduled",true);}
                case "surface_scene" -> {SurfaceScene.start(client);out.addProperty("scheduled",true);}
                case "ladder_scene" -> {LadderScene.start(client);out.addProperty("scheduled",true);}
                case "track_scene" -> {TrackScene.start(client);out.addProperty("scheduled",true);}
                case "altar_scene" -> {AltarScene.start(client);out.addProperty("scheduled",true);}
                case "weather_scene" -> {WeatherScene.start(client);out.addProperty("scheduled",true);}
                case "vents_scene" -> {VentScene.start(client);out.addProperty("scheduled",true);}
                case "materials_scene" -> {MaterialScene.start(client);out.addProperty("scheduled",true);}
                case "hellcat_scene","helldog_scene" -> {InputScene.startHellcat(client,op.equals("helldog_scene"));out.addProperty("scheduled",true);}
                case "hellcat_crossing_scene" -> {
                    var at=request.has("origin")?dev.jakmc.player.JakPlayers.vector(request.getAsJsonArray("origin")):client.player.position();
                    InputScene.startHellcatCrossing(client,at);out.addProperty("scheduled",true);
                }
                case "leaper_scene" -> {InputScene.startLeaper(client);out.addProperty("scheduled",true);}
                case "dark_motion_scene" -> {InputScene.startDarkMotion(client);out.addProperty("scheduled",true);}
                case "powers_scene" -> {InputScene.startPowers(client);out.addProperty("scheduled",true);}
                case "input_scene" -> {InputScene.start(client);out.addProperty("scheduled",true);}
                case "rings_scene" -> {WeaponScene.startRings(client);out.addProperty("scheduled",true);}
                case "first_person_scene" -> {WeaponScene.startFirstPerson(client);if(request.has("mode"))WeaponScene.onlyMode(request.get("mode").getAsInt());out.addProperty("scheduled",true);}
                case "weapons_scene" -> {WeaponScene.start(client);out.addProperty("scheduled",true);}
                case "effects_scene" -> {PortScene.start(client);out.addProperty("scheduled",true);}
                case "enemies_scene" -> {EnemyScene.start(client);out.addProperty("scheduled",true);}
                case "systems_scene" -> {EnemyScene.startSystems(client);out.addProperty("scheduled",true);}
                case "spydroid_scene" -> {SpydroidScene.start(client);out.addProperty("scheduled",true);}
                case "kermit_scene" -> {KermitScene.start(client);out.addProperty("scheduled",true);}
                case "lote_scene" -> {LoteScene.start(client);out.addProperty("scheduled",true);}
                case "lote2_scene" -> {Lote2Scene.start(client);out.addProperty("scheduled",true);}
                case "lote3_scene" -> {Lote3Scene.start(client);out.addProperty("scheduled",true);}
                case "lote4_scene" -> {Lote4Scene.start(client);out.addProperty("scheduled",true);}
                case "lote5_scene" -> {Lote5Scene.start(client);out.addProperty("scheduled",true);}
                case "lote6_scene" -> {Lote6Scene.start(client);out.addProperty("scheduled",true);}
                case "showcase_scene" -> {ShowcaseScene.start(client);out.addProperty("scheduled",true);}
                case "lote7_scene" -> {Lote7Scene.start(client);out.addProperty("scheduled",true);}
                case "lote8_scene" -> {Lote8Scene.start(client);out.addProperty("scheduled",true);}
                case "lote9_scene" -> {Lote9Scene.start(client);out.addProperty("scheduled",true);}
                case "replay_probe_scene" -> {ReplayProbeScene.start(client);out.addProperty("scheduled",true);}
                case "trees_scene" -> {TreeScene.start(client,dev.jakmc.player.JakPlayers.vector(request.getAsJsonArray("origin")));out.addProperty("scheduled",true);}
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
