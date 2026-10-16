pub mod havok;

use std::{error::Error, fmt, ops::Range, str};

pub const HAVOK_MAGIC_0: u32 = 0x57E0_E057;
pub const HAVOK_MAGIC_1: u32 = 0x10C0_C010;

/// Header size observed in Sonic Unleashed Xbox 360 HKX packfiles.
pub const PACKFILE_HEADER_SIZE: usize = 0x40;

/// Section-directory record size observed immediately after the 0x40-byte header.
///
/// Proven byte layout:
/// - `0x00..0x13`: 19-byte NUL-padded ASCII tag
/// - `0x13`: marker byte (`0xff` in all locally inspected Sonic Unleashed HKX files)
/// - `0x14..0x18`: absolute section-data start, big-endian u32
/// - `0x18..0x30`: six monotonic section-relative boundary offsets, big-endian u32
///
/// This module intentionally does not assign Havok-specific meanings to the six boundaries.
pub const PACKFILE_SECTION_HEADER_SIZE: usize = 0x30;
pub const PACKFILE_SECTION_TAG_LEN: usize = 19;

pub const SONIC_UNLEASHED_FILE_VERSION: u32 = 5;
pub const SONIC_UNLEASHED_LAYOUT_WORD: u32 = 0x0400_0001;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CollisionParseError(String);

impl CollisionParseError {
    fn new(message: impl Into<String>) -> Self {
        Self(message.into())
    }
}

impl fmt::Display for CollisionParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl Error for CollisionParseError {}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PackfileHeader<'a> {
    pub file_version: u32,
    pub layout_word: u32,
    pub section_count: u32,

    /// Raw big-endian words at header offsets 0x18, 0x1c, 0x20, and 0x24.
    ///
    /// Their Havok semantics are deliberately not encoded here because this task only relies on
    /// byte relationships directly verified in local Sonic Unleashed assets.
    pub words_18_24: [u32; 4],

    pub version_string: &'a str,

    /// Raw big-endian words at 0x38 and 0x3c.
    pub trailing_words: [u32; 2],
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PackfileSection<'a> {
    pub tag: &'a str,
    pub marker: u8,
    pub absolute_data_start: u32,

    /// Six monotonic section-relative boundaries from record offsets 0x18..0x30.
    ///
    /// The final boundary is the observed section end offset.
    pub boundaries: [u32; 6],

    /// Entire bounded section payload: `absolute_data_start .. absolute_data_start + boundaries[5]`.
    pub data: &'a [u8],
}

impl PackfileSection<'_> {
    pub fn end_offset(&self) -> u32 {
        self.boundaries[5]
    }

    pub fn file_range(&self) -> Range<usize> {
        let start = self.absolute_data_start as usize;
        start..start + self.end_offset() as usize
    }

    /// File range before the first observed relative boundary.
    pub fn leading_range(&self) -> Range<usize> {
        let start = self.absolute_data_start as usize;
        start..start + self.boundaries[0] as usize
    }

    /// File range between two adjacent observed boundaries.
    ///
    /// `index` is 0 through 4, selecting boundaries `[index]..[index + 1]`.
    pub fn boundary_range(&self, index: usize) -> Option<Range<usize>> {
        if index >= 5 {
            return None;
        }

        let base = self.absolute_data_start as usize;
        Some(base + self.boundaries[index] as usize..base + self.boundaries[index + 1] as usize)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HavokPackfile<'a> {
    pub header: PackfileHeader<'a>,
    pub sections: Vec<PackfileSection<'a>>,
}

