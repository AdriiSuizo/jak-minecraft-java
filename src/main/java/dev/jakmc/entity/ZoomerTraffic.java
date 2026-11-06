package dev.jakmc.entity;

import dev.jakmc.JakData;
import java.util.ArrayList;
import java.util.Collections;
import java.util.List;
import java.util.Set;
import java.util.WeakHashMap;
import net.fabricmc.fabric.api.event.lifecycle.v1.ServerTickEvents;
import net.minecraft.core.BlockPos;
import net.minecraft.core.registries.Registries;
import net.minecraft.server.level.ServerLevel;
import net.minecraft.world.entity.Entity;
import net.minecraft.world.entity.EntitySpawnReason;
import net.minecraft.world.entity.EntityTypes;
import net.minecraft.world.entity.npc.villager.Villager;
import net.minecraft.world.entity.player.Player;
import net.minecraft.world.phys.Vec3;

/**
 * Haven's traffic (Rust render_anim zoomers.rs, Adrii 24/10: "tráfico dando
 * vueltas por el anillo"): zoomers with NPC drivers round a town's ring road
 * while a player is near, gone once the town is left behind.
 * <p>The final Rust build took it out again once Haven became the barrio
 * layout, whose plan has no ring at that radius; Adrii wants it back (05/11),
 * so it goes round the barrio's own streets (JakCityStructure.trafficLoop).
 * JAKMC_TRAFICO=0 (or -Djakmc.trafico=false) turns it off.
 */
public final class ZoomerTraffic {
    private ZoomerTraffic(){}
    public static final boolean ENABLED=!"0".equals(System.getenv("JAKMC_TRAFICO"))&&!"false".equals(System.getProperty("jakmc.trafico"));
    /** Traffic zoomers a town near a player keeps, and how near (blocks) it is for them; past the second they go. */
    static final int TRAFFIC=5;
    static final double TRAFFIC_NEAR=110,TRAFFIC_GONE=170;
    private static final double M=VehicleWreck.METRE;
    private static final Set<JakVehicle> FLEET=Collections.newSetFromMap(new WeakHashMap<>());

    /** A town's streets open to the traffic, and the ones leaving each junction. */
    public record Network(List<dev.jakmc.worldgen.JakCityStructure.Street> streets,double[] lanes,java.util.Map<BlockPos,List<Integer>> ways,boolean oneWay){}
    /** How far right of a street's middle the traffic keeps (blocks), where the street is wide enough. */
    static final double LANE=1.5;
    /**
     * Rust zoomers.rs Route: the points it drives through in order, the one it
     * heads for, its town's middle. stand-in: the Rust ring went round and
     * round; the barrio's streets make no ring, so at each junction it takes
     * one of the other streets leaving it at random (the original's traffic
     * picks among a junction's branches), and turns back at a dead end.
     */
    static final class Route {
        final Network net;
        final Vec3 town;
        final ArrayList<Vec3> points=new ArrayList<>();
        int next,street;
        BlockPos end;
        Route(Network net,int street,boolean forward,int from,Vec3 town){
            this.net=net;this.town=town;this.street=street;
            var s=net.streets().get(street);
            var stretch=lane(street,forward);
            points.addAll(stretch.subList(Math.min(from,stretch.size()-1),stretch.size()));
            end=junction(forward?s.to():s.from());
        }
        private void extend(net.minecraft.util.RandomSource random){
            var leaving=new ArrayList<>(net.ways().getOrDefault(end,List.of()));
            leaving.remove(Integer.valueOf(street));
            // A dead end: back the way it came.
            int pick=leaving.isEmpty()?street:leaving.get(random.nextInt(leaving.size()));
            var s=net.streets().get(pick);
            boolean forward=junction(s.from()).equals(end);
            var stretch=lane(pick,forward);
            points.addAll(stretch.subList(1,stretch.size()));
            street=pick;end=junction(forward?s.to():s.from());
        }
        /** A street's points in going order, kept to the right of its middle. */
        private List<Vec3> lane(int index,boolean forward){
            var stretch=new ArrayList<>(net.streets().get(index).points());
            if(!forward)java.util.Collections.reverse(stretch);
            Vec3 way=flat(stretch.getLast().subtract(stretch.getFirst())).normalize();
            Vec3 right=new Vec3(-way.z,0,way.x).scale(net.lanes()[index]);
            stretch.replaceAll(p->p.add(right));
            return stretch;
        }
        Vec3 target(net.minecraft.util.RandomSource random){while(next+1>=points.size())extend(random);return points.get(next);}
        Vec3 after(net.minecraft.util.RandomSource random){while(next+1>=points.size())extend(random);return points.get(next+1);}
        void advance(){if(++next>16){points.subList(0,next).clear();next=0;}}
    }
    /** Rust zoomers.rs Controls, as autopilot fills it. */
    record Controls(float steer,float throttle,float brake){static final Controls NONE=new Controls(0,0,0);}

