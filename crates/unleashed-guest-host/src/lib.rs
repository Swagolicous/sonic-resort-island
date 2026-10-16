//! Sonic, running as the game's own code: the lifted guest functions (crates/unleashed-guest)
//! stepped frame by frame on a capture-build snapshot (re/capture, guest_snapshot.h).
//!
//! Each frame writes the pad into the game's SWA::SPadState, then runs Sonic's game object
//! the way Hedgehog's update loop does: CSonic UpdateParallel (8235D338: state
//! machine, posture input and posture, body) and UpdateSerial (82302D10), with an
//! SUpdateInfo; then the player camera (CPlayer3DNormalCamera3 vf3).
//! and reads Sonic back from his character proxy and speed context (sheets/context_fields.csv).

use xenon_rt::*;

pub mod gfx;
pub mod terrain;

/// Vtables (Decomp/rtti_classes.jsonl).
pub const VT_POSTURE_3D_STANDARD: u32 = 0x8201_3390;
pub const VT_POSTURE_INPUT_3D_STANDARD: u32 = 0x8201_37F4;
pub const VT_CHARACTER_PROXY: u32 = 0x820C_0444;
pub const VT_PLAYER_3D_NORMAL_CAMERA3: u32 = 0x8202_AAA8;
/// SWA::CLightManager, primary vtable (offset 0).
pub const VT_LIGHT_MANAGER: u32 = 0x8203_AE30;
/// SWA::CRenderDirectorFxPipeline (owns the stage's scene lighting; 8258D378 samples the
/// light field through it).
pub const VT_RENDER_DIRECTOR_FX: u32 = 0x8203_9AFC;
/// SWA::CCamera (vtable at offset 0: CUpdateUnit, UpdateParallel 8246B590).
pub const VT_CAMERA: u32 = 0x8202_9104;
pub const CAMERA_UPDATE_PARALLEL: u32 = 0x8246_B590;
/// SWA::Player::CSonic, primary vtable (CUpdateUnit: dtor, ExecuteParallelJob,
/// UpdateParallel 8235D338, UpdateSerial 82302D10 shared with CPlayer, ...).
pub const VT_SONIC: u32 = 0x8201_6BF4;
pub const PLAYER_UPDATE_PARALLEL: u32 = 0x8235_D338;
pub const PLAYER_UPDATE_SERIAL: u32 = 0x8230_2D10;
/// Havok (Decomp/rtti_classes.jsonl). Layouts are Havok 5.5's, 32-bit: hkpWorldObject
/// {+8 world, +12 user data, +16 collidable {+16 shape, +44 collision filter info}},
/// hkpEntity motion at +208 {+216 type (7 = fixed), +272 position}.
pub const VT_HKP_WORLD: u32 = 0x8212_57A8;
pub const VT_HKP_RIGID_BODY: u32 = 0x8212_5110;
pub const VT_HKP_BOX_SHAPE: u32 = 0x8212_F9E4;
/// The input manager singleton sub_8234EE88 asks for the pad (lis 0x8336 + 8028).
const INPUT_MANAGER: u32 = 0x8336_1F5C;

/// SWA::SPadState button bits (api/SWA/System/PadState.h).
pub mod button {
    pub const A: u32 = 0x1;
    pub const B: u32 = 0x2;
    pub const X: u32 = 0x8;
    pub const Y: u32 = 0x10;
    pub const LEFT_BUMPER: u32 = 0x1000;
    pub const RIGHT_BUMPER: u32 = 0x2000;
    pub const LEFT_TRIGGER: u32 = 0x4000;
    pub const RIGHT_TRIGGER: u32 = 0x8000;
}

/// One frame of pad input, as SPadState holds it (sticks in [-1, 1], +y up).
#[derive(Clone, Copy, Default, Debug)]
pub struct Pad {
    pub down: u32,
    pub left: [f32; 2],
    pub right: [f32; 2],
    pub left_trigger: f32,
    pub right_trigger: f32,
}

/// The player camera after a frame (CPlayer3DNormalCamera3: +0x64 fov; +0x70/+0x90 its
/// working eye and target; +0x120/+0x130 the eye and look-at it hands on, the eye equal
/// to its collision proxy's position).
#[derive(Clone, Copy, Debug)]
pub struct Camera {
    pub fov_y: f32,
    pub eye: [f32; 3],
    pub target: [f32; 3],
}

/// The scene lights (see GuestSonic::lights).
#[derive(Clone, Copy, Debug)]
pub struct Lights {
    pub global_diffuse: [f32; 4],
    pub global_specular: [f32; 4],
    pub global_direction: [f32; 4],
    pub eye_diffuse: [f32; 4],
    pub eye_specular: [f32; 4],
    pub eye_range: [f32; 4],
    pub eye_attribute: [f32; 4],
}

/// One draw of a particle effect (GuestSonic::render_effects).
#[derive(Clone, Debug)]
pub struct EffectDraw {
    pub draw: gfx::Draw,
    /// Names found next to the holders of the bound texture (resource_names; each such
    /// search walks the whole heap, so only the texture is looked up).
    pub textures: Vec<String>,
}

/// Sonic as the game holds him after a frame.
#[derive(Clone, Copy, Debug)]
pub struct Sonic {
    /// Character proxy position (+0x120) and velocity (+0x130).
    pub position: [f32; 3],
    pub proxy_velocity: [f32; 3],
    /// Speed context +0x210 velocity and +0x440 rotation (x, y, z, w).
    pub velocity: [f32; 3],
    pub rotation: [f32; 4],
    /// The body object (speed context +0x10) the game renders Sonic with: rotation
    /// (+0x60, x y z w) and translation (+0x70, at his feet).
    pub body_rotation: [f32; 4],
    pub body_position: [f32; 3],
}

struct GuestHost {
    kernel: kernel::Kernel,
    gfx: gfx::Gfx,
    missing: std::collections::BTreeSet<u32>,
}

impl Host for GuestHost {
    fn import(&mut self, name: &'static str, ctx: &mut PPCContext, base: *mut u8) {
        if !self.kernel.import(name, ctx, base) {
            ctx.r3.u64 = 0;
        }
    }
    fn hook(&mut self, address: u32, ctx: &mut PPCContext, base: *mut u8) {
        if !self.kernel.hook(address, ctx, base) {
            assert!(self.gfx.hook(&self.kernel, address, ctx, base), "host function {address:08X} not implemented");
        }
    }
    fn missing(&mut self, address: u32, ctx: &mut PPCContext, _base: *mut u8) {
        // The graphics device's render-state setters are host functions at addresses of
        // their own (not guest code).
        if self.gfx.state_setter(address, ctx, _base) {
            return;
        }
        // Keep going (the call is skipped) and report it: sheets/lift_roots.csv needs it.
        self.missing.insert(address);
        ctx.r3.u64 = 0;
    }
}

