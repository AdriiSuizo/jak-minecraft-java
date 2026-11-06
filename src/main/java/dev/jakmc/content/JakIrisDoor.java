package dev.jakmc.content;

import dev.jakmc.JakData;
import java.util.LinkedHashMap;
import net.minecraft.core.BlockPos;
import net.minecraft.core.Direction;
import net.minecraft.core.Registry;
import net.minecraft.core.registries.BuiltInRegistries;
import net.minecraft.core.registries.Registries;
import net.minecraft.resources.ResourceKey;
import net.minecraft.server.level.ServerLevel;
import net.minecraft.sounds.SoundEvent;
import net.minecraft.sounds.SoundSource;
import net.minecraft.world.entity.Entity;
import net.minecraft.world.item.BlockItem;
import net.minecraft.world.item.Item;
import net.minecraft.world.item.context.BlockPlaceContext;
import net.minecraft.world.level.BlockGetter;
import net.minecraft.world.level.Level;
import net.minecraft.world.level.block.Block;
import net.minecraft.world.level.block.Blocks;
import net.minecraft.world.level.block.EntityBlock;
import net.minecraft.world.level.block.HorizontalDirectionalBlock;
import net.minecraft.world.level.block.entity.BlockEntity;
import net.minecraft.world.level.block.entity.BlockEntityTicker;
import net.minecraft.world.level.block.entity.BlockEntityType;
import net.minecraft.world.level.block.state.BlockState;
import net.minecraft.world.level.block.state.StateDefinition;
import net.minecraft.world.level.block.state.properties.BlockStateProperties;
import net.minecraft.world.level.block.state.properties.BooleanProperty;
import net.minecraft.world.level.block.state.properties.IntegerProperty;
import net.minecraft.world.phys.AABB;
import net.minecraft.world.phys.Vec3;
import net.minecraft.world.phys.shapes.CollisionContext;
import net.minecraft.world.phys.shapes.Shapes;
import net.minecraft.world.phys.shapes.VoxelShape;

/**
 * Geyser Rock's iris door (training-obs.gc `tra-iris-door`, an `eco-door` of baseplat.gc with the jng-iris-door
 * art): it opens when Jak, carrying blue eco, comes within its open distance (32768, 8 m), "blue-eco-on" as it
 * does, its blades turning open (jng-iris-door-idle forward), and then is gone. Adrii 06/11: 3x3 blocks, only
 * the blades. stand-in: the original stays open once opened (entity complete); here it closes again behind
 * him past its close distance (49152, 12 m), as an auto-close door does, with nothing in its way.
 *
 * <p>The same 3x3 door, one block thick (Adrii 06/11), is also Haven's airlock (Jak 2 airlock.gc `com-airlock`): there an outer and an inner
 * door close a passage between two zones while the console loads the next. Adrii 06/11: one door, both faces,
 * each with its own opening and closing (com-airlock-outer and com-airlock-inner back to back). It opens to
 * anyone near, no eco needed. stand-in: its open distance (143360, 35 m, for a door some 20 m wide) is taken
 * as the iris door's 8 m for one three blocks wide, and it closes past 12 m, nothing loading in between.
 */
public final class JakIrisDoor extends HorizontalDirectionalBlock implements EntityBlock {
    public static final IntegerProperty PART=IntegerProperty.create("part",0,15);
    public static final BooleanProperty OPEN=BlockStateProperties.OPEN;
    /** The cell the door is drawn and run from: the bottom row's middle. */
    public static final int ANCHOR=1;
    private static final double OPEN_DISTANCE=8*dev.jakmc.player.JakPlayers.METRE,CLOSE_DISTANCE=12*dev.jakmc.player.JakPlayers.METRE;
    private static final VoxelShape NORTH_SOUTH=Block.box(0,0,6,16,16,10),EAST_WEST=Block.box(6,0,0,10,16,16);
    public static JakIrisDoor BLOCK,AIRLOCK;public static Item ITEM,AIRLOCK_ITEM;public static BlockEntityType<Door> TYPE;
    private static boolean removing;
    /** Haven's airlock rather than the iris door. */
    public final boolean airlock;
    /** Cells across and up (3 for both; Adrii tried the airlock at 4 and kept 3). */
    public final int size;

