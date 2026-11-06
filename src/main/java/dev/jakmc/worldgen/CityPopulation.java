package dev.jakmc.worldgen;

import dev.jakmc.JakData;
import dev.jakmc.citizens.CitizenTown;
import dev.jakmc.entity.CitizenKind;
import dev.jakmc.entity.CitizenState;
import dev.jakmc.entity.JakEnemy;
import dev.jakmc.entity.JakEntities;
import dev.jakmc.entity.JakLeaper;
import java.util.*;
import net.minecraft.core.BlockPos;
import net.minecraft.core.registries.BuiltInRegistries;
import net.minecraft.resources.ResourceKey;
import net.minecraft.server.level.ServerLevel;
import net.minecraft.util.RandomSource;
import net.minecraft.world.entity.EntitySpawnReason;
import net.minecraft.world.entity.EntityTypes;
import net.minecraft.world.entity.npc.villager.Villager;
import net.minecraft.world.level.ChunkPos;
import net.minecraft.world.level.Level;
import net.minecraft.world.level.levelgen.structure.BoundingBox;
import net.minecraft.world.level.levelgen.structure.StructureStart;
import net.minecraft.world.level.levelgen.structure.pieces.StructurePieceSerializationContext;
import net.minecraft.world.phys.AABB;
import net.minecraft.world.phys.Vec3;

/**
 * Rust: jak2_villagers.rs step, peopling a town that comes within PEOPLED of Jak (emptied past EMPTIED):
 * Spargus 6 residents, the barrio (its EXPANDED_MARK) 28 + 17 x random, at most its beds, on its verified
 * streets; and leapers.rs population_needed, Spargus's two loose leapers.
 * stand-in: Rust removes an emptied town's people and peoples it afresh; these residents are native
 * entities that stay, so coming back only tops the town up to its number.
 */
public final class CityPopulation {
 static final double PEOPLED=150,EMPTIED=220;
 private static final Map<ResourceKey<Level>,Set<BlockPos>> PEOPLED_TOWNS=new HashMap<>();
 /** The peopled Havens, for their garrison. */
 private record Town(String city,BlockPos origin,int turns,BoundingBox bounds){}
 private static final Map<ResourceKey<Level>,Map<BlockPos,Town>> TOWNS=new HashMap<>();
 /** stand-in (Adrii 06/11): a guard to each beat, up to this many; Rust kept 8 (jak_entities.rs). */
 static final int GARRISON=48;
 /** Null while the plan is still being read (off this thread); the town is tried again later. */
 private static CityBlueprint plan(String city){return CityBlueprint.ready(city);}
 private CityPopulation(){}

 /** Rust: jak2_villagers.rs population (once per town: 7 to 10, an expanded town's 28 to 44; Spargus 6). */
 static int wanted(String city,BlockPos origin,long seed){
  if(city.equals("spargus"))return 6;
  // stand-in: Rust draws it once from its running random; here from the world seed and the town, so it stays.
  var random=RandomSource.create(seed^origin.asLong()*0x9E3779B97F4A7C15L);
  return 28+(int)(random.nextFloat()*17);
 }
 public static boolean resident(Villager v){
  if(v.getType()!=EntityTypes.VILLAGER||!v.isAlive()||v.isPassenger())return false;
  int variant=((CitizenState)v).citizenVariant();
  return variant<CitizenKind.HAVEN_VARIANTS||variant==CitizenKind.SPARGUS_MALE||variant==CitizenKind.SPARGUS_FEMALE;
 }

 public static void tick(ServerLevel level){
  if(level.getGameTime()%40!=20)return;
  var type=BuiltInRegistries.STRUCTURE_PIECE.getValue(JakData.id("city_blocks"));
  var peopled=PEOPLED_TOWNS.computeIfAbsent(level.dimension(),k->new HashSet<>());
  // Rust: far towns emptied (every player past EMPTIED).
  peopled.removeIf(t->level.players().stream().noneMatch(p->!p.isSpectator()&&Math.hypot(t.getX()-p.getX(),t.getZ()-p.getZ())<=EMPTIED));
  var towns=TOWNS.computeIfAbsent(level.dimension(),k->new HashMap<>());
  towns.keySet().retainAll(peopled);
  // Rust: jak_entities.rs, a guard every 8 s while the barrio is short of them. stand-in (Adrii 06/11): by day only.
  if(level.getGameTime()%160==20)for(var town:towns.values()){
   if(loaded(level,town.bounds))beds(level,AABB.of(town.bounds).inflate(0,24,0));
   garrison(level,town);
  }
  StructurePieceSerializationContext context=null;
  for(var p:level.players()){
   if(p.isSpectator())continue;
   int cx=p.blockPosition().getX()>>4,cz=p.blockPosition().getZ()>>4,reach=(int)Math.ceil(PEOPLED/16);
   for(int dx=-reach;dx<=reach;dx++)for(int dz=-reach;dz<=reach;dz++){
    var chunk=level.getChunkSource().getChunkNow(cx+dx,cz+dz);
    if(chunk==null)continue;
    for(var start:chunk.getAllStarts().values()){
     if(!start.isValid())continue;
     for(var piece:start.getPieces()){
      if(piece.getType()!=type)continue;
      if(context==null)context=StructurePieceSerializationContext.fromLevel(level);
      var tag=piece.createTag(context);
      String city=tag.getStringOr("City","");
      if(!city.equals("haven")&&!city.equals("spargus"))continue;
      var origin=new BlockPos(tag.getIntOr("X",0),tag.getIntOr("Y",0),tag.getIntOr("Z",0));
      if(peopled.contains(origin)||Math.hypot(origin.getX()-p.getX(),origin.getZ()-p.getZ())>=PEOPLED)continue;
      int turns=tag.getIntOr("Turns",0)&3;
      if(people(level,city,origin,turns,start)){
       peopled.add(origin);
       if(city.equals("haven"))TOWNS.computeIfAbsent(level.dimension(),k->new HashMap<>()).put(origin,new Town(city,origin,turns,start.getBoundingBox()));
      }
     }
    }
   }
  }
 }

