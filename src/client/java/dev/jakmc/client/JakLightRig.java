package dev.jakmc.client;

import dev.jakmc.JakData;
import java.nio.ByteBuffer;
import net.minecraft.client.Minecraft;

/**
 * Jak's light by the hour, for entity.vsh and the shine (minecraft:globals.glsl's JakLight block).
 * The rig is Rust minecraft_light.rs mc_entity_light (lights.gc's three directional slots). The shine's colour is
 * time-of-day.gc update-environment-colors over Jak 3's no-cloud table, each hour by JAK3 interp. The flashes are
 * powerups.gc target-color-effect-process, as game-info.gc sends them.
 */
public final class JakLightRig {
    private JakLightRig(){}
    /** `48 * (1.5 light + 0.5 ambient)` of *no-cloud-mood-color-table*'s eight times of day, over 128. */
    private static final float[][] ENV={{121,102,14},{126,118,93},{143,134,118},{126,118,93},{128,101,14},{31,19,48},{14,35,70},{13,50,38}};
    /** *jak3 interp table*: the hour's two times and the morph at its start and end. */
    private static final float[][] INTERP={{6,7,.2f,.4f},{6,7,.4f,.6f},{6,7,.6f,.8f},{6,7,.8f,.99f},{0,7,.99f,.8f},{0,7,.8f,.6f},{0,7,.6f,0},{0,1,0,.5f},
        {0,1,.5f,.75f},{0,1,.75f,.99f},{2,1,.99f,.5f},{2,1,.5f,0},{2,3,0,.5f},{2,3,.5f,.99f},{4,3,.99f,.84f},{4,3,.84f,.66f},{4,3,.66f,.5f},{4,3,.5f,0},
        {4,5,0,.99f},{6,5,.99f,.75f},{6,5,.75f,.5f},{6,5,.5f,.25f},{6,5,.25f,0},{6,7,0,.2f}};
    /** color-mult and color-emissive at their height, in the order of AMMO (red, yellow, blue, dark), then health, pill dark, pill light. */
    private static final float[][] FLASH={{1,.6f,.6f,.3f,0,0},{1,1,0,.3f,.3f,0},{.6f,.8f,1,0,.1f,.3f},{1,.7f,.8f,.2f,0,.1f},{.5f,1,.5f,0,.5f,0},{1,0,1,.3f,0,.3f},{5,5,5,2,2,2}};
    private static final int HEALTH=4,PILL_DARK=5,PILL_LIGHT=6;
    /** (seconds 0.2), game-info.gc's. */
    private static final float FLASH_SECONDS=.2f;
    private static final float[] env={.75f,.75f,.75f};
    private static final float[] seen=new float[7];
    private static net.minecraft.world.entity.player.Player watched;
    private static int flash=-1;
    private static long flashStart;

    /** The shine's colour now (128 a channel = 1). */
    public static float[] env(){return env;}
    public static int flashes;

    private static void watch(Minecraft client){
        var player=client.player;
        if(player==null)return;
        float[] now=new float[7];
        for(int i=0;i<4;i++)now[i]=player.getAttachedOrCreate(JakData.AMMO.get(i));
        now[HEALTH]=player.getHealth();
        now[PILL_DARK]=player.getAttachedOrCreate(JakData.DARK_ECO);
        now[PILL_LIGHT]=player.getAttachedOrCreate(JakData.LIGHT_ECO);
        if(watched==player)for(int i=0;i<7;i++)if(now[i]>=seen[i]+(i==HEALTH?2:.5f)){flash=i;flashStart=System.nanoTime();flashes++;}
        watched=player;
        System.arraycopy(now,0,seen,0,7);
    }

    public static void write(ByteBuffer buffer,int at){
        var client=Minecraft.getInstance();var level=client.level;
        float[] sun={0,1,0},key={1,1,1},mult={1,1,1},glow={0,0,0};
        float weather=1;
        if(level!=null){
            float hour=(Math.floorMod(level.getOverworldClockTime(),24000)/1000f+6)%24;
            int whole=(int)hour%24;float[] row=INTERP[whole];
            float morph=row[2]+(row[3]-row[2])*(hour-(int)hour);
            float[] from=ENV[(int)row[0]],to=ENV[(int)row[1]];
            for(int c=0;c<3;c++)env[c]=(from[c]+(to[c]-from[c])*morph)/128;
            // mc_entity_light: the sun by day, the moon by night, and the sunset's colour on the key.
            double angle=(hour-12)/24*2*Math.PI;
            float x=(float)-Math.sin(angle),y=(float)Math.cos(angle);
            float t=Math.clamp((y+.08f)/.16f,0,1),daylight=t*t*(3-2*t);
            float side=daylight>=.5f?1:-1;
            sun=new float[]{x*side,Math.max(y*side,.05f),0};
            float size=(float)Math.sqrt(sun[0]*sun[0]+sun[1]*sun[1]);sun[0]/=size;sun[1]/=size;
            // stand-in: vanilla's sunrise colour in the Rust; its orange here, by the sun's height.
            float sunset=Math.clamp(1-Math.abs(y)/.4f,0,1)*daylight;
            key=new float[]{1,1-.45f*sunset,1-.7f*sunset};
            weather=1-.8f*level.getRainLevel(1);
            watch(client);
            float age=(System.nanoTime()-flashStart)/1e9f;
            if(flash>=0&&age<FLASH_SECONDS){
                float f=age<.25f*FLASH_SECONDS?1:1-(age-.25f*FLASH_SECONDS)/(.75f*FLASH_SECONDS);
                var height=FLASH[flash];
                for(int c=0;c<3;c++){mult[c]=1+(height[c]-1)*f;glow[c]=height[c+3]*f;}
            }
        }
        put(buffer,at,sun[0],sun[1],sun[2],weather);
        put(buffer,at+16,key[0],key[1],key[2],0);
        put(buffer,at+32,env[0],env[1],env[2],0);
        put(buffer,at+48,mult[0],mult[1],mult[2],0);
        put(buffer,at+64,glow[0],glow[1],glow[2],0);
    }
    private static void put(ByteBuffer buffer,int at,float x,float y,float z,float w){
        buffer.putFloat(at,x).putFloat(at+4,y).putFloat(at+8,z).putFloat(at+12,w);
    }
}
