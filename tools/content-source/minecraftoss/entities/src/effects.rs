//! Status effects on mobs (pinned 26.3 `MobEffectInstance`, `MobEffects`,
//! `HealOrHarmMobEffect`, `PoisonMobEffect`, `RegenerationMobEffect` and
//! `LivingEntity.addEffect`/`canBeAffected`/`tickEffects`): the effects the
//! potions of witches carry. An instance keeps a weaker but longer effect
//! hidden under a stronger one (`update`), and falls back to it when its
//! own time runs out. Each server tick counts every effect down after its
//! work for the tick (poison and regeneration on their intervals, an
//! instant effect once). Speed and slowness scale `MOVEMENT_SPEED`
//! (`ADD_MULTIPLIED_TOTAL`), weakness lowers `ATTACK_DAMAGE`.
//!
//! Vanilla keeps the effects in a `HashMap` keyed by identity-hashed
//! holders, so the order two effects tick in on the same tick varies from
//! run to run; here they tick in the order they were added.

/// A status effect a mob can carry: every effect `MobEffects` registers,
/// in its registration order.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum MobEffect {
    Speed,
    Slowness,
    Haste,
    MiningFatigue,
    Strength,
    InstantHealth,
    InstantDamage,
    JumpBoost,
    Nausea,
    Regeneration,
    Resistance,
    FireResistance,
    WaterBreathing,
    Invisibility,
    Blindness,
    NightVision,
    Hunger,
    Weakness,
    Poison,
    Wither,
    HealthBoost,
    Absorption,
    Saturation,
    Glowing,
    Levitation,
    Luck,
    Unluck,
    SlowFalling,
    ConduitPower,
    DolphinsGrace,
    BadOmen,
    HeroOfTheVillage,
    Darkness,
    TrialOmen,
    RaidOmen,
    WindCharged,
    Weaving,
    Oozing,
    Infested,
    BreathOfTheNautilus,
}

/// `MobEffects.register`: each effect, its ID and its colour (RGB).
const REGISTRY: [(MobEffect, &str, u32); 40] = [
    (MobEffect::Speed, "minecraft:speed", 3402751),
    (MobEffect::Slowness, "minecraft:slowness", 9154528),
    (MobEffect::Haste, "minecraft:haste", 14270531),
    (MobEffect::MiningFatigue, "minecraft:mining_fatigue", 4866583),
    (MobEffect::Strength, "minecraft:strength", 16762624),
    (MobEffect::InstantHealth, "minecraft:instant_health", 16262179),
    (MobEffect::InstantDamage, "minecraft:instant_damage", 11101546),
    (MobEffect::JumpBoost, "minecraft:jump_boost", 16646020),
    (MobEffect::Nausea, "minecraft:nausea", 5578058),
    (MobEffect::Regeneration, "minecraft:regeneration", 13458603),
    (MobEffect::Resistance, "minecraft:resistance", 9520880),
    (MobEffect::FireResistance, "minecraft:fire_resistance", 16750848),
    (MobEffect::WaterBreathing, "minecraft:water_breathing", 10017472),
    (MobEffect::Invisibility, "minecraft:invisibility", 16185078),
    (MobEffect::Blindness, "minecraft:blindness", 2039587),
    (MobEffect::NightVision, "minecraft:night_vision", 12779366),
    (MobEffect::Hunger, "minecraft:hunger", 5797459),
    (MobEffect::Weakness, "minecraft:weakness", 4738376),
    (MobEffect::Poison, "minecraft:poison", 8889187),
    (MobEffect::Wither, "minecraft:wither", 7561558),
    (MobEffect::HealthBoost, "minecraft:health_boost", 16284963),
    (MobEffect::Absorption, "minecraft:absorption", 2445989),
    (MobEffect::Saturation, "minecraft:saturation", 16262179),
    (MobEffect::Glowing, "minecraft:glowing", 9740385),
    (MobEffect::Levitation, "minecraft:levitation", 13565951),
    (MobEffect::Luck, "minecraft:luck", 5882118),
    (MobEffect::Unluck, "minecraft:unluck", 12624973),
    (MobEffect::SlowFalling, "minecraft:slow_falling", 15978425),
    (MobEffect::ConduitPower, "minecraft:conduit_power", 1950417),
    (MobEffect::DolphinsGrace, "minecraft:dolphins_grace", 8954814),
    (MobEffect::BadOmen, "minecraft:bad_omen", 745784),
    (MobEffect::HeroOfTheVillage, "minecraft:hero_of_the_village", 4521796),
    (MobEffect::Darkness, "minecraft:darkness", 2696993),
    (MobEffect::TrialOmen, "minecraft:trial_omen", 1484454),
    (MobEffect::RaidOmen, "minecraft:raid_omen", 14565464),
    (MobEffect::WindCharged, "minecraft:wind_charged", 12438015),
    (MobEffect::Weaving, "minecraft:weaving", 7891290),
    (MobEffect::Oozing, "minecraft:oozing", 10092451),
    (MobEffect::Infested, "minecraft:infested", 9214860),
    (MobEffect::BreathOfTheNautilus, "minecraft:breath_of_the_nautilus", 65518),
];

