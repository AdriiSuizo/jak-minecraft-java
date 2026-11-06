package dev.jakmc.client.camera;

/**
 * Room collections for Jak's third-person camera (Adrii's request 06/11, after the "Recreating Baldur's
 * Gate 3's Camera System in UE5" series: room collections, fading out rooms). Not a Jak port. The room
 * is found from the blocks: a flood fill over open, roofed cells around Jak's storey, run a few columns a
 * frame. A room counts as a building when most of its ceiling is thin (at most 2 blocks with open space
 * above): then the shader hides everything over its floor plan from the ceiling up, the walls round it
 * above the ceiling, and the walls on the camera's side down to one block over the floor. Caves and
 * tunnels (thick ceilings) are left to the camera collision, so no hollow rock is ever cut open.
 */
public final class JakCameraRoom {
    /** Grid of N x N columns centred on Jak, LEVELS blocks tall from one below his feet. */
    public static final int N=32,LEVELS=24;
    /** Storey levels the flood may use (one below the feet to two above) and how far up a roof may be. */
    static final int STOREY=4,ROOF_SCAN=10,MAX_CELLS=1400,MAX_FOOT=700,THIN_ROOF=2;
    /** Columns fetched per frame. */
    public static final int BUDGET=32;

    /** Block facts for one column: bit i of open/roof is block y0+i. */
    public interface World {
        /** open: Jak and the camera fit (not a full collision cube); roof: a solid, non-leaf block overhead. */
        void column(int x,int y0,int z,int[] out);
    }

    /** A finished room: absolute grid origin, floor and cut heights, floor plan and wall ring bits. */
    public static final class Shape {
        public int ox,oz,floorY,cutY,cells;
        public final int[] foot=new int[N],rim=new int[N];

        public boolean sameAs(Shape o){
            return o!=null&&o.ox==ox&&o.oz==oz&&o.floorY==floorY&&o.cutY==cutY
                &&java.util.Arrays.equals(foot,o.foot)&&java.util.Arrays.equals(rim,o.rim);
        }
        public boolean foot(int cx,int cz){return cx>=0&&cz>=0&&cx<N&&cz<N&&(foot[cz]>>>cx&1)!=0;}
    }

    // Job state: the start cell, fetched columns and the flood queue.
    private int sx,sy,sz;
    private boolean running;
    private final int[] open=new int[N*N],roof=new int[N*N];
    private final boolean[] fetched=new boolean[N*N];
    private final boolean[] seen=new boolean[N*N*STOREY];
    private final int[] queue=new int[N*N*STOREY];
    private int head,tail,accepted,fetchedCount;
    private boolean leaky;
    private final int[] scratch=new int[2];

    public boolean running(){return running;}
    public int startX(){return sx;}
    public int startY(){return sy;}
    public int startZ(){return sz;}

    /** Starts a flood from the cell at Jak's feet. */
    public void start(int x,int y,int z){
        sx=x;sy=y;sz=z;running=true;
        java.util.Arrays.fill(fetched,false);
        java.util.Arrays.fill(seen,false);
        head=tail=accepted=fetchedCount=0;leaky=false;
        int c=index(N/2,N/2,1);
        seen[c]=true;queue[tail++]=c;
    }

    private static int index(int x,int z,int level){return (level*N+z)*N+x;}

    private boolean fetch(World world,int x,int z){
        int col=z*N+x;
        if(fetched[col])return false;
        world.column(sx-N/2+x,sy-1,sz-N/2+z,scratch);
        open[col]=scratch[0];roof[col]=scratch[1];fetched[col]=true;fetchedCount++;
        return true;
    }

    /** An open cell with a roof within ROOF_SCAN blocks above it. */
    private boolean accepts(int col,int level){
        if((open[col]>>>level&1)==0)return false;
        int above=roof[col]>>>(level+1);
        return (above&((1<<ROOF_SCAN)-1))!=0;
    }

    /**
     * Advances the flood by at most `budget` column fetches. Returns the finished room (null when Jak is
     * not in a building) once done, or `PENDING` while it still runs.
     */
    public Shape step(World world,int budget){
        if(!running)return null;
        int spent=0;
        while(head<tail){
            int c=queue[head];
            int x=c%N,z=c/N%N,level=c/(N*N);
            int col=z*N+x;
            if(!fetched[col]){
                if(spent>=budget)return PENDING;
                fetch(world,x,z);spent++;
            }
            head++;
            if(!accepts(col,level)){
                if(c==index(N/2,N/2,1)){running=false;return null;}// Jak stands in the open
                continue;
            }
            if(++accepted>=MAX_CELLS){leaky=true;break;}
            if(x==0||z==0||x==N-1||z==N-1)leaky=true;
            if(x>0)push(index(x-1,z,level));
            if(x<N-1)push(index(x+1,z,level));
            if(z>0)push(index(x,z-1,level));
            if(z<N-1)push(index(x,z+1,level));
            if(level>0)push(index(x,z,level-1));
            if(level<STOREY-1)push(index(x,z,level+1));
        }
        running=false;
        return finish();
    }

    /** Marker for a flood still in progress. */
    public static final Shape PENDING=new Shape();

    private void push(int c){if(!seen[c]){seen[c]=true;queue[tail++]=c;}}

    private Shape finish(){
        var shape=new Shape();
        shape.ox=sx-N/2;shape.oz=sz-N/2;shape.floorY=sy;shape.cells=accepted;
        int footCount=0,thin=0,cut=Integer.MAX_VALUE,innerCut=Integer.MAX_VALUE;
        int[] ceilings=new int[N*N];
        for(int z=0;z<N;z++)for(int x=0;x<N;x++){
            int col=z*N+x,top=-1;
            for(int level=STOREY-1;level>=0;level--)if(seen[index(x,z,level)]&&fetched[col]&&accepts(col,level)){top=level;break;}
            if(top<0)continue;
            shape.foot[z]|=1<<x;footCount++;
            int ceil=top+1;
            while(ceil<LEVELS&&(roof[col]>>>ceil&1)==0)ceil++;
            cut=Math.min(cut,ceil);ceilings[col]=ceil;
            int thick=0;
            while(ceil+thick<LEVELS&&((roof[col]>>>(ceil+thick)&1)!=0||(open[col]>>>(ceil+thick)&1)==0))thick++;
            if(thick<=THIN_ROOF&&ceil+thick<LEVELS)thin++;
        }
        if(accepted<6||footCount>MAX_FOOT||thin*10<footCount*6||cut==Integer.MAX_VALUE)return null;
        if(leaky&&accepted>=MAX_CELLS)return null;
        // Ceiling cut: the lowest ceiling over the room's inner columns (a doorway lintel on the edge keeps
        // its wall), the lowest anywhere for a room with no inner column, never inside Jak's own two blocks.
        for(int z=1;z<N-1;z++)for(int x=1;x<N-1;x++){
            if(!shape.foot(x,z)||!shape.foot(x-1,z)||!shape.foot(x+1,z)||!shape.foot(x,z-1)||!shape.foot(x,z+1))continue;
            innerCut=Math.min(innerCut,ceilings[z*N+x]);
        }
        shape.cutY=Math.max(sy-1+(innerCut==Integer.MAX_VALUE?cut:innerCut),sy+2);
        for(int z=0;z<N;z++){
            int ring=0;
            for(int dz=-1;dz<=1;dz++){
                int row=z+dz;
                if(row<0||row>=N)continue;
                int f=shape.foot[row];
                ring|=f|f<<1|f>>>1;
            }
            shape.rim[z]=ring&~shape.foot[z];
        }
        return shape;
    }
}