/// Enumerates the proven section envelope of Sonic Unleashed Xbox 360 Havok 5 packfiles.
///
/// This parser deliberately stops at packfile section boundaries. It does not decode MOPP data,
/// mesh/triangle storage, fixup records, or serialized Havok object layouts.
pub fn parse_havok_packfile(bytes: &[u8]) -> Result<HavokPackfile<'_>, CollisionParseError> {
    if bytes.len() < PACKFILE_HEADER_SIZE {
        return Err(CollisionParseError::new(
            "HKX file is smaller than the observed 0x40-byte packfile header",
        ));
    }

    let magic_0 = read_u32_be(bytes, 0x00)?;
    let magic_1 = read_u32_be(bytes, 0x04)?;
    if magic_0 != HAVOK_MAGIC_0 || magic_1 != HAVOK_MAGIC_1 {
        return Err(CollisionParseError::new(format!(
            "bad HKX magic {magic_0:#010x} {magic_1:#010x}"
        )));
    }

    let file_version = read_u32_be(bytes, 0x0c)?;
    if file_version != SONIC_UNLEASHED_FILE_VERSION {
        return Err(CollisionParseError::new(format!(
            "unsupported HKX file version {file_version}; only locally observed version 5 is implemented"
        )));
    }

    let layout_word = read_u32_be(bytes, 0x10)?;
    if layout_word != SONIC_UNLEASHED_LAYOUT_WORD {
        return Err(CollisionParseError::new(format!(
            "unsupported HKX layout word {layout_word:#010x}; expected locally observed {SONIC_UNLEASHED_LAYOUT_WORD:#010x}"
        )));
    }

    let section_count = read_u32_be(bytes, 0x14)?;
    if section_count == 0 {
        return Err(CollisionParseError::new("HKX has zero sections"));
    }

    let section_count_usize = usize::try_from(section_count)
        .map_err(|_| CollisionParseError::new("HKX section count does not fit usize"))?;
    let directory_size = section_count_usize
        .checked_mul(PACKFILE_SECTION_HEADER_SIZE)
        .ok_or_else(|| CollisionParseError::new("HKX section directory size overflow"))?;
    let directory_end = PACKFILE_HEADER_SIZE
        .checked_add(directory_size)
        .ok_or_else(|| CollisionParseError::new("HKX section directory end overflow"))?;
    if directory_end > bytes.len() {
        return Err(CollisionParseError::new(format!(
            "HKX section directory ends at {directory_end:#x}, past file size {:#x}",
            bytes.len()
        )));
    }

    let version_bytes = &bytes[0x28..0x38];
    let version_len = version_bytes
        .iter()
        .position(|&byte| byte == 0)
        .unwrap_or(version_bytes.len());
    if version_len == 0 {
        return Err(CollisionParseError::new("HKX version string is empty"));
    }
    let version_string = str::from_utf8(&version_bytes[..version_len])
        .map_err(|_| CollisionParseError::new("HKX version string is not valid UTF-8"))?;

    let header = PackfileHeader {
        file_version,
        layout_word,
        section_count,
        words_18_24: [
            read_u32_be(bytes, 0x18)?,
            read_u32_be(bytes, 0x1c)?,
            read_u32_be(bytes, 0x20)?,
            read_u32_be(bytes, 0x24)?,
        ],
        version_string,
        trailing_words: [read_u32_be(bytes, 0x38)?, read_u32_be(bytes, 0x3c)?],
    };

    let mut sections = Vec::with_capacity(section_count_usize);
    let mut previous_end = directory_end;

    for index in 0..section_count_usize {
        let offset = PACKFILE_HEADER_SIZE + index * PACKFILE_SECTION_HEADER_SIZE;
        let tag_region = &bytes[offset..offset + PACKFILE_SECTION_TAG_LEN];
        let tag_len = tag_region
            .iter()
            .position(|&byte| byte == 0)
            .unwrap_or(tag_region.len());

        if tag_len == 0 {
            return Err(CollisionParseError::new(format!(
                "HKX section {index} has an empty tag"
            )));
        }

        if tag_region[tag_len..].iter().any(|&byte| byte != 0) {
            return Err(CollisionParseError::new(format!(
                "HKX section {index} tag has nonzero bytes after its first NUL"
            )));
        }

        let tag = str::from_utf8(&tag_region[..tag_len]).map_err(|_| {
            CollisionParseError::new(format!("HKX section {index} tag is not valid UTF-8"))
        })?;
        if !tag.bytes().all(|byte| byte.is_ascii_graphic()) {
            return Err(CollisionParseError::new(format!(
                "HKX section {index} tag contains non-printable ASCII"
            )));
        }

        let marker = bytes[offset + 0x13];
        if marker != 0xff {
            return Err(CollisionParseError::new(format!(
                "HKX section {index} has unsupported marker {marker:#04x}; locally observed Sonic Unleashed packfiles use 0xff"
            )));
        }

        let absolute_data_start = read_u32_be(bytes, offset + 0x14)?;
        let boundaries = [
            read_u32_be(bytes, offset + 0x18)?,
            read_u32_be(bytes, offset + 0x1c)?,
            read_u32_be(bytes, offset + 0x20)?,
            read_u32_be(bytes, offset + 0x24)?,
            read_u32_be(bytes, offset + 0x28)?,
            read_u32_be(bytes, offset + 0x2c)?,
        ];

        if boundaries.windows(2).any(|pair| pair[0] > pair[1]) {
            return Err(CollisionParseError::new(format!(
                "HKX section {index} has non-monotonic relative boundaries: {boundaries:?}"
            )));
        }

        let absolute_start = usize::try_from(absolute_data_start)
            .map_err(|_| CollisionParseError::new("HKX section start does not fit usize"))?;
        let relative_end = usize::try_from(boundaries[5])
            .map_err(|_| CollisionParseError::new("HKX section end does not fit usize"))?;
        let absolute_end = absolute_start
            .checked_add(relative_end)
            .ok_or_else(|| CollisionParseError::new("HKX section end overflow"))?;

        if absolute_start < directory_end {
            return Err(CollisionParseError::new(format!(
                "HKX section {index} starts at {absolute_start:#x}, inside header/directory ending at {directory_end:#x}"
            )));
        }
        if absolute_start < previous_end {
            return Err(CollisionParseError::new(format!(
                "HKX section {index} starts at {absolute_start:#x}, overlapping previous range ending at {previous_end:#x}"
            )));
        }
        if absolute_end > bytes.len() {
            return Err(CollisionParseError::new(format!(
                "HKX section {index} ends at {absolute_end:#x}, past file size {:#x}",
                bytes.len()
            )));
        }

        sections.push(PackfileSection {
            tag,
            marker,
            absolute_data_start,
            boundaries,
            data: &bytes[absolute_start..absolute_end],
        });

        previous_end = absolute_end;
    }

    Ok(HavokPackfile { header, sections })
}

