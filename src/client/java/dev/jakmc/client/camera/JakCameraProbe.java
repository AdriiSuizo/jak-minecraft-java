package dev.jakmc.client.camera;

/**
 * Collision core of Jak's third-person camera. Not a Jak port: Adrii asked (06/11) for the camera of the
 * "Recreating Baldur's Gate 3's Camera System in UE5" series (camera collision, bounds, occlusion masking).
 * Thin walls, fences and leaves between the camera and Jak are looked through (the shader masks them);
 * only thick rock, floors and walls the camera would sit inside pull it in. Pure Java so
 * tools/JakCameraCheck.java replays corridors, stairs and brushes headless. Lengths are blocks.
 */
public final class JakCameraProbe {
    /** The sphere the camera never lets touch a block: covers the 0.05 near plane up to a 110 degree FOV. */
    public static final double HARD=.2;
    /** The look-ahead sphere that starts pulling in before HARD has to. */
    public static final double SOFT=.45;
    /** Free space the camera needs behind a wall it looks through. */
    static final double GAP=.9;
    /** Thickness along the view line the mask may look through (leaves-only stretches get more), and its hysteresis. */
    static final double THIN=2.1,THIN_LEAVES=4.5,THIN_SLACK=.35;
    /** A wall looked through must stand clear of Jak's feet: the mask keeps the floor and the lowest part of walls. */
    static final double FEET_CLEAR=.45;
    /** Blocked stretches closer than this merge: the camera cannot sit in such a gap anyway. */
    static final double MERGE=.25;
    private JakCameraProbe(){}

    /** Reused flat list of collision boxes in absolute block coordinates; `leaves` marks foliage. */
    public static final class Boxes {
        double[] v=new double[6*256];
        boolean[] leaves=new boolean[256];
        int n;
        public void clear(){n=0;}
        public int size(){return n;}
        public void add(double x0,double y0,double z0,double x1,double y1,double z1,boolean leaf){
            if(n==leaves.length){
                v=java.util.Arrays.copyOf(v,v.length*2);
                leaves=java.util.Arrays.copyOf(leaves,leaves.length*2);
            }
            int o=6*n;
            v[o]=x0;v[o+1]=y0;v[o+2]=z0;v[o+3]=x1;v[o+4]=y1;v[o+5]=z1;
            leaves[n++]=leaf;
        }
    }

    /** What one view line found: where the camera may sit and whether it looks through anything. */
    public static final class Result {
        public double distance;
        /** Thin stretches the camera looks through (the mask has to open them). */
        public int lookedThrough;
        /** True when the distance was cut short by a stretch the camera may not look through. */
        public boolean pulled;
    }

    // Scratch for the merged stretches; the camera thread is the only user.
    private static double[] runA=new double[64],runB=new double[64],runLow=new double[64];
    private static boolean[] runLeaves=new boolean[64];
    private static int[] order=new int[64];
    private static double[] hitA=new double[64],hitB=new double[64];
    private static boolean[] hitLeaves=new boolean[64];
    private static final double[] P=new double[3],V=new double[3];

    /** Shortest distance from a point to any box (capped at `cap`): the room the pivot has. */
    public static double clearance(Boxes boxes,double px,double py,double pz,double cap){
        double best=cap*cap;
        for(int i=0;i<boxes.n;i++){
            int o=6*i;
            double dx=Math.max(Math.max(boxes.v[o]-px,0),px-boxes.v[o+3]);
            double dy=Math.max(Math.max(boxes.v[o+1]-py,0),py-boxes.v[o+4]);
            double dz=Math.max(Math.max(boxes.v[o+2]-pz,0),pz-boxes.v[o+5]);
            double d=dx*dx+dy*dy+dz*dz;
            if(d<best)best=d;
        }
        return Math.sqrt(best);
    }

