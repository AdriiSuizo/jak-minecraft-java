package dev.jakmc.content;

import com.google.gson.JsonArray;
import java.util.ArrayList;
import java.util.HashSet;
import java.util.LinkedHashMap;
import java.util.List;
import java.util.Set;
import net.minecraft.core.BlockPos;
import net.minecraft.core.Direction;
import net.minecraft.core.component.DataComponents;
import net.minecraft.server.level.ServerLevel;
import net.minecraft.sounds.SoundEvents;
import net.minecraft.sounds.SoundSource;
import net.minecraft.util.RandomSource;
import net.minecraft.world.InteractionResult;
import net.minecraft.world.entity.player.Player;
import net.minecraft.world.item.BlockItem;
import net.minecraft.world.item.component.BlockItemStateProperties;
import net.minecraft.world.item.context.BlockPlaceContext;
import net.minecraft.world.level.BlockGetter;
import net.minecraft.world.level.Level;
import net.minecraft.world.level.LevelReader;
import net.minecraft.world.level.ScheduledTickAccess;
import net.minecraft.world.level.block.Block;
import net.minecraft.world.level.block.Blocks;
import net.minecraft.world.level.block.HorizontalDirectionalBlock;
import net.minecraft.world.level.block.state.BlockState;
import net.minecraft.world.level.block.state.StateDefinition;
import net.minecraft.world.level.block.state.properties.BlockStateProperties;
import net.minecraft.world.level.block.state.properties.BooleanProperty;
import net.minecraft.world.level.block.state.properties.IntegerProperty;
import net.minecraft.world.level.gameevent.GameEvent;
import net.minecraft.world.level.redstone.Orientation;
import net.minecraft.world.phys.BlockHitResult;
import net.minecraft.world.phys.shapes.CollisionContext;
import net.minecraft.world.phys.shapes.VoxelShape;

/** A three-block steel gate rolls into a fourth cell and remains visible above the opening. */
public final class JakKrimsonGate extends HorizontalDirectionalBlock {
    private static final int HEIGHT=4;
    private static final int MAX_ADJACENT=8;
    private static final IntegerProperty SECTION=IntegerProperty.create("section",0,HEIGHT-1);
    private static final BooleanProperty OPEN=BlockStateProperties.OPEN;
    private static final BooleanProperty POWERED=BlockStateProperties.POWERED;
    private static final BooleanProperty IN_WALL=BlockStateProperties.IN_WALL;
    private static final BooleanProperty LEFT=BooleanProperty.create("left_jamb");
    private static final BooleanProperty RIGHT=BooleanProperty.create("right_jamb");
    private record Removing(ServerLevel level,BlockPos base){}
    private static final Set<Removing> REMOVING=new HashSet<>();
    private final VoxelShape[][][][] shapes=new VoxelShape[2][HEIGHT][4][4];

    JakKrimsonGate(Properties properties,JsonArray geometry){
        super(properties);
        registerDefaultState(stateDefinition.any().setValue(FACING,Direction.SOUTH).setValue(OPEN,false)
            .setValue(POWERED,false).setValue(IN_WALL,false).setValue(SECTION,0)
            .setValue(LEFT,true).setValue(RIGHT,true));
        for(int open=0;open<2;open++)for(int section=0;section<HEIGHT;section++){
            var boxes=geometry.get(open).getAsJsonArray().get(section).getAsJsonArray();
            for(int jambs=0;jambs<4;jambs++){
                var shaped=boxes.deepCopy();
                if(open!=0){
                    if((jambs&1)!=0)addJamb(shaped,0,section==3);
                    if((jambs&2)!=0)addJamb(shaped,15,section==3);
                }
                for(int turns=0;turns<4;turns++)shapes[open][section][jambs][turns]=JakAuthoredBlock.shape(shaped,turns);
            }
        }
    }

    private static void addJamb(JsonArray boxes,int x,boolean top){
        var box=new JsonArray();
        for(int coordinate:new int[]{x,0,top?0:5,x+1,16,11})box.add(coordinate);
        boxes.add(box);
    }

