//! Gameplay joints for the original 60 Hz effects, independent of the render thread.
use crate::art_group::ArtAnim;
use glam::{Mat4, Quat, Vec3};
use serde_json::Value;
use std::{collections::HashMap, path::Path};

struct Joint {
    parent: Option<usize>,
    position: Vec3,
    rotation: Quat,
}

pub struct BodyJoints {
    joints: Vec<Joint>,
    clips: HashMap<String, Vec<Vec<[f32; 10]>>>,
}

impl BodyJoints {
    pub fn load(root: &Path, authored: &mut HashMap<String, ArtAnim>) -> Result<Self, String> {
        let data = std::fs::read(root.join("levels/common/jakb-c-lod0.glb"))
            .map_err(|e| e.to_string())?;
        let json_length = data.get(12..16).ok_or("Missing Jak GLB header")?;
        let json_length = u32::from_le_bytes(json_length.try_into().unwrap()) as usize;
        let glb: Value = serde_json::from_slice(data.get(20..20 + json_length).ok_or("Missing Jak GLB JSON")?)
            .map_err(|e| e.to_string())?;
        let nodes = glb["nodes"].as_array().ok_or("Missing Jak nodes")?;
        let skin = glb["skins"][0]["joints"].as_array().ok_or("Missing Jak joints")?;
        let joint_nodes: Vec<usize> = skin.iter().map(|node| {
            node.as_u64().filter(|id| *id < nodes.len() as u64).map(|id| id as usize)
                .ok_or("Invalid Jak joint node")
        }).collect::<Result<_, _>>()?;
        let mut parents = vec![None; nodes.len()];
        for (index, node) in nodes.iter().enumerate() {
            if let Some(children) = node["children"].as_array() {
                for child in children {
                    if let Some(slot) = child.as_u64().and_then(|id| parents.get_mut(id as usize)) {
                        *slot = Some(index);
                    }
                }
            }
        }
        let mut joints = Vec::with_capacity(skin.len());
        for (index, &node) in joint_nodes.iter().enumerate() {
            let parent = parents[node].and_then(|p| joint_nodes.iter().position(|&n| n == p));
            if parent.is_some_and(|p| p >= index) {
                return Err("Original Jak joints are not in parent-first order".into());
            }
            let read = |key: &str, i: usize, default: f32| {
                nodes[node][key][i].as_f64().map_or(default, |value| value as f32)
            };
            let position = Vec3::new(read("translation",0,0.0),read("translation",1,0.0),read("translation",2,0.0));
            let rotation = Quat::from_xyzw(read("rotation",0,0.0),read("rotation",1,0.0),
                read("rotation",2,0.0),read("rotation",3,1.0));
            let (position, rotation) = if let Some(values) = nodes[node]["matrix"].as_array() {
                if values.len() != 16 {
                    return Err("Invalid Jak bind matrix".into());
                }
                let matrix = std::array::from_fn(|i| values[i].as_f64().unwrap_or(f64::NAN) as f32);
                let (_, rotation, translation) = Mat4::from_cols_array(&matrix).to_scale_rotation_translation();
                if !rotation.is_finite() || !translation.is_finite() {
                    return Err("Non-finite Jak bind pose".into());
                }
                (translation, rotation)
            } else {
                (position, rotation)
            };
            joints.push(Joint { parent, position, rotation });
        }
        let clips = authored.iter_mut().filter_map(|(name, art)| {
            if !name.starts_with("jakb-") {
                return None;
            }
            let poses = art.joints.take()?;
            // Art frames exclude align and prejoint. The motion solver owns align.
            poses.first().is_some_and(|p| p.len()+2 == joints.len()).then(|| (name.clone(),poses))
        }).collect();
        Ok(Self { joints, clips })
    }

    pub fn sample(&self, jak: &mut jak_core::Jak, rotations: &mut Vec<Quat>) {
        jak.joints.resize(self.joints.len(), Vec3::ZERO);
        rotations.resize(self.joints.len(), Quat::IDENTITY);
        let frames = self.clips.get(jak.chan.anim.name()).map(Vec::as_slice);
        let frame = jak.chan.frame.max(0.0);
        let samples = frames.map(|frames| {
            let lo = (frame as usize).min(frames.len()-1);
            let hi = (lo+1).min(frames.len()-1);
            (&frames[lo], &frames[hi], (frame-lo as f32).clamp(0.0,1.0))
        });
        for (index, joint) in self.joints.iter().enumerate() {
            let (mut position, mut rotation) = (joint.position * jak_core::METER, joint.rotation);
            if let Some((lo, hi, mix)) = samples {
                if index >= 2 {
                    let a = &lo[index-2];
                    let b = &hi[index-2];
                    position = Vec3::from_slice(a).lerp(Vec3::from_slice(b),mix) * jak_core::METER;
                    let qa = Quat::from_slice(&a[3..7]).normalize();
                    let qb = Quat::from_slice(&b[3..7]).normalize();
                    rotation = qa.slerp(qb,mix);
                }
            }
            if let Some(parent) = joint.parent {
                position = jak.joints[parent] + rotations[parent] * position;
                rotation = rotations[parent] * rotation;
            }
            jak.joints[index] = position;
            rotations[index] = rotation;
        }
    }
}
