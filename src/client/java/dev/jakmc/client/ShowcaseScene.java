package dev.jakmc.client;

import com.google.gson.JsonObject;
import dev.jakmc.JakData;
import dev.jakmc.content.JakItems;
import dev.jakmc.player.JakPlayers;
import dev.jakmc.worldgen.JakCityStructure;
import java.io.IOException;
import java.io.UncheckedIOException;
import java.nio.file.Files;
import java.util.ArrayList;
import java.util.Arrays;
import java.util.List;
import java.util.UUID;
import net.fabricmc.fabric.api.client.event.lifecycle.v1.ClientTickEvents;
import net.minecraft.client.CameraType;
import net.minecraft.client.Minecraft;
import net.minecraft.client.Screenshot;
import net.minecraft.core.BlockPos;
import net.minecraft.core.Direction;
import net.minecraft.core.Holder;
import net.minecraft.core.registries.BuiltInRegistries;
import net.minecraft.core.registries.Registries;
import net.minecraft.network.chat.Component;
import net.minecraft.resources.Identifier;
import net.minecraft.resources.ResourceKey;
import net.minecraft.server.level.ServerLevel;
import net.minecraft.server.level.ServerPlayer;
import net.minecraft.tags.BlockTags;
import net.minecraft.util.Mth;
import net.minecraft.world.InteractionHand;
import net.minecraft.world.entity.Entity;
import net.minecraft.world.entity.EntitySpawnReason;
import net.minecraft.world.entity.EntityTypes;
import net.minecraft.world.entity.LivingEntity;
import net.minecraft.world.entity.Mob;
import net.minecraft.world.item.DyeColor;
import net.minecraft.world.item.Item;
import net.minecraft.world.item.ItemStack;
import net.minecraft.world.item.context.UseOnContext;
import net.minecraft.world.level.ChunkPos;
import net.minecraft.world.level.GameType;
import net.minecraft.world.level.biome.Biomes;
import net.minecraft.world.level.block.Block;
import net.minecraft.world.level.block.Blocks;
import net.minecraft.world.level.block.ChestBlock;
import net.minecraft.world.level.block.StandingSignBlock;
import net.minecraft.world.level.block.entity.ChestBlockEntity;
import net.minecraft.world.level.block.entity.SignBlockEntity;
import net.minecraft.world.level.block.entity.SignText;
import net.minecraft.world.level.block.entity.SignTextSlot;
import net.minecraft.world.level.block.state.BlockState;
import net.minecraft.world.level.block.state.properties.RotationSegment;
import net.minecraft.world.level.levelgen.Heightmap;
import net.minecraft.world.level.levelgen.densityfunction.SamplerContext;
import net.minecraft.world.level.levelgen.structure.BoundingBox;
import net.minecraft.world.level.levelgen.structure.Structure;
import net.minecraft.world.level.levelgen.structure.StructureStart;
import net.minecraft.world.phys.BlockHitResult;
import net.minecraft.world.phys.Vec3;

/**
 * Adrii's 07/11 showcase world, built once on a normal world: a real Spargus on a desert and a real Haven on the land
 * beside it, as close as their sizes let them be, and down the corridor between them a road of numbered stations, each
 * with what it shows laid out to be used. Everything is the game's own: the cities are their structures, placed as
 * /place does and recorded in their chunks (citizens and traffic find them as they do natural ones).
 */
final class ShowcaseScene {
    /** Between stations along the road, a station's half length, and how far either side the ground is levelled. */
    private static final int SPACING=44,PAD=17,FLAT=19,BLEND=14,STATIONS=9;
    private static final int LENGTH=SPACING*STATIONS,CIRCLE=4;
    private static final long BUDGET=45_000_000L;
    private static final BlockState PATH=Blocks.SMOOTH_SANDSTONE.defaultBlockState(),EDGE=Blocks.STONE_BRICKS.defaultBlockState(),AIR=Blocks.AIR.defaultBlockState();
    private static final String[] CREATURES={"babak","lurkerpuppy","lurkercrab","bonelurker","swamp_rat","baby_spider","snow_bunny","hopper",
        "double_lurker","grunt","flitter","gekko","mantis","kg_grunt","spydroid","predator","kermit","leaper","yeti","yakow","crimson_guard"};

    private static volatile boolean running,busy;
    private static boolean pause;
    private static UUID owner;
    private static int stage,wait,step,shot;
    private static volatile InputScene.CameraView view;
    private static final JsonObject result=new JsonObject();

    private static Holder.Reference<Structure> spargusType,havenType;
    private static StructureStart spargus,haven;
    private static final List<int[]> offsets=new ArrayList<>();
    private static BlockPos desert,line;
    private static Direction forward,right;
    private static final List<int[]> chunks=new ArrayList<>();
    private static final int[] heights=new int[STATIONS];
    private static final BlockState[] floors=new BlockState[STATIONS];
    private static int station,floor;
    private static LivingEntity pig,front;
    private static BlockPos spargusStreet,havenStreet;
    private static int havenSide;

    static InputScene.CameraView camera(){return view;}
    static void initialize(){ClientTickEvents.END_CLIENT_TICK.register(ShowcaseScene::tick);}

    static void start(Minecraft client){
        if(client.player==null||client.getSingleplayerServer()==null)throw new IllegalStateException("Open the development world");
        owner=client.player.getUUID();stage=0;wait=0;step=0;shot=0;view=null;spargus=haven=null;busy=false;pig=front=null;shots.clear();
        result.entrySet().clear();chunks.clear();offsets.clear();
        pause=client.options.pauseOnLostFocus;client.options.pauseOnLostFocus=false;
        client.options.setCameraType(CameraType.THIRD_PERSON_BACK);
        running=true;
    }

