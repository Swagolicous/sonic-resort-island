//! Havok 5.5 skeletons (`.skl.hkx`) and spline-compressed skeletal
//! animations (`.anm.hkx`, `hkaSplineSkeletalAnimation`).
//!
//! Object layouts (32-bit, big-endian packfile; Havok 5.5 serialises some
//! arrays as `{ptr, count}` "simple arrays"):
//!
//! ```text
//! hkaSkeleton:  name*, parentIndices{ptr,count}<i16>, bones{ptr,count}<hkaBone*>,
//!               referencePose{ptr,count}<hkQsTransform: t4, r4, s4>
//! hkaBone:      name*, lockTranslation
//! hkaSplineSkeletalAnimation (+offset):
//!   0x08 type  0x0C duration  0x10 numTransformTracks  0x14 numFloatTracks
//!   0x18 extractedMotion*  0x1C annotations{ptr,count}
//!   0x24 numFrames  0x28 numBlocks  0x2C maxFramesPerBlock  0x30 maskAndQuantizationSize
//!   0x34 blockDuration  0x38 blockInverseDuration  0x3C frameDuration
//!   0x40 blockOffsets hkArray<u32>  0x4C floatBlockOffsets  0x70 data hkArray<u8>
//! hkaAnimationBinding: animation*, transformTrackToBoneIndices{ptr,count}<i16>
//! ```
//!
//! The spline `data` blob is little-endian regardless of the packfile byte
//! order. Per block: one 4-byte mask per track (quantization, position,
//! rotation, scale flags), float-track masks, pad to 4, then per track the
//! position / rotation / scale channels as static values or clamped
//! B-splines with u8 knot vectors. Verified against all 220 Sonic animations
//! (every block consumed exactly, every quaternion unit length).

use anyhow::{Context, Result, bail};

use crate::collision::havok::Packfile;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct QsTransform {
    pub translation: [f32; 3],
    /// Quaternion (x, y, z, w).
    pub rotation: [f32; 4],
    pub scale: [f32; 3],
}

impl QsTransform {
    pub const IDENTITY: Self = Self {
        translation: [0.0; 3],
        rotation: [0.0, 0.0, 0.0, 1.0],
        scale: [1.0; 3],
    };
}

#[derive(Debug, Clone)]
pub struct Skeleton {
    pub name: String,
    pub bones: Vec<String>,
    pub parents: Vec<i16>,
    pub reference_pose: Vec<QsTransform>,
}

pub fn parse_skeleton(bytes: &[u8]) -> Result<Skeleton> {
    let pf = Packfile::parse(bytes)?;
    let (at, _) = pf
        .objects()
        .iter()
        .find(|(_, class)| class == "hkaSkeleton")
        .context("no hkaSkeleton in file")?;
    let at = *at;
    let name = pf.ptr(at).map(|p| pf.c_str(p)).transpose()?.unwrap_or_default();
    let count = pf.u32(at + 0x08)? as usize;
    let parents_at = pf.ptr(at + 0x04).context("no parent indices")?;
    let bones_at = pf.ptr(at + 0x0C).context("no bones")?;
    let pose_at = pf.ptr(at + 0x14).context("no reference pose")?;
    if pf.u32(at + 0x10)? as usize != count || pf.u32(at + 0x18)? as usize != count {
        bail!("skeleton array counts disagree");
    }
    let mut skeleton = Skeleton {
        name,
        bones: Vec::with_capacity(count),
        parents: Vec::with_capacity(count),
        reference_pose: Vec::with_capacity(count),
    };
    for i in 0..count as u32 {
        let word = pf.u32((parents_at + i * 2) & !3)?;
        let parent = if (parents_at + i * 2) & 2 == 0 { word >> 16 } else { word & 0xFFFF };
        skeleton.parents.push(parent as u16 as i16);
        let bone = pf.ptr(bones_at + i * 4).context("dangling bone pointer")?;
        let bone_name = pf.ptr(bone).context("bone without name")?;
        skeleton.bones.push(pf.c_str(bone_name)?);
        let p = pose_at + i * 48;
        let t = pf.vec4(p)?;
        let r = pf.vec4(p + 16)?;
        let s = pf.vec4(p + 32)?;
        skeleton.reference_pose.push(QsTransform {
            translation: [t[0], t[1], t[2]],
            rotation: r,
            scale: [s[0], s[1], s[2]],
        });
    }
    Ok(skeleton)
}