impl MobEffect {
    pub const ALL: [MobEffect; 40] = [
        Self::Speed,
        Self::Slowness,
        Self::Haste,
        Self::MiningFatigue,
        Self::Strength,
        Self::InstantHealth,
        Self::InstantDamage,
        Self::JumpBoost,
        Self::Nausea,
        Self::Regeneration,
        Self::Resistance,
        Self::FireResistance,
        Self::WaterBreathing,
        Self::Invisibility,
        Self::Blindness,
        Self::NightVision,
        Self::Hunger,
        Self::Weakness,
        Self::Poison,
        Self::Wither,
        Self::HealthBoost,
        Self::Absorption,
        Self::Saturation,
        Self::Glowing,
        Self::Levitation,
        Self::Luck,
        Self::Unluck,
        Self::SlowFalling,
        Self::ConduitPower,
        Self::DolphinsGrace,
        Self::BadOmen,
        Self::HeroOfTheVillage,
        Self::Darkness,
        Self::TrialOmen,
        Self::RaidOmen,
        Self::WindCharged,
        Self::Weaving,
        Self::Oozing,
        Self::Infested,
        Self::BreathOfTheNautilus,
    ];

    /// Its registry ID.
    pub fn id(self) -> &'static str {
        REGISTRY[self as usize].1
    }

    /// Its colour (`MobEffect.getColor`, the registration's).
    pub fn color(self) -> u32 {
        REGISTRY[self as usize].2
    }

    pub fn from_id(id: &str) -> Option<Self> {
        REGISTRY.iter().find(|e| e.1 == id).map(|e| e.0)
    }

    /// `MobEffect.isInstantaneous`: instant health and damage.
    pub fn instantaneous(self) -> bool {
        matches!(self, Self::InstantHealth | Self::InstantDamage)
    }

    /// `shouldApplyEffectTickThisTick` for the ticks left: an instant
    /// effect while any are left, poison every 25 ticks and regeneration
    /// every 50, halved a level (every tick once the shift runs out).
    fn applies_this_tick(self, ticks: i32, amplifier: i32) -> bool {
        let every = |interval: i32| {
            // Java's shift takes its distance modulo 32.
            let interval = interval.wrapping_shr(amplifier as u32);
            interval <= 0 || ticks % interval == 0
        };
        match self {
            Self::InstantHealth | Self::InstantDamage => ticks >= 1,
            Self::Poison => every(25),
            Self::Regeneration => every(50),
            _ => false,
        }
    }

    /// Its `MOVEMENT_SPEED` modifier's amount at level 1
    /// (`ADD_MULTIPLIED_TOTAL`, a float literal widened).
    fn speed_amount(self) -> Option<f64> {
        match self {
            Self::Speed => Some(f64::from(0.2_f32)),
            Self::Slowness => Some(f64::from(-0.15_f32)),
            _ => None,
        }
    }
}