    private JakIrisDoor(Properties properties,boolean airlock){
        super(properties);
        this.airlock=airlock;this.size=3;
        registerDefaultState(stateDefinition.any().setValue(FACING,Direction.NORTH).setValue(PART,ANCHOR).setValue(OPEN,false));
    }

    public static void initialize(){
        var id=JakData.id("iris_door");
        BLOCK=door("iris_door",false);ITEM=item(BLOCK);
        AIRLOCK=door("airlock_door",true);AIRLOCK_ITEM=item(AIRLOCK);
        TYPE=Registry.register(BuiltInRegistries.BLOCK_ENTITY_TYPE,id,new BlockEntityType<>(Door::new,java.util.Set.of(BLOCK,AIRLOCK)));
    }
    private static JakIrisDoor door(String name,boolean airlock){
        var key=ResourceKey.create(Registries.BLOCK,JakData.id(name));
        return Registry.register(BuiltInRegistries.BLOCK,key,new JakIrisDoor(Properties.of().setId(key).strength(50,1200).noOcclusion(),airlock));
    }
    private static Item item(JakIrisDoor door){
        var key=ResourceKey.create(Registries.ITEM,BuiltInRegistries.BLOCK.getKey(door));
        return Registry.register(BuiltInRegistries.ITEM,key,new DoorItem(door,new Item.Properties().setId(key).useBlockDescriptionPrefix()));
    }

    @Override protected void createBlockStateDefinition(StateDefinition.Builder<Block,BlockState> builder){builder.add(FACING,PART,OPEN);}
    @Override public BlockState getStateForPlacement(BlockPlaceContext context){
        return defaultBlockState().setValue(FACING,context.getHorizontalDirection().getOpposite());
    }
    private static VoxelShape slab(BlockState state){return state.getValue(FACING).getAxis()==Direction.Axis.Z?NORTH_SOUTH:EAST_WEST;}
    @Override protected VoxelShape getShape(BlockState state,BlockGetter level,BlockPos at,CollisionContext context){return slab(state);}
    @Override protected VoxelShape getCollisionShape(BlockState state,BlockGetter level,BlockPos at,CollisionContext context){
        return state.getValue(OPEN)?Shapes.empty():slab(state);
    }

    /** The anchor of the door this cell is part of. */
    public static BlockPos anchor(BlockPos at,BlockState state){
        int part=state.getValue(PART),size=((JakIrisDoor)state.getBlock()).size;
        return at.below(part/size).relative(state.getValue(FACING).getClockWise(),-(part%size-1));
    }
    static BlockPos cell(BlockPos anchor,Direction facing,int part,int size){
        return anchor.above(part/size).relative(facing.getClockWise(),part%size-1);
    }

    @Override public BlockEntity newBlockEntity(BlockPos at,BlockState state){return state.getValue(PART)==ANCHOR?new Door(at,state):null;}
    @Override public <T extends BlockEntity> BlockEntityTicker<T> getTicker(Level level,BlockState state,BlockEntityType<T> type){
        if(level.isClientSide()||type!=TYPE)return null;
        return (world,at,current,door)->((Door)door).serverTick((ServerLevel)world,at,current);
    }

    @Override protected void affectNeighborsAfterRemoval(BlockState state,ServerLevel level,BlockPos at,boolean piston){
        if(removing)return;
        removing=true;
        try{
            var anchor=anchor(at,state);var facing=state.getValue(FACING);
            for(int part=0;part<size*size;part++){
                var other=cell(anchor,facing,part,size);
                if(other.equals(at))continue;
                var found=level.getBlockState(other);
                if(found.getBlock()==this&&found.getValue(PART)==part&&found.getValue(FACING)==facing)level.setBlockAndUpdate(other,Blocks.AIR.defaultBlockState());
            }
        }finally{removing=false;}
    }

