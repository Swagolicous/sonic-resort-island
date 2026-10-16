//! Big-endian reader for Hedgehog Engine ("Mirage") resource files.
//!
//! Every Mirage resource observed in Sonic Unleashed (Xbox 360) starts with a
//! 0x18-byte header:
//!
//! ```text
//! +0x00 file size        +0x04 root version
//! +0x08 data size        +0x0C data offset (always 0x18)
//! +0x10 fixup table      +0x14 end of fixup table
//! ```
//!
//! All offsets stored inside the data region are relative to the data offset.

use anyhow::{Context, Result, bail};

#[derive(Clone, Copy)]
pub struct MirageReader<'a> {
    bytes: &'a [u8],
    data: usize,
    pub version: u32,
}

impl<'a> MirageReader<'a> {
    pub fn new(bytes: &'a [u8]) -> Result<Self> {
        if bytes.len() < 0x18 {
            bail!("Mirage resource is smaller than its 0x18-byte header");
        }
        let declared = be_u32(bytes, 0)? as usize;
        if declared != bytes.len() {
            bail!(
                "Mirage header declares {declared:#x} bytes, file has {:#x}",
                bytes.len()
            );
        }
        let version = be_u32(bytes, 4)?;
        let data = be_u32(bytes, 0x0C)? as usize;
        if data != 0x18 {
            bail!("unexpected Mirage data offset {data:#x}");
        }
        Ok(Self {
            bytes,
            data,
            version,
        })
    }

    pub fn u32(&self, rel: usize) -> Result<u32> {
        be_u32(self.bytes, self.data + rel)
    }

    pub fn i32(&self, rel: usize) -> Result<i32> {
        Ok(self.u32(rel)? as i32)
    }

    pub fn f32(&self, rel: usize) -> Result<f32> {
        Ok(f32::from_bits(self.u32(rel)?))
    }

    pub fn u8(&self, rel: usize) -> Result<u8> {
        self.bytes
            .get(self.data + rel)
            .copied()
            .context("Mirage read past end of file")
    }

    /// Reads a data-relative offset stored at `rel`.
    pub fn ptr(&self, rel: usize) -> Result<usize> {
        Ok(self.u32(rel)? as usize)
    }

    pub fn bytes(&self, rel: usize, len: usize) -> Result<&'a [u8]> {
        let start = self.data + rel;
        self.bytes
            .get(start..start + len)
            .with_context(|| format!("Mirage range {rel:#x}+{len:#x} out of bounds"))
    }

    /// NUL-terminated string at a data-relative offset.
    pub fn str_at(&self, rel: usize) -> Result<&'a str> {
        let start = self.data + rel;
        let tail = self
            .bytes
            .get(start..)
            .context("Mirage string offset out of bounds")?;
        let end = tail
            .iter()
            .position(|b| *b == 0)
            .context("unterminated Mirage string")?;
        std::str::from_utf8(&tail[..end]).context("Mirage string is not UTF-8")
    }

    /// Like [`Self::str_at`] but tolerates non-UTF-8 (e.g. Shift-JIS) names.
    pub fn string_lossy_at(&self, rel: usize) -> Result<String> {
        let start = self.data + rel;
        let tail = self
            .bytes
            .get(start..)
            .context("Mirage string offset out of bounds")?;
        let end = tail
            .iter()
            .position(|b| *b == 0)
            .context("unterminated Mirage string")?;
        Ok(String::from_utf8_lossy(&tail[..end]).into_owned())
    }

    /// String referenced by the offset stored at `rel`.
    pub fn str_ptr(&self, rel: usize) -> Result<&'a str> {
        self.str_at(self.ptr(rel)?)
    }

    pub fn matrix(&self, rel: usize) -> Result<[f32; 16]> {
        let mut m = [0.0; 16];
        for (i, v) in m.iter_mut().enumerate() {
            *v = self.f32(rel + i * 4)?;
        }
        Ok(m)
    }
}

pub(crate) fn be_u32(bytes: &[u8], offset: usize) -> Result<u32> {
    let slice = bytes
        .get(offset..offset + 4)
        .with_context(|| format!("read u32 at {offset:#x} past end ({:#x})", bytes.len()))?;
    Ok(u32::from_be_bytes(slice.try_into().unwrap()))
}

pub(crate) fn be_u16(bytes: &[u8], offset: usize) -> Result<u16> {
    let slice = bytes
        .get(offset..offset + 2)
        .with_context(|| format!("read u16 at {offset:#x} past end ({:#x})", bytes.len()))?;
    Ok(u16::from_be_bytes(slice.try_into().unwrap()))
}

pub(crate) fn half_to_f32(h: u16) -> f32 {
    let sign = ((h >> 15) & 1) as u32;
    let exp = ((h >> 10) & 0x1F) as u32;
    let frac = (h & 0x3FF) as u32;
    let bits = match (exp, frac) {
        (0, 0) => sign << 31,
        (0, _) => {
            // Subnormal: renormalise.
            let mut e = 127 - 15 + 1;
            let mut f = frac;
            while f & 0x400 == 0 {
                f <<= 1;
                e -= 1;
            }
            (sign << 31) | ((e as u32) << 23) | ((f & 0x3FF) << 13)
        }
        (0x1F, 0) => (sign << 31) | 0x7F80_0000,
        (0x1F, _) => (sign << 31) | 0x7FC0_0000,
        _ => (sign << 31) | ((exp + 127 - 15) << 23) | (frac << 13),
    };
    f32::from_bits(bits)
}

#[cfg(test)]
mod tests {
    use super::half_to_f32;

    #[test]
    fn half_floats() {
        assert_eq!(half_to_f32(0x3C00), 1.0);
        assert_eq!(half_to_f32(0xC000), -2.0);
        assert_eq!(half_to_f32(0x3800), 0.5);
        assert_eq!(half_to_f32(0x0000), 0.0);
        assert!((half_to_f32(0x0001) - 5.960_464_5e-8).abs() < 1e-12);
    }
}
