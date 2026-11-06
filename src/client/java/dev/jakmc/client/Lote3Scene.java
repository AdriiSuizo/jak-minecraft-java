package dev.jakmc.client;

import com.google.gson.JsonArray;
import com.google.gson.JsonObject;
import dev.jakmc.JakData;
import dev.jakmc.client.input.SdlVirtualPad;
import dev.jakmc.content.JakItems;
import dev.jakmc.entity.JakVehicle;
import dev.jakmc.player.JakPlayers;
import java.io.IOException;
import java.io.UncheckedIOException;
import java.nio.file.Files;
import java.util.ArrayList;
import java.util.LinkedHashMap;
import java.util.LinkedHashSet;
import java.util.UUID;
import net.fabricmc.fabric.api.client.event.lifecycle.v1.ClientTickEvents;
import net.minecraft.client.CameraType;
import net.minecraft.client.Minecraft;
import net.minecraft.client.Screenshot;
import net.minecraft.core.BlockPos;
import net.minecraft.core.Direction;
import net.minecraft.core.registries.BuiltInRegistries;
import net.minecraft.resources.Identifier;
import net.minecraft.server.level.ServerPlayer;
import net.minecraft.world.InteractionHand;
import net.minecraft.world.entity.EntitySpawnReason;
import net.minecraft.world.item.ItemStack;
import net.minecraft.world.item.context.UseOnContext;
import net.minecraft.world.level.GameType;
import net.minecraft.world.level.block.Blocks;
import net.minecraft.world.level.block.state.BlockState;
import net.minecraft.world.level.levelgen.Heightmap;
import net.minecraft.world.phys.BlockHitResult;
import net.minecraft.world.phys.Vec3;
import org.lwjgl.sdl.SDLGamepad;

/**
 * Adrii's 06/11 night batch in one bounded run: the game body under the cutscene head, the board on his back,
 * the camera in a roofed room and a roofed corridor, a three-high wall (no hanging under its top), the 5x5
 * launcher under his feet, a splash in water one block deep, a dive (its speed) and climbing out (no fall
 * damage), a zoomer got on, up a hill, paused and resumed, got off on a high place, and the Supernova.
 */
final class Lote3Scene {
    private static boolean running,pause;
    private static int tick,carId=-1;
    private static UUID owner;
    private static BlockPos base;
    private static SdlVirtualPad pad;
    private static InputScene.CameraView view;
    private static Runnable restore;
    private static final JsonObject result=new JsonObject();
    private static final LinkedHashSet<String> wallStates=new LinkedHashSet<>(),carStates=new LinkedHashSet<>();
    private static final JsonArray diveTrack=new JsonArray(),offTrack=new JsonArray(),hillTrack=new JsonArray();
    private static double wallPeak,launcherLow,diveBottomAt;
    private static float healthBefore,healthLow;
    static InputScene.CameraView camera(){return view;}
    static boolean backgroundPad(int id){return pad!=null&&pad.id==id;}
    static void initialize(){ClientTickEvents.END_CLIENT_TICK.register(Lote3Scene::tick);}

    private static Vec3 at(double x,double y,double z){return Vec3.atBottomCenterOf(base).add(x,y,z);}
    private static void look(Vec3 eye,Vec3 target){
        var d=target.subtract(eye);
        view=new InputScene.CameraView(eye,(float)Math.toDegrees(Math.atan2(-d.x,d.z)),(float)-Math.toDegrees(Math.atan2(d.y,d.horizontalDistance())));
    }
    private static void capture(Minecraft c,String file){Screenshot.grab(c.gameDirectory,file,c.gameRenderer.mainRenderTarget(),1,_->{});}
    private static void put(ServerPlayer p,double x,double y,double z,float yaw){var s=at(x,y,z);p.teleportTo(s.x,s.y,s.z);p.setYRot(yaw);p.setXRot(0);p.setDeltaMovement(Vec3.ZERO);}

