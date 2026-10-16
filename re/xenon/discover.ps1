# Discovery loop: run a guest-host example, add the guest functions it reports as
# `missing {..}` to sheets/lift_roots.csv, relift, rebuild, and repeat until a run
# reports none (or -Rounds is reached).
#
# usage: re/xenon/discover.ps1 -Example blocks -ExampleArgs C:\snaps\one\82346F08_pre.bin,7 [-System havok] [-Rounds 8]
param(
    [Parameter(Mandatory)] [string] $Example,
    [string[]] $ExampleArgs = @(),
    [string] $System = "engine",
    [int] $Rounds = 8
)
$rs = Resolve-Path "$PSScriptRoot\..\.."
Set-Location $rs
$env:PYTHONIOENCODING = "utf-8"
for ($round = 1; $round -le $Rounds; $round++) {
    # rustc sometimes dies with STATUS_ACCESS_VIOLATION on the lifted crate; a retry builds.
    foreach ($attempt in 1..3) {
        cargo build --release -p unleashed-guest-host --example $Example 2>&1 | Select-String -Pattern '^error|Finished' | Select-Object -Last 5
        if ($LASTEXITCODE -eq 0) { break }
    }
    if ($LASTEXITCODE -ne 0) { "round ${round}: build failed"; exit 1 }
    $out = & ".\target\release\examples\$Example.exe" @ExampleArgs 2>&1
    $code = $LASTEXITCODE
    # @(): one result must stay an array, or splatting it below passes its characters.
    $missing = @($out | Select-String -Pattern 'missing \{([^}]*)\}' -AllMatches | ForEach-Object { $_.Matches } |
        ForEach-Object { $_.Groups[1].Value -split ',\s*' } | Where-Object { $_ } | Sort-Object -Unique)
    "round ${round}: exit $code, missing: $($missing -join ' ')"
    if (-not $missing) { $out | Select-Object -Last 25; "discover: clean after $round round(s)"; exit 0 }
    $added = python re/xenon/add_roots.py $System "Reached by $Example.rs." @missing
    "  $added"
    if ($added -match '\+0$') { $out | Select-Object -Last 25; "discover: the missing targets are not guest functions (a skipped call returned garbage)"; exit 2 }
    python sheets/sheetc.py lift 2>&1 | Select-Object -Last 2
    # A function the lifter cannot translate stays a skipped call (lift_failures.json); only a
    # sheet error stops the loop.
    if ($LASTEXITCODE -ne 0 -and -not (Test-Path crates\unleashed-guest\lift_failures.json)) { "round ${round}: lift failed"; exit 1 }
}
"discover: still missing after $Rounds rounds"
exit 3
