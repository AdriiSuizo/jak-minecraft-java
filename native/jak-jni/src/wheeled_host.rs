use super::*;
use jak_core::particles::wcars::{self, Specs};
use jak_core::wheeled::{Car, Controls, Event, Ground, Model};

static SPECS: OnceLock<HashMap<String, Specs>> = OnceLock::new();
#[derive(Default)]
struct Cars {
    next: u64,
    active: HashMap<u64, (Car, Terrain)>,
}
static CARS: OnceLock<Mutex<Cars>> = OnceLock::new();

pub fn configure(root: &std::path::Path) -> Result<(), String> {
    if SPECS.get().is_some() {
        return Ok(());
    }
    let path = root
        .parent()
        .and_then(|p| p.parent())
        .ok_or("Invalid Jak 3 export")?
        .join("goal_src/jak3/levels/desert/wvehicle/wcar.gc");
    let file = std::fs::read_to_string(&path).map_err(|e| format!("{}: {e}", path.display()))?;
    let specs = wcars::read(&file);
    if !["toad", "snake"].iter().all(|model| {
        specs.get(*model).is_some_and(|s| {
            s.gears.len() >= 2
                && s.wheels
                    .iter()
                    .all(|w| w.travel > 0. && w.radius > 0. && w.scale > 0.)
        })
    }) {
        return Err("Incomplete original wheeled vehicle specifications".into());
    }
    SPECS
        .set(specs)
        .map_err(|_| "Wheeled vehicle configuration raced".into())
}

fn cars() -> Result<std::sync::MutexGuard<'static, Cars>, String> {
    CARS.get_or_init(Mutex::default)
        .lock()
        .map_err(|e| e.to_string())
}

pub(super) struct Terrain {
    pub(super) query: GlobalRef,
    pub(super) error: Option<String>,
}
impl Ground for Terrain {
    fn sweep_hull(&mut self,at:Vec3,delta:Vec3,half:Vec3,forward:Vec3)->Option<(f32,Vec3)>{
        let result=(||->jni::errors::Result<[f32;4]>{
            let mut env=JVM.get().unwrap().attach_current_thread()?;
            let mut data=[0.0;4];
            let args=[at.x,at.y,at.z,delta.x,delta.y,delta.z,half.x,half.y,half.z,forward.x,forward.y,forward.z].map(JValue::Float);
            let result=env.call_method(self.query.as_obj(),"sweepHull","(FFFFFFFFFFFF)[F",&args).and_then(|value|{
                let array=jni::objects::JFloatArray::from(value.l()?);
                let read=env.get_float_array_region(&array,0,&mut data);let _=env.delete_local_ref(array);read
            });
            if result.is_err(){let _=env.exception_clear();}
            result.map(|_|data)
        })();
        match result{
            Ok([t,x,y,z]) if (0.0..1.0).contains(&t)=>Some((t,Vec3::new(x,y,z))),
            Ok(_)=>None,Err(error)=>{self.error=Some(error.to_string());None}
        }
    }

    fn below(&mut self, at: Vec3, reach: f32) -> Option<f32> {
        let result = (|| -> jni::errors::Result<f32> {
            let mut env = JVM.get().unwrap().attach_current_thread()?;
            let result = env
                .call_method(
                    self.query.as_obj(),
                    "support",
                    "(FFFF)F",
                    &[
                        JValue::Float(at.x),
                        JValue::Float(at.y),
                        JValue::Float(at.z),
                        JValue::Float(reach),
                    ],
                )
                .and_then(|v| v.f());
            if result.is_err() {
                let _ = env.exception_clear();
            }
            result
        })();
        match result {
            Ok(v) => v.is_finite().then_some(v),
            Err(e) => {
                self.error = Some(e.to_string());
                None
            }
        }
    }
    fn sweep(&mut self, at: Vec3, delta: Vec3, half: Vec3) -> Option<(f32, Vec3)> {
        let result = (|| -> jni::errors::Result<[f32; 4]> {
            let mut env = JVM.get().unwrap().attach_current_thread()?;
            let mut data = [0.; 4];
            let args = [
                at.x, at.y, at.z, delta.x, delta.y, delta.z, half.x, half.y, half.z,
            ]
            .map(JValue::Float);
            let result = env
                .call_method(self.query.as_obj(), "sweep", "(FFFFFFFFF)[F", &args)
                .and_then(|value| {
                    let array = jni::objects::JFloatArray::from(value.l()?);
                    let result = env.get_float_array_region(&array, 0, &mut data);
                    let _ = env.delete_local_ref(array);
                    result
                });
            if result.is_err() {
                let _ = env.exception_clear();
            }
            result.map(|_| data)
        })();
        match result {
            Ok([t, x, y, z]) if (0.0..1.0).contains(&t) => Some((t, Vec3::new(x, y, z))),
            Ok(_) => None,
            Err(e) => {
                self.error = Some(e.to_string());
                None
            }
        }
    }
}

