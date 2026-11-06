package dev.jakmc.content;

import dev.jakmc.JakData;
import java.util.LinkedHashMap;
import net.minecraft.core.BlockPos;
import net.minecraft.core.Registry;
import net.minecraft.core.registries.BuiltInRegistries;
import net.minecraft.core.registries.Registries;
import net.minecraft.network.chat.Component;
import net.minecraft.resources.ResourceKey;
import net.minecraft.server.level.ServerLevel;
import net.minecraft.world.item.BlockItem;
import net.minecraft.world.item.Item;
import net.minecraft.world.item.context.BlockPlaceContext;
import net.minecraft.world.level.BlockGetter;
import net.minecraft.world.level.block.Block;
import net.minecraft.world.level.block.Blocks;
import net.minecraft.world.level.block.EntityBlock;
import net.minecraft.world.level.block.entity.BlockEntity;
import net.minecraft.world.level.block.entity.BlockEntityType;
import net.minecraft.world.level.block.state.BlockState;
import net.minecraft.world.level.block.state.StateDefinition;
import net.minecraft.world.level.block.state.properties.IntegerProperty;
import net.minecraft.world.phys.shapes.CollisionContext;
import net.minecraft.world.phys.shapes.Shapes;
import net.minecraft.world.phys.shapes.VoxelShape;

/**
 * Jak 1's blue eco launcher (generic-obs.gc `launcher`, carried by citadel-obs.gc `citb-launcher`): within its
 * active distance it wakes only while Jak carries blue eco (its particles and launch-idle loop), and then
 * throws him up its spring height (target2.gc target-launch). Adrii 06/11: in place of the skatepark pad.
 *
 * <p>Its mesh is a round pad 8.02 m across and 1.786 m high (measured on citb-launcher-lod0), 4.42 blocks across
 * at the creatures' scale, so it is a block of 5x5 cells in one layer like the 3x3 doors: placed, it takes all
 * its cells (and is refused if any is taken or a creature stands in one), it collides only as the disc it is
 * drawn as, up to the mesh's top (so not into the block above), and breaking one cell breaks it all. Adrii
 * 06/11: it was drawn inside the blocks round it and its collision went through them.
 * stand-in: its sensor is the block's top instead of the 3 m sphere, and (Adrii 06/11) it throws him as he
 * jumps from it, not on touch as launcher-active does.
 */
public final class JakEcoLauncher extends Block implements EntityBlock {
 /** citb-launcher's spring-height default (163840 units, 40 m). */
 public static final float SPRING_METRES=163840/4096f;
 /** citb-launcher baseplat-method-26: the launcher it spawns wakes within 81920 units (20 m). */
 public static final float ACTIVE_METRES=20;
 /** citb-launcher-lod0's depth below its top (1.79 m), in blocks at the creatures' scale: its top above the block's bottom. */
 public static final float TOP=1.79f*dev.jakmc.entity.JakEnemyKind.SCALE;
 /** Cells across and the middle one, the cell it is drawn and run from. */
 public static final int SIZE=5,ANCHOR=SIZE*SIZE/2;
 public static final IntegerProperty PART=IntegerProperty.create("part",0,SIZE*SIZE-1);
 /** Its rim's radius in blocks: 4.011 m (the widest vertices of citb-launcher-lod0) at the creatures' scale. */
 private static final float RIM=4.011f*dev.jakmc.entity.JakEnemyKind.SCALE;
 /** Each cell's collision and outline: the part of the disc it holds, in quarter blocks, as high as the mesh. */
 private static final VoxelShape[] SHAPES=new VoxelShape[SIZE*SIZE];
 static{
  for(int part=0;part<SHAPES.length;part++){
   double cx=part%SIZE-SIZE/2,cz=part/SIZE-SIZE/2;
   VoxelShape shape=Shapes.empty();
   for(int a=0;a<4;a++)for(int b=0;b<4;b++){
    double x=cx-.5+(a+.5)/4,z=cz-.5+(b+.5)/4;
    if(Math.hypot(x,z)<=RIM)shape=Shapes.or(shape,Block.box(a*4,0,b*4,a*4+4,16*TOP,b*4+4));
   }
   SHAPES[part]=shape.optimize();
  }
 }
 public static JakEcoLauncher BLOCK;public static Item ITEM;public static BlockEntityType<Pad> TYPE;
 private static boolean removing;
 private JakEcoLauncher(Properties properties){
  super(properties);
  registerDefaultState(stateDefinition.any().setValue(PART,ANCHOR));
 }
 public static void initialize(){
  var id=JakData.id("eco_launcher");
  var key=ResourceKey.create(Registries.BLOCK,id);
  BLOCK=Registry.register(BuiltInRegistries.BLOCK,key,new JakEcoLauncher(Properties.of().setId(key).strength(1.5f,6).noOcclusion()));
  var item=ResourceKey.create(Registries.ITEM,id);
  ITEM=Registry.register(BuiltInRegistries.ITEM,item,new LauncherItem(BLOCK,new Item.Properties().setId(item).useBlockDescriptionPrefix()));
  TYPE=Registry.register(BuiltInRegistries.BLOCK_ENTITY_TYPE,id,new BlockEntityType<>(Pad::new,java.util.Set.of(BLOCK)));
 }
 @Override protected void createBlockStateDefinition(StateDefinition.Builder<Block,BlockState> builder){builder.add(PART);}
 @Override protected VoxelShape getShape(BlockState state,BlockGetter level,BlockPos at,CollisionContext context){return SHAPES[state.getValue(PART)];}
 @Override protected VoxelShape getCollisionShape(BlockState state,BlockGetter level,BlockPos at,CollisionContext context){return SHAPES[state.getValue(PART)];}
 /** Only the middle cell carries the mesh and its effects. */
 @Override public BlockEntity newBlockEntity(BlockPos at,BlockState state){return state.getValue(PART)==ANCHOR?new Pad(at,state):null;}