    private static void tick(Minecraft client){
        if(!running||client.level==null||client.player==null||busy)return;
        if(wait>0){wait--;return;}
        if(stage==6){tour(client);return;}
        if(stage==8){finish(client);return;}
        var server=client.getSingleplayerServer();
        busy=true;
        server.execute(()->{
            try{
                var player=server.getPlayerList().getPlayer(owner);
                if(player!=null)advance(player);
            }catch(Throwable failure){
                dev.jakmc.JakMod.LOG.error("Showcase world failed",failure);
                result.addProperty("error",String.valueOf(failure));stage=8;
            }finally{busy=false;}
        });
    }

    private static void advance(ServerPlayer p){
        var level=p.level();
        long deadline=System.nanoTime()+BUDGET;
        switch(stage){
            case 0->{prepare(p);stage=1;}
            case 1->{if(search(level,deadline))begin(p);}
            case 2->{
                while(step<chunks.size()&&System.nanoTime()<deadline)city(level,chunks.get(step++));
                if(step>=chunks.size()){step=-16;measure(level);stage=3;}
            }
            case 3->{
                while(step<LENGTH+16&&System.nanoTime()<deadline)level(level,step++);
                if(step>=LENGTH+16){step=0;stage=4;}
            }
            case 4->{build(p,step++);if(step>=STATIONS){roads(level);step=0;stage=6;}}
            case 7->kick(p);
            default->{}
        }
    }

    private static void command(ServerPlayer p,String text){
        var server=p.level().getServer();
        server.getCommands().performPrefixedCommand(server.createCommandSourceStack().withSuppressedOutput(),text);
    }

    private static void prepare(ServerPlayer p){
        for(var text:List.of("time set noon","weather clear","difficulty normal","gamerule keep_inventory true","gamerule advance_time false",
            "gamerule advance_weather false","gamerule spawn_monsters false","gamerule spawn_phantoms false","gamerule spawn_patrols false",
            "gamerule spawn_wandering_traders false","gamerule respawn_radius 0"))command(p,text);
        p.setGameMode(GameType.CREATIVE);p.getAbilities().flying=true;p.onUpdateAbilities();
        var level=p.level();
        var found=level.findClosestBiome3d(biome->biome.is(Biomes.DESERT),p.blockPosition(),6400,32,64);
        if(found==null)throw new IllegalStateException("No desert in this world");
        desert=found.getFirst();
        var structures=level.registryAccess().lookupOrThrow(Registries.STRUCTURE);
        spargusType=structures.getOrThrow(ResourceKey.create(Registries.STRUCTURE,JakData.id("spargus")));
        havenType=structures.getOrThrow(ResourceKey.create(Registries.STRUCTURE,JakData.id("haven")));
        for(int x=-80;x<=80;x+=2)for(int z=-80;z<=80;z+=2)offsets.add(new int[]{x,z});
        offsets.sort(java.util.Comparator.comparingInt(o->o[0]*o[0]+o[1]*o[1]));
        step=0;
        result.addProperty("desert",desert.toShortString());
    }

    /** As /place's: this structure's own checks at this chunk, its own biomes. */
    private static StructureStart attempt(ServerLevel level,Holder.Reference<Structure> type,int x,int z){
        var generator=level.getChunkSource().getGenerator();
        var random=level.getChunkSource().randomState();
        return type.value().generate(type,level.dimension(),level.registryAccess(),generator,generator.getBiomeSource(),
            random.createClimateSampler(SamplerContext.EMPTY_UNCACHED),random,level.getStructureTemplateManager(),level.getSeed(),
            new ChunkPos(x,z),0,level,type.value().biomes()::contains);
    }

    /** The nearest Spargus to the desert with a Haven beside it across a dry corridor. */
    private static boolean search(ServerLevel level,long deadline){
        int cx=desert.getX()>>4,cz=desert.getZ()>>4;
        while(step<offsets.size()&&System.nanoTime()<deadline){
            var offset=offsets.get(step++);
            var first=attempt(level,spargusType,cx+offset[0],cz+offset[1]);
            if(!first.isValid())continue;
            var a=first.getBoundingBox();
            for(int gap=72;gap<=136;gap+=16)for(var side:Direction.Plane.HORIZONTAL)for(int across:new int[]{0,16,-16,32,-32,48,-48}){
                int edge=switch(side){case EAST->a.maxX();case WEST->a.minX();case SOUTH->a.maxZ();default->a.minZ();};
                int middle=side.getAxis()==Direction.Axis.X?(a.minZ()+a.maxZ())/2:(a.minX()+a.maxX())/2;
                int along=edge+(side.getAxisDirection()==Direction.AxisDirection.POSITIVE?1:-1)*(gap+102);
                int hx=side.getAxis()==Direction.Axis.X?along:middle+across,hz=side.getAxis()==Direction.Axis.X?middle+across:along;
                var second=attempt(level,havenType,hx>>4,hz>>4);
                if(!second.isValid())continue;
                var b=second.getBoundingBox();
                int mid=(edge+(side.getAxisDirection()==Direction.AxisDirection.POSITIVE?(side.getAxis()==Direction.Axis.X?b.minX():b.minZ()):(side.getAxis()==Direction.Axis.X?b.maxX():b.maxZ())))/2;
                int centre=(middle+(side.getAxis()==Direction.Axis.X?(b.minZ()+b.maxZ())/2:(b.minX()+b.maxX())/2))/2;
                var cross=side.getAxis()==Direction.Axis.X?new BlockPos(mid,0,centre):new BlockPos(centre,0,mid);
                // The road comes in from one side or the other, whichever is the dry land.
                for(var way:List.of(side.getClockWise(),side.getCounterClockWise())){
                    var begin=cross.relative(way,-(LENGTH-SPACING/2));
                    if(!dry(level,begin,way))continue;
                    spargus=first;haven=second;
                    forward=way;right=forward.getClockWise();line=begin;havenSide=side==right?1:-1;
                    result.addProperty("gap",gap);result.addProperty("side",side.getName());
                    return true;
                }
            }
        }
        if(step>=offsets.size())throw new IllegalStateException("No place for the two cities near "+desert.toShortString());
        return false;
    }

