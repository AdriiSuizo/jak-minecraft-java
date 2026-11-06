package dev.jakmc.worldgen;

import com.google.gson.JsonParser;
import com.mojang.brigadier.exceptions.CommandSyntaxException;
import dev.jakmc.JakMod;
import dev.jakmc.content.JakAssemblies;
import dev.jakmc.content.JakKrimsonGate;
import dev.jakmc.content.JakBlocks;
import dev.jakmc.content.JakProps;
import java.io.BufferedInputStream;
import it.unimi.dsi.fastutil.ints.Int2ObjectOpenHashMap;
import java.io.DataInputStream;
import java.io.IOException;
import java.io.UncheckedIOException;
import java.nio.charset.StandardCharsets;
import java.util.ArrayList;
import java.util.HashMap;
import java.util.LinkedHashMap;
import java.util.List;
import java.util.Map;
import java.util.TreeSet;
import java.util.zip.GZIPInputStream;
import net.minecraft.commands.arguments.blocks.BlockStateParser;
import net.minecraft.core.BlockPos;
import net.minecraft.core.Direction;
import net.minecraft.core.registries.BuiltInRegistries;
import net.minecraft.resources.Identifier;
import net.minecraft.world.level.block.Block;
import net.minecraft.world.level.block.Blocks;
import net.minecraft.world.level.block.Rotation;
import net.minecraft.world.level.block.state.BlockState;
import net.minecraft.world.level.block.state.properties.BlockStateProperties;

/** Resolve the ordered source plan once and retain only its immutable per-chunk cells. */
final class CityBlueprint {
    record Fragment(int chunkX,int chunkZ,int[] cells,BlockState[] states){
        private static final Fragment EMPTY=new Fragment(0,0,new int[0],new BlockState[0]);
        BlockPos pos(int index){
            int cell=cells[index];
            return new BlockPos((chunkX<<4)+(cell&15),cell>>8,(chunkZ<<4)+(cell>>4&15));
        }
        int size(){return cells.length;}
    }
    record Patrol(BlockPos start,BlockPos end,Direction facing){}
    record Garage(BlockPos pos,Direction facing,boolean duneHopper){}
    private record Prop(int kind,BlockPos origin,int turns){}
    private record Facade(JakAssemblies.Model model,Direction direction){}
    private static final Rotation[] ROTATIONS={Rotation.NONE,Rotation.CLOCKWISE_90,Rotation.CLOCKWISE_180,Rotation.COUNTERCLOCKWISE_90};
    private final Map<Long,Fragment> chunks;
    final List<Garage> garages;
    final List<Patrol> patrols;
    /** The Krimson station's guard posts (jak:krimson_puesto): the garrison's base. */
    final List<BlockPos> posts;
    final int minX,minY,minZ,maxX,maxY,maxZ;
    /** The plan's street junctions (its light-blue markers) and the ways a street leaves each. */
    private final Map<BlockPos,java.util.Set<Direction>> streets;

    private CityBlueprint(Map<Long,Fragment> chunks,List<Garage> garages,List<Patrol> patrols,List<BlockPos> posts,int[] bounds,Map<BlockPos,java.util.Set<Direction>> streets){
        this.chunks=chunks;
        this.posts=posts;
        this.garages=garages;
        this.patrols=patrols;
        this.streets=streets;
        minX=bounds[0];minY=bounds[1];minZ=bounds[2];
        maxX=bounds[3];maxY=bounds[4];maxZ=bounds[5];
    }

    private static final java.util.concurrent.ConcurrentHashMap<String,java.util.concurrent.CompletableFuture<CityBlueprint>> PLANS=new java.util.concurrent.ConcurrentHashMap<>();
    /** The plan, read once for worldgen and the town tickers alike (each used to read its own copy). */
    static CityBlueprint read(String id){
        try{return loading(id).join();}
        catch(java.util.concurrent.CompletionException e){throw e.getCause() instanceof RuntimeException r?r:e;}
    }
    /** The plan if it is read already; otherwise null while it is read off the server thread (Haven's took ~0.5 s, a hitch on the first town). */
    static CityBlueprint ready(String id){
        var plan=loading(id);
        return plan.isDone()?read(id):null;
    }
    private static java.util.concurrent.CompletableFuture<CityBlueprint> loading(String id){
        return PLANS.computeIfAbsent(id,k->java.util.concurrent.CompletableFuture.supplyAsync(()->load(k)));
    }

