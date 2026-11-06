package dev.jakmc.client;

import com.google.gson.JsonArray;
import com.google.gson.JsonObject;
import dev.jakmc.JakData;
import dev.jakmc.entity.JakEntities;
import dev.jakmc.entity.JakYakow;
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
import net.minecraft.core.registries.Registries;
import net.minecraft.data.worldgen.features.TreeFeatures;
import net.minecraft.server.level.ServerPlayer;
import net.minecraft.util.RandomSource;
import net.minecraft.world.InteractionHand;
import net.minecraft.world.effect.MobEffectInstance;
import net.minecraft.world.effect.MobEffects;
import net.minecraft.world.entity.EntitySpawnReason;
import net.minecraft.world.item.ItemStack;
import net.minecraft.world.item.Items;
import net.minecraft.world.level.GameType;
import net.minecraft.world.level.block.Blocks;
import net.minecraft.world.level.block.state.BlockState;
import net.minecraft.world.level.levelgen.Heightmap;
import net.minecraft.world.phys.Vec3;

/**
 * Adrii's 06/11 late batch in one bounded run: every Jak creature and the yakow on low ground with Jak up a
 * one-block step (do they climb it?), the yakow's clock tick by tick on the client (smooth, not in jerks), and
 * Jak jumping at tree trunks (four and five logs, and a real acacia from four sides) to hang from their tops.
 */
final class Lote4Scene {
    private static boolean running,pause;
    private static int tick,yakowId=-1;
    private static UUID owner;
    private static BlockPos base,acacia;
    private static int acaciaHeight;
    private static InputScene.CameraView view;
    private static Runnable restore;
    private static final JsonObject result=new JsonObject();
    private static final LinkedHashMap<String,Integer> mobs=new LinkedHashMap<>();
    private static final LinkedHashMap<String,Double> highest=new LinkedHashMap<>();
    private static final JsonArray yakowClock=new JsonArray(),grabs=new JsonArray();
    private static final LinkedHashSet<String> states=new LinkedHashSet<>();
    private static double peak;
    private static String why="";
    private static float lastAge=-1;
    private static String lastClip="";
    private static int yakowStill,yakowSteps;
    /** The trunks jumped at: where Jak starts (blocks from the base) and his yaw, facing it. */
    private record Jump(String name,double x,double z,float yaw){}
    private static final ArrayList<Jump> jumps=new ArrayList<>();
    private static final int MOBS_END=330,JUMP=70;
    static InputScene.CameraView camera(){return view;}
    static void initialize(){ClientTickEvents.END_CLIENT_TICK.register(Lote4Scene::tick);}

    private static Vec3 at(double x,double y,double z){return Vec3.atBottomCenterOf(base).add(x,y,z);}
    private static void look(Vec3 eye,Vec3 target){
        var d=target.subtract(eye);
        view=new InputScene.CameraView(eye,(float)Math.toDegrees(Math.atan2(-d.x,d.z)),(float)-Math.toDegrees(Math.atan2(d.y,d.horizontalDistance())));
    }
    private static void capture(Minecraft c,String file){Screenshot.grab(c.gameDirectory,file,c.gameRenderer.mainRenderTarget(),1,_->{});}
    private static void put(ServerPlayer p,double x,double y,double z,float yaw){var s=at(x,y,z);p.teleportTo(s.x,s.y,s.z);p.setYRot(yaw);p.setXRot(0);p.setDeltaMovement(Vec3.ZERO);}

