param(
    [switch]$Force,
    [int]$ProgressEvery = 100
)

$ErrorActionPreference = 'Stop'

$Workspace = (Resolve-Path (Join-Path $PSScriptRoot '..\..\..')).Path
$GameDb = Join-Path $Workspace 'tools\bin\gamedb.exe'
$PrimaryRoot = Join-Path $Workspace 'refs\UnleashedRecomp\UnleashedRecomp'
$LibRoot = Join-Path $Workspace 'refs\UnleashedRecomp\UnleashedRecompLib'

$PrimaryDb = Join-Path $PSScriptRoot 'unleashedrecomp.sqlite'
$LibDb = Join-Path $PSScriptRoot 'unleashedrecomplib.sqlite'

if (-not (Test-Path $GameDb)) {
    throw "gamedb.exe not found: $GameDb"
}

function Invoke-Index {
    param(
        [string]$Root,
        [string]$Database
    )

    $args = @(
        'index',
        '-r', $Root,
        '--db', $Database,
        '-v', $ProgressEvery
    )

    if ($Force) {
        $args += '--force'
    }

    & $GameDb @args
    if ($LASTEXITCODE -ne 0) {
        throw "gamedb index failed for $Root (exit $LASTEXITCODE)"
    }
}

Write-Host '== UnleashedRecomp =='
Invoke-Index -Root $PrimaryRoot -Database $PrimaryDb

Write-Host "`n== UnleashedRecompLib =="
Invoke-Index -Root $LibRoot -Database $LibDb

Write-Host "`n== Stats =="
& $GameDb stats -r $PrimaryRoot --db $PrimaryDb --json
& $GameDb stats -r $LibRoot --db $LibDb --json
