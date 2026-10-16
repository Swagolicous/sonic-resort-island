[CmdletBinding()]
param(
    [ValidateSet('release', 'debug')]
    [string] $Profile = 'release',

    [string] $InstallRoot,

    [Parameter(ValueFromRemainingArguments = $true)]
    [string[]] $LaunchArguments
)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

. (Join-Path $PSScriptRoot 'Common.ps1')

$repoRoot = Get-UnleashedRepoRoot
$installRoot = Resolve-InstallRoot -InstallRoot $InstallRoot
$cargo = Resolve-CargoExe
$targetDir = Get-CargoTargetDirectory -Cargo $cargo -RepoRoot $repoRoot
$exe = Join-Path $targetDir "$Profile\unleashed.exe"

if (-not (Test-Path -LiteralPath $exe -PathType Leaf)) {
    $buildHint = if ($Profile -eq 'release') {
        '.\scripts\Build-Release.ps1'
    }
    else {
        'cargo build'
    }

    throw "Executable not found: $exe`nBuild it first with: $buildHint"
}

Write-Host "Executable:   $exe"
Write-Host "Install root: $installRoot"
Write-Host "Profile:      $Profile"

Push-Location $installRoot
try {
    & $exe @LaunchArguments
    $exitCode = $LASTEXITCODE
}
finally {
    Pop-Location
}

if ($exitCode -ne 0) {
    throw "unleashed.exe exited with code $exitCode"
}

Write-Host 'Launch exited successfully.' -ForegroundColor Green
