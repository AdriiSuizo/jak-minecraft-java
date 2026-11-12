package dev.jakmc.player;

import dev.jakmc.JakData;
import dev.jakmc.content.JakItems;
import java.util.ArrayList;
import java.util.List;
import net.fabricmc.fabric.api.transfer.v1.context.ContainerItemContext;
import net.fabricmc.fabric.api.transfer.v1.item.ItemStorage;
import net.fabricmc.fabric.api.transfer.v1.item.ItemVariant;
import net.fabricmc.fabric.api.transfer.v1.item.PlayerInventoryStorage;
import net.fabricmc.fabric.api.transfer.v1.storage.Storage;
import net.fabricmc.fabric.api.transfer.v1.transaction.Transaction;
import net.minecraft.core.component.DataComponents;
import net.minecraft.core.component.TypedDataComponent;
import net.minecraft.server.level.ServerPlayer;
import net.minecraft.world.item.Item;
import net.minecraft.world.item.ItemStack;
import net.minecraft.world.item.component.ContainerComponent;
import net.minecraft.world.item.component.CustomData;

/** Carried resource stacks and the conversion of legacy fractional light eco. */
public final class JakResources {
    private static final int PRECISION=100_000;
    private static Item item(int kind){return kind==0?JakItems.DARK_ECO:kind==1?JakItems.LIGHT_ECO:JakItems.AMMUNITION.get(kind-2);}
    private static final int LIGHT=1, LIGHT_UNIT=2*PRECISION;
    private static int capacity(int kind){return kind==LIGHT?LIGHT_UNIT:(kind==0?4:1)*PRECISION;}
    private static int kind(ItemStack stack){
        for(int i=0;i<6;i++)if(stack.is(item(i)))return i;
        return -1;
    }
    private static int charge(ItemStack stack,int kind){
        var data=stack.get(DataComponents.CUSTOM_DATA);
        int spent=data==null?stack.getDamageValue():data.copyTag().getIntOr("JakResourceSpent",0);
        return Math.clamp(capacity(kind)-spent,0,capacity(kind));
    }
    public static void migrate(ServerPlayer player){
        if(player.getAttachedOrCreate(JakData.INVENTORY_RESOURCES))return;
        Float[] old=new Float[6];
        old[0]=player.getAttached(JakData.DARK_ECO);old[1]=player.getAttached(JakData.LIGHT_ECO);
        for(int i=0;i<4;i++)old[i+2]=player.getAttached(JakData.AMMO.get(i));
        player.setAttached(JakData.INVENTORY_RESOURCES,true);
        for(int i=0;i<6;i++)if(old[i]!=null&&Float.isFinite(old[i])&&old[i]>0)grant(player,i,old[i]);
    }
    public static void grant(ServerPlayer player,int kind,float amount){
        long points=Math.max(0,Math.round((double)amount*PRECISION));
        if(kind==LIGHT){
            storeLight(player,points);
            return;
        }
        int each=capacity(kind);
        while(points>=each){
            int count=(int)Math.min(64,points/each);
            offer(player,new ItemStack(item(kind),count));points-=(long)count*each;
        }
        if(points>0){
            var partial=new ItemStack(item(kind));
            setSpent(partial,each-(int)points);offer(player,partial);
        }
    }
    private static void storeLight(ServerPlayer player,long points){
        if(points<=0)return;
        points+=player.getAttachedOrCreate(JakData.LEGACY_LIGHT_REMAINDER);
        player.setAttached(JakData.LEGACY_LIGHT_REMAINDER,(int)(points%LIGHT_UNIT));
        long units=points/LIGHT_UNIT;
        while(units>0){
            int count=(int)Math.min(64,units);
            offer(player,new ItemStack(JakItems.LIGHT_ECO,count));
            units-=count;
        }
    }

    private static boolean partialLight(ItemStack stack){
        return stack.is(JakItems.LIGHT_ECO)&&(stack.getMaxStackSize()==1||charge(stack,LIGHT)!=LIGHT_UNIT);
    }

