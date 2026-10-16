use std::error::Error;
use std::fmt;

pub const PFI_ENTRY_HEADER_SIZE: usize = 0x24;
pub const PFD_TRAILER_SIZE: usize = 0x10;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PfiEntryHeader {
    /// Unknown field at +0x00. Kept raw until its semantics are evidenced.
    pub raw_00: u32,
    /// Unknown field at +0x04. Kept raw until its semantics are evidenced.
    pub raw_04: u32,
    /// Unknown field at +0x08. Kept raw until its semantics are evidenced.
    pub raw_08: u32,
    /// Unknown field at +0x0c. Kept raw until its semantics are evidenced.
    pub raw_0c: u32,
    /// Size of the complete record, relative to the start of this entry.
    pub record_span: u32,
    /// Size of the payload at `data_offset`.
    pub data_size: u32,
    /// Offset of the payload, relative to the start of this entry.
    pub data_offset: u32,
    /// Unknown field at +0x1c. Kept raw until its semantics are evidenced.
    pub raw_1c: u32,
    /// Unknown field at +0x20. Kept raw until its semantics are evidenced.
    pub raw_20: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PackedEntry<'a> {
    pub name: &'a str,
    pub record_offset: usize,
    pub payload_offset: usize,
    pub header: PfiEntryHeader,
    pub data: &'a [u8],
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PackedFileIndex<'a> {
    entries: Vec<PackedEntry<'a>>,
    trailer: &'a [u8],
}

impl<'a> PackedFileIndex<'a> {
    pub fn entries(&self) -> &[PackedEntry<'a>] {
        &self.entries
    }

    pub fn trailer(&self) -> &'a [u8] {
        self.trailer
    }

    pub fn find(&self, name: &str) -> Option<&PackedEntry<'a>> {
        self.entries.iter().find(|entry| entry.name == name)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PackedFileError {
    HeaderTruncated {
        offset: usize,
        remaining: usize,
    },
    MissingTrailer {
        len: usize,
    },
    ZeroRecordSpan {
        offset: usize,
    },
    SpanMismatch {
        offset: usize,
        record_span: u32,
        data_offset: u32,
        data_size: u32,
    },
    DataOffsetBeforeName {
        offset: usize,
        data_offset: u32,
    },
    RecordOutOfBounds {
        offset: usize,
        record_span: u32,
        records_end: usize,
    },
    UnterminatedName {
        offset: usize,
    },
    InvalidNameUtf8 {
        offset: usize,
    },
}

impl fmt::Display for PackedFileError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::HeaderTruncated { offset, remaining } => {
                write!(
                    f,
                    "packed-file entry header at {offset:#x} is truncated ({remaining} bytes remain)"
                )
            }
            Self::MissingTrailer { len } => {
                write!(
                    f,
                    "packed file is {len} bytes, smaller than the {PFD_TRAILER_SIZE:#x}-byte trailer"
                )
            }
            Self::ZeroRecordSpan { offset } => {
                write!(f, "packed-file entry at {offset:#x} has a zero record span")
            }
            Self::SpanMismatch {
                offset,
                record_span,
                data_offset,
                data_size,
            } => {
                write!(
                    f,
                    "packed-file entry at {offset:#x} has span {record_span:#x}, but data_offset + data_size is {:#x}",
                    u64::from(*data_offset) + u64::from(*data_size)
                )
            }
            Self::DataOffsetBeforeName {
                offset,
                data_offset,
            } => {
                write!(
                    f,
                    "packed-file entry at {offset:#x} has data offset {data_offset:#x}, leaving no room for a NUL-terminated name"
                )
            }
            Self::RecordOutOfBounds {
                offset,
                record_span,
                records_end,
            } => {
                write!(
                    f,
                    "packed-file entry at {offset:#x} with span {record_span:#x} extends past record region ending at {records_end:#x}"
                )
            }
            Self::UnterminatedName { offset } => {
                write!(
                    f,
                    "packed-file entry at {offset:#x} has no NUL terminator before its payload"
                )
            }
            Self::InvalidNameUtf8 { offset } => {
                write!(
                    f,
                    "packed-file entry at {offset:#x} has a non-UTF-8 file name"
                )
            }
        }
    }
}

impl Error for PackedFileError {}

