package dev.jakmc;
import net.fabricmc.api.ModInitializer;
import org.slf4j.Logger;
import org.slf4j.LoggerFactory;
public final class JakMod implements ModInitializer {
 public static final String ID="jakmc";
 public static final Logger LOG=LoggerFactory.getLogger(ID);
 @Override public void onInitialize() {
  RuntimeConfig.initialize();
  JakData.initialize();
  dev.jakmc.content.JakItems.initialize();
  dev.jakmc.content.JakBlocks.initialize();
  dev.jakmc.content.JakBoardAltar.initialize();
  dev.jakmc.content.JakEcoVent.initialize();
  dev.jakmc.content.JakEcoLauncher.initialize();dev.jakmc.content.JakAmmoCrate.initialize();dev.jakmc.content.JakHutLamp.initialize();
  dev.jakmc.content.JakIrisDoor.initialize();
  dev.jakmc.content.darkeco.DarkEco.initialize();
  dev.jakmc.content.JakProps.initialize();
  dev.jakmc.content.JakAssemblies.initialize();
  dev.jakmc.content.JakFacadeParts.initialize();
  dev.jakmc.content.JakLooseObjects.initialize();
  dev.jakmc.content.JakGrindRail.initialize();
  dev.jakmc.entity.JakHellcat.initialize();
  dev.jakmc.entity.JakVehicle.initialize();
  dev.jakmc.entity.JakLeaper.initialize();
  dev.jakmc.entity.JakFlutFlut.initialize();
  dev.jakmc.entity.JakFlutEgg.initialize();
  dev.jakmc.entity.JakEntities.initialize();
  dev.jakmc.ecology.JakEcology.initialize();
  dev.jakmc.entity.MassInverter.initialize();
  dev.jakmc.entity.JakCitizens.initialize();
  dev.jakmc.citizens.CitizenTown.initialize();
  dev.jakmc.network.JakPackets.initialize();
  dev.jakmc.player.JakPlayers.initialize();
  dev.jakmc.player.JakRescue.initialize();
  dev.jakmc.commands.JakCommands.initialize();
  dev.jakmc.worldgen.JakCityStructure.initialize();
  LOG.info("Jak Worlds: Minecraft host initialized");
 }
}
