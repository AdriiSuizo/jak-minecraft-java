package dev.jakmc.content;

import com.google.gson.JsonArray;
import com.google.gson.JsonObject;
import java.nio.ByteBuffer;
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
import net.minecraft.world.level.block.EntityBlock;
import net.minecraft.world.level.block.entity.BlockEntity;
import net.minecraft.world.level.block.entity.BlockEntityType;
import net.minecraft.world.phys.Vec3;
import net.minecraft.world.level.block.state.BlockBehaviour;
import net.minecraft.world.level.block.state.BlockState;
import net.minecraft.world.level.block.state.properties.BlockStateProperties;
import net.minecraft.world.level.material.PushReaction;
import net.minecraft.world.phys.shapes.CollisionContext;
import net.minecraft.world.phys.shapes.VoxelShape;

/** Native grid assemblies use one inventory item and an owned, bounded set of Minecraft block states. */
public final class JakAssemblies {
    public record PartModel(Block block,BlockPos offset,JsonArray triangles){}
    public record Model(int id,String path,int extent,BlockPos size,BlockPos anchor,List<PartModel> parts){}
    public static final Map<Integer,Model> MODELS=new LinkedHashMap<>();
    public static final Map<String,Model> FACADES=new LinkedHashMap<>();

    private record Removed(ServerLevel level,BlockPos origin,Assembly assembly){}
    private static final Set<Removed> REMOVING=new HashSet<>();
    private static BlockEntityType<RailAnchor> railType;
    private static final int PLACE_FLAGS=Block.UPDATE_CLIENTS|Block.UPDATE_KNOWN_SHAPE;

    public static void initialize(){
        var railBlocks=new HashSet<Block>();
        // loose_objects: jak_objects.rs placed, the level's loose things standing on the ground (plants included).
        for(var list:new String[]{"assemblies","loose_objects"})
        for(var entry:JakBlocks.resource("/data/jakmc/catalog/"+list+".json").getAsJsonObject().entrySet()){
            int id=Integer.parseInt(entry.getKey());
            var source=JakBlocks.resource("/data/jakmc/catalog/objects/"+id+".json").getAsJsonObject();
            var assembly=new Assembly(source,position(entry.getValue().getAsJsonArray()),list.equals("loose_objects"));
            var model=new Model(id,assembly.path,assembly.extent,assembly.size,assembly.anchorOffset,List.copyOf(assembly.models));
            MODELS.put(id,model);
            if(source.get("group").getAsString().equals("facade"))FACADES.put(assembly.path,model);
            JakBlocks.BLOCKS.put(assembly.path,assembly.anchor);
            if(assembly.rail)railBlocks.add(assembly.anchor);
        }
        railType=Registry.register(BuiltInRegistries.BLOCK_ENTITY_TYPE,Identifier.fromNamespaceAndPath("jak","grind_rail"),
            new BlockEntityType<>(RailAnchor::new,railBlocks));
    }

    private static BlockPos position(JsonArray row){
        return new BlockPos(row.get(0).getAsInt(),row.get(1).getAsInt(),row.get(2).getAsInt());
    }

    private static final class Assembly {
        final String path;
        final BlockPos anchorOffset;
        final int extent;
        final BlockPos size;
        final boolean wallMounted,soft;
        final boolean track,rail;
        final List<Part> parts=new ArrayList<>();
        final List<PartModel> models=new ArrayList<>();
        final List<List<Vec3>> rails;
        Part anchor;
        Item item;

