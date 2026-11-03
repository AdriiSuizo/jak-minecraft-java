package dev.jakmc.content;

import net.minecraft.core.BlockPos;
import net.minecraft.core.Direction;
import net.minecraft.world.item.context.BlockPlaceContext;
import net.minecraft.world.level.BlockGetter;
import net.minecraft.world.level.block.Block;
import net.minecraft.world.level.block.state.BlockState;
import net.minecraft.world.level.block.state.StateDefinition;
import net.minecraft.world.level.block.state.properties.BlockStateProperties;
import net.minecraft.world.phys.shapes.CollisionContext;
import net.minecraft.world.phys.shapes.VoxelShape;

/** Stone and metal terrain courses with matching visual and physical heights. */
final class JakSolidLayers extends Block {
    private static final VoxelShape[] SHAPES=Block.boxes(8,height->Block.column(16,0,height*2));
    JakSolidLayers(Properties properties){
        super(properties);
        registerDefaultState(stateDefinition.any().setValue(BlockStateProperties.LAYERS,1));
    }

    @Override protected void createBlockStateDefinition(StateDefinition.Builder<Block,BlockState> builder){
        builder.add(BlockStateProperties.LAYERS);
    }
    @Override protected VoxelShape getShape(BlockState state,BlockGetter level,BlockPos pos,CollisionContext context){
        return SHAPES[state.getValue(BlockStateProperties.LAYERS)];
    }
    @Override protected VoxelShape getCollisionShape(BlockState state,BlockGetter level,BlockPos pos,CollisionContext context){
        return getShape(state,level,pos,context);
    }
    @Override protected boolean useShapeForLightOcclusion(BlockState state){return true;}
    @Override protected boolean canBeReplaced(BlockState state,BlockPlaceContext context){
        return context.getItemInHand().is(asItem())&&state.getValue(BlockStateProperties.LAYERS)<8
            &&(!context.replacingClickedOnBlock()||context.getClickedFace()==Direction.UP);
    }
    @Override public BlockState getStateForPlacement(BlockPlaceContext context){
        var state=context.getLevel().getBlockState(context.getClickedPos());
        return state.is(this)?state.setValue(BlockStateProperties.LAYERS,Math.min(8,state.getValue(BlockStateProperties.LAYERS)+1))
            :defaultBlockState();
    }
}
