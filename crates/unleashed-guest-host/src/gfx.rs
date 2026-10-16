//! The game's draw calls, captured.
//!
//! The guest draws through a small set of functions UnleashedRecomp replaces with host
//! code (sheets/host_functions.csv: LockVertexBuffer, SetTexture, DrawPrimitive, ...) and
//! through the device structure it writes shader constants into. When the lifted render
//! code runs here, this module is that host: it gives locked buffers guest memory to
//! write to, tracks the bound state, and records every draw with the vertices, constants,
//! textures, shaders and render states it was issued with. Nothing is rasterised here;
//! the host renderer (crates/unleashed-bevy) turns the records into its own draws.
//!
//! Layouts are UnleashedRecomp's (gpu/video.h): GuestDevice has its render-state setter
//! table at +64 (0x65 guest function addresses, state = index * 4), the vertex shader
//! float constants at +0x780 and the pixel shader's at +0x1780 (256 vec4 each).

use std::collections::{BTreeMap, HashMap};
use xenon_rt::*;

const DEVICE_STATE_SETTERS: u32 = 64;
const DEVICE_VERTEX_CONSTANTS: u32 = 0x780;
const DEVICE_PIXEL_CONSTANTS: u32 = 0x1780;
const DEVICE_SAMPLERS: u32 = 0x480;

/// One captured draw call.
#[derive(Clone, Debug, Default)]
pub struct Draw {
    /// D3D primitive type as the 360 numbers them (4 triangle list, 5 fan, 6 strip,
    /// 8 rect list, 13 quad list).
    pub primitive: u32,
    pub primitive_count: u32,
    pub vertex_count: u32,
    pub stride: u32,
    /// The vertices of stream 0 this draw uses, as the guest wrote them (big-endian), or
    /// empty when the buffer was filled before the snapshot (a model's mesh).
    pub vertices: Vec<u8>,
    /// For indexed draws: the indices used (16-bit), or empty when not captured.
    pub indices: Vec<u16>,
    pub indexed: bool,
    /// Guest objects bound at the draw: stream 0's vertex buffer, the index buffer, the
    /// vertex declaration, the shaders, the textures per sampler.
    pub vertex_buffer: u32,
    pub index_buffer: u32,
    pub declaration: u32,
    pub vertex_shader: u32,
    pub pixel_shader: u32,
    pub textures: [u32; 16],
    /// c0..c255 of each stage.
    pub vertex_constants: Vec<[f32; 4]>,
    pub pixel_constants: Vec<[f32; 4]>,
    /// D3DRS state -> value, as last set.
    pub render_states: BTreeMap<u32, u32>,
    /// The device's sampler state words for samplers 0..16 (GuestSamplerState: six words
    /// each, the 360's packed sampler registers).
    pub samplers: Vec<[u32; 6]>,
}

#[derive(Default)]
pub struct Gfx {
    /// Record draws (otherwise the hooks only keep the guest running).
    pub capture: bool,
    pub draws: Vec<Draw>,
    /// Hooks reached, by name (to see what a render path uses).
    pub calls: BTreeMap<&'static str, u32>,
    device: u32,
    streams: [(u32, u32, u32); 4],
    index_buffer: u32,
    declaration: u32,
    vertex_shader: u32,
    pixel_shader: u32,
    textures: [u32; 16],
    states: BTreeMap<u32, u32>,
    /// Guest memory given to a locked buffer: buffer -> (address, size).
    storage: HashMap<u32, (u32, u32)>,
    /// Render-state setter address -> state, read from the device once it is known.
    setters: HashMap<u32, u32>,
}

fn rd(base: *mut u8, a: u32) -> u32 {
    unsafe { PPC_LOAD_U32(base, a) }
}

impl Gfx {
    fn count(&mut self, name: &'static str) {
        *self.calls.entry(name).or_default() += 1;
    }

    fn see_device(&mut self, base: *mut u8, device: u32) {
        if self.device == device || device < 0x1000 {
            return;
        }
        self.device = device;
        self.setters = (0..0x65u32).map(|i| (rd(base, device + DEVICE_STATE_SETTERS + 4 * i), 4 * i)).filter(|(f, _)| *f != 0).collect();
    }

