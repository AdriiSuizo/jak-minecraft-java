package dev.jakmc.worldgen;

import dev.jakmc.JakMod;
import java.util.*;
import java.util.concurrent.ConcurrentHashMap;
import net.minecraft.core.BlockPos;
import net.minecraft.core.Holder;
import net.minecraft.core.registries.BuiltInRegistries;
import net.minecraft.resources.Identifier;
import net.minecraft.resources.ResourceKey;
import net.minecraft.world.level.ChunkPos;
import net.minecraft.world.level.Level;
import net.minecraft.world.level.WorldGenLevel;
import net.minecraft.world.level.biome.Biome;
import net.minecraft.world.level.biome.BiomeResolver;
import net.minecraft.world.level.biome.Biomes;
import net.minecraft.world.level.block.Block;
import net.minecraft.world.level.block.Blocks;
import net.minecraft.world.level.block.state.BlockState;
import net.minecraft.world.level.block.state.properties.Property;
import net.minecraft.world.level.chunk.ChunkAccess;
import net.minecraft.world.level.chunk.ChunkGenerator;
import net.minecraft.world.level.levelgen.Heightmap;
import net.minecraft.world.level.levelgen.NoiseBasedChunkGenerator;
import net.minecraft.world.level.levelgen.NoiseGeneratorSettings;
import net.minecraft.world.level.levelgen.RandomState;

/**
 * The Jak lands (Rust minecraft_terrain jak_lands.rs, wasteland.rs, snowy.rs,
 * swamp.rs; Adrii 23/10): Minecraft's deserts are Jak 3's Wasteland, its snowy
 * lands Jak 1's Snowy Mountain and its swamps Boggy Swamp. The ground stays
 * Minecraft's (its noise, caves, water and deep stone); in each land's columns
 * the blocks down to `depth` under the top become the land's, rule by rule, by
 * the slope they stand on; tops get eighths up to where the noise truly tops
 * out; cave walls and floors are the land's; then each land's own decor.
 *
 * Two passes: the noise's own top and its sub-block surface are recorded as
 * the surface is built (before carvers, trees and plants), and the chunk is
 * made the land's after Minecraft's features, so trees, plants and snow are
 * converted too, as the Rust did on its final chunks.
 */
public final class JakLands {
    private JakLands(){}

    public enum Slope{FLAT,EDGE,STEEP,FOOT}
    @FunctionalInterface interface Rule{String to(Slope slope,int depth);}
    record Layers(List<String> tops,String whole,String eighths,int least){}
    @FunctionalInterface interface Decor{void decorate(Context context,Shaped shaped);}
    record Land(String name,List<ResourceKey<Biome>> biomes,int depth,int steep,long salt,List<String> strip,
                Map<String,Rule> rules,List<Layers> layers,String caveWall,String caveFloor,Decor decor){}

    /** How far over a column's ground what grew on it reaches (Minecraft's tallest trees). */
    private static final int ABOVE=40;

    // ---- Wasteland (wasteland.rs) ----
    static final String SAND="jak:wasteland_sand",ROCK="jak:wasteland_rock",CLIFF="jak:wasteland_cliff",RED_ROCK="jak:red_rock",
        DRY_GRASS="jak:dry_grass",PEBBLES="jak:desert_pebbles",PALM_LOG="jak:palm_log",PALM_LEAVES="jak:palm_leaves",
        CLIFF_EDGE="jak:wasteland_cliff_edge",SCRUB_ROCK="jak:wasteland_scrub_rock",CAVE_ROCK="jak:wasteland_cave_rock",
        CAVE_FLOOR="jak:wasteland_cave_floor";
    static final Land WASTELAND=new Land("Wasteland",List.of(Biomes.DESERT),12,3,71,List.of(),Map.of(
        "minecraft:sand",(s,d)->s==Slope.STEEP&&d<4?CLIFF:s==Slope.EDGE&&d<2?CLIFF_EDGE:s==Slope.FOOT&&d==0?SCRUB_ROCK:SAND,
        "minecraft:sandstone",(s,d)->s==Slope.STEEP?CLIFF:ROCK,
        "minecraft:stone",(s,d)->s==Slope.STEEP&&d<4?CLIFF:null,
        "minecraft:dead_bush",(s,d)->DRY_GRASS),
        List.of(new Layers(List.of(SAND),SAND,"jak:wasteland_sand_layer",0),
                new Layers(List.of(ROCK,SCRUB_ROCK,CLIFF_EDGE),ROCK,"jak:wasteland_rock_layer",0)),
        CAVE_ROCK,CAVE_FLOOR,JakLands::wasteland);

