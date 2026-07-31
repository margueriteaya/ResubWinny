param(
    [Parameter(Mandatory = $true)]
    [string]$SourceCache,
    [Parameter(Mandatory = $true)]
    [string]$PatchedToolchain,
    [Parameter(Mandatory = $true)]
    [string]$BuildRecipe,
    [Parameter(Mandatory = $true)]
    [string]$BinaryDirectory,
    [Parameter(Mandatory = $true)]
    [string]$OutputDirectory
)

$ErrorActionPreference = 'Stop'
$root = (Resolve-Path (Join-Path $PSScriptRoot '..')).Path
$versions = Get-Content -Raw -LiteralPath (Join-Path $root 'third_party\versions.json') | ConvertFrom-Json
$pinned = $versions.dependencies.libmpvWindowsX86_64

function Resolve-Directory([string]$Path, [string]$Label) {
    if (-not (Test-Path -LiteralPath $Path -PathType Container)) {
        throw "$Label directory does not exist: $Path"
    }
    return (Resolve-Path -LiteralPath $Path).Path
}

function Invoke-Git([string]$Directory, [string[]]$Arguments) {
    $result = & git -C $Directory @Arguments
    if ($LASTEXITCODE -ne 0) {
        throw "git $($Arguments -join ' ') failed in $Directory"
    }
    return ($result -join "`n").Trim()
}

$sourceCachePath = Resolve-Directory $SourceCache 'Source cache'
$toolchainPath = Resolve-Directory $PatchedToolchain 'Patched toolchain'
$recipePath = Resolve-Directory $BuildRecipe 'Build recipe'
$binaryPath = Resolve-Directory $BinaryDirectory 'Binary'

$recipeHead = Invoke-Git $recipePath @('rev-parse', 'HEAD')
if ($recipeHead -cne $pinned.buildRecipeCommit) {
    throw "Build recipe mismatch. Expected $($pinned.buildRecipeCommit), found $recipeHead."
}

& git -C $toolchainPath merge-base --is-ancestor $pinned.toolchainCommit HEAD
if ($LASTEXITCODE -ne 0) {
    throw "Pinned toolchain commit $($pinned.toolchainCommit) is not an ancestor of the patched toolchain."
}

$mpvDefinition = Get-Content -Raw -LiteralPath (Join-Path $toolchainPath 'packages\mpv.cmake')
$ffmpegDefinition = Get-Content -Raw -LiteralPath (Join-Path $toolchainPath 'packages\ffmpeg.cmake')
if ($mpvDefinition -notmatch [regex]::Escape('-Dgpl=false')) {
    throw 'Patched toolchain does not disable GPL features in mpv.'
}
if ($ffmpegDefinition -match '(?m)^\s*--enable-gpl\s*$') {
    throw 'Patched toolchain still enables GPL features in FFmpeg.'
}

$requiredPackages = @('mpv', 'ffmpeg', 'libplacebo', 'libass', 'libaribcaption')
foreach ($package in $requiredPackages) {
    $packagePath = Join-Path $sourceCachePath $package
    if (-not (Test-Path -LiteralPath $packagePath -PathType Container)) {
        throw "Required source package is missing from the build cache: $package"
    }
    if (-not (Test-Path -LiteralPath (Join-Path $packagePath '.git'))) {
        throw "Required source package has no Git provenance: $package"
    }
}

$mpvHead = Invoke-Git (Join-Path $sourceCachePath 'mpv') @('rev-parse', 'HEAD')
if ($mpvHead -cne $pinned.mpvCommit) {
    throw "mpv source mismatch. Expected $($pinned.mpvCommit), found $mpvHead."
}
$ffmpegHead = Invoke-Git (Join-Path $sourceCachePath 'ffmpeg') @('rev-parse', 'HEAD')
if ($ffmpegHead -cne $pinned.ffmpegCommit) {
    throw "FFmpeg source mismatch. Expected $($pinned.ffmpegCommit), found $ffmpegHead."
}

$packageDirectories = @(Get-ChildItem -LiteralPath $sourceCachePath -Directory | Sort-Object Name)
if ($packageDirectories.Count -lt 30) {
    throw "Source cache contains only $($packageDirectories.Count) package directories; refusing an incomplete archive."
}

$packages = foreach ($directory in $packageDirectories) {
    $gitDirectory = Join-Path $directory.FullName '.git'
    if (Test-Path -LiteralPath $gitDirectory) {
        [PSCustomObject]@{
            name = $directory.Name
            kind = 'git'
            commit = Invoke-Git $directory.FullName @('rev-parse', 'HEAD')
            remote = Invoke-Git $directory.FullName @('config', '--get', 'remote.origin.url')
            status = Invoke-Git $directory.FullName @('status', '--porcelain=v1')
        }
    } else {
        [PSCustomObject]@{
            name = $directory.Name
            kind = 'source-directory'
            commit = $null
            remote = $null
            status = $null
        }
    }
}

