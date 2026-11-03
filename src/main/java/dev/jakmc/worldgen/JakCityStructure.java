package dev.jakmc.worldgen;

import com.mojang.serialization.Codec;
import com.mojang.serialization.MapCodec;
import com.mojang.serialization.codecs.RecordCodecBuilder;
import dev.jakmc.JakData;
import dev.jakmc.entity.JakVehicle;
import dev.jakmc.entity.JakEntities;
import dev.jakmc.entity.CitizenKind;
import dev.jakmc.entity.CitizenState;
import dev.jakmc.entity.JakLeaper;
import java.util.Arrays;
import java.util.Optional;
import net.minecraft.core.BlockPos;
import net.minecraft.core.QuartPos;
import net.minecraft.core.Registry;
import net.minecraft.core.registries.BuiltInRegistries;
import net.minecraft.nbt.CompoundTag;
import net.minecraft.util.RandomSource;
import net.minecraft.world.entity.EntitySpawnReason;
import net.minecraft.world.entity.EntityTypes;
import net.minecraft.world.level.block.state.properties.BedPart;
import net.minecraft.world.level.block.state.properties.BlockStateProperties;
import net.minecraft.world.level.ChunkPos;
import net.minecraft.world.level.StructureManager;
import net.minecraft.world.level.WorldGenLevel;
import net.minecraft.world.level.block.Block;
import net.minecraft.world.level.block.Blocks;
import net.minecraft.world.level.chunk.ChunkGenerator;
import net.minecraft.world.level.levelgen.Heightmap;
import net.minecraft.world.level.levelgen.structure.BoundingBox;
import net.minecraft.world.level.levelgen.structure.Structure;
import net.minecraft.world.level.levelgen.structure.StructurePiece;
import net.minecraft.world.level.levelgen.structure.StructureType;
import net.minecraft.world.level.levelgen.structure.pieces.StructurePieceSerializationContext;
import net.minecraft.world.level.levelgen.structure.pieces.StructurePieceType;
import net.minecraft.world.phys.Vec3;

/** Native discoverable city starts with chunk-local placement and seed-stable orientation. */
public final class JakCityStructure extends Structure {
    private enum Site {
        HAVEN("haven"), SPARGUS("spargus");
        final String id;
        private CityBlueprint blueprint;
        Site(String id){this.id=id;}
        synchronized CityBlueprint plan(){
            if(blueprint==null)blueprint=CityBlueprint.read(id);
            return blueprint;
        }
        static Site of(String name){
            for(var site:values())if(site.id.equals(name))return site;
            throw new IllegalArgumentException("Unknown Jak city: "+name);
        }
    }

    private static final MapCodec<JakCityStructure> CODEC=RecordCodecBuilder.mapCodec(instance->instance.group(
        settingsCodec(instance),Codec.STRING.fieldOf("city").forGetter(value->value.site.id)
    ).apply(instance,(settings,city)->new JakCityStructure(settings,Site.of(city))));
    private static StructureType<JakCityStructure> type;
    private static StructurePieceType pieceType;
    private final Site site;

    private JakCityStructure(StructureSettings settings,Site site){super(settings);this.site=site;}

    public static void initialize(){
        type=Registry.register(BuiltInRegistries.STRUCTURE_TYPE,JakData.id("city"),()->CODEC);
        pieceType=Registry.register(BuiltInRegistries.STRUCTURE_PIECE,JakData.id("city_blocks"),
            (StructurePieceType.ContextlessType)CityPiece::new);
    }
    @Override public StructureType<?> type(){return type;}

    @Override protected Optional<GenerationStub> findGenerationPoint(GenerationContext context){
        int x=context.chunkPos().getMinBlockX(),z=context.chunkPos().getMinBlockZ();
        int y=height(context,x,z);
        int sea=context.chunkGenerator().getSeaLevel();
        if(y<=sea||!valid(context,x,y,z))return Optional.empty();
        int[] elevations=new int[49];
        int index=0;
        int submerged=0;
        for(int dz=-84;dz<=84;dz+=28)for(int dx=-84;dx<=84;dx+=28){
            int at=height(context,x+dx,z+dz);
            elevations[index++]=at;
            if(at<=sea)submerged++;
        }
        if(submerged>4)return Optional.empty();
        Arrays.sort(elevations);
        int median=elevations[24];
        if(elevations[43]-elevations[5]>26||Math.abs(y-median)>15)return Optional.empty();
        int base=Math.max(sea+3,median);
        if(site==Site.SPARGUS){
            for(int dx=-56;dx<=56;dx+=56)for(int dz=-56;dz<=56;dz+=56)
                if(!valid(context,x+dx,base,z+dz))return Optional.empty();
        }
        var plan=site.plan();
        if(base+plan.maxY>=context.heightAccessor().getMaxY()-4)return Optional.empty();
        var origin=new BlockPos(x,base,z);
        int turns=context.random().nextInt(4);
        return Optional.of(new GenerationStub(origin,builder->builder.addPiece(new CityPiece(site,origin,turns))));
    }

