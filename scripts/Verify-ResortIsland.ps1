[CmdletBinding()]
param(
    [string] $Name = ('check-' + (Get-Date -Format 'yyyyMMdd-HHmmss')),
    [string] $Autopilot = '0:',
    [double] $Seconds = 9
)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'
. (Join-Path $PSScriptRoot 'Common.ps1')
$repoRoot = Get-UnleashedRepoRoot
$cargo = Resolve-CargoExe
$targetDir = Get-CargoTargetDirectory -Cargo $cargo -RepoRoot $repoRoot
$exe = Join-Path $targetDir 'release\windmill-isle.exe'
if ($Name -notmatch '^[a-zA-Z0-9_-]+$') { throw 'Name must contain letters, digits, hyphens or underscores.' }
$logDir = Join-Path $repoRoot ('artifacts\resort-island\' + $Name)
New-Item -ItemType Directory -Path $logDir -Force | Out-Null
$taskEnvKeys = @('UNLEASHED_NO_GAMEPAD', 'UNLEASHED_SHOTS', 'UNLEASHED_SHOT_EVERY', 'UNLEASHED_EXIT_AFTER', 'UNLEASHED_TELEMETRY', 'UNLEASHED_AUTOPILOT', 'RUST_LOG')
$taskOldEnv = @{}
foreach ($key in $taskEnvKeys) { $taskOldEnv[$key] = [Environment]::GetEnvironmentVariable($key, 'Process') }
try {
    $env:UNLEASHED_NO_GAMEPAD = '1'
    $env:UNLEASHED_SHOTS = $logDir
    $env:UNLEASHED_SHOT_EVERY = '2'
    $env:UNLEASHED_EXIT_AFTER = $Seconds.ToString([Globalization.CultureInfo]::InvariantCulture)
    $env:UNLEASHED_TELEMETRY = '1'
    $env:UNLEASHED_AUTOPILOT = $Autopilot
    $env:RUST_LOG = 'warn,windmill_isle=info'
    Copy-Item -LiteralPath (Join-Path $repoRoot 'assets\resort-island\level.json') -Destination (Join-Path $logDir 'level.json')
    $game = Start-Process -FilePath $exe -ArgumentList @('resort-island') -WorkingDirectory $repoRoot -WindowStyle Hidden -RedirectStandardOutput (Join-Path $logDir 'stdout.log') -RedirectStandardError (Join-Path $logDir 'stderr.log') -PassThru
    [pscustomobject]@{ ProcessId = $game.Id; Artifacts = $logDir } | ConvertTo-Json
}
finally {
    foreach ($key in $taskEnvKeys) { [Environment]::SetEnvironmentVariable($key, $taskOldEnv[$key], 'Process') }
}
