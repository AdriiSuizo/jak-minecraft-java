//! Jak's animations as the player's own art groups (`*.go`, the game's
//! linked object files) hold them: each animation's playback speed and
//! artist frame numbering, and the motion of its align joint and the
//! placing of its prejoint frame by frame, which the GLB export leaves out,
//! and the effects its frames fire (`effect-name` and `effect-param` in its
//! res lump: the footsteps, landings and rolls `effect-control` plays).
use std::collections::HashMap;

/// One effect an animation fires as it reaches an artist frame: its name
/// (`effect-walk-step-left`, or a sound's own, `hand-grab`) and the volume
/// (a fraction) and pitch (octaves) its `effect-param` gives, if any.
#[derive(Clone, Debug, PartialEq)]
pub struct ArtEffect {
    pub frame: f32,
    pub name: String,
    pub volume: Option<f32>,
    pub pitch: Option<f32>,
}

/// One animation's numbering and motion.
#[derive(Clone, Debug, PartialEq)]
pub struct ArtAnim {
    pub speed: f32,
    pub artist_base: f32,
    pub artist_step: f32,
    /// The align joint at each frame: translation (units) then rotation as
    /// a quaternion (x, y, z, w), in the animation's own frame.
    pub align: Option<Vec<[f32; 7]>>,
    /// The prejoint at each frame, which every bone of the mesh hangs
    /// from, as a column-major matrix (units), when it is not where the
    /// root is.
    pub prejoint: Option<Vec<[f32; 16]>>,
    pub joints: Option<Vec<Vec<[f32; 10]>>>,
    /// The effects its frames fire, by artist frame.
    pub effects: Vec<ArtEffect>,
    /// Its eyes at each frame (`art-joint-anim`'s `eye-anim`), left and
    /// right: as `convert-eye-data` reads a `merc-eye-anim-frame`, the
    /// pupil's x and y and the lid (signed bytes over 128), then the iris,
    /// pupil and lid scales (bytes over 64).
    pub eyes: Option<Vec<[Eye; 2]>>,
}

/// One eye at one frame (`eye`: `x`, `y`, `lid`, `iris-scale`,
/// `pupil-scale`, `lid-scale`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Eye {
    pub x: f32,
    pub y: f32,
    pub lid: f32,
    pub iris_scale: f32,
    pub pupil_scale: f32,
    pub lid_scale: f32,
}

/// The eye frames of a `merc-eye-anim-block` at `at`: its `max-frame`
/// (int16) and then two `merc-eye-anim-frame` dwords a frame (left, right)
/// from its eighth byte.
fn eyes(seg: &[u8], at: usize) -> Option<Vec<[Eye; 2]>> {
    let max = i16::from_le_bytes(seg.get(at..at + 2)?.try_into().ok()?);
    if !(0..4096).contains(&max) {
        return None;
    }
    let eye = |p: usize| -> Option<Eye> {
        let b = seg.get(p..p + 8)?;
        Some(Eye {
            x: f32::from(b[0] as i8) / 128.0,
            y: f32::from(b[1] as i8) / 128.0,
            lid: f32::from(b[2] as i8) / 128.0,
            iris_scale: f32::from(b[4]) / 64.0,
            pupil_scale: f32::from(b[5]) / 64.0,
            lid_scale: f32::from(b[6]) / 64.0,
        })
    };
    (0..=max as usize).map(|f| Some([eye(at + 8 + f * 16)?, eye(at + 16 + f * 16)?])).collect()
}

/// Every one of Jak's animations in a linked art group, by name.
pub fn read(bytes: &[u8]) -> HashMap<String, ArtAnim> {
    read_prefixes(bytes,&[b"jakb-",b"daxter-",b"gun-",b"board-",b"hellcat-"])
}

/// NPC art groups carry the same animation resources and authored events.
pub fn read_named(bytes:&[u8],prefix:&str)->HashMap<String,ArtAnim>{
    if prefix.is_empty(){return HashMap::new();}
    read_prefixes(bytes,&[prefix.as_bytes()])
}

