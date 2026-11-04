//! The house kit of the Jak lands (Adrii, 23/10): buildings of pieces that
//! join as Minecraft's walls and panes do, entered by their doors.
//!
//! Each land has its kit (`Kit`): a wall that joins its neighbours (a clone
//! of vanilla's walls: thick, low or tall by what is over it, a post at its
//! ends and corners), a beam for the corners, a door (vanilla's, both
//! halves), roof tiles that are stairs (they turn corners by themselves when
//! a player lays them), a ridge and a floor, all in the textures of that
//! land's own buildings in the original games (`jak_items::BLOCK_LOOKS`).
//!
//! The roofs are gables of those tiles: rows rising from both eaves to a
//! ridge, the gable ends filled with wall, so the inside is open to the
//! roof and walked into by the door.
//!
//! `house` lays one in its own frame (`u` the way its door faces, `v`
//! across), resolving every wall's sides and post as `WallBlock` would
//! (`makeWallState`, `shouldRaisePost`) over the plan before it is placed,
//! since a chunk's blocks are written without neighbour updates.
//!
//! `Sites` puts buildings into the world as the Spargus outposts are: one a
//! square perhaps, at a spot by the seed, made once and cut by chunk.
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use minecraftoss_core::block::BlockStateId;
use minecraftoss_core::chunk::{Chunk, HeightmapKind};
use minecraftoss_world::chunk_map::WorldGen;

use crate::jak_lands::{mix, Land};
use crate::jak_structures::Placed;

/// One land's pieces (block names, no state).
#[derive(Clone, Copy, Debug)]
pub struct Kit {
    pub wall: &'static str,
    /// The corners' beam (a log: upright).
    pub beam: &'static str,
    pub door: &'static str,
    /// The roof's tiles (stairs).
    pub roof: &'static str,
    /// The ridge along the roof's top (a full block).
    pub ridge: &'static str,
    pub floor: &'static str,
    /// What hangs under the eaves (Snowy Mountain's icicles), if anything.
    pub eaves: Option<&'static str>,
}

/// Boggy Billy's hut (`swamp`): its wood, its tiled roof, its plank floor.
pub const SWAMP: Kit = Kit {
    wall: "jak:swamp_hut_wall",
    beam: "jak:swamp_hut_log",
    door: "jak:swamp_hut_door",
    roof: "jak:swamp_roof_stairs",
    ridge: "jak:swamp_roof",
    floor: "jak:swamp_planks",
    eaves: None,
};

/// The lurkers' cabins in their fort on Snowy Mountain (`snow`): wood
/// panels, poles, its gate's wood, a metal roof.
pub const SNOW: Kit = Kit {
    wall: "jak:snow_wood_wall",
    beam: "jak:snow_pole",
    door: "jak:snow_gate_door",
    roof: "jak:snow_roof_stairs",
    ridge: "jak:snow_roof",
    floor: "jak:snow_planks",
    eaves: Some("jak:snow_icicle[waterlogged=false]"),
};

/// Spargus's houses (`wascitya`): bleached stucco, metal poles, the
/// airlock's door, the corrugated roof.
pub const SPARGUS: Kit = Kit {
    wall: "jak:spargus_stucco_wall",
    beam: "jak:spargus_pole_log",
    door: "jak:spargus_door",
    roof: "jak:spargus_roof_stairs",
    ridge: "jak:spargus_roof",
    floor: "jak:spargus_planks",
    eaves: None,
};

/// The fort's ring wall on Snowy Mountain (`snow-fortwallbase`).
pub const SNOW_FORT_WALL: &str = "jak:snow_fort_wall";

/// A state's name kept for the life of the game (the few hundred a kit's
/// pieces take), so a plan is `Placed` as every structure's is.
fn intern(name: String) -> &'static str {
    static NAMES: Mutex<Option<HashMap<String, &'static str>>> = Mutex::new(None);
    let mut names = NAMES.lock().unwrap_or_else(|e| e.into_inner());
    let names = names.get_or_insert_with(HashMap::new);
    if let Some(&name) = names.get(&name) {
        return name;
    }
    let kept: &'static str = Box::leak(name.clone().into_boxed_str());
    names.insert(name, kept);
    kept
}

