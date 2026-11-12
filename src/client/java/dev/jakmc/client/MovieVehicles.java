package dev.jakmc.client;

import com.google.gson.JsonObject;
import dev.jakmc.entity.JakVehicle;
import dev.jakmc.player.JakPlayers;
import java.util.*;

/** The hull and both kinds of rider share the same two recorded simulation steps. */
public final class MovieVehicles {
    public interface Presentation {
        void jakmc$presentVehicle(JsonObject before, JsonObject after, double tick, float fraction);
    }

    private record Sample(double time, long tick, JsonObject frame) {}
    private static final Map<JakVehicle, JsonObject> seen = new WeakHashMap<>();
    private static final Map<Integer, List<Sample>> tracks = new HashMap<>();

    private MovieVehicles() {}

    public static void capture(JakVehicle vehicle) {
        if (!MovieMotion.recording()) return;
        var frame = vehicle.frame();
        if (frame == null || !frame.has("serverTick") || seen.put(vehicle, frame) == frame) return;
        MovieMotion.receivedVehicle(vehicle.getId(), frame);
    }

    static void add(JsonObject entry) {
        var frame = entry.getAsJsonObject("vehicle");
        tracks.computeIfAbsent(entry.get("id").getAsInt(), ignored -> new ArrayList<>())
            .add(new Sample(entry.get("time").getAsDouble(), frame.get("serverTick").getAsLong(), frame));
    }

    static void stabilize() {
        tracks.replaceAll((id, samples) -> {
            samples.sort(Comparator.comparingDouble(Sample::time));
            var stabilized = new ArrayList<Sample>(samples.size());
            for (int i = 0; i < samples.size(); i++) {
                var current = samples.get(i);
                double sum = 0;
                int count = 0;
                for (int j = Math.max(0, i - 3); j <= Math.min(samples.size() - 1, i + 3); j++) {
                    var nearby = samples.get(j);
                    double candidate = nearby.time - (nearby.tick - current.tick) * 50;
                    if (Math.abs(candidate - current.time) > 110) continue;
                    sum += candidate;
                    count++;
                }
                double time = sum / count;
                if (!stabilized.isEmpty()) time = Math.max(time, stabilized.getLast().time + 1);
                stabilized.add(new Sample(time, current.tick, current.frame));
            }
            return stabilized;
        });
    }

    static void clear() { tracks.clear(); }

    public static void sample(JakVehicle vehicle, float partial) {
        var samples = tracks.get(vehicle.getId());
        if (!MovieMotion.active() || samples == null) {
            vehicle.sampleReplayPresentation(partial);
            return;
        }
        double time = MovieMotion.seconds() * 1000;
        int low = 0, high = samples.size();
        while (low < high) {
            int mid = (low + high) >>> 1;
            if (samples.get(mid).time <= time) low = mid + 1;
            else high = mid;
        }
        var before = samples.get(Math.max(0, low - 1));
        var after = samples.get(Math.min(low, samples.size() - 1));
        if (time < before.time - 175 || time > after.time + 175) {
            vehicle.sampleReplayPresentation(partial);
            return;
        }
        var a = JakPlayers.vector(before.frame.getAsJsonArray("worldPosition"));
        var b = JakPlayers.vector(after.frame.getAsJsonArray("worldPosition"));
        long tickGap = after.tick - before.tick;
        double step = Math.max(JakPlayers.vector(before.frame.getAsJsonArray("position")).length(),
            JakPlayers.vector(after.frame.getAsJsonArray("position")).length());
        double plausibleTravel = Math.max(6, step * tickGap * 1.6 + 1);
        if (tickGap > 25 || a.distanceToSqr(b) > plausibleTravel * plausibleTravel) after = before;
        float fraction = before.time == after.time ? 0 :
            (float) Math.clamp((time - before.time) / (after.time - before.time), 0, 1);
        double tick = before.tick + (after.tick - before.tick) * fraction;
        ((Presentation) (Object) vehicle).jakmc$presentVehicle(before.frame, after.frame, tick, fraction);
    }
}
