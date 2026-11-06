package dev.jakmc.client;

import com.google.gson.JsonArray;
import com.google.gson.JsonObject;
import dev.jakmc.JakData;
import dev.jakmc.content.JakEcoVent;
import dev.jakmc.content.JakIrisDoor;
import dev.jakmc.content.JakItems;
import dev.jakmc.player.JakPlayers;
import dev.jakmc.player.JakResources;
import java.io.IOException;
import java.io.UncheckedIOException;
import java.nio.file.Files;
import java.util.ArrayList;
import java.util.LinkedHashMap;
import java.util.LinkedHashSet;
import java.util.List;
import java.util.UUID;
import net.fabricmc.fabric.api.client.event.lifecycle.v1.ClientTickEvents;
import net.minecraft.client.CameraType;
import net.minecraft.client.Minecraft;
import net.minecraft.client.Screenshot;
import net.minecraft.core.BlockPos;
import net.minecraft.core.Direction;
import net.minecraft.core.registries.BuiltInRegistries;
import net.minecraft.resources.Identifier;
import net.minecraft.server.level.ServerPlayer;
import net.minecraft.world.InteractionHand;
import net.minecraft.world.entity.EntitySpawnReason;
import net.minecraft.world.entity.LivingEntity;
import net.minecraft.world.item.Item;
import net.minecraft.world.item.ItemStack;
import net.minecraft.world.item.context.UseOnContext;
import net.minecraft.world.level.GameType;
import net.minecraft.world.level.block.Blocks;
import net.minecraft.world.level.block.state.BlockState;
import net.minecraft.world.level.levelgen.Heightmap;
import net.minecraft.world.phys.BlockHitResult;
import net.minecraft.world.phys.Vec3;

/**
 * Adrii's 06/11 batch in one bounded run: the new decoration placed by hand (facing, pot, palm, barrels,
 * burning drum, icons on the hotbar), the ghost of a 3x3 door, Haven's airlock opening and closing from both
 * sides, the iris door and the blue eco launcher with blue eco (Jak's launch), Dark Jak's bomb and blast on
 * husks, Light Jak, the JET-Board on his back and over water.
 */
final class LoteScene {
    private static boolean running,pause;
    private static int tick;
    private static UUID owner;
    private static BlockPos base;
    private static InputScene.CameraView view;
    private static Runnable restore;
    private static final JsonObject result=new JsonObject();
    private static final LinkedHashSet<String> launchStates=new LinkedHashSet<>(),darkStates=new LinkedHashSet<>(),lightStates=new LinkedHashSet<>();
    private static final List<UUID> husks=new ArrayList<>();
    private static int bombShot,blastShot,domeShot;
    private static double boardLow=Double.MAX_VALUE,launchPeak;
    static InputScene.CameraView camera(){return view;}
    static void initialize(){ClientTickEvents.END_CLIENT_TICK.register(LoteScene::tick);}

    private static Item item(String id){return BuiltInRegistries.ITEM.getValue(Identifier.parse(id));}
    private static Vec3 at(double x,double y,double z){return Vec3.atBottomCenterOf(base).add(x,y,z);}
    private static void look(Vec3 eye,Vec3 target){
        var d=target.subtract(eye);
        view=new InputScene.CameraView(eye,(float)Math.toDegrees(Math.atan2(-d.x,d.z)),(float)-Math.toDegrees(Math.atan2(d.y,d.horizontalDistance())));
    }
    private static void capture(Minecraft c,String file){Screenshot.grab(c.gameDirectory,file,c.gameRenderer.mainRenderTarget(),1,_->{});}

