param(
    [Parameter(Mandatory=$true)]
    [string]$Executable,

    [string]$GameRoot = 'D:\UnleashedRecomp-Windows',

    [string]$Output = '',

    [string]$ProxyGuest = '',

    [switch]$Discovery,

    # Directory for a guest snapshot: press F8 in a stage and the next per-frame update
    # (sub_82346F08) is written as pre.bin / post.bin (patches/guest_snapshot.h).
    [string]$Snapshot = '',

    [string[]]$Arguments = @()
)

$ErrorActionPreference = 'Stop'

if (-not (Test-Path $Executable)) {
    throw "Executable not found: $Executable"
}

if (-not (Test-Path (Join-Path $GameRoot 'game'))) {
    throw "Game root must contain game\: $GameRoot"
}

if (-not $Output) {
    $stamp = Get-Date -Format 'yyyyMMdd-HHmmss'
    $Output = Join-Path $PSScriptRoot "traces\reference-$stamp.jsonl"
}

$Output = [IO.Path]::GetFullPath($Output)
New-Item -ItemType Directory -Force -Path (Split-Path $Output -Parent) | Out-Null

if ($ProxyGuest) {
    if ($ProxyGuest -notmatch '^0[xX][0-9a-fA-F]{1,8}$') {
        throw 'ProxyGuest must be a guest address such as 0x8ABCDEF0.'
    }
    $env:UNLEASHED_TRACE_PROXY = $ProxyGuest
}
else {
    Remove-Item Env:UNLEASHED_TRACE_PROXY -ErrorAction SilentlyContinue
}

$env:UNLEASHED_TRACE_OUT = $Output
if ($Snapshot) {
    $Snapshot = [IO.Path]::GetFullPath($Snapshot)
    New-Item -ItemType Directory -Force -Path $Snapshot | Out-Null
    $env:UNLEASHED_SNAPSHOT_OUT = $Snapshot
    Write-Host "Snapshot:  $Snapshot (press F8 while playing a stage)"
}
$env:UNLEASHED_TRACE_DISCOVERY = if ($Discovery) { '1' } else { '0' }

Write-Host "Trace:     $Output"
Write-Host "Discovery: $($Discovery.IsPresent)"
Write-Host "Proxy:     $(if($ProxyGuest){$ProxyGuest}else{'unset'})"
Write-Host ''
Write-Host 'Known sample PCs:'
Write-Host '  0x8234EEE8  CPlayerSpeedPostureInput3DStandard'
Write-Host '  0x8234F194  CPlayerSpeedPostureInputOnPath / ForcePath / 2DStandard'
Write-Host '  0x8234F610  CPlayerSpeedPostureInputPathLocal'
Write-Host 'State probe: 0x8231F824'
Write-Host ''

Push-Location $GameRoot
try {
    & $Executable @Arguments
    $exitCode = $LASTEXITCODE
}
finally {
    Pop-Location
    Remove-Item Env:UNLEASHED_TRACE_OUT -ErrorAction SilentlyContinue
    Remove-Item Env:UNLEASHED_TRACE_DISCOVERY -ErrorAction SilentlyContinue
    Remove-Item Env:UNLEASHED_TRACE_PROXY -ErrorAction SilentlyContinue
    Remove-Item Env:UNLEASHED_SNAPSHOT_OUT -ErrorAction SilentlyContinue
}

if ($exitCode -ne 0) {
    throw "UnleashedRecomp exited with code $exitCode"
}

Write-Output $Output
