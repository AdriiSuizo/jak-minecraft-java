package dev.jakmc.content;

import dev.jakmc.JakData;
import dev.jakmc.player.JakPlayers;
import java.util.HashSet;
import java.util.Set;
import java.util.UUID;
import net.fabricmc.fabric.api.creativetab.v1.FabricCreativeModeTab;
import net.minecraft.core.BlockPos;
import net.minecraft.core.Direction;
import net.minecraft.core.Registry;
import net.minecraft.core.registries.BuiltInRegistries;
import net.minecraft.core.registries.Registries;
import net.minecraft.network.chat.Component;
import net.minecraft.resources.ResourceKey;
import net.minecraft.server.level.ServerLevel;
import net.minecraft.server.level.ServerPlayer;
import net.minecraft.sounds.SoundEvent;
import net.minecraft.sounds.SoundSource;
import net.minecraft.world.InteractionResult;
import net.minecraft.world.effect.MobEffectInstance;
import net.minecraft.world.effect.MobEffects;
import net.minecraft.world.entity.LivingEntity;
import net.minecraft.world.entity.player.Player;
import net.minecraft.world.item.BlockItem;
import net.minecraft.world.item.Item;
import net.minecraft.world.item.ItemStack;
import net.minecraft.world.level.BlockGetter;
import net.minecraft.world.level.Level;
import net.minecraft.world.level.LevelReader;
import net.minecraft.world.level.ScheduledTickAccess;
import net.minecraft.util.RandomSource;
import net.minecraft.world.level.block.Block;
import net.minecraft.world.level.block.EntityBlock;
import net.minecraft.world.level.block.Blocks;
import net.minecraft.world.level.block.entity.BlockEntity;
import net.minecraft.world.level.block.entity.BlockEntityTicker;
import net.minecraft.world.level.block.entity.BlockEntityType;
import net.minecraft.world.level.block.state.BlockState;
import net.minecraft.world.level.block.state.StateDefinition;
import net.minecraft.world.level.block.state.properties.BooleanProperty;
import net.minecraft.world.level.storage.ValueInput;
import net.minecraft.world.level.storage.ValueOutput;
import net.minecraft.world.phys.AABB;
import net.minecraft.world.phys.BlockHitResult;
import net.minecraft.world.phys.Vec3;
import net.minecraft.world.phys.shapes.CollisionContext;
import net.minecraft.world.phys.shapes.Shapes;
import net.minecraft.world.phys.shapes.VoxelShape;

