package dev.jakmc.ecology;

import dev.jakmc.entity.JakEntities;
import dev.jakmc.player.JakEntityClock;
import java.util.*;
import net.fabricmc.fabric.api.event.lifecycle.v1.*;
import net.minecraft.core.*;
import net.minecraft.server.level.*;
import net.minecraft.tags.FluidTags;
import net.minecraft.world.Difficulty;
import net.minecraft.world.attribute.EnvironmentAttributes;
import net.minecraft.world.entity.*;
import net.minecraft.world.entity.ai.village.poi.*;
import net.minecraft.world.level.*;
import net.minecraft.world.level.block.Blocks;
import net.minecraft.world.level.chunk.LevelChunk;
import net.minecraft.world.level.levelgen.Heightmap;
import net.minecraft.world.phys.*;
import net.minecraft.world.phys.shapes.Shapes;

/** Jak's enemy ecology in the Overworld: nests with their bands, killed-out bands
 * coming again, night prowlers and the city Metal Head by the villages.
 * Rust: crates/render_anim/src/jak_enemies.rs (state kept per session, as there). */
public final class JakEcology {
 // Rust: jak_enemies.rs CELL, NESTS_IN, NEAR, FAR, RESPAWN, PROWLERS, PROWL_NEAR/FAR/GONE (blocks, seconds)
 static final int CELL=80;static final long NESTS_IN=55;static final double NEAR=56,FAR=110;static final float RESPAWN=240;
 static final int PROWLERS=4;static final double PROWL_NEAR=24,PROWL_FAR=44,PROWL_GONE=96;
 /** Rust's map metre (M=25 units) in blocks (BLOCK=36 units). */
 static final double M=25/36.0;
 // Rust: jak_enemies.rs VILLAGE_PROWLER (KINDS[16])
 static final String VILLAGE_PROWLER="predator";
 public static final String NEST_TAG="jakmc.nest",PROWLER_TAG="jakmc.prowler";
 // Rust: jak_enemies.rs CAVE_KEY (a crater cave's nest keyed by its mark, apart from the squares'); jak_structures.rs CAVE_MARK
 static final int CAVE_KEY=0x2000_0000;
 static final net.minecraft.world.level.block.Block CAVE_MARK=Blocks.CHISELED_POLISHED_BLACKSTONE;
 private static final class Nest{final int cx,cz;final boolean cave;final long key;String[] band=new String[0];Vec3 at;boolean out;float retry;final Set<UUID> members=new HashSet<>();
  Nest(int cx,int cz){this.cx=cx;this.cz=cz;cave=false;key=key(cx,cz);}
  // A cave's nest: cx, cz are its mark's block.
  Nest(BlockPos mark){cx=mark.getX();cz=mark.getZ();cave=true;key=key(cx,cz+CAVE_KEY);}
  double x(){return cave?cx:nestOf(cx,cz)[0];}
  double z(){return cave?cz:nestOf(cx,cz)[1];}
 }
 // Rust: jak2_villagers.rs marks_in / set_chunk_marks / cave_marks (the marks of the loaded chunks)
 private static final Map<Long,List<BlockPos>> caveMarks=new HashMap<>();
 private static final Map<Long,Nest> nests=new HashMap<>();
 private static final Map<Long,Float> cleared=new HashMap<>();
 private static final Map<UUID,Long> memberOf=new HashMap<>();
 private static final Map<UUID,UUID> prowlers=new HashMap<>();
 private static final Map<UUID,Float> prowlWait=new HashMap<>();
 private static final List<Entity> drop=new ArrayList<>();
 private static long rng=0x6c75726b65727321L,last=-1;private static float clock;

