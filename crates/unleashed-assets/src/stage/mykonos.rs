use anyhow::{Context, Result, bail};
use std::{
    collections::{HashMap, HashSet},
    fs,
    path::{Path, PathBuf},
};

use crate::{
    ar::{ArchiveEntryRef, parse_archive_list, parse_archive_ref},
    mirage::{
        InstanceTransform, TerrainMesh, parse_terrain_instance_transform, parse_terrain_mesh,
    },
    packed::{PackedEntry, parse_pfd},
    xcompress,
};

const CONTROL_ARL: &str = "#ActD_MykonosAct1.arl";
const CONTROL_AR_PART: &str = "#ActD_MykonosAct1.ar.00";
const STAGE_PFI_ENTRY: &str = "Stage.pfi";
const STAGE_PFD_RELATIVE: &str = "Packed/ActD_MykonosAct1/Stage.pfd";

const PFI_RECORD_STRIDE: usize = 0x18;
const PFI_NAME_BYTES: usize = 12;
const PACKED_XCOMPRESS_PREFIX: usize = 0x10;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiagnosticSeverity {
    Info,
    Warning,
    Error,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StageDiagnostic {
    pub severity: DiagnosticSeverity,
    pub scope: String,
    pub message: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct StageCounts {
    pub control_entries: usize,
    pub pfi_entries: usize,
    pub pfd_entries: usize,
    pub terrain_groups_total: usize,
    pub terrain_groups_loaded: usize,
    pub terrain_groups_empty: usize,
    pub terrain_groups_skipped: usize,
    pub terrain_models: usize,
    pub terrain_instances: usize,
    pub resources_skipped: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TerrainModelResource {
    pub group_name: String,
    pub resource_name: String,
    pub mesh: TerrainMesh,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TerrainInstanceResource {
    pub group_name: String,
    pub resource_name: String,
    pub transform: InstanceTransform,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ActDMykonosAct1 {
    pub game_dir: PathBuf,
    pub models: Vec<TerrainModelResource>,
    pub instances: Vec<TerrainInstanceResource>,
    pub counts: StageCounts,
    pub diagnostics: Vec<StageDiagnostic>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct PfiRecord {
    name: String,
    /// Observed Stage.pfi value. For this stage it points at the XCompress
    /// signature, 0x10 bytes after the start of the PFD entry payload.
    xcompress_offset: usize,
    /// Observed Stage.pfi byte count. Together with `xcompress_offset`, this is
    /// the authoritative raw Stage.pfd slice consumed by XCompress.
    packed_size: usize,
}

fn diagnostic(
    diagnostics: &mut Vec<StageDiagnostic>,
    severity: DiagnosticSeverity,
    scope: impl Into<String>,
    message: impl Into<String>,
) {
    diagnostics.push(StageDiagnostic {
        severity,
        scope: scope.into(),
        message: message.into(),
    });
}

/// Load the evidenced Windmill Isle Act 1 streamed terrain path.
///
/// `root` may be either the install root containing `game/`, or the `game/`
/// directory itself. Required source paths come from `re/windmill-isle.json`.
/// Unsupported/corrupt individual terrain-group records are skipped with
/// structured diagnostics; missing authoritative stage inputs are fatal.
pub fn load_actd_mykonos_act1(root: impl AsRef<Path>) -> Result<ActDMykonosAct1> {
    let game_dir = resolve_game_dir(root.as_ref())?;
    let mut diagnostics = Vec::new();
    let mut counts = StageCounts::default();

    let arl_path = game_dir.join(CONTROL_ARL);
    let arl_compressed =
        fs::read(&arl_path).with_context(|| format!("read {}", arl_path.display()))?;
    let arl_bytes = xcompress::decompress(&arl_compressed)
        .with_context(|| format!("decompress {}", arl_path.display()))?;
    let arl = parse_archive_list(&arl_bytes)
        .with_context(|| format!("parse decompressed {}", arl_path.display()))?;

    if arl.archive_sizes.len() != 1 {
        bail!(
            "{} lists {} AR segments; the evidenced ActD_MykonosAct1 control layer has exactly one",
            arl_path.display(),
            arl.archive_sizes.len()
        );
    }

    let control_path = game_dir.join(CONTROL_AR_PART);
    let control_compressed =
        fs::read(&control_path).with_context(|| format!("read {}", control_path.display()))?;
    let control_bytes = xcompress::decompress(&control_compressed)
        .with_context(|| format!("decompress {}", control_path.display()))?;

    if control_bytes.len() != arl.archive_sizes[0] as usize {
        diagnostic(
            &mut diagnostics,
            DiagnosticSeverity::Warning,
            CONTROL_AR_PART,
            format!(
                "decompressed segment size {} does not match ARL size {}",
                control_bytes.len(),
                arl.archive_sizes[0]
            ),
        );
    }

    let control_entries = parse_archive_ref(&control_bytes)
        .with_context(|| format!("parse decompressed {}", control_path.display()))?;
    counts.control_entries = control_entries.len();

    if !arl.names.iter().any(|name| *name == STAGE_PFI_ENTRY) {
        diagnostic(
            &mut diagnostics,
            DiagnosticSeverity::Warning,
            CONTROL_ARL,
            format!("ARL name table does not list required logical entry {STAGE_PFI_ENTRY}"),
        );
    }

    let stage_pfi = find_archive_entry(&control_entries, STAGE_PFI_ENTRY)
        .with_context(|| format!("{STAGE_PFI_ENTRY} not found in {CONTROL_AR_PART}"))?;
    let pfi_records = parse_stage_pfi(stage_pfi.data).context("parse Stage.pfi")?;
    counts.pfi_entries = pfi_records.len();

    let pfd_path = game_dir.join(STAGE_PFD_RELATIVE);
    let pfd_bytes = fs::read(&pfd_path).with_context(|| format!("read {}", pfd_path.display()))?;
    let pfd = parse_pfd(&pfd_bytes).with_context(|| format!("parse {}", pfd_path.display()))?;
    counts.pfd_entries = pfd.entries().len();

    validate_pfi_against_pfd(&pfi_records, pfd.entries())?;

    let mut models = Vec::new();
    let mut instances = Vec::new();

    for pfi in pfi_records
        .iter()
        .filter(|record| is_terrain_group_name(&record.name))
    {
        counts.terrain_groups_total += 1;

        let packed = match pfi_slice(&pfd_bytes, pfi) {
            Ok(packed) => packed,
            Err(error) => {
                counts.terrain_groups_skipped += 1;
                diagnostic(
                    &mut diagnostics,
                    DiagnosticSeverity::Error,
                    &pfi.name,
                    error.to_string(),
                );
                continue;
            }
        };

        match load_terrain_group(packed) {
            Ok(TerrainGroupLoad::Empty) => {
                counts.terrain_groups_empty += 1;
                diagnostic(
                    &mut diagnostics,
                    DiagnosticSeverity::Info,
                    &pfi.name,
                    "decompressed terrain-group payload is entirely zero; no resources loaded",
                );
            }
            Ok(TerrainGroupLoad::Archive { bytes, logical_len }) => {
                let archive = match parse_archive_ref(&bytes[..logical_len]) {
                    Ok(archive) => archive,
                    Err(error) => {
                        counts.terrain_groups_skipped += 1;
                        diagnostic(
                            &mut diagnostics,
                            DiagnosticSeverity::Error,
                            &pfi.name,
                            format!("inner terrain archive rejected: {error}"),
                        );
                        continue;
                    }
                };

                counts.terrain_groups_loaded += 1;
                parse_group_resources(
                    &pfi.name,
                    &archive,
                    &mut models,
                    &mut instances,
                    &mut counts,
                    &mut diagnostics,
                );
            }
            Err(error) => {
                counts.terrain_groups_skipped += 1;
                diagnostic(
                    &mut diagnostics,
                    DiagnosticSeverity::Error,
                    &pfi.name,
                    error.to_string(),
                );
            }
        }
    }

    counts.terrain_models = models.len();
    counts.terrain_instances = instances.len();

    diagnostic(
        &mut diagnostics,
        DiagnosticSeverity::Info,
        "terrain",
        format!(
            "loaded {} terrain models and {} terrain transforms from {} non-empty groups",
            counts.terrain_models, counts.terrain_instances, counts.terrain_groups_loaded
        ),
    );

    Ok(ActDMykonosAct1 {
        game_dir,
        models,
        instances,
        counts,
        diagnostics,
    })
}

fn resolve_game_dir(root: &Path) -> Result<PathBuf> {
    if root.join(CONTROL_ARL).is_file() {
        return Ok(root.to_path_buf());
    }

    let nested = root.join("game");
    if nested.join(CONTROL_ARL).is_file() {
        return Ok(nested);
    }

    bail!(
        "{} is neither a game directory nor an install root containing game/{}",
        root.display(),
        CONTROL_ARL
    )
}

fn find_archive_entry<'a>(
    entries: &'a [ArchiveEntryRef<'a>],
    name: &str,
) -> Option<&'a ArchiveEntryRef<'a>> {
    entries.iter().find(|entry| entry.name == name)
}

/// Parse only the exact Stage.pfi record layout evidenced by Windmill Isle.
///
/// Header observations used here:
/// - BE declared file size at +0x00;
/// - BE record stride at +0x0c (0x18);
/// - BE end of record region at +0x10;
/// - BE record count at +0x18;
/// - records occupy `count * stride` bytes immediately before that end;
/// - each record is three BE u32 values followed by a 12-byte NUL-terminated
///   ASCII name.
///
/// Unknown header/record fields remain uninterpreted.
fn parse_stage_pfi(bytes: &[u8]) -> Result<Vec<PfiRecord>> {
    if bytes.len() < 0x20 {
        bail!("Stage.pfi is smaller than the evidenced header");
    }

    let declared_size = read_be_u32(bytes, 0x00)? as usize;
    if declared_size != bytes.len() {
        bail!(
            "Stage.pfi declared size {declared_size:#x} does not match actual size {:#x}",
            bytes.len()
        );
    }

    let stride = read_be_u32(bytes, 0x0c)? as usize;
    if stride != PFI_RECORD_STRIDE {
        bail!("unsupported Stage.pfi record stride {stride:#x}, expected {PFI_RECORD_STRIDE:#x}");
    }

    let records_end = read_be_u32(bytes, 0x10)? as usize;
    let count = read_be_u32(bytes, 0x18)? as usize;
    let records_len = count
        .checked_mul(stride)
        .context("Stage.pfi record region size overflow")?;
    let records_start = records_end
        .checked_sub(records_len)
        .context("Stage.pfi record region starts before file")?;

    if records_start < 0x20 || records_end > bytes.len() {
        bail!("Stage.pfi record region {records_start:#x}..{records_end:#x} is out of bounds");
    }

    let mut records = Vec::with_capacity(count);
    let mut seen = HashSet::with_capacity(count);

    for index in 0..count {
        let offset = records_start + index * stride;
        let xcompress_offset = read_be_u32(bytes, offset + 4)? as usize;
        let packed_size = read_be_u32(bytes, offset + 8)? as usize;
        let name_bytes = &bytes[offset + 12..offset + 12 + PFI_NAME_BYTES];
        let nul = name_bytes
            .iter()
            .position(|byte| *byte == 0)
            .context("Stage.pfi record name is not NUL-terminated in its 12-byte field")?;

        if nul == 0 {
            bail!("Stage.pfi record {index} has an empty name");
        }

        let name = std::str::from_utf8(&name_bytes[..nul])
            .context("Stage.pfi record name is not UTF-8")?
            .to_owned();

        if !seen.insert(name.clone()) {
            bail!("Stage.pfi contains duplicate record name {name}");
        }

        records.push(PfiRecord {
            name,
            xcompress_offset,
            packed_size,
        });
    }

    Ok(records)
}

fn validate_pfi_against_pfd(pfi: &[PfiRecord], pfd: &[PackedEntry<'_>]) -> Result<()> {
    if pfi.len() != pfd.len() {
        bail!(
            "Stage.pfi has {} records but Stage.pfd has {} records",
            pfi.len(),
            pfd.len()
        );
    }

    let by_name: HashMap<&str, &PackedEntry<'_>> =
        pfd.iter().map(|entry| (entry.name, entry)).collect();

    for record in pfi {
        let packed = by_name
            .get(record.name.as_str())
            .with_context(|| format!("Stage.pfi record {} missing from Stage.pfd", record.name))?;

        let expected_xcompress = packed
            .payload_offset
            .checked_add(PACKED_XCOMPRESS_PREFIX)
            .context("PFD XCompress offset overflow")?;

        if record.xcompress_offset != expected_xcompress {
            bail!(
                "Stage.pfi record {} XCompress offset {:#x} does not match Stage.pfd payload+0x10 {expected_xcompress:#x}",
                record.name,
                record.xcompress_offset
            );
        }

        if record.packed_size != packed.header.data_size as usize {
            bail!(
                "Stage.pfi record {} size {:#x} does not match Stage.pfd indexed size {:#x}",
                record.name,
                record.packed_size,
                packed.header.data_size
            );
        }
    }

    Ok(())
}

fn is_terrain_group_name(name: &str) -> bool {
    name.starts_with("tg-") && name.ends_with(".ar")
}

enum TerrainGroupLoad {
    Empty,
    Archive { bytes: Vec<u8>, logical_len: usize },
}

fn pfi_slice<'a>(pfd_bytes: &'a [u8], record: &PfiRecord) -> Result<&'a [u8]> {
    let end = record
        .xcompress_offset
        .checked_add(record.packed_size)
        .context("Stage.pfi byte range overflow")?;

    pfd_bytes
        .get(record.xcompress_offset..end)
        .with_context(|| {
            format!(
                "Stage.pfi record {} range {:#x}..{end:#x} extends past Stage.pfd size {:#x}",
                record.name,
                record.xcompress_offset,
                pfd_bytes.len()
            )
        })
}

fn load_terrain_group(compressed: &[u8]) -> Result<TerrainGroupLoad> {
    let bytes = xcompress::decompress(compressed)
        .context("decompress Stage.pfi-resolved terrain-group stream")?;

    if bytes.iter().all(|byte| *byte == 0) {
        return Ok(TerrainGroupLoad::Empty);
    }

    let logical_len = zero_padded_archive_len(&bytes)?;
    if logical_len == 0 {
        return Ok(TerrainGroupLoad::Empty);
    }

    Ok(TerrainGroupLoad::Archive { bytes, logical_len })
}

/// Locate the end of an AR followed by zero fill without reimplementing AR
/// resource parsing. We inspect only the fixed record-span field needed to skip
/// from one record boundary to the next; `parse_archive_ref` remains
/// authoritative for all names, payload bounds, UTF-8, and record validation.
fn zero_padded_archive_len(bytes: &[u8]) -> Result<usize> {
    if bytes.iter().all(|byte| *byte == 0) {
        return Ok(0);
    }
    if bytes.len() < 0x10 {
        bail!("inner terrain archive is smaller than its header");
    }

    let reserved = read_le_u32(bytes, 0x00)?;
    let header_size = read_le_u32(bytes, 0x04)? as usize;
    let entry_header_size = read_le_u32(bytes, 0x08)? as usize;

    if reserved != 0 || header_size != 0x10 || entry_header_size != 0x14 {
        bail!(
            "unsupported inner terrain archive header {reserved:#x}/{header_size:#x}/{entry_header_size:#x}"
        );
    }

    let mut cursor = header_size;
    while cursor < bytes.len() {
        if bytes[cursor..].iter().all(|byte| *byte == 0) {
            return Ok(cursor);
        }

        if bytes.len() - cursor < entry_header_size {
            bail!("non-zero trailing bytes after final inner archive record");
        }

        let record_span = read_le_u32(bytes, cursor)? as usize;
        if record_span < entry_header_size {
            bail!("inner archive record at {cursor:#x} has invalid span {record_span:#x}");
        }

        cursor = cursor
            .checked_add(record_span)
            .filter(|end| *end <= bytes.len())
            .context("inner archive record extends past decompressed terrain-group payload")?;
    }

    Ok(cursor)
}

fn parse_group_resources(
    group_name: &str,
    archive: &[ArchiveEntryRef<'_>],
    models: &mut Vec<TerrainModelResource>,
    instances: &mut Vec<TerrainInstanceResource>,
    counts: &mut StageCounts,
    diagnostics: &mut Vec<StageDiagnostic>,
) {
    for entry in archive {
        if entry.name.ends_with(".terrain-model") {
            match parse_terrain_mesh(entry.data) {
                Ok(mesh) => models.push(TerrainModelResource {
                    group_name: group_name.to_owned(),
                    resource_name: entry.name.to_owned(),
                    mesh,
                }),
                Err(error) => {
                    counts.resources_skipped += 1;
                    diagnostic(
                        diagnostics,
                        DiagnosticSeverity::Warning,
                        format!("{group_name}/{}", entry.name),
                        format!("terrain model skipped: {error}"),
                    );
                }
            }
        } else if entry.name.ends_with(".terrain-instanceinfo") {
            match parse_terrain_instance_transform(entry.data) {
                Ok(transform) => instances.push(TerrainInstanceResource {
                    group_name: group_name.to_owned(),
                    resource_name: entry.name.to_owned(),
                    transform,
                }),
                Err(error) => {
                    counts.resources_skipped += 1;
                    diagnostic(
                        diagnostics,
                        DiagnosticSeverity::Warning,
                        format!("{group_name}/{}", entry.name),
                        format!("terrain instance skipped: {error}"),
                    );
                }
            }
        } else {
            counts.resources_skipped += 1;
            diagnostic(
                diagnostics,
                DiagnosticSeverity::Info,
                format!("{group_name}/{}", entry.name),
                "unsupported inner terrain resource type; skipped without interpretation",
            );
        }
    }
}

fn read_be_u32(bytes: &[u8], offset: usize) -> Result<u32> {
    let end = offset.checked_add(4).context("BE u32 offset overflow")?;
    let slice = bytes
        .get(offset..end)
        .context("unexpected end of input reading BE u32")?;
    Ok(u32::from_be_bytes(slice.try_into()?))
}

fn read_le_u32(bytes: &[u8], offset: usize) -> Result<u32> {
    let end = offset.checked_add(4).context("LE u32 offset overflow")?;
    let slice = bytes
        .get(offset..end)
        .context("unexpected end of input reading LE u32")?;
    Ok(u32::from_le_bytes(slice.try_into()?))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn put_be_u32(bytes: &mut [u8], offset: usize, value: u32) {
        bytes[offset..offset + 4].copy_from_slice(&value.to_be_bytes());
    }

    fn synthetic_pfi(records: &[(&str, u32, u32)]) -> Vec<u8> {
        let records_end = 0x40 + records.len() * PFI_RECORD_STRIDE;
        let mut bytes = vec![0u8; records_end + 0x20];

        let total_len = bytes.len() as u32;
        put_be_u32(&mut bytes, 0x00, total_len);
        put_be_u32(&mut bytes, 0x0c, PFI_RECORD_STRIDE as u32);
        put_be_u32(&mut bytes, 0x10, records_end as u32);
        put_be_u32(&mut bytes, 0x18, records.len() as u32);

        let start = records_end - records.len() * PFI_RECORD_STRIDE;
        for (index, (name, offset, size)) in records.iter().enumerate() {
            assert!(name.len() < PFI_NAME_BYTES);
            let base = start + index * PFI_RECORD_STRIDE;
            put_be_u32(&mut bytes, base, base as u32);
            put_be_u32(&mut bytes, base + 4, *offset);
            put_be_u32(&mut bytes, base + 8, *size);
            bytes[base + 12..base + 12 + name.len()].copy_from_slice(name.as_bytes());
        }

        bytes
    }

    fn synthetic_ar_with_zero_tail() -> Vec<u8> {
        let name = b"mesh.terrain-model";
        let payload = [1u8, 2, 3, 4];
        let data_offset = 0x14 + name.len() + 1;
        let span = data_offset + payload.len();

        let mut bytes = Vec::new();
        bytes.extend_from_slice(&0u32.to_le_bytes());
        bytes.extend_from_slice(&0x10u32.to_le_bytes());
        bytes.extend_from_slice(&0x14u32.to_le_bytes());
        bytes.extend_from_slice(&0x10u32.to_le_bytes());

        bytes.extend_from_slice(&(span as u32).to_le_bytes());
        bytes.extend_from_slice(&(payload.len() as u32).to_le_bytes());
        bytes.extend_from_slice(&(data_offset as u32).to_le_bytes());
        bytes.extend_from_slice(&0u32.to_le_bytes());
        bytes.extend_from_slice(&0u32.to_le_bytes());
        bytes.extend_from_slice(name);
        bytes.push(0);
        bytes.extend_from_slice(&payload);

        let logical_len = bytes.len();
        bytes.extend_from_slice(&[0u8; 37]);
        assert_eq!(zero_padded_archive_len(&bytes).unwrap(), logical_len);
        bytes
    }

    #[test]
    fn parses_evidenced_pfi_record_region() {
        let bytes = synthetic_pfi(&[
            ("tg-0068.ar", 0x800, 0x5958),
            ("gia-136.ar", 0x1d3000, 0x118e),
        ]);
        let records = parse_stage_pfi(&bytes).unwrap();

        assert_eq!(records.len(), 2);
        assert_eq!(records[0].name, "tg-0068.ar");
        assert_eq!(records[0].xcompress_offset, 0x800);
        assert_eq!(records[0].packed_size, 0x5958);
        assert_eq!(records[1].name, "gia-136.ar");
    }

    #[test]
    fn rejects_unterminated_pfi_name() {
        let mut bytes = synthetic_pfi(&[("tg-0068.ar", 0x800, 0x5958)]);
        let records_end = read_be_u32(&bytes, 0x10).unwrap() as usize;
        let base = records_end - PFI_RECORD_STRIDE;
        bytes[base + 12..base + 24].fill(b'X');

        assert!(
            parse_stage_pfi(&bytes)
                .unwrap_err()
                .to_string()
                .contains("not NUL-terminated")
        );
    }

    #[test]
    fn trims_only_zero_padding_before_using_shared_ar_parser() {
        let bytes = synthetic_ar_with_zero_tail();
        let logical_len = zero_padded_archive_len(&bytes).unwrap();
        let parsed = parse_archive_ref(&bytes[..logical_len]).unwrap();

        assert_eq!(parsed.len(), 1);
        assert_eq!(parsed[0].name, "mesh.terrain-model");
        assert_eq!(parsed[0].data, &[1, 2, 3, 4]);
    }

    #[test]
    fn recognizes_all_zero_terrain_group_as_empty() {
        assert_eq!(zero_padded_archive_len(&[0u8; 64]).unwrap(), 0);
    }

    #[test]
    fn terrain_group_name_filter_is_narrow() {
        assert!(is_terrain_group_name("tg-0068.ar"));
        assert!(!is_terrain_group_name("gia-136.ar"));
        assert!(!is_terrain_group_name("tg-0068.terrain-group"));
    }

    #[test]
    fn pfi_slice_uses_authoritative_raw_pfd_range() {
        let bytes: Vec<u8> = (0u8..64).collect();
        let record = PfiRecord {
            name: "tg-0068.ar".to_owned(),
            xcompress_offset: 16,
            packed_size: 24,
        };

        assert_eq!(pfi_slice(&bytes, &record).unwrap(), &bytes[16..40]);
    }

    #[test]
    fn pfi_slice_rejects_out_of_bounds_range() {
        let bytes = [0u8; 32];
        let record = PfiRecord {
            name: "tg-0068.ar".to_owned(),
            xcompress_offset: 16,
            packed_size: 32,
        };

        assert!(
            pfi_slice(&bytes, &record)
                .unwrap_err()
                .to_string()
                .contains("extends past Stage.pfd")
        );
    }
}