    private static CityBlueprint load(String id){
        String path="/data/jakmc/catalog/"+id+".jbp";
        var cells=new HashMap<Long,Int2ObjectOpenHashMap<BlockState>>();
        var garages=new ArrayList<Garage>();
        var posts=new ArrayList<BlockPos>();
        var routes=new LinkedHashMap<BlockPos,java.util.EnumSet<Direction>>();
        var gates=new LinkedHashMap<BlockPos,BlockState>();
        var facades=new LinkedHashMap<BlockPos,Facade>();
        var missing=new TreeSet<String>();
        var bounds=new int[]{Integer.MAX_VALUE,Integer.MAX_VALUE,Integer.MAX_VALUE,
            Integer.MIN_VALUE,Integer.MIN_VALUE,Integer.MIN_VALUE};
        BlockPos palace=null;
        try(var resource=CityBlueprint.class.getResourceAsStream(path)){
            if(resource==null)throw new IOException("Missing city plan "+path);
            try(var data=new DataInputStream(new BufferedInputStream(new GZIPInputStream(resource)))){
                if(data.readInt()!=0x4A414B42||data.readUnsignedShort()!=1)throw new IOException("Unknown city blueprint format "+id);
                int count=data.readInt();
                if(count<=0||count>4096)throw new IOException("Invalid blueprint palette "+id);
                String[] names=new String[count];
                BlockState[] states=new BlockState[count];
                for(int i=0;i<count;i++){
                    names[i]=new String(data.readNBytes(data.readUnsignedShort()),StandardCharsets.UTF_8);
                    states[i]=resolve(names[i],missing);
                }
                int length=data.readInt();
                if(length<=0||length>2_000_000)throw new IOException("Invalid city cell count "+id);
                for(int i=0;i<length;i++){
                    var pos=new BlockPos(data.readShort(),data.readShort(),data.readShort());
                    int index=data.readUnsignedShort();
                    if(index>=count)throw new IOException("Invalid city palette index "+index);
                    String name=names[index];
                    if(name.startsWith("minecraft:light_blue_glazed_terracotta[")){
                        var junction=new BlockPos(pos.getX(),0,pos.getZ());
                        routes.computeIfAbsent(junction,_ ->java.util.EnumSet.noneOf(Direction.class)).add(sourceDirection(name));
                    }
                    if(name.startsWith("jak:garaje_")){
                        garages.add(new Garage(pos,sourceDirection(name),name.contains("salta_dunas")));
                        continue;
                    }
                    if(name.equals("jak:altar_turbotabla"))palace=pos.offset(0,-6,0);
                    if(name.startsWith("jak:krimson_puesto"))posts.add(pos);
                    if(name.startsWith("jak:pieza_haven_fuego")||name.startsWith("jak:pieza_haven_follaje"))continue;
                    BlockState state=states[index];
                    if(state==null)continue;
                    put(cells,bounds,pos,state);
                    if(name.startsWith("jak:")){
                        int propertyStart=name.indexOf('[');
                        var facade=JakAssemblies.FACADES.get(name.substring(4,propertyStart<0?name.length():propertyStart));
                        if(facade!=null)facades.put(pos,new Facade(facade,sourceDirection(name)));
                    }
                    if(state.getBlock() instanceof JakKrimsonGate)gates.put(pos,state);
                }
                int bytes=data.readInt();
                if(bytes<0||bytes>250_000)throw new IOException("Invalid city prop size "+id);
                String encoded=new String(data.readNBytes(bytes),StandardCharsets.UTF_8);
                for(var value:JsonParser.parseString(encoded).getAsJsonArray()){
                    var entry=value.getAsJsonObject();
                    var origin=entry.getAsJsonArray("origin");
                    addProp(cells,bounds,new Prop(entry.get("kind").getAsInt(),
                        new BlockPos(origin.get(0).getAsInt(),origin.get(1).getAsInt(),origin.get(2).getAsInt()),
                        entry.get("turns").getAsInt()));
                }
            }
        }catch(IOException failure){throw new UncheckedIOException(failure);}
        facades.forEach((anchor,facade)->{
            var model=facade.model();
            var state=cell(cells,anchor);
            if(state==null||!state.is(JakBlocks.BLOCKS.get(model.path())))return;
            for(var part:model.parts()){
                var relative=part.offset().subtract(model.anchor());
                var at=anchor.offset(rotate(relative,JakProps.turns(facade.direction())));
                var present=cell(cells,at);
                // Neighbouring door, floor and wall cells take precedence over decorative projection.
                if(!at.equals(anchor)&&present!=null&&!present.isAir())continue;
                put(cells,bounds,at,part.block().defaultBlockState().setValue(BlockStateProperties.HORIZONTAL_FACING,facade.direction()));
            }
        });
        gates.forEach((at,base)->{
            Direction facing=base.getValue(BlockStateProperties.HORIZONTAL_FACING);
            Direction lateral=facing.getClockWise();
            boolean left=!gates.containsKey(at.relative(lateral.getOpposite()));
            boolean right=!gates.containsKey(at.relative(lateral));
            for(int y=0;y<4;y++)put(cells,bounds,at.above(y),JakKrimsonGate.generationSection(base,y,left,right));
        });
        if(palace!=null){
            for(int x=-16;x<6;x++){
                addProp(cells,bounds,new Prop(105,palace.offset(x,6,1),0));
                addProp(cells,bounds,new Prop(105,palace.offset(x,6,-17),2));
            }
            addProp(cells,bounds,new Prop(106,palace.offset(-19,6,-9),0));
            addProp(cells,bounds,new Prop(107,palace.offset(-24,6,-16),0));
        }
        var index=new HashMap<Long,Fragment>();
        cells.forEach((key,chunk)->{
            int[] positions=chunk.keySet().toIntArray();
            java.util.Arrays.sort(positions);
            var states=new BlockState[positions.length];
            for(int i=0;i<positions.length;i++)states[i]=chunk.get(positions[i]);
            index.put(key,new Fragment((int)(key>>32),key.intValue(),positions,states));
        });
        JakMod.LOG.info("Jak city plan {}: {} chunk fragments, {} garage bays, {} compatibility mappings",
            id,index.size(),garages.size(),missing.size());
        if(!missing.isEmpty())JakMod.LOG.info("Jak city {} legacy palette mappings: {}",id,missing);
        var patrols=new ArrayList<Patrol>();
        routes.forEach((start,directions)->{
            // stand-in (Adrii 06/11, "un montón" of guards): a beat from every junction, where the
            // Rust kept one guard per intersection pair.
            for(var facing:directions){
                BlockPos end=start.relative(facing,28);
                if(!routes.getOrDefault(end,java.util.EnumSet.noneOf(Direction.class)).contains(facing.getOpposite()))continue;
                patrols.add(new Patrol(start,end,facing));break;
            }
        });
        List<Patrol> selected=patrols;
        if(id.equals("spargus")){
            // Two separate original street beats, one on each side of the plaza.
            var spargusPatrols=new ArrayList<Patrol>(2);
            var nearest=java.util.Comparator.comparingInt((Patrol p)->p.start().getX()*p.start().getX()+p.start().getZ()*p.start().getZ());
            for(int side:new int[]{-1,1}){
                patrols.stream().filter(p->p.start().getX()*side>0&&p.end().getX()*side>0)
                    .min(nearest).ifPresent(spargusPatrols::add);
            }
            selected=spargusPatrols;
        }
        var streets=new LinkedHashMap<BlockPos,java.util.Set<Direction>>();
        routes.forEach((junction,ways)->streets.put(junction,java.util.Collections.unmodifiableSet(java.util.EnumSet.copyOf(ways))));
        return new CityBlueprint(Map.copyOf(index),List.copyOf(garages),List.copyOf(selected),List.copyOf(posts),bounds,streets);
    }