    // ---- Snowy Mountain (snowy.rs) ----
    static final String SNOW="jak:snow_ground",TURF="jak:snow_turf",PACKED="jak:snow_packed",SNOW_ROCK="jak:snow_rock",
        ROCK_HI="jak:snow_rock_hi",ICE_WALL="jak:snow_ice_wall",ICICLE="jak:snow_icicle";
    static final Land SNOWY=new Land("Snowy Mountain",List.of(Biomes.SNOWY_PLAINS,Biomes.SNOWY_SLOPES,Biomes.FROZEN_PEAKS,
        Biomes.JAGGED_PEAKS,Biomes.SNOWY_TAIGA,Biomes.ICE_SPIKES,Biomes.GROVE,Biomes.SNOWY_BEACH),12,3,171,List.of("minecraft:snow"),
        rules(
        "minecraft:snow_block",(Rule)(s,d)->s==Slope.STEEP&&d<4?SNOW_ROCK:SNOW,
        "minecraft:powder_snow",(Rule)(s,d)->SNOW,
        "minecraft:grass_block",(Rule)(s,d)->TURF,
        "minecraft:dirt",(Rule)(s,d)->PACKED,
        "minecraft:coarse_dirt",(Rule)(s,d)->PACKED,
        "minecraft:podzol",(Rule)(s,d)->TURF,
        "minecraft:stone",(Rule)(s,d)->SNOW_ROCK,
        "minecraft:packed_ice",(Rule)(s,d)->ICE_WALL,
        "minecraft:blue_ice",(Rule)(s,d)->ICE_WALL,
        "minecraft:ice",(Rule)(s,d)->"jak:snow_ice",
        "minecraft:short_grass",(Rule)(s,d)->"jak:snow_grass",
        "minecraft:fern",(Rule)(s,d)->"jak:snow_grass",
        "minecraft:spruce_log",(Rule)(s,d)->"jak:snow_pole",
        "minecraft:spruce_leaves",(Rule)(s,d)->"jak:snow_pine"),
        List.of(new Layers(List.of(SNOW,TURF,PACKED),SNOW,"jak:snow_layer",1)),ROCK_HI,SNOW_ROCK,JakLands::icicles);

    // ---- Boggy Swamp (swamp.rs) ----
    static final Land SWAMP=new Land("Boggy Swamp",List.of(Biomes.SWAMP,Biomes.MANGROVE_SWAMP),12,3,271,List.of(),
        rules(
        "minecraft:grass_block",(Rule)(s,d)->"jak:swamp_turf",
        "minecraft:dirt",(Rule)(s,d)->"jak:swamp_mud",
        "minecraft:mud",(Rule)(s,d)->"jak:swamp_mud_top",
        "minecraft:clay",(Rule)(s,d)->"jak:swamp_mud_bot",
        "minecraft:stone",(Rule)(s,d)->"jak:swamp_stone",
        "minecraft:oak_log",(Rule)(s,d)->"jak:swamp_bark",
        "minecraft:mangrove_log",(Rule)(s,d)->"jak:swamp_bark",
        "minecraft:mangrove_roots",(Rule)(s,d)->"jak:swamp_roots",
        "minecraft:muddy_mangrove_roots",(Rule)(s,d)->"jak:swamp_roots",
        "minecraft:oak_leaves",(Rule)(s,d)->"jak:swamp_leaves",
        "minecraft:mangrove_leaves",(Rule)(s,d)->"jak:swamp_leaves",
        "minecraft:vine",(Rule)(s,d)->"jak:swamp_moss",
        "minecraft:lily_pad",(Rule)(s,d)->"jak:swamp_lily",
        "minecraft:short_grass",(Rule)(s,d)->"jak:swamp_grass",
        "minecraft:fern",(Rule)(s,d)->"jak:swamp_fern"),
        List.of(new Layers(List.of("jak:swamp_turf"),"jak:swamp_turf","jak:swamp_leafy_layer",0),
                new Layers(List.of("jak:swamp_mud_top","jak:swamp_mud"),"jak:swamp_mud_top","jak:swamp_mud_layer",0)),
        "jak:swamp_stonewall","jak:swamp_mud",JakLands::scum);

