// Included by crafting.rs. LoomMenu rules, with pattern lists loaded from Java's tags.
impl RecipeBook {
    pub fn loom_accepts(&self, slot:usize, stack:&ItemStack)->bool {
        if stack.count==0 {return false;}
        match slot {
            0=>self.matches_ingredient(&Ingredient(vec!["#minecraft:banners".into()]),&stack.id),
            1=>self.matches_ingredient(&Ingredient(vec!["#minecraft:loom_dyes".into()]),&stack.id) && loom_dye(stack).is_some(),
            2=>self.matches_ingredient(&Ingredient(vec!["#minecraft:loom_patterns".into()]),&stack.id) && loom_pattern_tag(stack).is_some(),
            _=>false,
        }
    }
    pub fn loom_patterns(&self, pattern:Option<&ItemStack>)->Vec<String> {
        let tag=match pattern {None=>"minecraft:no_item_required".into(),Some(s)=>match loom_pattern_tag(s) {Some(t)=>t,None=>return Vec::new()}};
        self.banner_tags.get(tag.trim_start_matches('#')).cloned().unwrap_or_default()
    }
    pub fn loom_result(&self, inputs:&[Option<ItemStack>],pattern:&str)->Option<ItemStack> {
        let banner=inputs.first()?.as_ref()?;let dye=inputs.get(1)?.as_ref()?;
        if !self.loom_accepts(0,banner)||!self.loom_accepts(1,dye) {return None;}
        if !self.loom_patterns(inputs.get(2).and_then(Option::as_ref)).iter().any(|p|p==pattern) {return None;}
        let mut layers=banner.components.as_ref().and_then(|c|c.get("minecraft:banner_patterns")).and_then(Value::as_array).cloned().unwrap_or_default();
        if layers.len()>=6 {return None;}
        layers.push(serde_json::json!({"pattern":pattern,"color":loom_dye(dye)?}));
        let mut result=banner.clone();result.count=1;
        result.components.get_or_insert_with(||serde_json::json!({})).as_object_mut()?.insert("minecraft:banner_patterns".into(),layers.into());
        Some(result)
    }
}
fn loom_dye(s:&ItemStack)->Option<String> {
    if let Some(v)=s.components.as_ref().and_then(|c|c.get("minecraft:dye")) {return v.as_str().map(str::to_owned);}
    s.id.strip_prefix("minecraft:")?.strip_suffix("_dye").map(str::to_owned)
}
fn loom_pattern_tag(s:&ItemStack)->Option<String> {
    if let Some(v)=s.components.as_ref().and_then(|c|c.get("minecraft:provides_banner_patterns")) {return v.as_str().map(str::to_owned);}
    // Native Items banner-pattern component defaults. Membership is still checked against the loaded item tag.
    let item=s.id.strip_prefix("minecraft:")?.strip_suffix("_banner_pattern")?;
    Some(format!("minecraft:pattern_item/{item}"))
}
