//! Jak 3's sparticles for the guns, read from the game's own definitions
//! (`gun-part.gc`: `defpart` and `defpartgroup`) and run the way the
//! engine runs them: 300 Hz ticks for velocity, growth, spin and fades,
//! 60 Hz frames for acceleration and friction, an `sp-item` firing
//! for its `:length` in every `:period`, children bound to a parent
//! (`:binding`) trailing it, and `:next-time`/`:next-launcher` switching a
//! particle to new rates partway through its life.
//!
//! Bevy-free: the host hands over the source text, spawns groups by name
//! where things happen and reads back camera-facing [`Sprite`]s each
//! frame, in Jak's units (4096 per meter, y up). Colours keep the PS2's
//! scale: 128 leaves the texture as it is, alpha 128 is opaque.
use std::collections::HashMap;
use std::sync::Arc;

use glam::{Mat3, Quat, Vec3};

mod callbacks;
pub mod journal;
pub mod vehicle_lights;
pub mod vehicle_effects;
pub mod wcars;
mod lightning;
use callbacks::{CurveState, Curves, TextureSequence};
pub use lightning::LightningRequest;

pub fn sprite_spline_axes(direction: Vec3, forward: Vec3, rotation: f32) -> Option<(Vec3, Vec3)> {
    let forward = forward.try_normalize()?;
    let along = (direction - forward * direction.dot(forward)).try_normalize()?;
    let side = along.cross(forward).try_normalize()?;
    let (s, c) = rotation.sin_cos();
    Some((along * c + side * s, side * c - along * s))
}

const METER: f32 = 4096.0;
/// Jak's angles: 65536 to the turn.
const ANGLE: f32 = std::f32::consts::TAU / 65536.0;
const TICKS_PER_SECOND: f32 = 300.0;
/// `defpartgroup`'s defaults in both games. A tracker obeys these;
/// a launch control explicitly kept by its owner runs until stopped.
const DEFAULT_DURATION: f32 = 3000.0;
const DEFAULT_LINGER: f32 = 1500.0;
/// More than this many live particles and new ones are skipped: the 2D
/// system's (`*sp-particle-system-2d*`, 1920 in Jak 1's `sparticle.gc`).
const MAX_PARTICLES: usize = 1920;

/// One spec value: a base, plus a random part.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Value {
    /// `base + random(0..range)`.
    Range(f32, f32),
    /// `base + random_int(0..count) * step`.
    Steps(f32, f32, f32),
    /// The value another field took.
    Copy(Field),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
enum Field {
    Num,
    X,
    Y,
    Z,
    ScaleX,
    ScaleY,
    RotZ,
    /// A flat (3D) one's turn in its plane.
    RotY,
    RotX,
    R,
    G,
    B,
    A,
    VelX,
    VelY,
    VelZ,
    ScalevelX,
    ScalevelY,
    RotvelZ,
    RotvelX,
    RotvelY,
    FadeR,
    FadeG,
    FadeB,
    FadeA,
    AccelX,
    AccelY,
    AccelZ,
    Friction,
    Timer,
    NextTime,
    ConerotX,
    ConerotY,
    ConerotZ,
    ConerotRadius,
    RotateX,
    RotateY,
    RotateZ,
    Omega,
    LaunchRotX,
    LaunchRotY,
    LaunchRotZ,
    UserData,
}

impl Field {
    fn parse(name: &str) -> Option<Self> {
        Some(match name {
            "num" => Self::Num,
            "x" => Self::X,
            "y" => Self::Y,
            "z" => Self::Z,
            "scale-x" => Self::ScaleX,
            "scale-y" => Self::ScaleY,
            "rot-z" => Self::RotZ,
            "rot-y" => Self::RotY,
            "rot-x" => Self::RotX,
            "r" => Self::R,
            "g" => Self::G,
            "b" => Self::B,
            "a" => Self::A,
            "vel-x" => Self::VelX,
            "vel-y" => Self::VelY,
            "vel-z" => Self::VelZ,
            "scalevel-x" => Self::ScalevelX,
            "scalevel-y" => Self::ScalevelY,
            "rotvel-z" => Self::RotvelZ,
            "rotvel-x" => Self::RotvelX,
            "rotvel-y" => Self::RotvelY,
            "userdata" => Self::UserData,
            "fade-r" => Self::FadeR,
            "fade-g" => Self::FadeG,
            "fade-b" => Self::FadeB,
            "fade-a" => Self::FadeA,
            "accel-x" => Self::AccelX,
            "accel-y" => Self::AccelY,
            "accel-z" => Self::AccelZ,
            "friction" => Self::Friction,
            "timer" => Self::Timer,
            "next-time" => Self::NextTime,
            "conerot-x" => Self::ConerotX,
            "conerot-y" => Self::ConerotY,
            "conerot-z" => Self::ConerotZ,
            "conerot-radius" => Self::ConerotRadius,
            "rotate-x" => Self::RotateX,
            "rotate-y" => Self::RotateY,
            "rotate-z" => Self::RotateZ,
            "omega" => Self::Omega,
            "launchrot-x" => Self::LaunchRotX,
            "launchrot-y" => Self::LaunchRotY,
            "launchrot-z" => Self::LaunchRotZ,
            _ => return None,
        })
    }
}

/// How a sprite goes over what is behind it. A particle's `gs-alpha` is
/// its adgif's (`particle-setup-adgif`: blended) unless its flags change
/// it as `sp-launch-particles-var` does after `particle-adgif` (OpenGOAL's
/// mips2c of it, Jak 1 and Jak 3 alike: `sp-cpuinfo-flag-4` takes 66,
/// then `sp-cpuinfo-flag-3` 72 over it); `glow` parts go to Jak 2 and 3's
/// glow renderer instead. Ordered so the later rule wins.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Blend {
    /// (Cs − Cd)·As + Cd (`gs-alpha` b 1 d 1, OpenGOAL's `SRC_DST_SRC_DST`:
    /// SRC_ALPHA, ONE_MINUS_SRC_ALPHA).
    #[default]
    Normal,
    /// Cd − Cs·As (66: a 2 b 0 c 0 d 1, `ZERO_SRC_SRC_DST`: SRC_ALPHA, ONE,
    /// reverse subtract). `sp-cpuinfo-flag-4`, Jak 1's `bit4`.
    Subtract,
    /// Cs·As + Cd (72: a 0 b 2 c 0 d 1, `SRC_0_SRC_DST`: SRC_ALPHA, ONE).
    /// `sp-cpuinfo-flag-3`, Jak 1's `bit3`.
    Add,
    /// The glow renderer's (`glow`): the texture's colour times the
    /// particle's colour and alpha, doubled, added on (ONE, ONE); the
    /// texture's alpha unused.
    Glow,
}

/// A `defpart`: its texture, its fields in the order written, its flags.
#[derive(Clone, Debug, Default)]
pub struct Part {
    pub texture: Option<String>,
    texture_page: Option<String>,
    fields: Vec<(Field, Value)>,
    /// How it is drawn over what is behind it (its flags).
    pub blend: Blend,
    /// `:rot-x` of a 2D sprite: the sprite's `flag`, which corner table
    /// its quad takes (sprite3_3d's `xy_array`): 0 centred on it, 4 from it
    /// out along its turn (a ray: a pickup's flares).
    pub corner: u8,
    /// `sparticle-motion-blur`: stretched along its velocity.
    pub motion_blur: bool,
    /// `launch-along-z`: its velocity follows the launch's z axis.
    pub along_z: bool,
    /// `left-multiply-quat`: a 3D particle turned by its launch's own
    /// turn too (`sp-relaunch-particle-3d`).
    left_multiply: bool,
    /// `:rot-x` as a 3D particle's turn about x (a 2D one's is `corner`).
    rot_x: f32,
    /// Jak 1's bit7 and Jak 2/3's bit6 invoke `sp-orbiter`.
    orbit: bool,
    /// `distort`: drawn by the sprite distorter (`sprite-distort.gc`), the
    /// picture behind it bent, not as a sprite: its `r`, `g` and `b` the
    /// radii (outer, inner, and where the inner ring takes its picture
    /// from), its `rot-x` the slices of its circle.
    distort: bool,
    jak1: bool,
    global_accel: bool,
    next_launcher: Option<u32>,
    birth_func: Option<String>,
    live_func: Option<String>,
    sequence: Option<TextureSequence>,
    userdata: f32,
}

#[derive(Clone, Debug)]
struct Item {
    part: u32,
    period: Option<f32>,
    length: Option<f32>,
    offset: f32,
    binding: Option<u32>,
    /// `start-dead` (Jak 3's `sp2`): not launched on its own, only once a
    /// particle of an item binding it (`:binding`) takes it, then from
    /// that particle while it lives.
    bound: bool,
    /// `launch-asap` (Jak 3's `sp3`): its particles launched once, not
    /// every frame.
    asap: bool,
    /// `is-3d`: its particles are quads in the world, turned by their
    /// rotations, not sprites facing the camera.
    is_3d: bool,
}

/// A `defpartgroup`.
#[derive(Clone, Debug)]
pub struct Group {
    pub name: String,
    pub id: u32,
    /// Explicit duration in ticks (none: the macro's 3000-tick default).
    pub duration: Option<f32>,
    /// Ticks it stays after (`:linger-duration`): its bound items still
    /// launching from their particles, its trackers still followed.
    pub linger: Option<f32>,
    items: Vec<Item>,
}

/// Every part and group read from the source.
#[derive(Clone, Debug, Default)]
pub struct Library {
    preserve_texture_pages: bool,
    parts: HashMap<u32, Part>,
    groups: HashMap<String, Group>,
    texture_names: HashMap<u32, String>,
    curves: Curves,
    trackers:callbacks::TrackerSpecs,
    water_drop_parts:HashMap<String,Vec<(u32,bool)>>,
    lightning: HashMap<String, lightning::Spec>,
}

impl Library {
    /// Exported pages can contain different sprites with the same basename.
    pub fn preserve_texture_pages(&mut self){self.preserve_texture_pages=true;}
    /// Reads `defpart` and `defpartgroup` forms from Jak 3 source text;
    /// anything else is skipped. Several files can be read into one.
    pub fn read(&mut self, source: &str) {
        self.read_for_game(source, false);
    }

    fn read_for_game(&mut self, source: &str, jak1: bool) {
        let forms = parse(source);
        for form in &forms {
            let Sexp::List(items) = form else { continue };
            match items.first().and_then(Sexp::symbol) {
                Some("defpart") => {
                    if let Some((id, mut part)) = read_part(items) {
                        part.jak1 = jak1;
                        if self.preserve_texture_pages&&!jak1 {
                            if let (Some(name),Some(page))=(&part.texture,&part.texture_page){if !name.contains(':')&&!name.contains('/'){part.texture=Some(format!("{page}/{name}"));}}
                        }
                        self.parts.insert(id, part);
                    }
                }
                Some("defpartgroup") => {
                    if let Some(group) = read_group(items) {
                        self.groups.insert(group.name.clone(), group);
                    }
                }
                _ => {}
            }
        }
        self.curves.read(&forms);
        self.trackers.read(&forms);
        for name in ["check-water-level-drop","check-water-level-drop-motion"]{
            if let Some(parts)=callbacks::water_drop_parts(&forms,name){self.water_drop_parts.insert(name.into(),parts);}
        }
        lightning::read(&forms, &mut self.lightning);
    }

    /// Reads Jak 1 source as [`Self::read`] does, its textures named for
    /// Jak 1's own export: `jak1:<name>` on its `effects` page,
    /// `jak1:<page>/<name>` on another (`HUD`'s).
    pub fn read_jak1(&mut self, source: &str) {
        self.read_paged(source,"jak1",true);
    }

    /// Jak II's particle IDs and sprite pages belong to its own library.
    pub fn read_jak2(&mut self, source: &str) {
        self.read_paged(source,"jak2",false);
    }

    fn read_paged(&mut self, source:&str, game:&str, jak1:bool) {
        const AT: &str = "(:texture (";
        let mut text = String::with_capacity(source.len() + 1024);
        let mut rest = source;
        while let Some(found) = rest.find(AT) {
            let start = found + AT.len();
            text.push_str(&rest[..start]);
            rest = &rest[start..];
            let Some(close) = rest.find(')') else { break };
            let mut words = rest[..close].split_whitespace();
            // (Not a `texture-id` written out: `(new 'static 'texture-id ...)`.)
            if let (Some(name), Some(page)) = (words.next(), words.next())
                && name != "new"
            {
                if page == "effects" && jak1 {
                    text.push_str(&format!("{game}:{name} {page}"));
                } else {
                    text.push_str(&format!("{game}:{page}/{name} {page}"));
                }
                rest = &rest[close..];
            }
        }
        text.push_str(rest);
        self.read_for_game(&text, jak1);
    }

    pub fn read_texture_names(&mut self, pages: &str, textures: &str) {
        self.read_texture_names_for(pages,textures,None);
    }
    pub fn read_jak2_texture_names(&mut self,pages:&str,textures:&str){self.read_texture_names_for(pages,textures,Some("jak2"));}
    fn read_texture_names_for(&mut self,pages:&str,textures:&str,game:Option<&str>){
        let pages: HashMap<String, u32> = parse(pages)
            .iter()
            .filter_map(|form| {
                let Sexp::List(items) = form else { return None };
                (items.first()?.symbol()? == "defconstant").then_some(())?;
                Some((
                    items.get(1)?.symbol()?.to_owned(),
                    items.get(2)?.number()? as u32,
                ))
            })
            .collect();
        for form in parse(textures) {
            let Sexp::List(items) = form else { continue };
            if items.first().and_then(Sexp::symbol) != Some("def-tex") {
                continue;
            }
            let (Some(name), Some(page), Some(index)) = (
                items.get(1).and_then(Sexp::symbol),
                items.get(2).and_then(Sexp::symbol),
                items.get(3).and_then(Sexp::number),
            ) else {
                continue;
            };
            if let Some(id) = pages.get(page) {
                self.texture_names
                    .insert((id << 20) | ((index as u32) << 8), game.map_or_else(||if self.preserve_texture_pages{format!("{page}/{name}")}else{name.to_owned()},|game|format!("{game}:{page}/{name}")));
            }
        }
    }

    pub fn unsupported_callbacks(&self) -> Vec<(u32, &str)> {
        let mut out = Vec::new();
        for (&id, part) in &self.parts {
            for (birth, name) in [
                (true, part.birth_func.as_deref()),
                (false, part.live_func.as_deref()),
            ] {
                if let Some(name) = name.filter(|name| !callbacks::supported(name, birth)) {
                    out.push((id, name));
                }
            }
        }
        out.sort_unstable();
        out
    }

    pub fn has_lightning(&self, name: &str) -> bool {
        self.lightning.contains_key(name)
    }

    /// Bolt `id` of lightning `name`: its `duration` and its whole life
    /// with the fade, in ticks at 300 Hz (the same draw the bolt makes).
    pub fn lightning_life(&self, name: &str, id: u64) -> Option<(f32, f32)> {
        self.lightning.get(name).map(|spec| lightning::life_ticks(spec, id))
    }

    /// Whether part `id` was read.
    pub fn has_part(&self, id: u32) -> bool {
        self.parts.contains_key(&id)
    }

    pub fn group(&self, name: &str) -> Option<&Group> {
        self.groups.get(name)
    }

    pub fn group_count(&self) -> usize {
        self.groups.len()
    }

    pub fn part_count(&self) -> usize {
        self.parts.len()
    }

    /// Every texture the parts draw with (names in `level-default-sprite`).
    pub fn textures(&self) -> Vec<String> {
        let mut names: Vec<_> = self
            .parts
            .values()
            .filter_map(|p| p.texture.clone())
            .collect();
        for id in self
            .parts
            .values()
            .filter_map(|p| p.sequence.as_ref())
            .flat_map(|s| s.frames.iter())
        {
            if let Some(name) = self.texture_names.get(id) {
                names.push(name.clone());
            }
        }
        for spec in self.lightning.values() {
            if let Some(name) = self.texture_names.get(&spec.texture) {
                names.push(name.clone());
            }
        }
        names.sort();
        names.dedup();
        names
    }
}

/// A particle as drawn: a sprite facing the camera.
#[derive(Clone, Debug, PartialEq)]
pub struct Sprite {
    /// Centre, Jak units.
    pub pos: Vec3,
    /// Full width and height, Jak units.
    pub size: [f32; 2],
    /// Turn in the screen's plane, radians.
    pub rotation: f32,
    /// PS2 scale: 128 is the texture's own colour, alpha 128 opaque.
    pub rgba: [f32; 4],
    pub texture: String,
    /// How it goes over what is behind it.
    pub blend: Blend,
    /// Its quad's corners (see [`Part::corner`]).
    pub corner: u8,
    /// Stretched along this direction (world), its width along it.
    pub stretch: Option<Vec3>,
    /// Lying flat in the plane with this normal (world), not facing the
    /// camera: a part of the 3D particle system (`is-3d`, the board's glow
    /// under it, a wake on the water).
    pub flat: Option<Vec3>,
    pub screen_aligned: bool,
}

/// One distort sprite (`sprite-aux-list`'s `distort` entries).
#[derive(Clone, Copy, Debug)]
pub struct Distorter {
    /// Centre, Jak units.
    pub pos: Vec3,
    /// Outer radius (its picture its own), inner radius, and the radius the
    /// inner ring takes its picture from: Jak units.
    pub radii: [f32; 3],
    /// Slices of its circle, 3 to 11.
    pub turns: u8,
}