    public static final List<Land> ALL=List.of(WASTELAND,SNOWY,SWAMP);

    private static Map<String,Rule> rules(Object... pairs){
        var out=new LinkedHashMap<String,Rule>();
        for(int i=0;i<pairs.length;i+=2)out.put((String)pairs[i],(Rule)pairs[i+1]);
        return out;
    }

    // ---- Blocks by name, looked up once ----
    private static final Map<String,BlockState> STATES=new ConcurrentHashMap<>();
    static BlockState state(String name){
        return STATES.computeIfAbsent(name,n->{
            var id=Identifier.parse(n);
            if(!BuiltInRegistries.BLOCK.containsKey(id)){JakMod.LOG.warn("Jak land block missing: {}",n);return Blocks.AIR.defaultBlockState();}
            return BuiltInRegistries.BLOCK.getValue(id).defaultBlockState();
        });
    }
    static Block block(String name){return state(name).getBlock();}
    private static final Map<String,BlockState[]> EIGHTHS=new ConcurrentHashMap<>();
    /** The layer block holding `n` (1 to 7) eighths. */
    static BlockState eighths(String name,int n){
        return EIGHTHS.computeIfAbsent(name,k->{
            var base=state(k);var out=new BlockState[8];
            for(int i=1;i<8;i++)out[i]=with(base,"layers",String.valueOf(i));
            return out;
        })[n];
    }
    static BlockState with(BlockState state,String property,String value){
        Property<?> p=state.getBlock().getStateDefinition().getProperty(property);
        return p==null?state:set(state,p,value);
    }
    private static <T extends Comparable<T>> BlockState set(BlockState s,Property<T> p,String v){
        return p.getValue(v).map(x->s.setValue(p,x)).orElse(s);
    }
    private static <T extends Comparable<T>> String name(BlockState s,Property<T> p){return p.getName(s.getValue(p));}
    /** `to` with every property it shares with `from` as `from` has it (a log's axis, leaves' distance, a vine's faces). */
    static BlockState carried(BlockState from,BlockState to){
        for(Property<?> p:from.getProperties()){
            var q=to.getBlock().getStateDefinition().getProperty(p.getName());
            if(q!=null)to=set(to,q,name(from,p));
        }
        return to;
    }

    /** jak_lands::mix, bit for bit. */
    static long mix(long seed,int a,int b,long salt){
        long h=seed^(((long)a<<32)|(b&0xffffffffL))^(salt*0x9E3779B97F4A7C15L);
        h^=h>>>33;h*=0xff51afd7ed558ccdL;h^=h>>>33;h*=0xc4ceb9fe1a85ec53L;
        return h^(h>>>33);
    }
    static int percent(long h,int n){return (int)Long.remainderUnsigned(h,n);}

    // ---- Pass 1: the noise's own top as the surface is built ----
    /** A chunk's columns' top block and the share of the block over it the noise fills. */
    record Surface(int[] tops,float[] fractions){}
    private static final Map<Long,Surface> SURFACES=new ConcurrentHashMap<>();
    /** Diagnostics for the fixtures: chunks recorded, columns with a sub-block surface, chunks shaped with a record, eighths laid. */
    public static final java.util.concurrent.atomic.AtomicLong RECORDED=new java.util.concurrent.atomic.AtomicLong(),
        FRACTIONS=new java.util.concurrent.atomic.AtomicLong(),WITH_RECORD=new java.util.concurrent.atomic.AtomicLong(),
        WITHOUT_RECORD=new java.util.concurrent.atomic.AtomicLong(),EIGHTHS_LAID=new java.util.concurrent.atomic.AtomicLong(),
        NANOS=new java.util.concurrent.atomic.AtomicLong();
    public static volatile String sample="";

