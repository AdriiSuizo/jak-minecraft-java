package dev.jakmc.client.camera;

import java.util.HashMap;
import java.util.Map;

/**
 * Headless replay of Jak's third-person camera core (JakCameraProbe, JakCameraRoom): open ground, thin
 * walls looked through, thick rock pulling in, a 1-block corridor, brushing past single blocks and a bumpy
 * wall at 144 Hz (no snapping, no breathing), and the room flood for a house, a cave and a tree.
 * Run: java -cp build/classes/java/client tools/JakCameraCheck.java
 */
public final class JakCameraCheck {
    static int failures;

    public static void main(String[] args){
        openGround();
        thinWall();
        thickRock();
        corridor();
        brushing();
        bumpyWall();
        stairs();
        lowCave();
        rooms();
        System.out.println(failures==0?"JakCameraCheck: all passed":"JakCameraCheck: "+failures+" FAILED");
        if(failures>0)System.exit(1);
    }

    /** A sparse block world: solid full cubes, optionally leaves. */
    static final class World {
        final Map<Long,Boolean> solid=new HashMap<>();
        static long key(int x,int y,int z){return ((long)x&0x1FFFFF)<<42|((long)y&0x1FFFFF)<<21|((long)z&0x1FFFFF);}
        void set(int x,int y,int z,boolean leaf){solid.put(key(x,y,z),leaf);}
        void box(int x0,int y0,int z0,int x1,int y1,int z1){for(int x=x0;x<=x1;x++)for(int y=y0;y<=y1;y++)for(int z=z0;z<=z1;z++)set(x,y,z,false);}
        void clear(int x0,int y0,int z0,int x1,int y1,int z1){for(int x=x0;x<=x1;x++)for(int y=y0;y<=y1;y++)for(int z=z0;z<=z1;z++)solid.remove(key(x,y,z));}
        boolean is(int x,int y,int z){return solid.containsKey(key(x,y,z));}
        boolean leaf(int x,int y,int z){return Boolean.TRUE.equals(solid.get(key(x,y,z)));}
        void collect(JakCameraProbe.Boxes out,double x0,double y0,double z0,double x1,double y1,double z1){
            out.clear();
            for(int x=(int)Math.floor(x0);x<=(int)Math.floor(x1);x++)
                for(int y=(int)Math.floor(y0);y<=(int)Math.floor(y1);y++)
                    for(int z=(int)Math.floor(z0);z<=(int)Math.floor(z1);z++)
                        if(is(x,y,z))out.add(x,y,z,x+1,y+1,z+1,leaf(x,y,z));
        }
        JakCameraRoom.World room(){
            return (x,y0,z,out)->{
                int open=0,roof=0;
                for(int i=0;i<JakCameraRoom.LEVELS;i++){
                    if(!is(x,y0+i,z))open|=1<<i;
                    else if(!leaf(x,y0+i,z))roof|=1<<i;
                }
                out[0]=open;out[1]=roof;
            };
        }
    }

    /** One camera with its rig, stepping at 144 Hz. */
    static final class Camera {
        final World world;
        final JakCameraProbe.Boxes boxes=new JakCameraProbe.Boxes();
        final JakCameraProbe.Result hard=new JakCameraProbe.Result(),soft=new JakCameraProbe.Result();
        final JakCameraProbe.Rig rig=new JakCameraProbe.Rig();
        boolean looking,bounds;
        double pitchTarget,pitchOffset;
        Camera(World world){this.world=world;}

        double line(double px,double py,double pz,double yaw,double pitch,double desired,double r,double feet){
            double[] v=JakCameraProbe.back(yaw,pitch);
            double ex=px+v[0]*desired,ey=py+v[1]*desired,ez=pz+v[2]*desired,g=r+.1;
            world.collect(boxes,Math.min(px,ex)-g,Math.min(py,ey)-g,Math.min(pz,ez)-g,Math.max(px,ex)+g,Math.max(py,ey)+g,Math.max(pz,ez)+g);
            JakCameraProbe.decide(boxes,px,py,pz,v[0],v[1],v[2],desired,r,feet,looking,0,hard);
            return hard.distance;
        }

