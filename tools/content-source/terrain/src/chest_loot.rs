//! Container loot: the `data/minecraft/loot_table/chests/*` tables of the
//! game's data JAR, rolled when a generated chest or barrel is first opened
//! (`RandomizableContainer.unpackLootTable`). The pools, their rolls, weighted
//! item and empty entries, nested tables, `set_count` and `random_chance`
//! are followed; enchanting and damage functions are left out, so such an
//! item comes plain. The rolled stacks are spread over random free slots
//! as `LootTable.fill` does, without its splitting of stacks.
use anyhow::{Context, Result};
use serde_json::Value;
use std::collections::HashMap;
use std::io::Read;
use std::path::Path;

/// Every container table by id (`minecraft:chests/simple_dungeon`), and
/// the fishing ones (`minecraft:gameplay/fishing`).
#[derive(Default)]
pub struct ChestLoot {
    tables: HashMap<String, Value>,
}

/// What a roll knows of where it is: the luck (`LootContext.getLuck`: Luck
/// of the Sea and the Luck effect) that weighs entries by their `quality`,
/// and whether a fishing hook is in open water (`FishingHookPredicate`).
#[derive(Clone, Copy, Debug, Default)]
pub struct RollContext {
    pub luck: f32,
    pub open_water: bool,
}

/// The rolls a table draws from: `java.util.Random`-like, seeded per chest.
pub struct LootRng(u64);

impl LootRng {
    pub fn new(seed: i64) -> Self {
        Self((seed as u64 ^ 0x5DEE_CE66D) & ((1 << 48) - 1))
    }

    fn next(&mut self, bits: u32) -> i32 {
        self.0 = (self.0.wrapping_mul(0x5DEE_CE66D).wrapping_add(0xB)) & ((1 << 48) - 1);
        (self.0 >> (48 - bits)) as i32
    }

    pub fn below(&mut self, bound: i32) -> i32 {
        if bound <= 1 {
            return 0;
        }
        ((self.next(31) as i64 * bound as i64) >> 31) as i32
    }

    pub fn float(&mut self) -> f32 {
        self.next(24) as f32 / (1 << 24) as f32
    }
}

impl ChestLoot {
    pub fn from_jar(path: &Path) -> Result<Self> {
        let file = std::fs::File::open(path)
            .with_context(|| format!("open data JAR {}", path.display()))?;
        let mut archive = zip::ZipArchive::new(file)?;
        let mut tables = HashMap::new();
        for index in 0..archive.len() {
            let mut entry = archive.by_index(index)?;
            let name = entry.name().to_owned();
            let Some(stem) = name
                .strip_prefix("data/minecraft/loot_table/")
                .and_then(|n| n.strip_suffix(".json"))
            else {
                continue;
            };
            if !(stem.starts_with("chests/") || stem.starts_with("equipment/") || stem.starts_with("gameplay/fishing")) {
                continue;
            }
            let mut source = String::new();
            entry.read_to_string(&mut source)?;
            let value: Value =
                serde_json::from_str(&source).with_context(|| format!("parse {name}"))?;
            tables.insert(format!("minecraft:{stem}"), value);
        }
        Ok(Self { tables })
    }

    pub fn len(&self) -> usize {
        self.tables.len()
    }

    pub fn is_empty(&self) -> bool {
        self.tables.is_empty()
    }

    /// The stacks a table gives: `(item id, count)`.
    pub fn roll(&self, table: &str, rng: &mut LootRng) -> Vec<(String, u8)> {
        self.roll_with(table, rng, RollContext::default())
    }

    /// As [`Self::roll`], with the roll's luck and place.
    pub fn roll_with(&self, table: &str, rng: &mut LootRng, ctx: RollContext) -> Vec<(String, u8)> {
        let mut out = Vec::new();
        self.roll_into(table, rng, &mut out, 0, &ctx);
        out
    }

