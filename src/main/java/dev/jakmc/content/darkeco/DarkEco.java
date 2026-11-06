package dev.jakmc.content.darkeco;

import dev.jakmc.JakData;
import dev.jakmc.content.JakItems;
import java.util.*;
import net.fabricmc.fabric.api.creativetab.v1.CreativeModeTabEvents;
import net.fabricmc.fabric.api.event.lifecycle.v1.ServerTickEvents;
import net.minecraft.core.*;
import net.minecraft.core.registries.*;
import net.minecraft.resources.ResourceKey;
import net.minecraft.server.level.*;
import net.minecraft.sounds.*;
import net.minecraft.util.Mth;
import net.minecraft.world.damagesource.*;
import net.minecraft.world.entity.*;
import net.minecraft.world.entity.player.Player;
import net.minecraft.world.item.*;
import net.minecraft.world.level.*;
import net.minecraft.world.level.block.*;
import net.minecraft.world.level.block.state.*;
import net.minecraft.world.level.gameevent.GameEvent;
import net.minecraft.world.level.material.*;

/** Dark eco as a liquid (Rust render_anim/src/dark_eco.rs): Jak 1's pool flowing as Overworld lava, poured and scooped by bucket, killing what goes in. */
public final class DarkEco {
    private DarkEco(){}
    public static final FlowingFluid FLOWING=Registry.register(BuiltInRegistries.FLUID,JakData.id("flowing_dark_eco"),new DarkEcoFluid.Flowing());
    public static final FlowingFluid SOURCE=Registry.register(BuiltInRegistries.FLUID,JakData.id("dark_eco"),new DarkEcoFluid.Source());
    private static final ResourceKey<Block> BLOCK_KEY=ResourceKey.create(Registries.BLOCK,JakData.id("dark_eco"));
    public static final Block BLOCK=Registry.register(BuiltInRegistries.BLOCK,BLOCK_KEY,new LiquidBlock(SOURCE,BlockBehaviour.Properties.of().setId(BLOCK_KEY)
        .mapColor(MapColor.COLOR_PURPLE).replaceable().noCollision().strength(100).pushReaction(PushReaction.POPPED).noLootTable().liquid().sound(SoundType.EMPTY)));
    private static final ResourceKey<Item> BUCKET_KEY=ResourceKey.create(Registries.ITEM,JakData.id("dark_eco_bucket"));
    // dark_eco.rs module doc + minecraft_game weapons.rs (DarkEcoBucket stacks to 1).
    public static final Item BUCKET=Registry.register(BuiltInRegistries.ITEM,BUCKET_KEY,new Bucket(SOURCE,new Item.Properties().setId(BUCKET_KEY).craftRemainder(Items.BUCKET).stacksTo(1)));
    public static final ResourceKey<DamageType> DAMAGE=ResourceKey.create(Registries.DAMAGE_TYPE,JakData.id("dark_eco"));
    private static final SoundEvent DEATH_SOUND=SoundEvent.createVariableRangeEvent(JakData.id("sfx/jak1/death_darkeco"));
    /** dark_eco.rs Pool.died: when each player last died in it (game ticks). */
    private static final Map<UUID,Long> DIED=new HashMap<>();
    public static void initialize(){
        ServerTickEvents.END_LEVEL_TICK.register(DarkEco::playersIn);
        CreativeModeTabEvents.modifyOutputEvent(ResourceKey.create(Registries.CREATIVE_MODE_TAB,JakData.id("equipment"))).register(out->out.insertAfter(JakItems.DARK_ECO,BUCKET));
    }
    static DamageSource source(Level level){return new DamageSource(level.registryAccess().lookupOrThrow(Registries.DAMAGE_TYPE).getOrThrow(DAMAGE));}
    /** dark_eco.rs touches: any of a box (blocks, min and max) is in the liquid, by its height in each cell. */
    static boolean touches(Level level,double x0,double y0,double z0,double x1,double y1,double z1){
        var at=new BlockPos.MutableBlockPos();
        for(int x=Mth.floor(x0);x<=Mth.floor(x1);x++)for(int y=Mth.floor(y0);y<=Mth.floor(y1);y++)for(int z=Mth.floor(z0);z<=Mth.floor(z1);z++){
            FluidState fluid=level.getFluidState(at.set(x,y,z));
            if(fluid.getType().isSame(SOURCE)&&y0<y+fluid.getHeight(level,at))return true;
        }
        return false;
    }
    /** render_anim minecraft_world.rs (dark eco block of the frame): the player in it, not riding, dead on the spot (1000, DamageKind::Death). */
    private static void playersIn(ServerLevel level){
        for(ServerPlayer p:List.copyOf(level.players())){
            // stand-in: spectators fly through blocks in Minecraft; the Rust world had none, they're left alone.
            if(!p.isAlive()||p.isSpectator()||p.isPassenger())continue;
            double x=p.getX(),y=p.getY(),z=p.getZ();
            if(!touches(level,x-.3,y,z-.3,x+.3,y+1.6,z+.3))continue;
            p.hurtServer(level,source(level),1000);
            diedInIt(level,p,x,y+.5,z);
        }
    }
    /** dark_eco.rs died_in_it: Jak 1's death in dark eco, once a death (3 s), the death-darkeco sound at feet + 0.5. */
    private static void diedInIt(ServerLevel level,ServerPlayer p,double x,double y,double z){
        long now=level.getGameTime();Long last=DIED.get(p.getUUID());
        if(last!=null&&now-last<=60)return;
        DIED.put(p.getUUID(),now);
        level.playSound(null,x,y,z,DEATH_SOUND,SoundSource.PLAYERS,1,1);
        // The group-dark-eco-death burst there, for everyone who sees it.
        var packet=new dev.jakmc.network.JakPackets.DarkEcoDeath(level.dimension().identifier(),new net.minecraft.world.phys.Vec3(x,y,z));
        for(var watcher:net.fabricmc.fabric.api.networking.v1.PlayerLookup.around(level,new net.minecraft.world.phys.Vec3(x,y,z),96))
            if(net.fabricmc.fabric.api.networking.v1.ServerPlayNetworking.canSend(watcher,packet.type()))
                net.fabricmc.fabric.api.networking.v1.ServerPlayNetworking.send(watcher,packet);
    }
    /** dark_eco.rs module doc: a bucket poured as lava's is (Request::Pour sound item.bucket.empty_lava). */
    static final class Bucket extends BucketItem {
        Bucket(Fluid fluid,Item.Properties properties){super(fluid,properties);}
        @Override protected void playEmptySound(LivingEntity user,LevelAccessor level,BlockPos pos){
            level.playSound(user,pos,SoundEvents.BUCKET_EMPTY_LAVA,SoundSource.BLOCKS,1,1);
            level.gameEvent(user,GameEvent.FLUID_PLACE,pos);
        }
    }
    /** The liquid: dark_eco.rs flow/step_in, which follow LavaFluid in the Overworld. */
    abstract static class DarkEcoFluid extends FlowingFluid {
        @Override public Fluid getFlowing(){return FLOWING;}
        @Override public Fluid getSource(){return SOURCE;}
        @Override public Item getBucket(){return BUCKET;}
        @Override public boolean isSame(Fluid other){return other==SOURCE||other==FLOWING;}
        // dark_eco.rs STEP_SECONDS: LavaFluid.getTickDelay in the Overworld, 30 ticks (1.5 s), no random slowdown.
        @Override public int getTickDelay(LevelReader level){return 30;}
        // dark_eco.rs DROP_OFF: two levels a block, so three blocks out (levels 2, 4, 6).
        @Override protected int getDropOff(LevelReader level){return 2;}
        // LavaFluid.getSlopeFindDistance in the Overworld (dark_eco.rs's own sim set it aside; Minecraft's flow has it).
        @Override protected int getSlopeFindDistance(LevelReader level){return 2;}
        // dark_eco.rs step_in: only poured cells are sources; it never makes new ones.
        @Override protected boolean canConvertToSource(ServerLevel level){return false;}
        // dark_eco.rs Space::Solid "water too: it stops there": neither replaces the other.
        @Override protected boolean canBeReplacedWith(FluidState state,BlockGetter level,BlockPos pos,Fluid other,Direction direction){return false;}
        @Override protected void spreadTo(LevelAccessor level,BlockPos pos,BlockState state,Direction direction,FluidState target){
            FluidState there=level.getFluidState(pos);
            if(!there.isEmpty()&&!there.getType().isSame(this))return;
            super.spreadTo(level,pos,state,direction,target);
        }
        // minecraft_world.rs dark_eco_washes: plants, snow and fire are washed away as lava does; Rust leaves no drops or fizz.
        @Override protected void beforeDestroyingBlock(LevelAccessor level,BlockPos pos,BlockState state){}
        @Override protected float getExplosionResistance(){return 100;}
        @Override protected BlockState createLegacyBlock(FluidState state){return BLOCK.defaultBlockState().setValue(LiquidBlock.LEVEL,getLegacyLevel(state));}
        // dark_eco.rs Request::Scoop: item.bucket.fill_lava.
        @Override public Optional<SoundEvent> getPickupSound(){return Optional.of(SoundEvents.BUCKET_FILL_LAVA);}
        /** minecraft_world.rs (dark eco block): the mobs and Jak's creatures in it are shot for 10000; players go by playersIn's box. */
        @Override protected void entityInside(Level level,BlockPos pos,Entity entity,InsideBlockEffectApplier effects){
            if(level instanceof ServerLevel server&&entity instanceof LivingEntity living&&!(entity instanceof Player)&&living.isAlive())
                living.hurtServer(server,source(server),10_000);
        }
        static final class Flowing extends DarkEcoFluid {
            @Override protected void createFluidStateDefinition(StateDefinition.Builder<Fluid,FluidState> builder){super.createFluidStateDefinition(builder);builder.add(LEVEL);}
            @Override public int getAmount(FluidState state){return state.getValue(LEVEL);}
            @Override public boolean isSource(FluidState state){return false;}
        }
        static final class Source extends DarkEcoFluid {
            @Override public int getAmount(FluidState state){return 8;}
            @Override public boolean isSource(FluidState state){return true;}
        }
    }
}
