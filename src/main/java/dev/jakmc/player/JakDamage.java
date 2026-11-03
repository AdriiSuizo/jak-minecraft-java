package dev.jakmc.player;
import dev.jakmc.JakData;
import net.minecraft.core.registries.Registries;
import net.minecraft.resources.ResourceKey;
import net.minecraft.server.level.ServerPlayer;
import net.minecraft.world.damagesource.*;
/** Real Minecraft damage types preserve armor, enchantments, attribution and tags. */
public final class JakDamage {
 public static DamageSource shot(ServerPlayer player,boolean explosive){
  var key=ResourceKey.create(Registries.DAMAGE_TYPE,JakData.id(explosive?"morph_explosion":"morph_gun"));
  return new DamageSource(player.level().registryAccess().lookupOrThrow(Registries.DAMAGE_TYPE).getOrThrow(key),player,player);
 }
}
