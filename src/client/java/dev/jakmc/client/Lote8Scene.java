package dev.jakmc.client;

import com.google.gson.JsonArray;
import com.google.gson.JsonObject;
import dev.jakmc.JakData;
import dev.jakmc.content.JakEcoLauncher;
import dev.jakmc.content.JakIrisDoor;
import dev.jakmc.content.darkeco.DarkEco;
import dev.jakmc.entity.JakEnemy;
import dev.jakmc.player.JakPlayers;
import java.io.IOException;
import java.io.UncheckedIOException;
import java.nio.file.Files;
import java.util.ArrayList;
import java.util.HashMap;
import java.util.LinkedHashSet;
import java.util.List;
import java.util.Map;
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
import net.minecraft.world.entity.item.ItemEntity;
import net.minecraft.world.item.ItemStack;
import net.minecraft.world.item.Items;
import net.minecraft.world.item.context.UseOnContext;
import net.minecraft.world.level.GameType;
import net.minecraft.world.level.block.Blocks;
import net.minecraft.world.level.block.state.properties.BlockStateProperties;
import net.minecraft.world.level.levelgen.Heightmap;
import net.minecraft.world.phys.BlockHitResult;
import net.minecraft.world.phys.Vec3;

/**
 * Adrii's 07/11 second batch in one run: the dive (no rising on his own, out of the bottom with X), the 2x2 airlock and
 * Jak through it, a ledge and a block's corner with no harm, the eco timer, the launcher's jump, an item going in dark
 * eco, and creatures chasing over steps with how often their clips change.
 */
final class Lote8Scene {
    private static boolean running,pause;
    private static int tick;
    private static UUID owner;
    private static BlockPos base;
    private static volatile InputScene.CameraView view;
    private static final JsonObject result=new JsonObject();
    private static final JsonArray dive=new JsonArray(),launch=new JsonArray(),jump=new JsonArray(),flight=new JsonArray();
    private static final LinkedHashSet<String> launchClips=new LinkedHashSet<>(),ledgeStates=new LinkedHashSet<>();
    private static final Map<Integer,String> clips=new HashMap<>();
    private static final Map<Integer,Integer> flips=new HashMap<>();
    private static final List<JakEnemy> enemies=new ArrayList<>();
    private static float health;
    private static double peak;
    private static ItemEntity item;
    static InputScene.CameraView camera(){return view;}
    static void initialize(){ClientTickEvents.END_CLIENT_TICK.register(Lote8Scene::tick);}
    private static Vec3 at(double x,double y,double z){return Vec3.atBottomCenterOf(base).add(x,y,z);}
    private static void look(Vec3 eye,Vec3 target){
        var d=target.subtract(eye);
        view=new InputScene.CameraView(eye,(float)Math.toDegrees(Math.atan2(-d.x,d.z)),(float)-Math.toDegrees(Math.atan2(d.y,d.horizontalDistance())));
    }
    private static void capture(Minecraft c,String file){Screenshot.grab(c.gameDirectory,file,c.gameRenderer.mainRenderTarget(),1,_->{});}
    private static void put(ServerPlayer p,double x,double y,double z,float yaw){var s=at(x,y,z);p.teleportTo(s.x,s.y,s.z);p.setYRot(yaw);p.setXRot(0);p.setDeltaMovement(Vec3.ZERO);}
    private static void place(ServerPlayer p,String id,int x,int z){
        var cell=base.offset(x,0,z);
        p.setItemInHand(InteractionHand.MAIN_HAND,new ItemStack(BuiltInRegistries.ITEM.getValue(Identifier.parse(id))));
        result.addProperty(id+"_placed",p.getMainHandItem().getItem().useOn(new UseOnContext(p,InteractionHand.MAIN_HAND,
            new BlockHitResult(Vec3.atBottomCenterOf(cell),Direction.UP,cell.below(),false))).consumesAction());
        p.setItemInHand(InteractionHand.MAIN_HAND,ItemStack.EMPTY);
    }

