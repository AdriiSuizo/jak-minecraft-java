//! The follow camera's zoom, shared by every host: L1 with the D-pad (or
//! the right stick) up brings it in, down takes it out, from twice Jak 3's
//! usual distance down to first person. Hosts keep the value and scale
//! their camera's distance by [`Zoom::factor`].
use crate::pad::{PadInput, button};

/// How far back the camera sits, meters at Jak's scale.
pub const DEFAULT: f32 = 6.0;
pub const MAX: f32 = 12.0;
/// Nearer than this it is in Jak's eyes.
pub const FIRST_PERSON: f32 = 0.6;
/// Meters a second while held.
const RATE: f32 = 6.0;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Zoom {
    pub distance: f32,
}

impl Default for Zoom {
    fn default() -> Self {
        Self { distance: DEFAULT }
    }
}

impl Zoom {
    /// One frame of the controls. Returns the buttons the zoom used (L1
    /// and the D-pad's up and down), for the host to keep from the game.
    pub fn update(&mut self, input: &PadInput, dt: f32) -> u32 {
        if input.held & button::L1 == 0 {
            return 0;
        }
        let dpad = if input.held & button::UP != 0 {
            -1.0
        } else if input.held & button::DOWN != 0 {
            1.0
        } else {
            0.0
        };
        let stick = if input.right[1].abs() > 0.25 { -input.right[1] } else { 0.0 };
        let way = if dpad != 0.0 { dpad } else { stick };
        if way == 0.0 {
            return 0;
        }
        self.distance = self.distance.max(FIRST_PERSON) + way * RATE * dt;
        if self.distance < FIRST_PERSON {
            self.distance = 0.0;
        }
        self.distance = self.distance.min(MAX);
        button::L1 | button::UP | button::DOWN
    }

    /// The camera's distance against the usual one: 1 as it starts, up to
    /// 2 zoomed out, 0 in first person. A vehicle's chase camera scales by
    /// it too.
    pub fn factor(&self) -> f32 {
        self.distance / DEFAULT
    }

    pub fn first_person(&self) -> bool {
        self.distance < FIRST_PERSON
    }
}

/// The shoulder modifier zooms with the D-pad; neither stick changes distance.
pub fn shoulder_zoom(distance: f32, held: u32, dt: f32) -> Option<f32> {
    if held & button::R1 == 0 || !dt.is_finite() || dt <= 0.0 {
        return None;
    }
    let way = match (held & button::UP != 0, held & button::DOWN != 0) {
        (true, false) => -1.0,
        (false, true) => 1.0,
        _ => return None,
    };
    let next = distance.max(FIRST_PERSON) + way * RATE * dt;
    Some(if next < FIRST_PERSON {
        0.0
    } else {
        next.min(MAX)
    })
}
