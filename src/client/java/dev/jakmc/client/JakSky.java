package dev.jakmc.client;

import dev.jakmc.JakData;
import dev.jakmc.client.sky.JakSkyRender;
import dev.jakmc.client.sky.JakWeatherFx;
import java.util.ArrayList;
import java.util.List;
import net.minecraft.client.Minecraft;
import net.minecraft.client.multiplayer.ClientLevel;
import net.minecraft.client.resources.sounds.SimpleSoundInstance;
import net.minecraft.client.resources.sounds.SoundInstance;
import net.minecraft.sounds.SoundEvent;
import net.minecraft.sounds.SoundSource;
import org.joml.Vector3f;

import static dev.jakmc.client.JakMood.*;

/** Jak 1's sky and Jak 3's weather over the block world: minecraft_terrain/jak_sky.rs and render_anim/jak_weather.rs. */
public final class JakSky {
 public enum Climate{RAINY,COLD,DRY,SWAMP,NONE}
 // jak_sky.rs SUN, GREEN_SUN (sky.gc): high noon (hours), tilt, rise (degrees), distance (meters).
 public static final float[] SUN={12.5f,-15,0,9950},GREEN_SUN={4,0,60,9950};
 public static final int NUM_STARS=85;
 // jak_sky.rs FLASHES (*flash0* .. *flash7*): each value two frames.
 static final float[][] FLASHES={
  {1,0,.5f,1,.5f,0,.5f,.35f,.4f,.35f,.25f,.1f,.04f},
  {1,.8f,0,1,.5f,1,.4f,.2f,.1f},
  {1,.9f,.8f,.7f,0,0,1,0,1,.5f},
  {.5f,0,1,.9f,1,.8f,.3f,0,0,.5f,.1f,.5f,.35f},
  {1,0,1,0,1,.9f,.8f,.7f,.6f,.5f,.4f,.3f,.2f,.5f,.4f,.3f,.2f,.1f},
  {1,0,1,0,1,.95f,.9f,.85f,.8f,.75f,.7f,.65f,.6f,.55f,.5f,.45f,.4f,.35f,.3f,.25f,.2f,.5f,.45f,.4f,.35f,.3f,.25f,.2f,.15f,.1f,.05f},
  {1,0,1,0,.5f,0,.5f,.35f,0,0,1,0,.2f,.1f},
  {1,.8f,.3f,0,.6f,.5f,.4f,.3f,.2f,.5f,.4f,.3f,.2f,.1f}};
 // jak_mood_tables.rs VILLAGE1_SUN (village1's mood-sun-table): the sun's colour.
 static final float[][] VILLAGE1_SUN={{255,128,0},{255,225,96},{255,225,96},{255,225,96},{255,128,0},{255,225,96},{255,225,96},{255,225,96}};
 /** jak_sky.rs shader_data: [1] sky textures and weights, [2] lightning, [3] clouds, [4] overcast, [5] desert, [6] fog. */
 public static final float[][] SHADER=new float[8][4];
 /** jak_weather.rs Sky.bolt: the flash's bolt, blocks from the camera (Minecraft axes). */
 public static final List<Vector3f> BOLT=new ArrayList<>();
 /** jak_weather.rs Star: direction (Jak axes), size, colour, fade-in seconds, birth frame. */
 public static final List<float[]> STARS=new ArrayList<>();
 public static float hour,nightFrames,dawnFrames,dt;
 public static double frames;
 private static boolean on,celestial;
 private static long weatherRng,skyRng,last;
 private static int lightningWait,flashWhich,flashFrame,thunders;
 private static final float[] SNAP=new float[4],A=new float[4],B=new float[4],C=new float[4];
 private static final long[] CELL=new long[1];
 static {reset();}
 static void initialize(){JakSkyRender.initialize();JakWeatherFx.initialize();}
 // jak_sky.rs Weather::new(0x1234_5678) and jak_weather.rs with_sky (rng 0x2545_F491 for the first player).
 private static void reset(){
  weatherRng=0x12345678L|1;weatherRandom();weatherRandom();
  skyRng=0x2545F491L;lightningWait=240;flashWhich=-1;flashFrame=0;thunders=0;frames=0;
  STARS.clear();BOLT.clear();nightFrames=dawnFrames=0;
 }
 public static boolean weatherOn(){return on;}
 public static float weatherRain(){return JakAtmosphere.rain();}
 public static float weatherSnow(){return JakAtmosphere.snow();}
 public static float weatherSwamp(){return JakAtmosphere.swamp();}
 public static float overcast(){return SHADER[4][3];}
 public static boolean celestial(){return celestial;}
 // jak_sky.rs Weather::random / jak_weather.rs Sky::random (xorshift64*).
 private static float next(long[] s){
  long x=s[0]==0?1:s[0];x^=x>>>12;x^=x<<25;x^=x>>>27;s[0]=x;
  return (float)((x*0x2545F4914F6CDD1DL)>>>40)/(1<<24);
 }
 private static float weatherRandom(){CELL[0]=weatherRng;float v=next(CELL);weatherRng=CELL[0];return v;}
 public static float skyRandom(){CELL[0]=skyRng;float v=next(CELL);skyRng=CELL[0];return v;}
 private static int range(int low,int high){return low+(int)(weatherRandom()*(high-low));}
 // jak_sky.rs hour_of: Minecraft's tick 0 is 6:00.
 public static float hourOf(double ticks){double h=(ticks/1000.0+6.0)%24.0;return (float)(h<0?h+24:h);}
 // jak_sky.rs snapshots: the two snapshots an hour blends and their weights.
 static void snapshots(float[][] table,float hour,float[] out){
  float whole=(float)Math.floor(hour),f=hour-whole;float[] s=table[Math.floorMod((int)whole,24)];
  float morph=(1-f)*s[2]+f*s[3];
  out[0]=s[0];out[2]=s[1];
  if(s[0]==s[1]){out[1]=1;out[3]=0;}else{out[1]=1-morph;out[3]=morph;}
 }
 private static float[] lerp4(float[] a,float[] b,float t,float[] out){for(int i=0;i<4;i++)out[i]=a[i]+(b[i]-a[i])*t;return out;}
 // jak_sky.rs fog_at (its colour).
 static float[] fogAt(float[][] table,float[][] interp,float hour,float[] out){
  snapshots(interp,hour,SNAP);float[] a=table[(int)SNAP[0]],b=table[(int)SNAP[2]];return lerp4(a,b,SNAP[3],out);
 }
 // jak_sky.rs sun_color: village1's mood-sun-table at the hour, 0-255.
 public static float[] sunColor(float hour,float[] out){
  snapshots(DEFAULT_INTERP,hour,SNAP);float[] a=VILLAGE1_SUN[(int)SNAP[0]],b=VILLAGE1_SUN[(int)SNAP[2]];
  for(int i=0;i<3;i++)out[i]=a[i]+(b[i]-a[i])*SNAP[3];return out;
 }
 // jak_sky.rs orbit_dir (sky-make-sun-data): a unit direction in Jak's axes, y up.
 public static Vector3f orbitDir(float[] o,float hour,Vector3f out){
  double a=Math.toRadians((hour-o[0])*15),along=Math.sin(a)*o[3],up=Math.cos(a)*o[3],tilt=Math.toRadians(o[1]),rise=Math.toRadians(o[2]);
  double y=up*Math.cos(tilt),side=up*Math.sin(tilt),z=-(along*Math.cos(rise)+side*Math.sin(rise)),x=side*Math.cos(rise)-along*Math.sin(rise);
  double l=Math.max(Math.sqrt(x*x+y*y+z*z),1e-6);return out.set((float)(x/l),(float)(y/l),(float)(z/l));
 }
 // jak_sky.rs sun_up, green_sun_up, stars_out (time-of-day-update's windows).
 public static boolean sunUp(float h){return h>=6.25f&&h<18.75f;}
 public static boolean greenSunUp(float h){return h>=21.75f||h<=10.25f;}
 public static boolean starsOut(float h){int k=(int)Math.floor(h);return k>=19||k<=5;}
 // jak_sky.rs cloud_offsets (update-sky-tng-data: 16, 32 and -21, 42 a frame over 65536).
 static void cloudOffsets(double frames,float[] out){double[] r={16,32,-21,42};for(int i=0;i<4;i++){double v=frames*r[i]/65536.0;out[i]=(float)(v-Math.floor(v));}}
 // jak_sky.rs climate_of: a biome's climate by its name.
 public static Climate climateOf(String biome){
  String n=biome.substring(biome.lastIndexOf(':')+1);
  if(n.contains("swamp"))return Climate.SWAMP;
  if(n.contains("desert")||n.contains("badlands"))return Climate.DRY;
  if(n.contains("snow")||n.contains("frozen")||n.contains("ice")||n.contains("grove")||n.contains("jagged")||n.contains("peaks")||n.contains("cold"))return Climate.COLD;
  if(n.contains("nether")||n.contains("crimson")||n.contains("warped")||n.contains("basalt")||n.contains("soul_sand")||n.contains("the_end")
   ||n.contains("end_")||n.contains("the_void")||n.contains("deep_dark")||n.contains("dripstone")||n.contains("lush_caves"))return Climate.NONE;
  return Climate.RAINY;
 }
 // jak_sky.rs Weather::flash: the lightning flash now (0 to 1).
 public static float flash(){
  if(flashWhich<0)return 0;float[] f=FLASHES[flashWhich];int k=flashFrame/2;return k<f.length?f[k]:0;
 }
 // jak_weather.rs update: one frame of the sky (called as the sky is extracted).
 public static void frame(ClientLevel level){
  long now=System.nanoTime();float step=last==0?1/60f:Math.clamp((now-last)/1e9f,0,.1f);last=now;
  var client=Minecraft.getInstance();
  boolean was=on;on=JakAtmosphere.active(level);
  // Weather belongs to Jak 3 and works without Jak 1's sky textures (jak_weather.rs sprites_in).
  celestial=on&&JakSkyRender.ready();
  if(!on){if(was)BOLT.clear();return;}
  hour=hourOf(level.getOverworldClockTime());
  if(client.isPaused()){dt=0;atmosphere();return;}
  dt=step;
  frames+=dt*60.0;
  lightning(dt);
  if(flash()<=0)BOLT.clear();
  atmosphere();
  if(celestial)stars(dt*60);
 }
 // jak_sky.rs Weather::step (its lightning: update-mood-lightning, a flash every 3 to 8 s in storms and over swamps).
 private static void lightning(float dt){
  int frames=Math.max(1,Math.round(dt*60));
  boolean lit=JakAtmosphere.storm()>.3f||JakAtmosphere.swamp()>.5f;
  if(flashWhich>=0){flashFrame+=frames;if(flashFrame/2>=FLASHES[flashWhich].length)flashWhich=-1;}
  else if(lit){
   lightningWait-=frames;
   if(lightningWait<=0){
    lightningWait=range(180,480);
    flashWhich=(int)(weatherRandom()*8)%8;flashFrame=0;
    weatherRandom();// the thunder's volume, 0.5 + 0.5 * random (jak_weather.rs plays it at its own level)
    thunder(thunders%3);thunders++;
   }
  }
 }
 // jak_weather.rs update: Event::Thunder's sound and its bolt 40 to 120 blocks off, from the clouds down, crooked.
 private static void thunder(int which){
  var client=Minecraft.getInstance();var at=client.player==null?client.gameRenderer.mainCamera().position():client.player.position();
  var event=SoundEvent.createVariableRangeEvent(JakData.id(new String[]{"sfx/thunder_a","sfx/thunder_b","sfx/thunder_c"}[which%3]));
  client.getSoundManager().play(new SimpleSoundInstance(event,SoundSource.WEATHER,1,1,SoundInstance.createUnseededRandom(),at.x,at.y,at.z));
  BOLT.clear();
  float a=skyRandom()*(float)(Math.PI*2),d=40+80*skyRandom();
  float bx=(float)Math.cos(a)*d,by=(float)Math.sin(a)*d,pz=140;
  BOLT.add(new Vector3f(bx,pz,-by));
  for(int k=0;k<12;k++){
   float drop=140f/12,wx=(skyRandom()-.5f)*9,wy=(skyRandom()-.5f)*9,w=1-k/12f;
   pz-=drop;
   // Map (x, y, z up) to Minecraft (x, z up, -y).
   BOLT.add(new Vector3f(bx+wx*w,pz,-(by+wy*w)));
  }
 }
 // jak_sky.rs Weather::atmosphere (the sky's part of it) and shader_data.
 private static void atmosphere(){
  float rain=JakAtmosphere.rain(),snow=JakAtmosphere.snow(),sand=JakAtmosphere.sand(),swamp=JakAtmosphere.swamp(),
   desert=JakAtmosphere.dry(),storm=JakAtmosphere.storm(),cold=JakAtmosphere.cold();
  float[] fog=fogAt(VILLAGE1_FOG,DEFAULT_INTERP,hour,new float[4]);
  toward(fog,SNOW_FOG,DEFAULT_INTERP,cold*.6f);
  toward(fog,DESERT_FOG,JAK3_INTERP,desert*.8f);
  toward(fog,SANDSTORM_END_FOG,JAK3_INTERP,sand);
  toward(fog,OVERCAST_RAIN_FOG,JAK3_INTERP,rain*(1-storm)*.8f);
  toward(fog,STORM_RAIN_FOG,JAK3_INTERP,storm);
  toward(fog,SWAMP_FOG,DEFAULT_INTERP,swamp);
  float[] grey=lerp4(fogAt(OVERCAST_RAIN_FOG,JAK3_INTERP,hour,A),fogAt(STORM_RAIN_FOG,JAK3_INTERP,hour,B),storm,C);
  float[] dust=fogAt(SANDSTORM_END_FOG,JAK3_INTERP,hour,A);
  float wet=.75f*rain+.6f*snow+.5f*swamp,dusty=.9f*sand;
  grey=lerp4(grey,dust,wet+dusty>0?dusty/(wet+dusty):0,B);
  snapshots(JAK3_INTERP,hour,SNAP);float[] sky3=lerp4(JAK3_SKY[(int)SNAP[0]],JAK3_SKY[(int)SNAP[2]],SNAP[3],A);
  snapshots(DEFAULT_INTERP,hour,SNAP);
  float[][] s=SHADER;
  s[0][0]=1;
  s[1][0]=SNAP[0];s[1][1]=SNAP[1];s[1][2]=SNAP[2];s[1][3]=SNAP[3];
  // Lightning lights the sky as the noon sky (village1's flash snapshot), by the flash.
  s[2][0]=2;s[2][1]=flash();
  cloudOffsets(frames,s[3]);
  s[4][0]=grey[0]/255;s[4][1]=grey[1]/255;s[4][2]=grey[2]/255;s[4][3]=Math.min(wet+dusty,.9f);
  s[5][0]=sky3[0]/128;s[5][1]=sky3[1]/128;s[5][2]=sky3[2]/128;s[5][3]=desert;
  s[6][0]=fog[0]/255;s[6][1]=fog[1]/255;s[6][2]=fog[2]/255;s[6][3]=0;
 }
 private static void toward(float[] fog,float[][] table,float[][] interp,float amount){if(amount>0)lerp4(fog,fogAt(table,interp,hour,C),Math.min(amount,1),fog);}
 // jak_weather.rs sprites_in (group-stars): from 19:00 a group a frame until num-stars, kept; gone from 6:00.
 private static void stars(float frames){
  if(starsOut(hour)){
   if(STARS.isEmpty()){
    float[][] groups={{-89,178,256},{30,59,256},{60,29,128}};
    for(int k=0;k<NUM_STARS;k++)for(float[] g:groups){
     float tilt=(float)Math.toRadians(g[0]+skyRandom()*g[1]);
     float turn=skyRandom()*(float)(Math.PI*2);
     // Mat3::from_rotation_y(turn) * Mat3::from_rotation_x(tilt) * Y
     float x=(float)(Math.sin(tilt)*Math.sin(turn)),y=(float)Math.cos(tilt),z=(float)(Math.sin(tilt)*Math.cos(turn));
     STARS.add(new float[]{x,y,z,40+40*skyRandom(),g[2],.2f+.797f*skyRandom(),k});
    }
    nightFrames=0;
   }
   nightFrames+=frames;dawnFrames=0;
  }else if(!STARS.isEmpty()){
   dawnFrames+=frames;
   if(dawnFrames>60)STARS.clear();
  }
 }
}
