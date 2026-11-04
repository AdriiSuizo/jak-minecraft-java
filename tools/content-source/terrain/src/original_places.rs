//! The Jak games' own places in the block world, at one block a metre, as
//! pieces of their levels (`jak_pieces`: each metre drawn as the level's own
//! triangles in it; Adrii, 23/10: "que la forma sea la de las cosas de
//! jak", not Minecraft cubes in Jak's textures):
//! - Spargus, Jak 3's city (`wascitya`, `wascityb`, the palace `waspala`,
//!   the gates `wasdoors`, the arena `wasstada`), in the Wasteland;
//! - Haven City's slums, Jak 2 (`ctysluma`, `ctyslumb`, `ctyslumc`), on
//!   plains;
//! - Sandover, Jak 1 (`village1`), on a beach;
//! - Snowy Mountain, Jak 1 (`snow`), in the snowy lands;
//! - Boggy Swamp, Jak 1 (`swamp`), in the swamps.
//!
//! Where a place stands (its crop, and any column with a piece of it) the
//! land is cleared down to the place's lowest piece round there and to its
//! sea's height (the game's own: its ocean map, or its water), the sea put
//! back under that, and the ground under each lowest floor filled with
//! pieces of nothing of the level: whole blocks of the floor over them
//! (a hill's inside, solid to stand on and drawn where it meets the air). Each stands,
//! once a world, at the nearest land of its kind to the origin, apart from
//! the others. None
//! where the export is not there. Its textures are the ones its levels
//! carry (their glTF images), at the PS2's alpha.
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, OnceLock};

use glam::Vec3;
use minecraftoss_core::block::BlockStateId;
use minecraftoss_core::chunk::{Chunk, HeightmapKind};
use minecraftoss_world::chunk_map::WorldGen;

use crate::spargus_city_blocks::{
    BOGGY_TEXTURES, CityTexture, HAVEN_SLUMS_TEXTURES, SANDOVER_TEXTURES, SNOWY_TEXTURES, SPARGUS_TEXTURES,
};

/// Which of a column's floors a landmark stands on.
#[derive(Clone, Copy, Debug)]
pub enum Floor {
    /// The one nearest this height (metres of the level).
    Near(i32),
    /// The highest (a palace on its rock, not inside the rock).
    Top,
    /// The lowest (an arena's floor, under its stands).
    Bottom,
}

/// Where a place goes.
#[derive(Clone, Copy, Debug)]
pub enum Site {
    /// The Wasteland nearest the origin (`wasteland::nearest`).
    Wasteland,
    /// The nearest column well inside these biomes.
    Biomes(&'static [&'static str]),
}

/// One of the games' places.
pub struct Place {
    /// Its name for `mc lugar` and the log.
    pub name: &'static str,
    /// Its export (`jak1`, `jak2`, `jak3`).
    game: &'static str,
    /// Its levels: folder and file.
    levels: &'static [(&'static str, &'static str)],
    /// The part of them that is the place (metres: x, z, then y): the
    /// distant scenery left out.
    pub(crate) crop: [f32; 6],
    /// The crop's middle (x, z), put at its site.
    middle: (i32, i32),
    /// The level's height 0 in the block world.
    y_off: i32,
    /// Its sea's height, metres of the level.
    ocean: Option<i32>,
    textures: &'static [CityTexture],
    site: Site,
    /// Its places to go to: name, column (x, z) and which floor.
    landmarks: &'static [(&'static str, i32, i32, Floor)],
}

pub static PLACES: [Place; 5] = [
    Place {
        name: "spargus",
        game: "jak3",
        levels: &[
            ("wascitya", "wascitya-background.glb"),
            ("wascityb", "wascityb-background.glb"),
            ("waspala", "waspala-background.glb"),
            ("wasdoors", "wasdoors-background.glb"),
            ("wasstada", "wasstada-background.glb"),
        ],
        crop: [1370.0, 2640.0, -800.0, 260.0, -40.0, 320.0],
        middle: (2005, -270),
        // Jak 3's sea at 8 (the palace's top, 310 m over it, under the
        // world's ceiling).
        y_off: 8,
        // `*ocean-map-wascity*`'s start corner, 36864 units.
        ocean: Some(9),
        textures: &SPARGUS_TEXTURES,
        site: Site::Wasteland,
        landmarks: &[
            ("ciudad", 2320, -150, Floor::Near(30)),
            ("playa", 1700, -240, Floor::Near(30)),
            ("palacio", 2008, -460, Floor::Top),
            // Its stands (its sand floor has no texture of its own in the
            // level, so is not made).
            ("arena", 2330, -480, Floor::Near(40)),
        ],
    },
    Place {
        name: "haven",
        game: "jak2",
        levels: &[
            ("ctysluma", "ctysluma-background.glb"),
            ("ctyslumb", "ctyslumb-background.glb"),
            ("ctyslumc", "ctyslumc-background.glb"),
        ],
        crop: [340.0, 1220.0, -800.0, 420.0, -30.0, 120.0],
        middle: (780, -190),
        // Its sea's top on Minecraft's (water up to 62, as the sea level
        // 63 fills it; 23/10: a block under it, its edge showed).
        y_off: 63,
        // `*ocean-map-city*`'s start corner: height 0.
        ocean: Some(0),
        textures: &HAVEN_SLUMS_TEXTURES,
        site: Site::Biomes(&["minecraft:plains", "minecraft:sunflower_plains"]),
        landmarks: &[
            ("barrio", 967, 158, Floor::Near(10)),
            ("barrio-b", 580, -195, Floor::Near(10)),
            ("barrio-c", 600, -520, Floor::Near(10)),
        ],
    },
    Place {
        name: "villarrefugio",
        game: "jak1",
        levels: &[("village1", "village1-background.glb")],
        crop: [-260.0, 300.0, -340.0, 300.0, -40.0, 160.0],
        middle: (20, -20),
        y_off: 63,
        // `*ocean-map-village1*`'s start corner: height 0.
        ocean: Some(0),
        textures: &SANDOVER_TEXTURES,
        site: Site::Biomes(&["minecraft:beach"]),
        landmarks: &[("pueblo", 10, 20, Floor::Top), ("plaza", 44, -60, Floor::Near(22)), ("playa", 100, 68, Floor::Near(14))],
    },
    Place {
        name: "montana",
        game: "jak1",
        levels: &[("snow", "snow-background.glb")],
        crop: [355.0, 1115.0, -3665.0, -3085.0, 60.0, 385.0],
        middle: (735, -3375),
        // Its 70 m foot to 385 m peaks between the world's floor and ceiling.
        y_off: -66,
        ocean: None,
        textures: &SNOWY_TEXTURES,
        site: Site::Biomes(crate::snowy::LAND.biomes),
        landmarks: &[("cumbre", 735, -3375, Floor::Top)],
    },
    Place {
        name: "pantano",
        game: "jak1",
        levels: &[("swamp", "swamp-background.glb")],
        crop: [175.0, 775.0, -2245.0, -1755.0, -40.0, 110.0],
        middle: (475, -2000),
        y_off: 63,
        // Its water (`swp-water`) lies at 0.
        ocean: Some(0),
        textures: &BOGGY_TEXTURES,
        site: Site::Biomes(crate::swamp::LAND.biomes),
        landmarks: &[("cabana", 475, -2000, Floor::Near(5))],
    },
];