/// `MobEffectInstance`: an effect's time left (-1 forever), level and
/// display flags, with the weaker effect hidden under it.
#[derive(Clone, Debug, PartialEq)]
pub struct EffectInstance {
    pub effect: MobEffect,
    pub duration: i32,
    pub amplifier: i32,
    pub ambient: bool,
    pub visible: bool,
    pub show_icon: bool,
    pub hidden: Option<Box<EffectInstance>>,
}

impl EffectInstance {
    /// `new MobEffectInstance(effect, duration, amplifier)`: not ambient,
    /// with particles and an icon.
    pub fn new(effect: MobEffect, duration: i32, amplifier: i32) -> Self {
        Self { effect, duration, amplifier: amplifier.clamp(0, 255), ambient: false, visible: true, show_icon: true, hidden: None }
    }

    /// `MobEffectInstance(copy)`: the details without the hidden effect.
    fn copy(&self) -> Self {
        Self { hidden: None, ..self.clone() }
    }

    pub fn infinite(&self) -> bool {
        self.duration == -1
    }

    fn has_remaining_duration(&self) -> bool {
        self.infinite() || self.duration > 0
    }

    fn shorter_than(&self, other: &Self) -> bool {
        !self.infinite() && (self.duration < other.duration || other.infinite())
    }

    /// `endsWithin`.
    pub fn ends_within(&self, ticks: i32) -> bool {
        !self.infinite() && self.duration <= ticks
    }

    /// `mapDuration`: infinite and spent durations stay as they are.
    pub fn map_duration(&self, f: impl FnOnce(i32) -> i32) -> i32 {
        if !self.infinite() && self.duration != 0 {
            f(self.duration)
        } else {
            self.duration
        }
    }

    /// `update(takeOver)`: a stronger effect takes over (hiding this one if
    /// it would outlast it), a longer one of the same level extends it, and
    /// a longer weaker one hides beneath. Returns whether it changed.
    fn update(&mut self, take_over: &Self) -> bool {
        let mut changed = false;
        if take_over.amplifier > self.amplifier {
            if take_over.shorter_than(self) {
                let previous = self.hidden.take();
                let mut hidden = self.copy();
                hidden.hidden = previous;
                self.hidden = Some(Box::new(hidden));
            }
            self.amplifier = take_over.amplifier;
            self.duration = take_over.duration;
            changed = true;
        } else if self.shorter_than(take_over) {
            if take_over.amplifier == self.amplifier {
                self.duration = take_over.duration;
                changed = true;
            } else if let Some(hidden) = self.hidden.as_mut() {
                hidden.update(take_over);
            } else {
                self.hidden = Some(Box::new(take_over.copy()));
            }
        }
        if (!take_over.ambient && self.ambient) || changed {
            self.ambient = take_over.ambient;
            changed = true;
        }
        if take_over.visible != self.visible {
            self.visible = take_over.visible;
            changed = true;
        }
        if take_over.show_icon != self.show_icon {
            self.show_icon = take_over.show_icon;
            changed = true;
        }
        changed
    }

    fn tick_down(&mut self) {
        if let Some(hidden) = self.hidden.as_mut() {
            hidden.tick_down();
        }
        self.duration = self.map_duration(|d| d - 1);
    }

    /// `downgradeToHiddenEffect`: spent, the hidden effect comes back.
    fn downgrade_to_hidden(&mut self) -> bool {
        if self.duration != 0 {
            return false;
        }
        let Some(hidden) = self.hidden.take() else { return false };
        let hidden = *hidden;
        self.duration = hidden.duration;
        self.amplifier = hidden.amplifier;
        self.ambient = hidden.ambient;
        self.visible = hidden.visible;
        self.show_icon = hidden.show_icon;
        self.hidden = hidden.hidden;
        true
    }
}

/// What an effect does to its mob on a tick it works
/// (`applyEffectTick`), for the mob to carry out.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum EffectWork {
    /// `heal`: only a living mob heals.
    Heal(f32),
    /// `hurtServer` with `magic`.
    HurtMagic(f32),
    /// Poison: 1 magic damage while more than 1 health is left.
    Poison,
    /// Regeneration: 1 health below full.
    Regenerate,
}