    public static void recordSurface(NoiseBasedChunkGenerator generator,ChunkAccess chunk,net.minecraft.world.level.levelgen.NoiseChunk noise,RandomState random){
        long start=System.nanoTime();
        try{record(generator,chunk,noise,random);}finally{NANOS.addAndGet(System.nanoTime()-start);}
    }
    private static void record(NoiseBasedChunkGenerator generator,ChunkAccess chunk,net.minecraft.world.level.levelgen.NoiseChunk noise,RandomState random){
        if(!generator.stable(NoiseGeneratorSettings.OVERWORLD))return;
        var pos=chunk.getPos();
        int qy=(Math.max(chunk.getHeight(Heightmap.Types.WORLD_SURFACE_WG,8,8)+1,63))>>2;
        boolean any=false;
        outer:
        for(int i=0;i<4;i++)for(int j=0;j<4;j++){
            var biome=chunk.getNoiseBiome((pos.x()<<2)+i,qy,(pos.z()<<2)+j);
            for(var land:ALL)if(in(land,biome)){any=true;break outer;}
        }
        // Lands reach one quart past their biome (their edge blends column by column).
        if(!any){
            var resolver=generator.getBiomeSource().createUncachedResolver(random);
            ring:
            for(int i=-1;i<=4;i++)for(int j=-1;j<=4;j++){
                if(i>=0&&i<4&&j>=0&&j<4)continue;
                var biome=resolver.getNoiseBiome((pos.x()<<2)+i,qy,(pos.z()<<2)+j);
                for(var land:ALL)if(in(land,biome)){any=true;break ring;}
            }
            if(!any)return;
        }
        var density=generator.generatorSettings().value().noiseRouter().finalDensity();
        int[] tops=new int[256];float[] fractions=new float[256];
        var at=new BlockPos.MutableBlockPos();
        for(int x=0;x<16;x++)for(int z=0;z<16;z++){
            int t=topOf(chunk,x,z,at);
            tops[x*16+z]=t;
            int wx=pos.getMinBlockX()+x,wz=pos.getMinBlockZ()+z;
            // The chunk's own samplers: the density its blocks were filled from (the
            // uncached one skips the cell interpolation and stands off the surface).
            var samplers=noise.cachingSamplers();
            float d0=samplers.sampleValue(density,wx,t,wz),d1=samplers.sampleValue(density,wx,t+1,wz);
            fractions[x*16+z]=d0>0&&d1<=0?Math.clamp(d0/(d0-d1),0f,1f):0f;
            if(fractions[x*16+z]>0)FRACTIONS.incrementAndGet();
            if(x==8&&z==8)sample="t="+t+" d0="+d0+" d1="+d1;
        }
        RECORDED.incrementAndGet();
        if(SURFACES.size()>8192)SURFACES.clear();
        SURFACES.put(ChunkPos.pack(pos.x(),pos.z()),new Surface(tops,fractions));
    }
    private static int topOf(ChunkAccess chunk,int x,int z,BlockPos.MutableBlockPos at){
        // ChunkAccess.getHeight is the top block itself (the heightmap's first free height less one).
        int t=chunk.getHeight(Heightmap.Types.WORLD_SURFACE_WG,x,z);
        if(t>chunk.getMinY())return t;
        for(int y=chunk.getMaxY();y>chunk.getMinY();y--)if(!chunk.getBlockState(at.set(x,y,z)).isAir())return y;
        return chunk.getMinY();
    }
    static boolean in(Land land,Holder<Biome> biome){
        for(var key:land.biomes)if(biome.is(key))return true;
        return false;
    }