    /// Fills `slots` (a container's, all empty) with a table's loot at
    /// random free slots.
    pub fn fill(&self, table: &str, seed: i64, slots: usize) -> Vec<Option<(String, u8)>> {
        self.fill_with(table,seed,slots,RollContext::default())
    }
    pub fn fill_with(&self,table:&str,seed:i64,slots:usize,ctx:RollContext)->Vec<Option<(String,u8)>> {
        let mut rng = LootRng::new(seed);
        let items = self.roll_with(table, &mut rng,ctx);
        let mut out = vec![None; slots];
        let mut free: Vec<usize> = (0..slots).collect();
        for i in (1..free.len()).rev() {
            let j = rng.below(i as i32 + 1) as usize;
            free.swap(i, j);
        }
        for (item, slot) in items.into_iter().zip(free) {
            out[slot] = Some(item);
        }
        out
    }

    fn roll_into(&self, table: &str, rng: &mut LootRng, out: &mut Vec<(String, u8)>, depth: u32, ctx: &RollContext) {
        let Some(table) = self.tables.get(table) else {
            return;
        };
        if depth > 4 {
            return;
        }
        for pool in table["pools"].as_array().into_iter().flatten() {
            if !conditions_pass(&pool["conditions"], rng, ctx) {
                continue;
            }
            let rolls=(number(&pool["rolls"],rng).floor() as i32).saturating_add((number_float(&pool["bonus_rolls"],rng)*ctx.luck).floor() as i32);
            let entries: Vec<&Value> = pool["entries"]
                .as_array()
                .into_iter()
                .flatten()
                .filter(|e| conditions_pass(&e["conditions"], rng, ctx))
                .collect();
            // `LootPoolSingletonContainer.getWeight`: weight plus quality
            // times luck, never under nothing.
            let weight = |e: &Value| {
                let w = e["weight"].as_i64().unwrap_or(1) as f32 + e["quality"].as_i64().unwrap_or(0) as f32 * ctx.luck;
                w.floor().max(0.0) as i32
            };
            let total: i32 = entries.iter().map(|e| weight(e)).sum();
            for _ in 0..rolls {
                if total <= 0 {
                    break;
                }
                let mut pick = rng.below(total);
                let Some(entry) = entries.iter().find(|e| {
                    pick -= weight(e);
                    pick < 0
                }) else {
                    continue;
                };
                self.entry_into(entry, rng, out, depth, ctx);
            }
        }
    }

    fn entry_into(&self, entry: &Value, rng: &mut LootRng, out: &mut Vec<(String, u8)>, depth: u32, ctx: &RollContext) {
        match entry["type"].as_str().unwrap_or("") {
            "minecraft:item" => {
                let Some(name) = entry["name"].as_str() else {
                    return;
                };
                let mut count = 1.0f32;
                for function in entry["functions"].as_array().into_iter().flatten() {
                    if !conditions_pass(&function["conditions"], rng, ctx) {
                        continue;
                    }
                    if function["function"].as_str() == Some("minecraft:set_count") {
                        let n = number(&function["count"], rng);
                        if function["add"].as_bool() == Some(true) {
                            count += n;
                        } else {
                            count = n;
                        }
                    }
                }
                let count = count.floor().clamp(0.0, 64.0) as u8;
                if count > 0 {
                    out.push((name.to_owned(), count));
                }
            }
            "minecraft:loot_table" => {
                if let Some(value) = entry["value"].as_str().or(entry["name"].as_str()) {
                    self.roll_into(value, rng, out, depth + 1, ctx);
                }
            }
            "minecraft:alternatives" | "minecraft:group" | "minecraft:sequence" => {
                for child in entry["children"].as_array().into_iter().flatten() {
                    if conditions_pass(&child["conditions"], rng, ctx) {
                        self.entry_into(child, rng, out, depth, ctx);
                        if entry["type"].as_str() == Some("minecraft:alternatives") {
                            break;
                        }
                    }
                }
            }
            _ => {}
        }
    }
}

/// Float providers use nextFloat even when both uniform endpoints are integers.
fn number_float(value:&Value,rng:&mut LootRng)->f32 {
    if value.is_null(){return 0.0;}if let Some(n)=value.as_f64(){return n as f32;}
    match value["type"].as_str().unwrap_or("minecraft:uniform") {
        "minecraft:constant"=>value["value"].as_f64().unwrap_or(0.0) as f32,
        "minecraft:binomial"=>number(value,rng),
        _=>{let min=number_float(&value["min"],rng);let max=number_float(&value["max"],rng);if min>=max{min}else{min+rng.float()*(max-min)}}
    }
}

