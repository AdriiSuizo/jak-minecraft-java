//! Area grades adapted from Jak Enhanced Trilogy (ISC).
//! Pinned source and notice: third_party/jak-enhanced-trilogy.
//! Visual only: never changes Minecraft light levels or mob spawning.
use crate::jak_sky::Mix;
/// Weather's Rainy includes plains and ocean. Only vegetation biomes
/// receive the forest look; the rest use the neutral wilds profile.
pub fn climate_of(biome: &str) -> crate::jak_sky::Climate {
    use crate::jak_sky::Climate;
    match crate::jak_sky::climate_of(biome) {
        Climate::Rainy
            if !(biome.contains("forest")
                || biome.contains("jungle")
                || biome.contains("taiga")) =>
        {
            Climate::None
        }
        other => other,
    }
}
pub type Grade = [[f32; 4]; 3];
pub const IDENTITY: Grade = [[0., 0., 0., 1.], [1., 1., 1., 0.], [0., 0., 0., 1.]];
const WILDS: [Grade; 2] = [
    [
        [0.0, 0.006, 0.01, 0.98],
        [1.01, 1.02, 0.98, 0.06],
        [0.0, 0.0, 0.0, 1.0],
    ],
    [
        [0.0, 0.006, 0.026, 0.95],
        [0.97, 0.99, 1.05, 0.06],
        [-0.03, 0.0, 0.0, 1.0],
    ],
];
const FOREST: [Grade; 2] = [
    [
        [0.0, 0.008, 0.004, 1.12],
        [1.07, 1.0, 0.86, 0.06],
        [0.03, 0.0, 0.0, 1.0],
    ],
    [
        [0.0, 0.006, 0.022, 1.0],
        [0.98, 1.0, 1.04, 0.05],
        [-0.02, 0.0, 0.0, 1.0],
    ],
];
const DESERT: [Grade; 2] = [
    [
        [0.004, 0.004, 0.008, 1.0],
        [1.02, 1.0, 0.97, 0.14],
        [0.0, 0.0, 0.0, 1.0],
    ],
    [
        [0.0, 0.004, 0.02, 0.95],
        [0.98, 0.98, 1.05, 0.04],
        [-0.03, 0.0, 0.0, 1.0],
    ],
];
const SWAMP: [Grade; 2] = [
    [
        [0.004, 0.008, 0.004, 0.88],
        [0.98, 1.0, 0.94, 0.04],
        [0.0, 0.0, 0.0, 1.0],
    ],
    [
        [0.0, 0.006, 0.02, 0.9],
        [0.96, 0.98, 1.02, 0.05],
        [-0.02, 0.0, 0.0, 1.0],
    ],
];
const SNOW: [Grade; 2] = [
    [
        [0.0, 0.006, 0.02, 0.95],
        [1.0, 1.02, 1.06, 0.05],
        [-0.04, 0.0, 0.0, 1.0],
    ],
    [
        [0.0, 0.006, 0.035, 0.92],
        [0.97, 0.99, 1.06, 0.06],
        [-0.05, 0.0, 0.0, 1.0],
    ],
];

fn blend(a: Grade, b: Grade, t: f32) -> Grade {
    std::array::from_fn(|r| std::array::from_fn(|c| a[r][c] + (b[r][c] - a[r][c]) * t))
}
pub fn target(mix: Mix, hour: f32) -> Grade {
    let hour = hour.rem_euclid(24.);
    let night = if hour < 12. {
        ((7. - hour) / 2.).clamp(0., 1.)
    } else {
        ((hour - 19.) / 2.).clamp(0., 1.)
    };
    let weights = [mix.rainy, mix.dry, mix.swamp, mix.cold]
        .map(|w| if w.is_finite() { w.clamp(0., 1.) } else { 0. });
    let total = weights.iter().sum::<f32>().max(1.);
    let base = blend(WILDS[0], WILDS[1], night);
    let mut out = base;
    for (zone, w) in [FOREST, DESERT, SWAMP, SNOW].iter().zip(weights) {
        let look = blend(zone[0], zone[1], night);
        for r in 0..3 {
            for c in 0..4 {
                out[r][c] += (look[r][c] - base[r][c]) * w / total;
            }
        }
    }
    out
}
/// Exact exponential easing, independent of render frequency. No image history.
pub fn update(previous: Grade, target: Grade, dt: f32) -> Grade {
    if previous[2][3] == 0. {
        return target;
    }
    let dt = if dt.is_finite() { dt.max(0.) } else { 0. };
    blend(previous, target, 1. - (-dt / 1.5).exp())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn enhanced36_profiles_and_midnight_are_continuous() {
        assert_eq!(
            target(
                Mix {
                    dry: 1.,
                    ..Default::default()
                },
                12.
            ),
            DESERT[0]
        );
        assert_eq!(
            target(
                Mix {
                    cold: 1.,
                    ..Default::default()
                },
                0.
            ),
            SNOW[1]
        );
        assert_eq!(target(Mix::default(), 0.), target(Mix::default(), 24.));
    }
    #[test]
    fn enhanced36_blends_are_frame_rate_independent() {
        let goal = target(
            Mix {
                dry: 1.,
                ..Default::default()
            },
            12.,
        );
        let mut fast = IDENTITY;
        let mut slow = IDENTITY;
        for _ in 0..150 {
            fast = update(fast, goal, 1. / 150.);
        }
        for _ in 0..30 {
            slow = update(slow, goal, 1. / 30.);
        }
        for r in 0..3 {
            for c in 0..4 {
                assert!((fast[r][c] - slow[r][c]).abs() < 0.00001);
            }
        }
        assert_eq!(update(IDENTITY, goal, 0.), IDENTITY);
    }
}
