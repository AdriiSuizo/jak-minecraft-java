package dev.jakmc.network;

import dev.jakmc.JakData;
import dev.jakmc.player.JakPlayers;
import net.minecraft.network.RegistryFriendlyByteBuf;
import net.minecraft.network.codec.StreamCodec;
import net.minecraft.network.protocol.common.custom.CustomPacketPayload;
import net.fabricmc.fabric.api.networking.v1.*;
import java.io.*;
import java.nio.charset.StandardCharsets;
import java.util.zip.*;

public final class JakPackets {
    public static final int FIRST_PERSON=1<<16;
    public static final int CANCEL_POWER=1<<17;
    public record Input(int sequence,int held,int pressed,float strafe,float forward,float rightX,float rightY,float yaw,float pitch,float brake,float throttle) implements CustomPacketPayload {
        public static final Type<Input> TYPE=new Type<>(JakData.id("input_v3"));
        public static final StreamCodec<RegistryFriendlyByteBuf,Input> CODEC=StreamCodec.of((b,v)->{
            b.writeVarInt(v.sequence);b.writeInt(v.held);b.writeInt(v.pressed);b.writeFloat(v.strafe);b.writeFloat(v.forward);b.writeFloat(v.rightX);b.writeFloat(v.rightY);b.writeFloat(v.yaw);b.writeFloat(v.pitch);b.writeFloat(v.brake);b.writeFloat(v.throttle);
        },b->new Input(b.readVarInt(),b.readInt(),b.readInt(),b.readFloat(),b.readFloat(),b.readFloat(),b.readFloat(),b.readFloat(),b.readFloat(),b.readFloat(),b.readFloat()));
        public Type<Input> type(){return TYPE;}
        public boolean valid(){return sequence>=0&&(held&~(65535|FIRST_PERSON|CANCEL_POWER))==0&&(pressed&~65535)==0&&Float.isFinite(strafe)&&Float.isFinite(forward)&&Math.abs(strafe)<=1&&Math.abs(forward)<=1&&Float.isFinite(rightX)&&Float.isFinite(rightY)&&Math.abs(rightX)<=1&&Math.abs(rightY)<=1&&Float.isFinite(yaw)&&Float.isFinite(pitch)&&Math.abs(pitch)<=90&&Float.isFinite(brake)&&brake>=0&&brake<=1&&Float.isFinite(throttle)&&throttle>=0&&throttle<=1;}
    }
    public record Pose(int entityId,int sequence,double originX,double originY,double originZ,String json) implements CustomPacketPayload {
        public static final Type<Pose> TYPE=new Type<>(JakData.id("pose_v3"));
        public static final StreamCodec<RegistryFriendlyByteBuf,Pose> CODEC=StreamCodec.of((b,v)->{b.writeVarInt(v.entityId);b.writeVarInt(v.sequence);b.writeDouble(v.originX);b.writeDouble(v.originY);b.writeDouble(v.originZ);b.writeByteArray(compress(v.json));},b->new Pose(b.readVarInt(),b.readVarInt(),b.readDouble(),b.readDouble(),b.readDouble(),inflate(b.readByteArray(262144))));
        public Type<Pose> type(){return TYPE;}
    }
    public record EntityTime(net.minecraft.resources.Identifier dimension,float rate,double progress,long skipped) implements CustomPacketPayload {
        public static final Type<EntityTime> TYPE=new Type<>(JakData.id("entity_time_v1"));
        public static final StreamCodec<RegistryFriendlyByteBuf,EntityTime> CODEC=StreamCodec.of((b,v)->{
            b.writeIdentifier(v.dimension);b.writeFloat(v.rate);b.writeDouble(v.progress);b.writeVarLong(v.skipped);
        },b->new EntityTime(b.readIdentifier(),b.readFloat(),b.readDouble(),b.readVarLong()));
        public Type<EntityTime> type(){return TYPE;}
    }

    public record PropBreak(net.minecraft.resources.Identifier dimension,net.minecraft.core.BlockPos position) implements CustomPacketPayload {
        public static final Type<PropBreak> TYPE=new Type<>(JakData.id("prop_break_v1"));
        public static final StreamCodec<RegistryFriendlyByteBuf,PropBreak> CODEC=StreamCodec.of((b,p)->{
            b.writeIdentifier(p.dimension);b.writeBlockPos(p.position);
        },b->new PropBreak(b.readIdentifier(),b.readBlockPos()));
        @Override public Type<PropBreak> type(){return TYPE;}
    }

    public record CreatureBurst(net.minecraft.resources.Identifier dimension,net.minecraft.world.phys.Vec3 at) implements CustomPacketPayload {
        public static final Type<CreatureBurst> TYPE=new Type<>(JakData.id("spydroid_explosion_v1"));
        public static final StreamCodec<RegistryFriendlyByteBuf,CreatureBurst> CODEC=StreamCodec.of((b,p)->{
            b.writeIdentifier(p.dimension);b.writeDouble(p.at.x);b.writeDouble(p.at.y);b.writeDouble(p.at.z);
        },b->new CreatureBurst(b.readIdentifier(),new net.minecraft.world.phys.Vec3(b.readDouble(),b.readDouble(),b.readDouble())));
        @Override public Type<CreatureBurst> type(){return TYPE;}
    }

    private static byte[] compress(String json){
        byte[] data=json.getBytes(StandardCharsets.UTF_8);if(data.length>2_000_000)throw new IllegalArgumentException("Jak pose exceeds bounded packet size");
        var deflater=new Deflater(Deflater.BEST_SPEED);
        try{var out=new ByteArrayOutputStream();try(var zip=new DeflaterOutputStream(out,deflater)){zip.write(data);}byte[] result=out.toByteArray();if(result.length>262144)throw new IllegalArgumentException("Jak compressed pose exceeds packet size");return result;}catch(IOException e){throw new UncheckedIOException(e);}finally{deflater.end();}
    }
    private static String inflate(byte[] compressed){
        try(var in=new InflaterInputStream(new ByteArrayInputStream(compressed))){byte[] data=in.readNBytes(2_000_001);if(data.length>2_000_000)throw new IllegalArgumentException("Jak packet expands beyond limit");return new String(data,StandardCharsets.UTF_8);}catch(IOException e){throw new UncheckedIOException(e);}
    }
    public static void initialize(){
        PayloadTypeRegistry.serverboundPlay().register(Input.TYPE,Input.CODEC);
        PayloadTypeRegistry.clientboundPlay().register(Pose.TYPE,Pose.CODEC);
        PayloadTypeRegistry.clientboundPlay().register(EntityTime.TYPE,EntityTime.CODEC);
        PayloadTypeRegistry.clientboundPlay().register(PropBreak.TYPE,PropBreak.CODEC);
        PayloadTypeRegistry.clientboundPlay().register(CreatureBurst.TYPE,CreatureBurst.CODEC);
        ServerPlayNetworking.registerGlobalReceiver(Input.TYPE,(packet,context)->{if(packet.valid())JakPlayers.input(context.player(),packet);});
    }
}