pub struct GuestSonic {
    mem: GuestMemory,
    host: GuestHost,
    start: PPCContext,
    csr: u32,
    dt: f64,
    pub posture: u32,
    pub posture_input: u32,
    pub context: u32,
    pub proxy: u32,
    pub pad: u32,
    pub camera: Option<u32>,
    /// The game camera (SWA::CCamera) the player camera belongs to (player camera +0x08).
    pub game_camera: Option<u32>,
    /// Sonic's CPlayerSpeed game object, and a Hedgehog SUpdateInfo {f32 DeltaTime,
    /// u32 Frame, CSharedString Category (null = empty)} in guest memory.
    pub player: u32,
    info: u32,
    frame: u32,
    /// Sonic's animation pose: the hkQsTransform array (translation, rotation, scale; 48
    /// bytes per bone, parent-relative) the game's animation system writes each frame.
    pub pose_array: Option<(u32, usize)>,
    /// Every run of 40+ hkQsTransforms on the heap (address, length): pose candidates.
    pub pose_candidates: Vec<(u32, usize)>,
    /// The stage light field (82510090 through the render director), and scratch for
    /// sampling it.
    light_field: Option<u32>,
    lf_scratch: u32,
    light_manager: Option<u32>,
    hk_scratch: u32,
    /// Debugging: a guest address whose four words are recorded into `watch_log` after
    /// each call of a frame (player parallel, player serial, player camera, game camera).
    /// Run the cameras before the player in a frame (experiment switch).
    pub camera_first: bool,
    /// SWA::CRenderDirectorFxPipeline (step_effects).
    pub director: Option<u32>,
    /// SWA::CFxBoostEffect3D, the boost shockwave job of the Fx pipeline.
    pub boost_effect: Option<u32>,
    /// SWA::CSparkleManager (the particle system's game object), and whether step_effects
    /// services it.
    pub sparkle_manager: Option<u32>,
    /// Hedgehog::Sparkle::CParticleEffectManagerD3D9 (holds the live effects).
    pub particle_manager: Option<u32>,
    pub sparkle: bool,
    resource_cache: std::collections::HashMap<u32, Vec<String>>,
    /// (render info, typical renderer) for render_effects, once looked for.
    effect_render: Option<Option<(u32, u32)>>,
    /// The snapshot this was loaded from (reload starts over from it).
    snapshot_path: std::path::PathBuf,
    /// Run the engine's whole message pass each frame (every actor), not only the
    /// Sonic -> render director route.
    pub message_pass: bool,
    pub watch: Option<u32>,
    pub watch_log: Vec<[u32; 4]>,
    /// The skinning palette's buffers (address, contents after the last frame) and the
    /// one the last frame wrote (see track_skin).
    skin_runs: Vec<(u32, Vec<[f32; 16]>)>,
    skin: Option<u32>,
    /// Matrix runs of 60+ at load, with their contents (to see which ones animate).
    initial_matrix_runs: Vec<(u32, usize, Vec<[f32; 16]>)>,
}

fn rd(base: *mut u8, a: u32) -> u32 {
    unsafe { PPC_LOAD_U32(base, a) }
}
fn rf(base: *mut u8, a: u32) -> f32 {
    f32::from_bits(rd(base, a))
}

impl GuestSonic {
    /// Loads a `82346F08_pre.bin` snapshot (its r3 is Sonic's 3D standard posture).
    pub fn load(path: &std::path::Path) -> Result<Self, String> {
        let mut snap = Snapshot::open(path).map_err(|e| e.to_string())?;
        let mut mem = GuestMemory::new();
        snap.load_into(&mut mem).map_err(|e| e.to_string())?;
        let base = mem.base;
        let posture = unsafe { snap.ctx.r3.u32 };
        if rd(base, posture) != VT_POSTURE_3D_STANDARD {
            return Err(format!("r3 {posture:#010X} is not a CPlayerSpeedPosture3DStandard"));
        }
        let context = rd(base, posture + 8);
        let proxy = Self::find(base, context, VT_CHARACTER_PROXY).ok_or("no character proxy")?;
        // The posture input sits in the same posture set: the object with its vtable.
        let posture_input = Self::scan_heap(base, VT_POSTURE_INPUT_3D_STANDARD).ok_or("no CPlayerSpeedPostureInput3DStandard")?;
        let kernel = kernel::Kernel::from_memory(base).ok_or("no o1heap user heap in the snapshot")?;
        let mut s = GuestSonic {
            mem,
            host: GuestHost { kernel, gfx: Default::default(), missing: Default::default() },
            start: snap.ctx,
            csr: snap.csr,
            dt: snap.delta_time,
            posture,
            posture_input,
            context,
            proxy,
            pad: 0,
            camera: Self::scan_heap(base, VT_PLAYER_3D_NORMAL_CAMERA3),
            game_camera: None,
            player: Self::scan_heap(base, VT_SONIC).ok_or("no SWA::Player::CSonic")?,
            info: 0,
            frame: 0,
            pose_array: None,
            pose_candidates: Self::find_poses(base),
            initial_matrix_runs: Vec::new(),
            light_field: None,
            lf_scratch: 0,
            light_manager: Self::scan_heap(base, VT_LIGHT_MANAGER),
            hk_scratch: 0,
            camera_first: std::env::var_os("UNLEASHED_CAMERA_FIRST").is_some(),
            director: Self::scan_heap(base, VT_RENDER_DIRECTOR_FX),
            boost_effect: Self::scan_heap(base, 0x820C_A6C0),
            sparkle_manager: Self::scan_heap(base, 0x8207_55F0),
            particle_manager: Self::scan_heap(base, 0x8207_54BC),
            sparkle: std::env::var_os("UNLEASHED_NO_SPARKLE").is_none(),
            resource_cache: Default::default(),
            effect_render: None,
            snapshot_path: path.to_path_buf(),
            message_pass: std::env::var_os("UNLEASHED_NO_MESSAGE_PASS").is_none(),
            watch: None,
            watch_log: Vec::new(),
            skin_runs: Vec::new(),
            skin: None,
        };
        s.initial_matrix_runs = s.matrix_runs(60).into_iter().map(|(a, n)| (a, n, s.matrices(a, n))).collect();
        s.info = s.host.kernel.alloc(32); // two SUpdateInfo: the player's, the effects'
        s.lf_scratch = s.host.kernel.alloc(0x400);
        if let Some(director) = Self::scan_heap(base, VT_RENDER_DIRECTOR_FX) {
            // As CRenderDirectorFxPipeline vf5 / 8258D378 do: world = 8258C4E0(director),
            // then 82510090(&out, world) -> the stage light field.
            let out = s.lf_scratch;
            unsafe { PPC_STORE_U32(base, out, 0) };
            let world = unsafe { s.call(0x8258_C4E0, |c| c.r3.u64 = director as u64).r3.u32 };
            s.call(0x8251_0090, |c| {
                c.r3.u64 = out as u64;
                c.r4.u64 = world as u64;
            });
            let lf = rd(base, out);
            s.light_field = (lf != 0).then_some(lf);
        }
        s.game_camera = s.camera.map(|c| rd(base, c + 8)).filter(|&g| g >= 0x1000 && rd(base, g) == VT_CAMERA);
        s.pad = s.find_pad();
        set_pc_fixes(snap.pc_fixes);
        set_delta_time(snap.delta_time);
        Ok(s)
    }

    pub fn base(&self) -> *mut u8 {
        self.mem.base
    }

    /// Guest functions reached that are not lifted yet (their calls were skipped).
    pub fn missing(&self) -> &std::collections::BTreeSet<u32> {
        &self.host.missing
    }