    /// A buffer's guest memory: what a lock returns. The size is the buffer's own
    /// (GuestBuffer.dataSize, a host-endian u32 at +32) when it is sane.
    fn lock(&mut self, kernel: &kernel::Kernel, base: *mut u8, buffer: u32) -> u32 {
        if let Some(&(address, _)) = self.storage.get(&buffer) {
            return address;
        }
        let stored = u32::from_le_bytes(std::array::from_fn(|i| unsafe { PPC_LOAD_U8(base, buffer + 32 + i as u32) }));
        let size = if (1..=64 << 20).contains(&stored) { stored } else { 1 << 20 };
        let address = kernel.alloc(size);
        self.storage.insert(buffer, (address, size));
        address
    }

    fn create_buffer(&mut self, kernel: &kernel::Kernel, base: *mut u8, size: u32) -> u32 {
        // A stand-in for the host's GuestBuffer: 64 zero bytes with the size where the
        // real one keeps it.
        let buffer = kernel.alloc(64);
        for i in 0..16 {
            unsafe { PPC_STORE_U32(base, buffer + 4 * i, 0) };
        }
        for (i, b) in size.to_le_bytes().into_iter().enumerate() {
            unsafe { PPC_STORE_U8(base, buffer + 32 + i as u32, b) };
        }
        unsafe { PPC_STORE_U32(base, buffer + 4, 1) }; // reference count
        let address = kernel.alloc(size.max(16));
        self.storage.insert(buffer, (address, size));
        buffer
    }

    fn draw(&mut self, base: *mut u8, primitive: u32, primitive_count: u32, first: u32, indexed: Option<(i32, u32)>, up: Option<(u32, u32)>) {
        if !self.capture {
            return;
        }
        let vertex_count = match primitive {
            1 => primitive_count,
            2 => 2 * primitive_count,
            3 => primitive_count + 1,
            4 | 8 => 3 * primitive_count,
            5 | 6 => primitive_count + 2,
            13 => 4 * primitive_count,
            _ => 3 * primitive_count,
        };
        let (buffer, offset, stride) = self.streams[0];
        let bytes = |address: u32, len: u32| -> Vec<u8> { (0..len).map(|i| unsafe { PPC_LOAD_U8(base, address + i) }).collect() };
        let mut d = Draw {
            primitive,
            primitive_count,
            vertex_count,
            stride,
            indexed: indexed.is_some(),
            vertex_buffer: buffer,
            index_buffer: self.index_buffer,
            declaration: self.declaration,
            vertex_shader: self.vertex_shader,
            pixel_shader: self.pixel_shader,
            textures: self.textures,
            render_states: self.states.clone(),
            ..Default::default()
        };
        if let Some((data, stride)) = up {
            d.stride = stride;
            d.vertex_buffer = 0;
            d.vertices = bytes(data, vertex_count * stride);
        } else if let Some((base_vertex, start_index)) = indexed {
            if let Some(&(address, size)) = self.storage.get(&self.index_buffer) {
                let n = vertex_count.min(size / 2);
                d.indices = (0..n).map(|i| unsafe { PPC_LOAD_U16(base, address + 2 * (start_index + i)) }).collect();
            }
            if let (Some(&(address, size)), false) = (self.storage.get(&buffer), d.indices.is_empty()) {
                let top = d.indices.iter().copied().max().unwrap_or(0) as u32 + 1;
                let start = address + offset + (base_vertex.max(0) as u32) * stride;
                d.vertices = bytes(start, (top * stride).min(size));
            }
        } else if let Some(&(address, size)) = self.storage.get(&buffer) {
            d.vertices = bytes(address + offset + first * stride, (vertex_count * stride).min(size));
        }
        if self.device != 0 {
            let constants = |at: u32| -> Vec<[f32; 4]> { (0..256).map(|i| std::array::from_fn(|k| f32::from_bits(rd(base, self.device + at + 16 * i + 4 * k as u32)))).collect() };
            d.vertex_constants = constants(DEVICE_VERTEX_CONSTANTS);
            d.pixel_constants = constants(DEVICE_PIXEL_CONSTANTS);
            d.samplers = (0..16).map(|i| std::array::from_fn(|k| rd(base, self.device + DEVICE_SAMPLERS + 24 * i + 4 * k as u32))).collect();
        }
        self.draws.push(d);
    }

