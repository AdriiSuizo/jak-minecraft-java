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
import net.minecraft.core.registries.Registries;
import net.minecraft.resources.ResourceKey;
import net.minecraft.world.level.storage.loot.LootTable;
import net.minecraft.world.level.block.entity.ChestBlockEntity;
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
    private record DecorationWork(CityPiece city,ChunkPos chunk){}
    private static final java.util.Map<net.minecraft.server.level.ServerLevel,java.util.LinkedHashMap<Long,DecorationWork>> DECORATING=new java.util.WeakHashMap<>();

    static void queueDecorations(net.minecraft.server.level.ServerLevel level,net.minecraft.world.level.levelgen.structure.StructureStart start){
        for(var piece:start.getPieces())if(piece instanceof CityPiece city){
            var plan=CityBlueprint.ready(city.site.id);
            if(plan==null||plan.decorations.isEmpty())continue;
            var queue=DECORATING.computeIfAbsent(level,key->new java.util.LinkedHashMap<>());
            for(long key:plan.decorations.keySet()){
                var local=CityBlueprint.rotate(new BlockPos((int)(key>>32),0,(int)key),city.turns);
                var chunk=new ChunkPos((city.origin.getX()>>4)+local.getX(),(city.origin.getZ()>>4)+local.getZ());
                var loaded=level.getChunkSource().getChunkNow(chunk.x(),chunk.z());
                if(loaded!=null&&!Boolean.TRUE.equals(loaded.getAttached(JakData.CITY_DECORATED)))
                    queue.putIfAbsent(ChunkPos.pack(chunk.x(),chunk.z()),new DecorationWork(city,chunk));
            }
        }
    }

    static void tickDecorations(net.minecraft.server.level.ServerLevel level){
        var queue=DECORATING.get(level);
        if(queue==null)return;
        var next=queue.pollFirstEntry();
        if(next==null){DECORATING.remove(level);return;}
        var work=next.getValue();
        var chunk=level.getChunkSource().getChunkNow(work.chunk.x(),work.chunk.z());
        if(chunk==null||Boolean.TRUE.equals(chunk.getAttached(JakData.CITY_DECORATED)))return;
        work.city.decorate(level,work.chunk);
        chunk.setAttached(JakData.CITY_DECORATED,true);
    }

    /** A street between two junctions in world blocks, its floor points from one to the other: see CityBlueprint.stretches. */
    public record Street(BlockPos from,BlockPos to,java.util.List<Vec3> points){}
    public static java.util.List<Street> streets(net.minecraft.world.level.levelgen.structure.StructureStart start){
        for(var piece:start.getPieces())if(piece instanceof CityPiece city){
            var out=new java.util.ArrayList<Street>();
            for(var stretch:city.site.plan().stretches()){
                var points=new java.util.ArrayList<Vec3>();
                for(var point:stretch.points())points.add(Vec3.atBottomCenterOf(CityBlueprint.rotate(point,city.turns).offset(city.origin)));
                out.add(new Street(CityBlueprint.rotate(stretch.from(),city.turns).offset(city.origin),
                    CityBlueprint.rotate(stretch.to(),city.turns).offset(city.origin),points));
            }
            return out;
        }
        return java.util.List.of();
    }
    private enum Site {
        HAVEN("haven","haven",0), SPARGUS("spargus","spargus",0),
        SWAMP_HUT("swamp_hut","jak1",4), SNOW_FORT("snow_fort","jak1",8),
        SANDOVER("sandover","jak1",26), CRATER_CAVE("crater_cave","crater_cave",10),
        // jak_structures.rs decorate: Style::Sandover with teja/cumbrera nevada and pantano.
        SANDOVER_SNOW("sandover_snow","jak1",26), SANDOVER_SWAMP("sandover_swamp","jak1",26);
        boolean sandover(){return this==SANDOVER||this==SANDOVER_SNOW||this==SANDOVER_SWAMP;}
        final String id;
        final ResourceKey<LootTable> loot;
        final int reach;
        private CityBlueprint blueprint;
        Site(String id,String loot,int reach){
            this.id=id;this.loot=ResourceKey.create(Registries.LOOT_TABLE,JakData.id("chests/"+loot));this.reach=reach;
        }
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
        if(!valid(context,x,y,z))return Optional.empty();
        int turns=context.random().nextInt(4);
        int base;
        if(site==Site.CRATER_CAVE){
            base=craterBase(context,x,z,y,turns);
            if(base==Integer.MIN_VALUE)return Optional.empty();
        }else if(site.sandover()){
            base=sandoverBase(context,x,z,y);
            if(base==Integer.MIN_VALUE)return Optional.empty();
        }else if(site==Site.SPARGUS){
            base=spargusBase(context,x,z,y);
            if(base==Integer.MIN_VALUE)return Optional.empty();
        }else if(site.reach>0){
            base=compactBase(context,x,z,y);
            if(base==Integer.MIN_VALUE)return Optional.empty();
        }else{
            if(y<=sea)return Optional.empty();
            int[] elevations=new int[49];
            int index=0,submerged=0;
            for(int dz=-84;dz<=84;dz+=28)for(int dx=-84;dx<=84;dx+=28){
                int at=height(context,x+dx,z+dz);
                elevations[index++]=at;
                if(at<=sea)submerged++;
            }
            if(submerged>4)return Optional.empty();
            Arrays.sort(elevations);
            int median=elevations[24];
            if(elevations[43]-elevations[5]>26||Math.abs(y-median)>15)return Optional.empty();
            base=Math.max(sea+3,median);
        }
        var plan=site.plan();
        if(base+plan.maxY>=context.heightAccessor().getMaxY()-4
            ||base+plan.minY<=context.heightAccessor().getMinY()+4)return Optional.empty();
        var origin=new BlockPos(x,base,z);
        return Optional.of(new GenerationStub(origin,builder->builder.addPiece(new CityPiece(site,origin,turns))));
    }

    private int craterBase(GenerationContext context,int x,int z,int ground,int turns){
        int lo=ground,hi=ground;
        var generator=context.chunkGenerator();
        for(var local:new BlockPos[]{BlockPos.ZERO,new BlockPos(10,0,0),new BlockPos(-10,0,0),
            new BlockPos(0,0,10),new BlockPos(0,0,-10),new BlockPos(40,0,0)}){
            var at=CityBlueprint.rotate(local,turns).offset(x,0,z);
            int surface=generator.getBaseHeight(at.getX(),at.getZ(),Heightmap.Types.WORLD_SURFACE_WG,
                context.heightAccessor(),context.randomState());
            if(surface<=generator.getSeaLevel()||surface!=height(context,at.getX(),at.getZ()))return Integer.MIN_VALUE;
            lo=Math.min(lo,surface);hi=Math.max(hi,surface);
        }
        return hi-lo<=8?ground:Integer.MIN_VALUE;
    }

    private int compactBase(GenerationContext context,int x,int z,int ground){
        int lo=Integer.MAX_VALUE,hi=Integer.MIN_VALUE,reach=site.reach;
        for(int dz=-reach;dz<=reach;dz+=reach)for(int dx=-reach;dx<=reach;dx+=reach){
            int surface=context.chunkGenerator().getBaseHeight(x+dx,z+dz,Heightmap.Types.WORLD_SURFACE_WG,
                context.heightAccessor(),context.randomState());
            if(!valid(context,x+dx,surface,z+dz))return Integer.MIN_VALUE;
            if(site==Site.SANDOVER&&(surface<=context.chunkGenerator().getSeaLevel()
                ||surface!=height(context,x+dx,z+dz)))return Integer.MIN_VALUE;
            lo=Math.min(lo,surface);hi=Math.max(hi,surface);
        }
        boolean swamp=site==Site.SWAMP_HUT;
        if(hi-lo>(swamp?3:site==Site.SANDOVER?7:5))return Integer.MIN_VALUE;
        if(swamp)return hi;
        return ground>context.chunkGenerator().getSeaLevel()&&hi-ground<=(site==Site.SANDOVER?7:5)?ground:Integer.MIN_VALUE;
    }

    // jak_facades.rs spargus_village: nine probes 16 apart, all dry Wasteland, their ground within 16.
    private static int spargusBase(GenerationContext context,int x,int z,int ground){
        int lo=Integer.MAX_VALUE,hi=Integer.MIN_VALUE;
        for(int[] p:new int[][]{{0,0},{16,0},{-16,0},{0,16},{0,-16},{16,16},{-16,-16},{16,-16},{-16,16}}){
            int floor=height(context,x+p[0],z+p[1]);
            if(surface(context,x+p[0],z+p[1])>floor||!valid(context,x+p[0],floor,z+p[1]))return Integer.MIN_VALUE;
            lo=Math.min(lo,floor);hi=Math.max(hi,floor);
        }
        return hi-lo>16?Integer.MIN_VALUE:ground;
    }

    // jak_structures.rs village_roofed: the plaza's 21x21 under the land's roofs, nine dry probes, ground within 7.
    private static int sandoverBase(GenerationContext context,int x,int z,int ground){
        // stand-in: roof_allowed tests all 441 columns; one column per biome quart gives the same biomes.
        for(int qx=(x-10)>>2;qx<=(x+10)>>2;qx++)for(int qz=(z-10)>>2;qz<=(z+10)>>2;qz++){
            int cx=Math.clamp(qx*4+2,x-10,x+10),cz=Math.clamp(qz*4+2,z-10,z+10);
            if(!valid(context,cx,height(context,cx,cz),cz))return Integer.MIN_VALUE;
        }
        int lo=Integer.MAX_VALUE,hi=Integer.MIN_VALUE;
        for(int[] p:new int[][]{{0,0},{12,0},{-12,0},{0,12},{0,-12},{18,18},{-18,-18},{18,-18},{-18,18}}){
            int floor=height(context,x+p[0],z+p[1]);
            if(surface(context,x+p[0],z+p[1])>floor)return Integer.MIN_VALUE;
            lo=Math.min(lo,floor);hi=Math.max(hi,floor);
        }
        return hi-lo>7?Integer.MIN_VALUE:ground;
    }

    private static int surface(GenerationContext context,int x,int z){
        return context.chunkGenerator().getBaseHeight(x,z,Heightmap.Types.WORLD_SURFACE_WG,
            context.heightAccessor(),context.randomState());
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
            var fill=switch(site){
                case SPARGUS -> Blocks.SANDSTONE.defaultBlockState();
                case SWAMP_HUT -> dev.jakmc.content.JakBlocks.BLOCKS.get("swamp_hut_log").defaultBlockState();
                case SNOW_FORT -> dev.jakmc.content.JakBlocks.BLOCKS.get("snow_rock").defaultBlockState();
                default -> Blocks.STONE.defaultBlockState();
            };
            for(int i=0;site!=Site.CRATER_CAVE&&i<cells.size();i++){
                var local=cells.pos(i);
                if(cells.states()[i].isAir()||local.getY()>(site==Site.SWAMP_HUT?0:3))continue;
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
                if(level.getBlockState(pos)!=state&&level.setBlock(pos,state,Block.UPDATE_CLIENTS|Block.UPDATE_KNOWN_SHAPE)
                    &&state.is(Blocks.CHEST)&&level.getBlockEntity(pos) instanceof ChestBlockEntity chest){
                    chest.setLootTable(site.loot,lootSeed(level.getSeed(),pos));
                }
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
            decorate(level,chunk);
            // The crater cave's band: JakEcology's nest at its CAVE_MARK (Rust: jak_enemies.rs cave_band).
            if(site==Site.CRATER_CAVE)return;
            if(site.sandover()){
                spawnResident(level,chunk,random,cells);
                return;
            }
            if(site.reach>0)return;
            // Haven's and Spargus's residents: CityPopulation, as jak2_villagers.rs peoples a town Jak comes near.
            if(site==Site.HAVEN){
                for(var patrol:plan.patrols)spawnGuard(level,chunkBounds,patrol);
                placeCrates(level,chunk,cells);
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

        private void decorate(net.minecraft.world.level.LevelAccessor level,ChunkPos chunk){
            var local=CityBlueprint.rotate(new BlockPos(chunk.x()-(origin.getX()>>4),0,chunk.z()-(origin.getZ()>>4)),(4-turns)&3);
            long key=((long)local.getX()<<32)|(local.getZ()&0xffffffffL);
            var decorations=site.plan().decorations.get(key);
            if(decorations==null)return;
            for(var decoration:decorations){
                var at=CityBlueprint.rotate(decoration.at(),turns).offset(origin);
                var support=CityBlueprint.rotate(decoration.support(),turns).offset(origin);
                if(!level.isEmptyBlock(at)||!level.getBlockState(support).is(decoration.expected().getBlock()))continue;
                if(decoration.lamp()&&!level.isEmptyBlock(at.below()))continue;
                if(!decoration.lamp()&&!level.isEmptyBlock(at.above()))continue;
                level.setBlock(at,decoration.state(),Block.UPDATE_CLIENTS|Block.UPDATE_KNOWN_SHAPE);
            }
        }

        /** The floors of Haven's buildings (houses, the Hip Hog, the Krimson station, the palace and its tower). */
        private static final java.util.Set<String> INSIDE_FLOORS=java.util.Set.of("barrio_suelo","bar_suelo","krimson_suelo",
            "palacio_suelo_metal","palacio_suelo_piedra","palacio_suelo_metal_losa");
        /**
         * Adrii 06/11: Jak's ammo crates (JakAmmoCrate) inside every Haven building, the tower and the Krimson station
         * among them, and never in the streets. stand-in for the original's spots (Haven's crates stood in its slum and
         * market streets, ctysluma and ctymarkb, and in the palace's rooms): a spot on a building's floor with a roof
         * within four blocks (not a gallery or the station's forecourt), against a wall (Haven's are pane-like blocks), two blocks clear of doors,
         * railings, beds, chests, ladders and other crates (one block in the small, furnished station); one such spot in 32
         * (in 6 in the station), three yellow to a red (ctysluma's).
         * Only spots two blocks inside the chunk, so what is round them is already this chunk's town.
         */
        private void placeCrates(WorldGenLevel level,ChunkPos chunk,CityBlueprint.Fragment cells){
            for(int i=0;i<cells.size();i++){
                var floor=cells.states()[i];String floorId=BuiltInRegistries.BLOCK.getKey(floor.getBlock()).getPath();
                if(!INSIDE_FLOORS.contains(floorId))continue;
                // The station is small and furnished: a spot in 6 there, and one block clear of its furniture.
                boolean station=floorId.equals("krimson_suelo");int every=station?6:32,room=station?1:2;
                var spot=CityBlueprint.rotate(cells.pos(i),turns).offset(origin).above();
                int x=spot.getX()-chunk.getMinBlockX(),z=spot.getZ()-chunk.getMinBlockZ();
                if(x<2||x>13||z<2||z>13)continue;
                long hash=lootSeed(level.getSeed()^0x6372617465L,spot);
                if(Long.remainderUnsigned(hash,every)!=0||!level.isEmptyBlock(spot)||!level.isEmptyBlock(spot.above()))continue;
                boolean roof=false;
                for(int h=2;h<=4&&!roof;h++)roof=!level.getBlockState(spot.above(h)).getCollisionShape(level,spot.above(h)).isEmpty();
                boolean wall=false,clear=true;
                for(var side:net.minecraft.core.Direction.Plane.HORIZONTAL)wall|=!level.getBlockState(spot.relative(side)).getCollisionShape(level,spot.relative(side)).isEmpty();
                for(var near:BlockPos.betweenClosed(spot.offset(-room,0,-room),spot.offset(room,1,room))){
                    var state=level.getBlockState(near);String id=BuiltInRegistries.BLOCK.getKey(state.getBlock()).getPath();
                    if(state.getBlock() instanceof net.minecraft.world.level.block.DoorBlock||state.getBlock() instanceof dev.jakmc.content.JakIrisDoor||id.contains("puerta")||id.contains("portal")||id.contains("baranda")
                        ||state.is(net.minecraft.tags.BlockTags.BEDS)||state.is(Blocks.CHEST)||state.is(Blocks.LADDER)||state.is(dev.jakmc.content.JakAmmoCrate.BLOCK)){clear=false;break;}
                }
                if(!roof||!wall||!clear)continue;
                var contents=Long.remainderUnsigned(hash>>>16,4)==0?dev.jakmc.content.JakAmmoCrate.Contents.RED:dev.jakmc.content.JakAmmoCrate.Contents.YELLOW;
                level.setBlock(spot,dev.jakmc.content.JakAmmoCrate.BLOCK.defaultBlockState().setValue(dev.jakmc.content.JakAmmoCrate.CONTENTS,contents),Block.UPDATE_CLIENTS);
            }
        }

        private static long lootSeed(long seed,BlockPos at){
            long value=seed^0x9e3779b97f4a7c15L^0x632be59bd9b4e019L;
            value=lootCoordinate(value,at.getX());
            value=lootCoordinate(value,at.getY());
            value=lootCoordinate(value,at.getZ());
            return value==0?1:value;
        }

        private static long lootCoordinate(long seed,int coordinate){
            long value=(seed^Integer.toUnsignedLong(coordinate))*0xff51afd7ed558ccdL;
            return value^(value>>>33);
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
                    int variant=site==Site.HAVEN||site.sandover()?random.nextInt(CitizenKind.HAVEN_VARIANTS)
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
