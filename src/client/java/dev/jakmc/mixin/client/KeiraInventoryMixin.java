package dev.jakmc.mixin.client;

import dev.jakmc.client.KeiraBook;
import net.minecraft.client.Minecraft;
import net.minecraft.client.gui.components.Button;
import net.minecraft.client.gui.screens.inventory.InventoryScreen;
import net.minecraft.network.chat.Component;
import org.spongepowered.asm.mixin.Mixin;
import org.spongepowered.asm.mixin.Unique;
import org.spongepowered.asm.mixin.injection.At;
import org.spongepowered.asm.mixin.injection.Inject;
import org.spongepowered.asm.mixin.injection.callback.CallbackInfo;

@Mixin(InventoryScreen.class)
abstract class KeiraInventoryMixin extends net.minecraft.client.gui.screens.Screen {
    @Unique private Button jakmc$notebook;

    protected KeiraInventoryMixin(Component title){super(title);}

    @Inject(method="init",at=@At("TAIL"))
    private void jakmc$addNotebook(CallbackInfo ci){
        var inventory=(InventoryScreen)(Object)this;
        var title=KeiraBook.title();
        int buttonWidth=Minecraft.getInstance().font.width(title)+16;
        jakmc$notebook=addRenderableWidget(Button.builder(title,
            button->Minecraft.getInstance().gui.setScreen(new KeiraBook(inventory))).size(buttonWidth,20).build());
        jakmc$moveNotebook();
    }

    @Inject(method="onRecipeBookButtonClick",at=@At("TAIL"))
    private void jakmc$followInventory(CallbackInfo ci){jakmc$moveNotebook();}

    @Unique private void jakmc$moveNotebook(){
        if(jakmc$notebook==null)return;
        var bounds=(JakContainerBounds)this;
        int bottom=bounds.jakmc$top()+169;
        int y=bottom+20<height?bottom:Math.max(1,bounds.jakmc$top()-23);
        int x=Math.clamp(bounds.jakmc$left()+88-jakmc$notebook.getWidth()/2,0,Math.max(0,width-jakmc$notebook.getWidth()));
        jakmc$notebook.setPosition(x,y);
    }
}