pub(crate) fn levels_root(game: &str) -> Option<PathBuf> {
    let env: &[&str] = match game {
        "jak3" => &["IW4L_JAK3_ASSETS", "IW4L_JAK_ASSETS"],
        "jak2" => &["IW4L_JAK2_ASSETS"],
        _ => &["IW4L_JAK1_ASSETS"],
    };
    env.iter()
        .filter_map(std::env::var_os)
        .map(PathBuf::from)
        .chain(std::env::var_os("USERPROFILE").map(|home| {
            PathBuf::from(home).join(format!(r"Documents\Jak OpenGOAL\active\{game}\data\decompiler_out\{game}"))
        }))
        .map(|root| root.join("levels"))
        .find(|levels| levels.is_dir())
}

fn index_of(place: &Place) -> usize {
    PLACES.iter().position(|p| std::ptr::eq(p, place)).unwrap_or(0)
}

/// A place's triangles in its crop with what the level draws them with:
/// texture coordinates, vertex colours, and whether each texture repeats
/// (its glTF sampler), for `jak_pieces`. None without its levels.
pub(crate) fn read_place_detailed(place: &Place) -> Option<Vec<crate::jak_pieces::SourceTri>> {
    let root = levels_root(place.game)?;
    let by_name: HashMap<&str, u16> = place.textures.iter().enumerate().map(|(i, t)| (t.texture, i as u16)).collect();
    let mut out = Vec::new();
    for (folder, file) in place.levels {
        if let Err(error) = read_level_detailed(&root.join(folder).join(file), &by_name, place.crop, &mut out) {
            diag::warn!(World, "{}: {file} not read ({error})", place.name);
        }
    }
    Some(out)
}

/// Small original street actors, read with the owning city's texture indices.
/// Normalize only referenced triangles: actor exports also contain unused
/// vertices from other models, whose bounds can be hundreds of metres wide.
pub(crate) fn read_street_actor(block: &str) -> Option<Vec<crate::jak_pieces::SourceTri>> {
    let (folder, file) = street_actor(block)?;
    let place=named(if block.starts_with("objeto_spargus_"){"spargus"}else{"haven"})?;
    let root=levels_root(place.game)?;
    let textures=textures_of(place);
    let names=textures.iter().enumerate().map(|(i,&t)|(t,i as u16)).collect();
    let mut triangles = Vec::new();
    read_level_detailed(&root.join(folder).join(file), &names,
        [-10000.0, 10000.0, -10000.0, 10000.0, -10000.0, 10000.0], &mut triangles).ok()?;
    if block.ends_with("_bar") {
        let normalize=|tris:&mut Vec<crate::jak_pieces::SourceTri>,height:f32,center:Vec3|->Option<()> {
            let lo=tris.iter().flat_map(|t|t.p).reduce(Vec3::min)?;
            let hi=tris.iter().flat_map(|t|t.p).reduce(Vec3::max)?;
            let scale=height/(hi.y-lo.y).max(0.001);
            let offset=center-Vec3::new((hi.x-lo.x)*scale*0.5,0.,(hi.z-lo.z)*scale*0.5);
            for t in tris{t.p=t.p.map(|p|(p-lo)*scale+offset);}Some(())
        };
        if block=="objeto_haven_bandeja_bar" {
            normalize(&mut triangles,0.48,Vec3::new(0.32,0.06,0.5))?;
            let mut mug=Vec::new();read_level_detailed(&root.join("lhipout/hip-mug-lod0.glb"),&names,[-10000.,10000.,-10000.,10000.,-10000.,10000.],&mut mug).ok()?;
            normalize(&mut mug,0.25,Vec3::new(0.67,0.06,0.5))?;triangles.extend(mug);
            crate::jak_bar::tray(&mut triangles,*names.get("hip-tmetring01")?);
        }else{normalize(&mut triangles,if block=="objeto_haven_jarra_bar"{0.25}else{0.48},Vec3::new(0.5,0.,0.5))?;}
        return Some(triangles);
    }
    let corner = triangles.iter().flat_map(|t| t.p).reduce(Vec3::min)?;
    for triangle in &mut triangles {
        triangle.p = triangle.p.map(|p| (p - corner) * 0.75);
    }
    Some(triangles)
}

pub(crate) fn street_actor(block: &str) -> Option<(&'static str, &'static str)> {
    match block {
        "objeto_haven_botella_bar" | "objeto_haven_bandeja_bar" => Some(("lhipout","hip-bottle-c-lod0.glb")),
        "objeto_haven_jarra_bar" => Some(("lhipout","hip-mug-lod0.glb")),
        "objeto_haven_puesto" => Some(("ctysluma", "cty-fruit-stand-lod0.glb")),
        "objeto_spargus_urna_alta" => Some(("wascitya","urn-a-lod0.glb")),
        "objeto_spargus_urna_baja" => Some(("wascitya","urn-c-lod0.glb")),
        "objeto_spargus_caja_mercado" => Some(("wascitya","market-crate-lod0.glb")),
        "objeto_haven_propaganda" | "objeto_haven_propaganda_ajustada" | "objeto_haven_propaganda_bloque" => Some(("ctywide", "propa-lod0.glb")),
        _ => None,
    }
}

pub(crate) fn read_level_detailed(
    path: &Path,
    by_name: &HashMap<&str, u16>,
    crop: [f32; 6],
    out: &mut Vec<crate::jak_pieces::SourceTri>,
) -> anyhow::Result<()> {
    let bytes = std::fs::read(path)?;
    let glb = Glb::parse(&bytes)?;
    let json = &glb.json;
    // Each material's texture (one of the place's) and its wrapping.
    let materials: Vec<Option<(u16, [bool; 2], bool)>> = json["materials"]
        .as_array()
        .map(|all| {
            all.iter()
                .map(|m| {
                    let texture = by_name.get(m["name"].as_str()?.trim_end_matches("-envmap")).copied()?;
                    let sampler = m["pbrMetallicRoughness"]["baseColorTexture"]["index"]
                        .as_u64()
                        .and_then(|t| json["textures"][t as usize]["sampler"].as_u64())
                        .map(|s| &json["samplers"][s as usize]);
                    // glTF's default is to repeat (10497); 33071 clamps.
                    let repeats = |key: &str| sampler.and_then(|s| s[key].as_u64()).is_none_or(|w| w != 33071);
                    Some((texture, [repeats("wrapS"), repeats("wrapT")], m["alphaMode"].as_str() == Some("BLEND")))
                })
                .collect()
        })
        .unwrap_or_default();
    for (mesh_index, mesh) in json["meshes"].as_array().into_iter().flatten().enumerate() {
        for primitive in mesh["primitives"].as_array().into_iter().flatten() {
            let Some((texture, repeat, blend)) = primitive["material"].as_u64().and_then(|m| materials.get(m as usize).copied().flatten()) else { continue };
            let attributes = &primitive["attributes"];
            let (Some(p), Some(i), Some(t)) = (attributes["POSITION"].as_u64(), primitive["indices"].as_u64(), attributes["TEXCOORD_0"].as_u64()) else {
                continue;
            };
            let mut points = glb.vec3s(p as usize)?;
            let indices = glb.indices(i as usize)?;
            if path.file_name().and_then(|n|n.to_str()).is_some_and(|n|n.starts_with("hip-")) {
                glb.pose_decor(mesh_index, attributes, &indices, &mut points)?;
            }
            let uvs = glb.floats::<2>(t as usize)?;
            let colors = match attributes["COLOR_0"].as_u64() {
                Some(c) => glb.floats::<4>(c as usize)?,
                None => vec![[0.5; 4]; points.len()],
            };
            for tri in indices.chunks_exact(3) {
                let k = [tri[0] as usize, tri[1] as usize, tri[2] as usize];
                if k.iter().any(|&k| k >= points.len() || k >= uvs.len() || k >= colors.len()) {
                    continue;
                }
                let [a, b, c] = k.map(|k| Vec3::from_array(points[k]));
                let centre = (a + b + c) / 3.0;
                if centre.x < crop[0] || centre.x > crop[1] || centre.z < crop[2] || centre.z > crop[3] || centre.y < crop[4] || centre.y > crop[5] {
                    continue;
                }
                if (b - a).cross(c - a).length_squared() < 1e-10 {
                    continue;
                }
                out.push(crate::jak_pieces::SourceTri {
                    p: [a, b, c],
                    uv: k.map(|k| glam::Vec2::from_array(uvs[k])),
                    color: k.map(|k| colors[k].map(|c| (c * 255.0).round().clamp(0.0, 255.0) as u8)),
                    texture,
                    repeat,
                    blend,
                });
            }
        }
    }
    Ok(())
}

