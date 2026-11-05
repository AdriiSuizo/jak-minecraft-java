package dev.jakmc.player;

import com.google.gson.*;
import dev.jakmc.*;
import dev.jakmc.content.JakItems;
import dev.jakmc.entity.JakLeaper;
import dev.jakmc.entity.JakHellcat;
import dev.jakmc.nativebridge.*;
import dev.jakmc.network.JakPackets;
import java.nio.ByteBuffer;
import java.util.*;
import net.fabricmc.fabric.api.event.lifecycle.v1.ServerTickEvents;
import net.fabricmc.fabric.api.networking.v1.*;
import net.minecraft.server.level.ServerPlayer;
import net.minecraft.world.entity.Entity;
import net.minecraft.world.phys.*;

/** Minecraft server owns movement, survival damage, inventory and actor lifetime. */
public final class JakPlayers {
    private static final Set<UUID> CONTROLLED=new HashSet<>();
    private static final Set<UUID> HOST_CONTROLLED=new HashSet<>();
    private static final Map<UUID,JakPackets.Input> PROBE_INPUT=new HashMap<>();
    public static void controlForProbe(ServerPlayer player,boolean controlled){if(controlled){CONTROLLED.add(player.getUUID());probeInput(player,0,0,0);}else{CONTROLLED.remove(player.getUUID());HOST_CONTROLLED.remove(player.getUUID());PROBE_INPUT.remove(player.getUUID());}}
    public static void hostControlsForProbe(ServerPlayer player){
        if(!CONTROLLED.contains(player.getUUID()))throw new IllegalStateException("Only a controlled development fixture can select its input route");
        HOST_CONTROLLED.add(player.getUUID());
    }
    public static void probeInput(ServerPlayer p,int held,int pressed,float forward){PROBE_INPUT.put(p.getUUID(),new JakPackets.Input(0,held,pressed,0,forward,0,0,p.getYRot(),p.getXRot(),0,0));}
    private static final Map<UUID,Session> LIVE=new HashMap<>();
    public static void resetForProbe(ServerPlayer player){
        if(!CONTROLLED.contains(player.getUUID()))throw new IllegalStateException("Only a controlled development fixture can reset its native session");
        var old=LIVE.remove(player.getUUID());if(old!=null)old.close();
    }
    private static final int BOARD_BUTTON=1<<7;
    private static final net.minecraft.resources.Identifier VANILLA_SPRINT=
        net.minecraft.resources.Identifier.withDefaultNamespace("sprinting");
    private static final Map<net.minecraft.world.level.block.state.BlockState,Integer> FLOOR_SOUNDS=new IdentityHashMap<>();
    private record Hurt(int mode,Vec3 from){}
    private static final class Session implements AutoCloseable {
        final long nativeId;
        final MinecraftCollision.Query collision;
        final ByteBuffer controls=NativeJak.buffer(64),noTriangles=NativeJak.buffer(0);
        final DirectBufferArena actorBuffers=new DirectBufferArena(1024*1024);
        Vec3 origin;
        String dimension;
        Vec3 last;
        JakPackets.Input input=new JakPackets.Input(0,0,0,0,0,0,0,0,0,0,0);
        int pressed;
        final ArrayList<Hurt> hurts=new ArrayList<>();
        int boardUseTicks;
        boolean shield,shieldHit,altarMount,usingBoard,grinding,hostMoved;
        List<List<Vec3>> rails=List.of();
        long lastInput;
        float entityRate=1,bodyRadius=.3f,bodyHeight=1.8f;
        JakLeaper mount;
        JakHellcat craft;
        Vec3 boardingStart;
        int boardingTicks;
        void releaseCraft(){
            if(craft!=null)craft.release();
            craft=null;
        }
        boolean mountReleased;
        Vec3 mountHit;
        void releaseMount(){
            if(mount!=null&&mount.riderId()>=0)mount.release();
            mount=null;
            mountReleased=false;
        }
        Session(ServerPlayer player){nativeId=NativeJak.create(player.getYRot());origin=player.position();last=origin;dimension=player.level().dimension().identifier().toString();collision=new MinecraftCollision.Query(player.level(),player,origin);NativeJak.bindWorld(nativeId,collision);}
        public void close(){releaseMount();releaseCraft();NativeJak.destroy(nativeId);}
    }
    public static boolean active(Entity player){return player.getAttachedOrCreate(JakData.ENABLED)&&!player.isPassenger()&&player.isAlive();}
    public static void chooseMode(ServerPlayer player,boolean enabled){
        player.setAttached(JakData.ENABLED,enabled);
        player.setAttached(JakData.MODE_SELECTED,true);
    }
    public static void hostTeleport(ServerPlayer player){
        Session session=LIVE.get(player.getUUID());
        if(session!=null&&player.position().distanceToSqr(session.last)>1e-10)session.hostMoved=true;
    }
    public static void input(ServerPlayer player,JakPackets.Input input){
        if(CONTROLLED.contains(player.getUUID()))return;
        if(player.getVehicle() instanceof dev.jakmc.entity.JakVehicle car){car.input(player,input);return;}
        if(!active(player))return;
        Session s=LIVE.computeIfAbsent(player.getUUID(),id->new Session(player));
        if(input.sequence()<=s.input.sequence())return;
        s.input=input;s.pressed|=input.pressed();s.lastInput=player.level().getGameTime();
    }
    /** Queue after real Minecraft health damage, outside the native collision callback. */
    public static void hurt(ServerPlayer player,net.minecraft.world.damagesource.DamageSource source){
        if(!active(player)||!player.isAlive())return;
        Session session=LIVE.get(player.getUUID());if(session==null||session.hurts.size()>=8)return;
        int mode;
        if(source.is(net.minecraft.tags.DamageTypeTags.IS_EXPLOSION))mode=1;
        else if(source.is(net.minecraft.world.damagesource.DamageTypes.LIGHTNING_BOLT))mode=3;
        else if(source.is(net.minecraft.world.damagesource.DamageTypes.FALL))mode=5;
        else if(source.is(net.minecraft.world.damagesource.DamageTypes.ON_FIRE))mode=4;
        else if(source.is(net.minecraft.tags.DamageTypeTags.IS_FIRE))mode=2;
        else if(source.getEntity()!=null||source.is(net.minecraft.tags.DamageTypeTags.IS_PROJECTILE)
            ||source.is(net.minecraft.world.damagesource.DamageTypes.CACTUS)
            ||source.is(net.minecraft.world.damagesource.DamageTypes.SWEET_BERRY_BUSH))mode=0;
        else mode=4;
        if(session.craft!=null&&mode!=5)mode=4;
        session.hurts.add(new Hurt(mode,source.getSourcePosition()));
    }
    public static boolean mount(ServerPlayer player,JakLeaper mount){
        if(!active(player)||player.isSleeping()||player.isSpectator()||mount.riderId()>=0||
            !mount.isAlive()||!mount.readyToMount()||mount.level()!=player.level()||player.distanceToSqr(mount)>16||
            !player.hasLineOfSight(mount))return false;
        var session=LIVE.computeIfAbsent(player.getUUID(),id->new Session(player));
        if(session.mount!=null||session.craft!=null)return false;
        Vec3 at=mount.position().subtract(session.origin);
        NativeJak.mountControl(session.nativeId,0,(float)at.x,(float)at.y,(float)at.z,mount.getYRot());
        session.mount=mount;
        mount.reserve(player);
        return true;
    }
    public static boolean boardHellcat(ServerPlayer player,JakHellcat craft){
        if(!active(player)||player.isSleeping()||craft.pilotId()>=0)return false;
        var s=LIVE.computeIfAbsent(player.getUUID(),id->new Session(player));
        if(s.craft!=null||s.mount!=null)return false;
        s.craft=craft;
        s.boardingStart=player.position();
        s.boardingTicks=0;
        craft.reserve(player);
        return true;
    }
    public static void mountHit(ServerPlayer player,JakLeaper mount,Vec3 at){
        var session=LIVE.get(player.getUUID());
        if(session!=null&&session.mount==mount)session.mountHit=at;
    }

