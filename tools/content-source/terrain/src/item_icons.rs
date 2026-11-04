//! Inventory icons and names for items, as MinecraftOSS's viewer makes them
//! (`engine/viewer/src/interface.rs`: `load_catalog_icons`, `item_name`).
use crate::interface::{item_model_reference, item_model_tints};
use crate::pack::{PackStack, ResourceId};
use anyhow::Result;
use image::{RgbaImage, imageops::FilterType};
use std::collections::HashMap;

/// An item's GUI icon: its block model rastered, its generated layers, or
/// its flat texture.
pub fn item_icon(packs: &PackStack, key: &str, icon_size: usize) -> Result<Option<RgbaImage>> {
    if let Some(key)=key.strip_prefix("bannercloth:") {
        let Some((key,part))=key.rsplit_once('#') else {return Ok(None)};
        let Some(front)=banner_front(packs,key)? else {return Ok(None)};
        let mut tile=RgbaImage::new(32,32);
        let y=if part=="1" {32}else{0};
        image::imageops::overlay(&mut tile,&image::imageops::crop_imm(&front,0,y,20,(40-y).min(32)).to_image(),0,0);
        return Ok(Some(tile));
    }
    if let Some((item,color))=key.split_once("#potion=") {
        let color=u32::from_str_radix(color,16).unwrap_or(0x385dc6);
        let tint=[(color>>16) as u8,(color>>8) as u8,color as u8];
        let id=ResourceId::parse(item)?;
        if let Some(def)=packs.item_definition(&id)? {
            if let Some(model)=item_model_reference(&def["model"]) {
                return generated_item_icon(packs,model,&[tint],icon_size);
            }
        }
        return item_icon(packs,item,icon_size);
    }
    if key.contains("#banner=") {return banner_icon(packs,key,icon_size);}
    if let Some(tile)=key.strip_prefix("preview:") {return preview_tile(packs,tile);}
    if let Some((item,material))=key.split_once("#trim=") {
        let id=ResourceId::parse(item)?;
        if let Some(def)=packs.item_definition(&id)? {
            let select=&def["model"];
            if select["property"]=="minecraft:trim_material" {
                if let Some(model)=select["cases"].as_array().into_iter().flatten()
                    .find(|case|case["when"].as_str()==Some(material))
                    .and_then(|case|item_model_reference(&case["model"])) {
                return generated_item_icon(packs,model,&[],icon_size);
                }
            }
        }
        return item_icon(packs,item,icon_size);
    }
    let id = ResourceId::parse(key)?;
    let definition = packs.item_definition(&id)?;
    // Drawn in vanilla by a special renderer (chests, heads, shulker boxes,
    // banners, the shield, the trident...): cut from what it paints them
    // with, or its base model (16/10: their icons were missing).
    if let Some(special) = definition.as_ref().and_then(find_special) {
        match special_icon(packs, special, &id.path, icon_size) {
            Ok(Some(icon)) => return Ok(Some(icon)),
            Ok(None) => {}
            Err(error) => eprintln!("special icon of {key}: {error}"),
        }
    }
    let model = definition
        .as_ref()
        .and_then(|value| item_model_reference(&value["model"]));
    let tints = definition
        .as_ref()
        .map(|value| item_model_tints(packs, &value["model"]))
        .transpose()?
        .unwrap_or_default();
    if let Some(icon) = model
        .map(|model| crate::item_icon::block_icon(packs, model, &tints, icon_size))
        .transpose()?
        .flatten()
    {
        return Ok(Some(icon));
    }
    if let Some(icon) = model
        .map(|model| generated_item_icon(packs, model, &tints, icon_size))
        .transpose()?
        .flatten()
    {
        return Ok(Some(icon));
    }
    let texture = model
        .map(|model| model_texture(packs, model))
        .transpose()?
        .flatten();
    let texture =
        texture.or_else(|| ResourceId::parse(&format!("{}:item/{}", id.namespace, id.path)).ok());
    let Some(texture) = texture else {
        return Ok(None);
    };
    let Some(bytes) = packs.texture(&texture)? else {
        return Ok(None);
    };
    let decoded = image::load_from_memory(&bytes)?.to_rgba8();
    let side = decoded.width().min(decoded.height());
    let first = image::imageops::crop_imm(&decoded, 0, 0, side, side).to_image();
    Ok(Some(image::imageops::resize(
        &first,
        icon_size as u32,
        icon_size as u32,
        FilterType::Nearest,
    )))
}

