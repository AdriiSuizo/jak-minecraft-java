use crate::{Event, Jak, PadInput, Push, State, anim, anim::NumFunc, collide::CollideWorld};

/// `darkjak-punch-inc`: what each dark punch of a combo spends (times
/// [`crate::lightjak::ECO_COST`], Adrii 22/10).
pub(crate) const DARK_PUNCH: f32 = 4.0;

impl Jak {
    /// Dark eco spent (`get-pickup eco-pill-dark` of minus `amount`).
    pub(crate) fn spend_dark(&mut self, amount: f32) {
        self.dark_eco = (self.dark_eco - amount * crate::lightjak::ECO_COST).max(0.0);
    }

    /// `target-darkjak-process`: with under a point of dark eco left, back
    /// to Jak (here a point is [`crate::lightjak::ECO_COST`]).
    pub(crate) fn dark_eco_frame(&mut self) {
        if self.dark && self.dark_transition.is_none() && self.dark_eco < crate::lightjak::ECO_COST && self.attack.danger.is_none() {
            self.request_dark(false);
        }
    }

    pub fn request_dark(&mut self, on: bool) {
        if self.dark_transition.is_some() || self.dark == on || self.state.is_board() || (on && self.powers_blocked) {
            return;
        }
        // `want-to-darkjak?`: only with dark eco.
        if on && self.dark_eco <= 0.0 {
            return;
        }
        self.lightjak_end_mode(true);
        self.gun.end_mode();
        // A board called for before (R2 pressed while it could not come)
        // never comes once he changes back (Adrii 17/10).
        self.board.latch = false;
        self.go(State::Stance);
        self.pending = None;
        self.danger_set(None);
        self.dark_transition = Some(on);
        if on {
            self.dark = true;
        }
        self.push = Push {
            tick: self.time,
            ticks: crate::math::seconds(if on { 0.1 } else { 0.05 }),
        };
        self.chan.set(
            if on {
                anim::DARKJAK_GET_ON
            } else {
                anim::DARKJAK_GET_OFF
            },
            NumFunc::seek(1.0),
            0.0,
        );
        self.events.push(Event::DarkJak { on });
    }

    pub(crate) fn dark_transition_step(&mut self, world: &mut dyn CollideWorld) {
        let Some(on) = self.dark_transition else {
            return;
        };
        self.pad.update(&PadInput::default());
        if self.control.on_surface() {
            self.control.transv = crate::glam::Vec3::ZERO;
        }
        self.chan.eval(&self.anims);
        self.target_post(world);
        self.pending = None;
        if self.chan.done(&self.anims) {
            if !on && self.chan.is(anim::DARKJAK_GET_OFF) {
                self.chan
                    .set(anim::DARKJAK_GET_OFF_END, NumFunc::seek(1.0), 0.0);
            } else {
                self.dark = on;
                self.dark_transition = None;
                self.board.latch = false;
                self.go(if self.control.on_surface() {
                    State::Stance
                } else {
                    State::Falling { uppercut: false }
                });
            }
        }
    }
}

#[cfg(test)]
mod riding_tests {
    use crate::{Jak, glam::Vec3};

    #[test]
    fn no_dark_jak_at_a_vehicles_controls() {
        let mut jak = Jak::new(Vec3::ZERO, 0.0);
        jak.powers_blocked = true;
        jak.dark_eco = 100.0;
        jak.request_dark(true);
        assert!(!jak.dark && jak.dark_transition.is_none(), "driving: no Dark Jak");
        assert!(!jak.want_to_powerjak(), "nor Light Jak");
        // On foot again: both as ever.
        jak.powers_blocked = false;
        jak.dark_eco = 100.0;
        jak.request_dark(true);
        assert!(jak.dark);
    }

    /// Jak 3's `want-to-darkjak?`: no Dark Jak without dark eco; a dark
    /// punch spends `darkjak-punch-inc` (a tenth of it, Adrii 22/10); under
    /// a point he changes back.
    #[test]
    fn dark_jak_runs_on_dark_eco() {
        let mut jak = Jak::new(Vec3::ZERO, 0.0);
        jak.request_dark(true);
        assert!(!jak.dark, "none without dark eco");
        jak.dark_eco = 1.0;
        jak.request_dark(true);
        assert!(jak.dark);
        jak.dark_transition = None;
        jak.spend_dark(super::DARK_PUNCH);
        assert!((jak.dark_eco - 0.6).abs() < 1e-5);
        jak.dark_eco = 0.05;
        jak.dark_eco_frame();
        assert_eq!(jak.dark_transition, Some(false), "changes back with under a point");
    }
}
