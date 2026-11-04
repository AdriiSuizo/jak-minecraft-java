//! The end game's creatures as entity meshes: blazes (`BlazeModel`), the
//! Ender Dragon (`DragonModel` posed from its flight history as
//! `EnderDragonRenderer` does), End crystals (`EndCrystalModel`) with the
//! beam that heals the dragon, and flat sprites for thrown Eyes of Ender,
//! fireballs and dragon's breath. Each model is built with the renderer's
//! own matrix stack (`PoseStack`), from model pixels to world blocks.
use crate::cow_render::entity_shade;
use crate::mesh::{Atlas, ChunkMesh, Vertex};
use crate::pack::ResourceId;
use glam::{DVec3, Mat4, Quat, Vec2, Vec3};

pub const BLAZE: &str = "minecraft:entity/blaze/blaze";
pub const DRAGON: &str = "minecraft:entity/enderdragon/dragon";
pub const DRAGON_EYES: &str = "minecraft:entity/enderdragon/dragon_eyes";
pub const CRYSTAL: &str = "minecraft:entity/end_crystal/end_crystal";
pub const CRYSTAL_BEAM: &str = "minecraft:entity/end_crystal/end_crystal_beam";
pub const EYE: &str = "minecraft:item/ender_eye";
pub const FIRE_CHARGE: &str = "minecraft:item/fire_charge";
pub const DRAGON_FIREBALL: &str = "minecraft:entity/enderdragon/dragon_fireball";
pub const BREATH: &str = "minecraft:particle/generic_5";

/// Every texture these meshes sample, for the atlas.
pub const TEXTURES: [&str; 9] = [
    BLAZE,
    DRAGON,
    DRAGON_EYES,
    CRYSTAL,
    CRYSTAL_BEAM,
    EYE,
    FIRE_CHARGE,
    DRAGON_FIREBALL,
    BREATH,
];

/// Light as the GPU wants it: sky and block levels 0..15.
#[derive(Clone, Copy)]
pub struct Light {
    pub sky: f32,
    pub block: f32,
}

impl Light {
    pub const FULL: Light = Light {
        sky: 15.0,
        block: 15.0,
    };
}

/// A model cuboid under a pose (model pixels to world blocks), with
/// `ModelPart.Cube`'s face order and UV layout (as `cow_render` lays them).
#[allow(clippy::too_many_arguments)]
fn emit_box(
    mesh: &mut ChunkMesh,
    pose: &Mat4,
    region: [f32; 4],
    texture_size: [f32; 2],
    from: [f32; 3],
    to: [f32; 3],
    uv: [f32; 2],
    mirror: bool,
    light: Light,
    color: [f32; 4],
) {
    let [x0, y0, z0] = from;
    let [x1, y1, z1] = to;
    let (x_lo, x_hi) = if mirror { (x1, x0) } else { (x0, x1) };
    let (w, h, d) = (x1 - x0, y1 - y0, z1 - z0);
    let (u, v) = (uv[0], uv[1]);
    let u0 = u;
    let u1 = u + d;
    let u2 = u1 + w;
    let u22 = u2 + w;
    let u3 = u2 + d;
    let u4 = u3 + w;
    let v0 = v;
    let v1 = v + d;
    let v2 = v1 + h;
    let faces = [
        (
            [[x_hi, y0, z1], [x_lo, y0, z1], [x_lo, y0, z0], [x_hi, y0, z0]],
            [u1, v0, u2, v1],
            [0.0, -1.0, 0.0],
        ),
        (
            [[x_hi, y1, z0], [x_lo, y1, z0], [x_lo, y1, z1], [x_hi, y1, z1]],
            [u2, v1, u22, v0],
            [0.0, 1.0, 0.0],
        ),
        (
            [[x_lo, y0, z0], [x_lo, y0, z1], [x_lo, y1, z1], [x_lo, y1, z0]],
            [u0, v1, u1, v2],
            [if mirror { 1.0 } else { -1.0 }, 0.0, 0.0],
        ),
        (
            [[x_hi, y0, z0], [x_lo, y0, z0], [x_lo, y1, z0], [x_hi, y1, z0]],
            [u1, v1, u2, v2],
            [0.0, 0.0, -1.0],
        ),
        (
            [[x_hi, y0, z1], [x_hi, y0, z0], [x_hi, y1, z0], [x_hi, y1, z1]],
            [u2, v1, u3, v2],
            [if mirror { -1.0 } else { 1.0 }, 0.0, 0.0],
        ),
        (
            [[x_lo, y0, z1], [x_hi, y0, z1], [x_hi, y1, z1], [x_lo, y1, z1]],
            [u3, v1, u4, v2],
            [0.0, 0.0, 1.0],
        ),
    ];
    // A flipped pose (the renderers' scale(-1, -1, 1)) keeps its faces
    // facing out only with the winding reversed.
    let flipped = pose.determinant() < 0.0;
    for (corners, tex, normal) in faces {
        let n = pose.transform_vector3(Vec3::from_array(normal));
        let n = if flipped { -n } else { n }.normalize_or_zero();
        let shade = entity_shade(n);
        let start = mesh.vertices.len() as u32;
        let mut vertices = std::array::from_fn::<_, 4, _>(|i| {
            (
                corners[i],
                [
                    [tex[2], tex[1]],
                    [tex[0], tex[1]],
                    [tex[0], tex[3]],
                    [tex[2], tex[3]],
                ][i],
            )
        });
        if mirror {
            vertices.reverse();
        }
        for (corner, coord) in vertices {
            let world = pose.transform_point3(Vec3::from_array(corner));
            mesh.vertices.push(Vertex {
                position: world.to_array(),
                uv: [
                    region[0] + (region[2] - region[0]) * coord[0] / texture_size[0],
                    region[1] + (region[3] - region[1]) * coord[1] / texture_size[1],
                ],
                color: [shade * color[0], shade * color[1], shade * color[2], color[3]],
                sky_light: light.sky,
                block_light: light.block,
            });
        }
        if flipped {
            mesh.indices
                .extend_from_slice(&[start, start + 2, start + 1, start, start + 3, start + 2]);
        } else {
            mesh.indices
                .extend_from_slice(&[start, start + 1, start + 2, start, start + 2, start + 3]);
        }
        mesh.faces += 1;
    }
}

