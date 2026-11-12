package dev.jakmc.mixin.client;

import com.replaymod.recording.packet.PacketListener;
import com.replaymod.replaystudio.replay.ReplayFile;
import dev.jakmc.client.MovieMotion;
import io.netty.channel.ChannelHandlerContext;
import java.io.OutputStreamWriter;
import java.nio.charset.StandardCharsets;
import java.util.concurrent.ExecutorService;
import org.spongepowered.asm.mixin.Final;
import org.spongepowered.asm.mixin.Mixin;
import org.spongepowered.asm.mixin.Shadow;
import org.spongepowered.asm.mixin.injection.At;
import org.spongepowered.asm.mixin.injection.Inject;
import org.spongepowered.asm.mixin.injection.callback.CallbackInfo;

@Mixin(value = PacketListener.class, remap = false)
abstract class MovieReplayRecordingMixin {
    @Shadow @Final private ReplayFile replayFile;
    @Shadow @Final private ExecutorService saveService;

    @Inject(method = "channelInactive", at = @At("HEAD"))
    private void jakmc$writeMotion(ChannelHandlerContext context, CallbackInfo ci) {
        var samples = MovieMotion.finishRecording((PacketListener) (Object) this);
        if (samples == null || samples.isEmpty()) return;
        saveService.execute(() -> {
            try (var writer = new OutputStreamWriter(replayFile.write("jakmc-motion.json"), StandardCharsets.UTF_8)) {
                MovieMotion.writeTrack(samples, writer);
            } catch (Exception error) {
                dev.jakmc.JakMod.LOG.warn("Could not save the replay's Jak motion track", error);
            }
        });
    }
}
