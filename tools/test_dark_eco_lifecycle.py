"""Exercise the production dark-eco death cooldown with isolated level fixtures.

python tools/test_dark_eco_lifecycle.py [--revision b4f8bca]
Compiles the real DIED field and diedInIt method, with fake packet/sound sinks.
Tests cooldown control flow, not Minecraft damage, actual audio or rendering.
Requires JDK 17+; only temporary files are created.
"""
import argparse
import os
from pathlib import Path
import re
import shutil
import subprocess
import tempfile

ROOT = Path(__file__).resolve().parents[1]


def method(source, marker):
    start = source.index(marker)
    opening = source.index("{", start)
    depth = 1
    end = opening + 1
    while depth:
        if source[end] == "{":
            depth += 1
        elif source[end] == "}":
            depth -= 1
        end += 1
    return source[start:end]


FIXTURE = r'''
import java.util.*;
import java.util.function.*;
class Vec3 {Vec3(double x,double y,double z){}}
class ServerLevel {
 long tick;int sounds,bursts;ServerLevel(long tick){this.tick=tick;}
 long getGameTime(){return tick;}Key dimension(){return new Key();}
 void playSound(Object p,double x,double y,double z,Object sound,Object source,int volume,int pitch){sounds++;}
}
class Key {Object identifier(){return this;}}
record ServerPlayer(UUID id){UUID getUUID(){return id;}}
class SoundSource {static final Object PLAYERS=new Object();}
record DarkEcoDeath(Object dimension,Vec3 point){Object type(){return this;}}
class PlayerLookup {static List<ServerPlayer> around(ServerLevel l,Vec3 at,int radius){return List.of();}}
class ServerPlayNetworking {static boolean canSend(ServerPlayer p,Object t){return true;}static void send(ServerPlayer p,Object packet){}}
class Event {List<Consumer<Object>> callbacks=new ArrayList<>();void register(Consumer<Object> c){callbacks.add(c);}void fire(){callbacks.forEach(c->c.accept(new Object()));}}
class ServerLifecycleEvents {static final Event SERVER_STOPPED=new Event();}
class EcoLifecycleCheck {
 static int failures;
 static void check(boolean ok,String name){System.out.println((ok?"PASS ":"FAIL ")+name);if(!ok)failures++;}
 static void die(ServerLevel level,ServerPlayer player){DarkEco.diedInIt(level,player,0,0,0);}
 public static void main(String[] args)throws Exception {
  DarkEco.initialize();
  var player=new ServerPlayer(UUID.randomUUID());var a=new ServerLevel(100);
  die(a,player);die(a,player);check(a.sounds==1&&a.bursts==1,"same tick emits once");
  a.tick=160;die(a,player);check(a.sounds==1,"three-second deduplication remains");
  a.tick=161;die(a,player);check(a.sounds==2,"sound and burst return after cooldown");
  var oldWorld=new ServerLevel(100000);die(oldWorld,player);
  var newWorld=new ServerLevel(100);die(newWorld,player);check(newWorld.sounds==1&&newWorld.bursts==1,"new world clock does not inherit old cooldown");
  var otherDimension=new ServerLevel(100000);die(otherDimension,player);check(otherDimension.sounds==1,"other dimension has its own cooldown");
  var secondPlayer=new ServerPlayer(UUID.randomUUID());die(oldWorld,secondPlayer);check(oldWorld.sounds==2,"other player's death is independent");
  ServerLifecycleEvents.SERVER_STOPPED.fire();
  var field=DarkEco.class.getDeclaredField("DIED");field.setAccessible(true);
  check(((Map<?,?>)field.get(null)).isEmpty(),"server stop releases death history");
  if(failures!=0)throw new AssertionError(failures+" cooldown checks failed");
 }
}
'''


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--revision")
    args = parser.parse_args()
    relative = "src/main/java/dev/jakmc/content/darkeco/DarkEco.java"
    source = subprocess.check_output(["git", "show", f"{args.revision}:{relative}"], cwd=ROOT, text=True, encoding="utf-8") if args.revision else (ROOT / relative).read_text(encoding="utf-8")
    field = re.search(r"    private static final Map[^\n]+ DIED=[^\n]+", source).group(0)
    code = method(source, "private static void diedInIt").replace("private static", "static", 1)
    for old, new in {
        "net.minecraft.world.phys.Vec3": "Vec3",
        "dev.jakmc.network.JakPackets.DarkEcoDeath": "DarkEcoDeath",
        "net.fabricmc.fabric.api.networking.v1.PlayerLookup": "PlayerLookup",
        "net.fabricmc.fabric.api.networking.v1.ServerPlayNetworking": "ServerPlayNetworking",
    }.items():
        code = code.replace(old, new)
    clear = re.search(r"[^\n]*ServerLifecycleEvents\.SERVER_STOPPED[^\n]*", source)
    cleanup = clear.group(0).strip() if clear else ""
    generated = "import java.util.*;\nclass DarkEco {\n" + field + "\nstatic final Object DEATH_SOUND=new Object();\n" + code
    generated += "\nstatic Vec3 over(ServerLevel l,Vec3 p){return p;}\nstatic void burst(ServerLevel l,Vec3 p,boolean b){l.bursts++;}\n"
    generated += "static void initialize(){" + cleanup + "}\n}\n"
    javac = str(Path(os.environ["JAVA_HOME"]) / "bin/javac.exe") if os.environ.get("JAVA_HOME") else shutil.which("javac")
    java = str(Path(javac).with_name("java.exe" if os.name == "nt" else "java"))
    with tempfile.TemporaryDirectory(prefix="jak-eco-check-") as folder:
        work = Path(folder)
        (work / "DarkEco.java").write_text(generated, encoding="utf-8")
        (work / "Fixture.java").write_text(FIXTURE, encoding="utf-8")
        subprocess.run([javac, "-encoding", "UTF-8", "-d", folder, str(work / "DarkEco.java"), str(work / "Fixture.java")], check=True)
        subprocess.run([java, "-cp", folder, "EcoLifecycleCheck"], check=True)


if __name__ == "__main__":
    main()
