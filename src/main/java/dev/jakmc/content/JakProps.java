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
import net.minecraft.server.level.ServerLevel;
import net.minecraft.world.entity.player.Player;
import net.minecraft.world.level.Level;
import net.minecraft.world.level.block.state.properties.IntegerProperty;
import net.fabricmc.fabric.api.networking.v1.PlayerLookup;
import net.fabricmc.fabric.api.networking.v1.ServerPlayNetworking;
import dev.jakmc.network.JakPackets;
import net.minecraft.world.item.BlockItem;
import net.minecraft.world.item.Item;
import net.minecraft.world.item.context.BlockPlaceContext;
import net.minecraft.world.level.BlockGetter;
import net.minecraft.world.level.block.Block;
import net.minecraft.world.level.block.Blocks;
import net.minecraft.world.level.block.HorizontalDirectionalBlock;
import net.minecraft.world.level.block.EntityBlock;
import net.minecraft.world.level.block.entity.BlockEntity;
import net.minecraft.world.level.block.entity.BlockEntityType;
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
    public static final int FIRE_BARREL=94;
    public static final int PROJECTOR=86;
    public static final IntegerProperty HITS=IntegerProperty.create("hits",0,2);
    private static BlockEntityType<Projector> projectorType;
    private static BlockEntityType<FireBarrel> fireBarrelType;
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
            Block block=switch(index){
                case FIRE_BARREL -> new BarrelBlock(properties.lightLevel(_ ->12),cell);
                case PROJECTOR -> new ProjectorBlock(properties.lightLevel(_ ->7),cell);
                default -> new PropBlock(properties,cell);
            };
            Registry.register(BuiltInRegistries.BLOCK,blockKey,block);
            var itemKey=ResourceKey.create(Registries.ITEM,name);
            Registry.register(BuiltInRegistries.ITEM,itemKey,new BlockItem(block,new Item.Properties().setId(itemKey).useBlockDescriptionPrefix()));
            MODELS.put(index,new Model(index,path,block,cell.getAsJsonArray("triangles")));
            JakBlocks.BLOCKS.put(path,block);
        }
        var barrel=MODELS.get(FIRE_BARREL).block();
        fireBarrelType=Registry.register(BuiltInRegistries.BLOCK_ENTITY_TYPE,
            Identifier.fromNamespaceAndPath("jak","burning_barrel"),
            new BlockEntityType<>(FireBarrel::new,java.util.Set.of(barrel)));
        projectorType=Registry.register(BuiltInRegistries.BLOCK_ENTITY_TYPE,
            Identifier.fromNamespaceAndPath("jak","praxis_projector"),
            new BlockEntityType<>(Projector::new,java.util.Set.of(MODELS.get(PROJECTOR).block())));
    }

    public static int turns(Direction direction){
        return switch(direction){
            case EAST -> 1;
            case SOUTH -> 2;
            case WEST -> 3;
            default -> 0;
        };
    }

    public static final class FireBarrel extends BlockEntity {
        FireBarrel(BlockPos pos,BlockState state){super(fireBarrelType,pos,state);}
    }

    public static final class Projector extends BlockEntity {
        long lastHit=Long.MIN_VALUE;
        Projector(BlockPos pos,BlockState state){super(projectorType,pos,state);}
    }

    public static boolean hitProjector(ServerLevel level,BlockPos pos,Player player){
        var state=level.getBlockState(pos);
        if(!(state.getBlock() instanceof ProjectorBlock)||!(level.getBlockEntity(pos) instanceof Projector projector))return false;
        if(player.isSpectator()||!level.mayInteract(player,pos)||projector.lastHit==level.getGameTime())return true;
        projector.lastHit=level.getGameTime();
        int hits=state.getValue(HITS);
        if(hits<2){
            level.setBlockAndUpdate(pos,state.setValue(HITS,hits+1));
            level.levelEvent(null,2001,pos,Block.getId(state));
        }else{
            if(!level.destroyBlock(pos,false,player))return true;
            if(!player.hasInfiniteMaterials())Block.popResource(level,pos,new net.minecraft.world.item.ItemStack(state.getBlock()));
            var cue=new JakPackets.PropBreak(level.dimension().identifier(),pos);
            for(var watcher:PlayerLookup.tracking(level,pos))
                if(ServerPlayNetworking.canSend(watcher,JakPackets.PropBreak.TYPE))ServerPlayNetworking.send(watcher,cue);
        }
        return true;
    }

    private static final class ProjectorBlock extends PropBlock implements EntityBlock {
        ProjectorBlock(Properties properties,JsonObject cell){super(properties,cell);}
        @Override protected void createBlockStateDefinition(StateDefinition.Builder<Block,BlockState> builder){
            super.createBlockStateDefinition(builder);
            builder.add(HITS);
        }
        @Override protected void attack(BlockState state,Level level,BlockPos pos,Player player){
            if(level instanceof ServerLevel server)hitProjector(server,pos,player);
        }
        @Override protected float getDestroyProgress(BlockState state,Player player,BlockGetter level,BlockPos pos){
            return player.hasInfiniteMaterials()?1:0;
        }
        @Override public BlockEntity newBlockEntity(BlockPos pos,BlockState state){return new Projector(pos,state);}
    }

    private static final class BarrelBlock extends PropBlock implements EntityBlock {
        BarrelBlock(Properties properties,JsonObject cell){super(properties,cell);}
        @Override public BlockEntity newBlockEntity(BlockPos pos,BlockState state){return new FireBarrel(pos,state);}
    }

    static class PropBlock extends HorizontalDirectionalBlock {
        private final VoxelShape[] shapes;
        PropBlock(Properties properties,JsonObject cell){
            super(properties);
            registerDefaultState(stateDefinition.any().setValue(FACING,Direction.NORTH));
            shapes=gridShapes(cell.getAsJsonArray("boxes"));
        }
        static VoxelShape[] gridShapes(JsonArray boxes){
            // Rasterize source collision onto Minecraft's 1/16 grid once. Keeping every
            // floating-point subdivision makes vanilla's boolean shape union explode in size.
            var voxels=new BitSetDiscreteVoxelShape[4];
            for(int i=0;i<4;i++)voxels[i]=new BitSetDiscreteVoxelShape(16,16,16);
            for(var row:boxes){
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
            var shapes=new VoxelShape[4];
            for(int i=0;i<4;i++)shapes[i]=new CubeVoxelShape(voxels[i]);
            return shapes;
        }
        private static int edge(JsonArray values,int index,boolean upper){
            double scaled=Math.clamp(values.get(index).getAsDouble(),0,1)*16;
            return (int)(upper?Math.ceil(scaled-1e-5):Math.floor(scaled+1e-5));
        }
        @Override protected void createBlockStateDefinition(StateDefinition.Builder<Block,BlockState> builder){builder.add(FACING);}
        @Override public BlockState getStateForPlacement(BlockPlaceContext context){
            // The source meshes face +Z (south) at no turns, so the way the player looks turns them to face him
            // (Adrii 06/11: the fish trophy faced away).
            return defaultBlockState().setValue(FACING,context.getHorizontalDirection());
        }
        @Override protected VoxelShape getShape(BlockState state,BlockGetter level,BlockPos pos,CollisionContext context){
            return shapes[turns(state.getValue(FACING))];
        }
        @Override protected VoxelShape getCollisionShape(BlockState state,BlockGetter level,BlockPos pos,CollisionContext context){
            return getShape(state,level,pos,context);
        }
    }
}