/// A glTF binary: its JSON and its buffer.
struct Glb<'a> {
    json: serde_json::Value,
    bin: &'a [u8],
}

impl<'a> Glb<'a> {
    fn parse(bytes: &'a [u8]) -> anyhow::Result<Self> {
        anyhow::ensure!(bytes.len() > 20 && &bytes[0..4] == b"glTF", "not a GLB");
        let json_len = u32::from_le_bytes(bytes[12..16].try_into()?) as usize;
        let json = serde_json::from_slice(&bytes[20..20 + json_len])?;
        let at = 20 + json_len;
        let bin_len = u32::from_le_bytes(bytes[at..at + 4].try_into()?) as usize;
        Ok(Self { json, bin: &bytes[at + 8..at + 8 + bin_len] })
    }

    /// An accessor's bytes, its count, the size of one, the stride and the
    /// component type.
    fn view(&self, accessor: usize) -> anyhow::Result<(&'a [u8], usize, usize, usize, u64)> {
        let a = &self.json["accessors"][accessor];
        let view = &self.json["bufferViews"][a["bufferView"].as_u64().unwrap_or(0) as usize];
        let offset = view["byteOffset"].as_u64().unwrap_or(0) as usize + a["byteOffset"].as_u64().unwrap_or(0) as usize;
        let count = a["count"].as_u64().unwrap_or(0) as usize;
        let component = a["componentType"].as_u64().unwrap_or(5126);
        let width = match component {
            5121 => 1,
            5123 => 2,
            _ => 4,
        };
        let parts = match a["type"].as_str() {
            Some("VEC3") => 3,
            Some("VEC2") => 2,
            Some("VEC4") => 4,
            Some("MAT4") => 16,
            _ => 1,
        };
        let size = width * parts;
        let stride = view["byteStride"].as_u64().map_or(size, |s| s as usize);
        anyhow::ensure!(offset + stride * count.saturating_sub(1) + size <= self.bin.len(), "accessor {accessor} past the buffer");
        Ok((&self.bin[offset..], count, size, stride, component))
    }

    /// Static furniture uses the first native idle pose. Actor exports keep joint
    /// vertices disassembled in bind space (notably the bottle's neck and base).
    /// Scope this to Hip Hog furniture; world/background extraction stays unchanged.
    fn pose_decor(&self, mesh:usize, attrs:&serde_json::Value, indices:&[u32], points:&mut [[f32;3]]) -> anyhow::Result<()> {
        use glam::{Mat4,Quat};
        let Some(nodes)=self.json["nodes"].as_array() else{return Ok(());};
        let Some(node)=nodes.iter().find(|n|n["mesh"].as_u64()==Some(mesh as u64)) else{return Ok(());};
        let Some(skin)=node["skin"].as_u64() else{return Ok(());};
        let skin=&self.json["skins"][skin as usize];
        let Some(joints)=skin["joints"].as_array() else{return Ok(());};
        let Some(ja)=attrs["JOINTS_0"].as_u64() else{return Ok(());};
        let Some(wa)=attrs["WEIGHTS_0"].as_u64() else{return Ok(());};
        let mut local:Vec<_>=nodes.iter().map(|n| {
            if let Some(m)=n["matrix"].as_array() { Mat4::from_cols_array(&std::array::from_fn(|i|m[i].as_f64().unwrap_or(0.) as f32)).to_scale_rotation_translation() }
            else {
                let vec=|key:&str,default:f32|Vec3::from_array(std::array::from_fn(|i|n[key][i].as_f64().map_or(default,|v|v as f32)));
                let q=Quat::from_array(std::array::from_fn(|i|n["rotation"][i].as_f64().map_or(if i==3{1.}else{0.},|v|v as f32)));
                (vec("scale",1.),q,vec("translation",0.))
            }
        }).collect();
        if let Some(animation)=self.json["animations"].as_array().and_then(|a|a.iter().find(|a|a["name"].as_str().is_some_and(|n|n.contains("idle"))).or_else(||a.first())) {
            for channel in animation["channels"].as_array().into_iter().flatten() {
                let Some(index)=channel["target"]["node"].as_u64() else{continue;};
                let Some(transform)=local.get_mut(index as usize) else{continue;};
                let sampler=&animation["samplers"][channel["sampler"].as_u64().unwrap_or(0) as usize];
                let Some(output)=sampler["output"].as_u64() else{continue;};
                // CUBICSPLINE stores in-tangent, value, out-tangent.
                let frame=usize::from(sampler["interpolation"].as_str()==Some("CUBICSPLINE"));
                match channel["target"]["path"].as_str() {
                    Some("translation")=>if let Some(v)=self.floats::<3>(output as usize)?.get(frame){transform.2=Vec3::from_array(*v);},
                    Some("scale")=>if let Some(v)=self.floats::<3>(output as usize)?.get(frame){transform.0=Vec3::from_array(*v);},
                    Some("rotation")=>if let Some(v)=self.floats::<4>(output as usize)?.get(frame){transform.1=Quat::from_array(*v).normalize();},
                    _=>{}
                }
            }
        }
        let local:Vec<_>=local.into_iter().map(|(s,r,t)|Mat4::from_scale_rotation_translation(s,r,t)).collect();
        let mut parents=vec![None;nodes.len()];
        for (i,n) in nodes.iter().enumerate(){for child in n["children"].as_array().into_iter().flatten(){if let Some(c)=child.as_u64().and_then(|c|parents.get_mut(c as usize)){*c=Some(i);}}}
        let globals:Vec<_>=(0..nodes.len()).map(|i|{let mut m=local[i];let mut p=parents[i];let mut remaining=nodes.len();while let Some(j)=p{if remaining==0{break;}m=local[j]*m;p=parents[j];remaining-=1;}m}).collect();
        let inverse=match skin["inverseBindMatrices"].as_u64(){Some(a)=>self.floats::<16>(a as usize)?,None=>vec![Mat4::IDENTITY.to_cols_array();joints.len()]};
        let matrices:Vec<_>=joints.iter().enumerate().map(|(i,j)|globals[j.as_u64().unwrap_or(0) as usize]*Mat4::from_cols_array(&inverse[i])).collect();
        let weights=self.floats::<4>(wa as usize)?;
        let (bytes,count,_,stride,component)=self.view(ja as usize)?;
        anyhow::ensure!(matches!(component,5121|5123),"unsupported furniture joints");
        let width=if component==5121{1}else{2};
        let used:std::collections::HashSet<_>=indices.iter().copied().collect();
        for (i,p) in points.iter_mut().enumerate(){
            if !used.contains(&(i as u32)){continue;}
            anyhow::ensure!(i<count && i<weights.len(),"missing furniture vertex weights");
            let original=Vec3::from_array(*p);let mut posed=Vec3::ZERO;let mut total=0.;
            for c in 0..4{let at=i*stride+c*width;let joint=if width==1{bytes[at] as usize}else{u16::from_le_bytes([bytes[at],bytes[at+1]]) as usize};let weight=weights[i][c];if weight>0.{let m=matrices.get(joint).ok_or_else(||anyhow::anyhow!("invalid furniture joint"))?;posed+=m.transform_point3(original)*weight;total+=weight;}}
            if total>0.{*p=(posed/total).to_array();}
        }
        Ok(())
    }