#[derive(Clone, Debug)]
struct Particle {
    part: u32,
    launcher: u32,
    pos: Vec3,
    vel: Vec3,
    accel: Vec3,
    friction: f32,
    scale: [f32; 2],
    scalevel: [f32; 2],
    rot: f32,
    rotvel: f32,
    turnvel:Vec3,
    rgba: [f32; 4],
    fade: [f32; 4],
    timer: f32,
    next_time: Option<f32>,
    /// The bound child part this particle trails, with its rate.
    trail: Option<u32>,
    /// Its own number, for a bound item to follow it.
    id: u64,
    /// The tag of the launch it came from (a `part-tracker`'s): moved with
    /// it by `sparticle-track-root`.
    tag: Option<u64>,
    /// Launched by `draw-beam`: laid along this direction, its scale-y
    /// the length.
    beam: Option<Vec3>,
    /// Launched flat (the 3D particle system): its plane's normal.
    flat: Option<Vec3>,
    /// Killed on falling below this height (rain's `check-drop-level-rain`:
    /// the ground under where it was born), noted as landed.
    floor: Option<f32>,
    age: f32,
    texture_offset: usize,
    curve: Option<CurveState>,
    source: Option<u64>,
    origin: Vec3,
    omega: f32,
    converge: Option<Vec3>,
    quad: Option<Mat3>,
    /// Its rotations as launched (`rot-x/y/z`, radians), the frame it was
    /// launched in (`rotate-x/y/z` then the launch's matrix), and whether a
    /// relaunch keeps its turn (`left-multiply-quat`): an `is-3d` item's
    /// quad is made of these.
    turn: Vec3,
    frame: Mat3,
    keeps_turn: bool,
    /// Launched by an `is-3d` item (relaunched as `sp-relaunch-particle-3d`).
    three_d: bool,
    owner: Option<u64>,
    parent: Option<u64>,
    orbit: Option<Orbit>,
    live_func: Option<String>,
    fading: bool,
    corner:u8,
    relative_pose:(Vec3,Mat3),
}

#[derive(Clone, Debug)]
struct Orbit {
    radius: f32,
    rotation: Quat,
    centre: Vec3,
}

#[derive(Clone, Debug)]
struct Launch {
    id: u64,
    continuous: bool,
    group: String,
    pos: Vec3,
    basis: Mat3,
    age: f32,
    /// Frames' worth of launching not yet done, per item.
    carry: Vec<f32>,
    /// A caller's tag, for a group that goes where its caller says (a
    /// `part-tracker` following its target).
    tag: Option<u64>,
    /// Per item: the particle a bound item is bound to (its launch
    /// state's `origin`), and whether a `launch-asap` item has launched.
    bound: Vec<Option<u64>>,
    fired: Vec<bool>,
    ranges:Vec<(u32,Field,f32,f32)>,
}

impl Launch {
    fn duration(&self, group: &Group) -> Option<f32> {
        (!self.continuous).then_some(group.duration.unwrap_or(DEFAULT_DURATION))
    }
}

/// What a caller writes into a part for one launch, as Jak 3 code sets a
/// part's init-specs before `sp-launch-particles-var` (`vehicle-method-78`
/// sets its exhaust's number and alpha from the engine's power).
#[derive(Clone, Copy, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct Overrides {
    /// `:num`'s base (its random range the part's own).
    pub num: Option<f32>,
    /// `:a`'s base and random range.
    pub alpha: Option<(f32, f32)>,
    /// `birth-func-set-vel`: every particle starts with `*particle-vel*`
    /// (Jak units a 300 Hz tick, world axes).
    pub vel: Option<Vec3>,
    /// `:scale-x` and `:scale-y`, each its base and as much again at
    /// random (`update-rain` sets both so).
    pub scale: Option<[f32; 2]>,
    /// `spt-timer` set before the launch (ticks at 300 Hz): the shock
    /// effects' glow lasts as long as its bolt.
    pub timer: Option<f32>,
}

pub(crate) struct ProjectileSource {
    pub id: u64,
    pub at: Vec3,
    pub parts: &'static [u32],
    pub charge: Option<f32>,
}

/// The live particles.
#[derive(Clone, Debug)]
pub struct Particles {
    journal: Option<Vec<journal::Command>>,
    journal_depth: u32,
    library: Arc<Library>,
    launches: Vec<Launch>,
    live: Vec<Particle>,
    seed: u32,
    /// Particles that fell to their floor since last taken: part and where.
    landed: Vec<(u32, Vec3)>,
    roots: HashMap<u64, Vec3>,
    source: Option<u64>,
    target: Option<(Vec3, Vec3)>,
    bolts: Vec<lightning::Bolt>,
    ground: Option<f32>,
    /// The view's eye (`math-camera-pos`), Jak space, for callbacks that
    /// look at the camera; none until the host says.
    camera: Option<Vec3>,
    /// The view's forward and up and the tangent of half its vertical
    /// field of view (Jak space), for `point-off-screen`.
    view: Option<(Vec3, Vec3, f32)>,
    /// Parts' fields set from outside ([`Self::set_part_value`]).
    overrides: Vec<(u32, Field, f32)>,
    ranges:Vec<(u32,Field,f32,f32)>,
    target_orientation:Option<Mat3>,
    source_basis:HashMap<u64,Mat3>,
    particle_velocity:Vec3,
    source_carry: HashMap<(u64, u32), f32>,
    /// The last particle's own number (for bindings to name it).
    next_id: u64,
    next_launch: u64,
    step_carry: f64,
    /// The light trails being tracked ([`crate::light_trail`]), the water's wake.
    trails: Vec<crate::light_trail::Trail>,
}

impl Particles {
    /// The live particles and launchers carried into a new frame (see
    /// [`crate::reframe`]); their fall stays the frame's down.
    pub fn reframe(&mut self, frame: &crate::reframe::Frame) {
        for p in &mut self.live {
            p.pos = frame.point(p.pos);
            p.relative_pose=(frame.point(p.relative_pose.0),frame.mat3(p.relative_pose.1));
            p.origin = frame.point(p.origin);
            p.converge = p.converge.map(|v| frame.dir(v));
            p.quad = p.quad.map(|m| frame.mat3(m));
            if p.orbit.is_none() { p.vel = frame.dir(p.vel); }
            if let Some(orbit) = &mut p.orbit {
                orbit.centre = frame.point(orbit.centre);
                orbit.rotation = Quat::from_mat3(&frame.mat3(Mat3::from_quat(orbit.rotation)));
            }
            p.beam = p.beam.map(|v| frame.dir(v));
        }
        for l in &mut self.launches {
            l.pos = frame.point(l.pos);
            l.basis = frame.mat3(l.basis);
        }
        self.particle_velocity=frame.dir(self.particle_velocity);
        self.target_orientation=self.target_orientation.map(|m|frame.mat3(m));
        for m in self.source_basis.values_mut(){*m=frame.mat3(*m);}
        for root in self.roots.values_mut() {
            *root = frame.point(*root);
        }
        if let Some((root, muzzle)) = self.target.as_mut() {
            *root = frame.point(*root);
            *muzzle = frame.point(*muzzle);
        }
        for bolt in &mut self.bolts {
            bolt.reframe(frame);
        }
        for trail in &mut self.trails {
            trail.reframe(frame);
        }
    }

    pub fn new(library: Arc<Library>) -> Self {
        Self {
            journal: None, journal_depth: 0,
            library,
            launches: Vec::new(),
            live: Vec::new(),
            seed: 0x9e37_79b9,
            landed: Vec::new(),
            roots: HashMap::new(),
            source: None,
            target: None,
            bolts: Vec::new(),
            ground: None,
            camera: None,
            view: None,
            overrides: Vec::new(),
            ranges:Vec::new(),
            target_orientation:None,
            source_basis:HashMap::new(),
            particle_velocity:Vec3::ZERO,
            source_carry: HashMap::new(),
            next_id: 0,
            next_launch: 0,
            step_carry: 0.0,
            trails: Vec::new(),
        }
    }

    fn set_target_original(&mut self, root: Vec3, muzzle: Vec3) {
        self.target = Some((root, muzzle));
    }

    /// The camera's eye this frame (Jak space).
    pub fn set_camera(&mut self, eye: Vec3) {
        self.camera = eye.is_finite().then_some(eye);
    }

    /// The camera's look this frame (Jak space): forward, up, and its
    /// vertical field of view (degrees).
    pub fn set_view(&mut self, forward: Vec3, up: Vec3, fov_degrees: f32) {
        self.view = (forward.is_finite() && up.is_finite()).then(|| (forward, up, (fov_degrees.to_radians() * 0.5).tan()));
    }

    /// `point-off-screen` for `target` (Jak space): where, on the upright
    /// plane through it facing the camera, the screen point 280 of its 208
    /// half-lines over the middle lies (`reverse-transform-point!` of
    /// `(0 -280 0)`): above the top of the screen.
    pub fn point_off_screen(&self, target: Vec3) -> Option<Vec3> {
        let eye = self.camera?;
        let (forward, up, tan_half) = self.view?;
        let normal = Vec3::new(eye.x - target.x, 0.0, eye.z - target.z).try_normalize()?;
        let ray = forward + up * (280.0 / 208.0 * tan_half);
        let along = normal.dot(target - eye) / normal.dot(ray);
        (along.is_finite() && along > 0.0).then(|| eye + ray * along)
    }

    /// `birth-func-target-orient`: align to the contact normal, then use
    /// the target's world heading. The callback adds its half turn/user angle.
    fn set_target_orientation_original(&mut self,normal:Vec3,yaw:f32) {
        if let Some(normal)=normal.try_normalize().filter(|n|n.is_finite()) {
            self.target_orientation=Some(Mat3::from_quat(Quat::from_rotation_arc(Vec3::Y,normal)*Quat::from_rotation_y(yaw)));
        }
    }

    /// The caller's `*particle-vel*`, already in world axes as in GOAL.
    fn set_particle_velocity_original(&mut self,velocity:Vec3) {
        if velocity.is_finite(){self.particle_velocity=velocity;}
    }

    fn set_target_ground_original(&mut self, height: f32) {
        self.ground = height.is_finite().then_some(height);
    }

    fn set_sources_original(&mut self, sources: impl IntoIterator<Item = (u64, Vec3)>) {
        self.roots.clear();
        self.roots.extend(sources);
        self.source_basis.retain(|id,_|self.roots.contains_key(id));
        self.source_carry
            .retain(|(id, _), _| self.roots.contains_key(id));
    }

    fn launch_from_original(&mut self, id: u32, source: u64, pos: Vec3, basis: Mat3) {
        self.roots.insert(source,pos);self.source_basis.insert(source,rigid_basis(basis));
        self.source = Some(source);
        self.launch(id, pos, basis);
        self.source = None;
    }

    fn launch_peacemaker_original(&mut self, source: u64, at: Vec3, charge: f32) {
        let charge = charge.clamp(0.0, 1.0);
        let library = self.library.clone();
        self.source = Some(source);
        for id in [258, 259, 260, 261] {
            let count = if id == 258 { 0.1 + 0.9 * charge } else { 1.0 };
            let carry = self.source_carry.entry((source, id)).or_insert(0.0);
            *carry += count;
            let whole = carry.floor() as usize;
            *carry -= whole as f32;
            for _ in 0..whole {
                let before = self.live.len();
                self.birth(&library, id, None, at, Mat3::IDENTITY);
                if let Some(p) = self.live.get_mut(before) {
                    let (base, range, alpha) = match id {
                        258 => (0.1 + 0.8 * charge, 0.0, None),
                        259 => (0.1 + 1.9 * charge, 0.0, None),
                        260 => (0.1 + 2.15 * charge, 0.1, Some(32.0 * charge)),
                        _ => (0.1 + 7.9 * charge, 0.1, Some(16.0 * charge)),
                    };
                    let width = (base + random(&mut self.seed) * range) * METER;
                    if let Some(x) = p.scale.first_mut() {
                        *x = width;
                    }
                    if id != 258 {
                        if let Some(y) = p.scale.get_mut(1) {
                            *y = width;
                        }
                    }
                    if let (Some(alpha), Some(a)) = (alpha, p.rgba.get_mut(3)) {
                        *a = alpha;
                    }
                }
            }
        }
        self.source = None;
    }

    fn sync_lightning_original(&mut self, requests: Vec<LightningRequest>) {
        self.bolts
            .retain(|bolt| requests.iter().any(|r| r.id == bolt.request.id));
        for request in requests {
            if let Some(bolt) = self.bolts.iter_mut().find(|b| b.request.id == request.id) {
                bolt.request = request;
            } else if let Some(spec) = self.library.lightning.get(request.spec) {
                self.bolts.push(lightning::Bolt::new(request, spec));
            }
        }
    }

    pub fn library(&self) -> &Arc<Library> {
        &self.library
    }

    /// Starts the group `name` at `pos` with `basis` as its frame (x right,
    /// y up, z ahead; a surface's normal as y, a gun's aim as z). Unknown
    /// groups are ignored.
    fn spawn_original(&mut self, name: &str, pos: Vec3, basis: Mat3) {
        let Some(group) = self.library.group(name) else {
            return;
        };
        let carry = vec![0.0; group.items.len()];
        self.next_launch += 1;
        self.launches.push(Launch {
            id: self.next_launch,
            continuous: false,
            group: name.to_owned(),
            pos,
            basis,
            age: 0.0,
            carry,
            tag: None,
            bound: vec![None; group.items.len()],
            fired: vec![false; group.items.len()],
            ranges:self.ranges.clone(),
        });
    }

    /// [`Self::spawn`] as a `part-tracker`: the group launches where
    /// [`Self::move_tagged`] last put it (its target as it moves), for its
    /// own duration.
    fn spawn_tagged_original(&mut self, name: &str, pos: Vec3, basis: Mat3, tag: u64) {
        self.spawn(name, pos, basis);
        if let Some(launch) = self.launches.last_mut().filter(|l| l.group == name) {
            launch.tag = Some(tag);
        }
    }

    /// Moves the groups tagged `tag` to `pos`; how far each is through its
    /// duration (0 to 1, none without one), for a tracker that moves to
    /// its target over its life (`part-tracker-move-to-target`).
    fn move_tagged_original(&mut self, tag: u64, pos: impl Fn(f32) -> Vec3) {
        let library = self.library.clone();
        for launch in self.launches.iter_mut().filter(|l| l.tag == Some(tag)) {
            let t = library
                .group(&launch.group)
                .and_then(|g| launch.duration(g))
                .map_or(0.0, |d| (launch.age / d.max(1.0)).clamp(0.0, 1.0));
            launch.pos = pos(t);
            for p in self.live.iter_mut().filter(|p| p.owner == Some(launch.id)) {
                if callbacks::tracks_root(p.live_func.as_deref()) {
                    p.pos += launch.pos - p.origin;
                    p.origin = launch.pos;
                }
            }
        }
        // Its particles that track their tracker's root (`sparticle-track-
        // root`, `part-tracker-track-root`), there too, its launch gone or
        // not.
        for p in self.live.iter_mut().filter(|p| p.tag == Some(tag)) {
            if !self.launches.iter().any(|l| p.owner == Some(l.id))
                && callbacks::tracks_root(p.live_func.as_deref()) {
                let next = pos(1.0);
                p.pos += next - p.origin;
                p.origin = next;
            }
        }
    }

    /// Whether a group `name` tagged `tag` is still launching.
    pub fn follow_board_joint(&mut self, joint: Vec3) {
        for tag in [0xb0a2_d000, 0xb0a2_c000] {
            self.move_tagged(tag, |_| joint);
        }
    }

    pub fn tagged(&self, tag: u64, name: &str) -> bool {
        self.launches.iter().any(|l| l.tag == Some(tag) && l.group == name)
    }

    /// Stops the groups tagged `tag` launching (their particles live on).
    fn stop_tagged_original(&mut self, tag: u64) {
        for p in self.live.iter_mut().filter(|p| p.tag == Some(tag)) {
            p.fading = true;
            // An immortal shaft belongs to its launch control. Finite
            // particles, including eco's next-launcher fade, finish normally.
            if p.timer == -1.0 && p.next_time.is_none() {
                p.timer = 0.0;
            }
        }
        self.launches.retain(|l| l.tag != Some(tag));
    }

    /// A group kept going as a `launch-control` its owner spawns every
    /// frame (`(spawn (-> self part) pos)`): one launch, tagged `tag`,
    /// started again only once it ends and moved to `pos`; another group
    /// under the same tag stops (a vent touched, untouched).
    fn keep_original(&mut self, tag: u64, name: &str, pos: Vec3, basis: Mat3) {
        if !self.tagged(tag, name) {
            self.stop_tagged(tag);
            self.spawn_tagged(name, pos, basis, tag);
        }
        for launch in self.launches.iter_mut().filter(|l| l.tag == Some(tag)) {
            launch.basis = basis;
            launch.continuous = true;
        }
        self.move_tagged(tag, |_| pos);
    }

    /// A glow attached to its owner for its entire lifetime. Short native
    /// glow sprites have no tracking callback because OpenGOAL recreates
    /// them every rendered frame. Keep their old live copies on the same
    /// interpolated attachment when rendering faster than the 60 Hz clock.
    fn keep_attached_original(&mut self, tag: u64, name: &str, pos: Vec3, basis: Mat3) {
        let previous = self.launches.iter().find(|l|l.tag==Some(tag)).map(|l|l.pos);
        self.keep(tag,name,pos,basis);
        if let Some(previous)=previous {
            for p in self.live.iter_mut().filter(|p|p.tag==Some(tag)
                && !callbacks::tracks_root(p.live_func.as_deref())) {
                p.pos += pos-previous;
                p.origin += pos-previous;
            }
        }
    }

    pub fn len(&self) -> usize {
        self.live.len()
    }

    pub fn is_empty(&self) -> bool {
        self.live.is_empty() && self.trails.is_empty()
    }

    /// Advances `dt` seconds: launches, then moves, fades and retires.
    fn step_original(&mut self, dt: f32) {
        if !dt.is_finite() || dt <= 0.0 {
            return;
        }
        self.step_carry += f64::from(dt) * 60.0;
        while self.step_carry >= 1.0 - 1e-6 {
            self.step_carry = (self.step_carry - 1.0).max(0.0);
            self.step_frame(1.0 / 60.0);
        }
    }