/// `LivingEntityRenderer.setupRotations` and the model flip: from model
/// pixels to world blocks for a mob at `feet` turned to `body_yaw`.
fn living_pose(feet: DVec3, body_yaw: f32) -> Mat4 {
    Mat4::from_translation(feet.as_vec3())
        * Mat4::from_rotation_y((180.0 - body_yaw).to_radians())
        * Mat4::from_scale(Vec3::new(-1.0, -1.0, 1.0))
        * Mat4::from_translation(Vec3::new(0.0, -1.501, 0.0))
        * Mat4::from_scale(Vec3::splat(1.0 / 16.0))
}

/// `ModelPart.translateAndRotate`: the part's offset, then Z, Y, X.
fn part(offset: Vec3, rotation: Vec3) -> Mat4 {
    Mat4::from_translation(offset)
        * Mat4::from_quat(Quat::from_euler(
            glam::EulerRot::ZYX,
            rotation.z,
            rotation.y,
            rotation.x,
        ))
}

/// The overlay alpha: the hurt flash's red, or none.
fn overlay(hurt: bool) -> f32 {
    if hurt {
        -crate::client_mobs::RED_OVERLAY_ALPHA
    } else {
        1.0
    }
}

/// A blaze as it is drawn this frame.
pub struct BlazePose {
    pub feet: DVec3,
    /// Body and head yaw, head pitch, in degrees.
    pub body_yaw: f32,
    pub head_yaw: f32,
    pub head_pitch: f32,
    /// Ticks lived, with the partial tick.
    pub age: f32,
    pub hurt: bool,
}