    public static void initialize(){
        if(!ENABLED)return;
        ServerTickEvents.END_LEVEL_TICK.register(level->{
            // The towns' traffic, once a second.
            if(level.getGameTime()%20==0)manage(level);
            if(level.getGameTime()%5==0)clearPath(level);
        });
    }
    static void forget(JakVehicle car){FLEET.remove(car);}
    /** Traffic zoomers going round now (development fixtures). */
    public static long going(){return FLEET.stream().filter(z->!z.isRemoved()&&z.route!=null).count();}
    /** Each traffic zoomer: where, the point it heads for, and what stands ahead of it (development fixtures). */
    public static List<String> report(){
        var out=new ArrayList<String>();
        for(var z:FLEET){
            if(z.isRemoved()||z.route==null)continue;
            Vec3 pos=z.position(),target=z.route.target(z.getRandom());
            double yaw=Math.toRadians(z.getYRot());
            Vec3 forward=new Vec3(-Math.sin(yaw),0,Math.cos(yaw)),left=new Vec3(forward.z,0,-forward.x);
            var ahead=new StringBuilder();
            for(var e:z.level().getEntities(z,z.getBoundingBox().inflate(9*M+2))){
                Vec3 rel=flat(e.position().subtract(pos));double along=rel.dot(forward);
                if(along>0&&along<9*M&&Math.abs(rel.dot(left))<2.2*M){
                    ahead.append(e.getType().toShortString());
                    if(e instanceof Villager v)ahead.append(String.format("{%.1f,%.1f %s}",e.getX(),e.getZ(),((dev.jakmc.citizens.CitizenLifeState)v).citizenLife().debug()));
                    ahead.append(' ');
                }
            }
            var blocks=new StringBuilder();
            for(int d=1;d<=3;d++)for(int h=-2;h<=1;h++){
                var at=net.minecraft.core.BlockPos.containing(pos.add(forward.scale(d)).add(0,h,0));var st=z.level().getBlockState(at);
                if(!st.getCollisionShape(z.level(),at).isEmpty())blocks.append(d).append('/').append(h).append('=').append(net.minecraft.core.registries.BuiltInRegistries.BLOCK.getKey(st.getBlock()).getPath()).append(' ');
            }
            out.add(String.format("%.1f,%.1f,%.1f yaw%.0f street %d/%d %.1f,%.1f,%.1f ahead[%s] blocks[%s] frame %s",pos.x,pos.y,pos.z,z.getYRot(),z.route.street,z.route.net.streets().size(),
                target.x,target.y,target.z,ahead.toString().trim(),blocks.toString().trim(),z.trafficFrame()));
        }
        return out;
    }
    /** Where the traffic zoomers are (development fixtures). */
    public static List<Vec3> positions(){return FLEET.stream().filter(z->!z.isRemoved()&&z.route!=null).map(Entity::position).toList();}

