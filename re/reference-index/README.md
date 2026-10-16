# UnleashedRecomp reference index

This directory is a local, rerunnable `gamedb` index of the upstream reference implementation. It does not modify either reference repository.

## Inputs

- `refs\UnleashedRecomp\UnleashedRecomp`
- `refs\UnleashedRecomp\UnleashedRecompLib`
- indexer: `tools\bin\gamedb.exe` 1.0.0

Databases are deliberately stored here:

- `unleashedrecomp.sqlite`
- `unleashedrecomplib.sqlite`

## Refresh incrementally

From the workspace root:

```powershell
.\unleashed-rs\re\reference-index\index.ps1
```

`gamedb index` is incremental by default. Force a complete re-index only when needed:

```powershell
.\unleashed-rs\re\reference-index\index.ps1 -Force
```

Query one subsystem:

```powershell
.\unleashed-rs\re\reference-index\query.ps1 boot
.\unleashed-rs\re\reference-index\query.ps1 video
.\unleashed-rs\re\reference-index\query.ps1 assets
.\unleashed-rs\re\reference-index\query.ps1 filesystem
.\unleashed-rs\re\reference-index\query.ps1 input
.\unleashed-rs\re\reference-index\query.ps1 app-update
```

Or dump all current summaries:

```powershell
.\unleashed-rs\re\reference-index\query.ps1 all
```

## Current index snapshot

At creation time:

| corpus | files | functions | strings | symbols | call edges |
|---|---:|---:|---:|---:|---:|
| UnleashedRecomp | 298 | 945 | 6,757 | 1,982 | 12,237 |
| UnleashedRecompLib | 2 | 0 | 17 | 13 | 0 |

The main corpus derived 14 path modules. Largest indexed function buckets were:

| derived module | functions |
|---|---:|
| `unmapped` | 378 |
| `ns.kernel` | 259 |
| `ns.ui` | 127 |
| `ns.patches` | 125 |
| `ns.user` | 28 |
| `ns.install` | 14 |
| `ns.gpu.imgui` | 11 |

A task-oriented SQL bucketing of the same index found:

| area | files | functions |
|---|---:|---:|
| video | 11 | 203 |
| input | 4 | 34 |
| filesystem | 8 | 18 |
| assets/install | 25 | 12 |
| boot/app | 2 | 7 |

The task buckets overlap conceptual responsibilities and are not intended to replace `gamedb`'s path-derived modules.

## Key mapping

### Boot

Primary control path is `main.cpp:195-367`.

Important stages:

1. `PreloadContext::PreloadExecutable` — `preload_executable.cpp:19-104`
   - Windows-only host executable prefetch/lock.
2. `Config::Load`
3. optional install integrity check
4. periodic update-check launch
5. `HostStartup` — `main.cpp:52-59`
6. `Installer::checkGameInstall`
7. optional `Video::CreateHostDevice` for installer UI
8. `ModLoader::Init`
9. persistent-storage load
10. `KiSystemStartup` — `main.cpp:62-120`
11. `LdrLoadModule` — `main.cpp:122-170`
12. `Video::CreateHostDevice` for normal game path
13. `Video::StartPipelinePrecompilation`
14. `GuestThread::Start({ entry, 0, 0 })`

The indexed call graph for `main` resolves direct calls to `HostStartup`, `KiSystemStartup`, and `LdrLoadModule`.

### App update

`app.cpp` wraps original guest application functions with two important hooks.

- `SWA::CApplication::CApplication` wrapper — `app.cpp:31-43`
  - marks app initialized
  - probes DLC
  - captures configured language
  - initializes `SWA::SGlobals`
  - saves registry state
  - delegates to original guest function
- `SWA::CApplication::Update` wrapper — `app.cpp:47-96`
  - `Video::WaitOnSwapChain`
  - normalizes small frame-delta errors
  - updates `App::s_deltaTime` / `App::s_time`
  - pumps SDL events only on the main thread
  - updates the game window
  - runs audio/inspire patch updates
  - applies subtitle/debug settings
  - delegates to original guest update

Indexer caveat: macro-wrapped guest functions are recorded under generic `PPC_FUNC`, so source comments/line ranges are more reliable identifiers than name search for these two hooks.

### Video

Dominant implementation: `gpu/video.cpp` with 200+ indexed functions.

High-value entries:

