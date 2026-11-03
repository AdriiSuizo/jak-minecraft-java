//! Jak 1's eco powers (Adrii, 22/10: from the vents of a base): the eco Jak
//! carries, how long it lasts and what it does, as `fact-info-target` keeps
//! them (`eco-type`, `eco-level`, `eco-timeout`, `eco-pickup-time`) and
//! `game-info.gc`'s `pickup-collectable!` sets them: each eco adds
//! `eco-single-timeout` (5 s) up to `eco-full-timeout` (20 s) counted from
//! the first; another colour starts afresh. `target-powerup-process` ends
//! it when its time is up. Blue eco makes him faster on foot and jumping
//! (`logic-target.gc` `build-conversions`), yellow eco turns his punch into
//! a shot ([`crate::eco_blast`]).
use crate::Jak;
use crate::math::seconds;

/// `*FACT-bank*`: what one eco adds, and the most there can be.
pub const SINGLE_TIMEOUT: i64 = seconds(5.0);
pub const FULL_TIMEOUT: i64 = seconds(20.0);
/// `eco-full-inc`: a vent's eco, a whole eco's worth.
pub const FULL_INC: f32 = 5.0;
/// `build-conversions`: what blue eco adds to his speed running and in a
/// jump (5 m/s).
pub const BLUE_SPEED: f32 = 20480.0;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EcoKind {
    Red,
    Yellow,
    Blue,
    Green,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct EcoPower {
    pub kind: EcoKind,
    /// 1 carrying it, 2 when its time is full (`eco-level`).
    pub level: f32,
    pub timeout: i64,
    pub pickup_time: i64,
}

impl EcoPower {
    /// Time left, Jak's ticks.
    pub fn left(&self, now: i64) -> i64 {
        (self.timeout - (now - self.pickup_time)).max(0)
    }
}

impl Jak {
    /// `pickup-collectable!` for red, blue and yellow eco (`amount` ecos).
    pub fn get_eco_power(&mut self, kind: EcoKind, amount: f32) {
        let now = self.time;
        let mut power = match self.eco_power {
            Some(p) if p.kind == kind => p,
            _ => EcoPower { kind, level: 0.0, timeout: 0, pickup_time: now },
        };
        let had = power.level;
        power.level = 1.0;
        if had == 0.0 {
            power.pickup_time = now;
        }
        power.timeout = (power.timeout + SINGLE_TIMEOUT * amount as i64)
            .min(FULL_TIMEOUT + (now - power.pickup_time));
        if power.timeout - (now - power.pickup_time) >= FULL_TIMEOUT {
            power.level = 2.0;
        }
        self.eco_power = Some(power);
    }

    /// The eco he carries and its time left as a share of the most there
    /// can be (the HUD's eco timer, `part-hud-eco-timer-0x-func`).
    pub fn eco_power_shown(&self) -> Option<(EcoKind, f32)> {
        let p = self.eco_power?;
        Some((p.kind, p.left(self.time) as f32 / FULL_TIMEOUT as f32))
    }

    /// He carries this eco now (`eco-level` at least 1).
    pub fn has_eco(&self, kind: EcoKind) -> bool {
        self.eco_power.is_some_and(|p| p.kind == kind && p.level >= 1.0)
    }

    /// `target-powerup-process`: its time up, the eco is gone.
    pub(crate) fn eco_power_frame(&mut self) {
        if let Some(p) = self.eco_power
            && self.time - p.pickup_time >= p.timeout
        {
            self.eco_power = None;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::glam::Vec3;

    #[test]
    fn a_vent_gives_the_full_twenty_seconds_and_another_colour_starts_afresh() {
        let mut jak = Jak::new(Vec3::ZERO, 0.0);
        jak.get_eco_power(EcoKind::Blue, FULL_INC);
        let p = jak.eco_power.unwrap();
        assert_eq!((p.level, p.timeout), (2.0, FULL_TIMEOUT));
        assert!(jak.has_eco(EcoKind::Blue));
        jak.time += seconds(8.0);
        // One small eco after 8 s: 5 s more on the 20 (never more than 20
        // past the first): 17 s left.
        jak.get_eco_power(EcoKind::Blue, 1.0);
        assert_eq!(jak.eco_power.unwrap().left(jak.time), seconds(17.0));
        jak.get_eco_power(EcoKind::Yellow, 1.0);
        let p = jak.eco_power.unwrap();
        assert_eq!((p.kind, p.level, p.left(jak.time)), (EcoKind::Yellow, 1.0, SINGLE_TIMEOUT));
        jak.time += SINGLE_TIMEOUT;
        jak.eco_power_frame();
        assert!(jak.eco_power.is_none());
    }
}