    fn step_frame(&mut self, dt: f32) {
        // Each tracked trail's `common-trans!` (after the tracker's crumb for the frame), and its end once it is nearly
        // gone (`die`).
        let ticks = (dt * 300.0).round() as i64;
        for trail in &mut self.trails {
            trail.common_trans(ticks);
        }
        self.trails.retain(|trail| !(trail.ending && trail.crumbs() <= 1));
        let frames = dt * 60.0;
        let ticks = dt * TICKS_PER_SECOND;
        let library = self.library.clone();
        // The original checks the previous timer before decrementing it.
        // A five-tick sprite must be drawn for one frame, including the
        // frame in which its timer reaches zero.
        self.live.retain(|p| p.timer > 0.0 || p.timer == -1.0);
        for bolt in &mut self.bolts {
            if let Some(spec) = library.lightning.get(bolt.request.spec) {
                bolt.step(dt, spec);
            }
        }
        let mut launches = std::mem::take(&mut self.launches);
        // Where each particle a bound item follows is (its `origin`); those
        // born after this (this frame) are taken up next frame.
        let newest = self.next_id;
        let alive: HashMap<u64, (Vec3,f32,f32,Option<f32>,f32)> = if launches.iter().any(|l| l.bound.iter().any(Option::is_some)) {
            self.live.iter().map(|p| (p.id, (p.pos,p.rot,p.rgba[3],p.floor,p.age))).collect()
        } else {
            HashMap::new()
        };
        launches.retain_mut(|launch| {
            let Some(group) = library.group(&launch.group) else {
                return false;
            };
            let previous_ranges=std::mem::replace(&mut self.ranges,launch.ranges.clone());
            let mut active = false;
            // Its own items launch for its duration; after it, while it
            // lingers, only its bound ones (from their particles).
            let duration = launch.duration(group);
            let launching = duration.is_none_or(|d| launch.age < d);
            for (index, item) in group.items.iter().enumerate() {
                if !item.bound && !launching {
                    continue;
                }
                if item.bound {
                    // Active while the particle it is bound to lives
                    // (`sp-free-particle` frees it), launching from it.
                    let Some(parent) = launch.bound[index] else { continue };
                    if parent > newest {
                        active = true;
                        continue;
                    }
                    let Some(&(at,rotation,alpha,floor,parent_age)) = alive.get(&parent) else {
                        launch.bound[index] = None;
                        launch.fired[index] = false;
                        continue;
                    };
                    active = true;
                    // Binding chooses the emitter origin; it does not remove
                    // the item's period/length/offset. Eco's staggered bound
                    // emitters otherwise launch continuously and hit the cap.
                    let window=match(item.period,item.length){
                        (Some(period),Some(length))=>parent_age>=item.offset&&
                            (parent_age-item.offset).rem_euclid(period.max(1.0))<length.max(ticks),
                        (None,Some(length))=>parent_age>=item.offset&&parent_age<item.offset+length.max(ticks),
                        _=>parent_age>=item.offset,
                    };
                    if !window{continue;}
                    if item.asap {
                        if !launch.fired[index] {
                            launch.fired[index] = true;
                            let born = self.launch_part(&library, item.part, None, at, launch.basis);
                            self.make_3d(&library, item, born.clone(), launch.basis);
                            for p in &mut self.live[born.clone()] { p.parent = Some(parent); callbacks::bound_birth(&library,p,rotation,alpha,floor); }
                            self.bind(launch, group, item.binding, born);
                        }
                    } else {
                        launch.carry[index] += frames;
                        while launch.carry[index] >= 1.0 {
                            launch.carry[index] -= 1.0;
                            let born = self.launch_part(&library, item.part, None, at, launch.basis);
                            self.make_3d(&library, item, born.clone(), launch.basis);
                            for p in &mut self.live[born.clone()] { p.parent = Some(parent); callbacks::bound_birth(&library,p,rotation,alpha,floor); }
                            self.bind(launch, group, item.binding, born);
                        }
                    }
                    continue;
                }
                if item.asap {
                    if !launch.fired[index] && launch.age >= item.offset {
                        launch.fired[index] = true;
                        active = true;
                        let born = self.launch_part(&library, item.part, None, launch.pos, launch.basis);
                        self.make_3d(&library, item, born.clone(), launch.basis);
                        self.bind(launch, group, item.binding, born);
                    }
                    continue;
                }
                // Launching while the item's window in its period is open,
                // once a frame.
                let window = match (item.period, item.length) {
                    (Some(period), Some(length)) => {
                        let t = (launch.age - item.offset).rem_euclid(period.max(1.0));
                        launch.age >= item.offset && t < length.max(ticks)
                    }
                    (None, Some(length)) => {
                        launch.age >= item.offset && launch.age < item.offset + length.max(ticks)
                    }
                    _ => launch.age >= item.offset,
                };
                if window {
                    active = true;
                    launch.carry[index] += frames;
                    while launch.carry[index] >= 1.0 {
                        launch.carry[index] -= 1.0;
                        let born = self.launch_part(&library, item.part, None, launch.pos, launch.basis);
                        self.make_3d(&library, item, born.clone(), launch.basis);
                        self.bind(launch, group, item.binding, born);
                    }
                }
            }
            launch.age += ticks;
            // Its bound items still following their particles.
            let following = launch.bound.iter().any(Option::is_some);
            let lingering = duration.is_some_and(|d| launch.age < d + group.linger.unwrap_or(DEFAULT_LINGER));
            let scheduled = duration.is_some()
                || group
                    .items
                    .iter()
                    .enumerate()
                    .any(|(i, item)| !item.bound && (!item.asap || !launch.fired[i]));
            let owned = self.live.iter().any(|p| p.owner == Some(launch.id));
            let keep=match duration {
                Some(d) => launch.age < d + group.linger.unwrap_or(DEFAULT_LINGER),
                None => (launching && (active || scheduled)) || following || lingering || owned,
            };
            self.ranges=previous_ranges;keep
        });
        launches.append(&mut self.launches);
        self.launches = launches;

        let mut born = Vec::new();
        let mut positions: HashMap<u64, Vec3> = self.live.iter().map(|p| (p.id, p.pos)).collect();
        let owners: HashMap<u64, (Vec3, bool, Mat3)> = self.launches.iter().map(|l| {
            let launching = library.group(&l.group).is_some_and(|g| l.duration(g).is_none_or(|d| l.age <= d));
            (l.id, (l.pos, launching,rigid_basis(l.basis)))
        }).collect();
        for p in &mut self.live {
            p.age += ticks;
            p.vel += p.accel * frames;
            if p.friction != 0.0 { p.vel *= p.friction.powf(frames); }
            p.pos += p.vel * ticks;
            p.scale[0] += p.scalevel[0] * ticks;
            p.scale[1] += p.scalevel[1] * ticks;
            p.rot += p.rotvel * ticks;
            if let Some(quad)=p.quad.as_mut(){*quad=*quad*quaternion_zxy(Vec3::new(p.turnvel.x,p.turnvel.y,p.rotvel)*ticks);}
            for k in 0..4 {
                p.rgba[k] = (p.rgba[k] + p.fade[k] * ticks).clamp(0.0, 255.0);
            }
            if p.timer != -1.0 { p.timer = (p.timer - ticks).max(0.0); }
            if let Some(owner) = p.owner {
                p.fading |= owners.get(&owner).is_none_or(|(_, launching,_)| !launching);
            }
            if p.fading && p.timer == -1.0 && p.next_time.is_none() { p.timer = 0.0; }
            if p.live_func.as_deref()==Some("spt-func-relative-pos") {
                let binding=p.owner.and_then(|id|owners.get(&id).map(|&(at,_,basis)|(at,basis)))
                    .or_else(||p.source.and_then(|id|Some((*self.roots.get(&id)?,*self.source_basis.get(&id)?))));
                if let Some((at,basis))=binding {
                    let (previous,old)=p.relative_pose;let turn=basis*old.transpose();
                    p.pos=at+turn*(p.pos-previous);p.vel=turn*p.vel;p.accel=turn*p.accel;
                    p.relative_pose=(at,basis);
                }
            }
            if let Some(part) = library.parts.get(&p.part) {
                callbacks::update(part, p, &self.roots, self.target, self.camera, dt);
            }
            if let Some(curve) = &p.curve {
                curve.apply(&library.curves, p.age, &mut p.rgba, &mut p.scale);
            }
            if let Some(orbit) = &mut p.orbit {
                if let Some(at) = p.parent.and_then(|id| positions.get(&id)) {
                    orbit.centre = *at;
                }
                p.omega += p.vel.x * ticks;
                orbit.radius += p.vel.z * ticks;
                let (s, c) = (p.omega * ANGLE).sin_cos();
                let turn = Quat::from_axis_angle(Vec3::new(c, 0.0, s), p.vel.y * ANGLE * frames);
                orbit.rotation = (turn * orbit.rotation).normalize();
                p.pos = orbit.centre + orbit.rotation * Vec3::new(s * orbit.radius, 0.0, c * orbit.radius);
            }
            positions.insert(p.id, p.pos);
            if let Some(next) = p.next_time.as_mut() {
                *next -= ticks;
                if *next <= 0.0 {
                    p.next_time = None;
                    if let Some(launcher) = library
                        .parts
                        .get(&p.launcher)
                        .and_then(|part| part.next_launcher)
                    {
                        relaunch(&library, launcher, p, &mut self.seed);
                    }
                }
            }
            if let Some(child) = p.trail {
                born.push((child, p.pos, p.scale));
            }
        }
        let mut shells = Vec::new();
        let mut scorp_shells=Vec::new();
        for p in &mut self.live {
            if let Some(floor) = p.floor
                && p.pos.y < floor
                && p.vel.y < 0.0
            {
                if matches!(p.part,240|971) {
                    p.pos.y = floor;
                    p.vel.y = 0.0;
                    p.accel.y = 0.0;
                    continue;
                }
                self.landed
                    .push((p.part, Vec3::new(p.pos.x, floor, p.pos.z)));
                if let Some(part) = library.parts.get(&p.part) {
                    let child = match part.live_func.as_deref() {
                        Some("check-shell-level1") => Some((239,None)),
                        Some("check-shell-level2") => Some((240,None)),
                        Some("check-scorp-shell-level1")=>{scorp_shells.push((970,Vec3::new(p.pos.x,floor,p.pos.z)));None}
                        Some("check-scorp-shell-level2")=>{scorp_shells.push((971,Vec3::new(p.pos.x,floor,p.pos.z)));None}
                        Some(name @ ("check-water-level-drop"|"check-water-level-drop-motion"))=>{
                            for &(id,flat) in library.water_drop_parts.get(name).into_iter().flatten(){shells.push((id,Vec3::new(p.pos.x,floor,p.pos.z),Some(flat)));}
                            None
                        },
                        _ => None,
                    };
                    if let Some((child,water)) = child {
                        shells.push((child, Vec3::new(p.pos.x, floor, p.pos.z),water));
                    }
                }
                p.timer = 0.0;
            }
        }
        self.live.retain(|p| {
            (p.timer >= 0.0 || p.timer == -1.0)
                && (p.scale[0] > 0.0 || p.scalevel[0] > 0.0)
                && (p.scale[1] > 0.0 || p.scalevel[1] > 0.0)
        });
        for (child, pos, parent) in born {
            let count = library.parts.get(&child).map_or(0.0, |part| {
                part.value(Field::Num, &mut self.seed, &[]).unwrap_or(1.0)
            }) * frames;
            let whole = count.floor() as usize + usize::from(random(&mut self.seed) < count.fract());
            let inherit = library.parts.get(&child).is_some_and(|part| part.birth_func.as_deref() == Some("birth-func-inherit-size"));
            for _ in 0..whole {
                let before = self.live.len();
                self.birth(&library, child, None, pos, Mat3::IDENTITY);
                // `birth-func-inherit-size`: its size times its launcher's.
                if inherit && let Some(p) = self.live.get_mut(before) {
                    p.scale[0] *= parent[0];
                    p.scale[1] *= parent[1];
                }
            }
        }
        for (child,pos) in scorp_shells {
            let first=self.live.len();self.launch(child,pos,Mat3::IDENTITY);self.launch(974,pos,Mat3::IDENTITY);
            for p in &mut self.live[first..] {if p.part==child {p.floor=Some(pos.y);p.omega=pos.y;}}
        }
        for (child, pos, water) in shells {
            if let Some(flat)=water {
                if flat {self.launch_flat(child,pos,Mat3::IDENTITY,1.0);}
                else {let first=self.live.len();self.launch(child,pos,Mat3::IDENTITY);for p in &mut self.live[first..]{p.floor=Some(pos.y);}}
                continue;
            }
            let first = self.live.len();
            self.launch(child, pos, Mat3::IDENTITY);
            self.launch(243, pos, Mat3::IDENTITY);
            for p in self.live.get_mut(first..).into_iter().flatten() {
                if p.part == child {
                    p.floor = Some(pos.y);
                }
            }
        }
    }

    /// Launches part `id` once at `pos` in `basis` (`launch-particles`).
    fn launch_original(&mut self, id: u32, pos: Vec3, basis: Mat3) {
        let library = self.library.clone();
        self.launch_part(&library, id, None, pos, basis);
    }

    /// `draw-beam`: part `id` laid along `dir` from `from`, its middle half
    /// its scale-y out (the Blaster family's muzzle flashes).
    pub fn launch_beam(&mut self, id: u32, from: Vec3, dir: Vec3) {
        self.launch_beam_len(id, from, dir, f32::INFINITY);
    }

    /// [`Self::launch_beam`] at the size a launch sets (`vehicle-draw-thruster`
    /// sets the flame part's width and length from the thrust): `length`
    /// along `dir`, `width` across.
    fn launch_beam_sized_original(&mut self, id: u32, from: Vec3, dir: Vec3, length: f32, width: f32) {
        let Some(dir) = dir.try_normalize() else {
            return;
        };
        let library = self.library.clone();
        let first = self.live.len();
        self.launch_part(&library, id, None, from, Mat3::IDENTITY);
        for p in &mut self.live[first..] {
            p.scale = [width, length];
            p.pos = from + dir * length * 0.5;
            p.beam = Some(dir);
            p.quad = None;
        }
    }

    /// One blade of a 3D particle (`sp-system3d`, `:origin-is-matrix`): part
    /// `id` lying in the plane with `normal`, `length` along `dir` from
    /// `from` (its middle half its length out) and `width` across, as
    /// `vehicle-draw-thruster` lays its flame's three turned blades.
    fn launch_blade_original(&mut self, id: u32, from: Vec3, dir: Vec3, normal: Vec3, length: f32, width: f32) {
        let Some(dir) = dir.try_normalize() else {
            return;
        };
        let library = self.library.clone();
        let first = self.live.len();
        self.launch_part(&library, id, None, from, Mat3::IDENTITY);
        for p in &mut self.live[first..] {
            p.scale = [width, length];
            p.pos = from + dir * length * 0.5;
            p.beam = Some(dir);
            p.flat = Some(normal);
            p.quad = None;
        }
    }

    /// `sp-launch-particles-var` of part `id` with what its caller set
    /// (`overrides`) and its `birthaccum`: the number's fractions carried
    /// in `accum` from launch to launch.
    fn launch_var_original(&mut self, id: u32, pos: Vec3, basis: Mat3, overrides: Overrides, accum: &mut f32) {
        self.launch_var_on(id, pos, basis, overrides, accum, &|_| None);
    }

    /// [`Self::launch_var`], each particle given the floor under where it
    /// is born (rain's `birth-func-rain`: the ground it falls to).
    pub fn launch_var_on(
        &mut self,
        id: u32,
        pos: Vec3,
        basis: Mat3,
        overrides: Overrides,
        accum: &mut f32,
        floor: &dyn Fn(Vec3) -> Option<f32>,
    ) {
        let library = self.library.clone();
        let Some(part) = library.parts.get(&id) else {
            return;
        };
        let count = match overrides.num {
            Some(base) => {
                let range = match part.fields.iter().rev().find(|(f, _)| *f == Field::Num) {
                    Some((_, Value::Range(_, range))) => *range,
                    _ => 0.0,
                };
                base + random(&mut self.seed) * range
            }
            None => part.value(Field::Num, &mut self.seed, &[]).unwrap_or(1.0),
        };
        *accum += count;
        let whole = accum.floor().max(0.0);
        *accum -= whole;
        for _ in 0..whole as usize {
            let first = self.live.len();
            self.birth_with(&library, id, None, pos, basis, overrides.alpha);
            for p in &mut self.live[first..] {
                if let Some(vel) = overrides.vel {
                    p.vel = vel;
                }
                if let Some([x, y]) = overrides.scale {
                    p.scale = [x * (1.0 + random(&mut self.seed)), y * (1.0 + random(&mut self.seed))];
                }
                if let Some(timer) = overrides.timer {
                    p.timer = timer.trunc();
                }
                p.floor = floor(p.pos);
            }
        }
    }

    /// Particles that fell to their floor since last asked: part and where.
    pub fn take_landed(&mut self) -> Vec<(u32, Vec3)> {
        std::mem::take(&mut self.landed)
    }

    /// [`Self::launch_beam`] no longer than `length`, as `draw-main-shot`
    /// caps its beam's scale-y at how far the shot has flown.
    fn launch_beam_len_original(&mut self, id: u32, from: Vec3, dir: Vec3, length: f32) {
        let Some(dir) = dir.try_normalize() else {
            return;
        };
        // `draw-beam`: the part three times into the 3D system at the
        // beam's middle, turned to it (`forward-up->quaternion` with the
        // world's up) and each next one 60 degrees about it (`quaternion-
        // rotate-local-z!` 10922.667): three crossed blades along the shot.
        let side = Vec3::Y.cross(dir).try_normalize().unwrap_or(Vec3::X);
        let up = dir.cross(side);
        let library = self.library.clone();
        for k in 0..3 {
            let (s, c) = (k as f32 * std::f32::consts::FRAC_PI_3).sin_cos();
            let normal = up * c - side * s;
            let first = self.live.len();
            self.launch_part(&library, id, None, from, Mat3::IDENTITY);
            for p in &mut self.live[first..] {
                p.scale[1] = p.scale[1].min(length);
                p.pos = from + dir * p.scale[1] * 0.5;
                p.beam = Some(dir);
                p.flat = Some(normal);
                p.quad = None;
            }
        }
    }

