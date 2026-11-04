//! Jak's sky and weather over the block world (Adrii, 16/10).
//!
//! The sky is Jak 1's (`sky-tng.gc`, `time-of-day.gc`, `mood.gc`): its
//! hour is Minecraft's (tick 0 is 6:00), village1's eight sky textures
//! blended by `*default-interp-table*` as `update-mood-sky-texture` blends
//! them, its two cloud layers scrolling at `update-sky-tng-data`'s rates,
//! its sun and green sun on their orbits (`sky-make-sun-data`) at the hours
//! `time-of-day-update` shows them, its stars at night; its fog is
//! village1's. The Minecraft moon stays.
//!
//! The weather goes by biome: deserts and badlands never rain, they have
//! the Wasteland's sandstorms and Jak 3's desert sky (`*desert-mood-fog-
//! table*`, `*sandstorm-*-mood-fog-table*`, `*mood-sky-table*`); forests,
//! plains, jungles and the sea get Jak 3's rain, and its storms with
//! lightning (Jak 1's `update-mood-lightning` flashes); cold and snowy
//! biomes get Snowy Mountain's snow; swamps Boggy Billy's fog
//! (`*swamp-mood-fog-table*`) and its lightning, and rain. Rain comes and
//! goes as Minecraft's weather does (`ServerLevel.advanceWeatherCycle`), and
//! everything eases from biome to biome. This is the logic; the shader
//! draws the dome and the clouds, the particles are Jak 3's own.
use crate::jak_mood_tables::*;

/// The hour of the day (0 to 24, Jak's `time-of-day`) at Minecraft's
/// `day_ticks` (0 sunrise at 6:00, 6000 noon).
pub fn hour_of(day_ticks: f64) -> f32 {
    ((day_ticks / 1000.0 + 6.0).rem_euclid(24.0)) as f32
}

/// The two snapshots an hour blends and their weights
/// (`update-mood-sky-texture`, `update-mood-fog`).
pub fn snapshots(table: &[(usize, usize, f32, f32); 24], hour: f32) -> [(usize, f32); 2] {
    let whole = hour.floor();
    let f = hour - whole;
    let (s1, s2, start, end) = table[(whole as i32).rem_euclid(24) as usize];
    let morph = (1.0 - f) * start + f * end;
    if s1 == s2 { [(s1, 1.0), (s2, 0.0)] } else { [(s1, 1.0 - morph), (s2, morph)] }
}

fn lerp4(a: [f32; 4], b: [f32; 4], t: f32) -> [f32; 4] {
    std::array::from_fn(|i| a[i] + (b[i] - a[i]) * t)
}

/// A fog table at `hour` (`update-mood-fog`).
pub fn fog_at(table: &[Fog; 8], interp: &[(usize, usize, f32, f32); 24], hour: f32) -> Fog {
    let [(a, _), (b, t)] = snapshots(interp, hour);
    let (fa, fb) = (table[a], table[b]);
    Fog { color: lerp4(fa.color, fb.color, t), dists: lerp4(fa.dists, fb.dists, t), erase: lerp4(fa.erase, fb.erase, t) }
}

/// The sun's colour at `hour` (village1's `mood-sun-table`), 0-255.
pub fn sun_color(hour: f32) -> [f32; 3] {
    let [(a, _), (b, t)] = snapshots(&DEFAULT_INTERP, hour);
    let c = lerp4(VILLAGE1_SUN[a].0, VILLAGE1_SUN[b].0, t);
    [c[0], c[1], c[2]]
}

/// A body's orbit (`sky-set-orbit`): its high noon (hours), tilt and rise
/// (degrees), distance (meters).
#[derive(Clone, Copy, Debug)]
pub struct Orbit {
    pub high_noon: f32,
    pub tilt: f32,
    pub rise: f32,
    pub dist: f32,
}

/// `sky.gc`'s three: the sun, the green sun, the moon.
pub const SUN: Orbit = Orbit { high_noon: 12.5, tilt: -15.0, rise: 0.0, dist: 9950.0 };
pub const GREEN_SUN: Orbit = Orbit { high_noon: 4.0, tilt: 0.0, rise: 60.0, dist: 9950.0 };
pub const MOON: Orbit = Orbit { high_noon: 0.0, tilt: 0.0, rise: -10.0, dist: 9950.0 };

