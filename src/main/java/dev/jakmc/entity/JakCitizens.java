package dev.jakmc.entity;
import dev.jakmc.*;
import net.minecraft.core.*;
import net.minecraft.core.component.DataComponents;
import net.minecraft.core.registries.*;
import net.minecraft.resources.ResourceKey;
import net.minecraft.world.entity.EntityTypes;
import net.minecraft.world.item.*;
import net.minecraft.world.item.component.TypedEntityData;
import net.minecraft.nbt.CompoundTag;
import java.util.*;
/** Spawn eggs keep the canonical Minecraft villager type, so breeding, POIs and trades remain native. */
public final class JakCitizens {
 public static final List<Item> EGGS=new ArrayList<>();
 public static void initialize(){CitizenKind.initialize();for(int i=0;i<CitizenKind.ALL.size();i++){var key=ResourceKey.create(Registries.ITEM,JakData.id(CitizenKind.of(i).model().replace('-','_')+"_spawn_egg"));var nbt=new CompoundTag();nbt.putInt("JakCitizenVariant",i);var item=new SpawnEggItem(new Item.Properties().setId(key).spawnEgg(EntityTypes.VILLAGER).component(DataComponents.ENTITY_DATA,TypedEntityData.of(EntityTypes.VILLAGER,nbt)));EGGS.add(Registry.register(BuiltInRegistries.ITEM,key,item));}}
}
