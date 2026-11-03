package dev.jakmc.content;
/** Original gem-deploy and add-blue-motion math in seconds/metres; host sweeps the result. */
public final class GemMotion {
 public double seconds,angle,spin=1024,lastBounce=-1,suckSeconds,speed,turn;
 public int bounces;public boolean settled;public final double acceleration,direction;
 public GemMotion(double random,double direction){acceleration=80+120*random;this.direction=direction;}
 public void tick(){seconds+=.05;if(seconds>=15)settled=true;if(!settled)spin=Math.max(0,spin-90*.05);spin+=Math.clamp(110-spin,-360*.05,360*.05);angle=(angle+spin*.05)%360;}
 public double[] bounce(double[] velocity,boolean[] axes,double reflection,double damping,double unit){
  if(seconds-lastBounce>=.1){bounces++;lastBounce=seconds;}
  double impact=length(velocity)*20/unit;double[] out=velocity.clone();for(int i=0;i<3;i++)if(axes[i])out[i]*=1-(1.6+reflection*.6);out[0]*=.7;out[2]*=.7;out[1]*=.7+damping*.05;spin=Math.min(720,spin+Math.min(30,impact)*24);
  if(axes[1]&&(length(out)*20/unit<.3||seconds>=10)){settled=true;out=new double[3];}return out;
 }
 public double[] suck(double[] from,double[] target,double unit){double t=Math.clamp((suckSeconds-.75)/.25,0,1);speed+=(10+(acceleration-10)*t)*.05;suckSeconds+=.05;double[] d={from[0]-target[0],from[1]-target[1],from[2]-target[2]};double length=length(d),ratio=length>1e-9?Math.max(0,length-speed*unit*.05)/length:0;turn=Math.min(1600,turn+400*.05);double a=Math.toRadians(turn)*direction*.05,s=Math.sin(a),c=Math.cos(a);return new double[]{target[0]+(d[0]*c+d[2]*s)*ratio-from[0],target[1]+d[1]*ratio-from[1],target[2]+(d[2]*c-d[0]*s)*ratio-from[2]};}
 private static double length(double[] v){return Math.sqrt(v[0]*v[0]+v[1]*v[1]+v[2]*v[2]);}
}
