package dev.jakmc.mixin.client;
import dev.jakmc.client.WeaponScene;
import net.minecraft.client.Camera;
import net.minecraft.world.phys.Vec3;
import org.spongepowered.asm.mixin.*;
import org.spongepowered.asm.mixin.injection.*;
import org.spongepowered.asm.mixin.injection.callback.CallbackInfo;
/** Only the bounded developer scene; ordinary gameplay camera is untouched. */
@Mixin(Camera.class)
abstract class DevSceneCameraMixin {
 @Shadow protected abstract void setPosition(Vec3 position);
 @Shadow protected abstract void setRotation(float yaw,float pitch);
 @Inject(method="alignWithEntity",at=@At("RETURN"))
 private void jakmc$sceneCamera(float partialTicks,CallbackInfo ci){var at=WeaponScene.cameraPosition();if(at!=null){setPosition(at);setRotation(45,12);}else{var input=dev.jakmc.client.GemScene.camera();if(input==null)input=dev.jakmc.client.CitizenScene.camera();if(input==null)input=dev.jakmc.client.InputScene.camera();if(input!=null){setPosition(input.position());setRotation(input.yaw(),input.pitch());}}}
}