#[derive(Debug, Clone)]
enum Channel<const N: usize> {
    Default,
    Static([f32; N]),
    Spline {
        degree: usize,
        knots: Vec<u8>,
        points: Vec<[f32; N]>,
    },
}

impl<const N: usize> Channel<N> {
    fn sample(&self, frame: f32, default: [f32; N]) -> [f32; N] {
        match self {
            Channel::Default => default,
            Channel::Static(v) => *v,
            Channel::Spline {
                degree,
                knots,
                points,
            } => evaluate_spline(*degree, knots, points, frame),
        }
    }
}

#[derive(Debug, Clone)]
struct Track {
    position: Channel<3>,
    rotation: Channel<4>,
    scale: Channel<3>,
}

#[derive(Debug, Clone)]
pub struct Animation {
    pub duration: f32,
    pub frame_duration: f32,
    pub num_frames: u32,
    max_frames_per_block: u32,
    /// `blocks[b][track]`
    blocks: Vec<Vec<Track>>,
    /// Bone index driven by each transform track.
    pub track_to_bone: Vec<i16>,
}

impl Animation {
    pub fn track_count(&self) -> usize {
        self.track_to_bone.len()
    }

    /// Samples every track at `time` (seconds, clamped to the clip).
    pub fn sample(&self, time: f32, out: &mut Vec<QsTransform>) {
        out.clear();
        let frame = (time.clamp(0.0, self.duration) / self.frame_duration)
            .min(self.num_frames.saturating_sub(1) as f32);
        let per_block = (self.max_frames_per_block - 1).max(1) as f32;
        let block = ((frame / per_block) as usize).min(self.blocks.len() - 1);
        let local = frame - block as f32 * per_block;
        for track in &self.blocks[block] {
            let rotation = track.rotation.sample(local, [0.0, 0.0, 0.0, 1.0]);
            let len = rotation.iter().map(|c| c * c).sum::<f32>().sqrt().max(1e-6);
            out.push(QsTransform {
                translation: track.position.sample(local, [0.0; 3]),
                rotation: rotation.map(|c| c / len),
                scale: track.scale.sample(local, [1.0; 3]),
            });
        }
    }
}

pub fn parse_animation(bytes: &[u8]) -> Result<Animation> {
    let pf = Packfile::parse(bytes)?;
    let find = |class: &str| {
        pf.objects()
            .iter()
            .find(|(_, c)| c == class)
            .map(|(o, _)| *o)
    };
    let at = find("hkaSplineSkeletalAnimation")
        .context("no hkaSplineSkeletalAnimation (other compressions are not supported)")?;
    let duration = pf.f32(at + 0x0C)?;
    let track_count = pf.u32(at + 0x10)? as usize;
    let float_count = pf.u32(at + 0x14)? as usize;
    let num_frames = pf.u32(at + 0x24)?;
    let block_count = pf.u32(at + 0x28)? as usize;
    let max_frames_per_block = pf.u32(at + 0x2C)?;
    let frame_duration = pf.f32(at + 0x3C)?;
    let offsets_at = pf.ptr(at + 0x40).context("no block offsets")?;
    if pf.u32(at + 0x44)? as usize != block_count {
        bail!("block offset count mismatch");
    }
    let data_at = pf.ptr(at + 0x70).context("no spline data")?;
    let data_len = pf.u32(at + 0x74)? as usize;
    let data = pf.raw(data_at, data_len)?;

    let mut blocks = Vec::with_capacity(block_count);
    for b in 0..block_count as u32 {
        let start = pf.u32(offsets_at + b * 4)? as usize;
        let mut reader = BlockReader { data, at: start };
        blocks.push(reader.block(track_count, float_count).with_context(|| format!("block {b}"))?);
    }

    let mut track_to_bone: Vec<i16> = (0..track_count as i16).collect();
    if let Some(binding) = find("hkaAnimationBinding") {
        if let Some(map) = pf.ptr(binding + 0x04) {
            let count = pf.u32(binding + 0x08)? as usize;
            if count == track_count {
                for (i, slot) in track_to_bone.iter_mut().enumerate() {
                    let at = map + i as u32 * 2;
                    let word = pf.u32(at & !3)?;
                    *slot = (if at & 2 == 0 { word >> 16 } else { word & 0xFFFF }) as u16 as i16;
                }
            }
        }
    }

    Ok(Animation {
        duration,
        frame_duration,
        num_frames,
        max_frames_per_block,
        blocks,
        track_to_bone,
    })
}

