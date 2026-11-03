package dev.jakmc.client;

import net.fabricmc.fabric.api.client.event.lifecycle.v1.ClientTickEvents;
import net.fabricmc.fabric.api.client.networking.v1.ClientPlayConnectionEvents;
import net.minecraft.client.Camera;
import net.minecraft.client.Minecraft;
import net.minecraft.client.multiplayer.ClientLevel;
import net.minecraft.client.renderer.fog.FogData;
import net.minecraft.client.renderer.state.level.SkyRenderState;
import net.minecraft.core.BlockPos;
import net.minecraft.util.Mth;
import net.minecraft.world.effect.MobEffects;
import net.minecraft.world.entity.LivingEntity;
import net.minecraft.world.level.Level;
import net.minecraft.world.level.LightLayer;
import net.minecraft.world.level.material.FogType;
import org.joml.Vector3f;

import static dev.jakmc.client.JakMood.*;

/** Adapt the original time-of-day palettes to Minecraft's weather clock and light engine. */
public final class JakAtmosphere {
    private static final int[][] SAMPLES={{0,0},{-16,0},{16,0},{0,-16},{0,16}};
    private static final Vector3f fog=new Vector3f(),sky=new Vector3f(),scratch=new Vector3f();
    private static ClientLevel world;
    private static float dryTarget,coldTarget,swampTarget,coverTarget;
    private static float dry,cold,swamp,rain,snow,storm,sand,cover,dust,scale,skyMix;
    private static final Vector3f blendedSky=new Vector3f();
    private static long sampleAt=Long.MIN_VALUE;

