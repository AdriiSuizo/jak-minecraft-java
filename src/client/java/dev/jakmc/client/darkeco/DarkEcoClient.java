package dev.jakmc.client.darkeco;

import dev.jakmc.JakData;
import dev.jakmc.content.darkeco.DarkEco;
import net.fabricmc.fabric.api.client.render.fluid.v1.FluidRenderingRegistry;
import net.minecraft.client.renderer.block.FluidModel;
import net.minecraft.client.resources.model.sprite.Material;

/** Dark eco's look: render_gpu drawsurf/minecraft_world.rs darkeco_fragment, baked by tools/import_dark_eco.py into its sprites. */
public final class DarkEcoClient {
    private DarkEcoClient(){}
    public static void initialize(){
        // stand-in: the pool's three-wave ripple (darkeco_wave), the view-reflected environment-darkeco shine and the slow
        // texture drift are shader work Minecraft's fluid quads can't do; the colour is baked into still/flow sprites, untinted.
        FluidRenderingRegistry.register(DarkEco.SOURCE,DarkEco.FLOWING,new FluidModel.Unbaked(
            new Material(JakData.id("block/dark_eco_still")),new Material(JakData.id("block/dark_eco_flow")),null,null));
    }
}