    private static BlockState resolve(String source,TreeSet<String> missing){
        String name=source.split("\\[",2)[0];
        if(name.startsWith("jak:garaje_")||name.equals("jak:pieza_haven_fuego")||name.equals("jak:pieza_haven_follaje"))
            return null;
        // The source uses facing for its guard marker; the concrete block has no orientation.
        if(name.equals("jak:krimson_puesto"))source=name;
        try{return BlockStateParser.parseForBlock(BuiltInRegistries.BLOCK,source,false).blockState();}
        catch(CommandSyntaxException e){
            missing.add(source);
            Block block=BuiltInRegistries.BLOCK.getOptional(Identifier.parse(name)).orElse(null);
            if(block==null){
                block=switch(name){
                    case "jak:barrio_entramado","jak:barrio_toldo","jak:haven_canes","jak:barrio_andamio" -> Blocks.SPRUCE_SLAB;
                    case "jak:barrio_portal_doble","jak:sparg_portal_doble" -> Blocks.AIR;
                    case "jak:krimson_blindaje","jak:krimson_dintel","jak:krimson_insignia" -> Blocks.POLISHED_DEEPSLATE;
                    case "jak:krimson_franja" -> Blocks.RED_NETHER_BRICKS;
                    case "jak:sparg_cuerno","jak:sparg_costilla","jak:sparg_columna" -> Blocks.CUT_SANDSTONE;
                    case "jak:barrio_codo","jak:barrio_desague","jak:haven_chimenea_remate" -> Blocks.DEEPSLATE_TILE_WALL;
                    case "jak:barrio_cartel","jak:barrio_cartel_b","jak:haven_estandarte" -> Blocks.AIR;
                    default -> null;
                };
            }
            if(block==null)return null;
            var state=block.defaultBlockState();
            if(state.hasProperty(BlockStateProperties.HORIZONTAL_FACING))
                state=state.setValue(BlockStateProperties.HORIZONTAL_FACING,sourceDirection(source));
            return state;
        }
    }

