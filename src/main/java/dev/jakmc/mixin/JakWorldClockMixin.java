package dev.jakmc.mixin;

import dev.jakmc.player.JakEntityClock;
import net.minecraft.world.level.Level;
import org.spongepowered.asm.mixin.Mixin;
import org.spongepowered.asm.mixin.Unique;

@Mixin(Level.class)
abstract class JakWorldClockMixin implements JakEntityClock {
    @Unique private final State jakmc$clock=new State();

    @Override public State jakmc$entityClock(){return jakmc$clock;}
}
