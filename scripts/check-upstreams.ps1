param(
    [switch]$Online,
    [switch]$FailOnUpdate,
    [switch]$RequireRuntime
)

$ErrorActionPreference = 'Stop'
$repositoryRoot = Split-Path -Parent $PSScriptRoot
$manifestPath = Join-Path $repositoryRoot 'third_party\versions.json'
$manifest = Get-Content -Raw -LiteralPath $manifestPath | ConvertFrom-Json
$dependencies = $manifest.dependencies
$updatesAvailable = $false

function Assert-Equal {
    param(
        [string]$Label,
        [string]$Actual,
        [string]$Expected
    )

    if ($Actual -cne $Expected) {
        throw "$Label mismatch. Expected $Expected, found $Actual."
    }
    Write-Host "[ok] ${Label}: $Actual"
}

function Get-RemoteHead {
    param([string]$Url)

    $line = git ls-remote $Url HEAD
    if ($LASTEXITCODE -ne 0 -or -not $line) {
        throw "Could not query upstream HEAD: $Url"
    }
    return ($line -split '\s+')[0]
}

function Get-SourceSnapshotHash([string]$Directory) {
    $root = (Resolve-Path -LiteralPath $Directory).Path
    $gitPrefix = (Join-Path $root '.git') + [IO.Path]::DirectorySeparatorChar
    $hash = [Security.Cryptography.IncrementalHash]::CreateHash(
        [Security.Cryptography.HashAlgorithmName]::SHA256
    )
    try {
        Get-ChildItem -LiteralPath $root -Recurse -File |
            Where-Object { -not $_.FullName.StartsWith($gitPrefix) } |
            Sort-Object { [IO.Path]::GetRelativePath($root, $_.FullName).Replace('\', '/') } |
            ForEach-Object {
                $relative = [IO.Path]::GetRelativePath($root, $_.FullName).Replace('\', '/')
                $hash.AppendData([Text.Encoding]::UTF8.GetBytes($relative))
                $hash.AppendData([byte[]]@(0))
                $hash.AppendData([BitConverter]::GetBytes([Int64]$_.Length))
                $hash.AppendData([IO.File]::ReadAllBytes($_.FullName))
            }
        return [Convert]::ToHexString($hash.GetHashAndReset())
    } finally {
        $hash.Dispose()
    }
}

$libaribcaptionPath = Join-Path $repositoryRoot 'third_party\libaribcaption'
$libaribcaptionGit = Join-Path $libaribcaptionPath '.git'
if (Test-Path -LiteralPath $libaribcaptionGit) {
    $localCommit = git -C $libaribcaptionPath rev-parse HEAD
    if ($LASTEXITCODE -ne 0) {
        throw 'Could not read the vendored libaribcaption commit.'
    }
    Assert-Equal 'libaribcaption vendored commit' $localCommit $dependencies.libaribcaption.commit
} else {
    Write-Host '[info] libaribcaption is a source snapshot; manifest commit is authoritative.'
}
$libaribcaptionSnapshotHash = Get-SourceSnapshotHash $libaribcaptionPath
Assert-Equal 'libaribcaption source snapshot SHA-256' `
    $libaribcaptionSnapshotHash `
    $dependencies.libaribcaption.sourceSnapshotSha256

$libmpvPath = Join-Path $repositoryRoot 'third_party\libmpv\windows-x86_64\libmpv-2.dll'
if (-not (Test-Path -LiteralPath $libmpvPath -PathType Leaf)) {
    if ($RequireRuntime) {
        throw 'Windows libmpv is not installed. Run scripts/setup-libmpv.ps1 first.'
    }
    Write-Host '[skip] Windows libmpv runtime is not installed; pinned metadata remains available.'
} else {
    $libmpvHash = (Get-FileHash -LiteralPath $libmpvPath -Algorithm SHA256).Hash
    Assert-Equal 'libmpv-2.dll SHA-256' $libmpvHash $dependencies.libmpvWindowsX86_64.dllSha256

    $importLibraryPath = Join-Path $repositoryRoot 'third_party\libmpv\windows-x86_64\libmpv.dll.a'
    if (-not (Test-Path -LiteralPath $importLibraryPath -PathType Leaf)) {
        throw 'The libmpv DLL is installed without its pinned import library.'
    }
    $importLibraryHash = (Get-FileHash -LiteralPath $importLibraryPath -Algorithm SHA256).Hash
    Assert-Equal 'libmpv import library SHA-256' $importLibraryHash $dependencies.libmpvWindowsX86_64.importLibrarySha256
}

$fontPath = Join-Path $repositoryRoot 'third_party\rounded-mplus-1m-arib\rounded-mplus-1m-arib.ttf'
$fontHash = (Get-FileHash -LiteralPath $fontPath -Algorithm SHA256).Hash
Assert-Equal 'Rounded M+ 1m for ARIB SHA-256' $fontHash $dependencies.roundedMplus1mArib.sha256

$libmpvWorkflowPath = Join-Path $repositoryRoot '.github\workflows\libmpv-lgpl.yml'
$libmpvWorkflow = Get-Content -Raw -LiteralPath $libmpvWorkflowPath
foreach ($requiredReference in @('third_party/versions.json', 'buildRecipeCommit', 'toolchainCommit', 'mpvCommit', 'ffmpegCommit')) {
    if (-not $libmpvWorkflow.Contains($requiredReference, [StringComparison]::Ordinal)) {
        throw "libmpv workflow does not read the canonical dependency field: $requiredReference"
    }
}
Write-Host '[ok] libmpv workflow reads its pins from the dependency manifest'

if ($Online) {
    $libaribcaptionHead = Get-RemoteHead $dependencies.libaribcaption.upstream
    if ($libaribcaptionHead -eq $dependencies.libaribcaption.commit) {
        Write-Host "[current] libaribcaption: $libaribcaptionHead"
    } else {
        Write-Host "[update] libaribcaption: $($dependencies.libaribcaption.commit) -> $libaribcaptionHead"
        $updatesAvailable = $true
    }

    $aribb62Head = Get-RemoteHead $dependencies.aribb62Js.upstream
    if ($aribb62Head -eq $dependencies.aribb62Js.reviewedCommit) {
        Write-Host "[current] aribb62.js reference: $aribb62Head"
    } else {
        Write-Host "[review] aribb62.js reference changed: $($dependencies.aribb62Js.reviewedCommit) -> $aribb62Head"
        $updatesAvailable = $true
    }

    $tagRef = "refs/tags/$($dependencies.libmpvWindowsX86_64.buildTag)"
    $tagLine = git ls-remote $dependencies.libmpvWindowsX86_64.buildUpstream $tagRef
    if ($LASTEXITCODE -ne 0 -or -not $tagLine) {
        throw "Could not verify libmpv build tag $tagRef."
    }
    $tagCommit = ($tagLine -split '\s+')[0]
    Assert-Equal 'libmpv release tag commit' $tagCommit $dependencies.libmpvWindowsX86_64.releaseTagCommit

    $buildRecipeHead = Get-RemoteHead $dependencies.libmpvWindowsX86_64.buildUpstream
    if ($buildRecipeHead -eq $dependencies.libmpvWindowsX86_64.buildRecipeCommit) {
        Write-Host "[current] libmpv build recipe: $buildRecipeHead"
    } else {
        Write-Host "[info] libmpv build recipe is pinned: $($dependencies.libmpvWindowsX86_64.buildRecipeCommit); current HEAD is $buildRecipeHead"
    }

    $toolchainHead = Get-RemoteHead $dependencies.libmpvWindowsX86_64.toolchainUpstream
    if ($toolchainHead -eq $dependencies.libmpvWindowsX86_64.toolchainCommit) {
        Write-Host "[current] libmpv toolchain: $toolchainHead"
    } else {
        Write-Host "[info] libmpv toolchain is pinned: $($dependencies.libmpvWindowsX86_64.toolchainCommit); current HEAD is $toolchainHead"
    }
}

if ($updatesAvailable -and $FailOnUpdate) {
    throw 'One or more upstream updates require maintainer review.'
}

if ($updatesAvailable) {
    Write-Host 'Upstream changes are available. Review is required; nothing was updated automatically.'
} else {
    Write-Host 'Pinned third-party dependencies are current for the checks performed.'
}