    private static boolean dry(ServerLevel level,BlockPos begin,Direction way){
        var generator=level.getChunkSource().getGenerator();var random=level.getChunkSource().randomState();
        int wet=0,lo=Integer.MAX_VALUE,hi=Integer.MIN_VALUE;
        for(int t=0;t<=LENGTH;t+=16){
            var at=begin.relative(way,t);
            int ground=generator.getBaseHeight(at.getX(),at.getZ(),Heightmap.Types.OCEAN_FLOOR_WG,level,random);
            int surface=generator.getBaseHeight(at.getX(),at.getZ(),Heightmap.Types.WORLD_SURFACE_WG,level,random);
            if(surface>ground)wet++;
            lo=Math.min(lo,ground);hi=Math.max(hi,ground);
        }
        return wet<=3&&hi-lo<=30;
    }

    private static void begin(ServerPlayer p){
        chunks.clear();
        for(var start:List.of(spargus,haven)){
            var box=start.getBoundingBox();
            for(int x=box.minX()>>4;x<=box.maxX()>>4;x++)for(int z=box.minZ()>>4;z<=box.maxZ()>>4;z++)chunks.add(new int[]{x,z,start==spargus?0:1});
        }
        var middle=line.relative(forward,LENGTH/2);
        p.teleportTo(middle.getX()+.5,190,middle.getZ()+.5);
        result.addProperty("spargus",spargus.getBoundingBox().getCenter().toShortString());
        result.addProperty("haven",haven.getBoundingBox().getCenter().toShortString());
        result.addProperty("chunks",chunks.size());
        step=0;stage=2;
    }

    private static void city(ServerLevel level,int[] at){
        var start=at[2]==0?spargus:haven;
        var type=at[2]==0?spargusType:havenType;
        var pos=new ChunkPos(at[0],at[1]);
        var chunk=level.getChunk(at[0],at[1]);
        start.placeInChunk(level,level.structureManager(),level.getChunkSource().getGenerator(),level.getRandom(),
            new BoundingBox(pos.getMinBlockX(),level.getMinY(),pos.getMinBlockZ(),pos.getMaxBlockX(),level.getMaxY()+1,pos.getMaxBlockZ()),pos);
        chunk.addReferenceForStructure(type.value(),ChunkPos.pack(start.getChunkPos().x(),start.getChunkPos().z()));
        if(start.getChunkPos().equals(pos))chunk.setStartForStructure(type.value(),start);
        if(at[2]==1)clear(level,pos,start.getBoundingBox());
    }

    /** Haven stands on land a desert's rock spires reach, which natural Havens never do: they go where it stands. */
    private static void clear(ServerLevel level,ChunkPos pos,BoundingBox box){
        var spire=List.of(block("jak:red_rock").getBlock(),block("jak:wasteland_cliff").getBlock());
        var at=new BlockPos.MutableBlockPos();
        for(int x=Math.max(pos.getMinBlockX(),box.minX());x<=Math.min(pos.getMaxBlockX(),box.maxX());x++)
            for(int z=Math.max(pos.getMinBlockZ(),box.minZ());z<=Math.min(pos.getMaxBlockZ(),box.maxZ());z++)
                for(int y=level.getSeaLevel();y<level.getSeaLevel()+80;y++)
                    if(spire.contains(level.getBlockState(at.set(x,y,z)).getBlock()))level.setBlock(at,AIR,2);
    }

    private static boolean city(int x,int z){
        for(var start:List.of(spargus,haven)){
            var box=start.getBoundingBox();
            if(x>=box.minX()-1&&x<=box.maxX()+1&&z>=box.minZ()-1&&z<=box.maxZ()+1)return true;
        }
        return false;
    }
    private static BlockPos cell(int u,int v,int y){return line.relative(forward,u).relative(right,v).atY(y);}
    private static int centre(int index){return SPACING/2+SPACING*index;}

    /** The first real ground of a column, under trees, plants and water. */
    private static int ground(ServerLevel level,int x,int z){
        var at=new BlockPos.MutableBlockPos(x,level.getHeight(Heightmap.Types.MOTION_BLOCKING_NO_LEAVES,x,z),z);
        while(at.getY()>level.getMinY()){
            var state=level.getBlockState(at);
            if(!state.isAir()&&!state.canBeReplaced()&&state.getFluidState().isEmpty()&&!state.is(BlockTags.LOGS)&&!state.is(BlockTags.LEAVES))break;
            at.move(0,-1,0);
        }
        return at.getY();
    }