    @Override protected void createBlockStateDefinition(StateDefinition.Builder<Block,BlockState> builder){
        builder.add(FACING,OPEN,POWERED,IN_WALL,SECTION,LEFT,RIGHT);
    }

    @Override public BlockState getStateForPlacement(BlockPlaceContext context){
        return defaultBlockState().setValue(FACING,context.getHorizontalDirection().getOpposite());
    }

    private VoxelShape shape(BlockState state){
        int turns=(JakProps.turns(state.getValue(FACING))+2)%4;
        int jambs=(state.getValue(LEFT)?1:0)|(state.getValue(RIGHT)?2:0);
        return shapes[state.getValue(OPEN)?1:0][state.getValue(SECTION)][jambs][turns];
    }

    @Override protected VoxelShape getShape(BlockState state,BlockGetter level,BlockPos pos,CollisionContext context){
        return shape(state);
    }

    @Override protected VoxelShape getCollisionShape(BlockState state,BlockGetter level,BlockPos pos,CollisionContext context){
        return shape(state);
    }

    @Override protected InteractionResult useWithoutItem(BlockState state,Level level,BlockPos pos,Player player,BlockHitResult hit){
        if(level instanceof ServerLevel server){
            var columns=connected(server,pos,state);
            if(!change(server,columns,!state.getValue(OPEN),null,player))return InteractionResult.FAIL;
        }
        return InteractionResult.SUCCESS;
    }

    private List<BlockPos> connected(ServerLevel level,BlockPos pos,BlockState state){
        BlockPos base=pos.below(state.getValue(SECTION));
        Direction facing=state.getValue(FACING);
        var columns=new ArrayList<BlockPos>();
        if(!intact(level,base,facing))return columns;
        columns.add(base);
        Direction lateral=facing.getClockWise();
        for(Direction side:new Direction[]{lateral,lateral.getOpposite()}){
            for(int distance=1;distance<=MAX_ADJACENT;distance++){
                var at=base.relative(side,distance);
                if(!intact(level,at,facing))break;
                columns.add(at);
            }
        }
        return columns;
    }

    private boolean intact(ServerLevel level,BlockPos base,Direction facing){
        if(!level.hasChunkAt(base))return false;
        for(int i=0;i<HEIGHT;i++){
            BlockState state=level.getBlockState(base.above(i));
            if(!state.is(this)||state.getValue(SECTION)!=i||state.getValue(FACING)!=facing)return false;
        }
        return true;
    }

    private boolean change(ServerLevel level,List<BlockPos> columns,boolean open,Boolean powered,Player player){
        if(columns.isEmpty())return false;
        var changes=new LinkedHashMap<BlockPos,BlockState>();
        for(var base:columns)for(int i=0;i<HEIGHT;i++){
            var at=base.above(i);
            if(player!=null&&!level.mayInteract(player,at))return false;
            BlockState old=level.getBlockState(at);
            BlockState next=old.setValue(OPEN,open);
            if(powered!=null)next=next.setValue(POWERED,powered);
            if(old==next)continue;
            if(old.getValue(OPEN)!=open&&!level.isUnobstructed(next,at,CollisionContext.empty()))return false;
            changes.put(at,next);
        }
        if(changes.isEmpty())return true;
        changes.forEach((at,state)->level.setBlock(at,state,Block.UPDATE_CLIENTS|Block.UPDATE_KNOWN_SHAPE));
        var base=columns.getFirst();
        level.playSound(null,base,open?SoundEvents.IRON_DOOR_OPEN:SoundEvents.IRON_DOOR_CLOSE,SoundSource.BLOCKS,1,1);
        level.gameEvent(player,open?GameEvent.BLOCK_OPEN:GameEvent.BLOCK_CLOSE,base);
        return true;
    }

