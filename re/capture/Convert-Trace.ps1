param(
    [Parameter(Mandatory=$true)]
    [string]$InputPath,

    [string]$CsvPath = ''
)

$ErrorActionPreference = 'Stop'

if (-not (Test-Path $InputPath)) {
    throw "Trace not found: $InputPath"
}

if (-not $CsvPath) {
    $CsvPath = [IO.Path]::ChangeExtension([IO.Path]::GetFullPath($InputPath), '.csv')
}

$lines = Get-Content -LiteralPath $InputPath
if (-not $lines) {
    throw 'Trace is empty.'
}

$rows = New-Object System.Collections.Generic.List[object]
$meta = $null
$sampleCount = 0
$invalidSamples = 0
$stateProbeCount = 0
$dtProbeCount = 0
$cameraProbeCount = 0

foreach ($line in $lines) {
    if ([string]::IsNullOrWhiteSpace($line)) {
        continue
    }

    try {
        $record = $line | ConvertFrom-Json
    }
    catch {
        throw "Invalid JSONL record: $line"
    }

    switch ($record.kind) {
        'meta' {
            if (-not $meta) { $meta = $record }
        }
        'sample' {
            ++$sampleCount
            if (-not $record.proxy_valid) { ++$invalidSamples }

            $rows.Add([pscustomobject]@{
                frame_index      = $record.frame_index
                guest_pc         = $record.guest_pc
                mode             = $record.mode
                dt_seconds       = $record.dt_seconds
                guest_dt_seconds = $record.guest_dt_seconds
                guest_dt_pc      = $record.guest_dt_pc
                proxy_guest      = $record.proxy_guest
                proxy_valid      = $record.proxy_valid
                pos_x            = $record.position.x
                pos_y            = $record.position.y
                pos_z            = $record.position.z
                vel_x            = $record.velocity.x
                vel_y            = $record.velocity.y
                vel_z            = $record.velocity.z
                speed            = $record.velocity.magnitude
                state_status     = $record.state.status
                state_seen       = $record.state.seen
                state_epoch      = $record.state.epoch
                state_r29        = $record.state.r29
            })
        }
        'state_probe' { ++$stateProbeCount }
        'dt_probe' { ++$dtProbeCount }
        'camera_probe' { ++$cameraProbeCount }
    }
}

if (-not $meta) {
    throw 'Trace contains no meta record.'
}

$rows | Export-Csv -LiteralPath $CsvPath -NoTypeInformation -Encoding utf8

Write-Host "reference_commit : $($meta.reference_commit)"
Write-Host "proxy_guest      : $($meta.proxy_guest)"
Write-Host "discovery        : $($meta.discovery)"
Write-Host "samples          : $sampleCount"
Write-Host "invalid samples  : $invalidSamples"
Write-Host "state probes     : $stateProbeCount"
Write-Host "dt probes        : $dtProbeCount"
Write-Host "camera probes    : $cameraProbeCount"
Write-Host "csv              : $CsvPath"

if ($sampleCount -eq 0) {
    throw 'No posture sample records were captured.'
}

if ($meta.proxy_guest -ne '0x00000000' -and $invalidSamples -gt 0) {
    Write-Warning "$invalidSamples sample(s) had an invalid/non-finite proxy snapshot. Re-verify UNLEASHED_TRACE_PROXY."
}

Write-Output $CsvPath
