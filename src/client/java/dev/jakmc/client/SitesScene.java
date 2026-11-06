package dev.jakmc.client;

import com.google.gson.JsonObject;
import dev.jakmc.JakData;
import dev.jakmc.content.JakAssemblies;
import java.io.IOException;
import java.io.UncheckedIOException;
import java.nio.file.Files;
import net.fabricmc.fabric.api.client.event.lifecycle.v1.ClientTickEvents;
import net.minecraft.client.Minecraft;
import net.minecraft.client.Screenshot;
import net.minecraft.core.BlockPos;
import net.minecraft.core.HolderSet;
import net.minecraft.core.registries.BuiltInRegistries;
import net.minecraft.core.registries.Registries;
import net.minecraft.resources.ResourceKey;
import net.minecraft.server.level.ServerPlayer;
import net.minecraft.world.level.GameType;
import net.minecraft.world.level.block.entity.ChestBlockEntity;
import net.minecraft.world.level.levelgen.structure.BoundingBox;
import net.minecraft.world.phys.Vec3;

final class SitesScene {
    private static final String[] ORIGINAL_SITES={"swamp_hut","snow_fort","haven","spargus"};
    private static final String[] NEW_SITES={"sandover","crater_cave"};
    private static final JsonObject result=new JsonObject();
    private static final java.util.Set<String> JAK1=java.util.Set.of("green_eco","vent_blue","vent_yellow","vent_red","vent_green","flut_egg");
    private static volatile boolean active;
    private static int index,ticks;
    private static boolean pause;
    private static BoundingBox bounds;
    private static BlockPos origin;
    private static Runnable restore;
    private static String[] sites;
    private static String scene;
    private static volatile InputScene.CameraView view;
    static InputScene.CameraView camera(){return view;}
    private static java.util.UUID owner;
    static void initialize(){ClientTickEvents.END_CLIENT_TICK.register(SitesScene::tick);}
    static void start(Minecraft client){start(client,false);}
    static void startCities(Minecraft client){start(client,false,true);}
    static void start(Minecraft client,boolean recent){start(client,recent,false);}
    private static void start(Minecraft client,boolean recent,boolean citiesOnly){
        var server=client.getSingleplayerServer();
        owner=client.player.getUUID();
        pause=client.options.pauseOnLostFocus;client.options.pauseOnLostFocus=false;
        result.entrySet().clear();index=citiesOnly?2:0;ticks=0;active=false;
        sites=recent?NEW_SITES:ORIGINAL_SITES;
        scene=citiesOnly?"natural_cities":recent?"new_sites":"sites";
        server.execute(()->{
            var p=server.getPlayerList().getPlayer(owner);
            if(p==null)return;
            var old=p.position();var mode=p.gameMode.getGameModeForPlayer();
            var enabled=p.getAttachedOrCreate(JakData.ENABLED);
            boolean flying=p.getAbilities().flying;
            origin=p.blockPosition();
            p.setAttached(JakData.ENABLED,false);
            p.setGameMode(GameType.CREATIVE);p.getAbilities().flying=true;p.onUpdateAbilities();
            restore=()->{
                p.teleportTo(old.x,old.y,old.z);
                p.setAttached(JakData.ENABLED,enabled);p.setGameMode(mode);
                p.getAbilities().flying=flying;p.onUpdateAbilities();
            };
            locate(client,p);
        });
    }
    private static void locate(Minecraft client,ServerPlayer p){
        while(index<sites.length){
            var level=p.level();
            var key=ResourceKey.create(Registries.STRUCTURE,JakData.id(sites[index]));
            var type=level.registryAccess().lookupOrThrow(Registries.STRUCTURE).getOrThrow(key);
            var point=level.findNearestMapStructure(HolderSet.direct(type),origin,128,false);
            if(point!=null){
                var start=level.getChunk(point).getStartForStructure(type.value());
                bounds=start.getBoundingBox();
                var at=Vec3.atCenterOf(bounds.getCenter());
                double height=sites==NEW_SITES&&index==1?bounds.maxY()-25:
                    index<2?bounds.maxY()+8:Math.max(bounds.minY()+55,level.getSeaLevel()+15);
                p.teleportTo(at.x,height,at.z);
                ticks=0;active=true;
                return;
            }
            var failure=new JsonObject();failure.addProperty("found",false);result.add(sites[index++],failure);
        }
        finish(client);
    }
    private static void tick(Minecraft client){
        if(!active)return;
        int t=++ticks;
        var server=client.getSingleplayerServer();
        if(t==85)server.execute(()->{
            var p=server.getPlayerList().getPlayer(owner);
            if(p!=null)inspect(p);
        });
        // jak2_villagers.rs peopling: the town's residents (and Spargus's loose leapers) once Jak is near.
        boolean city=sites[index].equals("haven")||sites[index].equals("spargus");
        if(city&&t==(sites[index].equals("haven")?280:196))server.execute(()->{
            var p=server.getPlayerList().getPlayer(owner);
            if(p==null)return;
            var level=p.level();
            var box=net.minecraft.world.phys.AABB.of(bounds);
            var info=result.getAsJsonObject(sites[index]);
            info.addProperty("population_residents",level.getEntitiesOfClass(net.minecraft.world.entity.npc.villager.Villager.class,box,dev.jakmc.worldgen.CityPopulation::resident).size());
            info.addProperty("population_peopled",dev.jakmc.worldgen.CityPopulation.peopled(level).stream().anyMatch(bounds::isInside));
            info.addProperty("population_loose_leapers",level.getEntitiesOfClass(dev.jakmc.entity.JakLeaper.class,box,l->l.isAlive()&&!l.patrolling()).size());
        });
        // jak2_villagers.rs struck: Jak's second blow on one citizen raises the city's alert.
        boolean alarm=sites[index].equals("haven");
        if(alarm&&t==290)server.execute(()->{
            var p=server.getPlayerList().getPlayer(owner);
            if(p==null)return;
            p.setGameMode(GameType.SURVIVAL);
            p.getAbilities().invulnerable=true;p.getAbilities().mayfly=true;p.getAbilities().flying=true;p.onUpdateAbilities();
        });
        if(alarm&&(t==300||t==315))server.execute(()->{
            var p=server.getPlayerList().getPlayer(owner);
            if(p==null)return;
            var level=p.level();
            var v=level.getEntitiesOfClass(net.minecraft.world.entity.npc.villager.Villager.class,net.minecraft.world.phys.AABB.of(bounds),dev.jakmc.worldgen.CityPopulation::resident)
                .stream().min(java.util.Comparator.comparingInt(net.minecraft.world.entity.Entity::getId)).orElse(null);
            var info=result.getAsJsonObject("haven");
            if(v==null){info.addProperty("alarm_citizen",false);return;}
            info.addProperty("alarm_citizen",true);
            info.addProperty("alarm_before_"+ticks,dev.jakmc.entity.CityAlert.on(level));
            v.hurtServer(level,level.damageSources().playerAttack(p),.5f);
            v.setHealth(v.getMaxHealth());
        });
        if(alarm&&t==345)server.execute(()->{
            var p=server.getPlayerList().getPlayer(owner);
            if(p==null)return;
            var level=p.level();
            var info=result.getAsJsonObject("haven");
            info.addProperty("alarm_on",dev.jakmc.entity.CityAlert.on(level));
            info.addProperty("alarm_guards_after_jak",level.getEntitiesOfClass(dev.jakmc.entity.JakEnemy.class,net.minecraft.world.phys.AABB.of(bounds),g->g.kind.guard()&&g.getTarget()==p).size());
            info.addProperty("alarm_guards",level.getEntitiesOfClass(dev.jakmc.entity.JakEnemy.class,net.minecraft.world.phys.AABB.of(bounds),g->g.kind.guard()&&g.isAlive()).size());
            dev.jakmc.entity.CityAlert.clear();
            for(var g:level.getEntitiesOfClass(dev.jakmc.entity.JakEnemy.class,net.minecraft.world.phys.AABB.of(bounds),g->g.getTarget()==p))g.setTarget(null);
            p.setGameMode(GameType.CREATIVE);p.getAbilities().flying=true;p.onUpdateAbilities();
        });
        // The barrio's people seen from the street: the citizens round the one nearest its middle.
        if(alarm&&t==282&&client.level!=null){
            var middle=Vec3.atCenterOf(bounds.getCenter());
            var near=client.level.getEntitiesOfClass(net.minecraft.world.entity.npc.villager.Villager.class,net.minecraft.world.phys.AABB.of(bounds),v->!v.isPassenger())
                .stream().min(java.util.Comparator.comparingDouble(v->v.position().distanceToSqr(middle))).orElse(null);
            if(near!=null)look(near.position().add(5,4,5),near.position().add(0,1,0));
        }
        if(alarm&&t==288)Screenshot.grab(client.gameDirectory,"sites-haven-citizens.png",client.gameRenderer.mainRenderTarget(),1,_-> {});
        if(t==100&&view!=null)Screenshot.grab(client.gameDirectory,"sites-"+sites[index]+".png",
            client.gameRenderer.mainRenderTarget(),1,_-> {});
        // Haven's traffic: where the zoomers are, and again after a while, then one seen from the street.
        boolean traffic=sites[index].equals("haven");
        if(traffic&&(t==200||t==380))server.execute(()->{
            var list=new com.google.gson.JsonArray();for(var v:dev.jakmc.entity.ZoomerTraffic.report())list.add(v);
            result.getAsJsonObject("haven").add("traffic_at_"+ticks,list);
        });
        if(traffic&&t>=385&&t<=398){var at=dev.jakmc.entity.ZoomerTraffic.positions();
            if(!at.isEmpty()){var car=at.getFirst();var eye=car.add(3,9,3);var d=car.subtract(eye);
                view=new InputScene.CameraView(eye,(float)Math.toDegrees(Math.atan2(-d.x,d.z)),(float)Math.toDegrees(Math.atan2(-d.y,d.horizontalDistance())));}}
        if(traffic&&t==398)Screenshot.grab(client.gameDirectory,"sites-haven-traffic.png",client.gameRenderer.mainRenderTarget(),1,_-> {});
        if(t==(traffic?404:sites[index].equals("spargus")?204:104)){
            active=false;view=null;
            server.execute(()->{
                var p=server.getPlayerList().getPlayer(owner);
                if(p!=null){index++;locate(client,p);}
            });
        }
    }
    private static void inspect(ServerPlayer player){
        var level=player.level();
        var site=sites[index];
        boolean crater=site.equals("crater_cave");
        String expected=index<2&&!crater?"jak1":site;
        int chests=0,assigned=0,items=0;
        boolean sourceLoot=true;
        var facadeBlocks=new java.util.HashMap<net.minecraft.world.level.block.Block,String>();
        var facadeCounts=new java.util.TreeMap<String,Integer>();
        String portal=index==2?"barrio_portal_doble":"sparg_portal_doble";
        if(index>=2)JakAssemblies.FACADES.forEach((name,model)->model.parts().forEach(part->facadeBlocks.put(part.block(),name)));
        BlockPos[] doorway={null};
        int[] doors={0};
        BlockPos[] chestPosition={null};
        int[] structures={0,0,0};
        for(int x=bounds.minX()>>4;x<=bounds.maxX()>>4;x++)for(int z=bounds.minZ()>>4;z<=bounds.maxZ()>>4;z++){
            var chunk=level.getChunk(x,z,net.minecraft.world.level.chunk.status.ChunkStatus.FULL,false);
            if(!(chunk instanceof net.minecraft.world.level.chunk.LevelChunk loaded))continue;
            if(sites==NEW_SITES)loaded.findBlocks(state->crater
                ?state.is(net.minecraft.world.level.block.Blocks.MAGMA_BLOCK)||state.is(net.minecraft.world.level.block.Blocks.LAVA)
                :state.is(net.minecraft.world.level.block.Blocks.BELL)||state.hasProperty(net.minecraft.world.level.block.state.properties.BlockStateProperties.BED_PART),
                (at,state)->{
                    if(!bounds.isInside(at))return;
                    boolean primary=state.is(crater?net.minecraft.world.level.block.Blocks.MAGMA_BLOCK:net.minecraft.world.level.block.Blocks.BELL);
                    if(primary)structures[0]++;
                    else if(crater)structures[1]++;
                    else if(state.getValue(net.minecraft.world.level.block.state.properties.BlockStateProperties.BED_PART)==
                        net.minecraft.world.level.block.state.properties.BedPart.FOOT){
                        var next=level.getBlockState(at.relative(state.getValue(net.minecraft.world.level.block.state.properties.BlockStateProperties.HORIZONTAL_FACING)));
                        structures[1]++;
                        if(next.getBlock()==state.getBlock()&&next.getValue(net.minecraft.world.level.block.state.properties.BlockStateProperties.BED_PART)==
                            net.minecraft.world.level.block.state.properties.BedPart.HEAD)structures[2]++;
                    }
                });
            if(index>=2)loaded.findBlocks(state->facadeBlocks.containsKey(state.getBlock()),(at,state)->{
                if(!bounds.isInside(at))return;
                String name=facadeBlocks.get(state.getBlock());
                facadeCounts.merge(name,1,Integer::sum);
                if(!name.equals(portal)||!BuiltInRegistries.BLOCK.getKey(state.getBlock()).getPath().equals(name))return;
                var lower=at.below(3);
                if(!(level.getBlockState(lower).getBlock() instanceof net.minecraft.world.level.block.DoorBlock))return;
                doors[0]++;
                if(doorway[0]==null||at.distSqr(bounds.getCenter())<doorway[0].distSqr(bounds.getCenter()))doorway[0]=at.immutable();
            });
            for(var entry:loaded.getBlockEntities().entrySet()){
                if(!(entry.getValue() instanceof ChestBlockEntity chest)||!bounds.isInside(entry.getKey()))continue;
                chests++;
                var table=chest.getLootTable();
                if(table==null||!table.identifier().equals(JakData.id("chests/"+expected)))continue;
                assigned++;
                if(assigned>1)continue;
                chestPosition[0]=entry.getKey();
                chest.unpackLootTable(player);
                for(int slot=0;slot<chest.getContainerSize();slot++){
                    var stack=chest.getItem(slot);
                    if(stack.isEmpty())continue;
                    var id=BuiltInRegistries.ITEM.getKey(stack.getItem());
                    sourceLoot&=id.getNamespace().equals("jakmc")&&(index>=2||crater||JAK1.contains(id.getPath()));
                    items+=stack.getCount();
                }
            }
        }
        var info=new JsonObject();
        info.addProperty("biome",level.getBiome(bounds.getCenter()).unwrapKey().orElseThrow().identifier().toString());
        info.addProperty("found",true);info.addProperty("x",bounds.getCenter().getX());info.addProperty("z",bounds.getCenter().getZ());
        info.addProperty("chests",chests);info.addProperty("lootTables",assigned);info.addProperty("generatedItems",items);
        info.addProperty("matchesEra",sourceLoot&&(crater?items==100:items>0));
        if(sites==NEW_SITES){
            var box=new net.minecraft.world.phys.AABB(bounds.minX(),bounds.minY(),bounds.minZ(),bounds.maxX()+1,bounds.maxY()+1,bounds.maxZ()+1);
            if(crater){
                int enemies=level.getEntitiesOfClass(dev.jakmc.entity.JakEnemy.class,box,
                    entity->entity.getY()<bounds.maxY()-20).size();
                info.addProperty("magma",structures[0]);info.addProperty("lava",structures[1]);info.addProperty("defenders",enemies);
                info.addProperty("layout",structures[0]>0&&structures[1]>=18&&enemies>=3);
                if(chestPosition[0]!=null){
                    var at=chestPosition[0];
                    var facing=level.getBlockState(at).getValue(net.minecraft.world.level.block.state.properties.BlockStateProperties.HORIZONTAL_FACING);
                    var target=Vec3.atCenterOf(at);
                    look(target.add(facing.getStepX()*5,2,facing.getStepZ()*5),target);
                }
            }else{
                int villagers=level.getEntitiesOfClass(net.minecraft.world.entity.npc.villager.AbstractVillager.class,box).size();
                info.addProperty("bells",structures[0]);info.addProperty("beds",structures[1]);
                info.addProperty("completeBeds",structures[2]);info.addProperty("residents",villagers);
                info.addProperty("layout",structures[0]==1&&structures[1]>=4&&structures[1]==structures[2]&&villagers>0);
                var at=Vec3.atCenterOf(bounds.getCenter()).with(net.minecraft.core.Direction.Axis.Y,bounds.maxY()-9);
                look(at.add(16,19,-21),at);
            }
        }
        if(index>=2){
            var parts=new JsonObject();facadeCounts.forEach(parts::addProperty);
            info.add("facades",parts);info.addProperty("doorsBelowPortal",doors[0]);
            info.addProperty("hasFacades",doors[0]>0&&facadeCounts.size()==(index==2?4:2));
            if(doorway[0]!=null){
                var state=level.getBlockState(doorway[0]);
                var facing=state.getValue(net.minecraft.world.level.block.state.properties.BlockStateProperties.HORIZONTAL_FACING);
                var centre=Vec3.atCenterOf(doorway[0]).add(0,-1.1,0);
                var eye=centre.add(facing.getStepX()*5.4+.6,.8,facing.getStepZ()*5.4+.6);
                var look=centre.subtract(eye);
                view=new InputScene.CameraView(eye,(float)Math.toDegrees(Math.atan2(-look.x,look.z)),
                    (float)-Math.toDegrees(Math.atan2(look.y,look.horizontalDistance())));
            }
        }
        if(site.equals("haven")){
            var start=level.structureManager().startsForStructure(bounds.getCenter().getX()>>4,bounds.getCenter().getZ()>>4,
                level.registryAccess().lookupOrThrow(Registries.STRUCTURE).getValue(JakData.id("haven")));
            if(!start.isEmpty()){
                var streets=dev.jakmc.worldgen.JakCityStructure.streets(start.getFirst());
                info.addProperty("traffic_streets",streets.size());
                var each=new com.google.gson.JsonArray();
                for(var street:streets)each.add(street.from().getX()+","+street.from().getZ()+">"+street.to().getX()+","+street.to().getZ()+" "+dev.jakmc.entity.ZoomerTraffic.clear(level,street.points())+" "+dev.jakmc.entity.ZoomerTraffic.blocker(level,street.points()));
                info.add("traffic_street_state",each);
                var net=dev.jakmc.entity.ZoomerTraffic.network(level,streets);
                info.addProperty("traffic_open_streets",net==null?-1:net.streets().size());
                if(net!=null){var ends=new com.google.gson.JsonArray();net.ways().forEach((k,v)->ends.add(k.getX()+","+k.getZ()+":"+v.size()));info.add("traffic_junctions",ends);}
            }
            info.addProperty("traffic_going",dev.jakmc.entity.ZoomerTraffic.going());
        }
        result.add(site,info);
    }
    private static void look(Vec3 eye,Vec3 target){
        var delta=target.subtract(eye);
        view=new InputScene.CameraView(eye,(float)Math.toDegrees(Math.atan2(-delta.x,delta.z)),
            (float)-Math.toDegrees(Math.atan2(delta.y,delta.horizontalDistance())));
    }
    private static void finish(Minecraft client){
        boolean ok=result.asMap().values().stream().allMatch(row->row.getAsJsonObject().has("matchesEra")
            &&row.getAsJsonObject().get("matchesEra").getAsBoolean()
            &&(!row.getAsJsonObject().has("hasFacades")||row.getAsJsonObject().get("hasFacades").getAsBoolean())
            &&(!row.getAsJsonObject().has("layout")||row.getAsJsonObject().get("layout").getAsBoolean()));
        result.addProperty("reason",ok?"complete":"Natural site acceptance failed");
        try{
            var dir=client.gameDirectory.toPath().resolve("jakmc/evidence/"+scene);
            Files.createDirectories(dir);Files.writeString(dir.resolve("result.json"),result.toString());
        }catch(IOException failure){throw new UncheckedIOException(failure);}
        restore.run();
        client.execute(()->{client.options.pauseOnLostFocus=pause;client.stop();});
    }
}
