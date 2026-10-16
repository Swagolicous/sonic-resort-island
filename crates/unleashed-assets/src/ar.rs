use anyhow::{Context, Result, bail};
use std::str;

pub const AR_HEADER_SIZE: usize = 0x10;
pub const AR_ENTRY_HEADER_SIZE: usize = 0x14;
pub const ARL_MAGIC: [u8; 4] = *b"ARL2";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArchiveEntry<'a> {
    pub name: String,
    pub data: &'a [u8],
}

/// Fully borrowed AR entry. Both the file name and payload reference `bytes`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ArchiveEntryRef<'a> {
    pub name: &'a str,
    pub data: &'a [u8],
    pub hash: u32,
    pub timestamp: u32,
}

/// ARL2 archive list. File names borrow directly from the ARL input.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArchiveList<'a> {
    /// Expected decompressed byte size of each `.ar.NN` segment, in order.
    pub archive_sizes: Vec<u32>,
    /// File names in archive order, spanning all listed `.ar.NN` segments.
    pub names: Vec<&'a str>,
}

/// Compatibility API: owns names, but payloads remain zero-copy.
pub fn parse_archive(bytes: &[u8]) -> Result<Vec<ArchiveEntry<'_>>> {
    Ok(parse_archive_ref(bytes)?
        .into_iter()
        .map(|entry| ArchiveEntry {
            name: entry.name.to_owned(),
            data: entry.data,
        })
        .collect())
}

/// Parses one decompressed Sonic Unleashed `.ar.NN` segment.
///
/// Observed header layout:
/// `reserved:u32, header_size:u32, entry_header_size:u32, alignment:u32`.
///
/// Each record begins with:
/// `entry_size:u32, data_size:u32, data_offset:u32, hash:u32, timestamp:u32`,
/// followed by a NUL-terminated UTF-8 name, optional padding, and payload.
/// `entry_size` advances to the next record.
pub fn parse_archive_ref(bytes: &[u8]) -> Result<Vec<ArchiveEntryRef<'_>>> {
    if bytes.len() < AR_HEADER_SIZE {
        bail!("AR archive is smaller than 0x10-byte header");
    }

    let reserved = read_u32_le(bytes, 0x00)?;
    let header_size = read_u32_le(bytes, 0x04)? as usize;
    let entry_header_size = read_u32_le(bytes, 0x08)? as usize;

    if reserved != 0 {
        bail!("unsupported AR header reserved field {reserved:#x}");
    }
    if header_size != AR_HEADER_SIZE {
        bail!("unsupported AR header size {header_size:#x}, expected {AR_HEADER_SIZE:#x}");
    }
    if entry_header_size != AR_ENTRY_HEADER_SIZE {
        bail!(
            "unsupported AR entry header size {entry_header_size:#x}, expected {AR_ENTRY_HEADER_SIZE:#x}"
        );
    }

    let mut entries = Vec::new();
    let mut cursor = header_size;

    while cursor < bytes.len() {
        let remaining = bytes.len() - cursor;
        if remaining < entry_header_size {
            bail!(
                "AR archive has {remaining} trailing byte(s) at offset {cursor:#x}, smaller than an entry header"
            );
        }

        let entry_size = read_u32_le(bytes, cursor)? as usize;
        let data_size = read_u32_le(bytes, cursor + 0x04)? as usize;
        let data_offset = read_u32_le(bytes, cursor + 0x08)? as usize;
        let hash = read_u32_le(bytes, cursor + 0x0c)?;
        let timestamp = read_u32_le(bytes, cursor + 0x10)?;

        if entry_size < entry_header_size {
            bail!("AR entry at offset {cursor:#x} has invalid size {entry_size:#x}");
        }

        let entry_end = cursor
            .checked_add(entry_size)
            .context("AR entry size overflow")?;
        if entry_end > bytes.len() {
            bail!("AR entry at offset {cursor:#x} extends past archive");
        }

        // data_offset must leave at least one byte for a non-empty name plus NUL.
        if data_offset <= entry_header_size + 1 {
            bail!("AR entry at offset {cursor:#x} has invalid data offset {data_offset:#x}");
        }
        if data_offset > entry_size {
            bail!("AR entry at offset {cursor:#x} has data offset past the end of its record");
        }

        let payload_rel_end = data_offset
            .checked_add(data_size)
            .context("AR payload size overflow")?;
        if payload_rel_end > entry_size {
            bail!("AR entry at offset {cursor:#x} has payload extending past its record");
        }

        let name_start = cursor + entry_header_size;
        let name_end_limit = cursor + data_offset;
        let name_region = &bytes[name_start..name_end_limit];
        let nul = name_region
            .iter()
            .position(|&b| b == 0)
            .context("AR entry name is not NUL-terminated before payload")?;

        if nul == 0 {
            bail!("AR entry at offset {cursor:#x} has an empty name");
        }

        // Some archives carry Shift-JIS names; they are never referenced by
        // the ASCII resource names the engine looks up, so skip them.
        let Ok(name) = str::from_utf8(&name_region[..nul]) else {
            cursor = entry_end;
            continue;
        };

        let payload_start = cursor + data_offset;
        let payload_end = payload_start + data_size;

        entries.push(ArchiveEntryRef {
            name,
            data: &bytes[payload_start..payload_end],
            hash,
            timestamp,
        });

        cursor = entry_end;
    }

    Ok(entries)
}

