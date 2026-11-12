package dev.jakmc.client;

import com.google.gson.*;
import java.io.IOException;
import java.nio.file.*;
import java.util.*;
import java.util.function.*;
import net.minecraft.util.Mth;
import net.minecraft.world.phys.Vec3;
import org.joml.Quaternionf;

/** Samples recorded server poses against Replay Mod's exported frame time. */
public final class MovieMotion {
    private record Sample(double time, JakPoses.Pose pose) {}
    public record Flight(Vec3 at, Quaternionf attitude) {}
    record Recorder(Object listener, long milliseconds) {}
    private static LongSupplier recordingClock;
    private static Supplier<Recorder> liveRecorder;
    private static DoubleSupplier playbackClock;
    private static DoubleSupplier sessionClock;
    private static final List<JsonObject> capture = new ArrayList<>();
    private static final Map<Object,List<JsonObject>> recordings = new WeakHashMap<>();
    private static final Set<Object> closedRecordings = Collections.newSetFromMap(new WeakHashMap<>());
    private static final Set<String> EFFECT_FIELDS = Set.of("events", "effects", "sounds", "loops", "models", "attackSpheres");
    private static final Gson JSON = new GsonBuilder().serializeNulls().create();
    private static final Map<Integer, List<Sample>> history = new HashMap<>();
    private static final Map<Integer, JakPoses.Pose> poses = new HashMap<>();
    private static final Map<Integer, Double> ticks = new HashMap<>();
    private static final Map<Integer, Flight> flights = new HashMap<>();
    private static double cachedTime = Double.NaN;

    private MovieMotion() {}

    public static void record(LongSupplier clock) { recordingClock = clock; }

    static void installRecorder(Supplier<Recorder> supplier) { liveRecorder = supplier; }

    private static Recorder recorder() {
        return recordingClock == null && liveRecorder != null && !JakPoses.inReplay() ? liveRecorder.get() : null;
    }

    static boolean recording() {
        return recordingClock != null || recorder() != null;
    }

    static void receivedVehicle(int entity, JsonObject frame) {
        var recording = recorder();
        if (recording == null && recordingClock == null) return;
        var entry = new JsonObject();
        entry.addProperty("id", entity);
        entry.addProperty("time", recording == null ? recordingClock.getAsLong() : recording.milliseconds());
        var data = frame.deepCopy();
        data.remove("loops");
        data.remove("sounds");
        entry.add("vehicle", data);
        append(recording, entry);
    }

    private static void append(Recorder recording, JsonObject entry) {
        if (recording == null) capture.add(entry);
        else synchronized (recordings) {
            if (!closedRecordings.contains(recording.listener()))
                recordings.computeIfAbsent(recording.listener(), key -> new ArrayList<>()).add(entry);
        }
    }

    static void received(int entity, JakPoses.Pose pose) {
        var recording = recorder();
        if (recordingClock == null && recording == null) return;
        var p = new JsonObject();
        p.addProperty("time", recordingClock == null ? recording.milliseconds() : recordingClock.getAsLong());
        p.addProperty("id", entity);
        var at = new JsonArray();
        at.add(pose.position().x); at.add(pose.position().y); at.add(pose.position().z);
        p.add("position", at);
        var data = new JsonObject();
        for (var e : pose.raw().entrySet()) {
            if (!EFFECT_FIELDS.contains(e.getKey())) {
                data.add(e.getKey(), e.getValue());
            }
        }
        p.add("pose", data);
        append(recording, p);
    }

    public static List<JsonObject> finishRecording(Object listener) {
        synchronized (recordings) {
            closedRecordings.add(listener);
            return recordings.remove(listener);
        }
    }

    public static void writeTrack(List<JsonObject> samples, java.io.Writer out) { JSON.toJson(samples, out); }

    public static void save(Path path) throws IOException {
        try (var writer = Files.newBufferedWriter(path)) { writeTrack(capture, writer); }
        capture.clear();
        recordingClock = null;
    }

    public static void start(Path path, DoubleSupplier clock) throws IOException {
        try (var reader = Files.newBufferedReader(path)) {
            start(reader, clock);
        }
    }