/** Minecraft owns placement and persistence; the six sources keep Jak's charge and replenishment rules. */
public final class JakEcoVent extends Block implements EntityBlock {
    public enum Color {
        RED, YELLOW, BLUE, GREEN, DARK, LIGHT;
        public JakEcoVent block;
        public Item item;
        public String path(){return name().toLowerCase(java.util.Locale.ROOT);}
        public boolean model(){return this==DARK||this==LIGHT;}
        public String ambient(){return (model()?"jak3:":"jak1:")+"eco-bg-"+path();}
        SoundEvent pickup(){return sound(model()?"jak3":"jak1",model()?path()+"-charge":"get-"+path()+"-eco");}
    }
    public static BlockEntityType<Vent> TYPE;
    public static final BooleanProperty OPEN=BooleanProperty.create("open"), BREAKING=BooleanProperty.create("breaking"),
        TOUCHED=BooleanProperty.create("touched");
    private static final VoxelShape TOP=Shapes.or(Block.box(.4,0,.4,15.6,1.1,15.6),
        Block.box(2,1.1,2,14,2.1,14),Block.box(3.25,2.1,3.25,12.75,3.25,12.75));
    private static final VoxelShape NOZZLE=Block.box(5,0,5,11,1.5,11);
    private final Color color;
    private JakEcoVent(Color color,Properties properties){
        super(properties);this.color=color;
        registerDefaultState(stateDefinition.any().setValue(OPEN,true).setValue(BREAKING,false).setValue(TOUCHED,false));
    }
    public static void initialize(){
        var blocks=new HashSet<Block>();
        for(var color:Color.values()){
            var id=JakData.id("vent_"+color.path());
            var key=ResourceKey.create(Registries.BLOCK,id);
            color.block=Registry.register(BuiltInRegistries.BLOCK,key,new JakEcoVent(color,
                Properties.of().setId(key).strength(1.5f,6).noOcclusion().requiresCorrectToolForDrops()
                    .pushReaction(net.minecraft.world.level.material.PushReaction.IMMOVEABLE)
                    .lightLevel(state->state.getValue(OPEN)&&!state.getValue(BREAKING)?color==Color.LIGHT?13:7:0)));
            var item=ResourceKey.create(Registries.ITEM,id);
            color.item=Registry.register(BuiltInRegistries.ITEM,item,new BlockItem(color.block,new Item.Properties().setId(item).useBlockDescriptionPrefix()));
            blocks.add(color.block);
        }
        TYPE=Registry.register(BuiltInRegistries.BLOCK_ENTITY_TYPE,JakData.id("eco_vent"),new BlockEntityType<>(Vent::new,blocks));
        Registry.register(BuiltInRegistries.CREATIVE_MODE_TAB,JakData.id("eco_vents"),
            FabricCreativeModeTab.builder().title(Component.translatable("tab.jakmc.eco_vents")).icon(()->new ItemStack(Color.LIGHT.item))
                .displayItems((context,out)->{for(var color:Color.values())out.accept(color.item);}).build());
    }
    @Override protected void createBlockStateDefinition(StateDefinition.Builder<Block,BlockState> states){states.add(OPEN,BREAKING,TOUCHED);}
    @Override protected VoxelShape getShape(BlockState state,BlockGetter level,BlockPos at,CollisionContext context){return color.model()?TOP:NOZZLE;}
    @Override protected VoxelShape getCollisionShape(BlockState state,BlockGetter level,BlockPos at,CollisionContext context){
        return color.model()?TOP:Shapes.empty();
    }
    @Override protected boolean canSurvive(BlockState state,LevelReader level,BlockPos at){
        var floor=at.below();
        return level.getBlockState(floor).isFaceSturdy(level,floor,Direction.UP);
    }
    @Override protected BlockState updateShape(BlockState state,LevelReader level,ScheduledTickAccess ticks,
            BlockPos at,Direction direction,BlockPos neighbor,BlockState neighborState,RandomSource random){
        return direction==Direction.DOWN&&!canSurvive(state,level,at)?Blocks.AIR.defaultBlockState():state;
    }
    @Override public BlockEntity newBlockEntity(BlockPos at,BlockState state){return new Vent(at,state);}
    @Override public <T extends BlockEntity> BlockEntityTicker<T> getTicker(Level level,BlockState state,BlockEntityType<T> type){
        return level.isClientSide()||type!=TYPE?null:(world,pos,block,entity)->((Vent)entity).tick((ServerLevel)world);
    }
    @Override protected InteractionResult useWithoutItem(BlockState state,Level level,BlockPos at,Player player,BlockHitResult hit){
        if(!level.mayInteract(player,at)||state.getValue(BREAKING))return InteractionResult.PASS;
        if(!level.isClientSide()){
            boolean open=!state.getValue(OPEN);
            level.setBlockAndUpdate(at,state.setValue(OPEN,open).setValue(TOUCHED,false));
            level.blockEvent(at,this,1,open?1:0);
        }
        return InteractionResult.SUCCESS;
    }
    @Override protected boolean triggerEvent(BlockState state,Level level,BlockPos at,int event,int value){
        return level.getBlockEntity(at) instanceof Vent vent&&vent.triggerEvent(event,value);
    }
    private static SoundEvent sound(String game,String name){
        return SoundEvent.createVariableRangeEvent(JakData.id("sfx/"+game+"/"+name.replace('-','_')));
    }
    public static void groundPound(ServerLevel level,ServerPlayer player,Vec3 impact){
        BlockPos at=BlockPos.containing(impact);
        for(var point:BlockPos.betweenClosed(at.offset(-1,-1,-1),at.offset(1,1,1))){
            if(!level.hasChunkAt(point)||!level.mayInteract(player,point)||!(level.getBlockEntity(point) instanceof Vent vent))continue;
            if(Vec3.atBottomCenterOf(point).distanceToSqr(impact)<=1.1)vent.pound(level);
        }
    }

