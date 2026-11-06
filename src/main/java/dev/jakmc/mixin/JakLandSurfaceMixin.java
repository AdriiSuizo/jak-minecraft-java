package dev.jakmc.mixin;
import dev.jakmc.worldgen.JakLands;
import java.util.Set;
import net.minecraft.core.Holder;
import net.minecraft.world.level.biome.Biome;
import net.minecraft.world.level.biome.BiomeManager;
import net.minecraft.world.level.chunk.ChunkAccess;
import net.minecraft.world.level.levelgen.NoiseBasedChunkGenerator;
import net.minecraft.world.level.levelgen.NoiseChunk;
import net.minecraft.world.level.levelgen.RandomState;
import net.minecraft.world.level.levelgen.material.rule.MaterialRule;
import org.spongepowered.asm.mixin.Mixin;
import org.spongepowered.asm.mixin.injection.*;
import org.spongepowered.asm.mixin.injection.callback.CallbackInfo;
/** Records the noise's own top for the Jak lands as Minecraft's surface is laid. */
@Mixin(NoiseBasedChunkGenerator.class)
abstract class JakLandSurfaceMixin {
    @Inject(method="buildSurface",at=@At("TAIL"))
    private void jakmc$recordSurface(ChunkAccess chunk,NoiseChunk noise,RandomState random,BiomeManager biomes,
                                     Set<Holder<Biome>> seen,MaterialRule rule,CallbackInfo ci){
        JakLands.recordSurface((NoiseBasedChunkGenerator)(Object)this,chunk,noise,random);
    }
}