    public static void ventPulse(ServerPlayer player,boolean blue,int reaction){
        if(!active(player))return;
        var session=LIVE.get(player.getUUID());
        if(session!=null&&session.mount==null&&session.craft==null&&player.getVehicle()==null)
            NativeJak.ventPulse(session.nativeId,blue,reaction);
    }
    public static boolean onBoard(ServerPlayer player){
        Session session=LIVE.get(player.getUUID());
        return session!=null&&session.usingBoard;
    }

    public static void requestAltarMount(ServerPlayer player){
        LIVE.computeIfAbsent(player.getUUID(),id->new Session(player)).altarMount=true;
    }

    public static boolean blockWithLightShield(ServerPlayer player){
        if(!active(player))return false;
        var session=LIVE.get(player.getUUID());
        if(session==null||!session.shield)return false;
        session.shieldHit=true;
        return true;
    }

    public static void initialize(){
        ServerTickEvents.START_LEVEL_TICK.register(level->{
            if(level.tickRateManager().runsNormally()){JakEntityClock.of(level).advance();JakWorldDamage.tick(level);}
        });
        ServerTickEvents.END_SERVER_TICK.register(server->{
            var present=new HashSet<UUID>();
            for(ServerPlayer p:server.getPlayerList().getPlayers()){
                // Old development saves were manually enabled; new launcher
                // worlds were not. Migrate once, then respect explicit /jak off.
                if(!p.getAttachedOrCreate(JakData.MODE_SELECTED))chooseMode(p,true);
                JakResources.migrate(p);
                if(!active(p)||p.isSleeping()){JakResources.synchronize(p,JakResources.totals(p));continue;}
                present.add(p.getUUID());
                Session s=LIVE.get(p.getUUID());
                String dimension=p.level().dimension().identifier().toString();
                if(s!=null&&(s.hostMoved||!s.dimension.equals(dimension)||p.position().distanceToSqr(s.last)>16||s.mount!=null&&(s.mount.isRemoved()||s.mount.isDeadOrDying()||s.mount.level()!=p.level()))) {s.close();LIVE.remove(p.getUUID());s=null;}
                if(s==null){s=new Session(p);LIVE.put(p.getUUID(),s);}
                try{step(p,s);}catch(RuntimeException failure){s.close();LIVE.remove(p.getUUID());p.setDeltaMovement(Vec3.ZERO);JakMod.LOG.error("Jak session stopped at last safe Minecraft pose for {}",p.getScoreboardName(),failure);}
            }
            var it=LIVE.entrySet().iterator();while(it.hasNext()){var e=it.next();if(!present.contains(e.getKey())){e.getValue().close();it.remove();}}
            for(var level:server.getAllLevels()){
                float rate=1;
                for(var p:level.players()){
                    var session=LIVE.get(p.getUUID());
                    if(session!=null)rate=Math.min(rate,session.entityRate);
                }
                var clock=JakEntityClock.of(level);
                boolean synchronize=rate<1||clock.rate()!=rate||clock.skipped()>0&&level.getGameTime()%20==0;
                clock.setRate(rate);
                if(synchronize){
                    var packet=new JakPackets.EntityTime(level.dimension().identifier(),rate,clock.progress(),clock.skipped());
                    for(var p:level.players())if(ServerPlayNetworking.canSend(p,JakPackets.EntityTime.TYPE))ServerPlayNetworking.send(p,packet);
                }
            }
        });
        ServerPlayConnectionEvents.DISCONNECT.register((handler,server)->{var s=LIVE.remove(handler.getPlayer().getUUID());if(s!=null)s.close();});
        net.fabricmc.fabric.api.event.lifecycle.v1.ServerLifecycleEvents.SERVER_STOPPED.register(server->{for(var s:LIVE.values())s.close();LIVE.clear();CONTROLLED.clear();PROBE_INPUT.clear();});
    }
    private static void step(ServerPlayer p,Session s){
        float[] resources=JakResources.totals(p);
        JakResources.synchronize(p,resources);
        boolean rebased=!s.grinding&&p.position().distanceToSqr(s.origin)>256*256;
        if(rebased){Vec3 delta=p.position().subtract(s.origin);NativeJak.rebase(s.nativeId,(float)delta.x,(float)delta.y,(float)delta.z);s.origin=p.position();}
        s.collision.origin=s.origin;
        refreshGrindRails(p,s,rebased);
        boolean controlled=CONTROLLED.contains(p.getUUID());
        var input=controlled?PROBE_INPUT.get(p.getUUID()):s.input;
        boolean fresh=controlled||p.level().getGameTime()-s.lastInput<=5;
        int edges=fresh?(controlled?input.pressed():s.pressed):0;
        JsonObject flight=null;
        if(s.craft!=null){
            boolean missing=s.craft.isRemoved()||s.craft.level()!=p.level();
            boolean forced=missing||s.craft.wrecked();
            boolean leaving=forced||((edges&8)!=0&&s.boardingTicks>=14);
            Vec3 exit=leaving&&!missing?s.craft.exit(p.level()):null;
            if(forced||exit!=null){
                s.releaseCraft();
                relocate(s,exit==null?p.position():exit,new Vec3(0,1.92,0));
            }else{
                ++s.boardingTicks;
                flight=s.craft.advance(p.level(),p,fresh?input:null,edges,s.boardingTicks==14);
            }
        }
        if(s.mount!=null){
            if((edges&8)!=0)NativeJak.mountControl(s.nativeId,1,0,0,0,0);
            if(s.mountHit!=null){
                Vec3 hit=s.mountHit.subtract(s.origin);
                NativeJak.mountControl(s.nativeId,2,(float)hit.x,(float)hit.y,(float)hit.z,0);
                s.mountHit=null;
            }
        }
        int boardRequest=s.altarMount&&s.craft==null&&s.mount==null&&!p.isInWater()?BOARD_BUTTON:0;
        s.altarMount=false;
        ByteBuffer b=s.controls.clear();
        boolean equippedBoard=p.getAttachedOrCreate(JakData.BOARD).is(JakItems.BOARD);
        int nativeHeld=(fresh&&flight==null?input.held():0)|boardRequest;
        int nativePressed=flight==null?edges|boardRequest:0;
        if(!equippedBoard){nativeHeld&=~BOARD_BUTTON;nativePressed&=~BOARD_BUTTON;}
        b.putInt(nativeHeld).putInt(nativePressed);
        b.putFloat(fresh&&flight==null?input.strafe():0).putFloat(fresh&&flight==null?input.forward():0);
        b.putFloat(fresh?input.rightX():0).putFloat(fresh?input.rightY():0).putFloat(fresh?input.brake():0).putFloat(fresh?input.throttle():0);
        b.putFloat(input.yaw()).putFloat(input.pitch());
        float water=waterSurface(p,s.origin);
        var movement=p.getAttribute(net.minecraft.world.entity.ai.attributes.Attributes.MOVEMENT_SPEED);
        var vanillaSprint=movement.getModifier(VANILLA_SPRINT);
        double speed=movement.getValue();
        // Jak's stick and move state choose pace. Keep potion and equipment effects, but
        // remove the stock double-tap sprint multiplier sent by the Minecraft client.
        if(vanillaSprint!=null)speed/=1+vanillaSprint.amount();
        b.putFloat(Float.isFinite(water)?water:0).putFloat((float)(speed/.1));
        int flags=p.getAttachedOrCreate(JakData.BOARD).is(JakItems.BOARD)?1:0;
        if(Float.isFinite(water))flags|=16;
        if(p.hasInfiniteMaterials())flags|=32;
        if(flight!=null)flags|=2;
        if(p.hasEffect(net.minecraft.world.effect.MobEffects.FIRE_RESISTANCE))flags|=4;
        if(p.hasEffect(net.minecraft.world.effect.MobEffects.SLOW_FALLING))flags|=8;
        flags|=floor(p)<<8;
        if(!controlled||HOST_CONTROLLED.contains(p.getUUID())){
            flags|=1<<13; // Physical controls are routed by this host.
            if((input.held()&JakPackets.FIRST_PERSON)!=0)flags|=1<<14;
            if(!fresh||(input.held()&JakPackets.CANCEL_POWER)!=0)flags|=1<<15;
        }
        if(s.shieldHit){
            flags|=1<<16;
            s.shieldHit=false;
        }
        b.putInt(flags).putFloat(Math.min(100,resources[0])).putFloat(Math.min(100,resources[1]));
        b.putInt(s.mount!=null||flight!=null?-1:JakItems.GUNS.indexOf(p.getMainHandItem().getItem()));
        AABB region=p.getBoundingBox().inflate(6,5,6);
        var triangles=s.noTriangles; // Queries use the live Minecraft shape callback.
        s.actorBuffers.reset();var actors=flight!=null?s.noTriangles:MinecraftCollision.actors(p.level(),p,s.origin,region.inflate(256),s.actorBuffers);
        if(flight!=null)relocate(s,vector(flight.getAsJsonArray("seat")),Vec3.ZERO);
        NativeJak.setStats(s.nativeId,p.getHealth()/p.getMaxHealth(),p.getAttachedOrCreate(JakData.AMMO.get(0)),p.getAttachedOrCreate(JakData.AMMO.get(1)),p.getAttachedOrCreate(JakData.AMMO.get(2)),p.getAttachedOrCreate(JakData.AMMO.get(3)));
        boolean rideBody=s.mount!=null&&!s.mountReleased;
        float radius=rideBody?s.mount.getBbWidth()/2:.3f,height=rideBody?s.mount.getBbHeight()+1:1.8f;
        if(radius!=s.bodyRadius||height!=s.bodyHeight){
            NativeJak.bodyDimensions(s.nativeId,radius,height);
            s.bodyRadius=radius;s.bodyHeight=height;
        }
        for(var hurt:s.hurts){
            Vec3 from=hurt.from()==null?Vec3.ZERO:hurt.from().subtract(s.origin);
            NativeJak.hurt(s.nativeId,hurt.mode(),hurt.from()!=null,(float)from.x,(float)from.y,(float)from.z);
        }
        s.hurts.clear();
        JsonObject pose=JsonParser.parseString(NativeJak.step(s.nativeId,b,triangles,actors,3)).getAsJsonObject();
        s.shield=pose.get("shield").getAsBoolean();
        s.entityRate=pose.get("entityRate").getAsFloat();
        Vec3 position=vector(pose.getAsJsonArray("pos")).add(s.origin);
        if(flight!=null){
            Vec3 seat=vector(flight.getAsJsonArray("seat"));
            float reach=Math.min(1,s.boardingTicks/(20f*10/30));
            double eased=Math.sin(reach*Math.PI/2);
            var boarding=s.boardingStart;
            position=new Vec3(net.minecraft.util.Mth.lerp(eased,boarding.x,seat.x),
                net.minecraft.util.Mth.lerp(reach,boarding.y,seat.y),net.minecraft.util.Mth.lerp(eased,boarding.z,seat.z));
            float yaw=net.minecraft.util.Mth.rotLerp(reach,input.yaw(),s.craft.getYRot());
            JsonObject channel=new JsonObject();
            channel.addProperty("entity",s.craft.getId());
            channel.add("offset",coordinates(s.craft.position().subtract(position)));
            channel.add("rotation",flight.get("rotation"));
            channel.addProperty("boarding",s.boardingTicks<14?s.boardingTicks/20f:-1);
            channel.add("steering",flight.get("steering"));
            channel.add("throttle",flight.get("throttle"));
            channel.add("lean",flight.get("lean"));
            channel.add("lamps",flight.get("lamps"));
            channel.addProperty("speed",vector(flight.getAsJsonArray("velocity")).length());
            channel.add("boost",flight.get("boost"));
            for(var event:flight.getAsJsonArray("events")){
                if(event.getAsJsonObject().get("kind").getAsString().equals("fire"))channel.addProperty("fired",true);
            }
            pose.add("hellcat",channel);
            pose.add("pos",coordinates(position.subtract(s.origin)));
            pose.addProperty("yaw",yaw);
            pose.addProperty("status",0);
            pose.addProperty("board",false);
            pose.getAsJsonArray("events").asList().clear();
        }
        s.usingBoard=pose.get("board").getAsBoolean();
        if(s.usingBoard&&position.distanceToSqr(s.last)>.0001&&++s.boardUseTicks>=20){
            s.boardUseTicks=0;
            JakBoards.wear(p);
        }
        s.grinding=s.usingBoard&&pose.get("state").getAsString().equals("board-grind");
        if(!Double.isFinite(position.x)||!Double.isFinite(position.y)||!Double.isFinite(position.z))throw new IllegalStateException("Nonfinite Jak pose");
        if(s.mount!=null){
            var ride=pose.get("ride");
            var dismounted=pose.get("dismounted");
            if(dismounted.isJsonObject()){
                var landing=dismounted.getAsJsonObject();
                updateMount(s.mount,vector(landing.getAsJsonArray("pos")).add(s.origin),landing.get("yaw").getAsFloat(),(pose.get("status").getAsInt()&2)!=0);
                s.mountReleased=true;
            }
            if(ride.isJsonObject()){
                var channel=ride.getAsJsonObject();
                channel.addProperty("entity",s.mount.getId());
                channel.addProperty("flut",s.mount instanceof dev.jakmc.entity.JakFlutFlut);
                if(s.mountReleased)channel.addProperty("fixed",true);
                if(!s.mountReleased&&!channel.get("fixed").getAsBoolean()){
                    Vec3 requested=position;
                    position=updateMount(s.mount,requested,pose.get("yaw").getAsFloat(),(pose.get("status").getAsInt()&2)!=0);
                    if(position.distanceToSqr(requested)>1e-10){
                        Vec3 velocity=vector(pose.getAsJsonArray("velocity"));
                        velocity=new Vec3(Math.abs(position.x-requested.x)>1e-5?0:velocity.x,
                            Math.abs(position.y-requested.y)>1e-5?0:velocity.y,
                            Math.abs(position.z-requested.z)>1e-5?0:velocity.z);
                        relocate(s,position,velocity);
                        pose.add("pos",coordinates(position.subtract(s.origin)));
                        pose.add("velocity",coordinates(velocity));
                    }
                }
            }else s.releaseMount();
        }
        p.setPos(position);
        // The native solver owns movement and cancels the vanilla move packet.
        // Publish its section changes through the same server chunk tracker.
        if(!p.getLastSectionPos().equals(net.minecraft.core.SectionPos.of(p))){
            p.level().getChunkSource().move(p);
        }
        p.setDeltaMovement(Vec3.ZERO);p.setOnGround((pose.get("status").getAsInt()&2)!=0);p.resetFallDistance();
        p.setYRot(input.yaw());p.setXRot(input.pitch());p.setYBodyRot(pose.get("yaw").getAsFloat());s.last=position;s.pressed=0;
        if(controlled)probeInput(p,input.held(),0,input.forward());
        JakResources.consume(p,0,Math.max(0,Math.min(100,resources[0])-pose.get("darkEco").getAsFloat()));
        JakResources.consume(p,1,Math.max(0,Math.min(100,resources[1])-pose.get("lightEco").getAsFloat()));
        float[] caps={50,100,100,20};
        for(int i=0;i<4;i++)JakResources.consume(p,i+2,Math.max(0,Math.min(caps[i],resources[i+2])-pose.getAsJsonArray("ammo").get(i).getAsFloat()));
        JakResources.synchronize(p,JakResources.totals(p));
        if(pose.get("heal").getAsFloat()>0)p.heal(pose.get("heal").getAsFloat()*p.getMaxHealth());
        for(JsonElement e:pose.getAsJsonArray("events")){
            applyEvent(p,s.origin,e.getAsJsonObject());
            if(s.hostMoved)return;
        }
        pose.addProperty("serverTick",p.level().getGameTime());
        var packet=new JakPackets.Pose(p.getId(),input.sequence(),s.origin.x,s.origin.y,s.origin.z,pose.toString());
        ServerPlayNetworking.send(p,packet);
        for(ServerPlayer watcher:PlayerLookup.tracking(p))if(watcher!=p)ServerPlayNetworking.send(watcher,packet);
    }
    private static void refreshGrindRails(ServerPlayer player,Session session,boolean rebased){
        List<List<Vec3>> lines=session.usingBoard?session.collision.rails(player.position()):List.of();
        if(lines.equals(session.rails)&&!rebased)return;
        int bytes=0;
        for(var line:lines)bytes+=4+line.size()*12;
        var buffer=NativeJak.buffer(bytes);
        for(var line:lines){
            buffer.putInt(line.size());
            for(var point:line)buffer.putFloat((float)(point.x-session.origin.x))
                .putFloat((float)(point.y-session.origin.y)).putFloat((float)(point.z-session.origin.z));
        }
        NativeJak.setGrindRails(session.nativeId,buffer);
        session.rails=lines;
    }