    private static Direction sourceDirection(String state){
        for(var facing:Direction.Plane.HORIZONTAL)if(state.contains("facing="+facing.getName()))return facing;
        return Direction.NORTH;
    }

    private static void addProp(Map<Long,Int2ObjectOpenHashMap<BlockState>> cells,int[] bounds,Prop prop){
        var facing=ROTATIONS[prop.turns].rotate(Direction.NORTH);
        var model=JakProps.MODELS.get(prop.kind);
        if(model!=null){
            put(cells,bounds,prop.origin,model.block().defaultBlockState().setValue(BlockStateProperties.HORIZONTAL_FACING,facing));
        }else{
            var assembly=JakAssemblies.MODELS.get(prop.kind);
            if(assembly==null)return;
            for(var part:assembly.parts()){
                var cell=part.offset();
                for(int i=0;i<prop.turns;i++)cell=new BlockPos(-cell.getZ()-1,cell.getY(),cell.getX());
                int w=assembly.size().getX(),d=assembly.size().getZ();
                cell=switch(prop.turns){
                    case 1 -> cell.east(d);
                    case 2 -> cell.east(w).south(d);
                    case 3 -> cell.south(w);
                    default -> cell;
                };
                put(cells,bounds,prop.origin.offset(cell),
                    part.block().defaultBlockState().setValue(BlockStateProperties.HORIZONTAL_FACING,facing));
            }
        }
    }

    private static BlockState cell(Map<Long,Int2ObjectOpenHashMap<BlockState>> cells,BlockPos at){
        var chunk=cells.get(key(at.getX()>>4,at.getZ()>>4));
        return chunk==null?null:chunk.get((at.getY()<<8)|((at.getZ()&15)<<4)|(at.getX()&15));
    }

    private static void put(Map<Long,Int2ObjectOpenHashMap<BlockState>> cells,int[] bounds,BlockPos pos,BlockState state){
        bounds[0]=Math.min(bounds[0],pos.getX());bounds[3]=Math.max(bounds[3],pos.getX());
        bounds[1]=Math.min(bounds[1],pos.getY());bounds[4]=Math.max(bounds[4],pos.getY());
        bounds[2]=Math.min(bounds[2],pos.getZ());bounds[5]=Math.max(bounds[5],pos.getZ());
        int packed=(pos.getY()<<8)|((pos.getZ()&15)<<4)|(pos.getX()&15);
        cells.computeIfAbsent(key(pos.getX()>>4,pos.getZ()>>4),_ ->new Int2ObjectOpenHashMap<>()).put(packed,state);
    }