 public static void initialize(){
  ServerTickEvents.END_LEVEL_TICK.register(JakEcology::tick);
  ServerEntityEvents.ENTITY_LOAD.register((e,level)->{if((e.entityTags().contains(NEST_TAG)||e.entityTags().contains(PROWLER_TAG))&&!memberOf.containsKey(e.getUUID())&&!prowlers.containsKey(e.getUUID()))drop.add(e);});
  ServerEntityEvents.ENTITY_UNLOAD.register(JakEcology::gone);
  ServerChunkEvents.CHUNK_LOAD.register((level,chunk,generated)->{if(level.dimension()==Level.OVERWORLD)marks(chunk);});
  ServerChunkEvents.CHUNK_UNLOAD.register((level,chunk)->{if(level.dimension()==Level.OVERWORLD)caveMarks.remove(chunk.getPos().pack());});
  ServerLifecycleEvents.SERVER_STOPPED.register(server->clear());
 }
 // Rust: jak_enemies.rs clear
 static void clear(){caveMarks.clear();nests.clear();cleared.clear();memberOf.clear();prowlers.clear();prowlWait.clear();drop.clear();rng=0x6c75726b65727321L;last=-1;clock=0;}
 // Rust: jak_enemies.rs Enemies::random
 static float random(){long x=rng==0?1:rng;x^=x>>>12;x^=x<<25;x^=x>>>27;rng=x;return (float)((x*0x2545F4914F6CDD1DL)>>>40)/(1<<24);}
 // Rust: jak_enemies.rs mix
 static long mix(int a,int b,long salt){
  long h=Integer.toUnsignedLong(a)<<32|Integer.toUnsignedLong(b);h^=salt*0x9E3779B97F4A7C15L;
  h^=h>>>33;h*=0xff51afd7ed558ccdL;h^=h>>>33;h*=0xc4ceb9fe1a85ec53L;return h^(h>>>33);
 }
 // Rust: jak_enemies.rs nest_of (block x, z and the band pick; null for a square without one)
 static long[] nestOf(int cx,int cz){
  if(Long.remainderUnsigned(mix(cx,cz,1),100)>=NESTS_IN)return null;
  return new long[]{cx*CELL+16+Long.remainderUnsigned(mix(cx,cz,2),CELL-32),cz*CELL+16+Long.remainderUnsigned(mix(cx,cz,3),CELL-32),mix(cx,cz,4)};
 }
 static long key(int cx,int cz){return (long)cx<<32|(cz&0xffffffffL);}
 // Rust: jak2_villagers.rs marks_in (each CAVE_MARK block of a chunk as it loads)
 private static void marks(LevelChunk chunk){
  var found=new ArrayList<BlockPos>();
  for(int i=0;i<chunk.getSectionsCount();i++){
   var section=chunk.getSection(i);
   if(section.hasOnlyAir()||!section.maybeHas(s->s.is(CAVE_MARK)))continue;
   int y0=chunk.getSectionYFromSectionIndex(i)<<4;
   for(int x=0;x<16;x++)for(int y=0;y<16;y++)for(int z=0;z<16;z++)
    if(section.getBlockState(x,y,z).is(CAVE_MARK))found.add(new BlockPos(chunk.getPos().getMinBlockX()+x,y0+y,chunk.getPos().getMinBlockZ()+z));
  }
  if(found.isEmpty())caveMarks.remove(chunk.getPos().pack());else caveMarks.put(chunk.getPos().pack(),found);
 }
 /** Rust: jak_enemies.rs `mc summon jak:<model>` (Enemy::new with no nest): a summoned enemy is a prowler. */
 public static void adoptProwler(ServerLevel level,Entity e){
  if(level.dimension()!=Level.OVERWORLD)return;
  var owner=level.getNearestPlayer(e,-1);if(owner==null)return;
  e.addTag(PROWLER_TAG);prowlers.put(e.getUUID(),owner.getUUID());
 }

