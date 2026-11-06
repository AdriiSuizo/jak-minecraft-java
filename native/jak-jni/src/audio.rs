//! Original action/animation/loop selectors; Minecraft owns playback.
use std::collections::HashMap;
use glam::Vec3;
use jak_core::{Event, GunKind, Jak, State, Trick};
pub struct JakSoundRequest {
    pub name: &'static str,
    pub position: Vec3,
    pub gain: f32,
    pub pitch: f32,
}

/// The sounds' queue, shared by both local players; what each one's Jak
/// last did (to cue only changes) and the loops each keeps going are their
/// own, as the two take turns frame by frame.
#[derive(Default)]
pub struct JakSoundQueue {
    requests: Vec<JakSoundRequest>,
    last_cue: HashMap<(&'static str, usize), i64>,
    players: [PlayerSounds; 2],
    /// Whose Jak the calls now are about.
    player: usize,
    /// Frames played, to drop the loops of a player no longer here.
    tick: u64,
}

#[derive(Default)]
struct PlayerSounds {
    loops: HashMap<&'static str, (Vec3, f32)>,
    pitches: HashMap<&'static str,f32>,
    /// The frame its loops were last set on.
    seen: u64,
    last_anim: Option<jak_core::anim::Anim>,
    steps: crate::jak_steps::StepClock,
    dark: bool,
    dark_transition: bool,
    freezing: bool,
    healing: bool,
    gun: bool,
}

impl JakSoundQueue {
    pub fn pending(&self) -> &[JakSoundRequest] {
        &self.requests
    }

    pub fn loop_active(&self, name: &str) -> bool {
        self.players.iter().any(|p| p.loops.contains_key(name))
    }

    fn loops(&mut self) -> &mut HashMap<&'static str, (Vec3, f32)> {
        &mut self.players[self.player].loops
    }

    /// Every player's loops still kept going (a player whose frames stopped
    /// coming has none), one of each name.
    fn live_loops(&self) -> HashMap<&'static str, (Vec3, f32)> {
        let mut out = HashMap::new();
        for p in &self.players {
            if self.tick.saturating_sub(p.seen) <= 3 {
                for (&name, &v) in &p.loops {
                    out.entry(name).or_insert(v);
                }
            }
        }
        out
    }

    pub fn play_at(&mut self, name: &'static str, position: Vec3) {
        self.push(name, position, 1.0, 1.0);
    }

    /// The HellCat's loops: the engine always, the boost with the
    /// throttle, the wind as it banks, and the damage alarms as the hull
    /// fails (`damage-low` under half, `-med` under 0.2, `-high` under 0.15).
    pub fn vehicle_engine(&mut self, position: Vec3, boost: f32, wind: f32, health: f32) {
        self.loops().insert("HELLCAT_ENGINE", (position, 1.0));
        if boost > 0.01 {
            self.loops()
                .insert("HELLCAT_BOOST", (position, boost.clamp(0.0, 1.0)));
        }
        if wind > 0.01 {
            self.loops().insert("HELLCAT_BANK", (position, wind.clamp(0.0, 1.0)));
        }
        let alarm = if health < 0.15 {
            Some("DAMAGE_HIGH")
        } else if health < 0.2 {
            Some("DAMAGE_MED")
        } else if health < 0.5 {
            Some("DAMAGE_LOW")
        } else {
            None
        };
        if let Some(alarm) = alarm {
            self.loops().insert(alarm, (position, 0.8));
        }
    }

    /// A loop kept going this frame (another world's: a zoomer's engine).
    pub fn loop_at(&mut self, name: &'static str, position: Vec3, gain: f32) {
        self.loops().insert(name, (position, gain.clamp(0.0, 1.0)));
    }

    pub fn loop_at_pitch(&mut self,name:&'static str,position:Vec3,gain:f32,pitch:f32){
        self.loop_at(name,position,gain);
        self.players[self.player].pitches.insert(name,if pitch.is_finite(){pitch.clamp(0.25,4.)}else{1.});
    }
    fn live_pitch(&self,name:&str)->f32 {
        self.players.iter().find(|p|self.tick.saturating_sub(p.seen)<=3&&p.loops.contains_key(name))
            .and_then(|p|p.pitches.get(name).copied()).unwrap_or(1.)
    }
    fn push(&mut self, name: &'static str, position: Vec3, gain: f32, pitch: f32) {
        if self.requests.len() < 128 {
            self.requests.push(JakSoundRequest {
                name,
                position,
                gain,
                pitch,
            });
        }
    }

    fn cue(&mut self, name: &'static str, position: Vec3, tick: i64, gain: f32, pitch: f32) {
        let cooldown = jak_core::math::seconds(0.07);
        let key = (name, self.player);
        if self
            .last_cue
            .get(&key)
            .is_some_and(|last| tick >= *last && tick - *last < cooldown)
        {
            return;
        }
        self.last_cue.insert(key, tick);
        self.push(name, position, gain, pitch);
    }
}