    fn find(base: *mut u8, context: u32, vtable: u32) -> Option<u32> {
        let ok = |p: u32| p >= 0x1000 && p & 3 == 0 && rd(base, p) == vtable;
        for o in (0..0x1000).step_by(4) {
            let p = rd(base, context + o);
            if ok(p) {
                return Some(p);
            }
        }
        for o in (0..0x1000).step_by(4) {
            let q = rd(base, context + o);
            if q < 0x1000 || q & 3 != 0 {
                continue;
            }
            for o2 in (0..0x400).step_by(4) {
                let p = rd(base, q + o2);
                if ok(p) {
                    return Some(p);
                }
            }
        }
        None
    }

    /// Every run of 40+ hkQsTransforms (unit rotation, scale 1) on the user heap.
    fn find_poses(base: *mut u8) -> Vec<(u32, usize)> {
        const ONE: u32 = 0x3F80_0000;
        let is_qs = |a: u32| {
            rd(base, a + 32) == ONE && rd(base, a + 36) == ONE && rd(base, a + 40) == ONE && {
                let q: [f32; 4] = std::array::from_fn(|i| rf(base, a + 16 + 4 * i as u32));
                (q.iter().map(|x| x * x).sum::<f32>() - 1.0).abs() < 1e-3
            }
        };
        let mut found = Vec::new();
        let mut a = 0x0002_0000u32;
        while a < 0x7FEA_0000 {
            if rd(base, a + 32) == ONE && is_qs(a) {
                let mut n = 1;
                while is_qs(a + 48 * n as u32) {
                    n += 1;
                }
                if n >= 40 {
                    found.push((a, n));
                }
                a += 48 * n as u32;
            } else {
                a += 16;
            }
        }
        found
    }

    /// Runs of 4x4 row-major affine transforms (rows 0-2 w = 0, row 3 w = 1, finite)
    /// on the user heap, at least `min` long: (address, count). Sonic's model-space bone
    /// matrices are one of them, one per model bone.
    pub fn matrix_runs(&self, min: usize) -> Vec<(u32, usize)> {
        let b = self.mem.base;
        let ok = |a: u32| {
            (rf(b, a + 60) - 1.0).abs() < 1e-4 && rf(b, a + 12).abs() < 1e-4 && rf(b, a + 28).abs() < 1e-4 && rf(b, a + 44).abs() < 1e-4 && {
                let r: [f32; 12] = std::array::from_fn(|i| rf(b, a + 4 * (i + i / 3) as u32));
                r.iter().all(|x| x.is_finite() && x.abs() < 1e4) && r[..3].iter().any(|&x| x != 0.0)
            }
        };
        let mut out = Vec::new();
        let mut a = 0x0002_0000u32;
        while a < 0x7FEA_0000 {
            if (rf(b, a + 60) - 1.0).abs() < 1e-4 && ok(a) {
                let mut n = 1;
                while ok(a + 64 * n as u32) {
                    n += 1;
                }
                if n >= min {
                    out.push((a, n));
                    a += 64 * n as u32;
                } else {
                    a += 16; // a short run may start 16 bytes off a real one
                }
            } else {
                a += 16;
            }
        }
        out
    }

    /// The runs of exactly `count` matrices whose contents changed since load: the
    /// per-frame model-space pose of a `count`-bone model the game is animating.
    pub fn animated_matrix_runs(&self, count: usize) -> Vec<u32> {
        self.initial_matrix_runs
            .iter()
            .filter(|(a, n, m)| *n == count && self.matrices(*a, *n) != *m)
            .map(|(a, _, _)| *a)
            .collect()
    }

    /// `count` row-major 4x4 matrices at `address` (16 floats each, as in guest memory).
    pub fn matrices(&self, address: u32, count: usize) -> Vec<[f32; 16]> {
        let b = self.mem.base;
        (0..count).map(|i| std::array::from_fn(|k| rf(b, address + 64 * i as u32 + 4 * k as u32))).collect()
    }

    /// The bone names of the game's skeleton (an hkaSkeleton's {char* name, bool} array)
    /// whose first bone is `root`, in the game's order: the order of its pose arrays.
    pub fn bone_names(&self, root: &str) -> Option<Vec<String>> {
        let b = self.mem.base;
        let cstr = |x: u32| -> String {
            (0..64).map(|i| unsafe { PPC_LOAD_U8(b, x + i) }).take_while(|&c| c != 0).map(|c| c as char).collect()
        };
        let heap = unsafe { core::slice::from_raw_parts(b.add(0x2_0000), 0x7FEA_0000 - 0x2_0000) };
        let mut key = root.as_bytes().to_vec();
        key.push(0);
        let strings: std::collections::HashSet<u32> =
            heap.windows(key.len()).enumerate().filter(|(_, w)| *w == key.as_slice()).map(|(i, _)| 0x2_0000 + i as u32).collect();
        let mut best: Option<Vec<String>> = None;
        let mut q = 0x2_0000u32;
        while q < 0x7FEA_0000 {
            if strings.contains(&rd(b, q)) {
                let names: Vec<String> = (0..256)
                    .map(|i| rd(b, q + 8 * i))
                    .take_while(|&n| (0x1000..0xF000_0000).contains(&n))
                    .map(cstr)
                    .take_while(|s| !s.is_empty() && s.chars().all(|c| c.is_ascii_graphic()))
                    .collect();
                if names.len() > best.as_ref().map_or(1, |v| v.len()) {
                    best = Some(names);
                }
            }
            q += 4;
        }
        best
    }

    /// Picks the pose array (one of `pose_candidates`) that the host matched to its skeleton.
    pub fn set_pose_array(&mut self, array: (u32, usize)) {
        self.pose_array = Some(array);
    }

    /// Bones of candidate `array` from `first` (for matching against a skeleton).
    pub fn pose_in(&self, array: (u32, usize), first: usize, bones: usize) -> Option<Vec<([f32; 3], [f32; 4], [f32; 3])>> {
        let (a, n) = array;
        if first + bones > n {
            return None;
        }
        let b = self.mem.base;
        Some((0..bones).map(|i| {
            let o = a + 48 * (first + i) as u32;
            let v = |k: u32| rf(b, o + 4 * k);
            ([v(0), v(1), v(2)], [v(4), v(5), v(6), v(7)], [v(8), v(9), v(10)])
        }).collect())
    }

    /// The game's current pose for `bones` bones starting at bone `first` of the array.
    pub fn pose(&self, first: usize, bones: usize) -> Option<Vec<([f32; 3], [f32; 4], [f32; 3])>> {
        self.pose_in(self.pose_array?, first, bones)
    }

    fn scan_heap(base: *mut u8, vtable: u32) -> Option<u32> {
        (0x0002_0000u32..0x7FEA_0000).step_by(16).find(|&p| rd(base, p) == vtable)
    }

    /// Runs one guest function with the snapshot's stack and thread registers.
    fn call(&mut self, address: u32, setup: impl FnOnce(&mut PPCContext)) -> PPCContext {
        let mut ctx = self.start;
        setup(&mut ctx);
        let base = self.mem.base;
        let csr = self.csr;
        set_guest_base(base);
        with_host(&mut self.host, || unsafe {
            #[allow(deprecated)]
            let saved = x86_64::_mm_getcsr();
            #[allow(deprecated)]
            x86_64::_mm_setcsr(csr);
            ctx.fpscr.csr = csr;
            unleashed_guest::call_indirect(&mut ctx, base, address);
            #[allow(deprecated)]
            x86_64::_mm_setcsr(saved);
        });
        ctx
    }

