package dev.jakmc.client.camera;

import dev.jakmc.client.JakInput;
import dev.jakmc.client.JakPoses;
import dev.jakmc.player.JakRescue;
import net.fabricmc.fabric.api.client.event.lifecycle.v1.ClientTickEvents;
import net.fabricmc.fabric.api.client.networking.v1.ClientPlayNetworking;
import net.minecraft.client.CameraType;
import net.minecraft.client.Minecraft;
import net.minecraft.world.phys.Vec3;

/** The rescue key (K) and Jak's falling-but-not-moving free; the server half is dev.jakmc.player.JakRescue. */
public final class JakRescueClient {
    static final double M=4096/6400.0;
    private static float fallStuck;
    private static int fallFrees;
    private static Vec3 fallLast=Vec3.ZERO;
    private JakRescueClient(){}

    public static void initialize(){
        ClientTickEvents.END_CLIENT_TICK.register(client->{
            while(JakInput.RESCUE.consumeClick())rescue(client);
            watch(client);
        });
    }

    /**
     * console/src/plugin/mod.rs (K requests the rescue) and render_anim/src/jak.rs update,
     * the rescue's Jak part: his camera behind him (zoom DEFAULT_ZOOM, aim 0, pitch -12).
     */
    static void rescue(Minecraft client){
        if(client.player==null||!ClientPlayNetworking.canSend(JakRescue.Request.TYPE))return;
        boolean jak=client.player.getAttachedOrCreate(dev.jakmc.JakData.ENABLED);
        ClientPlayNetworking.send(new JakRescue.Request(JakRescue.RESCUE,0));
        if(!jak)return;
        JakInput.rescueCamera();
        client.options.setCameraType(CameraType.THIRD_PERSON_BACK);
        // Rust pitch -12 looks 12 degrees down; Minecraft's xRot is positive downward.
        client.player.setXRot(12);
        fallStuck=0;fallFrees=0;
    }

    /**
     * render_anim/src/jak.rs update: falling and not moving for a second (caught where
     * every side counts as a wall): stood on what holds him, and if that does not take,
     * moved off it.
     */
    static void watch(Minecraft client){
        var player=client.player;
        var pose=player==null?null:JakPoses.get(player.getId());
        if(pose==null||pose.raw().has("hellcat")){fallStuck=0;fallFrees=0;return;} // Piloting a HellCat Jak sits in it, held "falling" by relocate: a craft at rest (a pause) is not a fall.
        Vec3 here=pose.position();
        boolean falling="falling".equals(pose.raw().get("state").getAsString());
        // One client tick is the original's frame time (time.delta_secs) here: 0.05 s.
        if(falling&&here.distanceTo(fallLast)<.01*M)fallStuck+=.05f;
        else if(!falling){fallStuck=0;fallFrees=0;}
        else fallStuck=0;
        fallLast=here;
        if(fallStuck>1){
            fallStuck=0;
            fallFrees++;
            // stand-in: the odd free sets Jak's state to Stance with ON_SURFACE; the native core has no call for that, so it waits for the next free.
            if(fallFrees%2==0)ClientPlayNetworking.send(new JakRescue.Request(JakRescue.FREE,fallFrees));
        }
    }
}
