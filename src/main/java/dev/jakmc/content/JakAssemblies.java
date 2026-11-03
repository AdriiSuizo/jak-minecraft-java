package dev.jakmc.content;

import com.google.gson.JsonArray;
import com.google.gson.JsonObject;
import java.util.ArrayList;
import java.util.LinkedHashMap;
import java.util.List;
import java.util.Map;
import java.util.Optional;
import java.util.HashSet;
import java.util.Set;
import net.minecraft.core.BlockPos;
import net.minecraft.core.component.DataComponents;
import net.minecraft.core.Direction;
import net.minecraft.core.Registry;
import net.minecraft.core.registries.BuiltInRegistries;
import net.minecraft.core.registries.Registries;
import net.minecraft.resources.Identifier;
import net.minecraft.resources.ResourceKey;
import net.minecraft.server.level.ServerLevel;
import net.minecraft.world.item.BlockItem;
import net.minecraft.world.item.Item;
import net.minecraft.world.item.context.BlockPlaceContext;
import net.minecraft.world.item.component.BlockItemStateProperties;
import net.minecraft.world.level.BlockGetter;
import net.minecraft.world.level.Level;
import net.minecraft.world.level.block.Block;
import net.minecraft.world.level.block.Blocks;
import net.minecraft.world.level.block.state.BlockBehaviour;
import net.minecraft.world.level.block.state.BlockState;
import net.minecraft.world.level.block.state.properties.BlockStateProperties;
import net.minecraft.world.level.material.PushReaction;
import net.minecraft.world.phys.shapes.CollisionContext;
import net.minecraft.world.phys.shapes.VoxelShape;

/** Native grid assemblies use one inventory item and an owned, bounded set of Minecraft block states. */
public final class JakAssemblies {
    public record PartModel(Block block,BlockPos offset,JsonArray triangles){}
    public record Model(int id,String path,int extent,List<PartModel> parts){}
    public static final Map<Integer,Model> MODELS=new LinkedHashMap<>();

    private record Removed(ServerLevel level,BlockPos origin,Assembly assembly){}
    private static final Set<Removed> REMOVING=new HashSet<>();
    private static final int PLACE_FLAGS=Block.UPDATE_CLIENTS|Block.UPDATE_KNOWN_SHAPE;

    public static void initialize(){
        for(var entry:JakBlocks.resource("/data/jakmc/catalog/assemblies.json").getAsJsonObject().entrySet()){
            int id=Integer.parseInt(entry.getKey());
            var source=JakBlocks.resource("/data/jakmc/catalog/objects/"+id+".json").getAsJsonObject();
            var assembly=new Assembly(source,position(entry.getValue().getAsJsonArray()));
            MODELS.put(id,new Model(id,assembly.path,assembly.extent,List.copyOf(assembly.models)));
            JakBlocks.BLOCKS.put(assembly.path,assembly.anchor);
        }
    }

    private static BlockPos position(JsonArray row){
        return new BlockPos(row.get(0).getAsInt(),row.get(1).getAsInt(),row.get(2).getAsInt());
    }

    private static final class Assembly {
        final String path;
        final BlockPos anchorOffset;
        final int extent;
        final List<Part> parts=new ArrayList<>();
        final List<PartModel> models=new ArrayList<>();
        Part anchor;
        Item item;

        Assembly(JsonObject source,BlockPos anchorOffset){
            this.path=source.get("path").getAsString();
            this.anchorOffset=anchorOffset;
            var size=position(source.getAsJsonArray("size"));
            this.extent=Math.max(size.getY(),Math.max(size.getX(),size.getZ()));
            boolean palm=source.get("group").getAsString().equals("palmera");
            var loot=ResourceKey.create(Registries.LOOT_TABLE,Identifier.fromNamespaceAndPath("jak","blocks/"+path));
            for(var row:source.getAsJsonArray("cells")){
                var cell=row.getAsJsonObject();
                var offset=position(cell.getAsJsonArray("cell"));
                var name=offset.equals(anchorOffset)?path:path+"/part_"+offset.getX()+"_"+offset.getY()+"_"+offset.getZ();
                var key=ResourceKey.create(Registries.BLOCK,Identifier.fromNamespaceAndPath("jak",name));
                var properties=BlockBehaviour.Properties.ofLegacyCopy(palm?Blocks.OAK_PLANKS:Blocks.TERRACOTTA)
                    .noOcclusion().setId(key).overrideLootTable(Optional.of(loot)).pushReaction(PushReaction.IMMOVEABLE);
                var part=new Part(properties,cell,this,offset,palm);
                Registry.register(BuiltInRegistries.BLOCK,key,part);
                parts.add(part);
                models.add(new PartModel(part,offset,cell.getAsJsonArray("triangles")));
                if(offset.equals(anchorOffset))anchor=part;
            }
            if(anchor==null)throw new IllegalStateException("Jak assembly has no base cell: "+path);
            var key=ResourceKey.create(Registries.ITEM,Identifier.fromNamespaceAndPath("jak",path));
            item=Registry.register(BuiltInRegistries.ITEM,key,
                new AssemblyItem(anchor,new Item.Properties().setId(key).useBlockDescriptionPrefix(),this));
        }
    }

    private static BlockPos rotatedOffset(BlockPos offset,Direction direction){
        return switch(direction){
            case EAST -> new BlockPos(-offset.getZ(),offset.getY(),offset.getX());
            case SOUTH -> new BlockPos(-offset.getX(),offset.getY(),-offset.getZ());
            case WEST -> new BlockPos(offset.getZ(),offset.getY(),-offset.getX());
            default -> offset;
        };
    }

    private static final class Part extends JakProps.PropBlock {
        final Assembly assembly;
        final BlockPos relative;
        final VoxelShape[] trunk;

