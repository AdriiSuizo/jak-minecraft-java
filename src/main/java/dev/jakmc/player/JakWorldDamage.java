package dev.jakmc.player;

import dev.jakmc.content.JakProps;
import java.util.HashMap;
import java.util.Map;
import java.util.WeakHashMap;
import net.minecraft.core.BlockPos;
import net.minecraft.server.level.ServerLevel;
import net.minecraft.server.level.ServerPlayer;
import net.minecraft.world.entity.Entity;
import net.minecraft.world.level.BlockGetter;
import net.minecraft.world.level.Explosion;
import net.minecraft.world.level.ExplosionDamageCalculator;
import net.minecraft.world.level.ServerExplosion;
import net.minecraft.world.level.block.state.BlockState;
import net.minecraft.world.phys.Vec3;

/** World consequences of source simulation events; entity damage stays in the source sim. */
public final class JakWorldDamage {
    private JakWorldDamage(){}
    private static final Map<ServerLevel,Map<BlockPos,Damage>> DAMAGE=new WeakHashMap<>();
    private static int nextCrack=-1_000_000;
    private static final float DAMAGE_TO_MW2=100f/8,DAMAGE_PER_HARDNESS=160;
    private static final class Damage {
        final BlockState state;
        final int crack=nextCrack--;
        float progress;
        long until;
        Damage(BlockState state){this.state=state;}
    }
    public static void tick(ServerLevel level){
        var entries=DAMAGE.get(level);
        if(entries==null)return;
        var it=entries.entrySet().iterator();
        while(it.hasNext()){
            var entry=it.next();var damage=entry.getValue();
            if(damage.until<level.getGameTime()||level.getBlockState(entry.getKey())!=damage.state){
                level.destroyBlockProgress(damage.crack,entry.getKey(),-1);it.remove();
            }
        }
        if(entries.isEmpty())DAMAGE.remove(level);
    }
    private static boolean permitted(ServerPlayer player,ServerLevel level,BlockPos pos){
        return level.hasChunkAt(pos)&&(player==null||!player.isSpectator()
            &&!player.blockActionRestricted(level,pos,player.gameMode.getGameModeForPlayer())&&level.mayInteract(player,pos));
    }
    public static void hit(ServerPlayer player,BlockPos pos,float strength){
        if(!Float.isFinite(strength)||strength<=0)return;
        // Rust mining: the blow in MW2 points (x100/8) over 160 a hardness.
        work(player,pos,strength*DAMAGE_TO_MW2/DAMAGE_PER_HARDNESS,player.hasInfiniteMaterials());
    }
    /** A kick clears plants and opens Krimson ammunition crates. */
    public static void kick(ServerPlayer player,BlockPos pos){
        var level=player.level();
        if(!permitted(player,level,pos))return;
        var block=level.getBlockState(pos).getBlock();
        if(block==dev.jakmc.content.JakAmmoCrate.BLOCK
            ||block instanceof net.minecraft.world.level.block.VegetationBlock
            ||block instanceof net.minecraft.world.level.block.BambooStalkBlock
            ||block instanceof net.minecraft.world.level.block.BambooSaplingBlock
            ||block instanceof net.minecraft.world.level.block.SugarCaneBlock)
            work(player,pos,1,true);
    }
    private static void work(ServerPlayer player,BlockPos pos,float work,boolean whole){
        var level=player.level();
        if(!permitted(player,level,pos))return;
        if(JakProps.hitProjector(level,pos,player))return;
        var state=level.getBlockState(pos);
        float hardness=state.getDestroySpeed(level,pos);
        if(state.isAir()||hardness<0)return;
        var entries=DAMAGE.computeIfAbsent(level,k->new HashMap<>());
        // Expire interrupted work and cap bookkeeping when spraying terrain.
        tick(level);
        entries=DAMAGE.computeIfAbsent(level,k->new HashMap<>());
        if(entries.size()>=256&&!entries.containsKey(pos))return;
        var damage=entries.get(pos);
        if(damage==null||damage.state!=state){damage=new Damage(state);entries.put(pos.immutable(),damage);}
        damage.until=level.getGameTime()+100;
        damage.progress+=whole||hardness==0?1:work/hardness;
        if(damage.progress>=1){
            level.destroyBlockProgress(damage.crack,pos,-1);entries.remove(pos);
            // Minecraft leaves a breaker's particles and sound to his client's own digging, and Jak's blows are the server's.
            if(player.gameMode.destroyBlock(pos))player.connection.send(new net.minecraft.network.protocol.game.ClientboundLevelEventPacket(2001,pos,net.minecraft.world.level.block.Block.getId(state),false));
        }else level.destroyBlockProgress(damage.crack,pos,Math.min(9,(int)(damage.progress*10)));
    }
    /**
     * The Super Nova's blast on the world: a big TNT one (Adrii 06/11: "como una nuke"). stand-in: the
     * terrain power controls the crater. The weapon's own radius bounds entity damage and knockback.
     */
    public static final float NUKE_POWER=16;
    public static void explosion(ServerLevel level,Entity source,ServerPlayer owner,Vec3 center,float power){
        explosion(level,source,owner,center,power,0);
    }
    static void explosion(ServerLevel level,Entity source,ServerPlayer owner,Vec3 center,float power,float entityRadius){
        if(!Float.isFinite(power)||power<=0||!Double.isFinite(center.lengthSqr()))return;
        boolean nuke=entityRadius>0&&Float.isFinite(entityRadius);
        var calculator=new ExplosionDamageCalculator(){
            private boolean inBlast(Entity entity){
                if(!nuke||entity==owner||entity==owner.getVehicle())return false;
                var box=entity.getBoundingBox();
                double x=center.x-Math.clamp(center.x,box.minX,box.maxX);
                double y=center.y-Math.clamp(center.y,box.minY,box.maxY);
                double z=center.z-Math.clamp(center.z,box.minZ,box.maxZ);
                return x*x+y*y+z*z<=entityRadius*entityRadius;
            }
            @Override public boolean shouldDamageEntity(Explosion explosion,Entity entity){return inBlast(entity);}
            @Override public float getKnockbackMultiplier(Entity entity){return inBlast(entity)?1:0;}
            @Override public boolean shouldBlockExplode(Explosion explosion,BlockGetter world,BlockPos pos,BlockState state,float energy){
                return permitted(owner,level,pos);
            }
        };
        // Native resistance, fluid shielding, block callbacks and drops. Original
        // particle/sound streams already present this blast; don't play it twice.
        if(nuke){new ServerExplosion(level,source,null,calculator,center,NUKE_POWER,false,Explosion.BlockInteraction.DESTROY_WITH_DECAY).explode();return;}
        new ServerExplosion(level,source,null,calculator,center,Math.min(9,power),false,Explosion.BlockInteraction.DESTROY).explode();
    }
}