    /** Each station's height, the median ground under it, a walkable step from the next. */
    private static void measure(ServerLevel level){
        int sea=level.getChunkSource().getGenerator().getSeaLevel();
        for(int i=0;i<STATIONS;i++){
            int[] samples=new int[81];int n=0;
            var tops=new java.util.HashMap<BlockState,Integer>();
            for(int u=-16;u<=16;u+=4)for(int v=-16;v<=16;v+=4){
                var at=cell(centre(i)+u,v,0);int top=ground(level,at.getX(),at.getZ());samples[n++]=top;
                tops.merge(level.getBlockState(at.atY(top)),1,Integer::sum);
            }
            floors[i]=java.util.Collections.max(tops.entrySet(),java.util.Map.Entry.comparingByValue()).getKey();
            Arrays.sort(samples);
            heights[i]=Math.max(samples[40],sea+1);
        }
        for(int i=1;i<STATIONS;i++)heights[i]=Mth.clamp(heights[i],heights[i-1]-4,heights[i-1]+4);
        for(int i=STATIONS-2;i>=0;i--)heights[i]=Mth.clamp(heights[i],heights[i+1]-4,heights[i+1]+4);
        result.addProperty("heights",Arrays.toString(heights));
    }
    private static int road(int u){
        int index=Mth.clamp(Math.floorDiv(u-SPACING/2,SPACING),0,STATIONS-1),next=Math.min(index+1,STATIONS-1);
        int from=centre(index)+PAD,to=centre(next)-PAD;
        if(u<=from||next==index)return heights[index];
        if(u>=to)return heights[next];
        return Math.round(Mth.lerp((u-from)/(float)(to-from),heights[index],heights[next]));
    }

    /** One slice across the road: flat under it, eased back into the land either side and past its ends. */
    private static void level(ServerLevel level,int u){
        int target=road(u);
        for(int v=-FLAT-BLEND;v<=FLAT+BLEND;v++){
            var at=cell(u,v,0);int x=at.getX(),z=at.getZ();
            if(city(x,z))continue;
            int natural=ground(level,x,z);
            float ease=Mth.clamp(Math.max(Math.abs(v)-FLAT,Math.max(-u,u-LENGTH)*2)/(float)BLEND,0,1);
            ease=ease*ease*(3-2*ease);
            int top=Math.round(Mth.lerp(ease,target,natural));
            var surface=level.getBlockState(at.atY(natural));
            if(surface.is(Blocks.SNOW)||surface.is(Blocks.ICE))surface=Blocks.DIRT.defaultBlockState();
            // Level ground is one ground: what the cut laid bare stays on the banks.
            if(ease<1)surface=floors[Mth.clamp(Math.round((u-SPACING/2)/(float)SPACING),0,STATIONS-1)];
            var fill=surface.is(Blocks.GRASS_BLOCK)?Blocks.DIRT.defaultBlockState():surface;
            for(int y=natural;y<top;y++)level.setBlock(at.atY(y),fill,2);
            level.setBlock(at.atY(top),surface,2);
            for(int y=top+1;y<=Math.max(top,natural)+26;y++)if(!level.getBlockState(at.atY(y)).isAir())level.setBlock(at.atY(y),AIR,2);
            if(Math.abs(v)<=2&&u>=0&&u<=LENGTH)level.setBlock(at.atY(top),PATH,2);
        }
    }

    // ---- a station's cells: du along the road from its middle, dv across it (+ to its right), dy above its floor ----
    private static BlockPos at(int du,int dv,int dy){return cell(centre(station)+du,dv,floor+1+dy);}
    private static void set(ServerLevel level,int du,int dv,int dy,BlockState state){level.setBlock(at(du,dv,dy),state,2);}
    private static void box(ServerLevel level,int u0,int v0,int y0,int u1,int v1,int y1,BlockState state){
        for(int u=u0;u<=u1;u++)for(int v=v0;v<=v1;v++)for(int y=y0;y<=y1;y++)set(level,u,v,y,state);
    }
    private static Item item(String id){return BuiltInRegistries.ITEM.getValue(Identifier.parse(id));}
    private static BlockState block(String id){return BuiltInRegistries.BLOCK.getValue(Identifier.parse(id)).defaultBlockState();}
    /** As the item's own click on the floor there, Jak looking along `facing`. */
    private static void use(ServerPlayer p,String id,int du,int dv,Direction facing){
        var item=item(id);
        p.setYRot(facing.toYRot());
        p.setItemInHand(InteractionHand.MAIN_HAND,new ItemStack(item));
        var below=at(du,dv,-1);
        var hit=new BlockHitResult(Vec3.atCenterOf(below).add(0,.5,0),Direction.UP,below,false);
        if(!item.useOn(new UseOnContext(p,InteractionHand.MAIN_HAND,hit)).consumesAction())result.addProperty("refused_"+id.replace(':','_')+"_"+station,du+","+dv);
        p.setItemInHand(InteractionHand.MAIN_HAND,ItemStack.EMPTY);
    }
    private static void sign(ServerLevel level,int du,int dv,Direction facing,Component... lines){
        var pos=at(du,dv,0);
        level.setBlock(pos,Blocks.OAK_SIGN.defaultBlockState().setValue(StandingSignBlock.ROTATION,RotationSegment.convertToSegment(facing)),2);
        if(level.getBlockEntity(pos) instanceof SignBlockEntity sign){
            var text=new ArrayList<Component>();
            for(int i=0;i<4;i++)text.add(i<lines.length?lines[i]:Component.empty());
            sign.setText(new SignText(text,text,DyeColor.BLACK,false),SignTextSlot.FRONT);sign.setWaxed(true);
        }
    }
    private static void sign(ServerLevel level,int du,int dv,Direction facing,String... lines){
        var parts=new Component[lines.length];
        for(int i=0;i<lines.length;i++)parts[i]=Component.literal(lines[i]);
        sign(level,du,dv,facing,parts);
    }
    private static void chest(ServerLevel level,int du,int dv,Direction facing,ItemStack... stacks){
        var pos=at(du,dv,0);
        level.setBlock(pos,Blocks.CHEST.defaultBlockState().setValue(ChestBlock.FACING,facing),2);
        if(level.getBlockEntity(pos) instanceof ChestBlockEntity chest)for(int i=0;i<stacks.length;i++)chest.setItem(i,stacks[i]);
    }
    private static void title(ServerPlayer p,String text){
        var pos=at(0,0,8);
        command(p,"summon minecraft:text_display "+pos.getX()+" "+pos.getY()+" "+pos.getZ()+" {text:{text:\""+text+"\",color:\"gold\",bold:true},billboard:\"center\",alignment:\"center\",background:0,shadow:1b,view_range:1.5f,"
            +"transformation:{left_rotation:[0f,0f,0f,1f],right_rotation:[0f,0f,0f,1f],translation:[0f,0f,0f],scale:[5f,5f,5f]}}");
    }
    private static Entity creature(ServerLevel level,String name,int du,int dv){
        var made=BuiltInRegistries.ENTITY_TYPE.getValue(JakData.id(name)).create(level,EntitySpawnReason.COMMAND);
        if(made==null){result.addProperty("missing_"+name,true);return null;}
        var spot=Vec3.atBottomCenterOf(at(du,dv,0));
        made.snapTo(spot.x,spot.y,spot.z,forward.getOpposite().toYRot(),0);
        if(made instanceof Mob mob)mob.setPersistenceRequired();
        level.addFreshEntity(made);
        return made;
    }
    private static void fence(ServerLevel level,int u0,int v0,int u1,int v1,int gate){
        var post=Blocks.OAK_FENCE.defaultBlockState();
        for(int u=u0;u<=u1;u++)for(int v=v0;v<=v1;v++){
            if(u!=u0&&u!=u1&&v!=v0&&v!=v1)continue;
            for(int y=0;y<2;y++)set(level,u,v,y,post);
        }
    }