    private BlockState frame(BlockGetter level,BlockPos pos,BlockState state){
        Direction left=state.getValue(FACING).getClockWise();
        return state.setValue(LEFT,!adjacent(level.getBlockState(pos.relative(left)),state))
            .setValue(RIGHT,!adjacent(level.getBlockState(pos.relative(left.getOpposite())),state));
    }

    private boolean adjacent(BlockState other,BlockState state){
        return other.is(this)&&other.getValue(FACING)==state.getValue(FACING)
            &&other.getValue(SECTION).equals(state.getValue(SECTION));
    }

    @Override protected BlockState updateShape(BlockState state,LevelReader level,ScheduledTickAccess ticks,
        BlockPos at,Direction changed,BlockPos neighbor,BlockState neighborState,RandomSource random){
        if(state.getValue(SECTION)==0&&changed==Direction.DOWN
            &&!neighborState.isFaceSturdy(level,neighbor,Direction.UP))return Blocks.AIR.defaultBlockState();
        return frame(level,at,state);
    }

    @Override protected void neighborChanged(BlockState state,Level level,BlockPos pos,Block neighbor,Orientation orientation,boolean moved){
        if(level instanceof ServerLevel server){
            var corrected=frame(server,pos,state);
            if(corrected!=state)server.setBlock(pos,corrected,Block.UPDATE_CLIENTS);
            updatePower(server,pos,corrected);
        }
    }

    @Override protected void tick(BlockState state,ServerLevel level,BlockPos pos,RandomSource random){
        updatePower(level,pos,state);
    }

    private void updatePower(ServerLevel level,BlockPos at,BlockState state){
        var columns=connected(level,at,state);
        if(columns.isEmpty())return;
        boolean powered=columns.stream().anyMatch(base->{
            for(int i=0;i<HEIGHT;i++)if(level.hasNeighborSignal(base.above(i)))return true;
            return false;
        });
        if(powered==state.getValue(POWERED))return;
        if(!change(level,columns,powered,powered,null))level.scheduleTick(at,this,5);
    }

    @Override protected void affectNeighborsAfterRemoval(BlockState state,ServerLevel level,BlockPos at,boolean piston){
        var base=at.below(state.getValue(SECTION));
        var removal=new Removing(level,base);
        if(!REMOVING.add(removal))return;
        try{
            for(int i=0;i<HEIGHT;i++){
                var part=base.above(i);
                if(part.equals(at))continue;
                BlockState other=level.getBlockState(part);
                if(other.is(this)&&other.getValue(SECTION)==i&&other.getValue(FACING)==state.getValue(FACING))
                    level.setBlockAndUpdate(part,Blocks.AIR.defaultBlockState());
            }
        }finally{REMOVING.remove(removal);}
    }

    static final class GateItem extends BlockItem {
        GateItem(JakKrimsonGate gate,Properties properties){super(gate,properties);}

        @Override protected boolean placeBlock(BlockPlaceContext context,BlockState state){
            var level=context.getLevel();
            var player=context.getPlayer();
            var base=context.getClickedPos();
            if(!level.getBlockState(base.below()).isFaceSturdy(level,base.below(),Direction.UP))return false;
            var itemState=context.getItemInHand().getOrDefault(DataComponents.BLOCK_STATE,BlockItemStateProperties.EMPTY);
            state=itemState.apply(state);
            if(state.getValue(SECTION)!=0)return false;
            var planned=new LinkedHashMap<BlockPos,BlockState>();
            var gate=(JakKrimsonGate)getBlock();
            for(int i=0;i<HEIGHT;i++){
                var at=base.above(i);
                var part=gate.frame(level,at,state.setValue(SECTION,i));
                if(!level.isInWorldBounds(at)||!level.hasChunkAt(at)||player!=null&&!level.mayInteract(player,at)
                    ||!level.getBlockState(at).canBeReplaced()||!level.isUnobstructed(part,at,CollisionContext.placementContext(player)))
                    return false;
                planned.put(at,part);
            }
            return JakAssemblies.placeAll(level,planned);
        }
    }
}