/// Where a body is at `hour`, as `sky-make-sun-data` puts it: a unit
/// direction in Jak's axes (y up).
pub fn orbit_dir(o: Orbit, hour: f32) -> [f32; 3] {
    // 2730.6667 rotation units an hour: 15 degrees.
    let a = (hour - o.high_noon) * 15f32.to_radians();
    let along = a.sin() * o.dist;
    let up = a.cos() * o.dist;
    let (tilt, rise) = (o.tilt.to_radians(), o.rise.to_radians());
    let y = up * tilt.cos();
    let side = up * tilt.sin();
    let z = -(along * rise.cos() + side * rise.sin());
    let x = side * rise.cos() - along * rise.sin();
    let l = (x * x + y * y + z * z).sqrt().max(1e-6);
    [x / l, y / l, z / l]
}

/// `time-of-day-update`'s windows: the sun from 6:15 to 18:45, the green
/// sun from 21:45 to 10:15, stars from 19:00 to 5:59.
pub fn sun_up(hour: f32) -> bool {
    (6.25..18.75).contains(&hour)
}

pub fn green_sun_up(hour: f32) -> bool {
    hour >= 21.75 || hour <= 10.25
}

pub fn stars_out(hour: f32) -> bool {
    let h = hour.floor() as i32;
    h >= 19 || h <= 5
}

/// `*default-mood*`'s and village1's stars.
pub const NUM_STARS: usize = 85;

/// The cloud layers' texture offsets after `frames` 60 Hz frames
/// (`update-sky-tng-data`: 16, 32 and -21, 42 sixty-five-thousandths a
/// frame), wrapped: s and t of layer 0, then of layer 1.
pub fn cloud_offsets(frames: f64) -> [f32; 4] {
    let at = |rate: f64| ((frames * rate / 65536.0).rem_euclid(1.0)) as f32;
    [at(16.0), at(32.0), at(-21.0), at(42.0)]
}

/// Jak 1's lightning flashes (`*flash0*` .. `*flash7*`): each value lasts
/// two frames.
pub const FLASHES: [&[f32]; 8] = [
    &[1.0, 0.0, 0.5, 1.0, 0.5, 0.0, 0.5, 0.35, 0.4, 0.35, 0.25, 0.1, 0.04],
    &[1.0, 0.8, 0.0, 1.0, 0.5, 1.0, 0.4, 0.2, 0.1],
    &[1.0, 0.9, 0.8, 0.7, 0.0, 0.0, 1.0, 0.0, 1.0, 0.5],
    &[0.5, 0.0, 1.0, 0.9, 1.0, 0.8, 0.3, 0.0, 0.0, 0.5, 0.1, 0.5, 0.35],
    &[1.0, 0.0, 1.0, 0.0, 1.0, 0.9, 0.8, 0.7, 0.6, 0.5, 0.4, 0.3, 0.2, 0.5, 0.4, 0.3, 0.2, 0.1],
    &[
        1.0, 0.0, 1.0, 0.0, 1.0, 0.95, 0.9, 0.85, 0.8, 0.75, 0.7, 0.65, 0.6, 0.55, 0.5, 0.45, 0.4, 0.35, 0.3, 0.25,
        0.2, 0.5, 0.45, 0.4, 0.35, 0.3, 0.25, 0.2, 0.15, 0.1, 0.05,
    ],
    &[1.0, 0.0, 1.0, 0.0, 0.5, 0.0, 0.5, 0.35, 0.0, 0.0, 1.0, 0.0, 0.2, 0.1],
    &[1.0, 0.8, 0.3, 0.0, 0.6, 0.5, 0.4, 0.3, 0.2, 0.5, 0.4, 0.3, 0.2, 0.1],
];

/// What a biome's weather is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Climate {
    /// Rain and storms (forests, plains, jungles, the sea, and the rest).
    Rainy,
    /// Snow (cold and snowy biomes, high peaks).
    Cold,
    /// Never rain: sandstorms (deserts, badlands).
    Dry,
    /// Fog, lightning and rain (swamps).
    Swamp,
    /// No weather (the Nether, the End, caves under the sky's reach).
    None,
}

