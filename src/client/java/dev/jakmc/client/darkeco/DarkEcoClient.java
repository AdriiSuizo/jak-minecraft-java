package dev.jakmc.client.darkeco;

import dev.jakmc.JakData;
import dev.jakmc.content.darkeco.DarkEco;
import java.nio.ByteBuffer;
import net.fabricmc.fabric.api.client.render.fluid.v1.FluidRenderingRegistry;
import net.minecraft.client.Minecraft;
import net.minecraft.client.color.block.BlockTintSources;
import net.minecraft.client.renderer.block.FluidModel;
import net.minecraft.client.renderer.texture.TextureAtlasSprite;
import net.minecraft.client.resources.model.sprite.Material;
import net.minecraft.data.AtlasIds;
import net.minecraft.resources.Identifier;

/** Dark eco's look: the final silo's pool (jakmc:dark_eco.glsl), its faces marked by a tint nothing else has. */
public final class DarkEcoClient {
    private DarkEcoClient(){}
    private static final Identifier WATER=JakData.id("block/dark_eco_still"),SHINE=JakData.id("block/dark_eco_flow");
    public static void initialize(){
        FluidRenderingRegistry.register(DarkEco.SOURCE,DarkEco.FLOWING,new FluidModel.Unbaked(
            new Material(WATER),new Material(SHINE),null,BlockTintSources.constant(0xff00ff)));
    }
    private static void sprite(ByteBuffer buffer,int at,TextureAtlasSprite sprite){
        buffer.putFloat(at,sprite.getX()).putFloat(at+4,sprite.getY());
        buffer.putFloat(at+8,sprite.contents().width()).putFloat(at+12,sprite.contents().height());
    }
    /** JakEcoWater, JakEcoShine, JakEcoClock (minecraft:globals.glsl). */
    public static void write(ByteBuffer buffer,int at){
        if(Minecraft.getInstance().level==null)return;
        var atlas=Minecraft.getInstance().getAtlasManager().getAtlasOrThrow(AtlasIds.BLOCKS);
        sprite(buffer,at,atlas.getSprite(WATER));
        sprite(buffer,at+16,atlas.getSprite(SHINE));
        double seconds=System.nanoTime()/1e9%4096;
        // dark-eco-pool water-vol-idle: individual-normal-scale 0.5 + cos^2(5 t)/2 of the pool's normal-scale 4, t at 300 a second.
        // stand-in: twice that here, the swirls of the map coming across a pool a few blocks wide and not a silo's 80 m.
        double swell=Math.cos(5*seconds*300/65536*2*Math.PI);swell*=swell;
        buffer.putFloat(at+32,(float)seconds).putFloat(at+36,(float)(8*(.5+swell/2)));
    }
}
