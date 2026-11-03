package dev.jakmc;

import com.mojang.serialization.Codec;
import net.fabricmc.fabric.api.attachment.v1.*;
import net.minecraft.network.codec.ByteBufCodecs;
import net.minecraft.resources.Identifier;
import net.minecraft.world.item.ItemStack;

public final class JakData {
    public static final AttachmentType<Boolean> ENABLED=AttachmentRegistry.create(id("enabled"), b->b.initializer(()->false).persistent(Codec.BOOL).copyOnDeath().syncWith(ByteBufCodecs.BOOL,AttachmentSyncPredicate.all()));
    public static final AttachmentType<ItemStack> BOARD=AttachmentRegistry.create(id("jet_board"), b->b.initializer(()->ItemStack.EMPTY).persistent(ItemStack.OPTIONAL_CODEC).copyOnDeath().syncWith(ItemStack.OPTIONAL_STREAM_CODEC,AttachmentSyncPredicate.all()));
    public static final AttachmentType<Float> DARK_ECO=eco("dark_eco");
    public static final AttachmentType<Float> LIGHT_ECO=eco("light_eco");
    public static final java.util.List<AttachmentType<Float>> AMMO=java.util.List.of(ammo("red",50),ammo("yellow",100),ammo("blue",100),ammo("dark",20));
    private static AttachmentType<Float> ammo(String color,float initial){return AttachmentRegistry.create(id("ammo_"+color),b->b.initializer(()->initial).persistent(Codec.FLOAT).copyOnDeath().syncWith(ByteBufCodecs.FLOAT,AttachmentSyncPredicate.targetOnly()));}
    private static AttachmentType<Float> eco(String name){return AttachmentRegistry.create(id(name),b->b.initializer(()->0f).persistent(Codec.FLOAT).copyOnDeath().syncWith(ByteBufCodecs.FLOAT,AttachmentSyncPredicate.targetOnly()));}
    public static Identifier id(String path){return Identifier.fromNamespaceAndPath(JakMod.ID,path);}
    public static void initialize() { }
}