        Assembly(JsonObject source,BlockPos anchorOffset,boolean ground){
            this.path=source.get("path").getAsString();
            this.anchorOffset=anchorOffset;
            size=position(source.getAsJsonArray("size"));
            this.extent=Math.max(size.getY(),Math.max(size.getX(),size.getZ()));
            boolean palm=source.get("group").getAsString().equals("palmera");
            soft=source.get("soft").getAsBoolean();
            wallMounted=!ground&&(soft||source.has("wallMounted")&&source.get("wallMounted").getAsBoolean());
            String group=source.get("group").getAsString();
            rail=group.equals("rail");
            track=rail||group.equals("track");
            var railLines=new ArrayList<List<Vec3>>();
            if(rail)for(var line:source.getAsJsonArray("rails")){
                var points=new ArrayList<Vec3>();
                for(var point:line.getAsJsonArray()){
                    var coordinates=point.getAsJsonArray();
                    points.add(new Vec3(coordinates.get(0).getAsDouble(),coordinates.get(1).getAsDouble(),
                        coordinates.get(2).getAsDouble()));
                }
                railLines.add(List.copyOf(points));
            }
            rails=List.copyOf(railLines);
            var loot=ResourceKey.create(Registries.LOOT_TABLE,Identifier.fromNamespaceAndPath("jak","blocks/"+path));
            for(var row:source.getAsJsonArray("cells")){
                var cell=row.getAsJsonObject();
                var offset=position(cell.getAsJsonArray("cell"));
                var name=offset.equals(anchorOffset)?path:path+"/part_"+offset.getX()+"_"+offset.getY()+"_"+offset.getZ();
                var key=ResourceKey.create(Registries.BLOCK,Identifier.fromNamespaceAndPath("jak",name));
                var properties=BlockBehaviour.Properties.ofLegacyCopy(palm||wallMounted?Blocks.OAK_PLANKS:Blocks.TERRACOTTA)
                    .noOcclusion().setId(key).overrideLootTable(Optional.of(loot)).pushReaction(PushReaction.IMMOVEABLE);
                Part part=rail&&offset.equals(anchorOffset)?new RailPart(properties,cell,this,offset):
                    new Part(properties,cell,this,offset,palm);
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
        Direction support(Direction facing){return !wallMounted?Direction.DOWN:soft?facing:facing.getOpposite();}
    }

    private static BlockPos rotatedOffset(BlockPos offset,Direction direction){
        return switch(direction){
            case EAST -> new BlockPos(-offset.getZ(),offset.getY(),offset.getX());
            case SOUTH -> new BlockPos(-offset.getX(),offset.getY(),-offset.getZ());
            case WEST -> new BlockPos(offset.getZ(),offset.getY(),-offset.getX());
            default -> offset;
        };
    }

    private static class Part extends JakProps.PropBlock {
        final Assembly assembly;
        final BlockPos relative;
        final VoxelShape[] trunk;
        final TrackMesh[] trackMeshes;

        Part(Properties properties,JsonObject cell,Assembly assembly,BlockPos offset,boolean palm){
            super(properties,cell);
            this.assembly=assembly;
            relative=offset.subtract(assembly.anchorOffset);
            trunk=palm?gridShapes(trunkBoxes(cell)):null;
            if(assembly.track){
                trackMeshes=new TrackMesh[4];
                for(int turns=0;turns<4;turns++)trackMeshes[turns]=new TrackMesh(cell.getAsJsonArray("triangles"),turns,assembly.rail);
            }else trackMeshes=null;
        }

        @Override public BlockState getStateForPlacement(BlockPlaceContext context){
            if(!assembly.wallMounted)return super.getStateForPlacement(context);
            var face=context.getClickedFace();
            return face.getAxis().isHorizontal()?defaultBlockState().setValue(FACING,assembly.soft?face.getOpposite():face):null;
        }

        @Override public Item asItem(){
            return assembly==null||assembly.item==null?super.asItem():assembly.item;
        }

        @Override protected VoxelShape getCollisionShape(BlockState state,BlockGetter level,BlockPos at,CollisionContext context){
            if(assembly!=null&&assembly.soft)return net.minecraft.world.phys.shapes.Shapes.empty();
            return trunk==null?super.getCollisionShape(state,level,at,context):trunk[JakProps.turns(state.getValue(BlockStateProperties.HORIZONTAL_FACING))];
        }

        @Override protected BlockState updateShape(BlockState state,net.minecraft.world.level.LevelReader level,
            net.minecraft.world.level.ScheduledTickAccess ticks,BlockPos at,Direction changed,BlockPos neighbor,
            BlockState neighborState,net.minecraft.util.RandomSource random){
            Direction support=assembly.support(state.getValue(FACING));
            if(relative.equals(BlockPos.ZERO)&&changed==support
                &&!neighborState.isFaceSturdy(level,neighbor,support.getOpposite()))return Blocks.AIR.defaultBlockState();
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

    private static final class RailPart extends Part implements EntityBlock {
        RailPart(Properties properties,JsonObject cell,Assembly assembly,BlockPos offset){
            super(properties,cell,assembly,offset,false);
        }
        @Override public BlockEntity newBlockEntity(BlockPos at,BlockState state){return new RailAnchor(at,state);}
    }

    public static final class RailAnchor extends BlockEntity {
        private Direction previous;
        private List<List<Vec3>> worldRails=List.of();
        RailAnchor(BlockPos at,BlockState state){super(railType,at,state);}
        public List<List<Vec3>> lines(){
            Direction facing=getBlockState().getValue(BlockStateProperties.HORIZONTAL_FACING);
            if(facing!=previous){
                previous=facing;
                var part=(RailPart)getBlockState().getBlock();
                int turns=JakProps.turns(facing);
                Vec3 relative=Vec3.atBottomCenterOf(part.assembly.anchorOffset);
                Vec3 center=Vec3.atBottomCenterOf(getBlockPos());
                worldRails=part.assembly.rails.stream().map(line->line.stream().map(source->{
                    Vec3 point=source.subtract(relative);
                    for(int i=0;i<turns;i++)point=new Vec3(-point.z,point.y,point.x);
                    return point.add(center);
                }).toList()).toList();
            }
            return worldRails;
        }
    }

    public static TrackMesh trackMesh(BlockState state){
        if(!(state.getBlock() instanceof Part part)||part.trackMeshes==null)return null;
        return part.trackMeshes[JakProps.turns(state.getValue(BlockStateProperties.HORIZONTAL_FACING))];
    }

    /** These source triangles are already clipped to their owner cell and baked for each rotation. */
    public static final class TrackMesh {
        private final float[] xyz;
        private final byte[] flags;
        private final Direction[] seams;

        TrackMesh(JsonArray triangles,int turns,boolean rail){
            xyz=new float[triangles.size()*9];
            flags=new byte[triangles.size()];
            seams=new Direction[triangles.size()];
            for(int i=0;i<triangles.size();i++){
                var points=triangles.get(i).getAsJsonObject().getAsJsonArray("p");
                for(int j=0;j<3;j++){
                    var point=points.get(j).getAsJsonArray();
                    float x=point.get(0).getAsFloat(),z=point.get(2).getAsFloat();
                    for(int turn=0;turn<turns;turn++){float nextX=1-z;z=x;x=nextX;}
                    int index=i*9+j*3;
                    xyz[index]=x;xyz[index+1]=point.get(1).getAsFloat();xyz[index+2]=z;
                }
                int index=i*9;
                float abX=xyz[index+3]-xyz[index],abZ=xyz[index+5]-xyz[index+2];
                float acX=xyz[index+6]-xyz[index],acZ=xyz[index+8]-xyz[index+2];
                flags[i]=(byte)(abZ*acX-abX*acZ>1e-7?(rail?16:3):1);
                for(Direction direction:Direction.values()){
                    int axis=switch(direction.getAxis()){case X->0;case Y->1;case Z->2;};
                    float plane=direction.getAxisDirection()==Direction.AxisDirection.POSITIVE?1:0;
                    if(Math.abs(xyz[index+axis]-plane)<1e-5&&Math.abs(xyz[index+axis+3]-plane)<1e-5
                        &&Math.abs(xyz[index+axis+6]-plane)<1e-5){seams[i]=direction;break;}
                }
            }
        }

        public int capacity(){return flags.length*40;}

        public void write(ByteBuffer out,Level level,BlockPos at,BlockState state,double x,double y,double z){
            if(flags.length==0)return;
            Direction facing=state.getValue(BlockStateProperties.HORIZONTAL_FACING);
            Direction positive=facing.getClockWise(),negative=positive.getOpposite();
            boolean sharesPositive=matches(level,at.relative(positive),state);
            boolean sharesNegative=matches(level,at.relative(negative),state);
            for(int i=0;i<flags.length;i++){
                Direction seam=seams[i];
                // Identical parts placed side by side share the same profile.
                // Its exposed side is retained if the neighbouring part is missing.
                if(seam==positive&&sharesPositive||seam==negative&&sharesNegative)continue;
                int index=i*9;
                for(int j=0;j<9;j+=3)
                    out.putFloat((float)(x+xyz[index+j])).putFloat((float)(y+xyz[index+j+1]))
                        .putFloat((float)(z+xyz[index+j+2]));
                out.putInt(flags[i]);
            }
        }

        private boolean matches(Level level,BlockPos at,BlockState state){
            return level.hasChunkAt(at)&&level.getBlockState(at)==state;
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
            Direction direction=state.getValue(BlockStateProperties.HORIZONTAL_FACING);
            Direction attachedTo=assembly.support(direction);
            var support=origin.relative(attachedTo);
            if(!level.getBlockState(support).isFaceSturdy(level,support,attachedTo.getOpposite()))return Map.of();
            var planned=new LinkedHashMap<BlockPos,BlockState>();
            for(var part:assembly.parts){
                BlockPos at=origin.offset(rotatedOffset(part.relative,direction));
                BlockState target=part.defaultBlockState().setValue(BlockStateProperties.HORIZONTAL_FACING,direction);
                if(!level.isInWorldBounds(at)||!level.hasChunkAt(at)||player!=null&&!level.mayInteract(player,at)
                    ||!level.getBlockState(at).canBeReplaced()||!level.isUnobstructed(target,at,CollisionContext.placementContext(player)))return Map.of();
                if(assembly.track&&part.relative.getY()==0){
                    var below=at.below();
                    if(!level.getBlockState(below).isFaceSturdy(level,below,Direction.UP))return Map.of();
                }
                planned.put(at,target);
            }
            return planned;
        }

        @Override protected boolean placeBlock(BlockPlaceContext context,BlockState state){
            var itemState=context.getItemInHand().getOrDefault(DataComponents.BLOCK_STATE,BlockItemStateProperties.EMPTY);
            var planned=placement(context,itemState.apply(state));
            if(planned.isEmpty())return false;
            return placeAll(context.getLevel(),planned);
        }
    }

    static boolean placeAll(Level level,Map<BlockPos,BlockState> planned){
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
