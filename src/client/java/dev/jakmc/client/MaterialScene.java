package dev.jakmc.client;

import com.google.gson.JsonObject;
import dev.jakmc.content.JakBlocks;
import java.nio.file.Files;
import net.fabricmc.fabric.api.client.event.lifecycle.v1.ClientTickEvents;
import net.minecraft.client.Minecraft;
import net.minecraft.client.Screenshot;
import net.minecraft.core.BlockPos;
import net.minecraft.core.Direction;
import net.minecraft.server.level.ServerPlayer;
import net.minecraft.world.InteractionHand;
import net.minecraft.world.item.ItemStack;
import net.minecraft.world.item.Items;
import net.minecraft.world.item.context.UseOnContext;
import net.minecraft.world.level.GameType;
import net.minecraft.world.level.block.Block;
import net.minecraft.world.level.block.Blocks;
import net.minecraft.world.level.block.state.properties.BlockStateProperties;
import net.minecraft.world.phys.BlockHitResult;
import net.minecraft.world.phys.Vec3;

final class MaterialScene {
    private static int captureTicks;
    private static volatile JsonObject result;
    private static volatile Vec3 subject;
    private static Vec3 assemblySubject;
    private static Runnable cleanup;
    private static Runnable hitProjector;
    private static volatile boolean thirdHitChecked;
    private static boolean burstDrawn;

    static void initialize(){
        ClientTickEvents.END_CLIENT_TICK.register(client->{
            if(result==null)return;
            ++captureTicks;
            burstDrawn|=JakEffects.drawnBurstVertices>0;
            if(captureTicks==60){
                result.getAsJsonObject("actions").addProperty("projector_visible",JakEffects.drawnProjectorVertices>0);
                Screenshot.grab(client.gameDirectory,"jak-materials.png",client.gameRenderer.mainRenderTarget(),1,_->{});
            }
            if(captureTicks==68||captureTicks==76||captureTicks==84)
                client.getSingleplayerServer().execute(hitProjector);
            if(captureTicks==95)subject=assemblySubject;
            if(captureTicks<110||!thirdHitChecked)return;
            Screenshot.grab(client.gameDirectory,"jak-assemblies.png",client.gameRenderer.mainRenderTarget(),1,_->{});
            result.getAsJsonObject("actions").addProperty("projector_break_effect",burstDrawn&&JakEffects.drawnProjectorVertices==0);
            result.getAsJsonObject("actions").addProperty("original_fire_vertices",JakEffects.drawnBarrelVertices>0);
            boolean complete=result.getAsJsonObject("actions").entrySet().stream().allMatch(e->e.getValue().getAsBoolean());
            result.addProperty("reason",complete?"complete":"Material or decoration assertion failed");
            try{
                var dir=client.gameDirectory.toPath().resolve("jakmc/evidence/materials");
                Files.createDirectories(dir);
                Files.writeString(dir.resolve("result.json"),result.toString());
            }catch(java.io.IOException e){throw new java.io.UncheckedIOException(e);}
            result=null;
            var restore=cleanup;
            if(restore!=null)client.getSingleplayerServer().execute(restore);
            client.stop();
        });
    }

    static InputScene.CameraView camera(){
        if(result==null||subject==null)return null;
        Vec3 pos=captureTicks>=95?subject.add(4.2,2.7,-6):subject.add(2.7,2.2,-3.6);
        Vec3 look=subject.subtract(pos);
        return new InputScene.CameraView(pos,(float)Math.toDegrees(Math.atan2(-look.x,look.z)),
            (float)-Math.toDegrees(Math.atan2(look.y,look.horizontalDistance())));
    }

