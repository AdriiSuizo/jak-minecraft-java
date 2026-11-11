use super::*;
use jni::sys::jboolean;
use crate::wheeled_host::Terrain;
use jak_core::particles::vehicle_lights;
use jak_core::particles::vehicle_effects;
use jak_core::pilot::{self, ClipTiming, Pilot, Smack, Stance};
use jak_core::roll::RollModel;
use jak_core::zoomer::{Controls, Event, Model, Zoomer};

type Catalog = HashMap<String, vehicle_lights::Specs>;
static SPECS: OnceLock<Catalog> = OnceLock::new();

struct Session {
    zoomer: Zoomer,
    terrain: Terrain,
    lights: vehicle_lights::State,
    /// target-pilot-trans, stepped with the vehicle: how Jak leans in the seat.
    pilot: Pilot,
    /// `(current-time)` in 300 Hz ticks, for the pilot's wobble.
    ticks: f64,
    seed: u64,
}

#[derive(Default)]
struct Vehicles {
    next: u64,
    active: HashMap<u64, Session>,
}
static LIVE: OnceLock<Mutex<Vehicles>> = OnceLock::new();
fn vehicles() -> Result<std::sync::MutexGuard<'static, Vehicles>, String> {
    // A panic in one vehicle's step poisons the lock; its Java owner closes that
    // vehicle, so the others keep driving instead of failing forever.
    let mutex = LIVE.get_or_init(Mutex::default);
    match mutex.lock() { Ok(guard) => Ok(guard), Err(poison) => { mutex.clear_poison(); Ok(poison.into_inner()) } }
}

pub(super) fn impulse(id: u64, impulse: Vec3) -> Result<(), String> {
    if let Some(session)=vehicles()?.active.get_mut(&id) { session.zoomer.velocity+=impulse; }
    Ok(())
}

pub fn configure(root: &std::path::Path) -> Result<(), String> {
    if SPECS.get().is_some() {
        return Ok(());
    }
    let source = root
        .parent()
        .and_then(|p| p.parent())
        .ok_or("Invalid owned Jak export")?
        .join("goal_src/jak3/levels/city/traffic/vehicle");
    let mut specs = HashMap::new();
    for name in ["bike.gc", "car.gc"] {
        let path = source.join(name);
        let code =
            std::fs::read_to_string(&path).map_err(|e| format!("{}: {e}", path.display()))?;
        specs.extend(vehicle_lights::read(&code));
    }
    if !["bikea", "bikeb", "bikec", "cara", "carb", "carc"]
        .into_iter()
        .all(|name| {
            specs.get(name).is_some_and(|spec| {
                !spec.heads.is_empty()
                    && !spec.tails.is_empty()
                    && spec.thrusters.len() == 2
                    && spec.exhaust.len() == 2
                    && spec.exhaust_dir.len() == 2
                    && spec.smoke_pos.len() == 2
                    && spec.smoke_vel.len() == 2
                    && spec.flame_length > 0.
                    && spec.max_engine_thrust > 0.
            })
        })
    {
        return Err("Incomplete original Zoomer light and engine points".into());
    }
    SPECS
        .set(specs)
        .map_err(|_| "Zoomer source configuration raced".into())
}