    public static void start(java.io.Reader reader, DoubleSupplier clock) {
        stop();
        for (var element : JsonParser.parseReader(reader).getAsJsonArray()) {
                var entry = element.getAsJsonObject();
                if (entry.has("vehicle")) {
                    MovieVehicles.add(entry);
                    continue;
                }
                var p = entry.getAsJsonObject("pose");
                if (p.get("darkPose") instanceof JsonObject dark && !dark.has("next")) dark.add("next", JsonNull.INSTANCE);
                var at = dev.jakmc.player.JakPlayers.vector(entry.getAsJsonArray("position"));
                var board = p.get("boardPose") instanceof JsonObject b ? JakPoses.BoardPose.read(b) : null;
                var root = JakPoses.RootPose.read(p.getAsJsonObject("rootPose"));
                float yaw = p.get("yaw").getAsFloat();
                var pose = new JakPoses.Pose(at, yaw, p.get("clip").getAsString(), p.get("frame").getAsFloat(),
                    p.get("board").getAsBoolean(), p.get("dark").getAsBoolean(), p.get("light").getAsBoolean(),
                    p, 0, board, board, root, root, yaw);
                history.computeIfAbsent(entry.get("id").getAsInt(), key -> new ArrayList<>())
                    .add(new Sample(entry.get("time").getAsDouble(), pose));
        }
        history.replaceAll((id, samples) -> smooth(samples));
        MovieVehicles.stabilize();
        sessionClock = playbackClock = clock;
    }

    public static void useClock(DoubleSupplier clock) {
        if (sessionClock != null) playbackClock = clock;
    }

    public static void restoreClock() {
        playbackClock = sessionClock;
        cachedTime = Double.NaN;
    }

    private static List<Sample> smooth(List<Sample> samples) {
        var output = new ArrayList<Sample>(samples.size());
        for (int i = 0; i < samples.size(); i++) {
            Sample current = samples.get(i);
            long tick = tick(current);
            double elapsed = 0;
            int count = 0;
            for (int j = Math.max(0, i - 3); j <= Math.min(samples.size() - 1, i + 3); j++) {
                var nearby = samples.get(j);
                long difference = tick(nearby) - tick;
                if (Math.abs(nearby.time - current.time - difference * 50) > 110) continue;
                elapsed += nearby.time - difference * 50;
                count++;
            }
            double time = elapsed / count;
            if (!output.isEmpty()) time = Math.max(time, output.getLast().time + 1);
            output.add(new Sample(time, current.pose));
        }
        return output;
    }

    private static long tick(Sample sample) { return sample.pose.raw().get("serverTick").getAsLong(); }

    public static void stop() { sessionClock = playbackClock = null; history.clear(); poses.clear(); ticks.clear(); flights.clear(); MovieVehicles.clear(); cachedTime = Double.NaN; }
    public static boolean active() { return playbackClock != null; }
    public static long effectTime() {
        return active() && JakPoses.inReplay()
            ? 1_000_000_000L + Math.round(playbackClock.getAsDouble() * 1_000_000)
            : System.nanoTime();
    }

    static double renderedTick(int id) {
        prepare();
        return ticks.getOrDefault(id, Double.NaN);
    }

    static boolean blueEcoNear(Vec3 at, double reach) {
        prepare();
        double range = reach * reach;
        for (var pose : poses.values()) {
            if (pose.raw().has("blueEco") && pose.raw().get("blueEco").getAsBoolean()
                && pose.position().distanceToSqr(at) <= range) return true;
        }
        return false;
    }
    public static double seconds() {
        if (playbackClock != null) return playbackClock.getAsDouble() / 1000;
        var client = net.minecraft.client.Minecraft.getInstance();
        return client.level == null ? 0 : (client.level.getGameTime() + client.getDeltaTracker().getGameTimeDeltaPartialTick(false)) / 20.0;
    }

    private static void prepare() {
        if (!active()) return;
        double time = playbackClock.getAsDouble();
        if (time == cachedTime) return;
        cachedTime = time;
        poses.clear();
        ticks.clear();
        flights.clear();
        for (var entry : history.entrySet()) interpolate(entry.getKey(), entry.getValue(), time);
    }

