package dev.jakmc.client;

import com.google.gson.JsonObject;
import dev.jakmc.JakData;
import dev.jakmc.entity.JakVehicle;
import java.io.IOException;
import java.io.UncheckedIOException;
import java.nio.file.Files;
import net.fabricmc.fabric.api.client.event.lifecycle.v1.ClientTickEvents;
import net.minecraft.client.Minecraft;
import net.minecraft.client.Screenshot;
import net.minecraft.core.BlockPos;
import net.minecraft.core.HolderSet;
import net.minecraft.core.registries.Registries;
import net.minecraft.resources.ResourceKey;
import net.minecraft.server.level.ServerPlayer;
import net.minecraft.world.level.GameType;
import net.minecraft.world.level.block.state.properties.BedPart;
import net.minecraft.world.level.block.state.properties.BlockStateProperties;
import net.minecraft.world.level.levelgen.Heightmap;
import net.minecraft.world.phys.AABB;
import net.minecraft.world.phys.Vec3;

final class CityScene {
    private static final JsonObject results=new JsonObject();
    private static volatile BlockPos target;
    private static volatile boolean ready,finished;
    private static int ticks,index;
    private static InputScene.CameraView view;
    private static GameType beforeMode;
    private static Vec3 before;
    private static BlockPos searchFrom;
    private static boolean pause,wasJak,wasFlying;
    private static volatile int guardFightId=-1;
    private static boolean sawGuardFire;
    private static int guardScreenshotAt;
    private static net.minecraft.world.Difficulty beforeDifficulty;
    private static float healthBeforeFight,absorptionBeforeFight,healthBefore;
    private static final java.util.Map<java.util.UUID,Vec3> guardStarts=new java.util.HashMap<>();