/// Lossless 32px tiles for a HUD entity model; trim palettes and matching-material
/// overrides come from Java's equipment resources. Atlas keys never become items.
fn preview_tile(packs:&PackStack,key:&str)->Result<Option<RgbaImage>> {
    let Some((key,tile))=key.rsplit_once('#') else {return Ok(None)};
    let tile:u32=tile.parse()?;
    let mut parts=key.split('@');
    let texture=ResourceId::parse(parts.next().unwrap())?;
    let Some(bytes)=packs.texture(&texture)? else {return Ok(None)};
    let mut source=image::load_from_memory(&bytes)?.to_rgba8();
    if let (Some(material),Some(asset))=(parts.next(),parts.next()) {
        let material=format!("minecraft:{material}");
        let equipment=packs.equipment(&ResourceId::parse(&format!("minecraft:{asset}"))?)?;
        let palette=equipment.as_ref().and_then(|v|v["trim_overrides"].as_array())
            .and_then(|v|v.iter().find(|v|v["when"]["material"].as_str()==Some(&material)))
            .and_then(|v|v["palette"].as_str()).map(str::to_owned)
            .unwrap_or_else(||format!("minecraft:trim/{}",material.trim_start_matches("minecraft:")));
        let mut palette=ResourceId::parse(&palette)?;palette.path=format!("palettes/{}",palette.path);
        let base=packs.animation(&texture)?.and_then(|v|v["palette"]["base_palette"].as_str().map(str::to_owned)).unwrap_or_else(||"trim_base".into());
        let mut base=ResourceId::parse(&base)?;base.path=format!("palettes/{}",base.path);
        let (Some(from),Some(to))=(packs.texture(&base)?,packs.texture(&palette)?) else {return Ok(None)};
        let from=image::load_from_memory(&from)?.to_rgba8();let to=image::load_from_memory(&to)?.to_rgba8();
        let colors:HashMap<[u8;3],[u8;4]>=from.pixels().zip(to.pixels()).map(|(a,b)|([a[0],a[1],a[2]],b.0)).collect();
        for p in source.pixels_mut() {
            if p[3]>0 {if let Some(c)=colors.get(&[p[0],p[1],p[2]]) {
                *p=image::Rgba([c[0],c[1],c[2],(u16::from(p[3])*u16::from(c[3])/255) as u8]);
            }}
        }
    }
    let width=source.width();let height=if key.contains("armorstand") {64}else{32};
    let source=image::imageops::resize(&source,64,height,FilterType::Nearest);
    let (x,y)=((tile%2)*32,(tile/2)*32);
    if width==0 || y+32>height {return Ok(None)};
    Ok(Some(image::imageops::crop_imm(&source,x,y,32,32).to_image()))
}

/// The first `minecraft:special` model in an item definition (its
/// fallback before its cases).
fn find_special(value: &serde_json::Value) -> Option<&serde_json::Value> {
    if value.get("type").and_then(serde_json::Value::as_str) == Some("minecraft:special") {
        return Some(value);
    }
    for key in ["model", "fallback", "on_false", "on_true"] {
        if let Some(found) = value.get(key).filter(|v| v.is_object()).and_then(find_special) {
            return Some(found);
        }
    }
    value
        .get("cases")
        .and_then(serde_json::Value::as_array)
        .and_then(|cases| cases.iter().find_map(find_special))
}

/// A texture of the pack, decoded.
fn pack_texture(packs: &PackStack, path: &str) -> Result<Option<RgbaImage>> {
    let id = ResourceId::parse(path)?;
    Ok(packs
        .texture(&id)?
        .and_then(|bytes| image::load_from_memory(&bytes).ok())
        .map(|img| img.to_rgba8()))
}

/// Cut `(x, y, w, h)` of a texture laid out at `width` across (scaled to
/// the texture's own size), on a square of its larger side.
fn cut(texture: &RgbaImage, layout_width: u32, rect: [u32; 4]) -> RgbaImage {
    let k = (texture.width() / layout_width.max(1)).max(1);
    let [x, y, w, h] = rect.map(|v| v * k);
    let piece = image::imageops::crop_imm(texture, x, y, w.min(texture.width().saturating_sub(x)), h.min(texture.height().saturating_sub(y))).to_image();
    let side = piece.width().max(piece.height());
    let mut square = RgbaImage::new(side, side);
    image::imageops::overlay(&mut square, &piece, i64::from((side - piece.width()) / 2), i64::from((side - piece.height()) / 2));
    square
}

