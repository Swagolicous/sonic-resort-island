//! Sega "CSD" UI projects (`.yncp`), the format of Sonic Unleashed's HUD.
//!
//! Container: `CPAF` + resource 0 (`NYIF` info, `nCPJ` project chunk, `NOF0`
//! pointer table, `NEND`) + resource 1 (`NYIF`, `NXTL` texture list, ...).
//! Chunk sizes are little-endian; chunk payloads are big-endian with offsets
//! relative to the chunk start (validated: all 4,189 `NOF0` pointers of
//! `ui_playscreen.yncp` resolve inside the chunk with that base).
//!
//! ```text
//! project:    root_node*, name*, "DXL", font_list*
//! scene node: scene_count, scenes**, scene_names*{name*, index},
//!             node_count, nodes*(24 B each), node_names*{name*, index}
//! scene:      version, z, fps, ?, ?, texture_count, texture_sizes*, subimage_count,
//!             subimages*{texture, tl[2], br[2]}, cast_node_count, cast_nodes*,
//!             cast_count, cast_names*{name*, node, index}, anim_count,
//!             anim_keys*, anim_names*{name*, index}, aspect, anim_frames*{start, end}
//! cast node:  cast_count, casts**, root_index, links*{child, sibling}
//! cast:       field00, kind (0 null, 1 sprite, 2 font), enabled, tl, bl, tr, br,
//!             flags2c, info*, inherit, flags38, subimage_count, subimages*[i32],
//!             font_chars*, font_name*, spacing, width, height, ...
//! cast info:  hide, translation[2], rotation, scale[2], subimage, color, gradient[4]
//! anim keys:  per anim {node_count, nodes*} -> per node {cast_count, casts*}
//!             -> per cast {property_mask, tracks*} -> per set bit
//!             {flags, key_count, keys*{frame, value, interpolation, in, out, ?}}
//! font list:  count, fonts*{char_count, map*{char, subimage}}, names*{name*, index}
//! ```

use std::collections::HashMap;