/// Parses a decompressed Sonic Unleashed `ARL2` archive list.
///
/// Layout:
/// `b"ARL2"`, `archive_count:u32`, `archive_count * u32 archive_sizes`,
/// then one-byte-length-prefixed UTF-8 file names through EOF.
pub fn parse_archive_list(bytes: &[u8]) -> Result<ArchiveList<'_>> {
    if bytes.len() < 8 {
        bail!("ARL file is smaller than 8-byte header");
    }

    if bytes[..4] != ARL_MAGIC {
        bail!("bad ARL magic");
    }

    let archive_count = read_u32_le(bytes, 4)? as usize;
    let size_table_len = archive_count
        .checked_mul(4)
        .context("ARL archive count overflow")?;
    let names_start = 8usize
        .checked_add(size_table_len)
        .context("ARL header size overflow")?;

    if names_start > bytes.len() {
        bail!("ARL archive size table is truncated");
    }

    let mut archive_sizes = Vec::with_capacity(archive_count);
    for index in 0..archive_count {
        archive_sizes.push(read_u32_le(bytes, 8 + index * 4)?);
    }

    let mut names = Vec::new();
    let mut cursor = names_start;

    while cursor < bytes.len() {
        let length_offset = cursor;
        let name_len = bytes[cursor] as usize;
        cursor += 1;

        if name_len == 0 {
            bail!("ARL contains an empty filename at offset {length_offset:#x}");
        }

        let end = cursor
            .checked_add(name_len)
            .context("ARL filename length overflow")?;

        if end > bytes.len() {
            bail!("ARL filename at offset {length_offset:#x} is truncated");
        }

        names.push(str::from_utf8(&bytes[cursor..end]).unwrap_or("<non-utf8 name>"));
        cursor = end;
    }

    Ok(ArchiveList {
        archive_sizes,
        names,
    })
}