    /** A stretch of street between two neighbouring junctions: its floor points from one to the other, every few blocks. */
    record Stretch(BlockPos from,BlockPos to,List<BlockPos> points){}
    private List<Stretch> stretches;
    /**
     * The plan's streets for the traffic (Rust zoomers.rs Route, its town's
     * ring road). stand-in: the Rust ring sat round the old plaza, which the
     * barrio plan has not; here every street between two junctions that
     * join both ways, on street floor all along. The host keeps those open
     * in the world (the palace's walls cross some) and goes round a block.
     */
    synchronized List<Stretch> stretches(){
        if(stretches!=null)return stretches;
        var out=new ArrayList<Stretch>();
        for(var from:streets.keySet())for(var way:new Direction[]{Direction.EAST,Direction.SOUTH}){
            if(!joined(from,way))continue;
            var points=new ArrayList<BlockPos>();
            for(int along=0;along<=28;along+=4){
                var foot=streetFoot(from.relative(way,along));
                if(foot==null){points=null;break;}
                points.add(foot);
            }
            if(points!=null)out.add(new Stretch(from,from.relative(way,28),List.copyOf(points)));
        }
        // stand-in (Adrii 06/11, the ring road): the plan marks only some of the streets round its
        // edge though the rest are paved; every edge street of the junction grid on street floor
        // all along is a street too, so the traffic can go round the barrio.
        if(!streets.isEmpty()){
            int minX=Integer.MAX_VALUE,maxX=Integer.MIN_VALUE,minZ=Integer.MAX_VALUE,maxZ=Integer.MIN_VALUE;
            for(var at:streets.keySet()){minX=Math.min(minX,at.getX());maxX=Math.max(maxX,at.getX());minZ=Math.min(minZ,at.getZ());maxZ=Math.max(maxZ,at.getZ());}
            var edges=new ArrayList<BlockPos[]>();
            for(int x=minX;x+28<=maxX;x+=28)for(int z:new int[]{minZ,maxZ})edges.add(new BlockPos[]{new BlockPos(x,0,z),new BlockPos(x+28,0,z)});
            for(int z=minZ;z+28<=maxZ;z+=28)for(int x:new int[]{minX,maxX})edges.add(new BlockPos[]{new BlockPos(x,0,z),new BlockPos(x,0,z+28)});
            for(var edge:edges){
                var way=edge[1].getX()>edge[0].getX()?Direction.EAST:Direction.SOUTH;
                if(joined(edge[0],way))continue;
                var points=new ArrayList<BlockPos>();
                for(int along=0;along<=28;along+=4){
                    var foot=streetFoot(edge[0].relative(way,along));
                    if(foot==null){points=null;break;}
                    points.add(foot);
                }
                if(points!=null)out.add(new Stretch(edge[0],edge[1],List.copyOf(points)));
            }
        }
        return stretches=List.copyOf(out);
    }
    private boolean joined(BlockPos junction,Direction way){
        var ways=streets.get(junction);
        if(ways==null||!ways.contains(way))return false;
        var next=streets.get(junction.relative(way,28));
        return next!=null&&next.contains(way.getOpposite());
    }

    private int beds=-1;
    /** Rust: settlement_housing capacity, a bed to each resident: the plan's beds (their foot halves). */
    synchronized int beds(){
        if(beds>=0)return beds;
        int n=0;
        for(var fragment:chunks.values())for(var state:fragment.states())
            if(state.hasProperty(net.minecraft.world.level.block.state.properties.BlockStateProperties.BED_PART)
                &&state.getValue(net.minecraft.world.level.block.state.properties.BlockStateProperties.BED_PART)==net.minecraft.world.level.block.state.properties.BedPart.FOOT)n++;
        return beds=n;
    }

    BlockPos streetFoot(BlockPos junction){
        var fragment=chunks.get(key(junction.getX()>>4,junction.getZ()>>4));
        if(fragment==null)return null;
        int column=(junction.getX()&15)|((junction.getZ()&15)<<4);
        for(int y=2;y>=-3;y--){
            int floor=java.util.Arrays.binarySearch(fragment.cells(),((y-1)<<8)|column);
            if(floor<0||fragment.states()[floor].isAir())continue;
            boolean clear=true;
            for(int h=0;h<4;h++){
                int index=java.util.Arrays.binarySearch(fragment.cells(),((y+h)<<8)|column);
                if(index>=0&&!fragment.states()[index].isAir()){clear=false;break;}
            }
            if(clear)return new BlockPos(junction.getX(),y,junction.getZ());
        }
        return null;
    }

    Fragment cells(int chunkX,int chunkZ,int turns){
        var source=rotate(new BlockPos(chunkX,0,chunkZ),(4-turns)&3);
        return chunks.getOrDefault(key(source.getX(),source.getZ()),Fragment.EMPTY);
    }

    static BlockPos rotate(BlockPos pos,int turns){
        for(int i=0;i<turns;i++)pos=new BlockPos(-pos.getZ()-1,pos.getY(),pos.getX());
        return pos;
    }

    static BlockState rotate(BlockState state,int turns){
        return turns==0?state:state.rotate(ROTATIONS[turns]);
    }

    static Direction rotate(Direction dir,int turns){return ROTATIONS[turns].rotate(dir);}
    private static long key(int x,int z){return ((long)x<<32)|(z&0xffffffffL);}
}
