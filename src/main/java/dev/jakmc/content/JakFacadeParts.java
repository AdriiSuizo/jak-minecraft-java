package dev.jakmc.content;

import java.util.Optional;
import net.minecraft.core.Registry;
import net.minecraft.core.registries.BuiltInRegistries;
import net.minecraft.core.registries.Registries;
import net.minecraft.resources.Identifier;
import net.minecraft.resources.ResourceKey;
import net.minecraft.world.item.BlockItem;
import net.minecraft.world.item.Item;
import net.minecraft.world.level.block.Blocks;
import net.minecraft.world.level.block.state.BlockBehaviour;

/** Shaped facade parts whose models turn or overhang their cell, drawn by their source model and turned like a carved pumpkin. */
public final class JakFacadeParts {
    // jak_facades.rs FACADE_BLOCKS / jak_barrio.rs BARRIO_BLOCKS (BlockLook::Shaped, vanilla carved_pumpkin);
    // collision: jak_facades.rs shaped_collision, baked by tools/import_facade_parts.py.
    public static void initialize(){
        for(var entry:JakBlocks.resource("/data/jakmc/catalog/facade_parts.json").getAsJsonObject().entrySet()){
            String path=entry.getKey();
            if(JakBlocks.BLOCKS.containsKey(path))continue;
            var id=Identifier.fromNamespaceAndPath("jak",path);
            var key=ResourceKey.create(Registries.BLOCK,id);
            var loot=ResourceKey.create(Registries.LOOT_TABLE,id.withPrefix("blocks/"));
            var properties=BlockBehaviour.Properties.ofLegacyCopy(Blocks.CARVED_PUMPKIN).noOcclusion().setId(key).overrideLootTable(Optional.of(loot));
            var block=Registry.register(BuiltInRegistries.BLOCK,key,new JakAuthoredBlock(properties,entry.getValue().getAsJsonArray()));
            var itemKey=ResourceKey.create(Registries.ITEM,id);
            Registry.register(BuiltInRegistries.ITEM,itemKey,new BlockItem(block,new Item.Properties().setId(itemKey).useBlockDescriptionPrefix()));
            JakBlocks.BLOCKS.put(path,block);
        }
    }
}