/// A dye's colour (`DyeColor` texture diffuse colours).
pub fn dye(name: &str) -> [u8; 3] {
    match name {
        "orange" => [0xF9, 0x80, 0x1D],
        "magenta" => [0xC7, 0x4E, 0xBD],
        "light_blue" => [0x3A, 0xB3, 0xDA],
        "yellow" => [0xFE, 0xD8, 0x3D],
        "lime" => [0x80, 0xC7, 0x1F],
        "pink" => [0xF3, 0x8B, 0xAA],
        "gray" => [0x47, 0x4F, 0x52],
        "light_gray" => [0x9D, 0x9D, 0x97],
        "cyan" => [0x16, 0x9C, 0x9C],
        "purple" => [0x89, 0x32, 0xB8],
        "blue" => [0x3C, 0x44, 0xAA],
        "brown" => [0x83, 0x54, 0x32],
        "green" => [0x5E, 0x7C, 0x16],
        "red" => [0xB0, 0x2E, 0x26],
        "black" => [0x1D, 0x1D, 0x21],
        _ => [0xF9, 0xFF, 0xFE],
    }
}

/// A special-rendered item's icon: its chest's front, its head's face, its
/// shulker box's side, its banner in its colour, the shield's face; else
/// its base model's texture.
fn special_icon(packs: &PackStack, special: &serde_json::Value, path: &str, size: usize) -> Result<Option<RgbaImage>> {
    let inner = &special["model"];
    let kind = inner.get("type").and_then(serde_json::Value::as_str).unwrap_or("");
    let name = |v: &serde_json::Value| v.as_str().map(|s| s.trim_start_matches("minecraft:").to_owned());
    let icon: Option<RgbaImage> = match kind {
        "minecraft:chest" => {
            let texture = name(&inner["texture"]).unwrap_or_else(|| "normal".into());
            pack_texture(packs, &format!("minecraft:entity/chest/{texture}"))?.map(|t| {
                // The lid's front over the base's, the latch on them.
                let k = (t.width() / 64).max(1);
                let lid = image::imageops::crop_imm(&t, 14 * k, 14 * k, 14 * k, 5 * k).to_image();
                let base = image::imageops::crop_imm(&t, 14 * k, 33 * k, 14 * k, 10 * k).to_image();
                let latch = image::imageops::crop_imm(&t, k, k, 2 * k, 4 * k).to_image();
                let mut front = RgbaImage::new(16 * k, 16 * k);
                image::imageops::overlay(&mut front, &lid, i64::from(k), i64::from(k));
                image::imageops::overlay(&mut front, &base, i64::from(k), i64::from(6 * k));
                image::imageops::overlay(&mut front, &latch, i64::from(7 * k), i64::from(4 * k));
                front
            })
        }
        "minecraft:head" => {
            let kind = name(&inner["kind"]).unwrap_or_default();
            let (texture, layout, rect) = match kind.as_str() {
                "skeleton" => ("entity/skeleton/skeleton", 64, [8, 8, 8, 8]),
                "wither_skeleton" => ("entity/skeleton/wither_skeleton", 64, [8, 8, 8, 8]),
                "zombie" => ("entity/zombie/zombie", 64, [8, 8, 8, 8]),
                "creeper" => ("entity/creeper/creeper", 64, [8, 8, 8, 8]),
                "piglin" => ("entity/piglin/piglin", 64, [8, 8, 10, 8]),
                "dragon" => ("entity/enderdragon/dragon", 256, [128, 46, 16, 16]),
                _ => ("entity/player/wide/steve", 64, [8, 8, 8, 8]),
            };
            pack_texture(packs, &format!("minecraft:{texture}"))?.map(|t| cut(&t, layout, rect))
        }
        "minecraft:shulker_box" => pack_texture(packs, &format!("minecraft:block/{path}"))?
            .or(pack_texture(packs, "minecraft:block/shulker_box")?),
        "minecraft:banner" => return banner_icon(packs,&format!("minecraft:{path}"),size),
        "minecraft:shield" => pack_texture(packs, "minecraft:entity/shield/shield_base")?.map(|t| cut(&t, 64, [1, 1, 12, 22])),
        _ => None,
    };
    if let Some(icon) = icon {
        return Ok(Some(image::imageops::resize(&icon, size as u32, size as u32, FilterType::Nearest)));
    }
    // Its base model: rastered, its layers, or a texture of it.
    let Some(base) = special.get("base").and_then(serde_json::Value::as_str) else {
        return Ok(None);
    };
    if let Some(icon) = crate::item_icon::block_icon(packs, base, &Vec::new(), size)? {
        return Ok(Some(icon));
    }
    if let Some(icon) = generated_item_icon(packs, base, &Vec::new(), size)? {
        return Ok(Some(icon));
    }
    let Some(texture) = model_texture(packs, base)? else {
        return Ok(None);
    };
    let Some(bytes) = packs.texture(&texture)? else {
        return Ok(None);
    };
    let decoded = image::load_from_memory(&bytes)?.to_rgba8();
    let side = decoded.width().min(decoded.height());
    let first = image::imageops::crop_imm(&decoded, 0, 0, side, side).to_image();
    Ok(Some(image::imageops::resize(&first, size as u32, size as u32, FilterType::Nearest)))
}