        double step(double px,double py,double pz,double yaw,double pitch,double desired,double feet){
            world.collect(boxes,px-1.3,py-1.3,pz-1.3,px+1.3,py+1.3,pz+1.3);
            double clear=JakCameraProbe.clearance(boxes,px,py,pz,1);
            double r=Math.max(.05,Math.min(JakCameraProbe.HARD,clear*.95));
            double s=Math.max(r,Math.min(JakCameraProbe.SOFT,clear-.05));
            if(bounds){
                pitchTarget=JakCameraProbe.boundPitch(pitch,pitchTarget,desired,p->line(px,py,pz,yaw,p,desired,r,feet));
                pitchOffset+=(pitchTarget-pitchOffset)*(1-Math.exp(-6/144.0));
                pitch+=pitchOffset;
            }
            double[] v=JakCameraProbe.back(yaw,pitch);
            double ex=px+v[0]*desired,ey=py+v[1]*desired,ez=pz+v[2]*desired,g=s+.1;
            world.collect(boxes,Math.min(px,ex)-g,Math.min(py,ey)-g,Math.min(pz,ez)-g,Math.max(px,ex)+g,Math.max(py,ey)+g,Math.max(pz,ez)+g);
            JakCameraProbe.decide(boxes,px,py,pz,v[0],v[1],v[2],desired,r,feet,looking,0,hard);
            JakCameraProbe.decide(boxes,px,py,pz,v[0],v[1],v[2],desired,s,feet,looking,0,soft);
            looking=hard.lookedThrough>0;
            double d=rig.step(hard.distance,soft.distance+(s-r),desired,1/144.0);
            // The camera sphere must never touch a block.
            double cx=px+v[0]*d,cy=py+v[1]*d,cz=pz+v[2]*d;
            world.collect(boxes,cx-1,cy-1,cz-1,cx+1,cy+1,cz+1);
            double camClear=JakCameraProbe.clearance(boxes,cx,cy,cz,1);
            if(camClear<r-1e-6&&d>1e-3)fail("camera sphere touches a block: clearance "+camClear+" < "+r+" at d="+d);
            return d;
        }
    }

    static void check(boolean ok,String what){
        if(!ok){fail(what);}else System.out.println("  ok  "+what);
    }
    static void fail(String what){failures++;System.out.println("  FAIL "+what);}

    // Jak stands at (0.5, 64, 0.5) facing +z (yaw 0); the camera sits behind along -z, above at pitch 20.
    static final double FEET=64,PIVOT=65.62,D=4;

    static void openGround(){
        System.out.println("open ground");
        var w=new World();
        w.box(-10,63,-10,10,63,10);
        var c=new Camera(w);
        double d=0;
        for(int i=0;i<60;i++)d=c.step(.5,PIVOT,.5,0,20,D,FEET);
        check(Math.abs(d-D)<1e-6,"camera at full distance ("+d+")");
        check(!c.looking,"nothing looked through");
    }

    static void thinWall(){
        System.out.println("thin wall behind Jak");
        var w=new World();
        w.box(-10,63,-10,10,63,10);
        w.box(-5,64,-2,5,66,-2);// a 1-thick, 3-high wall 2 blocks behind Jak
        var c=new Camera(w);
        double d=0;
        for(int i=0;i<60;i++)d=c.step(.5,PIVOT,.5,0,10,D,FEET);
        check(Math.abs(d-D)<1e-6,"camera stays out behind the wall ("+d+")");
        check(c.looking,"the wall is looked through (masked)");
    }

    static void thickRock(){
        System.out.println("thick rock behind Jak");
        var w=new World();
        w.box(-10,63,-10,10,63,10);
        w.box(-5,64,-8,5,70,-2);// 7 blocks of rock
        var c=new Camera(w);
        double d=0;
        for(int i=0;i<200;i++)d=c.step(.5,PIVOT,.5,0,10,D,FEET);
        double wall=(.5+1)/Math.cos(Math.toRadians(10));// the rock face (z -1) along the line
        check(d<wall&&d>wall-.9,"camera pulled in front of the rock ("+d+", face at "+wall+")");
        check(!c.looking,"rock is not looked through");
    }

