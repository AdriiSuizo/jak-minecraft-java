package dev.jakmc.content;

import dev.jakmc.JakData;
import net.minecraft.core.BlockPos;
import net.minecraft.core.Registry;
import net.minecraft.core.registries.BuiltInRegistries;
import net.minecraft.core.registries.Registries;
import net.minecraft.resources.ResourceKey;
import net.minecraft.server.level.ServerLevel;
import net.minecraft.sounds.SoundEvent;
import net.minecraft.sounds.SoundSource;
import net.minecraft.util.StringRepresentable;
import net.minecraft.world.entity.player.Player;
import net.minecraft.world.item.BlockItem;
import net.minecraft.world.item.Item;
import net.minecraft.world.item.ItemStack;
import net.minecraft.world.level.BlockGetter;
import net.minecraft.world.level.Explosion;
import net.minecraft.world.level.Level;
import net.minecraft.world.level.block.Block;
import net.minecraft.world.level.block.EntityBlock;
import net.minecraft.world.level.block.SoundType;
import net.minecraft.world.level.block.entity.BlockEntity;
import net.minecraft.world.level.block.entity.BlockEntityType;
import net.minecraft.world.level.block.state.BlockState;
import net.minecraft.world.level.block.state.StateDefinition;
import net.minecraft.world.level.block.state.properties.EnumProperty;
import net.minecraft.world.phys.shapes.CollisionContext;
import net.minecraft.world.phys.shapes.VoxelShape;

/**
 * Jak II's wooden Krimson crate. Breaking one rolls an ammunition colour and amount, plays the
 * original break sound and emits the crate explosion at the block.
 */
public final class JakAmmoCrate extends Block implements EntityBlock {
    /** The two saved crate appearances. */
    public enum Contents implements StringRepresentable {
        YELLOW("yellow"),RED("red");
        final String name;
        Contents(String name){this.name=name;}
        @Override public String getSerializedName(){return name;}
    }
    public static final EnumProperty<Contents> CONTENTS=EnumProperty.create("contents",Contents.class);
    private static final VoxelShape SHAPE=Block.box(.5,0,.5,15.5,10,15.5);
    public static JakAmmoCrate BLOCK;public static Item ITEM;public static BlockEntityType<Crate> TYPE;

    private JakAmmoCrate(Properties properties){
        super(properties);
        registerDefaultState(stateDefinition.any().setValue(CONTENTS,Contents.YELLOW));
    }
    public static void initialize(){
        var id=JakData.id("ammo_crate");
        var key=ResourceKey.create(Registries.BLOCK,id);
        BLOCK=Registry.register(BuiltInRegistries.BLOCK,key,new JakAmmoCrate(Properties.of().setId(key).instabreak().sound(SoundType.WOOD).noOcclusion()));
        var item=ResourceKey.create(Registries.ITEM,id);
        ITEM=Registry.register(BuiltInRegistries.ITEM,item,new BlockItem(BLOCK,new Item.Properties().setId(item).useBlockDescriptionPrefix()));
        TYPE=Registry.register(BuiltInRegistries.BLOCK_ENTITY_TYPE,id,new BlockEntityType<>(Crate::new,java.util.Set.of(BLOCK)));
    }
    @Override protected void createBlockStateDefinition(StateDefinition.Builder<Block,BlockState> builder){builder.add(CONTENTS);}
    @Override protected VoxelShape getShape(BlockState state,BlockGetter level,BlockPos at,CollisionContext context){return SHAPE;}
    @Override protected VoxelShape getCollisionShape(BlockState state,BlockGetter level,BlockPos at,CollisionContext context){return SHAPE;}
    @Override public BlockEntity newBlockEntity(BlockPos at,BlockState state){return new Crate(at,state);}

    @Override public BlockState playerWillDestroy(Level level,BlockPos at,BlockState state,Player player){
        if(level instanceof ServerLevel server)open(server,at);
        return super.playerWillDestroy(level,at,state,player);
    }
    @Override protected void onExplosionHit(BlockState state,ServerLevel level,BlockPos at,Explosion explosion,java.util.function.BiConsumer<ItemStack,BlockPos> drops){
        var blast=explosion.getBlockInteraction();
        if(blast==Explosion.BlockInteraction.DESTROY||blast==Explosion.BlockInteraction.DESTROY_WITH_DECAY)open(level,at);
        super.onExplosionHit(state,level,at,explosion,drops);
    }
    /** crate `die`: "wcrate-break", then drop-pickup (the amount split into its pickups). */
    private static void open(ServerLevel level,BlockPos at){
        level.playSound(null,at,SoundEvent.createVariableRangeEvent(JakData.id("sfx/jak2/wcrate_break")),SoundSource.BLOCKS,1,1);
        var cue=new dev.jakmc.network.JakPackets.CrateBreak(level.dimension().identifier(),at);
        for(var watcher:net.fabricmc.fabric.api.networking.v1.PlayerLookup.tracking(level,at))
            if(net.fabricmc.fabric.api.networking.v1.ServerPlayNetworking.canSend(watcher,dev.jakmc.network.JakPackets.CrateBreak.TYPE))net.fabricmc.fabric.api.networking.v1.ServerPlayNetworking.send(watcher,cue);
        // The saved colour is the crate's appearance. Roll the contents when it breaks.
        int kind=level.getRandom().nextInt(JakItems.AMMUNITION.size());
        int amount=switch(kind){case 0,1->10+level.getRandom().nextInt(11);case 2->20+level.getRandom().nextInt(21);default->2+level.getRandom().nextInt(5);};
        popResource(level,at,new ItemStack(JakItems.AMMUNITION.get(kind),amount));
    }

    /** Carries the mesh (CrateRenderer). */
    public static final class Crate extends BlockEntity {
        Crate(BlockPos at,BlockState state){super(TYPE,at,state);}
    }
}
