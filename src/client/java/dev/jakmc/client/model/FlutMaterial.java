package dev.jakmc.client.model;

import com.mojang.blaze3d.platform.NativeImage;
import dev.jakmc.JakData;
import dev.jakmc.RuntimeConfig;
import java.io.IOException;
import java.io.UncheckedIOException;
import java.nio.file.Path;
import java.util.Set;
import net.minecraft.client.Minecraft;
import net.minecraft.client.renderer.texture.DynamicTexture;
import net.minecraft.resources.Identifier;

record FlutMaterial(int color,boolean blink) {
    private static final Set<String> FEATHERS=Set.of("flut-neck","flut-dkbluefeathers","flut-yellow2dkblue",
        "flut-creamfeathers","flut-plume","flut-wingends","flut-wing","flut-tail","flut-brow");
    private static final int[] TINTS={0xffffcc,0xffffb3,0xffff99,0xffff80,0xffff66,0xffe680,0xffcc99,
        0xffcc80,0xffcc66,0xe6e6ff,0xcce6ff,0xccffff,0xccffcc};
    private static final Identifier[] EYES=new Identifier[2];

    static Path jak1(){
        return RuntimeConfig.assets.getParent().getParent().getParent().getParent()
            .resolve("jak1/data/decompiler_out/jak1");
    }
    int tint(String material){return FEATHERS.contains(material)?TINTS[color]:0xffffff;}
    Identifier eyes(){
        int eye=blink?1:0;
        if(EYES[eye]!=null)return EYES[eye];
        String name=blink?"closed":"open";
        var id=JakData.id("local/flut_eyes_"+name);
        try(var in=FlutMaterial.class.getResourceAsStream("/assets/jakmc/flut/eye_"+name+".png")){
            if(in==null)throw new IOException("Import the original Flut Flut eye layers");
            Minecraft.getInstance().getTextureManager().register(id,new DynamicTexture(id::toString,NativeImage.read(in)));
        }catch(IOException failure){throw new UncheckedIOException(failure);}
        return EYES[eye]=id;
    }
    static final class Blink {
        private long key,start,end;
        Blink(int seed){
            key=Integer.toUnsignedLong(seed)*0x9e37_79b9_7f4a_7c15L;
            advance();
        }
        private void advance(){
            key^=key>>>29;key*=0xbf58_476d_1ce4_e5b9L;
            start=end+60+(key>>>33)%181;
            end=start+10;
        }
        boolean closed(float seconds){
            long frame=(long)(seconds*60);
            while(frame>=end)advance();
            return frame>=start+1&&frame<=start+6;
        }
    }
}