    static void start(Minecraft client){
        if(client.player==null||client.getSingleplayerServer()==null)throw new IllegalStateException("Open the development world");
        owner=client.player.getUUID();tick=0;result.entrySet().clear();item=null;peak=0;
        for(var a:new JsonArray[]{dive,launch,jump,flight})while(!a.isEmpty())a.remove(0);
        launchClips.clear();ledgeStates.clear();clips.clear();flips.clear();enemies.clear();
        pause=client.options.pauseOnLostFocus;client.options.pauseOnLostFocus=false;
        client.options.setCameraType(CameraType.THIRD_PERSON_BACK);
        onServer(client,p->{
            var server=p.level().getServer();
            for(var text:new String[]{"time set noon","weather clear","gamerule spawn_monsters false"})
                server.getCommands().performPrefixedCommand(server.createCommandSourceStack().withSuppressedOutput(),text);
            var level=p.level();
            var near=p.blockPosition();
            base=near.atY(level.getHeight(Heightmap.Types.MOTION_BLOCKING_NO_LEAVES,near.getX(),near.getZ())+24);
            var stone=Blocks.STONE.defaultBlockState();var air=Blocks.AIR.defaultBlockState();
            for(int x=-30;x<=54;x++)for(int z=-12;z<=12;z++)for(int y=-6;y<=30;y++)level.setBlock(base.offset(x,y,z),y<0?stone:air,2);
            // The pool, as the showcase's: 6 by 5 and four deep.
            for(int x=10;x<=15;x++)for(int z=4;z<=8;z++)for(int y=-4;y<=-1;y++)level.setBlock(base.offset(x,y,z),Blocks.WATER.defaultBlockState(),3);
            // A wall with the airlock in it, not a hole by its sides.
            for(int x=-8;x<=8;x++)for(int y=0;y<4;y++)level.setBlock(base.offset(x,y,-6),stone,2);
            var anchor=base.offset(0,0,-6);
            for(int part=0;part<4;part++)level.setBlock(JakIrisDoor.cell(anchor,Direction.SOUTH,part,2),air,2);
            for(int part=0;part<4;part++)level.setBlock(JakIrisDoor.cell(anchor,Direction.SOUTH,part,2),
                JakIrisDoor.AIRLOCK.defaultBlockState().setValue(BlockStateProperties.HORIZONTAL_FACING,Direction.SOUTH).setValue(JakIrisDoor.PART,part),3);
            // A pillar for the ledge and the corner.
            for(int x=-16;x<=-12;x++)for(int z=-4;z<=0;z++)for(int y=0;y<4;y++)level.setBlock(base.offset(x,y,z),stone,2);
            // Dark eco.
            for(int x=20;x<=26;x++)for(int z=-6;z<=0;z++)level.setBlock(base.offset(x,-1,z),DarkEco.BLOCK.defaultBlockState(),3);
            // Steps for the creatures.
            for(int x=30;x<=46;x++)for(int z=-8;z<=8;z++)for(int y=0;y<((x-30)/2)%3;y++)level.setBlock(base.offset(x,y,z),stone,2);
            JakPlayers.chooseMode(p,true);
            place(p,"jakmc:vent_blue",-22,0);
            place(p,"jakmc:eco_launcher",-22,6);
            p.setGameMode(GameType.SURVIVAL);
            p.getInventory().clearContent();
            put(p,12.5,.2,6,0);
            JakPlayers.controlForProbe(p,true);JakPlayers.hostControlsForProbe(p);
        });
        running=true;
    }

