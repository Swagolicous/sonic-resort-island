//! Baked global illumination ("GI atlases").
//!
//! `Stage.pfd` holds `gia-NNN.ar` records; each contains atlas textures
//! (`a0000.dds`, DXT5) and an `atlasinfo` table placing every terrain
//! instance's GI tile inside an atlas. The terrain vertex shader maps the
//! instance's TEXCOORD1 into the atlas with `mrgGIAtlasParam`
//! (`uv * param.xy + param.zw`); the pixel shader reads RGB as light (stored
//! as sqrt, squared on use) and A as baked sun visibility.
//!
//! `atlasinfo` layout (read by sub_824F???? in default.xex; note the
//! little-endian counts in an otherwise big-endian game):
//! ```text
//! u8 version (0), u16le atlas_count
//! per atlas:   u8 len, name, u16le entry_count
//! per entry:   u8 len, name ("<instance>-level2"), u8 log2_w, u8 log2_h, u8 x, u8 y
//! ```
//! with scale = 1 / 2^log2 and offset = byte / 256.

use std::{collections::HashMap, path::Path, sync::Arc};

use anyhow::{Context, Result, bail, ensure};

use crate::{ar, resources};

#[derive(Clone, Debug, PartialEq)]
pub struct AtlasEntry {
    /// Index into [`GiAtlases::textures`].
    pub texture: usize,
    /// `mrgGIAtlasParam`: (scale u, scale v, offset u, offset v).
    pub param: [f32; 4],
    /// GI level suffix (`-level2` is the full-resolution set).
    pub level: u8,
}

#[derive(Default)]
pub struct GiAtlases {
    /// (`gia-NNN/aNNNN`, DDS bytes)
    pub textures: Vec<(String, Arc<[u8]>)>,
    /// Terrain instance name -> best (highest level) tile.
    pub entries: HashMap<String, AtlasEntry>,
}

/// One parsed `atlasinfo`: (atlas name, [(entry name, param)]).
pub fn parse_atlasinfo(b: &[u8]) -> Result<Vec<(String, Vec<(String, [f32; 4])>)>> {
    let mut off = 0usize;
    let mut take = |n: usize| -> Result<&[u8]> {
        ensure!(off + n <= b.len(), "atlasinfo truncated at {off}");
        let s = &b[off..off + n];
        off += n;
        Ok(s)
    };
    if take(1)?[0] != 0 {
        bail!("unknown atlasinfo version");
    }
    let count = |s: &[u8]| u16::from_le_bytes([s[0], s[1]]) as usize;
    let atlases = count(take(2)?);
    let mut out = Vec::with_capacity(atlases);
    for _ in 0..atlases {
        let len = take(1)?[0] as usize;
        let name = String::from_utf8_lossy(take(len)?).into_owned();
        let n = count(take(2)?);
        let mut entries = Vec::with_capacity(n);
        for _ in 0..n {
            let len = take(1)?[0] as usize;
            let entry = String::from_utf8_lossy(take(len)?).into_owned();
            let p = take(4)?;
            let param = [
                1.0 / (1u32 << p[0]) as f32,
                1.0 / (1u32 << p[1]) as f32,
                p[2] as f32 / 256.0,
                p[3] as f32 / 256.0,
            ];
            entries.push((entry, param));
        }
        out.push((name, entries));
    }
    ensure!(off == b.len(), "atlasinfo has {} trailing bytes", b.len() - off);
    Ok(out)
}

/// Loads every `gia-*` record of a stage's `Stage.pfd`.
pub fn load_gi_atlases(pfd_path: &Path) -> Result<GiAtlases> {
    let mut out = GiAtlases::default();
    for (record, inner) in resources::pfd_records(pfd_path, |n| n.starts_with("gia-"))? {
        let files = ar::parse_archive_ref(&inner).with_context(|| format!("{record}: inner archive"))?;
        let stem = record.trim_end_matches(".ar");
        let info = files.iter().find(|f| f.name == "atlasinfo").with_context(|| format!("{record}: no atlasinfo"))?;
        for (atlas, entries) in parse_atlasinfo(info.data).with_context(|| format!("{record}/atlasinfo"))? {
            let file = format!("{atlas}.dds");
            let Some(dds) = files.iter().find(|f| f.name == file) else {
                bail!("{record}: atlasinfo names missing {file}");
            };
            let texture = out.textures.len();
            out.textures.push((format!("{stem}/{atlas}"), Arc::from(dds.data)));
            for (name, param) in entries {
                let (instance, level) = match name.rsplit_once("-level") {
                    Some((i, l)) => (i.to_owned(), l.parse().unwrap_or(0)),
                    None => (name, 0),
                };
                let keep = out.entries.get(&instance).is_none_or(|e| e.level < level);
                if keep {
                    out.entries.insert(instance, AtlasEntry { texture, param, level });
                }
            }
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_atlasinfo() {
        let mut b = vec![0u8, 1, 0];
        b.push(5);
        b.extend_from_slice(b"a0000");
        b.extend_from_slice(&[2, 0]);
        for (name, p) in [("rock-level2", [2u8, 2, 0, 0]), ("tree-level2", [4, 4, 0xF0, 0x20])] {
            b.push(name.len() as u8);
            b.extend_from_slice(name.as_bytes());
            b.extend_from_slice(&p);
        }
        let a = parse_atlasinfo(&b).unwrap();
        assert_eq!(a[0].0, "a0000");
        assert_eq!(a[0].1[0], ("rock-level2".into(), [0.25, 0.25, 0.0, 0.0]));
        assert_eq!(a[0].1[1], ("tree-level2".into(), [0.0625, 0.0625, 0.9375, 0.125]));
    }
}