    // ---- Pass 2: after Minecraft's features, the chunk becomes the land's ----
    public static final class Context {
        final WorldGenLevel level;final ChunkAccess chunk;final NoiseBasedChunkGenerator generator;final RandomState random;
        final long seed;final int x0,z0;final BlockPos.MutableBlockPos at=new BlockPos.MutableBlockPos();
        private BiomeResolver resolver;
        Context(WorldGenLevel level,ChunkAccess chunk,NoiseBasedChunkGenerator generator,RandomState random){
            this.level=level;this.chunk=chunk;this.generator=generator;this.random=random;seed=level.getSeed();
            x0=chunk.getPos().getMinBlockX();z0=chunk.getPos().getMinBlockZ();
        }
        BlockState get(int x,int y,int z){return chunk.getBlockState(at.set(x0+x,y,z0+z));}
        // A structure's chest, spawner or bed stays: the land shapes round it.
        void put(int x,int y,int z,BlockState state){var pos=new BlockPos(x0+x,y,z0+z);if(chunk.getBlockEntity(pos)==null)chunk.setBlockState(pos,state);}
        BiomeResolver biomes(){if(resolver==null)resolver=generator.getBiomeSource().createUncachedResolver(random);return resolver;}
        /** The first free height over the column, as the Rust heightmap gave it. */
        int surface(int x,int z){return chunk.getHeight(Heightmap.Types.WORLD_SURFACE_WG,x,z)+1;}
        int ceiling(){return chunk.getMaxY();}
    }
    public static final class Shaped {
        final boolean[] columns=new boolean[256];final int[] tops=new int[256];
        final Slope[] slopes=new Slope[256];final boolean[] layered=new boolean[256];
        boolean column(int x,int z){return columns[x*16+z];}
        int top(int x,int z){return tops[x*16+z];}
    }

    public static void decorate(WorldGenLevel level,ChunkAccess chunk,ChunkGenerator generator){
        long start=System.nanoTime();
        try{shapeChunk(level,chunk,generator);}finally{NANOS.addAndGet(System.nanoTime()-start);}
    }
    private static void shapeChunk(WorldGenLevel level,ChunkAccess chunk,ChunkGenerator generator){
        if(!(generator instanceof NoiseBasedChunkGenerator noise)||!noise.stable(NoiseGeneratorSettings.OVERWORLD))return;
        if(level.getLevel().dimension()!=Level.OVERWORLD)return;
        var surface=SURFACES.remove(ChunkPos.pack(chunk.getPos().x(),chunk.getPos().z()));
        var random=level.getLevel().getChunkSource().randomState();
        var context=new Context(level,chunk,noise,random);
        (surface==null?WITHOUT_RECORD:WITH_RECORD).incrementAndGet();
        for(var land:ALL){
            try{
                var shaped=shape(land,context,surface);
                if(shaped!=null&&land.decor!=null)land.decor.decorate(context,shaped);
            }catch(RuntimeException failure){
                JakMod.LOG.error("Jak land {} could not shape chunk {}",land.name,chunk.getPos(),failure);
            }
        }
    }

    /** The land's share round each quart of the chunk (jak_lands::shares); null where it has none. */
    private static float[] shares(Land land,Context c){
        var pos=c.chunk.getPos();
        int qy=Math.max(c.surface(8,8),63)>>2;
        boolean[] grid=new boolean[36];boolean any=false;
        for(int i=0;i<6;i++)for(int j=0;j<6;j++){
            int qx=(pos.x()<<2)+i-1,qz=(pos.z()<<2)+j-1;
            boolean inside=i>=1&&i<5&&j>=1&&j<5;
            var biome=inside?c.chunk.getNoiseBiome(qx,qy,qz):c.biomes().getNoiseBiome(qx,qy,qz);
            grid[i*6+j]=in(land,biome);any|=grid[i*6+j];
        }
        if(!any)return null;
        float[] out=new float[16];
        for(int i=0;i<4;i++)for(int j=0;j<4;j++){
            int n=0;for(int di=0;di<3;di++)for(int dj=0;dj<3;dj++)if(grid[(i+di)*6+j+dj])n++;
            out[i*4+j]=n/9f;
        }
        return out;
    }
    static boolean columnIs(Land land,long seed,int x,int z,float share){
        return share>=1||share>0&&percent(mix(seed,x,z,land.salt),1000)/1000f<share;
    }

