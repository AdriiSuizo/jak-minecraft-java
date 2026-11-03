package dev.jakmc.player;

import net.minecraft.world.entity.Entity;
import net.minecraft.world.entity.player.Player;
import net.minecraft.world.level.Level;

/** A dimension's entity clock. The player and their ridden vehicle use the ordinary clock. */
public interface JakEntityClock {
    State jakmc$entityClock();

    static State of(Level level){return ((JakEntityClock)level).jakmc$entityClock();}
    static long time(Level level){return level.getGameTime()-of(level).skipped;}

    static boolean affected(Entity entity){
        return !(entity instanceof Player)&&entity.countPlayerPassengers()==0
            &&(!(entity instanceof dev.jakmc.entity.JakLeaper mount)||mount.riderId()<0);
    }

    final class State {
        private double progress;
        private long skipped;
        private float rate=1;
        private boolean advance=true;

        public void advance(){
            if(rate==1){
                advance=true;
                progress=0;
            }else{
                progress+=rate;
                advance=progress>=1;
                if(advance)progress-=1;
                else skipped++;
            }
        }

        public boolean advancing(){return advance;}
        public float partial(float partial){return rate==1?partial:(float)Math.min(1,progress+rate*partial);}
        public long skipped(){return skipped;}
        public double progress(){return progress;}
        public float rate(){return rate;}
        public void setRate(float rate){this.rate=rate;}
        public void synchronize(float rate,double progress,long skipped){
            this.rate=rate;
            this.progress=progress;
            this.skipped=skipped;
        }
    }
}