    static void start(Minecraft client){
        if(client.player==null||client.getSingleplayerServer()==null)throw new IllegalStateException("Open the development world");
        owner=client.player.getUUID();tick=0;carId=-1;result.entrySet().clear();
        wallStates.clear();carStates.clear();
        for(var a:new JsonArray[]{diveTrack,offTrack,hillTrack})while(!a.isEmpty())a.remove(0);
        wallPeak=0;launcherLow=99;diveBottomAt=-1;healthBefore=healthLow=0;
        pause=client.options.pauseOnLostFocus;client.options.pauseOnLostFocus=false;
        client.options.setCameraType(CameraType.THIRD_PERSON_BACK);
        pad=new SdlVirtualPad();
        onServer(client,p->{
            var server=p.level().getServer();
            server.getCommands().performPrefixedCommand(server.createCommandSourceStack(),"time set noon");
            server.getCommands().performPrefixedCommand(server.createCommandSourceStack(),"weather clear");
            var level=p.level();
            var old=p.position();var mode=p.gameMode.getGameModeForPlayer();
            boolean enabled=p.getAttachedOrCreate(JakData.ENABLED);
            var board=p.getAttachedOrCreate(JakData.BOARD).copy();
            var saved=new ArrayList<ItemStack>();
            for(int i=0;i<p.getInventory().getContainerSize();i++)saved.add(p.getInventory().getItem(i).copy());
            var near=p.blockPosition().offset(24,0,0);
            int y=near.getY();
            for(int x=-20;x<=40;x++)for(int z=-16;z<=16;z++)y=Math.max(y,level.getHeight(Heightmap.Types.MOTION_BLOCKING_NO_LEAVES,near.getX()+x,near.getZ()+z));
            base=near.atY(y+41);
            var snapshot=new LinkedHashMap<BlockPos,BlockState>();
            var stone=Blocks.SMOOTH_STONE.defaultBlockState();
            for(int x=-20;x<=40;x++)for(int z=-16;z<=16;z++)for(int h=-5;h<=10;h++){
                var cell=base.offset(x,h,z);
                snapshot.put(cell.immutable(),level.getBlockState(cell));
                // A pool four deep (the dive), water one deep (the splash), stone round them.
                boolean pool=x>=8&&x<=16&&z>=3&&z<=11&&h>=-4,shallow=x>=0&&x<=4&&z>=10&&z<=13&&h==-1;
                level.setBlock(cell,h==-5||h<0&&!pool&&!shallow?stone:(pool||shallow)&&h<0?Blocks.WATER.defaultBlockState():Blocks.AIR.defaultBlockState(),2);
            }
            // A roofed room 5x5 inside with its door to the north, and a roofed corridor one wide.
            for(int x=-15;x<=-9;x++)for(int z=-5;z<=1;z++)for(int h=0;h<=3;h++){
                boolean wall=x==-15||x==-9||z==-5||z==1,roof=h==3,door=z==-5&&x==-12&&h<2;
                if((wall||roof)&&!door)level.setBlock(base.offset(x,h,z),Blocks.STONE_BRICKS.defaultBlockState(),2);
            }
            for(int z=-12;z<=-2;z++)for(int h=0;h<=2;h++){
                level.setBlock(base.offset(-6,h,z),Blocks.STONE_BRICKS.defaultBlockState(),2);
                level.setBlock(base.offset(-4,h,z),Blocks.STONE_BRICKS.defaultBlockState(),2);
                if(h==2)level.setBlock(base.offset(-5,h,z),Blocks.STONE_BRICKS.defaultBlockState(),2);
            }
            // A wall three high: its second block has a block on it (no ledge there), its top is the ledge.
            for(int x=-18;x<=-14;x++)for(int h=0;h<3;h++)level.setBlock(base.offset(x,h,8),stone,2);
            // The zoomer's hill: up one block every two to five high, then a flat top.
            for(int z=-6;z<=14;z++)for(int x=26;x<=34;x++){int top=z<4?Math.max(0,(z+6)/2):5;for(int h=0;h<top;h++)level.setBlock(base.offset(x,h,z),stone,2);}
            p.setGameMode(GameType.CREATIVE);p.setAttached(JakData.ENABLED,true);
            p.setAttached(JakData.BOARD,new ItemStack(JakItems.BOARD));
            p.getInventory().clearContent();
            var launcher=base.offset(5,0,-6);
            p.setItemInHand(InteractionHand.MAIN_HAND,new ItemStack(BuiltInRegistries.ITEM.getValue(Identifier.parse("jakmc:eco_launcher"))));
            result.addProperty("launcher_placed",p.getMainHandItem().getItem().useOn(new UseOnContext(p,InteractionHand.MAIN_HAND,
                new BlockHitResult(Vec3.atBottomCenterOf(launcher),Direction.UP,launcher.below(),false))).consumesAction());
            p.setItemInHand(InteractionHand.MAIN_HAND,ItemStack.EMPTY);
            put(p,0,0,0,180);
            JakPlayers.controlForProbe(p,true);JakPlayers.hostControlsForProbe(p);
            restore=()->{
                if(carId>=0){var e=level.getEntity(carId);if(e!=null){p.stopRiding();e.discard();}}
                snapshot.forEach((cell,state)->level.setBlock(cell,state,2));
                p.getInventory().clearContent();
                for(int i=0;i<saved.size();i++)p.getInventory().setItem(i,saved.get(i));
                JakPlayers.controlForProbe(p,false);
                p.setAttached(JakData.ENABLED,enabled);p.setAttached(JakData.BOARD,board);
                p.setGameMode(mode);p.teleportTo(old.x,old.y,old.z);
            };
            client.execute(()->{look(at(0,1.8,-1.5),at(0,1.7,0));running=true;});
        });
    }

