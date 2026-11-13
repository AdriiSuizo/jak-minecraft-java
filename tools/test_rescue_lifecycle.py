"""Run the real JakRescue source against an isolated server fixture (JDK 17+).

python tools/test_rescue_lifecycle.py
Only imports/package names are adapted. This exercises rescue control flow and
Fabric lifecycle callbacks; it is not a Minecraft collision/integration test.
No player worlds, launcher configuration or game assets are read or changed.
"""
from pathlib import Path
import argparse
import os
import re
import shutil
import subprocess
import tempfile

ROOT = Path(__file__).resolve().parents[1]
FIXTURE = r'''
import java.util.*;
import java.util.function.*;

class Vec3 {
 final double x,y,z;
 Vec3(double x,double y,double z){this.x=x;this.y=y;this.z=z;}
 Vec3 add(double x,double y,double z){return new Vec3(this.x+x,this.y+y,this.z+z);}
 Vec3 add(Vec3 o){return add(o.x,o.y,o.z);}
 Vec3 scale(double n){return new Vec3(x*n,y*n,z*n);}
 static Vec3 atBottomCenterOf(BlockPos p){return new Vec3(p.x+.5,p.y,p.z+.5);}
}
class BlockPos {
 final int x,y,z;
 BlockPos(int x,int y,int z){this.x=x;this.y=y;this.z=z;}
 static BlockPos containing(double x,double y,double z){return new BlockPos((int)Math.floor(x),(int)Math.floor(y),(int)Math.floor(z));}
 int getX(){return x;} int getY(){return y;} int getZ(){return z;}
}
class Shape { final boolean full; Shape(boolean full){this.full=full;} boolean isEmpty(){return !full;} }
class Block { static boolean isShapeFullBlock(Shape s){return s.full;} }
class BlockState {
 final boolean solid; BlockState(boolean solid){this.solid=solid;}
 Shape getCollisionShape(Level l,BlockPos p){return new Shape(solid);}
 Block getBlock(){return new Block();}
}
class Key {String getPath(){return "stone";}}
class Registry {Key getKey(Block b){return new Key();}}
class BuiltInRegistries {static final Registry BLOCK=new Registry();}
class Heightmap {enum Types {MOTION_BLOCKING_NO_LEAVES}}
record RespawnData(Object dimension,BlockPos pos){}
record RespawnConfig(RespawnData respawnData){}
class Level {
 static final Object OVERWORLD=new Object(),NETHER=new Object();
 final Object key; Level(Object key){this.key=key;}
 Object dimension(){return key;}
 boolean hasChunk(int x,int z){return true;}
 int getMinY(){return -64;} int getMaxY(){return 320;}
 int getHeight(Heightmap.Types type,int x,int z){return 64;}
 BlockState getBlockState(BlockPos at){return new BlockState(at.y>=0&&at.y<64);}
 RespawnData getRespawnData(){return new RespawnData(key,new BlockPos(0,64,0));}
}
class ServerLevel extends Level {ServerLevel(Object key){super(key);}}
class Food {int n=20; int getFoodLevel(){return n;} void setFoodLevel(int n){this.n=n;}}
class ServerPlayer {
 final UUID id; ServerLevel level; Vec3 at; boolean ground,alive=true,spectator; int teleports;
 float yBodyRot; Object containerMenu,inventoryMenu; Food food=new Food();
 ServerPlayer(UUID id,ServerLevel level,Vec3 at){this.id=id;this.level=level;this.at=at;}
 UUID getUUID(){return id;} ServerLevel level(){return level;} Vec3 position(){return at;}
 boolean isAlive(){return alive;} boolean isSpectator(){return spectator;} boolean onGround(){return ground;}
 boolean getAttachedOrCreate(Object k){return true;} boolean isSleeping(){return false;}
 void closeContainer(){} void stopSleepInBed(boolean a,boolean b){} void stopRiding(){}
 Food getFoodData(){return food;} float getHealth(){return 20;} float getMaxHealth(){return 20;} void heal(float n){}
 RespawnConfig getRespawnConfig(){return null;} float getYRot(){return 0;} float getXRot(){return 0;}
 Vec3 getEyePosition(){return at.add(0,1.62,0);} String getScoreboardName(){return "Fixture";}
 void sendSystemMessage(Object message,boolean overlay){}
 void teleportTo(ServerLevel level,double x,double y,double z,Set<?> flags,float yaw,float pitch,boolean a){at=new Vec3(x,y,z);teleports++;}
}
class JakPlayers {static boolean active(ServerPlayer p){return true;} static void rescue(ServerPlayer p){} static void chooseMode(ServerPlayer p,boolean v){}}
class JakData {static final Object ENABLED=new Object(); static String id(String s){return s;}}
class JakMod {static final Log LOG=new Log(); static class Log {void info(String s,Object...args){}}}
class Component {static String literal(String s){return s;}}
class RegistryFriendlyByteBuf {void writeVarInt(int n){} int readVarInt(){return 0;}}
interface CustomPacketPayload {record Type<T>(Object id){} Type<?> type();}
class StreamCodec<B,V> {static <B,V> StreamCodec<B,V> of(BiConsumer<B,V> write,Function<B,V> read){return new StreamCodec<>();}}
class PayloadTypeRegistry {static PayloadTypeRegistry serverboundPlay(){return new PayloadTypeRegistry();} <T> void register(CustomPacketPayload.Type<T> t,StreamCodec<?,T> c){}}
record Context(ServerPlayer player){}
class ServerPlayNetworking {static <T> void registerGlobalReceiver(CustomPacketPayload.Type<T> t,BiConsumer<T,Context> c){}}
record PlayerList(List<ServerPlayer> players){List<ServerPlayer> getPlayers(){return players;}}
record Server(PlayerList players){PlayerList getPlayerList(){return players;}}
class Event {List<Consumer<Server>> callbacks=new ArrayList<>();void register(Consumer<Server> f){callbacks.add(f);}void fire(Server s){callbacks.forEach(f->f.accept(s));}}
class ServerTickEvents {static final Event END_SERVER_TICK=new Event();}
class ServerLifecycleEvents {static final Event SERVER_STOPPED=new Event();}
record Handler(ServerPlayer player){ServerPlayer getPlayer(){return player;}}
class Disconnect {List<BiConsumer<Handler,Server>> callbacks=new ArrayList<>();void register(BiConsumer<Handler,Server> f){callbacks.add(f);}void fire(ServerPlayer p){callbacks.forEach(f->f.accept(new Handler(p),new Server(new PlayerList(List.of()))));}}
class ServerPlayConnectionEvents {static final Disconnect DISCONNECT=new Disconnect();}

class RescueLifecycleCheck {
 static int failures;
 static void check(boolean condition,String name){System.out.println((condition?"PASS ":"FAIL ")+name);if(!condition)failures++;}
 static ServerPlayer grounded(ServerLevel l){var p=new ServerPlayer(UUID.randomUUID(),l,new Vec3(80,64,90));p.ground=true;JakRescue.watchFall(p);p.ground=false;return p;}
 static void fall(ServerPlayer p){p.at=new Vec3(0,-100,0);JakRescue.watchFall(p);JakRescue.watchFall(p);}
 static void checkSpawn(ServerPlayer p,String test){check(Math.abs(p.at.x)<2&&Math.abs(p.at.z)<2&&p.at.y>=64,test);}
 public static void main(String[] args)throws Exception {
  JakRescue.initialize();
  var a=new ServerLevel(Level.OVERWORLD);
  var normal=grounded(a);fall(normal);check(normal.at.x>79&&normal.at.z>89,"same-world fall keeps last safe ground");
  var dimension=grounded(a);dimension.level=new ServerLevel(Level.NETHER);fall(dimension);checkSpawn(dimension,"dimension change discards other dimension coordinates");
  var world=grounded(a);world.level=new ServerLevel(Level.OVERWORLD);fall(world);checkSpawn(world,"new world with same dimension key discards old coordinates");
  var original=grounded(a);ServerPlayConnectionEvents.DISCONNECT.fire(original);
  var reconnect=new ServerPlayer(original.id,a,new Vec3(0,-100,0));fall(reconnect);checkSpawn(reconnect,"reconnection discards former session coordinates");
  var dead=grounded(a);var respawned=new ServerPlayer(dead.id,a,new Vec3(0,-100,0));fall(respawned);checkSpawn(respawned,"respawned entity discards former life coordinates");
  var manual=grounded(a);manual.level=new ServerLevel(Level.NETHER);JakRescue.rescue(manual);fall(manual);checkSpawn(manual,"manual rescue updates current-world anchor");
  var spectator=grounded(a);spectator.spectator=true;fall(spectator);check(spectator.teleports==0,"spectators remain untouched");
  ServerLifecycleEvents.SERVER_STOPPED.fire(new Server(new PlayerList(List.of())));
  var field=JakRescue.class.getDeclaredField("WATCH");field.setAccessible(true);
  check(((Map<?,?>)field.get(null)).isEmpty(),"server stop releases rescue state");
  if(failures!=0)throw new AssertionError(failures+" lifecycle checks failed");
 }
}
'''


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--revision", help="Read JakRescue.java from this Git revision without changing the working tree")
    args = parser.parse_args()
    java_home = os.environ.get("JAVA_HOME")
    javac = str(Path(java_home) / "bin/javac.exe") if java_home else shutil.which("javac")
    java = str(Path(javac).with_name("java.exe")) if os.name == "nt" else str(Path(javac).with_name("java"))
    relative = "src/main/java/dev/jakmc/player/JakRescue.java"
    source = subprocess.check_output(["git", "show", f"{args.revision}:{relative}"], cwd=ROOT, text=True, encoding="utf-8") if args.revision else (ROOT / relative).read_text(encoding="utf-8")
    source = re.sub(r"^package .*;\s*", "", source, count=1)
    source = re.sub(r"^import (?:dev|net)\..*;\s*", "", source, flags=re.MULTILINE)
    with tempfile.TemporaryDirectory(prefix="jak-rescue-check-") as folder:
        work = Path(folder)
        (work / "JakRescue.java").write_text(source, encoding="utf-8")
        (work / "Fixture.java").write_text(FIXTURE, encoding="utf-8")
        subprocess.run([javac, "-encoding", "UTF-8", "-d", folder, str(work / "JakRescue.java"), str(work / "Fixture.java")], check=True)
        subprocess.run([java, "-cp", folder, "RescueLifecycleCheck"], check=True)


if __name__ == "__main__":
    main()
