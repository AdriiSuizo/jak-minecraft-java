//! The integrated server's world simulation for a streamed world: the
//! exact `minecraftoss_world::level::Level` owns the loaded FULL chunks,
//! takes the player's edits and uses, ticks at 20 Hz, and hands back the
//! positions whose blocks changed so the client scene and the saved chunks
//! follow it.

use crate::scene::{Block, BlockPos, HandcraftedScene, Scene};
use crate::terrain::BlockStates;
use minecraftoss_core::{BlockStateId, Chunk, ChunkPos};
use minecraftoss_world::chunk_map::WorldGen;
use minecraftoss_world::level::{Level, update};
use minecraftoss_world::natural_spawner::tick::{CensusMob, SpawnPlayer};
use std::sync::Arc;

/// How a player changed a block (`BlockItem.place` sets with flags 11,
/// breaking removes the block with flags 3).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PlayerEdit {
    Place,
    Break,
}

pub struct ServerSim {
    level: Level<'static>,
    states: Arc<BlockStates>,
    /// Mobs, ticked after the level each tick.
    mobs: minecraftoss_entities::world::EntityWorld,
    /// Each block state as mobs read it.
    mob_tables: crate::server_mobs::MobTables,
    /// The tag each mob was loaded or spawned from, by entity ID: saving
    /// overwrites what the entity world simulates and keeps the rest.
    mob_tags: std::collections::HashMap<u64, minecraftoss_core::nbt::Tag>,
    /// The entity storage the level's entities are saved to, shared with
    /// the chunk map (`EntityStorage`).
    storage: Option<Arc<minecraftoss_world::storage::ChunkStorage>>,
    /// Server ticks since the last autosave.
    ticks_since_save: u32,
    entity_loot: Option<minecraftoss_entities::loot::EntityLootBook>,
    shearing_loot: Option<minecraftoss_entities::loot::ShearingLootBook>,
    /// Spawned or loaded mobs the entity world does not simulate yet, each
    /// with its riders, and the tag that saves them unchanged.
    dormant: Vec<(Vec<CensusMob>, minecraftoss_core::nbt::Tag)>,
    /// Creeper blasts since the client last heard of them.
    explosions: Vec<minecraftoss_entities::creeper::CreeperExplosion>,
    /// The recipe book (the items' stack sizes, for hopper minecarts).
    recipe_book: Option<Arc<minecraftoss_player::crafting::RecipeBook>>,
    /// The player's `takeXpDelay`: ticks until it can take another orb.
    take_xp_delay: i32,
    /// The client was asked to close its trading screen.
    merchant_closing: bool,
    /// Mobs the Mass Inverter took gravity from: ticks left.
    lifts: std::collections::HashMap<u64, u32>,
    /// Mobs riding a boat or a minecart: where their seat is (they sit
    /// there, still, until let go).
    seats: std::collections::HashMap<u64, glam::DVec3>,
    /// Mobs named with a name tag (`CustomName`), by entity ID, and whether
    /// the client has yet to hear the list.
    names: std::collections::HashMap<u64, String>,
    names_dirty: bool,
    thrown: Vec<crate::thrown::Flying>,
    arrows: Vec<crate::thrown::FlyingArrow>,
    arrow_pickup_id: i32,
    /// Native skeleton arrows have physical state but no render history.
    mob_arrows: std::collections::HashMap<u64, crate::arrow_render::ArrowView>,
    clouds: Vec<crate::thrown::Cloud>,
    throw_random: minecraftoss_player::rng::LegacyRandom,
}

/// What a player's hit or use on a mob did, for the client to present.
#[derive(Clone, Debug, Default)]
pub struct MobResult {
    pub sounds: Vec<crate::mob_actions::MobSound>,
    /// The acting player's inventory slots the action changed, with what
    /// they now hold.
    pub slots: Vec<(usize, Option<minecraftoss_player::inventory::ItemStack>)>,
    /// A trading screen the action opened (`openTradingScreen`).
    pub merchant: Option<MerchantView>,
}

/// A player's trading screen as the client shows it: the villager's
/// offers, profession, level and experience
/// (`ClientboundMerchantOffersPacket`) and the menu's slots.
#[derive(Clone, Debug, PartialEq)]
pub struct MerchantView {
    pub villager: u64,
    pub profession: &'static str,
    pub level: i32,
    pub xp: i32,
    pub offers: Vec<minecraftoss_entities::trading::MerchantOffer>,
    pub payment: [Option<minecraftoss_player::inventory::ItemStack>; 2],
    pub result: Option<minecraftoss_player::inventory::ItemStack>,
    pub future_xp: i32,
    /// Items' maximum stack sizes the offers name, for the client's
    /// prices.
    pub max_stacks: Vec<(String, i32)>,
}

/// What the player does on the trading screen.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MerchantOp {
    /// An offer picked in the list.
    Select(i32),
    /// A payment slot clicked (shift: moved back).
    Payment {
        slot: usize,
        right: bool,
        shift: bool,
    },
    /// The result clicked (shift: traded as often as it goes).
    Result {
        shift: bool,
    },
    Close,
}

/// A trading screen's new state after an operation, or its closing
/// (`view` none), with the acting player's inventory slots it changed and
/// the cursor.
#[derive(Clone, Debug)]
pub struct MerchantUpdate {
    pub view: Option<MerchantView>,
    pub slots: Vec<(usize, Option<minecraftoss_player::inventory::ItemStack>)>,
    pub cursor: Option<minecraftoss_player::inventory::ItemStack>,
    /// The server asks the client to close the screen (the villager went
    /// away or stopped trading); the client answers with `Close`.
    pub closing: bool,
}

/// An item entity on the server, as the client shows it. Components travel
/// as their JSON text.
#[derive(Clone, Debug)]
pub struct ServerItem {
    pub id: i32,
    pub item: String,
    pub count: i32,
    pub components: Option<String>,
    pub position: [f64; 3],
    pub previous_position: [f64; 3],
    pub velocity: [f64; 3],
    pub age: i32,
    pub pickup_delay: i32,
    pub on_ground: bool,
    pub skull_gem: Option<minecraftoss_core::mashup_skull_gem::Motion>,
}

/// A falling block entity as the client shows it.
#[derive(Clone, Debug)]
pub struct ServerFallingBlock {
    pub position: [f64; 3],
    pub previous_position: [f64; 3],
    pub block: String,
}

/// An experience orb as the client shows it.
#[derive(Clone, Copy, Debug)]
pub struct ServerOrb {
    pub id: i32,
    pub position: [f64; 3],
    pub previous_position: [f64; 3],
    pub value: i32,
    /// `tickCount`, which drives its colour.
    pub tick_count: i32,
}

/// The level's entities as the client shows them after a tick.
#[derive(Clone, Debug, Default)]
pub struct EntitySnapshot {
    pub items: Vec<ServerItem>,
    pub tnt: Vec<ServerTnt>,
    pub falling: Vec<ServerFallingBlock>,
    pub orbs: Vec<ServerOrb>,
}

/// A primed TNT entity as the client shows it.
#[derive(Clone, Copy, Debug)]
pub struct ServerTnt {
    pub position: [f64; 3],
    pub previous_position: [f64; 3],
    pub fuse: i32,
}

fn stack_of(
    item: &str,
    count: i32,
    components: Option<&str>,
) -> minecraftoss_core::item::ItemStack {
    let mut stack = minecraftoss_core::item::ItemStack::new(item, count);
    stack.components = components.map(|json| minecraftoss_core::nbt::Tag::String(json.to_owned()));
    stack
}

fn components_of(stack: &minecraftoss_core::item::ItemStack) -> Option<String> {
    match &stack.components {
        Some(minecraftoss_core::nbt::Tag::String(json)) => Some(json.clone()),
        Some(other) => Some(format!("{other:?}")),
        None => None,
    }
}

impl ServerSim {
    /// A dimension owns its generation context while its simulation lives.
    /// Closing it releases that context and all retained generation caches.
    pub fn new(worldgen: Arc<WorldGen>, states: Arc<BlockStates>, dimension_type: &str) -> Self {
        let range = states.vertical_range();
        let mut level = Level::with_worldgen(worldgen.clone(), range.start, range.end - range.start);
        level.entity_chunk_barrier = true;
        // Items/TNT use the same native pieces as actors and the rendered
        // floor, rather than the catalog's fallback half-slab.
        level.block_collision_override=Some(|r,state,pos| {
            let name=r.blocks.block(r.blocks.block_of(state)).name.to_string();
            let place=crate::jak_pieces::place_of(name.strip_prefix("jak:")?)?;
            let boxes=crate::jak_pieces::collision_boxes(place as u8,[pos.x,pos.y,pos.z])?;
            Some(boxes.iter().map(|b|b.map(f64::from)).collect())
        });
        level.random_sequences =
            minecraftoss_core::loot::RandomSequences::new(worldgen.terrain.seed);
        if let Err(e) = level.set_dimension(dimension_type) {
            eprintln!("world simulation without environment: {e}");
        }
        let mob_tables = crate::server_mobs::MobTables::new(&level, &states);
        // Natural spawning, when the entity catalog can create mobs.
        if level.registries().entities.is_some() {
            match minecraftoss_world::natural_spawner::CreatureSpawns::load(
                worldgen.terrain.registries.clone(),
                dimension_type,
                false,
            ) {
                Ok(spawns) => {
                    level.natural_spawning = Some(
                        minecraftoss_world::level::spawning::NaturalSpawning::new(Arc::new(spawns)),
                    )
                }
                Err(e) => eprintln!("natural spawning unavailable: {e}"),
            }
        }
        // The players are real entities: arrows hit them.
        let mut mobs = minecraftoss_entities::world::EntityWorld::default();
        mobs.set_players_pickable(true);
        // New mobs' UUIDs differ from session to session, as vanilla's
        // random ones do.
        mobs.set_uuid_salt(
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map_or(0, |d| d.as_nanos() as u64),
        );
        mobs.pois =
            minecraftoss_entities::poi::PoiManager::new(range.start >> 4, (range.end - 1) >> 4);
        Self {
            level,
            states,
            mobs,
            mob_tables,
            mob_tags: std::collections::HashMap::new(),
            storage: None,
            ticks_since_save: 0,
            entity_loot: None,
            shearing_loot: None,
            dormant: Vec::new(),
            explosions: Vec::new(),
            recipe_book: None,
            take_xp_delay: 0,
            merchant_closing: false,
            lifts: std::collections::HashMap::new(),
            seats: std::collections::HashMap::new(),
            names: std::collections::HashMap::new(),
            names_dirty: false,
            thrown: Vec::new(),
            arrows: Vec::new(),
            arrow_pickup_id: -1,
            mob_arrows: Default::default(),
            clouds: Vec::new(),
            throw_random: minecraftoss_player::rng::LegacyRandom::new(0x7318),
        }
    }

    fn tick_thrown(&mut self, players: &[minecraftoss_entities::tempt::PlayerCandidate], external: &[crate::thrown::ExternalTarget], out: &mut Output) {
        use minecraftoss_entities::effects::{EffectWork, MobEffect};
        use minecraftoss_entities::world::PlayerSplash;
        use minecraftoss_core::nbt::Tag;
        use glam::DVec3;
        let mut flying = std::mem::take(&mut self.thrown);
        for flight in &mut flying {
            let mut targets = self.mobs.potion_targets(None, players);
            // potion_targets uses a reserved ID range for players. Mark the
            // owner's actual box explicitly, never a mob with the same ID.
            let owner = players.iter().find(|p| p.id == flight.request.player);
            let owner_box = owner.map(|p| (p.position - DVec3::new(0.3, 0.0, 0.3), p.position + DVec3::new(0.3, 1.8, 0.3)));
            for target in &mut targets {
                target.owner = owner_box.is_some_and(|(min, max)| target.min == min && target.max == max);
            }
            // Jak's creatures, mounts and vehicles and the end game's: struck
            // as any other entity, except the one the thrower sits in.
            let eye = flight.request.eye;
            targets.extend(external.iter().enumerate().filter(|(_, (_, b))| {
                !(eye.x >= b[0] && eye.x <= b[3] && eye.y >= b[1] - 1.0 && eye.y <= b[4] + 1.0 && eye.z >= b[2] && eye.z <= b[5])
            }).map(|(i, (_, b))| minecraftoss_entities::potion::PotionTarget {
                id: crate::thrown::EXTERNAL_TARGET + i as u64,
                min: DVec3::new(b[0], b[1], b[2]), max: DVec3::new(b[3], b[4], b[5]), owner: false,
            }));
            flight.previous = flight.body.position;
            let gravity = flight.gravity();
            let hit = {
                let world = crate::server_mobs::MobWorld { level: std::cell::RefCell::new(&mut self.level), states: &self.states, tables: &self.mob_tables };
                flight.body.tick_with_gravity(&world, &targets, owner_box, gravity)
            };
            let Some(hit) = hit else { continue };
            let at = hit.location;
            let struck = hit.entity.and_then(|id| match id.checked_sub(crate::thrown::EXTERNAL_TARGET) {
                Some(i) => external.get(i as usize).map(|(key, _)| (*key, false)),
                None => (id < minecraftoss_entities::world::PLAYER_TARGET).then_some((id, true)),
            });
            if let Some((key, native)) = struck.filter(|_| crate::thrown::strikes(&flight.request.item)) {
                let damage = if flight.request.item == crate::thrown::TRIDENT {
                    crate::thrown::TRIDENT_DAMAGE
                } else if crate::thrown::lands(&flight.request.item) {
                    let roll = self.throw_random.next_float();
                    crate::thrown::arrow_damage(flight.body.velocity.length(), flight.request.base_damage, flight.request.critical, roll)
                } else {
                    0.0
                };
                out.thrown_hits.push(crate::thrown::ThrownHit {
                    key, native, item: flight.request.item.clone(), damage,
                    from: flight.previous, player: flight.request.player,
                });
            }
            if crate::thrown::lands(&flight.request.item) {
                out.thrown_landed.push(crate::thrown::Landed {
                    player: flight.request.player,
                    item: flight.request.item.clone(),
                    components: flight.request.components.clone(),
                    position: at,
                });
            }
            if flight.request.item.ends_with("potion") || flight.request.item == "minecraft:experience_bottle" {
                out.potion_breaks.push(minecraftoss_entities::world::PotionBreak { position: at, color: minecraftoss_entities::potion::contents_color(&flight.request.effects), instant: flight.request.effects.iter().any(|e| e.effect.instantaneous()) });
                out.mob_sounds.push(minecraftoss_entities::world::MobSound { event: "minecraft:entity.splash_potion.break".into(), position: at, volume: 1.0, pitch: 1.0, category: "players_volume" });
            }
            match flight.request.item.as_str() {
                "minecraft:ender_pearl" => {
                    // The projectile, not the camera, decides the landing.
                    out.teleports.push((flight.request.player, at.to_array()));
                    out.player_splashes.push((flight.request.player, PlayerSplash {
                        effect: MobEffect::InstantDamage, amplifier: 0, duration: 0,
                        instant: Some(EffectWork::HurtMagic(5.0)), at,
                    }));
                }
                "minecraft:experience_bottle" => {
                    let amount = 3 + self.throw_random.next_int(5) + self.throw_random.next_int(5);
                    self.level.award_experience(at.to_array(), amount as i32);
                }
                "minecraft:egg" => {
                    if self.throw_random.next_int(8) == 0 {
                        let count = if self.throw_random.next_int(32) == 0 { 4 } else { 1 };
                        for _ in 0..count {
                            let tag = Tag::Compound([(String::from("Age"), Tag::Int(-24000))].into_iter().collect());
                            let _ = self.summon("minecraft:chicken", at.to_array(), Some(&tag), 0.0);
                        }
                    }
                }
                "minecraft:splash_potion" => {
                    self.mobs.splash(&flight.body, hit, players);
                    // Instant harm reaches what is outside the world's own
                    // entities too, by nearness (`onHitAsPotion`); they keep
                    // no lasting effects of their own.
                    let margin = flight.body.margin();
                    for (key, b) in external {
                        let (min, max) = (DVec3::new(b[0], b[1], b[2]), DVec3::new(b[3], b[4], b[5]));
                        let Some(scale) = minecraftoss_entities::potion::splash_scale(at, margin, min, max) else { continue };
                        for effect in flight.request.effects.iter().filter(|e| e.effect == MobEffect::InstantDamage) {
                            if let Some(EffectWork::HurtMagic(damage)) = minecraftoss_entities::effects::instant_work(effect.effect, effect.amplifier, false, scale) {
                                out.thrown_hits.push(crate::thrown::ThrownHit {
                                    key: *key, native: false, item: flight.request.item.clone(), damage,
                                    from: at, player: flight.request.player,
                                });
                            }
                        }
                    }
                }
                "minecraft:lingering_potion" => {
                    if !flight.request.effects.is_empty() { self.clouds.push(crate::thrown::Cloud::new(flight.body.clone())); }
                }
                _ => {}
            }
        }
        let targets = self.mobs.potion_targets(None, players);
        for cloud in &mut self.clouds {
            cloud.age += 1;
            if cloud.age < 20 { continue; }
            cloud.radius -= 3.0 / 600.0;
            for target in &targets {
                let at = cloud.body.position;
                let centre = (target.min + target.max) * 0.5;
                let distance = (centre.x - at.x).hypot(centre.z - at.z);
                if distance > f64::from(cloud.radius) || target.max.y < at.y || target.min.y > at.y + 0.5 { continue; }
                if !cloud.ready(target.id) { continue; }
                for effect in cloud.body.contents.clone().unwrap_or_default() {
                    let player = target.id.checked_sub(minecraftoss_entities::world::PLAYER_TARGET);
                    if let Some(player) = player {
                        out.player_splashes.push((player, PlayerSplash {
                            effect: effect.effect, amplifier: effect.amplifier,
                            duration: effect.duration / 4,
                            instant: minecraftoss_entities::effects::instant_work(effect.effect, effect.amplifier, false, 0.5), at,
                        }));
                    } else if effect.effect.instantaneous() {
                        self.mobs.splash_instant(target.id, effect.effect, effect.amplifier, 0.5, at, cloud.body.owner);
                    } else {
                        let mut effect = effect;
                        effect.duration = effect.map_duration(|d| d / 4);
                        self.mobs.add_mob_effect(target.id, effect);
                    }
                }
                cloud.radius -= 0.5;
            }
        }
        self.clouds.retain(|c| c.age < 620 && c.radius >= 0.5);
        out.clouds = Some(self.clouds.iter().map(|c| crate::thrown::CloudView { position: c.body.position, radius: c.radius, color: minecraftoss_entities::potion::contents_color(c.body.contents.as_deref().unwrap_or(&[])) }).collect());
        flying.retain(|f| f.body.alive && f.body.tick_count < 1200);
        out.thrown = Some(flying.iter().map(crate::thrown::Flying::view).collect());
        self.thrown = flying;
    }

    fn throw(&mut self, request: crate::thrown::Request) {
        if crate::thrown::is_arrow(&request.item) {
            self.arrows.push(crate::thrown::FlyingArrow::new(request, &mut self.throw_random, self.arrow_pickup_id));
            self.arrow_pickup_id = self.arrow_pickup_id.checked_sub(1).unwrap_or(-1);
        } else {
            self.thrown.push(crate::thrown::Flying::new(request, &mut self.throw_random));
        }
    }

    fn tick_arrows(&mut self, players: &[minecraftoss_entities::tempt::PlayerCandidate], external: &[crate::thrown::ExternalTarget], advance: bool, out: &mut Output) {
        use glam::DVec3;
        use minecraftoss_entities::{projectile::ArrowTarget, world::{PLAYER_TARGET, PlayerHit, PlayerHitKind}};
        if advance {
            for flight in &mut self.arrows {
                flight.previous = flight.body.position;
                if flight.body.velocity.length_squared() > 1e-8 { flight.direction = flight.body.velocity; }
                let eye = flight.request.eye;
                let mut targets: Vec<_> = self.mobs.potion_targets(None, players).into_iter()
                    .filter(|t| t.id != PLAYER_TARGET + flight.request.player)
                    .map(|t| ArrowTarget { id: t.id, min: t.min, max: t.max }).collect();
                targets.extend(external.iter().enumerate().filter(|(_, (_, b))| {
                    !(eye.x >= b[0] && eye.x <= b[3] && eye.y >= b[1] - 1.0 && eye.y <= b[4] + 1.0 && eye.z >= b[2] && eye.z <= b[5])
                }).map(|(i, (_, b))| ArrowTarget { id: crate::thrown::EXTERNAL_TARGET + i as u64, min: DVec3::new(b[0], b[1], b[2]), max: DVec3::new(b[3], b[4], b[5]) }));
                let fields: Vec<_> = self.level.weightless.iter().map(|&(p, r)| (DVec3::from_array(p), r)).collect();
                let (scale, weightless) = minecraftoss_entities::world::projectile_field_at(&fields, flight.body.position);
                flight.body.time_scale = scale;
                flight.body.weightless = weightless;
                let hit = {
                    let world = crate::server_mobs::MobWorld { level: std::cell::RefCell::new(&mut self.level), states: &self.states, tables: &self.mob_tables };
                    flight.body.tick_world(&world, &targets)
                };
                if let Some(hit) = hit {
                    let damage = crate::thrown::arrow_damage(hit.velocity.length(), flight.request.base_damage, flight.request.critical, self.throw_random.next_float());
                    let mut accepted = true;
                    if hit.target_id >= crate::thrown::EXTERNAL_TARGET {
                        if let Some((key, _)) = external.get((hit.target_id - crate::thrown::EXTERNAL_TARGET) as usize) {
                            out.thrown_hits.push(crate::thrown::ThrownHit { key: *key, native: false, item: flight.request.item.clone(), damage, from: flight.previous, player: flight.request.player });
                        }
                    } else if hit.target_id >= PLAYER_TARGET {
                        let id = hit.target_id - PLAYER_TARGET;
                        accepted = players.iter().any(|p| p.id == id && p.attackable);
                        if accepted { out.player_hits.push(PlayerHit { player_id: id, damage, kind: PlayerHitKind::Arrow { velocity: hit.velocity, effect: None }, source: Some(flight.request.player) }); }
                    } else {
                        out.thrown_hits.push(crate::thrown::ThrownHit { key: hit.target_id, native: true, item: flight.request.item.clone(), damage, from: flight.previous, player: flight.request.player });
                        for effect in &flight.request.effects { self.mobs.add_mob_effect(hit.target_id, effect.clone()); }
                    }
                    flight.body.resolve_entity_hit(accepted);
                }
                if let Some((position, pitch)) = flight.body.take_hit_sound() {
                    out.mob_sounds.push(minecraftoss_entities::world::MobSound { event: "minecraft:entity.arrow.hit".into(), position, volume: 1.0, pitch, category: "friendly_volume" });
                }
            }
        }
        self.arrows.retain(|f| f.body.alive);
        let mut views: Vec<_> = self.arrows.iter().map(crate::thrown::FlyingArrow::view).collect();
        self.mob_arrows.retain(|id, _| self.mobs.arrows().iter().any(|a| a.id == *id && a.arrow.alive));
        for entity in self.mobs.arrows().iter().filter(|a| a.arrow.alive) {
            let body = &entity.arrow;
            let old = self.mob_arrows.get(&entity.id);
            let view = crate::arrow_render::ArrowView {
                item: if entity.effect.is_some() { "minecraft:tipped_arrow" } else { "minecraft:arrow" }.into(),
                previous: if advance { old.map_or(body.position - body.velocity, |v| v.position) } else { old.map_or(body.position, |v| v.previous) },
                position: body.position,
                direction: if body.velocity.length_squared() > 1e-8 { body.velocity } else { old.map_or(DVec3::X, |v| v.direction) },
                shake: body.shake_time as f32,
            };
            self.mob_arrows.insert(entity.id, view.clone());
            views.push(view);
        }
        out.arrows = Some(views);
    }

    /// Loads the entity and shearing loot tables and the villager trade
    /// sets from the data JAR, with the world seed's random sequences.
    pub fn load_loot(&mut self, jar: &std::path::Path, seed: u64) {
        self.entity_loot = minecraftoss_entities::loot::EntityLootBook::from_jar(jar, seed)
            .map_err(|e| eprintln!("server entity loot unavailable: {e:#}"))
            .ok();
        self.shearing_loot = minecraftoss_entities::loot::ShearingLootBook::from_jar(jar, seed)
            .map_err(|e| eprintln!("server shearing loot unavailable: {e:#}"))
            .ok();
        // The trade sets read the items' enchantability and stack sizes
        // from the exported item catalog: beside a JAR fetched into a
        // MinecraftOSS-shaped folder, in a checkout, or under the working
        // directory.
        const CATALOG: &str = "artifacts/item-catalog/26.3.json";
        let catalog = [
            jar.parent().map(|root| root.join(CATALOG)),
            std::env::var_os("MINECRAFTOSS_ROOT")
                .map(|root| std::path::Path::new(&root).join(CATALOG)),
            Some(std::path::PathBuf::from(CATALOG)),
        ]
        .into_iter()
        .flatten()
        .find(|path| path.is_file());
        match catalog.map(|path| minecraftoss_entities::trading::TradeBook::from_jar(jar, &path)) {
            Some(Ok(book)) => self.mobs.set_trades(Arc::new(book), seed),
            Some(Err(e)) => eprintln!("server villager trades unavailable: {e:#}"),
            None => eprintln!("server villager trades unavailable: no item catalog"),
        }
    }

    /// A player's hit (`attack`, with what the player brings to it) or item
    /// use on a mob, with a copy of the player's inventory. Drops enter the
    /// level as item entities.
    pub fn mob_action(
        &mut self,
        hit: minecraftoss_entities::world::MobHit,
        attack: Option<minecraftoss_entities::world::PlayerAttack>,
        mut inventory: minecraftoss_player::inventory::Inventory,
        selected: usize,
        infinite: bool,
    ) -> MobResult {
        let before = inventory.slots.clone();
        let mut actor = crate::mob_actions::Actor {
            inventory: &mut inventory,
            selected,
            infinite,
            entity_loot: self.entity_loot.as_mut(),
            shearing_loot: self.shearing_loot.as_mut(),
        };
        let outcome = if let Some(attack) = &attack {
            crate::mob_actions::attack(&mut self.mobs, hit, &mut actor, attack)
        } else {
            crate::mob_actions::interact(&mut self.mobs, hit, &mut actor)
        };
        for (stack, position) in outcome.drops {
            let components = stack.components.as_ref().map(|c| c.to_string());
            let stack = stack_of(&stack.id, i32::from(stack.count), components.as_deref());
            self.level.spawn_at_location(position.to_array(), stack);
        }
        for (position, amount) in outcome.experience {
            self.level.award_experience(position.to_array(), amount);
        }
        let slots = inventory
            .slots
            .iter()
            .zip(&before)
            .enumerate()
            .filter(|(_, (now, was))| now != was)
            .map(|(slot, (now, _))| (slot, now.clone()))
            .collect();
        self.spawn_trade_experience();
        MobResult {
            sounds: outcome.sounds,
            slots,
            merchant: self.merchant_view(0),
        }
    }

