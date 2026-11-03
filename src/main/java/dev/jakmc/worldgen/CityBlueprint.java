package dev.jakmc.worldgen;

import com.google.gson.JsonParser;
import com.mojang.brigadier.exceptions.CommandSyntaxException;
import dev.jakmc.JakMod;
import dev.jakmc.content.JakAssemblies;
import dev.jakmc.content.JakKrimsonGate;
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
    record Garage(BlockPos pos,Direction facing,boolean duneHopper){}
    private record Prop(int kind,BlockPos origin,int turns){}
    private static final Rotation[] ROTATIONS={Rotation.NONE,Rotation.CLOCKWISE_90,Rotation.CLOCKWISE_180,Rotation.COUNTERCLOCKWISE_90};
    private final Map<Long,Fragment> chunks;
    final List<Garage> garages;
    final int minX,minY,minZ,maxX,maxY,maxZ;

    private CityBlueprint(Map<Long,Fragment> chunks,List<Garage> garages,int[] bounds){
        this.chunks=chunks;
        this.garages=garages;
        minX=bounds[0];minY=bounds[1];minZ=bounds[2];
        maxX=bounds[3];maxY=bounds[4];maxZ=bounds[5];
    }

    static CityBlueprint read(String id){
        String path="/data/jakmc/catalog/"+id+".jbp";
        var cells=new HashMap<Long,Int2ObjectOpenHashMap<BlockState>>();
        var garages=new ArrayList<Garage>();
        var gates=new LinkedHashMap<BlockPos,BlockState>();
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
                    if(name.startsWith("jak:garaje_")){
                        garages.add(new Garage(pos,sourceDirection(name),name.contains("salta_dunas")));
                        continue;
                    }
                    if(name.equals("jak:altar_turbotabla"))palace=pos.offset(0,-6,0);
                    if(name.startsWith("jak:pieza_haven_fuego")||name.startsWith("jak:pieza_haven_follaje"))continue;
                    BlockState state=states[index];
                    if(state==null)continue;
                    put(cells,bounds,pos,state);
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
        return new CityBlueprint(Map.copyOf(index),List.copyOf(garages),bounds);
    }

    private static BlockState resolve(String source,TreeSet<String> missing){
        String name=source.split("\\[",2)[0];
        if(name.startsWith("jak:garaje_")||name.equals("jak:pieza_haven_fuego")||name.equals("jak:pieza_haven_follaje"))
            return null;
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

    private static void put(Map<Long,Int2ObjectOpenHashMap<BlockState>> cells,int[] bounds,BlockPos pos,BlockState state){
        bounds[0]=Math.min(bounds[0],pos.getX());bounds[3]=Math.max(bounds[3],pos.getX());
        bounds[1]=Math.min(bounds[1],pos.getY());bounds[4]=Math.max(bounds[4],pos.getY());
        bounds[2]=Math.min(bounds[2],pos.getZ());bounds[5]=Math.max(bounds[5],pos.getZ());
        int packed=(pos.getY()<<8)|((pos.getZ()&15)<<4)|(pos.getX()&15);
        cells.computeIfAbsent(key(pos.getX()>>4,pos.getZ()>>4),_ ->new Int2ObjectOpenHashMap<>()).put(packed,state);
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