    static void start(Minecraft client){
        if(client.player==null||client.getSingleplayerServer()==null)throw new IllegalStateException("Open the development world");
        owner=client.player.getUUID();tick=0;result.entrySet().clear();
        launchStates.clear();darkStates.clear();lightStates.clear();husks.clear();bombShot=blastShot=domeShot=0;boardLow=Double.MAX_VALUE;launchPeak=0;
        pause=client.options.pauseOnLostFocus;client.options.pauseOnLostFocus=false;
        client.options.setCameraType(CameraType.THIRD_PERSON_BACK);
        onServer(client,p->{
            var server=p.level().getServer();
            server.getCommands().performPrefixedCommand(server.createCommandSourceStack(),"time set noon");
            server.getCommands().performPrefixedCommand(server.createCommandSourceStack(),"weather clear");
            var level=p.level();
            var old=p.position();var mode=p.gameMode.getGameModeForPlayer();
            boolean enabled=p.getAttachedOrCreate(JakData.ENABLED);
            var board=p.getAttachedOrCreate(JakData.BOARD).copy();
            var saved=new ArrayList<ItemStack>();
            for(int i=0;i<p.getInventory().getContainerSize();i++)saved.add(p.getInventory().getItem(i).copy());
            var near=p.blockPosition().offset(24,0,0);
            int y=near.getY();
            for(int x=-16;x<=16;x++)for(int z=-14;z<=14;z++)y=Math.max(y,level.getHeight(Heightmap.Types.MOTION_BLOCKING_NO_LEAVES,near.getX()+x,near.getZ()+z));
            // Well above everything round (a Haven roof overhung the launcher).
            base=near.atY(y+41);
            var snapshot=new LinkedHashMap<BlockPos,BlockState>();
            for(int x=-16;x<=16;x++)for(int z=-14;z<=14;z++)for(int h=-2;h<=8;h++){
                var cell=base.offset(x,h,z);
                snapshot.put(cell.immutable(),level.getBlockState(cell));
                boolean pool=x>=8&&x<=14&&z>=3&&z<=10;
                level.setBlock(cell,h==-2?Blocks.SMOOTH_STONE.defaultBlockState():h==-1?(pool?Blocks.WATER.defaultBlockState():Blocks.SMOOTH_STONE.defaultBlockState()):Blocks.AIR.defaultBlockState(),2);
            }
            p.setGameMode(GameType.CREATIVE);p.setAttached(JakData.ENABLED,true);
            p.setAttached(JakData.BOARD,new ItemStack(JakItems.BOARD));
            p.getInventory().clearContent();
            // The new decoration placed as Adrii places it: standing south of it, facing north.
            String[][] row={{"jakmc:eco_launcher","-10"},{"jakmc:iris_door","-6"},{"jakmc:airlock_door","-1"},{"jak:objeto_spargus_urna_alta","3"},
                {"jak:objeto_spargus_palmera_completa","5"},{"jak:objeto_villarrefugio_barril_1","8"},{"jak:objeto_haven_bidon_fuego","10"},
                {"jak:objeto_haven_trofeo_pez_bar","12"},{"jak:objeto_spargus_caja_mercado","14"}};
            var placed=new JsonObject();
            for(var entry:row){
                var thing=item(entry[0]);int x=Integer.parseInt(entry[1]);
                var cell=base.offset(x,0,-6);
                var stand=at(x,0,-3);p.teleportTo(stand.x,stand.y,stand.z);p.setYRot(180);p.setXRot(30);
                p.setItemInHand(InteractionHand.MAIN_HAND,new ItemStack(thing));
                var hit=new BlockHitResult(Vec3.atBottomCenterOf(cell),Direction.UP,cell.below(),false);
                placed.addProperty(entry[0],thing!=net.minecraft.world.item.Items.AIR&&thing.useOn(new UseOnContext(p,InteractionHand.MAIN_HAND,hit)).consumesAction());
            }
            // Blue eco: its vent before the launcher.
            var vent=base.offset(-10,0,-3);
            p.setItemInHand(InteractionHand.MAIN_HAND,new ItemStack(JakEcoVent.Color.BLUE.item));
            placed.addProperty("blue_vent",p.getMainHandItem().getItem().useOn(new UseOnContext(p,InteractionHand.MAIN_HAND,
                new BlockHitResult(Vec3.atBottomCenterOf(vent),Direction.UP,vent.below(),false))).consumesAction());
            result.add("placed",placed);
            var airlock=level.getBlockState(base.offset(-1,0,-6));
            result.addProperty("airlock_block",String.valueOf(BuiltInRegistries.BLOCK.getKey(airlock.getBlock())));
            // The hotbar: the new items' icons.
            String[] bar={"jakmc:eco_launcher","jakmc:iris_door","jakmc:airlock_door","jak:objeto_spargus_urna_alta","jak:objeto_spargus_palmera_completa",
                "jak:objeto_villarrefugio_barril_1","jak:objeto_haven_bidon_fuego","jak:objeto_haven_trofeo_pez_bar","jak:objeto_villarrefugio_barril_2"};
            for(int i=0;i<bar.length;i++)p.getInventory().setItem(i,new ItemStack(item(bar[i])));
            p.getInventory().setSelectedSlot(0);
            // The eco after the hotbar, in the slots left free.
            JakResources.grant(p,0,100);JakResources.grant(p,1,100);
            var home=at(0,0,0);p.teleportTo(home.x,home.y,home.z);p.setYRot(180);p.setXRot(0);
            JakPlayers.controlForProbe(p,true);JakPlayers.hostControlsForProbe(p);
            restore=()->{
                for(var id:husks){var e=level.getEntity(id);if(e!=null)e.discard();}
                snapshot.forEach((cell,state)->level.setBlock(cell,state,2));
                p.getInventory().clearContent();
                for(int i=0;i<saved.size();i++)p.getInventory().setItem(i,saved.get(i));
                JakPlayers.controlForProbe(p,false);
                p.setAttached(JakData.ENABLED,enabled);p.setAttached(JakData.BOARD,board);
                p.setGameMode(mode);p.teleportTo(old.x,old.y,old.z);
            };
            client.execute(()->{look(at(1.5,4.2,3.5),at(1.5,.8,-6));running=true;});
        });
    }