    static void start(Minecraft client){
        if(client.getSingleplayerServer()==null||client.player==null)throw new IllegalStateException("Open the development world");
        captureTicks=0;
        thirdHitChecked=false;
        burstDrawn=false;
        var server=client.getSingleplayerServer();
        var uuid=client.player.getUUID();
        server.execute(()->{
            ServerPlayer p=server.getPlayerList().getPlayer(uuid);
            if(p==null)return;
            var level=p.level();
            var near=p.blockPosition().offset(3,0,3);
            int openY=near.getY();
            for(int x=-2;x<=5;x++)for(int z=-5;z<=2;z++)
                openY=Math.max(openY,level.getHeight(net.minecraft.world.level.levelgen.Heightmap.Types.MOTION_BLOCKING,
                    near.getX()+x,near.getZ()+z));
            var base=near.atY(openY+1);
            var soil=base.below();
            var hand=InteractionHand.MAIN_HAND;
            var previous=p.getItemInHand(hand).copy();
            var previousMode=p.gameMode.getGameModeForPlayer();
            var initial=level.getBlockState(base);
            var initialFloor=level.getBlockState(soil);
            var sample=base.east();
            var initialSample=level.getBlockState(sample);
            var propPos=sample.above();
            var initialProp=level.getBlockState(propPos);
            var barrelFloor=sample.east();
            var barrelPos=barrelFloor.above();
            var initialBarrelFloor=level.getBlockState(barrelFloor);
            var initialBarrel=level.getBlockState(barrelPos);
            var projectorFloor=base.west();
            var projectorPos=projectorFloor.above();
            var initialProjectorFloor=level.getBlockState(projectorFloor);
            var initialProjector=level.getBlockState(projectorPos);
            var urnPos=base.west(3);
            var palmPos=base.offset(0,0,8);
            var assemblySnapshot=new java.util.LinkedHashMap<BlockPos,net.minecraft.world.level.block.state.BlockState>();
            for(int x=-5;x<=5;x++)for(int z=3;z<=13;z++)for(int y=-1;y<=5;y++){
                var cell=base.offset(x,y,z);
                assemblySnapshot.put(cell,level.getBlockState(cell));
            }
            for(int y=-1;y<=1;y++)assemblySnapshot.put(urnPos.above(y),level.getBlockState(urnPos.above(y)));
            cleanup=()->{
                level.setBlockAndUpdate(soil,initialFloor);
                level.setBlockAndUpdate(base,initial);
                level.setBlockAndUpdate(sample,initialSample);
                level.setBlockAndUpdate(propPos,initialProp);
                level.setBlockAndUpdate(barrelFloor,initialBarrelFloor);
                level.setBlockAndUpdate(barrelPos,initialBarrel);
                level.setBlockAndUpdate(projectorPos,initialProjector);
                level.setBlockAndUpdate(projectorFloor,initialProjectorFloor);
                assemblySnapshot.forEach(level::setBlockAndUpdate);
                p.setItemInHand(hand,previous);
            };
            var layer=JakBlocks.BLOCKS.get("barrio_losa_capa");
            var tests=new JsonObject();
            try{
                level.setBlockAndUpdate(soil,Blocks.STONE.defaultBlockState());
                level.setBlockAndUpdate(base,Blocks.AIR.defaultBlockState());
                p.setGameMode(GameType.SURVIVAL);
                p.setItemInHand(hand,new ItemStack(layer,8));
                boolean placed=use(p,soil,1)&&use(p,base,.125)&&use(p,base,.25);
                var state=level.getBlockState(base);
                tests.addProperty("placed_and_stacked",placed&&p.getItemInHand(hand).getCount()==5&&state.is(layer)
                    &&state.getValue(BlockStateProperties.LAYERS)==3);
                tests.addProperty("collision_matches_height",state.is(layer)&&state.getCollisionShape(level,base).max(Direction.Axis.Y)==.375
                    &&state.getShape(level,base).max(Direction.Axis.Y)==.375);
                var pickaxe=new ItemStack(Items.DIAMOND_PICKAXE);
                tests.addProperty("native_loot",Block.getDrops(state,level,base,null,p,pickaxe).stream()
                    .filter(stack->stack.is(layer.asItem())).mapToInt(ItemStack::getCount).sum()==3);
                tests.addProperty("material_catalog",JakBlocks.BLOCKS.size()>1000);
                tests.addProperty("native_door",JakBlocks.BLOCKS.values().stream().anyMatch(block->block instanceof net.minecraft.world.level.block.DoorBlock));
                subject=Vec3.atBottomCenterOf(base).add(.5,.9,0);
                level.setBlockAndUpdate(sample,JakBlocks.BLOCKS.get("barrio_losa").defaultBlockState());
                level.setBlockAndUpdate(propPos,Blocks.AIR.defaultBlockState());
                var tray=JakBlocks.BLOCKS.get("objeto_haven_bandeja_bar");
                p.setItemInHand(hand,new ItemStack(tray,2));
                boolean placedTray=use(p,sample,1);
                var trayState=level.getBlockState(propPos);
                tests.addProperty("native_prop_placement",placedTray&&trayState.is(tray)&&p.getItemInHand(hand).getCount()==1
                    &&trayState.getValue(BlockStateProperties.HORIZONTAL_FACING)==p.getDirection().getOpposite());
                var trayShape=trayState.getCollisionShape(level,propPos);
                tests.addProperty("native_prop_shape",!trayShape.isEmpty()&&trayShape.max(Direction.Axis.Y)<1
                    &&trayShape.equals(trayState.getShape(level,propPos)));
                tests.addProperty("native_prop_loot",Block.getDrops(trayState,level,propPos,null,p,pickaxe).stream()
                    .filter(stack->stack.is(tray.asItem())).mapToInt(ItemStack::getCount).sum()==1);
                tests.addProperty("native_prop_inventory",p.getItemInHand(hand).is(tray.asItem()));
                var barrel=JakBlocks.BLOCKS.get("objeto_haven_bidon_fuego");
                level.setBlockAndUpdate(barrelFloor,JakBlocks.BLOCKS.get("barrio_losa").defaultBlockState());
                level.setBlockAndUpdate(barrelPos,barrel.defaultBlockState());
                var halfWall=JakBlocks.BLOCKS.get("palacio_pared_azul_medio_vertical");
                var north=halfWall.defaultBlockState();
                var east=north.setValue(BlockStateProperties.HORIZONTAL_FACING,Direction.EAST);
                var n=north.getCollisionShape(level,propPos);
                var e=east.getCollisionShape(level,propPos);
                tests.addProperty("architectural_shape_rotates",n.min(Direction.Axis.Z)==0&&n.max(Direction.Axis.Z)==.5
                    &&e.min(Direction.Axis.X)==.5&&e.max(Direction.Axis.X)==1
                    &&n.equals(north.getShape(level,propPos))&&e.equals(east.getShape(level,propPos)));
                tests.addProperty("barrel_light_and_lifecycle",level.getBlockState(barrelPos).getLightEmission()==12
                    &&level.getBlockEntity(barrelPos) instanceof dev.jakmc.content.JakProps.FireBarrel);
                var projector=JakBlocks.BLOCKS.get("objeto_haven_propaganda_bloque");
                level.setBlockAndUpdate(projectorFloor,JakBlocks.BLOCKS.get("barrio_losa").defaultBlockState());
                level.setBlockAndUpdate(projectorPos,projector.defaultBlockState());
                tests.addProperty("projector_grid",level.getBlockEntity(projectorPos) instanceof dev.jakmc.content.JakProps.Projector
                    &&level.getBlockState(projectorPos.above()).isAir());
                var urn=JakBlocks.BLOCKS.get("objeto_spargus_urna_baja");
                level.setBlockAndUpdate(urnPos.below(),Blocks.STONE.defaultBlockState());
                level.setBlockAndUpdate(urnPos,Blocks.AIR.defaultBlockState());
                level.setBlockAndUpdate(urnPos.above(),Blocks.STONE.defaultBlockState());
                p.setItemInHand(hand,new ItemStack(urn,2));
                tests.addProperty("assembly_rejects_obstruction",!use(p,urnPos.below(),1)&&p.getItemInHand(hand).getCount()==2
                    &&level.getBlockState(urnPos).isAir()&&level.getBlockState(urnPos.above()).is(Blocks.STONE));
                level.setBlockAndUpdate(urnPos.above(),Blocks.AIR.defaultBlockState());
                boolean urnPlaced=use(p,urnPos.below(),1);
                var upper=level.getBlockState(urnPos.above());
                tests.addProperty("two_cell_urn",urnPlaced&&level.getBlockState(urnPos).is(urn)
                    &&upper.getBlock().asItem()==urn.asItem()&&p.getItemInHand(hand).getCount()==1
                    &&!upper.getCollisionShape(level,urnPos.above()).isEmpty());
                level.destroyBlock(urnPos.above(),true,p);
                var urnDrops=level.getEntitiesOfClass(net.minecraft.world.entity.item.ItemEntity.class,
                    new net.minecraft.world.phys.AABB(urnPos).inflate(2),item->item.getItem().is(urn.asItem()));
                tests.addProperty("assembly_one_drop",level.getBlockState(urnPos).isAir()&&urnDrops.size()==1
                    &&urnDrops.getFirst().getItem().getCount()==1);
                urnDrops.forEach(net.minecraft.world.entity.Entity::discard);
                for(int x=-4;x<=4;x++)for(int z=-4;z<=4;z++){
                    level.setBlockAndUpdate(palmPos.offset(x,-1,z),Blocks.SANDSTONE.defaultBlockState());
                    for(int y=0;y<5;y++)level.setBlockAndUpdate(palmPos.offset(x,y,z),Blocks.AIR.defaultBlockState());
                }
                var palm=JakBlocks.BLOCKS.get("objeto_spargus_palmera_completa");
                p.setItemInHand(hand,new ItemStack(palm,2));
                boolean palmPlaced=use(p,palmPos.below(),1);
                int palmCells=0,softCells=0;
                for(var at:assemblySnapshot.keySet()){
                    var placedPalm=level.getBlockState(at);
                    if(placedPalm.getBlock().asItem()!=palm.asItem())continue;
                    ++palmCells;
                    if(placedPalm.getCollisionShape(level,at).isEmpty()&&!placedPalm.getShape(level,at).isEmpty())++softCells;
                }
                tests.addProperty("palm_complete",palmPlaced&&palmCells==73&&p.getItemInHand(hand).getCount()==1);
                tests.addProperty("palm_soft_leaf_solid_trunk",softCells>0
                    &&!level.getBlockState(palmPos).getCollisionShape(level,palmPos).isEmpty());
                p.setItemInHand(hand,new ItemStack(tray));
                assemblySubject=Vec3.atBottomCenterOf(palmPos).add(0,2,0);

                int[] hitCount={0};
                hitProjector=()->{
                    var old=level.getBlockState(projectorPos);
                    var mode=p.gameMode.getGameModeForPlayer();
                    p.setGameMode(GameType.SURVIVAL);
                    try{old.attack(level,projectorPos,p);}finally{p.setGameMode(mode);}
                    var current=level.getBlockState(projectorPos);
                    int hit=++hitCount[0];
                    var drops=level.getEntitiesOfClass(net.minecraft.world.entity.item.ItemEntity.class,
                        new net.minecraft.world.phys.AABB(projectorPos).inflate(1),item->item.getItem().is(projector.asItem()));
                    boolean ok=hit<3?current.is(projector)&&current.getValue(dev.jakmc.content.JakProps.HITS)==hit&&drops.isEmpty()
                        :current.isAir()&&drops.size()==1&&drops.getFirst().getItem().getCount()==1;
                    if(hit==3)drops.forEach(net.minecraft.world.entity.Entity::discard);
                    client.execute(()->{
                        tests.addProperty("projector_hit_"+hit,ok);
                        if(hit==3)thirdHitChecked=true;
                    });
                };
            }finally{
                p.setGameMode(previousMode);
            }
            var report=new JsonObject();
            report.add("actions",tests);
            report.addProperty("blocks",JakBlocks.BLOCKS.size());
            report.addProperty("reason",tests.entrySet().stream().allMatch(e->e.getValue().getAsBoolean())?"complete":"Material assertion failed");
            client.execute(()->result=report);
        });
    }

    private static boolean use(ServerPlayer player,BlockPos clicked,double height){
        var location=new Vec3(clicked.getX()+.5,clicked.getY()+height,clicked.getZ()+.5);
        var hit=new BlockHitResult(location,Direction.UP,clicked,false);
        return player.getItemInHand(InteractionHand.MAIN_HAND).getItem()
            .useOn(new UseOnContext(player,InteractionHand.MAIN_HAND,hit)).consumesAction();
    }
}
