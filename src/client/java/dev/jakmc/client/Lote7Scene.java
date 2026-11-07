package dev.jakmc.client;

import com.google.gson.JsonObject;
import dev.jakmc.JakData;
import dev.jakmc.content.darkeco.DarkEco;
import java.io.IOException;
import java.io.UncheckedIOException;
import java.nio.file.Files;
import java.util.UUID;
import net.fabricmc.fabric.api.client.event.lifecycle.v1.ClientTickEvents;
import net.minecraft.client.CameraType;
import net.minecraft.client.Minecraft;
import net.minecraft.client.Screenshot;
import net.minecraft.core.BlockPos;
import net.minecraft.server.level.ServerPlayer;
import net.minecraft.world.entity.EntitySpawnReason;
import net.minecraft.world.entity.EntityTypes;
import net.minecraft.world.entity.LivingEntity;
import net.minecraft.world.entity.item.ItemEntity;
import net.minecraft.world.item.ItemStack;
import net.minecraft.world.item.Items;
import net.minecraft.world.level.GameType;
import net.minecraft.world.level.block.Blocks;
import net.minecraft.world.level.levelgen.Heightmap;
import net.minecraft.world.phys.Vec3;

/**
 * Adrii's 07/11 dark eco: a pool as the final silo's and a stream poured off a ledge, looked at from above and from low
 * down; then an item, a pig, a Jak creature and Jak himself going in it.
 */
final class Lote7Scene {
    private static boolean running,pause;
    private static int tick;
    private static UUID owner;
    private static BlockPos base;
    private static volatile InputScene.CameraView view;
    private static final JsonObject result=new JsonObject();
    private static LivingEntity pig,babak;
    private static ItemEntity item;
    static InputScene.CameraView camera(){return view;}
    static void initialize(){ClientTickEvents.END_CLIENT_TICK.register(Lote7Scene::tick);}
    private static Vec3 at(double x,double y,double z){return Vec3.atBottomCenterOf(base).add(x,y,z);}
    private static void look(Vec3 eye,Vec3 target){
        var d=target.subtract(eye);
        view=new InputScene.CameraView(eye,(float)Math.toDegrees(Math.atan2(-d.x,d.z)),(float)-Math.toDegrees(Math.atan2(d.y,d.horizontalDistance())));
    }
    private static void capture(Minecraft c,String file){Screenshot.grab(c.gameDirectory,file,c.gameRenderer.mainRenderTarget(),1,_->{});}

    static void start(Minecraft client){
        if(client.player==null||client.getSingleplayerServer()==null)throw new IllegalStateException("Open the development world");
        owner=client.player.getUUID();tick=0;result.entrySet().clear();pig=babak=null;item=null;
        pause=client.options.pauseOnLostFocus;client.options.pauseOnLostFocus=false;
        client.options.setCameraType(CameraType.THIRD_PERSON_BACK);
        onServer(client,p->{
            var server=p.level().getServer();
            for(var text:new String[]{"time set noon","weather clear","gamerule spawn_monsters false"})
                server.getCommands().performPrefixedCommand(server.createCommandSourceStack().withSuppressedOutput(),text);
            var level=p.level();
            var near=p.blockPosition();
            base=near.atY(level.getHeight(Heightmap.Types.MOTION_BLOCKING_NO_LEAVES,near.getX(),near.getZ())+24);
            // A basin twenty across, its liquid a block deep, and a ledge at its side that a stream falls off.
            for(int x=-14;x<=14;x++)for(int z=-14;z<=14;z++)for(int y=-3;y<=12;y++){
                var cell=base.offset(x,y,z);
                boolean rim=Math.abs(x)>10||Math.abs(z)>10;
                var state=Blocks.AIR.defaultBlockState();
                if(y==-3||y==-2||y==-1&&rim||y==0&&rim)state=Blocks.STONE.defaultBlockState();
                else if(y==-1||y==0)state=Blocks.AIR.defaultBlockState();
                if(x>=6&&x<=10&&z>=-10&&z<=-6&&y>=-1&&y<=3)state=Blocks.STONE.defaultBlockState();
                level.setBlock(cell,state,2);
            }
            for(int x=-10;x<=10;x++)for(int z=-10;z<=10;z++){
                var cell=base.offset(x,-1,z);
                if(level.getBlockState(cell).isAir())level.setBlock(cell,DarkEco.BLOCK.defaultBlockState(),3);
            }
            level.setBlock(base.offset(8,4,-8),DarkEco.BLOCK.defaultBlockState(),3);
            p.setGameMode(GameType.CREATIVE);p.getAbilities().flying=true;p.onUpdateAbilities();
            var hover=at(-12,8,12);p.teleportTo(hover.x,hover.y,hover.z);
        });
        running=true;
    }

