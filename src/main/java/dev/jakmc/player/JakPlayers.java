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
    // An edge waits for the native step that reads it, even when two fixture ticks land before one server tick.
    public static void probeInput(ServerPlayer p,int held,int pressed,float forward){var old=PROBE_INPUT.get(p.getUUID());probeSet(p,held,pressed|(old==null?0:old.pressed()),forward);}
    private static void probeSet(ServerPlayer p,int held,int pressed,float forward){PROBE_INPUT.put(p.getUUID(),new JakPackets.Input(0,held,pressed,0,forward,0,0,p.getYRot(),p.getXRot(),0,0));}
    private static final Map<UUID,Session> LIVE=new HashMap<>();
    public static void resetForProbe(ServerPlayer player){
        if(!CONTROLLED.contains(player.getUUID()))throw new IllegalStateException("Only a controlled development fixture can reset its native session");
        var old=LIVE.remove(player.getUUID());if(old!=null)old.close();
    }
    /** The native session a controlled fixture's Jak runs in (0: none), to check it survives host moves. */
    public static long probeSession(ServerPlayer player){
        if(!CONTROLLED.contains(player.getUUID()))throw new IllegalStateException("Only a controlled development fixture can inspect its native session");
        var s=LIVE.get(player.getUUID());return s==null?0:s.nativeId;
    }
    private static final int BOARD_BUTTON=1<<7;
    /** target-pilot-get-off (pilot-states.gc): a pilot let off a vehicle takes the original exit, with the vehicle's velocity (blocks a second). */
    private record PilotExit(boolean bike,Vec3 velocity,long tick){}
    private static final java.util.Map<java.util.UUID,PilotExit> PILOT_EXITS=new java.util.HashMap<>();
    /** The pilot is being let off: the Jak made for him next takes the get-off, then the jump, then the fall (NativeJak.pilotExit). */
    public static void pilotExit(ServerPlayer player,boolean bike,Vec3 velocity){
        PILOT_EXITS.put(player.getUUID(),new PilotExit(bike,velocity,player.level().getGameTime()));
    }
    /** Whether this player is leaving a vehicle by the original exit (the vehicle then puts him down on its seat). */
    public static boolean pilotExiting(ServerPlayer player){return PILOT_EXITS.containsKey(player.getUUID());}
    /** Adrii 06/11: three times Minecraft's reach to place blocks; attacking keeps its own (entity) reach. */
    public static final net.minecraft.resources.Identifier REACH=dev.jakmc.JakData.id("jak_block_reach");
    private static final net.minecraft.resources.Identifier VANILLA_SPRINT=
        net.minecraft.resources.Identifier.withDefaultNamespace("sprinting");
    private static final Map<net.minecraft.world.level.block.state.BlockState,Integer> FLOOR_SOUNDS=new IdentityHashMap<>();
    private record Hurt(int mode,Vec3 from){}
    private static final class Session implements AutoCloseable {
        /** The blocks this spin kick has already struck: each takes it once. */
        final java.util.HashSet<net.minecraft.core.BlockPos> kicked=new java.util.HashSet<>();
        final long nativeId;
        MinecraftCollision.Query collision;
        /** The Minecraft body this session was made for; a respawn is a new one. */
        final ServerPlayer owner;
        final ByteBuffer controls=NativeJak.buffer(68),noTriangles=NativeJak.buffer(0);
        final DirectBufferArena actorBuffers=new DirectBufferArena(1024*1024);
        Vec3 origin;
        String dimension;
        Vec3 last;
        /** The native state last step (minecraft_survival.rs: a jump's hunger as Jak goes into "jump"). */
        String lastState="";
        boolean wideActorQuery;
        JakPackets.Input input=new JakPackets.Input(0,0,0,0,0,0,0,0,0,0,0);
        int pressed;
        final ArrayList<Hurt> hurts=new ArrayList<>();
        final ArrayList<Vec3> tongues=new ArrayList<>();
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
        Session(ServerPlayer player){owner=player;nativeId=NativeJak.create(player.getYRot());origin=player.position();last=origin;dimension=player.level().dimension().identifier().toString();collision=new MinecraftCollision.Query(player.level(),player,origin);NativeJak.bindWorld(nativeId,collision);}
        public void close(){releaseMount();releaseCraft();NativeJak.destroy(nativeId);}
        /**
         * The host moved Jak (a portal, /tp, an ender pearl): carry the same Jak
         * there, keeping his form, eco, gun and board, instead of starting a new
         * one. A new dimension is a new world to collide with.
         */
        void carry(ServerPlayer player){
            String now=player.level().dimension().identifier().toString();
            if(!now.equals(dimension)){
                collision=new MinecraftCollision.Query(player.level(),player,player.position());
                NativeJak.bindWorld(nativeId,collision);
                dimension=now;
            }
            origin=player.position();
            collision.origin=origin;
            NativeJak.relocate(nativeId,0,0,0,0,0,0);
            last=origin;
            hostMoved=false;
            hurts.clear();
            rails=null;
        }
    }
    public static boolean active(Entity player){return player.getAttachedOrCreate(JakData.ENABLED)&&!player.isPassenger()&&player.isAlive();}
    /** Hanging from a ledge, or climbing up or letting go of it (target-edge-grab and its two ways out). */
    public static boolean hanging(Entity player){var s=LIVE.get(player.getUUID());return s!=null&&s.lastState.startsWith("edge-grab");}
    public static void chooseMode(ServerPlayer player,boolean enabled){
        player.setAttached(JakData.ENABLED,enabled);
        player.setAttached(JakData.MODE_SELECTED,true);
    }
    /** The rescue (K): a fresh Jak next tick, off mount and craft (render_anim/src/jak.rs update, `Jak::new`). */ public static void rescue(ServerPlayer player){var s=LIVE.remove(player.getUUID());if(s!=null)s.close();}
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
    /** kermit.gc kermit-tongue-stuck: a frog's tongue, its base at `from`, on this Jak this tick. */
    /** For the scenes: frog tongue pulls queued. */
    public static int tonguePulls;
    public static void tongue(ServerPlayer player,Vec3 from){
        if(!active(player))return;
        Session session=LIVE.get(player.getUUID());if(session!=null&&session.tongues.size()<8){session.tongues.add(from);tonguePulls++;}
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
                boolean moved=s!=null&&(s.hostMoved||!s.dimension.equals(dimension)||p.position().distanceToSqr(s.last)>16);
                boolean mountLost=s!=null&&s.mount!=null&&(s.mount.isRemoved()||s.mount.isDeadOrDying()||s.mount.level()!=p.level());
                if(s!=null&&(s.owner!=p||mountLost||moved&&(s.mount!=null||s.craft!=null))) {s.close();LIVE.remove(p.getUUID());s=null;}
                else if(moved)s.carry(p);
                if(s==null){s=new Session(p);LIVE.put(p.getUUID(),s);}
                var exit=PILOT_EXITS.remove(p.getUUID());
                if(exit!=null&&p.level().getGameTime()-exit.tick()<=5)
                    NativeJak.pilotExit(s.nativeId,exit.bike(),(float)exit.velocity().x,(float)exit.velocity().y,(float)exit.velocity().z);
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
                // jak_entities.rs control_vehicle: an exploding craft throws him off (blownOff).
                Vec3[] thrown=!missing&&s.craft.wrecked()?s.craft.blownOff():null;
                s.releaseCraft();
                if(thrown!=null)relocate(s,thrown[0],thrown[1]);
                else relocate(s,exit==null?p.position():exit,new Vec3(0,1.92,0));
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
        s.collision.board(s.usingBoard);
        float water=waterSurface(p,s.origin,s.usingBoard);
        var blockReach=p.getAttribute(net.minecraft.world.entity.ai.attributes.Attributes.BLOCK_INTERACTION_RANGE);
        if(blockReach!=null&&!blockReach.hasModifier(REACH))blockReach.addTransientModifier(new net.minecraft.world.entity.ai.attributes.AttributeModifier(
            REACH,2,net.minecraft.world.entity.ai.attributes.AttributeModifier.Operation.ADD_MULTIPLIED_BASE));
        var movement=p.getAttribute(net.minecraft.world.entity.ai.attributes.Attributes.MOVEMENT_SPEED);
        var vanillaSprint=movement.getModifier(VANILLA_SPRINT);
        double speed=movement.getValue();
        // Jak's stick and move state choose pace. Keep potion and equipment effects, but
        // remove the stock double-tap sprint multiplier sent by the Minecraft client.
        if(vanillaSprint!=null)speed/=1+vanillaSprint.amount();
        b.putFloat(Float.isFinite(water)?water:0).putFloat((float)(speed/.1));
        int flags=p.getAttachedOrCreate(JakData.BOARD).is(JakItems.BOARD)?1:0;
        if(Float.isFinite(water))flags|=16;
        // Creative supplies the Morph Gun and powers without spending carried resources.
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
        int equippedGun=s.mount!=null||flight!=null?-1:JakItems.GUNS.indexOf(p.getMainHandItem().getItem());
        b.putInt(equippedGun);
        // Rust minecraft_survival.rs: Jump Boost scales Jak's jump as it does a player's, (0.42 + 0.1·level) / 0.42.
        b.putFloat((float)((.42+p.getJumpBoostPower())/.42));
        AABB region=p.getBoundingBox().inflate(6,5,6);
        var triangles=s.noTriangles; // Queries use the live Minecraft shape callback.
        // Melee and powers reach the nearby cells. Keep the wider query for equipped guns and shots still in flight.
        s.actorBuffers.reset();
        double actorReach=equippedGun>=0||s.wideActorQuery?256:40;
        var actors=flight!=null?s.noTriangles:MinecraftCollision.actors(p.level(),p,s.origin,region.inflate(actorReach),s.actorBuffers);
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
        for(var tongue:s.tongues){Vec3 from=tongue.subtract(s.origin);NativeJak.tongue(s.nativeId,(float)from.x,(float)from.y,(float)from.z);}
        s.tongues.clear();
        JsonObject pose=JsonParser.parseString(NativeJak.step(s.nativeId,b,triangles,actors,3)).getAsJsonObject();
        s.wideActorQuery=pose.has("wideActorQuery")&&pose.remove("wideActorQuery").getAsBoolean();
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
        // Rust minecraft_game survival.rs: on foot (the board too) every block run costs 0.05 of exhaustion,
        // a jump 0.05 (minecraft_world.rs, under 4 blocks a step: no teleports).
        if(s.last!=null&&flight==null&&s.mount==null){
            double run=Math.hypot(position.x-s.last.x,position.z-s.last.z);
            if(run<4)p.causeFoodExhaustion((float)(.05*run));
        }
        String nativeState=pose.get("state").getAsString();
        boolean jumped=nativeState.equals("jump")&&!s.lastState.equals("jump");
        if(jumped)p.causeFoodExhaustion(.05f);
        s.lastState=nativeState;strikeState=nativeState;
        burn(p,s);
        BLUE_ECO.put(p,pose.has("blueEco")&&pose.get("blueEco").getAsBoolean());
        // generic-obs.gc launcher (stand-in: the block under the feet), its 'launch with blue eco. stand-in (Adrii 06/11):
        // the original throws Jak as he touches it; here as he jumps from it (the step his jump starts, from where he stood).
        Vec3 stood=s.last!=null?s.last:position;
        if(flight==null&&s.mount==null&&jumped
            &&p.level().getBlockState(net.minecraft.core.BlockPos.containing(stood.x,stood.y-.05,stood.z)).is(dev.jakmc.content.JakEcoLauncher.BLOCK)
            &&NativeJak.launch(s.nativeId,dev.jakmc.content.JakEcoLauncher.SPRING_METRES)){
            // Adrii 06/11, its sounds with the rest: launch-fire stands some 5 dB over Jak 1's (mean -18.6 dB against a median -23).
            var fire=net.minecraft.sounds.SoundEvent.createVariableRangeEvent(dev.jakmc.JakData.id("sfx/jak1/launch_fire"));
            p.level().playSound(null,position.x,position.y,position.z,fire,net.minecraft.sounds.SoundSource.PLAYERS,.55f,1);
        }
        p.setYRot(input.yaw());p.setXRot(input.pitch());p.setYBodyRot(pose.get("yaw").getAsFloat());s.last=position;s.pressed=0;
        if(controlled)probeSet(p,input.held(),0,input.forward());
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
        boolean kicking=KICKS.contains(nativeState);
        if(pose.get("attackSpheres") instanceof JsonArray swept)struckInSpheres(p,s.origin,swept,kicking,s.kicked);
        if(!kicking)s.kicked.clear();
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

    /** A game metre in blocks (Jak at 0.64). */
    public static final double METRE=.64;
    private static final java.util.Map<net.minecraft.world.entity.player.Player,Boolean> BLUE_ECO=new java.util.WeakHashMap<>();
    /** Whether this Jak carries blue eco now (eco_power.rs has_eco), for the launchers and the iris doors. */
    public static boolean blueEco(net.minecraft.world.entity.player.Player p){return BLUE_ECO.getOrDefault(p,false);}
    /** Jak set down at `at` moving at `velocity` (blocks a second): thrown by a blast. */
    public static void blownOff(ServerPlayer p,Vec3 at,Vec3 velocity){
        Session s=LIVE.get(p.getUUID());
        if(s==null||!active(p)||s.craft!=null||s.mount!=null)return;
        relocate(s,at,velocity);
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
    /** The liquid's top under Jak; on the board dark eco's too, which it rides as water (Rust minecraft_world board water + dark_eco::board_triangles). */
    /**
     * Rust minecraft_world.rs, Jak alight: standing on lava burns him too (feet 0.35 under), as in it for 15 s;
     * alight he loses a point a second, in lava as well (vanilla skips it there, Jak only burns in it); the
     * shield puts it out (water does in vanilla). Lava's and a fire block's own hurt is not his (JakShieldMixin).
     */
    private static void burn(ServerPlayer p,Session s){
        if(s.shield){if(p.getRemainingFireTicks()>0)p.clearFire();return;}
        if(p.hasEffect(net.minecraft.world.effect.MobEffects.FIRE_RESISTANCE))return;
        var under=net.minecraft.core.BlockPos.containing(p.getX(),p.getY()-.35,p.getZ());
        if(p.level().getFluidState(under).is(net.minecraft.tags.FluidTags.LAVA))p.setRemainingFireTicks(Math.max(p.getRemainingFireTicks(),300));
        if(p.isInLava()&&p.getRemainingFireTicks()>0&&p.getRemainingFireTicks()%20==0)p.hurtServer(p.level(),p.damageSources().onFire(),1);
    }
    /** Rust minecraft_world.rs LAVA_SURFACE: the top lava cell from the feet + 3 down to the feet − 1, for Jak to swim in. */
    private static float lavaSurface(ServerPlayer p,Vec3 origin){
        var level=p.level();var feet=p.blockPosition();
        for(int dy=3;dy>=-1;dy--){
            var at=feet.above(dy);var fluid=level.getFluidState(at);
            if(fluid.is(net.minecraft.tags.FluidTags.LAVA))return (float)(at.getY()-origin.y+fluid.getHeight(level,at));
        }
        return Float.NaN;
    }
    private static float waterSurface(ServerPlayer p,Vec3 origin,boolean board){
        var level=p.level();
        var surface=p.blockPosition().mutable();
        var fluid=level.getFluidState(surface);
        java.util.function.Predicate<net.minecraft.world.level.material.FluidState> liquid=f->f.is(net.minecraft.tags.FluidTags.WATER)
            ||board&&f.getType().isSame(dev.jakmc.content.darkeco.DarkEco.SOURCE);
        if(!liquid.test(fluid)&&board){surface.move(net.minecraft.core.Direction.DOWN);fluid=level.getFluidState(surface);
            if(!fluid.getType().isSame(dev.jakmc.content.darkeco.DarkEco.SOURCE)){surface.move(net.minecraft.core.Direction.UP);fluid=level.getFluidState(surface);}}
        if(!liquid.test(fluid))return board?Float.NaN:lavaSurface(p,origin);
        while(surface.getY()<level.getMaxY()){
            var above=level.getFluidState(surface.above());
            if(!liquid.test(above))break;
            surface.move(net.minecraft.core.Direction.UP);
            fluid=above;
        }
        return (float)(surface.getY()-origin.y+fluid.getHeight(level,surface));
    }
    private static final java.util.Map<ServerPlayer,Integer> LAST_FLOOR=new java.util.WeakHashMap<>();
    private static int floor(ServerPlayer p){
        if(p.isInWater())return p.isUnderWater()?22:21;
        // Rust minecraft_world.rs: past air, 0.2, 0.7 and 1.2 below, and in the air the floor last stood on
        // (a landing on leaves played stone, the air under him).
        net.minecraft.world.level.block.state.BlockState state=null;
        for(double below:new double[]{.05,.2,.7,1.2}){
            var at=p.level().getBlockState(net.minecraft.core.BlockPos.containing(p.getX(),p.getY()-below,p.getZ()));
            if(!at.isAir()){state=at;break;}
        }
        if(state==null)return LAST_FLOOR.getOrDefault(p,0);
        int floor=floorOf(state);
        LAST_FLOOR.put(p,floor);
        return floor;
    }
    private static int floorOf(net.minecraft.world.level.block.state.BlockState state){
        return FLOOR_SOUNDS.computeIfAbsent(state,s->NativeJak.floor(net.minecraft.core.registries.BuiltInRegistries.BLOCK.getKey(s.getBlock()).getPath(),s.getSoundType().getStepSound().location().toString()));
    }
    public static Vec3 vector(JsonArray a){return new Vec3(a.get(0).getAsDouble(),a.get(1).getAsDouble(),a.get(2).getAsDouble());}
    /** The block each player's blows last took from Minecraft's outline. */
    /** Fixture diagnostics: blows on the world, and those that found their block. */
    public static final java.util.concurrent.atomic.AtomicInteger WORLD_STRIKES=new java.util.concurrent.atomic.AtomicInteger(),
        WORLD_STRIKES_FOUND=new java.util.concurrent.atomic.AtomicInteger();
    private static net.minecraft.world.phys.BlockHitResult clip(ServerPlayer p,Vec3 from,Vec3 to){
        var hit=p.level().clip(new net.minecraft.world.level.ClipContext(from,to,
            net.minecraft.world.level.ClipContext.Block.COLLIDER,net.minecraft.world.level.ClipContext.Fluid.NONE,p));
        return hit.getType()==net.minecraft.world.phys.HitResult.Type.BLOCK&&!hit.isInside()?hit:null;
    }
    /** The native state of the step whose events are being applied (what kind of blow a strike is). */
    private static String strikeState="";
    // Rust jak.rs 3454-3472: the kicks and the pounds among the blows (the rest are punches).
    private static final java.util.Set<String> KICKS=java.util.Set.of("attack","attack-air","board-jump-kick","board-wall-kick"),
        POUNDS=java.util.Set.of("flop","flop-hit-ground");
    /**
     * Rust minecraft_world.rs, a blow on a block: a bell rings to a kick (and calls the villagers, CitizenBellMixin)
     * and breaks to a pound, a punch leaves it; a kick works a block that is used by hand (minecraft_game
     * interaction.rs: lever, repeater, comparator, note block, buttons, wooden doors, trapdoors and gates)
     * and does not damage it. Kicks only, not punches (Adrii 06/11; the Rust's punches worked them too).
     */
    private static void strikeBlock(ServerPlayer p,net.minecraft.core.BlockPos at,float damage,boolean kick){
        var level=p.level();var state=level.getBlockState(at);
        boolean pound=POUNDS.contains(strikeState);
        if(state.getBlock() instanceof net.minecraft.world.level.block.BellBlock bell){
            if(kick)bell.attemptToRing(p,level,at,null);
            else if(pound)level.destroyBlock(at,true,p);
            return;
        }
        if(kick&&workedByHand(state)){
            state.useWithoutItem(level,p,new net.minecraft.world.phys.BlockHitResult(Vec3.atCenterOf(at),net.minecraft.core.Direction.UP,at,false));
            return;
        }
        if(!kick)JakWorldDamage.hit(p,at,damage);
    }
    /**
     * A kick works switches, cuts plants and opens ammunition crates in its swept volume.
     */
    private static void struckInSpheres(ServerPlayer p,Vec3 origin,JsonArray swept,boolean lastKick,java.util.Set<net.minecraft.core.BlockPos> kicked){
        var level=p.level();var seen=new java.util.HashSet<net.minecraft.core.BlockPos>();
        for(JsonElement e:swept){
            var a=e.getAsJsonArray();Vec3 c=new Vec3(a.get(0).getAsDouble(),a.get(1).getAsDouble(),a.get(2).getAsDouble()).add(origin);double r=a.get(3).getAsDouble();
            boolean kick=a.size()>4?a.get(4).getAsBoolean():lastKick;
            var box=new net.minecraft.world.phys.AABB(c,c).inflate(r);
            for(var at:net.minecraft.core.BlockPos.betweenClosed(net.minecraft.core.BlockPos.containing(box.minX,box.minY,box.minZ),net.minecraft.core.BlockPos.containing(box.maxX,box.maxY,box.maxZ))){
                var state=level.getBlockState(at);
                if(!kick&&state.is(dev.jakmc.content.JakAmmoCrate.BLOCK)){
                    if(a.size()>5&&a.get(5).getAsBoolean()&&!punchCanReach(p,at,p.getViewVector(1)))continue;
                    var crate=state.getCollisionShape(level,at).bounds().move(at);
                    Vec3 inside=new Vec3(Math.clamp(c.x,crate.minX,crate.maxX),Math.clamp(c.y,crate.minY,crate.maxY),Math.clamp(c.z,crate.minZ,crate.maxZ));
                    if(inside.distanceToSqr(c)<=r*r&&seen.add(at.immutable()))JakWorldDamage.hit(p,at.immutable(),1);
                    continue;
                }
                if(!kick||state.isAir()||!state.getFluidState().isEmpty()||at.getY()+1<=p.getY()+.05)continue;
                // The cell's nearest point inside the sphere.
                Vec3 near=new Vec3(Math.clamp(c.x,at.getX(),at.getX()+1),Math.clamp(c.y,at.getY(),at.getY()+1),Math.clamp(c.z,at.getZ(),at.getZ()+1));
                if(near.distanceToSqr(c)>r*r||!kicked.add(at.immutable()))continue;
                if(state.getBlock() instanceof net.minecraft.world.level.block.BellBlock||workedByHand(state)){
                    // A door is worked once, by its lower half.
                    if(!state.hasProperty(net.minecraft.world.level.block.state.properties.BlockStateProperties.DOUBLE_BLOCK_HALF)
                        ||state.getValue(net.minecraft.world.level.block.state.properties.BlockStateProperties.DOUBLE_BLOCK_HALF)==net.minecraft.world.level.block.state.properties.DoubleBlockHalf.LOWER)strikeBlock(p,at.immutable(),0,true);
                }else JakWorldDamage.kick(p,at.immutable());
            }
        }
    }
    // minecraft_game interaction.rs uses_by_hand
    private static boolean workedByHand(net.minecraft.world.level.block.state.BlockState state){
        String id=net.minecraft.core.registries.BuiltInRegistries.BLOCK.getKey(state.getBlock()).getPath();
        return id.equals("lever")||id.equals("repeater")||id.equals("comparator")||id.equals("note_block")||id.endsWith("_button")
            ||id.endsWith("_door")&&!id.equals("iron_door")||id.endsWith("_trapdoor")&&!id.equals("iron_trapdoor")||id.endsWith("_fence_gate");
    }
    private static boolean punchCanReach(ServerPlayer player,net.minecraft.core.BlockPos block,Vec3 direction){
        if(block.getY()<net.minecraft.util.Mth.ceil(player.getY()-.05))return false;
        Vec3 forward=direction.multiply(1,0,1).normalize();
        return Vec3.atCenterOf(block).subtract(player.position()).dot(forward)>.15;
    }
    private static void applyEvent(ServerPlayer p,Vec3 origin,JsonObject e){
        String kind=e.get("kind").getAsString();
        boolean kick=kind.equals("strike")&&(e.has("kick")?e.get("kick").getAsBoolean():KICKS.contains(strikeState));
        boolean punch=kind.equals("strike")&&e.has("punch")&&e.get("punch").getAsBoolean();
        if(kind.equals("world_hit")){
            Vec3 target=vector(e.getAsJsonArray("pos")).add(origin);
            Vec3 normal=vector(e.getAsJsonArray("normal"));
            JakWorldDamage.hit(p,net.minecraft.core.BlockPos.containing(target.subtract(normal.scale(.03))),e.get("damage").getAsFloat());
            return;
        }
        if(kind.equals("explosion")){
            float power=switch(e.get("gun").getAsString()){
                case "Red3" -> 4;case "Dark1" -> 3;case "Dark3" -> JakWorldDamage.NUKE_POWER;case "Yellow3" -> 1.5f;default -> 0;
            };
            float radius=power==JakWorldDamage.NUKE_POWER?e.get("radius").getAsFloat():0;
            JakWorldDamage.explosion(p.level(),p,p,vector(e.getAsJsonArray("pos")).add(origin),power,radius);
            return;
        }
        if(kind.equals("freeze")){
            // The Dark Blast's 'ice: Minecraft's freezing, as from powder snow, on what it struck.
            if(p.level().getEntity(e.get("actor").getAsInt()) instanceof net.minecraft.world.entity.LivingEntity victim&&victim.canFreeze())
                victim.setTicksFrozen(Math.max(victim.getTicksFrozen(),victim.getTicksRequiredToFreeze()+160));
            return;
        }
        if(kind.equals("ground_pound")){
            dev.jakmc.content.JakEcoVent.groundPound(p.level(),p,vector(e.getAsJsonArray("pos")).add(origin));
            return;
        }
        if((kind.equals("strike")||kind.equals("impact"))&&e.has("actor")&&!e.get("actor").isJsonNull()){
            Entity victim=p.level().getEntity(e.get("actor").getAsInt());
            if(victim!=null&&victim!=p){
                if(kick){
                    Vec3 away=victim.position().subtract(p.position()).multiply(1,0,1);
                    if(away.lengthSqr()<1e-8)away=p.getViewVector(1).multiply(1,0,1);
                    away=away.normalize().scale(.55);
                    Vec3 impulse=away.add(0,.15,0);
                    if(victim instanceof ServerPlayer target&&active(target))blownOff(target,target.position(),impulse.scale(20));
                    else if(victim instanceof dev.jakmc.entity.JakVehicle vehicle)vehicle.kickPush(impulse);
                    else if(victim instanceof dev.jakmc.entity.JakHellcat craft)craft.kickPush();
                    else victim.push(impulse);
                    return;
                }
                boolean heavy=e.has("heavy")&&e.get("heavy").getAsBoolean();
                var source=kind.equals("impact")?JakDamage.shot(p,heavy):p.damageSources().playerAttack(p);
                float damage=e.get("damage").getAsFloat()*2.5f;
                // Rust jak.rs Strike: + 5·attack_bonus in MW2 points (5 to a health point), with minecraftoss
                // effects.rs attack_bonus = 3·(Strength level) − 4·(Weakness level); melee only, never below 0.
                if(kind.equals("strike")){
                    var strength=p.getEffect(net.minecraft.world.effect.MobEffects.STRENGTH);
                    var weakness=p.getEffect(net.minecraft.world.effect.MobEffects.WEAKNESS);
                    if(strength!=null)damage+=3*(strength.getAmplifier()+1);
                    if(weakness!=null)damage-=4*(weakness.getAmplifier()+1);
                    damage=Math.max(0,damage);
                }
                // mobs.rs 661: the blow spins a floating (or tumbling) creature along it.
                if(victim instanceof net.minecraft.world.entity.LivingEntity living){
                    Vec3 along=e.has("dir")?vector(e.getAsJsonArray("dir")):victim.position().subtract(p.position());
                    dev.jakmc.entity.MassInverter.hit(living,along,e.get("damage").getAsFloat());
                }
                // survival.rs: a blow that lands on a creature costs 0.1 of exhaustion.
                if(kind.equals("strike"))p.causeFoodExhaustion(.1f);
                boolean melee=kind.equals("strike")&&!strikeState.startsWith("darkjak-bomb");
                if(victim instanceof dev.jakmc.entity.JakEnemy enemy)enemy.hurtFromJak(p.level(),source,damage,heavy,melee);
                else{
                    if(melee&&victim instanceof net.minecraft.world.entity.monster.Monster enemy)damage=Math.min(damage,enemy.getMaxHealth()*.5f);
                    victim.hurtServer(p.level(),source,damage);
                }
            }
        }else if(kind.equals("impact")||kind.equals("strike")){
            Vec3 target=vector(e.getAsJsonArray("pos")).add(origin);
            if(kind.equals("impact")){
                Vec3 normal=vector(e.getAsJsonArray("normal"));
                var at=net.minecraft.core.BlockPos.containing(target.subtract(normal.scale(.03)));
                JakWorldDamage.hit(p,at,e.get("damage").getAsFloat());
            }else{
                // Punches use the block outlined by the player's interaction ray.
                if(e.has("center")){
                    // A kick strikes what its sphere goes through, not what the crosshair is on (struckInSpheres).
                    if(kick)return;
                    WORLD_STRIKES.incrementAndGet();
                    if(p.pick(p.blockInteractionRange(),1,false) instanceof net.minecraft.world.phys.BlockHitResult pick
                        &&pick.getType()==net.minecraft.world.phys.HitResult.Type.BLOCK
                        &&Vec3.atCenterOf(pick.getBlockPos()).distanceTo(p.position().add(0,1,0))<2.5
                        &&(!punch||punchCanReach(p,pick.getBlockPos(),vector(e.getAsJsonArray("dir"))))){
                        WORLD_STRIKES_FOUND.incrementAndGet();
                        strikeBlock(p,pick.getBlockPos(),e.get("damage").getAsFloat(),false);
                    }
                    return;
                }
                // A slam (no sphere), Rust block_struck: the block along the blow,
                // else under the fist or foot (a low block, the ground).
                Vec3 along=vector(e.getAsJsonArray("dir"));
                along=along.lengthSqr()>1e-6?along.normalize():new Vec3(0,-1,0);
                var hit=clip(p,target.subtract(along.scale(.6)),target.add(along.scale(.5)));
                if(hit==null)hit=clip(p,target.subtract(along.scale(.3)),target.add(0,-1.2,0));
                WORLD_STRIKES.incrementAndGet();
                if(hit!=null){WORLD_STRIKES_FOUND.incrementAndGet();strikeBlock(p,hit.getBlockPos(),e.get("damage").getAsFloat(),kick);}
            }
        } else if(kind.equals("fall_damage")) p.hurtServer(p.level(),p.damageSources().fall(),e.get("damage").getAsFloat()*2.5f);
        else if(kind.equals("gravity_field"))dev.jakmc.entity.MassInverter.field(p,vector(e.getAsJsonArray("pos")).add(origin));
        else if(kind.equals("lift")){
            Entity victim=p.level().getEntity(e.get("actor").getAsInt());
            // Rust server.rs lift / float_lifted (was Levitation II).
            if(victim instanceof net.minecraft.world.entity.LivingEntity living)dev.jakmc.entity.MassInverter.lift(living,e.get("seconds").getAsFloat());
        }
    }
}
