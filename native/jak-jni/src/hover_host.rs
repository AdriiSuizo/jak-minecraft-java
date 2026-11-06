use super::*;
use jni::sys::jboolean;
use crate::wheeled_host::Terrain;
use jak_core::particles::vehicle_lights;
use jak_core::zoomer::{Controls, Event, Model, Zoomer};

type Catalog = HashMap<String, vehicle_lights::Specs>;
static SPECS: OnceLock<Catalog> = OnceLock::new();

struct Session {
    zoomer: Zoomer,
    terrain: Terrain,
    lights: vehicle_lights::State,
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
            })
        })
    {
        return Err("Incomplete original Zoomer light and engine points".into());
    }
    SPECS
        .set(specs)
        .map_err(|_| "Zoomer source configuration raced".into())
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
                },
                lights: Default::default(),
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
        zoomer.routed = buttons & 8 != 0;
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
        if engine != 0 {
            loops.push(json!({"name":format!("{family}_engine_1"),"gain":(0.38+zoomer.thrust.abs()*0.62).min(1.),
                "pitch":(0.75+speed/56.).min(1.7),"pos":[0,0,0]}));
        }
        let points = |src: &[Vec3], div: f32| {
            src.iter()
                .map(|p| (*p / div).to_array())
                .collect::<Vec<_>>()
        };
        Ok(Some(json!({
            "position":(zoomer.position/metres).to_array(),"velocity":(zoomer.velocity/metres).to_array(),
            "seat":seat.to_array(),"rotation":orient.to_array(),"yaw":-zoomer.yaw.to_degrees(),
            "pitch":zoomer.pitch,"roll":zoomer.bank,"grounded":usize::from(zoomer.grounded),
            "climb":zoomer.climb/metres,
            "steer":zoomer.steer,"lean":zoomer.lean,"throttle":zoomer.throttle,"brake":zoomer.brake,
            "boost":false,"thrusterPower":if engine==0{0.}else{zoomer.thrust.max(0.)},
            "high":zoomer.high,"lamps":session.lights.factor,"mask":spec.masks[stage],
            "heads":points(&spec.heads,1.),"tails":points(&spec.tails,1.),
            "thrusters":points(&spec.thrusters,4096.),"exhaust":points(&spec.exhaust,4096.),
            "loops":loops,"emitters":[],"events":events
        }).to_string()))
    });
    encoded
        .and_then(|s| env.new_string(s).ok())
        .map_or(std::ptr::null_mut(), |s| s.into_raw())
}