    private static void build(ServerPlayer p,int index){
        var level=p.level();
        station=index;floor=heights[index];
        // The pad: the floor all level, its edge in brick.
        for(int u=-PAD;u<=PAD;u++)for(int v=-FLAT+1;v<=FLAT-1;v++){
            boolean edge=Math.abs(u)==PAD||Math.abs(v)==FLAT-1;
            boolean walk=Math.abs(v)<=2;
            if(edge&&!walk)set(level,u,v,-1,EDGE);
            else if(!walk&&index==1)set(level,u,v,-1,Blocks.GRASS_BLOCK.defaultBlockState());
        }
        var back=forward.getOpposite();
        switch(index){
            case 0->start(p,back);
            case 1->moves(p,back);
            case 2->objects(p,back);
            case 3->eco(p,back);
            case 4->guns(p,back);
            case 5->powers(p,back);
            case 6->zoo(p,back);
            case 7->garage(p,back);
            default->cities(p,back);
        }
    }
    private static void start(ServerPlayer p,Direction back){
        var level=p.level();
        title(p,"Jincana de Jak");
        sign(level,-6,3,back,"JINCANA DE JAK","Sigue el camino","estación a estación");
        sign(level,-6,-3,back,"Spargus a un lado","Villarefugio","al otro");
    }
    private static void moves(ServerPlayer p,Direction back){
        var level=p.level();
        title(p,"1 · Patada y movimientos");
        sign(level,-15,3,back,"1 Movimientos","Patada giratoria:","rompe todo lo","que hay alrededor");
        // Grass and flowers to kick through.
        var plants=new BlockState[]{Blocks.SHORT_GRASS.defaultBlockState(),Blocks.FERN.defaultBlockState(),Blocks.POPPY.defaultBlockState(),Blocks.DANDELION.defaultBlockState()};
        for(int u=-14;u<=-3;u++)for(int v=5;v<=16;v++)set(level,u,v,0,plants[Math.floorMod(u*7+v*13,plants.length)]);
        // A ring of blocks to stand in: dirt falls in two kicks, what's harder takes more.
        int[][] ring={{-2,0},{-2,1},{-1,2},{0,2},{1,2},{2,1},{2,0},{2,-1},{1,-2},{0,-2},{-1,-2},{-2,-1}};
        var kinds=new BlockState[]{Blocks.DIRT.defaultBlockState(),Blocks.DIRT.defaultBlockState(),Blocks.OAK_LOG.defaultBlockState(),Blocks.STONE.defaultBlockState()};
        for(int i=0;i<ring.length;i++)for(int y=0;y<2;y++)set(level,6+ring[i][0],9+ring[i][1],y,kinds[i%kinds.length]);
        // A wall of ledges of three heights, and steps up it.
        for(int u=-12;u<=12;u++)for(int v=-16;v<=-13;v++){
            int high=u<-4?3:u<4?5:7;
            for(int y=0;y<high;y++)set(level,u,v,y,Blocks.STONE_BRICKS.defaultBlockState());
        }
        // A pool to swim in, cut in the floor.
        box(level,-12,-11,-5,-5,-5,-1,Blocks.STONE_BRICKS.defaultBlockState());
        box(level,-11,-10,-4,-6,-6,-1,Blocks.WATER.defaultBlockState());
    }
    private static void objects(ServerPlayer p,Direction back){
        var level=p.level();
        title(p,"2 · Objetos");
        sign(level,-15,3,back,"2 Objetos","Rompe barriles y","cajas: sueltan","partículas");
        int u=-15;
        for(var id:List.of("objeto_montana_barril_1","objeto_montana_barril_2","objeto_montana_barril_3","objeto_villarrefugio_barril_1",
            "objeto_spargus_barril_1","objeto_haven_bidon_1")){use(p,"jak:"+id,u,8,back);u+=5;}
        use(p,"jak:objeto_pantano_barril_1",14,12,back);
        for(int i=0;i<6;i++)use(p,"jakmc:ammo_crate",-12+i*4,14,back);
        // The lamps hang from a beam, swinging.
        var wood=Blocks.DARK_OAK_LOG.defaultBlockState();
        for(int y=0;y<6;y++){set(level,-13,-5,y,wood);set(level,13,-5,y,wood);}
        for(int i=-13;i<=13;i++)set(level,i,-5,6,Blocks.DARK_OAK_PLANKS.defaultBlockState());
        for(int i=-10;i<=10;i+=5)set(level,i,-5,5,block("jakmc:hut_lamp"));
        // The two doors in a wall: the Precursor iris, and Haven's airlock.
        var left=right.getOpposite();
        use(p,"jakmc:iris_door",-6,-13,right);
        use(p,"jakmc:airlock_door",6,-13,right);
        for(int i=-12;i<=12;i++)for(int y=0;y<5;y++)if(level.getBlockState(at(i,-13,y)).isAir())set(level,i,-13,y,EDGE);
        sign(level,-6,-10,left,"Puerta iris","precursora");
        sign(level,6,-10,left,"Esclusa de","Villarefugio");
    }
    private static void eco(ServerPlayer p,Direction back){
        var level=p.level();
        title(p,"3 · Eco");
        sign(level,-15,3,back,"3 Eco","Pasa por las","fuentes y el","lanzador azul");
        int u=-14;
        for(var color:List.of("red","yellow","blue","green","dark","light")){use(p,"jakmc:vent_"+color,u,8,back);u+=5;}
        use(p,"jakmc:eco_launcher",-8,-9,back);
        // A pit of dark eco, rimmed.
        box(level,4,-13,-3,12,-5,-1,EDGE);
        box(level,5,-12,-2,11,-6,-1,block("jakmc:dark_eco"));
        chest(level,-14,14,back,new ItemStack(JakItems.GREEN_ECO,64),new ItemStack(JakItems.DARK_ECO,64),new ItemStack(JakItems.LIGHT_ECO,64));
    }
    private static void guns(ServerPlayer p,Direction back){
        var level=p.level();
        title(p,"4 · Armas");
        sign(level,-15,3,back,"4 Armas","Coge las armas y","la munición de","los cofres");
        var guns=new ArrayList<ItemStack>();
        for(var gun:JakItems.GUNS)guns.add(new ItemStack(gun));
        chest(level,-8,-4,back,guns.toArray(ItemStack[]::new));
        var ammo=new ArrayList<ItemStack>();
        for(var kind:JakItems.AMMUNITION){ammo.add(new ItemStack(kind,64));ammo.add(new ItemStack(kind,64));}
        chest(level,-6,-4,back,ammo.toArray(ItemStack[]::new));
        // A pen of lurkers to shoot at across the fence, and a wall to blow up.
        fence(level,-14,6,14,16,0);
        int i=0;
        for(var name:List.of("babak","babak","bonelurker","lurkerpuppy","lurkerpuppy","babak","bonelurker","lurkercrab"))creature(level,name,-10+(i++)*3,11);
        for(int u=-12;u<=12;u++)for(int y=0;y<5;y++)set(level,u,-15,y,((u+y)&3)==0?Blocks.STONE.defaultBlockState():Blocks.DIRT.defaultBlockState());
    }
    private static void powers(ServerPlayer p,Direction back){
        var level=p.level();
        title(p,"5 · Jak Oscuro y Luminoso");
        sign(level,-15,3,back,"5 Poderes","Eco oscuro y","luminoso en","el cofre");
        chest(level,-8,-4,back,new ItemStack(JakItems.SKULL_GEM,64),new ItemStack(JakItems.DARK_ECO,64),new ItemStack(JakItems.LIGHT_ECO,64));
        // The arena: a fence to hop, Metal Heads inside (the Guard would fight them before Jak came).
        fence(level,-14,5,14,16,0);
        int i=0;
        for(var name:List.of("grunt","grunt","gekko","grunt","gekko","grunt","gekko"))creature(level,name,-10+(i++)*3,11);
    }
    private static void zoo(ServerPlayer p,Direction back){
        var level=p.level();
        title(p,"6 · Bichos");
        sign(level,-16,3,back,"6 Bichos","Todos en sus","vitrinas: rompe","el cristal");
        var glass=Blocks.GLASS.defaultBlockState();
        int index=0;
        for(int row=0;row<4&&index<CREATURES.length;row++){
            int v0=row==0?4:row==1?8:row==2?-8:-12;
            for(int column=0;column<6&&index<CREATURES.length;column++){
                int u0=-15+column*5,name=index++;
                for(int u=u0;u<=u0+4;u++)for(int v=v0;v<=v0+4;v++)for(int y=0;y<=4;y++){
                    boolean wall=u==u0||u==u0+4||v==v0||v==v0+4||y==4;
                    if(wall)set(level,u,v,y,glass);
                }
                var made=creature(level,CREATURES[name],u0+2,v0+2);
                if(made!=null)sign(level,u0+2,row==0?3:row==1?13:row==2?-3:-13,row==0||row==3?right.getOpposite():right,made.getType().getDescription());
            }
        }
    }
    private static void garage(ServerPlayer p,Direction back){
        var level=p.level();
        title(p,"7 · Vehículos");
        sign(level,-15,3,back,"7 Vehículos","Sube a cualquiera","Tabla y rieles","en el cofre");
        var fly=List.of("jakmc:hellcat","jakmc:helldog");
        var cars=List.of("jakmc:dune_hopper","jakmc:sand_shark","jakmc:zoomer_car_a","jakmc:zoomer_car_b","jakmc:zoomer_car_c");
        var bikes=List.of("jakmc:zoomer_bike_a","jakmc:zoomer_bike_b","jakmc:zoomer_bike_c");
        int u=-12;
        for(var id:cars){use(p,id,u,7,forward);u+=7;}
        u=-11;
        for(var id:bikes){use(p,id,u,-6,forward);u+=4;}
        use(p,fly.get(0),5,-7,forward);use(p,fly.get(1),11,-7,forward);
        creature(level,"flut_flut",-12,-12);creature(level,"yakow",-8,-12);
        chest(level,-4,-4,back,new ItemStack(JakItems.BOARD),new ItemStack(item("jak:grind_rail"),64));
        // A grind rail on posts down the far edge, for the board.
        for(int i=-14;i<=14;i++){
            set(level,i,14,2,EDGE);
            if(i%7==0)for(int y=0;y<2;y++)set(level,i,14,y,EDGE);
        }
        for(int i=-14;i<=14;i++)level.setBlock(at(i,14,3),block("jak:grind_rail"),3);
    }
    private static void cities(ServerPlayer p,Direction back){
        var level=p.level();
        title(p,"8 · Spargus y Villarefugio");
        sign(level,-6,-4*havenSide,back,"Spargus","por este lado");
        sign(level,-6,4*havenSide,back,"Villarefugio","por este lado","Esclusas y cajas","de munición");
    }

