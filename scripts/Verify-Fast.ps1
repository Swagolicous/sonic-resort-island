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
    Invoke-Cargo -Cargo $cargo -Arguments @('fmt', '--all', '--', '--check')
    Invoke-Cargo -Cargo $cargo -Arguments @('test', '--workspace')
    Invoke-Cargo -Cargo $cargo -Arguments @('check', '--workspace', '--all-targets')
}
finally {
    Pop-Location
}

Write-Host 'Fast verification passed.' -ForegroundColor Green
