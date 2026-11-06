//! Light Jak's transformation, eco, flight, healing, shield and freeze request.
use crate::anim::{self, Anim, Anims, Channel, NumFunc};
use crate::control::{STANDARD_GRAVITY, STANDARD_GRAVITY_MAX, status};
use crate::math::*;
use crate::pad::button;
use crate::surface::{self, flag};
use crate::{Event, Jak, Push, State};

/// The light eco a flight's first flap costs, and a shot fired as Light
/// Jak.
pub const SWOOP_INC: f32 = 1.0;
/// A full light eco meter.
pub const ECO_MAX: f32 = 100.0;
/// Adrii 22/10: light and dark eco last ten times as long. Every cost and
/// threshold of Jak 3's (`*FACT-bank*`, `target-lightjak.gc`,
/// `target-darkjak.gc`) times this; Light Jak's flight costs nothing.
pub const ECO_COST: f32 = 0.1;
const SWOOP_GRAVITY: f32 = 163840.0;
const SWOOP_GRAVITY_MAX: f32 = 245760.0;
/// The mashup's flight (Adrii, 22/10, replacing 21/10's endless climb on X
/// held): X held keeps the wings flapping by themselves, each flap as its
/// animation comes round again, and he comes down gently, his fall easing
/// to at most this many units a second (4 m/s) instead of diving. Every
/// press flaps as hard as the first, whenever in the air, as often as it
/// is pressed: pressing again and again climbs. Let go, he falls as usual.
const GLIDE_FALL: f32 = 16384.0;
/// Asking for the flight while changing.
const LATCH_SWOOP: u32 = 16;
pub const LONG_AGO: i64 = i64::MIN / 4;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum LightPower {
    #[default]
    Flight,
    Heal,
    Shield,
    Freeze,
}

/// Jak's animations the wings follow frame for frame; any other leaves
/// them folded.
const FOLLOWED: [Anim; 7] = [
    anim::LIGHTJAK_SWOOP1,
    anim::LIGHTJAK_SWOOP2,
    anim::LIGHTJAK_SWOOP_FALL,
    anim::LIGHTJAK_SWOOP_FALL_LOOP,
    anim::LIGHTJAK_GET_OFF,
    anim::LIGHTJAK_GET_ON_OUT,
    anim::LIGHTJAK_GET_ON_LAND,
];

const SWOOP_ANIMS: [Anim; 4] = [
    anim::LIGHTJAK_SWOOP1,
    anim::LIGHTJAK_SWOOP2,
    anim::LIGHTJAK_SWOOP_FALL,
    anim::LIGHTJAK_SWOOP_FALL_LOOP,
];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WingsMode {
    /// Folded on his back in their own stance.
    Idle,
    /// Following Jak's animation.
    Use,
    /// Folding away as Light Jak ends.
    Close,
}

/// How folded wings start: unfolding, already folded, or neither.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WingsStart {
    Open,
    Force,
    Plain,
}

/// The wings on Light Jak's back, with their own animation channel.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Wings {
    pub mode: WingsMode,
    pub chan: Channel,
    pub push: Push,
    /// Drawn: not once folded away, nor while Jak plays an animation they
    /// have none for.
    pub shown: bool,
    start: WingsStart,
    lock: bool,
    pc: u8,
    arrived: bool,
}

impl Wings {
    fn new(start: WingsStart) -> Self {
        let mut wings = Self {
            mode: WingsMode::Idle,
            chan: Channel::default(),
            push: Push::default(),
            shown: true,
            start,
            lock: false,
            pc: 0,
            arrived: true,
        };
        wings.idle(start);
        wings
    }

    fn idle(&mut self, start: WingsStart) {
        self.mode = WingsMode::Idle;
        self.start = start;
        self.lock = start != WingsStart::Plain;
        self.pc = 0;
        self.arrived = true;
    }

    fn go(&mut self, mode: WingsMode) {
        self.mode = mode;
        self.pc = 0;
        self.arrived = true;
    }

    /// Asked to spread: from folded, they follow Jak.
    fn open(&mut self) {
        if self.mode == WingsMode::Idle {
            self.go(WingsMode::Use);
        }
    }

    fn end_mode(&mut self) {
        if self.mode != WingsMode::Close {
            self.go(WingsMode::Close);
        }
    }

    fn push(&mut self, now: i64, ticks: i64) {
        self.push = Push { tick: now, ticks };
    }

    /// A do-while wait on the wings' channel.
    fn wait(&mut self, anims: &Anims) -> bool {
        if self.arrived {
            return false;
        }
        self.chan.eval(anims);
        self.arrived = true;
        self.chan.done(anims)
    }

    /// One frame: the checks, the sequence, then copying Jak's pose while
    /// following him.
    fn step(&mut self, anims: &Anims, jak: &Channel, jak_push: Push, now: i64) {
        self.arrived = false;
        let followed = FOLLOWED.contains(&jak.anim);
        match self.mode {
            WingsMode::Idle if !self.lock && followed => self.go(WingsMode::Use),
            WingsMode::Use if !followed => self.idle(WingsStart::Plain),
            _ => {}
        }
        match self.mode {
            WingsMode::Idle => self.idle_code(anims, now),
            WingsMode::Use => {
                if self.pc == 0 {
                    self.push(now, seconds(0.1));
                    self.pc = 1;
                }
                match wing_anim(jak.anim) {
                    Some(anim) => {
                        self.chan = Channel { anim, ..*jak };
                        self.push = jak_push;
                        self.shown = true;
                    }
                    None => self.shown = false,
                }
            }
            WingsMode::Close => {
                if self.pc == 0 {
                    self.push(now, seconds(0.1));
                    self.chan
                        .set(anim::WINGS_LIGHTJAK_GET_OFF, NumFunc::seek(1.0), 0.0);
                    self.pc = 1;
                } else if self.pc == 1 && self.wait(anims) {
                    self.shown = false;
                    self.pc = 2;
                }
            }
        }
    }