/// A biome's climate by its name (`minecraft:...`).
pub fn climate_of(biome: &str) -> Climate {
    let name = biome.rsplit(':').next().unwrap_or(biome);
    if name.contains("swamp") {
        Climate::Swamp
    } else if name.contains("desert") || name.contains("badlands") {
        Climate::Dry
    } else if name.contains("snow")
        || name.contains("frozen")
        || name.contains("ice")
        || name.contains("grove")
        || name.contains("jagged")
        || name.contains("peaks")
        || name.contains("cold")
    {
        Climate::Cold
    } else if name.contains("nether")
        || name.contains("crimson")
        || name.contains("warped")
        || name.contains("basalt")
        || name.contains("soul_sand")
        || name.contains("the_end")
        || name.contains("end_")
        || name.contains("the_void")
        || name.contains("deep_dark")
        || name.contains("dripstone")
        || name.contains("lush_caves")
    {
        Climate::None
    } else {
        Climate::Rainy
    }
}

/// The biomes around the camera: what share of the samples has each
/// climate (they may not add up to 1: the rest has none).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Mix {
    pub rainy: f32,
    pub cold: f32,
    pub dry: f32,
    pub swamp: f32,
}

impl Mix {
    pub fn of(climates: &[Climate]) -> Self {
        let n = climates.len().max(1) as f32;
        let share = |c: Climate| climates.iter().filter(|&&k| k == c).count() as f32 / n;
        Self { rainy: share(Climate::Rainy), cold: share(Climate::Cold), dry: share(Climate::Dry), swamp: share(Climate::Swamp) }
    }
}

/// What the weather does this frame, for the renderer and the particles.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Atmosphere {
    /// The fog's colour (0-1) and how much of its distance is left (1 none
    /// taken).
    pub fog: [f32; 3],
    pub fog_scale: f32,
    /// Minecraft's rain and thunder levels (its own sky and light
    /// darkening).
    pub rain_level: f32,
    pub thunder_level: f32,
    /// A lightning flash (0 to 1).
    pub flash: f32,
    /// The sky drawn over towards this colour (rain's overcast) by so much.
    pub overcast: [f32; 4],
    /// Jak 3's desert sky (its zenith colour, 0-1) by so much.
    pub desert: [f32; 4],
}

impl Atmosphere {
    /// Weather shortens the visible landscape, including its distant mesh.
    /// The loaded chunk radius is not a visibility limit when LODs are present.
    pub fn fog_distances(&self, distances: [f32; 4]) -> [f32; 4] {
        let scale = self.fog_scale.clamp(0.05, 1.0);
        let end = (distances[1] * scale).max(12.0);
        let start = (distances[0] * scale).max(0.0).min(end * 0.72);
        [start, end, distances[2], distances[3]]
    }
}

#[cfg(test)]
mod distant_weather_tests {
    use super::*;
    #[test]
    fn clear_weather_keeps_far_hills_visible_but_storms_shorten_the_view() {
        let clear = Atmosphere { fog_scale: 0.88, ..Default::default() };
        let storm = Atmosphere { fog_scale: 0.10, ..Default::default() };
        let source = [512.0, 4096.0, 4032.0, 4096.0];
        let far = clear.fog_distances(source);
        assert!(far[0] > 256.0 && far[1] > 3500.0);
        let storm = storm.fog_distances(source);
        assert!(storm[1] < 420.0 && storm[1] > storm[0]);
        assert_eq!(far[2..], source[2..]);
        let near = clear.fog_distances([24.0, 192.0, 180.0, 192.0]);
        assert!(near[1] < 192.0);
    }
}

