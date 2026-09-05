<#
.SYNOPSIS
Validates scoped B62 DRCS conflict, report, mapping, and retry semantics against a private native TLV capture.

.DESCRIPTION
All generated captions, mappings, reports, and extracted resources stay in an isolated temporary directory and are removed before exit. The command prints only aggregate validation counts.
#>
param(
    [Parameter(Mandatory = $true)]
    [string]$Source,
    [string]$Worker = "",
    [string]$Replacement = "映",
    [int]$MaxMappingPasses = 16
)

$ErrorActionPreference = 'Stop'
$repositoryRoot = Split-Path -Parent $PSScriptRoot
$sourcePath = (Resolve-Path -LiteralPath $Source).Path
if (-not (Test-Path -LiteralPath $sourcePath -PathType Leaf)) {
    throw 'The B62 validation source must be a readable file.'
}
if ([string]::IsNullOrEmpty($Replacement)) {
    throw 'Replacement must contain explicit mapping text.'
}
if ($MaxMappingPasses -lt 1) {
    throw 'MaxMappingPasses must be at least one.'
}

if (-not $Worker) {
    $Worker = Join-Path $repositoryRoot 'build\cargo\release\arib-caption-worker.exe'
}
$workerPath = (Resolve-Path -LiteralPath $Worker).Path
$validationRoot = Join-Path ([System.IO.Path]::GetTempPath()) ("resubwinny-b62-drcs-" + [guid]::NewGuid().ToString('N'))
[void](New-Item -ItemType Directory -Path $validationRoot)
$outputPath = Join-Path $validationRoot 'validation.ass'
$publishedPath = [System.IO.Path]::ChangeExtension($outputPath, '.srt')
$reportPath = [System.IO.Path]::ChangeExtension($outputPath, '.drcs.json')
$assetDirectory = [System.IO.Path]::ChangeExtension($outputPath, '.drcs')
$mappingPath = Join-Path $validationRoot 'mapping.json'

function Invoke-Worker([string[]]$Arguments, [string]$Label) {
    $startInfo = [System.Diagnostics.ProcessStartInfo]::new()
    $startInfo.FileName = $workerPath
    $startInfo.UseShellExecute = $false
    $startInfo.CreateNoWindow = $true
    $startInfo.RedirectStandardOutput = $true
    $startInfo.RedirectStandardError = $true
    foreach ($argument in $Arguments) {
        [void]$startInfo.ArgumentList.Add($argument)
    }
    $process = [System.Diagnostics.Process]::new()
    $process.StartInfo = $startInfo
    if (-not $process.Start()) { throw "Could not start Worker for $Label." }
    $stdoutTask = $process.StandardOutput.ReadToEndAsync()
    $stderrTask = $process.StandardError.ReadToEndAsync()
    $process.WaitForExit()
    $stdout = $stdoutTask.GetAwaiter().GetResult()
    $stderr = $stderrTask.GetAwaiter().GetResult()
    $events = @()
    foreach ($line in $stdout -split "`r?`n") {
        if ([string]::IsNullOrWhiteSpace($line)) { continue }
        try { $events += $line | ConvertFrom-Json } catch {
            throw "Worker emitted invalid JSON during ${Label}: $line"
        }
    }
    [pscustomobject]@{
        ExitCode = $process.ExitCode
        Events = $events
        Stderr = $stderr
    }
}

function Assert-NoCaptionPublication {
    if (Test-Path -LiteralPath $publishedPath) {
        throw 'A B62 DRCS conflict published the selected caption output.'
    }
    $partial = Get-ChildItem -LiteralPath $validationRoot -Filter '*.part' -File
    if ($partial) {
        throw 'A B62 DRCS conflict left a publishable partial caption artifact.'
    }
}

