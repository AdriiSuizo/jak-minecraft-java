//! What the chests of the Jak lands' houses hold: things of their own game
//! (Adrii 23/10: "que los cofres de las construcciones tengan loot
//! relacionado su universo"). Spargus's are Jak 3's (its guns' ammo, dark
//! and green eco, now and then one of its guns or a light eco vent),
//! Haven's Jak 2's (his guns' ammo, dark and green eco, now and then his
//! JET-Board or blaster), Snowy Mountain's forts and Boggy Billy's huts
//! Jak 1's (green eco, now and then a vent of its four ecos or a Flut Flut
//! egg).

use std::collections::BTreeMap;

use minecraftoss_core::chunk::Chunk;
use minecraftoss_core::nbt::Tag;

/// Which game a house is of.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Game {
    Jak1,
    Jak2,
    Jak3,
}

/// One thing a chest may hold: its id, how many at least and at most, and
/// its weight among the others.
type Thing = (&'static str, u8, u8, u32);

const JAK1: &[Thing] = &[
    ("jak:green_eco", 4, 16, 40),
    ("jak:vent_blue", 1, 1, 6),
    ("jak:vent_yellow", 1, 1, 6),
    ("jak:vent_red", 1, 1, 6),
    ("jak:vent_green", 1, 1, 6),
    ("jak:flut_egg", 1, 1, 5),
];

const JAK2: &[Thing] = &[
    ("jak:yellow_ammo", 10, 40, 25),
    ("jak:red_ammo", 4, 15, 18),
    ("jak:blue_ammo", 10, 40, 18),
    ("jak:dark_ammo", 1, 3, 8),
    ("jak:dark_eco", 2, 8, 15),
    ("jak:green_eco", 4, 12, 20),
    ("jak:vent_dark", 1, 1, 2),
    ("jak:blaster", 1, 1, 2),
    ("jak:jet_board", 1, 1, 2),
];

const JAK3: &[Thing] = &[
    ("jak:yellow_ammo", 10, 40, 22),
    ("jak:red_ammo", 4, 15, 16),
    ("jak:blue_ammo", 10, 40, 16),
    ("jak:dark_ammo", 1, 3, 8),
    ("jak:dark_eco", 2, 8, 12),
    ("jak:green_eco", 4, 12, 16),
    ("jak:gyro_burster", 1, 1, 1),
    ("jak:arc_wielder", 1, 1, 1),
    ("jak:plasmite_rpg", 1, 1, 1),
    ("jak:super_nova", 1, 1, 1),
    ("jak:vent_light", 1, 1, 3),
    ("jak:vent_yellow", 1, 1, 2),
];

fn hash(seed: i64, (x, y, z): (i32, i32, i32), salt: u64) -> u64 {
    let mut h = (seed as u64) ^ 0x9e37_79b9_7f4a_7c15 ^ salt.wrapping_mul(0x632b_e59b_d9b4_e019);
    for v in [x, y, z] {
        h ^= v as u32 as u64;
        h = h.wrapping_mul(0xff51_afd7_ed55_8ccd);
        h ^= h >> 33;
    }
    h
}

/// The things of a chest of `game` at `at` (3 to 6 stacks in scattered
/// slots), as a chest's block entity tag.
pub fn chest(game: Game, seed: i64, at: (i32, i32, i32)) -> Tag {
    let table = match game {
        Game::Jak1 => JAK1,
        Game::Jak2 => JAK2,
        Game::Jak3 => JAK3,
    };
    let total: u32 = table.iter().map(|t| t.3).sum();
    let stacks = 3 + (hash(seed, at, 1) % 4) as usize;
    let mut slots: Vec<u8> = Vec::new();
    let mut list = Vec::new();
    for k in 0..stacks {
        let mut roll = (hash(seed, at, 10 + k as u64) % u64::from(total)) as u32;
        let &(id, lo, hi, _) = table.iter().find(|t| {
            if roll < t.3 {
                true
            } else {
                roll -= t.3;
                false
            }
        })
        .unwrap_or(&table[0]);
        let count = lo + (hash(seed, at, 20 + k as u64) % u64::from(hi - lo + 1)) as u8;
        // A slot of the 27 not taken yet.
        let mut slot = (hash(seed, at, 30 + k as u64) % 27) as u8;
        while slots.contains(&slot) {
            slot = (slot + 1) % 27;
        }
        slots.push(slot);
        let mut m = BTreeMap::new();
        m.insert("Slot".to_owned(), Tag::Byte(slot as i8));
        m.insert("id".to_owned(), Tag::String(id.to_owned()));
        m.insert("count".to_owned(), Tag::Int(i32::from(count)));
        list.push(Tag::Compound(m));
    }
    let mut tag = BTreeMap::new();
    tag.insert("id".to_owned(), Tag::String("minecraft:chest".to_owned()));
    tag.insert("Items".to_owned(), Tag::List(list));
    Tag::Compound(tag)
}

/// Fills a placed chest at `at` (a block of the world, in `chunk`) with
/// `game`'s things, when `state` is a chest's.
pub fn fill(chunk: &mut Chunk, at: (i32, i32, i32), state: &str, game: Game, seed: i64) {
    if state.starts_with("minecraft:chest") {
        chunk.block_entities.pending.insert(at, chest(game, seed, at));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every thing is one of the mashup's items.
    #[test]
    fn every_thing_is_an_item() {
        for t in JAK1.iter().chain(JAK2).chain(JAK3) {
            assert!(crate::jak_items::item(t.0).is_some(), "{}", t.0);
            assert!(t.1 <= t.2);
        }
    }

    #[test]
    fn chests_hold_their_games_things() {
        for (game, table) in [(Game::Jak1, JAK1), (Game::Jak2, JAK2), (Game::Jak3, JAK3)] {
            for k in 0..50 {
                let Tag::Compound(tag) = chest(game, 7, (k, 64, -k)) else { panic!() };
                let Some(Tag::List(items)) = tag.get("Items") else { panic!() };
                assert!((3..=6).contains(&items.len()));
                let mut slots = Vec::new();
                for item in items {
                    let Tag::Compound(item) = item else { panic!() };
                    let Some(Tag::String(id)) = item.get("id") else { panic!() };
                    assert!(table.iter().any(|t| t.0 == id), "{id}");
                    let Some(Tag::Byte(slot)) = item.get("Slot") else { panic!() };
                    assert!(!slots.contains(slot));
                    slots.push(*slot);
                }
            }
        }
    }
}