impl EffectWork {
    /// What it comes to for a mob at `health` of `max_health` (poison and
    /// regeneration look at the health when they work).
    pub fn resolve(self, health: f32, max_health: f32) -> Option<EffectWork> {
        match self {
            Self::Poison => (health > 1.0).then_some(Self::HurtMagic(1.0)),
            Self::Regenerate => (health < max_health).then_some(Self::Heal(1.0)),
            other => Some(other),
        }
    }
}

/// A mob's active effects.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct MobEffects {
    active: Vec<EffectInstance>,
}

impl MobEffects {
    /// 26.3 LivingEntity.getVisibilityPercent: count occupied armor slots, not armor points.
    pub fn visibility_percent(&self, armor_items:usize)->f64 {
        if self.has(MobEffect::Invisibility) {0.7*(armor_items.min(4) as f64/4.0).max(0.1)}else{1.0}
    }

    pub fn is_empty(&self) -> bool {
        self.active.is_empty()
    }

    pub fn iter(&self) -> impl Iterator<Item = &EffectInstance> {
        self.active.iter()
    }

    /// `hasEffect`.
    pub fn has(&self, effect: MobEffect) -> bool {
        self.active.iter().any(|e| e.effect == effect)
    }

    pub fn get(&self, effect: MobEffect) -> Option<&EffectInstance> {
        self.active.iter().find(|e| e.effect == effect)
    }

    /// `LivingEntity.addEffect` once `canBeAffected` has agreed: a new
    /// effect, or an update of the one it has. Returns whether it changed.
    pub fn add(&mut self, instance: EffectInstance) -> bool {
        match self.active.iter_mut().find(|e| e.effect == instance.effect) {
            None => {
                self.active.push(instance);
                true
            }
            Some(effect) => effect.update(&instance),
        }
    }

    /// Puts an effect back as saved (`readAdditionalSaveData`).
    pub fn insert(&mut self, instance: EffectInstance) {
        self.active.retain(|e| e.effect != instance.effect);
        self.active.push(instance);
    }

    /// The server half of `LivingEntity.tickEffects`: each effect works if
    /// its time has come (`MobEffectInstance.tickServer`), counts down,
    /// takes back its hidden effect when spent and goes when nothing is
    /// left. Returns the work due, in order, for the mob to carry out (none
    /// of these effects stops on its work, and none looks at the others').
    /// `inverted` is `isInvertedHealAndHarm` (the undead).
    pub fn tick(&mut self, inverted: bool) -> Vec<EffectWork> {
        let mut work = Vec::new();
        self.active.retain_mut(|effect| {
            if !effect.has_remaining_duration() {
                return false;
            }
            // Infinite effects tick by the mob's age; none here are.
            if effect.effect.applies_this_tick(effect.duration, effect.amplifier) {
                work.extend(effect_work(effect.effect, effect.amplifier, inverted));
            }
            effect.tick_down();
            effect.downgrade_to_hidden();
            effect.has_remaining_duration()
        });
        work
    }

    /// `MOVEMENT_SPEED` from its value before the effects' modifiers:
    /// speed's and slowness's `ADD_MULTIPLIED_TOTAL` in the order the
    /// attribute's hash map keeps them (`effect.speed`'s slot comes
    /// before `effect.slowness`'s), clamped to the attribute's range.
    pub fn movement_speed(&self, value: f64) -> f64 {
        let mut result = value;
        for effect in [MobEffect::Speed, MobEffect::Slowness] {
            if let (Some(instance), Some(amount)) = (self.get(effect), effect.speed_amount()) {
                result *= 1.0 + amount * f64::from(instance.amplifier + 1);
            }
        }
        result.clamp(0.0, 1024.0)
    }