    /// A guest function UnleashedRecomp implements in the host. True when handled.
    pub fn hook(&mut self, kernel: &kernel::Kernel, address: u32, ctx: &mut PPCContext, base: *mut u8) -> bool {
        let (a, b, c, d, e) = unsafe { (ctx.r3.u32, ctx.r4.u32, ctx.r5.u32, ctx.r6.u32, ctx.r7.u32) };
        let mut result: Option<u32> = None;
        match address {
            0x82BE_6B98 => {
                self.count("LockVertexBuffer");
                result = Some(self.lock(kernel, base, a));
            }
            0x82BE_6CA8 => {
                self.count("LockIndexBuffer");
                result = Some(self.lock(kernel, base, a));
            }
            0x82BE_6BE8 => self.count("UnlockVertexBuffer"),
            0x82BE_6CF0 => self.count("UnlockIndexBuffer"),
            0x82BE_61D0 | 0x82BE_6200 => {
                // GuestBufferDesc: format, type, usage, pool, size, fvf.
                self.count("GetBufferDesc");
                let size = self.storage.get(&a).map_or(0, |s| s.1);
                for i in 0..6 {
                    unsafe { PPC_STORE_U32(base, b + 4 * i, if i == 4 { size } else { 0 }) };
                }
            }
            0x82BE_6AD0 => {
                self.count("CreateVertexBuffer");
                result = Some(self.create_buffer(kernel, base, a));
            }
            0x82BE_6BF8 => {
                self.count("CreateIndexBuffer");
                result = Some(self.create_buffer(kernel, base, a));
            }
            0x82BE_9818 => {
                self.count("SetTexture");
                self.see_device(base, a);
                if (b as usize) < 16 {
                    self.textures[b as usize] = c;
                }
            }
            0x82BD_D0F8 => {
                self.count("SetStreamSource");
                self.see_device(base, a);
                if (b as usize) < 4 {
                    self.streams[b as usize] = (c, d, e);
                }
            }
            0x82BD_D218 => {
                self.count("SetIndices");
                self.see_device(base, a);
                self.index_buffer = b;
            }
            0x82BE_02E0 => {
                self.count("SetVertexDeclaration");
                self.see_device(base, a);
                self.declaration = b;
            }
            0x82BE_0110 => {
                self.count("SetVertexShader");
                self.see_device(base, a);
                self.vertex_shader = b;
            }
            0x82BD_FE58 => {
                self.count("SetPixelShader");
                self.see_device(base, a);
                self.pixel_shader = b;
            }
            0x82BE_5900 => {
                self.count("DrawPrimitive");
                self.see_device(base, a);
                self.draw(base, b, d, c, None, None);
            }
            0x82BE_5CF0 => {
                self.count("DrawIndexedPrimitive");
                self.see_device(base, a);
                self.draw(base, b, e, 0, Some((c as i32, d)), None);
            }
            0x82BE_52F8 => {
                self.count("DrawPrimitiveUP");
                self.see_device(base, a);
                // (device, type, primitive count, vertices, stride)
                self.draw(base, b, c, 0, None, Some((d, e)));
            }
            // State the capture does not need: targets, viewport, scissor, clears.
            0x82BD_D9F0 | 0x82BD_DD38 | 0x82BF_E4C8 | 0x82BD_D8C0 | 0x82BD_CFB0 | 0x82BF_6400 => self.count("target/viewport/clear"),
            _ => return false,
        }
        if let Some(r) = result {
            ctx.r3.u64 = r as u64;
        }
        true
    }

    /// An indirect call to an address that is not guest code: one of the device's
    /// render-state setters (device, value) if it is in the device's table.
    pub fn state_setter(&mut self, address: u32, ctx: &mut PPCContext, base: *mut u8) -> bool {
        if self.setters.is_empty() {
            // The device is the first argument of every setter.
            let device = unsafe { ctx.r3.u32 };
            if device >= 0x1000 && (0..0x65u32).any(|i| rd(base, device + DEVICE_STATE_SETTERS + 4 * i) == address) {
                self.see_device(base, device);
            }
        }
        match self.setters.get(&address) {
            Some(&state) => {
                self.states.insert(state, unsafe { ctx.r4.u32 });
                true
            }
            None => false,
        }
    }
}
