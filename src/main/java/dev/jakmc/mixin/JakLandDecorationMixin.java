package dev.jakmc.mixin;
import dev.jakmc.worldgen.JakLands;
import net.minecraft.world.level.StructureManager;
import net.minecraft.world.level.WorldGenLevel;
import net.minecraft.world.level.chunk.ChunkAccess;
import net.minecraft.world.level.chunk.ChunkGenerator;
import org.spongepowered.asm.mixin.Mixin;
import org.spongepowered.asm.mixin.injection.*;
import org.spongepowered.asm.mixin.injection.callback.CallbackInfo;
/** After Minecraft's features, the deserts, snowy lands and swamps become Jak's lands. */
@Mixin(ChunkGenerator.class)
abstract class JakLandDecorationMixin {
    @Inject(method="applyBiomeDecoration",at=@At("TAIL"))
    private void jakmc$jakLands(WorldGenLevel level,ChunkAccess chunk,StructureManager structures,CallbackInfo ci){
        JakLands.decorate(level,chunk,(ChunkGenerator)(Object)this);
    }
}
