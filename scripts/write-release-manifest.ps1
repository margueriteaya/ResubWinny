param(
    [Parameter(Mandatory = $true)]
    [string]$ArtifactDirectory,

    [Parameter(Mandatory = $true)]
    [string]$Tag,

    [string]$Revision = 'HEAD',

    [ValidateSet('SourceRelease', 'UnsignedWindowsAlpha', 'SignedStable')]
    [string]$Tier = 'UnsignedWindowsAlpha'
)

$ErrorActionPreference = 'Stop'
$root = (Resolve-Path (Join-Path $PSScriptRoot '..')).Path

function Resolve-GitCommit([string]$Reference) {
    $commit = (& git rev-parse --verify "$Reference^{commit}" 2>$null).Trim()
    if ($LASTEXITCODE -ne 0 -or -not $commit) {
        throw "Git revision does not exist: $Reference"
    }
    return $commit
}

Push-Location $root
try {
    $artifactCandidate = if ([IO.Path]::IsPathRooted($ArtifactDirectory)) {
        $ArtifactDirectory
    } else {
        Join-Path $root $ArtifactDirectory
    }
    $artifactPath = [IO.Path]::GetFullPath($artifactCandidate)
    if (-not (Test-Path -LiteralPath $artifactPath -PathType Container)) {
        throw "Artifact directory does not exist: $artifactPath"
    }

    $version = (Get-Content -Raw studio-tauri/package.json | ConvertFrom-Json).version
    if ($Tier -eq 'SignedStable') {
        throw 'SignedStable manifests require Authenticode verification and are not supported by this unsigned-release tool.'
    }
    $expectedTag = "v$version"
    if ($Tag -cne $expectedTag) {
        throw "Release tag differs from the package version: expected $expectedTag, found $Tag"
    }

    $commit = Resolve-GitCommit $Revision
    $tagCommit = Resolve-GitCommit $Tag
    if ($tagCommit -cne $commit) {
        throw "Release tag $Tag resolves to $tagCommit, not requested commit $commit"
    }

    $manifestPath = Join-Path $artifactPath 'RELEASE-MANIFEST.json'
    $sumsPath = Join-Path $artifactPath 'SHA256SUMS.txt'
    $excluded = @($manifestPath, $sumsPath)
    $files = @(Get-ChildItem -LiteralPath $artifactPath -Recurse -File |
        Where-Object { $excluded -notcontains $_.FullName } |
        Sort-Object FullName)
    if (-not $files) {
        throw "Artifact directory contains no release payloads: $artifactPath"
    }

    $artifacts = foreach ($file in $files) {
        $relativePath = [IO.Path]::GetRelativePath($artifactPath, $file.FullName).Replace('\', '/')
        [ordered]@{
            path = $relativePath
            bytes = $file.Length
            sha256 = (Get-FileHash -LiteralPath $file.FullName -Algorithm SHA256).Hash
        }
    }

    $manifest = [ordered]@{
        schemaVersion = 1
        releaseTier = $Tier
        version = $version
        tag = $Tag
        commit = $commit
        codeSigned = $false
        unsignedWarningRequired = $Tier -eq 'UnsignedWindowsAlpha'
        generatedAtUtc = [DateTime]::UtcNow.ToString('o')
        artifacts = $artifacts
    }
    [IO.File]::WriteAllText(
        $manifestPath,
        (($manifest | ConvertTo-Json -Depth 5) + "`n"),
        [Text.UTF8Encoding]::new($false)
    )

    $sumLines = $artifacts | ForEach-Object { "$($_.sha256)  $($_.path)" }
    [IO.File]::WriteAllText(
        $sumsPath,
        (($sumLines -join "`n") + "`n"),
        [Text.UTF8Encoding]::new($false)
    )

    Write-Output "Created $manifestPath"
    Write-Output "Created $sumsPath"
} finally {
    Pop-Location
}