pub fn parse_pfi_entry_header(bytes: &[u8]) -> Result<PfiEntryHeader, PackedFileError> {
    if bytes.len() < PFI_ENTRY_HEADER_SIZE {
        return Err(PackedFileError::HeaderTruncated {
            offset: 0,
            remaining: bytes.len(),
        });
    }

    let u32_le = |offset: usize| {
        u32::from_le_bytes(
            bytes[offset..offset + 4]
                .try_into()
                .expect("PFI header bounds checked above"),
        )
    };

    Ok(PfiEntryHeader {
        raw_00: u32_le(0x00),
        raw_04: u32_le(0x04),
        raw_08: u32_le(0x08),
        raw_0c: u32_le(0x0c),
        record_span: u32_le(0x10),
        data_size: u32_le(0x14),
        data_offset: u32_le(0x18),
        raw_1c: u32_le(0x1c),
        raw_20: u32_le(0x20),
    })
}

/// Parse the packed-file record layout observed in `game/Packed/ActD_MykonosAct1/Stage.pfd`.
///
/// Evidence from that file:
/// - each record starts with a 0x24-byte little-endian PFI-style header;
/// - the NUL-terminated archive name starts at +0x24;
/// - `record_span == data_offset + data_size`;
/// - `record_span` is relative to the current record start;
/// - records fill the file up to a final 0x10-byte trailer.
///
/// Other stage PFD variants exist and are intentionally rejected when they do not satisfy these
/// invariants rather than guessing their layout.
pub fn parse_pfd(bytes: &[u8]) -> Result<PackedFileIndex<'_>, PackedFileError> {
    if bytes.len() < PFD_TRAILER_SIZE {
        return Err(PackedFileError::MissingTrailer { len: bytes.len() });
    }

    let records_end = bytes.len() - PFD_TRAILER_SIZE;
    let mut entries = Vec::new();
    let mut cursor = 0usize;

    while cursor < records_end {
        let remaining = records_end - cursor;
        if remaining < PFI_ENTRY_HEADER_SIZE {
            return Err(PackedFileError::HeaderTruncated {
                offset: cursor,
                remaining,
            });
        }

        let header = parse_pfi_entry_header(&bytes[cursor..cursor + PFI_ENTRY_HEADER_SIZE])?;

        if header.record_span == 0 {
            return Err(PackedFileError::ZeroRecordSpan { offset: cursor });
        }

        let expected_span = u64::from(header.data_offset) + u64::from(header.data_size);
        if u64::from(header.record_span) != expected_span {
            return Err(PackedFileError::SpanMismatch {
                offset: cursor,
                record_span: header.record_span,
                data_offset: header.data_offset,
                data_size: header.data_size,
            });
        }

        if header.data_offset as usize <= PFI_ENTRY_HEADER_SIZE {
            return Err(PackedFileError::DataOffsetBeforeName {
                offset: cursor,
                data_offset: header.data_offset,
            });
        }

        let record_end = cursor
            .checked_add(header.record_span as usize)
            .filter(|&end| end <= records_end)
            .ok_or(PackedFileError::RecordOutOfBounds {
                offset: cursor,
                record_span: header.record_span,
                records_end,
            })?;

        let payload_offset = cursor + header.data_offset as usize;
        let name_start = cursor + PFI_ENTRY_HEADER_SIZE;
        let name_region = &bytes[name_start..payload_offset];
        let nul = name_region
            .iter()
            .position(|&byte| byte == 0)
            .ok_or(PackedFileError::UnterminatedName { offset: cursor })?;
        let name = std::str::from_utf8(&name_region[..nul])
            .map_err(|_| PackedFileError::InvalidNameUtf8 { offset: cursor })?;

        entries.push(PackedEntry {
            name,
            record_offset: cursor,
            payload_offset,
            header,
            data: &bytes[payload_offset..record_end],
        });

        cursor = record_end;
    }

    Ok(PackedFileIndex {
        entries,
        trailer: &bytes[records_end..],
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const MYKONOS_FIRST_HEADER: [u8; PFI_ENTRY_HEADER_SIZE] = [
        0x00, 0x00, 0x00, 0x00, // +00
        0x10, 0x00, 0x00, 0x00, // +04
        0x14, 0x00, 0x00, 0x00, // +08
        0x00, 0x08, 0x00, 0x00, // +0c
        0x48, 0x61, 0x00, 0x00, // +10 record span
        0x58, 0x59, 0x00, 0x00, // +14 data size
        0xf0, 0x07, 0x00, 0x00, // +18 data offset
        0x04, 0x9c, 0x8e, 0xab, // +1c
        0x5e, 0xee, 0xca, 0x08, // +20
    ];

    #[test]
    fn parses_real_mykonos_first_entry_header() {
        let header = parse_pfi_entry_header(&MYKONOS_FIRST_HEADER).unwrap();

        assert_eq!(header.raw_00, 0x0000_0000);
        assert_eq!(header.raw_04, 0x0000_0010);
        assert_eq!(header.raw_08, 0x0000_0014);
        assert_eq!(header.raw_0c, 0x0000_0800);
        assert_eq!(header.record_span, 0x0000_6148);
        assert_eq!(header.data_size, 0x0000_5958);
        assert_eq!(header.data_offset, 0x0000_07f0);
        assert_eq!(header.raw_1c, 0xab8e_9c04);
        assert_eq!(header.raw_20, 0x08ca_ee5e);
        assert_eq!(header.record_span, header.data_offset + header.data_size);
    }

    fn push_record(bytes: &mut Vec<u8>, name: &str, data: &[u8]) {
        let data_offset = PFI_ENTRY_HEADER_SIZE + name.len() + 1;
        let record_span = data_offset + data.len();

        bytes.extend_from_slice(&0x1111_1111u32.to_le_bytes());
        bytes.extend_from_slice(&0x2222_2222u32.to_le_bytes());
        bytes.extend_from_slice(&0x3333_3333u32.to_le_bytes());
        bytes.extend_from_slice(&0u32.to_le_bytes());
        bytes.extend_from_slice(&(record_span as u32).to_le_bytes());
        bytes.extend_from_slice(&(data.len() as u32).to_le_bytes());
        bytes.extend_from_slice(&(data_offset as u32).to_le_bytes());
        bytes.extend_from_slice(&0x4444_4444u32.to_le_bytes());
        bytes.extend_from_slice(&0x5555_5555u32.to_le_bytes());
        bytes.extend_from_slice(name.as_bytes());
        bytes.push(0);
        bytes.extend_from_slice(data);
    }

    #[test]
    fn parses_records_and_locates_payload_by_name() {
        let mut bytes = Vec::new();
        push_record(&mut bytes, "tg-0068.ar", b"terrain-a");
        push_record(&mut bytes, "gia-136.ar", b"terrain-b");
        bytes.extend_from_slice(&[0xa5; PFD_TRAILER_SIZE]);

        let index = parse_pfd(&bytes).unwrap();

        assert_eq!(index.entries().len(), 2);
        assert_eq!(index.trailer(), &[0xa5; PFD_TRAILER_SIZE]);

        let tg = index.find("tg-0068.ar").unwrap();
        assert_eq!(tg.record_offset, 0);
        assert_eq!(tg.data, b"terrain-a");
        assert_eq!(
            tg.header.record_span,
            tg.header.data_offset + tg.header.data_size
        );

        let gia = index.find("gia-136.ar").unwrap();
        assert_eq!(gia.data, b"terrain-b");
        assert_eq!(
            gia.payload_offset,
            gia.record_offset + gia.header.data_offset as usize
        );

        assert!(index.find("tg-9999.ar").is_none());
    }

    #[test]
    fn rejects_record_span_that_does_not_match_payload_extent() {
        let mut bytes = Vec::new();
        push_record(&mut bytes, "tg-0068.ar", b"abc");
        bytes.extend_from_slice(&[0; PFD_TRAILER_SIZE]);

        let span = u32::from_le_bytes(bytes[0x10..0x14].try_into().unwrap());
        bytes[0x10..0x14].copy_from_slice(&(span + 1).to_le_bytes());

        assert!(matches!(
            parse_pfd(&bytes),
            Err(PackedFileError::SpanMismatch { offset: 0, .. })
        ));
    }

    #[test]
    fn rejects_name_without_nul_before_payload() {
        let mut bytes = Vec::new();
        push_record(&mut bytes, "tg-0068.ar", b"abc");
        bytes.extend_from_slice(&[0; PFD_TRAILER_SIZE]);

        let data_offset = u32::from_le_bytes(bytes[0x18..0x1c].try_into().unwrap()) as usize;
        bytes[PFI_ENTRY_HEADER_SIZE + "tg-0068.ar".len()] = b'X';

        assert!(data_offset > PFI_ENTRY_HEADER_SIZE);
        assert!(matches!(
            parse_pfd(&bytes),
            Err(PackedFileError::UnterminatedName { offset: 0 })
        ));
    }
}
