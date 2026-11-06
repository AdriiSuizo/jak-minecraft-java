package dev.jakmc.content;

import java.util.*;
import net.minecraft.core.Registry;
import net.minecraft.core.registries.*;
import net.minecraft.resources.*;
import net.minecraft.server.level.ServerLevel;
import net.minecraft.util.Mth;
import net.minecraft.world.item.*;
import net.minecraft.world.level.block.*;
import net.minecraft.world.level.block.state.BlockBehaviour;
import net.minecraft.world.level.block.state.properties.RailShape;
import net.minecraft.world.phys.Vec3;

/** Jak 2's grind rail (render_anim grind_rails.rs): laid and joined as Minecraft's rail is, ridden on the JET-Board. */
public final class JakGrindRail {
    private static final int ACROSS=24,BELOW=12,ABOVE=12;
    private static final double BAR_TOP=.5;
    public static Block BLOCK;
    public static Item ITEM;
    // minecraftoss registries.rs MASHUP_BLOCKS ("jak:grind_rail","minecraft:rail"), block.rs add_clone; jak_items.rs GrindRail.
    public static void initialize(){
        var id=Identifier.fromNamespaceAndPath("jak","grind_rail");
        var key=ResourceKey.create(Registries.BLOCK,id);
        var loot=ResourceKey.create(Registries.LOOT_TABLE,Identifier.fromNamespaceAndPath("jak","blocks/grind_rail"));
        BLOCK=Registry.register(BuiltInRegistries.BLOCK,key,new RailBlock(BlockBehaviour.Properties.ofLegacyCopy(Blocks.RAIL).setId(key).overrideLootTable(Optional.of(loot))));
        var itemKey=ResourceKey.create(Registries.ITEM,id);
        ITEM=Registry.register(BuiltInRegistries.ITEM,itemKey,new BlockItem(BLOCK,new Item.Properties().setId(itemKey).useBlockDescriptionPrefix()));
        JakItems.ITEMS.put("grind_rail",ITEM);
    }
    // grind_rails.rs piece: the shape's ends at the bar's top, through the middle round a corner.
    static double[][] piece(RailShape shape){
        double[] middle={.5,BAR_TOP,.5};
        return switch(shape){
            case EAST_WEST -> new double[][]{side(-1,0,0),side(1,0,0)};
            case ASCENDING_EAST -> new double[][]{side(-1,0,0),side(1,0,1)};
            case ASCENDING_WEST -> new double[][]{side(-1,0,1),side(1,0,0)};
            case ASCENDING_NORTH -> new double[][]{side(0,-1,1),side(0,1,0)};
            case ASCENDING_SOUTH -> new double[][]{side(0,-1,0),side(0,1,1)};
            case SOUTH_EAST -> new double[][]{side(0,1,0),middle,side(1,0,0)};
            case SOUTH_WEST -> new double[][]{side(0,1,0),middle,side(-1,0,0)};
            case NORTH_WEST -> new double[][]{side(0,-1,0),middle,side(-1,0,0)};
            case NORTH_EAST -> new double[][]{side(0,-1,0),middle,side(1,0,0)};
            default -> new double[][]{side(0,-1,0),side(0,1,0)};
        };
    }
    private static double[] side(double dx,double dz,double up){return new double[]{.5+dx*.5,BAR_TOP+up,.5+dz*.5};}
    private record Key(long x,long y,long z){}
    private static Key key(double[] p){return new Key(Math.round(p[0]*1e6),Math.round(p[1]*1e6),Math.round(p[2]*1e6));}
    private static boolean same(double[] a,double[] b){return Math.abs(a[0]-b[0])<=1e-6&&Math.abs(a[1]-b[1])<=1e-6&&Math.abs(a[2]-b[2])<=1e-6;}
    // grind_rails.rs join: the pieces joined where their ends meet, onward from the last end then back from the first.
    static List<List<double[]>> join(List<double[][]> all){
        var pieces=all.stream().filter(p->p.length>=2).toList();
        var atEnd=new HashMap<Key,List<Integer>>();
        for(int i=0;i<pieces.size();i++){
            atEnd.computeIfAbsent(key(pieces.get(i)[0]),k->new ArrayList<>()).add(i);
            atEnd.computeIfAbsent(key(pieces.get(i)[pieces.get(i).length-1]),k->new ArrayList<>()).add(i);
        }
        boolean[] used=new boolean[pieces.size()];
        var lines=new ArrayList<List<double[]>>();
        for(int start=0;start<pieces.size();start++){
            if(used[start])continue;
            used[start]=true;
            var line=new ArrayDeque<double[]>(Arrays.asList(pieces.get(start)));
            for(boolean forward:new boolean[]{true,false})while(true){
                double[] end=forward?line.peekLast():line.peekFirst();
                int j=-1;
                for(int k:atEnd.getOrDefault(key(end),List.of()))
                    if(!used[k]&&(same(pieces.get(k)[0],end)||same(pieces.get(k)[pieces.get(k).length-1],end))){j=k;break;}
                if(j<0)break;
                used[j]=true;
                var points=new ArrayList<>(Arrays.asList(pieces.get(j)));
                if(!same(points.get(0),end))Collections.reverse(points);
                for(var p:points.subList(1,points.size()))if(forward)line.addLast(p);else line.addFirst(p);
            }
            lines.add(List.copyOf(line));
        }
        return lines;
    }
    // grind_rails.rs features_round + minecraft_world.rs (the eight blocks' square round the feet): the rail blocks'
    // pieces and the module rails (`native`, JakAssemblies.RailAnchor) joined in one pass, in world space.
    public static List<List<Vec3>> lines(ServerLevel level,Vec3 at,List<List<Vec3>> modules){
        int fx=Math.floorDiv(Mth.floor(at.x),8)*8+4,fy=Mth.floor(at.y),fz=Math.floorDiv(Mth.floor(at.z),8)*8+4;
        var pieces=new ArrayList<double[][]>();
        for(var line:modules)pieces.add(line.stream().map(p->new double[]{p.x,p.y,p.z}).toArray(double[][]::new));
        for(int cx=(fx-ACROSS)>>4;cx<=(fx+ACROSS)>>4;cx++)for(int cz=(fz-ACROSS)>>4;cz<=(fz+ACROSS)>>4;cz++){
            var chunk=level.getChunkSource().getChunkNow(cx,cz);
            if(chunk==null)continue;
            for(int sy=(fy-BELOW)>>4;sy<=(fy+ABOVE)>>4;sy++){
                int index=chunk.getSectionIndexFromSectionY(sy);
                if(index<0||index>=chunk.getSectionsCount())continue;
                var section=chunk.getSection(index);
                if(section.hasOnlyAir()||!section.maybeHas(state->state.is(BLOCK)))continue;
                for(int x=Math.max(fx-ACROSS,cx<<4);x<=Math.min(fx+ACROSS,(cx<<4)+15);x++)
                    for(int y=Math.max(fy-BELOW,sy<<4);y<=Math.min(fy+ABOVE,(sy<<4)+15);y++)
                        for(int z=Math.max(fz-ACROSS,cz<<4);z<=Math.min(fz+ACROSS,(cz<<4)+15);z++){
                            var state=section.getBlockState(x&15,y&15,z&15);
                            if(!state.is(BLOCK))continue;
                            int bx=x,by=y,bz=z;
                            pieces.add(Arrays.stream(piece(state.getValue(RailBlock.SHAPE))).map(p->new double[]{bx+p[0],by+p[1],bz+p[2]}).toArray(double[][]::new));
                        }
            }
        }
        return join(pieces).stream().map(line->line.stream().map(p->new Vec3(p[0],p[1],p[2])).toList()).toList();
    }
}