    private static Shaped shape(Land land,Context c,Surface surface){
        float[] shares=shares(land,c);
        if(shares==null)return null;
        var out=new Shaped();
        BlockState air=Blocks.AIR.defaultBlockState();
        for(int x=0;x<16;x++)for(int z=0;z<16;z++){
            out.columns[x*16+z]=columnIs(land,c.seed,c.x0+x,c.z0+z,shares[(x/4)*4+z/4]);
            out.tops[x*16+z]=surface!=null?surface.tops[x*16+z]:fallbackTop(c,x,z);
        }
        var strip=new HashSet<Block>();for(var n:land.strip)strip.add(block(n));
        var rules=new HashMap<Block,Rule>();land.rules.forEach((n,r)->rules.put(block(n),r));
        int ceiling=c.ceiling();
        for(int x=0;x<16;x++)for(int z=0;z<16;z++){
            if(!out.column(x,z))continue;
            int t=out.top(x,z);
            int r1=rise(out,x,z,1,t),r2=rise(out,x,z,2,t);
            Slope slope=r1>=land.steep?Slope.STEEP:r1==land.steep-1?Slope.EDGE:r2>=land.steep?Slope.FOOT:Slope.FLAT;
            out.slopes[x*16+z]=slope;
            for(int y=Math.min(t+ABOVE,ceiling);y>=t-land.depth;y--){
                var old=c.get(x,y,z);
                if(old.isAir())continue;
                var b=old.getBlock();
                if(strip.contains(b)){c.put(x,y,z,air);continue;}
                var rule=rules.get(b);
                if(rule==null)continue;
                String to=rule.to(slope,Math.max(t-y,0));
                if(to!=null)c.put(x,y,z,carried(old,state(to)));
            }
            // Its eighths over its top block, up to the noise's true surface.
            if(!c.get(x,t+1,z).isAir())continue;
            var top=c.get(x,t,z);
            Layers layers=null;
            for(var l:land.layers)for(var n:l.tops)if(top.is(block(n)))layers=l;
            if(layers==null)continue;
            float fraction=surface==null?0:surface.fractions[x*16+z];
            int n=Math.max(Math.round(fraction*8),layers.least);
            if(n==0)continue;
            c.put(x,t+1,z,n>=8?state(layers.whole):eighths(layers.eighths,n));
            out.layered[x*16+z]=true;
            EIGHTHS_LAID.incrementAndGet();
        }
        if(land.caveWall!=null)caves(c,out,state(land.caveWall),state(land.caveFloor));
        return out;
    }
    private static int rise(Shaped s,int x,int z,int reach,int t){
        int most=0;
        for(int[] d:new int[][]{{1,0},{-1,0},{0,1},{0,-1}}){
            int nx=x+d[0]*reach,nz=z+d[1]*reach;
            if(nx>=0&&nx<16&&nz>=0&&nz<16)most=Math.max(most,Math.abs(s.top(nx,nz)-t));
        }
        return most;
    }
    /** Without the noise's record (a chunk resumed from an older save): the first ground block, past what grew on it. */
    private static int fallbackTop(Context c,int x,int z){
        for(int y=c.surface(x,z)-1;y>c.chunk.getMinY();y--){
            var s=c.get(x,y,z);
            if(s.isAir()||s.canBeReplaced()||s.is(net.minecraft.tags.BlockTags.LEAVES)||s.is(net.minecraft.tags.BlockTags.LOGS)
                ||s.is(Blocks.CACTUS)||s.is(Blocks.SNOW))continue;
            return y;
        }
        return c.chunk.getMinY();
    }
    /** The stone a cave's air touches is the land's cave wall; under that air, its floor. */
    private static void caves(Context c,Shaped s,BlockState wall,BlockState floor){
        BlockState stone=Blocks.STONE.defaultBlockState();
        int bottom=c.chunk.getMinY();
        var found=new ArrayList<long[]>();
        for(int x=0;x<16;x++)for(int z=0;z<16;z++){
            if(!s.column(x,z))continue;
            for(int y=bottom+1;y<s.top(x,z)-1;y++){
                if(c.get(x,y,z)!=stone)continue;
                if(open(c,x,y+1,z))found.add(new long[]{x,y,z,1});
                else if(open(c,x+1,y,z)||open(c,x-1,y,z)||open(c,x,y,z+1)||open(c,x,y,z-1)||open(c,x,y-1,z))found.add(new long[]{x,y,z,0});
            }
        }
        for(long[] f:found)c.put((int)f[0],(int)f[1],(int)f[2],f[3]==1?floor:wall);
    }
    private static boolean open(Context c,int x,int y,int z){return x>=0&&x<16&&z>=0&&z<16&&c.get(x,y,z).isAir();}

