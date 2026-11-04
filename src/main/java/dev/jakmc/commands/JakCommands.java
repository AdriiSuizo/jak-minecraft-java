package dev.jakmc.commands;
import dev.jakmc.*;
import dev.jakmc.content.JakItems;
import net.fabricmc.fabric.api.command.v2.CommandRegistrationCallback;
import net.minecraft.commands.Commands;
import net.minecraft.network.chat.Component;
import net.minecraft.world.item.ItemStack;
public final class JakCommands {
 public static void initialize(){
  CommandRegistrationCallback.EVENT.register((dispatcher,context,environment)->dispatcher.register(Commands.literal("jak")
   .then(Commands.literal("on").executes(c->{var p=c.getSource().getPlayerOrException();p.setAttached(JakData.ENABLED,true);c.getSource().sendSuccess(()->Component.literal("Jak mode enabled"),false);return 1;}))
   .then(Commands.literal("off").executes(c->{c.getSource().getPlayerOrException().setAttached(JakData.ENABLED,false);return 1;}))
   .then(Commands.literal("equipment").executes(c->{var p=c.getSource().getPlayerOrException();
    if(!p.hasInfiniteMaterials()){c.getSource().sendFailure(Component.literal("Creative mode required"));return 0;}
    for(var item:JakItems.ITEMS.values())p.getInventory().add(new ItemStack(item));return JakItems.ITEMS.size();}))
   .then(Commands.literal("locate").then(Commands.argument("city",com.mojang.brigadier.arguments.StringArgumentType.word())
    .suggests((c,b)->{b.suggest("haven");b.suggest("spargus");return b.buildFuture();})
    .executes(c->{var p=c.getSource().getPlayerOrException();String city=com.mojang.brigadier.arguments.StringArgumentType.getString(c,"city");
     if(!city.equals("haven")&&!city.equals("spargus")){c.getSource().sendFailure(Component.literal("Use haven or spargus"));return 0;}
     var level=p.level();var key=net.minecraft.resources.ResourceKey.create(net.minecraft.core.registries.Registries.STRUCTURE,JakData.id(city));
     var type=level.registryAccess().lookupOrThrow(net.minecraft.core.registries.Registries.STRUCTURE).getOrThrow(key);
     var point=level.findNearestMapStructure(net.minecraft.core.HolderSet.direct(type),p.blockPosition(),128,false);
     if(point==null){c.getSource().sendFailure(Component.literal("No "+city+" found nearby in this dimension"));return 0;}
     c.getSource().sendSuccess(()->Component.literal(city+": X "+point.getX()+" Z "+point.getZ()+" (normal world generation)"),false);return 1;})))
   .then(Commands.literal("where").executes(c->{var p=c.getSource().getPlayerOrException();c.getSource().sendSuccess(()->Component.literal(String.format("X %.2f Y %.2f Z %.2f",p.getX(),p.getY(),p.getZ())),false);return 1;}))
  ));
 }
}
