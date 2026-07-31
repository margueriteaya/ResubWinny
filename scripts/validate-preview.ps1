param(
    [string]$FixtureDirectory = $env:ARIB_FIXTURE_DIR,
    [switch]$Long,
    [int]$DurationSeconds = 120,
    [double]$MinimumPresentsPerSecond = 20,
    [double]$MaximumPeakWorkingSetMiB = 2048,
    [double]$MaximumWorkingSetGrowthMiB = 512,
    [double]$MaximumStartupMs = 10000,
    [double]$MaximumControlLatencyMs = 1000,
    [double]$MaximumCaptionUploadMs = 1000,
    [double]$MaximumShutdownMs = 3000,
    [string]$ReportPath = ""
)

$ErrorActionPreference = 'Stop'
$repositoryRoot = Split-Path -Parent $PSScriptRoot
$manifestPath = Join-Path $repositoryRoot 'studio-tauri\src-tauri\Cargo.toml'

if (-not $FixtureDirectory) {
    throw 'Set ARIB_FIXTURE_DIR or pass -FixtureDirectory with the local recording corpus.'
}

$sample = Join-Path $FixtureDirectory 'bs4k_test_2.ts'
if (-not (Test-Path -LiteralPath $sample -PathType Leaf)) {
    throw "Missing native preview smoke sample: $sample"
}

$env:RESUBWINNY_RENDER_SMOKE_SOURCE = $sample
cargo test --manifest-path $manifestPath --no-run
if ($LASTEXITCODE -ne 0) {
    throw "Could not compile the native preview gate (exit code $LASTEXITCODE)."
}
cargo test --manifest-path $manifestPath render_worker_starts_and_stops_on_a_real_recording -- --ignored --test-threads=1
if ($LASTEXITCODE -ne 0) {
    throw "Native preview smoke gate failed with exit code $LASTEXITCODE."
}

if (-not $Long) {
    return
}

if (-not $ReportPath) {
    $reportDirectory = Join-Path $repositoryRoot 'build\validation'
    [void](New-Item -ItemType Directory -Force -Path $reportDirectory)
    $ReportPath = Join-Path $reportDirectory 'preview-performance-windows-4k.json'
}
$resolvedReportDirectory = Split-Path -Parent ([System.IO.Path]::GetFullPath($ReportPath))
[void](New-Item -ItemType Directory -Force -Path $resolvedReportDirectory)

$env:RESUBWINNY_RENDER_GATE_SECONDS = $DurationSeconds.ToString([System.Globalization.CultureInfo]::InvariantCulture)
$env:RESUBWINNY_RENDER_GATE_MIN_FPS = $MinimumPresentsPerSecond.ToString([System.Globalization.CultureInfo]::InvariantCulture)
$env:RESUBWINNY_RENDER_GATE_MAX_RSS_MIB = $MaximumPeakWorkingSetMiB.ToString([System.Globalization.CultureInfo]::InvariantCulture)
$env:RESUBWINNY_RENDER_GATE_MAX_GROWTH_MIB = $MaximumWorkingSetGrowthMiB.ToString([System.Globalization.CultureInfo]::InvariantCulture)
$env:RESUBWINNY_RENDER_GATE_MAX_STARTUP_MS = $MaximumStartupMs.ToString([System.Globalization.CultureInfo]::InvariantCulture)
$env:RESUBWINNY_RENDER_GATE_MAX_CONTROL_MS = $MaximumControlLatencyMs.ToString([System.Globalization.CultureInfo]::InvariantCulture)
$env:RESUBWINNY_RENDER_GATE_MAX_OVERLAY_MS = $MaximumCaptionUploadMs.ToString([System.Globalization.CultureInfo]::InvariantCulture)
$env:RESUBWINNY_RENDER_GATE_MAX_SHUTDOWN_MS = $MaximumShutdownMs.ToString([System.Globalization.CultureInfo]::InvariantCulture)
$env:RESUBWINNY_RENDER_GATE_REPORT = [System.IO.Path]::GetFullPath($ReportPath)

cargo test --manifest-path $manifestPath render_worker_meets_the_long_4k_performance_gate -- --ignored --test-threads=1 --nocapture
if ($LASTEXITCODE -ne 0) {
    throw "Native preview 4K performance gate failed with exit code $LASTEXITCODE."
}
Write-Output "Native preview performance report: $($env:RESUBWINNY_RENDER_GATE_REPORT)"