    // ---- Wasteland's own ----
    private static void wasteland(Context c,Shaped s){
        BlockState sand=state(SAND);
        for(int x=0;x<16;x++)for(int z=0;z<16;z++){
            int t=s.top(x,z);
            if(!s.column(x,z)||s.layered[x*16+z]||c.get(x,t,z)!=sand||!c.get(x,t+1,z).isAir())continue;
            int roll=percent(mix(c.seed,c.x0+x,c.z0+z,72),100);
            if(roll<=4)c.put(x,t+1,z,state(DRY_GRASS));
            else if(roll<=6)c.put(x,t+1,z,state(PEBBLES));
        }
        // Palms by the water: on its sand, a crown two blocks round inside the chunk.
        BlockState water=Blocks.WATER.defaultBlockState();
        for(int x=2;x<14;x++)for(int z=2;z<14;z++){
            if(!s.column(x,z))continue;
            int wx=c.x0+x,wz=c.z0+z,t=s.top(x,z);
            if(c.get(x,t,z)!=sand||percent(mix(c.seed,wx,wz,73),100)>=7)continue;
            boolean wet=false;
            for(int dx=-3;dx<=3&&!wet;dx++)for(int dz=-3;dz<=3&&!wet;dz++){
                int nx=x+dx,nz=z+dz;
                wet=nx>=0&&nx<16&&nz>=0&&nz<16&&c.get(nx,t,nz)==water;
            }
            if(wet)palm(c,x,t+1,z,mix(c.seed,wx,wz,74));
        }
        rockFormations(c);
    }
    private static void palm(Context c,int x,int y,int z,long h){
        int tall=5+percent(h,3);
        BlockState log=with(state(PALM_LOG),"axis","y"),leaves=with(state(PALM_LEAVES),"persistent","true");
        for(int k=0;k<tall;k++)c.put(x,y+k,z,log);
        int crown=y+tall;
        int[][] flat={{0,0,0},{1,0,0},{-1,0,0},{0,0,1},{0,0,-1},{2,-1,0},{-2,-1,0},{0,-1,2},{0,-1,-2},{1,-1,1},{-1,-1,1},{1,-1,-1},{-1,-1,-1}};
        for(int[] d:flat){
            int px=x+d[0],pz=z+d[2];
            if(px>=0&&px<16&&pz>=0&&pz<16&&c.get(px,crown+d[1],pz).isAir())c.put(px,crown+d[1],pz,leaves);
        }
    }
    private static final int ROCK_CELL=40;
    private static final Map<Long,Optional<Integer>> ROCK_GROUND=new ConcurrentHashMap<>();
    private static long cachedSeed;
    /** The red rock's spires and mesas: one perhaps in each 40-block square of the Wasteland, its part over this chunk. */
    private static void rockFormations(Context c){
        synchronized(ROCK_GROUND){if(cachedSeed!=c.seed||ROCK_GROUND.size()>65536){ROCK_GROUND.clear();cachedSeed=c.seed;}}
        BlockState red=state(RED_ROCK),cliff=state(CLIFF),grass=state(DRY_GRASS);
        for(int cx=Math.floorDiv(c.x0-14,ROCK_CELL);cx<=Math.floorDiv(c.x0+30,ROCK_CELL);cx++)
        for(int cz=Math.floorDiv(c.z0-14,ROCK_CELL);cz<=Math.floorDiv(c.z0+30,ROCK_CELL);cz++){
            long h=mix(c.seed,cx,cz,81);
            if(percent(h,100)>=45)continue;
            int sx=cx*ROCK_CELL+8+percent(mix(c.seed,cx,cz,82),ROCK_CELL-16);
            int sz=cz*ROCK_CELL+8+percent(mix(c.seed,cx,cz,83),ROCK_CELL-16);
            final int fx=sx,fz=sz;
            var found=ROCK_GROUND.computeIfAbsent((long)cx<<32|(cz&0xffffffffL),k->{
                int ground=c.generator.getBaseHeight(fx,fz,Heightmap.Types.OCEAN_FLOOR_WG,c.level,c.random);
                return c.biomes().getNoiseBiome(fx>>2,ground>>2,fz>>2).is(Biomes.DESERT)?Optional.of(ground):Optional.empty();
            });
            if(found.isEmpty())continue;
            int ground=found.get();
            boolean mesa=percent(h,100)<12;
            float radius=mesa?7+percent(mix(c.seed,cx,cz,84),5):2+percent(mix(c.seed,cx,cz,84),3);
            int height=mesa?7+percent(mix(c.seed,cx,cz,85),6):12+percent(mix(c.seed,cx,cz,85),15);
            int reach=(int)radius+2;
            for(int x=0;x<16;x++)for(int z=0;z<16;z++){
                int wx=c.x0+x,wz=c.z0+z;
                float dx=wx-sx,dz=wz-sz;
                if(Math.abs(dx)>reach||Math.abs(dz)>reach)continue;
                float d=(float)Math.sqrt(dx*dx+dz*dz);
                int base=c.surface(x,z)-1;
                for(int k=0;k<height;k++){
                    int y=ground+k;
                    float wobble=(percent(mix(c.seed,wx,y,86),100)/100f-.5f)*1.2f;
                    float r=mesa?radius+wobble-(k>=height-2?.8f:0):radius*(1-.55f*k/height)+wobble*.6f;
                    if(d>r)continue;
                    c.put(x,y,z,k==height-1||k%5==4?cliff:red);
                }
                if(d<=radius)for(int y=Math.min(base,ground);y<ground;y++){
                    var here=c.get(x,y,z);
                    if(here.isAir()||here==grass)c.put(x,y,z,red);
                }
                if(mesa&&d<=radius-1.5f&&percent(mix(c.seed,wx,wz,87),100)<10)c.put(x,ground+height,z,grass);
            }
        }
    }