    /** From the last station to the nearest street of each city, on the land as it lies. */
    private static void roads(ServerLevel level){
        var end=cell(centre(STATIONS-1),0,0);
        for(var start:List.of(spargus,haven)){
            BlockPos best=null;
            for(var street:JakCityStructure.streets(start))for(var point:street.points()){
                var at=BlockPos.containing(point);
                if(best==null||at.distManhattan(end.atY(at.getY()))<best.distManhattan(end.atY(best.getY())))best=at;
            }
            if(best==null)continue;
            result.addProperty(start==spargus?"spargus_street":"haven_street",best.toShortString());
            if(start==spargus)spargusStreet=best;else havenStreet=best;
            int x=end.getX(),z=end.getZ();
            // Across first, off the road; then along.
            if(right.getAxis()==Direction.Axis.X)while(x!=best.getX()){x+=Integer.signum(best.getX()-x);widen(level,x,z);}
            while(z!=best.getZ()){z+=Integer.signum(best.getZ()-z);widen(level,x,z);}
            while(x!=best.getX()){x+=Integer.signum(best.getX()-x);widen(level,x,z);}
        }
    }
    private static void widen(ServerLevel level,int x,int z){
        for(int a=-1;a<=1;a++)for(int b=-1;b<=1;b++){
            if(city(x+a,z+b))continue;
            int across=(x+a-line.getX())*right.getStepX()+(z+b-line.getZ())*right.getStepZ();
            int top=Math.abs(across)<FLAT?heights[STATIONS-1]:ground(level,x+a,z+b);
            level.setBlock(new BlockPos(x+a,top,z+b),PATH,2);
            for(int y=top+1;y<=top+5;y++)level.setBlock(new BlockPos(x+a,y,z+b),AIR,2);
        }
    }

