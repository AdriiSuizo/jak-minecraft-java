// Included by crafting.rs. Java 26.3 supplies brewing as ordinary recipe data.
impl RecipeBook {
    fn potion_ingredient(&self, v: &Value, s: &ItemStack, contents: bool) -> bool {
        if !self.station_ingredient(&v["item"], s) { return false; }
        if !contents { return true; }
        let Some(expected) = v.get("potion_contents") else { return true; };
        let actual = s.components.as_ref().and_then(|c| c.get("minecraft:potion_contents"));
        if let Some(potions) = expected.get("potions") {
            let potion = actual.and_then(|c| c.get("potion")).and_then(Value::as_str);
            if !potions.as_str().is_some_and(|p| Some(p) == potion)
                && !potions.as_array().is_some_and(|a| a.iter().any(|p| p.as_str() == potion)) { return false; }
        }
        true
    }

    pub fn brewing_accepts(&self, slot: usize, s: &ItemStack) -> bool {
        match slot {
            0..=2 => self.station_ingredient(&Value::String("#minecraft:brewing_potion_inputs".into()), s)
                || self.stations.iter().any(|(_,v)| v["type"] == "minecraft:brewing" && self.potion_ingredient(&v["input"], s, false)),
            3 => self.stations.iter().any(|(_,v)| v["type"] == "minecraft:brewing" && self.potion_ingredient(&v["reagent"], s, false)),
            4 => self.brewing_fuel(s).is_some(),
            _ => false,
        }
    }

    pub fn brew(&self, bottle: &ItemStack, reagent: &ItemStack) -> Option<ItemStack> {
        let (_,v) = self.stations.iter().find(|(_,v)| v["type"] == "minecraft:brewing"
            && self.potion_ingredient(&v["input"], bottle, true)
            && self.potion_ingredient(&v["reagent"], reagent, true))?;
        // BrewingRecipe.assemble creates the output template, not a copy of input components.
        let out = &v["output"];
        let id = out["id"].as_str()?;
        Some(ItemStack {id:id.into(),count:out["count"].as_u64().unwrap_or(1).clamp(1,64) as u8,
            max:self.max_stack(id),components:out.get("components").cloned()})
    }

    pub fn brewing_fuel(&self, s: &ItemStack) -> Option<(u32,f32)> {
        if let Some(v) = s.components.as_ref().and_then(|c|c.get("minecraft:brewing_fuel")) {
            if v.is_null() {return None;}
            return Some((v["uses"].as_u64().unwrap_or(20).min(u32::MAX as u64) as u32,
                v["speed_multiplier"].as_f64().unwrap_or(1.0) as f32));
        }
        (s.id == "minecraft:blaze_powder").then_some((20,1.0))
    }
}