    /// `ATTACK_DAMAGE`'s `ADD_VALUE` modifiers: strength's +3 a level and
    /// weakness's -4 a level (`MobEffects`' registrations).
    pub fn luck_modifier(&self)->f32 {
        let level=|kind|self.get(kind).map_or(0.0,|e|(e.amplifier+1) as f32);
        (level(MobEffect::Luck)-level(MobEffect::Unluck)).clamp(-1024.0,1024.0)
    }

    pub fn attack_damage_modifier(&self) -> f64 {
        let level = |effect| self.get(effect).map_or(0.0, |e: &EffectInstance| f64::from(e.amplifier + 1));
        3.0 * level(MobEffect::Strength) - 4.0 * level(MobEffect::Weakness)
    }

    /// `LivingEntity.getDamageAfterMagicAbsorb`'s resistance: 5 of 25
    /// parts less a level, never below nothing. Callers leave out the
    /// damage `BYPASSES_EFFECTS` / `BYPASSES_RESISTANCE` (starving, `/kill`).
    pub fn after_resistance(&self, damage: f32) -> f32 {
        match self.get(MobEffect::Resistance) {
            Some(e) => (damage * (25 - (e.amplifier + 1) * 5) as f32 / 25.0).max(0.0),
            None => damage,
        }
    }

    /// `LivingEntity.getJumpBoostPower`: 0.1 a level of jump boost.
    pub fn jump_boost_power(&self) -> f32 {
        self.get(MobEffect::JumpBoost).map_or(0.0, |e| 0.1 * (e.amplifier as f32 + 1.0))
    }

    /// `SAFE_FALL_DISTANCE`'s `ADD_VALUE` from jump boost: 1 a level.
    pub fn safe_fall_bonus(&self) -> f64 {
        self.get(MobEffect::JumpBoost).map_or(0.0, |e| f64::from(e.amplifier + 1))
    }
}

/// `applyEffectTick` for one effect: instant health heals 4 a level
/// (doubling) and instant damage hurts 6 (the other way round for the
/// undead), poison hurts 1 while more than 1 health is left, regeneration
/// heals 1 below full health ([`EffectWork::resolve`] checks the health).
fn effect_work(effect: MobEffect, amplifier: i32, inverted: bool) -> Option<EffectWork> {
    let heal = || EffectWork::Heal(4_i32.wrapping_shl(amplifier as u32).max(0) as f32);
    let harm = || EffectWork::HurtMagic(6_i32.wrapping_shl(amplifier as u32) as f32);
    match effect {
        MobEffect::InstantHealth => Some(if inverted { harm() } else { heal() }),
        MobEffect::InstantDamage => Some(if inverted { heal() } else { harm() }),
        MobEffect::Poison => Some(EffectWork::Poison),
        MobEffect::Regeneration => Some(EffectWork::Regenerate),
        _ => None,
    }
}

/// `applyInstantaneousEffect` from a splash at `scale` (1 at the heart):
/// the heal or the harm, rounded (`(int)(scale * n + 0.5)`).
pub fn instant_work(effect: MobEffect, amplifier: i32, inverted: bool, scale: f64) -> Option<EffectWork> {
    let heal_amount = || (scale * f64::from(4_i32.wrapping_shl(amplifier as u32)) + 0.5) as i32 as f32;
    let harm_amount = || (scale * f64::from(6_i32.wrapping_shl(amplifier as u32)) + 0.5) as i32 as f32;
    let harm = matches!(effect, MobEffect::InstantDamage);
    if !effect.instantaneous() {
        return None;
    }
    Some(if harm == inverted { EffectWork::Heal(heal_amount()) } else { EffectWork::HurtMagic(harm_amount()) })
}

/// The effects a mob of this kind cannot take (`canBeAffected`): the
/// undead ignore poison and regeneration (`#ignores_poison_and_regen`),
/// spiders poison (`Spider.canBeAffected`).
pub fn can_be_affected(effect: MobEffect, undead: bool, spider: bool) -> bool {
    match effect {
        MobEffect::Poison => !undead && !spider,
        MobEffect::Regeneration => !undead,
        _ => true,
    }
}

