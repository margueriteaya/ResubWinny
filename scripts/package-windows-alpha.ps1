param(
    [Parameter(Mandatory = $true)]
    [string]$DesktopExecutable,

    [Parameter(Mandatory = $true)]
    [string]$WorkerExecutable,

    [Parameter(Mandatory = $true)]
    [string]$InstallerDirectory,

    [Parameter(Mandatory = $true)]
    [string]$SourceDirectory,

    [Parameter(Mandatory = $true)]
    [string]$LibmpvCandidateDirectory,

    [Parameter(Mandatory = $true)]
    [string]$Tag,

    [string]$Revision = 'HEAD',
    [string]$OutputDirectory = 'build/release/windows-alpha'
)

$ErrorActionPreference = 'Stop'
$root = (Resolve-Path (Join-Path $PSScriptRoot '..')).Path

function Resolve-InputPath([string]$Path, [string]$Label, [bool]$Directory) {
    $candidate = if ([IO.Path]::IsPathRooted($Path)) { $Path } else { Join-Path $root $Path }
    $kind = if ($Directory) { 'Container' } else { 'Leaf' }
    if (-not (Test-Path -LiteralPath $candidate -PathType $kind)) {
        throw "$Label does not exist: $candidate"
    }
    return (Resolve-Path -LiteralPath $candidate).Path
}

function Resolve-SingleFile([string]$Directory, [string]$Filter, [string]$Label) {
    $matches = @(Get-ChildItem -LiteralPath $Directory -Recurse -File -Filter $Filter)
    if ($matches.Count -ne 1) {
        throw "$Label must contain exactly one $Filter file; found $($matches.Count)."
    }
    return $matches[0].FullName
}

function Assert-Hash([string]$Path, [string]$Expected, [string]$Label) {
    $actual = (Get-FileHash -LiteralPath $Path -Algorithm SHA256).Hash
    if ($actual -cne $Expected) {
        throw "$Label SHA-256 mismatch. Expected $Expected, found $actual."
    }
}

function Copy-ReleaseFile(
    [string]$Source,
    [string]$DestinationDirectory,
    [string]$DestinationName = (Split-Path -Leaf $Source)
) {
    [IO.Directory]::CreateDirectory($DestinationDirectory) | Out-Null
    Copy-Item -LiteralPath $Source -Destination (Join-Path $DestinationDirectory $DestinationName)
}

