# UnleashedRecomp reference movement capture

This directory is a repeatable, read-only telemetry plan for producing a reference movement trace from the cloned UnleashedRecomp source. Nothing here modifies `refs\UnleashedRecomp` in place.

Target reference commit:

`cf829a9eca8fb680fba4b0409ddeb6ca92f22e3c`

The source facts come from `..\telemetry.json`, `UnleashedRecompLib\config\SWA.toml`, and the reference API layouts.

## What is captured

The primary JSONL sample is emitted at Sonic posture input sites:

| guest PC | symbol / region | registers already exposed by SWA.toml |
|---|---|---|
| `0x8234EEE8` | `SWA::Player::CPlayerSpeedPostureInput3DStandard` | `r31`, `f12`, `f13` |
| `0x8234F194` | `CPlayerSpeedPostureInputOnPath` / `CPlayerSpeedPostureInputForcePath` / `CPlayerSpeedPostureInput2DStandard` | `r31`, `f13`, `f0` |
| `0x8234F610` | `SWA::Player::CPlayerSpeedPostureInputPathLocal` | `r30`, `f0`, `f13` |

A state-transition correlation probe is captured at:

- `0x8231F824` — shared region for `CPlayerSpeedStateNormalDamageStandUp`, `CSonicStateStartEvent`, and `CSonicStateStompingLand`; registers `r29`, `f12`, `f13`.

Guest dt is observed before the existing HFR clamp at the eight 2D spline sites:

- `0x82345468`
- `0x8234547C`
- `0x823454B0`
- `0x82345534`
- `0x823455EC`
- `0x8234564C`
- `0x823457A8`
- `0x823458A0`

Independent camera correlation probes are preserved at:

- `0x824761AC` — player velocity, `f0` / `f28`
- `0x824762AC` — player position, `f0` / `f28`

Position and velocity use the confirmed `SWA::CCharacterProxy` layout:

- position: guest proxy `+0x120` (`X/Y/Z`, big-endian float32)
- velocity: guest proxy `+0x130` (`X/Y/Z`, big-endian float32)

Every posture sample contains:

- monotonically increasing capture `frame_index`
- exact `guest_pc` and mode (`3d`, `2d`, or `path_local`)
- host application `dt_seconds`
- most recently observed guest `f1` dt and its exact probe PC
- proxy position XYZ
- proxy velocity XYZ plus magnitude
- state-probe identity (`r29`) and a change epoch
- raw registers attached to that posture site

The state field is intentionally tagged `probe_identity_not_decoded`. `0x8231F824` is a confirmed state-related probe, but `r29` has not yet been proven to be the game's canonical state enum/hash. Do not relabel it as a decoded state until write-site/dispatch correlation establishes that mapping.

## Why the proxy address is an explicit input

The reference headers confirm the `CCharacterProxy` layout, but they do not expose a canonical global/accessor for Sonic's current proxy. The registers at the posture hooks are known pad/input registers and must not be guessed to be the proxy.

Therefore the trace patch reads position/velocity only from a guest address explicitly supplied through:

`UNLEASHED_TRACE_PROXY=0x........`

This is safer than speculative pointer chasing and keeps capture read-only.

## 1. Validate and stage a patched build

The patch is **not applied to the reference checkout**. `Prepare-ReferenceCapture.ps1` first runs `git apply --check`, clones the reference into `re\capture\_work`, applies the patch only there, and copies the three private build inputs from this workspace.

From `D:\UnleashedRecomp-Windows`:

```powershell
.\unleashed-rs\re\capture\Prepare-ReferenceCapture.ps1
```

Default preset:

`x64-Clang-RelWithDebInfo`

To stage without compiling:

```powershell
.\unleashed-rs\re\capture\Prepare-ReferenceCapture.ps1 -SkipBuild
```

Required inputs, already present in this workspace:

- `game\default.xex`
- `update\default.xexp`
- `game\shader.ar`

Windows build requirements are the same as upstream: Visual Studio 2022 C++ tooling, Clang/clang-cl, CMake/Ninja, and the repository vcpkg tree.