    fn vec3s(&self, accessor: usize) -> anyhow::Result<Vec<[f32; 3]>> {
        let (bytes, count, _, stride, _) = self.view(accessor)?;
        Ok((0..count)
            .map(|k| {
                let at = k * stride;
                std::array::from_fn(|c| f32::from_le_bytes(bytes[at + c * 4..at + c * 4 + 4].try_into().unwrap()))
            })
            .collect())
    }

    /// An accessor of `N` floats each.
    fn floats<const N: usize>(&self, accessor: usize) -> anyhow::Result<Vec<[f32; N]>> {
        let (bytes, count, size, stride, component) = self.view(accessor)?;
        anyhow::ensure!(component == 5126 && size == 4 * N, "accessor {accessor} is not {N} floats");
        Ok((0..count)
            .map(|k| {
                let at = k * stride;
                std::array::from_fn(|c| f32::from_le_bytes(bytes[at + c * 4..at + c * 4 + 4].try_into().unwrap()))
            })
            .collect())
    }

    fn indices(&self, accessor: usize) -> anyhow::Result<Vec<u32>> {
        let (bytes, count, _, stride, component) = self.view(accessor)?;
        Ok((0..count)
            .map(|k| {
                let at = k * stride;
                match component {
                    5121 => u32::from(bytes[at]),
                    5123 => u32::from(u16::from_le_bytes([bytes[at], bytes[at + 1]])),
                    _ => u32::from_le_bytes(bytes[at..at + 4].try_into().unwrap()),
                }
            })
            .collect())
    }
}

/// A texture one of the places' levels carries (their glTF images, PNG data
/// URIs), by its name: width, height and RGBA at the PS2's alpha (128
/// opaque). The images' text is gathered once, a texture decoded when asked.
pub fn embedded_texture(name: &str) -> Option<(u32, u32, Vec<u8>)> {
    static IMAGES: OnceLock<HashMap<String, String>> = OnceLock::new();
    let images = IMAGES.get_or_init(|| {
        let mut out = HashMap::new();
        let precinct:&[(&str,&str)]=&[("prison","prison-background.glb"),("forexita","forexita-background.glb")];
        let street_actors: &[(&str, &str)] = &[("ctysluma", "cty-fruit-stand-lod0.glb"), ("ctywide", "propa-lod0.glb"), ("ctywide", "ctywide-background.glb")];
        let spargus_props:&[(&str,&str)]=&[("wascitya","urn-a-lod0.glb"),("wascitya","urn-c-lod0.glb"),("wascitya","market-crate-lod0.glb")];
        for (game,levels) in PLACES.iter().map(|p|(p.game,p.levels))
            .chain([("jak3",spargus_props), ("jak2",precinct), ("jak2",crate::jak_bar::LEVELS), ("jak2",crate::jak_decor::LEVELS), ("jak2",street_actors), ("jak2",crate::jak_skate_tower::SKATE_LEVELS)]) {
            let Some(root) = levels_root(game) else { continue };
            for (folder, file) in levels {
                let Ok(bytes) = std::fs::read(root.join(folder).join(file)) else { continue };
                let Ok(glb) = Glb::parse(&bytes) else { continue };
                for image in glb.json["images"].as_array().into_iter().flatten() {
                    if let (Some(name), Some(uri)) = (image["name"].as_str(), image["uri"].as_str()) {
                        if let Some((_, data)) = uri.split_once("base64,") {
                            out.entry(name.to_owned()).or_insert_with(|| data.to_owned());
                        }
                    }
                }
            }
        }
        out
    });
    let png = base64_decode(images.get(name)?)?;
    let mut image = image::load_from_memory(&png).ok()?.to_rgba8();
    // Hip Hog glass is MASK with native cutoff 38/255, not a translucent
    // surface. Preserve that coverage in Minecraft's stricter cutout pass.
    if name=="hip-tbotblue02" {for pixel in image.pixels_mut(){pixel.0[3]=if pixel.0[3]>=38{128}else{0};}}
    Some((image.width(), image.height(), image.into_raw()))
}

/// Standard base64 (the images' data URIs).
fn base64_decode(text: &str) -> Option<Vec<u8>> {
    let value = |c: u8| -> Option<u32> {
        Some(match c {
            b'A'..=b'Z' => c - b'A',
            b'a'..=b'z' => c - b'a' + 26,
            b'0'..=b'9' => c - b'0' + 52,
            b'+' => 62,
            b'/' => 63,
            _ => return None,
        } as u32)
    };
    let bytes: Vec<u8> = text.bytes().filter(|&c| c != b'=' && !c.is_ascii_whitespace()).collect();
    let mut out = Vec::with_capacity(bytes.len() * 3 / 4);
    for chunk in bytes.chunks(4) {
        let mut n = 0u32;
        for (i, &c) in chunk.iter().enumerate() {
            n |= value(c)? << (18 - 6 * i);
        }
        out.push((n >> 16) as u8);
        if chunk.len() > 2 {
            out.push((n >> 8) as u8);
        }
        if chunk.len() > 3 {
            out.push(n as u8);
        }
    }
    Some(out)
}

/// Every place's site (x, z of its middle) for a world: found once a seed,
/// in order, each the nearest land of its kind to the origin clear of the
/// places before it.
pub fn sites(registries: Arc<minecraftoss_core::Registries>, seed: i64) -> [Option<(i32, i32)>; 5] {
    // One search a seed, whoever asks first (the server's chunks, the far
    // terrain, tests with seeds of their own at once); the others wait.
    let cell = {
        let mut cache = sites_cache().lock().unwrap_or_else(|e| e.into_inner());
        cache.last = Some(seed);
        cache.registries = Some(registries.clone());
        cache.by_seed.entry(seed).or_default().clone()
    };
    *cell.get_or_init(|| find_sites(registries, seed))
}