    public static final class DoorItem extends BlockItem {
        DoorItem(JakIrisDoor door,Properties properties){super(door,properties);}
        @Override protected boolean placeBlock(BlockPlaceContext context,BlockState state){
            var planned=plan(context,state);
            return planned!=null&&JakAssemblies.placeAll(context.getLevel(),planned);
        }
        /** Its nine cells where the context would place it, or null where it does not fit. */
        public LinkedHashMap<BlockPos,BlockState> plan(BlockPlaceContext context,BlockState state){
            var level=context.getLevel();var player=context.getPlayer();var anchor=context.getClickedPos();
            var facing=state.getValue(FACING);
            var planned=new LinkedHashMap<BlockPos,BlockState>();
            int size=((JakIrisDoor)getBlock()).size;
            for(int part=0;part<size*size;part++){
                var at=cell(anchor,facing,part,size);
                var cell=state.setValue(PART,part).setValue(OPEN,false);
                if(!level.isInWorldBounds(at)||!level.hasChunkAt(at)||player!=null&&!level.mayInteract(player,at)
                    ||!level.getBlockState(at).canBeReplaced()||!level.isUnobstructed(cell,at,CollisionContext.placementContext(player)))return null;
                planned.put(at,cell);
            }
            return planned;
        }
    }

    /** The anchor's state; on the client also when it last opened or closed (the blades' clip). */
    public static final class Door extends BlockEntity {
        public long changedAt=Long.MIN_VALUE/2;
        /** Client: the frame each of its doors showed last, where a change of mind starts from, and the step
         * of the opening or closing each is in (for its sounds). */
        public final float[] shown=new float[2],from=new float[2];
        public final int[] step={-1,-1};
        private boolean wasOpen;
        Door(BlockPos at,BlockState state){super(TYPE,at,state);wasOpen=state.getValue(OPEN);}

        void serverTick(ServerLevel level,BlockPos at,BlockState state){
            if(level.getGameTime()%4!=0)return;
            var middle=Vec3.atCenterOf(at.above());
            boolean airlock=((JakIrisDoor)state.getBlock()).airlock;
            if(!state.getValue(OPEN)){
                // door-closed: Jak near, carrying blue eco (the airlock: anyone near, want-cross-airlock?).
                for(var player:level.players()){
                    if(player.isSpectator()||player.position().distanceTo(middle)>OPEN_DISTANCE+1||!airlock&&!dev.jakmc.player.JakPlayers.blueEco(player))continue;
                    change(level,at,state,true);
                    if(!airlock)level.playSound(null,middle.x,middle.y,middle.z,sound("blue-eco-on"),SoundSource.BLOCKS,1,1);
                    return;
                }
                return;
            }
            // door-open, auto-closing: no Jak within the close distance and (door-closing) nothing overlapping it.
            if(level.players().stream().anyMatch(p->!p.isSpectator()&&p.position().distanceTo(middle)<CLOSE_DISTANCE))return;
            var facing=state.getValue(FACING);
            int size=((JakIrisDoor)state.getBlock()).size;
            var box=new AABB(cell(at,facing,0,size)).minmax(new AABB(cell(at,facing,size*size-1,size)));
            if(!level.getEntities((Entity)null,box,e->e.isAlive()&&!e.isSpectator()).isEmpty())return;
            change(level,at,state,false);
        }
        private static void change(ServerLevel level,BlockPos anchor,BlockState state,boolean open){
            var facing=state.getValue(FACING);
            int size=((JakIrisDoor)state.getBlock()).size;
            for(int part=0;part<size*size;part++){
                var at=cell(anchor,facing,part,size);
                var found=level.getBlockState(at);
                if(found.getBlock()==state.getBlock()&&found.getValue(PART)==part)level.setBlock(at,found.setValue(OPEN,open),Block.UPDATE_CLIENTS|Block.UPDATE_KNOWN_SHAPE);
            }
        }
        private static SoundEvent sound(String name){
            return SoundEvent.createVariableRangeEvent(JakData.id("sfx/jak1/"+name.replace('-','_')));
        }
        @Override public void setBlockState(BlockState state){
            super.setBlockState(state);
            seen(state.getValue(OPEN));
        }
        /** Client: the door as the world has it now (its own cached state is not always updated there). */
        public void seen(boolean open){
            if(open!=wasOpen){wasOpen=open;changedAt=System.nanoTime();System.arraycopy(shown,0,from,0,2);step[0]=step[1]=-1;}
        }
    }
}
