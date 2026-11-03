package dev.jakmc.client;
import net.fabricmc.api.ClientModInitializer;
public final class JakClient implements ClientModInitializer {
 @Override public void onInitializeClient() {
  JakInput.initialize();
  net.fabricmc.fabric.api.client.event.lifecycle.v1.ClientTickEvents.END_CLIENT_TICK.register(JakPoses::prune);
  JakSounds.initialize();
  JakEffects.initialize();
  JakCreatureWater.initialize();
  DevBridge.initialize();
  ScaleScene.initialize();
  PortScene.initialize();
  EnemyScene.initialize();
  WeaponScene.initialize();
  dev.jakmc.client.model.GltfItemRenderer.initialize();
  dev.jakmc.client.model.EcoItemRenderer.initialize();
  dev.jakmc.client.model.JakShotRenderer.initialize();
  dev.jakmc.client.model.JakEnemyRenderer.initialize();
  net.fabricmc.fabric.api.client.networking.v1.ClientPlayNetworking.registerGlobalReceiver(dev.jakmc.network.JakPackets.Pose.TYPE,(p,c)->JakPoses.receive(p));
 }
}