    /// The SPadState sub_8234EE88 reads, found the way it finds it.
    fn find_pad(&mut self) -> u32 {
        let base = self.mem.base;
        let scratch = unsafe { self.start.r1.u32 } - 0x400;
        let manager = rd(base, INPUT_MANAGER);
        self.call(0x82DF98C0, |c| {
            c.r3.u64 = scratch as u64;
            c.r4.u64 = manager as u64;
            c.r5.u64 = 0;
            c.r1.u64 = (scratch - 0x200) as u64;
        });
        let handle = rd(base, scratch);
        let pad = unsafe { self.call(0x82B4E2F0, |c| {
            c.r3.u64 = handle as u64;
            c.r1.u64 = (scratch - 0x200) as u64;
        }).r3.u32 };
        self.call(0x82DF9958, |c| {
            c.r3.u64 = scratch as u64;
            c.r1.u64 = (scratch - 0x200) as u64;
        });
        pad
    }

    /// The frame time the game's dt global held in the snapshot.
    pub fn snapshot_dt(&self) -> f64 {
        self.dt
    }

    /// One game frame with `pad`.
    pub fn step(&mut self, pad: &Pad) -> Sonic {
        let base = self.mem.base;
        let p = self.pad;
        unsafe {
            let prev = PPC_LOAD_U32(base, p);
            PPC_STORE_U32(base, p, pad.down); // DownState
            PPC_STORE_U32(base, p + 4, !pad.down); // UpState
            PPC_STORE_U32(base, p + 8, pad.down & !prev); // TappedState
            PPC_STORE_U32(base, p + 12, prev & !pad.down); // ReleasedState
            PPC_STORE_U32(base, p + 16, pad.left[0].to_bits());
            PPC_STORE_U32(base, p + 20, pad.left[1].to_bits());
            PPC_STORE_U32(base, p + 28, pad.right[0].to_bits());
            PPC_STORE_U32(base, p + 32, pad.right[1].to_bits());
            PPC_STORE_U32(base, p + 40, pad.left_trigger.to_bits());
            PPC_STORE_U32(base, p + 44, pad.right_trigger.to_bits());
        }
        // The game object update, as Hedgehog's update loop calls it each frame.
        self.frame = self.frame.wrapping_add(1);
        unsafe {
            PPC_STORE_U32(base, self.info, (self.dt as f32).to_bits());
            PPC_STORE_U32(base, self.info + 4, self.frame);
            PPC_STORE_U32(base, self.info + 8, 0);
        }
        let (player, info) = (self.player, self.info);
        self.watch_log.clear();
        if self.camera_first {
            self.step_cameras();
        }
        for f in [PLAYER_UPDATE_PARALLEL, PLAYER_UPDATE_SERIAL] {
            self.call(f, |c| {
                c.r3.u64 = player as u64;
                c.r4.u64 = info as u64;
            });
            self.record_watch();
        }
        if !self.camera_first {
            self.step_cameras();
        }
        self.step_effects();
        self.record_watch();
        // The palette buffer this frame wrote.
        for i in 0..self.skin_runs.len() {
            let (address, count) = (self.skin_runs[i].0, self.skin_runs[i].1.len());
            let now = self.matrices(address, count);
            if now != self.skin_runs[i].1 {
                self.skin = Some(address);
                self.skin_runs[i].1 = now;
            }
        }
        self.sonic()
    }

    /// The cameras' frame: SWA::CCamera UpdateParallel (and what it runs).
    fn step_cameras(&mut self) {
        let base = self.mem.base;
        let info = self.info;
        let _ = base;
        // The player camera is updated by the game camera below (SWA::CCamera's update
        // runs its active camera through a virtual call). Calling it here as well updated
        // it twice a frame, which doubled how far it leads a turn: above ~30 m/s the view
        // ran away from Sonic's heading and he circled after it (examples/camyaw.rs).
        // Only a snapshot without a game camera runs it directly.
        if let Some(cam) = self.camera.filter(|_| self.game_camera.is_none()) {
            // CPlayer3DNormalCamera3 vf3(this, dt): the game's player camera.
            let dt = self.dt;
            self.call(0x8247D458, |c| {
                c.r3.u64 = cam as u64;
                c.f1.f64 = dt;
            });
        }
        if let Some(gc) = self.game_camera {
            // SWA::CCamera UpdateParallel: the view the game renders and reads the pad against.
            self.call(CAMERA_UPDATE_PARALLEL, |c| {
                c.r3.u64 = gc as u64;
                c.r4.u64 = info as u64;
            });
        }
        self.record_watch();
    }

    /// The render director's frame, for the effects Sonic drives through it (the Fx
    /// pipeline's jobs: SWA::CFxBoostEffect3D, CFxAfterimage, ...).
    ///
    /// A message an actor sends waits in the sender's outgoing list until the engine's
    /// message pass moves it to its target (sub_82E62568 on the sender) and the target
    /// processes what it received (sub_82E601C8). The engine does that for every actor;
    /// this host does it for the one route it runs both ends of: Sonic to the render
    /// director. Then the director's update unit runs (UpdateParallel, UpdateSerial),
    /// which updates its jobs.
    fn step_effects(&mut self) {
        let Some(director) = self.director else { return };
        let base = self.mem.base;
        // Hedgehog runs its update units once per update category, and the Fx jobs act
        // only in category "0" (sub_82BA7308 compares SUpdateInfo's category with the
        // string the CSharedString at 0x8328E950 holds): the effects get an SUpdateInfo
        // with that category.
        let info = self.info + 16;
        unsafe {
            PPC_STORE_U32(base, info, (self.dt as f32).to_bits());
            PPC_STORE_U32(base, info + 4, self.frame);
            PPC_STORE_U32(base, info + 8, rd(base, 0x8328_E950));
        }
        let player = self.player;
        self.refresh_effect_nodes();
        if self.message_pass {
            self.message_pass(info);
        } else {
            self.call(0x82E6_2568, |c| c.r3.u64 = (player + 40) as u64);
            self.call(0x82E6_01C8, |c| c.r3.u64 = (director + 44) as u64);
        }
        for slot in [8, 12] {
            let function = rd(base, rd(base, director + 4) + slot);
            self.call(function, |c| {
                c.r3.u64 = (director + 4) as u64;
                c.r4.u64 = info as u64;
            });
        }
        // The particle system: SWA::CSparkleManager's message actor (+40) receives what
        // Sonic's code asks for (effects by name: sn_spin, sn_boost, ...), and its update
        // unit advances every live effect.
        if let Some(manager) = self.sparkle_manager.filter(|_| self.sparkle) {
            if !self.message_pass {
                self.call(0x82E6_01C8, |c| c.r3.u64 = (manager + 40) as u64);
            }
            for slot in [8, 12] {
                let function = rd(base, rd(base, manager) + slot);
                self.call(function, |c| {
                    c.r3.u64 = manager as u64;
                    c.r4.u64 = info as u64;
                });
            }
        }
        // The boost effect job's own update (its vtable slot 4, sub_82BA7308: the ring's
        // radius grows by dt, its state machine runs, its centre follows Sonic on screen).
        if let Some(fx) = self.boost_effect {
            let update = rd(base, rd(base, fx) + 16);
            self.call(update, |c| {
                c.r3.u64 = fx as u64;
                c.r4.u64 = info as u64;
            });
        }
    }

