//! Dump decompressed archive contents for format inspection.
//!
//! usage:
//!   extract arl <game_dir> <stem> <out_dir>      e.g. stem = "#ActD_MykonosAct1"
//!   extract pfd <pfd_path> <out_dir>             dumps every packed record's inner AR
//!   extract ar <ar_path> <out_dir>               one (optionally XCompressed) AR file

use anyhow::{Context, Result, bail};
use std::{fs, path::Path};
use unleashed_assets::{ar, packed, xcompress};

fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args.iter().map(String::as_str).collect::<Vec<_>>().as_slice() {
        ["arl", game_dir, stem, out] => extract_arl(Path::new(game_dir), stem, Path::new(out)),
        ["pfd", pfd, out] => extract_pfd(Path::new(pfd), Path::new(out)),
        ["ar", file, out] => {
            let bytes = maybe_decompress(fs::read(file)?)?;
            println!("{file}: {} entries", write_entries(&bytes, Path::new(out))?);
            Ok(())
        }
        _ => bail!("usage: extract arl <game_dir> <stem> <out> | extract pfd <pfd> <out> | extract ar <file> <out>"),
    }
}

fn maybe_decompress(bytes: Vec<u8>) -> Result<Vec<u8>> {
    if xcompress::XCompressHeader::parse(&bytes).is_ok() {
        xcompress::decompress(&bytes)
    } else {
        Ok(bytes)
    }
}

fn write_entries(archive: &[u8], out: &Path) -> Result<usize> {
    let entries = ar::parse_archive_ref(archive)?;
    fs::create_dir_all(out)?;
    for entry in &entries {
        fs::write(out.join(entry.name), entry.data)
            .with_context(|| format!("write {}", entry.name))?;
    }
    Ok(entries.len())
}

fn extract_arl(game_dir: &Path, stem: &str, out: &Path) -> Result<()> {
    let arl_bytes = maybe_decompress(fs::read(game_dir.join(format!("{stem}.arl")))?)?;
    let arl = ar::parse_archive_list(&arl_bytes)?;
    let mut total = 0;
    for part in 0..arl.archive_sizes.len() {
        let path = game_dir.join(format!("{stem}.ar.{part:02}"));
        let bytes = maybe_decompress(fs::read(&path).with_context(|| path.display().to_string())?)?;
        total += write_entries(&bytes, &out.join(stem.trim_start_matches('#')))?;
    }
    println!("{stem}: {total} entries");
    Ok(())
}

fn extract_pfd(pfd_path: &Path, out: &Path) -> Result<()> {
    let bytes = fs::read(pfd_path)?;
    let index = packed::parse_pfd(&bytes)?;
    for entry in index.entries() {
        let start = entry.payload_offset + 0x10;
        let compressed = &bytes[start..start + entry.header.data_size as usize];
        let inner = xcompress::decompress(compressed)?;
        if inner.iter().all(|b| *b == 0) {
            continue;
        }
        let logical = trim_zero_padding(&inner);
        match write_entries(logical, &out.join(entry.name)) {
            Ok(n) => println!("{}: {n} entries", entry.name),
            Err(e) => println!("{}: not an AR ({e})", entry.name),
        }
    }
    Ok(())
}

/// Inner archives are zero-padded to the packed record size.
fn trim_zero_padding(bytes: &[u8]) -> &[u8] {
    let mut cursor = 0x10;
    while cursor + 0x14 <= bytes.len() {
        let span = u32::from_le_bytes(bytes[cursor..cursor + 4].try_into().unwrap()) as usize;
        if span == 0 || cursor + span > bytes.len() {
            break;
        }
        cursor += span;
    }
    &bytes[..cursor.min(bytes.len())]
}