    private static boolean valid(GenerationContext context,int x,int y,int z){
        var biome=context.biomeResolver().getNoiseBiome(QuartPos.fromBlock(x),QuartPos.fromBlock(y),QuartPos.fromBlock(z));
        return context.validBiome().test(biome);
    }
    private static int height(GenerationContext context,int x,int z){
        return context.chunkGenerator().getBaseHeight(x,z,Heightmap.Types.OCEAN_FLOOR_WG,
            context.heightAccessor(),context.randomState());
    }

    private static final class CityPiece extends StructurePiece {
        private final BlockPos origin;
        private final Site site;
        private final int turns;

        CityPiece(Site site,BlockPos origin,int turns){
            super(pieceType,0,bounds(site.plan(),origin,turns));
            this.origin=origin;
            this.site=site;
            this.turns=turns;
        }
        CityPiece(CompoundTag data){
            super(pieceType,data);
            site=Site.of(data.getStringOr("City","haven"));
            origin=new BlockPos(data.getIntOr("X",0),data.getIntOr("Y",0),data.getIntOr("Z",0));
            turns=data.getIntOr("Turns",0)&3;
        }

        private static BoundingBox bounds(CityBlueprint plan,BlockPos origin,int turns){
            var low=CityBlueprint.rotate(new BlockPos(plan.minX,plan.minY-32,plan.minZ),turns).offset(origin);
            var high=CityBlueprint.rotate(new BlockPos(plan.maxX,plan.maxY+4,plan.maxZ),turns).offset(origin);
            return BoundingBox.fromCorners(low,high);
        }

        @Override protected void addAdditionalSaveData(StructurePieceSerializationContext context,CompoundTag data){
            data.putString("City",site.id);
            data.putInt("X",origin.getX());
            data.putInt("Y",origin.getY());
            data.putInt("Z",origin.getZ());
            data.putInt("Turns",turns);
        }

        @Override public void postProcess(WorldGenLevel level,StructureManager manager,ChunkGenerator generator,
            RandomSource random,BoundingBox chunkBounds,ChunkPos chunk,BlockPos reference){
            var plan=site.plan();
            var cells=plan.cells(chunk.x()-(origin.getX()>>4),chunk.z()-(origin.getZ()>>4),turns);
            if(cells.size()==0)return;
            var foundations=new int[256];
            Arrays.fill(foundations,Integer.MAX_VALUE);
            var fill=site==Site.SPARGUS?Blocks.SANDSTONE.defaultBlockState():Blocks.STONE.defaultBlockState();
            for(int i=0;i<cells.size();i++){
                var local=cells.pos(i);
                if(cells.states()[i].isAir()||local.getY()>3)continue;
                var pos=CityBlueprint.rotate(local,turns).offset(origin);
                int column=(pos.getX()&15)|((pos.getZ()&15)<<4);
                foundations[column]=Math.min(foundations[column],pos.getY());
            }
            var at=new BlockPos.MutableBlockPos();
            var air=Blocks.AIR.defaultBlockState();
            for(int index=0;index<foundations.length;index++){
                int top=foundations[index];
                if(top==Integer.MAX_VALUE)continue;
                int x=chunk.getMinBlockX()+(index&15),z=chunk.getMinBlockZ()+(index>>4);
                int natural=level.getHeight(Heightmap.Types.WORLD_SURFACE_WG,x,z);
                // Existing cliff overhangs and canopy above the planned clearance
                // have no support once the district cuts their lower terrain.
                for(int y=natural-1;y>=top;y--){
                    if(!chunkBounds.isInside(at.set(x,y,z))||level.getBlockState(at).isAir())continue;
                    level.setBlock(at,air,Block.UPDATE_KNOWN_SHAPE);
                }
            }
            var rotated=new java.util.IdentityHashMap<net.minecraft.world.level.block.state.BlockState,net.minecraft.world.level.block.state.BlockState>();
            for(int i=0;i<cells.size();i++){
                var pos=CityBlueprint.rotate(cells.pos(i),turns).offset(origin);
                if(!chunkBounds.isInside(pos))continue;
                var state=rotated.computeIfAbsent(cells.states()[i],base->CityBlueprint.rotate(base,turns));
                if(level.getBlockState(pos)!=state)
                    level.setBlock(pos,state,Block.UPDATE_CLIENTS|Block.UPDATE_KNOWN_SHAPE);
            }
            for(int index=0;index<foundations.length;index++){
                int top=foundations[index];
                if(top==Integer.MAX_VALUE)continue;
                int x=chunk.getMinBlockX()+(index&15),z=chunk.getMinBlockZ()+(index>>4);
                for(int y=top-1;y>=Math.max(top-28,level.getMinY());y--){
                    at.set(x,y,z);
                    var current=level.getBlockState(at);
                    if(current.isSolidRender())break;
                    level.setBlock(at,fill,Block.UPDATE_KNOWN_SHAPE);
                }
            }
            spawnResident(level,chunk,random,cells);
            if(site==Site.HAVEN){
                for(var patrol:plan.patrols)spawnGuard(level,chunkBounds,patrol);
            }else{
                for(var patrol:plan.patrols)spawnLeaper(level,chunkBounds,patrol.start(),patrol.end());
                spawnLeaper(level,chunkBounds,new BlockPos(-12,0,-16),null);
                spawnLeaper(level,chunkBounds,new BlockPos(12,0,-16),null);
            }
            for(var garage:plan.garages){
                var pos=CityBlueprint.rotate(garage.pos(),turns).offset(origin);
                if(!chunkBounds.isInside(pos))continue;
                var model=garage.duneHopper()?JakVehicle.Model.TOAD:JakVehicle.Model.SNAKE;
                var car=model.type.create(level.getLevel(),EntitySpawnReason.STRUCTURE);
                if(car==null)continue;
                var spot=Vec3.atBottomCenterOf(pos.above()).add(0,1.1,0);
                car.snapTo(spot.x,spot.y,spot.z,CityBlueprint.rotate(garage.facing(),turns).toYRot(),0);
                level.addFreshEntity(car);
            }
        }