    /// Live particle effect instances: the elements of the two lists the particle
    /// manager (Hedgehog::Sparkle::CParticleEffectManagerD3D9) draws from, at +60 and +72
    /// (sub_82E90E30 walks them: node +0 next, +8 the effect instance pointer).
    pub fn sparkle_effects(&self) -> Vec<u32> {
        let b = self.mem.base;
        let Some(manager) = self.particle_manager else { return Vec::new() };
        let mut out = Vec::new();
        for list in [60, 72] {
            let head = rd(b, manager + list);
            let mut node = rd(b, head);
            while node != head && node >= 0x1000 && out.len() < 4096 {
                out.push(rd(b, node + 8));
                node = rd(b, node);
            }
        }
        out
    }

    /// The emitters of a live effect instance and, for each, its particle objects, as
    /// sub_82E8FF98 walks them: sub_82E88150(effect) returns a block whose +4 is a list
    /// head (walked through +4, element at +8); an emitter's +268 is the list of its
    /// particle objects (same shape).
    pub fn sparkle_emitters(&mut self, effect: u32) -> Vec<(u32, Vec<u32>)> {
        let b = self.mem.base;
        let block = unsafe { self.call(0x82E8_8150, |c| c.r3.u64 = effect as u64).r3.u32 };
        if block < 0x1000 {
            return Vec::new();
        }
        let walk = |head: u32| -> Vec<u32> {
            let mut out = Vec::new();
            if head < 0x1000 {
                return out;
            }
            let mut node = rd(b, head + 4);
            while node != head && node >= 0x1000 && out.len() < 1024 {
                out.push(rd(b, node + 8));
                node = rd(b, node + 4);
            }
            out
        };
        walk(rd(b, block + 4)).into_iter().map(|e| (e, walk(rd(b, e + 268)))).collect()
    }

    /// The boost shockwave's pixel constants for this frame, as SWA::CFxBoostEffect3D's
    /// render routine (sub_82BA6870) uploads them for the FxBoostBlur program, or None
    /// while the job does not draw (its +105 flag, and the tunable enable at 0x83302DC0):
    ///   g_Param  = (job +0x70, tunable, tunable, 0)
    ///   g_Param2 = (job +0x7C, job +0x80: the centre; tunable; job +0x78: the radius)
    ///   g_Param3 = (tunable width, tunable strength, 0, 0)
    /// with the tunables at 0x83302DC4.. (0.5, 2.0, 1.0, 0.06, 0.09 in the retail game).
    pub fn boost_shockwave(&self) -> Option<[[f32; 4]; 3]> {
        const TUNABLES: u32 = 0x8330_2DC0;
        let (b, fx) = (self.mem.base, self.boost_effect?);
        let drawing = unsafe { PPC_LOAD_U8(b, fx + 105) != 0 && PPC_LOAD_U8(b, TUNABLES) != 0 };
        let (job, tunable) = (|o: u32| rf(b, fx + o), |o: u32| rf(b, TUNABLES + o));
        drawing.then(|| {
            [
                [job(0x70), tunable(8), tunable(12), 0.0],
                [job(0x7C), job(0x80), tunable(4), job(0x78)],
                [tunable(16), tunable(20), 0.0, 0.0],
            ]
        })
    }

    /// The engine's message pass, as its frame function runs it (sub_824EC080): twice,
    /// every actor with messages to send hands them to their targets (sub_82E6A340),
    /// every actor that received any processes them (sub_82E69B50; delayed messages
    /// count down by the dt at `info`), and the manager tidies its lists (sub_82E6B548).
    /// The message manager is the object behind the handle every actor holds at +112.
    fn message_pass(&mut self, info: u32) {
        let base = self.mem.base;
        let scratch = unsafe { self.start.r1.u32 } - 0x400;
        let handle = rd(base, self.player + 40 + 112);
        self.call(0x82DF_98C0, |c| {
            c.r3.u64 = scratch as u64;
            c.r4.u64 = handle as u64;
            c.r5.u64 = 0;
            c.r1.u64 = (scratch - 0x200) as u64;
        });
        let manager = rd(base, scratch);
        for _ in 0..2 {
            self.call(0x82E6_A340, |c| {
                c.r3.u64 = manager as u64;
                c.r1.u64 = (scratch - 0x200) as u64;
            });
            for function in [0x82E6_9B50u32, 0x82E6_B548] {
                self.call(function, |c| {
                    c.r3.u64 = manager as u64;
                    c.r4.u64 = info as u64;
                    c.r1.u64 = (scratch - 0x200) as u64;
                });
            }
        }
        self.call(0x82DF_9958, |c| {
            c.r3.u64 = scratch as u64;
            c.r1.u64 = (scratch - 0x200) as u64;
        });
    }

    /// Brings the matrix nodes live effects hang from up to date with Sonic. Each effect
    /// follows a matrix node (effect +0xF0); between it and Sonic's body node (speed
    /// context +0x10) sit nodes his own update does not refresh (the engine does that
    /// when it updates his model). For every live effect this calls each node's
    /// UpdateMatrix (vtable slot 1: node, parent) from the body node down
    /// (examples/nodes.rs).
    fn refresh_effect_nodes(&mut self) {
        let base = self.mem.base;
        let body = rd(base, self.context + 0x10);
        let heap = |p: u32| (0x2_0000..0x7FEA_0000).contains(&p) && p & 3 == 0;
        let mut done: Vec<u32> = Vec::new();
        for effect in self.sparkle_effects() {
            let mut chain = vec![rd(base, effect + 0xF0)];
            while let Some(&n) = chain.last() {
                if !heap(n) || n == body || chain.len() > 16 {
                    break;
                }
                chain.push(rd(base, n + 4));
            }
            if chain.last() != Some(&body) {
                continue; // not one of Sonic's nodes
            }
            for w in chain.windows(2).rev() {
                let (node, parent) = (w[0], w[1]);
                if done.contains(&node) {
                    continue;
                }
                done.push(node);
                let update = rd(base, rd(base, node) + 4);
                self.call(update, |c| {
                    c.r3.u64 = node as u64;
                    c.r4.u64 = parent as u64;
                });
            }
        }
    }

    fn record_watch(&mut self) {
        if let Some(a) = self.watch {
            let b = self.mem.base;
            self.watch_log.push(std::array::from_fn(|i| rd(b, a + 4 * i as u32)));
        }
    }

    /// Starts following the skinning palette of a `count`-bone model. The game keeps it
    /// in two buffers and writes them on alternate frames (examples/animrate.rs), so
    /// reading one of them shows the animation at half rate; `skin` is the buffer the
    /// last frame wrote.
    pub fn track_skin(&mut self, count: usize) {
        if self.skin_runs.is_empty() {
            self.skin_runs = self.animated_matrix_runs(count).into_iter().map(|a| (a, self.matrices(a, count))).collect();
        }
    }

    pub fn skin(&self) -> Option<u32> {
        self.skin
    }