    // ---- looked at from above, station by station, and from high up ----
    private static void look(Vec3 eye,Vec3 target){
        var d=target.subtract(eye);
        view=new InputScene.CameraView(eye,(float)Math.toDegrees(Math.atan2(-d.x,d.z)),(float)-Math.toDegrees(Math.atan2(d.y,d.horizontalDistance())));
    }
    private static void capture(Minecraft c,String file){Screenshot.grab(c.gameDirectory,file,c.gameRenderer.mainRenderTarget(),1,_->{});}
    private record Shot(String name,BlockPos stand,Vec3 eye,Vec3 target,int ticks){}
    private static final List<Shot> shots=new ArrayList<>();
    private static void plan(){
        shots.clear();
        var side=Vec3.atLowerCornerOf(forward.getUnitVec3i());
        var across=Vec3.atLowerCornerOf(right.getUnitVec3i());
        for(int i=0;i<STATIONS;i++){
            var middle=cell(centre(i),0,heights[i]);var target=Vec3.atCenterOf(middle);
            shots.add(new Shot("station-"+i,middle.above(40),target.add(0,26,0).add(side.scale(-34)),target,110));
        }
        var end=cell(centre(STATIONS-1),0,heights[STATIONS-1]);var last=Vec3.atCenterOf(end);
        shots.add(new Shot("high-0",end.above(40),last.add(0,190,0).add(side.scale(-120)),last,200));
        shots.add(new Shot("high-1",end.above(40),last.add(0,70,0).add(side.scale(-200)),last,200));
        shots.add(new Shot("high-2",end.above(40),last.add(across.scale(-60)).add(0,30,0),last.add(across.scale(80)),200));
        var things=Vec3.atCenterOf(cell(centre(2),0,heights[2]+1));
        shots.add(new Shot("close-barrels",cell(centre(2),0,heights[2]+20),things.add(side.scale(-6)).add(across.scale(1)).add(0,3,0),things.add(across.scale(9)),60));
        shots.add(new Shot("close-lamps",cell(centre(2),0,heights[2]+20),things.add(across.scale(1)).add(0,2.5,0),things.add(across.scale(-8)).add(0,2,0),60));
        var eco=Vec3.atCenterOf(cell(centre(3),0,heights[3]+1));
        shots.add(new Shot("close-eco",cell(centre(3),0,heights[3]+20),eco.add(across.scale(1)).add(0,3,0),eco.add(across.scale(9)),60));
        if(spargusStreet!=null)shots.add(new Shot("spargus",spargusStreet.above(30),Vec3.atCenterOf(spargusStreet).add(0,14,0).add(across.scale(-24)),Vec3.atCenterOf(spargusStreet).add(across.scale(14)),150));
        if(havenStreet!=null)shots.add(new Shot("haven",havenStreet.above(30),Vec3.atCenterOf(havenStreet).add(0,14,0).add(across.scale(24)),Vec3.atCenterOf(havenStreet).add(across.scale(-14)),150));
    }
    private static void tour(Minecraft c){
        if(shots.isEmpty())plan();
        if(shot>=shots.size()){view=null;stage=7;step=0;return;}
        var next=shots.get(shot);
        if(step==0){
            var server=c.getSingleplayerServer();
            server.execute(()->{var p=server.getPlayerList().getPlayer(owner);if(p!=null)p.teleportTo(next.stand.getX()+.5,next.stand.getY(),next.stand.getZ()+.5);});
            look(next.eye,next.target);
            step=1;wait=next.ticks;
        }else{
            capture(c,"showcase-"+next.name+".png");
            var server=c.getSingleplayerServer();
            server.execute(()->{var p=server.getPlayerList().getPlayer(owner);if(p!=null)strays(p.level());});
            step=0;shot++;wait=4;
        }
    }