## 2. Discover and validate the active proxy guest address

Start with discovery enabled and no proxy:

```powershell
.\unleashed-rs\re\capture\Start-ReferenceCapture.ps1 `
  -Executable '<capture-build>\UnleashedRecomp.exe' `
  -Discovery
```

Run controlled scenarios:

1. stand idle for several seconds;
2. run a straight 3D segment;
3. jump and fall;
4. enter a known 2D/on-path segment;
5. return to 3D.

The discovery JSONL records the exact posture/state registers plus dt/camera probes. Use those timestamps/PCs in Ghidra or a host debugger to inspect callers and locate the object that owns/references a `CCharacterProxy`. Validate a candidate proxy by confirming:

- `candidate + 0x120` tracks Sonic world position;
- `candidate + 0x130` tracks Sonic velocity;
- all six XYZ values stay finite;
- position changes continuously with motion;
- velocity approaches zero at rest;
- the same guest pointer remains coherent across 3D and 2D transitions.

A useful source-backed pointer pattern to look for is the existing object code in `patches\object_patches.cpp`, where a character proxy is obtained through a guest pointer at object `+0x100` before reading `m_Velocity`. That is evidence for the proxy layout/access style, not proof that a posture register has the same owner layout.

### Ghidra/runtime inspection procedure

Use the seeded Xenon Ghidra project under `..\ghidra` or any debugger attached to the staged host build.

At each posture PC, inspect the surrounding guest PPC and identify the owning context/object register before the pad-state load:

- `0x8234EEE8`
- `0x8234F194`
- `0x8234F610`

At the state probe, inspect callers/dispatch around:

- `0x8231F824`

For a candidate owning object, follow guest pointers and look for a member referencing a block whose `+0x120/+0x130` values correlate with Sonic position/velocity. Confirm the pointer through at least idle, run, airborne, and 2D transitions before using it for parity captures.

## 3. Capture the coherent reference trace

Once the proxy guest address is validated:

```powershell
.\unleashed-rs\re\capture\Start-ReferenceCapture.ps1 `
  -Executable '<capture-build>\UnleashedRecomp.exe' `
  -ProxyGuest 0x89ABCDEF `
  -Output .\unleashed-rs\re\capture\traces\mykonos-reference.jsonl
```

Run the same deterministic gameplay route intended for the Rust implementation. Exit normally after the desired segment.

The patch is observational only: it snapshots registers/memory and then calls the original HFR/D-pad hook behavior.

## 4. Validate JSONL and produce CSV

```powershell
.\unleashed-rs\re\capture\Convert-Trace.ps1 `
  -InputPath .\unleashed-rs\re\capture\traces\mykonos-reference.jsonl
```

This produces a CSV beside the JSONL with one row per posture sample:

`frame_index,guest_pc,mode,dt_seconds,guest_dt_seconds,guest_dt_pc,proxy_guest,proxy_valid,pos_x,pos_y,pos_z,vel_x,vel_y,vel_z,speed,state_status,state_seen,state_epoch,state_r29`

Keep the JSONL as the lossless reference because it also contains raw probe records.

## Patch contents

`unleashed-reference-trace.patch` changes only a disposable reference build when explicitly applied. It:

- adds `UnleashedRecomp\patches\telemetry_capture.h`;
- wraps the eight existing dt hook sites with unique telemetry names;
- wraps the two camera correlation hooks with unique telemetry names;
- wraps the three Sonic posture hooks with unique telemetry names;
- wraps `0x8231F824` with a state correlation hook;
- preserves the original underlying HFR and D-pad hook calls.

No patch is applied to `refs\UnleashedRecomp` by this capture kit.

## Open discovery item

The canonical Sonic state ID/hash and active `CCharacterProxy*` are not exposed by the current reference API. The state PC and proxy layout are confirmed, but those two identities still require runtime/write-site correlation. This kit keeps that uncertainty explicit rather than baking guessed offsets into the parity trace.
