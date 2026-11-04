//! Authoritative sockets from the owned rigs, independent of the client render thread.
//! Units here are the export's metres; conversion to Minecraft/native space is last.
use crate::art_group::ArtAnim;
use glam::{Mat4, Quat, Vec3};
use serde_json::Value;
use std::{collections::HashMap, path::Path};

#[derive(Clone, Copy)]
struct Trs {
    t: Vec3,
    r: Quat,
    s: Vec3,
}
impl Trs {
    fn matrix(self) -> Mat4 {
        Mat4::from_scale_rotation_translation(self.s, self.r, self.t)
    }
    fn mix(self, b: Self, w: f32) -> Self {
        Self {
            t: self.t.lerp(b.t, w),
            r: self.r.slerp(b.r, w).normalize(),
            s: self.s.lerp(b.s, w),
        }
    }
    fn from(m: Mat4) -> Self {
        let (s, r, t) = m.to_scale_rotation_translation();
        Self { t, r, s }
    }
}
struct Glb {
    json: Value,
    bin: Vec<u8>,
}
impl Glb {
    fn load(path: &Path) -> Result<Self, String> {
        let b = std::fs::read(path).map_err(|e| format!("{}: {e}", path.display()))?;
        let word = |at: usize| {
            b.get(at..at + 4)
                .map(|v| u32::from_le_bytes(v.try_into().unwrap()) as usize)
                .ok_or("Truncated GLB")
        };
        if b.get(..4) != Some(b"glTF") {
            return Err("Invalid GLB magic".into());
        }
        let n = word(12)?;
        let json = serde_json::from_slice(b.get(20..20 + n).ok_or("Missing GLB JSON")?)
            .map_err(|e| e.to_string())?;
        let size = word(20 + n)?;
        let bin = b
            .get(28 + n..28 + n + size)
            .ok_or("Missing GLB binary")?
            .to_vec();
        Ok(Self { json, bin })
    }
    fn value(&self, accessor: usize, row: usize, column: usize) -> Result<f32, String> {
        let a = &self.json["accessors"][accessor];
        let view = &self.json["bufferViews"]
            [a["bufferView"].as_u64().ok_or("Missing buffer view")? as usize];
        let kind = a["componentType"]
            .as_u64()
            .ok_or("Missing component type")?;
        let bytes = match kind {
            5121 => 1,
            5123 => 2,
            5125 | 5126 => 4,
            _ => return Err("Unsupported rig accessor component".into()),
        };
        let width = match a["type"].as_str().unwrap_or("") {
            "SCALAR" => 1,
            "VEC2" => 2,
            "VEC3" => 3,
            "VEC4" => 4,
            "MAT4" => 16,
            _ => return Err("Unsupported rig accessor type".into()),
        };
        if row >= a["count"].as_u64().unwrap_or(0) as usize || column >= width {
            return Err("Rig accessor outside bounds".into());
        }
        let offset = view["byteOffset"].as_u64().unwrap_or(0) as usize
            + a["byteOffset"].as_u64().unwrap_or(0) as usize
            + row
                * view["byteStride"]
                    .as_u64()
                    .unwrap_or((width * bytes) as u64) as usize
            + column * bytes;
        let data = self
            .bin
            .get(offset..offset + bytes)
            .ok_or("Truncated rig accessor")?;
        let mut value = match kind {
            5121 => data[0] as f32,
            5123 => u16::from_le_bytes(data.try_into().unwrap()) as f32,
            5125 => u32::from_le_bytes(data.try_into().unwrap()) as f32,
            _ => f32::from_le_bytes(data.try_into().unwrap()),
        };
        if a["normalized"] == true {
            value /= match kind {
                5121 => 255.,
                5123 => 65535.,
                _ => 1.,
            };
        }
        if !value.is_finite() {
            return Err("Non-finite rig accessor".into());
        }
        Ok(value)
    }
    fn matrix(&self, accessor: usize, row: usize) -> Result<Mat4, String> {
        let mut values = [0.; 16];
        for (i, v) in values.iter_mut().enumerate() {
            *v = self.value(accessor, row, i)?;
        }
        Ok(Mat4::from_cols_array(&values))
    }
}
struct Clip {
    frames: Vec<Vec<Trs>>,
    prejoint: Option<Vec<Mat4>>,
}
struct Rig {
    names: Vec<String>,
    parents: Vec<Option<usize>>,
    rest: Vec<Trs>,
    inverse: Vec<Mat4>,
    clips: HashMap<String, Clip>,
    durations: HashMap<String, f32>,
    inherit_scale: bool,
}
impl Rig {
    fn load(
        path: &Path,
        clip_path: &Path,
        prefix: &str,
        source: &HashMap<String, ArtAnim>,
        inherit_scale: bool,
    ) -> Result<Self, String> {
        let glb = Glb::load(path)?;
        let nodes = glb.json["nodes"].as_array().ok_or("Missing rig nodes")?;
        let skin = &glb.json["skins"][0];
        let joints = skin["joints"].as_array().ok_or("Missing skin joints")?;
        let ids: Vec<usize> = joints
            .iter()
            .map(|v| v.as_u64().map(|v| v as usize).ok_or("Invalid joint"))
            .collect::<Result<_, _>>()?;
        let mut node_parents = vec![None; nodes.len()];
        for (i, node) in nodes.iter().enumerate() {
            if let Some(children) = node["children"].as_array() {
                for child in children {
                    let n = child.as_u64().ok_or("Invalid child")? as usize;
                    *node_parents.get_mut(n).ok_or("Invalid child node")? = Some(i);
                }
            }
        }
        let mut names = Vec::new();
        let mut parents = Vec::new();
        let mut rest = Vec::new();
        let mut inverse = Vec::new();
        let binds = skin["inverseBindMatrices"]
            .as_u64()
            .ok_or("Missing inverse binds")? as usize;
        for (i, &id) in ids.iter().enumerate() {
            let node = nodes.get(id).ok_or("Missing joint node")?;
            names.push(node["name"].as_str().ok_or("Unnamed joint")?.to_owned());
            let parent = node_parents[id].and_then(|p| ids.iter().position(|&n| n == p));
            if parent.is_some_and(|p| p >= i) {
                return Err("Rig must be parent-first".into());
            }
            parents.push(parent);
            let values = node["matrix"]
                .as_array()
                .ok_or("Original joint has no bind matrix")?;
            if values.len() != 16 {
                return Err("Invalid bind matrix".into());
            }
            let a = std::array::from_fn(|j| values[j].as_f64().unwrap_or(f64::NAN) as f32);
            let trs = Trs::from(Mat4::from_cols_array(&a));
            if !trs.t.is_finite() || !trs.r.is_finite() || !trs.s.is_finite() {
                return Err("Invalid bind transform".into());
            }
            rest.push(trs);
            inverse.push(glb.matrix(binds, i)?);
        }
        let anims = Glb::load(clip_path)?;
        let mut durations = HashMap::new();
        let mut clips = HashMap::new();
        let anim_nodes = anims.json["nodes"]
            .as_array()
            .ok_or("Missing animation nodes")?;
        if let Some(animations) = anims.json["animations"].as_array() {
            for anim in animations {
                let Some(name) = anim["name"].as_str().filter(|n| n.starts_with(prefix)) else {
                    continue;
                };
                let samplers = anim["samplers"]
                    .as_array()
                    .ok_or("Missing animation samplers")?;
                let mut duration = 0f32;
                let mut count = 1;
                for sampler in samplers {
                    let input =
                        sampler["input"].as_u64().ok_or("Missing animation input")? as usize;
                    let n = anims.json["accessors"][input]["count"]
                        .as_u64()
                        .unwrap_or(0) as usize;
                    if n > 0 {
                        duration = duration.max(anims.value(input, n - 1, 0)?);
                        count = count.max(n);
                    }
                }
                let mut frames = vec![rest.clone(); count];
                for channel in anim["channels"]
                    .as_array()
                    .ok_or("Missing animation channels")?
                {
                    let node = channel["target"]["node"]
                        .as_u64()
                        .ok_or("Missing channel node")? as usize;
                    let Some(joint) = anim_nodes
                        .get(node)
                        .and_then(|n| n["name"].as_str())
                        .and_then(|n| names.iter().position(|v| v == n))
                    else {
                        continue;
                    };
                    let sampler = &samplers[channel["sampler"]
                        .as_u64()
                        .ok_or("Missing channel sampler")?
                        as usize];
                    let output =
                        sampler["output"].as_u64().ok_or("Missing channel output")? as usize;
                    let n = anims.json["accessors"][output]["count"]
                        .as_u64()
                        .unwrap_or(0) as usize;
                    if n == 0 {
                        continue;
                    }
                    let path = channel["target"]["path"].as_str().unwrap_or("");
                    for (row, frame) in frames.iter_mut().enumerate() {
                        let at = row.min(n - 1);
                        if path == "rotation" {
                            frame[joint].r = Quat::from_xyzw(
                                anims.value(output, at, 0)?,
                                anims.value(output, at, 1)?,
                                anims.value(output, at, 2)?,
                                anims.value(output, at, 3)?,
                            )
                            .normalize();
                        } else if path == "translation" || path == "scale" {
                            let value = Vec3::new(
                                anims.value(output, at, 0)?,
                                anims.value(output, at, 1)?,
                                anims.value(output, at, 2)?,
                            );
                            if path == "translation" {
                                frame[joint].t = value;
                            } else {
                                frame[joint].s = value;
                            }
                        }
                    }
                }
                let prejoint = source
                    .get(name)
                    .and_then(|a| a.prejoint.as_ref())
                    .map(|p| p.iter().map(Mat4::from_cols_array).collect());
                durations.insert(name.to_owned(), duration);
                clips.insert(name.to_owned(), Clip { frames, prejoint });
            }
        }
        Ok(Self {
            names,
            parents,
            rest,
            inverse,
            clips,
            durations,
            inherit_scale,
        })
    }
    fn index(&self, name: &str) -> Result<usize, String> {
        self.names
            .iter()
            .position(|n| n == name)
            .ok_or_else(|| format!("Missing original joint {name}"))
    }
    fn duration(&self, name: &str) -> f32 {
        self.durations.get(name).copied().unwrap_or(0.)
    }
    fn local(&self, name: &str, frame: f32) -> Vec<Trs> {
        let Some(clip) = self.clips.get(name) else {
            return self.rest.clone();
        };
        let frames = &clip.frames;
        let f = frame.clamp(0., (frames.len() - 1) as f32);
        let lo = f as usize;
        let hi = (lo + 1).min(frames.len() - 1);
        let w = f - lo as f32;
        let mut out: Vec<_> = frames[lo]
            .iter()
            .zip(&frames[hi])
            .map(|(a, b)| a.mix(*b, w))
            .collect();
        if self.inherit_scale {
            if let Some(pre) = clip.prejoint.as_ref().filter(|p| !p.is_empty()) {
                let at = frame.clamp(0., (pre.len() - 1) as f32);
                let lo = at as usize;
                let hi = (lo + 1).min(pre.len() - 1);
                if pre[lo].determinant().abs() > 1e-12 && pre[hi].determinant().abs() > 1e-12 {
                    let mut a = Trs::from(pre[lo]);
                    a.t /= jak_core::METER;
                    let mut b = Trs::from(pre[hi]);
                    b.t /= jak_core::METER;
                    out[1] = a.mix(b, at - lo as f32);
                }
            }
        }
        out
    }
    fn at_time(&self, name: &str, age: f32) -> Vec<Trs> {
        let count = self.clips.get(name).map_or(1, |c| c.frames.len());
        self.local(
            name,
            if self.duration(name) > 0. {
                (age / self.duration(name)).clamp(0., 1.) * (count - 1) as f32
            } else {
                0.
            },
        )
    }
    fn resolve(&self, local: &[Trs]) -> Vec<Mat4> {
        let mut world = Vec::with_capacity(local.len());
        for (i, trs) in local.iter().enumerate() {
            let parent = self.parents[i].map(|p| {
                if self.inherit_scale {
                    world[p]
                } else {
                    let t = Trs::from(world[p]);
                    Mat4::from_rotation_translation(t.r, t.t)
                }
            });
            world.push(parent.map_or_else(|| trs.matrix(), |p| p * trs.matrix()));
        }
        world
    }
    fn blend(a: &mut [Trs], b: &[Trs], weight: f32) {
        let w = weight.clamp(0., 1.);
        for (a, b) in a.iter_mut().zip(b) {
            *a = a.mix(*b, w);
        }
    }
    // The cinematic mesh is worn on the gameplay rig, exactly as on the client.
    fn height_bounds(&self, path: &Path, pose: &[Mat4], root: Mat4) -> Result<(f32, f32), String> {
        let mesh = Glb::load(path)?;
        let nodes = mesh.json["nodes"]
            .as_array()
            .ok_or("Missing cinematic nodes")?;
        let ids = mesh.json["skins"][0]["joints"]
            .as_array()
            .ok_or("Missing cinematic skin")?;
        let mut parents = vec![None; nodes.len()];
        for (i, n) in nodes.iter().enumerate() {
            if let Some(children) = n["children"].as_array() {
                for c in children {
                    *parents
                        .get_mut(c.as_u64().ok_or("Invalid cinematic child")? as usize)
                        .ok_or("Invalid cinematic child node")? = Some(i);
                }
            }
        }
        let mut mapping = Vec::new();
        for id in ids {
            let mut node = Some(id.as_u64().ok_or("Invalid cinematic joint")? as usize);
            let mut target = None;
            while let Some(i) = node {
                let n = nodes.get(i).ok_or("Missing cinematic joint node")?;
                target = n["name"]
                    .as_str()
                    .and_then(|name| self.names.iter().position(|v| v == name));
                if target.is_some() {
                    break;
                }
                node = parents[i];
            }
            mapping.push(target.unwrap_or(0));
        }
        let mut low = f32::INFINITY;
        let mut high = f32::NEG_INFINITY;
        for m in mesh.json["meshes"]
            .as_array()
            .ok_or("Missing cinematic meshes")?
        {
            for primitive in m["primitives"]
                .as_array()
                .ok_or("Missing cinematic primitives")?
            {
                let a = &primitive["attributes"];
                let position = a["POSITION"].as_u64().ok_or("Missing mesh position")? as usize;
                let indices = primitive["indices"].as_u64().map(|v| v as usize);
                let count = mesh.json["accessors"][indices.unwrap_or(position)]["count"]
                    .as_u64()
                    .ok_or("Missing mesh count")? as usize;
                for row in 0..count {
                    let vertex = if let Some(indices) = indices {
                        mesh.value(indices, row, 0)? as usize
                    } else {
                        row
                    };
                    let p = Vec3::new(
                        mesh.value(position, vertex, 0)?,
                        mesh.value(position, vertex, 1)?,
                        mesh.value(position, vertex, 2)?,
                    );
                    let mut skinned = Vec3::ZERO;
                    let mut total = 0.;
                    for channel in 0..2 {
                        let Some(joints) = a[format!("JOINTS_{channel}")].as_u64() else {
                            continue;
                        };
                        let weights = a[format!("WEIGHTS_{channel}")]
                            .as_u64()
                            .ok_or("Missing mesh weights")?
                            as usize;
                        for slot in 0..4 {
                            let weight = mesh.value(weights, vertex, slot)?;
                            if weight <= 0. {
                                continue;
                            }
                            let joint = mesh.value(joints as usize, vertex, slot)? as usize;
                            let target = *mapping.get(joint).ok_or("Mesh joint outside rig")?;
                            skinned +=
                                (pose[target] * self.inverse[target]).transform_point3(p) * weight;
                            total += weight;
                        }
                    }
                    let y = root
                        .transform_point3(if total > 0. { skinned / total } else { p })
                        .y;
                    low = low.min(y);
                    high = high.max(y);
                }
            }
        }
        if !low.is_finite() || !high.is_finite() || high <= low {
            return Err("Invalid cinematic bounds".into());
        }
        Ok((low, high))
    }
}