    /// [`Self::launch`] into the 3D particle system: lying flat in the plane
    /// of `basis`'s y (the board's glow, a wake on the water), `count`
    /// times the part's number.
    fn launch_flat_original(&mut self, id: u32, pos: Vec3, basis: Mat3, count: f32) {
        let library = self.library.clone();
        let Some(part) = library.parts.get(&id) else {
            return;
        };
        let num = part.value(Field::Num, &mut self.seed, &[]).unwrap_or(1.0) * count;
        let whole = num.floor() as usize + usize::from(random(&mut self.seed) < num.fract());
        let normal = basis.y_axis.normalize_or(Vec3::Y);
        for _ in 0..whole {
            let first = self.live.len();
            self.birth(&library, id, None, pos, basis);
            for p in &mut self.live[first..] {
                p.flat = Some(normal);
                p.quad = None;
            }
        }
    }

    /// [`Self::launch`] with the part's starting size times `scale`, as a
    /// shot sets a spec for one launch (the Blaster's sparks are smaller
    /// while its beam is short).
    fn launch_scaled_original(&mut self, id: u32, pos: Vec3, basis: Mat3, scale: f32) {
        let library = self.library.clone();
        let first = self.live.len();
        self.launch_part(&library, id, None, pos, basis);
        for p in &mut self.live[first..] {
            p.scale = p.scale.map(|s| s * scale);
        }
    }

    fn launch_part(&mut self, library: &Library, id: u32, binding: Option<u32>, pos: Vec3, basis: Mat3) -> std::ops::Range<usize> {
        let first = self.live.len();
        let Some(part) = library.parts.get(&id) else {
            return first..first;
        };
        let count=if let Some(&(_,_,base,range))=self.ranges.iter().find(|(p,f,_,_)|*p==id&&*f==Field::Num){base+random(&mut self.seed)*range}else{part.value(Field::Num,&mut self.seed,&[]).unwrap_or(1.0)};
        let whole = count.floor() as usize + usize::from(random(&mut self.seed) < count.fract());
        for _ in 0..whole {
            self.birth(library, id, binding, pos, basis);
        }
        first..self.live.len()
    }

    /// An `is-3d` item's particles: quads, turned as Jak 3's
    /// `sp-launch-particles-var` turns them (its mips2c): the sprite's
    /// rotation slot is the launch's `conerot`, so its `rot-x/y/z` less the
    /// angles of the frame it is launched in (`sp-adjust-launch` with its
    /// 3D flag: `matrix->eul` order 13 of `rotate-x/y/z` times the launch
    /// matrix, `conerot` x, y, z less its y, z, x), made a quaternion by
    /// `sp-euler-convert` (`quaternion-zxy!`).
    fn make_3d(&mut self, _library: &Library, item: &Item, born: std::ops::Range<usize>, _basis: Mat3) {
        if !item.is_3d {
            return;
        }
        for i in born {
            let p = &mut self.live[i];
            // A birth function's own quad (`birth-func-set-quat`, beams) stays.
            if p.quad.is_some() {
                continue;
            }
            let e = euler_zxy_r(p.frame);
            p.quad = Some(quaternion_zxy(p.turn - Vec3::new(e.y, e.z, e.x)));
            p.rot = 0.0;
            p.three_d = true;
        }
    }

    /// Part `part`'s field `field` set from outside for its launches from
    /// now on (`(set! (-> *part-id-table* N init-specs I initial-valuef)
    /// ...)`, as Light Jak's heal does), or back to its own with None.
    /// An init-spec's `random-rangef` set from outside (`(set! (-> *part-id-table* 758 init-specs 2 random-rangef) ...)`):
    /// the part's authored base stays; none puts the authored range back.
    fn set_part_range_original(&mut self, part: u32, field: &str, range: Option<f32>) {
        let Some(field) = Field::parse(field) else { return };
        self.ranges.retain(|(p, f, _, _)| !(*p == part && *f == field));
        let Some(range) = range else { return };
        let authored = self.library.parts.get(&part).and_then(|p| p.fields.iter().rev().find(|(f, _)| *f == field)).map(|(_, v)| *v);
        if let Some(Value::Range(base, _)) = authored {
            self.ranges.push((part, field, base, range));
        }
    }

    fn set_part_value_original(&mut self, part: u32, field: &str, value: Option<f32>) {
        let Some(field) = Field::parse(field) else { return };
        self.overrides.retain(|(p, f, _)| !(*p == part && *f == field));
        if let Some(v) = value {
            self.overrides.push((part, field, v));
        }
    }

    /// Evaluate the original tracker's constant arithmetic and init-spec
    /// assignments against its userdata/root, retaining authored ranges.
    fn configure_tracker_original(&mut self,name:&str,userdata:f32,root:Vec3)->Result<(),String>{
        let values=self.library.trackers.values(name,userdata,root,&self.library.parts)?;
        for(part,field,base,range)in values{self.ranges.retain(|(p,f,_,_)|*p!=part||*f!=field);self.ranges.push((part,field,base,range));}
        Ok(())
    }

    /// `sp-launch-particles-var`'s binding: each particle an item with a
    /// `:binding` launches takes the first free bound item of that part in
    /// its launch (none free: none).
    fn bind(&mut self, launch: &mut Launch, group: &Group, binding: Option<u32>, born: std::ops::Range<usize>) {
        for p in &mut self.live[born.clone()] {
            p.tag = launch.tag;
            p.owner = Some(launch.id);
        }
        let Some(child) = binding else { return };
        for p in &self.live[born] {
            let free = group
                .items
                .iter()
                .enumerate()
                .find(|(j, item)| item.bound && item.part == child && launch.bound[*j].is_none());
            let Some((j, _)) = free else { return };
            launch.bound[j] = Some(p.id);
            launch.fired[j] = false;
            launch.carry[j] = 0.0;
        }
    }

    fn birth(&mut self, library: &Library, id: u32, binding: Option<u32>, at: Vec3, basis: Mat3) {
        self.birth_with(library, id, binding, at, basis, None);
    }

    fn birth_with(
        &mut self,
        library: &Library,
        id: u32,
        binding: Option<u32>,
        at: Vec3,
        basis: Mat3,
        alpha: Option<(f32, f32)>,
    ) {
        if self.live.len() >= MAX_PARTICLES {
            return;
        }
        let Some(part) = library.parts.get(&id) else {
            return;
        };
        let mut taken: Vec<(Field, f32)> = Vec::with_capacity(part.fields.len());
        for (field, _) in &part.fields {
            let v = match (field, alpha) {
                (Field::A, Some((base, range))) => Some(base + random(&mut self.seed) * range),
                _ => if let Some(&(_,_,base,range))=self.ranges.iter().find(|(p,f,_,_)|*p==id&&f==field){Some(base+random(&mut self.seed)*range)}else{match self.overrides.iter().find(|(p, f, _)| *p == id && f == field) {
                    Some(&(_, _, v)) => Some(v),
                    None => part.value(*field, &mut self.seed, &taken),
                }},
            };
            if let Some(v) = v {
                taken.push((*field, v));
            }
        }
        let get = |f: Field, default: f32| {
            taken
                .iter()
                .rev()
                .find(|(k, _)| *k == f)
                .map_or(default, |(_, v)| *v)
        };
        // sp-adjust-launch rotates velocity by launchrot and conerot;
        // conerot offsets only the cone radius, not the authored position.
        // rotate-xyz then rotates position, velocity and local acceleration.
        let xyz = |x: Field, y: Field, z: Field| {
            Mat3::from_rotation_z(get(z, 0.0) * ANGLE)
                * Mat3::from_rotation_y(get(y, 0.0) * ANGLE)
                * Mat3::from_rotation_x(get(x, 0.0) * ANGLE)
        };
        let cone = xyz(Field::ConerotX, Field::ConerotY, Field::ConerotZ);
        let launch = xyz(Field::LaunchRotX, Field::LaunchRotY, Field::LaunchRotZ);
        let frame = basis * xyz(Field::RotateX, Field::RotateY, Field::RotateZ);
        let radius = get(Field::ConerotRadius, 0.0);
        let radius_axis = if part.jak1 && part.along_z { Vec3::Z } else { Vec3::Y };
        let authored = Vec3::new(get(Field::X, 0.0), get(Field::Y, 0.0), get(Field::Z, 0.0));
        let offset = authored + cone * radius_axis * radius;
        let vel = Vec3::new(
            get(Field::VelX, 0.0),
            get(Field::VelY, 0.0),
            get(Field::VelZ, 0.0),
        );
        let accel = Vec3::new(
            get(Field::AccelX, 0.0),
            get(Field::AccelY, 0.0),
            get(Field::AccelZ, 0.0),
        );
        let next_time = taken
            .iter()
            .find(|(k, _)| *k == Field::NextTime)
            .map(|(_, v)| v.trunc());
        self.next_id += 1;
        let own = self.next_id;
        let omega = get(Field::Omega, 0.0);
        let (s, c) = (omega * ANGLE).sin_cos();
        let orbit = part.orbit.then(|| Orbit {
            radius: offset.z,
            rotation: Quat::from_mat3(&basis) * Quat::from_axis_angle(Vec3::new(c, 0.0, -s), offset.y * ANGLE),
            centre: at,
        });
        self.live.push(Particle {
            id: own,
            tag: None,
            part: id,
            launcher: id,
            pos: orbit.as_ref().map_or(at + frame * offset, |o| at + o.rotation * Vec3::new(s * o.radius, 0.0, c * o.radius)),
            vel: if part.orbit { vel } else { frame * cone * launch * vel },
            accel: if part.global_accel || (!part.jak1 && part.along_z) { accel } else { frame * accel },
            friction: get(Field::Friction, 1.0),
            scale: [
                get(Field::ScaleX, METER),
                get(Field::ScaleY, get(Field::ScaleX, METER)),
            ],
            scalevel: [get(Field::ScalevelX, 0.0), get(Field::ScalevelY, 0.0)],
            rot: get(Field::RotZ, get(Field::RotY, 0.0)) * ANGLE,
            rotvel: get(Field::RotvelZ, 0.0) * ANGLE,
            turnvel:Vec3::new(get(Field::RotvelX,0.0),get(Field::RotvelY,0.0),0.0)*ANGLE,
            rgba: [
                get(Field::R, 0.0),
                get(Field::G, 0.0),
                get(Field::B, 0.0),
                get(Field::A, 0.0),
            ],
            fade: [
                get(Field::FadeR, 0.0),
                get(Field::FadeG, 0.0),
                get(Field::FadeB, 0.0),
                get(Field::FadeA, 0.0),
            ],
            timer: get(Field::Timer, TICKS_PER_SECOND).trunc(),
            next_time,
            trail: binding,
            beam: None,
            flat: None,
            floor: None,
            age: 0.0,
            texture_offset: part
                .sequence
                .as_ref()
                .filter(|_| matches!(part.birth_func.as_deref(),Some("birth-func-texture-group" | "spt-birth-func-part-buggy-rocks" | "spt-birth-func-part-dbuggy-debris")))
                .map_or(0, |s| {
                    (random(&mut self.seed) * s.frames.len() as f32) as usize
                }),
            curve: None,
            source: self.source,
            origin: at,
            omega,
            converge: None,
            quad: None,
            turn: Vec3::new(get(Field::RotX,part.rot_x), get(Field::RotY, 0.0), get(Field::RotZ, 0.0)) * ANGLE,
            three_d: false,
            frame,
            keeps_turn: part.left_multiply,
            owner: None,
            parent: None,
            orbit,
            live_func: part.live_func.clone(),
            fading: false,
            corner:part.corner,
            relative_pose:(at,rigid_basis(basis)),
        });
        if let Some(p) = self.live.last_mut() {
            p.curve = CurveState::birth(&library.curves, id, p, &mut self.seed);
            if part.birth_func.as_deref()==Some("birth-func-flip-based-on-scale") {
                for axis in 0..2 {if p.scale[axis]<0. {p.scale[axis]=-p.scale[axis];p.corner|=16<<axis;}}
            }
            if matches!(part.birth_func.as_deref(),Some("birth-func-set-vel" | "birth-func-set-vel-2d" | "spt-birth-func-part-dbuggy-debris")) {
                p.vel=self.particle_velocity;
            }
            if matches!(part.birth_func.as_deref(),Some("spt-birth-func-part-buggy-rocks" | "spt-birth-func-part-dbuggy-debris")) {
                // wvehicle-part.gc: integer brightness choices stored in the
                // 2D launchinfo's rotate-xyz aliases for RGB, not orientation.
                let red=200.+(random(&mut self.seed)*51.).floor();
                let _unused=random(&mut self.seed);
                let green=red-20.-(random(&mut self.seed)*11.).floor();
                let blue=red-60.-(random(&mut self.seed)*11.).floor();
                p.rgba[..3].copy_from_slice(&[red,green,blue]);
            }
            if part.birth_func.as_deref() == Some("birth-func-set-alpha-from-userdata") {
                p.pos = at + frame * (authored + cone * radius_axis * radius * part.userdata);
            }
            if part.birth_func.as_deref()==Some("birth-func-target-orient") {
                if let Some(target)=self.target_orientation {
                    p.quad=Some(target*Mat3::from_rotation_y(std::f32::consts::PI+get(Field::UserData,part.userdata)*ANGLE));
                }
            }
            if part.birth_func.as_deref()==Some("birth-func-camera-orient") {
                // forward-up-nopitch->quaternion, then quaternion-rotate-x! 16384.
                if let Some((forward,_,_))=self.view {
                    let yaw=forward.x.atan2(forward.z);
                    p.quad=Some(Mat3::from_quat(Quat::from_rotation_y(yaw)*Quat::from_rotation_x(std::f32::consts::FRAC_PI_2)));
                }
            }
            if part.birth_func.as_deref() == Some("birth-func-converge") {
                if let Some((_, muzzle)) = self.target {
                    p.converge = Some(((p.pos - muzzle).normalize_or(Vec3::Y) * METER).trunc() / METER);
                }
            }
            if matches!(
                part.birth_func.as_deref(),
                Some("birth-func-set-quat" | "birth-func-setup-beam" | "birth-func-laser-pointer")
            ) {
                p.quad = Some(frame * cone);
            }
            if matches!(
                part.live_func.as_deref(),
                Some("check-shell-level1" | "check-shell-level2")
            ) {
                p.floor = self.ground;
            }
            if matches!(part.live_func.as_deref(),Some("check-scorp-shell-level1"|"check-scorp-shell-level2")) {
                p.floor=self.ground.or(Some(omega));
            }
            if part.live_func.as_deref().is_some_and(|f|f.starts_with("check-water-level-")){p.floor=Some(get(Field::UserData,part.userdata));}
        }
    }

    /// Every live particle as a sprite to draw.
    /// The `lightning-spec` bolts out this frame, drawn as `lightning-draw`
    /// draws them: two strips each, facing the camera (none before the host
    /// has told the camera, [`Self::set_view`]).
    /// The light trails' ribbons (`build-prim-strip!`), each a triangle strip.
    pub fn trail_strips(&self) -> Vec<crate::lightning_bolt::Strip> {
        self.trails.iter().map(crate::light_trail::Trail::strip).filter(|strip| !strip.verts.is_empty()).collect()
    }

    pub fn lightning_strips(&self) -> Vec<crate::lightning_bolt::Strip> {
        let Some((forward, up, _)) = self.view else {
            return Vec::new();
        };
        let right = forward.cross(up).normalize_or_zero();
        self.bolts
            .iter()
            .filter_map(|bolt| self.library.lightning.get(bolt.request.spec).map(|spec| (bolt, spec)))
            .flat_map(|(bolt, spec)| bolt.strips(spec, &self.library.texture_names, right, up))
            .collect()
    }

    /// The distorters alive (`sprite-draw-distorters`' frame data): where,
    /// their radii from their `r`, `g`, `b` (Jak units), and their slices
    /// (`rot-x`; one out of 3 to 11 made 11, as the game does).
    pub fn distorters(&self) -> Vec<Distorter> {
        self.live
            .iter()
            .filter_map(|p| {
                let part = self.library.parts.get(&p.part)?;
                part.distort.then(|| Distorter {
                    pos: p.pos,
                    radii: [p.rgba[0], p.rgba[1], p.rgba[2]],
                    turns: if (3..=11).contains(&part.corner) { part.corner } else { 11 },
                })
            })
            .collect()
    }

    pub fn sprites(&self) -> Vec<Sprite> {
        self.tagged_sprites().into_iter().map(|(_,sprite)|sprite).collect()
    }

    /// Draw identity survives collection so a pickup can scale its whole
    /// native group, including orbital offsets and finite particle tails.
    pub fn tagged_sprites(&self) -> Vec<(Option<u64>,Sprite)> {
        let sprites: Vec<_> = self
            .live
            .iter()
            .filter_map(|p| {
                let sprite=(|| {
                let part = self.library.parts.get(&p.part)?;
                let texture_age = if matches!(
                    p.live_func.as_deref(),
                    Some("sparticle-texture-animate" | "sparticle-dark-shot-lightning" | "spt-func-part-vent-eco-dark-shaft")
                ) {
                    p.age
                } else {
                    0.0
                };
                let texture = part
                    .sequence
                    .as_ref()
                    .and_then(|sequence| {
                        sequence.texture(texture_age, p.texture_offset, &self.library.texture_names)
                    })
                    .or_else(|| part.texture.clone())?;
                // These 3D callbacks replace the authored quaternion with
                // an upright plane facing the camera. Light's x axis is
                // vertical; the dark shaft's y axis is vertical.
                if matches!(p.live_func.as_deref(), Some("sparticle-3d-rotate-xz-to-camera" | "spt-func-part-vent-eco-dark-shaft")) {
                    let dark = p.live_func.as_deref() == Some("spt-func-part-vent-eco-dark-shaft");
                    return Some(Sprite {
                        screen_aligned: false, pos: p.pos,
                        size: if dark { [p.scale[1], p.scale[0]] } else { p.scale },
                        rotation: 0.0, rgba: p.rgba, texture, blend: part.blend,
                        corner: 0, stretch: Some(Vec3::Y), flat: None,
                    });
                }
                if let Some(quad) = p.quad {
                    // All three games' sprite shaders scale X/Z before
                    // applying the quaternion. RotZ is already in that
                    // quaternion; rotating the plane by it again is wrong.
                    return Some(Sprite {
                        screen_aligned: false,
                        pos: p.pos,
                        size: p.scale,
                        rotation: 0.0,
                        rgba: p.rgba,
                        texture,
                        blend: part.blend,
                        corner: p.corner,
                        stretch: Some(quad.x_axis),
                        flat: Some(-quad.y_axis),
                    });
                }
                if callbacks::spline(part) {
                    let direction = p.vel.try_normalize()?;
                    return Some(Sprite {
                        pos: p.pos,
                        size: [p.scale[1], p.scale[0]],
                        rotation: if part.live_func.as_deref() == Some("sparticle-lightning-2d-spline-align-plus-rotz") { part.value(Field::RotZ, &mut 1, &[]).unwrap_or(0.0) * ANGLE } else { 0.0 },
                        rgba: p.rgba,
                        texture,
                        blend: part.blend,
                        corner: p.corner,
                        stretch: Some(direction),
                        flat: p.flat,
                        screen_aligned: true,
                    });
                }
                Some(match p.beam {
                    // Along the beam the length is scale-y, across scale-x.
                    Some(dir) => Sprite {
                        screen_aligned: false,
                        pos: p.pos,
                        size: [p.scale[1], p.scale[0]],
                        rotation: p.rot,
                        rgba: p.rgba,
                        texture,
                        blend: part.blend,
                        corner: p.corner,
                        stretch: Some(dir),
                        // A 3D blade lies in its plane, along its beam.
                        flat: p.flat,
                    },
                    None => Sprite {
                        screen_aligned: false,
                        pos: p.pos,
                        size: p.scale,
                        rotation: p.rot,
                        rgba: p.rgba,
                        texture,
                        blend: part.blend,
                        corner: p.corner,
                        stretch: (part.motion_blur
                            && p.vel.length_squared() > 1e-6
                            && p.flat.is_none())
                        .then(|| p.vel.normalize()),
                        flat: p.flat,
                    },
                })
                })();
                sprite.map(|sprite|(p.tag,sprite))
            })
            .collect();

        sprites
    }
}