    /// The player's trading screen as the client shows it.
    pub fn merchant_view(&mut self, player: u64) -> Option<MerchantView> {
        let menu = self.mobs.merchant_menu(player)?.clone();
        let offers = self
            .mobs
            .villager_offers(menu.villager)
            .map(<[_]>::to_vec)
            .unwrap_or_default();
        let villager = self
            .mobs
            .villagers()
            .iter()
            .find(|e| e.id == menu.villager)?;
        let mut max_stacks = Vec::new();
        for offer in &offers {
            for id in std::iter::once(&offer.buy.id)
                .chain(offer.buy_b.as_ref().map(|b| &b.id))
                .chain(std::iter::once(&offer.sell.id))
            {
                if !max_stacks
                    .iter()
                    .any(|(item, _): &(String, i32)| item == id)
                {
                    max_stacks.push((id.clone(), self.mobs.item_max_stack(id)));
                }
            }
        }
        Some(MerchantView {
            villager: menu.villager,
            profession: villager.villager.profession.id(),
            level: villager.villager.level,
            xp: villager.villager.xp,
            offers,
            payment: menu.payment.clone(),
            result: menu.result.clone(),
            future_xp: menu.future_xp,
            max_stacks,
        })
    }

    /// An operation on the player's trading screen, with a copy of the
    /// player's inventory (and the hotbar slot selected): the screen's new
    /// state, the slots it changed and the cursor. Closing returns the
    /// payments and the cursor to the inventory; what does not fit drops
    /// at the player's feet.
    pub fn merchant(
        &mut self,
        op: MerchantOp,
        mut inventory: minecraftoss_player::inventory::Inventory,
        selected: usize,
        feet: [f64; 3],
    ) -> MerchantUpdate {
        let before = inventory.slots.clone();
        match op {
            MerchantOp::Select(index) => self.mobs.merchant_select(0, index, &mut inventory),
            MerchantOp::Payment { slot, right, shift } => {
                self.mobs
                    .merchant_click_payment(0, slot, right, shift, &mut inventory)
            }
            MerchantOp::Result { shift } => {
                self.mobs.merchant_click_result(0, shift, &mut inventory)
            }
            MerchantOp::Close => {
                for stack in self.mobs.merchant_close(0, &mut inventory, selected) {
                    let components = stack.components.as_ref().map(|c| c.to_string());
                    self.level.spawn_at_location(
                        feet,
                        stack_of(&stack.id, i32::from(stack.count), components.as_deref()),
                    );
                }
                self.merchant_closing = false;
            }
        }
        self.spawn_trade_experience();
        let slots = inventory
            .slots
            .iter()
            .zip(&before)
            .enumerate()
            .filter(|(_, (now, was))| now != was)
            .map(|(slot, (now, _))| (slot, now.clone()))
            .collect();
        MerchantUpdate {
            view: self.merchant_view(0),
            slots,
            cursor: inventory.cursor.clone(),
            closing: false,
        }
    }

    /// `ServerPlayer.tick`'s `stillValid` check on an open trading screen:
    /// once the villager is out of reach, dead or done trading, the client
    /// is asked (once) to close it.
    fn check_merchant(
        &mut self,
        players: &[minecraftoss_entities::tempt::PlayerCandidate],
    ) -> Option<MerchantUpdate> {
        if self.merchant_closing || self.mobs.merchant_menu(0).is_none() {
            return None;
        }
        let player = players.iter().find(|p| p.id == 0)?;
        let eyes = player.position + glam::DVec3::Y * f64::from(player.eye_height);
        if self.mobs.merchant_still_valid(0, eyes) {
            return None;
        }
        self.merchant_closing = true;
        Some(MerchantUpdate {
            view: None,
            slots: Vec::new(),
            cursor: None,
            closing: true,
        })
    }

    /// The orbs trades dropped, each whole into the level.
    fn spawn_trade_experience(&mut self) {
        for (position, value) in self.mobs.take_trade_experience() {
            self.level.spawn_experience_orb(position.to_array(), value);
        }
    }

    /// Where the level's entities are saved (the chunk map's storage).
    pub fn set_entity_storage(
        &mut self,
        storage: Option<Arc<minecraftoss_world::storage::ChunkStorage>>,
    ) {
        self.storage = storage;
    }

    /// A chunk joins the level with its entities (`EntityStorage`): those
    /// saved for it, or for a chunk the level never saved, the ones
    /// generation made. A chunk sent again keeps the entities it has.
    pub fn load_chunk(&mut self, chunk: &Chunk) {
        let fresh = self.level.chunk(chunk.pos).is_none();
        self.level.insert_chunk(chunk.clone());
        if !fresh {
            return;
        }
        self.load_pois(chunk);
        let saved = self.storage.as_ref().and_then(|storage| {
            storage.load_entities(chunk.pos).unwrap_or_else(|e| {
                eprintln!("entities of chunk {:?} failed to load: {e}", chunk.pos);
                None
            })
        });
        for tag in saved.unwrap_or_else(|| chunk.generation.entities.clone()) {
            self.add_saved_entity(tag);
        }
    }

    /// Re-streaming must carry the same edit overlay as rendering and Jak
    /// collision. The generated chunk alone is an older, unedited snapshot.
    pub fn load_chunk_with_edits(&mut self, chunk: &Chunk, edits: &[(BlockPos, Option<Block>)]) {
        self.load_chunk(chunk);
        for (pos, block) in edits { self.player_edit_block(*pos, block.as_ref(), PlayerEdit::Place); }
    }

    /// A chunk's points of interest (`PoiManager.checkConsistencyWithBlocks`
    /// for each section that may hold one).
    fn load_pois(&mut self, chunk: &Chunk) {
        let table = &self.mob_tables.pois;
        let kind = |state: BlockStateId| table.get(state.0 as usize).copied().flatten();
        for (i, section) in chunk.sections().iter().enumerate() {
            let sy = chunk.min_section_y() + i as i32;
            let maybe = match &section.blocks {
                minecraftoss_core::palette::PalettedContainer::Single(state) => {
                    kind(*state).is_some()
                }
                minecraftoss_core::palette::PalettedContainer::Direct(values) => {
                    values.iter().any(|&s| kind(s).is_some())
                }
            };
            if maybe {
                self.mobs
                    .pois
                    .load_section((chunk.pos.x, sy, chunk.pos.z), |(x, y, z)| {
                        kind(section.block((x & 15) as usize, (y & 15) as usize, (z & 15) as usize))
                    });
            }
        }
    }

    /// The blocks set since the last call move the points of interest
    /// (`ServerLevel.updatePOIOnBlockStateChange`).
    fn sync_pois(&mut self) {
        for (x, y, z) in self.level.take_block_log() {
            let state = self.level.block(minecraftoss_core::BlockPos::new(x, y, z));
            let new = self
                .mob_tables
                .pois
                .get(state.0 as usize)
                .copied()
                .flatten();
            let old = self.mobs.pois.kind((x, y, z));
            self.mobs.pois.block_changed((x, y, z), old, new);
        }
    }

    /// A saved entity joins the level: items and experience orbs as level
    /// entities, mobs the entity world simulates into it, and the rest
    /// dormant.
    fn add_saved_entity(&mut self, tag: minecraftoss_core::nbt::Tag) {
        self.mobs.set_day_time(self.level.overworld_clock());
        if crate::server_mobs::load_level_entity(&mut self.level, &tag) {
            return;
        }
        match crate::server_mobs::spawn_saved(&mut self.mobs, &tag) {
            Some(id) => {
                if let Some(name) = custom_name(&tag) {
                    self.names.insert(id, name);
                    self.names_dirty = true;
                }
                self.mob_tags.insert(id, tag);
            }
            None => self.dormant.push((
                minecraftoss_world::natural_spawner::tick::census_of(&tag),
                tag,
            )),
        }
    }

    /// Saves a chunk's entities: its mobs (from the tag each came from,
    /// with what the entity world changed), its dormant mobs as they came,
    /// and its items and experience orbs. Unloading also takes them out.
    fn save_chunk_entities(&mut self, pos: ChunkPos, unload: bool) {
        let inside = |p: glam::DVec3| crate::server_mobs::in_chunk(p, pos);
        let mut tags: Vec<minecraftoss_core::nbt::Tag> =
            crate::server_mobs::mob_tags(&self.mobs, inside, &self.mob_tags)
                .into_iter()
                .map(|(_, tag)| tag)
                .collect();
        tags.extend(
            self.dormant
                .iter()
                .filter(|(group, _)| inside(glam::DVec3::from_array(group[0].pos)))
                .map(|(_, tag)| tag.clone()),
        );
        tags.extend(crate::server_mobs::level_entity_tags(&self.level, inside));
        if unload {
            self.mobs.remove_where(inside);
            self.dormant
                .retain(|(group, _)| !inside(glam::DVec3::from_array(group[0].pos)));
            self.level.entities.retain(|e| {
                !(inside(glam::DVec3::from_array(e.pos))
                    && (e.item_data().is_some() || e.orb_data().is_some()))
            });
            let alive: std::collections::HashSet<u64> = self.mobs.mob_ids().collect();
            self.mob_tags.retain(|id, _| alive.contains(id));
            let named = self.names.len();
            self.names.retain(|id, _| alive.contains(id));
            self.names_dirty |= self.names.len() != named;
        }
        if let Some(storage) = &self.storage {
            if let Err(e) = storage.save_entities(pos, &tags) {
                eprintln!("entities of chunk {pos:?} failed to save: {e}");
            }
        }
    }

    /// Saves every loaded chunk's entities and writes the entity storage
    /// (autosave and shutdown).
    pub fn save_all_entities(&mut self) {
        let loaded: Vec<ChunkPos> = self.level.chunks().map(|c| c.pos).collect();
        for pos in loaded {
            self.save_chunk_entities(pos, false);
        }
        if let Some(storage) = &self.storage {
            if let Err(e) = storage.flush() {
                eprintln!("entity storage write failed: {e}");
            }
        }
    }

    /// Before the level ticks: natural spawning's players (registered with
    /// its spawn counter), the mob census for its caps, and the simulation
    /// area.
    pub fn prepare_spawning(&mut self, players: &[minecraftoss_entities::tempt::PlayerCandidate]) {
        let Some(natural) = &mut self.level.natural_spawning else {
            return;
        };
        let spawn_players: Vec<SpawnPlayer> = players
            .iter()
            .map(|p| SpawnPlayer {
                pos: p.position.to_array(),
                spectator: p.spectator,
            })
            .collect();
        natural.set_players(&spawn_players);
        let mut census: Vec<CensusMob> = self
            .mobs
            .census()
            .into_iter()
            .map(|m| {
                let half = f64::from(m.width / 2.0);
                let p = m.position;
                CensusMob {
                    kind: m.kind.to_owned(),
                    pos: p.to_array(),
                    bb: [
                        p.x - half,
                        p.y,
                        p.z - half,
                        p.x + half,
                        p.y + f64::from(m.height),
                        p.z + half,
                    ],
                    persistent: m.persistent,
                    riding: false,
                }
            })
            .collect();
        census.extend(
            self.dormant
                .iter()
                .flat_map(|(group, _)| group.iter().cloned()),
        );
        natural.census = census;
        natural.simulation = self
            .level
            .ticking_chunks
            .iter()
            .flatten()
            .copied()
            .collect();
    }

    /// Mobs natural spawning made join the entity world; types it does not
    /// simulate yet (and jockeys) wait dormant: they count toward the caps
    /// and block spawns until they despawn.
    fn take_spawned(&mut self) {
        for tag in std::mem::take(&mut self.level.spawned) {
            self.add_saved_entity(tag);
        }
    }

    /// `Mob.checkDespawn` for the mobs in loaded chunks, before they tick.
    fn despawn_mobs(
        &mut self,
        players: &[minecraftoss_entities::tempt::PlayerCandidate],
        difficulty: i32,
    ) {
        let feet: Vec<glam::DVec3> = players
            .iter()
            .filter(|p| !p.spectator)
            .map(|p| p.position)
            .collect();
        let level = &self.level;
        let loaded = |p: glam::DVec3| {
            level
                .chunk(ChunkPos::new(
                    (p.x.floor() as i32) >> 4,
                    (p.z.floor() as i32) >> 4,
                ))
                .is_some()
        };
        self.mobs.check_despawn(&feet, difficulty == 0, &loaded);
        // Dormant monsters go in peaceful and beyond 128 blocks (they never
        // idle: they do not tick).
        self.dormant.retain(|(group, _)| {
            let root = &group[0];
            if root.persistent
                || minecraftoss_world::natural_spawner::spawning::MobCategory::of_type(&root.kind)
                    != Some(minecraftoss_world::natural_spawner::spawning::MobCategory::Monster)
            {
                return true;
            }
            if difficulty == 0 {
                return false;
            }
            let at = glam::DVec3::from_array(root.pos);
            feet.iter()
                .map(|p| p.distance_squared(at))
                .min_by(f64::total_cmp)
                .is_none_or(|d| d <= 128.0 * 128.0)
        });
    }

    /// The mobs spawned that the entity world does not simulate.
    pub fn dormant_count(&self) -> usize {
        self.dormant.len()
    }

    /// The recipe book mobs consult (breeding colours).
    pub fn set_recipe_book(&mut self, recipes: Arc<minecraftoss_player::crafting::RecipeBook>) {
        self.recipe_book = Some(recipes.clone());
        self.mobs.set_recipe_book(recipes);
    }

    /// Mobs tick after the level, inside the entity-ticking range (the
    /// level's ticking chunks, `DistanceManager.inEntityTickingRange`).
    pub fn tick_mobs(
        &mut self,
        players: &[minecraftoss_entities::tempt::PlayerCandidate],
        bright_outside: bool,
    ) {
        self.sync_pois();
        let ticking: std::collections::HashSet<ChunkPos> = self
            .level
            .ticking_chunks
            .iter()
            .flatten()
            .copied()
            .collect();
        self.mobs.set_bright_outside(bright_outside);
        self.mobs.set_monsters_burn(self.level.monsters_burn());
        let Self {
            level,
            states,
            mobs,
            mob_tables,
            lifts: self_lifts,
            seats: self_seats,
            ..
        } = self;
        let mut world = crate::server_mobs::MobWorld {
            level: std::cell::RefCell::new(level),
            states,
            tables: mob_tables,
        };
        let ticks = |p: glam::DVec3| {
            ticking.contains(&ChunkPos::new(
                (p.x.floor() as i32) >> 4,
                (p.z.floor() as i32) >> 4,
            ))
        };
        // Brains draw from the level's own random, and villagers keep the
        // overworld clock's schedule.
        mobs.set_day_time(world.level.borrow().overworld_clock());
        let shared = match &world.level.borrow().random {
            minecraftoss_core::random::AnyRandom::Legacy(random) => {
                Some((random.state(), random.gaussian_cache()))
            }
            _ => None,
        };
        if let Some((state, _)) = shared {
            *mobs.level_random_mut() =
                minecraftoss_player::rng::LegacyRandom::from_raw_state(state);
        }
        // Mobs the Mass Inverter holds float where they are (below).
        let lifted: Vec<(u64, glam::DVec3)> = self_lifts
            .keys()
            .filter_map(|&id| mobs.body_mut(id).map(|b| (id, b.position)))
            .collect();
        mobs.tick_with_players_where(&mut world, players, &ticks);
        float_lifted(mobs, &world, self_lifts, &lifted);
        // Riding: on their seat, still (`Entity.rideTick`).
        self_seats.retain(|&id, seat| {
            let Some(body) = mobs.body_mut(id) else { return false };
            body.position = *seat;
            body.velocity = glam::DVec3::ZERO;
            body.fall_distance = 0.0;
            body.on_ground = true;
            body.needs_sync = true;
            true
        });
        // Births and summons are listed for replays that tag them; the game
        // needs no list.
        let _ = (
            mobs.take_born_villagers(),
            mobs.take_born_wolves(),
            mobs.take_summoned_golems(),
        );
        if let Some((_, gaussian)) = shared {
            if let minecraftoss_core::random::AnyRandom::Legacy(random) =
                &mut world.level.borrow_mut().random
            {
                random.set_seed((mobs.level_random_mut().raw_state() ^ 0x5DEE_CE66D) as i64);
                random.set_gaussian_cache(gaussian);
            }
        }
        // `ServerPlayer.doTick` (the connection tick, after the levels):
        // the players push the mobs they walk into.
        mobs.push_from_players(players, &ticks);
        // Entity events (a villager's hearts, anger, happiness) are for the
        // client's particles, which it does not draw yet.
        let _ = mobs.take_entity_events();
        // A blast's victims drop their loot before its blocks break
        // (`ServerExplosion.explode`: `hurtEntities`, then the blocks).
        self.drop_death_loot();
        self.explode_creepers();
        // The orbs bred animals dropped.
        self.spawn_trade_experience();
    }

    /// The loot and experience of the mobs that died, into the level as
    /// item entities and experience orbs (`dropAllDeathLoot`).
    fn drop_death_loot(&mut self) {
        let (drops, experience) =
            crate::mob_actions::death_remains(&mut self.mobs, self.entity_loot.as_mut());
        for (stack, position) in drops {
            let components = stack.components.as_ref().map(|c| c.to_string());
            let stack = stack_of(&stack.id, i32::from(stack.count), components.as_deref());
            self.level.spawn_at_location(position.to_array(), stack);
        }
        for (position, amount) in experience {
            self.level.award_experience(position.to_array(), amount);
        }
    }

    /// `Player.tick`'s `takeXpDelay` countdown, then `Player.aiStep` touching
    /// one orb (at random) when the player at `feet` can take one: returns
    /// the orb's ID, position and value.
    pub fn take_experience(&mut self, feet: Option<[f64; 3]>) -> Option<(i32, [f64; 3], i32)> {
        if self.take_xp_delay > 0 {
            self.take_xp_delay -= 1;
        }
        if self.take_xp_delay != 0 {
            return None;
        }
        let taken = self.level.player_touch_orb(feet?)?;
        self.take_xp_delay = 2;
        Some(taken)
    }

    /// The server's experience orbs.
    pub fn orbs(&self) -> Vec<ServerOrb> {
        self.level
            .entities
            .iter()
            .filter_map(|e| {
                e.orb_data().map(|d| ServerOrb {
                    id: e.id,
                    position: e.pos,
                    previous_position: e.old_position(),
                    value: d.value,
                    tick_count: e.tick_count,
                })
            })
            .collect()
    }

    /// A mob named (a name tag): it never despawns.
    pub fn require_persistence(&mut self, id: u64) {
        self.mobs.require_persistence(id);
    }

    /// A mob named with a name tag: kept from despawning, and its name
    /// saved with it as vanilla's `CustomName` (17/10: names were lost on
    /// reload).
    pub fn name(&mut self, id: u64, name: String) {
        self.mobs.require_persistence(id);
        let tag = self
            .mob_tags
            .entry(id)
            .or_insert_with(|| minecraftoss_core::nbt::Tag::Compound(Default::default()));
        if let minecraftoss_core::nbt::Tag::Compound(map) = tag {
            map.insert("CustomName".into(), minecraftoss_core::nbt::Tag::String(name.clone()));
        }
        self.names.insert(id, name);
        self.names_dirty = true;
    }

    /// The names, when they changed since the last call.
    fn take_names(&mut self) -> Option<Vec<(u64, String)>> {
        if !std::mem::take(&mut self.names_dirty) {
            return None;
        }
        Some(self.names.iter().map(|(id, name)| (*id, name.clone())).collect())
    }

    /// A mob sat on a seat (or let go: none).
    pub fn seat(&mut self, id: u64, at: Option<[f64; 3]>) {
        match at {
            Some(at) => {
                self.seats.insert(id, glam::DVec3::from_array(at));
            }
            None => {
                self.seats.remove(&id);
            }
        }
    }

    /// The host's care of a mob: healed up to its maximum (`heal` clamps),
    /// an effect added as `addEffect` would, and `igniteForSeconds` (fire
    /// ticks never shortened).
    pub fn mob_care(&mut self, id: u64, heal: bool, effect: Option<minecraftoss_entities::effects::EffectInstance>, ignite_ticks: i32) {
        if heal {
            self.mobs.heal_mob_by(id, f32::MAX);
        }
        if let Some(effect) = effect {
            self.mobs.add_mob_effect(id, effect);
        }
        if ignite_ticks > 0 {
            if let Some(body) = self.mobs.body_mut(id) {
                body.fire_ticks = body.fire_ticks.max(ignite_ticks);
            }
        }
    }

    /// A host contact uses authoritative block shapes, not damage or a
    /// teleport/seat override. Motion is blocks per tick.
    pub fn contact(&mut self, id: u64, delta: [f64; 3], velocity: [f64; 3]) {
        if delta.iter().chain(&velocity).any(|v| !v.is_finite()) || self.seats.contains_key(&id) {
            return;
        }
        let Some(body) = self.mobs.body_mut(id) else {
            return;
        };
        let world = crate::server_mobs::MobWorld {
            level: std::cell::RefCell::new(&mut self.level),
            states: &self.states,
            tables: &self.mob_tables,
        };
        let bounds =
            minecraftoss_player::collision::Bounds::standing(body.position, body.width, body.height);
        body.position += minecraftoss_player::collision::move_body(
            &world,
            bounds,
            glam::DVec3::from_array(delta),
            body.on_ground,
            0.0,
        );
        body.velocity += glam::DVec3::from_array(velocity);
        body.needs_sync = true;
    }

    /// Experience at a position (`ExperienceOrb.award`), as orbs.
    pub fn award_experience(&mut self, position: [f64; 3], amount: i32) {
        self.level.award_experience(position, amount);
    }

    /// The Mass Inverter takes a mob's gravity for `seconds`.
    pub fn lift(&mut self, id: u64, seconds: f32) {
        let ticks = (seconds.max(0.0) * 20.0).round() as u32;
        let left = self.lifts.entry(id).or_insert(0);
        *left = (*left).max(ticks);
    }

    /// `summon minecraft:experience_orb` with its `Value`: a still orb.
    pub fn summon_orb(&mut self, position: [f64; 3], value: i32) {
        self.level
            .add_entity(minecraftoss_world::level::entity::Entity::experience_orb(
                position, value,
            ));
    }

    /// `/summon` for a mob: the level makes its tag (`SummonCommand`), and
    /// it joins the entity world as a loaded mob would.
    pub fn summon(
        &mut self,
        kind: &str,
        position: [f64; 3],
        nbt: Option<&minecraftoss_core::nbt::Tag>,
        y_rot: f32,
    ) -> Result<(), String> {
        let tag = self.level.summon_mob(kind, position, nbt, y_rot)?;
        // A new brain reads the schedule for the time it is.
        self.mobs.set_day_time(self.level.overworld_clock());
        self.add_saved_entity(tag);
        Ok(())
    }

    /// Creeper blasts reach the level (`ServerLevel.explode` with
    /// `ExplosionInteraction.MOB`): blocks break when mobs may grief, with
    /// the drops decaying (`mob_explosion_drop_decay`), and items, TNT and
    /// falling blocks are hurt and pushed. The entity world already hurt
    /// its mobs and the players.
    fn explode_creepers(&mut self) {
        use minecraftoss_world::level::explosion::{BlockInteraction, Explosion};
        for blast in self.mobs.take_creeper_explosions() {
            let interaction = if self.mobs.mob_griefing() {
                BlockInteraction::DestroyWithDecay
            } else {
                BlockInteraction::Keep
            };
            self.level.explode(Explosion {
                center: blast.position.to_array(),
                radius: blast.radius,
                fire: false,
                interaction,
                source: None,
            });
            self.explosions.push(blast);
        }
    }

    /// The blasts since the last call, for the client's sound and particles.
    pub fn take_explosions(&mut self) -> Vec<minecraftoss_entities::creeper::CreeperExplosion> {
        std::mem::take(&mut self.explosions)
    }

    /// The mobs a player at `position` tracks (`ChunkMap.TrackedEntity`:
    /// within `range` blocks horizontally).
    pub fn tracked_mobs(
        &self,
        position: [f64; 3],
        range: f64,
    ) -> minecraftoss_entities::world::EntityWorld {
        let range_sq = range * range;
        self.mobs.clone_where(|p| {
            let (dx, dz) = (p.x - position[0], p.z - position[2]);
            dx * dx + dz * dz <= range_sq
        })
    }

    /// A chunk leaves the level; its entities are saved and taken out.
    pub fn unload_chunk(&mut self, pos: ChunkPos) {
        self.save_chunk_entities(pos, true);
        self.level.remove_chunk(pos);
        self.mobs.pois.unload_chunk(pos.x, pos.z);
    }

    fn state_of(&self, block: Option<&Block>) -> BlockStateId {
        block
            .and_then(|b| self.states.state_of(b))
            .unwrap_or(BlockStateId::AIR)
    }

    /// Applies a player's edit that the client scene already shows.
    pub fn player_edit(&mut self, scene: &HandcraftedScene, pos: BlockPos, edit: PlayerEdit) {
        self.player_edit_block(pos, Scene::block(scene, pos), edit);
    }

    /// A player's edit to `block` (`None` for air) at a position.
    pub fn player_edit_block(&mut self, pos: BlockPos, block: Option<&Block>, edit: PlayerEdit) {
        let target = self.state_of(block);
        let at = minecraftoss_core::BlockPos::new(pos.0, pos.1, pos.2);
        if self.level.block(at) == target {
            return;
        }
        let flags = match edit {
            PlayerEdit::Place => update::ALL | 8,
            PlayerEdit::Break => update::ALL,
        };
        self.level.set_block(at, target, flags, update::LIMIT);
        // Its placement state: a wall, pane or fence joined to what is round
        // it, a door's upper half (`Level::settle_placed`).
        if edit == PlayerEdit::Place {
            self.level.settle_placed(at);
        }
    }

    /// `useWithoutItem` on a simulated block; false when not simulated.
    pub fn use_block(&mut self, pos: BlockPos, player_facing: &str) -> bool {
        let facing = minecraftoss_core::pos::Direction::from_name(player_facing);
        self.level.use_block_facing(
            minecraftoss_core::BlockPos::new(pos.0, pos.1, pos.2),
            facing,
        )
    }

    /// `BoneMealItem.useOn` on a clicked face: grow the block, or water
    /// plants in front of a sturdy face. True when the bone meal is used.
    pub fn bone_meal(&mut self, pos: BlockPos, face: &str) -> bool {
        let at = minecraftoss_core::BlockPos::new(pos.0, pos.1, pos.2);
        if self.level.grow_crop(at) {
            return true;
        }
        let Some(face) = minecraftoss_core::pos::Direction::from_name(face) else {
            return false;
        };
        let state = self.level.block(at);
        self.level.registries().blocks.is_face_sturdy(
            state,
            face,
            minecraftoss_core::SupportType::Full,
        ) && self.level.grow_water_plant(at.relative(face, 1))
    }

