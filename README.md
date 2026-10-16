# Sonic Resort Island prototype

A work-in-progress Rust/Bevy runtime with a Sonic R Resort Island importer and
Sonic Unleashed guest-controller integration. The guest mode uses the same Sonic
controller, camera and animation path as this project's Minecraft world mode.

This is an experimental source release, not a standalone playable download.
Generated guest code, extracted game files, course meshes/textures, memory
captures and build output are deliberately excluded from the public export.

## What's here

- `crates/unleashed-bevy`: level import, rendering, lighting, input and camera integration.
- `crates/unleashed-guest-host` and `crates/xenon-rt`: the guest execution host and support code.
- `re/xenon`, `re/xenos` and `sheets`: conversion tools and configuration for locally generated code/shaders.
- `crates/unleashed-assets`: readers for the original game's resource formats.
- `crates/unleashed-minecraft` and `third_party`: the Minecraft world integration and its dependencies.
- `assets/resort-island/level.json`: the level's scale, lighting, camera and spawn configuration.
- `scripts/Launch-ResortIsland.ps1`: the Windows launcher, with gamepad input enabled by default.

## Required local inputs

The source export does not include these inputs:

1. Your own Sonic Unleashed game files, including `game/default.xex`, the matching
   update and shader/resource archives. The launcher expects a parent install
   directory containing `game/`.
2. A compatible guest-state capture, such as `82346F08_pre.bin`. Set
   `UNLEASHED_SNAPSHOT` to its local path. A snapshot contains game data and is
   not part of this source release.
3. The Resort Island course rip: `island.obj`, `island.mtl`, `ISLAND00.png` through
   `ISLAND06.png`, and `islandfloor.png`, supplied locally in
   `assets/resort-island/`. The course rip used during development is credited to
   InvisibleUp; its mesh and textures are not bundled here.
4. Generated guest Rust sources in `crates/unleashed-guest/src/`, generated from
   your local XenonRecomp output in `re/xenon/ppc/`. Character rendering also needs
   the generated shader outputs, which are excluded from this release.

The toolchain is Rust with edition 2024 support, the Windows MSVC C++ build tools,
and Python 3 for conversion scripts. The reference-capture build has additional
CMake/Clang requirements documented in `re/capture/README.md`.

This is a development snapshot: there is not yet a fully automated fresh-clone
setup, and some reverse-engineering helper scripts retain development-machine
paths. A fresh source export has not been validated as a complete clean build.

## Generate, build and run

After preparing your own recompiler inputs and the other local files above, the
guest generator entry point is:

```powershell
python .\sheets\sheetc.py lift
```

This command requires the reference inputs used by the sheet preflight; it does
not download or extract a game. See `re/capture/README.md`, `re/xenon/ppc2rust.py`
and the sheet configuration for the development pipeline.

Once the local generation/setup is complete:

```powershell
cargo build --release --config 'profile.release.package.unleashed-guest.opt-level=1' -p unleashed-bevy --bin windmill-isle
$env:UNLEASHED_SNAPSHOT = 'C:\path\to\82346F08_pre.bin'
.\scripts\Launch-ResortIsland.ps1
```

The scoped guest optimization setting avoids the excessive compile cost seen at
the workspace's normal release optimization level.

## Controls and current limits

PS5: left stick moves, right stick moves the camera, Cross jumps, Square boosts,
Circle slides/stomps, and Share respawns. Keyboard controls and level settings are
documented in `assets/resort-island/LEVEL-README.md`.

Bridge and solid-water collision are implemented. Motion blur is disabled for
Resort Island. Rendering interpolates the original fixed-step simulation for
high-refresh displays. First-use effect hitches and incomplete guest calls remain;
this is not a complete recreation of Sonic R's race rules or Sonic Unleashed.

## Credits and licensing

- [UnleashedRecomp](https://github.com/hedge-dev/UnleashedRecomp) and XenonRecomp:
  reference/capture and recompilation tooling. These projects are not included as
  complete checkouts in the export; their own terms apply when obtained separately.
- Minecraft world dependencies were adapted from
  [2010 Rust Rewrite Mashup](https://github.com/chasmlol/2010-rust-rewrite-mashup)
  at `f608f85`; its notice identifies the vendored MinecraftOSS code as its author's
  own in-tree project. The export includes the upstream Apache-2.0 license and
  notice under `third_party/`.
- The LZX reader retains its original MIT/Apache-2.0 license texts under
  `crates/unleashed-assets/src/xcompress_lzx/`.
- Sonic and the referenced games, characters and game assets belong to their
  respective owners. No project-wide license for this project's code has been
  selected; existing third-party license terms and notices are retained.

## Preparing an upload

`python scripts/Export-PublicSource.py --output <new-folder>` creates a separate
source-only draft, an archive and a file/hash manifest. It leaves the working
game, original Git history, snapshots and extracted assets in place. Review that
draft before publishing it; do not upload the full development/install folder.