/// What a planned cell is.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Cell {
    Air,
    /// A full block (or anything a wall joins to and that covers it).
    Solid(&'static str),
    /// Stairs: a wall under it is covered.
    Stairs(&'static str),
    /// A wall of this name, resolved when placed.
    Wall(&'static str),
    /// Anything else, as it is (a door's halves, a lantern, a bed).
    Thing(&'static str),
}

/// A building planned cell by cell before it is placed.
#[derive(Default)]
pub struct Plan {
    cells: HashMap<(i32, i32, i32), Cell>,
    order: Vec<(i32, i32, i32)>,
}

const SIDES: [(&str, (i32, i32)); 4] = [("north", (0, -1)), ("east", (1, 0)), ("south", (0, 1)), ("west", (-1, 0))];

impl Plan {
    fn put(&mut self, at: (i32, i32, i32), cell: Cell) {
        if self.cells.insert(at, cell).is_none() {
            self.order.push(at);
        }
    }

    pub fn air(&mut self, at: (i32, i32, i32)) {
        self.put(at, Cell::Air);
    }

    pub fn solid(&mut self, at: (i32, i32, i32), name: &'static str) {
        self.put(at, Cell::Solid(name));
    }

    pub fn wall(&mut self, at: (i32, i32, i32), name: &'static str) {
        self.put(at, Cell::Wall(name));
    }

    pub fn thing(&mut self, at: (i32, i32, i32), name: &'static str) {
        self.put(at, Cell::Thing(name));
    }

    fn stairs(&mut self, at: (i32, i32, i32), name: &'static str) {
        self.put(at, Cell::Stairs(name));
    }

    /// The blocks, every wall joined as `WallBlock` joins: a side toward a
    /// wall or a full block is joined, tall when what is over it covers it
    /// (a wall joined that way, a full block, stairs), low else; its post
    /// raised by `shouldRaisePost`. Walls are resolved from the top down,
    /// as each reads the one over it.
    pub fn placed(&self, out: &mut Vec<Placed>) {
        let mut walls: Vec<(i32, i32, i32)> = self.order.iter().copied().filter(|at| matches!(self.cells[at], Cell::Wall(_))).collect();
        walls.sort_by_key(|at| std::cmp::Reverse(at.1));
        // Each wall's sides (none 0, low 1, tall 2) and post.
        let mut resolved: HashMap<(i32, i32, i32), ([u8; 4], bool)> = HashMap::new();
        for &(x, y, z) in &walls {
            let above = self.cells.get(&(x, y + 1, z)).copied().unwrap_or(Cell::Air);
            let mut sides = [0u8; 4];
            for (k, (_, (dx, dz))) in SIDES.iter().enumerate() {
                let joined = matches!(self.cells.get(&(x + dx, y, z + dz)), Some(Cell::Wall(_) | Cell::Solid(_)));
                if !joined {
                    continue;
                }
                let covered = match above {
                    Cell::Solid(_) | Cell::Stairs(_) => true,
                    Cell::Wall(_) => resolved.get(&(x, y + 1, z)).is_some_and(|(s, _)| s[k] > 0),
                    _ => false,
                };
                sides[k] = if covered { 2 } else { 1 };
            }
            // `shouldRaisePost`.
            let [n, e, s, w] = sides;
            let post = if matches!(above, Cell::Wall(_)) && resolved.get(&(x, y + 1, z)).is_some_and(|(_, up)| *up) {
                true
            } else if (n == 0 && e == 0 && s == 0 && w == 0) || (s == 0) != (n == 0) || (w == 0) != (e == 0) {
                true
            } else if (n == 2 && s == 2) || (e == 2 && w == 2) {
                false
            } else {
                matches!(above, Cell::Solid(_))
            };
            resolved.insert((x, y, z), (sides, post));
        }
        let side = |v: u8| ["none", "low", "tall"][usize::from(v)];
        for at in &self.order {
            let name = match self.cells[at] {
                Cell::Air => "minecraft:air",
                Cell::Solid(n) | Cell::Stairs(n) | Cell::Thing(n) => n,
                Cell::Wall(n) => {
                    let ([nn, ee, ss, ww], up) = resolved[at];
                    intern(format!(
                        "{n}[east={},north={},south={},up={up},waterlogged=false,west={}]",
                        side(ee),
                        side(nn),
                        side(ss),
                        side(ww)
                    ))
                }
            };
            out.push((*at, name));
        }
    }
}

/// A building's frame: its origin (the floor's middle, where one stands),
/// `u` the way its front faces (one step), `v` across it.
#[derive(Clone, Copy, Debug)]
pub struct Frame {
    pub origin: (i32, i32, i32),
    pub facing: (i32, i32),
}

impl Frame {
    pub fn at(&self, u: i32, v: i32, k: i32) -> (i32, i32, i32) {
        let (fx, fz) = self.facing;
        let (x, y, z) = self.origin;
        (x + u * fx - v * fz, y + k, z + u * fz + v * fx)
    }

    /// The compass name of a step `(du, dv)` in the frame.
    pub fn toward(&self, du: i32, dv: i32) -> &'static str {
        let (fx, fz) = self.facing;
        compass((du * fx - dv * fz, du * fz + dv * fx))
    }
}

pub fn compass(step: (i32, i32)) -> &'static str {
    match step {
        (0, -1) => "north",
        (1, 0) => "east",
        (0, 1) => "south",
        _ => "west",
    }
}

/// A house's size: `half` blocks from its middle to its walls (5 wide at
/// 2, 7 at 3), its walls `height` blocks tall, raised `stilts` blocks on
/// beams over the ground (a porch and a ladder before its door then).
#[derive(Clone, Copy, Debug)]
pub struct House {
    pub half: i32,
    pub height: i32,
    pub stilts: i32,
}

/// A house of the kit in its frame: its floor, its beams at the corners,
/// its walls with the door in the front's middle and windows (gaps) in the
/// sides and back, its gable roof (eaves along the sides, one block out),
/// a lantern under the ridge. `ground` is the land's height at a column
/// (where a foundation or the stilts go down to).
pub fn house(plan: &mut Plan, kit: &Kit, frame: Frame, house: House, ground: &dyn Fn(i32, i32) -> i32) {
    let House { half: w, height: h, stilts } = house;
    let roof_top = h + w;
    // Cleared: the house's room and its roof's.
    for u in -w - 1..=w + 1 {
        for v in -w - 1..=w + 1 {
            for k in 0..=roof_top + 1 {
                plan.air(frame.at(u, v, k));
            }
        }
    }
    // The floor and what holds it up.
    let beam = intern(format!("{}[axis=y]", kit.beam));
    for u in -w..=w {
        for v in -w..=w {
            let (x, y, z) = frame.at(u, v, -1);
            plan.solid((x, y, z), kit.floor);
            let corner = u.abs() == w && v.abs() == w;
            let g = ground(x, z);
            if stilts > 0 {
                // Beams at the corners and the sides' middles down to the
                // ground (or the water's bed).
                if corner || (u.abs() == w && v == 0) || (v.abs() == w && u == 0) {
                    for y in g..y {
                        plan.solid((x, y, z), beam);
                    }
                }
            } else {
                for y in g.max(y - 6)..y {
                    plan.solid((x, y, z), kit.floor);
                }
            }
        }
    }
    // Beams up the corners, walls between.
    for u in -w..=w {
        for v in -w..=w {
            if u.abs() != w && v.abs() != w {
                continue;
            }
            for k in 0..h {
                if u.abs() == w && v.abs() == w {
                    plan.solid(frame.at(u, v, k), beam);
                } else {
                    plan.wall(frame.at(u, v, k), kit.wall);
                }
            }
        }
    }
    // The door, in the front's middle, facing in (as a player coming in
    // puts it down).
    let inward = frame.toward(-1, 0);
    for (k, half) in [(0, "lower"), (1, "upper")] {
        plan.thing(
            frame.at(w, 0, k),
            intern(format!("{}[facing={inward},half={half},hinge=left,open=false,powered=false]", kit.door)),
        );
    }
    // Windows: gaps at eye height in the sides and the back.
    for (u, v) in [(0, w), (0, -w), (-w, 0)] {
        if h > 2 {
            plan.air(frame.at(u, v, 1));
        }
    }
    // The gable roof: rows of tiles from the eaves (along `v` = +-(w+1),
    // level with the walls' top) up to the ridge, one block past the gable
    // ends; the gables walled.
    for i in 0..=w {
        let k = h - 1 + i;
        for (side, up) in [(-(w + 1 - i), 1), (w + 1 - i, -1)] {
            let facing = frame.toward(0, up);
            let tile = intern(format!("{}[facing={facing},half=bottom,shape=straight,waterlogged=false]", kit.roof));
            for u in -w - 1..=w + 1 {
                plan.stairs(frame.at(u, side, k), tile);
            }
        }
        if i >= 1 {
            for v in -(w - i)..=(w - i) {
                for u in [-w, w] {
                    plan.wall(frame.at(u, v, k), kit.wall);
                }
            }
        }
    }
    for u in -w - 1..=w + 1 {
        plan.solid(frame.at(u, 0, roof_top), kit.ridge);
    }
    plan.thing(frame.at(0, 0, roof_top - 1), "minecraft:lantern[hanging=true,waterlogged=false]");
    // Under the eaves, every other block.
    if let Some(hanging) = kit.eaves {
        for u in (-w - 1..=w + 1).step_by(2) {
            for v in [-(w + 1), w + 1] {
                plan.thing(frame.at(u, v, h - 2), hanging);
            }
        }
    }
    // Raised: a porch before the door and a ladder down from it to the
    // ground, on a beam.
    if stilts > 0 {
        for v in -1..=1 {
            plan.solid(frame.at(w + 1, v, -1), kit.floor);
        }
        let (x, y, z) = frame.at(w + 2, 0, -1);
        let ladder = intern(format!("minecraft:ladder[facing={},waterlogged=false]", frame.toward(1, 0)));
        let (bx, _, bz) = frame.at(w + 1, 0, -1);
        for yy in ground(x, z)..=y {
            plan.thing((x, yy, z), ladder);
            if yy < y {
                plan.solid((bx, yy, bz), beam);
            }
        }
    }
}

/// A bed (head toward the back), a crafting table and a chest (of its
/// game's things, `jak_loot`) inside a house of `half` 2 or more.
pub fn furnish(plan: &mut Plan, frame: Frame, half: i32, bed: &'static str) {
    let back = -(half - 1);
    let toward_back = frame.toward(-1, 0);
    plan.thing(frame.at(back, half - 1, 0), intern(format!("{bed}[facing={toward_back},occupied=false,part=head]")));
    plan.thing(frame.at(back + 1, half - 1, 0), intern(format!("{bed}[facing={toward_back},occupied=false,part=foot]")));
    plan.thing(frame.at(back, -(half - 1), 0), "minecraft:crafting_table");
    plan.thing(frame.at(back + 1, -(half - 1), 0), intern(format!("minecraft:chest[facing={},type=single,waterlogged=false]", frame.toward(1, 0))));
}

/// What a site reads of the land: its ground (under any water), its
/// surface (the water's, where there is), and whether a column is the
/// land's.
pub struct View<'a> {
    pub ground: &'a dyn Fn(i32, i32) -> i32,
    pub surface: &'a dyn Fn(i32, i32) -> i32,
    pub inside: &'a dyn Fn(i32, i32) -> bool,
}