    private static void relocate(Session s,Vec3 at,Vec3 velocity){
        Vec3 pos=at.subtract(s.origin);
        NativeJak.relocate(s.nativeId,(float)pos.x,(float)pos.y,(float)pos.z,
            (float)velocity.x,(float)velocity.y,(float)velocity.z);
    }
    private static JsonArray coordinates(Vec3 v){
        var array=new JsonArray();
        array.add(v.x);array.add(v.y);array.add(v.z);
        return array;
    }
    private static Vec3 updateMount(JakLeaper mount,Vec3 position,float yaw,boolean ground){
        // Native spheres resolve the source motion; the host box also covers
        // corners of the actual Minecraft mount rather than teleporting into them.
        mount.move(net.minecraft.world.entity.MoverType.SELF,position.subtract(mount.position()));
        mount.setYRot(yaw);
        mount.setYBodyRot(yaw);
        mount.setYHeadRot(yaw);
        mount.setOnGround(ground);
        return mount.position();
    }
    private static float waterSurface(ServerPlayer p,Vec3 origin){
        var level=p.level();
        var surface=p.blockPosition().mutable();
        var fluid=level.getFluidState(surface);
        if(!fluid.is(net.minecraft.tags.FluidTags.WATER))return Float.NaN;
        while(surface.getY()<level.getMaxY()){
            var above=level.getFluidState(surface.above());
            if(!above.is(net.minecraft.tags.FluidTags.WATER))break;
            surface.move(net.minecraft.core.Direction.UP);
            fluid=above;
        }
        return (float)(surface.getY()-origin.y+fluid.getHeight(level,surface));
    }
    private static int floor(ServerPlayer p){
        if(p.isInWater())return p.isUnderWater()?22:21;
        var state=p.level().getBlockState(net.minecraft.core.BlockPos.containing(p.getX(),p.getY()-.05,p.getZ()));
        return FLOOR_SOUNDS.computeIfAbsent(state,s->NativeJak.floor(net.minecraft.core.registries.BuiltInRegistries.BLOCK.getKey(s.getBlock()).getPath(),s.getSoundType().getStepSound().location().toString()));
    }
    public static Vec3 vector(JsonArray a){return new Vec3(a.get(0).getAsDouble(),a.get(1).getAsDouble(),a.get(2).getAsDouble());}
    private static void applyEvent(ServerPlayer p,Vec3 origin,JsonObject e){
        String kind=e.get("kind").getAsString();
        if(kind.equals("world_hit")){
            Vec3 target=vector(e.getAsJsonArray("pos")).add(origin);
            Vec3 normal=vector(e.getAsJsonArray("normal"));
            JakWorldDamage.hit(p,net.minecraft.core.BlockPos.containing(target.subtract(normal.scale(.03))),e.get("damage").getAsFloat());
            return;
        }
        if(kind.equals("explosion")){
            float power=switch(e.get("gun").getAsString()){
                case "Red3" -> 4;case "Dark1" -> 3;case "Dark3" -> 9;case "Yellow3" -> 1.5f;default -> 0;
            };
            JakWorldDamage.explosion(p.level(),p,p,vector(e.getAsJsonArray("pos")).add(origin),power);
            return;
        }
        if(kind.equals("ground_pound")){
            dev.jakmc.content.JakEcoVent.groundPound(p.level(),p,vector(e.getAsJsonArray("pos")).add(origin));
            return;
        }
        if((kind.equals("strike")||kind.equals("impact"))&&e.has("actor")&&!e.get("actor").isJsonNull()){
            Entity victim=p.level().getEntity(e.get("actor").getAsInt());
            if(victim!=null&&victim!=p){
                boolean heavy=e.has("heavy")&&e.get("heavy").getAsBoolean();
                var source=kind.equals("impact")?JakDamage.shot(p,heavy):p.damageSources().playerAttack(p);
                float damage=e.get("damage").getAsFloat()*2.5f;
                if(victim instanceof dev.jakmc.entity.JakEnemy enemy)enemy.hurtFromJak(p.level(),source,damage,heavy);else victim.hurtServer(p.level(),source,damage);
            }
        }else if(kind.equals("impact")||kind.equals("strike")){
            Vec3 target=vector(e.getAsJsonArray("pos")).add(origin);
            if(kind.equals("impact")){
                Vec3 normal=vector(e.getAsJsonArray("normal"));
                var at=net.minecraft.core.BlockPos.containing(target.subtract(normal.scale(.03)));
                JakWorldDamage.hit(p,at,e.get("damage").getAsFloat());
            }else{
                var ray=new net.minecraft.world.level.ClipContext(p.position().add(0,1,0),target,
                    net.minecraft.world.level.ClipContext.Block.COLLIDER,
                    net.minecraft.world.level.ClipContext.Fluid.NONE,p);
                var hit=p.level().clip(ray);
                if(hit.getType()==net.minecraft.world.phys.HitResult.Type.BLOCK)
                    JakWorldDamage.hit(p,hit.getBlockPos(),e.get("damage").getAsFloat());
            }
        } else if(kind.equals("fall_damage")) p.hurtServer(p.level(),p.damageSources().fall(),e.get("damage").getAsFloat()*2.5f);
        else if(kind.equals("lift")){
            Entity victim=p.level().getEntity(e.get("actor").getAsInt());
            if(victim instanceof net.minecraft.world.entity.LivingEntity living)living.addEffect(new net.minecraft.world.effect.MobEffectInstance(net.minecraft.world.effect.MobEffects.LEVITATION,(int)(20*e.get("seconds").getAsFloat()),1));
        }
    }
}
