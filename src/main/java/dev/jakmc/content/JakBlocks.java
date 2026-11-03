package dev.jakmc.content;

import com.google.gson.*;
import dev.jakmc.*;
import java.io.*;
import java.nio.charset.StandardCharsets;
import java.util.*;
import net.fabricmc.fabric.api.creativetab.v1.FabricCreativeModeTab;
import net.minecraft.core.*;
import net.minecraft.core.registries.*;
import net.minecraft.resources.*;
import net.minecraft.network.chat.Component;
import net.minecraft.world.item.*;
import net.minecraft.world.item.context.BlockPlaceContext;
import net.minecraft.world.level.block.*;
import net.minecraft.world.level.block.sounds.AmbientLeavesBlockSoundPlayer;
import net.minecraft.world.level.block.state.*;
import net.minecraft.world.level.block.state.properties.*;

/** Native blocks keep vanilla placement, state updates, waterlogging and tools. */
public final class JakBlocks {
    public static final Map<String,Block> BLOCKS=new LinkedHashMap<>();
    public static final Map<String,JsonObject> DEFINITIONS=new LinkedHashMap<>();
    public static JsonElement resource(String name){
        try(var in=JakBlocks.class.getResourceAsStream(name)){
            if(in==null)throw new FileNotFoundException(name);
            return JsonParser.parseReader(new InputStreamReader(in,StandardCharsets.UTF_8));
        }catch(IOException e){throw new IllegalStateException("Missing generated Jak data: "+name,e);}
    }
    public static void initialize(){
        for(var element:resource("/data/jakmc/catalog/blocks.json").getAsJsonArray()){
            var def=element.getAsJsonObject();String path=def.get("path").getAsString();DEFINITIONS.put(path,def);
            // Whole imported objects are registered by the multiblock object system.
            if(path.startsWith("objeto_"))continue;
            var id=Identifier.fromNamespaceAndPath("jak",path);
            var key=ResourceKey.create(Registries.BLOCK,id);
            var look=def.getAsJsonObject("look");String kind=look.get("kind").getAsString();
            String vanilla=look.has("vanilla")?look.get("vanilla").getAsString():"stone";
            Block base=BuiltInRegistries.BLOCK.getValue(Identifier.withDefaultNamespace(vanilla));
            var properties=BlockBehaviour.Properties.ofFullCopy(base).setId(key);
            if(path.contains("luz")||path.contains("luminosa")||path.contains("neon"))properties.lightLevel(state->15);
            Block block=switch(kind){
                case "slab" -> new SlabBlock(properties);
                case "layers" -> new SnowLayerBlock(properties);
                case "pillar" -> new RotatedPillarBlock(properties);
                case "leaves" -> new LeavesBlock(AmbientLeavesBlockSoundPlayer.noAmbientSound(),BlockBehaviour.Properties.ofFullCopy(Blocks.SPRUCE_LEAVES).setId(key));
                case "plant" -> new Block(properties.noCollision().noOcclusion().instabreak());
                case "vine" -> new VineBlock(BlockBehaviour.Properties.ofFullCopy(Blocks.VINE).setId(key));
                case "native" -> nativeBlock(vanilla,properties);
                default -> new Block(properties);
            };
            Registry.register(BuiltInRegistries.BLOCK,key,block);BLOCKS.put(path,block);
            var itemKey=ResourceKey.create(Registries.ITEM,id);
            Registry.register(BuiltInRegistries.ITEM,itemKey,new BlockItem(block,new Item.Properties().setId(itemKey).useBlockDescriptionPrefix()));
        }
        tab("biomes","Jak: biomas",p->!detail(p)&&!construction(p));
        tab("building","Jak: construcción",JakBlocks::construction);
        tab("details","Jak: detalles",JakBlocks::detail);
        JakMod.LOG.info("Native Minecraft Jak construction blocks registered: {}",BLOCKS.size());
    }
    private static Block nativeBlock(String name,BlockBehaviour.Properties p){
        return switch(name){
            case "oak_door" -> new DoorBlock(BlockSetType.OAK,p);
            case "oak_trapdoor","spruce_trapdoor" -> new TrapDoorBlock(BlockSetType.OAK,p);
            case "oak_stairs","stone_stairs" -> new StairBlock(Blocks.STONE.defaultBlockState(),p);
            case "oak_slab","smooth_quartz_slab","spruce_slab","crimson_slab" -> new SlabBlock(p);
            case "cobblestone_wall" -> new WallBlock(p);
            case "glass_pane","iron_bars" -> new IronBarsBlock(p);
            case "lightning_rod" -> new LightningRodBlock(p);
            case "oak_fence_gate" -> new FenceGateBlock(WoodType.OAK,p);
            case "leaf_litter" -> new LeafLitterBlock(p);
            case "carved_pumpkin" -> new FacingDecorBlock(p);
            default -> new Block(p);
        };
    }
    private static boolean construction(String p){return !detail(p)&&(p.startsWith("palacio_")||p.startsWith("barrio_")||p.startsWith("haven_")||p.startsWith("spargus_")||p.contains("pared")||p.contains("suelo")||p.contains("techo")||p.contains("losa")||p.contains("escalera"));}
    private static boolean detail(String p){return p.startsWith("objeto_")||p.startsWith("detalle_")||p.contains("neon")||p.contains("lampara")||p.contains("altar")||p.contains("propa");}
    private static void tab(String name,String title,java.util.function.Predicate<String> include){
        Registry.register(BuiltInRegistries.CREATIVE_MODE_TAB,JakData.id(name),FabricCreativeModeTab.builder().title(Component.literal(title)).icon(()->new ItemStack(BLOCKS.values().iterator().next())).displayItems((context,out)->BLOCKS.forEach((path,block)->{if(include.test(path))out.accept(block);})).build());
    }
    private static final class FacingDecorBlock extends HorizontalDirectionalBlock {
        FacingDecorBlock(BlockBehaviour.Properties p){super(p);registerDefaultState(stateDefinition.any().setValue(FACING,Direction.NORTH));}
        @Override protected void createBlockStateDefinition(StateDefinition.Builder<Block,BlockState> builder){builder.add(FACING);}
        @Override public BlockState getStateForPlacement(BlockPlaceContext context){return defaultBlockState().setValue(FACING,context.getHorizontalDirection().getOpposite());}
    }
}