 private static boolean loaded(ServerLevel level,BoundingBox bounds){
  for(int x=bounds.minX()>>4;x<=bounds.maxX()>>4;x++)for(int z=bounds.minZ()>>4;z<=bounds.maxZ()>>4;z++)
   if(!level.areEntitiesLoaded(ChunkPos.pack(x,z)))return false;
  return true;
 }

 private static boolean people(ServerLevel level,String city,BlockPos origin,int turns,StructureStart start){
  var bounds=start.getBoundingBox();
  if(!loaded(level,bounds))return false;
  var plan=plan(city);
  if(plan==null)return false;
  var box=AABB.of(bounds);
  boolean spargus=city.equals("spargus");
  // Rust: walking.min(housing::capacity), a resident to a bed.
  int wanted=Math.min(wanted(city,origin,level.getSeed()),plan.beds());
  int living=level.getEntitiesOfClass(Villager.class,box,CityPopulation::resident).size();
  if(living<wanted){
   var sites=sites(level,plan,origin,turns,start,wanted-living);
   var random=level.getRandom();
   for(var at:sites){
    var v=EntityTypes.VILLAGER.create(level,EntitySpawnReason.EVENT);
    if(v==null)break;
    // Rust: Spargus's wastelanders (65 % men), elsewhere the three Haven citizens.
    int variant=spargus?random.nextFloat()<.65f?CitizenKind.SPARGUS_MALE:CitizenKind.SPARGUS_FEMALE:random.nextInt(CitizenKind.HAVEN_VARIANTS);
    ((CitizenState)v).citizenVariant(variant);
    v.snapTo(at.x,at.y,at.z,random.nextFloat()*360,0);
    v.setPersistenceRequired();
    level.addFreshEntity(v);
   }
  }
  if(spargus)loose(level,plan,origin,turns,box);
  beds(level,box.inflate(0,24,0));
  return true;
 }

 /**
  * Rust: settlement_housing, a bed to each resident. stand-in (Adrii 06/11, "cada ciudadano una cama"):
  * the resident is handed a free bed of its own town at once (vanilla's HOME, the bed's POI taken) instead
  * of waiting for vanilla to find one; at night vanilla's rest takes it there, its doors shut behind it.
  * The Krimson station's beds (on its floor) are the guards' barracks, left out.
  */
 private static void beds(ServerLevel level,AABB box){
  int radius=(int)Math.ceil(Math.hypot(box.getXsize(),box.getZsize())/2)+2;
  for(var v:level.getEntitiesOfClass(Villager.class,box,CityPopulation::resident)){
   if(v.getBrain().hasMemoryValue(net.minecraft.world.entity.ai.memory.MemoryModuleType.HOME))continue;
   level.getPoiManager().take(h->h.is(net.minecraft.world.entity.ai.village.poi.PoiTypes.HOME),
     (h,pos)->box.contains(Vec3.atCenterOf(pos))&&!barracks(level,pos),v.blockPosition(),radius)
    .ifPresent(pos->v.getBrain().setMemory(net.minecraft.world.entity.ai.memory.MemoryModuleType.HOME,net.minecraft.core.GlobalPos.of(level.dimension(),pos)));
  }
 }
 private static boolean barracks(ServerLevel level,BlockPos bed){
  return BuiltInRegistries.BLOCK.getKey(level.getBlockState(bed.below()).getBlock()).getPath().equals("krimson_suelo");
 }

