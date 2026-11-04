//! Pinned 26.3 ArrowModel/ArrowRenderer: two crossed fins and the back,
//! original 32×32 projectile skins, flight rotation and seven-tick shake.
use crate::{cow_render::cube_tinted_pose, lighting::SkyLight, mesh::{Atlas, ChunkMesh}, pack::ResourceId};
use glam::{DVec3, Quat, Vec3};

pub const TEXTURES: [&str; 3] = ["minecraft:entity/projectiles/arrow", "minecraft:entity/projectiles/arrow_spectral", "minecraft:entity/projectiles/arrow_tipped"];

#[derive(Clone, Debug)]
pub struct ArrowView {
    pub item: String,
    pub previous: DVec3,
    pub position: DVec3,
    /// Retained after impact: the native physical arrow stops its velocity.
    pub direction: DVec3,
    pub shake: f32,
}

fn rotation(direction: DVec3, shake: f32) -> Quat {
    let d = direction.normalize_or(DVec3::X);
    let yaw = d.x.atan2(d.z) as f32 - std::f32::consts::FRAC_PI_2;
    let pitch = d.y.atan2(d.x.hypot(d.z)) as f32;
    let wobble = if shake > 0.0 { -(shake * 3.0).sin() * shake } else { 0.0 };
    Quat::from_rotation_y(yaw) * Quat::from_rotation_z(pitch + wobble.to_radians())
}

fn append_model(mesh: &mut ChunkMesh, at: DVec3, direction: DVec3, shake: f32, region: [f32; 4], sky: f32, block: f32) {
    let start = mesh.vertices.len();
    let cross = std::f32::consts::FRAC_PI_4;
    // ArrowModel.createBodyLayer: root scale .9; back child scale .8;
    // the cross's texScaleV=.8 changes its effective sheet height to 25.6.
    for (from, to, pivot, angle, texture) in [
        ([0., -2., -2.], [0., 2., 2.], [-11., 0., 0.], cross, [32., 32.]),
        ([-12., -2., 0.], [4., 2., 0.], [0.; 3], cross, [32., 25.6]),
        ([-12., -2., 0.], [4., 2., 0.], [0.; 3], cross * 3., [32., 25.6]),
    ] {
        cube_tinted_pose(mesh, DVec3::ZERO, Quat::IDENTITY, 0.9, region, sky, block,
            from, to, [0.; 2], pivot, Quat::from_rotation_x(angle), [1.; 3], texture, None);
    }
    let pose = rotation(direction, shake);
    for vertex in &mut mesh.vertices[start..] {
        // Reuse ModelPart's exact UVs, undo the living renderer's feet offset
        // and axis flips: ArrowRenderer renders directly at the projectile.
        let p = Vec3::from_array(vertex.position);
        let local = Vec3::new(-p.x, 24.016 * 0.9 / 16.0 - p.y, p.z);
        vertex.position = (at.as_vec3() + pose * local).to_array();
    }
}

pub fn append_arrows<'a>(mesh: &mut ChunkMesh, arrows: impl IntoIterator<Item=&'a ArrowView>, atlas: &Atlas, light: &SkyLight, partial: f32) {
    let textures = TEXTURES.map(|id| ResourceId::parse(id).unwrap());
    for arrow in arrows {
        let skin = &textures[match arrow.item.as_str() { "minecraft:spectral_arrow" => 1, "minecraft:tipped_arrow" => 2, _ => 0 }];
        if !atlas.contains(skin) { continue; }
        let at = arrow.previous.lerp(arrow.position, f64::from(partial));
        let cell = (at.x.floor() as i32, at.y.floor() as i32, at.z.floor() as i32);
        append_model(mesh, at, arrow.direction, arrow.shake - partial, atlas.entity_region(skin), light.get(cell) as f32, light.get_block(cell) as f32);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn original_crossed_arrow_is_pointed_along_its_flight_and_keeps_pixel_uvs() {
        for direction in [DVec3::X, DVec3::NEG_Z, DVec3::new(1., 1., 1.).normalize()] {
            let pose = rotation(direction, 0.);
            assert!((pose * Vec3::X - direction.as_vec3()).length() < 1e-5);
            let at = DVec3::new(8., 70., -5.);
            let mut mesh = ChunkMesh::default();
            append_model(&mut mesh, at, direction, 0., [0., 0., 1., 1.], 15., 0.);
            assert_eq!(mesh.vertices.len(), 72);
            let axial: Vec<_> = mesh.vertices.iter().map(|v| (Vec3::from_array(v.position) - at.as_vec3()).dot(direction.as_vec3())).collect();
            assert!((axial.iter().copied().fold(f32::INFINITY, f32::min) + 12. * 0.9 / 16.).abs() < 1e-5);
            assert!((axial.iter().copied().fold(f32::NEG_INFINITY, f32::max) - 4. * 0.9 / 16.).abs() < 1e-5);
            assert!(mesh.vertices.iter().all(|v| v.uv.iter().all(|c| (0.0..=1.0).contains(c))));
            assert!(mesh.vertices.iter().any(|v| (v.uv[1] - 4. / 25.6).abs() < 1e-5));
        }
        assert_ne!(rotation(DVec3::X, 6.), rotation(DVec3::X, 0.));
    }
}