    /**
     * The camera line from pivot p along unit v for `desired` blocks with a sphere of radius r: the boxes grown
     * by r are slab-tested, merged into blocked stretches and walked outwards from Jak. A stretch is looked
     * through when it is thin, stands above his feet and leaves GAP free behind it; the first one that is not
     * stops the camera in front of it.
     */
    public static void decide(Boxes boxes,double px,double py,double pz,double vx,double vy,double vz,
                              double desired,double r,double feetY,boolean wasLooking,double thinBoost,Result out){
        int hits=0;
        double[] bv=boxes.v;
        P[0]=px;P[1]=py;P[2]=pz;V[0]=vx;V[1]=vy;V[2]=vz;
        double[] p=P,d=V;
        for(int i=0;i<boxes.n;i++){
            int o=6*i;
            double lo=0,hi=desired;
            boolean miss=false;
            for(int axis=0;axis<3&&!miss;axis++){
                double min=bv[o+axis]-r,max=bv[o+3+axis]+r;
                if(Math.abs(d[axis])<1e-9){
                    if(p[axis]<=min||p[axis]>=max)miss=true;
                }else{
                    double t1=(min-p[axis])/d[axis],t2=(max-p[axis])/d[axis];
                    if(t1>t2){double t=t1;t1=t2;t2=t;}
                    lo=Math.max(lo,t1);hi=Math.min(hi,t2);
                    if(lo>=hi)miss=true;
                }
            }
            if(miss)continue;
            if(hits==hitA.length)growHits();
            hitA[hits]=lo;hitB[hits]=hi;hitLeaves[hits]=boxes.leaves[i];hits++;
        }
        // Insertion sort by start (a handful of hits per frame).
        if(order.length<hits)order=new int[hitA.length];
        for(int i=0;i<hits;i++){
            int k=i;order[k]=i;
            while(k>0&&hitA[order[k-1]]>hitA[order[k]]){int t=order[k];order[k]=order[k-1];order[k-1]=t;k--;}
        }
        int runs=0;
        for(int k=0;k<hits;k++){
            int i=order[k];
            if(runs>0&&hitA[i]<=runB[runs-1]+MERGE){
                runB[runs-1]=Math.max(runB[runs-1],hitB[i]);
                runLeaves[runs-1]&=hitLeaves[i];
            }else{
                if(runs==runA.length)growRuns();
                runA[runs]=hitA[i];runB[runs]=hitB[i];runLeaves[runs]=hitLeaves[i];runs++;
            }
        }
        for(int k=0;k<runs;k++)runLow[k]=py+Math.min(vy*runA[k],vy*runB[k]);
        double slack=wasLooking?THIN_SLACK:-THIN_SLACK;
        out.distance=desired;out.lookedThrough=0;out.pulled=false;
        for(int k=0;k<runs;k++){
            double a=runA[k],b=runB[k];
            if(a>=desired)break;
            double thick=Math.max(0,b-a-2*r);
            double limit=(runLeaves[k]?THIN_LEAVES:THIN)+thinBoost+slack;
            double next=k+1<runs?Math.min(runA[k+1],desired):desired;
            boolean through=b<desired&&thick<=limit&&runLow[k]>=feetY+FEET_CLEAR+(wasLooking?-.1:0)&&next-b>=GAP;
            if(through){out.lookedThrough++;continue;}
            out.distance=a;out.pulled=true;
            return;
        }
    }

    /**
     * Whether any box crosses the segment from s to e (radius r) before `stopShort` blocks from e, at a height
     * of at least minY: the walls the shader mask would have to open to show Jak.
     */
    public static boolean blocks(Boxes boxes,double sx,double sy,double sz,double ex,double ey,double ez,
                                 double r,double stopShort,double minY){
        double dx=ex-sx,dy=ey-sy,dz=ez-sz,length=Math.sqrt(dx*dx+dy*dy+dz*dz);
        if(length<1e-6)return false;
        double end=(length-stopShort)/length;
        if(end<=0)return false;
        P[0]=sx;P[1]=sy;P[2]=sz;V[0]=dx;V[1]=dy;V[2]=dz;
        double[] b=boxes.v;
        for(int i=0;i<boxes.n;i++){
            int o=6*i;
            if(b[o+4]<minY)continue;
            double lo=0,hi=end;
            boolean miss=false;
            for(int axis=0;axis<3&&!miss;axis++){
                double min=b[o+axis]-r,max=b[o+3+axis]+r;
                if(Math.abs(V[axis])<1e-9){
                    if(P[axis]<=min||P[axis]>=max)miss=true;
                }else{
                    double t1=(min-P[axis])/V[axis],t2=(max-P[axis])/V[axis];
                    if(t1>t2){double t=t1;t1=t2;t2=t;}
                    lo=Math.max(lo,t1);hi=Math.min(hi,t2);
                    if(lo>=hi)miss=true;
                }
            }
            if(!miss&&Math.max(sy+dy*lo,sy+dy*hi)>=minY-1e-6)return true;
        }
        return false;
    }