        Part(Properties properties,JsonObject cell,Assembly assembly,BlockPos offset,boolean palm){
            super(properties,cell);
            this.assembly=assembly;
            relative=offset.subtract(assembly.anchorOffset);
            trunk=palm?gridShapes(trunkBoxes(cell)):null;
        }

        @Override public Item asItem(){
            return assembly==null||assembly.item==null?super.asItem():assembly.item;
        }

        @Override protected VoxelShape getCollisionShape(BlockState state,BlockGetter level,BlockPos at,CollisionContext context){
            return trunk==null?super.getCollisionShape(state,level,at,context):trunk[JakProps.turns(state.getValue(BlockStateProperties.HORIZONTAL_FACING))];
        }

        @Override protected BlockState updateShape(BlockState state,net.minecraft.world.level.LevelReader level,
            net.minecraft.world.level.ScheduledTickAccess ticks,BlockPos at,Direction changed,BlockPos neighbor,
            BlockState neighborState,net.minecraft.util.RandomSource random){
            if(relative.equals(BlockPos.ZERO)&&changed==Direction.DOWN
                &&!neighborState.isFaceSturdy(level,neighbor,Direction.UP))return Blocks.AIR.defaultBlockState();
            return state;
        }

        @Override protected void affectNeighborsAfterRemoval(BlockState state,ServerLevel level,BlockPos pos,boolean piston){
            Direction direction=state.getValue(BlockStateProperties.HORIZONTAL_FACING);
            BlockPos origin=pos.subtract(rotatedOffset(relative,direction));
            var removal=new Removed(level,origin,assembly);
            if(!REMOVING.add(removal))return;
            try{
                for(var part:assembly.parts){
                    BlockPos at=origin.offset(rotatedOffset(part.relative,direction));
                    if(at.equals(pos))continue;
                    BlockState present=level.getBlockState(at);
                    if(present.is(part)&&present.getValue(BlockStateProperties.HORIZONTAL_FACING)==direction)
                        level.setBlockAndUpdate(at,Blocks.AIR.defaultBlockState());
                }
            }finally{REMOVING.remove(removal);}
        }
    }

    private static JsonArray trunkBoxes(JsonObject cell){
        // Leaf cards stay selectable while only the authored wood blocks movement.
        double[] min={1,1,1},max={0,0,0};
        for(var row:cell.getAsJsonArray("triangles")){
            var triangle=row.getAsJsonObject();
            if(!triangle.get("texture").getAsString().contains("trunk"))continue;
            for(var raw:triangle.getAsJsonArray("p")){
                var point=raw.getAsJsonArray();
                for(int axis=0;axis<3;axis++){
                    double value=point.get(axis).getAsDouble();
                    min[axis]=Math.min(min[axis],value);
                    max[axis]=Math.max(max[axis],value);
                }
            }
        }
        var clipped=new JsonArray();
        for(var raw:cell.getAsJsonArray("boxes")){
            var box=raw.getAsJsonArray();
            double[] lower=new double[3],upper=new double[3];
            for(int axis=0;axis<3;axis++){
                lower[axis]=Math.max(min[axis],box.get(axis).getAsDouble());
                upper[axis]=Math.min(max[axis],box.get(axis+3).getAsDouble());
            }
            if(lower[0]>=upper[0]||lower[1]>=upper[1]||lower[2]>=upper[2])continue;
            var volume=new JsonArray();
            for(double n:lower)volume.add(n);
            for(double n:upper)volume.add(n);
            clipped.add(volume);
        }
        return clipped;
    }

    private static final class AssemblyItem extends BlockItem {
        final Assembly assembly;
        AssemblyItem(Block block,Properties properties,Assembly assembly){
            super(block,properties);
            this.assembly=assembly;
        }

        private Map<BlockPos,BlockState> placement(BlockPlaceContext context,BlockState state){
            var level=context.getLevel();
            var player=context.getPlayer();
            var origin=context.getClickedPos();
            var support=origin.below();
            if(!level.getBlockState(support).isFaceSturdy(level,support,Direction.UP))return Map.of();
            Direction direction=state.getValue(BlockStateProperties.HORIZONTAL_FACING);
            var planned=new LinkedHashMap<BlockPos,BlockState>();
            for(var part:assembly.parts){
                BlockPos at=origin.offset(rotatedOffset(part.relative,direction));
                BlockState target=part.defaultBlockState().setValue(BlockStateProperties.HORIZONTAL_FACING,direction);
                if(!level.isInWorldBounds(at)||!level.hasChunkAt(at)||player!=null&&!level.mayInteract(player,at)
                    ||!level.getBlockState(at).canBeReplaced()||!level.isUnobstructed(target,at,CollisionContext.placementContext(player)))return Map.of();
                planned.put(at,target);
            }
            return planned;
        }

        @Override protected boolean placeBlock(BlockPlaceContext context,BlockState state){
            var itemState=context.getItemInHand().getOrDefault(DataComponents.BLOCK_STATE,BlockItemStateProperties.EMPTY);
            var planned=placement(context,itemState.apply(state));
            if(planned.isEmpty())return false;
            Level level=context.getLevel();
            var previous=new LinkedHashMap<BlockPos,BlockState>();
            for(var cell:planned.entrySet()){
                previous.put(cell.getKey(),level.getBlockState(cell.getKey()));
                if(!level.setBlock(cell.getKey(),cell.getValue(),PLACE_FLAGS)){
                    previous.forEach((at,old)->level.setBlock(at,old,PLACE_FLAGS));
                    return false;
                }
            }
            for(var cell:planned.entrySet())level.updateNeighborsAt(cell.getKey(),cell.getValue().getBlock());
            return true;
        }
    }
}
