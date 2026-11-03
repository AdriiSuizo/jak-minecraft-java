package dev.jakmc.content;

import com.google.gson.JsonArray;
import net.minecraft.core.BlockPos;
import net.minecraft.core.Direction;
import net.minecraft.world.item.context.BlockPlaceContext;
import net.minecraft.world.level.BlockGetter;
import net.minecraft.world.level.block.Block;
import net.minecraft.world.level.block.HorizontalDirectionalBlock;
import net.minecraft.world.level.block.state.BlockState;
import net.minecraft.world.level.block.state.StateDefinition;
import net.minecraft.world.phys.shapes.CollisionContext;
import net.minecraft.world.phys.shapes.Shapes;
import net.minecraft.world.phys.shapes.VoxelShape;

/** For cell-sized architectural models, use the very same cuboids for collision and selection. */
final class JakAuthoredBlock extends HorizontalDirectionalBlock {
    private final VoxelShape[] shapes = new VoxelShape[4];

    JakAuthoredBlock(Properties properties, JsonArray boxes) {
        super(properties);
        registerDefaultState(stateDefinition.any().setValue(FACING, Direction.NORTH));
        for (int turns = 0; turns < shapes.length; turns++) {
            VoxelShape shape = Shapes.empty();
            for (var row : boxes) {
                var box = row.getAsJsonArray();
                double x0 = box.get(0).getAsDouble(), y0 = box.get(1).getAsDouble(), z0 = box.get(2).getAsDouble();
                double x1 = box.get(3).getAsDouble(), y1 = box.get(4).getAsDouble(), z1 = box.get(5).getAsDouble();
                for (int turn = 0; turn < turns; turn++) {
                    double nextMinX = 16 - z1, nextMaxX = 16 - z0;
                    z0 = x0;
                    z1 = x1;
                    x0 = nextMinX;
                    x1 = nextMaxX;
                }
                shape = Shapes.or(shape, Block.box(x0, y0, z0, x1, y1, z1));
            }
            shapes[turns] = shape;
        }
    }

    @Override protected void createBlockStateDefinition(StateDefinition.Builder<Block, BlockState> builder) {
        builder.add(FACING);
    }

    @Override public BlockState getStateForPlacement(BlockPlaceContext context) {
        return defaultBlockState().setValue(FACING, context.getHorizontalDirection().getOpposite());
    }

    @Override protected VoxelShape getShape(BlockState state, BlockGetter level, BlockPos at, CollisionContext context) {
        return shapes[JakProps.turns(state.getValue(FACING))];
    }

    @Override protected VoxelShape getCollisionShape(BlockState state, BlockGetter level, BlockPos at, CollisionContext context) {
        return getShape(state, level, at, context);
    }
}