    pub fn attack_block(&mut self, pos: BlockPos) -> bool {
        self.level
            .attack_block(minecraftoss_core::BlockPos::new(pos.0, pos.1, pos.2))
    }

    /// The players the level can see (fire spreads near them), and the
    /// difficulty (`Difficulty.getId`).
    pub fn set_players(&mut self, positions: &[[f64; 3]], difficulty: i32) {
        self.level.players = positions.to_vec();
        self.level.difficulty = difficulty;
    }

    /// The chunks random ticks run in: every loaded chunk within
    /// `distance` (Chebyshev) of `center`, in X then Z order.
    pub fn set_simulation_area(&mut self, center: (i32, i32), distance: i32) {
        let mut list = Vec::with_capacity(((2 * distance + 1) * (2 * distance + 1)) as usize);
        for x in center.0 - distance..=center.0 + distance {
            for z in center.1 - distance..=center.1 + distance {
                let pos = ChunkPos::new(x, z);
                if self.level.chunk(pos).is_some() {
                    list.push(pos);
                }
            }
        }
        self.level.ticking_chunks = Some(list);
    }

    /// Keeps the level's default clock with the client's day time.
    pub fn set_host_weather(&mut self,weather:Option<HostWeather>){
        self.level.external_weather=weather.is_some();
        let Some(input)=weather else{return;};let Some(sky)=self.level.sky.as_mut()else{return;};
        let unit=|x:f32|if x.is_finite(){x.clamp(0.0,1.0)}else{0.0};
        let w=&mut sky.weather;
        w.raining=sky.can_have_weather&&input.raining;w.thundering=sky.can_have_weather&&input.thundering;
        w.rain_level=if sky.can_have_weather{unit(input.rain_level)}else{0.0};
        w.thunder_level=if sky.can_have_weather{unit(input.thunder_level)}else{0.0};
    }

    pub fn set_time(&mut self, ticks: i64) {
        self.level.set_time(ticks);
    }

    pub fn tick(&mut self) {
        self.level.tick();
        self.level.unsupported.clear();
    }

    /// Hands an item entity the client created (a drop, a spill) to the
    /// server, which simulates it from then on; returns its entity ID.
    #[allow(clippy::too_many_arguments)]
    pub fn spawn_item(
        &mut self,
        item: &str,
        count: i32,
        components: Option<&str>,
        position: [f64; 3],
        velocity: [f64; 3],
        pickup_delay: i32,
        age: i32,
    ) -> i32 {
        self.level.spawn_item_with(
            position,
            stack_of(item, count, components),
            velocity,
            pickup_delay,
            age,
        )
    }

    /// The server's item entities.
    pub fn items(&self) -> Vec<ServerItem> {
        self.level
            .entities
            .iter()
            .filter_map(|e| e.item_data().map(|d| (e, d)))
            .map(|(e, d)| ServerItem {
                id: e.id,
                item: d.stack.id.clone(),
                count: d.stack.count,
                components: components_of(&d.stack),
                position: e.pos,
                previous_position: e.old_position(),
                velocity: e.delta,
                age: d.age,
                pickup_delay: d.pickup_delay,
                skull_gem: d.skull_gem,
                on_ground: e.on_ground,
            })
            .collect()
    }

    /// The server's falling blocks.
    pub fn falling_blocks(&self) -> Vec<ServerFallingBlock> {
        self.level
            .entities
            .iter()
            .filter_map(|e| {
                e.falling_data().map(|d| ServerFallingBlock {
                    position: e.pos,
                    previous_position: e.old_position(),
                    block: self
                        .level
                        .registries()
                        .blocks
                        .block(self.level.registries().blocks.block_of(d.state))
                        .name
                        .to_string(),
                })
            })
            .collect()
    }

    /// The server's primed TNT.
    pub fn primed_tnt(&self) -> Vec<ServerTnt> {
        self.level
            .entities
            .iter()
            .filter_map(|e| {
                e.tnt_data().map(|d| ServerTnt {
                    position: e.pos,
                    previous_position: e.old_position(),
                    fuse: d.fuse,
                })
            })
            .collect()
    }

    /// `ItemEntity.playerTouch` for a player standing at `feet`: `take`
    /// offers each touching stack (item, count, components) and returns how
    /// many the inventory took. Returns (entity ID, position, item, count,
    /// components) per pickup.
    pub fn pickup(
        &mut self,
        feet: [f64; 3],
        mut take: impl FnMut(&str, i32, Option<&str>) -> i32,
    ) -> Vec<(i32, [f64; 3], String, i32, Option<String>)> {
        let mut picked: Vec<_> = self.level
            .player_touch_items(feet, |stack| {
                let components = components_of(stack);
                take(&stack.id, stack.count, components.as_deref())
            })
            .into_iter()
            .map(|(id, position, stack)| {
                let components = components_of(&stack);
                (id, position, stack.id, stack.count, components)
            })
            .collect();
        for flight in &mut self.arrows {
            if !flight.can_pick_up(feet) { continue; }
            let components = flight.request.components.as_ref().map(|c| c.to_string());
            if take(&flight.request.item, 1, components.as_deref()) == 1 {
                flight.body.alive = false;
                picked.push((flight.pickup_id, flight.body.position.to_array(), flight.request.item.clone(), 1, components));
            }
        }
        self.arrows.retain(|f| f.body.alive);
        picked
    }

    /// A hopper minecart at `feet` (blocks) taking what lies over it into
    /// `slots` as far as they hold it: the box a hopper sucks from
    /// (`Hopper.SUCK_AABB`, from 11/16 to two blocks up, moved to the cart's
    /// `getLevelY`, half a block over its feet).
    pub fn hopper_pickup(
        &mut self,
        feet: [f64; 3],
        slots: &[Option<minecraftoss_player::inventory::ItemStack>],
    ) -> Vec<(String, i32, Option<String>)> {
        let mut room = slots.to_vec();
        let book = self.recipe_book.clone();
        let self_max = |id: &str| book.as_ref().map_or(64, |b| b.max_stack(id));
        let min = [feet[0] - 0.5, feet[1] + 0.5 - 0.5 + 0.6875, feet[2] - 0.5];
        let max = [feet[0] + 0.5, feet[1] + 0.5 - 0.5 + 2.0, feet[2] + 0.5];
        self.level
            .touch_items_in(min, max, |stack| {
                let components = components_of(stack);
                let max = self_max(&stack.id);
                let mut left = stack.count;
                for slot in room.iter_mut() {
                    if left == 0 {
                        break;
                    }
                    match slot {
                        Some(s) if s.id == stack.id && s.components.as_ref().map(|c| c.to_string()) == components => {
                            let fit = (i32::from(s.max) - i32::from(s.count)).clamp(0, left);
                            s.count += fit as u8;
                            left -= fit;
                        }
                        None => {
                            let fit = left.min(i32::from(max));
                            let mut s = minecraftoss_player::inventory::ItemStack::new(&stack.id, fit as u8);
                            s.max = max;
                            *slot = Some(s);
                            left -= fit;
                        }
                        _ => {}
                    }
                }
                stack.count - left
            })
            .into_iter()
            .map(|(_, _, stack)| {
                let components = components_of(&stack);
                (stack.id.clone(), stack.count, components)
            })
            .collect()
    }

    /// Positions changed since the last call, with the block now there.
    pub fn take_changes(&mut self) -> Vec<(BlockPos, Option<Block>)> {
        let changed = self.level.take_changed();
        if std::env::var_os("MINECRAFTOSS_DEBUG_SERVER").is_some() && !changed.is_empty() {
            eprintln!(
                "server changes at tick {}: {} (first {:?})",
                self.level.game_time,
                changed.len(),
                &changed[..changed.len().min(4)]
            );
        }
        changed
            .into_iter()
            .map(|(x, y, z)| {
                let state = self.level.block(minecraftoss_core::BlockPos::new(x, y, z));
                let block = if self.level.registries().blocks.is_air(state) {
                    None
                } else {
                    self.states.block(state).cloned()
                };
                ((x, y, z), block)
            })
            .collect()
    }
}


/// Jak's Mass Inverter (`gravity-spinner`'s zero g and `get-float-speed`):
/// a lifted mob stands still where its brain would walk it, drifts with the
/// blows it takes, rises to a metre over the ground, hangs between one and
/// three and a half, and sinks back above that. When its time is up it is
/// let go to fall.
fn float_lifted(
    mobs: &mut minecraftoss_entities::world::EntityWorld,
    world: &crate::server_mobs::MobWorld<'_>,
    lifts: &mut std::collections::HashMap<u64, u32>,
    before: &[(u64, glam::DVec3)],
) {
    use minecraftoss_player::World as PlayerWorld;
    const TICK: f64 = 1.0 / 20.0;
    for &(id, start) in before {
        let Some(body) = mobs.body_mut(id) else {
            lifts.remove(&id);
            continue;
        };
        // Height over the ground under it, up to eight blocks down.
        let (x, z) = (start.x.floor() as i32, start.z.floor() as i32);
        let feet = start.y.floor() as i32;
        let ground = (0..8)
            .map(|d| feet - d)
            .find(|&y| !PlayerWorld::collision_boxes(world, (x, y, z)).is_empty())
            .map_or(start.y - 8.0, |y| f64::from(y + 1));
        let h = start.y - ground;
        let rise = if h < 1.0 {
            10.0 * (1.0 - h)
        } else if h <= 3.5 {
            0.0
        } else {
            // In Jak's units (4096 a metre), then back to metres.
            let over = (h - 3.5) * 4096.0;
            -10.0 * (0.000_081_4 * over * over).min(15.0 * 4096.0) / 4096.0
        };
        body.velocity.x *= 0.9;
        body.velocity.z *= 0.9;
        body.position = glam::DVec3::new(
            start.x + body.velocity.x,
            start.y + rise * TICK,
            start.z + body.velocity.z,
        );
        body.velocity.y = 0.0;
        body.fall_distance = 0.0;
        body.on_ground = false;
        body.needs_sync = true;
        if let Some(left) = lifts.get_mut(&id) {
            *left = left.saturating_sub(1);
            if *left == 0 {
                lifts.remove(&id);
            }
        }
    }
}
/// What the client asks of the integrated server thread, in order.
pub enum Command {
    /// Explicit barrier for hosts that exit without running destructors.
    Save,
    Throw(crate::thrown::Request),
    Contact{id:u64,delta:[f64;3],velocity:[f64;3]},
    /// What the host's world does to a mob outside its own rules (Jak's eco
    /// vents and shots): heal it whole, give it an effect, set it alight.
    MobCare{id:u64,heal:bool,effect:Option<minecraftoss_entities::effects::EffectInstance>,ignite_ticks:i32},
    LoadChunk(Arc<Chunk>),
    LoadEditedChunk { chunk: Arc<Chunk>, edits: Vec<(BlockPos, Option<Block>)> },
    /// A mob the Mass Inverter caught, and for how long.
    Lift { id: u64, seconds: f32 },
    /// A mob on a boat's or a minecart's seat, or let go.
    Seat { id: u64, at: Option<[f64; 3]> },
    /// A mob named: kept from despawning.
    Persist { id: u64 },
    /// A mob named with a name tag (kept, and its name saved).
    Name { id: u64, name: String },
    UnloadChunk(ChunkPos),
    /// A block the client set, as the client scene now shows it.
    PlayerEdit {
        pos: BlockPos,
        block: Option<Block>,
        edit: PlayerEdit,
    },
    UseBlock {
        pos: BlockPos,
        facing: &'static str,
    },
    BoneMeal {
        pos: BlockPos,
        face: &'static str,
    },
    Attack(BlockPos),
    SpawnItem {
        item: String,
        count: i32,
        components: Option<String>,
        position: [f64; 3],
        velocity: [f64; 3],
        pickup_delay: i32,
        age: i32,
    },
    /// The recipe book, for mobs.
    RecipeBook(Arc<minecraftoss_player::crafting::RecipeBook>),
    /// The data JAR and world seed the server's loot tables come from.
    LootTables {
        jar: std::path::PathBuf,
        seed: u64,
    },
    /// `summon minecraft:experience_orb`.
    SummonOrb {
        position: [f64; 3],
        value: i32,
    },
    /// `ExperienceOrb.award`: experience at a point, as vanilla's orbs.
    AwardExperience {
        position: [f64; 3],
        amount: i32,
    },
    /// `/summon` for a mob, with the command's NBT and the new mob's own
    /// random yaw.
    Summon {
        kind: String,
        position: [f64; 3],
        nbt: Option<minecraftoss_core::nbt::Tag>,
        y_rot: f32,
    },
    /// A player's hit (`attack`) or item use on a mob, with a copy of its
    /// inventory.
    MobAction {
        hit: minecraftoss_entities::world::MobHit,
        attack: Option<minecraftoss_entities::world::PlayerAttack>,
        inventory: Box<minecraftoss_player::inventory::Inventory>,
        selected: usize,
        infinite: bool,
    },
    /// An operation on the player's trading screen.
    Merchant {
        op: MerchantOp,
        inventory: Box<minecraftoss_player::inventory::Inventory>,
        selected: usize,
        feet: [f64; 3],
    },
    /// One server tick, with the player's state for it.
    Tick(Box<TickInput>),
}

/// One authoritative weather sample shared with the host sky, independent of
/// the renderer. Native levels without a host keep their own weather cycle.
#[derive(Clone,Copy,Debug,Default,PartialEq)]
pub struct HostWeather {pub raining:bool,pub thundering:bool,pub rain_level:f32,pub thunder_level:f32}

/// The client state a server tick reads.
pub struct TickInput {
    pub day_ticks: i64,
    pub weather: Option<HostWeather>,
    pub players: Vec<[f64; 3]>,
    pub difficulty: i32,
    pub simulation_center: (i32, i32),
    pub simulation_distance: i32,
    /// Hopper minecarts taking items (`MinecartHopper.suckInItems`): each
    /// cart's id, its feet (blocks) and its five slots as they are.
    pub hoppers: Vec<(u32, [f64; 3], Vec<Option<minecraftoss_player::inventory::ItemStack>>)>,
    /// Who [`Self::pickup`] is (local two-player: 0 or 1), handed back
    /// with what they took ([`Output::pickup_player`]).
    pub pickup_player: u64,
    /// The player's feet, when it can pick items up, with its inventory and
    /// selected slot (the server offers touching stacks to this copy).
    pub pickup: Option<(
        [f64; 3],
        Box<minecraftoss_player::inventory::Inventory>,
        usize,
    )>,
    /// The player as mobs see it (held food, whether it can be targeted).
    pub mob_players: Vec<minecraftoss_entities::tempt::PlayerCandidate>,
    /// Where each of those players looks (endermen read stares from it).
    pub mob_views: Vec<(u64, minecraftoss_entities::enderman::PlayerView)>,
    /// Each of those players' health, effects and motion (witches pick
    /// their potions by them).
    pub mob_vitals: Vec<(u64, minecraftoss_entities::monster_ai::PlayerVitals)>,
    /// `Level.isBrightOutside`.
    pub bright_outside: bool,
    /// The player's position and how far away it tracks mobs.
    pub tracking: ([f64; 3], f64),
    /// The hurts that landed on the player since the last tick: the mob
    /// behind each and the damage type (its tame wolves avenge it).
    pub player_hurts: Vec<(Option<u64>, &'static str)>,
    /// The `spawn_mobs` game rule.
    pub spawn_mobs: bool,
    /// Whether the entities move this tick: Light Jak's slowed time runs
    /// them on a clock of their own while the blocks keep theirs (16/10).
    pub entities_tick: bool,
    /// The Mass Inverter's fields (centre, radius, blocks): projectiles
    /// fly slow and weightless in them.
    pub projectile_fields: Vec<([f64; 3], f64)>,
    pub light_barriers: Vec<crate::light_barrier::Barrier>,
    /// What thrown items may strike besides the world's own entities.
    pub thrown_targets: Vec<crate::thrown::ExternalTarget>,
}

/// What the server thread sends back after handling commands.
#[derive(Default)]
pub struct Output {
    pub weather:Option<HostWeather>,
    pub drips:Vec<minecraftoss_world::level::dripstone::DripEffect>,
    /// Thrown tridents come down this tick.
    pub thrown_landed: Vec<crate::thrown::Landed>,
    pub shield_hits: Vec<u32>,
    /// Blocks that changed, with the block now there.
    pub changes: Vec<(BlockPos, Option<Block>)>,
    /// Entity snapshots, after a tick.
    pub entities: Option<EntitySnapshot>,
    /// Experience orbs the player took: (entity ID, position, value).
    pub orbs_taken: Vec<(i32, [f64; 3], i32)>,
    /// What hopper minecarts took: the cart's id, the item, how many and
    /// its components.
    pub hopper_picked: Vec<(u32, String, i32, Option<String>)>,
    /// The player whose pickup took them and `picked` (the tick's
    /// [`TickInput::pickup_player`]): with two players the answer may come
    /// on the other's frame.
    pub pickup_player: u64,
    /// The mobs the player tracks, after a tick.
    pub mobs: Option<Box<minecraftoss_entities::world::EntityWorld>>,
    /// What the player's mob actions did, in order.
    pub mob_results: Vec<MobResult>,
    /// The trading screen's updates, in order.
    pub merchant: Vec<MerchantUpdate>,
    /// Mobs' hits on the players, in order.
    pub player_hits: Vec<minecraftoss_entities::world::PlayerHit>,
    /// Splash potions that broke near players, in order.
    pub player_splashes: Vec<(u64, minecraftoss_entities::world::PlayerSplash)>,
    /// Where splash potions broke (their sound and colour).
    pub potion_breaks: Vec<minecraftoss_entities::world::PotionBreak>,
    /// Creeper blasts this tick (`ClientboundExplodePacket`).
    pub explosions: Vec<minecraftoss_entities::creeper::CreeperExplosion>,
    /// Sounds mobs made this tick.
    pub mob_sounds: Vec<minecraftoss_entities::world::MobSound>,
    /// Items the player picked up: (entity ID, position, item, count, components).
    pub picked: Vec<(i32, [f64; 3], String, i32, Option<String>)>,
    /// What each `/summon` did: the type it made, or why it failed.
    pub summoned: Vec<Result<String, String>>,
    /// Positions where bone meal was used.
    pub bone_meal_used: Vec<BlockPos>,
    /// How many commands the server has handled so far.
    pub handled: u64,
    pub thrown: Option<Vec<crate::thrown::View>>,
    pub arrows: Option<Vec<crate::arrow_render::ArrowView>>,
    /// Thrown items' blows on entities this tick, for the host's shared
    /// damage route.
    pub thrown_hits: Vec<crate::thrown::ThrownHit>,
    pub teleports: Vec<(u64, [f64; 3])>,
    pub clouds: Option<Vec<crate::thrown::CloudView>>,
    /// The last tick's phases, light solves and total milliseconds.
    pub tick_phases: Option<([f64; 6], (u32, f64), f64)>,
    /// Every named mob (entity ID, name), when the list changed.
    pub names: Option<Vec<(u64, String)>>,
}

/// The integrated server on its own thread, as vanilla runs it: the client
/// sends commands and never waits for a tick.
pub struct ServerHandle {
    commands: Option<std::sync::mpsc::Sender<Command>>,
    outputs: std::sync::mpsc::Receiver<Output>,
    states: Arc<BlockStates>,
    /// Per block state: whether the level acts on a use or an attack.
    uses: Arc<Vec<bool>>,
    attacks: Arc<Vec<bool>>,
    sent: u64,
    received: u64,
    /// Outputs [`Self::wait_idle`] received ahead of the next poll.
    waited: Vec<Output>,
    thread: Option<std::thread::JoinHandle<()>>,
}

impl ServerHandle {
    pub fn throw_item(&mut self, request: crate::thrown::Request) { self.send(Command::Throw(request)); }
    pub fn mob_care(&mut self, id: u64, heal: bool, effect: Option<minecraftoss_entities::effects::EffectInstance>, ignite_ticks: i32) {
        self.send(Command::MobCare { id, heal, effect, ignite_ticks });
    }
    pub fn contact(&mut self, id: u64, delta: [f64; 3], velocity: [f64; 3]) {
        self.send(Command::Contact {
            id,
            delta,
            velocity,
        });
    }
    pub fn spawn(sim: ServerSim) -> Self {
        let states = sim.states.clone();
        let count = sim.level.registries().blocks.state_count();
        let uses: Vec<bool> = (0..count)
            .map(|i| sim.level.handles_use(BlockStateId(i as u16)))
            .collect();
        let attacks: Vec<bool> = (0..count)
            .map(|i| sim.level.handles_attack(BlockStateId(i as u16)))
            .collect();
        let (commands, receiver) = std::sync::mpsc::channel::<Command>();
        let (sender, outputs) = std::sync::mpsc::channel::<Output>();
        let thread = std::thread::Builder::new()
            .name("Server thread".into())
            .spawn(move || server_loop(sim, receiver, sender))
            .expect("server thread starts");
        Self {
            commands: Some(commands),
            outputs,
            states,
            uses: Arc::new(uses),
            attacks: Arc::new(attacks),
            sent: 0,
            received: 0,
            waited: Vec::new(),
            thread: Some(thread),
        }
    }

    fn send(&mut self, command: Command) {
        self.sent += 1;
        if let Some(commands) = &self.commands {
            let _ = commands.send(command);
        }
    }

    /// Commands sent so far (compare with `Output::handled`).
    pub fn sent(&self) -> u64 {
        self.sent
    }

    pub fn load_chunk(&mut self, chunk: &Arc<Chunk>) {
        self.send(Command::LoadChunk(chunk.clone()));
    }

    pub fn load_scene_chunk(&mut self, chunk: &Arc<Chunk>, scene: &HandcraftedScene) {
        let (placed, cleared) = scene.chunk_edits((chunk.pos.x, chunk.pos.z));
        if placed.is_none() && cleared.is_none() { self.load_chunk(chunk); return; }
        let edits = cleared.into_iter().flat_map(|v|v.iter().map(|&p|(p,None)))
            .chain(placed.into_iter().flat_map(|v|v.iter().map(|(&p,b)|(p,Some(b.clone()))))).collect();
        self.send(Command::LoadEditedChunk { chunk: chunk.clone(), edits });
    }

    pub fn unload_chunk(&mut self, pos: ChunkPos) {
        self.send(Command::UnloadChunk(pos));
    }

    pub fn player_edit(&mut self, scene: &HandcraftedScene, pos: BlockPos, edit: PlayerEdit) {
        let block = Scene::block(scene, pos).cloned();
        self.send(Command::PlayerEdit { pos, block, edit });
    }

    fn state_in(&self, scene: &HandcraftedScene, pos: BlockPos) -> Option<BlockStateId> {
        Scene::block(scene, pos).and_then(|b| self.states.state_of(b))
    }

    /// `useWithoutItem` on a simulated block: true (and sent) when the level
    /// acts on the block the scene shows.
    pub fn use_block(
        &mut self,
        scene: &HandcraftedScene,
        pos: BlockPos,
        facing: &'static str,
    ) -> bool {
        let handled = self
            .state_in(scene, pos)
            .is_some_and(|s| self.uses.get(usize::from(s.0)).copied().unwrap_or(false));
        if handled {
            self.send(Command::UseBlock { pos, facing });
        }
        handled
    }

    /// Bone meal on a clicked face; whether it was used arrives in an output.
    pub fn bone_meal(&mut self, pos: BlockPos, face: &'static str) {
        self.send(Command::BoneMeal { pos, face });
    }

    /// A player's attack; true (and sent) when the level acts on the block.
    pub fn attack_block(&mut self, scene: &HandcraftedScene, pos: BlockPos) -> bool {
        let handled = self
            .state_in(scene, pos)
            .is_some_and(|s| self.attacks.get(usize::from(s.0)).copied().unwrap_or(false));
        if handled {
            self.send(Command::Attack(pos));
        }
        handled
    }

    #[allow(clippy::too_many_arguments)]
    pub fn spawn_item(
        &mut self,
        item: &str,
        count: i32,
        components: Option<&str>,
        position: [f64; 3],
        velocity: [f64; 3],
        pickup_delay: i32,
        age: i32,
    ) {
        self.send(Command::SpawnItem {
            item: item.to_owned(),
            count,
            components: components.map(str::to_owned),
            position,
            velocity,
            pickup_delay,
            age,
        });
    }

    pub fn tick(&mut self, input: TickInput) {
        self.send(Command::Tick(Box::new(input)));
    }

    pub fn set_recipe_book(&mut self, recipes: Arc<minecraftoss_player::crafting::RecipeBook>) {
        self.send(Command::RecipeBook(recipes));
    }

    pub fn load_loot(&mut self, jar: std::path::PathBuf, seed: u64) {
        self.send(Command::LootTables { jar, seed });
    }

    /// An operation on the player's trading screen, with a copy of the
    /// player's inventory.
    pub fn merchant(
        &mut self,
        op: MerchantOp,
        inventory: &minecraftoss_player::inventory::Inventory,
        selected: usize,
        feet: [f64; 3],
    ) {
        self.send(Command::Merchant {
            op,
            inventory: Box::new(inventory.clone()),
            selected,
            feet,
        });
    }

    /// Summons a still experience orb worth `value`.
    pub fn summon_orb(&mut self, position: [f64; 3], value: i32) {
        self.send(Command::SummonOrb { position, value });
    }

    /// Experience at a point, split into orbs as vanilla splits it (a
    /// broken ore's, a furnace's, a death's).
    pub fn award_experience(&mut self, position: [f64; 3], amount: i32) {
        if amount > 0 {
            self.send(Command::AwardExperience { position, amount });
        }
    }

    /// The Mass Inverter takes a mob's gravity for `seconds`.
    pub fn lift(&mut self, id: u64, seconds: f32) {
        self.send(Command::Lift { id, seconds });
    }

    /// A mob on a seat (a boat's, a minecart's), or let go.
    pub fn seat(&mut self, id: u64, at: Option<[f64; 3]>) {
        self.send(Command::Seat { id, at });
    }

    /// A mob named: kept from despawning.
    pub fn require_persistence(&mut self, id: u64) {
        self.send(Command::Persist { id });
    }

    /// A mob named with a name tag: kept, and the name saved with it.
    pub fn name(&mut self, id: u64, name: String) {
        self.send(Command::Name { id, name });
    }

    /// `/summon` for a mob; what it did arrives in an output. The new
    /// mob's yaw comes from its own random (`LivingEntity`'s constructor:
    /// `nextFloat() * (float)(Math.PI * 2)`, in degrees), seeded here from
    /// the clock as a fresh entity's is.
    pub fn summon(
        &mut self,
        kind: String,
        position: [f64; 3],
        nbt: Option<minecraftoss_core::nbt::Tag>,
    ) {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |d| d.as_nanos() as i64);
        let y_rot = minecraftoss_core::random::LegacyRandom::new(nanos).next_f32()
            * (std::f64::consts::PI * 2.0) as f32;
        self.send(Command::Summon {
            kind,
            position,
            nbt,
            y_rot,
        });
    }

