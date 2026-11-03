package dev.jakmc;

import dev.jakmc.nativebridge.NativeJak;
import net.fabricmc.loader.api.FabricLoader;
import java.io.*;
import java.nio.file.*;
import java.security.MessageDigest;
import java.util.*;

public final class RuntimeConfig {
    public static Path assets;
    public static void initialize() {
        try {
            var properties=new Properties();
            Path config=FabricLoader.getInstance().getConfigDir().resolve("jakmc.properties");
            if(Files.isRegularFile(config))try(var in=Files.newInputStream(config)){properties.load(in);}
            assets=Path.of(System.getProperty("jakmc.assets",properties.getProperty("assets",Path.of(System.getProperty("user.home"),"Documents","Jak OpenGOAL","active","jak3","data","decompiler_out","jak3").toString())));
            String override=System.getProperty("jakmc.native",properties.getProperty("native",""));
            Path dll;
            if(!override.isBlank())dll=Path.of(override);
            else {
                String resource="/natives/windows-x86_64/jak_jni.dll";
                byte[] bytes;
                try(var in=RuntimeConfig.class.getResourceAsStream(resource)) {
                    if(in==null)throw new IOException("Native core missing from mod: "+resource);
                    bytes=in.readAllBytes();
                }
                String hash=HexFormat.of().formatHex(MessageDigest.getInstance("SHA-256").digest(bytes));
                dll=FabricLoader.getInstance().getGameDir().resolve("jakmc/cache/"+hash+"/jak_jni.dll");
                if(!Files.isRegularFile(dll)){Files.createDirectories(dll.getParent());Files.write(dll,bytes);}
            }
            NativeJak.load(dll);
            int animations=NativeJak.configure(assets.toString());
            JakMod.LOG.info("Jak source timings loaded: {} animations, native ABI {}",animations,NativeJak.ABI);
        } catch(Exception e){throw new IllegalStateException("Cannot initialize Jak from local owned exports",e);}
    }
}