fn find_sites(registries: Arc<minecraftoss_core::Registries>, seed: i64) -> [Option<(i32, i32)>; 5] {
    let mut found: [Option<(i32, i32)>; 5] = [None; 5];
    let terrain = minecraftoss_generator::terrain::TerrainGenerator::overworld(registries.clone(), seed).ok();
    for (k, place) in PLACES.iter().enumerate() {
        let half = |p: &Place| (((p.crop[1] - p.crop[0]) / 2.0) as i32 + 32, ((p.crop[3] - p.crop[2]) / 2.0) as i32 + 32);
        let (hx, hz) = half(place);
        let clear = |x: i32, z: i32| {
            PLACES.iter().zip(found.iter()).take(k).all(|(other, site)| {
                site.is_none_or(|(ox, oz)| {
                    let (ux, uz) = half(other);
                    (x - ox).abs() > hx + ux || (z - oz).abs() > hz + uz
                })
            })
        };
        found[k] = match place.site {
            // (Spargus is a small city of blocks now, `spargus_town`; its
            // whole level only when asked for.)
            Site::Wasteland if !crate::spargus_town::level_wanted() => None,
            Site::Wasteland => crate::wasteland::nearest(registries.clone(), seed, (0, 0), 8000)
                .map(|(x, _, z)| (x, z))
                .filter(|&(x, z)| clear(x, z)),
            Site::Biomes(biomes) => terrain.as_ref().and_then(|terrain| {
                let ids: Vec<_> = biomes.iter().filter_map(|b| registries.biomes.id(b)).collect();
                let inside = |x: i32, z: i32| {
                    let y = terrain.base_height(x, z, HeightmapKind::WorldSurfaceWg);
                    ids.contains(&terrain.biome_at_quart(x >> 2, y >> 2, z >> 2))
                };
                (0..=8000 / 64).find_map(|ring| {
                    let r = ring * 64;
                    (-ring..=ring).find_map(|i| {
                        [(i * 64, -r), (i * 64, r), (-r, i * 64), (r, i * 64)].into_iter().find(|&(x, z)| {
                            clear(x, z)
                                && inside(x, z)
                                && [(48, 0), (-48, 0), (0, 48), (0, -48)].iter().all(|&(a, b)| inside(x + a, z + b))
                        })
                    })
                })
            }),
        };
    }
    found
}

/// The sites of the world last asked for (`sites`): where the places stand
/// in the world being played.
pub fn current_sites() -> Option<[Option<(i32, i32)>; 5]> {
    let cache = sites_cache().lock().unwrap_or_else(|e| e.into_inner());
    cache.by_seed.get(&cache.last?)?.get().copied()
}

/// The seed and registries of the world last asked for (`sites`): the
/// world being played.
pub fn current_world() -> Option<(i64, Arc<minecraftoss_core::Registries>)> {
    let cache = sites_cache().lock().unwrap_or_else(|e| e.into_inner());
    Some((cache.last?, cache.registries.clone()?))
}

/// Bind native pieces and decorative objects to the active world even
/// when whole imported levels are disabled. Finding those optional sites
/// is deliberately separate from opening an ordinary generated world.
pub fn bind_world(registries: Arc<minecraftoss_core::Registries>, seed: i64) {
    let mut cache = sites_cache().lock().unwrap_or_else(|e| e.into_inner());
    cache.last = Some(seed);
    cache.registries = Some(registries);
}

/// A block of the world as a metre of the place's levels, with the place
/// at `site` (`to_world` the other way).
pub fn to_level(place: &Place, (sx, sz): (i32, i32), (x, y, z): (i32, i32, i32)) -> (i32, i32, i32) {
    (x - sx + place.middle.0, y - place.y_off, z - sz + place.middle.1)
}

/// A metre of the place's levels as a block of the world, with the place
/// at `site` (`to_level` the other way).
pub fn to_world_block(place: &Place, (sx, sz): (i32, i32), (x, y, z): (i32, i32, i32)) -> (i32, i32, i32) {
    (x - place.middle.0 + sx, y + place.y_off, z - place.middle.1 + sz)
}

/// The sites found, by seed, and the seed last asked for.
#[derive(Default)]
struct SitesCache {
    last: Option<i64>,
    registries: Option<Arc<minecraftoss_core::Registries>>,
    by_seed: HashMap<i64, Arc<OnceLock<[Option<(i32, i32)>; 5]>>>,
}

fn sites_cache() -> &'static Mutex<SitesCache> {
    static SITES: OnceLock<Mutex<SitesCache>> = OnceLock::new();
    SITES.get_or_init(Default::default)
}

/// Whether the games' places, their pieces' hamlets, Spargus's town and the
/// lands' flats are put in the world: only with `IW4L_LUGARES_JAK=1`
/// (Adrii, 23/10: "vuelve a los desiertos etc como estaban"; the lands are
/// as they were, their villages built of facade blocks, `jak_facades`).
pub fn jak_places_on() -> bool {
    static ON: OnceLock<bool> = OnceLock::new();
    *ON.get_or_init(|| std::env::var_os("IW4L_LUGARES_JAK").is_some_and(|v| v == "1"))
}

/// A place's position as a block of the world, with the place at `site`.
pub fn to_world(place: &Place, (sx, sz): (i32, i32), at: Vec3) -> (i32, i32, i32) {
    (at.x.floor() as i32 - place.middle.0 + sx, at.y.floor() as i32 + place.y_off, at.z.floor() as i32 - place.middle.1 + sz)
}

/// What the far terrain shows of a column (`far_terrain`): a place's top
/// there, or its sea.
pub enum FarPlace {
    /// Its highest triangle: the world height of its top, which of the
    /// place's textures it wears (`textures_of`) and its vertex colour.
    Top { place: usize, y: f32, texture: u16, color: [u8; 3] },
    /// Inside its crop with nothing of it: its sea, at this world height.
    Sea(f32),
}

/// A place's tops by column over its crop (x fastest): the height (level
/// metres), texture and colour of its highest triangle; `None` where it
/// has none.
type FarMap = Vec<Option<(f32, u16, [u8; 3])>>;

/// Whether world column `(x, z)` of the world of `seed` is inside one of
/// the places' crops (where its land is the place's, not the land's own).
pub fn inside_a_place(registries: &Arc<minecraftoss_core::Registries>, seed: i64, x: i32, z: i32) -> bool {
    if crate::spargus_town::inside(registries, seed, x, z) {
        return true;
    }
    let sites = sites(registries.clone(), seed);
    PLACES.iter().zip(sites).any(|(place, site)| {
        site.is_some_and(|site| {
            let (jx, _, jz) = to_level(place, site, (x, 0, z));
            (place.crop[0]..=place.crop[1]).contains(&(jx as f32)) && (place.crop[2]..=place.crop[3]).contains(&(jz as f32))
        })
    })
}

