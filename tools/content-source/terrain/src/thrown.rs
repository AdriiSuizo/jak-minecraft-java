//! Player thrown items use the same swept ThrowableProjectile simulation as
//! witch potions, on the integrated server's 20 Hz clock.
use glam::DVec3;
use minecraftoss_entities::effects::EffectInstance;
use minecraftoss_entities::potion::{Potion, ThrownPotion};
use minecraftoss_player::rng::LegacyRandom;

#[derive(Clone, Debug)]
pub struct Request {
    pub item: String,
    /// The item's components (a trident's enchantments and wear), kept for
    /// when it lands.
    pub components: Option<serde_json::Value>,
    /// How fast it leaves (blocks a tick), when not the item's own: a bow's
    /// draw times 3, a crossbow's 3.15.
    pub speed: Option<f32>,
    /// An arrow's base damage (`AbstractArrow.baseDamage`: 2, and Power's
    /// half a level and a half more) and whether it is a critical one (a
    /// full draw).
    pub base_damage: f32,
    pub critical: bool,
    pub effects: Vec<EffectInstance>,
    pub player: u64,
    pub eye: DVec3,
    pub direction: DVec3,
}

#[derive(Clone)]
pub(crate) struct Flying {
    pub request: Request,
    pub body: ThrownPotion,
    pub previous: DVec3,
}

#[derive(Clone, Debug)]
pub struct View {
    pub item: String,
    pub previous: DVec3,
    pub position: DVec3,
}

pub fn is_arrow(item: &str) -> bool {
    matches!(item, "minecraft:arrow" | "minecraft:spectral_arrow" | "minecraft:tipped_arrow")
}

/// Player arrows use AbstractArrow's own block collision, persistence,
/// despawn and release-on-block-removal rules, rather than ThrowableProjectile.
pub(crate) struct FlyingArrow {
    pub request: Request,
    pub body: minecraftoss_entities::projectile::Arrow,
    pub previous: DVec3,
    pub direction: DVec3,
    pub pickup_id: i32,
}
impl FlyingArrow {
    pub fn new(request: Request, random: &mut LegacyRandom, pickup_id: i32) -> Self {
        let mut body = minecraftoss_entities::projectile::Arrow::shoot(request.eye, request.direction, request.speed.unwrap_or(3.0), 1.0, random);
        body.base_damage = f64::from(request.base_damage);
        Self { previous: request.eye, direction: body.velocity, request, body, pickup_id }
    }
    pub fn view(&self) -> crate::arrow_render::ArrowView {
        crate::arrow_render::ArrowView { item: self.request.item.clone(), previous: self.previous, position: self.body.position, direction: self.direction, shake: self.body.shake_time as f32 }
    }
    pub fn can_pick_up(&self, feet: [f64; 3]) -> bool {
        if !self.body.alive || !self.body.in_ground || self.body.shake_time > 0 || self.request.components.as_ref().and_then(|c| c.get("mashup:no_pickup")).is_some() { return false; }
        let delta = self.body.position - DVec3::from_array(feet);
        // Player's .6×1.8 box inflated by (1,.5,1), intersecting the
        // arrow's .5×.5 box, as Player.aiStep/playerTouch.
        delta.x.abs() < 1.55 && delta.z.abs() < 1.55 && delta.y > -1.0 && delta.y < 2.3
    }
}

impl Flying {
    pub fn new(request: Request, random: &mut LegacyRandom) -> Self {
        let potion = request.item.ends_with("potion");
        let bottle = request.item == "minecraft:experience_bottle";
        let direction = if potion || bottle {
            // shootFromRotation's -20 degree pitch offset.
            let horizontal = request.direction.x.hypot(request.direction.z);
            let pitch = request.direction.y.atan2(horizontal) + 20_f64.to_radians();
            let flat = DVec3::new(request.direction.x, 0.0, request.direction.z).normalize_or(DVec3::Z);
            flat * pitch.cos() + DVec3::Y * pitch.sin()
        } else { request.direction };
        // `TridentItem.releaseUsing`: `shootFromRotation(.., 2.5, 1.0)`.
        let trident = request.item == TRIDENT;
        let speed = request.speed.unwrap_or(if potion { 0.5 } else if bottle { 0.7 } else if trident { 2.5 } else { 1.5 });
        let mut body = ThrownPotion::shoot(Potion::Healing, request.player, request.eye, direction, speed, 1.0, random);
        body.contents = Some(request.effects.clone());
        Self { previous: request.eye, request, body }
    }
    pub fn gravity(&self) -> f64 {
        // (`AbstractArrow.getDefaultGravity`: 0.05, a trident's too.)
        if self.request.item.ends_with("potion") || self.request.item == "minecraft:experience_bottle" || lands(&self.request.item) { 0.05 } else { 0.03 }
    }
    pub fn view(&self) -> View { View { item: self.request.item.clone(), previous: self.previous, position: self.body.position } }
}

/// Something outside the block world's own entities a thrown item can
/// strike (the end game's blazes and dragon, Jak's creatures, mounts and
/// vehicles): its shootable key and box in blocks, mins then maxs.
pub type ExternalTarget = (u64, [f64; 6]);

