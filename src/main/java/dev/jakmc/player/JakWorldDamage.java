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
        var level=player.level();
        if(!Float.isFinite(strength)||strength<=0||!permitted(player,level,pos))return;
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
        damage.progress+=player.hasInfiniteMaterials()||hardness==0?1:
            Math.max(state.getDestroyProgress(player,level,pos),strength/(20*Math.max(.1f,hardness)));
        if(damage.progress>=1){
            level.destroyBlockProgress(damage.crack,pos,-1);entries.remove(pos);
            player.gameMode.destroyBlock(pos);
        }else level.destroyBlockProgress(damage.crack,pos,Math.min(9,(int)(damage.progress*10)));
    }
    public static void explosion(ServerLevel level,Entity source,ServerPlayer owner,Vec3 center,float power){
        if(!Float.isFinite(power)||power<=0||!Double.isFinite(center.lengthSqr()))return;
        var calculator=new ExplosionDamageCalculator(){
            @Override public boolean shouldDamageEntity(Explosion explosion,Entity entity){return false;}
            @Override public float getKnockbackMultiplier(Entity entity){return 0;}
            @Override public boolean shouldBlockExplode(Explosion explosion,BlockGetter world,BlockPos pos,BlockState state,float energy){
                return permitted(owner,level,pos);
            }
        };
        // Native resistance, fluid shielding, block callbacks and drops. Original
        // particle/sound streams already present this blast; don't play it twice.
        new ServerExplosion(level,source,null,calculator,center,Math.min(9,power),false,Explosion.BlockInteraction.DESTROY).explode();
    }
}