struct BlockReader<'a> {
    data: &'a [u8],
    at: usize,
}

const QUANT_ALIGN: [usize; 6] = [4, 1, 2, 1, 2, 4];
const QUANT_SIZE: [usize; 6] = [4, 5, 6, 3, 2, 16];

impl BlockReader<'_> {
    fn bytes(&mut self, n: usize) -> Result<&[u8]> {
        let s = self
            .data
            .get(self.at..self.at + n)
            .context("spline data truncated")?;
        self.at += n;
        Ok(s)
    }

    fn u8(&mut self) -> Result<u8> {
        Ok(self.bytes(1)?[0])
    }

    fn u16(&mut self) -> Result<u16> {
        Ok(u16::from_le_bytes(self.bytes(2)?.try_into().unwrap()))
    }

    fn f32(&mut self) -> Result<f32> {
        Ok(f32::from_le_bytes(self.bytes(4)?.try_into().unwrap()))
    }

    fn align(&mut self, to: usize) {
        self.at = self.at.div_ceil(to) * to;
    }

    fn block(&mut self, tracks: usize, floats: usize) -> Result<Vec<Track>> {
        let masks: Vec<[u8; 4]> = (0..tracks)
            .map(|_| Ok(self.bytes(4)?.try_into().unwrap()))
            .collect::<Result<_>>()?;
        self.bytes(floats)?;
        self.align(4);
        masks
            .iter()
            .map(|m| {
                let quant = m[0];
                Ok(Track {
                    position: self.vector(m[1], quant & 3, 0.0)?,
                    rotation: self.rotation(m[2], ((quant >> 2) & 0xF) as usize)?,
                    scale: self.vector(m[3], (quant >> 6) & 3, 1.0)?,
                })
            })
            .collect()
    }

    fn knots(&mut self) -> Result<(usize, usize, Vec<u8>)> {
        let items = self.u16()? as usize;
        let degree = self.u8()? as usize;
        let knots = self.bytes(items + degree + 2)?.to_vec();
        Ok((items + 1, degree, knots))
    }

    fn vector(&mut self, flags: u8, quant: u8, default: f32) -> Result<Channel<3>> {
        if flags & 0xF0 != 0 {
            let (count, degree, knots) = self.knots()?;
            self.align(4);
            let mut ranges = [None; 3];
            let mut constant = [default; 3];
            for axis in 0..3 {
                if flags & (0x10 << axis) != 0 {
                    ranges[axis] = Some((self.f32()?, self.f32()?));
                } else if flags & (1 << axis) != 0 {
                    constant[axis] = self.f32()?;
                }
            }
            let mut points = Vec::with_capacity(count);
            for _ in 0..count {
                let mut p = constant;
                for axis in 0..3 {
                    if let Some((lo, hi)) = ranges[axis] {
                        let t = if quant == 0 {
                            self.u8()? as f32 / 255.0
                        } else {
                            self.u16()? as f32 / 65535.0
                        };
                        p[axis] = lo + (hi - lo) * t;
                    }
                }
                points.push(p);
            }
            self.align(4);
            Ok(Channel::Spline {
                degree,
                knots,
                points,
            })
        } else if flags & 0x0F != 0 {
            let mut p = [default; 3];
            for (axis, v) in p.iter_mut().enumerate() {
                if flags & (1 << axis) != 0 {
                    *v = self.f32()?;
                }
            }
            Ok(Channel::Static(p))
        } else {
            Ok(Channel::Default)
        }
    }

    fn rotation(&mut self, flags: u8, quant: usize) -> Result<Channel<4>> {
        if quant >= QUANT_SIZE.len() {
            bail!("unknown rotation quantization {quant}");
        }
        let channel = if flags & 0xF0 != 0 {
            let (count, degree, knots) = self.knots()?;
            self.align(QUANT_ALIGN[quant]);
            let points = (0..count)
                .map(|_| self.quaternion(quant))
                .collect::<Result<_>>()?;
            Channel::Spline {
                degree,
                knots,
                points,
            }
        } else if flags & 0x0F != 0 {
            self.align(QUANT_ALIGN[quant]);
            Channel::Static(self.quaternion(quant)?)
        } else {
            Channel::Default
        };
        self.align(4);
        Ok(channel)
    }

    fn quaternion(&mut self, quant: usize) -> Result<[f32; 4]> {
        let b = self.bytes(QUANT_SIZE[quant])?;
        let place = |v: [f32; 3], w: f32, slot: usize| {
            let mut q = [0.0; 4];
            let mut k = 0;
            for (i, c) in q.iter_mut().enumerate() {
                if i == slot {
                    *c = w;
                } else {
                    *c = v[k];
                    k += 1;
                }
            }
            q
        };
        let rest = |v: [f32; 3]| (1.0 - v.iter().map(|c| c * c).sum::<f32>()).max(0.0).sqrt();
        Ok(match quant {
            // THREECOMP40: 3 x 12-bit components, 2-bit slot, sign bit.
            1 => {
                let mut raw = [0u8; 8];
                raw[..5].copy_from_slice(b);
                let c = u64::from_le_bytes(raw);
                const MASK: u64 = (1 << 12) - 1;
                const SCALE: f32 = 0.000_345_436;
                let v = [0, 12, 24].map(|s| (((c >> s) & MASK) as f32 - (MASK >> 1) as f32) * SCALE);
                let w = if (c >> 38) & 1 != 0 { -rest(v) } else { rest(v) };
                place(v, w, ((c >> 36) & 3) as usize)
            }
            // THREECOMP48: 3 x 15-bit components; slot and sign in the top bits.
            2 => {
                let x = u16::from_le_bytes([b[0], b[1]]);
                let y = u16::from_le_bytes([b[2], b[3]]);
                let z = u16::from_le_bytes([b[4], b[5]]);
                const MASK: u16 = (1 << 15) - 1;
                const SCALE: f32 = 0.000_043_161;
                let v = [x, y, z].map(|c| ((c & MASK) as f32 - (MASK >> 1) as f32) * SCALE);
                let slot = (((y >> 14) & 2) | ((x >> 15) & 1)) as usize;
                let w = if z >> 15 != 0 { -rest(v) } else { rest(v) };
                place(v, w, slot)
            }
            5 => [0, 4, 8, 12].map(|o| f32::from_le_bytes(b[o..o + 4].try_into().unwrap())),
            other => bail!("rotation quantization {other} not implemented"),
        })
    }
}