    private static void tick(Minecraft c){
        if(!running)return;
        int t=++tick;
        if(t<440||t>482)c.gui.setScreen(null);
        var pose=JakPoses.get(c.player.getId());
        String state=pose==null?"":pose.raw().get("state").getAsString();
        String clip=pose==null?"":pose.clip();
        double feet=pose==null?0:pose.position().y-at(0,0,0).y;
        // The face: the cutscene head on the game body, then the board on his back.
        if(t==22)capture(c,"lote3-head.png");
        if(t==23)look(at(0,1.8,1.5),at(0,1.7,0));
        if(t==25)capture(c,"lote3-head-2.png");
        if(t==26)look(at(-1.2,2.4,1.8),at(0,1.2,0));
        if(t==32)capture(c,"lote3-board-back.png");
        // The real camera in the room (looking at its back wall, then along it) and in the corridor.
        if(t==36){view=null;onServer(c,p->put(p,-12,0,-2,0));}
        if(t==40)c.player.setYRot(0);
        if(t==60)capture(c,"lote3-room.png");
        if(t==62)c.player.setYRot(90);
        if(t==76)capture(c,"lote3-room-side.png");
        if(t==78)onServer(c,p->put(p,-5,0,-11,0));
        if(t==82){c.player.setYRot(0);onServer(c,p->JakPlayers.probeInput(p,0,0,.6f));}
        if(t==96)capture(c,"lote3-corridor.png");
        if(t==100)onServer(c,p->JakPlayers.probeInput(p,0,0,0));
        // The three-high wall: running and jumping at it, no hanging under its top.
        if(t==104)onServer(c,p->put(p,-16,0,12,180));
        if(t==108){c.player.setYRot(180);onServer(c,p->JakPlayers.probeInput(p,0,0,1));}
        if(t==113)onServer(c,p->JakPlayers.probeInput(p,1,1,1));
        if(t==118)onServer(c,p->JakPlayers.probeInput(p,1,0,1));
        if(t>=108&&t<160){if(!state.isEmpty())wallStates.add(state+"/"+clip);wallPeak=Math.max(wallPeak,feet);}
        if(t==122)look(at(-11,2.5,12),at(-16,1.5,8));
        if(t==126||t==132||t==140||t==150)capture(c,"lote3-wall-"+t+".png");
        if(t==160){view=null;onServer(c,p->JakPlayers.probeInput(p,0,0,0));}
        // On the launcher: his feet on its top, not in it.
        if(t==162)onServer(c,p->put(p,5,dev.jakmc.content.JakEcoLauncher.TOP+.05,-6,180));
        if(t==164)look(at(10,1.6,-6),at(5,.8,-6));
        if(t>=168&&t<178)launcherLow=Math.min(launcherLow,feet);
        if(t==174)capture(c,"lote3-launcher.png");
        // Into water one block deep: the splash must stay on the water.
        // Dropped into it from 2.5 blocks: the splash and its ripples must sit on the water's surface.
        if(t==178){onServer(c,p->put(p,2,2.5,11.5,0));look(at(7,2.6,8),at(2,-.2,11.5));}
        if(t==186||t==190||t==196)capture(c,"lote3-shallow-"+t+".png");
        if(t>=178&&t<202){ripples(pose,"shallow");track(t,state,clip,feet,pose);}
        // The dive in four blocks of water: how fast he goes down, then up and out with no fall damage.
        if(t==202){view=null;onServer(c,p->{put(p,12,-.6,7,180);healthBefore=healthLow=p.getHealth();});}
        // Swimming on the deep pool's surface first: its ripples round him.
        if(t==204)onServer(c,p->JakPlayers.probeInput(p,0,0,.5f));
        if(t==205)look(at(15,2.5,9.5),at(12,-.2,6));
        if(t>=204&&t<212){ripples(pose,"deep");track(t,state,clip,feet,pose);}
        if(t==210)capture(c,"lote3-swim-ripples.png");
        if(t==212){view=null;onServer(c,p->JakPlayers.probeInput(p,2,2,0));}
        if(t>=212&&t<252){diveTrack.add(Math.round(feet*100)/100.0);if(diveBottomAt<0&&feet<-3.7)diveBottomAt=(t-212)/20.0;}
        if(t==226)look(at(12,-.5,3.6),at(12,-2.5,7));
        if(t==232)capture(c,"lote3-dive.png");
        if(t==252){view=null;onServer(c,p->JakPlayers.probeInput(p,0,0,0));}
        if(t==262){c.player.setYRot(180);onServer(c,p->JakPlayers.probeInput(p,1,1,1));}
        if(t>=262&&t<330)onServer(c,p->{healthLow=Math.min(healthLow,p.getHealth());if(tick%4==0)JakPlayers.probeInput(p,1,1,1);});
        if(t==330)onServer(c,p->JakPlayers.probeInput(p,0,0,0));
        // A zoomer on the hill: got on, up it, paused there, got off on its top.
        if(t==332)onServer(c,p->{
            JakPlayers.controlForProbe(p,false);
            var model=JakVehicle.Model.values()[2];
            var car=model.type.create(p.level(),EntitySpawnReason.COMMAND);
            car.setPos(at(30,model.parkingClearance(),-12));car.setYRot(0);
            if(p.level().addFreshEntity(car))carId=car.getId();
            put(p,31.5,0,-12,0);
        });
        if(t==340){onServer(c,p->{var car=p.level().getEntity(carId);if(car!=null)car.interact(p,InteractionHand.MAIN_HAND,new Vec3(0,-.55,-1));});look(at(35,1.6,-10),at(30,1,-12));}
        // Each tick of getting on: riding (client), Jak's state, the player's height and the zoomer's (blocks over the floor).
        if(t>=338&&t<372){
            var car=c.level.getEntity(carId);
            carStates.add(t+" "+(c.player.getVehicle()!=null)+" "+state+"/"+clip+" player "+Math.round((c.player.getY()-at(0,0,0).y)*100)/100.0
                +" pose "+Math.round(feet*100)/100.0+" car "+(car==null?"-":Math.round((car.getY()-at(0,0,0).y)*100)/100.0));
        }
        if(t==344)capture(c,"lote3-mount-1.png");
        if(t==352)capture(c,"lote3-mount-2.png");
        if(t==370){view=null;c.player.setYRot(0);result.addProperty("pad_ready",JakInput.controllerReady(pad.id));pad.axis(5,.25f);}
        if(t>=370&&t<440&&c.level.getEntity(carId) instanceof JakVehicle car){
            var rider=pose==null?Double.NaN:pose.position().y-car.position().y;
            hillTrack.add(Math.round((car.position().z-at(0,0,0).z)*10)/10.0);hillTrack.add(Math.round(rider*100)/100.0);
            if(car.position().z>at(0,0,2).z){pad.axis(5,-1);pad.axis(4,1);}
        }
        if(t==384)capture(c,"lote3-zoomer-hill.png");
        if(t==392)capture(c,"lote3-zoomer-hill-2.png");
        if(t==430)pad.axis(4,-1);
        if(t==440)c.pauseGame(false);
        if(t==450)result.addProperty("paused_screen",c.gui.screen()!=null);
        if(t==480)c.gui.setScreen(null);
        if(t==500)result.addProperty("still_riding_after_pause",c.player.getVehicle()!=null);
        if(t==504)pad.button(SDLGamepad.SDL_GAMEPAD_BUTTON_NORTH,true);
        if(t==508)pad.button(SDLGamepad.SDL_GAMEPAD_BUTTON_NORTH,false);
        if(t>=504&&t<570&&t%2==0)onServer(c,p->{
            int ground=p.level().getHeight(Heightmap.Types.MOTION_BLOCKING,p.getBlockX(),p.getBlockZ());
            synchronized(offTrack){offTrack.add(Math.round((p.getY()-ground)*100)/100.0);}
        });
        if(t==520)look(at(36,7.5,10),at(30,5.5,7));
        if(t==540)capture(c,"lote3-dismount.png");
        // The Supernova on the hill's top.
        if(t==572){view=null;onServer(c,p->{
            JakPlayers.controlForProbe(p,true);JakPlayers.hostControlsForProbe(p);
            p.setItemInHand(InteractionHand.MAIN_HAND,new ItemStack(JakItems.GUNS.get(11)));
            for(int kind=2;kind<6;kind++)dev.jakmc.player.JakResources.grant(p,kind,Math.max(0,100-dev.jakmc.player.JakResources.totals(p)[kind]));
            // Along the platform's floor: the shot comes down on it in its arc (embedded: the small blast after 1.4 s).
            put(p,-18,0,0,-90);
        });}
        if(t==580)c.player.setYRot(-90);
        if(t==590)onServer(c,p->JakPlayers.probeInput(p,64,64,0));
        if(t==600)onServer(c,p->JakPlayers.probeInput(p,0,0,0));
        if(t==615||t==625||t==635||t==650||t==670||t==700)capture(c,"lote3-nova-"+t+".png");
        if(t>=590&&t<720){var nova=pose==null?null:pose.raw().get("nuke");if(nova!=null&&!nova.isJsonNull()&&!result.has("nuke_seen_tick"))result.addProperty("nuke_seen_tick",t);}
        if(t==740){
            view=null;
            pad.axis(5,-1);pad.axis(4,-1);
            result.add("wall_states",strings(wallStates));result.addProperty("wall_peak_feet",wallPeak);
            result.addProperty("launcher_feet_min",launcherLow);result.addProperty("launcher_top",dev.jakmc.content.JakEcoLauncher.TOP);
            result.add("dive_feet",diveTrack);result.addProperty("dive_seconds_to_bottom",diveBottomAt);
            result.addProperty("health_before_swim",healthBefore);result.addProperty("health_lowest_after",healthLow);
            result.add("car_states",strings(carStates));result.add("hill_z_and_rider_over_car",hillTrack);
            synchronized(offTrack){result.add("dismount_feet_over_ground",offTrack.deepCopy());}
            result.addProperty("visual_review","required");result.addProperty("reason","complete");
            onServer(c,p->{
                try{
                    var dir=c.gameDirectory.toPath().resolve("jakmc/evidence/lote3");
                    Files.createDirectories(dir);Files.writeString(dir.resolve("result.json"),result.toString());
                }catch(IOException failure){throw new UncheckedIOException(failure);}
                restore.run();
                c.execute(()->{running=false;view=null;pad.close();pad=null;c.options.pauseOnLostFocus=pause;c.stop();});
            });
        }
    }
    // The drop and the swim tick by tick: Jak's state, his feet, and each ripple's height over the surface.
    private static void track(int t,String state,String clip,double feet,JakPoses.Pose pose){
        if(!result.has("water_track"))result.add("water_track",new JsonArray());
        var line=new StringBuilder(t+" "+state+"/"+clip+" feet "+Math.round(feet*100)/100.0);
        if(pose!=null&&pose.raw().get("ripples") instanceof JsonArray list)for(var e:list){
            var a=e.getAsJsonArray();line.append(" ripple ").append(Math.round((pose.position().y+a.get(1).getAsDouble()-at(0,-.11,0).y)*100)/100.0).append("@").append(Math.round(a.get(4).getAsDouble()*100)/100.0);
        }
        result.getAsJsonArray("water_track").add(line.toString());
    }
    // Each ripple's height over the water's surface (0.11 under the floor's top), and how many there were.
    private static void ripples(JakPoses.Pose pose,String where){
        if(pose==null||!(pose.raw().get("ripples") instanceof JsonArray list))return;
        double surface=at(0,-.11,0).y;
        for(var e:list){
            var r=e.getAsJsonArray();double over=pose.position().y+r.get(1).getAsDouble()-surface;
            result.addProperty(where+"_ripples",(result.has(where+"_ripples")?result.get(where+"_ripples").getAsInt():0)+1);
            if(!result.has(where+"_ripple_over_max")||over>result.get(where+"_ripple_over_max").getAsDouble())result.addProperty(where+"_ripple_over_max",Math.round(over*1000)/1000.0);
            if(!result.has(where+"_ripple_over_min")||over<result.get(where+"_ripple_over_min").getAsDouble())result.addProperty(where+"_ripple_over_min",Math.round(over*1000)/1000.0);
        }
    }
    private static JsonArray strings(java.util.Collection<String> values){var out=new JsonArray();values.forEach(out::add);return out;}
    private static void onServer(Minecraft client,java.util.function.Consumer<ServerPlayer> action){
        var server=client.getSingleplayerServer();
        server.execute(()->{var player=server.getPlayerList().getPlayer(owner);if(player!=null)action.accept(player);});
    }
}