 /** The cell of `part` for a launcher whose middle cell is `anchor`. */
 public static BlockPos cell(BlockPos anchor,int part){return anchor.offset(part%SIZE-SIZE/2,0,part/SIZE-SIZE/2);}
 /** The middle cell of the launcher this cell is part of. */
 public static BlockPos anchor(BlockPos at,BlockState state){int part=state.getValue(PART);return at.offset(SIZE/2-part%SIZE,0,SIZE/2-part/SIZE);}

 /** Breaking one cell takes the rest (without another drop: the cell broken gave the launcher). */
 @Override protected void affectNeighborsAfterRemoval(BlockState state,ServerLevel level,BlockPos at,boolean piston){
  if(removing)return;
  removing=true;
  try{
   var anchor=anchor(at,state);
   for(int part=0;part<SIZE*SIZE;part++){
    var other=cell(anchor,part);
    if(other.equals(at))continue;
    var found=level.getBlockState(other);
    if(found.getBlock()==this&&found.getValue(PART)==part)level.setBlockAndUpdate(other,Blocks.AIR.defaultBlockState());
   }
  }finally{removing=false;}
 }

 /** Places all its cells, or none where any cell is taken, out of the world or protected, or a creature stands in it. */
 public static final class LauncherItem extends BlockItem {
  LauncherItem(Block block,Properties properties){super(block,properties);}
  @Override protected boolean placeBlock(BlockPlaceContext context,BlockState state){
   var planned=plan(context,state);
   if(planned==null){
    if(context.getPlayer() instanceof net.minecraft.server.level.ServerPlayer player)player.sendOverlayMessage(Component.translatable("block.jakmc.eco_launcher.no_room"));
    return false;
   }
   return JakAssemblies.placeAll(context.getLevel(),planned);
  }
  /** Its cells where the context would place it, or null where it does not fit. */
  public LinkedHashMap<BlockPos,BlockState> plan(BlockPlaceContext context,BlockState state){
   var level=context.getLevel();var player=context.getPlayer();var anchor=context.getClickedPos();
   var planned=new LinkedHashMap<BlockPos,BlockState>();
   for(int part=0;part<SIZE*SIZE;part++){
    var at=cell(anchor,part);
    var cell=state.setValue(PART,part);
    boolean free=level.isInWorldBounds(at)&&level.hasChunkAt(at)&&(player==null||level.mayInteract(player,at))
     &&level.getBlockState(at).canBeReplaced()&&level.isUnobstructed(cell,at,CollisionContext.placementContext(player));
    if(!free)return null;
    planned.put(at,cell);
   }
   return planned;
  }
 }

 /** For the original mesh and its particles (the middle cell). */
 public static final class Pad extends BlockEntity{Pad(BlockPos at,BlockState state){super(TYPE,at,state);}}
}