/// `:next-launcher`: the launcher's fields replace the particle's.
fn relaunch(library: &Library, id: u32, p: &mut Particle, seed: &mut u32) {
    let Some(part) = library.parts.get(&id) else {
        return;
    };
    let mut taken = Vec::new();
    for (field, _) in &part.fields {
        let Some(v) = part.value(*field, seed, &taken) else {
            continue;
        };
        taken.push((*field, v));
        match field {
            Field::ScaleX => p.scale[0] = v,
            Field::ScaleY => p.scale[1] = v,
            Field::ScalevelX => p.scalevel[0] = v,
            Field::ScalevelY => p.scalevel[1] = v,
            Field::R => p.rgba[0] = v,
            Field::G => p.rgba[1] = v,
            Field::B => p.rgba[2] = v,
            Field::A => p.rgba[3] = v,
            Field::FadeR => p.fade[0] = v,
            Field::FadeG => p.fade[1] = v,
            Field::FadeB => p.fade[2] = v,
            Field::FadeA => p.fade[3] = v,
            Field::RotvelZ => p.rotvel = v * ANGLE,
            Field::RotvelX => p.turnvel.x=v*ANGLE,
            Field::RotvelY => p.turnvel.y=v*ANGLE,
            Field::Friction => p.friction = v,
            Field::AccelX => p.accel.x = v,
            Field::AccelY => p.accel.y = v,
            Field::AccelZ => p.accel.z = v,
            Field::VelX => p.vel.x = v,
            Field::VelY => p.vel.y = v,
            Field::VelZ => p.vel.z = v,
            Field::Timer => p.timer = v.trunc(),
            Field::NextTime => p.next_time = Some(v.trunc()),
            _ => {}
        }
    }
    p.part = if part.texture.is_some() { id } else { p.part };
    if part.live_func.is_some() { p.live_func = part.live_func.clone(); }
    p.launcher = id;
    // `sp-relaunch-particle-3d`: the new part's rotations made the quad,
    // after the old turn only with `left-multiply-quat`.
    if let Some(old) = p.quad.filter(|_| p.three_d) {
        let get = |f: Field| taken.iter().rev().find(|(k, _)| *k == f).map_or(0.0, |(_, v)| *v) * ANGLE;
        let new = quaternion_zxy(Vec3::new(part.rot_x * ANGLE, get(Field::RotY), get(Field::RotZ)));
        p.quad = Some(if p.keeps_turn { old * new } else { new });
    }
}

/// `quaternion-zxy!`: about z, then x, then y.
fn quaternion_zxy(a: Vec3) -> Mat3 {
    Mat3::from_rotation_y(a.y) * Mat3::from_rotation_x(a.x) * Mat3::from_rotation_z(a.z)
}

/// `matrix->eul` with order 13 (`EulOrdZXYr`): Shoemake's, as `euler.gc`
/// has it, on the rows of the game's matrix (`rvec uvec fvec`, the
/// columns of ours).
fn euler_zxy_r(f: Mat3) -> Vec3 {
    let m = |r: usize, c: usize| f.col(r)[c];
    let cy = (m(1, 1) * m(1, 1) + m(0, 1) * m(0, 1)).sqrt();
    let (x, y, z) = if cy > 1e-11 {
        (m(2, 0).atan2(m(2, 2)), (-m(2, 1)).atan2(cy), m(0, 1).atan2(m(1, 1)))
    } else {
        ((-m(0, 2)).atan2(m(0, 0)), (-m(2, 1)).atan2(cy), 0.0)
    };
    // Odd parity: all three negated; a rotating frame: x and z swapped.
    Vec3::new(-z, -y, -x)
}

impl Part {
    fn value(&self, field: Field, seed: &mut u32, taken: &[(Field, f32)]) -> Option<f32> {
        let (_, value) = self.fields.iter().rev().find(|(f, _)| *f == field)?;
        Some(match *value {
            Value::Range(base, range) => base + random(seed) * range,
            Value::Steps(base, count, step) => base + (random(seed) * (count + 1.0)).floor().min(count.max(0.0)) * step,
            Value::Copy(other) => taken.iter().rev().find(|(f, _)| *f == other)?.1,
        })
    }
}

fn random(seed: &mut u32) -> f32 {
    *seed ^= *seed << 13;
    *seed ^= *seed >> 17;
    *seed ^= *seed << 5;
    (*seed >> 8) as f32 / (1u32 << 24) as f32
}

// ---- reading the source ----

#[derive(Clone, Debug)]
enum Sexp {
    Atom(String),
    List(Vec<Sexp>),
}

impl Sexp {
    fn symbol(&self) -> Option<&str> {
        match self {
            Self::Atom(s) => Some(s.trim_start_matches('\'')),
            Self::List(_) => None,
        }
    }

    fn number(&self) -> Option<f32> {
        match self {
            Self::Atom(s) => {
                if let Some(hex) = s.strip_prefix("#x") {
                    i64::from_str_radix(hex, 16).ok().map(|v| v as f32)
                } else {
                    s.parse::<f32>().ok()
                }
            }
            // (meters x), (degrees x), (seconds x)
            Self::List(items) => {
                let unit = items.first()?.symbol()?;
                let value = items.get(1)?.number()?;
                match unit {
                    "meters" => Some(value * METER),
                    "degrees" => Some(value * 65536.0 / 360.0),
                    "seconds" => Some(value * TICKS_PER_SECOND),
                    _ => None,
                }
            }
        }
    }
}

fn parse(source: &str) -> Vec<Sexp> {
    let mut stack: Vec<Vec<Sexp>> = vec![Vec::new()];
    let mut chars = source.chars().peekable();
    let mut atom = String::new();
    let flush = |atom: &mut String, stack: &mut Vec<Vec<Sexp>>| {
        if !atom.is_empty() {
            if let Some(top) = stack.last_mut() {
                top.push(Sexp::Atom(std::mem::take(atom)));
            }
        }
    };
    while let Some(c) = chars.next() {
        match c {
            ';' => {
                flush(&mut atom, &mut stack);
                for c in chars.by_ref() {
                    if c == '\n' {
                        break;
                    }
                }
            }
            '"' => {
                flush(&mut atom, &mut stack);
                let mut text = String::new();
                while let Some(c) = chars.next() {
                    match c {
                        '\\' => {
                            if let Some(n) = chars.next() {
                                text.push(n);
                            }
                        }
                        '"' => break,
                        _ => text.push(c),
                    }
                }
                if let Some(top) = stack.last_mut() {
                    top.push(Sexp::Atom(text));
                }
            }
            '(' => {
                flush(&mut atom, &mut stack);
                stack.push(Vec::new());
            }
            ')' => {
                flush(&mut atom, &mut stack);
                if stack.len() > 1 {
                    let list = stack.pop().unwrap_or_default();
                    if let Some(top) = stack.last_mut() {
                        top.push(Sexp::List(list));
                    }
                }
            }
            c if c.is_whitespace() => flush(&mut atom, &mut stack),
            c => atom.push(c),
        }
    }
    flush(&mut atom, &mut stack);
    stack.into_iter().next().unwrap_or_default()
}

/// The value after `key` in a keyword list.
fn keyword<'a>(items: &'a [Sexp], key: &str) -> Option<&'a Sexp> {
    items
        .iter()
        .position(|s| s.symbol() == Some(key))
        .and_then(|i| items.get(i + 1))
}

fn read_part(items: &[Sexp]) -> Option<(u32, Part)> {
    let id = items.get(1)?.number()? as u32;
    let Sexp::List(specs) = keyword(items, ":init-specs")? else {
        return None;
    };
    let mut part = Part::default();
    let mut stored = None;
    for spec in specs {
        let Sexp::List(spec) = spec else { continue };
        let Some(key) = spec.first().and_then(Sexp::symbol) else {
            continue;
        };
        let key = key.trim_start_matches(':');
        let args = &spec[1..];
        match key {
            "rot-x" => {
                if let Some(v) = args.first().and_then(Sexp::number) {
                    part.corner = v as u8;
                    part.rot_x = v;
                    if let Some(range)=args.get(1).and_then(Sexp::number){part.fields.push((Field::RotX,Value::Range(v,range)));}
                }
            }
            "texture" => {
                if let Some(Sexp::List(tex)) = args.first() {
                    part.texture = tex.first().and_then(Sexp::symbol).filter(|name|*name!="new").map(str::to_owned);
                    part.texture_page=tex.get(1).and_then(Sexp::symbol).map(str::to_owned);
                }
            }
            "flags" => {
                if let Some(Sexp::List(flags)) = args.first() {
                    for flag in flags.iter().filter_map(Sexp::symbol) {
                        match flag {
                            // The later rule wins (see [`Blend`]); Jak 1
                            // names the bits `bitN`.
                            "glow" => part.blend = part.blend.max(Blend::Glow),
                            "sp-cpuinfo-flag-3" | "bit3" => part.blend = part.blend.max(Blend::Add),
                            "sp-cpuinfo-flag-4" | "bit4" => part.blend = part.blend.max(Blend::Subtract),
                            "launch-along-z" => part.along_z = true,
                            "left-multiply-quat" => part.left_multiply = true,
                            "bit7" => part.orbit = true,
                            // Confirmed in Jak 3's sp-process-block-2d: mask
                            // 0x40 dispatches sp-orbiter. The export's name is
                            // historical; these x/y values encode angles.
                            "ready-to-launch" => part.orbit = true,
                            "distort" => part.distort = true,
                            "use-global-acc" => part.global_accel = true,
                            _ => {}
                        }
                    }
                }
            }
            "func" => {
                part.live_func = args.first().and_then(Sexp::symbol).map(str::to_owned);
                part.motion_blur |=
                    matches!(args.first().and_then(Sexp::symbol),Some("sparticle-motion-blur"|"check-water-level-drop-motion"|"check-water-level-drop-and-die-motion"));
            }
            "birth-func" => {
                part.birth_func = args.first().and_then(Sexp::symbol).map(str::to_owned)
            }
            "userdata" => {
                part.sequence = TextureSequence::read(args);
                part.userdata = args.first().and_then(Sexp::number).unwrap_or(0.0);
                if let Some(value)=args.first().and_then(Sexp::number){part.fields.push((Field::UserData,Value::Range(value,0.0)));}
            }
            "next-launcher" => {
                part.next_launcher = args.first().and_then(Sexp::number).map(|v| v as u32);
            }
            _ => {
                let Some(field) = Field::parse(key) else {
                    continue;
                };
                let value = if args.first().and_then(Sexp::symbol) == Some("*sp-temp*") {
                    match stored { Some(field) => Value::Copy(field), None => continue }
                } else if args.first().and_then(Sexp::symbol) == Some(":copy") {
                    match args.get(1).and_then(Sexp::symbol).and_then(Field::parse) {
                        Some(other) => Value::Copy(other),
                        None => continue,
                    }
                } else {
                    let numbers: Vec<f32> = args.iter().filter_map(Sexp::number).collect();
                    match numbers.as_slice() {
                        [base] => Value::Range(*base, 0.0),
                        [base, range] => Value::Range(*base, *range),
                        [base, count, step, ..] => {
                            let integer = matches!(args.get(1), Some(Sexp::Atom(s)) if s.parse::<i32>().is_ok());
                            if integer { Value::Steps(*base, *count, *step) }
                            else { Value::Range(*base, *count * *step) }
                        },
                        [] => continue,
                    }
                };
                part.fields.push((field, value));
                if args.iter().any(|arg| arg.symbol() == Some(":store")) { stored = Some(field); }
            }
        }
    }
    Some((id, part))
}

fn read_group(items: &[Sexp]) -> Option<Group> {
    let name = items.get(1)?.symbol()?.to_owned();
    let id = keyword(items, ":id").and_then(Sexp::number).unwrap_or(0.0) as u32;
    let duration = keyword(items, ":duration").and_then(Sexp::number).map(f32::trunc);
    let linger = keyword(items, ":linger-duration").and_then(Sexp::number).map(f32::trunc);
    let Sexp::List(parts) = keyword(items, ":parts")? else {
        return None;
    };
    let mut entries = Vec::new();
    for entry in parts {
        let Sexp::List(entry) = entry else { continue };
        if entry.first().and_then(Sexp::symbol) != Some("sp-item") {
            continue;
        }
        let Some(part) = entry.get(1).and_then(Sexp::number) else { continue };
        let rest = &entry[2..];
        let flags: Vec<String> = match keyword(rest, ":flags") {
            Some(Sexp::List(f)) => f.iter().filter_map(Sexp::symbol).map(str::to_owned).collect(),
            _ => Vec::new(),
        };
        let binding = keyword(rest, ":binding").and_then(Sexp::number).map(|v| v as u32);
        entries.push(Item {
            part: part as u32,
            period: keyword(rest, ":period").and_then(Sexp::number).map(f32::trunc),
            length: keyword(rest, ":length").and_then(Sexp::number).map(f32::trunc),
            offset: keyword(rest, ":offset").and_then(Sexp::number).unwrap_or(0.0).trunc(),
            binding,
            bound: flags.iter().any(|f| f == "start-dead" || f == "sp2"),
            asap: flags.iter().any(|f| f == "launch-asap" || f == "sp3"),
            is_3d: flags.iter().any(|f| f == "is-3d"),
        });
    }
    Some(Group {
        name,
        id,
        duration,
        linger,
        items: entries,
    })
}

/// The group Jak 3 starts where each gun event happens (`gun-*-shot.gc`'s
/// `part-tracker-spawn` calls), with the frame it launches in.
fn group_for(event: &crate::Event, kind: crate::GunKind) -> Vec<(&'static str, Vec3, Mat3)> {
    use crate::{Event, GunKind as K};
    match event {
        Event::GunFire { kind, from } => {
            let group = match kind {
                K::Red1 => "group-gun-red-shot-fired",
                K::Red3 => "group-gun-red3-shot-fired",
                K::Yellow3 => "group-gun-yellow3-muzzle-smoke",
                K::Dark1 => "group-gun-dark-shot-fired",
                K::Dark2 => "group-gravity-gun-muzzle",
                _ => return Vec::new(),
            };
            vec![(group, *from, Mat3::IDENTITY)]
        }
        Event::Impact(hit) => {
            let group = match (kind, hit.actor.is_some()) {
                (K::Yellow1 | K::Yellow2, false) => "group-gun-yellow-shot-hit",
                (K::Yellow1 | K::Yellow2, true) => "group-gun-yellow-shot-hit-object",
                (K::Yellow3, false) => "group-gun-yellow-shot-hit-3",
                (K::Yellow3, true) => "group-gun-yellow-shot-hit-object-3",
                // (The Arc Wielder's shot draws nothing where it strikes:
                // `gun-blue-shot-2`'s `projectile-method-26` is empty.)
                (K::Blue1, _) => "group-gun-blue-shot-hit",
                (K::Blue3, _) => "group-gun-blue3-shot-impact",
                (K::Dark1, true) => "group-lightning-dark-shot-tip-hit",
                _ => return Vec::new(),
            };
            vec![(group, hit.pos, surface_frame(hit.normal))]
        }
        Event::Explosion { kind, center, .. } => match kind {
            K::Red3 => vec![("group-gun-red3-shot-explode", *center, Mat3::IDENTITY)],
            K::Dark1 => vec![("group-jak-peacemaker-shot-hit", *center, Mat3::IDENTITY)],
            K::Dark3 => vec![
                ("group-gun-dark3-stalk", *center, Mat3::IDENTITY),
                ("group-gun-dark3-ring", *center, Mat3::IDENTITY),
            ],
            K::Yellow3 => vec![("group-gun-yellow-saucer-fizz", *center, Mat3::IDENTITY)],
            _ => Vec::new(),
        },
        // Into the water and out of it (`water-part.gc`), `spawn-ripples` setting it 40.96 over the surface.
        Event::Splash { pos, speed } => vec![(
            if *speed >= 0.0 { "group-part-water-splash" } else { "group-part-water-splash-small" },
            *pos + Vec3::Y * crate::swim::SURFACE_LIFT,
            Mat3::IDENTITY,
        )],
        _ => Vec::new(),
    }
}

/// GOAL normalizes the three basis columns before applying relative motion,
/// so moving/scaling an owner never applies its model scale a second time.
fn rigid_basis(basis:Mat3)->Mat3 {
    Mat3::from_cols(basis.x_axis.normalize_or(Vec3::X),basis.y_axis.normalize_or(Vec3::Y),basis.z_axis.normalize_or(Vec3::Z))
}

