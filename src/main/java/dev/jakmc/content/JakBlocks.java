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
            if(!def.get("ready").getAsBoolean())continue;
            var id=Identifier.fromNamespaceAndPath("jak",path);
            var key=ResourceKey.create(Registries.BLOCK,id);
            var look=def.getAsJsonObject("look");String kind=look.get("kind").getAsString();
            String vanilla=look.has("vanilla")?look.get("vanilla").getAsString():"stone";
            Block base=BuiltInRegistries.BLOCK.getValue(Identifier.withDefaultNamespace(vanilla));
            var loot=ResourceKey.create(Registries.LOOT_TABLE,Identifier.fromNamespaceAndPath("jak","blocks/"+path));
            var properties=BlockBehaviour.Properties.ofLegacyCopy(base).setId(key).overrideLootTable(Optional.of(loot));
            if(path.contains("luz")||path.contains("luminosa")||path.contains("neon"))properties.lightLevel(state->15);
            if(look.has("see_through"))properties.noOcclusion();
            Block block=path.equals("altar_turbotabla")||path.equals("altar_turbotabla_vacio")
                ?new JakBoardAltar(properties.noOcclusion(),path.equals("altar_turbotabla"),look.getAsJsonArray("boxes"))
                :switch(kind){
                case "gate" -> new JakKrimsonGate(properties.noOcclusion().pushReaction(net.minecraft.world.level.material.PushReaction.IMMOVEABLE),look.getAsJsonArray("parts"));
                case "static_authored" -> new JakAuthoredBlock.Fixed(properties.noOcclusion(),look.getAsJsonArray("boxes"));
                case "authored" -> new JakAuthoredBlock(properties.noOcclusion(),look.getAsJsonArray("boxes"));
                case "slab" -> new SlabBlock(properties);
                case "layers" -> path.equals("snow_layer")?
                    new SnowLayerBlock(BlockBehaviour.Properties.ofLegacyCopy(Blocks.SNOW).setId(key).overrideLootTable(Optional.of(loot))):
                    new JakSolidLayers(properties);
                case "pillar" -> new RotatedPillarBlock(properties);
                case "leaves" -> new LeavesBlock(AmbientLeavesBlockSoundPlayer.noAmbientSound(),BlockBehaviour.Properties.ofLegacyCopy(Blocks.SPRUCE_LEAVES).setId(key).overrideLootTable(Optional.of(loot)));
                case "plant" -> new TallGrassBlock(BlockBehaviour.Properties.ofLegacyCopy(Blocks.SHORT_GRASS).setId(key).overrideLootTable(Optional.of(loot)));
                case "vine" -> new VineBlock(BlockBehaviour.Properties.ofLegacyCopy(Blocks.VINE).setId(key).overrideLootTable(Optional.of(loot)));
                case "lily_pad" -> new LilyPadBlock(BlockBehaviour.Properties.ofLegacyCopy(Blocks.LILY_PAD).setId(key).overrideLootTable(Optional.of(loot)));
                case "native" -> nativeBlock(vanilla,properties);
                default -> new Block(properties);
            };
            Registry.register(BuiltInRegistries.BLOCK,key,block);BLOCKS.put(path,block);
            var itemKey=ResourceKey.create(Registries.ITEM,id);
            var itemProperties=new Item.Properties().setId(itemKey).useBlockDescriptionPrefix();
            Registry.register(BuiltInRegistries.ITEM,itemKey,block instanceof JakKrimsonGate gate?
                new JakKrimsonGate.GateItem(gate,itemProperties):new BlockItem(block,itemProperties));
        }
        tab("biomes",p->!detail(p)&&!construction(p));
        tab("building",JakBlocks::construction);
        tab("details",JakBlocks::detail);
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
            case "ice" -> new IceBlock(p);
            case "leaf_litter" -> new LeafLitterBlock(p);
            default -> new Block(p);
        };
    }
    private static boolean construction(String p){return !detail(p)&&(p.startsWith("krimson_")||p.startsWith("palacio_")||p.startsWith("barrio_")||p.startsWith("haven_")||p.startsWith("spargus_")||p.contains("pared")||p.contains("suelo")||p.contains("techo")||p.contains("losa")||p.contains("escalera"));}
    private static boolean detail(String p){return p.startsWith("objeto_")||p.startsWith("detalle_")||p.contains("neon")||p.contains("lampara")||p.contains("altar")||p.contains("propa");}
    private static void tab(String name,java.util.function.Predicate<String> include){
        Registry.register(BuiltInRegistries.CREATIVE_MODE_TAB,JakData.id(name),FabricCreativeModeTab.builder().title(Component.translatable("tab.jakmc."+name)).icon(()->new ItemStack(BLOCKS.values().iterator().next())).displayItems((context,out)->BLOCKS.forEach((path,block)->{if(include.test(path))out.accept(block);})).build());
    }
}