    private static long collectPartialLight(ItemStack stack,int depth){
        if(stack.isEmpty()||depth>16)return 0;
        if(partialLight(stack)){
            long points=(long)charge(stack,LIGHT)*stack.getCount();
            stack.setCount(0);
            return points;
        }
        long points=0;
        for(var component:stack.getComponents()){
            if(!(component.value() instanceof ContainerComponent<?> contents))continue;
            var children=contents.itemCopies().toList();
            boolean changed=false;
            for(var child:children){
                boolean occupied=!child.isEmpty();
                long collected=collectPartialLight(child,depth+1);
                points+=collected;
                changed|=collected>0||occupied&&child.isEmpty();
            }
            if(changed)replace(stack,component,contents.copyWithContents(children.stream()));
        }
        return points;
    }

    private static long collectPartialLight(Storage<ItemVariant> storage){
        long points=0;
        try(var transaction=Transaction.openOuter()){
            for(var view:storage.nonEmptyViews()){
                var variant=view.getResource();
                var stack=variant.toStack();
                if(partialLight(stack))
                    points+=charge(stack,LIGHT)*storage.extract(variant,view.getAmount(),transaction);
            }
            transaction.commit();
        }
        return points;
    }
    private static void setSpent(ItemStack stack,int spent){
        // Fuel is not tool damage: native repair recipes must never regenerate eco/ammunition.
        stack.set(DataComponents.MAX_STACK_SIZE,1);
        stack.remove(DataComponents.MAX_DAMAGE);stack.remove(DataComponents.DAMAGE);
        CustomData.update(DataComponents.CUSTOM_DATA,stack,tag->tag.putInt("JakResourceSpent",spent));
    }
    private static void offer(ServerPlayer player,ItemStack stack){
        if(!player.getInventory().add(stack))player.spawnAtLocation(player.level(),stack);
    }
    private static boolean container(ItemStack stack){
        for(var component:stack.getComponents())if(component.value() instanceof ContainerComponent<?>)return true;
        return false;
    }
    public static float[] totals(ServerPlayer player){
        if(player.hasInfiniteMaterials())return new float[]{100,100,50,100,100,20};
        float[] values=new float[6];
        long legacyLight=0;
        var inventory=player.getInventory();
        var slots=PlayerInventoryStorage.of(player);
        for(int i=0;i<inventory.getContainerSize();i++){
            var stack=inventory.getItem(i);
            legacyLight+=collectPartialLight(stack,0);
            count(stack,values,0);
            if(!stack.isEmpty()&&!container(stack)){
                var storage=ContainerItemContext.ofPlayerSlot(player,slots.getSlot(i)).find(ItemStorage.ITEM);
                if(storage==null||!storage.supportsExtraction())continue;
                legacyLight+=collectPartialLight(storage);
                try(var transaction=Transaction.openOuter()){
                    // One speculative extraction across the whole storage. Independent
                    // rollbacks per view can count the same accessible slot repeatedly
                    // when another matching pocket is protected against extraction.
                    for(var view:storage.nonEmptyViews()){
                        var contents=view.getResource().toStack();int kind=kind(contents);
                        if(kind>=0){
                            long accessible=storage.extract(view.getResource(),view.getAmount(),transaction);
                            values[kind]+=charge(contents,kind)*(double)accessible/PRECISION;
                        }
                    }
                }
            }
        }
        if(legacyLight>0){
            storeLight(player,legacyLight);
            return totals(player);
        }
        return values;
    }
    private static void count(ItemStack stack,float[] values,int depth){
        if(stack.isEmpty()||depth>16)return;
        int kind=kind(stack);
        if(kind>=0){values[kind]+=(double)charge(stack,kind)*stack.getCount()/PRECISION;return;}
        for(var component:stack.getComponents())if(component.value() instanceof ContainerComponent<?> contents)
            contents.itemCopies().forEach(child->count(child,values,depth+1));
    }
    public static void synchronize(ServerPlayer player,float[] values){
        set(player,JakData.DARK_ECO,values[0]);set(player,JakData.LIGHT_ECO,values[1]);
        for(int i=0;i<4;i++)set(player,JakData.AMMO.get(i),values[i+2]);
    }
    private static void set(ServerPlayer player,net.fabricmc.fabric.api.attachment.v1.AttachmentType<Float> type,float value){
        if(player.getAttachedOrCreate(type)!=value)player.setAttached(type,value);
    }
    public static void consume(ServerPlayer player,int kind,float amount){
        if(player.hasInfiniteMaterials())return;
        long needed=Math.max(0,Math.round((double)amount*PRECISION));
        if(kind==LIGHT)needed=Math.round((double)needed/LIGHT_UNIT)*LIGHT_UNIT;
        if(needed==0)return;
        var partials=new ArrayList<ItemStack>();
        var inventory=player.getInventory();
        var slots=PlayerInventoryStorage.of(player);
        for(int i=0;i<inventory.getContainerSize()&&needed>0;i++){
            var stack=inventory.getItem(i);
            needed=consumeStack(stack,kind,needed,partials,0);
            if(needed>0&&!stack.isEmpty()&&!container(stack)){
                var storage=ContainerItemContext.ofPlayerSlot(player,slots.getSlot(i)).find(ItemStorage.ITEM);
                if(storage!=null)needed=consumeStorage(storage,kind,needed,partials);
            }
        }
        // Commit container replacements before offering a residual unit to the outer inventory.
        for(var partial:partials)if(!inventory.add(partial))player.spawnAtLocation(player.level(),partial);
        inventory.setChanged();
    }
    private static long consumeStack(ItemStack stack,int kind,long needed,List<ItemStack> partials,int depth){
        if(stack.isEmpty()||needed==0||depth>16)return needed;
        if(stack.is(item(kind))){
            int each=charge(stack,kind);
            if(each==0){stack.setCount(0);return needed;}
            int whole=(int)Math.min(stack.getCount(),needed/each);
            stack.shrink(whole);needed-=(long)whole*each;
            if(needed>0&&!stack.isEmpty()){
                int used=(int)Math.min(needed,each);
                var partial=stack.copyWithCount(1);
                int spent=capacity(kind)-each+used;
                setSpent(partial,spent);
                if(depth==0&&stack.getCount()==1){setSpent(stack,spent);}
                else{stack.shrink(1);partials.add(partial);}
                needed-=used;
            }
            return needed;
        }
        for(var component:List.copyOf(stack.getComponents().stream().toList())){
            if(!(component.value() instanceof ContainerComponent<?> contents))continue;
            var children=contents.itemCopies().collect(java.util.stream.Collectors.toCollection(ArrayList::new));
            long before=needed;
            for(var child:children)needed=consumeStack(child,kind,needed,partials,depth+1);
            if(needed!=before)replace(stack,component,contents.copyWithContents(children.stream()));
        }
        return needed;
    }
    @SuppressWarnings("unchecked")
    private static <T> void replace(ItemStack stack,TypedDataComponent<T> component,Object value){stack.set(component.type(),(T)value);}
    private static long consumeStorage(Storage<ItemVariant> storage,int kind,long needed,List<ItemStack> partials){
        try(var transaction=Transaction.openOuter()){
            var residuals=new ArrayList<ItemStack>();
            for(var view:storage.nonEmptyViews()){
                var variant=view.getResource();var stack=variant.toStack();
                if(!stack.is(item(kind)))continue;
                int each=charge(stack,kind);if(each==0)continue;
                long count=Math.min(view.getAmount(),(needed+each-1)/each);
                long extracted=storage.extract(variant,count,transaction);
                long paid=Math.min(needed,extracted*each);
                long left=extracted*each-paid;
                if(left>0){
                    setSpent(stack,capacity(kind)-(int)left);residuals.add(stack);
                }
                needed-=paid;if(needed==0)break;
            }
            transaction.commit();partials.addAll(residuals);
        }
        return needed;
    }
}