/// Sprite UV flips set by `birth-func-flip-based-on-scale`. The low nibble
/// still selects the geometric corner table; these two flags only mirror UVs.
pub fn sprite_uvs(flags:u8)->[[f32;2];4] {
    [[0.,1.],[1.,1.],[1.,0.],[0.,0.]].map(|[u,v]|[
        if flags&16!=0 {1.-u}else{u},if flags&32!=0 {1.-v}else{v}])
}

/// A frame with `normal` as its up.
fn surface_frame(normal: Vec3) -> Mat3 {
    let up = normal.try_normalize().unwrap_or(Vec3::Y);
    let side = if up.y.abs() < 0.99 { Vec3::Y } else { Vec3::X };
    let x = side.cross(up).normalize();
    let z = x.cross(up);
    Mat3::from_cols(x, up, z)
}

impl crate::Jak {
    /// Gives Jak the game's particle definitions; from then on his guns'
    /// fire, hits and blasts launch their original groups.
    pub fn set_particles(&mut self, library: Arc<Library>) {
        self.particles = Some(Particles::new(library));
    }

    /// This frame's distorters (the board's heat haze; Jak units).
    pub fn particle_distorters(&self) -> Vec<Distorter> {
        self.particles.as_ref().map_or_else(Vec::new, Particles::distorters)
    }

    /// Presentation-only joint refresh: the authored callbacks stay on the
    /// board drawn this frame even when rendering faster than the 60 Hz clock.
    pub fn sync_board_particle_joint(&mut self,joint:Vec3) {
        if let Some(particles)=self.particles.as_mut() {
            particles.follow_board_joint(joint);
        }
    }

    /// This frame's gun particles as sprites (Jak units).
    pub fn particle_sprites(&self) -> Vec<Sprite> {
        let mut out = self.particles.as_ref().map_or_else(Vec::new, Particles::sprites);
        out.extend(self.gravity_ring_sprites());
        out
    }

    pub(crate) fn gun_particles(&mut self, first_event: usize) {
        self.ambient_particles();
        let kind = self.gun.kind;
        // His splashes (into the water and out of it): the splash group and the ripple mesh (`spawn-ripples`).
        let splashes: Vec<(Vec3, f32)> = self.events[first_event.min(self.events.len())..]
            .iter()
            .filter_map(|event| if let crate::Event::Splash { pos, speed } = event { Some((*pos, *speed)) } else { None })
            .collect();
        for (pos, speed) in splashes {
            self.splash_event(pos, speed);
        }
        let Some(particles) = self.particles.as_mut() else {
            return;
        };
        let facing = self.control.quat;
        particles.set_target(self.control.trans, self.gun.fire_point);
        particles.set_target_ground(self.control.ground_touch_point.y);
        for event in &self.events[first_event.min(self.events.len())..] {
            if matches!(event, crate::Event::Splash { .. }) {
                continue;
            }
            for (group, pos, basis) in group_for(event, kind) {
                particles.spawn(
                    group,
                    pos,
                    if matches!(event, crate::Event::GunFire { .. }) {
                        forward_frame(self.gun.fire_dir)
                    } else {
                        basis
                    },
                );
            }
            // Single parts the shots launch themselves: the Blaster
            // family's flashes (`muzzle-flash-part` through `draw-beam`)
            // and the Vulcan Fury's smoke and shell casings, turned to
            // Jak's heading (`rotate-y` set from `y-angle`).
            match *event {
                crate::Event::Fire { from, dir } => particles.launch_beam(268, from, dir),
                // The Vulcan Fury's beam to where it landed (231) and its
                // flash at the muzzle (234).
                crate::Event::Trace {
                    kind: crate::GunKind::Blue1,
                    from,
                    to,
                } => {
                    let path = to - from;
                    particles.launch_beam_len(231, from, path, path.length());
                    particles.launch_beam(234, from, path);
                }
                crate::Event::GunFire { kind, from } => {
                    let ahead = facing * Vec3::Z;
                    match kind {
                        // The Scatter Gun's flash along the barrel (213) and
                        // its two spent shells (216, `rotate-y` his heading).
                        crate::GunKind::Red1 => {
                            particles.launch_beam(213, from, self.gun.fire_dir);
                            particles.launch(216, from, Mat3::from_rotation_y(ahead.x.atan2(ahead.z)));
                        }
                        crate::GunKind::Red3 => particles.launch_beam(217, from, self.gun.fire_dir),
                        crate::GunKind::Dark1 => particles.launch_beam(250, from, self.gun.fire_dir),
                        // Their muzzle flash along the shot (`starting-dir`).
                        crate::GunKind::Yellow2 => particles.launch_beam(274, from, self.gun.fire_dir),
                        crate::GunKind::Yellow3 => particles.launch_beam(275, from, self.gun.fire_dir),
                        crate::GunKind::Blue1 => {
                            let turn = Mat3::from_rotation_y(ahead.x.atan2(ahead.z));
                            particles.launch(236, from, turn);
                            particles.launch(235, from, turn);
                        }
                        _ => {}
                    }
                }
                _ => {}
            }
        }
        // The board's own, where Jak 3 spawns them (`board-states.gc`,
        // `target-board.gc`): a charged jump's launch at his root (`group-
        // board-launch`: the blue clouds, sparks and star flash), a hard
        // landing's puff with its bounce (`group-land-poof-stn`), and the
        // zap's ring (`group-board-zap-attack`, kept on the board's joint
        // while it lasts, `board-zap-track`). (Its spin's shows only on a
        // hit, and the spin strikes nothing here; the grind's sparks have no
        // rails to grind; `group-board-land-straight`'s part 249 is in no
        // file of the game.)
        const BOARD_ZAP: u64 = 0xb0a2_d000;
        let board_at = self
            .joints
            .get(BOARD_JOINT)
            .map_or(self.control.trans, |&local| self.control.trans + facing * local);
        for event in &self.events[first_event.min(self.events.len())..] {
            match event {
                crate::Event::BoardLaunch => {
                    particles.spawn("group-board-launch", self.control.trans, Mat3::from_quat(facing));
                }
                crate::Event::BoardBounce { .. } => {
                    particles.spawn("group-land-poof-stn", self.control.trans, Mat3::IDENTITY);
                }
                crate::Event::BoardZap { .. } => {
                    particles.stop_tagged(BOARD_ZAP);
                    particles.spawn_tagged("group-board-zap-attack", board_at, Mat3::IDENTITY, BOARD_ZAP);
                }
                _ => {}
            }
        }
        if particles.tagged(BOARD_ZAP, "group-board-zap-attack") {
            particles.move_tagged(BOARD_ZAP, |_| board_at);
        }
        // The shots in flight, every frame as theirs do
        // (`projectile-method-25`): the Blaster's beam as long as it has
        // flown, up to 16 m (264), its glow four fifths of the way to its
        // head (265) and sparks falling off the head (267, smaller while the
        // beam is short); the Gyro Burster's bullets their thinner beam (276).
        for p in &self.projectiles {
            let flown = p.trans - p.tail;
            let length = flown.length();
            match p.kind {
                crate::GunKind::Yellow1 => {
                    particles.launch_beam_len(264, p.tail, flown, length);
                    particles.launch(265, p.tail + flown * 0.8, Mat3::IDENTITY);
                    if let Some(ahead) = flown.try_normalize() {
                        let scale = (length / (16.0 * METER)).min(1.0);
                        particles.launch_scaled(267, p.trans, forward_frame(ahead), scale);
                    }
                }
                crate::GunKind::Yellow3 => particles.launch_beam_len(276, p.tail, flown, length),
                _ => {}
            }
        }
        let sources = self.weapons.particle_sources(self.time);
        particles.set_sources(sources.iter().map(|s| (s.id, s.at)));
        for source in sources {
            if let Some(charge) = source.charge {
                particles.launch_peacemaker(source.id, source.at, charge);
            } else {
                for part in source.parts {
                    particles.launch_from(*part, source.id, source.at, Mat3::IDENTITY);
                }
            }
        }
        // The Arc Wielder's own: its muzzle flare spawned every frame it is
        // out (`(spawn (-> self part) fire-point)`), the sparks' tips.
        const ARC_FLARE: u64 = 0xa7c1;
        if self.weapons.arc_active() {
            particles.keep(ARC_FLARE, "group-gun-blue-2-muzzle-flare", self.gun.fire_point, Mat3::IDENTITY);
        } else {
            particles.stop_tagged(ARC_FLARE);
        }
        for (group, at, basis) in self.weapons.take_pending_groups() {
            particles.spawn(group, at, basis);
        }
        let mut bolts = self.weapons.lightning_requests(self.time);
        bolts.extend(self.dark_shock.lightning_requests(self.time));
        particles.sync_lightning(bolts);
        particles.step(1.0 / 60.0);
    }

    /// What goes with him every frame, as the game launches it: the board's
    /// blue swirl under it while he crouches to charge a jump
    /// (`group-target-board-duck-charge`: the flat `laser-hit2` 669, turned
    /// at random and shrinking), riding on the ground its blue glow and
    /// streaks under it (672, 673), and in the water its wake, ripples and
    /// drops ([`Jak::water_particles`]).
    fn ambient_particles(&mut self) {
        /// Light Jak's followed effects' tags, clear of the others'.
        const LIGHT_FX_TAGS: u64 = 0x4c4a_0000_0000;
        // The water's own: its splash, his wake and ripples, the drops (`water-control`).
        self.water_particles();
        if let Some(particles)=self.particles.as_mut() {
            particles.set_target_orientation(self.control.local_normal,self.control.y_angle());
        }
        let pos = self.control.trans;
        let heading = crate::math::z_axis(self.control.quat);
        let flat = Vec3::new(heading.x, 0.0, heading.z).try_normalize().unwrap_or(Vec3::Z);
        let level = Mat3::from_cols(Vec3::Y.cross(flat), Vec3::Y, flat);
        let charging = self.state.is_board() && self.board.charge_progress != 0.0;
        // Grinding, the sparks off the rail (`target-board-ride-edge`: part
        // 670 a metre back along the way he goes, its `num` from 0.1 to 2 as
        // his speed comes to 0.3 of the ride's top).
        let sparks = (self.state == crate::State::BoardGrind)
            .then(|| self.velocity())
            .filter(|v| v.length() > METER)
            .map(|v| {
                let speed = v.length();
                let top = 0.3 * self.control.mod_surface.transv_max.max(METER);
                (pos - v / speed * METER, 0.1 + 1.9 * (speed / top).min(1.0))
            });
        // The shield's opening: Light Jak's sparkles rising round him
        // (`group-lightjak-shield-start`, on his joint 3).
        let shield_opened = self.lightjak.shield && !self.lightjak.shield_was;
        self.lightjak.shield_was = self.lightjak.shield;
        // His joints in the world (his blows' `trans + quat * joint`).
        let world: Vec<Vec3> = self.joints.iter().map(|&local| pos + self.control.quat * local).collect();
        let joint_at = |j: usize| world.get(j).copied().unwrap_or(pos);
        let fx = std::mem::take(&mut self.lightjak.fx);
        // His eyes' glow as Light Jak (part 650 on `neckB`), from the change
        // on until the change back, turned with his body.
        let eyes: Vec<Vec3> = if self.lightjak.on && self.lightjak.interp >= 1.0 && self.state != crate::State::LightJakGetOff && world.len() > crate::lightjak::JOINT_NECK {
            let neck = joint_at(crate::lightjak::JOINT_NECK);
            crate::lightjak::EYES.iter().map(|e| neck + self.control.quat * *e).collect()
        } else {
            Vec::new()
        };
        let mut tracked = std::mem::take(&mut self.lightjak.fx_tracked);
        let mut tag = self.lightjak.fx_tag;
        let healing = self.lightjak.healing;
        let Some(particles) = self.particles.as_mut() else {
            self.lightjak.fx_tracked = tracked;
            return;
        };
        // The heal's light shafts reach from him up past the top of the
        // screen: their `:scale-y` set each frame of the heal from
        // `point-off-screen` (`target-lightjak-regen`).
        let shaft = healing.then(|| particles.point_off_screen(pos)).flatten().map(|top| top.y - pos.y);
        for part in 642..=645 {
            particles.set_part_value(part, "scale-y", shaft);
        }
        for (group, at) in fx {
            match at {
                crate::lightjak::LightFxAt::Follow(j) => {
                    tag += 1;
                    particles.spawn_tagged(group, joint_at(j), level, LIGHT_FX_TAGS + tag);
                    tracked.push((LIGHT_FX_TAGS + tag, group, j));
                }
                crate::lightjak::LightFxAt::Joint(j) => particles.spawn(group, joint_at(j), level),
                // `*launch-matrix*` with only its translation set.
                crate::lightjak::LightFxAt::Feet => particles.spawn(group, pos, Mat3::IDENTITY),
            }
        }
        // `part-tracker-track-target`: each on its joint while it lasts.
        tracked.retain(|&(t, group, _)| particles.tagged(t, group));
        for &(t, _, j) in &tracked {
            particles.move_tagged(t, |_| joint_at(j));
        }
        for eye in &eyes {
            particles.launch(650, *eye, Mat3::IDENTITY);
        }
        self.lightjak.fx_tracked = tracked;
        self.lightjak.fx_tag = tag;
        let Some(particles) = self.particles.as_mut() else {
            return;
        };
        if shield_opened {
            particles.spawn("group-lightjak-shield-start", pos + Vec3::Y * METER, level);
        }
        if let Some((at, num)) = sparks {
            let overrides = Overrides { num: Some(num), ..Default::default() };
            particles.launch_var(670, at, Mat3::IDENTITY, overrides, &mut self.grind.spark_carry);
        }
        const BOARD_CHARGE:u64=0xb0a2_c000;
        let board_joint=joint_at(BOARD_JOINT);
        if charging {
            particles.keep(BOARD_CHARGE,"group-target-board-duck-charge",board_joint,level);
        } else {particles.stop_tagged(BOARD_CHARGE);}
        // Native charge particles finish on joint 37 after release as well.
        particles.move_tagged(BOARD_CHARGE,|_|board_joint);
        // Riding, the board's blue glow under it (`group-target-board`'s
        // flat `hotdot` 672, 4 to 6 m across, a quarter metre up, turned
        // with the board by `birth-func-target-orient`, launched every
        // frame): seen from below as the game's (17/10).
        // `target-board-effect` spawns the whole group while the board is
        // on the ground (`board-on-ground?`) and drawn: its glow and its
        // blue streaks shot off under it (673, `sparticle-motion-blur`;
        // Adrii 23/10: the board lit wants its effects, the streaks back).
        // Its heat haze (671, `distort`) goes to the sprite distorter.
        if self.state.is_board() && self.control.on_surface() {
            let board = Mat3::from_quat(self.control.quat);
            // The authored Y offset is already 0.25 m; do not add it twice.
            particles.launch(672,pos,board);
            particles.launch(673, pos, Mat3::IDENTITY);
        }
    }

    /// Whether the game's particles draw the Blaster's and the Gyro
    /// Burster's shots in flight (else the host's plain beams do).
    pub(crate) fn particles_draw_shots(&self) -> bool {
        self.particles.is_some()
    }
}

/// The board's joint in Jak's skeleton (`jakb-lod0-jg` `board`, the
/// `:mat-joint 37` its effects are spawned on).
const BOARD_JOINT: usize = 37;

/// `forward-up->inv-matrix`: a frame whose z is `ahead`, y as near up as it
/// can be.
fn forward_frame(ahead: Vec3) -> Mat3 {
    let x = Vec3::Y.cross(ahead).try_normalize().unwrap_or(Vec3::X);
    Mat3::from_cols(x, ahead.cross(x), ahead)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Jak 3's vehicle parts (`vehicle-part.gc`), when the install is there.
    fn vehicle_parts() -> Option<Particles> {
        let home = std::env::var_os("USERPROFILE")?;
        let path = std::path::PathBuf::from(home)
            .join(r"Documents\Jak OpenGOAL\active\jak3\data\goal_src\jak3\levels\city\traffic\vehicle\vehicle-part.gc");
        let text = std::fs::read_to_string(path).ok()?;
        let mut library = Library::default();
        library.read(&text);
        Some(Particles::new(Arc::new(library)))
    }

    /// `vehicle-method-78`'s exhaust: its alpha and velocity as the caller
    /// set them, its number's fractions carried in the `birthaccum`.
    #[test]
    fn a_launch_takes_what_its_caller_set() {
        let Some(mut particles) = vehicle_parts() else { return };
        let vel = Vec3::new(10.0, 20.0, -30.0);
        let overrides = Overrides { num: Some(0.25), alpha: Some((16.0, 0.0)), vel: Some(vel), scale: None, timer: None };
        let mut accum = 0.0;
        // 923's number: 0.25 plus up to 0.2 a launch; 20 launches, 5 to 9.
        for _ in 0..20 {
            particles.launch_var(923, Vec3::ZERO, Mat3::IDENTITY, overrides, &mut accum);
        }
        assert!((5..=9).contains(&particles.len()), "{} puffs", particles.len());
        assert!((0.0..1.0).contains(&accum));
        for p in &particles.live {
            assert_eq!(p.rgba[3], 16.0, "its alpha the caller's");
            assert_eq!(p.vel, vel, "`birth-func-set-vel`");
        }
    }

    /// The thruster's flame is a 3D blade: in its plane, along its beam.
    #[test]
    fn a_blade_lies_in_its_plane_along_its_beam() {
        let Some(mut particles) = vehicle_parts() else { return };
        particles.launch_blade(922, Vec3::ZERO, Vec3::NEG_Z, Vec3::Y, 8192.0, 2457.6);
        let sprites = particles.sprites();
        assert!(!sprites.is_empty());
        for s in sprites {
            assert_eq!(s.stretch, Some(Vec3::NEG_Z));
            assert_eq!(s.flat, Some(Vec3::Y));
            assert_eq!(s.size, [8192.0, 2457.6], "its length along, its width across");
            assert_eq!(s.pos, Vec3::new(0.0, 0.0, -4096.0), "its middle half its length out");
        }
    }
}