/// One kind of building put about a land.
pub struct Sites {
    pub name: &'static str,
    pub land: &'static Land,
    /// One perhaps a square this many blocks wide.
    pub cell: i32,
    /// How many of a hundred squares have one.
    pub chance: u64,
    pub salt: u64,
    /// How far its blocks reach from its spot.
    pub reach: i32,
    pub build: fn(seed: i64, cell: (i32, i32), spot: (i32, i32), view: &View) -> Option<Vec<Placed>>,
    /// The game its chests' things are of (`jak_loot`).
    pub game: crate::jak_loot::Game,
}

impl Sites {
    pub fn spot(&self, seed: i64, cell: (i32, i32)) -> Option<(i32, i32)> {
        if mix(seed, cell.0, cell.1, self.salt) % 100 >= self.chance {
            return None;
        }
        let margin = self.reach + 8;
        let span = (self.cell - 2 * margin).max(1) as u64;
        let x = cell.0 * self.cell + margin + (mix(seed, cell.0, cell.1, self.salt + 1) % span) as i32;
        let z = cell.1 * self.cell + margin + (mix(seed, cell.0, cell.1, self.salt + 2) % span) as i32;
        Some((x, z))
    }
}

/// The ground's spread over probes round a spot (lowest, highest), none
/// where any probe is off the land.
pub fn probe(view: &View, (x, z): (i32, i32), r: i32, surface: bool) -> Option<(i32, i32)> {
    let mut lo = i32::MAX;
    let mut hi = i32::MIN;
    for (dx, dz) in [(0, 0), (r, 0), (-r, 0), (0, r), (0, -r), (r, r), (-r, -r), (r, -r), (-r, r)] {
        let (px, pz) = (x + dx, z + dz);
        if !(view.inside)(px, pz) {
            return None;
        }
        let y = if surface { (view.surface)(px, pz) } else { (view.ground)(px, pz) };
        lo = lo.min(y);
        hi = hi.max(y);
    }
    Some((lo, hi))
}

