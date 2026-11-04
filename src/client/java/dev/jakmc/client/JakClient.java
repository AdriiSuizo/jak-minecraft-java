package dev.jakmc.client;
import net.fabricmc.api.ClientModInitializer;
public final class JakClient implements ClientModInitializer {
 @Override public void onInitializeClient() {
  JakInput.initialize();
  net.fabricmc.fabric.api.client.networking.v1.ClientPlayNetworking.registerGlobalReceiver(
      dev.jakmc.network.JakPackets.EntityTime.TYPE,(packet,context)->{
       var level=context.client().level;
       if(level!=null&&level.dimension().identifier().equals(packet.dimension())){
        dev.jakmc.player.JakEntityClock.of(level).synchronize(packet.rate(),packet.progress(),packet.skipped());
       }
      });
  net.fabricmc.fabric.api.client.event.lifecycle.v1.ClientTickEvents.END_CLIENT_TICK.register(JakPoses::prune);
  JakSounds.initialize();
  JakAtmosphere.initialize();
  JakMinimap.initialize();
  WeatherScene.initialize();
  dev.jakmc.client.model.JakFirstPerson.initialize();
  JakEffects.initialize();
  JakCreatureWater.initialize();
  DevBridge.initialize();
  ScaleScene.initialize();
  PortScene.initialize();
  EnemyScene.initialize();
  SpydroidScene.initialize();
  WeaponScene.initialize();
  InputScene.initialize();
  MenuScene.initialize();
  GemScene.initialize();
  CollisionPerfScene.initialize();
  CitizenScene.initialize();
  MaterialScene.initialize();
  VentScene.initialize();
  AltarScene.initialize();
  TrackScene.initialize();
  LadderScene.initialize();
  SurfaceScene.initialize();
  YakowScene.initialize();
  CarScene.initialize();
  CityScene.initialize();
  SitesScene.initialize();
  EconomyScene.initialize();
  EquipmentScene.initialize();
  VehicleEffects.initialize();
  dev.jakmc.client.model.GltfItemRenderer.initialize();
  dev.jakmc.client.model.PropModels.initialize();
  net.minecraft.client.renderer.blockentity.BlockEntityRenderers.register(
      dev.jakmc.content.JakBoardAltar.TYPE,ctx->new dev.jakmc.client.model.BoardAltarRenderer());
  net.minecraft.client.renderer.blockentity.BlockEntityRenderers.register(
      dev.jakmc.content.JakEcoVent.TYPE,ctx->new dev.jakmc.client.model.EcoVentRenderer());
  dev.jakmc.client.model.SkullGemRenderer.initialize();
  dev.jakmc.client.model.EcoItemRenderer.initialize();
  dev.jakmc.client.model.JakShotRenderer.initialize();
  dev.jakmc.client.model.JakEnemyRenderer.initialize();
  net.fabricmc.fabric.api.client.rendering.v1.EntityRendererRegistry.register(
      dev.jakmc.entity.JakYakow.TYPE,dev.jakmc.client.model.YakowRenderer::new);
  dev.jakmc.client.model.JakLeaperRenderer.initialize();
  dev.jakmc.client.model.HellcatRenderer.initialize();
  dev.jakmc.client.model.JakVehicleRenderer.initialize();
  net.fabricmc.fabric.api.client.rendering.v1.EntityRendererRegistry.register(
      dev.jakmc.entity.JakFlutEgg.TYPE,dev.jakmc.client.model.JakFlutEggRenderer::new);
  net.fabricmc.fabric.api.client.networking.v1.ClientPlayNetworking.registerGlobalReceiver(dev.jakmc.network.JakPackets.Pose.TYPE,(p,c)->JakPoses.receive(p));
 }
}