/// `PotionTarget` IDs for external targets: above every mob's and player's.
pub const EXTERNAL_TARGET: u64 = u64::MAX / 2 + (1 << 48);

/// A thrown item's blow on an entity (`onHitEntity`): the shootable key
/// (a native mob's own ID when `native`), the item, the damage in
/// Minecraft health before the target's own rules, and where it came from.
#[derive(Clone, Debug, PartialEq)]
pub struct ThrownHit {
    pub key: u64,
    pub native: bool,
    pub item: String,
    pub damage: f32,
    pub from: DVec3,
    pub player: u64,
}

/// What a thrown item's own `onHitEntity` does to what it strikes:
/// snowballs (`Snowball`: 3 to a blaze, else 0), eggs and ender pearls
/// (`hurt(thrown, 0)`). Bottles and potions only break (`onHit`).
pub fn strikes(item: &str) -> bool {
    matches!(item, "minecraft:snowball" | "minecraft:egg" | "minecraft:ender_pearl") || lands(item)
}

/// Shot or thrown things that come down to be picked up again (a trident,
/// arrows: `AbstractArrow`).
pub fn lands(item: &str) -> bool {
    item == TRIDENT || matches!(item, "minecraft:arrow" | "minecraft:spectral_arrow" | "minecraft:tipped_arrow")
}

/// An arrow's blow (`AbstractArrow.onHitEntity`): its speed times its base
/// damage, rounded up; a critical one up to half and two more (`roll`, 0..1).
pub fn arrow_damage(speed: f64, base: f32, critical: bool, roll: f32) -> f32 {
    let i = (speed * f64::from(base)).clamp(0.0, f64::from(i32::MAX)).ceil() as i64;
    let extra = if critical { (roll * (i / 2 + 2) as f32).floor() as i64 } else { 0 };
    (i + extra).min(i64::from(i32::MAX)) as f32
}

pub const TRIDENT: &str = "minecraft:trident";

/// `ThrownTrident.onHitEntity`'s 8 (Impaling's extra is for the aquatic).
pub const TRIDENT_DAMAGE: f32 = 8.0;

/// A thrown trident come down (on a block, or off what it struck): the
/// thrower, its components and where, to lie there or go back to them.
#[derive(Clone, Debug)]
pub struct Landed {
    pub player: u64,
    pub item: String,
    pub components: Option<serde_json::Value>,
    pub position: DVec3,
}

#[derive(Clone)]
pub(crate) struct Cloud {
    pub body: ThrownPotion,
    pub radius: f32,
    pub age: u32,
    pub used: std::collections::HashMap<u64, u32>,
}
#[derive(Clone, Debug)]
pub struct CloudView { pub position: DVec3, pub radius: f32, pub color: u32 }
impl Cloud {
    pub fn new(body: ThrownPotion) -> Self { Self { body, radius: 3.0, age: 0, used: Default::default() } }
    pub fn ready(&mut self, id: u64) -> bool {
        if self.age < 20 || self.age % 5 != 0 || self.used.get(&id).is_some_and(|age| self.age < *age + 20) { return false; }
        self.used.insert(id, self.age);
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    /// `AbstractArrow.onHitEntity`: a full bow draw (3 a tick, base 2)
    /// strikes for 6, a critical one up to 5 more.
    #[test]
    fn arrows_strike_by_their_speed() {
        assert_eq!(arrow_damage(3.0, 2.0, false, 0.9), 6.0);
        assert_eq!(arrow_damage(3.0, 2.0, true, 0.0), 6.0);
        assert_eq!(arrow_damage(3.0, 2.0, true, 0.99), 10.0);
        assert_eq!(arrow_damage(1.0, 2.0, false, 0.0), 2.0);
        assert!(lands("minecraft:arrow") && lands(TRIDENT) && !lands("minecraft:snowball"));
    }

    #[test]
    fn throw_power_and_cloud_reapplication_follow_the_item_rules() {
        let request = |item: &str| Request { item: item.into(), components: None, speed: None, base_damage: 2.0, critical: false, effects: Vec::new(), player: 0, eye: DVec3::Y * 2.0, direction: DVec3::Z };
        let mut random = LegacyRandom::new(3);
        let ball = Flying::new(request("minecraft:snowball"), &mut random);
        assert_eq!(ball.gravity(), 0.03);
        assert!((ball.body.velocity.length() - 1.5).abs() < 0.05);
        let potion = Flying::new(request("minecraft:splash_potion"), &mut random);
        assert_eq!(potion.gravity(), 0.05);
        assert!(potion.body.velocity.y > 0.15);
        let mut cloud = Cloud::new(potion.body);
        assert!(!cloud.ready(1)); cloud.age = 20; assert!(cloud.ready(1));
        cloud.age = 25; assert!(!cloud.ready(1)); assert!(cloud.ready(2));
        cloud.age = 40; assert!(cloud.ready(1));
        assert!(strikes("minecraft:snowball") && strikes("minecraft:egg") && !strikes("minecraft:splash_potion"));
        assert!(EXTERNAL_TARGET > minecraftoss_entities::world::PLAYER_TARGET + (1 << 40));
    }
}
