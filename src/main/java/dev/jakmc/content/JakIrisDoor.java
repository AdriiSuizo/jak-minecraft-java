package dev.jakmc.content;

import dev.jakmc.JakData;
import java.util.LinkedHashMap;
import net.fabricmc.fabric.api.event.player.AttackBlockCallback;
import net.fabricmc.fabric.api.event.player.UseBlockCallback;
import net.fabricmc.fabric.api.event.player.UseItemCallback;
import net.minecraft.core.BlockPos;
import net.minecraft.core.Direction;
import net.minecraft.core.Registry;
import net.minecraft.core.registries.BuiltInRegistries;
import net.minecraft.core.registries.Registries;
import net.minecraft.resources.ResourceKey;
import net.minecraft.server.level.ServerLevel;
import net.minecraft.sounds.SoundEvent;
import net.minecraft.sounds.SoundSource;
import net.minecraft.world.InteractionResult;
import net.minecraft.world.entity.Entity;
import net.minecraft.world.entity.player.Player;
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
import net.minecraft.world.phys.BlockHitResult;
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
 * <p>A door one block thick (Adrii 06/11; 2x2 since 07/11) is also Haven's airlock (Jak 2 airlock.gc `com-airlock`): there an outer and an inner
 * door close a passage between two zones while the console loads the next. Adrii 06/11: one door, both faces,
 * each with its own opening and closing (com-airlock-outer and com-airlock-inner back to back). stand-in
 * (Adrii 06/11): in place of want-cross-airlock? (anyone within its 143360 open distance) it opens and shuts
 * to a left click or the pad's L2 (Minecraft's use), and it collides as whole blocks shut and not at all open.
 * Adrii 07/11: no cells at its sides to leave holes in a wall; its leaves slide into what stands there. Adrii 08/11:
 * 3x3 as the iris door, and L2 (use) anywhere near it opens it, whatever the crosshair is on or the hand holds:
 * in third person it fell on the blocks round it and placed them (stand-in for want-cross-airlock's distance).
 */
public final class JakIrisDoor extends HorizontalDirectionalBlock implements EntityBlock {
    public static final IntegerProperty PART=IntegerProperty.create("part",0,15);
    public static final BooleanProperty OPEN=BlockStateProperties.OPEN;
    /** The cell the door is drawn and run from: the bottom row's second. */
    public static final int ANCHOR=1;
    private static final double OPEN_DISTANCE=8*dev.jakmc.player.JakPlayers.METRE,CLOSE_DISTANCE=12*dev.jakmc.player.JakPlayers.METRE;
    private static final VoxelShape NORTH_SOUTH=Block.box(0,0,6,16,16,10),EAST_WEST=Block.box(6,0,0,10,16,16);
    public static JakIrisDoor BLOCK,AIRLOCK;public static Item ITEM,AIRLOCK_ITEM;public static BlockEntityType<Door> TYPE;
    private static boolean removing;
    /** The last tick each player clicked an airlock: a held click (repeated by Minecraft every few ticks) is one. */
    private static final java.util.Map<Player,Long> CLICKS=new java.util.WeakHashMap<>();
    /** Haven's airlock rather than the iris door. */
    public final boolean airlock;
    /** Cells across and up: 3 both (Adrii 08/11; the airlock was 2 from 07/11). */
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
        // The airlock's left click (fired on both sides; the client then keeps from breaking it). Sneaking, it breaks as any block.
        AttackBlockCallback.EVENT.register((player,level,hand,at,face)->{
            var state=level.getBlockState(at);
            if(!(state.getBlock() instanceof JakIrisDoor door)||!door.airlock||player.isShiftKeyDown()||player.isSpectator())return InteractionResult.PASS;
            if(level instanceof ServerLevel server)click(server,player,at,state);
            return InteractionResult.SUCCESS;
        });
        UseBlockCallback.EVENT.register((player,level,hand,hit)->near(player,level,hand));
        UseItemCallback.EVENT.register((player,level,hand)->near(player,level,hand));
    }
    /** L2 by an airlock: that door, not what the crosshair or the hand would do. */
    private static InteractionResult near(Player player,Level level,net.minecraft.world.InteractionHand hand){
        if(player.isShiftKeyDown()||player.isSpectator())return InteractionResult.PASS;
        var middle=player.position().add(0,.9,0);BlockPos best=null;double closest=REACH*REACH;
        var from=player.blockPosition();
        for(var at:BlockPos.betweenClosed(from.offset(-3,-2,-3),from.offset(3,3,3))){
            var state=level.getBlockState(at);
            if(state.getBlock()!=AIRLOCK||state.getValue(PART)>=AIRLOCK.parts())continue;
            double d=Vec3.atCenterOf(at).distanceToSqr(middle);
            if(d<closest){closest=d;best=anchor(at,state);}
        }
        if(best==null)return InteractionResult.PASS;
        if(hand==net.minecraft.world.InteractionHand.MAIN_HAND&&level instanceof ServerLevel server)click(server,player,best,server.getBlockState(best));
        return InteractionResult.SUCCESS;
    }
    /** How near a cell of the airlock L2 reaches it from: Jak's middle within some two and a half blocks. */
    private static final double REACH=2.6;
    private static JakIrisDoor door(String name,boolean airlock){
        var key=ResourceKey.create(Registries.BLOCK,JakData.id(name));
        return Registry.register(BuiltInRegistries.BLOCK,key,new JakIrisDoor(Properties.of().setId(key).strength(50,1200).noOcclusion(),airlock));
    }
    private static Item item(JakIrisDoor door){
        var key=ResourceKey.create(Registries.ITEM,BuiltInRegistries.BLOCK.getKey(door));
        return Registry.register(BuiltInRegistries.ITEM,key,new DoorItem(door,new Item.Properties().setId(key).useBlockDescriptionPrefix()));
    }

    public int parts(){return size*size;}
    @Override protected void createBlockStateDefinition(StateDefinition.Builder<Block,BlockState> builder){builder.add(FACING,PART,OPEN);}
    @Override public BlockState getStateForPlacement(BlockPlaceContext context){
        return defaultBlockState().setValue(FACING,context.getHorizontalDirection().getOpposite());
    }
    private static VoxelShape slab(BlockState state){return state.getValue(FACING).getAxis()==Direction.Axis.Z?NORTH_SOUTH:EAST_WEST;}
    private VoxelShape shape(BlockState state,boolean collision){
        boolean open=state.getValue(OPEN);
        if(!airlock)return collision&&open?Shapes.empty():slab(state);
        // Whole blocks shut; open, they still take a click. (A cell past its four is left of a 3x3 of an older world.)
        if(state.getValue(PART)>=parts())return Shapes.empty();
        return collision&&open?Shapes.empty():Shapes.block();
    }
    @Override protected VoxelShape getShape(BlockState state,BlockGetter level,BlockPos at,CollisionContext context){return shape(state,false);}
    @Override protected VoxelShape getCollisionShape(BlockState state,BlockGetter level,BlockPos at,CollisionContext context){return shape(state,true);}
    @Override protected InteractionResult useWithoutItem(BlockState state,Level level,BlockPos at,Player player,BlockHitResult hit){
        if(!airlock)return InteractionResult.PASS;
        if(level instanceof ServerLevel server)click(server,player,at,state);
        return InteractionResult.SUCCESS;
    }
    private static void click(ServerLevel level,Player player,BlockPos at,BlockState state){
        long now=level.getGameTime();Long last=CLICKS.put(player,now);
        if(last==null||now-last>6)toggle(level,anchor(at,state));
    }
    /** The airlock opened if shut, shut if open; not onto anyone in its way (eco-door door-closing waits on
     * find-overlapping-shapes; here the click is refused). */
    static boolean toggle(ServerLevel level,BlockPos anchor){
        var state=level.getBlockState(anchor);
        if(!(state.getBlock() instanceof JakIrisDoor door)||state.getValue(PART)!=ANCHOR)return false;
        boolean open=!state.getValue(OPEN);var facing=state.getValue(FACING);
        for(int part=0;part<door.parts();part++){
            if(open)break;
            var at=cell(anchor,facing,part,door.size);var found=level.getBlockState(at);
            if(found.getBlock()!=door||found.getValue(PART)!=part)continue;
            if(!level.getEntities((Entity)null,new AABB(at),e->e.isAlive()&&!e.isSpectator()).isEmpty())return false;
        }
        Door.change(level,anchor,state,open);
        return true;
    }

    /** The anchor of the door this cell is part of. */
    public static BlockPos anchor(BlockPos at,BlockState state){
        int part=state.getValue(PART),size=((JakIrisDoor)state.getBlock()).size;
        return at.below(part/size).relative(state.getValue(FACING).getClockWise(),-(part%size-1));
    }
    public static BlockPos cell(BlockPos anchor,Direction facing,int part,int size){
        return anchor.above(part/size).relative(facing.getClockWise(),part%size-1);
    }

    @Override public BlockEntity newBlockEntity(BlockPos at,BlockState state){return state.getValue(PART)==ANCHOR?new Door(at,state):null;}
    @Override public <T extends BlockEntity> BlockEntityTicker<T> getTicker(Level level,BlockState state,BlockEntityType<T> type){
        if(level.isClientSide()||type!=TYPE||((JakIrisDoor)state.getBlock()).airlock)return null;
        return (world,at,current,door)->((Door)door).serverTick((ServerLevel)world,at,current);
    }

    @Override protected void affectNeighborsAfterRemoval(BlockState state,ServerLevel level,BlockPos at,boolean piston){
        if(removing)return;
        removing=true;
        try{
            var anchor=anchor(at,state);var facing=state.getValue(FACING);
            for(int part=0;part<parts();part++){
                var other=cell(anchor,facing,part,size);
                if(other.equals(at))continue;
                var found=level.getBlockState(other);
                if(found.getBlock()==this&&found.getValue(PART)==part&&found.getValue(FACING)==facing){
                    level.setBlockAndUpdate(other,Blocks.AIR.defaultBlockState());
                    level.levelEvent(2001,other,Block.getId(found));
                }
            }
        }finally{removing=false;}
    }

    public static final class DoorItem extends BlockItem {
        DoorItem(JakIrisDoor door,Properties properties){super(door,properties);}
        @Override protected boolean placeBlock(BlockPlaceContext context,BlockState state){
            var planned=plan(context,state);
            return planned!=null&&JakAssemblies.placeAll(context.getLevel(),planned);
        }
        /** Its cells where the context would place it, or null where it does not fit. */
        public LinkedHashMap<BlockPos,BlockState> plan(BlockPlaceContext context,BlockState state){
            var level=context.getLevel();var player=context.getPlayer();var anchor=context.getClickedPos();
            var facing=state.getValue(FACING);
            var planned=new LinkedHashMap<BlockPos,BlockState>();
            var door=(JakIrisDoor)getBlock();int size=door.size;
            for(int part=0;part<door.parts();part++){
                var at=cell(anchor,facing,part,size);
                var cell=state.setValue(PART,part).setValue(OPEN,false);
                boolean free=level.isInWorldBounds(at)&&level.hasChunkAt(at)&&(player==null||level.mayInteract(player,at))
                    &&level.getBlockState(at).canBeReplaced()&&level.isUnobstructed(cell,at,CollisionContext.placementContext(player));
                if(!free)return null;
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
            if(!state.getValue(OPEN)){
                // door-closed: Jak near, carrying blue eco (the iris door only: the airlock answers clicks).
                for(var player:level.players()){
                    if(player.isSpectator()||player.position().distanceTo(middle)>OPEN_DISTANCE+1||!dev.jakmc.player.JakPlayers.blueEco(player))continue;
                    change(level,at,state,true);
                    level.playSound(null,middle.x,middle.y,middle.z,sound("blue-eco-on"),SoundSource.BLOCKS,1,1);
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
            var door=(JakIrisDoor)state.getBlock();int size=door.size;
            for(int part=0;part<door.parts();part++){
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