/// Minecraft's weather cycle and the weather at the camera, eased.
#[derive(Clone, Debug)]
pub struct Weather {
    pub raining: bool,
    pub thundering: bool,
    rain_time: i32,
    thunder_time: i32,
    /// The world's rain and thunder levels (0 to 1, a hundredth a tick).
    pub rain_level: f32,
    pub thunder_level: f32,
    /// At the camera, eased: rain, snow, sandstorm, swamp fog, the desert's
    /// sky, storm, cold (its fog), and the snow's ground.
    pub rain: f32,
    pub snow: f32,
    pub sand: f32,
    pub swamp: f32,
    pub desert: f32,
    pub storm: f32,
    pub cold: f32,
    ticks: f64,
    rng: u64,
    /// Lightning (`update-mood-lightning`): frames to the next, the flash
    /// playing and how far into it, and how many thunders so far.
    lightning_wait: i32,
    flash: Option<(usize, u32)>,
    pub thunders: u32,
}

/// Something the weather did this frame.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Event {
    /// Lightning: a thunder (0, 1 or 2: `THUNDER_A`..`C`) and its volume
    /// (0 to 1).
    Thunder { which: u32, volume: f32 },
}

impl Default for Weather {
    fn default() -> Self {
        Self::new(0x1234_5678)
    }
}

impl Weather {
    pub fn new(seed: u64) -> Self {
        let mut w = Self {
            raining: false,
            thundering: false,
            rain_time: 0,
            thunder_time: 0,
            rain_level: 0.0,
            thunder_level: 0.0,
            rain: 0.0,
            snow: 0.0,
            sand: 0.0,
            swamp: 0.0,
            desert: 0.0,
            storm: 0.0,
            cold: 0.0,
            ticks: 0.0,
            rng: seed | 1,
            lightning_wait: 240,
            flash: None,
            thunders: 0,
        };
        w.rain_time = w.range(12_000, 180_000);
        w.thunder_time = w.range(12_000, 180_000);
        w
    }

    fn random(&mut self) -> f32 {
        let mut x = self.rng;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.rng = x;
        ((x.wrapping_mul(0x2545_F491_4F6C_DD1D) >> 40) as f32) / (1u64 << 24) as f32
    }

    fn range(&mut self, low: i32, high: i32) -> i32 {
        low + (self.random() * (high - low) as f32) as i32
    }

    /// Set from the console: clear, rain or a storm.
    pub fn force(&mut self, rain: bool, thunder: bool) {
        self.raining = rain;
        self.thundering = thunder;
        self.rain_time = if rain { self.range(12_000, 24_000) } else { self.range(12_000, 180_000) };
        self.thunder_time = if thunder { self.range(3_600, 15_600) } else { self.range(12_000, 180_000) };
    }

    /// One Minecraft tick of `ServerLevel.advanceWeatherCycle` (no clear
    /// weather command): the thunder's and the rain's countdowns, and their
    /// levels a hundredth a tick towards on or off.
    fn tick_cycle(&mut self) {
        self.thunder_time -= 1;
        if self.thunder_time <= 0 {
            self.thundering = !self.thundering;
            self.thunder_time = if self.thundering { self.range(3_600, 15_600) } else { self.range(12_000, 180_000) };
        }
        self.rain_time -= 1;
        if self.rain_time <= 0 {
            self.raining = !self.raining;
            self.rain_time = if self.raining { self.range(12_000, 24_000) } else { self.range(12_000, 180_000) };
        }
        let toward = |level: f32, on: bool| (level + if on { 0.01 } else { -0.01 }).clamp(0.0, 1.0);
        self.rain_level = toward(self.rain_level, self.raining);
        self.thunder_level = toward(self.thunder_level, self.thundering);
    }