/// `BlazeModel` posed by `setupAnim`; blazes are drawn at full brightness
/// (`BlazeRenderer.getBlockLightLevel`).
pub fn append_blazes(mesh: &mut ChunkMesh, blazes: &[BlazePose], atlas: &Atlas) {
    let Ok(id) = ResourceId::parse(BLAZE) else {
        return;
    };
    if !atlas.contains(&id) {
        return;
    }
    let region = atlas.entity_region(&id);
    let tex = [64.0, 32.0];
    for blaze in blazes {
        let root = living_pose(blaze.feet, blaze.body_yaw);
        let color = [1.0, 1.0, 1.0, overlay(blaze.hurt)];
        let head = root
            * part(
                Vec3::ZERO,
                Vec3::new(
                    blaze.head_pitch.to_radians(),
                    (blaze.head_yaw - blaze.body_yaw).to_radians(),
                    0.0,
                ),
            );
        emit_box(
            mesh,
            &head,
            region,
            tex,
            [-4.0, -4.0, -4.0],
            [4.0, 4.0, 4.0],
            [0.0, 0.0],
            false,
            Light::FULL,
            color,
        );
        let age = blaze.age;
        let pi = std::f32::consts::PI;
        let mut rods = [Vec3::ZERO; 12];
        let mut f = age * pi * -0.1;
        for (i, rod) in rods.iter_mut().enumerate().take(4) {
            *rod = Vec3::new(
                f.cos() * 9.0,
                -2.0 + ((i as f32 * 2.0 + age) * 0.25).cos(),
                f.sin() * 9.0,
            );
            f += pi / 2.0;
        }
        f = pi / 4.0 + age * pi * 0.03;
        for (i, rod) in rods.iter_mut().enumerate().skip(4).take(4) {
            *rod = Vec3::new(
                f.cos() * 7.0,
                2.0 + ((i as f32 * 2.0 + age) * 0.25).cos(),
                f.sin() * 7.0,
            );
            f += pi / 2.0;
        }
        f = 0.471_238_94 + age * pi * -0.05;
        for (i, rod) in rods.iter_mut().enumerate().skip(8) {
            *rod = Vec3::new(
                f.cos() * 5.0,
                11.0 + ((i as f32 * 1.5 + age) * 0.5).cos(),
                f.sin() * 5.0,
            );
            f += pi / 2.0;
        }
        for rod in rods {
            emit_box(
                mesh,
                &(root * Mat4::from_translation(rod)),
                region,
                tex,
                [0.0, 0.0, 0.0],
                [2.0, 8.0, 2.0],
                [0.0, 16.0],
                false,
                Light::FULL,
                color,
            );
        }
    }
}

/// One tick of the dragon's flight history (`EnderDragon.positions`): its
/// yaw in degrees and its height.
#[derive(Clone, Copy, Default, Debug)]
pub struct FlightSample {
    pub yaw: f32,
    pub y: f32,
}

/// The dragon as it is drawn this frame.
pub struct DragonPose<'a> {
    pub position: DVec3,
    /// `EnderDragon.getLatencyPos(i)`: 0 the newest sample, older after.
    pub history: &'a dyn Fn(usize) -> FlightSample,
    /// `oFlapTime`..`flapTime`, interpolated.
    pub flap: f32,
    /// The neck and head bend as `getHeadPartYOffset` gives it, by part
    /// index (0..=6): sitting, the parts' indices; flying, the height
    /// differences.
    pub head_part_offset: &'a dyn Fn(usize, FlightSample, FlightSample) -> f32,
    pub hurt: bool,
    /// Ticks into its death, 0 alive.
    pub dying: f32,
}

fn wrap_degrees(v: f32) -> f32 {
    let mut v = v % 360.0;
    if v >= 180.0 {
        v -= 360.0;
    }
    if v < -180.0 {
        v += 360.0;
    }
    v
}