    private static void tick(Minecraft c){
        if(!running||c.player==null||c.level==null)return;
        int t=tick++;
        if(t==200)look(at(-8,9,8),at(0,-.5,0));
        if(t==240)capture(c,"lote7-pool-high.png");
        if(t==280)capture(c,"lote7-pool-high-later.png");
        if(t==285)look(at(-9,1.2,9),at(4,-.4,-4));
        if(t==310)capture(c,"lote7-pool-low.png");
        if(t==315)look(at(0,3,-2),at(8,1.5,-8));
        if(t==340)capture(c,"lote7-stream.png");
        if(t==345)look(at(-6,4,6),at(-1,0,1));
        if(t==360)onServer(c,p->{
            var level=p.level();var drop=at(-1,1,1);
            item=new ItemEntity(level,drop.x,drop.y,drop.z,new ItemStack(Items.DIAMOND,3));
            item.setDeltaMovement(Vec3.ZERO);level.addFreshEntity(item);
        });
        if(t>=366&&t<=384&&t%6==0)capture(c,"lote7-item-"+t+".png");
        if(t==400)onServer(c,p->{
            result.addProperty("item_gone",item.isRemoved());
            var level=p.level();
            pig=EntityTypes.PIG.create(level,EntitySpawnReason.COMMAND);
            var drop=at(-1,1,1);pig.snapTo(drop.x,drop.y,drop.z,0,0);level.addFreshEntity(pig);
            var type=net.minecraft.core.registries.BuiltInRegistries.ENTITY_TYPE.getValue(JakData.id("babak"));
            babak=(LivingEntity)type.create(level,EntitySpawnReason.COMMAND);
            var other=at(1,1,-1);babak.snapTo(other.x,other.y,other.z,0,0);level.addFreshEntity(babak);
        });
        if(t>=406&&t<=430&&t%6==0)capture(c,"lote7-mob-"+t+".png");
        if(t==460)onServer(c,p->{
            result.addProperty("pig_dead",!pig.isAlive());result.addProperty("babak_dead",!babak.isAlive());
            p.getAbilities().flying=false;p.onUpdateAbilities();p.setGameMode(GameType.SURVIVAL);
            var over=at(-3,1,3);p.teleportTo(over.x,over.y,over.z);p.setDeltaMovement(Vec3.ZERO);
        });
        if(t>=466&&t<=496&&t%6==0)capture(c,"lote7-jak-"+t+".png");
        if(t==520){
            view=null;
            onServer(c,p->{
                result.addProperty("jak_dead",!p.isAlive());
                result.addProperty("dark_eco_deaths",JakEffects.darkEcoDeaths);
                result.addProperty("visual_review","required");result.addProperty("reason","complete");
                try{
                    var dir=c.gameDirectory.toPath().resolve("jakmc/evidence/lote7");
                    Files.createDirectories(dir);Files.writeString(dir.resolve("result.json"),result.toString());
                }catch(IOException failure){throw new UncheckedIOException(failure);}
                c.execute(()->{running=false;c.options.pauseOnLostFocus=pause;c.stop();});
            });
        }
    }
    private static void onServer(Minecraft client,java.util.function.Consumer<ServerPlayer> action){
        var server=client.getSingleplayerServer();
        server.execute(()->{var player=server.getPlayerList().getPlayer(owner);if(player!=null)action.accept(player);});
    }
}
