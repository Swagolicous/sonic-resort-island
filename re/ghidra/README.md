# Ghidra seed + JSONL export

This directory contains the first automation layer for joining Ghidra analysis of
Sonic Unleashed's Xenon executable with the existing XenonRecomp/gamedb reverse-
engineering corpus.

It deliberately does **not** modify XenonRecomp, `gamedb`, or the local Ghidra CLI.

## Inputs and provenance

The current local XenonRecomp build emits its complete function address/name table
to:

```text
D:\decomp\UnleashedRecomp\UnleashedRecompLib\ppc\ppc_func_mapping.cpp
```

The current table contains 66,833 entries. Most names are synthetic
`sub_<address>` identifiers, while imports, runtime save/restore helpers, and
`_xstart` carry stronger semantic names.

Explicit boundary overrides live in:

```text
D:\decomp\UnleashedRecomp\UnleashedRecompLib\config\SWA.toml
```

At the time this automation was introduced, that config contains 42 explicit
function boundaries. Those are marked separately from ordinary XenonRecomp
analysis in the seed; they are not silently treated as equivalent evidence.

The seed builder writes the SHA-256 of the mapping, config, and optional binary
into every seed record. The Ghidra exporter writes the current program's
executable SHA-256, exporter name, executable format, and Ghidra version into
every JSONL record.

## 1. Build the XenonRecomp seed

From `D:\UnleashedRecomp-Windows`:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File `
  .\unleashed-rs\re\ghidra\BuildXenonSeed.ps1 `
  -MappingPath D:\decomp\UnleashedRecomp\UnleashedRecompLib\ppc\ppc_func_mapping.cpp `
  -ConfigPath D:\decomp\UnleashedRecomp\UnleashedRecompLib\config\SWA.toml `
  -BinaryPath .\patched\default.xex `
  -OutputPath .\unleashed-rs\re\ghidra\out\xenon_seed.jsonl
```

Each `unleashed_re.xenon_seed.v1` record contains:

- `address_hex` and `address_u64`
- XenonRecomp `name`
- `name_kind`
- optional explicit `size`
- independent address/name/boundary confidence
- mapping/config/binary paths and SHA-256 provenance

The builder fails on duplicate addresses or an explicit TOML boundary that is
missing from the emitted XenonRecomp mapping.

## 2. Import/analyze the XEX in Ghidra

The local wrapper is `tools\bin\ghidra.exe` (`ghidra-cli 0.2.2`) and can run
arbitrary checked-in Java scripts by absolute or relative path.

Keep Ghidra project/database artifacts outside the repository. For example:

```powershell
$GhidraProjects = Join-Path $env:TEMP 'unleashed-ghidra-projects'
New-Item -ItemType Directory -Force $GhidraProjects | Out-Null

.\tools\bin\ghidra.exe import .\patched\default.xex `
  --projects-dir $GhidraProjects `
  --project unleashed-xenon `
  --program default.xex
```

If the program was imported with `--no-analyze`, run:

```powershell
.\tools\bin\ghidra.exe analyze `
  --projects-dir $GhidraProjects `
  --project unleashed-xenon `
  --program default.xex
```

The exact loader/language selected by Ghidra is part of the project state. Verify
the program summary before applying seeds:

```powershell
.\tools\bin\ghidra.exe summary `
  --projects-dir $GhidraProjects `
  --project unleashed-xenon `
  --program default.xex
```

## 3. Apply XenonRecomp function seeds

```powershell
.\tools\bin\ghidra.exe script run `
  .\unleashed-rs\re\ghidra\ApplyXenonSeed.java `
  --projects-dir $GhidraProjects `
  --project unleashed-xenon `
  --program default.xex `
  -- `
  .\unleashed-rs\re\ghidra\out\xenon_seed.jsonl
```

Behavior is intentionally conservative:

- only seed addresses that exist in the current Ghidra memory map;
- create a missing function entry using Ghidra's standard `CreateFunctionCmd`;
- do **not** force the XenonRecomp size into the Ghidra body;
- do **not** turn synthetic `sub_<address>` names into imported analyst names;
- import semantic XenonRecomp names only when the existing name is a Ghidra
  default;
- preserve existing analyst/non-default names.

Use `--overwrite-names` after the seed path only when replacing existing
non-default names is intentional.

Use `--no-create` to apply semantic names only to functions already known by
Ghidra.

## 4. Export joinable JSONL

```powershell
New-Item -ItemType Directory -Force `
  .\unleashed-rs\re\ghidra\out\export | Out-Null

.\tools\bin\ghidra.exe script run `
  .\unleashed-rs\re\ghidra\ExportJoinJsonl.java `
  --projects-dir $GhidraProjects `
  --project unleashed-xenon `
  --program default.xex `
  --expect .\unleashed-rs\re\ghidra\out\export\functions.jsonl:1 `
  --expect .\unleashed-rs\re\ghidra\out\export\addresses.jsonl:1 `
  --expect .\unleashed-rs\re\ghidra\out\export\names.jsonl:1 `
  --allow-empty `
  -- `
  .\unleashed-rs\re\ghidra\out\export
```

The wrapper's artifact contract reports output path, byte size, JSONL row count,
SHA-256, Ghidra program, binary SHA-256, executable format, and Ghidra version.

The exporter writes five streaming relations:

| File | Natural key / purpose |
| --- | --- |
| `addresses.jsonl` | `(binary_sha256,address_u64,role)`; common address spine |
| `names.jsonl` | `(binary_sha256,address_u64,name)`; all non-external Ghidra symbols |
| `functions.jsonl` | `(binary_sha256,address_u64)`; Ghidra function/body/signature metadata |
| `strings.jsonl` | `(binary_sha256,address_u64)`; defined string data |
| `call_edges.jsonl` | `(binary_sha256,caller_address_u64,callsite_address_u64,target_address_u64)` |

`address_hex` is included for humans. `address_u64` is the stable numeric join
column.

Call edges are explicitly marked `direct_calls_only`. Ghidra reference-manager
calls do not represent every virtual call, function pointer, computed branch, or
other indirect transfer.

## Joining with gamedb

The current `tools\bin\gamedb.exe` indexes decompiled source into SQLite; it does
not natively ingest these JSONL files. Treat the JSONL as an external relation
set and join through the common Xenon address encoded in generated source names
such as `sub_822C0000`, or export/query gamedb's SQLite data with `gamedb sql`.

Typical source-side address extraction rule:

```text
sub_822C0000 -> 0x822C0000 -> 2183921664
```

Prefer this join order:

1. `binary_sha256`
2. numeric Xenon address (`address_u64`)
3. source/generated function name only as a validation column

Do not join solely on `sub_<address>` text when a numeric address is available.

## Schemas

Current record schema IDs:

```text
unleashed_re.xenon_seed.v1
unleashed_re.ghidra.address.v1
unleashed_re.ghidra.name.v1
unleashed_re.ghidra.function.v1
unleashed_re.ghidra.string.v1
unleashed_re.ghidra.call_edge.v1
```

Schema IDs are versioned so later decompiler/p-code/basic-block exports can be
added without silently changing these relations.