/// `EnderDragonRenderer` and `DragonModel` (the classic layout): the body
/// rolled with the turn, two wings and four legs, five neck plates leading
/// to the head and twelve tail plates, each following where the dragon flew
/// a few ticks before.
pub fn append_dragon(mesh: &mut ChunkMesh, dragon: &DragonPose, atlas: &Atlas, light: Light) {
    let Ok(id) = ResourceId::parse(DRAGON) else {
        return;
    };
    if !atlas.contains(&id) {
        return;
    }
    let region = atlas.entity_region(&id);
    let tex = [256.0, 256.0];
    let color = [1.0, 1.0, 1.0, overlay(dragon.hurt)];
    let history = dragon.history;
    let deg = std::f32::consts::PI / 180.0;
    // `EnderDragonRenderer.setupRotations`.
    let yaw7 = history(7).yaw;
    let tilt = history(5).y - history(10).y;
    let mut entity = Mat4::from_translation(dragon.position.as_vec3())
        * Mat4::from_rotation_y(-yaw7 * deg)
        * Mat4::from_rotation_x(tilt * 10.0 * deg)
        * Mat4::from_translation(Vec3::new(0.0, 0.0, 1.0));
    if dragon.dying > 0.0 {
        // Dying, it shakes: `deathTime` jitter in the death animation.
        let shake = (dragon.dying * 0.37).sin() * 0.05;
        entity *= Mat4::from_translation(Vec3::new(shake, 0.0, -shake));
    }
    let entity = entity
        * Mat4::from_scale(Vec3::new(-1.0, -1.0, 1.0))
        * Mat4::from_translation(Vec3::new(0.0, -1.501, 0.0));
    let f = dragon.flap;
    let f8 = f * std::f32::consts::TAU;
    let jaw_x = ((f8).sin() + 1.0) * 0.2;
    let mut f1 = (f8 - 1.0).sin() + 1.0;
    f1 = (f1 * f1 + f1 * 2.0) * 0.05;
    // The model's own lift and bob, in blocks before the pixel scale.
    let model = entity
        * Mat4::from_translation(Vec3::new(0.0, f1 - 2.0, -3.0))
        * Mat4::from_rotation_x(f1 * 2.0 * deg)
        * Mat4::from_scale(Vec3::splat(1.0 / 16.0));
    let s6 = history(6);
    let f6 = wrap_degrees(history(5).yaw - history(10).yaw);
    let f7 = wrap_degrees(history(5).yaw + f6 / 2.0);
    let neck_box = |mesh: &mut ChunkMesh, pose: &Mat4| {
        emit_box(mesh, pose, region, tex, [-5.0, -5.0, -5.0], [5.0, 5.0, 5.0], [192.0, 104.0], false, light, color);
        emit_box(mesh, pose, region, tex, [-1.0, -9.0, -3.0], [1.0, -5.0, 3.0], [48.0, 0.0], false, light, color);
    };
    // The neck.
    let (mut x, mut y, mut z) = (0.0f32, 20.0f32, -12.0f32);
    for i in 0..5 {
        let s = history(5 - i);
        let wave = ((i as f32) * 0.45 + f8).cos() * 0.15;
        let ry = wrap_degrees(s.yaw - s6.yaw) * deg * 1.5;
        let rx = wave + (dragon.head_part_offset)(i, s6, s) * deg * 1.5 * 5.0;
        let rz = -wrap_degrees(s.yaw - f7) * deg * 1.5;
        neck_box(mesh, &(model * part(Vec3::new(x, y, z), Vec3::new(rx, ry, rz))));
        y += rx.sin() * 10.0;
        z -= ry.cos() * rx.cos() * 10.0;
        x -= ry.sin() * rx.cos() * 10.0;
    }
    // The head and jaw.
    let s0 = history(0);
    let head_rot = Vec3::new(
        wrap_degrees((dragon.head_part_offset)(6, s6, s0)) * deg * 1.5 * 5.0,
        wrap_degrees(s0.yaw - s6.yaw) * deg,
        -wrap_degrees(s0.yaw - f7) * deg,
    );
    let head = model * part(Vec3::new(x, y, z), head_rot);
    let fz = -16.0;
    emit_box(mesh, &head, region, tex, [-6.0, -1.0, -8.0 + fz], [6.0, 4.0, 8.0 + fz], [176.0, 44.0], false, light, color);
    emit_box(mesh, &head, region, tex, [-8.0, -8.0, 6.0 + fz], [8.0, 8.0, 22.0 + fz], [112.0, 30.0], false, light, color);
    emit_box(mesh, &head, region, tex, [-5.0, -12.0, 12.0 + fz], [-3.0, -8.0, 18.0 + fz], [0.0, 0.0], true, light, color);
    emit_box(mesh, &head, region, tex, [-5.0, -3.0, -6.0 + fz], [-3.0, -1.0, -2.0 + fz], [112.0, 0.0], true, light, color);
    emit_box(mesh, &head, region, tex, [3.0, -12.0, 12.0 + fz], [5.0, -8.0, 18.0 + fz], [0.0, 0.0], false, light, color);
    emit_box(mesh, &head, region, tex, [3.0, -3.0, -6.0 + fz], [5.0, -1.0, -2.0 + fz], [112.0, 0.0], false, light, color);
    let jaw = head * part(Vec3::new(0.0, 4.0, 8.0 + fz), Vec3::new(jaw_x, 0.0, 0.0));
    emit_box(mesh, &jaw, region, tex, [-6.0, 0.0, -16.0], [6.0, 4.0, 0.0], [176.0, 65.0], false, light, color);
    // The body, rolled into the turn about a point a block up.
    let group = model
        * Mat4::from_translation(Vec3::new(0.0, 16.0, 0.0))
        * Mat4::from_rotation_z(-f6 * 1.5 * deg)
        * Mat4::from_translation(Vec3::new(0.0, -16.0, 0.0));
    let body = group * part(Vec3::new(0.0, 4.0, 8.0), Vec3::ZERO);
    emit_box(mesh, &body, region, tex, [-12.0, 0.0, -16.0], [12.0, 24.0, 48.0], [0.0, 0.0], false, light, color);
    for z0 in [-10.0, 10.0, 30.0] {
        emit_box(mesh, &body, region, tex, [-1.0, -6.0, z0], [1.0, 0.0, z0 + 12.0], [220.0, 53.0], false, light, color);
    }
    // Wings and legs: the right side, then the left as its mirror image.
    let wing_x = 0.125 - f8.cos() * 0.2;
    let wing_z = (f8.sin() + 0.125) * 0.8;
    let tip_z = -((f8 + 2.0).sin() + 0.5) * 0.75;
    for side in [1.0f32, -1.0] {
        let m = side < 0.0;
        // The mirror flips x, and with it the Y and Z turns.
        let x = |v: f32| v * side;
        let wing = group
            * part(
                Vec3::new(x(-12.0), 5.0, 2.0),
                Vec3::new(wing_x, x(0.25), x(wing_z)),
            );
        let (bone_from, bone_to) = if m { ([0.0, -4.0, -4.0], [56.0, 4.0, 4.0]) } else { ([-56.0, -4.0, -4.0], [0.0, 4.0, 4.0]) };
        emit_box(mesh, &wing, region, tex, bone_from, bone_to, [112.0, 88.0], m, light, color);
        let (skin_from, skin_to) = if m { ([0.0, 0.0, 2.0], [56.0, 0.0, 58.0]) } else { ([-56.0, 0.0, 2.0], [0.0, 0.0, 58.0]) };
        emit_box(mesh, &wing, region, tex, skin_from, skin_to, [-56.0, 88.0], m, light, color);
        let tip = wing * part(Vec3::new(x(-56.0), 0.0, 0.0), Vec3::new(0.0, 0.0, x(tip_z)));
        let (bone_from, bone_to) = if m { ([0.0, -2.0, -2.0], [56.0, 2.0, 2.0]) } else { ([-56.0, -2.0, -2.0], [0.0, 2.0, 2.0]) };
        emit_box(mesh, &tip, region, tex, bone_from, bone_to, [112.0, 136.0], m, light, color);
        emit_box(mesh, &tip, region, tex, skin_from, skin_to, [-56.0, 144.0], m, light, color);
        let front = group * part(Vec3::new(x(-12.0), 20.0, 2.0), Vec3::new(1.3 + f1 * 0.1, 0.0, 0.0));
        emit_box(mesh, &front, region, tex, [-4.0, -4.0, -4.0], [4.0, 20.0, 4.0], [112.0, 104.0], m, light, color);
        let front_tip = front * part(Vec3::new(0.0, 20.0, -1.0), Vec3::new(-0.5 - f1 * 0.1, 0.0, 0.0));
        emit_box(mesh, &front_tip, region, tex, [-3.0, -1.0, -3.0], [3.0, 23.0, 3.0], [226.0, 138.0], m, light, color);
        let front_foot = front_tip * part(Vec3::new(0.0, 23.0, 0.0), Vec3::new(0.75 + f1 * 0.1, 0.0, 0.0));
        emit_box(mesh, &front_foot, region, tex, [-4.0, 0.0, -12.0], [4.0, 4.0, 4.0], [144.0, 104.0], m, light, color);
        let rear = group * part(Vec3::new(x(-16.0), 16.0, 42.0), Vec3::new(1.0 + f1 * 0.1, 0.0, 0.0));
        emit_box(mesh, &rear, region, tex, [-8.0, -4.0, -8.0], [8.0, 28.0, 8.0], [0.0, 0.0], m, light, color);
        let rear_tip = rear * part(Vec3::new(0.0, 32.0, -4.0), Vec3::new(0.5 + f1 * 0.1, 0.0, 0.0));
        emit_box(mesh, &rear_tip, region, tex, [-6.0, -2.0, 0.0], [6.0, 30.0, 12.0], [196.0, 0.0], m, light, color);
        let rear_foot = rear_tip * part(Vec3::new(0.0, 31.0, 4.0), Vec3::new(0.75 + f1 * 0.1, 0.0, 0.0));
        emit_box(mesh, &rear_foot, region, tex, [-9.0, 0.0, -20.0], [9.0, 6.0, 4.0], [112.0, 0.0], m, light, color);
    }
    // The tail.
    let (mut x, mut y, mut z) = (0.0f32, 10.0f32, 60.0f32);
    let s11 = history(11);
    let mut bend = 0.0f32;
    for k in 0..12 {
        let s = history(12 + k);
        bend += ((k as f32) * 0.45 + f8).sin() * 0.05;
        let ry = (wrap_degrees(s.yaw - s11.yaw) * 1.5 + 180.0) * deg;
        let rx = bend + (s.y - s11.y) * deg * 1.5 * 5.0;
        let rz = wrap_degrees(s.yaw - f7) * deg * 1.5;
        neck_box(mesh, &(model * part(Vec3::new(x, y, z), Vec3::new(rx, ry, rz))));
        y += rx.sin() * 10.0;
        z -= ry.cos() * rx.cos() * 10.0;
        x -= ry.sin() * rx.cos() * 10.0;
    }
}