/// Whether world column `(x, z)` of the world being played is inside one
/// of the places' crops (the drawing and the collision, which ask for
/// every block: no search).
pub fn inside_current_place(x: i32, z: i32) -> bool {
    let Some(sites) = current_sites() else { return false };
    PLACES.iter().zip(sites).any(|(place, site)| {
        site.is_some_and(|site| {
            let (jx, _, jz) = to_level(place, site, (x, 0, z));
            (place.crop[0]..=place.crop[1]).contains(&(jx as f32)) && (place.crop[2]..=place.crop[3]).contains(&(jz as f32))
        })
    })
}

/// How high the clouds lie over world column `(x, z)` of the world being
/// played: Minecraft's 192, lifted over a place's top near by (23/10: they
/// cut through Haven's towers), easing in over 192 blocks round its crop.
pub fn cloud_floor(x: i32, z: i32) -> f32 {
    const BASE: f32 = 192.33;
    if !jak_places_on() {
        return BASE;
    }
    let Some(sites) = current_sites() else { return BASE };
    let mut floor = BASE;
    for (place, site) in PLACES.iter().zip(sites) {
        let Some(site) = site else { continue };
        let (jx, _, jz) = to_level(place, site, (x, 0, z));
        let out = (place.crop[0] - jx as f32).max(jx as f32 - place.crop[1]).max(0.0).max((place.crop[2] - jz as f32).max(jz as f32 - place.crop[3]).max(0.0));
        let weight = (1.0 - out / 192.0).clamp(0.0, 1.0);
        let needed = place.crop[5] + place.y_off as f32 + 24.0;
        floor = floor.max(BASE + (needed - BASE).max(0.0) * weight);
    }
    floor
}

/// The place standing at world column `(x, z)` of the world of `seed`, as
/// the far terrain shows it.
pub fn far_place(registries: &Arc<minecraftoss_core::Registries>, seed: i64, x: i32, z: i32) -> Option<FarPlace> {
    let sites = sites(registries.clone(), seed);
    for (k, place) in PLACES.iter().enumerate() {
        let Some(site) = sites[k] else { continue };
        let (jx, _, jz) = to_level(place, site, (x, 0, z));
        let [x0, x1, z0, z1, ..] = place.crop.map(|c| c as i32);
        if jx < x0 || jx >= x1 || jz < z0 || jz >= z1 {
            continue;
        }
        let Some(map) = far_map(k) else { continue };
        let width = (x1 - x0) as usize;
        if let Some(Some((y, texture, color))) = map.get((jz - z0) as usize * width + (jx - x0) as usize) {
            return Some(FarPlace::Top { place: k, y: y + place.y_off as f32, texture: *texture, color: *color });
        }
        if let Some(ocean) = place.ocean {
            return Some(FarPlace::Sea((ocean + place.y_off) as f32));
        }
    }
    None
}

/// A place's tops (`FarMap`), made once from its levels read afresh (the
/// far terrain needs them for places nobody is near, whose triangles are
/// not kept).
fn far_map(k: usize) -> Option<&'static FarMap> {
    static MAPS: [OnceLock<Option<FarMap>>; 5] = [const { OnceLock::new() }; 5];
    MAPS[k]
        .get_or_init(|| {
            let place = &PLACES[k];
            let tris = read_place_detailed(place)?;
            let [x0, x1, z0, z1, ..] = place.crop.map(|c| c as i32);
            let width = (x1 - x0) as usize;
            let mut map: FarMap = vec![None; width * (z1 - z0) as usize];
            for t in &tris {
                // Points over the triangle a metre apart, its corners too.
                let [a, b, c] = t.p;
                let longest = (b - a).length().max((c - b).length()).max((a - c).length());
                let n = (longest.ceil() as usize).clamp(1, 200);
                let color = [0, 1, 2].map(|i| ((u16::from(t.color[0][i]) + u16::from(t.color[1][i]) + u16::from(t.color[2][i])) / 3) as u8);
                for i in 0..=n {
                    for j in 0..=n - i {
                        let p = a + (b - a) * (i as f32 / n as f32) + (c - a) * (j as f32 / n as f32);
                        let (px, pz) = (p.x.floor() as i32, p.z.floor() as i32);
                        if px < x0 || px >= x1 || pz < z0 || pz >= z1 {
                            continue;
                        }
                        let cell = &mut map[(pz - z0) as usize * width + (px - x0) as usize];
                        if cell.is_none_or(|(y, ..)| p.y > y) {
                            *cell = Some((p.y, t.texture, color));
                        }
                    }
                }
            }
            Some(map)
        })
        .as_ref()
}

/// The names of a place's textures, by their index.
pub fn textures_of(place: &Place) -> Vec<&'static str> {
    let mut names: Vec<_> = place.textures.iter().map(|t| t.texture).collect();
    // Source 7 uses this extended bank; all old Haven indices stay exactly
    // where they were, including the legacy and expanded palace pieces.
    if place.name == "haven" {
        for &name in crate::jak_skate_tower::SKATE_TEXTURES.iter().chain(crate::jak_bar::TEXTURES).chain(crate::jak_decor::TEXTURES) {
            if !names.contains(&name) { names.push(name); }
        }
    }
    if place.name=="spargus" {
        for name in ["common_sandstone_taper01","common_sandstone_ground01","common_sandstone_trim01","common_sandstone_base01","city-mark-wood-plain"] {
            if !names.contains(&name){names.push(name);}
        }
    }
    names
}

/// A place by its name (`mc lugar`).
pub fn named(name: &str) -> Option<&'static Place> {
    PLACES.iter().find(|p| p.name == name)
}

/// A landmark's place to stand (metres of the level): over the floor in
/// its column (else the nearest column round it with one, 40 m at most) as
/// it says, with two metres free over it. Its first one if none is named.
pub fn landmark(place: &Place, name: Option<&str>) -> Option<Vec3> {
    let &(_, x0, z0, which) = match name {
        Some(name) => place.landmarks.iter().find(|l| l.0 == name)?,
        None => place.landmarks.first()?,
    };
    let k = index_of(place);
    if crate::jak_pieces::source(k).is_none() {
        return Some(Vec3::new(x0 as f32, 30.0, z0 as f32));
    }
    for ring in 0..=40i32 {
        for dx in -ring..=ring {
            for dz in -ring..=ring {
                if dx.abs().max(dz.abs()) != ring {
                    continue;
                }
                let (x, z) = (x0 + dx, z0 + dz);
                let column = crate::jak_pieces::column(k, (x.div_euclid(16) * 16, z.div_euclid(16) * 16))?;
                let solid = |y: i32| column.get(&(x, y, z)).is_some_and(|p| !p.tris.is_empty());
                let floors = column
                    .iter()
                    .filter(|(&(px, py, pz), piece)| {
                        px == x && pz == z && piece.floor.is_some() && !solid(py + 1) && !solid(py + 2) && place.ocean.is_none_or(|o| py >= o)
                    })
                    .map(|(&(_, py, _), piece)| (py, piece.floor.unwrap_or(1.0)));
                let floor = match which {
                    Floor::Near(near) => floors.min_by_key(|&(y, _)| (y - near).abs()),
                    Floor::Top => floors.max_by_key(|&(y, _)| y),
                    Floor::Bottom => floors.min_by_key(|&(y, _)| y),
                };
                if let Some((y, top)) = floor {
                    return Some(Vec3::new(x as f32 + 0.5, y as f32 + top + 0.05, z as f32 + 0.5));
                }
            }
        }
    }
    None
}