- `CreateDevice` — `gpu/video.cpp:2095-2147`
- `CheckSwapChain` — `1530-1555`
- `SetRenderState` / `ProcSetRenderState` — `1009-1016`, `1093-1248`
- `EnqueuePipelineTask` — `423-437`
- `CreateGraphicsPipelineInRenderThread` — `4135-4229`
- `CompilePipeline` — `6247-6264`
- `PipelineCompilerThread` — `6266-6308`
- `EnqueueGraphicsPipelineCompilation` — `6337-6389`
- `CompileMeshPipelines` — `6757-6810`
- `GetDatabaseDataMidAsmHook` — `7006-7034`
- `PipelineTaskConsumerThread` — `7119-7375`
- `LoadTexture` implementation — `5624-5813`

`GetDatabaseDataMidAsmHook` directly queues pipeline work through `EnqueuePipelineTask`, making it a useful bridge between guest database/model discovery and host-side pipeline compilation.

Guest rendering enums and public texture/config declarations are in `gpu/video.h`; notable public declaration is `LoadTexture` at line 411.

### Assets / install

Useful pivots:

- `Installer::checkGameInstall` is invoked directly by boot code.
- install integrity work is rooted in `install/installer.cpp`.
- known install payload strings include `default.xex`, `default.xexp`, `shader.ar`, and `shader.arl`.
- `LdrLoadModule` — `main.cpp:122-170` loads the selected game module before guest-thread startup.
- `LoadTexture` — `gpu/video.cpp:5624-5813` is the concrete host texture ingestion path.
- `GetDatabaseDataMidAsmHook` — `gpu/video.cpp:7006-7034` is a useful runtime database-data interception point for rendering/pipeline work.

`UnleashedRecompLib` currently contributes configuration/shader-cache metadata but no indexed function bodies.

### Filesystem

Primary guest filesystem implementation is `kernel/io/file_system.cpp`.

Important functions include:

- `XCreateFileA` — `82-122`
- `XGetFileSizeA` — `124-139`
- `XGetFileSizeExA` — `141-156`
- `XSetFilePointer` / `XSetFilePointerEx`
- `XFindFirstFileA`
- `XFindNextFileA` — `298-311`
- `XReadFileEx` — `313-337`
- `XGetFileAttributesA` — `339-348`
- `XWriteFile` — `350-362`
- `FileSystem::ResolvePath` — starts at `364`

`FileSystem::ResolvePath` first consults the mod loader when requested, so guest file access and mod overlay resolution converge here.

Install-side host filesystem adapters live under:

- `install/directory_file_system.h`
- `install/iso_file_system.cpp`
- `install/xcontent_file_system.cpp`

### Input

Main host/guest bridge:

- `kernel/xam.cpp`
  - `XamInputGetCapabilities` — `385-402`
  - `XamInputGetState` — `404-488`
  - `XamInputSetState` — `490-499`
- `hid/driver/sdl_hid.cpp`
  - SDL controller state translation and vibration
  - `HID_OnSDLEvent` — starts at line 221
  - controller activity switches the active input device
  - keyboard/mouse events also update active input-device state
- `patches/input_patches.cpp`
  - gameplay/world-map input compatibility and deadzone/D-pad hooks
  - `Update` — `82-122`

`HID_OnSDLEvent` calls `SetControllerInputDevice` on controller activity; this relationship is present in the indexed call graph.

## Indexer limitations relevant to reverse engineering

- Functions generated through macros may index under generic names such as `PPC_FUNC`.
- Inline/header-only methods can appear inconsistently depending on syntax shape.
- Name-only search is substring based; use SQL with `files.path`, line ranges, strings, and graph queries when names are ambiguous.
- A missing graph edge does not prove no runtime relationship exists; macro expansion, indirect calls, function pointers, or parser limits can hide edges.
- `UnleashedRecompLib` currently contains almost no parseable function bodies, so its database is mainly a change detector for future reference growth.

## Useful direct commands

```powershell
# Stats
tools\bin\gamedb.exe stats -r refs\UnleashedRecomp\UnleashedRecomp --db unleashed-rs\re\reference-index\unleashedrecomp.sqlite

# Function-name search
tools\bin\gamedb.exe search -r refs\UnleashedRecomp\UnleashedRecomp --db unleashed-rs\re\reference-index\unleashedrecomp.sqlite CreateDevice

# Literal search
tools\bin\gamedb.exe strings -r refs\UnleashedRecomp\UnleashedRecomp --db unleashed-rs\re\reference-index\unleashedrecomp.sqlite shader.ar

# Call graph
tools\bin\gamedb.exe graph -r refs\UnleashedRecomp\UnleashedRecomp --db unleashed-rs\re\reference-index\unleashedrecomp.sqlite main --direction both

# Derived modules
tools\bin\gamedb.exe modules -r refs\UnleashedRecomp\UnleashedRecomp --db unleashed-rs\re\reference-index\unleashedrecomp.sqlite --rules derive
```