/// An End crystal as it is drawn this frame.
pub struct CrystalPose {
    pub position: DVec3,
    /// `EndCrystal.time` with the partial tick.
    pub time: f32,
}

/// `EndCrystalRenderer`: two spinning glass cubes round a core, bobbing
/// above where it stands, at full brightness.
pub fn append_crystals(mesh: &mut ChunkMesh, crystals: &[CrystalPose], atlas: &Atlas) {
    let Ok(id) = ResourceId::parse(CRYSTAL) else {
        return;
    };
    if !atlas.contains(&id) {
        return;
    }
    let region = atlas.entity_region(&id);
    let tex = [64.0, 32.0];
    let tilt = Quat::from_axis_angle(
        Vec3::new(std::f32::consts::FRAC_1_SQRT_2, 0.0, std::f32::consts::FRAC_1_SQRT_2),
        60f32.to_radians(),
    );
    for crystal in crystals {
        let t = crystal.time;
        let mut bob = (t * 0.2).sin() / 2.0 + 0.5;
        bob = (bob * bob + bob) * 0.2;
        let spin = Quat::from_rotation_y((t * 3.0).to_radians());
        let mut pose = Mat4::from_translation(crystal.position.as_vec3())
            * Mat4::from_scale(Vec3::splat(2.0))
            * Mat4::from_translation(Vec3::new(0.0, -0.5, 0.0))
            * Mat4::from_quat(spin)
            * Mat4::from_translation(Vec3::new(0.0, 0.8 + bob, 0.0))
            * Mat4::from_quat(tilt);
        let px = Mat4::from_scale(Vec3::splat(1.0 / 16.0));
        let color = [1.0, 1.0, 1.0, 1.0];
        emit_box(mesh, &(pose * px), region, tex, [-4.0; 3], [4.0; 3], [0.0, 0.0], false, Light::FULL, color);
        pose = pose * Mat4::from_scale(Vec3::splat(0.875)) * Mat4::from_quat(tilt) * Mat4::from_quat(spin);
        emit_box(mesh, &(pose * px), region, tex, [-4.0; 3], [4.0; 3], [0.0, 0.0], false, Light::FULL, color);
        pose = pose * Mat4::from_scale(Vec3::splat(0.875)) * Mat4::from_quat(tilt) * Mat4::from_quat(spin);
        emit_box(mesh, &(pose * px), region, tex, [-4.0; 3], [4.0; 3], [32.0, 0.0], false, Light::FULL, color);
    }
}