/// `LivingEntity.heal`: only the living heal, up to their maximum.
pub fn heal(health: &mut f32, max_health: f32, amount: f32) {
    if *health > 0.0 {
        *health = (*health + amount).clamp(0.0, max_health);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_stronger_shorter_effect_hides_the_longer_one_until_it_runs_out() {
        let mut effects = MobEffects::default();
        effects.add(EffectInstance::new(MobEffect::Speed, 100, 0));
        assert!(effects.add(EffectInstance::new(MobEffect::Speed, 3, 1)));
        let speed = effects.get(MobEffect::Speed).unwrap();
        assert_eq!((speed.amplifier, speed.duration), (1, 3));
        assert_eq!(speed.hidden.as_ref().unwrap().duration, 100);
        for _ in 0..3 {
            effects.tick(false);
        }
        let speed = effects.get(MobEffect::Speed).unwrap();
        assert_eq!((speed.amplifier, speed.duration), (0, 97));
        assert!(speed.hidden.is_none());
    }

    #[test]
    fn poison_hurts_every_25_ticks_and_instant_health_once() {
        let mut effects = MobEffects::default();
        effects.add(EffectInstance::new(MobEffect::Poison, 50, 0));
        effects.add(EffectInstance::new(MobEffect::InstantHealth, 1, 0));
        assert_eq!(effects.tick(false), vec![EffectWork::Poison, EffectWork::Heal(4.0)]);
        assert_eq!(EffectWork::Poison.resolve(1.0, 20.0), None);
        assert!(!effects.has(MobEffect::InstantHealth));
        let hurts = (0..49).map(|_| effects.tick(false).len()).sum::<usize>();
        assert_eq!(hurts, 1);
        assert!(!effects.has(MobEffect::Poison));
    }

    #[test]
    fn undead_are_healed_by_harming_and_ignore_poison() {
        assert_eq!(instant_work(MobEffect::InstantDamage, 0, true, 0.75), Some(EffectWork::Heal(3.0)));
        assert_eq!(instant_work(MobEffect::InstantDamage, 0, false, 0.75), Some(EffectWork::HurtMagic(5.0)));
        assert!(!can_be_affected(MobEffect::Poison, true, false));
        assert!(!can_be_affected(MobEffect::Poison, false, true));
    }

    #[test]
    fn speed_and_slowness_scale_movement_speed() {
        let mut effects = MobEffects::default();
        effects.add(EffectInstance::new(MobEffect::Speed, 100, 0));
        assert_eq!(effects.movement_speed(0.25), 0.25 * (1.0 + f64::from(0.2_f32)));
        effects.add(EffectInstance::new(MobEffect::Slowness, 100, 1));
        assert_eq!(effects.movement_speed(0.25), 0.25 * (1.0 + f64::from(0.2_f32)) * (1.0 + f64::from(-0.15_f32) * 2.0));
    }
}

#[cfg(test)]mod invisibility_regressions {
 use super::*;
 #[test]fn native_visibility_depends_on_occupied_armor_and_expires(){
  let mut fx=MobEffects::default();assert_eq!(fx.visibility_percent(4),1.0);
  fx.add(EffectInstance::new(MobEffect::Invisibility,2,0));
  for (slots,expected) in [(0,0.07),(1,0.175),(2,0.35),(3,0.525),(4,0.7),(9,0.7)] {
   assert!((fx.visibility_percent(slots)-expected).abs()<1e-12);
  }
  fx.tick(false);assert!((fx.visibility_percent(0)-0.07).abs()<1e-12);
  fx.tick(false);assert_eq!(fx.visibility_percent(0),1.0);
 }
}

#[cfg(test)]mod luck_attribute_regressions {
    use super::*;
    #[test]fn luck_and_unluck_add_their_native_attribute_modifiers(){let mut e=MobEffects::default();e.add(EffectInstance::new(MobEffect::Luck,200,2));e.add(EffectInstance::new(MobEffect::Unluck,100,0));assert_eq!(e.luck_modifier(),2.0);for _ in 0..100{e.tick(false);}assert_eq!(e.luck_modifier(),3.0);}
}