    /// The hkpWorld Sonic's character proxy lives in: the world (hkpWorldObject +8) of the
    /// proxy's phantom.
    pub fn havok_world(&self) -> Option<u32> {
        let b = self.mem.base;
        (0..0x400).step_by(4).find_map(|o| {
            let q = rd(b, self.proxy + o);
            let heap = |p: u32| (0x2_0000..0x7FEA_0000).contains(&p) && p & 3 == 0;
            (heap(q) && heap(rd(b, q + 8)) && rd(b, rd(b, q + 8)) == VT_HKP_WORLD).then(|| rd(b, q + 8))
        })
    }

    /// Every hkpRigidBody of `world`: (body, motion type, collision filter info, shape).
    pub fn rigid_bodies(&self, world: u32) -> Vec<(u32, u8, u32, u32)> {
        let b = self.mem.base;
        (0x0002_0000u32..0x7FEA_0000)
            .step_by(16)
            .filter(|&p| rd(b, p) == VT_HKP_RIGID_BODY && rd(b, p + 8) == world)
            .map(|p| (p, unsafe { PPC_LOAD_U8(b, p + 216) }, rd(b, p + 44), rd(b, p + 16)))
            .collect()
    }

    /// What the HUD shows, from Sonic's speed context: the ring count (+0x538) and the
    /// ring energy (boost) gauge (+0x53C, 0..100). Found by behaviour (examples/hudscan.rs):
    /// the gauge is the one float on the heap that drains while X is held and holds
    /// exactly still otherwise.
    pub fn hud(&self) -> (u32, f32) {
        let b = self.mem.base;
        (rd(b, self.context + 0x538), rf(b, self.context + 0x53C))
    }

    /// Guest memory from the game's user heap (for blocks the host hands to guest code).
    pub fn guest_alloc(&mut self, size: u32) -> u32 {
        (self.host.kernel.alloc(size + 16) + 15) & !15
    }

    /// The name of the game resource a guest graphics object belongs to (a texture, a
    /// shader, a vertex buffer): the engine's data object that holds the pointer keeps
    /// its name as a string pointer a few words before or after it. Returns every
    /// distinct name found next to a holder of `pointer` (the caller picks by kind).
    pub fn resource_names(&mut self, pointer: u32) -> Vec<String> {
        if let Some(names) = self.resource_cache.get(&pointer) {
            return names.clone();
        }
        let b = self.mem.base;
        let text = |a: u32| -> Option<String> {
            if !(0x1_0000..0xF000_0000).contains(&a) {
                return None;
            }
            let s: String = (0..96).map(|i| unsafe { PPC_LOAD_U8(b, a + i) }).take_while(|&c| c != 0).map(|c| c as char).collect();
            (s.len() >= 4 && s.len() < 96 && s.chars().all(|c| c.is_ascii_alphanumeric() || "_-.[]@ #".contains(c))).then_some(s)
        };
        let mut names = Vec::new();
        let heap = unsafe { core::slice::from_raw_parts(b.add(0x2_0000), 0x7FEA_0000 - 0x2_0000) };
        // SIMD byte search finds the same aligned big-endian pointer holders
        // as the scalar word loop, without half a billion per-word loads.
        if pointer != 0 {
            let needle = pointer.to_be_bytes();
            let finder = memchr::memmem::Finder::new(&needle);
            let mut cursor = 0;
            while let Some(relative) = finder.find(&heap[cursor..]) {
                let offset = cursor + relative;
                // An unaligned match can overlap an aligned one when the
                // pointer has repeated bytes. Do not skip that next holder.
                cursor = offset + if offset & 3 == 0 { 4 } else { 1 };
                if offset & 3 != 0 { continue; }
                let p = 0x2_0000 + offset as u32;
                for o in (0..0x60u32).step_by(4) {
                    for q in [p.wrapping_sub(o), p + o] {
                        if let Some(s) = text(rd(b, q)) {
                            if !names.contains(&s) {
                                names.push(s);
                            }
                        }
                    }
                }
            }
        }
        self.resource_cache.insert(pointer, names.clone());
        names
    }

    /// Starts over from the snapshot (a respawn: Sonic, the camera and the whole game
    /// state are as captured again).
    pub fn reload(&mut self) -> Result<(), String> {
        let path = self.snapshot_path.clone();
        let fresh = Self::load(&path)?;
        *self = fresh;
        Ok(())
    }

    /// This frame's particle effects as the game draws them: runs the game's particle
    /// renderer (SWA::CSparkleManager::CTypicalRenderer::Render, sub_827D9358) for its
    /// three passes ("opaque", "punchthrough", "transparent") with the game's own render
    /// info, and returns the draws it issued (gfx.rs), each with the names found for its
    /// texture (resource_names: candidates, the real name is a suffix).
    ///
    /// The render info is the block the game keeps for its scene pass: {rendering
    /// infrastructure, rendering device wrapper, the game camera's Mirage camera block
    /// (game camera + 0x120)}; it is found once by its camera and infrastructure words.
    pub fn render_effects(&mut self) -> Vec<EffectDraw> {
        let b = self.mem.base;
        let (Some(manager), Some(camera)) = (self.particle_manager, self.game_camera) else { return Vec::new() };
        if self.sparkle_effects().is_empty() {
            return Vec::new();
        }
        if self.effect_render.is_none() {
            let infrastructure = rd(b, manager + 16);
            let mirage_camera = camera + 0x120;
            let mut info = None;
            let mut renderer = None;
            let mut p = 0x0002_0000u32;
            while p < 0x7FEA_0000 && (info.is_none() || renderer.is_none()) {
                let w = rd(b, p);
                if w == infrastructure && rd(b, p + 8) == mirage_camera && info.is_none() {
                    info = Some(p);
                }
                if w == 0x8207_561C && rd(b, p + 8) == manager && renderer.is_none() {
                    renderer = Some(p);
                }
                p += 4;
            }
            self.effect_render = Some(info.zip(renderer));
        }
        let Some(Some((info, renderer))) = self.effect_render else { return Vec::new() };
        self.host.gfx.draws.clear();
        self.host.gfx.capture = true;
        for symbol in [0x8336_9060u32, 0x8336_905C, 0x8336_9058] {
            let category = rd(b, symbol);
            self.call(0x827D_9358, |c| {
                c.r3.u64 = renderer as u64;
                c.r4.u64 = info as u64;
                c.r5.u64 = category as u64;
                c.r6.u64 = 0;
            });
        }
        self.host.gfx.capture = false;
        let draws = std::mem::take(&mut self.host.gfx.draws);
        draws
            .into_iter()
            .map(|draw| {
                let textures = self.resource_names(draw.textures[0]);
                EffectDraw { draw, textures }
            })
            .collect()
    }

    /// The capture of the game's draw calls (gfx.rs).
    pub fn gfx(&mut self) -> &mut gfx::Gfx {
        &mut self.host.gfx
    }

    /// Runs a guest function with integer arguments in r3.. (experiments and host glue);
    /// returns r3.
    pub fn guest_call(&mut self, address: u32, args: &[u32]) -> u32 {
        let args = args.to_vec();
        unsafe {
            self.call(address, |c| {
                let regs = [&mut c.r3, &mut c.r4, &mut c.r5, &mut c.r6];
                for (r, a) in regs.into_iter().zip(args) {
                    r.u64 = a as u64;
                }
            })
            .r3
            .u32
        }
    }

    /// The first object on the heap with `vtable` in its first word.
    pub fn find_object(&self, vtable: u32) -> Option<u32> {
        Self::scan_heap(self.mem.base, vtable)
    }