    /// Hands a player's hit or use on a mob to the server; its result
    /// arrives in an output.
    pub fn mob_action(
        &mut self,
        hit: minecraftoss_entities::world::MobHit,
        attack: Option<minecraftoss_entities::world::PlayerAttack>,
        inventory: &minecraftoss_player::inventory::Inventory,
        selected: usize,
        infinite: bool,
    ) {
        self.send(Command::MobAction {
            hit,
            attack,
            inventory: Box::new(inventory.clone()),
            selected,
            infinite,
        });
    }

    /// Outputs the server has produced since the last call.
    pub fn poll(&mut self) -> Vec<Output> {
        let mut outputs = std::mem::take(&mut self.waited);
        outputs.extend(self.outputs.try_iter());
        if let Some(last) = outputs.last() { self.received = self.received.max(last.handled); }
        outputs
    }

    /// Blocks until the server has handled every command sent so far (a
    /// capture's settle ticks, each answered before the next), keeping the
    /// outputs for the next [`Self::poll`].
    pub fn wait_idle(&mut self) {
        while self.received < self.sent {
            match self.outputs.recv() {
                Ok(out) => { self.received = out.handled; self.waited.push(out); }
                Err(_) => break,
            }
        }
    }

    pub fn save_and_wait(&mut self) {
        self.send(Command::Save);
        self.wait_idle();
    }
}