/// A facing by the seed.
pub fn facing_of(seed: i64, cell: (i32, i32), salt: u64) -> (i32, i32) {
    [(1, 0), (0, 1), (-1, 0), (0, -1)][(mix(seed, cell.0, cell.1, salt) % 4) as usize]
}

/// The sites' buildings by kind and square (made once, kept for the chunks
/// round).
static MADE: Mutex<Option<(i64, HashMap<(u64, (i32, i32)), Option<Arc<Vec<Placed>>>>)>> = Mutex::new(None);

fn made(sites: &Sites, seed: i64, cell: (i32, i32), gen: &WorldGen) -> Option<Arc<Vec<Placed>>> {
    let key = (sites.salt, cell);
    if let Some(found) = MADE.lock().ok().and_then(|m| m.as_ref().filter(|(s, _)| *s == seed).and_then(|(_, f)| f.get(&key).cloned())) {
        return found;
    }
    let spot = sites.spot(seed, cell)?;
    let biomes = &gen.terrain.registries.biomes;
    let ids: Vec<_> = sites.land.biomes.iter().filter_map(|b| biomes.id(b)).collect();
    let ground = |x: i32, z: i32| gen.terrain.base_height(x, z, HeightmapKind::OceanFloorWg);
    let surface = |x: i32, z: i32| gen.terrain.base_height(x, z, HeightmapKind::WorldSurfaceWg);
    let inside = |x: i32, z: i32| ids.contains(&gen.terrain.biome_at_quart(x >> 2, ground(x, z) >> 2, z >> 2));
    let view = View { ground: &ground, surface: &surface, inside: &inside };
    let built = (sites.build)(seed, cell, spot, &view).map(Arc::new);
    if let Ok(mut m) = MADE.lock() {
        let slot = m.get_or_insert_with(|| (seed, HashMap::new()));
        if slot.0 != seed {
            *slot = (seed, HashMap::new());
        }
        slot.1.insert(key, built.clone());
    }
    built
}