/// The pack's English names (`lang/en_us.json`).
pub fn language(packs: &PackStack) -> Result<HashMap<String, String>> {
    // The game speaks Spanish: its names over the English ones, which fill
    // whatever the Spanish file lacks (or all of it, before it is fetched).
    let mut names = HashMap::new();
    for code in ["en_us", "es_es"] {
        let id = ResourceId::parse(&format!("minecraft:lang/{code}"))?;
        let Some(value) = packs.json(&id, &format!("lang/{code}.json"))? else {
            continue;
        };
        if let Some(entries) = value.as_object() {
            names.extend(
                entries
                    .iter()
                    .filter_map(|(key, text)| Some((key.clone(), text.as_str()?.to_owned()))),
            );
        }
    }
    Ok(names)
}

/// `item_name`: the item's or its block's name, else its id's path.
pub fn item_name(language: &HashMap<String, String>, id: &str) -> String {
    let path = id.split(':').nth(1).unwrap_or(id);
    language
        .get(&format!("item.minecraft.{path}"))
        .or_else(|| language.get(&format!("block.minecraft.{path}")))
        .cloned()
        .unwrap_or_else(|| path.replace('_', " "))
}

/// Item/generated renders each layer in order; layer 0's tint is the liquid,
/// and the untinted bottle on layer 1 is drawn over it.
fn generated_item_icon(
    packs: &PackStack,
    model: &str,
    tints: &[[u8; 3]],
    icon_size: usize,
) -> Result<Option<RgbaImage>> {
    let mut textures = HashMap::<String, String>::new();
    let mut current = Some(ResourceId::parse(model)?);
    for _ in 0..12 {
        let Some(id) = current.take() else { break };
        let Some(value) = packs.model(&id)? else {
            break;
        };
        if let Some(entries) = value.get("textures").and_then(serde_json::Value::as_object) {
            for (key, texture) in entries {
                if let Some(texture) = texture
                    .as_str()
                    .or_else(|| texture.get("sprite").and_then(serde_json::Value::as_str))
                {
                    textures
                        .entry(key.clone())
                        .or_insert_with(|| texture.into());
                }
            }
        }
        current = value
            .get("parent")
            .and_then(serde_json::Value::as_str)
            .map(ResourceId::parse)
            .transpose()?;
    }
    if !textures.contains_key("layer1") {
        return Ok(None);
    }
    let mut icon = RgbaImage::new(icon_size as u32, icon_size as u32);
    for layer in 0..16 {
        let key = format!("layer{layer}");
        let Some(mut texture) = textures.get(&key).map(String::as_str) else {
            break;
        };
        for _ in 0..8 {
            if let Some(reference) = texture.strip_prefix('#') {
                let Some(next) = textures.get(reference) else {
                    break;
                };
                texture = next;
            } else {
                break;
            }
        }
        let Some(source) = item_sprite(packs,&ResourceId::parse(texture)?)? else {
            continue;
        };
        let side = source.width().min(source.height());
        if side == 0 {
            continue;
        }
        let tint = tints.get(layer).copied().unwrap_or([255; 3]);
        let mut frame = image::imageops::crop_imm(&source, 0, 0, side, side).to_image();
        for pixel in frame.pixels_mut() {
            for channel in 0..3 {
                pixel[channel] = (pixel[channel] as u16 * tint[channel] as u16 / 255) as u8;
            }
        }
        let scaled = image::imageops::resize(
            &frame,
            icon_size as u32,
            icon_size as u32,
            FilterType::Nearest,
        );
        image::imageops::overlay(&mut icon, &scaled, 0, 0);
    }
    Ok((icon.pixels().any(|pixel| pixel[3] > 0)).then_some(icon))
}

