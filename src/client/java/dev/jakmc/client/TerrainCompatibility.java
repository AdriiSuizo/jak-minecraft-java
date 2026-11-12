package dev.jakmc.client;

import net.fabricmc.loader.api.FabricLoader;

/** Sodium owns its terrain pipeline and does not run Minecraft's terrain shaders. */
public final class TerrainCompatibility {
    private static final boolean VANILLA_TERRAIN = !FabricLoader.getInstance().isModLoaded("sodium");

    private TerrainCompatibility() {}

    public static boolean vanillaTerrain() { return VANILLA_TERRAIN; }
}