    /// The SUpdateInfo this host passes to update calls (dt, frame, category).
    pub fn update_info(&self) -> u32 {
        self.info
    }

    /// The game document's running scores (CGameDocument at *0x83367900, member +0x1D8
    /// SScoreInfo: enemy score +0x34, trick score +0x38; api/SWA/System/GameDocument.h).
    pub fn scores(&self) -> (u32, u32) {
        let b = self.mem.base;
        let document = rd(b, 0x8336_7900);
        if document < 0x1000 {
            return (0, 0);
        }
        let info = rd(b, document + 8) + 0x1D8;
        (rd(b, info + 0x34), rd(b, info + 0x38))
    }

    /// A flag of the speed context's flag set, as sub_82333D78 tests it: the set is the
    /// object at *(context + 0x4AC), its bytes (one per flag) are at *(set + 4); indices
    /// in sheets/context_flags.csv.
    pub fn flag(&self, index: u32) -> bool {
        let b = self.mem.base;
        let set = rd(b, self.context + 0x4AC);
        if set < 0x1000 {
            return false;
        }
        let flags = rd(b, set + 4);
        flags >= 0x1000 && unsafe { PPC_LOAD_U8(b, flags + index) } != 0
    }

    /// Sonic is boosting (flag 14).
    pub fn boosting(&self) -> bool {
        self.flag(14)
    }

    /// A rigid body's position (its motion state's translation, +272).
    pub fn rigid_body_position(&self, body: u32) -> [f32; 3] {
        std::array::from_fn(|i| rf(self.mem.base, body + 272 + 4 * i as u32))
    }

    /// Havok memory as the game takes it: hkThreadMemory::allocateChunk (82EA8A30) on the
    /// thread memory at *(r13[0] + 20), with the size in the object's memSizeAndFlags.
    fn hk_alloc(&mut self, size: u32) -> u32 {
        let b = self.mem.base;
        let thread_memory = rd(b, rd(b, unsafe { self.start.r13.u32 }) + 20);
        let p = unsafe {
            self.call(0x82EA_8A30, |c| {
                c.r3.u64 = thread_memory as u64;
                c.r4.u64 = size as u64;
            })
            .r3
            .u32
        };
        unsafe { PPC_STORE_U16(b, p + 4, size as u16) };
        p
    }

    /// 16-aligned scratch for Havok construction infos (their constructors store with VMX).
    fn hk_scratch(&mut self) -> u32 {
        if self.hk_scratch == 0 {
            self.hk_scratch = (self.host.kernel.alloc(0x200) + 15) & !15;
        }
        self.hk_scratch
    }

    /// Runs `f` with `world` locked, as SWA's physics world brackets its hkpWorld calls
    /// (822EE630: 82ED6B88 before, 82ED5390 after).
    fn with_world_locked<T>(&mut self, world: u32, f: impl FnOnce(&mut Self) -> T) -> T {
        self.call(0x82ED_6B88, |c| c.r3.u64 = world as u64);
        let out = f(self);
        self.call(0x82ED_5390, |c| c.r3.u64 = world as u64);
        out
    }

    /// Adds a fixed box to `world`, built by the game's own Havok: hkpBoxShape (82F28538),
    /// hkpRigidBodyCinfo (82ED4D70) with the shape, position, filter info and
    /// MOTION_FIXED (7), hkpRigidBody (82ED3C60), hkpWorld::addEntity (82ED5628). The
    /// world then holds the only reference. Returns the body.
    pub fn add_fixed_box(&mut self, world: u32, center: [f32; 3], half_extents: [f32; 3], rotation: [f32; 4], radius: f32, filter_info: u32) -> u32 {
        let b = self.mem.base;
        let scratch = self.hk_scratch();
        let (extents, cinfo) = (scratch, scratch + 0x20);
        for i in 0..3 {
            unsafe { PPC_STORE_U32(b, extents + 4 * i as u32, half_extents[i].to_bits()) };
        }
        unsafe { PPC_STORE_U32(b, extents + 12, 0) };
        let shape = self.hk_alloc(48);
        self.call(0x82F2_8538, |c| {
            c.r3.u64 = shape as u64;
            c.r4.u64 = extents as u64;
            c.f1.f64 = radius as f64;
        });
        self.call(0x82ED_4D70, |c| c.r3.u64 = cinfo as u64);
        unsafe {
            PPC_STORE_U32(b, cinfo, filter_info);
            PPC_STORE_U32(b, cinfo + 4, shape);
            for i in 0..3 {
                PPC_STORE_U32(b, cinfo + 16 + 4 * i as u32, center[i].to_bits());
            }
            for i in 0..4 {
                PPC_STORE_U32(b, cinfo + 32 + 4 * i as u32, rotation[i].to_bits()); // x, y, z, w
            }
            PPC_STORE_U8(b, cinfo + 176, 7);
        }
        let memory = self.hk_alloc(512);
        let body = unsafe {
            self.call(0x82ED_3C60, |c| {
                c.r3.u64 = memory as u64;
                c.r4.u64 = cinfo as u64;
            })
            .r3
            .u32
        };
        self.with_world_locked(world, |s| {
            s.call(0x82ED_5628, |c| {
                c.r3.u64 = world as u64;
                c.r4.u64 = body as u64;
                c.r5.u64 = 1; // HK_ENTITY_ACTIVATION_DO_ACTIVATE
            });
        });
        // The body holds the shape and the world holds the body.
        for object in [shape, body] {
            self.call(0x822E_9F40, |c| c.r3.u64 = object as u64);
        }
        body
    }

    /// hkpWorld::removeEntity (82ED57C8: result, world, entity).
    pub fn remove_body(&mut self, world: u32, body: u32) {
        let result = self.hk_scratch() + 0x100;
        self.with_world_locked(world, |s| {
            s.call(0x82ED_57C8, |c| {
                c.r3.u64 = result as u64;
                c.r4.u64 = world as u64;
                c.r5.u64 = body as u64;
            });
        });
    }

    /// A course triangle in the game's Havok world. The shape layout matches
    /// hkpSimpleMeshShape::getChildShape (82F31A38): a 96-byte hkpTriangleShape,
    /// type 4, non-extruded, with its three aligned vertices at 0x20/0x30/0x40.
    /// The rigid body is built and registered by the same retail functions as boxes.
    pub fn add_fixed_triangle(&mut self, world: u32, vertices: [[f32; 3]; 3], filter_info: u32) -> u32 {
        self.add_triangle_shape(world, vertices, filter_info, 0, 6)
    }

    /// Import adjacent-face angles for Havok's original contact-normal welding.
    /// This removes internal edge contacts without changing Sonic's controller.
    pub fn add_fixed_triangle_welded(&mut self, world: u32, vertices: [[f32; 3]; 3], filter_info: u32, welding_info: u16) -> u32 {
        self.add_triangle_shape(world, vertices, filter_info, welding_info, 0)
    }