fn read_prefixes(bytes:&[u8],prefixes:&[&[u8]])->HashMap<String,ArtAnim>{
    let mut out = HashMap::new();
    let Some(header) = u32_at(bytes, 4)
        .map(|h| h as usize)
        .filter(|&h| h < bytes.len())
    else {
        return out;
    };
    let seg = &bytes[header..];
    let mut names = HashMap::new();
    let mut i = 4;
    while i + 5 <= seg.len() {
        if !prefixes.iter().any(|prefix| seg[i..].starts_with(prefix))
        {
            i += 1;
            continue;
        }
        let end = seg[i..]
            .iter()
            .position(|&b| !(b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-'))
            .map_or(seg.len(), |n| i + n);
        if seg.get(end) == Some(&0)
            && u32_at(seg, i - 4) == Some((end - i) as u32)
            && let Ok(name) = std::str::from_utf8(&seg[i..end])
        {
            names.insert((i - 4) as u32, name.to_owned());
        }
        i = end.max(i + 1);
    }
    for at in (4..seg.len().saturating_sub(4)).step_by(4) {
        let Some(name) = u32_at(seg, at).and_then(|w| names.get(&w)) else {
            continue;
        };
        let anim = at - 4;
        let (Some(speed), Some(artist_base), Some(artist_step)) = (
            f32_at(seg, anim + 44),
            f32_at(seg, anim + 48),
            f32_at(seg, anim + 52),
        ) else {
            continue;
        };
        if 0.0 < speed
            && speed <= 4.0
            // (`jakb-swim-down` and `-up` count 8.57 artist frames a frame, `powerup` 9.0.)
            && 0.0 < artist_step
            && artist_step <= 12.0
            && artist_base.abs() < 1000.0
        {
            let matrices = u32_at(seg, anim + 32).and_then(|frames| matrices(seg, frames as usize));
            let (align, prejoint) = matrices.map_or((None, None), |(a, p)| (Some(a), p));
            out.insert(
                name.clone(),
                ArtAnim {
                    speed,
                    artist_base,
                    artist_step,
                    align,
                    prejoint,
                    joints: u32_at(seg, anim + 32).and_then(|at| joints(seg, at as usize)),
                    effects: u32_at(seg, anim + 12)
                        .map_or_else(Vec::new, |lump| effects(seg, lump as usize, &HashMap::new(), 1.0)),
                    eyes: u32_at(seg, anim + 24).filter(|&p| p != 0).and_then(|p| eyes(seg, p as usize)),
                },
            );
        }
    }
    out
}

/// Jak 1's animations in one of its linked art groups (`eichar-*-ag.go`,
/// the game's v4 objects): the data after the 16-byte link header, its
/// pointers relative to it; an animation is named at 8, its speed and
/// artist numbering at 20 to 28 and its compressed frames (the same
/// format as Jak 2's and 3's, never LZO packed) at 44.
pub fn read_jak1(bytes: &[u8]) -> HashMap<String, ArtAnim> {
    read_jak1_named(bytes, "eichar-")
}

/// Jak 1 creature art groups have the same format as the player groups.
pub fn read_jak1_named(bytes: &[u8], prefix: &str) -> HashMap<String, ArtAnim> {
    read_v4_named(bytes,prefix,0.01)
}

/// Jak 2 still uses linked v4 objects, with fractional cue volume.
pub fn read_v4_named(bytes: &[u8], prefix: &str, volume_scale: f32) -> HashMap<String, ArtAnim> {
    let mut out = HashMap::new();
    let Some(seg) = bytes.get(16..) else {
        return out;
    };
    let mut names = HashMap::new();
    let mut i = 4;
    while i + 7 <= seg.len() {
        if prefix.is_empty() || !seg[i..].starts_with(prefix.as_bytes()) {
            i += 1;
            continue;
        }
        let end = seg[i..]
            .iter()
            .position(|&b| !(b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-'))
            .map_or(seg.len(), |n| i + n);
        if seg.get(end) == Some(&0)
            && u32_at(seg, i - 4) == Some((end - i) as u32)
            && let Ok(name) = std::str::from_utf8(&seg[i..end])
        {
            names.insert((i - 4) as u32, name.to_owned());
        }
        i = end.max(i + 1);
    }
    let symbols = jak1_symbols(bytes);
    for at in (8..seg.len().saturating_sub(4)).step_by(4) {
        let Some(name) = u32_at(seg, at).and_then(|w| names.get(&w)) else {
            continue;
        };
        let anim = at - 8;
        let (Some(speed), Some(artist_base), Some(artist_step), Some(control)) = (
            f32_at(seg, anim + 20),
            f32_at(seg, anim + 24),
            f32_at(seg, anim + 28),
            u32_at(seg, anim + 44),
        ) else {
            continue;
        };
        if 0.0 < speed && speed <= 4.0 && 0.0 < artist_step && artist_step <= 8.0 && artist_base.abs() < 1000.0 {
            let matrices = matrices(seg, control as usize);
            let (align, prejoint) = matrices.map_or((None, None), |(a, p)| (Some(a), p));
            out.insert(
                name.clone(),
                ArtAnim {
                    speed,
                    artist_base,
                    artist_step,
                    align,
                    prejoint,
                    joints: joints(seg, control as usize),
                    eyes: None,
                    // Jak 1 gives its volumes in percent.
                    effects: u32_at(seg, anim + 16)
                        .map_or_else(Vec::new, |lump| effects(seg, lump as usize, &symbols, volume_scale)),
                },
            );
        }
    }
    out
}

/// The effects in an animation's res lump (`extra`) at `lump`: each
/// `effect-name` tag's name at its key frame, with the volume and pitch of
/// an `effect-param` tag at the same frame. The tags' own names are
/// linked symbols the file leaves blank, so they are told apart by their
/// data: a name is a string (Jak 2 and 3) or a linked symbol (Jak 1, its
/// name from `symbols`), a parameter an inline list of (kind, value) floats.
/// `percent` scales the volume to a fraction.
fn effects(seg: &[u8], lump: usize, symbols: &HashMap<usize, String>, percent: f32) -> Vec<ArtEffect> {
    let mut out = Vec::new();
    let (Some(length), Some(data_base), Some(tags)) =
        (u32_at(seg, lump), u32_at(seg, lump + 8), u32_at(seg, lump + 24))
    else {
        return out;
    };
    if lump == 0 || lump >= seg.len() || length == 0 || length > 256 {
        return out;
    }
    let mut params: Vec<(f32, Option<f32>, Option<f32>)> = Vec::new();
    for k in 0..length as usize {
        let tag = tags as usize + 16 * k;
        let (Some(key), Some(word)) = (f32_at(seg, tag + 4), u32_at(seg, tag + 12)) else {
            break;
        };
        let at = data_base as usize + (word & 0xffff) as usize;
        let count = ((word >> 16) & 0x7fff) as usize;
        let inlined = word >> 31 == 1;
        if !inlined && count == 1 {
            let name = match u32_at(seg, at) {
                Some(0xffff_ffff) => symbols.get(&at).cloned(),
                Some(p) => goal_string(seg, p as usize),
                None => None,
            };
            if let Some(name) = name.filter(|n| n.starts_with(|c: char| c.is_ascii_lowercase())) {
                out.push(ArtEffect {
                    frame: key,
                    name,
                    volume: None,
                    pitch: None,
                });
            }
        } else if inlined && count >= 2 && count % 2 == 0 && count <= 16 {
            let pairs: Option<Vec<(f32, f32)>> = (0..count / 2)
                .map(|i| Some((f32_at(seg, at + 8 * i)?, f32_at(seg, at + 8 * i + 4)?)))
                .collect();
            let Some(pairs) = pairs else { continue };
            if !pairs.iter().all(|(kind, _)| kind.fract() == 0.0 && (3.0..=25.0).contains(kind)) {
                continue;
            }
            let value = |want: f32| pairs.iter().find(|(kind, _)| *kind == want).map(|(_, v)| *v);
            params.push((key, value(3.0).map(|v| v * percent), value(5.0)));
        }
    }
    for effect in &mut out {
        if let Some((_, volume, pitch)) = params.iter().find(|(key, ..)| *key == effect.frame) {
            effect.volume = *volume;
            effect.pitch = *pitch;
        }
    }
    out.sort_by(|a, b| a.frame.total_cmp(&b.frame));
    out
}

/// A GOAL string at `p`: its length, then its characters.
fn goal_string(seg: &[u8], p: usize) -> Option<String> {
    let n = u32_at(seg, p)? as usize;
    if n == 0 || n > 64 {
        return None;
    }
    let raw = seg.get(p + 4..p + 4 + n)?;
    raw.iter()
        .all(|c| c.is_ascii_graphic())
        .then(|| String::from_utf8_lossy(raw).into_owned())
}

/// Where a Jak 1 object (v4) links each symbol, by the offset in its data
/// (after the 16-byte header) of the word the symbol goes into. The link
/// block after the data lists each symbol as its name, then the offsets of
/// its uses as differences from the one before: a byte whose low two bits
/// are clear is one difference; with them set, they say how many more bytes
/// (1 to 3) carry its higher bits; a zero ends the list. Type links (their
/// names flagged with a byte over 0x7f before them) are not read.
fn jak1_symbols(bytes: &[u8]) -> HashMap<usize, String> {
    let mut out = HashMap::new();
    let Some(data_len) = u32_at(bytes, 12).map(|n| n as usize) else {
        return out;
    };
    let Some(link) = bytes.get(16 + data_len..) else {
        return out;
    };
    let mut i = 1;
    while i < link.len() {
        if link[i - 1] != 0 || !link[i].is_ascii_lowercase() {
            i += 1;
            continue;
        }
        let end = link[i..]
            .iter()
            .position(|&b| !(b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-' || b == b'?' || b == b'*'))
            .map_or(link.len(), |n| i + n);
        if link.get(end) != Some(&0) || end - i < 2 {
            i = end.max(i + 1);
            continue;
        }
        let name = String::from_utf8_lossy(&link[i..end]).into_owned();
        let (mut at, mut j) = (0usize, end + 1);
        while let Some(&first) = link.get(j) {
            if first == 0 {
                break;
            }
            let extra = usize::from(first & 3);
            let mut delta = usize::from(first & 0xfc);
            for k in 1..=extra {
                delta |= usize::from(*link.get(j + k).unwrap_or(&0)) << (8 * k);
            }
            j += 1 + extra;
            at += delta;
            out.insert(at, name.clone());
        }
        i = j + 1;
    }
    out
}

fn u32_at(bytes: &[u8], at: usize) -> Option<u32> {
    bytes
        .get(at..at + 4)
        .map(|b| u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
}

fn f32_at(bytes: &[u8], at: usize) -> Option<f32> {
    u32_at(bytes, at).map(f32::from_bits)
}

fn joints(seg: &[u8], control: usize) -> Option<Vec<Vec<[f32; 10]>>> {
    let head = u32_at(seg, control)?;
    let frames = (head & 0xffff) as usize;
    let fixed_size = u32_at(seg, control + 4)? as usize * 16;
    let frame_size = u32_at(seg, control + 8)? as usize * 16;
    let at = u32_at(seg, control + 12)? as usize;
    if frames == 0 || frames > 4096 || fixed_size < 80 {
        return None;
    }
    let unpacked;
    let (fixed, blocks): (&[u8], Vec<&[u8]>) = if head & 65536 != 0 {
        let size = fixed_size.checked_add(frames.checked_mul(frame_size)?)?;
        let mut data = lzo1x(seg.get(at..)?, size)?;
        data.resize(size, 0);
        unpacked = data;
        (
            &unpacked[..fixed_size],
            (0..frames)
                .map(|i| &unpacked[fixed_size + i * frame_size..fixed_size + (i + 1) * frame_size])
                .collect(),
        )
    } else {
        let blocks = (0..frames)
            .map(|i| {
                let at = u32_at(seg, control + 16 + i * 4)? as usize;
                seg.get(at..at.checked_add(frame_size)?)
            })
            .collect::<Option<Vec<_>>>()?;
        (seg.get(at..at.checked_add(fixed_size)?)?, blocks)
    };
    let count = u32_at(fixed, 56)? as usize;
    if count == 0 || count > 112 {
        return None;
    }
    let bits = u32_at(fixed, 60)?;
    let mut fixed_offsets = [
        80 + u32_at(fixed, 64)? as usize,
        80 + u32_at(fixed, 68)? as usize,
        80 + u32_at(fixed, 72)? as usize,
    ];
    fixed_offsets[0] += (usize::from(bits & 1 == 0) + usize::from(bits & 2 == 0)) * 64;
    let read =
        |data: &[u8], offsets: &mut [usize; 3], flag: u32, channel: usize| -> Option<Vec<f32>> {
            let mut scalar = |stream: usize, float: bool, divisor: f32| -> Option<f32> {
                let p = &mut offsets[stream];
                let value = if float {
                    let value = f32_at(data, *p)?;
                    *p += 4;
                    value
                } else {
                    let b = data.get(*p..*p + 2)?;
                    *p += 2;
                    f32::from(i16::from_le_bytes([b[0], b[1]]))
                };
                Some(value / divisor)
            };
            Some(match channel {
                0 if flag & 8 != 0 => vec![
                    scalar(0, true, 4096.0)?,
                    scalar(0, true, 4096.0)?,
                    scalar(1, true, 4096.0)?,
                ],
                0 => vec![
                    scalar(1, false, 1024.0)?,
                    scalar(1, false, 1024.0)?,
                    scalar(2, false, 1024.0)?,
                ],
                1 => vec![
                    scalar(0, false, 32768.0)?,
                    scalar(0, false, 32768.0)?,
                    scalar(0, false, 32768.0)?,
                    scalar(0, false, 32768.0)?,
                ],
                _ => vec![
                    scalar(1, false, 4096.0)?,
                    scalar(1, false, 4096.0)?,
                    scalar(2, false, 4096.0)?,
                ],
            })
        };
    let mut base = vec![[0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 1.0, 1.0, 1.0, 1.0]; count];
    for (joint, value) in base.iter_mut().enumerate() {
        let flag = (u32_at(fixed, joint / 8 * 4)? >> (joint % 8 * 4)) & 15;
        for channel in 0..3 {
            if flag & (1 << channel) == 0 {
                let v = read(fixed, &mut fixed_offsets, flag, channel)?;
                let at = [0, 3, 7][channel];
                value[at..at + v.len()].copy_from_slice(&v);
            }
        }
    }
    let mut result = Vec::with_capacity(frames);
    for block in blocks {
        let mut offsets = [
            16 + u32_at(block, 0)? as usize,
            16 + u32_at(block, 4)? as usize,
            16 + u32_at(block, 8)? as usize,
        ];
        offsets[0] += (usize::from(bits & 1 != 0) + usize::from(bits & 2 != 0)) * 64;
        let mut pose = base.clone();
        for (joint, value) in pose.iter_mut().enumerate() {
            let flag = (u32_at(fixed, joint / 8 * 4)? >> (joint % 8 * 4)) & 15;
            for channel in 0..3 {
                if flag & (1 << channel) != 0 {
                    let v = read(block, &mut offsets, flag, channel)?;
                    let at = [0, 3, 7][channel];
                    value[at..at + v.len()].copy_from_slice(&v);
                }
            }
        }
        if pose.iter().flatten().any(|v| !v.is_finite()) {
            return None;
        }
        result.push(pose);
    }
    Some(result)
}

/// The align joint at each frame, and the prejoint's matrices unless they
/// are all the identity, from the compressed animation: one fixed block,
/// then a block per frame, the whole optionally LZO compressed. The two
/// are full matrices, each either in the fixed block (it does not move) or
/// in each frame's block, align first, at the start of the 64-bit data.
#[allow(clippy::type_complexity)]
fn matrices(seg: &[u8], control: usize) -> Option<(Vec<[f32; 7]>, Option<Vec<[f32; 16]>>)> {
    let head = u32_at(seg, control)?;
    let frames = (head & 0xffff) as usize;
    let compressed = (head >> 16) & 1 != 0;
    let fixed_qwc = u32_at(seg, control + 4)? as usize;
    let frame_qwc = u32_at(seg, control + 8)? as usize;
    let fixed_at = u32_at(seg, control + 12)? as usize;
    if frames == 0 || frames > 4096 || fixed_qwc < 5 {
        return None;
    }
    let unpacked;
    let (fixed, frame_blocks): (&[u8], Vec<&[u8]>) = if compressed {
        let size = (fixed_qwc + frames * frame_qwc) * 16;
        let mut packed = lzo1x(seg.get(fixed_at..)?, size)?;
        // The packed stream may stop short of the last frame's padding.
        packed.resize(size, 0);
        unpacked = packed;
        let base = fixed_qwc * 16;
        (
            &unpacked[..base],
            (0..frames)
                .map(|i| &unpacked[base + i * frame_qwc * 16..base + (i + 1) * frame_qwc * 16])
                .collect(),
        )
    } else {
        let fixed = seg.get(fixed_at..fixed_at + fixed_qwc * 16)?;
        let mut blocks = Vec::with_capacity(frames);
        for i in 0..frames {
            let at = u32_at(seg, control + 16 + 4 * i)? as usize;
            blocks.push(seg.get(at..at + frame_qwc * 16)?);
        }
        (fixed, blocks)
    };
    let matrix_bits = u32_at(fixed, 60)?;
    let fixed_64 = u32_at(fixed, 64)? as usize;
    let read = |bytes: &[u8], at: usize| -> Option<[f32; 16]> {
        let m: Vec<f32> = (0..16)
            .map(|k| f32_at(bytes, at + 4 * k))
            .collect::<Option<_>>()?;
        m.try_into().ok()
    };
    let align_varies = matrix_bits & 1 != 0;
    let prejoint_varies = matrix_bits & 2 != 0;
    let fixed_prejoint = if prejoint_varies {
        None
    } else {
        let skip = if align_varies { 0 } else { 64 };
        Some(read(fixed, 80 + fixed_64 + skip)?)
    };
    let mut out = Vec::with_capacity(frames);
    let mut prejoints = Vec::with_capacity(frames);
    for block in frame_blocks {
        let frame_64 = 16 + u32_at(block, 0)? as usize;
        let m = if align_varies {
            read(block, frame_64)?
        } else {
            read(fixed, 80 + fixed_64)?
        };
        prejoints.push(match fixed_prejoint {
            Some(p) => p,
            None => read(block, frame_64 + if align_varies { 64 } else { 0 })?,
        });
        let col = |r: usize| {
            glam::Vec3::new(m[4 * r], m[4 * r + 1], m[4 * r + 2])
                .normalize_or(glam::Vec3::ZERO)
        };
        let rot = glam::Mat3::from_cols(col(0), col(1), col(2));
        let q = glam::Quat::from_mat3(&rot).normalize();
        out.push([m[12], m[13], m[14], q.x, q.y, q.z, q.w]);
    }
    let identity = glam::Mat4::IDENTITY.to_cols_array();
    let moved = prejoints
        .iter()
        .any(|p| p.iter().zip(&identity).any(|(a, b)| (a - b).abs() > 1e-4));
    Some((out, moved.then_some(prejoints)))
}

/// LZO1X decompression, as the game unpacks animations it keeps packed.
fn lzo1x(src: &[u8], out_len: usize) -> Option<Vec<u8>> {
    let mut out: Vec<u8> = Vec::with_capacity(out_len);
    let mut ip = 0usize;
    let next = |ip: &mut usize| -> Option<usize> {
        let b = *src.get(*ip)?;
        *ip += 1;
        Some(usize::from(b))
    };
    let run = |ip: &mut usize, base: usize| -> Option<usize> {
        let mut n = 0;
        while *src.get(*ip)? == 0 {
            n += 255;
            *ip += 1;
        }
        Some(n + base + next(ip)?)
    };
    let mut t = next(&mut ip)?;
    let mut state;
    if t > 17 {
        t -= 17;
        out.extend_from_slice(src.get(ip..ip + t)?);
        ip += t;
        state = t.min(4);
        t = next(&mut ip)?;
    } else {
        state = 0;
    }
    while out.len() < out_len && ip <= src.len() {
        let (len, dist, tail);
        if t >= 64 {
            dist = ((t >> 2) & 7) + (next(&mut ip)? << 3) + 1;
            len = (t >> 5) + 1;
            tail = t & 3;
        } else if t >= 32 {
            let l = if t & 31 == 0 {
                run(&mut ip, 31)?
            } else {
                t & 31
            };
            len = l + 2;
            let d = next(&mut ip)? | (next(&mut ip)? << 8);
            dist = (d >> 2) + 1;
            tail = d & 3;
        } else if t >= 16 {
            let l = if t & 7 == 0 { run(&mut ip, 7)? } else { t & 7 };
            len = l + 2;
            let d = next(&mut ip)? | (next(&mut ip)? << 8);
            let far = ((t & 8) << 11) + (d >> 2);
            if far == 0 {
                break;
            }
            dist = far + 16384;
            tail = d & 3;
        } else if state == 0 {
            let l = if t == 0 { run(&mut ip, 15)? } else { t } + 3;
            out.extend_from_slice(src.get(ip..ip + l)?);
            ip += l;
            state = 4;
            t = next(&mut ip)?;
            continue;
        } else if state == 4 {
            dist = (t >> 2) + (next(&mut ip)? << 2) + 2049;
            len = 3;
            tail = t & 3;
        } else {
            dist = (t >> 2) + (next(&mut ip)? << 2) + 1;
            len = 2;
            tail = t & 3;
        }
        if dist > out.len() {
            return None;
        }
        for _ in 0..len {
            let b = out[out.len() - dist];
            out.push(b);
        }
        out.extend_from_slice(src.get(ip..ip + tail)?);
        ip += tail;
        state = tail;
        t = if ip < src.len() { next(&mut ip)? } else { 0 };
    }
    out.truncate(out_len);
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn export(game: &str) -> Option<std::path::PathBuf> {
        let home = std::env::var_os("USERPROFILE")?;
        let dir = std::path::PathBuf::from(home)
            .join(format!(r"Documents\Jak OpenGOAL\active\{game}\data\decompiler_out\{game}\raw_obj"));
        dir.is_dir().then_some(dir)
    }

    #[test]
    fn jak3s_eyes_are_read_from_its_animations() {
        let Some(root) = export("jak3") else { return };
        let path = root.join("jakb-ag.go");
        if !path.is_file() {
            return;
        }
        let anims = read(&std::fs::read(path).unwrap());
        let mut with = 0;
        for name in ["jakb-stance-loop", "daxter-jakb-stance-loop", "jakb-walk", "daxter-jakb-walk"] {
            if let Some(eyes) = anims.get(name).and_then(|a| a.eyes.as_ref()) {
                println!("{name}: {} frames, first {:?}", eyes.len(), eyes[0]);
                with += 1;
            }
        }
        let daxter: Vec<&String> = anims.keys().filter(|k| k.starts_with("daxter-") && k.contains("stance")).take(6).collect();
        println!("daxter stance anims: {daxter:?}; anims with eyes {}", anims.values().filter(|a| a.eyes.is_some()).count());
        let first = anims.get("jakb-stance-loop").and_then(|a| a.eyes.as_ref()).expect("Jak's stance has its eyes")[0];
        for eye in first {
            assert!((0.2..1.2).contains(&eye.iris_scale) && eye.x.abs() < 0.5 && eye.y.abs() < 0.5, "{eye:?}");
        }
        assert!(with > 0);
    }

    #[test]
    fn jak3s_walk_steps_where_its_art_group_has_them() {
        let Some(dir) = export("jak3") else { return };
        let Ok(bytes) = std::fs::read(dir.join("jakb-ag.go")) else { return };
        let anims = read(&bytes);
        let walk = &anims["jakb-walk"].effects;
        let steps: Vec<_> = walk
            .iter()
            .filter(|e| e.name.contains("step"))
            .map(|e| (e.frame, e.name.as_str(), e.volume))
            .collect();
        assert_eq!(
            steps,
            [(22.0, "effect-walk-step-right", Some(0.7)), (52.0, "effect-walk-step-left", Some(0.7))]
        );
        let land = &anims["jakb-jump-land"].effects;
        assert!(land.iter().any(|e| e.frame == 67.0 && e.name == "effect-land" && e.volume == Some(0.5)));
        let run = &anims["jakb-run"].effects;
        assert!(run.iter().any(|e| e.frame == 26.0 && e.name == "effect-run-step-right" && e.volume.is_none()));
    }

    #[test]
    fn jak1s_ice_skate_steps_through_its_linked_symbols() {
        let Some(dir) = export("jak1") else { return };
        let Ok(bytes) = std::fs::read(dir.join("eichar-ice+0-ag.go")) else { return };
        let anims = read_jak1(&bytes);
        let steps: Vec<_> = anims["eichar-ice-skate"]
            .effects
            .iter()
            .filter(|e| e.name.starts_with("effect-"))
            .map(|e| (e.frame, e.name.as_str()))
            .collect();
        assert_eq!(steps, [(14.0, "effect-walk-step-right"), (45.0, "effect-walk-step-left")]);
        let walk = std::fs::read(dir.join("eichar-ag.go")).map(|b| read_jak1(&b)).unwrap_or_default();
        if let Some(walk) = walk.get("eichar-walk") {
            assert!(walk.effects.iter().any(|e| e.frame == 22.0 && e.name.starts_with("effect-walk-step") && e.volume == Some(0.7)));
        }
    }
}
