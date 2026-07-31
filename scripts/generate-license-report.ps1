param(
    [switch]$Check
)

$ErrorActionPreference = 'Stop'
$root = (Resolve-Path (Join-Path $PSScriptRoot '..')).Path
$outputPath = Join-Path $root 'docs\dependency-licenses.md'

function Get-CargoPackages([string]$ManifestPath, [string]$Component) {
    $json = & cargo metadata --locked --format-version 1 --manifest-path $ManifestPath 2>$null
    if ($LASTEXITCODE -ne 0) {
        throw "cargo metadata failed for $ManifestPath"
    }

    $metadata = ($json -join [Environment]::NewLine) | ConvertFrom-Json
    foreach ($package in $metadata.packages) {
        if (-not $package.source) {
            continue
        }
        [PSCustomObject]@{
            Ecosystem = 'Cargo'
            Component = $Component
            Name = [string]$package.name
            Version = [string]$package.version
            License = if ($package.license) { [string]$package.license } elseif ($package.license_file) { "See $($package.license_file)" } else { 'UNKNOWN' }
            Source = [string]$package.source
        }
    }
}

$cargoPackages = @()
$cargoPackages += Get-CargoPackages (Join-Path $root 'Cargo.toml') 'Worker'
$cargoPackages += Get-CargoPackages (Join-Path $root 'studio-tauri\src-tauri\Cargo.toml') 'Desktop service'
$cargoPackages += Get-CargoPackages (Join-Path $root 'fuzz\Cargo.toml') 'Fuzz targets'

$cargoRows = $cargoPackages |
    Group-Object Name, Version, License, Source |
    ForEach-Object {
        $first = $_.Group[0]
        [PSCustomObject]@{
            Name = $first.Name
            Version = $first.Version
            License = $first.License
            UsedBy = ($_.Group.Component | Sort-Object -Unique) -join ', '
        }
    } |
    Sort-Object Name, Version

$lockPath = Join-Path $root 'studio-tauri\package-lock.json'
$nodeScript = @'
const fs = require('fs');
const lock = JSON.parse(fs.readFileSync(process.argv[1], 'utf8'));
const rows = Object.entries(lock.packages ?? {})
  .filter(([path, value]) => path && value?.version)
  .map(([path, value]) => ({
    Name: path.replace(/^node_modules\//, ''),
    Version: String(value.version),
    License: value.license ? String(value.license) : 'UNKNOWN'
  }));
process.stdout.write(JSON.stringify(rows));
'@
$npmJson = & node -e $nodeScript $lockPath
if ($LASTEXITCODE -ne 0) {
    throw "npm lock parsing failed for $lockPath"
}
$npmRows = $npmJson | ConvertFrom-Json
$npmRows = $npmRows | Sort-Object Name, Version

$lines = [Collections.Generic.List[string]]::new()
$lines.Add('# Package-managed dependency licenses')
$lines.Add('')
$lines.Add('This file is generated from the committed Cargo and npm lock data by')
$lines.Add('`scripts/generate-license-report.ps1`. It records package metadata for review;')
$lines.Add('the dependency source distributions remain the authoritative license texts.')
$lines.Add('')
$lines.Add("Generated inventory: $($cargoRows.Count) Cargo packages and $($npmRows.Count) npm packages.")
$lines.Add('')
$lines.Add('## Cargo')
$lines.Add('')
$lines.Add('| Package | Version | License expression | Used by |')
$lines.Add('| --- | --- | --- | --- |')
$code = [char]96
foreach ($row in $cargoRows) {
    $license = $row.License -replace '\|', '\|'
    $lines.Add("| $code$($row.Name)$code | $code$($row.Version)$code | $license | $($row.UsedBy) |")
}
$lines.Add('')
$lines.Add('## npm')
$lines.Add('')
$lines.Add('| Package | Version | License expression |')
$lines.Add('| --- | --- | --- |')
foreach ($row in $npmRows) {
    $license = $row.License -replace '\|', '\|'
    $lines.Add("| $code$($row.Name)$code | $code$($row.Version)$code | $license |")
}
$lines.Add('')

$content = ($lines -join "`n") + "`n"
if ($Check) {
    if (-not (Test-Path -LiteralPath $outputPath)) {
        throw 'Dependency license report is missing. Run scripts/generate-license-report.ps1.'
    }
    $existing = (Get-Content -Raw -LiteralPath $outputPath) -replace "`r`n", "`n"
    if ($existing -cne $content) {
        throw 'Dependency license report is stale. Run scripts/generate-license-report.ps1.'
    }
    Write-Output 'Dependency license report is current.'
    return
}

[IO.File]::WriteAllText($outputPath, $content, [Text.UTF8Encoding]::new($false))
Write-Output "Wrote $outputPath"