impl Drop for ServerHandle {
    fn drop(&mut self) {
        // Closing the channel ends the loop.
        self.commands = None;
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

fn make_stack(
    recipes: &minecraftoss_player::crafting::RecipeBook,
    item: &str,
    count: i32,
    components: Option<&str>,
) -> minecraftoss_player::inventory::ItemStack {
    let mut stack = minecraftoss_player::inventory::ItemStack::new(item, count.clamp(0, 255) as u8);
    stack.components = components.and_then(|c| serde_json::from_str(c).ok());
    if stack.components.is_none() {
        stack.max = stack.max.min(recipes.max_stack(item));
    }
    stack
}

fn server_loop(
    mut sim: ServerSim,
    commands: std::sync::mpsc::Receiver<Command>,
    outputs: std::sync::mpsc::Sender<Output>,
) {
    let mut handled = 0u64;
    while let Ok(first) = commands.recv() {
        let mut out = Output::default();
        // Everything already queued is handled before answering.
        let mut next = Some(first);
        while let Some(command) = next.take().or_else(|| commands.try_recv().ok()) {
            handled += 1;
            match command {
                Command::Save => sim.save_all_entities(),
                Command::Throw(request) => sim.throw(request),
                Command::LoadChunk(chunk) => sim.load_chunk(&chunk),
                Command::LoadEditedChunk { chunk, edits } => sim.load_chunk_with_edits(&chunk, &edits),
                Command::UnloadChunk(pos) => sim.unload_chunk(pos),
                Command::PlayerEdit { pos, block, edit } => {
                    sim.player_edit_block(pos, block.as_ref(), edit)
                }
                Command::UseBlock { pos, facing } => {
                    sim.use_block(pos, facing);
                }
                Command::BoneMeal { pos, face } => {
                    if sim.bone_meal(pos, face) {
                        out.bone_meal_used.push(pos);
                    }
                }
                Command::Attack(pos) => {
                    sim.attack_block(pos);
                }
                Command::SpawnItem {
                    item,
                    count,
                    components,
                    position,
                    velocity,
                    pickup_delay,
                    age,
                } => {
                    sim.spawn_item(
                        &item,
                        count,
                        components.as_deref(),
                        position,
                        velocity,
                        pickup_delay,
                        age,
                    );
                }
                Command::RecipeBook(recipes) => sim.set_recipe_book(recipes),
                Command::LootTables { jar, seed } => sim.load_loot(&jar, seed),
                Command::SummonOrb { position, value } => sim.summon_orb(position, value),
                Command::AwardExperience { position, amount } => sim.award_experience(position, amount),
                Command::Contact{id,delta,velocity}=>sim.contact(id,delta,velocity),
                Command::MobCare{id,heal,effect,ignite_ticks}=>sim.mob_care(id,heal,effect,ignite_ticks),
                Command::Lift { id, seconds } => sim.lift(id, seconds),
                Command::Seat { id, at } => sim.seat(id, at),
                Command::Persist { id } => sim.require_persistence(id),
                Command::Name { id, name } => sim.name(id, name),
                Command::Summon {
                    kind,
                    position,
                    nbt,
                    y_rot,
                } => {
                    out.summoned.push(
                        sim.summon(&kind, position, nbt.as_ref(), y_rot)
                            .map(|()| kind),
                    );
                }
                Command::MobAction {
                    hit,
                    attack,
                    inventory,
                    selected,
                    infinite,
                } => {
                    out.mob_results
                        .push(sim.mob_action(hit, attack, *inventory, selected, infinite));
                }
                Command::Merchant {
                    op,
                    inventory,
                    selected,
                    feet,
                } => {
                    out.merchant
                        .push(sim.merchant(op, *inventory, selected, feet));
                }
                Command::Tick(input) => {
                    let started = std::time::Instant::now();
                    let input = *input;
                    // `MinecraftServer.autoSave`: every five minutes.
                    sim.ticks_since_save += 1;
                    let autosave=sim.ticks_since_save>=6000;
                    if sim.ticks_since_save >= 6000 {
                        sim.ticks_since_save = 0;
                        sim.save_all_entities();
                    }
                    sim.set_time(input.day_ticks);
                    sim.set_host_weather(input.weather);
                    sim.set_players(&input.players, input.difficulty);
                    // Living, non-spectating players draw experience orbs.
                    sim.level.living_players = input
                        .mob_players
                        .iter()
                        .filter(|p| p.alive && !p.spectator)
                        .map(|p| (p.position.to_array(), f64::from(p.eye_height)))
                        .collect();
                    sim.set_simulation_area(input.simulation_center, input.simulation_distance);
                    if let Some(natural) = &mut sim.level.natural_spawning {
                        natural.spawn_mobs = input.spawn_mobs;
                    }
                    sim.prepare_spawning(&input.mob_players);
                    sim.level.skull_gem_collector = input.pickup.as_ref().and_then(|(feet, inventory, selected)| {
                        let mut available = inventory.clone();
                        let stack = make_stack(&available.recipes, minecraftoss_core::mashup_skull_gem::ID, 1, None);
                        available.add_item(stack, *selected).is_none()
                            .then_some([feet[0], feet[1] + 0.9, feet[2]])
                    });
                    sim.tick();
                    out.drips=std::mem::take(&mut sim.level.drip_effects);
                    out.weather=sim.level.sky.as_ref().map(|s|HostWeather{raining:s.weather.raining,thundering:s.weather.thundering,rain_level:s.weather.rain_level,thunder_level:s.weather.thunder_level});
                    let mobs_started = std::time::Instant::now();
                    sim.take_spawned();
                    sim.despawn_mobs(&input.mob_players, input.difficulty);
                    // The player's fight memory: its `tickCount` moves on and
                    // the hurts the client took land in it.
                    sim.mobs.tick_player(0);
                    for &(source, kind) in &input.player_hurts {
                        sim.mobs.player_hurt(0, source, kind);
                    }
                    sim.mobs.set_projectile_fields(
                        input
                            .projectile_fields
                            .iter()
                            .map(|&(c, r)| (glam::DVec3::from_array(c), r))
                            .collect(),
                    );
                    // Items, falling blocks and TNT float in them too.
                    sim.level.weightless = input.projectile_fields.clone();
                    sim.mobs.set_player_views(input.mob_views.clone());
                    sim.mobs.set_player_vitals(input.mob_vitals.clone());
                    // What the player holds, as villagers see it
                    // (`ShowTradesToPlayer`).
                    let held = input.pickup.as_ref().and_then(|(_, inventory, selected)| {
                        inventory
                            .slots
                            .get(*selected)
                            .and_then(Option::as_ref)
                            .map(|s| s.id.clone())
                    });
                    sim.mobs.set_player_main_hand(0, held.as_deref());
                    if input.entities_tick {
                        sim.tick_mobs(&input.mob_players, input.bright_outside);
                    }
                    // Resolve before publishing the authoritative snapshot. No
                    // damage is applied: this is a contact barrier, not an attack.
                    if !input.light_barriers.is_empty() {
                        let world = crate::server_mobs::MobWorld {
                            level: std::cell::RefCell::new(&mut sim.level),
                            states: &sim.states,
                            tables: &sim.mob_tables,
                        };
                        let ids: Vec<_> = sim.mobs.mob_ids().collect();
                        for id in ids {
                            let Some(body) = sim.mobs.body_mut(id) else {
                                continue;
                            };
                            for &b in &input.light_barriers {
                                if crate::light_barrier::resolve_body(&world, b, body) {
                                    if !out.shield_hits.contains(&b.client) {
                                        out.shield_hits.push(b.client);
                                    }
                                }
                            }
                        }
                    }
                    sim.tick_thrown(&input.mob_players, &input.thrown_targets, &mut out);
                    sim.tick_arrows(&input.mob_players, &input.thrown_targets, input.entities_tick, &mut out);
                    sim.spawn_trade_experience();
                    out.merchant.extend(sim.check_merchant(&input.mob_players));
                    out.player_hits.extend(sim.mobs.take_player_hits());
                    out.player_splashes.extend(sim.mobs.take_player_splashes());
                    out.potion_breaks.extend(sim.mobs.take_potion_breaks());
                    out.explosions.extend(sim.take_explosions());
                    out.mob_sounds.extend(sim.mobs.take_sounds());
                    sim.level.last_tick_phases[5] += mobs_started.elapsed().as_secs_f64() * 1000.0;
                    out.mobs = Some(Box::new(
                        sim.tracked_mobs(input.tracking.0, input.tracking.1),
                    ));
                    let pickup_feet = input.pickup.as_ref().map(|(feet, _, _)| *feet);
                    out.pickup_player = input.pickup_player;
                    if let Some((feet, mut inventory, selected)) = input.pickup {
                        let recipes = inventory.recipes.clone();
                        let picked = sim.pickup(feet, |item, count, components| {
                            let stack = make_stack(&recipes, item, count, components);
                            match inventory.add_item(stack, selected) {
                                None => count,
                                Some(rest) => count - i32::from(rest.count),
                            }
                        });
                        out.picked.extend(picked);
                    }
                    // After the items, one experience orb.
                    out.orbs_taken.extend(sim.take_experience(pickup_feet));
                    for (id, feet, slots) in &input.hoppers {
                        for (item, count, components) in sim.hopper_pickup(*feet, slots) {
                            out.hopper_picked.push((*id, item, count, components));
                        }
                    }
                    out.entities = Some(EntitySnapshot {
                        items: sim.items(),
                        tnt: sim.primed_tnt(),
                        falling: sim.falling_blocks(),
                        orbs: sim.orbs(),
                    });
                    let (solves, ms) = sim.level.light_solves.replace((0, 0.0));
                    let elapsed=started.elapsed().as_secs_f64()*1000.0;
                    if elapsed>200.0{diag::info!(World,"Minecraft slow server: elapsed_ms={elapsed:.3} autosave={autosave} phases={:?} light_solves={solves} light_ms={ms:.3} chunks={}",sim.level.last_tick_phases,sim.level.chunks().count());}
                    out.tick_phases = Some((
                        sim.level.last_tick_phases,
                        (solves, ms),
                        started.elapsed().as_secs_f64() * 1000.0,
                    ));
                }
            }
        }
        out.changes = sim.take_changes();
        out.names = sim.take_names();
        out.handled = handled;
        if outputs.send(out).is_err() {
            break;
        }
    }
    // The client closed the world: its entities are saved.
    sim.save_all_entities();
}

#[cfg(test)]
mod tests {
    use super::*;
    use minecraftoss_core::Registries;
    use minecraftoss_core::registries::DataPaths;
    use minecraftoss_generator::terrain::TerrainGenerator;
    use minecraftoss_world::chunk_map::ChunkMap;

    #[test]
    fn native_cow_collision_stands_on_the_visible_canal_rim_in_every_direction(){
        use glam::DVec3;
        use minecraftoss_player::collision::{Bounds,move_body};
        let paths=DataPaths::discover().unwrap();
        let registries=Arc::new(Registries::load(&paths).unwrap());
        assert_eq!(registries.blocks.collision_boxes(registries.blocks.parse_state("minecraft:carved_pumpkin").unwrap()),vec![[0.,0.,0.,1.,1.,1.]],"vanilla is unchanged");
        let worldgen=Arc::new(WorldGen::new(Arc::new(TerrainGenerator::overworld(registries.clone(),0).unwrap())).unwrap());
        let states=Arc::new(BlockStates::new(registries,0,-64,384).unwrap());
        let mut map=ChunkMap::with_worldgen(worldgen.clone(),2,4);
        let mut server=ServerSim::new(worldgen,states,"minecraft:overworld");
        server.load_chunk(&map.load_now(ChunkPos::new(0,0)));
        for x in 5..12 {for z in 5..12 {server.player_edit_block((x,249,z),Some(&Block::new("minecraft:stone")),PlayerEdit::Place);}}
        for (turn,facing) in ["north","east","south","west"].into_iter().enumerate(){
            let rim=Block{ id:crate::pack::ResourceId::parse("jak:barrio_zanja_borde").unwrap(), properties:[("facing".into(),facing.into())].into_iter().collect()};
            server.player_edit_block((8,250,8),Some(&rim),PlayerEdit::Place);
            let(mut x,mut z)=(0.5,0.35);for _ in 0..turn{(x,z)=(1.-z,x);}
            let feet=DVec3::new(8.+x,252.,8.+z);
            let world=crate::server_mobs::MobWorld{level:std::cell::RefCell::new(&mut server.level),states:&server.states,tables:&server.mob_tables};
            let down=move_body(&world,Bounds::standing(feet,0.9,1.4),DVec3::NEG_Y*3.,false,0.6);
            assert!((feet.y+down.y-(250.+f64::from(3.6f32)/16.)).abs()<1e-6,"{facing}: a cow must land on the lip, not the invisible full cube: {:?}",feet+down);
        }
    }

    #[test]
    fn arrows_keep_the_native_flight_stick_release_and_inventory_pickup() {
        use glam::DVec3;
        use crate::thrown::Request;
        let paths = DataPaths::discover().unwrap();
        let registries = Arc::new(Registries::load(&paths).unwrap());
        let worldgen = Arc::new(WorldGen::new(Arc::new(TerrainGenerator::overworld(registries.clone(), 0).unwrap())).unwrap());
        let states = Arc::new(BlockStates::new(registries, 0, -64, 384).unwrap());
        let mut map = ChunkMap::with_worldgen(worldgen.clone(), 2, 4);
        let mut server = ServerSim::new(worldgen, states, "minecraft:overworld");
        server.load_chunk(&map.load_now(ChunkPos::new(0, 0)));
        let wall = (8, 250, 2);
        server.player_edit_block(wall, Some(&Block::new("minecraft:stone")), PlayerEdit::Place);
        let request = || Request { item: "minecraft:arrow".into(), components: None, speed: Some(3.), base_damage: 2., critical: false, effects: vec![], player: 7, eye: DVec3::new(2.5, 250.5, 2.5), direction: DVec3::X };
        server.throw(request());
        for _ in 0..3 { server.tick_arrows(&[], &[], true, &mut Output::default()); }
        let arrow = &server.arrows[0];
        assert!(arrow.body.in_ground);
        assert!(arrow.direction.x > 2.9);
        let feet = (arrow.body.position - DVec3::Y * 0.5).to_array();
        assert!(server.pickup(feet, |_, _, _| 1).is_empty(), "shaking arrows cannot be picked up");
        for _ in 0..8 { server.tick_arrows(&[], &[], true, &mut Output::default()); }
        assert!(server.pickup(feet, |_, _, _| 0).is_empty(), "a full inventory leaves the arrow in its block");
        let picked = server.pickup(feet, |id, count, _| { assert_eq!(id, "minecraft:arrow"); count });
        assert_eq!(picked.len(), 1);
        assert!(server.pickup(feet, |_, count, _| count).is_empty());
        let mut no_pickup = request();
        no_pickup.components = Some(serde_json::json!({"mashup:no_pickup":true}));
        server.throw(no_pickup);
        for _ in 0..12 { server.tick_arrows(&[], &[], true, &mut Output::default()); }
        assert!(server.pickup(feet, |_, count, _| count).is_empty());
        server.player_edit_block(wall, None, PlayerEdit::Break);
        server.tick_arrows(&[], &[], true, &mut Output::default());
        assert!(!server.arrows[0].body.in_ground, "removing the supporting block releases the arrow");
        server.arrows.clear();
        // Both the player and the host's Jak actors receive exactly one hit.
        server.throw(request());
        let external = [(0xB1A2E, [6., 249., 1.5, 7., 252., 3.5])];
        let mut out = Output::default();
        for _ in 0..4 { server.tick_arrows(&[], &external, true, &mut out); }
        assert_eq!(out.thrown_hits.len(), 1);
        assert_eq!(out.thrown_hits[0].key, 0xB1A2E);
        assert!(out.thrown_hits[0].damage >= 5.);
        assert!(server.arrows.is_empty());
        // Skeleton shots have the same visible model and a stable direction
        // after impact; none turn into a pickup from a mob's infinite quiver.
        server.player_edit_block(wall, Some(&Block::new("minecraft:stone")), PlayerEdit::Place);
        server.mobs.spawn_owned_arrow(99, minecraftoss_entities::projectile::Arrow::in_flight(DVec3::new(7., 250.5, 2.5), DVec3::X * 2., minecraftoss_player::rng::LegacyRandom::new(2)));
        let mut out = Output::default();
        server.tick_arrows(&[], &[], true, &mut out);
        assert_eq!(out.arrows.unwrap().len(), 1);
        assert!(server.pickup([7., 250., 2.5], |_, count, _| count).is_empty());
    }

    /// Thrown items strike what lies outside the world's own entities (an
    /// external box: a blaze, a Jak creature) as well as its mobs, and
    /// report each blow for the host's damage route; a harming splash
    /// reaches the external box by nearness. The flight never strikes the
    /// thrower's own seat.
    #[test]
    fn thrown_items_strike_external_and_native_targets() {
        use crate::thrown::{Flying, Request};
        let Ok(paths) = DataPaths::discover() else { return };
        let registries = Arc::new(Registries::load(&paths).unwrap());
        let worldgen = Arc::new(WorldGen::new(Arc::new(TerrainGenerator::overworld(registries.clone(), 0).unwrap())).unwrap());
        let states = Arc::new(BlockStates::new(registries, 0, -64, 384).unwrap());
        let mut map = ChunkMap::with_worldgen(worldgen.clone(), 2, 4);
        let mut server = ServerSim::new(worldgen, states, "minecraft:overworld");
        server.load_chunk(&map.load_now(ChunkPos::new(0, 0)));
        let eye = glam::DVec3::new(2.5, 250.0, 2.5);
        let request = |item: &str, effects| Request { item: item.into(), components: None, speed: None, base_damage: 2.0, critical: false, effects, player: 7, eye, direction: glam::DVec3::X };
        let external = vec![(0xB1A2E_u64, [6.0, 249.0, 1.5, 7.0, 251.5, 3.5]), (0x5EA7, [2.0, 248.0, 2.0, 3.0, 251.0, 3.0])];
        let run = |server: &mut ServerSim, flight: Flying| {
            server.thrown.push(flight);
            let mut hits = Vec::new();
            for _ in 0..40 {
                let mut out = Output::default();
                server.tick_thrown(&[], &external, &mut out);
                hits.extend(out.thrown_hits);
            }
            hits
        };
        let snowball = Flying::new(request("minecraft:snowball", Vec::new()), &mut server.throw_random);
        let hits = run(&mut server, snowball);
        assert_eq!(hits.len(), 1, "{hits:?}");
        assert_eq!((hits[0].key, hits[0].native, hits[0].damage), (0xB1A2E, false, 0.0));
        let harming = minecraftoss_entities::potion::registry("minecraft:strong_harming").unwrap();
        let splash = Flying::new(request("minecraft:splash_potion", harming), &mut server.throw_random);
        let hits = run(&mut server, splash);
        assert!(hits.iter().any(|h| h.key == 0xB1A2E && h.damage > 0.0), "{hits:?}");
        // The splash, by nearness, reaches the mount it was thrown from as it
        // would any mob; only the flight itself never strikes that seat (the
        // snowball's single hit above).
        server.summon("minecraft:zombie", [5.0, 249.0, 2.5], None, 0.0).unwrap();
        let id = server.mobs.zombies().last().unwrap().id;
        let egg = Flying::new(request("minecraft:egg", Vec::new()), &mut server.throw_random);
        let hits = run(&mut server, egg);
        assert!(hits.iter().any(|h| h.key == id && h.native), "{hits:?}");
    }

    /// The host's care of a mob (Jak's eco vents and yellow shots): healed
    /// whole, an effect given, set alight for as long as asked (never
    /// shortened).
    #[test]
    fn host_care_heals_gives_effects_and_ignites_a_mob() {
        use minecraftoss_entities::effects::{EffectInstance, MobEffect};
        let Ok(paths) = DataPaths::discover() else { return };
        let registries = Arc::new(Registries::load(&paths).unwrap());
        let worldgen = Arc::new(WorldGen::new(Arc::new(TerrainGenerator::overworld(registries.clone(), 0).unwrap())).unwrap());
        let states = Arc::new(BlockStates::new(registries, 0, -64, 384).unwrap());
        let mut map = ChunkMap::with_worldgen(worldgen.clone(), 2, 4);
        let mut server = ServerSim::new(worldgen, states, "minecraft:overworld");
        server.load_chunk(&map.load_now(ChunkPos::new(0, 0)));
        server.summon("minecraft:zombie", [4.5, 250.0, 4.5], None, 0.0).unwrap();
        let id = server.mobs.zombies().last().unwrap().id;
        fn server_health(server: &ServerSim, id: u64) -> f32 {
            server.mobs.zombies().iter().find(|z| z.id == id).unwrap().zombie.health
        }
        let full = server_health(&server, id);
        // Instant health harms the undead (`isInvertedHealAndHarm`).
        server.mobs.splash_instant(id, MobEffect::InstantHealth, 1, 1.0, glam::DVec3::new(4.5, 250.0, 4.5), None);
        assert!(server_health(&server, id) < full);
        server.mob_care(id, true, None, 0);
        assert_eq!(server_health(&server, id), full);
        server.mob_care(id, false, Some(EffectInstance::new(MobEffect::Speed, 400, 1)), 100);
        let (effects, ..) = server.mobs.mob_effects_mut(id).unwrap();
        assert_eq!(effects.get(MobEffect::Speed).map(|e| (e.duration, e.amplifier)), Some((400, 1)));
        assert_eq!(server.mobs.body_mut(id).unwrap().fire_ticks, 100);
        server.mob_care(id, false, None, 20);
        assert_eq!(server.mobs.body_mut(id).unwrap().fire_ticks, 100);
    }

    #[test]
    fn host_contacts_use_server_collision_and_never_move_a_seated_mob() {
        let Ok(paths) = DataPaths::discover() else { return };
        let registries = Arc::new(Registries::load(&paths).unwrap());
        let worldgen = Arc::new(
            WorldGen::new(Arc::new(
                TerrainGenerator::overworld(registries.clone(), 0).unwrap(),
            ))
            .unwrap(),
        );
        let states = Arc::new(BlockStates::new(registries, 0, -64, 384).unwrap());
        let mut map = ChunkMap::with_worldgen(worldgen.clone(), 2, 4);
        let mut server = ServerSim::new(worldgen, states, "minecraft:overworld");
        server.load_chunk(&map.load_now(ChunkPos::new(0, 0)));
        for y in 200..=204 {
            for z in 3..=6 {
                server.player_edit_block(
                    (6, y, z),
                    Some(&Block::new("minecraft:stone")),
                    PlayerEdit::Place,
                );
            }
        }
        server
            .summon("minecraft:zombie", [4.5, 201.0, 4.5], None, 0.0)
            .unwrap();
        let id = server.mobs.zombies().last().unwrap().id;
        server.contact(id, [4.0, 0.0, 0.0], [0.1, 0.0, 0.0]);
        let body = server.mobs.body_mut(id).unwrap();
        assert!(
            body.position.x < 6.0 && body.position.x > 4.5,
            "{:?}",
            body.position
        );
        assert_eq!(body.velocity.x, 0.1);
        assert!(body.needs_sync);
        let at = body.position;
        server.seat(id, Some(at.to_array()));
        server.contact(id, [-2.0, 1.0, 0.0], [1.0, 0.0, 0.0]);
        assert_eq!(server.mobs.body_mut(id).unwrap().position, at);
        server.seat(id, None);
        server.contact(id, [f64::NAN, 0.0, 0.0], [0.0; 3]);
        assert_eq!(server.mobs.body_mut(id).unwrap().position, at);
    }

    /// A block the player places reaches the level, which runs the block's
    /// behaviour and reports the blocks it changed back to the scene.
    #[test]
    fn placed_water_flows_through_the_level() {
        let Ok(paths) = DataPaths::discover() else {
            return;
        };
        let Ok(registries) = Registries::load(&paths) else {
            return;
        };
        let registries = Arc::new(registries);
        let worldgen = Arc::new(
            WorldGen::new(Arc::new(
                TerrainGenerator::overworld(registries.clone(), 0).unwrap(),
            ))
            .unwrap(),
        );
        let states = Arc::new(BlockStates::new(registries.clone(), 0, -64, 384).unwrap());
        let mut map = ChunkMap::with_worldgen(worldgen.clone(), 2, 4);
        let mut server = ServerSim::new(worldgen, states.clone(), "minecraft:overworld");
        let mut scene = HandcraftedScene::streamed(states);
        for x in -1..=1 {
            for z in -1..=1 {
                let chunk = map.load_now(ChunkPos::new(x, z));
                server.load_chunk(&chunk);
                scene.insert_chunk(chunk);
            }
        }
        // A water source in the air above the terrain.
        let pos = (8, 200, 8);
        scene.set(pos, Some(Block::new("minecraft:water")));
        server.player_edit(&scene, pos, PlayerEdit::Place);
        let placed = server.take_changes();
        assert_eq!(placed.len(), 1, "{placed:?}");
        let mut flowed = 0;
        for _ in 0..10 {
            server.tick();
            flowed += server.take_changes().len();
        }
        assert!(flowed > 0, "water should fall from the source");
    }

    /// With a send limit, finished chunks reach the client a few a tick,
    /// nearest first, each once, until every tracked chunk is in.
    #[test]
    fn chunks_reach_the_client_a_few_a_tick() {
        let Ok(paths) = DataPaths::discover() else {
            return;
        };
        let Ok(registries) = Registries::load(&paths) else {
            return;
        };
        let registries = Arc::new(registries);
        let worldgen = Arc::new(
            WorldGen::new(Arc::new(
                TerrainGenerator::overworld(registries.clone(), 77).unwrap(),
            ))
            .unwrap(),
        );
        let mut map = ChunkMap::with_worldgen(worldgen, 2, 4);
        map.set_send_limit(3);
        let centre = ChunkPos::new(0, 0);
        let mut sent: Vec<ChunkPos> = Vec::new();
        let started = std::time::Instant::now();
        while started.elapsed() < std::time::Duration::from_secs(300) {
            let loads: Vec<ChunkPos> = map
                .tick(centre)
                .into_iter()
                .filter_map(|event| match event {
                    minecraftoss_world::ChunkEvent::Load(chunk) => Some(chunk.pos),
                    _ => None,
                })
                .collect();
            assert!(loads.len() <= 3, "{} chunks in one tick", loads.len());
            // Nearest first within a tick.
            assert!(loads.windows(2).all(|w| centre.distance_squared(w[0])
                <= centre.distance_squared(w[1])));
            sent.extend(loads);
            if map.is_idle() {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(2));
        }
        assert!(map.is_idle(), "every tracked chunk sent in time");
        let unique: std::collections::HashSet<ChunkPos> = sent.iter().copied().collect();
        assert_eq!(unique.len(), sent.len(), "a chunk sent twice");
        assert!(sent.len() >= 9, "{} chunks sent", sent.len());
    }

    /// Local two-player: two players far apart both get every chunk round
    /// them, nearest either first, none twice; and how long that takes
    /// against one player (printed: `-- --nocapture`).
    #[test]
    fn two_players_far_apart_both_get_their_chunks() {
        let Ok(paths) = DataPaths::discover() else {
            return;
        };
        let Ok(registries) = Registries::load(&paths) else {
            return;
        };
        let registries = Arc::new(registries);
        let worldgen = Arc::new(
            WorldGen::new(Arc::new(
                TerrainGenerator::overworld(registries.clone(), 12345).unwrap(),
            ))
            .unwrap(),
        );
        let load = |second: Option<ChunkPos>| {
            let mut map = ChunkMap::with_worldgen(worldgen.clone(), 4, 6);
            let first = ChunkPos::new(0, 0);
            let mut sent: Vec<ChunkPos> = Vec::new();
            let started = std::time::Instant::now();
            while started.elapsed() < std::time::Duration::from_secs(300) {
                sent.extend(map.tick_two(first, second).into_iter().filter_map(|event| match event {
                    minecraftoss_world::ChunkEvent::Load(chunk) => Some(chunk.pos),
                    _ => None,
                }));
                if map.is_idle() {
                    break;
                }
                std::thread::sleep(std::time::Duration::from_millis(2));
            }
            assert!(map.is_idle(), "every tracked chunk sent in time");
            (sent, started.elapsed())
        };
        let (one, one_time) = load(None);
        let far = ChunkPos::new(40, -25);
        let (two, two_time) = load(Some(far));
        eprintln!(
            "chunk load, view 4: one player {} chunks in {:.2}s; two far apart {} chunks in {:.2}s",
            one.len(),
            one_time.as_secs_f64(),
            two.len(),
            two_time.as_secs_f64()
        );
        let unique: std::collections::HashSet<ChunkPos> = two.iter().copied().collect();
        assert_eq!(unique.len(), two.len(), "a chunk sent twice");
        for centre in [ChunkPos::new(0, 0), far] {
            for x in -4..=4 {
                for z in -4..=4 {
                    let pos = ChunkPos::new(centre.x + x, centre.z + z);
                    assert!(unique.contains(&pos), "{pos:?} round {centre:?} never came");
                }
            }
        }
        // One player: exactly what it always was.
        assert!(one.iter().all(|p| p.chebyshev(ChunkPos::new(0, 0)) <= 5));
    }

    /// Creatures the SPAWN step generated join the server's mobs with their
    /// chunk, once per session, and wander while inside the ticking area.
    #[test]
    fn generated_mobs_join_the_server_and_wander() {
        let Ok(paths) = DataPaths::discover() else {
            return;
        };
        let Ok(registries) = Registries::load(&paths) else {
            return;
        };
        let registries = Arc::new(registries);
        let worldgen = Arc::new(
            WorldGen::new(Arc::new(
                TerrainGenerator::overworld(registries.clone(), 1234).unwrap(),
            ))
            .unwrap(),
        );
        let states = Arc::new(BlockStates::new(registries.clone(), 1234, -64, 384).unwrap());
        let mut map = ChunkMap::with_worldgen(worldgen.clone(), 6, 8);
        let mut server = ServerSim::new(worldgen, states, "minecraft:overworld");
        // Plains around chunk (20, 128), a SPAWN probe with creatures.
        let mut generated = 0;
        for x in 15..=25 {
            for z in 123..=133 {
                let chunk = map.load_now(ChunkPos::new(x, z));
                generated += chunk
                    .generation
                    .entities
                    .iter()
                    .filter(|t| {
                        t.get("id")
                            .and_then(minecraftoss_core::nbt::Tag::as_str)
                            .is_some_and(|id| {
                                [
                                    "minecraft:cow",
                                    "minecraft:pig",
                                    "minecraft:chicken",
                                    "minecraft:sheep",
                                ]
                                .contains(&id)
                            })
                    })
                    .count();
                server.load_chunk(&chunk);
            }
        }
        assert!(generated > 0, "seed 1234 generates animals there");
        let animals = |world: &minecraftoss_entities::world::EntityWorld| {
            world.cows().iter().filter(|c| c.horse.is_none()).count()
                + world.pigs().len()
                + world.chickens().len()
                + world.sheep().len()
        };
        assert_eq!(
            animals(&server.mobs),
            generated,
            "every generated animal joins"
        );
        // Loading a chunk again does not add its creatures twice.
        server.load_chunk(&map.load_now(ChunkPos::new(20, 128)));
        assert_eq!(animals(&server.mobs), generated);
        let start: Vec<glam::DVec3> = server
            .mobs
            .cows()
            .iter()
            .map(|c| c.cow.body.position)
            .chain(server.mobs.sheep().iter().map(|s| s.body.position))
            .collect();
        server.set_simulation_area((20, 128), 5);
        for _ in 0..400 {
            server.tick();
            server.tick_mobs(&[], true);
        }
        let end: Vec<glam::DVec3> = server
            .mobs
            .cows()
            .iter()
            .map(|c| c.cow.body.position)
            .chain(server.mobs.sheep().iter().map(|s| s.body.position))
            .collect();
        let moved = start
            .iter()
            .zip(&end)
            .filter(|(a, b)| a.distance(**b) > 0.5)
            .count();
        assert!(
            moved > 0,
            "some animal strolls within 20 seconds: {start:?} -> {end:?}"
        );
        assert!(
            end.iter().all(|p| p.y > 40.0),
            "animals stay on the ground: {end:?}"
        );
        // The client tracks the ones near its player.
        let near = server.tracked_mobs([328.0, 80.0, 2056.0], 32.0);
        assert!(animals(&near) <= animals(&server.mobs));
    }

    /// Generated animals wander about a player among them, climbing the
    /// terrain's steps (`jumpFromGround`); around a spectator they idle
    /// once `noActionTime` passes 100, as `Mob.checkDespawn` only resets it
    /// near a player it counts.
    #[test]
    fn idle_animals_wander_near_a_player() {
        let Ok(paths) = DataPaths::discover() else {
            return;
        };
        let Ok(registries) = Registries::load(&paths) else {
            return;
        };
        let registries = Arc::new(registries);
        let worldgen = Arc::new(
            WorldGen::new(Arc::new(
                TerrainGenerator::overworld(registries.clone(), 1234).unwrap(),
            ))
            .unwrap(),
        );
        let states = Arc::new(BlockStates::new(registries.clone(), 1234, -64, 384).unwrap());
        let mut map = ChunkMap::with_worldgen(worldgen.clone(), 6, 8);
        let mut server = ServerSim::new(worldgen, states, "minecraft:overworld");
        for x in 15..=25 {
            for z in 123..=133 {
                server.load_chunk(&map.load_now(ChunkPos::new(x, z)));
            }
        }
        // Among the first animals generated.
        let herd = server
            .mobs
            .cows()
            .iter()
            .map(|e| e.cow.body.position)
            .chain(server.mobs.sheep().iter().map(|e| e.body.position))
            .next()
            .expect("animals");
        let mut player = minecraftoss_entities::tempt::PlayerCandidate {
            id: 0,
            position: herd + glam::DVec3::new(3.0, 4.0, 0.0),
            eye_height: 1.62,
            main_hand_cow_food: false,
            offhand_cow_food: false,
            main_hand_pig_food: false,
            offhand_pig_food: false,
            main_hand_chicken_food: false,
            offhand_chicken_food: false,
            main_hand_carrot_on_a_stick: false,
            offhand_carrot_on_a_stick: false,
            main_hand_wolf_interest: false,
            offhand_wolf_interest: false,
            main_hand_horse_tempt: false,
            offhand_horse_tempt: false,
            alive: true,
            spectator: false,
            attackable: true, visibility: 1.0,
        };
        let mut share = Vec::new();
        for spectator in [false, true] {
            player.spectator = spectator;
            let (mut moving, mut samples) = (0, 0);
            let mut last: std::collections::HashMap<u64, glam::DVec3> = Default::default();
            for tick in 0..1200 {
                server.set_players(&[player.position.to_array()], 2);
                server.set_simulation_area((20, 128), 5);
                server.tick();
                server.despawn_mobs(&[player], 2);
                server.tick_mobs(&[player], true);
                let w = &server.mobs;
                let animals = w
                    .cows()
                    .iter()
                    .map(|e| (e.id, e.cow.body.position))
                    .chain(w.sheep().iter().map(|e| (e.id, e.body.position)));
                for (id, p) in animals {
                    if p.distance(player.position) > 32.0 {
                        continue;
                    }
                    if let Some(old) = last.insert(id, p) {
                        if tick >= 200 {
                            samples += 1;
                            moving += usize::from(
                                glam::DVec2::new(old.x - p.x, old.z - p.z).length() > 0.01,
                            );
                        }
                    }
                }
            }
            share.push(moving as f64 / samples.max(1) as f64);
        }
        assert!(share[0] > 0.15, "animals walk about a player: {share:?}");
        assert!(share[1] < 0.01, "and idle about a spectator: {share:?}");
    }

    /// Mobs leave with their chunk into the entity storage and come back
    /// from it as they were: where they wandered, hurt, sheared, and one
    /// killed stays dead, not respawned from generation.
    #[test]
    fn mobs_are_saved_with_their_chunks() {
        let Ok(paths) = DataPaths::discover() else {
            return;
        };
        let Ok(registries) = Registries::load(&paths) else {
            return;
        };
        let registries = Arc::new(registries);
        let worldgen = Arc::new(
            WorldGen::new(Arc::new(
                TerrainGenerator::overworld(registries.clone(), 1234).unwrap(),
            ))
            .unwrap(),
        );
        let states = Arc::new(BlockStates::new(registries.clone(), 1234, -64, 384).unwrap());
        let mut map = ChunkMap::with_worldgen(worldgen.clone(), 6, 8);
        let dir =
            std::env::temp_dir().join(format!("minecraftoss-entity-save-{}", std::process::id()));
        let storage = Arc::new(minecraftoss_world::storage::ChunkStorage::new(
            &dir,
            "minecraft:overworld",
            registries.clone(),
            -64,
            384,
        ));
        let mut server = ServerSim::new(worldgen, states, "minecraft:overworld");
        server.set_entity_storage(Some(storage));
        let chunks: Vec<std::sync::Arc<Chunk>> = (15..=25)
            .flat_map(|x| (123..=133).map(move |z| (x, z)))
            .map(|(x, z)| map.load_now(ChunkPos::new(x, z)))
            .collect();
        for chunk in &chunks {
            server.load_chunk(chunk);
        }
        server.set_simulation_area((20, 128), 5);
        for _ in 0..100 {
            server.tick();
            server.tick_mobs(&[], true);
        }
        let animals = |world: &minecraftoss_entities::world::EntityWorld| {
            world.cows().iter().filter(|c| c.horse.is_none()).count()
                + world.pigs().len()
                + world.chickens().len()
                + world.sheep().len()
        };
        let before = animals(&server.mobs);
        assert!(before > 1, "seed 1234 has animals there");
        // Hurt one animal and kill another.
        let hurt_id = server.mobs.mob_ids().next().unwrap();
        let victim = server.mobs.mob_ids().nth(1).unwrap();
        let hurt_at = server.mobs.body_mut(hurt_id).unwrap().position;
        for (id, amount) in [(hurt_id, 1.0), (victim, 100.0)] {
            if let Some(e) = server.mobs.cow_mut(id) {
                e.hurt(
                    amount,
                    minecraftoss_entities::world::DamageSourceKind::Generic,
                );
            }
            if let Some(e) = server.mobs.sheep_mut(id) {
                e.hurt(
                    amount,
                    minecraftoss_entities::world::DamageSourceKind::Generic,
                );
            }
            if let Some(e) = server.mobs.pig_mut(id) {
                e.hurt(
                    amount,
                    minecraftoss_entities::world::DamageSourceKind::Generic,
                );
            }
            if let Some(e) = server.mobs.chicken_mut(id) {
                e.hurt(amount);
            }
        }
        let positions: Vec<glam::DVec3> = server
            .mobs
            .cows()
            .iter()
            .map(|c| c.cow.body.position)
            .collect();
        for chunk in &chunks {
            server.unload_chunk(chunk.pos);
        }
        assert_eq!(animals(&server.mobs), 0, "unloading takes the mobs out");
        for chunk in &chunks {
            server.load_chunk(chunk);
        }
        assert_eq!(
            animals(&server.mobs),
            before - 1,
            "they come back without the one killed"
        );
        let back: Vec<glam::DVec3> = server
            .mobs
            .cows()
            .iter()
            .map(|c| c.cow.body.position)
            .collect();
        assert!(
            positions
                .iter()
                .all(|p| back.iter().any(|b| b.distance(*p) < 1.0e-9)),
            "where they were: {positions:?} -> {back:?}"
        );
        let health = [
            server
                .mobs
                .cows()
                .iter()
                .map(|c| (c.cow.body.position, c.cow.health))
                .collect::<Vec<_>>(),
            server
                .mobs
                .sheep()
                .iter()
                .map(|s| (s.body.position, s.health))
                .collect(),
            server
                .mobs
                .pigs()
                .iter()
                .map(|p| (p.pig.body.position, p.pig.health))
                .collect(),
            server
                .mobs
                .chickens()
                .iter()
                .map(|c| (c.chicken.body.position, c.chicken.health))
                .collect(),
        ]
        .concat();
        let (_, restored) = health
            .iter()
            .find(|(p, _)| p.distance(hurt_at) < 2.0)
            .copied()
            .expect("the hurt animal is back");
        assert!(restored < 10.0, "hurt as it was: {restored}");
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// At midnight on Normal, natural spawning fills the dark around a
    /// player with monsters up to the cap, and they despawn once the player
    /// is far away.
    #[test]
    fn night_spawns_monsters_around_the_player() {
        let Ok(paths) = DataPaths::discover() else {
            return;
        };
        let Ok(registries) = Registries::load(&paths) else {
            return;
        };
        if registries.entities.is_none() {
            return;
        }
        let registries = Arc::new(registries);
        let worldgen = Arc::new(
            WorldGen::new(Arc::new(
                TerrainGenerator::overworld(registries.clone(), 1234).unwrap(),
            ))
            .unwrap(),
        );
        let states = Arc::new(BlockStates::new(registries.clone(), 1234, -64, 384).unwrap());
        let mut map = ChunkMap::with_worldgen(worldgen.clone(), 6, 8);
        let mut server = ServerSim::new(worldgen, states, "minecraft:overworld");
        for x in 11..=29 {
            for z in 119..=137 {
                server.load_chunk(&map.load_now(ChunkPos::new(x, z)));
            }
        }
        let surface = minecraftoss_generator::feature::World::height_at(
            &server.level,
            minecraftoss_core::chunk::HeightmapKind::MotionBlocking,
            328,
            2056,
        );
        let mut player = minecraftoss_entities::tempt::PlayerCandidate {
            id: 0,
            position: glam::DVec3::new(328.5, f64::from(surface), 2056.5),
            eye_height: 1.62,
            main_hand_cow_food: false,
            offhand_cow_food: false,
            main_hand_pig_food: false,
            offhand_pig_food: false,
            main_hand_chicken_food: false,
            offhand_chicken_food: false,
            main_hand_carrot_on_a_stick: false,
            offhand_carrot_on_a_stick: false,
            main_hand_wolf_interest: false,
            offhand_wolf_interest: false,
            main_hand_horse_tempt: false,
            offhand_horse_tempt: false,
            alive: true,
            spectator: false,
            attackable: true, visibility: 1.0,
        };
        let monsters = |server: &ServerSim| {
            let census = server.mobs.census();
            // Dormant monsters count; generated minecarts and horses wait
            // dormant too.
            let monster = |kind: &str| {
                minecraftoss_world::natural_spawner::spawning::MobCategory::of_type(kind)
                    == Some(minecraftoss_world::natural_spawner::spawning::MobCategory::Monster)
            };
            census
                .iter()
                .filter(|m| ["minecraft:zombie", "minecraft:skeleton"].contains(&m.kind))
                .count()
                + server
                    .dormant
                    .iter()
                    .filter(|(g, _)| monster(&g[0].kind))
                    .count()
        };
        server.set_time(18000);
        for _ in 0..40 {
            server.set_players(&[player.position.to_array()], 2);
            server.set_simulation_area((20, 128), 5);
            server.prepare_spawning(&[player]);
            server.tick();
            server.take_spawned();
            server.despawn_mobs(&[player], 2);
            server.tick_mobs(&[player], false);
        }
        let spawned = monsters(&server);
        assert!(spawned > 20, "the night brings monsters: {spawned}");
        // The cap is 70 for the 289 chunks around one player, overshot at
        // most by one tick's spawning.
        assert!(spawned <= 70 + 40, "monsters stay near the cap: {spawned}");
        assert!(
            server
                .mobs
                .zombies()
                .iter()
                .all(|z| z.zombie.body.position.distance(player.position) <= 132.0 + 40.0)
        );
        // Far away, they despawn.
        player.position.x += 3000.0;
        server.prepare_spawning(&[player]);
        server.despawn_mobs(&[player], 2);
        assert_eq!(monsters(&server), 0, "monsters beyond 128 blocks despawn");
    }

    /// `/summon`: a mob made without NBT is finalized as a fresh spawn and
    /// joins the entity world; NBT is loaded over the fresh mob; monsters
    /// are refused in peaceful.
    /// A summoned villager's brain walks it about at the time of day it
    /// idles in.
    #[test]
    fn villagers_wander_by_day() {
        let Ok(paths) = DataPaths::discover() else {
            return;
        };
        let Ok(registries) = Registries::load(&paths) else {
            return;
        };
        if registries.entities.is_none() {
            return;
        }
        let registries = Arc::new(registries);
        let worldgen = Arc::new(
            WorldGen::new(Arc::new(
                TerrainGenerator::overworld(registries.clone(), 1234).unwrap(),
            ))
            .unwrap(),
        );
        let states = Arc::new(BlockStates::new(registries.clone(), 1234, -64, 384).unwrap());
        let mut map = ChunkMap::with_worldgen(worldgen.clone(), 6, 8);
        let mut server = ServerSim::new(worldgen, states, "minecraft:overworld");
        for x in 19..=21 {
            for z in 127..=129 {
                server.load_chunk(&map.load_now(ChunkPos::new(x, z)));
            }
        }
        let surface = minecraftoss_generator::feature::World::height_at(
            &server.level,
            minecraftoss_core::chunk::HeightmapKind::MotionBlocking,
            328,
            2056,
        );
        let at = [328.5, f64::from(surface), 2056.5];
        server.set_players(&[at], 2);
        server.set_time(1000);
        server.summon("minecraft:villager", at, None, 0.0).unwrap();
        let start = server.mobs.villagers()[0].villager.body.position;
        let mut farthest = 0.0_f64;
        for _ in 0..600 {
            server.set_players(&[at], 2);
            server.set_simulation_area((20, 128), 4);
            server.tick();
            server.tick_mobs(&[], true);
            let villager = &server.mobs.villagers()[0];
            assert!(villager.villager.health > 0.0, "it lives");
            farthest = farthest.max(villager.villager.body.position.distance(start));
        }
        assert!(farthest > 1.0, "it wandered ({farthest})");
        // At night a homeless villager rests: it heads for (any) village.
        server.set_time(13000);
        for _ in 0..40 {
            server.set_players(&[at], 2);
            server.set_simulation_area((20, 128), 4);
            server.tick();
            server.tick_mobs(&[], true);
        }
        let brain = &server.mobs.villagers()[0].ai.as_ref().unwrap().brain;
        assert!(
            brain
                .activities
                .active
                .contains(&minecraftoss_entities::villager_brain::Activity::Rest),
            "it rests at night"
        );
    }

    /// A summoned iron golem strolls about (no village near: any spot),
    /// and comes back from its saved tag cracked, built by a player or not.
    #[test]
    fn iron_golems_stroll_and_keep_their_tags() {
        let Ok(paths) = DataPaths::discover() else {
            return;
        };
        let Ok(registries) = Registries::load(&paths) else {
            return;
        };
        if registries.entities.is_none() {
            return;
        }
        let registries = Arc::new(registries);
        let worldgen = Arc::new(
            WorldGen::new(Arc::new(
                TerrainGenerator::overworld(registries.clone(), 1234).unwrap(),
            ))
            .unwrap(),
        );
        let states = Arc::new(BlockStates::new(registries.clone(), 1234, -64, 384).unwrap());
        let mut map = ChunkMap::with_worldgen(worldgen.clone(), 6, 8);
        let mut server = ServerSim::new(worldgen, states, "minecraft:overworld");
        for x in 19..=21 {
            for z in 127..=129 {
                server.load_chunk(&map.load_now(ChunkPos::new(x, z)));
            }
        }
        let surface = minecraftoss_generator::feature::World::height_at(
            &server.level,
            minecraftoss_core::chunk::HeightmapKind::MotionBlocking,
            328,
            2056,
        );
        let at = [328.5, f64::from(surface), 2056.5];
        server.set_players(&[at], 2);
        server.set_time(1000);
        let nbt = minecraftoss_core::snbt::parse_compound("{PlayerCreated:1b}").unwrap();
        server
            .summon("minecraft:iron_golem", at, Some(&nbt), 0.0)
            .unwrap();
        let start = server.mobs.iron_golems()[0].golem.body.position;
        let mut farthest = 0.0_f64;
        for _ in 0..2000 {
            server.set_players(&[at], 2);
            server.set_simulation_area((20, 128), 4);
            server.tick();
            server.tick_mobs(&[], true);
            let golem = &server.mobs.iron_golems()[0];
            assert!(golem.golem.health > 0.0, "it lives");
            farthest = farthest.max(golem.golem.body.position.distance(start));
        }
        assert!(farthest > 1.0, "it strolled ({farthest})");
        let id = server.mobs.iron_golems()[0].id;
        server.mobs.iron_golem_mut(id).unwrap().hurt(55.0);
        let tags =
            crate::server_mobs::mob_tags(&server.mobs, |_| true, &std::collections::HashMap::new());
        let (_, tag) = tags
            .iter()
            .find(|(tag_id, _)| *tag_id == id)
            .expect("the golem is saved");
        let mut loaded = minecraftoss_entities::world::EntityWorld::default();
        crate::server_mobs::spawn_saved(&mut loaded, tag).expect("golems load");
        let golem = &loaded.iron_golems()[0].golem;
        assert_eq!(golem.health, 45.0);
        assert!(golem.player_created, "built by a player");
        assert_eq!(
            golem.crackiness(),
            minecraftoss_entities::iron_golem::Crackiness::Medium
        );
    }

    /// A summoned wolf wanders about, and comes back from its saved tag
    /// with its variant, voice, collar, owner and sitting order.
    #[test]
    fn wolves_wander_and_keep_their_tags() {
        let Ok(paths) = DataPaths::discover() else {
            return;
        };
        let Ok(registries) = Registries::load(&paths) else {
            return;
        };
        if registries.entities.is_none() {
            return;
        }
        let registries = Arc::new(registries);
        let worldgen = Arc::new(
            WorldGen::new(Arc::new(
                TerrainGenerator::overworld(registries.clone(), 1234).unwrap(),
            ))
            .unwrap(),
        );
        let states = Arc::new(BlockStates::new(registries.clone(), 1234, -64, 384).unwrap());
        let mut map = ChunkMap::with_worldgen(worldgen.clone(), 6, 8);
        let mut server = ServerSim::new(worldgen, states, "minecraft:overworld");
        for x in 19..=21 {
            for z in 127..=129 {
                server.load_chunk(&map.load_now(ChunkPos::new(x, z)));
            }
        }
        let surface = minecraftoss_generator::feature::World::height_at(
            &server.level,
            minecraftoss_core::chunk::HeightmapKind::MotionBlocking,
            328,
            2056,
        );
        let at = [328.5, f64::from(surface), 2056.5];
        server.set_players(&[at], 2);
        server.set_time(1000);
        let nbt = minecraftoss_core::snbt::parse_compound(
            "{variant:\"minecraft:woods\",sound_variant:\"minecraft:big\"}",
        )
        .unwrap();
        server
            .summon("minecraft:wolf", at, Some(&nbt), 0.0)
            .unwrap();
        let start = server.mobs.wolves()[0].wolf.body.position;
        let mut farthest = 0.0_f64;
        for _ in 0..1200 {
            server.set_players(&[at], 2);
            server.set_simulation_area((20, 128), 4);
            server.tick();
            server.tick_mobs(&[], true);
            let wolf = &server.mobs.wolves()[0];
            assert!(wolf.wolf.health > 0.0, "it lives");
            farthest = farthest.max(wolf.wolf.body.position.distance(start));
        }
        assert!(farthest > 1.0, "it wandered ({farthest})");
        let id = server.mobs.wolves()[0].id;
        {
            // Tamed by someone and told to sit, in a blue collar.
            let wolf = &mut server.mobs.wolf_mut(id).unwrap().wolf;
            wolf.owner = Some(0x1234_5678_9abc_def0_0fed_cba9_8765_4321);
            wolf.set_tame(true, true);
            wolf.ordered_to_sit = true;
            wolf.collar = 11;
        }
        let tags =
            crate::server_mobs::mob_tags(&server.mobs, |_| true, &std::collections::HashMap::new());
        let (_, tag) = tags
            .iter()
            .find(|(tag_id, _)| *tag_id == id)
            .expect("the wolf is saved");
        let mut loaded = minecraftoss_entities::world::EntityWorld::default();
        crate::server_mobs::spawn_saved(&mut loaded, tag).expect("wolves load");
        let wolf = &loaded.wolves()[0].wolf;
        assert_eq!(
            (wolf.variant.as_str(), wolf.sound_variant.as_str()),
            ("woods", "big")
        );
        assert_eq!(wolf.max_health(), 40.0, "taming's health stays");
        assert_eq!(wolf.collar, 11);
        assert_eq!(wolf.owner, Some(0x1234_5678_9abc_def0_0fed_cba9_8765_4321));
        assert!(
            wolf.tame && wolf.ordered_to_sit && wolf.sitting,
            "tame and sitting"
        );
    }

    /// A librarian makes its offers from the data JAR's trade sets when
    /// they are first needed; they are saved with it (`Offers`, typed as
    /// vanilla writes them) and come back unchanged, while a villager that
    /// never made any saves none. Its gossip and UUID come back too.
    #[test]
    fn villagers_keep_their_offers_and_gossip() {
        use minecraftoss_core::nbt::Tag;
        let Ok(paths) = DataPaths::discover() else {
            return;
        };
        let Ok(registries) = Registries::load(&paths) else {
            return;
        };
        let jar = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../harness/.gradle/loom-cache/minecraftMaven/net/minecraft/minecraft-common-1fad6b3808/26.3/minecraft-common-1fad6b3808-26.3.jar");
        if !jar.exists() || registries.entities.is_none() {
            return;
        }
        let registries = Arc::new(registries);
        let worldgen = Arc::new(
            WorldGen::new(Arc::new(
                TerrainGenerator::overworld(registries.clone(), 1234).unwrap(),
            ))
            .unwrap(),
        );
        let states = Arc::new(BlockStates::new(registries.clone(), 1234, -64, 384).unwrap());
        let mut server = ServerSim::new(worldgen, states, "minecraft:overworld");
        server.load_loot(&jar, 1234);
        let nbt = minecraftoss_core::snbt::parse_compound("{NoAI:1b,VillagerData:{type:\"minecraft:plains\",profession:\"minecraft:librarian\",level:3}}").unwrap();
        server
            .summon("minecraft:villager", [0.5, 100.0, 0.5], Some(&nbt), 0.0)
            .unwrap();
        server
            .summon("minecraft:villager", [4.5, 100.0, 0.5], Some(&nbt), 0.0)
            .unwrap();
        let (id, idle) = (server.mobs.villagers()[0].id, server.mobs.villagers()[1].id);
        let offers = server
            .mobs
            .villager_offers(id)
            .expect("a villager")
            .to_vec();
        let about = server.mobs.uuid_of(idle);
        server.mobs.villager_hurt_by(id, idle);
        std::sync::Arc::make_mut(&mut server.mobs.villager_mut(id).unwrap().gossips).add(
            7,
            minecraftoss_entities::gossip::GossipType::Trading,
            12,
        );
        server.mobs.villager_mut(id).unwrap().last_gossip_decay = 1234;
        let gossip = server.mobs.villager_mut(id).unwrap().gossips.unpack();
        assert_eq!(gossip.len(), 2);
        // `updateTrades`: the two of its current level's set only.
        assert_eq!(offers.len(), 2, "{offers:?}");
        let tags =
            crate::server_mobs::mob_tags(&server.mobs, |_| true, &std::collections::HashMap::new());
        let (_, tag) = tags
            .iter()
            .find(|(tag_id, _)| *tag_id == id)
            .expect("the villager is saved");
        let recipes = tag
            .get("Offers")
            .and_then(|o| o.get("Recipes"))
            .and_then(Tag::as_list)
            .expect("its offers are saved");
        assert_eq!(recipes.len(), 2);
        assert!(
            recipes
                .iter()
                .all(|r| matches!(r.get("priceMultiplier"), Some(Tag::Float(_)))),
            "price multipliers are floats"
        );
        assert!(matches!(tag.get("LastRestock"), Some(Tag::Long(0))));
        let (_, idle_tag) = tags.iter().find(|(tag_id, _)| *tag_id == idle).unwrap();
        assert!(
            idle_tag.get("Offers").is_none(),
            "no offers made, none saved"
        );
        let mut loaded = minecraftoss_entities::world::EntityWorld::default();
        let back = crate::server_mobs::spawn_saved(&mut loaded, tag).expect("villagers load");
        assert_eq!(
            loaded.villager_mut(back).unwrap().offers.as_deref(),
            Some(&offers[..])
        );
        assert_eq!(loaded.villager_mut(back).unwrap().gossips.unpack(), gossip);
        assert_eq!(loaded.villager_mut(back).unwrap().last_gossip_decay, 1234);
        assert_eq!(
            loaded.uuid_of(back),
            server.mobs.uuid_of(id),
            "it keeps its UUID"
        );
        assert!(
            gossip.iter().any(|g| g.0 == about),
            "gossip about the other villager"
        );
    }

    /// Using a farmer opens its trades; picking one moves the wheat in,
    /// shift-clicking the result trades until the wheat runs short (levelling
    /// the farmer up and dropping experience), and closing gives the rest
    /// back.
    #[test]
    fn players_trade_with_villagers() {
        use minecraftoss_player::inventory::{Inventory, ItemStack};
        let Ok(paths) = DataPaths::discover() else {
            return;
        };
        let Ok(registries) = Registries::load(&paths) else {
            return;
        };
        let jar = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../harness/.gradle/loom-cache/minecraftMaven/net/minecraft/minecraft-common-1fad6b3808/26.3/minecraft-common-1fad6b3808-26.3.jar");
        if !jar.exists() || registries.entities.is_none() {
            return;
        }
        let registries = Arc::new(registries);
        let worldgen = Arc::new(
            WorldGen::new(Arc::new(
                TerrainGenerator::overworld(registries.clone(), 1234).unwrap(),
            ))
            .unwrap(),
        );
        let states = Arc::new(BlockStates::new(registries.clone(), 1234, -64, 384).unwrap());
        let mut server = ServerSim::new(worldgen, states, "minecraft:overworld");
        server.load_loot(&jar, 1234);
        let nbt = minecraftoss_core::snbt::parse_compound(
            "{NoAI:1b,Xp:8,VillagerData:{type:\"minecraft:plains\",profession:\"minecraft:farmer\",level:1},Offers:{Recipes:[{buy:{id:\"minecraft:wheat\",count:20},sell:{id:\"minecraft:emerald\",count:1},maxUses:16,xp:2,priceMultiplier:0.05f}]}}",
        )
        .unwrap();
        server
            .summon("minecraft:villager", [0.5, 100.0, 0.5], Some(&nbt), 0.0)
            .unwrap();
        let id = server.mobs.villagers()[0].id;
        let mut inventory = Inventory::default();
        let mut wheat = ItemStack::new("minecraft:wheat", 50);
        wheat.max = 64;
        inventory.slots[9] = Some(wheat);
        let result = server.mob_action(
            minecraftoss_entities::world::MobHit::Villager(id),
            None,
            inventory.clone(),
            0,
            false,
        );
        let view = result.merchant.expect("the trading screen opens");
        assert_eq!(
            (view.profession, view.level, view.offers.len()),
            ("minecraft:farmer", 1, 1)
        );
        let apply = |inventory: &mut Inventory, update: &MerchantUpdate| {
            for (slot, stack) in &update.slots {
                inventory.slots[*slot] = stack.clone();
            }
            inventory.cursor = update.cursor.clone();
        };
        let update = server.merchant(MerchantOp::Select(0), inventory.clone(), 0, [0.0; 3]);
        apply(&mut inventory, &update);
        let view = update.view.expect("still open");
        assert_eq!(
            view.payment[0].as_ref().map(|s| s.count),
            Some(50),
            "the wheat moved in"
        );
        assert_eq!(
            view.result.as_ref().map(|s| s.id.as_str()),
            Some("minecraft:emerald")
        );
        let update = server.merchant(
            MerchantOp::Result { shift: true },
            inventory.clone(),
            0,
            [0.0; 3],
        );
        apply(&mut inventory, &update);
        let view = update.view.expect("still open");
        assert_eq!(
            view.payment[0].as_ref().map(|s| s.count),
            Some(10),
            "two trades of twenty"
        );
        assert_eq!(inventory.count("minecraft:emerald"), 2);
        assert_eq!(
            (view.level, view.xp),
            (2, 12),
            "the first trade levelled the farmer up"
        );
        assert!(view.offers.len() > 1, "with its next level's offers");
        assert!(
            server.level.experience_total() > 0,
            "trades dropped experience"
        );
        let update = server.merchant(MerchantOp::Close, inventory.clone(), 0, [0.0; 3]);
        apply(&mut inventory, &update);
        assert!(update.view.is_none(), "closed");
        assert_eq!(inventory.count("minecraft:wheat"), 10, "the rest came back");
        assert!(server.mobs.villagers()[0].trading_player.is_none());
    }

    /// A villager claims the bed and bell placed near it by day, and sleeps
    /// in the bed at night: the level's blocks feed the points of interest.
    #[test]
    fn villagers_claim_beds_and_sleep() {
        let Ok(paths) = DataPaths::discover() else {
            return;
        };
        let Ok(registries) = Registries::load(&paths) else {
            return;
        };
        if registries.entities.is_none() {
            return;
        }
        let registries = Arc::new(registries);
        let worldgen = Arc::new(
            WorldGen::new(Arc::new(
                TerrainGenerator::overworld(registries.clone(), 1234).unwrap(),
            ))
            .unwrap(),
        );
        let states = Arc::new(BlockStates::new(registries.clone(), 1234, -64, 384).unwrap());
        let mut map = ChunkMap::with_worldgen(worldgen.clone(), 6, 8);
        let mut server = ServerSim::new(worldgen, states, "minecraft:overworld");
        for x in 19..=21 {
            for z in 127..=129 {
                server.load_chunk(&map.load_now(ChunkPos::new(x, z)));
            }
        }
        let height = |server: &ServerSim, x, z| {
            minecraftoss_generator::feature::World::height_at(
                &server.level,
                minecraftoss_core::chunk::HeightmapKind::MotionBlocking,
                x,
                z,
            )
        };
        let surface = height(&server, 328, 2056);
        let place = |server: &mut ServerSim, x: i32, z: i32, block: Block| {
            let y = height(server, x, z);
            let state = server.state_of(Some(&block));
            server
                .level
                .set_block_and_update(minecraftoss_core::BlockPos::new(x, y, z), state);
            (x, y, z)
        };
        let bell = place(&mut server, 330, 2056, Block::new("minecraft:bell"));
        let bed = |part: &str| {
            Block::new("minecraft:red_bed")
                .with("facing", "east")
                .with("part", part)
                .with("occupied", "false")
        };
        place(&mut server, 326, 2058, bed("foot"));
        let head = place(&mut server, 327, 2058, bed("head"));
        let at = [328.5, f64::from(surface), 2056.5];
        server.set_players(&[at], 2);
        server.set_time(1000);
        server.summon("minecraft:villager", at, None, 0.0).unwrap();
        let tick = |server: &mut ServerSim| {
            server.set_players(&[at], 2);
            server.set_simulation_area((20, 128), 4);
            server.tick();
            server.tick_mobs(&[], true);
        };
        for _ in 0..200 {
            tick(&mut server);
        }
        let memories = &server.mobs.villagers()[0]
            .ai
            .as_ref()
            .unwrap()
            .brain
            .memories;
        assert_eq!(
            memories.home.get().copied(),
            Some(head),
            "it claims the bed"
        );
        assert_eq!(
            memories.meeting_point.get().copied(),
            Some(bell),
            "it claims the bell"
        );
        assert_eq!(
            server.mobs.pois.record(head).map(|r| r.free_tickets),
            Some(0)
        );
        server.set_time(12500);
        let mut slept = false;
        for _ in 0..600 {
            tick(&mut server);
            slept |= server.mobs.villagers()[0].sleeping == Some(head);
        }
        assert!(slept, "it sleeps in its bed at night");
        // A lectern nearby by day: it becomes a librarian and works there.
        let lectern = place(&mut server, 331, 2054, Block::new("minecraft:lectern"));
        server.set_time(1800);
        let mut worked = false;
        for _ in 0..800 {
            tick(&mut server);
            worked |= server.mobs.villagers()[0]
                .ai
                .as_ref()
                .unwrap()
                .brain
                .running()
                .iter()
                .any(|b| b == "RunOne:WorkAtPoi");
        }
        let villager = &server.mobs.villagers()[0];
        assert_eq!(
            villager.villager.profession,
            minecraftoss_entities::villager::Profession::Librarian,
            "it takes the lectern's profession"
        );
        assert_eq!(
            villager
                .ai
                .as_ref()
                .unwrap()
                .brain
                .memories
                .job_site
                .get()
                .copied(),
            Some(lectern)
        );
        assert!(worked, "it works at its lectern");
    }

    /// A farmer takes the composter by its field, harvests the ripe wheat
    /// (the level drops its loot, which the farmer picks up) and sows the
    /// bare farmland again.
    #[test]
    fn farmers_harvest_and_sow() {
        let Ok(paths) = DataPaths::discover() else {
            return;
        };
        let Ok(registries) = Registries::load(&paths) else {
            return;
        };
        if registries.entities.is_none() {
            return;
        }
        let registries = Arc::new(registries);
        let worldgen = Arc::new(
            WorldGen::new(Arc::new(
                TerrainGenerator::overworld(registries.clone(), 1234).unwrap(),
            ))
            .unwrap(),
        );
        let states = Arc::new(BlockStates::new(registries.clone(), 1234, -64, 384).unwrap());
        let mut map = ChunkMap::with_worldgen(worldgen.clone(), 6, 8);
        let mut server = ServerSim::new(worldgen, states, "minecraft:overworld");
        for x in 19..=21 {
            for z in 127..=129 {
                server.load_chunk(&map.load_now(ChunkPos::new(x, z)));
            }
        }
        let height = |server: &ServerSim, x, z| {
            minecraftoss_generator::feature::World::height_at(
                &server.level,
                minecraftoss_core::chunk::HeightmapKind::MotionBlocking,
                x,
                z,
            )
        };
        let ground = height(&server, 328, 2056);
        let set = |server: &mut ServerSim, (x, y, z): (i32, i32, i32), block: Block| {
            let state = server.state_of(Some(&block));
            server
                .level
                .set_block_and_update(minecraftoss_core::BlockPos::new(x, y, z), state);
        };
        // A flat stone yard with a three-by-three field of ripe wheat beside
        // a composter.
        for x in 322..=334 {
            for z in 2050..=2062 {
                for y in ground..ground + 4 {
                    set(&mut server, (x, y, z), Block::new("minecraft:air"));
                }
                set(
                    &mut server,
                    (x, ground - 1, z),
                    Block::new("minecraft:stone"),
                );
            }
        }
        let mut field = Vec::new();
        for x in 328..=330 {
            for z in 2055..=2057 {
                set(
                    &mut server,
                    (x, ground - 1, z),
                    Block::new("minecraft:farmland").with("moisture", "7"),
                );
                set(
                    &mut server,
                    (x, ground, z),
                    Block::new("minecraft:wheat").with("age", "7"),
                );
                field.push((x, ground, z));
            }
        }
        set(
            &mut server,
            (327, ground, 2056),
            Block::new("minecraft:composter").with("level", "0"),
        );
        let at = [326.5, f64::from(ground), 2056.5];
        server.set_players(&[at], 2);
        server.set_time(3000);
        let farmer = minecraftoss_core::snbt::parse_compound(
            "{CanPickUpLoot:1b,Xp:1,VillagerData:{type:\"minecraft:plains\",profession:\"minecraft:farmer\",level:1},Inventory:[{id:\"minecraft:wheat_seeds\",count:4}]}",
        )
        .unwrap();
        server
            .summon("minecraft:villager", at, Some(&farmer), 0.0)
            .unwrap();
        let age = |server: &ServerSim, (x, y, z): (i32, i32, i32)| {
            let state = server
                .level
                .block(minecraftoss_core::BlockPos::new(x, y, z));
            let blocks = &server.level.registries().blocks;
            (
                blocks
                    .block(blocks.block_of(state))
                    .name
                    .as_str()
                    .to_owned(),
                blocks.property(state, "age").map(str::to_owned),
            )
        };
        let mut sown = false;
        for _ in 0..2400 {
            server.set_players(&[at], 2);
            server.set_simulation_area((20, 128), 4);
            server.tick();
            server.tick_mobs(&[], true);
            sown |= field.iter().any(|&pos| {
                age(&server, pos) == ("minecraft:wheat".to_owned(), Some("0".to_owned()))
            });
            if sown {
                break;
            }
        }
        let villager = &server.mobs.villagers()[0];
        assert!(
            villager
                .ai
                .as_ref()
                .unwrap()
                .brain
                .memories
                .job_site
                .get()
                .is_some(),
            "it takes the composter"
        );
        assert!(sown, "it harvested ripe wheat and sowed the farmland again");
        assert!(
            field
                .iter()
                .any(|&pos| age(&server, pos).1.as_deref() != Some("7")),
            "a ripe crop was harvested"
        );
    }

    /// A villager with plenty of bread throws some to one with none as they
    /// meet, and the hungry one picks it up from the level.
    #[test]
    fn villagers_share_food() {
        let Ok(paths) = DataPaths::discover() else {
            return;
        };
        let Ok(registries) = Registries::load(&paths) else {
            return;
        };
        if registries.entities.is_none() {
            return;
        }
        let registries = Arc::new(registries);
        let worldgen = Arc::new(
            WorldGen::new(Arc::new(
                TerrainGenerator::overworld(registries.clone(), 1234).unwrap(),
            ))
            .unwrap(),
        );
        let states = Arc::new(BlockStates::new(registries.clone(), 1234, -64, 384).unwrap());
        let mut map = ChunkMap::with_worldgen(worldgen.clone(), 6, 8);
        let mut server = ServerSim::new(worldgen, states, "minecraft:overworld");
        for x in 19..=21 {
            for z in 127..=129 {
                server.load_chunk(&map.load_now(ChunkPos::new(x, z)));
            }
        }
        let height = |server: &ServerSim, x, z| {
            minecraftoss_generator::feature::World::height_at(
                &server.level,
                minecraftoss_core::chunk::HeightmapKind::MotionBlocking,
                x,
                z,
            )
        };
        let at = [328.5, f64::from(height(&server, 328, 2056)), 2056.5];
        server.set_players(&[at], 2);
        server.set_time(1000);
        let plenty = minecraftoss_core::snbt::parse_compound(
            "{CanPickUpLoot:1b,Inventory:[{id:\"minecraft:bread\",count:30}]}",
        )
        .unwrap();
        let none = minecraftoss_core::snbt::parse_compound("{CanPickUpLoot:1b}").unwrap();
        server
            .summon("minecraft:villager", at, Some(&plenty), 0.0)
            .unwrap();
        server
            .summon(
                "minecraft:villager",
                [at[0] + 2.0, at[1], at[2] + 1.0],
                Some(&none),
                90.0,
            )
            .unwrap();
        let hungry = server.mobs.villagers()[1].id;
        let fed = |server: &ServerSim| {
            server
                .mobs
                .villagers()
                .iter()
                .find(|v| v.id == hungry)
                .unwrap()
                .inventory
                .count("minecraft:bread")
        };
        for _ in 0..1200 {
            server.set_players(&[at], 2);
            server.set_simulation_area((20, 128), 4);
            server.tick();
            server.tick_mobs(&[], true);
            if fed(&server) > 0 {
                break;
            }
        }
        assert_eq!(
            fed(&server),
            6,
            "six loaves thrown (thirty less twenty-four) and picked up"
        );
        assert_eq!(
            server.mobs.villagers()[0]
                .inventory
                .count("minecraft:bread"),
            24
        );
    }

    /// Two villagers with bread near free beds have a baby, which takes a
    /// bed; they eat their bread and rest from breeding.
    #[test]
    fn villagers_breed_into_a_free_bed() {
        let Ok(paths) = DataPaths::discover() else {
            return;
        };
        let Ok(registries) = Registries::load(&paths) else {
            return;
        };
        if registries.entities.is_none() {
            return;
        }
        let registries = Arc::new(registries);
        let worldgen = Arc::new(
            WorldGen::new(Arc::new(
                TerrainGenerator::overworld(registries.clone(), 1234).unwrap(),
            ))
            .unwrap(),
        );
        let states = Arc::new(BlockStates::new(registries.clone(), 1234, -64, 384).unwrap());
        let mut map = ChunkMap::with_worldgen(worldgen.clone(), 6, 8);
        let mut server = ServerSim::new(worldgen, states, "minecraft:overworld");
        for x in 19..=21 {
            for z in 127..=129 {
                server.load_chunk(&map.load_now(ChunkPos::new(x, z)));
            }
        }
        let height = |server: &ServerSim, x, z| {
            minecraftoss_generator::feature::World::height_at(
                &server.level,
                minecraftoss_core::chunk::HeightmapKind::MotionBlocking,
                x,
                z,
            )
        };
        let place = |server: &mut ServerSim, x: i32, z: i32, block: Block| {
            let y = height(server, x, z);
            let state = server.state_of(Some(&block));
            server
                .level
                .set_block_and_update(minecraftoss_core::BlockPos::new(x, y, z), state);
            (x, y, z)
        };
        let bed = |part: &str| {
            Block::new("minecraft:red_bed")
                .with("facing", "east")
                .with("part", part)
                .with("occupied", "false")
        };
        let mut beds = Vec::new();
        for z in [2052, 2055, 2058] {
            place(&mut server, 326, z, bed("foot"));
            beds.push(place(&mut server, 327, z, bed("head")));
        }
        let at = [329.5, f64::from(height(&server, 329, 2055)), 2055.5];
        server.set_players(&[at], 2);
        server.set_time(1000);
        let fed = minecraftoss_core::snbt::parse_compound(
            "{CanPickUpLoot:1b,Inventory:[{id:\"minecraft:bread\",count:3}]}",
        )
        .unwrap();
        server
            .summon("minecraft:villager", at, Some(&fed), 0.0)
            .unwrap();
        server
            .summon(
                "minecraft:villager",
                [at[0] + 2.0, at[1], at[2] + 1.0],
                Some(&fed),
                90.0,
            )
            .unwrap();
        let parents: Vec<u64> = server.mobs.villagers().iter().map(|v| v.id).collect();
        for _ in 0..2400 {
            server.set_players(&[at], 2);
            server.set_simulation_area((20, 128), 4);
            server.tick();
            server.tick_mobs(&[], true);
            if server.mobs.villagers().len() > 2 {
                break;
            }
        }
        let villagers = server.mobs.villagers();
        let baby = villagers
            .iter()
            .find(|v| !parents.contains(&v.id))
            .expect("the villagers had a baby");
        assert!(baby.villager.age.baby());
        let home = baby.ai.as_ref().unwrap().brain.memories.home.get().copied();
        assert!(
            home.is_some_and(|h| beds.contains(&h)),
            "the baby has a bed: {home:?}"
        );
        for parent in villagers.iter().filter(|v| parents.contains(&v.id)) {
            assert!(
                parent.villager.age.ticks > 0,
                "the parents rest from breeding"
            );
            assert_eq!(
                parent.inventory.food_points(),
                0,
                "the parents ate their bread"
            );
        }
    }

    /// A villager that can pick things up walks to bread lying near it and
    /// takes it; one summoned without `CanPickUpLoot` leaves its bread be,
    /// and the flag survives saving.
    #[test]
    fn villagers_pick_up_food() {
        let Ok(paths) = DataPaths::discover() else {
            return;
        };
        let Ok(registries) = Registries::load(&paths) else {
            return;
        };
        if registries.entities.is_none() {
            return;
        }
        let registries = Arc::new(registries);
        let worldgen = Arc::new(
            WorldGen::new(Arc::new(
                TerrainGenerator::overworld(registries.clone(), 1234).unwrap(),
            ))
            .unwrap(),
        );
        let states = Arc::new(BlockStates::new(registries.clone(), 1234, -64, 384).unwrap());
        let mut map = ChunkMap::with_worldgen(worldgen.clone(), 6, 8);
        let mut server = ServerSim::new(worldgen, states, "minecraft:overworld");
        for x in 19..=21 {
            for z in 127..=129 {
                server.load_chunk(&map.load_now(ChunkPos::new(x, z)));
            }
        }
        let height = |server: &ServerSim, x, z| {
            minecraftoss_generator::feature::World::height_at(
                &server.level,
                minecraftoss_core::chunk::HeightmapKind::MotionBlocking,
                x,
                z,
            )
        };
        let at = [328.5, f64::from(height(&server, 328, 2056)), 2056.5];
        let other = [334.5, f64::from(height(&server, 334, 2062)), 2062.5];
        server.set_players(&[at], 2);
        server.set_time(1000);
        let loot = minecraftoss_core::snbt::parse_compound("{CanPickUpLoot:1b}").unwrap();
        server
            .summon("minecraft:villager", at, Some(&loot), 0.0)
            .unwrap();
        server
            .summon("minecraft:villager", other, None, 0.0)
            .unwrap();
        let (picker, idle) = (server.mobs.villagers()[0].id, server.mobs.villagers()[1].id);
        let bread =
            |count| minecraftoss_world::level::container::Stack::new("minecraft:bread", count);
        let near = [at[0] + 2.0, f64::from(height(&server, 330, 2056)), at[2]];
        let kept = server.level.spawn_item_with(
            [other[0] + 0.5, other[1], other[2]],
            bread(2),
            [0.0; 3],
            0,
            0,
        );
        let taken = server.level.spawn_item_with(near, bread(3), [0.0; 3], 0, 0);
        for _ in 0..200 {
            server.set_players(&[at], 2);
            server.set_simulation_area((20, 128), 4);
            server.tick();
            server.tick_mobs(&[], true);
        }
        let villager = |id| server.mobs.villagers().iter().find(|v| v.id == id).unwrap();
        assert_eq!(
            villager(picker).inventory.food_points(),
            12,
            "it took the three loaves"
        );
        assert!(
            server.level.item_entity(taken).is_none(),
            "the bread is gone"
        );
        assert!(
            !villager(idle).can_pick_up_loot,
            "a summoned villager is told whether it may"
        );
        assert_eq!(villager(idle).inventory.food_points(), 0);
        assert_eq!(
            server
                .level
                .item_entity(kept)
                .map(|(_, stack, _)| stack.count),
            Some(2),
            "its bread lies where it was"
        );
        let tags =
            crate::server_mobs::mob_tags(&server.mobs, |_| true, &std::collections::HashMap::new());
        let (_, tag) = tags.iter().find(|(tag_id, _)| *tag_id == picker).unwrap();
        let mut loaded = minecraftoss_entities::world::EntityWorld::default();
        let back = crate::server_mobs::spawn_saved(&mut loaded, tag).unwrap();
        let back = loaded.villager_mut(back).unwrap();
        assert!(back.can_pick_up_loot);
        assert_eq!(back.inventory.food_points(), 12, "its inventory is saved");
    }

    #[test]
    fn summon_makes_mobs() {
        let Ok(paths) = DataPaths::discover() else {
            return;
        };
        let Ok(registries) = Registries::load(&paths) else {
            return;
        };
        if registries.entities.is_none() {
            return;
        }
        let registries = Arc::new(registries);
        let worldgen = Arc::new(
            WorldGen::new(Arc::new(
                TerrainGenerator::overworld(registries.clone(), 1234).unwrap(),
            ))
            .unwrap(),
        );
        let states = Arc::new(BlockStates::new(registries.clone(), 1234, -64, 384).unwrap());
        let mut map = ChunkMap::with_worldgen(worldgen.clone(), 6, 8);
        let mut server = ServerSim::new(worldgen, states, "minecraft:overworld");
        for x in 19..=21 {
            for z in 127..=129 {
                server.load_chunk(&map.load_now(ChunkPos::new(x, z)));
            }
        }
        let surface = minecraftoss_generator::feature::World::height_at(
            &server.level,
            minecraftoss_core::chunk::HeightmapKind::MotionBlocking,
            328,
            2056,
        );
        let at = [328.5, f64::from(surface), 2056.5];
        server.set_players(&[at], 2);
        let before = server.mobs.zombies().len();
        server.summon("minecraft:husk", at, None, 1.5).unwrap();
        let nbt = minecraftoss_core::snbt::parse_compound(r#"{IsBaby:1b,PersistenceRequired:1b,VillagerData:{type:"minecraft:desert",profession:"minecraft:farmer",level:1}}"#).unwrap();
        server
            .summon(
                "minecraft:zombie_villager",
                [at[0] + 2.0, at[1], at[2]],
                Some(&nbt),
                0.0,
            )
            .unwrap();
        let zombies = server.mobs.zombies();
        assert_eq!(zombies.len(), before + 2, "both join the entity world");
        let husk = zombies
            .iter()
            .find(|z| z.zombie.kind == minecraftoss_entities::zombie::ZombieKind::Husk)
            .unwrap();
        assert_eq!(
            husk.zombie.body.position.to_array(),
            at,
            "at the command's position"
        );
        let villager = zombies
            .iter()
            .find(|z| z.zombie.kind == minecraftoss_entities::zombie::ZombieKind::ZombieVillager)
            .unwrap();
        assert!(
            villager.zombie.baby && villager.zombie.persistence_required,
            "the NBT is loaded"
        );
        assert_eq!(
            villager.zombie.villager,
            Some(("minecraft:desert".to_owned(), "minecraft:farmer".to_owned()))
        );
        server.summon("minecraft:cow", at, None, 0.0).unwrap();
        assert_eq!(server.mobs.cows().len(), 1, "animals are summoned too");
        // A villager loads without its brain (not ported) instead of
        // tripping the entity world's assertion.
        server.summon("minecraft:villager", at, None, 0.0).unwrap();
        assert_eq!(server.mobs.villagers().len(), 1);
        // `NoAI` holds a mob still, turned to its saved yaw.
        let still =
            minecraftoss_core::snbt::parse_compound("{NoAI:1b,Rotation:[180f,0f]}").unwrap();
        server
            .summon(
                "minecraft:zombie",
                [at[0] - 2.0, at[1], at[2]],
                Some(&still),
                0.0,
            )
            .unwrap();
        let zombie = server
            .mobs
            .zombies()
            .iter()
            .find(|z| z.no_ai)
            .expect("a NoAI zombie");
        assert_eq!(
            (
                zombie.yaw,
                zombie.body_rotation.body_yaw,
                zombie.look_control.head_yaw
            ),
            (180.0, 180.0, 180.0)
        );
        let id = zombie.id;
        for _ in 0..20 {
            server.tick_mobs(&[], false);
        }
        let zombie = server.mobs.zombies().iter().find(|z| z.id == id).unwrap();
        assert_eq!(
            zombie.zombie.body.position.to_array(),
            [at[0] - 2.0, at[1], at[2]],
            "it stays put"
        );
        // A horse loads its variant and grazing (`EatingHaystack`), which
        // lasts fifty ticks though it has no AI.
        let grazing = minecraftoss_core::snbt::parse_compound(
            "{NoAI:1b,Variant:1027,EatingHaystack:1b,Age:-24000}",
        )
        .unwrap();
        server
            .summon(
                "minecraft:horse",
                [at[0] + 4.0, at[1], at[2]],
                Some(&grazing),
                0.0,
            )
            .unwrap();
        let horse = server
            .mobs
            .cows()
            .iter()
            .find_map(|c| c.horse.clone())
            .expect("a horse");
        assert_eq!((horse.variant, horse.eating), (1027, true));
        for _ in 0..40 {
            server.set_simulation_area((20, 128), 4);
            server.tick();
            server.tick_mobs(&[], false);
        }
        let horse = server
            .mobs
            .cows()
            .iter()
            .find_map(|c| c.horse.clone())
            .unwrap();
        assert!(horse.eating && horse.eat_anim == 1.0, "{horse:?}");
        server.set_players(&[at], 0);
        assert!(
            server.summon("minecraft:zombie", at, None, 0.0).is_err(),
            "no monsters in peaceful"
        );
        assert!(
            server.summon("minecraft:cow", at, None, 0.0).is_ok(),
            "animals come in peaceful"
        );
    }

    /// A creeper sees the player across a platform, walks up, swells and
    /// blows up: the blast breaks the planks (mobs may grief), hurts and
    /// pushes the player, and the client hears of it.
    #[test]
    fn creeper_stalks_and_blows_up_the_player() {
        use minecraftoss_core::nbt::Tag;
        let Ok(paths) = DataPaths::discover() else {
            return;
        };
        let Ok(registries) = Registries::load(&paths) else {
            return;
        };
        let registries = Arc::new(registries);
        let worldgen = Arc::new(
            WorldGen::new(Arc::new(
                TerrainGenerator::overworld(registries.clone(), 0).unwrap(),
            ))
            .unwrap(),
        );
        let states = Arc::new(BlockStates::new(registries.clone(), 0, -64, 384).unwrap());
        let mut map = ChunkMap::with_worldgen(worldgen.clone(), 2, 4);
        let mut server = ServerSim::new(worldgen, states, "minecraft:overworld");
        for x in -1..=2 {
            for z in -1..=1 {
                server.load_chunk(&map.load_now(ChunkPos::new(x, z)));
            }
        }
        // A plank platform high above the terrain.
        for x in 0..=24 {
            for z in 0..=12 {
                server.player_edit_block(
                    (x, 200, z),
                    Some(&Block::new("minecraft:oak_planks")),
                    PlayerEdit::Place,
                );
            }
        }
        server.take_changes();
        let player = minecraftoss_entities::tempt::PlayerCandidate {
            id: 0,
            position: glam::DVec3::new(18.5, 201.0, 6.5),
            eye_height: 1.62,
            main_hand_cow_food: false,
            offhand_cow_food: false,
            main_hand_pig_food: false,
            offhand_pig_food: false,
            main_hand_chicken_food: false,
            offhand_chicken_food: false,
            main_hand_carrot_on_a_stick: false,
            offhand_carrot_on_a_stick: false,
            main_hand_wolf_interest: false,
            offhand_wolf_interest: false,
            main_hand_horse_tempt: false,
            offhand_horse_tempt: false,
            alive: true,
            spectator: false,
            attackable: true, visibility: 1.0,
        };
        let mut tag = std::collections::BTreeMap::new();
        tag.insert("id".to_owned(), Tag::String("minecraft:creeper".to_owned()));
        tag.insert(
            "Pos".to_owned(),
            Tag::List(vec![Tag::Double(8.5), Tag::Double(201.0), Tag::Double(6.5)]),
        );
        tag.insert(
            "Rotation".to_owned(),
            Tag::List(vec![Tag::Float(0.0), Tag::Float(0.0)]),
        );
        tag.insert("UUID".to_owned(), Tag::IntArray(vec![1, 2, 3, 4]));
        let id = crate::server_mobs::spawn_saved(&mut server.mobs, &Tag::Compound(tag))
            .expect("creepers are simulated");
        assert!(
            server
                .mobs
                .creepers()
                .iter()
                .any(|c| c.id == id && !c.no_ai)
        );
        server.set_time(18000);
        let (mut hits, mut blasts, mut broken) = (Vec::new(), Vec::new(), 0);
        for _ in 0..400 {
            server.set_players(&[player.position.to_array()], 2);
            server.set_simulation_area((0, 0), 4);
            server.tick();
            server.tick_mobs(&[player], false);
            hits.extend(server.mobs.take_player_hits());
            blasts.extend(server.take_explosions());
            broken += server
                .take_changes()
                .iter()
                .filter(|(pos, block)| pos.1 == 200 && block.is_none())
                .count();
            if !blasts.is_empty() {
                break;
            }
        }
        assert_eq!(blasts.len(), 1, "the creeper explodes");
        assert!(
            server.mobs.creepers().iter().all(|c| c.id != id),
            "and is gone"
        );
        let blast = blasts[0];
        assert!(
            blast.position.distance(player.position) < 3.5,
            "beside the player: {:?}",
            blast.position
        );
        assert!(broken > 0, "the blast breaks planks");
        let hit = hits
            .iter()
            .find(|h| {
                matches!(
                    h.kind,
                    minecraftoss_entities::world::PlayerHitKind::Explosion { .. }
                )
            })
            .expect("the blast hits the player");
        assert!(hit.damage > 10.0, "a close blast hurts: {}", hit.damage);
    }

    /// A player's wheat makes a generated cow fall in love (using up the
    /// wheat), and hits kill it, dropping its loot into the level.
    #[test]
    fn players_feed_and_hit_server_mobs() {
        let Ok(paths) = DataPaths::discover() else {
            return;
        };
        let Ok(registries) = Registries::load(&paths) else {
            return;
        };
        let jar = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../harness/.gradle/loom-cache/minecraftMaven/net/minecraft/minecraft-common-1fad6b3808/26.3/minecraft-common-1fad6b3808-26.3.jar");
        if !jar.exists() {
            return;
        }
        let registries = Arc::new(registries);
        let worldgen = Arc::new(
            WorldGen::new(Arc::new(
                TerrainGenerator::overworld(registries.clone(), 1234).unwrap(),
            ))
            .unwrap(),
        );
        let states = Arc::new(BlockStates::new(registries.clone(), 1234, -64, 384).unwrap());
        let mut map = ChunkMap::with_worldgen(worldgen.clone(), 4, 8);
        let mut server = ServerSim::new(worldgen, states, "minecraft:overworld");
        server.load_loot(&jar, 1234);
        let recipes = Arc::new(minecraftoss_player::crafting::RecipeBook::from_jar(&jar).unwrap());
        server.set_recipe_book(recipes.clone());
        for x in 20..=23 {
            for z in 129..=132 {
                server.load_chunk(&map.load_now(ChunkPos::new(x, z)));
            }
        }
        server.set_simulation_area((21, 130), 4);
        let cow = server
            .mobs
            .cows()
            .iter()
            .find(|c| !c.cow.age.baby())
            .map(|c| c.id)
            .expect("an adult cow near (356, 65, 2097)");
        let hit = minecraftoss_entities::world::MobHit::Cow(cow);
        let mut inventory = minecraftoss_player::inventory::Inventory::default();
        inventory.recipes = recipes;
        inventory.slots[0] = Some(minecraftoss_player::inventory::ItemStack::new(
            "minecraft:wheat",
            3,
        ));
        let fed = server.mob_action(hit, None, inventory.clone(), 0, false);
        assert_eq!(fed.slots.len(), 1, "one wheat is used: {:?}", fed.slots);
        assert_eq!(fed.slots[0].1.as_ref().map(|s| s.count), Some(2));
        assert!(
            server.mobs.cow_mut(cow).unwrap().cow.in_love > 0,
            "the cow falls in love"
        );
        // Full-strength bare-handed hits, one point each past each hurt
        // cooldown, until it dies.
        let items_before = server.items().len();
        let mut sounds = Vec::new();
        for _ in 0..40 {
            let at = server
                .mobs
                .cows()
                .iter()
                .find(|c| c.id == cow)
                .map(|c| c.cow.body.position);
            let Some(at) = at else { break };
            let attack = minecraftoss_entities::world::PlayerAttack {
                player_id: 0,
                position: at + glam::DVec3::X,
                yaw: 90.0,
                attack_damage: 1.0,
                strength: 1.0,
                sprinting: false,
                can_critical: false,
                can_sweep: false,
            };
            let result = server.mob_action(hit, Some(attack), inventory.clone(), 0, false);
            sounds.extend(
                result
                    .sounds
                    .into_iter()
                    .map(|s| s.event)
                    .filter(|s| s.starts_with("entity.cow")),
            );
            for _ in 0..11 {
                server.tick_mobs(&[], true);
            }
            if sounds.last().is_some_and(|s| s.ends_with("death")) {
                break;
            }
        }
        assert!(
            sounds.first().is_some_and(|s| s.ends_with(".hurt")),
            "{sounds:?}"
        );
        assert!(
            sounds.last().is_some_and(|s| s.ends_with(".death")),
            "the cow dies: {sounds:?}"
        );
        assert!(
            server.items().len() > items_before,
            "its loot drops into the level"
        );
    }

    /// Five experience land as orbs of vanilla's sizes (3, 1 and 1); the
    /// orbs are drawn to a player standing beside them, who takes one every
    /// other tick until none are left.
    #[test]
    fn experience_orbs_are_awarded_and_taken() {
        let Ok(paths) = DataPaths::discover() else {
            return;
        };
        let Ok(registries) = Registries::load(&paths) else {
            return;
        };
        let registries = Arc::new(registries);
        let worldgen = Arc::new(
            WorldGen::new(Arc::new(
                TerrainGenerator::overworld(registries.clone(), 0).unwrap(),
            ))
            .unwrap(),
        );
        let states = Arc::new(BlockStates::new(registries.clone(), 0, -64, 384).unwrap());
        let mut map = ChunkMap::with_worldgen(worldgen.clone(), 2, 4);
        let mut server = ServerSim::new(worldgen, states.clone(), "minecraft:overworld");
        let mut scene = HandcraftedScene::streamed(states);
        for x in -1..=1 {
            for z in -1..=1 {
                let chunk = map.load_now(ChunkPos::new(x, z));
                server.load_chunk(&chunk);
                scene.insert_chunk(chunk);
            }
        }
        // A floating platform, high above the terrain.
        for x in 2..=14 {
            for z in 2..=14 {
                scene.set((x, 200, z), Some(Block::new("minecraft:stone")));
                server.player_edit(&scene, (x, 200, z), PlayerEdit::Place);
            }
        }
        server.award_experience([8.5, 201.0, 8.5], 5);
        let mut values: Vec<i32> = server.orbs().iter().map(|o| o.value).collect();
        values.sort_unstable();
        assert_eq!(values, [1, 1, 3]);
        let feet = [10.5, 201.0, 8.5];
        server.level.living_players = vec![(feet, 1.62)];
        let mut taken = Vec::new();
        for tick in 0..60 {
            server.tick();
            if let Some((_, _, value)) = server.take_experience(Some(feet)) {
                taken.push((tick, value));
            }
        }
        assert_eq!(
            taken.iter().map(|(_, v)| v).sum::<i32>(),
            5,
            "every orb is taken: {taken:?}"
        );
        assert!(
            taken.windows(2).all(|w| w[1].0 - w[0].0 >= 2),
            "one orb every other tick at most: {taken:?}"
        );
        assert!(server.orbs().is_empty());
    }

    /// A hopper minecart takes the items over it (any pickup delay) as far
    /// as its five slots hold them; what lies beside it stays.
    #[test]
    fn a_hopper_minecart_takes_what_lies_over_it() {
        let Ok(paths) = DataPaths::discover() else {
            return;
        };
        let Ok(registries) = Registries::load(&paths) else {
            return;
        };
        let registries = Arc::new(registries);
        let worldgen = Arc::new(WorldGen::new(Arc::new(TerrainGenerator::overworld(registries.clone(), 0).unwrap())).unwrap());
        let states = Arc::new(BlockStates::new(registries.clone(), 0, -64, 384).unwrap());
        let mut map = ChunkMap::with_worldgen(worldgen.clone(), 2, 4);
        let mut server = ServerSim::new(worldgen, states.clone(), "minecraft:overworld");
        let mut scene = HandcraftedScene::streamed(states);
        for x in -1..=1 {
            for z in -1..=1 {
                let chunk = map.load_now(ChunkPos::new(x, z));
                server.load_chunk(&chunk);
                scene.insert_chunk(chunk);
            }
        }
        for x in 2..=14 {
            for z in 2..=14 {
                scene.set((x, 200, z), Some(Block::new("minecraft:stone")));
                server.player_edit(&scene, (x, 200, z), PlayerEdit::Place);
            }
        }
        // Over the cart (at 8.5, 201, 8.5): 70 cobblestone, with a long
        // delay; beside it, far off: dirt.
        server.spawn_item("minecraft:cobblestone", 64, None, [8.5, 201.9, 8.5], [0.0; 3], 40, 0);
        server.spawn_item("minecraft:cobblestone", 6, None, [8.5, 202.2, 8.5], [0.0; 3], 40, 0);
        server.spawn_item("minecraft:dirt", 5, None, [12.5, 201.2, 12.5], [0.0; 3], 0, 0);
        // Four slots full of stone, one free: one stack's worth fits.
        let full = Some(minecraftoss_player::inventory::ItemStack::new("minecraft:stone", 64));
        let slots = vec![full.clone(), full.clone(), full.clone(), full, None];
        let taken = server.hopper_pickup([8.5, 201.0, 8.5], &slots);
        let count: i32 = taken.iter().filter(|(item, ..)| item == "minecraft:cobblestone").map(|(_, n, _)| n).sum();
        assert_eq!(count, 64, "{taken:?}");
        assert!(taken.iter().all(|(item, ..)| item != "minecraft:dirt"));
        // With room, the rest.
        let taken = server.hopper_pickup([8.5, 201.0, 8.5], &[None, None, None, None, None]);
        assert_eq!(taken.iter().map(|(_, n, _)| n).sum::<i32>(), 6, "{taken:?}");
    }

    /// `cargo test -j 6 -p minecraft_terrain --lib mob_tick_cost -- --nocapture`
    #[test]
    fn mob_tick_cost() {
        let paths = DataPaths::discover().expect("Minecraft data paths");
        let registries = Registries::load(&paths).expect("Minecraft registries");
        let registries = Arc::new(registries);
        let worldgen = Arc::new(
            WorldGen::new(Arc::new(
                TerrainGenerator::overworld(registries.clone(), 1234).unwrap(),
            ))
            .unwrap(),
        );
        let states = Arc::new(BlockStates::new(registries.clone(), 1234, -64, 384).unwrap());
        let mut map = ChunkMap::with_worldgen(worldgen.clone(), 10, 16);
        let mut server = ServerSim::new(worldgen, states, "minecraft:overworld");
        for x in 10..=30 {
            for z in 118..=138 {
                server.load_chunk(&map.load_now(ChunkPos::new(x, z)));
            }
        }
        server.set_simulation_area((20, 128), 10);
        let mut times = Vec::new();
        for _ in 0..600 {
            let started = std::time::Instant::now();
            server.tick_mobs(&[], true);
            times.push(started.elapsed().as_secs_f64() * 1000.0);
        }
        times.sort_by(f64::total_cmp);
        let p = |q: usize| times[(times.len() - 1) * q / 100];
        eprintln!(
            "{} mobs: tick p50 {:.2} p95 {:.2} p99 {:.2} max {:.2} ms",
            server.mobs.len(),
            p(50),
            p(95),
            p(99),
            times[times.len() - 1]
        );
    }

    #[test]
    fn skull_gem_and_items_do_not_fall_into_unloaded_chunks() {
        use minecraftoss_core::mashup_skull_gem as gem;
        let paths=DataPaths::discover().expect("Minecraft pack");
        let registries=Arc::new(Registries::load(&paths).expect("registries"));
        let worldgen=Arc::new(WorldGen::new(Arc::new(TerrainGenerator::overworld(registries.clone(),0).unwrap())).unwrap());
        let states=Arc::new(BlockStates::new(registries,0,-64,384).unwrap());
        let mut map=ChunkMap::with_worldgen(worldgen.clone(),2,4);
        let mut scene=HandcraftedScene::streamed(states.clone());
        let mut server=ServerSim::new(worldgen,states,"minecraft:overworld");
        let chunk=map.load_now(ChunkPos::new(0,0));server.load_chunk(&chunk);scene.insert_chunk(chunk);
        for x in 0..16 {for z in 0..16 {
            scene.set((x,200,z),Some(Block::new("minecraft:stone")));
            server.player_edit(&scene,(x,200,z),PlayerEdit::Place);
        }}
        server.spawn_item(gem::ID,3,None,[15.5,201.0,8.0],[2.0,0.0,0.0],1000,0);
        let bread=server.spawn_item("minecraft:bread",2,None,[15.5,201.0,10.0],[2.0,0.0,0.0],1000,0);
        server.spawn_item("minecraft:bread",1,None,[15.95,201.0,12.0],[2.0,0.0,0.0],1000,0);
        let waiting=server.spawn_item(gem::ID,4,None,[20.0,201.0,8.0],[0.0;3],1000,0);
        for _ in 0..80 {server.tick();}
        let items=server.items();assert_eq!(items.iter().map(|i|i.count).sum::<i32>(),10);
        for item in items.iter().filter(|i|i.id!=waiting) {
            assert!((0.0..16.0).contains(&item.position[0]),"crossed unloaded border");
            assert!(item.position[1]>=201.0-1e-6,"fell through missing terrain");
        }
        let pending=items.iter().find(|i|i.id==waiting).unwrap();
        assert_eq!(pending.position,[20.0,201.0,8.0]);assert_eq!(pending.age,0);
        let chunk=map.load_now(ChunkPos::new(1,0));server.load_chunk(&chunk);scene.insert_chunk(chunk);
        for x in 16..32 {for z in 0..16 {
            scene.set((x,200,z),Some(Block::new("minecraft:stone")));
            server.player_edit(&scene,(x,200,z),PlayerEdit::Place);
        }}
        server.level.entities.iter_mut().find(|i|i.id==bread).unwrap().delta=[0.5,0.0,0.0];
        for _ in 0..5 {server.tick();}
        let items=server.items();
        assert!(items.iter().find(|i|i.id==bread).unwrap().position[0]>16.0,"border disappears when the neighbor arrives");
        assert_eq!(items.iter().find(|i|i.id==waiting).unwrap().age,5);
    }

    #[test]
    fn skull_gem_homing_respects_delay_walls_and_survives_disk_reload() {
        use minecraftoss_core::mashup_skull_gem as gem;
        let paths=DataPaths::discover().expect("Minecraft pack");
        let registries=Arc::new(Registries::load(&paths).expect("registries"));
        let worldgen=Arc::new(WorldGen::new(Arc::new(TerrainGenerator::overworld(registries.clone(),0).unwrap())).unwrap());
        let states=Arc::new(BlockStates::new(registries.clone(),0,-64,384).unwrap());
        let mut map=ChunkMap::with_worldgen(worldgen.clone(),2,4);
        let chunk=map.load_now(ChunkPos::new(0,0));
        let mut scene=HandcraftedScene::streamed(states.clone());
        scene.insert_chunk(chunk.clone());
        let mut server=ServerSim::new(worldgen.clone(),states.clone(),"minecraft:overworld");
        server.load_chunk(&chunk);
        for x in 0..16 {for z in 0..16 {
            scene.set((x,200,z),Some(Block::new("minecraft:stone")));
            server.player_edit(&scene,(x,200,z),PlayerEdit::Place);
        }}
        let feet=[12.0,201.0,8.0];
        let target=[feet[0],feet[1]+0.9,feet[2]];
        server.spawn_item(gem::ID,7,None,[8.0,201.0,8.0],[0.0;3],20,0);
        server.level.skull_gem_collector=Some(target);
        for _ in 0..19 {server.tick();}
        assert!(server.items()[0].skull_gem.unwrap().suction.is_none(),"throw delay prevents immediate return");
        for z in 4..=12 {for y in 201..=205 {
            scene.set((10,y,z),Some(Block::new("minecraft:stone")));
            server.player_edit(&scene,(10,y,z),PlayerEdit::Place);
        }}
        for _ in 0..40 {server.tick();}
        assert!(server.items()[0].skull_gem.unwrap().suction.is_none(),"no attraction through walls");
        for z in 4..=12 {for y in 201..=205 {
            scene.set((10,y,z),None);
            server.player_edit(&scene,(10,y,z),PlayerEdit::Break);
        }}
        // An unavailable collector (full inventory, dead or spectator) must
        // leave the item on its physical support, not suspended in the air.
        server.level.skull_gem_collector=None;
        for _ in 0..20 {server.tick();}
        assert!(server.items()[0].skull_gem.unwrap().suction.is_none());
        server.level.skull_gem_collector=Some(target);
        for _ in 0..8 {server.tick();}
        let before=server.items().remove(0);
        assert!(before.skull_gem.unwrap().suction.is_some());
        let dir=std::env::temp_dir().join(format!("jak-gem-homing-{}-{}",std::process::id(),std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()));
        let storage=|| Arc::new(minecraftoss_world::storage::ChunkStorage::new(&dir,"minecraft:overworld",registries.clone(),-64,384));
        server.set_entity_storage(Some(storage()));
        server.save_all_entities();
        drop(server);
        let mut server=ServerSim::new(worldgen,states,"minecraft:overworld");
        server.set_entity_storage(Some(storage()));
        let edits:Vec<_>=scene.chunk_edits((0,0)).0.unwrap().iter().map(|(&p,b)|(p,Some(b.clone()))).collect();
        server.load_chunk_with_edits(&chunk,&edits);
        assert_eq!(server.items().len(),1);
        let after=server.items().remove(0);
        assert_eq!(before.position,after.position);
        assert_eq!(before.velocity,after.velocity);
        assert_eq!(before.skull_gem.unwrap().suction.unwrap().speed,after.skull_gem.unwrap().suction.unwrap().speed);
        assert_eq!(before.skull_gem.unwrap().suction.unwrap().direction,after.skull_gem.unwrap().suction.unwrap().direction);
        server.level.skull_gem_collector=Some(target);
        let mut taken=0;
        for _ in 0..60 {
            server.tick();
            taken+=server.level.player_touch_items(feet,|s|s.count).iter().map(|(_,_,s)|s.count).sum::<i32>();
        }
        assert_eq!(taken,7,"one authoritative transfer for the entire stack");
        assert!(server.items().is_empty());
        server.save_all_entities();
        drop(server);
        assert_eq!(storage().load_entities(chunk.pos).unwrap().unwrap().iter().filter(|t|t.get("Item").is_some()).count(),0,"collected gems must not return on reload");
    }

    #[test]
    fn skull_gem_arc_persists_and_inventory_transfer_never_duplicates() {
        use minecraftoss_core::mashup_skull_gem as gem;
        let paths=DataPaths::discover().expect("Minecraft pack");
        let registries=Arc::new(Registries::load(&paths).expect("registries"));
        let worldgen=Arc::new(WorldGen::new(Arc::new(TerrainGenerator::overworld(registries.clone(),0).unwrap())).unwrap());
        let states=Arc::new(BlockStates::new(registries,0,-64,384).unwrap());
        let mut map=ChunkMap::with_worldgen(worldgen.clone(),2,4);
        let mut server=ServerSim::new(worldgen,states.clone(),"minecraft:overworld");
        let mut scene=HandcraftedScene::streamed(states);
        for x in -1..=1 {for z in -1..=1 {let chunk=map.load_now(ChunkPos::new(x,z));server.load_chunk(&chunk);scene.insert_chunk(chunk);}}
        for x in 1..=15 {for z in 1..=15 {
            scene.set((x,200,z),Some(Block::new("minecraft:stone")));
            server.player_edit(&scene,(x,200,z),PlayerEdit::Place);
        }}
        // The render stream can deliver its generated chunk again after an
        // edit or distance change. Physics must retain the built platform.
        let raw=scene.generated_chunk((0,0)).unwrap().clone();
        let edits:Vec<_>=scene.chunk_edits((0,0)).0.unwrap().iter().map(|(&p,b)|(p,Some(b.clone()))).collect();
        server.load_chunk_with_edits(&raw,&edits);
        server.spawn_item(gem::ID,3,None,[8.5,201.1,8.5],gem::launch(0.5,0.5),400,0);
        let mut highest=201.1f64;
        for _ in 0..13{server.tick();highest=highest.max(server.items()[0].position[1]);}
        let before=server.items().remove(0);
        let tags=crate::server_mobs::level_entity_tags(&server.level,|_|true);
        server.level.entities.clear();
        assert_eq!(tags.len(),1);
        assert!(crate::server_mobs::load_level_entity(&mut server.level,&tags[0]));
        let loaded=server.items().remove(0);
        assert_eq!(loaded.position,before.position);assert_eq!(loaded.velocity,before.velocity);
        let a=loaded.skull_gem.unwrap();let b=before.skull_gem.unwrap();
        assert_eq!((a.seconds,a.angle,a.spin,a.settled),(b.seconds,b.angle,b.spin,b.settled));
        for _ in 0..400 {
            server.tick();let item=server.items().remove(0);
            highest=highest.max(item.position[1]);
            assert!(item.position[1]>=201.0-1e-7,"gem tunneled through floor");
        }
        let item=server.items().remove(0);
        assert!((highest-201.1-20.0*20.0/(2.0*30.0)*gem::blocks_per_meter()).abs()<0.2,"native 20m/s apex: {highest}");
        assert!(item.skull_gem.unwrap().settled);assert!((item.position[1]-201.0).abs()<1e-6);
        assert!(server.level.player_touch_items(item.position,|_|0).is_empty(),"full inventory rejects transfer");
        assert_eq!(server.items()[0].count,3);
        let first=server.level.player_touch_items(item.position,|_|1);
        assert_eq!(first.iter().map(|(_,_,s)|s.count).sum::<i32>(),1);
        assert_eq!(server.items()[0].count,2);
        let rest=server.level.player_touch_items(item.position,|s|s.count);
        assert_eq!(rest.iter().map(|(_,_,s)|s.count).sum::<i32>(),2);
        assert!(server.level.player_touch_items(item.position,|s|s.count).is_empty(),"already taken");
    }

    #[test]
    fn skull_gem_batches_escape_initial_overlap_without_tunnelling_or_losing_items() {
        use minecraftoss_core::mashup_skull_gem as gem;
        let paths=DataPaths::discover().expect("Minecraft pack");
        let registries=Arc::new(Registries::load(&paths).expect("registries"));
        let worldgen=Arc::new(WorldGen::new(Arc::new(TerrainGenerator::overworld(registries.clone(),0).unwrap())).unwrap());
        let states=Arc::new(BlockStates::new(registries,0,-64,384).unwrap());
        let mut map=ChunkMap::with_worldgen(worldgen.clone(),2,4);
        let mut server=ServerSim::new(worldgen,states.clone(),"minecraft:overworld");
        let mut scene=HandcraftedScene::streamed(states);
        for x in -1..=1 {for z in -1..=1 {let chunk=map.load_now(ChunkPos::new(x,z));server.load_chunk(&chunk);scene.insert_chunk(chunk);}}
        // Closed room: both a ceiling and a wall can intercept the native arc.
        for x in 1..=15 {for z in 1..=15 {for y in 200..=206 {
            if y==200 || y==206 || x==1 || x==15 || z==1 || z==15 {
                scene.set((x,y,z),Some(Block::new("minecraft:stone")));
                server.player_edit(&scene,(x,y,z),PlayerEdit::Place);
            }
        }}}
        for i in 0..48 {
            let p=match i%4 {
                0 => [5.5,200.9,5.5], // born slightly inside the floor
                1 => [8.5,205.7,8.5], // the gem's top intersects the ceiling
                2 => [2.1,203.0,10.5], // sideways overlap with the wall
                _ => [11.5,201.1,11.5],
            };
            server.spawn_item(gem::ID,1,None,p,gem::launch(0.5,0.5),1000,0);
            server.tick();
        }
        for _ in 0..400 {server.tick();}
        let items=server.items();
        assert_eq!(items.iter().map(|i|i.count).sum::<i32>(),48,"merging must preserve every dropped gem");
        for item in &items {
            assert!((item.position[1]-201.0).abs()<1e-5,"unsupported gem after settling: {:?}",item.position);
            assert!(item.velocity.iter().all(|v|v.abs()<1e-6));
            assert!(item.skull_gem.unwrap().settled);
            assert!(item.age>=400,"an old local snapshot must not replace the server item");
        }
        // A settled gem must fall again if its support is removed.
        for x in 2..=14 {for z in 2..=14 {
            scene.set((x,200,z),Some(Block::new("minecraft:air")));
            server.player_edit(&scene,(x,200,z),PlayerEdit::Place);
        }}
        for _ in 0..20 {server.tick();}
        assert!(server.items().iter().all(|i|i.position[1]<200.0),"settled is not a permanent freeze flag");
    }

    /// The Mass Inverter's field (16/10 11:47): an item dropped in it floats
    /// up instead of falling; one outside falls.
    #[test]
    fn items_float_in_a_mass_inverter_field() {
        let Ok(paths) = DataPaths::discover() else {
            return;
        };
        let Ok(registries) = Registries::load(&paths) else {
            return;
        };
        let registries = Arc::new(registries);
        let worldgen = Arc::new(
            WorldGen::new(Arc::new(
                TerrainGenerator::overworld(registries.clone(), 0).unwrap(),
            ))
            .unwrap(),
        );
        let states = Arc::new(BlockStates::new(registries.clone(), 0, -64, 384).unwrap());
        let mut map = ChunkMap::with_worldgen(worldgen.clone(), 2, 4);
        let mut server = ServerSim::new(worldgen, states.clone(), "minecraft:overworld");
        for x in -1..=1 {
            for z in -1..=1 {
                let chunk = map.load_now(ChunkPos::new(x, z));
                server.load_chunk(&chunk);
            }
        }
        let stack = || minecraftoss_world::level::container::Stack::new("minecraft:bread", 1);
        let inside = server.level.spawn_item_with([4.5, 200.0, 4.5], stack(), [0.0; 3], 0, 0);
        let outside = server.level.spawn_item_with([12.5, 200.0, 12.5], stack(), [0.0; 3], 0, 0);
        server.level.weightless = vec![([4.5, 200.0, 4.5], 4.0)];
        for _ in 0..40 {
            server.tick();
        }
        let y = |id| server.items().iter().find(|i| i.id == id).map(|i| i.position[1]);
        let (a, b) = (y(inside).expect("floating bread"), y(outside).expect("falling bread"));
        assert!(a > 200.0, "it floats up in the field: {a}");
        assert!(b < 190.0, "it falls outside: {b}");
    }

    /// TNT primed by a placed redstone block becomes an entity the client
    /// can render, explodes, and leaves items the player can pick up.
    #[test]
    fn primed_tnt_explodes_and_drops_items() {
        let Ok(paths) = DataPaths::discover() else {
            return;
        };
        let Ok(registries) = Registries::load(&paths) else {
            return;
        };
        let registries = Arc::new(registries);
        let worldgen = Arc::new(
            WorldGen::new(Arc::new(
                TerrainGenerator::overworld(registries.clone(), 0).unwrap(),
            ))
            .unwrap(),
        );
        let states = Arc::new(BlockStates::new(registries.clone(), 0, -64, 384).unwrap());
        let mut map = ChunkMap::with_worldgen(worldgen.clone(), 2, 4);
        let mut server = ServerSim::new(worldgen, states.clone(), "minecraft:overworld");
        let mut scene = HandcraftedScene::streamed(states);
        for x in -1..=1 {
            for z in -1..=1 {
                let chunk = map.load_now(ChunkPos::new(x, z));
                server.load_chunk(&chunk);
                scene.insert_chunk(chunk);
            }
        }
        // A floating platform of planks with TNT on it, high above terrain.
        for x in 4..=12 {
            for z in 4..=12 {
                scene.set((x, 200, z), Some(Block::new("minecraft:oak_planks")));
                server.player_edit(&scene, (x, 200, z), PlayerEdit::Place);
            }
        }
        scene.set((8, 201, 8), Some(Block::new("minecraft:tnt")));
        server.player_edit(&scene, (8, 201, 8), PlayerEdit::Place);
        scene.set((9, 201, 8), Some(Block::new("minecraft:redstone_block")));
        server.player_edit(&scene, (9, 201, 8), PlayerEdit::Place);
        server.take_changes();
        assert_eq!(server.primed_tnt().len(), 1, "the TNT should prime");
        for _ in 0..90 {
            server.tick();
        }
        assert!(
            server.primed_tnt().is_empty(),
            "the TNT should have exploded"
        );
        let changes = server.take_changes();
        assert!(
            changes
                .iter()
                .any(|(pos, block)| *pos == (8, 200, 8) && block.is_none()),
            "the blast should break the planks under it"
        );
        let items = server.items();
        assert!(
            items.iter().any(|i| i.item == "minecraft:oak_planks"),
            "broken planks drop: {items:?}"
        );
        // Let the drops land, then pick them up standing among them.
        for _ in 0..60 {
            server.tick();
        }
        let item = server
            .items()
            .into_iter()
            .find(|i| i.pickup_delay == 0)
            .expect("a landed item");
        let mut taken = 0;
        let picked = server.pickup(
            [item.position[0], item.position[1], item.position[2]],
            |_, count, _| {
                taken += count;
                count
            },
        );
        assert!(
            !picked.is_empty() && taken > 0,
            "the player should pick up touching items"
        );
        assert!(
            server.items().iter().all(|i| i.id != picked[0].0),
            "a fully picked-up item is removed"
        );
    }

    /// Sand placed in the air falls as an entity and lands on the ground.
    #[test]
    fn placed_sand_falls_and_lands() {
        let Ok(paths) = DataPaths::discover() else {
            return;
        };
        let Ok(registries) = Registries::load(&paths) else {
            return;
        };
        let registries = Arc::new(registries);
        let worldgen = Arc::new(
            WorldGen::new(Arc::new(
                TerrainGenerator::overworld(registries.clone(), 0).unwrap(),
            ))
            .unwrap(),
        );
        let states = Arc::new(BlockStates::new(registries.clone(), 0, -64, 384).unwrap());
        let mut map = ChunkMap::with_worldgen(worldgen.clone(), 2, 4);
        let mut server = ServerSim::new(worldgen, states.clone(), "minecraft:overworld");
        let mut scene = HandcraftedScene::streamed(states);
        for x in -1..=1 {
            for z in -1..=1 {
                let chunk = map.load_now(ChunkPos::new(x, z));
                server.load_chunk(&chunk);
                scene.insert_chunk(chunk);
            }
        }
        scene.set((8, 200, 8), Some(Block::new("minecraft:stone")));
        server.player_edit(&scene, (8, 200, 8), PlayerEdit::Place);
        scene.set((8, 205, 8), Some(Block::new("minecraft:sand")));
        server.player_edit(&scene, (8, 205, 8), PlayerEdit::Place);
        server.take_changes();
        for _ in 0..3 {
            server.tick();
        }
        assert_eq!(
            server.falling_blocks().len(),
            1,
            "the sand should be falling"
        );
        for _ in 0..40 {
            server.tick();
        }
        assert!(
            server.falling_blocks().is_empty(),
            "the sand should have landed"
        );
        let changes = server.take_changes();
        assert!(
            changes.iter().any(|(pos, block)| *pos == (8, 201, 8)
                && block
                    .as_ref()
                    .is_some_and(|b| *b == Block::new("minecraft:sand"))),
            "{changes:?}"
        );
    }

    /// Bone meal the player uses on a crop grows it through the level.
    #[test]
    fn bone_meal_grows_a_crop() {
        let Ok(paths) = DataPaths::discover() else {
            return;
        };
        let Ok(registries) = Registries::load(&paths) else {
            return;
        };
        let registries = Arc::new(registries);
        let worldgen = Arc::new(
            WorldGen::new(Arc::new(
                TerrainGenerator::overworld(registries.clone(), 0).unwrap(),
            ))
            .unwrap(),
        );
        let states = Arc::new(BlockStates::new(registries.clone(), 0, -64, 384).unwrap());
        let mut map = ChunkMap::with_worldgen(worldgen.clone(), 2, 4);
        let mut server = ServerSim::new(worldgen, states.clone(), "minecraft:overworld");
        let mut scene = HandcraftedScene::streamed(states);
        for x in -1..=1 {
            for z in -1..=1 {
                let chunk = map.load_now(ChunkPos::new(x, z));
                server.load_chunk(&chunk);
                scene.insert_chunk(chunk);
            }
        }
        scene.set((8, 200, 8), Some(Block::new("minecraft:farmland")));
        server.player_edit(&scene, (8, 200, 8), PlayerEdit::Place);
        scene.set((8, 201, 8), Some(Block::new("minecraft:wheat")));
        server.player_edit(&scene, (8, 201, 8), PlayerEdit::Place);
        server.take_changes();
        assert!(server.bone_meal((8, 201, 8), "up"), "wheat takes bone meal");
        let changes = server.take_changes();
        let grown = changes
            .iter()
            .find(|(pos, _)| *pos == (8, 201, 8))
            .and_then(|(_, b)| b.clone())
            .expect("the wheat changed");
        let age: i32 = grown
            .properties
            .get("age")
            .and_then(|a| a.parse().ok())
            .unwrap_or(0);
        assert!((2..=5).contains(&age), "bone meal adds 2 to 5 ages: {age}");
        assert!(
            !server.bone_meal((8, 200, 8), "up"),
            "farmland does not take bone meal"
        );
    }
}

/// A saved mob's `CustomName`: plain text, or the text of a JSON text
/// component (`{"text":"..."}`, as older worlds keep it).
fn custom_name(tag: &minecraftoss_core::nbt::Tag) -> Option<String> {
    let minecraftoss_core::nbt::Tag::Compound(map) = tag else {
        return None;
    };
    let raw = match map.get("CustomName")? {
        minecraftoss_core::nbt::Tag::String(s) => s.clone(),
        minecraftoss_core::nbt::Tag::Compound(c) => match c.get("text")? {
            minecraftoss_core::nbt::Tag::String(s) => s.clone(),
            _ => return None,
        },
        _ => return None,
    };
    let text = if raw.starts_with('{') {
        let at = raw.find("\"text\"")?;
        let rest = &raw[at + 6..];
        let open = rest.find('"')? + 1;
        let close = rest[open..].find('"')? + open;
        rest[open..close].to_owned()
    } else {
        raw.trim_matches('"').to_owned()
    };
    (!text.is_empty()).then_some(text)
}

#[cfg(test)]
mod custom_name_tests {
    use super::custom_name;
    use minecraftoss_core::nbt::Tag;

    fn tag(name: Tag) -> Tag {
        let mut map = std::collections::BTreeMap::new();
        map.insert("CustomName".to_owned(), name);
        Tag::Compound(map)
    }

    #[test]
    fn plain_and_json_names_read() {
        assert_eq!(custom_name(&tag(Tag::String("Daxter".into()))).as_deref(), Some("Daxter"));
        assert_eq!(custom_name(&tag(Tag::String("{\"text\":\"Keira\"}".into()))).as_deref(), Some("Keira"));
        assert_eq!(custom_name(&tag(Tag::String(String::new()))), None);
        assert_eq!(custom_name(&Tag::Compound(Default::default())), None);
    }
}

#[cfg(test)]mod host_weather_regressions {
    use super::*;
    use minecraftoss_core::{registries::DataPaths,Registries,Chunk};
    use minecraftoss_generator::terrain::TerrainGenerator;
    fn server()->ServerSim {
        let paths=DataPaths::discover().expect("installed native data required for weather regression");
        let r=Arc::new(Registries::load(&paths).unwrap());
        let gen=Arc::new(minecraftoss_world::chunk_map::WorldGen::new(Arc::new(TerrainGenerator::overworld(r.clone(),0).unwrap())).unwrap());
        let states=Arc::new(BlockStates::new(r,0,-64,384).unwrap());ServerSim::new(gen,states,"minecraft:overworld")
    }
    #[test]fn host_weather_matches_visual_levels_without_drift_and_respects_dimension(){
        let mut s=server();let input=HostWeather{raining:true,thundering:true,rain_level:0.37,thunder_level:0.22};
        s.set_host_weather(Some(input));for _ in 0..20{s.tick();}
        let w=&s.level.sky.as_ref().unwrap().weather;assert_eq!((w.raining,w.thundering,w.rain_level,w.thunder_level),(true,true,0.37,0.22));
        for dim in["minecraft:the_nether","minecraft:the_end"]{s.level.set_dimension(dim).unwrap();s.set_host_weather(Some(input));s.tick();let w=&s.level.sky.as_ref().unwrap().weather;assert_eq!((w.raining,w.thundering,w.rain_level,w.thunder_level),(false,false,0.0,0.0));}
        s.level.set_dimension("minecraft:overworld").unwrap();s.set_host_weather(Some(HostWeather{rain_level:f32::NAN,thunder_level:2.0,..input}));let w=&s.level.sky.as_ref().unwrap().weather;assert_eq!((w.rain_level,w.thunder_level),(0.0,1.0));s.set_host_weather(None);assert!(!s.level.external_weather);
    }
    #[test]fn visible_rain_fills_exposed_cauldrons_through_existing_native_random_ticks(){
        let mut s=server();s.level.natural_spawning=None;
        let plains=s.level.registries().biomes.id("minecraft:plains").unwrap();let raw=Chunk::new(ChunkPos::new(0,0),-64,384,plains);s.load_chunk(&raw);s.set_simulation_area((0,0),0);
        let blocks=&s.level.registries().blocks;let tub=blocks.parse_state("minecraft:cauldron").unwrap();let roof=blocks.parse_state("minecraft:stone").unwrap();
        for x in 0..16{for z in 0..16{s.level.set_block_and_update(minecraftoss_core::BlockPos::new(x,150,z),tub);if x>=8{s.level.set_block_and_update(minecraftoss_core::BlockPos::new(x,153,z),roof);}}}
        s.set_host_weather(Some(HostWeather::default()));for _ in 0..1000{s.tick();}
        for x in 0..16{for z in 0..16{assert_eq!(s.level.block(minecraftoss_core::BlockPos::new(x,150,z)),tub);}}
        s.set_host_weather(Some(HostWeather{raining:true,rain_level:1.0,..Default::default()}));for _ in 0..16000{s.tick();}
        let mut filled=0;for x in 0..16{for z in 0..16{let state=s.level.block(minecraftoss_core::BlockPos::new(x,150,z));if x>=8{assert_eq!(state,tub,"the roof keeps native rain out");}else if state!=tub{filled+=1;let name=s.level.registries().blocks.block(s.level.registries().blocks.block_of(state)).name.as_str();assert_eq!(name,"minecraft:water_cauldron");}}}
        assert!(filled>0,"real native precipitation must change exposed cauldrons");
    }
}

#[cfg(test)]mod dripstone_regressions {
    use super::*;
    use minecraftoss_core::{registries::DataPaths,Registries,Chunk,BlockPos};
    use minecraftoss_generator::terrain::TerrainGenerator;
    use minecraftoss_world::level::dripstone::DripEffect;
    fn server()->ServerSim {
        let paths=DataPaths::discover().expect("installed native data required for dripstone regression");
        let r=Arc::new(Registries::load(&paths).unwrap());let plains=r.biomes.id("minecraft:plains").unwrap();
        let gen=Arc::new(WorldGen::new(Arc::new(TerrainGenerator::overworld(r.clone(),0).unwrap())).unwrap());
        let states=Arc::new(BlockStates::new(r,0,-64,384).unwrap());let mut s=ServerSim::new(gen,states,"minecraft:overworld");
        s.level.natural_spawning=None;s.load_chunk(&Chunk::new(ChunkPos::new(0,0),-64,384,plains));s.set_host_weather(Some(HostWeather::default()));s
    }
    fn put(s:&mut ServerSim,x:i32,y:i32,name:&str){let state=s.level.registries().blocks.parse_state(name).unwrap();s.level.set_block_and_update(BlockPos::new(x,y,8),state);}
    fn name(s:&ServerSim,x:i32,y:i32)->String {let b=&s.level.registries().blocks;b.block(b.block_of(s.level.block(BlockPos::new(x,y,8)))).name.to_string()}
    fn fixture(s:&mut ServerSim,x:i32,source:&str){
        put(s,x,149,"minecraft:stone");put(s,x,150,"minecraft:cauldron");put(s,x,153,"minecraft:dripstone_block");
        for (dx,dz) in [(-1,0),(1,0),(0,-1),(0,1)] {let stone=s.level.registries().blocks.parse_state("minecraft:stone").unwrap();s.level.set_block_and_update(BlockPos::new(x+dx,154,8+dz),stone);}
        put(s,x,154,source);put(s,x,152,"minecraft:pointed_dripstone[vertical_direction=down,thickness=tip,waterlogged=false]");
    }
    #[test]fn dripstone_delayed_fill_uses_source_fluid_and_rechecks_route(){
        let mut s=server();for(x,source)in[(2,"minecraft:water"),(4,"minecraft:lava"),(6,"minecraft:water"),(8,"minecraft:water"),(10,"minecraft:water[level=1]")]{fixture(&mut s,x,source);s.level.transfer_dripstone_fluid(BlockPos::new(x,152,8),0.0);}
        assert_eq!(s.level.drip_effects.iter().filter(|e|matches!(e,DripEffect::Drop{..})).count(),4,"flowing water must not transfer");
        put(&mut s,6,154,"minecraft:air");put(&mut s,8,151,"minecraft:stone");
        for _ in 0..51{s.tick();}assert_eq!(name(&s,2,150),"minecraft:cauldron");assert_eq!(name(&s,4,150),"minecraft:cauldron");
        s.tick();assert_eq!(name(&s,2,150),"minecraft:water_cauldron");assert_eq!(name(&s,4,150),"minecraft:lava_cauldron");
        for x in[6,8,10]{assert_eq!(name(&s,x,150),"minecraft:cauldron");}
        assert_eq!(s.level.drip_effects.iter().filter(|e|matches!(e,DripEffect::Fill{..})).count(),2);
        for expected in [2,3]{s.level.transfer_dripstone_fluid(BlockPos::new(2,152,8),0.0);for _ in 0..52{s.tick();}assert_eq!(s.level.registries().blocks.property(s.level.block(BlockPos::new(2,150,8)),"level"),Some(expected.to_string().as_str()));}
    }
    #[test]fn dripstone_random_ticks_reach_the_transfer_and_native_scheduler(){
        let mut s=server();s.set_simulation_area((0,0),0);
        for x in [2,4,6,8,10,12]{fixture(&mut s,x,"minecraft:water");}
        for _ in 0..20000{s.tick();}
        let filled=[2,4,6,8,10,12].into_iter().filter(|&x|name(&s,x,150)=="minecraft:water_cauldron").count();
        assert!(filled>0,"the actual random tick path, not a direct transfer call, must fill a vessel");
    }
    #[test]fn dripstone_obstruction_mud_evaporation_and_block_replacement(){
        let mut s=server();fixture(&mut s,2,"minecraft:mud");fixture(&mut s,4,"minecraft:water");fixture(&mut s,6,"minecraft:water");
        put(&mut s,4,151,"minecraft:oak_trapdoor[open=false]");
        s.level.transfer_dripstone_fluid(BlockPos::new(2,152,8),0.0);assert_eq!(name(&s,2,154),"minecraft:clay");assert!(s.level.drip_effects.iter().any(|e|matches!(e,DripEffect::DryMud{..})));
        s.level.transfer_dripstone_fluid(BlockPos::new(4,152,8),0.0);assert!(!s.level.drip_effects.iter().any(|e|matches!(e,DripEffect::Drop{tip,..}if tip.x==4)));
        s.level.transfer_dripstone_fluid(BlockPos::new(6,152,8),0.0);put(&mut s,6,150,"minecraft:powder_snow_cauldron[level=2]");for _ in 0..52{s.tick();}assert_eq!(name(&s,6,150),"minecraft:powder_snow_cauldron");
        s.level.set_dimension("minecraft:the_nether").unwrap();fixture(&mut s,10,"minecraft:mud");s.level.transfer_dripstone_fluid(BlockPos::new(10,152,8),0.0);assert_eq!(name(&s,10,154),"minecraft:mud","water evaporation comes from the dimension environment");
    }
}

#[cfg(test)]
mod world_owner_tests {
    use super::*;
    use minecraftoss_core::Registries;
    use minecraftoss_generator::terrain::TerrainGenerator;
    #[test]
    fn closing_simulation_releases_world_generation_and_its_caches() {
        let paths=minecraftoss_core::registries::DataPaths::discover().expect("installed native catalog");
        let registries=Arc::new(Registries::load(&paths).unwrap());
        let generator=Arc::new(TerrainGenerator::overworld(registries.clone(),42).unwrap());
        let world=Arc::new(WorldGen::new(generator).unwrap());
        let weak=Arc::downgrade(&world);
        let states=Arc::new(BlockStates::new(registries,42,-64,384).unwrap());
        let server=ServerSim::new(world.clone(),states,"minecraft:overworld");
        drop(world);
        assert!(weak.upgrade().is_some(),"live simulation retains its native generation context");
        drop(server);
        assert!(weak.upgrade().is_none(),"closing must not leak a world's full generation caches");
    }
}