/// The sites' blocks over one chunk.
pub fn place(sites: &Sites, chunk: &mut Chunk, gen: &WorldGen, states: &mut HashMap<&'static str, BlockStateId>) {
    let seed = gen.terrain.seed;
    let (x0, z0) = (chunk.pos.x * 16, chunk.pos.z * 16);
    let cell_of = |v: i32| v.div_euclid(sites.cell);
    let registries = gen.terrain.registries.clone();
    for cx in cell_of(x0 - sites.reach)..=cell_of(x0 + 15 + sites.reach) {
        for cz in cell_of(z0 - sites.reach)..=cell_of(z0 + 15 + sites.reach) {
            let cell = (cx, cz);
            let Some((sx, sz)) = sites.spot(seed, cell) else { continue };
            if sx + sites.reach < x0 || sx - sites.reach > x0 + 15 || sz + sites.reach < z0 || sz - sites.reach > z0 + 15 {
                continue;
            }
            let Some(blocks) = made(sites, seed, cell, gen) else { continue };
            for &((x, y, z), state) in blocks.iter() {
                if x < x0 || x >= x0 + 16 || z < z0 || z >= z0 + 16 {
                    continue;
                }
                let id = *states.entry(state).or_insert_with(|| registries.blocks.parse_state(state).unwrap_or(BlockStateId::AIR));
                chunk.set_block_raw((x - x0) as usize, y, (z - z0) as usize, id, &registries);
                crate::jak_loot::fill(chunk, (x, y, z), state, sites.game, seed);
            }
        }
    }
}

/// Where a building is walked into: its (first) door's lower half and the
/// way in through it (one step).
#[derive(Clone, Copy, Debug)]
pub struct Entry {
    pub door: (i32, i32, i32),
    pub inward: (i32, i32),
}

/// A building's way in, by its first door.
pub fn entry(blocks: &[Placed]) -> Option<Entry> {
    blocks.iter().find(|(_, s)| s.contains("_door[") && s.contains("half=lower")).map(|&(door, s)| {
        let inward = ["north", "east", "south", "west"]
            .iter()
            .zip([(0, -1), (1, 0), (0, 1), (-1, 0)])
            .find(|(name, _)| s.contains(&format!("facing={name}")))
            .map_or((0, 1), |(_, step)| step);
        Entry { door, inward }
    })
}