    static InputScene.CameraView camera(){return view;}
    static void initialize(){
        ClientTickEvents.END_CLIENT_TICK.register(client->{
            if(!ready||finished)return;
            if(guardFightId>=0){
                if(client.level.getEntity(guardFightId) instanceof dev.jakmc.entity.JakEnemy guard&&!guard.guardShot().isEmpty()){
                    if(!sawGuardFire){
                        sawGuardFire=true;
                        var look=guard.position().add(0,1.2,0);
                        Vec3 eye=client.player.getEyePosition();
                        Vec3 d=look.subtract(eye);
                        view=new InputScene.CameraView(eye,(float)Math.toDegrees(Math.atan2(-d.x,d.z)),
                            (float)-Math.toDegrees(Math.atan2(d.y,d.horizontalDistance())));
                        guardScreenshotAt=ticks+1;
                    }
                }
                if(guardScreenshotAt>0&&ticks==guardScreenshotAt)
                    Screenshot.grab(client.gameDirectory,"cities-guard.png",client.gameRenderer.mainRenderTarget(),1,_-> {});
                if(++ticks>=145)onServer(client,p->finishGuardFight(client,p));
                return;
            }
            if(++ticks==70&&index==0)onServer(client,p->{
                for(var guard:guards(p))guardStarts.put(guard.getUUID(),guard.position());
            });
            if(ticks==100){
                Vec3 point=Vec3.atCenterOf(target).add(0,index==0?38:6,0);
                Vec3 eye=point.add(index==0?93:49,index==0?21:36,index==0?105:68);
                Vec3 d=point.subtract(eye);
                view=new InputScene.CameraView(eye,(float)Math.toDegrees(Math.atan2(-d.x,d.z)),
                    (float)-Math.toDegrees(Math.atan2(d.y,d.horizontalDistance())));
            }
            if(ticks==120)Screenshot.grab(client.gameDirectory,index==0?"cities-haven.png":"cities-spargus.png",
                client.gameRenderer.mainRenderTarget(),1,_-> {});
            if(ticks==180){
                ready=false;
                onServer(client,p->inspect(client,p));
            }
        });
    }
    static void start(Minecraft client,Vec3 search){
        if(client.getSingleplayerServer()==null||client.player==null)throw new IllegalStateException("Open the normal development world");
        pause=client.options.pauseOnLostFocus;
        client.options.pauseOnLostFocus=false;
        finished=ready=false;
        ticks=index=0;guardFightId=-1;sawGuardFire=false;guardScreenshotAt=0;
        results.entrySet().clear();guardStarts.clear();
        onServer(client,p->{
            before=p.position();healthBefore=p.getHealth();absorptionBeforeFight=p.getAbsorptionAmount();
            searchFrom=BlockPos.containing(search==null?before:search);
            beforeMode=p.gameMode.getGameModeForPlayer();
            wasJak=p.getAttachedOrCreate(JakData.ENABLED);
            p.setAttached(JakData.ENABLED,false);
            wasFlying=p.getAbilities().flying;
            p.setGameMode(GameType.CREATIVE);
            p.getAbilities().flying=true;
            p.onUpdateAbilities();
            locate(client,p);
        });
    }
    private static void locate(Minecraft client,ServerPlayer p){
        String name=index==0?"haven":"spargus";
        var level=p.level();
        var key=ResourceKey.create(Registries.STRUCTURE,JakData.id(name));
        var type=level.registryAccess().lookupOrThrow(Registries.STRUCTURE).getOrThrow(key);
        var pos=level.findNearestMapStructure(HolderSet.direct(type),searchFrom,96,false);
        if(pos==null){
            var failure=new JsonObject();
            failure.addProperty("found",false);
            results.add(name,failure);
            next(client,p);
            return;
        }
        var start=level.getChunk(pos).getStartForStructure(type.value());
        var box=start.getBoundingBox();
        int y=box.minY()+(index==0?42:44);
        target=new BlockPos(pos.getX(),y,pos.getZ());
        p.teleportTo(target.getX()+.5,target.getY()+12,target.getZ()+.5);
        ticks=0;
        ready=true;
    }
    private static java.util.List<dev.jakmc.entity.JakEnemy> guards(ServerPlayer p){
        var center=Vec3.atCenterOf(target);
        return p.level().getEntitiesOfClass(dev.jakmc.entity.JakEnemy.class,new AABB(center,center).inflate(85),e->e.kind.guard());
    }
    private static void inspect(Minecraft client,ServerPlayer p){
        var report=new JsonObject();
        report.addProperty("found",true);
        var level=p.level();
        var at=new BlockPos.MutableBlockPos();
        int structures=0,roads=0,projectors=0,palms=0,beds=0,brokenBeds=0,altar=0,ramps=0;
        var palette=net.minecraft.core.registries.BuiltInRegistries.BLOCK;
        for(int dx=-98;dx<=98;dx++)for(int dz=-98;dz<=98;dz++){
            int x=target.getX()+dx,z=target.getZ()+dz;
            if(!level.hasChunkAt(at.set(x,target.getY(),z)))continue;
            int surface=level.getHeight(Heightmap.Types.WORLD_SURFACE,x,z);
            for(int y=target.getY()-8;y<=Math.min(surface,target.getY()+22);y++){
                var state=level.getBlockState(at.set(x,y,z));
                if(state.isAir())continue;
                if(state.hasProperty(BlockStateProperties.BED_PART)&&state.getValue(BlockStateProperties.BED_PART)==BedPart.FOOT){
                    beds++;
                    var head=level.getBlockState(at.relative(state.getValue(BlockStateProperties.HORIZONTAL_FACING)));
                    if(!head.is(state.getBlock())||head.getValue(BlockStateProperties.BED_PART)!=BedPart.HEAD)brokenBeds++;
                }
                var id=palette.getKey(state.getBlock());
                if(!id.getNamespace().equals("jak"))continue;
                structures++;
                String path=id.getPath();
                if(path.equals("barrio_losa")||path.equals("sparg_losa"))roads++;
                if(path.equals("objeto_haven_propaganda_bloque"))projectors++;
                if(path.equals("objeto_spargus_palmera_completa"))palms++;
                if(path.equals("altar_turbotabla"))altar++;
                if(path.startsWith("objeto_haven_rampa_cuarto"))ramps++;
            }
        }
        var cars=level.getEntitiesOfClass(JakVehicle.class,new AABB(Vec3.atCenterOf(target),Vec3.atCenterOf(target)).inflate(120),e->true);
        report.addProperty("x",target.getX());report.addProperty("z",target.getZ());
        report.addProperty("jakBlocks",structures);
        report.addProperty("roadBlocks",roads);
        report.addProperty("projectors",projectors);
        report.addProperty("palms",palms);
        report.addProperty("beds",beds);
        report.addProperty("brokenBeds",brokenBeds);
        report.addProperty("altars",altar);
        report.addProperty("rampCells",ramps);
        report.addProperty("cars",cars.size());
        if(index==0){
            var guards=guards(p);
            report.addProperty("guards",guards.size());
            report.addProperty("guardsPatrolling",guards.stream().filter(g->guardStarts.containsKey(g.getUUID())
                &&g.position().distanceToSqr(guardStarts.get(g.getUUID()))>4).count());
            var patrols=new com.google.gson.JsonArray();
            for(var guard:guards){
                var row=guard.diagnostics();
                row.addProperty("ticks",guard.tickCount);
                var initial=guardStarts.get(guard.getUUID());
                if(initial!=null)row.addProperty("travel",initial.distanceTo(guard.position()));
                patrols.add(row);
            }
            report.add("patrols",patrols);
        }
        if(index==1){
            var bounds=new AABB(Vec3.atCenterOf(target),Vec3.atCenterOf(target)).inflate(120);
            var residents=level.getEntitiesOfClass(net.minecraft.world.entity.npc.villager.Villager.class,bounds);
            report.addProperty("wastelanders",residents.stream().filter(e->dev.jakmc.entity.CitizenKind.of(
                ((dev.jakmc.entity.CitizenState)e).citizenVariant()).wastelander()).count());
            var mounts=level.getEntitiesOfClass(dev.jakmc.entity.JakLeaper.class,bounds,e->e.getType()==dev.jakmc.entity.JakLeaper.TYPE);
            report.addProperty("leapers",mounts.size());
            report.addProperty("monkPatrols",mounts.stream().filter(e->e.getFirstPassenger() instanceof dev.jakmc.entity.CitizenState resident
                &&resident.citizenVariant()==dev.jakmc.entity.CitizenKind.MONK).count());
        }
        results.add(index==0?"haven":"spargus",report);
        if(index==0&&startGuardFight(p))return;
        next(client,p);
    }
    private static boolean startGuardFight(ServerPlayer p){
        var level=p.level();
        for(var guard:guards(p)){
            if(!guard.onGround()||guard.getTarget()!=null)continue;
            for(var facing:net.minecraft.core.Direction.Plane.HORIZONTAL){
                var at=guard.blockPosition().relative(facing,8);
                if(!level.hasChunkAt(at)||!level.isEmptyBlock(at)||!level.isEmptyBlock(at.above())
                    ||level.getBlockState(at.below()).getCollisionShape(level,at.below()).isEmpty())continue;
                var base=Vec3.atBottomCenterOf(at);
                var trace=level.clip(new net.minecraft.world.level.ClipContext(guard.position().add(0,.8,0),base.add(0,.9,0),
                    net.minecraft.world.level.ClipContext.Block.COLLIDER,net.minecraft.world.level.ClipContext.Fluid.NONE,guard));
                if(trace.getType()!=net.minecraft.world.phys.HitResult.Type.MISS)continue;
                beforeDifficulty=level.getDifficulty();
                if(beforeDifficulty==net.minecraft.world.Difficulty.PEACEFUL)level.getServer().setDifficulty(net.minecraft.world.Difficulty.NORMAL,true);
                p.setGameMode(GameType.SURVIVAL);
                p.setAbsorptionAmount(20);
                p.teleportTo(base.x,base.y,base.z);
                healthBeforeFight=p.getHealth()+p.getAbsorptionAmount();
                guard.hurtServer(level,p.damageSources().playerAttack(p),.5f);
                guardFightId=guard.getId();
                ticks=0;ready=true;
                return true;
            }
        }
        return false;
    }
    private static void finishGuardFight(Minecraft client,ServerPlayer p){
        if(guardFightId<0)return;
        var level=p.level();
        var report=results.getAsJsonObject("haven");
        report.addProperty("guardShotPresented",sawGuardFire);
        report.addProperty("guardShotDamage",healthBeforeFight-p.getHealth()-p.getAbsorptionAmount());
        if(level.getEntity(guardFightId) instanceof dev.jakmc.entity.JakEnemy guard){
            var region=guard.getBoundingBox().inflate(3);
            guard.hurtServer(level,p.damageSources().playerAttack(p),1000);
            report.addProperty("guardAmmoDrop",level.getEntitiesOfClass(net.minecraft.world.entity.item.ItemEntity.class,region,
                e->dev.jakmc.content.JakItems.AMMUNITION.contains(e.getItem().getItem()))
                .stream().mapToInt(e->e.getItem().getCount()).sum());
        }
        guardFightId=-1;ready=false;
        if(level.getDifficulty()!=beforeDifficulty)level.getServer().setDifficulty(beforeDifficulty,true);
        p.setHealth(healthBefore);
        p.setAbsorptionAmount(absorptionBeforeFight);
        p.setGameMode(GameType.CREATIVE);
        p.getAbilities().flying=true;p.onUpdateAbilities();
        next(client,p);
    }
    private static void next(Minecraft client,ServerPlayer p){
        view=null;
        if(index++==0){locate(client,p);return;}
        var haven=results.getAsJsonObject("haven");
        var spargus=results.getAsJsonObject("spargus");
        boolean ok=haven.has("roadBlocks")&&haven.get("roadBlocks").getAsInt()>100
            &&haven.get("guardsPatrolling").getAsInt()>0&&haven.has("guardAmmoDrop")
            &&haven.get("guardAmmoDrop").getAsInt()>=5&&haven.get("guardShotPresented").getAsBoolean()
            &&haven.get("guardShotDamage").getAsFloat()>0&&haven.get("altars").getAsInt()==1&&haven.get("rampCells").getAsInt()>20
            &&spargus.has("roadBlocks")&&spargus.get("roadBlocks").getAsInt()>100&&spargus.get("cars").getAsInt()==2
            &&spargus.get("wastelanders").getAsInt()>0&&spargus.get("monkPatrols").getAsInt()==2;
        results.addProperty("reason",ok?"complete":"Normal-world city acceptance failed");
        try{
            var path=client.gameDirectory.toPath().resolve("jakmc/evidence/cities");
            Files.createDirectories(path);
            Files.writeString(path.resolve("result.json"),results.toString());
        }catch(IOException e){throw new UncheckedIOException(e);}
        p.setGameMode(beforeMode);
        p.getAbilities().flying=wasFlying;
        p.onUpdateAbilities();
        p.setAttached(JakData.ENABLED,wasJak);
        p.teleportTo(before.x,before.y,before.z);
        finished=true;
        client.execute(()->{
            client.options.pauseOnLostFocus=pause;
            client.stop();
        });
    }
    private static void onServer(Minecraft client,java.util.function.Consumer<ServerPlayer> call){
        var server=client.getSingleplayerServer();
        var uuid=client.player.getUUID();
        server.execute(()->{
            var p=server.getPlayerList().getPlayer(uuid);
            if(p!=null)call.accept(p);
        });
    }
}