fn read_u32_le(bytes: &[u8], offset: usize) -> Result<u32> {
    let end = offset.checked_add(4).context("u32 offset overflow")?;
    let slice = bytes
        .get(offset..end)
        .context("unexpected end of input while reading u32")?;
    Ok(u32::from_le_bytes(slice.try_into()?))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_ar(entries: &[(&str, &[u8])]) -> Vec<u8> {
        let mut out = Vec::new();
        out.extend_from_slice(&0u32.to_le_bytes());
        out.extend_from_slice(&(AR_HEADER_SIZE as u32).to_le_bytes());
        out.extend_from_slice(&(AR_ENTRY_HEADER_SIZE as u32).to_le_bytes());
        out.extend_from_slice(&0x40u32.to_le_bytes());

        for (index, (name, data)) in entries.iter().enumerate() {
            let data_offset = AR_ENTRY_HEADER_SIZE + name.len() + 1;
            let entry_size = data_offset + data.len();

            out.extend_from_slice(&(entry_size as u32).to_le_bytes());
            out.extend_from_slice(&(data.len() as u32).to_le_bytes());
            out.extend_from_slice(&(data_offset as u32).to_le_bytes());
            out.extend_from_slice(&(0x1000_0000u32 + index as u32).to_le_bytes());
            out.extend_from_slice(&(0x2000_0000u32 + index as u32).to_le_bytes());
            out.extend_from_slice(name.as_bytes());
            out.push(0);
            out.extend_from_slice(data);
        }

        out
    }

    fn make_arl(sizes: &[u32], names: &[&str]) -> Vec<u8> {
        let mut out = Vec::new();
        out.extend_from_slice(&ARL_MAGIC);
        out.extend_from_slice(&(sizes.len() as u32).to_le_bytes());

        for size in sizes {
            out.extend_from_slice(&size.to_le_bytes());
        }

        for name in names {
            assert!(name.len() <= u8::MAX as usize);
            out.push(name.len() as u8);
            out.extend_from_slice(name.as_bytes());
        }

        out
    }

    #[test]
    fn parses_ar_zero_copy() {
        let bytes = make_ar(&[("hello.bin", &[1, 2, 3]), ("terrain-group", &[4, 5])]);

        let entries = parse_archive_ref(&bytes).unwrap();

        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].name, "hello.bin");
        assert_eq!(entries[0].data, &[1, 2, 3]);
        assert_eq!(entries[0].hash, 0x1000_0000);
        assert_eq!(entries[0].timestamp, 0x2000_0000);
        assert_eq!(entries[1].name, "terrain-group");
        assert_eq!(entries[1].data, &[4, 5]);

        let base = bytes.as_ptr() as usize;
        let name_ptr = entries[0].name.as_ptr() as usize;
        let data_ptr = entries[0].data.as_ptr() as usize;
        assert!((base..base + bytes.len()).contains(&name_ptr));
        assert!((base..base + bytes.len()).contains(&data_ptr));
    }

    #[test]
    fn compatibility_api_keeps_payload_borrowed() {
        let bytes = make_ar(&[("foo", &[7, 8, 9])]);
        let owned = parse_archive(&bytes).unwrap();
        let borrowed = parse_archive_ref(&bytes).unwrap();

        assert_eq!(owned[0].name, borrowed[0].name);
        assert_eq!(owned[0].data, borrowed[0].data);

        let base = bytes.as_ptr() as usize;
        let data_ptr = owned[0].data.as_ptr() as usize;
        assert!((base..base + bytes.len()).contains(&data_ptr));
    }

    #[test]
    fn rejects_ar_payload_that_escapes_record() {
        let mut bytes = make_ar(&[("foo", &[1, 2, 3])]);
        let entry = AR_HEADER_SIZE;
        bytes[entry + 4..entry + 8].copy_from_slice(&0x100u32.to_le_bytes());

        assert!(
            parse_archive_ref(&bytes)
                .unwrap_err()
                .to_string()
                .contains("payload extending past its record")
        );
    }

    #[test]
    fn rejects_ar_name_without_nul_before_payload() {
        let mut bytes = make_ar(&[("foo", &[1])]);
        let entry = AR_HEADER_SIZE;
        let data_offset = AR_ENTRY_HEADER_SIZE + 3;
        bytes[entry + 8..entry + 12].copy_from_slice(&(data_offset as u32).to_le_bytes());

        assert!(
            parse_archive_ref(&bytes)
                .unwrap_err()
                .to_string()
                .contains("not NUL-terminated")
        );
    }

    #[test]
    fn rejects_ar_entry_past_eof_and_trailing_garbage() {
        let mut past_eof = make_ar(&[("foo", &[1])]);
        let entry = AR_HEADER_SIZE;
        past_eof[entry..entry + 4].copy_from_slice(&0x100u32.to_le_bytes());
        assert!(
            parse_archive_ref(&past_eof)
                .unwrap_err()
                .to_string()
                .contains("extends past archive")
        );

        let mut trailing = make_ar(&[("foo", &[1])]);
        trailing.push(0xff);
        assert!(
            parse_archive_ref(&trailing)
                .unwrap_err()
                .to_string()
                .contains("trailing byte")
        );
    }

    #[test]
    fn parses_arl2_zero_copy_names_and_sizes() {
        let bytes = make_arl(
            &[0x1234, 0x5678],
            &[
                "Autodraw.txt",
                "visibility-tree.vt",
                "tg-0068.terrain-group",
            ],
        );

        let arl = parse_archive_list(&bytes).unwrap();

        assert_eq!(arl.archive_sizes, vec![0x1234, 0x5678]);
        assert_eq!(
            arl.names,
            vec![
                "Autodraw.txt",
                "visibility-tree.vt",
                "tg-0068.terrain-group"
            ]
        );

        let base = bytes.as_ptr() as usize;
        let name_ptr = arl.names[0].as_ptr() as usize;
        assert!((base..base + bytes.len()).contains(&name_ptr));
    }

    #[test]
    fn supports_one_and_two_segment_arl_headers() {
        let one = make_arl(&[0x267ad0], &["Stage.stg.xml", "Base.set.xml"]);
        assert_eq!(one[12], 13);
        let arl = parse_archive_list(&one).unwrap();
        assert_eq!(arl.archive_sizes, vec![0x267ad0]);
        assert_eq!(arl.names, vec!["Stage.stg.xml", "Base.set.xml"]);

        let two = make_arl(&[0x9da568, 0x41f7f8], &["foo"]);
        assert_eq!(two[16], 3);
        let arl = parse_archive_list(&two).unwrap();
        assert_eq!(arl.archive_sizes, vec![0x9da568, 0x41f7f8]);
        assert_eq!(arl.names, vec!["foo"]);
    }

    #[test]
    fn rejects_bad_or_truncated_arl() {
        let mut bad_magic = make_arl(&[1], &["foo"]);
        bad_magic[0] = b'X';
        assert!(parse_archive_list(&bad_magic).is_err());

        let mut short_table = b"ARL2".to_vec();
        short_table.extend_from_slice(&2u32.to_le_bytes());
        short_table.extend_from_slice(&10u32.to_le_bytes());
        assert!(
            parse_archive_list(&short_table)
                .unwrap_err()
                .to_string()
                .contains("size table is truncated")
        );

        let mut empty_name = make_arl(&[], &[]);
        empty_name.push(0);
        assert!(
            parse_archive_list(&empty_name)
                .unwrap_err()
                .to_string()
                .contains("empty filename")
        );

        let mut truncated_name = make_arl(&[], &[]);
        truncated_name.push(5);
        truncated_name.extend_from_slice(b"abc");
        assert!(
            parse_archive_list(&truncated_name)
                .unwrap_err()
                .to_string()
                .contains("truncated")
        );
    }

    #[test]
    fn validates_mykonos_fixtures_when_available() {
        use std::fs;

        let root = std::env::temp_dir().join("ur-mykonos");
        let main_arl_path = root.join("ActD_MykonosAct1.arl.dec");
        let main_ar0_path = root.join("ActD_MykonosAct1.ar.00.dec");
        let main_ar1_path = root.join("ActD_MykonosAct1.ar.01.dec");
        let stage_arl_path = root.join("#ActD_MykonosAct1.arl.dec");
        let stage_ar0_path = root.join("#ActD_MykonosAct1.ar.00.dec");

        if ![
            &main_arl_path,
            &main_ar0_path,
            &main_ar1_path,
            &stage_arl_path,
            &stage_ar0_path,
        ]
        .iter()
        .all(|path| path.exists())
        {
            return;
        }

        let main_arl_bytes = fs::read(main_arl_path).unwrap();
        let main_ar0_bytes = fs::read(main_ar0_path).unwrap();
        let main_ar1_bytes = fs::read(main_ar1_path).unwrap();

        let main_arl = parse_archive_list(&main_arl_bytes).unwrap();
        let main_ar0 = parse_archive_ref(&main_ar0_bytes).unwrap();
        let main_ar1 = parse_archive_ref(&main_ar1_bytes).unwrap();

        assert_eq!(
            main_arl.archive_sizes,
            vec![main_ar0_bytes.len() as u32, main_ar1_bytes.len() as u32]
        );
        assert_eq!(main_ar0.len(), 377);
        assert_eq!(main_ar1.len(), 95);
        assert_eq!(main_arl.names.len(), 472);

        let archive_names: Vec<_> = main_ar0
            .iter()
            .chain(main_ar1.iter())
            .map(|entry| entry.name)
            .collect();
        assert_eq!(main_arl.names, archive_names);
        assert_eq!(main_arl.names.first().copied(), Some("Autodraw.txt"));
        assert_eq!(
            main_arl.names.last().copied(),
            Some("cmn_wood_ms_woodbox_pow.dds")
        );

        let stage_arl_bytes = fs::read(stage_arl_path).unwrap();
        let stage_ar0_bytes = fs::read(stage_ar0_path).unwrap();

        let stage_arl = parse_archive_list(&stage_arl_bytes).unwrap();
        let stage_ar0 = parse_archive_ref(&stage_ar0_bytes).unwrap();

        assert_eq!(stage_arl.archive_sizes, vec![stage_ar0_bytes.len() as u32]);
        assert_eq!(stage_ar0.len(), 114);
        assert_eq!(stage_arl.names.len(), 114);

        let stage_names: Vec<_> = stage_ar0.iter().map(|entry| entry.name).collect();
        assert_eq!(stage_arl.names, stage_names);
        assert_eq!(stage_arl.names.first().copied(), Some("Stage.stg.xml"));
        assert_eq!(
            stage_arl.names.last().copied(),
            Some("myk_col_act01.phy.hkx")
        );
    }
}
