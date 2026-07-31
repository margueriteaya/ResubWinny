param(
    [Parameter(Mandatory = $true)]
    [string]$Source,
    [int]$TrackId = 0,
    [int]$MaxPeakWorkingSetMiB = 384,
    [double]$MinimumInputGiB = 1.0,
    [string]$Worker = ""
)

$ErrorActionPreference = 'Stop'
$repositoryRoot = Split-Path -Parent $PSScriptRoot
$sourcePath = (Resolve-Path -LiteralPath $Source).Path
$sourceBytes = (Get-Item -LiteralPath $sourcePath).Length
$minimumBytes = [int64]($MinimumInputGiB * 1GB)
if ($sourceBytes -lt $minimumBytes) {
    throw "Memory validation requires at least $MinimumInputGiB GiB of input; '$sourcePath' is $([math]::Round($sourceBytes / 1GB, 2)) GiB."
}

if (-not $Worker) {
    $candidate = Join-Path $repositoryRoot 'build\cargo\release\arib-caption-worker.exe'
    if (-not (Test-Path -LiteralPath $candidate)) {
        $candidate = Join-Path $repositoryRoot 'target\release\arib-caption-worker.exe'
    }
    $Worker = $candidate
}
$workerPath = (Resolve-Path -LiteralPath $Worker).Path

$validationRoot = Join-Path ([System.IO.Path]::GetTempPath()) ("resubwinny-memory-" + [guid]::NewGuid().ToString('N'))
[void](New-Item -ItemType Directory -Path $validationRoot)
$outputPath = Join-Path $validationRoot 'memory-gate.ass'
$stdoutPath = Join-Path $validationRoot 'worker.stdout.jsonl'
$stderrPath = Join-Path $validationRoot 'worker.stderr.log'

try {
    $startInfo = [System.Diagnostics.ProcessStartInfo]::new()
    $startInfo.FileName = $workerPath
    $startInfo.UseShellExecute = $false
    $startInfo.CreateNoWindow = $true
    $startInfo.RedirectStandardOutput = $true
    $startInfo.RedirectStandardError = $true
    foreach ($argument in @('convert', $sourcePath, $outputPath, '--archive', '--overwrite')) {
        [void]$startInfo.ArgumentList.Add($argument)
    }
    if ($TrackId -gt 0) {
        [void]$startInfo.ArgumentList.Add('--track-id')
        [void]$startInfo.ArgumentList.Add($TrackId.ToString())
    }

    $process = [System.Diagnostics.Process]::new()
    $process.StartInfo = $startInfo
    if (-not $process.Start()) { throw 'Could not start the Worker memory validation process.' }
    $stdoutTask = $process.StandardOutput.ReadToEndAsync()
    $stderrTask = $process.StandardError.ReadToEndAsync()
    $peakBytes = 0L
    while (-not $process.HasExited) {
        $process.Refresh()
        $peakBytes = [math]::Max($peakBytes, [int64]$process.WorkingSet64)
        Start-Sleep -Milliseconds 100
    }
    $process.Refresh()
    $peakBytes = [math]::Max($peakBytes, [int64]$process.PeakWorkingSet64)
    $stdout = $stdoutTask.GetAwaiter().GetResult()
    $stderr = $stderrTask.GetAwaiter().GetResult()
    [System.IO.File]::WriteAllText($stdoutPath, $stdout)
    [System.IO.File]::WriteAllText($stderrPath, $stderr)
    if ($process.ExitCode -ne 0) {
        throw "Worker exited with code $($process.ExitCode): $stderr"
    }

    $peakMiB = $peakBytes / 1MB
    $inputGiB = $sourceBytes / 1GB
    $result = [ordered]@{
        source = $sourcePath
        inputGiB = [math]::Round($inputGiB, 3)
        peakWorkingSetMiB = [math]::Round($peakMiB, 1)
        maximumMiB = $MaxPeakWorkingSetMiB
        peakMiBPerInputGiB = [math]::Round($peakMiB / $inputGiB, 2)
        passed = $peakMiB -le $MaxPeakWorkingSetMiB
    }
    $result | ConvertTo-Json
    if (-not $result.passed) {
        throw "Worker peak working set $([math]::Round($peakMiB, 1)) MiB exceeded the $MaxPeakWorkingSetMiB MiB release gate."
    }
} finally {
    if (Test-Path -LiteralPath $validationRoot) {
        Remove-Item -LiteralPath $validationRoot -Recurse -Force
    }
}