/// Java's paletted_permutations are atlas sprites, not PNGs on disk. Resolve
/// their source texture and native palette instead of silently losing layer1.
fn item_sprite(packs:&PackStack,id:&ResourceId)->Result<Option<RgbaImage>> {
    if let Some(bytes)=packs.texture(id)? {return Ok(Some(image::load_from_memory(&bytes)?.to_rgba8()));}
    let Some(atlas)=packs.atlas(&ResourceId::parse("minecraft:items")?)? else {return Ok(None);};
    for source in atlas["sources"].as_array().into_iter().flatten() {
        if source["type"]!="minecraft:paletted_permutations" {continue;}
        for base in source["textures"].as_array().into_iter().flatten().filter_map(serde_json::Value::as_str) {
            let Some(suffix)=id.key().strip_prefix(&format!("{base}_")).map(str::to_owned) else {continue;};
            let Some(palette)=source["permutations"][&suffix].as_str() else {continue;};
            let Some(key)=source["palette_key"].as_str() else {continue;};
            let palette_id=|name:&str|->Result<ResourceId>{let mut id=ResourceId::parse(name)?;id.path=format!("palettes/{}",id.path);Ok(id)};
            let (Some(raw),Some(from),Some(to))=(packs.texture(&ResourceId::parse(base)?)?,packs.texture(&palette_id(key)?)?,packs.texture(&palette_id(palette)?)?) else {continue;};
            let mut sprite=image::load_from_memory(&raw)?.to_rgba8();
            let from=image::load_from_memory(&from)?.to_rgba8();let to=image::load_from_memory(&to)?.to_rgba8();
            let colors:HashMap<[u8;3],[u8;4]>=from.pixels().zip(to.pixels()).map(|(a,b)|([a[0],a[1],a[2]],b.0)).collect();
            for p in sprite.pixels_mut() {
                if p[3]==0 {continue;}
                if let Some(c)=colors.get(&[p[0],p[1],p[2]]) {let alpha=(u16::from(p[3])*u16::from(c[3])/255) as u8;*p=image::Rgba([c[0],c[1],c[2],alpha]);}
            }
            return Ok(Some(sprite));
        }
    }
    Ok(None)
}

fn model_texture(packs: &PackStack, model: &str) -> Result<Option<ResourceId>> {
    let mut textures = HashMap::<String, String>::new();
    let mut current = Some(ResourceId::parse(model)?);
    for _ in 0..12 {
        let Some(id) = current.take() else { break };
        let Some(value) = packs.model(&id)? else {
            break;
        };
        if let Some(entries) = value.get("textures").and_then(serde_json::Value::as_object) {
            for (key, texture) in entries {
                if let Some(texture) = texture
                    .as_str()
                    .or_else(|| texture.get("sprite").and_then(serde_json::Value::as_str))
                {
                    textures
                        .entry(key.clone())
                        .or_insert_with(|| texture.into());
                }
            }
        }
        current = value
            .get("parent")
            .and_then(serde_json::Value::as_str)
            .map(ResourceId::parse)
            .transpose()?;
    }
    for key in [
        "layer0", "all", "top", "up", "side", "front", "end", "particle",
    ] {
        let Some(mut texture) = textures.get(key).map(String::as_str) else {
            continue;
        };
        for _ in 0..8 {
            if let Some(reference) = texture.strip_prefix('#') {
                let Some(next) = textures.get(reference) else {
                    break;
                };
                texture = next;
            } else {
                return ResourceId::parse(texture).map(Some);
            }
        }
    }
    Ok(None)
}

#[cfg(test)]
mod every_icon_tests {
    use super::*;

    /// Every item of the pack has an icon (the creative screen shows them
    /// all; 16/10: chests, heads, shulker boxes, banners had none).
    #[test]
    fn every_item_has_an_icon() {
        let Ok(paths) = minecraftoss_core::registries::DataPaths::discover() else {
            return;
        };
        let Some(pack) = paths
            .datapack
            .parent()
            .and_then(|p| p.parent())
            .and_then(|p| p.parent())
            .map(|root| root.join("resourcepacks/local/minecraft-26.3"))
            .filter(|p| p.is_dir())
        else {
            return;
        };
        let packs = PackStack::open(vec![pack]).unwrap();
        let items: Vec<String> = packs
            .list("minecraft", "items/")
            .unwrap()
            .into_iter()
            .filter_map(|p| p.rsplit('/').next().and_then(|f| f.strip_suffix(".json")).map(|s| format!("minecraft:{s}")))
            .filter(|id| id != "minecraft:air")
            .collect();
        assert!(items.len() > 1200, "{} items", items.len());
        let missing: Vec<(&String, String)> = items
            .iter()
            .filter_map(|id| match item_icon(&packs, id, 32) {
                Ok(Some(_)) => None,
                other => Some((id, format!("{:?}", other.map(|o| o.is_some())))),
            })
            .collect();
        assert!(missing.is_empty(), "{} of {} without an icon: {missing:?}", missing.len(), items.len());
    }
}