/// A number provider: a constant, `uniform`, `binomial` or `constant`.
fn number(value: &Value, rng: &mut LootRng) -> f32 {
    if value.is_null(){return 0.0;}
    if let Some(n) = value.as_f64() {
        return n as f32;
    }
    match value["type"].as_str().unwrap_or("minecraft:uniform") {
        "minecraft:constant" => value["value"].as_f64().unwrap_or(1.0) as f32,
        "minecraft:binomial" => {
            let n = number(&value["n"], rng).floor() as i32;
            let p = number(&value["p"], rng);
            (0..n).filter(|_| rng.float() < p).count() as f32
        }
        _ => {
            let (min, max) = (number(&value["min"], rng), number(&value["max"], rng));
            if (min.fract() == 0.0) && (max.fract() == 0.0) && max >= min {
                // `UniformGenerator.getInt`: `Mth.nextInt(min, max)` inclusive.
                min + rng.below((max - min) as i32 + 1) as f32
            } else {
                min + rng.float() * (max - min)
            }
        }
    }
}

/// Conditions a chest roll can meet without a player or a location:
/// `random_chance` rolls, anything else passes.
fn conditions_pass(conditions: &Value, rng: &mut LootRng, ctx: &RollContext) -> bool {
    conditions.as_array().into_iter().flatten().all(|c| {
        match c["condition"].as_str() {
            Some("minecraft:random_chance") => rng.float() < number(&c["chance"], rng),
            // The treasure's `this` fishing hook in open water.
            Some("minecraft:entity_properties")
                if c["predicate"]["type_specific"]["type"].as_str() == Some("minecraft:fishing_hook") =>
            {
                c["predicate"]["type_specific"]["in_open_water"].as_bool().is_none_or(|w| w == ctx.open_water)
            }
            _ => true,
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rolls_a_simple_table() {
        let table: Value = serde_json::from_str(
            r#"{"pools":[{"rolls":{"type":"minecraft:uniform","min":2,"max":3},
            "entries":[{"type":"minecraft:item","name":"minecraft:bread","weight":3,
              "functions":[{"function":"minecraft:set_count","count":{"type":"minecraft:uniform","min":1,"max":4}}]},
              {"type":"minecraft:empty","weight":1}]}]}"#,
        )
        .unwrap();
        let mut loot = ChestLoot::default();
        loot.tables.insert("minecraft:chests/test".into(), table);
        let slots = loot.fill("minecraft:chests/test", 42, 27);
        assert_eq!(slots.len(), 27);
        let items: Vec<_> = slots.iter().flatten().collect();
        assert!(items.len() <= 3);
        assert!(items.iter().all(|(id, n)| id == "minecraft:bread" && (1..=4).contains(n)));
    }
}

#[cfg(test)]mod luck_loot_regressions {
    use super::*;
    #[test]fn quality_and_bonus_rolls_use_the_openers_luck(){
        let mut loot=ChestLoot::default();loot.tables.insert("minecraft:chests/luck".into(),serde_json::json!({"pools":[{"rolls":1,"bonus_rolls":2,"entries":[{"type":"minecraft:item","name":"minecraft:diamond","weight":0,"quality":1},{"type":"minecraft:item","name":"minecraft:dirt","weight":1,"quality":-1}]}]}));
        let neutral=loot.fill_with("minecraft:chests/luck",42,27,RollContext::default());assert_eq!(neutral.iter().flatten().cloned().collect::<Vec<_>>(),vec![("minecraft:dirt".into(),1)]);
        let lucky=loot.fill_with("minecraft:chests/luck",42,27,RollContext{luck:1.0,open_water:false});assert_eq!(lucky.iter().flatten().count(),3);assert!(lucky.iter().flatten().all(|(id,n)|id=="minecraft:diamond"&&*n==1));
        let unlucky=loot.fill_with("minecraft:chests/luck",42,27,RollContext{luck:-1.0,open_water:false});assert!(unlucky.iter().all(Option::is_none));
    }
    #[test]fn a_missing_bonus_is_zero_and_float_ranges_are_not_integer_rounded(){let mut r=LootRng::new(42);assert_eq!(number_float(&Value::Null,&mut r),0.0);let v=number_float(&serde_json::json!({"type":"minecraft:uniform","min":0,"max":1}),&mut r);assert!(v>0.0&&v<1.0);}
}
