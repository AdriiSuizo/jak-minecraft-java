package dev.jakmc.entity;

/**
 * The HellCat's speed gears (Rust jak_mode hellcat.rs: R2 tapped twice up,
 * L2 tapped twice down, slow / normal / fast) as the host sees them: the
 * native host steps them but does not export them, so this follows the same
 * presses frame by frame to show the gear, and gives the stand-in for the
 * Minecraft top speed (Rust jak_entities advance_vehicle: FlightEnv max_speed
 * ELYTRA_ROCKET_SPEED) the host cannot set.
 */
final class HellcatGears {
    static final float[] GEARS={.6f,1f,1.6f};
    static final int AIR_GEARS=3,START_GEAR=1;
    private static final int X=1,L2=1<<5,R2=1<<7;
    private static final long FRAME_TICKS=5,DOUBLE_TAP=(long)(.35f*300),LONG_AGO=Long.MIN_VALUE/4;
    /** m/s: STREET_TOP 122880 units/s, the boost's four times, an elytra with rockets (33.5 blocks/s at 36 map units a block, 25 a metre). */
    static final float STREET_TOP=122880f/4096,BOOST=4,ELYTRA_ROCKET_SPEED=33.5f*36/25;
    int gear=START_GEAR;
    float gearScale=1;
    private long time,xTap=LONG_AGO,squareTap=LONG_AGO;

    /** Rust hellcat.rs HellCat::step and controls, the gear part: `steps` frames, the presses on the first. */
    boolean step(int steps,boolean piloted,int edges){
        int before=gear;
        for(int s=0;s<steps;s++){
            time+=FRAME_TICKS;
            int pressed=piloted&&s==0?edges:0;
            if((pressed&R2)!=0){
                if(time-xTap<=DOUBLE_TAP){
                    if(gear+1<GEARS.length&&gear+1<AIR_GEARS)gear++;
                    xTap=LONG_AGO;
                }else xTap=time;
            }
            if((pressed&L2)!=0){
                if(time-squareTap<=DOUBLE_TAP){
                    gear=Math.max(0,gear-1);
                    squareTap=LONG_AGO;
                }else squareTap=time;
            }
            // Into a planet's air: down to the fast gear by itself.
            if(gear>=AIR_GEARS)gear=AIR_GEARS-1;
            float want=GEARS[Math.min(gear,GEARS.length-1)];
            double now=log2(Math.max(gearScale,.1f)),target=log2(want);
            gearScale=(float)Math.pow(2,now+(target-now)*Math.min(2.2/60,1));
        }
        return gear!=before;
    }
    private static double log2(double v){return Math.log(v)/Math.log(2);}
    /** Rust hellcat.rs HellCat::leave: back to the normal gear. */
    void leave(){gear=START_GEAR;gearScale=1;}
    /** Rust hellcat.rs HellCat::gear_name. */
    String name(){
        return switch(gear){
            case 0 -> "slow";
            case 1 -> "normal";
            case 2 -> "fast";
            default -> "space "+(gear-AIR_GEARS+1)+"/"+(GEARS.length-AIR_GEARS);
        };
    }
    /**
     * stand-in: the original caps the HellCat at an elytra's speed with
     * rockets (`env.max_speed`: the throttle's top and the whole velocity);
     * the host cannot set it, so the throttle is scaled so that top times
     * throttle stays under the cap (its velocity in a dive is not clamped).
     */
    float cappedThrottle(float trigger,int held){
        float pedal=trigger>0?Math.clamp((trigger-.03f)/.97f,0,1):(held&R2)!=0?1:0;
        float boost=(held&X)!=0&&pedal>0&&gear<AIR_GEARS?BOOST:1;
        float top=STREET_TOP*Math.max(gearScale,.1f)*boost;
        float k=Math.min(1,ELYTRA_ROCKET_SPEED/top);
        if(k>=1||pedal<=0)return trigger;
        return .03f+.97f*k*pedal;
    }
}
