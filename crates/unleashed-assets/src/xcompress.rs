use anyhow::{Context, Result, bail};

#[allow(dead_code)]
#[path = "xcompress_lzx/lib.rs"]
mod lzx;

use lzx::{Lzxd, WindowSize};

pub const XCOMPRESS_SIGNATURE: u32 = 0x0FF5_12EE;
pub const HEADER_SIZE: usize = 0x30;
const LZX_FRAME_SIZE: usize = 32 * 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct XCompressHeader {
    pub window_size: u32,
    pub compressed_block_size: u32,
    pub uncompressed_size: u64,
    pub compressed_size: u64,
    pub uncompressed_block_size: u32,
}

impl XCompressHeader {
    pub fn parse(bytes: &[u8]) -> Result<Self> {
        if bytes.len() < HEADER_SIZE {
            bail!("XCompress header truncated");
        }

        let be_u32 =
            |off: usize| -> Result<u32> { Ok(u32::from_be_bytes(bytes[off..off + 4].try_into()?)) };
        let be_u64 =
            |off: usize| -> Result<u64> { Ok(u64::from_be_bytes(bytes[off..off + 8].try_into()?)) };

        let signature = be_u32(0x00)?;
        if signature != XCOMPRESS_SIGNATURE {
            bail!("bad XCompress signature {signature:#010x}");
        }

        Ok(Self {
            window_size: be_u32(0x10)?,
            compressed_block_size: be_u32(0x14)?,
            uncompressed_size: be_u64(0x18)?,
            compressed_size: be_u64(0x20)?,
            uncompressed_block_size: be_u32(0x28)?,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct LzxFrameHeader {
    compressed_size: usize,
    uncompressed_size: usize,
    header_size: usize,
}

fn parse_lzx_frame_header(bytes: &[u8]) -> Result<LzxFrameHeader> {
    if bytes.len() < 2 {
        bail!("XCompress LZX frame header truncated");
    }

    let first = u16::from_be_bytes([bytes[0], bytes[1]]);
    if (first & 0xFF00) == 0xFF00 {
        if bytes.len() < 5 {
            bail!("XCompress extended LZX frame header truncated");
        }

        let uncompressed_size = (((first & 0x00FF) as usize) << 8) | bytes[2] as usize;
        let compressed_size = u16::from_be_bytes([bytes[3], bytes[4]]) as usize;

        if uncompressed_size == 0 || uncompressed_size > LZX_FRAME_SIZE {
            bail!("invalid XCompress LZX frame output size {uncompressed_size}");
        }

        Ok(LzxFrameHeader {
            compressed_size,
            uncompressed_size,
            header_size: 5,
        })
    } else {
        Ok(LzxFrameHeader {
            compressed_size: first as usize,
            uncompressed_size: LZX_FRAME_SIZE,
            header_size: 2,
        })
    }
}

fn window_size(value: u32) -> Result<WindowSize> {
    match value {
        0x0000_8000 => Ok(WindowSize::KB32),
        0x0001_0000 => Ok(WindowSize::KB64),
        0x0002_0000 => Ok(WindowSize::KB128),
        0x0004_0000 => Ok(WindowSize::KB256),
        0x0008_0000 => Ok(WindowSize::KB512),
        0x0010_0000 => Ok(WindowSize::MB1),
        0x0020_0000 => Ok(WindowSize::MB2),
        _ => bail!("unsupported regular LZX window size {value:#x}"),
    }
}

/// Decompresses a Sonic Unleashed XCompress container.
///
/// The framing matches `refs/UnleashedRecomp/tools/x_decompress/x_decompress.cpp`:
/// each outer block begins with a big-endian `u32` compressed byte count and
/// contains one or more LZX frames prefixed by big-endian 16-bit lengths. A
/// `0xFFxx` frame prefix carries the final frame's explicit uncompressed size
/// in `xx` plus the following byte, then a second big-endian 16-bit compressed
/// size.
pub fn decompress(bytes: &[u8]) -> Result<Vec<u8>> {
    let header = XCompressHeader::parse(bytes)?;
    let expected_size =
        usize::try_from(header.uncompressed_size).context("XCompress output is too large")?;
    let declared_compressed_size =
        usize::try_from(header.compressed_size).context("XCompress input is too large")?;

    if header.uncompressed_block_size == 0 {
        bail!("XCompress uncompressed block size is zero");
    }
    if header.compressed_block_size < 2 {
        bail!("XCompress compressed block buffer size is invalid");
    }

    let declared_end = HEADER_SIZE
        .checked_add(declared_compressed_size)
        .context("XCompress declared size overflow")?;
    if declared_end > bytes.len() {
        bail!(
            "XCompress payload truncated: header declares {} bytes after the header, only {} available",
            declared_compressed_size,
            bytes.len().saturating_sub(HEADER_SIZE)
        );
    }

    let lzx_window = window_size(header.window_size)?;
    let mut output = Vec::with_capacity(expected_size);
    let mut outer_pos = HEADER_SIZE;
    let payload_end = declared_end;

    while output.len() < expected_size {
        if outer_pos + 4 > payload_end {
            bail!("XCompress outer block header truncated");
        }

        let outer_size = u32::from_be_bytes(
            bytes[outer_pos..outer_pos + 4]
                .try_into()
                .expect("four-byte slice"),
        ) as usize;
        outer_pos += 4;

        let outer_end = outer_pos
            .checked_add(outer_size)
            .context("XCompress outer block size overflow")?;
        if outer_end > payload_end {
            bail!("XCompress outer block exceeds declared payload");
        }

        let block_output_size = usize::min(
            header.uncompressed_block_size as usize,
            expected_size - output.len(),
        );
        let block_output_end = output
            .len()
            .checked_add(block_output_size)
            .context("XCompress output size overflow")?;

        let mut decoder = Lzxd::new(lzx_window);
        let mut frame_pos = outer_pos;

        while output.len() < block_output_end {
            let frame = parse_lzx_frame_header(&bytes[frame_pos..outer_end])?;
            frame_pos += frame.header_size;

            let frame_end = frame_pos
                .checked_add(frame.compressed_size)
                .context("XCompress LZX frame size overflow")?;
            if frame_end > outer_end {
                bail!("XCompress LZX frame exceeds outer block");
            }

            let remaining = block_output_end - output.len();
            let frame_output_size = usize::min(frame.uncompressed_size, remaining);
            let decoded = decoder
                .decompress_next(&bytes[frame_pos..frame_end], frame_output_size)
                .with_context(|| {
                    format!(
                        "LZX decode failed at input offset {frame_pos:#x} for {frame_output_size} output bytes"
                    )
                })?;

            if decoded.len() != frame_output_size {
                bail!(
                    "LZX decoder returned {} bytes, expected {}",
                    decoded.len(),
                    frame_output_size
                );
            }
            output.extend_from_slice(decoded);
            frame_pos = frame_end;
        }

        // Match the reference implementation: the outer compressed size is
        // authoritative. Any unused bytes at the end of an outer block are
        // padding/unused input and are skipped as a unit.
        outer_pos = outer_end;
    }

    if output.len() != expected_size {
        bail!(
            "XCompress decompressed size mismatch: got {}, expected {}",
            output.len(),
            expected_size
        );
    }

    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_header(
        window_size: u32,
        compressed_size: u64,
        uncompressed_size: u64,
        uncompressed_block_size: u32,
    ) -> [u8; HEADER_SIZE] {
        let mut header = [0u8; HEADER_SIZE];
        header[0x00..0x04].copy_from_slice(&XCOMPRESS_SIGNATURE.to_be_bytes());
        header[0x04..0x08].copy_from_slice(&0x0103_0000u32.to_be_bytes());
        header[0x10..0x14].copy_from_slice(&window_size.to_be_bytes());
        header[0x14..0x18].copy_from_slice(&0x0008_0000u32.to_be_bytes());
        header[0x18..0x20].copy_from_slice(&uncompressed_size.to_be_bytes());
        header[0x20..0x28].copy_from_slice(&compressed_size.to_be_bytes());
        header[0x28..0x2C].copy_from_slice(&uncompressed_block_size.to_be_bytes());
        header
    }

    #[test]
    fn parses_header_fields_big_endian() {
        let bytes = make_header(0x0002_0000, 0x1234, 0x5678, 0x9ABC);
        let header = XCompressHeader::parse(&bytes).unwrap();

        assert_eq!(
            header,
            XCompressHeader {
                window_size: 0x0002_0000,
                compressed_block_size: 0x0008_0000,
                uncompressed_size: 0x5678,
                compressed_size: 0x1234,
                uncompressed_block_size: 0x9ABC,
            }
        );
    }

    #[test]
    fn parses_normal_lzx_frame_header() {
        let frame = parse_lzx_frame_header(&[0x12, 0x34]).unwrap();
        assert_eq!(
            frame,
            LzxFrameHeader {
                compressed_size: 0x1234,
                uncompressed_size: LZX_FRAME_SIZE,
                header_size: 2,
            }
        );
    }

    #[test]
    fn parses_extended_lzx_frame_header() {
        let frame = parse_lzx_frame_header(&[0xFF, 0x09, 0xB3, 0x01, 0xE8]).unwrap();
        assert_eq!(
            frame,
            LzxFrameHeader {
                compressed_size: 0x01E8,
                uncompressed_size: 0x09B3,
                header_size: 5,
            }
        );
    }

    #[test]
    fn decompresses_extended_single_frame_container() {
        // lzxd's own known-good uncompressed LZX stream for "abc".
        let lzx = [
            0x00, 0x30, 0x30, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, 0x00,
            0x00, 0x00, b'a', b'b', b'c', 0x00,
        ];

        let outer_size = 5 + lzx.len();
        let compressed_size = 4 + outer_size;
        let mut bytes = make_header(0x0000_8000, compressed_size as u64, 3, 3).to_vec();
        bytes.extend_from_slice(&(outer_size as u32).to_be_bytes());
        bytes.extend_from_slice(&[0xFF, 0x00, 0x03]);
        bytes.extend_from_slice(&(lzx.len() as u16).to_be_bytes());
        bytes.extend_from_slice(&lzx);

        assert_eq!(decompress(&bytes).unwrap(), b"abc");
    }

    #[test]
    fn rejects_truncated_outer_block() {
        let mut bytes = make_header(0x0002_0000, 8, 1, 1).to_vec();
        bytes.extend_from_slice(&16u32.to_be_bytes());
        bytes.extend_from_slice(&[0, 0, 0, 0]);

        assert!(decompress(&bytes).is_err());
    }
}