/// Every place's blocks over one chunk (none where none reaches).
pub fn place(chunk: &mut Chunk, gen: &WorldGen, states: &mut HashMap<&'static str, BlockStateId>) {
    if !gen.sky_light {
        return;
    }
    let sites = sites(gen.terrain.registries.clone(), gen.terrain.seed);
    for (place_def, site) in PLACES.iter().zip(sites) {
        if let Some(site) = site {
            place_one(place_def, site, chunk, gen, states);
        }
    }
}

fn place_one(place: &Place, site: (i32, i32), chunk: &mut Chunk, gen: &WorldGen, states: &mut HashMap<&'static str, BlockStateId>) {
    let k = index_of(place);
    let (jx0, jz0) = crate::jak_pieces::chunk_corner(k, site, (chunk.pos.x, chunk.pos.z));
    if ((jx0 + 16) as f32) < place.crop[0] || (jx0 as f32) > place.crop[1] || ((jz0 + 16) as f32) < place.crop[2] || (jz0 as f32) > place.crop[3] {
        return;
    }
    let Some(column) = crate::jak_pieces::column(k, (jx0, jz0)) else { return };
    // Each metre with enough of the level in it: whether it is low (all
    // under its middle: a bottom slab, else a top one; never a whole block,
    // which the game would smother Jak inside).
    let mut pieces: HashMap<(i32, i32, i32), bool> = HashMap::new();
    let mut floored = std::collections::HashSet::new();
    for (&at, piece) in column.iter() {
        let mut total = 0.0f32;
        let mut high = 0.0f32;
        for t in &piece.tris {
            let [a, b, c] = t.p.map(Vec3::from_array);
            total += (b - a).cross(c - a).length() * 0.5;
            high = high.max(a.y).max(b.y).max(c.y);
        }
        if total < 0.002 {
            continue;
        }
        pieces.insert(at, high <= 0.5);
        if piece.floor.is_some() {
            floored.insert(at);
        }
    }
    if pieces.is_empty() && place.ocean.is_none() {
        return;
    }
    let registries = gen.terrain.registries.clone();
    let blocks = &registries.blocks;
    let state = |name: &str, states: &mut HashMap<&'static str, BlockStateId>| -> BlockStateId {
        if let Some(&id) = states.get(name) {
            return id;
        }
        let id = blocks.parse_state(name).unwrap_or(BlockStateId::AIR);
        states.insert(Box::leak(name.to_owned().into_boxed_str()), id);
        id
    };
    let y_off = place.y_off;
    let air = BlockStateId::AIR;
    let top = 319;
    let water = state("minecraft:water[level=0]", states);
    let fill = state(&format!("{}[type=double,waterlogged=false]", crate::jak_pieces::block_of(place)), states);
    // Per column of the chunk: the place's lowest piece there.
    let mut lowest: HashMap<(i32, i32), i32> = HashMap::new();
    for &(x, y, z) in pieces.keys() {
        let e = lowest.entry((x, z)).or_insert(y);
        *e = (*e).min(y);
    }
    // The place's ground round here: its lowest piece in this chunk and the
    // eight round it. No Minecraft land stands over it anywhere in the
    // chunk (23/10: under Sandover's cliff tops, palms and docks, which
    // have nothing of the level under them, its beach showed).
    let ground = (-1..=1)
        .flat_map(|dx| (-1..=1).map(move |dz| (dx, dz)))
        .filter_map(|(dx, dz)| {
            let corner = crate::jak_pieces::chunk_corner(k, site, (chunk.pos.x + dx, chunk.pos.z + dz));
            crate::jak_pieces::column(k, corner)?.keys().map(|c| c.1).min()
        })
        .min();
    let sea = place.ocean.map(|o| o + y_off);
    let enclosed = crate::jak_pieces::enclosed(k);
    let inside = |jx: i32, jz: i32| {
        (jx as f32) >= place.crop[0] && (jx as f32) <= place.crop[1] && (jz as f32) >= place.crop[2] && (jz as f32) <= place.crop[3]
    };
    for lx in 0..16 {
        for lz in 0..16 {
            let (jx, jz) = (jx0 + lx as i32, jz0 + lz as i32);
            let low = lowest.get(&(jx, jz)).copied();
            if low.is_none() && !inside(jx, jz) {
                continue;
            }
            // The land cleared down to the place's ground and to its sea
            // (inside a cliff too: none of it is Minecraft's), and its sea
            // put back under the sea's height.
            let from = match (ground.map(|g| g + y_off), sea) {
                (Some(g), Some(s)) => g.min(s),
                (g, s) => match g.or(s) {
                    Some(from) => from,
                    None => continue,
                },
            };
            let from = from.max(-63);
            for y in from..=top {
                chunk.set_block_raw(lx, y, lz, air, &registries);
            }
            if let Some(sea) = sea {
                let mut y = sea - 1;
                while y > sea - 40 && y > -60 && chunk.block(lx, y, lz) == air {
                    chunk.set_block_raw(lx, y, lz, water, &registries);
                    y -= 1;
                }
            }
            // Under its lowest floor, pieces with nothing of the level in
            // them down to the land or the sea (whole blocks of the floor
            // over them: a hill's inside, solid, drawn where it meets air).
            // Only the level's inside (`jak_pieces::enclosed`): nothing
            // under a walkway, a balcony or a stand over open air.
            if let Some(low) = low.filter(|&low| floored.contains(&(jx, low, jz))) {
                let mut y = (low + y_off).max(-63) - 1;
                while y > -60 && chunk.block(lx, y, lz) == air {
                    if enclosed.is_some_and(|e| !e.is_enclosed(jx, y - y_off, jz)) {
                        break;
                    }
                    chunk.set_block_raw(lx, y, lz, fill, &registries);
                    y -= 1;
                }
            }
        }
    }
    let block = crate::jak_pieces::block_of(place);
    for (&(jx, jy, jz), &low) in &pieces {

        let y = jy + y_off;
        if !(-63..=top).contains(&y) {
            continue;
        }
        // A low piece under the sea holds its water.
        let wet = low && place.ocean.is_some_and(|ocean| jy < ocean);
        let name = format!("{block}[type={},waterlogged={wet}]", if low { "bottom" } else { "top" });
        let id = state(&name, states);
        chunk.set_block_raw((jx - jx0) as usize, y, (jz - jz0) as usize, id, &registries);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every place from the player's export: its levels read with their
    /// texture coordinates and colours, the square by its landmark cut into
    /// pieces, its landmark found on them, and its textures in the levels'
    /// images. Prints the counts; skips without the export.
    #[test]
    fn every_place_cuts_into_pieces() {
        for (k, place) in PLACES.iter().enumerate() {
            let Some(source) = crate::jak_pieces::source(k) else {
                println!("{}: no export, skipped", place.name);
                continue;
            };
            let (x, z) = (place.landmarks[0].1, place.landmarks[0].2);
            let column = crate::jak_pieces::column(k, (x.div_euclid(16) * 16, z.div_euclid(16) * 16)).unwrap();
            let floors = column.values().filter(|p| p.floor.is_some()).count();
            let textured = embedded_texture(place.textures[0].texture).is_some();
            println!(
                "{}: {} triangles; {} pieces ({floors} floors) by its landmark; first texture in its images: {textured}; landmark {:?}",
                place.name,
                source.tris.len(),
                column.len(),
                landmark(place, None)
            );
            assert!(source.tris.len() > 10_000, "{}", place.name);
            assert!(column.len() > 50, "{}: {}", place.name, column.len());
            assert!(textured, "{}'s textures", place.name);
            assert!(landmark(place, None).is_some(), "{}'s landmark", place.name);
        }
    }

    /// Every place's block is one the world knows.
    #[test]
    fn every_place_block_parses() {
        let Ok(paths) = minecraftoss_core::registries::DataPaths::discover() else { return };
        if !paths.block_catalog.is_file() {
            return;
        }
        let registries = minecraftoss_core::Registries::load(&paths).unwrap();
        for place in &PLACES {
            for t in place.textures {
                for name in [Some(t.block), t.pane, t.layer].into_iter().flatten() {
                    assert!(registries.blocks.parse_state(name).is_ok(), "{name}");
                }
            }
            for kind in ["bottom", "top"] {
                let name = format!("{}[type={kind},waterlogged=false]", crate::jak_pieces::block_of(place));
                assert!(registries.blocks.parse_state(&name).is_ok(), "{name}");
            }
        }
    }

    /// Sandover as the world makes it (seed 12345): nothing but its pieces
    /// over its sea anywhere in its crop (23/10: a Minecraft beach showed in
    /// its bay).
    #[test]
    #[cfg_attr(debug_assertions, ignore = "makes all of Sandover's chunks: run it with --release")]
    fn no_minecraft_land_pokes_out_of_sandover() {
        // (Only with the games' places in the world, `IW4L_LUGARES_JAK=1`.)
        if !crate::original_places::jak_places_on() {
            return;
        }
        let Ok(paths) = minecraftoss_core::registries::DataPaths::discover() else { return };
        if !paths.block_catalog.is_file() {
            return;
        }
        let registries = Arc::new(minecraftoss_core::Registries::load(&paths).unwrap());
        let place = named("villarrefugio").unwrap();
        let k = index_of(place);
        if crate::jak_pieces::source(k).is_none() {
            return;
        }
        let site = sites(registries.clone(), 12345)[k].unwrap();
        let terrain = Arc::new(minecraftoss_generator::terrain::TerrainGenerator::overworld(registries.clone(), 12345).unwrap());
        let gen = Arc::new(WorldGen::new(terrain).unwrap());
        crate::jak_structures::install();
        let mut map = minecraftoss_world::chunk_map::ChunkMap::with_worldgen(gen, 2, 4);
        let blocks = &registries.blocks;
        let sea = place.ocean.unwrap() + place.y_off;
        // Every chunk of its crop.
        let (x0, _, z0) = to_world_block(place, site, (place.crop[0] as i32, 0, place.crop[2] as i32));
        let (x1, _, z1) = to_world_block(place, site, (place.crop[1] as i32, 0, place.crop[3] as i32));
        let mut poking = Vec::new();
        for cx in (x0 >> 4) + 1..(x1 >> 4) {
            for cz in (z0 >> 4) + 1..(z1 >> 4) {
                let chunk = map.load_now(minecraftoss_core::ChunkPos::new(cx, cz));
                for lx in 0..16 {
                    for lz in 0..16 {
                        for y in sea..sea + 40 {
                            let name = blocks.block(blocks.block_of(chunk.block(lx, y, lz))).name.to_string();
                            if !name.ends_with("air") && !name.ends_with("water") && !name.contains("pieza_") {
                                poking.push(format!("{} {y} {} {name}", chunk.pos.x * 16 + lx as i32, chunk.pos.z * 16 + lz as i32));
                            }
                        }
                    }
                }
            }
        }
        println!("{} Minecraft blocks over Sandover's sea; first {:?}", poking.len(), poking.iter().take(12).collect::<Vec<_>>());
        if let Some(first) = poking.first() {
            let v: Vec<i32> = first.split(' ').take(3).map(|t| t.parse().unwrap()).collect();
            let (wx, wz) = (v[0], v[2]);
            let (jx, _, jz) = to_level(place, site, (wx, 0, wz));
            let corner = crate::jak_pieces::chunk_corner(k, site, (wx >> 4, wz >> 4));
            let column = crate::jak_pieces::column(k, corner).unwrap();
            let mut ys: Vec<(i32, usize, Option<f32>)> = column.iter().filter(|(c, _)| c.0 == jx && c.2 == jz).map(|(c, p)| (c.1, p.tris.len(), p.floor)).collect();
            ys.sort_by_key(|y| y.0);
            println!("level {jx} {jz}, chunk corner {corner:?}, crop {:?}, pieces in the column {ys:?}", place.crop);
        }
        assert!(poking.is_empty());
    }

    /// From afar (seed 12345), Sandover's plaza shows its own top (its
    /// ground at about 22 m) and the open water of its crop its sea.
    #[test]
    fn far_off_sandover_is_its_own_ground() {
        let Ok(paths) = minecraftoss_core::registries::DataPaths::discover() else { return };
        if !paths.block_catalog.is_file() {
            return;
        }
        let registries = Arc::new(minecraftoss_core::Registries::load(&paths).unwrap());
        let place = named("villarrefugio").unwrap();
        if crate::jak_pieces::source(index_of(place)).is_none() {
            return;
        }
        let site = sites(registries.clone(), 12345)[index_of(place)].unwrap();
        let (x, _, z) = to_world_block(place, site, (44, 0, -60));
        match far_place(&registries, 12345, x, z) {
            Some(FarPlace::Top { y, .. }) => assert!((y - (22.0 + 63.0)).abs() < 6.0, "plaza top {y}"),
            _ => panic!("no top by the plaza"),
        }
        let sea = (place.crop[0] as i32..place.crop[1] as i32)
            .step_by(7)
            .flat_map(|jx| (place.crop[2] as i32..place.crop[3] as i32).step_by(7).map(move |jz| (jx, jz)))
            .filter(|&(jx, jz)| {
                let (x, _, z) = to_world_block(place, site, (jx, 0, jz));
                matches!(far_place(&registries, 12345, x, z), Some(FarPlace::Sea(_)))
            })
            .count();
        println!("{sea} sampled columns of Sandover's crop are its sea");
        assert!(sea > 100);
    }

    /// The sites of a world (seed 12345): each found, none over another.
    #[test]
    fn the_places_have_their_sites() {
        let Ok(paths) = minecraftoss_core::registries::DataPaths::discover() else { return };
        if !paths.block_catalog.is_file() {
            return;
        }
        let registries = Arc::new(minecraftoss_core::Registries::load(&paths).unwrap());
        let sites = sites(registries, 12345);
        println!("sites: {:?}", PLACES.iter().map(|p| p.name).zip(sites).collect::<Vec<_>>());
        // (Spargus's level stands only when asked for: `spargus_town`.)
        assert!(sites.iter().skip(1).all(Option::is_some), "{sites:?}");
        assert_eq!(sites[0].is_some(), crate::spargus_town::level_wanted());
    }
}
