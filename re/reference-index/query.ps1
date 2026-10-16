param(
    [ValidateSet('stats','modules','boot','video','assets','filesystem','input','app-update','all')]
    [string]$Area = 'all'
)

$ErrorActionPreference = 'Stop'

$Workspace = (Resolve-Path (Join-Path $PSScriptRoot '..\..\..')).Path
$GameDb = Join-Path $Workspace 'tools\bin\gamedb.exe'
$Root = Join-Path $Workspace 'refs\UnleashedRecomp\UnleashedRecomp'
$Db = Join-Path $PSScriptRoot 'unleashedrecomp.sqlite'
$LibRoot = Join-Path $Workspace 'refs\UnleashedRecomp\UnleashedRecompLib'
$LibDb = Join-Path $PSScriptRoot 'unleashedrecomplib.sqlite'

if (-not (Test-Path $Db)) {
    throw "Primary index not found. Run .\index.ps1 first."
}

function Run-Sql([string]$Sql) {
    & $GameDb sql -r $Root --db $Db --sql $Sql --json
}

function Search([string]$Name, [int]$Limit = 50) {
    & $GameDb search -r $Root --db $Db $Name --limit $Limit --json
}

function Show-Stats {
    Write-Host '== Primary stats =='
    & $GameDb stats -r $Root --db $Db --json
    Write-Host "`n== Lib stats =="
    & $GameDb stats -r $LibRoot --db $LibDb --json
}

function Show-Modules {
    & $GameDb modules -r $Root --db $Db --rules derive --json
}

function Show-Boot {
    Run-Sql @"
SELECT f.path, fn.name, fn.start_line, fn.end_line
FROM functions fn
JOIN files f ON f.id=fn.file_id
WHERE f.path IN ('main.cpp','app.cpp','preload_executable.cpp')
ORDER BY f.path, fn.start_line;
"@
    Write-Host "`n== main graph =="
    & $GameDb graph -r $Root --db $Db main --direction both --json
}

function Show-Video {
    Run-Sql @"
SELECT f.path, fn.name, fn.start_line, fn.end_line
FROM functions fn
JOIN files f ON f.id=fn.file_id
WHERE f.path='gpu/video.cpp'
  AND (
    fn.name LIKE '%Device%' OR
    fn.name LIKE '%Swap%' OR
    fn.name LIKE '%Render%' OR
    fn.name LIKE '%Pipeline%' OR
    fn.name LIKE '%Texture%' OR
    fn.name LIKE '%Database%' OR
    fn.name LIKE '%Present%'
  )
ORDER BY fn.start_line;
"@
}

function Show-Assets {
    Write-Host '== installer / module loading =='
    foreach ($q in @('LdrLoadModule','LoadTexture','GetDatabaseDataMidAsmHook')) {
        Write-Host "`n-- $q --"
        Search $q
    }

    Write-Host "`n== asset-related strings =="
    foreach ($q in @('default.xex','shader.ar','shader.arl')) {
        Write-Host "`n-- $q --"
        & $GameDb strings -r $Root --db $Db $q --limit 50 --json
    }
}

function Show-Filesystem {
    Run-Sql @"
SELECT f.path, fn.name, fn.start_line, fn.end_line
FROM functions fn
JOIN files f ON f.id=fn.file_id
WHERE
  f.path='kernel/io/file_system.cpp' OR
  f.path LIKE 'install/%file_system%'
ORDER BY f.path, fn.start_line;
"@
}

function Show-Input {
    Run-Sql @"
SELECT f.path, fn.name, fn.start_line, fn.end_line
FROM functions fn
JOIN files f ON f.id=fn.file_id
WHERE
  f.path IN ('kernel/xam.cpp','hid/hid.cpp','hid/driver/sdl_hid.cpp','patches/input_patches.cpp')
ORDER BY f.path, fn.start_line;
"@
}

function Show-AppUpdate {
    Run-Sql @"
SELECT f.path, fn.name, fn.start_line, fn.end_line
FROM functions fn
JOIN files f ON f.id=fn.file_id
WHERE
  f.path IN ('app.cpp','install/update_checker.cpp','patches/CTitleStateIntro_patches.cpp')
  OR fn.name LIKE '%Update%'
ORDER BY f.path, fn.start_line;
"@
}

switch ($Area) {
    'stats'       { Show-Stats }
    'modules'     { Show-Modules }
    'boot'        { Show-Boot }
    'video'       { Show-Video }
    'assets'      { Show-Assets }
    'filesystem'  { Show-Filesystem }
    'input'       { Show-Input }
    'app-update'  { Show-AppUpdate }
    'all' {
        Show-Stats
        Write-Host "`n== Derived modules =="
        Show-Modules
        Write-Host "`n== Boot =="
        Show-Boot
        Write-Host "`n== Video =="
        Show-Video
        Write-Host "`n== Assets =="
        Show-Assets
        Write-Host "`n== Filesystem =="
        Show-Filesystem
        Write-Host "`n== Input =="
        Show-Input
        Write-Host "`n== App update =="
        Show-AppUpdate
    }
}