fn definition(model: Model) -> &'static Specs {
    &SPECS.get().unwrap()[model.name()]
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_dev_jakmc_nativebridge_NativeJak_carCreate(
    mut env: JNIEnv,
    _: JClass,
    query: JObject,
    model: jint,
    yaw: f32,
    vx: f32,
    vy: f32,
    vz: f32,
    pitch: f32,
    roll: f32,
) -> jlong {
    let binding = env.new_global_ref(query).map_err(|e| e.to_string());
    let jvm = env.get_java_vm().map_err(|e| e.to_string());
    protect(&mut env, || {
        if [yaw, vx, vy, vz, pitch, roll]
            .iter()
            .any(|v| !v.is_finite())
        {
            return Err("Nonfinite wheeled vehicle state".into());
        }
        let _ = JVM.set(jvm?);
        let model = match model {
            0 => Model::Toad,
            1 => Model::Snake,
            _ => return Err("Unknown wheeled vehicle".into()),
        };
        let mut car = Car::new(model, Vec3::ZERO, -yaw.to_radians(), definition(model));
        let metres_per_block = BLOCK / jak_core::METER;
        car.velocity = Vec3::new(vx, vy, vz) * metres_per_block;
        car.pitch = pitch;
        car.roll = roll;
        let mut cars = cars()?;
        cars.next += 1;
        let id = cars.next;
        cars.active.insert(
            id,
            (
                car,
                Terrain {
                    query: binding?,
                    error: None,
                },
            ),
        );
        Ok(id as jlong)
    })
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_dev_jakmc_nativebridge_NativeJak_carDestroy(
    mut env: JNIEnv,
    _: JClass,
    id: jlong,
) {
    protect(&mut env, || {
        cars()?.active.remove(&(id as u64));
        Ok(())
    });
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_dev_jakmc_nativebridge_NativeJak_carStep(
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
    steps: jint,
) -> jstring {
    let encoded = protect(&mut env, || {
        if !(0..=3).contains(&steps)
            || [steer, lean, throttle, brake, x, y, z]
                .iter()
                .any(|v| !v.is_finite())
        {
            return Err("Invalid wheeled vehicle controls".into());
        }
        let mut cars = cars()?;
        let (car, terrain) = cars
            .active
            .get_mut(&(id as u64))
            .ok_or("Stale wheeled vehicle session")?;
        let metres_per_block = BLOCK / jak_core::METER;
        car.position -= Vec3::new(x, y, z) * metres_per_block;
        let c = Controls {
            steer: steer.clamp(-1., 1.),
            lean: lean.clamp(-1., 1.),
            throttle: throttle.clamp(0., 1.),
            brake: brake.clamp(0., 1.),
            hop: buttons & 1 != 0,
            handbrake: buttons & 2 != 0,
            turbo: buttons & 4 != 0,
            fire: buttons & 8 != 0,
        };
        let specs = definition(car.model);
        let mut events = Vec::new();
        for _ in 0..steps {
            car.step(specs, c, terrain, &mut events);
        }
        if let Some(error) = terrain.error.take() {
            return Err(format!("Wheeled collision callback: {error}"));
        }
        let orientation = car.orientation();
        let seat = (car.position + orientation * specs.seat) / metres_per_block;
        let wheels = (0..4)
            .map(|index| {
                let (offset, turn, scale) = car.wheel_pose(specs, index);
                let wheel = &specs.wheels[index / 2];
                let angle = wheel.suspension_angle(car.extension[index]);
                let side = if index % 2 == 0 { 1. } else { -1. };
                let (mut anchor, base, length, sign) = match (car.model, index < 2) {
                    (Model::Toad, _) => (Vec3::new(2.15, 1.3, -2.9), std::f32::consts::PI, 0.9, 1.),
                    (Model::Snake, true) => (
                        Vec3::new(0.92, 0.9, 1.65),
                        25486.223 * std::f32::consts::TAU / 65536.,
                        0.8,
                        side,
                    ),
                    (Model::Snake, false) => (
                        Vec3::new(1.42, 1.28, -2.92),
                        29127.111 * std::f32::consts::TAU / 65536.,
                        0.8,
                        side,
                    ),
                };
                anchor.x *= side;
                let mut end =
                    wheel.anchor + Vec3::new(angle.cos(), angle.sin(), 0.) * (wheel.arm - 0.5);
                end.x *= side;
                let shock = end - anchor;
                let axle = if car.model == Model::Toad && index < 2 {
                    angle - 5355.7476 * std::f32::consts::TAU / 65536.
                } else {
                    side * (angle
                        + if car.model == Model::Snake && index < 2 {
                            std::f32::consts::PI / 30.
                        } else {
                            0.
                        })
                };
                json!({"pos":offset.to_array(),"rotation":turn.to_array(),"scale":scale,"axle":axle,
    "shock":base-shock.x.atan2(shock.y)*sign,"stroke":-(shock.length()-length)*sign})
            })
            .collect::<Vec<_>>();
        let events=events.into_iter().map(|e|match e{
   Event::PrepareHop=>json!({"sound":"toad-prehop"}),
   Event::Jump=>json!({"sound":specs.sounds.get("jump-sound")}),
   Event::Creak=>json!({"sound":specs.sounds.get("susp-creak-sound")}),
   Event::BottomOut=>json!({"sound":specs.sounds.get("susp-bottom-out-sound")}),
   Event::Impact(speed)=>json!({"impact":speed,"sound":if speed>6.{specs.sounds.get("impact-sound")}else{specs.sounds.get("glance-sound")}}),
   Event::Fire(right)=>json!({"fire":right}),
  }).collect::<Vec<_>>();
        let rpm = car.rpm.max(specs.idle_rpm);
        let mix = ((rpm - specs.idle_crossover)
            / (specs.engine_crossover - specs.idle_crossover).max(1.))
        .clamp(0., 1.);
        let mut loops = Vec::new();
        for (key, gain, reference, scale) in [
            (
                "idle-sound",
                (1. - mix).cbrt(),
                specs.audio_idle_rpm,
                specs.idle_pitch_scale,
            ),
            (
                "engine-sound",
                mix.cbrt(),
                specs.audio_engine_rpm,
                specs.engine_pitch_scale,
            ),
        ] {
            if let Some(name) = specs.sounds.get(key).filter(|_| gain > 0.01) {
                loops.push(json!({"name":format!("jak3/{name}"),"gain":gain,"pitch":(rpm/reference.max(1.)).powf(scale),"pos":[0,0,0]}));
            }
        }
        let lateral = car.velocity.dot(Vec3::Y.cross(car.heading())).abs();
        if car.grounded > 0 && lateral > 1.5 {
            loops.push(json!({"name":format!("jak3/{}-skid-road",car.model.name()),"gain":(lateral/8.).min(1.),"pitch":1,"pos":[0,0,0]}));
        }
        if car.boosted
            && let Some(name) = specs.sounds.get("turbo-sound")
        {
            loops.push(json!({"name":format!("jak3/{name}"),"gain":1,"pitch":1,"pos":[0,0,0]}));
        }
        let mut emitters = Vec::new();
        let dust = ((car.velocity.length() - 10.) / 15.).clamp(0., 1.) * 0.25;
        if dust > 0.01 && car.grounded > 0 {
            for (i, extension) in car
                .extension
                .iter()
                .enumerate()
                .filter(|(_, extension)| **extension < 0.99)
            {
                let _ = extension;
                let (hub, _, _) = car.wheel_pose(specs, i);
                let wheel = &specs.wheels[i / 2];
                let point = orientation * hub - Vec3::Y * wheel.radius * wheel.scale;
                emitters.push(json!([964, point.to_array(), dust]));
            }
        }
        let points = |kind: &str| {
            specs
                .points
                .get(kind)
                .into_iter()
                .flatten()
                .map(|point| point.to_array())
                .collect::<Vec<_>>()
        };
        Ok(Some(json!({"position":(car.position/metres_per_block).to_array(),
   "velocity":(car.velocity/metres_per_block).to_array(),"seat":seat.to_array(),"rotation":orientation.to_array(),
   "yaw":-car.yaw.to_degrees(),"pitch":car.pitch,"roll":car.roll,"grounded":car.grounded,
   "steer":car.steer,"rpm":car.rpm,"gear":car.gear,"boost":car.boosted,
   "throttle":car.throttle,"lean":c.lean,"recoil":car.recoil,
   "loops":loops,"emitters":emitters,"thrusters":points("thruster-local-pos"),"exhaust":points("exhaust-local-pos"),
   "blur":car.velocity.dot(car.heading()).abs()/(specs.wheels[1].radius*specs.wheels[1].scale)>20.,
   "wheels":wheels,"events":events}).to_string()))
    });
    encoded
        .and_then(|s| env.new_string(s).ok())
        .map_or(std::ptr::null_mut(), |s| s.into_raw())
}
