param(
    [ValidateSet('libaribcaption', 'libaribtlv', 'zlib')]
    [string]$Dependency = 'libaribcaption'
)

$ErrorActionPreference = 'Stop'
$root = (Resolve-Path (Join-Path $PSScriptRoot '..')).Path
$source = Join-Path $root "third_party\$Dependency"
$gitDirectory = Join-Path $source '.git'

if (-not (Test-Path -LiteralPath $source -PathType Container)) {
    throw "Vendored $Dependency source is missing: $source"
}
if (-not (Test-Path -LiteralPath $gitDirectory)) {
    Write-Output "$Dependency is already a plain vendored source snapshot."
    return
}

$manifest = Get-Content -Raw -LiteralPath (Join-Path $root 'third_party\versions.json') |
    ConvertFrom-Json
$metadata = $manifest.dependencies.$Dependency
$expected = [string]$metadata.commit
$actual = (& git -C $source rev-parse HEAD).Trim()
if ($LASTEXITCODE -ne 0 -or $actual -cne $expected) {
    throw "$Dependency revision mismatch. Expected $expected, found $actual."
}
$changes = & git -C $source status --porcelain=v1
if ($LASTEXITCODE -ne 0) { throw 'Could not inspect the libaribcaption snapshot.' }
if ($changes) {
    throw "Refusing to remove nested Git metadata from a modified $Dependency checkout."
}

$hash = [Security.Cryptography.IncrementalHash]::CreateHash(
    [Security.Cryptography.HashAlgorithmName]::SHA256
)
$gitPrefix = $gitDirectory + [IO.Path]::DirectorySeparatorChar
try {
    Get-ChildItem -LiteralPath $source -Recurse -File |
        Where-Object { -not $_.FullName.StartsWith($gitPrefix) } |
        Sort-Object { [IO.Path]::GetRelativePath($source, $_.FullName).Replace('\', '/') } |
        ForEach-Object {
            $relative = [IO.Path]::GetRelativePath($source, $_.FullName).Replace('\', '/')
            $hash.AppendData([Text.Encoding]::UTF8.GetBytes($relative))
            $hash.AppendData([byte[]]@(0))
            $hash.AppendData([BitConverter]::GetBytes([Int64]$_.Length))
            $hash.AppendData([IO.File]::ReadAllBytes($_.FullName))
        }
    $snapshotHash = [Convert]::ToHexString($hash.GetHashAndReset())
} finally {
    $hash.Dispose()
}
if ($snapshotHash -cne [string]$metadata.sourceContentSha256) {
    throw "$Dependency source content hash mismatch: $snapshotHash"
}

$resolvedGitDirectory = (Resolve-Path -LiteralPath $gitDirectory).Path
if (-not $resolvedGitDirectory.StartsWith(
    $root + [IO.Path]::DirectorySeparatorChar,
    [StringComparison]::OrdinalIgnoreCase
)) {
    throw "Refusing to remove metadata outside the workspace: $resolvedGitDirectory"
}

Remove-Item -LiteralPath $resolvedGitDirectory -Recurse -Force
Write-Output "Converted $Dependency $actual to a plain vendored source snapshot."
