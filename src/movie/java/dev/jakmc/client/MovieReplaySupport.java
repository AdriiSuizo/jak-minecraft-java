package dev.jakmc.client;

import com.replaymod.replay.events.ReplayClosingCallback;
import com.replaymod.replay.events.ReplayOpenedCallback;
import java.io.InputStreamReader;
import java.nio.charset.StandardCharsets;
import net.fabricmc.api.ClientModInitializer;

/** The Movie profile reads motion tracks embedded by the cinematic recorder. */
public final class MovieReplaySupport implements ClientModInitializer {
    private static MovieMotion.Recorder recorder() {
        var module = com.replaymod.recording.ReplayModRecording.instance;
        var listener = module == null || module.getConnectionEventHandler() == null
            ? null : module.getConnectionEventHandler().getPacketListener();
        return listener == null ? null : new MovieMotion.Recorder(listener, listener.getCurrentDuration());
    }

    @Override public void onInitializeClient() {
        MovieMotion.installRecorder(MovieReplaySupport::recorder);
        var events = new com.replaymod.lib.de.johni0702.minecraft.gui.utils.EventRegistrations();
        events.on(ReplayOpenedCallback.EVENT, handler -> {
            MovieMotion.stop();
            JakPoses.clearReplayFallback();
            var motion = handler.getReplayFile().get("jakmc-motion.json");
            if (!motion.isPresent()) return;
            try (var reader = new InputStreamReader(motion.get(), StandardCharsets.UTF_8)) {
                MovieMotion.start(reader, () -> handler.getReplaySender().currentTimeStamp());
            } catch (Exception invalidTrack) {
                MovieMotion.stop();
                dev.jakmc.JakMod.LOG.warn("Could not load the replay's Jak motion track", invalidTrack);
            }
        });
        events.on(ReplayClosingCallback.EVENT, handler -> {
            MovieMotion.stop();
            JakPoses.clearReplayFallback();
        });
        events.register();
    }
}
