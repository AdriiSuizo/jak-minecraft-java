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
    // minecraft_world.rs: the nine noise biomes round the eye that give the weather.
    private static final int[][] SAMPLES={{0,0},{24,0},{-24,0},{0,24},{0,-24},{17,17},{-17,17},{17,-17},{-17,-17}};
    private static final Vector3f fog=new Vector3f(),sky=new Vector3f(),scratch=new Vector3f();
    private static ClientLevel world;
    private static float dryTarget,coldTarget,swampTarget,coverTarget,rainyTarget;
    /** enhanced_grade.rs climate_of's shares of the same samples (Rainy only in vegetation biomes). */
    private static float gradeRainy,gradeDry,gradeSwamp,gradeCold;
    private static float dry,cold,swamp,rain,snow,storm,sand,cover,dust,scale,skyMix;
    private static final Vector3f blendedSky=new Vector3f();
    private static long sampleAt=Long.MIN_VALUE;

    static void initialize(){
        ClientTickEvents.END_CLIENT_TICK.register(JakAtmosphere::tick);
        ClientPlayConnectionEvents.DISCONNECT.register((handler,client)->reset(null));
    }
    private static void reset(ClientLevel level){
        world=level;sampleAt=Long.MIN_VALUE;
        dry=cold=swamp=rain=snow=storm=sand=dryTarget=coldTarget=swampTarget=coverTarget=cover=dust=rainyTarget=0;
        gradeRainy=gradeDry=gradeSwamp=gradeCold=0;
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
            // jak_sky.rs climate_of and Mix::of: each climate's share of the samples (None ones count, with no weather).
            var eye=BlockPos.containing(client.gameRenderer.mainCamera().position());
            int count=0,r=0,d=0,c=0,s=0,gr=0,gd=0,gc=0,gs=0;
            for(int[] offset:SAMPLES){
                var pos=eye.offset(offset[0],0,offset[1]);
                if(!level.hasChunkAt(pos))continue;
                var key=level.getNoiseBiome(pos.getX()>>2,pos.getY()>>2,pos.getZ()>>2).unwrapKey().orElse(null);
                if(key==null)continue;
                switch(JakSky.climateOf(key.identifier().getPath())){
                    case RAINY->r++;case DRY->d++;case COLD->c++;case SWAMP->s++;case NONE->{}
                }
                switch(dev.jakmc.client.sky.JakGrade.climateOf(key.identifier().getPath())){
                    case RAINY->gr++;case DRY->gd++;case COLD->gc++;case SWAMP->gs++;case NONE->{}
                }
                count++;
            }
            if(count>0){
                rainyTarget=r/(float)count;dryTarget=d/(float)count;coldTarget=c/(float)count;swampTarget=s/(float)count;
                gradeRainy=gr/(float)count;gradeDry=gd/(float)count;gradeCold=gc/(float)count;gradeSwamp=gs/(float)count;
            }
            coverTarget=level.hasChunkAt(at)?level.getBrightness(LightLayer.SKY,BlockPos.containing(client.player.getEyePosition()))/15f:0;
        }
        float wet=level.getRainLevel(1),thunder=level.getThunderLevel(1);
        dry=approach(dry,dryTarget);cold=approach(cold,coldTarget);swamp=approach(swamp,swampTarget);
        sand=approach(sand,wet*dryTarget);
        snow=approach(snow,wet*coldTarget);
        // jak_sky.rs Weather::step targets: rain in the wet biomes, swamps always a little; storms there.
        rain=approach(rain,wet*(rainyTarget+swampTarget)+.35f*swampTarget*(1-wet));
        storm=approach(storm,thunder*(rainyTarget+swampTarget));
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
        // Minecraft's own blindness and darkness fogs stay theirs.
        if(camera.entity() instanceof LivingEntity living&&(living.hasEffect(MobEffects.BLINDNESS)
            ||living.hasEffect(MobEffects.DARKNESS)||living.hasEffect(MobEffects.NIGHT_VISION)))return;
        // minecraft_world.rs: Jak's fog colour (village1's by the hour, then the weather's) and
        // Atmosphere::fog_distances: Minecraft's environmental fog closer by the weather's scale.
        float s=Math.clamp(scale,.05f,1);
        float end=Math.max(12,data.environmentalEnd*s);
        data.environmentalStart=Math.min(Math.max(0,data.environmentalStart*s),end*.72f);
        data.environmentalEnd=end;
        data.color.x=fog.x;data.color.y=fog.y;data.color.z=fog.z;
    }
    public static void applySky(ClientLevel level,SkyRenderState state){
        if(level!=world||skyMix<=0||state.skyColor==null)return;
        state.skyColor=blendedSky.set(state.skyColor).lerp(sky,skyMix);
        state.rainBrightness=Mth.lerp(dry,state.rainBrightness,1-.9f*sand);
    }
    /** enhanced_grade.rs target for this frame (minecraft_world.rs), eased by JakGrade; identity off Jak's weather. */
    public static void gradeFrame(){
        var level=Minecraft.getInstance().level;
        boolean on=active(level);
        float hour=on?JakSky.hourOf(level.getOverworldClockTime()):0;
        dev.jakmc.client.sky.JakGrade.frame(on,gradeRainy,gradeDry,gradeSwamp,gradeCold,hour);
    }
    static float dust(){return dust;}
    static boolean active(ClientLevel level){return level!=null&&level==world;}
    static float rain(){return rain;}static float snow(){return snow;}static float sand(){return sand;}static float swamp(){return swamp;}
    static float dry(){return dry;}static float storm(){return storm;}static float cold(){return cold;}
    static float distanceScale(){return scale;}
}
