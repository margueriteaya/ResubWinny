param(
    [switch]$Long
)

$ErrorActionPreference = 'Stop'
$root = Split-Path -Parent $PSScriptRoot
$fixtureDirectory = $env:ARIB_FIXTURE_DIR
if ([string]::IsNullOrWhiteSpace($fixtureDirectory)) {
    throw 'Set ARIB_FIXTURE_DIR to a legal local recording corpus first.'
}

$worker = if ($env:RESUBWINNY_WORKER_PATH) {
    $env:RESUBWINNY_WORKER_PATH
} else {
    Join-Path $root 'build\cargo\release\arib-caption-worker.exe'
}
if (-not (Test-Path -LiteralPath $worker)) {
    throw "Worker executable was not found: $worker"
}

function Assert-File([string]$Path) {
    if (-not (Test-Path -LiteralPath $Path)) {
        throw "Expected artifact was not created: $Path"
    }
}

function Inspect-Recording([string]$Name, [string]$ExpectedKind) {
    $path = Join-Path $fixtureDirectory $Name
    Assert-File $path
    $json = (& $worker inspect $path | Select-Object -Last 1 | ConvertFrom-Json)
    if ($json.probe.kind -ne $ExpectedKind) {
        throw "$Name was detected as '$($json.probe.kind)', expected '$ExpectedKind'."
    }
    Write-Output "inspect $Name -> $ExpectedKind"
}

function Convert-Recording([string]$Name, [string]$Output, [hashtable]$Options, [hashtable]$Expected, [int]$TrackId = -1) {
    $path = Join-Path $fixtureDirectory $Name
    $arguments = @('convert', $path, $Output) + $Options.Keys.ForEach({ $_ })
    if ($TrackId -ge 0) { $arguments += @('--track-id', $TrackId.ToString()) }
    $events = @(& $worker @arguments | ForEach-Object { $_ | ConvertFrom-Json })
    if ($LASTEXITCODE -ne 0) { throw "$Name conversion failed." }
    $completed = $events | Where-Object { $_.type -eq 'completed' } | Select-Object -Last 1
    if (-not $completed) { throw "$Name conversion emitted no completed event." }
    foreach ($key in $Expected.Keys) {
        $actual = $completed.summary.$key
        if ($actual -ne $Expected[$key]) {
            throw "$Name summary $key was $actual, expected $($Expected[$key])."
        }
    }
    Write-Output "convert $Name -> captions=$($completed.summary.captions), characters=$($completed.summary.characters)"
}

function Assert-ArchiveSummary([string]$Path, [hashtable]$Expected, [bool]$RequireRenderedImage) {
    $summary = $null
    $renderedImages = 0
    foreach ($line in Get-Content -LiteralPath $Path) {
        try { $record = $line | ConvertFrom-Json } catch { continue }
        if ($record.type -eq 'summary') { $summary = $record.value }
        if ($record.type -eq 'scene' -and $record.value.rendered_image) { $renderedImages++ }
    }
    if (-not $summary) { throw "Archive has no summary record: $Path" }
    foreach ($key in $Expected.Keys) {
        if ($summary.$key -ne $Expected[$key]) {
            throw "Archive summary $key was $($summary.$key), expected $($Expected[$key])."
        }
    }
    if ($RequireRenderedImage -and $renderedImages -eq 0) {
        throw "Archive has no rendered scene image: $Path"
    }
    Write-Output "archive $([System.IO.Path]::GetFileName($Path)) -> summary verified; rendered_images=$renderedImages"
}

function Assert-ArchiveCaptionGeometry([string]$Path, [int64]$StartMilliseconds, [int]$X, [int]$Y, [int]$Width, [int]$Height) {
    $caption = $null
    foreach ($line in [System.IO.File]::ReadLines($Path)) {
        try { $candidate = $line | ConvertFrom-Json } catch { continue }
        if ($candidate.type -eq 'caption' -and ([int64]$candidate.value.start_ms) -eq $StartMilliseconds) {
            $caption = $candidate.value
            break
        }
    }
    if ($null -eq $caption) { throw "Archive has no caption beginning at $StartMilliseconds ms: $Path" }
    if ($caption.x -ne $X -or $caption.y -ne $Y -or $caption.width -ne $Width -or $caption.height -ne $Height) {
        throw "Archive caption geometry at $StartMilliseconds ms was ($($caption.x), $($caption.y), $($caption.width), $($caption.height)); expected ($X, $Y, $Width, $Height)."
    }
    Write-Output "archive $([System.IO.Path]::GetFileName($Path)) -> 4K plane geometry verified at $StartMilliseconds ms"
}