    private static void growHits(){
        int n=hitA.length*2;
        hitA=java.util.Arrays.copyOf(hitA,n);hitB=java.util.Arrays.copyOf(hitB,n);hitLeaves=java.util.Arrays.copyOf(hitLeaves,n);
        order=new int[n];
    }

    private static void growRuns(){
        int n=runA.length*2;
        runA=java.util.Arrays.copyOf(runA,n);runB=java.util.Arrays.copyOf(runB,n);
        runLow=java.util.Arrays.copyOf(runLow,n);runLeaves=java.util.Arrays.copyOf(runLeaves,n);
    }

    /**
     * Camera bounds: the pitch offset (degrees) for a view line cut short by a ceiling, a slope or the floor.
     * The player's own pitch wins once its line is 80% free; a found tilt is kept while it stays 70% free;
     * otherwise the smallest tilt in 7 degree steps is searched, toward the horizontal before away from it,
     * taken when it frees 85% of the line or a quarter more than now. `line` gives a pitch's free distance.
     */
    public static double boundPitch(double pitch,double current,double desired,java.util.function.DoubleUnaryOperator line){
        double own=line.applyAsDouble(pitch);
        if(own>=.8*desired)return 0;
        if(current!=0&&line.applyAsDouble(pitch+current)>=.7*desired)return current;
        double best=own,choice=0;
        int toward=pitch>=0?-1:1;
        for(int k=1;k<=12&&best<.85*desired;k++){
            double offset=(k%2==1?toward:-toward)*7.0*((k+1)/2),tilted=pitch+offset;
            if(tilted<-30||tilted>80)continue;
            double t=line.applyAsDouble(tilted);
            if(t>best+.25*desired||t>=.85*desired&&t>best){best=t;choice=offset;}
        }
        return choice;
    }

    /** Minecraft's camera forward for yaw/pitch in degrees (Camera.setRotation): the camera sits along its negative. */
    public static double[] back(double yaw,double pitch){
        double y=Math.toRadians(yaw),p=Math.toRadians(pitch);
        return new double[]{Math.sin(y)*Math.cos(p),Math.sin(p),-Math.cos(y)*Math.cos(p)};
    }

    /**
     * Filtered camera distance: pulls in quickly but smoothly toward the look-ahead target, never past the hard
     * limit, waits a moment before easing back out, and follows zoom input at once while the line is free.
     */
    public static final class Rig {
        public double distance=Double.NaN;
        double hold,lastDesired=Double.NaN;
        boolean free;
        /** Pull-in and ease-out rates (1/s) and the wait before easing out (s). */
        static final double IN=16,OUT=2.6,HOLD=.35,LEAD=.35;

        public void reset(){distance=Double.NaN;hold=0;free=false;lastDesired=Double.NaN;}

        public double step(double hard,double soft,double desired,double dt){
            // The look-ahead may hold the camera at most LEAD in front of the hard limit: enough that grazing
            // a bumpy wall never clamps it, little enough that a corner it passes does not steal distance.
            double target=Math.max(Math.min(soft,hard),hard-LEAD);
            if(!Double.isFinite(distance)){
                distance=target;
            }else if(free&&target>=desired-1e-3){
                distance=target;// unobstructed: the zoom input moves the camera directly
            }else if(target<distance-1e-4){
                distance+=(target-distance)*(1-Math.exp(-IN*dt));
                hold=HOLD;
            }else if(hold>0){
                hold-=dt;
            }else{
                distance+=(target-distance)*(1-Math.exp(-OUT*dt));
                if(target-distance<1e-3)distance=target;
            }
            distance=Math.min(distance,hard);
            free=distance>=desired-1e-3;
            lastDesired=desired;
            return distance;
        }
    }
}