    static void corridor(){
        System.out.println("1-block corridor");
        var w=new World();
        w.box(-3,63,-12,3,68,12);
        w.clear(0,64,-12,0,65,12);// 1 wide, 2 high tunnel along z
        var c=new Camera(w);
        double d=0;
        for(int i=0;i<200;i++)d=c.step(.5,64+1.62,.5,0,0,D,FEET);
        check(Math.abs(d-D)<1e-6,"camera straight down the corridor keeps its distance ("+d+")");
        var c2=new Camera(w);
        for(int i=0;i<200;i++)d=c2.step(.6,64+1.62,.5,0,0,D,FEET);
        check(Math.abs(d-D)<1e-6,"off-centre Jak still keeps it ("+d+")");
        var c3=new Camera(w);
        double prev=Double.NaN,maxJump=0;
        for(int i=0;i<400;i++){// the player turns the camera 90 degrees over ~2.8 s
            d=c3.step(.5,64+1.62,.5,i*90/400.0,0,D,FEET);
            if(Double.isFinite(prev)&&i>0)maxJump=Math.max(maxJump,Math.abs(d-prev));
            prev=d;
        }
        check(d<.5,"turned toward the corridor wall the camera comes in ("+d+")");
        check(maxJump<.35,"turning never snaps more than 0.35 a frame (max "+maxJump+")");
    }

    static void brushing(){
        System.out.println("brushing past single blocks");
        var w=new World();
        w.box(-30,63,-30,30,63,30);
        for(int x=-20;x<=20;x+=4)w.set(x,65,-2,false);// lone blocks at camera height
        for(int x=-18;x<=20;x+=4)w.set(x,65,-1,true);// lone leaves
        var c=new Camera(w);
        double min=D;
        for(int i=0;i<1440;i++){// Jak runs 8 blocks/s along x for 10 s
            double x=-20+i*8/144.0/ 2;
            double d=c.step(x,PIVOT,.5,0,8,D,FEET);
            if(i>20)min=Math.min(min,d);
        }
        check(min>D-1e-6,"never pulled in by lone blocks or leaves (min "+min+")");
    }

    static void bumpyWall(){
        System.out.println("bumpy cliff alongside");
        var w=new World();
        w.box(-40,63,-40,40,63,40);
        // A cliff on Jak's left whose face juts in and out every block: the camera line grazes it.
        for(int x=-30;x<=30;x++){int jut=(x&1)==0?0:1;w.box(x,64,1+jut+0,x,72,8);}
        var c=new Camera(w);
        double prev=Double.NaN,maxJump=0;int reversals=0;double lastSign=0;
        for(int i=0;i<1440;i++){
            double x=-20+i*4/144.0;
            double d=c.step(x,PIVOT,.2,-120,15,D,FEET);// camera swung toward the cliff, grazing it
            if(Double.isFinite(prev)&&i>30){
                double delta=d-prev;
                maxJump=Math.max(maxJump,Math.abs(delta));
                if(Math.abs(delta)>.02){double sign=Math.signum(delta);if(lastSign!=0&&sign!=lastSign)reversals++;lastSign=sign;}
            }
            prev=d;
        }
        check(prev<D-.5,"the cliff pulls the camera in ("+prev+")");
        check(maxJump<.35,"no snap along the cliff (max step "+maxJump+")");
        check(reversals<=20,"no breathing in and out ("+reversals+" reversals over 10 s)");
    }

    static void stairs(){
        System.out.println("walking down a filled staircase, camera up the slope behind");
        var w=new World();
        w.box(-10,50,-40,10,63,40);
        for(int k=1;k<=20;k++)w.box(-2,64,-k,2,63+k,-k);// one step up per block toward -z
        var c=new Camera(w);
        double prev=Double.NaN,maxJump=0;int reversals=0;double lastSign=0,last=0;
        for(int i=0;i<720;i++){// 5 s down from step 15 at 3 blocks/s along the slope
            double z=-15+i*3/144.0;
            if(z>0)z=0;
            double feet=64+Math.max(0,-Math.floor(z+.5));// on top of the step Jak stands in
            double d=c.step(.5,feet+1.62,z+.5,0,20,D,feet);
            if(Double.isFinite(prev)&&i>30){
                double delta=d-prev;
                maxJump=Math.max(maxJump,Math.abs(delta));
                if(Math.abs(delta)>.02){double sign=Math.signum(delta);if(lastSign!=0&&sign!=lastSign)reversals++;lastSign=sign;}
            }
            prev=d;last=d;
        }
        check(last<D-.5,"without bounds the slope pulls the camera in ("+last+")");
        check(maxJump<.5,"no snap down the stairs (max step "+maxJump+")");
        check(reversals<=12,"no breathing on the steps ("+reversals+" reversals)");
        var b=new Camera(w);b.bounds=true;
        prev=Double.NaN;maxJump=0;double min=D;
        for(int i=0;i<720;i++){
            double z=-15+i*3/144.0;
            if(z>0)z=0;
            double feet=64+Math.max(0,-Math.floor(z+.5));
            double d=b.step(.5,feet+1.62,z+.5,0,20,D,feet);
            if(Double.isFinite(prev)&&i>144){maxJump=Math.max(maxJump,Math.abs(d-prev));min=Math.min(min,d);}
            prev=d;
        }
        check(min>.6*D,"with bounds the camera rises over the slope and keeps its distance (min "+min+", tilt "+b.pitchOffset+")");
        check(maxJump<.35,"and does not snap (max step "+maxJump+")");
    }

