param([switch]$Force)

$ErrorActionPreference = 'Stop'
$root = (Resolve-Path (Join-Path $PSScriptRoot '..')).Path
$manifest = Get-Content -Raw -LiteralPath (Join-Path $root 'third_party\versions.json') |
    ConvertFrom-Json
$dependency = $manifest.dependencies.libmpvWindowsX86_64
$destination = Join-Path $root 'third_party\libmpv\windows-x86_64'
$dll = Join-Path $destination 'libmpv-2.dll'
$importLibrary = Join-Path $destination 'libmpv.dll.a'

function Test-ExpectedFile([string]$Path, [string]$ExpectedHash) {
    if (-not (Test-Path -LiteralPath $Path -PathType Leaf)) { return $false }
    return (Get-FileHash -LiteralPath $Path -Algorithm SHA256).Hash -eq $ExpectedHash
}

if (-not $Force -and
    (Test-ExpectedFile $dll $dependency.dllSha256) -and
    (Test-ExpectedFile $importLibrary $dependency.importLibrarySha256)) {
    Write-Output 'Pinned Windows libmpv runtime is already installed.'
    return
}

$sevenZip = Get-Command 7z, 7zz, 7za -ErrorAction SilentlyContinue |
    Select-Object -First 1
if (-not $sevenZip) {
    throw '7-Zip is required to install the pinned libmpv development runtime.'
}

$downloadDirectory = Join-Path $root 'build\downloads'
$extractDirectory = Join-Path $root 'build\libmpv-runtime'
New-Item -ItemType Directory -Force -Path $downloadDirectory | Out-Null
if (Test-Path -LiteralPath $extractDirectory) {
    Remove-Item -LiteralPath $extractDirectory -Recurse -Force
}
New-Item -ItemType Directory -Force -Path $extractDirectory | Out-Null

$archive = Join-Path $downloadDirectory $dependency.asset
$url = "https://github.com/zhongfly/mpv-winbuild/releases/download/$($dependency.buildTag)/$($dependency.asset)"
if ($Force -or -not (Test-ExpectedFile $archive $dependency.assetSha256)) {
    Invoke-WebRequest -Uri $url -OutFile $archive
}
if (-not (Test-ExpectedFile $archive $dependency.assetSha256)) {
    throw 'Downloaded libmpv archive does not match third_party/versions.json.'
}

& $sevenZip.Source x $archive "-o$extractDirectory" -y | Out-Null
if ($LASTEXITCODE -ne 0) { throw 'Could not extract the libmpv archive.' }

$stagedDll = Get-ChildItem -LiteralPath $extractDirectory -Recurse -File -Filter 'libmpv-2.dll' |
    Select-Object -First 1
$stagedImportLibrary = Get-ChildItem -LiteralPath $extractDirectory -Recurse -File -Filter 'libmpv.dll.a' |
    Select-Object -First 1
if (-not $stagedDll -or -not $stagedImportLibrary) {
    throw 'The pinned archive does not contain the expected libmpv files.'
}
if (-not (Test-ExpectedFile $stagedDll.FullName $dependency.dllSha256) -or
    -not (Test-ExpectedFile $stagedImportLibrary.FullName $dependency.importLibrarySha256)) {
    throw 'Extracted libmpv files do not match third_party/versions.json.'
}

New-Item -ItemType Directory -Force -Path $destination | Out-Null
Copy-Item -LiteralPath $stagedDll.FullName -Destination $dll -Force
Copy-Item -LiteralPath $stagedImportLibrary.FullName -Destination $importLibrary -Force
Write-Output "Installed pinned Windows libmpv runtime in $destination"
