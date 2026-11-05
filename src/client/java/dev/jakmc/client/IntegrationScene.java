package dev.jakmc.client;

import com.google.gson.JsonObject;
import dev.jakmc.JakData;
import dev.jakmc.content.JakItems;
import dev.jakmc.entity.JakFlutFlut;
import dev.jakmc.player.JakPlayers;
import dev.jakmc.player.JakWorldDamage;
import java.nio.file.Files;
import net.fabricmc.fabric.api.client.event.lifecycle.v1.ClientTickEvents;
import net.minecraft.client.Minecraft;
import net.minecraft.client.Screenshot;
import net.minecraft.core.BlockPos;
import net.minecraft.server.level.ServerPlayer;
import net.minecraft.world.InteractionHand;
import net.minecraft.world.entity.EntitySpawnReason;
import net.minecraft.world.item.ItemStack;
import net.minecraft.world.item.Items;
import net.minecraft.world.level.GameType;
import net.minecraft.world.level.block.Blocks;
import net.minecraft.world.phys.Vec3;

/** One bounded regression take in the runner's private world copy. */
final class IntegrationScene {
    private static volatile boolean running;
    private static java.util.UUID player;
    private static int serverTick;
    private static int tick;
    private static volatile int stage;
    private static JsonObject checks;
    private static BlockPos base;
    private static JakFlutFlut bird;
    private static float health;
    private static double peak;
    private static final java.util.Set<String> WATER_STATES=new java.util.concurrent.ConcurrentSkipListSet<>();
    private static InputScene.CameraView view;
    static InputScene.CameraView camera(){return view;}
    static void initialize(){
        ClientTickEvents.END_CLIENT_TICK.register(IntegrationScene::tick);
        net.fabricmc.fabric.api.event.lifecycle.v1.ServerTickEvents.START_SERVER_TICK.register(server->{
            if(!running)return;
            var p=server.getPlayerList().getPlayer(player);
            if(p!=null&&serverTick<=255)step(p,++serverTick);
        });
    }
    static void start(Minecraft c){
        if(running||c.player==null||c.getSingleplayerServer()==null)throw new IllegalStateException("Open the isolated dev world");
        WATER_STATES.clear();tick=stage=serverTick=0;checks=new JsonObject();peak=-100;bird=null;view=null;player=c.player.getUUID();running=true;
        c.options.setCameraType(net.minecraft.client.CameraType.THIRD_PERSON_BACK);
    }
    private static void check(String key,boolean value){checks.addProperty(key,value);}
    private static void control(ServerPlayer p,int held,int pressed,float forward){JakPlayers.probeInput(p,held,pressed,forward);}
    private static void move(ServerPlayer p,Vec3 at){
        p.teleportTo(at.x,at.y,at.z);p.setYRot(0);p.setXRot(0);
        p.level().getChunkSource().move(p);JakPlayers.resetForProbe(p);control(p,0,0,0);
    }
    private static void setup(ServerPlayer p){
        var l=p.level();
        base=new BlockPos(p.getBlockX(),l.getMaxY()-12,p.getBlockZ());
        p.level().getServer().getCommands().performPrefixedCommand(p.level().getServer().createCommandSourceStack(),"time set noon");
        for(int x=-2;x<=34;x++)for(int z=-2;z<=18;z++){
            l.setBlock(base.offset(x,-1,z),Blocks.STONE.defaultBlockState(),3);
            for(int h=0;h<=6;h++)l.setBlock(base.offset(x,h,z),Blocks.AIR.defaultBlockState(),3);
        }
        for(int x=-1;x<=8;x++)for(int z=-1;z<=8;z++)for(int h=-4;h<=-1;h++)
            l.setBlock(base.offset(x,h,z),(x==-1||x==8||z==-1||z==8||h==-4?Blocks.STONE:Blocks.WATER).defaultBlockState(),3);
        p.setAttached(JakData.ENABLED,true);p.setAttached(JakData.BOARD,ItemStack.EMPTY);
        p.setItemInHand(InteractionHand.MAIN_HAND,ItemStack.EMPTY);p.setGameMode(GameType.SURVIVAL);
        JakPlayers.controlForProbe(p,true);move(p,Vec3.atBottomCenterOf(base.offset(3,-1,3)));
        view=look(Vec3.atBottomCenterOf(base.offset(7,2,0)),Vec3.atBottomCenterOf(base.offset(3,0,3)));
    }
    private static void step(ServerPlayer p,int t){
        var l=p.level();
        if(t==1)setup(p);
        if(t==30)control(p,1,1,0);
        if(t>=31&&t<70){control(p,1,0,0);peak=Math.max(peak,p.getY()-(base.getY()-.111111));}
        if(t==70){check("swim_jump_feet_clear_surface",peak>.3);checks.addProperty("swim_jump_clearance",peak);checks.addProperty("water_states",WATER_STATES.toString());}
        if(t==75){
            move(p,Vec3.atBottomCenterOf(base.offset(12,0,2)));
            for(int x=9;x<=17;x++)for(int h=0;h<=5;h++)l.setBlock(base.offset(x,h,8),Blocks.STONE.defaultBlockState(),3);
            bird=JakFlutFlut.TYPE.create(l,EntitySpawnReason.COMMAND);bird.setNoAi(true);
            bird.setPos(Vec3.atBottomCenterOf(base.offset(12,0,4)));l.addFreshEntity(bird);
            p.setItemInHand(InteractionHand.MAIN_HAND,new ItemStack(Items.WHEAT_SEEDS,34));
            for(int n=0;n<34;n++)bird.mobInteract(p,InteractionHand.MAIN_HAND);
            p.setItemInHand(InteractionHand.MAIN_HAND,ItemStack.EMPTY);bird.mobInteract(p,InteractionHand.MAIN_HAND);
            p.setItemInHand(InteractionHand.MAIN_HAND,new ItemStack(Items.SADDLE));bird.mobInteract(p,InteractionHand.MAIN_HAND);
            p.setItemInHand(InteractionHand.MAIN_HAND,ItemStack.EMPTY);health=bird.getHealth();
            check("flut_ready_and_mounted",bird.readyToMount()&&JakPlayers.mount(p,bird));
            view=look(Vec3.atBottomCenterOf(base.offset(16,3,5)),Vec3.atBottomCenterOf(base.offset(12,1,7)));
        }
        if(t>=105&&t<155)control(p,0,0,1);
        if(t==155){
            check("flut_stopped_at_wall",bird.getZ()+bird.getBbWidth()/2<=base.getZ()+8.001&&bird.getZ()>base.getZ()+6);
            check("flut_no_block_overlap",!l.getBlockCollisions(bird,bird.getBoundingBox().deflate(.001)).iterator().hasNext());
            check("flut_no_wall_damage",bird.getHealth()==health);
            control(p,8,8,0);
        }
        if(t==175){
            bird.discard();move(p,Vec3.atBottomCenterOf(base.offset(25,0,2)));p.setGameMode(GameType.CREATIVE);
            p.setItemInHand(InteractionHand.MAIN_HAND,new ItemStack(JakItems.GUNS.get(3)));
            for(int x=24;x<=26;x++)for(int h=0;h<=3;h++)l.setBlock(base.offset(x,h,8),Blocks.GLASS.defaultBlockState(),3);
            view=look(Vec3.atBottomCenterOf(base.offset(28,3,0)),Vec3.atBottomCenterOf(base.offset(25,1,6)));
        }
        if(t>=195&&t<235)control(p,64,t==195?64:0,0);
        if(t==240){
            int left=0;for(int x=24;x<=26;x++)for(int h=0;h<=3;h++)if(l.getBlockState(base.offset(x,h,8)).is(Blocks.GLASS))left++;
            check("native_gun_impacts_destroy_blocks",left<12);control(p,0,0,0);
            p.setGameMode(GameType.SURVIVAL);p.setItemInHand(InteractionHand.MAIN_HAND,ItemStack.EMPTY);
            var at=base.offset(29,0,13);
            for(int x=-2;x<=2;x++)for(int z=-2;z<=2;z++)l.setBlock(at.offset(x,0,z),Blocks.OAK_LEAVES.defaultBlockState(),3);
            l.setBlock(at,Blocks.DIRT.defaultBlockState(),3);
            l.setBlock(at.above(),Blocks.SHORT_GRASS.defaultBlockState(),3);
            check("vegetation_present_before_blast",l.getBlockState(at.above()).is(Blocks.SHORT_GRASS));
            var bedrock=at.below();l.setBlock(bedrock,Blocks.BEDROCK.defaultBlockState(),3);
            var cow=dev.jakmc.entity.JakLeaper.TYPE.create(l,EntitySpawnReason.COMMAND);
            cow.setNoAi(true);cow.setPos(Vec3.atBottomCenterOf(at.offset(0,0,2)));l.addFreshEntity(cow);
            float before=cow.getHealth();
            JakWorldDamage.explosion(l,p,p,Vec3.atCenterOf(at),3);
            check("explosion_breaks_vegetation",l.getBlockState(at.above()).isAir());
            check("explosion_native_drops",!l.getEntitiesOfClass(net.minecraft.world.entity.item.ItemEntity.class,new net.minecraft.world.phys.AABB(at).inflate(5),e->e.getItem().is(Items.DIRT)).isEmpty());
            check("explosion_respects_bedrock",l.getBlockState(bedrock).is(Blocks.BEDROCK));
            check("world_explosion_no_duplicate_actor_damage",cow.getHealth()==before&&cow.getDeltaMovement().lengthSqr()<1e-10);
            cow.discard();
            var wall=base.offset(44,3,12);l.setBlock(wall,Blocks.STONE.defaultBlockState(),3);
            int k=0;
            for(var model:dev.jakmc.entity.JakVehicle.Model.values()){
                float hx=model==dev.jakmc.entity.JakVehicle.Model.TOAD?2.3f:model==dev.jakmc.entity.JakVehicle.Model.SNAKE?1.8f:model.bike()?.8f:1.4f;
                float hy=model==dev.jakmc.entity.JakVehicle.Model.TOAD?1.15f:model==dev.jakmc.entity.JakVehicle.Model.SNAKE?.65f:model.bike()?.6f:.7f;
                float hz=model==dev.jakmc.entity.JakVehicle.Model.TOAD?4.8f:model==dev.jakmc.entity.JakVehicle.Model.SNAKE?3.5f:model.bike()?1.5f:2.4f;
                double rx=(hx+hz)*.64/Math.sqrt(2),rz=(hz-hx)*.64/Math.sqrt(2);
                Vec3 corner=new Vec3(wall.getX()-rx+.03,wall.getY()+.5,wall.getZ()-rz+.2);
                var query=new dev.jakmc.nativebridge.WheeledCollision(l,p,corner,model.hover());
                var hit=query.sweepHull(0,0,0,0,0,0,hx,hy,hz,.70710677f,0,.70710677f);
                check("oriented_nose_corner_"+model.id,hit[0]==0);
                query.origin=new Vec3(wall.getX()-rx+.03,wall.getY()+.5,wall.getZ()-rx+.03);
                hit=query.sweepHull(0,0,0,0,0,0,hx,hy,hz,.70710677f,0,.70710677f);
                check("oriented_hull_not_oversized_"+model.id,hit[0]==1);
                query.origin=corner.add(-.5,0,0);
                hit=query.sweepHull(0,0,0,.78125f,0,0,hx,hy,hz,.70710677f,0,.70710677f);
                check("oriented_hull_continuous_"+model.id,hit[0]>0&&hit[0]<1);
                k++;
            }
            check("all_vehicle_hulls_checked",k==8);
        }
        stage=t;
    }
    private static void tick(Minecraft c){
        if(!running)return;
        if(c.player==null||c.getSingleplayerServer()==null){running=false;view=null;return;}
        if(tick<70){var pose=JakPoses.get(c.player.getId());if(pose!=null)WATER_STATES.add(pose.raw().get("state").getAsString()+"/"+pose.clip());}
        int previous=tick;
        int t=stage;tick=t;
        if(t>255){
            try{
                var out=c.gameDirectory.toPath().resolve("jakmc/evidence/integration");Files.createDirectories(out);
                boolean success=checks.entrySet().stream().filter(e->e.getValue().isJsonPrimitive()&&e.getValue().getAsJsonPrimitive().isBoolean()).allMatch(e->e.getValue().getAsBoolean());
                var result=new JsonObject();result.addProperty("reason",success?"complete":"integration regression failed");result.add("checks",checks);
                Files.writeString(out.resolve("result.json"),result.toString());
            }catch(Exception error){throw new RuntimeException(error);}
            running=false;view=null;c.stop();return;
        }
        for(int shot:new int[]{43,155,240})if(previous<shot&&t>=shot)
            Screenshot.grab(c.gameDirectory,"integration-"+shot+".png",c.gameRenderer.mainRenderTarget(),1,m->{});
    }
    private static InputScene.CameraView look(Vec3 at,Vec3 target){
        Vec3 d=target.subtract(at);return new InputScene.CameraView(at,(float)Math.toDegrees(Math.atan2(-d.x,d.z)),(float)Math.toDegrees(Math.atan2(-d.y,d.horizontalDistance())));
    }
}
