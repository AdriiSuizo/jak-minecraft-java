package dev.jakmc.mixin.client;

import com.replaymod.pathing.properties.TimestampProperty;
import com.replaymod.render.rendering.VideoRenderer;
import com.replaymod.replaystudio.pathing.path.Timeline;
import dev.jakmc.client.MovieMotion;
import org.spongepowered.asm.mixin.Final;
import org.spongepowered.asm.mixin.Mixin;
import org.spongepowered.asm.mixin.Shadow;
import org.spongepowered.asm.mixin.injection.At;
import org.spongepowered.asm.mixin.injection.Inject;
import org.spongepowered.asm.mixin.injection.callback.CallbackInfo;

@Mixin(value = VideoRenderer.class, remap = false)
abstract class MovieReplayExportMixin {
    @Shadow @Final private Timeline timeline;
    @Shadow public abstract int getFramesDone();
    @Shadow public abstract com.replaymod.render.RenderSettings getRenderSettings();

    @Inject(method = "setup", at = @At("TAIL"))
    private void jakmc$movieStart(CallbackInfo ci) {
        MovieMotion.useClock(() -> {
            double t = Math.max(0, getFramesDone() - 1) * 1000.0 / getRenderSettings().getFramesPerSecond();
            long lo = (long) Math.floor(t);
            int from = timeline.getValue(TimestampProperty.PROPERTY, lo).orElse(0);
            int to = timeline.getValue(TimestampProperty.PROPERTY, lo + 1).orElse(from);
            return from + (to - from) * (t - lo);
        });
    }

    @Inject(method = "finish", at = @At("TAIL"))
    private void jakmc$movieFinish(CallbackInfo ci) { MovieMotion.restoreClock(); }
}
