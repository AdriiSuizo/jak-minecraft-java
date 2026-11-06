package dev.jakmc.citizens;

import dev.jakmc.content.JakBlocks;
import java.util.*;
import net.minecraft.core.BlockPos;
import net.minecraft.core.Direction;
import net.minecraft.core.GlobalPos;
import net.minecraft.server.level.ServerLevel;
import net.minecraft.world.entity.ai.memory.MemoryModuleType;
import net.minecraft.world.entity.npc.villager.Villager;
import net.minecraft.world.level.block.DoorBlock;
import net.minecraft.world.level.block.state.BlockState;
import net.minecraft.world.level.block.state.properties.BedPart;
import net.minecraft.world.level.block.state.properties.BlockStateProperties;
import net.minecraft.world.level.block.state.properties.DoorHingeSide;
import net.minecraft.world.level.block.state.properties.DoubleBlockHalf;
import net.minecraft.world.phys.Vec3;

/** Rust: settlement_housing.rs (a bed's entrance) and jak2_villagers.rs home_visit, home_inside, bedside, for the native HOME bed. */
final class CitizenHomes {
 // Rust: jak2_villagers.rs marks_in (the lower, left-hinged leaf of the tall Haven and Spargus doors)
 static boolean entrance(BlockState state){
  if(!(state.getBlock() instanceof DoorBlock))return false;
  var b=state.getBlock();
  return (b==JakBlocks.BLOCKS.get("haven_puerta_alta")||b==JakBlocks.BLOCKS.get("spargus_puerta_alta"))
   &&state.getValue(DoorBlock.HALF)==DoubleBlockHalf.LOWER&&state.getValue(DoorBlock.HINGE)==DoorHingeSide.LEFT;
 }
 // stand-in: Rust's settlement_housing claims a bed per resident; the native brain's HOME bed is that claim here.
 static BlockPos bedHead(ServerLevel level,Villager v){
  var home=v.getBrain().getMemory(MemoryModuleType.HOME).filter(g->g.dimension()==level.dimension()).map(GlobalPos::pos).orElse(null);
  if(home==null)return null;
  var state=level.getBlockState(home);
  if(!state.hasProperty(BlockStateProperties.BED_PART))return null;
  return state.getValue(BlockStateProperties.BED_PART)==BedPart.HEAD?home:home.relative(state.getValue(BlockStateProperties.HORIZONTAL_FACING));
 }
 static CitizenLife.Door door(ServerLevel level,CitizenLife life){
  var bed=bedHead(level,life.body);
  if(bed==null){life.home=null;life.homeBed=null;return null;}
  long now=level.getGameTime();
  // stand-in: Rust keeps the claimed entrance for good; it is looked up again every 30 s here.
  if(bed.equals(life.homeBed)&&now-life.homeChecked<600)return life.home;
  life.homeBed=bed;life.homeChecked=now;life.home=find(level,bed);
  return life.home;
 }
 // Rust: settlement_housing.rs claim_checked (doors a block up or down within 13, the bed on their inside; nearest first, the first with a visit and a bedside)
 private static CitizenLife.Door find(ServerLevel level,BlockPos bed){
  var way=level.getBlockState(bed).getValue(BlockStateProperties.HORIZONTAL_FACING);
  var doors=new ArrayList<BlockPos>();var at=new BlockPos.MutableBlockPos();
  for(int dy=-1;dy<=1;dy++)for(int dx=-13;dx<=13;dx++)for(int dz=-13;dz<=13;dz++){
   var state=level.getBlockState(at.set(bed.getX()+dx,bed.getY()+dy,bed.getZ()+dz));
   if(!entrance(state))continue;
   var out=state.getValue(DoorBlock.FACING);
   if(-dx*out.getStepX()-dz*out.getStepZ()<0)doors.add(at.immutable());
  }
  doors.sort(Comparator.comparingLong(d->(long)(d.getX()-bed.getX())*(d.getX()-bed.getX())+(long)(d.getZ()-bed.getZ())*(d.getZ()-bed.getZ())));
  for(var d:doors){var visit=visit(level,d,bed,way);if(visit!=null&&visit.bedside()!=null)return visit;}
  return null;
 }
 // Rust: jak2_villagers.rs home_visit and home_inside
 private static CitizenLife.Door visit(ServerLevel level,BlockPos at,BlockPos bed,Direction way){
  var out=level.getBlockState(at).getValue(DoorBlock.FACING);int ox=out.getStepX(),oz=out.getStepZ();
  var outside=standing(level,point(at,ox,oz,2));if(outside==null)return null;
  Vec3 inside=null;
  for(double n:new double[]{1.5,2,2.5})if((inside=standing(level,point(at,ox,oz,-n)))!=null)break;
  if(inside==null)return null;
  // Tall double doors retain two open leaves, each 1/16 block thick.
  return new CitizenLife.Door(at,ox,oz,point(at,ox,oz,0),1-1/16.,outside,inside,bedside(level,bed,way,inside));
 }
 // Rust: settlement_housing.rs Door::point (the two leaves' middle, half a block to the right, `outward` blocks out)
 static Vec3 point(BlockPos at,int ox,int oz,double outward){
  return new Vec3(at.getX()+.5-oz*.5+ox*outward,at.getY(),at.getZ()+.5+ox*.5+oz*outward);
 }
 // Rust: settlement_nav.rs standing (five thin probes, a level footprint, the walker's body clear and dry)
 static Vec3 standing(ServerLevel level,Vec3 at){
  double r=.69,lo=Double.POSITIVE_INFINITY,hi=Double.NEGATIVE_INFINITY,first=0;int i=0;
  for(double[] o:new double[][]{{0,0},{-r,-r},{-r,r},{r,-r},{r,r}}){
   double z=CitizenTown.ground(level,at.add(o[0],1.25,o[1]),2.75,.04);
   if(Double.isNaN(z))return null;
   if(i++==0)first=z;lo=Math.min(lo,z);hi=Math.max(hi,z);
  }
  if(hi-lo>.55||Math.abs(first-at.y)>1.1)return null;
  var feet=new Vec3(at.x,hi,at.z);
  if(!CitizenTown.clear(level,feet.add(0,.02,0),.78,2.5)||CitizenTown.wet(level,feet.add(0,.15,0)))return null;
  return feet;
 }
 // Rust: jak2_villagers.rs bedside (beside the bed, else the nearest standing quarter-block spot of the room)
 private static Vec3 bedside(ServerLevel level,BlockPos head,Direction way,Vec3 entry){
  int wx=way.getStepX(),wz=way.getStepZ();
  Vec3 floor=new Vec3(head.getX()+.5-wx*.5,head.getY(),head.getZ()+.5-wz*.5),h=new Vec3(wx,0,wz),side=new Vec3(wz,0,-wx);
  // stand-in: Rust also needs a Nav route from the entry; the native path finds it when walked.
  Vec3[] offsets={side.scale(1.3),side.scale(-1.3),side.scale(1.3).add(h.scale(.9)),side.scale(-1.3).add(h.scale(.9)),
   side.scale(1.3).add(h.scale(-.9)),side.scale(-1.3).add(h.scale(-.9)),h.scale(-1.85),h.scale(1.85),
   h.scale(-1.85).add(side.scale(.5)),h.scale(-1.85).add(side.scale(-.5)),h.scale(1.85).add(side.scale(.5)),h.scale(1.85).add(side.scale(-.5))};
  for(var d:offsets){var p=standing(level,floor.add(d));if(p!=null)return p;}
  var around=new ArrayList<int[]>();
  for(int x=-8;x<=8;x++)for(int y=-8;y<=8;y++)if(x*x+y*y<=81)around.add(new int[]{x,y});
  around.sort(Comparator.comparingInt(p->p[0]*p[0]+p[1]*p[1]));
  for(var p:around){var at=standing(level,floor.add(side.scale(p[0]*.25)).add(h.scale(p[1]*.25)));if(at!=null)return at;}
  return null;
 }
 // Rust: jak2_villagers.rs step, already_inside (behind the threshold, near the inner landing)
 static boolean indoors(Vec3 at,CitizenLife.Door d){
  // stand-in: without Rust's Nav route test from here to the inner landing.
  return (at.x-d.threshold().x)*d.outX()+(at.z-d.threshold().z)*d.outZ()<-.75&&at.distanceTo(d.inside())<12;
 }
 // Rust: jak2_villagers.rs step, the stroll's middle (the bed's head, else the village bell)
 static Vec3 center(ServerLevel level,Villager v){
  var bed=bedHead(level,v);
  if(bed!=null)return Vec3.atBottomCenterOf(bed);
  var meet=v.getBrain().getMemory(MemoryModuleType.MEETING_POINT).filter(g->g.dimension()==level.dimension()).map(GlobalPos::pos).orElse(null);
  return meet!=null?Vec3.atBottomCenterOf(meet):v.position();
 }
}
