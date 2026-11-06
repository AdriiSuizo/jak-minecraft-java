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
 * Jak 1's blue eco launcher (generic-obs.gc `launcher`, carried by citadel-obs.gc `citb-launcher`): within its
 * active distance it wakes only while Jak carries blue eco (its particles and launch-idle loop), and then
 * throws him up its spring height (target2.gc target-launch). Adrii 06/11: in place of the skatepark pad.
 * stand-in: its sensor is the block's top instead of the 3 m sphere, and (Adrii 06/11) it throws him as he
 * jumps from it, not on touch as launcher-active does. Its mesh sits on the block below, wider than its
 * block; it collides as its own block alone, up to the mesh's top.
 */
public final class JakEcoLauncher extends Block implements EntityBlock {
 /** citb-launcher's spring-height default (163840 units, 40 m). */
 public static final float SPRING_METRES=163840/4096f;
 /** citb-launcher baseplat-method-26: the launcher it spawns wakes within 81920 units (20 m). */
 public static final float ACTIVE_METRES=20;
 /** citb-launcher-lod0's depth below its top (1.79 m), in blocks at the creatures' scale: its top above the block's bottom. */
 public static final float TOP=1.79f*dev.jakmc.entity.JakEnemyKind.SCALE;
 private static final VoxelShape SHAPE=Block.box(0,0,0,16,16*TOP,16);
 public static JakEcoLauncher BLOCK;public static Item ITEM;public static BlockEntityType<Pad> TYPE;
 private JakEcoLauncher(Properties properties){super(properties);}
 public static void initialize(){
  var id=JakData.id("eco_launcher");
  var key=ResourceKey.create(Registries.BLOCK,id);
  BLOCK=Registry.register(BuiltInRegistries.BLOCK,key,new JakEcoLauncher(Properties.of().setId(key).strength(1.5f,6).noOcclusion()));
  var item=ResourceKey.create(Registries.ITEM,id);
  ITEM=Registry.register(BuiltInRegistries.ITEM,item,new BlockItem(BLOCK,new Item.Properties().setId(item).useBlockDescriptionPrefix()));
  TYPE=Registry.register(BuiltInRegistries.BLOCK_ENTITY_TYPE,id,new BlockEntityType<>(Pad::new,java.util.Set.of(BLOCK)));
 }
 @Override protected VoxelShape getShape(BlockState state,BlockGetter level,BlockPos at,CollisionContext context){return SHAPE;}
 @Override public BlockEntity newBlockEntity(BlockPos at,BlockState state){return new Pad(at,state);}
 /** For the original mesh and its particles. */
 public static final class Pad extends BlockEntity{Pad(BlockPos at,BlockState state){super(TYPE,at,state);}}
}