        private void spawnLeaper(WorldGenLevel level,BoundingBox chunk,BlockPos start,BlockPos end){
            var ground=site.plan().streetFoot(start);
            var destination=end==null?null:site.plan().streetFoot(end);
            if(ground==null||end!=null&&destination==null)return;
            var point=CityBlueprint.rotate(ground,turns).offset(origin);
            if(!chunk.isInside(point)||!level.isEmptyBlock(point)||!level.isEmptyBlock(point.above())
                ||level.getBlockState(point.below()).getCollisionShape(level,point.below()).isEmpty())return;
            var leaper=JakLeaper.TYPE.create(level.getLevel(),EntitySpawnReason.STRUCTURE);
            if(leaper==null)return;
            var at=Vec3.atBottomCenterOf(point);
            leaper.snapTo(at.x,at.y,at.z,0,0);
            leaper.setPersistenceRequired();
            if(end!=null){
                var monk=EntityTypes.VILLAGER.create(level.getLevel(),EntitySpawnReason.STRUCTURE);
                if(monk==null)return;
                ((CitizenState)monk).citizenVariant(CitizenKind.MONK);
                monk.snapTo(at.x,at.y+1.6,at.z,0,0);
                monk.setPersistenceRequired();
                if(!monk.startRiding(leaper,true,false))return;
                leaper.patrol(point,CityBlueprint.rotate(destination,turns).offset(origin));
                // Worldgen serializes the mount with its passenger as one NBT tree.
            }
            level.addFreshEntity(leaper);
        }

        private void spawnGuard(WorldGenLevel level,BoundingBox chunk,CityBlueprint.Patrol route){
            var point=CityBlueprint.rotate(route.start(),turns).offset(origin);
            if(!chunk.isInside(point)||!level.isEmptyBlock(point)||!level.isEmptyBlock(point.above()))return;
            var destination=CityBlueprint.rotate(route.end(),turns).offset(origin);
            var guard=JakEntities.TYPES.get("crimson-guard").create(level.getLevel(),EntitySpawnReason.STRUCTURE);
            if(guard==null)return;
            guard.patrol(point,destination);
            var at=Vec3.atBottomCenterOf(point);
            guard.snapTo(at.x,at.y,at.z,CityBlueprint.rotate(route.facing(),turns).toYRot(),0);
            guard.setPersistenceRequired();
            level.addFreshEntity(guard);
        }

        private void spawnResident(WorldGenLevel level,ChunkPos chunk,RandomSource random,CityBlueprint.Fragment cells){
            if(random.nextInt(4)==0)return;
            for(int i=0;i<cells.size();i++){
                var state=cells.states()[i];
                if(!state.hasProperty(BlockStateProperties.BED_PART)||state.getValue(BlockStateProperties.BED_PART)!=BedPart.FOOT)continue;
                var bed=CityBlueprint.rotate(cells.pos(i),turns).offset(origin);
                var facing=CityBlueprint.rotate(state.getValue(BlockStateProperties.HORIZONTAL_FACING),turns);
                for(var side:new net.minecraft.core.Direction[]{facing.getClockWise(),facing.getCounterClockWise(),facing.getOpposite()}){
                    var point=bed.relative(side);
                    if((point.getX()>>4)!=chunk.x()||(point.getZ()>>4)!=chunk.z()
                        ||!level.isEmptyBlock(point)||!level.isEmptyBlock(point.above())
                        ||!level.getBlockState(point.below()).isSolidRender())continue;
                    var resident=EntityTypes.VILLAGER.create(level.getLevel(),EntitySpawnReason.STRUCTURE);
                    if(resident==null)return;
                    int variant=site==Site.HAVEN?random.nextInt(CitizenKind.HAVEN_VARIANTS)
                        :random.nextFloat()<.65f?CitizenKind.SPARGUS_MALE:CitizenKind.SPARGUS_FEMALE;
                    ((CitizenState)resident).citizenVariant(variant);
                    var at=Vec3.atBottomCenterOf(point);
                    resident.snapTo(at.x,at.y,at.z,side.toYRot(),0);
                    resident.setPersistenceRequired();
                    level.addFreshEntity(resident);
                    return;
                }
            }
        }
    }
}
