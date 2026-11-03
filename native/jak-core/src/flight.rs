//! Open flight, as the universe's ship flew (`crates/universe` moved its
//! math here so the HellCat and the universe share one rule): pure
//! functions in meters and seconds, no world up assumed.
use glam::{DVec3, Quat};

fn smoothstep(edge0: f64, edge1: f64, x: f64) -> f64 {
    let t = ((x - edge0) / (edge1 - edge0)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// The top speed at `clearance` meters over the ground, m/s: it grows with
/// the height (about a third of it a second, so coming down takes its
/// time), up to supercruise far out in space.
pub fn cruise_speed(clearance: f64) -> f64 {
    (40.0 + clearance.max(0.0) * 0.35).min(30_000.0)
}

/// The turbo between planets: only a little help low down, three times the
/// speed once well clear of the ground.
pub fn boost_factor(clearance: f64) -> f64 {
    1.0 + 2.0 * smoothstep(800.0, 4_000.0, clearance)
}

/// The velocity one step of `dt` seconds later, heading for `target`: the
/// speed changes by at most a fraction of itself a second (plus a little to
/// get going), so it never jolts.
pub fn limit_accel(vel: DVec3, target: DVec3, dt: f64) -> DVec3 {
    let change = target - vel;
    let most = (30.0 + vel.length() * 0.7) * dt;
    let length = change.length();
    if length > most {
        vel + change * (most / length)
    } else {
        target
    }
}

/// The chase camera behind and above a craft of `size` meters at `pos`
/// turned by `rot` (its forward -Z, up +Y), `zoom` times out: from the
/// craft's own axes, so it rolls and loops with it.
pub fn chase_camera(pos: DVec3, rot: Quat, size: f64, zoom: f64) -> (DVec3, Quat) {
    let back = (rot * glam::Vec3::Z).as_dvec3();
    let up = (rot * glam::Vec3::Y).as_dvec3();
    (
        pos + back * (size * 1.7 * zoom) + up * (size * 0.5 * zoom),
        rot * Quat::from_rotation_x(-0.12),
    )
}

/// The craft's turn from the stick, with no world up: yaw and pitch from
/// the stick or mouse, roll from its own input, all about its own axes.
pub fn turn(rot: Quat, yaw: f32, pitch: f32, roll: f32) -> Quat {
    (rot * Quat::from_rotation_y(yaw) * Quat::from_rotation_x(pitch) * Quat::from_rotation_z(roll))
        .normalize()
}

/// How a camera blends from level with the world (`level`) to the craft's
/// own frame (`craft`) as `air` goes from 0 to 1.
pub fn blend_view(level: Quat, craft: Quat, air: f32) -> Quat {
    level.slerp(craft, air.clamp(0.0, 1.0))
}