    /// `dt` seconds with the biomes `mix` round the camera: the cycle on,
    /// each weather eased towards what the biomes there have (a quarter a
    /// second: four seconds from one to another), and the lightning.
    pub fn step(&mut self, dt: f32, mix: Mix, events: &mut Vec<Event>) {
        let before = self.ticks;
        self.ticks += f64::from(dt) * 20.0;
        for _ in 0..(self.ticks.floor() - before.floor()).max(0.0) as i32 {
            self.tick_cycle();
        }
        let rain = self.rain_level;
        let thunder = self.thunder_level * rain;
        let targets = [
            // Rain: the wet biomes when it rains, swamps always a little.
            rain * (mix.rainy + mix.swamp) + 0.35 * mix.swamp * (1.0 - rain),
            // Snow where it is cold, when it comes down.
            rain * mix.cold,
            // Sandstorms in the deserts instead of rain.
            rain * mix.dry,
            mix.swamp,
            mix.dry,
            thunder * (mix.rainy + mix.swamp),
            mix.cold,
        ];
        let step = 0.25 * dt;
        let ease = |value: &mut f32, target: f32| *value += (target - *value).clamp(-step, step);
        ease(&mut self.rain, targets[0]);
        ease(&mut self.snow, targets[1]);
        ease(&mut self.sand, targets[2]);
        ease(&mut self.swamp, targets[3]);
        ease(&mut self.desert, targets[4]);
        ease(&mut self.storm, targets[5]);
        ease(&mut self.cold, targets[6]);
        // Lightning in storms and over swamps, a flash every 3 to 8 s
        // (`update-mood-lightning`), each value two frames.
        let frames = (dt * 60.0).round().max(1.0) as u32;
        let lit = self.storm > 0.3 || self.swamp > 0.5;
        if let Some((which, frame)) = self.flash.as_mut() {
            *frame += frames;
            if (*frame / 2) as usize >= FLASHES[*which].len() {
                self.flash = None;
            }
        } else if lit {
            self.lightning_wait -= frames as i32;
            if self.lightning_wait <= 0 {
                self.lightning_wait = self.range(180, 480);
                let which = (self.random() * 8.0) as usize % 8;
                self.flash = Some((which, 0));
                let volume = 0.5 + 0.5 * self.random();
                events.push(Event::Thunder { which: self.thunders % 3, volume });
                self.thunders += 1;
            }
        }
    }

    /// The lightning flash now (0 to 1).
    pub fn flash(&self) -> f32 {
        self.flash.map_or(0.0, |(which, frame)| FLASHES[which].get((frame / 2) as usize).copied().unwrap_or(0.0))
    }

    /// What it all does to the sky, the fog and the light at `hour`.
    pub fn atmosphere(&self, hour: f32) -> Atmosphere {
        let mut fog = fog_at(&VILLAGE1_FOG, &DEFAULT_INTERP, hour).color;
        let mut scale = 1.0;
        let mut toward = |table: &[Fog; 8], interp: &[(usize, usize, f32, f32); 24], amount: f32| {
            if amount > 0.0 {
                fog = lerp4(fog, fog_at(table, interp, hour).color, amount.min(1.0));
            }
        };
        // Snowy Mountain's fog where it is cold; Jak 3's desert where it is
        // dry, its sandstorm in a storm; the overcast and the storm's grey
        // in the rain; Boggy Billy's swamp.
        toward(&SNOW_FOG, &DEFAULT_INTERP, self.cold * 0.6);
        toward(&DESERT_FOG, &JAK3_INTERP, self.desert * 0.8);
        toward(&SANDSTORM_END_FOG, &JAK3_INTERP, self.sand);
        toward(&OVERCAST_RAIN_FOG, &JAK3_INTERP, self.rain * (1.0 - self.storm) * 0.8);
        toward(&STORM_RAIN_FOG, &JAK3_INTERP, self.storm);
        toward(&SWAMP_FOG, &DEFAULT_INTERP, self.swamp);
        // Each takes its share of the distance: Boggy Billy's fog ends at an
        // eighth of village1's, the sandstorm's at a third of the desert's.
        scale *= 1.0 - 0.75 * self.swamp;
        scale *= 1.0 - 0.86 * self.sand;
        scale *= 0.88 - 0.16*self.desert;
        scale *= 1.0 - 0.4 * self.rain;
        scale *= 1.0 - 0.25 * self.storm;
        scale *= 1.0 - 0.3 * self.snow;
        let flash = self.flash();
        let overcast_color = fog_at(&OVERCAST_RAIN_FOG, &JAK3_INTERP, hour).color;
        let storm_color = fog_at(&STORM_RAIN_FOG, &JAK3_INTERP, hour).color;
        let grey = lerp4(overcast_color, storm_color, self.storm);
        // A sandstorm hides the sky behind its sand.
        let sand = fog_at(&SANDSTORM_END_FOG, &JAK3_INTERP, hour).color;
        let wet = 0.75 * self.rain + 0.6 * self.snow + 0.5 * self.swamp;
        let dusty = 0.9 * self.sand;
        let grey = lerp4(grey, sand, if wet + dusty > 0.0 { dusty / (wet + dusty) } else { 0.0 });
        let [(a, _), (b, t)] = snapshots(&JAK3_INTERP, hour);
        let sky3 = lerp4(JAK3_SKY[a], JAK3_SKY[b], t);
        Atmosphere {
            fog: [fog[0] / 255.0, fog[1] / 255.0, fog[2] / 255.0],
            fog_scale: scale.max(0.05),
            rain_level: (self.rain + self.snow).min(1.0),
            thunder_level: self.storm,
            flash,
            overcast: [grey[0] / 255.0, grey[1] / 255.0, grey[2] / 255.0, (wet + dusty).min(0.9)],
            desert: [sky3[0] / 128.0, sky3[1] / 128.0, sky3[2] / 128.0, self.desert],
        }
    }
}

