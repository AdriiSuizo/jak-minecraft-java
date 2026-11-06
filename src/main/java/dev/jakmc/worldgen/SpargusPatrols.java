package dev.jakmc.worldgen;

import dev.jakmc.JakData;
import dev.jakmc.entity.CitizenKind;
import dev.jakmc.entity.CitizenState;
import dev.jakmc.entity.JakLeaper;
import java.util.*;
import net.minecraft.core.BlockPos;
import net.minecraft.core.registries.BuiltInRegistries;
import net.minecraft.server.level.ServerLevel;
import net.minecraft.world.entity.EntitySpawnReason;
import net.minecraft.world.entity.EntityTypes;
import net.minecraft.world.entity.npc.villager.Villager;
import net.minecraft.world.level.ChunkPos;
import net.minecraft.world.level.levelgen.structure.BoundingBox;
import net.minecraft.world.level.levelgen.structure.pieces.StructurePieceSerializationContext;
import net.minecraft.world.phys.AABB;
import net.minecraft.world.phys.Vec3;

/** Rust: leapers.rs step, a Spargus town near Jak keeps its two mounted monk patrols; a monk alive on foot keeps its place. */
public final class SpargusPatrols {
 private static CityBlueprint plan;
 private static boolean monk(Villager v){return v.getType()==EntityTypes.VILLAGER&&((CitizenState)v).citizenVariant()==CitizenKind.MONK;}

 // Rust: leapers.rs step (every 2 s, the towns within 150 blocks of Jak)
 public static void tick(ServerLevel level){
  if(level.getGameTime()%40!=0)return;
  var type=BuiltInRegistries.STRUCTURE_PIECE.getValue(JakData.id("city_blocks"));
  StructurePieceSerializationContext context=null;
  var seen=new HashSet<BlockPos>();
  for(var p:level.players()){
   if(p.isSpectator())continue;
   int cx=p.blockPosition().getX()>>4,cz=p.blockPosition().getZ()>>4;
   for(int dx=-10;dx<=10;dx++)for(int dz=-10;dz<=10;dz++){
    var chunk=level.getChunkSource().getChunkNow(cx+dx,cz+dz);
    if(chunk==null)continue;
    for(var start:chunk.getAllStarts().values()){
     if(!start.isValid())continue;
     for(var piece:start.getPieces()){
      if(piece.getType()!=type)continue;
      if(context==null)context=StructurePieceSerializationContext.fromLevel(level);
      var tag=piece.createTag(context);
      if(!tag.getStringOr("City","").equals("spargus"))continue;
      var origin=new BlockPos(tag.getIntOr("X",0),tag.getIntOr("Y",0),tag.getIntOr("Z",0));
      if(Math.hypot(origin.getX()-p.getX(),origin.getZ()-p.getZ())>150||!seen.add(origin))continue;
      town(level,origin,tag.getIntOr("Turns",0)&3,start.getBoundingBox());
     }
    }
   }
  }
 }
 // Rust: leapers.rs population_at and population_needed (two patrols; a living rider, or one on foot, keeps its place)
 private static void town(ServerLevel level,BlockPos origin,int turns,BoundingBox bounds){
  for(int x=bounds.minX()>>4;x<=bounds.maxX()>>4;x++)for(int z=bounds.minZ()>>4;z<=bounds.maxZ()>>4;z++)
   if(!level.areEntitiesLoaded(ChunkPos.pack(x,z)))return;
  var box=AABB.of(bounds);
  // stand-in: Rust marks each leaper with its birth town, so a stolen animal still holds a patrol place; the town's box counts here.
  int riding=level.getEntitiesOfClass(JakLeaper.class,box,l->l.isAlive()&&l.getFirstPassenger() instanceof Villager v&&monk(v)).size();
  int onFoot=level.getEntitiesOfClass(Villager.class,box,v->v.isAlive()&&!v.isPassenger()&&monk(v)).size();
  if(plan==null)plan=CityBlueprint.read("spargus");
  for(int k=riding+onFoot;k<2&&k<plan.patrols.size();k++)spawn(level,origin,turns,plan.patrols.get(k));
 }
 // As JakCityStructure.spawnLeaper places the generated patrols.
 private static void spawn(ServerLevel level,BlockPos origin,int turns,CityBlueprint.Patrol route){
  var ground=plan.streetFoot(route.start());var destination=plan.streetFoot(route.end());
  if(ground==null||destination==null)return;
  var point=CityBlueprint.rotate(ground,turns).offset(origin);
  if(!level.isEmptyBlock(point)||!level.isEmptyBlock(point.above())
   ||level.getBlockState(point.below()).getCollisionShape(level,point.below()).isEmpty())return;
  var leaper=JakLeaper.TYPE.create(level,EntitySpawnReason.EVENT);
  var monk=EntityTypes.VILLAGER.create(level,EntitySpawnReason.EVENT);
  if(leaper==null||monk==null)return;
  var at=Vec3.atBottomCenterOf(point);
  leaper.snapTo(at.x,at.y,at.z,0,0);leaper.setPersistenceRequired();
  ((CitizenState)monk).citizenVariant(CitizenKind.MONK);
  monk.snapTo(at.x,at.y+1.6,at.z,0,0);monk.setPersistenceRequired();
  if(!monk.startRiding(leaper,true,false))return;
  leaper.patrol(point,CityBlueprint.rotate(destination,turns).offset(origin));
  level.addFreshEntityWithPassengers(leaper);
 }
}