    /**
     * Rust zoomers.rs autopilot: along its route point by point, slower in the
     * corners, braking for whoever stands ahead of it.
     */
    static Controls autopilot(JakVehicle z,Vec3 velocity){
        var route=z.route;
        if(route==null||route.net.streets().isEmpty())return Controls.NONE;
        Vec3 pos=z.position();
        double speed=velocity.horizontalDistance()/M;
        double yaw=Math.toRadians(z.getYRot());
        Vec3 forward=new Vec3(-Math.sin(yaw),0,Math.cos(yaw)),left=new Vec3(forward.z,0,-forward.x);
        // stand-in: the Rust ring's points were far apart and its corners wide; in the barrio's
        // narrow streets a point already passed beside it counts as reached too.
        Vec3 ahead=flat(route.target(z.getRandom()).subtract(pos));
        if(ahead.length()<3.5*M||ahead.length()<4&&ahead.dot(forward)<0)route.advance();
        Vec3 target=route.target(z.getRandom());
        Vec3 to=flat(target.subtract(pos));
        // Left is the original's positive turn.
        double turn=Math.atan2(to.dot(left),to.dot(forward));
        // The corner coming: the turn from this point's way to the next's.
        Vec3 after=flat(route.after(z.getRandom()).subtract(target));
        double bend=Math.abs(Math.atan2(after.x*to.z-after.z*to.x,after.dot(to)));
        boolean blocked=false;
        for(var e:z.level().getEntities(z,z.getBoundingBox().inflate(9*M+2),e->e instanceof JakVehicle&&!e.isRemoved()
            ||e instanceof Player p&&!p.isSpectator()||e instanceof Villager v&&v.getVehicle()==null&&v.isAlive())){
            Vec3 rel=flat(e.position().subtract(pos));
            double along=rel.dot(forward);
            if(along>0&&along<9*M&&Math.abs(rel.dot(left))<2.2*M){blocked=true;break;}
        }
        // stand-in: a street corner (a right angle) is taken at walking pace, the Rust ring's bends at 7.
        double cruise=bend>1.2||Math.abs(turn)>1.2?3.5:Math.abs(turn)>.6||bend>.6?7:12;
        return new Controls((float)Math.clamp(turn*2.5,-1,1),!blocked&&speed<cruise?1:0,blocked||speed>cruise+2.5?1:0);
    }
    private static Vec3 flat(Vec3 v){return new Vec3(v.x,0,v.z);}