    static void start(Minecraft client){
        if(client.player==null||client.getSingleplayerServer()==null)throw new IllegalStateException("Open the development world");
        owner=client.player.getUUID();tick=0;yakowId=-1;result.entrySet().clear();
        mobs.clear();highest.clear();states.clear();jumps.clear();
        for(var a:new JsonArray[]{yakowClock,grabs})while(!a.isEmpty())a.remove(0);
        peak=0;why="";lastAge=-1;lastClip="";yakowStill=yakowSteps=0;
        pause=client.options.pauseOnLostFocus;client.options.pauseOnLostFocus=false;
        client.options.setCameraType(CameraType.THIRD_PERSON_BACK);
        onServer(client,p->{
            var server=p.level().getServer();
            server.getCommands().performPrefixedCommand(server.createCommandSourceStack(),"time set noon");
            server.getCommands().performPrefixedCommand(server.createCommandSourceStack(),"weather clear");
            var level=p.level();
            var old=p.position();var mode=p.gameMode.getGameModeForPlayer();
            boolean enabled=p.getAttachedOrCreate(JakData.ENABLED);
            var saved=new ArrayList<ItemStack>();
            for(int i=0;i<p.getInventory().getContainerSize();i++)saved.add(p.getInventory().getItem(i).copy());
            var near=p.blockPosition().offset(24,0,0);
            int y=near.getY();
            for(int x=-22;x<=46;x++)for(int z=-16;z<=16;z++)y=Math.max(y,level.getHeight(Heightmap.Types.MOTION_BLOCKING_NO_LEAVES,near.getX()+x,near.getZ()+z));
            base=near.atY(y+41);
            var snapshot=new LinkedHashMap<BlockPos,BlockState>();
            var stone=Blocks.SMOOTH_STONE.defaultBlockState();
            // A floor; over x -22..12 from z 0 on, a step one block up to a plateau.
            for(int x=-22;x<=46;x++)for(int z=-16;z<=16;z++)for(int h=-2;h<=14;h++){
                var cell=base.offset(x,h,z);
                snapshot.put(cell.immutable(),level.getBlockState(cell));
                boolean plateau=x<=12&&z>=0&&h==0;
                level.setBlock(cell,h==-1||plateau?stone:Blocks.AIR.defaultBlockState(),2);
            }
            // Trunks: four and five oak logs with nothing on them, and a real acacia (a savanna tree, as in Adrii's picture).
            for(int h=0;h<4;h++)level.setBlock(base.offset(22,h,-8),Blocks.OAK_LOG.defaultBlockState(),2);
            for(int h=0;h<5;h++)level.setBlock(base.offset(28,h,-8),Blocks.OAK_LOG.defaultBlockState(),2);
            // A real acacia (seed 11: six straight logs, then the bend), out of his reach; and one built as in Adrii's
            // picture, four logs then the bend to the east and its canopy, in reach.
            var grownAt=base.offset(38,0,8);
            level.setBlock(grownAt.below(),Blocks.DIRT.defaultBlockState(),2);
            boolean grown=level.registryAccess().lookupOrThrow(Registries.FEATURE).getOrThrow(TreeFeatures.ACACIA).value()
                .place(level,level.getChunkSource().getGenerator(),RandomSource.create(11),grownAt);
            acaciaHeight=0;while(level.getBlockState(grownAt.above(acaciaHeight)).is(Blocks.ACACIA_LOG))acaciaHeight++;
            result.addProperty("acacia_grown",grown);result.addProperty("acacia_straight_logs",acaciaHeight);
            acacia=base.offset(38,0,-4);
            var log=Blocks.ACACIA_LOG.defaultBlockState();
            for(int h=0;h<4;h++)level.setBlock(acacia.above(h),log,2);
            level.setBlock(acacia.offset(1,4,0),log,2);level.setBlock(acacia.offset(2,5,0),log,2);
            for(int x=-1;x<=5;x++)for(int z=-2;z<=2;z++)if(Math.abs(x-2)+Math.abs(z)<=3)level.setBlock(acacia.offset(x,6,z),Blocks.ACACIA_LEAVES.defaultBlockState(),2);
            jumps.add(new Jump("oak-4",22,-4.4,180));
            jumps.add(new Jump("oak-5",28,-4.4,180));
            double ax=38,az=-4;
            jumps.add(new Jump("acacia-grown",ax,8-3.9,0));
            jumps.add(new Jump("acacia-north",ax,az-3.9,0));
            jumps.add(new Jump("acacia-south",ax,az+3.9,180));
            jumps.add(new Jump("acacia-west",ax-3.9,az,-90));
            jumps.add(new Jump("acacia-east",ax+3.9,az,90));
            // Every Jak creature in a row on the low ground, the yakow among them; Jak up on the plateau.
            int i=0;
            for(var e:JakEntities.TYPES.entrySet()){
                var mob=e.getValue().create(level,EntitySpawnReason.COMMAND);
                if(mob==null)continue;
                // Two rows in front of the step, all of them in sight of him (13.8 blocks for the crab and the guard).
                var s=at(-12+i%10*2.2,0,i<10?-3:-5.5);mob.snapTo(s.x,s.y,s.z,0,0);level.addFreshEntity(mob);mob.setPersistenceRequired();
                mobs.put(e.getKey(),mob.getId());i++;
            }
            var yakow=JakYakow.TYPE.create(level,EntitySpawnReason.COMMAND);
            var ys=at(-2,0,-1.5);yakow.snapTo(ys.x,ys.y,ys.z,0,0);level.addFreshEntity(yakow);
            yakowId=yakow.getId();mobs.put("yakow",yakowId);
            p.setGameMode(GameType.SURVIVAL);p.setAttached(JakData.ENABLED,true);
            p.getInventory().clearContent();
            p.setItemInHand(InteractionHand.MAIN_HAND,new ItemStack(Items.WHEAT));
            p.addEffect(new MobEffectInstance(MobEffects.RESISTANCE,20*60,4,false,false));
            p.addEffect(new MobEffectInstance(MobEffects.REGENERATION,20*60,4,false,false));
            put(p,-2,1,5,180);
            JakPlayers.controlForProbe(p,true);JakPlayers.hostControlsForProbe(p);
            restore=()->{
                for(int id:mobs.values()){var m=level.getEntity(id);if(m!=null)m.discard();}
                snapshot.forEach((cell,state)->level.setBlock(cell,state,2));
                p.removeAllEffects();p.getInventory().clearContent();
                for(int k=0;k<saved.size();k++)p.getInventory().setItem(k,saved.get(k));
                JakPlayers.controlForProbe(p,false);
                p.setAttached(JakData.ENABLED,enabled);
                p.setGameMode(mode);p.teleportTo(old.x,old.y,old.z);
            };
            client.execute(()->{look(at(-2,7,9),at(-2,0,-4));running=true;});
        });
    }