 /**
  * Rust: jak_entities.rs (the Haven guards topped up, one at a time, at a site no guard is near).
  * stand-in (Adrii 06/11): only by day, they come out of the Krimson station (its guard posts) and
  * take a beat no guard walks; no bed, the station is their home.
  */
 private static void garrison(ServerLevel level,Town town){
  if(!level.isBrightOutside()||!loaded(level,town.bounds))return;
  var plan=plan(town.city);
  if(plan==null)return;
  var box=AABB.of(town.bounds).inflate(0,24,0);
  var guards=level.getEntitiesOfClass(JakEnemy.class,box,e->e.isAlive()&&e.kind.guard());
  if(guards.size()>=Math.min(GARRISON,plan.patrols.size()))return;
  BlockPos from=null,to=null;
  for(var beat:plan.patrols){
   var a=CityBlueprint.rotate(beat.start(),town.turns).offset(town.origin);
   if(guards.stream().anyMatch(g->Math.hypot(g.getX()-a.getX(),g.getZ()-a.getZ())<12))continue;
   from=a;to=CityBlueprint.rotate(beat.end(),town.turns).offset(town.origin);break;
  }
  if(from==null)return;
  Vec3 at=null;
  var posts=new ArrayList<>(plan.posts);Collections.shuffle(posts);
  for(var post:posts){
   at=CitizenTown.standing(level,Vec3.atBottomCenterOf(CityBlueprint.rotate(post,town.turns).offset(town.origin).above()));
   if(at!=null)break;
  }
  if(at==null)at=CitizenTown.standing(level,Vec3.atBottomCenterOf(from));
  if(at==null)return;
  var guard=JakEntities.TYPES.get("crimson-guard").create(level,EntitySpawnReason.EVENT);
  if(guard==null)return;
  guard.patrol(from,to);
  guard.snapTo(at.x,at.y,at.z,level.getRandom().nextFloat()*360,0);
  guard.setPersistenceRequired();
  level.addFreshEntity(guard);
 }

 /** Rust: settlement_nav spawn_sites, spread over the town's streets: the farthest from those taken, each standing clear. */
 private static List<Vec3> sites(ServerLevel level,CityBlueprint plan,BlockPos origin,int turns,StructureStart start,int count){
  var candidates=new ArrayList<Vec3>();
  // Jak II's civilians keep to their own ways, never the traffic's: none on the ring road.
  var streets=JakCityStructure.streets(start);
  var ring=dev.jakmc.entity.ZoomerTraffic.ringStreets(streets);
  var road=new ArrayList<Vec3>();for(var street:ring)road.addAll(street.points());
  for(var street:streets)if(!ring.contains(street))for(var c:street.points())
   if(road.stream().noneMatch(r->Math.hypot(r.x-c.x,r.z-c.z)<4))candidates.add(c);
  // stand-in: a town without the plan's street markers (Spargus) takes its open floor, every 4 blocks.
  if(candidates.isEmpty())for(int x=plan.minX;x<=plan.maxX;x+=4)for(int z=plan.minZ;z<=plan.maxZ;z+=4){
   var foot=plan.streetFoot(new BlockPos(x,0,z));
   if(foot!=null)candidates.add(Vec3.atBottomCenterOf(CityBlueprint.rotate(foot,turns).offset(origin)));
  }
  var standing=new ArrayList<Vec3>();
  for(var c:candidates){var feet=CitizenTown.standing(level,c);if(feet!=null)standing.add(feet);}
  var out=new ArrayList<Vec3>();
  if(standing.isEmpty())return out;
  var random=RandomSource.create(origin.asLong());
  out.add(standing.get(random.nextInt(standing.size())));
  while(out.size()<count&&out.size()<standing.size()){
   Vec3 best=null;double far=-1;
   for(var c:standing){double near=Double.MAX_VALUE;for(var o:out)near=Math.min(near,c.distanceToSqr(o));if(near>far){far=near;best=c;}}
   if(best==null||far<1)break;
   out.add(best);
  }
  return out;
 }

 /** Rust: leapers.rs population_needed, two loose leapers a Spargus town keeps (where JakCityStructure first put them). */
 private static void loose(ServerLevel level,CityBlueprint plan,BlockPos origin,int turns,AABB box){
  int loose=level.getEntitiesOfClass(JakLeaper.class,box,l->l.isAlive()&&!l.patrolling()).size();
  BlockPos[] spots={new BlockPos(-12,0,-16),new BlockPos(12,0,-16)};
  for(int k=loose;k<2;k++){
   var ground=plan.streetFoot(spots[k]);
   if(ground==null)continue;
   var point=CityBlueprint.rotate(ground,turns).offset(origin);
   if(!level.isEmptyBlock(point)||!level.isEmptyBlock(point.above())
    ||level.getBlockState(point.below()).getCollisionShape(level,point.below()).isEmpty())continue;
   var leaper=JakLeaper.TYPE.create(level,EntitySpawnReason.EVENT);
   if(leaper==null)return;
   var at=Vec3.atBottomCenterOf(point);
   leaper.snapTo(at.x,at.y,at.z,0,0);leaper.setPersistenceRequired();
   level.addFreshEntity(leaper);
  }
 }

 /** For the scenes: the towns peopled now. */
 public static Set<BlockPos> peopled(ServerLevel level){return Set.copyOf(PEOPLED_TOWNS.getOrDefault(level.dimension(),Set.of()));}
 public static void clear(){PEOPLED_TOWNS.clear();TOWNS.clear();}
}
