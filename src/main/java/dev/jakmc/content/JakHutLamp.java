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
import net.minecraft.world.level.block.SoundType;
import net.minecraft.world.level.block.entity.BlockEntity;
import net.minecraft.world.level.block.entity.BlockEntityType;
import net.minecraft.world.level.block.state.BlockState;
import net.minecraft.world.phys.shapes.CollisionContext;
import net.minecraft.world.phys.shapes.Shapes;
import net.minecraft.world.phys.shapes.VoxelShape;

/**
 * Sandover's hanging lamp (village-obs.gc `hutlamp`): its mesh at the creatures' scale, hung from the top of its block,
 * swinging on joint 3 (HutLampRenderer). The original has no particles of its own. stand-in: its lit paper shows
 * through Minecraft's block light and a full-bright draw (Sandover has it in its level lights).
 */
public final class JakHutLamp extends Block implements EntityBlock {
    private static final VoxelShape SHAPE=Block.box(3,0,3,13,16,13);
    public static JakHutLamp BLOCK;public static Item ITEM;public static BlockEntityType<Lamp> TYPE;

    private JakHutLamp(Properties properties){super(properties);}
    public static void initialize(){
        var id=JakData.id("hut_lamp");
        var key=ResourceKey.create(Registries.BLOCK,id);
        BLOCK=Registry.register(BuiltInRegistries.BLOCK,key,new JakHutLamp(Properties.of().setId(key).strength(.5f).sound(SoundType.WOOD).noOcclusion().lightLevel(_->15)));
        var item=ResourceKey.create(Registries.ITEM,id);
        ITEM=Registry.register(BuiltInRegistries.ITEM,item,new BlockItem(BLOCK,new Item.Properties().setId(item).useBlockDescriptionPrefix()));
        TYPE=Registry.register(BuiltInRegistries.BLOCK_ENTITY_TYPE,id,new BlockEntityType<>(Lamp::new,java.util.Set.of(BLOCK)));
    }
    @Override protected VoxelShape getShape(BlockState state,BlockGetter level,BlockPos at,CollisionContext context){return SHAPE;}
    @Override protected VoxelShape getCollisionShape(BlockState state,BlockGetter level,BlockPos at,CollisionContext context){return Shapes.empty();}
    @Override public BlockEntity newBlockEntity(BlockPos at,BlockState state){return new Lamp(at,state);}

    /** Carries the mesh (HutLampRenderer). */
    public static final class Lamp extends BlockEntity {
        Lamp(BlockPos at,BlockState state){super(TYPE,at,state);}
    }
}
