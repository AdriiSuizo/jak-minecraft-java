package dev.jakmc.content;

import dev.jakmc.JakData;
import net.minecraft.core.BlockPos;
import net.minecraft.core.Registry;
import net.minecraft.core.registries.BuiltInRegistries;
import net.minecraft.core.registries.Registries;
import net.minecraft.resources.ResourceKey;
import net.minecraft.world.item.BlockItem;
import net.minecraft.world.item.Item;
import net.minecraft.world.level.BlockGetter;
import net.minecraft.world.level.block.Block;
import net.minecraft.world.level.block.EntityBlock;
import net.minecraft.world.level.block.entity.BlockEntity;
import net.minecraft.world.level.block.entity.BlockEntityType;
import net.minecraft.world.level.block.state.BlockState;
import net.minecraft.world.phys.shapes.CollisionContext;
import net.minecraft.world.phys.shapes.VoxelShape;

/**
 * Jak II's skatepark jump pad, `skatea-jump-pad` (skatea-obs.gc; Rust native_skate.rs springs): stood on,
 * it throws Jak up its spring height, the original's only pad's 102400 units (25 m). stand-in: the
 * original sits in the skatepark, which this world does not have whole; here a block to place, drawn
 * with the original mesh, its sensor the block's top instead of the 1.5 m sphere a metre over it.
 */
public final class JakJumpPad extends Block implements EntityBlock {
 /** skatea-actors.json skatea-jump-pad-4 lump spring-height, in metres. */
 public static final float SPRING_METRES=102400/4096f;
 private static final VoxelShape SHAPE=Block.box(0,0,0,16,2,16);
 public static JakJumpPad BLOCK;public static Item ITEM;public static BlockEntityType<Pad> TYPE;
 private JakJumpPad(Properties properties){super(properties);}
 public static void initialize(){
  var id=JakData.id("skate_jump_pad");
  var key=ResourceKey.create(Registries.BLOCK,id);
  BLOCK=Registry.register(BuiltInRegistries.BLOCK,key,new JakJumpPad(Properties.of().setId(key).strength(1.5f,6).noOcclusion()));
  var item=ResourceKey.create(Registries.ITEM,id);
  ITEM=Registry.register(BuiltInRegistries.ITEM,item,new BlockItem(BLOCK,new Item.Properties().setId(item).useBlockDescriptionPrefix()));
  TYPE=Registry.register(BuiltInRegistries.BLOCK_ENTITY_TYPE,id,new BlockEntityType<>(Pad::new,java.util.Set.of(BLOCK)));
 }
 @Override protected VoxelShape getShape(BlockState state,BlockGetter level,BlockPos at,CollisionContext context){return SHAPE;}
 @Override public BlockEntity newBlockEntity(BlockPos at,BlockState state){return new Pad(at,state);}
 /** Only for the original mesh to be drawn. */
 public static final class Pad extends BlockEntity{Pad(BlockPos at,BlockState state){super(TYPE,at,state);}}
}
