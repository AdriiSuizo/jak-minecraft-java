package dev.jakmc.mixin;
import dev.jakmc.content.*;
import net.minecraft.world.entity.item.ItemEntity;
import net.minecraft.world.phys.*;
import net.minecraft.server.level.ServerPlayer;
import net.minecraft.world.level.storage.*;
import org.spongepowered.asm.mixin.*;
import org.spongepowered.asm.mixin.injection.*;
import org.spongepowered.asm.mixin.injection.callback.*;
import java.util.UUID;
@Mixin(ItemEntity.class)
abstract class GemPhysicsMixin implements GemState {
 @Unique private static final net.minecraft.network.syncher.EntityDataAccessor<org.joml.Vector3fc> JAK_GEM_MOTION=net.minecraft.network.syncher.SynchedEntityData.defineId(ItemEntity.class,net.minecraft.network.syncher.EntityDataSerializers.VECTOR3);
 @Unique private org.joml.Vector3fc jakmc$sample;@Unique private int jakmc$sampleTick;
 @Inject(method="defineSynchedData",at=@At("TAIL"))
 private void jakmc$define(net.minecraft.network.syncher.SynchedEntityData.Builder builder,CallbackInfo ci){builder.define(JAK_GEM_MOTION,new org.joml.Vector3f(-1,0,110));}
 public float gemAngle(float partial){var item=(ItemEntity)(Object)this;var value=item.getEntityData().get(JAK_GEM_MOTION);if(value!=jakmc$sample){jakmc$sample=value;jakmc$sampleTick=item.tickCount;}return value.y()+value.z()*Math.clamp((item.tickCount+partial-jakmc$sampleTick)/20f,0,.2f);}
 @Shadow private UUID target;
 @Unique private GemMotion jakmc$motion;
 @Unique private UUID jakmc$sucking;
 @Unique private GemMotion jakmc$motion(){if(jakmc$motion==null){var item=(ItemEntity)(Object)this;jakmc$motion=new GemMotion(item.level().getRandom().nextDouble(),item.level().getRandom().nextBoolean()?1:-1);}return jakmc$motion;}
 @Inject(method="getDefaultGravity",at=@At("HEAD"),cancellable=true)
 private void jakmc$gravity(CallbackInfoReturnable<Double> ci){var item=(ItemEntity)(Object)this;if(item.getItem().is(JakItems.SKULL_GEM))ci.setReturnValue(30*GemPhysics.UNIT/400);}
 @Inject(method="tick",at=@At("HEAD"))
 private void jakmc$begin(CallbackInfo ci){var item=(ItemEntity)(Object)this;if(!item.getItem().is(JakItems.SKULL_GEM))return;var m=jakmc$motion();m.tick();if(item.level().isClientSide())return;item.setYRot((float)m.angle);item.getEntityData().set(JAK_GEM_MOTION,new org.joml.Vector3f((float)m.seconds,(float)m.angle,(float)m.spin));
  if(item.getAge()==0){GemPhysics.sound(item,"gem_spawn",1);GemPhysics.spawnCues++;}
  if(item.hasPickUpDelay()||item.isInWater()||item.isInLava())return;
  // collectables.gc: once sucked in (`suck-in`) it keeps going for its collector at any distance (Rust entity.rs
  // motion.suction.is_some() ||), losing them only for a wall between, a full inventory or their going.
  ServerPlayer best=null;double distance=7.5*GemPhysics.UNIT;for(var candidate:item.level().players())if(candidate instanceof ServerPlayer p&&GemPhysics.accepts(p)&&(target==null||target.equals(p.getUUID()))){Vec3 at=p.position().add(0,.81,0);double d=item.position().distanceTo(at);if(p.getUUID().equals(jakmc$sucking))d=Math.min(d,0);if(d<distance&&item.level().clip(new net.minecraft.world.level.ClipContext(item.position().add(0,.125,0),at,net.minecraft.world.level.ClipContext.Block.COLLIDER,net.minecraft.world.level.ClipContext.Fluid.NONE,item)).getType()==HitResult.Type.MISS){distance=d;best=p;}}
  if(best==null){jakmc$sucking=null;m.suckSeconds=m.speed=m.turn=0;return;}
  if(!best.getUUID().equals(jakmc$sucking)){GemPhysics.sound(item,"pickup_suck",1);GemPhysics.suckCues++;jakmc$sucking=best.getUUID();m.suckSeconds=m.speed=m.turn=0;}
  var p=item.position();var q=best.position().add(0,.81,0);double[] delta=m.suck(new double[]{p.x,p.y,p.z},new double[]{q.x,q.y,q.z},GemPhysics.UNIT);item.setDeltaMovement(delta[0],delta[1]+item.getGravity(),delta[2]);item.needsSync=true;
 }
 /** Three native 60 Hz collision sweeps within one Minecraft tick. Single 20 Hz
  * gravity steps impose a minimum 1.5 m/s impact and never reach GOAL's .3 m/s settle. */
 @Redirect(method="tick",at=@At(value="INVOKE",target="Lnet/minecraft/world/entity/item/ItemEntity;move(Lnet/minecraft/world/entity/MoverType;Lnet/minecraft/world/phys/Vec3;)V"))
 private void jakmc$move(ItemEntity item,net.minecraft.world.entity.MoverType type,Vec3 requested){
  if(!item.getItem().is(JakItems.SKULL_GEM)||item.isInWater()||item.isInLava()){item.move(type,requested);return;}
  var m=jakmc$motion();double gravity=item.getGravity();boolean homing=jakmc$sucking!=null;Vec3 velocity=homing?requested:requested.add(0,gravity,0);
  for(int step=0;step<3;step++){
   if(!homing)velocity=velocity.add(0,-gravity/3,0);Vec3 from=item.position(),delta=velocity.scale(1.0/3);item.setDeltaMovement(velocity);item.move(type,delta);Vec3 actual=item.position().subtract(from);boolean[] axes={Math.abs(actual.x-delta.x)>1e-7,Math.abs(actual.y-delta.y)>1e-7,Math.abs(actual.z-delta.z)>1e-7};
   if(axes[0]||axes[1]||axes[2]){
    if(m.settled){velocity=Vec3.ZERO;break;}
    int old=m.bounces;double[] bounced=m.bounce(new double[]{velocity.x,velocity.y,velocity.z},axes,item.level().getRandom().nextDouble(),item.level().getRandom().nextDouble(),GemPhysics.UNIT);velocity=new Vec3(bounced[0],bounced[1],bounced[2]);
    if(!item.level().isClientSide()){item.needsSync=true;if(m.bounces!=old){GemPhysics.sound(item,"gem_bounce",(float)(.5+.5*Math.min(1,requested.length()*20/GemPhysics.UNIT/10)));GemPhysics.bounceCues++;}}
    if(m.settled)break;
   }
  }item.setDeltaMovement(velocity);
 }
 @Inject(method="addAdditionalSaveData",at=@At("TAIL"))
 private void jakmc$save(ValueOutput out,CallbackInfo ci){if(jakmc$motion!=null){var m=jakmc$motion;out.putDouble("JakGemSeconds",m.seconds);
   // Rust saves the suction too (speed, turn, time and whom).
   if(jakmc$sucking!=null){out.putString("JakGemSucking",jakmc$sucking.toString());out.putDouble("JakGemSuck",m.suckSeconds);out.putDouble("JakGemSpeed",m.speed);out.putDouble("JakGemTurn",m.turn);}out.putDouble("JakGemAngle",m.angle);out.putDouble("JakGemSpin",m.spin);out.putBoolean("JakGemSettled",m.settled);}}
 @Inject(method="readAdditionalSaveData",at=@At("TAIL"))
 private void jakmc$load(ValueInput in,CallbackInfo ci){if(((ItemEntity)(Object)this).getItem().is(JakItems.SKULL_GEM)){var m=jakmc$motion();m.seconds=in.getDoubleOr("JakGemSeconds",0);m.angle=in.getDoubleOr("JakGemAngle",0);m.spin=in.getDoubleOr("JakGemSpin",1024);m.settled=in.getBooleanOr("JakGemSettled",false);
   var who=in.getStringOr("JakGemSucking","");if(!who.isEmpty())try{jakmc$sucking=UUID.fromString(who);m.suckSeconds=in.getDoubleOr("JakGemSuck",0);m.speed=in.getDoubleOr("JakGemSpeed",0);m.turn=in.getDoubleOr("JakGemTurn",0);}catch(IllegalArgumentException e){jakmc$sucking=null;}}}
}