/// Clamped B-spline evaluation (de Boor basis), knots in frames.
fn evaluate_spline<const N: usize>(degree: usize, knots: &[u8], points: &[[f32; N]], frame: f32) -> [f32; N] {
    let n = points.len();
    if n == 0 {
        return [0.0; N];
    }
    if n == 1 || degree == 0 {
        return points[0];
    }
    let k = |i: usize| knots[i.min(knots.len() - 1)] as f32;
    let span = if frame >= k(n) {
        n - 1
    } else {
        let (mut low, mut high) = (degree, n);
        let mut mid = (low + high) / 2;
        while frame < k(mid) || frame >= k(mid + 1) {
            if frame < k(mid) {
                high = mid;
            } else {
                low = mid;
            }
            let next = (low + high) / 2;
            if next == mid {
                break;
            }
            mid = next;
        }
        mid
    };
    let mut basis = [0.0f32; 6];
    basis[0] = 1.0;
    let degree = degree.min(4);
    for i in 1..=degree {
        for j in (0..i).rev() {
            let lo = k(span - j);
            let denom = k(span + i - j) - lo;
            let a = if denom.abs() > 1e-6 { (frame - lo) / denom } else { 0.0 };
            let tmp = basis[j] * a;
            basis[j + 1] += basis[j] - tmp;
            basis[j] = tmp;
        }
    }
    let mut out = [0.0; N];
    for (i, w) in basis.iter().enumerate().take(degree + 1) {
        let p = points[span.saturating_sub(i).min(n - 1)];
        for c in 0..N {
            out[c] += p[c] * w;
        }
    }
    out
}