/// The nearest built site of a kind to `(x, z)` (tests, `mc casa`): its
/// way in, within `reach` blocks.
pub fn nearest(sites: &Sites, registries: Arc<minecraftoss_core::Registries>, seed: i64, (x, z): (i32, i32), reach: i32) -> Option<Entry> {
    let terrain = minecraftoss_generator::terrain::TerrainGenerator::overworld(registries.clone(), seed).ok()?;
    let ids: Vec<_> = sites.land.biomes.iter().filter_map(|b| registries.biomes.id(b)).collect();
    let ground = |x: i32, z: i32| terrain.base_height(x, z, HeightmapKind::OceanFloorWg);
    let surface = |x: i32, z: i32| terrain.base_height(x, z, HeightmapKind::WorldSurfaceWg);
    let inside = |x: i32, z: i32| ids.contains(&terrain.biome_at_quart(x >> 2, ground(x, z) >> 2, z >> 2));
    let view = View { ground: &ground, surface: &surface, inside: &inside };
    let (cx, cz) = (x.div_euclid(sites.cell), z.div_euclid(sites.cell));
    let rings = reach / sites.cell + 1;
    let mut best: Option<(i64, Entry)> = None;
    for dx in -rings..=rings {
        for dz in -rings..=rings {
            let cell = (cx + dx, cz + dz);
            let Some((sx, sz)) = sites.spot(seed, cell) else { continue };
            let d = i64::from(sx - x).pow(2) + i64::from(sz - z).pow(2);
            if best.is_some_and(|(b, _)| d >= b) {
                continue;
            }
            if let Some(way) = (sites.build)(seed, cell, (sx, sz), &view).and_then(|b| entry(&b)) {
                best = Some((d, way));
            }
        }
    }
    best.map(|(_, at)| at)
}

/// Boggy Billy's huts: raised three blocks on roped poles over the swamp's
/// mud or water, a porch and a ladder before the door, a palisade round
/// their foot (built of `jak_facades`' parts, as Jak 1's `swamp` has them;
/// Adrii 23/10).
pub static SWAMP_HUTS: Sites = Sites {
    name: "cabaña del pantano",
    land: &crate::swamp::LAND,
    cell: 144,
    chance: 45,
    salt: 401,
    reach: 7,
    build: swamp_hut,
    game: crate::jak_loot::Game::Jak1,
};

fn swamp_hut(seed: i64, cell: (i32, i32), (x, z): (i32, i32), view: &View) -> Option<Vec<Placed>> {
    let (lo, hi) = probe(view, (x, z), 4, true)?;
    if hi - lo > 3 {
        return None;
    }
    let floor = hi + 3;
    let front = way_of(facing_of(seed, cell, 402));
    Some(crate::jak_facades::swamp_hut((x, floor, z), front, view.ground))
}

/// A facing as `jak_facades`' ways: 0 north, 1 east, 2 south, 3 west.
fn way_of(facing: (i32, i32)) -> u8 {
    match facing {
        (0, -1) => 0,
        (1, 0) => 1,
        (0, 1) => 2,
        _ => 3,
    }
}

/// The lurkers' forts on Snowy Mountain on a levelled yard: a palisade of
/// sharpened stakes, a gate between two towers of panels under snow, a
/// watchtower and a cabin (built of `jak_facades`' parts, as Jak 1's
/// `snow` has them; Adrii 23/10).
pub static SNOW_FORTS: Sites = Sites {
    name: "fuerte de la Montaña Nevada",
    land: &crate::snowy::LAND,
    cell: 192,
    chance: 50,
    salt: 501,
    reach: 9,
    build: snow_fort,
    game: crate::jak_loot::Game::Jak1,
};