    private static void sample(Minecraft c,JsonArray into,int t,String mark){
        var pose=JakPoses.rendered(c.player.getId());
        var row=new JsonObject();row.addProperty("t",t);
        row.addProperty("y",Math.round((c.player.getY()-base.getY())*100)/100.0);
        if(pose!=null){row.addProperty("state",pose.raw().get("state").getAsString());row.addProperty("clip",pose.clip());row.add("v",pose.raw().get("velocity"));row.add("status",pose.raw().get("status"));if(pose.raw().has("trail"))row.add("trail",pose.raw().get("trail"));}
        row.addProperty("x",Math.round((c.player.getX()-base.getX())*100)/100.0);row.addProperty("z",Math.round((c.player.getZ()-base.getZ())*100)/100.0);
        row.addProperty("air",c.player.getAirSupply());row.addProperty("hp",c.player.getHealth());
        if(mark!=null)row.addProperty("at",mark);
        into.add(row);
    }

    private static void tick(Minecraft c){
        if(!running||c.player==null||c.level==null)return;
        int t=tick++;
        var pose=JakPoses.rendered(c.player.getId());
        // The dive. A tap of Square: he goes under and stays. Held: to the bottom, pushing at the wall. Then X: out.
        if(t==200)onServer(c,p->{put(p,12.5,-.6,6,0);p.setHealth(p.getMaxHealth());});
        if(t==215)onServer(c,p->JakPlayers.probeInput(p,2,2,0));
        if(t==222)onServer(c,p->JakPlayers.probeInput(p,0,0,0));
        if(t==300)onServer(c,p->JakPlayers.probeInput(p,2,2,0));
        if(t==360)onServer(c,p->JakPlayers.probeInput(p,2,0,.8f));
        if(t==400)onServer(c,p->JakPlayers.probeInput(p,0,0,.8f));
        if(t==420)onServer(c,p->JakPlayers.probeInput(p,0,0,0));
        if(t==390)capture(c,"lote8-dive-bottom.png");
        if(t==430)onServer(c,p->JakPlayers.probeInput(p,1,1,0));
        if(t==432)onServer(c,p->JakPlayers.probeInput(p,1,0,0));
        if(t==470)look(at(7,3,6),at(12.5,3,6));
        if(t==500)capture(c,"lote8-bubbles.png");
        if(t==520)onServer(c,p->JakPlayers.probeInput(p,0,0,0));
        if(t>=205&&t<=520&&t%5==0)sample(c,dive,t,null);
        // The airlock: shut in its wall, then open and Jak through it.
        if(t==530){onServer(c,p->put(p,-.5,0,0,180));look(at(-2.5,2,-1),at(-.5,1,-6));}
        if(t==545)capture(c,"lote8-door-shut.png");
        if(t==550)onServer(c,p->{
            var anchor=base.offset(0,0,-6);
            for(int part=0;part<4;part++){var cell=JakIrisDoor.cell(anchor,Direction.SOUTH,part,2);p.level().setBlock(cell,p.level().getBlockState(cell).setValue(JakIrisDoor.OPEN,true),3);}
        });
        if(t==610)capture(c,"lote8-door-open.png");
        if(t==640)capture(c,"lote8-door-through.png");
        if(t==612){view=null;onServer(c,p->{put(p,1,0,-9,0);JakPlayers.probeInput(p,0,0,1);});}
        if(t==650)onServer(c,p->{JakPlayers.probeInput(p,0,0,0);result.addProperty("door_crossed",p.getZ()>at(0,0,-5.4).z);result.addProperty("door_z",p.getZ()-base.getZ());result.addProperty("door_x",p.getX()-base.getX());
            var cell=base.offset(0,0,-6);var state=p.level().getBlockState(cell);result.addProperty("door_state",state.toString());result.addProperty("door_shape_empty",state.getCollisionShape(p.level(),cell).isEmpty());});
        // A ledge: falling past the pillar's top against it. Then a block's corner, pushed into.
        if(t==660){onServer(c,p->{p.setHealth(p.getMaxHealth());health=p.getHealth();put(p,-13.5,4.6,1.1,180);JakPlayers.probeInput(p,0,0,1);});look(at(-9,5,4),at(-13.5,4,.5));c.player.setYRot(180);}
        if(t>=660&&t<740&&pose!=null)ledgeStates.add(pose.raw().get("state").getAsString());
        if(t==700)capture(c,"lote8-ledge.png");
        if(t==740)onServer(c,p->{JakPlayers.probeInput(p,0,0,0);result.addProperty("ledge_hp_lost",health-p.getHealth());});
        if(t==745){onServer(c,p->{p.setHealth(p.getMaxHealth());health=p.getHealth();put(p,-11.2,0,.8,135);JakPlayers.probeInput(p,0,0,1);});c.player.setYRot(135);}
        if(t==770)capture(c,"lote8-corner.png");
        if(t==800)onServer(c,p->{JakPlayers.probeInput(p,0,0,0);result.addProperty("corner_hp_lost",health-p.getHealth());});
        // Blue eco, the timer, the launcher.
        if(t==805){view=null;onServer(c,p->{var s=at(-22,.24,0);p.teleportTo(s.x,s.y,s.z);});}
        if(t==830)onServer(c,p->{result.addProperty("blue_eco",JakPlayers.blueEco(p));p.setHealth(p.getMaxHealth());health=p.getHealth();put(p,-22,JakEcoLauncher.TOP+.02,6,180);});
        if(t==836)capture(c,"lote8-timer-full.png");
        c.getFramerateLimitTracker().onInputReceived();
        if(t==838){view=null;onServer(c,p->JakPlayers.probeInput(p,1,1,0));
            long start=System.nanoTime();
            dev.jakmc.client.model.JakRenderer.frameTrace=(pose2,frame)->{
                var row=new JsonArray();row.add((System.nanoTime()-start)/100_000/10.0);row.add(pose2.clip());row.add(Math.round(frame*100)/100f);
                var shown=JakPoses.presented();row.add(shown==null?-1:Math.round((shown.y-base.getY())*1000)/1000.0);
                row.add(dev.jakmc.client.model.JakRenderer.traceStateY-base.getY());row.add(dev.jakmc.client.model.JakRenderer.traceCameraY-base.getY());
                row.add(dev.jakmc.client.model.JakRenderer.traceChestY);row.add(c.gameRenderer.mainCamera().xRot());
                synchronized(flight){flight.add(row);}
            };
        }
        if(t==1000)dev.jakmc.client.model.JakRenderer.frameTrace=null;
        if(t==842)onServer(c,p->JakPlayers.probeInput(p,0,0,0));
        if(t>=838&&t<1040&&pose!=null){launchClips.add(pose.clip());peak=Math.max(peak,c.player.getY()-base.getY());}
        if(t>=838&&t<1040&&t%10==0)sample(c,launch,t,null);
        if(t==846||t==850||t==854||t==860||t==870||t==880||t==890)capture(c,"lote8-launch-"+t+".png");
        if(t==1040){view=null;capture(c,"lote8-timer-later.png");
            if(pose!=null&&pose.raw().get("ecoTimer") instanceof JsonArray timer)result.addProperty("timer_left",timer.get(1).getAsFloat());
            onServer(c,p->result.addProperty("launch_hp_lost",health-p.getHealth()));}
        // An item in dark eco.
        if(t==1045)look(at(19,3,3),at(23,0,-3));
        if(t==1055)onServer(c,p->{
            var level=p.level();var drop=at(23,1.2,-3);
            item=new ItemEntity(level,drop.x,drop.y,drop.z,new ItemStack(Items.DIAMOND,3));
            item.setDeltaMovement(Vec3.ZERO);level.addFreshEntity(item);
        });
        if(t>=1059&&t<=1085&&t%3==0)capture(c,"lote8-item-"+t+".png");
        if(t==1090)onServer(c,p->result.addProperty("item_gone",item.isRemoved()));
        // Creatures chasing over steps: how often the clip of each changes.
        if(t==1095)onServer(c,p->{
            var level=p.level();
            put(p,50,0,0,90);
            var type=BuiltInRegistries.ENTITY_TYPE.getValue(JakData.id("babak"));
            for(int i=0;i<3;i++){
                var enemy=(JakEnemy)type.create(level,EntitySpawnReason.COMMAND);
                var s=at(31,1,-4+i*4);enemy.snapTo(s.x,s.y,s.z,0,0);level.addFreshEntity(enemy);enemies.add(enemy);
            }
        });
        if(t==1100)look(at(42,6,10),at(40,1,0));
        if(t>=1110&&t<1300)for(var enemy:c.level.getEntitiesOfClass(JakEnemy.class,c.player.getBoundingBox().inflate(40))){
            String now=enemy.animation(),was=clips.put(enemy.getId(),now);
            if(was!=null&&!was.equals(now))flips.merge(enemy.getId(),1,Integer::sum);
        }
        if(t==1180||t==1240)capture(c,"lote8-enemies-"+t+".png");
        // A plain jump, each drawn frame: how evenly its clips play and where he is.
        if(t==1300){
            view=null;
            onServer(c,p->{for(var e:enemies)e.discard();p.setHealth(p.getMaxHealth());put(p,50,0,-8,90);});
        }
        if(t==1320){
            long start=System.nanoTime();
            dev.jakmc.client.model.JakRenderer.frameTrace=(pose2,frame)->{
                var row=new JsonArray();row.add((System.nanoTime()-start)/1_000_000);row.add(pose2.clip());row.add(Math.round(frame*100)/100f);
                var shown=JakPoses.presented();row.add(shown==null?-1:Math.round((shown.y-base.getY())*1000)/1000.0);
                synchronized(jump){jump.add(row);}
            };
            onServer(c,p->JakPlayers.probeInput(p,1,1,0));
        }
        if(t==1324)onServer(c,p->JakPlayers.probeInput(p,0,0,0));
        // Jak's shine, close and from where the hair's has gone.
        if(t==1358){var jak=c.player.position();look(jak.add(1.6,1.6,1.6),jak.add(0,1.3,0));}
        if(t==1361)capture(c,"lote8-jak-close.png");
        if(t==1363){var jak=c.player.position();look(jak.add(4,2.5,4),jak.add(0,1.2,0));}
        if(t==1366)capture(c,"lote8-jak-far.png");
        if(t==1370){
            dev.jakmc.client.model.JakRenderer.frameTrace=null;
            view=null;
            onServer(c,p->{
                synchronized(jump){result.add("jump",jump);}
                synchronized(flight){result.add("flight",flight);}
                result.add("dive",dive);result.add("launch",launch);
                var seen=new JsonArray();launchClips.forEach(seen::add);result.add("launch_clips",seen);
                var states=new JsonArray();ledgeStates.forEach(states::add);result.add("ledge_states",states);
                result.addProperty("launch_peak",peak);
                var counts=new JsonArray();flips.values().forEach(counts::add);result.add("enemy_clip_changes_in_9s",counts);
                result.addProperty("enemies_seen",clips.size());
                result.addProperty("visual_review","required");result.addProperty("reason","complete");
                JakPlayers.controlForProbe(p,false);
                try{
                    var dir=c.gameDirectory.toPath().resolve("jakmc/evidence/lote8");
                    Files.createDirectories(dir);Files.writeString(dir.resolve("result.json"),result.toString());
                }catch(IOException failure){throw new UncheckedIOException(failure);}
                c.execute(()->{running=false;c.options.pauseOnLostFocus=pause;c.stop();});
            });
        }
    }
    private static void onServer(Minecraft client,java.util.function.Consumer<ServerPlayer> action){
        var server=client.getSingleplayerServer();
        server.execute(()->{var player=server.getPlayerList().getPlayer(owner);if(player!=null)action.accept(player);});
    }
}