    fn idle_code(&mut self, anims: &Anims, now: i64) {
        loop {
            match self.pc {
                0 => match self.start {
                    WingsStart::Open => {
                        let to = anims.aframe(anim::WINGS_LIGHTJAK_GET_ON_LAND, 45.0);
                        self.chan.set(
                            anim::WINGS_LIGHTJAK_GET_ON_LAND,
                            NumFunc::seek_to(to, 1.0),
                            0.0,
                        );
                        self.pc = 1;
                    }
                    WingsStart::Force => {
                        let max = anims.max(anim::WINGS_LIGHTJAK_STANCE);
                        self.chan
                            .set(anim::WINGS_LIGHTJAK_STANCE, NumFunc::Identity, max);
                        self.pc = 3;
                        return;
                    }
                    WingsStart::Plain => self.pc = 4,
                },
                1 => {
                    if !self.wait(anims) {
                        return;
                    }
                    self.chan.func = NumFunc::seek(1.0);
                    self.pc = 2;
                }
                2 => {
                    if self.chan.done(anims) {
                        self.pc = 4;
                        continue;
                    }
                    if !self.arrived {
                        self.chan.eval(anims);
                        self.arrived = true;
                        continue;
                    }
                    return;
                }
                3 => self.pc = 4,
                4 => {
                    self.lock = false;
                    self.push(now, seconds(0.1));
                    self.pc = 5;
                }
                5 => {
                    self.chan
                        .set(anim::WINGS_LIGHTJAK_STANCE, NumFunc::seek(1.0), 0.0);
                    self.pc = 6;
                }
                _ => {
                    if !self.wait(anims) {
                        return;
                    }
                    self.pc = 5;
                }
            }
        }
    }
}

/// The wings' animation to an animation of Jak's: its name with `wings-`
/// after Jak's prefix.
fn wing_anim(jak: Anim) -> Option<Anim> {
    let rest = jak.name().strip_prefix("jakb-")?;
    Anim::by_name(&format!("jakb-wings-{rest}"))
}

