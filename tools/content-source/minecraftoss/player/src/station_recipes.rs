// Included by crafting.rs so every workstation shares its native item tags/catalog.
fn is_station_recipe(value: &Value) -> bool {
    matches!(value["type"].as_str(), Some("minecraft:brewing" | "minecraft:stonecutting" | "minecraft:smithing_transform" | "minecraft:smithing_trim"))
}

impl RecipeBook {
    fn station_ingredient(&self, value: &Value, stack: &ItemStack) -> bool {
        stack.count > 0 && Ingredient::parse(value).is_some_and(|i| self.matches_ingredient(&i, &stack.id))
    }

    /// Native stonecutter entries for this input, retaining the recipe's output quantity.
    pub fn stonecutting(&self, input: &ItemStack) -> Vec<(String, ItemStack)> {
        let mut out: Vec<_> = self.stations.iter().filter_map(|(id, v)| {
            if v["type"] != "minecraft:stonecutting" || !self.station_ingredient(&v["ingredient"], input) { return None; }
            let mut result = parse_result(v)?;
            result.max = self.max_stack(&result.id);
            Some((id.clone(), result))
        }).collect();
        // Stable result ordering keeps selection predictable across inventory updates.
        out.sort_by(|a,b| a.1.id.cmp(&b.1.id).then(a.0.cmp(&b.0)));
        out
    }

    pub fn smithing_accepts(&self, slot: usize, input: &ItemStack) -> bool {
        let Some(key) = ["template", "base", "addition"].get(slot) else { return false; };
        self.stations.iter().any(|(_,v)| v["type"].as_str().is_some_and(|s|s.starts_with("minecraft:smithing_"))
            && self.station_ingredient(&v[*key], input))
    }

    /// Smithing transforms copy the base's component patch, not its item's defaults.
    /// A trim identical to the existing material/pattern produces no result.
    pub fn smithing(&self, input: &[Option<ItemStack>]) -> Option<ItemStack> {
        let template = input.first()?.as_ref()?;
        let base = input.get(1)?.as_ref()?;
        let addition = input.get(2)?.as_ref()?;
        for (_, v) in &self.stations {
            if !self.station_ingredient(&v["template"], template)
                || !self.station_ingredient(&v["base"], base)
                || !self.station_ingredient(&v["addition"], addition) { continue; }
            let mut out = base.clone();
            out.count = 1;
            match v["type"].as_str() {
                Some("minecraft:smithing_transform") => {
                    out.id = v["result"]["id"].as_str()?.into();
                    if let Some(patch) = v["result"]["components"].as_object() {
                        let c = out.components.get_or_insert_with(|| serde_json::json!({})).as_object_mut()?;
                        c.extend(patch.clone());
                    }
                    out.max = self.max_stack(&out.id);
                    // Older mashup saves stored durability as a fraction. Convert it to
                    // Java's absolute damage before changing the material's durability.
                    if let (Some(health),Some((native_damage,old_max)),Some((_,new_max)))=(
                        base.components.as_ref().and_then(|c|c.get("health")).and_then(Value::as_f64),self.durability(base),self.durability(&out)) {
                        let damage=if base.components.as_ref().is_some_and(|c|c.get("minecraft:damage").is_some()) {native_damage}
                            else {((1.0-health.clamp(0.0,1.0))*old_max as f64).round() as u32};
                        let c=out.components.as_mut()?.as_object_mut()?;
                        c.insert("minecraft:damage".into(),damage.into());
                        c.insert("health".into(),serde_json::json!(1.0-damage.min(new_max) as f64/new_max as f64));
                    }
                    return Some(out);
                }
                Some("minecraft:smithing_trim") => {
                    let material = trim_material(addition)?;
                    let trim = serde_json::json!({"material":material,"pattern":v["pattern"]});
                    let c = out.components.get_or_insert_with(|| serde_json::json!({})).as_object_mut()?;
                    if c.get("minecraft:trim") == Some(&trim) { return None; }
                    c.insert("minecraft:trim".into(), trim);
                    return Some(out);
                }
                _ => {}
            }
        }
        None
    }
}

/// Items' PROVIDES_TRIM_MATERIAL defaults; explicit stack patches take precedence.
fn trim_material(stack: &ItemStack) -> Option<String> {
    if let Some(value) = stack.components.as_ref().and_then(|c|c.get("minecraft:provides_trim_material")) {
        return value.as_str().or_else(||value.get("material").and_then(Value::as_str)).map(str::to_owned);
    }
    let material = match stack.id.strip_prefix("minecraft:")? {
        "amethyst_shard" => "amethyst", "copper_ingot" => "copper", "diamond" => "diamond",
        "emerald" => "emerald", "gold_ingot" => "gold", "iron_ingot" => "iron",
        "lapis_lazuli" => "lapis", "netherite_ingot" => "netherite", "quartz" => "quartz",
        "redstone" => "redstone", "resin_brick" => "resin", _ => return None,
    };
    Some(format!("minecraft:{material}"))
}