fn map_position(p: jak_core::glam::Vec3) -> Vec3 {
    p
}

/// Player one's Jak's sounds, on unknown ground.
pub fn emit_jak_sounds(jak: &Jak, events: &[Event], queue: &mut JakSoundQueue) {
    emit_jak_sounds_for(jak, 0, Some(crate::jak_steps::Floor::Unknown), events, queue);
}

/// Player `player`'s (0 or 1) Jak's sounds this frame: his events', his
/// animation's (its footsteps and landings on `floor`; none while `floor`
/// is `None`, riding something) and his loops'.
pub fn emit_jak_sounds_for(
    jak: &Jak,
    player: usize,
    floor: Option<crate::jak_steps::Floor>,
    events: &[Event],
    queue: &mut JakSoundQueue,
) {
    let player = player.min(1);
    queue.player = player;
    queue.players[player].seen = queue.tick;
    let at = map_position(jak.trans());
    let ground = floor.unwrap_or_default().sounds();
    for event in events {
        let (name, position, gain, pitch) = match *event {
            Event::BoardOn | Event::BoardOff => ("BOARD_BOOTS", at, 1.0, 1.0),
            Event::BoardJump => ("BOARD_JUMP", at, 1.0, 1.0),
            Event::BoardLaunch => ("BOARD_LAUNCH", at, 1.0, 1.0),
            Event::BoardFlip => ("BOARD_FLIP", at, 1.0, 1.0),
            Event::BoardBoost => ("BOARD_BOOST", at, 1.0, 1.0),
            Event::BoardGlance => ("BOARD_GLANCE", at, 1.0, 1.0),
            Event::BoardBounce { pitch } => ("BOARD_BOUNCE", at, 1.0, 2.0f32.powf(pitch)),
            Event::BoardZap { center, .. } => ("BOARD_ZAP", map_position(center), 1.0, 1.0),
            Event::Trick { trick, .. } => {
                let name = match trick {
                    Trick::Nosegrab
                    | Trick::Method
                    | Trick::MethodCross
                    | Trick::Backgrab
                    | Trick::Airwalk => "BOARD_NOSEGRAB",
                    Trick::Noseflip => "BOARD_NOSEFLIP",
                    Trick::Kickspin => "BOARD_KICK",
                    Trick::Kickflip => "BOARD_KICKFLIP",
                    Trick::BoardSpin => "BOARD_BOARDSPIN",
                    Trick::DuckJump => "BOARD_K_JUMP",
                    Trick::QuickJump => "BOARD_Q_JUMP",
                    Trick::Spin | Trick::Boost | Trick::Flip | Trick::Jump => continue,
                };
                (name, at, 1.0, 1.0)
            }
            // On the Flut Flut its own (`target-flut-jump`: Jak 1's, a
            // half octave down).
            Event::Jump if jak.state.is_flut() => ("jak1:jump", at, 0.8, 2f32.powf(-0.5)),
            Event::Jump => (
                if matches!(jak.state, State::DoubleJump { .. }) {
                    "JUMP_DOUBLE"
                } else {
                    "JUMP"
                },
                at,
                0.8,
                1.0,
            ),
            // `target-land-effect`: the board's own, the Flut Flut's on the
            // ground (`effect-flut-land`, Jak 1's), Jak's on it
            // (`effect-land`, at full volume).
            Event::Land => (
                if jak.state.is_board() {
                    "BOARD_IMPACT"
                } else if jak.state.is_flut() {
                    ground.flut_land
                } else {
                    ground.land
                },
                at,
                1.0,
                1.0,
            ),
            Event::HardLanding { .. } => ("LAND_HARD", at, 1.0, 1.0),
            Event::Fire { from, .. } => ("YELLOW_SHOT_FIR", map_position(from), 1.0, 1.0),
            // Each gun's shot sounds its own impact (`play-impact-sound` with
            // `po0`, as `projectile`'s `impact` state plays it); the Scatter
            // Gun's `red-shot-hit` and the Gyro Burster's have none exported
            // (none at all for the latter); blasts sound as explosions.
            Event::Impact(hit) => match jak.gun.kind {
                GunKind::Yellow1 => ("YELLOW_GUN_BURN", map_position(hit.pos), 1.0, 1.0),
                GunKind::Yellow2 => ("YELLOW2_BURN", map_position(hit.pos), 1.0, 1.0),
                GunKind::Blue1 | GunKind::Blue2 => ("BLUE_SHOT_HIT", map_position(hit.pos), 1.0, 1.0),
                GunKind::Blue3 => ("BLUE_GUN3_RICCO", map_position(hit.pos), 1.0, 1.0),
                _ => continue,
            },
            // Jak 1's yellow eco shot (`projectile-yellow`): its fire, its
            // burst (`projectile-impact`) and its fizzle (`-dissipate`).
            Event::EcoBlast(jak_core::eco_blast::BlastEvent::Launch { pos, .. }) => ("jak1:yellow-fire", map_position(pos), 1.0, 1.0),
            Event::EcoBlast(jak_core::eco_blast::BlastEvent::Burst { pos, .. }) => ("jak1:yellow-explode", map_position(pos), 1.0, 1.0),
            Event::EcoBlast(jak_core::eco_blast::BlastEvent::Fade { pos, .. }) => ("jak1:yellow-fizzle", map_position(pos), 1.0, 1.0),
            // A blow's hit sounds on a creature only: on a block the block's
            // own sound plays (19/10, Adrii: a punch or kick into a wall
            // made Jak's hit sound).
            Event::Strike { actor: None, .. } | Event::PunchWall | Event::WorldHit { .. } => continue,
            Event::Strike { pos, .. } => (
                if jak.dark {
                    "DJAK_PUNCH_HIT"
                } else if matches!(jak.state, State::Attack | State::AttackAir { .. }) {
                    "SPIN_HIT"
                } else {
                    "PUNCH_HIT"
                },
                map_position(pos),
                1.0,
                1.0,
            ),
            Event::LightJak { on } => {
                if jak.lightjak.wings.is_some() {
                    queue.cue(
                        if on {
                            "LTJAK_WINGS_OUT"
                        } else {
                            "LTJAK_WINGS_IN"
                        },
                        at,
                        jak.time,
                        1.0,
                        1.0,
                    );
                }
                // `target-lightjak-get-on`: `jak-suspend` (LIGHT_CHARGE is the light
                // vent's); `ljak-off` changing back.
                (if on { "JAK_SUSPEND" } else { "LJAK_OFF" }, at, 1.0, 1.0)
            }
            Event::DarkJak { on } => (if on { "DJAK_TRANSFORM" } else { "DJAK_OFF" }, at, 1.0, 1.0),
            Event::Flap => ("LTJAK_FLAP", at, 1.0, 1.0),
            Event::Splash { pos, speed } => (
                if speed >= 0.0 { "SPLASH" } else { "SPLASH_OUT" },
                map_position(pos),
                1.0,
                1.0,
            ),
            Event::GunSelect { kind } => (
                match kind.mode() {
                    1 => "GUN_TRANS_1",
                    2 => "GUN_TRANS_2",
                    _ => "GUN_TRANS_3",
                },
                at,
                0.8,
                1.0,
            ),
            Event::GunFire { kind:GunKind::Red2, .. } => continue,
            Event::GunFire { kind, from } => {
                let name = match kind {
                    GunKind::Red1 => "RED_SHOT_FIRE",
                    GunKind::Red2 => "RED2_CHARGE",
                    GunKind::Red3 => "RED3_FIRE",
                    GunKind::Yellow1 => "YELLOW_SHOT_FIR",
                    GunKind::Yellow2 => "YELLOW2_FIRE",
                    GunKind::Yellow3 => "YELLOW3_FIRE",
                    GunKind::Blue1 => "BLUE_SHOT_FIRE",
                    GunKind::Blue2 => "BLUE_GUN2_SPIN",
                    GunKind::Blue3 => "BLUE_GUN3_SHOT",
                    GunKind::Dark1 if jak.weapons.charging() => "PMKR_CHARGE",
                    GunKind::Dark1 => "PMKR_FIRE",
                    GunKind::Dark2 => "GRAV_GUNX",
                    GunKind::Dark3 => "PURPLE_3_SHOT",
                };
                (name, map_position(from), 1.0, 1.0)
            }
            Event::GunBounce { kind, pos } => (
                match kind {
                    GunKind::Red3 => "RED3_BOUNCE",
                    GunKind::Yellow3 => "YELLOW3_BOUNCE",
                    GunKind::Blue3 => "BLUE_GUN3_RICCO",
                    GunKind::Dark3 => "BEEP",
                    _ => "YELLOW2_RICCO",
                },
                map_position(pos),
                1.0,
                1.0,
            ),
            Event::Explosion { kind, center, .. } => {
                let (name, gain, pitch) = match kind {
                    GunKind::Red2 => ("RED2_SHOT", 1.0, 1.0),
                    GunKind::Red3 => ("RED3_BLAST", 1.0, 1.0),
                    GunKind::Yellow3 => ("YELLOW3_ZAP", 1.0, 1.0),
                    GunKind::Dark1 => ("EXPLOSION", 1.0, 1.4),
                    GunKind::Dark3 => ("HELLCAT_EXPLODE", 1.0, 0.6),
                    _ => ("EXPLOSION", 1.0, 1.0),
                };
                // The Super Nova is heard from anywhere.
                let position = if kind == GunKind::Dark3 { at } else { map_position(center) };
                (name, position, gain, pitch)
            }
            Event::Sound { name, volume, pitch } => (name, at, volume, 2f32.powf(pitch)),
            Event::SoundAt { name, pos } => (name, map_position(pos), 1.0, 1.0),
            Event::GroundPound { .. } | Event::Lift { .. } | Event::GravityField { .. } | Event::Build { .. } | Event::Trace { .. } | Event::FlutSlam { .. } | Event::Hurt { .. } => continue,
        };
        queue.cue(name, position, jak.time, gain, pitch);
    }
    let anim = jak.chan.anim;
    if queue.players[player].last_anim != Some(anim) {
        let name = anim.name();
        let cue = if name.contains("attack-uppercut") {
            Some("UPPERCUT_SWIRL")
        } else if matches!(jak.state, State::RunningAttack) {
            Some(if jak.dark { "DJAK_PNCH_SWING" } else { "PUNCH" })
        } else if matches!(jak.state, State::Attack | State::AttackAir { .. } | State::LightJakShield) {
            Some(if jak.dark { "DJAK_CLAW" } else { "SPIN_KICK" })
        } else if name.contains("flop-air") || name.contains("flop-forward") {
            Some("FLOP_SWISH")
        } else if name.contains("flop-hit") || name.contains("flop-land") {
            Some("FLOP_HIT")
        } else if name.contains("roll-flip") {
            Some("ROLL_FLIP")
        } else if name.contains("roll-loop") {
            Some("ROLL_STN")
        } else {
            None
        };
        if let Some(cue) = cue {
            queue.cue(cue, at, jak.time, 1.0, 1.0);
        }
    }
    let me = &queue.players[player];
    if jak.dark != me.dark
        && !me.dark_transition
        && jak.dark_transition.is_none()
        && !events
            .iter()
            .any(|event| matches!(event, Event::DarkJak { .. }))
    {
        queue.cue(
            if jak.dark {
                "DJAK_TRANSFORM"
            } else {
                "DJAK_OFF"
            },
            at,
            jak.time,
            1.0,
            1.0,
        );
    }
    if jak.gun.out != queue.players[player].gun {
        queue.cue(
            if jak.gun.out {
                "GUN_TAKEOUT"
            } else {
                "GUN_PUTAWAY"
            },
            at,
            jak.time,
            0.8,
            1.0,
        );
    }
    // `target-lightjak-regen`'s "heal-rise", as the heal begins.
    let healing = jak.state == jak_core::State::LightJakRegen;
    let freezing = jak.lightjak.on && jak.time < jak.lightjak.freeze_until;
    if healing && !queue.players[player].healing {
        queue.cue("HEAL_RISE", at, jak.time, 1.0, 1.0);
    }
    if freezing && !queue.players[player].freezing {
        queue.cue("FREEZE_CLAP", at, jak.time, 1.0, 1.0);
    }

    // His animation's footsteps, landings, rolls and slides.
    let mut steps = Vec::new();
    crate::jak_steps::step_sounds(jak, &mut queue.players[player].steps, floor, &mut steps);
    for (name, volume, pitch) in steps {
        queue.cue(name, at, jak.time, volume, 2f32.powf(pitch));
    }
    queue.loops().clear();
    if jak.gun.kind==GunKind::Red2&&jak.weapons.charging(){queue.loops().insert("RED2_CHARGE",(at,1.));}
    queue.players[player].pitches.clear();
    let speed = jak.velocity().length() / jak_core::METER;
    if jak.state == State::BoardGrind && speed > 0.1 {
        queue
            .loops()
            .insert("BOARD_RAIL_MTL", (at, (speed / 12.0).clamp(0.25, 0.85)));
    } else if jak.state.is_board() && jak.control.on_surface() && speed > 1.0 {
        queue
            .loops()
            .insert("BOARD_STEADY", (at, (speed / 20.0).clamp(0.15, 0.6)));
    }
    if jak.board.charge_progress > 0.0 && jak.state == State::BoardDuckStance {
        queue.loops().insert("BOARD_CHARGE", (at, 0.6));
    }
    if jak.lightjak.shield {
        queue.loops().insert("LTJAK_SHIELD", (at, 0.75));
    }
    // The blue gun's barrels whine as they spin; the Arc Wielder hums.
    if jak.gun.out && jak.gun.spinv > 1.0 {
        let volume = (jak.gun.spinv * 0.000004577637 * 2.0).min(1.0);
        queue.loops().insert("BLUE_GUN_WHINE", (at, volume));
    }
    if jak.weapons.arc_active() {
        queue.loops().insert("BLUE_GUN2_LOOP", (at, 0.75));
    }
    if freezing {
        queue.loops().insert("LTJAK_CHILL", (at, 0.6));
    }
    let me = &mut queue.players[player];
    me.last_anim = Some(anim);
    me.dark = jak.dark;
    me.dark_transition = jak.dark_transition.is_some();
    me.gun = jak.gun.out;
    me.freezing = freezing;
    me.healing = healing;
}


impl JakSoundQueue {
 pub fn take_requests(&mut self)->Vec<JakSoundRequest>{std::mem::take(&mut self.requests)}
 pub fn loop_requests(&self)->Vec<JakSoundRequest>{self.live_loops().into_iter().map(|(name,(position,gain))|JakSoundRequest{name,position,gain,pitch:self.live_pitch(name)}).collect()}
 pub fn advance(&mut self){self.tick+=1;}
}