Inspect-Recording 'chijo_digital_test.ts' 'mpeg_ts'
Inspect-Recording 'bs4k_test.m2ts' 'm2ts'
Inspect-Recording 'bs4k_test_2.ts' 'mpeg_ts'

if ($Long) {
    $validationDirectory = Join-Path ([System.IO.Path]::GetTempPath()) 'resubwinny-corpus-validation'
    New-Item -ItemType Directory -Force -Path $validationDirectory | Out-Null
    $terrestrialOutput = Join-Path $validationDirectory 'chijo_digital_test.ass'
    $bs4kOutput = Join-Path $validationDirectory 'bs4k_test.ass'
    $bs4kB24Output = Join-Path $validationDirectory 'bs4k_test_2.ass'
    $bs4kB24InactiveOutput = Join-Path $validationDirectory 'bs4k_test_2_inactive.ass'

    Convert-Recording 'chijo_digital_test.ts' $terrestrialOutput @{
        '--archive' = $true; '--raw' = $true; '--drcs-report' = $true; '--overwrite' = $true
    } @{ pes_packets = 13653; captions = 2230; regions = 2736; characters = 29892; drcs_glyphs = 61; decoder_errors = 0 }
    Convert-Recording 'bs4k_test.m2ts' $bs4kOutput @{
        '--ttml' = $true; '--archive' = $true; '--raw' = $true; '--overwrite' = $true
    } @{ pes_packets = 330; captions = 422; characters = 5051; decoder_errors = 0 }
    Convert-Recording 'bs4k_test_2.ts' $bs4kB24Output @{
        '--archive' = $true; '--raw' = $true; '--drcs-report' = $true; '--overwrite' = $true
    } @{ pes_packets = 2038; captions = 118; regions = 157; characters = 1661; drcs_glyphs = 0; decoder_errors = 0 } -TrackId 304
    Convert-Recording 'bs4k_test_2.ts' $bs4kB24InactiveOutput @{
        '--archive' = $true; '--raw' = $true; '--overwrite' = $true
    } @{ pes_packets = 0; captions = 0; regions = 0; characters = 0; drcs_glyphs = 0; decoder_errors = 0 } -TrackId 312

    Assert-File $terrestrialOutput
    $terrestrialArchive = [System.IO.Path]::ChangeExtension($terrestrialOutput, '.caption.jsonl')
    Assert-File $terrestrialArchive
    Assert-ArchiveSummary $terrestrialArchive @{ captions = 2230; regions = 2736; characters = 29892; drcs_glyphs = 61 } $true
    Assert-File $bs4kOutput
    $bs4kArchive = [System.IO.Path]::ChangeExtension($bs4kOutput, '.caption.jsonl')
    Assert-File $bs4kArchive
    Assert-ArchiveSummary $bs4kArchive @{ captions = 422; characters = 5051 } $false
    Assert-ArchiveCaptionGeometry $bs4kArchive 23000 276 838 160 120
    Assert-File $bs4kB24Output
    $bs4kB24Archive = [System.IO.Path]::ChangeExtension($bs4kB24Output, '.caption.jsonl')
    Assert-File $bs4kB24Archive
    Assert-ArchiveSummary $bs4kB24Archive @{ captions = 118; regions = 157; characters = 1661; drcs_glyphs = 0 } $true
    Assert-File $bs4kB24InactiveOutput
    $bs4kB24InactiveArchive = [System.IO.Path]::ChangeExtension($bs4kB24InactiveOutput, '.caption.jsonl')
    Assert-File $bs4kB24InactiveArchive
    Assert-ArchiveSummary $bs4kB24InactiveArchive @{ captions = 0; regions = 0; characters = 0; drcs_glyphs = 0 } $false
    Write-Output "long validation artifacts -> $validationDirectory"
}

Write-Output 'corpus validation passed'