fn read_u32_be(bytes: &[u8], offset: usize) -> Result<u32, CollisionParseError> {
    let end = offset
        .checked_add(4)
        .ok_or_else(|| CollisionParseError::new("u32 read offset overflow"))?;
    let slice = bytes
        .get(offset..end)
        .ok_or_else(|| CollisionParseError::new(format!("truncated u32 at offset {offset:#x}")))?;

    Ok(u32::from_be_bytes(slice.try_into().expect(
        "slice length was checked to be exactly four bytes",
    )))
}

#[cfg(test)]
mod tests {
    use super::*;

    const MYKONOS_FILE_LEN: usize = 0x141ad0;

    fn put_u32_be(bytes: &mut [u8], offset: usize, value: u32) {
        bytes[offset..offset + 4].copy_from_slice(&value.to_be_bytes());
    }

    fn put_section(
        bytes: &mut [u8],
        index: usize,
        tag: &[u8],
        absolute_data_start: u32,
        boundaries: [u32; 6],
    ) {
        let offset = PACKFILE_HEADER_SIZE + index * PACKFILE_SECTION_HEADER_SIZE;
        assert!(tag.len() <= PACKFILE_SECTION_TAG_LEN);
        bytes[offset..offset + tag.len()].copy_from_slice(tag);
        bytes[offset + 0x13] = 0xff;
        put_u32_be(bytes, offset + 0x14, absolute_data_start);
        for (boundary_index, value) in boundaries.into_iter().enumerate() {
            put_u32_be(bytes, offset + 0x18 + boundary_index * 4, value);
        }
    }

    fn observed_mykonos_envelope() -> Vec<u8> {
        let mut bytes = vec![0u8; MYKONOS_FILE_LEN];

        put_u32_be(&mut bytes, 0x00, HAVOK_MAGIC_0);
        put_u32_be(&mut bytes, 0x04, HAVOK_MAGIC_1);
        put_u32_be(&mut bytes, 0x0c, SONIC_UNLEASHED_FILE_VERSION);
        put_u32_be(&mut bytes, 0x10, SONIC_UNLEASHED_LAYOUT_WORD);
        put_u32_be(&mut bytes, 0x14, 3);

        // Raw observed header words 0x18..0x24.
        put_u32_be(&mut bytes, 0x18, 2);
        put_u32_be(&mut bytes, 0x1c, 0);
        put_u32_be(&mut bytes, 0x20, 0);
        put_u32_be(&mut bytes, 0x24, 0x20c);

        bytes[0x28..0x36].copy_from_slice(b"Havok-5.5.0-r1");
        put_u32_be(&mut bytes, 0x38, u32::MAX);
        put_u32_be(&mut bytes, 0x3c, u32::MAX);

        // Exact section-directory values observed in myk_col_act01.phy.hkx
        // SHA-256 dbc85eb61442171bbc15bf14cc8a28cb54852ecd6ef0407213145ffe18414a4c.
        put_section(
            &mut bytes,
            0,
            b"__classnames__",
            0x0000_00d0,
            [0x770, 0x770, 0x770, 0x770, 0x770, 0x770],
        );
        put_section(
            &mut bytes,
            1,
            b"__types__",
            0x0000_0840,
            [0x7b00, 0x9500, 0x9ad0, 0x9f20, 0x9f20, 0x9f20],
        );
        put_section(
            &mut bytes,
            2,
            b"__data__",
            0x0000_a760,
            [
                0x0013_6350,
                0x0013_68f0,
                0x0013_6e30,
                0x0013_7370,
                0x0013_7370,
                0x0013_7370,
            ],
        );

        bytes
    }

