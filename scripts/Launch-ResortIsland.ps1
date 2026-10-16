[CmdletBinding()]
param([switch] $Build, [switch] $Gamepad, [switch] $KeyboardOnly)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'
. (Join-Path $PSScriptRoot 'Common.ps1')
$repoRoot = Get-UnleashedRepoRoot
$cargo = Resolve-CargoExe
$targetDir = Get-CargoTargetDirectory -Cargo $cargo -RepoRoot $repoRoot
$exe = Join-Path $targetDir 'release\windmill-isle.exe'
if ($Build -or -not (Test-Path -LiteralPath $exe)) {
    Push-Location $repoRoot
    try { Invoke-Cargo -Cargo $cargo -Arguments @('build', '--release', '--config', 'profile.release.package.unleashed-guest.opt-level=1', '-p', 'unleashed-bevy', '--bin', 'windmill-isle') }
    finally { Pop-Location }
}
$logDir = Join-Path $repoRoot ('artifacts\resort-island\play-' + (Get-Date -Format 'yyyyMMdd-HHmmss'))
New-Item -ItemType Directory -Path $logDir -Force | Out-Null
$previousGamepad = $env:UNLEASHED_NO_GAMEPAD
$previousLog = $env:RUST_LOG
try {
    if ($KeyboardOnly) { $env:UNLEASHED_NO_GAMEPAD = '1' }
    else { Remove-Item Env:\UNLEASHED_NO_GAMEPAD -ErrorAction SilentlyContinue }
    $env:RUST_LOG = 'warn,windmill_isle=info'
    $game = Start-Process -FilePath $exe -ArgumentList @('resort-island') -WorkingDirectory $repoRoot -WindowStyle Hidden -RedirectStandardOutput (Join-Path $logDir 'stdout.log') -RedirectStandardError (Join-Path $logDir 'stderr.log') -PassThru
    Write-Output "Resort Island PID: $($game.Id)"
    Write-Output "Logs: $logDir"
}
finally {
    $env:UNLEASHED_NO_GAMEPAD = $previousGamepad
    $env:RUST_LOG = $previousLog
}
