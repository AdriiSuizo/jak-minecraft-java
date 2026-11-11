package dev.jakmc.client;

import com.google.gson.JsonElement;
import com.google.gson.JsonParser;
import java.io.InputStreamReader;
import java.nio.charset.StandardCharsets;
import java.util.ArrayList;
import java.util.LinkedHashMap;
import java.util.List;
import net.minecraft.ChatFormatting;
import net.minecraft.client.Minecraft;
import net.minecraft.client.gui.screens.Screen;
import net.minecraft.client.gui.screens.inventory.BookViewScreen;
import net.minecraft.core.registries.BuiltInRegistries;
import net.minecraft.network.chat.ClickEvent;
import net.minecraft.network.chat.Component;
import net.minecraft.network.chat.MutableComponent;
import net.minecraft.resources.Identifier;
import net.minecraft.world.item.ItemStack;

/** Keira's notebook, with the actual patterns and amounts from Jak's built-in recipes. */
public final class KeiraBook extends BookViewScreen {
    private static final String KEY="book.jakmc.keira.";
    private record Chapter(String name,List<String> recipes) {}
    private static final List<Chapter> CHAPTERS=List.of(
        new Chapter("guns",List.of("scatter_gun","wave_concussor","plasmite_rpg","blaster","beam_reflexor",
            "gyro_burster","vulcan_fury","arc_wielder","needle_lazer","peace_maker","mass_inverter","super_nova")),
        new Chapter("ammo",List.of("red_ammo","red_ammo_from_redstone","red_ammo_from_gunpowder","yellow_ammo",
            "yellow_ammo_from_glowstone","yellow_ammo_from_gold","yellow_ammo_from_gunpowder","blue_ammo",
            "blue_ammo_from_lapis","blue_ammo_from_gunpowder","dark_ammo","dark_ammo_from_tnt","dark_eco","dark_eco_bucket")),
        new Chapter("tools",List.of("jet_board","grind_rail","eco_launcher","iris_door","airlock_door","hut_lamp",
            "vent_blue","vent_red","vent_yellow","vent_green","vent_dark","vent_light")),
        new Chapter("vehicles",List.of("zoomer_bike_a","zoomer_bike_b","zoomer_bike_c","zoomer_car_a","zoomer_car_b",
            "zoomer_car_c","dune_hopper","sand_shark","hellcat","helldog")));
    private final Screen inventory;

    public KeiraBook(Screen inventory){
        super(new BookAccess(pages()));
        this.inventory=inventory;
    }

    @Override public void onClose(){Minecraft.getInstance().gui.setScreen(inventory);}
    @Override public boolean isPauseScreen(){return false;}
    @Override public boolean mouseScrolled(double x,double y,double horizontal,double vertical){
        if(vertical>0)pageBack();
        if(vertical<0)pageForward();
        return vertical!=0;
    }

    public static Component title(){return Component.translatable(KEY+"title");}

    private static List<Component> pages(){
        List<Component> pages=new ArrayList<>();
        pages.add(Component.empty());
        pages.add(Component.empty());
        MutableComponent index=Component.translatable(KEY+"index").withStyle(ChatFormatting.BOLD).append("\n\n");
        for(Chapter chapter:CHAPTERS){
            int number=pages.size()+1;
            Component heading=Component.translatable(KEY+chapter.name());
            index.append(heading.copy().withStyle(style->style.withColor(ChatFormatting.DARK_BLUE)
                .withUnderlined(true).withClickEvent(new ClickEvent.ChangePage(number)))).append("\n");
            pages.add(Component.empty().append(heading.copy().withStyle(ChatFormatting.BOLD)).append("\n\n")
                .append(Component.translatable(KEY+chapter.name()+".intro")));
            for(String recipe:chapter.recipes())addRecipe(pages,recipe);
        }
        pages.set(0,Component.empty().append(title().copy().withStyle(ChatFormatting.BOLD)).append("\n\n")
            .append(Component.translatable(KEY+"intro")));
        pages.set(1,index);
        return pages;
    }

    private static void addRecipe(List<Component> pages,String id){
        try(var stream=KeiraBook.class.getResourceAsStream("/data/jakmc/recipe/"+id+".json")){
            if(stream==null)throw new IllegalStateException("Missing recipe "+id);
            var recipe=JsonParser.parseReader(new InputStreamReader(stream,StandardCharsets.UTF_8)).getAsJsonObject();
            var result=recipe.getAsJsonObject("result");
            Component heading=name(result.get("id").getAsString());
            int count=result.has("count")?result.get("count").getAsInt():1;
            var lines=new ArrayList<Component>();
            lines.add(heading.copy().withStyle(ChatFormatting.BOLD));
            if(count!=1)lines.add(Component.translatable(KEY+"makes",count));
            lines.add(Component.empty());
            if(recipe.has("pattern")){
                var symbols=new LinkedHashMap<Character,Component>();
                for(var entry:recipe.getAsJsonObject("key").entrySet())symbols.put(entry.getKey().charAt(0),ingredient(entry.getValue()));
                for(var row:recipe.getAsJsonArray("pattern")){
                    var formatted=new StringBuilder();
                    for(char symbol:row.getAsString().toCharArray())formatted.append(symbol==' '?'·':symbol).append("   ");
                    lines.add(Component.literal(formatted.toString()).withStyle(ChatFormatting.DARK_BLUE));
                }
                lines.add(Component.empty());
                symbols.forEach((symbol,ingredient)->lines.add(Component.literal(symbol+"  ").append(ingredient)));
            }else if(recipe.has("ingredients")){
                var quantities=new LinkedHashMap<JsonElement,Integer>();
                recipe.getAsJsonArray("ingredients").forEach(item->quantities.merge(item,1,Integer::sum));
                lines.add(Component.translatable(KEY+"any_order"));
                quantities.forEach((item,quantity)->lines.add(Component.literal(quantity+" × ").append(ingredient(item))));
            }else if(recipe.has("ingredient")){
                lines.add(Component.translatable(KEY+"stonecutter"));
                lines.add(ingredient(recipe.get("ingredient")));
            }
            addPages(pages,heading,lines);
        }catch(java.io.IOException e){throw new java.io.UncheckedIOException(e);}
    }

    private static void addPages(List<Component> pages,Component title,List<Component> entries){
        var font=Minecraft.getInstance().font;
        MutableComponent page=Component.empty();
        int lines=0;
        for(Component entry:entries){
            int height=Math.max(1,font.split(entry,114).size());
            if(lines+height>14){
                pages.add(page);
                page=Component.empty().append(title.copy().withStyle(ChatFormatting.BOLD)).append("\n");
                lines=Math.max(1,font.split(title,114).size())+1;
            }
            if(lines!=0)page.append("\n");
            page.append(entry);
            lines+=height;
        }
        pages.add(page);
    }

    private static Component ingredient(JsonElement value){
        if(value.isJsonArray()){
            var options=Component.empty();
            for(var item:value.getAsJsonArray()){
                if(!options.getSiblings().isEmpty())options.append(" / ");
                options.append(ingredient(item));
            }
            return options;
        }
        if(value.isJsonObject()&&value.getAsJsonObject().has("base"))return ingredient(value.getAsJsonObject().get("base"));
        String id=value.getAsString();
        return id.startsWith("#")?Component.translatable("tag.item."+id.substring(1).replace(':','.').replace('/','.')):name(id);
    }

    private static Component name(String id){
        return BuiltInRegistries.ITEM.getOptional(Identifier.parse(id)).map(item->new ItemStack(item).getHoverName())
            .orElseGet(()->Component.literal(id));
    }
}
