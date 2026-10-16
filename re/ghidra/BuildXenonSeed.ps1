param(
    [Parameter(Mandatory = $true)]
    [string]$MappingPath,

    [Parameter(Mandatory = $true)]
    [string]$ConfigPath,

    [Parameter(Mandatory = $true)]
    [string]$OutputPath,

    [string]$BinaryPath
)

$ErrorActionPreference = 'Stop'

function Get-Sha256([string]$Path) {
    if ([string]::IsNullOrWhiteSpace($Path) -or -not (Test-Path -LiteralPath $Path -PathType Leaf)) {
        return $null
    }
    return (Get-FileHash -LiteralPath $Path -Algorithm SHA256).Hash.ToLowerInvariant()
}

function Json-Escape([string]$Value) {
    if ($null -eq $Value) { return '' }
    $sb = New-Object System.Text.StringBuilder
    foreach ($ch in $Value.ToCharArray()) {
        switch ([int][char]$ch) {
            8  { [void]$sb.Append('\b') }
            9  { [void]$sb.Append('\t') }
            10 { [void]$sb.Append('\n') }
            12 { [void]$sb.Append('\f') }
            13 { [void]$sb.Append('\r') }
            34 { [void]$sb.Append('\"') }
            92 { [void]$sb.Append('\\') }
            default {
                $code = [int][char]$ch
                if ($code -lt 0x20) {
                    [void]$sb.Append(('\u{0:x4}' -f $code))
                } else {
                    [void]$sb.Append($ch)
                }
            }
        }
    }
    return $sb.ToString()
}

$mapping = (Resolve-Path -LiteralPath $MappingPath).Path
$config = (Resolve-Path -LiteralPath $ConfigPath).Path
$output = [System.IO.Path]::GetFullPath($OutputPath)
$outputDir = Split-Path -Parent $output
if ($outputDir) {
    New-Item -ItemType Directory -Force $outputDir | Out-Null
}

$mappingSha = Get-Sha256 $mapping
$configSha = Get-Sha256 $config
$binaryResolved = $null
$binarySha = $null
if ($BinaryPath) {
    $binaryResolved = (Resolve-Path -LiteralPath $BinaryPath).Path
    $binarySha = Get-Sha256 $binaryResolved
}

$manualBounds = @{}
$configText = Get-Content -LiteralPath $config -Raw
$boundRegex = [regex]'\{\s*address\s*=\s*0x(?<address>[0-9A-Fa-f]+)\s*,\s*size\s*=\s*0x(?<size>[0-9A-Fa-f]+)\s*\}'
foreach ($m in $boundRegex.Matches($configText)) {
    $addr = [Convert]::ToUInt64($m.Groups['address'].Value, 16)
    $size = [Convert]::ToUInt64($m.Groups['size'].Value, 16)
    $manualBounds[$addr] = $size
}

$mapRegex = [regex]'\{\s*0x(?<address>[0-9A-Fa-f]+)\s*,\s*(?<name>[A-Za-z_][A-Za-z0-9_]*)\s*\},'
$seen = New-Object 'System.Collections.Generic.HashSet[UInt64]'
$rows = New-Object 'System.Collections.Generic.List[object]'

Get-Content -LiteralPath $mapping | ForEach-Object {
    $m = $mapRegex.Match($_)
    if (-not $m.Success) { return }

    $address = [Convert]::ToUInt64($m.Groups['address'].Value, 16)
    $name = $m.Groups['name'].Value
    if (-not $seen.Add($address)) {
        throw ('duplicate mapping address 0x{0:X}' -f $address)
    }

    $manual = $manualBounds.ContainsKey($address)
    $size = if ($manual) { [UInt64]$manualBounds[$address] } else { $null }

    $nameKind =
        if ($name -match '^sub_[0-9A-Fa-f]+$') { 'synthetic_address' }
        elseif ($name -match '^__(rest|save)(gpr|fpr|vmx)') { 'runtime_helper' }
        elseif ($name -eq '_xstart') { 'entrypoint' }
        elseif ($name -match '^__imp__') { 'xex_import' }
        else { 'xenonrecomp_symbol' }

    $nameConfidence =
        if ($nameKind -eq 'synthetic_address') { 'derived' }
        else { 'high' }

    $boundaryConfidence =
        if ($manual) { 'explicit' }
        else { 'xenonrecomp_analysis' }

    $rows.Add([pscustomobject]@{
        Address = $address
        Name = $name
        NameKind = $nameKind
        NameConfidence = $nameConfidence
        BoundaryConfidence = $boundaryConfidence
        ExplicitSize = $size
    })
}

$missingManual = @($manualBounds.Keys | Where-Object { -not $seen.Contains([UInt64]$_) })
if ($missingManual.Count -gt 0) {
    throw ('{0} explicit function boundaries are absent from ppc_func_mapping.cpp; first=0x{1:X}' -f
        $missingManual.Count, [UInt64]$missingManual[0])
}

$rows = @($rows | Sort-Object Address)

$tmp = $output + '.tmp'
$utf8NoBom = New-Object System.Text.UTF8Encoding($false)
$writer = New-Object System.IO.StreamWriter($tmp, $false, $utf8NoBom)
try {
    foreach ($r in $rows) {
        $addrHex = '0x{0:X8}' -f $r.Address
        $sizeJson = if ($null -eq $r.ExplicitSize) { 'null' } else { [string][UInt64]$r.ExplicitSize }
        $sizeHexJson = if ($null -eq $r.ExplicitSize) { 'null' } else { '"0x{0:X}"' -f [UInt64]$r.ExplicitSize }
        $binaryPathJson = if ($binaryResolved) { '"' + (Json-Escape $binaryResolved) + '"' } else { 'null' }
        $binaryShaJson = if ($binarySha) { '"' + $binarySha + '"' } else { 'null' }

        $line =
            '{"schema":"unleashed_re.xenon_seed.v1",' +
            '"record_type":"function_seed",' +
            '"address_hex":"' + $addrHex + '",' +
            '"address_u64":' + [string][UInt64]$r.Address + ',' +
            '"name":"' + (Json-Escape $r.Name) + '",' +
            '"name_kind":"' + $r.NameKind + '",' +
            '"explicit_size":' + $sizeJson + ',' +
            '"explicit_size_hex":' + $sizeHexJson + ',' +
            '"confidence":{"address":"high","name":"' + $r.NameConfidence + '","boundary":"' + $r.BoundaryConfidence + '"},' +
            '"provenance":{' +
                '"producer":"BuildXenonSeed.ps1",' +
                '"mapping_path":"' + (Json-Escape $mapping) + '",' +
                '"mapping_sha256":"' + $mappingSha + '",' +
                '"config_path":"' + (Json-Escape $config) + '",' +
                '"config_sha256":"' + $configSha + '",' +
                '"binary_path":' + $binaryPathJson + ',' +
                '"binary_sha256":' + $binaryShaJson +
            '}}'
        $writer.WriteLine($line)
    }
}
finally {
    $writer.Dispose()
}

Move-Item -Force $tmp $output

$syntheticCount = @($rows | Where-Object NameKind -eq 'synthetic_address').Count
$explicitCount = @($rows | Where-Object { $null -ne $_.ExplicitSize }).Count

Write-Output ("wrote {0} seed records to {1}" -f $rows.Count, $output)
Write-Output ("synthetic_names={0} explicit_boundaries={1}" -f $syntheticCount, $explicitCount)
Write-Output ("mapping_sha256={0}" -f $mappingSha)
Write-Output ("config_sha256={0}" -f $configSha)
if ($binarySha) {
    Write-Output ("binary_sha256={0}" -f $binarySha)
}