use anyhow::{Context, Result, bail};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SubImage {
    pub texture: u32,
    pub top_left: [f32; 2],
    pub bottom_right: [f32; 2],
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CastInfo {
    pub hide: bool,
    pub translation: [f32; 2],
    pub rotation: f32,
    pub scale: [f32; 2],
    pub sub_image: f32,
    /// RGBA, 0xRRGGBBAA.
    pub color: u32,
    /// Corner colours: top-left, bottom-left, top-right, bottom-right.
    pub gradient: [u32; 4],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CastKind {
    Null,
    Sprite,
    Font,
}

#[derive(Debug, Clone)]
pub struct Cast {
    pub name: String,
    pub parent: Option<usize>,
    pub kind: CastKind,
    pub enabled: bool,
    /// Quad corners relative to the cast origin: tl, bl, tr, br (normalized screen units).
    pub corners: [[f32; 2]; 4],
    pub flags: [u32; 3],
    pub sub_images: Vec<i32>,
    pub font: Option<String>,
    pub text: Option<String>,
    pub spacing: f32,
    pub size: [u32; 2],
    pub info: CastInfo,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Property {
    Hide,
    X,
    Y,
    Rotation,
    ScaleX,
    ScaleY,
    SubImage,
    Color,
    GradientTL,
    GradientBL,
    GradientTR,
    GradientBR,
    Unknown(u32),
}

impl Property {
    fn from_bit(bit: u32) -> Self {
        match bit {
            0 => Self::Hide,
            1 => Self::X,
            2 => Self::Y,
            3 => Self::Rotation,
            4 => Self::ScaleX,
            5 => Self::ScaleY,
            6 => Self::SubImage,
            7 => Self::Color,
            8 => Self::GradientTL,
            9 => Self::GradientBL,
            10 => Self::GradientTR,
            11 => Self::GradientBR,
            other => Self::Unknown(other),
        }
    }

    /// Colour tracks store packed RGBA in the value's bit pattern.
    pub fn is_color(self) -> bool {
        matches!(
            self,
            Self::Color | Self::GradientTL | Self::GradientBL | Self::GradientTR | Self::GradientBR
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Key {
    pub frame: f32,
    pub value: f32,
    /// 0 = constant, 1 = linear, 2 = hermite.
    pub interpolation: u32,
    pub in_tangent: f32,
    pub out_tangent: f32,
}

#[derive(Debug, Clone)]
pub struct Track {
    pub cast: usize,
    pub property: Property,
    pub keys: Vec<Key>,
}

impl Track {
    pub fn sample(&self, frame: f32) -> f32 {
        let keys = &self.keys;
        if keys.is_empty() {
            return 0.0;
        }
        if frame <= keys[0].frame {
            return keys[0].value;
        }
        for pair in keys.windows(2) {
            let (a, b) = (pair[0], pair[1]);
            if frame < b.frame {
                if self.property.is_color() {
                    let t = (frame - a.frame) / (b.frame - a.frame).max(1e-6);
                    return f32::from_bits(lerp_rgba(a.value.to_bits(), b.value.to_bits(), t));
                }
                return match a.interpolation {
                    0 => a.value,
                    2 => {
                        let span = (b.frame - a.frame).max(1e-6);
                        let t = (frame - a.frame) / span;
                        let (t2, t3) = (t * t, t * t * t);
                        a.value * (2.0 * t3 - 3.0 * t2 + 1.0)
                            + a.out_tangent * span * (t3 - 2.0 * t2 + t)
                            + b.value * (-2.0 * t3 + 3.0 * t2)
                            + b.in_tangent * span * (t3 - t2)
                    }
                    _ => {
                        let t = (frame - a.frame) / (b.frame - a.frame).max(1e-6);
                        a.value + (b.value - a.value) * t
                    }
                };
            }
        }
        keys[keys.len() - 1].value
    }
}

fn lerp_rgba(a: u32, b: u32, t: f32) -> u32 {
    let mut out = 0;
    for shift in [24, 16, 8, 0] {
        let ca = ((a >> shift) & 0xFF) as f32;
        let cb = ((b >> shift) & 0xFF) as f32;
        out |= ((ca + (cb - ca) * t).round().clamp(0.0, 255.0) as u32) << shift;
    }
    out
}

#[derive(Debug, Clone)]
pub struct Animation {
    pub name: String,
    pub start: f32,
    pub end: f32,
    pub tracks: Vec<Track>,
}

#[derive(Debug, Clone)]
pub struct Scene {
    /// Path from the project root, e.g. `ring_count` or `add/u_info`.
    pub name: String,
    pub z_index: f32,
    pub fps: f32,
    pub aspect: f32,
    pub texture_sizes: Vec<[f32; 2]>,
    pub sub_images: Vec<SubImage>,
    /// All casts, parents before children.
    pub casts: Vec<Cast>,
    pub animations: Vec<Animation>,
}

impl Scene {
    pub fn cast(&self, name: &str) -> Option<usize> {
        self.casts.iter().position(|c| c.name == name)
    }

    pub fn animation(&self, name: &str) -> Option<&Animation> {
        self.animations.iter().find(|a| a.name == name)
    }
}

#[derive(Debug, Clone)]
pub struct Project {
    pub name: String,
    pub scenes: Vec<Scene>,
    /// font name -> (character, sub image index)
    pub fonts: HashMap<String, Vec<(char, u32)>>,
    pub textures: Vec<String>,
}

impl Project {
    pub fn scene(&self, name: &str) -> Option<&Scene> {
        self.scenes.iter().find(|s| s.name == name)
    }
}

struct Chunk<'a> {
    bytes: &'a [u8],
    base: usize,
}

impl<'a> Chunk<'a> {
    fn u32(&self, at: u32) -> Result<u32> {
        let o = self.base + at as usize;
        let s = self.bytes.get(o..o + 4).context("CSD read out of bounds")?;
        Ok(u32::from_be_bytes(s.try_into().unwrap()))
    }

    fn i32(&self, at: u32) -> Result<i32> {
        Ok(self.u32(at)? as i32)
    }

    fn f32(&self, at: u32) -> Result<f32> {
        Ok(f32::from_bits(self.u32(at)?))
    }

    fn string(&self, at: u32) -> Result<String> {
        let o = self.base + at as usize;
        let tail = self.bytes.get(o..).context("CSD string out of bounds")?;
        let end = tail.iter().position(|b| *b == 0).context("unterminated CSD string")?;
        Ok(String::from_utf8_lossy(&tail[..end]).into_owned())
    }

    fn string_ptr(&self, at: u32) -> Result<Option<String>> {
        match self.u32(at)? {
            0 => Ok(None),
            p => self.string(p).map(Some),
        }
    }
}

fn find_chunk(bytes: &[u8], tag: &[u8; 4], from: usize) -> Option<usize> {
    bytes[from..]
        .windows(4)
        .position(|w| w == tag)
        .map(|p| p + from)
}

pub fn parse_project(bytes: &[u8]) -> Result<Project> {
    if bytes.get(0..4) != Some(b"CPAF") {
        bail!("not a CSD (CPAF) file");
    }
    let project_at = find_chunk(bytes, b"nCPJ", 0)
        .or_else(|| find_chunk(bytes, b"NCPJ", 0))
        .context("no project chunk")?;
    let c = Chunk {
        bytes,
        base: project_at,
    };
    let project = c.u32(8)?;
    let root = c.u32(project)?;
    let name = c.string_ptr(project + 4)?.unwrap_or_default();

    let mut scenes = Vec::new();
    walk_node(&c, root, "", &mut scenes)?;

    let mut fonts = HashMap::new();
    if c.u32(project + 8)? == u32::from_be_bytes(*b".DXL") || c.u32(project + 12)? != 0 {
        let list = c.u32(project + 12)?;
        if list != 0 {
            let count = c.u32(list)?;
            let table = c.u32(list + 4)?;
            let names = c.u32(list + 8)?;
            for k in 0..count {
                let font_name = c.string(c.u32(names + 8 * k)?)?;
                let index = c.u32(names + 8 * k + 4)?;
                let font = table + 8 * index;
                let chars = c.u32(font)?;
                let map = c.u32(font + 4)?;
                let mut mapping = Vec::with_capacity(chars as usize);
                for j in 0..chars {
                    let code = c.u32(map + 8 * j)?;
                    let sub = c.u32(map + 8 * j + 4)?;
                    if let Some(ch) = char::from_u32(code) {
                        mapping.push((ch, sub));
                    }
                }
                fonts.insert(font_name, mapping);
            }
        }
    }

    let textures = parse_texture_list(bytes).unwrap_or_default();
    Ok(Project {
        name,
        scenes,
        fonts,
        textures,
    })
}

fn parse_texture_list(bytes: &[u8]) -> Result<Vec<String>> {
    let at = find_chunk(bytes, b"NXTL", 0).context("no texture list")?;
    let c = Chunk { bytes, base: at };
    let list = c.u32(8)?;
    let count = c.u32(list)?;
    let entries = c.u32(list + 4)?;
    (0..count)
        .map(|k| c.string(c.u32(entries + 8 * k)?))
        .collect()
}

fn walk_node(c: &Chunk<'_>, node: u32, path: &str, out: &mut Vec<Scene>) -> Result<()> {
    let scene_count = c.u32(node)?;
    let scenes = c.u32(node + 4)?;
    let scene_names = c.u32(node + 8)?;
    for k in 0..scene_count {
        let name = c.string(c.u32(scene_names + 8 * k)?)?;
        let index = c.u32(scene_names + 8 * k + 4)?;
        let at = c.u32(scenes + 4 * index)?;
        out.push(parse_scene(c, at, format!("{path}{name}")).with_context(|| format!("scene {path}{name}"))?);
    }
    let node_count = c.u32(node + 12)?;
    let nodes = c.u32(node + 16)?;
    let node_names = c.u32(node + 20)?;
    for k in 0..node_count {
        let name = c.string(c.u32(node_names + 8 * k)?)?;
        let index = c.u32(node_names + 8 * k + 4)?;
        walk_node(c, nodes + 24 * index, &format!("{path}{name}/"), out)?;
    }
    Ok(())
}

fn parse_scene(c: &Chunk<'_>, at: u32, name: String) -> Result<Scene> {
    let texture_count = c.u32(at + 0x14)?;
    let textures = c.u32(at + 0x18)?;
    let texture_sizes = (0..texture_count)
        .map(|k| Ok([c.f32(textures + 8 * k)?, c.f32(textures + 8 * k + 4)?]))
        .collect::<Result<_>>()?;
    let sub_count = c.u32(at + 0x1C)?;
    let subs = c.u32(at + 0x20)?;
    let sub_images = (0..sub_count)
        .map(|k| {
            let s = subs + 20 * k;
            Ok(SubImage {
                texture: c.u32(s)?,
                top_left: [c.f32(s + 4)?, c.f32(s + 8)?],
                bottom_right: [c.f32(s + 12)?, c.f32(s + 16)?],
            })
        })
        .collect::<Result<_>>()?;

    let node_count = c.u32(at + 0x24)?;
    let nodes = c.u32(at + 0x28)?;
    let cast_count = c.u32(at + 0x2C)?;
    let cast_names = c.u32(at + 0x30)?;
    let mut names = HashMap::new();
    for k in 0..cast_count {
        let e = cast_names + 12 * k;
        names.insert((c.u32(e + 4)?, c.u32(e + 8)?), c.string(c.u32(e)?)?);
    }

    // Flatten each cast node's tree (child/sibling links) parents-first.
    let mut casts = Vec::new();
    let mut index_of = HashMap::new();
    for n in 0..node_count {
        let node = nodes + 16 * n;
        let count = c.u32(node)?;
        let table = c.u32(node + 4)?;
        let root = c.i32(node + 8)?;
        let links = c.u32(node + 12)?;
        let mut stack = vec![(root, None::<usize>)];
        let mut visited = 0;
        while let Some((index, parent)) = stack.pop() {
            if index < 0 || index as u32 >= count || visited > count {
                continue;
            }
            visited += 1;
            let i = index as u32;
            let cast_at = c.u32(table + 4 * i)?;
            let flat = casts.len();
            casts.push(parse_cast(c, cast_at, names.get(&(n, i)).cloned().unwrap_or_default(), parent)?);
            index_of.insert((n, i), flat);
            let child = c.i32(links + 8 * i)?;
            let sibling = c.i32(links + 8 * i + 4)?;
            stack.push((sibling, parent));
            stack.push((child, Some(flat)));
        }
    }

    let anim_count = c.u32(at + 0x34)?;
    let keys = c.u32(at + 0x38)?;
    let anim_names = c.u32(at + 0x3C)?;
    let frames = c.u32(at + 0x44)?;
    let mut animations = Vec::new();
    for k in 0..anim_count {
        let anim_name = c.string(c.u32(anim_names + 8 * k)?)?;
        let index = c.u32(anim_names + 8 * k + 4)?;
        let (start, end) = if frames != 0 {
            (c.f32(frames + 8 * index)?, c.f32(frames + 8 * index + 4)?)
        } else {
            (0.0, 0.0)
        };
        let entry = keys + 8 * index;
        let anim_nodes = c.u32(entry)?;
        let anim_table = c.u32(entry + 4)?;
        let mut tracks = Vec::new();
        for n in 0..anim_nodes {
            let node = anim_table + 8 * n;
            let node_casts = c.u32(node)?;
            let cast_table = c.u32(node + 4)?;
            for ci in 0..node_casts {
                let cast_entry = cast_table + 8 * ci;
                let mask = c.u32(cast_entry)?;
                let track_table = c.u32(cast_entry + 4)?;
                if mask == 0 || track_table == 0 {
                    continue;
                }
                let Some(&cast) = index_of.get(&(n, ci)) else { continue };
                let mut slot = 0;
                for bit in 0..32 {
                    if mask & (1 << bit) == 0 {
                        continue;
                    }
                    let t = track_table + 12 * slot;
                    slot += 1;
                    let key_count = c.u32(t + 4)?;
                    let key_at = c.u32(t + 8)?;
                    let keys = (0..key_count)
                        .map(|j| {
                            let o = key_at + 24 * j;
                            Ok(Key {
                                frame: c.u32(o)? as f32,
                                value: c.f32(o + 4)?,
                                interpolation: c.u32(o + 8)?,
                                in_tangent: c.f32(o + 12)?,
                                out_tangent: c.f32(o + 16)?,
                            })
                        })
                        .collect::<Result<_>>()?;
                    tracks.push(Track {
                        cast,
                        property: Property::from_bit(bit),
                        keys,
                    });
                }
            }
        }
        animations.push(Animation {
            name: anim_name,
            start,
            end,
            tracks,
        });
    }

    Ok(Scene {
        name,
        z_index: c.f32(at + 4)?,
        fps: c.f32(at + 8)?,
        aspect: c.f32(at + 0x40)?,
        texture_sizes,
        sub_images,
        casts,
        animations,
    })
}

fn parse_cast(c: &Chunk<'_>, at: u32, name: String, parent: Option<usize>) -> Result<Cast> {
    let corner = |o: u32| -> Result<[f32; 2]> { Ok([c.f32(at + o)?, c.f32(at + o + 4)?]) };
    let info_at = c.u32(at + 0x30)?;
    let info = if info_at != 0 {
        let g = |o: u32| c.u32(info_at + o);
        CastInfo {
            hide: g(0)? != 0,
            translation: [c.f32(info_at + 4)?, c.f32(info_at + 8)?],
            rotation: c.f32(info_at + 12)?,
            scale: [c.f32(info_at + 16)?, c.f32(info_at + 20)?],
            sub_image: c.f32(info_at + 24)?,
            color: g(28)?,
            gradient: [g(32)?, g(36)?, g(40)?, g(44)?],
        }
    } else {
        CastInfo {
            hide: false,
            translation: [0.0; 2],
            rotation: 0.0,
            scale: [1.0; 2],
            sub_image: -1.0,
            color: 0xFFFF_FFFF,
            gradient: [0xFFFF_FFFF; 4],
        }
    };
    let sub_count = c.u32(at + 0x3C)?;
    let subs_at = c.u32(at + 0x40)?;
    let sub_images = if subs_at != 0 {
        (0..sub_count.min(64))
            .map(|k| c.i32(subs_at + 4 * k))
            .collect::<Result<_>>()?
    } else {
        Vec::new()
    };
    Ok(Cast {
        name,
        parent,
        kind: match c.u32(at + 4)? {
            1 => CastKind::Sprite,
            2 => CastKind::Font,
            _ => CastKind::Null,
        },
        enabled: c.u32(at + 8)? != 0,
        corners: [corner(0x0C)?, corner(0x14)?, corner(0x1C)?, corner(0x24)?],
        flags: [c.u32(at + 0x2C)?, c.u32(at + 0x34)?, c.u32(at + 0x38)?],
        sub_images,
        text: c.string_ptr(at + 0x44)?,
        font: c.string_ptr(at + 0x48)?,
        spacing: c.f32(at + 0x4C)?,
        size: [c.u32(at + 0x50)?, c.u32(at + 0x54)?],
        info,
    })
}