$outputPath = [IO.Path]::GetFullPath($OutputDirectory)
[IO.Directory]::CreateDirectory($outputPath) | Out-Null
$archiveName = "libmpv-corresponding-source-$($pinned.buildTag).tar.gz"
$archivePath = Join-Path $outputPath $archiveName
$receiptPath = Join-Path $outputPath 'SOURCE-RECEIPT.json'
$toolchainDiffPath = Join-Path $outputPath 'PATCHED-TOOLCHAIN.diff'

$dllPath = Join-Path $binaryPath 'libmpv-2.dll'
$importLibraryPath = Join-Path $binaryPath 'libmpv.dll.a'
if (-not (Test-Path -LiteralPath $dllPath -PathType Leaf)) {
    throw "Built libmpv DLL is missing: $dllPath"
}
if (-not (Test-Path -LiteralPath $importLibraryPath -PathType Leaf)) {
    throw "Built libmpv import library is missing: $importLibraryPath"
}

$toolchainDiff = & git -C $toolchainPath diff --binary HEAD
if ($LASTEXITCODE -ne 0) {
    throw 'Could not capture the patched toolchain diff.'
}
[IO.File]::WriteAllText($toolchainDiffPath, (($toolchainDiff -join "`n") + "`n"), [Text.UTF8Encoding]::new($false))
$toolchainDiffHash = (Get-FileHash -LiteralPath $toolchainDiffPath -Algorithm SHA256).Hash

$receipt = [ordered]@{
    schemaVersion = 1
    generatedAtUtc = [DateTime]::UtcNow.ToString('o')
    binary = [ordered]@{
        dll = 'libmpv-2.dll'
        dllSha256 = (Get-FileHash -LiteralPath $dllPath -Algorithm SHA256).Hash
        importLibrary = 'libmpv.dll.a'
        importLibrarySha256 = (Get-FileHash -LiteralPath $importLibraryPath -Algorithm SHA256).Hash
    }
    provenance = [ordered]@{
        releaseTag = $pinned.buildTag
        releaseTagCommit = $pinned.releaseTagCommit
        buildRecipeCommit = $pinned.buildRecipeCommit
        buildRunId = $pinned.buildRunId
        toolchainBaseCommit = $pinned.toolchainCommit
        patchedToolchainCommit = Invoke-Git $toolchainPath @('rev-parse', 'HEAD')
        patchedToolchainStatus = Invoke-Git $toolchainPath @('status', '--porcelain=v1')
        patchedToolchainDiffSha256 = $toolchainDiffHash
        mpvCommit = $pinned.mpvCommit
        ffmpegCommit = $pinned.ffmpegCommit
    }
    packageCount = $packages.Count
    packages = $packages
}
[IO.File]::WriteAllText($receiptPath, ($receipt | ConvertTo-Json -Depth 6), [Text.UTF8Encoding]::new($false))

if (Test-Path -LiteralPath $archivePath) {
    throw "Output archive already exists: $archivePath"
}

$sourceParent = Split-Path -Parent $sourceCachePath
$sourceName = Split-Path -Leaf $sourceCachePath
$toolchainParent = Split-Path -Parent $toolchainPath
$toolchainName = Split-Path -Leaf $toolchainPath
$recipeParent = Split-Path -Parent $recipePath
$recipeName = Split-Path -Leaf $recipePath

& tar -czf $archivePath `
    "--exclude=$toolchainName/src_packages" `
    "--exclude=$toolchainName/build*" `
    "--exclude=$toolchainName/clang_root" `
    "--exclude=$toolchainName/install_rustup" `
    "--exclude=$toolchainName/release" `
    -C $outputPath (Split-Path -Leaf $receiptPath) `
    -C $outputPath (Split-Path -Leaf $toolchainDiffPath) `
    -C $sourceParent $sourceName `
    -C $toolchainParent $toolchainName `
    -C $recipeParent $recipeName
if ($LASTEXITCODE -ne 0) {
    throw 'tar failed while creating the corresponding-source archive.'
}

$archiveHash = (Get-FileHash -LiteralPath $archivePath -Algorithm SHA256).Hash
$receipt.archive = [ordered]@{
    file = $archiveName
    sha256 = $archiveHash
    bytes = (Get-Item -LiteralPath $archivePath).Length
}
[IO.File]::WriteAllText($receiptPath, ($receipt | ConvertTo-Json -Depth 6), [Text.UTF8Encoding]::new($false))

Write-Output "Created $archivePath"
Write-Output "SHA-256 $archiveHash"
Write-Output "Receipt $receiptPath"