fn snow_fort(seed: i64, cell: (i32, i32), (x, z): (i32, i32), view: &View) -> Option<Vec<Placed>> {
    let (lo, hi) = probe(view, (x, z), 8, true)?;
    if hi - lo > 5 || (view.surface)(x, z) > (view.ground)(x, z) {
        return None;
    }
    let y0 = (view.ground)(x, z);
    let r = 8;
    let facing = facing_of(seed, cell, 502);
    let frame = Frame { origin: (x, y0, z), facing };
    let mut plan = Plan::default();
    // The yard levelled: packed snow over what is under it, clear above.
    for u in -r..=r {
        for v in -r..=r {
            let (px, _, pz) = frame.at(u, v, 0);
            let g = (view.ground)(px, pz);
            for y in g.min(y0 - 1)..y0 - 1 {
                plan.solid((px, y, pz), "jak:snow_rock");
            }
            plan.solid((px, y0 - 1, pz), "jak:snow_packed");
            for k in 0..10 {
                plan.air(frame.at(u, v, k));
            }
        }
    }
    let mut out = Vec::new();
    plan.placed(&mut out);
    out.extend(crate::jak_facades::snow_fort((x, y0, z), way_of(facing)));
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn registries() -> Option<Arc<minecraftoss_core::Registries>> {
        let paths = minecraftoss_core::registries::DataPaths::discover().ok()?;
        paths.block_catalog.is_file().then(|| Arc::new(minecraftoss_core::Registries::load(&paths).unwrap()))
    }

    fn flat_house(kit: &Kit, spec: House) -> (Frame, HashMap<(i32, i32, i32), &'static str>) {
        let frame = Frame { origin: (0, 70, 0), facing: (0, 1) };
        let mut plan = Plan::default();
        house(&mut plan, kit, frame, spec, &|_, _| 64);
        furnish(&mut plan, frame, spec.half, "minecraft:red_bed");
        let mut out = Vec::new();
        plan.placed(&mut out);
        (frame, out.into_iter().collect())
    }

    /// A house of each kit: every state Minecraft's; its walls joined
    /// (straight runs tall without a post, ends by the door and windows
    /// posted), its door's two halves facing in, its roof tiles rising to
    /// the ridge, and a way in: from before the door to its middle through
    /// air and the door only.
    #[test]
    fn a_kit_house_is_whole_joined_and_walked_into() {
        let registries = registries().expect("the Minecraft data");
        for kit in [&SWAMP, &SNOW, &SPARGUS] {
            for spec in [House { half: 3, height: 3, stilts: 0 }, House { half: 2, height: 3, stilts: 3 }] {
                let (frame, blocks) = flat_house(kit, spec);
                for name in blocks.values() {
                    assert!(registries.blocks.parse_state(name).is_ok(), "{name}");
                }
                let w = spec.half;
                // A wall in the side's top row, under the roof: joined both
                // ways, tall, no post (a straight run).
                let side_wall = blocks[&frame.at(-1, w, spec.height - 1)];
                assert!(side_wall.contains("up=false"), "{side_wall}");
                assert!(side_wall.matches("tall").count() == 2, "{side_wall}");
                // The wall by the door: a post at its end.
                let by_door = blocks[&frame.at(w, 1, 0)];
                assert!(by_door.contains("up=true"), "{by_door}");
                assert!(blocks[&frame.at(w, 0, 0)].contains("half=lower") && blocks[&frame.at(w, 0, 1)].contains("half=upper"));
                assert!(blocks[&frame.at(w, 0, 0)].contains("facing=north"), "the door faces in");
                // The roof's tiles rise from the eaves to the ridge.
                assert!(blocks[&frame.at(0, w + 1, spec.height - 1)].starts_with(kit.roof));
                assert_eq!(blocks[&frame.at(0, 0, spec.height + w)], kit.ridge);
                // In by the door: before it, the door, then air to the middle.
                for u in 0..w {
                    assert_eq!(blocks[&frame.at(u, 0, 0)], "minecraft:air", "inside at {u}");
                    assert_eq!(blocks[&frame.at(u, 0, 1)], "minecraft:air", "headroom at {u}");
                }
                if spec.stilts > 0 {
                    assert!(blocks[&frame.at(w + 2, 0, -1)].starts_with("minecraft:ladder"), "the ladder up to the porch");
                    assert_eq!(blocks[&frame.at(w + 1, 0, -1)], kit.floor, "the porch");
                }
            }
        }
    }

    /// The forts and huts stand in their lands of the world (seed 12345):
    /// one of each near the origin, its pieces there.
    #[test]
    fn forts_and_huts_are_in_their_lands() {
        let registries = registries().expect("the Minecraft data");
        for sites in [&SNOW_FORTS, &SWAMP_HUTS] {
            let at = nearest(sites, registries.clone(), 12345, (0, 0), 4000);
            println!("{}: {at:?}", sites.name);
            assert!(at.is_some(), "a {} within 4000 blocks", sites.name);
        }
    }
}
