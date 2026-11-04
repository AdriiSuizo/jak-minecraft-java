//! Boats and minecarts drawn as vanilla draws them: `BoatModel`,
//! `RaftModel` and `MinecartModel`'s layers (their boxes, texture offsets
//! and part poses; the chest boats' and chest rafts' chests their own
//! parts), a minecart's block (chest, furnace, hopper, TNT) inside it as
//! `AbstractMinecartRenderer` sets it,
//! placed as `BoatRenderer` and `MinecartRenderer` place them (0.375 up,
//! turned `180 - yaw`, `scale(-1, -1, 1)`; the boat a quarter turn more),
//! the boat's paddles swung as `BoatModel.animatePaddle` swings them, and
//! both rocked by a hit (`hurtTime`, `damage`).
use crate::{
    lighting::SkyLight,
    mesh::{Atlas, ChunkMesh},
    pack::{PackStack, ResourceId},
};
use glam::{DVec3, EulerRot, Quat};
use std::f32::consts::PI;

/// The boats' woods (`Boat.Type`, a texture each); bamboo's is the raft,
/// with its own model.
pub const WOODS: [&str; 11] =
    ["oak", "spruce", "birch", "jungle", "acacia", "cherry", "dark_oak", "mangrove", "pale_oak", "poplar", "bamboo"];

/// The wood whose boat is the bamboo raft (`Raft`, `RaftModel`).
pub const RAFT: u8 = 10;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum VehicleModel {
    /// A boat of `WOODS[i]` (the raft for bamboo).
    Boat(u8),
    /// A chest boat of `WOODS[i]` (the chest raft for bamboo).
    ChestBoat(u8),
    Minecart,
    ChestMinecart,
    FurnaceMinecart,
    HopperMinecart,
    TntMinecart,
}

impl VehicleModel {
    pub fn is_boat(self) -> bool {
        matches!(self, VehicleModel::Boat(_) | VehicleModel::ChestBoat(_))
    }

    pub fn is_cart(self) -> bool {
        !self.is_boat()
    }

    /// Its wood (a boat's), the raft's bamboo for a raft.
    pub fn wood(self) -> Option<u8> {
        match self {
            VehicleModel::Boat(w) | VehicleModel::ChestBoat(w) => Some(w),
            _ => None,
        }
    }

    pub fn is_raft(self) -> bool {
        self.wood() == Some(RAFT)
    }

    /// The slots it carries: a chest's 27, a hopper's 5, none.
    pub fn slots(self) -> usize {
        match self {
            VehicleModel::ChestBoat(_) | VehicleModel::ChestMinecart => 27,
            VehicleModel::HopperMinecart => 5,
            _ => 0,
        }
    }

    /// The seats it has: a boat two, a chest boat one (its chest takes the
    /// back), a minecart one, the others none.
    pub fn seats(self) -> usize {
        match self {
            VehicleModel::Boat(_) => 2,
            VehicleModel::ChestBoat(_) | VehicleModel::Minecart => 1,
            _ => 0,
        }
    }

    /// The block a minecart carries (`getDefaultDisplayBlockState`) and how
    /// high (`getDefaultDisplayOffset`, sixteenths).
    pub fn display(self) -> Option<(&'static str, i32)> {
        match self {
            VehicleModel::ChestMinecart => Some(("chest", 8)),
            VehicleModel::FurnaceMinecart => Some(("furnace", 6)),
            VehicleModel::HopperMinecart => Some(("hopper", 1)),
            VehicleModel::TntMinecart => Some(("tnt", 6)),
            _ => None,
        }
    }
}

/// A vehicle to draw: where it is (blocks), its yaw and pitch (degrees, as
/// the entity's), its paddles' rowing times (radians), and its hit's wobble
/// (`hurtTime` ticks left, `damage`, the side it rocks to).
#[derive(Clone, Copy, Debug)]
pub struct VehicleView {
    pub model: VehicleModel,
    pub pos: DVec3,
    pub yaw: f32,
    pub pitch: f32,
    pub paddles: [f32; 2],
    pub hurt_time: f32,
    pub damage: f32,
    pub hurt_dir: f32,
    /// A furnace minecart burning its fuel (its furnace lit).
    pub lit: bool,
    /// A TNT minecart's fuse (ticks left), once primed: it flashes.
    pub fuse: Option<f32>,
}

