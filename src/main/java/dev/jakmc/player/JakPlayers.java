package dev.jakmc.player;

import com.google.gson.*;
import dev.jakmc.*;
import dev.jakmc.content.JakItems;
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
    private static final Map<UUID,JakPackets.Input> PROBE_INPUT=new HashMap<>();
    public static void controlForProbe(ServerPlayer player,boolean controlled){if(controlled){CONTROLLED.add(player.getUUID());probeInput(player,0,0,0);}else{CONTROLLED.remove(player.getUUID());PROBE_INPUT.remove(player.getUUID());}}
    public static void probeInput(ServerPlayer p,int held,int pressed,float forward){PROBE_INPUT.put(p.getUUID(),new JakPackets.Input(0,held,pressed,0,forward,p.getYRot(),p.getXRot(),0,0));}
    private static final Map<UUID,Session> LIVE=new HashMap<>();
    private static final Map<net.minecraft.world.level.block.state.BlockState,Integer> FLOOR_SOUNDS=new IdentityHashMap<>();
    private static final class Session implements AutoCloseable {
        final long nativeId;
        final MinecraftCollision.Query collision;
        Vec3 origin;
        String dimension;
        Vec3 last;
        JakPackets.Input input=new JakPackets.Input(0,0,0,0,0,0,0,0,0);
        int pressed;
        long lastInput;
        Session(ServerPlayer player){nativeId=NativeJak.create(player.getYRot());origin=player.position();last=origin;dimension=player.level().dimension().identifier().toString();collision=new MinecraftCollision.Query(player.level(),player,origin);NativeJak.bindWorld(nativeId,collision);}
        public void close(){NativeJak.destroy(nativeId);}
    }
    public static boolean active(Entity player){return player.getAttachedOrCreate(JakData.ENABLED)&&!player.isPassenger()&&player.isAlive();}
    public static void input(ServerPlayer player,JakPackets.Input input){
        if(CONTROLLED.contains(player.getUUID()))return;
        if(!active(player))return;
        Session s=LIVE.computeIfAbsent(player.getUUID(),id->new Session(player));
        if(input.sequence()<=s.input.sequence())return;
        s.input=input;s.pressed|=input.pressed();s.lastInput=player.level().getGameTime();
    }
    public static void initialize(){
        ServerTickEvents.END_SERVER_TICK.register(server->{
            var present=new HashSet<UUID>();
            for(ServerPlayer p:server.getPlayerList().getPlayers()){
                if(!active(p)||p.isSleeping())continue;
                present.add(p.getUUID());
                Session s=LIVE.get(p.getUUID());
                String dimension=p.level().dimension().identifier().toString();
                if(s!=null&&(!s.dimension.equals(dimension)||p.position().distanceToSqr(s.last)>16)) {s.close();LIVE.remove(p.getUUID());s=null;}
                if(s==null){s=new Session(p);LIVE.put(p.getUUID(),s);}
                try{step(p,s);}catch(RuntimeException failure){s.close();LIVE.remove(p.getUUID());p.setDeltaMovement(Vec3.ZERO);JakMod.LOG.error("Jak session stopped at last safe Minecraft pose for {}",p.getScoreboardName(),failure);}
            }
            var it=LIVE.entrySet().iterator();while(it.hasNext()){var e=it.next();if(!present.contains(e.getKey())){e.getValue().close();it.remove();}}
        });
        ServerPlayConnectionEvents.DISCONNECT.register((handler,server)->{var s=LIVE.remove(handler.getPlayer().getUUID());if(s!=null)s.close();});
        net.fabricmc.fabric.api.event.lifecycle.v1.ServerLifecycleEvents.SERVER_STOPPED.register(server->{for(var s:LIVE.values())s.close();LIVE.clear();CONTROLLED.clear();PROBE_INPUT.clear();});
    }
    private static void step(ServerPlayer p,Session s){
        if(p.position().distanceToSqr(s.origin)>256*256){Vec3 delta=p.position().subtract(s.origin);NativeJak.rebase(s.nativeId,(float)delta.x,(float)delta.y,(float)delta.z);s.origin=p.position();}
        s.collision.origin=s.origin;
        boolean controlled=CONTROLLED.contains(p.getUUID());
        var input=controlled?PROBE_INPUT.get(p.getUUID()):s.input;
        boolean fresh=controlled||p.level().getGameTime()-s.lastInput<=5;
        ByteBuffer b=NativeJak.buffer(64);
        b.putInt(fresh?input.held():0).putInt(fresh?(controlled?input.pressed():s.pressed):0);
        b.putFloat(fresh?input.strafe():0).putFloat(fresh?input.forward():0);
        b.putFloat(0).putFloat(0).putFloat(input.brake()).putFloat(input.throttle());
        b.putFloat(input.yaw()).putFloat(input.pitch());
        float water=waterSurface(p,s.origin);
        float speed=(float)(p.getAttributeValue(net.minecraft.world.entity.ai.attributes.Attributes.MOVEMENT_SPEED)/.1);
        b.putFloat(Float.isFinite(water)?water:0).putFloat(speed);
        int flags=p.getAttachedOrCreate(JakData.BOARD).isEmpty()?0:1;
        if(Float.isFinite(water))flags|=16;
        if(p.hasInfiniteMaterials())flags|=32;
        if(p.hasEffect(net.minecraft.world.effect.MobEffects.FIRE_RESISTANCE))flags|=4;
        if(p.hasEffect(net.minecraft.world.effect.MobEffects.SLOW_FALLING))flags|=8;
        flags|=floor(p)<<8;
        b.putInt(flags).putFloat(p.getAttachedOrCreate(JakData.DARK_ECO)).putFloat(p.getAttachedOrCreate(JakData.LIGHT_ECO));
        b.putInt(JakItems.GUNS.indexOf(p.getMainHandItem().getItem()));
        AABB region=p.getBoundingBox().inflate(6,5,6);
        var triangles=NativeJak.buffer(0); // Queries use the live Minecraft shape callback.
        var actors=MinecraftCollision.actors(p.level(),p,s.origin,region.inflate(256));
        NativeJak.setStats(s.nativeId,p.getHealth()/p.getMaxHealth(),p.getAttachedOrCreate(JakData.AMMO.get(0)),p.getAttachedOrCreate(JakData.AMMO.get(1)),p.getAttachedOrCreate(JakData.AMMO.get(2)),p.getAttachedOrCreate(JakData.AMMO.get(3)));
        JsonObject pose=JsonParser.parseString(NativeJak.step(s.nativeId,b,triangles,actors,3)).getAsJsonObject();
        Vec3 position=vector(pose.getAsJsonArray("pos")).add(s.origin);
        if(!Double.isFinite(position.x)||!Double.isFinite(position.y)||!Double.isFinite(position.z))throw new IllegalStateException("Nonfinite Jak pose");
        p.setPos(position);p.setDeltaMovement(Vec3.ZERO);p.setOnGround((pose.get("status").getAsInt()&2)!=0);p.resetFallDistance();
        p.setYBodyRot(pose.get("yaw").getAsFloat());s.last=position;s.pressed=0;
        if(controlled)probeInput(p,input.held(),0,input.forward());
        setEco(p,JakData.DARK_ECO,pose.get("darkEco").getAsFloat());setEco(p,JakData.LIGHT_ECO,pose.get("lightEco").getAsFloat());
        for(int i=0;i<4;i++)setEco(p,JakData.AMMO.get(i),pose.getAsJsonArray("ammo").get(i).getAsFloat());
        if(pose.get("heal").getAsFloat()>0)p.heal(pose.get("heal").getAsFloat()*p.getMaxHealth());
        for(JsonElement e:pose.getAsJsonArray("events"))applyEvent(p,s.origin,e.getAsJsonObject());
        var packet=new JakPackets.Pose(p.getId(),input.sequence(),s.origin.x,s.origin.y,s.origin.z,pose.toString());
        ServerPlayNetworking.send(p,packet);
        for(ServerPlayer watcher:PlayerLookup.tracking(p))if(watcher!=p)ServerPlayNetworking.send(watcher,packet);
    }
    private static void setEco(ServerPlayer p,net.fabricmc.fabric.api.attachment.v1.AttachmentType<Float> type,float value){if(p.getAttachedOrCreate(type)!=value)p.setAttached(type,value);}
    private static float waterSurface(ServerPlayer p,Vec3 origin){
        var pos=p.blockPosition();
        if(!p.level().getFluidState(pos).is(net.minecraft.tags.FluidTags.WATER))return Float.NaN;
        for(int y=0;y<16;y++){var q=pos.above(y);var fluid=p.level().getFluidState(q);if(!fluid.is(net.minecraft.tags.FluidTags.WATER))return (float)(q.getY()-origin.y);}
        return (float)(pos.getY()+16-origin.y);
    }
    private static int floor(ServerPlayer p){
        if(p.isInWater())return p.isUnderWater()?22:21;
        var state=p.level().getBlockState(net.minecraft.core.BlockPos.containing(p.getX(),p.getY()-.05,p.getZ()));
        return FLOOR_SOUNDS.computeIfAbsent(state,s->NativeJak.floor(net.minecraft.core.registries.BuiltInRegistries.BLOCK.getKey(s.getBlock()).getPath(),s.getSoundType().getStepSound().location().toString()));
    }
    public static Vec3 vector(JsonArray a){return new Vec3(a.get(0).getAsDouble(),a.get(1).getAsDouble(),a.get(2).getAsDouble());}
    private static void applyEvent(ServerPlayer p,Vec3 origin,JsonObject e){
        String kind=e.get("kind").getAsString();
        if((kind.equals("strike")||kind.equals("impact"))&&e.has("actor")&&!e.get("actor").isJsonNull()){
            Entity victim=p.level().getEntity(e.get("actor").getAsInt());
            if(victim!=null&&victim!=p){
                boolean heavy=e.has("heavy")&&e.get("heavy").getAsBoolean();
                var source=kind.equals("impact")?JakDamage.shot(p,heavy):p.damageSources().playerAttack(p);
                float damage=e.get("damage").getAsFloat()*2.5f;
                if(victim instanceof dev.jakmc.entity.JakEnemy enemy)enemy.hurtFromJak(p.level(),source,damage,heavy);else victim.hurtServer(p.level(),source,damage);
            }
        } else if(kind.equals("fall_damage")) p.hurtServer(p.level(),p.damageSources().fall(),e.get("damage").getAsFloat()*2.5f);
        else if(kind.equals("lift")){
            Entity victim=p.level().getEntity(e.get("actor").getAsInt());
            if(victim instanceof net.minecraft.world.entity.LivingEntity living)living.addEffect(new net.minecraft.world.effect.MobEffectInstance(net.minecraft.world.effect.MobEffects.LEVITATION,(int)(20*e.get("seconds").getAsFloat()),1));
        }
    }
}