    /** Rust zoomers.rs manage_traffic: keeps each town near a player in its traffic, lets that of towns left behind go. */
    static void manage(ServerLevel level){
        var haven=level.registryAccess().lookupOrThrow(Registries.STRUCTURE).getValue(JakData.id("haven"));
        if(haven==null)return;
        // stand-in: the original's towns are the ones it laid out and loaded; here
        // the Haven each player is over (at any height), its middle the box's.
        var towns=new java.util.LinkedHashMap<Vec3,Network>();
        for(var player:level.players())for(var start:level.structureManager().startsForStructure(player.getBlockX()>>4,player.getBlockZ()>>4,haven)){
            if(start==null||!start.isValid())continue;
            var box=start.getBoundingBox();
            if(player.getX()<box.minX()||player.getX()>box.maxX()+1||player.getZ()<box.minZ()||player.getZ()>box.maxZ()+1)continue;
            Vec3 centre=new Vec3(box.getCenter().getX(),0,box.getCenter().getZ());
            if(towns.keySet().stream().noneMatch(t->t.distanceTo(centre)<1)){
                var net=NETWORKS.get(start);
                if(net==null){net=network(level,dev.jakmc.worldgen.JakCityStructure.streets(start));if(net!=null)NETWORKS.put(start,net);}
                if(net!=null&&!net.streets().isEmpty())towns.put(centre,net);
            }
        }
        for(var z:new ArrayList<>(FLEET)){
            if(z.isRemoved()||z.level()!=level||z.route==null)continue;
            boolean kept=towns.keySet().stream().anyMatch(c->c.distanceTo(z.route.town)<1&&near(level,c,TRAFFIC_GONE));
            if(!kept&&!(z.getFirstPassenger() instanceof Player)){
                // Gone at once, as one sunk, its driver with it.
                z.route=null;
                z.dropDriver();
                FLEET.remove(z);
                z.discard();
            }
        }
        for(var town:towns.entrySet()){
            var centre=town.getKey();
            if(!near(level,centre,TRAFFIC_NEAR))continue;
            long going=FLEET.stream().filter(z->!z.isRemoved()&&z.route!=null&&z.route.town.distanceTo(centre)<1).count();
            var net=town.getValue();
            int n=net.streets().size();
            int count=n>=6?TRAFFIC:3;
            if(going>=count)continue;
            int tx=(int)centre.x,tz=(int)centre.z;
            for(int k=(int)going;k<count;k++){
                // Spread over the streets, half one way and half the other, from a street's middle.
                int street=(k*7+Math.abs(tx))%n;boolean forward=net.oneWay()||k%2==0;
                var points=net.streets().get(street).points();
                int middle=points.size()/2;
                Vec3 from=points.get(middle);
                if(level.players().stream().anyMatch(p->flat(p.position()).distanceTo(flat(from))<6)
                    ||FLEET.stream().anyMatch(z->!z.isRemoved()&&flat(z.position()).distanceTo(flat(from))<6))continue;
                if(!level.isPositionEntityTicking(BlockPos.containing(from)))continue;
                var route=new Route(net,street,forward,forward?middle:points.size()-1-middle,centre);
                Vec3 way=route.after(level.getRandom()).subtract(from);
                var model=JakVehicle.Model.values()[JakVehicle.Model.BIKEA.ordinal()+(k+Math.abs(tz))%6];
                var car=model.type.create(level,EntitySpawnReason.EVENT);
                if(car==null)continue;
                double hover=(model.bike()?2.3:2.5)+18.0/25;
                car.snapTo(from.x,from.y+hover*M,from.z,(float)-Math.toDegrees(Math.atan2(way.x,way.z)),0);
                if(!level.noCollision(car))continue;
                car.route=route;
                if(!level.addFreshEntity(car))continue;
                FLEET.add(car);
                driver(level,car);
            }
        }
    }
    private static final java.util.Map<net.minecraft.world.level.levelgen.structure.StructureStart,Network> NETWORKS=new WeakHashMap<>();
    /** The town's streets open in the world (clear one and two blocks over the floor all along); null till their ground is loaded. */
    public static Network network(ServerLevel level,List<dev.jakmc.worldgen.JakCityStructure.Street> streets){
        var open=new ArrayList<dev.jakmc.worldgen.JakCityStructure.Street>();
        for(var street:streets){
            int state=clear(level,street.points());
            if(state<0)return null;
            if(state>0)open.add(street);
        }
        var lanes=new double[open.size()];
        for(int i=0;i<open.size();i++){
            // Two lanes where both sides of the middle are clear too; else the middle.
            var points=open.get(i).points();
            Vec3 way=flat(points.getLast().subtract(points.getFirst())).normalize();
            Vec3 right=new Vec3(-way.z,0,way.x).scale(LANE);
            lanes[i]=clear(level,points.stream().map(p->p.add(right)).toList())>0&&clear(level,points.stream().map(p->p.subtract(right)).toList())>0?LANE:0;
        }
        var ring=ring(streets,open);
        if(ring!=null)return ring;
        // stand-in: the original's traffic keeps to its lanes both ways; a street one lane wide would
        // jam two zoomers nose to nose, so the traffic takes only the streets with room for two.
        var wide=new ArrayList<dev.jakmc.worldgen.JakCityStructure.Street>();
        for(int i=0;i<open.size();i++)if(lanes[i]>0)wide.add(open.get(i));
        wide=loops(wide);
        var wideWays=new java.util.HashMap<BlockPos,List<Integer>>();
        for(int i=0;i<wide.size();i++){
            wideWays.computeIfAbsent(junction(wide.get(i).from()),k->new ArrayList<>()).add(i);
            wideWays.computeIfAbsent(junction(wide.get(i).to()),k->new ArrayList<>()).add(i);
        }
        var wideLanes=new double[wide.size()];java.util.Arrays.fill(wideLanes,LANE);
        return new Network(List.copyOf(wide),wideLanes,java.util.Map.copyOf(wideWays),false);
    }
    /** The streets along the edge of the town's junction grid (the ring road's). */
    public static java.util.Set<dev.jakmc.worldgen.JakCityStructure.Street> ringStreets(List<dev.jakmc.worldgen.JakCityStructure.Street> plan){
        var out=new java.util.HashSet<dev.jakmc.worldgen.JakCityStructure.Street>();
        if(plan.isEmpty())return out;
        int minX=Integer.MAX_VALUE,maxX=Integer.MIN_VALUE,minZ=Integer.MAX_VALUE,maxZ=Integer.MIN_VALUE;
        for(var s:plan)for(var at:new BlockPos[]{s.from(),s.to()}){
            minX=Math.min(minX,at.getX());maxX=Math.max(maxX,at.getX());minZ=Math.min(minZ,at.getZ());maxZ=Math.max(maxZ,at.getZ());
        }
        for(var s:plan){
            var a=s.from();var b=s.to();
            boolean side=a.getX()==b.getX()?(a.getX()==minX||a.getX()==maxX):(a.getZ()==minZ||a.getZ()==maxZ);
            if(side)out.add(s);
        }
        return out;
    }
    /**
     * Jak II traffic-engine update-danger-from-target and civilian 'clear-path: a craft going past 5 m/s
     * (a pilot's danger) warns the citizens in the line it covers in 2 s, 5 m either side, and they
     * walk off it to the side they are on, up to 10 m. stand-in: the original does it for Jak's craft
     * and its civilians find their way round the nav mesh; here every craft in town does it (the
     * traffic, too, has citizens on its streets) and they walk there by Minecraft's path finding.
     */
    private static void clearPath(ServerLevel level){
        var crafts=new java.util.LinkedHashSet<JakVehicle>();
        for(var z:FLEET)if(!z.isRemoved()&&z.level()==level)crafts.add(z);
        for(var p:level.players())if(p.getVehicle() instanceof JakVehicle ridden)crafts.add(ridden);
        for(var craft:crafts){
            Vec3 velocity=craft.getDeltaMovement().scale(20);
            velocity=new Vec3(velocity.x,0,velocity.z);
            boolean traffic=FLEET.contains(craft);
            if(velocity.length()<5*M&&!traffic)continue;
            // stand-in: traffic brakes for the citizens ahead before it is fast enough to warn them,
            // so it warns the line it faces at 5 m/s at least.
            if(traffic&&velocity.length()<5*M)velocity=Vec3.directionFromRotation(0,craft.getYRot()).scale(5*M);
            Vec3 from=flat(craft.position()),reach=velocity.scale(2);
            double radius=5*M,side=10*M;
            var box=new net.minecraft.world.phys.AABB(craft.position(),craft.position().add(reach)).inflate(radius,3,radius);
            for(var villager:level.getEntitiesOfClass(Villager.class,box,v->!v.isPassenger()&&v.isAlive())){
                Vec3 at=flat(villager.position()).subtract(from);
                double t=Math.clamp(at.dot(reach)/reach.lengthSqr(),0,1);
                Vec3 off=at.subtract(reach.scale(t));
                if(off.length()>radius)continue;
                Vec3 across=new Vec3(-reach.z,0,reach.x).normalize();
                if(off.dot(across)<0)across=across.scale(-1);
                // stand-in: the original's nav mesh only offers the walkable side; here the farthest spot
                // that can be stood on, walking out from the citizen, this side first and then the other.
                Vec3 to=aside(level,villager.position(),across,off.length(),radius,side);
                if(to==null)to=aside(level,villager.position(),across.scale(-1),-off.length(),radius,side);
                if(to==null)continue;
                if(((dev.jakmc.citizens.CitizenLifeState)villager).citizenLife().stepAside(level,to))continue;
                villager.getBrain().setMemory(net.minecraft.world.entity.ai.memory.MemoryModuleType.WALK_TARGET,
                    new net.minecraft.world.entity.ai.memory.WalkTarget(to,0.7f,0));
            }
        }
    }
    /** The farthest spot from `at` along `across` to stand on, the way open from `at`, at least `radius` off the line; null if none. */
    private static Vec3 aside(ServerLevel level,Vec3 at,Vec3 across,double off,double radius,double side){
        Vec3 best=null;
        BlockPos foot=BlockPos.containing(at);
        for(double d=.5;d<=side-off+1e-6;d+=.5){
            var p=BlockPos.containing(at.add(across.scale(d)));
            BlockPos stand=null;
            for(int dy:new int[]{0,1,-1}){
                var q=new BlockPos(p.getX(),foot.getY()+dy,p.getZ());
                if(open(level,q)&&open(level,q.above())&&!level.getBlockState(q.below()).getCollisionShape(level,q.below()).isEmpty()){stand=q;break;}
            }
            if(stand==null)break;
            foot=stand;
            if(off+d>=radius)best=Vec3.atBottomCenterOf(stand);
        }
        return best;
    }
    private static boolean open(ServerLevel level,BlockPos p){return level.getBlockState(p).getCollisionShape(level,p).isEmpty();}
    /**
     * Rust zoomers.rs Route's ring road, round and round one way (Adrii 06/11: a ring road round the
     * barrio). stand-in: the streets along the edge of the town's junction grid, each turned the way
     * the ring goes and driven down its middle (some have room for one only); null unless every one
     * of them is open in the world.
     */
    static Network ring(List<dev.jakmc.worldgen.JakCityStructure.Street> plan,List<dev.jakmc.worldgen.JakCityStructure.Street> open){
        if(plan.isEmpty())return null;
        int minX=Integer.MAX_VALUE,maxX=Integer.MIN_VALUE,minZ=Integer.MAX_VALUE,maxZ=Integer.MIN_VALUE;
        for(var s:plan)for(var at:new BlockPos[]{s.from(),s.to()}){
            minX=Math.min(minX,at.getX());maxX=Math.max(maxX,at.getX());minZ=Math.min(minZ,at.getZ());maxZ=Math.max(maxZ,at.getZ());
        }
        if(maxX-minX<28||maxZ-minZ<28)return null;
        final int x0=minX,x1=maxX,z0=minZ,z1=maxZ;
        java.util.function.Predicate<BlockPos> edge=at->at.getX()==x0||at.getX()==x1||at.getZ()==z0||at.getZ()==z1;
        var along=new java.util.HashMap<BlockPos,List<dev.jakmc.worldgen.JakCityStructure.Street>>();
        int count=0;
        for(var s:open){
            var a=junction(s.from());var b=junction(s.to());
            boolean side=a.getX()==b.getX()?(a.getX()==x0||a.getX()==x1):(a.getZ()==z0||a.getZ()==z1);
            if(!side||!edge.test(a)||!edge.test(b))continue;
            along.computeIfAbsent(a,k->new ArrayList<>()).add(s);
            along.computeIfAbsent(b,k->new ArrayList<>()).add(s);
            count++;
        }
        if(count!=(x1-x0)/28*2+(z1-z0)/28*2)return null;
        var ordered=new ArrayList<dev.jakmc.worldgen.JakCityStructure.Street>();
        var start=new BlockPos(x0,0,z0);var at=start;dev.jakmc.worldgen.JakCityStructure.Street last=null;
        for(int i=0;i<count;i++){
            var here=along.get(at);
            if(here==null||here.size()!=2)return null;
            var next=here.get(0)==last?here.get(1):here.get(0);
            boolean forward=junction(next.from()).equals(at);
            var points=new ArrayList<>(next.points());
            if(!forward)java.util.Collections.reverse(points);
            ordered.add(new dev.jakmc.worldgen.JakCityStructure.Street(forward?next.from():next.to(),forward?next.to():next.from(),List.copyOf(points)));
            last=next;at=junction(forward?next.to():next.from());
        }
        if(!at.equals(start))return null;
        var ways=new java.util.HashMap<BlockPos,List<Integer>>();
        for(int i=0;i<ordered.size();i++){
            ways.computeIfAbsent(junction(ordered.get(i).from()),k->new ArrayList<>()).add(i);
            ways.computeIfAbsent(junction(ordered.get(i).to()),k->new ArrayList<>()).add(i);
        }
        return new Network(List.copyOf(ordered),new double[ordered.size()],java.util.Map.copyOf(ways),true);
    }
    /**
     * stand-in (Adrii 06/11): the original's traffic runs on a city graph with no dead ends, and the Rust
     * ring had none; the barrio's wide streets have many. Streets ending at a junction no other street
     * leaves are dropped until none is left, and the traffic keeps the largest joined part of the rest,
     * so it always has another street to take. A town whose streets make no loop keeps them all.
     */
    static ArrayList<dev.jakmc.worldgen.JakCityStructure.Street> loops(List<dev.jakmc.worldgen.JakCityStructure.Street> streets){
        var kept=new ArrayList<>(streets);
        // A street from a junction to itself is no way on.
        kept.removeIf(s->junction(s.from()).equals(junction(s.to())));
        boolean dropped=true;
        while(dropped){
            var count=new java.util.HashMap<BlockPos,Integer>();
            for(var s:kept){count.merge(junction(s.from()),1,Integer::sum);count.merge(junction(s.to()),1,Integer::sum);}
            dropped=kept.removeIf(s->count.get(junction(s.from()))<2||count.get(junction(s.to()))<2);
        }
        if(kept.isEmpty())return new ArrayList<>(streets);
        // The largest part joined through junctions.
        var parent=new java.util.HashMap<BlockPos,BlockPos>();
        java.util.function.UnaryOperator<BlockPos> root=new java.util.function.UnaryOperator<>(){
            public BlockPos apply(BlockPos at){
                var up=parent.getOrDefault(at,at);
                if(up.equals(at))return at;
                var top=apply(up);parent.put(at,top);return top;
            }
        };
        for(var s:kept)parent.put(root.apply(junction(s.from())),root.apply(junction(s.to())));
        var size=new java.util.HashMap<BlockPos,Integer>();
        for(var s:kept)size.merge(root.apply(junction(s.from())),1,Integer::sum);
        var biggest=java.util.Collections.max(size.entrySet(),java.util.Map.Entry.comparingByValue()).getKey();
        kept.removeIf(s->!root.apply(junction(s.from())).equals(biggest));
        return kept;
    }
    private static BlockPos junction(BlockPos at){return new BlockPos(at.getX(),0,at.getZ());}
    /** What first crosses the hover's way along these points (development fixtures), or "". */
    public static String blocker(ServerLevel level,List<Vec3> points){
        for(int i=0;i+1<points.size();i++){
            Vec3 from=points.get(i),to=points.get(i+1);
            int steps=(int)Math.ceil(flat(to.subtract(from)).length());
            for(int k=0;k<=steps;k++){
                var at=BlockPos.containing(from.lerp(to,steps==0?0:k/(double)steps));
                for(int h=1;h<3;h++){
                    var state=level.getBlockState(at.above(h));
                    if(!state.getCollisionShape(level,at.above(h)).isEmpty())
                        return net.minecraft.core.registries.BuiltInRegistries.BLOCK.getKey(state.getBlock()).getPath()+"@"+at.getX()+","+(at.getY()+h)+","+at.getZ();
                }
            }
        }
        return "";
    }
    /** 1 when the hover's way along these points is clear block by block, 0 when a wall crosses it, -1 when not loaded. */
    public static int clear(ServerLevel level,List<Vec3> points){
        for(int i=0;i+1<points.size();i++){
            Vec3 from=points.get(i),to=points.get(i+1);
            int steps=(int)Math.ceil(flat(to.subtract(from)).length());
            for(int k=0;k<=steps;k++){
                var at=BlockPos.containing(from.lerp(to,steps==0?0:k/(double)steps));
                if(!level.hasChunkAt(at))return -1;
                // The hover rides two blocks over the floor: a kerb or strip under it is no wall.
                for(int h=1;h<3;h++)if(!level.getBlockState(at.above(h)).getCollisionShape(level,at.above(h)).isEmpty())return 0;
            }
        }
        return 1;
    }
    private static boolean near(ServerLevel level,Vec3 centre,double reach){
        return level.players().stream().anyMatch(p->flat(p.position()).distanceTo(flat(centre))<reach);
    }
    /**
     * Rust zoomers.rs Driver::new: `vehicle-rider`'s citizen at the controls.
     * stand-in: the original draws `citizen-norm-rider` in its bike or car
     * stance with rider clothing bits; here a Haven citizen (citizen-norm)
     * sits in the seat, in its own clothes.
     */
    private static void driver(ServerLevel level,JakVehicle car){
        var villager=EntityTypes.VILLAGER.create(level,EntitySpawnReason.EVENT);
        if(villager==null)return;
        int variant=0;
        for(int i=0;i<CitizenKind.ALL.size();i++)if(CitizenKind.ALL.get(i).model().equals("citizen-norm")){variant=i;break;}
        ((CitizenState)villager).citizenVariant(variant);
        villager.snapTo(car.getX(),car.getY(),car.getZ(),car.getYRot(),0);
        if(level.addFreshEntity(villager)&&!villager.startRiding(car,true,false))villager.discard();
    }
    /**
     * Rust zoomers.rs eject_driver and jak2_villagers eject_driver: Jak II's
     * `knocked-off-vehicle`, thrown 8 m/s diagonally to its seat's side and
     * 17.5 up with the craft's speed; it gets up a citizen.
     */
    static void eject(JakVehicle car,Entity driver,Vec3 velocity){
        driver.stopRiding();
        double yaw=Math.toRadians(car.getYRot());
        Vec3 forward=new Vec3(-Math.sin(yaw),0,Math.cos(yaw)),left=new Vec3(forward.z,0,-forward.x);
        // The bikes' and cars' first seats are on their middle line or to the left (Rust kind seat y >= 0).
        double side=car.model.bike()?-1:1;
        Vec3 impulse=forward.add(left.scale(side)).normalize().scale(8);
        Vec3 metres=velocity.scale(1/M).add(impulse).add(0,17.5,0);
        driver.setDeltaMovement(metres.scale(M/20));
        if(driver instanceof CitizenState citizen)citizen.citizenHit();
    }
}
