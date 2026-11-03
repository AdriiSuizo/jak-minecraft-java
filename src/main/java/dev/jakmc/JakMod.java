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
  dev.jakmc.entity.JakLeaper.initialize();
  dev.jakmc.entity.JakEntities.initialize();
  dev.jakmc.entity.JakCitizens.initialize();
  dev.jakmc.network.JakPackets.initialize();
  dev.jakmc.player.JakPlayers.initialize();
  dev.jakmc.commands.JakCommands.initialize();
  LOG.info("Jak Worlds: Minecraft host initialized");
 }
}