#[cfg(test)]
mod faithful_tests {
    use super::*;

    #[test]
    fn original_board_zap_follows_the_board_joint_while_its_ring_expands() {
        let home = std::path::PathBuf::from(std::env::var_os("USERPROFILE").unwrap());
        let path = home.join(r"Documents\Jak OpenGOAL\active\jak3\data\goal_src\jak3\engine\target\board\board-part.gc");
        let mut library = Library::default();
        library.read(&std::fs::read_to_string(path).expect("original board particles"));
        let mut particles = Particles::new(Arc::new(library));
        let tag = 37;
        let initial = Vec3::new(0.0, 20.0 * METER, 0.0);
        particles.spawn_tagged("group-board-zap-attack", initial, Mat3::IDENTITY, tag);
        for _ in 0..8 {
            particles.step(1.0 / 60.0);
            if particles.live.iter().any(|p| p.part == 678) { break; }
        }
        assert!(particles.live.iter().any(|p| p.part == 678));
        let mut observed = 0;
        for i in 1..8 {
            let joint = initial + Vec3::new(i as f32 * 0.3 * METER, 0.0, 0.0);
            particles.move_tagged(tag, |_| joint);
            for p in particles.live.iter().filter(|p| p.part == 678) {
                observed += 1;
                assert!(p.pos.distance(joint) < 0.001, "ring left behind the moving board");
            }
            particles.step(1.0 / 60.0);
            for p in particles.live.iter().filter(|p| p.part == 678) {
                assert!(p.pos.distance(joint) < 0.001, "native callback lost the board joint");
            }
        }
        assert!(observed >= 7, "native ring must remain visible while moving");
    }

    #[test]
    fn original_water_and_board_attack_rings_expand_in_xz_without_a_vertical_wall() {
        let home=std::path::PathBuf::from(std::env::var_os("USERPROFILE").unwrap());
        let root=home.join(r"Documents\Jak OpenGOAL\active\jak3\data\goal_src\jak3");
        let mut library=Library::default();
        for file in ["engine/common-obs/water-part.gc","engine/target/board/board-part.gc"]{
            library.read(&std::fs::read_to_string(root.join(file)).expect("original particle source"));
        }
        let library=Arc::new(library);
        for (group,ids) in [("group-part-water-splash",&[767u32,768][..]),("group-board-zap-attack",&[678][..])]{
            for yaw in [0.0,0.7,2.4]{
                let mut p=Particles::new(library.clone());
                p.spawn(group,Vec3::new(20.,30.,40.),Mat3::from_rotation_y(yaw));
                let mut drawn=0;let mut smallest=f32::MAX;let mut largest=0.0f32;
                for _ in 0..35{
                    p.step(1.0/60.0);
                    p.live.retain(|a|ids.contains(&a.part));
                    for s in p.sprites(){
                        drawn+=1;smallest=smallest.min(s.size[0]);largest=largest.max(s.size[0]);
                        let n=s.flat.expect("native 3D ring").normalize();
                        let x=s.stretch.expect("authored quaternion X").normalize();
                        assert!(n.y.abs()>0.9999,"{group} has vertical normal {n:?}");
                        assert!(x.y.abs()<0.0001,"{group} grows vertically: {x:?}");
                        assert!(n.cross(x).y.abs()<0.0001,"second scale must stay on the water/board plane");
                    }
                }
                assert!(drawn>0,"native {group} emitted no observed ring");
                assert!(largest-smallest>0.05*METER,"authored scale animation did not run");
            }
        }
    }

    #[test]
    fn three_dimensional_rotations_are_applied_once_and_include_all_velocity_axes(){
        let mut library=Library::default();
        library.read("(defpart 1 :init-specs ((:texture (hotdot effects)) (:num 1) (:timer 300) (:rot-z (degrees 90)) (:rotvel-y (degrees 1)) (:scale-x 2) (:scale-y 3))) (defpartgroup turn :parts ((sp-item 1 :flags (is-3d))))");
        let mut p=Particles::new(Arc::new(library));p.spawn("turn",Vec3::ZERO,Mat3::IDENTITY);p.step(1.0/60.0);
        let a=p.live.first().unwrap();let q=a.quad.unwrap();let s=p.sprites().remove(0);
        assert!((s.stretch.unwrap()-q.x_axis).length()<1e-5,"RotZ must not turn the quaternion's plane twice");
        let before=q;p.step(1.0/60.0);let after=p.live[0].quad.unwrap();
        assert!(before.x_axis.distance(after.x_axis)>0.001,"3D Y rotation velocity must animate its quaternion");
    }

    #[test]
    fn drops_use_their_own_games_callback_ids_and_actual_water_level(){
        let home=std::path::PathBuf::from(std::env::var_os("USERPROFILE").unwrap()).join(r"Documents\Jak OpenGOAL\active");
        for(game,drop,ring,secondary)in [("jak2",517,501,536),("jak3",777,780,781)]{
            let root=home.join(game).join("data/goal_src").join(game);
            let mut library=Library::default();
            if game=="jak2"{library.read_jak2(&std::fs::read_to_string(root.join("engine/common_objs/water.gc")).unwrap());}
            else{for file in ["engine/common-obs/water-part.gc","engine/common-obs/water.gc"]{library.read(&std::fs::read_to_string(root.join(file)).unwrap());}}
            assert_eq!(library.water_drop_parts["check-water-level-drop"],vec![(ring,true)]);
            assert_eq!(library.water_drop_parts["check-water-level-drop-motion"],vec![(ring,true),(secondary,false)]);
            let mut p=Particles::new(Arc::new(library));let root=Vec3::new(0.,80.*METER,0.);
            p.configure_tracker("part-water-splash-callback",0.7,root).unwrap();
            for _ in 0..20{p.launch(drop,root,Mat3::IDENTITY);}
            assert!(!p.live.is_empty(),"{game} droplet emission");
            assert!(p.live.iter().all(|p|p.floor==Some(root.y)),"{game}: floor must not be world zero");
            let mut landed=false;
            for _ in 0..300{
                p.step(1.0/60.0);
                if p.live.iter().any(|p|p.part==ring){landed=true;break;}
            }
            assert!(landed,"{game}: water drops must emit their original impact ring");
            for ring in p.sprites().into_iter().filter(|s|(s.pos.y-root.y).abs()<0.001){
                assert!(ring.flat.is_some_and(|n|n.normalize().y.abs()>0.9999));
            }
        }
    }

    /// Jak 1's `collectables-part.gc`, read as Jak 1's, when the install is
    /// there.
    fn jak1_collectables() -> Option<Arc<Library>> {
        let home = std::env::var_os("USERPROFILE")?;
        let path = std::path::PathBuf::from(home)
            .join(r"Documents\Jak OpenGOAL\active\jak1\data\goal_src\jak1\engine\common-obs\collectables-part.gc");
        let mut library = Library::default();
        library.read_jak1(&std::fs::read_to_string(path).ok()?);
        Some(Arc::new(library))
    }

    #[test]
    fn ordinary_groups_use_source_defaults_while_kept_controls_continue() {
        let mut library = Library::default();
        library.read("(defpart 1 :init-specs ((:num 1.0) (:timer 5) (:texture (glow common)))) (defpartgroup finite :parts ((sp-item 1)))");
        let library = Arc::new(library);
        let mut finite = Particles::new(library.clone());
        finite.spawn("finite", Vec3::ZERO, Mat3::IDENTITY);
        finite.step(16.0);
        assert!(finite.launches.is_empty());
        assert!(finite.is_empty());
        let mut kept = Particles::new(library);
        for _ in 0..1920 { kept.keep(1, "finite", Vec3::ZERO, Mat3::IDENTITY); kept.step(1.0/60.0); }
        assert_eq!(kept.launches.len(), 1);
        assert!(!kept.is_empty());
    }

    #[test]
    fn original_green_puff_rises_and_fades_on_the_300_hz_clock() {
        let Some(library) = jak1_collectables() else { return };
        let mut p = Particles::new(library);
        for _ in 0..20 { p.launch(226, Vec3::ZERO, Mat3::IDENTITY); }
        assert!(!p.live.is_empty());
        let before = p.live.clone();
        p.step(0.5);
        assert_eq!(p.len(), before.len());
        for (after, before) in p.live.iter().zip(before) {
            assert!((after.pos.y - before.pos.y - before.vel.y * 150.0).abs() < 0.1);
            assert!((after.rgba[3] - 16.0).abs() < 0.01);
            assert_eq!(after.rgba[2], 0.0);
        }
        p.step(0.5);
        assert!(p.live.iter().all(|p| p.rgba[3] < 0.01), "the one-second puffs fade away");
        p.step(1.0 / 60.0);
        assert!(p.is_empty(), "the one-second puffs retire on the following frame");
    }

    #[test]
    fn original_green_heads_orbit_their_rising_parent_instead_of_crossing_the_sky() {
        let Some(library) = jak1_collectables() else { return };
        let mut p = Particles::new(library);
        let group = "group-part-vent-green-active";
        let mut seen = 0;
        for _ in 0..600 {
            p.keep(0, group, Vec3::ZERO, Mat3::IDENTITY);
            p.step(1.0 / 60.0);
            for head in p.live.iter().filter(|p| p.part == 223) {
                let orbit = head.orbit.as_ref().expect("Jak 1 bit7 invokes sp-orbiter");
                assert!(head.parent.is_some(), "its bound origin is the rising part 222");
                assert!((head.pos.distance(orbit.centre) - orbit.radius.abs()).abs() < 0.1);
                assert!(head.pos.x.abs() < 2.0 * METER && head.pos.z.abs() < 2.0 * METER);
                assert!(head.pos.y < 9.0 * METER);
                seen += 1;
            }
        }
        assert!(seen > 100);
        p.stop_tagged(0);
        p.step(5.0);
        assert!(p.is_empty(), "stopping the vent leaves no immortal heads or trails");
    }

    #[test]
    fn vent_emission_is_the_same_at_30_60_and_144_fps_and_survives_thirty_seconds() {
        let Some(library) = jak1_collectables() else { return };
        for group in ["group-part-vent-red-active", "group-part-vent-yellow-active", "group-part-vent-blue-active", "group-part-vent-green-active"] {
            let run = |fps| {
                let mut p = Particles::new(library.clone());
                for _ in 0..fps * 32 {
                    p.keep(0, group, Vec3::ZERO, Mat3::IDENTITY);
                    p.step(1.0 / fps as f32);
                }
                assert_eq!(p.launches.len(), 1, "{group}: one continuous control after 32 seconds");
                p
            };
            let expected = run(60);
            for fps in [30, 144] {
                let actual = run(fps);
                assert_eq!(actual.len(), expected.len(), "{group}, {fps} fps");
                for (a,b) in actual.live.iter().zip(&expected.live) {
                    assert_eq!(a.part, b.part);
                    assert!((a.pos-b.pos).length() < 0.01, "{group}, {fps} fps");
                    assert_eq!(a.rgba, b.rgba);
                }
            }
        }
    }

    #[test]
    fn indefinite_eco_holds_until_its_owner_stops_then_uses_the_authored_fade() {
        let Some(library) = jak1_collectables() else { return };
        let mut p = Particles::new(library);
        p.spawn_tagged("group-eco-green-pill", Vec3::ZERO, Mat3::IDENTITY, 7);
        p.step(2.0);
        assert!(p.live.iter().any(|p| p.part == 209 && p.timer == -1.0));
        assert!(p.live.iter().any(|p| p.part == 212 && p.timer == -1.0));
        p.stop_tagged(7);
        p.step(2.0);
        assert!(p.is_empty(), "eco-fadeout must release next-launcher after stop");
    }

    #[test]
    fn integer_random_choices_include_both_ends_and_float_products_stay_continuous() {
        let mut lib = Library::default();
        lib.read("(defpart 1 :init-specs ((:num 64.0) (:rotvel-z (degrees -0.1) 1 (degrees 0.2)) (:vel-x 1.0 2.0 3.0) (:g 4.0 1.0 :store) (:b '*sp-temp*) (:timer 100))))");
        let mut p = Particles::new(Arc::new(lib));
        p.launch(1, Vec3::ZERO, Mat3::IDENTITY);
        assert!(p.live.iter().any(|p| p.rotvel < 0.0));
        assert!(p.live.iter().any(|p| p.rotvel > 0.0));
        assert!(p.live.iter().all(|p| (1.0..7.0).contains(&p.vel.x) && p.rgba[1] == p.rgba[2]));
        assert!(p.live.iter().any(|p| (p.vel.x - 1.0).abs() > 0.01 && (p.vel.x - 4.0).abs() > 0.01 && (p.vel.x - 7.0).abs() > 0.01));
    }

    #[test]
    fn dark_vent_sparks_accelerate_toward_their_centre_in_every_launch_direction() {
        let Some(home) = std::env::var_os("USERPROFILE") else { return };
        let path = std::path::PathBuf::from(home).join(r"Documents\Jak OpenGOAL\active\jak3\data\goal_src\jak3\engine\common-obs\collectables-part.gc");
        let Ok(source) = std::fs::read_to_string(path) else { return };
        let mut library = Library::default();
        library.read(&source);
        let mut p = Particles::new(Arc::new(library));
        for _ in 0..20 { p.launch(518, Vec3::ZERO, Mat3::IDENTITY); }
        assert!(p.len() >= 40);
        for spark in &p.live {
            let inward = -spark.pos.normalize();
            assert!(spark.accel.normalize().dot(inward) > 0.9999, "radial acceleration became global X");
        }
    }

    #[test]
    fn a_five_tick_flash_is_drawn_once_and_then_released() {
        let mut library = Library::default();
        library.read("(defpart 1 :init-specs ((:texture (hotdot effects)) (:num 1.0) (:a 128.0) (:scale-x 100.0) (:scale-y 100.0) (:timer (seconds 0.017)))))");
        let mut p = Particles::new(Arc::new(library));
        p.launch(1, Vec3::ZERO, Mat3::IDENTITY);
        p.step(1.0 / 60.0);
        assert_eq!(p.sprites().len(), 1, "one-frame effects must not vanish before being drawn");
        p.step(1.0 / 60.0);
        assert!(p.is_empty());
    }

    #[test]
    fn integer_tick_windows_emit_once_per_pulse_and_relaunch_on_time() {
        let mut library = Library::default();
        library.read("(defpart 1 :init-specs ((:num 1.0) (:timer (seconds 10)) (:scale-x 100.0) (:scale-y 100.0))))
            (defpartgroup pulse :parts ((sp-item 1 :period (seconds 0.16) :length (seconds 0.017))))
            (defpart 2 :init-specs ((:num 1.0) (:a 128.0) (:timer (seconds 1)) (:next-time (seconds 0.035)) (:next-launcher 3))))
            (defpart 3 :init-specs ((:a 0.0))))");
        let library = Arc::new(library);
        let mut p = Particles::new(library.clone());
        for _ in 0..96 {
            p.keep(0, "pulse", Vec3::ZERO, Mat3::IDENTITY);
            p.step(1.0 / 60.0);
        }
        assert_eq!(p.len(), 10, "1.6 seconds has ten 0.16-second pulses, not twelve");
        let mut p = Particles::new(library);
        p.launch(2, Vec3::ZERO, Mat3::IDENTITY);
        p.step(2.0 / 60.0);
        assert_eq!(p.live[0].rgba[3], 0.0, "0.035 seconds truncates to ten ticks");
    }

    /// The blend `sp-launch-particles-var` gives a particle by its flags:
    /// `bit4` / `sp-cpuinfo-flag-4` subtracts, `bit3` / `sp-cpuinfo-flag-3`
    /// adds over it, none blends; `glow` is the glow renderer's. Jak 1's
    /// textures are named for its own export, by page.
    #[test]
    fn blends_and_textures_follow_their_sources() {
        let mut library = Library::default();
        library.read_jak1(
            "(defpart 1 :init-specs ((:texture (hotdot effects)) (:flags (bit2 bit3))))
             (defpart 2 :init-specs ((:texture (hotdot effects)) (:flags (bit4))))
             (defpart 3 :init-specs ((:texture (hotdot effects)) (:flags (bit4 bit3))))
             (defpart 4 :init-specs ((:texture (left Hud)) (:flags (bit2))))",
        );
        library.read(
            "(defpart 5 :init-specs ((:texture (glow level-default-sprite)) (:flags (sp-cpuinfo-flag-3 glow))))
             (defpart 6 :init-specs ((:texture (middot level-default-sprite)) (:flags (sp-cpuinfo-flag-2 sp-cpuinfo-flag-4))))",
        );
        let of = |id: u32| {
            let part = &library.parts[&id];
            (part.blend, part.texture.clone().unwrap_or_default())
        };
        assert_eq!(of(1), (Blend::Add, "jak1:hotdot".into()));
        assert_eq!(of(2), (Blend::Subtract, "jak1:hotdot".into()));
        assert_eq!(of(3), (Blend::Add, "jak1:hotdot".into()), "flag 3 over flag 4");
        assert_eq!(of(4), (Blend::Normal, "jak1:Hud/left".into()));
        assert_eq!(of(5), (Blend::Glow, "glow".into()));
        assert_eq!(of(6), (Blend::Subtract, "middot".into()));
    }

    /// Jak 1's vents well up as many particles as their groups launch: each
    /// item's `num` a frame (60 a second) living its `timer`, give or take
    /// their randomness; held, not piling up, and under the 2D system's
    /// 1920. Their colours and alphas stay those of their parts (PS2 scale:
    /// 0 to 255), and every sprite adds on as `bit3` asks.
    #[test]
    fn jak_1_vents_launch_their_groups_counts() {
        let Some(library) = jak1_collectables() else { return };
        for name in [
            "group-part-vent-red-active",
            "group-part-vent-yellow-active",
            "group-part-vent-blue-active",
            "group-part-vent-green-active",
        ] {
            let group = library.group(name).unwrap_or_else(|| panic!("{name}"));
            let mean = |part: &Part, field: Field, default: f32| match part.fields.iter().rev().find(|(f, _)| *f == field) {
                Some((_, Value::Range(base, range))) => base + range * 0.5,
                Some((_, Value::Steps(base, count, step))) => base + count.max(0.0) * 0.5 * step,
                _ => default,
            };
            let most = |part: &Part, field: Field, default: f32| match part.fields.iter().rev().find(|(f, _)| *f == field) {
                Some((_, Value::Range(base, range))) => base + range,
                Some((_, Value::Steps(base, count, step))) => base + count.max(0.0) * step,
                _ => default,
            };
            let mut particles = Particles::new(library.clone());
            // Live on average over seconds 4 to 6, and 8 to 10 (their
            // periodic bursts come and go), and at most.
            let (mut early, mut late, mut most_live) = (0.0, 0.0, 0usize);
            for frame in 0..600 {
                particles.keep(0, name, Vec3::ZERO, Mat3::IDENTITY);
                particles.step(1.0 / 60.0);
                most_live = most_live.max(particles.len());
                match frame {
                    240..360 => early += particles.len() as f32 / 120.0,
                    480..600 => late += particles.len() as f32 / 120.0,
                    _ => {}
                }
            }
            let at10 = particles.len();
            assert!(most_live < MAX_PARTICLES, "{name}: {most_live}");
            assert!((late - early).abs() <= 0.25 * early.max(10.0), "{name}: {early} then {late}, piling up");
            let mut ids: Vec<u32> = group.items.iter().map(|i| i.part).collect();
            ids.sort_unstable();
            ids.dedup();
            for id in ids {
                let part = &library.parts[&id];
                let life = |f: &dyn Fn(&Part, Field, f32) -> f32| f(part, Field::Timer, TICKS_PER_SECOND) / TICKS_PER_SECOND;
                // On its own: `num` a frame while its window is open,
                // living its timer.
                let own: f32 = group
                    .items
                    .iter()
                    .filter(|i| i.part == id && !i.bound && !i.asap)
                    .map(|i| {
                        let share = match (i.period, i.length) {
                            (Some(period), Some(length)) => (length.max(TICKS_PER_SECOND / 60.0) / period).min(1.0),
                            _ => 1.0,
                        };
                        mean(part, Field::Num, 1.0) * 60.0 * life(&mean) * share
                    })
                    .sum();
                // Bound: at most one launch a slot (`launch-asap`) or its
                // `num` a frame from each, while their particles live.
                let bound: f32 = group
                    .items
                    .iter()
                    .filter(|i| i.part == id && i.bound)
                    .map(|i| if i.asap { most(part, Field::Num, 1.0).ceil() } else { most(part, Field::Num, 1.0) * 60.0 * life(&most) })
                    .sum();
                let live = particles.live.iter().filter(|p| p.part == id).count() as f32;
                assert!(
                    live >= own * 0.6 - 6.0 && live <= own * 1.4 + bound + 6.0,
                    "{name}: part {id}: {live} live, its items launch about {own} (and up to {bound} bound)"
                );
                println!("{name}: part {id}: {live} live (about {own} on its own, up to {bound} bound)");
            }
            let sprites = particles.sprites();
            assert!(!sprites.is_empty(), "{name}");
            for s in &sprites {
                assert!(s.rgba.iter().all(|c| (0.0..=255.0).contains(c)), "{name}: {:?}", s.rgba);
                assert!(s.texture.starts_with("jak1:"), "{name}: {}", s.texture);
                let part_blends: Vec<_> = group.items.iter().map(|i| library.parts[&i.part].blend).collect();
                assert!(part_blends.contains(&s.blend), "{name}: {:?}", s.blend);
            }
            println!("{name}: {at10} live, {} sprites", sprites.len());
        }
    }
}