/// The sky shader's data (`TerrainView::jak_sky`): on, the two sky
/// textures and their weights, the lightning's sky texture and its weight,
/// the clouds' offsets, the overcast and the desert sky.
pub fn shader_data(hour: f32, frames: f64, atmosphere: &Atmosphere) -> [[f32; 4]; 8] {
    let [(a, wa), (b, wb)] = snapshots(&DEFAULT_INTERP, hour);
    let clouds = cloud_offsets(frames);
    [
        [1.0, 0.0, 0.0, 0.0],
        [a as f32, wa, b as f32, wb],
        // Lightning lights the sky as the noon sky (village1's flash
        // snapshot), by the flash.
        [2.0, atmosphere.flash, 0.0, 0.0],
        clouds,
        atmosphere.overcast,
        atmosphere.desert,
        [atmosphere.fog[0], atmosphere.fog[1], atmosphere.fog[2], 0.0],
        [0.0; 4],
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn minecraft_noon_is_jak_noon() {
        assert_eq!(hour_of(0.0), 6.0);
        assert_eq!(hour_of(6000.0), 12.0);
        assert_eq!(hour_of(18000.0), 0.0);
        assert!((hour_of(23999.0) - 5.999).abs() < 1e-3);
    }

    /// `*default-interp-table*` at noon: snapshot 2 to 3, a third of the
    /// way at 13:00; the weights always add to one.
    #[test]
    fn the_sky_blends_its_hours() {
        assert_eq!(snapshots(&DEFAULT_INTERP, 12.0), [(2, 1.0), (3, 0.0)]);
        let [(a, wa), (b, wb)] = snapshots(&DEFAULT_INTERP, 13.0);
        assert_eq!((a, b), (2, 3));
        assert!((wb - 0.33).abs() < 1e-5 && (wa + wb - 1.0).abs() < 1e-6);
        for k in 0..240 {
            let [(_, x), (_, y)] = snapshots(&DEFAULT_INTERP, k as f32 * 0.1);
            assert!((x + y - 1.0).abs() < 1e-5 || (x == 1.0 && y == 0.0));
        }
    }

    /// The sun high at its noon (12:30) and under the horizon at
    /// midnight; the green sun up at 4:00.
    #[test]
    fn the_suns_keep_their_orbits() {
        let noon = orbit_dir(SUN, 12.5);
        assert!(noon[1] > 0.95, "{noon:?}");
        assert!(orbit_dir(SUN, 0.5)[1] < -0.9);
        assert!(orbit_dir(GREEN_SUN, 4.0)[1] > 0.95);
        assert!(sun_up(12.0) && !sun_up(20.0) && green_sun_up(23.0) && !green_sun_up(15.0));
        assert!(stars_out(22.0) && stars_out(3.0) && !stars_out(12.0) && !stars_out(18.9));
    }

    #[test]
    fn biomes_have_their_weather() {
        for (b, c) in [
            ("minecraft:desert", Climate::Dry),
            ("minecraft:eroded_badlands", Climate::Dry),
            ("minecraft:snowy_plains", Climate::Cold),
            ("minecraft:frozen_peaks", Climate::Cold),
            ("minecraft:ice_spikes", Climate::Cold),
            ("minecraft:swamp", Climate::Swamp),
            ("minecraft:mangrove_swamp", Climate::Swamp),
            ("minecraft:forest", Climate::Rainy),
            ("minecraft:jungle", Climate::Rainy),
            ("minecraft:deep_ocean", Climate::Rainy),
            ("minecraft:plains", Climate::Rainy),
            ("minecraft:nether_wastes", Climate::None),
            ("minecraft:the_end", Climate::None),
        ] {
            assert_eq!(climate_of(b), c, "{b}");
        }
    }

    fn run(w: &mut Weather, seconds: f32, mix: Mix) -> Vec<Event> {
        let mut events = Vec::new();
        for _ in 0..(seconds * 60.0) as usize {
            w.step(1.0 / 60.0, mix, &mut events);
        }
        events
    }

    /// In the rain the desert has a sandstorm and no rain, the forest rain,
    /// the snowfields snow; walking from one to the other the weather eases
    /// over seconds.
    #[test]
    fn each_biome_has_its_own_weather_in_the_rain() {
        let forest = Mix { rainy: 1.0, ..Mix::default() };
        let desert = Mix { dry: 1.0, ..Mix::default() };
        let snow = Mix { cold: 1.0, ..Mix::default() };
        let mut w = Weather::new(3);
        w.force(true, false);
        run(&mut w, 10.0, desert);
        assert!(w.rain < 0.01 && w.sand > 0.99 && w.desert > 0.99, "{w:?}");
        run(&mut w, 1.0, forest);
        assert!(w.rain > 0.2 && w.rain < 0.3 && w.sand > 0.7, "half way: {w:?}");
        run(&mut w, 5.0, forest);
        assert!(w.rain > 0.99 && w.sand < 0.01);
        run(&mut w, 5.0, snow);
        assert!(w.snow > 0.99 && w.rain < 0.01);
        // Clear again: everything eases off (5 s for the level, 4 more).
        w.force(false, false);
        run(&mut w, 10.0, forest);
        assert!(w.rain < 0.01 && w.snow < 0.01);
    }

    /// Storms flash every 3 to 8 seconds with a thunder each; swamps
    /// flash even in clear weather (Boggy Billy's), and their fog closes in.
    #[test]
    fn storms_and_swamps_have_lightning() {
        let mut w = Weather::new(7);
        w.force(true, true);
        let events = run(&mut w, 60.0, Mix { rainy: 1.0, ..Mix::default() });
        let thunders = events.len();
        assert!((7..=20).contains(&thunders), "{thunders}");
        let mut clear = Weather::new(9);
        clear.force(false, false);
        assert!(run(&mut clear, 60.0, Mix { rainy: 1.0, ..Mix::default() }).is_empty());
        let swamp = run(&mut clear, 60.0, Mix { swamp: 1.0, ..Mix::default() });
        assert!(!swamp.is_empty());
        let a = clear.atmosphere(12.0);
        assert!(a.fog_scale < 0.3, "{a:?}");
        // Its fog is Boggy Billy's teal.
        assert!(a.fog[1] > a.fog[0] && a.fog[2] > a.fog[0]);
    }

    /// Minecraft's cycle: rain within its days, never for good.
    #[test]
    fn the_rain_comes_and_goes() {
        let mut w = Weather::new(11);
        let mut rained = false;
        let mut cleared = false;
        let mut events = Vec::new();
        // A few in-game days at a coarse step.
        for _ in 0..(20 * 60 * 60 * 3) {
            w.step(1.0 / 20.0 * 60.0 / 60.0, Mix { rainy: 1.0, ..Mix::default() }, &mut events);
            rained |= w.raining;
            cleared |= rained && !w.raining;
        }
        assert!(rained && cleared);
    }

    #[test]
    fn the_clouds_drift_at_their_rates() {
        let o = cloud_offsets(60.0);
        assert!((o[0] - 960.0 / 65536.0).abs() < 1e-6);
        assert!((o[1] - 1920.0 / 65536.0).abs() < 1e-6);
        assert!((o[2] - (1.0 - 1260.0 / 65536.0)).abs() < 1e-5);
    }
}