pub fn texture(model: VehicleModel) -> ResourceId {
    match model {
        VehicleModel::Boat(i) => ResourceId::parse(&format!("minecraft:entity/boat/{}", WOODS[usize::from(i) % WOODS.len()])),
        VehicleModel::ChestBoat(i) => {
            ResourceId::parse(&format!("minecraft:entity/chest_boat/{}", WOODS[usize::from(i) % WOODS.len()]))
        }
        // (26.3 keeps it in its own folder: `entity/minecart/minecart`.)
        _ => ResourceId::parse("minecraft:entity/minecart/minecart"),
    }
    .expect("vehicle texture id")
}

/// Every vehicle texture, for the entity atlas.
pub fn textures() -> Vec<ResourceId> {
    (0..WOODS.len() as u8)
        .flat_map(|i| [texture(VehicleModel::Boat(i)), texture(VehicleModel::ChestBoat(i))])
        .chain([texture(VehicleModel::Minecart)])
        .collect()
}

/// A box: from, size, texture offset; a part: its pivot and rotation (x,
/// y, z radians, applied z then y then x as `ModelPart` does).
type Part = (&'static [([f32; 3], [f32; 3], [f32; 2])], [f32; 3], [f32; 3]);

/// `MinecartModel.createBodyLayer` (64 x 32).
const MINECART: [Part; 5] = [
    (&[([-10.0, -8.0, -1.0], [20.0, 16.0, 2.0], [0.0, 10.0])], [0.0, 4.0, 0.0], [PI / 2.0, 0.0, 0.0]),
    (&[([-8.0, -9.0, -1.0], [16.0, 8.0, 2.0], [0.0, 0.0])], [-9.0, 4.0, 0.0], [0.0, PI * 1.5, 0.0]),
    (&[([-8.0, -9.0, -1.0], [16.0, 8.0, 2.0], [0.0, 0.0])], [9.0, 4.0, 0.0], [0.0, PI / 2.0, 0.0]),
    (&[([-8.0, -9.0, -1.0], [16.0, 8.0, 2.0], [0.0, 0.0])], [0.0, 4.0, -7.0], [0.0, PI, 0.0]),
    (&[([-8.0, -9.0, -1.0], [16.0, 8.0, 2.0], [0.0, 0.0])], [0.0, 4.0, 7.0], [0.0, 0.0, 0.0]),
];

/// `BoatModel.createBoatModel`'s hull (128 x 64).
const BOAT: [Part; 5] = [
    (&[([-14.0, -9.0, -3.0], [28.0, 16.0, 3.0], [0.0, 0.0])], [0.0, 3.0, 1.0], [PI / 2.0, 0.0, 0.0]),
    (&[([-13.0, -7.0, -1.0], [18.0, 6.0, 2.0], [0.0, 19.0])], [-15.0, 4.0, 4.0], [0.0, PI * 1.5, 0.0]),
    (&[([-8.0, -7.0, -1.0], [16.0, 6.0, 2.0], [0.0, 27.0])], [15.0, 4.0, 0.0], [0.0, PI / 2.0, 0.0]),
    (&[([-14.0, -7.0, -1.0], [28.0, 6.0, 2.0], [0.0, 35.0])], [0.0, 4.0, -9.0], [0.0, PI, 0.0]),
    (&[([-14.0, -7.0, -1.0], [28.0, 6.0, 2.0], [0.0, 43.0])], [0.0, 4.0, 9.0], [0.0, 0.0, 0.0]),
];
/// `RaftModel.createChildren`'s deck (128 x 64): two slabs, turned flat.
const RAFT_DECK: [Part; 1] = [(
    &[([-14.0, -11.0, -4.0], [28.0, 20.0, 4.0], [0.0, 0.0]), ([-14.0, -9.0, -8.0], [28.0, 16.0, 4.0], [0.0, 0.0])],
    [0.0, -2.1, 1.0],
    [1.5708, 0.0, 0.0],
)];
/// The raft's paddles: the boat's shapes, laid out at (0, 24) and (40, 24),
/// pivoted at (3, -4, 9) and (3, -4, -9).
const RAFT_LEFT_PADDLE: &[([f32; 3], [f32; 3], [f32; 2])] =
    &[([-1.0, 0.0, -5.0], [2.0, 2.0, 18.0], [0.0, 24.0]), ([-1.001, -3.0, 8.0], [1.0, 6.0, 7.0], [0.0, 24.0])];
const RAFT_RIGHT_PADDLE: &[([f32; 3], [f32; 3], [f32; 2])] =
    &[([-1.0, 0.0, -5.0], [2.0, 2.0, 18.0], [40.0, 24.0]), ([0.001, -3.0, 8.0], [1.0, 6.0, 7.0], [40.0, 24.0])];
/// `BoatModel.createChestBoatModel`'s chest (128 x 128): its bottom, lid
/// and lock, turned a quarter, at the boat's back.
const BOAT_CHEST: [Part; 3] = [
    (&[([0.0, 0.0, 0.0], [12.0, 8.0, 12.0], [0.0, 76.0])], [-2.0, -5.0, -6.0], [0.0, -PI / 2.0, 0.0]),
    (&[([0.0, 0.0, 0.0], [12.0, 4.0, 12.0], [0.0, 59.0])], [-2.0, -9.0, -6.0], [0.0, -PI / 2.0, 0.0]),
    (&[([0.0, 0.0, 0.0], [2.0, 4.0, 1.0], [0.0, 59.0])], [-1.0, -6.0, -1.0], [0.0, -PI / 2.0, 0.0]),
];
/// `RaftModel.createChestRaftModel`'s: the same chest, on the raft's deck.
const RAFT_CHEST: [Part; 3] = [
    (&[([0.0, 0.0, 0.0], [12.0, 8.0, 12.0], [0.0, 76.0])], [-2.0, -10.1, -6.0], [0.0, -PI / 2.0, 0.0]),
    (&[([0.0, 0.0, 0.0], [12.0, 4.0, 12.0], [0.0, 59.0])], [-2.0, -14.1, -6.0], [0.0, -PI / 2.0, 0.0]),
    (&[([0.0, 0.0, 0.0], [2.0, 4.0, 1.0], [0.0, 59.0])], [-1.0, -11.1, -1.0], [0.0, -PI / 2.0, 0.0]),
];
/// Its paddles: the left at (3, -5, 9), the right at (3, -5, -9).
const LEFT_PADDLE: &[([f32; 3], [f32; 3], [f32; 2])] =
    &[([-1.0, 0.0, -5.0], [2.0, 2.0, 18.0], [62.0, 0.0]), ([-1.001, -3.0, 8.0], [1.0, 6.0, 7.0], [62.0, 0.0])];
const RIGHT_PADDLE: &[([f32; 3], [f32; 3], [f32; 2])] =
    &[([-1.0, 0.0, -5.0], [2.0, 2.0, 18.0], [62.0, 20.0]), ([0.001, -3.0, 8.0], [1.0, 6.0, 7.0], [62.0, 20.0])];

/// `BoatModel.animatePaddle`: the paddle's x and y turn at its rowing time.
pub fn paddle_turn(time: f32, right: bool) -> (f32, f32) {
    let lerp = |a: f32, b: f32, t: f32| a + (b - a) * t.clamp(0.0, 1.0);
    let x = lerp(-PI / 3.0, -PI / 12.0, ((-time).sin() + 1.0) / 2.0);
    let mut y = lerp(-PI / 4.0, PI / 4.0, ((-time + 1.0).sin() + 1.0) / 2.0);
    if right {
        y = PI - y;
    }
    (x, y)
}

pub fn append(mesh: &mut ChunkMesh, vehicles: &[VehicleView], atlas: &Atlas, light: &SkyLight) {
    for v in vehicles {
        let region = atlas.entity_region(&texture(v.model));
        let pos = (v.pos.x.floor() as i32, (v.pos.y + 0.2).floor() as i32, v.pos.z.floor() as i32);
        let sky = light.get(pos) as f32;
        let block = light.get_block(pos) as f32;
        // `BoatRenderer`/`MinecartRenderer`: 0.375 up, turned 180 - yaw, the
        // hit's rock about x (`hurtTime / 10 * damage / 10` degrees by its
        // sine), the cart's pitch about z; `scale(-1, -1, 1)` is the model's
        // own (here in `model_to_entity`, which also lifts it 1.501).
        let wobble = if v.hurt_time > 0.0 {
            (v.hurt_time).sin() * v.hurt_time * v.damage.max(0.0) / 10.0 * v.hurt_dir
        } else {
            0.0
        };
        let mut rotation = Quat::from_rotation_y((180.0 - v.yaw).to_radians()) * Quat::from_rotation_x(wobble.to_radians());
        let (parts, size, quarter): (&[Part], [f32; 2], bool) = match v.model {
            VehicleModel::Boat(RAFT) => (&RAFT_DECK, [128.0, 64.0], true),
            VehicleModel::Boat(_) => (&BOAT, [128.0, 64.0], true),
            VehicleModel::ChestBoat(RAFT) => (&RAFT_DECK, [128.0, 128.0], true),
            VehicleModel::ChestBoat(_) => (&BOAT, [128.0, 128.0], true),
            _ => {
                rotation *= Quat::from_rotation_z((-v.pitch).to_radians());
                (&MINECART, [64.0, 32.0], false)
            }
        };
        if quarter {
            rotation *= Quat::from_rotation_y(PI / 2.0);
        }
        let feet = v.pos + DVec3::new(0.0, 0.375 - 1.501, 0.0);
        let mut emit = |boxes: &[([f32; 3], [f32; 3], [f32; 2])], pivot: [f32; 3], turn: [f32; 3]| {
            let part = Quat::from_euler(EulerRot::ZYX, turn[2], turn[1], turn[0]);
            for (from, dims, uv) in boxes {
                let to = [from[0] + dims[0], from[1] + dims[1], from[2] + dims[2]];
                crate::cow_render::cube_tinted_pose_mirror(
                    mesh, feet, rotation, 1.0, region, sky, block, *from, to, *uv, pivot, part, [1.0; 3], size, None,
                    false,
                );
            }
        };
        for (boxes, pivot, turn) in parts {
            emit(boxes, *pivot, *turn);
        }
        if let VehicleModel::ChestBoat(wood) = v.model {
            for (boxes, pivot, turn) in if wood == RAFT { &RAFT_CHEST } else { &BOAT_CHEST } {
                emit(boxes, *pivot, *turn);
            }
        }
        if let Some(wood) = v.model.wood() {
            let (left, right, y) = if wood == RAFT {
                (RAFT_LEFT_PADDLE, RAFT_RIGHT_PADDLE, -4.0)
            } else {
                (LEFT_PADDLE, RIGHT_PADDLE, -5.0)
            };
            let (lx, ly) = paddle_turn(v.paddles[0], false);
            emit(left, [3.0, y, 9.0], [lx, ly, PI / 16.0]);
            let (rx, ry) = paddle_turn(v.paddles[1], true);
            emit(right, [3.0, y, -9.0], [rx, ry, PI / 16.0]);
        }
    }
}

/// The blocks minecarts carry, inside them (`AbstractMinecartRenderer`:
/// scaled to three quarters, `(displayOffset - 8) / 16` up from its middle,
/// turned a quarter; a lit furnace while it burns, the TNT flashing white
/// on alternate five ticks of its fuse as `TntMinecartRenderer` does).
/// A block that could not be drawn is left out; the last such failure is
/// returned.
pub fn append_contents(
    mesh: &mut ChunkMesh,
    vehicles: &[VehicleView],
    packs: &PackStack,
    atlas: &Atlas,
    light: &SkyLight,
) -> anyhow::Result<()> {
    use glam::{Mat4, Vec3};
    let mut failed = Ok(());
    for v in vehicles {
        let Some((block, offset)) = v.model.display() else { continue };
        let mut state = crate::scene::Block::new(&format!("minecraft:{block}"));
        state = match v.model {
            VehicleModel::ChestMinecart => state.with("facing", "north"),
            VehicleModel::FurnaceMinecart => state.with("facing", "north").with("lit", if v.lit { "true" } else { "false" }),
            VehicleModel::HopperMinecart => state.with("facing", "down").with("enabled", "true"),
            _ => state,
        };
        let pos = (v.pos.x.floor() as i32, (v.pos.y + 0.2).floor() as i32, v.pos.z.floor() as i32);
        let sky = light.get(pos) as f32;
        let block_light = light.get_block(pos) as f32;
        let wobble = if v.hurt_time > 0.0 {
            (v.hurt_time).sin() * v.hurt_time * v.damage.max(0.0) / 10.0 * v.hurt_dir
        } else {
            0.0
        };
        let mut scale = 0.75;
        let mut brightness = 1.0;
        if let Some(fuse) = v.fuse {
            // `TntMinecartRenderer.renderWhiteSolidBlock`: swelling over its
            // last ten ticks, white on alternate fives.
            if fuse < 10.0 {
                let g = (1.0 - fuse / 10.0).clamp(0.0, 1.0);
                let g = g * g;
                scale *= 1.0 + g * g * 0.3;
            }
            if (fuse as i32) / 5 % 2 == 0 {
                brightness = 4.0;
            }
        }
        let pose = Mat4::from_translation(v.pos.as_vec3() + Vec3::Y * 0.375)
            * Mat4::from_quat(
                Quat::from_rotation_y((180.0 - v.yaw).to_radians())
                    * Quat::from_rotation_x(wobble.to_radians())
                    * Quat::from_rotation_z((-v.pitch).to_radians()),
            )
            * Mat4::from_scale(Vec3::splat(scale))
            * Mat4::from_translation(Vec3::new(-0.5, (offset - 8) as f32 / 16.0, 0.5))
            * Mat4::from_rotation_y(PI / 2.0);
        if let Err(error) = crate::mesh::append_block_posed(mesh, packs, atlas, &state, pose, sky, block_light, brightness) {
            failed = Err(error.context(format!("minecart {block}")));
        }
    }
    failed
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every vehicle's texture is named as the pack names it.
    #[test]
    fn textures_are_the_packs() {
        let all: Vec<String> = textures().iter().map(|t| format!("{}:{}", t.namespace, t.path)).collect();
        assert!(all.contains(&"minecraft:entity/boat/oak".to_owned()));
        assert!(all.contains(&"minecraft:entity/minecart/minecart".to_owned()));
        assert!(all.contains(&"minecraft:entity/chest_boat/bamboo".to_owned()));
        assert_eq!(all.len(), WOODS.len() * 2 + 1);
    }

    /// Every vehicle texture is a file of the game's (18/10: the minecart's
    /// moved to `entity/minecart/minecart` and every cart was drawn with
    /// the missing texture), checked against the client JAR when it is here.
    #[test]
    fn textures_are_in_the_game() {
        let Some(root) = std::env::var_os("MINECRAFTOSS_ROOT") else { return };
        let jar = std::path::Path::new(&root).join("client.jar");
        let Ok(file) = std::fs::File::open(&jar) else { return };
        let mut zip = zip::ZipArchive::new(file).expect("client.jar");
        for t in textures() {
            let path = format!("assets/{}/textures/{}.png", t.namespace, t.path);
            assert!(zip.by_name(&path).is_ok(), "{path} is not in the game");
        }
    }

    /// A paddle at rest and swung through a stroke stays within its arcs.
    #[test]
    fn paddles_swing_within_their_arcs() {
        for k in 0..32 {
            let (x, y) = paddle_turn(k as f32 * PI / 8.0, false);
            assert!((-PI / 3.0 - 1e-4..=-PI / 12.0 + 1e-4).contains(&x));
            assert!((-PI / 4.0 - 1e-4..=PI / 4.0 + 1e-4).contains(&y));
        }
    }
}