    private static void tick(Minecraft c){
        if(!running)return;
        int t=++tick;
        c.gui.setScreen(null);
        var pose=JakPoses.get(c.player.getId());
        String state=pose==null?"":pose.raw().get("state").getAsString();
        double feet=pose==null?0:pose.position().y-at(0,0,0).y;
        // The creatures: how high each gets over the low ground (the plateau's top is one block up).
        if(t<MOBS_END)onServer(c,p->{
            for(var e:mobs.entrySet()){
                var m=p.level().getEntity(e.getValue());
                if(m!=null)highest.merge(e.getKey(),m.getY()-at(0,0,0).y,Math::max);
            }
        });
        // The yakow's clock as drawn, each client tick: a still clock while it walks is a jerk.
        if(t>=40&&t<MOBS_END&&c.level.getEntity(yakowId) instanceof JakYakow yakow){
            float age=yakow.animationAge(0);String clip=yakow.clip();
            if(clip.equals(lastClip)&&(clip.equals("yakow-walk")||clip.equals("yakow-run"))){
                yakowSteps++;if(Math.abs(age-lastAge)<1e-4)yakowStill++;
                if(yakowClock.size()<60)yakowClock.add(clip.substring(6)+" "+Math.round((age-lastAge)*1000)/1000.0);
            }
            lastAge=age;lastClip=clip;
        }
        if(t==120)capture(c,"lote4-mobs-120.png");
        if(t==200){look(at(-12,4,4),at(-12,0,-3));}
        if(t==204)capture(c,"lote4-mobs-step.png");
        if(t==300)capture(c,"lote4-mobs-300.png");
        if(t==MOBS_END){
            view=null;
            onServer(c,p->{for(int id:mobs.values()){var m=p.level().getEntity(id);if(m!=null)m.discard();}
                p.setGameMode(GameType.CREATIVE);});
            var climbed=new JsonObject();highest.forEach((k,v)->climbed.addProperty(k,Math.round(v*100)/100.0));
            result.add("mob_highest_over_low_ground",climbed);
            result.addProperty("yakow_walking_ticks",yakowSteps);result.addProperty("yakow_still_ticks",yakowStill);result.add("yakow_clock",yakowClock);
        }
        // The trunks: run, jump, jump again at the top of the first, the stick on them throughout.
        int k=t-MOBS_END-10;
        if(k>=0&&k<jumps.size()*JUMP){
            var jump=jumps.get(k/JUMP);int s=k%JUMP;
            if(s==0){states.clear();peak=0;why="";onServer(c,p->{JakPlayers.probeInput(p,0,0,0);put(p,jump.x,0,jump.z,jump.yaw);});
                double dx=-Math.sin(Math.toRadians(jump.yaw)),dz=Math.cos(Math.toRadians(jump.yaw));
                look(at(jump.x-dz*6-dx*2,3.5,jump.z+dx*6-dz*2),at(jump.x+dx*3,2.5,jump.z+dz*3));}
            if(s==4)c.player.setYRot(jump.yaw);
            if(s==6)onServer(c,p->{p.setYRot(jump.yaw);JakPlayers.probeInput(p,0,0,1);});
            if(s==9)onServer(c,p->JakPlayers.probeInput(p,1,1,1));
            // The second press a third of a second after the first (the highest of the native sweep).
            if(s==14)onServer(c,p->JakPlayers.probeInput(p,0,0,1));
            if(s==16)onServer(c,p->JakPlayers.probeInput(p,1,1,1));
            if(s>=6&&s<60){if(!state.isEmpty())states.add(state);peak=Math.max(peak,feet);
                if(why.isEmpty()&&pose!=null&&state.startsWith("falling")&&pose.raw().has("ledgeWhy"))why=pose.raw().get("ledgeWhy").getAsString();}
            if(s==30||s==48)capture(c,"lote4-"+jump.name+"-"+s+".png");
            if(s==60)onServer(c,p->JakPlayers.probeInput(p,0,0,0));
            if(s==JUMP-1){
                var row=new JsonObject();row.addProperty("trunk",jump.name);row.addProperty("grabbed",states.stream().anyMatch(v->v.startsWith("edge-grab")));
                row.addProperty("peak_feet",Math.round(peak*100)/100.0);row.add("states",strings(states));row.addProperty("ledge_why",why);
                grabs.add(row);
            }
        }
        if(k==jumps.size()*JUMP+2){
            view=null;
            result.add("trunk_jumps",grabs);
            result.addProperty("visual_review","required");result.addProperty("reason","complete");
            onServer(c,p->{
                try{
                    var dir=c.gameDirectory.toPath().resolve("jakmc/evidence/lote4");
                    Files.createDirectories(dir);Files.writeString(dir.resolve("result.json"),result.toString());
                }catch(IOException failure){throw new UncheckedIOException(failure);}
                restore.run();
                c.execute(()->{running=false;view=null;c.options.pauseOnLostFocus=pause;c.stop();});
            });
        }
    }
    private static JsonArray strings(java.util.Collection<String> values){var out=new JsonArray();values.forEach(out::add);return out;}
    private static void onServer(Minecraft client,java.util.function.Consumer<ServerPlayer> action){
        var server=client.getSingleplayerServer();
        server.execute(()->{var player=server.getPlayerList().getPlayer(owner);if(player!=null)action.accept(player);});
    }
}