 // Rust: jak_enemies.rs step (the nest, band, respawn and prowler parts)
 static void tick(ServerLevel level){
  if(level.dimension()!=Level.OVERWORLD)return;
  long now=JakEntityClock.time(level);float dt=last<0?0:Math.min(.1f,Math.max(0,now-last)/20f);last=now;clock+=dt;
  var dropping=List.copyOf(drop);drop.clear();for(var e:dropping)if(!e.isRemoved())e.discard();
  var players=level.players().stream().filter(p->!p.isSpectator()).toList();
  if(players.isEmpty())return;
  // Rust: minecraft_world.rs set_outside(!bright_outside) with bright_outside = sky_light_level() > 11
  boolean night=!(level.environmentAttributes().getDimensionValue(EnvironmentAttributes.SKY_LIGHT_LEVEL)>11);
  // stand-in: Minecraft removes monsters in Peaceful; the original has no difficulty, so nothing is sent then.
  boolean peaceful=level.getDifficulty()==Difficulty.PEACEFUL;
  for(var p:players){
   int hx=Math.floorDiv((int)Math.floor(p.getX()),CELL),hz=Math.floorDiv((int)Math.floor(p.getZ()),CELL);
   for(int dx=-1;dx<=1;dx++)for(int dz=-1;dz<=1;dz++){
    int cx=hx+dx,cz=hz+dz;if(nests.containsKey(key(cx,cz)))continue;
    var n=nestOf(cx,cz);if(n==null)continue;
    double d=Math.hypot(n[0]-p.getX(),n[1]-p.getZ());
    if(d>NEAR||bellWithin(level,new BlockPos((int)n[0],(int)p.getY(),(int)n[1]),40))continue;
    nests.put(key(cx,cz),new Nest(cx,cz));
   }
  }
  // The crater caves' chambers: a band about each mark (Rust: step, cave_marks).
  for(var marks:caveMarks.values())for(var mark:marks){
   var nest=new Nest(mark);if(nests.containsKey(nest.key))continue;
   if(players.stream().noneMatch(p->Math.hypot(mark.getX()-p.getX(),mark.getZ()-p.getZ())<=NEAR))continue;
   nest.at=new Vec3(mark.getX()+.5,mark.getY()+1,mark.getZ()+.5);
   // Rust: jak_enemies.rs cave_band
   nest.band=JakHabitat.CAVE.bandOf(mix(mark.getX(),mark.getZ(),5));
   nests.put(nest.key,nest);
  }
  for(var it=nests.entrySet().iterator();it.hasNext();){
   var nest=it.next().getValue();double nx=nest.x(),nz=nest.z();
   if(players.stream().allMatch(p->Math.hypot(nx-p.getX(),nz-p.getZ())>FAR)){it.remove();forget(level,nest);}
  }
  for(var nest:List.copyOf(nests.values())){
   int x=(int)nest.x(),z=(int)nest.z();
   if(nest.at==null){
    nest.retry-=dt;if(nest.retry>0)continue;nest.retry=2;
    if(!level.hasChunkAt(x,z))continue;
    var anchor=players.stream().min(Comparator.comparingDouble(p->Math.hypot(x-p.getX(),z-p.getZ()))).orElseThrow();
    var column=new Vec3(x+.5,anchor.getY()+40,z+.5);double y=freeSpot(level,column,90,anchor.getY(),48);
    if(Double.isNaN(y))continue;nest.at=new Vec3(column.x,y,column.z);
   }
   if(nest.band.length==0){
    var pos=new BlockPos(x,(int)Math.floor(nest.at.y),z);if(!level.hasChunkAt(pos))continue;
    nest.band=habitat(level,pos).bandOf(nestOf(nest.cx,nest.cz)[2]);
    if(nest.band.length>0)dev.jakmc.JakMod.LOG.info("Jak nest at {} {} {}: its band {}",x,pos.getY(),z,Arrays.toString(nest.band));
   }
   if(nest.out||nest.band.length==0||peaceful)continue;
   var due=cleared.get(nest.key);if(due!=null&&clock<due)continue;
   // stand-in: the Rust asks a dark spot of a cave's band too, but its chamber's own lantern (8 blocks off)
   // and lava light every spot there, so it never came; a cave's band comes regardless of light.
   if(!nest.cave&&!unlit(level,nest.at))continue;
   if(nest.cave&&!level.hasChunkAt(BlockPos.containing(nest.at)))continue;
   cleared.remove(nest.key);nest.out=true;
   for(int k=0;k<nest.band.length;k++){
    var type=JakEntities.TYPES.get(nest.band[k]);if(type==null)continue;
    float angle=k*2.1f+random();double far=2+random()*3;Vec3 spot=nest.at.add(Math.cos(angle)*far,0,Math.sin(angle)*far);
    var size=type.getDimensions();
    Vec3 feet=spawn(level,spot.add(0,3,0),8,nest.at.y,3,size.width()/2,size.height());
    if(feet==null||!nest.cave&&!unlit(level,feet))continue;
    float yaw=random()*360;random();
    var e=type.create(level,EntitySpawnReason.NATURAL);if(e==null)continue;
    e.snapTo(feet.x,feet.y,feet.z,yaw,0);e.setPersistenceRequired();e.addTag(NEST_TAG);
    nest.members.add(e.getUUID());memberOf.put(e.getUUID(),nest.key);
    if(!level.addFreshEntity(e)){nest.members.remove(e.getUUID());memberOf.remove(e.getUUID());}
   }
  }
  // At night a few prowl about each player; by day those far off go, and new ones stop coming.
  // stand-in: the original prowls about player one only; here each player has their own (owner below).
  for(var it=prowlers.entrySet().iterator();it.hasNext();){
   var en=it.next();var e=level.getEntity(en.getKey());
   if(e==null){it.remove();continue;}
   var owner=level.getPlayerByUUID(en.getValue());
   double d=owner==null?Double.MAX_VALUE:Math.hypot(e.getX()-owner.getX(),e.getZ()-owner.getZ());
   if(!(d<PROWL_GONE&&(night||d<PROWL_NEAR))){it.remove();e.discard();}
  }
  for(var p:players){
   float wait=prowlWait.getOrDefault(p.getUUID(),0f)-dt;
   long prowling=prowlers.values().stream().filter(p.getUUID()::equals).count();
   if(night&&!peaceful&&wait<=0&&prowling<PROWLERS){
    wait=8+random()*14;
    float angle=random()*(float)(Math.PI*2);double reach=PROWL_NEAR+random()*(PROWL_FAR-PROWL_NEAR);
    Vec3 at=p.position().add(Math.cos(angle)*reach,0,Math.sin(angle)*reach);
    var block=BlockPos.containing(at);int pick=(int)(random()*1000);
    boolean village=bellWithin(level,block,48);String model=null;
    if(village)model=VILLAGE_PROWLER;
    else if(level.hasChunkAt(block)){var band=habitat(level,block).bandOf(pick);if(band.length>0)model=band[(pick/7)%band.length];}
    if(model==null)wait=1;
    var type=model==null?null:JakEntities.TYPES.get(model);
    if(type!=null){
     var size=type.getDimensions();Vec3 feet;
     if(village){var street=streetGroundNear(level,at,24);feet=street==null?null:spawn(level,street.add(0,1.5,0),3,street.y,1,size.width()/2,size.height());}
     else feet=spawn(level,new Vec3(at.x,p.getY()+24,at.z),48,p.getY(),12,size.width()/2,size.height());
     if(feet!=null&&unlit(level,feet)){
      random();var e=type.create(level,EntitySpawnReason.NATURAL);
      if(e!=null){
       double fx=p.getX()-feet.x,fz=p.getZ()-feet.z;
       e.snapTo(feet.x,feet.y,feet.z,(float)Math.toDegrees(Math.atan2(-fx,fz)),0);e.setPersistenceRequired();e.addTag(PROWLER_TAG);
       prowlers.put(e.getUUID(),p.getUUID());if(!level.addFreshEntity(e))prowlers.remove(e.getUUID());
      }
     }
    }
   }
   prowlWait.put(p.getUUID(),wait);
  }
 }
 /** A nest past FAR goes with its band (Rust: nests.remove + enemies.retain in step). */
 private static void forget(ServerLevel level,Nest nest){
  for(var id:nest.members){memberOf.remove(id);var e=level.getEntity(id);if(e!=null&&!e.isRemoved())drop.add(e);}nest.members.clear();
 }
 // Rust: jak_enemies.rs step, "a band killed out comes again in a while" (gone_from / cleared)
 private static void gone(Entity e,ServerLevel level){
  prowlers.remove(e.getUUID());
  Long cell=memberOf.remove(e.getUUID());if(cell==null)return;
  var nest=nests.get(cell);if(nest==null)return;nest.members.remove(e.getUUID());
  var reason=e.getRemovalReason();
  if(reason!=null&&reason.shouldDestroy()){if(nest.members.isEmpty()){nest.out=false;cleared.put(cell,clock+RESPAWN);}return;}
  // stand-in: the original keeps every enemy in memory until its nest is past FAR; Minecraft saves an
  // unloaded one into its chunk. The nest goes as past FAR (and comes back whole), the saved copy is dropped on load.
  nests.remove(cell);forget(level,nest);
 }
 static JakHabitat habitat(ServerLevel level,BlockPos pos){return JakHabitat.of(level.getBiome(pos).unwrapKey().map(k->k.identifier().toString()).orElse(""));}
 // Rust: jak2_villagers.rs bell_within (bells within `reach` in x and z)
 static boolean bellWithin(ServerLevel level,BlockPos at,int reach){
  return level.getPoiManager().getInSquare(h->h.is(PoiTypes.MEETING),at,reach,PoiManager.Occupancy.ANY).findAny().isPresent();
 }
 // Rust: jak_enemies.rs unlit_spawn (block light 0 in a ready column)
 static boolean unlit(ServerLevel level,Vec3 at){var pos=BlockPos.containing(at);return level.hasChunkAt(pos)&&level.getBrightness(LightLayer.BLOCK,pos)==0;}
 // Rust: creature_body.rs ground (a flat box swept down `reach`; NaN when inside or nothing under)
 static double ground(ServerLevel level,Vec3 at,double reach,double half){
  var box=new AABB(at.x-half,at.y,at.z-half,at.x+half,at.y+.1*M,at.z+half);
  if(!level.noBlockCollision(null,box))return Double.NaN;
  double moved=Shapes.collide(Direction.Axis.Y,box,level.getBlockCollisions(null,box.expandTowards(0,-reach,0)),-reach);
  return moved>-reach+1e-7?at.y+moved:Double.NaN;
 }
 // Rust: jak2_villagers.rs free_spot (ground with two blocks clear over it, within `rise` of `level`)
 static double freeSpot(ServerLevel level,Vec3 at,double reach,double levelY,double rise){
  double y=ground(level,at,reach,.25*M);if(Double.isNaN(y)||Math.abs(y-levelY)>rise)return Double.NaN;
  double r=.3*M;return level.noBlockCollision(null,new AABB(at.x-r,y+.05,at.z-r,at.x+r,y+2+.1*M,at.z+r))?y:Double.NaN;
 }
 // Rust: creature_navigation.rs spawn (even ground under the whole footprint, no lava, room for the body)
 static Vec3 spawn(ServerLevel level,Vec3 at,double reach,double levelY,double rise,double half,double height){
  if(!level.hasChunkAt(BlockPos.containing(at)))return null;
  double y=ground(level,at,reach,half*.25);if(Double.isNaN(y)||Math.abs(y-levelY)>rise)return null;
  double r=half*.8,lo=y,hi=y;
  for(int i=0;i<4;i++){
   double h=ground(level,new Vec3(at.x+((i&1)==0?-r:r),y+.7,at.z+((i&2)==0?-r:r)),1.4,.03);
   if(Double.isNaN(h))return null;lo=Math.min(lo,h);hi=Math.max(hi,h);
  }
  if(hi-lo>.6)return null;
  Vec3 feet=new Vec3(at.x,hi,at.z);
  // Rust: sim/src/voxel.rs eyes_in_fluid(feet + 0.1 block, lava)
  double eye=feet.y+.1-.11111111;var pos=BlockPos.containing(feet.x,eye,feet.z);var fluid=level.getFluidState(pos);
  if(fluid.is(FluidTags.LAVA)&&pos.getY()+fluid.getHeight(level,pos)>eye)return null;
  return level.noBlockCollision(null,new AABB(feet.x-half,feet.y+.02,feet.z-half,feet.x+half,feet.y+.02+height,feet.z+half))?feet:null;
 }
 // stand-in: Rust jak2_villagers.rs street_ground_near walks its villages' own street navigation; vanilla
 // villages have none, so their streets are taken as the dirt paths: the nearest path top within `reach`.
 static Vec3 streetGroundNear(ServerLevel level,Vec3 at,int reach){
  Vec3 best=null;double bestD=reach*reach;var pos=new BlockPos.MutableBlockPos();
  for(int dx=-reach;dx<=reach;dx++)for(int dz=-reach;dz<=reach;dz++){
   int x=(int)Math.floor(at.x)+dx,z=(int)Math.floor(at.z)+dz;if(!level.hasChunkAt(x,z))continue;
   int top=level.getHeight(Heightmap.Types.MOTION_BLOCKING_NO_LEAVES,x,z);
   if(!level.getBlockState(pos.set(x,top-1,z)).is(Blocks.DIRT_PATH))continue;
   var p=new Vec3(x+.5,top-1+15/16.0,z+.5);double d=p.distanceToSqr(at);if(d<=bestD){bestD=d;best=p;}
  }
  return best;
 }
}