/// The beam from a crystal to the dragon it heals (`EnderDragonRenderer.
/// renderCrystalBeams`): a scrolling strip, drawn as two crossed quads.
pub fn append_beam(mesh: &mut ChunkMesh, from: Vec3, to: Vec3, time: f32, atlas: &Atlas) {
    let Ok(id) = ResourceId::parse(CRYSTAL_BEAM) else {
        return;
    };
    if !atlas.contains(&id) {
        return;
    }
    let [u0, v0, u1, v1] = atlas.region(&id);
    let along = to - from;
    let length = along.length();
    if length < 0.01 {
        return;
    }
    let dir = along / length;
    let side_a = dir.any_orthonormal_vector();
    let side_b = dir.cross(side_a);
    // The texture repeats every block along the beam; the strip is clamped
    // to one tile here, scrolled with time.
    let scroll = (time * 0.01).fract();
    for side in [side_a, side_b] {
        let half = side * 0.15;
        let start = mesh.vertices.len() as u32;
        for (point, uv) in [
            (from - half, [u0, v0 + (v1 - v0) * scroll]),
            (from + half, [u1, v0 + (v1 - v0) * scroll]),
            (to + half, [u1, v1]),
            (to - half, [u0, v1]),
        ] {
            mesh.vertices.push(Vertex {
                position: point.to_array(),
                uv,
                color: [1.0, 1.0, 1.0, 1.0],
                sky_light: 15.0,
                block_light: 15.0,
            });
        }
        mesh.indices.extend_from_slice(&[
            start,
            start + 1,
            start + 2,
            start,
            start + 2,
            start + 3,
            start + 2,
            start + 1,
            start,
            start + 3,
            start + 2,
            start,
        ]);
        mesh.faces += 2;
    }
}