/// The five `jakb-pilot-bike-smack-*` (or `-car-`) clips of `target-pilot-impact`, as the export's art groups time them.
fn smack_timings(bike: bool) -> [Option<ClipTiming>; 5] {
    Smack::ALL.map(|smack| {
        let name = format!("jakb-pilot-{}-{}", if bike { "bike" } else { "car" }, smack.suffix());
        let art = EXTRA_ART.get()?.get(&name)?;
        Some(ClipTiming {
            frames: art.joints.as_ref()?.len(),
            // A frame advances `speed` of a frame each 60 Hz tick.
            frames_per_second: art.speed * 60.,
            base: art.artist_base,
            step: art.artist_step,
        })
    })
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_dev_jakmc_nativebridge_NativeJak_hoverCreate(
    mut env: JNIEnv,
    _: JClass,
    query: JObject,
    model: jint,
    yaw: f32,
    vx: f32,
    vy: f32,
    vz: f32,
    pitch: f32,
    bank: f32,
    high: jboolean,
    seed: jlong,
) -> jlong {
    let binding = env.new_global_ref(query).map_err(|e| e.to_string());
    let vm = env.get_java_vm().map_err(|e| e.to_string());
    protect(&mut env, || {
        if [yaw, vx, vy, vz, pitch, bank]
            .iter()
            .any(|v| !v.is_finite())
        {
            return Err("Nonfinite Zoomer pose".into());
        }
        let model = match model {
            0 => Model::BikeA,
            1 => Model::BikeB,
            2 => Model::BikeC,
            3 => Model::CarA,
            4 => Model::CarB,
            5 => Model::CarC,
            _ => return Err("Unknown Zoomer model".into()),
        };
        let _ = JVM.set(vm?);
        let mut zoomer = Zoomer::new(model, -yaw.to_radians());
        zoomer.velocity = Vec3::new(vx, vy, vz) * (BLOCK / jak_core::METER);
        zoomer.pitch = pitch;
        zoomer.bank = bank;
        zoomer.high = high != 0;
        // hvehicle-physics.gc's roll thrusters, with the rigid body's own numbers.
        zoomer.roll_model = SPECS.get().and_then(|specs| specs.get(model.name())).and_then(RollModel::from_specs);
        zoomer.seed(seed as u64);
        let mut live = vehicles()?;
        live.next += 1;
        let id = live.next;
        live.active.insert(
            id,
            Session {
                zoomer,
                terrain: Terrain {
                    query: binding?,
                    error: None,
                    entity_contact: false,
                },
                lights: Default::default(),
                pilot: {
                    let mut pilot = Pilot::new(if model.car() { Stance::Car } else { Stance::Bike });
                    pilot.set_smacks(smack_timings(!model.car()));
                    pilot
                },
                ticks: 0.,
                seed: seed as u64,
            },
        );
        Ok(id as jlong)
    })
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_dev_jakmc_nativebridge_NativeJak_hoverDestroy(
    mut env: JNIEnv,
    _: JClass,
    id: jlong,
) {
    protect(&mut env, || {
        vehicles()?.active.remove(&(id as u64));
        Ok(())
    });
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_dev_jakmc_nativebridge_NativeJak_hoverStep(
    mut env: JNIEnv,
    _: JClass,
    id: jlong,
    steer: f32,
    lean: f32,
    throttle: f32,
    brake: f32,
    buttons: jint,
    x: f32,
    y: f32,
    z: f32,
    hour: f32,
    health: f32,
    engine: jboolean,
) -> jstring {
    let encoded = protect(&mut env, || {
        if [steer, lean, throttle, brake, x, y, z, hour, health]
            .iter()
            .any(|v| !v.is_finite())
        {
            return Err("Invalid Zoomer controls".into());
        }
        let mut live = vehicles()?;
        let session = live
            .active
            .get_mut(&(id as u64))
            .ok_or("Stale Zoomer session")?;
        let zoomer = &mut session.zoomer;
        let metres = BLOCK / jak_core::METER;
        zoomer.position -= Vec3::new(x, y, z) * metres;
        zoomer.impact_pos -= Vec3::new(x, y, z) * metres;
        zoomer.routed = buttons & 8 != 0;
        // vehicle-flag riding: somebody is aboard, and the hit-points the power fluctuation follows.
        zoomer.riding = buttons & 16 != 0;
        zoomer.hit_points = health;
        let mut control = Controls {
            steer: steer.clamp(-1., 1.),
            lean: lean.clamp(-1., 1.),
            throttle: throttle.clamp(0., 1.),
            brake: brake.clamp(0., 1.),
            lane: buttons & 1 != 0,
            handbrake: buttons & 2 != 0,
            hop: buttons & 4 != 0,
        };
        let mut events = Vec::new();
        let stage = if health < 0.3 {
            2
        } else if health < 0.65 {
            1
        } else {
            0
        };
        let mut lamps_broken = false;
        for _ in 0..3 {
            zoomer.step(control, &mut session.terrain, &mut events);
            control.lane = false;
            session.ticks += 5.;
            // vehicle.gc clear-impulse-force-flag!: lin-acceleration, then target-pilot-trans every frame.
            session.pilot.step(pilot::accel_units(zoomer.accel), zoomer.orientation(), zoomer.steer, zoomer.lean, session.ticks);
            // target-pilot-impact: the smack a hard hit gives him once he is in the stance (32: getting on is over).
            session.pilot.after_step(buttons & 32 != 0);
            lamps_broken |= session.lights.step(
                1. / 60.,
                hour,
                engine != 0,
                stage,
                health >= 1.,
                session.seed,
            );
        }
        if let Some(error) = session.terrain.error.take() {
            return Err(format!("Zoomer collision callback: {error}"));
        }
        let spec = &SPECS.get().unwrap()[zoomer.model.name()];
        let loop_pose = session.pilot.loop_pose();
        let orient = zoomer.orientation();
        let seat = (zoomer.position + orient * zoomer.model.seat()) / metres;
        let mut events = events
            .into_iter()
            .map(|e| match e {
                Event::Lane(up) => json!({"sound":if up{"bike-up"}else{"bike-down"}}),
                Event::Hop => json!({"sound":"bike-hop"}),
                Event::Impact(speed) => json!({"impact":speed}),
            })
            .collect::<Vec<_>>();
        if lamps_broken {
            events.push(json!({"sound":"headlight-pop"}));
        }
        let speed = zoomer.velocity.with_y(0.).length();
        let family = if zoomer.model.car() { "car" } else { "bike" };
        let mut loops = Vec::new();
        // vehicle.gc rigid-body-object-method-38: the scrape sound at the point of the impact, as loud as the envelope.
        if zoomer.scrape > 0. {
            loops.push(json!({"name":format!("{family}_scrape_stn"),"gain":zoomer.scrape,"pitch":1,
                "pos":((zoomer.impact_pos - zoomer.position) / metres).to_array()}));
        }
        // bike.gc rigid-body-object-method-38 (h-bike-base): "bike-thrust" at the roll thrusters' push, four times the
        // push as a share of the engine's thrust, scaled by .32, half an octave up (pitch-mod 1524 an octave).
        // stand-in: sputter-sound-envelope is taken as 1 (it is the engine's power-level, within a hair of it).
        let roll_force = zoomer.roll.force[0] + zoomer.roll.force[1];
        if !zoomer.model.car() && engine != 0 && zoomer.riding && roll_force > 0. && spec.max_engine_thrust > 0. {
            let gain = 0.32 * 4. * roll_force / spec.max_engine_thrust;
            loops.push(json!({"name":"bike_thrust","gain":gain,"pitch":2f32.powf(0.5),"pos":[0,0,0]}));
        }
        if engine != 0 {
            loops.push(json!({"name":format!("{family}_engine_1"),"gain":(0.38+zoomer.thrust.abs()*0.62).min(1.),
                "pitch":(0.75+speed/56.).min(1.7),"pos":[0,0,0]}));
        }
        let points = |src: &[Vec3], div: f32| {
            src.iter()
                .map(|p| (*p / div).to_array())
                .collect::<Vec<_>>()
        };
        let smack = session.pilot.smack();
        let mut frame = json!({
            "position":(zoomer.position/metres).to_array(),"velocity":(zoomer.velocity/metres).to_array(),
            "seat":seat.to_array(),"rotation":orient.to_array(),"yaw":-zoomer.yaw.to_degrees(),
            "pitch":zoomer.pitch,"roll":zoomer.bank,"grounded":usize::from(zoomer.grounded),
            "climb":zoomer.climb/metres,
            "steer":zoomer.steer,"lean":zoomer.lean,"throttle":zoomer.throttle,"brake":zoomer.brake,
            "boost":false,"thrusterPower":if engine==0{0.}else{zoomer.thrust.max(0.)},
            "thrust":zoomer.thrust,"hp":health,"ignition":engine!=0,
            "rollThrust":zoomer.roll_thrust(),"scrape":zoomer.scrape,
            "impactPos":(zoomer.impact_pos/metres).to_array(),
            "rollFlames":if spec.roll_flame{vehicle_effects::ROLL_FLAMES.iter().map(|(at,toward)|{
                let toward=toward.normalize();
                [at.x/4096.,at.y/4096.,at.z/4096.,toward.x,toward.y,toward.z]}).collect::<Vec<_>>()}else{Vec::new()},
            "rollFlame":[vehicle_effects::ROLL_FLAME_WIDTH/4096.,vehicle_effects::ROLL_FLAME_LENGTH/4096.],
            // vehicle-method-113: the pilot's hands, metres: on a bike the seat plus the offset (vehicle axes), on a car the offset in
            // the steering wheel's joint.
            "hands":spec.hand_offsets.iter().map(|hand|((if zoomer.model.car(){Vec3::ZERO}else{spec.seat})+*hand)/4096.).map(|p|p.to_array()).collect::<Vec<_>>(),
            "pilot":[loop_pose.turn,loop_pose.front,loop_pose.vertical,loop_pose.vertical_weight],
            "high":zoomer.high,"lamps":session.lights.factor,"mask":spec.masks[stage],
            "heads":points(&spec.heads,1.),"tails":points(&spec.tails,1.),
            "thrusters":points(&spec.thrusters,4096.),"exhaust":points(&spec.exhaust,4096.),
            "flame":[spec.flame_width/4096.,spec.flame_length/4096.],
            "loops":loops,"emitters":[],"events":events
        });
        if let Some(smack) = smack {
            frame["smack"] = json!({"clip":smack.smack.suffix(),"frame":smack.frame,"weight":smack.weight});
        }
        Ok(Some(frame.to_string()))
    });
    encoded
        .and_then(|s| env.new_string(s).ok())
        .map_or(std::ptr::null_mut(), |s| s.into_raw())
}

/// The engine particles of the zoomers being drawn: one emitter each (the parts' `birthaccum`s and the
/// vehicle's `power-level`), for the particle session it feeds.
static EMITTERS: OnceLock<Mutex<HashMap<u64, vehicle_effects::Emitter>>> = OnceLock::new();
/// What Java hands over a frame of a zoomer: x y z (blocks from the particle session's origin),
/// the bone transform's quaternion x y z w, the velocity (blocks a second), flags (1 ignition,
/// 2 riding, 4 seen already running), engine-thrust, hit-points, the two roll thrusters' thrust, the
/// scrape envelope and the impact point (x y z, blocks from the origin).
const RECORD: usize = 19;

fn model_name(model: jint) -> Option<&'static str> {
    Some(match model {
        0 => Model::BikeA,
        1 => Model::BikeB,
        2 => Model::BikeC,
        3 => Model::CarA,
        4 => Model::CarB,
        5 => Model::CarC,
        _ => return None,
    }
    .name())
}

/// `vehicle-method-78` of the zoomers (vehicle-effects.gc, hvehicle-effects.gc): their engine's
/// particles launched into particle session `id`, once for each of `count` 60 Hz frames, each at the
/// bone transform it is given, as the game launches them in its own frame.
#[unsafe(no_mangle)]
pub extern "system" fn Java_dev_jakmc_nativebridge_NativeJak_fxVehicle(
    mut env: JNIEnv,
    _: JClass,
    id: jlong,
    model: jint,
    records: jni::objects::JFloatArray,
    count: jint,
) -> jint {
    let mut data = vec![0f32; count.clamp(0, 8) as usize * RECORD];
    let read = env
        .get_float_array_region(&records, 0, &mut data)
        .map_err(|e| e.to_string());
    let mut sounds = 0;
    protect(&mut env, || {
        read?;
        if data.iter().any(|v| !v.is_finite()) {
            return Err("Nonfinite zoomer particle frame".into());
        }
        let spec = model_name(model)
            .and_then(|name| SPECS.get()?.get(name))
            .ok_or("Unknown zoomer particle model")?;
        let mut emitters = EMITTERS
            .get_or_init(Mutex::default)
            .lock()
            .unwrap_or_else(|poison| poison.into_inner());
        if emitters.len() > 32 {
            emitters.retain(|key, _| crate::effects::exists(*key));
        }
        let emitter = emitters
            .entry(id as u64)
            .or_insert_with(|| vehicle_effects::Emitter::new(id as u64));
        crate::effects::with_particles(id as u64, |particles| {
            for r in data.chunks_exact(RECORD) {
                let frame = vehicle_effects::Frame {
                    trans: Vec3::new(r[0], r[1], r[2]) * BLOCK,
                    rotation: Quat::from_xyzw(r[3], r[4], r[5], r[6]).normalize(),
                    velocity: Vec3::new(r[7], r[8], r[9]) * BLOCK,
                    ignition: (r[10] as u32) & 1 != 0,
                    riding: (r[10] as u32) & 2 != 0,
                    engine_thrust: r[11],
                    hit_points: r[12],
                    roll_thrust: [r[13], r[14]],
                    scrape: r[15],
                    impact_pos: Vec3::new(r[16], r[17], r[18]) * BLOCK,
                };
                if (r[10] as u32) & 4 != 0 {
                    emitter.adopt(frame.ignition);
                }
                emitter.step(spec, &frame, particles);
                // The sounds the frame asks for: 1 damage-zaps, 2 damage-pops.
                sounds |= i32::from(emitter.zapped) | (i32::from(emitter.popped) << 1);
            }
        })
    });
    sounds
}

/// How long Jak's get-on animations run (pilot-states.gc `target-pilot-get-on`): seconds, and how many
/// artist frames a second they count, from the exported art group.
#[unsafe(no_mangle)]
pub extern "system" fn Java_dev_jakmc_nativebridge_NativeJak_pilotTimings(env: JNIEnv, _: JClass) -> jstring {
    let clip = |name: &str| {
        let art = EXTRA_ART.get()?.get(name)?;
        let frames = art.joints.as_ref()?.len();
        // A frame advances `speed` of a frame each 60 Hz tick.
        let rate = art.speed * 60.;
        Some(json!({
            "seconds": frames.saturating_sub(1) as f32 / rate,
            "aframesPerSecond": rate * art.artist_step,
            "base": art.artist_base
        }))
    };
    let report = json!({"bikeOn": clip("jakb-pilot-bike-get-on"), "carOn": clip("jakb-pilot-car-get-on")});
    env.new_string(report.to_string()).map_or(std::ptr::null_mut(), |s| s.into_raw())
}

/// `target-pilot-get-off`: this Jak, just let off a vehicle, takes the original exit: the get-off
/// animation, then a 2.5 m jump with the vehicle's velocity (and the stick's push) and the fall
/// that follows. `velocity` is the vehicle's, blocks a second.
#[unsafe(no_mangle)]
pub extern "system" fn Java_dev_jakmc_nativebridge_NativeJak_pilotExit(
    mut env: JNIEnv,
    _: JClass,
    id: jlong,
    bike: jboolean,
    vx: f32,
    vy: f32,
    vz: f32,
) {
    protect(&mut env, || {
        let velocity = Vec3::new(vx, vy, vz) * BLOCK;
        if !velocity.is_finite() {
            return Err("Nonfinite pilot exit velocity".into());
        }
        let mut sessions = lock_sessions()?;
        let jak = &mut sessions.live.get_mut(&(id as u64)).ok_or("stale session")?.jak;
        jak.control.transv = velocity;
        jak.pilot_get_off(bike != 0);
        Ok(())
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The six zoomers' engine data from the game's own `bike.gc` and `car.gc` (`JAKMC_ASSETS`, the
    /// export root).
    #[test]
    fn engine_points_come_from_the_originals() {
        let Ok(root) = std::env::var("JAKMC_ASSETS") else {
            eprintln!("JAKMC_ASSETS unset");
            return;
        };
        let dir = std::path::Path::new(&root)
            .parent()
            .and_then(|p| p.parent())
            .unwrap()
            .join("goal_src/jak3/levels/city/traffic/vehicle");
        let mut specs = HashMap::new();
        for name in ["bike.gc", "car.gc"] {
            specs.extend(vehicle_lights::read(&std::fs::read_to_string(dir.join(name)).unwrap()));
        }
        for name in ["bikea", "bikeb", "bikec", "cara", "carb", "carc"] {
            let spec = &specs[name];
            assert_eq!(spec.exhaust.len(), 2, "{name}");
            assert_eq!(spec.exhaust_dir.len(), 2, "{name}");
            assert_eq!(spec.smoke_pos.len(), 2, "{name}");
            assert_eq!(spec.thrusters.len(), 2, "{name}");
            assert!(spec.flame_length > 0. && spec.flame_width > 0., "{name}");
            assert!(spec.max_engine_thrust > 0., "{name}");
            assert!(spec.exhaust_dir[0].z < -0.9, "{name}: the exhaust points back");
        }
        // bike.gc, h-bike-a: exhausts 1024 to each side, 0.05 m up, 0.8 m behind; flames 0.6 by 2 m; 50 m of thrust.
        let bike = &specs["bikea"];
        assert_eq!(bike.exhaust[0], Vec3::new(1024., 204.8, -3276.8));
        assert_eq!(bike.exhaust[1], Vec3::new(-1024., 204.8, -3276.8));
        assert_eq!(bike.smoke_pos[0], Vec3::new(1638.4, -2048., 7372.8));
        assert!((bike.flame_width - 0.6 * 4096.).abs() < 1e-2 && (bike.flame_length - 2. * 4096.).abs() < 1e-2);
        assert_eq!(bike.max_engine_thrust, 50. * 4096.);
        // car.gc, h-car-a: the same, 39 m.
        assert_eq!(specs["cara"].max_engine_thrust, 39. * 4096.);
        // The rigid body and the roll thrusters (hvehicle-physics.gc apply-gravity!), from the same constants.
        for name in ["bikea", "bikeb", "bikec", "cara", "carb", "carc"] {
            let spec = &specs[name];
            let model = RollModel::from_specs(spec).unwrap_or_else(|| panic!("{name}: no roll model"));
            assert_eq!(model.thrusters.len(), 2, "{name}");
            assert!(model.thrusters[0].position.x > 0. && model.thrusters[1].position.x < 0., "{name}: left and right");
            assert!(model.thrusters.iter().all(|t| t.normal.y < 0.), "{name}: the thrusters push up");
            assert!(model.inertia > 0. && model.weight > 0. && model.control > 0. && model.damping > 0.9 && model.damping < 1., "{name}");
            assert_eq!(spec.roll_flame, name.starts_with("bike"), "{name}: only the bikes draw the roll flames");
        }
        // The pilot's hands (vehicle-method-113): the same offsets on the bikes, from each bike's own seat; the car's at the wheel.
        for name in ["bikea", "bikeb", "bikec"] {
            let s = &specs[name];
            assert_eq!(s.hand_offsets, vec![Vec3::new(2457.6, 409.6, 4710.4), Vec3::new(-2457.6, 409.6, 4710.4)], "{name}");
        }
        assert_eq!(specs["bikea"].seat, Vec3::ZERO);
        assert_eq!(specs["bikeb"].seat, Vec3::new(0., 757.76, 1175.552));
        assert_eq!(specs["bikec"].seat, Vec3::new(0., 802.816, 1318.912));
        assert_eq!(specs["cara"].hand_offsets, vec![Vec3::new(614.4, 614.4, 1433.6), Vec3::new(614.4, 614.4, -1433.6)]);
        assert_eq!(specs["cara"].seat, Vec3::new(4079.616, 819.2, -311.296));
        let a = RollModel::from_specs(bike).unwrap();
        // bike.gc h-bike-a: mass 2, 40 m/s^2, a 3 x 4 x 6 m box, 0.995; thrusters 1.7 m out and 0.2 m up of the centre of mass;
        // the rider weighs 40 and sits 0.6 m off the middle when the steering is full.
        assert_eq!((a.mass, a.gravity, a.damping), (2., 40., 0.995));
        assert!((a.inertia - 2. / 12. * (9. + 16.)).abs() < 1e-4, "{}", a.inertia);
        assert!((a.thrusters[0].position - Vec3::new(1.7, 0.2, 0.)).length() < 1e-3, "{:?}", a.thrusters[0].position);
        assert_eq!(a.thrusters[0].normal, Vec3::new(0.3, -0.6, 0.));
        assert!((a.weight - 40.).abs() < 1e-3 && (a.shift - 0.6).abs() < 1e-3, "{} {}", a.weight, a.shift);
    }
}