    #[test]
    fn parses_observed_mykonos_collision_packfile_envelope() {
        let bytes = observed_mykonos_envelope();
        let packfile = parse_havok_packfile(&bytes).unwrap();

        assert_eq!(packfile.header.version_string, "Havok-5.5.0-r1");
        assert_eq!(packfile.header.section_count, 3);
        assert_eq!(packfile.header.words_18_24, [2, 0, 0, 0x20c]);

        assert_eq!(packfile.sections[0].tag, "__classnames__");
        assert_eq!(packfile.sections[0].file_range(), 0xd0..0x840);

        assert_eq!(packfile.sections[1].tag, "__types__");
        assert_eq!(packfile.sections[1].file_range(), 0x840..0xa760);

        assert_eq!(packfile.sections[2].tag, "__data__");
        assert_eq!(packfile.sections[2].file_range(), 0xa760..MYKONOS_FILE_LEN);
        assert_eq!(packfile.sections[2].leading_range(), 0xa760..0x140ab0);
        assert_eq!(
            packfile.sections[2].boundary_range(0),
            Some(0x140ab0..0x141050)
        );
        assert_eq!(
            packfile.sections[2].boundary_range(1),
            Some(0x141050..0x141590)
        );
        assert_eq!(
            packfile.sections[2].boundary_range(2),
            Some(0x141590..0x141ad0)
        );
        assert_eq!(
            packfile.sections[2].boundary_range(3),
            Some(0x141ad0..0x141ad0)
        );
        assert_eq!(
            packfile.sections[2].boundary_range(4),
            Some(0x141ad0..0x141ad0)
        );
        assert_eq!(packfile.sections[2].boundary_range(5), None);
    }

    #[test]
    fn rejects_non_monotonic_section_boundaries() {
        let mut bytes = observed_mykonos_envelope();
        let section_1 = PACKFILE_HEADER_SIZE + PACKFILE_SECTION_HEADER_SIZE;
        put_u32_be(&mut bytes, section_1 + 0x18, 0x9600);
        put_u32_be(&mut bytes, section_1 + 0x1c, 0x9500);

        let error = parse_havok_packfile(&bytes).unwrap_err();
        assert!(error.to_string().contains("non-monotonic"));
    }

    #[test]
    fn rejects_section_extending_past_eof() {
        let mut bytes = observed_mykonos_envelope();
        let section_2 = PACKFILE_HEADER_SIZE + 2 * PACKFILE_SECTION_HEADER_SIZE;
        put_u32_be(&mut bytes, section_2 + 0x2c, 0x0013_7380);

        let error = parse_havok_packfile(&bytes).unwrap_err();
        assert!(error.to_string().contains("past file size"));
    }

    #[test]
    fn rejects_unproven_layout_word() {
        let mut bytes = observed_mykonos_envelope();
        put_u32_be(&mut bytes, 0x10, 0x0800_0001);

        let error = parse_havok_packfile(&bytes).unwrap_err();
        assert!(error.to_string().contains("unsupported HKX layout word"));
    }

    #[test]
    fn rejects_truncated_section_directory() {
        let mut bytes = observed_mykonos_envelope();
        put_u32_be(&mut bytes, 0x14, 0x1000);

        let error = parse_havok_packfile(&bytes[..0x100]).unwrap_err();
        assert!(error.to_string().contains("section directory"));
    }
}