    private static void interpolate(int id, List<Sample> samples, double time) {
        int low = 0, high = samples.size();
        while (low < high) {
            int mid = (low + high) >>> 1;
            if (samples.get(mid).time <= time) low = mid + 1;
            else high = mid;
        }
        if (low == 0) return;
        var left = samples.get(low - 1);
        var right = samples.get(Math.min(low, samples.size() - 1));
        var a = left.pose;
        var b = right.pose;
        boolean sameSession = a.raw().get("session").getAsLong() == b.raw().get("session").getAsLong();
        long tickGap = tick(right) - tick(left);
        boolean sameFlight = right.time > left.time && sameSession && tickGap > 0 && tickGap <= 4
            && a.raw().get("hellcat") instanceof JsonObject f
            && b.raw().get("hellcat") instanceof JsonObject g
            && f.get("entity").equals(g.get("entity"))
            && a.position().distanceToSqr(b.position()) < 16;
        if (time - left.time > 160 && !sameFlight) return;
        boolean smooth = right.time > left.time && sameSession
            && (right.time - left.time < 200 || sameFlight)
            && a.position().distanceToSqr(b.position()) < 16;
        float u = smooth ? (float) Math.clamp((time - left.time) / (right.time - left.time), 0, 1) : 0;
        Vec3 at = a.position().lerp(b.position(), u);
        float yaw = Mth.rotLerp(u, a.yaw(), b.yaw());
        float frame = a.frame();
        if (smooth && a.clip().equals(b.clip())) {
            float difference = b.frame() - frame;
            float period = a.raw().has("framePeriod") ? a.raw().get("framePeriod").getAsFloat() : 0;
            if (period > 0 && Math.abs(difference) > period / 2) difference -= Math.round(difference / period) * period;
            frame += difference * u;
        } else if (a.raw().has("frameRate")) {
            frame += a.raw().get("frameRate").getAsFloat() * (float) Math.min(0.05, (time - left.time) / 1000);
        }
        var root = smooth ? b.rootPose().interpolate(a.rootPose(), u) : a.rootPose();
        var board = smooth && a.boardPose() != null && b.boardPose() != null
            ? b.boardPose().interpolate(a.boardPose(), u) : a.boardPose();
        var data = a.raw();
        if (smooth && u > 0) {
            data = new JsonObject();
            for (var field : a.raw().entrySet()) data.add(field.getKey(), field.getValue());
            blendChannel(data, b.raw(), "hellcat", "entity", u, "steering", "lean", "throttle");
            blendChannel(data, b.raw(), "wings", "clip", u, "frame");
            blendChannel(data, b.raw(), "ride", "entity", u, "runMix");
            blendChannel(data, b.raw(), "walkMix", "clips", u, "run", "up", "side");
        }
        poses.put(id, new JakPoses.Pose(at, yaw, a.clip(), frame, a.board(), a.dark(), a.light(),
            data, 0, board, board, root, root, yaw));
        ticks.put(id, tick(left) + (tick(right) - tick(left)) * (double) u);
        if (a.raw().get("hellcat") instanceof JsonObject x) {
            int craft = x.get("entity").getAsInt();
            Vec3 body = a.position().add(dev.jakmc.player.JakPlayers.vector(x.getAsJsonArray("offset")));
            var rotation = rotation(x);
            if (smooth && b.raw().get("hellcat") instanceof JsonObject y && y.get("entity").getAsInt() == craft) {
                body = body.lerp(b.position().add(dev.jakmc.player.JakPlayers.vector(y.getAsJsonArray("offset"))), u);
                rotation.slerp(rotation(y), u);
            }
            flights.put(craft, new Flight(body, rotation));
        }
    }

    private static void blendChannel(JsonObject a, JsonObject b, String channel, String identity, float u, String... fields) {
        if (!(a.get(channel) instanceof JsonObject left) || !(b.get(channel) instanceof JsonObject right)
            || !Objects.equals(left.get(identity), right.get(identity))) return;
        var blended = left.deepCopy();
        for (String field : fields) {
            if (left.get(field) instanceof JsonPrimitive x && right.get(field) instanceof JsonPrimitive y) {
                if (field.equals("frame") && y.getAsFloat() < x.getAsFloat()) continue;
                blended.addProperty(field, Mth.lerp(u, x.getAsFloat(), y.getAsFloat()));
            }
        }
        a.add(channel, blended);
    }

    private static Quaternionf rotation(JsonObject p) {
        var q = p.getAsJsonArray("rotation");
        return new Quaternionf(q.get(0).getAsFloat(), q.get(1).getAsFloat(), q.get(2).getAsFloat(), q.get(3).getAsFloat());
    }

    public static JakPoses.Pose pose(int id) { prepare(); return poses.get(id); }
    public static Flight flight(int id) { prepare(); return flights.get(id); }
}