    public static final class Vent extends BlockEntity {
        private static final int CAPACITY=16, PRODUCE_TICKS=300;
        private final Set<UUID> touching=new HashSet<>();
        private final Set<UUID> accepted=new HashSet<>();
        private float stock=CAPACITY;
        private int replenish,cracks,brokenFor;
        private long transition=Long.MIN_VALUE;
        public int pickupEntity=-1;
        public long pickupTime=Long.MIN_VALUE;
        Vent(BlockPos at,BlockState state){super(TYPE,at,state);}
        public Color color(){return ((JakEcoVent)getBlockState().getBlock()).color;}
        public boolean open(){return getBlockState().getValue(OPEN);}
        public boolean touched(){return getBlockState().getValue(TOUCHED);}
        public boolean breaking(){return getBlockState().getValue(BREAKING);}
        public float transition(float partial){
            return level==null||transition==Long.MIN_VALUE?Float.POSITIVE_INFINITY:(level.getGameTime()-transition+partial)/20f;
        }
        @Override public boolean triggerEvent(int event,int value){
            if(level==null)return false;
            if(event==2){pickupEntity=value;pickupTime=level.getGameTime();return true;}
            if(event!=1)return false;
            transition=level.getGameTime();
            return true;
        }
        private void pound(ServerLevel level){
            if(breaking())return;
            if(++cracks==3){
                level.setBlockAndUpdate(worldPosition,getBlockState().setValue(OPEN,false).setValue(BREAKING,true).setValue(TOUCHED,false));
                level.blockEvent(worldPosition,getBlockState().getBlock(),1,0);
            }else level.levelEvent(2001,worldPosition,Block.getId(getBlockState()));
            setChanged();
        }
        private void tick(ServerLevel level){
            if(breaking()){
                if(++brokenFor>=24)level.removeBlock(worldPosition,false);
                return;
            }
            if(!open()){touching.clear();accepted.clear();return;}
            if(stock<CAPACITY){
                if(++replenish>=PRODUCE_TICKS){stock=Math.min(CAPACITY,stock+1);replenish=0;}
                if(replenish%20==0)setChanged();
            }
            if((level.getGameTime()&1)!=0)return;
            var center=Vec3.atBottomCenterOf(worldPosition);
            double radius=color().model()?.38:.52;
            var space=new AABB(center.x-radius,center.y-.05,center.z-radius,center.x+radius,center.y+1.15,center.z+radius);
            var entities=level.getEntitiesOfClass(LivingEntity.class,space,actor->actor.isAlive()&&!actor.isSpectator()
                &&actor.position().subtract(center).horizontalDistanceSqr()<=radius*radius
                &&actor.getY()>=center.y-.05&&actor.getY()<=center.y+1.15);
            var contacts=new HashSet<UUID>();
            for(var actor:entities){
                contacts.add(actor.getUUID());
                boolean first=!touching.contains(actor.getUUID());
                if(accept(level,actor,first)){
                    level.playSound(null,worldPosition,color().pickup(),SoundSource.BLOCKS,1,1);
                    if(!color().model())level.blockEvent(worldPosition,getBlockState().getBlock(),2,actor.getId());
                    if(actor instanceof ServerPlayer player)JakPlayers.ventPulse(player,false,color()==Color.LIGHT?2:1);
                }
            }
            touching.retainAll(contacts);touching.addAll(contacts);accepted.retainAll(contacts);
            if(color().model()&&touched()!=!contacts.isEmpty())
                level.setBlock(worldPosition,getBlockState().setValue(TOUCHED,!contacts.isEmpty()),UPDATE_CLIENTS);
        }
        private boolean accept(ServerLevel level,LivingEntity actor,boolean first){
            Color type=color();
            boolean regular=first||level.getGameTime()%20==0;
            boolean guest=actor instanceof ServerPlayer p&&JakPlayers.active(p);
            if(type==Color.GREEN){
                if(regular&&actor.getHealth()<actor.getMaxHealth()){actor.setHealth(actor.getMaxHealth());return true;}
                return false;
            }
            if(type==Color.BLUE&&regular){
                if(guest)JakPlayers.ventPulse((ServerPlayer)actor,true,0);
                else actor.addEffect(new MobEffectInstance(MobEffects.SPEED,400,0,false,false));
            }
            if(!guest){
                if(type==Color.DARK&&regular&&!(actor instanceof net.minecraft.world.entity.monster.Blaze))
                    actor.hurtServer(level,actor.damageSources().magic(),5);
                return false;
            }
            var player=(ServerPlayer)actor;
            if(stock==0)return false;
            if(accepted.contains(actor.getUUID())||!regular)return false;
            Item item=type==Color.LIGHT?JakItems.LIGHT_ECO:type==Color.DARK?JakItems.DARK_ECO:JakItems.AMMUNITION.get(type.ordinal());
            int count=Math.min(5,(int)stock);
            var stack=new ItemStack(item,count);
            player.getInventory().add(stack);
            int accepted=count-stack.getCount();
            if(accepted==0)return false;
            stock-=accepted;this.accepted.add(actor.getUUID());setChanged();return true;
        }
        @Override protected void saveAdditional(ValueOutput out){
            super.saveAdditional(out);
            out.putFloat("stock",stock);out.putInt("replenish",replenish);
            out.putInt("cracks",cracks);out.putInt("brokenFor",brokenFor);
        }
        @Override protected void loadAdditional(ValueInput in){
            super.loadAdditional(in);
            stock=Math.clamp(in.getFloatOr("stock",CAPACITY),0,CAPACITY);
            replenish=Math.clamp(in.getIntOr("replenish",0),0,PRODUCE_TICKS-1);
            cracks=Math.clamp(in.getIntOr("cracks",0),0,3);brokenFor=Math.clamp(in.getIntOr("brokenFor",0),0,24);
        }
    }
}
