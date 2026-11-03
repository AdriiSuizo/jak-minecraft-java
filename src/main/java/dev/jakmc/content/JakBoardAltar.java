package dev.jakmc.content;

import dev.jakmc.JakData;
import dev.jakmc.player.JakPlayers;
import java.util.Set;
import net.minecraft.core.BlockPos;
import net.minecraft.core.Registry;
import net.minecraft.core.registries.BuiltInRegistries;
import net.minecraft.resources.Identifier;
import net.minecraft.server.level.ServerLevel;
import net.minecraft.server.level.ServerPlayer;
import net.minecraft.world.InteractionResult;
import net.minecraft.world.entity.player.Player;
import net.minecraft.world.item.ItemStack;
import net.minecraft.world.level.Level;
import net.minecraft.world.level.block.Block;
import net.minecraft.world.level.block.EntityBlock;
import net.minecraft.world.level.block.entity.BlockEntity;
import net.minecraft.world.level.block.entity.BlockEntityType;
import net.minecraft.world.level.block.state.BlockState;
import net.minecraft.world.level.storage.ValueInput;
import net.minecraft.world.level.storage.ValueOutput;
import net.minecraft.world.phys.BlockHitResult;

/** The pedestal and Jak's dedicated equipment slot exchange a single persistent board. */
public final class JakBoardAltar extends JakAuthoredBlock.Fixed implements EntityBlock {
    private static final int ANIMATE=1;
    private static JakBoardAltar filled,empty;
    public static BlockEntityType<Display> TYPE;
    public final boolean occupied;

    JakBoardAltar(Properties properties,boolean occupied,com.google.gson.JsonArray boxes){
        super(properties,boxes);
        this.occupied=occupied;
    }

    public static void initialize(){
        filled=(JakBoardAltar)JakBlocks.BLOCKS.get("altar_turbotabla");
        empty=(JakBoardAltar)JakBlocks.BLOCKS.get("altar_turbotabla_vacio");
        TYPE=Registry.register(BuiltInRegistries.BLOCK_ENTITY_TYPE,
            Identifier.fromNamespaceAndPath("jak","board_altar"),
            new BlockEntityType<>(Display::new,Set.of(filled,empty)));
    }

    @Override public BlockEntity newBlockEntity(BlockPos at,BlockState state){return new Display(at,state);}

    @Override protected boolean triggerEvent(BlockState state,Level level,BlockPos at,int id,int data){
        return level.getBlockEntity(at) instanceof Display display&&display.triggerEvent(id,data);
    }


    @Override protected InteractionResult useWithoutItem(BlockState state,Level level,BlockPos at,Player player,BlockHitResult hit){
        if(!JakPlayers.active(player)||player.isSpectator())return InteractionResult.PASS;
        if(level.isClientSide())return InteractionResult.SUCCESS;
        if(!(player instanceof ServerPlayer server)||!level.mayInteract(player,at)
            ||!(level.getBlockEntity(at) instanceof Display display))return InteractionResult.FAIL;
        ItemStack equipped=server.getAttachedOrCreate(JakData.BOARD);
        if(occupied&&!equipped.isEmpty()||!occupied&&(!equipped.is(JakItems.BOARD)||JakPlayers.onBoard(server)))return InteractionResult.FAIL;

        ItemStack transfer=occupied?display.board().copy():equipped.copy();
        JakBoardAltar next=occupied?empty:filled;
        display.transferring=true;
        if(!level.setBlockAndUpdate(at,next.defaultBlockState())){
            display.transferring=false;
            return InteractionResult.FAIL;
        }
        if(occupied){
            server.setAttached(JakData.BOARD,transfer);
            JakPlayers.requestAltarMount(server);
        }else{
            if(level.getBlockEntity(at) instanceof Display newDisplay){
                newDisplay.stored=transfer;
                newDisplay.setChanged();
            }
            server.setAttached(JakData.BOARD,ItemStack.EMPTY);
        }
        level.blockEvent(at,next,ANIMATE,0);
        return InteractionResult.SUCCESS;
    }

    public static final class Display extends BlockEntity {
        private ItemStack stored=ItemStack.EMPTY;
        private boolean transferring;
        private long start=Long.MIN_VALUE;

        Display(BlockPos at,BlockState state){super(TYPE,at,state);}
        public boolean occupied(){return ((JakBoardAltar)getBlockState().getBlock()).occupied;}
        private ItemStack board(){return stored.isEmpty()?new ItemStack(JakItems.BOARD):stored;}
        public float transition(float partial){
            return level==null||start==Long.MIN_VALUE?Float.POSITIVE_INFINITY:(level.getGameTime()-start+partial)/20f;
        }
        @Override public boolean triggerEvent(int id,int data){
            if(id!=ANIMATE||level==null)return super.triggerEvent(id,data);
            start=level.getGameTime();
            return true;
        }
        @Override protected void saveAdditional(ValueOutput out){
            super.saveAdditional(out);
            if(!stored.isEmpty())out.store("board",ItemStack.CODEC,stored);
        }
        @Override protected void loadAdditional(ValueInput in){
            super.loadAdditional(in);
            stored=in.read("board",ItemStack.CODEC).filter(item->item.is(JakItems.BOARD)).orElse(ItemStack.EMPTY);
        }
        @Override public void preRemoveSideEffects(BlockPos at,BlockState state){
            if(!transferring&&occupied()&&level instanceof ServerLevel server)
                Block.popResource(server,at,board());
        }
    }
}