    static void lowCave(){
        System.out.println("3-high cave, camera looking down 35 degrees");
        var w=new World();
        w.box(-20,60,-30,20,75,30);
        w.clear(-2,64,-25,2,66,25);
        var c=new Camera(w);
        double d=0;
        for(int i=0;i<300;i++)d=c.step(.5,64+1.62,.5,0,35,D,64);
        check(d<.6*D,"without bounds the ceiling pulls the camera in ("+d+")");
        var b=new Camera(w);b.bounds=true;
        for(int i=0;i<600;i++)d=b.step(.5,64+1.62,.5,0,35,D,64);
        check(d>.6*D,"with bounds it slides under the ceiling ("+d+", tilt "+b.pitchOffset+")");
    }

    static void rooms(){
        System.out.println("rooms");
        // A 7x7 house (5x5 inside), walls 3 high, flat 1-thick roof, door in the south wall.
        var w=new World();
        w.box(-20,63,-20,20,63,20);
        w.box(-3,64,-3,3,67,3);
        w.clear(-2,64,-2,2,66,2);
        w.clear(0,64,-3,0,65,-3);
        var job=new JakCameraRoom();
        job.start(0,64,0);
        JakCameraRoom.Shape shape=JakCameraRoom.PENDING;
        int frames=0;
        while(shape==JakCameraRoom.PENDING&&frames<200){shape=job.step(w.room(),JakCameraRoom.BUDGET);frames++;}
        check(shape!=null&&shape!=JakCameraRoom.PENDING,"house found ("+frames+" frames)");
        if(shape!=null&&shape!=JakCameraRoom.PENDING){
            int foot=0,rim=0;
            for(int z=0;z<JakCameraRoom.N;z++){foot+=Integer.bitCount(shape.foot[z]);rim+=Integer.bitCount(shape.rim[z]);}
            check(foot==26,"floor plan is the 25 inside columns and the doorway ("+foot+")");
            check(shape.cutY==67,"ceiling cut at the roof, y 67 ("+shape.cutY+")");
            check(rim>=24,"wall ring round it ("+rim+")");
            check(shape.foot(16,16)&&!shape.foot(16+3,16),"Jak's column in, the east wall out");
        }
        // Standing outside: no room.
        job.start(0,64,-8);
        shape=JakCameraRoom.PENDING;
        while(shape==JakCameraRoom.PENDING)shape=job.step(w.room(),JakCameraRoom.BUDGET);
        check(shape==null,"outside the house: no room");
        // A tunnel under 6 blocks of rock: a cave, left to the collision.
        var cave=new World();
        cave.box(-20,60,-20,20,75,20);
        cave.clear(-1,64,-10,1,66,10);
        job.start(0,64,0);
        shape=JakCameraRoom.PENDING;
        while(shape==JakCameraRoom.PENDING)shape=job.step(cave.room(),JakCameraRoom.BUDGET);
        check(shape==null,"cave tunnel is not a building");
        // Under a tree canopy: leaves are no roof.
        var tree=new World();
        tree.box(-20,63,-20,20,63,20);
        for(int x=-3;x<=3;x++)for(int z=-3;z<=3;z++)for(int y=67;y<=69;y++)tree.set(x,y,z,true);
        job.start(1,64,1);
        shape=JakCameraRoom.PENDING;
        while(shape==JakCameraRoom.PENDING)shape=job.step(tree.room(),JakCameraRoom.BUDGET);
        check(shape==null,"under a tree: no room");
    }
}