/// A flat sprite facing the camera (`ThrownItemRenderer`,
/// `DragonFireballRenderer`), `size` blocks across.
pub fn append_sprite(
    mesh: &mut ChunkMesh,
    texture: &str,
    at: Vec3,
    size: f32,
    forward: Vec3,
    color: [f32; 4],
    light: Light,
    atlas: &Atlas,
) {
    let Ok(id) = ResourceId::parse(texture) else {
        return;
    };
    if !atlas.contains(&id) {
        return;
    }
    let [u0, v0, u1, v1] = atlas.region(&id);
    let right = forward.cross(Vec3::Y).normalize_or(Vec3::X);
    let up = right.cross(forward).normalize_or(Vec3::Y);
    let start = mesh.vertices.len() as u32;
    let h = size * 0.5;
    for (corner, uv) in [
        (Vec2::new(-h, -h), [u0, v1]),
        (Vec2::new(h, -h), [u1, v1]),
        (Vec2::new(h, h), [u1, v0]),
        (Vec2::new(-h, h), [u0, v0]),
    ] {
        let point = at + right * corner.x + up * corner.y;
        mesh.vertices.push(Vertex {
            position: point.to_array(),
            uv,
            color,
            sky_light: light.sky,
            block_light: light.block,
        });
    }
    mesh.indices
        .extend_from_slice(&[start, start + 1, start + 2, start, start + 2, start + 3]);
    mesh.faces += 1;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_living_box_faces_outward() {
        let mut mesh = ChunkMesh::default();
        let pose = living_pose(DVec3::ZERO, 0.0);
        emit_box(
            &mut mesh,
            &pose,
            [0.0, 0.0, 1.0, 1.0],
            [64.0, 32.0],
            [-4.0, -4.0, -4.0],
            [4.0, 4.0, 4.0],
            [0.0, 0.0],
            false,
            Light::FULL,
            [1.0; 4],
        );
        assert_eq!(mesh.faces, 6);
        let center = pose.transform_point3(Vec3::ZERO);
        for face in 0..6 {
            let i = face * 6;
            let p = |k: usize| Vec3::from_array(mesh.vertices[mesh.indices[i + k] as usize].position);
            let (a, b, c) = (p(0), p(1), p(2));
            let normal = (b - a).cross(c - a);
            let mid = (a + b + c) / 3.0;
            assert!(normal.dot(mid - center) > 0.0, "face {face} points inward");
        }
    }

    /// Draws a mesh into a picture with a z-buffer, sampling the atlas: a
    /// look at the models without the game (`MC_PACK` names the resource
    /// pack, `MC_RENDER_OUT` the folder the pictures go to).
    fn rasterize(mesh: &ChunkMesh, atlas: &Atlas, eye: Vec3, at: Vec3, path: &std::path::Path) {
        let (w, h) = (640u32, 480u32);
        let view = Mat4::look_at_rh(eye, at, Vec3::Y);
        let proj = Mat4::perspective_rh(60f32.to_radians(), w as f32 / h as f32, 0.1, 500.0);
        let m = proj * view;
        let mut img = image::RgbaImage::from_pixel(w, h, image::Rgba([40, 30, 50, 255]));
        let mut depth = vec![f32::INFINITY; (w * h) as usize];
        let (aw, ah) = (atlas.pixels.width() as f32, atlas.pixels.height() as f32);
        for tri in mesh.indices.chunks(3) {
            let v: Vec<&Vertex> = tri.iter().map(|&i| &mesh.vertices[i as usize]).collect();
            let p: Vec<Vec3> = v
                .iter()
                .map(|v| {
                    let c = m * Vec3::from_array(v.position).extend(1.0);
                    Vec3::new(
                        (c.x / c.w * 0.5 + 0.5) * w as f32,
                        (1.0 - (c.y / c.w * 0.5 + 0.5)) * h as f32,
                        c.w,
                    )
                })
                .collect();
            if p.iter().any(|p| p.z <= 0.1) {
                continue;
            }
            let (x0, x1) = (p.iter().map(|p| p.x).fold(f32::MAX, f32::min), p.iter().map(|p| p.x).fold(f32::MIN, f32::max));
            let (y0, y1) = (p.iter().map(|p| p.y).fold(f32::MAX, f32::min), p.iter().map(|p| p.y).fold(f32::MIN, f32::max));
            let area = (p[1].x - p[0].x) * (p[2].y - p[0].y) - (p[2].x - p[0].x) * (p[1].y - p[0].y);
            if area.abs() < 1e-6 {
                continue;
            }
            for y in (y0.max(0.0) as u32)..(y1.min(h as f32 - 1.0) as u32 + 1) {
                for x in (x0.max(0.0) as u32)..(x1.min(w as f32 - 1.0) as u32 + 1) {
                    let (px, py) = (x as f32 + 0.5, y as f32 + 0.5);
                    let b0 = ((p[1].x - px) * (p[2].y - py) - (p[2].x - px) * (p[1].y - py)) / area;
                    let b1 = ((p[2].x - px) * (p[0].y - py) - (p[0].x - px) * (p[2].y - py)) / area;
                    let b2 = 1.0 - b0 - b1;
                    if b0 < 0.0 || b1 < 0.0 || b2 < 0.0 {
                        continue;
                    }
                    let z = b0 * p[0].z + b1 * p[1].z + b2 * p[2].z;
                    let k = (y * w + x) as usize;
                    if z >= depth[k] {
                        continue;
                    }
                    let u = b0 * v[0].uv[0] + b1 * v[1].uv[0] + b2 * v[2].uv[0];
                    let t = b0 * v[0].uv[1] + b1 * v[1].uv[1] + b2 * v[2].uv[1];
                    let texel = atlas.pixels.get_pixel(
                        ((u * aw) as u32).min(atlas.pixels.width() - 1),
                        ((t * ah) as u32).min(atlas.pixels.height() - 1),
                    );
                    if texel[3] < 26 {
                        continue;
                    }
                    depth[k] = z;
                    let shade = v[0].color[0].clamp(0.0, 1.0);
                    img.put_pixel(
                        x,
                        y,
                        image::Rgba([
                            (texel[0] as f32 * shade) as u8,
                            (texel[1] as f32 * shade) as u8,
                            (texel[2] as f32 * shade) as u8,
                            255,
                        ]),
                    );
                }
            }
        }
        img.save(path).unwrap();
    }

    #[test]
    fn render_end_game_models() {
        let pack = std::env::var("MC_PACK").expect("MC_PACK must point to the original resource pack");
        let out = std::env::var("MC_RENDER_OUT").expect("MC_RENDER_OUT must point to a test output directory");
        let out = std::path::PathBuf::from(out);
        std::fs::create_dir_all(&out).expect("create test render output");
        let packs = crate::pack::PackStack::open(vec![std::path::PathBuf::from(pack)]).unwrap();
        let ids = TEXTURES.iter().map(|t| ResourceId::parse(t).unwrap()).collect();
        let atlas = crate::mesh::make_atlas(&packs, ids).unwrap();
        assert!(atlas.missing.is_empty(), "missing {:?}", atlas.missing);
        for (name, yaw, sitting, eye) in [
            ("dragon_side", 0.0f32, false, Vec3::new(22.0, 4.0, 0.0)),
            ("dragon_front", 0.0, false, Vec3::new(4.0, 6.0, -22.0)),
            ("dragon_turning", 30.0, false, Vec3::new(-15.0, 12.0, 15.0)),
            ("dragon_sitting", 0.0, true, Vec3::new(14.0, 6.0, -14.0)),
        ] {
            let mut mesh = ChunkMesh::default();
            let history = |i: usize| FlightSample {
                yaw: yaw * (1.0 - i as f32 / 24.0),
                y: 0.0,
            };
            let offset = |part: usize, s: FlightSample, h: FlightSample| {
                if sitting { part as f32 } else if part == 6 { 0.0 } else { h.y - s.y }
            };
            append_dragon(
                &mut mesh,
                &DragonPose {
                    position: DVec3::ZERO,
                    history: &history,
                    flap: 0.3,
                    head_part_offset: &offset,
                    hurt: false,
                    dying: 0.0,
                },
                &atlas,
                Light::FULL,
            );
            rasterize(&mesh, &atlas, eye, Vec3::new(0.0, 2.0, -2.0), &out.join(format!("{name}.png")));
        }
        let mut mesh = ChunkMesh::default();
        append_blazes(
            &mut mesh,
            &[BlazePose { feet: DVec3::ZERO, body_yaw: 30.0, head_yaw: 30.0, head_pitch: 0.0, age: 10.0, hurt: false }],
            &atlas,
        );
        append_crystals(&mut mesh, &[CrystalPose { position: DVec3::new(2.5, 0.0, 0.0), time: 5.0 }], &atlas);
        rasterize(&mesh, &atlas, Vec3::new(1.2, 1.6, 5.0), Vec3::new(1.2, 1.0, 0.0), &out.join("blaze_crystal.png"));
    }
}
