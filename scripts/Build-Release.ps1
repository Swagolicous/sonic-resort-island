[CmdletBinding()]
param()

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

. (Join-Path $PSScriptRoot 'Common.ps1')

$repoRoot = Get-UnleashedRepoRoot
$cargo = Resolve-CargoExe

Write-Host "Repository: $repoRoot"
Write-Host "Cargo:      $cargo"

Push-Location $repoRoot
try {
    Invoke-Cargo -Cargo $cargo -Arguments @('build', '--workspace', '--release')
}
finally {
    Pop-Location
}

$targetDir = Get-CargoTargetDirectory -Cargo $cargo -RepoRoot $repoRoot
$exe = Join-Path $targetDir 'release\unleashed.exe'

if (-not (Test-Path -LiteralPath $exe -PathType Leaf)) {
    throw "Release build completed, but the expected launcher executable was not found: $exe"
}

Write-Host "Release build ready: $exe" -ForegroundColor Green