try {
    $inspection = Invoke-Worker @('inspect', $sourcePath) 'inspection'
    if ($inspection.ExitCode -ne 0) {
        throw "B62 inspection failed (exit $($inspection.ExitCode))."
    }
    $inspectionResult = $inspection.Events | Where-Object { $_.type -eq 'input_probe' } | Select-Object -Last 1
    if (-not $inspectionResult -or $inspectionResult.probe.kind -ne 'tlv') {
        throw 'The validation source is not a content-probed native TLV recording.'
    }

    $baseArguments = @(
        'convert', $sourcePath, $outputPath,
        '--srt', '--no-ass', '--drcs-report',
        '--drop-position', '--drop-color', '--drop-ruby', '--overwrite'
    )
    $mappings = [ordered]@{}
    $conflictPasses = 0
    $completed = $null

    for ($pass = 1; $pass -le $MaxMappingPasses; $pass++) {
        $arguments = $baseArguments
        if ($mappings.Count -gt 0) {
            $mappings | ConvertTo-Json | Set-Content -LiteralPath $mappingPath -Encoding utf8NoBOM
            $arguments += @('--drcs-map', $mappingPath)
        }
        $run = Invoke-Worker $arguments "mapping pass $pass"
        if ($run.ExitCode -eq 0) {
            $completed = $run.Events | Where-Object { $_.type -eq 'completed' } | Select-Object -Last 1
            if (-not $completed) { throw 'Successful B62 conversion emitted no completed event.' }
            break
        }

        $failure = $run.Events | Where-Object { $_.type -eq 'failed' -and $_.code -eq 'export_conflict' } | Select-Object -Last 1
        if (-not $failure -or $failure.parameters.feature -ne 'drcs' -or $failure.parameters.issue_code -ne 'unresolved_drcs_text_target') {
            throw "B62 conversion failed for a reason other than unresolved scoped DRCS (exit $($run.ExitCode))."
        }
        Assert-NoCaptionPublication
        if (-not (Test-Path -LiteralPath $reportPath -PathType Leaf)) {
            throw 'The DRCS conflict produced no mapping report.'
        }
        $reportEvent = $run.Events | Where-Object {
            $_.type -eq 'artifact-created' -and $_.kind -eq 'drcs-report'
        } | Select-Object -Last 1
        if (-not $reportEvent) { throw 'The DRCS report was not recorded as a completed diagnostic artifact.' }

        $report = Get-Content -LiteralPath $reportPath -Raw | ConvertFrom-Json
        if ($report.version -ne 2 -or $report.glyphs.Count -lt 1 -or $report.glyphs.Count -gt 64) {
            throw 'The native B62 DRCS report has an invalid version or glyph count.'
        }
        $added = 0
        $assetRoot = [System.IO.Path]::GetFullPath($assetDirectory).TrimEnd('\') + '\'
        foreach ($glyph in $report.glyphs) {
            if ($glyph.mapping_id -notmatch '^b62:sha256:[0-9a-f]{64}:u\+[0-9A-F]+$') {
                throw 'The B62 report contains a non-scoped mapping identity.'
            }
            $assetPath = [System.IO.Path]::GetFullPath([string]$glyph.asset)
            if (-not $assetPath.StartsWith($assetRoot, [System.StringComparison]::OrdinalIgnoreCase)) {
                throw 'The B62 report references an asset outside its isolated directory.'
            }
            if (-not (Test-Path -LiteralPath $assetPath -PathType Leaf)) {
                throw 'The B62 report references a missing resource asset.'
            }
            if (-not $mappings.Contains([string]$glyph.mapping_id)) {
                $mappings[[string]$glyph.mapping_id] = $Replacement
                $added++
            }
        }
        if ($added -eq 0) { throw 'The B62 mapping retry made no progress.' }
        $conflictPasses++
    }

    if (-not $completed) {
        throw "B62 mapping did not converge within $MaxMappingPasses passes."
    }
    if (-not (Test-Path -LiteralPath $publishedPath -PathType Leaf)) {
        throw 'Mapped B62 conversion did not publish the selected SRT artifact.'
    }
    if (-not (Select-String -LiteralPath $publishedPath -SimpleMatch $Replacement -Quiet)) {
        throw 'The published SRT does not contain the explicit DRCS replacement.'
    }
    if (Get-ChildItem -LiteralPath $validationRoot -Filter '*.part' -File) {
        throw 'Completed B62 validation left a partial artifact.'
    }

    [ordered]@{
        passed = $true
        inputBytes = (Get-Item -LiteralPath $sourcePath).Length
        conflictPasses = $conflictPasses
        scopedMappings = $mappings.Count
    } | ConvertTo-Json
} finally {
    if (Test-Path -LiteralPath $validationRoot) {
        Remove-Item -LiteralPath $validationRoot -Recurse -Force
    }
}
