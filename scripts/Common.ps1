Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

function Get-UnleashedRepoRoot {
    return (Split-Path -Parent $PSScriptRoot)
}

function Resolve-CargoExe {
    $candidates = [System.Collections.Generic.List[string]]::new()

    foreach ($name in @('cargo.exe', 'cargo')) {
        $command = Get-Command $name -ErrorAction SilentlyContinue
        if ($null -ne $command -and -not [string]::IsNullOrWhiteSpace($command.Source)) {
            $candidates.Add($command.Source)
        }
    }

    if (-not [string]::IsNullOrWhiteSpace($env:CARGO_HOME)) {
        $candidates.Add((Join-Path $env:CARGO_HOME 'bin\cargo.exe'))
    }

    if (-not [string]::IsNullOrWhiteSpace($HOME)) {
        $candidates.Add((Join-Path $HOME '.cargo\bin\cargo.exe'))
    }

    foreach ($candidate in $candidates | Select-Object -Unique) {
        if (Test-Path -LiteralPath $candidate -PathType Leaf) {
            return (Resolve-Path -LiteralPath $candidate).Path
        }
    }

    throw @'
Could not find cargo.exe.
Install Rust with rustup, or ensure Cargo is available through PATH, CARGO_HOME, or %USERPROFILE%\.cargo\bin.
'@
}

function Resolve-InstallRoot {
    param(
        [string] $InstallRoot
    )

    $repoRoot = Get-UnleashedRepoRoot

    if ([string]::IsNullOrWhiteSpace($InstallRoot)) {
        $InstallRoot = Split-Path -Parent $repoRoot
    }

    if (-not (Test-Path -LiteralPath $InstallRoot -PathType Container)) {
        throw "Install root does not exist: $InstallRoot"
    }

    $resolved = (Resolve-Path -LiteralPath $InstallRoot).Path
    $gameDir = Join-Path $resolved 'game'

    if (-not (Test-Path -LiteralPath $gameDir -PathType Container)) {
        throw "Install root is missing the required game directory: $gameDir"
    }

    return $resolved
}

function Invoke-Cargo {
    param(
        [Parameter(Mandatory = $true)]
        [string] $Cargo,

        [Parameter(Mandatory = $true)]
        [string[]] $Arguments
    )

    Write-Host "> cargo $($Arguments -join ' ')" -ForegroundColor Cyan
    & $Cargo @Arguments

    if ($LASTEXITCODE -ne 0) {
        throw "cargo command failed with exit code $LASTEXITCODE`: cargo $($Arguments -join ' ')"
    }
}

function Get-CargoTargetDirectory {
    param(
        [Parameter(Mandatory = $true)]
        [string] $Cargo,

        [Parameter(Mandatory = $true)]
        [string] $RepoRoot
    )

    Push-Location $RepoRoot
    try {
        $metadataText = (& $Cargo 'metadata' '--no-deps' '--format-version' '1' | Out-String)
        if ($LASTEXITCODE -ne 0) {
            throw "cargo metadata failed with exit code $LASTEXITCODE"
        }

        $metadata = $metadataText | ConvertFrom-Json
        if ([string]::IsNullOrWhiteSpace($metadata.target_directory)) {
            throw 'cargo metadata did not return target_directory'
        }

        return $metadata.target_directory
    }
    finally {
        Pop-Location
    }
}

function Assert-NonEmptyFile {
    param(
        [Parameter(Mandatory = $true)]
        [string] $Path
    )

    if (-not (Test-Path -LiteralPath $Path -PathType Leaf)) {
        throw "Required file is missing: $Path"
    }

    $item = Get-Item -LiteralPath $Path
    if ($item.Length -le 0) {
        throw "Required file is empty: $Path"
    }

    return $item
}