    /** Jak stands at the start, the crosshair away from it all: two kicks reach the pigs and the dirt round him. */
    private static void kick(ServerPlayer p){
        var level=p.level();
        station=0;floor=heights[0];
        var spot=Vec3.atBottomCenterOf(at(0,0,0));
        var ahead=Vec3.atLowerCornerOf(forward.getUnitVec3i());
        switch(step++){
            case 0->{
                p.getAbilities().flying=false;p.onUpdateAbilities();p.setGameMode(GameType.SURVIVAL);
                p.teleportTo(spot.x,spot.y,spot.z);p.setYRot(forward.toYRot());p.setXRot(-70);p.setDeltaMovement(Vec3.ZERO);
                JakPlayers.controlForProbe(p,true);
                wait=60;
            }
            case 1->{
                pig=spawnPig(level,spot.add(ahead.scale(-1.1)));front=spawnPig(level,spot.add(ahead.scale(1.1)));
                set(level,0,1,0,Blocks.DIRT.defaultBlockState());
                wait=10;
            }
            case 2->{
                result.addProperty("pig_behind_before",pig.getHealth());result.addProperty("pig_ahead_before",front.getHealth());
                result.addProperty("jak_on_ground",p.onGround());
                JakPlayers.probeInput(p,CIRCLE,CIRCLE,0);
                wait=2;
            }
            case 3->{JakPlayers.probeInput(p,0,0,0);wait=40;}
            case 4->{
                result.addProperty("pig_behind_after_1",pig.getHealth());result.addProperty("pig_ahead_after_1",front.getHealth());
                JakPlayers.probeInput(p,CIRCLE,CIRCLE,0);
                wait=2;
            }
            case 5->{JakPlayers.probeInput(p,0,0,0);wait=40;}
            case 6->{
                result.addProperty("pig_behind_after_2",pig.getHealth());result.addProperty("pig_ahead_after_2",front.getHealth());
                result.addProperty("dirt_left",level.getBlockState(at(0,1,0)).is(Blocks.DIRT));
                pig.discard();front.discard();
                set(level,0,1,0,AIR);
                JakPlayers.controlForProbe(p,false);
                p.setXRot(0);
                var arena=new net.minecraft.world.phys.AABB(Vec3.atCenterOf(cell(centre(5),0,heights[5])),Vec3.atCenterOf(cell(centre(5),0,heights[5]))).inflate(24,8,24);
                result.addProperty("arena_left",level.getEntitiesOfClass(Mob.class,arena).size());
                command(p,"setworldspawn "+(int)Math.floor(spot.x)+" "+(int)Math.floor(spot.y)+" "+(int)Math.floor(spot.z));
                result.addProperty("spawn",BlockPos.containing(spot).toShortString());
                result.addProperty("road_direction",forward.getName());
                result.addProperty("keep_inventory",level.getGameRules().get(net.minecraft.world.level.gamerules.GameRules.KEEP_INVENTORY));
                result.addProperty("advance_time",level.getGameRules().get(net.minecraft.world.level.gamerules.GameRules.ADVANCE_TIME));
                stage=8;
            }
            default->{}
        }
    }
    /** What the land spawned while the road was being built: only what the stations placed stays near it. */
    private static void strays(ServerLevel level){
        int removed=0;
        for(int i=0;i<STATIONS;i++){
            var middle=Vec3.atCenterOf(cell(centre(i),0,heights[i]));
            for(var mob:level.getEntitiesOfClass(Mob.class,new net.minecraft.world.phys.AABB(middle,middle).inflate(46,30,46)))
                if(!mob.isPersistenceRequired()&&!city(mob.getBlockX(),mob.getBlockZ())){mob.discard();removed++;}
        }
        if(removed>0)result.addProperty("strays",removed+(result.has("strays")?result.get("strays").getAsInt():0));
    }
    private static LivingEntity spawnPig(ServerLevel level,Vec3 at){
        var made=EntityTypes.PIG.create(level,EntitySpawnReason.COMMAND);
        made.snapTo(at.x,at.y,at.z,0,0);made.setNoAi(true);level.addFreshEntity(made);
        return made;
    }

    private static void finish(Minecraft c){
        view=null;running=false;
        result.addProperty("visual_review","required");result.addProperty("reason","complete");
        var server=c.getSingleplayerServer();
        server.execute(()->{
            try{
                var dir=c.gameDirectory.toPath().resolve("jakmc/evidence/showcase");
                Files.createDirectories(dir);Files.writeString(dir.resolve("result.json"),result.toString());
            }catch(IOException failure){throw new UncheckedIOException(failure);}
            c.execute(()->{c.options.pauseOnLostFocus=pause;c.stop();});
        });
    }
}
