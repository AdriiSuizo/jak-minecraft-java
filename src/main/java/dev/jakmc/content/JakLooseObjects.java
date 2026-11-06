package dev.jakmc.content;

import dev.jakmc.JakData;
import java.util.ArrayList;
import java.util.List;
import net.fabricmc.fabric.api.creativetab.v1.CreativeModeTabEvents;
import net.minecraft.core.Registry;
import net.minecraft.core.registries.BuiltInRegistries;
import net.minecraft.core.registries.Registries;
import net.minecraft.resources.Identifier;
import net.minecraft.resources.ResourceKey;
import net.minecraft.world.InteractionResult;
import net.minecraft.world.item.Item;
import net.minecraft.world.item.context.UseOnContext;

/** Older Jak object items: each places the corrected whole object its kind became, as the source does. */
public final class JakLooseObjects {
    private static final List<Item> ALIASES=new ArrayList<>();

    // jak_objects.rs placement_kind (palms and planta_2 -> palmera_completa, bidones -> bidon_fuego,
    // propaganda -> propaganda_bloque, haven_planta_N -> colgante_N); the pairs come from import_facade_parts.py.
    public static void initialize(){
        for(var entry:JakBlocks.resource("/data/jakmc/catalog/object_aliases.json").getAsJsonObject().entrySet()){
            var source=JakBlocks.resource("/data/jakmc/catalog/objects/"+entry.getKey()+".json").getAsJsonObject();
            var key=ResourceKey.create(Registries.ITEM,Identifier.fromNamespaceAndPath("jak",source.get("path").getAsString()));
            int target=entry.getValue().getAsInt();
            ALIASES.add(Registry.register(BuiltInRegistries.ITEM,key,new Alias(target,new Item.Properties().setId(key).useBlockDescriptionPrefix())));
        }
        CreativeModeTabEvents.modifyOutputEvent(ResourceKey.create(Registries.CREATIVE_MODE_TAB,JakData.id("details")))
            .register(output->ALIASES.forEach(output::accept));
    }

    // render_anim minecraft_world.rs place_object: placement_kind(kind) before placing.
    private static final class Alias extends Item {
        private final int target;
        Alias(int target,Properties properties){super(properties);this.target=target;}
        private Item placed(){
            var assembly=JakAssemblies.MODELS.get(target);
            if(assembly!=null)for(var part:assembly.parts())if(part.offset().equals(assembly.anchor()))return part.block().asItem();
            var prop=JakProps.MODELS.get(target);
            return prop==null?null:prop.block().asItem();
        }
        @Override public InteractionResult useOn(UseOnContext context){
            Item item=placed();
            return item==null?InteractionResult.FAIL:item.useOn(context);
        }
    }
}
