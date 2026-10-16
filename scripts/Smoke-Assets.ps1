[CmdletBinding()]
param(
    [string] $InstallRoot
)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

. (Join-Path $PSScriptRoot 'Common.ps1')

$repoRoot = Get-UnleashedRepoRoot
$installRoot = Resolve-InstallRoot -InstallRoot $InstallRoot
$cargo = Resolve-CargoExe

Write-Host "Repository:   $repoRoot"
Write-Host "Install root: $installRoot"
Write-Host "Cargo:        $cargo"

$requiredDirectories = @(
    'game',
    'update',
    'dlc',
    'game\Packed\ActD_MykonosAct1',
    'update\work\ActD_MykonosAct1',
    'dlc\Apotos & Shamar Adventure Pack\Additional\ActD_MykonosAct1'
)

foreach ($relativePath in $requiredDirectories) {
    $path = Join-Path $installRoot $relativePath
    if (-not (Test-Path -LiteralPath $path -PathType Container)) {
        throw "Required asset directory is missing: $path"
    }
    Write-Host "OK dir  $relativePath"
}

$requiredFiles = @(
    'game\#ActD_MykonosAct1.arl',
    'game\#ActD_MykonosAct1.ar.00',
    'game\ActD_MykonosAct1.arl',
    'game\ActD_MykonosAct1.ar.00',
    'game\Packed\ActD_MykonosAct1\Stage.pfd',
    'update\work\ActD_MykonosAct1\Base.set.xml',
    'dlc\Apotos & Shamar Adventure Pack\Additional\ActD_MykonosAct1\Stage-Add.pfd'
)

foreach ($relativePath in $requiredFiles) {
    $path = Join-Path $installRoot $relativePath
    $item = Assert-NonEmptyFile -Path $path
    Write-Host ("OK file {0} ({1:N0} bytes)" -f $relativePath, $item.Length)
}

$inspectionTargets = @(
    'game\#ActD_MykonosAct1.arl',
    'game\#ActD_MykonosAct1.ar.00',
    'game\Packed\ActD_MykonosAct1\Stage.pfd'
)

Push-Location $repoRoot
try {
    foreach ($relativePath in $inspectionTargets) {
        $path = Join-Path $installRoot $relativePath
        Write-Host "`nInspecting $relativePath" -ForegroundColor Cyan
        Invoke-Cargo -Cargo $cargo -Arguments @(
            'run',
            '--quiet',
            '-p',
            'asset-inspector',
            '--',
            $path
        )
    }
}
finally {
    Pop-Location
}

Write-Host "`nAsset smoke checks passed." -ForegroundColor Green