    // ---- Snowy Mountain's icicles under overhangs ----
    private static void icicles(Context c,Shaped s){
        var holds=new HashSet<BlockState>();for(var n:List.of(SNOW,SNOW_ROCK,ROCK_HI,PACKED,ICE_WALL))holds.add(state(n));
        BlockState icicle=with(state(ICICLE),"waterlogged","false");
        for(int x=0;x<16;x++)for(int z=0;z<16;z++){
            int t=c.surface(x,z);
            for(int y=Math.max(t-24,-60);y<t+8;y++){
                if(!c.get(x,y,z).isAir()||!c.get(x,y-1,z).isAir()||!holds.contains(c.get(x,y+1,z)))continue;
                if(percent(mix(c.seed,c.x0+x,c.z0+z,SNOWY.salt+(y&0xffffffffL)),4)==0)c.put(x,y,z,icicle);
            }
        }
    }

    // ---- Boggy Swamp's scum on still water ----
    private static void scum(Context c,Shaped s){
        BlockState scum=state("jak:swamp_scum");
        for(int x=0;x<16;x++)for(int z=0;z<16;z++){
            int t=c.surface(x,z);
            for(int y=t+2;y>=t-3;y--){
                var here=c.get(x,y,z);
                if(here.isAir())continue;
                if(here.is(Blocks.WATER)&&c.get(x,y+1,z).isAir()&&percent(mix(c.seed,c.x0+x,c.z0+z,SWAMP.salt+7),8)==0)c.put(x,y+1,z,scum);
                break;
            }
        }
    }
}
