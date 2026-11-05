package dev.jakmc.client.model;

import dev.jakmc.client.JakPoses;

/** Original 300 Hz hurt timers, presented on the avatar's shared render clock. */
final class JakHurtVisuals {
    record Frame(boolean hidden,float red,float green,float blue) {
        int tint(int color){
            if(red==1&&green==1&&blue==1)return color;
            int r=Math.clamp(Math.round(((color>>>16)&255)*red),0,255);
            int g=Math.clamp(Math.round(((color>>>8)&255)*green),0,255);
            int b=Math.clamp(Math.round((color&255)*blue),0,255);
            return (color&0xff000000)|(r<<16)|(g<<8)|b;
        }
    }
    static final Frame REST=new Frame(false,1,1,1);
    private static final ThreadLocal<Frame> CURRENT=ThreadLocal.withInitial(()->REST);
    static Frame current(){return CURRENT.get();}
    static void current(Frame frame){CURRENT.set(frame);}
    static Frame sample(JakPoses.Pose pose){
        if(pose==null||!pose.raw().has("hurt"))return REST;
        var fx=pose.raw().getAsJsonObject("hurt");
        long ticks=fx.get("ticks").getAsLong()+(long)(JakPoses.elapsed(pose)*300);
        boolean hidden=ticks<fx.get("invulnerableUntil").getAsLong()&&(ticks/5)%4==0;
        if(ticks<fx.get("shockUntil").getAsLong()){
            int x=(int)(ticks/5)*0x9e3779b9^0x85ebca6b;
            x^=x>>>15;x*=0x2c1b3c6d;x^=x>>>12;
            float f=.5f+1.5f*(x&0xffff)/65535f;
            return new Frame(hidden,f,f,f+.5f);
        }
        var start=fx.get("burnStart");
        if(start!=null&&!start.isJsonNull()){
            float age=(ticks-start.getAsLong())/1200f;
            if(age>=0&&age<1){float f=1-Math.min(age,1-age);return new Frame(hidden,f,f,f);}
        }
        return hidden?new Frame(true,1,1,1):REST;
    }
}
