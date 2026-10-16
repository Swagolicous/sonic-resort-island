//! Flat name -> bytes resource database built from AR/ARL archive sets and
//! packed (PFD) terrain groups, mirroring how the engine resolves resources
//! by file name regardless of which archive they came from.

use std::{collections::HashMap, fs, path::Path, sync::Arc};

use anyhow::{Context, Result};

use crate::{ar, xcompress};

#[derive(Default, Clone)]
pub struct ResourceDb {
    files: HashMap<String, Arc<[u8]>>,
}

impl ResourceDb {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn insert(&mut self, name: impl Into<String>, data: impl Into<Arc<[u8]>>) {
        self.files.insert(name.into(), data.into());
    }

    pub fn get(&self, name: &str) -> Option<&[u8]> {
        self.files.get(name).map(|d| &**d)
    }

    pub fn contains(&self, name: &str) -> bool {
        self.files.contains_key(name)
    }

    pub fn len(&self) -> usize {
        self.files.len()
    }

    pub fn is_empty(&self) -> bool {
        self.files.is_empty()
    }

    pub fn names(&self) -> impl Iterator<Item = &str> {
        self.files.keys().map(String::as_str)
    }

    /// Loads every `.ar.NN` part listed by `<stem>.arl`. Later loads override
    /// earlier entries with the same name. Returns the number of entries.
    pub fn load_arl(&mut self, dir: &Path, stem: &str) -> Result<usize> {
        let arl_path = dir.join(format!("{stem}.arl"));
        let arl_bytes = read_maybe_compressed(&arl_path)?;
        let arl = ar::parse_archive_list(&arl_bytes)
            .with_context(|| format!("parse {}", arl_path.display()))?;
        let mut count = 0;
        for part in 0..arl.archive_sizes.len() {
            let path = dir.join(format!("{stem}.ar.{part:02}"));
            count += self.load_ar_bytes(&read_maybe_compressed(&path)?)
                .with_context(|| format!("parse {}", path.display()))?;
        }
        Ok(count)
    }

    pub fn load_ar_bytes(&mut self, bytes: &[u8]) -> Result<usize> {
        let entries = ar::parse_archive_ref(bytes)?;
        for entry in &entries {
            self.insert(entry.name, entry.data);
        }
        Ok(entries.len())
    }

    /// Loads the inner archives of a packed stage file whose names match
    /// `filter` (e.g. terrain groups `tg-*.ar`). Returns loaded group names.
    pub fn load_pfd(&mut self, pfd_path: &Path, filter: impl Fn(&str) -> bool) -> Result<Vec<String>> {
        let mut loaded = Vec::new();
        for (name, inner) in pfd_records(pfd_path, filter)? {
            self.load_ar_bytes(&inner)
                .with_context(|| format!("parse inner archive {name}"))?;
            loaded.push(name);
        }
        Ok(loaded)
    }
}

/// Decompressed inner archives of a `Stage.pfd`, by record name. Stage.pfd is
/// an ordinary 0x800-aligned AR whose records hold XCompress streams
/// (preceded by a small prefix); all-zero records are skipped.
pub fn pfd_records(pfd_path: &Path, filter: impl Fn(&str) -> bool) -> Result<Vec<(String, Vec<u8>)>> {
    let bytes = fs::read(pfd_path).with_context(|| format!("read {}", pfd_path.display()))?;
    let entries = ar::parse_archive_ref(&bytes)?;
    let mut out = Vec::new();
    for entry in &entries {
        if !filter(entry.name) {
            continue;
        }
        let signature = xcompress::XCOMPRESS_SIGNATURE.to_be_bytes();
        let start = entry
            .data
            .windows(4)
            .take(0x40)
            .position(|w| w == signature)
            .with_context(|| format!("{} has no XCompress stream", entry.name))?;
        // The record's data size excludes the prefix; the stream may run
        // past the AR payload into the record padding.
        let record_start = entry.data.as_ptr() as usize - bytes.as_ptr() as usize + start;
        let compressed = &bytes[record_start..];
        let inner = xcompress::decompress(compressed)
            .with_context(|| format!("decompress {}", entry.name))?;
        if inner.iter().all(|b| *b == 0) {
            continue;
        }
        out.push((entry.name.to_owned(), trim_zero_padding(&inner).to_vec()));
    }
    Ok(out)
}

pub fn read_maybe_compressed(path: &Path) -> Result<Vec<u8>> {
    let bytes = fs::read(path).with_context(|| format!("read {}", path.display()))?;
    decompress_if_needed(bytes)
}

pub fn decompress_if_needed(bytes: Vec<u8>) -> Result<Vec<u8>> {
    if xcompress::XCompressHeader::parse(&bytes).is_ok() {
        xcompress::decompress(&bytes)
    } else {
        Ok(bytes)
    }
}

/// Inner terrain archives are zero-padded to their packed record size.
fn trim_zero_padding(bytes: &[u8]) -> &[u8] {
    let mut cursor = ar::AR_HEADER_SIZE;
    while cursor + ar::AR_ENTRY_HEADER_SIZE <= bytes.len() {
        let span = u32::from_le_bytes(bytes[cursor..cursor + 4].try_into().unwrap()) as usize;
        if span == 0 || cursor + span > bytes.len() {
            break;
        }
        cursor += span;
    }
    &bytes[..cursor.min(bytes.len())]
}
