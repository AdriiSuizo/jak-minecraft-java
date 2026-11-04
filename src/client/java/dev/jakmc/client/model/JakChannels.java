package dev.jakmc.client.model;

import com.google.gson.JsonObject;
import dev.jakmc.client.JakPoses;

final class JakChannels {
    private JakChannels() {}

    static GltfModel.Posed sample(GltfModel rig, boolean sidekick, JakPoses.Pose pose,
                                  JakPoses.BoardPose board, float frame) {
        String clip = clip(rig, sidekick, pose.clip());
        if (board != null) {
            if (Float.isFinite(board.grindFrame()) && rig.hasClip(clip)) {
                var grind=rig.localPose(clip, board.grindFrame());
                rig.resetAlignment(grind);
                return rig.resolve(grind);
            }
            if (board.stance() || !rig.hasClip(clip)) {
                String prefix = sidekick ? "daxter-" : "jakb-";
                String turn = prefix + "board-turn";
                float phase = board.phase();
                var stance = rig.localPose(turn, phase * (rig.frames(turn) - 1));
                stance = lean(rig, stance, turn + (board.ahead() >= 0 ? "-up" : "-down"),
                    Math.abs(board.ahead()), phase, true);
                stance = lean(rig, stance, turn + (board.side() >= 0 ? "-left" : "-right"),
                    Math.abs(board.side()), phase, true);
                String duck = prefix + (board.duck() >= 0 ? "board-duck-turn" : "board-air-turn");
                return rig.resolve(lean(rig, stance, duck, Math.abs(board.duck()), phase, true));
            }
        }
        if (pose.raw().get("walkMix") instanceof JsonObject mix) {
            float run = mix.get("run").getAsFloat();
            var walking = run < 1 ? gait(rig, sidekick, mix, 0, frame) : null;
            var running = run > 0 ? gait(rig, sidekick, mix, 3, frame) : null;
            if (walking != null && running != null) return rig.resolve(rig.blendLocal(walking, running, run));
            if (walking != null) return rig.resolve(walking);
            if (running != null) return rig.resolve(running);
        }
        if (rig.hasClip(clip)) return rig.pose(clip, frame);
        return rig.pose(sidekick ? "daxter-stance-loop" : "jakb-stance-loop", 0);
    }

    private static GltfModel.LocalPose gait(GltfModel rig, boolean sidekick, JsonObject mix, int start, float frame) {
        var clips = mix.getAsJsonArray("clips");
        String base = clip(rig, sidekick, clips.get(start).getAsString());
        if (!rig.hasClip(base)) return null;
        var posed = rig.localPose(base, frame);
        posed = lean(rig, posed, clip(rig, sidekick, clips.get(start + 1).getAsString()),
            Math.abs(mix.get("up").getAsFloat()), frame, false);
        return lean(rig, posed, clip(rig, sidekick, clips.get(start + 2).getAsString()),
            Math.abs(mix.get("side").getAsFloat()), frame, false);
    }

    private static GltfModel.LocalPose lean(GltfModel rig, GltfModel.LocalPose pose, String clip, float weight,
                                       float frame, boolean phase) {
        if (weight <= 0 || !rig.hasClip(clip)) return pose;
        var other = rig.localPose(clip, phase ? frame * (rig.frames(clip) - 1) : frame);
        return weight >= 1 ? other : rig.blendLocal(pose, other, weight);
    }

    private static String clip(GltfModel rig, boolean sidekick, String bodyClip) {
        if (!sidekick || !bodyClip.startsWith("jakb-")) return bodyClip;
        String suffix = bodyClip.substring(5);
        String direct = "daxter-" + suffix;
        if (rig.hasClip(direct)) return direct;
        return "daxter-" + switch (suffix) {
            case "run-to-stance-left", "run-to-stance-right", "run-to-stance-up", "run-to-stance-down" -> "run-to-stance";
            case "run-to-stance-loop-left", "run-to-stance-loop-right",
                 "run-to-stance-loop-up", "run-to-stance-loop-down" -> "run-to-stance-loop";
            case "stance-loop-left", "stance-loop-right", "stance-loop-up", "stance-loop-down" -> "stance-loop";
            case "run-right", "run-left" -> "run";
            case "walk-right", "walk-left" -> "walk";
            case "gun-hit-elec" -> "hit-elec";
            default -> suffix;
        };
    }

    record Pair(GltfModel.Posed body, GltfModel.Posed daxter) {}

    static final class Transition {
        private long session = -1;
        private long pushTick;
        private float duration;
        private Pair from, last;

        Pair blend(GltfModel body, GltfModel daxter, Pair next, JakPoses.Pose pose, float elapsed) {
            if (!(pose.raw().get("transition") instanceof JsonObject input)) return next;
            long newSession = pose.raw().get("session").getAsLong();
            long nextPush = input.get("pushTick").getAsLong();
            if (newSession != session) {
                from = null;
                last = null;
            } else if (nextPush != pushTick) {
                from = last;
                duration = input.get("pushDuration").getAsFloat();
            }
            session = newSession;
            pushTick = nextPush;
            float age = input.get("pushAge").getAsFloat() + elapsed;
            if (from != null && duration > 0 && age < duration) {
                float weight = Math.clamp(age / duration, 0, 1);
                next = new Pair(body.blend(from.body, next.body, weight), daxter.blend(from.daxter, next.daxter, weight));
            } else {
                from = null;
            }
            last = next;
            return next;
        }
    }
}
