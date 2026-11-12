package dev.jakmc.mixin.client;

import com.google.gson.JsonObject;
import dev.jakmc.client.MovieVehicles;
import dev.jakmc.entity.JakVehicle;
import dev.jakmc.player.JakPlayers;
import net.minecraft.network.syncher.EntityDataAccessor;
import net.minecraft.world.phys.Vec3;
import org.spongepowered.asm.mixin.Mixin;
import org.spongepowered.asm.mixin.Shadow;
import org.spongepowered.asm.mixin.injection.At;
import org.spongepowered.asm.mixin.injection.Inject;
import org.spongepowered.asm.mixin.injection.callback.CallbackInfo;

@Mixin(JakVehicle.class)
abstract class MovieVehiclePresentationMixin implements MovieVehicles.Presentation {
    @Shadow private JsonObject displayBefore;
    @Shadow private JsonObject displayAfter;
    @Shadow private Vec3 displayPosition;
    @Shadow private double displayClock;
    @Shadow private float displayFraction;
    @Shadow private double displayBob;

    @Inject(method = "onSyncedDataUpdated", at = @At("TAIL"))
    private void jakmc$recordVehicleStep(EntityDataAccessor<?> key, CallbackInfo ci) {
        var vehicle = (JakVehicle) (Object) this;
        if (vehicle.level().isClientSide()) MovieVehicles.capture(vehicle);
    }

    @Override public void jakmc$presentVehicle(JsonObject before, JsonObject after, double tick, float fraction) {
        displayBefore = before;
        displayAfter = after;
        displayClock = tick;
        displayFraction = fraction;
        displayPosition = JakPlayers.vector(before.getAsJsonArray("worldPosition"))
            .lerp(JakPlayers.vector(after.getAsJsonArray("worldPosition")), fraction);
        var vehicle = (JakVehicle) (Object) this;
        displayBob = vehicle.model.hover()
            ? Math.sin(tick / 20 * 2.1 + (vehicle.getId() * .618 % 1) * Math.TAU) * .07 / (6400.0 / 4096) : 0;
    }
}
