package dev.jakmc.content;

import com.google.gson.JsonArray;
import com.google.gson.JsonObject;
import java.util.LinkedHashMap;
import java.util.Map;
import java.util.Optional;
import net.minecraft.core.BlockPos;
import net.minecraft.core.Direction;
import net.minecraft.core.Registry;
import net.minecraft.core.registries.BuiltInRegistries;
import net.minecraft.core.registries.Registries;
import net.minecraft.resources.Identifier;
import net.minecraft.resources.ResourceKey;
import net.minecraft.world.item.BlockItem;
import net.minecraft.world.item.Item;
import net.minecraft.world.item.context.BlockPlaceContext;
import net.minecraft.world.level.BlockGetter;
import net.minecraft.world.level.block.Block;
import net.minecraft.world.level.block.Blocks;
import net.minecraft.world.level.block.HorizontalDirectionalBlock;
import net.minecraft.world.level.block.state.BlockBehaviour;
import net.minecraft.world.level.block.state.BlockState;
import net.minecraft.world.level.block.state.StateDefinition;
import net.minecraft.world.phys.shapes.BitSetDiscreteVoxelShape;
import net.minecraft.world.phys.shapes.CollisionContext;
import net.minecraft.world.phys.shapes.CubeVoxelShape;
import net.minecraft.world.phys.shapes.VoxelShape;

/** One-cell decorative meshes. Both the outline and collision use the source cell shapes. */
public final class JakProps {
    public record Model(int id,String path,Block block,JsonArray triangles){}
    public static final Map<Integer,Model> MODELS=new LinkedHashMap<>();

    public static void initialize(){
        for(var id:JakBlocks.resource("/data/jakmc/catalog/decorations.json").getAsJsonArray()){
            int index=id.getAsInt();
            var source=JakBlocks.resource("/data/jakmc/catalog/objects/"+index+".json").getAsJsonObject();
            String path=source.get("path").getAsString();
            var cell=source.getAsJsonArray("cells").get(0).getAsJsonObject();
            var name=Identifier.fromNamespaceAndPath("jak",path);
            var blockKey=ResourceKey.create(Registries.BLOCK,name);
            var loot=ResourceKey.create(Registries.LOOT_TABLE,name.withPrefix("blocks/"));
            var properties=BlockBehaviour.Properties.ofLegacyCopy(Blocks.TERRACOTTA).noOcclusion()
                .setId(blockKey).overrideLootTable(Optional.of(loot));
            var block=new PropBlock(properties,cell);
            Registry.register(BuiltInRegistries.BLOCK,blockKey,block);
            var itemKey=ResourceKey.create(Registries.ITEM,name);
            Registry.register(BuiltInRegistries.ITEM,itemKey,new BlockItem(block,new Item.Properties().setId(itemKey).useBlockDescriptionPrefix()));
            MODELS.put(index,new Model(index,path,block,cell.getAsJsonArray("triangles")));
            JakBlocks.BLOCKS.put(path,block);
        }
    }

    public static int turns(Direction direction){
        return switch(direction){
            case EAST -> 1;
            case SOUTH -> 2;
            case WEST -> 3;
            default -> 0;
        };
    }

    private static final class PropBlock extends HorizontalDirectionalBlock {
        private final VoxelShape[] shapes=new VoxelShape[4];
        PropBlock(Properties properties,JsonObject cell){
            super(properties);
            registerDefaultState(stateDefinition.any().setValue(FACING,Direction.NORTH));
            // Rasterize source collision onto Minecraft's 1/16 grid once. Keeping every
            // floating-point subdivision makes vanilla's boolean shape union explode in size.
            var voxels=new BitSetDiscreteVoxelShape[4];
            for(int i=0;i<4;i++)voxels[i]=new BitSetDiscreteVoxelShape(16,16,16);
            for(var row:cell.getAsJsonArray("boxes")){
                var box=row.getAsJsonArray();
                int minX=edge(box,0,false),minY=edge(box,1,false),minZ=edge(box,2,false);
                int maxX=edge(box,3,true),maxY=edge(box,4,true),maxZ=edge(box,5,true);
                for(int x=minX;x<maxX;x++)for(int y=minY;y<maxY;y++)for(int z=minZ;z<maxZ;z++){
                    voxels[0].fill(x,y,z);
                    voxels[1].fill(15-z,y,x);
                    voxels[2].fill(15-x,y,15-z);
                    voxels[3].fill(z,y,15-x);
                }
            }
            for(int i=0;i<4;i++)shapes[i]=new CubeVoxelShape(voxels[i]);
        }
        private static int edge(JsonArray values,int index,boolean upper){
            double scaled=Math.clamp(values.get(index).getAsDouble(),0,1)*16;
            return (int)(upper?Math.ceil(scaled-1e-5):Math.floor(scaled+1e-5));
        }
        @Override protected void createBlockStateDefinition(StateDefinition.Builder<Block,BlockState> builder){builder.add(FACING);}
        @Override public BlockState getStateForPlacement(BlockPlaceContext context){
            return defaultBlockState().setValue(FACING,context.getHorizontalDirection().getOpposite());
        }
        @Override protected VoxelShape getShape(BlockState state,BlockGetter level,BlockPos pos,CollisionContext context){
            return shapes[turns(state.getValue(FACING))];
        }
        @Override protected VoxelShape getCollisionShape(BlockState state,BlockGetter level,BlockPos pos,CollisionContext context){
            return getShape(state,level,pos,context);
        }
    }
}