pub struct PosedGun {
    body: Rig,
    gun: Rig,
    body_gun: usize,
    muzzle: usize,
    upper: usize,
    pub scale: f32,
    pub floor: f32,
}
impl PosedGun {
    pub fn load(root: &Path, source: &HashMap<String, ArtAnim>) -> Result<Self, String> {
        let common = root.join("levels/common");
        let body = Rig::load(
            &common.join("jakb-c-lod0.glb"),
            &common.join("jakb-lod0.glb"),
            "jakb-",
            source,
            false,
        )?;
        // Other level art groups can contain same-name gun clips with cinematic
        // scale overrides. Match the client: the gameplay gun owns its prejoint.
        let gun_art = crate::art_group::read_named(
            &std::fs::read(root.join("raw_obj/gun-ag.go")).map_err(|e| e.to_string())?,
            "gun-",
        );
        let gun = Rig::load(
            &common.join("gun-lod0.glb"),
            &common.join("gun-lod0.glb"),
            "gun-",
            &gun_art,
            true,
        )?;
        let dax = Rig::load(
            &common.join("daxter-lod0.glb"),
            &common.join("daxter-lod0.glb"),
            "daxter-",
            source,
            false,
        )?;
        let stance = body.resolve(&body.local("jakb-stance-loop", 0.));
        let anchor = stance[body.index("LshoulderPad")?].w_axis.truncate();
        let a = body.height_bounds(
            &root.join("levels/arenacst/jakc-highres-lod0.glb"),
            &stance,
            Mat4::IDENTITY,
        )?;
        let b = dax.height_bounds(
            &root.join("levels/arenacst/daxter-highres-lod0.glb"),
            &dax.resolve(&dax.local("daxter-stance-loop", 0.)),
            Mat4::from_scale_rotation_translation(Vec3::splat(1. / 1.4), Quat::IDENTITY, anchor),
        )?;
        let floor = -a.0.min(b.0);
        let scale = 1.94 / (a.1.max(b.1) - a.0.min(b.0));
        let body_gun = body.index("gun")?;
        let upper = body.index("upper_body")?;
        let muzzle = gun.index("muzzle")?;
        Ok(Self {
            body,
            gun,
            body_gun,
            muzzle,
            upper,
            scale,
            floor,
        })
    }
    pub fn muzzle(&self, j: &jak_core::Jak, upper_yaw: f32) -> Vec3 {
        let clip = j.chan.anim.name();
        let frame = j.chan.frame;
        let mut local = self.body.local(clip, frame);
        if let Some(mix) = j.chan.mix {
            let clips = mix.anims();
            let gait = |start: usize| {
                let mut base = self.body.local(clips[start].name(), frame);
                Rig::blend(
                    &mut base,
                    &self.body.local(clips[start + 1].name(), frame),
                    mix.up.abs(),
                );
                Rig::blend(
                    &mut base,
                    &self.body.local(clips[start + 2].name(), frame),
                    mix.side.abs(),
                );
                base
            };
            local = gait(0);
            Rig::blend(&mut local, &gait(3), mix.run);
        }
        let kind = j.gun.equipped.flatten().unwrap_or(j.gun.kind) as usize;
        let color = ["red", "yellow", "blue", "dark"][kind / 3];
        let time = j.time as f32 / 300.;
        let fire = if j.gun.fire_time > 0 {
            (j.time - j.gun.fire_time) as f32 / 300.
        } else {
            999.
        };
        if j.gun.out
            && !j.dark
            && !j.lightjak.shield
            && !j.state.is_board()
            && !j.state.is_flut()
            && (clip.contains("stance") || clip.contains("walk") || clip.contains("jump"))
        {
            let take = format!("jakb-gun-{color}-takeout");
            let draw = (j.time - j.gun.get_on_time) as f32 / 300.;
            let idle = match kind / 3 {
                0 => "jakb-gun-stance",
                1 => "jakb-gun-stance-yellow",
                2 => "jakb-gun-stance-blue",
                _ => "jakb-gun-stance-dark",
            };
            let fire_body = [
                "jakb-gun-red-fire",
                "jakb-gun-red-fire-2",
                "jakb-gun-red-fire",
                "jakb-gun-yellow-fire",
                "jakb-gun-yellow-fire",
                "jakb-gun-yellow-fire-3",
                "jakb-gun-blue-fire-single",
                "jakb-gun-blue-fire-2",
                "jakb-gun-blue-fire-single",
                "jakb-gun-dark-fire",
                "jakb-gun-dark-fire",
                "jakb-gun-dark-fire",
            ];
            let (name, age) = if draw >= 0. && draw < self.body.duration(&take) {
                (take.as_str(), draw)
            } else if (0. ..0.45).contains(&fire) {
                (fire_body[kind], fire)
            } else if clip.contains("walk") {
                let run = j.chan.mix.is_some_and(|m| m.run > 0.5);
                let walk = if kind / 3 == 2 {
                    if run {
                        "jakb-gun-run-blue"
                    } else {
                        "jakb-gun-walk-blue"
                    }
                } else if run {
                    "jakb-gun-front-run"
                } else {
                    "jakb-gun-front-walk"
                };
                let count = self.body.clips.get(clip).map_or(1, |c| c.frames.len());
                (
                    walk,
                    frame / (count.saturating_sub(1).max(1) as f32) * self.body.duration(walk),
                )
            } else {
                (idle, time % self.body.duration(idle).max(0.001))
            };
            let overlay = self.body.at_time(name, age);
            for i in 0..local.len() {
                let mut node = Some(i);
                while let Some(n) = node {
                    if n == self.upper {
                        local[i] = overlay[i];
                        break;
                    }
                    node = self.body.parents[n];
                }
            }
        }
        if upper_yaw != 0. {
            let world = self.body.resolve(&local);
            let parent =
                self.body.parents[self.upper].map_or(Quat::IDENTITY, |p| Trs::from(world[p]).r);
            local[self.upper].r = (parent.conjugate()
                * Quat::from_rotation_y(upper_yaw)
                * parent
                * local[self.upper].r)
                .normalize();
        }
        let body = self.body.resolve(&local);
        let hand = Trs::from(body[self.body_gun]);
        let suffix = if kind % 3 == 0 {
            String::new()
        } else {
            (kind % 3 + 1).to_string()
        };
        let idle = format!("gun-idle-{color}{suffix}");
        let fire_names = [
            "gun-gun-red-fire",
            "gun-gun-red-fire-2",
            "gun-gun-red-fire-3",
            "gun-gun-yellow-fire",
            "gun-gun-yellow-fire-2",
            "gun-gun-yellow-fire-3",
            "gun-gun-blue-fire-single",
            "gun-gun-blue-fire-2",
            "gun-gun-blue-fire-3-single",
            "gun-gun-dark-fire",
            "gun-gun-dark-fire-2",
            "gun-gun-dark-fire-3",
        ];
        let gun = self.gun.resolve(&self.gun.at_time(
            if (0. ..0.45).contains(&fire) {
                fire_names[kind]
            } else {
                &idle
            },
            if (0. ..0.45).contains(&fire) {
                fire
            } else {
                time % self.gun.duration(&idle).max(0.001)
            },
        ));
        let hand = Mat4::from_rotation_translation(hand.r, hand.t);
        let nozzle = gun[self.muzzle].w_axis.truncate();
        let position = hand.transform_point3(nozzle);
        (position + Vec3::Y * self.floor) * self.scale * crate::BLOCK
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn owned_rigs_supply_all_twelve_animated_muzzles() {
        let Some(home) = std::env::var_os("USERPROFILE") else {
            return;
        };
        let root = std::path::PathBuf::from(home)
            .join("Documents/Jak OpenGOAL/active/jak3/data/decompiler_out/jak3");
        if !root.is_dir() {
            return;
        }
        let mut art = HashMap::new();
        for entry in std::fs::read_dir(root.join("raw_obj")).unwrap() {
            let entry = entry.unwrap();
            let name = entry.file_name().to_string_lossy().into_owned();
            if name.ends_with("-ag.go")
                && (name.starts_with("jakb")
                    || name.starts_with("daxter")
                    || name.starts_with("gun"))
            {
                art.extend(crate::art_group::read(
                    &std::fs::read(entry.path()).unwrap(),
                ));
            }
        }
        // A same-name cinematic track must never override gameplay gun scale.
        if let Some(a) = art.get_mut("gun-gun-red-fire") {
            a.prejoint = Some(vec![Mat4::from_scale(Vec3::splat(1.2)).to_cols_array()]);
        }
        let rig = PosedGun::load(&root, &art).unwrap();
        println!(
            "visible rig scale={} floor={} clips={} gunclips={} bodyIdle={} gunIdle={}",
            rig.scale,
            rig.floor,
            rig.body.clips.len(),
            rig.gun.clips.len(),
            rig.body.clips.contains_key("jakb-gun-stance"),
            rig.gun.clips.contains_key("gun-idle-red")
        );
        assert!((0.5..1.).contains(&rig.scale));
        let mut j = jak_core::Jak::new(Vec3::ZERO, 0.);
        j.gun.out = true;
        j.time = 3000;
        j.time = 240;
        j.gun.fire_time = 230;
        j.gun.get_on_time = 5;
        j.chan.anim = jak_core::anim::JUMP_LAND;
        j.chan.frame = 10.985157;
        j.gun.kind = jak_core::weapons::GunKind::ALL[0];
        // Independent actual Java GltfModel result captured at this authored frame.
        let recoil = rig.muzzle(&j, 0.);
        assert!(
            (recoil / crate::BLOCK).distance(Vec3::new(-0.31616175, 1.2937366, 0.45767182))
                < 0.00001
        );
        j.time = 3000;
        for kind in 0..12 {
            j.gun.kind = jak_core::weapons::GunKind::ALL[kind];
            j.gun.fire_time = 0;
            let idle = rig.muzzle(&j, 0.) / crate::BLOCK;
            assert!(
                idle.is_finite() && (0.2..2.).contains(&idle.y),
                "mode {kind}: {idle:?}"
            );
            j.gun.fire_time = j.time - 30;
            let fired = rig.muzzle(&j, 0.) / crate::BLOCK;
            assert!(
                fired.is_finite() && (0.2..2.).contains(&fired.y),
                "mode {kind}: {fired:?}"
            );
            assert!(
                idle.distance(fired) > 0.001,
                "mode {kind} lost the authored recoil socket"
            );
        }
    }
}
