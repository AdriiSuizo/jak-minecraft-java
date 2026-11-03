use super::*;
use jak_core::hellcat::{HellCat, HellCatEvent};
use jak_core::pad::button;
use jak_core::particles::vehicle_lights::State as Lamps;

struct Vehicle {
    craft: HellCat,
    world: World,
    actors: Actors,
    lamps: Lamps,
    key: u64,
}

#[derive(Default)]
struct Vehicles {
    next: u64,
    active: HashMap<u64, Vehicle>,
}

static VEHICLES: OnceLock<Mutex<Vehicles>> = OnceLock::new();

fn vehicles() -> Result<std::sync::MutexGuard<'static, Vehicles>, String> {
    VEHICLES.get_or_init(Mutex::default).lock().map_err(|e| e.to_string())
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_dev_jakmc_nativebridge_NativeJak_hellcatCreate(
    mut env: JNIEnv, _: JClass, query: JObject, yaw: f32, vx: f32, vy: f32,
    vz: f32, health: f32, flags: jint, torpedoes: jint, key: jlong,
) -> jlong {
    let binding = env.new_global_ref(query).map_err(|e| e.to_string());
    let vm = env.get_java_vm().map_err(|e| e.to_string());
    protect(&mut env, || {
        if [yaw, vx, vy, vz, health].iter().any(|v| !v.is_finite()) {
            return Err("Nonfinite HellCat state".into());
        }
        let _ = JVM.set(vm?);
        let mut craft = HellCat::new(Vec3::ZERO, -yaw.to_radians());
        craft.vel = Vec3::new(vx, vy, vz) * BLOCK;
        craft.health = health.clamp(-1.0, 1.0);
        craft.hold = flags & 1 != 0;
        craft.idling = flags & 2 != 0;
        craft.destroyed = craft.health < 0.0;
        craft.torpedoes = torpedoes.clamp(0, 8) as u8;
        let mut lamps = Lamps::default();
        lamps.dead = flags & 4 != 0;
        let world = World { query: Some(binding?), ..World::default() };
        let mut vehicles = vehicles()?;
        vehicles.next += 1;
        let id = vehicles.next;
        vehicles.active.insert(id, Vehicle { craft, world, lamps, actors: Actors::default(), key: key as u64 });
        Ok(id as jlong)
    })
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_dev_jakmc_nativebridge_NativeJak_hellcatDestroy(
    mut env: JNIEnv, _: JClass, id: jlong,
) {
    protect(&mut env, || {
        vehicles()?.active.remove(&(id as u64));
        Ok(())
    });
}

fn flight_pad(mut input: PadInput) -> PadInput {
    fn remap(bits: u32) -> u32 {
        (bits & !(button::X | button::CIRCLE))
            | if bits & button::X != 0 { button::CIRCLE } else { 0 }
            | if bits & button::CIRCLE != 0 { button::X } else { 0 }
    }
    input.held = remap(input.held);
    input.pressed = remap(input.pressed);
    input
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_dev_jakmc_nativebridge_NativeJak_hellcatStep(
    mut env: JNIEnv, _: JClass, id: jlong, input: JByteBuffer, actors: JByteBuffer,
    x: f32, y: f32, z: f32, hour: f32, commands: jint, steps: jint,
) -> jstring {
    let bytes = |b: &JByteBuffer| -> Result<&[u8], String> {
        let n = env.get_direct_buffer_capacity(b).map_err(|e| e.to_string())?;
        let p = env.get_direct_buffer_address(b).map_err(|e| e.to_string())?;
        Ok(unsafe { std::slice::from_raw_parts(p, n) })
    };
    let controls = bytes(&input);
    let actor_data = bytes(&actors);
    let encoded: Option<String> = protect(&mut env, || -> Result<Option<String>, String> {
        if !(0..=3).contains(&steps) || [x,y,z,hour].iter().any(|v| !v.is_finite()) {
            return Err("Invalid HellCat frame".into());
        }
        let b = controls?;
        let a = actor_data?;
        if b.len() != 32 || a.len() % 32 != 0 || a.len() > 262_144
            || b[8..].chunks_exact(4).any(|n| !f32::from_le_bytes(n.try_into().unwrap()).is_finite()) {
            return Err("Invalid HellCat packet".into());
        }
        let mut vehicles = vehicles()?;
        let v = vehicles.active.get_mut(&(id as u64)).ok_or("Stale HellCat")?;
        let mut pad = flight_pad(PadInput {
            held: read_u(b, 0) & 0xffff, pressed: read_u(b, 4) & 0xffff,
            left: [read_f(b, 8), read_f(b, 12)], right: [read_f(b, 16), read_f(b, 20)],
            triggers: [read_f(b, 24), read_f(b, 28)],
        });
        v.actors.boxes.clear();
        for row in a.chunks_exact(32) {
            let lo = vec(row, 8) * BLOCK;
            let hi = vec(row, 20) * BLOCK;
            if !lo.is_finite() || !hi.is_finite() || !lo.cmple(hi).all() {
                return Err("Invalid HellCat actor bounds".into());
            }
            v.actors.boxes.push((u64::from_le_bytes(row[..8].try_into().unwrap()),lo,hi));
        }
        let craft = &mut v.craft;
        craft.reframe(Quat::IDENTITY, -Vec3::new(x,y,z) * BLOCK);
        if commands & 1 != 0 && !craft.destroyed { craft.board(); }
        if commands & 2 != 0 && craft.piloted {
            craft.leave();
            craft.vel = Vec3::ZERO;
        }
        if commands & 4 != 0 {
            if craft.idling || craft.hold {
                craft.idling = false;
                craft.hold = false;
                craft.vel.y = -jak_core::math::meters(4.0);
            }
            craft.damage();
        }
        let mut popped = false;
        for _ in 0..steps {
            craft.step(&pad, &mut v.world, Some(&mut v.actors));
            pad.pressed = 0;
            let stage = ((1.0 - craft.health.clamp(0.0, 1.0)) * 3.0) as usize;
            popped |= v.lamps.step(1.0/60.0, hour, !craft.destroyed && (craft.piloted || craft.idling),
                stage, craft.health >= 1.0, v.key);
        }
        if let Some(error) = v.world.error.take() { return Err(error); }
        let events = std::mem::take(&mut craft.events).into_iter().map(|event| match event {
            HellCatEvent::Fire { pos:p } => json!({"kind":"fire","pos":pos(p)}),
            HellCatEvent::FireAlt { pos:p } => json!({"kind":"fire_alt","pos":pos(p)}),
            HellCatEvent::Crash { pos:p,speed } => json!({"kind":"crash","pos":pos(p),"speed":speed}),
            HellCatEvent::Brush { pos:p,normal } => json!({"kind":"brush","pos":pos(p),"normal":normal.to_array()}),
            HellCatEvent::Impact(hit) => json!({"kind":"impact","actor":hit.actor,"pos":pos(hit.surface),"damage":hit.damage}),
            HellCatEvent::Blast { center } => json!({"kind":"blast","pos":pos(center)}),
            HellCatEvent::Explode { center } => json!({"kind":"explode","pos":pos(center)}),
        }).collect::<Vec<_>>();
        let beams = craft.beams().into_iter().map(|b| json!({
            "head":pos(b.head),"tail":pos(b.tail),"rgba":b.rgba,"width":b.width,
        })).collect::<Vec<_>>();
        let flags = u8::from(craft.hold) | (u8::from(craft.idling)<<1) | (u8::from(v.lamps.dead)<<2);
        Ok(Some(json!({
            "position":pos(craft.trans),"velocity":pos(craft.vel),
            "seat":pos(craft.seat()),"rotation":craft.orientation().to_array(),
            "heading":craft.heading().to_array(),"health":craft.health,"flags":flags,
            "pilot":craft.piloted,"destroyed":craft.destroyed,"torpedoes":craft.torpedoes,
            "engine":craft.engine,"wind":craft.wind,"boost":craft.boosting,
            "steering":craft.steering,"lean":craft.lean,"throttle":craft.throttle,
            "lamps":v.lamps.factor,"lampsPopped":popped,"events":events,"beams":beams,
        }).to_string()))
    });
    encoded.and_then(|s|env.new_string(s).ok()).map_or(std::ptr::null_mut(), |s|s.into_raw())
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_dev_jakmc_nativebridge_NativeJak_hellcatFlames(
    env: JNIEnv, _: JClass, thrust: f32, age: jint, out: JByteBuffer,
) -> jint {
    if !thrust.is_finite() { return 0; }
    let Ok(n) = env.get_direct_buffer_capacity(&out) else { return 0 };
    let Ok(p) = env.get_direct_buffer_address(&out) else { return 0 };
    if n != 120 * 36 { return 0; }
    let buffer = unsafe { std::slice::from_raw_parts_mut(p, n) };
    let mut craft = HellCat::new(Vec3::ZERO, 0.0);
    craft.time = i64::from(age) * jak_core::FRAME_TICKS;
    let sprites = craft.thruster_sprite_pose(thrust, true);
    for (sprite, row) in sprites.iter().zip(buffer.chunks_exact_mut(36)) {
        let mut values = [0.; 9];
        values[..3].copy_from_slice(&(sprite.pos / BLOCK).to_array());
        values[3] = sprite.size[0] / BLOCK;
        values[4..8].copy_from_slice(&sprite.rgba);
        values[8] = f32::from(sprite.texture == "glow");
        for (n, bytes) in values.into_iter().zip(row.chunks_exact_mut(4)) {
            bytes.copy_from_slice(&n.to_le_bytes());
        }
    }
    sprites.len() as jint
}