    fn add_triangle_shape(&mut self, world: u32, vertices: [[f32; 3]; 3], filter_info: u32, welding_info: u16, welding_type: u8) -> u32 {
        let b = self.mem.base;
        let center: [f32; 3] = std::array::from_fn(|k| (vertices[0][k] + vertices[1][k] + vertices[2][k]) / 3.0);
        let shape = self.hk_alloc(96);
        unsafe {
            PPC_STORE_U32(b, shape, 0x8204_7A24);
            PPC_STORE_U16(b, shape + 6, 1);
            PPC_STORE_U32(b, shape + 8, 0);
            PPC_STORE_U32(b, shape + 12, 4);
            PPC_STORE_U32(b, shape + 16, 0.02f32.to_bits());
            PPC_STORE_U32(b, shape + 20, 0);
            PPC_STORE_U16(b, shape + 20, welding_info);
            PPC_STORE_U8(b, shape + 22, welding_type);
            for v in 0..3 {
                for k in 0..3 {
                    PPC_STORE_U32(b, shape + 0x20 + v as u32 * 16 + k as u32 * 4,
                        (vertices[v][k] - center[k]).to_bits());
                }
                PPC_STORE_U32(b, shape + 0x20 + v as u32 * 16 + 12, 0);
            }
            for k in 0..4 { PPC_STORE_U32(b, shape + 0x50 + k * 4, 0); }
        }
        let cinfo = self.hk_scratch() + 0x20;
        self.call(0x82ED_4D70, |c| c.r3.u64 = cinfo as u64);
        unsafe {
            PPC_STORE_U32(b, cinfo, filter_info);
            PPC_STORE_U32(b, cinfo + 4, shape);
            for k in 0..3 { PPC_STORE_U32(b, cinfo + 16 + k as u32 * 4, center[k].to_bits()); }
            for k in 0..4 { PPC_STORE_U32(b, cinfo + 32 + k * 4, if k == 3 { 1.0f32.to_bits() } else { 0 }); }
            PPC_STORE_U8(b, cinfo + 176, 7);
        }
        let memory = self.hk_alloc(512);
        let body = unsafe { self.call(0x82ED_3C60, |c| {
            c.r3.u64 = memory as u64; c.r4.u64 = cinfo as u64;
        }).r3.u32 };
        self.with_world_locked(world, |s| { s.call(0x82ED_5628, |c| {
            c.r3.u64 = world as u64; c.r4.u64 = body as u64; c.r5.u64 = 1;
        }); });
        for object in [shape, body] { self.call(0x822E_9F40, |c| c.r3.u64 = object as u64); }
        body
    }

    /// The scene lights as SWA::CLightManager holds them (offsets matched against
    /// SceneEffect.prm.xml's Light_Manager values in a snapshot): global light diffuse,
    /// specular, direction; eye light diffuse, specular, range, attribute.
    pub fn lights(&self) -> Option<Lights> {
        let (b, m) = (self.mem.base, self.light_manager?);
        let v = |o: u32| -> [f32; 4] { std::array::from_fn(|i| rf(b, m + o + 4 * i as u32)) };
        Some(Lights {
            global_diffuse: v(0xE0),
            global_specular: v(0xF0),
            global_direction: v(0x100),
            eye_diffuse: v(0x110),
            eye_specular: v(0x120),
            eye_range: v(0x130),
            eye_attribute: v(0x140),
        })
    }

    pub fn light_field_handle(&self) -> Option<u32> {
        self.light_field
    }

    /// The game's light-field sample at `position` (82E2B8A8, called as 8258D378 calls it:
    /// position.y + the constant at 0x820CA998, f1 = f2 = the constant at 0x820008C4,
    /// r7 = 2, r9 = an 82E86528-initialised block): six RGBA colours (g_aLightField) and
    /// the shadow term.
    pub fn light_field_sample(&mut self, position: [f32; 3]) -> Option<([[f32; 4]; 6], f32)> {
        let lf = self.light_field?;
        let b = self.mem.base;
        let (scratch, out, pos, block) = (self.lf_scratch, self.lf_scratch + 0x100, self.lf_scratch + 0x200, self.lf_scratch + 0x240);
        let _ = scratch;
        let up = rf(b, 0x820C_A998);
        let k = f32::from_bits(rd(b, 0x8200_08C4)) as f64;
        unsafe {
            PPC_STORE_U32(b, pos, position[0].to_bits());
            PPC_STORE_U32(b, pos + 4, (position[1] + up).to_bits());
            PPC_STORE_U32(b, pos + 8, position[2].to_bits());
            PPC_STORE_U32(b, pos + 12, 1f32.to_bits());
        }
        self.call(0x82E8_6528, |c| c.r3.u64 = block as u64);
        self.call(0x82E2_B8A8, |c| {
            c.r3.u64 = lf as u64;
            c.r4.u64 = out as u64;
            c.r5.u64 = pos as u64;
            c.r7.u64 = 2;
            c.r9.u64 = block as u64;
            c.f1.f64 = k;
            c.f2.f64 = k;
        });
        let colours = std::array::from_fn(|i| std::array::from_fn(|j| rf(b, out + 16 * i as u32 + 4 * j as u32)));
        Some((colours, rf(b, out + 96)))
    }

    /// The view the game renders: SWA::CCamera's eye (+0x1B0), look-at (+0x240) and
    /// m_VertFieldOfView (+0x350; api/SWA/Camera/Camera.h, offsets checked in a snapshot).
    pub fn game_view(&self) -> Option<Camera> {
        let (b, c) = (self.mem.base, self.game_camera?);
        Some(Camera {
            fov_y: rf(b, c + 0x350),
            eye: [rf(b, c + 0x1B0), rf(b, c + 0x1B4), rf(b, c + 0x1B8)],
            target: [rf(b, c + 0x240), rf(b, c + 0x244), rf(b, c + 0x248)],
        })
    }

    pub fn camera(&self) -> Option<Camera> {
        let (b, c) = (self.mem.base, self.camera?);
        Some(Camera {
            fov_y: rf(b, c + 0x64),
            eye: [rf(b, c + 0x70), rf(b, c + 0x74), rf(b, c + 0x78)],
            target: [rf(b, c + 0x90), rf(b, c + 0x94), rf(b, c + 0x98)],
        })
    }

    /// Sonic's world matrix as the game renders his model: the body object's (speed
    /// context +0x10) row-major matrix at +0x80 (rows: x, y, z axes, then position).
    pub fn body_matrix(&self) -> [f32; 16] {
        let b = self.mem.base;
        let o = rd(b, self.context + 0x10) + 0x80;
        std::array::from_fn(|i| rf(b, o + 4 * i as u32))
    }

    pub fn sonic(&self) -> Sonic {
        let b = self.mem.base;
        let (x, c) = (self.proxy, self.context);
        Sonic {
            position: [rf(b, x + 0x120), rf(b, x + 0x124), rf(b, x + 0x128)],
            proxy_velocity: [rf(b, x + 0x130), rf(b, x + 0x134), rf(b, x + 0x138)],
            velocity: [rf(b, c + 0x210), rf(b, c + 0x214), rf(b, c + 0x218)],
            rotation: [rf(b, c + 0x440), rf(b, c + 0x444), rf(b, c + 0x448), rf(b, c + 0x44C)],
            body_rotation: {
                let o = rd(b, c + 0x10);
                [rf(b, o + 0x60), rf(b, o + 0x64), rf(b, o + 0x68), rf(b, o + 0x6C)]
            },
            body_position: {
                let o = rd(b, c + 0x10);
                [rf(b, o + 0x70), rf(b, o + 0x74), rf(b, o + 0x78)]
            },
        }
    }
}