    static void initialize(){
        ClientTickEvents.END_CLIENT_TICK.register(JakAtmosphere::tick);
        ClientPlayConnectionEvents.DISCONNECT.register((handler,client)->reset(null));
    }
    private static void reset(ClientLevel level){
        world=level;sampleAt=Long.MIN_VALUE;
        dry=cold=swamp=rain=snow=storm=sand=dryTarget=coldTarget=swampTarget=coverTarget=cover=dust=0;
        skyMix=0;scale=1;
        JakSounds.stormWind(0);
    }
    private static void tick(Minecraft client){
        var level=client.level;
        if(level==null||client.player==null||!level.dimension().equals(Level.OVERWORLD)){
            if(world!=null)reset(null);
            return;
        }
        if(level!=world)reset(level);
        if(client.isPaused())return;
        var at=client.player.blockPosition();
        long now=level.getGameTime();
        if(sampleAt==Long.MIN_VALUE||now-sampleAt>=5||now<sampleAt){
            sampleAt=now;
            int count=0,d=0,c=0,s=0;
            for(int[] offset:SAMPLES){
                var pos=at.offset(offset[0],0,offset[1]);
                if(!level.hasChunkAt(pos))continue;
                var key=level.getBiome(pos).unwrapKey().orElse(null);
                if(key==null)continue;
                String name=key.identifier().getPath();
                if(name.contains("desert")||name.contains("badlands"))d++;
                else if(name.contains("swamp"))s++;
                else if(name.contains("snow")||name.contains("frozen")||name.contains("ice")||name.contains("cold")
                    ||name.contains("grove")||name.contains("jagged")||name.contains("peaks"))c++;
                else if(name.equals("deep_dark")||name.endsWith("_caves"))continue;
                count++;
            }
            if(count>0){
                dryTarget=d/(float)count;coldTarget=c/(float)count;swampTarget=s/(float)count;
            }
            coverTarget=level.hasChunkAt(at)?level.getBrightness(LightLayer.SKY,BlockPos.containing(client.player.getEyePosition()))/15f:0;
        }
        float wet=level.getRainLevel(1),thunder=level.getThunderLevel(1);
        dry=approach(dry,dryTarget);cold=approach(cold,coldTarget);swamp=approach(swamp,swampTarget);
        sand=approach(sand,wet*dryTarget);
        snow=approach(snow,wet*coldTarget);
        rain=approach(rain,wet*(1-dryTarget-coldTarget)+.35f*swampTarget*(1-wet));
        storm=approach(storm,thunder*(1-dryTarget-coldTarget));
        cover+=Math.clamp(coverTarget-cover,-.09f,.09f);
        var camera=client.gameRenderer.mainCamera();
        dust=camera.getFluidInCamera()==FogType.NONE?sand*cover:0;
        JakSounds.stormWind(sand*(.24f+.76f*cover));
        sampleMood((Math.floorMod(level.getOverworldClockTime(),24000)/1000f+6)%24);
    }
    private static float approach(float value,float target){return value+Math.clamp(target-value,-.0125f,.0125f);}
    private static Vector3f at(float[][] palette,float[][] hours,float hour,Vector3f dest){
        int index=(int)hour;
        float[] snapshot=hours[index];
        float[] a=palette[(int)snapshot[0]],b=palette[(int)snapshot[1]];
        float t=Mth.lerp(hour-index,snapshot[2],snapshot[3]);
        return dest.set(Mth.lerp(t,a[0],b[0]),Mth.lerp(t,a[1],b[1]),Mth.lerp(t,a[2],b[2]));
    }
    private static void sampleMood(float hour){
        at(VILLAGE1_FOG,DEFAULT_INTERP,hour,fog);
        fog.lerp(at(SNOW_FOG,DEFAULT_INTERP,hour,scratch),cold*.6f);
        fog.lerp(at(DESERT_FOG,JAK3_INTERP,hour,scratch),dry*.8f);
        fog.lerp(at(SANDSTORM_START_FOG,JAK3_INTERP,hour,scratch),sand*(1-sand));
        fog.lerp(at(SANDSTORM_END_FOG,JAK3_INTERP,hour,scratch),sand);
        fog.lerp(at(OVERCAST_RAIN_FOG,JAK3_INTERP,hour,scratch),rain*(1-storm)*.8f);
        fog.lerp(at(STORM_RAIN_FOG,JAK3_INTERP,hour,scratch),storm);
        fog.lerp(at(SWAMP_FOG,DEFAULT_INTERP,hour,scratch),swamp).div(255);
        scale=(1-.75f*swamp)*(1-.86f*sand)*(.88f-.16f*dry)*(1-.4f*rain)*(1-.25f*storm)*(1-.3f*snow);
        at(JAK3_SKY,JAK3_INTERP,hour,sky).div(128).min(scratch.set(1));
        sky.lerp(fog,sand);
        skyMix=Math.min(1,dry*.68f+sand*.32f);
    }
    public static void applyFog(ClientLevel level,Camera camera,FogData data){
        if(level!=world||camera.getFluidInCamera()!=FogType.NONE)return;
        if(camera.entity() instanceof LivingEntity living&&(living.hasEffect(MobEffects.BLINDNESS)
            ||living.hasEffect(MobEffects.DARKNESS)||living.hasEffect(MobEffects.NIGHT_VISION)))return;
        // Vanilla keeps its complete render-distance fog and stronger local effects.
        // Keep the first blocks clear so weather never paints over Jak or the cockpit.
        float blend=cover*(.50f+.50f*Math.max(sand,swamp));
        if(blend<=0)return;
        float end=Math.max(24,data.renderDistanceEnd*Math.max(.05f,scale));
        if(end<data.environmentalEnd){
            data.environmentalStart=Math.max(16,end*(.72f-.17f*sand));
            data.environmentalEnd=end;
            data.skyEnd=Math.min(data.skyEnd,end*3);
            data.cloudEnd=Math.min(data.cloudEnd,end*1.8f);
        }
        data.color.x=Mth.lerp(blend,data.color.x,fog.x);
        data.color.y=Mth.lerp(blend,data.color.y,fog.y);
        data.color.z=Mth.lerp(blend,data.color.z,fog.z);
    }
    public static void applySky(ClientLevel level,SkyRenderState state){
        if(level!=world||skyMix<=0||state.skyColor==null)return;
        state.skyColor=blendedSky.set(state.skyColor).lerp(sky,skyMix);
        state.rainBrightness=Mth.lerp(dry,state.rainBrightness,1-.9f*sand);
    }
    static float dust(){return dust;}
    static float distanceScale(){return scale;}
}