#[cfg(test)] mod remaining_native_fx_regressions {
    use super::*;
    fn original()->Arc<Library> {
        let root=std::path::PathBuf::from(std::env::var_os("USERPROFILE").unwrap()).join("Documents/Jak OpenGOAL/active/jak3/data/goal_src/jak3");
        let mut library=Library::default();
        for file in ["engine/target/board/board-part.gc","levels/desert/wvehicle/wvehicle-part.gc"] {
            library.read(&std::fs::read_to_string(root.join(file)).expect("original OpenGOAL particle source"));
        }
        Arc::new(library)
    }
    #[test] fn original_board_charge_remains_on_the_rendered_joint_after_release_at_144_hz() {
        let mut p=Particles::new(original());let tag=0xb0a2_c000;
        p.keep(tag,"group-target-board-duck-charge",Vec3::Y*METER,Mat3::IDENTITY);
        for _ in 0..10 {p.step(1./60.);}
        assert!(p.live.iter().any(|p|p.part==669));p.stop_tagged(tag);
        let mut jak=crate::Jak::new(Vec3::ZERO,0.);jak.particles=Some(p);
        for tick in 0..72 {
            let joint=Vec3::new(tick as f32*18.,METER+tick as f32*3.,-40.);
            let before=jak.particles.as_ref().unwrap().live.len();
            jak.sync_board_particle_joint(joint);
            let p=jak.particles.as_ref().unwrap();
            assert_eq!(p.live.len(),before,"render refresh cannot spawn effects");
            assert!(p.live.iter().filter(|p|p.part==669).all(|p|p.pos.distance(joint)<0.01));
            if tick%3==0 {jak.particles.as_mut().unwrap().step(1./48.);}
        }
        assert!(!jak.particles.as_ref().unwrap().live.iter().any(|p|p.part==669),"released charge expires on its native timer");
    }
    #[test] fn native_rocks_have_random_texture_frames_warm_brightness_and_world_velocity() {
        let mut p=Particles::new(original());let vel=Vec3::new(42.,78.,-21.);p.set_particle_velocity(vel);
        for _ in 0..100 {p.launch(954,Vec3::ZERO,Mat3::from_rotation_y(1.3));}
        assert!(p.live.len()>=100);
        assert!(p.live.iter().all(|p|p.vel==vel && (200. ..=250.).contains(&p.rgba[0])
            && (20. ..=30.).contains(&(p.rgba[0]-p.rgba[1])) && (60. ..=70.).contains(&(p.rgba[0]-p.rgba[2]))));
        assert!(p.live.iter().any(|p|p.texture_offset>0),"rock texture groups must not all start on frame zero");
    }
    #[test] fn negative_native_sprite_scales_mirror_the_texture_and_survive_the_particle_step() {
        let mut library=Library::default();library.read("(defpart 60000 :init-specs ((:texture (hotdot level-default-sprite)) (:num 1) (:birth-func 'birth-func-flip-based-on-scale) (:scale-x (meters -2)) (:scale-y (meters -3)) (:timer (seconds 1)) (:r 128) (:a 128)))");
        let mut p=Particles::new(Arc::new(library));p.launch(60000,Vec3::ZERO,Mat3::IDENTITY);p.step(1./60.);
        let s=p.sprites();assert_eq!(s.len(),1);assert_eq!(s[0].size,[2.*METER,3.*METER]);assert_eq!(s[0].corner&48,48);
        assert_eq!(sprite_uvs(s[0].corner),[[1.,0.],[0.,0.],[0.,1.],[1.,1.]]);
    }
}

#[cfg(test)] mod remaining_binding_regressions {
    use super::*;
    #[test] fn target_oriented_glow_uses_contact_plane_heading_and_one_authored_offset() {
        let mut l=Library::default();l.read("(defpart 60001 :init-specs ((:texture (hotdot effects)) (:birth-func 'birth-func-target-orient) (:num 1) (:y (meters 0.25)) (:scale-x 100) (:scale-y 100) (:timer 300)))");
        let mut p=Particles::new(Arc::new(l));let at=Vec3::new(30.,50.,80.);let normal=Vec3::new(0.2,1.,0.35).normalize();
        let frame=Mat3::from_quat(Quat::from_rotation_arc(Vec3::Y,normal));
        p.set_target_orientation(normal,0.7);p.launch(60001,at,frame);
        let a=&p.live[0];assert!((a.pos-at).distance(normal*METER*0.25)<0.001);
        let s=p.sprites().remove(0);assert!(s.flat.unwrap().dot(normal).abs()>0.9999);
        assert!(s.stretch.unwrap().dot(normal).abs()<0.0001);
    }
    #[test] fn relative_particles_follow_rotation_translation_and_not_changes_in_model_scale() {
        let mut l=Library::default();l.read("(defpart 60002 :init-specs ((:texture (hotdot effects)) (:func 'spt-func-relative-pos) (:num 1) (:x 12) (:vel-x 2) (:accel-z 0.3) (:timer 3000))) (defpartgroup attached :parts ((sp-item 60002 :flags (is-3d))))");
        let mut p=Particles::new(Arc::new(l));let at=Vec3::new(100.,200.,300.);
        p.keep(123,"attached",at,Mat3::IDENTITY);p.step(1./60.);
        // Stop emission, preserve the already live particle and its binding.
        p.launches[0].fired.fill(true);p.launches[0].carry.fill(-10000.);
        let before=p.live[0].clone();let shift=Vec3::new(70.,-5.,40.);let turn=Mat3::from_rotation_y(std::f32::consts::FRAC_PI_2);
        p.keep(123,"attached",at+shift,turn*Mat3::from_diagonal(Vec3::new(2.,3.,4.)));
        p.step(1./60.);
        let after=&p.live[0];
        let velocity=before.vel+before.accel;let expected=at+shift+turn*(before.pos+velocity*5.-at);
        assert!(after.pos.distance(expected)<0.002,"relative contact frame: {:?} vs {expected:?}",after.pos);
        assert!(after.vel.distance(turn*velocity)<0.001);assert!(after.accel.distance(turn*before.accel)<0.001);
        // A second stationary frame must integrate once, without rotating again.
        let before=after.clone();p.step(1./60.);let after=&p.live[0];
        assert!(after.pos.distance(before.pos+(before.vel+before.accel)*5.)<0.002);
    }
    #[test] fn original_scorpion_casings_bounce_twice_emit_original_impacts_and_expire() {
        let root=std::path::PathBuf::from(std::env::var_os("USERPROFILE").unwrap()).join("Documents/Jak OpenGOAL/active/jak3/data/goal_src/jak3/levels/desert/wvehicle/wvehicle-part.gc");
        let mut l=Library::default();l.read(&std::fs::read_to_string(root).expect("original scorpion particles"));
        let mut p=Particles::new(Arc::new(l));let floor=10.*METER;p.set_target_ground(floor);
        for _ in 0..20 {p.launch(967,Vec3::Y*floor,Mat3::IDENTITY);}
        let mut first=false;let mut second=false;let mut impact=false;
        for _ in 0..300 {
            p.step(1./60.);
            first|=p.live.iter().any(|p|p.part==970);second|=p.live.iter().any(|p|p.part==971);impact|=p.live.iter().any(|p|p.part==974);
            assert!(p.live.iter().filter(|p|p.part==971).all(|p|p.pos.y>=floor-0.001));
        }
        assert!(first&&second&&impact,"native bounce stages: first {first}, second {second}, impact {impact}");
        for _ in 0..3000 {p.step(1./60.);}
        assert!(p.live.is_empty(),"shells finish their authored fade and release all particles");
    }
}

impl Particles {
 pub fn set_target(&mut self,root:Vec3,muzzle:Vec3){self.record(journal::Command::Target(root,muzzle),|p|p.set_target_original(root,muzzle))}
 pub fn set_target_orientation(&mut self,normal:Vec3,yaw:f32){self.record(journal::Command::Orientation(normal,yaw),|p|p.set_target_orientation_original(normal,yaw))}
 pub fn set_particle_velocity(&mut self,velocity:Vec3){self.record(journal::Command::Velocity(velocity),|p|p.set_particle_velocity_original(velocity))}
 pub fn set_target_ground(&mut self,height:f32){self.record(journal::Command::Ground(height),|p|p.set_target_ground_original(height))}
 pub fn launch_from(&mut self,id:u32,source:u64,pos:Vec3,basis:Mat3){self.record(journal::Command::From(id,source,pos,basis),|p|p.launch_from_original(id,source,pos,basis))}
 pub fn launch_peacemaker(&mut self,source:u64,at:Vec3,charge:f32){self.record(journal::Command::Peace(source,at,charge),|p|p.launch_peacemaker_original(source,at,charge))}
 pub fn spawn(&mut self,name:&str,pos:Vec3,basis:Mat3){self.record(journal::Command::Spawn(name.to_owned(),pos,basis),|p|p.spawn_original(name,pos,basis))}
 pub fn spawn_tagged(&mut self,name:&str,pos:Vec3,basis:Mat3,tag:u64){self.record(journal::Command::Tagged(name.to_owned(),pos,basis,tag),|p|p.spawn_tagged_original(name,pos,basis,tag))}
 pub fn stop_tagged(&mut self,tag:u64){self.record(journal::Command::Stop(tag),|p|p.stop_tagged_original(tag))}
 pub fn keep(&mut self,tag:u64,name:&str,pos:Vec3,basis:Mat3){self.record(journal::Command::Keep(tag,name.to_owned(),pos,basis),|p|p.keep_original(tag,name,pos,basis))}
 pub fn keep_attached(&mut self,tag:u64,name:&str,pos:Vec3,basis:Mat3){self.record(journal::Command::Attached(tag,name.to_owned(),pos,basis),|p|p.keep_attached_original(tag,name,pos,basis))}
 pub fn step(&mut self,dt:f32){self.record(journal::Command::Step(dt),|p|p.step_original(dt))}
 pub fn launch(&mut self,id:u32,pos:Vec3,basis:Mat3){self.record(journal::Command::Launch(id,pos,basis),|p|p.launch_original(id,pos,basis))}
 pub fn launch_beam_len(&mut self,id:u32,from:Vec3,dir:Vec3,length:f32){self.record(journal::Command::Beam(id,from,dir,length.min(f32::MAX)),|p|p.launch_beam_len_original(id,from,dir,length))}
 pub fn launch_beam_sized(&mut self,id:u32,from:Vec3,dir:Vec3,length:f32,width:f32){self.record(journal::Command::BeamSized(id,from,dir,length,width),|p|p.launch_beam_sized_original(id,from,dir,length,width))}
 pub fn launch_blade(&mut self,id:u32,from:Vec3,dir:Vec3,normal:Vec3,length:f32,width:f32){self.record(journal::Command::Blade(id,from,dir,normal,length,width),|p|p.launch_blade_original(id,from,dir,normal,length,width))}
 pub fn launch_flat(&mut self,id:u32,pos:Vec3,basis:Mat3,count:f32){self.record(journal::Command::Flat(id,pos,basis,count),|p|p.launch_flat_original(id,pos,basis,count))}
 pub fn launch_scaled(&mut self,id:u32,pos:Vec3,basis:Mat3,scale:f32){self.record(journal::Command::Scaled(id,pos,basis,scale),|p|p.launch_scaled_original(id,pos,basis,scale))}
 pub fn trail_crumb(&mut self,id:u64,pos:Vec3){self.record(journal::Command::TrailCrumb(id,pos),|p|{
  // `water-control-method-17` spawns the tracker; its trail holds a quarter of the max age's ticks in crumbs.
  if !p.trails.iter().any(|t|t.id==id){let appearance=crate::light_trail::WATER_WAKE;let max=(0.25*appearance.max_age as f32) as usize;p.trails.push(crate::light_trail::Trail::new(id,appearance,max,0));}
  if let Some(trail)=p.trails.iter_mut().find(|t|t.id==id){if !trail.ending{trail.add_crumb(pos,5);}}
 })}
 pub fn trail_end(&mut self,id:u64){self.record(journal::Command::TrailEnd(id),|p|{if let Some(trail)=p.trails.iter_mut().find(|t|t.id==id){trail.ending=true;}})}
 pub fn set_part_range(&mut self,part:u32,field:&str,range:Option<f32>){self.record(journal::Command::Range(part,field.to_owned(),range),|p|p.set_part_range_original(part,field,range))}
 pub fn set_part_value(&mut self,part:u32,field:&str,value:Option<f32>){self.record(journal::Command::Value(part,field.to_owned(),value),|p|p.set_part_value_original(part,field,value))}
 pub fn configure_tracker(&mut self,name:&str,userdata:f32,root:Vec3)->Result<(),String>{self.record(journal::Command::Tracker(name.to_owned(),userdata,root),|p|p.configure_tracker_original(name,userdata,root))}
 pub fn set_sources(&mut self,sources:impl IntoIterator<Item=(u64,Vec3)>){let sources:Vec<_>=sources.into_iter().collect();self.record(journal::Command::Sources(sources.clone()),|p|p.set_sources_original(sources));}
 pub fn move_tagged(&mut self,tag:u64,pos:impl Fn(f32)->Vec3){
  let mut points=Vec::new();for l in self.launches.iter().filter(|l|l.tag==Some(tag)){
   let t=self.library.group(&l.group).and_then(|g|l.duration(g)).map_or(0.,|d|(l.age/d.max(1.)).clamp(0.,1.));points.push((t,pos(t)));
  }points.push((1.,pos(1.)));self.record(journal::Command::Move(tag,points),|p|p.move_tagged_original(tag,pos));
 }
 pub fn sync_lightning(&mut self,requests:Vec<LightningRequest>){let owned=requests.iter().map(|r|journal::Lightning{id:r.id,spec:r.spec.into(),from:r.from,to:r.to,age:r.age,probe:r.probe}).collect();self.record(journal::Command::Lightning(owned),|p|p.sync_lightning_original(requests));}
 pub fn start_journal(&mut self){self.journal=Some(Vec::new());}
 pub fn take_journal(&mut self)->Vec<journal::Command>{self.journal.as_mut().map(std::mem::take).unwrap_or_default()}
 fn record<T>(&mut self,command:journal::Command,run:impl FnOnce(&mut Self)->T)->T{
  if self.journal_depth==0{if let Some(journal)=&mut self.journal{journal.push(command);}}
  self.journal_depth+=1;let result=run(self);self.journal_depth-=1;result
 }

}

impl Particles {pub fn launch_var(&mut self,id:u32,pos:Vec3,basis:Mat3,overrides:Overrides,accum:&mut f32){self.record(journal::Command::Var(id,pos,basis,overrides,*accum),|p|p.launch_var_original(id,pos,basis,overrides,accum));}}