    private static void tick(Minecraft c){
        if(!running)return;
        int t=++tick;
        // No menu over the captures (the pause menu opens when the window loses focus).
        c.gui.setScreen(null);
        var pose=JakPoses.get(c.player.getId());
        String state=pose==null?"":pose.raw().get("state").getAsString();
        String clip=pose==null?"":pose.clip();
        if(t==30)capture(c,"lote-deco.png");
        if(t==32)look(at(13,2.6,-2.5),at(10,.8,-6));
        if(t==36)capture(c,"lote-deco-right.png");
        // The ghost: the airlock in hand, looking down at the floor ahead.
        if(t==38){view=null;c.player.getInventory().setSelectedSlot(2);c.player.setYRot(180);c.player.setXRot(28);onServer(c,p->{p.getInventory().setSelectedSlot(2);p.setYRot(180);p.setXRot(28);});}
        if(t==48)capture(c,"lote-ghost.png");
        if(t==50){
            c.player.setXRot(10);
            c.player.getInventory().setSelectedSlot(8);
            onServer(c,p->{p.getInventory().setSelectedSlot(8);var s=at(-1,0,-3);p.teleportTo(s.x,s.y,s.z);});
            look(at(2.5,2.2,-1),at(-1,1.5,-6));
        }
        if(t==57)capture(c,"lote-airlock-opening.png");
        if(t==200)capture(c,"lote-airlock-open.png");
        if(t==202)look(at(2.5,2.2,-11),at(-1,1.5,-6));
        if(t==206)capture(c,"lote-airlock-open-back.png");
        if(t==208)onServer(c,p->{
            result.addProperty("airlock_opened",p.level().getBlockState(base.offset(-1,0,-6)).getValue(JakIrisDoor.OPEN));
            var s=at(12,0,12);p.teleportTo(s.x,s.y,s.z);
        });
        if(t==224)capture(c,"lote-airlock-closing-back.png");
        if(t==260)capture(c,"lote-airlock-closed-back.png");
        if(t==262){
            onServer(c,p->result.addProperty("airlock_closed",!p.level().getBlockState(base.offset(-1,0,-6)).getValue(JakIrisDoor.OPEN)));
            look(at(-13,3,0),at(-7,1.2,-6));
        }
        // Blue eco from its vent, then the iris door and the launcher.
        if(t==266)onServer(c,p->{var s=at(-10,.24,-3);p.teleportTo(s.x,s.y,s.z);});
        if(t==296){capture(c,"lote-iris.png");onServer(c,p->{
            result.addProperty("blue_eco",JakPlayers.blueEco(p));
            result.addProperty("iris_opened",p.level().getBlockState(base.offset(-6,0,-6)).getValue(JakIrisDoor.OPEN));
            var s=at(-10,.2,-6);p.teleportTo(s.x,s.y,s.z);
        });}
        if(t>=296&&t<360){
            if(!state.isEmpty())launchStates.add(state+"/"+clip);
            if(pose!=null)launchPeak=Math.max(launchPeak,pose.position().y-at(0,0,0).y);
        }
        if(t==298)view=null;
        if(t==304)capture(c,"lote-launch-crouch.png");
        if(t==316)capture(c,"lote-launch-air.png");
        // Dark Jak: three husks, his change, the bomb (X then Square), again, the blast (X then Circle).
        if(t==360)onServer(c,p->{
            var s=at(0,0,6);p.teleportTo(s.x,s.y,s.z);p.setYRot(0);
            for(int i=0;i<3;i++){
                var husk=net.minecraft.world.entity.EntityTypes.HUSK.create(p.level(),EntitySpawnReason.COMMAND);
                if(husk==null)continue;
                var h=at(-3+3*i,0,10+(i==1?1:0));husk.setPos(h);husk.setNoAi(true);p.level().addFreshEntity(husk);husks.add(husk.getUUID());
            }
        });
        if(t==362)look(at(7,3.5,3),at(0,1,9));
        if(t==366)input(c,256,256);
        if(t==369)input(c,0,0);
        if(t==400)result.addProperty("dark_before_bomb",pose!=null&&pose.dark());
        if(t==402)input(c,1,1);
        if(t==408)input(c,1|2,2);
        if(t==410)input(c,0,0);
        if(t>=402&&t<480){
            if(!state.isEmpty())darkStates.add(state+"/"+clip);
            if(bombShot==0&&clip.equals("jakb-darkjak-bomb-land"))bombShot=t+3;
        }
        if(t==bombShot)capture(c,"lote-dark-bomb.png");
        if(domeShot==0&&t>=402&&t<480&&pose!=null&&pose.raw().get("bombBlast") instanceof JsonArray blast&&blast.get(3).getAsFloat()>=.12f){domeShot=t;capture(c,"lote-dark-bomb-dome.png");}
        if(t==482)onServer(c,p->{
            result.add("husks_after_bomb",health(p));result.addProperty("dark_eco_after_bomb",JakResources.totals(p)[0]);
            // Fresh husks for the blast.
            for(var id:husks){var e=p.level().getEntity(id);if(e!=null)e.discard();}
            husks.clear();
            for(int i=0;i<3;i++){
                var husk=net.minecraft.world.entity.EntityTypes.HUSK.create(p.level(),EntitySpawnReason.COMMAND);
                if(husk==null)continue;
                var h=at(-4+4*i,0,12);husk.setPos(h);husk.setNoAi(true);p.level().addFreshEntity(husk);husks.add(husk.getUUID());
            }
        });
        // Back to Dark Jak once the change back is over.
        if(t==496&&pose!=null&&!pose.dark())input(c,256,256);
        if(t==499)input(c,0,0);
        if(t==520)result.addProperty("dark_before_blast",pose!=null&&pose.dark());
        if(t==522)input(c,1,1);
        if(t==530)input(c,1|4,4);
        if(t==532)input(c,0,0);
        if(t>=522&&t<600){
            if(!state.isEmpty())darkStates.add(state+"/"+clip);
            if(blastShot==0&&state.equals("darkjak-bomb1")&&clip.startsWith("jakb-darkjak-attack-ice-loop"))blastShot=t+6;
        }
        if(t==blastShot)capture(c,"lote-dark-blast.png");
        if(t==602)onServer(c,p->{
            result.add("husks_after_blast",health(p));result.addProperty("dark_eco_after_blast",JakResources.totals(p)[0]);
            var frozen=new JsonArray();for(var id:husks)if(p.level().getEntity(id) instanceof LivingEntity l)frozen.add(l.getTicksFrozen());
            result.add("husks_frozen_ticks",frozen);
            result.addProperty("dark_after_blast",false);
        });
        if(t==604)result.addProperty("dark_after_blast",pose!=null&&pose.dark());
        if(t==606&&pose!=null&&pose.dark()){input(c,256,256);}
        if(t==609)input(c,0,0);
        // Light Jak with wings: L3 held, then X (CONTROLES.md), with light eco.
        if(t==640)input(c,256,256);
        if(t==644)input(c,256|1,1);
        if(t==648)input(c,256,0);
        if(t>=640&&t<720&&!state.isEmpty())lightStates.add(state+"/"+clip);
        if(t==672)input(c,0,0);
        if(t==676)look(at(4,2.2,4),at(0,1,6));
        if(t==680)capture(c,"lote-light.png");
        if(t==686)look(at(1.2,1.6,8.5),at(0,1,6));
        if(t==690)capture(c,"lote-light-close.png");
        if(t==692){input(c,256,256);}
        if(t==695)input(c,0,0);
        // The board on his back, then on it over the water (R2, then the stick on east).
        if(t==720){onServer(c,p->{var s=at(5,0,6);p.teleportTo(s.x,s.y,s.z);p.setYRot(0);});look(at(5,2,2.5),at(5,1,6));}
        if(t==726)capture(c,"lote-board-back.png");
        if(t==728)onServer(c,p->{var s=at(6,0,6.5);p.teleportTo(s.x,s.y,s.z);p.setYRot(-90);});
        if(t==732)input(c,128,128);
        if(t==735)input(c,0,0);
        if(t>=750&&t<820){
            onServer(c,p->JakPlayers.probeInput(p,0,0,1));
            if(pose!=null&&pose.position().x>at(8.5,0,0).x)boardLow=Math.min(boardLow,pose.position().y-at(0,0,0).y);
        }
        if(t==760){view=null;}
        if(t==780)capture(c,"lote-board-water.png");
        if(t==820){
            onServer(c,p->JakPlayers.probeInput(p,0,0,0));
            result.add("launch_states",strings(launchStates));result.addProperty("launch_peak_blocks",launchPeak);
            result.add("dark_states",strings(darkStates));result.add("light_states",strings(lightStates));
            result.addProperty("board_lowest_over_water",boardLow==Double.MAX_VALUE?-99:boardLow);
            result.addProperty("visual_review","required");result.addProperty("reason","complete");
            onServer(c,p->{
                var recipes=p.level().getServer().getRecipeManager();
                int cut=0;for(var holder:recipes.getRecipes())if(holder.id().identifier().getNamespace().equals("jak")&&holder.value().getType()==net.minecraft.world.item.crafting.RecipeType.STONECUTTING)cut++;
                result.addProperty("jak_stonecutting_recipes",cut);
                for(String id:new String[]{"eco_launcher","iris_door","airlock_door"})
                    result.addProperty("recipe_"+id,recipes.byKey(net.minecraft.resources.ResourceKey.create(net.minecraft.core.registries.Registries.RECIPE,JakData.id(id))).isPresent());
                try{
                    var dir=c.gameDirectory.toPath().resolve("jakmc/evidence/lote");
                    Files.createDirectories(dir);Files.writeString(dir.resolve("result.json"),result.toString());
                }catch(IOException failure){throw new UncheckedIOException(failure);}
                restore.run();
                c.execute(()->{running=false;view=null;c.options.pauseOnLostFocus=pause;c.stop();});
            });
        }
    }
    private static JsonArray strings(java.util.Collection<String> values){var out=new JsonArray();values.forEach(out::add);return out;}
    private static JsonArray health(ServerPlayer p){
        var out=new JsonArray();
        for(var id:husks)out.add(p.level().getEntity(id) instanceof LivingEntity l&&l.isAlive()?l.getHealth():-1);
        return out;
    }
    private static void input(Minecraft c,int held,int pressed){onServer(c,p->JakPlayers.probeInput(p,held,pressed,0));}
    private static void onServer(Minecraft client,java.util.function.Consumer<ServerPlayer> action){
        var server=client.getSingleplayerServer();
        server.execute(()->{var player=server.getPlayerList().getPlayer(owner);if(player!=null)action.accept(player);});
    }
}