/// Light Jak's part of Jak.
#[derive(Clone, Debug, PartialEq)]
pub struct LightJak {
    pub power: LightPower,
    pub power_started: i64,
    pub shield: bool,
    /// The shield up last frame (its opening's sparkles go once).
    pub shield_was: bool,
    pub shield_hit_time: i64,
    /// When the shield's last kick began (`shield-start-time`), and since when
    /// it has been up.
    pub shield_started: i64,
    pub shield_on_time: i64,
    pub freeze_until: i64,
    /// The freeze asked for (`lightjak-stage freeze`), its clap still to come:
    /// straight away as Light Jak, after the get-on otherwise.
    pub freeze_wanted: bool,
    /// When the clap's hands last met (aframe 20: its burst, the pad's buzz
    /// and the camera's smush), `LONG_AGO` before any.
    pub freeze_clap: i64,
    pub health_fraction: f32,
    pub heal_fraction: f32,
    /// Changed into Light Jak.
    pub on: bool,
    /// The flight is his: the wings come with the change.
    pub swoop: bool,
    /// Light eco, out of [`ECO_MAX`].
    pub eco: f32,
    /// When he last changed into Light Jak.
    pub start_time: i64,
    /// When the change back began, out of the moves that cannot stop for
    /// it; 0 when it has not.
    pub latch_out_time: i64,
    /// What the power button asked for while it was held.
    pub get_on_latch: u32,
    /// Letting go of the power button does not change him back.
    pub get_off_lock: bool,
    pub swoop_count: u32,
    /// How far the change has gone, 0 to 1.
    pub interp: f32,
    /// The upward speed a flap sets.
    pub swoop_impulse: f32,
    /// This flap is X held keeping the wings going, not a press.
    pub glide: bool,
    pub wings: Option<Wings>,
    /// `target-lightjak.gc`'s effects to launch (its `part-tracker`s): a
    /// group on one of his joints, followed there, or left where it began.
    pub(crate) fx: Vec<(&'static str, LightFxAt)>,
    /// Those following a joint: their tag, group and joint.
    pub(crate) fx_tracked: Vec<(u64, &'static str, usize)>,
    pub(crate) fx_tag: u64,
    /// Healing last frame (`target-lightjak-regen`'s glow once a heal).
    pub(crate) healing_was: bool,
    /// The heal's light up (its loop, `target-lightjak-regen`): the light
    /// shafts reach off the top of the screen.
    pub healing: bool,
    /// Points of health the heal has still to give, the heal loops played,
    /// and when the last point came (`eco-source-time`).
    regen_left: i32,
    regen_loops: u32,
    regen_last: i64,
    /// Light Jak before the heal (`lightjak-before-powerjak`): he stays so
    /// after it; else the change ends with it.
    pub(crate) before_regen: bool,
}

/// Where a Light Jak effect goes (`part-tracker-spawn`'s `:mat-joint`, or
/// `*launch-matrix*` set to a point, or his feet).
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum LightFxAt {
    Follow(usize),
    Joint(usize),
    Feet,
}

/// jakb's joints (`joint-nodes.gc`): `main`, `chest`, `neckB`, `sk_lhand`,
/// `sk_rhand`.
pub(crate) const JOINT_CHEST: usize = 6;
pub(crate) const JOINT_NECK: usize = 8;
pub(crate) const JOINT_LHAND: usize = 19;
pub(crate) const JOINT_RHAND: usize = 28;
/// `add-connection *part-engine* self 8 self 650`: his eyes' glow, either
/// side of `neckB`.
pub(crate) const EYES: [glam::Vec3; 2] = [glam::Vec3::new(278.528, 643.072, 757.76), glam::Vec3::new(-278.528, 643.072, 757.76)];

impl Default for LightJak {
    fn default() -> Self {
        Self {
            power: LightPower::Flight,
            power_started: 0,
            shield: false,
            shield_was: false,
            shield_hit_time: LONG_AGO,
            shield_started: LONG_AGO,
            shield_on_time: LONG_AGO,
            freeze_until: 0,
            freeze_wanted: false,
            freeze_clap: LONG_AGO,
            health_fraction: 1.0,
            heal_fraction: 0.0,
            on: false,
            swoop: false,
            eco: 0.0,
            start_time: LONG_AGO,
            latch_out_time: 0,
            get_on_latch: 0,
            get_off_lock: false,
            swoop_count: 0,
            interp: 0.0,
            swoop_impulse: 0.0,
            fx: Vec::new(),
            fx_tracked: Vec::new(),
            fx_tag: 0,
            healing_was: false,
            healing: false,
            regen_left: 0,
            regen_loops: 0,
            regen_last: 0,
            before_regen: false,
            glide: false,
            wings: None,
        }
    }
}

impl LightJak {
    /// What the debug readout calls his state.
    pub fn describe(&self) -> String {
        let wings = self.wings.map_or("none".to_owned(), |w| {
            format!(
                "{:?} {}{}",
                w.mode,
                w.chan.anim.name().trim_start_matches("jakb-wings-"),
                if w.shown { "" } else { " (hidden)" }
            )
        });
        format!(
            "{}  eco {:.0}/{:.0}  flight {}  wings {}  swoops {}{}",
            if self.on { "on" } else { "off" },
            self.eco,
            ECO_MAX,
            if self.swoop { "yes" } else { "no" },
            wings,
            self.swoop_count,
            if self.latch_out_time != 0 {
                "  ending"
            } else {
                ""
            }
        )
    }
}

impl Jak {
    /// `lightjak-shield hit`: each distinct hit costs two eco, plays its
    /// original particle group and sound. Contact is debounced while the hit
    /// animation reacts, instead of charging eco every simulation frame.
    pub fn light_shield_hit(&mut self) -> bool {
        if !self.lightjak.shield || self.time - self.lightjak.shield_hit_time < seconds(0.33) {
            return false;
        }
        self.lightjak.shield_hit_time = self.time;
        self.lightjak.eco = (self.lightjak.eco - 2.0 * ECO_COST).max(0.0);
        if let Some(particles) = self.particles.as_mut() {
            particles.spawn(
                "group-lightjak-shield-hit",
                self.control.trans + glam::Vec3::Y * METER,
                glam::Mat3::IDENTITY,
            );
        }
        self.events.push(Event::Sound {
            name: "SHIELD_HIT",
            volume: 1.0,
            pitch: 0.0,
        });
        true
    }

    /// How fast the world's time runs against his (Light Jak's freeze,
    /// `freeze-control`): from 1 down to 0.05 over its first 0.95 s (its
    /// `clock-pos` seeks 0.05 at one a second), held the freeze's 15 s,
    /// then back up at two a second. The entities' clock runs at it.
    pub fn world_time_scale(&self) -> f32 {
        let start = self.lightjak.freeze_until - seconds(15.0);
        if self.lightjak.freeze_until == 0 || self.time < start {
            return 1.0;
        }
        let t = (self.time - start) as f32 / crate::TICKS_PER_SECOND as f32;
        if t < 0.95 {
            1.0 - t
        } else if t < 15.95 {
            0.05
        } else {
            (0.05 + (t - 15.95) * 2.0).min(1.0)
        }
    }
}

impl Jak {
    /// Light Jak, ready to act.
    pub fn light(&self) -> bool {
        self.lightjak.on
    }

    /// Holding the power button where Jak may change.
    pub(crate) fn want_to_powerjak(&self) -> bool {
        (self.pad.hold(button::L2) || self.lightjak.get_on_latch != 0)
            && !self.state.is_board()
            && !self.powers_blocked
            && self.time_elapsed(self.lightjak.start_time, seconds(0.05))
    }

    /// Able to change into Light Jak for `power` now: with some light eco,
    /// but for the flight, which is free (Adrii 22/10).
    fn want_to_lightjak_for(&self, power: LightPower) -> bool {
        !self.state.is_board()
            && !self.powers_blocked
            && self.time_elapsed(self.lightjak.start_time, seconds(0.05))
            && (0.0 < self.lightjak.eco || power == LightPower::Flight)
            && self.lightjak.latch_out_time == 0
    }

    fn spawn_wings(&mut self, start: WingsStart) {
        if self.lightjak.wings.is_none() {
            self.lightjak.wings = Some(Wings::new(start));
        }
    }

    /// Light Jak ends: at once, or by starting the change back the moves
    /// in progress finish first.
    pub(crate) fn lightjak_end_mode(&mut self, now: bool) {
        if !self.lightjak.on {
            return;
        }
        let l = &mut self.lightjak;
        if now {
            l.get_off_lock = false;
            l.on = false;
            l.shield = false;
            if l.eco < ECO_COST {
                l.eco = (l.eco - ECO_COST).max(0.0);
            }
            if !self
                .chan
                .is_any(&[anim::LIGHTJAK_GET_OFF, anim::LIGHTJAK_GET_ON])
                && self.chan.anim.name().starts_with("jakb-lightjak-")
            {
                self.chan.set(anim::STANCE_LOOP, NumFunc::Identity, 0.0);
            }
            let l = &mut self.lightjak;
            l.wings = None;
            l.latch_out_time = 0;
            self.events.push(Event::LightJak { on: false });
        } else if l.latch_out_time == 0 {
            l.latch_out_time = self.time;
            if let Some(w) = &mut l.wings {
                w.end_mode();
            }
        }
    }

    /// Every frame on foot: the power button's lock, the change back once
    /// the eco runs out, and the wings.
    pub(crate) fn lightjak_process(&mut self) {
        // A freeze asked for waits for its get-on or its clap; one cut off asks for nothing.
        if self.lightjak.freeze_wanted
            && !matches!(self.state, State::LightJakGetOn { .. } | State::PowerJakGetOn | State::LightJakFreeze)
        {
            self.lightjak.freeze_wanted = false;
        }
        let dt = crate::FRAME_TICKS as f32 / crate::TICKS_PER_SECOND as f32;
        let light = &mut self.lightjak;
        light.shield = light.on
            // Original shield process listens to Circle independently of flight.
            && self.time >= light.freeze_until
            && light.latch_out_time == 0
            && self.pad.hold(button::CIRCLE)
            && light.eco >= ECO_COST;
        if light.shield {
            light.eco = (light.eco - 3.0 * ECO_COST * dt).max(0.0);
        }
        // The shield's post: up for 0.1 s, on his way into a standing, a jump, a fall
        // or a landing, and on a surface since its last kick: `target-lightjak-shield`.
        if !light.shield {
            light.shield_on_time = LONG_AGO;
        } else if light.shield_on_time == LONG_AGO {
            light.shield_on_time = self.time;
        }
        if light.shield
            && self.time - light.shield_on_time >= seconds(0.1)
            && self.control.last_time_on_surface >= light.shield_started
            && self.state_time == self.time
            && matches!(self.state, State::Stance | State::Jump { .. } | State::Falling { .. } | State::HitGround { .. } | State::DoubleJump { .. })
            && self.pending.is_none()
        {
            light.shield_started = self.time;
            self.pending = Some(State::LightJakShield);
        }

        if !self.pad.hold(button::L2) && self.state != State::PowerJakGetOn {
            self.lightjak.get_off_lock = false;
        }
        if self.lightjak.latch_out_time != 0 {
            if self.time_elapsed(self.lightjak.latch_out_time, seconds(0.4)) {
                self.lightjak_end_mode(true);
            }
        } else if self.lightjak.on {
            let changing = matches!(
                self.state,
                State::LightJakGetOn { .. }
                    | State::LightJakGetOff
                    | State::LightJakSwoop { .. }
                    | State::LightJakSwoopFalling
            );
            // Flying is free (Adrii 22/10): no eco never takes the wings off. The
            // flight is its own stage bit (`lightjak-stage swoop`), kept whatever came after it.
            let flying = self.lightjak.swoop;
            if self.lightjak.eco < ECO_COST && !changing && !flying && self.attack.danger.is_none() {
                if matches!(self.state, State::Stance | State::HitGround { .. }) {
                    self.pending = Some(State::LightJakGetOff);
                } else {
                    self.lightjak_end_mode(false);
                }
            }
        } else {
            self.lightjak.interp = 0.0;
        }
        let (chan, push, now) = (self.chan, self.push, self.time);
        if let Some(w) = &mut self.lightjak.wings {
            w.step(&self.anims, &chan, push, now);
        }
    }

    /// A double jump's press as Light Jak with the flight: a swoop instead.
    pub(crate) fn swoop_instead(&self) -> bool {
        let l = &self.lightjak;
        self.pad.pressed(button::X)
            && l.on
            && l.swoop
            && self.time_elapsed(
                self.control.last_time_of_stuck,
                crate::target::STUCK_TIMEOUT,
            )
            && self.control.current.flags & (flag::NO_ATTACK | flag::NO_FEET) == 0
            && l.latch_out_time == 0
    }

    pub(crate) fn lightjak_enter(&mut self, state: &State) {
        match *state {
            State::LightJakGetOff => {
                self.control.mod_surface = surface::foot::LIGHTJAK_TRANS;
                self.lightjak.fx.push(("group-lightjak-get-off", LightFxAt::Follow(JOINT_CHEST)));
            }
            State::LightJakSwoop { first, held } => {
                let l = &mut self.lightjak;
                l.get_off_lock = true;
                self.spawn_wings(WingsStart::Open);
                let l = &mut self.lightjak;
                if first {
                    // (Jak 3 takes `SWOOP_INC` a flight; Adrii 22/10: free.)
                    if let Some(w) = &mut l.wings {
                        w.open();
                    }
                }
                l.swoop_count += 1;
                // A flap of the wings kept going by X held, not a press.
                l.glide = !self.pad.pressed(button::X);
                // Every flap the first's (the mashup's; Jak 3's later flaps
                // grow with the time since the last).
                let _ = (first, held);
                let lift = 1.4;
                l.swoop_impulse = (1.0 / 600.0) * (285.0 * lift) * SWOOP_GRAVITY;
                let c = &mut self.control;
                c.gravity_max = SWOOP_GRAVITY_MAX;
                c.gravity_length = SWOOP_GRAVITY;
                c.status &= !(status::ON_SURFACE | status::ON_GROUND | status::TOUCH_SURFACE);
                c.mod_surface = surface::foot::LIGHTJAK_SWOOP;
                c.last_trans_any_surf = c.trans;
                self.foot.sliding_start_time = self.time;
                self.events.push(Event::Flap);
            }
            State::LightJakSwoopFalling => self.control.mod_surface = surface::JUMP,
            State::LightJakRegen => {
                self.lightjak.get_off_lock = true;
                self.lightjak.healing = false;
            }
            State::LightJakShield | State::LightJakFreeze => self.lightjak.get_off_lock = true,
            _ => {}
        }
    }

    pub(crate) fn lightjak_exit(&mut self, state: &State, next: &State) {
        match *state {
            State::PowerJakGetOn => {
                self.lightjak.get_on_latch = 0;
                self.gun.end_mode();
            }
            State::LightJakGetOn { .. } => {
                self.lightjak.interp = 1.0;
                self.danger_set(None);
                self.restore_gravity();
                self.gun.end_mode();
            }
            State::LightJakGetOff => self.lightjak_end_mode(true),
            // Its exit: gravity back, the light down.
            State::LightJakRegen => {
                self.restore_gravity();
                self.lightjak.healing = false;
                self.target_exit_light();
            }
            State::LightJakShield => {
                self.restore_gravity();
                self.target_exit_light();
            }
            State::LightJakFreeze => self.target_exit_light(),
            State::LightJakSwoop { .. } | State::LightJakSwoopFalling => {
                if !matches!(next, State::LightJakSwoop { .. }) {
                    self.restore_gravity();
                }
                self.target_exit_light();
            }
            _ => {}
        }
    }

    fn restore_gravity(&mut self) {
        self.control.gravity_max = STANDARD_GRAVITY_MAX;
        self.control.gravity_length = STANDARD_GRAVITY;
    }

    fn target_exit_light(&mut self) {
        let c = &mut self.control;
        c.mod_surface = surface::WALK;
        c.draw_offset_y = 0.0;
        c.force_turn_to_strength = 0.0;
        c.bend_target = 0.0;
        self.danger_set(None);
    }

    fn near_ground(&self, height: f32) -> bool {
        let c = &self.control;
        c.height_above_ground() < height
            && matches!(
                c.gspot_pat.mode,
                crate::collide::PatMode::Ground | crate::collide::PatMode::Halfpipe
            )
    }

    /// The mashup's flight on the wings: X held, the fall eases to a glide.
    fn hold_flight(&mut self) {
        if !self.pad.hold(button::X) {
            return;
        }
        let c = &mut self.control;
        let up = c.gravity_normal.dot(c.transv);
        if up < -GLIDE_FALL {
            // A sixth of the excess off each frame, counting the pull this
            // frame still adds: it eases in to the glide, not a stop.
            let pull = c.gravity_length * crate::SECONDS_PER_FRAME;
            let eased = -GLIDE_FALL + (up + GLIDE_FALL) * (5.0 / 6.0) + pull;
            c.transv += c.gravity_normal * (eased - up);
        }
    }

    /// X held as a flap's animation ends: the next flap, by itself.
    fn held_flap(&self) -> Option<State> {
        self.pad.hold(button::X).then_some(State::LightJakSwoop { first: false, held: 0.0 })
    }

    /// The mashup's (Adrii, 21/10): as Light Jak with the flight, X in the
    /// air flaps again, from any fall.
    pub(crate) fn flap_from_the_air(&self) -> Option<State> {
        let l = &self.lightjak;
        (l.on
            && l.swoop
            && l.wings.is_some()
            && !self.control.on_surface()
            && self.pad.pressed(button::X)
            && matches!(self.state, State::Falling { .. } | State::DoubleJump { .. }))
        .then_some(State::LightJakSwoop { first: false, held: 1.0 })
    }

    pub(crate) fn lightjak_trans(&mut self) -> Option<State> {
        if matches!(self.state, State::LightJakSwoop { .. } | State::LightJakSwoopFalling) {
            self.hold_flight();
        }
        match self.state {
            State::LightJakSwoop { .. } => {
                if self.control.on_surface() {
                    return Some(State::HitGround { stuck: false });
                }
                self.foot.sliding_start_time = self.time;
                let up = self.control.gravity_normal.dot(self.control.transv);
                (up < 0.0 && self.near_ground(4096.0)).then_some(State::LightJakSwoopFalling)
            }
            State::LightJakSwoopFalling => {
                if self.pad.hold(button::X) {
                    return Some(State::LightJakSwoop {
                        first: false,
                        held: 1.0,
                    });
                }
                self.falling_trans(0, false)
            }
            _ => None,
        }
    }

    pub(crate) fn lightjak_code(&mut self) -> Option<State> {
        match self.state {
            State::PowerJakGetOn => self.powerjak_get_on_code(),
            State::LightJakGetOn { swoop } => self.lightjak_get_on_code(swoop),
            State::LightJakGetOff => self.lightjak_get_off_code(),
            State::LightJakRegen => self.regen_code(),
            State::LightJakShield => self.shield_code(),
            State::LightJakFreeze => self.freeze_code(),
            State::LightJakSwoop { first: true, .. } => self.swoop_code(),
            State::LightJakSwoop { first: false, .. } => self.swoop_again_code(),
            State::LightJakSwoopFalling => self.swoop_falling_code(),
            _ => None,
        }
    }

    /// After the change: the heal it was for (`lightjak-stage regen`), else
    /// standing.
    /// `target-lightjak-freeze`'s code: light on both hands, the clap from aframe
    /// 0 (9 once Light Jak for 2 s) to 20, where the burst goes from the left hand
    /// and `freeze-control` starts slowing the world, then the rest of it.
    fn freeze_code(&mut self) -> Option<State> {
        loop {
            match self.code.pc {
                0 => {
                    self.lightjak.fx.extend([
                        ("group-lightjak-freeze-hands", LightFxAt::Follow(JOINT_LHAND)),
                        ("group-lightjak-freeze-hands", LightFxAt::Follow(JOINT_RHAND)),
                    ]);
                    self.control.mod_surface = surface::foot::ROLL_FLIP;
                    let from = if self.time_elapsed(self.lightjak.start_time, seconds(2.0)) { 9.0 } else { 0.0 };
                    let (start, clap) = (self.aframe(anim::LIGHTJAK_FREEZE, from), self.aframe(anim::LIGHTJAK_FREEZE, 20.0));
                    self.ja_push(seconds(0.1));
                    self.ja_set(anim::LIGHTJAK_FREEZE, NumFunc::seek_to(clap, 1.0), start);
                    self.goto(1);
                }
                1 => {
                    if self.ja_wait() {
                        self.lightjak.fx.push(("group-lightjak-freeze-burst", LightFxAt::Joint(JOINT_LHAND)));
                        self.lightjak.freeze_clap = self.time;
                        self.lightjak.freeze_wanted = false;
                        // world_time_scale counts its 15 s back from here.
                        self.lightjak.freeze_until = self.time + seconds(15.0);
                        let frame = self.chan.frame;
                        self.ja_set(anim::LIGHTJAK_FREEZE, NumFunc::seek(1.0), frame);
                        self.goto(2);
                        return None;
                    }
                    return None;
                }
                2 => {
                    if self.ja_wait() {
                        return Some(State::Stance);
                    }
                    return None;
                }
                _ => return None,
            }
        }
    }

    /// `target-lightjak-shield`'s code: up 8.25 m/s off the ground (in the air,
    /// enough to rise half a metre more), its own gravity, a turn of 7.5 degrees
    /// either side of the kick, aligned to its animation.
    fn shield_code(&mut self) -> Option<State> {
        loop {
            match self.code.pc {
                0 => {
                    let c = &mut self.control;
                    c.mod_surface = if c.status & status::ON_GROUND != 0 { surface::foot::ATTACK } else { surface::foot::JUMP_ATTACK };
                    let up = c.transv.y;
                    c.transv.y = if up <= 0.0 {
                        33775.48
                    } else {
                        let spf = crate::FRAME_TICKS as f32 / crate::TICKS_PER_SECOND as f32;
                        let rise = up / (c.gravity_length * spf) / 2.0 * spf * up;
                        1024.0 + (245760.0 * (2048.0 + rise)).sqrt()
                    };
                    c.gravity_length = 122880.0;
                    c.last_trans_any_surf = c.trans;
                    self.rotate_heading(-1365.3334);
                    self.ja_push(seconds(0.05));
                    let rate = self.control.current.align_speed;
                    self.ja_play(anim::LIGHTJAK_SHIELD, rate);
                    self.goto(1);
                }
                1 => {
                    if self.ja_wait() {
                        self.rotate_heading(-1365.3334);
                        return Some(if self.control.on_surface() { State::Stance } else { State::Falling { uppercut: false } });
                    }
                    self.compute_delta_align();
                    self.align(crate::align::opts::QUAT, 1.0, 1.0, 1.0);
                    return None;
                }
                _ => return None,
            }
        }
    }

    fn after_get_on(&mut self) -> State {
        if self.lightjak.freeze_wanted {
            return State::LightJakFreeze;
        }
        if self.lightjak.power == LightPower::Heal && self.lightjak.health_fraction < 1.0 {
            self.lightjak.before_regen = false;
            State::LightJakRegen
        } else {
            State::Stance
        }
    }

    /// Triangle as Light Jak with health to win, out of a standing, a walk
    /// or a fall (`target-stance`, `target-walk`, `target-falling-trans`).
    pub(crate) fn regen_wanted(&mut self) -> bool {
        let l = &self.lightjak;
        let wanted = l.on && self.pad.pressed(button::TRIANGLE) && l.health_fraction < 1.0 && l.latch_out_time == 0;
        if wanted {
            self.lightjak.before_regen = true;
        }
        wanted
    }

    /// `target-lightjak-regen`'s code: the heal's rise (from its artist
    /// frame 0 out of Light Jak's stance, 14 in the air, 6 else; forward
    /// speed held to 0.6 of what it was), its light from the sky and the
    /// loop, weightless, a point of health each half second while there is
    /// light eco for it (12.5 a point: `eco-pill-light-max-default` over
    /// `health-max-default`), two loops at least; then its end on the
    /// ground (1.4 m or less under him) or a fall.
    fn regen_code(&mut self) -> Option<State> {
        const HEALTH_MAX: f32 = 8.0;
        const ECO_A_POINT: f32 = ECO_MAX / HEALTH_MAX;
        loop {
            match self.code.pc {
                0 => {
                    let l = &mut self.lightjak;
                    let missing = (1.0 - l.health_fraction) * HEALTH_MAX;
                    l.regen_left = missing.min((l.eco / ECO_A_POINT).ceil()) as i32;
                    l.regen_loops = 0;
                    l.regen_last = self.time;
                    l.healing = false;
                    let from = if self.chan.is(anim::LIGHTJAK_STANCE) {
                        self.ja_push(seconds(0.05));
                        0.0
                    } else if self.control.mod_surface.flags & flag::AIR != 0 && !self.control.on_surface() {
                        self.ja_push(seconds(0.1));
                        14.0
                    } else {
                        self.ja_push(seconds(0.1));
                        6.0
                    };
                    let frame = self.aframe(anim::LIGHTJAK_HEAL, from);
                    self.ja_set(anim::LIGHTJAK_HEAL, NumFunc::seek(1.0), frame);
                    self.goto(1);
                }
                1 => {
                    if self.code.arrived {
                        self.code.arrived = false;
                        let speed = self.control.ctrl_xz_vel * 0.6;
                        self.set_forward_vel(speed);
                        return None;
                    }
                    self.chan.eval(&self.anims);
                    if self.chan.done(&self.anims) {
                        // The light from the sky, at his feet; weightless.
                        self.lightjak.fx.push(("group-lightjak-regen", LightFxAt::Feet));
                        self.lightjak.healing = true;
                        self.control.gravity_max = 0.0;
                        self.control.gravity_length = 0.0;
                        self.ja_play(anim::LIGHTJAK_HEAL_LOOP, 1.0);
                        self.goto(2);
                    } else {
                        self.code.arrived = true;
                    }
                }
                2 => {
                    if self.code.arrived {
                        self.code.arrived = false;
                        let l = &mut self.lightjak;
                        if l.regen_left > 0 && self.time - l.regen_last >= seconds(0.5) {
                            l.regen_last = self.time;
                            let point = (1.0 / HEALTH_MAX).min(1.0 - l.health_fraction);
                            l.heal_fraction += point;
                            l.health_fraction += point;
                            l.eco = (l.eco - ECO_A_POINT).max(0.0);
                            l.regen_left -= 1;
                        }
                        return None;
                    }
                    self.chan.eval(&self.anims);
                    if self.chan.done(&self.anims) {
                        self.lightjak.regen_loops += 1;
                        if self.lightjak.regen_left > 0 || self.lightjak.regen_loops < 2 {
                            self.ja_play(anim::LIGHTJAK_HEAL_LOOP, 1.0);
                        } else {
                            self.restore_gravity();
                            self.lightjak.healing = false;
                            if self.control.height_above_ground() <= 5734.4 {
                                self.ja_push(seconds(0.05));
                                self.ja_play(anim::LIGHTJAK_HEAL_END, 1.0);
                                self.goto(3);
                                continue;
                            }
                            if !self.lightjak.before_regen {
                                self.lightjak_end_mode(false);
                            }
                            return Some(State::Falling { uppercut: false });
                        }
                    }
                    self.code.arrived = true;
                }
                _ => {
                    if self.code.arrived {
                        self.code.arrived = false;
                        return None;
                    }
                    self.chan.eval(&self.anims);
                    if self.chan.done(&self.anims) {
                        if !self.lightjak.before_regen {
                            self.lightjak_end_mode(false);
                        }
                        return Some(State::Stance);
                    }
                    self.code.arrived = true;
                }
            }
        }
    }

    /// The falling animation while waiting to land before a change.
    fn falling_anim_trans(&mut self) {
        let c = self.chan;
        if !c.is_any(&[anim::JUMP_LOOP, anim::JUMP_LAND]) {
            self.ja_push(seconds(0.33));
            self.chan.anim = anim::JUMP_LOOP;
        } else if self.control.on_surface()
            && !c.is(anim::JUMP_LAND)
            && self.control.status & status::ON_WATER == 0
        {
            self.ja_push(seconds(0.02));
            self.chan.anim = anim::JUMP_LAND;
        } else if c.is(anim::JUMP_LOOP) {
            self.chan
                .eval_with(&self.anims, NumFunc::Loop { rate: 1.0 });
        } else {
            self.chan.eval_with(&self.anims, NumFunc::seek(1.0));
        }
    }

    /// Waiting in the air for the ground, at most a second; then a fall.
    fn wait_to_land(&mut self) -> Result<bool, State> {
        if self.control.on_surface() {
            return Ok(true);
        }
        self.falling_anim_trans();
        self.code.count += crate::FRAME_TICKS as i32;
        if i64::from(self.code.count) >= seconds(1.0) {
            return Err(State::Falling { uppercut: false });
        }
        Ok(false)
    }

    fn blending(&self) -> bool {
        self.time < self.push.tick + self.push.ticks
    }

    /// The power button held: what it is asked for, and when it lets go.
    fn powerjak_poll(&mut self, settled: bool) -> Option<State> {
        let light = self.lightjak.on;
        let l2 = self.pad.hold(button::L2);
        if light && !l2 && self.lightjak.get_on_latch == 0 {
            return Some(if self.lightjak.get_off_lock {
                State::Stance
            } else {
                State::LightJakGetOff
            });
        }
        if (!self.want_to_powerjak() || self.pad.pressed(button::R2))
            && self.time_elapsed(self.state_time, seconds(0.05))
        {
            return Some(State::Stance);
        }
        if self.pad.pressed(button::TRIANGLE) {
            self.lightjak.get_off_lock = true;
            if self.lightjak.health_fraction < 1.0 && self.lightjak.eco >= ECO_COST {
                // Latch 8: Light Jak already, straight to the heal; else the
                // change first, the heal after it.
                if light {
                    self.lightjak.before_regen = true;
                    return Some(State::LightJakRegen);
                }
                self.lightjak.power = LightPower::Heal;
                self.lightjak.get_on_latch = 8;
            }
        }
        let mut latched = false;
        if self.pad.pressed(button::X) {
            self.lightjak.get_off_lock = true;
            if self.control.current.flags & (flag::NO_ATTACK | flag::NO_FEET) == 0
                && self.want_to_lightjak_for(LightPower::Flight)
            {
                self.lightjak.get_on_latch = LATCH_SWOOP;
                self.lightjak.power = LightPower::Flight;
                latched = true;
            }
        }
        if !latched && self.pad.pressed(button::SQUARE) {
            self.lightjak.get_off_lock = true;
            if self.lightjak.eco >= 2.0 * ECO_COST && self.time >= self.lightjak.freeze_until && !self.lightjak.freeze_wanted {
                self.lightjak.power = LightPower::Freeze;
                self.lightjak.get_on_latch = 32;
            }
        }
        if !latched && self.pad.pressed(button::CIRCLE) {
            self.lightjak.get_off_lock = true;
            if self.lightjak.eco >= 3.0 * ECO_COST {
                self.lightjak.power = LightPower::Shield;
                self.lightjak.get_on_latch = 64;
            }
        }
        if self.blending() || !settled {
            return None;
        }
        if self.lightjak.get_on_latch != 0 && self.want_to_lightjak_for(self.lightjak.power) {
            let swoop = self.lightjak.power == LightPower::Flight;
            self.lightjak.power_started = self.time;
            if self.lightjak.power == LightPower::Freeze {
                self.lightjak.eco -= (50.0 * ECO_COST).min(self.lightjak.eco - ECO_COST);
                // Latch 32: Light Jak already, straight to the clap; else the get-on
                // first (`lightjak-stage freeze`), the clap after it.
                self.lightjak.freeze_wanted = true;
                if light {
                    self.lightjak.get_on_latch = 0;
                    return Some(State::LightJakFreeze);
                }
            }
            if light {
                // Powers are concurrent stage bits in Jak 3, not exclusive modes.
                self.lightjak.swoop |= swoop;
                if swoop {
                    self.spawn_wings(WingsStart::Open);
                }
                self.lightjak.get_on_latch = 0;
                return Some(State::Stance);
            }
            return Some(State::LightJakGetOn { swoop });
        }
        if self.lightjak.get_on_latch != 0 {
            self.lightjak.get_on_latch = 0;
        }
        None
    }

    /// Holding the power button: Jak stops and readies the change while it
    /// is held, and changes once a power is asked for.
    fn powerjak_get_on_code(&mut self) -> Option<State> {
        loop {
            match self.code.pc {
                0 => {
                    self.control.mod_surface = surface::foot::LIGHTJAK_TRANS;
                    self.lightjak.get_on_latch = 0;
                    self.gun.end_mode();
                    self.set_forward_vel(0.0);
                    self.code.count = 0;
                    self.goto(1);
                }
                1 => match self.wait_to_land() {
                    Err(fall) => return Some(fall),
                    Ok(true) => self.goto(2),
                    Ok(false) => return self.powerjak_poll(false),
                },
                2 => {
                    self.ja_push(seconds(0.1));
                    self.ja_play(anim::POWERJAK_GET_ON, 1.0);
                    self.goto(3);
                }
                3 | 5 => {
                    if !self.code.arrived {
                        self.chan.eval(&self.anims);
                        self.code.arrived = true;
                        if self.chan.done(&self.anims) {
                            self.goto(4);
                            continue;
                        }
                    }
                    return self.powerjak_poll(true);
                }
                _ => {
                    self.ja_play(anim::POWERJAK_GET_ON_LOOP, 1.0);
                    self.goto(5);
                }
            }
        }
    }

    /// The change into Light Jak: he rises on the change's animation,
    /// comes out of it and lands, the wings spreading if the flight came
    /// with it.
    fn lightjak_get_on_code(&mut self, swoop: bool) -> Option<State> {
        loop {
            match self.code.pc {
                0 => {
                    self.gun.end_mode();
                    self.dark = false;
                    self.dark_transition = None;
                    let l = &mut self.lightjak;
                    l.swoop = swoop;
                    l.start_time = self.time;
                    l.latch_out_time = 0;
                    l.on = true;
                    self.events.push(Event::LightJak { on: true });
                    self.control.mod_surface = surface::foot::LIGHTJAK_TRANS;
                    self.start_attack();
                    self.danger_set(Some(crate::Danger::GetOn));
                    self.set_forward_vel(0.0);
                    self.ja_push(seconds(0.1));
                    self.ja_play(anim::LIGHTJAK_GET_ON, 1.0);
                    self.code.x = if swoop { 2.0 } else { 1.0 };
                    self.goto(1);
                }
                1 => {
                    if !self.code.arrived {
                        self.chan.eval(&self.anims);
                        self.code.arrived = true;
                        if self.chan.done(&self.anims) {
                            self.goto(2);
                            continue;
                        }
                    }
                    self.compute_delta_align();
                    self.align(crate::align::opts::Y_VEL, 1.0, self.code.x, 1.0);
                    return None;
                }
                2 => {
                    // The change itself, on his chest. Its charge
                    // (`group-lightjak-get-on-charge`, the shafts) lasts only
                    // while Jak 3 waits in `jakb-lightjak-get-on-loop` for the
                    // light body and is sent `'die` once it is there; here it
                    // already is, so it would end at once.
                    self.lightjak.fx.push(("group-lightjak-get-on", LightFxAt::Follow(JOINT_CHEST)));
                    self.control.transv = glam::Vec3::ZERO;
                    self.lightjak.interp = 1.0;
                    self.restore_gravity();
                    self.chan
                        .set(anim::LIGHTJAK_GET_ON_OUT, NumFunc::seek(1.0), 0.0);
                    self.goto(3);
                }
                3 => {
                    if !self.ja_wait() {
                        return None;
                    }
                    if swoop {
                        self.spawn_wings(WingsStart::Open);
                    }
                    self.chan
                        .set(anim::LIGHTJAK_GET_ON_LAND, NumFunc::seek(1.0), 0.0);
                    self.goto(4);
                }
                _ => {
                    if !self.code.arrived {
                        self.chan.eval(&self.anims);
                        self.code.arrived = true;
                        if self.chan.done(&self.anims) {
                            return Some(self.after_get_on());
                        }
                    }
                    if self.pad.stick0_speed != 0.0 && 31.0 < self.chan.aframe_num(&self.anims) {
                        return Some(self.after_get_on());
                    }
                    return None;
                }
            }
        }
    }

    /// The change back: on the ground, the change's animation backwards if
    /// it had only begun, else the change back's own.
    fn lightjak_get_off_code(&mut self) -> Option<State> {
        loop {
            match self.code.pc {
                0 => {
                    self.code.count = 0;
                    if self.chan.is(anim::LIGHTJAK_GET_ON) {
                        self.goto(2);
                    } else {
                        self.goto(1);
                    }
                }
                1 => match self.wait_to_land() {
                    Err(fall) => return Some(fall),
                    Ok(true) => self.goto(2),
                    Ok(false) => return None,
                },
                2 => {
                    self.control.transv = glam::Vec3::ZERO;
                    self.lightjak_end_mode(true);
                    if self.control.status & status::ON_WATER != 0 {
                        return Some(State::Stance);
                    }
                    if self.chan.is(anim::LIGHTJAK_GET_ON) {
                        self.chan.func = NumFunc::seek_to(0.0, 1.0);
                        self.goto(3);
                    } else {
                        self.ja_push(seconds(0.05));
                        self.ja_play(anim::LIGHTJAK_GET_OFF, 1.0);
                        self.goto(4);
                    }
                }
                3 => {
                    if self.chan.done(&self.anims) {
                        self.ja_push(seconds(0.1));
                        return Some(State::Stance);
                    }
                    if self.code.arrived {
                        let l = &mut self.lightjak;
                        l.interp = seek(l.interp, 0.0, 2.0 * SECONDS_PER_FRAME);
                        return None;
                    }
                    self.chan.eval(&self.anims);
                    self.code.arrived = true;
                }
                _ => {
                    if !self.code.arrived {
                        self.chan.eval(&self.anims);
                        self.code.arrived = true;
                        if self.chan.done(&self.anims) {
                            return Some(State::Stance);
                        }
                    }
                    let aframe = self.chan.aframe_num(&self.anims);
                    self.lightjak.interp = lerp_scale(1.0, 0.0, aframe, 10.0, 60.0);
                    if aframe >= 24.0 && self.pad.stick0_speed != 0.0 && self.lightjak.interp == 0.0
                    {
                        return Some(State::Stance);
                    }
                    return None;
                }
            }
        }
    }

    /// A flap: the vertical speed set to the flap's own, the horizontal
    /// kept. One kept going by X held only holds him up for a moment: he
    /// still comes down, gently.
    fn flap(&mut self) {
        let glide = self.lightjak.glide;
        let c = &mut self.control;
        let up = c.gravity_normal.dot(c.transv);
        let flat = c.transv - c.gravity_normal * up;
        let vertical = if glide { up.max(0.0).min(self.lightjak.swoop_impulse) } else { self.lightjak.swoop_impulse };
        c.transv = c.gravity_normal * vertical + flat;
    }

    /// The press after half a second in the air: another flap, as strong
    /// as the time since the last allows.
    fn again(&self) -> Option<State> {
        self.pad.pressed(button::X).then(
            || State::LightJakSwoop {
                first: false,
                held: (self.foot.sliding_start_time - self.state_time) as f32
                    / crate::TICKS_PER_SECOND as f32,
            },
        )
    }

    fn swoop_code(&mut self) -> Option<State> {
        if self.code.pc == 0 {
            self.ja_push(seconds(0.05));
            self.chan
                .set(anim::LIGHTJAK_SWOOP1, NumFunc::seek(1.0), 0.0);
            self.flap();
            self.goto(1);
            return None;
        }
        if self.code.arrived {
            return None;
        }
        self.chan.eval(&self.anims);
        self.code.arrived = true;
        if let Some(next) = self.again() {
            return Some(next);
        }
        if !self.chan.done(&self.anims) {
            return None;
        }
        self.held_flap().or(Some(State::LightJakSwoopFalling))
    }

    fn swoop_again_code(&mut self) -> Option<State> {
        loop {
            match self.code.pc {
                0 => {
                    let aframe = self.chan.aframe_num(&self.anims);
                    self.code.x = 44.0;
                    self.code.count = 6;
                    if self.chan.is(anim::LIGHTJAK_SWOOP1) {
                        self.code.x = lerp_scale(48.0, 44.0, aframe, 30.0, 44.0);
                        self.code.count = 30;
                    } else if self.chan.is(anim::LIGHTJAK_SWOOP2) {
                        self.code.x = lerp_scale(48.0, 44.0, aframe, 54.0, 74.0);
                        self.code.count = 30;
                    } else if self
                        .chan
                        .is_any(&[anim::LIGHTJAK_SWOOP_FALL, anim::LIGHTJAK_SWOOP_FALL_LOOP])
                    {
                        self.ja_push(seconds(0.2));
                        let from = self.aframe(anim::LIGHTJAK_SWOOP2, 69.0);
                        self.chan
                            .set(anim::LIGHTJAK_SWOOP2, NumFunc::seek(2.0), from);
                        self.goto(1);
                        continue;
                    }
                    self.goto(2);
                }
                1 => {
                    if !self.code.arrived {
                        self.chan.eval(&self.anims);
                        self.code.arrived = true;
                        if self.chan.done(&self.anims) {
                            self.code.count = 0;
                            self.goto(2);
                            continue;
                        }
                    }
                    let c = &mut self.control;
                    let up = c.gravity_normal.dot(c.transv);
                    let flat = c.transv - c.gravity_normal * up;
                    c.transv = c.gravity_normal * (0.95 * up) + flat;
                    return None;
                }
                2 => {
                    self.ja_push(i64::from(self.code.count));
                    let from = self.aframe(anim::LIGHTJAK_SWOOP2, self.code.x);
                    self.chan
                        .set(anim::LIGHTJAK_SWOOP2, NumFunc::seek(1.0), from);
                    self.goto(3);
                }
                3 => {
                    if self.chan.aframe_num(&self.anims) >= 48.0 {
                        self.flap();
                        self.goto(4);
                        return None;
                    }
                    if self.code.arrived {
                        return None;
                    }
                    self.chan.eval(&self.anims);
                    self.code.arrived = true;
                }
                _ => {
                    if self.code.arrived {
                        return None;
                    }
                    self.chan.eval(&self.anims);
                    self.code.arrived = true;
                    if self.chan.aframe_num(&self.anims) >= 54.0
                        && let Some(next) = self.again()
                    {
                        return Some(next);
                    }
                    if !self.chan.done(&self.anims) {
                        return None;
                    }
                    return self.held_flap().or(Some(State::LightJakSwoopFalling));
                }
            }
        }
    }

    fn swoop_falling_code(&mut self) -> Option<State> {
        loop {
            match self.code.pc {
                0 => {
                    self.ja_push(seconds(0.1));
                    self.ja_play(anim::LIGHTJAK_SWOOP_FALL, 1.0);
                    self.goto(1);
                }
                1 => {
                    if !self.ja_wait() {
                        return None;
                    }
                    self.goto(2);
                }
                2 => {
                    self.ja_play(anim::LIGHTJAK_SWOOP_FALL_LOOP, 1.0);
                    self.goto(3);
                }
                _ => {
                    if !self.ja_wait() {
                        return None;
                    }
                    self.goto(2);
                }
            }
        }
    }

    /// Landing out of a flight plays the swoop's landing.
    pub(crate) fn landing_from_swoop(&self) -> bool {
        self.lightjak.on && self.chan.is_any(&SWOOP_ANIMS)
    }

    /// The animation standing plays as Light Jak, and whether it comes in
    /// through the stance change.
    pub(crate) fn light_stance(&self) -> Option<bool> {
        let l = &self.lightjak;
        (l.on && l.latch_out_time == 0).then(|| {
            !self
                .chan
                .is_any(&[anim::LIGHTJAK_SWOOP_LAND, anim::LIGHTJAK_GET_ON_LAND])
        })
    }
}

#[cfg(test)]
mod time_tests {
    use super::*;

    #[test]
    fn the_freeze_slows_the_world_to_a_twentieth_and_back() {
        let mut jak = Jak::new(crate::glam::Vec3::ZERO, 0.0);
        assert_eq!(jak.world_time_scale(), 1.0);
        jak.lightjak.freeze_until = jak.time + seconds(15.0);
        assert!((jak.world_time_scale() - 1.0).abs() < 1e-3);
        jak.time += seconds(0.5);
        assert!((jak.world_time_scale() - 0.5).abs() < 0.02);
        jak.time += seconds(2.0);
        assert!((jak.world_time_scale() - 0.05).abs() < 1e-4);
        jak.time += seconds(13.7);
        assert!(jak.world_time_scale() > 0.05 && jak.world_time_scale() < 1.0);
        jak.time += seconds(1.0);
        assert_eq!(jak.world_time_scale(), 1.0);
    }
}
