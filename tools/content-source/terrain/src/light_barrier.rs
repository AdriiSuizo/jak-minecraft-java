//! Contact volumes supplied by a host's Light Jak shields, in block space.
#[derive(Clone, Copy, Debug)]
pub struct Barrier {
    pub client: u32,
    pub feet: [f64; 3],
    pub radius: f64,
    pub height: f64,
}

pub fn resolve(
    b: Barrier,
    feet: [f64; 3],
    radius: f64,
    height: f64,
    velocity: [f64; 3],
) -> Option<([f64; 3], [f64; 3])> {
    if feet[1] >= b.feet[1] + b.height || feet[1] + height <= b.feet[1] {
        return None;
    }
    let (dx, dz) = (feet[0] - b.feet[0], feet[2] - b.feet[2]);
    let d = dx.hypot(dz);
    let reach = radius + b.radius;
    if !d.is_finite() || d >= reach {
        return None;
    }
    let (nx, nz) = if d > 1e-6 {
        (dx / d, dz / d)
    } else {
        (1.0, 0.0)
    };
    let inward = (velocity[0] * nx + velocity[2] * nz).min(0.0);
    Some((
        [
            feet[0] + nx * (reach - d),
            feet[1],
            feet[2] + nz * (reach - d),
        ],
        [
            velocity[0] - nx * inward,
            velocity[1],
            velocity[2] - nz * inward,
        ],
    ))
}

/// Resolve shield contact through the same block-shape collision as mob
/// movement. Raising the shield beside a wall must not teleport a mob into it.
pub fn resolve_body(
    world: &impl minecraftoss_player::World,
    barrier: Barrier,
    body: &mut minecraftoss_entities::movement::Body,
) -> bool {
    let Some((at, velocity)) = resolve(
        barrier,
        body.position.to_array(),
        f64::from(body.width) * 0.5,
        f64::from(body.height),
        body.velocity.to_array(),
    ) else {
        return false;
    };
    let wanted = glam::DVec3::from_array(at) - body.position;
    let bounds =
        minecraftoss_player::collision::Bounds::standing(body.position, body.width, body.height);
    body.position +=
        minecraftoss_player::collision::move_body(world, bounds, wanted, body.on_ground, 0.0);
    body.velocity = glam::DVec3::from_array(velocity);
    body.needs_sync = true;
    true
}