/// Java BannerFlagModel has a 20x40 cloth, UV (1,1), in a 64x64 atlas.
/// Paint native masks in component order; no invented silhouette or coloured placeholder.
fn banner_front(packs:&PackStack,key:&str)->Result<Option<RgbaImage>> {
    let (item,patterns)=key.split_once("#banner=").unwrap_or((key,""));
    let color=item.strip_prefix("minecraft:").and_then(|s|s.strip_suffix("_banner")).unwrap_or("white");
    let Some(mut texture)=pack_texture(packs,"minecraft:entity/banner/base")? else {return Ok(None)};
    texture=image::imageops::resize(&texture,64,64,FilterType::Nearest);
    let tint=dye(color);
    for p in texture.pixels_mut() {for k in 0..3 {p[k]=(u16::from(p[k])*u16::from(tint[k])/255) as u8;}}
    for layer in patterns.split(';').take(16) {
        let Some((pattern,color))=layer.split_once('=') else {continue;};
        let id=ResourceId::parse(pattern)?;
        let Some(mask)=pack_texture(packs,&format!("{}:entity/banner/{}",id.namespace,id.path))? else {continue;};
        let mut mask=image::imageops::resize(&mask,64,64,FilterType::Nearest);
        let tint=dye(color);
        for p in mask.pixels_mut() {for k in 0..3 {p[k]=(u16::from(p[k])*u16::from(tint[k])/255) as u8;}}
        image::imageops::overlay(&mut texture,&mask,0,0);
    }
    Ok(Some(image::imageops::crop_imm(&texture,1,1,20,40).to_image()))
}
fn banner_icon(packs:&PackStack,key:&str,size:usize)->Result<Option<RgbaImage>> {
    let Some(front)=banner_front(packs,key)? else {return Ok(None)};
    let mut icon=RgbaImage::new(48,48);
    if let Some(wood)=pack_texture(packs,"minecraft:entity/banner/banner_base")? {
        let wood=image::imageops::resize(&wood,64,64,FilterType::Nearest);
        image::imageops::overlay(&mut icon,&image::imageops::crop_imm(&wood,2,44,2,4).to_image(),23,43);
        image::imageops::overlay(&mut icon,&image::imageops::crop_imm(&wood,0,43,22,2).to_image(),13,1);
    }
    image::imageops::overlay(&mut icon,&front,14,3);
    Ok(Some(image::imageops::resize(&icon,size as u32,size as u32,FilterType::Nearest)))
}

/// Liquid tint shared by icons, the item in hand, and dropped potion entities.
pub fn potion_color(stack:&minecraftoss_player::inventory::ItemStack)->Option<u32> {
    if !matches!(stack.id.as_str(),"minecraft:potion"|"minecraft:splash_potion"|"minecraft:lingering_potion"|"minecraft:tipped_arrow") {return None;}
    let contents=stack.components.as_ref().and_then(|v|v.get("minecraft:potion_contents"));
    if let Some(color)=contents.and_then(|v|v["custom_color"].as_u64()) {return Some(color as u32 & 0xffffff);}
    let mut effects=contents.and_then(|v|v["potion"].as_str()).and_then(minecraftoss_entities::potion::registry).unwrap_or_default();
    for e in contents.and_then(|v|v["custom_effects"].as_array()).into_iter().flatten() {
        if let Some(kind)=e["id"].as_str().and_then(minecraftoss_entities::effects::MobEffect::from_id) {
            let mut effect=minecraftoss_entities::effects::EffectInstance::new(kind,1,e["amplifier"].as_i64().unwrap_or(0) as i32);
            effect.visible=e["show_particles"].as_bool().unwrap_or(true);effects.push(effect);
        }
    }
    Some(minecraftoss_entities::potion::contents_color(&effects))
}
pub fn potion_tint(stack:&minecraftoss_player::inventory::ItemStack)->Option<[f32;3]> {
    potion_color(stack).map(|c|[((c>>16)&255) as f32/255.0,((c>>8)&255) as f32/255.0,(c&255) as f32/255.0])
}
