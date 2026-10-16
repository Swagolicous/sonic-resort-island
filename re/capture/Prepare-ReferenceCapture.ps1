param(
    [string]$Workspace = 'D:\UnleashedRecomp-Windows',
    [string]$Destination = '',
    [ValidateSet('x64-Clang-Debug','x64-Clang-RelWithDebInfo','x64-Clang-Release')]
    [string]$Preset = 'x64-Clang-RelWithDebInfo',
    [switch]$SkipBuild
)

$ErrorActionPreference = 'Stop'

$CaptureRoot = $PSScriptRoot
$Reference = Join-Path $Workspace 'refs\UnleashedRecomp'
$Patch = Join-Path $CaptureRoot 'unleashed-reference-trace.patch'
$ExpectedCommit = 'cf829a9eca8fb680fba4b0409ddeb6ca92f22e3c'

if (-not $Destination) {
    $Destination = Join-Path $CaptureRoot '_work\UnleashedRecomp-trace'
}

if (-not (Test-Path $Reference)) {
    throw "Reference clone not found: $Reference"
}

if (-not (Test-Path $Patch)) {
    throw "Capture patch not found: $Patch"
}

$actualCommit = (& git -C $Reference rev-parse HEAD).Trim()
if ($actualCommit -ne $ExpectedCommit) {
    throw "Reference commit mismatch. Expected $ExpectedCommit, got $actualCommit"
}

$dirty = & git -C $Reference status --porcelain=v1
if ($dirty) {
    throw "Reference clone is dirty; refusing to stage from it.`n$dirty"
}

& git -C $Reference apply --check $Patch
if ($LASTEXITCODE -ne 0) {
    throw 'Capture patch does not apply cleanly to the reference clone.'
}

if (Test-Path $Destination) {
    Remove-Item -Recurse -Force $Destination
}
New-Item -ItemType Directory -Force -Path (Split-Path $Destination -Parent) | Out-Null

# Copy the verified-clean working tree (submodules already checked out) instead of
# cloning: a recursive clone would refetch every submodule from GitHub.
Write-Host "Copying clean reference tree to $Destination"
& robocopy $Reference $Destination /E /XD .git out /NFL /NDL /NJH /NJS /NP | Out-Null
if ($LASTEXITCODE -ge 8) {
    throw "robocopy failed ($LASTEXITCODE)"
}

# The copied submodule .git files point at ../.git/modules/... relative to the copy,
# which has no .git. Point them (and the root) at the reference repository instead:
# vcpkg reads its baseline through git, and nothing here writes to it.
Get-ChildItem $Destination -Recurse -Force -File -Filter .git | ForEach-Object {
    $rel = $_.Directory.FullName.Substring($Destination.Length).TrimStart('\')
    $gitdir = ((Get-Content $_.FullName -Raw) -replace '^gitdir:\s*', '').Trim()
    $abs = [IO.Path]::GetFullPath((Join-Path (Join-Path $Reference $rel) $gitdir))
    Set-Content -Path $_.FullName -Value "gitdir: $($abs -replace '\\', '/')" -Encoding ascii -NoNewline
}
Set-Content -Path (Join-Path $Destination '.git') -Value "gitdir: $((Join-Path $Reference '.git') -replace '\\', '/')" -Encoding ascii -NoNewline

Push-Location $Destination
& git apply $Patch
$applied = $LASTEXITCODE
Pop-Location
if ($applied -ne 0) {
    throw 'git apply failed in staged clone'
}

$private = Join-Path $Destination 'UnleashedRecompLib\private'
New-Item -ItemType Directory -Force -Path $private | Out-Null

$required = @(
    @{ Source = Join-Path $Workspace 'game\default.xex'; Name = 'default.xex' },
    @{ Source = Join-Path $Workspace 'update\default.xexp'; Name = 'default.xexp' },
    @{ Source = Join-Path $Workspace 'game\shader.ar'; Name = 'shader.ar' }
)

foreach ($item in $required) {
    if (-not (Test-Path $item.Source)) {
        throw "Required build input missing: $($item.Source)"
    }
    Copy-Item -Force $item.Source (Join-Path $private $item.Name)
}

if ($SkipBuild) {
    Write-Host 'Patched staging clone prepared; build skipped.'
    Write-Output $Destination
    exit 0
}

Push-Location $Destination
try {
    $ninja = (Get-Command ninja.exe -ErrorAction Stop).Source
    & cmake . --preset $Preset "-DCMAKE_MAKE_PROGRAM=$ninja"
    if ($LASTEXITCODE -ne 0) {
        throw 'CMake configure failed'
    }

    $buildDir = Join-Path $Destination "out\build\$Preset"
    & cmake --build $buildDir --target UnleashedRecomp
    if ($LASTEXITCODE -ne 0) {
        throw 'UnleashedRecomp build failed'
    }
}
finally {
    Pop-Location
}

$exe = Get-ChildItem (Join-Path $Destination "out\build\$Preset") -Recurse -File -Filter 'UnleashedRecomp.exe' |
    Select-Object -First 1 -ExpandProperty FullName

if (-not $exe) {
    throw "Build succeeded but UnleashedRecomp.exe was not found under out\build\$Preset"
}

Write-Host "Capture build: $exe"
Write-Output $exe