Push-Location $root
try {
    $desktopPath = Resolve-InputPath $DesktopExecutable 'Desktop executable' $false
    $workerPath = Resolve-InputPath $WorkerExecutable 'Worker executable' $false
    $installerPath = Resolve-InputPath $InstallerDirectory 'Installer directory' $true
    $sourcePath = Resolve-InputPath $SourceDirectory 'Source candidate directory' $true
    $libmpvPath = Resolve-InputPath $LibmpvCandidateDirectory 'libmpv candidate directory' $true

    $installers = @(Get-ChildItem -LiteralPath $installerPath -Recurse -File |
        Where-Object { $_.Extension -eq '.msi' -or $_.Name -like '*setup.exe' })
    if (-not $installers) {
        throw "Installer directory contains no MSI or setup executable: $installerPath"
    }

    $sourceArchive = Resolve-SingleFile $sourcePath 'resubwinny-*-source.zip' 'Source candidate directory'
    $sourceSums = Resolve-SingleFile $sourcePath 'SOURCE-SHA256SUMS.txt' 'Source candidate directory'
    $sourceSumText = Get-Content -Raw -LiteralPath $sourceSums
    $sourceArchiveHash = (Get-FileHash -LiteralPath $sourceArchive -Algorithm SHA256).Hash
    if (-not $sourceSumText.Contains($sourceArchiveHash)) {
        throw "SOURCE-SHA256SUMS.txt does not match $sourceArchive."
    }

    $receiptPath = Resolve-SingleFile $libmpvPath 'SOURCE-RECEIPT.json' 'libmpv candidate directory'
    $libmpvDll = Resolve-SingleFile $libmpvPath 'libmpv-2.dll' 'libmpv candidate directory'
    $importLibrary = Resolve-SingleFile $libmpvPath 'libmpv.dll.a' 'libmpv candidate directory'
    $correspondingSource = Resolve-SingleFile $libmpvPath 'libmpv-corresponding-source-*.tar.gz' 'libmpv candidate directory'
    $patchedToolchain = Resolve-SingleFile $libmpvPath 'PATCHED-TOOLCHAIN.diff' 'libmpv candidate directory'
    $buildEnvironment = Resolve-SingleFile $libmpvPath 'BUILD-ENVIRONMENT.json' 'libmpv candidate directory'

    $receipt = Get-Content -Raw -LiteralPath $receiptPath | ConvertFrom-Json
    if ($receipt.schemaVersion -ne 1) {
        throw "Unsupported libmpv SOURCE-RECEIPT schema: $($receipt.schemaVersion)"
    }
    if ($receipt.binary.dll -cne (Split-Path -Leaf $libmpvDll) -or
        $receipt.binary.importLibrary -cne (Split-Path -Leaf $importLibrary)) {
        throw 'libmpv receipt binary names do not match the candidate files.'
    }
    if ($receipt.buildEnvironment.file -cne (Split-Path -Leaf $buildEnvironment)) {
        throw 'libmpv receipt build environment name does not match the candidate file.'
    }
    $packages = @($receipt.packages)
    if ($receipt.packageCount -lt 30 -or $packages.Count -ne $receipt.packageCount) {
        throw 'libmpv receipt does not describe a complete source package set.'
    }
    $packageNames = @($packages | ForEach-Object { $_.name })
    foreach ($requiredPackage in @('mpv', 'ffmpeg', 'libplacebo', 'libass', 'libaribcaption')) {
        if ($packageNames -notcontains $requiredPackage) {
            throw "libmpv receipt is missing required source package: $requiredPackage"
        }
    }
    Assert-Hash $libmpvDll $receipt.binary.dllSha256 'libmpv DLL receipt'
    Assert-Hash $importLibrary $receipt.binary.importLibrarySha256 'libmpv import library receipt'
    if ((Split-Path -Leaf $correspondingSource) -cne $receipt.archive.file) {
        throw 'Corresponding-source archive name does not match SOURCE-RECEIPT.json.'
    }
    Assert-Hash $correspondingSource $receipt.archive.sha256 'libmpv corresponding source receipt'
    Assert-Hash $buildEnvironment $receipt.buildEnvironment.sha256 'libmpv build environment receipt'
    Assert-Hash $patchedToolchain $receipt.provenance.patchedToolchainDiffSha256 'libmpv patched toolchain receipt'

    $pins = (Get-Content -Raw third_party/versions.json | ConvertFrom-Json).dependencies.libmpvWindowsX86_64
    Assert-Hash $libmpvDll $pins.dllSha256 'Pinned libmpv DLL'
    Assert-Hash $importLibrary $pins.importLibrarySha256 'Pinned libmpv import library'
    foreach ($field in @('releaseTag', 'releaseTagCommit', 'buildRecipeCommit', 'buildRunId', 'toolchainBaseCommit', 'mpvCommit', 'ffmpegCommit')) {
        $expectedField = switch ($field) {
            'releaseTag' { 'buildTag' }
            'toolchainBaseCommit' { 'toolchainCommit' }
            default { $field }
        }
        if ([string]$receipt.provenance.$field -cne [string]$pins.$expectedField) {
            throw "libmpv receipt provenance $field does not match third_party/versions.json."
        }
    }

    $outputCandidate = if ([IO.Path]::IsPathRooted($OutputDirectory)) {
        $OutputDirectory
    } else {
        Join-Path $root $OutputDirectory
    }
    $outputPath = [IO.Path]::GetFullPath($outputCandidate)
    if (Test-Path -LiteralPath $outputPath) {
        throw "Output directory already exists; refusing to mix release candidates: $outputPath"
    }

    $applicationOutput = Join-Path $outputPath 'application'
    $installerOutput = Join-Path $applicationOutput 'installers'
    Copy-ReleaseFile $desktopPath $applicationOutput
    Copy-ReleaseFile $workerPath $applicationOutput
    foreach ($installer in $installers) {
        Copy-ReleaseFile $installer.FullName $installerOutput
    }

    $sourceOutput = Join-Path $outputPath 'source'
    Copy-ReleaseFile $sourceArchive $sourceOutput
    Copy-ReleaseFile $sourceSums $sourceOutput

    $libmpvOutput = Join-Path $outputPath 'libmpv'
    foreach ($file in @($receiptPath, $libmpvDll, $importLibrary, $correspondingSource, $patchedToolchain, $buildEnvironment)) {
        Copy-ReleaseFile $file $libmpvOutput
    }

    $legalOutput = Join-Path $outputPath 'legal'
    $legalFiles = [ordered]@{
        'LICENSE' = 'LICENSE'
        'UNSIGNED-WINDOWS-ALPHA.txt' = 'UNSIGNED-WINDOWS-ALPHA.txt'
        'THIRD_PARTY_NOTICES.md' = 'THIRD_PARTY_NOTICES.md'
        'docs/dependency-licenses.md' = 'dependency-licenses.md'
        'third_party/libaribcaption/LICENSE' = 'libaribcaption-MIT.txt'
        'third_party/libmpv/LICENSE.LGPL' = 'libmpv-LGPL-2.1.txt'
        'third_party/libmpv/COPYRIGHT.mpv' = 'libmpv-COPYRIGHT.txt'
        'third_party/rounded-mplus-1m-arib/LICENSE.txt' = 'rounded-mplus-1m-arib-LICENSE.txt'
    }
    foreach ($file in $legalFiles.GetEnumerator()) {
        $sourceFile = Resolve-InputPath $file.Key "Release legal file $($file.Key)" $false
        Copy-ReleaseFile $sourceFile $legalOutput $file.Value
    }

    ./scripts/write-release-manifest.ps1 `
        -ArtifactDirectory $outputPath `
        -Tag $Tag `
        -Revision $Revision `
        -Tier UnsignedWindowsAlpha

    Write-Output "Created unsigned Windows Alpha candidate: $outputPath"
} finally {
    Pop-Location
}
